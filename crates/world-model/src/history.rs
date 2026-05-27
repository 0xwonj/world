use std::collections::BTreeMap;

use world_core::{
    CausalTransactionId, DefinitionId, EventRecordId, ProvenanceKey, SimulationTime, StoreCursor,
};

#[cfg(test)]
use crate::ModelError;

/// Committed transaction metadata stored by event history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransactionRecord {
    id: CausalTransactionId,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}

impl TransactionRecord {
    /// Returns the transaction id.
    pub const fn id(&self) -> CausalTransactionId {
        self.id
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

#[cfg(test)]
impl TransactionRecord {
    pub(crate) const fn new(
        id: CausalTransactionId,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            id,
            occurred_at,
            provenance,
        }
    }
}

/// Committed event metadata stored by event history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRecord {
    id: EventRecordId,
    transaction: CausalTransactionId,
    event_definition: DefinitionId,
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

    /// Returns the checked event definition id.
    pub const fn event_definition(&self) -> DefinitionId {
        self.event_definition
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

#[cfg(test)]
impl EventRecord {
    pub(crate) const fn new(
        id: EventRecordId,
        transaction: CausalTransactionId,
        event_definition: DefinitionId,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            id,
            transaction,
            event_definition,
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
impl EventHistoryStore {
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

    fn reserve_cursor(&mut self) -> Result<StoreCursor, ModelError> {
        let cursor = self.next_cursor;
        let Some(next) = cursor.next() else {
            return Err(ModelError::StoreCursorExhausted);
        };
        self.next_cursor = next;
        Ok(cursor)
    }
}
