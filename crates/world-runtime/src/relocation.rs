use std::collections::BTreeMap;

use world_core::{ActorId, SimMoment, SimTime};
use world_model::{
    DirectedRoute, RelocationProcess, RelocationProcessError, RelocationProcessGeneration,
    RelocationProcessId, RelocationProcessStatus, RelocationProcessVersion,
    RelocationWakeGeneration,
};

/// Exact completion wake for one active relocation-process generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RelocationProcessWake {
    process: RelocationProcessId,
    process_generation: RelocationProcessGeneration,
    expected_version: RelocationProcessVersion,
    wake_generation: RelocationWakeGeneration,
    due: SimMoment,
}

impl RelocationProcessWake {
    /// Derives the sole current wake from an active process.
    #[must_use]
    pub(crate) fn for_active(process: RelocationProcess) -> Option<Self> {
        let RelocationProcessStatus::Active {
            due_at,
            wake_generation,
            ..
        } = process.status()
        else {
            return None;
        };
        Some(Self {
            process: process.id(),
            process_generation: process.generation(),
            expected_version: process.version(),
            wake_generation,
            due: SimMoment::at(due_at),
        })
    }

    #[must_use]
    pub(crate) const fn process(self) -> RelocationProcessId {
        self.process
    }

    #[must_use]
    pub(crate) const fn process_generation(self) -> RelocationProcessGeneration {
        self.process_generation
    }

    #[must_use]
    pub(crate) const fn expected_version(self) -> RelocationProcessVersion {
        self.expected_version
    }

    #[must_use]
    pub(crate) const fn wake_generation(self) -> RelocationWakeGeneration {
        self.wake_generation
    }

    #[must_use]
    pub(crate) const fn due(self) -> SimMoment {
        self.due
    }
}

/// Runtime classification of a delivered relocation wake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RelocationWakeClassification {
    Current(RelocationProcessId),
    Obsolete,
}

/// Why the concrete relocation-process ledger rejected a transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RelocationProcessLedgerError {
    LiveProcessExists {
        actor: ActorId,
        process: RelocationProcessId,
    },
    ActorGenerationOverflow {
        actor: ActorId,
    },
    UnknownProcess {
        process: RelocationProcessId,
    },
    ProcessNotLive {
        actor: ActorId,
        process: RelocationProcessId,
    },
    ProcessValueMismatch {
        process: RelocationProcessId,
    },
    InvalidTransition {
        process: RelocationProcessId,
        error: RelocationProcessError,
    },
}

/// Authoritative runtime-control state for the concrete relocation process.
///
/// Completed values remain retained for exact replay, while the actor index
/// admits at most one Active or Paused process.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RelocationProcessLedger {
    processes: BTreeMap<RelocationProcessId, RelocationProcess>,
    live_by_actor: BTreeMap<ActorId, RelocationProcessId>,
    latest_generation: BTreeMap<ActorId, RelocationProcessGeneration>,
}

impl RelocationProcessLedger {
    #[must_use]
    pub(crate) fn get(&self, process: RelocationProcessId) -> Option<RelocationProcess> {
        self.processes.get(&process).copied()
    }

    #[must_use]
    pub(crate) fn live_for(&self, actor: ActorId) -> Option<RelocationProcess> {
        self.live_by_actor
            .get(&actor)
            .and_then(|process| self.processes.get(process))
            .copied()
    }

    #[must_use]
    pub(crate) fn classify_wake(
        &self,
        wake: RelocationProcessWake,
    ) -> RelocationWakeClassification {
        let Some(process) = self.get(wake.process()) else {
            return RelocationWakeClassification::Obsolete;
        };
        let RelocationProcessStatus::Active {
            due_at,
            wake_generation,
            ..
        } = process.status()
        else {
            return RelocationWakeClassification::Obsolete;
        };
        let current = process.generation() == wake.process_generation()
            && process.version() == wake.expected_version()
            && wake_generation == wake.wake_generation()
            && SimMoment::at(due_at) == wake.due()
            && self.live_by_actor.get(&process.actor()) == Some(&process.id());
        if current {
            RelocationWakeClassification::Current(process.id())
        } else {
            RelocationWakeClassification::Obsolete
        }
    }

    pub(crate) fn start(
        &mut self,
        actor: ActorId,
        route: DirectedRoute,
        started_at: SimTime,
    ) -> Result<RelocationProcess, RelocationProcessLedgerError> {
        if let Some(process) = self.live_by_actor.get(&actor).copied() {
            return Err(RelocationProcessLedgerError::LiveProcessExists { actor, process });
        }
        let generation = match self.latest_generation.get(&actor).copied() {
            None => RelocationProcessGeneration::new(0),
            Some(previous) => RelocationProcessGeneration::new(
                previous
                    .get()
                    .checked_add(1)
                    .ok_or(RelocationProcessLedgerError::ActorGenerationOverflow { actor })?,
            ),
        };
        let process =
            RelocationProcess::start(actor, route, generation, started_at).map_err(|error| {
                RelocationProcessLedgerError::InvalidTransition {
                    process: RelocationProcessId::derive(actor, route.id(), generation),
                    error,
                }
            })?;
        if self.processes.insert(process.id(), process).is_some() {
            unreachable!("strictly advancing actor-local generation must produce a fresh process");
        }
        if self.live_by_actor.insert(actor, process.id()).is_some() {
            unreachable!("live-process absence was checked above");
        }
        self.latest_generation.insert(actor, generation);
        Ok(process)
    }

    pub(crate) fn pause(
        &mut self,
        process: RelocationProcessId,
        expected_version: RelocationProcessVersion,
        paused_at: SimTime,
    ) -> Result<(RelocationProcess, RelocationProcess), RelocationProcessLedgerError> {
        let before = self.require_live(process)?;
        let after = before
            .pause(expected_version, paused_at)
            .map_err(|error| RelocationProcessLedgerError::InvalidTransition { process, error })?;
        self.replace_live(before, after)?;
        Ok((before, after))
    }

    pub(crate) fn resume(
        &mut self,
        process: RelocationProcessId,
        expected_version: RelocationProcessVersion,
        resumed_at: SimTime,
    ) -> Result<(RelocationProcess, RelocationProcess), RelocationProcessLedgerError> {
        let before = self.require_live(process)?;
        let after = before
            .resume(expected_version, resumed_at)
            .map_err(|error| RelocationProcessLedgerError::InvalidTransition { process, error })?;
        self.replace_live(before, after)?;
        Ok((before, after))
    }

    pub(crate) fn complete(
        &mut self,
        wake: RelocationProcessWake,
        completed_at: SimTime,
    ) -> Result<(RelocationProcess, RelocationProcess), RelocationProcessLedgerError> {
        let before = self.require_live(wake.process())?;
        if self.classify_wake(wake) != RelocationWakeClassification::Current(before.id()) {
            return Err(RelocationProcessLedgerError::ProcessValueMismatch {
                process: wake.process(),
            });
        }
        let after = before
            .complete(
                wake.expected_version(),
                wake.wake_generation(),
                completed_at,
            )
            .map_err(|error| RelocationProcessLedgerError::InvalidTransition {
                process: wake.process(),
                error,
            })?;
        if self.processes.insert(before.id(), after) != Some(before) {
            return Err(RelocationProcessLedgerError::ProcessValueMismatch {
                process: before.id(),
            });
        }
        if self.live_by_actor.remove(&before.actor()) != Some(before.id()) {
            return Err(RelocationProcessLedgerError::ProcessNotLive {
                actor: before.actor(),
                process: before.id(),
            });
        }
        Ok((before, after))
    }

    fn require_live(
        &self,
        process: RelocationProcessId,
    ) -> Result<RelocationProcess, RelocationProcessLedgerError> {
        let current = self
            .processes
            .get(&process)
            .copied()
            .ok_or(RelocationProcessLedgerError::UnknownProcess { process })?;
        if self.live_by_actor.get(&current.actor()) != Some(&process) {
            return Err(RelocationProcessLedgerError::ProcessNotLive {
                actor: current.actor(),
                process,
            });
        }
        Ok(current)
    }

    fn replace_live(
        &mut self,
        before: RelocationProcess,
        after: RelocationProcess,
    ) -> Result<(), RelocationProcessLedgerError> {
        if before.id() != after.id()
            || before.actor() != after.actor()
            || self.processes.insert(before.id(), after) != Some(before)
        {
            return Err(RelocationProcessLedgerError::ProcessValueMismatch {
                process: before.id(),
            });
        }
        Ok(())
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.processes.is_empty()
            && self.live_by_actor.is_empty()
            && self.latest_generation.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use world_core::{EntityId, SimDuration};

    use super::*;

    fn actor(byte: u8) -> ActorId {
        ActorId::from_bytes([byte; 32])
    }

    fn route() -> DirectedRoute {
        DirectedRoute::new(
            EntityId::from_bytes([0x21; 32]),
            EntityId::from_bytes([0x22; 32]),
            SimDuration::from_ticks(10),
        )
        .unwrap_or_else(|error| panic!("route fixture must be valid: {error}"))
    }

    #[test]
    fn one_live_process_per_actor_and_monotonic_generations() {
        let actor = actor(0x11);
        let mut ledger = RelocationProcessLedger::default();
        let first = ledger
            .start(actor, route(), SimTime::from_ticks(4))
            .unwrap_or_else(|error| panic!("first relocation must start: {error:?}"));
        assert_eq!(first.generation().get(), 0);
        assert!(matches!(
            ledger.start(actor, route(), SimTime::from_ticks(5)),
            Err(RelocationProcessLedgerError::LiveProcessExists { .. })
        ));

        let wake = RelocationProcessWake::for_active(first)
            .unwrap_or_else(|| panic!("active process must derive a wake"));
        let (_, completed) = ledger
            .complete(wake, wake.due().time())
            .unwrap_or_else(|error| panic!("current wake must complete: {error:?}"));
        assert!(matches!(
            completed.status(),
            RelocationProcessStatus::Completed { .. }
        ));

        let second = ledger
            .start(actor, route(), SimTime::from_ticks(20))
            .unwrap_or_else(|error| panic!("successor relocation must start: {error:?}"));
        assert_eq!(second.generation().get(), 1);
        assert_ne!(first.id(), second.id());
    }

    #[test]
    fn pause_and_resume_make_the_old_wake_obsolete() {
        let actor = actor(0x11);
        let mut ledger = RelocationProcessLedger::default();
        let active = ledger
            .start(actor, route(), SimTime::from_ticks(4))
            .unwrap_or_else(|error| panic!("relocation must start: {error:?}"));
        let old = RelocationProcessWake::for_active(active)
            .unwrap_or_else(|| panic!("active process must derive a wake"));
        let (_, paused) = ledger
            .pause(active.id(), active.version(), SimTime::from_ticks(7))
            .unwrap_or_else(|error| panic!("relocation must pause: {error:?}"));
        assert_eq!(
            ledger.classify_wake(old),
            RelocationWakeClassification::Obsolete
        );
        let (_, resumed) = ledger
            .resume(paused.id(), paused.version(), SimTime::from_ticks(20))
            .unwrap_or_else(|error| panic!("relocation must resume: {error:?}"));
        let current = RelocationProcessWake::for_active(resumed)
            .unwrap_or_else(|| panic!("resumed process must derive a wake"));
        assert_eq!(
            ledger.classify_wake(current),
            RelocationWakeClassification::Current(resumed.id())
        );
        assert_eq!(
            ledger.classify_wake(old),
            RelocationWakeClassification::Obsolete
        );
        assert_eq!(
            ledger.complete(old, old.due().time()),
            Err(RelocationProcessLedgerError::ProcessValueMismatch {
                process: old.process(),
            })
        );
        assert_eq!(ledger.live_for(actor), Some(resumed));
        assert_eq!(
            ledger.classify_wake(current),
            RelocationWakeClassification::Current(resumed.id())
        );
    }
}
