//! Public composition, attempt, and read facade for the simulation engine.
//!
//! The engine resolves exact immutable execution semantics and coordinates
//! runtime capabilities. It cannot construct a session head, authority record,
//! accepted delta proposal, reservation, or publication receipt.
//!
//! Resolved executions and mutation attempts are capabilities, not
//! caller-assembled data:
//!
//! ```compile_fail
//! let _ = world_engine::ResolvedExecution {};
//! ```
//!
//! ```compile_fail
//! use world_engine::ResolvedExecution;
//! fn assert_default<T: Default>() {}
//! assert_default::<ResolvedExecution>();
//! ```
//!
//! ```compile_fail
//! use world_engine::RunAttempt;
//! fn assert_clone<T: Clone>() {}
//! assert_clone::<RunAttempt>();
//! ```
//!
//! ```compile_fail
//! let _ = world_engine::RunAttempt {};
//! ```
//!
//! ```compile_fail
//! let _ = world_engine::WorldSession {};
//! ```
//!
//! A running attempt cannot replace the action policy bound to its resolved
//! execution:
//!
//! ```compile_fail
//! use world_engine::{ActionPolicy, AdvanceRequest, RunAttempt, SimMoment};
//!
//! fn replace_policy(attempt: &mut RunAttempt, policy: &dyn ActionPolicy) {
//!     let _ = attempt.advance_with_policy(
//!         AdvanceRequest::through(SimMoment::ORIGIN),
//!         policy,
//!     );
//! }
//! ```

mod action;
mod artifact;
mod attempt;
mod distribution;
mod engine;
mod lifecycle;
mod resolution;
mod routing;
mod session;

pub use action::{
    ActionEvaluationResultCapture, DeferredActionEvaluatorDescriptor, PendingActionEvaluation,
};
pub use artifact::{ArtifactResolveError, ArtifactResolver};
pub use attempt::{
    ActionEvaluationCaptureError, AdvanceOutcome, AdvanceRequest, AttemptError, CommandResolution,
    ResolvedCommandDelivery, RunAttempt, RunAttemptStatus, SystemCommandAdmissionOutcome,
    SystemCommandError, SystemCommandRequest,
};
pub use distribution::{
    ActionPolicyInstallation, DistributionError, EngineDistribution, LifecycleImplementationSet,
    LifecycleInstallationError, LifecyclePort, LifecycleResolutionError,
    baseline_lifecycle_profiles,
};
pub use engine::{Engine, EngineBuildError, EngineBuilder, StartAttemptError};
pub use resolution::{
    ExecutionActivationError, ExecutionOrigin, ExecutionSpecInput, ResolveExecutionError,
    ResolvedExecution,
};
pub use session::{ContainmentInspection, Inspector, SessionRead, SessionReadError, WorldSession};

pub use world_context::{
    ActionContextPayload, ActionContextPayloadSchemaId, ActionInputFingerprint,
    ActionPolicySemanticsId, ActionReadWitnessSchemaId, ActorSafeActionInteraction,
    ActorSafeContainmentInteraction, ActorSafeRelocationInteraction,
    ActorSafeRelocationInteractionEntry, CandidateResolutionTableSchemaId,
    GroundedActionCandidateId, GroundedActionInteraction, RelocationActionVerb,
};
pub use world_core::{
    ActorId, EntityId, Microstep, SimDuration, SimMoment, SimTime, WorldRevision,
};
pub use world_decision::{
    ActionDecision, ActionDecisionSchemaId, ActionPolicy, ActionPolicyError, BaselineActionPolicy,
    BaselineActivityController, activity_state_schema,
};
pub use world_defs::{
    ArtifactEnvelope, BindingName, DefinitionKey, EngineProtocolVersion, PackLock, PackLockEntry,
    ValueKind,
};
pub use world_model::{
    AcceptedState, ActionEvaluationInvocationId, ActionInteractionScope, ActionOpportunity,
    ActionOpportunityDisposition, ActionOpportunityGeneration, ActionOpportunityId,
    ActionOpportunityState, ActionSponsor, Activity, ActivityControllerId, ActivityFocus,
    ActivityGeneration, ActivityState, ActivityStatus, ActorLocation, ActorPosition,
    ActorReactionCause, AgencyState, CommandAttemptOutcome, CommandBinding, CommandId,
    CommandSource, CommandValue, ContainerAuthorityRecord, ContainerRecord,
    ContainmentInteractionScope, ContainmentRecord, ContainmentTransferDelta, DesiredCondition,
    DirectedRoute, DomainState, EpistemicState, EpistemicVersion, EvidenceDeliveryGeneration,
    EvidenceRecord, Intent, IntentGeneration, PhysicalEvent, RelocationInteraction,
    RelocationInteractionAnchor, RelocationInteractionScope, SocialState, StableCommandRejection,
    SystemCommandSourceId, TravelActivityState, TravelActivityStep,
};
pub use world_runtime::{
    ActionEvaluationCaptureId, ActionEvaluationCaptureOutcome, ActionEvaluationCaptureTiming,
    ActionEvaluationManagementDisposition, ActionEvaluationRequestId, ActionEvaluationResultId,
    ActionPolicyBindingV1, ActionPolicyExecutionV1, AttemptBinding, AttemptKey, AuthorityCursor,
    AuthorityRecordId, CancelAttemptOutcome, CancelAttemptRequest, CancelAttemptRequestId,
    CancelReason, ContainmentConflictPolicyV1, DeferredActionAdmissionModeV1,
    DeferredActionControlError, DeferredActionControlV1, DeferredActionFallbackV1,
    ExecutionConfigArtifactV3, ExecutionConfigError, FinalizationPolicyV1, InputId,
    KernelSafetyBlocker, KernelSafetyCause, KernelSafetyDisposition, KernelSafetyDueSetEvidence,
    LedgerRetirement, LifecycleBindingV1, LifecycleImplementationId, LifecycleProfilesV2,
    LifecycleStateBindingV1, LifecycleStateSchemaId, ManageOutcome, ManageRequest,
    ManagementRequestId, MomentResolutionPolicyV2, OptionalLifecycleBindingV1,
    PostCommitRoutingPolicyV1, RandomKeyPolicyV1, RandomOraclePolicyV1, RootSeed, RunAttemptId,
    RunFinalization, RunFinalizationCause, RuntimeService, SameTimeWaveExhaustionPolicyV1,
    SameTimeWaveTranche, SessionManagement, SessionMode, TerminationContractV1, TrajectoryId,
    WorkPopulationExhaustionPolicyV1,
};
