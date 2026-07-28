//! Pure actor-relative projections and grounded action candidates.
//!
//! This crate reads immutable model snapshots and checked definitions. It
//! exposes actor-safe policy input while retaining the exact references
//! required by trusted engine lowering. It owns no mutation, scheduling, or
//! runtime authority.

mod action;
mod activity;
mod appraisal;
mod evidence;
mod identity;
mod intent;

pub use action::{
    ActionArtifactCodecError, ActionContextBuild, ActionContextPayload, ActionExecutionWitness,
    ActionProjectionWitness, ActionReadWitness, ActorSafeActionInteraction, ActorSafeBinding,
    ActorSafeBindingValue, ActorSafeContainmentInteraction, ActorSafeRelocationInteraction,
    ActorSafeRelocationInteractionEntry, CandidateCoverage, CandidateResolutionTable,
    ContainmentBeliefObservation, ContainmentCandidateExecutionWitness, ContainmentPolicyWitness,
    ContainmentProjectionError, ContainmentTransferActionDefinitions,
    ContainmentTransferActionDefinitionsError, ContainmentTransferProjector,
    GroundedActionCandidate, GroundedActionCandidateSet, GroundedActionInteraction,
    RelocationActionDefinitions, RelocationActionDefinitionsError, RelocationActionVerb,
    RelocationProjectionError, RelocationProjector, ResolvedActionSelection,
    ResolvedContainmentTransfer, ResolvedRelocationAction, action_context_payload_schema,
    action_execution_witness_schema, action_projection_witness_schema, action_read_witness_schema,
    candidate_resolution_table_schema, decode_action_context_payload,
    decode_action_execution_witness, decode_action_projection_witness, decode_action_read_witness,
    decode_candidate_resolution_table, encode_action_context_payload,
    encode_action_execution_witness, encode_action_projection_witness, encode_action_read_witness,
    encode_candidate_resolution_table,
};
pub use activity::{
    ActivityAdvancementPayload, ActivityEvaluationCause, ActivityProjectionError,
    ActivityProjector, ContainmentActivityAdvancementPayload,
    ContainmentActivityInitializationPayload, ContainmentActivityProjector,
    TravelActivityAdvancementPayload,
};
pub use appraisal::{
    ContainmentAppraisalPayload, ContainmentAppraisalProjectionError,
    ContainmentAppraisalProjector, ContainmentAppraisalSubject,
};
pub use evidence::{EvidenceAssimilationPayload, EvidenceAssimilationPayloadError};
pub use identity::{
    ActionContextPayloadSchemaId, ActionExecutionWitnessSchemaId, ActionInputFingerprint,
    ActionPolicySemanticsId, ActionProjectionWitnessSchemaId, ActionReadWitnessSchemaId,
    ActivityAdvancementInputFingerprint, ActivityControllerSemanticsId,
    ActivityInitializationInputFingerprint, ActorSafeObjectRef, AppraisalEvaluatorSemanticsId,
    CandidateResolutionTableSchemaId, ContainmentAppraisalInputFingerprint,
    EvidenceAssimilationInputFingerprint, EvidenceAssimilationSemanticsId,
    GroundedActionCandidateId, GroundedCandidateSetFingerprint, GroundedIntentCandidateId,
    GroundedIntentCandidateSetFingerprint, GroundingSemanticsId, IntentGroundingSemanticsId,
    IntentInputFingerprint, IntentPolicySemanticsId,
};
pub use intent::{
    ContainmentIntentContextBuild, ContainmentIntentPayload, ContainmentIntentProjectionError,
    ContainmentIntentProjector, GroundedIntentCandidate, GroundedIntentCandidateSet,
    IntentCandidateResolutionTable, ResolvedContainmentIntent,
};
