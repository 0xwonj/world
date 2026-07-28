use core::{convert::Infallible, fmt};
use std::collections::BTreeSet;

use minicbor::{Decoder, Encoder};
use world_core::{
    ActorId, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest, EntityId,
};
use world_defs::{BindingName, DefinitionKey, LocalDefinitionName, PackKey};
use world_model::{ActionOpportunityId, RelocationInteraction, RelocationRouteId};

use crate::identity::{
    ActionContextPayloadSchemaId, ActionExecutionWitnessSchemaId, ActionProjectionWitnessSchemaId,
    ActionReadWitnessSchemaId, CandidateResolutionTableSchemaId,
};

use super::*;

const ACTION_CONTEXT_PAYLOAD_SCHEMA_VERSION: u16 = 1;
const CANDIDATE_RESOLUTION_TABLE_SCHEMA_VERSION: u16 = 1;
const ACTION_PROJECTION_WITNESS_SCHEMA_VERSION: u16 = 1;
const ACTION_EXECUTION_WITNESS_SCHEMA_VERSION: u16 = 1;
const ACTION_READ_WITNESS_SCHEMA_VERSION: u16 = 1;

const ACTION_CONTEXT_PAYLOAD_SCHEMA_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-context-payload-schema-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action context payload schema domain must be valid"),
    };
const CANDIDATE_RESOLUTION_TABLE_SCHEMA_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("candidate-resolution-table-schema-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("candidate resolution table schema domain must be valid"),
    };
const ACTION_PROJECTION_WITNESS_SCHEMA_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-projection-witness-schema-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action projection witness schema domain must be valid"),
    };
const ACTION_EXECUTION_WITNESS_SCHEMA_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-execution-witness-schema-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action execution witness schema domain must be valid"),
    };
const ACTION_READ_WITNESS_SCHEMA_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-read-witness-schema-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action read witness schema domain must be valid"),
    };

const PAYLOAD_ARITY: u64 = 7;
const CANDIDATE_SET_ARITY: u64 = 6;
const CANDIDATE_ARITY: u64 = 5;
const DEFINITION_KEY_ARITY: u64 = 2;
const BINDING_ARITY: u64 = 2;
const BINDING_VALUE_ARITY: u64 = 2;
const CONTAINMENT_INTERACTION_ARITY: u64 = 4;
const RELOCATION_INTERACTION_ARITY: u64 = 2;
const RELOCATION_ENTRY_ARITY: u64 = 3;
const CONTAINMENT_CANDIDATE_INTERACTION_ARITY: u64 = 1;
const RELOCATION_CANDIDATE_INTERACTION_ARITY: u64 = 2;
const RESOLUTION_TABLE_ARITY: u64 = 3;
const PRIVATE_REFERENCE_ARITY: u64 = 2;
const PRIVATE_CONTAINMENT_RESOLUTION_ARITY: u64 = 7;
const PRIVATE_RELOCATION_RESOLUTION_ARITY: u64 = 5;
const EXACT_RELOCATION_INTERACTION_ARITY: u64 = 2;
const CONTAINMENT_WITNESS_ARITY: u64 = 4;
const RELOCATION_WITNESS_ARITY: u64 = 2;
const BELIEF_OBSERVATION_ARITY: u64 = 2;
const ABSENT_BELIEF_ARITY: u64 = 1;
const PRESENT_BELIEF_ARITY: u64 = 2;
const CONTAINMENT_EXECUTION_WITNESS_ARITY: u64 = 3;
const RELOCATION_EXECUTION_WITNESS_ARITY: u64 = 2;
const CANDIDATE_EXECUTION_OBSERVATION_ARITY: u64 = 6;
const ABSENT_OPTION_ARITY: u64 = 1;
const PRESENT_OPTION_ARITY: u64 = 2;
const ACTION_READ_WITNESS_ARITY: u64 = 3;

const CONTAINMENT_TAG: u16 = 0;
const RELOCATION_TAG: u16 = 1;
const ACTOR_BINDING_TAG: u16 = 0;
const OBJECT_BINDING_TAG: u16 = 1;
const COMPLETE_COVERAGE_TAG: u16 = 0;
const BUDGET_LIMITED_COVERAGE_TAG: u16 = 1;
const START_RELOCATION_TAG: u16 = 0;
const PAUSE_RELOCATION_TAG: u16 = 1;
const RESUME_RELOCATION_TAG: u16 = 2;
const ABSENT_BELIEF_TAG: u16 = 0;
const PRESENT_BELIEF_TAG: u16 = 1;

/// Returns the fixed schema identity of actor-safe action-context artifacts.
#[must_use]
pub fn action_context_payload_schema() -> ActionContextPayloadSchemaId {
    ActionContextPayloadSchemaId::from_bytes(schema_identity(
        ACTION_CONTEXT_PAYLOAD_SCHEMA_DOMAIN,
        ACTION_CONTEXT_PAYLOAD_SCHEMA_VERSION,
    ))
}

/// Returns the fixed schema identity of private candidate-resolution artifacts.
#[must_use]
pub fn candidate_resolution_table_schema() -> CandidateResolutionTableSchemaId {
    CandidateResolutionTableSchemaId::from_bytes(schema_identity(
        CANDIDATE_RESOLUTION_TABLE_SCHEMA_DOMAIN,
        CANDIDATE_RESOLUTION_TABLE_SCHEMA_VERSION,
    ))
}

/// Returns the fixed schema identity of action-projection witness artifacts.
#[must_use]
pub fn action_projection_witness_schema() -> ActionProjectionWitnessSchemaId {
    ActionProjectionWitnessSchemaId::from_bytes(schema_identity(
        ACTION_PROJECTION_WITNESS_SCHEMA_DOMAIN,
        ACTION_PROJECTION_WITNESS_SCHEMA_VERSION,
    ))
}

/// Returns the fixed schema identity of private execution-validation witnesses.
#[must_use]
pub fn action_execution_witness_schema() -> ActionExecutionWitnessSchemaId {
    ActionExecutionWitnessSchemaId::from_bytes(schema_identity(
        ACTION_EXECUTION_WITNESS_SCHEMA_DOMAIN,
        ACTION_EXECUTION_WITNESS_SCHEMA_VERSION,
    ))
}

/// Returns the fixed schema identity of combined private action-read records.
#[must_use]
pub fn action_read_witness_schema() -> ActionReadWitnessSchemaId {
    ActionReadWitnessSchemaId::from_bytes(schema_identity(
        ACTION_READ_WITNESS_SCHEMA_DOMAIN,
        ACTION_READ_WITNESS_SCHEMA_VERSION,
    ))
}

fn schema_identity(domain: CanonicalDomain, version: u16) -> [u8; 32] {
    let mut writer = CanonicalWriter::new(domain);
    writer.write_u16(version);
    ContentDigest::of_canonical(&writer.finish()).into_bytes()
}

/// Encodes one complete checked actor-safe action payload.
#[must_use]
pub fn encode_action_context_payload(payload: &ActionContextPayload) -> Vec<u8> {
    let mut encoder = Encoder::new(Vec::new());
    encode_array(&mut encoder, "action context payload", PAYLOAD_ARITY);
    encode_u16(
        &mut encoder,
        "action context payload schema",
        ACTION_CONTEXT_PAYLOAD_SCHEMA_VERSION,
    );
    encode_bytes(&mut encoder, "actor", payload.actor.as_bytes());
    encode_bytes(
        &mut encoder,
        "action opportunity",
        payload.opportunity.as_bytes(),
    );
    encode_actor_safe_interaction(&mut encoder, &payload.interaction);
    encode_candidate_set(&mut encoder, &payload.candidates);
    encode_bytes(
        &mut encoder,
        "policy semantics",
        payload.policy_semantics.as_bytes(),
    );
    encode_bytes(
        &mut encoder,
        "action input fingerprint",
        payload.input_fingerprint.as_bytes(),
    );
    encoder.into_writer()
}

/// Decodes and revalidates one complete canonical actor-safe action payload.
pub fn decode_action_context_payload(
    bytes: &[u8],
) -> Result<ActionContextPayload, ActionArtifactCodecError> {
    let mut decoder = Decoder::new(bytes);
    expect_array(&mut decoder, "action context payload", PAYLOAD_ARITY)?;
    decode_schema(
        &mut decoder,
        "action context payload",
        ACTION_CONTEXT_PAYLOAD_SCHEMA_VERSION,
    )?;
    let actor = ActorId::from_bytes(decode_fixed(&mut decoder, "actor")?);
    let opportunity =
        ActionOpportunityId::from_bytes(decode_fixed(&mut decoder, "action opportunity")?);
    let interaction = decode_actor_safe_interaction(&mut decoder)?;
    let candidates = decode_candidate_set(&mut decoder)?;
    let policy_semantics =
        ActionPolicySemanticsId::from_bytes(decode_fixed(&mut decoder, "policy semantics")?);
    let input_fingerprint =
        ActionInputFingerprint::from_bytes(decode_fixed(&mut decoder, "action input fingerprint")?);
    finish(&decoder, bytes)?;

    let payload = ActionContextPayload {
        actor,
        opportunity,
        interaction,
        candidates,
        policy_semantics,
        input_fingerprint,
    };
    validate_payload(&payload)?;
    require_canonical(bytes, &encode_action_context_payload(&payload))?;
    Ok(payload)
}

/// Encodes one complete private candidate-resolution table.
#[must_use]
pub fn encode_candidate_resolution_table(table: &CandidateResolutionTable) -> Vec<u8> {
    let mut encoder = Encoder::new(Vec::new());
    encode_array(
        &mut encoder,
        "candidate resolution table",
        RESOLUTION_TABLE_ARITY,
    );
    encode_u16(
        &mut encoder,
        "candidate resolution table schema",
        CANDIDATE_RESOLUTION_TABLE_SCHEMA_VERSION,
    );
    encode_sequence(
        &mut encoder,
        "private object references",
        table.references.len(),
    );
    for reference in &table.references {
        encode_array(
            &mut encoder,
            "private object reference",
            PRIVATE_REFERENCE_ARITY,
        );
        encode_bytes(
            &mut encoder,
            "actor-safe object reference",
            reference.actor_safe.as_bytes(),
        );
        encode_bytes(
            &mut encoder,
            "exact object identity",
            reference.exact.as_bytes(),
        );
    }
    encode_sequence(
        &mut encoder,
        "private candidate resolutions",
        table.candidates.len(),
    );
    for resolution in &table.candidates {
        encode_private_resolution(&mut encoder, resolution);
    }
    encoder.into_writer()
}

/// Decodes and revalidates one canonical private candidate-resolution table.
pub fn decode_candidate_resolution_table(
    bytes: &[u8],
) -> Result<CandidateResolutionTable, ActionArtifactCodecError> {
    let mut decoder = Decoder::new(bytes);
    expect_array(
        &mut decoder,
        "candidate resolution table",
        RESOLUTION_TABLE_ARITY,
    )?;
    decode_schema(
        &mut decoder,
        "candidate resolution table",
        CANDIDATE_RESOLUTION_TABLE_SCHEMA_VERSION,
    )?;
    let reference_len = decode_sequence_len(&mut decoder, "private object references")?;
    let mut references = Vec::new();
    for _ in 0..reference_len {
        expect_array(
            &mut decoder,
            "private object reference",
            PRIVATE_REFERENCE_ARITY,
        )?;
        references.push(PrivateObjectResolution {
            actor_safe: ActorSafeObjectRef::from_bytes(decode_fixed(
                &mut decoder,
                "actor-safe object reference",
            )?),
            exact: EntityId::from_bytes(decode_fixed(&mut decoder, "exact object identity")?),
        });
    }
    let candidate_len = decode_sequence_len(&mut decoder, "private candidate resolutions")?;
    let mut candidates = Vec::new();
    for _ in 0..candidate_len {
        candidates.push(decode_private_resolution(&mut decoder)?);
    }
    finish(&decoder, bytes)?;
    let table = CandidateResolutionTable {
        references,
        candidates,
    };
    validate_resolution_table(&table)?;
    require_canonical(bytes, &encode_candidate_resolution_table(&table))?;
    Ok(table)
}

/// Encodes one complete narrow action-projection witness.
#[must_use]
pub fn encode_action_projection_witness(witness: &ActionProjectionWitness) -> Vec<u8> {
    let mut encoder = Encoder::new(Vec::new());
    match witness {
        ActionProjectionWitness::Containment(witness) => {
            encode_array(
                &mut encoder,
                "containment projection witness",
                CONTAINMENT_WITNESS_ARITY,
            );
            encode_u16(
                &mut encoder,
                "action projection witness schema",
                ACTION_PROJECTION_WITNESS_SCHEMA_VERSION,
            );
            encode_u16(&mut encoder, "projection witness kind", CONTAINMENT_TAG);
            encode_bytes(&mut encoder, "witness actor", witness.actor.as_bytes());
            encode_sequence(
                &mut encoder,
                "containment belief observations",
                witness.observations.len(),
            );
            for observation in &witness.observations {
                encode_belief_observation(&mut encoder, observation);
            }
        }
        ActionProjectionWitness::RelocationNoRead => {
            encode_array(
                &mut encoder,
                "relocation projection witness",
                RELOCATION_WITNESS_ARITY,
            );
            encode_u16(
                &mut encoder,
                "action projection witness schema",
                ACTION_PROJECTION_WITNESS_SCHEMA_VERSION,
            );
            encode_u16(&mut encoder, "projection witness kind", RELOCATION_TAG);
        }
    }
    encoder.into_writer()
}

/// Decodes and revalidates one canonical narrow action-projection witness.
pub fn decode_action_projection_witness(
    bytes: &[u8],
) -> Result<ActionProjectionWitness, ActionArtifactCodecError> {
    let mut decoder = Decoder::new(bytes);
    let position = decoder.position();
    let arity = decoder
        .array()
        .map_err(|error| unexpected_cbor(error, position, "action projection witness"))?
        .ok_or(ActionArtifactCodecError::IndefiniteArray {
            position,
            context: "action projection witness",
        })?;
    decode_schema(
        &mut decoder,
        "action projection witness",
        ACTION_PROJECTION_WITNESS_SCHEMA_VERSION,
    )?;
    let tag_position = decoder.position();
    let tag = decode_u16(&mut decoder, "projection witness kind")?;
    let witness = match tag {
        CONTAINMENT_TAG => {
            require_array_length(
                position,
                "containment projection witness",
                CONTAINMENT_WITNESS_ARITY,
                arity,
            )?;
            let actor = ActorId::from_bytes(decode_fixed(&mut decoder, "witness actor")?);
            let len = decode_sequence_len(&mut decoder, "containment belief observations")?;
            let mut observations = Vec::new();
            for _ in 0..len {
                observations.push(decode_belief_observation(&mut decoder)?);
            }
            ActionProjectionWitness::Containment(ContainmentPolicyWitness {
                actor,
                observations,
            })
        }
        RELOCATION_TAG => {
            require_array_length(
                position,
                "relocation projection witness",
                RELOCATION_WITNESS_ARITY,
                arity,
            )?;
            ActionProjectionWitness::RelocationNoRead
        }
        actual => {
            return Err(ActionArtifactCodecError::InvalidTag {
                position: tag_position,
                context: "projection witness kind",
                actual,
            });
        }
    };
    finish(&decoder, bytes)?;
    validate_projection_witness(&witness)?;
    require_canonical(bytes, &encode_action_projection_witness(&witness))?;
    Ok(witness)
}

/// Encodes one complete private action execution-validation witness.
#[must_use]
pub fn encode_action_execution_witness(witness: &ActionExecutionWitness) -> Vec<u8> {
    let mut encoder = Encoder::new(Vec::new());
    match witness {
        ActionExecutionWitness::Containment(observations) => {
            encode_array(
                &mut encoder,
                "containment execution witness",
                CONTAINMENT_EXECUTION_WITNESS_ARITY,
            );
            encode_u16(
                &mut encoder,
                "action execution witness schema",
                ACTION_EXECUTION_WITNESS_SCHEMA_VERSION,
            );
            encode_u16(&mut encoder, "execution witness kind", CONTAINMENT_TAG);
            encode_sequence(
                &mut encoder,
                "candidate execution observations",
                observations.len(),
            );
            for observation in observations {
                encode_candidate_execution_observation(&mut encoder, *observation);
            }
        }
        ActionExecutionWitness::RelocationNoRead => {
            encode_array(
                &mut encoder,
                "relocation execution witness",
                RELOCATION_EXECUTION_WITNESS_ARITY,
            );
            encode_u16(
                &mut encoder,
                "action execution witness schema",
                ACTION_EXECUTION_WITNESS_SCHEMA_VERSION,
            );
            encode_u16(&mut encoder, "execution witness kind", RELOCATION_TAG);
        }
    }
    encoder.into_writer()
}

/// Decodes and revalidates one canonical execution-validation witness.
pub fn decode_action_execution_witness(
    bytes: &[u8],
) -> Result<ActionExecutionWitness, ActionArtifactCodecError> {
    let mut decoder = Decoder::new(bytes);
    let position = decoder.position();
    let arity = decoder
        .array()
        .map_err(|error| unexpected_cbor(error, position, "action execution witness"))?
        .ok_or(ActionArtifactCodecError::IndefiniteArray {
            position,
            context: "action execution witness",
        })?;
    decode_schema(
        &mut decoder,
        "action execution witness",
        ACTION_EXECUTION_WITNESS_SCHEMA_VERSION,
    )?;
    let tag_position = decoder.position();
    let witness = match decode_u16(&mut decoder, "execution witness kind")? {
        CONTAINMENT_TAG => {
            require_array_length(
                position,
                "containment execution witness",
                CONTAINMENT_EXECUTION_WITNESS_ARITY,
                arity,
            )?;
            let len = decode_sequence_len(&mut decoder, "candidate execution observations")?;
            let mut observations = Vec::new();
            for _ in 0..len {
                observations.push(decode_candidate_execution_observation(&mut decoder)?);
            }
            ActionExecutionWitness::Containment(observations)
        }
        RELOCATION_TAG => {
            require_array_length(
                position,
                "relocation execution witness",
                RELOCATION_EXECUTION_WITNESS_ARITY,
                arity,
            )?;
            ActionExecutionWitness::RelocationNoRead
        }
        actual => {
            return Err(ActionArtifactCodecError::InvalidTag {
                position: tag_position,
                context: "execution witness kind",
                actual,
            });
        }
    };
    finish(&decoder, bytes)?;
    validate_execution_witness(&witness)?;
    require_canonical(bytes, &encode_action_execution_witness(&witness))?;
    Ok(witness)
}

/// Encodes one complete combined private action-read witness.
#[must_use]
pub fn encode_action_read_witness(witness: &ActionReadWitness) -> Vec<u8> {
    let projection = encode_action_projection_witness(&witness.projection);
    let execution = encode_action_execution_witness(&witness.execution);
    let mut encoder = Encoder::new(Vec::new());
    encode_array(
        &mut encoder,
        "action read witness",
        ACTION_READ_WITNESS_ARITY,
    );
    encode_u16(
        &mut encoder,
        "action read witness schema",
        ACTION_READ_WITNESS_SCHEMA_VERSION,
    );
    encode_bytes(&mut encoder, "projection witness artifact", &projection);
    encode_bytes(&mut encoder, "execution witness artifact", &execution);
    encoder.into_writer()
}

/// Decodes and revalidates one combined canonical private action-read witness.
pub fn decode_action_read_witness(
    bytes: &[u8],
) -> Result<ActionReadWitness, ActionArtifactCodecError> {
    let mut decoder = Decoder::new(bytes);
    expect_array(
        &mut decoder,
        "action read witness",
        ACTION_READ_WITNESS_ARITY,
    )?;
    decode_schema(
        &mut decoder,
        "action read witness",
        ACTION_READ_WITNESS_SCHEMA_VERSION,
    )?;
    let projection = decode_action_projection_witness(decode_blob(
        &mut decoder,
        "projection witness artifact",
    )?)?;
    let execution =
        decode_action_execution_witness(decode_blob(&mut decoder, "execution witness artifact")?)?;
    finish(&decoder, bytes)?;
    let witness = ActionReadWitness {
        projection,
        execution,
    };
    validate_read_witness(&witness)?;
    require_canonical(bytes, &encode_action_read_witness(&witness))?;
    Ok(witness)
}

fn encode_actor_safe_interaction(
    encoder: &mut Encoder<Vec<u8>>,
    interaction: &ActorSafeActionInteraction,
) {
    match interaction {
        ActorSafeActionInteraction::Containment(interaction) => {
            encode_array(
                encoder,
                "containment actor-safe interaction",
                CONTAINMENT_INTERACTION_ARITY,
            );
            encode_u16(encoder, "actor-safe interaction kind", CONTAINMENT_TAG);
            encode_bytes(
                encoder,
                "containment source reference",
                interaction.source.as_bytes(),
            );
            encode_sequence(
                encoder,
                "containment destination references",
                interaction.destinations.len(),
            );
            for destination in &interaction.destinations {
                encode_bytes(
                    encoder,
                    "containment destination reference",
                    destination.as_bytes(),
                );
            }
            encode_sequence(
                encoder,
                "containment item references",
                interaction.items.len(),
            );
            for item in &interaction.items {
                encode_bytes(encoder, "containment item reference", item.as_bytes());
            }
        }
        ActorSafeActionInteraction::Relocation(interaction) => {
            encode_array(
                encoder,
                "relocation actor-safe interaction",
                RELOCATION_INTERACTION_ARITY,
            );
            encode_u16(encoder, "actor-safe interaction kind", RELOCATION_TAG);
            encode_sequence(
                encoder,
                "relocation interaction entries",
                interaction.interactions.len(),
            );
            for entry in &interaction.interactions {
                encode_array(
                    encoder,
                    "relocation interaction entry",
                    RELOCATION_ENTRY_ARITY,
                );
                encode_relocation_verb(encoder, entry.verb);
                encode_bytes(
                    encoder,
                    "relocation source reference",
                    entry.source.as_bytes(),
                );
                encode_bytes(
                    encoder,
                    "relocation destination reference",
                    entry.destination.as_bytes(),
                );
            }
        }
    }
}

fn decode_actor_safe_interaction(
    decoder: &mut Decoder<'_>,
) -> Result<ActorSafeActionInteraction, ActionArtifactCodecError> {
    let position = decoder.position();
    let arity = decoder
        .array()
        .map_err(|error| unexpected_cbor(error, position, "actor-safe interaction"))?
        .ok_or(ActionArtifactCodecError::IndefiniteArray {
            position,
            context: "actor-safe interaction",
        })?;
    let tag_position = decoder.position();
    match decode_u16(decoder, "actor-safe interaction kind")? {
        CONTAINMENT_TAG => {
            require_array_length(
                position,
                "containment actor-safe interaction",
                CONTAINMENT_INTERACTION_ARITY,
                arity,
            )?;
            let source = ActorSafeObjectRef::from_bytes(decode_fixed(
                decoder,
                "containment source reference",
            )?);
            let destination_len =
                decode_sequence_len(decoder, "containment destination references")?;
            let mut destinations = Vec::new();
            for _ in 0..destination_len {
                destinations.push(ActorSafeObjectRef::from_bytes(decode_fixed(
                    decoder,
                    "containment destination reference",
                )?));
            }
            let item_len = decode_sequence_len(decoder, "containment item references")?;
            let mut items = Vec::new();
            for _ in 0..item_len {
                items.push(ActorSafeObjectRef::from_bytes(decode_fixed(
                    decoder,
                    "containment item reference",
                )?));
            }
            Ok(ActorSafeActionInteraction::Containment(
                ActorSafeContainmentInteraction {
                    source,
                    destinations,
                    items,
                },
            ))
        }
        RELOCATION_TAG => {
            require_array_length(
                position,
                "relocation actor-safe interaction",
                RELOCATION_INTERACTION_ARITY,
                arity,
            )?;
            let len = decode_sequence_len(decoder, "relocation interaction entries")?;
            let mut interactions = Vec::new();
            for _ in 0..len {
                expect_array(
                    decoder,
                    "relocation interaction entry",
                    RELOCATION_ENTRY_ARITY,
                )?;
                interactions.push(ActorSafeRelocationInteractionEntry {
                    verb: decode_relocation_verb(decoder)?,
                    source: ActorSafeObjectRef::from_bytes(decode_fixed(
                        decoder,
                        "relocation source reference",
                    )?),
                    destination: ActorSafeObjectRef::from_bytes(decode_fixed(
                        decoder,
                        "relocation destination reference",
                    )?),
                });
            }
            Ok(ActorSafeActionInteraction::Relocation(
                ActorSafeRelocationInteraction { interactions },
            ))
        }
        actual => Err(ActionArtifactCodecError::InvalidTag {
            position: tag_position,
            context: "actor-safe interaction kind",
            actual,
        }),
    }
}

fn encode_candidate_set(encoder: &mut Encoder<Vec<u8>>, candidates: &GroundedActionCandidateSet) {
    encode_array(encoder, "grounded candidate set", CANDIDATE_SET_ARITY);
    encode_bytes(
        encoder,
        "candidate-set opportunity",
        candidates.opportunity.as_bytes(),
    );
    encode_bytes(
        encoder,
        "grounding semantics",
        candidates.grounding_semantics.as_bytes(),
    );
    encode_u32(encoder, "candidate limit", candidates.candidate_limit);
    encode_coverage(encoder, candidates.coverage);
    encode_sequence(
        encoder,
        "grounded action candidates",
        candidates.candidates.len(),
    );
    for candidate in &candidates.candidates {
        encode_candidate(encoder, candidate);
    }
    encode_bytes(
        encoder,
        "candidate-set fingerprint",
        candidates.fingerprint.as_bytes(),
    );
}

fn decode_candidate_set(
    decoder: &mut Decoder<'_>,
) -> Result<GroundedActionCandidateSet, ActionArtifactCodecError> {
    expect_array(decoder, "grounded candidate set", CANDIDATE_SET_ARITY)?;
    let opportunity =
        ActionOpportunityId::from_bytes(decode_fixed(decoder, "candidate-set opportunity")?);
    let grounding_semantics =
        GroundingSemanticsId::from_bytes(decode_fixed(decoder, "grounding semantics")?);
    let candidate_limit = decode_u32(decoder, "candidate limit")?;
    let coverage = decode_coverage(decoder)?;
    let len = decode_sequence_len(decoder, "grounded action candidates")?;
    let mut candidates = Vec::new();
    for _ in 0..len {
        candidates.push(decode_candidate(decoder)?);
    }
    let fingerprint = GroundedCandidateSetFingerprint::from_bytes(decode_fixed(
        decoder,
        "candidate-set fingerprint",
    )?);
    Ok(GroundedActionCandidateSet {
        opportunity,
        grounding_semantics,
        candidate_limit,
        coverage,
        candidates,
        fingerprint,
    })
}

fn encode_candidate(encoder: &mut Encoder<Vec<u8>>, candidate: &GroundedActionCandidate) {
    encode_array(encoder, "grounded action candidate", CANDIDATE_ARITY);
    encode_bytes(encoder, "candidate identity", candidate.id.as_bytes());
    encode_bytes(
        encoder,
        "candidate opportunity",
        candidate.opportunity.as_bytes(),
    );
    encode_definition_key(encoder, &candidate.action);
    encode_candidate_interaction(encoder, candidate.interaction);
    encode_sequence(
        encoder,
        "actor-safe candidate bindings",
        candidate.bindings.len(),
    );
    for binding in &candidate.bindings {
        encode_binding(encoder, binding);
    }
}

fn decode_candidate(
    decoder: &mut Decoder<'_>,
) -> Result<GroundedActionCandidate, ActionArtifactCodecError> {
    expect_array(decoder, "grounded action candidate", CANDIDATE_ARITY)?;
    let id = GroundedActionCandidateId::from_bytes(decode_fixed(decoder, "candidate identity")?);
    let opportunity =
        ActionOpportunityId::from_bytes(decode_fixed(decoder, "candidate opportunity")?);
    let action = decode_definition_key(decoder)?;
    let interaction = decode_candidate_interaction(decoder)?;
    let len = decode_sequence_len(decoder, "actor-safe candidate bindings")?;
    let mut bindings = Vec::new();
    for _ in 0..len {
        bindings.push(decode_binding(decoder)?);
    }
    Ok(GroundedActionCandidate {
        id,
        opportunity,
        action,
        interaction,
        bindings,
    })
}

fn encode_definition_key(encoder: &mut Encoder<Vec<u8>>, key: &DefinitionKey) {
    encode_array(encoder, "definition key", DEFINITION_KEY_ARITY);
    encode_text(encoder, "pack key", key.pack_key().as_str());
    encode_text(encoder, "local definition name", key.local_name().as_str());
}

fn decode_definition_key(
    decoder: &mut Decoder<'_>,
) -> Result<DefinitionKey, ActionArtifactCodecError> {
    expect_array(decoder, "definition key", DEFINITION_KEY_ARITY)?;
    let pack_position = decoder.position();
    let pack = decode_text(decoder, "pack key")?;
    let pack = PackKey::parse(pack).map_err(|_| ActionArtifactCodecError::InvalidName {
        position: pack_position,
        context: "pack key",
    })?;
    let local_position = decoder.position();
    let local = decode_text(decoder, "local definition name")?;
    let local =
        LocalDefinitionName::parse(local).map_err(|_| ActionArtifactCodecError::InvalidName {
            position: local_position,
            context: "local definition name",
        })?;
    Ok(DefinitionKey::new(pack, local))
}

fn encode_candidate_interaction(
    encoder: &mut Encoder<Vec<u8>>,
    interaction: GroundedActionInteraction,
) {
    match interaction {
        GroundedActionInteraction::ContainmentTransfer => {
            encode_array(
                encoder,
                "containment candidate interaction",
                CONTAINMENT_CANDIDATE_INTERACTION_ARITY,
            );
            encode_u16(encoder, "candidate interaction kind", CONTAINMENT_TAG);
        }
        GroundedActionInteraction::Relocation(verb) => {
            encode_array(
                encoder,
                "relocation candidate interaction",
                RELOCATION_CANDIDATE_INTERACTION_ARITY,
            );
            encode_u16(encoder, "candidate interaction kind", RELOCATION_TAG);
            encode_relocation_verb(encoder, verb);
        }
    }
}

fn decode_candidate_interaction(
    decoder: &mut Decoder<'_>,
) -> Result<GroundedActionInteraction, ActionArtifactCodecError> {
    let position = decoder.position();
    let arity = decoder
        .array()
        .map_err(|error| unexpected_cbor(error, position, "candidate interaction"))?
        .ok_or(ActionArtifactCodecError::IndefiniteArray {
            position,
            context: "candidate interaction",
        })?;
    let tag_position = decoder.position();
    match decode_u16(decoder, "candidate interaction kind")? {
        CONTAINMENT_TAG => {
            require_array_length(
                position,
                "containment candidate interaction",
                CONTAINMENT_CANDIDATE_INTERACTION_ARITY,
                arity,
            )?;
            Ok(GroundedActionInteraction::ContainmentTransfer)
        }
        RELOCATION_TAG => {
            require_array_length(
                position,
                "relocation candidate interaction",
                RELOCATION_CANDIDATE_INTERACTION_ARITY,
                arity,
            )?;
            Ok(GroundedActionInteraction::Relocation(
                decode_relocation_verb(decoder)?,
            ))
        }
        actual => Err(ActionArtifactCodecError::InvalidTag {
            position: tag_position,
            context: "candidate interaction kind",
            actual,
        }),
    }
}

fn encode_binding(encoder: &mut Encoder<Vec<u8>>, binding: &ActorSafeBinding) {
    encode_array(encoder, "actor-safe binding", BINDING_ARITY);
    encode_text(encoder, "binding name", binding.name.as_str());
    encode_array(encoder, "actor-safe binding value", BINDING_VALUE_ARITY);
    match binding.value {
        ActorSafeBindingValue::Actor(actor) => {
            encode_u16(encoder, "binding value kind", ACTOR_BINDING_TAG);
            encode_bytes(encoder, "bound actor", actor.as_bytes());
        }
        ActorSafeBindingValue::Object(object) => {
            encode_u16(encoder, "binding value kind", OBJECT_BINDING_TAG);
            encode_bytes(encoder, "bound object reference", object.as_bytes());
        }
    }
}

fn decode_binding(decoder: &mut Decoder<'_>) -> Result<ActorSafeBinding, ActionArtifactCodecError> {
    expect_array(decoder, "actor-safe binding", BINDING_ARITY)?;
    let name_position = decoder.position();
    let name = decode_text(decoder, "binding name")?;
    let name = BindingName::parse(name).map_err(|_| ActionArtifactCodecError::InvalidName {
        position: name_position,
        context: "binding name",
    })?;
    expect_array(decoder, "actor-safe binding value", BINDING_VALUE_ARITY)?;
    let tag_position = decoder.position();
    let value = match decode_u16(decoder, "binding value kind")? {
        ACTOR_BINDING_TAG => {
            ActorSafeBindingValue::Actor(ActorId::from_bytes(decode_fixed(decoder, "bound actor")?))
        }
        OBJECT_BINDING_TAG => ActorSafeBindingValue::Object(ActorSafeObjectRef::from_bytes(
            decode_fixed(decoder, "bound object reference")?,
        )),
        actual => {
            return Err(ActionArtifactCodecError::InvalidTag {
                position: tag_position,
                context: "binding value kind",
                actual,
            });
        }
    };
    Ok(ActorSafeBinding { name, value })
}

fn encode_private_resolution(
    encoder: &mut Encoder<Vec<u8>>,
    resolution: &PrivateCandidateResolution,
) {
    match resolution {
        PrivateCandidateResolution::Containment {
            candidate,
            action,
            actor,
            item,
            source,
            destination,
        } => {
            encode_array(
                encoder,
                "private containment resolution",
                PRIVATE_CONTAINMENT_RESOLUTION_ARITY,
            );
            encode_u16(encoder, "private resolution kind", CONTAINMENT_TAG);
            encode_bytes(encoder, "resolved candidate", candidate.as_bytes());
            encode_definition_key(encoder, action);
            encode_bytes(encoder, "resolved actor", actor.as_bytes());
            encode_bytes(encoder, "resolved item reference", item.as_bytes());
            encode_bytes(encoder, "resolved source reference", source.as_bytes());
            encode_bytes(
                encoder,
                "resolved destination reference",
                destination.as_bytes(),
            );
        }
        PrivateCandidateResolution::Relocation {
            candidate,
            action,
            actor,
            interaction,
        } => {
            encode_array(
                encoder,
                "private relocation resolution",
                PRIVATE_RELOCATION_RESOLUTION_ARITY,
            );
            encode_u16(encoder, "private resolution kind", RELOCATION_TAG);
            encode_bytes(encoder, "resolved candidate", candidate.as_bytes());
            encode_definition_key(encoder, action);
            encode_bytes(encoder, "resolved actor", actor.as_bytes());
            encode_exact_relocation_interaction(encoder, *interaction);
        }
    }
}

fn decode_private_resolution(
    decoder: &mut Decoder<'_>,
) -> Result<PrivateCandidateResolution, ActionArtifactCodecError> {
    let position = decoder.position();
    let arity = decoder
        .array()
        .map_err(|error| unexpected_cbor(error, position, "private candidate resolution"))?
        .ok_or(ActionArtifactCodecError::IndefiniteArray {
            position,
            context: "private candidate resolution",
        })?;
    let tag_position = decoder.position();
    match decode_u16(decoder, "private resolution kind")? {
        CONTAINMENT_TAG => {
            require_array_length(
                position,
                "private containment resolution",
                PRIVATE_CONTAINMENT_RESOLUTION_ARITY,
                arity,
            )?;
            Ok(PrivateCandidateResolution::Containment {
                candidate: GroundedActionCandidateId::from_bytes(decode_fixed(
                    decoder,
                    "resolved candidate",
                )?),
                action: decode_definition_key(decoder)?,
                actor: ActorId::from_bytes(decode_fixed(decoder, "resolved actor")?),
                item: ActorSafeObjectRef::from_bytes(decode_fixed(
                    decoder,
                    "resolved item reference",
                )?),
                source: ActorSafeObjectRef::from_bytes(decode_fixed(
                    decoder,
                    "resolved source reference",
                )?),
                destination: ActorSafeObjectRef::from_bytes(decode_fixed(
                    decoder,
                    "resolved destination reference",
                )?),
            })
        }
        RELOCATION_TAG => {
            require_array_length(
                position,
                "private relocation resolution",
                PRIVATE_RELOCATION_RESOLUTION_ARITY,
                arity,
            )?;
            Ok(PrivateCandidateResolution::Relocation {
                candidate: GroundedActionCandidateId::from_bytes(decode_fixed(
                    decoder,
                    "resolved candidate",
                )?),
                action: decode_definition_key(decoder)?,
                actor: ActorId::from_bytes(decode_fixed(decoder, "resolved actor")?),
                interaction: decode_exact_relocation_interaction(decoder)?,
            })
        }
        actual => Err(ActionArtifactCodecError::InvalidTag {
            position: tag_position,
            context: "private resolution kind",
            actual,
        }),
    }
}

fn encode_exact_relocation_interaction(
    encoder: &mut Encoder<Vec<u8>>,
    interaction: RelocationInteraction,
) {
    encode_array(
        encoder,
        "exact relocation interaction",
        EXACT_RELOCATION_INTERACTION_ARITY,
    );
    match interaction {
        RelocationInteraction::Start(route) => {
            encode_u16(encoder, "exact relocation kind", START_RELOCATION_TAG);
            encode_bytes(encoder, "relocation route", route.as_bytes());
        }
        RelocationInteraction::Pause(route) => {
            encode_u16(encoder, "exact relocation kind", PAUSE_RELOCATION_TAG);
            encode_bytes(encoder, "relocation route", route.as_bytes());
        }
        RelocationInteraction::Resume(route) => {
            encode_u16(encoder, "exact relocation kind", RESUME_RELOCATION_TAG);
            encode_bytes(encoder, "relocation route", route.as_bytes());
        }
    }
}

fn decode_exact_relocation_interaction(
    decoder: &mut Decoder<'_>,
) -> Result<RelocationInteraction, ActionArtifactCodecError> {
    expect_array(
        decoder,
        "exact relocation interaction",
        EXACT_RELOCATION_INTERACTION_ARITY,
    )?;
    let tag_position = decoder.position();
    let tag = decode_u16(decoder, "exact relocation kind")?;
    let route = RelocationRouteId::from_bytes(decode_fixed(decoder, "relocation route")?);
    match tag {
        START_RELOCATION_TAG => Ok(RelocationInteraction::Start(route)),
        PAUSE_RELOCATION_TAG => Ok(RelocationInteraction::Pause(route)),
        RESUME_RELOCATION_TAG => Ok(RelocationInteraction::Resume(route)),
        actual => Err(ActionArtifactCodecError::InvalidTag {
            position: tag_position,
            context: "exact relocation kind",
            actual,
        }),
    }
}

fn encode_belief_observation(
    encoder: &mut Encoder<Vec<u8>>,
    observation: &ContainmentBeliefObservation,
) {
    encode_array(
        encoder,
        "containment belief observation",
        BELIEF_OBSERVATION_ARITY,
    );
    encode_bytes(encoder, "observed item", observation.item.as_bytes());
    match observation.believed_container {
        None => {
            encode_array(encoder, "absent containment belief", ABSENT_BELIEF_ARITY);
            encode_u16(encoder, "belief presence", ABSENT_BELIEF_TAG);
        }
        Some(container) => {
            encode_array(encoder, "present containment belief", PRESENT_BELIEF_ARITY);
            encode_u16(encoder, "belief presence", PRESENT_BELIEF_TAG);
            encode_bytes(encoder, "believed container", container.as_bytes());
        }
    }
}

fn decode_belief_observation(
    decoder: &mut Decoder<'_>,
) -> Result<ContainmentBeliefObservation, ActionArtifactCodecError> {
    expect_array(
        decoder,
        "containment belief observation",
        BELIEF_OBSERVATION_ARITY,
    )?;
    let item = EntityId::from_bytes(decode_fixed(decoder, "observed item")?);
    let position = decoder.position();
    let arity = decoder
        .array()
        .map_err(|error| unexpected_cbor(error, position, "containment belief presence"))?
        .ok_or(ActionArtifactCodecError::IndefiniteArray {
            position,
            context: "containment belief presence",
        })?;
    let tag_position = decoder.position();
    let believed_container = match decode_u16(decoder, "belief presence")? {
        ABSENT_BELIEF_TAG => {
            require_array_length(
                position,
                "absent containment belief",
                ABSENT_BELIEF_ARITY,
                arity,
            )?;
            None
        }
        PRESENT_BELIEF_TAG => {
            require_array_length(
                position,
                "present containment belief",
                PRESENT_BELIEF_ARITY,
                arity,
            )?;
            Some(EntityId::from_bytes(decode_fixed(
                decoder,
                "believed container",
            )?))
        }
        actual => {
            return Err(ActionArtifactCodecError::InvalidTag {
                position: tag_position,
                context: "belief presence",
                actual,
            });
        }
    };
    Ok(ContainmentBeliefObservation {
        item,
        believed_container,
    })
}

fn encode_candidate_execution_observation(
    encoder: &mut Encoder<Vec<u8>>,
    observation: ContainmentCandidateExecutionWitness,
) {
    encode_array(
        encoder,
        "candidate execution observation",
        CANDIDATE_EXECUTION_OBSERVATION_ARITY,
    );
    encode_bytes(
        encoder,
        "execution candidate identity",
        observation.candidate.as_bytes(),
    );
    encode_optional_entity(encoder, "item container", observation.item_container);
    encode_bool(encoder, "source existence", observation.source_exists);
    encode_bool(
        encoder,
        "actor source control",
        observation.actor_controls_source,
    );
    encode_optional_u32(
        encoder,
        "destination capacity",
        observation.destination_capacity,
    );
    encode_u64(
        encoder,
        "destination direct-item count",
        observation.destination_direct_item_count,
    );
}

fn decode_candidate_execution_observation(
    decoder: &mut Decoder<'_>,
) -> Result<ContainmentCandidateExecutionWitness, ActionArtifactCodecError> {
    expect_array(
        decoder,
        "candidate execution observation",
        CANDIDATE_EXECUTION_OBSERVATION_ARITY,
    )?;
    Ok(ContainmentCandidateExecutionWitness {
        candidate: GroundedActionCandidateId::from_bytes(decode_fixed(
            decoder,
            "execution candidate identity",
        )?),
        item_container: decode_optional_entity(decoder, "item container")?,
        source_exists: decode_bool(decoder, "source existence")?,
        actor_controls_source: decode_bool(decoder, "actor source control")?,
        destination_capacity: decode_optional_u32(decoder, "destination capacity")?,
        destination_direct_item_count: decode_u64(decoder, "destination direct-item count")?,
    })
}

fn encode_optional_entity(
    encoder: &mut Encoder<Vec<u8>>,
    context: &'static str,
    value: Option<EntityId>,
) {
    match value {
        None => {
            encode_array(encoder, context, ABSENT_OPTION_ARITY);
            encode_u16(encoder, "option presence", ABSENT_BELIEF_TAG);
        }
        Some(value) => {
            encode_array(encoder, context, PRESENT_OPTION_ARITY);
            encode_u16(encoder, "option presence", PRESENT_BELIEF_TAG);
            encode_bytes(encoder, context, value.as_bytes());
        }
    }
}

fn decode_optional_entity(
    decoder: &mut Decoder<'_>,
    context: &'static str,
) -> Result<Option<EntityId>, ActionArtifactCodecError> {
    decode_option(decoder, context, |decoder| {
        Ok(EntityId::from_bytes(decode_fixed(decoder, context)?))
    })
}

fn encode_optional_u32(encoder: &mut Encoder<Vec<u8>>, context: &'static str, value: Option<u32>) {
    match value {
        None => {
            encode_array(encoder, context, ABSENT_OPTION_ARITY);
            encode_u16(encoder, "option presence", ABSENT_BELIEF_TAG);
        }
        Some(value) => {
            encode_array(encoder, context, PRESENT_OPTION_ARITY);
            encode_u16(encoder, "option presence", PRESENT_BELIEF_TAG);
            encode_u32(encoder, context, value);
        }
    }
}

fn decode_optional_u32(
    decoder: &mut Decoder<'_>,
    context: &'static str,
) -> Result<Option<u32>, ActionArtifactCodecError> {
    decode_option(decoder, context, |decoder| decode_u32(decoder, context))
}

fn decode_option<T>(
    decoder: &mut Decoder<'_>,
    context: &'static str,
    present: impl FnOnce(&mut Decoder<'_>) -> Result<T, ActionArtifactCodecError>,
) -> Result<Option<T>, ActionArtifactCodecError> {
    let position = decoder.position();
    let arity = decoder
        .array()
        .map_err(|error| unexpected_cbor(error, position, context))?
        .ok_or(ActionArtifactCodecError::IndefiniteArray { position, context })?;
    let tag_position = decoder.position();
    match decode_u16(decoder, "option presence")? {
        ABSENT_BELIEF_TAG => {
            require_array_length(position, context, ABSENT_OPTION_ARITY, arity)?;
            Ok(None)
        }
        PRESENT_BELIEF_TAG => {
            require_array_length(position, context, PRESENT_OPTION_ARITY, arity)?;
            present(decoder).map(Some)
        }
        actual => Err(ActionArtifactCodecError::InvalidTag {
            position: tag_position,
            context: "option presence",
            actual,
        }),
    }
}

fn validate_payload(payload: &ActionContextPayload) -> Result<(), ActionArtifactCodecError> {
    let set = &payload.candidates;
    if set.opportunity != payload.opportunity {
        return Err(ActionArtifactCodecError::InvalidValue {
            context: "candidate-set opportunity",
        });
    }
    if set.candidate_limit == 0
        || set.candidates.len()
            > usize::try_from(set.candidate_limit).map_err(|_| {
                ActionArtifactCodecError::InvalidValue {
                    context: "candidate limit",
                }
            })?
    {
        return Err(ActionArtifactCodecError::InvalidValue {
            context: "candidate limit",
        });
    }
    if set.coverage == CandidateCoverage::BudgetLimited
        && set.candidates.len() != set.candidate_limit as usize
    {
        return Err(ActionArtifactCodecError::InvalidValue {
            context: "budget-limited candidate set",
        });
    }

    let mut candidate_ids = BTreeSet::new();
    for candidate in &set.candidates {
        if candidate.opportunity != payload.opportunity {
            return Err(ActionArtifactCodecError::InvalidValue {
                context: "candidate opportunity",
            });
        }
        if !candidate_ids.insert(candidate.id) {
            return Err(ActionArtifactCodecError::InvalidValue {
                context: "duplicate candidate identity",
            });
        }
        let expected = derive_candidate_id(
            candidate.opportunity,
            &candidate.action,
            candidate.interaction,
            &candidate.bindings,
            set.grounding_semantics,
        )
        .map_err(ActionArtifactCodecError::Canonical)?;
        if candidate.id != expected {
            return Err(ActionArtifactCodecError::IdentityMismatch {
                context: "grounded action candidate",
            });
        }
    }

    match &payload.interaction {
        ActorSafeActionInteraction::Containment(interaction) => {
            validate_strictly_sorted(
                &interaction.destinations,
                "containment destination references",
            )?;
            validate_strictly_sorted(&interaction.items, "containment item references")?;
            let mut order = None;
            let mut used_items = BTreeSet::new();
            for candidate in &set.candidates {
                if candidate.interaction != GroundedActionInteraction::ContainmentTransfer {
                    return Err(ActionArtifactCodecError::InvalidValue {
                        context: "containment candidate interaction",
                    });
                }
                let (destination, item) =
                    validate_containment_bindings(payload.actor, interaction, candidate)?;
                let current = (candidate.action.clone(), destination, item);
                if order.as_ref().is_some_and(|previous| previous >= &current) {
                    return Err(ActionArtifactCodecError::InvalidValue {
                        context: "containment candidate order",
                    });
                }
                order = Some(current);
                used_items.insert(item);
            }
            if used_items.len() != interaction.items.len()
                || interaction
                    .items
                    .iter()
                    .any(|item| !used_items.contains(item))
            {
                return Err(ActionArtifactCodecError::InvalidValue {
                    context: "containment interaction items",
                });
            }
        }
        ActorSafeActionInteraction::Relocation(interaction) => {
            if interaction.interactions.len() != set.candidates.len() {
                return Err(ActionArtifactCodecError::InvalidValue {
                    context: "relocation interaction count",
                });
            }
            let mut previous_id = None;
            for (entry, candidate) in interaction.interactions.iter().zip(&set.candidates) {
                if previous_id.is_some_and(|previous| previous >= candidate.id) {
                    return Err(ActionArtifactCodecError::InvalidValue {
                        context: "relocation candidate order",
                    });
                }
                previous_id = Some(candidate.id);
                if candidate.interaction != GroundedActionInteraction::Relocation(entry.verb) {
                    return Err(ActionArtifactCodecError::InvalidValue {
                        context: "relocation candidate interaction",
                    });
                }
                validate_relocation_bindings(payload.actor, *entry, candidate)?;
            }
        }
    }

    let expected_set = candidate_set_fingerprint(
        set.opportunity,
        set.grounding_semantics,
        set.candidate_limit,
        set.coverage,
        &set.candidates,
    )
    .map_err(ActionArtifactCodecError::Canonical)?;
    if set.fingerprint != expected_set {
        return Err(ActionArtifactCodecError::IdentityMismatch {
            context: "grounded candidate set",
        });
    }
    let expected_input = action_input_fingerprint(
        payload.actor,
        payload.opportunity,
        &payload.interaction,
        &payload.candidates,
        payload.policy_semantics,
    )
    .map_err(ActionArtifactCodecError::Canonical)?;
    if payload.input_fingerprint != expected_input {
        return Err(ActionArtifactCodecError::IdentityMismatch {
            context: "action context payload",
        });
    }
    Ok(())
}

fn validate_containment_bindings(
    actor: ActorId,
    interaction: &ActorSafeContainmentInteraction,
    candidate: &GroundedActionCandidate,
) -> Result<(ActorSafeObjectRef, ActorSafeObjectRef), ActionArtifactCodecError> {
    let [
        actor_binding,
        destination_binding,
        item_binding,
        source_binding,
    ] = candidate.bindings.as_slice()
    else {
        return Err(ActionArtifactCodecError::InvalidValue {
            context: "containment candidate binding count",
        });
    };
    require_binding(
        actor_binding,
        ACTOR_ROLE,
        ActorSafeBindingValue::Actor(actor),
    )?;
    let destination = require_object_binding(destination_binding, DESTINATION_ROLE)?;
    let item = require_object_binding(item_binding, ITEM_ROLE)?;
    let source = require_object_binding(source_binding, SOURCE_ROLE)?;
    if source != interaction.source
        || interaction
            .destinations
            .binary_search(&destination)
            .is_err()
        || interaction.items.binary_search(&item).is_err()
    {
        return Err(ActionArtifactCodecError::InvalidValue {
            context: "containment candidate binding",
        });
    }
    Ok((destination, item))
}

fn validate_relocation_bindings(
    actor: ActorId,
    entry: ActorSafeRelocationInteractionEntry,
    candidate: &GroundedActionCandidate,
) -> Result<(), ActionArtifactCodecError> {
    let [actor_binding, destination_binding, source_binding] = candidate.bindings.as_slice() else {
        return Err(ActionArtifactCodecError::InvalidValue {
            context: "relocation candidate binding count",
        });
    };
    require_binding(
        actor_binding,
        ACTOR_ROLE,
        ActorSafeBindingValue::Actor(actor),
    )?;
    require_binding(
        destination_binding,
        DESTINATION_ROLE,
        ActorSafeBindingValue::Object(entry.destination),
    )?;
    require_binding(
        source_binding,
        SOURCE_ROLE,
        ActorSafeBindingValue::Object(entry.source),
    )
}

fn require_object_binding(
    binding: &ActorSafeBinding,
    name: &'static str,
) -> Result<ActorSafeObjectRef, ActionArtifactCodecError> {
    let ActorSafeBindingValue::Object(object) = binding.value else {
        return Err(ActionArtifactCodecError::InvalidValue {
            context: "actor-safe binding value",
        });
    };
    if binding.name.as_str() != name {
        return Err(ActionArtifactCodecError::InvalidValue {
            context: "actor-safe binding name",
        });
    }
    Ok(object)
}

fn require_binding(
    binding: &ActorSafeBinding,
    name: &'static str,
    value: ActorSafeBindingValue,
) -> Result<(), ActionArtifactCodecError> {
    if binding.name.as_str() != name || binding.value != value {
        return Err(ActionArtifactCodecError::InvalidValue {
            context: "actor-safe binding",
        });
    }
    Ok(())
}

fn validate_resolution_table(
    table: &CandidateResolutionTable,
) -> Result<(), ActionArtifactCodecError> {
    validate_strictly_sorted_by(
        &table.references,
        |reference| reference.actor_safe,
        "private object references",
    )?;
    let mut candidates = BTreeSet::new();
    let mut family = None;
    for resolution in &table.candidates {
        if !candidates.insert(resolution.candidate()) {
            return Err(ActionArtifactCodecError::InvalidValue {
                context: "duplicate private candidate resolution",
            });
        }
        let tag = match resolution {
            PrivateCandidateResolution::Containment {
                item,
                source,
                destination,
                ..
            } => {
                for reference in [item, source, destination] {
                    if table.resolve_object(*reference).is_none() {
                        return Err(ActionArtifactCodecError::InvalidValue {
                            context: "unresolved private object reference",
                        });
                    }
                }
                CONTAINMENT_TAG
            }
            PrivateCandidateResolution::Relocation { .. } => RELOCATION_TAG,
        };
        if family.is_some_and(|existing| existing != tag) {
            return Err(ActionArtifactCodecError::InvalidValue {
                context: "mixed private resolution families",
            });
        }
        family = Some(tag);
    }
    if family == Some(RELOCATION_TAG) && !table.references.is_empty() {
        return Err(ActionArtifactCodecError::InvalidValue {
            context: "relocation private references",
        });
    }
    Ok(())
}

fn validate_projection_witness(
    witness: &ActionProjectionWitness,
) -> Result<(), ActionArtifactCodecError> {
    let ActionProjectionWitness::Containment(witness) = witness else {
        return Ok(());
    };
    validate_strictly_sorted_by(
        &witness.observations,
        |observation| observation.item,
        "containment belief observations",
    )?;
    Ok(())
}

fn validate_execution_witness(
    witness: &ActionExecutionWitness,
) -> Result<(), ActionArtifactCodecError> {
    let ActionExecutionWitness::Containment(observations) = witness else {
        return Ok(());
    };
    let mut candidates = BTreeSet::new();
    for observation in observations {
        if !candidates.insert(observation.candidate) {
            return Err(ActionArtifactCodecError::InvalidValue {
                context: "duplicate execution candidate observation",
            });
        }
    }
    Ok(())
}

fn validate_read_witness(witness: &ActionReadWitness) -> Result<(), ActionArtifactCodecError> {
    match (&witness.projection, &witness.execution) {
        (ActionProjectionWitness::Containment(_), ActionExecutionWitness::Containment(_))
        | (ActionProjectionWitness::RelocationNoRead, ActionExecutionWitness::RelocationNoRead) => {
            Ok(())
        }
        _ => Err(ActionArtifactCodecError::InvalidValue {
            context: "action read witness family",
        }),
    }
}

fn validate_strictly_sorted<T: Ord>(
    values: &[T],
    context: &'static str,
) -> Result<(), ActionArtifactCodecError> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        Err(ActionArtifactCodecError::InvalidValue { context })
    } else {
        Ok(())
    }
}

fn validate_strictly_sorted_by<T, K: Ord + Copy>(
    values: &[T],
    key: impl Fn(&T) -> K,
    context: &'static str,
) -> Result<(), ActionArtifactCodecError> {
    if values.windows(2).any(|pair| key(&pair[0]) >= key(&pair[1])) {
        Err(ActionArtifactCodecError::InvalidValue { context })
    } else {
        Ok(())
    }
}

fn encode_coverage(encoder: &mut Encoder<Vec<u8>>, coverage: CandidateCoverage) {
    let tag = match coverage {
        CandidateCoverage::Complete => COMPLETE_COVERAGE_TAG,
        CandidateCoverage::BudgetLimited => BUDGET_LIMITED_COVERAGE_TAG,
    };
    encode_u16(encoder, "candidate coverage", tag);
}

fn decode_coverage(
    decoder: &mut Decoder<'_>,
) -> Result<CandidateCoverage, ActionArtifactCodecError> {
    let position = decoder.position();
    match decode_u16(decoder, "candidate coverage")? {
        COMPLETE_COVERAGE_TAG => Ok(CandidateCoverage::Complete),
        BUDGET_LIMITED_COVERAGE_TAG => Ok(CandidateCoverage::BudgetLimited),
        actual => Err(ActionArtifactCodecError::InvalidTag {
            position,
            context: "candidate coverage",
            actual,
        }),
    }
}

fn encode_relocation_verb(encoder: &mut Encoder<Vec<u8>>, verb: RelocationActionVerb) {
    let tag = match verb {
        RelocationActionVerb::Start => START_RELOCATION_TAG,
        RelocationActionVerb::Pause => PAUSE_RELOCATION_TAG,
        RelocationActionVerb::Resume => RESUME_RELOCATION_TAG,
    };
    encode_u16(encoder, "relocation verb", tag);
}

fn decode_relocation_verb(
    decoder: &mut Decoder<'_>,
) -> Result<RelocationActionVerb, ActionArtifactCodecError> {
    let position = decoder.position();
    match decode_u16(decoder, "relocation verb")? {
        START_RELOCATION_TAG => Ok(RelocationActionVerb::Start),
        PAUSE_RELOCATION_TAG => Ok(RelocationActionVerb::Pause),
        RESUME_RELOCATION_TAG => Ok(RelocationActionVerb::Resume),
        actual => Err(ActionArtifactCodecError::InvalidTag {
            position,
            context: "relocation verb",
            actual,
        }),
    }
}

fn decode_schema(
    decoder: &mut Decoder<'_>,
    context: &'static str,
    expected: u16,
) -> Result<(), ActionArtifactCodecError> {
    let position = decoder.position();
    let actual = decode_u16(decoder, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(ActionArtifactCodecError::SchemaMismatch {
            position,
            context,
            expected,
            actual,
        })
    }
}

fn expect_array(
    decoder: &mut Decoder<'_>,
    context: &'static str,
    expected: u64,
) -> Result<(), ActionArtifactCodecError> {
    let position = decoder.position();
    let actual = decoder
        .array()
        .map_err(|error| unexpected_cbor(error, position, context))?
        .ok_or(ActionArtifactCodecError::IndefiniteArray { position, context })?;
    require_array_length(position, context, expected, actual)
}

fn require_array_length(
    position: usize,
    context: &'static str,
    expected: u64,
    actual: u64,
) -> Result<(), ActionArtifactCodecError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ActionArtifactCodecError::WrongArrayLength {
            position,
            context,
            expected,
            actual,
        })
    }
}

fn decode_sequence_len(
    decoder: &mut Decoder<'_>,
    context: &'static str,
) -> Result<usize, ActionArtifactCodecError> {
    let position = decoder.position();
    let length = decoder
        .array()
        .map_err(|error| unexpected_cbor(error, position, context))?
        .ok_or(ActionArtifactCodecError::IndefiniteArray { position, context })?;
    usize::try_from(length).map_err(|_| ActionArtifactCodecError::CollectionTooLarge {
        position,
        context,
        actual: length,
    })
}

fn decode_fixed(
    decoder: &mut Decoder<'_>,
    context: &'static str,
) -> Result<[u8; 32], ActionArtifactCodecError> {
    let position = decoder.position();
    let value = decoder
        .bytes()
        .map_err(|error| unexpected_cbor(error, position, context))?;
    value
        .try_into()
        .map_err(|_| ActionArtifactCodecError::InvalidByteLength {
            position,
            context,
            expected: 32,
            actual: value.len(),
        })
}

fn decode_blob<'bytes>(
    decoder: &mut Decoder<'bytes>,
    context: &'static str,
) -> Result<&'bytes [u8], ActionArtifactCodecError> {
    let position = decoder.position();
    decoder
        .bytes()
        .map_err(|error| unexpected_cbor(error, position, context))
}

fn decode_text<'bytes>(
    decoder: &mut Decoder<'bytes>,
    context: &'static str,
) -> Result<&'bytes str, ActionArtifactCodecError> {
    let position = decoder.position();
    decoder
        .str()
        .map_err(|error| unexpected_cbor(error, position, context))
}

fn decode_u16(
    decoder: &mut Decoder<'_>,
    context: &'static str,
) -> Result<u16, ActionArtifactCodecError> {
    let position = decoder.position();
    decoder
        .u16()
        .map_err(|error| unexpected_cbor(error, position, context))
}

fn decode_u32(
    decoder: &mut Decoder<'_>,
    context: &'static str,
) -> Result<u32, ActionArtifactCodecError> {
    let position = decoder.position();
    decoder
        .u32()
        .map_err(|error| unexpected_cbor(error, position, context))
}

fn decode_u64(
    decoder: &mut Decoder<'_>,
    context: &'static str,
) -> Result<u64, ActionArtifactCodecError> {
    let position = decoder.position();
    decoder
        .u64()
        .map_err(|error| unexpected_cbor(error, position, context))
}

fn decode_bool(
    decoder: &mut Decoder<'_>,
    context: &'static str,
) -> Result<bool, ActionArtifactCodecError> {
    let position = decoder.position();
    decoder
        .bool()
        .map_err(|error| unexpected_cbor(error, position, context))
}

fn finish(decoder: &Decoder<'_>, bytes: &[u8]) -> Result<(), ActionArtifactCodecError> {
    if decoder.position() == bytes.len() {
        Ok(())
    } else {
        Err(ActionArtifactCodecError::TrailingBytes {
            position: decoder.position(),
            remaining: bytes.len() - decoder.position(),
        })
    }
}

fn require_canonical(original: &[u8], canonical: &[u8]) -> Result<(), ActionArtifactCodecError> {
    if original == canonical {
        Ok(())
    } else {
        Err(ActionArtifactCodecError::NonCanonicalEncoding)
    }
}

fn unexpected_cbor(
    error: minicbor::decode::Error,
    fallback_position: usize,
    expected: &'static str,
) -> ActionArtifactCodecError {
    ActionArtifactCodecError::UnexpectedCbor {
        position: error.position().unwrap_or(fallback_position),
        expected,
    }
}

fn encode_sequence(encoder: &mut Encoder<Vec<u8>>, context: &'static str, length: usize) {
    encode_array(encoder, context, length as u64);
}

fn encode_array(encoder: &mut Encoder<Vec<u8>>, context: &'static str, length: u64) {
    encode_result(context, encoder.array(length));
}

fn encode_u16(encoder: &mut Encoder<Vec<u8>>, context: &'static str, value: u16) {
    encode_result(context, encoder.u16(value));
}

fn encode_u32(encoder: &mut Encoder<Vec<u8>>, context: &'static str, value: u32) {
    encode_result(context, encoder.u32(value));
}

fn encode_u64(encoder: &mut Encoder<Vec<u8>>, context: &'static str, value: u64) {
    encode_result(context, encoder.u64(value));
}

fn encode_bool(encoder: &mut Encoder<Vec<u8>>, context: &'static str, value: bool) {
    encode_result(context, encoder.bool(value));
}

fn encode_text(encoder: &mut Encoder<Vec<u8>>, context: &'static str, value: &str) {
    encode_result(context, encoder.str(value));
}

fn encode_bytes(encoder: &mut Encoder<Vec<u8>>, context: &'static str, value: &[u8]) {
    encode_result(context, encoder.bytes(value));
}

fn encode_result<T>(context: &'static str, result: Result<T, minicbor::encode::Error<Infallible>>) {
    result
        .unwrap_or_else(|_| unreachable!("Vec-backed CBOR encoder failed while writing {context}"));
}

/// Why bytes could not be accepted as one canonical action artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionArtifactCodecError {
    /// A CBOR item had the wrong primitive or was truncated.
    UnexpectedCbor {
        /// Exact input byte position.
        position: usize,
        /// Protocol value expected at that position.
        expected: &'static str,
    },
    /// A protocol array used CBOR's indefinite representation.
    IndefiniteArray {
        /// Exact input byte position.
        position: usize,
        /// Array being decoded.
        context: &'static str,
    },
    /// A fixed protocol array had the wrong arity.
    WrongArrayLength {
        /// Exact input byte position.
        position: usize,
        /// Array being decoded.
        context: &'static str,
        /// Required array length.
        expected: u64,
        /// Encoded array length.
        actual: u64,
    },
    /// A collection length cannot be represented on this host.
    CollectionTooLarge {
        /// Exact input byte position.
        position: usize,
        /// Collection being decoded.
        context: &'static str,
        /// Encoded collection length.
        actual: u64,
    },
    /// A schema version did not match the selected artifact schema.
    SchemaMismatch {
        /// Exact input byte position.
        position: usize,
        /// Schema being decoded.
        context: &'static str,
        /// Required version.
        expected: u16,
        /// Encoded version.
        actual: u16,
    },
    /// A closed enum carried an unknown tag.
    InvalidTag {
        /// Exact input byte position.
        position: usize,
        /// Closed enum being decoded.
        context: &'static str,
        /// Unknown tag.
        actual: u16,
    },
    /// A fixed-width identity had the wrong byte length.
    InvalidByteLength {
        /// Exact input byte position.
        position: usize,
        /// Identity being decoded.
        context: &'static str,
        /// Required byte length.
        expected: usize,
        /// Encoded byte length.
        actual: usize,
    },
    /// A checked domain name was not valid.
    InvalidName {
        /// Exact input byte position.
        position: usize,
        /// Name family being decoded.
        context: &'static str,
    },
    /// Decoded values violated a concrete action-domain invariant.
    InvalidValue {
        /// Invariant that was violated.
        context: &'static str,
    },
    /// A stored derived identity did not match the decoded body.
    IdentityMismatch {
        /// Derived identity being checked.
        context: &'static str,
    },
    /// Bytes remained after one complete root value.
    TrailingBytes {
        /// First trailing byte position.
        position: usize,
        /// Number of unconsumed bytes.
        remaining: usize,
    },
    /// The CBOR value had a valid meaning but used a noncanonical spelling.
    NonCanonicalEncoding,
    /// A decoded body could not enter the identity canonicalization protocol.
    Canonical(CanonicalError),
}

impl fmt::Display for ActionArtifactCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid action artifact: {self:?}")
    }
}

impl std::error::Error for ActionArtifactCodecError {}

#[cfg(test)]
mod tests {
    use world_core::ContentDigest;

    use super::*;

    fn actor(byte: u8) -> ActorId {
        ActorId::from_bytes([byte; 32])
    }

    fn entity(byte: u8) -> EntityId {
        EntityId::from_bytes([byte; 32])
    }

    fn object(byte: u8) -> ActorSafeObjectRef {
        ActorSafeObjectRef::from_bytes([byte; 32])
    }

    fn name(value: &str) -> BindingName {
        BindingName::parse(value)
            .unwrap_or_else(|error| panic!("binding fixture must be valid: {error}"))
    }

    fn definition() -> DefinitionKey {
        DefinitionKey::new(
            PackKey::parse("example.pack")
                .unwrap_or_else(|error| panic!("pack fixture must be valid: {error}")),
            LocalDefinitionName::parse("move-item")
                .unwrap_or_else(|error| panic!("definition fixture must be valid: {error}")),
        )
    }

    fn fixture() -> (
        ActionContextPayload,
        CandidateResolutionTable,
        ActionProjectionWitness,
        ActionExecutionWitness,
    ) {
        let actor = actor(0x11);
        let opportunity = ActionOpportunityId::from_bytes([0x21; 32]);
        let grounding_semantics = GroundingSemanticsId::from_bytes([0x31; 32]);
        let policy_semantics = ActionPolicySemanticsId::from_bytes([0x41; 32]);
        let source = object(0x51);
        let destination = object(0x52);
        let item = object(0x53);
        let bindings = vec![
            ActorSafeBinding::new(name(ACTOR_ROLE), ActorSafeBindingValue::Actor(actor)),
            ActorSafeBinding::new(
                name(DESTINATION_ROLE),
                ActorSafeBindingValue::Object(destination),
            ),
            ActorSafeBinding::new(name(ITEM_ROLE), ActorSafeBindingValue::Object(item)),
            ActorSafeBinding::new(name(SOURCE_ROLE), ActorSafeBindingValue::Object(source)),
        ];
        let action = definition();
        let candidate_id = derive_candidate_id(
            opportunity,
            &action,
            GroundedActionInteraction::ContainmentTransfer,
            &bindings,
            grounding_semantics,
        )
        .unwrap_or_else(|error| panic!("candidate fixture must be canonical: {error}"));
        let candidate = GroundedActionCandidate {
            id: candidate_id,
            opportunity,
            action: action.clone(),
            interaction: GroundedActionInteraction::ContainmentTransfer,
            bindings,
        };
        let candidates = vec![candidate];
        let candidate_fingerprint = candidate_set_fingerprint(
            opportunity,
            grounding_semantics,
            4,
            CandidateCoverage::Complete,
            &candidates,
        )
        .unwrap_or_else(|error| panic!("candidate-set fixture must be canonical: {error}"));
        let candidate_set = GroundedActionCandidateSet {
            opportunity,
            grounding_semantics,
            candidate_limit: 4,
            coverage: CandidateCoverage::Complete,
            candidates,
            fingerprint: candidate_fingerprint,
        };
        let interaction =
            ActorSafeActionInteraction::Containment(ActorSafeContainmentInteraction {
                source,
                destinations: vec![destination],
                items: vec![item],
            });
        let input_fingerprint = action_input_fingerprint(
            actor,
            opportunity,
            &interaction,
            &candidate_set,
            policy_semantics,
        )
        .unwrap_or_else(|error| panic!("payload fixture must be canonical: {error}"));
        let payload = ActionContextPayload {
            actor,
            opportunity,
            interaction,
            candidates: candidate_set,
            policy_semantics,
            input_fingerprint,
        };
        let resolution = CandidateResolutionTable {
            references: vec![
                PrivateObjectResolution {
                    actor_safe: source,
                    exact: entity(0x61),
                },
                PrivateObjectResolution {
                    actor_safe: destination,
                    exact: entity(0x62),
                },
                PrivateObjectResolution {
                    actor_safe: item,
                    exact: entity(0x63),
                },
            ],
            candidates: vec![PrivateCandidateResolution::Containment {
                candidate: candidate_id,
                action,
                actor,
                item,
                source,
                destination,
            }],
        };
        let witness = ActionProjectionWitness::Containment(ContainmentPolicyWitness {
            actor,
            observations: vec![
                ContainmentBeliefObservation {
                    item: entity(0x63),
                    believed_container: Some(entity(0x61)),
                },
                ContainmentBeliefObservation {
                    item: entity(0x64),
                    believed_container: None,
                },
            ],
        });
        let execution_witness =
            ActionExecutionWitness::Containment(vec![ContainmentCandidateExecutionWitness {
                candidate: candidate_id,
                item_container: Some(entity(0x61)),
                source_exists: true,
                actor_controls_source: true,
                destination_capacity: Some(4),
                destination_direct_item_count: 1,
            }]);
        (payload, resolution, witness, execution_witness)
    }

    fn read_witness(
        projection: ActionProjectionWitness,
        execution: ActionExecutionWitness,
    ) -> ActionReadWitness {
        ActionReadWitness {
            projection,
            execution,
        }
    }

    #[test]
    fn action_artifacts_round_trip_complete_domain_values() {
        let (payload, resolution, witness, execution_witness) = fixture();
        let read_witness = read_witness(witness.clone(), execution_witness.clone());

        let payload_bytes = encode_action_context_payload(&payload);
        let resolution_bytes = encode_candidate_resolution_table(&resolution);
        let witness_bytes = encode_action_projection_witness(&witness);
        let execution_witness_bytes = encode_action_execution_witness(&execution_witness);
        let read_witness_bytes = encode_action_read_witness(&read_witness);

        assert_eq!(
            decode_action_context_payload(&payload_bytes),
            Ok(payload.clone())
        );
        assert_eq!(
            decode_candidate_resolution_table(&resolution_bytes),
            Ok(resolution.clone())
        );
        assert_eq!(
            decode_action_projection_witness(&witness_bytes),
            Ok(witness.clone())
        );
        assert_eq!(
            decode_action_execution_witness(&execution_witness_bytes),
            Ok(execution_witness)
        );
        assert_eq!(
            decode_action_read_witness(&read_witness_bytes),
            Ok(read_witness)
        );
        assert_eq!(
            encode_action_context_payload(
                &decode_action_context_payload(&payload_bytes)
                    .unwrap_or_else(|error| panic!("round-trip decode must succeed: {error}")),
            ),
            payload_bytes
        );
    }

    #[test]
    fn action_artifact_schemas_and_canonical_bytes_are_frozen() {
        let (payload, resolution, witness, execution_witness) = fixture();
        let read_witness = read_witness(witness.clone(), execution_witness.clone());
        let payload_bytes = encode_action_context_payload(&payload);
        let resolution_bytes = encode_candidate_resolution_table(&resolution);
        let witness_bytes = encode_action_projection_witness(&witness);
        let execution_witness_bytes = encode_action_execution_witness(&execution_witness);
        let read_witness_bytes = encode_action_read_witness(&read_witness);

        assert_eq!(
            (
                action_context_payload_schema().to_string(),
                candidate_resolution_table_schema().to_string(),
                action_projection_witness_schema().to_string(),
                payload_bytes.len(),
                ContentDigest::of_blob_bytes(&payload_bytes).to_string(),
                resolution_bytes.len(),
                ContentDigest::of_blob_bytes(&resolution_bytes).to_string(),
                witness_bytes.len(),
                ContentDigest::of_blob_bytes(&witness_bytes).to_string(),
                action_execution_witness_schema().to_string(),
                execution_witness_bytes.len(),
                ContentDigest::of_blob_bytes(&execution_witness_bytes).to_string(),
            ),
            (
                "9bcc8344c4339868da12c4840de279cc3b1d78c4ad50dd16a73eb851903e67d4".to_owned(),
                "dcde2e57df583cf0249f14673e831addedcca916e481d1a867cfb5ec2010503a".to_owned(),
                "6c280865243a34d32fd9889abbb414bcc7c3deec28d1b67c258ad3d49c83c9ce".to_owned(),
                624,
                "51035a6f1b7fc2dcbb94cc2b80edcf1c316fe890dc38c48e85f1e016afb730a7".to_owned(),
                407,
                "1b81741fc7addf618a8f810b8afe54c85967d7819ff4356db2fffd4fe0b20ee7".to_owned(),
                146,
                "53c111f43619ba3bbfca843d7131222d39ff3b72cdd4b11d64e101cf91aaede3".to_owned(),
                "3697fb44957289975b388d02a2bc41a47c4054e4aa26b283afb45512e0395a89".to_owned(),
                81,
                "ae0e5be0cf7ea53f0bbf4fb5c808c9c68641738e25415fff8e3c4c02d335a488".to_owned(),
            )
        );
        assert_eq!(
            (
                action_read_witness_schema().to_string(),
                read_witness_bytes.len(),
                ContentDigest::of_blob_bytes(&read_witness_bytes).to_string(),
            ),
            (
                "6f2b19ea59c0ef6465c44025c8c7b0ebcaac32c340fa9640cbb6d97b8af83b3d".to_owned(),
                233,
                "71431caf8d0adc97ce7da91bc05b4bb3726ee3188dabeb26492504383dc04ccb".to_owned(),
            )
        );
    }

    #[test]
    fn payload_decoder_rejects_structure_names_and_noncanonical_spelling() {
        let (payload, _, _, _) = fixture();
        let encoded = encode_action_context_payload(&payload);

        let mut truncated = encoded.clone();
        truncated.pop();
        assert!(matches!(
            decode_action_context_payload(&truncated),
            Err(ActionArtifactCodecError::UnexpectedCbor { .. })
        ));

        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(matches!(
            decode_action_context_payload(&trailing),
            Err(ActionArtifactCodecError::TrailingBytes { .. })
        ));

        let mut wrong_arity = encoded.clone();
        wrong_arity[0] = 0x86;
        assert!(matches!(
            decode_action_context_payload(&wrong_arity),
            Err(ActionArtifactCodecError::WrongArrayLength { .. })
        ));

        let mut invalid_tag = encoded.clone();
        let mut decoder = Decoder::new(&invalid_tag);
        decoder
            .array()
            .unwrap_or_else(|error| panic!("root array must decode: {error}"));
        decoder
            .u16()
            .unwrap_or_else(|error| panic!("schema must decode: {error}"));
        decoder
            .bytes()
            .unwrap_or_else(|error| panic!("actor must decode: {error}"));
        decoder
            .bytes()
            .unwrap_or_else(|error| panic!("opportunity must decode: {error}"));
        decoder
            .array()
            .unwrap_or_else(|error| panic!("interaction must decode: {error}"));
        let tag_position = decoder.position();
        invalid_tag[tag_position] = 9;
        assert!(matches!(
            decode_action_context_payload(&invalid_tag),
            Err(ActionArtifactCodecError::InvalidTag {
                context: "actor-safe interaction kind",
                ..
            })
        ));

        let mut invalid_name = encoded.clone();
        let offset = invalid_name
            .windows(b"example.pack".len())
            .position(|window| window == b"example.pack")
            .unwrap_or_else(|| panic!("encoded fixture must contain its pack key"));
        invalid_name[offset] = b'E';
        assert!(matches!(
            decode_action_context_payload(&invalid_name),
            Err(ActionArtifactCodecError::InvalidName {
                context: "pack key",
                ..
            })
        ));

        let mut noncanonical = Vec::with_capacity(encoded.len() + 1);
        noncanonical.push(encoded[0]);
        noncanonical.extend_from_slice(&[0x18, 0x01]);
        noncanonical.extend_from_slice(&encoded[2..]);
        assert_eq!(
            decode_action_context_payload(&noncanonical),
            Err(ActionArtifactCodecError::NonCanonicalEncoding)
        );
    }

    #[test]
    fn payload_decoder_recomputes_candidate_set_and_input_identities() {
        let (mut payload, _, _, _) = fixture();
        payload.candidates.candidates[0].id = GroundedActionCandidateId::from_bytes([0xff; 32]);
        let forged_candidate = encode_action_context_payload(&payload);
        assert_eq!(
            decode_action_context_payload(&forged_candidate),
            Err(ActionArtifactCodecError::IdentityMismatch {
                context: "grounded action candidate",
            })
        );

        let (mut payload, _, _, _) = fixture();
        payload.candidates.fingerprint = GroundedCandidateSetFingerprint::from_bytes([0xfe; 32]);
        let forged_set = encode_action_context_payload(&payload);
        assert_eq!(
            decode_action_context_payload(&forged_set),
            Err(ActionArtifactCodecError::IdentityMismatch {
                context: "grounded candidate set",
            })
        );

        let (mut payload, _, _, _) = fixture();
        payload.input_fingerprint = ActionInputFingerprint::from_bytes([0xfd; 32]);
        let forged_input = encode_action_context_payload(&payload);
        assert_eq!(
            decode_action_context_payload(&forged_input),
            Err(ActionArtifactCodecError::IdentityMismatch {
                context: "action context payload",
            })
        );
    }

    #[test]
    fn every_action_artifact_decoder_rejects_bad_framing_and_closed_tags() {
        fn assert_bad_framing<T>(
            encoded: &[u8],
            decode: impl Fn(&[u8]) -> Result<T, ActionArtifactCodecError>,
        ) {
            let mut truncated = encoded.to_vec();
            truncated.pop();
            assert!(matches!(
                decode(&truncated),
                Err(ActionArtifactCodecError::UnexpectedCbor { .. })
            ));

            let mut trailing = encoded.to_vec();
            trailing.push(0);
            assert!(matches!(
                decode(&trailing),
                Err(ActionArtifactCodecError::TrailingBytes { .. })
            ));
        }

        let (payload, resolution, projection, execution) = fixture();
        let read = read_witness(projection.clone(), execution.clone());
        let payload = encode_action_context_payload(&payload);
        let resolution = encode_candidate_resolution_table(&resolution);
        let projection = encode_action_projection_witness(&projection);
        let execution = encode_action_execution_witness(&execution);
        let read = encode_action_read_witness(&read);
        assert_bad_framing(&payload, decode_action_context_payload);
        assert_bad_framing(&resolution, decode_candidate_resolution_table);
        assert_bad_framing(&projection, decode_action_projection_witness);
        assert_bad_framing(&execution, decode_action_execution_witness);
        assert_bad_framing(&read, decode_action_read_witness);

        let mut invalid_read_schema = read;
        invalid_read_schema[1] = 9;
        assert!(matches!(
            decode_action_read_witness(&invalid_read_schema),
            Err(ActionArtifactCodecError::SchemaMismatch {
                context: "action read witness",
                ..
            })
        ));

        let mut invalid_projection_tag = projection;
        let mut decoder = Decoder::new(&invalid_projection_tag);
        decoder
            .array()
            .unwrap_or_else(|error| panic!("projection root must decode: {error}"));
        decoder
            .u16()
            .unwrap_or_else(|error| panic!("projection schema must decode: {error}"));
        let projection_tag = decoder.position();
        invalid_projection_tag[projection_tag] = 9;
        assert!(matches!(
            decode_action_projection_witness(&invalid_projection_tag),
            Err(ActionArtifactCodecError::InvalidTag {
                context: "projection witness kind",
                ..
            })
        ));

        let mut invalid_execution_tag = execution;
        let mut decoder = Decoder::new(&invalid_execution_tag);
        decoder
            .array()
            .unwrap_or_else(|error| panic!("execution root must decode: {error}"));
        decoder
            .u16()
            .unwrap_or_else(|error| panic!("execution schema must decode: {error}"));
        let execution_tag = decoder.position();
        invalid_execution_tag[execution_tag] = 9;
        assert!(matches!(
            decode_action_execution_witness(&invalid_execution_tag),
            Err(ActionArtifactCodecError::InvalidTag {
                context: "execution witness kind",
                ..
            })
        ));
    }

    #[test]
    fn private_artifact_decoders_restore_owner_invariants() {
        let (_, mut resolution, mut projection, mut execution) = fixture();

        resolution.references.swap(0, 1);
        assert_eq!(
            decode_candidate_resolution_table(&encode_candidate_resolution_table(&resolution)),
            Err(ActionArtifactCodecError::InvalidValue {
                context: "private object references",
            })
        );

        let ActionProjectionWitness::Containment(witness) = &mut projection else {
            panic!("fixture must use containment projection")
        };
        witness.observations.swap(0, 1);
        assert_eq!(
            decode_action_projection_witness(&encode_action_projection_witness(&projection)),
            Err(ActionArtifactCodecError::InvalidValue {
                context: "containment belief observations",
            })
        );

        let ActionExecutionWitness::Containment(observations) = &mut execution else {
            panic!("fixture must use containment execution")
        };
        observations.push(observations[0]);
        assert_eq!(
            decode_action_execution_witness(&encode_action_execution_witness(&execution)),
            Err(ActionArtifactCodecError::InvalidValue {
                context: "duplicate execution candidate observation",
            })
        );

        let mismatched = read_witness(
            ActionProjectionWitness::RelocationNoRead,
            ActionExecutionWitness::Containment(Vec::new()),
        );
        assert_eq!(
            decode_action_read_witness(&encode_action_read_witness(&mismatched)),
            Err(ActionArtifactCodecError::InvalidValue {
                context: "action read witness family",
            })
        );
    }
}
