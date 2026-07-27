/// Integer coordinate on the session's virtual timeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimTime {
    ticks: u64,
}

impl SimTime {
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
    pub const fn checked_add(self, duration: SimDuration) -> Option<Self> {
        match self.ticks.checked_add(duration.ticks) {
            Some(ticks) => Some(Self { ticks }),
            None => None,
        }
    }

    /// Computes the elapsed duration from an earlier time.
    pub const fn checked_duration_since(self, earlier: Self) -> Option<SimDuration> {
        match self.ticks.checked_sub(earlier.ticks) {
            Some(ticks) => Some(SimDuration { ticks }),
            None => None,
        }
    }
}

/// Non-wall-clock duration on the simulation timeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimDuration {
    ticks: u64,
}

impl SimDuration {
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

/// Causal index for work that consumes no modeled time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Microstep {
    value: u64,
}

impl Microstep {
    /// First causal position at one simulation time.
    pub const ZERO: Self = Self { value: 0 };

    /// Creates a microstep from its exact causal index.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self { value }
    }

    /// Returns the causal index.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.value
    }

    /// Returns the next same-time causal index, or `None` on overflow.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.value.checked_add(1) {
            Some(value) => Some(Self { value }),
            None => None,
        }
    }
}

/// Exact scheduler position ordered by simulation time and then microstep.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimMoment {
    time: SimTime,
    microstep: Microstep,
}

impl SimMoment {
    /// Initial scheduler position.
    pub const ORIGIN: Self = Self::new(SimTime::ZERO, Microstep::ZERO);

    /// Creates an exact scheduler position.
    #[must_use]
    pub const fn new(time: SimTime, microstep: Microstep) -> Self {
        Self { time, microstep }
    }

    /// Creates the first causal position at `time`.
    #[must_use]
    pub const fn at(time: SimTime) -> Self {
        Self::new(time, Microstep::ZERO)
    }

    /// Returns the simulation-time coordinate.
    #[must_use]
    pub const fn time(self) -> SimTime {
        self.time
    }

    /// Returns the same-time causal index.
    #[must_use]
    pub const fn microstep(self) -> Microstep {
        self.microstep
    }

    /// Advances within the same simulation time, or returns `None` on
    /// microstep overflow.
    #[must_use]
    pub const fn checked_next_microstep(self) -> Option<Self> {
        match self.microstep.checked_next() {
            Some(microstep) => Some(Self::new(self.time, microstep)),
            None => None,
        }
    }
}
