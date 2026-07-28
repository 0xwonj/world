use core::{convert::Infallible, fmt};

use minicbor::{Decoder, Encoder};
use world_context::{ActionInputFingerprint, GroundedActionCandidateId};
use world_core::{CanonicalDomain, CanonicalWriter, ContentDigest};

use crate::ActionDecision;

const ACTION_DECISION_SCHEMA_VERSION: u16 = 1;
const ACTION_DECISION_SCHEMA_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-decision-schema-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action decision schema domain must be valid"),
    };
const SELECT_ARITY: u64 = 4;
const NO_APPLICABLE_ACTION_ARITY: u64 = 3;
const SELECT_TAG: u16 = 0;
const NO_APPLICABLE_ACTION_TAG: u16 = 1;

/// Fixed schema identity of canonical action-decision artifacts.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionDecisionSchemaId([u8; 32]);

impl ActionDecisionSchemaId {
    /// Constructs an identity decoded from its owning artifact protocol.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the exact identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the identity and returns its bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ActionDecisionSchemaId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ActionDecisionSchemaId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ActionDecisionSchemaId({self})")
    }
}

/// Returns the fixed schema identity of canonical action-decision artifacts.
#[must_use]
pub fn action_decision_schema() -> ActionDecisionSchemaId {
    let mut writer = CanonicalWriter::new(ACTION_DECISION_SCHEMA_DOMAIN);
    writer.write_u16(ACTION_DECISION_SCHEMA_VERSION);
    ActionDecisionSchemaId(ContentDigest::of_canonical(&writer.finish()).into_bytes())
}

/// Encodes one complete checked action decision in canonical array form.
#[must_use]
pub fn encode_action_decision(decision: ActionDecision) -> Vec<u8> {
    let mut encoder = Encoder::new(Vec::new());
    match decision {
        ActionDecision::Select { candidate, input } => {
            encode_array(&mut encoder, "selected action decision", SELECT_ARITY);
            encode_u16(
                &mut encoder,
                "action decision schema",
                ACTION_DECISION_SCHEMA_VERSION,
            );
            encode_u16(&mut encoder, "action decision kind", SELECT_TAG);
            encode_bytes(&mut encoder, "action input fingerprint", input.as_bytes());
            encode_bytes(
                &mut encoder,
                "selected candidate identity",
                candidate.as_bytes(),
            );
        }
        ActionDecision::NoApplicableAction { input } => {
            encode_array(
                &mut encoder,
                "no-applicable-action decision",
                NO_APPLICABLE_ACTION_ARITY,
            );
            encode_u16(
                &mut encoder,
                "action decision schema",
                ACTION_DECISION_SCHEMA_VERSION,
            );
            encode_u16(
                &mut encoder,
                "action decision kind",
                NO_APPLICABLE_ACTION_TAG,
            );
            encode_bytes(&mut encoder, "action input fingerprint", input.as_bytes());
        }
    }
    encoder.into_writer()
}

/// Decodes one complete canonical action decision.
pub fn decode_action_decision(bytes: &[u8]) -> Result<ActionDecision, ActionDecisionCodecError> {
    let mut decoder = Decoder::new(bytes);
    let root_position = decoder.position();
    let arity = decoder
        .array()
        .map_err(|error| unexpected_cbor(error, root_position, "action decision"))?
        .ok_or(ActionDecisionCodecError::IndefiniteArray {
            position: root_position,
        })?;
    let schema_position = decoder.position();
    let schema = decode_u16(&mut decoder, "action decision schema")?;
    if schema != ACTION_DECISION_SCHEMA_VERSION {
        return Err(ActionDecisionCodecError::SchemaMismatch {
            position: schema_position,
            expected: ACTION_DECISION_SCHEMA_VERSION,
            actual: schema,
        });
    }
    let tag_position = decoder.position();
    let decision = match decode_u16(&mut decoder, "action decision kind")? {
        SELECT_TAG => {
            require_arity(
                root_position,
                "selected action decision",
                SELECT_ARITY,
                arity,
            )?;
            let input = ActionInputFingerprint::from_bytes(decode_fixed(
                &mut decoder,
                "action input fingerprint",
            )?);
            let candidate = GroundedActionCandidateId::from_bytes(decode_fixed(
                &mut decoder,
                "selected candidate identity",
            )?);
            ActionDecision::Select { candidate, input }
        }
        NO_APPLICABLE_ACTION_TAG => {
            require_arity(
                root_position,
                "no-applicable-action decision",
                NO_APPLICABLE_ACTION_ARITY,
                arity,
            )?;
            let input = ActionInputFingerprint::from_bytes(decode_fixed(
                &mut decoder,
                "action input fingerprint",
            )?);
            ActionDecision::NoApplicableAction { input }
        }
        actual => {
            return Err(ActionDecisionCodecError::InvalidTag {
                position: tag_position,
                actual,
            });
        }
    };
    if decoder.position() != bytes.len() {
        return Err(ActionDecisionCodecError::TrailingBytes {
            position: decoder.position(),
            remaining: bytes.len() - decoder.position(),
        });
    }
    if encode_action_decision(decision) != bytes {
        return Err(ActionDecisionCodecError::NonCanonicalEncoding);
    }
    Ok(decision)
}

fn require_arity(
    position: usize,
    context: &'static str,
    expected: u64,
    actual: u64,
) -> Result<(), ActionDecisionCodecError> {
    if expected == actual {
        Ok(())
    } else {
        Err(ActionDecisionCodecError::WrongArrayLength {
            position,
            context,
            expected,
            actual,
        })
    }
}

fn decode_fixed(
    decoder: &mut Decoder<'_>,
    context: &'static str,
) -> Result<[u8; 32], ActionDecisionCodecError> {
    let position = decoder.position();
    let value = decoder
        .bytes()
        .map_err(|error| unexpected_cbor(error, position, context))?;
    value
        .try_into()
        .map_err(|_| ActionDecisionCodecError::InvalidByteLength {
            position,
            context,
            expected: 32,
            actual: value.len(),
        })
}

fn decode_u16(
    decoder: &mut Decoder<'_>,
    expected: &'static str,
) -> Result<u16, ActionDecisionCodecError> {
    let position = decoder.position();
    decoder
        .u16()
        .map_err(|error| unexpected_cbor(error, position, expected))
}

fn unexpected_cbor(
    error: minicbor::decode::Error,
    fallback_position: usize,
    expected: &'static str,
) -> ActionDecisionCodecError {
    ActionDecisionCodecError::UnexpectedCbor {
        position: error.position().unwrap_or(fallback_position),
        expected,
    }
}

fn encode_array(encoder: &mut Encoder<Vec<u8>>, context: &'static str, length: u64) {
    encode_result(context, encoder.array(length));
}

fn encode_u16(encoder: &mut Encoder<Vec<u8>>, context: &'static str, value: u16) {
    encode_result(context, encoder.u16(value));
}

fn encode_bytes(encoder: &mut Encoder<Vec<u8>>, context: &'static str, value: &[u8]) {
    encode_result(context, encoder.bytes(value));
}

fn encode_result<T>(context: &'static str, result: Result<T, minicbor::encode::Error<Infallible>>) {
    result
        .unwrap_or_else(|_| unreachable!("Vec-backed CBOR encoder failed while writing {context}"));
}

/// Why bytes could not be accepted as a canonical action decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionDecisionCodecError {
    /// A CBOR item had the wrong primitive or was truncated.
    UnexpectedCbor {
        /// Exact input byte position.
        position: usize,
        /// Protocol value expected at that position.
        expected: &'static str,
    },
    /// The root used CBOR's indefinite array representation.
    IndefiniteArray {
        /// Exact input byte position.
        position: usize,
    },
    /// The selected decision variant had the wrong arity.
    WrongArrayLength {
        /// Exact input byte position.
        position: usize,
        /// Decision variant being decoded.
        context: &'static str,
        /// Required array length.
        expected: u64,
        /// Encoded array length.
        actual: u64,
    },
    /// The encoded schema version was not selected.
    SchemaMismatch {
        /// Exact input byte position.
        position: usize,
        /// Required schema version.
        expected: u16,
        /// Encoded schema version.
        actual: u16,
    },
    /// The decision carried an unknown closed tag.
    InvalidTag {
        /// Exact input byte position.
        position: usize,
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
    /// Bytes remained after the complete decision.
    TrailingBytes {
        /// First trailing byte position.
        position: usize,
        /// Number of unconsumed bytes.
        remaining: usize,
    },
    /// The decision used a semantically equivalent noncanonical CBOR spelling.
    NonCanonicalEncoding,
}

impl fmt::Display for ActionDecisionCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid action decision artifact: {self:?}")
    }
}

impl std::error::Error for ActionDecisionCodecError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection() -> ActionDecision {
        ActionDecision::Select {
            candidate: GroundedActionCandidateId::from_bytes([0x31; 32]),
            input: ActionInputFingerprint::from_bytes([0x41; 32]),
        }
    }

    #[test]
    fn both_closed_decisions_round_trip_canonically() {
        let selected = selection();
        let none = ActionDecision::NoApplicableAction {
            input: ActionInputFingerprint::from_bytes([0x42; 32]),
        };

        for decision in [selected, none] {
            let encoded = encode_action_decision(decision);
            assert_eq!(decode_action_decision(&encoded), Ok(decision));
            assert_eq!(
                encode_action_decision(decode_action_decision(&encoded).unwrap_or_else(|error| {
                    panic!("canonical action decision must decode: {error}")
                })),
                encoded
            );
        }
    }

    #[test]
    fn action_decision_schema_and_array_representation_are_frozen() {
        let encoded = encode_action_decision(selection());
        let mut expected = vec![0x84, 0x01, 0x00, 0x58, 0x20];
        expected.extend_from_slice(&[0x41; 32]);
        expected.extend_from_slice(&[0x58, 0x20]);
        expected.extend_from_slice(&[0x31; 32]);

        assert_eq!(
            action_decision_schema().to_string(),
            "b7caa715d418cdb161401cb244ba4e3792429f770570b5998cf63c9dd1aa425c"
        );
        assert_eq!(encoded, expected);
    }

    #[test]
    fn decoder_rejects_truncation_trailing_bytes_tags_lengths_and_alternate_cbor() {
        let encoded = encode_action_decision(selection());

        let mut truncated = encoded.clone();
        truncated.pop();
        assert!(matches!(
            decode_action_decision(&truncated),
            Err(ActionDecisionCodecError::UnexpectedCbor { .. })
        ));

        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(matches!(
            decode_action_decision(&trailing),
            Err(ActionDecisionCodecError::TrailingBytes { .. })
        ));

        let mut invalid_tag = encoded.clone();
        invalid_tag[2] = 9;
        assert_eq!(
            decode_action_decision(&invalid_tag),
            Err(ActionDecisionCodecError::InvalidTag {
                position: 2,
                actual: 9,
            })
        );

        let mut invalid_arity = encoded.clone();
        invalid_arity[0] = 0x83;
        assert!(matches!(
            decode_action_decision(&invalid_arity),
            Err(ActionDecisionCodecError::WrongArrayLength { .. })
        ));

        let mut invalid_identity_length = encoded.clone();
        invalid_identity_length[3] = 0x58;
        invalid_identity_length[4] = 0x1f;
        assert!(matches!(
            decode_action_decision(&invalid_identity_length),
            Err(ActionDecisionCodecError::InvalidByteLength {
                context: "action input fingerprint",
                actual: 31,
                ..
            })
        ));

        let mut noncanonical = Vec::with_capacity(encoded.len() + 1);
        noncanonical.push(encoded[0]);
        noncanonical.extend_from_slice(&[0x18, 0x01]);
        noncanonical.extend_from_slice(&encoded[2..]);
        assert_eq!(
            decode_action_decision(&noncanonical),
            Err(ActionDecisionCodecError::NonCanonicalEncoding)
        );
    }
}
