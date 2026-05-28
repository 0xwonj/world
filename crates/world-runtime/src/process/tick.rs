use world_core::{ProcessInstanceId, ProvenanceKey, ScheduledWakeupId, SimulationTime};

/// A process wakeup selected by the scheduler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessTick {
    process: ProcessInstanceId,
    pub(super) occurred_at: SimulationTime,
    source_wakeup: ScheduledWakeupId,
    pub(super) provenance: Option<ProvenanceKey>,
}

impl ProcessTick {
    /// Creates a process tick.
    #[must_use]
    pub const fn new(
        process: ProcessInstanceId,
        occurred_at: SimulationTime,
        source_wakeup: ScheduledWakeupId,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            process,
            occurred_at,
            source_wakeup,
            provenance,
        }
    }

    /// Returns the process to advance.
    pub const fn process(self) -> ProcessInstanceId {
        self.process
    }

    /// Returns the wakeup that caused this tick.
    pub const fn source_wakeup(self) -> ScheduledWakeupId {
        self.source_wakeup
    }
}
