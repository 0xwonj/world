use world_core::{ReservationId, ScheduledWakeupId};

/// Process wait condition.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WaitCondition {
    /// Waiting for a scheduler wakeup.
    Wakeup(ScheduledWakeupId),
    /// Waiting for a reservation to become available.
    Reservation(ReservationId),
    /// Waiting on host or policy input.
    Host,
}

/// Pause reason.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PauseReason {
    /// Host requested pause.
    Host,
    /// Process is waiting on another runtime-control condition.
    Waiting,
}

/// Interruption reason.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InterruptReason {
    /// Host or player-facing controller interrupted the process.
    Host,
    /// A reservation needed by the process is no longer held.
    ReservationLost,
}

/// Process failure reason.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProcessFailureReason {
    /// Checked process definition is missing.
    MissingDefinition,
    /// Process definition does not support the requested resolution tier.
    UnsupportedResolution,
    /// Process state is invalid for the requested transition.
    InvalidState,
    /// Runtime work was blocked.
    Blocked,
}

/// Durable process lifecycle state.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessLifecycle {
    /// Process has been created but not scheduled.
    Created,
    /// Process has an active scheduled wakeup.
    Scheduled { wakeup: ScheduledWakeupId },
    /// Process is waiting on a condition.
    Waiting { condition: WaitCondition },
    /// Source wakeup has been consumed and durable execution/progress is in flight.
    ///
    /// Current atomic ticks may move directly from `Scheduled` to a result state.
    /// This state is for future non-atomic process execution claims that must
    /// survive save/load before publishing the final transition.
    Advancing,
    /// Process is paused.
    Paused { reason: PauseReason },
    /// Process was interrupted.
    Interrupted { reason: InterruptReason },
    /// Process completed successfully.
    Completed,
    /// Process failed.
    Failed { reason: ProcessFailureReason },
    /// Process was abandoned.
    Abandoned,
}

impl ProcessLifecycle {
    /// Returns whether ordinary ticks should not advance this process.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed { .. } | Self::Abandoned
        )
    }
}
