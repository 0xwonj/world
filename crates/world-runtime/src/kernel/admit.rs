use core::fmt;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter, ContentDigest, SimMoment};
use world_model::{CommandEnvelope, CommandId, CommandSource};

use crate::authority::AuthorityRecordId;
use crate::execution::{EpochLineageId, ExternalInputBindingDigest, ExternalInputNamespaceId};

/// Canonical schema of an admitted input request.
pub const INPUT_REQUEST_SCHEMA_VERSION: u16 = 1;
const INPUT_REQUEST_NAMESPACE_SCHEMA_VERSION: u16 = 1;

const INPUT_REQUEST_DOMAIN: CanonicalDomain = match CanonicalDomain::new("input-request-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("input request domain must be valid"),
};

const INPUT_REQUEST_NAMESPACE_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("input-request-namespace-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("input request namespace domain must be valid"),
    };

/// Host-issued identity of one input admission request.
///
/// Zero is a valid value. Issuance policy belongs to the input namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputId(u64);

impl InputId {
    /// Constructs an input identity from its exact namespace-local value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the exact namespace-local value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Canonical identity of one input request body.
///
/// [`InputId`] is deliberately omitted so retained lookup can distinguish
/// exact retries from reuse of the same identity with different content.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputRequestFingerprint(ContentDigest);

impl InputRequestFingerprint {
    fn derive(effective: SimMoment, command: &CommandEnvelope) -> Self {
        Self(ContentDigest::of_canonical(&input_request_bytes(
            effective,
            command.source(),
            command.id(),
            command.fingerprint().as_bytes(),
        )))
    }

    /// Returns the exact fingerprint bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Consumes the fingerprint and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0.into_bytes()
    }
}

impl fmt::Display for InputRequestFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for InputRequestFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "InputRequestFingerprint({self})")
    }
}

/// One checked command submitted for delivery at an exact simulation moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmitRequest {
    id: InputId,
    effective: SimMoment,
    command: CommandEnvelope,
    fingerprint: InputRequestFingerprint,
}

impl AdmitRequest {
    /// Captures a checked command and derives its singular request identity.
    #[must_use]
    pub fn new(id: InputId, effective: SimMoment, command: CommandEnvelope) -> Self {
        let fingerprint = InputRequestFingerprint::derive(effective, &command);
        Self {
            id,
            effective,
            command,
            fingerprint,
        }
    }

    /// Returns the input request identity.
    #[must_use]
    pub const fn id(&self) -> InputId {
        self.id
    }

    /// Returns the requested delivery moment.
    #[must_use]
    pub const fn effective(&self) -> SimMoment {
        self.effective
    }

    /// Returns the complete checked command.
    #[must_use]
    pub const fn command(&self) -> &CommandEnvelope {
        &self.command
    }

    /// Returns the canonical request fingerprint.
    #[must_use]
    pub const fn fingerprint(&self) -> InputRequestFingerprint {
        self.fingerprint
    }
}

/// Retained successful result of one input admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmitOutcome {
    /// The input was captured by an authority record and scheduled.
    #[non_exhaustive]
    Scheduled {
        /// Record that captured the input and ledger result.
        record: AuthorityRecordId,
        /// Actual scheduled delivery moment.
        effective: SimMoment,
    },
}

impl AdmitOutcome {
    #[must_use]
    pub(crate) const fn scheduled(record: AuthorityRecordId, effective: SimMoment) -> Self {
        Self::Scheduled { record, effective }
    }

    /// Returns the record that captured the input.
    #[must_use]
    pub const fn record(self) -> AuthorityRecordId {
        match self {
            Self::Scheduled { record, .. } => record,
        }
    }

    /// Returns the actual scheduled delivery moment.
    #[must_use]
    pub const fn effective(self) -> SimMoment {
        match self {
            Self::Scheduled { effective, .. } => effective,
        }
    }
}

pub(crate) fn derive_input_request_namespace(
    lineage: EpochLineageId,
    binding: ExternalInputBindingDigest,
) -> ExternalInputNamespaceId {
    ExternalInputNamespaceId::from_bytes(
        ContentDigest::of_canonical(&input_request_namespace_bytes(lineage, binding)).into_bytes(),
    )
}

fn input_request_bytes(
    effective: SimMoment,
    source: CommandSource,
    command_id: CommandId,
    command_fingerprint: &[u8; 32],
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(INPUT_REQUEST_DOMAIN);
    writer.write_u16(INPUT_REQUEST_SCHEMA_VERSION);
    write_moment(&mut writer, effective);
    write_fixed_bytes(&mut writer, source.as_bytes());
    writer.write_u64(command_id.get());
    write_fixed_bytes(&mut writer, command_fingerprint);
    writer.finish()
}

fn input_request_namespace_bytes(
    lineage: EpochLineageId,
    binding: ExternalInputBindingDigest,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(INPUT_REQUEST_NAMESPACE_DOMAIN);
    writer.write_u16(INPUT_REQUEST_NAMESPACE_SCHEMA_VERSION);
    write_fixed_bytes(&mut writer, lineage.as_bytes());
    write_fixed_bytes(&mut writer, binding.as_bytes());
    writer.finish()
}

fn write_moment(writer: &mut CanonicalWriter, moment: SimMoment) {
    writer.write_u64(moment.time().ticks());
    writer.write_u64(moment.microstep().get());
}

fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width identity length must fit the canonical protocol");
    }
}

#[cfg(test)]
mod tests {
    use world_core::{Microstep, SimTime};

    use crate::kernel::fixtures;

    use super::*;

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
            encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        encoded
    }

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(ticks), Microstep::new(microstep))
    }

    #[test]
    fn input_and_namespace_preimages_are_byte_complete() {
        let request_bytes = input_request_bytes(
            moment(7, 9),
            CommandSource::from_bytes([0x11; 32]),
            CommandId::new(5),
            &[0x22; 32],
        );
        let namespace_bytes = input_request_namespace_bytes(
            EpochLineageId::from_bytes([0x33; 32]),
            ExternalInputBindingDigest::from_bytes([0x44; 32]),
        );

        assert_eq!(
            hex(request_bytes.as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d76310000000000000010696e7075742d72657175",
                "6573742d763100010000000000000007000000000000000900000000000000201111111111",
                "11111111111111111111111111111111111111111111111111111100000000000000050000",
                "00000000002022222222222222222222222222222222222222222222222222222222222222",
                "22"
            )
        );
        assert_eq!(
            ContentDigest::of_canonical(&request_bytes).to_string(),
            "e3a402fcfd233ae8f4d6e6dc00ddaf5ecd07853207038786fd8ebb485348b5dd"
        );
        assert_eq!(
            hex(namespace_bytes.as_bytes()),
            "776f726c642d63616e6f6e6963616c2d7631000000000000001a696e7075742d726571756573742d6e616d6573706163652d763100010000000000000020333333333333333333333333333333333333333333333333333333333333333300000000000000204444444444444444444444444444444444444444444444444444444444444444"
        );
        assert_eq!(
            ContentDigest::of_canonical(&namespace_bytes).to_string(),
            "577632494e43e07ff0dc0cf0e8086c95513b575e7c6b9cfcb3b4229deb561830"
        );
    }

    #[test]
    fn input_identity_is_omitted_but_request_content_is_committed() {
        let command = fixtures::command(0x51, 12);
        let first = AdmitRequest::new(InputId::new(0), moment(3, 4), command.clone());
        let retry = AdmitRequest::new(InputId::new(99), moment(3, 4), command.clone());
        let other_moment = AdmitRequest::new(InputId::new(0), moment(3, 5), command);
        let other_command =
            AdmitRequest::new(InputId::new(0), moment(3, 4), fixtures::command(0x52, 12));

        assert_eq!(InputId::new(0).get(), 0);
        assert_eq!(first.fingerprint(), retry.fingerprint());
        assert_ne!(first.fingerprint(), other_moment.fingerprint());
        assert_ne!(first.fingerprint(), other_command.fingerprint());
    }

    #[test]
    fn namespace_commits_lineage_and_external_binding() {
        let lineage = EpochLineageId::from_bytes([0x61; 32]);
        let binding = ExternalInputBindingDigest::from_bytes([0x62; 32]);
        let namespace = derive_input_request_namespace(lineage, binding);

        assert_ne!(
            namespace,
            derive_input_request_namespace(EpochLineageId::from_bytes([0x63; 32]), binding)
        );
        assert_ne!(
            namespace,
            derive_input_request_namespace(
                lineage,
                ExternalInputBindingDigest::from_bytes([0x64; 32])
            )
        );
    }

    #[test]
    fn scheduled_outcome_retains_materialized_record_and_moment() {
        let record = AuthorityRecordId::from_bytes([0x71; 32]);
        let effective = moment(8, 2);
        let outcome = AdmitOutcome::scheduled(record, effective);

        assert_eq!(outcome.record(), record);
        assert_eq!(outcome.effective(), effective);
    }
}
