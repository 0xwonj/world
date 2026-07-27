macro_rules! semantic_identity {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Constructs an identity decoded or derived by its semantic
            /// owner.
            ///
            /// This validates representation shape only. The owning schema is
            /// responsible for deriving or verifying these bytes.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Returns the exact identity bytes.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }

            /// Consumes the identity and returns its exact bytes.
            #[must_use]
            pub const fn into_bytes(self) -> [u8; 32] {
                self.0
            }
        }

    };
}

semantic_identity!(
    /// Durable semantic identity for an entity across model and runtime
    /// planes.
    ///
    /// Entity schemas own derivation and verification. This representation
    /// type deliberately has no conversion from another identity or generic
    /// content digest.
    ///
    /// ```compile_fail
    /// use world_core::{ActorId, EntityId};
    ///
    /// let entity = EntityId::from_bytes([0; 32]);
    /// let _: ActorId = entity.into();
    /// ```
    EntityId
);

semantic_identity!(
    /// Durable semantic identity for an actor or actor-like controller.
    ///
    /// Actor schemas own derivation and verification. This representation
    /// type deliberately has no conversion from another identity or generic
    /// content digest.
    ActorId
);
