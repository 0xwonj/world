use core::fmt;

use crate::CanonicalBytes;

/// Byte length of every selected content digest.
pub const CONTENT_DIGEST_LENGTH: usize = 32;

/// Hash algorithm carried by artifact and persistence protocol metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DigestAlgorithm {
    /// BLAKE3 with its default 256-bit output.
    Blake3_256,
}

impl DigestAlgorithm {
    /// Stable protocol identifier for serialized metadata.
    #[must_use]
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::Blake3_256 => "blake3-256",
        }
    }

    fn digest_bytes(self, bytes: &[u8]) -> ContentDigest {
        match self {
            Self::Blake3_256 => ContentDigest(*blake3::hash(bytes).as_bytes()),
        }
    }
}

/// Digest algorithm selected by `world-canonical-v1`.
pub const SELECTED_DIGEST_ALGORITHM: DigestAlgorithm = DigestAlgorithm::Blake3_256;

/// Exact 256-bit content or canonical-identity digest.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentDigest([u8; CONTENT_DIGEST_LENGTH]);

impl ContentDigest {
    /// Constructs a digest value decoded from exact protocol bytes.
    ///
    /// This validates shape only. Artifact owners still verify the digest
    /// against the referenced content before trusting an envelope.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; CONTENT_DIGEST_LENGTH]) -> Self {
        Self(bytes)
    }

    /// Hashes exact storage or artifact blob bytes with the selected
    /// algorithm.
    ///
    /// Semantic identities use [`Self::of_canonical`] instead so their domain
    /// framing is explicit.
    #[must_use]
    pub fn of_blob_bytes(bytes: &[u8]) -> Self {
        SELECTED_DIGEST_ALGORITHM.digest_bytes(bytes)
    }

    /// Hashes a completed canonical identity preimage.
    #[must_use]
    pub fn of_canonical(bytes: &CanonicalBytes) -> Self {
        SELECTED_DIGEST_ALGORITHM.digest_bytes(bytes.as_bytes())
    }

    /// Returns the exact digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; CONTENT_DIGEST_LENGTH] {
        &self.0
    }

    /// Consumes the value and returns its bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; CONTENT_DIGEST_LENGTH] {
        self.0
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ContentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ContentDigest({self})")
    }
}
