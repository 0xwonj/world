use std::collections::{BTreeMap, BTreeSet};

use world_core::{
    CausalSource, CausalTransactionId, DefinitionId, EntityId, EventRecordId, ProcessInstanceId,
    ProvenanceKey, ReplayLevel, ScheduledWakeupId, SimulationTime, StoreCursor,
};
use world_defs::{EventRecordSpec, ResolutionTier, RoleName};

use crate::ModelError;

/// Committed transaction metadata stored by event history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransactionRecord {
    id: CausalTransactionId,
    source: CausalSource,
    cause: TransactionCause,
    replay_level: ReplayLevel,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}

/// Runtime cause for a committed transaction.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionCause {
    /// A player, actor-policy, tooling, or engine action request.
    Action {
        /// Action definition accepted by runtime validation.
        action: DefinitionId,
        /// Effect program interpreted for this transaction.
        effect_program: DefinitionId,
    },
    /// A scheduler-selected durable process tick.
    ///
    /// This records the tick that actually occurred, not every definition-level
    /// program that could implement it. When a process tick executes an
    /// interpreted effect program, record that through explicit execution
    /// metadata instead of making `effect_program` mandatory for all process
    /// ticks.
    ProcessTick {
        /// Process instance advanced by the tick.
        process: ProcessInstanceId,
        /// Process definition used to select tick semantics.
        process_definition: DefinitionId,
        /// Resolution tier used by the process instance.
        resolution: ResolutionTier,
        /// Scheduled wakeup consumed by this tick.
        wakeup: ScheduledWakeupId,
    },
}

impl TransactionCause {
    /// Returns the action definition for action-request transactions.
    pub const fn action(self) -> Option<DefinitionId> {
        match self {
            Self::Action { action, .. } => Some(action),
            Self::ProcessTick { .. } => None,
        }
    }

    /// Returns the effect program associated with action-request transactions.
    pub const fn effect_program(self) -> Option<DefinitionId> {
        match self {
            Self::Action { effect_program, .. } => Some(effect_program),
            Self::ProcessTick { .. } => None,
        }
    }
}

impl TransactionRecord {
    /// Returns the transaction id.
    pub const fn id(&self) -> CausalTransactionId {
        self.id
    }

    /// Returns the runtime source that submitted the transaction.
    pub const fn source(&self) -> CausalSource {
        self.source
    }

    /// Returns the runtime cause for this transaction.
    pub const fn cause(&self) -> TransactionCause {
        self.cause
    }

    /// Returns the action definition for action-request transactions.
    pub const fn action(&self) -> Option<DefinitionId> {
        self.cause.action()
    }

    /// Returns the effect program associated with action-request transactions.
    pub const fn effect_program(&self) -> Option<DefinitionId> {
        self.cause.effect_program()
    }

    /// Returns declared replay strength for this transaction.
    pub const fn replay_level(&self) -> ReplayLevel {
        self.replay_level
    }

    /// Returns the simulation time of the transaction.
    pub const fn occurred_at(&self) -> SimulationTime {
        self.occurred_at
    }

    /// Returns transaction provenance, if known.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }
}

impl TransactionRecord {
    pub(crate) const fn new(
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
}

/// Entity bound to one role in an emitted event record.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventRoleBinding {
    role: RoleName,
    entity: EntityId,
}

impl EventRoleBinding {
    /// Creates an event-role binding from checked runtime bindings.
    pub fn new(role: RoleName, entity: EntityId) -> Self {
        Self { role, entity }
    }

    /// Returns the event role name.
    pub fn role(&self) -> &RoleName {
        &self.role
    }

    /// Returns the entity bound to the event role.
    pub const fn entity(&self) -> EntityId {
        self.entity
    }
}

/// Committed event metadata stored by event history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRecord {
    id: EventRecordId,
    transaction: CausalTransactionId,
    spec: EventRecordSpec,
    roles: Vec<EventRoleBinding>,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}

impl EventRecord {
    /// Returns the event id.
    pub const fn id(&self) -> EventRecordId {
        self.id
    }

    /// Returns the transaction that emitted the event.
    pub const fn transaction(&self) -> CausalTransactionId {
        self.transaction
    }

    /// Returns the checked event record spec committed by runtime.
    pub fn spec(&self) -> &EventRecordSpec {
        &self.spec
    }

    /// Returns runtime role bindings captured with the event record.
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
}

impl EventRecord {
    pub(crate) fn new(
        id: EventRecordId,
        transaction: CausalTransactionId,
        spec: EventRecordSpec,
        roles: Vec<EventRoleBinding>,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            id,
            transaction,
            spec,
            roles,
            occurred_at,
            provenance,
        }
    }
}

/// Transaction record plus append cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredTransactionRecord {
    record: TransactionRecord,
    cursor: StoreCursor,
}

impl StoredTransactionRecord {
    /// Returns the stored transaction metadata.
    pub const fn record(&self) -> &TransactionRecord {
        &self.record
    }

    /// Returns the append cursor assigned by event history.
    pub const fn cursor(&self) -> StoreCursor {
        self.cursor
    }
}

impl StoredTransactionRecord {
    fn new(record: TransactionRecord, cursor: StoreCursor) -> Self {
        Self { record, cursor }
    }
}

/// Event record plus append cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredEventRecord {
    record: EventRecord,
    cursor: StoreCursor,
}

impl StoredEventRecord {
    /// Returns the stored event metadata.
    pub const fn record(&self) -> &EventRecord {
        &self.record
    }

    /// Returns the append cursor assigned by event history.
    pub const fn cursor(&self) -> StoreCursor {
        self.cursor
    }
}

impl StoredEventRecord {
    fn new(record: EventRecord, cursor: StoreCursor) -> Self {
        Self { record, cursor }
    }
}

/// Append-only committed hard history facade.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventHistoryStore {
    next_cursor: StoreCursor,
    transactions: BTreeMap<CausalTransactionId, StoredTransactionRecord>,
    transaction_order: BTreeMap<StoreCursor, CausalTransactionId>,
    events: BTreeMap<EventRecordId, StoredEventRecord>,
    event_order: BTreeMap<StoreCursor, EventRecordId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EventHistoryAppendPlan {
    transaction_cursor: StoreCursor,
    event_cursors: Vec<StoreCursor>,
    next_cursor: StoreCursor,
}

impl EventHistoryAppendPlan {
    pub(crate) const fn transaction_cursor(&self) -> StoreCursor {
        self.transaction_cursor
    }

    pub(crate) fn event_cursors(&self) -> &[StoreCursor] {
        &self.event_cursors
    }
}

impl Default for EventHistoryStore {
    fn default() -> Self {
        Self {
            next_cursor: StoreCursor::INITIAL,
            transactions: BTreeMap::new(),
            transaction_order: BTreeMap::new(),
            events: BTreeMap::new(),
            event_order: BTreeMap::new(),
        }
    }
}

impl EventHistoryStore {
    /// Returns a stored transaction by id.
    pub fn transaction(&self, id: CausalTransactionId) -> Option<&StoredTransactionRecord> {
        self.transactions.get(&id)
    }

    /// Returns a stored event by id.
    pub fn event(&self, id: EventRecordId) -> Option<&StoredEventRecord> {
        self.events.get(&id)
    }

    /// Returns the number of committed transactions.
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    /// Returns the number of committed events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns whether no committed history has been stored.
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty() && self.events.is_empty()
    }

    /// Iterates stored transactions in append order.
    pub fn transactions(&self) -> impl Iterator<Item = &StoredTransactionRecord> {
        self.transaction_order
            .values()
            .filter_map(|id| self.transactions.get(id))
    }

    /// Iterates stored events in append order.
    pub fn events(&self) -> impl Iterator<Item = &StoredEventRecord> {
        self.event_order
            .values()
            .filter_map(|id| self.events.get(id))
    }
}

impl EventHistoryStore {
    pub(crate) fn plan_append(
        &self,
        transaction: CausalTransactionId,
        events: &[EventRecordId],
    ) -> Result<EventHistoryAppendPlan, ModelError> {
        if self.transactions.contains_key(&transaction) {
            return Err(ModelError::DuplicateTransaction { transaction });
        }

        let mut seen_events = BTreeSet::new();
        for event in events {
            if self.events.contains_key(event) || !seen_events.insert(*event) {
                return Err(ModelError::DuplicateEvent { event: *event });
            }
        }

        let mut cursor = self.next_cursor;
        let mut cursors = Vec::with_capacity(events.len() + 1);
        for _ in 0..=events.len() {
            let Some(next) = cursor.next() else {
                return Err(ModelError::StoreCursorExhausted);
            };
            cursors.push(cursor);
            cursor = next;
        }

        let transaction_cursor = cursors[0];
        let event_cursors = cursors[1..].to_vec();
        Ok(EventHistoryAppendPlan {
            transaction_cursor,
            event_cursors,
            next_cursor: cursor,
        })
    }

    pub(crate) fn append_planned(
        &mut self,
        transaction: TransactionRecord,
        events: Vec<EventRecord>,
        plan: &EventHistoryAppendPlan,
    ) {
        let transaction_id = transaction.id();
        self.transactions.insert(
            transaction_id,
            StoredTransactionRecord::new(transaction, plan.transaction_cursor),
        );
        self.transaction_order
            .insert(plan.transaction_cursor, transaction_id);

        for (event, cursor) in events.into_iter().zip(plan.event_cursors.iter().copied()) {
            let event_id = event.id();
            self.events
                .insert(event_id, StoredEventRecord::new(event, cursor));
            self.event_order.insert(cursor, event_id);
        }

        self.next_cursor = plan.next_cursor;
    }

    #[cfg(test)]
    pub(crate) fn append_transaction(
        &mut self,
        record: TransactionRecord,
    ) -> Result<StoreCursor, ModelError> {
        let id = record.id();
        if self.transactions.contains_key(&id) {
            return Err(ModelError::DuplicateTransaction { transaction: id });
        }

        let cursor = self.reserve_cursor()?;
        self.transactions
            .insert(id, StoredTransactionRecord::new(record, cursor));
        self.transaction_order.insert(cursor, id);
        Ok(cursor)
    }

    #[cfg(test)]
    pub(crate) fn append_event(&mut self, record: EventRecord) -> Result<StoreCursor, ModelError> {
        let id = record.id();
        if self.events.contains_key(&id) {
            return Err(ModelError::DuplicateEvent { event: id });
        }

        let transaction = record.transaction();
        if !self.transactions.contains_key(&transaction) {
            return Err(ModelError::MissingTransaction { transaction });
        }

        let cursor = self.reserve_cursor()?;
        self.events
            .insert(id, StoredEventRecord::new(record, cursor));
        self.event_order.insert(cursor, id);
        Ok(cursor)
    }

    #[cfg(test)]
    fn reserve_cursor(&mut self) -> Result<StoreCursor, ModelError> {
        let cursor = self.next_cursor;
        let Some(next) = cursor.next() else {
            return Err(ModelError::StoreCursorExhausted);
        };
        self.next_cursor = next;
        Ok(cursor)
    }
}
