use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter};

use crate::authority::AuthorityCursor;

use super::{BranchTransformId, EpochLineageId, EpochOriginId, ExecutionSpecId, MigrationResetId};

/// Canonical schema of an origin epoch identity.
pub const EPOCH_ORIGIN_SCHEMA_VERSION: u16 = 1;

/// Canonical schema of an epoch lineage body.
pub const EPOCH_LINEAGE_SCHEMA_VERSION: u16 = 1;

const EPOCH_ORIGIN_DOMAIN: CanonicalDomain = match CanonicalDomain::new("epoch-origin-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("epoch origin domain must be valid"),
};

const EPOCH_LINEAGE_DOMAIN: CanonicalDomain = match CanonicalDomain::new("epoch-lineage-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("epoch lineage domain must be valid"),
};

/// Exact transformation that begins a child epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildEpochTransform {
    /// A branch transformation from an exact parent cursor.
    Branch(BranchTransformId),
    /// A migration or reset from an exact parent cursor.
    MigrationReset(MigrationResetId),
}

impl ChildEpochTransform {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Branch(_) => 0,
            Self::MigrationReset(_) => 1,
        }
    }

    const fn identity_bytes(self) -> [u8; 32] {
        match self {
            Self::Branch(identity) => identity.into_bytes(),
            Self::MigrationReset(identity) => identity.into_bytes(),
        }
    }
}

/// Complete semantic ancestry of one execution epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpochLineageBody {
    /// A root epoch derived from its child-execution-independent semantic state.
    Origin {
        /// Identity of the semantic root body before lineage is attached.
        origin: EpochOriginId,
    },
    /// A child epoch derived from an exact parent history position.
    Child {
        /// Parent execution named explicitly by the child lineage.
        parent_execution: ExecutionSpecId,
        /// Complete parent history cursor.
        parent_cursor: AuthorityCursor,
        /// Exact branch or migration transformation.
        transform: ChildEpochTransform,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EpochLineageCorrespondenceError {
    OriginSemanticBodyMismatch {
        expected: EpochOriginId,
        actual: EpochOriginId,
    },
    ChildParentExecutionMismatch {
        expected: ExecutionSpecId,
        actual: ExecutionSpecId,
    },
}

/// Canonical semantic lineage independent of the child execution specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EpochLineage {
    body: EpochLineageBody,
    id: EpochLineageId,
}

impl EpochLineage {
    pub(crate) fn origin(semantic_body: &CanonicalBytes) -> Self {
        let origin = EpochOriginId::of_canonical(&epoch_origin_bytes(semantic_body.as_bytes()));
        Self::from_body(EpochLineageBody::Origin { origin })
    }

    #[cfg(test)]
    pub(crate) fn child(parent_cursor: AuthorityCursor, transform: ChildEpochTransform) -> Self {
        let parent_execution = parent_cursor.epoch().execution();
        Self::from_body(EpochLineageBody::Child {
            parent_execution,
            parent_cursor,
            transform,
        })
    }

    fn from_body(body: EpochLineageBody) -> Self {
        let bytes = epoch_lineage_bytes(body);
        Self {
            body,
            id: EpochLineageId::of_canonical(&bytes),
        }
    }

    /// Returns the complete origin or child lineage body.
    #[must_use]
    pub const fn body(self) -> EpochLineageBody {
        self.body
    }

    /// Returns the semantic lineage identity.
    #[must_use]
    pub const fn id(self) -> EpochLineageId {
        self.id
    }

    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        epoch_lineage_bytes(self.body)
    }

    pub(crate) fn validate_semantic_correspondence(
        self,
        semantic_body: &CanonicalBytes,
    ) -> Result<(), EpochLineageCorrespondenceError> {
        match self.body {
            EpochLineageBody::Origin { origin } => {
                let expected =
                    EpochOriginId::of_canonical(&epoch_origin_bytes(semantic_body.as_bytes()));
                if origin != expected {
                    return Err(
                        EpochLineageCorrespondenceError::OriginSemanticBodyMismatch {
                            expected,
                            actual: origin,
                        },
                    );
                }
            }
            EpochLineageBody::Child {
                parent_execution,
                parent_cursor,
                ..
            } => {
                let expected = parent_cursor.epoch().execution();
                if parent_execution != expected {
                    return Err(
                        EpochLineageCorrespondenceError::ChildParentExecutionMismatch {
                            expected,
                            actual: parent_execution,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn fixture_from_body(body: EpochLineageBody) -> Self {
        Self::from_body(body)
    }
}

fn epoch_origin_bytes(semantic_body: &[u8]) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(EPOCH_ORIGIN_DOMAIN);
    writer.write_u16(EPOCH_ORIGIN_SCHEMA_VERSION);
    write_owned_bytes(&mut writer, semantic_body);
    writer.finish()
}

fn epoch_lineage_bytes(body: EpochLineageBody) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(EPOCH_LINEAGE_DOMAIN);
    writer.write_u16(EPOCH_LINEAGE_SCHEMA_VERSION);
    match body {
        EpochLineageBody::Origin { origin } => {
            writer.write_discriminant(0);
            write_fixed_bytes(&mut writer, origin.as_bytes());
        }
        EpochLineageBody::Child {
            parent_execution,
            parent_cursor,
            transform,
        } => {
            writer.write_discriminant(1);
            write_fixed_bytes(&mut writer, parent_execution.as_bytes());
            write_owned_bytes(&mut writer, parent_cursor.canonical_bytes().as_bytes());
            writer.write_discriminant(transform.canonical_tag());
            write_fixed_bytes(&mut writer, &transform.identity_bytes());
        }
    }
    writer.finish()
}

fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width identity length must fit the canonical protocol");
    }
}

fn write_owned_bytes(writer: &mut CanonicalWriter, bytes: &[u8]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("owned canonical bytes must fit the canonical protocol");
    }
}

#[cfg(test)]
mod tests {
    use world_core::{CanonicalDomain, CanonicalWriter};

    use crate::authority::{AuthorityCursor, EpochIdentity};

    use super::*;
    use crate::execution::InitialStateRootId;

    const FIXTURE_DOMAIN: CanonicalDomain = match CanonicalDomain::new("lineage-fixture-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("lineage fixture domain must be valid"),
    };

    fn semantic_body(value: u8) -> CanonicalBytes {
        let mut writer = CanonicalWriter::new(FIXTURE_DOMAIN);
        writer.write_u8(value);
        writer.finish()
    }

    fn parent_cursor() -> AuthorityCursor {
        AuthorityCursor::root(
            EpochIdentity::new(
                EpochLineageId::from_bytes([0x31; 32]),
                ExecutionSpecId::from_bytes([0x32; 32]),
            ),
            InitialStateRootId::from_bytes([0x33; 32]),
        )
    }

    #[test]
    fn origin_identity_is_derived_before_lineage_identity() {
        let first = EpochLineage::origin(&semantic_body(1));
        let same = EpochLineage::origin(&semantic_body(1));
        let changed = EpochLineage::origin(&semantic_body(2));

        assert_eq!(first, same);
        assert_ne!(first.id(), changed.id());
        let EpochLineageBody::Origin { origin } = first.body() else {
            panic!("origin construction must retain an origin body");
        };
        assert_ne!(origin.as_bytes(), first.id().as_bytes());
        assert_eq!(
            epoch_origin_bytes(semantic_body(1).as_bytes()).as_bytes(),
            b"world-canonical-v1\x00\x00\x00\x00\x00\x00\x00\x0fepoch-origin-v1\x00\x01\x00\x00\x00\x00\x00\x00\x00-world-canonical-v1\x00\x00\x00\x00\x00\x00\x00\x12lineage-fixture-v1\x01"
        );
        assert_eq!(
            origin.to_string(),
            "531508f6dcc049098e6f25059a8cda8de63f17b54e422a4e1b3f418dc47accce"
        );
        assert_eq!(
            first.canonical_bytes().as_bytes(),
            b"world-canonical-v1\x00\x00\x00\x00\x00\x00\x00\x10epoch-lineage-v1\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00 S\x15\x08\xf6\xdc\xc0I\x09\x8eo%\x05\x9a\x8c\xda\x8d\xe6?\x17\xb5NB*N\x1b?A\x8d\xc4z\xcc\xce"
        );
        assert_eq!(
            first.id().to_string(),
            "7ccc156d71de7e9b29b9b5a353168638f2cc59de939ebe6b8c00aa8d5e15c6b0"
        );
        assert_eq!(
            first.validate_semantic_correspondence(&semantic_body(1)),
            Ok(())
        );
    }

    #[test]
    fn child_lineage_commits_the_complete_parent_cursor_and_transform() {
        let parent = parent_cursor();
        let branch = EpochLineage::child(
            parent,
            ChildEpochTransform::Branch(BranchTransformId::from_bytes([0x41; 32])),
        );
        let changed_branch = EpochLineage::child(
            parent,
            ChildEpochTransform::Branch(BranchTransformId::from_bytes([0x42; 32])),
        );
        let migration = EpochLineage::child(
            parent,
            ChildEpochTransform::MigrationReset(MigrationResetId::from_bytes([0x41; 32])),
        );
        let changed_parent_lineage = EpochLineage::child(
            AuthorityCursor::root(
                EpochIdentity::new(
                    EpochLineageId::from_bytes([0x30; 32]),
                    ExecutionSpecId::from_bytes([0x32; 32]),
                ),
                InitialStateRootId::from_bytes([0x33; 32]),
            ),
            ChildEpochTransform::Branch(BranchTransformId::from_bytes([0x41; 32])),
        );
        let changed_parent_execution = EpochLineage::child(
            AuthorityCursor::root(
                EpochIdentity::new(
                    EpochLineageId::from_bytes([0x31; 32]),
                    ExecutionSpecId::from_bytes([0x34; 32]),
                ),
                InitialStateRootId::from_bytes([0x33; 32]),
            ),
            ChildEpochTransform::Branch(BranchTransformId::from_bytes([0x41; 32])),
        );
        let changed_parent_position = EpochLineage::child(
            AuthorityCursor::root(
                EpochIdentity::new(
                    EpochLineageId::from_bytes([0x31; 32]),
                    ExecutionSpecId::from_bytes([0x32; 32]),
                ),
                InitialStateRootId::from_bytes([0x34; 32]),
            ),
            ChildEpochTransform::Branch(BranchTransformId::from_bytes([0x41; 32])),
        );

        assert_ne!(branch.id(), changed_branch.id());
        assert_ne!(branch.id(), migration.id());
        assert_ne!(branch.id(), changed_parent_lineage.id());
        assert_ne!(branch.id(), changed_parent_execution.id());
        assert_ne!(branch.id(), changed_parent_position.id());
        let EpochLineageBody::Child {
            parent_execution,
            parent_cursor,
            ..
        } = branch.body()
        else {
            panic!("child construction must retain a child body");
        };
        assert_eq!(parent_execution, parent.epoch().execution());
        assert_eq!(parent_cursor, parent);
        assert_eq!(
            branch.canonical_bytes().as_bytes(),
            b"world-canonical-v1\x00\x00\x00\x00\x00\x00\x00\x10epoch-lineage-v1\x00\x01\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00 22222222222222222222222222222222\x00\x00\x00\x00\x00\x00\x00\xd3world-canonical-v1\x00\x00\x00\x00\x00\x00\x00\x13authority-cursor-v1\x00\x01\x00\x00\x00\x00\x00\x00\x00 11111111111111111111111111111111\x00\x00\x00\x00\x00\x00\x00 22222222222222222222222222222222\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00 \xde\xb12\x1e/o\x0c\xe4\xde\x0a\x08\x9aF%C\xb1\x96\x15u`\xfdw\x98pR\x11\x18\xc9\xca\xb9s\xbc\x00\x00\x00\x00\x00\x00\x00 \xeb\x01\xd1\xceg\xc1)\xf5\xf95!\xe4\xd6\xb7\x9bO\xe2\x03c\xbb\xc9\xec0\x02f\xdbe\x193\xca\x84\x9c\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00 AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        );
        assert_eq!(
            branch.id().to_string(),
            "bf2cdc357dc31cb5da4528ec66662425d793a4b0cd04fd20998c5d7d5a038844"
        );
        assert_eq!(
            branch.validate_semantic_correspondence(&semantic_body(1)),
            Ok(())
        );
    }

    #[test]
    fn correspondence_rejects_an_origin_for_a_different_semantic_body() {
        let original_body = semantic_body(1);
        let actual = EpochOriginId::of_canonical(&epoch_origin_bytes(semantic_body(2).as_bytes()));
        let lineage = EpochLineage::fixture_from_body(EpochLineageBody::Origin { origin: actual });
        let expected = EpochOriginId::of_canonical(&epoch_origin_bytes(original_body.as_bytes()));

        assert_eq!(
            lineage.validate_semantic_correspondence(&original_body),
            Err(EpochLineageCorrespondenceError::OriginSemanticBodyMismatch { expected, actual })
        );
    }

    #[test]
    fn correspondence_rejects_a_child_with_a_different_parent_execution() {
        let parent_cursor = parent_cursor();
        let actual = ExecutionSpecId::from_bytes([0x35; 32]);
        let lineage = EpochLineage::fixture_from_body(EpochLineageBody::Child {
            parent_execution: actual,
            parent_cursor,
            transform: ChildEpochTransform::Branch(BranchTransformId::from_bytes([0x41; 32])),
        });
        let expected = parent_cursor.epoch().execution();

        assert_eq!(
            lineage.validate_semantic_correspondence(&semantic_body(1)),
            Err(EpochLineageCorrespondenceError::ChildParentExecutionMismatch { expected, actual })
        );
    }
}
