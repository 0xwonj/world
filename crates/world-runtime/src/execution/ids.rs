use core::fmt;

use world_core::{CanonicalBytes, ContentDigest};

macro_rules! runtime_identity {
    ($(#[$metadata:meta])* $name:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Constructs a fixed-width identity decoded by an owning
            /// protocol.
            ///
            /// This proves representation shape only. The owning runtime
            /// protocol verifies or derives the identity before trusting it.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Returns the exact identity bytes.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }

            /// Consumes the value and returns its exact bytes.
            #[must_use]
            pub const fn into_bytes(self) -> [u8; 32] {
                self.0
            }

        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.0 {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}({self})", stringify!($name))
            }
        }
    };
}

runtime_identity!(
    /// Canonical identity of one complete initial session root.
    InitialStateRootId
);
runtime_identity!(
    /// Canonical identity of one pre-run execution specification.
    ExecutionSpecId
);
runtime_identity!(
    /// Canonical identity of normalized behavior-affecting execution
    /// semantics.
    ExecutionSemanticsManifestDigest
);
runtime_identity!(
    /// Canonical identity of one exact execution-configuration artifact.
    ExecutionConfigArtifactDigest
);
runtime_identity!(
    /// Canonical identity of one exact lifecycle-profile selection.
    LifecycleProfilesDigest
);
runtime_identity!(
    /// Identity of one behavior-affecting lifecycle implementation.
    LifecycleImplementationId
);
runtime_identity!(
    /// Identity of one implementation-owned persistent lifecycle-state schema.
    LifecycleStateSchemaId
);
runtime_identity!(
    /// Identity of one behavior-affecting semantic implementation.
    SemanticImplementationId
);
runtime_identity!(
    /// Canonical identity of the closure required to reopen one execution.
    ResolvedExecutionClosureManifestDigest
);
runtime_identity!(
    /// Semantic lineage identity of one session epoch.
    EpochLineageId
);
runtime_identity!(
    /// Identity of an origin epoch's execution-independent semantic root.
    EpochOriginId
);
runtime_identity!(
    /// Identity of an exact branch transformation.
    BranchTransformId
);
runtime_identity!(
    /// Identity of an exact migration reset.
    MigrationResetId
);
runtime_identity!(
    /// Non-reusable semantic namespace for serialized external input.
    ExternalInputNamespaceId
);
runtime_identity!(
    /// Canonical identity of an external-input binding.
    ExternalInputBindingDigest
);
runtime_identity!(
    /// Canonical identity of one termination clause.
    TerminationClauseId
);
runtime_identity!(
    /// Canonical identity of one complete termination contract.
    TerminationContractDigest
);

macro_rules! canonical_runtime_identity {
    ($name:ident) => {
        impl $name {
            pub(crate) fn of_canonical(bytes: &CanonicalBytes) -> Self {
                Self(ContentDigest::of_canonical(bytes).into_bytes())
            }
        }
    };
}

canonical_runtime_identity!(ExecutionConfigArtifactDigest);
canonical_runtime_identity!(ExecutionSemanticsManifestDigest);
canonical_runtime_identity!(ExecutionSpecId);
canonical_runtime_identity!(EpochLineageId);
canonical_runtime_identity!(EpochOriginId);
canonical_runtime_identity!(ExternalInputBindingDigest);
canonical_runtime_identity!(InitialStateRootId);
canonical_runtime_identity!(LifecycleProfilesDigest);
canonical_runtime_identity!(ResolvedExecutionClosureManifestDigest);
canonical_runtime_identity!(TerminationClauseId);
canonical_runtime_identity!(TerminationContractDigest);

/// Exact root seed carried by an execution specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootSeed([u8; 32]);

impl RootSeed {
    /// Constructs a root seed from its exact bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the exact seed bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the seed and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purpose_specific_identities_do_not_share_conversions() {
        let root = InitialStateRootId::from_bytes([1; 32]);
        let spec = ExecutionSpecId::from_bytes([1; 32]);
        let lifecycle = LifecycleImplementationId::from_bytes([1; 32]);
        let semantic = SemanticImplementationId::from_bytes([1; 32]);

        assert_eq!(root.as_bytes(), spec.as_bytes());
        assert_ne!(format!("{root:?}"), format!("{spec:?}"));
        assert_eq!(lifecycle.as_bytes(), semantic.as_bytes());
        assert_ne!(format!("{lifecycle:?}"), format!("{semantic:?}"));
    }
}
