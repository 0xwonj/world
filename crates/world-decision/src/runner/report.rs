use crate::{DecisionArtifact, DecisionArtifactRef, DecisionArtifactStore, DecisionTrace};

/// Outcome returned by a decision profile run.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DecisionRunOutcome {
    /// The profile produced an explicit terminal artifact.
    TerminalArtifact(DecisionArtifactRef),
    /// The profile intentionally abstained.
    Abstained,
    /// The profile failed before producing a terminal artifact.
    Failed,
}

/// Report returned by a decision profile run.
#[derive(Clone, Debug)]
pub struct DecisionRunReport {
    outcome: DecisionRunOutcome,
    trace: DecisionTrace,
    artifacts: DecisionArtifactStore,
}

impl DecisionRunReport {
    /// Creates a run report.
    #[must_use]
    pub fn new(
        outcome: DecisionRunOutcome,
        trace: DecisionTrace,
        artifacts: DecisionArtifactStore,
    ) -> Self {
        Self {
            outcome,
            trace,
            artifacts,
        }
    }

    /// Returns decision outcome.
    #[must_use]
    pub const fn outcome(&self) -> DecisionRunOutcome {
        self.outcome
    }

    /// Returns decision trace.
    #[must_use]
    pub const fn trace(&self) -> &DecisionTrace {
        &self.trace
    }

    /// Returns runtime-local artifacts produced during this decision run.
    #[must_use]
    pub const fn artifacts(&self) -> &DecisionArtifactStore {
        &self.artifacts
    }

    /// Looks up one produced artifact.
    #[must_use]
    pub fn artifact(&self, reference: DecisionArtifactRef) -> Option<&DecisionArtifact> {
        self.artifacts.get(reference)
    }

    /// Returns the terminal artifact when the run completed with one.
    #[must_use]
    pub fn terminal_artifact(&self) -> Option<&DecisionArtifact> {
        match self.outcome {
            DecisionRunOutcome::TerminalArtifact(reference) => self.artifact(reference),
            DecisionRunOutcome::Abstained | DecisionRunOutcome::Failed => None,
        }
    }

    /// Splits the report into owned outcome, trace, and runtime-local artifacts.
    #[must_use]
    pub fn into_parts(self) -> (DecisionRunOutcome, DecisionTrace, DecisionArtifactStore) {
        (self.outcome, self.trace, self.artifacts)
    }
}
