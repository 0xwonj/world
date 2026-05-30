use core::num::NonZeroU64;

use world_context::{ContextProjectionKind, ContextProvenance};
use world_core::DefinitionId;

use crate::RepresentationRole;

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

/// Input reference recorded for one decision pass execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DecisionInputRef {
    /// Input came directly from an actor-context projection family.
    Context(ContextProjectionKind),
    /// Input came from a prior decision artifact.
    Artifact(DecisionArtifactRef),
}

impl From<DecisionArtifactRef> for DecisionInputRef {
    fn from(artifact: DecisionArtifactRef) -> Self {
        Self::Artifact(artifact)
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
