use world_core::{
    ProcessInstanceId, ProvenanceKey, ReservationId, ScheduledWakeupId, SimulationTime,
};

use super::{
    process::ProcessInstanceRecord, reservation::ReservationRecord, wakeup::ScheduledWakeupRecord,
};

/// Durable runtime-control record identity.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeControlRecordKind {
    /// Durable process instance.
    Process(ProcessInstanceId),
    /// Runtime conflict-control reservation.
    Reservation(ReservationId),
    /// Scheduler wakeup state.
    ScheduledWakeup(ScheduledWakeupId),
}

/// Runtime-control record payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeControlRecordPayload {
    /// Durable process instance state.
    Process(ProcessInstanceRecord),
    /// Runtime conflict-control reservation state.
    Reservation(ReservationRecord),
    /// Durable scheduler wakeup state.
    ScheduledWakeup(ScheduledWakeupRecord),
}

impl RuntimeControlRecordPayload {
    /// Returns the identity key for the payload.
    #[must_use]
    pub const fn kind(&self) -> RuntimeControlRecordKind {
        match self {
            Self::Process(record) => RuntimeControlRecordKind::Process(record.id()),
            Self::Reservation(record) => RuntimeControlRecordKind::Reservation(record.id()),
            Self::ScheduledWakeup(record) => RuntimeControlRecordKind::ScheduledWakeup(record.id()),
        }
    }
}

/// Durable runtime-control record envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeControlRecord {
    payload: RuntimeControlRecordPayload,
    updated_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}

impl RuntimeControlRecord {
    /// Creates a runtime-control record envelope.
    #[must_use]
    pub const fn new(
        payload: RuntimeControlRecordPayload,
        updated_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            payload,
            updated_at,
            provenance,
        }
    }

    /// Returns the record identity derived from its payload.
    pub const fn kind(&self) -> RuntimeControlRecordKind {
        self.payload.kind()
    }

    /// Returns the record payload.
    pub const fn payload(&self) -> &RuntimeControlRecordPayload {
        &self.payload
    }

    /// Returns the update time.
    pub const fn updated_at(&self) -> SimulationTime {
        self.updated_at
    }

    /// Returns record provenance, if known.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }
}
