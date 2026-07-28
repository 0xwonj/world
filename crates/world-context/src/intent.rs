use core::fmt;

use world_core::{ActorId, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest};
use world_model::{
    ContainmentAppraisal, ContainmentAppraisalFingerprint, DesiredCondition, EvidenceDeliveryId,
    WorldSnapshot,
};

use crate::{
    GroundedIntentCandidateId, GroundedIntentCandidateSetFingerprint, IntentGroundingSemanticsId,
    IntentInputFingerprint, IntentPolicySemanticsId,
};

const CANDIDATE_SCHEMA_VERSION: u16 = 1;
const CANDIDATE_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("grounded-containment-intent-candidate-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("grounded containment intent candidate domain must be valid"),
    };
const CANDIDATE_SET_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("grounded-containment-intent-set-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("grounded containment intent set domain must be valid"),
    };
const INPUT_DOMAIN: CanonicalDomain = match CanonicalDomain::new("containment-intent-input-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("containment intent input domain must be valid"),
};
const GROUNDING_SEMANTICS_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("containment-intent-grounding-semantics-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("containment intent grounding semantics domain must be valid"),
    };

/// Why containment-intent policy input could not be projected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainmentIntentProjectionError {
    /// A canonical identity could not represent one of its fields.
    Canonical(CanonicalError),
}

impl fmt::Display for ContainmentIntentProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canonical(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ContainmentIntentProjectionError {}

impl From<CanonicalError> for ContainmentIntentProjectionError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

/// One actor-safe candidate for restoring a displaced item's container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundedIntentCandidate {
    id: GroundedIntentCandidateId,
    actor: ActorId,
    desired: DesiredCondition,
    supporting_appraisal: ContainmentAppraisalFingerprint,
    supporting_evidence: EvidenceDeliveryId,
}

impl GroundedIntentCandidate {
    /// Returns the stable candidate identity.
    #[must_use]
    pub const fn id(self) -> GroundedIntentCandidateId {
        self.id
    }

    /// Returns the actor who may adopt this desired condition.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the complete actor-relative desired condition.
    #[must_use]
    pub const fn desired(self) -> DesiredCondition {
        self.desired
    }

    /// Returns the material appraisal supporting this candidate.
    #[must_use]
    pub const fn supporting_appraisal(self) -> ContainmentAppraisalFingerprint {
        self.supporting_appraisal
    }

    /// Returns the actor-visible evidence supporting the appraisal.
    #[must_use]
    pub const fn supporting_evidence(self) -> EvidenceDeliveryId {
        self.supporting_evidence
    }
}

/// Complete deterministic intent candidates for one appraisal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundedIntentCandidateSet {
    grounding_semantics: IntentGroundingSemanticsId,
    candidates: Vec<GroundedIntentCandidate>,
    fingerprint: GroundedIntentCandidateSetFingerprint,
}

impl GroundedIntentCandidateSet {
    /// Returns the exact intent-grounding behavior identity.
    #[must_use]
    pub const fn grounding_semantics(&self) -> IntentGroundingSemanticsId {
        self.grounding_semantics
    }

    /// Returns the complete canonical candidate sequence.
    #[must_use]
    pub fn candidates(&self) -> &[GroundedIntentCandidate] {
        &self.candidates
    }

    /// Returns the canonical candidate-set identity.
    #[must_use]
    pub const fn fingerprint(&self) -> GroundedIntentCandidateSetFingerprint {
        self.fingerprint
    }

    /// Returns whether this exact set supplied a candidate identity.
    #[must_use]
    pub fn contains(&self, candidate: GroundedIntentCandidateId) -> bool {
        self.candidates
            .iter()
            .any(|supplied| supplied.id == candidate)
    }
}

/// Complete actor-safe input for deterministic containment-intent review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainmentIntentPayload {
    actor: ActorId,
    appraisal: ContainmentAppraisal,
    candidates: GroundedIntentCandidateSet,
    policy_semantics: IntentPolicySemanticsId,
    fingerprint: IntentInputFingerprint,
}

impl ContainmentIntentPayload {
    /// Returns the actor receiving the intent-policy input.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the exact actor-relative appraisal under review.
    #[must_use]
    pub const fn appraisal(&self) -> ContainmentAppraisal {
        self.appraisal
    }

    /// Returns the complete grounded candidate set.
    #[must_use]
    pub const fn candidates(&self) -> &GroundedIntentCandidateSet {
        &self.candidates
    }

    /// Returns the exact intent-policy behavior identity.
    #[must_use]
    pub const fn policy_semantics(&self) -> IntentPolicySemanticsId {
        self.policy_semantics
    }

    /// Returns the canonical identity of this complete policy input.
    #[must_use]
    pub const fn fingerprint(&self) -> IntentInputFingerprint {
        self.fingerprint
    }
}

/// Exact candidate material retained outside the policy boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedContainmentIntent {
    candidate: GroundedIntentCandidateId,
    actor: ActorId,
    desired: DesiredCondition,
}

impl ResolvedContainmentIntent {
    /// Returns the supplied candidate identity.
    #[must_use]
    pub const fn candidate(self) -> GroundedIntentCandidateId {
        self.candidate
    }

    /// Returns the actor who may own the accepted intent.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the exact desired condition selected by the candidate.
    #[must_use]
    pub const fn desired(self) -> DesiredCondition {
        self.desired
    }
}

/// Coordinator-private resolution for IDs supplied to one intent policy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IntentCandidateResolutionTable {
    candidates: Vec<ResolvedContainmentIntent>,
}

impl IntentCandidateResolutionTable {
    /// Resolves only a candidate supplied by the paired actor-safe payload.
    #[must_use]
    pub fn resolve(
        &self,
        candidate: GroundedIntentCandidateId,
    ) -> Option<&ResolvedContainmentIntent> {
        self.candidates
            .binary_search_by_key(&candidate, |resolved| resolved.candidate)
            .ok()
            .map(|index| &self.candidates[index])
    }

    /// Returns whether no candidate can be resolved.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }
}

/// Actor-safe intent-policy input paired with private candidate resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainmentIntentContextBuild {
    payload: ContainmentIntentPayload,
    resolution: IntentCandidateResolutionTable,
}

impl ContainmentIntentContextBuild {
    /// Returns the actor-safe policy input.
    #[must_use]
    pub const fn payload(&self) -> &ContainmentIntentPayload {
        &self.payload
    }

    /// Returns coordinator-private candidate resolution.
    #[must_use]
    pub const fn resolution(&self) -> &IntentCandidateResolutionTable {
        &self.resolution
    }

    /// Separates policy input from private resolution material.
    #[must_use]
    pub fn into_parts(self) -> (ContainmentIntentPayload, IntentCandidateResolutionTable) {
        (self.payload, self.resolution)
    }
}

/// Pure grounder for the first containment-restoration intent family.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContainmentIntentProjector {
    _private: (),
}

impl ContainmentIntentProjector {
    /// Constructs the containment-intent projector.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Returns the exact behavior identity of this fixed concrete grounder.
    #[must_use]
    pub fn semantics_id(self) -> IntentGroundingSemanticsId {
        let mut writer = CanonicalWriter::new(GROUNDING_SEMANTICS_DOMAIN);
        writer.write_u16(CANDIDATE_SCHEMA_VERSION);
        IntentGroundingSemanticsId(ContentDigest::of_canonical(&writer.finish()).into_bytes())
    }

    /// Builds a complete deterministic candidate set and private resolution.
    ///
    /// A candidate exists only when the appraisal describes displacement and
    /// the actor has no nonterminal accepted intent.
    pub fn build(
        self,
        snapshot: &WorldSnapshot,
        appraisal: ContainmentAppraisal,
        policy_semantics: IntentPolicySemanticsId,
    ) -> Result<ContainmentIntentContextBuild, ContainmentIntentProjectionError> {
        let grounding_semantics = self.semantics_id();
        let actor = appraisal.actor();
        let has_live_intent = snapshot
            .accepted()
            .agency()
            .intents()
            .iter()
            .any(|intent| intent.actor() == actor && !intent.status().is_terminal());
        let displaced = appraisal.believed_current_container() != appraisal.restore_container();

        let (candidates, resolutions) = if displaced && !has_live_intent {
            let desired = DesiredCondition::item_contained_in(
                appraisal.item(),
                appraisal.restore_container(),
            );
            let supporting_appraisal = appraisal.material_fingerprint();
            let candidate = grounded_candidate_id(
                actor,
                desired,
                supporting_appraisal,
                appraisal.supporting_evidence(),
                grounding_semantics,
            )?;
            (
                vec![GroundedIntentCandidate {
                    id: candidate,
                    actor,
                    desired,
                    supporting_appraisal,
                    supporting_evidence: appraisal.supporting_evidence(),
                }],
                vec![ResolvedContainmentIntent {
                    candidate,
                    actor,
                    desired,
                }],
            )
        } else {
            (Vec::new(), Vec::new())
        };

        let candidate_set_fingerprint =
            candidate_set_fingerprint(grounding_semantics, &candidates)?;
        let candidate_set = GroundedIntentCandidateSet {
            grounding_semantics,
            candidates,
            fingerprint: candidate_set_fingerprint,
        };
        let fingerprint =
            intent_input_fingerprint(actor, appraisal, &candidate_set, policy_semantics)?;

        Ok(ContainmentIntentContextBuild {
            payload: ContainmentIntentPayload {
                actor,
                appraisal,
                candidates: candidate_set,
                policy_semantics,
                fingerprint,
            },
            resolution: IntentCandidateResolutionTable {
                candidates: resolutions,
            },
        })
    }
}

fn grounded_candidate_id(
    actor: ActorId,
    desired: DesiredCondition,
    supporting_appraisal: ContainmentAppraisalFingerprint,
    supporting_evidence: EvidenceDeliveryId,
    grounding_semantics: IntentGroundingSemanticsId,
) -> Result<GroundedIntentCandidateId, CanonicalError> {
    let mut writer = CanonicalWriter::new(CANDIDATE_DOMAIN);
    writer.write_u16(CANDIDATE_SCHEMA_VERSION);
    writer.write_bytes(actor.as_bytes())?;
    write_desired(&mut writer, desired)?;
    writer.write_bytes(supporting_appraisal.as_bytes())?;
    writer.write_bytes(supporting_evidence.as_bytes())?;
    writer.write_bytes(grounding_semantics.as_bytes())?;
    Ok(GroundedIntentCandidateId(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

fn candidate_set_fingerprint(
    grounding_semantics: IntentGroundingSemanticsId,
    candidates: &[GroundedIntentCandidate],
) -> Result<GroundedIntentCandidateSetFingerprint, CanonicalError> {
    let mut writer = CanonicalWriter::new(CANDIDATE_SET_DOMAIN);
    write_candidate_set(&mut writer, grounding_semantics, candidates)?;
    Ok(GroundedIntentCandidateSetFingerprint(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

fn intent_input_fingerprint(
    actor: ActorId,
    appraisal: ContainmentAppraisal,
    candidates: &GroundedIntentCandidateSet,
    policy_semantics: IntentPolicySemanticsId,
) -> Result<IntentInputFingerprint, CanonicalError> {
    let mut writer = CanonicalWriter::new(INPUT_DOMAIN);
    writer.write_u16(CANDIDATE_SCHEMA_VERSION);
    writer.write_bytes(actor.as_bytes())?;
    write_appraisal(&mut writer, appraisal)?;
    write_candidate_set(
        &mut writer,
        candidates.grounding_semantics,
        &candidates.candidates,
    )?;
    writer.write_bytes(candidates.fingerprint.as_bytes())?;
    writer.write_bytes(policy_semantics.as_bytes())?;
    Ok(IntentInputFingerprint(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

fn write_candidate_set(
    writer: &mut CanonicalWriter,
    grounding_semantics: IntentGroundingSemanticsId,
    candidates: &[GroundedIntentCandidate],
) -> Result<(), CanonicalError> {
    writer.write_u16(CANDIDATE_SCHEMA_VERSION);
    writer.write_bytes(grounding_semantics.as_bytes())?;
    writer.write_sequence(candidates, |writer, candidate| {
        writer.write_bytes(candidate.id.as_bytes())?;
        writer.write_bytes(candidate.actor.as_bytes())?;
        write_desired(writer, candidate.desired)?;
        writer.write_bytes(candidate.supporting_appraisal.as_bytes())?;
        writer.write_bytes(candidate.supporting_evidence.as_bytes())
    })
}

pub(crate) fn write_appraisal(
    writer: &mut CanonicalWriter,
    appraisal: ContainmentAppraisal,
) -> Result<(), CanonicalError> {
    writer.write_bytes(appraisal.actor().as_bytes())?;
    writer.write_bytes(appraisal.item().as_bytes())?;
    writer.write_bytes(appraisal.believed_current_container().as_bytes())?;
    writer.write_bytes(appraisal.restore_container().as_bytes())?;
    writer.write_bytes(appraisal.supporting_evidence().as_bytes())
}

pub(crate) fn write_desired(
    writer: &mut CanonicalWriter,
    desired: DesiredCondition,
) -> Result<(), CanonicalError> {
    match desired {
        DesiredCondition::ItemContainedIn { item, container } => {
            writer.write_discriminant(0);
            writer.write_bytes(item.as_bytes())?;
            writer.write_bytes(container.as_bytes())
        }
        DesiredCondition::ActorAt { location } => {
            writer.write_discriminant(1);
            writer.write_bytes(location.as_bytes())
        }
    }
}
