use core::fmt;

use world_core::{
    ActorId, CanonicalBytes, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest,
    EntityId,
};

use crate::EvidenceDeliveryId;

/// Canonical schema version of the containment-appraisal material fingerprint.
pub const CONTAINMENT_APPRAISAL_MATERIAL_SCHEMA_VERSION: u16 = 1;

const CONTAINMENT_APPRAISAL_MATERIAL_CANONICAL_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("containment-appraisal-material-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("containment-appraisal material domain must be valid"),
    };

/// Canonical material identity of one actor-relative containment appraisal.
///
/// Coordinators may retain this value to detect whether a later appraisal is
/// materially different. It is not accepted world truth.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContainmentAppraisalFingerprint(ContentDigest);

impl ContainmentAppraisalFingerprint {
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

impl fmt::Display for ContainmentAppraisalFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for ContainmentAppraisalFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ContainmentAppraisalFingerprint({self})")
    }
}

/// Derived actor-relative meaning of a directly observed item transfer.
///
/// This projection is policy input, not an [`crate::AcceptedState`] partition.
/// Its evidence reference is resolved while projecting one immutable
/// epistemic snapshot; retaining it across a later snapshot requires the
/// coordinator's ordinary freshness protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainmentAppraisal {
    actor: ActorId,
    item: EntityId,
    believed_current_container: EntityId,
    restore_container: EntityId,
    supporting_evidence: EvidenceDeliveryId,
}

impl ContainmentAppraisal {
    /// Constructs actor-safe appraisal material from accepted epistemic
    /// projection.
    #[must_use]
    pub const fn new(
        actor: ActorId,
        item: EntityId,
        believed_current_container: EntityId,
        restore_container: EntityId,
        supporting_evidence: EvidenceDeliveryId,
    ) -> Self {
        Self {
            actor,
            item,
            believed_current_container,
            restore_container,
            supporting_evidence,
        }
    }

    /// Returns the appraising actor.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the observed item.
    #[must_use]
    pub const fn item(self) -> EntityId {
        self.item
    }

    /// Returns the actor's currently believed direct container.
    #[must_use]
    pub const fn believed_current_container(self) -> EntityId {
        self.believed_current_container
    }

    /// Returns the preceding container that restoration would target.
    #[must_use]
    pub const fn restore_container(self) -> EntityId {
        self.restore_container
    }

    /// Returns the accepted evidence supporting this projection.
    #[must_use]
    pub const fn supporting_evidence(self) -> EvidenceDeliveryId {
        self.supporting_evidence
    }

    /// Returns the canonical identity of actor-relative meaning.
    ///
    /// Evidence provenance remains part of the appraisal value but is
    /// intentionally excluded here: a later observation supporting the same
    /// meaning must not trigger intent reconsideration by itself.
    #[must_use]
    pub fn material_fingerprint(self) -> ContainmentAppraisalFingerprint {
        ContainmentAppraisalFingerprint(ContentDigest::of_canonical(&self.material_preimage()))
    }

    fn material_preimage(self) -> CanonicalBytes {
        let encoded = (|| -> Result<_, CanonicalError> {
            let mut writer = CanonicalWriter::new(CONTAINMENT_APPRAISAL_MATERIAL_CANONICAL_DOMAIN);
            writer.write_u16(CONTAINMENT_APPRAISAL_MATERIAL_SCHEMA_VERSION);
            writer.write_bytes(self.actor.as_bytes())?;
            writer.write_bytes(self.item.as_bytes())?;
            writer.write_bytes(self.believed_current_container.as_bytes())?;
            writer.write_bytes(self.restore_container.as_bytes())?;
            Ok(writer.finish())
        })();
        match encoded {
            Ok(bytes) => bytes,
            Err(error) => {
                unreachable!("fixed containment-appraisal material must be canonical: {error}")
            }
        }
    }
}
