use core::fmt;

use world_context::{
    ContainmentIntentContextBuild, GroundedIntentCandidateId, IntentInputFingerprint,
    IntentPolicySemanticsId,
};
use world_decision::{IntentDecision, IntentPolicy, IntentPolicyError};
use world_model::{AgencyState, AgencyTransitionError, Intent, IntentGeneration};

/// Why an intent-policy result could not become a checked agency operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum IntentCoordinationError {
    SemanticsMismatch {
        expected: IntentPolicySemanticsId,
        actual: IntentPolicySemanticsId,
    },
    Policy(IntentPolicyError),
    InputFingerprintMismatch {
        expected: IntentInputFingerprint,
        actual: IntentInputFingerprint,
    },
    UnexpectedNoCandidate,
    CandidateUnavailable {
        candidate: GroundedIntentCandidateId,
    },
    PrivateResolutionMissing {
        candidate: GroundedIntentCandidateId,
    },
    CandidateResolutionMismatch {
        candidate: GroundedIntentCandidateId,
    },
    AgencyContextMismatch,
    Agency(AgencyTransitionError),
}

impl fmt::Display for IntentCoordinationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticsMismatch { expected, actual } => write!(
                formatter,
                "intent input semantics {actual} do not match resolved policy semantics {expected}"
            ),
            Self::Policy(error) => error.fmt(formatter),
            Self::InputFingerprintMismatch { expected, actual } => write!(
                formatter,
                "intent decision input {actual} does not match prepared input {expected}"
            ),
            Self::UnexpectedNoCandidate => formatter.write_str(
                "intent policy declined a complete input that supplied grounded candidates",
            ),
            Self::CandidateUnavailable { candidate } => write!(
                formatter,
                "intent policy selected candidate {candidate} outside the supplied set"
            ),
            Self::PrivateResolutionMissing { candidate } => write!(
                formatter,
                "intent candidate {candidate} has no paired private resolution"
            ),
            Self::CandidateResolutionMismatch { candidate } => write!(
                formatter,
                "private resolution for intent candidate {candidate} does not match its public material"
            ),
            Self::AgencyContextMismatch => formatter.write_str(
                "grounded intent candidate no longer matches the accepted agency context",
            ),
            Self::Agency(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for IntentCoordinationError {}

/// Engine-private checked result of one grounded intent review.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CoordinatedIntentReview {
    Adopt {
        input: IntentInputFingerprint,
        intent: Intent,
    },
    NoChange {
        input: IntentInputFingerprint,
    },
}

impl CoordinatedIntentReview {
    #[cfg(test)]
    pub(crate) const fn input_fingerprint(self) -> IntentInputFingerprint {
        match self {
            Self::Adopt { input, .. } | Self::NoChange { input } => input,
        }
    }

    #[cfg(test)]
    pub(crate) const fn adopted_intent(self) -> Option<Intent> {
        match self {
            Self::Adopt { intent, .. } => Some(intent),
            Self::NoChange { .. } => None,
        }
    }
}

/// Coordinates grounded intent selection without exposing private resolution.
pub(crate) struct IntentCoordinator;

impl IntentCoordinator {
    pub(crate) fn coordinate(
        current: &AgencyState,
        build: ContainmentIntentContextBuild,
        generation: IntentGeneration,
        policy: &dyn IntentPolicy,
    ) -> Result<CoordinatedIntentReview, IntentCoordinationError> {
        let (input, resolution) = build.into_parts();
        let expected_semantics = policy.semantics_id();
        if input.policy_semantics() != expected_semantics {
            return Err(IntentCoordinationError::SemanticsMismatch {
                expected: expected_semantics,
                actual: input.policy_semantics(),
            });
        }
        let decision = policy
            .decide(&input)
            .map_err(IntentCoordinationError::Policy)?;
        if decision.input_fingerprint() != input.fingerprint() {
            return Err(IntentCoordinationError::InputFingerprintMismatch {
                expected: input.fingerprint(),
                actual: decision.input_fingerprint(),
            });
        }

        match decision {
            IntentDecision::NoCandidate { .. } => {
                if !input.candidates().candidates().is_empty() {
                    return Err(IntentCoordinationError::UnexpectedNoCandidate);
                }
                Ok(CoordinatedIntentReview::NoChange {
                    input: input.fingerprint(),
                })
            }
            IntentDecision::Adopt { candidate, .. } => {
                let public = input
                    .candidates()
                    .candidates()
                    .iter()
                    .find(|supplied| supplied.id() == candidate)
                    .copied()
                    .ok_or(IntentCoordinationError::CandidateUnavailable { candidate })?;
                let resolved = resolution
                    .resolve(candidate)
                    .copied()
                    .ok_or(IntentCoordinationError::PrivateResolutionMissing { candidate })?;
                if resolved.candidate() != public.id()
                    || resolved.actor() != public.actor()
                    || resolved.desired() != public.desired()
                    || resolved.actor() != input.actor()
                {
                    return Err(IntentCoordinationError::CandidateResolutionMismatch { candidate });
                }
                if current
                    .intents()
                    .iter()
                    .any(|intent| intent.actor() == input.actor() && !intent.status().is_terminal())
                {
                    return Err(IntentCoordinationError::AgencyContextMismatch);
                }

                let intent = Intent::adopt(resolved.actor(), generation, resolved.desired());
                current
                    .adopt_intent(intent)
                    .map_err(IntentCoordinationError::Agency)?;
                Ok(CoordinatedIntentReview::Adopt {
                    input: input.fingerprint(),
                    intent,
                })
            }
        }
    }
}
