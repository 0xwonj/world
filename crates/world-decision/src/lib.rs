//! Checked decision-substrate declarations and trace vocabulary.

mod error;
mod pass;
mod profile;
mod registry;
mod representation;
mod trace;

pub use error::DecisionError;
pub use pass::{
    DecisionPassContract, DeterminismPolicy, ImplementationMode, InputBinding, InputRequirement,
    PassClass, PassWritePolicy, RepresentationInput, RepresentationOutput, TracePolicy,
};
pub use profile::{DecisionProfile, DecisionProfileStep, ProfileOraclePolicy};
pub use registry::{DecisionRegistry, DecisionRegistryBuilder};
pub use representation::{
    RepresentationAuthority, RepresentationKindDef, RepresentationPersistence, RepresentationRole,
    RepresentationVisibility,
};
pub use trace::{
    DecisionArtifactRecord, DecisionArtifactRef, DecisionPassDiagnostic, DecisionTrace,
    DecisionTraceHeader, DecisionTraceStatus, DecisionTraceStep,
};

#[cfg(test)]
mod tests;
