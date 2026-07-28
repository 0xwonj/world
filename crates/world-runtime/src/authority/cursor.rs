use core::num::NonZeroU64;

use world_core::{
    CanonicalBytes, CanonicalDomain, CanonicalWriter, NonZeroWorldRevision, WorldRevision,
};

use crate::execution::{EpochLineageId, ExecutionSpecId, InitialStateRootId};

use super::{
    AuthorityRecordAnchor, AuthorityRecordId, CumulativeAuthorityHash, PreviousAuthorityHash,
};

/// Why a cursor could not advance to another published record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthorityCursorAdvanceError {
    /// The world revision coordinate reached its maximum value.
    RevisionOverflow,
    /// The outer authority-record sequence reached its maximum value.
    RecordSequenceOverflow,
}

/// One checked derivation of the coordinates required by the next record.
///
/// Sealing consumes this plan so the header, record preimage, and resulting
/// cursor cannot perform independent predecessor calculations.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AuthoritySuccessorPlan {
    epoch: EpochIdentity,
    revision: NonZeroWorldRevision,
    sequence: NonZeroRunRecordSeq,
    previous_authority: PreviousAuthorityHash,
    previous_cumulative: CumulativeAuthorityHash,
}

impl AuthoritySuccessorPlan {
    pub(crate) const fn lineage(&self) -> EpochLineageId {
        self.epoch.lineage()
    }

    pub(crate) const fn revision(&self) -> NonZeroWorldRevision {
        self.revision
    }

    pub(crate) const fn sequence(&self) -> NonZeroRunRecordSeq {
        self.sequence
    }

    pub(crate) const fn previous_authority(&self) -> PreviousAuthorityHash {
        self.previous_authority
    }

    pub(crate) const fn previous_cumulative(&self) -> CumulativeAuthorityHash {
        self.previous_cumulative
    }

    pub(crate) fn finish(
        self,
        record: AuthorityRecordId,
        cumulative: CumulativeAuthorityHash,
    ) -> AuthorityCursor {
        AuthorityCursor {
            epoch: self.epoch,
            position: AuthorityPosition::Record {
                revision: self.revision,
                sequence: self.sequence,
                record,
                cumulative,
            },
        }
    }
}

/// Canonical schema of a complete authority cursor.
pub const AUTHORITY_CURSOR_SCHEMA_VERSION: u16 = 1;

const EPOCH_RECORD_ANCHOR_SCHEMA_VERSION: u16 = 1;
const EPOCH_CUMULATIVE_ANCHOR_SCHEMA_VERSION: u16 = 1;

const EPOCH_RECORD_ANCHOR_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("epoch-record-anchor-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("epoch record anchor domain must be valid"),
    };

const EPOCH_CUMULATIVE_ANCHOR_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("epoch-cumulative-anchor-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("epoch cumulative anchor domain must be valid"),
    };

const AUTHORITY_CURSOR_DOMAIN: CanonicalDomain = match CanonicalDomain::new("authority-cursor-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("authority cursor domain must be valid"),
};

/// Immutable semantic identity shared by every cursor in one execution epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpochIdentity {
    lineage: EpochLineageId,
    execution: ExecutionSpecId,
}

impl EpochIdentity {
    pub(crate) const fn new(lineage: EpochLineageId, execution: ExecutionSpecId) -> Self {
        Self { lineage, execution }
    }

    /// Returns the semantic lineage identity.
    #[must_use]
    pub const fn lineage(self) -> EpochLineageId {
        self.lineage
    }

    /// Returns the exact execution specification identity.
    #[must_use]
    pub const fn execution(self) -> ExecutionSpecId {
        self.execution
    }
}

/// Nonzero sequence assigned to a published outer authority record.
///
/// The epoch root has no record sequence and is represented by a distinct
/// cursor variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonZeroRunRecordSeq(NonZeroU64);

impl NonZeroRunRecordSeq {
    /// Constructs a record sequence when the supplied value is nonzero.
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the numeric record sequence.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Returns the next nonzero record sequence, or `None` on overflow.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.get().checked_add(1) {
            Some(value) => Self::new(value),
            None => None,
        }
    }
}

/// Exact position of an authority cursor within its execution epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthorityPosition {
    /// Distinguished position before the first authority record.
    Root {
        /// Predecessor identity used by the first authority record.
        record_anchor: AuthorityRecordAnchor,
        /// Cumulative-history identity before the first authority record.
        cumulative_anchor: CumulativeAuthorityHash,
    },
    /// Position produced by one published authority record.
    Record {
        /// Nonzero authoritative session revision produced by the record.
        revision: NonZeroWorldRevision,
        /// Nonzero sequence assigned to the outer record.
        sequence: NonZeroRunRecordSeq,
        /// Canonical identity of the outer record.
        record: AuthorityRecordId,
        /// Cumulative identity of the complete history prefix.
        cumulative: CumulativeAuthorityHash,
    },
}

impl AuthorityPosition {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Root { .. } => 0,
            Self::Record { .. } => 1,
        }
    }
}

/// Complete compare-and-set coordinate of authoritative world history.
///
/// The epoch root and a published-record position are structurally distinct;
/// neither is represented by sentinel scalars or optional record fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthorityCursor {
    epoch: EpochIdentity,
    position: AuthorityPosition,
}

impl AuthorityCursor {
    pub(crate) fn root(epoch: EpochIdentity, initial_root: InitialStateRootId) -> Self {
        let record_anchor = AuthorityRecordAnchor::of_canonical(&epoch_record_anchor_bytes(
            initial_root,
            epoch.execution,
        ));
        let cumulative_anchor = CumulativeAuthorityHash::of_canonical(
            &epoch_cumulative_anchor_bytes(initial_root, epoch.execution),
        );

        Self {
            epoch,
            position: AuthorityPosition::Root {
                record_anchor,
                cumulative_anchor,
            },
        }
    }

    /// Returns the immutable epoch/execution binding.
    #[must_use]
    pub const fn epoch(self) -> EpochIdentity {
        self.epoch
    }

    /// Returns the structurally typed history position.
    #[must_use]
    pub const fn position(self) -> AuthorityPosition {
        self.position
    }

    /// Returns the world revision represented by this cursor.
    #[must_use]
    pub const fn revision(self) -> WorldRevision {
        match self.position {
            AuthorityPosition::Root { .. } => WorldRevision::ROOT,
            AuthorityPosition::Record { revision, .. } => WorldRevision::from_raw(revision.get()),
        }
    }

    /// Returns the cumulative identity of the represented history prefix.
    #[must_use]
    pub const fn cumulative(self) -> CumulativeAuthorityHash {
        match self.position {
            AuthorityPosition::Root {
                cumulative_anchor, ..
            } => cumulative_anchor,
            AuthorityPosition::Record { cumulative, .. } => cumulative,
        }
    }

    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        authority_cursor_bytes(self)
    }

    pub(crate) fn successor_plan(
        self,
    ) -> Result<AuthoritySuccessorPlan, AuthorityCursorAdvanceError> {
        let (revision, sequence, previous_authority, previous_cumulative) = match self.position {
            AuthorityPosition::Root {
                record_anchor,
                cumulative_anchor,
            } => (
                WorldRevision::ROOT
                    .checked_next()
                    .ok_or(AuthorityCursorAdvanceError::RevisionOverflow)?,
                NonZeroRunRecordSeq::new(1)
                    .ok_or(AuthorityCursorAdvanceError::RecordSequenceOverflow)?,
                PreviousAuthorityHash::from_root_anchor(record_anchor),
                cumulative_anchor,
            ),
            AuthorityPosition::Record {
                revision,
                sequence,
                record,
                cumulative,
            } => (
                revision
                    .checked_next()
                    .ok_or(AuthorityCursorAdvanceError::RevisionOverflow)?,
                sequence
                    .checked_next()
                    .ok_or(AuthorityCursorAdvanceError::RecordSequenceOverflow)?,
                PreviousAuthorityHash::from_record(record),
                cumulative,
            ),
        };

        Ok(AuthoritySuccessorPlan {
            epoch: self.epoch,
            revision,
            sequence,
            previous_authority,
            previous_cumulative,
        })
    }
}

fn epoch_record_anchor_bytes(
    initial_root: InitialStateRootId,
    execution: ExecutionSpecId,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(EPOCH_RECORD_ANCHOR_DOMAIN);
    writer.write_u16(EPOCH_RECORD_ANCHOR_SCHEMA_VERSION);
    write_fixed_bytes(&mut writer, initial_root.as_bytes());
    write_fixed_bytes(&mut writer, execution.as_bytes());
    writer.finish()
}

fn epoch_cumulative_anchor_bytes(
    initial_root: InitialStateRootId,
    execution: ExecutionSpecId,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(EPOCH_CUMULATIVE_ANCHOR_DOMAIN);
    writer.write_u16(EPOCH_CUMULATIVE_ANCHOR_SCHEMA_VERSION);
    write_fixed_bytes(&mut writer, initial_root.as_bytes());
    write_fixed_bytes(&mut writer, execution.as_bytes());
    writer.finish()
}

fn authority_cursor_bytes(cursor: AuthorityCursor) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(AUTHORITY_CURSOR_DOMAIN);
    writer.write_u16(AUTHORITY_CURSOR_SCHEMA_VERSION);
    write_epoch_identity(&mut writer, cursor.epoch);
    writer.write_discriminant(cursor.position.canonical_tag());
    match cursor.position {
        AuthorityPosition::Root {
            record_anchor,
            cumulative_anchor,
        } => {
            write_fixed_bytes(&mut writer, record_anchor.as_bytes());
            write_fixed_bytes(&mut writer, cumulative_anchor.as_bytes());
        }
        AuthorityPosition::Record {
            revision,
            sequence,
            record,
            cumulative,
        } => {
            writer.write_u64(revision.get());
            writer.write_u64(sequence.get());
            write_fixed_bytes(&mut writer, record.as_bytes());
            write_fixed_bytes(&mut writer, cumulative.as_bytes());
        }
    }
    writer.finish()
}

fn write_epoch_identity(writer: &mut CanonicalWriter, epoch: EpochIdentity) {
    write_fixed_bytes(writer, epoch.lineage.as_bytes());
    write_fixed_bytes(writer, epoch.execution.as_bytes());
}

fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width identity length must fit the canonical protocol");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch() -> EpochIdentity {
        EpochIdentity::new(
            EpochLineageId::from_bytes([0x11; 32]),
            ExecutionSpecId::from_bytes([0x22; 32]),
        )
    }

    fn nonzero_revision(value: u64) -> NonZeroWorldRevision {
        match NonZeroWorldRevision::new(value) {
            Some(revision) => revision,
            None => panic!("test revision must be nonzero"),
        }
    }

    fn nonzero_sequence(value: u64) -> NonZeroRunRecordSeq {
        match NonZeroRunRecordSeq::new(value) {
            Some(sequence) => sequence,
            None => panic!("test sequence must be nonzero"),
        }
    }

    fn record_cursor(
        lineage: u8,
        execution: u8,
        revision: u64,
        sequence: u64,
        record: u8,
        cumulative: u8,
    ) -> AuthorityCursor {
        AuthorityCursor {
            epoch: EpochIdentity::new(
                EpochLineageId::from_bytes([lineage; 32]),
                ExecutionSpecId::from_bytes([execution; 32]),
            ),
            position: AuthorityPosition::Record {
                revision: nonzero_revision(revision),
                sequence: nonzero_sequence(sequence),
                record: AuthorityRecordId::from_bytes([record; 32]),
                cumulative: CumulativeAuthorityHash::from_bytes([cumulative; 32]),
            },
        }
    }

    fn root_anchors(cursor: AuthorityCursor) -> (AuthorityRecordAnchor, CumulativeAuthorityHash) {
        match cursor.position() {
            AuthorityPosition::Root {
                record_anchor,
                cumulative_anchor,
            } => (record_anchor, cumulative_anchor),
            AuthorityPosition::Record { .. } => {
                panic!("root construction must produce a root position")
            }
        }
    }

    fn hex(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            use core::fmt::Write as _;
            if write!(&mut output, "{byte:02x}").is_err() {
                unreachable!("writing to a string cannot fail");
            }
        }
        output
    }

    #[test]
    fn root_anchors_bind_root_and_execution_under_separate_domains() {
        let root_id = InitialStateRootId::from_bytes([0x33; 32]);
        let cursor = AuthorityCursor::root(epoch(), root_id);
        let same = AuthorityCursor::root(epoch(), root_id);
        let other_root = AuthorityCursor::root(epoch(), InitialStateRootId::from_bytes([0x34; 32]));
        let other_execution = AuthorityCursor::root(
            EpochIdentity::new(
                EpochLineageId::from_bytes([0x11; 32]),
                ExecutionSpecId::from_bytes([0x23; 32]),
            ),
            root_id,
        );

        assert_eq!(cursor, same);
        assert_ne!(cursor, other_root);
        assert_ne!(cursor, other_execution);

        let (record_anchor, cumulative_anchor) = root_anchors(cursor);
        let (other_root_record, other_root_cumulative) = root_anchors(other_root);
        let (other_execution_record, other_execution_cumulative) = root_anchors(other_execution);

        assert_ne!(record_anchor.as_bytes(), cumulative_anchor.as_bytes());
        assert_ne!(record_anchor, other_root_record);
        assert_ne!(cumulative_anchor, other_root_cumulative);
        assert_ne!(record_anchor, other_execution_record);
        assert_ne!(cumulative_anchor, other_execution_cumulative);
        assert_eq!(cursor.revision(), WorldRevision::ROOT);
        assert_eq!(cursor.cumulative(), cumulative_anchor);
    }

    #[test]
    fn root_cursor_retains_lineage_beyond_its_derived_anchors() {
        let root_id = InitialStateRootId::from_bytes([0x33; 32]);
        let cursor = AuthorityCursor::root(epoch(), root_id);
        let other_lineage = AuthorityCursor::root(
            EpochIdentity::new(
                EpochLineageId::from_bytes([0x12; 32]),
                ExecutionSpecId::from_bytes([0x22; 32]),
            ),
            root_id,
        );

        assert_eq!(root_anchors(cursor), root_anchors(other_lineage));
        assert_ne!(cursor, other_lineage);
        assert_ne!(cursor.canonical_bytes(), other_lineage.canonical_bytes());
    }

    #[test]
    fn root_and_record_positions_are_structurally_distinct() {
        let root = AuthorityCursor::root(epoch(), InitialStateRootId::from_bytes([0x33; 32]));
        let record = record_cursor(0x11, 0x22, 1, 1, 0x44, 0x55);

        assert!(matches!(root.position(), AuthorityPosition::Root { .. }));
        assert!(matches!(
            record.position(),
            AuthorityPosition::Record { .. }
        ));
        assert_eq!(record.revision(), WorldRevision::from_raw(1));
        assert_ne!(root.canonical_bytes(), record.canonical_bytes());
    }

    #[test]
    fn full_cursor_preimage_is_sensitive_to_every_coordinate() {
        let base = record_cursor(0x11, 0x22, 7, 9, 0x44, 0x55);
        let base_bytes = base.canonical_bytes();

        for changed in [
            record_cursor(0x12, 0x22, 7, 9, 0x44, 0x55),
            record_cursor(0x11, 0x23, 7, 9, 0x44, 0x55),
            record_cursor(0x11, 0x22, 8, 9, 0x44, 0x55),
            record_cursor(0x11, 0x22, 7, 10, 0x44, 0x55),
            record_cursor(0x11, 0x22, 7, 9, 0x45, 0x55),
            record_cursor(0x11, 0x22, 7, 9, 0x44, 0x56),
        ] {
            assert_ne!(base_bytes, changed.canonical_bytes());
        }
    }

    #[test]
    fn anchor_and_cursor_preimages_have_frozen_vectors() {
        const RECORD_ANCHOR_PREIMAGE: &str = "776f726c642d63616e6f6e6963616c2d7631000000000000001665706f63682d7265636f72642d616e63686f722d763100010000000000000020333333333333333333333333333333333333333333333333333333333333333300000000000000202222222222222222222222222222222222222222222222222222222222222222";
        const CUMULATIVE_ANCHOR_PREIMAGE: &str = "776f726c642d63616e6f6e6963616c2d7631000000000000001a65706f63682d63756d756c61746976652d616e63686f722d763100010000000000000020333333333333333333333333333333333333333333333333333333333333333300000000000000202222222222222222222222222222222222222222222222222222222222222222";
        const ROOT_CURSOR_PREIMAGE: &str = "776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d637572736f722d763100010000000000000020111111111111111111111111111111111111111111111111111111111111111100000000000000202222222222222222222222222222222222222222222222222222222222222222000000000000000000000020fe287a7e292e65237924b865f0206f169983c3087b66f66018f339fcaf6ca6a10000000000000020ed9d9541455faf2439fa9e45e8f039e79cac4f417b603ad66b44214fe4cd6580";
        const RECORD_CURSOR_PREIMAGE: &str = "776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d637572736f722d76310001000000000000002011111111111111111111111111111111111111111111111111111111111111110000000000000020222222222222222222222222222222222222222222222222222222222222222200000001000000000000000700000000000000090000000000000020444444444444444444444444444444444444444444444444444444444444444400000000000000205555555555555555555555555555555555555555555555555555555555555555";

        let root_id = InitialStateRootId::from_bytes([0x33; 32]);
        let root = AuthorityCursor::root(epoch(), root_id);
        let record = record_cursor(0x11, 0x22, 7, 9, 0x44, 0x55);

        let record_anchor_bytes = epoch_record_anchor_bytes(root_id, epoch().execution());
        let cumulative_anchor_bytes = epoch_cumulative_anchor_bytes(root_id, epoch().execution());

        let (record_anchor, cumulative_anchor) = root_anchors(root);

        assert_eq!(hex(record_anchor_bytes.as_bytes()), RECORD_ANCHOR_PREIMAGE);
        assert_eq!(
            hex(cumulative_anchor_bytes.as_bytes()),
            CUMULATIVE_ANCHOR_PREIMAGE
        );
        assert_eq!(
            record_anchor.to_string(),
            "fe287a7e292e65237924b865f0206f169983c3087b66f66018f339fcaf6ca6a1"
        );
        assert_eq!(
            cumulative_anchor.to_string(),
            "ed9d9541455faf2439fa9e45e8f039e79cac4f417b603ad66b44214fe4cd6580"
        );
        assert_eq!(hex(root.canonical_bytes().as_bytes()), ROOT_CURSOR_PREIMAGE);
        assert_eq!(
            hex(record.canonical_bytes().as_bytes()),
            RECORD_CURSOR_PREIMAGE
        );
    }

    #[test]
    fn record_sequence_rejects_zero_and_detects_overflow() {
        assert_eq!(NonZeroRunRecordSeq::new(0), None);
        assert_eq!(nonzero_sequence(1).get(), 1);
        assert_eq!(nonzero_sequence(u64::MAX).checked_next(), None);
    }

    #[test]
    fn record_advancement_derives_coordinates_and_reports_each_overflow() {
        let root = AuthorityCursor::root(epoch(), InitialStateRootId::from_bytes([0x33; 32]));
        let plan = match root.successor_plan() {
            Ok(plan) => plan,
            Err(error) => panic!("root cursor must admit a first record: {error:?}"),
        };
        assert_eq!(plan.lineage(), epoch().lineage());
        assert_eq!(plan.revision(), nonzero_revision(1));
        assert_eq!(plan.sequence(), nonzero_sequence(1));
        let AuthorityPosition::Root {
            record_anchor,
            cumulative_anchor,
        } = root.position()
        else {
            panic!("fixture must be a root cursor");
        };
        assert_eq!(
            plan.previous_authority(),
            PreviousAuthorityHash::from_root_anchor(record_anchor)
        );
        assert_eq!(plan.previous_cumulative(), cumulative_anchor);

        let first = plan.finish(
            AuthorityRecordId::from_bytes([0x44; 32]),
            CumulativeAuthorityHash::from_bytes([0x55; 32]),
        );

        assert_eq!(
            first.position(),
            AuthorityPosition::Record {
                revision: nonzero_revision(1),
                sequence: nonzero_sequence(1),
                record: AuthorityRecordId::from_bytes([0x44; 32]),
                cumulative: CumulativeAuthorityHash::from_bytes([0x55; 32]),
            }
        );
        assert_eq!(
            record_cursor(0x11, 0x22, u64::MAX, 1, 0x44, 0x55).successor_plan(),
            Err(AuthorityCursorAdvanceError::RevisionOverflow)
        );
        assert_eq!(
            record_cursor(0x11, 0x22, 1, u64::MAX, 0x44, 0x55).successor_plan(),
            Err(AuthorityCursorAdvanceError::RecordSequenceOverflow)
        );
    }
}
