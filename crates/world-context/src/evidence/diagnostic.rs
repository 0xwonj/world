use world_core::DefinitionId;

/// Named projection family used by reports and diagnostics.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContextProjectionKind {
    /// Actor-visible observation projection.
    Observation,
    /// Actor-owned epistemic working-set projection.
    Epistemic,
    /// Actor-relative social context projection.
    Social,
    /// Actor capability projection.
    Capability,
    /// Actor action-repertoire projection.
    Repertoire,
    /// Actor-visible affordance projection.
    Affordance,
}

/// Completeness of one projected context family.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContextProjectionCompleteness {
    /// Projection is complete for the currently implemented representation.
    Complete,
    /// Projection is intentionally shallow and should not be read as full semantics.
    Shallow,
    /// Projection slot exists, but no meaningful projection is available yet.
    Unavailable,
}

/// Status record for one projected context family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContextProjectionStatus {
    projection: ContextProjectionKind,
    completeness: ContextProjectionCompleteness,
}

impl ContextProjectionStatus {
    /// Creates a projection status record.
    #[must_use]
    pub const fn new(
        projection: ContextProjectionKind,
        completeness: ContextProjectionCompleteness,
    ) -> Self {
        Self {
            projection,
            completeness,
        }
    }

    /// Returns the projection family.
    #[must_use]
    pub const fn projection(self) -> ContextProjectionKind {
        self.projection
    }

    /// Returns the projection completeness.
    #[must_use]
    pub const fn completeness(self) -> ContextProjectionCompleteness {
        self.completeness
    }
}

/// Source-free diagnostic emitted while assembling actor context.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextDiagnostic {
    /// A projection family has no rich implementation for the current model data.
    ProjectionUnavailable {
        /// Projection family that could not be enriched.
        projection: ContextProjectionKind,
    },
    /// A checked semantic declaration was observed but not interpreted here.
    UnsupportedSemanticDeclaration {
        /// Semantic declaration that requires a later semantic stage.
        definition: DefinitionId,
    },
    /// A projection result was intentionally truncated by request options.
    ContextTruncated {
        /// Projection family that was truncated.
        projection: ContextProjectionKind,
    },
}
