use core::fmt;

use world_context::{
    EvidenceAssimilationInputFingerprint, EvidenceAssimilationPayload,
    EvidenceAssimilationSemanticsId,
};
use world_core::{ActorId, CanonicalDomain};
use world_model::{EpistemicState, EpistemicTransitionError, EpistemicVersion, EvidenceRecord};

use super::{EvidenceAssimilator, identity};

const EVIDENCE_ASSIMILATOR_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("baseline-evidence-assimilator-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("baseline evidence-assimilator domain must be valid"),
    };

/// Why the baseline assimilator refused an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceAssimilationError {
    /// The input was constructed for a different assimilator behavior.
    SemanticsMismatch {
        /// Behavior identity required by this implementation.
        expected: EvidenceAssimilationSemanticsId,
        /// Behavior identity committed by the input.
        actual: EvidenceAssimilationSemanticsId,
    },
}

impl fmt::Display for EvidenceAssimilationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticsMismatch { expected, actual } => write!(
                formatter,
                "evidence input semantics {actual} do not match baseline semantics {expected}"
            ),
        }
    }
}

impl std::error::Error for EvidenceAssimilationError {}

/// One exact actor-local epistemic transition proposal.
///
/// Applying this proposal still uses the model's version and provenance
/// checks. Constructing it grants no accepted-state mutation authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceAssimilationProposal {
    input: EvidenceAssimilationInputFingerprint,
    actor: ActorId,
    expected_version: EpistemicVersion,
    evidence: Vec<EvidenceRecord>,
}

impl EvidenceAssimilationProposal {
    /// Returns the exact policy input that produced this proposal.
    #[must_use]
    pub const fn input_fingerprint(&self) -> EvidenceAssimilationInputFingerprint {
        self.input
    }

    /// Returns the actor whose epistemic state may change.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the actor-local version the proposal expects.
    #[must_use]
    pub const fn expected_version(&self) -> EpistemicVersion {
        self.expected_version
    }

    /// Returns the exact nonempty evidence batch.
    #[must_use]
    pub fn evidence(&self) -> &[EvidenceRecord] {
        &self.evidence
    }

    /// Constructs the checked epistemic successor against one accepted base.
    pub fn apply(
        &self,
        current: &EpistemicState,
    ) -> Result<EpistemicState, EpistemicTransitionError> {
        current.assimilate(self.actor, self.expected_version, self.evidence.clone())
    }
}

/// Deterministic baseline that accepts the exact projected evidence batch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BaselineEvidenceAssimilator {
    _private: (),
}

impl BaselineEvidenceAssimilator {
    /// Constructs the baseline evidence assimilator.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Returns the typed behavior identity used by compatible context input.
    #[must_use]
    pub fn semantics_id(self) -> EvidenceAssimilationSemanticsId {
        EvidenceAssimilationSemanticsId::from_bytes(identity(EVIDENCE_ASSIMILATOR_DOMAIN))
    }

    /// Returns the catalog-facing implementation identity.
    #[must_use]
    pub fn implementation_id(self) -> [u8; 32] {
        self.semantics_id().into_bytes()
    }

    /// Proposes assimilation of the exact supplied records.
    pub fn assimilate(
        self,
        input: &EvidenceAssimilationPayload,
    ) -> Result<EvidenceAssimilationProposal, EvidenceAssimilationError> {
        let expected = self.semantics_id();
        let actual = input.semantics();
        if actual != expected {
            return Err(EvidenceAssimilationError::SemanticsMismatch { expected, actual });
        }
        Ok(EvidenceAssimilationProposal {
            input: input.fingerprint(),
            actor: input.actor(),
            expected_version: input.expected_version(),
            evidence: input.evidence().to_vec(),
        })
    }
}

impl EvidenceAssimilator for BaselineEvidenceAssimilator {
    fn implementation_id(&self) -> [u8; 32] {
        (*self).implementation_id()
    }

    fn assimilate(
        &self,
        input: &EvidenceAssimilationPayload,
    ) -> Result<EvidenceAssimilationProposal, EvidenceAssimilationError> {
        (*self).assimilate(input)
    }
}
