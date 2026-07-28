use core::fmt;
use std::sync::Arc;

use world_core::{ActorId, SimMoment};
use world_defs::{BindingName, DefinitionKey, ValueKind};
use world_model::{
    ActionEvaluationInvocationId, ActionOpportunityDisposition, CommandAttemptOutcome,
    CommandBinding, CommandEnvelope, CommandEnvelopeError, CommandId, CommandSource,
    SystemCommandSourceId,
};
use world_runtime::{
    ActionEvaluationCaptureId, ActionEvaluationCaptureOutcome, ActionEvaluationCaptureTiming,
    AdmitRequest, AttemptBinding, AuthorityCursor, AuthorityRecordId, CancelAttemptOutcome,
    CancelAttemptRequest, CancelAttemptRequestId, CommandFireClassification,
    DeferredActionAdmissionModeV1, FirePreparation, FireRequest, InputId, KernelSafetyCause,
    KernelSafetyDisposition, LedgerRetirement, ManageOutcome, ManageRequest, ManagementRequestId,
    MomentWorkDecision, MomentWorkInput, MomentWorkProposals, PreparedFire, PreparedFireFailure,
    RunAttemptId, RunFinalization, RuntimeActionEvaluationCaptureError, RuntimeAttemptDriver,
    RuntimeAttemptStatus, RuntimeControlError, RuntimeDriveError, RuntimeEvaluationError,
    RuntimeReadError, SessionMode,
};

use crate::action::{
    ActionCoordinator, ActionEvaluationResultCapture, CoordinatedActionSubmission,
    PendingActionEvaluation,
};
use crate::resolution::ResolvedExecutionInner;
use crate::session::WorldSession;

/// Trusted exogenous request to invoke one exact checked pack action.
///
/// This is system ingress, not actor control. In-world actors act only through
/// action opportunities and the actor-safe [`crate::ActionPolicy`] boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemCommandRequest {
    input: InputId,
    effective: SimMoment,
    source: SystemCommandSourceId,
    command: CommandId,
    actor: ActorId,
    action: DefinitionKey,
    bindings: Vec<CommandBinding>,
}

impl SystemCommandRequest {
    /// Describes one exact trusted system invocation without exposing a runtime command.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        input: InputId,
        effective: SimMoment,
        source: SystemCommandSourceId,
        command: CommandId,
        actor: ActorId,
        action: DefinitionKey,
        bindings: Vec<CommandBinding>,
    ) -> Self {
        Self {
            input,
            effective,
            source,
            command,
            actor,
            action,
            bindings,
        }
    }
}

/// Stable result of trusted system-command ingress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemCommandAdmissionOutcome {
    /// The command is retained in one ingress record for its exact moment.
    Scheduled {
        /// Published ingress record.
        record: AuthorityRecordId,
        /// Exact scheduled moment.
        effective: SimMoment,
    },
}

/// Inclusive bound for one complete-moment advancement step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdvanceRequest {
    through: SimMoment,
}

impl AdvanceRequest {
    /// Constructs one inclusive simulation-time bound.
    #[must_use]
    pub const fn through(moment: SimMoment) -> Self {
        Self { through: moment }
    }
}

/// Stable command classification reported by one published advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandResolution {
    /// A genuinely new command was evaluated.
    New(CommandAttemptOutcome),
    /// An exact prior result was consumed without reevaluation.
    Retained(CommandAttemptOutcome),
    /// A source-scoped command ID was reused with different content.
    IdReuseMismatch,
    /// The source-scoped identity has a durable collision outcome.
    IdCollision,
    /// The command identity lies behind its source retirement frontier.
    Retired,
}

/// Stable result for one consumed command delivery in a published moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedCommandDelivery {
    source: CommandSource,
    command: CommandId,
    resolution: CommandResolution,
}

impl ResolvedCommandDelivery {
    /// Returns the semantic command-producing namespace.
    #[must_use]
    pub const fn source(self) -> CommandSource {
        self.source
    }

    /// Returns the source-scoped command identity.
    #[must_use]
    pub const fn command(self) -> CommandId {
        self.command
    }

    /// Returns the stable delivery classification.
    #[must_use]
    pub const fn resolution(self) -> CommandResolution {
        self.resolution
    }
}

/// Modeled result of one bounded complete-moment advance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdvanceOutcome {
    /// One complete least-due moment was atomically published.
    Published {
        /// Published authority record.
        record: AuthorityRecordId,
        /// Resulting authority cursor.
        cursor: AuthorityCursor,
        /// Exact simulation moment consumed by the record.
        moment: SimMoment,
        /// One stable result for every consumed command delivery.
        commands: Vec<ResolvedCommandDelivery>,
        /// Number of post-commit dispatches consumed by this record.
        post_commit_consumed: usize,
        /// Opportunities terminally consumed by actor-control evaluation.
        action_opportunities_consumed: Vec<world_model::ActionOpportunityId>,
        /// Outcome-neutral action-attempt continuations consumed by this record.
        attempt_resolved: Vec<world_model::ActionOpportunityId>,
    },
    /// A deterministic kernel limit changed session health without consuming due work.
    KernelSafety {
        /// Published management-family authority record.
        record: AuthorityRecordId,
        /// Resulting authority cursor.
        cursor: AuthorityCursor,
        /// Exact inspectable cause and due-set evidence.
        cause: KernelSafetyCause,
        /// Resulting session health disposition.
        disposition: KernelSafetyDisposition,
    },
    /// The scheduler contains no work.
    NoScheduledWork,
    /// The least work lies beyond the requested bound.
    NoWorkDue {
        /// Exact least scheduled moment.
        next: SimMoment,
        /// Inclusive requested bound.
        through: SimMoment,
    },
}

/// Read-only projection of one attempt's durable phase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunAttemptStatus {
    /// Another operation may be reserved.
    Active,
    /// One operation remains durably reserved.
    Reserved,
    /// An immutable terminal prefix has been selected.
    Finalized(Box<RunFinalization>),
}

/// Non-cloneable host capability over one controlled attempt.
pub struct RunAttempt {
    runtime: RuntimeAttemptDriver,
    execution: Arc<ResolvedExecutionInner>,
    binding: AttemptBinding,
}

impl RunAttempt {
    pub(crate) fn new(
        runtime: RuntimeAttemptDriver,
        execution: Arc<ResolvedExecutionInner>,
    ) -> Self {
        let binding = runtime.binding();
        Self {
            runtime,
            execution,
            binding,
        }
    }

    /// Returns the immutable physical attempt identity.
    #[must_use]
    pub const fn id(&self) -> RunAttemptId {
        self.binding.attempt()
    }

    /// Returns the permanent execution correspondence.
    #[must_use]
    pub const fn binding(&self) -> &AttemptBinding {
        &self.binding
    }

    /// Reads the current durable attempt-control phase.
    pub fn status(&self) -> Result<RunAttemptStatus, AttemptError> {
        self.runtime
            .status()
            .map(|status| match status {
                RuntimeAttemptStatus::Active => RunAttemptStatus::Active,
                RuntimeAttemptStatus::Reserved => RunAttemptStatus::Reserved,
                RuntimeAttemptStatus::Finalized(finalization) => {
                    RunAttemptStatus::Finalized(Box::new(finalization))
                }
            })
            .map_err(map_read_error)
    }

    /// Creates a cloneable read-only facade for the same world session.
    #[must_use]
    pub fn session(&self) -> WorldSession {
        WorldSession::new(self.runtime.session_reader())
    }

    /// Copies the currently dispatchable deferred action requests in
    /// deterministic invocation order.
    pub fn pending_action_evaluations(&self) -> Result<Vec<PendingActionEvaluation>, AttemptError> {
        let raw = self
            .runtime
            .pending_action_evaluations()
            .map_err(map_read_error)?;
        let action_execution = self.execution.lifecycle.action_execution();
        if !raw.is_empty() && action_execution.deferred_descriptor().is_none() {
            return Err(AttemptError::Integrity);
        }
        let expected_implementation = self
            .execution
            .lifecycle
            .profiles()
            .action()
            .binding()
            .implementation();
        let expected_semantics = action_execution.semantics_id();
        raw.into_iter()
            .map(|raw| {
                let pending =
                    PendingActionEvaluation::decode(raw).ok_or(AttemptError::Integrity)?;
                if pending.implementation() != expected_implementation
                    || pending.payload().policy_semantics() != expected_semantics
                {
                    return Err(AttemptError::Integrity);
                }
                Ok(pending)
            })
            .collect()
    }

    /// Atomically captures one typed deferred action result.
    ///
    /// Exact retries return the originally retained outcome. An oversized
    /// result is likewise a successful recorded outcome whose later fallback
    /// remains runtime-owned.
    pub fn capture_action_evaluation_result(
        &mut self,
        capture: ActionEvaluationResultCapture,
    ) -> Result<ActionEvaluationCaptureOutcome, ActionEvaluationCaptureError> {
        self.runtime
            .capture_action_evaluation_result(capture.into_submission())
            .map_err(map_action_evaluation_capture_error)
    }

    /// Resolves and admits one exact trusted system command.
    pub fn submit_system_command(
        &mut self,
        request: SystemCommandRequest,
    ) -> Result<SystemCommandAdmissionOutcome, AttemptError> {
        let SystemCommandRequest {
            input,
            effective,
            source,
            command,
            actor,
            action,
            bindings,
        } = request;
        let command = CommandEnvelope::new(
            &self.execution.definitions,
            CommandSource::derive_system(source),
            command,
            actor,
            action,
            bindings,
        )
        .map_err(|error| {
            AttemptError::InvalidSystemCommand(Box::new(map_system_command_error(error)))
        })?;
        let outcome = self
            .runtime
            .admit(AdmitRequest::new(input, effective, command))
            .map_err(map_drive_error)?;
        Ok(SystemCommandAdmissionOutcome::Scheduled {
            record: outcome.record(),
            effective: outcome.effective(),
        })
    }

    /// Advances one complete least-due moment using this execution's action policy.
    ///
    /// The policy receives only actor-safe action payloads. Runtime still
    /// owns opportunity consumption, command legality, resolution, mutation,
    /// publication, and causal scheduling.
    pub fn advance(&mut self, request: AdvanceRequest) -> Result<AdvanceOutcome, AttemptError> {
        let preparation = match self
            .runtime
            .prepare_fire(FireRequest::through(request.through))
        {
            Ok(prepared) => prepared,
            Err(RuntimeDriveError::NoScheduledWork) => {
                return Ok(AdvanceOutcome::NoScheduledWork);
            }
            Err(RuntimeDriveError::NoWorkDue { next, through }) => {
                return Ok(AdvanceOutcome::NoWorkDue { next, through });
            }
            Err(error) => return Err(map_drive_error(error)),
        };
        let prepared = match preparation {
            FirePreparation::Ready(prepared) => prepared,
            FirePreparation::KernelSafety(prepared) => {
                let outcome = self
                    .runtime
                    .complete_kernel_safety(prepared)
                    .map_err(map_drive_error)?;
                return Ok(AdvanceOutcome::KernelSafety {
                    record: outcome.record(),
                    cursor: outcome.cursor(),
                    cause: outcome.cause(),
                    disposition: outcome.disposition(),
                });
            }
        };

        let proposals = match evaluate_prepared(&self.execution, &prepared) {
            Ok(proposals) => proposals,
            Err(error) => {
                self.runtime
                    .fail_prepared_fire(prepared, PreparedFireFailure::EngineFailure)
                    .map_err(map_control_error)?;
                return Err(map_evaluation_error(error));
            }
        };
        let outcome = self
            .runtime
            .complete_fire(prepared, proposals)
            .map_err(map_drive_error)?;
        let commands = outcome
            .command_resolutions()
            .iter()
            .map(|resolution| ResolvedCommandDelivery {
                source: resolution.source(),
                command: resolution.command(),
                resolution: match resolution.classification() {
                    CommandFireClassification::New(outcome) => CommandResolution::New(outcome),
                    CommandFireClassification::Retained(outcome) => {
                        CommandResolution::Retained(outcome)
                    }
                    CommandFireClassification::IdReuseMismatch => {
                        CommandResolution::IdReuseMismatch
                    }
                    CommandFireClassification::IdCollision => CommandResolution::IdCollision,
                    CommandFireClassification::Retired => CommandResolution::Retired,
                },
            })
            .collect();
        Ok(AdvanceOutcome::Published {
            record: outcome.record(),
            cursor: outcome.cursor(),
            moment: outcome.moment(),
            commands,
            post_commit_consumed: outcome.post_commit_consumed(),
            action_opportunities_consumed: outcome.action_opportunities_consumed().to_vec(),
            attempt_resolved: outcome.attempt_resolved().to_vec(),
        })
    }

    /// Applies one idempotent host-management request.
    pub fn submit_management_request(
        &mut self,
        request: ManageRequest,
    ) -> Result<ManageOutcome, AttemptError> {
        self.runtime.manage(request).map_err(map_drive_error)
    }

    /// Selects the current reconciled prefix as terminal for this attempt.
    pub fn cancel(
        &mut self,
        request: CancelAttemptRequest,
    ) -> Result<CancelAttemptOutcome, AttemptError> {
        self.runtime
            .cancel_attempt(request)
            .map_err(map_control_error)
    }
}

fn evaluate_prepared(
    execution: &ResolvedExecutionInner,
    prepared: &PreparedFire,
) -> Result<MomentWorkProposals, RuntimeEvaluationError> {
    let mut decisions = Vec::with_capacity(prepared.work().len());
    for input in prepared.work() {
        let decision = match input {
            MomentWorkInput::EvaluateCommand { .. } => {
                execution.activation.evaluate_command_work(input)?
            }
            MomentWorkInput::PostCommitDispatch { .. } => {
                execution.post_commit_router.route(input)?
            }
            MomentWorkInput::EvidenceAssimilation {
                snapshot,
                actor,
                evidence,
                ..
            } => crate::lifecycle::assimilate_evidence(
                input,
                snapshot,
                actor,
                evidence,
                execution.lifecycle.evidence_assimilator(),
            )?,
            MomentWorkInput::Appraisal {
                snapshot,
                actor,
                evidence,
                previous,
                ..
            } => crate::lifecycle::appraise_containment(
                input,
                snapshot,
                actor,
                evidence,
                previous,
                execution.lifecycle.appraisal_evaluator(),
            )?,
            MomentWorkInput::IntentReview {
                snapshot,
                actor,
                generation,
                appraisals,
                ..
            } => crate::lifecycle::review_intent(
                input,
                snapshot,
                actor,
                generation,
                appraisals,
                execution.lifecycle.intent_policy(),
            )?,
            MomentWorkInput::ActivityInitialization {
                snapshot,
                actor,
                generation,
                intents,
                ..
            } => crate::lifecycle::initialize_activity(
                input,
                snapshot,
                actor,
                generation,
                intents,
                execution.lifecycle.activity_controller(),
            )?,
            MomentWorkInput::ActionReady {
                snapshot,
                opportunity,
                ..
            } => {
                if let Some(action_policy) = execution.lifecycle.action_execution().inline_policy()
                {
                    match ActionCoordinator::coordinate(
                        snapshot,
                        opportunity,
                        &execution.definitions,
                        execution.containment_actions.as_ref(),
                        execution.relocation_actions.as_ref(),
                        action_policy,
                    ) {
                        Ok(coordinated) => {
                            let (
                                coordinated_opportunity,
                                expected_version,
                                disposition,
                                submission,
                            ) = coordinated.into_parts();
                            if coordinated_opportunity != opportunity.id()
                                || expected_version != opportunity.version()
                            {
                                return Err(RuntimeEvaluationError::Integrity);
                            }
                            match submission {
                                Some(CoordinatedActionSubmission::Command(command))
                                    if disposition
                                        == ActionOpportunityDisposition::ActionSubmitted =>
                                {
                                    MomentWorkDecision::submit_action(input, command)
                                        .map_err(|_| RuntimeEvaluationError::Integrity)?
                                }
                                Some(CoordinatedActionSubmission::Relocation(relocation))
                                    if disposition
                                        == ActionOpportunityDisposition::ActionSubmitted =>
                                {
                                    MomentWorkDecision::submit_relocation_action(
                                        input,
                                        relocation.interaction(),
                                    )
                                    .map_err(|_| RuntimeEvaluationError::Integrity)?
                                }
                                None if disposition
                                    != ActionOpportunityDisposition::ActionSubmitted =>
                                {
                                    MomentWorkDecision::finish_action(input, disposition)
                                        .map_err(|_| RuntimeEvaluationError::Integrity)?
                                }
                                _ => return Err(RuntimeEvaluationError::Integrity),
                            }
                        }
                        Err(_) => MomentWorkDecision::finish_action(
                            input,
                            ActionOpportunityDisposition::Failed,
                        )
                        .map_err(|_| RuntimeEvaluationError::Integrity)?,
                    }
                } else if let Some(descriptor) =
                    execution.lifecycle.action_execution().deferred_descriptor()
                {
                    match ActionCoordinator::prepare_deferred(
                        snapshot,
                        opportunity,
                        &execution.definitions,
                        execution.containment_actions.as_ref(),
                        execution.relocation_actions.as_ref(),
                        descriptor,
                    ) {
                        Ok(deferred) => MomentWorkDecision::begin_deferred_action(input, deferred)
                            .map_err(|_| RuntimeEvaluationError::Integrity)?,
                        Err(_) => MomentWorkDecision::finish_action(
                            input,
                            ActionOpportunityDisposition::Failed,
                        )
                        .map_err(|_| RuntimeEvaluationError::Integrity)?,
                    }
                } else {
                    return Err(RuntimeEvaluationError::Integrity);
                }
            }
            MomentWorkInput::ActionEvaluationResultReady {
                snapshot,
                result_ready,
                opportunity,
                invocation,
                ..
            } => {
                let Some(descriptor) = execution.lifecycle.action_execution().deferred_descriptor()
                else {
                    return Err(RuntimeEvaluationError::Integrity);
                };
                let decision = ActionCoordinator::resolve_deferred(
                    snapshot,
                    opportunity,
                    result_ready,
                    invocation,
                    &execution.definitions,
                    execution.containment_actions.as_ref(),
                    execution.relocation_actions.as_ref(),
                    descriptor,
                );
                MomentWorkDecision::resolve_action_evaluation(input, decision)
                    .map_err(|_| RuntimeEvaluationError::Integrity)?
            }
            MomentWorkInput::AttemptResolved { .. } => {
                MomentWorkDecision::consume_attempt_resolution(input)
                    .map_err(|_| RuntimeEvaluationError::Integrity)?
            }
            MomentWorkInput::ActivityAdvance {
                snapshot,
                actor,
                activities,
                attempted,
                ..
            } => crate::lifecycle::advance_activity(
                input,
                snapshot,
                actor,
                activities,
                attempted,
                execution.lifecycle.activity_controller(),
            )?,
            MomentWorkInput::RelocationProcessWake { .. } => {
                MomentWorkDecision::complete_relocation_process(input)
                    .map_err(|_| RuntimeEvaluationError::Integrity)?
            }
        };
        decisions.push(decision);
    }
    execution
        .activation
        .complete_moment_proposals(prepared, decisions)
}

/// Failure of public attempt coordination rather than a modeled world outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemCommandError {
    /// The selected action was absent from the exact runtime definition set.
    DefinitionUnavailable {
        /// Missing durable action key.
        action: DefinitionKey,
    },
    /// The same action role was supplied more than once.
    DuplicateBinding {
        /// Reused role name.
        binding: BindingName,
    },
    /// A definition-required action role was not supplied.
    MissingBinding {
        /// Missing role name.
        binding: BindingName,
    },
    /// A supplied role is not declared by the selected action.
    UnexpectedBinding {
        /// Unknown role name.
        binding: BindingName,
    },
    /// A supplied value had a different kind from its declared role.
    BindingKindMismatch {
        /// Mismatched role name.
        binding: BindingName,
        /// Definition-declared kind.
        expected: ValueKind,
        /// Supplied concrete kind.
        actual: ValueKind,
    },
}

impl fmt::Display for SystemCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid system command: {self:?}")
    }
}

impl std::error::Error for SystemCommandError {}

/// Failure to capture one typed deferred action result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionEvaluationCaptureError {
    /// The bound attempt no longer exists in the authority domain.
    AttemptNotFound,
    /// The attempt has already selected an immutable terminal prefix.
    AttemptFinalized {
        /// Existing terminal selection.
        finalization: Box<RunFinalization>,
    },
    /// Another world step remains durably reserved.
    StepReserved,
    /// A capture identity was retained with different result content.
    CaptureIdReuse {
        /// Conflicting capture identity.
        capture: ActionEvaluationCaptureId,
    },
    /// No retained invocation has this identity.
    UnknownInvocation {
        /// Unknown logical invocation.
        invocation: ActionEvaluationInvocationId,
    },
    /// The invocation is known but can no longer accept a result.
    LateInvocation {
        /// Closed or already captured logical invocation.
        invocation: ActionEvaluationInvocationId,
    },
    /// The capture constructor does not match the invocation's admission mode.
    TimingModeMismatch {
        /// Admission mode fixed by the resolved execution.
        expected: DeferredActionAdmissionModeV1,
        /// Timing supplied by the capture.
        supplied: ActionEvaluationCaptureTiming,
    },
    /// A host-scheduled result is not later than invocation creation.
    EffectiveMomentNotAfterCreation {
        /// Requested result moment.
        effective: SimMoment,
        /// Invocation creation moment.
        creation: SimMoment,
    },
    /// A host-scheduled result lies behind the sealed admission frontier.
    EffectiveMomentBeforeFrontier {
        /// Requested result moment.
        effective: SimMoment,
        /// Current monotonic admission frontier.
        frontier: SimMoment,
    },
    /// Scheduling the result would exceed the complete-moment population.
    MomentPopulationExceeded {
        /// Requested effective moment.
        moment: SimMoment,
        /// Configured maximum work at one moment.
        maximum: u32,
        /// Population the atomic insertion would create.
        actual: usize,
    },
    /// Retained schemas or authority/control state violate the engine protocol.
    Integrity,
    /// The authority service could not be accessed.
    Unavailable,
}

impl fmt::Display for ActionEvaluationCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "action evaluation capture failed: {self:?}")
    }
}

impl std::error::Error for ActionEvaluationCaptureError {}

/// Failure of public attempt coordination rather than a modeled world outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttemptError {
    /// The requested action or bindings do not match the exact definition set.
    InvalidSystemCommand(Box<SystemCommandError>),
    /// The bound attempt no longer exists in the authority domain.
    AttemptNotFound,
    /// The attempt has already selected an immutable terminal prefix.
    AttemptFinalized(Box<RunFinalization>),
    /// Another world step remains durably reserved.
    StepReserved,
    /// An input identity was retained with different request content.
    InputIdReuse,
    /// The input identity lies behind its retained retirement frontier.
    InputRetired {
        /// Expired input-request identity.
        id: InputId,
    },
    /// A management identity was retained with a different operation.
    ManagementIdReuse,
    /// The management identity lies behind its retained retirement frontier.
    ManagementRetired {
        /// Expired management-request identity.
        id: ManagementRequestId,
    },
    /// A cancellation identity was retained with different request content.
    CancellationIdReuse {
        /// Conflicting attempt-control identity.
        id: CancelAttemptRequestId,
    },
    /// A world-ledger retirement target did not advance its namespace frontier.
    RetirementNotAdvancing {
        /// Exact namespace and target requested by the management operation.
        retirement: LedgerRetirement,
        /// Existing namespace-local frontier.
        retired_through: u64,
    },
    /// A world-ledger retirement target crossed an unresolved identity.
    RetirementGap {
        /// Exact namespace and target requested by the management operation.
        retirement: LedgerRetirement,
        /// First unresolved identity in the requested prefix.
        missing: u64,
    },
    /// A management request tried to retire its own exact-retry witness.
    ManagementRetirementTargetNotBeforeRequest {
        /// Requested management-ledger prefix endpoint.
        target: ManagementRequestId,
        /// Carrying management request.
        request: ManagementRequestId,
    },
    /// An admission seal did not advance the current ingress frontier.
    AdmissionFrontierNotAdvancing {
        /// Current first admissible moment.
        current: SimMoment,
        /// Requested first admissible moment.
        requested: SimMoment,
    },
    /// An admission seal would cross unresolved scheduled work.
    AdmissionSealCrossesScheduledWork {
        /// Requested first admissible moment.
        requested: SimMoment,
        /// Earliest scheduled work that must be resolved first.
        scheduled: SimMoment,
    },
    /// A retained action evaluation prevents simulation from crossing its frontier.
    ActionEvaluationFrontierBlocked {
        /// Minimum unresolved frontier-blocking evaluation boundary.
        blocked_at: SimMoment,
    },
    /// The requested effective moment lies behind the sealed admission frontier.
    EffectiveMomentBeforeFrontier {
        /// Requested effective moment.
        effective: SimMoment,
        /// Current monotonic admission frontier.
        frontier: SimMoment,
    },
    /// Admission would exceed the configured complete-moment population.
    MomentPopulationExceeded {
        /// Requested effective moment.
        moment: SimMoment,
        /// Configured maximum work at one moment.
        maximum: u32,
        /// Population the atomic insertion would create.
        actual: usize,
    },
    /// The requested management transition is illegal in the current mode.
    IllegalManagement {
        /// Current session mode.
        current: SessionMode,
    },
    /// The session mode does not permit ordinary moment execution.
    SessionNotRunning {
        /// Current session mode.
        current: SessionMode,
    },
    /// A prepared capability no longer names the live reservation.
    ReservationMismatch,
    /// Submitted decisions do not exactly cover the prepared work.
    ProposalMismatch,
    /// A trusted semantic implementation violated its bounded contract.
    EvaluationContractViolation,
    /// Retained authority and control state violate the engine protocol.
    Integrity,
    /// The authority service could not be accessed.
    Unavailable,
}

impl fmt::Display for AttemptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSystemCommand(error) => error.fmt(formatter),
            other => write!(formatter, "attempt operation failed: {other:?}"),
        }
    }
}

impl std::error::Error for AttemptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidSystemCommand(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

fn map_system_command_error(error: CommandEnvelopeError) -> SystemCommandError {
    match error {
        CommandEnvelopeError::DefinitionUnavailable { action } => {
            SystemCommandError::DefinitionUnavailable { action }
        }
        CommandEnvelopeError::DuplicateBinding { binding } => {
            SystemCommandError::DuplicateBinding { binding }
        }
        CommandEnvelopeError::MissingBinding { binding } => {
            SystemCommandError::MissingBinding { binding }
        }
        CommandEnvelopeError::UnexpectedBinding { binding } => {
            SystemCommandError::UnexpectedBinding { binding }
        }
        CommandEnvelopeError::BindingKindMismatch {
            binding,
            expected,
            actual,
        } => SystemCommandError::BindingKindMismatch {
            binding,
            expected,
            actual,
        },
    }
}

fn map_drive_error(error: RuntimeDriveError) -> AttemptError {
    match error {
        RuntimeDriveError::AttemptNotFound => AttemptError::AttemptNotFound,
        RuntimeDriveError::AttemptFinalized { finalization } => {
            AttemptError::AttemptFinalized(Box::new(finalization))
        }
        RuntimeDriveError::StepReserved => AttemptError::StepReserved,
        RuntimeDriveError::InputIdReuse => AttemptError::InputIdReuse,
        RuntimeDriveError::InputRetired { id } => AttemptError::InputRetired { id },
        RuntimeDriveError::ManagementIdReuse => AttemptError::ManagementIdReuse,
        RuntimeDriveError::ManagementRetired { id } => AttemptError::ManagementRetired { id },
        RuntimeDriveError::RetirementNotAdvancing {
            retirement,
            retired_through,
        } => AttemptError::RetirementNotAdvancing {
            retirement,
            retired_through,
        },
        RuntimeDriveError::RetirementGap {
            retirement,
            missing,
        } => AttemptError::RetirementGap {
            retirement,
            missing,
        },
        RuntimeDriveError::ManagementRetirementTargetNotBeforeRequest { target, request } => {
            AttemptError::ManagementRetirementTargetNotBeforeRequest { target, request }
        }
        RuntimeDriveError::AdmissionFrontierNotAdvancing { current, requested } => {
            AttemptError::AdmissionFrontierNotAdvancing { current, requested }
        }
        RuntimeDriveError::AdmissionSealCrossesScheduledWork {
            requested,
            scheduled,
        } => AttemptError::AdmissionSealCrossesScheduledWork {
            requested,
            scheduled,
        },
        RuntimeDriveError::ActionEvaluationFrontierBlocked { blocked_at } => {
            AttemptError::ActionEvaluationFrontierBlocked { blocked_at }
        }
        RuntimeDriveError::EffectiveMomentBeforeFrontier {
            effective,
            frontier,
        } => AttemptError::EffectiveMomentBeforeFrontier {
            effective,
            frontier,
        },
        RuntimeDriveError::MomentPopulationExceeded {
            moment,
            maximum,
            actual,
        } => AttemptError::MomentPopulationExceeded {
            moment,
            maximum,
            actual,
        },
        RuntimeDriveError::IllegalManagement { current } => {
            AttemptError::IllegalManagement { current }
        }
        RuntimeDriveError::SessionNotRunning { current } => {
            AttemptError::SessionNotRunning { current }
        }
        RuntimeDriveError::PreparedFireMismatch
        | RuntimeDriveError::PreparedKernelSafetyMismatch => AttemptError::ReservationMismatch,
        RuntimeDriveError::ProposalMismatch => AttemptError::ProposalMismatch,
        RuntimeDriveError::Integrity
        | RuntimeDriveError::NoScheduledWork
        | RuntimeDriveError::NoWorkDue { .. } => AttemptError::Integrity,
        RuntimeDriveError::Unavailable => AttemptError::Unavailable,
    }
}

fn map_control_error(error: RuntimeControlError) -> AttemptError {
    match error {
        RuntimeControlError::AttemptNotFound => AttemptError::AttemptNotFound,
        RuntimeControlError::AttemptFinalized { finalization } => {
            AttemptError::AttemptFinalized(Box::new(finalization))
        }
        RuntimeControlError::StepReserved => AttemptError::StepReserved,
        RuntimeControlError::CancellationIdReuse { id } => AttemptError::CancellationIdReuse { id },
        RuntimeControlError::PreparedFireMismatch => AttemptError::ReservationMismatch,
        RuntimeControlError::Integrity => AttemptError::Integrity,
        RuntimeControlError::Unavailable => AttemptError::Unavailable,
    }
}

fn map_read_error(error: RuntimeReadError) -> AttemptError {
    match error {
        RuntimeReadError::AttemptNotFound => AttemptError::AttemptNotFound,
        RuntimeReadError::Unavailable => AttemptError::Unavailable,
    }
}

fn map_action_evaluation_capture_error(
    error: RuntimeActionEvaluationCaptureError,
) -> ActionEvaluationCaptureError {
    match error {
        RuntimeActionEvaluationCaptureError::AttemptNotFound => {
            ActionEvaluationCaptureError::AttemptNotFound
        }
        RuntimeActionEvaluationCaptureError::AttemptFinalized { finalization } => {
            ActionEvaluationCaptureError::AttemptFinalized {
                finalization: Box::new(finalization),
            }
        }
        RuntimeActionEvaluationCaptureError::StepReserved => {
            ActionEvaluationCaptureError::StepReserved
        }
        RuntimeActionEvaluationCaptureError::CaptureIdReuse { capture } => {
            ActionEvaluationCaptureError::CaptureIdReuse { capture }
        }
        RuntimeActionEvaluationCaptureError::UnknownInvocation { invocation } => {
            ActionEvaluationCaptureError::UnknownInvocation { invocation }
        }
        RuntimeActionEvaluationCaptureError::LateInvocation { invocation } => {
            ActionEvaluationCaptureError::LateInvocation { invocation }
        }
        RuntimeActionEvaluationCaptureError::TimingModeMismatch { expected, supplied } => {
            ActionEvaluationCaptureError::TimingModeMismatch { expected, supplied }
        }
        RuntimeActionEvaluationCaptureError::EffectiveMomentNotAfterCreation {
            effective,
            creation,
        } => ActionEvaluationCaptureError::EffectiveMomentNotAfterCreation {
            effective,
            creation,
        },
        RuntimeActionEvaluationCaptureError::EffectiveMomentBeforeFrontier {
            effective,
            frontier,
        } => ActionEvaluationCaptureError::EffectiveMomentBeforeFrontier {
            effective,
            frontier,
        },
        RuntimeActionEvaluationCaptureError::MomentPopulationExceeded {
            moment,
            maximum,
            actual,
        } => ActionEvaluationCaptureError::MomentPopulationExceeded {
            moment,
            maximum,
            actual,
        },
        RuntimeActionEvaluationCaptureError::ResultSchemaMismatch { .. }
        | RuntimeActionEvaluationCaptureError::Integrity => ActionEvaluationCaptureError::Integrity,
        RuntimeActionEvaluationCaptureError::Unavailable => {
            ActionEvaluationCaptureError::Unavailable
        }
    }
}

fn map_evaluation_error(error: RuntimeEvaluationError) -> AttemptError {
    match error {
        RuntimeEvaluationError::ImplementationContractViolation => {
            AttemptError::EvaluationContractViolation
        }
        RuntimeEvaluationError::ActivationMismatch
        | RuntimeEvaluationError::WorkKindMismatch
        | RuntimeEvaluationError::Integrity => AttemptError::Integrity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_errors_preserve_client_actions_and_hide_schema_corruption() {
        let capture = ActionEvaluationCaptureId::new(9);
        let invocation = ActionEvaluationInvocationId::from_bytes([0x91; 32]);
        assert_eq!(
            map_action_evaluation_capture_error(
                RuntimeActionEvaluationCaptureError::CaptureIdReuse { capture }
            ),
            ActionEvaluationCaptureError::CaptureIdReuse { capture }
        );
        assert_eq!(
            map_action_evaluation_capture_error(
                RuntimeActionEvaluationCaptureError::UnknownInvocation { invocation }
            ),
            ActionEvaluationCaptureError::UnknownInvocation { invocation }
        );
        assert_eq!(
            map_action_evaluation_capture_error(
                RuntimeActionEvaluationCaptureError::TimingModeMismatch {
                    expected: DeferredActionAdmissionModeV1::FrontierBlocking,
                    supplied: ActionEvaluationCaptureTiming::HostScheduled(SimMoment::ORIGIN),
                }
            ),
            ActionEvaluationCaptureError::TimingModeMismatch {
                expected: DeferredActionAdmissionModeV1::FrontierBlocking,
                supplied: ActionEvaluationCaptureTiming::HostScheduled(SimMoment::ORIGIN),
            }
        );

        let schema = world_runtime::ActionEvaluationArtifactSchemaId::from_bytes([0x92; 32]);
        assert_eq!(
            map_action_evaluation_capture_error(
                RuntimeActionEvaluationCaptureError::ResultSchemaMismatch {
                    expected: schema,
                    actual: schema,
                }
            ),
            ActionEvaluationCaptureError::Integrity
        );
    }

    #[test]
    fn deferred_frontier_blocker_remains_an_actionable_public_error() {
        let blocked_at = SimMoment::ORIGIN;
        assert_eq!(
            map_drive_error(RuntimeDriveError::ActionEvaluationFrontierBlocked { blocked_at }),
            AttemptError::ActionEvaluationFrontierBlocked { blocked_at }
        );
    }
}
