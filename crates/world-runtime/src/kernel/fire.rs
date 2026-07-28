use std::collections::{BTreeMap, BTreeSet};

use world_core::{ActorId, SimMoment};
use world_model::{
    ActionOpportunity, ActionOpportunityDisposition, ActionOpportunityId, ActionOpportunityVersion,
    Activity, ActivityId, ActivityVersion, CommandAttemptOutcome, CommandEnvelope, CommandId,
    CommandRequestFingerprint, CommandSource, ContainmentAppraisal, ContainmentTransferDelta,
    EpistemicState, EpistemicVersion, EvidenceRecord, Intent, IntentVersion, RelocationInteraction,
    RelocationProcessId, StableCommandRejection, WorldSnapshot,
};

use crate::action_evaluation::{
    ActionEvaluationArtifactSchemaId, ActionEvaluationInvocationRecord,
    ActionEvaluationInvocationState, ActionEvaluationResultFreshness, ActionEvaluationResultReady,
    ActionEvaluationWork,
};
use crate::attempt::{
    AttemptAuthorityDomainId, AttemptStepId, ReservationGrant, RunAttemptId, RunFinalization,
};
use crate::authority::{AttemptRecordId, AuthorityCursor, AuthorityRecordId};
use crate::execution::ExecutionSpecId;
use crate::lifecycle::{
    ActivityAdvanceWork, ActivityInitializationWork, AppraisalWork, EvidenceDeliveryWork,
    EvidenceObservation, IntentReviewWork,
};
use crate::relocation::{RelocationProcessWake, RelocationWakeClassification};
use crate::scheduler::{
    ActionReady, AttemptResolved, PostCommitDispatch, PostCommitDispatchId, ScheduledCommand,
    ScheduledWork, SchedulerKey, SchedulerLaneV2,
};

/// Bound for one attempt to fire the complete globally least due moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireRequest {
    through: SimMoment,
}

impl FireRequest {
    /// Constructs an inclusive simulation-time bound.
    #[must_use]
    pub const fn through(moment: SimMoment) -> Self {
        Self { through: moment }
    }

    /// Returns the inclusive due-work bound.
    #[must_use]
    pub const fn through_moment(self) -> SimMoment {
        self.through
    }
}

/// One transient canonical artifact supplied while opening a deferred action
/// evaluation.
///
/// The bytes exist only in the prepared moment proposal. Runtime applies the
/// execution closure's role-specific bound before authority retention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeferredActionArtifactInput {
    schema: ActionEvaluationArtifactSchemaId,
    bytes: Vec<u8>,
}

impl DeferredActionArtifactInput {
    /// Binds canonical bytes to their exact codec schema.
    #[must_use]
    pub const fn new(schema: ActionEvaluationArtifactSchemaId, bytes: Vec<u8>) -> Self {
        Self { schema, bytes }
    }

    /// Returns the exact codec schema.
    #[must_use]
    pub const fn schema(&self) -> ActionEvaluationArtifactSchemaId {
        self.schema
    }

    /// Returns the transient canonical bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn into_parts(self) -> (ActionEvaluationArtifactSchemaId, Vec<u8>) {
        (self.schema, self.bytes)
    }
}

/// Complete transient input for opening one retained deferred action
/// evaluation.
///
/// Opportunity identity, versions, execution control, implementation,
/// authority provenance, and timing are deliberately runtime-derived.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeferredActionInvocationInput {
    policy_semantics: [u8; 32],
    action_input_fingerprint: [u8; 32],
    request: DeferredActionArtifactInput,
    result_schema: ActionEvaluationArtifactSchemaId,
    private_continuation: DeferredActionArtifactInput,
    private_read_witness: DeferredActionArtifactInput,
}

impl DeferredActionInvocationInput {
    /// Constructs one complete transient deferred-evaluation input.
    #[must_use]
    pub const fn new(
        policy_semantics: [u8; 32],
        action_input_fingerprint: [u8; 32],
        request: DeferredActionArtifactInput,
        result_schema: ActionEvaluationArtifactSchemaId,
        private_continuation: DeferredActionArtifactInput,
        private_read_witness: DeferredActionArtifactInput,
    ) -> Self {
        Self {
            policy_semantics,
            action_input_fingerprint,
            request,
            result_schema,
            private_continuation,
            private_read_witness,
        }
    }

    #[must_use]
    pub const fn policy_semantics(&self) -> [u8; 32] {
        self.policy_semantics
    }

    #[must_use]
    pub const fn action_input_fingerprint(&self) -> [u8; 32] {
        self.action_input_fingerprint
    }

    #[must_use]
    pub const fn request(&self) -> &DeferredActionArtifactInput {
        &self.request
    }

    #[must_use]
    pub const fn result_schema(&self) -> ActionEvaluationArtifactSchemaId {
        self.result_schema
    }

    #[must_use]
    pub const fn private_continuation(&self) -> &DeferredActionArtifactInput {
        &self.private_continuation
    }

    #[must_use]
    pub const fn private_read_witness(&self) -> &DeferredActionArtifactInput {
        &self.private_read_witness
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        [u8; 32],
        [u8; 32],
        DeferredActionArtifactInput,
        ActionEvaluationArtifactSchemaId,
        DeferredActionArtifactInput,
        DeferredActionArtifactInput,
    ) {
        (
            self.policy_semantics,
            self.action_input_fingerprint,
            self.request,
            self.result_schema,
            self.private_continuation,
            self.private_read_witness,
        )
    }
}

/// Existing action semantics selected by one validated deferred result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvaluatedAction {
    /// Submit one privately lowered command.
    Submit(Box<CommandEnvelope>),
    /// Submit one privately grounded relocation interaction.
    Relocate(RelocationInteraction),
    /// Finish because the policy selected no applicable action.
    NoApplicableAction,
}

impl EvaluatedAction {
    /// Constructs a command submission without exposing the private proposal
    /// representation.
    #[must_use]
    pub fn submit(command: CommandEnvelope) -> Self {
        Self::Submit(Box::new(command))
    }
}

/// Result-side failures that may require the configured later fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionEvaluationResultFailure {
    /// Captured bytes did not decode or validate for the original request.
    InvalidResult,
    /// Actor-visible input changed after the fixed reinvocation budget.
    VisibleReinvocationExhausted,
}

/// Closed engine decision for one retained captured action result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionEvaluationDecision {
    /// Apply the retained result through ordinary checked action semantics.
    Apply {
        /// Positive freshness evidence established by private rebuilding.
        freshness: ActionEvaluationResultFreshness,
        /// Existing action semantics selected by the result.
        action: EvaluatedAction,
    },
    /// Discard the old result and open one linked visible-input successor.
    Reinvoke(Box<DeferredActionInvocationInput>),
    /// Schedule the execution closure's fixed failure fallback.
    RequireFallback(ActionEvaluationResultFailure),
}

impl ActionEvaluationDecision {
    /// Constructs a linked visible-input reinvocation without exposing storage shape.
    #[must_use]
    pub fn reinvoke(input: DeferredActionInvocationInput) -> Self {
        Self::Reinvoke(Box::new(input))
    }
}

/// Opaque correlation identity for one engine-facing work item.
///
/// The identity is meaningful only inside the [`PreparedFire`] that issued
/// it. Runtime owns construction and does not expose its representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkId {
    step: AttemptStepId,
    position: u32,
}

impl WorkId {
    fn from_position(step: AttemptStepId, position: usize) -> Option<Self> {
        u32::try_from(position)
            .ok()
            .map(|position| Self { step, position })
    }
}

/// Immutable, capability-scoped input for one item in a prepared moment.
#[derive(Clone, Copy, Debug)]
pub enum MomentWorkInput<'a> {
    /// One genuinely new logical command evaluated against the shared base.
    EvaluateCommand {
        /// Opaque correlation identity.
        work: WorkId,
        /// Exact due moment.
        due: SimMoment,
        /// Immutable authoritative base snapshot.
        snapshot: &'a WorldSnapshot,
        /// Complete checked command.
        command: &'a CommandEnvelope,
    },
    /// One self-contained post-commit reaction dispatch.
    PostCommitDispatch {
        /// Opaque correlation identity.
        work: WorkId,
        /// Exact due moment.
        due: SimMoment,
        /// Complete dispatch captured by the authoritative scheduler.
        dispatch: &'a PostCommitDispatch,
    },
    /// One canonical actor-local batch of due evidence deliveries.
    EvidenceAssimilation {
        /// Opaque correlation identity.
        work: WorkId,
        /// Exact due moment.
        due: SimMoment,
        /// Immutable authoritative base snapshot.
        snapshot: &'a WorldSnapshot,
        /// Actor whose epistemic history owns the complete batch.
        actor: ActorId,
        /// Due evidence in actor-local generation order.
        evidence: &'a [EvidenceRecord],
    },
    /// One coalesced appraisal generation over accepted evidence.
    Appraisal {
        /// Opaque correlation identity.
        work: WorkId,
        /// Exact due moment.
        due: SimMoment,
        /// Immutable authoritative base snapshot.
        snapshot: &'a WorldSnapshot,
        /// Actor receiving the appraisal.
        actor: ActorId,
        /// Actor-local coalescing generation.
        generation: u64,
        /// Accepted evidence causes retained for this generation.
        evidence: &'a [EvidenceRecord],
        /// Previously retained appraisal values for affected subjects.
        previous: &'a [ContainmentAppraisal],
    },
    /// One coalesced intent review over materially changed appraisals.
    IntentReview {
        /// Opaque correlation identity.
        work: WorkId,
        /// Exact due moment.
        due: SimMoment,
        /// Immutable authoritative base snapshot.
        snapshot: &'a WorldSnapshot,
        /// Actor whose intent is being reconsidered.
        actor: ActorId,
        /// Actor-local coalescing generation.
        generation: u64,
        /// Exact materially changed appraisal causes.
        appraisals: &'a [ContainmentAppraisal],
    },
    /// One coalesced initialization request for accepted intents.
    ActivityInitialization {
        /// Opaque correlation identity.
        work: WorkId,
        /// Exact due moment.
        due: SimMoment,
        /// Immutable authoritative base snapshot.
        snapshot: &'a WorldSnapshot,
        /// Actor whose activity is being initialized.
        actor: ActorId,
        /// Actor-local coalescing generation.
        generation: u64,
        /// Exact accepted intent causes.
        intents: &'a [Intent],
    },
    /// One accepted open action opportunity ready for actor-relative grounding.
    ActionReady {
        /// Opaque correlation identity.
        work: WorkId,
        /// Exact due moment.
        due: SimMoment,
        /// Immutable authoritative base snapshot.
        snapshot: &'a WorldSnapshot,
        /// Exact durable opportunity resolved by runtime control.
        opportunity: &'a ActionOpportunity,
    },
    /// One captured result or fixed fallback for a waiting action opportunity.
    ActionEvaluationResultReady {
        /// Opaque correlation identity.
        work: WorkId,
        /// Exact due moment.
        due: SimMoment,
        /// Immutable authoritative base used for freshness rebuilding.
        snapshot: &'a WorldSnapshot,
        /// Exact captured-result work binding.
        result_ready: ActionEvaluationResultReady,
        /// Exact opportunity that still waits for the invocation.
        opportunity: &'a ActionOpportunity,
        /// Complete retained invocation record.
        invocation: &'a ActionEvaluationInvocationRecord,
    },
    /// One outcome-neutral continuation after an attempted opportunity.
    AttemptResolved {
        /// Opaque correlation identity.
        work: WorkId,
        /// Exact due moment.
        due: SimMoment,
        /// Consumed opportunity with its actor-safe sponsor reattached.
        opportunity: &'a ActionOpportunity,
    },
    /// One coalesced advancement request for an accepted activity.
    ActivityAdvance {
        /// Opaque correlation identity.
        work: WorkId,
        /// Exact due moment.
        due: SimMoment,
        /// Immutable authoritative base snapshot.
        snapshot: &'a WorldSnapshot,
        /// Actor whose activity is being advanced.
        actor: ActorId,
        /// Actor-local coalescing generation.
        generation: u64,
        /// Exact accepted activities named by retained causes.
        activities: &'a [Activity],
        /// Consumed opportunities supplying neutral attempted-action causes.
        attempted: &'a [ActionOpportunity],
    },
    /// One exact current relocation wake awaiting authoritative completion.
    RelocationProcessWake {
        /// Opaque correlation identity.
        work: WorkId,
        /// Exact due moment.
        due: SimMoment,
        /// Identity of the current checked process.
        process: RelocationProcessId,
    },
}

impl MomentWorkInput<'_> {
    /// Returns this input's opaque correlation identity.
    #[must_use]
    pub const fn work_id(self) -> WorkId {
        match self {
            Self::EvaluateCommand { work, .. }
            | Self::PostCommitDispatch { work, .. }
            | Self::EvidenceAssimilation { work, .. }
            | Self::Appraisal { work, .. }
            | Self::IntentReview { work, .. }
            | Self::ActivityInitialization { work, .. }
            | Self::ActionReady { work, .. }
            | Self::ActionEvaluationResultReady { work, .. }
            | Self::AttemptResolved { work, .. }
            | Self::ActivityAdvance { work, .. }
            | Self::RelocationProcessWake { work, .. } => work,
        }
    }

    /// Returns the exact shared delivery moment.
    #[must_use]
    pub const fn due_moment(self) -> SimMoment {
        match self {
            Self::EvaluateCommand { due, .. }
            | Self::PostCommitDispatch { due, .. }
            | Self::EvidenceAssimilation { due, .. }
            | Self::Appraisal { due, .. }
            | Self::IntentReview { due, .. }
            | Self::ActivityInitialization { due, .. }
            | Self::ActionReady { due, .. }
            | Self::ActionEvaluationResultReady { due, .. }
            | Self::AttemptResolved { due, .. }
            | Self::ActivityAdvance { due, .. }
            | Self::RelocationProcessWake { due, .. } => due,
        }
    }
}

/// Runtime-owned command fact established before evaluator work is exposed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreparedCommandResolution {
    Retained {
        original_attempt: AttemptRecordId,
        outcome: CommandAttemptOutcome,
    },
    IdReuseMismatch {
        original_attempt: AttemptRecordId,
    },
    NewCollision,
    RetainedCollision {
        original_attempt: AttemptRecordId,
    },
    Retired,
}

/// One scheduler delivery captured in the complete prepared due set.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PreparedDelivery {
    EvaluableCommand {
        key: SchedulerKey,
        scheduled: ScheduledCommand,
    },
    ResolvedCommand {
        key: SchedulerKey,
        scheduled: ScheduledCommand,
        resolution: PreparedCommandResolution,
    },
    PostCommit {
        key: SchedulerKey,
        dispatch: PostCommitDispatch,
    },
    EvidenceDelivery {
        key: SchedulerKey,
        delivery: EvidenceDeliveryWork,
    },
    Appraisal {
        key: SchedulerKey,
        appraisal: AppraisalWork,
        evidence: Vec<EvidenceRecord>,
        previous: Vec<ContainmentAppraisal>,
    },
    IntentReview {
        key: SchedulerKey,
        review: IntentReviewWork,
        appraisals: Vec<ContainmentAppraisal>,
    },
    ActivityInitialization {
        key: SchedulerKey,
        initialization: ActivityInitializationWork,
        intents: Vec<Intent>,
    },
    ActionReady {
        key: SchedulerKey,
        ready: ActionReady,
        opportunity: ActionOpportunity,
    },
    ActionEvaluation {
        key: SchedulerKey,
        evaluation: ActionEvaluationWork,
        opportunity: ActionOpportunity,
        invocation: Box<ActionEvaluationInvocationRecord>,
    },
    AttemptResolved {
        key: SchedulerKey,
        resolved: AttemptResolved,
        opportunity: ActionOpportunity,
    },
    ActivityAdvance {
        key: SchedulerKey,
        advance: ActivityAdvanceWork,
        activities: Vec<Activity>,
        attempted: Vec<ActionOpportunity>,
    },
    Process {
        key: SchedulerKey,
        wake: RelocationProcessWake,
        classification: RelocationWakeClassification,
    },
}

impl PreparedDelivery {
    #[must_use]
    pub(crate) const fn evaluable_command(key: SchedulerKey, scheduled: ScheduledCommand) -> Self {
        Self::EvaluableCommand { key, scheduled }
    }

    #[must_use]
    pub(crate) const fn resolved_command(
        key: SchedulerKey,
        scheduled: ScheduledCommand,
        resolution: PreparedCommandResolution,
    ) -> Self {
        Self::ResolvedCommand {
            key,
            scheduled,
            resolution,
        }
    }

    #[must_use]
    pub(crate) const fn post_commit(key: SchedulerKey, dispatch: PostCommitDispatch) -> Self {
        Self::PostCommit { key, dispatch }
    }

    #[must_use]
    pub(crate) const fn evidence_delivery(
        key: SchedulerKey,
        delivery: EvidenceDeliveryWork,
    ) -> Self {
        Self::EvidenceDelivery { key, delivery }
    }

    #[must_use]
    pub(crate) fn appraisal(
        key: SchedulerKey,
        appraisal: AppraisalWork,
        evidence: Vec<EvidenceRecord>,
        previous: Vec<ContainmentAppraisal>,
    ) -> Self {
        Self::Appraisal {
            key,
            appraisal,
            evidence,
            previous,
        }
    }

    #[must_use]
    pub(crate) fn intent_review(
        key: SchedulerKey,
        review: IntentReviewWork,
        appraisals: Vec<ContainmentAppraisal>,
    ) -> Self {
        Self::IntentReview {
            key,
            review,
            appraisals,
        }
    }

    #[must_use]
    pub(crate) fn activity_initialization(
        key: SchedulerKey,
        initialization: ActivityInitializationWork,
        intents: Vec<Intent>,
    ) -> Self {
        Self::ActivityInitialization {
            key,
            initialization,
            intents,
        }
    }

    #[must_use]
    pub(crate) const fn action_ready(
        key: SchedulerKey,
        ready: ActionReady,
        opportunity: ActionOpportunity,
    ) -> Self {
        Self::ActionReady {
            key,
            ready,
            opportunity,
        }
    }

    #[must_use]
    pub(crate) fn action_evaluation(
        key: SchedulerKey,
        evaluation: ActionEvaluationWork,
        opportunity: ActionOpportunity,
        invocation: ActionEvaluationInvocationRecord,
    ) -> Self {
        Self::ActionEvaluation {
            key,
            evaluation,
            opportunity,
            invocation: Box::new(invocation),
        }
    }

    #[must_use]
    pub(crate) fn attempt_resolved(
        key: SchedulerKey,
        resolved: AttemptResolved,
        opportunity: ActionOpportunity,
    ) -> Self {
        Self::AttemptResolved {
            key,
            resolved,
            opportunity,
        }
    }

    #[must_use]
    pub(crate) fn activity_advance(
        key: SchedulerKey,
        advance: ActivityAdvanceWork,
        activities: Vec<Activity>,
        attempted: Vec<ActionOpportunity>,
    ) -> Self {
        Self::ActivityAdvance {
            key,
            advance,
            activities,
            attempted,
        }
    }

    #[must_use]
    pub(crate) const fn process(
        key: SchedulerKey,
        wake: RelocationProcessWake,
        classification: RelocationWakeClassification,
    ) -> Self {
        Self::Process {
            key,
            wake,
            classification,
        }
    }

    #[must_use]
    pub(crate) const fn key(&self) -> SchedulerKey {
        match self {
            Self::EvaluableCommand { key, .. }
            | Self::ResolvedCommand { key, .. }
            | Self::PostCommit { key, .. }
            | Self::EvidenceDelivery { key, .. }
            | Self::Appraisal { key, .. }
            | Self::IntentReview { key, .. }
            | Self::ActivityInitialization { key, .. }
            | Self::ActionReady { key, .. }
            | Self::ActionEvaluation { key, .. }
            | Self::AttemptResolved { key, .. }
            | Self::ActivityAdvance { key, .. }
            | Self::Process { key, .. } => *key,
        }
    }

    #[must_use]
    pub(crate) const fn expected_lane(&self) -> SchedulerLaneV2 {
        match self {
            Self::EvaluableCommand { .. } | Self::ResolvedCommand { .. } => {
                SchedulerLaneV2::Command
            }
            Self::PostCommit { .. } => SchedulerLaneV2::PostCommit,
            Self::Process { .. } => SchedulerLaneV2::Process,
            Self::ActionReady { .. } => SchedulerLaneV2::ActionReady,
            Self::ActionEvaluation { .. } => SchedulerLaneV2::ActionEvaluation,
            Self::AttemptResolved { .. } => SchedulerLaneV2::AttemptResolved,
            Self::EvidenceDelivery { .. }
            | Self::Appraisal { .. }
            | Self::IntentReview { .. }
            | Self::ActivityInitialization { .. }
            | Self::ActivityAdvance { .. } => SchedulerLaneV2::Lifecycle,
        }
    }

    #[must_use]
    pub(crate) fn scheduled_work(&self) -> ScheduledWork {
        match self {
            Self::EvaluableCommand { scheduled, .. } | Self::ResolvedCommand { scheduled, .. } => {
                ScheduledWork::command(scheduled.clone())
            }
            Self::PostCommit { dispatch, .. } => ScheduledWork::PostCommit(dispatch.clone()),
            Self::Process { wake, .. } => ScheduledWork::process(*wake),
            Self::ActionReady { ready, .. } => ScheduledWork::ActionReady(*ready),
            Self::ActionEvaluation { evaluation, .. } => {
                ScheduledWork::action_evaluation(*evaluation)
            }
            Self::EvidenceDelivery { delivery, .. } => ScheduledWork::lifecycle(
                crate::lifecycle::LifecycleWork::EvidenceDelivery(*delivery),
            ),
            Self::Appraisal { appraisal, .. } => {
                ScheduledWork::lifecycle(crate::lifecycle::LifecycleWork::Appraisal(*appraisal))
            }
            Self::IntentReview { review, .. } => {
                ScheduledWork::lifecycle(crate::lifecycle::LifecycleWork::IntentReview(*review))
            }
            Self::ActivityInitialization { initialization, .. } => ScheduledWork::lifecycle(
                crate::lifecycle::LifecycleWork::ActivityInitialization(*initialization),
            ),
            Self::AttemptResolved { resolved, .. } => ScheduledWork::attempt_resolved(*resolved),
            Self::ActivityAdvance { advance, .. } => {
                ScheduledWork::lifecycle(crate::lifecycle::LifecycleWork::ActivityAdvance(*advance))
            }
        }
    }

    #[must_use]
    pub(crate) const fn command(&self) -> Option<&ScheduledCommand> {
        match self {
            Self::EvaluableCommand { scheduled, .. } | Self::ResolvedCommand { scheduled, .. } => {
                Some(scheduled)
            }
            Self::PostCommit { .. } => None,
            Self::EvidenceDelivery { .. }
            | Self::Appraisal { .. }
            | Self::IntentReview { .. }
            | Self::ActivityInitialization { .. }
            | Self::ActionReady { .. }
            | Self::ActionEvaluation { .. }
            | Self::AttemptResolved { .. }
            | Self::ActivityAdvance { .. }
            | Self::Process { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PreparedWorkKind {
    Command {
        representative: usize,
    },
    PostCommit {
        delivery: usize,
    },
    EvidenceAssimilation {
        actor: ActorId,
        evidence: Vec<EvidenceRecord>,
    },
    Appraisal {
        delivery: usize,
    },
    IntentReview {
        delivery: usize,
    },
    ActivityInitialization {
        delivery: usize,
    },
    ActionReady {
        delivery: usize,
    },
    ActionEvaluationResultReady {
        delivery: usize,
    },
    AttemptResolved {
        delivery: usize,
    },
    ActivityAdvance {
        delivery: usize,
    },
    RelocationProcess {
        delivery: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedWork {
    id: WorkId,
    kind: PreparedWorkKind,
}

/// Why a complete prepared-moment capability could not be formed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreparedFireBuildError {
    EmptyDueSet,
    MixedDueMoment {
        expected: SimMoment,
        supplied: SimMoment,
    },
    DuplicateSchedulerKey {
        key: SchedulerKey,
    },
    SchedulerLaneMismatch {
        key: SchedulerKey,
        expected: SchedulerLaneV2,
    },
    CommandMomentMismatch {
        key: SchedulerKey,
        effective: SimMoment,
    },
    MixedCommandPreparation {
        source: CommandSource,
        command: CommandId,
    },
    EvaluableCommandCollision {
        source: CommandSource,
        command: CommandId,
    },
    DuplicatePostCommitDispatch {
        dispatch: PostCommitDispatchId,
    },
    DuplicateLifecycleGeneration {
        actor: ActorId,
    },
    EmptyLifecycleInput {
        actor: ActorId,
    },
    DuplicateActionOpportunity {
        opportunity: ActionOpportunityId,
    },
    ActionOpportunityMismatch {
        opportunity: ActionOpportunityId,
    },
    DuplicateActionEvaluation {
        invocation: world_model::ActionEvaluationInvocationId,
    },
    ActionEvaluationMismatch {
        invocation: world_model::ActionEvaluationInvocationId,
    },
    DuplicateAttemptResolution {
        opportunity: ActionOpportunityId,
    },
    DuplicateCurrentRelocationWake {
        process: RelocationProcessId,
    },
    WorkPopulationOverflow,
}

/// Single-use process capability for one complete least-due moment.
///
/// Dropping this value performs no cleanup. The repository retains the
/// reservation until same-domain reconciliation.
pub struct PreparedFire {
    domain: AttemptAuthorityDomainId,
    attempt: RunAttemptId,
    execution: ExecutionSpecId,
    step: AttemptStepId,
    grant: ReservationGrant,
    moment: SimMoment,
    resulting_frontier: SimMoment,
    snapshot: WorldSnapshot,
    deliveries: Vec<PreparedDelivery>,
    work: Vec<PreparedWork>,
    delivery_work: Vec<Option<WorkId>>,
}

impl PreparedFire {
    #[allow(
        clippy::too_many_arguments,
        reason = "the capability constructor captures one complete reservation binding, moment, snapshot, and due set"
    )]
    pub(crate) fn new(
        domain: AttemptAuthorityDomainId,
        attempt: RunAttemptId,
        execution: ExecutionSpecId,
        step: AttemptStepId,
        grant: ReservationGrant,
        resulting_frontier: SimMoment,
        snapshot: WorldSnapshot,
        mut deliveries: Vec<PreparedDelivery>,
    ) -> Result<Self, PreparedFireBuildError> {
        deliveries.sort_by_key(PreparedDelivery::key);
        let Some(first) = deliveries.first() else {
            return Err(PreparedFireBuildError::EmptyDueSet);
        };
        let moment = first.key().moment();

        for pair in deliveries.windows(2) {
            if pair[0].key() == pair[1].key() {
                return Err(PreparedFireBuildError::DuplicateSchedulerKey { key: pair[0].key() });
            }
        }
        for delivery in &deliveries {
            let key = delivery.key();
            if key.moment() != moment {
                return Err(PreparedFireBuildError::MixedDueMoment {
                    expected: moment,
                    supplied: key.moment(),
                });
            }
            if key.lane() != delivery.expected_lane() {
                return Err(PreparedFireBuildError::SchedulerLaneMismatch {
                    key,
                    expected: delivery.expected_lane(),
                });
            }
            if let Some(scheduled) = delivery.command()
                && scheduled.effective() != moment
            {
                return Err(PreparedFireBuildError::CommandMomentMismatch {
                    key,
                    effective: scheduled.effective(),
                });
            }
        }

        let (work, delivery_work) = correlate_work(step, &deliveries)?;
        Ok(Self {
            domain,
            attempt,
            execution,
            step,
            grant,
            moment,
            resulting_frontier,
            snapshot,
            deliveries,
            work,
            delivery_work,
        })
    }

    /// Iterates over every and only engine-facing input in canonical order.
    ///
    /// Runtime-resolved command deliveries are deliberately absent.
    #[must_use]
    pub fn work(&self) -> impl ExactSizeIterator<Item = MomentWorkInput<'_>> + '_ {
        self.work.iter().map(|work| self.input_for(work))
    }

    pub(crate) const fn domain(&self) -> AttemptAuthorityDomainId {
        self.domain
    }

    pub(crate) const fn attempt(&self) -> RunAttemptId {
        self.attempt
    }

    pub(crate) const fn execution(&self) -> ExecutionSpecId {
        self.execution
    }

    pub(crate) const fn step(&self) -> AttemptStepId {
        self.step
    }

    pub(crate) const fn grant(&self) -> ReservationGrant {
        self.grant
    }

    pub(crate) const fn moment(&self) -> SimMoment {
        self.moment
    }

    pub(crate) const fn resulting_frontier(&self) -> SimMoment {
        self.resulting_frontier
    }

    pub(crate) const fn base_snapshot(&self) -> &WorldSnapshot {
        &self.snapshot
    }

    pub(crate) fn deliveries(&self) -> &[PreparedDelivery] {
        &self.deliveries
    }

    pub(crate) fn work_id_for_delivery(&self, position: usize) -> Option<WorkId> {
        self.delivery_work.get(position).copied().flatten()
    }

    pub(crate) fn validate_proposals(
        &self,
        proposals: &MomentWorkProposals,
    ) -> Result<(), ProposalBuildError> {
        proposals.validate_for(self)
    }

    fn input_for<'a>(&'a self, work: &'a PreparedWork) -> MomentWorkInput<'a> {
        match &work.kind {
            PreparedWorkKind::Command { representative } => {
                let scheduled = match &self.deliveries[*representative] {
                    PreparedDelivery::EvaluableCommand { scheduled, .. } => scheduled,
                    _ => {
                        unreachable!("prepared command work must retain its representative")
                    }
                };
                MomentWorkInput::EvaluateCommand {
                    work: work.id,
                    due: self.moment,
                    snapshot: &self.snapshot,
                    command: scheduled.command(),
                }
            }
            PreparedWorkKind::PostCommit { delivery } => {
                let dispatch = match &self.deliveries[*delivery] {
                    PreparedDelivery::PostCommit { dispatch, .. } => dispatch,
                    _ => {
                        unreachable!("prepared dispatch work must retain its delivery")
                    }
                };
                MomentWorkInput::PostCommitDispatch {
                    work: work.id,
                    due: self.moment,
                    dispatch,
                }
            }
            PreparedWorkKind::EvidenceAssimilation { actor, evidence } => {
                MomentWorkInput::EvidenceAssimilation {
                    work: work.id,
                    due: self.moment,
                    snapshot: &self.snapshot,
                    actor: *actor,
                    evidence,
                }
            }
            PreparedWorkKind::Appraisal { delivery } => {
                let (appraisal, evidence, previous) = match &self.deliveries[*delivery] {
                    PreparedDelivery::Appraisal {
                        appraisal,
                        evidence,
                        previous,
                        ..
                    } => (appraisal, evidence.as_slice(), previous.as_slice()),
                    _ => unreachable!("prepared appraisal work must retain its lifecycle input"),
                };
                MomentWorkInput::Appraisal {
                    work: work.id,
                    due: self.moment,
                    snapshot: &self.snapshot,
                    actor: appraisal.actor(),
                    generation: appraisal.generation().get(),
                    evidence,
                    previous,
                }
            }
            PreparedWorkKind::IntentReview { delivery } => {
                let (review, appraisals) = match &self.deliveries[*delivery] {
                    PreparedDelivery::IntentReview {
                        review, appraisals, ..
                    } => (review, appraisals.as_slice()),
                    _ => unreachable!("prepared intent work must retain its lifecycle input"),
                };
                MomentWorkInput::IntentReview {
                    work: work.id,
                    due: self.moment,
                    snapshot: &self.snapshot,
                    actor: review.actor(),
                    generation: review.generation().get(),
                    appraisals,
                }
            }
            PreparedWorkKind::ActivityInitialization { delivery } => {
                let (initialization, intents) = match &self.deliveries[*delivery] {
                    PreparedDelivery::ActivityInitialization {
                        initialization,
                        intents,
                        ..
                    } => (initialization, intents.as_slice()),
                    _ => {
                        unreachable!(
                            "prepared activity initialization must retain its lifecycle input"
                        )
                    }
                };
                MomentWorkInput::ActivityInitialization {
                    work: work.id,
                    due: self.moment,
                    snapshot: &self.snapshot,
                    actor: initialization.actor(),
                    generation: initialization.generation().get(),
                    intents,
                }
            }
            PreparedWorkKind::ActionReady { delivery } => {
                let opportunity = match &self.deliveries[*delivery] {
                    PreparedDelivery::ActionReady { opportunity, .. } => opportunity,
                    _ => {
                        unreachable!("prepared action work must retain its opportunity")
                    }
                };
                MomentWorkInput::ActionReady {
                    work: work.id,
                    due: self.moment,
                    snapshot: &self.snapshot,
                    opportunity,
                }
            }
            PreparedWorkKind::ActionEvaluationResultReady { delivery } => {
                let (evaluation, opportunity, invocation) = match &self.deliveries[*delivery] {
                    PreparedDelivery::ActionEvaluation {
                        evaluation,
                        opportunity,
                        invocation,
                        ..
                    } => (evaluation, opportunity, invocation.as_ref()),
                    _ => {
                        unreachable!("prepared action evaluation must retain its invocation")
                    }
                };
                let result_ready = evaluation.result_ready_binding().unwrap_or_else(|| {
                    unreachable!("only captured results become engine-facing evaluation work")
                });
                MomentWorkInput::ActionEvaluationResultReady {
                    work: work.id,
                    due: self.moment,
                    snapshot: &self.snapshot,
                    result_ready,
                    opportunity,
                    invocation,
                }
            }
            PreparedWorkKind::AttemptResolved { delivery } => {
                let opportunity = match &self.deliveries[*delivery] {
                    PreparedDelivery::AttemptResolved { opportunity, .. } => opportunity,
                    _ => {
                        unreachable!("prepared neutral wake must retain its opportunity")
                    }
                };
                MomentWorkInput::AttemptResolved {
                    work: work.id,
                    due: self.moment,
                    opportunity,
                }
            }
            PreparedWorkKind::ActivityAdvance { delivery } => {
                let (advance, activities, attempted) = match &self.deliveries[*delivery] {
                    PreparedDelivery::ActivityAdvance {
                        advance,
                        activities,
                        attempted,
                        ..
                    } => (advance, activities.as_slice(), attempted.as_slice()),
                    _ => unreachable!("prepared activity advance must retain its lifecycle input"),
                };
                MomentWorkInput::ActivityAdvance {
                    work: work.id,
                    due: self.moment,
                    snapshot: &self.snapshot,
                    actor: advance.actor(),
                    generation: advance.generation().get(),
                    activities,
                    attempted,
                }
            }
            PreparedWorkKind::RelocationProcess { delivery } => {
                let process = match &self.deliveries[*delivery] {
                    PreparedDelivery::Process {
                        classification: RelocationWakeClassification::Current(process),
                        ..
                    } => *process,
                    PreparedDelivery::Process {
                        classification: RelocationWakeClassification::Obsolete,
                        ..
                    } => {
                        unreachable!("obsolete process wakes must not produce engine work")
                    }
                    _ => {
                        unreachable!("prepared process work must retain one current wake")
                    }
                };
                MomentWorkInput::RelocationProcessWake {
                    work: work.id,
                    due: self.moment,
                    process,
                }
            }
        }
    }
}

fn correlate_work(
    step: AttemptStepId,
    deliveries: &[PreparedDelivery],
) -> Result<(Vec<PreparedWork>, Vec<Option<WorkId>>), PreparedFireBuildError> {
    type LogicalCommand = (CommandSource, CommandId);
    type EvaluableGroup = (CommandRequestFingerprint, usize, Vec<usize>);

    let mut evaluable = BTreeMap::<LogicalCommand, EvaluableGroup>::new();
    let mut resolved = BTreeSet::<LogicalCommand>::new();
    let mut dispatches = BTreeMap::<PostCommitDispatchId, usize>::new();
    let mut evidence = BTreeMap::<ActorId, (Vec<usize>, Vec<EvidenceRecord>)>::new();
    let mut appraisals = BTreeMap::<(ActorId, u64), usize>::new();
    let mut intent_reviews = BTreeMap::<(ActorId, u64), usize>::new();
    let mut activity_initializations = BTreeMap::<(ActorId, u64), usize>::new();
    let mut action_ready = BTreeMap::<ActionOpportunityId, usize>::new();
    let mut seen_action_evaluations = BTreeSet::<world_model::ActionEvaluationInvocationId>::new();
    let mut action_result_ready =
        BTreeMap::<world_model::ActionEvaluationInvocationId, usize>::new();
    let mut attempt_resolved = BTreeMap::<ActionOpportunityId, usize>::new();
    let mut activity_advances = BTreeMap::<(ActorId, u64), usize>::new();
    let mut process_wakes = BTreeMap::<RelocationProcessId, usize>::new();

    for (position, delivery) in deliveries.iter().enumerate() {
        match delivery {
            PreparedDelivery::EvaluableCommand { scheduled, .. } => {
                let command = scheduled.command();
                let identity = (command.source(), command.id());
                if resolved.contains(&identity) {
                    return Err(PreparedFireBuildError::MixedCommandPreparation {
                        source: identity.0,
                        command: identity.1,
                    });
                }
                match evaluable.entry(identity) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert((command.fingerprint(), position, vec![position]));
                    }
                    std::collections::btree_map::Entry::Occupied(mut entry) => {
                        if entry.get().0 != command.fingerprint() {
                            return Err(PreparedFireBuildError::EvaluableCommandCollision {
                                source: identity.0,
                                command: identity.1,
                            });
                        }
                        entry.get_mut().2.push(position);
                    }
                }
            }
            PreparedDelivery::ResolvedCommand { scheduled, .. } => {
                let identity = (scheduled.command().source(), scheduled.command().id());
                if evaluable.contains_key(&identity) {
                    return Err(PreparedFireBuildError::MixedCommandPreparation {
                        source: identity.0,
                        command: identity.1,
                    });
                }
                resolved.insert(identity);
            }
            PreparedDelivery::PostCommit { dispatch, .. } => {
                if dispatches.insert(dispatch.id(), position).is_some() {
                    return Err(PreparedFireBuildError::DuplicatePostCommitDispatch {
                        dispatch: dispatch.id(),
                    });
                }
            }
            PreparedDelivery::EvidenceDelivery { delivery, .. } => {
                let record = delivery.evidence();
                let entry = evidence.entry(record.observer()).or_default();
                entry.0.push(position);
                entry.1.push(record);
            }
            PreparedDelivery::Appraisal {
                appraisal,
                evidence,
                previous,
                ..
            } => {
                let actor = appraisal.actor();
                if evidence.is_empty()
                    || evidence.iter().any(|record| record.observer() != actor)
                    || previous.iter().any(|retained| retained.actor() != actor)
                {
                    return Err(PreparedFireBuildError::EmptyLifecycleInput { actor });
                }
                if appraisals
                    .insert((actor, appraisal.generation().get()), position)
                    .is_some()
                {
                    return Err(PreparedFireBuildError::DuplicateLifecycleGeneration { actor });
                }
            }
            PreparedDelivery::IntentReview {
                review, appraisals, ..
            } => {
                let actor = review.actor();
                if appraisals.is_empty()
                    || appraisals
                        .iter()
                        .any(|appraisal| appraisal.actor() != actor)
                {
                    return Err(PreparedFireBuildError::EmptyLifecycleInput { actor });
                }
                if intent_reviews
                    .insert((actor, review.generation().get()), position)
                    .is_some()
                {
                    return Err(PreparedFireBuildError::DuplicateLifecycleGeneration { actor });
                }
            }
            PreparedDelivery::ActivityInitialization {
                initialization,
                intents,
                ..
            } => {
                let actor = initialization.actor();
                if intents.is_empty() || intents.iter().any(|intent| intent.actor() != actor) {
                    return Err(PreparedFireBuildError::EmptyLifecycleInput { actor });
                }
                if activity_initializations
                    .insert((actor, initialization.generation().get()), position)
                    .is_some()
                {
                    return Err(PreparedFireBuildError::DuplicateLifecycleGeneration { actor });
                }
            }
            PreparedDelivery::ActionReady {
                ready, opportunity, ..
            } => {
                if ready.opportunity() != opportunity.id()
                    || ready.expected_version() != opportunity.version()
                    || opportunity.state() != world_model::ActionOpportunityState::Open
                {
                    return Err(PreparedFireBuildError::ActionOpportunityMismatch {
                        opportunity: ready.opportunity(),
                    });
                }
                if action_ready.insert(ready.opportunity(), position).is_some() {
                    return Err(PreparedFireBuildError::DuplicateActionOpportunity {
                        opportunity: ready.opportunity(),
                    });
                }
            }
            PreparedDelivery::ActionEvaluation {
                key,
                evaluation,
                opportunity,
                invocation,
                ..
            } => {
                let exact_waiting = opportunity.id() == evaluation.opportunity()
                    && opportunity.version() == evaluation.expected_waiting_version()
                    && opportunity.state()
                        == world_model::ActionOpportunityState::WaitingForEvaluation(
                            evaluation.invocation(),
                        );
                let exact_invocation = invocation.invocation() == evaluation.invocation()
                    && invocation.opportunity() == evaluation.opportunity()
                    && invocation.waiting_version() == evaluation.expected_waiting_version()
                    && match (evaluation, invocation.state()) {
                        (
                            ActionEvaluationWork::ResultReady { due, .. },
                            ActionEvaluationInvocationState::ResultCaptured {
                                effective,
                                scheduler_key,
                                ..
                            },
                        ) => *due == *effective && scheduler_key.moment() == *due,
                        (
                            ActionEvaluationWork::Fallback { cause, due, .. },
                            ActionEvaluationInvocationState::FallbackPending {
                                cause: retained,
                                scheduler_key,
                            },
                        ) => *cause == *retained && scheduler_key.moment() == *due,
                        _ => false,
                    }
                    && match invocation.state() {
                        ActionEvaluationInvocationState::ResultCaptured {
                            scheduler_key, ..
                        }
                        | ActionEvaluationInvocationState::FallbackPending {
                            scheduler_key, ..
                        } => scheduler_key == key,
                        ActionEvaluationInvocationState::DispatchPending
                        | ActionEvaluationInvocationState::Terminal(_) => false,
                    };
                if !exact_waiting || !exact_invocation {
                    return Err(PreparedFireBuildError::ActionEvaluationMismatch {
                        invocation: evaluation.invocation(),
                    });
                }
                if !seen_action_evaluations.insert(evaluation.invocation()) {
                    return Err(PreparedFireBuildError::DuplicateActionEvaluation {
                        invocation: evaluation.invocation(),
                    });
                }
                if matches!(evaluation, ActionEvaluationWork::ResultReady { .. }) {
                    action_result_ready.insert(evaluation.invocation(), position);
                }
            }
            PreparedDelivery::AttemptResolved {
                resolved,
                opportunity,
                ..
            } => {
                if resolved.opportunity() != opportunity.id()
                    || !matches!(
                        opportunity.state(),
                        world_model::ActionOpportunityState::Consumed(_)
                    )
                {
                    return Err(PreparedFireBuildError::ActionOpportunityMismatch {
                        opportunity: resolved.opportunity(),
                    });
                }
                if attempt_resolved
                    .insert(resolved.opportunity(), position)
                    .is_some()
                {
                    return Err(PreparedFireBuildError::DuplicateAttemptResolution {
                        opportunity: resolved.opportunity(),
                    });
                }
            }
            PreparedDelivery::ActivityAdvance {
                advance,
                activities,
                attempted,
                ..
            } => {
                let actor = advance.actor();
                if (activities.is_empty() && attempted.is_empty())
                    || activities.iter().any(|activity| activity.actor() != actor)
                    || attempted
                        .iter()
                        .any(|opportunity| opportunity.actor() != actor)
                {
                    return Err(PreparedFireBuildError::EmptyLifecycleInput { actor });
                }
                if activity_advances
                    .insert((actor, advance.generation().get()), position)
                    .is_some()
                {
                    return Err(PreparedFireBuildError::DuplicateLifecycleGeneration { actor });
                }
            }
            PreparedDelivery::Process {
                wake,
                classification,
                ..
            } => {
                if matches!(classification, RelocationWakeClassification::Current(_))
                    && process_wakes.insert(wake.process(), position).is_some()
                {
                    return Err(PreparedFireBuildError::DuplicateCurrentRelocationWake {
                        process: wake.process(),
                    });
                }
            }
        }
    }

    let mut work = Vec::with_capacity(
        evaluable.len()
            + dispatches.len()
            + process_wakes.len()
            + evidence.len()
            + appraisals.len()
            + intent_reviews.len()
            + activity_initializations.len()
            + activity_advances.len()
            + action_ready.len()
            + action_result_ready.len()
            + attempt_resolved.len(),
    );
    let mut delivery_work = vec![None; deliveries.len()];
    for (_, (_, representative, members)) in evaluable {
        let id = WorkId::from_position(step, work.len())
            .ok_or(PreparedFireBuildError::WorkPopulationOverflow)?;
        work.push(PreparedWork {
            id,
            kind: PreparedWorkKind::Command { representative },
        });
        for member in members {
            delivery_work[member] = Some(id);
        }
    }
    for (_, delivery) in dispatches {
        let id = WorkId::from_position(step, work.len())
            .ok_or(PreparedFireBuildError::WorkPopulationOverflow)?;
        work.push(PreparedWork {
            id,
            kind: PreparedWorkKind::PostCommit { delivery },
        });
        delivery_work[delivery] = Some(id);
    }
    for (_, delivery) in process_wakes {
        let id = WorkId::from_position(step, work.len())
            .ok_or(PreparedFireBuildError::WorkPopulationOverflow)?;
        work.push(PreparedWork {
            id,
            kind: PreparedWorkKind::RelocationProcess { delivery },
        });
        delivery_work[delivery] = Some(id);
    }
    for (actor, (members, mut records)) in evidence {
        let id = WorkId::from_position(step, work.len())
            .ok_or(PreparedFireBuildError::WorkPopulationOverflow)?;
        records.sort_by_key(|record| (record.generation(), record.id()));
        work.push(PreparedWork {
            id,
            kind: PreparedWorkKind::EvidenceAssimilation {
                actor,
                evidence: records,
            },
        });
        for member in members {
            delivery_work[member] = Some(id);
        }
    }
    for (_, delivery) in appraisals {
        let id = WorkId::from_position(step, work.len())
            .ok_or(PreparedFireBuildError::WorkPopulationOverflow)?;
        work.push(PreparedWork {
            id,
            kind: PreparedWorkKind::Appraisal { delivery },
        });
        delivery_work[delivery] = Some(id);
    }
    for (_, delivery) in intent_reviews {
        let id = WorkId::from_position(step, work.len())
            .ok_or(PreparedFireBuildError::WorkPopulationOverflow)?;
        work.push(PreparedWork {
            id,
            kind: PreparedWorkKind::IntentReview { delivery },
        });
        delivery_work[delivery] = Some(id);
    }
    for (_, delivery) in activity_initializations {
        let id = WorkId::from_position(step, work.len())
            .ok_or(PreparedFireBuildError::WorkPopulationOverflow)?;
        work.push(PreparedWork {
            id,
            kind: PreparedWorkKind::ActivityInitialization { delivery },
        });
        delivery_work[delivery] = Some(id);
    }
    for (_, delivery) in activity_advances {
        let id = WorkId::from_position(step, work.len())
            .ok_or(PreparedFireBuildError::WorkPopulationOverflow)?;
        work.push(PreparedWork {
            id,
            kind: PreparedWorkKind::ActivityAdvance { delivery },
        });
        delivery_work[delivery] = Some(id);
    }
    for (_, delivery) in action_ready {
        let id = WorkId::from_position(step, work.len())
            .ok_or(PreparedFireBuildError::WorkPopulationOverflow)?;
        work.push(PreparedWork {
            id,
            kind: PreparedWorkKind::ActionReady { delivery },
        });
        delivery_work[delivery] = Some(id);
    }
    for (_, delivery) in action_result_ready {
        let id = WorkId::from_position(step, work.len())
            .ok_or(PreparedFireBuildError::WorkPopulationOverflow)?;
        work.push(PreparedWork {
            id,
            kind: PreparedWorkKind::ActionEvaluationResultReady { delivery },
        });
        delivery_work[delivery] = Some(id);
    }
    for (_, delivery) in attempt_resolved {
        let id = WorkId::from_position(step, work.len())
            .ok_or(PreparedFireBuildError::WorkPopulationOverflow)?;
        work.push(PreparedWork {
            id,
            kind: PreparedWorkKind::AttemptResolved { delivery },
        });
        delivery_work[delivery] = Some(id);
    }

    Ok((work, delivery_work))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandProposal {
    Rejected(StableCommandRejection),
    AcceptedTransfer(ContainmentTransferDelta),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ActionProposal {
    Submit {
        expected_version: ActionOpportunityVersion,
        command: Box<CommandEnvelope>,
    },
    Finish {
        expected_version: ActionOpportunityVersion,
        disposition: ActionOpportunityDisposition,
    },
    Relocation {
        expected_version: ActionOpportunityVersion,
        interaction: RelocationInteraction,
    },
    BeginDeferred {
        expected_version: ActionOpportunityVersion,
        input: Box<DeferredActionInvocationInput>,
    },
}

impl ActionProposal {
    pub(crate) const fn expected_version(&self) -> ActionOpportunityVersion {
        match self {
            Self::Submit {
                expected_version, ..
            }
            | Self::Finish {
                expected_version, ..
            }
            | Self::Relocation {
                expected_version, ..
            }
            | Self::BeginDeferred {
                expected_version, ..
            } => *expected_version,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WorkProposal {
    Command(CommandProposal),
    PostCommit(Vec<EvidenceObservation>),
    EvidenceAssimilation {
        actor: ActorId,
        expected_version: EpistemicVersion,
        successor: Box<EpistemicState>,
    },
    Appraisal(Vec<AppraisalResult>),
    IntentReview(IntentReviewResult),
    ActivityInitialization(ActivityInitializationResult),
    Action(ActionProposal),
    ActionEvaluation(ActionEvaluationDecision),
    AttemptResolvedConsumed,
    ActivityAdvance(ActivityAdvanceResult),
    RelocationProcessCompleted,
}

/// Closed engine-owned decision for one post-commit routing input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PostCommitRoutingDecision {
    /// Consume the dispatch and create the supplied actor-addressed deliveries.
    DeliverEvidence(Vec<EvidenceObservation>),
}

/// One closed derived-appraisal mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppraisalResult {
    /// Establishes or replaces one exact appraisal value.
    Present {
        appraisal: ContainmentAppraisal,
        material_changed: bool,
    },
    /// Retracts one exact retained appraisal using actor-local absence evidence.
    Retract {
        before: ContainmentAppraisal,
        supporting_evidence: world_model::EvidenceDeliveryId,
    },
}

impl AppraisalResult {
    /// Constructs one coordinated appraisal result.
    #[must_use]
    pub const fn present(appraisal: ContainmentAppraisal, material_changed: bool) -> Self {
        Self::Present {
            appraisal,
            material_changed,
        }
    }

    /// Constructs one exact appraisal retraction.
    #[must_use]
    pub const fn retract(
        before: ContainmentAppraisal,
        supporting_evidence: world_model::EvidenceDeliveryId,
    ) -> Self {
        Self::Retract {
            before,
            supporting_evidence,
        }
    }
}

/// Concrete result of one coalesced intent review.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentReviewResult {
    /// Adopt one newly grounded intent.
    Adopt(Intent),
    /// Complete the generation without accepted agency change.
    NoChange,
}

/// Concrete result of one activity-initialization invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivityInitializationResult {
    /// Start one activity and open its first sponsored opportunity.
    Start {
        /// Newly accepted activity.
        activity: Box<Activity>,
        /// Newly opened opportunity.
        opportunity: ActionOpportunity,
    },
    /// Apply a terminal or suspension transition to the source intent.
    TransitionIntent {
        /// Expected accepted intent version.
        expected_version: IntentVersion,
        /// Exact checked successor.
        successor: Intent,
    },
}

/// Concrete result of one activity-advancement invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivityAdvanceResult {
    /// Advance activity state and open one successor opportunity.
    OpenAction {
        /// Expected accepted activity version.
        expected_version: ActivityVersion,
        /// Exact checked activity successor.
        successor: Box<Activity>,
        /// Newly opened opportunity.
        opportunity: ActionOpportunity,
    },
    /// Advance activity state without opening an opportunity.
    Transition {
        /// Expected accepted activity version.
        expected_version: ActivityVersion,
        /// Exact checked activity successor.
        successor: Box<Activity>,
    },
    /// Atomically terminate an activity and its owning intent.
    Terminal {
        /// Expected accepted activity version.
        expected_activity_version: ActivityVersion,
        /// Exact checked terminal activity successor.
        activity_successor: Box<Activity>,
        /// Expected accepted owning-intent version.
        expected_intent_version: IntentVersion,
        /// Exact checked terminal intent successor.
        intent_successor: Intent,
    },
    /// Complete the generation without accepted agency change.
    NoChange {
        /// Exact accepted activity.
        activity: ActivityId,
        /// Expected accepted activity version.
        expected_version: ActivityVersion,
    },
}

/// Opaque proposal for one capability-scoped item in a prepared moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MomentWorkDecision {
    work: WorkId,
    proposal: WorkProposal,
}

impl MomentWorkDecision {
    pub(crate) const fn command(work: WorkId, proposal: CommandProposal) -> Self {
        Self {
            work,
            proposal: WorkProposal::Command(proposal),
        }
    }

    /// Correlates one engine routing decision with its exact prepared input.
    pub fn route_post_commit(
        input: MomentWorkInput<'_>,
        decision: PostCommitRoutingDecision,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::PostCommitDispatch { work, .. } = input else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        match decision {
            PostCommitRoutingDecision::DeliverEvidence(mut observations) => {
                let MomentWorkInput::PostCommitDispatch { dispatch, .. } = input else {
                    unreachable!("post-commit input was matched above")
                };
                observations
                    .sort_by_key(|observation| (observation.event_index(), observation.observer()));
                for observation in &observations {
                    if usize::try_from(observation.event_index())
                        .ok()
                        .is_none_or(|index| index >= dispatch.reaction().events().len())
                    {
                        return Err(ProposalBuildError::UnknownEvidenceEvent {
                            work,
                            event_index: observation.event_index(),
                        });
                    }
                }
                if let Some(pair) = observations.windows(2).find(|pair| pair[0] == pair[1]) {
                    return Err(ProposalBuildError::DuplicateEvidenceObservation {
                        work,
                        observer: pair[0].observer(),
                        event_index: pair[0].event_index(),
                    });
                }
                Ok(Self {
                    work,
                    proposal: WorkProposal::PostCommit(observations),
                })
            }
        }
    }

    /// Publishes one checked actor-local evidence successor.
    pub fn assimilate_evidence(
        input: MomentWorkInput<'_>,
        successor: EpistemicState,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::EvidenceAssimilation {
            work,
            snapshot,
            actor,
            ..
        } = input
        else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        Ok(Self {
            work,
            proposal: WorkProposal::EvidenceAssimilation {
                actor,
                expected_version: snapshot.accepted().epistemic().actor_version(actor),
                successor: Box::new(successor),
            },
        })
    }

    /// Publishes all appraisal results for one coalesced generation.
    pub fn publish_appraisals(
        input: MomentWorkInput<'_>,
        results: Vec<AppraisalResult>,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::Appraisal { work, .. } = input else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        Ok(Self {
            work,
            proposal: WorkProposal::Appraisal(results),
        })
    }

    /// Publishes the concrete outcome of one intent review.
    pub fn review_intent(
        input: MomentWorkInput<'_>,
        result: IntentReviewResult,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::IntentReview { work, .. } = input else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        Ok(Self {
            work,
            proposal: WorkProposal::IntentReview(result),
        })
    }

    /// Publishes the concrete outcome of one activity initialization.
    pub fn initialize_activity(
        input: MomentWorkInput<'_>,
        result: ActivityInitializationResult,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::ActivityInitialization { work, .. } = input else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        Ok(Self {
            work,
            proposal: WorkProposal::ActivityInitialization(result),
        })
    }

    /// Correlates one privately lowered command with its exact opportunity.
    ///
    /// This is trusted engine-coordination substrate. The private coordinator
    /// must already have proved grounded-candidate membership and budget
    /// inclusion before calling it. Runtime independently rechecks the
    /// opportunity-owned actor, containment scope, action family, and current
    /// authoritative legality before publication.
    pub fn submit_action(
        input: MomentWorkInput<'_>,
        command: CommandEnvelope,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::ActionReady {
            work, opportunity, ..
        } = input
        else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        Ok(Self {
            work,
            proposal: WorkProposal::Action(ActionProposal::Submit {
                expected_version: opportunity.version(),
                command: Box::new(command),
            }),
        })
    }

    /// Applies one terminal non-submission disposition to an open opportunity.
    pub fn finish_action(
        input: MomentWorkInput<'_>,
        disposition: ActionOpportunityDisposition,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::ActionReady {
            work, opportunity, ..
        } = input
        else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        if disposition == ActionOpportunityDisposition::ActionSubmitted {
            return Err(ProposalBuildError::ActionSubmissionRequiresCommand { work });
        }
        Ok(Self {
            work,
            proposal: WorkProposal::Action(ActionProposal::Finish {
                expected_version: opportunity.version(),
                disposition,
            }),
        })
    }

    /// Correlates one privately grounded relocation interaction with its
    /// exact opportunity.
    ///
    /// Process identity, version, progress, and wake generation are not
    /// actor-facing selections. Runtime binds those values from its current
    /// relocation ledger and rechecks the exact interaction scope at seal.
    pub fn submit_relocation_action(
        input: MomentWorkInput<'_>,
        interaction: RelocationInteraction,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::ActionReady {
            work, opportunity, ..
        } = input
        else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        Ok(Self {
            work,
            proposal: WorkProposal::Action(ActionProposal::Relocation {
                expected_version: opportunity.version(),
                interaction,
            }),
        })
    }

    /// Atomically retains one deferred action evaluation for an open
    /// opportunity.
    ///
    /// Runtime derives invocation identity, execution control, authority
    /// provenance, and timing while sealing the enclosing moment.
    pub fn begin_deferred_action(
        input: MomentWorkInput<'_>,
        deferred: DeferredActionInvocationInput,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::ActionReady {
            work, opportunity, ..
        } = input
        else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        Ok(Self {
            work,
            proposal: WorkProposal::Action(ActionProposal::BeginDeferred {
                expected_version: opportunity.version(),
                input: Box::new(deferred),
            }),
        })
    }

    /// Resolves one captured deferred action result against its exact retained
    /// invocation.
    pub fn resolve_action_evaluation(
        input: MomentWorkInput<'_>,
        decision: ActionEvaluationDecision,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::ActionEvaluationResultReady { work, .. } = input else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        Ok(Self {
            work,
            proposal: WorkProposal::ActionEvaluation(decision),
        })
    }

    /// Consumes an outcome-neutral attempt-resolution wake.
    pub fn consume_attempt_resolution(
        input: MomentWorkInput<'_>,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::AttemptResolved { work, .. } = input else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        Ok(Self {
            work,
            proposal: WorkProposal::AttemptResolvedConsumed,
        })
    }

    /// Publishes the concrete outcome of one activity advancement.
    pub fn advance_activity(
        input: MomentWorkInput<'_>,
        result: ActivityAdvanceResult,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::ActivityAdvance { work, .. } = input else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        Ok(Self {
            work,
            proposal: WorkProposal::ActivityAdvance(result),
        })
    }

    /// Completes one exact current relocation-process wake.
    pub fn complete_relocation_process(
        input: MomentWorkInput<'_>,
    ) -> Result<Self, ProposalBuildError> {
        let MomentWorkInput::RelocationProcessWake { work, .. } = input else {
            return Err(ProposalBuildError::DecisionKindMismatch {
                work: input.work_id(),
            });
        };
        Ok(Self {
            work,
            proposal: WorkProposal::RelocationProcessCompleted,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CorrelatedWorkProposal {
    work: WorkId,
    proposal: WorkProposal,
}

/// Why a decision collection does not exactly cover its prepared work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposalBuildError {
    DuplicateWork {
        work: WorkId,
    },
    UnknownWork {
        work: WorkId,
    },
    IncompleteCoverage {
        expected: usize,
        supplied: usize,
    },
    DecisionKindMismatch {
        work: WorkId,
    },
    ActionSubmissionRequiresCommand {
        work: WorkId,
    },
    UnknownEvidenceEvent {
        work: WorkId,
        event_index: u32,
    },
    DuplicateEvidenceObservation {
        work: WorkId,
        observer: ActorId,
        event_index: u32,
    },
    PreparedFireMismatch,
}

/// Complete, canonically correlated decisions for one prepared moment.
pub struct MomentWorkProposals {
    step: AttemptStepId,
    entries: Vec<CorrelatedWorkProposal>,
}

impl MomentWorkProposals {
    pub(crate) fn from_decisions(
        prepared: &PreparedFire,
        mut decisions: Vec<MomentWorkDecision>,
    ) -> Result<Self, ProposalBuildError> {
        decisions.sort_by_key(|decision| decision.work);
        if let Some(pair) = decisions
            .windows(2)
            .find(|pair| pair[0].work == pair[1].work)
        {
            return Err(ProposalBuildError::DuplicateWork { work: pair[0].work });
        }

        for decision in &decisions {
            let Some(expected) = prepared
                .work
                .binary_search_by_key(&decision.work, |work| work.id)
                .ok()
                .map(|position| &prepared.work[position])
            else {
                return Err(ProposalBuildError::UnknownWork {
                    work: decision.work,
                });
            };
            let kind_matches = proposal_matches(&expected.kind, &decision.proposal);
            if !kind_matches {
                return Err(ProposalBuildError::DecisionKindMismatch {
                    work: decision.work,
                });
            }
        }

        if decisions.len() != prepared.work.len() {
            return Err(ProposalBuildError::IncompleteCoverage {
                expected: prepared.work.len(),
                supplied: decisions.len(),
            });
        }
        Ok(Self {
            step: prepared.step,
            entries: decisions
                .into_iter()
                .map(|decision| CorrelatedWorkProposal {
                    work: decision.work,
                    proposal: decision.proposal,
                })
                .collect(),
        })
    }

    pub(crate) fn proposal(&self, work: WorkId) -> Option<&WorkProposal> {
        self.entries
            .binary_search_by_key(&work, |entry| entry.work)
            .ok()
            .map(|position| &self.entries[position].proposal)
    }

    fn validate_for(&self, prepared: &PreparedFire) -> Result<(), ProposalBuildError> {
        if self.step != prepared.step
            || self.entries.len() != prepared.work.len()
            || self
                .entries
                .iter()
                .zip(&prepared.work)
                .any(|(entry, work)| {
                    entry.work != work.id || !proposal_matches(&work.kind, &entry.proposal)
                })
        {
            return Err(ProposalBuildError::PreparedFireMismatch);
        }
        Ok(())
    }
}

fn proposal_matches(kind: &PreparedWorkKind, proposal: &WorkProposal) -> bool {
    matches!(
        (kind, proposal),
        (PreparedWorkKind::Command { .. }, WorkProposal::Command(_))
            | (
                PreparedWorkKind::PostCommit { .. },
                WorkProposal::PostCommit(_)
            )
            | (
                PreparedWorkKind::EvidenceAssimilation { .. },
                WorkProposal::EvidenceAssimilation { .. }
            )
            | (
                PreparedWorkKind::Appraisal { .. },
                WorkProposal::Appraisal(_)
            )
            | (
                PreparedWorkKind::IntentReview { .. },
                WorkProposal::IntentReview(_)
            )
            | (
                PreparedWorkKind::ActivityInitialization { .. },
                WorkProposal::ActivityInitialization(_)
            )
            | (
                PreparedWorkKind::ActionReady { .. },
                WorkProposal::Action(_)
            )
            | (
                PreparedWorkKind::ActionEvaluationResultReady { .. },
                WorkProposal::ActionEvaluation(_)
            )
            | (
                PreparedWorkKind::AttemptResolved { .. },
                WorkProposal::AttemptResolvedConsumed
            )
            | (
                PreparedWorkKind::ActivityAdvance { .. },
                WorkProposal::ActivityAdvance(_)
            )
            | (
                PreparedWorkKind::RelocationProcess { .. },
                WorkProposal::RelocationProcessCompleted
            )
    )
}

/// Stable command classification reported for one consumed delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandFireClassification {
    /// A genuinely new logical command was resolved in this barrier.
    New(CommandAttemptOutcome),
    /// The command reused an exact retained result.
    Retained(CommandAttemptOutcome),
    /// The source-scoped identity reused different command content.
    IdReuseMismatch,
    /// The logical identity has a durable collision outcome.
    IdCollision,
    /// The logical identity lies behind the retained retirement frontier.
    Retired,
}

/// Projection-safe published result for one consumed command delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandFireResolution {
    source: CommandSource,
    command: CommandId,
    classification: CommandFireClassification,
}

impl CommandFireResolution {
    pub(crate) const fn new(
        source: CommandSource,
        command: CommandId,
        classification: CommandFireClassification,
    ) -> Self {
        Self {
            source,
            command,
            classification,
        }
    }

    /// Returns the source namespace of the consumed command.
    #[must_use]
    pub const fn source(self) -> CommandSource {
        self.source
    }

    /// Returns the source-scoped command identity.
    #[must_use]
    pub const fn command(self) -> CommandId {
        self.command
    }

    /// Returns the stable publication classification.
    #[must_use]
    pub const fn classification(self) -> CommandFireClassification {
        self.classification
    }
}

/// Result of one atomically published due moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FireOutcome {
    record: AuthorityRecordId,
    cursor: AuthorityCursor,
    moment: SimMoment,
    command_resolutions: Vec<CommandFireResolution>,
    post_commit_consumed: usize,
    action_opportunities_consumed: Vec<ActionOpportunityId>,
    attempt_resolved: Vec<ActionOpportunityId>,
}

impl FireOutcome {
    pub(crate) fn published(
        record: AuthorityRecordId,
        cursor: AuthorityCursor,
        moment: SimMoment,
        mut command_resolutions: Vec<CommandFireResolution>,
        post_commit_consumed: usize,
        mut action_opportunities_consumed: Vec<ActionOpportunityId>,
        mut attempt_resolved: Vec<ActionOpportunityId>,
    ) -> Self {
        command_resolutions.sort_unstable();
        action_opportunities_consumed.sort_unstable();
        attempt_resolved.sort_unstable();
        Self {
            record,
            cursor,
            moment,
            command_resolutions,
            post_commit_consumed,
            action_opportunities_consumed,
            attempt_resolved,
        }
    }

    /// Returns the published outer record.
    #[must_use]
    pub const fn record(&self) -> AuthorityRecordId {
        self.record
    }

    /// Returns the resulting authority cursor.
    #[must_use]
    pub const fn cursor(&self) -> AuthorityCursor {
        self.cursor
    }

    /// Returns the exact simulation moment consumed by the publication.
    #[must_use]
    pub const fn moment(&self) -> SimMoment {
        self.moment
    }

    /// Returns one canonical result per consumed command delivery.
    #[must_use]
    pub fn command_resolutions(&self) -> &[CommandFireResolution] {
        &self.command_resolutions
    }

    /// Returns the number of post-commit dispatches consumed by the barrier.
    #[must_use]
    pub const fn post_commit_consumed(&self) -> usize {
        self.post_commit_consumed
    }

    /// Returns opportunities terminally consumed by action evaluation.
    #[must_use]
    pub fn action_opportunities_consumed(&self) -> &[ActionOpportunityId] {
        &self.action_opportunities_consumed
    }

    /// Returns outcome-neutral action-attempt wakes consumed by this moment.
    #[must_use]
    pub fn attempt_resolved(&self) -> &[ActionOpportunityId] {
        &self.attempt_resolved
    }
}

/// Closed failure evidence accepted after Fire preparation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedFireFailure {
    /// Host-side evaluation budget exhausted.
    HostBudgetExceeded,
    /// A declared external evaluator failed.
    ExternalFailure,
    /// Engine coordination failed.
    EngineFailure,
}

/// Result of explicitly failing one prepared Fire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedFireFailureOutcome {
    finalization: RunFinalization,
}

impl PreparedFireFailureOutcome {
    pub(crate) const fn finalized(finalization: RunFinalization) -> Self {
        Self { finalization }
    }

    /// Returns the terminal selection produced by the retained disposition.
    #[must_use]
    pub const fn finalization(self) -> RunFinalization {
        self.finalization
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use world_core::{
        ActorId, EntityId, Microstep, SimDuration, SimMoment, SimTime, WorldRevision,
    };
    use world_model::{
        AcceptedState, ActionInteractionScope, ActionOpportunityGeneration, ActionSponsor,
        ActorReactionCause, AgencyState, ContainmentInteractionScope, ContainmentTransferDelta,
        DirectedRoute, DomainState, EpistemicState, SocialState,
    };

    use super::*;
    use crate::action_evaluation::{
        ActionEvaluationArtifactSchemaId, ActionEvaluationCaptureFingerprint,
        ActionEvaluationCaptureId, ActionEvaluationFallbackCause, ActionEvaluationInvocationLedger,
        ActionEvaluationPrivateContinuationArtifact, ActionEvaluationPrivateReadWitnessArtifact,
        ActionEvaluationRequestArtifact, ActionEvaluationResultArtifact, ActionEvaluationResultId,
    };
    use crate::authority::{CapturedInputRecordId, EpochIdentity, ReactionEnvelopeId};
    use crate::execution::{
        DeferredActionAdmissionModeV1, DeferredActionControlV1, EpochLineageId,
        ExternalInputNamespaceId, InitialStateRootId, LifecycleImplementationId,
    };
    use crate::kernel::{AdmitRequest, InputId, fixtures};
    use crate::relocation::RelocationProcessLedger;
    use crate::scheduler::{
        PreparedPostCommitDispatch, PreparedScheduledCommand, ReactionEnvelope, SchedulerInsertion,
        SchedulerProducerOrdinal, SchedulerSequence, SchedulerState,
    };

    fn valid<T, E: Debug>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("Fire boundary fixture must be valid: {error:?}"),
        }
    }

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(ticks), Microstep::new(microstep))
    }

    fn snapshot() -> WorldSnapshot {
        WorldSnapshot::new(
            WorldRevision::ROOT,
            AcceptedState::new(
                valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
                EpistemicState::empty(),
                SocialState::empty(),
                AgencyState::empty(),
            ),
        )
    }

    fn scheduled_command(input: u64, due: SimMoment, command: CommandEnvelope) -> ScheduledWork {
        let request = AdmitRequest::new(InputId::new(input), due, command);
        PreparedScheduledCommand::prepare(
            ExternalInputNamespaceId::from_bytes([0x51; 32]),
            &request,
        )
        .materialize(CapturedInputRecordId::from_bytes([input as u8; 32]))
        .into()
    }

    fn post_commit(source: SimMoment) -> ScheduledWork {
        let delta = valid(ContainmentTransferDelta::new(
            ActorId::from_bytes([0x61; 32]),
            EntityId::from_bytes([0x62; 32]),
            EntityId::from_bytes([0x63; 32]),
            EntityId::from_bytes([0x64; 32]),
        ));
        let dispatch = PreparedPostCommitDispatch::prepare(
            EpochLineageId::from_bytes([0x65; 32]),
            source,
            ReactionEnvelope::from_transfers(&[delta])
                .unwrap_or_else(|| panic!("one transfer must produce one reaction envelope")),
        )
        .materialize(ReactionEnvelopeId::from_bytes([0x66; 32]));
        ScheduledWork::PostCommit(dispatch)
    }

    fn planned(works: Vec<ScheduledWork>) -> Vec<(SchedulerKey, ScheduledWork)> {
        let insertions = works
            .into_iter()
            .enumerate()
            .map(|(position, work)| {
                SchedulerInsertion::new(SchedulerProducerOrdinal::new(position as u32), work)
            })
            .collect();
        valid(SchedulerState::empty().plan_batch(insertions))
            .entries()
            .to_vec()
    }

    fn prepared(deliveries: Vec<PreparedDelivery>) -> PreparedFire {
        valid(PreparedFire::new(
            AttemptAuthorityDomainId::from_bytes([0x71; 32]),
            RunAttemptId::from_bytes([0x72; 32]),
            ExecutionSpecId::from_bytes([0x73; 32]),
            AttemptStepId::from_bytes([0x74; 32]),
            ReservationGrant::FIRST,
            moment(9, 2),
            snapshot(),
            deliveries,
        ))
    }

    fn action_evaluation_delivery(fallback: bool) -> PreparedDelivery {
        let due = moment(9, 2);
        let scope = valid(ContainmentInteractionScope::new(
            EntityId::from_bytes([0x81; 32]),
            vec![EntityId::from_bytes([0x82; 32])],
            vec![EntityId::from_bytes([0x83; 32])],
            4,
        ));
        let open = ActionOpportunity::open(
            ActorId::from_bytes([0x84; 32]),
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x85; 32])),
            ActionInteractionScope::containment(scope),
            ActionOpportunityGeneration::new(1),
        );
        let policy_semantics = [0x86; 32];
        let action_input = [0x87; 32];
        let pre_wait_version = open.version();
        let (waiting, invocation) =
            valid(open.begin_evaluation(pre_wait_version, policy_semantics, action_input));
        let control = valid(DeferredActionControlV1::enabled(
            DeferredActionAdmissionModeV1::HostScheduled,
            1,
            16,
            16,
            16,
            16,
        ));
        let request = valid(ActionEvaluationRequestArtifact::new(
            ActionEvaluationArtifactSchemaId::from_bytes([0x88; 32]),
            vec![0x89; 2],
            control,
        ));
        let result_schema = ActionEvaluationArtifactSchemaId::from_bytes([0x8a; 32]);
        let record = valid(ActionEvaluationInvocationRecord::dispatch_pending(
            invocation,
            waiting.id(),
            pre_wait_version,
            waiting.version(),
            waiting.evaluation_generation(),
            policy_semantics,
            action_input,
            LifecycleImplementationId::from_bytes([0x8b; 32]),
            request,
            result_schema,
            valid(ActionEvaluationPrivateContinuationArtifact::new(
                ActionEvaluationArtifactSchemaId::from_bytes([0x8c; 32]),
                vec![0x8d; 3],
                control,
            )),
            valid(ActionEvaluationPrivateReadWitnessArtifact::new(
                ActionEvaluationArtifactSchemaId::from_bytes([0x8e; 32]),
                vec![0x8f; 4],
                control,
            )),
            moment(9, 1),
            AuthorityCursor::root(
                EpochIdentity::new(
                    EpochLineageId::from_bytes([0x90; 32]),
                    ExecutionSpecId::from_bytes([0x91; 32]),
                ),
                InitialStateRootId::from_bytes([0x92; 32]),
            ),
            None,
            control,
        ));
        let key = SchedulerKey::new(
            due,
            SchedulerLaneV2::ActionEvaluation,
            SchedulerSequence::new(1),
        );
        let mut ledger = ActionEvaluationInvocationLedger::default();
        valid(ledger.install_dispatch(record, &waiting));
        let work = if fallback {
            valid(ledger.begin_managed_fallback(
                invocation,
                waiting.version(),
                ActionEvaluationFallbackCause::HostFailure,
                key,
            ));
            ActionEvaluationWork::fallback(
                invocation,
                waiting.id(),
                waiting.version(),
                ActionEvaluationFallbackCause::HostFailure,
                due,
            )
        } else {
            let result = valid(ActionEvaluationResultArtifact::new(
                result_schema,
                vec![0x93; 3],
                control,
            ));
            let request_id = ledger
                .get(invocation)
                .and_then(ActionEvaluationInvocationRecord::request_id)
                .unwrap_or_else(|| panic!("dispatch fixture must retain request identity"));
            let result_id = ActionEvaluationResultId::derive(request_id, &result);
            let fingerprint = ActionEvaluationCaptureFingerprint::derive(
                invocation,
                request_id,
                result_id,
                due,
                DeferredActionAdmissionModeV1::HostScheduled,
                &result,
            );
            valid(ledger.capture_result(
                invocation,
                waiting.version(),
                ActionEvaluationCaptureId::new(1),
                fingerprint,
                result,
                due,
                key,
                control,
            ));
            ActionEvaluationWork::result_ready(invocation, waiting.id(), waiting.version(), due)
        };
        let retained = ledger
            .get(invocation)
            .unwrap_or_else(|| panic!("evaluation fixture must remain retained"))
            .clone();
        PreparedDelivery::action_evaluation(key, work, waiting, retained)
    }

    #[test]
    fn obsolete_relocation_wake_requires_no_engine_work() {
        let actor = ActorId::from_bytes([0x31; 32]);
        let route = valid(DirectedRoute::new(
            EntityId::from_bytes([0x32; 32]),
            EntityId::from_bytes([0x33; 32]),
            SimDuration::from_ticks(9),
        ));
        let mut ledger = RelocationProcessLedger::default();
        let active = valid(ledger.start(actor, route, SimTime::ZERO));
        let obsolete = RelocationProcessWake::for_active(active)
            .unwrap_or_else(|| panic!("active relocation must have one wake"));
        let (_, paused) =
            valid(ledger.pause(active.id(), active.version(), SimTime::from_ticks(1)));
        let _ = valid(ledger.resume(paused.id(), paused.version(), SimTime::from_ticks(2)));
        let entries = planned(vec![ScheduledWork::process(obsolete)]);
        let [(key, ScheduledWork::Process(wake))] = entries.as_slice() else {
            panic!("one process wake must produce one process-lane entry");
        };
        let prepared = prepared(vec![PreparedDelivery::process(
            *key,
            *wake,
            ledger.classify_wake(*wake),
        )]);

        assert_eq!(prepared.work().len(), 0);
        assert!(matches!(
            prepared.deliveries(),
            [PreparedDelivery::Process {
                classification: RelocationWakeClassification::Obsolete,
                ..
            }]
        ));
        assert!(MomentWorkProposals::from_decisions(&prepared, Vec::new()).is_ok());
    }

    #[test]
    fn only_result_ready_exposes_snapshot_bearing_engine_work() {
        let result_ready = prepared(vec![action_evaluation_delivery(false)]);
        let inputs = result_ready.work().collect::<Vec<_>>();
        let [
            MomentWorkInput::ActionEvaluationResultReady {
                due,
                snapshot,
                result_ready: binding,
                opportunity,
                invocation,
                ..
            },
        ] = inputs.as_slice()
        else {
            panic!("captured result must expose exactly one result-ready input");
        };
        assert_eq!(*due, moment(9, 2));
        assert_eq!(binding.due(), *due);
        assert_eq!(binding.invocation(), invocation.invocation());
        assert_eq!(binding.opportunity(), opportunity.id());
        assert_eq!(binding.expected_waiting_version(), opportunity.version());
        assert!(core::ptr::eq(*snapshot, result_ready.base_snapshot()));

        let fallback = prepared(vec![action_evaluation_delivery(true)]);
        assert_eq!(fallback.work().len(), 0);
        assert!(MomentWorkProposals::from_decisions(&fallback, Vec::new()).is_ok());
    }

    impl From<ScheduledCommand> for ScheduledWork {
        fn from(command: ScheduledCommand) -> Self {
            Self::command(command)
        }
    }

    #[test]
    fn opaque_work_covers_only_unique_evaluable_commands_and_dispatches() {
        let due = moment(9, 2);
        let exact = fixtures::command(0x11, 7);
        let resolved = fixtures::command(0x12, 8);
        let entries = planned(vec![
            scheduled_command(1, due, exact.clone()),
            scheduled_command(2, due, exact),
            scheduled_command(3, due, resolved),
            post_commit(moment(9, 1)),
        ]);
        let original_attempt = AttemptRecordId::from_bytes([0x75; 32]);
        let deliveries = entries
            .into_iter()
            .map(|(key, work)| match work {
                ScheduledWork::Command(scheduled) if scheduled.input() == Some(InputId::new(3)) => {
                    PreparedDelivery::resolved_command(
                        key,
                        *scheduled,
                        PreparedCommandResolution::Retained {
                            original_attempt,
                            outcome: CommandAttemptOutcome::Accepted,
                        },
                    )
                }
                ScheduledWork::Command(scheduled) => {
                    PreparedDelivery::evaluable_command(key, *scheduled)
                }
                ScheduledWork::PostCommit(dispatch) => PreparedDelivery::post_commit(key, dispatch),
                ScheduledWork::ActionReady(_)
                | ScheduledWork::ActionEvaluation(_)
                | ScheduledWork::Lifecycle(_)
                | ScheduledWork::Process(_) => {
                    unreachable!("fixture contains only commands and post-commit work")
                }
            })
            .collect();
        let prepared = prepared(deliveries);
        let work: Vec<_> = prepared.work().collect();

        assert_eq!(prepared.deliveries().len(), 4);
        assert_eq!(work.len(), 2);
        assert!(matches!(
            work[0],
            MomentWorkInput::EvaluateCommand { due: actual, .. } if actual == due
        ));
        assert!(matches!(
            work[1],
            MomentWorkInput::PostCommitDispatch { due: actual, .. } if actual == due
        ));
        assert_ne!(work[0].work_id(), work[1].work_id());

        let duplicate_ids: Vec<_> = prepared
            .deliveries()
            .iter()
            .enumerate()
            .filter_map(|(position, delivery)| match delivery {
                PreparedDelivery::EvaluableCommand { .. } => {
                    prepared.work_id_for_delivery(position)
                }
                PreparedDelivery::ResolvedCommand { .. } | PreparedDelivery::PostCommit { .. } => {
                    None
                }
                PreparedDelivery::EvidenceDelivery { .. }
                | PreparedDelivery::Appraisal { .. }
                | PreparedDelivery::IntentReview { .. }
                | PreparedDelivery::ActivityInitialization { .. }
                | PreparedDelivery::ActionReady { .. }
                | PreparedDelivery::ActionEvaluation { .. }
                | PreparedDelivery::AttemptResolved { .. }
                | PreparedDelivery::ActivityAdvance { .. }
                | PreparedDelivery::Process { .. } => {
                    unreachable!("fixture contains only commands and post-commit work")
                }
            })
            .collect();
        assert_eq!(duplicate_ids, vec![work[0].work_id(), work[0].work_id()]);
        assert!(
            prepared
                .deliveries()
                .iter()
                .enumerate()
                .any(|(position, delivery)| matches!(
                    delivery,
                    PreparedDelivery::ResolvedCommand { .. }
                ) && prepared.work_id_for_delivery(position).is_none())
        );
    }

    #[test]
    fn command_inputs_borrow_one_shared_immutable_base() {
        let due = moment(9, 2);
        let entries = planned(vec![
            scheduled_command(1, due, fixtures::command(0x21, 1)),
            scheduled_command(2, due, fixtures::command(0x22, 2)),
        ]);
        let deliveries = entries
            .into_iter()
            .map(|(key, work)| match work {
                ScheduledWork::Command(scheduled) => {
                    PreparedDelivery::evaluable_command(key, *scheduled)
                }
                ScheduledWork::PostCommit(_) => unreachable!("fixture contains only commands"),
                ScheduledWork::ActionReady(_)
                | ScheduledWork::ActionEvaluation(_)
                | ScheduledWork::Lifecycle(_)
                | ScheduledWork::Process(_) => {
                    unreachable!("fixture contains only commands")
                }
            })
            .collect();
        let prepared = prepared(deliveries);
        let snapshots: Vec<_> = prepared
            .work()
            .map(|input| match input {
                MomentWorkInput::EvaluateCommand { snapshot, .. } => snapshot,
                MomentWorkInput::PostCommitDispatch { .. } => {
                    unreachable!("fixture contains only commands")
                }
                MomentWorkInput::EvidenceAssimilation { .. }
                | MomentWorkInput::Appraisal { .. }
                | MomentWorkInput::IntentReview { .. }
                | MomentWorkInput::ActivityInitialization { .. }
                | MomentWorkInput::ActionReady { .. }
                | MomentWorkInput::ActionEvaluationResultReady { .. }
                | MomentWorkInput::AttemptResolved { .. }
                | MomentWorkInput::ActivityAdvance { .. }
                | MomentWorkInput::RelocationProcessWake { .. } => {
                    unreachable!("fixture contains only commands")
                }
            })
            .collect();

        assert_eq!(snapshots.len(), 2);
        assert!(core::ptr::eq(snapshots[0], prepared.base_snapshot()));
        assert!(core::ptr::eq(snapshots[0], snapshots[1]));
    }

    #[test]
    fn proposal_collection_rejects_duplicates_and_canonicalizes_completion_order() {
        let due = moment(9, 2);
        let entries = planned(vec![
            scheduled_command(1, due, fixtures::command(0x31, 1)),
            scheduled_command(2, due, fixtures::command(0x32, 2)),
        ]);
        let deliveries = entries
            .into_iter()
            .map(|(key, work)| match work {
                ScheduledWork::Command(scheduled) => {
                    PreparedDelivery::evaluable_command(key, *scheduled)
                }
                ScheduledWork::PostCommit(_) => unreachable!("fixture contains only commands"),
                ScheduledWork::ActionReady(_)
                | ScheduledWork::ActionEvaluation(_)
                | ScheduledWork::Lifecycle(_)
                | ScheduledWork::Process(_) => {
                    unreachable!("fixture contains only commands")
                }
            })
            .collect();
        let prepared = prepared(deliveries);
        let ids: Vec<_> = prepared.work().map(MomentWorkInput::work_id).collect();
        let rejected = CommandProposal::Rejected(StableCommandRejection::RequirementUnsatisfied);

        let out_of_order = valid(MomentWorkProposals::from_decisions(
            &prepared,
            vec![
                MomentWorkDecision::command(ids[1], rejected),
                MomentWorkDecision::command(ids[0], rejected),
            ],
        ));
        assert_eq!(prepared.validate_proposals(&out_of_order), Ok(()));
        let rejected_work = WorkProposal::Command(rejected);
        assert_eq!(out_of_order.proposal(ids[0]), Some(&rejected_work));
        assert_eq!(out_of_order.proposal(ids[1]), Some(&rejected_work));

        assert!(matches!(
            MomentWorkProposals::from_decisions(
                &prepared,
                vec![
                    MomentWorkDecision::command(ids[0], rejected),
                    MomentWorkDecision::command(ids[0], rejected),
                ],
            ),
            Err(ProposalBuildError::DuplicateWork { work }) if work == ids[0]
        ));
        assert!(matches!(
            MomentWorkProposals::from_decisions(
                &prepared,
                vec![MomentWorkDecision::command(ids[0], rejected)],
            ),
            Err(ProposalBuildError::IncompleteCoverage {
                expected: 2,
                supplied: 1,
            })
        ));
    }

    #[test]
    fn proposal_collection_is_invariant_to_completion_order_and_logical_worker_count() {
        let due = moment(9, 2);
        let entries = planned(vec![
            scheduled_command(1, due, fixtures::command(0x41, 1)),
            scheduled_command(2, due, fixtures::command(0x42, 2)),
            scheduled_command(3, due, fixtures::command(0x43, 3)),
        ]);
        let deliveries = entries
            .into_iter()
            .map(|(key, work)| match work {
                ScheduledWork::Command(scheduled) => {
                    PreparedDelivery::evaluable_command(key, *scheduled)
                }
                ScheduledWork::PostCommit(_) => unreachable!("fixture contains only commands"),
                ScheduledWork::ActionReady(_)
                | ScheduledWork::ActionEvaluation(_)
                | ScheduledWork::Lifecycle(_)
                | ScheduledWork::Process(_) => {
                    unreachable!("fixture contains only commands")
                }
            })
            .collect();
        let prepared = prepared(deliveries);
        let ids: Vec<_> = prepared.work().map(MomentWorkInput::work_id).collect();
        let decisions = [
            MomentWorkDecision::command(
                ids[0],
                CommandProposal::Rejected(StableCommandRejection::RequirementUnsatisfied),
            ),
            MomentWorkDecision::command(
                ids[1],
                CommandProposal::Rejected(StableCommandRejection::BindingMismatch),
            ),
            MomentWorkDecision::command(
                ids[2],
                CommandProposal::Rejected(StableCommandRejection::Stale),
            ),
        ];
        let permutations = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];

        for worker_count in 1..=3 {
            for permutation in permutations {
                let mut workers = vec![Vec::new(); worker_count];
                for (position, decision) in permutation
                    .into_iter()
                    .map(|index| decisions[index].clone())
                    .enumerate()
                {
                    workers[position % worker_count].push(decision);
                }
                let completion_order = workers.into_iter().rev().flatten().collect();
                let proposals = valid(MomentWorkProposals::from_decisions(
                    &prepared,
                    completion_order,
                ));

                assert_eq!(prepared.validate_proposals(&proposals), Ok(()));
                for decision in &decisions {
                    assert_eq!(proposals.proposal(decision.work), Some(&decision.proposal));
                }
            }
        }
    }

    #[test]
    fn fire_outcome_canonicalizes_complete_projection_safe_results() {
        let execution = ExecutionSpecId::from_bytes([0x81; 32]);
        let cursor = AuthorityCursor::root(
            EpochIdentity::new(EpochLineageId::from_bytes([0x82; 32]), execution),
            InitialStateRootId::from_bytes([0x83; 32]),
        );
        let record = AuthorityRecordId::from_bytes([0x84; 32]);
        let due = moment(9, 2);
        let resolutions = vec![
            CommandFireResolution::new(
                CommandSource::from_bytes([0x30; 32]),
                CommandId::new(3),
                CommandFireClassification::Retired,
            ),
            CommandFireResolution::new(
                CommandSource::from_bytes([0x10; 32]),
                CommandId::new(1),
                CommandFireClassification::New(CommandAttemptOutcome::Accepted),
            ),
            CommandFireResolution::new(
                CommandSource::from_bytes([0x20; 32]),
                CommandId::new(2),
                CommandFireClassification::IdCollision,
            ),
        ];

        let outcome =
            FireOutcome::published(record, cursor, due, resolutions, 2, Vec::new(), Vec::new());

        assert_eq!(outcome.record(), record);
        assert_eq!(outcome.cursor(), cursor);
        assert_eq!(outcome.moment(), due);
        assert_eq!(outcome.post_commit_consumed(), 2);
        assert_eq!(
            outcome
                .command_resolutions()
                .iter()
                .map(|resolution| resolution.source())
                .collect::<Vec<_>>(),
            vec![
                CommandSource::from_bytes([0x10; 32]),
                CommandSource::from_bytes([0x20; 32]),
                CommandSource::from_bytes([0x30; 32]),
            ]
        );
        assert_eq!(
            outcome.command_resolutions()[0].command(),
            CommandId::new(1)
        );
        assert_eq!(
            outcome.command_resolutions()[0].classification(),
            CommandFireClassification::New(CommandAttemptOutcome::Accepted)
        );
    }
}
