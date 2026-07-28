use core::fmt;

use crate::authority::{AuthorityCursor, EpochIdentity};

use super::{
    CanonicalExecutionSpecV1, EpochLineage, EpochLineageId, EpochOriginId,
    ExecutionSemanticsManifestDigest, ExecutionSemanticsManifestV1, ExecutionSpecId,
    InitialStateRootId, InitialStateRootV1, lineage::EpochLineageCorrespondenceError,
};

/// Immutable component whose stored identity failed canonical rederivation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialBindingComponent {
    /// Initial state root.
    InitialRoot,
    /// Epoch lineage.
    EpochLineage,
    /// Execution specification.
    ExecutionSpec,
    /// Execution-semantics manifest.
    ExecutionSemantics,
}

/// Why a root, specification, and semantics manifest could not be bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialExecutionBindingError {
    /// A retained value did not reproduce its stored identity.
    IdentityMismatch {
        /// Component that failed canonical rederivation.
        component: InitialBindingComponent,
    },
    /// The specification names a different initial state root.
    InitialRootMismatch {
        /// Identity derived from the supplied root.
        expected: InitialStateRootId,
        /// Identity named by the specification.
        actual: InitialStateRootId,
    },
    /// The specification names a different execution-semantics manifest.
    ExecutionSemanticsMismatch {
        /// Identity derived from the supplied manifest.
        expected: ExecutionSemanticsManifestDigest,
        /// Identity named by the specification.
        actual: ExecutionSemanticsManifestDigest,
    },
    /// An origin lineage does not identify the supplied root semantic body.
    OriginSemanticBodyMismatch {
        /// Origin identity derived from the supplied root semantic body.
        expected: EpochOriginId,
        /// Origin identity retained by the root lineage.
        actual: EpochOriginId,
    },
    /// A child lineage names a different parent execution than its parent cursor.
    ChildParentExecutionMismatch {
        /// Parent execution named by the retained parent cursor.
        expected: ExecutionSpecId,
        /// Parent execution named independently by the child lineage.
        actual: ExecutionSpecId,
    },
}

impl fmt::Display for InitialExecutionBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentityMismatch { component } => {
                write!(
                    formatter,
                    "retained {component:?} does not reproduce its canonical identity"
                )
            }
            Self::InitialRootMismatch { expected, actual } => write!(
                formatter,
                "execution specification names initial root {actual}, expected {expected}"
            ),
            Self::ExecutionSemanticsMismatch { expected, actual } => write!(
                formatter,
                "execution specification names semantics {actual}, expected {expected}"
            ),
            Self::OriginSemanticBodyMismatch { expected, actual } => write!(
                formatter,
                "origin lineage names semantic body {actual}, expected {expected}"
            ),
            Self::ChildParentExecutionMismatch { expected, actual } => write!(
                formatter,
                "child lineage names parent execution {actual}, expected {expected}"
            ),
        }
    }
}

impl std::error::Error for InitialExecutionBindingError {}

pub(crate) struct VerifiedInitialExecutionBinding {
    root: InitialStateRootV1,
    specification: CanonicalExecutionSpecV1,
    semantics: ExecutionSemanticsManifestV1,
    root_cursor: AuthorityCursor,
}

impl VerifiedInitialExecutionBinding {
    pub(crate) fn new(
        root: InitialStateRootV1,
        specification: CanonicalExecutionSpecV1,
        semantics: ExecutionSemanticsManifestV1,
    ) -> Result<Self, InitialExecutionBindingError> {
        if root.id() != InitialStateRootId::of_canonical(&root.canonical_bytes()) {
            return Err(InitialExecutionBindingError::IdentityMismatch {
                component: InitialBindingComponent::InitialRoot,
            });
        }
        if root.lineage_id() != EpochLineageId::of_canonical(&root.lineage().canonical_bytes()) {
            return Err(InitialExecutionBindingError::IdentityMismatch {
                component: InitialBindingComponent::EpochLineage,
            });
        }
        validate_lineage_correspondence(root.lineage(), &root.semantic_body_bytes())?;
        if specification.id() != ExecutionSpecId::of_canonical(&specification.canonical_bytes()) {
            return Err(InitialExecutionBindingError::IdentityMismatch {
                component: InitialBindingComponent::ExecutionSpec,
            });
        }
        if semantics.digest()
            != ExecutionSemanticsManifestDigest::of_canonical(&semantics.canonical_bytes())
        {
            return Err(InitialExecutionBindingError::IdentityMismatch {
                component: InitialBindingComponent::ExecutionSemantics,
            });
        }
        if specification.initial_root() != root.id() {
            return Err(InitialExecutionBindingError::InitialRootMismatch {
                expected: root.id(),
                actual: specification.initial_root(),
            });
        }
        if specification.semantics() != semantics.digest() {
            return Err(InitialExecutionBindingError::ExecutionSemanticsMismatch {
                expected: semantics.digest(),
                actual: specification.semantics(),
            });
        }

        let root_cursor = AuthorityCursor::root(
            EpochIdentity::new(root.lineage_id(), specification.id()),
            root.id(),
        );
        Ok(Self {
            root,
            specification,
            semantics,
            root_cursor,
        })
    }

    pub(crate) fn root(&self) -> &InitialStateRootV1 {
        &self.root
    }

    pub(crate) fn specification(&self) -> &CanonicalExecutionSpecV1 {
        &self.specification
    }

    pub(crate) fn semantics(&self) -> &ExecutionSemanticsManifestV1 {
        &self.semantics
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        InitialStateRootV1,
        CanonicalExecutionSpecV1,
        ExecutionSemanticsManifestV1,
        AuthorityCursor,
    ) {
        (
            self.root,
            self.specification,
            self.semantics,
            self.root_cursor,
        )
    }
}

fn validate_lineage_correspondence(
    lineage: EpochLineage,
    semantic_body: &world_core::CanonicalBytes,
) -> Result<(), InitialExecutionBindingError> {
    lineage
        .validate_semantic_correspondence(semantic_body)
        .map_err(|error| match error {
            EpochLineageCorrespondenceError::OriginSemanticBodyMismatch { expected, actual } => {
                InitialExecutionBindingError::OriginSemanticBodyMismatch { expected, actual }
            }
            EpochLineageCorrespondenceError::ChildParentExecutionMismatch { expected, actual } => {
                InitialExecutionBindingError::ChildParentExecutionMismatch { expected, actual }
            }
        })
}

#[cfg(test)]
mod tests {
    use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter};

    use crate::authority::{AuthorityCursor, EpochIdentity};

    use super::*;
    use crate::execution::{
        BranchTransformId, ChildEpochTransform, EpochLineageBody, InitialStateRootId,
    };

    const FIXTURE_DOMAIN: CanonicalDomain = match CanonicalDomain::new("binding-lineage-fixture-v1")
    {
        Ok(domain) => domain,
        Err(_) => panic!("binding lineage fixture domain must be valid"),
    };

    fn semantic_body(value: u8) -> CanonicalBytes {
        let mut writer = CanonicalWriter::new(FIXTURE_DOMAIN);
        writer.write_u8(value);
        writer.finish()
    }

    #[test]
    fn binding_reports_an_origin_semantic_body_mismatch() {
        let semantic_body = semantic_body(1);
        let EpochLineageBody::Origin { origin: expected } =
            EpochLineage::origin(&semantic_body).body()
        else {
            panic!("origin fixture must retain its origin identity");
        };
        let actual = EpochOriginId::from_bytes([0x21; 32]);
        let lineage = EpochLineage::fixture_from_body(EpochLineageBody::Origin { origin: actual });

        assert_eq!(
            validate_lineage_correspondence(lineage, &semantic_body),
            Err(InitialExecutionBindingError::OriginSemanticBodyMismatch { expected, actual })
        );
    }

    #[test]
    fn binding_reports_a_child_parent_execution_mismatch() {
        let expected = ExecutionSpecId::from_bytes([0x21; 32]);
        let actual = ExecutionSpecId::from_bytes([0x22; 32]);
        let parent_cursor = AuthorityCursor::root(
            EpochIdentity::new(EpochLineageId::from_bytes([0x31; 32]), expected),
            InitialStateRootId::from_bytes([0x32; 32]),
        );
        let lineage = EpochLineage::fixture_from_body(EpochLineageBody::Child {
            parent_execution: actual,
            parent_cursor,
            transform: ChildEpochTransform::Branch(BranchTransformId::from_bytes([0x41; 32])),
        });

        assert_eq!(
            validate_lineage_correspondence(lineage, &semantic_body(1)),
            Err(InitialExecutionBindingError::ChildParentExecutionMismatch { expected, actual })
        );
    }
}
