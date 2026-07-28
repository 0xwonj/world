use world_core::{SimMoment, SimTime};
use world_model::{AcceptedState, WorldSnapshot};

use crate::authority::AuthorityCursor;
use crate::control::RuntimeControlState;
use crate::execution::ResolvedExecutionClosureManifestV1;
use crate::kernel::{KernelSafetyBlocker, KernelSafetyCause};
use crate::scheduler::SchedulerState;

use super::SessionMode;

/// Virtual-time coordinates retained by one authoritative session head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SessionClock {
    now: SimMoment,
    frontier: SimMoment,
    same_time_tranche: SameTimeWaveTranche,
}

impl SessionClock {
    const fn root(now: SimMoment, frontier: SimMoment) -> Self {
        Self {
            now,
            frontier,
            same_time_tranche: SameTimeWaveTranche::new(now.time(), 0),
        }
    }

    #[cfg(test)]
    pub(crate) const fn from_coordinates(now: SimMoment, frontier: SimMoment) -> Self {
        Self::root(now, frontier)
    }

    pub(crate) const fn now(self) -> SimMoment {
        self.now
    }

    pub(crate) const fn frontier(self) -> SimMoment {
        self.frontier
    }

    pub(crate) const fn same_time_tranche(self) -> SameTimeWaveTranche {
        self.same_time_tranche
    }

    pub(crate) fn attempted_wave(self, due: SimMoment) -> u64 {
        if self.same_time_tranche.time == due.time() {
            u64::from(self.same_time_tranche.completed_waves) + 1
        } else {
            1
        }
    }

    pub(crate) fn after_fire(
        self,
        fired: SimMoment,
        resulting_frontier: SimMoment,
    ) -> Result<Self, SessionClockProjectionError> {
        if fired < self.now {
            return Err(SessionClockProjectionError::MomentRegressed {
                current: self.now,
                supplied: fired,
            });
        }
        if resulting_frontier < self.frontier {
            return Err(SessionClockProjectionError::FrontierRegressed {
                current: self.frontier,
                supplied: resulting_frontier,
            });
        }

        let same_time_tranche = if self.same_time_tranche.time == fired.time() {
            SameTimeWaveTranche::new(
                fired.time(),
                self.same_time_tranche
                    .completed_waves
                    .checked_add(1)
                    .ok_or(SessionClockProjectionError::WaveCountOverflow { time: fired.time() })?,
            )
        } else {
            SameTimeWaveTranche::new(fired.time(), 1)
        };
        Ok(Self {
            now: fired,
            frontier: resulting_frontier,
            same_time_tranche,
        })
    }

    pub(crate) fn seal_admission_through(
        self,
        target: SimMoment,
    ) -> Result<Self, SessionClockProjectionError> {
        if target <= self.frontier {
            return Err(SessionClockProjectionError::FrontierNotAdvanced {
                current: self.frontier,
                supplied: target,
            });
        }
        Ok(Self {
            now: self.now,
            frontier: target,
            same_time_tranche: self.same_time_tranche,
        })
    }

    pub(crate) const fn reset_same_time_tranche(self, preserved_due: SimMoment) -> Self {
        Self {
            now: self.now,
            frontier: self.frontier,
            same_time_tranche: SameTimeWaveTranche::new(preserved_due.time(), 0),
        }
    }
}

/// Published-wave accounting for one simulation-time tranche.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SameTimeWaveTranche {
    time: SimTime,
    completed_waves: u32,
}

impl SameTimeWaveTranche {
    const fn new(time: SimTime, completed_waves: u32) -> Self {
        Self {
            time,
            completed_waves,
        }
    }

    /// Returns the simulation time shared by this tranche.
    #[must_use]
    pub const fn time(self) -> SimTime {
        self.time
    }

    /// Returns the number of successfully published moments in the tranche.
    #[must_use]
    pub const fn completed_waves(self) -> u32 {
        self.completed_waves
    }
}

/// Why an authority projection could not advance the virtual clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SessionClockProjectionError {
    MomentRegressed {
        current: SimMoment,
        supplied: SimMoment,
    },
    FrontierRegressed {
        current: SimMoment,
        supplied: SimMoment,
    },
    FrontierNotAdvanced {
        current: SimMoment,
        supplied: SimMoment,
    },
    WaveCountOverflow {
        time: SimTime,
    },
}

/// Why host resume cannot clear the current kernel-safety blocker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SessionResumeProjectionError {
    KernelSafetyBlocked { cause: Box<KernelSafetyCause> },
}

/// Private aggregate root of authoritative session state.
///
/// The type is intentionally non-`Clone`; successors are produced only by
/// applying sealed authority records.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SessionHead {
    cursor: AuthorityCursor,
    mode: SessionMode,
    clock: SessionClock,
    accepted: AcceptedState,
    runtime_control: RuntimeControlState,
    scheduler: SchedulerState,
    safety_blocker: Option<KernelSafetyBlocker>,
}

impl SessionHead {
    pub(crate) fn root(closure: &ResolvedExecutionClosureManifestV1) -> Self {
        let root = closure.initial_root();
        Self {
            cursor: closure.root_cursor(),
            mode: root.mode(),
            clock: SessionClock::root(root.now(), root.admission_frontier()),
            accepted: root.accepted_state().clone(),
            runtime_control: RuntimeControlState::from_root(root.action_opportunities()),
            scheduler: SchedulerState::from_action_opportunities(
                root.now(),
                root.action_opportunities(),
            )
            .unwrap_or_else(|error| {
                unreachable!("validated initial root must produce a scheduler: {error:?}")
            }),
            safety_blocker: None,
        }
    }

    pub(crate) fn from_authority_projection(
        cursor: AuthorityCursor,
        mode: SessionMode,
        clock: SessionClock,
        accepted: AcceptedState,
        runtime_control: RuntimeControlState,
        scheduler: SchedulerState,
        safety_blocker: Option<KernelSafetyBlocker>,
    ) -> Self {
        Self {
            cursor,
            mode,
            clock,
            accepted,
            runtime_control,
            scheduler,
            safety_blocker,
        }
    }

    pub(crate) const fn cursor(&self) -> AuthorityCursor {
        self.cursor
    }

    pub(crate) const fn mode(&self) -> SessionMode {
        self.mode
    }

    pub(crate) const fn clock(&self) -> SessionClock {
        self.clock
    }

    pub(crate) const fn accepted(&self) -> &AcceptedState {
        &self.accepted
    }

    pub(crate) const fn runtime_control(&self) -> &RuntimeControlState {
        &self.runtime_control
    }

    pub(crate) const fn scheduler(&self) -> &SchedulerState {
        &self.scheduler
    }

    pub(crate) const fn safety_blocker(&self) -> Option<KernelSafetyBlocker> {
        self.safety_blocker
    }

    pub(crate) fn resume_projection(
        &self,
    ) -> Result<(SessionClock, Option<KernelSafetyBlocker>), SessionResumeProjectionError> {
        let Some(blocker) = self.safety_blocker else {
            return Ok((self.clock, None));
        };
        if !blocker.permits_resume() {
            return Err(SessionResumeProjectionError::KernelSafetyBlocked {
                cause: Box::new(blocker.cause()),
            });
        }
        Ok((
            self.clock
                .reset_same_time_tranche(blocker.cause().evidence().due()),
            None,
        ))
    }

    pub(crate) fn snapshot(&self) -> WorldSnapshot {
        WorldSnapshot::new(self.cursor.revision(), self.accepted.clone())
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use world_core::{ActorId, EntityId, SimMoment};
    use world_model::{
        AcceptedState, ActionOpportunity, ActionOpportunityGeneration, ActionSponsor,
        ActorReactionCause, AgencyState, ContainmentInteractionScope, DomainState, EpistemicState,
        SocialState,
    };

    use crate::control::test_support;
    use crate::execution::{
        CanonicalExecutionSpecV1, ExecutionConfigArtifactV3, ExecutionSemanticsManifestV1,
        ExternalInputBindingV1, InitialStateRootV1, RootSeed, TerminationContractV1,
    };

    use super::*;

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("root-head fixture must be valid: {error}"),
        }
    }

    #[test]
    fn root_head_is_derived_from_one_verified_closure() {
        let definitions = test_support::definitions();
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            definitions,
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            Vec::new(),
        ));
        let accepted = AcceptedState::new(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );
        let scope = valid(ContainmentInteractionScope::new(
            EntityId::from_bytes([0x31; 32]),
            vec![EntityId::from_bytes([0x32; 32])],
            vec![EntityId::from_bytes([0x33; 32])],
            8,
        ));
        let opportunity = ActionOpportunity::open(
            ActorId::from_bytes([0x41; 32]),
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x42; 32])),
            world_model::ActionInteractionScope::containment(scope),
            ActionOpportunityGeneration::new(0),
        );
        let root = valid(InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted.clone(),
            vec![opportunity.clone()],
        ));
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &semantics,
            RootSeed::from_bytes([0x61; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        let closure = valid(ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ));

        let head = SessionHead::root(&closure);
        let snapshot = head.snapshot();

        assert_eq!(head.cursor(), closure.root_cursor());
        assert_eq!(head.mode(), SessionMode::Running);
        assert_eq!(head.clock().now(), SimMoment::ORIGIN);
        assert_eq!(head.clock().frontier(), SimMoment::ORIGIN);
        assert_eq!(
            head.clock().same_time_tranche(),
            SameTimeWaveTranche::new(SimMoment::ORIGIN.time(), 0)
        );
        assert_eq!(head.safety_blocker(), None);
        assert_eq!(head.accepted(), &accepted);
        assert!(head.runtime_control().input().iter().next().is_none());
        assert!(head.runtime_control().management().iter().next().is_none());
        assert!(head.runtime_control().command().iter().next().is_none());
        assert_eq!(
            head.runtime_control()
                .action_opportunities()
                .get(opportunity.id()),
            Some(&opportunity)
        );
        assert_eq!(head.scheduler().entry_count_at(SimMoment::ORIGIN), 1);
        assert_eq!(snapshot.revision(), head.cursor().revision());
        assert_eq!(snapshot.accepted(), head.accepted());
    }

    #[test]
    fn clock_counts_published_moments_by_simulation_time() {
        let root = SessionClock::root(moment(3, 0), moment(3, 1));
        assert_eq!(root.attempted_wave(moment(3, 4)), 1);

        let first = root
            .after_fire(moment(3, 4), moment(3, 5))
            .unwrap_or_else(|error| panic!("first wave must project: {error:?}"));
        assert_eq!(first.same_time_tranche().time(), SimTime::from_ticks(3));
        assert_eq!(first.same_time_tranche().completed_waves(), 1);
        assert_eq!(first.attempted_wave(moment(3, 6)), 2);

        let second = first
            .after_fire(moment(3, 6), moment(4, 0))
            .unwrap_or_else(|error| panic!("second wave must project: {error:?}"));
        assert_eq!(second.same_time_tranche().completed_waves(), 2);

        let later = second
            .after_fire(moment(7, 0), moment(7, 1))
            .unwrap_or_else(|error| panic!("later time must start a tranche: {error:?}"));
        assert_eq!(later.same_time_tranche().time(), SimTime::from_ticks(7));
        assert_eq!(later.same_time_tranche().completed_waves(), 1);
        assert_eq!(later.attempted_wave(moment(8, 0)), 1);
    }

    #[test]
    fn resume_reset_starts_a_zero_wave_tranche_at_the_preserved_due_time() {
        let clock = SessionClock::root(moment(3, 0), moment(3, 1))
            .after_fire(moment(3, 4), moment(3, 5))
            .unwrap_or_else(|error| panic!("wave must project: {error:?}"));
        let reset = clock.reset_same_time_tranche(moment(3, 6));

        assert_eq!(reset.now(), clock.now());
        assert_eq!(reset.frontier(), clock.frontier());
        assert_eq!(reset.same_time_tranche().time(), SimTime::from_ticks(3));
        assert_eq!(reset.same_time_tranche().completed_waves(), 0);
        assert_eq!(reset.attempted_wave(moment(3, 6)), 1);
    }

    #[test]
    fn clock_projection_rejects_regression_without_partial_state() {
        let clock = SessionClock::root(moment(5, 2), moment(5, 3));
        assert!(matches!(
            clock.after_fire(moment(5, 1), moment(5, 4)),
            Err(SessionClockProjectionError::MomentRegressed { .. })
        ));
        assert!(matches!(
            clock.after_fire(moment(5, 3), moment(5, 2)),
            Err(SessionClockProjectionError::FrontierRegressed { .. })
        ));
        assert_eq!(clock.now(), moment(5, 2));
        assert_eq!(clock.frontier(), moment(5, 3));
    }

    #[test]
    fn admission_sealing_advances_only_the_frontier() {
        let clock = SessionClock::root(moment(5, 2), moment(5, 3))
            .after_fire(moment(6, 0), moment(6, 1))
            .unwrap_or_else(|error| panic!("clock fixture must project: {error:?}"));
        let sealed = clock
            .seal_admission_through(moment(8, 4))
            .unwrap_or_else(|error| panic!("admission frontier must advance: {error:?}"));

        assert_eq!(sealed.now(), clock.now());
        assert_eq!(sealed.frontier(), moment(8, 4));
        assert_eq!(sealed.same_time_tranche(), clock.same_time_tranche());
    }

    #[test]
    fn admission_sealing_requires_strict_frontier_progress() {
        let clock = SessionClock::root(moment(5, 2), moment(5, 3));

        assert_eq!(
            clock.seal_admission_through(moment(5, 3)),
            Err(SessionClockProjectionError::FrontierNotAdvanced {
                current: moment(5, 3),
                supplied: moment(5, 3),
            })
        );
        assert_eq!(
            clock.seal_admission_through(moment(5, 2)),
            Err(SessionClockProjectionError::FrontierNotAdvanced {
                current: moment(5, 3),
                supplied: moment(5, 2),
            })
        );
        assert_eq!(clock.frontier(), moment(5, 3));
    }

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(
            SimTime::from_ticks(ticks),
            world_core::Microstep::new(microstep),
        )
    }
}
