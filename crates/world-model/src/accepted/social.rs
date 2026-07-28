use core::fmt;

use world_core::{CanonicalDomain, CanonicalWriter, ContentDigest};

/// Canonical schema version of [`SocialState`].
pub const SOCIAL_STATE_SCHEMA_VERSION: u16 = 1;

const SOCIAL_STATE_CANONICAL_DOMAIN: CanonicalDomain = match CanonicalDomain::new("social-state-v1")
{
    Ok(domain) => domain,
    Err(_) => panic!("social-state identity domain must be valid"),
};

/// Canonical identity of accepted social state.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocialStateDigest(ContentDigest);

impl SocialStateDigest {
    /// Returns the exact digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Consumes the digest and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0.into_bytes()
    }
}

impl fmt::Display for SocialStateDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for SocialStateDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "SocialStateDigest({self})")
    }
}

/// Accepted actor-relative and institutional social state.
///
/// The partition remains empty until one concrete social interpretation has
/// a checked producer and consumer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SocialState;

impl SocialState {
    /// Constructs the canonical empty social state.
    #[must_use]
    pub const fn empty() -> Self {
        Self
    }

    /// Returns the canonical social-state identity.
    #[must_use]
    pub fn digest(&self) -> SocialStateDigest {
        let mut writer = CanonicalWriter::new(SOCIAL_STATE_CANONICAL_DOMAIN);
        writer.write_u16(SOCIAL_STATE_SCHEMA_VERSION);
        SocialStateDigest(ContentDigest::of_canonical(&writer.finish()))
    }
}
