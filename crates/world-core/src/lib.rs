//! Stable cross-plane primitives for the simulation engine.
//!
//! This crate owns canonical identity bytes, content digests, the few
//! identities shared by multiple lower layers, virtual time, and authoritative
//! revision scalars. Package-specific identities stay with their owners.

mod canonical;
mod content;
mod identity;
mod revision;
mod time;

pub use canonical::{
    CANONICAL_PROTOCOL_IDENTIFIER, CanonicalBytes, CanonicalDomain, CanonicalError,
    CanonicalWriter, MAX_CANONICAL_DOMAIN_LENGTH,
};
pub use content::{
    CONTENT_DIGEST_LENGTH, ContentDigest, DigestAlgorithm, SELECTED_DIGEST_ALGORITHM,
};
pub use identity::{ActorId, EntityId};
pub use revision::{NonZeroWorldRevision, WorldRevision};
pub use time::{Microstep, SimDuration, SimMoment, SimTime};
