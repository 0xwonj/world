use core::fmt;

use world_core::{
    ActorId, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest, EntityId,
};
use world_model::{
    ContainedInBelief, ContainmentAppraisal, EvidenceDeliveryId, EvidenceProvenance,
    EvidenceRecord, WorldSnapshot,
};

use crate::evidence::write_evidence;
use crate::{AppraisalEvaluatorSemanticsId, ContainmentAppraisalInputFingerprint};

const INPUT_SCHEMA_VERSION: u16 = 2;
const INPUT_DOMAIN: CanonicalDomain = match CanonicalDomain::new("containment-appraisal-input-v2") {
    Ok(domain) => domain,
    Err(_) => panic!("containment appraisal input domain must be valid"),
};

/// Why actor-relative containment appraisal input could not be projected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainmentAppraisalProjectionError {
    /// The actor has neither a current belief nor a concrete absence
    /// observation for the item.
    MissingContainmentEvidence {
        /// Actor whose epistemic history was inspected.
        actor: ActorId,
        /// Item whose appraisal was requested.
        item: EntityId,
    },
    /// A belief named evidence absent from the immutable epistemic state.
    MissingSupportingEvidence {
        /// Missing evidence identity.
        evidence: EvidenceDeliveryId,
    },
    /// The retained preceding appraisal belongs to another actor or item.
    PreviousAppraisalMismatch,
    /// A canonical input identity could not represent one of its fields.
    Canonical(CanonicalError),
}

impl fmt::Display for ContainmentAppraisalProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingContainmentEvidence { actor, item } => {
                write!(
                    formatter,
                    "actor {actor:?} has no containment evidence for item {item:?}"
                )
            }
            Self::MissingSupportingEvidence { evidence } => {
                write!(
                    formatter,
                    "containment belief references missing evidence {evidence}"
                )
            }
            Self::PreviousAppraisalMismatch => {
                formatter.write_str("previous appraisal belongs to another actor or item")
            }
            Self::Canonical(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ContainmentAppraisalProjectionError {}

impl From<CanonicalError> for ContainmentAppraisalProjectionError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

/// Current actor-safe containment meaning supplied to appraisal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainmentAppraisalSubject {
    /// The actor retains a positive contained-in belief.
    Present {
        /// Accepted actor-local belief.
        belief: ContainedInBelief,
        /// Latest accepted evidence supporting that belief.
        supporting_evidence: EvidenceRecord,
    },
    /// The actor has retracted an exact contained-in belief after a
    /// non-locating absence observation.
    Absent {
        /// Item whose expected containment was contradicted.
        item: EntityId,
        /// Exact expected container contradicted by the observation.
        expected_container: EntityId,
        /// Accepted non-locating absence evidence.
        supporting_evidence: EvidenceRecord,
    },
}

impl ContainmentAppraisalSubject {
    /// Returns the containment item being appraised.
    #[must_use]
    pub const fn item(&self) -> EntityId {
        match self {
            Self::Present { belief, .. } => belief.item(),
            Self::Absent { item, .. } => *item,
        }
    }

    /// Returns the exact accepted evidence supporting this subject state.
    #[must_use]
    pub const fn supporting_evidence(&self) -> EvidenceRecord {
        match self {
            Self::Present {
                supporting_evidence,
                ..
            }
            | Self::Absent {
                supporting_evidence,
                ..
            } => *supporting_evidence,
        }
    }
}

/// Actor-safe input for one containment-appraisal evaluation.
///
/// Both semantic records come from the actor's accepted epistemic partition.
/// The payload cannot represent hidden domain truth or runtime coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainmentAppraisalPayload {
    actor: ActorId,
    subject: ContainmentAppraisalSubject,
    previous: Option<ContainmentAppraisal>,
    evaluator_semantics: AppraisalEvaluatorSemanticsId,
    fingerprint: ContainmentAppraisalInputFingerprint,
}

impl ContainmentAppraisalPayload {
    /// Returns the actor receiving the appraisal input.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the actor's concrete present-or-absent containment subject.
    #[must_use]
    pub const fn subject(&self) -> &ContainmentAppraisalSubject {
        &self.subject
    }

    /// Returns the preceding retained appraisal, when one exists.
    #[must_use]
    pub const fn previous(&self) -> Option<ContainmentAppraisal> {
        self.previous
    }

    /// Returns the exact appraisal-evaluator behavior identity.
    #[must_use]
    pub const fn evaluator_semantics(&self) -> AppraisalEvaluatorSemanticsId {
        self.evaluator_semantics
    }

    /// Returns the canonical identity of this complete actor-safe input.
    #[must_use]
    pub const fn fingerprint(&self) -> ContainmentAppraisalInputFingerprint {
        self.fingerprint
    }
}

/// Pure projector for one actor's current containment appraisal input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContainmentAppraisalProjector {
    _private: (),
}

impl ContainmentAppraisalProjector {
    /// Constructs the containment appraisal projector.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Projects only accepted actor-relative belief and evidence.
    pub fn build(
        self,
        snapshot: &WorldSnapshot,
        actor: ActorId,
        item: EntityId,
        previous: Option<ContainmentAppraisal>,
        evaluator_semantics: AppraisalEvaluatorSemanticsId,
    ) -> Result<ContainmentAppraisalPayload, ContainmentAppraisalProjectionError> {
        if previous.is_some_and(|appraisal| appraisal.actor() != actor || appraisal.item() != item)
        {
            return Err(ContainmentAppraisalProjectionError::PreviousAppraisalMismatch);
        }
        let epistemic = snapshot.accepted().epistemic();
        let subject = match epistemic.contained_in(actor, item) {
            Some(belief) => {
                let supporting_evidence = belief
                    .support()
                    .iter()
                    .map(|evidence| {
                        epistemic.evidence_record(*evidence).copied().ok_or(
                            ContainmentAppraisalProjectionError::MissingSupportingEvidence {
                                evidence: *evidence,
                            },
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .max_by_key(|record| record.generation())
                    .unwrap_or_else(|| {
                        unreachable!("checked contained-in beliefs have nonempty support")
                    });
                ContainmentAppraisalSubject::Present {
                    belief: belief.clone(),
                    supporting_evidence,
                }
            }
            None => {
                let supporting_evidence = epistemic
                    .evidence()
                    .iter()
                    .copied()
                    .filter(|evidence| {
                        evidence.observer() == actor
                            && matches!(
                                evidence.provenance(),
                                EvidenceProvenance::DirectItemAbsent(observation)
                                    if observation.item() == item
                            )
                    })
                    .max_by_key(|evidence| evidence.generation())
                    .ok_or(
                        ContainmentAppraisalProjectionError::MissingContainmentEvidence {
                            actor,
                            item,
                        },
                    )?;
                let EvidenceProvenance::DirectItemAbsent(observation) =
                    supporting_evidence.provenance()
                else {
                    unreachable!("absence evidence was selected above")
                };
                ContainmentAppraisalSubject::Absent {
                    item,
                    expected_container: observation.expected_container(),
                    supporting_evidence,
                }
            }
        };
        let fingerprint = containment_appraisal_input_fingerprint(
            actor,
            &subject,
            previous,
            evaluator_semantics,
        )?;

        Ok(ContainmentAppraisalPayload {
            actor,
            subject,
            previous,
            evaluator_semantics,
            fingerprint,
        })
    }
}

fn containment_appraisal_input_fingerprint(
    actor: ActorId,
    subject: &ContainmentAppraisalSubject,
    previous: Option<ContainmentAppraisal>,
    evaluator_semantics: AppraisalEvaluatorSemanticsId,
) -> Result<ContainmentAppraisalInputFingerprint, CanonicalError> {
    let mut writer = CanonicalWriter::new(INPUT_DOMAIN);
    writer.write_u16(INPUT_SCHEMA_VERSION);
    writer.write_bytes(actor.as_bytes())?;
    write_subject(&mut writer, subject)?;
    write_optional_appraisal(&mut writer, previous)?;
    writer.write_bytes(evaluator_semantics.as_bytes())?;
    Ok(ContainmentAppraisalInputFingerprint(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

fn write_subject(
    writer: &mut CanonicalWriter,
    subject: &ContainmentAppraisalSubject,
) -> Result<(), CanonicalError> {
    match subject {
        ContainmentAppraisalSubject::Present {
            belief,
            supporting_evidence,
        } => {
            writer.write_discriminant(0);
            write_belief(writer, belief)?;
            write_evidence(writer, supporting_evidence)
        }
        ContainmentAppraisalSubject::Absent {
            item,
            expected_container,
            supporting_evidence,
        } => {
            writer.write_discriminant(1);
            writer.write_bytes(item.as_bytes())?;
            writer.write_bytes(expected_container.as_bytes())?;
            write_evidence(writer, supporting_evidence)
        }
    }
}

pub(crate) fn write_belief(
    writer: &mut CanonicalWriter,
    belief: &ContainedInBelief,
) -> Result<(), CanonicalError> {
    writer.write_bytes(belief.actor().as_bytes())?;
    writer.write_bytes(belief.item().as_bytes())?;
    writer.write_bytes(belief.container().as_bytes())?;
    writer.write_sequence(belief.support(), |writer, evidence| {
        writer.write_bytes(evidence.as_bytes())
    })
}

fn write_optional_appraisal(
    writer: &mut CanonicalWriter,
    appraisal: Option<ContainmentAppraisal>,
) -> Result<(), CanonicalError> {
    match appraisal {
        Some(appraisal) => {
            writer.write_discriminant(1);
            writer.write_bytes(appraisal.actor().as_bytes())?;
            writer.write_bytes(appraisal.item().as_bytes())?;
            writer.write_bytes(appraisal.believed_current_container().as_bytes())?;
            writer.write_bytes(appraisal.restore_container().as_bytes())?;
            writer.write_bytes(appraisal.supporting_evidence().as_bytes())
        }
        None => {
            writer.write_discriminant(0);
            Ok(())
        }
    }
}
