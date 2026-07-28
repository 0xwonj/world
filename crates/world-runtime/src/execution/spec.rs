use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter};

use super::{
    ExecutionSemanticsManifestDigest, ExecutionSemanticsManifestV1, ExecutionSpecId,
    ExternalInputBindingDigest, ExternalInputBindingV1, InitialStateRootId, InitialStateRootV1,
    RootSeed, TerminationContractV1,
};

/// Canonical schema of an execution specification.
pub const EXECUTION_SPEC_SCHEMA_VERSION: u16 = 1;

const EXECUTION_SPEC_DOMAIN: CanonicalDomain = match CanonicalDomain::new("execution-spec-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("execution specification domain must be valid"),
};

/// Immutable specification that binds a root to its exact execution semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalExecutionSpecV1 {
    initial_root: InitialStateRootId,
    semantics: ExecutionSemanticsManifestDigest,
    root_seed: RootSeed,
    termination: TerminationContractV1,
    external_input: ExternalInputBindingV1,
    id: ExecutionSpecId,
}

impl CanonicalExecutionSpecV1 {
    pub(crate) fn new(
        initial_root: &InitialStateRootV1,
        semantics: &ExecutionSemanticsManifestV1,
        root_seed: RootSeed,
        termination: TerminationContractV1,
        external_input: ExternalInputBindingV1,
    ) -> Self {
        let initial_root = initial_root.id();
        let semantics = semantics.digest();
        let bytes = execution_spec_bytes(
            initial_root,
            semantics,
            root_seed,
            termination,
            external_input,
        );
        Self {
            initial_root,
            semantics,
            root_seed,
            termination,
            external_input,
            id: ExecutionSpecId::of_canonical(&bytes),
        }
    }

    /// Returns the exact initial state root identity.
    #[must_use]
    pub const fn initial_root(&self) -> InitialStateRootId {
        self.initial_root
    }

    /// Returns the normalized execution-semantics identity.
    #[must_use]
    pub const fn semantics(&self) -> ExecutionSemanticsManifestDigest {
        self.semantics
    }

    /// Returns the exact root seed.
    #[must_use]
    pub const fn root_seed(&self) -> RootSeed {
        self.root_seed
    }

    /// Returns the closed semantic termination contract.
    #[must_use]
    pub const fn termination(&self) -> TerminationContractV1 {
        self.termination
    }

    /// Returns the exact external-input binding.
    #[must_use]
    pub const fn external_input(&self) -> ExternalInputBindingV1 {
        self.external_input
    }

    /// Returns the external-input binding identity.
    #[must_use]
    pub fn external_input_digest(&self) -> ExternalInputBindingDigest {
        self.external_input.digest()
    }

    /// Returns the complete execution specification identity.
    #[must_use]
    pub const fn id(&self) -> ExecutionSpecId {
        self.id
    }

    pub(crate) fn canonical_bytes(&self) -> CanonicalBytes {
        execution_spec_bytes(
            self.initial_root,
            self.semantics,
            self.root_seed,
            self.termination,
            self.external_input,
        )
    }
}

fn execution_spec_bytes(
    initial_root: InitialStateRootId,
    semantics: ExecutionSemanticsManifestDigest,
    root_seed: RootSeed,
    termination: TerminationContractV1,
    external_input: ExternalInputBindingV1,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(EXECUTION_SPEC_DOMAIN);
    writer.write_u16(EXECUTION_SPEC_SCHEMA_VERSION);
    write_fixed_bytes(&mut writer, initial_root.as_bytes());
    write_fixed_bytes(&mut writer, semantics.as_bytes());
    write_fixed_bytes(&mut writer, root_seed.as_bytes());
    write_owned_bytes(&mut writer, termination.canonical_bytes().as_bytes());
    write_fixed_bytes(&mut writer, external_input.digest().as_bytes());
    writer.finish()
}

fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width identity length must fit the canonical protocol");
    }
}

fn write_owned_bytes(writer: &mut CanonicalWriter, bytes: &[u8]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("owned canonical bytes must fit the canonical protocol");
    }
}

#[cfg(test)]
mod tests {
    use world_core::{Microstep, SimMoment, SimTime};

    use super::*;

    fn bytes(
        root_byte: u8,
        semantics_byte: u8,
        seed_byte: u8,
        termination: TerminationContractV1,
    ) -> CanonicalBytes {
        execution_spec_bytes(
            InitialStateRootId::from_bytes([root_byte; 32]),
            ExecutionSemanticsManifestDigest::from_bytes([semantics_byte; 32]),
            RootSeed::from_bytes([seed_byte; 32]),
            termination,
            ExternalInputBindingV1::HostSerialized,
        )
    }

    #[test]
    fn specification_identity_preimage_is_sensitive_to_every_input() {
        let base = bytes(1, 2, 3, TerminationContractV1::Never);
        let identity = ExecutionSpecId::of_canonical(&base);

        assert_ne!(
            identity,
            ExecutionSpecId::of_canonical(&bytes(2, 2, 3, TerminationContractV1::Never))
        );
        assert_ne!(
            identity,
            ExecutionSpecId::of_canonical(&bytes(1, 3, 3, TerminationContractV1::Never))
        );
        assert_ne!(
            identity,
            ExecutionSpecId::of_canonical(&bytes(1, 2, 4, TerminationContractV1::Never))
        );
        assert_ne!(
            identity,
            ExecutionSpecId::of_canonical(&bytes(
                1,
                2,
                3,
                TerminationContractV1::AtOrAfterMoment {
                    moment: SimMoment::new(SimTime::from_ticks(5), Microstep::new(1)),
                },
            ))
        );
    }

    #[test]
    fn specification_preimage_has_a_frozen_vector() {
        assert_eq!(
            hex(bytes(1, 2, 3, TerminationContractV1::Never).as_bytes()),
            "776f726c642d63616e6f6e6963616c2d76310000000000000011657865637574696f6e2d737065632d763100010000000000000020010101010101010101010101010101010101010101010101010101010101010100000000000000200202020202020202020202020202020202020202020202020202020202020202000000000000002003030303030303030303030303030303030303030303030303030303030303030000000000000037776f726c642d63616e6f6e6963616c2d763100000000000000177465726d696e6174696f6e2d636f6e74726163742d763100010000000000000000000000205919fad3baa1797372fcecc09044e854a03c2d656a6c69966caa3a5629a6b11f"
        );
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
