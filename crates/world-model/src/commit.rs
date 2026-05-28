use std::collections::BTreeSet;

use world_core::{
    AuthorityClass, CausalSource, CausalTransactionId, DefinitionId, EntityId, EventRecordId,
    ProvenanceKey, ReplayLevel, RuntimeEntityHandle, SimulationTime, StoreCursor,
};
use world_defs::EventRecordSpec;

use crate::{
    DerivedViewInvalidationReport, EventRecord, EventRoleBinding, InvalidationPackage,
    InvalidationSource, ModelError, RelationFamily, RuntimeControlChange, StoreFamily,
    TransactionCause, TransactionRecord,
};

/// Runtime-authority hard commit package applied by the model as one unit.
///
/// Normal code should produce this package by executing through the causal
/// runtime. The constructor is public because the runtime lives in a separate
/// crate; direct construction by other callers bypasses runtime discipline and
/// is guarded by repository authority tests rather than Rust friend visibility.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedHardCommit {
    transaction: TransactionCommit,
    events: Vec<EventCommit>,
    changes: Vec<HardStateChange>,
    control_changes: Vec<RuntimeControlChange>,
    invalidation: InvalidationPackage,
}

impl AcceptedHardCommit {
    /// Creates a hard commit package already accepted by runtime authority.
    ///
    /// This is an accepted-package constructor for the runtime producer, not a
    /// general hard-state mutation API.
    pub fn new(
        transaction: TransactionCommit,
        events: impl IntoIterator<Item = EventCommit>,
        changes: impl IntoIterator<Item = HardStateChange>,
        invalidation: InvalidationPackage,
    ) -> Result<Self, ModelError> {
        Self::with_control_changes(transaction, events, changes, [], invalidation)
    }

    /// Creates a runtime-authority hard commit package with runtime-control
    /// changes that must apply atomically with the hard transaction.
    ///
    /// This keeps hard state and runtime-control state in one accepted package
    /// when process execution commits both kinds of state.
    pub fn with_control_changes(
        transaction: TransactionCommit,
        events: impl IntoIterator<Item = EventCommit>,
        changes: impl IntoIterator<Item = HardStateChange>,
        control_changes: impl IntoIterator<Item = RuntimeControlChange>,
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
            control_changes: control_changes.into_iter().collect(),
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

    /// Returns runtime-control changes that are atomic with this hard commit.
    pub fn control_changes(&self) -> &[RuntimeControlChange] {
        &self.control_changes
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
        Vec<RuntimeControlChange>,
        InvalidationPackage,
    ) {
        (
            self.transaction,
            self.events,
            self.changes,
            self.control_changes,
            self.invalidation,
        )
    }
}

/// Transaction metadata accepted by the causal runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransactionCommit {
    id: CausalTransactionId,
    source: CausalSource,
    cause: TransactionCause,
    replay_level: ReplayLevel,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}

impl TransactionCommit {
    /// Creates committed transaction metadata for an action request.
    pub const fn new(
        id: CausalTransactionId,
        source: CausalSource,
        action: DefinitionId,
        effect_program: DefinitionId,
        replay_level: ReplayLevel,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self::for_action(
            id,
            source,
            action,
            effect_program,
            replay_level,
            occurred_at,
            provenance,
        )
    }

    /// Creates committed transaction metadata for an action request.
    pub const fn for_action(
        id: CausalTransactionId,
        source: CausalSource,
        action: DefinitionId,
        effect_program: DefinitionId,
        replay_level: ReplayLevel,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self::from_cause(
            id,
            source,
            TransactionCause::Action {
                action,
                effect_program,
            },
            replay_level,
            occurred_at,
            provenance,
        )
    }

    /// Creates committed transaction metadata from an explicit transaction cause.
    pub const fn from_cause(
        id: CausalTransactionId,
        source: CausalSource,
        cause: TransactionCause,
        replay_level: ReplayLevel,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            id,
            source,
            cause,
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

    /// Returns the runtime cause for this transaction.
    pub const fn cause(self) -> TransactionCause {
        self.cause
    }

    /// Returns the action definition for action-request transactions.
    pub const fn action(self) -> Option<DefinitionId> {
        self.cause.action()
    }

    /// Returns the effect program associated with action-request transactions.
    pub const fn effect_program(self) -> Option<DefinitionId> {
        self.cause.effect_program()
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
            self.cause,
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
