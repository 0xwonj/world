use core::{fmt, num::NonZeroU64};

use world_core::{
    ActorId, CanonicalBytes, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest,
    SimDuration, SimTime,
};

use crate::{DirectedRoute, RelocationRouteId};

/// Canonical schema version of [`RelocationProcessId`].
pub const RELOCATION_PROCESS_ID_SCHEMA_VERSION: u16 = 1;
/// Canonical schema version of [`RelocationProcess`].
pub const RELOCATION_PROCESS_SCHEMA_VERSION: u16 = 1;

const RELOCATION_PROCESS_ID_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("relocation-process-id-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("relocation process identity domain must be valid"),
    };
const RELOCATION_PROCESS_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("relocation-process-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("relocation process domain must be valid"),
    };

/// Actor-local sequence of relocation processes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelocationProcessGeneration(u64);

impl RelocationProcessGeneration {
    /// Constructs an actor-local generation.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the exact generation scalar.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Stable semantic identity of one actor's relocation process.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelocationProcessId([u8; 32]);

impl RelocationProcessId {
    /// Derives identity from the actor, route, and actor-local generation.
    #[must_use]
    pub fn derive(
        actor: ActorId,
        route: RelocationRouteId,
        generation: RelocationProcessGeneration,
    ) -> Self {
        let mut writer = CanonicalWriter::new(RELOCATION_PROCESS_ID_DOMAIN);
        writer.write_u16(RELOCATION_PROCESS_ID_SCHEMA_VERSION);
        if writer.write_bytes(actor.as_bytes()).is_err()
            || writer.write_bytes(route.as_bytes()).is_err()
        {
            unreachable!("fixed-width relocation identities must fit canonical encoding");
        }
        writer.write_u64(generation.get());
        Self(ContentDigest::of_canonical(&writer.finish()).into_bytes())
    }

    /// Constructs an identity decoded by the process-state owner.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the exact identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the identity and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for RelocationProcessId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for RelocationProcessId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RelocationProcessId({self})")
    }
}

/// Canonical identity of one complete relocation-process value.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelocationProcessDigest(ContentDigest);

impl RelocationProcessDigest {
    /// Returns the exact digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Consumes the digest and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0.into_bytes()
    }
}

impl fmt::Display for RelocationProcessDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for RelocationProcessDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RelocationProcessDigest({self})")
    }
}

/// Compare-and-set version of one relocation process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelocationProcessVersion(NonZeroU64);

impl RelocationProcessVersion {
    /// Version of a newly started process.
    pub const INITIAL: Self = match NonZeroU64::new(1) {
        Some(value) => Self(value),
        None => unreachable!(),
    };

    /// Returns the exact nonzero version.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    fn checked_next(self) -> Option<Self> {
        self.get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(Self)
    }
}

/// Generation carried by one scheduled process-completion wake.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelocationWakeGeneration(NonZeroU64);

impl RelocationWakeGeneration {
    /// First completion wake of a newly started process.
    pub const INITIAL: Self = match NonZeroU64::new(1) {
        Some(value) => Self(value),
        None => unreachable!(),
    };

    /// Returns the exact nonzero generation.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    fn checked_next(self) -> Option<Self> {
        self.get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(Self)
    }
}

/// Runtime-control state of one exact relocation process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocationProcessStatus {
    /// Virtual time is currently advancing toward the route destination.
    Active {
        /// Start of the current uninterrupted active segment.
        segment_started_at: SimTime,
        /// Progress retained before the current active segment.
        elapsed_before_segment: SimDuration,
        /// Exact virtual time of the current completion wake.
        due_at: SimTime,
        /// Generation required for that wake to complete the process.
        wake_generation: RelocationWakeGeneration,
    },
    /// Progress is retained and no completion wake is current.
    Paused {
        /// Total completed virtual duration.
        elapsed: SimDuration,
        /// Generation that invalidated the preceding wake.
        wake_generation: RelocationWakeGeneration,
    },
    /// Arrival was accepted exactly once.
    Completed {
        /// Exact accepted arrival time.
        arrived_at: SimTime,
    },
}

/// Why a relocation process could not start or transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocationProcessError {
    /// Computing a due time overflowed the simulation timeline.
    TimeOverflow,
    /// The caller named an obsolete process version.
    StaleVersion {
        /// Required version.
        expected: RelocationProcessVersion,
        /// Current process version.
        actual: RelocationProcessVersion,
    },
    /// Only an active process may be paused.
    NotActive,
    /// Only a paused process may be resumed.
    NotPaused,
    /// A terminal process cannot transition again.
    AlreadyCompleted,
    /// A pause preceded its current active segment.
    PauseBeforeSegment,
    /// The completion wake was already due and must resolve first.
    CompletionAlreadyDue,
    /// A completion wake did not match the current active generation.
    StaleWake {
        /// Required generation.
        expected: RelocationWakeGeneration,
        /// Supplied generation.
        actual: RelocationWakeGeneration,
    },
    /// A completion wake fired at a time other than its exact due time.
    WrongCompletionTime {
        /// Required due time.
        expected: SimTime,
        /// Supplied time.
        actual: SimTime,
    },
    /// A version or wake generation could not advance without wrapping.
    GenerationOverflow,
}

impl fmt::Display for RelocationProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid relocation process transition: {self:?}")
    }
}

impl std::error::Error for RelocationProcessError {}

/// Immutable checked runtime-control value for one time-bearing relocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelocationProcess {
    id: RelocationProcessId,
    actor: ActorId,
    route: DirectedRoute,
    generation: RelocationProcessGeneration,
    version: RelocationProcessVersion,
    status: RelocationProcessStatus,
}

impl RelocationProcess {
    /// Starts a relocation and derives its first exact completion wake.
    pub fn start(
        actor: ActorId,
        route: DirectedRoute,
        generation: RelocationProcessGeneration,
        started_at: SimTime,
    ) -> Result<Self, RelocationProcessError> {
        let due_at = started_at
            .checked_add(route.duration())
            .ok_or(RelocationProcessError::TimeOverflow)?;
        Ok(Self {
            id: RelocationProcessId::derive(actor, route.id(), generation),
            actor,
            route,
            generation,
            version: RelocationProcessVersion::INITIAL,
            status: RelocationProcessStatus::Active {
                segment_started_at: started_at,
                elapsed_before_segment: SimDuration::ZERO,
                due_at,
                wake_generation: RelocationWakeGeneration::INITIAL,
            },
        })
    }

    /// Returns the process identity.
    #[must_use]
    pub const fn id(self) -> RelocationProcessId {
        self.id
    }

    /// Returns the relocating actor.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the exact accepted route.
    #[must_use]
    pub const fn route(self) -> DirectedRoute {
        self.route
    }

    /// Returns the actor-local process generation.
    #[must_use]
    pub const fn generation(self) -> RelocationProcessGeneration {
        self.generation
    }

    /// Returns the compare-and-set version.
    #[must_use]
    pub const fn version(self) -> RelocationProcessVersion {
        self.version
    }

    /// Returns the current process state.
    #[must_use]
    pub const fn status(self) -> RelocationProcessStatus {
        self.status
    }

    /// Returns the canonical identity of the complete process value.
    #[must_use]
    pub fn digest(self) -> RelocationProcessDigest {
        let preimage = self.canonical_preimage().unwrap_or_else(|error| {
            unreachable!("checked relocation process must be canonical: {error}")
        });
        RelocationProcessDigest(ContentDigest::of_canonical(&preimage))
    }

    /// Pauses active progress and invalidates its scheduled completion wake.
    pub fn pause(
        self,
        expected_version: RelocationProcessVersion,
        paused_at: SimTime,
    ) -> Result<Self, RelocationProcessError> {
        self.require_version(expected_version)?;
        let RelocationProcessStatus::Active {
            segment_started_at,
            elapsed_before_segment,
            due_at,
            wake_generation,
        } = self.status
        else {
            return Err(match self.status {
                RelocationProcessStatus::Completed { .. } => {
                    RelocationProcessError::AlreadyCompleted
                }
                RelocationProcessStatus::Paused { .. } => RelocationProcessError::NotActive,
                RelocationProcessStatus::Active { .. } => unreachable!(),
            });
        };
        if paused_at >= due_at {
            return Err(RelocationProcessError::CompletionAlreadyDue);
        }
        let segment_elapsed = paused_at
            .checked_duration_since(segment_started_at)
            .ok_or(RelocationProcessError::PauseBeforeSegment)?;
        let elapsed = elapsed_before_segment
            .checked_add(segment_elapsed)
            .ok_or(RelocationProcessError::TimeOverflow)?;
        Ok(Self {
            version: self.next_version()?,
            status: RelocationProcessStatus::Paused {
                elapsed,
                wake_generation: wake_generation
                    .checked_next()
                    .ok_or(RelocationProcessError::GenerationOverflow)?,
            },
            ..self
        })
    }

    /// Resumes retained progress and schedules only the remaining duration.
    pub fn resume(
        self,
        expected_version: RelocationProcessVersion,
        resumed_at: SimTime,
    ) -> Result<Self, RelocationProcessError> {
        self.require_version(expected_version)?;
        let RelocationProcessStatus::Paused {
            elapsed,
            wake_generation,
        } = self.status
        else {
            return Err(match self.status {
                RelocationProcessStatus::Completed { .. } => {
                    RelocationProcessError::AlreadyCompleted
                }
                RelocationProcessStatus::Active { .. } => RelocationProcessError::NotPaused,
                RelocationProcessStatus::Paused { .. } => unreachable!(),
            });
        };
        let remaining_ticks = self
            .route
            .duration()
            .ticks()
            .checked_sub(elapsed.ticks())
            .ok_or(RelocationProcessError::TimeOverflow)?;
        if remaining_ticks == 0 {
            return Err(RelocationProcessError::CompletionAlreadyDue);
        }
        let due_at = resumed_at
            .checked_add(SimDuration::from_ticks(remaining_ticks))
            .ok_or(RelocationProcessError::TimeOverflow)?;
        Ok(Self {
            version: self.next_version()?,
            status: RelocationProcessStatus::Active {
                segment_started_at: resumed_at,
                elapsed_before_segment: elapsed,
                due_at,
                wake_generation: wake_generation
                    .checked_next()
                    .ok_or(RelocationProcessError::GenerationOverflow)?,
            },
            ..self
        })
    }

    /// Completes exactly the current active wake at its due time.
    pub fn complete(
        self,
        expected_version: RelocationProcessVersion,
        wake: RelocationWakeGeneration,
        completed_at: SimTime,
    ) -> Result<Self, RelocationProcessError> {
        self.require_version(expected_version)?;
        let RelocationProcessStatus::Active {
            due_at,
            wake_generation,
            ..
        } = self.status
        else {
            return Err(match self.status {
                RelocationProcessStatus::Completed { .. } => {
                    RelocationProcessError::AlreadyCompleted
                }
                RelocationProcessStatus::Paused { .. } => RelocationProcessError::NotActive,
                RelocationProcessStatus::Active { .. } => unreachable!(),
            });
        };
        if wake != wake_generation {
            return Err(RelocationProcessError::StaleWake {
                expected: wake_generation,
                actual: wake,
            });
        }
        if completed_at != due_at {
            return Err(RelocationProcessError::WrongCompletionTime {
                expected: due_at,
                actual: completed_at,
            });
        }
        Ok(Self {
            version: self.next_version()?,
            status: RelocationProcessStatus::Completed {
                arrived_at: completed_at,
            },
            ..self
        })
    }

    fn require_version(
        self,
        expected: RelocationProcessVersion,
    ) -> Result<(), RelocationProcessError> {
        if expected != self.version {
            return Err(RelocationProcessError::StaleVersion {
                expected,
                actual: self.version,
            });
        }
        Ok(())
    }

    fn next_version(self) -> Result<RelocationProcessVersion, RelocationProcessError> {
        self.version
            .checked_next()
            .ok_or(RelocationProcessError::GenerationOverflow)
    }

    fn canonical_preimage(self) -> Result<CanonicalBytes, CanonicalError> {
        let mut writer = CanonicalWriter::new(RELOCATION_PROCESS_DOMAIN);
        writer.write_u16(RELOCATION_PROCESS_SCHEMA_VERSION);
        writer.write_bytes(self.id.as_bytes())?;
        writer.write_bytes(self.actor.as_bytes())?;
        writer.write_bytes(self.route.id().as_bytes())?;
        writer.write_bytes(self.route.source().as_bytes())?;
        writer.write_bytes(self.route.destination().as_bytes())?;
        writer.write_u64(self.route.duration().ticks());
        writer.write_u64(self.generation.get());
        writer.write_u64(self.version.get());
        match self.status {
            RelocationProcessStatus::Active {
                segment_started_at,
                elapsed_before_segment,
                due_at,
                wake_generation,
            } => {
                writer.write_discriminant(0);
                writer.write_u64(segment_started_at.ticks());
                writer.write_u64(elapsed_before_segment.ticks());
                writer.write_u64(due_at.ticks());
                writer.write_u64(wake_generation.get());
            }
            RelocationProcessStatus::Paused {
                elapsed,
                wake_generation,
            } => {
                writer.write_discriminant(1);
                writer.write_u64(elapsed.ticks());
                writer.write_u64(wake_generation.get());
            }
            RelocationProcessStatus::Completed { arrived_at } => {
                writer.write_discriminant(2);
                writer.write_u64(arrived_at.ticks());
            }
        }
        Ok(writer.finish())
    }
}

#[cfg(test)]
mod tests {
    use world_core::EntityId;

    use super::*;

    fn route() -> DirectedRoute {
        DirectedRoute::new(
            EntityId::from_bytes([0x21; 32]),
            EntityId::from_bytes([0x22; 32]),
            SimDuration::from_ticks(10),
        )
        .unwrap_or_else(|error| panic!("route fixture must be valid: {error}"))
    }

    #[test]
    fn pause_resume_preserves_progress_and_invalidates_old_wakes() {
        let process = RelocationProcess::start(
            ActorId::from_bytes([0x11; 32]),
            route(),
            RelocationProcessGeneration::new(3),
            SimTime::from_ticks(5),
        )
        .unwrap_or_else(|error| panic!("process fixture must start: {error}"));
        let RelocationProcessStatus::Active {
            due_at,
            wake_generation: first_wake,
            ..
        } = process.status()
        else {
            panic!("new process must be active");
        };
        assert_eq!(due_at, SimTime::from_ticks(15));

        let paused = process
            .pause(process.version(), SimTime::from_ticks(9))
            .unwrap_or_else(|error| panic!("active process must pause: {error}"));
        let RelocationProcessStatus::Paused {
            elapsed,
            wake_generation: invalidating_wake,
        } = paused.status()
        else {
            panic!("paused process must retain progress");
        };
        assert_eq!(elapsed, SimDuration::from_ticks(4));
        assert_ne!(first_wake, invalidating_wake);
        assert_ne!(process.digest(), paused.digest());

        let resumed = paused
            .resume(paused.version(), SimTime::from_ticks(20))
            .unwrap_or_else(|error| panic!("paused process must resume: {error}"));
        let RelocationProcessStatus::Active {
            due_at,
            wake_generation: resumed_wake,
            ..
        } = resumed.status()
        else {
            panic!("resumed process must be active");
        };
        assert_eq!(due_at, SimTime::from_ticks(26));
        assert_ne!(resumed_wake, invalidating_wake);
        assert!(matches!(
            resumed.complete(resumed.version(), first_wake, SimTime::from_ticks(26)),
            Err(RelocationProcessError::StaleWake { .. })
        ));

        let completed = resumed
            .complete(resumed.version(), resumed_wake, SimTime::from_ticks(26))
            .unwrap_or_else(|error| panic!("current wake must complete: {error}"));
        assert_eq!(
            completed.status(),
            RelocationProcessStatus::Completed {
                arrived_at: SimTime::from_ticks(26)
            }
        );
        assert_eq!(
            completed.complete(completed.version(), resumed_wake, SimTime::from_ticks(26)),
            Err(RelocationProcessError::AlreadyCompleted)
        );
        assert_ne!(resumed.digest(), completed.digest());
    }
}
