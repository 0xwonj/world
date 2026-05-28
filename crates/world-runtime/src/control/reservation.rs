use world_core::{ProvenanceKey, SimulationTime};
use world_model::{
    ReservationHolder, ReservationRecord, ReservationState, ReservationTarget, RuntimeControlChange,
};

use crate::RuntimeError;

use super::RuntimeControlIds;

/// Request to acquire one exclusive runtime reservation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcquireReservationRequest {
    holder: ReservationHolder,
    target: ReservationTarget,
    acquired_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}

impl AcquireReservationRequest {
    /// Creates a reservation acquisition request.
    #[must_use]
    pub const fn new(
        holder: ReservationHolder,
        target: ReservationTarget,
        acquired_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            holder,
            target,
            acquired_at,
            provenance,
        }
    }
}

pub(crate) struct ReservationRuntime;

impl ReservationRuntime {
    pub(crate) fn acquire(
        ids: &mut RuntimeControlIds,
        request: AcquireReservationRequest,
    ) -> Result<RuntimeControlChange, RuntimeError> {
        let reservation = ids.issue_reservation()?;
        let record = ReservationRecord::new(
            reservation,
            request.holder,
            request.target,
            ReservationState::Held {
                acquired_at: request.acquired_at,
            },
            request.provenance,
        );
        Ok(RuntimeControlChange::AcquireReservation {
            reservation: record,
            updated_at: request.acquired_at,
            provenance: request.provenance,
        })
    }
}
