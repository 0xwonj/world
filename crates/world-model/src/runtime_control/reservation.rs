use world_core::{
    DefinitionId, EntityId, ProcessInstanceId, ProvenanceKey, ReservationId, SimulationTime,
};

/// Reservation holder.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReservationHolder {
    /// A process holds the reservation.
    Process(ProcessInstanceId),
    /// An entity holds the reservation.
    Entity(EntityId),
    /// Runtime-owned reservation.
    Runtime,
}

/// Reservation target.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReservationTarget {
    /// An entity is reserved.
    Entity(EntityId),
    /// A process is reserved.
    Process(ProcessInstanceId),
    /// A checked definition is reserved.
    Definition(DefinitionId),
}

/// Reservation cancellation reason.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReservationCancelReason {
    /// Host or controller canceled the reservation.
    Host,
    /// Owning process ended.
    OwnerEnded,
    /// Reservation became stale.
    Stale,
}

/// Durable reservation state.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReservationState {
    /// Reservation is active.
    Held { acquired_at: SimulationTime },
    /// Reservation was released.
    Released {
        acquired_at: SimulationTime,
        released_at: SimulationTime,
    },
    /// Reservation was canceled.
    Canceled {
        acquired_at: SimulationTime,
        canceled_at: SimulationTime,
        reason: ReservationCancelReason,
    },
}

impl ReservationState {
    /// Returns whether the reservation is active.
    #[must_use]
    pub const fn is_held(&self) -> bool {
        matches!(self, Self::Held { .. })
    }
}

/// Durable reservation record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationRecord {
    id: ReservationId,
    holder: ReservationHolder,
    target: ReservationTarget,
    state: ReservationState,
    provenance: Option<ProvenanceKey>,
}

impl ReservationRecord {
    /// Creates a reservation record.
    #[must_use]
    pub fn new(
        id: ReservationId,
        holder: ReservationHolder,
        target: ReservationTarget,
        state: ReservationState,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            id,
            holder,
            target,
            state,
            provenance,
        }
    }

    /// Returns the reservation id.
    pub const fn id(&self) -> ReservationId {
        self.id
    }

    /// Returns the reservation holder.
    pub const fn holder(&self) -> &ReservationHolder {
        &self.holder
    }

    /// Returns the reserved target.
    pub const fn target(&self) -> &ReservationTarget {
        &self.target
    }

    /// Returns reservation state.
    pub const fn state(&self) -> &ReservationState {
        &self.state
    }

    /// Returns provenance, if known.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }
}

/// Accepted terminal transition for a reservation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReservationTransition {
    /// Mark as released.
    Released { at: SimulationTime },
    /// Mark as canceled.
    Canceled {
        at: SimulationTime,
        reason: ReservationCancelReason,
    },
}

impl ReservationTransition {
    #[must_use]
    pub(super) fn transition_time(&self) -> SimulationTime {
        match self {
            Self::Released { at } | Self::Canceled { at, .. } => *at,
        }
    }

    pub(super) fn into_state(self, acquired_at: SimulationTime) -> ReservationState {
        match self {
            Self::Released { at } => ReservationState::Released {
                acquired_at,
                released_at: at,
            },
            Self::Canceled { at, reason } => ReservationState::Canceled {
                acquired_at,
                canceled_at: at,
                reason,
            },
        }
    }
}
