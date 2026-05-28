use world_core::{ProvenanceKey, ScheduledWakeupId, SimulationTime};
use world_model::{AcceptedRuntimeControlUpdate, RuntimeControlApplication, WakeupTarget};

use crate::WakeupScheduleKey;

/// Request to schedule a durable wakeup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleWakeupRequest {
    pub(super) schedule: WakeupScheduleKey,
    pub(super) target: WakeupTarget,
    pub(super) submitted_at: SimulationTime,
    pub(super) provenance: Option<ProvenanceKey>,
}

impl ScheduleWakeupRequest {
    /// Creates a wakeup scheduling request.
    #[must_use]
    pub const fn new(
        schedule: WakeupScheduleKey,
        target: WakeupTarget,
        submitted_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            schedule,
            target,
            submitted_at,
            provenance,
        }
    }
}

/// Result of scheduling a wakeup through runtime control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledWakeupOutcome {
    wakeup: ScheduledWakeupId,
    application: RuntimeControlApplication,
}

impl ScheduledWakeupOutcome {
    pub(crate) const fn new(
        wakeup: ScheduledWakeupId,
        application: RuntimeControlApplication,
    ) -> Self {
        Self {
            wakeup,
            application,
        }
    }

    /// Returns the scheduled wakeup id.
    pub const fn wakeup(&self) -> ScheduledWakeupId {
        self.wakeup
    }

    /// Returns model application data.
    pub const fn application(&self) -> &RuntimeControlApplication {
        &self.application
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreparedWakeup {
    wakeup: ScheduledWakeupId,
    update: AcceptedRuntimeControlUpdate,
}

impl PreparedWakeup {
    pub(super) fn new(wakeup: ScheduledWakeupId, update: AcceptedRuntimeControlUpdate) -> Self {
        Self { wakeup, update }
    }

    pub(crate) fn into_parts(self) -> (ScheduledWakeupId, AcceptedRuntimeControlUpdate) {
        (self.wakeup, self.update)
    }
}
