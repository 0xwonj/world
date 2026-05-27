use crate::SimulationTime;

/// Canonical scheduler ordering key: time, phase, priority, then sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WakeupOrderKey {
    time: SimulationTime,
    phase: u16,
    priority: i32,
    sequence: u64,
}

impl WakeupOrderKey {
    /// Creates a scheduler ordering key.
    pub const fn new(time: SimulationTime, phase: u16, priority: i32, sequence: u64) -> Self {
        Self {
            time,
            phase,
            priority,
            sequence,
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

    /// Returns the same-key tie breaker.
    pub const fn sequence(self) -> u64 {
        self.sequence
    }
}

/// Cursor into an append-only store or history surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoreCursor {
    value: u64,
}

impl StoreCursor {
    /// Initial cursor position.
    pub const INITIAL: Self = Self { value: 0 };

    /// Creates a store cursor from a raw position.
    pub const fn new(value: u64) -> Self {
        Self { value }
    }

    /// Returns the raw cursor position.
    pub const fn get(self) -> u64 {
        self.value
    }

    /// Returns the next cursor, or `None` on overflow.
    pub const fn next(self) -> Option<Self> {
        match self.value.checked_add(1) {
            Some(value) => Some(Self { value }),
            None => None,
        }
    }
}

/// Epoch used to distinguish read surfaces and cached query results.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryEpoch {
    value: u64,
}

impl QueryEpoch {
    /// Initial query epoch.
    pub const INITIAL: Self = Self { value: 0 };

    /// Creates a query epoch from a raw value.
    pub const fn new(value: u64) -> Self {
        Self { value }
    }

    /// Returns the raw epoch value.
    pub const fn get(self) -> u64 {
        self.value
    }

    /// Returns the next query epoch, or `None` on overflow.
    pub const fn next(self) -> Option<Self> {
        match self.value.checked_add(1) {
            Some(value) => Some(Self { value }),
            None => None,
        }
    }
}
