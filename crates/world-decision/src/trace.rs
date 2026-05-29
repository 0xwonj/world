use core::num::NonZeroU64;
use std::collections::BTreeSet;

use world_context::{ActorContextProjection, ContextProvenance, ContextReadSet};
use world_core::{ActorId, DefinitionId, VersionAnchor};

use crate::{
    DecisionError, ImplementationMode, ProfileOraclePolicy, RepresentationRole,
    error::empty_item_field,
};

/// Trace-local artifact identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionArtifactRef(NonZeroU64);

impl DecisionArtifactRef {
    /// Creates an artifact ref when the raw value is nonzero.
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the underlying trace-local numeric value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

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

/// One typed artifact produced or consumed by a decision trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionArtifactRecord {
    artifact: DecisionArtifactRef,
    kind: DefinitionId,
    role: RepresentationRole,
    producer: Option<DefinitionId>,
    provenance: ContextProvenance,
}

impl DecisionArtifactRecord {
    /// Creates an artifact record.
    #[must_use]
    pub fn new(
        artifact: DecisionArtifactRef,
        kind: DefinitionId,
        role: RepresentationRole,
        producer: Option<DefinitionId>,
        provenance: ContextProvenance,
    ) -> Self {
        Self {
            artifact,
            kind,
            role,
            producer,
            provenance,
        }
    }

    /// Returns the trace-local artifact ref.
    #[must_use]
    pub const fn artifact(&self) -> DecisionArtifactRef {
        self.artifact
    }

    /// Returns the representation kind.
    #[must_use]
    pub const fn kind(&self) -> DefinitionId {
        self.kind
    }

    /// Returns the broad role carried by this artifact record.
    #[must_use]
    pub const fn role(&self) -> RepresentationRole {
        self.role
    }

    /// Returns the producing pass id, if any.
    #[must_use]
    pub const fn producer(&self) -> Option<DefinitionId> {
        self.producer
    }

    /// Returns provenance anchors.
    #[must_use]
    pub const fn provenance(&self) -> &ContextProvenance {
        &self.provenance
    }
}

/// Structured pass diagnostic recorded by a trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionPassDiagnostic {
    pass: Option<DefinitionId>,
    message: String,
}

impl DecisionPassDiagnostic {
    /// Creates a diagnostic with a non-empty message.
    pub fn new(
        pass: Option<DefinitionId>,
        message: impl Into<String>,
    ) -> Result<Self, DecisionError> {
        let message = message.into();
        if message.trim().is_empty() {
            return Err(empty_item_field("DecisionPassDiagnostic", "message"));
        }

        Ok(Self { pass, message })
    }

    /// Returns the pass that emitted the diagnostic, if known.
    #[must_use]
    pub const fn pass(&self) -> Option<DefinitionId> {
        self.pass
    }

    /// Returns diagnostic text.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// One step record inside a decision trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionTraceStep {
    pass: DefinitionId,
    mode: ImplementationMode,
    inputs: Vec<DecisionArtifactRef>,
    outputs: Vec<DecisionArtifactRef>,
    diagnostics: Vec<DecisionPassDiagnostic>,
}

impl DecisionTraceStep {
    /// Creates a step trace record.
    #[must_use]
    pub fn new(
        pass: DefinitionId,
        mode: ImplementationMode,
        inputs: impl IntoIterator<Item = DecisionArtifactRef>,
        outputs: impl IntoIterator<Item = DecisionArtifactRef>,
        diagnostics: impl IntoIterator<Item = DecisionPassDiagnostic>,
    ) -> Self {
        Self {
            pass,
            mode,
            inputs: inputs.into_iter().collect(),
            outputs: outputs.into_iter().collect(),
            diagnostics: diagnostics.into_iter().collect(),
        }
    }

    /// Returns the pass id.
    #[must_use]
    pub const fn pass(&self) -> DefinitionId {
        self.pass
    }

    /// Returns the selected implementation mode.
    #[must_use]
    pub const fn mode(&self) -> ImplementationMode {
        self.mode
    }

    /// Returns input artifact refs.
    #[must_use]
    pub fn inputs(&self) -> &[DecisionArtifactRef] {
        &self.inputs
    }

    /// Returns output artifact refs.
    #[must_use]
    pub fn outputs(&self) -> &[DecisionArtifactRef] {
        &self.outputs
    }

    /// Returns diagnostics emitted by the step.
    #[must_use]
    pub fn diagnostics(&self) -> &[DecisionPassDiagnostic] {
        &self.diagnostics
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
        let steps = steps.into_iter().collect();
        let artifacts = artifacts.into_iter().collect::<Vec<_>>();
        validate_unique_artifacts(&artifacts)?;

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

fn validate_unique_artifacts(artifacts: &[DecisionArtifactRecord]) -> Result<(), DecisionError> {
    let mut seen = BTreeSet::new();
    for artifact in artifacts {
        if !seen.insert(artifact.artifact()) {
            return Err(DecisionError::DuplicateArtifactRef {
                artifact: artifact.artifact(),
            });
        }
    }

    Ok(())
}
