use core::fmt;

use world_core::{ActorId, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest};
use world_model::{EpistemicVersion, EvidenceDeliveryId, EvidenceProvenance, EvidenceRecord};

use crate::{EvidenceAssimilationInputFingerprint, EvidenceAssimilationSemanticsId};

const INPUT_SCHEMA_VERSION: u16 = 3;
const INPUT_DOMAIN: CanonicalDomain = match CanonicalDomain::new("evidence-assimilation-input-v3") {
    Ok(domain) => domain,
    Err(_) => panic!("evidence assimilation input domain must be valid"),
};

/// Why an evidence-assimilation input could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceAssimilationPayloadError {
    /// An assimilation invocation must contain at least one evidence record.
    EmptyEvidence,
    /// One record belonged to a different actor-local epistemic history.
    WrongObserver {
        /// Evidence whose observer did not match the payload actor.
        evidence: EvidenceDeliveryId,
    },
    /// A canonical input identity could not represent one of its fields.
    Canonical(CanonicalError),
}

impl fmt::Display for EvidenceAssimilationPayloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEvidence => formatter.write_str("evidence assimilation input is empty"),
            Self::WrongObserver { evidence } => {
                write!(
                    formatter,
                    "evidence {evidence} belongs to a different observer"
                )
            }
            Self::Canonical(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for EvidenceAssimilationPayloadError {}

impl From<CanonicalError> for EvidenceAssimilationPayloadError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

/// Exact actor-safe input for one evidence-assimilation invocation.
///
/// The batch is canonicalized by actor-local delivery generation. It contains
/// no source revision, authority record, scheduler coordinate, or host
/// delivery metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceAssimilationPayload {
    actor: ActorId,
    expected_version: EpistemicVersion,
    evidence: Vec<EvidenceRecord>,
    semantics: EvidenceAssimilationSemanticsId,
    fingerprint: EvidenceAssimilationInputFingerprint,
}

impl EvidenceAssimilationPayload {
    /// Validates and constructs one nonempty actor-local assimilation input.
    pub fn new(
        actor: ActorId,
        expected_version: EpistemicVersion,
        mut evidence: Vec<EvidenceRecord>,
        semantics: EvidenceAssimilationSemanticsId,
    ) -> Result<Self, EvidenceAssimilationPayloadError> {
        if evidence.is_empty() {
            return Err(EvidenceAssimilationPayloadError::EmptyEvidence);
        }
        if let Some(record) = evidence.iter().find(|record| record.observer() != actor) {
            return Err(EvidenceAssimilationPayloadError::WrongObserver {
                evidence: record.id(),
            });
        }
        evidence.sort_by_key(|record| (record.generation(), record.id()));
        let fingerprint =
            evidence_assimilation_fingerprint(actor, expected_version, &evidence, semantics)?;
        Ok(Self {
            actor,
            expected_version,
            evidence,
            semantics,
            fingerprint,
        })
    }

    /// Returns the actor whose epistemic state is being evaluated.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the actor-local version observed by the coordinator.
    #[must_use]
    pub const fn expected_version(&self) -> EpistemicVersion {
        self.expected_version
    }

    /// Returns the exact nonempty evidence batch in canonical order.
    #[must_use]
    pub fn evidence(&self) -> &[EvidenceRecord] {
        &self.evidence
    }

    /// Returns the exact assimilator behavior identity.
    #[must_use]
    pub const fn semantics(&self) -> EvidenceAssimilationSemanticsId {
        self.semantics
    }

    /// Returns the canonical identity of this complete actor-safe input.
    #[must_use]
    pub const fn fingerprint(&self) -> EvidenceAssimilationInputFingerprint {
        self.fingerprint
    }
}

fn evidence_assimilation_fingerprint(
    actor: ActorId,
    expected_version: EpistemicVersion,
    evidence: &[EvidenceRecord],
    semantics: EvidenceAssimilationSemanticsId,
) -> Result<EvidenceAssimilationInputFingerprint, CanonicalError> {
    let mut writer = CanonicalWriter::new(INPUT_DOMAIN);
    writer.write_u16(INPUT_SCHEMA_VERSION);
    writer.write_bytes(actor.as_bytes())?;
    writer.write_u64(expected_version.get());
    writer.write_sequence(evidence, write_evidence)?;
    writer.write_bytes(semantics.as_bytes())?;
    Ok(EvidenceAssimilationInputFingerprint(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

pub(crate) fn write_evidence(
    writer: &mut CanonicalWriter,
    evidence: &EvidenceRecord,
) -> Result<(), CanonicalError> {
    writer.write_bytes(evidence.id().as_bytes())?;
    writer.write_bytes(evidence.observer().as_bytes())?;
    writer.write_u64(evidence.generation().get());
    match evidence.provenance() {
        EvidenceProvenance::DirectItemTransfer(event) => {
            writer.write_discriminant(0);
            writer.write_bytes(event.actor().as_bytes())?;
            writer.write_bytes(event.item().as_bytes())?;
            writer.write_bytes(event.source().as_bytes())?;
            writer.write_bytes(event.destination().as_bytes())
        }
        EvidenceProvenance::DirectItemAbsent(observation) => {
            writer.write_discriminant(3);
            writer.write_bytes(observation.item().as_bytes())?;
            writer.write_bytes(observation.expected_container().as_bytes())
        }
        EvidenceProvenance::DirectActorDeparture(observation) => {
            writer.write_discriminant(1);
            writer.write_bytes(observation.actor().as_bytes())?;
            writer.write_bytes(observation.source().as_bytes())?;
            writer.write_bytes(observation.destination().as_bytes())
        }
        EvidenceProvenance::DirectActorArrival(observation) => {
            writer.write_discriminant(2);
            writer.write_bytes(observation.actor().as_bytes())?;
            writer.write_bytes(observation.source().as_bytes())?;
            writer.write_bytes(observation.destination().as_bytes())
        }
    }
}
