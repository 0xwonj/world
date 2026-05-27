use std::collections::BTreeSet;

use world_core::{
    AuthorityClass, CausalSource, CausalTransactionId, DefinitionId, EntityId, EventRecordId,
    ProvenanceKey, ReplayLevel, RuntimeEntityHandle, SimulationTime, StoreCursor,
};
use world_defs::EventRecordSpec;

use crate::{
    DerivedViewInvalidationReport, EntitySnapshot, EventRecord, EventRoleBinding,
    InvalidationPackage, InvalidationSource, ModelError, RelationFamily, RelationRecord,
    StoreFamily, TransactionRecord,
};

/// Runtime-accepted hard commit package applied by the model as one unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedHardCommit {
    transaction: TransactionCommit,
    events: Vec<EventCommit>,
    changes: Vec<HardStateChange>,
    invalidation: InvalidationPackage,
}

impl AcceptedHardCommit {
    /// Creates a hard commit package that has already been accepted by runtime authority.
    pub fn new(
        transaction: TransactionCommit,
        events: impl IntoIterator<Item = EventCommit>,
        changes: impl IntoIterator<Item = HardStateChange>,
        invalidation: InvalidationPackage,
    ) -> Result<Self, ModelError> {
        if invalidation.source() != InvalidationSource::HardCommit(transaction.id()) {
            return Err(ModelError::InvalidHardCommitInvalidation {
                transaction: transaction.id(),
                invalidation_source: invalidation.source(),
            });
        }

        let events = events.into_iter().collect::<Vec<_>>();
        for event in &events {
            validate_event_roles(event)?;
        }

        Ok(Self {
            transaction,
            events,
            changes: changes.into_iter().collect(),
            invalidation,
        })
    }

    /// Returns committed transaction metadata.
    pub const fn transaction(&self) -> &TransactionCommit {
        &self.transaction
    }

    /// Returns event metadata included in this commit.
    pub fn events(&self) -> &[EventCommit] {
        &self.events
    }

    /// Returns hard state changes included in this commit.
    pub fn changes(&self) -> &[HardStateChange] {
        &self.changes
    }

    /// Returns the derived-view invalidation package included in this commit.
    pub const fn invalidation(&self) -> &InvalidationPackage {
        &self.invalidation
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        TransactionCommit,
        Vec<EventCommit>,
        Vec<HardStateChange>,
        InvalidationPackage,
    ) {
        (
            self.transaction,
            self.events,
            self.changes,
            self.invalidation,
        )
    }
}

/// Transaction metadata accepted by the causal runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransactionCommit {
    id: CausalTransactionId,
    source: CausalSource,
    action: DefinitionId,
    effect_program: DefinitionId,
    replay_level: ReplayLevel,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}

impl TransactionCommit {
    /// Creates committed transaction metadata.
    pub const fn new(
        id: CausalTransactionId,
        source: CausalSource,
        action: DefinitionId,
        effect_program: DefinitionId,
        replay_level: ReplayLevel,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            id,
            source,
            action,
            effect_program,
            replay_level,
            occurred_at,
            provenance,
        }
    }

    /// Returns the committed transaction id.
    pub const fn id(self) -> CausalTransactionId {
        self.id
    }

    /// Returns the runtime source that submitted the transaction.
    pub const fn source(self) -> CausalSource {
        self.source
    }

    /// Returns the action definition accepted by runtime validation.
    pub const fn action(self) -> DefinitionId {
        self.action
    }

    /// Returns the effect program definition interpreted for this transaction.
    pub const fn effect_program(self) -> DefinitionId {
        self.effect_program
    }

    /// Returns declared replay strength for this transaction.
    pub const fn replay_level(self) -> ReplayLevel {
        self.replay_level
    }

    /// Returns the simulation time of the transaction.
    pub const fn occurred_at(self) -> SimulationTime {
        self.occurred_at
    }

    /// Returns transaction provenance, if known.
    pub const fn provenance(self) -> Option<ProvenanceKey> {
        self.provenance
    }

    pub(crate) const fn into_record(self) -> TransactionRecord {
        TransactionRecord::new(
            self.id,
            self.source,
            self.action,
            self.effect_program,
            self.replay_level,
            self.occurred_at,
            self.provenance,
        )
    }
}

/// Event metadata accepted with a hard commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventCommit {
    id: EventRecordId,
    spec: EventRecordSpec,
    roles: Vec<EventRoleBinding>,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}

impl EventCommit {
    /// Creates committed event metadata.
    pub fn new(
        id: EventRecordId,
        spec: EventRecordSpec,
        roles: impl IntoIterator<Item = EventRoleBinding>,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            id,
            spec,
            roles: roles.into_iter().collect(),
            occurred_at,
            provenance,
        }
    }

    /// Returns the committed event id.
    pub const fn id(&self) -> EventRecordId {
        self.id
    }

    /// Returns the checked event record spec.
    pub fn spec(&self) -> &EventRecordSpec {
        &self.spec
    }

    /// Returns role bindings captured with the event record.
    pub fn roles(&self) -> &[EventRoleBinding] {
        &self.roles
    }

    /// Returns the simulation time of the event.
    pub const fn occurred_at(&self) -> SimulationTime {
        self.occurred_at
    }

    /// Returns event provenance, if known.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }

    pub(crate) fn into_record(self, transaction: CausalTransactionId) -> EventRecord {
        EventRecord::new(
            self.id,
            transaction,
            self.spec,
            self.roles,
            self.occurred_at,
            self.provenance,
        )
    }
}

fn validate_event_roles(event: &EventCommit) -> Result<(), ModelError> {
    let mut roles = BTreeSet::new();
    for binding in event.roles() {
        if !roles.insert(binding.role().clone()) {
            return Err(ModelError::DuplicateEventRoleBinding {
                event: event.id(),
                role: binding.role().clone(),
            });
        }
    }

    for role in event.spec().roles() {
        if !roles.contains(role) {
            return Err(ModelError::MissingEventRoleBinding {
                event: event.id(),
                role: role.clone(),
            });
        }
    }

    Ok(())
}

/// Hard model state change included in an accepted causal commit.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HardStateChange {
    /// Insert a new entity snapshot into hard world state.
    InsertEntity {
        /// Entity being inserted.
        entity: EntityId,
        /// Runtime handle associated with the entity, if one exists.
        runtime_handle: Option<RuntimeEntityHandle>,
        /// Provenance for the inserted entity.
        provenance: Option<ProvenanceKey>,
    },
    /// Insert a hard relation into the relation store.
    InsertRelation {
        /// Relation subject.
        subject: EntityId,
        /// Relation family.
        family: RelationFamily,
        /// Relation object.
        object: EntityId,
        /// Provenance for the inserted relation.
        provenance: Option<ProvenanceKey>,
    },
}

impl HardStateChange {
    /// Creates an entity insertion change.
    pub const fn insert_entity(
        entity: EntityId,
        runtime_handle: Option<RuntimeEntityHandle>,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self::InsertEntity {
            entity,
            runtime_handle,
            provenance,
        }
    }

    /// Creates a relation insertion change.
    pub const fn insert_relation(
        subject: EntityId,
        family: RelationFamily,
        object: EntityId,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self::InsertRelation {
            subject,
            family,
            object,
            provenance,
        }
    }

    /// Returns which store family this change mutates.
    pub const fn changed_store_family(&self) -> StoreFamily {
        match self {
            Self::InsertEntity { .. } => StoreFamily::World,
            Self::InsertRelation { .. } => StoreFamily::Relation,
        }
    }

    pub(crate) fn validate_hard_authority(&self) -> Result<(), ModelError> {
        match self {
            Self::InsertEntity { .. } => Ok(()),
            Self::InsertRelation {
                subject,
                family,
                object,
                ..
            } if family.authority_class() != AuthorityClass::Hard => {
                Err(ModelError::NonHardRelationInHardCommit {
                    subject: *subject,
                    family: *family,
                    object: *object,
                })
            }
            Self::InsertRelation { .. } => Ok(()),
        }
    }

    pub(crate) fn to_entity_snapshot(&self) -> Option<EntitySnapshot> {
        match self {
            Self::InsertEntity {
                entity,
                runtime_handle,
                provenance,
            } => Some(EntitySnapshot::new(*entity, *runtime_handle, *provenance)),
            Self::InsertRelation { .. } => None,
        }
    }

    pub(crate) fn to_relation_record(&self) -> Option<RelationRecord> {
        match self {
            Self::InsertRelation {
                subject,
                family,
                object,
                provenance,
            } => Some(RelationRecord::new(*subject, *family, *object, *provenance)),
            Self::InsertEntity { .. } => None,
        }
    }
}

/// Result of applying an accepted hard commit to model storage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardCommitApplication {
    transaction_cursor: StoreCursor,
    event_cursors: Vec<StoreCursor>,
    invalidation: DerivedViewInvalidationReport,
}

impl HardCommitApplication {
    pub(crate) fn new(
        transaction_cursor: StoreCursor,
        event_cursors: Vec<StoreCursor>,
        invalidation: DerivedViewInvalidationReport,
    ) -> Self {
        Self {
            transaction_cursor,
            event_cursors,
            invalidation,
        }
    }

    /// Returns the append cursor assigned to the transaction record.
    pub const fn transaction_cursor(&self) -> StoreCursor {
        self.transaction_cursor
    }

    /// Returns event append cursors in package order.
    pub fn event_cursors(&self) -> &[StoreCursor] {
        &self.event_cursors
    }

    /// Returns the invalidation report produced by the commit.
    pub const fn invalidation(&self) -> DerivedViewInvalidationReport {
        self.invalidation
    }
}
