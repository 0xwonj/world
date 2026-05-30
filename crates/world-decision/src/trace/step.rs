use world_core::DefinitionId;

use crate::{
    DecisionArtifactRef, DecisionError, DecisionExecutionMetadata, DecisionInputRef,
    DecisionVerifierResult, ImplementationMode, error::empty_item_field,
};

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

/// Execution status for one recorded decision trace step.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DecisionTraceStepStatus {
    /// Step executed and completed normally.
    Completed,
    /// Step was intentionally skipped by profile mode.
    Skipped,
    /// Step executed and intentionally abstained.
    Abstained,
    /// Step failed.
    Failed,
}

/// One step record inside a decision trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionTraceStep {
    pass: DefinitionId,
    mode: ImplementationMode,
    inputs: Vec<DecisionInputRef>,
    outputs: Vec<DecisionArtifactRef>,
    diagnostics: Vec<DecisionPassDiagnostic>,
    status: DecisionTraceStepStatus,
    verifier: DecisionVerifierResult,
    metadata: Option<DecisionExecutionMetadata>,
}

impl DecisionTraceStep {
    /// Creates a completed step trace record with artifact-only inputs.
    #[must_use]
    pub fn new(
        pass: DefinitionId,
        mode: ImplementationMode,
        inputs: impl IntoIterator<Item = DecisionArtifactRef>,
        outputs: impl IntoIterator<Item = DecisionArtifactRef>,
        diagnostics: impl IntoIterator<Item = DecisionPassDiagnostic>,
    ) -> Self {
        Self::recorded(
            pass,
            mode,
            inputs.into_iter().map(DecisionInputRef::Artifact),
            outputs,
            diagnostics,
            DecisionTraceStepStatus::Completed,
            DecisionVerifierResult::not_run(),
            None,
        )
    }

    /// Creates a step trace record from fully resolved input refs.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn recorded(
        pass: DefinitionId,
        mode: ImplementationMode,
        inputs: impl IntoIterator<Item = DecisionInputRef>,
        outputs: impl IntoIterator<Item = DecisionArtifactRef>,
        diagnostics: impl IntoIterator<Item = DecisionPassDiagnostic>,
        status: DecisionTraceStepStatus,
        verifier: DecisionVerifierResult,
        metadata: Option<DecisionExecutionMetadata>,
    ) -> Self {
        Self {
            pass,
            mode,
            inputs: inputs.into_iter().collect(),
            outputs: outputs.into_iter().collect(),
            diagnostics: diagnostics.into_iter().collect(),
            status,
            verifier,
            metadata,
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

    /// Returns input refs.
    #[must_use]
    pub fn inputs(&self) -> &[DecisionInputRef] {
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

    /// Returns step execution status.
    #[must_use]
    pub const fn status(&self) -> DecisionTraceStepStatus {
        self.status
    }

    /// Returns verifier result metadata.
    #[must_use]
    pub const fn verifier(&self) -> &DecisionVerifierResult {
        &self.verifier
    }

    /// Returns execution metadata, if recorded.
    #[must_use]
    pub const fn metadata(&self) -> Option<&DecisionExecutionMetadata> {
        self.metadata.as_ref()
    }
}
