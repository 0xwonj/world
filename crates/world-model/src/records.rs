use std::collections::BTreeMap;
use std::num::NonZeroU64;

use world_core::{ActorId, DefinitionId, InvalidCoreValue, ProvenanceKey};

#[cfg(test)]
use crate::ModelError;

/// Local identity for accepted non-hard records hosted by the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AcceptedRecordId(NonZeroU64);

impl AcceptedRecordId {
    /// Creates a record id when the raw value is nonzero.
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

impl TryFrom<u64> for AcceptedRecordId {
    type Error = InvalidCoreValue;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(InvalidCoreValue::Zero {
            type_name: "AcceptedRecordId",
        })
    }
}

impl From<AcceptedRecordId> for u64 {
    fn from(value: AcceptedRecordId) -> Self {
        value.get()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecordEnvelope {
    id: AcceptedRecordId,
    definition: Option<DefinitionId>,
    provenance: Option<ProvenanceKey>,
}

#[cfg(test)]
impl RecordEnvelope {
    const fn new(
        id: AcceptedRecordId,
        definition: Option<DefinitionId>,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            id,
            definition,
            provenance,
        }
    }
}

macro_rules! authority_record {
    ($name:ident) => {
        #[doc = "Accepted record envelope for one authority family."]
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name {
            envelope: RecordEnvelope,
        }

        impl $name {
            /// Returns the record id.
            pub const fn id(&self) -> AcceptedRecordId {
                self.envelope.id
            }

            /// Returns the checked definition associated with the record, if any.
            pub const fn definition(&self) -> Option<DefinitionId> {
                self.envelope.definition
            }

            /// Returns record provenance, if known.
            pub const fn provenance(&self) -> Option<ProvenanceKey> {
                self.envelope.provenance
            }
        }

        #[cfg(test)]
        impl $name {
            pub(crate) const fn new(
                id: AcceptedRecordId,
                definition: Option<DefinitionId>,
                provenance: Option<ProvenanceKey>,
            ) -> Self {
                Self {
                    envelope: RecordEnvelope::new(id, definition, provenance),
                }
            }
        }
    };
}

authority_record!(SocialRecord);
authority_record!(ChronologyRecord);
authority_record!(AppraisalRecord);

/// Holder that owns holder-relative epistemic truth.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EpistemicHolder {
    /// Actor-owned perception, memory, belief, or knowledge.
    Actor(ActorId),
}

/// Accepted epistemic record envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpistemicRecord {
    envelope: RecordEnvelope,
    holder: EpistemicHolder,
}

impl EpistemicRecord {
    /// Returns the record id.
    pub const fn id(&self) -> AcceptedRecordId {
        self.envelope.id
    }

    /// Returns the holder that owns this epistemic record.
    pub const fn holder(&self) -> EpistemicHolder {
        self.holder
    }

    /// Returns the checked definition associated with the record, if any.
    pub const fn definition(&self) -> Option<DefinitionId> {
        self.envelope.definition
    }

    /// Returns record provenance, if known.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.envelope.provenance
    }
}

#[cfg(test)]
impl EpistemicRecord {
    pub(crate) const fn new(
        id: AcceptedRecordId,
        holder: EpistemicHolder,
        definition: Option<DefinitionId>,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            envelope: RecordEnvelope::new(id, definition, provenance),
            holder,
        }
    }
}

macro_rules! authority_store {
    ($name:ident, $record:ident) => {
        #[doc = "Store for accepted records in one authority family."]
        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        pub struct $name {
            records: BTreeMap<AcceptedRecordId, $record>,
        }

        impl $name {
            /// Returns whether the store contains the record id.
            pub fn contains(&self, id: AcceptedRecordId) -> bool {
                self.records.contains_key(&id)
            }

            /// Returns an accepted record envelope.
            pub fn record(&self, id: AcceptedRecordId) -> Option<&$record> {
                self.records.get(&id)
            }

            /// Returns the number of accepted records.
            pub fn len(&self) -> usize {
                self.records.len()
            }

            /// Returns whether the store is empty.
            pub fn is_empty(&self) -> bool {
                self.records.is_empty()
            }

            /// Iterates accepted records in id order.
            pub fn records(&self) -> impl Iterator<Item = &$record> {
                self.records.values()
            }
        }

        #[cfg(test)]
        impl $name {
            pub(crate) fn insert(&mut self, record: $record) -> Result<(), ModelError> {
                let id = record.id();
                if self.records.contains_key(&id) {
                    return Err(ModelError::DuplicateAcceptedRecord { record: id });
                }

                self.records.insert(id, record);
                Ok(())
            }
        }
    };
}

authority_store!(SocialInstitutionalStore, SocialRecord);
authority_store!(ChronologyStore, ChronologyRecord);
authority_store!(AppraisalRecordStore, AppraisalRecord);

/// Store for accepted holder-relative epistemic records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EpistemicStore {
    records: BTreeMap<AcceptedRecordId, EpistemicRecord>,
}

impl EpistemicStore {
    /// Returns whether the store contains the record id.
    pub fn contains(&self, id: AcceptedRecordId) -> bool {
        self.records.contains_key(&id)
    }

    /// Returns an accepted epistemic record envelope.
    pub fn record(&self, id: AcceptedRecordId) -> Option<&EpistemicRecord> {
        self.records.get(&id)
    }

    /// Returns the number of accepted epistemic records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Iterates accepted epistemic records in id order.
    pub fn records(&self) -> impl Iterator<Item = &EpistemicRecord> {
        self.records.values()
    }

    /// Iterates accepted epistemic records owned by one holder.
    pub fn records_for_holder(
        &self,
        holder: EpistemicHolder,
    ) -> impl Iterator<Item = &EpistemicRecord> {
        self.records
            .values()
            .filter(move |record| record.holder() == holder)
    }

    /// Counts accepted epistemic records owned by one holder.
    pub fn count_for_holder(&self, holder: EpistemicHolder) -> usize {
        self.records_for_holder(holder).count()
    }
}

#[cfg(test)]
impl EpistemicStore {
    pub(crate) fn insert(&mut self, record: EpistemicRecord) -> Result<(), ModelError> {
        let id = record.id();
        if self.records.contains_key(&id) {
            return Err(ModelError::DuplicateAcceptedRecord { record: id });
        }

        self.records.insert(id, record);
        Ok(())
    }
}
