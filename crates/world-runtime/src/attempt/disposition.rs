use core::fmt;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter, ContentDigest};

use super::AttemptBinding;

const CANCEL_ATTEMPT_REQUEST_SCHEMA_VERSION: u16 = 1;
const ATTEMPT_DISPOSITION_SCHEMA_VERSION: u16 = 1;

const CANCEL_ATTEMPT_REQUEST_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("cancel-attempt-request-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("cancel attempt request domain must be valid"),
    };

const ATTEMPT_DISPOSITION_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("world-attempt-disposition-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("attempt disposition domain must be valid"),
    };

/// Runner-assigned identity of one cancellation request.
///
/// Zero is a valid first identity. Reuse is classified by the attempt-control
/// request ledger rather than by reserving a sentinel value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CancelAttemptRequestId(u64);

impl CancelAttemptRequestId {
    /// Constructs a cancellation request identity.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the exact request identity.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Typed reason accepted by the cancellation request protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CancelReason {
    /// The host explicitly requested termination of this attempt.
    HostRequested,
}

impl CancelReason {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::HostRequested => 0,
        }
    }
}

/// Cancellation command submitted through an already attempt-bound driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CancelAttemptRequest {
    id: CancelAttemptRequestId,
    reason: CancelReason,
}

impl CancelAttemptRequest {
    /// Constructs a typed cancellation command.
    #[must_use]
    pub const fn new(id: CancelAttemptRequestId, reason: CancelReason) -> Self {
        Self { id, reason }
    }

    /// Returns the runner-assigned request identity.
    #[must_use]
    pub const fn id(self) -> CancelAttemptRequestId {
        self.id
    }

    /// Returns the typed cancellation reason.
    #[must_use]
    pub const fn reason(self) -> CancelReason {
        self.reason
    }

    pub(crate) fn bind(self, binding: AttemptBinding) -> BoundCancelAttemptRequest {
        BoundCancelAttemptRequest::new(binding, self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CancelAttemptRequestFingerprint([u8; 32]);

impl CancelAttemptRequestFingerprint {
    fn derive(binding: AttemptBinding, reason: CancelReason) -> Self {
        Self(
            ContentDigest::of_canonical(&cancel_attempt_request_bytes(binding, reason))
                .into_bytes(),
        )
    }

    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for CancelAttemptRequestFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for CancelAttemptRequestFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "CancelAttemptRequestFingerprint({self})")
    }
}

/// Cancellation request after the runtime binds it to authoritative attempt
/// identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BoundCancelAttemptRequest {
    binding: AttemptBinding,
    request: CancelAttemptRequest,
    fingerprint: CancelAttemptRequestFingerprint,
}

impl BoundCancelAttemptRequest {
    fn new(binding: AttemptBinding, request: CancelAttemptRequest) -> Self {
        Self {
            binding,
            request,
            fingerprint: CancelAttemptRequestFingerprint::derive(binding, request.reason()),
        }
    }

    #[cfg(test)]
    pub(crate) const fn binding(self) -> AttemptBinding {
        self.binding
    }

    pub(crate) const fn id(self) -> CancelAttemptRequestId {
        self.request.id()
    }

    pub(crate) const fn reason(self) -> CancelReason {
        self.request.reason()
    }

    pub(crate) const fn fingerprint(self) -> CancelAttemptRequestFingerprint {
        self.fingerprint
    }

    pub(crate) const fn into_disposition(self) -> AttemptDisposition {
        AttemptDisposition::CancelRequested {
            request: self.id(),
            fingerprint: self.fingerprint(),
            reason: self.reason(),
        }
    }
}

/// Content identity of one exact attempt-disposition value.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttemptDispositionId([u8; 32]);

impl AttemptDispositionId {
    /// Constructs a fixed-width identity decoded by attempt-control storage.
    ///
    /// The disposition store recomputes the content identity before trusting
    /// the decoded value.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    fn derive(disposition: AttemptDisposition) -> Self {
        Self(ContentDigest::of_canonical(&disposition.canonical_bytes()).into_bytes())
    }

    /// Returns the exact disposition identity bytes.
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

impl fmt::Display for AttemptDispositionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for AttemptDispositionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "AttemptDispositionId({self})")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttemptDisposition {
    CancelRequested {
        request: CancelAttemptRequestId,
        fingerprint: CancelAttemptRequestFingerprint,
        reason: CancelReason,
    },
    HostBudgetExceeded,
    ExternalFailure,
    EngineFailure,
}

impl AttemptDisposition {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::CancelRequested { .. } => 0,
            Self::HostBudgetExceeded => 1,
            Self::ExternalFailure => 2,
            Self::EngineFailure => 3,
        }
    }

    pub(crate) fn id(self) -> AttemptDispositionId {
        AttemptDispositionId::derive(self)
    }

    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        attempt_disposition_bytes(self)
    }
}

/// A retained disposition identity resolved to different canonical content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AttemptDispositionStoreConflict {
    id: AttemptDispositionId,
}

impl AttemptDispositionStoreConflict {
    #[cfg(test)]
    pub(crate) const fn id(self) -> AttemptDispositionId {
        self.id
    }
}

/// Exact content-addressed attempt-disposition evidence.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct AttemptDispositionStore {
    values: BTreeMap<AttemptDispositionId, AttemptDisposition>,
}

impl AttemptDispositionStore {
    pub(crate) fn retain(
        &mut self,
        disposition: AttemptDisposition,
    ) -> Result<AttemptDispositionId, AttemptDispositionStoreConflict> {
        let id = disposition.id();
        match self.values.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(disposition);
                Ok(id)
            }
            Entry::Occupied(entry) if *entry.get() == disposition => Ok(id),
            Entry::Occupied(_) => Err(AttemptDispositionStoreConflict { id }),
        }
    }

    pub(crate) fn get(&self, id: AttemptDispositionId) -> Option<AttemptDisposition> {
        self.values.get(&id).copied()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

fn cancel_attempt_request_bytes(binding: AttemptBinding, reason: CancelReason) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(CANCEL_ATTEMPT_REQUEST_DOMAIN);
    writer.write_u16(CANCEL_ATTEMPT_REQUEST_SCHEMA_VERSION);
    write_owned_bytes(&mut writer, binding.canonical_bytes().as_bytes());
    writer.write_discriminant(reason.canonical_tag());
    writer.finish()
}

fn attempt_disposition_bytes(disposition: AttemptDisposition) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(ATTEMPT_DISPOSITION_DOMAIN);
    writer.write_u16(ATTEMPT_DISPOSITION_SCHEMA_VERSION);
    writer.write_discriminant(disposition.canonical_tag());
    if let AttemptDisposition::CancelRequested {
        request,
        fingerprint,
        reason,
    } = disposition
    {
        writer.write_u64(request.get());
        write_fixed_bytes(&mut writer, fingerprint.as_bytes());
        writer.write_discriminant(reason.canonical_tag());
    }
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

    use world_core::{EntityId, SimMoment};
    use world_model::{
        AcceptedState, AgencyState, ContainerRecord, DomainState, EpistemicState, SocialState,
    };

    use crate::control::test_support;
    use crate::execution::{
        CanonicalExecutionSpecV1, ExecutionConfigArtifactV3, ExecutionSemanticsManifestV1,
        ExternalInputBindingV1, InitialStateRootV1, ResolvedExecutionClosureManifestV1, RootSeed,
        TerminationContractV1,
    };
    use crate::session::SessionMode;

    use super::super::{AttemptAuthorityDomainId, AttemptCreation, AttemptKey};
    use super::*;

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("cancellation fixture must be valid: {error}"),
        }
    }

    fn closure(container_byte: u8) -> ResolvedExecutionClosureManifestV1 {
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            test_support::definitions(),
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            Vec::new(),
        ));
        let accepted = AcceptedState::new(
            valid(DomainState::new(
                vec![ContainerRecord::new(
                    EntityId::from_bytes([container_byte; 32]),
                    1,
                )],
                Vec::new(),
                Vec::new(),
            )),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );
        let root = valid(InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted,
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

    fn binding(domain_byte: u8, key_byte: u8, container_byte: u8) -> AttemptBinding {
        AttemptCreation::derive(
            AttemptAuthorityDomainId::from_bytes([domain_byte; 32]),
            AttemptKey::from_bytes([key_byte; 32]),
            &closure(container_byte),
        )
        .binding()
    }

    fn request(id: u64) -> CancelAttemptRequest {
        CancelAttemptRequest::new(CancelAttemptRequestId::new(id), CancelReason::HostRequested)
    }

    fn bound_request(id: u64) -> BoundCancelAttemptRequest {
        request(id).bind(binding(0x11, 0x22, 0x31))
    }

    #[test]
    fn request_identity_accepts_zero_and_the_public_command_has_no_binding() {
        let request = request(0);

        assert_eq!(request.id().get(), 0);
        assert_eq!(request.reason(), CancelReason::HostRequested);
    }

    #[test]
    fn binding_derives_the_fingerprint_and_omits_request_identity() {
        let first = bound_request(0);
        let retry = bound_request(9);

        assert_eq!(first.binding(), retry.binding());
        assert_eq!(first.fingerprint(), retry.fingerprint());
        assert_ne!(first.id(), retry.id());
        assert_eq!(
            first.fingerprint(),
            CancelAttemptRequestFingerprint::derive(first.binding(), first.reason())
        );
    }

    #[test]
    fn cancellation_fingerprint_commits_the_complete_authoritative_binding() {
        let base = bound_request(0).fingerprint();
        let changed_domain = request(0).bind(binding(0x12, 0x22, 0x31)).fingerprint();
        let changed_attempt = request(0).bind(binding(0x11, 0x23, 0x31)).fingerprint();
        let changed_execution = request(0).bind(binding(0x11, 0x22, 0x32)).fingerprint();

        assert_ne!(base, changed_domain);
        assert_ne!(base, changed_attempt);
        assert_ne!(base, changed_execution);
    }

    #[test]
    fn disposition_identity_commits_variant_and_cancellation_evidence() {
        let cancellation = bound_request(0).into_disposition();
        let changed_id = bound_request(1).into_disposition();
        let changed_fingerprint = request(0)
            .bind(binding(0x12, 0x22, 0x31))
            .into_disposition();
        let ids = [
            cancellation.id(),
            AttemptDisposition::HostBudgetExceeded.id(),
            AttemptDisposition::ExternalFailure.id(),
            AttemptDisposition::EngineFailure.id(),
        ];

        assert_ne!(cancellation.id(), changed_id.id());
        assert_ne!(cancellation.id(), changed_fingerprint.id());
        for (index, id) in ids.iter().enumerate() {
            assert!(!ids[index + 1..].contains(id));
        }
    }

    #[test]
    fn store_is_idempotent_and_never_overwrites_mismatched_content() {
        let cancellation = bound_request(0).into_disposition();
        let cancellation_id = cancellation.id();
        let mut store = AttemptDispositionStore::default();

        assert!(store.is_empty());
        assert_eq!(store.retain(cancellation), Ok(cancellation_id));
        assert_eq!(store.retain(cancellation), Ok(cancellation_id));
        assert_eq!(store.len(), 1);
        assert_eq!(store.get(cancellation_id), Some(cancellation));

        store
            .values
            .insert(cancellation_id, AttemptDisposition::EngineFailure);
        let conflict = store.retain(cancellation);
        assert_eq!(
            conflict,
            Err(AttemptDispositionStoreConflict {
                id: cancellation_id,
            })
        );
        assert_eq!(
            conflict.map_err(AttemptDispositionStoreConflict::id),
            Err(cancellation_id)
        );
        assert_eq!(
            store.get(cancellation_id),
            Some(AttemptDisposition::EngineFailure)
        );
    }

    #[test]
    fn store_retains_each_distinct_content_value() {
        let values = [
            bound_request(0).into_disposition(),
            AttemptDisposition::HostBudgetExceeded,
            AttemptDisposition::ExternalFailure,
            AttemptDisposition::EngineFailure,
        ];
        let mut store = AttemptDispositionStore::default();

        for value in values {
            let id = match store.retain(value) {
                Ok(id) => id,
                Err(error) => panic!("distinct fixture disposition collided: {error:?}"),
            };
            assert_eq!(store.get(id), Some(value));
        }
        assert_eq!(store.len(), values.len());
    }

    #[test]
    fn canonical_values_have_frozen_vectors() {
        let bound = bound_request(0);
        let cancellation = bound.into_disposition();

        assert_eq!(
            hex(cancel_attempt_request_bytes(bound.binding(), bound.reason()).as_bytes()),
            "776f726c642d63616e6f6e6963616c2d7631000000000000001963616e63656c2d617474656d70742d726571756573742d7631000100000000000000f6776f726c642d63616e6f6e6963616c2d76310000000000000012617474656d70742d62696e64696e672d76310001000000000000002011111111111111111111111111111111111111111111111111111111111111110000000000000020657a5a84e97406bc0e9dc7eac627de4b4c0cfccfb2690bfc52f2c609b0b148400000000000000020b93005592cff390dc6bfeb3d7dc77fd80f8be6edb19fe45381d5042a970dfe70000000000000002071195a4ca293fece70c6261b367a5bf4b07e09d0e19971ab900d2b2a8278ab810000000000000020dea8d78c283eba82c4d6e512f9890b8ac69d2e5c1bb2905bbe19cab4b655954300000000"
        );
        assert_eq!(
            bound.fingerprint().to_string(),
            "4ea124e95ea7d7adfe969e7469b9ef011451ef1e9e6d8b8a1468f3756169988a"
        );
        assert_eq!(
            hex(cancellation.canonical_bytes().as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d7631000000000000001c776f726c642d617474656d70742d",
                "646973706f736974696f6e2d7631000100000000000000000000000000000000000000204ea124e9",
                "5ea7d7adfe969e7469b9ef011451ef1e9e6d8b8a1468f3756169988a00000000",
            )
        );
        assert_eq!(
            cancellation.id().to_string(),
            "2acca99b6a96ab0b3c0c0fd60027ce4e9504575019993550b625262e646c6bf9"
        );

        assert_eq!(
            hex(AttemptDisposition::HostBudgetExceeded
                .canonical_bytes()
                .as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d7631000000000000001c776f726c642d617474656d70742d",
                "646973706f736974696f6e2d7631000100000001",
            )
        );
        assert_eq!(
            AttemptDisposition::HostBudgetExceeded.id().to_string(),
            "e9df2390e40ef2e0ffa5b1fe3132b511bfd761ff3454069a52199e49aaed675f"
        );
        assert_eq!(
            hex(AttemptDisposition::ExternalFailure
                .canonical_bytes()
                .as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d7631000000000000001c776f726c642d617474656d70742d",
                "646973706f736974696f6e2d7631000100000002",
            )
        );
        assert_eq!(
            AttemptDisposition::ExternalFailure.id().to_string(),
            "6f4d8bd5a38189dc426d651ab73dcc9defe6042363827aef156bc01aadfc3c59"
        );
        assert_eq!(
            hex(AttemptDisposition::EngineFailure
                .canonical_bytes()
                .as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d7631000000000000001c776f726c642d617474656d70742d",
                "646973706f736974696f6e2d7631000100000003",
            )
        );
        assert_eq!(
            AttemptDisposition::EngineFailure.id().to_string(),
            "97e95240bfe80c68061c02fb7c540308c46a5f8cfdab6e16a882ded2db0b9779"
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
