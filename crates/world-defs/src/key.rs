//! Checked names, coordinates, and protocol versions used by pack definitions.

use std::fmt;
use std::num::NonZeroU16;

const PACK_KEY_MAX_BYTES: usize = 127;
const SEGMENT_MAX_BYTES: usize = 63;

/// Why a checked pack name or version could not be constructed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeyError {
    /// The identifier had no bytes.
    Empty {
        /// The semantic identifier type being constructed.
        kind: &'static str,
    },
    /// The identifier exceeded its byte-length limit.
    TooLong {
        /// The semantic identifier type being constructed.
        kind: &'static str,
        /// The largest accepted byte length.
        max_bytes: usize,
        /// The provided byte length.
        actual_bytes: usize,
    },
    /// The identifier contained a non-ASCII code point.
    NonAscii {
        /// The semantic identifier type being constructed.
        kind: &'static str,
        /// The byte index at which the non-ASCII encoding begins.
        byte_index: usize,
    },
    /// A dot-qualified identifier contained an empty segment.
    EmptySegment {
        /// The semantic identifier type being constructed.
        kind: &'static str,
        /// The byte index at which a segment was required.
        byte_index: usize,
    },
    /// A segment did not begin with a lowercase ASCII letter.
    InvalidSegmentStart {
        /// The semantic identifier type being constructed.
        kind: &'static str,
        /// The byte index of the invalid first byte.
        byte_index: usize,
        /// The invalid ASCII byte.
        byte: u8,
    },
    /// A segment contained a byte outside lowercase ASCII, digits, and hyphens.
    InvalidSegmentCharacter {
        /// The semantic identifier type being constructed.
        kind: &'static str,
        /// The byte index of the invalid byte.
        byte_index: usize,
        /// The invalid ASCII byte.
        byte: u8,
    },
    /// A hyphen was not followed by at least one lowercase ASCII letter or digit.
    InvalidHyphen {
        /// The semantic identifier type being constructed.
        kind: &'static str,
        /// The byte index of the invalid hyphen.
        byte_index: usize,
    },
    /// Semantic-interface version zero is reserved as invalid.
    ZeroInterfaceVersion,
}

impl fmt::Display for KeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { kind } => write!(formatter, "{kind} must not be empty"),
            Self::TooLong {
                kind,
                max_bytes,
                actual_bytes,
            } => write!(
                formatter,
                "{kind} is {actual_bytes} bytes; the maximum is {max_bytes}"
            ),
            Self::NonAscii { kind, byte_index } => {
                write!(
                    formatter,
                    "{kind} contains non-ASCII text at byte {byte_index}"
                )
            }
            Self::EmptySegment { kind, byte_index } => {
                write!(
                    formatter,
                    "{kind} has an empty segment at byte {byte_index}"
                )
            }
            Self::InvalidSegmentStart {
                kind,
                byte_index,
                byte,
            } => write!(
                formatter,
                "{kind} segment must start with a lowercase ASCII letter at byte \
                 {byte_index}, found {:?}",
                char::from(*byte)
            ),
            Self::InvalidSegmentCharacter {
                kind,
                byte_index,
                byte,
            } => write!(
                formatter,
                "{kind} contains invalid character {:?} at byte {byte_index}",
                char::from(*byte)
            ),
            Self::InvalidHyphen { kind, byte_index } => write!(
                formatter,
                "{kind} hyphen at byte {byte_index} must be followed by a lowercase ASCII \
                 letter or digit"
            ),
            Self::ZeroInterfaceVersion => {
                formatter.write_str("semantic-interface version must be nonzero")
            }
        }
    }
}

impl std::error::Error for KeyError {}

fn validate_length(kind: &'static str, value: &str, max_bytes: usize) -> Result<(), KeyError> {
    if value.is_empty() {
        return Err(KeyError::Empty { kind });
    }
    if value.len() > max_bytes {
        return Err(KeyError::TooLong {
            kind,
            max_bytes,
            actual_bytes: value.len(),
        });
    }
    if let Some(byte_index) = value.bytes().position(|byte| !byte.is_ascii()) {
        return Err(KeyError::NonAscii { kind, byte_index });
    }
    Ok(())
}

fn validate_segment_bytes(
    kind: &'static str,
    bytes: &[u8],
    base_index: usize,
) -> Result<(), KeyError> {
    let first = bytes[0];
    if !first.is_ascii_lowercase() {
        return Err(KeyError::InvalidSegmentStart {
            kind,
            byte_index: base_index,
            byte: first,
        });
    }

    for (offset, byte) in bytes.iter().copied().enumerate().skip(1) {
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            continue;
        }
        if byte == b'-' {
            let next = bytes.get(offset + 1).copied();
            if next.is_some_and(|next| next.is_ascii_lowercase() || next.is_ascii_digit()) {
                continue;
            }
            return Err(KeyError::InvalidHyphen {
                kind,
                byte_index: base_index + offset,
            });
        }
        return Err(KeyError::InvalidSegmentCharacter {
            kind,
            byte_index: base_index + offset,
            byte,
        });
    }

    Ok(())
}

fn validate_segment(kind: &'static str, value: &str) -> Result<(), KeyError> {
    validate_length(kind, value, SEGMENT_MAX_BYTES)?;
    validate_segment_bytes(kind, value.as_bytes(), 0)
}

fn validate_dot_name(kind: &'static str, value: &str) -> Result<(), KeyError> {
    validate_length(kind, value, PACK_KEY_MAX_BYTES)?;

    let bytes = value.as_bytes();
    let mut segment_start = 0;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if byte != b'.' {
            continue;
        }
        if index == segment_start {
            return Err(KeyError::EmptySegment {
                kind,
                byte_index: index,
            });
        }
        validate_segment_bytes(kind, &bytes[segment_start..index], segment_start)?;
        segment_start = index + 1;
    }

    if segment_start == bytes.len() {
        return Err(KeyError::EmptySegment {
            kind,
            byte_index: segment_start,
        });
    }
    validate_segment_bytes(kind, &bytes[segment_start..], segment_start)
}

macro_rules! checked_name {
    (
        $(#[$metadata:meta])*
        $name:ident,
        $kind:literal,
        $max_bytes:expr,
        $validator:ident
    ) => {
        $(#[$metadata])*
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// The maximum accepted UTF-8 byte length.
            pub const MAX_BYTE_LENGTH: usize = $max_bytes;

            /// Validates and takes ownership of an identifier.
            pub fn new(value: String) -> Result<Self, KeyError> {
                $validator($kind, &value)?;
                Ok(Self(value))
            }

            /// Validates and copies an identifier from a string slice.
            pub fn parse(value: &str) -> Result<Self, KeyError> {
                Self::new(value.to_owned())
            }

            /// Returns the normalized ASCII identifier.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

checked_name!(
    /// A durable dot-qualified pack name.
    PackKey,
    "PackKey",
    PACK_KEY_MAX_BYTES,
    validate_dot_name
);

checked_name!(
    /// A dot-qualified semantic-interface name.
    SemanticInterfaceKey,
    "SemanticInterfaceKey",
    PACK_KEY_MAX_BYTES,
    validate_dot_name
);

checked_name!(
    /// A pack-local definition name.
    LocalDefinitionName,
    "LocalDefinitionName",
    SEGMENT_MAX_BYTES,
    validate_segment
);

checked_name!(
    /// A semantic-interface operation name.
    OperationName,
    "OperationName",
    SEGMENT_MAX_BYTES,
    validate_segment
);

checked_name!(
    /// A semantic-interface parameter name.
    ParameterName,
    "ParameterName",
    SEGMENT_MAX_BYTES,
    validate_segment
);

checked_name!(
    /// A named action binding.
    BindingName,
    "BindingName",
    SEGMENT_MAX_BYTES,
    validate_segment
);

checked_name!(
    /// A field name in a physical event definition.
    EventFieldName,
    "EventFieldName",
    SEGMENT_MAX_BYTES,
    validate_segment
);

/// An exact three-component pack version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PackVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl PackVersion {
    /// Creates an exact pack version.
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns the major component.
    #[must_use]
    pub const fn major(self) -> u32 {
        self.major
    }

    /// Returns the minor component.
    #[must_use]
    pub const fn minor(self) -> u32 {
        self.minor
    }

    /// Returns the patch component.
    #[must_use]
    pub const fn patch(self) -> u32 {
        self.patch
    }
}

impl fmt::Display for PackVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// One exact version of a pack.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PackCoordinate {
    pack_key: PackKey,
    version: PackVersion,
}

impl PackCoordinate {
    /// Binds a durable pack key to one exact version.
    #[must_use]
    pub const fn new(pack_key: PackKey, version: PackVersion) -> Self {
        Self { pack_key, version }
    }

    /// Returns the durable pack key.
    #[must_use]
    pub const fn pack_key(&self) -> &PackKey {
        &self.pack_key
    }

    /// Returns the exact pack version.
    #[must_use]
    pub const fn version(&self) -> PackVersion {
        self.version
    }
}

impl fmt::Display for PackCoordinate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}@{}", self.pack_key, self.version)
    }
}

/// A durable pack-qualified definition identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DefinitionKey {
    pack_key: PackKey,
    local_name: LocalDefinitionName,
}

impl DefinitionKey {
    /// Qualifies a local definition name with its pack.
    #[must_use]
    pub const fn new(pack_key: PackKey, local_name: LocalDefinitionName) -> Self {
        Self {
            pack_key,
            local_name,
        }
    }

    /// Returns the definition's pack key.
    #[must_use]
    pub const fn pack_key(&self) -> &PackKey {
        &self.pack_key
    }

    /// Returns the pack-local definition name.
    #[must_use]
    pub const fn local_name(&self) -> &LocalDefinitionName {
        &self.local_name
    }
}

impl fmt::Display for DefinitionKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.pack_key, self.local_name)
    }
}

/// A nonzero version of a semantic-interface contract.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InterfaceVersion(NonZeroU16);

impl InterfaceVersion {
    /// Creates an interface version, rejecting the reserved value zero.
    pub fn new(value: u16) -> Result<Self, KeyError> {
        NonZeroU16::new(value)
            .map(Self)
            .ok_or(KeyError::ZeroInterfaceVersion)
    }

    /// Returns the nonzero protocol value.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }
}

impl fmt::Display for InterfaceVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The engine protocol version required by a pack.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EngineProtocolVersion(u16);

impl EngineProtocolVersion {
    /// Creates an engine protocol version.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the protocol value.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl fmt::Display for EngineProtocolVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normalized_dot_names() {
        let pack = PackKey::parse("world.standard-2.items");
        let interface = SemanticInterfaceKey::parse("world.standard.transfer");

        assert_eq!(
            pack.as_ref().map(PackKey::as_str),
            Ok("world.standard-2.items")
        );
        assert_eq!(
            interface.as_ref().map(SemanticInterfaceKey::as_str),
            Ok("world.standard.transfer")
        );
    }

    #[test]
    fn rejects_empty_dot_segments_at_their_byte_index() {
        assert_eq!(
            PackKey::parse("world..items"),
            Err(KeyError::EmptySegment {
                kind: "PackKey",
                byte_index: 6,
            })
        );
        assert_eq!(
            PackKey::parse("world."),
            Err(KeyError::EmptySegment {
                kind: "PackKey",
                byte_index: 6,
            })
        );
    }

    #[test]
    fn rejects_non_normalized_or_malformed_segments() {
        assert_eq!(
            OperationName::parse("Transfer"),
            Err(KeyError::InvalidSegmentStart {
                kind: "OperationName",
                byte_index: 0,
                byte: b'T',
            })
        );
        assert_eq!(
            BindingName::parse("source_container"),
            Err(KeyError::InvalidSegmentCharacter {
                kind: "BindingName",
                byte_index: 6,
                byte: b'_',
            })
        );
        assert_eq!(
            EventFieldName::parse("source--item"),
            Err(KeyError::InvalidHyphen {
                kind: "EventFieldName",
                byte_index: 6,
            })
        );
        assert_eq!(
            ParameterName::parse("item-"),
            Err(KeyError::InvalidHyphen {
                kind: "ParameterName",
                byte_index: 4,
            })
        );
    }

    #[test]
    fn reports_non_ascii_and_byte_length_errors() {
        assert_eq!(
            LocalDefinitionName::parse("éclair"),
            Err(KeyError::NonAscii {
                kind: "LocalDefinitionName",
                byte_index: 0,
            })
        );

        let too_long = "a".repeat(LocalDefinitionName::MAX_BYTE_LENGTH + 1);
        assert_eq!(
            LocalDefinitionName::new(too_long),
            Err(KeyError::TooLong {
                kind: "LocalDefinitionName",
                max_bytes: 63,
                actual_bytes: 64,
            })
        );
    }

    #[test]
    fn each_shared_grammar_has_a_distinct_owned_type() {
        let operation = OperationName::parse("transfer-item");
        let parameter = ParameterName::parse("transfer-item");
        let binding = BindingName::parse("transfer-item");
        let event_field = EventFieldName::parse("transfer-item");

        assert!(operation.is_ok());
        assert!(parameter.is_ok());
        assert!(binding.is_ok());
        assert!(event_field.is_ok());
    }

    #[test]
    fn composite_keys_expose_structural_parts() {
        let pack = PackKey::parse("world.standard");
        let local = LocalDefinitionName::parse("transfer-item");
        let (Ok(pack), Ok(local)) = (pack, local) else {
            panic!("valid fixture identifiers must parse");
        };

        let coordinate = PackCoordinate::new(pack.clone(), PackVersion::new(1, 2, 3));
        let definition = DefinitionKey::new(pack, local);

        assert_eq!(coordinate.pack_key().as_str(), "world.standard");
        assert_eq!(coordinate.version(), PackVersion::new(1, 2, 3));
        assert_eq!(coordinate.to_string(), "world.standard@1.2.3");
        assert_eq!(definition.pack_key().as_str(), "world.standard");
        assert_eq!(definition.local_name().as_str(), "transfer-item");
        assert_eq!(definition.to_string(), "world.standard:transfer-item");
    }

    #[test]
    fn versions_preserve_their_declared_widths() {
        let pack = PackVersion::new(u32::MAX, 0, 7);
        let interface = InterfaceVersion::new(u16::MAX);
        let engine = EngineProtocolVersion::new(0);

        assert_eq!(pack.major(), u32::MAX);
        assert_eq!(pack.minor(), 0);
        assert_eq!(pack.patch(), 7);
        assert_eq!(interface.map(InterfaceVersion::get), Ok(u16::MAX));
        assert_eq!(engine.get(), 0);
        assert_eq!(
            InterfaceVersion::new(0),
            Err(KeyError::ZeroInterfaceVersion)
        );
    }
}
