use world_core::DefinitionId;

use crate::{
    ContextProjectionCompleteness, ContextProjectionKind, ContextProvenance,
    context::ContextProjectionReportBuilder,
};

/// Actor-specific capability evidence projected from accepted context sources.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapabilitySet {
    entries: Vec<CapabilityEntry>,
}

impl CapabilitySet {
    /// Creates a capability set from deterministic entries.
    #[must_use]
    pub(crate) fn new(entries: Vec<CapabilityEntry>) -> Self {
        Self { entries }
    }

    /// Returns whether no capability evidence was projected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns projected capability entries.
    #[must_use]
    pub fn entries(&self) -> &[CapabilityEntry] {
        &self.entries
    }
}

/// One actor capability evidence entry.
///
/// Concrete capability vocabulary is definition-owned rather than encoded as
/// core enum variants. Materialized entries must preserve actor-specific
/// evidence such as subject, affected action, qualifiers, and provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityEntry {
    definition: Option<DefinitionId>,
    kind: CapabilityKind,
    status: CapabilityStatus,
    provenance: ContextProvenance,
}

impl CapabilityEntry {
    /// Returns the checked definition associated with the capability evidence, if any.
    #[must_use]
    pub const fn definition(&self) -> Option<DefinitionId> {
        self.definition
    }

    /// Returns the capability family.
    #[must_use]
    pub const fn kind(&self) -> CapabilityKind {
        self.kind
    }

    /// Returns how strongly this capability is supported by current context.
    #[must_use]
    pub const fn status(&self) -> CapabilityStatus {
        self.status
    }

    /// Returns the provenance anchors backing this capability evidence.
    #[must_use]
    pub const fn provenance(&self) -> &ContextProvenance {
        &self.provenance
    }
}

/// Coarse family of projected actor capability evidence.
///
/// Concrete capability kinds are carried by checked definitions, not by adding
/// game-specific variants here.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapabilityKind {
    /// Actor-specific evidence accepted or projected from actor-visible context.
    ActorEvidence,
}

/// Evidence strength for a projected capability.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapabilityStatus {
    /// Evidence is actor-specific but still not final runtime validation.
    EvidenceBacked,
}

pub(crate) fn derive(report: &mut ContextProjectionReportBuilder) -> CapabilitySet {
    report.push_status(
        ContextProjectionKind::Capability,
        ContextProjectionCompleteness::Unavailable,
    );
    CapabilitySet::new(Vec::new())
}
