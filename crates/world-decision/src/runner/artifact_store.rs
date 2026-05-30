use std::{any::Any, collections::BTreeMap, fmt::Debug, sync::Arc};

use world_context::ContextProvenance;
use world_core::DefinitionId;

use crate::{DecisionArtifactRecord, DecisionArtifactRef, DecisionError, RepresentationRole};

/// Type-erased body for a runtime-local decision artifact payload.
pub trait DecisionArtifactBody: Any + Debug + Send + Sync {
    /// Returns this value as `Any` for trusted executor downcasting.
    fn as_any(&self) -> &dyn Any;
}

impl<T> DecisionArtifactBody for T
where
    T: Any + Debug + Send + Sync,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Runtime-local payload carried by a decision artifact.
///
/// Payloads are not authority. The runner validates artifact metadata against
/// checked representation declarations; concrete payload meaning belongs to
/// representation-family executors.
#[derive(Clone, Debug)]
pub struct DecisionArtifactPayload {
    body: Arc<dyn DecisionArtifactBody>,
}

impl DecisionArtifactPayload {
    /// Creates a payload from a trusted executor-owned body.
    #[must_use]
    pub fn new<T>(body: T) -> Self
    where
        T: DecisionArtifactBody + 'static,
    {
        Self {
            body: Arc::new(body),
        }
    }

    /// Creates a marker payload for tests and metadata-only artifacts.
    #[must_use]
    pub fn marker() -> Self {
        Self::new(MarkerArtifact)
    }

    /// Attempts to view the payload as a concrete body type.
    #[must_use]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Any,
    {
        self.body.as_ref().as_any().downcast_ref()
    }
}

#[derive(Debug)]
struct MarkerArtifact;

/// Artifact value produced by a pass executor before trace-local ids are
/// assigned.
#[derive(Clone, Debug)]
pub struct ProducedDecisionArtifact {
    kind: DefinitionId,
    role: RepresentationRole,
    payload: DecisionArtifactPayload,
    provenance: ContextProvenance,
}

impl ProducedDecisionArtifact {
    /// Creates a produced artifact.
    #[must_use]
    pub fn new(
        kind: DefinitionId,
        role: RepresentationRole,
        payload: DecisionArtifactPayload,
        provenance: ContextProvenance,
    ) -> Self {
        Self {
            kind,
            role,
            payload,
            provenance,
        }
    }

    /// Creates a produced marker artifact.
    #[must_use]
    pub fn marker(kind: DefinitionId, role: RepresentationRole) -> Self {
        Self::new(
            kind,
            role,
            DecisionArtifactPayload::marker(),
            ContextProvenance::new(),
        )
    }

    /// Returns representation kind.
    #[must_use]
    pub const fn kind(&self) -> DefinitionId {
        self.kind
    }

    /// Returns broad representation role.
    #[must_use]
    pub const fn role(&self) -> RepresentationRole {
        self.role
    }

    /// Returns payload.
    #[must_use]
    pub const fn payload(&self) -> &DecisionArtifactPayload {
        &self.payload
    }

    /// Returns provenance.
    #[must_use]
    pub const fn provenance(&self) -> &ContextProvenance {
        &self.provenance
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        DefinitionId,
        RepresentationRole,
        DecisionArtifactPayload,
        ContextProvenance,
    ) {
        (self.kind, self.role, self.payload, self.provenance)
    }
}

/// Runtime artifact stored during one decision run.
#[derive(Clone, Debug)]
pub struct DecisionArtifact {
    record: DecisionArtifactRecord,
    payload: DecisionArtifactPayload,
}

impl DecisionArtifact {
    /// Creates a stored artifact.
    #[must_use]
    pub fn new(record: DecisionArtifactRecord, payload: DecisionArtifactPayload) -> Self {
        Self { record, payload }
    }

    /// Returns trace metadata for this artifact.
    #[must_use]
    pub const fn record(&self) -> &DecisionArtifactRecord {
        &self.record
    }

    /// Returns payload for this artifact.
    #[must_use]
    pub const fn payload(&self) -> &DecisionArtifactPayload {
        &self.payload
    }
}

/// In-memory artifact store for a single decision run.
#[derive(Clone, Debug, Default)]
pub struct DecisionArtifactStore {
    artifacts: BTreeMap<DecisionArtifactRef, DecisionArtifact>,
}

impl DecisionArtifactStore {
    /// Creates an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a stored artifact.
    pub fn insert(&mut self, artifact: DecisionArtifact) -> Result<(), DecisionError> {
        let reference = artifact.record().artifact();
        if self.artifacts.contains_key(&reference) {
            return Err(DecisionError::DuplicateArtifactRef {
                artifact: reference,
            });
        }
        self.artifacts.insert(reference, artifact);
        Ok(())
    }

    /// Looks up an artifact by trace-local ref.
    #[must_use]
    pub fn get(&self, reference: DecisionArtifactRef) -> Option<&DecisionArtifact> {
        self.artifacts.get(&reference)
    }

    /// Returns whether the store contains a ref.
    #[must_use]
    pub fn contains(&self, reference: DecisionArtifactRef) -> bool {
        self.artifacts.contains_key(&reference)
    }

    /// Returns the number of stored artifacts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    /// Returns whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    /// Returns stored artifacts in deterministic ref order.
    pub fn artifacts(&self) -> impl Iterator<Item = &DecisionArtifact> {
        self.artifacts.values()
    }
}
