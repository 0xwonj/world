use std::collections::BTreeMap;

use world_core::{
    ActivityId, ProcessInstanceId, ProvenanceKey, ReservationId, RngDrawId, RngStreamId,
    ScheduledWakeupId,
};

#[cfg(test)]
use crate::ModelError;

/// Durable runtime-control record identity.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeControlRecordKind {
    /// Durable process instance.
    Process(ProcessInstanceId),
    /// Durable activity state.
    Activity(ActivityId),
    /// Runtime conflict-control reservation.
    Reservation(ReservationId),
    /// Scheduler wakeup state.
    ScheduledWakeup(ScheduledWakeupId),
    /// Deterministic random stream state.
    RngStream(RngStreamId),
    /// Deterministic random draw record.
    RngDraw(RngDrawId),
}

/// Runtime-control record envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeControlRecord {
    kind: RuntimeControlRecordKind,
    provenance: Option<ProvenanceKey>,
}

impl RuntimeControlRecord {
    /// Returns the runtime-control record kind.
    pub const fn kind(&self) -> RuntimeControlRecordKind {
        self.kind
    }

    /// Returns record provenance, if known.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }
}

#[cfg(test)]
impl RuntimeControlRecord {
    pub(crate) const fn new(
        kind: RuntimeControlRecordKind,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self { kind, provenance }
    }
}

/// Store for durable runtime-control state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeControlStore {
    records: BTreeMap<RuntimeControlRecordKind, RuntimeControlRecord>,
}

impl RuntimeControlStore {
    /// Returns whether a runtime-control record exists.
    pub fn contains(&self, kind: RuntimeControlRecordKind) -> bool {
        self.records.contains_key(&kind)
    }

    /// Returns a runtime-control record.
    pub fn record(&self, kind: RuntimeControlRecordKind) -> Option<&RuntimeControlRecord> {
        self.records.get(&kind)
    }

    /// Returns the number of runtime-control records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Iterates runtime-control records in key order.
    pub fn records(&self) -> impl Iterator<Item = &RuntimeControlRecord> {
        self.records.values()
    }
}

#[cfg(test)]
impl RuntimeControlStore {
    pub(crate) fn insert(&mut self, record: RuntimeControlRecord) -> Result<(), ModelError> {
        let kind = record.kind();
        if self.records.contains_key(&kind) {
            return Err(ModelError::DuplicateRuntimeControlRecord { kind });
        }

        self.records.insert(kind, record);
        Ok(())
    }
}
