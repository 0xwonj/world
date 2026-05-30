use std::collections::{BTreeMap, BTreeSet};

use world_context::{ActorContextProjection, ContextProvenance, ContextReadSet};
use world_core::{ActorId, DefinitionId, VersionAnchor};

use crate::{DecisionError, ProfileOraclePolicy};

mod artifact;
mod builder;
mod metadata;
mod step;

pub use artifact::{DecisionArtifactRecord, DecisionArtifactRef, DecisionInputRef};
pub use builder::DecisionTraceBuilder;
pub use metadata::{
    DecisionExecutionMetadata, DecisionRunSeed, DecisionVerifierResult, DecisionVerifierStatus,
    ModelInvocationMetadata, OracleInvocationMetadata, ReplayInvocationMetadata,
};
pub use step::{DecisionPassDiagnostic, DecisionTraceStep, DecisionTraceStepStatus};

/// Header metadata shared by all steps and artifacts in one decision trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionTraceHeader {
    actor: ActorId,
    profile: DefinitionId,
    profile_version: VersionAnchor,
    context_reads: ContextReadSet,
    context_provenance: ContextProvenance,
    oracle_policy: ProfileOraclePolicy,
}

impl DecisionTraceHeader {
    /// Creates a trace header from explicit projection metadata.
    #[must_use]
    pub fn new(
        actor: ActorId,
        profile: DefinitionId,
        profile_version: VersionAnchor,
        context_reads: ContextReadSet,
        context_provenance: ContextProvenance,
        oracle_policy: ProfileOraclePolicy,
    ) -> Self {
        Self {
            actor,
            profile,
            profile_version,
            context_reads,
            context_provenance,
            oracle_policy,
        }
    }

    /// Creates a trace header from an actor-context projection.
    #[must_use]
    pub fn from_projection(
        projection: &ActorContextProjection,
        profile: DefinitionId,
        profile_version: VersionAnchor,
        oracle_policy: ProfileOraclePolicy,
    ) -> Self {
        Self::new(
            projection.context().actor(),
            profile,
            profile_version,
            projection.report().reads().clone(),
            projection.report().provenance().clone(),
            oracle_policy,
        )
    }

    /// Returns the actor scope.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the decision profile id.
    #[must_use]
    pub const fn profile(&self) -> DefinitionId {
        self.profile
    }

    /// Returns the decision profile version anchor.
    #[must_use]
    pub const fn profile_version(&self) -> VersionAnchor {
        self.profile_version
    }

    /// Returns context read dependencies.
    #[must_use]
    pub const fn context_reads(&self) -> &ContextReadSet {
        &self.context_reads
    }

    /// Returns context provenance anchors.
    #[must_use]
    pub const fn context_provenance(&self) -> &ContextProvenance {
        &self.context_provenance
    }

    /// Returns oracle policy metadata.
    #[must_use]
    pub const fn oracle_policy(&self) -> ProfileOraclePolicy {
        self.oracle_policy
    }
}

/// Overall status for a decision trace.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DecisionTraceStatus {
    /// Trace has started but is not final.
    Started,
    /// Trace completed normally.
    Completed,
    /// Trace completed by intentional abstention.
    Abstained,
    /// Trace failed before producing a final artifact.
    Failed,
}

/// Value vocabulary for a recorded decision pipeline run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionTrace {
    header: DecisionTraceHeader,
    steps: Vec<DecisionTraceStep>,
    artifacts: Vec<DecisionArtifactRecord>,
    status: DecisionTraceStatus,
}

impl DecisionTrace {
    /// Creates an empty started trace.
    #[must_use]
    pub fn new(header: DecisionTraceHeader) -> Self {
        Self {
            header,
            steps: Vec::new(),
            artifacts: Vec::new(),
            status: DecisionTraceStatus::Started,
        }
    }

    /// Creates a trace from explicit parts.
    pub fn from_parts(
        header: DecisionTraceHeader,
        steps: impl IntoIterator<Item = DecisionTraceStep>,
        artifacts: impl IntoIterator<Item = DecisionArtifactRecord>,
        status: DecisionTraceStatus,
    ) -> Result<Self, DecisionError> {
        let steps = steps.into_iter().collect::<Vec<_>>();
        let artifacts = artifacts.into_iter().collect::<Vec<_>>();
        let artifacts_by_ref = validate_unique_artifacts(&artifacts)?;
        validate_step_artifacts(&steps, &artifacts_by_ref)?;

        Ok(Self {
            header,
            steps,
            artifacts,
            status,
        })
    }

    /// Returns trace header metadata.
    #[must_use]
    pub const fn header(&self) -> &DecisionTraceHeader {
        &self.header
    }

    /// Returns step records in execution order.
    #[must_use]
    pub fn steps(&self) -> &[DecisionTraceStep] {
        &self.steps
    }

    /// Returns artifact records in trace-local id order chosen by the producer.
    #[must_use]
    pub fn artifacts(&self) -> &[DecisionArtifactRecord] {
        &self.artifacts
    }

    /// Returns trace status.
    #[must_use]
    pub const fn status(&self) -> DecisionTraceStatus {
        self.status
    }
}

fn validate_unique_artifacts(
    artifacts: &[DecisionArtifactRecord],
) -> Result<BTreeMap<DecisionArtifactRef, &DecisionArtifactRecord>, DecisionError> {
    let mut seen = BTreeSet::new();
    let mut by_ref = BTreeMap::new();
    for artifact in artifacts {
        if !seen.insert(artifact.artifact()) {
            return Err(DecisionError::DuplicateArtifactRef {
                artifact: artifact.artifact(),
            });
        }
        by_ref.insert(artifact.artifact(), artifact);
    }

    Ok(by_ref)
}

fn validate_step_artifacts(
    steps: &[DecisionTraceStep],
    artifacts: &BTreeMap<DecisionArtifactRef, &DecisionArtifactRecord>,
) -> Result<(), DecisionError> {
    for step in steps {
        for input in step.inputs() {
            if let DecisionInputRef::Artifact(artifact) = input
                && !artifacts.contains_key(artifact)
            {
                return Err(DecisionError::MissingTraceArtifact {
                    artifact: *artifact,
                });
            }
        }

        for output in step.outputs() {
            let Some(record) = artifacts.get(output) else {
                return Err(DecisionError::MissingTraceArtifact { artifact: *output });
            };
            if record.producer() != Some(step.pass()) {
                return Err(DecisionError::TraceOutputProducerMismatch {
                    pass: step.pass(),
                    artifact: *output,
                    producer: record.producer(),
                });
            }
        }
    }

    Ok(())
}
