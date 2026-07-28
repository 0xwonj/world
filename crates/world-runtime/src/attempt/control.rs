use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use crate::authority::AuthorityCursor;
use crate::execution::ResolvedExecutionClosureManifestV1;
use crate::session::SessionHead;

use super::{
    AttemptCreation, AttemptDisposition, BoundCancelAttemptRequest,
    CancelAttemptRequestFingerprint, CancelAttemptRequestId, FinalizationBindingError,
    ReservationGrant, ReservedOperationDescriptor, RunFinalization, StepReservation,
    project_run_finalization,
};

/// Retained result of one successful attempt-cancellation request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelAttemptOutcome {
    /// The request selected the terminal prefix without changing world state.
    Cancelled {
        /// Immutable finalization installed by this request.
        finalization: RunFinalization,
    },
}

impl CancelAttemptOutcome {
    pub(crate) const fn cancelled(finalization: RunFinalization) -> Self {
        Self::Cancelled { finalization }
    }

    /// Returns the immutable terminal selection.
    #[must_use]
    pub const fn finalization(self) -> RunFinalization {
        match self {
            Self::Cancelled { finalization } => finalization,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CancellationLookup {
    Absent,
    RetainedExact(Box<CancelAttemptOutcome>),
    IdReuseMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CancellationEntry {
    fingerprint: CancelAttemptRequestFingerprint,
    outcome: CancelAttemptOutcome,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CancellationLedger {
    entries: BTreeMap<CancelAttemptRequestId, CancellationEntry>,
}

impl CancellationLedger {
    fn classify(
        &self,
        id: CancelAttemptRequestId,
        fingerprint: CancelAttemptRequestFingerprint,
    ) -> CancellationLookup {
        match self.entries.get(&id) {
            None => CancellationLookup::Absent,
            Some(entry) if entry.fingerprint == fingerprint => {
                CancellationLookup::RetainedExact(Box::new(entry.outcome))
            }
            Some(_) => CancellationLookup::IdReuseMismatch,
        }
    }

    fn insert(
        &mut self,
        id: CancelAttemptRequestId,
        fingerprint: CancelAttemptRequestFingerprint,
        outcome: CancelAttemptOutcome,
    ) -> Result<(), CancellationLedgerInsertError> {
        match self.entries.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(CancellationEntry {
                    fingerprint,
                    outcome,
                });
                Ok(())
            }
            Entry::Occupied(_) => Err(CancellationLedgerInsertError),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CancellationLedgerInsertError;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AttemptPhase {
    Active(AuthorityCursor),
    Reserved(Box<StepReservation>),
    Finalized(RunFinalization),
}

/// Repository-owned lifecycle and immutable binding of one physical attempt.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RunAttemptControl {
    creation: AttemptCreation,
    retained_closure: ResolvedExecutionClosureManifestV1,
    cancellation: CancellationLedger,
    next_reservation_grant: Option<ReservationGrant>,
    phase: AttemptPhase,
}

impl RunAttemptControl {
    pub(crate) fn new(
        creation: AttemptCreation,
        closure: ResolvedExecutionClosureManifestV1,
        root: &SessionHead,
    ) -> Result<Self, FinalizationBindingError> {
        let mut control = Self {
            creation,
            retained_closure: closure,
            cancellation: CancellationLedger::default(),
            next_reservation_grant: Some(ReservationGrant::FIRST),
            phase: AttemptPhase::Active(root.cursor()),
        };
        if let Some(finalization) = control.project_finalization(root, None)? {
            control.phase = AttemptPhase::Finalized(finalization);
        }
        Ok(control)
    }

    pub(crate) const fn creation(&self) -> AttemptCreation {
        self.creation
    }

    pub(crate) const fn binding(&self) -> super::AttemptBinding {
        self.creation.binding()
    }

    pub(crate) const fn closure(&self) -> &ResolvedExecutionClosureManifestV1 {
        &self.retained_closure
    }

    pub(crate) const fn phase(&self) -> &AttemptPhase {
        &self.phase
    }

    pub(crate) fn reservation(&self) -> Option<&StepReservation> {
        match &self.phase {
            AttemptPhase::Reserved(reservation) => Some(reservation),
            AttemptPhase::Active(_) | AttemptPhase::Finalized(_) => None,
        }
    }

    pub(crate) fn reservation_mut(&mut self) -> Option<&mut StepReservation> {
        match &mut self.phase {
            AttemptPhase::Reserved(reservation) => Some(reservation),
            AttemptPhase::Active(_) | AttemptPhase::Finalized(_) => None,
        }
    }

    pub(crate) fn reserve(
        &mut self,
        actual: AuthorityCursor,
        operation: ReservedOperationDescriptor,
    ) -> Result<(), AttemptPhaseError> {
        match self.phase {
            AttemptPhase::Active(cursor) if cursor == actual => {
                let grant = self
                    .next_reservation_grant
                    .ok_or(AttemptPhaseError::ReservationGrantExhausted)?;
                self.next_reservation_grant = grant.checked_next();
                self.phase = AttemptPhase::Reserved(Box::new(StepReservation::new(
                    self.binding(),
                    grant,
                    cursor,
                    operation,
                )));
                Ok(())
            }
            AttemptPhase::Active(cursor) => Err(AttemptPhaseError::CursorMismatch {
                control: Box::new(cursor),
                session: Box::new(actual),
            }),
            AttemptPhase::Reserved(_) => Err(AttemptPhaseError::StepReserved),
            AttemptPhase::Finalized(finalization) => {
                Err(AttemptPhaseError::Finalized(Box::new(finalization)))
            }
        }
    }

    pub(crate) fn activate_reserved(
        &mut self,
        step: super::AttemptStepId,
        cursor: AuthorityCursor,
    ) -> Result<(), AttemptPhaseError> {
        match &self.phase {
            AttemptPhase::Reserved(reservation) if reservation.step() == step => {
                self.phase = AttemptPhase::Active(cursor);
                Ok(())
            }
            AttemptPhase::Reserved(_) => Err(AttemptPhaseError::ReservationMismatch),
            AttemptPhase::Active(_) => Err(AttemptPhaseError::NoReservation),
            AttemptPhase::Finalized(finalization) => {
                Err(AttemptPhaseError::Finalized(Box::new(*finalization)))
            }
        }
    }

    pub(crate) fn finalize_reserved(
        &mut self,
        step: super::AttemptStepId,
        finalization: RunFinalization,
    ) -> Result<(), AttemptPhaseError> {
        match &self.phase {
            AttemptPhase::Reserved(reservation) if reservation.step() == step => {
                self.phase = AttemptPhase::Finalized(finalization);
                Ok(())
            }
            AttemptPhase::Reserved(_) => Err(AttemptPhaseError::ReservationMismatch),
            AttemptPhase::Active(_) => Err(AttemptPhaseError::NoReservation),
            AttemptPhase::Finalized(existing) => {
                Err(AttemptPhaseError::Finalized(Box::new(*existing)))
            }
        }
    }

    pub(crate) fn finalize_active(
        &mut self,
        expected: AuthorityCursor,
        finalization: RunFinalization,
    ) -> Result<(), AttemptPhaseError> {
        match self.phase {
            AttemptPhase::Active(cursor) if cursor == expected => {
                self.phase = AttemptPhase::Finalized(finalization);
                Ok(())
            }
            AttemptPhase::Active(cursor) => Err(AttemptPhaseError::CursorMismatch {
                control: Box::new(cursor),
                session: Box::new(expected),
            }),
            AttemptPhase::Reserved(_) => Err(AttemptPhaseError::StepReserved),
            AttemptPhase::Finalized(existing) => {
                Err(AttemptPhaseError::Finalized(Box::new(existing)))
            }
        }
    }

    pub(crate) fn project_finalization(
        &self,
        head: &SessionHead,
        disposition: Option<AttemptDisposition>,
    ) -> Result<Option<RunFinalization>, FinalizationBindingError> {
        project_run_finalization(
            self.binding(),
            head.cursor(),
            head.clock().now(),
            self.closure().specification().termination(),
            self.closure().semantics().config().finalization_policy(),
            disposition,
        )
    }

    pub(crate) fn classify_cancellation(
        &self,
        request: BoundCancelAttemptRequest,
    ) -> CancellationLookup {
        self.cancellation
            .classify(request.id(), request.fingerprint())
    }

    pub(crate) fn retain_cancellation(
        &mut self,
        request: BoundCancelAttemptRequest,
        outcome: CancelAttemptOutcome,
    ) -> Result<(), CancellationLedgerInsertError> {
        self.cancellation
            .insert(request.id(), request.fingerprint(), outcome)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AttemptPhaseError {
    ReservationGrantExhausted,
    StepReserved,
    NoReservation,
    ReservationMismatch,
    CursorMismatch {
        control: Box<AuthorityCursor>,
        session: Box<AuthorityCursor>,
    },
    Finalized(Box<RunFinalization>),
}
