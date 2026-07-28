use core::fmt;

use world_context::{
    EvidenceAssimilationInputFingerprint, EvidenceAssimilationPayload,
    EvidenceAssimilationSemanticsId,
};
use world_core::ActorId;
use world_decision::{
    EvidenceAssimilationError, EvidenceAssimilationProposal, EvidenceAssimilator,
};
use world_model::{EpistemicState, EpistemicTransitionError, EpistemicVersion};

/// Why evidence assimilation could not produce a publishable semantic result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EvidenceCoordinationError {
    SemanticsMismatch {
        expected: EvidenceAssimilationSemanticsId,
        actual: EvidenceAssimilationSemanticsId,
    },
    Assimilator(EvidenceAssimilationError),
    InputFingerprintMismatch {
        expected: EvidenceAssimilationInputFingerprint,
        actual: EvidenceAssimilationInputFingerprint,
    },
    ActorMismatch {
        expected: ActorId,
        actual: ActorId,
    },
    VersionMismatch {
        expected: EpistemicVersion,
        actual: EpistemicVersion,
    },
    EvidenceMismatch,
    Transition(EpistemicTransitionError),
}

impl fmt::Display for EvidenceCoordinationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticsMismatch { expected, actual } => write!(
                formatter,
                "evidence input semantics {actual} do not match resolved assimilator semantics {expected}"
            ),
            Self::Assimilator(error) => error.fmt(formatter),
            Self::InputFingerprintMismatch { expected, actual } => write!(
                formatter,
                "evidence proposal input {actual} does not match prepared input {expected}"
            ),
            Self::ActorMismatch { expected, actual } => write!(
                formatter,
                "evidence proposal actor {actual:?} does not match prepared actor {expected:?}"
            ),
            Self::VersionMismatch { expected, actual } => write!(
                formatter,
                "evidence proposal version {} does not match prepared version {}",
                actual.get(),
                expected.get()
            ),
            Self::EvidenceMismatch => {
                formatter.write_str("evidence proposal changed the prepared evidence batch")
            }
            Self::Transition(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for EvidenceCoordinationError {}

/// Engine-private checked successor for one evidence-assimilation input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CoordinatedEvidenceAssimilation {
    input: EvidenceAssimilationInputFingerprint,
    actor: ActorId,
    expected_version: EpistemicVersion,
    successor: EpistemicState,
}

impl CoordinatedEvidenceAssimilation {
    pub(crate) fn into_parts(
        self,
    ) -> (
        EvidenceAssimilationInputFingerprint,
        ActorId,
        EpistemicVersion,
        EpistemicState,
    ) {
        (
            self.input,
            self.actor,
            self.expected_version,
            self.successor,
        )
    }
}

/// Coordinates one exact evidence payload against an already resolved port.
pub(crate) struct EvidenceCoordinator;

impl EvidenceCoordinator {
    pub(crate) fn coordinate(
        current: &EpistemicState,
        input: &EvidenceAssimilationPayload,
        assimilator: &dyn EvidenceAssimilator,
    ) -> Result<CoordinatedEvidenceAssimilation, EvidenceCoordinationError> {
        let expected_semantics = assimilator.semantics_id();
        if input.semantics() != expected_semantics {
            return Err(EvidenceCoordinationError::SemanticsMismatch {
                expected: expected_semantics,
                actual: input.semantics(),
            });
        }
        let proposal = assimilator
            .assimilate(input)
            .map_err(EvidenceCoordinationError::Assimilator)?;
        validate_proposal(input, &proposal)?;
        let successor = proposal
            .apply(current)
            .map_err(EvidenceCoordinationError::Transition)?;
        Ok(CoordinatedEvidenceAssimilation {
            input: input.fingerprint(),
            actor: input.actor(),
            expected_version: input.expected_version(),
            successor,
        })
    }
}

fn validate_proposal(
    input: &EvidenceAssimilationPayload,
    proposal: &EvidenceAssimilationProposal,
) -> Result<(), EvidenceCoordinationError> {
    if proposal.input_fingerprint() != input.fingerprint() {
        return Err(EvidenceCoordinationError::InputFingerprintMismatch {
            expected: input.fingerprint(),
            actual: proposal.input_fingerprint(),
        });
    }
    if proposal.actor() != input.actor() {
        return Err(EvidenceCoordinationError::ActorMismatch {
            expected: input.actor(),
            actual: proposal.actor(),
        });
    }
    if proposal.expected_version() != input.expected_version() {
        return Err(EvidenceCoordinationError::VersionMismatch {
            expected: input.expected_version(),
            actual: proposal.expected_version(),
        });
    }
    if proposal.evidence() != input.evidence() {
        return Err(EvidenceCoordinationError::EvidenceMismatch);
    }
    Ok(())
}
