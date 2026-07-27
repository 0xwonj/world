use core::{convert::Infallible, fmt};

use minicbor::{Decoder, Encoder};

use crate::definition::{
    ActionBindingData, ActionData, EffectCallData, EventData, EventEmissionData,
    EventFieldBindingData, EventFieldData, OperationCallData, RuntimeRequirementData,
};
use crate::interface::{
    OperationName, SemanticInterfaceDigest, SemanticInterfaceKey, SemanticInterfaceReference,
    ValueKind,
};
use crate::key::{
    BindingName, DefinitionKey, EngineProtocolVersion, EventFieldName, InterfaceVersion, KeyError,
    LocalDefinitionName, PackCoordinate, PackKey, PackVersion,
};

use super::{
    ARTIFACT_FORMAT_VERSION, ArtifactData, CheckedArtifactData, DEFINITION_FAMILY_SCHEMA_VERSION,
    DefinitionRef, MANIFEST_SCHEMA_VERSION, MAX_ACTION_BINDINGS, MAX_DEFINITIONS_PER_ARTIFACT,
    MAX_DIRECT_DEPENDENCIES, MAX_EFFECTS_PER_ACTION, MAX_EVENT_FIELDS, MAX_OPERATION_ARGUMENTS,
    MAX_REQUIRED_INTERFACES, MAX_REQUIREMENTS_PER_ACTION, MAX_SUCCESS_EVENTS_PER_ACTION,
    PackDependency, PackExportDigest, PackManifestData, ordered_definitions,
};

const ARTIFACT_ROOT_ARITY: u64 = 4;
const MANIFEST_ARITY: u64 = 4;
const COORDINATE_ARITY: u64 = 2;
const PACK_VERSION_ARITY: u64 = 3;
const DEPENDENCY_ARITY: u64 = 2;
const INTERFACE_REFERENCE_ARITY: u64 = 3;
const ACTION_ARITY: u64 = 7;
const EVENT_ARITY: u64 = 4;
const BINDING_ARITY: u64 = 2;
const OPERATION_CALL_ARITY: u64 = 3;
const EVENT_EMISSION_ARITY: u64 = 2;
const DEFINITION_KEY_ARITY: u64 = 2;
const EVENT_FIELD_MAPPING_ARITY: u64 = 2;
const EVENT_FIELD_ARITY: u64 = 2;

const ACTION_TAG: u16 = 0;
const EVENT_TAG: u16 = 1;
const ACTOR_VALUE_TAG: u16 = 0;
const ENTITY_VALUE_TAG: u16 = 1;

/// Encodes normalized, checked artifact data as deterministic ArtifactBlobV1
/// bytes.
pub(super) fn encode(data: &CheckedArtifactData) -> Vec<u8> {
    let mut encoder = Encoder::new(Vec::new());
    encode_array(&mut encoder, "artifact root", ARTIFACT_ROOT_ARITY);
    encode_u16(&mut encoder, "artifact schema", ARTIFACT_FORMAT_VERSION);
    encode_manifest(&mut encoder, data.manifest());
    encode_interfaces(&mut encoder, data.interfaces());
    encode_definitions(&mut encoder, data);
    encoder.into_writer()
}

/// Decodes one complete ArtifactBlobV1 value into unchecked aggregate data.
pub(super) fn decode(bytes: &[u8]) -> Result<ArtifactData, CodecError> {
    let mut decoder = Decoder::new(bytes);
    expect_array(&mut decoder, "artifact root", ARTIFACT_ROOT_ARITY)?;
    decode_schema(&mut decoder, "artifact", ARTIFACT_FORMAT_VERSION)?;
    let manifest = decode_manifest(&mut decoder)?;
    let interfaces = decode_interfaces(&mut decoder)?;
    let (actions, events) = decode_definitions(&mut decoder, &interfaces)?;

    if decoder.position() != bytes.len() {
        return Err(CodecError::TrailingBytes {
            position: decoder.position(),
            remaining: bytes.len() - decoder.position(),
        });
    }

    Ok(ArtifactData::new(manifest, interfaces, actions, events))
}

fn encode_manifest(encoder: &mut Encoder<Vec<u8>>, manifest: &PackManifestData) {
    encode_array(encoder, "pack manifest", MANIFEST_ARITY);
    encode_u16(encoder, "manifest schema", MANIFEST_SCHEMA_VERSION);
    encode_u16(encoder, "engine protocol", manifest.engine_protocol().get());
    encode_coordinate(encoder, manifest.coordinate());

    encode_sequence_start(
        encoder,
        "direct dependencies",
        manifest.dependencies().len(),
    );
    for dependency in manifest.dependencies() {
        encode_array(encoder, "dependency reference", DEPENDENCY_ARITY);
        encode_coordinate(encoder, dependency.coordinate());
        encode_bytes(
            encoder,
            "dependency export digest",
            dependency.expected_export_digest().as_bytes(),
        );
    }
}

fn encode_coordinate(encoder: &mut Encoder<Vec<u8>>, coordinate: &PackCoordinate) {
    encode_array(encoder, "pack coordinate", COORDINATE_ARITY);
    encode_text(encoder, "pack key", coordinate.pack_key().as_str());
    encode_array(encoder, "pack version", PACK_VERSION_ARITY);
    encode_u32(encoder, "pack major version", coordinate.version().major());
    encode_u32(encoder, "pack minor version", coordinate.version().minor());
    encode_u32(encoder, "pack patch version", coordinate.version().patch());
}

fn encode_interfaces(encoder: &mut Encoder<Vec<u8>>, interfaces: &[SemanticInterfaceReference]) {
    encode_sequence_start(encoder, "interface references", interfaces.len());
    for reference in interfaces {
        encode_array(
            encoder,
            "semantic-interface reference",
            INTERFACE_REFERENCE_ARITY,
        );
        encode_text(encoder, "semantic-interface key", reference.key().as_str());
        encode_u16(
            encoder,
            "semantic-interface version",
            reference.version().get(),
        );
        encode_bytes(
            encoder,
            "semantic-interface digest",
            reference.digest().as_bytes(),
        );
    }
}

fn encode_definitions(encoder: &mut Encoder<Vec<u8>>, data: &CheckedArtifactData) {
    let definitions = ordered_definitions(data);
    encode_sequence_start(encoder, "definitions", definitions.len());
    for definition in definitions {
        match definition {
            DefinitionRef::Action(action) => {
                encode_array(encoder, "action definition", ACTION_ARITY);
                encode_u16(encoder, "action family tag", ACTION_TAG);
                encode_u16(
                    encoder,
                    "action family schema",
                    DEFINITION_FAMILY_SCHEMA_VERSION,
                );
                encode_text(encoder, "action name", action.name().as_str());

                encode_sequence_start(encoder, "action bindings", action.bindings().len());
                for binding in action.bindings() {
                    encode_array(encoder, "action binding", BINDING_ARITY);
                    encode_text(encoder, "binding name", binding.name().as_str());
                    encode_value_kind(encoder, *binding.value_kind());
                }

                encode_sequence_start(encoder, "runtime requirements", action.requirements().len());
                for requirement in action.requirements() {
                    encode_operation_call(encoder, requirement.call(), data.interfaces());
                }

                encode_sequence_start(encoder, "effect calls", action.effects().len());
                for effect in action.effects() {
                    encode_operation_call(encoder, effect.call(), data.interfaces());
                }

                encode_sequence_start(encoder, "success events", action.success_events().len());
                for emission in action.success_events() {
                    encode_event_emission(encoder, emission);
                }
            }
            DefinitionRef::Event(event) => {
                encode_array(encoder, "event definition", EVENT_ARITY);
                encode_u16(encoder, "event family tag", EVENT_TAG);
                encode_u16(
                    encoder,
                    "event family schema",
                    DEFINITION_FAMILY_SCHEMA_VERSION,
                );
                encode_text(encoder, "event name", event.name().as_str());
                encode_sequence_start(encoder, "event fields", event.fields().len());
                for field in event.fields() {
                    encode_array(encoder, "event field", EVENT_FIELD_ARITY);
                    encode_text(encoder, "event field name", field.name().as_str());
                    encode_value_kind(encoder, *field.value_kind());
                }
            }
        }
    }
}

fn encode_operation_call(
    encoder: &mut Encoder<Vec<u8>>,
    call: &OperationCallData,
    interfaces: &[SemanticInterfaceReference],
) {
    encode_array(encoder, "operation call", OPERATION_CALL_ARITY);
    let slot = match interfaces.binary_search_by(|reference| reference.key().cmp(call.interface()))
    {
        Ok(slot) => slot as u16,
        Err(_) => unreachable!("checked operation call must have an interface slot"),
    };
    encode_u16(encoder, "semantic-interface slot", slot);
    encode_text(encoder, "operation name", call.operation().as_str());
    encode_sequence_start(encoder, "operation arguments", call.arguments().len());
    for argument in call.arguments() {
        encode_text(encoder, "operation binding argument", argument.as_str());
    }
}

fn encode_event_emission(encoder: &mut Encoder<Vec<u8>>, emission: &EventEmissionData) {
    encode_array(encoder, "event emission", EVENT_EMISSION_ARITY);
    encode_definition_key(encoder, emission.event());
    encode_sequence_start(
        encoder,
        "event field mappings",
        emission.field_bindings().len(),
    );
    for mapping in emission.field_bindings() {
        encode_array(encoder, "event field mapping", EVENT_FIELD_MAPPING_ARITY);
        encode_text(encoder, "event field name", mapping.field().as_str());
        encode_text(encoder, "event binding name", mapping.binding().as_str());
    }
}

fn encode_definition_key(encoder: &mut Encoder<Vec<u8>>, key: &DefinitionKey) {
    encode_array(encoder, "definition key", DEFINITION_KEY_ARITY);
    encode_text(encoder, "definition pack key", key.pack_key().as_str());
    encode_text(encoder, "definition local name", key.local_name().as_str());
}

fn encode_value_kind(encoder: &mut Encoder<Vec<u8>>, kind: ValueKind) {
    let tag = match kind {
        ValueKind::Actor => ACTOR_VALUE_TAG,
        ValueKind::Entity => ENTITY_VALUE_TAG,
    };
    encode_u16(encoder, "value-kind tag", tag);
}

fn decode_manifest(decoder: &mut Decoder<'_>) -> Result<PackManifestData, CodecError> {
    expect_array(decoder, "pack manifest", MANIFEST_ARITY)?;
    decode_schema(decoder, "manifest", MANIFEST_SCHEMA_VERSION)?;
    let engine_protocol = EngineProtocolVersion::new(decode_u16(decoder, "engine protocol")?);
    let coordinate = decode_coordinate(decoder)?;

    let dependency_count =
        decode_sequence_length(decoder, "direct dependencies", MAX_DIRECT_DEPENDENCIES)?;
    let mut dependencies = Vec::with_capacity(dependency_count);
    for _ in 0..dependency_count {
        expect_array(decoder, "dependency reference", DEPENDENCY_ARITY)?;
        let coordinate = decode_coordinate(decoder)?;
        let digest = decode_digest(decoder, "dependency export digest")?;
        dependencies.push(PackDependency::new(
            coordinate,
            PackExportDigest::from_bytes(digest),
        ));
    }

    Ok(PackManifestData::new(
        engine_protocol,
        coordinate,
        dependencies,
    ))
}

fn decode_coordinate(decoder: &mut Decoder<'_>) -> Result<PackCoordinate, CodecError> {
    expect_array(decoder, "pack coordinate", COORDINATE_ARITY)?;
    let pack_key = decode_checked_name(decoder, "pack key", PackKey::parse)?;
    expect_array(decoder, "pack version", PACK_VERSION_ARITY)?;
    let major = decode_u32(decoder, "pack major version")?;
    let minor = decode_u32(decoder, "pack minor version")?;
    let patch = decode_u32(decoder, "pack patch version")?;
    Ok(PackCoordinate::new(
        pack_key,
        PackVersion::new(major, minor, patch),
    ))
}

fn decode_interfaces(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<SemanticInterfaceReference>, CodecError> {
    let count = decode_sequence_length(decoder, "interface references", MAX_REQUIRED_INTERFACES)?;
    let mut interfaces = Vec::with_capacity(count);
    for _ in 0..count {
        expect_array(
            decoder,
            "semantic-interface reference",
            INTERFACE_REFERENCE_ARITY,
        )?;
        let key = decode_checked_name(
            decoder,
            "semantic-interface key",
            SemanticInterfaceKey::parse,
        )?;
        let version_position = decoder.position();
        let version_value = decode_u16(decoder, "semantic-interface version")?;
        let version =
            InterfaceVersion::new(version_value).map_err(|source| CodecError::InvalidKey {
                position: version_position,
                context: "semantic-interface version",
                source,
            })?;
        let digest = SemanticInterfaceDigest::from_bytes(decode_digest(
            decoder,
            "semantic-interface digest",
        )?);
        interfaces.push(SemanticInterfaceReference::new(key, version, digest));
    }
    Ok(interfaces)
}

fn decode_definitions(
    decoder: &mut Decoder<'_>,
    interfaces: &[SemanticInterfaceReference],
) -> Result<(Vec<ActionData>, Vec<EventData>), CodecError> {
    let count = decode_sequence_length(decoder, "definitions", MAX_DEFINITIONS_PER_ARTIFACT)?;
    let mut actions = Vec::new();
    let mut events = Vec::new();

    for _ in 0..count {
        let definition_position = decoder.position();
        let arity = decode_definite_array(decoder, "definition")?;
        let tag_position = decoder.position();
        let tag = decode_u16(decoder, "definition-family tag")?;
        match tag {
            ACTION_TAG => {
                require_array_length(
                    definition_position,
                    "action definition",
                    ACTION_ARITY,
                    arity,
                )?;
                decode_schema(decoder, "action family", DEFINITION_FAMILY_SCHEMA_VERSION)?;
                actions.push(decode_action(decoder, interfaces)?);
            }
            EVENT_TAG => {
                require_array_length(definition_position, "event definition", EVENT_ARITY, arity)?;
                decode_schema(decoder, "event family", DEFINITION_FAMILY_SCHEMA_VERSION)?;
                events.push(decode_event(decoder)?);
            }
            actual => {
                return Err(CodecError::UnknownDefinitionTag {
                    position: tag_position,
                    actual,
                });
            }
        }
    }

    Ok((actions, events))
}

fn decode_action(
    decoder: &mut Decoder<'_>,
    interfaces: &[SemanticInterfaceReference],
) -> Result<ActionData, CodecError> {
    let name = decode_checked_name(decoder, "action name", LocalDefinitionName::parse)?;

    let binding_count = decode_sequence_length(decoder, "action bindings", MAX_ACTION_BINDINGS)?;
    let mut bindings = Vec::with_capacity(binding_count);
    for _ in 0..binding_count {
        expect_array(decoder, "action binding", BINDING_ARITY)?;
        let name = decode_checked_name(decoder, "binding name", BindingName::parse)?;
        let value_kind = decode_value_kind(decoder)?;
        bindings.push(ActionBindingData::new(name, value_kind));
    }

    let requirement_count =
        decode_sequence_length(decoder, "runtime requirements", MAX_REQUIREMENTS_PER_ACTION)?;
    let mut requirements = Vec::with_capacity(requirement_count);
    for _ in 0..requirement_count {
        let call = decode_operation_call(decoder, interfaces)?;
        requirements.push(RuntimeRequirementData::new(call));
    }

    let effect_count = decode_sequence_length(decoder, "effect calls", MAX_EFFECTS_PER_ACTION)?;
    let mut effects = Vec::with_capacity(effect_count);
    for _ in 0..effect_count {
        let call = decode_operation_call(decoder, interfaces)?;
        effects.push(EffectCallData::new(call));
    }

    let success_count =
        decode_sequence_length(decoder, "success events", MAX_SUCCESS_EVENTS_PER_ACTION)?;
    let mut success_events = Vec::with_capacity(success_count);
    for _ in 0..success_count {
        success_events.push(decode_event_emission(decoder)?);
    }

    Ok(ActionData::new(
        name,
        bindings,
        requirements,
        effects,
        success_events,
    ))
}

fn decode_event(decoder: &mut Decoder<'_>) -> Result<EventData, CodecError> {
    let name = decode_checked_name(decoder, "event name", LocalDefinitionName::parse)?;
    let field_count = decode_sequence_length(decoder, "event fields", MAX_EVENT_FIELDS)?;
    let mut fields = Vec::with_capacity(field_count);
    for _ in 0..field_count {
        expect_array(decoder, "event field", EVENT_FIELD_ARITY)?;
        let name = decode_checked_name(decoder, "event field name", EventFieldName::parse)?;
        let value_kind = decode_value_kind(decoder)?;
        fields.push(EventFieldData::new(name, value_kind));
    }
    Ok(EventData::new(name, fields))
}

fn decode_operation_call(
    decoder: &mut Decoder<'_>,
    interfaces: &[SemanticInterfaceReference],
) -> Result<OperationCallData, CodecError> {
    expect_array(decoder, "operation call", OPERATION_CALL_ARITY)?;
    let slot_position = decoder.position();
    let slot = decode_u16(decoder, "semantic-interface slot")?;
    let Some(reference) = interfaces.get(usize::from(slot)) else {
        return Err(CodecError::InterfaceSlotOutOfRange {
            position: slot_position,
            slot,
            available: interfaces.len(),
        });
    };
    let operation = decode_checked_name(decoder, "operation name", OperationName::parse)?;
    let argument_count =
        decode_sequence_length(decoder, "operation arguments", MAX_OPERATION_ARGUMENTS)?;
    let mut arguments = Vec::with_capacity(argument_count);
    for _ in 0..argument_count {
        arguments.push(decode_checked_name(
            decoder,
            "operation binding argument",
            BindingName::parse,
        )?);
    }
    Ok(OperationCallData::new(
        reference.key().clone(),
        operation,
        arguments,
    ))
}

fn decode_event_emission(decoder: &mut Decoder<'_>) -> Result<EventEmissionData, CodecError> {
    expect_array(decoder, "event emission", EVENT_EMISSION_ARITY)?;
    let event = decode_definition_key(decoder)?;
    let mapping_count = decode_sequence_length(decoder, "event field mappings", MAX_EVENT_FIELDS)?;
    let mut mappings = Vec::with_capacity(mapping_count);
    for _ in 0..mapping_count {
        expect_array(decoder, "event field mapping", EVENT_FIELD_MAPPING_ARITY)?;
        let field = decode_checked_name(decoder, "event field name", EventFieldName::parse)?;
        let binding = decode_checked_name(decoder, "event binding name", BindingName::parse)?;
        mappings.push(EventFieldBindingData::new(field, binding));
    }
    Ok(EventEmissionData::new(event, mappings))
}

fn decode_definition_key(decoder: &mut Decoder<'_>) -> Result<DefinitionKey, CodecError> {
    expect_array(decoder, "definition key", DEFINITION_KEY_ARITY)?;
    let pack_key = decode_checked_name(decoder, "definition pack key", PackKey::parse)?;
    let local_name =
        decode_checked_name(decoder, "definition local name", LocalDefinitionName::parse)?;
    Ok(DefinitionKey::new(pack_key, local_name))
}

fn decode_value_kind(decoder: &mut Decoder<'_>) -> Result<ValueKind, CodecError> {
    let position = decoder.position();
    let tag = decode_u16(decoder, "value-kind tag")?;
    match tag {
        ACTOR_VALUE_TAG => Ok(ValueKind::Actor),
        ENTITY_VALUE_TAG => Ok(ValueKind::Entity),
        actual => Err(CodecError::UnknownValueKind { position, actual }),
    }
}

fn decode_schema(
    decoder: &mut Decoder<'_>,
    schema: &'static str,
    expected: u16,
) -> Result<(), CodecError> {
    let position = decoder.position();
    let actual = decode_u16(decoder, "schema version")?;
    if actual != expected {
        return Err(CodecError::UnsupportedSchema {
            position,
            schema,
            expected,
            actual,
        });
    }
    Ok(())
}

fn decode_checked_name<T>(
    decoder: &mut Decoder<'_>,
    context: &'static str,
    parse: impl FnOnce(&str) -> Result<T, KeyError>,
) -> Result<T, CodecError> {
    let position = decoder.position();
    let value = decode_text(decoder, context)?;
    parse(value).map_err(|source| CodecError::InvalidKey {
        position,
        context,
        source,
    })
}

fn decode_digest(decoder: &mut Decoder<'_>, context: &'static str) -> Result<[u8; 32], CodecError> {
    let position = decoder.position();
    let bytes = decode_bytes(decoder, context)?;
    if bytes.len() != 32 {
        return Err(CodecError::InvalidDigestLength {
            position,
            context,
            actual: bytes.len(),
        });
    }
    let mut digest = [0; 32];
    digest.copy_from_slice(bytes);
    Ok(digest)
}

fn expect_array(
    decoder: &mut Decoder<'_>,
    context: &'static str,
    expected: u64,
) -> Result<(), CodecError> {
    let position = decoder.position();
    let actual = decode_definite_array(decoder, context)?;
    require_array_length(position, context, expected, actual)
}

fn require_array_length(
    position: usize,
    context: &'static str,
    expected: u64,
    actual: u64,
) -> Result<(), CodecError> {
    if actual != expected {
        return Err(CodecError::WrongArrayLength {
            position,
            context,
            expected,
            actual,
        });
    }
    Ok(())
}

fn decode_sequence_length(
    decoder: &mut Decoder<'_>,
    collection: &'static str,
    maximum: usize,
) -> Result<usize, CodecError> {
    let position = decoder.position();
    let actual = decode_definite_array(decoder, collection)?;
    if actual > maximum as u64 {
        return Err(CodecError::CollectionLimit {
            position,
            collection,
            actual,
            maximum,
        });
    }
    usize::try_from(actual).map_err(|_| CodecError::CollectionLimit {
        position,
        collection,
        actual,
        maximum,
    })
}

fn decode_definite_array(
    decoder: &mut Decoder<'_>,
    context: &'static str,
) -> Result<u64, CodecError> {
    let position = decoder.position();
    match decoder.array() {
        Ok(Some(length)) => Ok(length),
        Ok(None) => Err(CodecError::IndefiniteArray { position, context }),
        Err(error) => Err(unexpected_cbor(error, position, context)),
    }
}

fn decode_u16(decoder: &mut Decoder<'_>, expected: &'static str) -> Result<u16, CodecError> {
    let position = decoder.position();
    decoder
        .u16()
        .map_err(|error| unexpected_cbor(error, position, expected))
}

fn decode_u32(decoder: &mut Decoder<'_>, expected: &'static str) -> Result<u32, CodecError> {
    let position = decoder.position();
    decoder
        .u32()
        .map_err(|error| unexpected_cbor(error, position, expected))
}

fn decode_text<'bytes>(
    decoder: &mut Decoder<'bytes>,
    expected: &'static str,
) -> Result<&'bytes str, CodecError> {
    let position = decoder.position();
    decoder
        .str()
        .map_err(|error| unexpected_cbor(error, position, expected))
}

fn decode_bytes<'bytes>(
    decoder: &mut Decoder<'bytes>,
    expected: &'static str,
) -> Result<&'bytes [u8], CodecError> {
    let position = decoder.position();
    decoder
        .bytes()
        .map_err(|error| unexpected_cbor(error, position, expected))
}

fn unexpected_cbor(
    error: minicbor::decode::Error,
    fallback_position: usize,
    expected: &'static str,
) -> CodecError {
    let position = match error.position() {
        Some(position) => position,
        None => fallback_position,
    };
    CodecError::UnexpectedCbor { position, expected }
}

fn encode_sequence_start(encoder: &mut Encoder<Vec<u8>>, context: &'static str, length: usize) {
    encode_array(encoder, context, length as u64)
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

/// Why exact bytes could not be decoded as ArtifactBlobV1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodecError {
    /// A CBOR item had the wrong primitive or structural type.
    UnexpectedCbor {
        /// Exact input byte position.
        position: usize,
        /// Protocol value expected at that position.
        expected: &'static str,
    },
    /// A schema array used CBOR's indefinite-length representation.
    IndefiniteArray {
        /// Exact input byte position.
        position: usize,
        /// Array being decoded.
        context: &'static str,
    },
    /// A structural array did not have its schema-selected arity.
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
    /// A collection exceeded its semantic protocol limit.
    CollectionLimit {
        /// Exact input byte position.
        position: usize,
        /// Collection being decoded.
        collection: &'static str,
        /// Encoded collection length.
        actual: u64,
        /// Maximum accepted collection length.
        maximum: usize,
    },
    /// A checked identifier or version rejected decoded text or digits.
    InvalidKey {
        /// Exact input byte position.
        position: usize,
        /// Identifier or version being decoded.
        context: &'static str,
        /// Leaf validation failure.
        source: KeyError,
    },
    /// A descriptor digest was not exactly 32 bytes.
    InvalidDigestLength {
        /// Exact input byte position.
        position: usize,
        /// Digest being decoded.
        context: &'static str,
        /// Encoded byte-string length.
        actual: usize,
    },
    /// An artifact, manifest, or definition-family schema is unsupported.
    UnsupportedSchema {
        /// Exact input byte position.
        position: usize,
        /// Schema family.
        schema: &'static str,
        /// Supported version.
        expected: u16,
        /// Encoded version.
        actual: u16,
    },
    /// A definition family tag is unknown.
    UnknownDefinitionTag {
        /// Exact input byte position.
        position: usize,
        /// Encoded tag.
        actual: u16,
    },
    /// A value-kind tag is unknown.
    UnknownValueKind {
        /// Exact input byte position.
        position: usize,
        /// Encoded tag.
        actual: u16,
    },
    /// An operation call references no decoded interface-table entry.
    InterfaceSlotOutOfRange {
        /// Exact input byte position.
        position: usize,
        /// Encoded interface slot.
        slot: u16,
        /// Number of available table entries.
        available: usize,
    },
    /// Bytes remain after the one complete root value.
    TrailingBytes {
        /// First trailing byte.
        position: usize,
        /// Number of trailing bytes.
        remaining: usize,
    },
}

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCbor { position, expected } => {
                write!(formatter, "expected {expected} at byte {position}")
            }
            Self::IndefiniteArray { position, context } => write!(
                formatter,
                "{context} uses an indefinite array at byte {position}"
            ),
            Self::WrongArrayLength {
                position,
                context,
                expected,
                actual,
            } => write!(
                formatter,
                "{context} has array length {actual} at byte {position}; expected {expected}"
            ),
            Self::CollectionLimit {
                position,
                collection,
                actual,
                maximum,
            } => write!(
                formatter,
                "{collection} has {actual} elements at byte {position}; maximum is {maximum}"
            ),
            Self::InvalidKey {
                position,
                context,
                source,
            } => write!(formatter, "invalid {context} at byte {position}: {source}"),
            Self::InvalidDigestLength {
                position,
                context,
                actual,
            } => write!(
                formatter,
                "{context} has {actual} bytes at byte {position}; expected 32"
            ),
            Self::UnsupportedSchema {
                position,
                schema,
                expected,
                actual,
            } => write!(
                formatter,
                "unsupported {schema} schema {actual} at byte {position}; expected {expected}"
            ),
            Self::UnknownDefinitionTag { position, actual } => write!(
                formatter,
                "unknown definition-family tag {actual} at byte {position}"
            ),
            Self::UnknownValueKind { position, actual } => {
                write!(
                    formatter,
                    "unknown value-kind tag {actual} at byte {position}"
                )
            }
            Self::InterfaceSlotOutOfRange {
                position,
                slot,
                available,
            } => write!(
                formatter,
                "interface slot {slot} at byte {position} is outside {available} entries"
            ),
            Self::TrailingBytes {
                position,
                remaining,
            } => write!(
                formatter,
                "{remaining} trailing bytes begin at byte {position}"
            ),
        }
    }
}

impl std::error::Error for CodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidKey { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ArtifactEnvelope, ArtifactValidator};
    use super::*;
    use crate::interface::{
        OperationKind, OperationParameter, ParameterName, SemanticInterfaceCatalog,
        SemanticInterfaceDescriptor, SemanticOperationDescriptor,
    };

    fn valid<T, E: fmt::Display>(value: Result<T, E>) -> T {
        match value {
            Ok(value) => value,
            Err(error) => panic!("invalid codec fixture: {error}"),
        }
    }

    fn transfer_fixture() -> (SemanticInterfaceCatalog, ArtifactData) {
        let interface_key = valid(SemanticInterfaceKey::parse("world.standard.transfer"));
        let operation_name = valid(OperationName::parse("transfer-item"));
        let parameter_name = valid(ParameterName::parse("item"));
        let operation = valid(SemanticOperationDescriptor::new(
            operation_name.clone(),
            OperationKind::Effect,
            vec![OperationParameter::new(parameter_name, ValueKind::Entity)],
        ));
        let descriptor = valid(SemanticInterfaceDescriptor::new(
            interface_key.clone(),
            valid(InterfaceVersion::new(1)),
            vec![operation],
        ));
        let reference = descriptor.reference();
        let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor]));

        let pack_key = valid(PackKey::parse("world.standard"));
        let event_name = valid(LocalDefinitionName::parse("item-transferred"));
        let action_name = valid(LocalDefinitionName::parse("transfer-item"));
        let binding_name = valid(BindingName::parse("item"));
        let field_name = valid(EventFieldName::parse("item"));

        let event = EventData::new(
            event_name.clone(),
            vec![EventFieldData::new(field_name.clone(), ValueKind::Entity)],
        );
        let call =
            OperationCallData::new(interface_key, operation_name, vec![binding_name.clone()]);
        let emission = EventEmissionData::new(
            DefinitionKey::new(pack_key.clone(), event_name),
            vec![EventFieldBindingData::new(field_name, binding_name.clone())],
        );
        let action = ActionData::new(
            action_name,
            vec![ActionBindingData::new(binding_name, ValueKind::Entity)],
            Vec::new(),
            vec![EffectCallData::new(call)],
            vec![emission],
        );
        let manifest = PackManifestData::new(
            EngineProtocolVersion::new(1),
            PackCoordinate::new(pack_key, PackVersion::new(1, 0, 0)),
            Vec::new(),
        );
        (
            catalog,
            ArtifactData::new(manifest, vec![reference], vec![action], vec![event]),
        )
    }

    #[test]
    fn checked_artifact_round_trips_through_exact_bytes() {
        let (catalog, data) = transfer_fixture();
        let validator = ArtifactValidator::new(&catalog);
        let verified = valid(validator.validate(data));
        let envelope = ArtifactEnvelope::new(
            verified.envelope().descriptor().clone(),
            verified.envelope().blob().to_vec(),
        );
        let loaded = valid(validator.load(envelope));

        assert_eq!(loaded.artifact_digest(), verified.artifact_digest());
        assert_eq!(
            loaded.semantic_fingerprint(),
            verified.semantic_fingerprint()
        );
        assert_eq!(loaded.actions().len(), 1);
        assert_eq!(loaded.events().len(), 1);
    }

    #[test]
    fn decoder_rejects_indefinite_roots_and_trailing_bytes() {
        assert_eq!(
            decode(&[0x9f]),
            Err(CodecError::IndefiniteArray {
                position: 0,
                context: "artifact root",
            })
        );

        let mut minimal = vec![
            0x84, 0x01, 0x84, 0x01, 0x00, 0x82, 0x61, b'a', 0x83, 0x00, 0x00, 0x00, 0x80, 0x80,
            0x80,
        ];
        minimal.push(0);
        assert_eq!(
            decode(&minimal),
            Err(CodecError::TrailingBytes {
                position: minimal.len() - 1,
                remaining: 1,
            })
        );
    }
}
