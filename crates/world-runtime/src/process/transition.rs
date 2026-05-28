use world_core::ScheduledWakeupId;
use world_model::{
    InterruptReason, PauseReason, ProcessFailureReason, ProcessInstanceRecord,
    RuntimeControlApplication, ScheduledWakeupRecord, StaleWakeupReason, WaitCondition,
};

/// Durable process transition computed by the process runtime.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessTransition {
    /// A process was created and scheduled.
    Started {
        /// Created process record.
        process: ProcessInstanceRecord,
        /// First scheduled wakeup.
        wakeup: ScheduledWakeupRecord,
    },
    /// A process advanced and scheduled a future tick.
    Rescheduled {
        /// Updated process record.
        process: ProcessInstanceRecord,
        /// Next scheduled wakeup.
        wakeup: ScheduledWakeupRecord,
    },
    /// A process completed.
    Completed {
        /// Completed process record.
        process: ProcessInstanceRecord,
    },
    /// A process failed.
    Failed {
        /// Failed process record.
        process: ProcessInstanceRecord,
        /// Failure reason.
        reason: ProcessFailureReason,
    },
    /// A process entered a wait state.
    Waiting {
        /// Waiting process record.
        process: ProcessInstanceRecord,
        /// Wait condition.
        condition: WaitCondition,
    },
    /// A process was paused.
    Paused {
        /// Paused process record.
        process: ProcessInstanceRecord,
        /// Pause reason.
        reason: PauseReason,
    },
    /// A process was interrupted.
    Interrupted {
        /// Interrupted process record.
        process: ProcessInstanceRecord,
        /// Interruption reason.
        reason: InterruptReason,
    },
    /// A process was resumed and scheduled.
    Resumed {
        /// Resumed process record.
        process: ProcessInstanceRecord,
        /// Next scheduled wakeup.
        wakeup: ScheduledWakeupRecord,
    },
    /// A process was abandoned.
    Abandoned {
        /// Abandoned process record.
        process: ProcessInstanceRecord,
    },
    /// A stale wakeup was skipped.
    Skipped {
        /// Skipped wakeup.
        wakeup: ScheduledWakeupId,
        /// Staleness reason.
        reason: StaleWakeupReason,
    },
}

/// Applied process runtime transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessControlOutcome {
    transition: ProcessTransition,
    application: RuntimeControlApplication,
}

impl ProcessControlOutcome {
    pub(crate) fn new(
        transition: ProcessTransition,
        application: RuntimeControlApplication,
    ) -> Self {
        Self {
            transition,
            application,
        }
    }

    /// Returns the transition computed by the process runtime.
    pub const fn transition(&self) -> &ProcessTransition {
        &self.transition
    }

    /// Returns the model application report.
    pub const fn application(&self) -> &RuntimeControlApplication {
        &self.application
    }
}

impl ProcessTransition {
    /// Returns the process record carried by process-state transitions.
    #[must_use]
    pub fn process(&self) -> Option<&ProcessInstanceRecord> {
        match self {
            Self::Started { process, .. }
            | Self::Rescheduled { process, .. }
            | Self::Completed { process }
            | Self::Failed { process, .. }
            | Self::Waiting { process, .. }
            | Self::Paused { process, .. }
            | Self::Interrupted { process, .. }
            | Self::Resumed { process, .. }
            | Self::Abandoned { process } => Some(process),
            Self::Skipped { .. } => None,
        }
    }
}
