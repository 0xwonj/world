use core::fmt;
use std::sync::Arc;

use world_context::{
    ActionContextPayload, ActionContextPayloadSchemaId, ActionInputFingerprint,
    ActionPolicySemanticsId, ActionReadWitness, ActionReadWitnessSchemaId,
    CandidateResolutionTable, CandidateResolutionTableSchemaId, ContainmentProjectionError,
    ContainmentTransferActionDefinitions, ContainmentTransferProjector, GroundedActionCandidateId,
    RelocationActionDefinitions, RelocationProjectionError, RelocationProjector,
    ResolvedActionSelection, ResolvedRelocationAction, action_context_payload_schema,
    action_read_witness_schema, candidate_resolution_table_schema, decode_action_context_payload,
    decode_action_read_witness, decode_candidate_resolution_table, encode_action_context_payload,
    encode_action_read_witness, encode_candidate_resolution_table,
};
use world_decision::{
    ActionDecision, ActionDecisionSchemaId, ActionPolicy, ActionPolicyError,
    action_decision_schema, decode_action_decision, encode_action_decision,
};
use world_defs::{BindingName, RuntimeDefinitionSet};
use world_model::{
    ActionEvaluationInvocationId, ActionInteractionScope, ActionOpportunity,
    ActionOpportunityDisposition, ActionOpportunityId, ActionOpportunityState,
    ActionOpportunityVersion, CommandBinding, CommandEnvelope, CommandEnvelopeError, CommandId,
    CommandSource, CommandValue, WorldSnapshot,
};
use world_runtime::{
    ActionEvaluationArtifactSchemaId, ActionEvaluationCaptureId, ActionEvaluationCaptureTiming,
    ActionEvaluationDecision, ActionEvaluationInvocationRecord, ActionEvaluationInvocationState,
    ActionEvaluationRequestId, ActionEvaluationResultFailure, ActionEvaluationResultFreshness,
    ActionEvaluationResultReady, ActionEvaluationResultSubmission, ActionPolicyExecutionV1,
    DeferredActionAdmissionModeV1, DeferredActionArtifactInput, DeferredActionInvocationInput,
    EvaluatedAction, LifecycleImplementationId, PendingActionEvaluationRaw,
};

/// One dispatchable deferred action request decoded at the engine boundary.
///
/// The view intentionally contains no authority cursor, revision, blocker,
/// private candidate resolution, or read witness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingActionEvaluation {
    invocation: ActionEvaluationInvocationId,
    request: ActionEvaluationRequestId,
    implementation: LifecycleImplementationId,
    request_schema: ActionContextPayloadSchemaId,
    result_schema: ActionDecisionSchemaId,
    payload: ActionContextPayload,
    admission_mode: DeferredActionAdmissionModeV1,
}

impl PendingActionEvaluation {
    pub(crate) fn decode(raw: PendingActionEvaluationRaw) -> Option<Self> {
        let request_schema =
            ActionContextPayloadSchemaId::from_bytes(raw.request_artifact().schema().into_bytes());
        let result_schema = ActionDecisionSchemaId::from_bytes(raw.result_schema().into_bytes());
        if request_schema != action_context_payload_schema()
            || result_schema != action_decision_schema()
        {
            return None;
        }
        let payload = decode_action_context_payload(raw.request_artifact().bytes()).ok()?;
        Some(Self {
            invocation: raw.invocation(),
            request: raw.request(),
            implementation: raw.implementation(),
            request_schema,
            result_schema,
            payload,
            admission_mode: raw.admission_mode(),
        })
    }

    /// Returns the logical evaluator invocation.
    #[must_use]
    pub const fn invocation(&self) -> ActionEvaluationInvocationId {
        self.invocation
    }

    /// Returns the exact retained request identity.
    #[must_use]
    pub const fn request(&self) -> ActionEvaluationRequestId {
        self.request
    }

    /// Returns the selected evaluator implementation.
    #[must_use]
    pub const fn implementation(&self) -> LifecycleImplementationId {
        self.implementation
    }

    /// Returns the owner-typed request codec identity.
    #[must_use]
    pub const fn request_schema(&self) -> ActionContextPayloadSchemaId {
        self.request_schema
    }

    /// Returns the owner-typed decision codec identity.
    #[must_use]
    pub const fn result_schema(&self) -> ActionDecisionSchemaId {
        self.result_schema
    }

    /// Returns the complete actor-safe policy input.
    #[must_use]
    pub const fn payload(&self) -> &ActionContextPayload {
        &self.payload
    }

    /// Returns how a result must obtain simulation time.
    #[must_use]
    pub const fn admission_mode(&self) -> DeferredActionAdmissionModeV1 {
        self.admission_mode
    }
}

/// One typed action-policy result submitted to a retained invocation.
///
/// Canonical decision bytes and the fixed decision schema are derived only
/// when crossing into runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionEvaluationResultCapture {
    capture: ActionEvaluationCaptureId,
    invocation: ActionEvaluationInvocationId,
    timing: ActionEvaluationCaptureTiming,
    decision: ActionDecision,
}

impl ActionEvaluationResultCapture {
    /// Captures a result at the invocation's retained blocking frontier.
    #[must_use]
    pub const fn at_invocation_frontier(
        capture: ActionEvaluationCaptureId,
        invocation: ActionEvaluationInvocationId,
        decision: ActionDecision,
    ) -> Self {
        Self {
            capture,
            invocation,
            timing: ActionEvaluationCaptureTiming::InvocationFrontier,
            decision,
        }
    }

    /// Captures a result at one explicit nonblocking simulation moment.
    #[must_use]
    pub const fn host_scheduled(
        capture: ActionEvaluationCaptureId,
        invocation: ActionEvaluationInvocationId,
        effective: world_core::SimMoment,
        decision: ActionDecision,
    ) -> Self {
        Self {
            capture,
            invocation,
            timing: ActionEvaluationCaptureTiming::HostScheduled(effective),
            decision,
        }
    }

    /// Returns the capture namespace identity.
    #[must_use]
    pub const fn capture(self) -> ActionEvaluationCaptureId {
        self.capture
    }

    /// Returns the retained invocation being answered.
    #[must_use]
    pub const fn invocation(self) -> ActionEvaluationInvocationId {
        self.invocation
    }

    /// Returns the typed decision being captured.
    #[must_use]
    pub const fn decision(self) -> ActionDecision {
        self.decision
    }

    pub(crate) fn into_submission(self) -> ActionEvaluationResultSubmission {
        let schema =
            ActionEvaluationArtifactSchemaId::from_bytes(action_decision_schema().into_bytes());
        let bytes = encode_action_decision(self.decision);
        match self.timing {
            ActionEvaluationCaptureTiming::InvocationFrontier => {
                ActionEvaluationResultSubmission::at_invocation_frontier(
                    self.capture,
                    self.invocation,
                    schema,
                    bytes,
                )
            }
            ActionEvaluationCaptureTiming::HostScheduled(effective) => {
                ActionEvaluationResultSubmission::host_scheduled(
                    self.capture,
                    self.invocation,
                    effective,
                    schema,
                    bytes,
                )
            }
        }
    }
}

/// Complete typed protocol expected by one deferred action evaluator.
///
/// The descriptor carries no callback, transport, or endpoint. It records
/// only the behavior identity and the exact owner-local artifact schemas that
/// a later capture/ingress adapter must preserve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeferredActionEvaluatorDescriptor {
    semantics: ActionPolicySemanticsId,
    request_payload_schema: ActionContextPayloadSchemaId,
    decision_result_schema: ActionDecisionSchemaId,
    candidate_table_continuation_schema: CandidateResolutionTableSchemaId,
    read_witness_schema: ActionReadWitnessSchemaId,
}

impl DeferredActionEvaluatorDescriptor {
    /// Describes a deferred evaluator using this engine's exact action
    /// artifact schemas.
    #[must_use]
    pub fn new(semantics: ActionPolicySemanticsId) -> Self {
        Self {
            semantics,
            request_payload_schema: action_context_payload_schema(),
            decision_result_schema: action_decision_schema(),
            candidate_table_continuation_schema: candidate_resolution_table_schema(),
            read_witness_schema: action_read_witness_schema(),
        }
    }

    /// Returns the evaluator's behavior identity.
    #[must_use]
    pub const fn semantics_id(self) -> ActionPolicySemanticsId {
        self.semantics
    }

    /// Returns the actor-safe request payload schema.
    #[must_use]
    pub const fn request_payload_schema(self) -> ActionContextPayloadSchemaId {
        self.request_payload_schema
    }

    /// Returns the closed action-decision result schema.
    #[must_use]
    pub const fn decision_result_schema(self) -> ActionDecisionSchemaId {
        self.decision_result_schema
    }

    /// Returns the private candidate-table continuation schema.
    #[must_use]
    pub const fn candidate_table_continuation_schema(self) -> CandidateResolutionTableSchemaId {
        self.candidate_table_continuation_schema
    }

    /// Returns the combined private read-witness schema.
    #[must_use]
    pub const fn read_witness_schema(self) -> ActionReadWitnessSchemaId {
        self.read_witness_schema
    }
}

/// One policy implementation and immutable semantics identity selected for
/// a resolved execution.
#[derive(Clone)]
pub(crate) struct InstalledActionPolicy {
    semantics: ActionPolicySemanticsId,
    implementation: Arc<dyn ActionPolicy>,
}

impl InstalledActionPolicy {
    pub(crate) fn new(implementation: Arc<dyn ActionPolicy>) -> Self {
        let semantics = implementation.semantics_id();
        Self {
            semantics,
            implementation,
        }
    }

    pub(crate) const fn semantics_id(&self) -> ActionPolicySemanticsId {
        self.semantics
    }

    fn decide(&self, input: &ActionContextPayload) -> Result<ActionDecision, ActionPolicyError> {
        self.implementation.decide(input)
    }
}

/// Closed installed execution chosen for the action-policy lifecycle port.
#[derive(Clone)]
pub(crate) enum InstalledActionExecution {
    Inline(InstalledActionPolicy),
    Deferred(DeferredActionEvaluatorDescriptor),
}

impl InstalledActionExecution {
    pub(crate) fn inline(implementation: Arc<dyn ActionPolicy>) -> Self {
        Self::Inline(InstalledActionPolicy::new(implementation))
    }

    pub(crate) const fn deferred(descriptor: DeferredActionEvaluatorDescriptor) -> Self {
        Self::Deferred(descriptor)
    }

    pub(crate) const fn execution_class(&self) -> ActionPolicyExecutionV1 {
        match self {
            Self::Inline(_) => ActionPolicyExecutionV1::InlineDeterministic,
            Self::Deferred(_) => ActionPolicyExecutionV1::DeferredCaptured,
        }
    }

    pub(crate) const fn semantics_id(&self) -> ActionPolicySemanticsId {
        match self {
            Self::Inline(policy) => policy.semantics_id(),
            Self::Deferred(descriptor) => descriptor.semantics_id(),
        }
    }

    pub(crate) fn request_payload_schema(&self) -> ActionContextPayloadSchemaId {
        match self {
            Self::Inline(_) => action_context_payload_schema(),
            Self::Deferred(descriptor) => descriptor.request_payload_schema(),
        }
    }

    pub(crate) fn decision_result_schema(&self) -> ActionDecisionSchemaId {
        match self {
            Self::Inline(_) => action_decision_schema(),
            Self::Deferred(descriptor) => descriptor.decision_result_schema(),
        }
    }

    pub(crate) fn candidate_table_continuation_schema(&self) -> CandidateResolutionTableSchemaId {
        match self {
            Self::Inline(_) => candidate_resolution_table_schema(),
            Self::Deferred(descriptor) => descriptor.candidate_table_continuation_schema(),
        }
    }

    pub(crate) fn read_witness_schema(&self) -> ActionReadWitnessSchemaId {
        match self {
            Self::Inline(_) => action_read_witness_schema(),
            Self::Deferred(descriptor) => descriptor.read_witness_schema(),
        }
    }

    pub(crate) const fn inline_policy(&self) -> Option<&InstalledActionPolicy> {
        match self {
            Self::Inline(policy) => Some(policy),
            Self::Deferred(_) => None,
        }
    }

    pub(crate) const fn deferred_descriptor(&self) -> Option<DeferredActionEvaluatorDescriptor> {
        match self {
            Self::Inline(_) => None,
            Self::Deferred(descriptor) => Some(*descriptor),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ActionCoordinationError {
    OpportunityConsumed {
        disposition: ActionOpportunityDisposition,
    },
    ContainmentProjection(ContainmentProjectionError),
    RelocationProjection(RelocationProjectionError),
    ContainmentDefinitionsUnbound,
    RelocationDefinitionsUnbound,
    InputFingerprintMismatch {
        expected: ActionInputFingerprint,
        actual: ActionInputFingerprint,
    },
    CandidateUnavailable {
        candidate: GroundedActionCandidateId,
    },
    CandidateOpportunityMismatch {
        expected: ActionOpportunityId,
        actual: ActionOpportunityId,
    },
    PrivateResolutionMissing {
        candidate: GroundedActionCandidateId,
    },
    Command(CommandEnvelopeError),
}

impl fmt::Display for ActionCoordinationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpportunityConsumed { disposition } => {
                write!(
                    formatter,
                    "action opportunity is consumed as {disposition:?}"
                )
            }
            Self::ContainmentProjection(error) => error.fmt(formatter),
            Self::RelocationProjection(error) => error.fmt(formatter),
            Self::ContainmentDefinitionsUnbound => {
                formatter.write_str("containment-transfer action definitions are not bound")
            }
            Self::RelocationDefinitionsUnbound => {
                formatter.write_str("relocation action definitions are not bound")
            }
            Self::InputFingerprintMismatch { expected, actual } => write!(
                formatter,
                "action decision input {actual} does not match prepared input {expected}"
            ),
            Self::CandidateUnavailable { candidate } => {
                write!(formatter, "action candidate {candidate} was not supplied")
            }
            Self::CandidateOpportunityMismatch { expected, actual } => write!(
                formatter,
                "candidate opportunity {actual} does not match prepared opportunity {expected}"
            ),
            Self::PrivateResolutionMissing { candidate } => {
                write!(formatter, "candidate {candidate} has no private resolution")
            }
            Self::Command(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ActionCoordinationError {}

impl From<ContainmentProjectionError> for ActionCoordinationError {
    fn from(error: ContainmentProjectionError) -> Self {
        Self::ContainmentProjection(error)
    }
}

impl From<RelocationProjectionError> for ActionCoordinationError {
    fn from(error: RelocationProjectionError) -> Self {
        Self::RelocationProjection(error)
    }
}

impl From<CommandEnvelopeError> for ActionCoordinationError {
    fn from(error: CommandEnvelopeError) -> Self {
        Self::Command(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CoordinatedActionSubmission {
    Command(CommandEnvelope),
    Relocation(ResolvedRelocationAction),
}

/// Crate-internal product consumed by action-ready runtime integration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CoordinatedAction {
    opportunity: ActionOpportunityId,
    expected_version: ActionOpportunityVersion,
    disposition: ActionOpportunityDisposition,
    submission: Option<CoordinatedActionSubmission>,
}

impl CoordinatedAction {
    pub(crate) fn into_parts(
        self,
    ) -> (
        ActionOpportunityId,
        ActionOpportunityVersion,
        ActionOpportunityDisposition,
        Option<CoordinatedActionSubmission>,
    ) {
        (
            self.opportunity,
            self.expected_version,
            self.disposition,
            self.submission,
        )
    }
}

/// One complete action projection retained until its decision is interpreted.
///
/// Deferred callers may persist the canonical artifacts, but decision
/// application uses the resolution table of the newly built value on which
/// it is invoked.
pub(crate) struct PreparedAction<'definitions> {
    opportunity: ActionOpportunityId,
    expected_version: ActionOpportunityVersion,
    definitions: &'definitions RuntimeDefinitionSet,
    payload: ActionContextPayload,
    resolution: CandidateResolutionTable,
    read_witness: ActionReadWitness,
}

impl<'definitions> PreparedAction<'definitions> {
    pub(crate) fn build(
        snapshot: &WorldSnapshot,
        opportunity: &ActionOpportunity,
        definitions: &'definitions RuntimeDefinitionSet,
        containment_actions: Option<&ContainmentTransferActionDefinitions>,
        relocation_actions: Option<&RelocationActionDefinitions>,
        policy_semantics: ActionPolicySemanticsId,
    ) -> Result<Self, ActionCoordinationError> {
        if let ActionOpportunityState::Consumed(disposition) = opportunity.state() {
            return Err(ActionCoordinationError::OpportunityConsumed { disposition });
        }
        let build = match opportunity.interaction_scope() {
            ActionInteractionScope::Containment(scope) => {
                let actions = containment_actions
                    .ok_or(ActionCoordinationError::ContainmentDefinitionsUnbound)?;
                ContainmentTransferProjector::new(actions).build(
                    snapshot,
                    opportunity.actor(),
                    opportunity.id(),
                    scope,
                    definitions,
                    policy_semantics,
                )?
            }
            ActionInteractionScope::Relocation(scope) => {
                let actions = relocation_actions
                    .ok_or(ActionCoordinationError::RelocationDefinitionsUnbound)?;
                RelocationProjector::new(actions).build(
                    opportunity.actor(),
                    opportunity.id(),
                    scope,
                    definitions,
                    policy_semantics,
                )?
            }
        };
        let (payload, resolution, read_witness) = build.into_parts();
        Ok(Self {
            opportunity: opportunity.id(),
            expected_version: opportunity.version(),
            definitions,
            payload,
            resolution,
            read_witness,
        })
    }

    pub(crate) const fn read_witness(&self) -> &ActionReadWitness {
        &self.read_witness
    }

    pub(crate) fn encode_request_payload(&self) -> Vec<u8> {
        encode_action_context_payload(&self.payload)
    }

    pub(crate) fn encode_private_continuation(&self) -> Vec<u8> {
        encode_candidate_resolution_table(&self.resolution)
    }

    pub(crate) fn encode_read_witness(&self) -> Vec<u8> {
        encode_action_read_witness(&self.read_witness)
    }

    fn deferred_input(
        &self,
        descriptor: DeferredActionEvaluatorDescriptor,
    ) -> DeferredActionInvocationInput {
        DeferredActionInvocationInput::new(
            descriptor.semantics_id().into_bytes(),
            self.payload.input_fingerprint().into_bytes(),
            DeferredActionArtifactInput::new(
                ActionEvaluationArtifactSchemaId::from_bytes(
                    descriptor.request_payload_schema().into_bytes(),
                ),
                self.encode_request_payload(),
            ),
            ActionEvaluationArtifactSchemaId::from_bytes(
                descriptor.decision_result_schema().into_bytes(),
            ),
            DeferredActionArtifactInput::new(
                ActionEvaluationArtifactSchemaId::from_bytes(
                    descriptor
                        .candidate_table_continuation_schema()
                        .into_bytes(),
                ),
                self.encode_private_continuation(),
            ),
            DeferredActionArtifactInput::new(
                ActionEvaluationArtifactSchemaId::from_bytes(
                    descriptor.read_witness_schema().into_bytes(),
                ),
                self.encode_read_witness(),
            ),
        )
    }

    pub(crate) fn invoke_inline(
        self,
        policy: &InstalledActionPolicy,
    ) -> Result<CoordinatedAction, ActionCoordinationError> {
        match policy.decide(&self.payload) {
            Ok(decision) => self.apply_decision(decision),
            Err(_) => Ok(self.finish(ActionOpportunityDisposition::Failed, None)),
        }
    }

    pub(crate) fn apply_decision(
        self,
        decision: ActionDecision,
    ) -> Result<CoordinatedAction, ActionCoordinationError> {
        let expected_input = self.payload.input_fingerprint();
        let actual_input = decision.input_fingerprint();
        if actual_input != expected_input {
            return Err(ActionCoordinationError::InputFingerprintMismatch {
                expected: expected_input,
                actual: actual_input,
            });
        }

        match decision {
            ActionDecision::Select { candidate, .. } => {
                let supplied = self
                    .payload
                    .candidates()
                    .candidates()
                    .iter()
                    .find(|supplied| supplied.id() == candidate)
                    .ok_or(ActionCoordinationError::CandidateUnavailable { candidate })?;
                if supplied.opportunity() != self.opportunity {
                    return Err(ActionCoordinationError::CandidateOpportunityMismatch {
                        expected: self.opportunity,
                        actual: supplied.opportunity(),
                    });
                }
                let resolution = self
                    .resolution
                    .resolve(candidate)
                    .ok_or(ActionCoordinationError::PrivateResolutionMissing { candidate })?;
                let submission = lower_selection(self.definitions, self.opportunity, resolution)?;
                Ok(self.finish(
                    ActionOpportunityDisposition::ActionSubmitted,
                    Some(submission),
                ))
            }
            ActionDecision::NoApplicableAction { .. } => {
                Ok(self.finish(ActionOpportunityDisposition::NoApplicableAction, None))
            }
        }
    }

    fn finish(
        self,
        disposition: ActionOpportunityDisposition,
        submission: Option<CoordinatedActionSubmission>,
    ) -> CoordinatedAction {
        CoordinatedAction {
            opportunity: self.opportunity,
            expected_version: self.expected_version,
            disposition,
            submission,
        }
    }
}

pub(crate) struct ActionCoordinator;

impl ActionCoordinator {
    pub(crate) fn prepare_deferred(
        snapshot: &WorldSnapshot,
        opportunity: &ActionOpportunity,
        definitions: &RuntimeDefinitionSet,
        containment_actions: Option<&ContainmentTransferActionDefinitions>,
        relocation_actions: Option<&RelocationActionDefinitions>,
        descriptor: DeferredActionEvaluatorDescriptor,
    ) -> Result<DeferredActionInvocationInput, ActionCoordinationError> {
        PreparedAction::build(
            snapshot,
            opportunity,
            definitions,
            containment_actions,
            relocation_actions,
            descriptor.semantics_id(),
        )
        .map(|prepared| prepared.deferred_input(descriptor))
    }

    pub(crate) fn coordinate(
        snapshot: &WorldSnapshot,
        opportunity: &ActionOpportunity,
        definitions: &RuntimeDefinitionSet,
        containment_actions: Option<&ContainmentTransferActionDefinitions>,
        relocation_actions: Option<&RelocationActionDefinitions>,
        policy: &InstalledActionPolicy,
    ) -> Result<CoordinatedAction, ActionCoordinationError> {
        PreparedAction::build(
            snapshot,
            opportunity,
            definitions,
            containment_actions,
            relocation_actions,
            policy.semantics_id(),
        )?
        .invoke_inline(policy)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "fresh result interpretation requires the same complete action projection boundary as initial preparation"
    )]
    pub(crate) fn resolve_deferred(
        snapshot: &WorldSnapshot,
        opportunity: &ActionOpportunity,
        result_ready: ActionEvaluationResultReady,
        invocation: &ActionEvaluationInvocationRecord,
        definitions: &RuntimeDefinitionSet,
        containment_actions: Option<&ContainmentTransferActionDefinitions>,
        relocation_actions: Option<&RelocationActionDefinitions>,
        descriptor: DeferredActionEvaluatorDescriptor,
    ) -> ActionEvaluationDecision {
        match resolve_deferred_action(
            snapshot,
            opportunity,
            result_ready,
            invocation,
            definitions,
            containment_actions,
            relocation_actions,
            descriptor,
        ) {
            Ok(decision) => decision,
            Err(failure) => action_evaluation_fallback(failure),
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "fresh result interpretation validates one complete retained action protocol"
)]
fn resolve_deferred_action(
    snapshot: &WorldSnapshot,
    opportunity: &ActionOpportunity,
    result_ready: ActionEvaluationResultReady,
    invocation: &ActionEvaluationInvocationRecord,
    definitions: &RuntimeDefinitionSet,
    containment_actions: Option<&ContainmentTransferActionDefinitions>,
    relocation_actions: Option<&RelocationActionDefinitions>,
    descriptor: DeferredActionEvaluatorDescriptor,
) -> Result<ActionEvaluationDecision, ActionEvaluationResultFailure> {
    let request = invocation
        .request()
        .ok_or(ActionEvaluationResultFailure::InvalidResult)?;
    let continuation = invocation
        .private_continuation()
        .ok_or(ActionEvaluationResultFailure::InvalidResult)?;
    let read_witness = invocation
        .private_read_witness()
        .ok_or(ActionEvaluationResultFailure::InvalidResult)?;
    let result_schema = invocation
        .result_schema()
        .ok_or(ActionEvaluationResultFailure::InvalidResult)?;
    let ActionEvaluationInvocationState::ResultCaptured {
        artifact: result,
        effective,
        scheduler_key,
        ..
    } = invocation.state()
    else {
        return Err(ActionEvaluationResultFailure::InvalidResult);
    };

    let request_schema = action_evaluation_schema(descriptor.request_payload_schema().into_bytes());
    let expected_result_schema =
        action_evaluation_schema(descriptor.decision_result_schema().into_bytes());
    let continuation_schema = action_evaluation_schema(
        descriptor
            .candidate_table_continuation_schema()
            .into_bytes(),
    );
    let witness_schema = action_evaluation_schema(descriptor.read_witness_schema().into_bytes());
    if result_ready.invocation() != invocation.invocation()
        || result_ready.opportunity() != opportunity.id()
        || result_ready.expected_waiting_version() != opportunity.version()
        || result_ready.due() != *effective
        || scheduler_key.moment() != *effective
        || invocation.opportunity() != opportunity.id()
        || invocation.waiting_version() != opportunity.version()
        || invocation.evaluation_generation() != opportunity.evaluation_generation()
        || opportunity.state()
            != ActionOpportunityState::WaitingForEvaluation(invocation.invocation())
        || request.schema() != request_schema
        || result_schema != expected_result_schema
        || result.schema() != expected_result_schema
        || continuation.schema() != continuation_schema
        || read_witness.schema() != witness_schema
        || invocation.policy_semantics() != descriptor.semantics_id().as_bytes()
    {
        return Err(ActionEvaluationResultFailure::InvalidResult);
    }

    let original_payload = decode_action_context_payload(request.bytes())
        .map_err(|_| ActionEvaluationResultFailure::InvalidResult)?;
    let original_resolution = decode_candidate_resolution_table(continuation.bytes())
        .map_err(|_| ActionEvaluationResultFailure::InvalidResult)?;
    let original_witness = decode_action_read_witness(read_witness.bytes())
        .map_err(|_| ActionEvaluationResultFailure::InvalidResult)?;
    let decision = decode_action_decision(result.bytes())
        .map_err(|_| ActionEvaluationResultFailure::InvalidResult)?;

    if original_payload.actor() != opportunity.actor()
        || original_payload.opportunity() != opportunity.id()
        || original_payload.policy_semantics() != descriptor.semantics_id()
        || original_payload.input_fingerprint().as_bytes() != invocation.action_input_fingerprint()
        || original_resolution.len() != original_payload.candidates().candidates().len()
        || original_payload
            .candidates()
            .candidates()
            .iter()
            .any(|candidate| original_resolution.resolve(candidate.id()).is_none())
    {
        return Err(ActionEvaluationResultFailure::InvalidResult);
    }

    validate_original_decision(
        PreparedAction {
            opportunity: original_payload.opportunity(),
            expected_version: invocation.pre_wait_version(),
            definitions,
            payload: original_payload,
            resolution: original_resolution,
            read_witness: original_witness.clone(),
        },
        decision,
    )?;

    let fresh = PreparedAction::build(
        snapshot,
        opportunity,
        definitions,
        containment_actions,
        relocation_actions,
        descriptor.semantics_id(),
    )
    .map_err(|_| ActionEvaluationResultFailure::InvalidResult)?;
    if request.bytes() != fresh.encode_request_payload() {
        return resolve_visible_change(
            &fresh,
            descriptor,
            invocation.remaining_visible_reinvocations(),
        );
    }

    let freshness = classify_result_freshness(&original_witness, fresh.read_witness());
    let action = evaluated_action(
        fresh
            .apply_decision(decision)
            .map_err(|_| ActionEvaluationResultFailure::InvalidResult)?,
        opportunity.id(),
        opportunity.version(),
    )
    .ok_or(ActionEvaluationResultFailure::InvalidResult)?;
    Ok(ActionEvaluationDecision::Apply { freshness, action })
}

fn resolve_visible_change(
    fresh: &PreparedAction<'_>,
    descriptor: DeferredActionEvaluatorDescriptor,
    remaining_visible_reinvocations: u32,
) -> Result<ActionEvaluationDecision, ActionEvaluationResultFailure> {
    if remaining_visible_reinvocations == 0 {
        return Err(ActionEvaluationResultFailure::VisibleReinvocationExhausted);
    }
    Ok(ActionEvaluationDecision::reinvoke(
        fresh.deferred_input(descriptor),
    ))
}

fn validate_original_decision(
    prepared: PreparedAction<'_>,
    decision: ActionDecision,
) -> Result<(), ActionEvaluationResultFailure> {
    prepared
        .apply_decision(decision)
        .map(drop)
        .map_err(|_| ActionEvaluationResultFailure::InvalidResult)
}

fn action_evaluation_fallback(failure: ActionEvaluationResultFailure) -> ActionEvaluationDecision {
    ActionEvaluationDecision::RequireFallback(failure)
}

fn action_evaluation_schema(bytes: [u8; 32]) -> ActionEvaluationArtifactSchemaId {
    ActionEvaluationArtifactSchemaId::from_bytes(bytes)
}

fn classify_result_freshness(
    original: &ActionReadWitness,
    fresh: &ActionReadWitness,
) -> ActionEvaluationResultFreshness {
    classify_result_freshness_from_matches(
        original.projection() == fresh.projection(),
        original.execution() == fresh.execution(),
    )
}

fn classify_result_freshness_from_matches(
    projection_matches: bool,
    execution_matches: bool,
) -> ActionEvaluationResultFreshness {
    match (projection_matches, execution_matches) {
        (true, true) => ActionEvaluationResultFreshness::Current,
        (false, true) => ActionEvaluationResultFreshness::ProjectionRebound,
        (true, false) => ActionEvaluationResultFreshness::ExecutionRevalidated,
        (false, false) => ActionEvaluationResultFreshness::ProjectionReboundAndExecutionRevalidated,
    }
}

fn evaluated_action(
    coordinated: CoordinatedAction,
    opportunity: ActionOpportunityId,
    version: ActionOpportunityVersion,
) -> Option<EvaluatedAction> {
    let (actual_opportunity, actual_version, disposition, submission) = coordinated.into_parts();
    if actual_opportunity != opportunity || actual_version != version {
        return None;
    }
    match (disposition, submission) {
        (
            ActionOpportunityDisposition::ActionSubmitted,
            Some(CoordinatedActionSubmission::Command(command)),
        ) => Some(EvaluatedAction::submit(command)),
        (
            ActionOpportunityDisposition::ActionSubmitted,
            Some(CoordinatedActionSubmission::Relocation(relocation)),
        ) => Some(EvaluatedAction::Relocate(relocation.interaction())),
        (ActionOpportunityDisposition::NoApplicableAction, None) => {
            Some(EvaluatedAction::NoApplicableAction)
        }
        _ => None,
    }
}

fn lower_selection(
    definitions: &RuntimeDefinitionSet,
    opportunity: ActionOpportunityId,
    resolution: ResolvedActionSelection,
) -> Result<CoordinatedActionSubmission, CommandEnvelopeError> {
    let resolution = match resolution {
        ResolvedActionSelection::Containment(resolution) => resolution,
        ResolvedActionSelection::Relocation(resolution) => {
            return Ok(CoordinatedActionSubmission::Relocation(resolution));
        }
    };
    CommandEnvelope::new(
        definitions,
        CommandSource::derive_action(opportunity),
        CommandId::new(0),
        resolution.actor(),
        resolution.action().clone(),
        vec![
            CommandBinding::new(
                transfer_binding("actor"),
                CommandValue::Actor(resolution.actor()),
            ),
            CommandBinding::new(
                transfer_binding("destination"),
                CommandValue::Entity(resolution.destination()),
            ),
            CommandBinding::new(
                transfer_binding("item"),
                CommandValue::Entity(resolution.item()),
            ),
            CommandBinding::new(
                transfer_binding("source"),
                CommandValue::Entity(resolution.source()),
            ),
        ],
    )
    .map(CoordinatedActionSubmission::Command)
}

fn transfer_binding(name: &'static str) -> BindingName {
    match BindingName::parse(name) {
        Ok(binding) => binding,
        Err(error) => {
            unreachable!("fixed containment-transfer binding {name} is invalid: {error}")
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use world_core::{ActorId, EntityId, SimDuration, WorldRevision};
    use world_decision::BaselineActionPolicy;
    use world_defs::{
        ActionBindingData, ActionData, ArtifactData, ArtifactValidator, DefinitionKey,
        DefinitionLinker, EffectCallData, EngineProtocolVersion, EventData, EventEmissionData,
        EventFieldBindingData, EventFieldData, EventFieldName, ExactPackSet, ExactPackageSelection,
        InterfaceVersion, LocalDefinitionName, OperationCallData, OperationKind, OperationName,
        OperationParameter, PackCoordinate, PackKey, PackManifestData, PackVersion, ParameterName,
        SelectedPackage, SemanticInterfaceCatalog, SemanticInterfaceDescriptor,
        SemanticInterfaceKey, SemanticOperationDescriptor, SourceSnapshotId, ValueKind,
    };
    use world_model::{
        AcceptedState, ActionInteractionScope, ActionOpportunityGeneration, ActionSponsor,
        ActorReactionCause, AgencyState, ContainerAuthorityRecord, ContainerRecord,
        ContainmentInteractionScope, ContainmentRecord, ContainmentTransferDelta, DirectedRoute,
        DomainState, EpistemicState, EpistemicVersion, EvidenceDeliveryGeneration, EvidenceRecord,
        PhysicalEvent, RelocationInteraction, RelocationInteractionAnchor,
        RelocationInteractionScope, SocialState,
    };

    use super::*;

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("action coordinator fixture must be valid: {error}"),
        }
    }

    fn actor(byte: u8) -> ActorId {
        ActorId::from_bytes([byte; 32])
    }

    fn entity(byte: u8) -> EntityId {
        EntityId::from_bytes([byte; 32])
    }

    fn checked<T, E: fmt::Display>(result: Result<T, E>) -> T {
        valid(result)
    }

    fn installed(policy: impl ActionPolicy) -> InstalledActionPolicy {
        InstalledActionPolicy::new(Arc::new(policy))
    }

    fn definitions() -> RuntimeDefinitionSet {
        let interface_key = checked(SemanticInterfaceKey::parse("example.containment-transfer"));
        let operation_name = checked(OperationName::parse("apply-transfer"));
        let parameters = vec![
            OperationParameter::new(checked(ParameterName::parse("actor")), ValueKind::Actor),
            OperationParameter::new(
                checked(ParameterName::parse("destination")),
                ValueKind::Entity,
            ),
            OperationParameter::new(checked(ParameterName::parse("item")), ValueKind::Entity),
            OperationParameter::new(checked(ParameterName::parse("source")), ValueKind::Entity),
        ];
        let operation = checked(SemanticOperationDescriptor::new(
            operation_name.clone(),
            OperationKind::Effect,
            parameters,
        ));
        let descriptor = checked(SemanticInterfaceDescriptor::new(
            interface_key.clone(),
            checked(InterfaceVersion::new(1)),
            vec![operation],
        ));
        let actor_binding = checked(BindingName::parse("actor"));
        let destination_binding = checked(BindingName::parse("destination"));
        let item_binding = checked(BindingName::parse("item"));
        let source_binding = checked(BindingName::parse("source"));
        let bindings = vec![
            ActionBindingData::new(actor_binding.clone(), ValueKind::Actor),
            ActionBindingData::new(destination_binding.clone(), ValueKind::Entity),
            ActionBindingData::new(item_binding.clone(), ValueKind::Entity),
            ActionBindingData::new(source_binding.clone(), ValueKind::Entity),
        ];
        let event_name = checked(LocalDefinitionName::parse("item-moved"));
        let pack_key = checked(PackKey::parse("example.pack"));
        let action = ActionData::new(
            checked(LocalDefinitionName::parse("move-item")),
            bindings,
            Vec::new(),
            vec![EffectCallData::new(OperationCallData::new(
                interface_key,
                operation_name,
                vec![
                    actor_binding.clone(),
                    destination_binding.clone(),
                    item_binding.clone(),
                    source_binding.clone(),
                ],
            ))],
            vec![EventEmissionData::new(
                DefinitionKey::new(pack_key.clone(), event_name.clone()),
                vec![
                    EventFieldBindingData::new(
                        checked(EventFieldName::parse("actor")),
                        actor_binding,
                    ),
                    EventFieldBindingData::new(
                        checked(EventFieldName::parse("destination")),
                        destination_binding,
                    ),
                    EventFieldBindingData::new(
                        checked(EventFieldName::parse("item")),
                        item_binding,
                    ),
                    EventFieldBindingData::new(
                        checked(EventFieldName::parse("source")),
                        source_binding,
                    ),
                ],
            )],
        );
        let event = EventData::new(
            event_name,
            vec![
                EventFieldData::new(checked(EventFieldName::parse("actor")), ValueKind::Actor),
                EventFieldData::new(
                    checked(EventFieldName::parse("destination")),
                    ValueKind::Entity,
                ),
                EventFieldData::new(checked(EventFieldName::parse("item")), ValueKind::Entity),
                EventFieldData::new(checked(EventFieldName::parse("source")), ValueKind::Entity),
            ],
        );
        let coordinate = PackCoordinate::new(pack_key, PackVersion::new(1, 0, 0));
        let manifest = PackManifestData::new(
            EngineProtocolVersion::new(1),
            coordinate.clone(),
            Vec::new(),
        );
        let artifact = valid(
            ArtifactValidator::new(&valid(SemanticInterfaceCatalog::new(vec![
                descriptor.clone(),
            ])))
            .validate(ArtifactData::new(
                manifest,
                vec![descriptor.reference()],
                vec![action],
                vec![event],
            )),
        );
        let selection = ExactPackageSelection::new(
            coordinate.clone(),
            vec![SelectedPackage::new(
                coordinate,
                SourceSnapshotId::from_bytes([0x51; 32]),
                Vec::new(),
            )],
        );
        valid(DefinitionLinker::link(valid(ExactPackSet::finalize(
            selection,
            vec![artifact],
        ))))
    }

    fn containment_actions(
        definitions: &RuntimeDefinitionSet,
    ) -> ContainmentTransferActionDefinitions {
        valid(ContainmentTransferActionDefinitions::new(
            definitions,
            vec![DefinitionKey::new(
                definitions.root().pack_key().clone(),
                checked(LocalDefinitionName::parse("move-item")),
            )],
        ))
    }

    fn relocation_definitions() -> RuntimeDefinitionSet {
        let interface_key = checked(SemanticInterfaceKey::parse("example.relocation"));
        let operation_name = checked(OperationName::parse("relocate"));
        let descriptor = checked(SemanticInterfaceDescriptor::new(
            interface_key.clone(),
            checked(InterfaceVersion::new(1)),
            vec![checked(SemanticOperationDescriptor::new(
                operation_name.clone(),
                OperationKind::Effect,
                vec![
                    OperationParameter::new(
                        checked(ParameterName::parse("actor")),
                        ValueKind::Actor,
                    ),
                    OperationParameter::new(
                        checked(ParameterName::parse("destination")),
                        ValueKind::Entity,
                    ),
                    OperationParameter::new(
                        checked(ParameterName::parse("source")),
                        ValueKind::Entity,
                    ),
                ],
            ))],
        ));
        let actor = checked(BindingName::parse("actor"));
        let destination = checked(BindingName::parse("destination"));
        let source = checked(BindingName::parse("source"));
        let bindings = vec![
            ActionBindingData::new(actor.clone(), ValueKind::Actor),
            ActionBindingData::new(destination.clone(), ValueKind::Entity),
            ActionBindingData::new(source.clone(), ValueKind::Entity),
        ];
        let event_name = checked(LocalDefinitionName::parse("relocation-requested"));
        let pack_key = checked(PackKey::parse("example.relocation-pack"));
        let actions = ["start-relocation", "pause-relocation", "resume-relocation"]
            .into_iter()
            .map(|name| {
                ActionData::new(
                    checked(LocalDefinitionName::parse(name)),
                    bindings.clone(),
                    Vec::new(),
                    vec![EffectCallData::new(OperationCallData::new(
                        interface_key.clone(),
                        operation_name.clone(),
                        vec![actor.clone(), destination.clone(), source.clone()],
                    ))],
                    vec![EventEmissionData::new(
                        DefinitionKey::new(pack_key.clone(), event_name.clone()),
                        vec![
                            EventFieldBindingData::new(
                                checked(EventFieldName::parse("actor")),
                                actor.clone(),
                            ),
                            EventFieldBindingData::new(
                                checked(EventFieldName::parse("destination")),
                                destination.clone(),
                            ),
                            EventFieldBindingData::new(
                                checked(EventFieldName::parse("source")),
                                source.clone(),
                            ),
                        ],
                    )],
                )
            })
            .collect();
        let coordinate = PackCoordinate::new(pack_key.clone(), PackVersion::new(1, 0, 0));
        let artifact = valid(
            ArtifactValidator::new(&valid(SemanticInterfaceCatalog::new(vec![
                descriptor.clone(),
            ])))
            .validate(ArtifactData::new(
                PackManifestData::new(
                    EngineProtocolVersion::new(1),
                    coordinate.clone(),
                    Vec::new(),
                ),
                vec![descriptor.reference()],
                actions,
                vec![EventData::new(
                    event_name,
                    vec![
                        EventFieldData::new(
                            checked(EventFieldName::parse("actor")),
                            ValueKind::Actor,
                        ),
                        EventFieldData::new(
                            checked(EventFieldName::parse("destination")),
                            ValueKind::Entity,
                        ),
                        EventFieldData::new(
                            checked(EventFieldName::parse("source")),
                            ValueKind::Entity,
                        ),
                    ],
                )],
            )),
        );
        let selection = ExactPackageSelection::new(
            coordinate.clone(),
            vec![SelectedPackage::new(
                coordinate,
                SourceSnapshotId::from_bytes([0x52; 32]),
                Vec::new(),
            )],
        );
        valid(DefinitionLinker::link(valid(ExactPackSet::finalize(
            selection,
            vec![artifact],
        ))))
    }

    fn relocation_action(definitions: &RuntimeDefinitionSet, name: &str) -> DefinitionKey {
        DefinitionKey::new(
            definitions.root().pack_key().clone(),
            checked(LocalDefinitionName::parse(name)),
        )
    }

    fn snapshot(acting: ActorId, item: Option<EntityId>) -> WorldSnapshot {
        let source = entity(0x30);
        let destination = entity(0x40);
        let containment = item
            .map(|item| vec![ContainmentRecord::new(item, source)])
            .unwrap_or_default();
        let epistemic = item.map_or_else(EpistemicState::empty, |item| {
            let delta = valid(ContainmentTransferDelta::new(
                acting,
                item,
                entity(0x50),
                source,
            ));
            let PhysicalEvent::ItemTransferred(event) = PhysicalEvent::item_transferred(delta)
            else {
                panic!("containment fixture must produce item-transfer evidence")
            };
            valid(EpistemicState::empty().assimilate(
                acting,
                EpistemicVersion::EMPTY,
                vec![EvidenceRecord::direct_item_transfer(
                    acting,
                    EvidenceDeliveryGeneration::new(1)
                        .unwrap_or_else(|| panic!("fixture evidence generation is nonzero")),
                    event,
                )],
            ))
        });
        let accepted = AcceptedState::new(
            valid(DomainState::new(
                vec![
                    ContainerRecord::new(source, 4),
                    ContainerRecord::new(destination, 4),
                ],
                containment,
                vec![ContainerAuthorityRecord::new(acting, source)],
            )),
            epistemic,
            SocialState::empty(),
            AgencyState::empty(),
        );
        WorldSnapshot::new(WorldRevision::ROOT, accepted)
    }

    fn opportunity(acting: ActorId, generation: u64) -> ActionOpportunity {
        ActionOpportunity::open(
            acting,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x61; 32])),
            ActionInteractionScope::containment(valid(ContainmentInteractionScope::new(
                entity(0x30),
                vec![entity(0x40)],
                vec![entity(0x20)],
                8,
            ))),
            ActionOpportunityGeneration::new(generation),
        )
    }

    #[derive(Clone, Copy)]
    struct FirstSuppliedPolicy {
        semantics: ActionPolicySemanticsId,
    }

    impl ActionPolicy for FirstSuppliedPolicy {
        fn semantics_id(&self) -> ActionPolicySemanticsId {
            self.semantics
        }

        fn decide(
            &self,
            input: &ActionContextPayload,
        ) -> Result<ActionDecision, ActionPolicyError> {
            Ok(match input.candidates().candidates().first() {
                Some(candidate) => ActionDecision::Select {
                    candidate: candidate.id(),
                    input: input.input_fingerprint(),
                },
                None => ActionDecision::NoApplicableAction {
                    input: input.input_fingerprint(),
                },
            })
        }
    }

    struct FabricatedPolicy {
        semantics: ActionPolicySemanticsId,
        candidate: GroundedActionCandidateId,
        fingerprint: Option<ActionInputFingerprint>,
    }

    impl ActionPolicy for FabricatedPolicy {
        fn semantics_id(&self) -> ActionPolicySemanticsId {
            self.semantics
        }

        fn decide(
            &self,
            input: &ActionContextPayload,
        ) -> Result<ActionDecision, ActionPolicyError> {
            Ok(ActionDecision::Select {
                candidate: self.candidate,
                input: self
                    .fingerprint
                    .unwrap_or_else(|| input.input_fingerprint()),
            })
        }
    }

    struct FailingPolicy {
        semantics: ActionPolicySemanticsId,
    }

    impl ActionPolicy for FailingPolicy {
        fn semantics_id(&self) -> ActionPolicySemanticsId {
            self.semantics
        }

        fn decide(&self, _: &ActionContextPayload) -> Result<ActionDecision, ActionPolicyError> {
            Err(ActionPolicyError::EvaluationFailed)
        }
    }

    #[test]
    fn policy_boundary_is_object_safe() {
        fn accepts_policy(_: &dyn ActionPolicy) {}

        accepts_policy(&BaselineActionPolicy::new());
    }

    #[test]
    fn prepared_action_retains_canonical_artifacts_for_fresh_interpretation() {
        let acting = actor(0x10);
        let snapshot = snapshot(acting, Some(entity(0x20)));
        let first_opportunity = opportunity(acting, 1);
        let second_opportunity = opportunity(acting, 2);
        let definitions = definitions();
        let actions = containment_actions(&definitions);
        let policy = BaselineActionPolicy::new();

        let first = valid(PreparedAction::build(
            &snapshot,
            &first_opportunity,
            &definitions,
            Some(&actions),
            None,
            policy.semantics_id(),
        ));
        assert_eq!(first.opportunity, first_opportunity.id());
        assert_eq!(first.expected_version, first_opportunity.version());
        assert_eq!(
            world_context::decode_action_context_payload(&first.encode_request_payload()),
            Ok(first.payload.clone())
        );
        assert_eq!(
            world_context::decode_candidate_resolution_table(&first.encode_private_continuation()),
            Ok(first.resolution.clone())
        );
        assert_eq!(
            world_context::decode_action_read_witness(&first.encode_read_witness()),
            Ok(first.read_witness().clone())
        );
        let decision = valid(policy.decide(&first.payload));

        let fresh = valid(PreparedAction::build(
            &snapshot,
            &second_opportunity,
            &definitions,
            Some(&actions),
            None,
            policy.semantics_id(),
        ));
        assert!(matches!(
            fresh.apply_decision(decision),
            Err(ActionCoordinationError::InputFingerprintMismatch { .. })
        ));
    }

    #[test]
    fn deferred_input_binds_the_exact_owner_local_action_artifacts() {
        let acting = actor(0x10);
        let snapshot = snapshot(acting, Some(entity(0x20)));
        let opportunity = opportunity(acting, 1);
        let definitions = definitions();
        let actions = containment_actions(&definitions);
        let descriptor =
            DeferredActionEvaluatorDescriptor::new(BaselineActionPolicy::new().semantics_id());
        let prepared = valid(PreparedAction::build(
            &snapshot,
            &opportunity,
            &definitions,
            Some(&actions),
            None,
            descriptor.semantics_id(),
        ));
        let payload = prepared.payload.clone();
        let resolution = prepared.resolution.clone();
        let witness = prepared.read_witness().clone();

        let deferred = prepared.deferred_input(descriptor);

        assert_eq!(
            deferred.policy_semantics(),
            descriptor.semantics_id().into_bytes()
        );
        assert_eq!(
            deferred.action_input_fingerprint(),
            payload.input_fingerprint().into_bytes()
        );
        assert_eq!(
            deferred.request().schema(),
            action_evaluation_schema(descriptor.request_payload_schema().into_bytes())
        );
        assert_eq!(
            decode_action_context_payload(deferred.request().bytes()),
            Ok(payload)
        );
        assert_eq!(
            deferred.private_continuation().schema(),
            action_evaluation_schema(
                descriptor
                    .candidate_table_continuation_schema()
                    .into_bytes()
            )
        );
        assert_eq!(
            decode_candidate_resolution_table(deferred.private_continuation().bytes()),
            Ok(resolution)
        );
        assert_eq!(
            deferred.private_read_witness().schema(),
            action_evaluation_schema(descriptor.read_witness_schema().into_bytes())
        );
        assert_eq!(
            decode_action_read_witness(deferred.private_read_witness().bytes()),
            Ok(witness)
        );
        assert_eq!(
            deferred.result_schema(),
            action_evaluation_schema(descriptor.decision_result_schema().into_bytes())
        );
    }

    #[test]
    fn typed_capture_derives_the_fixed_decision_schema_and_canonical_bytes() {
        let capture = ActionEvaluationCaptureId::new(7);
        let invocation = ActionEvaluationInvocationId::from_bytes([0x71; 32]);
        let decision = ActionDecision::NoApplicableAction {
            input: ActionInputFingerprint::from_bytes([0x72; 32]),
        };

        let frontier =
            ActionEvaluationResultCapture::at_invocation_frontier(capture, invocation, decision)
                .into_submission();
        assert_eq!(frontier.capture(), capture);
        assert_eq!(frontier.invocation(), invocation);
        assert_eq!(
            frontier.timing(),
            ActionEvaluationCaptureTiming::InvocationFrontier
        );
        assert_eq!(
            frontier.result_schema(),
            action_evaluation_schema(action_decision_schema().into_bytes())
        );
        assert_eq!(decode_action_decision(frontier.bytes()), Ok(decision));

        let effective = world_core::SimMoment::ORIGIN;
        let host =
            ActionEvaluationResultCapture::host_scheduled(capture, invocation, effective, decision)
                .into_submission();
        assert_eq!(
            host.timing(),
            ActionEvaluationCaptureTiming::HostScheduled(effective)
        );
        assert_eq!(decode_action_decision(host.bytes()), Ok(decision));
    }

    #[test]
    fn result_freshness_is_the_product_of_projection_and_execution_validation() {
        assert_eq!(
            classify_result_freshness_from_matches(true, true),
            ActionEvaluationResultFreshness::Current
        );
        assert_eq!(
            classify_result_freshness_from_matches(false, true),
            ActionEvaluationResultFreshness::ProjectionRebound
        );
        assert_eq!(
            classify_result_freshness_from_matches(true, false),
            ActionEvaluationResultFreshness::ExecutionRevalidated
        );
        assert_eq!(
            classify_result_freshness_from_matches(false, false),
            ActionEvaluationResultFreshness::ProjectionReboundAndExecutionRevalidated
        );
    }

    #[test]
    fn visible_change_reinvokes_only_while_the_fixed_budget_remains() {
        let acting = actor(0x10);
        let snapshot = snapshot(acting, Some(entity(0x20)));
        let opportunity = opportunity(acting, 1);
        let definitions = definitions();
        let actions = containment_actions(&definitions);
        let descriptor =
            DeferredActionEvaluatorDescriptor::new(BaselineActionPolicy::new().semantics_id());
        let fresh = valid(PreparedAction::build(
            &snapshot,
            &opportunity,
            &definitions,
            Some(&actions),
            None,
            descriptor.semantics_id(),
        ));

        let reinvocation = match resolve_visible_change(&fresh, descriptor, 1) {
            Ok(decision) => decision,
            Err(failure) => panic!("visible reinvocation must remain available: {failure:?}"),
        };
        let ActionEvaluationDecision::Reinvoke(input) = reinvocation else {
            panic!("remaining visible-change budget must create a linked successor input")
        };
        assert_eq!(
            input.action_input_fingerprint(),
            fresh.payload.input_fingerprint().into_bytes()
        );
        assert_eq!(
            resolve_visible_change(&fresh, descriptor, 0),
            Err(ActionEvaluationResultFailure::VisibleReinvocationExhausted)
        );
    }

    #[test]
    fn invalid_retained_decisions_require_the_modeled_later_fallback() {
        let acting = actor(0x10);
        let snapshot = snapshot(acting, Some(entity(0x20)));
        let opportunity = opportunity(acting, 1);
        let definitions = definitions();
        let actions = containment_actions(&definitions);
        let semantics = BaselineActionPolicy::new().semantics_id();

        let invalid_fingerprint = valid(PreparedAction::build(
            &snapshot,
            &opportunity,
            &definitions,
            Some(&actions),
            None,
            semantics,
        ));
        let candidate = invalid_fingerprint.payload.candidates().candidates()[0].id();
        let invalid_fingerprint = validate_original_decision(
            invalid_fingerprint,
            ActionDecision::Select {
                candidate,
                input: ActionInputFingerprint::from_bytes([0xff; 32]),
            },
        );
        let Err(failure) = invalid_fingerprint else {
            panic!("a result for another input must not validate")
        };
        assert_eq!(
            action_evaluation_fallback(failure),
            ActionEvaluationDecision::RequireFallback(ActionEvaluationResultFailure::InvalidResult)
        );

        let unknown_candidate = valid(PreparedAction::build(
            &snapshot,
            &opportunity,
            &definitions,
            Some(&actions),
            None,
            semantics,
        ));
        let input = unknown_candidate.payload.input_fingerprint();
        let unknown_candidate = validate_original_decision(
            unknown_candidate,
            ActionDecision::Select {
                candidate: GroundedActionCandidateId::from_bytes([0xfe; 32]),
                input,
            },
        );
        let Err(failure) = unknown_candidate else {
            panic!("a candidate absent from the original request must not validate")
        };
        assert_eq!(
            action_evaluation_fallback(failure),
            ActionEvaluationDecision::RequireFallback(ActionEvaluationResultFailure::InvalidResult)
        );
    }

    #[test]
    fn baseline_and_manual_policies_produce_the_same_lowered_command() {
        let acting = actor(0x10);
        let snapshot = snapshot(acting, Some(entity(0x20)));
        let opportunity = opportunity(acting, 1);
        let definitions = definitions();
        let actions = containment_actions(&definitions);
        let baseline = BaselineActionPolicy::new();
        let manual = FirstSuppliedPolicy {
            semantics: baseline.semantics_id(),
        };
        let baseline = installed(baseline);
        let manual = installed(manual);

        let baseline_result = valid(ActionCoordinator::coordinate(
            &snapshot,
            &opportunity,
            &definitions,
            Some(&actions),
            None,
            &baseline,
        ));
        let manual_result = valid(ActionCoordinator::coordinate(
            &snapshot,
            &opportunity,
            &definitions,
            Some(&actions),
            None,
            &manual,
        ));

        assert_eq!(baseline_result, manual_result);
        let (_, _, disposition, command) = baseline_result.into_parts();
        assert_eq!(disposition, ActionOpportunityDisposition::ActionSubmitted);
        assert!(command.is_some());
    }

    #[test]
    fn fabricated_and_cross_input_selections_are_rejected() {
        let acting = actor(0x10);
        let snapshot = snapshot(acting, Some(entity(0x20)));
        let first_opportunity = opportunity(acting, 1);
        let second_opportunity = opportunity(acting, 2);
        let definitions = definitions();
        let actions = containment_actions(&definitions);
        let semantics = BaselineActionPolicy::new().semantics_id();

        let foreign = valid(PreparedAction::build(
            &snapshot,
            &second_opportunity,
            &definitions,
            Some(&actions),
            None,
            semantics,
        ));
        let foreign_candidate = foreign.payload.candidates().candidates()[0].id();
        let foreign_fingerprint = foreign.payload.input_fingerprint();

        let fabricated = FabricatedPolicy {
            semantics,
            candidate: GroundedActionCandidateId::from_bytes([0xff; 32]),
            fingerprint: None,
        };
        let fabricated = installed(fabricated);
        assert!(matches!(
            ActionCoordinator::coordinate(
                &snapshot,
                &first_opportunity,
                &definitions,
                Some(&actions),
                None,
                &fabricated,
            ),
            Err(ActionCoordinationError::CandidateUnavailable { .. })
        ));

        let cross_fingerprint = FabricatedPolicy {
            semantics,
            candidate: foreign_candidate,
            fingerprint: Some(foreign_fingerprint),
        };
        let cross_fingerprint = installed(cross_fingerprint);
        assert!(matches!(
            ActionCoordinator::coordinate(
                &snapshot,
                &first_opportunity,
                &definitions,
                Some(&actions),
                None,
                &cross_fingerprint,
            ),
            Err(ActionCoordinationError::InputFingerprintMismatch { .. })
        ));

        let cross_candidate = FabricatedPolicy {
            semantics,
            candidate: foreign_candidate,
            fingerprint: None,
        };
        let cross_candidate = installed(cross_candidate);
        assert!(matches!(
            ActionCoordinator::coordinate(
                &snapshot,
                &first_opportunity,
                &definitions,
                Some(&actions),
                None,
                &cross_candidate,
            ),
            Err(ActionCoordinationError::CandidateUnavailable { .. })
        ));
    }

    #[test]
    fn complete_empty_input_produces_the_terminal_empty_disposition() {
        let acting = actor(0x10);
        let snapshot = snapshot(acting, None);
        let opportunity = opportunity(acting, 1);
        let definitions = definitions();
        let actions = containment_actions(&definitions);
        let policy = installed(BaselineActionPolicy::new());

        let result = valid(ActionCoordinator::coordinate(
            &snapshot,
            &opportunity,
            &definitions,
            Some(&actions),
            None,
            &policy,
        ));

        let (result_opportunity, result_version, disposition, command) = result.into_parts();
        assert_eq!(result_opportunity, opportunity.id());
        assert_eq!(result_version, opportunity.version());
        assert_eq!(
            disposition,
            ActionOpportunityDisposition::NoApplicableAction
        );
        assert_eq!(command, None);
    }

    #[test]
    fn bounded_policy_failure_uses_the_existing_terminal_disposition() {
        let acting = actor(0x10);
        let snapshot = snapshot(acting, Some(entity(0x20)));
        let opportunity = opportunity(acting, 1);
        let definitions = definitions();
        let actions = containment_actions(&definitions);
        let policy = FailingPolicy {
            semantics: BaselineActionPolicy::new().semantics_id(),
        };
        let policy = installed(policy);

        let result = valid(ActionCoordinator::coordinate(
            &snapshot,
            &opportunity,
            &definitions,
            Some(&actions),
            None,
            &policy,
        ));

        let (_, _, disposition, command) = result.into_parts();
        assert_eq!(disposition, ActionOpportunityDisposition::Failed);
        assert!(command.is_none());
    }

    #[test]
    fn private_resolution_lowers_exact_bindings_and_one_shot_namespace() {
        let acting = actor(0x10);
        let item = entity(0x20);
        let source = entity(0x30);
        let destination = entity(0x40);
        let snapshot = snapshot(acting, Some(item));
        let opportunity = opportunity(acting, 1);
        let definitions = definitions();
        let actions = containment_actions(&definitions);
        let policy = installed(BaselineActionPolicy::new());

        let result = valid(ActionCoordinator::coordinate(
            &snapshot,
            &opportunity,
            &definitions,
            Some(&actions),
            None,
            &policy,
        ));
        let (_, _, _, submission) = result.into_parts();
        let submission = submission.unwrap_or_else(|| panic!("nonempty action input must lower"));
        let CoordinatedActionSubmission::Command(command) = submission else {
            panic!("containment selection must lower to a command")
        };

        assert_eq!(
            command.source(),
            CommandSource::derive_action(opportunity.id())
        );
        assert_eq!(command.id(), CommandId::new(0));
        assert_eq!(command.actor(), acting);
        assert_eq!(command.definition_set_digest(), definitions.digest());
        assert_eq!(
            command.action(),
            &DefinitionKey::new(
                definitions.root().pack_key().clone(),
                checked(LocalDefinitionName::parse("move-item")),
            )
        );
        assert_eq!(
            command.bindings(),
            [
                CommandBinding::new(
                    checked(BindingName::parse("actor")),
                    CommandValue::Actor(acting),
                ),
                CommandBinding::new(
                    checked(BindingName::parse("destination")),
                    CommandValue::Entity(destination),
                ),
                CommandBinding::new(
                    checked(BindingName::parse("item")),
                    CommandValue::Entity(item),
                ),
                CommandBinding::new(
                    checked(BindingName::parse("source")),
                    CommandValue::Entity(source),
                ),
            ]
        );
    }

    #[test]
    fn relocation_scope_dispatches_without_reading_hidden_route_state() {
        let acting = actor(0x10);
        let route = valid(DirectedRoute::new(
            entity(0x20),
            entity(0x21),
            SimDuration::from_ticks(5),
        ));
        let changed_route = valid(DirectedRoute::new(
            route.source(),
            route.destination(),
            SimDuration::from_ticks(9),
        ));
        let snapshot = |routes| {
            let domain = valid(
                valid(DomainState::new(Vec::new(), Vec::new(), Vec::new()))
                    .with_mobility(routes, Vec::new()),
            );
            WorldSnapshot::new(
                WorldRevision::ROOT,
                AcceptedState::new(
                    domain,
                    EpistemicState::empty(),
                    SocialState::empty(),
                    AgencyState::empty(),
                ),
            )
        };
        let accepted_route = snapshot(vec![route]);
        let missing_route = snapshot(Vec::new());
        let changed_hidden_route = snapshot(vec![changed_route]);
        let opportunity = ActionOpportunity::open(
            acting,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x61; 32])),
            ActionInteractionScope::relocation(valid(RelocationInteractionScope::new(
                vec![RelocationInteractionAnchor::new(
                    RelocationInteraction::Pause(route.id()),
                    route.source(),
                    route.destination(),
                )],
                1,
            ))),
            ActionOpportunityGeneration::new(1),
        );
        let definitions = relocation_definitions();
        let actions = valid(RelocationActionDefinitions::new(
            &definitions,
            relocation_action(&definitions, "start-relocation"),
            relocation_action(&definitions, "pause-relocation"),
            relocation_action(&definitions, "resume-relocation"),
        ));
        let policy = installed(BaselineActionPolicy::new());

        assert_eq!(
            ActionCoordinator::coordinate(
                &accepted_route,
                &opportunity,
                &definitions,
                None,
                None,
                &policy,
            ),
            Err(ActionCoordinationError::RelocationDefinitionsUnbound)
        );

        let coordinated = valid(ActionCoordinator::coordinate(
            &accepted_route,
            &opportunity,
            &definitions,
            None,
            Some(&actions),
            &policy,
        ));
        let missing_coordinated = valid(ActionCoordinator::coordinate(
            &missing_route,
            &opportunity,
            &definitions,
            None,
            Some(&actions),
            &policy,
        ));
        let changed_coordinated = valid(ActionCoordinator::coordinate(
            &changed_hidden_route,
            &opportunity,
            &definitions,
            None,
            Some(&actions),
            &policy,
        ));
        assert_eq!(coordinated, missing_coordinated);
        assert_eq!(coordinated, changed_coordinated);

        let (selected_opportunity, version, disposition, submission) = coordinated.into_parts();
        assert_eq!(selected_opportunity, opportunity.id());
        assert_eq!(version, opportunity.version());
        assert_eq!(disposition, ActionOpportunityDisposition::ActionSubmitted);
        let Some(CoordinatedActionSubmission::Relocation(selection)) = submission else {
            panic!("relocation must remain a private process selection")
        };
        assert_eq!(selection.actor(), acting);
        assert_eq!(
            selection.action(),
            &relocation_action(&definitions, "pause-relocation")
        );
        assert_eq!(
            selection.interaction(),
            RelocationInteraction::Pause(route.id())
        );
    }
}
