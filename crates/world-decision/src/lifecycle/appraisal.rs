use core::fmt;

use world_context::{
    AppraisalEvaluatorSemanticsId, ContainmentAppraisalInputFingerprint,
    ContainmentAppraisalPayload, ContainmentAppraisalSubject,
};
use world_core::CanonicalDomain;
use world_model::{ContainmentAppraisal, EvidenceDeliveryId, EvidenceProvenance};

use super::{AppraisalEvaluator, identity};

const APPRAISAL_EVALUATOR_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("baseline-appraisal-evaluator-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("baseline appraisal-evaluator domain must be valid"),
    };

/// Why the baseline appraisal evaluator refused an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppraisalEvaluationError {
    /// The input was constructed for a different evaluator behavior.
    SemanticsMismatch {
        /// Behavior identity required by this implementation.
        expected: AppraisalEvaluatorSemanticsId,
        /// Behavior identity committed by the input.
        actual: AppraisalEvaluatorSemanticsId,
    },
    /// A containment appraisal was paired with another evidence family.
    NonContainmentEvidence,
}

impl fmt::Display for AppraisalEvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticsMismatch { expected, actual } => write!(
                formatter,
                "appraisal input semantics {actual} do not match baseline semantics {expected}"
            ),
            Self::NonContainmentEvidence => {
                formatter.write_str("containment appraisal input has non-containment evidence")
            }
        }
    }
}

impl std::error::Error for AppraisalEvaluationError {}

/// Closed derived outcome of one containment-appraisal evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainmentAppraisalEvaluation {
    /// Retain one current derived appraisal.
    Present {
        /// Exact actor-safe input that produced the result.
        input: ContainmentAppraisalInputFingerprint,
        /// Current derived appraisal.
        appraisal: ContainmentAppraisal,
        /// Whether the appraisal's semantic material changed.
        material_changed: bool,
    },
    /// Retract one exact preceding appraisal after a non-locating absence
    /// observation.
    Retract {
        /// Exact actor-safe input that produced the result.
        input: ContainmentAppraisalInputFingerprint,
        /// Exact retained value being retracted.
        before: ContainmentAppraisal,
        /// Accepted absence evidence supporting the retraction.
        supporting_evidence: EvidenceDeliveryId,
    },
    /// Complete evaluation when absence has no preceding appraisal to retract.
    NoChange {
        /// Exact actor-safe input that produced the result.
        input: ContainmentAppraisalInputFingerprint,
    },
}

impl ContainmentAppraisalEvaluation {
    /// Returns the exact policy input that produced this appraisal.
    #[must_use]
    pub const fn input_fingerprint(self) -> ContainmentAppraisalInputFingerprint {
        match self {
            Self::Present { input, .. }
            | Self::Retract { input, .. }
            | Self::NoChange { input } => input,
        }
    }
}

/// Deterministic baseline appraisal for directly observed displacement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BaselineAppraisalEvaluator {
    _private: (),
}

impl BaselineAppraisalEvaluator {
    /// Constructs the baseline appraisal evaluator.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Returns the typed behavior identity used by compatible context input.
    #[must_use]
    pub fn semantics_id(self) -> AppraisalEvaluatorSemanticsId {
        AppraisalEvaluatorSemanticsId::from_bytes(identity(APPRAISAL_EVALUATOR_DOMAIN))
    }

    /// Returns the catalog-facing implementation identity.
    #[must_use]
    pub fn implementation_id(self) -> [u8; 32] {
        self.semantics_id().into_bytes()
    }

    /// Derives containment appraisal solely from supplied belief and evidence.
    pub fn evaluate(
        self,
        input: &ContainmentAppraisalPayload,
    ) -> Result<ContainmentAppraisalEvaluation, AppraisalEvaluationError> {
        let expected = self.semantics_id();
        let actual = input.evaluator_semantics();
        if actual != expected {
            return Err(AppraisalEvaluationError::SemanticsMismatch { expected, actual });
        }

        match input.subject() {
            ContainmentAppraisalSubject::Present {
                belief,
                supporting_evidence,
            } => {
                let EvidenceProvenance::DirectItemTransfer(event) =
                    supporting_evidence.provenance()
                else {
                    return Err(AppraisalEvaluationError::NonContainmentEvidence);
                };
                let appraisal = ContainmentAppraisal::new(
                    input.actor(),
                    belief.item(),
                    belief.container(),
                    event.source(),
                    supporting_evidence.id(),
                );
                let material_changed = input
                    .previous()
                    .map(ContainmentAppraisal::material_fingerprint)
                    != Some(appraisal.material_fingerprint());

                Ok(ContainmentAppraisalEvaluation::Present {
                    input: input.fingerprint(),
                    appraisal,
                    material_changed,
                })
            }
            ContainmentAppraisalSubject::Absent {
                item,
                expected_container,
                supporting_evidence,
            } => {
                let EvidenceProvenance::DirectItemAbsent(observation) =
                    supporting_evidence.provenance()
                else {
                    return Err(AppraisalEvaluationError::NonContainmentEvidence);
                };
                if observation.item() != *item
                    || observation.expected_container() != *expected_container
                {
                    return Err(AppraisalEvaluationError::NonContainmentEvidence);
                }
                match input.previous() {
                    Some(before) if before.believed_current_container() == *expected_container => {
                        Ok(ContainmentAppraisalEvaluation::Retract {
                            input: input.fingerprint(),
                            before,
                            supporting_evidence: supporting_evidence.id(),
                        })
                    }
                    Some(_) | None => Ok(ContainmentAppraisalEvaluation::NoChange {
                        input: input.fingerprint(),
                    }),
                }
            }
        }
    }
}

impl AppraisalEvaluator for BaselineAppraisalEvaluator {
    fn implementation_id(&self) -> [u8; 32] {
        (*self).implementation_id()
    }

    fn evaluate(
        &self,
        input: &ContainmentAppraisalPayload,
    ) -> Result<ContainmentAppraisalEvaluation, AppraisalEvaluationError> {
        (*self).evaluate(input)
    }
}
