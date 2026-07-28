use core::fmt;

use world_core::{EntityId, SimMoment, WorldRevision};
use world_model::WorldSnapshot;
use world_runtime::{
    AuthorityCursor, KernelSafetyBlocker, RuntimeReadError, RuntimeSessionRead,
    RuntimeSessionReader, SameTimeWaveTranche, SessionMode,
};

/// Cloneable read-only facade over one authoritative world session.
#[derive(Clone)]
pub struct WorldSession {
    runtime: RuntimeSessionReader,
}

impl WorldSession {
    pub(crate) const fn new(runtime: RuntimeSessionReader) -> Self {
        Self { runtime }
    }

    /// Copies one atomic read model from the authoritative aggregate.
    pub fn read(&self) -> Result<SessionRead, SessionReadError> {
        self.runtime
            .read()
            .map(SessionRead::from_runtime)
            .map_err(map_read_error)
    }

    /// Copies the current authority cursor.
    pub fn cursor(&self) -> Result<AuthorityCursor, SessionReadError> {
        self.read().map(|read| read.cursor())
    }

    /// Copies one immutable snapshot from a single aggregate read.
    pub fn snapshot(&self) -> Result<WorldSnapshot, SessionReadError> {
        self.read().map(SessionRead::into_snapshot)
    }

    /// Creates another read-only capability for focused inspection.
    #[must_use]
    pub fn inspector(&self) -> Inspector {
        Inspector {
            runtime: self.runtime.clone(),
        }
    }
}

/// One atomic, read-only image of the public session state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRead {
    cursor: AuthorityCursor,
    mode: SessionMode,
    admission_frontier: SimMoment,
    snapshot: WorldSnapshot,
    safety_blocker: Option<KernelSafetyBlocker>,
    same_time_wave_tranche: SameTimeWaveTranche,
}

impl SessionRead {
    fn from_runtime(read: RuntimeSessionRead) -> Self {
        Self {
            cursor: read.cursor(),
            mode: read.mode(),
            admission_frontier: read.admission_frontier(),
            snapshot: read.snapshot().clone(),
            safety_blocker: read.safety_blocker(),
            same_time_wave_tranche: read.same_time_wave_tranche(),
        }
    }

    /// Returns the authority cursor read with this image.
    #[must_use]
    pub const fn cursor(&self) -> AuthorityCursor {
        self.cursor
    }

    /// Returns the session mode read with this image.
    #[must_use]
    pub const fn mode(&self) -> SessionMode {
        self.mode
    }

    /// Returns the first simulation moment open to new command ingress.
    #[must_use]
    pub const fn admission_frontier(&self) -> SimMoment {
        self.admission_frontier
    }

    /// Returns the immutable world snapshot read with this image.
    #[must_use]
    pub const fn snapshot(&self) -> &WorldSnapshot {
        &self.snapshot
    }

    /// Returns the deterministic cause currently blocking ordinary work.
    #[must_use]
    pub const fn safety_blocker(&self) -> Option<KernelSafetyBlocker> {
        self.safety_blocker
    }

    /// Returns published-wave accounting for the current simulation-time tranche.
    #[must_use]
    pub const fn same_time_wave_tranche(&self) -> SameTimeWaveTranche {
        self.same_time_wave_tranche
    }

    fn into_snapshot(self) -> WorldSnapshot {
        self.snapshot
    }
}

/// Cloneable focused read capability.
#[derive(Clone)]
pub struct Inspector {
    runtime: RuntimeSessionReader,
}

impl Inspector {
    /// Reads one direct-containment projection with its exact snapshot revision.
    pub fn direct_container(
        &self,
        item: EntityId,
    ) -> Result<ContainmentInspection, SessionReadError> {
        let read = self.runtime.read().map_err(map_read_error)?;
        let snapshot = read.snapshot();
        Ok(ContainmentInspection {
            revision: snapshot.revision(),
            container: snapshot
                .accepted()
                .domain()
                .containment_for(item)
                .map(|record| record.container()),
        })
    }
}

/// One direct-containment read and the revision from the same aggregate copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainmentInspection {
    revision: WorldRevision,
    container: Option<EntityId>,
}

impl ContainmentInspection {
    /// Returns the revision read atomically with the projection.
    #[must_use]
    pub const fn revision(self) -> WorldRevision {
        self.revision
    }

    /// Returns the item's direct container, if present.
    #[must_use]
    pub const fn container(self) -> Option<EntityId> {
        self.container
    }
}

/// Failure of a read-only session operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionReadError {
    /// The bound attempt no longer exists in the authority domain.
    AttemptNotFound,
    /// The authority service could not be accessed.
    Unavailable,
}

impl fmt::Display for SessionReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "session read failed: {self:?}")
    }
}

impl std::error::Error for SessionReadError {}

fn map_read_error(error: RuntimeReadError) -> SessionReadError {
    match error {
        RuntimeReadError::AttemptNotFound => SessionReadError::AttemptNotFound,
        RuntimeReadError::Unavailable => SessionReadError::Unavailable,
    }
}
