use core::fmt;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter, ContentDigest};

const CAPTURED_INPUT_RECORD_SCHEMA_VERSION: u16 = 1;
const ATTEMPT_RECORD_SCHEMA_VERSION: u16 = 1;
const COMMIT_RECORD_SCHEMA_VERSION: u16 = 1;
const REACTION_ENVELOPE_SCHEMA_VERSION: u16 = 1;

const CAPTURED_INPUT_RECORD_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("captured-input-record-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("captured-input record domain must be valid"),
    };

const ATTEMPT_RECORD_DOMAIN: CanonicalDomain = match CanonicalDomain::new("attempt-record-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("attempt record domain must be valid"),
};

const COMMIT_RECORD_DOMAIN: CanonicalDomain = match CanonicalDomain::new("commit-record-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("commit record domain must be valid"),
};

const REACTION_ENVELOPE_DOMAIN: CanonicalDomain = match CanonicalDomain::new("reaction-envelope-v1")
{
    Ok(domain) => domain,
    Err(_) => panic!("reaction envelope domain must be valid"),
};

const CAPTURED_INPUT_LOCAL_KIND_TAG: u32 = 0;
const ATTEMPT_LOCAL_KIND_TAG: u32 = 1;
const COMMIT_LOCAL_KIND_TAG: u32 = 2;
const REACTION_LOCAL_KIND_TAG: u32 = 3;
const CURRENT_RECORD_ROLE_TAG: u32 = 0;

macro_rules! authority_identity {
    ($(#[$metadata:meta])* $name:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Constructs a fixed-width identity decoded by its authority
            /// protocol owner.
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

            pub(crate) fn of_canonical(bytes: &CanonicalBytes) -> Self {
                Self(ContentDigest::of_canonical(bytes).into_bytes())
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

authority_identity!(
    /// Distinguished predecessor anchor before an epoch's first record.
    AuthorityRecordAnchor
);
authority_identity!(
    /// Canonical identity of one published outer authority record.
    AuthorityRecordId
);
authority_identity!(
    /// Cumulative hash of an authoritative history prefix.
    CumulativeAuthorityHash
);

macro_rules! local_index {
    ($(#[$metadata:meta])* $name:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub(crate) struct $name(u32);

        impl $name {
            /// Constructs a zero-based canonical local coordinate.
            #[must_use]
            pub(crate) const fn new(value: u32) -> Self {
                Self(value)
            }

            /// Returns the exact zero-based canonical coordinate.
            #[must_use]
            pub(crate) const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

local_index!(
    /// Canonical position of a captured input within its owning authority record.
    CapturedInputLocalIndex
);
local_index!(
    /// Canonical position of a command attempt within its owning authority record.
    AttemptLocalIndex
);
local_index!(
    /// Canonical position of an accepted commit within its owning authority record.
    CommitLocalIndex
);
local_index!(
    /// Canonical position of a reaction envelope within its owning authority record.
    ReactionLocalIndex
);

macro_rules! inner_record_identity {
    ($(#[$metadata:meta])* $name:ident, $index:ident, $derive:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Constructs a fixed-width identity decoded by the enclosing
            /// authority protocol.
            #[cfg(test)]
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Derives the identity from its outer record and canonical local
            /// coordinate.
            #[must_use]
            pub(crate) fn derive(owner: AuthorityRecordId, index: $index) -> Self {
                Self(
                    ContentDigest::of_canonical(&$derive(owner, index))
                        .into_bytes(),
                )
            }

            /// Consumes the identity and returns its exact bytes.
            #[cfg(test)]
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

inner_record_identity!(
    /// Canonical provenance identity of one captured input.
    CapturedInputRecordId,
    CapturedInputLocalIndex,
    captured_input_record_id_bytes
);
inner_record_identity!(
    /// Canonical provenance identity of one command attempt.
    AttemptRecordId,
    AttemptLocalIndex,
    attempt_record_id_bytes
);
inner_record_identity!(
    /// Canonical provenance identity of one accepted commit.
    CommitRecordId,
    CommitLocalIndex,
    commit_record_id_bytes
);
inner_record_identity!(
    /// Canonical provenance identity of one reaction envelope.
    ReactionEnvelopeId,
    ReactionLocalIndex,
    reaction_envelope_id_bytes
);

impl CapturedInputRecordId {
    #[must_use]
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl AttemptRecordId {
    #[must_use]
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[cfg(test)]
impl CommitRecordId {
    #[must_use]
    const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[cfg(test)]
impl ReactionEnvelopeId {
    #[must_use]
    const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Previous outer-authority identity in an authority-record preimage.
///
/// Root-anchor and prior-record sources share this private role. Its canonical
/// representation is exactly the source identity bytes, without a source tag.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PreviousAuthorityHash([u8; 32]);

impl PreviousAuthorityHash {
    #[must_use]
    pub(crate) const fn from_root_anchor(anchor: AuthorityRecordAnchor) -> Self {
        Self(*anchor.as_bytes())
    }

    #[must_use]
    pub(crate) const fn from_record(record: AuthorityRecordId) -> Self {
        Self(*record.as_bytes())
    }

    #[must_use]
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for PreviousAuthorityHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for PreviousAuthorityHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "PreviousAuthorityHash({self})")
    }
}

macro_rules! same_record_reference {
    ($(#[$metadata:meta])* $name:ident, $index:ident, $kind_tag:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub(crate) struct $name($index);

        impl $name {
            /// Selects one value in the enclosing authority record.
            #[must_use]
            pub(crate) const fn new(index: $index) -> Self {
                Self(index)
            }

            /// Returns the typed local coordinate.
            #[cfg(test)]
            #[must_use]
            pub(crate) const fn index(self) -> $index {
                self.0
            }

            pub(crate) fn write_canonical(self, writer: &mut CanonicalWriter) {
                writer.write_discriminant($kind_tag);
                writer.write_u32(self.0.get());
            }
        }
    };
}

same_record_reference!(
    /// Reference to a captured input in the enclosing authority record.
    SameRecordCapturedInputRef,
    CapturedInputLocalIndex,
    CAPTURED_INPUT_LOCAL_KIND_TAG
);
same_record_reference!(
    /// Reference to a command attempt in the enclosing authority record.
    SameRecordAttemptRef,
    AttemptLocalIndex,
    ATTEMPT_LOCAL_KIND_TAG
);
same_record_reference!(
    /// Reference to an accepted commit in the enclosing authority record.
    SameRecordCommitRef,
    CommitLocalIndex,
    COMMIT_LOCAL_KIND_TAG
);
same_record_reference!(
    /// Reference to a reaction envelope in the enclosing authority record.
    SameRecordReactionRef,
    ReactionLocalIndex,
    REACTION_LOCAL_KIND_TAG
);

/// Distinguished reference to the authority record whose body is being
/// encoded.
///
/// This is a one-value role protocol, not an outer-record identity or a
/// sentinel local coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CurrentRecordRef(());

impl CurrentRecordRef {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self(())
    }

    pub(crate) fn write_canonical(self, writer: &mut CanonicalWriter) {
        writer.write_discriminant(CURRENT_RECORD_ROLE_TAG);
    }
}

fn captured_input_record_id_bytes(
    owner: AuthorityRecordId,
    index: CapturedInputLocalIndex,
) -> CanonicalBytes {
    inner_record_id_bytes(
        CAPTURED_INPUT_RECORD_DOMAIN,
        CAPTURED_INPUT_RECORD_SCHEMA_VERSION,
        owner,
        index.get(),
    )
}

fn attempt_record_id_bytes(owner: AuthorityRecordId, index: AttemptLocalIndex) -> CanonicalBytes {
    inner_record_id_bytes(
        ATTEMPT_RECORD_DOMAIN,
        ATTEMPT_RECORD_SCHEMA_VERSION,
        owner,
        index.get(),
    )
}

fn commit_record_id_bytes(owner: AuthorityRecordId, index: CommitLocalIndex) -> CanonicalBytes {
    inner_record_id_bytes(
        COMMIT_RECORD_DOMAIN,
        COMMIT_RECORD_SCHEMA_VERSION,
        owner,
        index.get(),
    )
}

fn reaction_envelope_id_bytes(
    owner: AuthorityRecordId,
    index: ReactionLocalIndex,
) -> CanonicalBytes {
    inner_record_id_bytes(
        REACTION_ENVELOPE_DOMAIN,
        REACTION_ENVELOPE_SCHEMA_VERSION,
        owner,
        index.get(),
    )
}

fn inner_record_id_bytes(
    domain: CanonicalDomain,
    schema_version: u16,
    owner: AuthorityRecordId,
    index: u32,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(domain);
    writer.write_u16(schema_version);
    if writer.write_bytes(owner.as_bytes()).is_err() {
        unreachable!("fixed-width authority record identity must fit the canonical protocol");
    }
    writer.write_u32(index);
    writer.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    const REFERENCE_VECTOR_DOMAIN: CanonicalDomain =
        match CanonicalDomain::new("authority-reference-vector-v1") {
            Ok(domain) => domain,
            Err(_) => panic!("authority-reference vector domain must be valid"),
        };

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
            encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        encoded
    }

    #[test]
    fn inner_record_identities_match_byte_complete_vectors() {
        let owner = AuthorityRecordId::from_bytes([0x11; 32]);
        let captured_index = CapturedInputLocalIndex::new(0);
        let attempt_index = AttemptLocalIndex::new(1);
        let commit_index = CommitLocalIndex::new(2);
        let reaction_index = ReactionLocalIndex::new(3);

        let captured_bytes = captured_input_record_id_bytes(owner, captured_index);
        let attempt_bytes = attempt_record_id_bytes(owner, attempt_index);
        let commit_bytes = commit_record_id_bytes(owner, commit_index);
        let reaction_bytes = reaction_envelope_id_bytes(owner, reaction_index);

        assert_eq!(
            [
                (
                    hex(captured_bytes.as_bytes()),
                    CapturedInputRecordId::derive(owner, captured_index).to_string(),
                ),
                (
                    hex(attempt_bytes.as_bytes()),
                    AttemptRecordId::derive(owner, attempt_index).to_string(),
                ),
                (
                    hex(commit_bytes.as_bytes()),
                    CommitRecordId::derive(owner, commit_index).to_string(),
                ),
                (
                    hex(reaction_bytes.as_bytes()),
                    ReactionEnvelopeId::derive(owner, reaction_index).to_string(),
                ),
            ],
            [
                (
                    "776f726c642d63616e6f6e6963616c2d7631000000000000001863617074757265642d696e7075742d7265636f72642d763100010000000000000020111111111111111111111111111111111111111111111111111111111111111100000000".to_owned(),
                    "6a17492a2cee9124f97ab25f0f4a552445a7271902617191d313a817b94afd68".to_owned(),
                ),
                (
                    "776f726c642d63616e6f6e6963616c2d76310000000000000011617474656d70742d7265636f72642d763100010000000000000020111111111111111111111111111111111111111111111111111111111111111100000001".to_owned(),
                    "344e79a4ccf891690c8b96ebfc048b82e1ef2ba3795a77780949983cf8cabefd".to_owned(),
                ),
                (
                    "776f726c642d63616e6f6e6963616c2d76310000000000000010636f6d6d69742d7265636f72642d763100010000000000000020111111111111111111111111111111111111111111111111111111111111111100000002".to_owned(),
                    "7694401087638d0804526e1af5f94ce2a3484632bd87c8c1e800e416b0a4de83".to_owned(),
                ),
                (
                    "776f726c642d63616e6f6e6963616c2d763100000000000000147265616374696f6e2d656e76656c6f70652d763100010000000000000020111111111111111111111111111111111111111111111111111111111111111100000003".to_owned(),
                    "ca6a4f3d8e67e822728c6133e4fa1ea8af031d0bdc4c9115dd8ba6e8c0a47f8d".to_owned(),
                ),
            ]
        );
    }

    #[test]
    fn every_inner_identity_field_changes_its_identity() {
        let owner = AuthorityRecordId::from_bytes([0x21; 32]);
        let other_owner = AuthorityRecordId::from_bytes([0x22; 32]);

        let captured = CapturedInputRecordId::derive(owner, CapturedInputLocalIndex::new(0));
        assert_ne!(
            captured,
            CapturedInputRecordId::derive(other_owner, CapturedInputLocalIndex::new(0))
        );
        assert_ne!(
            captured,
            CapturedInputRecordId::derive(owner, CapturedInputLocalIndex::new(1))
        );

        let attempt = AttemptRecordId::derive(owner, AttemptLocalIndex::new(0));
        assert_ne!(
            attempt,
            AttemptRecordId::derive(other_owner, AttemptLocalIndex::new(0))
        );
        assert_ne!(
            attempt,
            AttemptRecordId::derive(owner, AttemptLocalIndex::new(1))
        );
        assert_ne!(
            attempt.as_bytes(),
            ContentDigest::of_canonical(&inner_record_id_bytes(
                ATTEMPT_RECORD_DOMAIN,
                ATTEMPT_RECORD_SCHEMA_VERSION + 1,
                owner,
                0,
            ))
            .as_bytes()
        );

        let commit = CommitRecordId::derive(owner, CommitLocalIndex::new(0));
        assert_ne!(
            commit,
            CommitRecordId::derive(other_owner, CommitLocalIndex::new(0))
        );
        assert_ne!(
            commit,
            CommitRecordId::derive(owner, CommitLocalIndex::new(1))
        );

        let reaction = ReactionEnvelopeId::derive(owner, ReactionLocalIndex::new(0));
        assert_ne!(
            reaction,
            ReactionEnvelopeId::derive(other_owner, ReactionLocalIndex::new(0))
        );
        assert_ne!(
            reaction,
            ReactionEnvelopeId::derive(owner, ReactionLocalIndex::new(1))
        );

        assert_ne!(captured.as_bytes(), attempt.as_bytes());
        assert_ne!(captured.as_bytes(), commit.as_bytes());
        assert_ne!(captured.as_bytes(), reaction.as_bytes());
        assert_ne!(attempt.as_bytes(), commit.as_bytes());
        assert_ne!(attempt.as_bytes(), reaction.as_bytes());
        assert_ne!(commit.as_bytes(), reaction.as_bytes());

        assert_eq!(
            CapturedInputRecordId::from_bytes(captured.into_bytes()),
            captured
        );
        assert_eq!(AttemptRecordId::from_bytes(attempt.into_bytes()), attempt);
        assert_eq!(CommitRecordId::from_bytes(commit.into_bytes()), commit);
        assert_eq!(
            ReactionEnvelopeId::from_bytes(reaction.into_bytes()),
            reaction
        );
    }

    #[test]
    fn local_references_match_the_frozen_kind_and_role_vector() {
        let captured = SameRecordCapturedInputRef::new(CapturedInputLocalIndex::new(9));
        let attempt = SameRecordAttemptRef::new(AttemptLocalIndex::new(8));
        let commit = SameRecordCommitRef::new(CommitLocalIndex::new(7));
        let reaction = SameRecordReactionRef::new(ReactionLocalIndex::new(6));
        let current = CurrentRecordRef::new();
        let mut writer = CanonicalWriter::new(REFERENCE_VECTOR_DOMAIN);
        writer.write_u16(1);
        captured.write_canonical(&mut writer);
        attempt.write_canonical(&mut writer);
        commit.write_canonical(&mut writer);
        reaction.write_canonical(&mut writer);
        current.write_canonical(&mut writer);
        let bytes = writer.finish();

        assert_eq!(CapturedInputLocalIndex::new(0).get(), 0);
        assert_eq!(captured.index().get(), 9);
        assert_eq!(attempt.index().get(), 8);
        assert_eq!(commit.index().get(), 7);
        assert_eq!(reaction.index().get(), 6);
        assert_eq!(
            hex(bytes.as_bytes()),
            "776f726c642d63616e6f6e6963616c2d7631000000000000001d617574686f726974792d7265666572656e63652d766563746f722d76310001000000000000000900000001000000080000000200000007000000030000000600000000"
        );
    }

    #[test]
    fn previous_authority_role_has_no_source_discriminant() {
        let root = AuthorityRecordAnchor::from_bytes([0x44; 32]);
        let record = AuthorityRecordId::from_bytes([0x44; 32]);
        let from_root = PreviousAuthorityHash::from_root_anchor(root);
        let from_record = PreviousAuthorityHash::from_record(record);

        assert_eq!(from_root, from_record);
        assert_eq!(from_root.as_bytes(), &[0x44; 32]);
        assert_ne!(
            from_root,
            PreviousAuthorityHash::from_record(AuthorityRecordId::from_bytes([0x45; 32]))
        );
    }
}
