use core::fmt;

use world_context::{
    ContainmentIntentPayload, GroundedIntentCandidateId, IntentInputFingerprint,
    IntentPolicySemanticsId,
};
use world_core::CanonicalDomain;

use super::{IntentPolicy, identity};

const INTENT_POLICY_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("baseline-intent-policy-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("baseline intent-policy domain must be valid"),
    };

/// Why the baseline intent policy refused an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentPolicyError {
    /// The input was constructed for a different intent-policy behavior.
    SemanticsMismatch {
        /// Behavior identity required by this implementation.
        expected: IntentPolicySemanticsId,
        /// Behavior identity committed by the input.
        actual: IntentPolicySemanticsId,
    },
}

impl fmt::Display for IntentPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticsMismatch { expected, actual } => write!(
                formatter,
                "intent input semantics {actual} do not match baseline semantics {expected}"
            ),
        }
    }
}

impl std::error::Error for IntentPolicyError {}

/// Closed grounded-intent decision bound to one exact actor-safe input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentDecision {
    /// Adopt one candidate supplied by the paired context build.
    Adopt {
        /// Supplied grounded candidate identity.
        candidate: GroundedIntentCandidateId,
        /// Exact actor-safe input against which selection occurred.
        input: IntentInputFingerprint,
    },
    /// The complete grounded set contained no adoptable candidate.
    NoCandidate {
        /// Exact actor-safe input whose candidate set was empty.
        input: IntentInputFingerprint,
    },
}

impl IntentDecision {
    /// Returns the exact actor-safe input fingerprint.
    #[must_use]
    pub const fn input_fingerprint(self) -> IntentInputFingerprint {
        match self {
            Self::Adopt { input, .. } | Self::NoCandidate { input } => input,
        }
    }

    /// Returns the selected supplied candidate, if any.
    #[must_use]
    pub const fn selected_candidate(self) -> Option<GroundedIntentCandidateId> {
        match self {
            Self::Adopt { candidate, .. } => Some(candidate),
            Self::NoCandidate { .. } => None,
        }
    }
}

/// Deterministic policy that selects the first canonical supplied intent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BaselineIntentPolicy {
    _private: (),
}

impl BaselineIntentPolicy {
    /// Constructs the baseline intent policy.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Returns the typed behavior identity used by compatible context input.
    #[must_use]
    pub fn semantics_id(self) -> IntentPolicySemanticsId {
        IntentPolicySemanticsId::from_bytes(identity(INTENT_POLICY_DOMAIN))
    }

    /// Returns the catalog-facing implementation identity.
    #[must_use]
    pub fn implementation_id(self) -> [u8; 32] {
        self.semantics_id().into_bytes()
    }

    /// Selects only from the supplied grounded candidate set.
    pub fn decide(
        self,
        input: &ContainmentIntentPayload,
    ) -> Result<IntentDecision, IntentPolicyError> {
        let expected = self.semantics_id();
        let actual = input.policy_semantics();
        if actual != expected {
            return Err(IntentPolicyError::SemanticsMismatch { expected, actual });
        }
        Ok(match input.candidates().candidates().first() {
            Some(candidate) => IntentDecision::Adopt {
                candidate: candidate.id(),
                input: input.fingerprint(),
            },
            None => IntentDecision::NoCandidate {
                input: input.fingerprint(),
            },
        })
    }
}

impl IntentPolicy for BaselineIntentPolicy {
    fn implementation_id(&self) -> [u8; 32] {
        (*self).implementation_id()
    }

    fn decide(
        &self,
        input: &ContainmentIntentPayload,
    ) -> Result<IntentDecision, IntentPolicyError> {
        (*self).decide(input)
    }
}
