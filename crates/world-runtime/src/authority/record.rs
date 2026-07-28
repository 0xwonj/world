use std::collections::{BTreeMap, BTreeSet};

use world_core::{
    CanonicalBytes, CanonicalDomain, CanonicalWriter, NonZeroWorldRevision, SimMoment,
};
use world_model::{
    ActionOpportunity, ActionOpportunityId, Activity, CommandAttemptOutcome, CommandEnvelope,
    CommandId, CommandRequestFingerprint, CommandSource, ContainmentAppraisal,
    ContainmentTransferDelta, DomainStateError, EpistemicState, EpistemicVersion,
    EvidenceDeliveryId, EvidenceRecord, Intent, PhysicalEvent, RelocationInteraction,
    RelocationProcess, RelocationProcessId, StableCommandRejection,
};

use crate::action_evaluation::{
    ActionEvaluationArtifactRole, ActionEvaluationCaptureId, ActionEvaluationCaptureOutcome,
    ActionEvaluationCaptureRequest, ActionEvaluationFallbackCause,
    ActionEvaluationInvocationDigest, ActionEvaluationInvocationRecord, ActionEvaluationWork,
};
use crate::execution::{
    ContainmentConflictPolicyV1, EpochLineageId, MomentResolutionPolicyV2, RandomKeyPolicyV1,
    RandomOraclePolicyV1,
};
use crate::kernel::{
    ActionEvaluationDecision, ActionProposal, ActivityAdvanceResult, ActivityInitializationResult,
    AdmitOutcome, AdmitRequest, AppraisalResult, CommandProposal, ContainmentCandidateOutcome,
    ContainmentCommandIdentity, ContainmentMomentResolution, ContainmentResolutionEvidence,
    ContainmentResolutionFallback, InputId, InputRequestFingerprint, IntentReviewResult,
    KernelSafetyCause, LedgerRetirement, ManageOutcome, ManageRequest,
    ManagementRequestFingerprint, ManagementRequestId, MomentWorkProposals,
    PreparedCommandResolution, PreparedDelivery, PreparedFire, ProposalBuildError,
    SessionManagement, WorkProposal,
};
use crate::lifecycle::{
    EvidenceDeliveryWork, EvidenceObservation, LifecycleCause, LifecycleGeneration, LifecycleRole,
    LifecycleWork,
};
use crate::randomness::{
    ContainmentConflictContenderV1, ContainmentConflictGroupV1, ContainmentConflictResourceV1,
    ContainmentRandomRankError,
};
use crate::relocation::{RelocationProcessWake, RelocationWakeClassification};
use crate::scheduler::{
    ActionReady, AttemptResolved, CommandTriggerId, PostCommitDispatch, PreparedPostCommitDispatch,
    PreparedScheduledCommand, ReactionEnvelope, ScheduledCommand, ScheduledCommandCause,
    ScheduledWork, SchedulerKey,
};
use crate::session::SessionMode;

use super::{
    AttemptRecordId, AuthorityCursor, AuthorityRecordId, CapturedInputRecordId, CommitRecordId,
    ContainmentTransitionError, CumulativeAuthorityHash, NonZeroRunRecordSeq,
    PreviousAuthorityHash, ReactionEnvelopeId, apply_containment_transfers,
};

/// Canonical schema of an outer authority record.
pub const AUTHORITY_RECORD_SCHEMA_VERSION: u16 = 3;
pub(crate) const CUMULATIVE_AUTHORITY_SCHEMA_VERSION: u16 = 1;

const AUTHORITY_RECORD_DOMAIN: CanonicalDomain = match CanonicalDomain::new("authority-record-v3") {
    Ok(domain) => domain,
    Err(_) => panic!("authority record domain must be valid"),
};
const CUMULATIVE_AUTHORITY_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("cumulative-authority-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("cumulative authority domain must be valid"),
    };

/// Immutable predecessor and identity context of one authority record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityRecordHeader {
    lineage: EpochLineageId,
    revision: NonZeroWorldRevision,
    sequence: NonZeroRunRecordSeq,
    previous_authority: PreviousAuthorityHash,
    previous_cumulative: CumulativeAuthorityHash,
    id: AuthorityRecordId,
    cumulative: CumulativeAuthorityHash,
}

impl AuthorityRecordHeader {
    pub(crate) const fn new(
        lineage: EpochLineageId,
        revision: NonZeroWorldRevision,
        sequence: NonZeroRunRecordSeq,
        previous_authority: PreviousAuthorityHash,
        previous_cumulative: CumulativeAuthorityHash,
        id: AuthorityRecordId,
        cumulative: CumulativeAuthorityHash,
    ) -> Self {
        Self {
            lineage,
            revision,
            sequence,
            previous_authority,
            previous_cumulative,
            id,
            cumulative,
        }
    }

    #[must_use]
    pub const fn lineage(&self) -> EpochLineageId {
        self.lineage
    }
    #[must_use]
    pub const fn revision(&self) -> NonZeroWorldRevision {
        self.revision
    }
    #[must_use]
    pub const fn sequence(&self) -> NonZeroRunRecordSeq {
        self.sequence
    }
    #[cfg(test)]
    #[must_use]
    pub const fn previous_authority_bytes(&self) -> &[u8; 32] {
        self.previous_authority.as_bytes()
    }
    #[cfg(test)]
    #[must_use]
    pub const fn previous_cumulative(&self) -> CumulativeAuthorityHash {
        self.previous_cumulative
    }
    #[must_use]
    pub const fn id(&self) -> AuthorityRecordId {
        self.id
    }
    #[must_use]
    pub const fn cumulative(&self) -> CumulativeAuthorityHash {
        self.cumulative
    }
}

/// One immutable published authority transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityRecord {
    header: AuthorityRecordHeader,
    body: AuthorityRecordBody,
}

impl AuthorityRecord {
    pub(crate) const fn new(header: AuthorityRecordHeader, body: AuthorityRecordBody) -> Self {
        Self { header, body }
    }
    #[must_use]
    pub const fn header(&self) -> &AuthorityRecordHeader {
        &self.header
    }
    #[must_use]
    pub const fn body(&self) -> &AuthorityRecordBody {
        &self.body
    }
}

/// Closed family of authoritative transition bodies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthorityRecordBody {
    Admission(AuthorityAdmissionRecord),
    Moment(Box<MomentBatchRecord>),
    Management(Box<ManagementBatchRecord>),
}

/// Closed family of protocols that admit host-owned input into authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthorityAdmissionRecord {
    /// One canonical batch of external commands.
    Commands(IngressBatchRecord),
    /// One captured result for a retained action-evaluation invocation.
    ActionEvaluation(Box<ActionEvaluationAdmissionRecord>),
}

/// One captured external input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedInputRecord {
    id: CapturedInputRecordId,
    input: InputId,
    request: InputRequestFingerprint,
    effective: SimMoment,
    command: CommandEnvelope,
}

impl CapturedInputRecord {
    pub(crate) fn new(id: CapturedInputRecordId, prepared: &PreparedScheduledCommand) -> Self {
        Self {
            id,
            input: prepared.input(),
            request: prepared.request_fingerprint(),
            effective: prepared.effective(),
            command: prepared.command().clone(),
        }
    }
    #[must_use]
    pub const fn id(&self) -> CapturedInputRecordId {
        self.id
    }
    #[must_use]
    pub const fn input(&self) -> InputId {
        self.input
    }
    #[must_use]
    pub const fn request_fingerprint(&self) -> InputRequestFingerprint {
        self.request
    }
    #[must_use]
    pub const fn effective(&self) -> SimMoment {
        self.effective
    }
    #[cfg(test)]
    #[must_use]
    pub const fn command(&self) -> &CommandEnvelope {
        &self.command
    }
}

/// One atomic input capture, retained outcome, and scheduler insertion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngressRecord {
    captured: CapturedInputRecord,
    trigger: CommandTriggerId,
    scheduler_key: SchedulerKey,
    scheduled_command: ScheduledCommand,
    outcome: AdmitOutcome,
}

impl IngressRecord {
    pub(crate) const fn new(
        captured: CapturedInputRecord,
        trigger: CommandTriggerId,
        scheduler_key: SchedulerKey,
        scheduled_command: ScheduledCommand,
        outcome: AdmitOutcome,
    ) -> Self {
        Self {
            captured,
            trigger,
            scheduler_key,
            scheduled_command,
            outcome,
        }
    }
    #[must_use]
    pub const fn captured(&self) -> &CapturedInputRecord {
        &self.captured
    }
    #[must_use]
    pub const fn trigger(&self) -> CommandTriggerId {
        self.trigger
    }
    #[must_use]
    pub const fn scheduler_key(&self) -> SchedulerKey {
        self.scheduler_key
    }
    #[must_use]
    pub const fn outcome(&self) -> AdmitOutcome {
        self.outcome
    }
    pub(crate) const fn scheduled_command(&self) -> &ScheduledCommand {
        &self.scheduled_command
    }
}

/// One canonical collection of atomically admitted external inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngressBatchRecord {
    entries: Vec<IngressRecord>,
}

impl IngressBatchRecord {
    pub(crate) const fn new(entries: Vec<IngressRecord>) -> Self {
        Self { entries }
    }

    /// Returns admitted inputs in canonical scheduler-insertion order.
    #[must_use]
    pub fn entries(&self) -> &[IngressRecord] {
        &self.entries
    }
}

/// One atomic action-evaluation capture and its scheduler effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionEvaluationAdmissionRecord {
    request: ActionEvaluationCaptureRequest,
    outcome: ActionEvaluationCaptureOutcome,
    transition: ActionEvaluationInvocationTransitionRecord,
    scheduler_insertion: SchedulerInsertionRecord,
}

impl ActionEvaluationAdmissionRecord {
    pub(crate) const fn new(
        request: ActionEvaluationCaptureRequest,
        outcome: ActionEvaluationCaptureOutcome,
        transition: ActionEvaluationInvocationTransitionRecord,
        scheduler_insertion: SchedulerInsertionRecord,
    ) -> Self {
        Self {
            request,
            outcome,
            transition,
            scheduler_insertion,
        }
    }

    pub(crate) const fn request(&self) -> &ActionEvaluationCaptureRequest {
        &self.request
    }

    #[must_use]
    pub const fn outcome(&self) -> ActionEvaluationCaptureOutcome {
        self.outcome
    }

    #[must_use]
    pub const fn transition(&self) -> &ActionEvaluationInvocationTransitionRecord {
        &self.transition
    }

    #[must_use]
    pub const fn scheduler_insertion(&self) -> &SchedulerInsertionRecord {
        &self.scheduler_insertion
    }
}

/// Durable resolution of a newly evaluated command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordedCommandResolution {
    Accepted { commit: CommitRecordId },
    Rejected(StableCommandRejection),
    CommandIdCollision,
}

impl RecordedCommandResolution {
    #[must_use]
    pub const fn outcome(self) -> CommandAttemptOutcome {
        match self {
            Self::Accepted { .. } => CommandAttemptOutcome::Accepted,
            Self::Rejected(reason) => CommandAttemptOutcome::Rejected(reason),
            Self::CommandIdCollision => {
                CommandAttemptOutcome::Rejected(StableCommandRejection::IdCollision)
            }
        }
    }
}

/// Canonical semantic subject of one logical command attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttemptSubjectRecord {
    EvaluatedCommand(CommandEnvelope),
    CommandIdCollision {
        source: CommandSource,
        command: CommandId,
        fingerprints: Box<[CommandRequestFingerprint]>,
    },
}

impl AttemptSubjectRecord {
    #[cfg(test)]
    #[must_use]
    pub const fn command(&self) -> Option<&CommandEnvelope> {
        match self {
            Self::EvaluatedCommand(command) => Some(command),
            Self::CommandIdCollision { .. } => None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub const fn collision(
        &self,
    ) -> Option<(CommandSource, CommandId, &[CommandRequestFingerprint])> {
        match self {
            Self::EvaluatedCommand(_) => None,
            Self::CommandIdCollision {
                source,
                command,
                fingerprints,
            } => Some((*source, *command, fingerprints)),
        }
    }
}

/// One newly established logical command attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptRecord {
    id: AttemptRecordId,
    subject: AttemptSubjectRecord,
    resolution: RecordedCommandResolution,
}

impl AttemptRecord {
    pub(crate) const fn new(
        id: AttemptRecordId,
        subject: AttemptSubjectRecord,
        resolution: RecordedCommandResolution,
    ) -> Self {
        Self {
            id,
            subject,
            resolution,
        }
    }
    #[must_use]
    pub const fn id(&self) -> AttemptRecordId {
        self.id
    }
    #[must_use]
    pub const fn subject(&self) -> &AttemptSubjectRecord {
        &self.subject
    }
    #[cfg(test)]
    #[must_use]
    pub const fn command(&self) -> Option<&CommandEnvelope> {
        self.subject.command()
    }
    #[must_use]
    pub const fn resolution(&self) -> RecordedCommandResolution {
        self.resolution
    }
}

/// One accepted transfer commit and its exactly derived event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainmentTransferCommitRecord {
    id: CommitRecordId,
    delta: ContainmentTransferDelta,
    event: PhysicalEvent,
}

impl ContainmentTransferCommitRecord {
    pub(crate) const fn new(id: CommitRecordId, delta: ContainmentTransferDelta) -> Self {
        Self {
            id,
            delta,
            event: PhysicalEvent::item_transferred(delta),
        }
    }
    #[cfg(test)]
    #[must_use]
    pub const fn id(self) -> CommitRecordId {
        self.id
    }
    #[cfg(test)]
    #[must_use]
    pub const fn delta(self) -> ContainmentTransferDelta {
        self.delta
    }
    #[cfg(test)]
    #[must_use]
    pub const fn event(self) -> PhysicalEvent {
        self.event
    }
}

/// One derived, nonempty reaction envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactionEnvelopeRecord {
    id: ReactionEnvelopeId,
    envelope: ReactionEnvelope,
}

impl ReactionEnvelopeRecord {
    pub(crate) const fn new(id: ReactionEnvelopeId, envelope: ReactionEnvelope) -> Self {
        Self { id, envelope }
    }
    #[cfg(test)]
    #[must_use]
    pub const fn id(&self) -> ReactionEnvelopeId {
        self.id
    }
    #[cfg(test)]
    #[must_use]
    pub const fn envelope(&self) -> &ReactionEnvelope {
        &self.envelope
    }
}

/// One command consumed at its exact scheduler coordinate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandDeliveryRecord {
    scheduler_key: SchedulerKey,
    scheduled: ScheduledCommand,
}

impl CommandDeliveryRecord {
    pub(crate) fn new(scheduler_key: SchedulerKey, scheduled: &ScheduledCommand) -> Self {
        Self {
            scheduler_key,
            scheduled: scheduled.clone(),
        }
    }
    #[must_use]
    pub const fn scheduler_key(&self) -> SchedulerKey {
        self.scheduler_key
    }
    #[must_use]
    pub const fn command(&self) -> &CommandEnvelope {
        self.scheduled.command()
    }
    #[must_use]
    pub(crate) const fn scheduled(&self) -> &ScheduledCommand {
        &self.scheduled
    }
}

/// One post-commit dispatch consumed at its exact scheduler coordinate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostCommitDeliveryRecord {
    scheduler_key: SchedulerKey,
    dispatch: PostCommitDispatch,
}

impl PostCommitDeliveryRecord {
    pub(crate) const fn new(scheduler_key: SchedulerKey, dispatch: PostCommitDispatch) -> Self {
        Self {
            scheduler_key,
            dispatch,
        }
    }

    #[must_use]
    pub const fn scheduler_key(&self) -> SchedulerKey {
        self.scheduler_key
    }

    #[must_use]
    pub const fn dispatch(&self) -> &PostCommitDispatch {
        &self.dispatch
    }
}

/// One action opportunity consumed at its exact ready-work coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionReadyDeliveryRecord {
    scheduler_key: SchedulerKey,
    ready: ActionReady,
}

impl ActionReadyDeliveryRecord {
    pub(crate) const fn new(scheduler_key: SchedulerKey, ready: ActionReady) -> Self {
        Self {
            scheduler_key,
            ready,
        }
    }

    #[must_use]
    pub const fn scheduler_key(self) -> SchedulerKey {
        self.scheduler_key
    }

    #[must_use]
    pub const fn ready(self) -> ActionReady {
        self.ready
    }
}

/// One retained action-evaluation wake consumed at its exact scheduler
/// coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionEvaluationDeliveryRecord {
    scheduler_key: SchedulerKey,
    work: ActionEvaluationWork,
}

impl ActionEvaluationDeliveryRecord {
    pub(crate) const fn new(scheduler_key: SchedulerKey, work: ActionEvaluationWork) -> Self {
        Self {
            scheduler_key,
            work,
        }
    }

    #[must_use]
    pub const fn scheduler_key(self) -> SchedulerKey {
        self.scheduler_key
    }

    #[must_use]
    pub const fn work(self) -> ActionEvaluationWork {
        self.work
    }
}

/// One neutral attempt-resolution wake consumed at its exact coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttemptResolvedDeliveryRecord {
    scheduler_key: SchedulerKey,
    resolved: AttemptResolved,
}

/// One deterministic lifecycle work item consumed at its exact coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifecycleDeliveryRecord {
    scheduler_key: SchedulerKey,
    work: LifecycleWork,
}

impl LifecycleDeliveryRecord {
    pub(crate) const fn new(scheduler_key: SchedulerKey, work: LifecycleWork) -> Self {
        Self {
            scheduler_key,
            work,
        }
    }

    #[must_use]
    pub const fn scheduler_key(self) -> SchedulerKey {
        self.scheduler_key
    }

    #[must_use]
    pub const fn work(self) -> LifecycleWork {
        self.work
    }
}

/// One relocation-process wake consumed at its exact scheduler coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelocationProcessDeliveryRecord {
    scheduler_key: SchedulerKey,
    wake: RelocationProcessWake,
}

impl RelocationProcessDeliveryRecord {
    pub(crate) const fn new(scheduler_key: SchedulerKey, wake: RelocationProcessWake) -> Self {
        Self {
            scheduler_key,
            wake,
        }
    }

    #[must_use]
    pub const fn scheduler_key(self) -> SchedulerKey {
        self.scheduler_key
    }

    #[must_use]
    pub(crate) const fn wake(self) -> RelocationProcessWake {
        self.wake
    }
}

impl AttemptResolvedDeliveryRecord {
    pub(crate) const fn new(scheduler_key: SchedulerKey, resolved: AttemptResolved) -> Self {
        Self {
            scheduler_key,
            resolved,
        }
    }

    #[must_use]
    pub const fn scheduler_key(self) -> SchedulerKey {
        self.scheduler_key
    }

    #[must_use]
    pub const fn resolved(self) -> AttemptResolved {
        self.resolved
    }
}

macro_rules! delivery_reference {
    ($(#[$metadata:meta])* $name:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u32);

        impl $name {
            pub(crate) fn from_position(position: usize) -> Option<Self> {
                u32::try_from(position).ok().map(Self)
            }

            /// Returns the canonical zero-based collection coordinate.
            #[must_use]
            pub const fn index(self) -> u32 {
                self.0
            }
        }
    };
}

delivery_reference!(
    /// Checked reference to a command delivery in the enclosing moment record.
    CommandDeliveryRef
);
delivery_reference!(
    /// Checked reference to a post-commit delivery in the enclosing moment record.
    PostCommitDeliveryRef
);
delivery_reference!(
    /// Checked reference to an action-ready delivery in the enclosing moment record.
    ActionReadyDeliveryRef
);
delivery_reference!(
    /// Checked reference to an action-evaluation delivery in the enclosing moment record.
    ActionEvaluationDeliveryRef
);
delivery_reference!(
    /// Checked reference to a neutral attempt-resolution delivery in the enclosing moment record.
    AttemptResolvedDeliveryRef
);
delivery_reference!(
    /// Checked reference to a deterministic lifecycle delivery.
    LifecycleDeliveryRef
);
delivery_reference!(
    /// Checked reference to a relocation-process delivery in the enclosing moment record.
    RelocationProcessDeliveryRef
);

/// The authoritative disposition of one exact consumed delivery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeliveryResolutionRecord {
    NewCommand {
        delivery: CommandDeliveryRef,
        attempt: AttemptRecordId,
    },
    RetainedCommand {
        delivery: CommandDeliveryRef,
        original_attempt: AttemptRecordId,
        original_outcome: CommandAttemptOutcome,
    },
    CommandIdReuseMismatch {
        delivery: CommandDeliveryRef,
        original_attempt: AttemptRecordId,
    },
    NewCollision {
        delivery: CommandDeliveryRef,
        attempt: AttemptRecordId,
    },
    RetainedCollision {
        delivery: CommandDeliveryRef,
        original_attempt: AttemptRecordId,
    },
    RetiredCommand {
        delivery: CommandDeliveryRef,
    },
    PostCommitConsumed {
        delivery: PostCommitDeliveryRef,
    },
    ActionReadyConsumed {
        delivery: ActionReadyDeliveryRef,
    },
    ActionEvaluationConsumed {
        delivery: ActionEvaluationDeliveryRef,
    },
    AttemptResolvedConsumed {
        delivery: AttemptResolvedDeliveryRef,
    },
    LifecycleConsumed {
        delivery: LifecycleDeliveryRef,
    },
    RelocationProcessCompleted {
        delivery: RelocationProcessDeliveryRef,
    },
    ObsoleteRelocationWake {
        delivery: RelocationProcessDeliveryRef,
    },
}

/// One exact scheduler entry inserted by an authority transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulerInsertionRecord {
    scheduler_key: SchedulerKey,
    work: ScheduledWork,
}

impl SchedulerInsertionRecord {
    pub(crate) const fn new(scheduler_key: SchedulerKey, work: ScheduledWork) -> Self {
        Self {
            scheduler_key,
            work,
        }
    }

    #[must_use]
    pub const fn scheduler_key(&self) -> SchedulerKey {
        self.scheduler_key
    }

    #[must_use]
    pub const fn work(&self) -> &ScheduledWork {
        &self.work
    }
}

/// One exact scheduler entry removed by an authority transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulerRemovalRecord {
    scheduler_key: SchedulerKey,
    work: ScheduledWork,
}

impl SchedulerRemovalRecord {
    pub(crate) const fn new(scheduler_key: SchedulerKey, work: ScheduledWork) -> Self {
        Self {
            scheduler_key,
            work,
        }
    }

    #[must_use]
    pub const fn scheduler_key(&self) -> SchedulerKey {
        self.scheduler_key
    }

    #[must_use]
    pub const fn work(&self) -> &ScheduledWork {
        &self.work
    }
}

/// One checked one-shot transition of durable action-opportunity state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionOpportunityTransitionRecord {
    before: ActionOpportunity,
    after: ActionOpportunity,
}

/// Exact consumed delivery that opened one retained action evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionEvaluationInvocationOpeningCause {
    /// Initial evaluation created while consuming an action-ready wake.
    ActionReady(ActionReadyDeliveryRef),
    /// Linked successor created while consuming a captured-result wake.
    VisibleReinvocation(ActionEvaluationDeliveryRef),
}

/// One newly installed retained action-evaluation invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionEvaluationInvocationOpeningRecord {
    cause: ActionEvaluationInvocationOpeningCause,
    invocation: ActionEvaluationInvocationRecord,
}

impl ActionEvaluationInvocationOpeningRecord {
    pub(crate) const fn new(
        cause: ActionEvaluationInvocationOpeningCause,
        invocation: ActionEvaluationInvocationRecord,
    ) -> Self {
        Self { cause, invocation }
    }

    #[must_use]
    pub const fn cause(&self) -> ActionEvaluationInvocationOpeningCause {
        self.cause
    }

    #[must_use]
    pub const fn invocation(&self) -> &ActionEvaluationInvocationRecord {
        &self.invocation
    }
}

/// Closed authoritative cause of one action-evaluation invocation transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionEvaluationInvocationTransitionCause {
    /// A retained result or fallback wake was consumed by ordinary Fire.
    EvaluationDelivery(ActionEvaluationDeliveryRef),
    /// A typed result capture retained the invocation's result bytes.
    ResultCapture(ActionEvaluationCaptureId),
    /// An idempotent management request required the transition.
    Management(ManagementRequestId),
}

/// One exact retained action-evaluation invocation transition shared by
/// moment, capture, and management authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionEvaluationInvocationTransitionRecord {
    cause: ActionEvaluationInvocationTransitionCause,
    expected_before: ActionEvaluationInvocationDigest,
    after: ActionEvaluationInvocationRecord,
}

impl ActionEvaluationInvocationTransitionRecord {
    pub(crate) const fn new(
        cause: ActionEvaluationInvocationTransitionCause,
        expected_before: ActionEvaluationInvocationDigest,
        after: ActionEvaluationInvocationRecord,
    ) -> Self {
        Self {
            cause,
            expected_before,
            after,
        }
    }

    #[must_use]
    pub const fn cause(&self) -> ActionEvaluationInvocationTransitionCause {
        self.cause
    }

    #[must_use]
    pub const fn expected_before(&self) -> ActionEvaluationInvocationDigest {
        self.expected_before
    }

    #[must_use]
    pub const fn after(&self) -> &ActionEvaluationInvocationRecord {
        &self.after
    }
}

/// One newly opened durable action opportunity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionOpportunityOpeningRecord {
    opportunity: ActionOpportunity,
}

impl ActionOpportunityOpeningRecord {
    pub(crate) const fn new(opportunity: ActionOpportunity) -> Self {
        Self { opportunity }
    }

    #[must_use]
    pub const fn opportunity(&self) -> &ActionOpportunity {
        &self.opportunity
    }
}

/// Closed authoritative source of one actor-addressed evidence delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EvidenceRoutingSource {
    /// An actor observed one exact event in a consumed post-commit dispatch.
    PhysicalEvent {
        dispatch: PostCommitDeliveryRef,
        event_index: u32,
    },
    /// A newly evaluated actor-sponsored containment attempt was rejected at its believed source.
    RejectedContainmentAttempt { attempt: AttemptRecordRef },
}

/// One authoritative source routed into an exact evidence delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceRoutingRecord {
    source: EvidenceRoutingSource,
    evidence: EvidenceRecord,
}

impl EvidenceRoutingRecord {
    pub(crate) const fn physical_event(
        dispatch: PostCommitDeliveryRef,
        event_index: u32,
        evidence: EvidenceRecord,
    ) -> Self {
        Self {
            source: EvidenceRoutingSource::PhysicalEvent {
                dispatch,
                event_index,
            },
            evidence,
        }
    }

    pub(crate) const fn rejected_containment_attempt(
        attempt: AttemptRecordRef,
        evidence: EvidenceRecord,
    ) -> Self {
        Self {
            source: EvidenceRoutingSource::RejectedContainmentAttempt { attempt },
            evidence,
        }
    }

    #[must_use]
    pub const fn source(self) -> EvidenceRoutingSource {
        self.source
    }

    #[must_use]
    pub const fn evidence(self) -> EvidenceRecord {
        self.evidence
    }
}

/// One accepted actor-local evidence-assimilation transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceAssimilationRecord {
    actor: world_core::ActorId,
    expected_version: EpistemicVersion,
    evidence: Vec<EvidenceRecord>,
}

impl EvidenceAssimilationRecord {
    pub(crate) const fn new(
        actor: world_core::ActorId,
        expected_version: EpistemicVersion,
        evidence: Vec<EvidenceRecord>,
    ) -> Self {
        Self {
            actor,
            expected_version,
            evidence,
        }
    }

    #[must_use]
    pub const fn actor(&self) -> world_core::ActorId {
        self.actor
    }

    #[must_use]
    pub const fn expected_version(&self) -> EpistemicVersion {
        self.expected_version
    }

    #[must_use]
    pub fn evidence(&self) -> &[EvidenceRecord] {
        &self.evidence
    }
}

/// One closed mutation of the retained derived-appraisal ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainmentAppraisalTransitionRecord {
    Present {
        before: Option<ContainmentAppraisal>,
        after: ContainmentAppraisal,
    },
    Retracted {
        before: ContainmentAppraisal,
        supporting_evidence: EvidenceDeliveryId,
    },
}

/// One newly adopted accepted intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntentAdoptionRecord {
    intent: Intent,
}

impl IntentAdoptionRecord {
    pub(crate) const fn new(intent: Intent) -> Self {
        Self { intent }
    }

    #[must_use]
    pub const fn intent(self) -> Intent {
        self.intent
    }
}

/// One checked accepted intent transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntentTransitionRecord {
    before: Intent,
    after: Intent,
}

impl IntentTransitionRecord {
    pub(crate) const fn new(before: Intent, after: Intent) -> Self {
        Self { before, after }
    }

    #[must_use]
    pub const fn before(self) -> Intent {
        self.before
    }

    #[must_use]
    pub const fn after(self) -> Intent {
        self.after
    }
}

/// One newly started accepted activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivityStartRecord {
    activity: Activity,
}

impl ActivityStartRecord {
    pub(crate) const fn new(activity: Activity) -> Self {
        Self { activity }
    }

    #[must_use]
    pub const fn activity(self) -> Activity {
        self.activity
    }
}

/// One checked accepted activity transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivityTransitionRecord {
    before: Activity,
    after: Activity,
}

impl ActivityTransitionRecord {
    pub(crate) const fn new(before: Activity, after: Activity) -> Self {
        Self { before, after }
    }

    #[must_use]
    pub const fn before(self) -> Activity {
        self.before
    }

    #[must_use]
    pub const fn after(self) -> Activity {
        self.after
    }
}

/// One atomic terminal transition of an activity and its owning intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivityTerminalTransitionRecord {
    activity_before: Activity,
    activity_after: Activity,
    intent_before: Intent,
    intent_after: Intent,
}

impl ActivityTerminalTransitionRecord {
    pub(crate) const fn new(
        activity_before: Activity,
        activity_after: Activity,
        intent_before: Intent,
        intent_after: Intent,
    ) -> Self {
        Self {
            activity_before,
            activity_after,
            intent_before,
            intent_after,
        }
    }

    #[must_use]
    pub const fn activity_before(self) -> Activity {
        self.activity_before
    }

    #[must_use]
    pub const fn activity_after(self) -> Activity {
        self.activity_after
    }

    #[must_use]
    pub const fn intent_before(self) -> Intent {
        self.intent_before
    }

    #[must_use]
    pub const fn intent_after(self) -> Intent {
        self.intent_after
    }
}

/// Canonical causes and completion applied to one lifecycle-control key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleControlMutationRecord {
    actor: world_core::ActorId,
    role: LifecycleRole,
    requested: Vec<LifecycleCause>,
    completed: Option<LifecycleGeneration>,
}

impl LifecycleControlMutationRecord {
    pub(crate) const fn new(
        actor: world_core::ActorId,
        role: LifecycleRole,
        requested: Vec<LifecycleCause>,
        completed: Option<LifecycleGeneration>,
    ) -> Self {
        Self {
            actor,
            role,
            requested,
            completed,
        }
    }

    #[must_use]
    pub const fn actor(&self) -> world_core::ActorId {
        self.actor
    }

    #[must_use]
    pub const fn role(&self) -> LifecycleRole {
        self.role
    }

    #[must_use]
    pub fn requested(&self) -> &[LifecycleCause] {
        &self.requested
    }

    #[must_use]
    pub const fn completed(&self) -> Option<LifecycleGeneration> {
        self.completed
    }
}

/// Stable authoritative reason why one submitted relocation was not applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocationAttemptRejection {
    /// The submitted directed route is not accepted in the current domain.
    RouteUnavailable,
    /// The actor's accepted position does not admit the requested transition.
    PositionMismatch,
    /// No matching live relocation process can be controlled.
    ProcessUnavailable,
    /// The matching process exists but is in the wrong state at this moment.
    ProcessStateConflict,
    /// A checked time, version, or generation bound was exhausted.
    LimitReached,
}

/// Durable result of one submitted relocation interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocationAttemptResolution {
    /// Runtime accepted the interaction and produced this process transition.
    Accepted {
        /// Exact process affected by the accepted interaction.
        process: RelocationProcessId,
    },
    /// Runtime rejected the interaction without changing world or process state.
    Rejected(RelocationAttemptRejection),
}

/// Exact consumed delivery whose action resolution submitted an interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionResolutionDeliveryRef {
    /// Synchronous resolution of an action-ready delivery.
    Ready(ActionReadyDeliveryRef),
    /// Resolution of a retained deferred-evaluation result.
    Evaluation(ActionEvaluationDeliveryRef),
}

/// One authoritative relocation attempt caused by an action resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelocationAttemptRecord {
    resolution_delivery: ActionResolutionDeliveryRef,
    interaction: RelocationInteraction,
    resolution: RelocationAttemptResolution,
}

impl RelocationAttemptRecord {
    pub(crate) const fn new(
        resolution_delivery: ActionResolutionDeliveryRef,
        interaction: RelocationInteraction,
        resolution: RelocationAttemptResolution,
    ) -> Self {
        Self {
            resolution_delivery,
            interaction,
            resolution,
        }
    }

    /// Returns the exact delivery whose action resolution submitted the interaction.
    #[must_use]
    pub const fn resolution_delivery(self) -> ActionResolutionDeliveryRef {
        self.resolution_delivery
    }

    /// Returns the exact submitted relocation interaction.
    #[must_use]
    pub const fn interaction(self) -> RelocationInteraction {
        self.interaction
    }

    /// Returns the authoritative accepted or rejected result.
    #[must_use]
    pub const fn resolution(self) -> RelocationAttemptResolution {
        self.resolution
    }
}

/// Exact authoritative cause of one relocation-process transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocationProcessTransitionCause {
    /// A submitted grounded action caused start, pause, or resume.
    Action(ActionResolutionDeliveryRef),
    /// The exact current process wake caused completion.
    Wake(RelocationProcessDeliveryRef),
}

/// One checked mutation of the concrete relocation-process ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelocationProcessTransitionRecord {
    cause: RelocationProcessTransitionCause,
    before: Option<RelocationProcess>,
    after: RelocationProcess,
    event: Option<PhysicalEvent>,
}

impl RelocationProcessTransitionRecord {
    pub(crate) const fn new(
        cause: RelocationProcessTransitionCause,
        before: Option<RelocationProcess>,
        after: RelocationProcess,
        event: Option<PhysicalEvent>,
    ) -> Self {
        Self {
            cause,
            before,
            after,
            event,
        }
    }

    /// Returns the exact action or process-wake cause.
    #[must_use]
    pub const fn cause(self) -> RelocationProcessTransitionCause {
        self.cause
    }

    #[must_use]
    pub const fn before(self) -> Option<RelocationProcess> {
        self.before
    }

    #[must_use]
    pub const fn after(self) -> RelocationProcess {
        self.after
    }

    #[must_use]
    pub const fn event(self) -> Option<PhysicalEvent> {
        self.event
    }
}

impl ActionOpportunityTransitionRecord {
    pub(crate) const fn new(before: ActionOpportunity, after: ActionOpportunity) -> Self {
        Self { before, after }
    }

    #[must_use]
    pub const fn before(&self) -> &ActionOpportunity {
        &self.before
    }

    #[must_use]
    pub const fn after(&self) -> &ActionOpportunity {
        &self.after
    }
}

/// One atomic complete least-due-moment transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MomentBatchRecord {
    moment: SimMoment,
    resulting_frontier: SimMoment,
    consumed_keys: Vec<SchedulerKey>,
    command_deliveries: Vec<CommandDeliveryRecord>,
    post_commit_deliveries: Vec<PostCommitDeliveryRecord>,
    lifecycle_deliveries: Vec<LifecycleDeliveryRecord>,
    action_ready_deliveries: Vec<ActionReadyDeliveryRecord>,
    action_evaluation_deliveries: Vec<ActionEvaluationDeliveryRecord>,
    attempt_resolved_deliveries: Vec<AttemptResolvedDeliveryRecord>,
    relocation_process_deliveries: Vec<RelocationProcessDeliveryRecord>,
    action_opportunity_transitions: Vec<ActionOpportunityTransitionRecord>,
    action_evaluation_invocation_openings: Vec<ActionEvaluationInvocationOpeningRecord>,
    action_evaluation_invocation_transitions: Vec<ActionEvaluationInvocationTransitionRecord>,
    action_opportunity_openings: Vec<ActionOpportunityOpeningRecord>,
    evidence_routing: Vec<EvidenceRoutingRecord>,
    evidence_assimilations: Vec<EvidenceAssimilationRecord>,
    appraisal_transitions: Vec<ContainmentAppraisalTransitionRecord>,
    intent_adoptions: Vec<IntentAdoptionRecord>,
    intent_transitions: Vec<IntentTransitionRecord>,
    activity_starts: Vec<ActivityStartRecord>,
    activity_transitions: Vec<ActivityTransitionRecord>,
    activity_terminal_transitions: Vec<ActivityTerminalTransitionRecord>,
    lifecycle_control_mutations: Vec<LifecycleControlMutationRecord>,
    relocation_attempts: Vec<RelocationAttemptRecord>,
    relocation_process_transitions: Vec<RelocationProcessTransitionRecord>,
    attempts: Vec<AttemptRecord>,
    commits: Vec<ContainmentTransferCommitRecord>,
    containment_delta: Vec<ContainmentTransferDelta>,
    reactions: Vec<ReactionEnvelopeRecord>,
    scheduler_insertions: Vec<SchedulerInsertionRecord>,
    resolutions: Vec<DeliveryResolutionRecord>,
    resolution_evidence: ContainmentResolutionEvidence,
}

impl MomentBatchRecord {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
        moment: SimMoment,
        resulting_frontier: SimMoment,
        consumed_keys: Vec<SchedulerKey>,
        command_deliveries: Vec<CommandDeliveryRecord>,
        post_commit_deliveries: Vec<PostCommitDeliveryRecord>,
        lifecycle_deliveries: Vec<LifecycleDeliveryRecord>,
        action_ready_deliveries: Vec<ActionReadyDeliveryRecord>,
        action_evaluation_deliveries: Vec<ActionEvaluationDeliveryRecord>,
        attempt_resolved_deliveries: Vec<AttemptResolvedDeliveryRecord>,
        relocation_process_deliveries: Vec<RelocationProcessDeliveryRecord>,
        action_opportunity_transitions: Vec<ActionOpportunityTransitionRecord>,
        action_evaluation_invocation_openings: Vec<ActionEvaluationInvocationOpeningRecord>,
        action_evaluation_invocation_transitions: Vec<ActionEvaluationInvocationTransitionRecord>,
        action_opportunity_openings: Vec<ActionOpportunityOpeningRecord>,
        evidence_routing: Vec<EvidenceRoutingRecord>,
        evidence_assimilations: Vec<EvidenceAssimilationRecord>,
        appraisal_transitions: Vec<ContainmentAppraisalTransitionRecord>,
        intent_adoptions: Vec<IntentAdoptionRecord>,
        intent_transitions: Vec<IntentTransitionRecord>,
        activity_starts: Vec<ActivityStartRecord>,
        activity_transitions: Vec<ActivityTransitionRecord>,
        activity_terminal_transitions: Vec<ActivityTerminalTransitionRecord>,
        lifecycle_control_mutations: Vec<LifecycleControlMutationRecord>,
        relocation_attempts: Vec<RelocationAttemptRecord>,
        relocation_process_transitions: Vec<RelocationProcessTransitionRecord>,
        attempts: Vec<AttemptRecord>,
        commits: Vec<ContainmentTransferCommitRecord>,
        containment_delta: Vec<ContainmentTransferDelta>,
        reactions: Vec<ReactionEnvelopeRecord>,
        scheduler_insertions: Vec<SchedulerInsertionRecord>,
        resolutions: Vec<DeliveryResolutionRecord>,
        resolution_evidence: ContainmentResolutionEvidence,
    ) -> Self {
        Self {
            moment,
            resulting_frontier,
            consumed_keys,
            command_deliveries,
            post_commit_deliveries,
            lifecycle_deliveries,
            action_ready_deliveries,
            action_evaluation_deliveries,
            attempt_resolved_deliveries,
            relocation_process_deliveries,
            action_opportunity_transitions,
            action_evaluation_invocation_openings,
            action_evaluation_invocation_transitions,
            action_opportunity_openings,
            evidence_routing,
            evidence_assimilations,
            appraisal_transitions,
            intent_adoptions,
            intent_transitions,
            activity_starts,
            activity_transitions,
            activity_terminal_transitions,
            lifecycle_control_mutations,
            relocation_attempts,
            relocation_process_transitions,
            attempts,
            commits,
            containment_delta,
            reactions,
            scheduler_insertions,
            resolutions,
            resolution_evidence,
        }
    }

    #[must_use]
    pub const fn moment(&self) -> SimMoment {
        self.moment
    }

    #[must_use]
    pub const fn resulting_frontier(&self) -> SimMoment {
        self.resulting_frontier
    }

    #[must_use]
    pub fn consumed_keys(&self) -> &[SchedulerKey] {
        &self.consumed_keys
    }

    #[must_use]
    pub fn command_deliveries(&self) -> &[CommandDeliveryRecord] {
        &self.command_deliveries
    }

    #[must_use]
    pub fn post_commit_deliveries(&self) -> &[PostCommitDeliveryRecord] {
        &self.post_commit_deliveries
    }

    #[must_use]
    pub fn lifecycle_deliveries(&self) -> &[LifecycleDeliveryRecord] {
        &self.lifecycle_deliveries
    }

    #[must_use]
    pub fn action_ready_deliveries(&self) -> &[ActionReadyDeliveryRecord] {
        &self.action_ready_deliveries
    }

    #[must_use]
    pub fn action_evaluation_deliveries(&self) -> &[ActionEvaluationDeliveryRecord] {
        &self.action_evaluation_deliveries
    }

    #[must_use]
    pub fn attempt_resolved_deliveries(&self) -> &[AttemptResolvedDeliveryRecord] {
        &self.attempt_resolved_deliveries
    }

    #[must_use]
    pub fn relocation_process_deliveries(&self) -> &[RelocationProcessDeliveryRecord] {
        &self.relocation_process_deliveries
    }

    #[must_use]
    pub fn action_opportunity_transitions(&self) -> &[ActionOpportunityTransitionRecord] {
        &self.action_opportunity_transitions
    }

    #[must_use]
    pub fn action_evaluation_invocation_openings(
        &self,
    ) -> &[ActionEvaluationInvocationOpeningRecord] {
        &self.action_evaluation_invocation_openings
    }

    #[must_use]
    pub fn action_evaluation_invocation_transitions(
        &self,
    ) -> &[ActionEvaluationInvocationTransitionRecord] {
        &self.action_evaluation_invocation_transitions
    }

    #[must_use]
    pub fn action_opportunity_openings(&self) -> &[ActionOpportunityOpeningRecord] {
        &self.action_opportunity_openings
    }

    #[must_use]
    pub fn evidence_routing(&self) -> &[EvidenceRoutingRecord] {
        &self.evidence_routing
    }

    #[must_use]
    pub fn evidence_assimilations(&self) -> &[EvidenceAssimilationRecord] {
        &self.evidence_assimilations
    }

    #[must_use]
    pub fn appraisal_transitions(&self) -> &[ContainmentAppraisalTransitionRecord] {
        &self.appraisal_transitions
    }

    #[must_use]
    pub fn intent_adoptions(&self) -> &[IntentAdoptionRecord] {
        &self.intent_adoptions
    }

    #[must_use]
    pub fn intent_transitions(&self) -> &[IntentTransitionRecord] {
        &self.intent_transitions
    }

    #[must_use]
    pub fn activity_starts(&self) -> &[ActivityStartRecord] {
        &self.activity_starts
    }

    #[must_use]
    pub fn activity_transitions(&self) -> &[ActivityTransitionRecord] {
        &self.activity_transitions
    }

    #[must_use]
    pub fn activity_terminal_transitions(&self) -> &[ActivityTerminalTransitionRecord] {
        &self.activity_terminal_transitions
    }

    #[must_use]
    pub fn lifecycle_control_mutations(&self) -> &[LifecycleControlMutationRecord] {
        &self.lifecycle_control_mutations
    }

    /// Returns submitted relocation attempts in action-delivery order.
    #[must_use]
    pub fn relocation_attempts(&self) -> &[RelocationAttemptRecord] {
        &self.relocation_attempts
    }

    #[must_use]
    pub fn relocation_process_transitions(&self) -> &[RelocationProcessTransitionRecord] {
        &self.relocation_process_transitions
    }

    #[must_use]
    pub fn attempts(&self) -> &[AttemptRecord] {
        &self.attempts
    }

    #[cfg(test)]
    #[must_use]
    pub fn commits(&self) -> &[ContainmentTransferCommitRecord] {
        &self.commits
    }

    #[must_use]
    pub fn containment_delta(&self) -> &[ContainmentTransferDelta] {
        &self.containment_delta
    }

    #[cfg(test)]
    #[must_use]
    pub fn reactions(&self) -> &[ReactionEnvelopeRecord] {
        &self.reactions
    }

    #[must_use]
    pub fn scheduler_insertions(&self) -> &[SchedulerInsertionRecord] {
        &self.scheduler_insertions
    }

    #[must_use]
    pub fn resolutions(&self) -> &[DeliveryResolutionRecord] {
        &self.resolutions
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn resolution_evidence(&self) -> &ContainmentResolutionEvidence {
        &self.resolution_evidence
    }
}

/// One atomic management transition of a retained action evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionEvaluationManagementRecord {
    transition: ActionEvaluationInvocationTransitionRecord,
    scheduler_removal: Option<SchedulerRemovalRecord>,
    fallback_insertion: SchedulerInsertionRecord,
}

impl ActionEvaluationManagementRecord {
    pub(crate) const fn new(
        transition: ActionEvaluationInvocationTransitionRecord,
        scheduler_removal: Option<SchedulerRemovalRecord>,
        fallback_insertion: SchedulerInsertionRecord,
    ) -> Self {
        Self {
            transition,
            scheduler_removal,
            fallback_insertion,
        }
    }

    #[must_use]
    pub const fn transition(&self) -> &ActionEvaluationInvocationTransitionRecord {
        &self.transition
    }

    #[must_use]
    pub const fn scheduler_removal(&self) -> Option<&SchedulerRemovalRecord> {
        self.scheduler_removal.as_ref()
    }

    #[must_use]
    pub const fn fallback_insertion(&self) -> &SchedulerInsertionRecord {
        &self.fallback_insertion
    }
}

/// One idempotently retained management transition and its optional
/// action-evaluation effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagementRecord {
    request: ManagementRequestId,
    fingerprint: ManagementRequestFingerprint,
    operation: SessionManagement,
    outcome: ManageOutcome,
    action_evaluation: Option<ActionEvaluationManagementRecord>,
}

impl ManagementRecord {
    pub(crate) const fn new(
        request: ManageRequest,
        outcome: ManageOutcome,
        action_evaluation: Option<ActionEvaluationManagementRecord>,
    ) -> Self {
        Self {
            request: request.id(),
            fingerprint: request.fingerprint(),
            operation: request.operation(),
            outcome,
            action_evaluation,
        }
    }
    #[must_use]
    pub const fn request(&self) -> ManagementRequestId {
        self.request
    }
    #[must_use]
    pub const fn fingerprint(&self) -> ManagementRequestFingerprint {
        self.fingerprint
    }
    #[must_use]
    pub const fn operation(&self) -> SessionManagement {
        self.operation
    }
    #[must_use]
    pub const fn outcome(&self) -> ManageOutcome {
        self.outcome
    }

    #[must_use]
    pub const fn action_evaluation(&self) -> Option<&ActionEvaluationManagementRecord> {
        self.action_evaluation.as_ref()
    }
}

/// Canonical cause family of one management publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManagementCauseRecord {
    /// One or more authenticated, idempotent host requests.
    HostRequests(Vec<ManagementRecord>),
    /// One deterministic kernel preflight cause.
    KernelSafety(KernelSafetyCause),
}

/// One canonical management transition and its preserved scheduler frontier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagementBatchRecord {
    cause: ManagementCauseRecord,
    resulting_mode: SessionMode,
    preserved_frontier: SimMoment,
}

impl ManagementBatchRecord {
    pub(crate) const fn host_requests(
        entries: Vec<ManagementRecord>,
        resulting_mode: SessionMode,
        preserved_frontier: SimMoment,
    ) -> Self {
        Self {
            cause: ManagementCauseRecord::HostRequests(entries),
            resulting_mode,
            preserved_frontier,
        }
    }

    pub(crate) const fn kernel_safety(
        cause: KernelSafetyCause,
        resulting_mode: SessionMode,
        preserved_frontier: SimMoment,
    ) -> Self {
        Self {
            cause: ManagementCauseRecord::KernelSafety(cause),
            resulting_mode,
            preserved_frontier,
        }
    }

    /// Returns the closed cause family captured by this transition.
    #[must_use]
    pub const fn cause(&self) -> &ManagementCauseRecord {
        &self.cause
    }

    #[must_use]
    pub fn entries(&self) -> &[ManagementRecord] {
        match &self.cause {
            ManagementCauseRecord::HostRequests(entries) => entries,
            ManagementCauseRecord::KernelSafety(_) => &[],
        }
    }

    /// Returns the deterministic kernel cause, if this is a safety transition.
    #[must_use]
    pub const fn kernel_safety_cause(&self) -> Option<KernelSafetyCause> {
        match &self.cause {
            ManagementCauseRecord::HostRequests(_) => None,
            ManagementCauseRecord::KernelSafety(cause) => Some(*cause),
        }
    }

    #[must_use]
    pub const fn resulting_mode(&self) -> SessionMode {
        self.resulting_mode
    }

    /// Returns the unresolved-work frontier retained by management.
    #[must_use]
    pub const fn preserved_frontier(&self) -> SimMoment {
        self.preserved_frontier
    }
}

/// Private input to the record sealer.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DraftAuthorityRecord {
    expected_cursor: AuthorityCursor,
    body: DraftAuthorityRecordBody,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DraftAuthorityAdmission {
    Commands(Vec<AdmitRequest>),
    ActionEvaluation(Box<ActionEvaluationCaptureRequest>),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DraftAuthorityRecordBody {
    Admission(DraftAuthorityAdmission),
    Moment { batch: DraftMomentBatch },
    Management { requests: Vec<ManageRequest> },
    KernelSafety { cause: KernelSafetyCause },
}

impl DraftAuthorityRecord {
    pub(crate) const fn admit_commands(
        expected_cursor: AuthorityCursor,
        requests: Vec<AdmitRequest>,
    ) -> Self {
        Self {
            expected_cursor,
            body: DraftAuthorityRecordBody::Admission(DraftAuthorityAdmission::Commands(requests)),
        }
    }

    pub(crate) fn admit_action_evaluation(
        expected_cursor: AuthorityCursor,
        request: ActionEvaluationCaptureRequest,
    ) -> Self {
        Self {
            expected_cursor,
            body: DraftAuthorityRecordBody::Admission(DraftAuthorityAdmission::ActionEvaluation(
                Box::new(request),
            )),
        }
    }

    pub(crate) const fn moment(expected_cursor: AuthorityCursor, batch: DraftMomentBatch) -> Self {
        Self {
            expected_cursor,
            body: DraftAuthorityRecordBody::Moment { batch },
        }
    }

    pub(crate) const fn management(
        expected_cursor: AuthorityCursor,
        requests: Vec<ManageRequest>,
    ) -> Self {
        Self {
            expected_cursor,
            body: DraftAuthorityRecordBody::Management { requests },
        }
    }

    pub(crate) const fn kernel_safety(
        expected_cursor: AuthorityCursor,
        cause: KernelSafetyCause,
    ) -> Self {
        Self {
            expected_cursor,
            body: DraftAuthorityRecordBody::KernelSafety { cause },
        }
    }

    pub(crate) const fn expected_cursor(&self) -> AuthorityCursor {
        self.expected_cursor
    }

    pub(crate) fn into_body(self) -> DraftAuthorityRecordBody {
        self.body
    }
}

/// Why a complete prepared moment and resolver result could not form an
/// authority draft graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DraftMomentBuildError {
    Proposal(ProposalBuildError),
    MissingDeliveryWork {
        delivery: usize,
    },
    UnexpectedDeliveryWork {
        delivery: usize,
    },
    ProposalKindMismatch {
        delivery: usize,
    },
    ActionOpportunityMismatch {
        opportunity: ActionOpportunityId,
    },
    DuplicateAttempt {
        identity: ContainmentCommandIdentity,
    },
    AttemptCoverageMismatch,
    AttemptActorMismatch {
        identity: ContainmentCommandIdentity,
    },
    AttemptProposalMismatch {
        identity: ContainmentCommandIdentity,
    },
    InvalidCollisionEvidence {
        identity: ContainmentCommandIdentity,
    },
    AcceptedDeltaMismatch,
    SuccessorMismatch,
    InvalidAcceptedDelta(ContainmentTransitionError),
}

impl From<ProposalBuildError> for DraftMomentBuildError {
    fn from(error: ProposalBuildError) -> Self {
        Self::Proposal(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DraftAttemptRef(ContainmentCommandIdentity);

impl DraftAttemptRef {
    const fn new(identity: ContainmentCommandIdentity) -> Self {
        Self(identity)
    }

    pub(crate) const fn identity(self) -> ContainmentCommandIdentity {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DraftAttemptOutcome {
    Accepted(ContainmentTransferDelta),
    Rejected(StableCommandRejection),
    CommandIdCollision,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DraftAttemptSubject {
    EvaluatedCommand(CommandEnvelope),
    CommandIdCollision {
        source: CommandSource,
        command: CommandId,
        fingerprints: Vec<CommandRequestFingerprint>,
    },
}

impl DraftAttemptSubject {
    pub(crate) fn identity(&self) -> ContainmentCommandIdentity {
        match self {
            Self::EvaluatedCommand(command) => ContainmentCommandIdentity::from_command(command),
            Self::CommandIdCollision {
                source, command, ..
            } => ContainmentCommandIdentity::new(*source, *command),
        }
    }

    pub(crate) const fn command(&self) -> Option<&CommandEnvelope> {
        match self {
            Self::EvaluatedCommand(command) => Some(command),
            Self::CommandIdCollision { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DraftAttemptRecord {
    subject: DraftAttemptSubject,
    outcome: DraftAttemptOutcome,
}

impl DraftAttemptRecord {
    pub(crate) fn identity(&self) -> ContainmentCommandIdentity {
        self.subject.identity()
    }

    pub(crate) const fn subject(&self) -> &DraftAttemptSubject {
        &self.subject
    }

    pub(crate) const fn command(&self) -> Option<&CommandEnvelope> {
        self.subject.command()
    }

    pub(crate) const fn outcome(&self) -> DraftAttemptOutcome {
        self.outcome
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DraftDeliveryResolution {
    NewCommand {
        attempt: DraftAttemptRef,
    },
    RetainedCommand {
        original_attempt: AttemptRecordId,
        original_outcome: CommandAttemptOutcome,
    },
    CommandIdReuseMismatch {
        original_attempt: AttemptRecordId,
    },
    NewCollision {
        attempt: DraftAttemptRef,
    },
    RetainedCollision {
        original_attempt: AttemptRecordId,
    },
    RetiredCommand,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DraftMomentDelivery {
    Command {
        key: SchedulerKey,
        scheduled: Box<ScheduledCommand>,
        resolution: DraftDeliveryResolution,
    },
    PostCommit {
        key: SchedulerKey,
        dispatch: PostCommitDispatch,
        observations: Vec<EvidenceObservation>,
    },
    EvidenceDelivery {
        key: SchedulerKey,
        delivery: EvidenceDeliveryWork,
        assimilation: Option<(world_core::ActorId, EpistemicVersion, Box<EpistemicState>)>,
    },
    Appraisal {
        key: SchedulerKey,
        work: crate::lifecycle::AppraisalWork,
        results: Vec<AppraisalResult>,
    },
    IntentReview {
        key: SchedulerKey,
        work: crate::lifecycle::IntentReviewWork,
        result: IntentReviewResult,
    },
    ActivityInitialization {
        key: SchedulerKey,
        work: crate::lifecycle::ActivityInitializationWork,
        result: ActivityInitializationResult,
    },
    ActionReady {
        key: SchedulerKey,
        ready: ActionReady,
        opportunity: Box<ActionOpportunity>,
        proposal: ActionProposal,
    },
    ActionEvaluation {
        key: SchedulerKey,
        evaluation: ActionEvaluationWork,
        opportunity: Box<ActionOpportunity>,
        invocation: Box<ActionEvaluationInvocationRecord>,
        proposal: Option<ActionEvaluationDecision>,
    },
    AttemptResolved {
        key: SchedulerKey,
        resolved: AttemptResolved,
    },
    ActivityAdvance {
        key: SchedulerKey,
        work: crate::lifecycle::ActivityAdvanceWork,
        result: ActivityAdvanceResult,
    },
    RelocationProcess {
        key: SchedulerKey,
        wake: RelocationProcessWake,
        classification: RelocationWakeClassification,
    },
}

impl DraftMomentDelivery {
    pub(crate) const fn key(&self) -> SchedulerKey {
        match self {
            Self::Command { key, .. }
            | Self::PostCommit { key, .. }
            | Self::EvidenceDelivery { key, .. }
            | Self::Appraisal { key, .. }
            | Self::IntentReview { key, .. }
            | Self::ActivityInitialization { key, .. }
            | Self::ActionReady { key, .. }
            | Self::ActionEvaluation { key, .. }
            | Self::AttemptResolved { key, .. }
            | Self::ActivityAdvance { key, .. }
            | Self::RelocationProcess { key, .. } => *key,
        }
    }

    pub(crate) fn scheduled_work(&self) -> crate::scheduler::ScheduledWork {
        match self {
            Self::Command { scheduled, .. } => {
                crate::scheduler::ScheduledWork::Command(scheduled.clone())
            }
            Self::PostCommit { dispatch, .. } => {
                crate::scheduler::ScheduledWork::PostCommit(dispatch.clone())
            }
            Self::EvidenceDelivery { delivery, .. } => crate::scheduler::ScheduledWork::lifecycle(
                crate::lifecycle::LifecycleWork::EvidenceDelivery(*delivery),
            ),
            Self::Appraisal { work, .. } => crate::scheduler::ScheduledWork::lifecycle(
                crate::lifecycle::LifecycleWork::Appraisal(*work),
            ),
            Self::IntentReview { work, .. } => crate::scheduler::ScheduledWork::lifecycle(
                crate::lifecycle::LifecycleWork::IntentReview(*work),
            ),
            Self::ActivityInitialization { work, .. } => {
                crate::scheduler::ScheduledWork::lifecycle(
                    crate::lifecycle::LifecycleWork::ActivityInitialization(*work),
                )
            }
            Self::ActionReady { ready, .. } => crate::scheduler::ScheduledWork::ActionReady(*ready),
            Self::ActionEvaluation { evaluation, .. } => {
                crate::scheduler::ScheduledWork::action_evaluation(*evaluation)
            }
            Self::AttemptResolved { resolved, .. } => {
                crate::scheduler::ScheduledWork::attempt_resolved(*resolved)
            }
            Self::RelocationProcess { wake, .. } => crate::scheduler::ScheduledWork::process(*wake),
            Self::ActivityAdvance { work, .. } => crate::scheduler::ScheduledWork::lifecycle(
                crate::lifecycle::LifecycleWork::ActivityAdvance(*work),
            ),
        }
    }
}

/// Complete checked authority-draft graph for one prepared least-due moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DraftMomentBatch {
    deliveries: Vec<DraftMomentDelivery>,
    attempts: Vec<DraftAttemptRecord>,
    resolution_evidence: ContainmentResolutionEvidence,
}

impl DraftMomentBatch {
    pub(crate) fn from_prepared(
        prepared: &PreparedFire,
        proposals: &MomentWorkProposals,
        resolution: &ContainmentMomentResolution,
    ) -> Result<Self, DraftMomentBuildError> {
        prepared.validate_proposals(proposals)?;

        let mut proposal_by_attempt = BTreeMap::new();
        let mut representative_by_attempt = BTreeMap::new();
        let mut collision_fingerprints =
            BTreeMap::<ContainmentCommandIdentity, BTreeSet<CommandRequestFingerprint>>::new();
        let mut deliveries = Vec::with_capacity(prepared.deliveries().len());
        let mut represented_work = BTreeSet::new();

        for (position, delivery) in prepared.deliveries().iter().enumerate() {
            match delivery {
                PreparedDelivery::EvaluableCommand { key, scheduled } => {
                    let work = prepared
                        .work_id_for_delivery(position)
                        .ok_or(DraftMomentBuildError::MissingDeliveryWork { delivery: position })?;
                    let Some(WorkProposal::Command(proposal)) = proposals.proposal(work) else {
                        return Err(DraftMomentBuildError::ProposalKindMismatch {
                            delivery: position,
                        });
                    };
                    let identity = ContainmentCommandIdentity::from_command(scheduled.command());
                    if let Some(previous) = proposal_by_attempt.insert(identity, proposal)
                        && previous != proposal
                    {
                        return Err(DraftMomentBuildError::AttemptProposalMismatch { identity });
                    }
                    representative_by_attempt
                        .entry(identity)
                        .or_insert_with(|| scheduled.command().clone());
                    deliveries.push(DraftMomentDelivery::Command {
                        key: *key,
                        scheduled: Box::new(scheduled.clone()),
                        resolution: DraftDeliveryResolution::NewCommand {
                            attempt: DraftAttemptRef::new(identity),
                        },
                    });
                }
                PreparedDelivery::ResolvedCommand {
                    key,
                    scheduled,
                    resolution,
                } => {
                    if prepared.work_id_for_delivery(position).is_some() {
                        return Err(DraftMomentBuildError::UnexpectedDeliveryWork {
                            delivery: position,
                        });
                    }
                    let resolution = match *resolution {
                        PreparedCommandResolution::Retained {
                            original_attempt,
                            outcome,
                        } => DraftDeliveryResolution::RetainedCommand {
                            original_attempt,
                            original_outcome: outcome,
                        },
                        PreparedCommandResolution::IdReuseMismatch { original_attempt } => {
                            DraftDeliveryResolution::CommandIdReuseMismatch { original_attempt }
                        }
                        PreparedCommandResolution::NewCollision => {
                            let identity =
                                ContainmentCommandIdentity::from_command(scheduled.command());
                            collision_fingerprints
                                .entry(identity)
                                .or_default()
                                .insert(scheduled.command().fingerprint());
                            DraftDeliveryResolution::NewCollision {
                                attempt: DraftAttemptRef::new(identity),
                            }
                        }
                        PreparedCommandResolution::RetainedCollision { original_attempt } => {
                            DraftDeliveryResolution::RetainedCollision { original_attempt }
                        }
                        PreparedCommandResolution::Retired => {
                            DraftDeliveryResolution::RetiredCommand
                        }
                    };
                    deliveries.push(DraftMomentDelivery::Command {
                        key: *key,
                        scheduled: Box::new(scheduled.clone()),
                        resolution,
                    });
                }
                PreparedDelivery::PostCommit { key, dispatch } => {
                    let work = prepared
                        .work_id_for_delivery(position)
                        .ok_or(DraftMomentBuildError::MissingDeliveryWork { delivery: position })?;
                    let Some(WorkProposal::PostCommit(observations)) = proposals.proposal(work)
                    else {
                        return Err(DraftMomentBuildError::ProposalKindMismatch {
                            delivery: position,
                        });
                    };
                    deliveries.push(DraftMomentDelivery::PostCommit {
                        key: *key,
                        dispatch: dispatch.clone(),
                        observations: observations.clone(),
                    });
                }
                PreparedDelivery::EvidenceDelivery { key, delivery } => {
                    let work = prepared
                        .work_id_for_delivery(position)
                        .ok_or(DraftMomentBuildError::MissingDeliveryWork { delivery: position })?;
                    let assimilation = if represented_work.insert(work) {
                        let Some(WorkProposal::EvidenceAssimilation {
                            actor,
                            expected_version,
                            successor,
                        }) = proposals.proposal(work)
                        else {
                            return Err(DraftMomentBuildError::ProposalKindMismatch {
                                delivery: position,
                            });
                        };
                        Some((*actor, *expected_version, successor.clone()))
                    } else {
                        None
                    };
                    deliveries.push(DraftMomentDelivery::EvidenceDelivery {
                        key: *key,
                        delivery: *delivery,
                        assimilation,
                    });
                }
                PreparedDelivery::Appraisal { key, appraisal, .. } => {
                    let work = prepared
                        .work_id_for_delivery(position)
                        .ok_or(DraftMomentBuildError::MissingDeliveryWork { delivery: position })?;
                    let Some(WorkProposal::Appraisal(results)) = proposals.proposal(work) else {
                        return Err(DraftMomentBuildError::ProposalKindMismatch {
                            delivery: position,
                        });
                    };
                    deliveries.push(DraftMomentDelivery::Appraisal {
                        key: *key,
                        work: *appraisal,
                        results: results.clone(),
                    });
                }
                PreparedDelivery::IntentReview { key, review, .. } => {
                    let work = prepared
                        .work_id_for_delivery(position)
                        .ok_or(DraftMomentBuildError::MissingDeliveryWork { delivery: position })?;
                    let Some(WorkProposal::IntentReview(result)) = proposals.proposal(work) else {
                        return Err(DraftMomentBuildError::ProposalKindMismatch {
                            delivery: position,
                        });
                    };
                    deliveries.push(DraftMomentDelivery::IntentReview {
                        key: *key,
                        work: *review,
                        result: *result,
                    });
                }
                PreparedDelivery::ActivityInitialization {
                    key,
                    initialization,
                    ..
                } => {
                    let work = prepared
                        .work_id_for_delivery(position)
                        .ok_or(DraftMomentBuildError::MissingDeliveryWork { delivery: position })?;
                    let Some(WorkProposal::ActivityInitialization(result)) =
                        proposals.proposal(work)
                    else {
                        return Err(DraftMomentBuildError::ProposalKindMismatch {
                            delivery: position,
                        });
                    };
                    deliveries.push(DraftMomentDelivery::ActivityInitialization {
                        key: *key,
                        work: *initialization,
                        result: result.clone(),
                    });
                }
                PreparedDelivery::ActionReady {
                    key,
                    ready,
                    opportunity,
                } => {
                    let work = prepared
                        .work_id_for_delivery(position)
                        .ok_or(DraftMomentBuildError::MissingDeliveryWork { delivery: position })?;
                    let Some(WorkProposal::Action(proposal)) = proposals.proposal(work) else {
                        return Err(DraftMomentBuildError::ProposalKindMismatch {
                            delivery: position,
                        });
                    };
                    if proposal.expected_version() != ready.expected_version()
                        || opportunity.id() != ready.opportunity()
                        || opportunity.version() != ready.expected_version()
                    {
                        return Err(DraftMomentBuildError::ActionOpportunityMismatch {
                            opportunity: ready.opportunity(),
                        });
                    }
                    deliveries.push(DraftMomentDelivery::ActionReady {
                        key: *key,
                        ready: *ready,
                        opportunity: Box::new(opportunity.clone()),
                        proposal: proposal.clone(),
                    });
                }
                PreparedDelivery::ActionEvaluation {
                    key,
                    evaluation,
                    opportunity,
                    invocation,
                } => {
                    let proposal = match evaluation {
                        ActionEvaluationWork::ResultReady { .. } => {
                            let work = prepared.work_id_for_delivery(position).ok_or(
                                DraftMomentBuildError::MissingDeliveryWork { delivery: position },
                            )?;
                            let Some(WorkProposal::ActionEvaluation(proposal)) =
                                proposals.proposal(work)
                            else {
                                return Err(DraftMomentBuildError::ProposalKindMismatch {
                                    delivery: position,
                                });
                            };
                            Some(proposal.clone())
                        }
                        ActionEvaluationWork::Fallback { .. } => {
                            if prepared.work_id_for_delivery(position).is_some() {
                                return Err(DraftMomentBuildError::UnexpectedDeliveryWork {
                                    delivery: position,
                                });
                            }
                            None
                        }
                    };
                    deliveries.push(DraftMomentDelivery::ActionEvaluation {
                        key: *key,
                        evaluation: *evaluation,
                        opportunity: Box::new(opportunity.clone()),
                        invocation: invocation.clone(),
                        proposal,
                    });
                }
                PreparedDelivery::AttemptResolved { key, resolved, .. } => {
                    let work = prepared
                        .work_id_for_delivery(position)
                        .ok_or(DraftMomentBuildError::MissingDeliveryWork { delivery: position })?;
                    if proposals.proposal(work) != Some(&WorkProposal::AttemptResolvedConsumed) {
                        return Err(DraftMomentBuildError::ProposalKindMismatch {
                            delivery: position,
                        });
                    }
                    deliveries.push(DraftMomentDelivery::AttemptResolved {
                        key: *key,
                        resolved: *resolved,
                    });
                }
                PreparedDelivery::ActivityAdvance { key, advance, .. } => {
                    let work = prepared
                        .work_id_for_delivery(position)
                        .ok_or(DraftMomentBuildError::MissingDeliveryWork { delivery: position })?;
                    let Some(WorkProposal::ActivityAdvance(result)) = proposals.proposal(work)
                    else {
                        return Err(DraftMomentBuildError::ProposalKindMismatch {
                            delivery: position,
                        });
                    };
                    deliveries.push(DraftMomentDelivery::ActivityAdvance {
                        key: *key,
                        work: *advance,
                        result: result.clone(),
                    });
                }
                PreparedDelivery::Process {
                    key,
                    wake,
                    classification,
                } => {
                    match classification {
                        RelocationWakeClassification::Current(_) => {
                            let work = prepared.work_id_for_delivery(position).ok_or(
                                DraftMomentBuildError::MissingDeliveryWork { delivery: position },
                            )?;
                            if proposals.proposal(work)
                                != Some(&WorkProposal::RelocationProcessCompleted)
                            {
                                return Err(DraftMomentBuildError::ProposalKindMismatch {
                                    delivery: position,
                                });
                            }
                        }
                        RelocationWakeClassification::Obsolete => {
                            if prepared.work_id_for_delivery(position).is_some() {
                                return Err(DraftMomentBuildError::UnexpectedDeliveryWork {
                                    delivery: position,
                                });
                            }
                        }
                    }
                    deliveries.push(DraftMomentDelivery::RelocationProcess {
                        key: *key,
                        wake: *wake,
                        classification: *classification,
                    });
                }
            }
        }

        let mut attempts = Vec::with_capacity(resolution.outcomes().len());
        let mut seen_attempts = BTreeSet::new();
        let mut accepted_deltas = Vec::new();
        for resolved in resolution.outcomes() {
            let identity = resolved.identity();
            if !seen_attempts.insert(identity) {
                return Err(DraftMomentBuildError::DuplicateAttempt { identity });
            }
            let command = representative_by_attempt
                .get(&identity)
                .ok_or(DraftMomentBuildError::AttemptCoverageMismatch)?;
            if command.actor() != resolved.actor() {
                return Err(DraftMomentBuildError::AttemptActorMismatch { identity });
            }
            let proposal = proposal_by_attempt
                .get(&identity)
                .ok_or(DraftMomentBuildError::AttemptCoverageMismatch)?;
            let outcome = match (proposal, resolved.outcome()) {
                (
                    CommandProposal::Rejected(proposed),
                    ContainmentCandidateOutcome::Rejected(actual),
                ) if proposed == actual && resolved.footprint().is_none() => {
                    DraftAttemptOutcome::Rejected(*actual)
                }
                (
                    CommandProposal::AcceptedTransfer(proposed),
                    ContainmentCandidateOutcome::Accepted { delta },
                ) if proposed == delta => {
                    accepted_deltas.push(*delta);
                    DraftAttemptOutcome::Accepted(*delta)
                }
                (
                    CommandProposal::AcceptedTransfer(_),
                    ContainmentCandidateOutcome::Rejected(reason),
                ) => DraftAttemptOutcome::Rejected(*reason),
                _ => {
                    return Err(DraftMomentBuildError::AttemptProposalMismatch { identity });
                }
            };
            attempts.push(DraftAttemptRecord {
                subject: DraftAttemptSubject::EvaluatedCommand(command.clone()),
                outcome,
            });
        }

        if seen_attempts.len() != representative_by_attempt.len()
            || seen_attempts.len() != proposal_by_attempt.len()
        {
            return Err(DraftMomentBuildError::AttemptCoverageMismatch);
        }
        for (identity, fingerprints) in collision_fingerprints {
            if !seen_attempts.insert(identity) || fingerprints.len() < 2 {
                return Err(DraftMomentBuildError::InvalidCollisionEvidence { identity });
            }
            attempts.push(DraftAttemptRecord {
                subject: DraftAttemptSubject::CommandIdCollision {
                    source: identity.source(),
                    command: identity.command(),
                    fingerprints: fingerprints.into_iter().collect(),
                },
                outcome: DraftAttemptOutcome::CommandIdCollision,
            });
        }
        if accepted_deltas != resolution.accepted_deltas() {
            return Err(DraftMomentBuildError::AcceptedDeltaMismatch);
        }
        let successor =
            apply_containment_transfers(prepared.base_snapshot().accepted(), &accepted_deltas)
                .map_err(DraftMomentBuildError::InvalidAcceptedDelta)?;
        if &successor != resolution.successor() {
            return Err(DraftMomentBuildError::SuccessorMismatch);
        }

        deliveries.sort_by_key(DraftMomentDelivery::key);
        attempts.sort_by_key(DraftAttemptRecord::identity);
        Ok(Self {
            deliveries,
            attempts,
            resolution_evidence: resolution.evidence().clone(),
        })
    }

    pub(crate) fn deliveries(&self) -> &[DraftMomentDelivery] {
        &self.deliveries
    }

    pub(crate) fn attempts(&self) -> &[DraftAttemptRecord] {
        &self.attempts
    }

    pub(crate) const fn resolution_evidence(&self) -> &ContainmentResolutionEvidence {
        &self.resolution_evidence
    }
}

macro_rules! moment_local_reference {
    (pub $name:ident, $index:ident, $same_record:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u32);

        impl $name {
            pub(crate) fn from_position(position: usize) -> Option<Self> {
                u32::try_from(position).ok().map(Self)
            }

            /// Returns the canonical zero-based attempt coordinate.
            #[must_use]
            pub const fn index(self) -> u32 {
                self.0
            }

            pub(crate) const fn position(self) -> u32 {
                self.0
            }

            pub(crate) fn write_canonical(self, writer: &mut CanonicalWriter) {
                super::$same_record::new(super::$index::new(self.0)).write_canonical(writer);
            }
        }
    };
    ($name:ident, $index:ident, $same_record:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub(crate) struct $name(u32);

        impl $name {
            pub(crate) fn from_position(position: usize) -> Option<Self> {
                u32::try_from(position).ok().map(Self)
            }

            pub(crate) const fn position(self) -> u32 {
                self.0
            }

            pub(crate) fn write_canonical(self, writer: &mut CanonicalWriter) {
                super::$same_record::new(super::$index::new(self.0)).write_canonical(writer);
            }
        }
    };
}

moment_local_reference!(pub AttemptRecordRef, AttemptLocalIndex, SameRecordAttemptRef);
moment_local_reference!(MomentCommitRef, CommitLocalIndex, SameRecordCommitRef);
moment_local_reference!(MomentReactionRef, ReactionLocalIndex, SameRecordReactionRef);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NormalizedIngressRecord {
    pub(crate) prepared: PreparedScheduledCommand,
    pub(crate) scheduler_key: SchedulerKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NormalizedActionEvaluationAdmission {
    pub(crate) request: ActionEvaluationCaptureRequest,
    pub(crate) transition: ActionEvaluationInvocationTransitionRecord,
    pub(crate) scheduler_key: SchedulerKey,
    pub(crate) work: ScheduledWork,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NormalizedAuthorityAdmission {
    Commands(Vec<NormalizedIngressRecord>),
    ActionEvaluation(Box<NormalizedActionEvaluationAdmission>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NormalizedActionEvaluationManagement {
    pub(crate) transition: ActionEvaluationInvocationTransitionRecord,
    pub(crate) removed: Option<(SchedulerKey, ScheduledWork)>,
    pub(crate) insertion_key: SchedulerKey,
    pub(crate) insertion_work: ScheduledWork,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NormalizedManagementRecord {
    pub(crate) request: ManageRequest,
    pub(crate) resulting_mode: SessionMode,
    pub(crate) action_evaluation: Option<NormalizedActionEvaluationManagement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NormalizedManagementCause {
    HostRequests(Vec<NormalizedManagementRecord>),
    KernelSafety(KernelSafetyCause),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NormalizedAttemptResolution {
    Accepted { commit: MomentCommitRef },
    Rejected(StableCommandRejection),
    CommandIdCollision,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NormalizedAttemptSubject {
    EvaluatedCommand(CommandEnvelope),
    CommandIdCollision {
        source: CommandSource,
        command: CommandId,
        fingerprints: Vec<CommandRequestFingerprint>,
    },
}

impl NormalizedAttemptSubject {
    pub(crate) fn identity(&self) -> ContainmentCommandIdentity {
        match self {
            Self::EvaluatedCommand(command) => ContainmentCommandIdentity::from_command(command),
            Self::CommandIdCollision {
                source, command, ..
            } => ContainmentCommandIdentity::new(*source, *command),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NormalizedAttemptRecord {
    pub(crate) subject: NormalizedAttemptSubject,
    pub(crate) resolution: NormalizedAttemptResolution,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NormalizedSchedulerInsertion {
    PostCommit {
        scheduler_key: SchedulerKey,
        reaction: MomentReactionRef,
        dispatch: PreparedPostCommitDispatch,
    },
    ActionCommand {
        scheduler_key: SchedulerKey,
        opportunity: ActionOpportunityId,
        effective: SimMoment,
        command: CommandEnvelope,
    },
    AttemptResolved {
        scheduler_key: SchedulerKey,
        resolved: AttemptResolved,
    },
    Lifecycle {
        scheduler_key: SchedulerKey,
        work: LifecycleWork,
    },
    ActionReady {
        scheduler_key: SchedulerKey,
        ready: ActionReady,
    },
    ActionEvaluation {
        scheduler_key: SchedulerKey,
        work: ActionEvaluationWork,
    },
    RelocationProcess {
        scheduler_key: SchedulerKey,
        wake: RelocationProcessWake,
    },
}

impl NormalizedSchedulerInsertion {
    pub(crate) const fn post_commit(
        scheduler_key: SchedulerKey,
        reaction: MomentReactionRef,
        dispatch: PreparedPostCommitDispatch,
    ) -> Self {
        Self::PostCommit {
            scheduler_key,
            reaction,
            dispatch,
        }
    }

    pub(crate) const fn action_command(
        scheduler_key: SchedulerKey,
        opportunity: ActionOpportunityId,
        effective: SimMoment,
        command: CommandEnvelope,
    ) -> Self {
        Self::ActionCommand {
            scheduler_key,
            opportunity,
            effective,
            command,
        }
    }

    pub(crate) const fn attempt_resolved(
        scheduler_key: SchedulerKey,
        resolved: AttemptResolved,
    ) -> Self {
        Self::AttemptResolved {
            scheduler_key,
            resolved,
        }
    }

    pub(crate) const fn relocation_process(
        scheduler_key: SchedulerKey,
        wake: RelocationProcessWake,
    ) -> Self {
        Self::RelocationProcess {
            scheduler_key,
            wake,
        }
    }

    pub(crate) const fn lifecycle(scheduler_key: SchedulerKey, work: LifecycleWork) -> Self {
        Self::Lifecycle {
            scheduler_key,
            work,
        }
    }

    pub(crate) const fn action_ready(scheduler_key: SchedulerKey, ready: ActionReady) -> Self {
        Self::ActionReady {
            scheduler_key,
            ready,
        }
    }

    pub(crate) const fn action_evaluation(
        scheduler_key: SchedulerKey,
        work: ActionEvaluationWork,
    ) -> Self {
        Self::ActionEvaluation {
            scheduler_key,
            work,
        }
    }

    pub(crate) const fn scheduler_key(&self) -> SchedulerKey {
        match self {
            Self::PostCommit { scheduler_key, .. }
            | Self::ActionCommand { scheduler_key, .. }
            | Self::AttemptResolved { scheduler_key, .. }
            | Self::Lifecycle { scheduler_key, .. }
            | Self::ActionReady { scheduler_key, .. }
            | Self::ActionEvaluation { scheduler_key, .. }
            | Self::RelocationProcess { scheduler_key, .. } => *scheduler_key,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NormalizedDeliveryResolution {
    NewCommand {
        delivery: CommandDeliveryRef,
        attempt: AttemptRecordRef,
    },
    RetainedCommand {
        delivery: CommandDeliveryRef,
        original_attempt: AttemptRecordId,
        original_outcome: CommandAttemptOutcome,
    },
    CommandIdReuseMismatch {
        delivery: CommandDeliveryRef,
        original_attempt: AttemptRecordId,
    },
    NewCollision {
        delivery: CommandDeliveryRef,
        attempt: AttemptRecordRef,
    },
    RetainedCollision {
        delivery: CommandDeliveryRef,
        original_attempt: AttemptRecordId,
    },
    RetiredCommand {
        delivery: CommandDeliveryRef,
    },
    PostCommitConsumed {
        delivery: PostCommitDeliveryRef,
    },
    ActionReadyConsumed {
        delivery: ActionReadyDeliveryRef,
    },
    ActionEvaluationConsumed {
        delivery: ActionEvaluationDeliveryRef,
    },
    AttemptResolvedConsumed {
        delivery: AttemptResolvedDeliveryRef,
    },
    LifecycleConsumed {
        delivery: LifecycleDeliveryRef,
    },
    RelocationProcessCompleted {
        delivery: RelocationProcessDeliveryRef,
    },
    ObsoleteRelocationWake {
        delivery: RelocationProcessDeliveryRef,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NormalizedMomentBatch {
    pub(crate) moment: SimMoment,
    pub(crate) resulting_frontier: SimMoment,
    pub(crate) consumed_keys: Vec<SchedulerKey>,
    pub(crate) command_deliveries: Vec<CommandDeliveryRecord>,
    pub(crate) post_commit_deliveries: Vec<PostCommitDeliveryRecord>,
    pub(crate) lifecycle_deliveries: Vec<LifecycleDeliveryRecord>,
    pub(crate) action_ready_deliveries: Vec<ActionReadyDeliveryRecord>,
    pub(crate) action_evaluation_deliveries: Vec<ActionEvaluationDeliveryRecord>,
    pub(crate) attempt_resolved_deliveries: Vec<AttemptResolvedDeliveryRecord>,
    pub(crate) relocation_process_deliveries: Vec<RelocationProcessDeliveryRecord>,
    pub(crate) action_opportunity_transitions: Vec<ActionOpportunityTransitionRecord>,
    pub(crate) action_evaluation_invocation_openings: Vec<ActionEvaluationInvocationOpeningRecord>,
    pub(crate) action_evaluation_invocation_transitions:
        Vec<ActionEvaluationInvocationTransitionRecord>,
    pub(crate) action_opportunity_openings: Vec<ActionOpportunityOpeningRecord>,
    pub(crate) evidence_routing: Vec<EvidenceRoutingRecord>,
    pub(crate) evidence_assimilations: Vec<EvidenceAssimilationRecord>,
    pub(crate) appraisal_transitions: Vec<ContainmentAppraisalTransitionRecord>,
    pub(crate) intent_adoptions: Vec<IntentAdoptionRecord>,
    pub(crate) intent_transitions: Vec<IntentTransitionRecord>,
    pub(crate) activity_starts: Vec<ActivityStartRecord>,
    pub(crate) activity_transitions: Vec<ActivityTransitionRecord>,
    pub(crate) activity_terminal_transitions: Vec<ActivityTerminalTransitionRecord>,
    pub(crate) lifecycle_control_mutations: Vec<LifecycleControlMutationRecord>,
    pub(crate) relocation_attempts: Vec<RelocationAttemptRecord>,
    pub(crate) relocation_process_transitions: Vec<RelocationProcessTransitionRecord>,
    pub(crate) attempts: Vec<NormalizedAttemptRecord>,
    pub(crate) commits: Vec<ContainmentTransferDelta>,
    pub(crate) containment_delta: Vec<ContainmentTransferDelta>,
    pub(crate) reactions: Vec<ReactionEnvelope>,
    pub(crate) scheduler_insertions: Vec<NormalizedSchedulerInsertion>,
    pub(crate) resolutions: Vec<NormalizedDeliveryResolution>,
    pub(crate) resolution_evidence: ContainmentResolutionEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NormalizedAuthorityRecordBody {
    Admission(NormalizedAuthorityAdmission),
    Moment(Box<NormalizedMomentBatch>),
    Management {
        cause: Box<NormalizedManagementCause>,
        resulting_mode: SessionMode,
        preserved_frontier: SimMoment,
    },
}

pub(crate) fn authority_record_preimage(
    lineage: EpochLineageId,
    sequence: NonZeroRunRecordSeq,
    previous: PreviousAuthorityHash,
    body: &NormalizedAuthorityRecordBody,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(AUTHORITY_RECORD_DOMAIN);
    writer.write_u16(AUTHORITY_RECORD_SCHEMA_VERSION);
    fixed(&mut writer, lineage.as_bytes());
    writer.write_u64(sequence.get());
    fixed(&mut writer, previous.as_bytes());
    write_body(&mut writer, body);
    writer.finish()
}

pub(crate) fn cumulative_authority_preimage(
    previous: CumulativeAuthorityHash,
    record: AuthorityRecordId,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(CUMULATIVE_AUTHORITY_DOMAIN);
    writer.write_u16(CUMULATIVE_AUTHORITY_SCHEMA_VERSION);
    fixed(&mut writer, previous.as_bytes());
    fixed(&mut writer, record.as_bytes());
    writer.finish()
}

fn write_body(writer: &mut CanonicalWriter, body: &NormalizedAuthorityRecordBody) {
    match body {
        NormalizedAuthorityRecordBody::Admission(admission) => {
            writer.write_discriminant(0);
            match admission {
                NormalizedAuthorityAdmission::Commands(entries) => {
                    writer.write_discriminant(0);
                    collection_len(writer, entries.len());
                    for (position, entry) in entries.iter().enumerate() {
                        let index = u32::try_from(position).unwrap_or_else(|_| {
                            unreachable!("normalized command admission index must fit u32")
                        });
                        writer.write_u32(index);
                        writer.write_u64(entry.prepared.input().get());
                        fixed(writer, entry.prepared.request_fingerprint().as_bytes());
                        moment(writer, entry.prepared.effective());
                        command(writer, entry.prepared.command());
                        scheduler_key_bytes(writer, entry.scheduler_key);
                        super::SameRecordCapturedInputRef::new(
                            super::CapturedInputLocalIndex::new(index),
                        )
                        .write_canonical(writer);
                        fixed(writer, entry.prepared.trigger().as_bytes());
                        writer.write_u64(entry.prepared.input().get());
                        fixed(writer, entry.prepared.request_fingerprint().as_bytes());
                        super::SameRecordCapturedInputRef::new(
                            super::CapturedInputLocalIndex::new(index),
                        )
                        .write_canonical(writer);
                        fixed(writer, entry.prepared.trigger().as_bytes());
                        super::CurrentRecordRef::new().write_canonical(writer);
                        moment(writer, entry.prepared.effective());
                    }
                }
                NormalizedAuthorityAdmission::ActionEvaluation(admission) => {
                    writer.write_discriminant(1);
                    admission.request.write_canonical(writer);
                    super::CurrentRecordRef::new().write_canonical(writer);
                    write_action_evaluation_invocation_transition(writer, &admission.transition);
                    scheduler_key_bytes(writer, admission.scheduler_key);
                    write_scheduled_work(writer, &admission.work);
                }
            }
        }
        NormalizedAuthorityRecordBody::Management {
            cause,
            resulting_mode,
            preserved_frontier,
        } => {
            writer.write_discriminant(2);
            match cause.as_ref() {
                NormalizedManagementCause::HostRequests(entries) => {
                    writer.write_discriminant(0);
                    collection_len(writer, entries.len());
                    for entry in entries {
                        writer.write_u64(entry.request.id().get());
                        fixed(writer, entry.request.fingerprint().as_bytes());
                        write_management_operation(writer, entry.request.operation());
                        super::CurrentRecordRef::new().write_canonical(writer);
                        writer.write_discriminant(entry.resulting_mode.canonical_tag());
                        match &entry.action_evaluation {
                            None => writer.write_discriminant(0),
                            Some(effect) => {
                                writer.write_discriminant(1);
                                write_action_evaluation_invocation_transition(
                                    writer,
                                    &effect.transition,
                                );
                                match &effect.removed {
                                    None => writer.write_discriminant(0),
                                    Some((key, work)) => {
                                        writer.write_discriminant(1);
                                        scheduler_key_bytes(writer, *key);
                                        write_scheduled_work(writer, work);
                                    }
                                }
                                scheduler_key_bytes(writer, effect.insertion_key);
                                write_scheduled_work(writer, &effect.insertion_work);
                            }
                        }
                    }
                }
                NormalizedManagementCause::KernelSafety(cause) => {
                    writer.write_discriminant(1);
                    if writer
                        .write_bytes(cause.canonical_bytes().as_bytes())
                        .is_err()
                    {
                        unreachable!("bounded kernel safety cause must fit canonical encoding");
                    }
                }
            }
            writer.write_discriminant(resulting_mode.canonical_tag());
            moment(writer, *preserved_frontier);
        }
        NormalizedAuthorityRecordBody::Moment(batch) => {
            writer.write_discriminant(1);
            write_moment_batch(writer, batch);
        }
    }
}

fn write_moment_batch(writer: &mut CanonicalWriter, batch: &NormalizedMomentBatch) {
    moment(writer, batch.moment);
    moment(writer, batch.resulting_frontier);

    collection_len(writer, batch.consumed_keys.len());
    for key in &batch.consumed_keys {
        scheduler_key_bytes(writer, *key);
    }

    collection_len(writer, batch.command_deliveries.len());
    for delivery in &batch.command_deliveries {
        scheduler_key_bytes(writer, delivery.scheduler_key());
        write_scheduled_command(writer, delivery.scheduled());
    }

    collection_len(writer, batch.post_commit_deliveries.len());
    for delivery in &batch.post_commit_deliveries {
        scheduler_key_bytes(writer, delivery.scheduler_key());
        write_post_commit_dispatch(writer, delivery.dispatch());
    }

    collection_len(writer, batch.lifecycle_deliveries.len());
    for delivery in &batch.lifecycle_deliveries {
        scheduler_key_bytes(writer, delivery.scheduler_key());
        write_lifecycle_work(writer, delivery.work());
    }

    collection_len(writer, batch.attempts.len());
    for attempt in &batch.attempts {
        match &attempt.subject {
            NormalizedAttemptSubject::EvaluatedCommand(command_envelope) => {
                writer.write_discriminant(0);
                command(writer, command_envelope);
            }
            NormalizedAttemptSubject::CommandIdCollision {
                source,
                command,
                fingerprints,
            } => {
                writer.write_discriminant(1);
                fixed(writer, source.as_bytes());
                writer.write_u64(command.get());
                collection_len(writer, fingerprints.len());
                for fingerprint in fingerprints {
                    fixed(writer, fingerprint.as_bytes());
                }
            }
        }
        match attempt.resolution {
            NormalizedAttemptResolution::Accepted { commit } => {
                writer.write_discriminant(0);
                commit.write_canonical(writer);
            }
            NormalizedAttemptResolution::Rejected(reason) => {
                writer.write_discriminant(1);
                writer.write_discriminant(rejection_tag(reason));
            }
            NormalizedAttemptResolution::CommandIdCollision => {
                writer.write_discriminant(2);
            }
        }
    }

    collection_len(writer, batch.commits.len());
    for delta in &batch.commits {
        transfer(writer, *delta);
        transfer_event(writer, *delta);
    }

    collection_len(writer, batch.containment_delta.len());
    for delta in &batch.containment_delta {
        transfer(writer, *delta);
    }

    collection_len(writer, batch.reactions.len());
    for reaction in &batch.reactions {
        write_reaction_envelope(writer, reaction);
    }

    collection_len(writer, batch.scheduler_insertions.len());
    for insertion in &batch.scheduler_insertions {
        match insertion {
            NormalizedSchedulerInsertion::PostCommit {
                scheduler_key,
                reaction,
                dispatch,
            } => {
                scheduler_key_bytes(writer, *scheduler_key);
                reaction.write_canonical(writer);
                write_prepared_post_commit_dispatch(writer, dispatch);
            }
            NormalizedSchedulerInsertion::ActionCommand {
                scheduler_key,
                opportunity,
                effective,
                command: action_command,
            } => {
                scheduler_key_bytes(writer, *scheduler_key);
                writer.write_discriminant(1);
                fixed(writer, opportunity.as_bytes());
                moment(writer, *effective);
                command(writer, action_command);
            }
            NormalizedSchedulerInsertion::AttemptResolved {
                scheduler_key,
                resolved,
            } => {
                scheduler_key_bytes(writer, *scheduler_key);
                write_attempt_resolved(writer, *resolved);
            }
            NormalizedSchedulerInsertion::RelocationProcess {
                scheduler_key,
                wake,
            } => {
                scheduler_key_bytes(writer, *scheduler_key);
                writer.write_discriminant(2);
                write_relocation_process_wake(writer, *wake);
            }
            NormalizedSchedulerInsertion::Lifecycle {
                scheduler_key,
                work,
            } => {
                scheduler_key_bytes(writer, *scheduler_key);
                writer.write_discriminant(3);
                write_lifecycle_work(writer, *work);
            }
            NormalizedSchedulerInsertion::ActionReady {
                scheduler_key,
                ready,
            } => {
                scheduler_key_bytes(writer, *scheduler_key);
                writer.write_discriminant(4);
                write_action_ready(writer, *ready);
            }
            NormalizedSchedulerInsertion::ActionEvaluation {
                scheduler_key,
                work,
            } => {
                scheduler_key_bytes(writer, *scheduler_key);
                writer.write_discriminant(5);
                write_action_evaluation_work(writer, *work);
            }
        }
    }

    collection_len(writer, batch.resolutions.len());
    for resolution in &batch.resolutions {
        match *resolution {
            NormalizedDeliveryResolution::NewCommand { delivery, attempt } => {
                writer.write_discriminant(0);
                write_command_delivery_ref(writer, delivery);
                attempt.write_canonical(writer);
            }
            NormalizedDeliveryResolution::RetainedCommand {
                delivery,
                original_attempt,
                original_outcome,
            } => {
                writer.write_discriminant(1);
                write_command_delivery_ref(writer, delivery);
                fixed(writer, original_attempt.as_bytes());
                outcome(writer, original_outcome);
            }
            NormalizedDeliveryResolution::CommandIdReuseMismatch {
                delivery,
                original_attempt,
            } => {
                writer.write_discriminant(2);
                write_command_delivery_ref(writer, delivery);
                fixed(writer, original_attempt.as_bytes());
            }
            NormalizedDeliveryResolution::NewCollision { delivery, attempt } => {
                writer.write_discriminant(4);
                write_command_delivery_ref(writer, delivery);
                attempt.write_canonical(writer);
            }
            NormalizedDeliveryResolution::RetainedCollision {
                delivery,
                original_attempt,
            } => {
                writer.write_discriminant(5);
                write_command_delivery_ref(writer, delivery);
                fixed(writer, original_attempt.as_bytes());
            }
            NormalizedDeliveryResolution::RetiredCommand { delivery } => {
                writer.write_discriminant(6);
                write_command_delivery_ref(writer, delivery);
            }
            NormalizedDeliveryResolution::PostCommitConsumed { delivery } => {
                writer.write_discriminant(3);
                write_post_commit_delivery_ref(writer, delivery);
            }
            NormalizedDeliveryResolution::ActionReadyConsumed { delivery } => {
                writer.write_discriminant(7);
                write_action_ready_delivery_ref(writer, delivery);
            }
            NormalizedDeliveryResolution::ActionEvaluationConsumed { delivery } => {
                writer.write_discriminant(12);
                write_action_evaluation_delivery_ref(writer, delivery);
            }
            NormalizedDeliveryResolution::AttemptResolvedConsumed { delivery } => {
                writer.write_discriminant(8);
                write_attempt_resolved_delivery_ref(writer, delivery);
            }
            NormalizedDeliveryResolution::RelocationProcessCompleted { delivery } => {
                writer.write_discriminant(9);
                write_relocation_process_delivery_ref(writer, delivery);
            }
            NormalizedDeliveryResolution::ObsoleteRelocationWake { delivery } => {
                writer.write_discriminant(10);
                write_relocation_process_delivery_ref(writer, delivery);
            }
            NormalizedDeliveryResolution::LifecycleConsumed { delivery } => {
                writer.write_discriminant(11);
                write_lifecycle_delivery_ref(writer, delivery);
            }
        }
    }

    write_containment_resolution_evidence(writer, &batch.resolution_evidence);

    if !batch.action_ready_deliveries.is_empty()
        || !batch.action_evaluation_deliveries.is_empty()
        || !batch.attempt_resolved_deliveries.is_empty()
        || !batch.action_opportunity_transitions.is_empty()
        || !batch.action_evaluation_invocation_openings.is_empty()
        || !batch.action_evaluation_invocation_transitions.is_empty()
        || !batch.action_opportunity_openings.is_empty()
        || !batch.evidence_routing.is_empty()
        || !batch.evidence_assimilations.is_empty()
        || !batch.appraisal_transitions.is_empty()
        || !batch.intent_adoptions.is_empty()
        || !batch.intent_transitions.is_empty()
        || !batch.activity_starts.is_empty()
        || !batch.activity_transitions.is_empty()
        || !batch.activity_terminal_transitions.is_empty()
        || !batch.lifecycle_control_mutations.is_empty()
        || !batch.relocation_process_deliveries.is_empty()
        || !batch.relocation_attempts.is_empty()
        || !batch.relocation_process_transitions.is_empty()
    {
        writer.write_discriminant(0);

        collection_len(writer, batch.action_ready_deliveries.len());
        for delivery in &batch.action_ready_deliveries {
            scheduler_key_bytes(writer, delivery.scheduler_key());
            write_action_ready(writer, delivery.ready());
        }

        collection_len(writer, batch.action_evaluation_deliveries.len());
        for delivery in &batch.action_evaluation_deliveries {
            scheduler_key_bytes(writer, delivery.scheduler_key());
            write_action_evaluation_work(writer, delivery.work());
        }

        collection_len(writer, batch.attempt_resolved_deliveries.len());
        for delivery in &batch.attempt_resolved_deliveries {
            scheduler_key_bytes(writer, delivery.scheduler_key());
            write_attempt_resolved(writer, delivery.resolved());
        }

        collection_len(writer, batch.action_opportunity_transitions.len());
        for transition in &batch.action_opportunity_transitions {
            write_action_opportunity(writer, transition.before());
            write_action_opportunity(writer, transition.after());
        }

        collection_len(writer, batch.action_evaluation_invocation_openings.len());
        for opening in &batch.action_evaluation_invocation_openings {
            match opening.cause() {
                ActionEvaluationInvocationOpeningCause::ActionReady(delivery) => {
                    writer.write_discriminant(0);
                    write_action_ready_delivery_ref(writer, delivery);
                }
                ActionEvaluationInvocationOpeningCause::VisibleReinvocation(delivery) => {
                    writer.write_discriminant(1);
                    write_action_evaluation_delivery_ref(writer, delivery);
                }
            }
            fixed(writer, opening.invocation().digest().as_bytes());
        }

        collection_len(writer, batch.action_evaluation_invocation_transitions.len());
        for transition in &batch.action_evaluation_invocation_transitions {
            write_action_evaluation_invocation_transition(writer, transition);
        }

        collection_len(writer, batch.action_opportunity_openings.len());
        for opening in &batch.action_opportunity_openings {
            write_action_opportunity(writer, opening.opportunity());
        }

        collection_len(writer, batch.evidence_routing.len());
        for routing in &batch.evidence_routing {
            match routing.source() {
                EvidenceRoutingSource::PhysicalEvent {
                    dispatch,
                    event_index,
                } => {
                    writer.write_discriminant(0);
                    write_post_commit_delivery_ref(writer, dispatch);
                    writer.write_u32(event_index);
                }
                EvidenceRoutingSource::RejectedContainmentAttempt { attempt } => {
                    writer.write_discriminant(1);
                    attempt.write_canonical(writer);
                }
            }
            write_evidence_record(writer, routing.evidence());
        }

        collection_len(writer, batch.evidence_assimilations.len());
        for assimilation in &batch.evidence_assimilations {
            fixed(writer, assimilation.actor().as_bytes());
            writer.write_u64(assimilation.expected_version().get());
            collection_len(writer, assimilation.evidence().len());
            for evidence in assimilation.evidence() {
                write_evidence_record(writer, *evidence);
            }
        }

        collection_len(writer, batch.appraisal_transitions.len());
        for transition in &batch.appraisal_transitions {
            match *transition {
                ContainmentAppraisalTransitionRecord::Present { before, after } => {
                    writer.write_discriminant(0);
                    match before {
                        Some(before) => {
                            writer.write_discriminant(1);
                            write_appraisal(writer, before);
                        }
                        None => writer.write_discriminant(0),
                    }
                    write_appraisal(writer, after);
                }
                ContainmentAppraisalTransitionRecord::Retracted {
                    before,
                    supporting_evidence,
                } => {
                    writer.write_discriminant(1);
                    write_appraisal(writer, before);
                    fixed(writer, supporting_evidence.as_bytes());
                }
            }
        }

        collection_len(writer, batch.intent_adoptions.len());
        for adoption in &batch.intent_adoptions {
            write_intent(writer, adoption.intent());
        }

        collection_len(writer, batch.intent_transitions.len());
        for transition in &batch.intent_transitions {
            write_intent(writer, transition.before());
            write_intent(writer, transition.after());
        }

        collection_len(writer, batch.activity_starts.len());
        for start in &batch.activity_starts {
            write_activity(writer, start.activity());
        }

        collection_len(writer, batch.activity_transitions.len());
        for transition in &batch.activity_transitions {
            write_activity(writer, transition.before());
            write_activity(writer, transition.after());
        }

        collection_len(writer, batch.activity_terminal_transitions.len());
        for transition in &batch.activity_terminal_transitions {
            write_activity(writer, transition.activity_before());
            write_activity(writer, transition.activity_after());
            write_intent(writer, transition.intent_before());
            write_intent(writer, transition.intent_after());
        }

        collection_len(writer, batch.lifecycle_control_mutations.len());
        for mutation in &batch.lifecycle_control_mutations {
            fixed(writer, mutation.actor().as_bytes());
            writer.write_discriminant(mutation.role().canonical_tag());
            collection_len(writer, mutation.requested().len());
            for cause in mutation.requested() {
                write_lifecycle_cause(writer, *cause);
            }
            match mutation.completed() {
                Some(generation) => {
                    writer.write_discriminant(1);
                    writer.write_u64(generation.get());
                }
                None => writer.write_discriminant(0),
            }
        }

        collection_len(writer, batch.relocation_process_deliveries.len());
        for delivery in &batch.relocation_process_deliveries {
            scheduler_key_bytes(writer, delivery.scheduler_key());
            write_relocation_process_wake(writer, delivery.wake());
        }

        collection_len(writer, batch.relocation_attempts.len());
        for attempt in &batch.relocation_attempts {
            write_action_resolution_delivery_ref(writer, attempt.resolution_delivery());
            write_relocation_interaction(writer, attempt.interaction());
            match attempt.resolution() {
                RelocationAttemptResolution::Accepted { process } => {
                    writer.write_discriminant(0);
                    fixed(writer, process.as_bytes());
                }
                RelocationAttemptResolution::Rejected(reason) => {
                    writer.write_discriminant(1);
                    writer.write_discriminant(relocation_rejection_tag(reason));
                }
            }
        }

        collection_len(writer, batch.relocation_process_transitions.len());
        for transition in &batch.relocation_process_transitions {
            match transition.cause() {
                RelocationProcessTransitionCause::Action(delivery) => {
                    writer.write_discriminant(0);
                    write_action_resolution_delivery_ref(writer, delivery);
                }
                RelocationProcessTransitionCause::Wake(delivery) => {
                    writer.write_discriminant(1);
                    write_relocation_process_delivery_ref(writer, delivery);
                }
            }
            match transition.before() {
                None => writer.write_discriminant(0),
                Some(before) => {
                    writer.write_discriminant(1);
                    fixed(writer, before.digest().as_bytes());
                }
            }
            fixed(writer, transition.after().digest().as_bytes());
            match transition.event() {
                None => writer.write_discriminant(0),
                Some(event) => {
                    writer.write_discriminant(1);
                    write_physical_event(writer, event);
                }
            }
        }
    }
}

fn write_scheduled_command(writer: &mut CanonicalWriter, scheduled: &ScheduledCommand) {
    match scheduled.cause() {
        ScheduledCommandCause::CapturedExternal {
            trigger, captured, ..
        } => {
            fixed(writer, trigger.as_bytes());
            fixed(writer, captured.as_bytes());
        }
        ScheduledCommandCause::ActionOpportunity(opportunity) => {
            writer.write_discriminant(1);
            fixed(writer, opportunity.as_bytes());
        }
    }
    command(writer, scheduled.command());
}

fn write_command_delivery_ref(writer: &mut CanonicalWriter, reference: CommandDeliveryRef) {
    writer.write_discriminant(0);
    writer.write_u32(reference.index());
}

fn write_post_commit_delivery_ref(writer: &mut CanonicalWriter, reference: PostCommitDeliveryRef) {
    writer.write_discriminant(1);
    writer.write_u32(reference.index());
}

fn write_action_ready_delivery_ref(
    writer: &mut CanonicalWriter,
    reference: ActionReadyDeliveryRef,
) {
    writer.write_discriminant(2);
    writer.write_u32(reference.index());
}

fn write_action_evaluation_delivery_ref(
    writer: &mut CanonicalWriter,
    reference: ActionEvaluationDeliveryRef,
) {
    writer.write_discriminant(6);
    writer.write_u32(reference.index());
}

fn write_action_evaluation_invocation_transition(
    writer: &mut CanonicalWriter,
    transition: &ActionEvaluationInvocationTransitionRecord,
) {
    match transition.cause() {
        ActionEvaluationInvocationTransitionCause::EvaluationDelivery(delivery) => {
            writer.write_discriminant(0);
            write_action_evaluation_delivery_ref(writer, delivery);
        }
        ActionEvaluationInvocationTransitionCause::ResultCapture(capture) => {
            writer.write_discriminant(1);
            writer.write_u64(capture.get());
        }
        ActionEvaluationInvocationTransitionCause::Management(request) => {
            writer.write_discriminant(2);
            writer.write_u64(request.get());
        }
    }
    fixed(writer, transition.expected_before().as_bytes());
    fixed(writer, transition.after().digest().as_bytes());
}

fn write_action_resolution_delivery_ref(
    writer: &mut CanonicalWriter,
    reference: ActionResolutionDeliveryRef,
) {
    match reference {
        ActionResolutionDeliveryRef::Ready(delivery) => {
            writer.write_discriminant(0);
            write_action_ready_delivery_ref(writer, delivery);
        }
        ActionResolutionDeliveryRef::Evaluation(delivery) => {
            writer.write_discriminant(1);
            write_action_evaluation_delivery_ref(writer, delivery);
        }
    }
}

fn write_attempt_resolved_delivery_ref(
    writer: &mut CanonicalWriter,
    reference: AttemptResolvedDeliveryRef,
) {
    writer.write_discriminant(3);
    writer.write_u32(reference.index());
}

fn write_relocation_process_delivery_ref(
    writer: &mut CanonicalWriter,
    reference: RelocationProcessDeliveryRef,
) {
    writer.write_discriminant(4);
    writer.write_u32(reference.index());
}

fn write_lifecycle_delivery_ref(writer: &mut CanonicalWriter, reference: LifecycleDeliveryRef) {
    writer.write_discriminant(5);
    writer.write_u32(reference.index());
}

fn write_relocation_interaction(writer: &mut CanonicalWriter, interaction: RelocationInteraction) {
    let (tag, route) = match interaction {
        RelocationInteraction::Start(route) => (0, route),
        RelocationInteraction::Pause(route) => (1, route),
        RelocationInteraction::Resume(route) => (2, route),
    };
    writer.write_discriminant(tag);
    fixed(writer, route.as_bytes());
}

const fn relocation_rejection_tag(rejection: RelocationAttemptRejection) -> u32 {
    match rejection {
        RelocationAttemptRejection::RouteUnavailable => 0,
        RelocationAttemptRejection::PositionMismatch => 1,
        RelocationAttemptRejection::ProcessUnavailable => 2,
        RelocationAttemptRejection::ProcessStateConflict => 3,
        RelocationAttemptRejection::LimitReached => 4,
    }
}

fn write_containment_resolution_evidence(
    writer: &mut CanonicalWriter,
    evidence: &ContainmentResolutionEvidence,
) {
    writer.write_discriminant(match evidence.resolution_policy() {
        MomentResolutionPolicyV2::CanonicalComponentGreedy => 1,
    });
    writer.write_discriminant(match evidence.conflict_policy() {
        ContainmentConflictPolicyV1::EqualHighestRandomWeight => 0,
    });
    writer.write_discriminant(match evidence.random_oracle_policy() {
        RandomOraclePolicyV1::Blake3KeyedPrf256 => 0,
    });
    writer.write_discriminant(match evidence.random_key_policy() {
        RandomKeyPolicyV1::SemanticContainmentConflict => 0,
    });

    collection_len(writer, evidence.components().len());
    for component in evidence.components() {
        collection_len(writer, component.contenders().len());
        for contender in component.contenders() {
            write_containment_contender(writer, *contender);
        }
        collection_len(writer, component.resources().len());
        for resource in component.resources() {
            write_containment_group(writer, resource.group());
            writer.write_u32(resource.admission_limit());
            collection_len(writer, resource.ranking().entries().len());
            for entry in resource.ranking().entries() {
                let key = entry.key().canonical_bytes();
                if writer.write_bytes(key.as_bytes()).is_err() {
                    unreachable!("semantic random key must fit canonical encoding");
                }
                fixed(writer, entry.key_id().as_bytes());
                fixed(writer, entry.score().as_bytes());
            }
            write_containment_contender(writer, resource.ranking().winner());
        }
    }

    match evidence.fallback() {
        None => writer.write_discriminant(0),
        Some(ContainmentResolutionFallback::RandomEvidence {
            group,
            admission_limit,
            error,
        }) => {
            writer.write_discriminant(1);
            write_containment_group(writer, group);
            writer.write_u32(admission_limit);
            write_random_rank_error(writer, error);
        }
        Some(ContainmentResolutionFallback::CombinedTransition { error }) => {
            writer.write_discriminant(2);
            write_containment_transition_error(writer, error);
        }
    }
}

fn write_containment_group(writer: &mut CanonicalWriter, group: ContainmentConflictGroupV1) {
    moment(writer, group.moment());
    match group.resource() {
        ContainmentConflictResourceV1::ExclusiveItem(item) => {
            writer.write_discriminant(0);
            fixed(writer, item.as_bytes());
        }
        ContainmentConflictResourceV1::DestinationCapacity(container) => {
            writer.write_discriminant(1);
            fixed(writer, container.as_bytes());
        }
    }
}

fn write_containment_contender(
    writer: &mut CanonicalWriter,
    contender: ContainmentConflictContenderV1,
) {
    fixed(writer, contender.actor().as_bytes());
    fixed(writer, contender.source().as_bytes());
    writer.write_u64(contender.command().get());
}

fn write_random_rank_error(writer: &mut CanonicalWriter, error: ContainmentRandomRankError) {
    match error {
        ContainmentRandomRankError::EmptyConflictGroup => writer.write_discriminant(0),
        ContainmentRandomRankError::SemanticKeyReuse { key, first, second } => {
            writer.write_discriminant(1);
            fixed(writer, key.as_bytes());
            write_containment_contender(writer, first);
            write_containment_contender(writer, second);
        }
        ContainmentRandomRankError::ScoreCollision {
            score,
            first,
            second,
        } => {
            writer.write_discriminant(2);
            fixed(writer, score.as_bytes());
            fixed(writer, first.as_bytes());
            fixed(writer, second.as_bytes());
        }
    }
}

fn write_containment_transition_error(
    writer: &mut CanonicalWriter,
    error: ContainmentTransitionError,
) {
    match error {
        ContainmentTransitionError::ItemNotContained { item } => {
            writer.write_discriminant(0);
            fixed(writer, item.as_bytes());
        }
        ContainmentTransitionError::SourceMismatch {
            item,
            actual,
            expected,
        } => {
            writer.write_discriminant(1);
            fixed(writer, item.as_bytes());
            fixed(writer, actual.as_bytes());
            fixed(writer, expected.as_bytes());
        }
        ContainmentTransitionError::DestinationContainerMissing { container } => {
            writer.write_discriminant(2);
            fixed(writer, container.as_bytes());
        }
        ContainmentTransitionError::SourceAuthorityMissing { actor, container } => {
            writer.write_discriminant(3);
            fixed(writer, actor.as_bytes());
            fixed(writer, container.as_bytes());
        }
        ContainmentTransitionError::DuplicateItemClaim { item } => {
            writer.write_discriminant(4);
            fixed(writer, item.as_bytes());
        }
        ContainmentTransitionError::InvalidSuccessor(error) => {
            writer.write_discriminant(5);
            write_domain_state_error(writer, error);
        }
    }
}

fn write_domain_state_error(writer: &mut CanonicalWriter, error: DomainStateError) {
    match error {
        DomainStateError::DuplicateContainer { container } => {
            writer.write_discriminant(0);
            fixed(writer, container.as_bytes());
        }
        DomainStateError::DuplicateContainment { item } => {
            writer.write_discriminant(1);
            fixed(writer, item.as_bytes());
        }
        DomainStateError::DuplicateContainerAuthority { actor, container } => {
            writer.write_discriminant(2);
            fixed(writer, actor.as_bytes());
            fixed(writer, container.as_bytes());
        }
        DomainStateError::MissingContainmentContainer { item, container } => {
            writer.write_discriminant(3);
            fixed(writer, item.as_bytes());
            fixed(writer, container.as_bytes());
        }
        DomainStateError::MissingAuthorityContainer { actor, container } => {
            writer.write_discriminant(4);
            fixed(writer, actor.as_bytes());
            fixed(writer, container.as_bytes());
        }
        DomainStateError::DirectSelfContainment { item } => {
            writer.write_discriminant(5);
            fixed(writer, item.as_bytes());
        }
        DomainStateError::ContainerUsedAsItem { item } => {
            writer.write_discriminant(6);
            fixed(writer, item.as_bytes());
        }
        DomainStateError::ContainerCapacityExceeded {
            container,
            capacity,
            actual,
        } => {
            writer.write_discriminant(7);
            fixed(writer, container.as_bytes());
            writer.write_u32(capacity);
            writer.write_u64(actual);
        }
        DomainStateError::DuplicateRoute { route } => {
            writer.write_discriminant(8);
            fixed(writer, route.as_bytes());
        }
        DomainStateError::DuplicateDirectedEndpoints {
            source,
            destination,
        } => {
            writer.write_discriminant(9);
            fixed(writer, source.as_bytes());
            fixed(writer, destination.as_bytes());
        }
        DomainStateError::DuplicateActorPosition { actor } => {
            writer.write_discriminant(10);
            fixed(writer, actor.as_bytes());
        }
        DomainStateError::MissingTransitRoute {
            actor,
            source,
            destination,
        } => {
            writer.write_discriminant(11);
            fixed(writer, actor.as_bytes());
            fixed(writer, source.as_bytes());
            fixed(writer, destination.as_bytes());
        }
    }
}

fn write_action_ready(writer: &mut CanonicalWriter, ready: ActionReady) {
    fixed(writer, ready.opportunity().as_bytes());
    writer.write_u64(ready.expected_version().get());
    moment(writer, ready.due());
}

fn write_scheduled_work(writer: &mut CanonicalWriter, work: &ScheduledWork) {
    match work {
        ScheduledWork::Command(command) => {
            writer.write_discriminant(0);
            write_scheduled_command(writer, command);
        }
        ScheduledWork::PostCommit(dispatch) => {
            writer.write_discriminant(1);
            write_post_commit_dispatch(writer, dispatch);
        }
        ScheduledWork::Process(wake) => {
            writer.write_discriminant(2);
            write_relocation_process_wake(writer, *wake);
        }
        ScheduledWork::Lifecycle(work) => {
            writer.write_discriminant(3);
            write_lifecycle_work(writer, *work);
        }
        ScheduledWork::ActionReady(ready) => {
            writer.write_discriminant(4);
            write_action_ready(writer, *ready);
        }
        ScheduledWork::ActionEvaluation(work) => {
            writer.write_discriminant(5);
            write_action_evaluation_work(writer, *work);
        }
    }
}

fn write_action_evaluation_work(writer: &mut CanonicalWriter, work: ActionEvaluationWork) {
    writer.write_discriminant(work.canonical_tag());
    fixed(writer, work.invocation().as_bytes());
    fixed(writer, work.opportunity().as_bytes());
    writer.write_u64(work.expected_waiting_version().get());
    moment(writer, work.due());
    if let Some(cause) = work.fallback_cause() {
        match cause {
            ActionEvaluationFallbackCause::Cancelled => writer.write_discriminant(0),
            ActionEvaluationFallbackCause::TimedOut => writer.write_discriminant(1),
            ActionEvaluationFallbackCause::HostFailure => writer.write_discriminant(2),
            ActionEvaluationFallbackCause::InvalidResult => writer.write_discriminant(3),
            ActionEvaluationFallbackCause::VisibleReinvocationExhausted => {
                writer.write_discriminant(4);
            }
            ActionEvaluationFallbackCause::ArtifactRejected(failure) => {
                writer.write_discriminant(5);
                writer.write_discriminant(match failure.role() {
                    ActionEvaluationArtifactRole::Request => 0,
                    ActionEvaluationArtifactRole::Result => 1,
                    ActionEvaluationArtifactRole::PrivateContinuation => 2,
                    ActionEvaluationArtifactRole::PrivateReadWitness => 3,
                });
                fixed(writer, failure.schema().as_bytes());
                writer.write_u64(failure.actual_length());
                fixed(writer, failure.digest().as_bytes());
            }
        }
    }
}

fn write_attempt_resolved(writer: &mut CanonicalWriter, resolved: AttemptResolved) {
    fixed(writer, resolved.opportunity().as_bytes());
    moment(writer, resolved.due());
}

fn write_relocation_process_wake(writer: &mut CanonicalWriter, wake: RelocationProcessWake) {
    fixed(writer, wake.process().as_bytes());
    writer.write_u64(wake.process_generation().get());
    writer.write_u64(wake.expected_version().get());
    writer.write_u64(wake.wake_generation().get());
    moment(writer, wake.due());
}

fn write_action_opportunity(writer: &mut CanonicalWriter, opportunity: &ActionOpportunity) {
    fixed(writer, opportunity.digest().as_bytes());
}

fn write_lifecycle_work(writer: &mut CanonicalWriter, work: LifecycleWork) {
    writer.write_discriminant(work.canonical_tag());
    match work {
        LifecycleWork::EvidenceDelivery(delivery) => {
            write_evidence_record(writer, delivery.evidence());
        }
        LifecycleWork::Appraisal(work) => {
            fixed(writer, work.actor().as_bytes());
            writer.write_u64(work.generation().get());
        }
        LifecycleWork::IntentReview(work) => {
            fixed(writer, work.actor().as_bytes());
            writer.write_u64(work.generation().get());
        }
        LifecycleWork::ActivityInitialization(work) => {
            fixed(writer, work.actor().as_bytes());
            writer.write_u64(work.generation().get());
        }
        LifecycleWork::AttemptResolved(work) => {
            fixed(writer, work.opportunity().as_bytes());
        }
        LifecycleWork::ActivityAdvance(work) => {
            fixed(writer, work.actor().as_bytes());
            writer.write_u64(work.generation().get());
        }
    }
    moment(writer, work.due());
}

fn write_lifecycle_cause(writer: &mut CanonicalWriter, cause: LifecycleCause) {
    match cause {
        LifecycleCause::Evidence(evidence) => {
            writer.write_discriminant(0);
            fixed(writer, evidence.as_bytes());
        }
        LifecycleCause::Appraisal {
            generation,
            material,
        } => {
            writer.write_discriminant(1);
            writer.write_u64(generation.get());
            fixed(writer, material.as_bytes());
        }
        LifecycleCause::Intent { intent, version } => {
            writer.write_discriminant(2);
            fixed(writer, intent.as_bytes());
            writer.write_u64(version.get());
        }
        LifecycleCause::AttemptResolved(opportunity) => {
            writer.write_discriminant(3);
            fixed(writer, opportunity.as_bytes());
        }
    }
}

fn write_evidence_record(writer: &mut CanonicalWriter, evidence: EvidenceRecord) {
    fixed(writer, evidence.id().as_bytes());
    fixed(writer, evidence.observer().as_bytes());
    writer.write_u64(evidence.generation().get());
    match evidence.provenance() {
        world_model::EvidenceProvenance::DirectItemTransfer(event) => {
            writer.write_discriminant(0);
            fixed(writer, event.actor().as_bytes());
            fixed(writer, event.item().as_bytes());
            fixed(writer, event.source().as_bytes());
            fixed(writer, event.destination().as_bytes());
        }
        world_model::EvidenceProvenance::DirectActorDeparture(observation) => {
            writer.write_discriminant(1);
            fixed(writer, observation.actor().as_bytes());
            fixed(writer, observation.source().as_bytes());
            fixed(writer, observation.destination().as_bytes());
        }
        world_model::EvidenceProvenance::DirectActorArrival(observation) => {
            writer.write_discriminant(2);
            fixed(writer, observation.actor().as_bytes());
            fixed(writer, observation.source().as_bytes());
            fixed(writer, observation.destination().as_bytes());
        }
        world_model::EvidenceProvenance::DirectItemAbsent(observation) => {
            writer.write_discriminant(3);
            fixed(writer, observation.item().as_bytes());
            fixed(writer, observation.expected_container().as_bytes());
        }
    }
}

fn write_appraisal(writer: &mut CanonicalWriter, appraisal: ContainmentAppraisal) {
    fixed(writer, appraisal.actor().as_bytes());
    fixed(writer, appraisal.item().as_bytes());
    fixed(writer, appraisal.believed_current_container().as_bytes());
    fixed(writer, appraisal.restore_container().as_bytes());
    fixed(writer, appraisal.supporting_evidence().as_bytes());
}

fn write_intent(writer: &mut CanonicalWriter, intent: Intent) {
    fixed(writer, intent.id().as_bytes());
    fixed(writer, intent.actor().as_bytes());
    writer.write_u64(intent.generation().get());
    writer.write_u64(intent.version().get());
    match intent.desired() {
        world_model::DesiredCondition::ItemContainedIn { item, container } => {
            writer.write_discriminant(0);
            fixed(writer, item.as_bytes());
            fixed(writer, container.as_bytes());
        }
        world_model::DesiredCondition::ActorAt { location } => {
            writer.write_discriminant(1);
            fixed(writer, location.as_bytes());
        }
    }
    writer.write_discriminant(match intent.status() {
        world_model::IntentStatus::Active => 0,
        world_model::IntentStatus::Suspended => 1,
        world_model::IntentStatus::Achieved => 2,
        world_model::IntentStatus::Abandoned => 3,
        world_model::IntentStatus::Failed => 4,
    });
}

fn write_activity(writer: &mut CanonicalWriter, activity: Activity) {
    fixed(writer, activity.id().as_bytes());
    fixed(writer, activity.actor().as_bytes());
    fixed(writer, activity.intent().as_bytes());
    writer.write_u64(activity.generation().get());
    writer.write_u64(activity.version().get());
    fixed(writer, activity.controller().as_bytes());
    fixed(writer, activity.state_schema().as_bytes());
    writer.write_discriminant(match activity.status() {
        world_model::ActivityStatus::Active => 0,
        world_model::ActivityStatus::Waiting => 1,
        world_model::ActivityStatus::Suspended => 2,
        world_model::ActivityStatus::Completed => 3,
        world_model::ActivityStatus::Failed => 4,
        world_model::ActivityStatus::Cancelled => 5,
    });
    match activity.state() {
        world_model::ActivityState::ContainmentTransfer(state) => {
            writer.write_discriminant(0);
            fixed(writer, state.item().as_bytes());
            fixed(writer, state.source().as_bytes());
            fixed(writer, state.destination().as_bytes());
            writer.write_u64(state.next_opportunity_generation().get());
            writer.write_u32(state.remaining_attempts());
        }
        world_model::ActivityState::Travel(state) => {
            writer.write_discriminant(1);
            fixed(writer, state.source().as_bytes());
            fixed(writer, state.destination().as_bytes());
            writer.write_u64(state.next_opportunity_generation().get());
            writer.write_discriminant(match state.step() {
                world_model::TravelActivityStep::Pause => 0,
                world_model::TravelActivityStep::Resume => 1,
                world_model::TravelActivityStep::AwaitArrival => 2,
            });
        }
    }
}

fn write_prepared_post_commit_dispatch(
    writer: &mut CanonicalWriter,
    dispatch: &PreparedPostCommitDispatch,
) {
    fixed(writer, dispatch.id().as_bytes());
    moment(writer, dispatch.source_moment());
    write_reaction_envelope(writer, dispatch.reaction());
}

fn write_post_commit_dispatch(writer: &mut CanonicalWriter, dispatch: &PostCommitDispatch) {
    fixed(writer, dispatch.id().as_bytes());
    moment(writer, dispatch.source_moment());
    write_reaction_envelope(writer, dispatch.reaction());
}

fn command(writer: &mut CanonicalWriter, command: &CommandEnvelope) {
    command_coordinates(
        writer,
        command.source(),
        command.id(),
        command.fingerprint().as_bytes(),
    );
}
fn command_coordinates(
    writer: &mut CanonicalWriter,
    source: world_model::CommandSource,
    id: world_model::CommandId,
    fingerprint: &[u8; 32],
) {
    fixed(writer, source.as_bytes());
    writer.write_u64(id.get());
    fixed(writer, fingerprint);
}
fn outcome(writer: &mut CanonicalWriter, value: CommandAttemptOutcome) {
    match value {
        CommandAttemptOutcome::Accepted => writer.write_discriminant(0),
        CommandAttemptOutcome::Rejected(reason) => {
            writer.write_discriminant(1);
            writer.write_discriminant(rejection_tag(reason));
        }
    }
}
fn transfer(writer: &mut CanonicalWriter, delta: ContainmentTransferDelta) {
    fixed(writer, delta.actor().as_bytes());
    fixed(writer, delta.item().as_bytes());
    fixed(writer, delta.expected_source().as_bytes());
    fixed(writer, delta.destination().as_bytes());
}
fn transfer_event(writer: &mut CanonicalWriter, delta: ContainmentTransferDelta) {
    writer.write_discriminant(0);
    transfer(writer, delta);
}
fn write_reaction_envelope(writer: &mut CanonicalWriter, reaction: &ReactionEnvelope) {
    writer.write_u64(
        u64::try_from(reaction.events().len()).unwrap_or_else(|error| {
            panic!("reaction envelope length must fit canonical encoding: {error}")
        }),
    );
    for event in reaction.events() {
        write_physical_event(writer, *event);
    }
}
fn write_physical_event(writer: &mut CanonicalWriter, event: PhysicalEvent) {
    match event {
        PhysicalEvent::ItemTransferred(event) => {
            writer.write_discriminant(0);
            fixed(writer, event.actor().as_bytes());
            fixed(writer, event.item().as_bytes());
            fixed(writer, event.source().as_bytes());
            fixed(writer, event.destination().as_bytes());
        }
        PhysicalEvent::ActorDeparted(event) => {
            writer.write_discriminant(1);
            fixed(writer, event.process().as_bytes());
            fixed(writer, event.actor().as_bytes());
            fixed(writer, event.source().as_bytes());
            fixed(writer, event.destination().as_bytes());
        }
        PhysicalEvent::ActorArrived(event) => {
            writer.write_discriminant(2);
            fixed(writer, event.process().as_bytes());
            fixed(writer, event.actor().as_bytes());
            fixed(writer, event.source().as_bytes());
            fixed(writer, event.destination().as_bytes());
        }
    }
}
fn scheduler_key_bytes(writer: &mut CanonicalWriter, key: SchedulerKey) {
    moment(writer, key.moment());
    writer.write_discriminant(key.lane().canonical_tag());
    writer.write_u64(key.sequence().get());
}
fn moment(writer: &mut CanonicalWriter, value: SimMoment) {
    writer.write_u64(value.time().ticks());
    writer.write_u64(value.microstep().get());
}
fn rejection_tag(reason: StableCommandRejection) -> u32 {
    match reason {
        StableCommandRejection::DefinitionUnavailable => 0,
        StableCommandRejection::BindingMismatch => 1,
        StableCommandRejection::Stale => 2,
        StableCommandRejection::RequirementUnsatisfied => 3,
        StableCommandRejection::Conflict => 4,
        StableCommandRejection::IdCollision => 5,
    }
}
fn write_management_operation(writer: &mut CanonicalWriter, operation: SessionManagement) {
    match operation {
        SessionManagement::Pause => writer.write_discriminant(0),
        SessionManagement::Resume => writer.write_discriminant(1),
        SessionManagement::Retire(retirement) => {
            writer.write_discriminant(2);
            match retirement {
                LedgerRetirement::InputThrough(target) => {
                    writer.write_discriminant(0);
                    writer.write_u64(target.get());
                }
                LedgerRetirement::ManagementThrough(target) => {
                    writer.write_discriminant(1);
                    writer.write_u64(target.get());
                }
                LedgerRetirement::CommandThrough { source, command } => {
                    writer.write_discriminant(2);
                    fixed(writer, source.as_bytes());
                    writer.write_u64(command.get());
                }
            }
        }
        SessionManagement::SealAdmissionThrough(frontier) => {
            writer.write_discriminant(3);
            moment(writer, frontier);
        }
        SessionManagement::Quarantine => writer.write_discriminant(4),
        SessionManagement::Fail => writer.write_discriminant(5),
        SessionManagement::ResolveActionEvaluation {
            invocation,
            disposition,
        } => {
            writer.write_discriminant(6);
            fixed(writer, invocation.as_bytes());
            writer.write_discriminant(disposition.canonical_tag());
        }
    }
}
fn collection_len(writer: &mut CanonicalWriter, len: usize) {
    writer.write_u64(
        u64::try_from(len)
            .unwrap_or_else(|_| unreachable!("normalized collection length must fit u64")),
    );
}
fn fixed(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width authority value must fit canonical encoding");
    }
}

#[cfg(test)]
mod tests {
    use world_core::{ActorId, ContentDigest, EntityId, Microstep, SimTime};
    use world_model::{
        AcceptedState, ActionInteractionScope, ActionOpportunityDisposition,
        ActionOpportunityGeneration, ActionSponsor, ActorReactionCause, AgencyState,
        ContainmentInteractionScope, DomainState, EpistemicState, SocialState,
    };

    use crate::action_evaluation::{
        ActionEvaluationArtifactSchemaId, ActionEvaluationCapturePayload,
        ActionEvaluationInvocationLedger, ActionEvaluationPrivateContinuationArtifact,
        ActionEvaluationPrivateReadWitnessArtifact, ActionEvaluationRequestArtifact,
        ActionEvaluationResultFreshness, ActionEvaluationResultSubmission,
    };
    use crate::authority::EpochIdentity;
    use crate::execution::{
        DeferredActionAdmissionModeV1, DeferredActionControlV1, ExecutionSpecId,
        ExternalInputNamespaceId, InitialStateRootId, LifecycleImplementationId, RootSeed,
    };
    use crate::kernel::{
        ActionEvaluationManagementDisposition, ContainmentCandidateSet, InputId,
        ManagementRequestId, resolve_containment_candidates,
    };
    use crate::randomness::Blake3KeyedPrf256V1;
    use crate::scheduler::{SchedulerLaneV2, SchedulerSequence};

    use super::*;

    fn fixture_context() -> (EpochLineageId, NonZeroRunRecordSeq, PreviousAuthorityHash) {
        let sequence = match NonZeroRunRecordSeq::new(5) {
            Some(sequence) => sequence,
            None => panic!("fixture sequence is nonzero"),
        };
        (
            EpochLineageId::from_bytes([0x11; 32]),
            sequence,
            PreviousAuthorityHash::from_root_anchor(
                super::super::AuthorityRecordAnchor::from_bytes([0x22; 32]),
            ),
        )
    }

    fn moment_at(time: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(time), Microstep::new(microstep))
    }

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            output.push(char::from(DIGITS[usize::from(byte >> 4)]));
            output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        output
    }

    fn command_scheduler_key(
        prepared: &PreparedScheduledCommand,
        captured: CapturedInputRecordId,
    ) -> SchedulerKey {
        let plan = crate::scheduler::SchedulerState::empty()
            .plan_batch(vec![crate::scheduler::SchedulerInsertion::new(
                crate::scheduler::SchedulerProducerOrdinal::new(0),
                crate::scheduler::ScheduledWork::command(prepared.clone().materialize(captured)),
            )])
            .unwrap_or_else(|error| panic!("command fixture must plan: {error:?}"));
        plan.entries()
            .first()
            .map(|(key, _)| *key)
            .unwrap_or_else(|| panic!("command fixture plan must contain one entry"))
    }

    fn command_admission_body() -> NormalizedAuthorityRecordBody {
        let namespace = ExternalInputNamespaceId::from_bytes([0x33; 32]);
        let request = AdmitRequest::new(
            InputId::new(7),
            moment_at(9, 2),
            crate::kernel::fixtures::command(0x44, 8),
        );
        let prepared = PreparedScheduledCommand::prepare(namespace, &request);
        let scheduler_key =
            command_scheduler_key(&prepared, CapturedInputRecordId::from_bytes([0x34; 32]));
        NormalizedAuthorityRecordBody::Admission(NormalizedAuthorityAdmission::Commands(vec![
            NormalizedIngressRecord {
                prepared,
                scheduler_key,
            },
        ]))
    }

    struct DeferredMomentFixture {
        control: DeferredActionControlV1,
        open: ActionOpportunity,
        waiting: ActionOpportunity,
        invocation: ActionEvaluationInvocationRecord,
    }

    fn deferred_moment_fixture() -> DeferredMomentFixture {
        let control = DeferredActionControlV1::enabled(
            DeferredActionAdmissionModeV1::HostScheduled,
            1,
            8,
            8,
            8,
            8,
        )
        .unwrap_or_else(|error| panic!("deferred control fixture must be valid: {error}"));
        let actor = ActorId::from_bytes([0xa1; 32]);
        let scope = ContainmentInteractionScope::new(
            EntityId::from_bytes([0xa2; 32]),
            vec![EntityId::from_bytes([0xa3; 32])],
            vec![EntityId::from_bytes([0xa4; 32])],
            4,
        )
        .unwrap_or_else(|error| panic!("deferred scope fixture must be valid: {error}"));
        let open = ActionOpportunity::open(
            actor,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0xa5; 32])),
            ActionInteractionScope::containment(scope),
            ActionOpportunityGeneration::new(7),
        );
        let policy_semantics = [0xa6; 32];
        let input_fingerprint = [0xa7; 32];
        let (waiting, invocation) = open
            .begin_evaluation(open.version(), policy_semantics, input_fingerprint)
            .unwrap_or_else(|error| panic!("deferred fixture must begin evaluation: {error}"));
        let request = ActionEvaluationRequestArtifact::new(
            ActionEvaluationArtifactSchemaId::from_bytes([0xa8; 32]),
            vec![0xa9],
            control,
        )
        .unwrap_or_else(|error| panic!("request artifact fixture must be valid: {error}"));
        let continuation = ActionEvaluationPrivateContinuationArtifact::new(
            ActionEvaluationArtifactSchemaId::from_bytes([0xaa; 32]),
            vec![0xab],
            control,
        )
        .unwrap_or_else(|error| panic!("continuation artifact fixture must be valid: {error}"));
        let witness = ActionEvaluationPrivateReadWitnessArtifact::new(
            ActionEvaluationArtifactSchemaId::from_bytes([0xac; 32]),
            vec![0xad],
            control,
        )
        .unwrap_or_else(|error| panic!("witness artifact fixture must be valid: {error}"));
        let creation = moment_at(14, 2);
        let source_cursor = AuthorityCursor::root(
            EpochIdentity::new(
                EpochLineageId::from_bytes([0xae; 32]),
                ExecutionSpecId::from_bytes([0xaf; 32]),
            ),
            InitialStateRootId::from_bytes([0xb0; 32]),
        );
        let invocation = ActionEvaluationInvocationRecord::dispatch_pending(
            invocation,
            open.id(),
            open.version(),
            waiting.version(),
            waiting.evaluation_generation(),
            policy_semantics,
            input_fingerprint,
            LifecycleImplementationId::from_bytes([0xb1; 32]),
            request,
            ActionEvaluationArtifactSchemaId::from_bytes([0xb2; 32]),
            continuation,
            witness,
            creation,
            source_cursor,
            None,
            control,
        )
        .unwrap_or_else(|error| panic!("deferred invocation fixture must be valid: {error:?}"));
        DeferredMomentFixture {
            control,
            open,
            waiting,
            invocation,
        }
    }

    fn action_ready_opening_body(fixture: &DeferredMomentFixture) -> NormalizedAuthorityRecordBody {
        let due = fixture.invocation.creation_moment();
        let scheduler_key = SchedulerKey::new(
            due,
            SchedulerLaneV2::ActionReady,
            SchedulerSequence::new(23),
        );
        let ready = ActionReady::new(fixture.open.id(), fixture.open.version(), due);
        let delivery = ActionReadyDeliveryRecord::new(scheduler_key, ready);
        let delivery_ref = ActionReadyDeliveryRef::from_position(0)
            .unwrap_or_else(|| panic!("zero is a valid action-ready delivery position"));
        NormalizedAuthorityRecordBody::Moment(Box::new(NormalizedMomentBatch {
            moment: due,
            resulting_frontier: due,
            consumed_keys: vec![scheduler_key],
            command_deliveries: Vec::new(),
            post_commit_deliveries: Vec::new(),
            lifecycle_deliveries: Vec::new(),
            action_ready_deliveries: vec![delivery],
            action_evaluation_deliveries: Vec::new(),
            attempt_resolved_deliveries: Vec::new(),
            relocation_process_deliveries: Vec::new(),
            action_opportunity_transitions: vec![ActionOpportunityTransitionRecord::new(
                fixture.open.clone(),
                fixture.waiting.clone(),
            )],
            action_evaluation_invocation_openings: vec![
                ActionEvaluationInvocationOpeningRecord::new(
                    ActionEvaluationInvocationOpeningCause::ActionReady(delivery_ref),
                    fixture.invocation.clone(),
                ),
            ],
            action_evaluation_invocation_transitions: Vec::new(),
            action_opportunity_openings: Vec::new(),
            evidence_routing: Vec::new(),
            evidence_assimilations: Vec::new(),
            appraisal_transitions: Vec::new(),
            intent_adoptions: Vec::new(),
            intent_transitions: Vec::new(),
            activity_starts: Vec::new(),
            activity_transitions: Vec::new(),
            activity_terminal_transitions: Vec::new(),
            lifecycle_control_mutations: Vec::new(),
            relocation_attempts: Vec::new(),
            relocation_process_transitions: Vec::new(),
            attempts: Vec::new(),
            commits: Vec::new(),
            containment_delta: Vec::new(),
            reactions: Vec::new(),
            scheduler_insertions: Vec::new(),
            resolutions: vec![NormalizedDeliveryResolution::ActionReadyConsumed {
                delivery: delivery_ref,
            }],
            resolution_evidence: empty_resolution_evidence(due),
        }))
    }

    fn action_evaluation_result_body(
        fixture: &DeferredMomentFixture,
    ) -> NormalizedAuthorityRecordBody {
        let effective = moment_at(14, 4);
        let invocation = fixture.invocation.invocation();
        let scheduler_key = SchedulerKey::new(
            effective,
            SchedulerLaneV2::ActionEvaluation,
            SchedulerSequence::new(24),
        );
        let capture = ActionEvaluationCaptureId::new(25);
        let result_schema = fixture
            .invocation
            .result_schema()
            .unwrap_or_else(|| panic!("dispatch fixture must retain a result schema"));
        let request = ActionEvaluationCaptureRequest::resolve(
            ActionEvaluationResultSubmission::host_scheduled(
                capture,
                invocation,
                effective,
                result_schema,
                vec![0xb3],
            ),
            &fixture.invocation,
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("result capture fixture must resolve: {error:?}"));
        let (result, artifact) = match request.payload() {
            ActionEvaluationCapturePayload::Result { result, artifact } => {
                (*result, artifact.clone())
            }
            ActionEvaluationCapturePayload::ArtifactRejected { .. } => {
                panic!("bounded result fixture must retain its exact artifact")
            }
        };
        let mut invocations = ActionEvaluationInvocationLedger::default();
        invocations
            .install_dispatch(fixture.invocation.clone(), &fixture.waiting)
            .unwrap_or_else(|error| panic!("dispatch fixture must install: {error:?}"));
        let captured = invocations
            .capture_result(
                invocation,
                fixture.waiting.version(),
                capture,
                request.fingerprint(),
                artifact,
                effective,
                scheduler_key,
                fixture.control,
            )
            .unwrap_or_else(|error| panic!("capture fixture must transition: {error:?}"))
            .clone();
        let expected_before = captured.digest();
        let terminal = invocations
            .finish_applied(
                invocation,
                fixture.waiting.version(),
                result,
                ActionEvaluationResultFreshness::ProjectionRebound,
            )
            .unwrap_or_else(|error| panic!("captured result fixture must terminalize: {error:?}"))
            .clone();
        let resumed = fixture
            .waiting
            .resume_evaluation(fixture.waiting.version(), invocation)
            .unwrap_or_else(|error| panic!("waiting fixture must reopen: {error}"));
        let consumed = resumed
            .consume(
                resumed.version(),
                ActionOpportunityDisposition::NoApplicableAction,
            )
            .unwrap_or_else(|error| panic!("resumed fixture must be consumable: {error}"));
        let work = ActionEvaluationWork::result_ready(
            invocation,
            fixture.waiting.id(),
            fixture.waiting.version(),
            effective,
        );
        let delivery = ActionEvaluationDeliveryRecord::new(scheduler_key, work);
        let delivery_ref = ActionEvaluationDeliveryRef::from_position(0)
            .unwrap_or_else(|| panic!("zero is a valid action-evaluation delivery position"));
        NormalizedAuthorityRecordBody::Moment(Box::new(NormalizedMomentBatch {
            moment: effective,
            resulting_frontier: effective,
            consumed_keys: vec![scheduler_key],
            command_deliveries: Vec::new(),
            post_commit_deliveries: Vec::new(),
            lifecycle_deliveries: Vec::new(),
            action_ready_deliveries: Vec::new(),
            action_evaluation_deliveries: vec![delivery],
            attempt_resolved_deliveries: Vec::new(),
            relocation_process_deliveries: Vec::new(),
            action_opportunity_transitions: vec![
                ActionOpportunityTransitionRecord::new(fixture.waiting.clone(), resumed.clone()),
                ActionOpportunityTransitionRecord::new(resumed, consumed),
            ],
            action_evaluation_invocation_openings: Vec::new(),
            action_evaluation_invocation_transitions: vec![
                ActionEvaluationInvocationTransitionRecord::new(
                    ActionEvaluationInvocationTransitionCause::EvaluationDelivery(delivery_ref),
                    expected_before,
                    terminal,
                ),
            ],
            action_opportunity_openings: Vec::new(),
            evidence_routing: Vec::new(),
            evidence_assimilations: Vec::new(),
            appraisal_transitions: Vec::new(),
            intent_adoptions: Vec::new(),
            intent_transitions: Vec::new(),
            activity_starts: Vec::new(),
            activity_transitions: Vec::new(),
            activity_terminal_transitions: Vec::new(),
            lifecycle_control_mutations: Vec::new(),
            relocation_attempts: Vec::new(),
            relocation_process_transitions: Vec::new(),
            attempts: Vec::new(),
            commits: Vec::new(),
            containment_delta: Vec::new(),
            reactions: Vec::new(),
            scheduler_insertions: Vec::new(),
            resolutions: vec![NormalizedDeliveryResolution::ActionEvaluationConsumed {
                delivery: delivery_ref,
            }],
            resolution_evidence: empty_resolution_evidence(effective),
        }))
    }

    fn action_evaluation_admission_body(
        fixture: &DeferredMomentFixture,
    ) -> NormalizedAuthorityRecordBody {
        let effective = moment_at(14, 4);
        let invocation = fixture.invocation.invocation();
        let scheduler_key = SchedulerKey::new(
            effective,
            SchedulerLaneV2::ActionEvaluation,
            SchedulerSequence::new(24),
        );
        let capture = ActionEvaluationCaptureId::new(25);
        let request = ActionEvaluationCaptureRequest::resolve(
            ActionEvaluationResultSubmission::host_scheduled(
                capture,
                invocation,
                effective,
                fixture
                    .invocation
                    .result_schema()
                    .unwrap_or_else(|| panic!("dispatch fixture must retain a result schema")),
                vec![0xb3],
            ),
            &fixture.invocation,
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("result admission fixture must resolve: {error:?}"));
        let artifact = match request.payload() {
            ActionEvaluationCapturePayload::Result { artifact, .. } => artifact.clone(),
            ActionEvaluationCapturePayload::ArtifactRejected { .. } => {
                panic!("bounded result admission must retain its exact artifact")
            }
        };
        let work = ActionEvaluationWork::result_ready(
            invocation,
            fixture.waiting.id(),
            fixture.waiting.version(),
            effective,
        );
        let mut invocations = ActionEvaluationInvocationLedger::default();
        invocations
            .install_dispatch(fixture.invocation.clone(), &fixture.waiting)
            .unwrap_or_else(|error| panic!("dispatch admission fixture must install: {error:?}"));
        let after = invocations
            .capture_result(
                invocation,
                fixture.waiting.version(),
                capture,
                request.fingerprint(),
                artifact,
                effective,
                scheduler_key,
                fixture.control,
            )
            .unwrap_or_else(|error| panic!("result admission fixture must capture: {error:?}"))
            .clone();
        NormalizedAuthorityRecordBody::Admission(NormalizedAuthorityAdmission::ActionEvaluation(
            Box::new(NormalizedActionEvaluationAdmission {
                request,
                transition: ActionEvaluationInvocationTransitionRecord::new(
                    ActionEvaluationInvocationTransitionCause::ResultCapture(capture),
                    fixture.invocation.digest(),
                    after,
                ),
                scheduler_key,
                work: ScheduledWork::action_evaluation(work),
            }),
        ))
    }

    fn action_evaluation_management_body(
        fixture: &DeferredMomentFixture,
    ) -> NormalizedAuthorityRecordBody {
        let request = ManageRequest::new(
            ManagementRequestId::new(26),
            SessionManagement::ResolveActionEvaluation {
                invocation: fixture.invocation.invocation(),
                disposition: ActionEvaluationManagementDisposition::HostFailure,
            },
        );
        let request_id = request.id();
        let cause = ActionEvaluationFallbackCause::HostFailure;
        let due = moment_at(14, 3);
        let insertion_key = SchedulerKey::new(
            due,
            SchedulerLaneV2::ActionEvaluation,
            SchedulerSequence::new(25),
        );
        let insertion_work = ScheduledWork::action_evaluation(ActionEvaluationWork::fallback(
            fixture.invocation.invocation(),
            fixture.waiting.id(),
            fixture.waiting.version(),
            cause,
            due,
        ));
        let mut invocations = ActionEvaluationInvocationLedger::default();
        invocations
            .install_dispatch(fixture.invocation.clone(), &fixture.waiting)
            .unwrap_or_else(|error| panic!("dispatch management fixture must install: {error:?}"));
        let after = invocations
            .begin_managed_fallback(
                fixture.invocation.invocation(),
                fixture.waiting.version(),
                cause,
                insertion_key,
            )
            .unwrap_or_else(|error| {
                panic!("action-evaluation management fixture must transition: {error:?}")
            })
            .clone();
        NormalizedAuthorityRecordBody::Management {
            cause: Box::new(NormalizedManagementCause::HostRequests(vec![
                NormalizedManagementRecord {
                    request,
                    resulting_mode: SessionMode::Running,
                    action_evaluation: Some(NormalizedActionEvaluationManagement {
                        transition: ActionEvaluationInvocationTransitionRecord::new(
                            ActionEvaluationInvocationTransitionCause::Management(request_id),
                            fixture.invocation.digest(),
                            after,
                        ),
                        removed: None,
                        insertion_key,
                        insertion_work,
                    }),
                },
            ])),
            resulting_mode: SessionMode::Running,
            preserved_frontier: fixture.invocation.creation_moment(),
        }
    }

    fn empty_resolution_evidence(moment: SimMoment) -> ContainmentResolutionEvidence {
        let domain = DomainState::new(Vec::new(), Vec::new(), Vec::new())
            .unwrap_or_else(|error| panic!("empty domain state must be valid: {error}"));
        let base = AcceptedState::new(
            domain,
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );
        let candidates = ContainmentCandidateSet::new(Vec::new())
            .unwrap_or_else(|error| panic!("empty candidate set must be valid: {error:?}"));
        let oracle = Blake3KeyedPrf256V1::from_root_seed(RootSeed::from_bytes([0x91; 32]));
        resolve_containment_candidates(moment, &base, &candidates, &oracle)
            .evidence()
            .clone()
    }

    fn rejected_body() -> NormalizedAuthorityRecordBody {
        let namespace = ExternalInputNamespaceId::from_bytes([0x55; 32]);
        let request = AdmitRequest::new(
            InputId::new(10),
            moment_at(12, 3),
            crate::kernel::fixtures::command(0x66, 11),
        );
        let prepared = PreparedScheduledCommand::prepare(namespace, &request);
        let captured = CapturedInputRecordId::from_bytes([0x77; 32]);
        let scheduler_key = command_scheduler_key(&prepared, captured);
        let scheduled = prepared.materialize(captured);
        let delivery = CommandDeliveryRecord::new(scheduler_key, &scheduled);
        NormalizedAuthorityRecordBody::Moment(Box::new(NormalizedMomentBatch {
            moment: request.effective(),
            resulting_frontier: request.effective(),
            consumed_keys: vec![scheduler_key],
            command_deliveries: vec![delivery],
            post_commit_deliveries: Vec::new(),
            lifecycle_deliveries: Vec::new(),
            action_ready_deliveries: Vec::new(),
            action_evaluation_deliveries: Vec::new(),
            attempt_resolved_deliveries: Vec::new(),
            relocation_process_deliveries: Vec::new(),
            action_opportunity_transitions: Vec::new(),
            action_evaluation_invocation_openings: Vec::new(),
            action_evaluation_invocation_transitions: Vec::new(),
            action_opportunity_openings: Vec::new(),
            evidence_routing: Vec::new(),
            evidence_assimilations: Vec::new(),
            appraisal_transitions: Vec::new(),
            intent_adoptions: Vec::new(),
            intent_transitions: Vec::new(),
            activity_starts: Vec::new(),
            activity_transitions: Vec::new(),
            activity_terminal_transitions: Vec::new(),
            lifecycle_control_mutations: Vec::new(),
            relocation_attempts: Vec::new(),
            relocation_process_transitions: Vec::new(),
            attempts: vec![NormalizedAttemptRecord {
                subject: NormalizedAttemptSubject::EvaluatedCommand(scheduled.command().clone()),
                resolution: NormalizedAttemptResolution::Rejected(
                    StableCommandRejection::RequirementUnsatisfied,
                ),
            }],
            commits: Vec::new(),
            containment_delta: Vec::new(),
            reactions: Vec::new(),
            scheduler_insertions: Vec::new(),
            resolutions: vec![NormalizedDeliveryResolution::NewCommand {
                delivery: CommandDeliveryRef::from_position(0)
                    .unwrap_or_else(|| panic!("zero is a valid delivery index")),
                attempt: AttemptRecordRef::from_position(0)
                    .unwrap_or_else(|| panic!("zero is a valid attempt index")),
            }],
            resolution_evidence: empty_resolution_evidence(request.effective()),
        }))
    }

    fn vector(body: &NormalizedAuthorityRecordBody) -> (String, String) {
        let (lineage, sequence, previous) = fixture_context();
        let bytes = authority_record_preimage(lineage, sequence, previous, body);
        (
            hex(bytes.as_bytes()),
            ContentDigest::of_canonical(&bytes).to_string(),
        )
    }

    #[test]
    fn authority_record_family_preimages_are_byte_complete() {
        let management = NormalizedAuthorityRecordBody::Management {
            cause: Box::new(NormalizedManagementCause::HostRequests(vec![
                NormalizedManagementRecord {
                    request: ManageRequest::new(
                        ManagementRequestId::new(13),
                        SessionManagement::Pause,
                    ),
                    resulting_mode: SessionMode::Paused,
                    action_evaluation: None,
                },
            ])),
            resulting_mode: SessionMode::Paused,
            preserved_frontier: SimMoment::ORIGIN,
        };
        let vectors = [
            vector(&command_admission_body()),
            vector(&management),
            vector(&rejected_body()),
        ];
        assert_eq!(
            vectors,
            [
                (
                    "776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d7265636f72642d7633000300000000000000201111111111111111111111111111111111111111111111111111111111111111000000000000000500000000000000202222222222222222222222222222222222222222222222222222222222222222000000000000000000000000000000010000000000000000000000070000000000000020bd3d2c6fe667bc8f63b5e6b12b9961181141a702ecce3da33027c8ee27e9037a000000000000000900000000000000020000000000000020444444444444444444444444444444444444444444444444444444444444444400000000000000080000000000000020bcb314da4bb5f26ff0d8c2b1f2eacfd2cf3ee35eba1fc791ce873a74f2025c0d0000000000000009000000000000000200000000000000000000000000000000000000000000000000000020929a0ff3147a62c56e8f9fa2492fa0e93bbc37a8072e2f1f00c604e2b69e7c4100000000000000070000000000000020bd3d2c6fe667bc8f63b5e6b12b9961181141a702ecce3da33027c8ee27e9037a00000000000000000000000000000020929a0ff3147a62c56e8f9fa2492fa0e93bbc37a8072e2f1f00c604e2b69e7c410000000000000000000000090000000000000002"
                        .to_owned(),
                    "091d4efc8b5357ff43789ab84b10eeea94ce88e53eb319d53d7f9e87608a8087"
                        .to_owned(),
                ),
                (
                    "776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d7265636f72642d763300030000000000000020111111111111111111111111111111111111111111111111111111111111111100000000000000050000000000000020222222222222222222222222222222222222222222222222222222222222222200000002000000000000000000000001000000000000000d0000000000000020a12c780c5033e15d75cfd27e28cd9c00a3ed4e31fc5c6bad690f751535a8ee7b000000000000000000000001000000000000000100000000000000000000000000000000".to_owned(),
                    "4c3cec1715d756cc474f9c9b4ab3085eaf09d4a92124fe11fc73086fc5c5178b".to_owned(),
                ),
                (
                    "776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d7265636f72642d763300030000000000000020111111111111111111111111111111111111111111111111111111111111111100000000000000050000000000000020222222222222222222222222222222222222222222222222222222222222222200000001000000000000000c0000000000000003000000000000000c00000000000000030000000000000001000000000000000c00000000000000030000000000000000000000000000000000000001000000000000000c00000000000000030000000000000000000000000000000000000020c93ebfb23c1834ecd4b797a115328b76c8a7223bc1914124a1e2d2a6d2c7838f0000000000000020777777777777777777777777777777777777777777777777777777777777777700000000000000206666666666666666666666666666666666666666666666666666666666666666000000000000000b0000000000000020a7d426a0b75561205b2bf4e33589874ad39d57b567f96967ad2257b74ba64c270000000000000000000000000000000000000000000000010000000000000000000000206666666666666666666666666666666666666666666666666666666666666666000000000000000b0000000000000020a7d426a0b75561205b2bf4e33589874ad39d57b567f96967ad2257b74ba64c27000000010000000300000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000010000000000000001000000000000000000000000000000000000000000000000"
                        .to_owned(),
                    "b5301ebcd035d2d01600a8417bea53e066158979da43bac8110d57d09e8051ab"
                        .to_owned(),
                ),
            ]
        );
    }

    #[test]
    fn m4_action_evaluation_authority_preimages_are_byte_complete() {
        let fixture = deferred_moment_fixture();
        let vectors = [
            vector(&action_evaluation_admission_body(&fixture)),
            vector(&action_evaluation_management_body(&fixture)),
            vector(&action_ready_opening_body(&fixture)),
            vector(&action_evaluation_result_body(&fixture)),
        ];
        assert_eq!(
            vectors,
            [
                (
                    "776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d7265636f72642d7633000300000000000000201111111111111111111111111111111111111111111111111111111111111111000000000000000500000000000000202222222222222222222222222222222222222222222222222222222222222222000000000000000100000000000000190000000000000020dfe72d93ba3e89c1ad824eea08bb8d2120d39db986b971ac90d025a60df385cd00000000000000205b1898c2ca7ac039dbc6ad55abebae348975ffffe009f1c974a46e7b29a9fede000000000000002034e6e6fffb4fdddb154d899d7b3e7a81e0aaeabe767007fedc6a2c2d09f19528000000000000000e0000000000000004000000010000000000000020b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b200000000000000010000000000000020b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2000000010000000000000001b300000000000000206ba6988e2700dd00d40571c48f3975d603407f3c193ab1416c91519ce4a5d1dd0000000000000020576c4d6110fc66a0088307d247abc05b883b2254565687c93b242757dad29fde000000000000000100000000000000190000000000000020bf1ded47c9d63f23d01b97ba5a12dccea7979946f05837456393f95641e142480000000000000020ca3e611dd7e47504b6d49e83c37016b8cf99da2a6e11754476b639c0aa8dd369000000000000000e000000000000000400000005000000000000001800000005000000000000000000000020dfe72d93ba3e89c1ad824eea08bb8d2120d39db986b971ac90d025a60df385cd00000000000000208c402b9e5350320bbbd917723fc3e0dc6f7e059f65bdd09abb846a3dab3973f80000000000000002000000000000000e0000000000000004".to_owned(),
                    "6ce835b112db1d99ed238ab5d3f560230372ac4648faca204f5bd48e658f8e9f".to_owned(),
                ),
                (
                    "776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d7265636f72642d763300030000000000000020111111111111111111111111111111111111111111111111111111111111111100000000000000050000000000000020222222222222222222222222222222222222222222222222222222222222222200000002000000000000000000000001000000000000001a0000000000000020020988fd4ea4f426ecf88e74d80b8f5cbbf4646dbc5c12617e37fbf9e148b3b4000000060000000000000020dfe72d93ba3e89c1ad824eea08bb8d2120d39db986b971ac90d025a60df385cd0000000200000000000000000000000100000002000000000000001a0000000000000020bf1ded47c9d63f23d01b97ba5a12dccea7979946f05837456393f95641e142480000000000000020eb3c51387bc73034104c1a7a93f05c358f9a1fa1cec62e7bbdce41fef0cbbc2800000000000000000000000e000000000000000300000005000000000000001900000005000000010000000000000020dfe72d93ba3e89c1ad824eea08bb8d2120d39db986b971ac90d025a60df385cd00000000000000208c402b9e5350320bbbd917723fc3e0dc6f7e059f65bdd09abb846a3dab3973f80000000000000002000000000000000e00000000000000030000000200000000000000000000000e0000000000000002".to_owned(),
                    "57a4041a20cf110bbebe55276f7cc2bedadb8ef0d0312acea35ea7f5890e2869".to_owned(),
                ),
                (
                    "776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d7265636f72642d763300030000000000000020111111111111111111111111111111111111111111111111111111111111111100000000000000050000000000000020222222222222222222222222222222222222222222222222222222222222222200000001000000000000000e0000000000000002000000000000000e00000000000000020000000000000001000000000000000e000000000000000200000004000000000000001700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000007000000020000000000000001000000000000000000000000000000000000000000000000000000000000000000000001000000000000000e000000000000000200000004000000000000001700000000000000208c402b9e5350320bbbd917723fc3e0dc6f7e059f65bdd09abb846a3dab3973f80000000000000001000000000000000e00000000000000020000000000000000000000000000000000000000000000010000000000000020d972ad2c6a10e7f720d765fc3e68f03929304770c4e769d776846beff13156950000000000000020448095b43406e26802ae6bfbdca3f93b314ca8de5a72d73461e5dae31904756e00000000000000010000000000000002000000000000000000000020bf1ded47c9d63f23d01b97ba5a12dccea7979946f05837456393f95641e1424800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".to_owned(),
                    "7c087038615d3cdfe0914e03dd7653a82a25c502c53213a42414bb13fd07abd9".to_owned(),
                ),
                (
                    "776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d7265636f72642d763300030000000000000020111111111111111111111111111111111111111111111111111111111111111100000000000000050000000000000020222222222222222222222222222222222222222222222222222222222222222200000001000000000000000e0000000000000004000000000000000e00000000000000040000000000000001000000000000000e00000000000000040000000500000000000000180000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000c0000000600000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000e0000000000000004000000050000000000000018000000000000000000000020dfe72d93ba3e89c1ad824eea08bb8d2120d39db986b971ac90d025a60df385cd00000000000000208c402b9e5350320bbbd917723fc3e0dc6f7e059f65bdd09abb846a3dab3973f80000000000000002000000000000000e0000000000000004000000000000000000000000000000020000000000000020448095b43406e26802ae6bfbdca3f93b314ca8de5a72d73461e5dae31904756e0000000000000020a36ee4f53305f0791f316b555c1d7263ee07dc37ee9c8b66c3a02ad4dd1be3290000000000000020a36ee4f53305f0791f316b555c1d7263ee07dc37ee9c8b66c3a02ad4dd1be3290000000000000020b3d1f8721b7402910e3be685eefd176394fd6ad2ebad1d8f9647f2f74454633a000000000000000000000000000000010000000000000006000000000000000000000020ca3e611dd7e47504b6d49e83c37016b8cf99da2a6e11754476b639c0aa8dd3690000000000000020a4924f821e969b5baf0d3d7ac608f8f5931b9e8db9abf939891f905d2c7934f10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".to_owned(),
                    "da784c3ceb618a76e1115cf007986f099bc8a595e1fb10397b66cf4a7cce56f7".to_owned(),
                ),
            ]
        );
    }

    #[test]
    fn cumulative_preimage_is_byte_complete() {
        let bytes = cumulative_authority_preimage(
            CumulativeAuthorityHash::from_bytes([0x88; 32]),
            AuthorityRecordId::from_bytes([0x99; 32]),
        );
        assert_eq!(
            hex(bytes.as_bytes()),
            "776f726c642d63616e6f6e6963616c2d7631000000000000001763756d756c61746976652d617574686f726974792d763100010000000000000020888888888888888888888888888888888888888888888888888888888888888800000000000000209999999999999999999999999999999999999999999999999999999999999999"
        );
        assert_eq!(
            ContentDigest::of_canonical(&bytes).to_string(),
            "9d8317c1becd6d0dec41a4fccd7e92258999aa26c7c2cbec4095655a317f9de2"
        );
    }
}
