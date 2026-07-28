use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter};

use super::ExternalInputBindingDigest;

/// Canonical schema of the V1 external-input binding.
pub const EXTERNAL_INPUT_BINDING_SCHEMA_VERSION: u16 = 1;

const EXTERNAL_INPUT_BINDING_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("external-input-binding-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("external input binding domain must be valid"),
    };

/// Exact external-input binding for serialized host admission.
///
/// Request namespaces derive from the epoch lineage, request family, and this
/// admission tag. Transport details, authentication material, and retry
/// metadata do not enter this binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalInputBindingV1 {
    /// The host serializes admission into the authoritative input order.
    HostSerialized,
}

impl ExternalInputBindingV1 {
    /// Selects serialized host admission.
    #[must_use]
    pub const fn host_serialized() -> Self {
        Self::HostSerialized
    }

    /// Returns the canonical external-input binding identity.
    #[must_use]
    pub fn digest(self) -> ExternalInputBindingDigest {
        ExternalInputBindingDigest::of_canonical(&self.canonical_bytes())
    }

    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        let mut writer = CanonicalWriter::new(EXTERNAL_INPUT_BINDING_DOMAIN);
        writer.write_u16(EXTERNAL_INPUT_BINDING_SCHEMA_VERSION);
        writer.write_discriminant(match self {
            Self::HostSerialized => 0,
        });
        writer.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_serialized_binding_matches_the_frozen_vector() {
        let binding = ExternalInputBindingV1::HostSerialized;

        assert_eq!(
            hex(binding.canonical_bytes().as_bytes()),
            "776f726c642d63616e6f6e6963616c2d7631000000000000001965787465726e616c2d696e7075742d62696e64696e672d7631000100000000"
        );
        assert_eq!(
            binding.digest().to_string(),
            "5919fad3baa1797372fcecc09044e854a03c2d656a6c69966caa3a5629a6b11f"
        );
        assert_eq!(binding, ExternalInputBindingV1::host_serialized());
    }

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
            encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        encoded
    }
}
