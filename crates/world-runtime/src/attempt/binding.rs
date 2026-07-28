use core::fmt;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter, ContentDigest};

use crate::authority::AuthorityCursor;
use crate::execution::{
    EpochLineageId, ExecutionSpecId, InitialStateRootId, ResolvedExecutionClosureManifestDigest,
    ResolvedExecutionClosureManifestV1,
};

const RUN_ATTEMPT_SCHEMA_VERSION: u16 = 1;
const ATTEMPT_BINDING_SCHEMA_VERSION: u16 = 1;
const ATTEMPT_CREATION_SCHEMA_VERSION: u16 = 1;
const ATTEMPT_CONTROL_FORMAT_VERSION: u16 = 1;
const ATTEMPT_AUTHORITY_DOMAIN_SCHEMA_VERSION: u16 = 1;

const RUN_ATTEMPT_DOMAIN: CanonicalDomain = match CanonicalDomain::new("run-attempt-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("run attempt domain must be valid"),
};

const ATTEMPT_BINDING_DOMAIN: CanonicalDomain = match CanonicalDomain::new("attempt-binding-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("attempt binding domain must be valid"),
};

const ATTEMPT_CREATION_DOMAIN: CanonicalDomain = match CanonicalDomain::new("attempt-creation-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("attempt creation domain must be valid"),
};

const ATTEMPT_AUTHORITY_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("attempt-authority-domain-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("attempt authority domain must be valid"),
    };

/// Identity of one independently writable attempt-authority domain.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttemptAuthorityDomainId([u8; 32]);

impl AttemptAuthorityDomainId {
    /// Constructs the identity decoded or minted by the runtime repository.
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn from_repository_ordinal(ordinal: u64) -> Self {
        let mut writer = CanonicalWriter::new(ATTEMPT_AUTHORITY_DOMAIN);
        writer.write_u16(ATTEMPT_AUTHORITY_DOMAIN_SCHEMA_VERSION);
        writer.write_u64(ordinal);
        Self(ContentDigest::of_canonical(&writer.finish()).into_bytes())
    }

    /// Returns the exact authority-domain identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the identity and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for AttemptAuthorityDomainId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for AttemptAuthorityDomainId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "AttemptAuthorityDomainId({self})")
    }
}

/// Fixed-width key assigned by a runner when it starts or reopens an attempt.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttemptKey([u8; 32]);

impl AttemptKey {
    /// Constructs an attempt key from its exact bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the exact attempt-key bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the key and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for AttemptKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for AttemptKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "AttemptKey({self})")
    }
}

/// Physical identity of one run attempt inside an authority domain.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunAttemptId([u8; 32]);

impl RunAttemptId {
    /// Constructs a fixed-width identity decoded by the runtime owner.
    ///
    /// This proves representation shape only. Opening an attempt verifies the
    /// identity against its complete creation descriptor.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn derive(
        domain: AttemptAuthorityDomainId,
        execution: ExecutionSpecId,
        key: AttemptKey,
    ) -> Self {
        Self(ContentDigest::of_canonical(&run_attempt_bytes(domain, execution, key)).into_bytes())
    }

    /// Returns the exact run-attempt identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the identity and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for RunAttemptId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for RunAttemptId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RunAttemptId({self})")
    }
}

/// Permanent correspondence among an attempt and its semantic execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttemptBinding {
    domain: AttemptAuthorityDomainId,
    attempt: RunAttemptId,
    execution: ExecutionSpecId,
    initial_root: InitialStateRootId,
    lineage: EpochLineageId,
}

impl AttemptBinding {
    fn derive(
        domain: AttemptAuthorityDomainId,
        key: AttemptKey,
        closure: &ResolvedExecutionClosureManifestV1,
    ) -> Self {
        let execution = closure.specification().id();
        Self {
            domain,
            attempt: RunAttemptId::derive(domain, execution, key),
            execution,
            initial_root: closure.initial_root().id(),
            lineage: closure.initial_root().lineage_id(),
        }
    }

    /// Returns the repository-owned attempt-authority domain.
    #[must_use]
    pub const fn domain(self) -> AttemptAuthorityDomainId {
        self.domain
    }

    /// Returns the physical run-attempt identity.
    #[must_use]
    pub const fn attempt(self) -> RunAttemptId {
        self.attempt
    }

    /// Returns the exact execution-specification identity.
    #[must_use]
    pub const fn execution(self) -> ExecutionSpecId {
        self.execution
    }

    /// Returns the exact initial-state-root identity.
    #[must_use]
    pub const fn initial_root(self) -> InitialStateRootId {
        self.initial_root
    }

    /// Returns the semantic epoch-lineage identity.
    #[must_use]
    pub const fn lineage(self) -> EpochLineageId {
        self.lineage
    }

    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        attempt_binding_bytes(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AttemptCreationDescriptor {
    binding: AttemptBinding,
    key: AttemptKey,
    root_cursor: AuthorityCursor,
    closure_digest: ResolvedExecutionClosureManifestDigest,
    control_format_version: u16,
}

impl AttemptCreationDescriptor {
    pub(crate) const fn binding(self) -> AttemptBinding {
        self.binding
    }

    #[cfg(test)]
    pub(crate) const fn key(self) -> AttemptKey {
        self.key
    }

    #[cfg(test)]
    pub(crate) const fn root_cursor(self) -> AuthorityCursor {
        self.root_cursor
    }

    #[cfg(test)]
    pub(crate) const fn closure_digest(self) -> ResolvedExecutionClosureManifestDigest {
        self.closure_digest
    }

    #[cfg(test)]
    pub(crate) const fn control_format_version(self) -> u16 {
        self.control_format_version
    }

    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        attempt_creation_bytes(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AttemptCreationFingerprint([u8; 32]);

impl AttemptCreationFingerprint {
    pub(crate) fn derive(descriptor: AttemptCreationDescriptor) -> Self {
        Self(ContentDigest::of_canonical(&descriptor.canonical_bytes()).into_bytes())
    }

    #[cfg(test)]
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for AttemptCreationFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for AttemptCreationFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "AttemptCreationFingerprint({self})")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AttemptCreation {
    descriptor: AttemptCreationDescriptor,
    fingerprint: AttemptCreationFingerprint,
}

impl AttemptCreation {
    pub(crate) fn derive(
        domain: AttemptAuthorityDomainId,
        key: AttemptKey,
        closure: &ResolvedExecutionClosureManifestV1,
    ) -> Self {
        let descriptor = AttemptCreationDescriptor {
            binding: AttemptBinding::derive(domain, key, closure),
            key,
            root_cursor: closure.root_cursor(),
            closure_digest: closure.digest(),
            control_format_version: ATTEMPT_CONTROL_FORMAT_VERSION,
        };
        Self {
            descriptor,
            fingerprint: AttemptCreationFingerprint::derive(descriptor),
        }
    }

    pub(crate) const fn binding(self) -> AttemptBinding {
        self.descriptor.binding()
    }

    #[cfg(test)]
    pub(crate) const fn descriptor(self) -> AttemptCreationDescriptor {
        self.descriptor
    }

    #[cfg(test)]
    pub(crate) const fn fingerprint(self) -> AttemptCreationFingerprint {
        self.fingerprint
    }
}

fn run_attempt_bytes(
    domain: AttemptAuthorityDomainId,
    execution: ExecutionSpecId,
    key: AttemptKey,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(RUN_ATTEMPT_DOMAIN);
    writer.write_u16(RUN_ATTEMPT_SCHEMA_VERSION);
    write_fixed_bytes(&mut writer, domain.as_bytes());
    write_fixed_bytes(&mut writer, execution.as_bytes());
    write_fixed_bytes(&mut writer, key.as_bytes());
    writer.finish()
}

fn attempt_binding_bytes(binding: AttemptBinding) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(ATTEMPT_BINDING_DOMAIN);
    writer.write_u16(ATTEMPT_BINDING_SCHEMA_VERSION);
    write_fixed_bytes(&mut writer, binding.domain.as_bytes());
    write_fixed_bytes(&mut writer, binding.attempt.as_bytes());
    write_fixed_bytes(&mut writer, binding.execution.as_bytes());
    write_fixed_bytes(&mut writer, binding.initial_root.as_bytes());
    write_fixed_bytes(&mut writer, binding.lineage.as_bytes());
    writer.finish()
}

fn attempt_creation_bytes(descriptor: AttemptCreationDescriptor) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(ATTEMPT_CREATION_DOMAIN);
    writer.write_u16(ATTEMPT_CREATION_SCHEMA_VERSION);
    write_owned_bytes(&mut writer, descriptor.binding.canonical_bytes().as_bytes());
    write_fixed_bytes(&mut writer, descriptor.key.as_bytes());
    write_owned_bytes(
        &mut writer,
        descriptor.root_cursor.canonical_bytes().as_bytes(),
    );
    write_fixed_bytes(&mut writer, descriptor.closure_digest.as_bytes());
    writer.write_u16(descriptor.control_format_version);
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

fn write_hex(formatter: &mut fmt::Formatter<'_>, bytes: &[u8; 32]) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use world_core::SimMoment;
    use world_model::{AcceptedState, AgencyState, DomainState, EpistemicState, SocialState};

    use crate::authority::EpochIdentity;
    use crate::control::test_support;
    use crate::execution::{
        CanonicalExecutionSpecV1, ExecutionConfigArtifactV3, ExecutionSemanticsManifestV1,
        ExternalInputBindingV1, InitialStateRootV1, RootSeed, TerminationContractV1,
    };
    use crate::session::SessionMode;

    use super::*;

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("attempt fixture must be valid: {error}"),
        }
    }

    fn closure() -> ResolvedExecutionClosureManifestV1 {
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            test_support::definitions(),
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            Vec::new(),
        ));
        let root = valid(InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            AcceptedState::new(
                valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
                EpistemicState::empty(),
                SocialState::empty(),
                AgencyState::empty(),
            ),
            Vec::new(),
        ));
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &semantics,
            RootSeed::from_bytes([0x61; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        valid(ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ))
    }

    fn domain(byte: u8) -> AttemptAuthorityDomainId {
        AttemptAuthorityDomainId::from_bytes([byte; 32])
    }

    fn key(byte: u8) -> AttemptKey {
        AttemptKey::from_bytes([byte; 32])
    }

    fn binding() -> AttemptBinding {
        let domain = domain(0x11);
        let execution = ExecutionSpecId::from_bytes([0x33; 32]);
        AttemptBinding {
            domain,
            attempt: RunAttemptId::derive(domain, execution, key(0x22)),
            execution,
            initial_root: InitialStateRootId::from_bytes([0x44; 32]),
            lineage: EpochLineageId::from_bytes([0x55; 32]),
        }
    }

    fn descriptor() -> AttemptCreationDescriptor {
        let binding = binding();
        AttemptCreationDescriptor {
            binding,
            key: key(0x22),
            root_cursor: AuthorityCursor::root(
                EpochIdentity::new(binding.lineage(), binding.execution()),
                binding.initial_root(),
            ),
            closure_digest: ResolvedExecutionClosureManifestDigest::from_bytes([0x66; 32]),
            control_format_version: ATTEMPT_CONTROL_FORMAT_VERSION,
        }
    }

    #[test]
    fn run_attempt_identity_is_sensitive_to_every_input() {
        let execution = ExecutionSpecId::from_bytes([0x33; 32]);
        let base = RunAttemptId::derive(domain(0x11), execution, key(0x22));

        assert_ne!(
            base,
            RunAttemptId::derive(domain(0x12), execution, key(0x22))
        );
        assert_ne!(
            base,
            RunAttemptId::derive(
                domain(0x11),
                ExecutionSpecId::from_bytes([0x34; 32]),
                key(0x22),
            )
        );
        assert_ne!(
            base,
            RunAttemptId::derive(domain(0x11), execution, key(0x23))
        );
    }

    #[test]
    fn binding_encoding_is_sensitive_to_every_field() {
        let base = binding();
        let bytes = base.canonical_bytes();

        let mut changed = base;
        changed.domain = domain(0x12);
        assert_ne!(bytes, changed.canonical_bytes());

        changed = base;
        changed.attempt = RunAttemptId::from_bytes([0x23; 32]);
        assert_ne!(bytes, changed.canonical_bytes());

        changed = base;
        changed.execution = ExecutionSpecId::from_bytes([0x34; 32]);
        assert_ne!(bytes, changed.canonical_bytes());

        changed = base;
        changed.initial_root = InitialStateRootId::from_bytes([0x45; 32]);
        assert_ne!(bytes, changed.canonical_bytes());

        changed = base;
        changed.lineage = EpochLineageId::from_bytes([0x56; 32]);
        assert_ne!(bytes, changed.canonical_bytes());
    }

    #[test]
    fn creation_fingerprint_is_sensitive_to_every_field() {
        let base = descriptor();
        let fingerprint = AttemptCreationFingerprint::derive(base);

        let mut changed = base;
        changed.binding.domain = domain(0x12);
        assert_ne!(fingerprint, AttemptCreationFingerprint::derive(changed));

        changed = base;
        changed.binding.attempt = RunAttemptId::from_bytes([0x23; 32]);
        assert_ne!(fingerprint, AttemptCreationFingerprint::derive(changed));

        changed = base;
        changed.binding.execution = ExecutionSpecId::from_bytes([0x34; 32]);
        assert_ne!(fingerprint, AttemptCreationFingerprint::derive(changed));

        changed = base;
        changed.binding.initial_root = InitialStateRootId::from_bytes([0x45; 32]);
        assert_ne!(fingerprint, AttemptCreationFingerprint::derive(changed));

        changed = base;
        changed.binding.lineage = EpochLineageId::from_bytes([0x56; 32]);
        assert_ne!(fingerprint, AttemptCreationFingerprint::derive(changed));

        changed = base;
        changed.key = key(0x23);
        assert_ne!(fingerprint, AttemptCreationFingerprint::derive(changed));

        changed = base;
        changed.root_cursor = AuthorityCursor::root(
            EpochIdentity::new(changed.binding.lineage(), changed.binding.execution()),
            InitialStateRootId::from_bytes([0x45; 32]),
        );
        assert_ne!(fingerprint, AttemptCreationFingerprint::derive(changed));

        changed = base;
        changed.closure_digest = ResolvedExecutionClosureManifestDigest::from_bytes([0x67; 32]);
        assert_ne!(fingerprint, AttemptCreationFingerprint::derive(changed));

        changed = base;
        changed.control_format_version += 1;
        assert_ne!(fingerprint, AttemptCreationFingerprint::derive(changed));
    }

    #[test]
    fn creation_is_derived_from_one_resolved_closure() {
        let closure = closure();
        let creation = AttemptCreation::derive(domain(0x11), key(0x22), &closure);
        let descriptor = creation.descriptor();

        assert_eq!(creation.binding().domain(), domain(0x11));
        assert_eq!(creation.binding().execution(), closure.specification().id());
        assert_eq!(
            creation.binding().initial_root(),
            closure.initial_root().id()
        );
        assert_eq!(
            creation.binding().lineage(),
            closure.initial_root().lineage_id()
        );
        assert_eq!(
            creation.binding().attempt(),
            RunAttemptId::derive(domain(0x11), closure.specification().id(), key(0x22))
        );
        assert_eq!(descriptor.key(), key(0x22));
        assert_eq!(descriptor.root_cursor(), closure.root_cursor());
        assert_eq!(descriptor.closure_digest(), closure.digest());
        assert_eq!(
            descriptor.control_format_version(),
            ATTEMPT_CONTROL_FORMAT_VERSION
        );
        assert_eq!(
            creation.fingerprint(),
            AttemptCreationFingerprint::derive(descriptor)
        );
    }

    #[test]
    fn authority_domain_changes_only_the_physical_attempt_binding() {
        let closure = closure();
        let execution = closure.specification().id();
        let root = closure.initial_root().id();
        let lineage = closure.initial_root().lineage_id();
        let cursor = closure.root_cursor();
        let closure_digest = closure.digest();
        let first = AttemptCreation::derive(domain(0x11), key(0x22), &closure);
        let second = AttemptCreation::derive(domain(0x12), key(0x22), &closure);

        assert_ne!(first.binding().attempt(), second.binding().attempt());
        assert_ne!(first.fingerprint(), second.fingerprint());
        assert_eq!(first.binding().execution(), execution);
        assert_eq!(second.binding().execution(), execution);
        assert_eq!(first.binding().initial_root(), root);
        assert_eq!(second.binding().initial_root(), root);
        assert_eq!(first.binding().lineage(), lineage);
        assert_eq!(second.binding().lineage(), lineage);
        assert_eq!(first.descriptor().root_cursor(), cursor);
        assert_eq!(second.descriptor().root_cursor(), cursor);
        assert_eq!(first.descriptor().closure_digest(), closure_digest);
        assert_eq!(second.descriptor().closure_digest(), closure_digest);
        assert_eq!(closure.specification().id(), execution);
        assert_eq!(closure.initial_root().id(), root);
    }

    #[test]
    fn canonical_values_have_frozen_vectors() {
        let binding = binding();
        let descriptor = descriptor();
        let run_bytes = run_attempt_bytes(binding.domain(), binding.execution(), key(0x22));

        assert_eq!(
            hex(run_bytes.as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d7631000000000000000e72756e2d617474656d7074",
                "2d76310001000000000000002011111111111111111111111111111111111111111111111111",
                "1111111111111100000000000000203333333333333333333333333333333333333333333333",
                "3333333333333333330000000000000020222222222222222222222222222222222222222222",
                "2222222222222222222222",
            )
        );
        assert_eq!(
            RunAttemptId::derive(binding.domain(), binding.execution(), key(0x22)).to_string(),
            "d2ed14fa948da4ca6a9f1d5b042a927176d244efb932bd541cae0e237a11a507"
        );
        assert_eq!(
            hex(binding.canonical_bytes().as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d76310000000000000012617474656d7074",
                "2d62696e64696e672d7631000100000000000000201111111111111111111111111111",
                "1111111111111111111111111111111111110000000000000020d2ed14fa948da4ca",
                "6a9f1d5b042a927176d244efb932bd541cae0e237a11a50700000000000000203333",
                "33333333333333333333333333333333333333333333333333333333333300000000",
                "00000020444444444444444444444444444444444444444444444444444444444444",
                "44440000000000000020555555555555555555555555555555555555555555555555",
                "5555555555555555",
            )
        );
        assert_eq!(
            hex(descriptor.canonical_bytes().as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d76310000000000000013617474656d7074",
                "2d6372656174696f6e2d7631000100000000000000f6776f726c642d63616e6f6e",
                "6963616c2d76310000000000000012617474656d70742d62696e64696e672d7631",
                "00010000000000000020111111111111111111111111111111111111111111111111",
                "11111111111111110000000000000020d2ed14fa948da4ca6a9f1d5b042a927176",
                "d244efb932bd541cae0e237a11a50700000000000000203333333333333333333333",
                "33333333333333333333333333333333333333333300000000000000204444444444",
                "44444444444444444444444444444444444444444444444444444400000000000000",
                "20555555555555555555555555555555555555555555555555555555555555555500",
                "00000000000020222222222222222222222222222222222222222222222222222222",
                "222222222200000000000000d3776f726c642d63616e6f6e6963616c2d76310000",
                "000000000013617574686f726974792d637572736f722d7631000100000000000000",
                "20555555555555555555555555555555555555555555555555555555555555555500",
                "00000000000020333333333333333333333333333333333333333333333333333333",
                "33333333330000000000000000000000209d4f6436f2e6041ea22775a2fd5d5520",
                "46da2973191ab0f5ef03437cbea73b590000000000000020965528b1da570d501262",
                "bfc00bab1661082727fb81065d28fb581905b768272700000000000000206666666666",
                "6666666666666666666666666666666666666666666666666666660001",
            )
        );
        assert_eq!(
            hex(AttemptCreationFingerprint::derive(descriptor).as_bytes()),
            "d9f9eb4a7c0c21d26a933ac01b768ce02d2a3f17ad50713a280a692faf0d18f0"
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
