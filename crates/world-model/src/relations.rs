use std::collections::BTreeMap;

use world_core::{AuthorityClass, EntityId, ProvenanceKey};

#[cfg(test)]
use crate::ModelError;

/// Typed relation family stored by the model.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RelationFamily {
    /// Hard containment or inventory relation.
    ContainedIn,
    /// Hard equipment-slot relation.
    EquippedInSlot,
    /// Hard attachment relation.
    AttachedTo,
    /// Hard embedded-object relation.
    EmbeddedIn,
    /// Hard location relation at the active resolution.
    LocatedIn,
    /// Hard passage or topology relation.
    PassageTo,
    /// Social membership relation.
    MemberOf,
    /// Social claim over an object, right, or role.
    SocialClaimOn,
}

impl RelationFamily {
    /// Returns the authority class that owns this relation family.
    pub const fn authority_class(self) -> AuthorityClass {
        match self {
            Self::ContainedIn
            | Self::EquippedInSlot
            | Self::AttachedTo
            | Self::EmbeddedIn
            | Self::LocatedIn
            | Self::PassageTo => AuthorityClass::Hard,
            Self::MemberOf | Self::SocialClaimOn => AuthorityClass::Social,
        }
    }
}

/// Identity of one typed relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationKey {
    subject: EntityId,
    family: RelationFamily,
    object: EntityId,
}

impl RelationKey {
    /// Creates a relation key.
    pub const fn new(subject: EntityId, family: RelationFamily, object: EntityId) -> Self {
        Self {
            subject,
            family,
            object,
        }
    }

    /// Returns the relation subject.
    pub const fn subject(self) -> EntityId {
        self.subject
    }

    /// Returns the relation family.
    pub const fn family(self) -> RelationFamily {
        self.family
    }

    /// Returns the relation object.
    pub const fn object(self) -> EntityId {
        self.object
    }

    /// Returns the authority class that owns the relation family.
    pub const fn authority_class(self) -> AuthorityClass {
        self.family.authority_class()
    }
}

/// Stored typed relation plus provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationRecord {
    key: RelationKey,
    provenance: Option<ProvenanceKey>,
}

impl RelationRecord {
    /// Returns the relation key.
    pub const fn key(&self) -> RelationKey {
        self.key
    }

    /// Returns the relation subject.
    pub const fn subject(&self) -> EntityId {
        self.key.subject()
    }

    /// Returns the relation family.
    pub const fn family(&self) -> RelationFamily {
        self.key.family()
    }

    /// Returns the relation object.
    pub const fn object(&self) -> EntityId {
        self.key.object()
    }

    /// Returns the authority class that owns the relation.
    pub const fn authority_class(&self) -> AuthorityClass {
        self.key.authority_class()
    }

    /// Returns relation provenance, if known.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }
}

#[cfg(test)]
impl RelationRecord {
    pub(crate) const fn new(
        subject: EntityId,
        family: RelationFamily,
        object: EntityId,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            key: RelationKey::new(subject, family, object),
            provenance,
        }
    }
}

/// Store for typed relation families.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationStore {
    relations: BTreeMap<RelationKey, RelationRecord>,
}

impl RelationStore {
    /// Returns whether a relation key is present.
    pub fn contains(&self, key: RelationKey) -> bool {
        self.relations.contains_key(&key)
    }

    /// Returns a relation record.
    pub fn relation(&self, key: RelationKey) -> Option<&RelationRecord> {
        self.relations.get(&key)
    }

    /// Returns the total number of relation records.
    pub fn len(&self) -> usize {
        self.relations.len()
    }

    /// Returns whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.relations.is_empty()
    }

    /// Counts relation records owned by the given authority class.
    pub fn count_by_authority(&self, authority: AuthorityClass) -> usize {
        self.relations
            .values()
            .filter(|record| record.authority_class() == authority)
            .count()
    }

    /// Iterates relation records in key order.
    pub fn relations(&self) -> impl Iterator<Item = &RelationRecord> {
        self.relations.values()
    }
}

#[cfg(test)]
impl RelationStore {
    pub(crate) fn insert(&mut self, record: RelationRecord) -> Result<(), ModelError> {
        let key = record.key();
        if self.relations.contains_key(&key) {
            return Err(ModelError::DuplicateRelation {
                subject: key.subject(),
                family: key.family(),
                object: key.object(),
            });
        }

        self.relations.insert(key, record);
        Ok(())
    }
}
