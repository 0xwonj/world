use world_core::{
    ProcessInstanceId, ProcessInstanceIdIssuer, ReservationId, ReservationIdIssuer,
    ScheduledWakeupId, ScheduledWakeupIdIssuer, WakeupOrderKey,
};
use world_model::{ModelError, RuntimeControlRecordPayload, RuntimeControlStore};

use crate::{RuntimeError, control::WakeupScheduleKey};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RuntimeControlIds {
    process_ids: ProcessInstanceIdIssuer,
    reservation_ids: ReservationIdIssuer,
    wakeup_ids: ScheduledWakeupIdIssuer,
    next_wakeup_sequence: u64,
}

impl RuntimeControlIds {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn from_store(store: &RuntimeControlStore) -> Result<Self, RuntimeError> {
        let mut next_process = 1_u64;
        let mut next_reservation = 1_u64;
        let mut next_wakeup = 1_u64;
        let mut next_wakeup_sequence = 0_u64;

        for record in store.records() {
            match record.payload() {
                RuntimeControlRecordPayload::Process(process) => {
                    next_process = next_issued_value(next_process, process.id().get())?;
                }
                RuntimeControlRecordPayload::Reservation(reservation) => {
                    next_reservation = next_issued_value(next_reservation, reservation.id().get())?;
                }
                RuntimeControlRecordPayload::ScheduledWakeup(wakeup) => {
                    next_wakeup = next_issued_value(next_wakeup, wakeup.id().get())?;
                    next_wakeup_sequence =
                        next_issued_value(next_wakeup_sequence, wakeup.order().sequence())?;
                }
            }
        }

        Ok(Self {
            process_ids: ProcessInstanceIdIssuer::starting_at(next_process)
                .map_err(|_| ModelError::RuntimeControlValueOverflow)?,
            reservation_ids: ReservationIdIssuer::starting_at(next_reservation)
                .map_err(|_| ModelError::RuntimeControlValueOverflow)?,
            wakeup_ids: ScheduledWakeupIdIssuer::starting_at(next_wakeup)
                .map_err(|_| ModelError::RuntimeControlValueOverflow)?,
            next_wakeup_sequence,
        })
    }

    pub(crate) fn issue_process(&mut self) -> Result<ProcessInstanceId, RuntimeError> {
        self.process_ids
            .issue()
            .ok_or(RuntimeError::ProcessInstanceIdExhausted)
    }

    pub(crate) fn issue_reservation(&mut self) -> Result<ReservationId, RuntimeError> {
        self.reservation_ids
            .issue()
            .ok_or(RuntimeError::ReservationIdExhausted)
    }

    pub(crate) fn issue_wakeup(&mut self) -> Result<ScheduledWakeupId, RuntimeError> {
        self.wakeup_ids
            .issue()
            .ok_or(RuntimeError::ScheduledWakeupIdExhausted)
    }

    pub(crate) fn issue_order(
        &mut self,
        schedule: WakeupScheduleKey,
    ) -> Result<WakeupOrderKey, RuntimeError> {
        let sequence = self.next_wakeup_sequence;
        self.next_wakeup_sequence = self
            .next_wakeup_sequence
            .checked_add(1)
            .ok_or(ModelError::RuntimeControlValueOverflow)?;
        Ok(WakeupOrderKey::new(
            schedule.time(),
            schedule.phase(),
            schedule.priority(),
            sequence,
        ))
    }
}

fn next_issued_value(current_next: u64, observed: u64) -> Result<u64, RuntimeError> {
    let observed_next = observed
        .checked_add(1)
        .ok_or(ModelError::RuntimeControlValueOverflow)?;
    Ok(current_next.max(observed_next))
}
