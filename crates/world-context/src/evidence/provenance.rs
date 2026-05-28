use world_core::{ActorId, DefinitionId, EventRecordId, ProvenanceKey};
use world_model::{AcceptedRecordId, AuthorityRead};

/// Coarse source anchor explaining why context data was projected.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContextProvenanceSource {
    /// Actor scope used for actor-relative projection.
    ActorScope(ActorId),
    /// Checked definition used by a derived context entry.
    Definition(DefinitionId),
    /// Accepted non-hard authority record read from the model.
    AcceptedRecord(AcceptedRecordId),
    /// Committed event record read from event history.
    EventRecord(EventRecordId),
    /// Model read label used by a query surface.
    QueryRead(AuthorityRead),
    /// Opaque model provenance key carried by an accepted record.
    RecordProvenance(ProvenanceKey),
}

/// Ordered provenance anchors for context projection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextProvenance {
    sources: Vec<ContextProvenanceSource>,
}

impl ContextProvenance {
    /// Creates an empty provenance set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Adds a source anchor if it is not already present.
    pub fn push(&mut self, source: ContextProvenanceSource) {
        if !self.sources.contains(&source) {
            self.sources.push(source);
        }
    }

    /// Returns whether no source anchors have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Returns source anchors in insertion order.
    #[must_use]
    pub fn sources(&self) -> &[ContextProvenanceSource] {
        &self.sources
    }
}
