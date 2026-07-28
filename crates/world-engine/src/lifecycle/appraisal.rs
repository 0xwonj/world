use core::fmt;

use world_context::{
    AppraisalEvaluatorSemanticsId, ContainmentAppraisalInputFingerprint,
    ContainmentAppraisalPayload, ContainmentAppraisalSubject,
};
use world_decision::{
    AppraisalEvaluationError, AppraisalEvaluator, ContainmentAppraisalEvaluation,
};
use world_model::{ContainmentAppraisal, EvidenceDeliveryId, EvidenceProvenance};

/// Why appraisal evaluation could not produce a trusted derived result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AppraisalCoordinationError {
    SemanticsMismatch {
        expected: AppraisalEvaluatorSemanticsId,
        actual: AppraisalEvaluatorSemanticsId,
    },
    Evaluator(AppraisalEvaluationError),
    InputFingerprintMismatch {
        expected: ContainmentAppraisalInputFingerprint,
        actual: ContainmentAppraisalInputFingerprint,
    },
    AppraisalMismatch,
    MaterialChangeMismatch {
        expected: bool,
        actual: bool,
    },
}

impl fmt::Display for AppraisalCoordinationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticsMismatch { expected, actual } => write!(
                formatter,
                "appraisal input semantics {actual} do not match resolved evaluator semantics {expected}"
            ),
            Self::Evaluator(error) => error.fmt(formatter),
            Self::InputFingerprintMismatch { expected, actual } => write!(
                formatter,
                "appraisal result input {actual} does not match prepared input {expected}"
            ),
            Self::AppraisalMismatch => {
                formatter.write_str("appraisal result changed the prepared actor-relative meaning")
            }
            Self::MaterialChangeMismatch { expected, actual } => write!(
                formatter,
                "appraisal material-change flag {actual} does not match derived value {expected}"
            ),
        }
    }
}

impl std::error::Error for AppraisalCoordinationError {}

/// Engine-private checked result of one appraisal evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CoordinatedAppraisal {
    Present {
        input: ContainmentAppraisalInputFingerprint,
        appraisal: ContainmentAppraisal,
        material_changed: bool,
    },
    Retract {
        input: ContainmentAppraisalInputFingerprint,
        before: ContainmentAppraisal,
        supporting_evidence: EvidenceDeliveryId,
    },
    NoChange {
        input: ContainmentAppraisalInputFingerprint,
    },
}

/// Coordinates one exact appraisal payload against an already resolved port.
pub(crate) struct AppraisalCoordinator;

impl AppraisalCoordinator {
    pub(crate) fn coordinate(
        input: &ContainmentAppraisalPayload,
        evaluator: &dyn AppraisalEvaluator,
    ) -> Result<CoordinatedAppraisal, AppraisalCoordinationError> {
        let expected_semantics = evaluator.semantics_id();
        if input.evaluator_semantics() != expected_semantics {
            return Err(AppraisalCoordinationError::SemanticsMismatch {
                expected: expected_semantics,
                actual: input.evaluator_semantics(),
            });
        }
        let evaluation = evaluator
            .evaluate(input)
            .map_err(AppraisalCoordinationError::Evaluator)?;
        validate_evaluation(input, evaluation)
    }
}

fn validate_evaluation(
    input: &ContainmentAppraisalPayload,
    evaluation: ContainmentAppraisalEvaluation,
) -> Result<CoordinatedAppraisal, AppraisalCoordinationError> {
    if evaluation.input_fingerprint() != input.fingerprint() {
        return Err(AppraisalCoordinationError::InputFingerprintMismatch {
            expected: input.fingerprint(),
            actual: evaluation.input_fingerprint(),
        });
    }

    match (input.subject(), input.previous(), evaluation) {
        (
            ContainmentAppraisalSubject::Present {
                belief,
                supporting_evidence,
            },
            previous,
            ContainmentAppraisalEvaluation::Present {
                appraisal,
                material_changed,
                ..
            },
        ) => {
            let EvidenceProvenance::DirectItemTransfer(event) = supporting_evidence.provenance()
            else {
                return Err(AppraisalCoordinationError::AppraisalMismatch);
            };
            let expected_appraisal = ContainmentAppraisal::new(
                input.actor(),
                belief.item(),
                belief.container(),
                event.source(),
                supporting_evidence.id(),
            );
            if appraisal != expected_appraisal {
                return Err(AppraisalCoordinationError::AppraisalMismatch);
            }
            let expected_changed = previous.map(ContainmentAppraisal::material_fingerprint)
                != Some(expected_appraisal.material_fingerprint());
            if material_changed != expected_changed {
                return Err(AppraisalCoordinationError::MaterialChangeMismatch {
                    expected: expected_changed,
                    actual: material_changed,
                });
            }
            Ok(CoordinatedAppraisal::Present {
                input: input.fingerprint(),
                appraisal,
                material_changed,
            })
        }
        (
            ContainmentAppraisalSubject::Absent {
                item,
                expected_container,
                supporting_evidence,
            },
            Some(previous),
            ContainmentAppraisalEvaluation::Retract {
                before,
                supporting_evidence: actual_evidence,
                ..
            },
        ) if previous.believed_current_container() == *expected_container => {
            let EvidenceProvenance::DirectItemAbsent(observation) =
                supporting_evidence.provenance()
            else {
                return Err(AppraisalCoordinationError::AppraisalMismatch);
            };
            if observation.item() != *item
                || observation.expected_container() != *expected_container
                || before != previous
                || actual_evidence != supporting_evidence.id()
            {
                return Err(AppraisalCoordinationError::AppraisalMismatch);
            }
            Ok(CoordinatedAppraisal::Retract {
                input: input.fingerprint(),
                before,
                supporting_evidence: actual_evidence,
            })
        }
        (
            ContainmentAppraisalSubject::Absent { .. },
            None,
            ContainmentAppraisalEvaluation::NoChange { .. },
        ) => Ok(CoordinatedAppraisal::NoChange {
            input: input.fingerprint(),
        }),
        (
            ContainmentAppraisalSubject::Absent {
                expected_container, ..
            },
            Some(previous),
            ContainmentAppraisalEvaluation::NoChange { .. },
        ) if previous.believed_current_container() != *expected_container => {
            Ok(CoordinatedAppraisal::NoChange {
                input: input.fingerprint(),
            })
        }
        _ => Err(AppraisalCoordinationError::AppraisalMismatch),
    }
}
