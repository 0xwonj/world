use world_core::WorldRevision;

use crate::AcceptedState;

/// Cloneable, non-authoritative read image at one world revision.
///
/// A snapshot can also be constructed as fixture data. Possessing one never
/// grants a session cursor, record-publication authority, or mutation access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldSnapshot {
    revision: WorldRevision,
    accepted: AcceptedState,
}

impl WorldSnapshot {
    /// Constructs an immutable read image from checked model state.
    #[must_use]
    pub const fn new(revision: WorldRevision, accepted: AcceptedState) -> Self {
        Self { revision, accepted }
    }

    /// Returns the authoritative revision represented by this image.
    #[must_use]
    pub const fn revision(&self) -> WorldRevision {
        self.revision
    }

    /// Returns the immutable accepted model state.
    #[must_use]
    pub const fn accepted(&self) -> &AcceptedState {
        &self.accepted
    }

    /// Consumes the snapshot and returns its accepted model state.
    #[must_use]
    pub fn into_accepted(self) -> AcceptedState {
        self.accepted
    }
}
