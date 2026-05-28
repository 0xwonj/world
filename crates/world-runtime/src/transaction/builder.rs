use std::collections::BTreeSet;

use world_core::{
    AuthorityClass, CausalTransactionId, EntityId, EventRecordId, ProvenanceKey, ReplayLevel,
    SimulationTime,
};
use world_defs::EventRecordSpec;
use world_model::{
    EventRoleBinding, HardStateChange, InvalidationPackage, RelationKey, RuntimeControlChange,
    StoreFamily, TransactionCause, WorldModel,
};

use crate::{RequestSource, RuntimeError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CausalTransactionBuilder {
    id: CausalTransactionId,
    source: RequestSource,
    cause: TransactionCause,
    occurred_at: SimulationTime,
    replay_level: ReplayLevel,
    provenance: Option<ProvenanceKey>,
    changes: Vec<HardStateChange>,
    control_changes: Vec<RuntimeControlChange>,
    events: Vec<PendingEventRecord>,
    invalidation: InvalidationPackage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CausalTransactionHeader {
    pub(crate) id: CausalTransactionId,
    pub(crate) source: RequestSource,
    pub(crate) cause: TransactionCause,
    pub(crate) occurred_at: SimulationTime,
    pub(crate) replay_level: ReplayLevel,
    pub(crate) provenance: Option<ProvenanceKey>,
}

impl CausalTransactionBuilder {
    pub(crate) fn new(header: CausalTransactionHeader, invalidation: InvalidationPackage) -> Self {
        Self {
            id: header.id,
            source: header.source,
            cause: header.cause,
            occurred_at: header.occurred_at,
            replay_level: header.replay_level,
            provenance: header.provenance,
            changes: Vec::new(),
            control_changes: Vec::new(),
            events: Vec::new(),
            invalidation,
        }
    }

    pub(crate) fn emitted_event_specs(&self) -> BTreeSet<EventRecordSpec> {
        self.events
            .iter()
            .map(PendingEventRecord::spec)
            .cloned()
            .collect()
    }

    pub(crate) const fn cause(&self) -> TransactionCause {
        self.cause
    }

    pub(crate) fn has_staged_events(&self) -> bool {
        !self.events.is_empty()
    }

    pub(crate) fn push_change(&mut self, change: HardStateChange) {
        self.invalidation
            .mark_store_family(change.changed_store_family());
        self.changes.push(change);
    }

    pub(crate) fn push_control_change(&mut self, change: RuntimeControlChange) {
        self.invalidation
            .mark_authority_class(AuthorityClass::RuntimeControl)
            .mark_store_family(StoreFamily::RuntimeControl);
        self.control_changes.push(change);
    }

    pub(crate) fn staged_entity_insert(&self, entity: EntityId) -> bool {
        self.changes.iter().any(|change| {
            matches!(
                change,
                HardStateChange::InsertEntity {
                    entity: staged, ..
                } if *staged == entity
            )
        })
    }

    pub(crate) fn staged_relation_insert(&self, key: RelationKey) -> bool {
        self.changes.iter().any(|change| {
            matches!(
                change,
                HardStateChange::InsertRelation {
                    subject,
                    family,
                    object,
                    ..
                } if RelationKey::new(*subject, *family, *object) == key
            )
        })
    }

    pub(crate) fn push_event(&mut self, event: PendingEventRecord) -> Result<(), RuntimeError> {
        if self
            .events
            .iter()
            .any(|existing| existing.id() == event.id())
        {
            return Err(RuntimeError::DuplicateStagedEvent {
                transaction: self.id,
                event: event.id(),
            });
        }

        self.events.push(event);
        Ok(())
    }

    pub(crate) fn into_parts(self) -> StagedTransactionParts {
        StagedTransactionParts {
            id: self.id,
            source: self.source,
            cause: self.cause,
            occurred_at: self.occurred_at,
            replay_level: self.replay_level,
            provenance: self.provenance,
            changes: self.changes,
            control_changes: self.control_changes,
            events: self.events,
            invalidation: self.invalidation,
        }
    }
}

pub(crate) struct StagedTransactionParts {
    pub(crate) id: CausalTransactionId,
    pub(crate) source: RequestSource,
    pub(crate) cause: TransactionCause,
    pub(crate) occurred_at: SimulationTime,
    pub(crate) replay_level: ReplayLevel,
    pub(crate) provenance: Option<ProvenanceKey>,
    pub(crate) changes: Vec<HardStateChange>,
    pub(crate) control_changes: Vec<RuntimeControlChange>,
    pub(crate) events: Vec<PendingEventRecord>,
    pub(crate) invalidation: InvalidationPackage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingEventRecord {
    id: EventRecordId,
    spec: EventRecordSpec,
    roles: Vec<EventRoleBinding>,
    provenance: Option<ProvenanceKey>,
}

impl PendingEventRecord {
    pub(crate) fn new(
        id: EventRecordId,
        spec: EventRecordSpec,
        roles: Vec<EventRoleBinding>,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            id,
            spec,
            roles,
            provenance,
        }
    }

    pub(crate) const fn id(&self) -> EventRecordId {
        self.id
    }

    pub(crate) fn spec(&self) -> &EventRecordSpec {
        &self.spec
    }

    pub(crate) fn roles(&self) -> &[EventRoleBinding] {
        &self.roles
    }

    pub(crate) const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }
}

pub(crate) struct EffectStager<'model, 'tx> {
    model: &'model WorldModel,
    transaction: &'tx mut CausalTransactionBuilder,
}

impl<'model, 'tx> EffectStager<'model, 'tx> {
    pub(crate) fn new(
        model: &'model WorldModel,
        transaction: &'tx mut CausalTransactionBuilder,
    ) -> Self {
        Self { model, transaction }
    }

    pub(crate) fn push_change(&mut self, change: HardStateChange) {
        self.transaction.push_change(change);
    }

    pub(crate) fn push_control_change(&mut self, change: RuntimeControlChange) {
        self.transaction.push_control_change(change);
    }

    pub(crate) fn push_event(&mut self, event: PendingEventRecord) -> Result<(), RuntimeError> {
        self.transaction.push_event(event)
    }

    pub(crate) fn contains_entity(&self, entity: EntityId) -> bool {
        self.transaction.staged_entity_insert(entity)
            || self.model.world_store().contains_entity(entity)
    }

    pub(crate) fn contains_relation(&self, key: RelationKey) -> bool {
        self.transaction.staged_relation_insert(key) || self.model.relation_store().contains(key)
    }
}
