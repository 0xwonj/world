/// Integer simulation time on the shared runtime timeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimulationTime {
    ticks: u64,
}

impl SimulationTime {
    /// Earliest simulation instant.
    pub const ZERO: Self = Self { ticks: 0 };

    /// Creates a simulation time from integer ticks.
    pub const fn from_ticks(ticks: u64) -> Self {
        Self { ticks }
    }

    /// Returns the integer tick value.
    pub const fn ticks(self) -> u64 {
        self.ticks
    }

    /// Adds a duration, returning `None` on overflow.
    pub const fn checked_add(self, duration: SimulationDuration) -> Option<Self> {
        match self.ticks.checked_add(duration.ticks) {
            Some(ticks) => Some(Self { ticks }),
            None => None,
        }
    }

    /// Computes the elapsed duration from an earlier time.
    pub const fn checked_duration_since(self, earlier: Self) -> Option<SimulationDuration> {
        match self.ticks.checked_sub(earlier.ticks) {
            Some(ticks) => Some(SimulationDuration { ticks }),
            None => None,
        }
    }
}

/// Non-wall-clock duration on the simulation timeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimulationDuration {
    ticks: u64,
}

impl SimulationDuration {
    /// Zero elapsed simulation time.
    pub const ZERO: Self = Self { ticks: 0 };

    /// Creates a simulation duration from integer ticks.
    pub const fn from_ticks(ticks: u64) -> Self {
        Self { ticks }
    }

    /// Returns the integer tick value.
    pub const fn ticks(self) -> u64 {
        self.ticks
    }

    /// Returns whether the duration has no elapsed ticks.
    pub const fn is_zero(self) -> bool {
        self.ticks == 0
    }

    /// Adds two durations, returning `None` on overflow.
    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.ticks.checked_add(other.ticks) {
            Some(ticks) => Some(Self { ticks }),
            None => None,
        }
    }
}
