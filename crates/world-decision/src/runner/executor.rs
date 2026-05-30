use crate::{
    DecisionError, DecisionExecutionMetadata, DecisionPassDiagnostic, DecisionPassExecutionContext,
    ProducedDecisionArtifact,
};
use world_core::DefinitionId;

use crate::ImplementationMode;

/// Disposition returned by one pass executor.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DecisionPassDisposition {
    /// Pass completed and produced its declared outputs.
    Completed,
    /// Pass intentionally produced no terminal decision.
    Abstained,
}

/// Result payload returned by a pass executor.
#[derive(Clone, Debug)]
pub struct DecisionPassExecution {
    disposition: DecisionPassDisposition,
    outputs: Vec<ProducedDecisionArtifact>,
    diagnostics: Vec<DecisionPassDiagnostic>,
    metadata: DecisionExecutionMetadata,
}

impl DecisionPassExecution {
    /// Creates a completed execution result.
    #[must_use]
    pub fn completed(
        outputs: impl IntoIterator<Item = ProducedDecisionArtifact>,
        metadata: DecisionExecutionMetadata,
    ) -> Self {
        Self {
            disposition: DecisionPassDisposition::Completed,
            outputs: outputs.into_iter().collect(),
            diagnostics: Vec::new(),
            metadata,
        }
    }

    /// Creates an abstained execution result.
    #[must_use]
    pub fn abstained(metadata: DecisionExecutionMetadata) -> Self {
        Self {
            disposition: DecisionPassDisposition::Abstained,
            outputs: Vec::new(),
            diagnostics: Vec::new(),
            metadata,
        }
    }

    /// Adds diagnostics to this execution result.
    #[must_use]
    pub fn with_diagnostics(
        mut self,
        diagnostics: impl IntoIterator<Item = DecisionPassDiagnostic>,
    ) -> Self {
        self.diagnostics = diagnostics.into_iter().collect();
        self
    }

    /// Returns execution disposition.
    #[must_use]
    pub const fn disposition(&self) -> DecisionPassDisposition {
        self.disposition
    }

    /// Returns produced artifacts.
    #[must_use]
    pub fn outputs(&self) -> &[ProducedDecisionArtifact] {
        &self.outputs
    }

    /// Returns diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[DecisionPassDiagnostic] {
        &self.diagnostics
    }

    /// Returns execution metadata.
    #[must_use]
    pub const fn metadata(&self) -> &DecisionExecutionMetadata {
        &self.metadata
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        DecisionPassDisposition,
        Vec<ProducedDecisionArtifact>,
        Vec<DecisionPassDiagnostic>,
        DecisionExecutionMetadata,
    ) {
        (
            self.disposition,
            self.outputs,
            self.diagnostics,
            self.metadata,
        )
    }
}

/// Trusted executor for one checked decision pass and implementation mode.
///
/// This trait is an engine-installed semantics boundary, not a sandbox and not
/// ordinary pack-authoring authority. Implementations receive only a restricted
/// decision context and cannot mutate world state through `world-decision`.
pub trait DecisionPassExecutor: Send + Sync + 'static {
    /// Returns the pass this executor implements.
    fn pass_id(&self) -> DefinitionId;

    /// Returns the implementation mode this executor implements.
    fn mode(&self) -> ImplementationMode;

    /// Executes the pass over the restricted context.
    fn execute(
        &self,
        context: DecisionPassExecutionContext<'_>,
    ) -> Result<DecisionPassExecution, DecisionError>;
}
