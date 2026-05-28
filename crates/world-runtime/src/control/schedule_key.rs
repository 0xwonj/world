use world_core::SimulationTime;

/// Scheduler request key before the runtime assigns the deterministic sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WakeupScheduleKey {
    time: SimulationTime,
    phase: u16,
    priority: i32,
}

impl WakeupScheduleKey {
    /// Creates a scheduler key without the runtime-owned sequence.
    #[must_use]
    pub const fn new(time: SimulationTime, phase: u16, priority: i32) -> Self {
        Self {
            time,
            phase,
            priority,
        }
    }

    /// Returns the scheduled simulation time.
    pub const fn time(self) -> SimulationTime {
        self.time
    }

    /// Returns the same-time scheduler phase.
    pub const fn phase(self) -> u16 {
        self.phase
    }

    /// Returns the phase-local priority.
    pub const fn priority(self) -> i32 {
        self.priority
    }
}
