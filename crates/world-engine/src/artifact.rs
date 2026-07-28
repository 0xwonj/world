use core::fmt;

use world_defs::{ArtifactEnvelope, PackLockEntry};

/// Failure of one read-only execution-artifact lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactResolveError {
    /// No artifact exists for the exact lock entry.
    NotFound,
    /// The backing artifact service is temporarily unavailable.
    Unavailable,
}

impl fmt::Display for ArtifactResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("artifact was not found"),
            Self::Unavailable => formatter.write_str("artifact service is unavailable"),
        }
    }
}

impl std::error::Error for ArtifactResolveError {}

/// Read-only host port for exact compiled execution artifacts.
///
/// Returned bytes are untrusted until `world-defs` decodes and validates them.
pub trait ArtifactResolver: Send + Sync + 'static {
    /// Resolves the exact artifact named by one immutable package-lock entry.
    fn resolve(&self, reference: &PackLockEntry) -> Result<ArtifactEnvelope, ArtifactResolveError>;
}
