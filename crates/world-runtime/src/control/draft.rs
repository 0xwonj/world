use world_core::{AuthorityClass, ProvenanceKey, ReplayLevel, ScheduledWakeupId, SimulationTime};
use world_model::{
    AcceptedRuntimeControlUpdate, InvalidationPackage, InvalidationSource, ModelError,
    ProcessInstanceRecord, ProcessLifecycle, RuntimeControlChange, RuntimeControlSource,
    RuntimeControlUpdateHeader, ScheduledWakeupRecord, StaleWakeupReason, StoreFamily,
    WakeupCancellationReason, WakeupConsumptionReason, WakeupTerminalTransition,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeControlDraft {
    source: RuntimeControlSource,
    occurred_at: SimulationTime,
    replay_level: ReplayLevel,
    provenance: Option<ProvenanceKey>,
    changes: Vec<RuntimeControlChange>,
}

impl RuntimeControlDraft {
    // If finalize lanes grow, consider lane-specific typestate/newtype wrappers here.
    pub(crate) fn new(
        source: RuntimeControlSource,
        occurred_at: SimulationTime,
        replay_level: ReplayLevel,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            source,
            occurred_at,
            replay_level,
            provenance,
            changes: Vec::new(),
        }
    }

    fn push_change(&mut self, change: RuntimeControlChange) {
        self.changes.push(change);
    }

    pub(crate) fn create_process(
        &mut self,
        updated_at: SimulationTime,
        process: ProcessInstanceRecord,
    ) {
        let provenance = process.provenance();
        self.push_change(RuntimeControlChange::CreateProcess {
            process,
            updated_at,
            provenance,
        });
    }

    pub(crate) fn update_process(
        &mut self,
        updated_at: SimulationTime,
        process: ProcessInstanceRecord,
    ) {
        let provenance = process.provenance();
        self.push_change(RuntimeControlChange::UpdateProcess {
            process,
            updated_at,
            provenance,
        });
    }

    pub(crate) fn schedule_wakeup(
        &mut self,
        updated_at: SimulationTime,
        wakeup: ScheduledWakeupRecord,
    ) {
        let provenance = wakeup.provenance();
        self.push_change(RuntimeControlChange::ScheduleWakeup {
            wakeup,
            updated_at,
            provenance,
        });
    }

    pub(crate) fn transition_wakeup(
        &mut self,
        wakeup: ScheduledWakeupId,
        transition: WakeupTerminalTransition,
    ) {
        self.push_change(RuntimeControlChange::TransitionWakeup { wakeup, transition });
    }

    pub(crate) fn cancel_current_wakeup(
        &mut self,
        lifecycle: &ProcessLifecycle,
        at: SimulationTime,
    ) {
        let ProcessLifecycle::Scheduled { wakeup } = lifecycle else {
            return;
        };
        self.transition_wakeup(
            *wakeup,
            WakeupTerminalTransition::Canceled {
                at,
                reason: WakeupCancellationReason::TargetCanceled,
            },
        );
    }

    pub(crate) fn consume_wakeup(
        &mut self,
        wakeup: ScheduledWakeupId,
        at: SimulationTime,
        reason: WakeupConsumptionReason,
    ) {
        self.transition_wakeup(wakeup, WakeupTerminalTransition::Consumed { at, reason });
    }

    pub(crate) fn skip_wakeup(
        &mut self,
        wakeup: ScheduledWakeupId,
        at: SimulationTime,
        reason: StaleWakeupReason,
    ) {
        self.transition_wakeup(wakeup, WakeupTerminalTransition::Skipped { at, reason });
    }

    pub(crate) fn cancel_wakeup(
        &mut self,
        wakeup: ScheduledWakeupId,
        at: SimulationTime,
        reason: WakeupCancellationReason,
    ) {
        self.transition_wakeup(wakeup, WakeupTerminalTransition::Canceled { at, reason });
    }

    pub(crate) fn accept_control_only(self) -> Result<AcceptedRuntimeControlUpdate, ModelError> {
        let mut invalidation = InvalidationPackage::new(InvalidationSource::RuntimeControl);
        invalidation
            .mark_authority_class(AuthorityClass::RuntimeControl)
            .mark_store_family(StoreFamily::RuntimeControl);

        AcceptedRuntimeControlUpdate::new(
            RuntimeControlUpdateHeader::new(
                self.source,
                self.occurred_at,
                self.replay_level,
                self.provenance,
            ),
            self.changes,
            invalidation,
        )
    }

    /// Returns runtime-control changes that will be committed inside a hard transaction.
    ///
    /// Draft metadata is intentionally not carried here because the surrounding
    /// hard commit owns transaction source, time, replay, and provenance.
    pub(crate) fn into_transaction_coupled_changes(self) -> RuntimeControlTransactionChanges {
        RuntimeControlTransactionChanges {
            changes: self.changes,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeControlTransactionChanges {
    changes: Vec<RuntimeControlChange>,
}

impl RuntimeControlTransactionChanges {
    pub(crate) fn into_changes(self) -> Vec<RuntimeControlChange> {
        self.changes
    }
}
