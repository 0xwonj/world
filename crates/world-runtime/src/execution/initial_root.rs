use core::fmt;
use std::collections::BTreeMap;

use world_core::{
    ActorId, CanonicalBytes, CanonicalDomain, CanonicalError, CanonicalWriter, SimMoment,
};
use world_model::{
    AcceptedState, AcceptedStateDigest, ActionOpportunity, ActionOpportunityId,
    ActionOpportunityState, ActionSponsor, ActivityId, ActivityStatus, ActivityVersion,
};

#[cfg(test)]
use crate::authority::AuthorityCursor;
use crate::session::SessionMode;

#[cfg(test)]
use super::ChildEpochTransform;
use super::{EpochLineage, EpochLineageId, InitialStateRootId};

/// Canonical schema of the execution-independent initial semantic body.
pub const INITIAL_ROOT_SEMANTIC_BODY_SCHEMA_VERSION: u16 = 1;

/// Canonical schema of the complete initial state root.
pub const INITIAL_STATE_ROOT_SCHEMA_VERSION: u16 = 1;

const INITIAL_ROOT_SEMANTIC_BODY_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("initial-root-semantic-body-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("initial root semantic body domain must be valid"),
    };

const INITIAL_STATE_ROOT_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("initial-state-root-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("initial state root domain must be valid"),
    };

/// Why an initial state root could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialRootError {
    /// The admission frontier was earlier than the current simulation moment.
    FrontierBeforeNow {
        /// Current simulation moment.
        now: SimMoment,
        /// Proposed admission frontier.
        frontier: SimMoment,
    },
    /// More than one opportunity used the same durable identity.
    DuplicateActionOpportunity {
        /// Reused opportunity identity.
        opportunity: ActionOpportunityId,
    },
    /// Initial opportunity state was already terminal.
    NonOpenActionOpportunity {
        /// Invalid opportunity identity.
        opportunity: ActionOpportunityId,
        /// Supplied terminal state.
        state: ActionOpportunityState,
    },
    /// One actor had more than one foreground opportunity.
    MultipleOpenActionOpportunities {
        /// Actor with conflicting foreground control.
        actor: ActorId,
        /// First opportunity in canonical identity order.
        first: ActionOpportunityId,
        /// Second opportunity in canonical identity order.
        second: ActionOpportunityId,
    },
    /// An activity-sponsored opportunity named no accepted activity.
    MissingSponsoringActivity {
        /// Opportunity whose sponsor could not be resolved.
        opportunity: ActionOpportunityId,
        /// Missing activity identity.
        activity: ActivityId,
    },
    /// An activity-sponsored opportunity belonged to a different actor.
    SponsoringActivityActorMismatch {
        /// Opportunity with the inconsistent owner.
        opportunity: ActionOpportunityId,
        /// Sponsoring activity identity.
        activity: ActivityId,
    },
    /// An activity-sponsored opportunity named a stale activity version.
    SponsoringActivityVersionMismatch {
        /// Opportunity with the stale sponsor.
        opportunity: ActionOpportunityId,
        /// Sponsoring activity identity.
        activity: ActivityId,
        /// Version captured by the opportunity.
        expected: ActivityVersion,
        /// Accepted activity version.
        actual: ActivityVersion,
    },
    /// An activity-sponsored opportunity named an activity that cannot act.
    SponsoringActivityNotActive {
        /// Opportunity with the inactive sponsor.
        opportunity: ActionOpportunityId,
        /// Sponsoring activity identity.
        activity: ActivityId,
        /// Accepted activity status.
        status: ActivityStatus,
    },
    /// An activity-sponsored opportunity did not belong to the actor's
    /// foreground activity.
    SponsoringActivityNotFocused {
        /// Opportunity whose sponsor was not focused.
        opportunity: ActionOpportunityId,
        /// Sponsoring activity identity.
        activity: ActivityId,
        /// Activity currently focused by the actor, if any.
        focused: Option<ActivityId>,
    },
    /// An activity-sponsored opportunity did not match the exact action
    /// represented by the activity's retained state.
    SponsoringActivityOpeningMismatch {
        /// Opportunity with the inconsistent method action.
        opportunity: ActionOpportunityId,
        /// Sponsoring activity identity.
        activity: ActivityId,
    },
    /// The canonical opportunity sequence length does not fit its protocol scalar.
    TooManyActionOpportunities {
        /// Supplied opportunity count.
        count: usize,
    },
}

impl fmt::Display for InitialRootError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrontierBeforeNow { now, frontier } => write!(
                formatter,
                "admission frontier {frontier:?} is earlier than current moment {now:?}"
            ),
            Self::DuplicateActionOpportunity { opportunity } => {
                write!(formatter, "duplicate action opportunity {opportunity}")
            }
            Self::NonOpenActionOpportunity { opportunity, state } => {
                write!(
                    formatter,
                    "initial action opportunity {opportunity} is not open: {state:?}"
                )
            }
            Self::MultipleOpenActionOpportunities {
                actor,
                first,
                second,
            } => write!(
                formatter,
                "actor {} has multiple open action opportunities {first} and {second}",
                Hex(actor.as_bytes())
            ),
            Self::MissingSponsoringActivity {
                opportunity,
                activity,
            } => write!(
                formatter,
                "action opportunity {opportunity} names missing sponsoring activity {activity}"
            ),
            Self::SponsoringActivityActorMismatch {
                opportunity,
                activity,
            } => write!(
                formatter,
                "action opportunity {opportunity} actor does not own sponsoring activity {activity}"
            ),
            Self::SponsoringActivityVersionMismatch {
                opportunity,
                activity,
                expected,
                actual,
            } => write!(
                formatter,
                "action opportunity {opportunity} expects sponsoring activity {activity} at version {expected:?}, but accepted version is {actual:?}"
            ),
            Self::SponsoringActivityNotActive {
                opportunity,
                activity,
                status,
            } => write!(
                formatter,
                "action opportunity {opportunity} names sponsoring activity {activity} in non-active state {status:?}"
            ),
            Self::SponsoringActivityNotFocused {
                opportunity,
                activity,
                focused,
            } => write!(
                formatter,
                "action opportunity {opportunity} names non-focused sponsoring activity {activity}; actor focus is {focused:?}"
            ),
            Self::SponsoringActivityOpeningMismatch {
                opportunity,
                activity,
            } => write!(
                formatter,
                "action opportunity {opportunity} does not match sponsoring activity {activity}'s retained opening"
            ),
            Self::TooManyActionOpportunities { count } => write!(
                formatter,
                "initial action opportunity count {count} exceeds the u32 protocol limit"
            ),
        }
    }
}

impl std::error::Error for InitialRootError {}

/// Complete immutable state from which one execution epoch can begin.
///
/// The root retains the accepted state rather than trusting an independently
/// supplied state digest. Its identity excludes every child execution
/// specification identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitialStateRootV1 {
    lineage: EpochLineage,
    mode: SessionMode,
    now: SimMoment,
    admission_frontier: SimMoment,
    accepted_state: AcceptedState,
    action_opportunities: Vec<ActionOpportunity>,
    id: InitialStateRootId,
}

impl InitialStateRootV1 {
    pub(crate) fn origin(
        mode: SessionMode,
        now: SimMoment,
        admission_frontier: SimMoment,
        accepted_state: AcceptedState,
        action_opportunities: Vec<ActionOpportunity>,
    ) -> Result<Self, InitialRootError> {
        validate_frontier(now, admission_frontier)?;
        let action_opportunities =
            validate_action_opportunities(&accepted_state, action_opportunities)?;
        let semantic_body = initial_root_semantic_body_bytes(
            mode,
            now,
            admission_frontier,
            accepted_state.digest(),
            &action_opportunities,
        );
        let lineage = EpochLineage::origin(&semantic_body);
        Ok(Self::from_checked_parts(
            lineage,
            mode,
            now,
            admission_frontier,
            accepted_state,
            action_opportunities,
            semantic_body,
        ))
    }

    #[cfg(test)]
    pub(crate) fn child(
        parent_cursor: AuthorityCursor,
        transform: ChildEpochTransform,
        mode: SessionMode,
        now: SimMoment,
        admission_frontier: SimMoment,
        accepted_state: AcceptedState,
        action_opportunities: Vec<ActionOpportunity>,
    ) -> Result<Self, InitialRootError> {
        validate_frontier(now, admission_frontier)?;
        let action_opportunities =
            validate_action_opportunities(&accepted_state, action_opportunities)?;
        let semantic_body = initial_root_semantic_body_bytes(
            mode,
            now,
            admission_frontier,
            accepted_state.digest(),
            &action_opportunities,
        );
        let lineage = EpochLineage::child(parent_cursor, transform);
        Ok(Self::from_checked_parts(
            lineage,
            mode,
            now,
            admission_frontier,
            accepted_state,
            action_opportunities,
            semantic_body,
        ))
    }

    fn from_checked_parts(
        lineage: EpochLineage,
        mode: SessionMode,
        now: SimMoment,
        admission_frontier: SimMoment,
        accepted_state: AcceptedState,
        action_opportunities: Vec<ActionOpportunity>,
        semantic_body: CanonicalBytes,
    ) -> Self {
        let bytes = initial_state_root_bytes(lineage, &semantic_body);
        Self {
            lineage,
            mode,
            now,
            admission_frontier,
            accepted_state,
            action_opportunities,
            id: InitialStateRootId::of_canonical(&bytes),
        }
    }

    /// Returns the complete semantic lineage.
    #[must_use]
    pub const fn lineage(&self) -> EpochLineage {
        self.lineage
    }

    /// Returns the semantic lineage identity.
    #[must_use]
    pub const fn lineage_id(&self) -> EpochLineageId {
        self.lineage.id()
    }

    /// Returns the starting session mode.
    #[must_use]
    pub const fn mode(&self) -> SessionMode {
        self.mode
    }

    /// Returns the starting simulation moment.
    #[must_use]
    pub const fn now(&self) -> SimMoment {
        self.now
    }

    /// Returns the first admissible command frontier.
    #[must_use]
    pub const fn admission_frontier(&self) -> SimMoment {
        self.admission_frontier
    }

    /// Returns the complete immutable accepted state.
    #[must_use]
    pub const fn accepted_state(&self) -> &AcceptedState {
        &self.accepted_state
    }

    /// Returns the accepted-state identity recomputed by the model owner.
    #[must_use]
    pub fn accepted_state_digest(&self) -> AcceptedStateDigest {
        self.accepted_state.digest()
    }

    /// Returns initial action opportunities in canonical identity order.
    #[must_use]
    pub fn action_opportunities(&self) -> &[ActionOpportunity] {
        &self.action_opportunities
    }

    /// Returns the complete initial-root identity.
    #[must_use]
    pub const fn id(&self) -> InitialStateRootId {
        self.id
    }

    pub(crate) fn canonical_bytes(&self) -> CanonicalBytes {
        let semantic_body = self.semantic_body_bytes();
        initial_state_root_bytes(self.lineage, &semantic_body)
    }

    pub(crate) fn semantic_body_bytes(&self) -> CanonicalBytes {
        initial_root_semantic_body_bytes(
            self.mode,
            self.now,
            self.admission_frontier,
            self.accepted_state.digest(),
            &self.action_opportunities,
        )
    }
}

fn validate_frontier(now: SimMoment, frontier: SimMoment) -> Result<(), InitialRootError> {
    if frontier < now {
        return Err(InitialRootError::FrontierBeforeNow { now, frontier });
    }
    Ok(())
}

fn validate_action_opportunities(
    accepted_state: &AcceptedState,
    mut opportunities: Vec<ActionOpportunity>,
) -> Result<Vec<ActionOpportunity>, InitialRootError> {
    if u32::try_from(opportunities.len()).is_err() {
        return Err(InitialRootError::TooManyActionOpportunities {
            count: opportunities.len(),
        });
    }
    opportunities.sort_by_key(ActionOpportunity::id);
    if let Some(opportunity) = opportunities
        .windows(2)
        .find_map(|pair| (pair[0].id() == pair[1].id()).then_some(pair[0].id()))
    {
        return Err(InitialRootError::DuplicateActionOpportunity { opportunity });
    }

    let mut open_by_actor = BTreeMap::<ActorId, ActionOpportunityId>::new();
    for opportunity in &opportunities {
        if opportunity.state() != ActionOpportunityState::Open {
            return Err(InitialRootError::NonOpenActionOpportunity {
                opportunity: opportunity.id(),
                state: opportunity.state(),
            });
        }
        if let Some(first) = open_by_actor.insert(opportunity.actor(), opportunity.id()) {
            return Err(InitialRootError::MultipleOpenActionOpportunities {
                actor: opportunity.actor(),
                first,
                second: opportunity.id(),
            });
        }
        let ActionSponsor::Activity(sponsor) = opportunity.sponsor() else {
            continue;
        };
        let Some(activity) = accepted_state
            .agency()
            .activity(sponsor.activity())
            .copied()
        else {
            return Err(InitialRootError::MissingSponsoringActivity {
                opportunity: opportunity.id(),
                activity: sponsor.activity(),
            });
        };
        if activity.actor() != opportunity.actor() {
            return Err(InitialRootError::SponsoringActivityActorMismatch {
                opportunity: opportunity.id(),
                activity: activity.id(),
            });
        }
        if activity.version() != sponsor.expected_version() {
            return Err(InitialRootError::SponsoringActivityVersionMismatch {
                opportunity: opportunity.id(),
                activity: activity.id(),
                expected: sponsor.expected_version(),
                actual: activity.version(),
            });
        }
        if activity.status() != ActivityStatus::Active {
            return Err(InitialRootError::SponsoringActivityNotActive {
                opportunity: opportunity.id(),
                activity: activity.id(),
                status: activity.status(),
            });
        }
        let focused = accepted_state
            .agency()
            .focused_activity(opportunity.actor());
        if focused != Some(activity.id()) {
            return Err(InitialRootError::SponsoringActivityNotFocused {
                opportunity: opportunity.id(),
                activity: activity.id(),
                focused,
            });
        }
        if !opportunity.matches_activity_opening(activity) {
            return Err(InitialRootError::SponsoringActivityOpeningMismatch {
                opportunity: opportunity.id(),
                activity: activity.id(),
            });
        }
    }

    Ok(opportunities)
}

fn initial_root_semantic_body_bytes(
    mode: SessionMode,
    now: SimMoment,
    admission_frontier: SimMoment,
    accepted_state: AcceptedStateDigest,
    action_opportunities: &[ActionOpportunity],
) -> CanonicalBytes {
    let encoded = (|| -> Result<_, CanonicalError> {
        let mut writer = CanonicalWriter::new(INITIAL_ROOT_SEMANTIC_BODY_DOMAIN);
        writer.write_u16(INITIAL_ROOT_SEMANTIC_BODY_SCHEMA_VERSION);
        writer.write_discriminant(mode.canonical_tag());
        write_moment(&mut writer, now);
        write_moment(&mut writer, admission_frontier);
        writer.write_bytes(accepted_state.as_bytes())?;
        writer.write_sequence(action_opportunities, |writer, opportunity| {
            writer.write_bytes(opportunity.digest().as_bytes())
        })?;
        Ok(writer.finish())
    })();
    match encoded {
        Ok(bytes) => bytes,
        Err(error) => unreachable!(
            "allocated initial action opportunities must fit the canonical protocol: {error}"
        ),
    }
}

fn initial_state_root_bytes(
    lineage: EpochLineage,
    semantic_body: &CanonicalBytes,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(INITIAL_STATE_ROOT_DOMAIN);
    writer.write_u16(INITIAL_STATE_ROOT_SCHEMA_VERSION);
    write_owned_bytes(&mut writer, lineage.canonical_bytes().as_bytes());
    write_fixed_bytes(&mut writer, lineage.id().as_bytes());
    write_owned_bytes(&mut writer, semantic_body.as_bytes());
    writer.finish()
}

fn write_moment(writer: &mut CanonicalWriter, moment: SimMoment) {
    writer.write_u64(moment.time().ticks());
    writer.write_u64(moment.microstep().get());
}

fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width identity length must fit the canonical protocol");
    }
}

fn write_owned_bytes(writer: &mut CanonicalWriter, bytes: &[u8]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("owned canonical bytes must fit the canonical protocol");
    }
}

struct Hex<'a>(&'a [u8]);

impl fmt::Display for Hex<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use world_core::{ActorId, EntityId, Microstep, SimTime};
    use world_model::{
        ActionInteractionScope, ActionOpportunityDisposition, ActionOpportunityGeneration,
        ActionOpportunityVersion, ActionSponsor, Activity, ActivityControllerId, ActivityFocus,
        ActivityGeneration, ActivityStateSchemaId, ActivityTransition, ActorReactionCause,
        AgencyState, ContainerRecord, ContainmentInteractionScope, ContainmentRecord,
        ContainmentTransferActivityState, DesiredCondition, DomainState, EpistemicState, Intent,
        IntentGeneration, SocialState,
    };

    use crate::authority::{AuthorityCursor, EpochIdentity};

    use super::*;
    use crate::execution::{BranchTransformId, EpochLineageBody, ExecutionSpecId};

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(ticks), Microstep::new(microstep))
    }

    fn accepted_with_agency(item_byte: u8, agency: AgencyState) -> AcceptedState {
        let container = EntityId::from_bytes([0x21; 32]);
        let destination = EntityId::from_bytes([0x22; 32]);
        let domain = match DomainState::new(
            vec![
                ContainerRecord::new(container, 1),
                ContainerRecord::new(destination, 1),
            ],
            vec![ContainmentRecord::new(
                EntityId::from_bytes([item_byte; 32]),
                container,
            )],
            Vec::new(),
        ) {
            Ok(state) => state,
            Err(error) => panic!("root fixture state must be valid: {error}"),
        };
        AcceptedState::new(
            domain,
            EpistemicState::empty(),
            SocialState::empty(),
            agency,
        )
    }

    fn accepted(item_byte: u8) -> AcceptedState {
        accepted_with_agency(item_byte, AgencyState::empty())
    }

    fn activity_fixture(actor: ActorId) -> (Intent, Activity) {
        let item = EntityId::from_bytes([0x31; 32]);
        let source = EntityId::from_bytes([0x21; 32]);
        let destination = EntityId::from_bytes([0x22; 32]);
        let intent = Intent::adopt(
            actor,
            IntentGeneration::new(1).unwrap_or_else(|| panic!("fixture generation is nonzero")),
            DesiredCondition::item_contained_in(item, destination),
        );
        let state = ContainmentTransferActivityState::new(
            item,
            source,
            destination,
            ActionOpportunityGeneration::new(1),
            2,
        )
        .and_then(ContainmentTransferActivityState::after_opening_opportunity)
        .unwrap_or_else(|error| panic!("fixture activity state must be valid: {error}"));
        let activity = Activity::start(
            actor,
            intent.id(),
            ActivityGeneration::new(1).unwrap_or_else(|| panic!("fixture generation is nonzero")),
            ActivityControllerId::from_bytes([0x71; 32]),
            ActivityStateSchemaId::from_bytes([0x72; 32]),
            state,
        );
        (intent, activity)
    }

    fn accepted_with_activity(intent: Intent, activity: Activity) -> AcceptedState {
        let focus = (activity.status() == ActivityStatus::Active)
            .then(|| ActivityFocus::new(activity.actor(), activity.id()))
            .into_iter()
            .collect();
        let agency = AgencyState::new(vec![intent], vec![activity], focus)
            .unwrap_or_else(|error| panic!("fixture agency state must be valid: {error}"));
        accepted_with_agency(0x31, agency)
    }

    fn activity_action(actor: ActorId, activity: Activity) -> ActionOpportunity {
        let scope = ContainmentInteractionScope::new(
            EntityId::from_bytes([0x21; 32]),
            vec![EntityId::from_bytes([0x22; 32])],
            vec![EntityId::from_bytes([0x31; 32])],
            1,
        )
        .unwrap_or_else(|error| panic!("root action scope must be valid: {error}"));
        ActionOpportunity::open(
            actor,
            ActionSponsor::activity(activity.id(), activity.version()),
            ActionInteractionScope::containment(scope),
            ActionOpportunityGeneration::new(1),
        )
    }

    fn action(
        actor_byte: u8,
        cause_byte: u8,
        source_byte: u8,
        destination_byte: u8,
        candidate_limit: u32,
        generation: u64,
    ) -> ActionOpportunity {
        let scope = ContainmentInteractionScope::new(
            EntityId::from_bytes([source_byte; 32]),
            vec![EntityId::from_bytes([destination_byte; 32])],
            vec![EntityId::from_bytes([source_byte.wrapping_add(1); 32])],
            candidate_limit,
        )
        .unwrap_or_else(|error| panic!("root action scope must be valid: {error}"));
        ActionOpportunity::open(
            ActorId::from_bytes([actor_byte; 32]),
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([cause_byte; 32])),
            world_model::ActionInteractionScope::containment(scope),
            ActionOpportunityGeneration::new(generation),
        )
    }

    #[test]
    fn root_rejects_a_frontier_before_now() {
        assert_eq!(
            InitialStateRootV1::origin(
                SessionMode::Running,
                moment(3, 1),
                moment(3, 0),
                accepted(0x31),
                Vec::new(),
            ),
            Err(InitialRootError::FrontierBeforeNow {
                now: moment(3, 1),
                frontier: moment(3, 0),
            })
        );
    }

    #[test]
    fn origin_root_identity_covers_semantic_state_and_lineage() {
        let first = InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted(0x31),
            Vec::new(),
        );
        let first = match first {
            Ok(root) => root,
            Err(error) => panic!("root fixture must be valid: {error}"),
        };
        let changed_state = InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted(0x32),
            Vec::new(),
        );
        let changed_state = match changed_state {
            Ok(root) => root,
            Err(error) => panic!("root fixture must be valid: {error}"),
        };
        let changed_mode = InitialStateRootV1::origin(
            SessionMode::Paused,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted(0x31),
            Vec::new(),
        );
        let changed_mode = match changed_mode {
            Ok(root) => root,
            Err(error) => panic!("root fixture must be valid: {error}"),
        };

        assert_ne!(first.id(), changed_state.id());
        assert_ne!(first.lineage_id(), changed_state.lineage_id());
        assert_ne!(first.id(), changed_mode.id());
        assert_eq!(
            first.id(),
            InitialStateRootId::of_canonical(&first.canonical_bytes())
        );
        assert_eq!(
            first.accepted_state_digest(),
            first.accepted_state().digest()
        );
    }

    #[test]
    fn root_canonicalizes_opportunities_and_covers_complete_open_semantics() {
        let first_action = action(0x40, 0x50, 0x10, 0x20, 8, 1);
        let second_action = action(0x41, 0x51, 0x11, 0x21, 9, 2);
        let canonical = InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted(0x31),
            vec![first_action.clone(), second_action.clone()],
        )
        .unwrap_or_else(|error| panic!("canonical opportunity root must be valid: {error}"));
        let reversed = InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted(0x31),
            vec![second_action, first_action.clone()],
        )
        .unwrap_or_else(|error| panic!("reversed opportunity root must be valid: {error}"));

        assert_eq!(canonical, reversed);
        assert!(
            canonical
                .action_opportunities()
                .windows(2)
                .all(|pair| pair[0].id() < pair[1].id())
        );

        let changed_scope = action(0x40, 0x50, 0x10, 0x22, 9, 1);
        assert_eq!(first_action.id(), changed_scope.id());
        let changed_scope_root = InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted(0x31),
            vec![changed_scope],
        )
        .unwrap_or_else(|error| panic!("changed scope root must be valid: {error}"));
        let first_only_root = InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted(0x31),
            vec![first_action.clone()],
        )
        .unwrap_or_else(|error| panic!("first-only root must be valid: {error}"));
        assert_ne!(first_only_root.id(), changed_scope_root.id());
    }

    #[test]
    fn root_rejects_duplicate_terminal_and_multi_foreground_opportunities() {
        let first = action(0x40, 0x50, 0x10, 0x20, 8, 1);
        assert_eq!(
            InitialStateRootV1::origin(
                SessionMode::Running,
                SimMoment::ORIGIN,
                SimMoment::ORIGIN,
                accepted(0x31),
                vec![first.clone(), first.clone()],
            ),
            Err(InitialRootError::DuplicateActionOpportunity {
                opportunity: first.id(),
            })
        );

        let consumed = first
            .consume(
                ActionOpportunityVersion::INITIAL,
                ActionOpportunityDisposition::NoApplicableAction,
            )
            .unwrap_or_else(|error| panic!("fixture opportunity must consume: {error}"));
        assert_eq!(
            InitialStateRootV1::origin(
                SessionMode::Running,
                SimMoment::ORIGIN,
                SimMoment::ORIGIN,
                accepted(0x31),
                vec![consumed.clone()],
            ),
            Err(InitialRootError::NonOpenActionOpportunity {
                opportunity: consumed.id(),
                state: consumed.state(),
            })
        );

        let second = action(0x40, 0x51, 0x11, 0x21, 8, 2);
        let mut ordered = [first.clone(), second.clone()];
        ordered.sort_by_key(ActionOpportunity::id);
        assert_eq!(
            InitialStateRootV1::origin(
                SessionMode::Running,
                SimMoment::ORIGIN,
                SimMoment::ORIGIN,
                accepted(0x31),
                vec![second, first],
            ),
            Err(InitialRootError::MultipleOpenActionOpportunities {
                actor: ActorId::from_bytes([0x40; 32]),
                first: ordered[0].id(),
                second: ordered[1].id(),
            })
        );
    }

    #[test]
    fn root_requires_an_exact_focused_activity_opening() {
        let actor = ActorId::from_bytes([0x40; 32]);
        let other_actor = ActorId::from_bytes([0x41; 32]);
        let (intent, activity) = activity_fixture(actor);
        let accepted = accepted_with_activity(intent, activity);

        let valid = activity_action(actor, activity);
        InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted.clone(),
            vec![valid],
        )
        .unwrap_or_else(|error| panic!("exact activity sponsor must be accepted: {error}"));

        let unfocused_state = accepted_with_agency(
            0x31,
            AgencyState::new(vec![intent], vec![activity], Vec::new())
                .unwrap_or_else(|error| panic!("unfocused agency state must be valid: {error}")),
        );
        let unfocused = activity_action(actor, activity);
        assert_eq!(
            InitialStateRootV1::origin(
                SessionMode::Running,
                SimMoment::ORIGIN,
                SimMoment::ORIGIN,
                unfocused_state,
                vec![unfocused.clone()],
            ),
            Err(InitialRootError::SponsoringActivityNotFocused {
                opportunity: unfocused.id(),
                activity: activity.id(),
                focused: None,
            })
        );

        let wrong_generation = ActionOpportunity::open(
            actor,
            ActionSponsor::activity(activity.id(), activity.version()),
            activity_action(actor, activity).interaction_scope().clone(),
            ActionOpportunityGeneration::new(2),
        );
        assert_eq!(
            InitialStateRootV1::origin(
                SessionMode::Running,
                SimMoment::ORIGIN,
                SimMoment::ORIGIN,
                accepted.clone(),
                vec![wrong_generation.clone()],
            ),
            Err(InitialRootError::SponsoringActivityOpeningMismatch {
                opportunity: wrong_generation.id(),
                activity: activity.id(),
            })
        );

        let wrong_scope = ContainmentInteractionScope::new(
            EntityId::from_bytes([0x21; 32]),
            vec![EntityId::from_bytes([0x23; 32])],
            vec![EntityId::from_bytes([0x31; 32])],
            1,
        )
        .unwrap_or_else(|error| {
            panic!("mismatched fixture scope must still be structural: {error}")
        });
        let wrong_method_action = ActionOpportunity::open(
            actor,
            ActionSponsor::activity(activity.id(), activity.version()),
            ActionInteractionScope::containment(wrong_scope),
            ActionOpportunityGeneration::new(1),
        );
        assert_eq!(
            InitialStateRootV1::origin(
                SessionMode::Running,
                SimMoment::ORIGIN,
                SimMoment::ORIGIN,
                accepted.clone(),
                vec![wrong_method_action.clone()],
            ),
            Err(InitialRootError::SponsoringActivityOpeningMismatch {
                opportunity: wrong_method_action.id(),
                activity: activity.id(),
            })
        );

        let missing_activity = ActivityId::from_bytes([0x81; 32]);
        let missing = ActionOpportunity::open(
            actor,
            ActionSponsor::activity(missing_activity, ActivityVersion::INITIAL),
            activity_action(actor, activity).interaction_scope().clone(),
            ActionOpportunityGeneration::new(2),
        );
        assert_eq!(
            InitialStateRootV1::origin(
                SessionMode::Running,
                SimMoment::ORIGIN,
                SimMoment::ORIGIN,
                accepted.clone(),
                vec![missing.clone()],
            ),
            Err(InitialRootError::MissingSponsoringActivity {
                opportunity: missing.id(),
                activity: missing_activity,
            })
        );

        let wrong_actor = activity_action(other_actor, activity);
        assert_eq!(
            InitialStateRootV1::origin(
                SessionMode::Running,
                SimMoment::ORIGIN,
                SimMoment::ORIGIN,
                accepted.clone(),
                vec![wrong_actor.clone()],
            ),
            Err(InitialRootError::SponsoringActivityActorMismatch {
                opportunity: wrong_actor.id(),
                activity: activity.id(),
            })
        );

        let stale_version =
            ActivityVersion::new(2).unwrap_or_else(|| panic!("fixture version is nonzero"));
        let stale = ActionOpportunity::open(
            actor,
            ActionSponsor::activity(activity.id(), stale_version),
            activity_action(actor, activity).interaction_scope().clone(),
            ActionOpportunityGeneration::new(3),
        );
        assert_eq!(
            InitialStateRootV1::origin(
                SessionMode::Running,
                SimMoment::ORIGIN,
                SimMoment::ORIGIN,
                accepted,
                vec![stale.clone()],
            ),
            Err(InitialRootError::SponsoringActivityVersionMismatch {
                opportunity: stale.id(),
                activity: activity.id(),
                expected: stale_version,
                actual: ActivityVersion::INITIAL,
            })
        );

        let suspended = activity
            .transition(ActivityVersion::INITIAL, ActivityTransition::Suspend)
            .unwrap_or_else(|error| panic!("fixture activity must suspend: {error}"));
        let inactive = activity_action(actor, suspended);
        assert_eq!(
            InitialStateRootV1::origin(
                SessionMode::Running,
                SimMoment::ORIGIN,
                SimMoment::ORIGIN,
                accepted_with_activity(intent, suspended),
                vec![inactive.clone()],
            ),
            Err(InitialRootError::SponsoringActivityNotActive {
                opportunity: inactive.id(),
                activity: suspended.id(),
                status: ActivityStatus::Suspended,
            })
        );
    }

    #[test]
    fn child_root_retains_the_exact_parent_cursor_lineage() {
        let parent = AuthorityCursor::root(
            EpochIdentity::new(
                EpochLineageId::from_bytes([0x41; 32]),
                ExecutionSpecId::from_bytes([0x42; 32]),
            ),
            InitialStateRootId::from_bytes([0x43; 32]),
        );
        let child = InitialStateRootV1::child(
            parent,
            ChildEpochTransform::Branch(BranchTransformId::from_bytes([0x44; 32])),
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted(0x31),
            Vec::new(),
        );
        let child = match child {
            Ok(root) => root,
            Err(error) => panic!("child root fixture must be valid: {error}"),
        };

        let EpochLineageBody::Child {
            parent_execution,
            parent_cursor,
            ..
        } = child.lineage().body()
        else {
            panic!("child root must retain child lineage");
        };
        assert_eq!(parent_execution, parent.epoch().execution());
        assert_eq!(parent_cursor, parent);
    }
}
