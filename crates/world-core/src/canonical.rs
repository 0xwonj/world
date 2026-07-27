use core::fmt;

/// Stable identifier written into every canonical preimage.
pub const CANONICAL_PROTOCOL_IDENTIFIER: &str = "world-canonical-v1";

/// Maximum byte length of a canonical identity-domain label.
pub const MAX_CANONICAL_DOMAIN_LENGTH: usize = 64;

/// Error returned when a value cannot be represented by the canonical
/// identity protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonicalError {
    /// An identity preimage omitted its mandatory domain.
    EmptyDomain,
    /// A domain exceeds the protocol's bounded label length.
    DomainTooLong {
        /// Supplied byte length.
        length: usize,
        /// Maximum accepted byte length.
        maximum: usize,
    },
    /// A domain contains a byte outside the stable label grammar.
    InvalidDomainByte {
        /// Byte index in the domain.
        index: usize,
        /// Rejected byte.
        byte: u8,
    },
    /// A host collection length cannot fit the protocol's `u64` length field.
    LengthOverflow {
        /// Supplied host length.
        length: usize,
    },
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDomain => formatter.write_str("canonical domain cannot be empty"),
            Self::DomainTooLong { length, maximum } => {
                write!(
                    formatter,
                    "canonical domain length {length} exceeds {maximum}"
                )
            }
            Self::InvalidDomainByte { index, byte } => {
                write!(
                    formatter,
                    "canonical domain byte 0x{byte:02x} at index {index} is invalid"
                )
            }
            Self::LengthOverflow { length } => {
                write!(formatter, "canonical length {length} does not fit u64")
            }
        }
    }
}

impl std::error::Error for CanonicalError {}

/// Checked, statically owned identity-domain label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalDomain(&'static str);

impl CanonicalDomain {
    /// Validates a stable domain label.
    ///
    /// Domains match `[a-z][a-z0-9-]{0,63}`.
    pub const fn new(value: &'static str) -> Result<Self, CanonicalError> {
        if value.is_empty() {
            return Err(CanonicalError::EmptyDomain);
        }
        if value.len() > MAX_CANONICAL_DOMAIN_LENGTH {
            return Err(CanonicalError::DomainTooLong {
                length: value.len(),
                maximum: MAX_CANONICAL_DOMAIN_LENGTH,
            });
        }

        let bytes = value.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            let valid =
                byte.is_ascii_lowercase() || (index > 0 && (byte.is_ascii_digit() || byte == b'-'));
            if !valid {
                return Err(CanonicalError::InvalidDomainByte { index, byte });
            }
            index += 1;
        }

        Ok(Self(value))
    }

    /// Returns the exact label included in the preimage.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Completed canonical preimage bytes.
///
/// Construction is restricted to [`CanonicalWriter`], which proves protocol
/// framing but not an owner's semantic schema. Storage encoders may copy these
/// bytes; the semantic owner remains responsible for validating field meaning
/// before treating the preimage as an identity source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalBytes(Vec<u8>);

impl CanonicalBytes {
    /// Returns the exact protocol bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consumes the value and returns the exact protocol bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for CanonicalBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

/// Explicit writer for `world-canonical-v1` identity preimages.
///
/// Schema owners define field order and convert logical maps to canonically
/// sorted sequences before writing. This type intentionally provides no
/// blanket serialization trait.
#[derive(Debug)]
pub struct CanonicalWriter {
    bytes: Vec<u8>,
}

impl CanonicalWriter {
    /// Begins a canonical preimage with the protocol and mandatory domain.
    #[must_use]
    pub fn new(domain: CanonicalDomain) -> Self {
        let domain = domain.as_str().as_bytes();
        let mut writer = Self {
            bytes: Vec::with_capacity(
                CANONICAL_PROTOCOL_IDENTIFIER.len() + core::mem::size_of::<u64>() + domain.len(),
            ),
        };
        writer
            .bytes
            .extend_from_slice(CANONICAL_PROTOCOL_IDENTIFIER.as_bytes());
        writer.write_known_bytes(domain);
        writer
    }

    /// Writes an unsigned one-byte integer.
    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    /// Writes an unsigned two-byte big-endian integer.
    pub fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// Writes an unsigned four-byte big-endian integer.
    pub fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// Writes an unsigned eight-byte big-endian integer.
    pub fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// Writes an unsigned sixteen-byte big-endian integer.
    pub fn write_u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// Writes an enum discriminant selected by the schema owner.
    ///
    /// The owner must derive this value through an exhaustive match over its
    /// enum; this writer owns only the stable `u32` representation.
    pub fn write_discriminant(&mut self, value: u32) {
        self.write_u32(value);
    }

    /// Writes a boolean as exactly `0` or `1`.
    pub fn write_bool(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    /// Writes a `u64` byte length followed by exact bytes.
    pub fn write_bytes(&mut self, value: &[u8]) -> Result<(), CanonicalError> {
        self.write_length(value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    /// Writes exact UTF-8 bytes without normalization.
    pub fn write_str(&mut self, value: &str) -> Result<(), CanonicalError> {
        self.write_bytes(value.as_bytes())
    }

    /// Writes an option tag followed by its present value.
    ///
    /// The tag is exactly `0` for absent and `1` for present. A failed value
    /// write restores the writer to its state before this option.
    pub fn write_option<T>(
        &mut self,
        value: Option<&T>,
        write_value: impl FnOnce(&mut Self, &T) -> Result<(), CanonicalError>,
    ) -> Result<(), CanonicalError> {
        let checkpoint = self.bytes.len();
        match value {
            None => {
                self.write_u8(0);
                Ok(())
            }
            Some(value) => {
                self.write_u8(1);
                if let Err(error) = write_value(self, value) {
                    self.bytes.truncate(checkpoint);
                    return Err(error);
                }
                Ok(())
            }
        }
    }

    /// Writes an ordered sequence using its actual element count.
    ///
    /// Logical maps and sets must be validated and converted to a canonical
    /// ordered slice by their semantic owner before this method is called. A
    /// failed element write restores the writer to its state before this
    /// sequence.
    pub fn write_sequence<T>(
        &mut self,
        values: &[T],
        mut write_element: impl FnMut(&mut Self, &T) -> Result<(), CanonicalError>,
    ) -> Result<(), CanonicalError> {
        let checkpoint = self.bytes.len();
        self.write_length(values.len())?;
        for value in values {
            if let Err(error) = write_element(self, value) {
                self.bytes.truncate(checkpoint);
                return Err(error);
            }
        }
        Ok(())
    }

    /// Completes the canonical preimage.
    #[must_use]
    pub fn finish(self) -> CanonicalBytes {
        CanonicalBytes(self.bytes)
    }

    fn write_known_bytes(&mut self, value: &[u8]) {
        let length = match u64::try_from(value.len()) {
            Ok(length) => length,
            Err(_) => unreachable!("checked canonical domain length must fit in u64"),
        };
        self.write_u64(length);
        self.bytes.extend_from_slice(value);
    }

    fn write_length(&mut self, length: usize) -> Result<(), CanonicalError> {
        let length =
            u64::try_from(length).map_err(|_| CanonicalError::LengthOverflow { length })?;
        self.write_u64(length);
        Ok(())
    }
}
