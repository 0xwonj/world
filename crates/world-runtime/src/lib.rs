//! Authoritative execution protocols for the simulation engine.
//!
//! This crate owns execution identity, session authority, and publication
//! protocols. Immutable model values remain owned by `world-model`; compiled
//! definitions remain owned by `world-defs`.
//!
//! Authority-bearing implementation types are not externally nameable:
//!
//! ```compile_fail
//! use world_runtime::authority::SealedAuthorityRecord;
//! ```
//!
//! Mutation and staged-publication capabilities are deliberately single-owner
//! process values:
//!
//! ```compile_fail
//! let _ = world_runtime::RuntimeService {};
//! ```
//!
//! ```compile_fail
//! let _ = world_runtime::RuntimeAttemptDriver {};
//! ```
//!
//! ```compile_fail
//! use world_runtime::RuntimeAttemptDriver;
//! fn assert_clone<T: Clone>() {}
//! assert_clone::<RuntimeAttemptDriver>();
//! ```
//!
//! ```compile_fail
//! use world_runtime::PreparedFire;
//! fn assert_clone<T: Clone>() {}
//! assert_clone::<PreparedFire>();
//! ```
//!
//! ```compile_fail
//! use world_runtime::PreparedKernelSafety;
//! fn assert_clone<T: Clone>() {}
//! assert_clone::<PreparedKernelSafety>();
//! ```
//!
//! Opaque capabilities do not reveal repository or reservation evidence
//! through derived formatting or equality:
//!
//! ```compile_fail
//! use core::fmt::Debug;
//! use world_runtime::RuntimeService;
//! fn assert_debug<T: Debug>() {}
//! assert_debug::<RuntimeService>();
//! ```
//!
//! ```compile_fail
//! use core::fmt::Debug;
//! use world_runtime::RuntimeAttemptDriver;
//! fn assert_debug<T: Debug>() {}
//! assert_debug::<RuntimeAttemptDriver>();
//! ```
//!
//! ```compile_fail
//! use core::fmt::Debug;
//! use world_runtime::RuntimeSessionReader;
//! fn assert_debug<T: Debug>() {}
//! assert_debug::<RuntimeSessionReader>();
//! ```
//!
//! ```compile_fail
//! use core::fmt::Debug;
//! use world_runtime::PreparedFire;
//! fn assert_debug<T: Debug>() {}
//! assert_debug::<PreparedFire>();
//! ```
//!
//! ```compile_fail
//! use core::fmt::Debug;
//! use world_runtime::PreparedKernelSafety;
//! fn assert_debug<T: Debug>() {}
//! assert_debug::<PreparedKernelSafety>();
//! ```
//!
//! ```compile_fail
//! use core::fmt::Debug;
//! use world_runtime::MomentWorkProposals;
//! fn assert_debug<T: Debug>() {}
//! assert_debug::<MomentWorkProposals>();
//! ```
//!
//! ```compile_fail
//! use world_runtime::PreparedFire;
//! fn assert_equality<T: PartialEq>() {}
//! assert_equality::<PreparedFire>();
//! ```
//!
//! Activated semantics are runtime-minted and cannot be assembled or cloned
//! into an independently remixable execution:
//!
//! ```compile_fail
//! let _ = world_runtime::ActivatedRuntimeExecution {};
//! ```
//!
//! ```compile_fail
//! use world_runtime::ActivatedRuntimeExecution;
//! fn assert_clone<T: Clone>() {}
//! assert_clone::<ActivatedRuntimeExecution>();
//! ```
//!
//! Typed semantic input is readable but not externally constructible:
//!
//! ```compile_fail
//! use world_runtime::ContainmentTransferInput;
//! let _ = ContainmentTransferInput {
//!     snapshot: todo!(),
//!     actor: todo!(),
//!     item: todo!(),
//!     source: todo!(),
//!     destination: todo!(),
//! };
//! ```
//!
//! Trusted transfer semantics can permit only the exact bound operation; they
//! cannot supply or redirect its state delta:
//!
//! ```compile_fail
//! let _ = world_runtime::ContainmentTransferEvaluation::Accepted(());
//! ```
//!
//! ```compile_fail
//! use world_runtime::MomentWorkProposals;
//! fn assert_equality<T: PartialEq>() {}
//! assert_equality::<MomentWorkProposals>();
//! ```

mod action_evaluation;
mod attempt;
mod authority;
mod control;
mod execution;
mod kernel;
mod lifecycle;
mod persistence;
mod randomness;
mod relocation;
mod scheduler;
mod service;
mod session;

pub use action_evaluation::{
    ActionEvaluationArtifactDigest, ActionEvaluationArtifactError, ActionEvaluationArtifactFailure,
    ActionEvaluationArtifactRole, ActionEvaluationArtifactSchemaId,
    ActionEvaluationCaptureFingerprint, ActionEvaluationCaptureId, ActionEvaluationCaptureOutcome,
    ActionEvaluationCaptureTiming, ActionEvaluationDispatchPayload, ActionEvaluationFallbackCause,
    ActionEvaluationInvocationCause, ActionEvaluationInvocationDigest,
    ActionEvaluationInvocationError, ActionEvaluationInvocationPayload,
    ActionEvaluationInvocationRecord, ActionEvaluationInvocationState,
    ActionEvaluationPrivateContinuationArtifact, ActionEvaluationPrivateReadWitnessArtifact,
    ActionEvaluationRequestArtifact, ActionEvaluationRequestId, ActionEvaluationResultArtifact,
    ActionEvaluationResultFreshness, ActionEvaluationResultId, ActionEvaluationResultReady,
    ActionEvaluationResultSubmission, ActionEvaluationTerminal, ActionEvaluationWork,
    PendingActionEvaluationRaw,
};
pub use attempt::{
    AttemptAuthorityDomainId, AttemptBinding, AttemptDispositionId, AttemptKey,
    CancelAttemptOutcome, CancelAttemptRequest, CancelAttemptRequestId, CancelReason, RunAttemptId,
    RunFinalization, RunFinalizationCause, TrajectoryId,
};
pub use authority::{
    ActionEvaluationDeliveryRecord, ActionEvaluationDeliveryRef,
    ActionEvaluationInvocationOpeningCause, ActionEvaluationInvocationOpeningRecord,
    ActionEvaluationInvocationTransitionCause, ActionEvaluationInvocationTransitionRecord,
    ActionReadyDeliveryRef, ActionResolutionDeliveryRef, AuthorityCursor, AuthorityPosition,
    AuthorityRecordAnchor, AuthorityRecordId, CumulativeAuthorityHash, EpochIdentity,
    NonZeroRunRecordSeq, RelocationAttemptRecord, RelocationAttemptRejection,
    RelocationAttemptResolution, RelocationProcessDeliveryRef, RelocationProcessTransitionCause,
    RelocationProcessTransitionRecord,
};
pub use execution::{
    ActionPolicyBindingV1, ActionPolicyExecutionV1, ActivatedContainmentTransferActions,
    ActivatedRelocationActionFamily, ActivatedRuntimeExecution, BranchTransformId,
    CanonicalExecutionSpecV1, ChildEpochTransform, ContainmentConflictPolicyV1,
    ContainmentTransferEvaluation, ContainmentTransferEvaluator, ContainmentTransferImplementation,
    ContainmentTransferInput, ContainmentTransferInstallationError, DeferredActionAdmissionModeV1,
    DeferredActionControlError, DeferredActionControlV1, DeferredActionFallbackV1, EpochLineage,
    EpochLineageBody, EpochLineageId, EpochOriginId, ExecutionConfigArtifactDigest,
    ExecutionConfigArtifactV3, ExecutionConfigError, ExecutionSemanticsManifestDigest,
    ExecutionSemanticsManifestV1, ExecutionSpecId, ExternalInputBindingDigest,
    ExternalInputBindingV1, ExternalInputNamespaceId, FinalizationPolicyV1,
    InitialBindingComponent, InitialExecutionBindingError, InitialRootError, InitialStateRootId,
    InitialStateRootV1, LifecycleBindingV1, LifecycleImplementationId, LifecycleProfilesDigest,
    LifecycleProfilesV2, LifecycleStateBindingV1, LifecycleStateSchemaId, MigrationResetId,
    MomentResolutionPolicyV2, OptionalLifecycleBindingV1, OriginExecutionInput,
    PostCommitRoutingPolicyV1, RandomKeyPolicyV1, RandomOraclePolicyV1,
    RelocationActionImplementation, RelocationActionInstallationError,
    ResolvedExecutionClosureManifestDigest, ResolvedExecutionClosureManifestV1, RootSeed,
    RuntimeActivationError, RuntimeArtifactReference, SUPPORTED_ENGINE_PROTOCOL_VERSION,
    SameTimeWaveExhaustionPolicyV1, SemanticBindingError, SemanticImplementationBinding,
    SemanticImplementationId, SemanticTerminationReasonV1, TerminationClauseId,
    TerminationContractDigest, TerminationContractV1, WorkPopulationExhaustionPolicyV1,
};
pub use kernel::{
    ActionEvaluationDecision, ActionEvaluationManagementDisposition, ActionEvaluationResultFailure,
    ActivityAdvanceResult, ActivityInitializationResult, AdmitOutcome, AdmitRequest,
    AppraisalResult, CommandFireClassification, CommandFireResolution, DeferredActionArtifactInput,
    DeferredActionInvocationInput, EvaluatedAction, FireOutcome, FirePreparation, FireRequest,
    INPUT_REQUEST_SCHEMA_VERSION, InputId, InputRequestFingerprint, IntentReviewResult,
    KERNEL_SAFETY_CAUSE_SCHEMA_VERSION, KernelSafetyBlocker, KernelSafetyCause,
    KernelSafetyDisposition, KernelSafetyDueSetEvidence, KernelSafetyOutcome,
    KernelSafetyTriggerCoordinate, KernelSafetyTriggerLane, KernelSafetyTriggerSample,
    LedgerRetirement, MANAGEMENT_REQUEST_SCHEMA_VERSION, ManageOutcome, ManageRequest,
    ManagementRequestFingerprint, ManagementRequestId, MomentWorkDecision, MomentWorkInput,
    MomentWorkProposals, PostCommitRoutingDecision, PreparedFire, PreparedFireFailure,
    PreparedFireFailureOutcome, PreparedKernelSafety, ProposalBuildError, SessionManagement,
    WorkId,
};
pub use lifecycle::EvidenceObservation;
pub use service::{
    RuntimeActionEvaluationCaptureError, RuntimeAttemptDriver, RuntimeAttemptStatus,
    RuntimeControlError, RuntimeDriveError, RuntimeEvaluationError, RuntimeReadError,
    RuntimeService, RuntimeSessionRead, RuntimeSessionReader, RuntimeStartError,
};
pub use session::{SameTimeWaveTranche, SessionMode};
