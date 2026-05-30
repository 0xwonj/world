use core::num::NonZeroU64;

use crate::{DecisionError, DeterminismPolicy, ImplementationMode, error::require_not_empty};

/// Seed captured for a seeded decision pass execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionRunSeed(NonZeroU64);

impl DecisionRunSeed {
    /// Creates a seed when the raw value is nonzero.
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the underlying seed value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Metadata for model-backed LLM or hybrid pass execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInvocationMetadata {
    model: String,
    prompt: String,
    sampling: Option<String>,
}

impl ModelInvocationMetadata {
    /// Creates model invocation metadata.
    pub fn new(
        model: impl Into<String>,
        prompt: impl Into<String>,
        sampling: Option<impl Into<String>>,
    ) -> Result<Self, DecisionError> {
        let model = model.into();
        let prompt = prompt.into();
        require_not_empty("ModelInvocationMetadata", "model", &model)?;
        require_not_empty("ModelInvocationMetadata", "prompt", &prompt)?;
        let sampling = sampling.map(Into::into);
        if let Some(sampling) = &sampling {
            require_not_empty("ModelInvocationMetadata", "sampling", sampling)?;
        }

        Ok(Self {
            model,
            prompt,
            sampling,
        })
    }

    /// Returns model identifier.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Returns prompt identifier or version.
    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// Returns sampling metadata, if any.
    #[must_use]
    pub fn sampling(&self) -> Option<&str> {
        self.sampling.as_deref()
    }
}

/// Metadata for oracle-backed pass execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleInvocationMetadata {
    source: String,
}

impl OracleInvocationMetadata {
    /// Creates oracle invocation metadata.
    pub fn new(source: impl Into<String>) -> Result<Self, DecisionError> {
        let source = source.into();
        require_not_empty("OracleInvocationMetadata", "source", &source)?;
        Ok(Self { source })
    }

    /// Returns oracle source identifier.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Metadata for replay-backed pass execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayInvocationMetadata {
    source: String,
}

impl ReplayInvocationMetadata {
    /// Creates replay invocation metadata.
    pub fn new(source: impl Into<String>) -> Result<Self, DecisionError> {
        let source = source.into();
        require_not_empty("ReplayInvocationMetadata", "source", &source)?;
        Ok(Self { source })
    }

    /// Returns replay source identifier.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Metadata recorded for one pass execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionExecutionMetadata {
    mode: ImplementationMode,
    determinism: DeterminismPolicy,
    seed: Option<DecisionRunSeed>,
    model: Option<ModelInvocationMetadata>,
    oracle: Option<OracleInvocationMetadata>,
    replay: Option<ReplayInvocationMetadata>,
}

impl DecisionExecutionMetadata {
    /// Creates execution metadata without optional external-source details.
    #[must_use]
    pub const fn new(mode: ImplementationMode, determinism: DeterminismPolicy) -> Self {
        Self {
            mode,
            determinism,
            seed: None,
            model: None,
            oracle: None,
            replay: None,
        }
    }

    /// Adds seeded execution metadata.
    #[must_use]
    pub const fn with_seed(mut self, seed: DecisionRunSeed) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Adds model invocation metadata.
    #[must_use]
    pub fn with_model(mut self, model: ModelInvocationMetadata) -> Self {
        self.model = Some(model);
        self
    }

    /// Adds oracle invocation metadata.
    #[must_use]
    pub fn with_oracle(mut self, oracle: OracleInvocationMetadata) -> Self {
        self.oracle = Some(oracle);
        self
    }

    /// Adds replay invocation metadata.
    #[must_use]
    pub fn with_replay(mut self, replay: ReplayInvocationMetadata) -> Self {
        self.replay = Some(replay);
        self
    }

    /// Returns execution mode metadata.
    #[must_use]
    pub const fn mode(&self) -> ImplementationMode {
        self.mode
    }

    /// Returns determinism metadata.
    #[must_use]
    pub const fn determinism(&self) -> DeterminismPolicy {
        self.determinism
    }

    /// Returns seeded execution metadata.
    #[must_use]
    pub const fn seed(&self) -> Option<DecisionRunSeed> {
        self.seed
    }

    /// Returns model invocation metadata.
    #[must_use]
    pub const fn model(&self) -> Option<&ModelInvocationMetadata> {
        self.model.as_ref()
    }

    /// Returns oracle invocation metadata.
    #[must_use]
    pub const fn oracle(&self) -> Option<&OracleInvocationMetadata> {
        self.oracle.as_ref()
    }

    /// Returns replay invocation metadata.
    #[must_use]
    pub const fn replay(&self) -> Option<&ReplayInvocationMetadata> {
        self.replay.as_ref()
    }
}

/// Status for contract verification performed around a pass execution.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DecisionVerifierStatus {
    /// No verifier was run for this step.
    NotRun,
    /// Verification passed.
    Passed,
    /// Verification failed.
    Failed,
}

/// Summary of verifier status recorded in the trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionVerifierResult {
    status: DecisionVerifierStatus,
}

impl DecisionVerifierResult {
    /// Creates a verifier result.
    #[must_use]
    pub const fn new(status: DecisionVerifierStatus) -> Self {
        Self { status }
    }

    /// Creates a not-run verifier result.
    #[must_use]
    pub const fn not_run() -> Self {
        Self::new(DecisionVerifierStatus::NotRun)
    }

    /// Creates a passed verifier result.
    #[must_use]
    pub const fn passed() -> Self {
        Self::new(DecisionVerifierStatus::Passed)
    }

    /// Creates a failed verifier result.
    #[must_use]
    pub const fn failed() -> Self {
        Self::new(DecisionVerifierStatus::Failed)
    }

    /// Returns verifier status.
    #[must_use]
    pub const fn status(&self) -> DecisionVerifierStatus {
        self.status
    }
}
