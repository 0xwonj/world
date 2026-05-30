//! Checked decision-substrate declarations and trace vocabulary.

mod error;
mod pass;
mod profile;
mod registry;
mod representation;
mod runner;
mod trace;

pub use error::DecisionError;
pub use pass::{
    DecisionPassContract, DeterminismPolicy, ImplementationMode, InputBinding, InputRequirement,
    PassClass, PassWritePolicy, RepresentationInput, RepresentationOutput,
};
pub use profile::{
    DecisionProfile, DecisionProfileExit, DecisionProfileOutput, DecisionProfileStep,
    ProfileOraclePolicy,
};
pub use registry::{DecisionRegistry, DecisionRegistryBuilder};
pub use representation::{
    RepresentationAuthority, RepresentationKindDef, RepresentationPersistence, RepresentationRole,
    RepresentationVisibility,
};
pub use runner::{
    DecisionArtifact, DecisionArtifactBody, DecisionArtifactPayload, DecisionArtifactStore,
    DecisionContextView, DecisionExecutorRegistry, DecisionPassDisposition, DecisionPassExecution,
    DecisionPassExecutionContext, DecisionPassExecutor, DecisionRunOutcome, DecisionRunReport,
    DecisionRunRequest, DecisionRunner, ProducedDecisionArtifact, ResolvedDecisionInput,
};
pub use trace::{
    DecisionArtifactRecord, DecisionArtifactRef, DecisionExecutionMetadata, DecisionInputRef,
    DecisionPassDiagnostic, DecisionRunSeed, DecisionTrace, DecisionTraceBuilder,
    DecisionTraceHeader, DecisionTraceStatus, DecisionTraceStep, DecisionTraceStepStatus,
    DecisionVerifierResult, DecisionVerifierStatus, ModelInvocationMetadata,
    OracleInvocationMetadata, ReplayInvocationMetadata,
};

#[cfg(test)]
mod tests;
