//! Pure action-policy decisions over actor-safe grounded candidates.
//!
//! This crate has no snapshot, runtime command, scheduler, or mutation
//! dependency. Policies can return only identities supplied by
//! `world-context` for the exact input fingerprint.

use core::fmt;

use world_context::{
    ActionContextPayload, ActionInputFingerprint, ActionPolicySemanticsId,
    GroundedActionCandidateId,
};
use world_core::{CanonicalDomain, CanonicalWriter, ContentDigest};

mod action_codec;
mod lifecycle;

pub use action_codec::{
    ActionDecisionCodecError, ActionDecisionSchemaId, action_decision_schema,
    decode_action_decision, encode_action_decision,
};
pub use lifecycle::{
    ActivityActionDirective, ActivityAdvancementDecision, ActivityController,
    ActivityControllerError, ActivityInitializationDecision, ActivityInitializationStart,
    AppraisalEvaluationError, AppraisalEvaluator, BaselineActivityController,
    BaselineAppraisalEvaluator, BaselineEvidenceAssimilator, BaselineIntentPolicy,
    ContainmentActionDirective, ContainmentAppraisalEvaluation, EvidenceAssimilationError,
    EvidenceAssimilationProposal, EvidenceAssimilator, IntentDecision, IntentPolicy,
    IntentPolicyError, RelocationActionDirective, activity_state_schema,
};

const POLICY_SCHEMA_VERSION: u16 = 1;
const BASELINE_POLICY_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("baseline-action-policy-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("baseline action policy domain must be valid"),
    };

/// A closed action-policy result bound to one exact actor-safe input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionDecision {
    /// Select one candidate identity supplied in the input set.
    Select {
        /// Supplied candidate identity.
        candidate: GroundedActionCandidateId,
        /// Exact actor-safe input against which selection occurred.
        input: ActionInputFingerprint,
    },
    /// The complete supplied set had no applicable candidate.
    NoApplicableAction {
        /// Exact actor-safe input whose candidate set was empty.
        input: ActionInputFingerprint,
    },
}

impl ActionDecision {
    /// Returns the exact actor-safe input fingerprint.
    #[must_use]
    pub const fn input_fingerprint(self) -> ActionInputFingerprint {
        match self {
            Self::Select { input, .. } | Self::NoApplicableAction { input } => input,
        }
    }

    /// Returns the selected supplied ID, if the policy selected an action.
    #[must_use]
    pub const fn selected_candidate(self) -> Option<GroundedActionCandidateId> {
        match self {
            Self::Select { candidate, .. } => Some(candidate),
            Self::NoApplicableAction { .. } => None,
        }
    }
}

/// Why a policy refused an input before making a logical decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionPolicyError {
    /// The payload was built for a different behavior identity.
    SemanticsMismatch {
        /// Behavior identity of this policy.
        expected: ActionPolicySemanticsId,
        /// Behavior identity committed by the input.
        actual: ActionPolicySemanticsId,
    },
    /// The policy exhausted its own bounded evaluation rule.
    EvaluationFailed,
}

impl fmt::Display for ActionPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticsMismatch { expected, actual } => write!(
                formatter,
                "action input policy semantics {actual} do not match baseline semantics {expected}"
            ),
            Self::EvaluationFailed => formatter.write_str("action policy evaluation failed"),
        }
    }
}

impl std::error::Error for ActionPolicyError {}

/// Object-safe policy boundary for one grounded action opportunity.
///
/// Implementations receive only actor-safe context and can select only an ID
/// supplied by that exact input.
pub trait ActionPolicy: Send + Sync + 'static {
    /// Returns the typed behavior identity committed into compatible input.
    fn semantics_id(&self) -> ActionPolicySemanticsId;

    /// Selects from one exact grounded candidate set.
    fn decide(&self, input: &ActionContextPayload) -> Result<ActionDecision, ActionPolicyError>;
}

/// Deterministic baseline policy for the first grounded-action vertical.
///
/// Candidate construction already establishes canonical actor-safe order, so
/// the baseline selects the first supplied candidate and otherwise returns
/// `NoApplicableAction`. It does not invent candidates or inspect private
/// model state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BaselineActionPolicy {
    _private: (),
}

impl BaselineActionPolicy {
    /// Constructs the fixed baseline policy.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Returns the exact behavior identity bound into compatible inputs.
    #[must_use]
    pub fn semantics_id(self) -> ActionPolicySemanticsId {
        let mut writer = CanonicalWriter::new(BASELINE_POLICY_DOMAIN);
        writer.write_u16(POLICY_SCHEMA_VERSION);
        ActionPolicySemanticsId::from_bytes(
            ContentDigest::of_canonical(&writer.finish()).into_bytes(),
        )
    }

    /// Selects only from the candidates supplied in one actor-safe payload.
    pub fn decide(self, input: &ActionContextPayload) -> Result<ActionDecision, ActionPolicyError> {
        let expected = self.semantics_id();
        let actual = input.policy_semantics();
        if actual != expected {
            return Err(ActionPolicyError::SemanticsMismatch { expected, actual });
        }
        Ok(select_first(
            input
                .candidates()
                .candidates()
                .first()
                .map(|candidate| candidate.id()),
            input.input_fingerprint(),
        ))
    }
}

impl ActionPolicy for BaselineActionPolicy {
    fn semantics_id(&self) -> ActionPolicySemanticsId {
        (*self).semantics_id()
    }

    fn decide(&self, input: &ActionContextPayload) -> Result<ActionDecision, ActionPolicyError> {
        (*self).decide(input)
    }
}

fn select_first(
    candidate: Option<GroundedActionCandidateId>,
    input: ActionInputFingerprint,
) -> ActionDecision {
    match candidate {
        Some(candidate) => ActionDecision::Select { candidate, input },
        None => ActionDecision::NoApplicableAction { input },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplied_candidate_is_selected_without_rewriting_its_identity() {
        let candidate = GroundedActionCandidateId::from_bytes([0x31; 32]);
        let input = ActionInputFingerprint::from_bytes([0x41; 32]);

        assert_eq!(
            select_first(Some(candidate), input),
            ActionDecision::Select { candidate, input }
        );
    }

    #[test]
    fn empty_input_has_a_closed_non_selection_result() {
        let input = ActionInputFingerprint::from_bytes([0x41; 32]);

        assert_eq!(
            select_first(None, input),
            ActionDecision::NoApplicableAction { input }
        );
    }

    #[test]
    fn baseline_semantics_are_stable_and_not_an_arbitrary_decoded_value() {
        let baseline = BaselineActionPolicy::new().semantics_id();

        assert_eq!(baseline, BaselineActionPolicy::new().semantics_id());
        assert_ne!(baseline, ActionPolicySemanticsId::from_bytes([0x55; 32]));
    }

    #[test]
    fn bounded_policy_failure_is_distinct_from_incompatible_input() {
        assert_eq!(
            ActionPolicyError::EvaluationFailed.to_string(),
            "action policy evaluation failed"
        );
        assert_ne!(
            ActionPolicyError::EvaluationFailed,
            ActionPolicyError::SemanticsMismatch {
                expected: ActionPolicySemanticsId::from_bytes([0x11; 32]),
                actual: ActionPolicySemanticsId::from_bytes([0x12; 32]),
            }
        );
    }
}
