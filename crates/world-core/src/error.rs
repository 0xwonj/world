use core::fmt;

/// Error returned when a raw value cannot become a core domain value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidCoreValue {
    /// Zero was supplied for a value whose zero representation is reserved.
    Zero {
        /// The domain type that rejected zero.
        type_name: &'static str,
    },
}

impl fmt::Display for InvalidCoreValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero { type_name } => write!(formatter, "{type_name} cannot be zero"),
        }
    }
}

impl std::error::Error for InvalidCoreValue {}

impl InvalidCoreValue {
    /// Returns the domain type that rejected the value.
    pub const fn type_name(self) -> &'static str {
        match self {
            Self::Zero { type_name } => type_name,
        }
    }
}
