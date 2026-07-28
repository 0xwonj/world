use core::{cmp::Ordering, fmt};
use std::collections::{BTreeMap, BTreeSet};

use world_core::{
    CanonicalBytes, CanonicalDomain, CanonicalWriter, ContentDigest, Microstep, SimDuration,
    SimMoment,
};
#[cfg(test)]
use world_model::ContainmentTransferDelta;
use world_model::{
    ActionOpportunity, ActionOpportunityId, ActionOpportunityState, ActionOpportunityVersion,
    CommandEnvelope, PhysicalEvent,
};

use crate::action_evaluation::ActionEvaluationWork;
use crate::authority::{CapturedInputRecordId, ReactionEnvelopeId};
use crate::execution::{EpochLineageId, ExternalInputNamespaceId};
use crate::kernel::{AdmitRequest, InputId, InputRequestFingerprint};
pub use crate::lifecycle::{AttemptResolved, LifecycleWork};
use crate::relocation::RelocationProcessWake;

/// Canonical schema of a command-delivery trigger identity.
pub const COMMAND_DELIVERY_TRIGGER_SCHEMA_VERSION: u16 = 1;

/// Canonical schema of a post-commit dispatch identity.
pub const POST_COMMIT_DISPATCH_SCHEMA_VERSION: u16 = 1;

const COMMAND_DELIVERY_TRIGGER_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("command-delivery-trigger-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("command delivery trigger domain must be valid"),
    };

const POST_COMMIT_DISPATCH_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("post-commit-dispatch-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("post-commit dispatch domain must be valid"),
    };

macro_rules! scheduler_identity {
    ($(#[$metadata:meta])* $name:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Constructs a fixed-width identity decoded by its owning
            /// scheduler protocol.
            #[cfg(test)]
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Returns the exact identity bytes.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }

            /// Consumes the identity and returns its exact bytes.
            #[cfg(test)]
            #[must_use]
            pub const fn into_bytes(self) -> [u8; 32] {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.0 {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}({self})", stringify!($name))
            }
        }
    };
}

scheduler_identity!(
    /// Semantic identity of one captured command-delivery trigger.
    CommandTriggerId
);
scheduler_identity!(
    /// Semantic identity of one post-commit reaction dispatch.
    PostCommitDispatchId
);

impl CommandTriggerId {
    pub(crate) fn derive(
        namespace: ExternalInputNamespaceId,
        input: InputId,
        request: InputRequestFingerprint,
    ) -> Self {
        Self(
            ContentDigest::of_canonical(&command_trigger_bytes(
                namespace,
                input,
                request.as_bytes(),
            ))
            .into_bytes(),
        )
    }
}

impl PostCommitDispatchId {
    fn derive(lineage: EpochLineageId, source_moment: SimMoment) -> Self {
        Self(
            ContentDigest::of_canonical(&post_commit_dispatch_bytes(lineage, source_moment))
                .into_bytes(),
        )
    }
}

/// Stable ordering lane for a scheduled work family.
///
/// Lane order canonicalizes one complete due set. It does not establish
/// visibility between entries: every entry at one moment is prepared from the
/// same immutable base snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchedulerLaneV2 {
    /// Checked external or action-originated command delivery.
    Command,
    /// Reaction routing after an accepted commit.
    PostCommit,
    /// Runtime-owned progress or completion of a time-bearing process.
    Process,
    /// Deterministic evidence, cognition, and agency continuation.
    Lifecycle,
    /// Evaluation of one accepted open action opportunity.
    ActionReady,
    /// Later interpretation or fallback of one retained action evaluation.
    ActionEvaluation,
    /// Outcome-neutral continuation after an action attempt.
    AttemptResolved,
}

impl SchedulerLaneV2 {
    /// Returns the canonical lane tag.
    #[must_use]
    pub const fn canonical_tag(self) -> u32 {
        match self {
            Self::Command => 0,
            Self::PostCommit => 1,
            Self::Process => 2,
            Self::Lifecycle => 3,
            Self::ActionReady => 4,
            Self::ActionEvaluation => 5,
            Self::AttemptResolved => 6,
        }
    }
}

/// Deterministic insertion coordinate within one moment and lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchedulerSequence(u64);

impl SchedulerSequence {
    /// Constructs a sequence from its exact protocol value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the exact protocol value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// Total ordering coordinate of one scheduled work item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchedulerKey {
    moment: SimMoment,
    lane: SchedulerLaneV2,
    sequence: SchedulerSequence,
}

impl SchedulerKey {
    pub(crate) const fn new(
        moment: SimMoment,
        lane: SchedulerLaneV2,
        sequence: SchedulerSequence,
    ) -> Self {
        Self {
            moment,
            lane,
            sequence,
        }
    }

    /// Returns the delivery moment.
    #[must_use]
    pub const fn moment(self) -> SimMoment {
        self.moment
    }

    /// Returns the stable work-family lane.
    #[must_use]
    pub const fn lane(self) -> SchedulerLaneV2 {
        self.lane
    }

    /// Returns the scheduler-owned global insertion sequence.
    #[must_use]
    pub const fn sequence(self) -> SchedulerSequence {
        self.sequence
    }
}

/// Complete command-delivery semantics before authority provenance exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedScheduledCommand {
    trigger: CommandTriggerId,
    input: InputId,
    request: InputRequestFingerprint,
    effective: SimMoment,
    command: CommandEnvelope,
}

impl PreparedScheduledCommand {
    pub(crate) fn prepare(namespace: ExternalInputNamespaceId, request: &AdmitRequest) -> Self {
        Self {
            trigger: CommandTriggerId::derive(namespace, request.id(), request.fingerprint()),
            input: request.id(),
            request: request.fingerprint(),
            effective: request.effective(),
            command: request.command().clone(),
        }
    }

    pub(crate) fn materialize(self, captured: CapturedInputRecordId) -> ScheduledCommand {
        let Self {
            trigger,
            input,
            request,
            effective,
            command,
        } = self;
        ScheduledCommand {
            cause: ScheduledCommandCause::CapturedExternal {
                trigger,
                captured,
                input,
                request,
            },
            effective,
            command,
        }
    }

    pub(crate) const fn trigger(&self) -> CommandTriggerId {
        self.trigger
    }

    pub(crate) const fn input(&self) -> InputId {
        self.input
    }

    pub(crate) const fn request_fingerprint(&self) -> InputRequestFingerprint {
        self.request
    }

    pub(crate) const fn effective(&self) -> SimMoment {
        self.effective
    }

    pub(crate) const fn command(&self) -> &CommandEnvelope {
        &self.command
    }
}

/// Semantic and authority provenance of one scheduled command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScheduledCommandCause {
    /// An admitted external request retained in authority history.
    CapturedExternal {
        /// Semantic delivery-trigger identity.
        trigger: CommandTriggerId,
        /// Authority provenance of the captured request.
        captured: CapturedInputRecordId,
        /// Admitted input identity.
        input: InputId,
        /// Exact admitted-request fingerprint.
        request: InputRequestFingerprint,
    },
    /// A command privately lowered for an accepted action opportunity.
    ActionOpportunity(ActionOpportunityId),
}

impl ScheduledCommandCause {
    /// Returns the external trigger identity when this is captured input.
    #[cfg(test)]
    #[must_use]
    pub const fn trigger(self) -> Option<CommandTriggerId> {
        match self {
            Self::CapturedExternal { trigger, .. } => Some(trigger),
            Self::ActionOpportunity(_) => None,
        }
    }

    /// Returns the captured-input authority provenance when present.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn captured(self) -> Option<CapturedInputRecordId> {
        match self {
            Self::CapturedExternal { captured, .. } => Some(captured),
            Self::ActionOpportunity(_) => None,
        }
    }

    /// Returns the external input identity when present.
    #[cfg(test)]
    #[must_use]
    pub const fn input(self) -> Option<InputId> {
        match self {
            Self::CapturedExternal { input, .. } => Some(input),
            Self::ActionOpportunity(_) => None,
        }
    }

    /// Returns the external request fingerprint when present.
    #[cfg(test)]
    #[must_use]
    pub const fn request_fingerprint(self) -> Option<InputRequestFingerprint> {
        match self {
            Self::CapturedExternal { request, .. } => Some(request),
            Self::ActionOpportunity(_) => None,
        }
    }

    /// Returns the originating action opportunity when present.
    #[must_use]
    pub const fn action_opportunity(self) -> Option<ActionOpportunityId> {
        match self {
            Self::CapturedExternal { .. } => None,
            Self::ActionOpportunity(opportunity) => Some(opportunity),
        }
    }
}

/// Complete command delivery retained until its exact scheduler entry is consumed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledCommand {
    cause: ScheduledCommandCause,
    effective: SimMoment,
    command: CommandEnvelope,
}

impl ScheduledCommand {
    pub(crate) fn from_action_opportunity(
        opportunity: ActionOpportunityId,
        effective: SimMoment,
        command: CommandEnvelope,
    ) -> Self {
        Self {
            cause: ScheduledCommandCause::ActionOpportunity(opportunity),
            effective,
            command,
        }
    }

    /// Returns the complete typed command cause.
    #[must_use]
    pub(crate) const fn cause(&self) -> ScheduledCommandCause {
        self.cause
    }

    /// Returns the external semantic trigger identity when present.
    #[cfg(test)]
    #[must_use]
    pub const fn trigger(&self) -> Option<CommandTriggerId> {
        self.cause.trigger()
    }

    /// Returns captured-input authority provenance when present.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn captured(&self) -> Option<CapturedInputRecordId> {
        self.cause.captured()
    }

    /// Returns the admitted external input identity when present.
    #[cfg(test)]
    #[must_use]
    pub const fn input(&self) -> Option<InputId> {
        self.cause.input()
    }

    /// Returns the external admitted-request fingerprint when present.
    #[cfg(test)]
    #[must_use]
    pub const fn request_fingerprint(&self) -> Option<InputRequestFingerprint> {
        self.cause.request_fingerprint()
    }

    /// Returns the originating action opportunity when present.
    #[must_use]
    pub const fn action_opportunity(&self) -> Option<ActionOpportunityId> {
        self.cause.action_opportunity()
    }

    /// Returns the delivery moment.
    #[must_use]
    pub const fn effective(&self) -> SimMoment {
        self.effective
    }

    /// Returns the complete checked command.
    #[must_use]
    pub const fn command(&self) -> &CommandEnvelope {
        &self.command
    }
}

/// Scheduler input that evaluates one exact open opportunity version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionReady {
    opportunity: ActionOpportunityId,
    expected_version: ActionOpportunityVersion,
    due: SimMoment,
}

impl ActionReady {
    pub(crate) const fn new(
        opportunity: ActionOpportunityId,
        expected_version: ActionOpportunityVersion,
        due: SimMoment,
    ) -> Self {
        Self {
            opportunity,
            expected_version,
            due,
        }
    }

    /// Returns the opportunity to evaluate.
    #[must_use]
    pub const fn opportunity(self) -> ActionOpportunityId {
        self.opportunity
    }

    /// Returns the opportunity version required at delivery.
    #[must_use]
    pub const fn expected_version(self) -> ActionOpportunityVersion {
        self.expected_version
    }

    /// Returns the exact delivery moment.
    #[must_use]
    pub const fn due(self) -> SimMoment {
        self.due
    }
}

/// Ordered nonempty physical events produced by one accepted commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactionEnvelope {
    events: Vec<PhysicalEvent>,
}

impl ReactionEnvelope {
    /// Builds one ordered nonempty reaction envelope from an already-canonical
    /// accepted transfer set.
    #[cfg(test)]
    pub(crate) fn from_transfers(deltas: &[ContainmentTransferDelta]) -> Option<Self> {
        Self::from_events(
            deltas
                .iter()
                .copied()
                .map(PhysicalEvent::item_transferred)
                .collect(),
        )
    }

    /// Builds one ordered reaction envelope from real physical transitions.
    pub(crate) fn from_events(events: Vec<PhysicalEvent>) -> Option<Self> {
        (!events.is_empty()).then_some(Self { events })
    }

    /// Returns physical events in their semantic order.
    #[must_use]
    pub fn events(&self) -> &[PhysicalEvent] {
        &self.events
    }
}

/// Why a strictly later post-commit delivery moment could not be represented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PostCommitScheduleError {
    /// Both the simulation time and its final microstep were exhausted.
    NoStrictlyLaterMoment {
        /// Source moment that cannot be advanced.
        source: SimMoment,
    },
}

/// Complete post-commit dispatch semantics before authority provenance exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedPostCommitDispatch {
    id: PostCommitDispatchId,
    source_moment: SimMoment,
    reaction: ReactionEnvelope,
}

impl PreparedPostCommitDispatch {
    pub(crate) fn prepare(
        lineage: EpochLineageId,
        source_moment: SimMoment,
        reaction: ReactionEnvelope,
    ) -> Self {
        Self {
            id: PostCommitDispatchId::derive(lineage, source_moment),
            source_moment,
            reaction,
        }
    }

    pub(crate) fn materialize(self, reaction_id: ReactionEnvelopeId) -> PostCommitDispatch {
        PostCommitDispatch {
            reaction_id,
            prepared: self,
        }
    }

    pub(crate) const fn id(&self) -> PostCommitDispatchId {
        self.id
    }

    pub(crate) const fn source_moment(&self) -> SimMoment {
        self.source_moment
    }

    pub(crate) const fn reaction(&self) -> &ReactionEnvelope {
        &self.reaction
    }
}

/// Self-contained reaction dispatch scheduled after an accepted commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostCommitDispatch {
    reaction_id: ReactionEnvelopeId,
    prepared: PreparedPostCommitDispatch,
}

impl PostCommitDispatch {
    /// Returns the semantic dispatch identity.
    #[must_use]
    pub const fn id(&self) -> PostCommitDispatchId {
        self.prepared.id()
    }

    /// Returns the authority provenance of the retained reaction envelope.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn reaction_id(&self) -> ReactionEnvelopeId {
        self.reaction_id
    }

    /// Returns the moment of the source batch.
    #[must_use]
    pub const fn source_moment(&self) -> SimMoment {
        self.prepared.source_moment()
    }

    /// Returns the nonempty reaction envelope.
    #[must_use]
    pub const fn reaction(&self) -> &ReactionEnvelope {
        self.prepared.reaction()
    }
}

/// Closed family of scheduled work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduledWork {
    /// One external or action-originated command delivery.
    Command(Box<ScheduledCommand>),
    /// One post-commit reaction dispatch.
    PostCommit(PostCommitDispatch),
    /// One generation-guarded relocation-process wake.
    Process(RelocationProcessWake),
    /// One deterministic lifecycle delivery or recomputation wake.
    Lifecycle(LifecycleWork),
    /// One accepted action opportunity ready for evaluation.
    ActionReady(ActionReady),
    /// One later retained-result or fixed-fallback evaluation.
    ActionEvaluation(ActionEvaluationWork),
}

impl ScheduledWork {
    pub(crate) fn command(command: ScheduledCommand) -> Self {
        Self::Command(Box::new(command))
    }

    pub(crate) const fn lifecycle(work: LifecycleWork) -> Self {
        Self::Lifecycle(work)
    }

    pub(crate) const fn process(wake: RelocationProcessWake) -> Self {
        Self::Process(wake)
    }

    pub(crate) const fn action_ready(ready: ActionReady) -> Self {
        Self::ActionReady(ready)
    }

    pub(crate) const fn action_evaluation(work: ActionEvaluationWork) -> Self {
        Self::ActionEvaluation(work)
    }

    pub(crate) const fn attempt_resolved(resolved: AttemptResolved) -> Self {
        Self::lifecycle(LifecycleWork::AttemptResolved(resolved))
    }
}

/// Producer-owned semantic ordinal for scheduler insertions created by one
/// authority publication.
///
/// The ordinal is assigned before scheduler sequence allocation. It is the
/// final tie-breaker between otherwise identical semantic insertions and must
/// not be derived from persistence identity or proposal arrival order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SchedulerProducerOrdinal(u32);

impl SchedulerProducerOrdinal {
    #[must_use]
    pub(crate) const fn new(value: u32) -> Self {
        Self(value)
    }
}

/// One scheduler insertion proposed by an authority publication before
/// canonical scheduler sequence allocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchedulerInsertion {
    producer_ordinal: SchedulerProducerOrdinal,
    work: ScheduledWork,
}

impl SchedulerInsertion {
    #[must_use]
    pub(crate) const fn new(
        producer_ordinal: SchedulerProducerOrdinal,
        work: ScheduledWork,
    ) -> Self {
        Self {
            producer_ordinal,
            work,
        }
    }
}

/// A canonically ordered, exactly allocated scheduler delta.
///
/// Construction is scheduler-owned so callers cannot attach arbitrary
/// sequences to a multi-entry publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchedulerInsertionPlan {
    entries: Vec<(SchedulerKey, ScheduledWork)>,
}

impl SchedulerInsertionPlan {
    #[must_use]
    pub(crate) fn entries(&self) -> &[(SchedulerKey, ScheduledWork)] {
        &self.entries
    }

    #[cfg(test)]
    #[must_use]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// An owned clone of every scheduler entry at one complete due moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ScheduledMoment {
    moment: SimMoment,
    entries: Vec<(SchedulerKey, ScheduledWork)>,
}

impl ScheduledMoment {
    #[must_use]
    pub(crate) const fn moment(&self) -> SimMoment {
        self.moment
    }

    #[must_use]
    pub(crate) fn entries(&self) -> &[(SchedulerKey, ScheduledWork)] {
        &self.entries
    }
}

/// Why an exact scheduler coordinate could not be planned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SchedulerPlanError {
    NoStrictlyLaterMoment { source: SimMoment },
    SequenceExhausted,
    KeyOccupied { key: SchedulerKey },
}

/// Why a collection of proposed scheduler insertions could not be planned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SchedulerBatchPlanError {
    DuplicateProducerOrdinal { ordinal: SchedulerProducerOrdinal },
    Scheduler(SchedulerPlanError),
}

impl From<SchedulerPlanError> for SchedulerBatchPlanError {
    fn from(error: SchedulerPlanError) -> Self {
        Self::Scheduler(error)
    }
}

/// Why planned work could not be installed at its exact coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SchedulerInstallError {
    SequenceExhausted,
    NoStrictlyLaterMoment {
        source: SimMoment,
    },
    SequenceMismatch {
        expected: SchedulerSequence,
        supplied: SchedulerSequence,
    },
    LaneMismatch {
        expected: SchedulerLaneV2,
        supplied: SchedulerLaneV2,
    },
    MomentMismatch {
        expected: SimMoment,
        supplied: SimMoment,
    },
    KeyOccupied {
        key: SchedulerKey,
    },
}

/// Why accepted origin opportunities could not become initial ready work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionOpportunityScheduleError {
    DuplicateOpportunity {
        opportunity: ActionOpportunityId,
    },
    OpportunityNotOpen {
        opportunity: ActionOpportunityId,
        state: ActionOpportunityState,
    },
    TooManyOpportunities,
    Plan(SchedulerBatchPlanError),
    Install(SchedulerInstallError),
}

/// One ordered authoritative scheduler map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchedulerState {
    next_sequence: Option<SchedulerSequence>,
    entries: BTreeMap<SchedulerKey, ScheduledWork>,
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self::empty()
    }
}

impl SchedulerState {
    #[must_use]
    pub(crate) const fn empty() -> Self {
        Self {
            next_sequence: Some(SchedulerSequence::new(0)),
            entries: BTreeMap::new(),
        }
    }

    pub(crate) fn from_action_opportunities(
        now: SimMoment,
        opportunities: &[ActionOpportunity],
    ) -> Result<Self, ActionOpportunityScheduleError> {
        let mut canonical = opportunities.iter().collect::<Vec<_>>();
        canonical.sort_by_key(|opportunity| opportunity.id());

        if let Some(opportunity) = canonical
            .windows(2)
            .find_map(|pair| (pair[0].id() == pair[1].id()).then_some(pair[0].id()))
        {
            return Err(ActionOpportunityScheduleError::DuplicateOpportunity { opportunity });
        }

        let mut insertions = Vec::with_capacity(canonical.len());
        for (index, opportunity) in canonical.into_iter().enumerate() {
            if opportunity.state() != ActionOpportunityState::Open {
                return Err(ActionOpportunityScheduleError::OpportunityNotOpen {
                    opportunity: opportunity.id(),
                    state: opportunity.state(),
                });
            }
            let ordinal = u32::try_from(index)
                .map_err(|_| ActionOpportunityScheduleError::TooManyOpportunities)?;
            insertions.push(SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(ordinal),
                ScheduledWork::action_ready(ActionReady::new(
                    opportunity.id(),
                    opportunity.version(),
                    now,
                )),
            ));
        }

        let mut scheduler = Self::empty();
        let plan = scheduler
            .plan_batch(insertions)
            .map_err(ActionOpportunityScheduleError::Plan)?;
        scheduler
            .install_batch_exact(plan)
            .map_err(ActionOpportunityScheduleError::Install)?;
        Ok(scheduler)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the least simulation moment represented by any scheduler lane.
    #[must_use]
    pub(crate) fn least_due_moment(&self) -> Option<SimMoment> {
        self.entries.first_key_value().map(|(key, _)| key.moment())
    }

    /// Returns the number of entries across all lanes at one moment.
    #[must_use]
    pub(crate) fn entry_count_at(&self, moment: SimMoment) -> usize {
        self.entries_at(moment).count()
    }

    /// Clones every entry at the least due moment in canonical scheduler order.
    #[must_use]
    pub(crate) fn clone_least_due(&self) -> Option<ScheduledMoment> {
        let moment = self.least_due_moment()?;
        let entries = self
            .entries_at(moment)
            .map(|(key, work)| (*key, work.clone()))
            .collect();
        Some(ScheduledMoment { moment, entries })
    }

    /// Canonicalizes and allocates every scheduler insertion created by one
    /// authority publication.
    ///
    /// Multiple entries may share one moment. Post-commit work is assigned to
    /// the exact strictly later microstep; existing occupancy never shifts
    /// causal time.
    pub(crate) fn plan_batch(
        &self,
        insertions: Vec<SchedulerInsertion>,
    ) -> Result<SchedulerInsertionPlan, SchedulerBatchPlanError> {
        let mut producer_ordinals = BTreeSet::new();
        let mut canonical = Vec::with_capacity(insertions.len());

        for insertion in insertions {
            if !producer_ordinals.insert(insertion.producer_ordinal) {
                return Err(SchedulerBatchPlanError::DuplicateProducerOrdinal {
                    ordinal: insertion.producer_ordinal,
                });
            }
            canonical.push(CanonicalSchedulerInsertion::from_proposed(insertion)?);
        }

        canonical.sort_by(compare_canonical_insertions);

        let mut next_sequence = self.next_sequence;
        let mut entries = Vec::with_capacity(canonical.len());
        for insertion in canonical {
            let sequence = next_sequence.ok_or(SchedulerPlanError::SequenceExhausted)?;
            let key = SchedulerKey::new(insertion.moment, insertion.lane, sequence);
            if self.entries.contains_key(&key) {
                return Err(SchedulerPlanError::KeyOccupied { key }.into());
            }
            entries.push((key, insertion.work));
            next_sequence = sequence.checked_next();
        }

        Ok(SchedulerInsertionPlan { entries })
    }

    /// Installs one previously planned collection without exposing a partially
    /// applied scheduler delta.
    pub(crate) fn install_batch_exact(
        &mut self,
        plan: SchedulerInsertionPlan,
    ) -> Result<(), SchedulerInstallError> {
        let mut expected_sequence = self.next_sequence;
        for (key, work) in &plan.entries {
            let sequence = expected_sequence.ok_or(SchedulerInstallError::SequenceExhausted)?;
            if key.sequence() != sequence {
                return Err(SchedulerInstallError::SequenceMismatch {
                    expected: sequence,
                    supplied: key.sequence(),
                });
            }

            let expected_lane = lane_of(work);
            if key.lane() != expected_lane {
                return Err(SchedulerInstallError::LaneMismatch {
                    expected: expected_lane,
                    supplied: key.lane(),
                });
            }
            if self.entries.contains_key(key) {
                return Err(SchedulerInstallError::KeyOccupied { key: *key });
            }

            let expected_moment = batch_moment_of(work).map_err(
                |PostCommitScheduleError::NoStrictlyLaterMoment { source }| {
                    SchedulerInstallError::NoStrictlyLaterMoment { source }
                },
            )?;
            if key.moment() != expected_moment {
                return Err(SchedulerInstallError::MomentMismatch {
                    expected: expected_moment,
                    supplied: key.moment(),
                });
            }

            expected_sequence = sequence.checked_next();
        }

        for (key, work) in plan.entries {
            if self.entries.insert(key, work).is_some() {
                unreachable!("the complete scheduler batch was validated before insertion");
            }
        }
        self.next_sequence = expected_sequence;
        Ok(())
    }

    #[must_use]
    pub(crate) fn get(&self, key: SchedulerKey) -> Option<&ScheduledWork> {
        self.entries.get(&key)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn first(&self) -> Option<(SchedulerKey, &ScheduledWork)> {
        self.entries
            .first_key_value()
            .map(|(key, work)| (*key, work))
    }

    pub(crate) fn consume_exact(&mut self, key: SchedulerKey) -> Option<ScheduledWork> {
        self.remove_exact(key)
    }

    /// Removes one exact retained entry without treating it as due-work delivery.
    ///
    /// Management uses this path when it supersedes a captured result with a
    /// recorded fallback. The returned payload lets authority application
    /// verify that the removed work is exactly the work named by its record.
    pub(crate) fn remove_exact(&mut self, key: SchedulerKey) -> Option<ScheduledWork> {
        self.entries.remove(&key)
    }

    fn entries_at(
        &self,
        moment: SimMoment,
    ) -> impl Iterator<Item = (&SchedulerKey, &ScheduledWork)> {
        let first = SchedulerKey::new(moment, SchedulerLaneV2::Command, SchedulerSequence::new(0));
        let last = SchedulerKey::new(
            moment,
            SchedulerLaneV2::AttemptResolved,
            SchedulerSequence::new(u64::MAX),
        );
        self.entries.range(first..=last)
    }
}

struct CanonicalSchedulerInsertion {
    moment: SimMoment,
    lane: SchedulerLaneV2,
    producer_ordinal: SchedulerProducerOrdinal,
    work: ScheduledWork,
}

impl CanonicalSchedulerInsertion {
    fn from_proposed(insertion: SchedulerInsertion) -> Result<Self, SchedulerPlanError> {
        let moment = batch_moment_of(&insertion.work).map_err(
            |PostCommitScheduleError::NoStrictlyLaterMoment { source }| {
                SchedulerPlanError::NoStrictlyLaterMoment { source }
            },
        )?;
        Ok(Self {
            moment,
            lane: lane_of(&insertion.work),
            producer_ordinal: insertion.producer_ordinal,
            work: insertion.work,
        })
    }
}

fn lane_of(work: &ScheduledWork) -> SchedulerLaneV2 {
    match work {
        ScheduledWork::Command(_) => SchedulerLaneV2::Command,
        ScheduledWork::PostCommit(_) => SchedulerLaneV2::PostCommit,
        ScheduledWork::Process(_) => SchedulerLaneV2::Process,
        ScheduledWork::Lifecycle(LifecycleWork::AttemptResolved(_)) => {
            SchedulerLaneV2::AttemptResolved
        }
        ScheduledWork::Lifecycle(_) => SchedulerLaneV2::Lifecycle,
        ScheduledWork::ActionReady(_) => SchedulerLaneV2::ActionReady,
        ScheduledWork::ActionEvaluation(_) => SchedulerLaneV2::ActionEvaluation,
    }
}

fn batch_moment_of(work: &ScheduledWork) -> Result<SimMoment, PostCommitScheduleError> {
    match work {
        ScheduledWork::Command(command) => Ok(command.effective()),
        ScheduledWork::PostCommit(dispatch) => strictly_later_moment(dispatch.source_moment()),
        ScheduledWork::Process(wake) => Ok(wake.due()),
        ScheduledWork::Lifecycle(work) => Ok(work.due()),
        ScheduledWork::ActionReady(ready) => Ok(ready.due()),
        ScheduledWork::ActionEvaluation(work) => Ok(work.due()),
    }
}

fn compare_canonical_insertions(
    left: &CanonicalSchedulerInsertion,
    right: &CanonicalSchedulerInsertion,
) -> Ordering {
    left.moment
        .cmp(&right.moment)
        .then_with(|| left.lane.cmp(&right.lane))
        .then_with(|| compare_work_semantics(&left.work, &right.work))
        .then_with(|| left.producer_ordinal.cmp(&right.producer_ordinal))
}

fn compare_work_semantics(left: &ScheduledWork, right: &ScheduledWork) -> Ordering {
    match (left, right) {
        (ScheduledWork::Command(left), ScheduledWork::Command(right)) => left
            .command()
            .source()
            .cmp(&right.command().source())
            .then_with(|| left.command().actor().cmp(&right.command().actor()))
            .then_with(|| left.command().id().cmp(&right.command().id()))
            .then_with(|| {
                left.command()
                    .fingerprint()
                    .cmp(&right.command().fingerprint())
            })
            .then_with(|| compare_command_causes(left.cause(), right.cause())),
        (ScheduledWork::PostCommit(left), ScheduledWork::PostCommit(right)) => left
            .source_moment()
            .cmp(&right.source_moment())
            .then_with(|| left.id().cmp(&right.id()))
            .then_with(|| compare_reactions(left.reaction(), right.reaction())),
        (ScheduledWork::Process(left), ScheduledWork::Process(right)) => left
            .process()
            .cmp(&right.process())
            .then_with(|| left.process_generation().cmp(&right.process_generation()))
            .then_with(|| left.expected_version().cmp(&right.expected_version()))
            .then_with(|| left.wake_generation().cmp(&right.wake_generation())),
        (ScheduledWork::Lifecycle(left), ScheduledWork::Lifecycle(right)) => {
            compare_lifecycle_work(*left, *right)
        }
        (ScheduledWork::ActionReady(left), ScheduledWork::ActionReady(right)) => left
            .opportunity()
            .cmp(&right.opportunity())
            .then_with(|| left.expected_version().cmp(&right.expected_version())),
        (ScheduledWork::ActionEvaluation(left), ScheduledWork::ActionEvaluation(right)) => left
            .opportunity()
            .cmp(&right.opportunity())
            .then_with(|| left.invocation().cmp(&right.invocation()))
            .then_with(|| {
                left.expected_waiting_version()
                    .cmp(&right.expected_waiting_version())
            })
            .then_with(|| left.canonical_tag().cmp(&right.canonical_tag()))
            .then_with(|| left.fallback_cause().cmp(&right.fallback_cause())),
        _ => lane_of(left).cmp(&lane_of(right)),
    }
}

fn compare_lifecycle_work(left: LifecycleWork, right: LifecycleWork) -> Ordering {
    left.canonical_tag()
        .cmp(&right.canonical_tag())
        .then_with(|| match (left, right) {
            (LifecycleWork::EvidenceDelivery(left), LifecycleWork::EvidenceDelivery(right)) => {
                left.evidence().id().cmp(&right.evidence().id())
            }
            (LifecycleWork::Appraisal(left), LifecycleWork::Appraisal(right)) => left
                .actor()
                .cmp(&right.actor())
                .then_with(|| left.generation().get().cmp(&right.generation().get())),
            (LifecycleWork::IntentReview(left), LifecycleWork::IntentReview(right)) => left
                .actor()
                .cmp(&right.actor())
                .then_with(|| left.generation().get().cmp(&right.generation().get())),
            (
                LifecycleWork::ActivityInitialization(left),
                LifecycleWork::ActivityInitialization(right),
            ) => left
                .actor()
                .cmp(&right.actor())
                .then_with(|| left.generation().get().cmp(&right.generation().get())),
            (LifecycleWork::AttemptResolved(left), LifecycleWork::AttemptResolved(right)) => {
                left.opportunity().cmp(&right.opportunity())
            }
            (LifecycleWork::ActivityAdvance(left), LifecycleWork::ActivityAdvance(right)) => left
                .actor()
                .cmp(&right.actor())
                .then_with(|| left.generation().get().cmp(&right.generation().get())),
            _ => Ordering::Equal,
        })
}

fn compare_command_causes(left: ScheduledCommandCause, right: ScheduledCommandCause) -> Ordering {
    match (left, right) {
        (
            ScheduledCommandCause::CapturedExternal {
                trigger: left_trigger,
                input: left_input,
                request: left_request,
                ..
            },
            ScheduledCommandCause::CapturedExternal {
                trigger: right_trigger,
                input: right_input,
                request: right_request,
                ..
            },
        ) => left_trigger
            .cmp(&right_trigger)
            .then_with(|| left_input.cmp(&right_input))
            .then_with(|| left_request.cmp(&right_request)),
        (
            ScheduledCommandCause::ActionOpportunity(left),
            ScheduledCommandCause::ActionOpportunity(right),
        ) => left.cmp(&right),
        (
            ScheduledCommandCause::CapturedExternal { .. },
            ScheduledCommandCause::ActionOpportunity(_),
        ) => Ordering::Less,
        (
            ScheduledCommandCause::ActionOpportunity(_),
            ScheduledCommandCause::CapturedExternal { .. },
        ) => Ordering::Greater,
    }
}

fn compare_reactions(left: &ReactionEnvelope, right: &ReactionEnvelope) -> Ordering {
    for (left, right) in left.events().iter().zip(right.events()) {
        let ordering = compare_physical_events(*left, *right);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.events().len().cmp(&right.events().len())
}

fn compare_physical_events(left: PhysicalEvent, right: PhysicalEvent) -> Ordering {
    match (left, right) {
        (PhysicalEvent::ItemTransferred(left), PhysicalEvent::ItemTransferred(right)) => left
            .actor()
            .cmp(&right.actor())
            .then_with(|| left.item().cmp(&right.item()))
            .then_with(|| left.source().cmp(&right.source()))
            .then_with(|| left.destination().cmp(&right.destination())),
        (PhysicalEvent::ActorDeparted(left), PhysicalEvent::ActorDeparted(right)) => left
            .actor()
            .cmp(&right.actor())
            .then_with(|| left.source().cmp(&right.source()))
            .then_with(|| left.destination().cmp(&right.destination()))
            .then_with(|| left.process().cmp(&right.process())),
        (PhysicalEvent::ActorArrived(left), PhysicalEvent::ActorArrived(right)) => left
            .actor()
            .cmp(&right.actor())
            .then_with(|| left.source().cmp(&right.source()))
            .then_with(|| left.destination().cmp(&right.destination()))
            .then_with(|| left.process().cmp(&right.process())),
        (left, right) => physical_event_tag(left).cmp(&physical_event_tag(right)),
    }
}

const fn physical_event_tag(event: PhysicalEvent) -> u32 {
    match event {
        PhysicalEvent::ItemTransferred(_) => 0,
        PhysicalEvent::ActorDeparted(_) => 1,
        PhysicalEvent::ActorArrived(_) => 2,
    }
}

pub(crate) fn strictly_later_moment(
    source: SimMoment,
) -> Result<SimMoment, PostCommitScheduleError> {
    if let Some(moment) = source.checked_next_microstep() {
        return Ok(moment);
    }
    source
        .time()
        .checked_add(SimDuration::from_ticks(1))
        .map(|time| SimMoment::new(time, Microstep::ZERO))
        .ok_or(PostCommitScheduleError::NoStrictlyLaterMoment { source })
}

fn command_trigger_bytes(
    namespace: ExternalInputNamespaceId,
    input: InputId,
    request_fingerprint: &[u8; 32],
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(COMMAND_DELIVERY_TRIGGER_DOMAIN);
    writer.write_u16(COMMAND_DELIVERY_TRIGGER_SCHEMA_VERSION);
    write_fixed_bytes(&mut writer, namespace.as_bytes());
    writer.write_u64(input.get());
    write_fixed_bytes(&mut writer, request_fingerprint);
    writer.finish()
}

fn post_commit_dispatch_bytes(lineage: EpochLineageId, source_moment: SimMoment) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(POST_COMMIT_DISPATCH_DOMAIN);
    writer.write_u16(POST_COMMIT_DISPATCH_SCHEMA_VERSION);
    write_fixed_bytes(&mut writer, lineage.as_bytes());
    write_moment(&mut writer, source_moment);
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

#[cfg(test)]
mod tests {
    use world_core::{ActorId, EntityId, SimTime};
    use world_model::{
        ActionOpportunityDisposition, ActionOpportunityGeneration, ActionSponsor,
        ActorReactionCause, ContainmentInteractionScope, DirectedRoute, RelocationProcess,
        RelocationProcessGeneration,
    };

    use crate::authority::{
        AuthorityRecordId, CapturedInputLocalIndex, CapturedInputRecordId, ReactionEnvelopeId,
        ReactionLocalIndex,
    };
    use crate::execution::{ExternalInputBindingDigest, ExternalInputNamespaceId};
    use crate::kernel::{derive_input_request_namespace, fixtures};

    use super::*;

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
            encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        encoded
    }

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(ticks), Microstep::new(microstep))
    }

    fn reaction() -> ReactionEnvelope {
        let delta = ContainmentTransferDelta::new(
            ActorId::from_bytes([0x71; 32]),
            EntityId::from_bytes([0x72; 32]),
            EntityId::from_bytes([0x73; 32]),
            EntityId::from_bytes([0x74; 32]),
        )
        .unwrap_or_else(|error| {
            panic!("reaction fixture transfer must be structurally valid: {error}")
        });
        ReactionEnvelope::from_transfers(&[delta])
            .unwrap_or_else(|| panic!("one transfer must produce one reaction envelope"))
    }

    fn namespace() -> ExternalInputNamespaceId {
        derive_input_request_namespace(
            EpochLineageId::from_bytes([0x61; 32]),
            ExternalInputBindingDigest::from_bytes([0x62; 32]),
        )
    }

    fn captured(owner_byte: u8, index: u32) -> CapturedInputRecordId {
        CapturedInputRecordId::derive(
            AuthorityRecordId::from_bytes([owner_byte; 32]),
            CapturedInputLocalIndex::new(index),
        )
    }

    fn reaction_id(owner_byte: u8, index: u32) -> ReactionEnvelopeId {
        ReactionEnvelopeId::derive(
            AuthorityRecordId::from_bytes([owner_byte; 32]),
            ReactionLocalIndex::new(index),
        )
    }

    fn scheduled_command(
        captured: CapturedInputRecordId,
        request: &AdmitRequest,
    ) -> ScheduledCommand {
        PreparedScheduledCommand::prepare(namespace(), request).materialize(captured)
    }

    fn action_opportunity(fixture: u8, generation: u64) -> ActionOpportunity {
        let scope = ContainmentInteractionScope::new(
            EntityId::from_bytes([fixture.wrapping_add(1); 32]),
            vec![EntityId::from_bytes([fixture.wrapping_add(2); 32])],
            vec![EntityId::from_bytes([fixture.wrapping_add(3); 32])],
            8,
        )
        .unwrap_or_else(|error| panic!("action scope fixture must be valid: {error}"));
        ActionOpportunity::open(
            ActorId::from_bytes([fixture; 32]),
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes(
                [fixture.wrapping_add(3); 32],
            )),
            world_model::ActionInteractionScope::containment(scope),
            ActionOpportunityGeneration::new(generation),
        )
    }

    fn post_commit_dispatch(
        lineage: EpochLineageId,
        provenance: ReactionEnvelopeId,
        source: SimMoment,
        envelope: ReactionEnvelope,
    ) -> PostCommitDispatch {
        PreparedPostCommitDispatch::prepare(lineage, source, envelope).materialize(provenance)
    }

    #[test]
    fn scheduler_identity_preimages_are_byte_complete() {
        let trigger_bytes = command_trigger_bytes(
            ExternalInputNamespaceId::from_bytes([0x11; 32]),
            InputId::new(5),
            &[0x22; 32],
        );
        let dispatch_bytes =
            post_commit_dispatch_bytes(EpochLineageId::from_bytes([0x33; 32]), moment(7, 9));

        assert_eq!(
            hex(trigger_bytes.as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d7631000000000000001b636f6d6d616e642d6465",
                "6c69766572792d747269676765722d76310001000000000000002011111111111111111111",
                "11111111111111111111111111111111111111111111000000000000000500000000000000",
                "202222222222222222222222222222222222222222222222222222222222222222"
            )
        );
        assert_eq!(
            ContentDigest::of_canonical(&trigger_bytes).to_string(),
            "6cace21e27aa056852acf94a097f107703e8522105245e9c2623260e04f94b6d"
        );
        assert_eq!(
            hex(dispatch_bytes.as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d76310000000000000017706f73742d636f6d6d69742d6469",
                "7370617463682d763100010000000000000020333333333333333333333333333333333333333333",
                "333333333333333333333300000000000000070000000000000009"
            )
        );
        assert_eq!(
            ContentDigest::of_canonical(&dispatch_bytes).to_string(),
            "0e84dd3aae2171a99e686b8110efd42c0a5cf12cd21ea6647d8573a99058462d"
        );
        let request = AdmitRequest::new(InputId::new(5), moment(2, 0), fixtures::command(0x41, 3));
        let trigger = CommandTriggerId::derive(
            ExternalInputNamespaceId::from_bytes([0x11; 32]),
            request.id(),
            request.fingerprint(),
        );
        assert_eq!(CommandTriggerId::from_bytes(trigger.into_bytes()), trigger);
        let dispatch =
            PostCommitDispatchId::derive(EpochLineageId::from_bytes([0x33; 32]), moment(7, 9));
        assert_eq!(
            PostCommitDispatchId::from_bytes(dispatch.into_bytes()),
            dispatch
        );
    }

    #[test]
    fn origin_action_opportunities_produce_canonical_exact_ready_work() {
        let now = moment(3, 7);
        let first = action_opportunity(0x31, 1);
        let second = action_opportunity(0x41, 1);

        let left = SchedulerState::from_action_opportunities(now, &[second.clone(), first.clone()])
            .unwrap_or_else(|error| panic!("open opportunities must schedule: {error:?}"));
        let right =
            SchedulerState::from_action_opportunities(now, &[first.clone(), second.clone()])
                .unwrap_or_else(|error| {
                    panic!("permuted open opportunities must schedule: {error:?}")
                });

        assert_eq!(left, right);
        assert_eq!(left.least_due_moment(), Some(now));
        assert_eq!(left.entry_count_at(now), 2);

        let ready = left
            .clone_least_due()
            .unwrap_or_else(|| panic!("origin ready work must be due"));
        let mut expected = vec![first.id(), second.id()];
        expected.sort();
        let actual = ready
            .entries()
            .iter()
            .map(|(key, work)| {
                assert_eq!(key.moment(), now);
                assert_eq!(key.lane(), SchedulerLaneV2::ActionReady);
                let ScheduledWork::ActionReady(ready) = work else {
                    panic!("origin opportunities must produce action-ready work");
                };
                assert_eq!(ready.expected_version(), ActionOpportunityVersion::INITIAL);
                assert_eq!(ready.due(), now);
                ready.opportunity()
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn origin_scheduler_rejects_duplicate_and_consumed_opportunities() {
        let opportunity = action_opportunity(0x51, 1);
        assert_eq!(
            SchedulerState::from_action_opportunities(
                SimMoment::ORIGIN,
                &[opportunity.clone(), opportunity.clone()],
            ),
            Err(ActionOpportunityScheduleError::DuplicateOpportunity {
                opportunity: opportunity.id(),
            })
        );

        let consumed = opportunity
            .consume(
                ActionOpportunityVersion::INITIAL,
                ActionOpportunityDisposition::NoApplicableAction,
            )
            .unwrap_or_else(|error| panic!("open opportunity must be consumable: {error}"));
        assert_eq!(
            SchedulerState::from_action_opportunities(
                SimMoment::ORIGIN,
                core::slice::from_ref(&consumed),
            ),
            Err(ActionOpportunityScheduleError::OpportunityNotOpen {
                opportunity: consumed.id(),
                state: consumed.state(),
            })
        );
    }

    #[test]
    fn scheduled_command_cause_exposes_only_applicable_provenance() {
        let due = moment(4, 2);
        let request = AdmitRequest::new(InputId::new(17), due, fixtures::command(0x52, 3));
        let captured = captured(0x53, 0);
        let external = scheduled_command(captured, &request);

        assert!(matches!(
            external.cause(),
            ScheduledCommandCause::CapturedExternal {
                captured: actual,
                input,
                request: fingerprint,
                ..
            } if actual == captured
                && input == request.id()
                && fingerprint == request.fingerprint()
        ));
        assert_eq!(external.captured(), Some(captured));
        assert_eq!(external.input(), Some(request.id()));
        assert_eq!(external.action_opportunity(), None);

        let opportunity = action_opportunity(0x54, 1);
        let action = ScheduledCommand::from_action_opportunity(
            opportunity.id(),
            due,
            request.command().clone(),
        );
        assert_eq!(
            action.cause(),
            ScheduledCommandCause::ActionOpportunity(opportunity.id())
        );
        assert_eq!(action.trigger(), None);
        assert_eq!(action.captured(), None);
        assert_eq!(action.input(), None);
        assert_eq!(action.request_fingerprint(), None);
        assert_eq!(action.action_opportunity(), Some(opportunity.id()));
        assert_eq!(action.effective(), due);
        assert_eq!(action.command(), request.command());
    }

    #[test]
    fn same_moment_work_is_permutation_invariant_across_every_scheduler_lane() {
        let due = moment(9, 0);
        let opportunity = action_opportunity(0x61, 1);
        let request = AdmitRequest::new(InputId::new(23), due, fixtures::command(0x62, 4));
        let command = ScheduledWork::command(scheduled_command(captured(0x63, 0), &request));
        let dispatch = ScheduledWork::PostCommit(post_commit_dispatch(
            EpochLineageId::from_bytes([0x64; 32]),
            reaction_id(0x65, 0),
            moment(8, u64::MAX),
            reaction(),
        ));
        let ready = ScheduledWork::action_ready(ActionReady::new(
            opportunity.id(),
            opportunity.version(),
            due,
        ));
        let (waiting, invocation) = opportunity
            .begin_evaluation(opportunity.version(), [0x66; 32], [0x67; 32])
            .unwrap_or_else(|error| panic!("evaluation fixture must begin: {error}"));
        let evaluation = ScheduledWork::action_evaluation(ActionEvaluationWork::result_ready(
            invocation,
            waiting.id(),
            waiting.version(),
            due,
        ));
        let resolved = ScheduledWork::attempt_resolved(AttemptResolved::new(opportunity.id(), due));
        let lifecycle = ScheduledWork::lifecycle(LifecycleWork::IntentReview(
            crate::lifecycle::IntentReviewWork::new(
                ActorId::from_bytes([0x68; 32]),
                crate::lifecycle::LifecycleGeneration::new(3),
                due,
            ),
        ));
        let route = DirectedRoute::new(
            EntityId::from_bytes([0x69; 32]),
            EntityId::from_bytes([0x6a; 32]),
            SimDuration::from_ticks(9),
        )
        .unwrap_or_else(|error| panic!("process fixture route must be valid: {error}"));
        let process = RelocationProcess::start(
            ActorId::from_bytes([0x6b; 32]),
            route,
            RelocationProcessGeneration::new(1),
            SimTime::ZERO,
        )
        .unwrap_or_else(|error| panic!("process fixture must start: {error}"));
        let process = ScheduledWork::process(
            RelocationProcessWake::for_active(process)
                .unwrap_or_else(|| panic!("active process must retain one exact wake")),
        );
        let mut scheduler = SchedulerState::empty();
        let plan = scheduler
            .plan_batch(vec![
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(6), resolved.clone()),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(5), evaluation.clone()),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(4), ready.clone()),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(3), process.clone()),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(2), lifecycle.clone()),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(1), dispatch.clone()),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(0), command.clone()),
            ])
            .unwrap_or_else(|error| panic!("mixed lifecycle work must plan: {error:?}"));
        let permuted = scheduler
            .plan_batch(vec![
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(0), command),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(2), lifecycle),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(4), ready),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(6), resolved),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(1), dispatch),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(5), evaluation),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(3), process),
            ])
            .unwrap_or_else(|error| panic!("permuted mixed work must plan: {error:?}"));

        assert_eq!(plan, permuted);
        assert!(plan.entries().iter().all(|(key, _)| key.moment() == due));
        assert_eq!(
            plan.entries()
                .iter()
                .map(|(key, _)| key.lane())
                .collect::<Vec<_>>(),
            vec![
                SchedulerLaneV2::Command,
                SchedulerLaneV2::PostCommit,
                SchedulerLaneV2::Process,
                SchedulerLaneV2::Lifecycle,
                SchedulerLaneV2::ActionReady,
                SchedulerLaneV2::ActionEvaluation,
                SchedulerLaneV2::AttemptResolved,
            ]
        );

        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("mixed lifecycle work must install: {error:?}"));
        assert_eq!(scheduler.entry_count_at(due), 7);
    }

    #[test]
    fn management_removal_returns_only_the_exact_retained_work() {
        let due = moment(10, 1);
        let opportunity = action_opportunity(0x68, 1);
        let (waiting, invocation) = opportunity
            .begin_evaluation(opportunity.version(), [0x69; 32], [0x6a; 32])
            .unwrap_or_else(|error| panic!("evaluation fixture must begin: {error}"));
        let work = ScheduledWork::action_evaluation(ActionEvaluationWork::result_ready(
            invocation,
            waiting.id(),
            waiting.version(),
            due,
        ));
        let mut scheduler = SchedulerState::empty();
        let plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                work.clone(),
            )])
            .unwrap_or_else(|error| panic!("result-ready work must plan: {error:?}"));
        let key = plan.entries()[0].0;
        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("result-ready work must install: {error:?}"));
        let wrong = SchedulerKey::new(
            due,
            SchedulerLaneV2::ActionEvaluation,
            SchedulerSequence::new(key.sequence().get() + 1),
        );

        assert_eq!(scheduler.remove_exact(wrong), None);
        assert_eq!(scheduler.get(key), Some(&work));
        assert_eq!(scheduler.remove_exact(key), Some(work));
        assert_eq!(scheduler.get(key), None);
        assert_eq!(scheduler.least_due_moment(), None);
    }

    #[test]
    fn scheduler_identities_commit_every_semantic_coordinate() {
        let namespace = ExternalInputNamespaceId::from_bytes([0x21; 32]);
        let trigger = ContentDigest::of_canonical(&command_trigger_bytes(
            namespace,
            InputId::new(8),
            &[0x22; 32],
        ));
        assert_ne!(
            trigger,
            ContentDigest::of_canonical(&command_trigger_bytes(
                ExternalInputNamespaceId::from_bytes([0x23; 32]),
                InputId::new(8),
                &[0x22; 32],
            ))
        );
        assert_ne!(
            trigger,
            ContentDigest::of_canonical(&command_trigger_bytes(
                namespace,
                InputId::new(9),
                &[0x22; 32],
            ))
        );
        assert_ne!(
            trigger,
            ContentDigest::of_canonical(&command_trigger_bytes(
                namespace,
                InputId::new(8),
                &[0x24; 32],
            ))
        );

        let lineage = EpochLineageId::from_bytes([0x31; 32]);
        let dispatch =
            ContentDigest::of_canonical(&post_commit_dispatch_bytes(lineage, moment(4, 5)));
        let variants = [
            post_commit_dispatch_bytes(EpochLineageId::from_bytes([0x34; 32]), moment(4, 5)),
            post_commit_dispatch_bytes(lineage, moment(4, 6)),
        ];
        for variant in variants {
            assert_ne!(dispatch, ContentDigest::of_canonical(&variant));
        }
    }

    #[test]
    fn batch_planning_canonicalizes_same_moment_commands_before_sequence_allocation() {
        let due = moment(6, 3);
        let first_request = AdmitRequest::new(InputId::new(31), due, fixtures::command(0x41, 1));
        let second_request = AdmitRequest::new(InputId::new(32), due, fixtures::command(0x41, 2));
        let third_request = AdmitRequest::new(InputId::new(33), due, fixtures::command(0x41, 3));
        let first = ScheduledWork::command(scheduled_command(captured(0x81, 0), &first_request));
        let second = ScheduledWork::command(scheduled_command(captured(0x82, 0), &second_request));
        let third = ScheduledWork::command(scheduled_command(captured(0x83, 0), &third_request));
        let scheduler = SchedulerState::empty();

        let left = scheduler
            .plan_batch(vec![
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(2), third.clone()),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(0), first.clone()),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(1), second.clone()),
            ])
            .unwrap_or_else(|error| panic!("same-moment command batch must plan: {error:?}"));
        let right = scheduler
            .plan_batch(vec![
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(1), second),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(0), first),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(2), third),
            ])
            .unwrap_or_else(|error| {
                panic!("permuted same-moment command batch must plan: {error:?}")
            });

        assert_eq!(left, right);
        assert_eq!(left.len(), 3);
        let command_ids: Vec<_> = left
            .entries()
            .iter()
            .map(|(key, work)| {
                assert_eq!(key.moment(), due);
                assert_eq!(key.lane(), SchedulerLaneV2::Command);
                let ScheduledWork::Command(command) = work else {
                    panic!("command insertion must retain command work");
                };
                command.command().id()
            })
            .collect();
        assert_eq!(
            command_ids,
            vec![
                first_request.command().id(),
                second_request.command().id(),
                third_request.command().id(),
            ]
        );
        assert_eq!(
            left.entries()
                .iter()
                .map(|(key, _)| key.sequence().get())
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        let mut installed = scheduler;
        installed
            .install_batch_exact(left)
            .unwrap_or_else(|error| panic!("planned command batch must install: {error:?}"));
        assert_eq!(installed.least_due_moment(), Some(due));
        assert_eq!(installed.entry_count_at(due), 3);
        let cloned = installed
            .clone_least_due()
            .unwrap_or_else(|| panic!("installed due moment must be cloneable"));
        assert_eq!(cloned.moment(), due);
        assert_eq!(cloned.entries().len(), 3);
        assert!(cloned.entries().iter().all(|(key, _)| key.moment() == due));
    }

    #[test]
    fn batch_post_commit_planning_uses_the_exact_strict_successor_even_when_occupied() {
        let source = moment(7, 4);
        let due = moment(7, 5);
        let request = AdmitRequest::new(InputId::new(41), due, fixtures::command(0x42, 1));
        let mut scheduler = SchedulerState::empty();
        let command = scheduled_command(captured(0x84, 0), &request);
        let command_plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(command),
            )])
            .unwrap_or_else(|error| panic!("command publication must plan: {error:?}"));
        scheduler
            .install_batch_exact(command_plan)
            .unwrap_or_else(|error| panic!("command publication must install: {error:?}"));

        let dispatch = post_commit_dispatch(
            EpochLineageId::from_bytes([0x85; 32]),
            reaction_id(0x86, 0),
            source,
            reaction(),
        );
        let plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::PostCommit(dispatch),
            )])
            .unwrap_or_else(|error| panic!("batch dispatch must plan: {error:?}"));
        assert_eq!(plan.entries()[0].0.moment(), due);
        assert_eq!(plan.entries()[0].0.lane(), SchedulerLaneV2::PostCommit);

        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("same-moment dispatch must install: {error:?}"));
        assert_eq!(scheduler.entry_count_at(due), 2);
        let cloned = scheduler
            .clone_least_due()
            .unwrap_or_else(|| panic!("complete mixed-lane moment must exist"));
        assert_eq!(
            cloned
                .entries()
                .iter()
                .map(|(key, _)| key.lane())
                .collect::<Vec<_>>(),
            vec![SchedulerLaneV2::Command, SchedulerLaneV2::PostCommit]
        );
    }

    #[test]
    fn producer_ordinal_breaks_semantic_ties_without_using_provenance_or_input_order() {
        let due = moment(8, 2);
        let request = AdmitRequest::new(InputId::new(51), due, fixtures::command(0x43, 1));
        let lower_provenance = captured(0x11, 0);
        let higher_provenance = captured(0xee, 0);
        let lower = ScheduledWork::command(scheduled_command(lower_provenance, &request));
        let higher = ScheduledWork::command(scheduled_command(higher_provenance, &request));
        let scheduler = SchedulerState::empty();

        let left = scheduler
            .plan_batch(vec![
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(9), lower.clone()),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(2), higher.clone()),
            ])
            .unwrap_or_else(|error| panic!("tied semantic work must plan: {error:?}"));
        let right = scheduler
            .plan_batch(vec![
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(2), higher),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(9), lower),
            ])
            .unwrap_or_else(|error| panic!("permuted tied semantic work must plan: {error:?}"));

        assert_eq!(left, right);
        let ScheduledWork::Command(first) = &left.entries()[0].1 else {
            panic!("first tied insertion must remain command work");
        };
        assert_eq!(first.captured(), Some(higher_provenance));
        assert_eq!(
            scheduler.plan_batch(vec![
                SchedulerInsertion::new(
                    SchedulerProducerOrdinal::new(4),
                    ScheduledWork::command(scheduled_command(lower_provenance, &request)),
                ),
                SchedulerInsertion::new(
                    SchedulerProducerOrdinal::new(4),
                    ScheduledWork::command(scheduled_command(higher_provenance, &request)),
                ),
            ]),
            Err(SchedulerBatchPlanError::DuplicateProducerOrdinal {
                ordinal: SchedulerProducerOrdinal::new(4),
            })
        );
    }

    #[test]
    fn batch_plan_and_install_fail_before_any_partial_scheduler_change() {
        let due = moment(9, 0);
        let first_request = AdmitRequest::new(InputId::new(61), due, fixtures::command(0x44, 1));
        let second_request = AdmitRequest::new(InputId::new(62), due, fixtures::command(0x44, 2));
        let first = ScheduledWork::command(scheduled_command(captured(0x87, 0), &first_request));
        let second = ScheduledWork::command(scheduled_command(captured(0x88, 0), &second_request));
        let exhausted = SchedulerState {
            next_sequence: Some(SchedulerSequence::new(u64::MAX)),
            entries: BTreeMap::new(),
        };

        assert_eq!(
            exhausted.plan_batch(vec![
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(0), first.clone()),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(1), second.clone()),
            ]),
            Err(SchedulerBatchPlanError::Scheduler(
                SchedulerPlanError::SequenceExhausted
            ))
        );
        assert!(exhausted.is_empty());
        assert_eq!(
            exhausted.next_sequence,
            Some(SchedulerSequence::new(u64::MAX))
        );

        let mut scheduler = SchedulerState::empty();
        let stale = scheduler
            .plan_batch(vec![
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(0), first),
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(1), second),
            ])
            .unwrap_or_else(|error| panic!("batch fixture must plan: {error:?}"));
        let intervening_request =
            AdmitRequest::new(InputId::new(63), moment(10, 0), fixtures::command(0x45, 1));
        let intervening = scheduled_command(captured(0x89, 0), &intervening_request);
        let intervening_plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(intervening),
            )])
            .unwrap_or_else(|error| panic!("intervening command publication must plan: {error:?}"));
        scheduler
            .install_batch_exact(intervening_plan)
            .unwrap_or_else(|error| {
                panic!("intervening command publication must install: {error:?}")
            });
        let before = scheduler.clone();

        assert_eq!(
            scheduler.install_batch_exact(stale),
            Err(SchedulerInstallError::SequenceMismatch {
                expected: SchedulerSequence::new(1),
                supplied: SchedulerSequence::new(0),
            })
        );
        assert_eq!(scheduler, before);
    }

    #[test]
    fn scheduler_queries_and_consumes_exact_allocated_keys() {
        let early = AdmitRequest::new(InputId::new(2), moment(2, 0), fixtures::command(0x51, 1));
        let late = AdmitRequest::new(InputId::new(1), moment(5, 0), fixtures::command(0x52, 2));
        let dispatch = post_commit_dispatch(
            EpochLineageId::from_bytes([0x53; 32]),
            reaction_id(0x54, 0),
            SimMoment::ORIGIN,
            reaction(),
        );
        let mut scheduler = SchedulerState::empty();

        let late_command = scheduled_command(captured(0x55, 0), &late);
        let early_command = scheduled_command(captured(0x56, 0), &early);
        let plan = scheduler
            .plan_batch(vec![
                SchedulerInsertion::new(
                    SchedulerProducerOrdinal::new(0),
                    ScheduledWork::command(late_command),
                ),
                SchedulerInsertion::new(
                    SchedulerProducerOrdinal::new(1),
                    ScheduledWork::command(early_command),
                ),
                SchedulerInsertion::new(
                    SchedulerProducerOrdinal::new(2),
                    ScheduledWork::PostCommit(dispatch),
                ),
            ])
            .unwrap_or_else(|error| panic!("scheduler publication must plan: {error:?}"));
        let late_key = plan
            .entries()
            .iter()
            .find_map(|(key, work)| match work {
                ScheduledWork::Command(command) if command.input() == Some(late.id()) => Some(*key),
                _ => None,
            })
            .unwrap_or_else(|| panic!("late command key must be allocated"));
        let early_key = plan
            .entries()
            .iter()
            .find_map(|(key, work)| match work {
                ScheduledWork::Command(command) if command.input() == Some(early.id()) => {
                    Some(*key)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("early command key must be allocated"));
        let dispatch_key = plan
            .entries()
            .iter()
            .find_map(|(key, work)| match work {
                ScheduledWork::PostCommit(_) => Some(*key),
                _ => None,
            })
            .unwrap_or_else(|| panic!("dispatch key must be allocated"));
        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("scheduler publication must install: {error:?}"));

        assert!(matches!(
            scheduler.get(early_key),
            Some(ScheduledWork::Command(command)) if command.input() == Some(early.id())
        ));
        assert!(scheduler.get(late_key).is_some());
        assert_eq!(scheduler.first().map(|(key, _)| key), Some(dispatch_key));
        assert!(matches!(
            scheduler.consume_exact(dispatch_key),
            Some(ScheduledWork::PostCommit(_))
        ));
        assert!(matches!(
            scheduler.consume_exact(early_key),
            Some(ScheduledWork::Command(_))
        ));
        assert!(matches!(
            scheduler.consume_exact(late_key),
            Some(ScheduledWork::Command(_))
        ));
    }

    #[test]
    fn scheduler_reports_sequence_exhaustion_without_reusing_a_coordinate() {
        let first = AdmitRequest::new(InputId::new(1), moment(7, 0), fixtures::command(0x61, 1));
        let second = AdmitRequest::new(InputId::new(2), moment(8, 0), fixtures::command(0x62, 2));
        let mut scheduler = SchedulerState {
            next_sequence: Some(SchedulerSequence::new(u64::MAX)),
            entries: BTreeMap::new(),
        };

        let command = scheduled_command(captured(0x63, 0), &first);
        let final_plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(command),
            )])
            .unwrap_or_else(|error| panic!("the final sequence remains allocatable: {error:?}"));
        let final_key = final_plan.entries()[0].0;
        scheduler
            .install_batch_exact(final_plan)
            .unwrap_or_else(|error| panic!("the final sequence remains installable: {error:?}"));
        assert_eq!(final_key.sequence().get(), u64::MAX);
        let second_command = scheduled_command(captured(0x64, 0), &second);
        assert_eq!(
            scheduler.plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(second_command),
            )]),
            Err(SchedulerBatchPlanError::Scheduler(
                SchedulerPlanError::SequenceExhausted
            ))
        );
        assert!(scheduler.get(final_key).is_some());
    }

    #[test]
    fn batch_planning_is_nonmutating_and_rejects_mismatched_installations() {
        let due = moment(9, 2);
        let request = AdmitRequest::new(InputId::new(1), due, fixtures::command(0x65, 1));
        let command = scheduled_command(captured(0x66, 0), &request);
        let mut scheduler = SchedulerState::empty();

        let planned = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(command.clone()),
            )])
            .unwrap_or_else(|error| {
                panic!("an empty scheduler must plan the command publication: {error:?}")
            });
        let planned_key = planned.entries()[0].0;
        assert_eq!(
            scheduler.plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(command.clone()),
            )]),
            Ok(planned.clone()),
            "planning must not consume a sequence"
        );

        let wrong_lane =
            SchedulerKey::new(due, SchedulerLaneV2::PostCommit, planned_key.sequence());
        assert_eq!(
            scheduler.install_batch_exact(SchedulerInsertionPlan {
                entries: vec![(wrong_lane, ScheduledWork::command(command.clone()))],
            }),
            Err(SchedulerInstallError::LaneMismatch {
                expected: SchedulerLaneV2::Command,
                supplied: SchedulerLaneV2::PostCommit,
            })
        );

        let wrong_moment = SchedulerKey::new(
            moment(9, 3),
            SchedulerLaneV2::Command,
            planned_key.sequence(),
        );
        assert_eq!(
            scheduler.install_batch_exact(SchedulerInsertionPlan {
                entries: vec![(wrong_moment, ScheduledWork::command(command.clone()))],
            }),
            Err(SchedulerInstallError::MomentMismatch {
                expected: due,
                supplied: moment(9, 3),
            })
        );

        scheduler
            .install_batch_exact(planned)
            .unwrap_or_else(|error| {
                panic!("the exact planned command publication must install: {error:?}")
            });

        let later = moment(10, 0);
        let later_request = AdmitRequest::new(InputId::new(2), later, fixtures::command(0x67, 2));
        let later_command = scheduled_command(captured(0x68, 0), &later_request);
        assert_eq!(
            scheduler.install_batch_exact(SchedulerInsertionPlan {
                entries: vec![(planned_key, ScheduledWork::command(later_command.clone()),)],
            }),
            Err(SchedulerInstallError::SequenceMismatch {
                expected: SchedulerSequence::new(1),
                supplied: SchedulerSequence::new(0),
            })
        );
        let later_plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(later_command),
            )])
            .unwrap_or_else(|error| {
                panic!("failed installation must preserve the allocator: {error:?}")
            });
        assert_eq!(
            later_plan.entries()[0].0.sequence(),
            SchedulerSequence::new(1)
        );
    }

    #[test]
    fn batch_planning_reports_exact_key_occupancy_without_shifting_moments() {
        let due = moment(11, 0);
        let request = AdmitRequest::new(InputId::new(3), due, fixtures::command(0x69, 3));
        let retained = scheduled_command(captured(0x6a, 0), &request);
        let occupied = SchedulerKey::new(due, SchedulerLaneV2::Command, SchedulerSequence::new(0));
        let mut entries = BTreeMap::new();
        entries.insert(occupied, ScheduledWork::command(retained.clone()));
        let mut scheduler = SchedulerState {
            next_sequence: Some(SchedulerSequence::new(0)),
            entries,
        };

        let dispatch = post_commit_dispatch(
            EpochLineageId::from_bytes([0x6b; 32]),
            reaction_id(0x6c, 0),
            moment(10, u64::MAX),
            reaction(),
        );
        let post_commit_plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::PostCommit(dispatch),
            )])
            .unwrap_or_else(|error| {
                panic!("post-commit planning must retain the exact successor: {error:?}")
            });
        let post_commit_key = post_commit_plan.entries()[0].0;
        assert_eq!(post_commit_key.moment(), due);
        assert_eq!(post_commit_key.lane(), SchedulerLaneV2::PostCommit);
        assert_eq!(post_commit_key.sequence(), SchedulerSequence::new(0));
        assert_eq!(
            scheduler.plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(retained.clone()),
            )]),
            Err(SchedulerBatchPlanError::Scheduler(
                SchedulerPlanError::KeyOccupied { key: occupied }
            ))
        );
        assert_eq!(
            scheduler.install_batch_exact(SchedulerInsertionPlan {
                entries: vec![(occupied, ScheduledWork::command(retained.clone()))],
            }),
            Err(SchedulerInstallError::KeyOccupied { key: occupied })
        );
        assert!(matches!(
            scheduler.get(occupied),
            Some(ScheduledWork::Command(command)) if command.as_ref() == &retained
        ));
        assert_eq!(scheduler.next_sequence, Some(SchedulerSequence::new(0)));
    }

    #[test]
    fn scheduler_planning_owns_strict_successor_exhaustion() {
        assert_eq!(strictly_later_moment(moment(4, 7)), Ok(moment(4, 8)));
        assert_eq!(strictly_later_moment(moment(4, u64::MAX)), Ok(moment(5, 0)));
        let terminal = moment(u64::MAX, u64::MAX);
        assert_eq!(
            strictly_later_moment(terminal),
            Err(PostCommitScheduleError::NoStrictlyLaterMoment { source: terminal })
        );
        let prepared = PreparedPostCommitDispatch::prepare(
            EpochLineageId::from_bytes([0x71; 32]),
            terminal,
            reaction(),
        );
        assert_eq!(prepared.source_moment(), terminal);
        let dispatch = prepared.materialize(reaction_id(0x72, 0));
        assert_eq!(
            SchedulerState::empty().plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::PostCommit(dispatch),
            )]),
            Err(SchedulerBatchPlanError::Scheduler(
                SchedulerPlanError::NoStrictlyLaterMoment { source: terminal }
            ))
        );
    }

    #[test]
    fn reaction_envelopes_are_nonempty_and_transfer_derived() {
        let envelope = reaction();
        assert_eq!(envelope.events().len(), 1);
        assert!(matches!(
            envelope.events()[0],
            PhysicalEvent::ItemTransferred(_)
        ));
    }

    #[test]
    fn combined_reaction_preserves_canonical_transfer_order() {
        let first = ContainmentTransferDelta::new(
            ActorId::from_bytes([0x31; 32]),
            EntityId::from_bytes([0x32; 32]),
            EntityId::from_bytes([0x33; 32]),
            EntityId::from_bytes([0x34; 32]),
        )
        .unwrap_or_else(|error| panic!("first transfer must be valid: {error}"));
        let second = ContainmentTransferDelta::new(
            ActorId::from_bytes([0x41; 32]),
            EntityId::from_bytes([0x42; 32]),
            EntityId::from_bytes([0x43; 32]),
            EntityId::from_bytes([0x44; 32]),
        )
        .unwrap_or_else(|error| panic!("second transfer must be valid: {error}"));

        assert_eq!(ReactionEnvelope::from_transfers(&[]), None);
        let combined = ReactionEnvelope::from_transfers(&[first, second])
            .unwrap_or_else(|| panic!("nonempty transfers must produce one envelope"));
        assert_eq!(
            combined.events(),
            &[
                PhysicalEvent::item_transferred(first),
                PhysicalEvent::item_transferred(second),
            ]
        );
    }

    #[test]
    fn post_commit_dispatch_retains_semantic_identity_and_reaction_provenance() {
        let provenance = reaction_id(0x6c, 0);
        let envelope = reaction();
        let prepared = PreparedPostCommitDispatch::prepare(
            EpochLineageId::from_bytes([0x6d; 32]),
            moment(12, 4),
            envelope.clone(),
        );
        let prepared_id = prepared.id();
        let dispatch = prepared.materialize(provenance);

        assert_eq!(dispatch.id(), prepared_id);
        assert_eq!(dispatch.reaction_id(), provenance);
        assert_eq!(dispatch.reaction(), &envelope);
        assert_eq!(dispatch.source_moment(), moment(12, 4));
    }

    #[test]
    fn scheduled_command_retains_the_complete_admission_binding() {
        let request = AdmitRequest::new(InputId::new(7), moment(8, 3), fixtures::command(0x71, 11));
        let captured = captured(0x72, 0);
        let mut scheduler = SchedulerState::empty();
        let command = scheduled_command(captured, &request);
        let plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(command),
            )])
            .unwrap_or_else(|error| panic!("command publication must plan: {error:?}"));
        let key = plan.entries()[0].0;
        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("command publication must install: {error:?}"));
        let ScheduledWork::Command(command) = scheduler
            .get(key)
            .unwrap_or_else(|| panic!("scheduled command must be retained"))
        else {
            panic!("command key must retain command work");
        };

        assert_eq!(command.captured(), Some(captured));
        assert_eq!(command.input(), Some(request.id()));
        assert_eq!(command.request_fingerprint(), Some(request.fingerprint()));
        assert_eq!(command.effective(), request.effective());
        assert_eq!(command.command(), request.command());
        assert_eq!(
            command.trigger(),
            Some(CommandTriggerId::derive(
                namespace(),
                request.id(),
                request.fingerprint()
            ))
        );
    }
}
