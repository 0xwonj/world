use crate::ContextProjectionKind;
use thiserror::Error;

/// Error returned when an actor-relative context cannot be projected.
///
/// Projection errors are source-free and do not carry parser spans. They
/// describe context assembly failures at the checked-definition/model boundary.
#[non_exhaustive]
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ContextError {
    /// A projection stage violated an internal invariant.
    #[error("context projection invariant failed for {projection:?}")]
    ProjectionInvariant {
        /// Projection stage that failed.
        projection: ContextProjectionKind,
    },
}
