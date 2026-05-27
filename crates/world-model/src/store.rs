use std::collections::BTreeMap;

use world_core::{AuthorityClass, EntityId, ProvenanceKey, RuntimeEntityHandle};

#[cfg(test)]
use crate::ModelError;

/// Coarse model store family used for query labels and invalidation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StoreFamily {
    /// Current hard world state.
    World,
    /// Typed relation store families.
    Relation,
    /// Committed hard event and transaction history.
    EventHistory,
    /// Durable runtime-control state.
    RuntimeControl,
    /// Committed social and institutional soft truth.
    SocialInstitutional,
    /// Authored, generated, or accepted chronology.
    Chronology,
    /// Holder-relative actor truth.
    Epistemic,
    /// Accepted appraisal and motivation records.
    AppraisalRecord,
    /// Cached or materialized derived view metadata.
    DerivedView,
}

impl StoreFamily {
    /// Returns the primary authority class for stores with a single owner.
    pub const fn primary_authority(self) -> Option<AuthorityClass> {
        match self {
            Self::World | Self::EventHistory => Some(AuthorityClass::Hard),
            Self::RuntimeControl => Some(AuthorityClass::RuntimeControl),
            Self::SocialInstitutional => Some(AuthorityClass::Social),
            Self::Chronology => Some(AuthorityClass::Chronology),
            Self::Epistemic => Some(AuthorityClass::ActorTruth),
            Self::AppraisalRecord => Some(AuthorityClass::Appraisal),
            Self::Relation | Self::DerivedView => None,
        }
    }
}

/// Label describing which authority class and store family a read surface used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthorityRead {
    authority: AuthorityClass,
    store: StoreFamily,
}

impl AuthorityRead {
    /// Returns the hard world-state read label.
    pub const fn hard_world() -> Self {
        Self {
            authority: AuthorityClass::Hard,
            store: StoreFamily::World,
        }
    }

    /// Returns the committed event-history read label.
    pub const fn event_history() -> Self {
        Self {
            authority: AuthorityClass::Hard,
            store: StoreFamily::EventHistory,
        }
    }

    /// Returns the runtime-control read label.
    pub const fn runtime_control() -> Self {
        Self {
            authority: AuthorityClass::RuntimeControl,
            store: StoreFamily::RuntimeControl,
        }
    }

    /// Returns the social store read label.
    pub const fn social_store() -> Self {
        Self {
            authority: AuthorityClass::Social,
            store: StoreFamily::SocialInstitutional,
        }
    }

    /// Returns the chronology store read label.
    pub const fn chronology_store() -> Self {
        Self {
            authority: AuthorityClass::Chronology,
            store: StoreFamily::Chronology,
        }
    }

    /// Returns the epistemic store read label.
    pub const fn epistemic_store() -> Self {
        Self {
            authority: AuthorityClass::ActorTruth,
            store: StoreFamily::Epistemic,
        }
    }

    /// Returns the appraisal store read label.
    pub const fn appraisal_store() -> Self {
        Self {
            authority: AuthorityClass::Appraisal,
            store: StoreFamily::AppraisalRecord,
        }
    }

    /// Returns the hard relation read label.
    pub const fn hard_relation() -> Self {
        Self {
            authority: AuthorityClass::Hard,
            store: StoreFamily::Relation,
        }
    }

    /// Returns the social relation read label.
    pub const fn social_relation() -> Self {
        Self {
            authority: AuthorityClass::Social,
            store: StoreFamily::Relation,
        }
    }

    /// Creates a read label for derived-view metadata.
    pub const fn derived_view(authority: AuthorityClass) -> Self {
        Self {
            authority,
            store: StoreFamily::DerivedView,
        }
    }

    /// Returns the authority class read by the query surface.
    pub const fn authority_class(self) -> AuthorityClass {
        self.authority
    }

    /// Returns the store family read by the query surface.
    pub const fn store_family(self) -> StoreFamily {
        self.store
    }
}

/// Minimal current-state entity snapshot stored by the model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntitySnapshot {
    entity: EntityId,
    runtime_handle: Option<RuntimeEntityHandle>,
    provenance: Option<ProvenanceKey>,
}

impl EntitySnapshot {
    /// Returns the durable entity id.
    pub const fn entity(&self) -> EntityId {
        self.entity
    }

    /// Returns the current runtime handle, if one exists.
    pub const fn runtime_handle(&self) -> Option<RuntimeEntityHandle> {
        self.runtime_handle
    }

    /// Returns provenance for the snapshot, if known.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }
}

impl EntitySnapshot {
    pub(crate) const fn new(
        entity: EntityId,
        runtime_handle: Option<RuntimeEntityHandle>,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            entity,
            runtime_handle,
            provenance,
        }
    }
}

/// Store for current hard entity state known to the model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldStore {
    entities: BTreeMap<EntityId, EntitySnapshot>,
}

impl WorldStore {
    /// Returns whether the store contains the entity.
    pub fn contains_entity(&self, entity: EntityId) -> bool {
        self.entities.contains_key(&entity)
    }

    /// Returns a stored entity snapshot.
    pub fn entity(&self, entity: EntityId) -> Option<&EntitySnapshot> {
        self.entities.get(&entity)
    }

    /// Returns the number of stored entity snapshots.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Iterates stored entity snapshots by durable entity id.
    pub fn entities(&self) -> impl Iterator<Item = &EntitySnapshot> {
        self.entities.values()
    }
}

impl WorldStore {
    pub(crate) fn insert_planned(&mut self, snapshot: EntitySnapshot) {
        self.entities.insert(snapshot.entity(), snapshot);
    }

    #[cfg(test)]
    pub(crate) fn insert(&mut self, snapshot: EntitySnapshot) -> Result<(), ModelError> {
        let entity = snapshot.entity();
        if self.entities.contains_key(&entity) {
            return Err(ModelError::DuplicateEntity { entity });
        }

        self.entities.insert(entity, snapshot);
        Ok(())
    }
}
