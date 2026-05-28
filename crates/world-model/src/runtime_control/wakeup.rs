use world_core::{
    ProcessInstanceId, ProvenanceKey, ScheduledWakeupId, SimulationTime, WakeupOrderKey,
};

use super::update::RuntimeControlSource;

/// Wakeup target.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WakeupTarget {
    /// Host should receive an input opportunity.
    HostInputOpportunity,
    /// Process should be advanced.
    Process(ProcessInstanceId),
    /// Passive process should be advanced.
    PassiveProcess(ProcessInstanceId),
}

/// Wakeup consumption reason.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WakeupConsumptionReason {
    /// Scheduler consumed the wakeup for dispatch.
    Dispatched,
    /// Wakeup completed its target work.
    Completed,
}

/// Wakeup cancellation reason.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WakeupCancellationReason {
    /// Host or controller canceled the wakeup.
    Host,
    /// Target was canceled.
    TargetCanceled,
}

/// Stale wakeup reason.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StaleWakeupReason {
    /// Target process is missing.
    MissingProcess,
    /// Target process is already terminal.
    TerminalProcess,
    /// Wakeup is no longer current for the process.
    Superseded,
}

/// Durable scheduled wakeup state.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduledWakeupStatus {
    /// Wakeup is active and can become due.
    Scheduled,
    /// Wakeup was consumed.
    Consumed {
        at: SimulationTime,
        reason: WakeupConsumptionReason,
    },
    /// Wakeup was canceled.
    Canceled {
        at: SimulationTime,
        reason: WakeupCancellationReason,
    },
    /// Wakeup was skipped as stale.
    Skipped {
        at: SimulationTime,
        reason: StaleWakeupReason,
    },
}

impl ScheduledWakeupStatus {
    /// Returns whether the wakeup is active.
    #[must_use]
    pub const fn is_scheduled(&self) -> bool {
        matches!(self, Self::Scheduled)
    }
}

/// Durable scheduled wakeup record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledWakeupRecord {
    id: ScheduledWakeupId,
    order: WakeupOrderKey,
    target: WakeupTarget,
    status: ScheduledWakeupStatus,
    source: RuntimeControlSource,
    provenance: Option<ProvenanceKey>,
}

impl ScheduledWakeupRecord {
    /// Creates a scheduled wakeup record.
    #[must_use]
    pub fn new(
        id: ScheduledWakeupId,
        order: WakeupOrderKey,
        target: WakeupTarget,
        status: ScheduledWakeupStatus,
        source: RuntimeControlSource,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            id,
            order,
            target,
            status,
            source,
            provenance,
        }
    }

    /// Returns the wakeup id.
    pub const fn id(&self) -> ScheduledWakeupId {
        self.id
    }

    /// Returns scheduler ordering key.
    pub const fn order(&self) -> WakeupOrderKey {
        self.order
    }

    /// Returns the typed wakeup target.
    pub const fn target(&self) -> &WakeupTarget {
        &self.target
    }

    /// Returns wakeup status.
    pub const fn status(&self) -> &ScheduledWakeupStatus {
        &self.status
    }

    /// Returns wakeup source.
    pub const fn source(&self) -> RuntimeControlSource {
        self.source
    }

    /// Returns wakeup provenance, if known.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }

    #[must_use]
    pub(super) fn with_status(mut self, status: ScheduledWakeupStatus) -> Self {
        self.status = status;
        self
    }
}

/// Accepted terminal transition for a scheduled wakeup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WakeupTerminalTransition {
    /// Mark as consumed.
    Consumed {
        at: SimulationTime,
        reason: WakeupConsumptionReason,
    },
    /// Mark as canceled.
    Canceled {
        at: SimulationTime,
        reason: WakeupCancellationReason,
    },
    /// Mark as skipped.
    Skipped {
        at: SimulationTime,
        reason: StaleWakeupReason,
    },
}

impl From<WakeupTerminalTransition> for ScheduledWakeupStatus {
    fn from(transition: WakeupTerminalTransition) -> Self {
        match transition {
            WakeupTerminalTransition::Consumed { at, reason } => Self::Consumed { at, reason },
            WakeupTerminalTransition::Canceled { at, reason } => Self::Canceled { at, reason },
            WakeupTerminalTransition::Skipped { at, reason } => Self::Skipped { at, reason },
        }
    }
}
