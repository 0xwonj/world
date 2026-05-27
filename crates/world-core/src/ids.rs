use core::num::NonZeroU64;

use crate::InvalidCoreValue;

macro_rules! core_value {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(NonZeroU64);

        impl $name {
            /// Creates the value when the raw identifier is nonzero.
            pub const fn new(value: u64) -> Option<Self> {
                match NonZeroU64::new(value) {
                    Some(value) => Some(Self(value)),
                    None => None,
                }
            }

            /// Returns the underlying stable numeric value.
            pub const fn get(self) -> u64 {
                self.0.get()
            }
        }

        impl TryFrom<u64> for $name {
            type Error = InvalidCoreValue;

            fn try_from(value: u64) -> Result<Self, Self::Error> {
                Self::new(value).ok_or(InvalidCoreValue::Zero {
                    type_name: stringify!($name),
                })
            }
        }

        impl From<$name> for u64 {
            fn from(value: $name) -> Self {
                value.get()
            }
        }
    };
}

macro_rules! issued_value {
    ($(#[$meta:meta])* $name:ident, $issuer:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(NonZeroU64);

        impl $name {
            /// Returns the underlying stable numeric value.
            pub const fn get(self) -> u64 {
                self.0.get()
            }
        }

            /// Monotonic issuer for allocation labels owned by a store or runtime boundary.
            ///
            /// Issued ids are stable references, not authority tokens. Committed records
            /// must still be accepted through the crate that owns the relevant authority gate.
            #[derive(Clone, Debug, PartialEq, Eq)]
            pub struct $issuer {
            next: Option<NonZeroU64>,
        }

        impl Default for $issuer {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $issuer {
            /// Creates an issuer starting at the first valid value.
            pub const fn new() -> Self {
                Self {
                    next: NonZeroU64::new(1),
                }
            }

            /// Creates an issuer whose next issued value is `next_value`.
            pub fn starting_at(next_value: u64) -> Result<Self, InvalidCoreValue> {
                let next = NonZeroU64::new(next_value).ok_or(InvalidCoreValue::Zero {
                    type_name: stringify!($name),
                })?;
                Ok(Self { next: Some(next) })
            }

            /// Issues the next value, returning `None` after the numeric space is exhausted.
            pub fn issue(&mut self) -> Option<$name> {
                let next = self.next?;
                self.next = next.get().checked_add(1).and_then(NonZeroU64::new);
                Some($name(next))
            }

            /// Returns the next value that will be issued, if the numeric space is not exhausted.
            pub const fn next_value(&self) -> Option<u64> {
                match self.next {
                    Some(next) => Some(next.get()),
                    None => None,
                }
            }
        }
    };
}

core_value!(
    /// Durable story identity for an entity.
    EntityId
);

core_value!(
    /// Durable identity for an actor or actor-like controller.
    ActorId
);

core_value!(
    /// Runtime-facing checked definition identity.
    DefinitionId
);

issued_value!(
    /// Durable committed event identity.
    EventRecordId,
    EventRecordIdIssuer
);

issued_value!(
    /// Durable causal transaction identity.
    CausalTransactionId,
    CausalTransactionIdIssuer
);

issued_value!(
    /// Durable runtime progress and execution-frame identity.
    ProcessInstanceId,
    ProcessInstanceIdIssuer
);

issued_value!(
    /// Durable actor-facing temporal execution identity.
    ActivityId,
    ActivityIdIssuer
);

issued_value!(
    /// Opaque in-memory entity handle, distinct from durable entity identity.
    RuntimeEntityHandle,
    RuntimeEntityHandleIssuer
);

core_value!(
    /// Opaque provenance identity used to link records to evidence.
    ProvenanceKey
);

core_value!(
    /// Opaque version anchor for schema, content, or checked definition versions.
    VersionAnchor
);
