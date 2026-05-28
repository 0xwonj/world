use world_core::{
    ProvenanceKey, ReplayLevel, ReservationId, ScheduledWakeupId, SimulationTime, StoreCursor,
};

use crate::{DerivedViewInvalidationReport, InvalidationPackage, ModelError};

use super::{
    process::ProcessInstanceRecord,
    record::RuntimeControlRecordKind,
    reservation::{ReservationRecord, ReservationTransition},
    wakeup::{ScheduledWakeupRecord, WakeupTerminalTransition},
};

/// Source of accepted runtime-control work.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeControlSource {
    /// Host, UI shell, or external controller decision.
    Host,
    /// Scheduler-owned transition.
    Scheduler,
    /// Process runtime transition.
    ProcessRuntime,
    /// Causal runtime transaction-coupled transition.
    CausalRuntime,
    /// Tooling or test harness source.
    Tooling,
}
/// Metadata shared by runtime-control updates and history records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeControlUpdateHeader {
    source: RuntimeControlSource,
    occurred_at: SimulationTime,
    replay_level: ReplayLevel,
    provenance: Option<ProvenanceKey>,
}

impl RuntimeControlUpdateHeader {
    /// Creates runtime-control update metadata.
    pub const fn new(
        source: RuntimeControlSource,
        occurred_at: SimulationTime,
        replay_level: ReplayLevel,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            source,
            occurred_at,
            replay_level,
            provenance,
        }
    }

    /// Returns the source that accepted the runtime-control update.
    pub const fn source(self) -> RuntimeControlSource {
        self.source
    }

    /// Returns when the update occurred on the simulation timeline.
    pub const fn occurred_at(self) -> SimulationTime {
        self.occurred_at
    }

    /// Returns the replay level claimed by the update.
    pub const fn replay_level(self) -> ReplayLevel {
        self.replay_level
    }

    /// Returns update provenance, if known.
    pub const fn provenance(self) -> Option<ProvenanceKey> {
        self.provenance
    }
}

/// Accepted runtime-control update history record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeControlUpdateRecord {
    header: RuntimeControlUpdateHeader,
    changed: Vec<RuntimeControlRecordKind>,
}

impl RuntimeControlUpdateRecord {
    pub(super) fn new(
        header: RuntimeControlUpdateHeader,
        changed: impl IntoIterator<Item = RuntimeControlRecordKind>,
    ) -> Self {
        Self {
            header,
            changed: changed.into_iter().collect(),
        }
    }

    /// Returns accepted update metadata.
    pub const fn header(&self) -> RuntimeControlUpdateHeader {
        self.header
    }

    /// Returns runtime-control records changed by the update.
    pub fn changed(&self) -> &[RuntimeControlRecordKind] {
        &self.changed
    }
}

/// Runtime-control update record plus append cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRuntimeControlUpdate {
    record: RuntimeControlUpdateRecord,
    cursor: StoreCursor,
}

impl StoredRuntimeControlUpdate {
    pub(super) fn new(record: RuntimeControlUpdateRecord, cursor: StoreCursor) -> Self {
        Self { record, cursor }
    }

    /// Returns the accepted runtime-control update record.
    pub const fn record(&self) -> &RuntimeControlUpdateRecord {
        &self.record
    }

    /// Returns the append cursor assigned by runtime-control history.
    pub const fn cursor(&self) -> StoreCursor {
        self.cursor
    }
}

/// Runtime-control state change accepted by runtime authority.
///
/// These variants describe already-authorized process, scheduler, and
/// reservation transitions. General gameplay code should request runtime
/// operations instead of constructing these changes for direct model
/// application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeControlChange {
    /// Create a durable process instance.
    CreateProcess {
        /// Process record to create.
        process: ProcessInstanceRecord,
        /// Update time for the stored record envelope.
        updated_at: SimulationTime,
        /// Record provenance, if known.
        provenance: Option<ProvenanceKey>,
    },
    /// Update a durable process instance through a lifecycle/progress transition.
    UpdateProcess {
        /// Updated process record.
        process: ProcessInstanceRecord,
        /// Update time for the stored record envelope.
        updated_at: SimulationTime,
        /// Record provenance, if known.
        provenance: Option<ProvenanceKey>,
    },
    /// Schedule a durable wakeup.
    ScheduleWakeup {
        /// Wakeup record to schedule.
        wakeup: ScheduledWakeupRecord,
        /// Update time for the stored record envelope.
        updated_at: SimulationTime,
        /// Record provenance, if known.
        provenance: Option<ProvenanceKey>,
    },
    /// Apply a terminal transition to a scheduled wakeup.
    TransitionWakeup {
        /// Wakeup to transition.
        wakeup: ScheduledWakeupId,
        /// Terminal transition.
        transition: WakeupTerminalTransition,
    },
    /// Acquire an exclusive reservation.
    AcquireReservation {
        /// Held reservation record to create.
        reservation: ReservationRecord,
        /// Update time for the stored record envelope.
        updated_at: SimulationTime,
        /// Record provenance, if known.
        provenance: Option<ProvenanceKey>,
    },
    /// Transition a held reservation to a terminal state.
    TransitionReservation {
        /// Reservation to transition.
        reservation: ReservationId,
        /// Terminal transition.
        transition: ReservationTransition,
    },
}

/// Runtime-control package accepted by runtime authority.
///
/// Normal code should produce this package through `world-runtime` control
/// APIs. The constructor is public because the runtime producer is in another
/// crate; direct construction by other callers is an authority bypass guarded
/// by repository source allowlist tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedRuntimeControlUpdate {
    header: RuntimeControlUpdateHeader,
    changes: Vec<RuntimeControlChange>,
    invalidation: InvalidationPackage,
}

impl AcceptedRuntimeControlUpdate {
    /// Creates an accepted runtime-control update package.
    ///
    /// This is an accepted-package constructor for runtime-control producers,
    /// not a general store mutation API.
    pub fn new(
        header: RuntimeControlUpdateHeader,
        changes: impl IntoIterator<Item = RuntimeControlChange>,
        invalidation: InvalidationPackage,
    ) -> Result<Self, ModelError> {
        let changes = changes.into_iter().collect::<Vec<_>>();
        if changes.is_empty() {
            return Err(ModelError::EmptyItemField {
                type_name: "AcceptedRuntimeControlUpdate",
                field: "changes",
            });
        }

        Ok(Self {
            header,
            changes,
            invalidation,
        })
    }

    /// Returns update metadata.
    pub const fn header(&self) -> RuntimeControlUpdateHeader {
        self.header
    }

    /// Returns accepted runtime-control changes.
    pub fn changes(&self) -> &[RuntimeControlChange] {
        &self.changes
    }

    /// Returns invalidation package.
    pub const fn invalidation(&self) -> &InvalidationPackage {
        &self.invalidation
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        RuntimeControlUpdateHeader,
        Vec<RuntimeControlChange>,
        InvalidationPackage,
    ) {
        (self.header, self.changes, self.invalidation)
    }
}

/// Result of applying an accepted runtime-control update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeControlApplication {
    update_cursor: StoreCursor,
    changed_records: Vec<RuntimeControlRecordKind>,
    invalidation: DerivedViewInvalidationReport,
}

impl RuntimeControlApplication {
    pub(crate) fn new(
        update_cursor: StoreCursor,
        changed_records: Vec<RuntimeControlRecordKind>,
        invalidation: DerivedViewInvalidationReport,
    ) -> Self {
        Self {
            update_cursor,
            changed_records,
            invalidation,
        }
    }

    /// Returns the append cursor assigned to the update history record.
    pub const fn update_cursor(&self) -> StoreCursor {
        self.update_cursor
    }

    /// Returns changed runtime-control record keys.
    pub fn changed_records(&self) -> &[RuntimeControlRecordKind] {
        &self.changed_records
    }

    /// Returns model invalidation caused by the update.
    pub const fn invalidation(&self) -> DerivedViewInvalidationReport {
        self.invalidation
    }
}
