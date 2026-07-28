use std::sync::Arc;

use world_core::SimMoment;
use world_model::WorldSnapshot;

use crate::action_evaluation::{
    ActionEvaluationArtifactSchemaId, ActionEvaluationCaptureId, ActionEvaluationCaptureOutcome,
    ActionEvaluationCaptureTiming, ActionEvaluationResultSubmission, PendingActionEvaluationRaw,
};
use crate::attempt::{
    AttemptKey, CancelAttemptOutcome, CancelAttemptRequest, RunAttemptId, RunFinalization,
};
use crate::execution::ResolvedExecutionClosureManifestV1;
use crate::execution::{
    ActivatedCommandEvaluation, ActivatedRuntimeExecution, ContainmentTransferImplementation,
    DeferredActionAdmissionModeV1, LifecycleProfilesV2, OriginExecutionInput,
    RelocationActionImplementation, RuntimeActivationError,
};
use crate::kernel::{
    AdmitOutcome, AdmitRequest, CommandProposal, FireOutcome, FirePreparation, FireRequest,
    KernelSafetyBlocker, KernelSafetyOutcome, ManageOutcome, ManageRequest, MomentWorkDecision,
    MomentWorkInput, MomentWorkProposals, PreparedFire, PreparedFireFailure,
    PreparedFireFailureOutcome, PreparedKernelSafety,
};
use crate::persistence::MemoryRepository;
use crate::session::{SameTimeWaveTranche, SessionMode};

/// Cloneable process capability over one runtime authority domain.
#[derive(Clone)]
pub struct RuntimeService {
    repository: Arc<MemoryRepository>,
}

impl RuntimeService {
    /// Creates one process-local authoritative in-memory runtime service.
    pub fn in_memory() -> Result<Self, RuntimeStartError> {
        Ok(Self {
            repository: Arc::new(MemoryRepository::new()?),
        })
    }

    /// Mints one sealed origin activation from verified definitions and trusted semantics.
    pub fn activate_origin(
        &self,
        definitions: world_defs::RuntimeDefinitionSet,
        transfer: Option<ContainmentTransferImplementation>,
        relocation: Option<RelocationActionImplementation>,
        lifecycle_profiles: LifecycleProfilesV2,
        input: OriginExecutionInput,
    ) -> Result<ActivatedRuntimeExecution, RuntimeActivationError> {
        ActivatedRuntimeExecution::origin(
            definitions,
            transfer,
            relocation,
            lifecycle_profiles,
            input,
        )
    }

    /// Starts or idempotently opens an attempt for one sealed activation.
    pub fn start_attempt(
        &self,
        activation: &ActivatedRuntimeExecution,
        key: AttemptKey,
    ) -> Result<RuntimeAttemptDriver, RuntimeStartError> {
        self.start_attempt_with_closure(activation.closure().clone(), key)
    }

    pub(crate) fn start_attempt_with_closure(
        &self,
        closure: ResolvedExecutionClosureManifestV1,
        key: AttemptKey,
    ) -> Result<RuntimeAttemptDriver, RuntimeStartError> {
        let opened = self.repository.create_or_open(closure, key)?;
        Ok(RuntimeAttemptDriver {
            repository: Arc::clone(&self.repository),
            attempt: opened.attempt(),
            binding: opened.binding(),
        })
    }
}

impl ActivatedRuntimeExecution {
    /// Evaluates one command item through this activation's private definition registry.
    pub fn evaluate_command_work(
        &self,
        input: MomentWorkInput<'_>,
    ) -> Result<MomentWorkDecision, RuntimeEvaluationError> {
        let MomentWorkInput::EvaluateCommand {
            work,
            snapshot,
            command,
            ..
        } = input
        else {
            return Err(RuntimeEvaluationError::WorkKindMismatch);
        };
        let proposal = match self.evaluate(snapshot, command) {
            ActivatedCommandEvaluation::Rejected(reason) => CommandProposal::Rejected(reason),
            ActivatedCommandEvaluation::AcceptedTransfer(delta) => {
                CommandProposal::AcceptedTransfer(delta)
            }
            ActivatedCommandEvaluation::ImplementationContractViolation => {
                return Err(RuntimeEvaluationError::ImplementationContractViolation);
            }
        };
        Ok(MomentWorkDecision::command(work, proposal))
    }

    /// Validates complete proposal coverage against this exact execution activation.
    pub fn complete_moment_proposals(
        &self,
        prepared: &PreparedFire,
        decisions: Vec<MomentWorkDecision>,
    ) -> Result<MomentWorkProposals, RuntimeEvaluationError> {
        if prepared.execution() != self.execution_id() {
            return Err(RuntimeEvaluationError::ActivationMismatch);
        }
        MomentWorkProposals::from_decisions(prepared, decisions)
            .map_err(|_| RuntimeEvaluationError::Integrity)
    }
}

/// Non-cloneable mutation capability bound to one physical attempt.
pub struct RuntimeAttemptDriver {
    repository: Arc<MemoryRepository>,
    attempt: RunAttemptId,
    binding: crate::attempt::AttemptBinding,
}

#[allow(
    clippy::result_large_err,
    reason = "the public driver returns complete retained finalization evidence on terminal replay"
)]
impl RuntimeAttemptDriver {
    /// Returns the immutable physical attempt identity.
    #[must_use]
    pub const fn attempt_id(&self) -> RunAttemptId {
        self.attempt
    }

    /// Returns the permanent execution correspondence for this attempt.
    #[must_use]
    pub const fn binding(&self) -> crate::attempt::AttemptBinding {
        self.binding
    }

    /// Reads the current durable attempt-control phase.
    pub fn status(&self) -> Result<RuntimeAttemptStatus, RuntimeReadError> {
        self.repository
            .read(self.attempt)
            .map(|read| *read.status())
    }

    /// Creates a cloneable read-only capability for the same session.
    #[must_use]
    pub fn session_reader(&self) -> RuntimeSessionReader {
        RuntimeSessionReader {
            repository: Arc::clone(&self.repository),
            attempt: self.attempt,
        }
    }

    /// Copies the currently dispatchable deferred action-evaluation requests.
    pub fn pending_action_evaluations(
        &self,
    ) -> Result<Vec<PendingActionEvaluationRaw>, RuntimeReadError> {
        self.repository
            .read(self.attempt)
            .map(|read| read.pending_action_evaluations().to_vec())
    }

    /// Atomically admits one typed deferred action-evaluation result.
    pub fn capture_action_evaluation_result(
        &mut self,
        submission: ActionEvaluationResultSubmission,
    ) -> Result<ActionEvaluationCaptureOutcome, RuntimeActionEvaluationCaptureError> {
        self.repository
            .capture_action_evaluation_result(self.attempt, submission)
    }

    /// Captures and schedules one checked command through an atomic ingress record.
    pub fn admit(&mut self, request: AdmitRequest) -> Result<AdmitOutcome, RuntimeDriveError> {
        self.repository.admit(self.attempt, request)
    }

    /// Preflights the globally least due moment and reserves its exact next operation.
    pub fn prepare_fire(
        &mut self,
        request: FireRequest,
    ) -> Result<FirePreparation, RuntimeDriveError> {
        self.repository.prepare_fire(self.attempt, request)
    }

    /// Consumes one prepared token and publishes its exact bounded proposal.
    pub fn complete_fire(
        &mut self,
        prepared: PreparedFire,
        proposals: MomentWorkProposals,
    ) -> Result<FireOutcome, RuntimeDriveError> {
        self.repository
            .complete_fire(self.attempt, prepared, proposals)
    }

    /// Publishes one reserved deterministic safety transition without consuming due work.
    pub fn complete_kernel_safety(
        &mut self,
        prepared: PreparedKernelSafety,
    ) -> Result<KernelSafetyOutcome, RuntimeDriveError> {
        self.repository
            .complete_kernel_safety(self.attempt, prepared)
    }

    /// Records a typed evaluation failure without changing the world session.
    pub fn fail_prepared_fire(
        &mut self,
        prepared: PreparedFire,
        failure: PreparedFireFailure,
    ) -> Result<PreparedFireFailureOutcome, RuntimeControlError> {
        self.repository
            .fail_prepared_fire(self.attempt, prepared, failure)
    }

    /// Applies one singular session-management request.
    pub fn manage(&mut self, request: ManageRequest) -> Result<ManageOutcome, RuntimeDriveError> {
        self.repository.manage(self.attempt, request)
    }

    /// Selects the current reconciled prefix as terminal for this attempt.
    pub fn cancel_attempt(
        &mut self,
        request: CancelAttemptRequest,
    ) -> Result<CancelAttemptOutcome, RuntimeControlError> {
        self.repository.cancel_attempt(self.attempt, request)
    }
}

/// Cloneable capability that can copy only the current aggregate read model.
#[derive(Clone)]
pub struct RuntimeSessionReader {
    repository: Arc<MemoryRepository>,
    attempt: RunAttemptId,
}

impl RuntimeSessionReader {
    /// Copies one atomic read model from the authoritative aggregate.
    pub fn read(&self) -> Result<RuntimeSessionRead, RuntimeReadError> {
        self.repository
            .read(self.attempt)
            .map(|read| RuntimeSessionRead {
                cursor: read.cursor(),
                mode: read.mode(),
                admission_frontier: read.admission_frontier(),
                snapshot: read.snapshot().clone(),
                safety_blocker: read.safety_blocker(),
                same_time_wave_tranche: read.same_time_wave_tranche(),
            })
    }

    /// Copies the current authority cursor.
    pub fn cursor(&self) -> Result<crate::authority::AuthorityCursor, RuntimeReadError> {
        self.read().map(|read| read.cursor())
    }

    /// Copies one immutable snapshot.
    pub fn snapshot(&self) -> Result<WorldSnapshot, RuntimeReadError> {
        self.read().map(|read| read.snapshot().clone())
    }
}

/// One atomic, read-only image of the authoritative session head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeSessionRead {
    cursor: crate::authority::AuthorityCursor,
    mode: SessionMode,
    admission_frontier: SimMoment,
    snapshot: WorldSnapshot,
    safety_blocker: Option<KernelSafetyBlocker>,
    same_time_wave_tranche: SameTimeWaveTranche,
}

impl RuntimeSessionRead {
    /// Returns the authority cursor read with this image.
    #[must_use]
    pub const fn cursor(&self) -> crate::authority::AuthorityCursor {
        self.cursor
    }

    /// Returns the session mode read with this image.
    #[must_use]
    pub const fn mode(&self) -> SessionMode {
        self.mode
    }

    /// Returns the first simulation moment open to new command ingress.
    #[must_use]
    pub const fn admission_frontier(&self) -> SimMoment {
        self.admission_frontier
    }

    /// Returns the immutable world snapshot read with this image.
    #[must_use]
    pub const fn snapshot(&self) -> &WorldSnapshot {
        &self.snapshot
    }

    /// Returns the deterministic cause retaining unresolved ordinary work.
    #[must_use]
    pub const fn safety_blocker(&self) -> Option<KernelSafetyBlocker> {
        self.safety_blocker
    }

    /// Returns published-wave accounting for the current simulation-time tranche.
    #[must_use]
    pub const fn same_time_wave_tranche(&self) -> SameTimeWaveTranche {
        self.same_time_wave_tranche
    }
}

/// Failure of attempt construction or same-domain reopening.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeStartError {
    /// The process exhausted its in-memory authority-domain ordinals.
    AuthorityDomainExhausted,
    /// The same attempt identity was reopened with another creation value.
    AttemptCreationConflict,
    /// No attempt with the requested identity exists.
    AttemptNotFound,
    /// Retained state violates the authority/control correspondence.
    Integrity,
    /// The in-memory aggregate lock was poisoned.
    Unavailable,
}

/// Failure of one world-transition request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeDriveError {
    /// No attempt with the driver's identity exists.
    AttemptNotFound,
    /// The attempt has already selected a terminal prefix.
    AttemptFinalized {
        /// Immutable existing terminal selection.
        finalization: RunFinalization,
    },
    /// Another step remains durably reserved.
    StepReserved,
    /// An input ID was retained with different request content.
    InputIdReuse,
    /// The input identity lies behind its retained retirement frontier.
    InputRetired {
        /// Expired input-request identity.
        id: crate::kernel::InputId,
    },
    /// A management ID was retained with another operation.
    ManagementIdReuse,
    /// The management identity lies behind its retained retirement frontier.
    ManagementRetired {
        /// Expired management-request identity.
        id: crate::kernel::ManagementRequestId,
    },
    /// A retirement target did not advance its namespace frontier.
    RetirementNotAdvancing {
        /// Exact namespace and target requested by the management operation.
        retirement: crate::kernel::LedgerRetirement,
        /// Existing source-local retirement frontier.
        retired_through: u64,
    },
    /// A retirement target crossed an identity without a terminal outcome.
    RetirementGap {
        /// Exact namespace and target requested by the management operation.
        retirement: crate::kernel::LedgerRetirement,
        /// First unresolved identity in the requested prefix.
        missing: u64,
    },
    /// A management retirement target includes its carrying request.
    ManagementRetirementTargetNotBeforeRequest {
        /// Requested management-ledger prefix endpoint.
        target: crate::kernel::ManagementRequestId,
        /// Request that would lose its exact-retry witness.
        request: crate::kernel::ManagementRequestId,
    },
    /// An admission seal did not advance the current ingress frontier.
    AdmissionFrontierNotAdvancing {
        /// Current first admissible simulation moment.
        current: SimMoment,
        /// Requested first admissible simulation moment.
        requested: SimMoment,
    },
    /// An admission seal would cross unresolved scheduled work.
    AdmissionSealCrossesScheduledWork {
        /// Requested first admissible simulation moment.
        requested: SimMoment,
        /// Earliest scheduled work that must be resolved first.
        scheduled: SimMoment,
    },
    /// A pending frontier-blocking action evaluation prevents time from advancing.
    ActionEvaluationFrontierBlocked {
        /// Earliest live frontier barrier across pending action evaluations.
        blocked_at: SimMoment,
    },
    /// The requested effective moment lies behind the sealed admission frontier.
    EffectiveMomentBeforeFrontier {
        /// Requested effective moment.
        effective: SimMoment,
        /// Current monotonic admission frontier.
        frontier: SimMoment,
    },
    /// Admitting this request would exceed the configured work population.
    MomentPopulationExceeded {
        /// Requested effective moment.
        moment: SimMoment,
        /// Configured maximum work at one moment.
        maximum: u32,
        /// Population that the atomic insertion would create.
        actual: usize,
    },
    /// The requested management transition is illegal in the current mode.
    IllegalManagement {
        /// Current session mode.
        current: SessionMode,
    },
    /// No scheduled work exists.
    NoScheduledWork,
    /// The least scheduled item lies beyond the requested bound.
    NoWorkDue {
        /// Least scheduled moment.
        next: SimMoment,
        /// Inclusive caller bound.
        through: SimMoment,
    },
    /// The token does not name the repository's live reservation.
    PreparedFireMismatch,
    /// The safety capability does not name the repository's live reservation.
    PreparedKernelSafetyMismatch,
    /// The proposal is missing or bound to another prepared command.
    ProposalMismatch,
    /// The session mode does not permit moment execution.
    SessionNotRunning {
        /// Current session mode.
        current: SessionMode,
    },
    /// Retained state violates the authority/control correspondence.
    Integrity,
    /// The in-memory aggregate lock was poisoned.
    Unavailable,
}

/// Failure to admit one deferred action-evaluation result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeActionEvaluationCaptureError {
    /// No attempt with the driver's identity exists.
    AttemptNotFound,
    /// The attempt has already selected a terminal prefix.
    AttemptFinalized {
        /// Immutable existing terminal selection.
        finalization: RunFinalization,
    },
    /// Another world step remains durably reserved.
    StepReserved,
    /// A capture identity is already retained with different content.
    CaptureIdReuse {
        /// Conflicting capture namespace identity.
        capture: ActionEvaluationCaptureId,
    },
    /// No retained invocation has the submitted identity.
    UnknownInvocation {
        /// Missing action-evaluation invocation.
        invocation: world_model::ActionEvaluationInvocationId,
    },
    /// The retained invocation can no longer accept a result.
    LateInvocation {
        /// Known invocation that is no longer dispatch-pending.
        invocation: world_model::ActionEvaluationInvocationId,
    },
    /// Capture timing does not match the invocation's admission mode.
    TimingModeMismatch {
        /// Admission mode fixed by the invocation.
        expected: DeferredActionAdmissionModeV1,
        /// Timing form supplied by the host.
        supplied: ActionEvaluationCaptureTiming,
    },
    /// The result artifact uses a schema other than the invocation's fixed schema.
    ResultSchemaMismatch {
        /// Schema fixed when the invocation was created.
        expected: ActionEvaluationArtifactSchemaId,
        /// Schema supplied by the capture.
        actual: ActionEvaluationArtifactSchemaId,
    },
    /// Host-scheduled capture time is not later than invocation creation.
    EffectiveMomentNotAfterCreation {
        /// Submitted effective simulation moment.
        effective: SimMoment,
        /// Invocation creation simulation moment.
        creation: SimMoment,
    },
    /// Host-scheduled capture time lies behind the admission frontier.
    EffectiveMomentBeforeFrontier {
        /// Submitted effective simulation moment.
        effective: SimMoment,
        /// Current admission frontier.
        frontier: SimMoment,
    },
    /// Admitting fallback or result work would exceed the configured moment population.
    MomentPopulationExceeded {
        /// Effective simulation moment.
        moment: SimMoment,
        /// Configured maximum work at the moment.
        maximum: u32,
        /// Population the atomic admission would create.
        actual: usize,
    },
    /// Retained state violates the authority or capture protocol.
    Integrity,
    /// The in-memory aggregate lock was poisoned.
    Unavailable,
}

/// Failure of one attempt-control-only request.
#[allow(
    clippy::large_enum_variant,
    reason = "the public control error retains complete finalization evidence and remains a copyable value API"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeControlError {
    /// No attempt with the driver's identity exists.
    AttemptNotFound,
    /// The attempt has already selected a terminal prefix.
    AttemptFinalized {
        /// Immutable existing terminal selection.
        finalization: RunFinalization,
    },
    /// A world step is reserved and must be completed or reconciled first.
    StepReserved,
    /// A retained cancellation identity was reused with different content.
    CancellationIdReuse {
        /// Conflicting attempt-control request identity.
        id: crate::attempt::CancelAttemptRequestId,
    },
    /// The token does not name the repository's live reservation.
    PreparedFireMismatch,
    /// The retained aggregate violates the control protocol.
    Integrity,
    /// The in-memory aggregate lock was poisoned.
    Unavailable,
}

/// Failure of a read-only aggregate copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeReadError {
    /// No attempt with the reader's identity exists.
    AttemptNotFound,
    /// The in-memory aggregate lock was poisoned.
    Unavailable,
}

/// Read-only projection of one attempt's durable control phase.
#[allow(
    clippy::large_enum_variant,
    reason = "the public status projection retains complete finalization evidence and remains a copyable value API"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeAttemptStatus {
    /// The attempt can reserve another operation.
    Active,
    /// One operation remains durably reserved.
    Reserved,
    /// The attempt selected an immutable terminal prefix.
    Finalized(RunFinalization),
}

/// Failure to derive a bounded proposal from one prepared command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeEvaluationError {
    /// The prepared command belongs to another activated execution.
    ActivationMismatch,
    /// A command evaluator was asked to handle another prepared work family.
    WorkKindMismatch,
    /// A trusted implementation returned an effect outside its bound input.
    ImplementationContractViolation,
    /// Runtime-owned prepared-input invariants were inconsistent.
    Integrity,
}
