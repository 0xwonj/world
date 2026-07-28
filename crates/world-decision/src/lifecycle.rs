use world_context::{
    ActivityAdvancementPayload, ActivityControllerSemanticsId, AppraisalEvaluatorSemanticsId,
    ContainmentActivityInitializationPayload, ContainmentAppraisalPayload,
    ContainmentIntentPayload, EvidenceAssimilationPayload, EvidenceAssimilationSemanticsId,
    IntentPolicySemanticsId,
};
use world_core::{CanonicalDomain, CanonicalWriter, ContentDigest};

mod activity;
mod appraisal;
mod evidence;
mod intent;

pub use activity::{
    ActivityActionDirective, ActivityAdvancementDecision, ActivityControllerError,
    ActivityInitializationDecision, ActivityInitializationStart, BaselineActivityController,
    ContainmentActionDirective, RelocationActionDirective, activity_state_schema,
};
pub use appraisal::{
    AppraisalEvaluationError, BaselineAppraisalEvaluator, ContainmentAppraisalEvaluation,
};
pub use evidence::{
    BaselineEvidenceAssimilator, EvidenceAssimilationError, EvidenceAssimilationProposal,
};
pub use intent::{BaselineIntentPolicy, IntentDecision, IntentPolicyError};

const IDENTITY_SCHEMA_VERSION: u16 = 1;

pub(super) fn identity(domain: CanonicalDomain) -> [u8; 32] {
    let mut writer = CanonicalWriter::new(domain);
    writer.write_u16(IDENTITY_SCHEMA_VERSION);
    ContentDigest::of_canonical(&writer.finish()).into_bytes()
}

/// Exact installed contract for evidence assimilation.
pub trait EvidenceAssimilator: Send + Sync + 'static {
    /// Returns the exact behavior identity of this implementation.
    fn implementation_id(&self) -> [u8; 32];

    /// Returns the typed behavior identity used by context input.
    fn semantics_id(&self) -> EvidenceAssimilationSemanticsId {
        EvidenceAssimilationSemanticsId::from_bytes(self.implementation_id())
    }

    /// Proposes one actor-local checked evidence transition.
    fn assimilate(
        &self,
        input: &EvidenceAssimilationPayload,
    ) -> Result<EvidenceAssimilationProposal, EvidenceAssimilationError>;
}

/// Exact installed contract for containment appraisal.
pub trait AppraisalEvaluator: Send + Sync + 'static {
    /// Returns the exact behavior identity of this implementation.
    fn implementation_id(&self) -> [u8; 32];

    /// Returns the typed behavior identity used by context input.
    fn semantics_id(&self) -> AppraisalEvaluatorSemanticsId {
        AppraisalEvaluatorSemanticsId::from_bytes(self.implementation_id())
    }

    /// Evaluates one actor-relative containment appraisal.
    fn evaluate(
        &self,
        input: &ContainmentAppraisalPayload,
    ) -> Result<ContainmentAppraisalEvaluation, AppraisalEvaluationError>;
}

/// Exact installed contract for grounded containment-intent review.
pub trait IntentPolicy: Send + Sync + 'static {
    /// Returns the exact behavior identity of this implementation.
    fn implementation_id(&self) -> [u8; 32];

    /// Returns the typed behavior identity used by context input.
    fn semantics_id(&self) -> IntentPolicySemanticsId {
        IntentPolicySemanticsId::from_bytes(self.implementation_id())
    }

    /// Selects only from grounded candidates supplied by the input.
    fn decide(&self, input: &ContainmentIntentPayload)
    -> Result<IntentDecision, IntentPolicyError>;
}

/// Exact installed behavior contract for the closed activity-method set.
pub trait ActivityController: Send + Sync + 'static {
    /// Returns the exact behavior identity of this implementation.
    fn implementation_id(&self) -> [u8; 32];

    /// Returns the typed behavior identity used by context input.
    fn semantics_id(&self) -> ActivityControllerSemanticsId {
        ActivityControllerSemanticsId::from_bytes(self.implementation_id())
    }

    /// Proposes the first concrete step for an accepted intent.
    fn initialize(
        &self,
        input: &ContainmentActivityInitializationPayload,
    ) -> Result<ActivityInitializationDecision, ActivityControllerError>;

    /// Proposes the next concrete step for a persistent activity.
    fn advance(
        &self,
        input: &ActivityAdvancementPayload,
    ) -> Result<ActivityAdvancementDecision, ActivityControllerError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_lifecycle_identities_are_stable_and_port_specific() {
        let identities = [
            BaselineEvidenceAssimilator::new().implementation_id(),
            BaselineAppraisalEvaluator::new().implementation_id(),
            BaselineIntentPolicy::new().implementation_id(),
            BaselineActivityController::new().implementation_id(),
        ];

        assert_eq!(
            identities[0],
            BaselineEvidenceAssimilator::new().implementation_id()
        );
        for (index, identity) in identities.iter().enumerate() {
            assert!(!identities[..index].contains(identity));
        }
        assert_ne!(
            BaselineActivityController::new().implementation_id(),
            activity_state_schema().into_bytes()
        );
    }
}
