use world_core::{
    CausalTransactionIdIssuer, ProvenanceKey, ReplayLevel, ScheduledWakeupId, SimulationTime,
};
use world_defs::DefinitionRegistry;
use world_model::{
    AcceptedRuntimeControlUpdate, DerivedViewInvalidationReport, InvalidationPackage,
    InvalidationSource, ProcessFailureReason, ProcessInstanceRecord, RuntimeControlSource,
    ScheduledWakeupRecord, ScheduledWakeupStatus, StaleWakeupReason, TransactionCause,
    WakeupCancellationReason, WakeupConsumptionReason, WakeupTarget, WorldModel,
};

use crate::{
    RequestSource, RuntimeError,
    control::{RuntimeControlDraft, RuntimeControlIds, RuntimeControlTransactionChanges},
    process::{ProcessRuntime, ProcessTransition},
    transaction::{CausalTransactionBuilder, CausalTransactionHeader, CommitFinalizer},
};

use super::{PreparedWakeup, ScheduleWakeupRequest};

/// Upper bound for scheduler work processed by one drain call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DrainBudget {
    max_wakeups: usize,
}

impl DrainBudget {
    /// Default maximum wakeups processed by one drain call.
    pub const DEFAULT_MAX_WAKEUPS: usize = 64;

    /// Creates a drain budget.
    #[must_use]
    pub const fn new(max_wakeups: usize) -> Self {
        Self { max_wakeups }
    }

    /// Returns how many wakeups may be processed.
    pub const fn max_wakeups(self) -> usize {
        self.max_wakeups
    }
}

impl Default for DrainBudget {
    fn default() -> Self {
        Self::new(Self::DEFAULT_MAX_WAKEUPS)
    }
}

/// Request to drain due scheduler work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrainRequest {
    until: SimulationTime,
    budget: DrainBudget,
}

impl DrainRequest {
    /// Creates a drain request through a simulation-time boundary.
    #[must_use]
    pub const fn new(until: SimulationTime, budget: DrainBudget) -> Self {
        Self { until, budget }
    }

    /// Creates a drain request with the default budget.
    #[must_use]
    pub const fn until(until: SimulationTime) -> Self {
        Self {
            until,
            budget: DrainBudget::new(DrainBudget::DEFAULT_MAX_WAKEUPS),
        }
    }

    /// Returns the inclusive simulation-time boundary.
    pub const fn until_time(self) -> SimulationTime {
        self.until
    }

    /// Returns the drain budget.
    pub const fn budget(self) -> DrainBudget {
        self.budget
    }
}

/// Result of a scheduler drain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrainReport {
    outcome: DrainOutcome,
    processed: Vec<ProcessedWakeup>,
}

impl DrainReport {
    fn new(outcome: DrainOutcome, processed: Vec<ProcessedWakeup>) -> Self {
        Self { outcome, processed }
    }

    /// Returns why the drain stopped.
    pub const fn outcome(&self) -> &DrainOutcome {
        &self.outcome
    }

    /// Returns processed wakeups in scheduler order.
    pub fn processed(&self) -> &[ProcessedWakeup] {
        &self.processed
    }
}

/// Reason a scheduler drain stopped.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DrainOutcome {
    /// No due work remains at or before the requested boundary.
    Quiescent,
    /// A host-facing input opportunity is due.
    InputOpportunity { wakeup: ScheduledWakeupId },
    /// The requested budget was reached while due work remains.
    BudgetExceeded,
}

/// Wakeup processed by a scheduler drain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessedWakeup {
    wakeup: ScheduledWakeupId,
    result: WakeupDrainResult,
    invalidation: DerivedViewInvalidationReport,
}

impl ProcessedWakeup {
    fn new(
        wakeup: ScheduledWakeupId,
        result: WakeupDrainResult,
        invalidation: DerivedViewInvalidationReport,
    ) -> Self {
        Self {
            wakeup,
            result,
            invalidation,
        }
    }

    /// Returns the processed wakeup.
    pub const fn wakeup(&self) -> ScheduledWakeupId {
        self.wakeup
    }

    /// Returns the domain result for this wakeup.
    pub const fn result(&self) -> &WakeupDrainResult {
        &self.result
    }

    /// Returns model invalidation caused by processing this wakeup.
    pub const fn invalidation(&self) -> DerivedViewInvalidationReport {
        self.invalidation
    }
}

/// Domain result for one due wakeup.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WakeupDrainResult {
    /// The wakeup advanced a process and scheduled another wakeup.
    Rescheduled,
    /// The wakeup completed a process.
    Completed,
    /// The wakeup failed a process.
    Failed(ProcessFailureReason),
    /// The wakeup moved a process into a wait state.
    Waiting,
    /// The wakeup paused a process.
    Paused,
    /// The wakeup interrupted a process.
    Interrupted,
    /// The wakeup abandoned a process.
    Abandoned,
    /// The wakeup was skipped as stale.
    Skipped(StaleWakeupReason),
}

pub(crate) struct Scheduler;

impl Scheduler {
    pub(crate) fn schedule(
        ids: &mut RuntimeControlIds,
        request: ScheduleWakeupRequest,
    ) -> Result<PreparedWakeup, RuntimeError> {
        let wakeup = ids.issue_wakeup()?;
        let order = ids.issue_order(request.schedule)?;
        let record = ScheduledWakeupRecord::new(
            wakeup,
            order,
            request.target,
            ScheduledWakeupStatus::Scheduled,
            RuntimeControlSource::Scheduler,
            request.provenance,
        );
        let mut draft = RuntimeControlDraft::new(
            RuntimeControlSource::Scheduler,
            request.submitted_at,
            ReplayLevel::AuditOnly,
            request.provenance,
        );
        draft.schedule_wakeup(request.submitted_at, record);

        Ok(PreparedWakeup::new(wakeup, draft.accept_control_only()?))
    }

    pub(crate) fn cancel(
        wakeup: ScheduledWakeupId,
        canceled_at: SimulationTime,
        reason: WakeupCancellationReason,
        provenance: Option<ProvenanceKey>,
    ) -> Result<AcceptedRuntimeControlUpdate, RuntimeError> {
        let mut draft = RuntimeControlDraft::new(
            RuntimeControlSource::Scheduler,
            canceled_at,
            ReplayLevel::AuditOnly,
            provenance,
        );
        draft.cancel_wakeup(wakeup, canceled_at, reason);

        Ok(draft.accept_control_only()?)
    }

    pub(crate) fn acknowledge_host_input(
        wakeup: ScheduledWakeupId,
        acknowledged_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<AcceptedRuntimeControlUpdate, RuntimeError> {
        let mut draft = RuntimeControlDraft::new(
            RuntimeControlSource::Host,
            acknowledged_at,
            ReplayLevel::AuditOnly,
            provenance,
        );
        draft.consume_wakeup(wakeup, acknowledged_at, WakeupConsumptionReason::Completed);

        Ok(draft.accept_control_only()?)
    }

    pub(crate) fn drain(
        definitions: &DefinitionRegistry,
        transaction_ids: &mut CausalTransactionIdIssuer,
        ids: &mut RuntimeControlIds,
        model: &mut WorldModel,
        request: DrainRequest,
    ) -> Result<DrainReport, RuntimeError> {
        let mut processed = Vec::new();

        loop {
            let Some(wakeup) = model
                .runtime_control_store()
                .due_wakeups(request.until_time())
                .next()
                .cloned()
            else {
                return Ok(DrainReport::new(DrainOutcome::Quiescent, processed));
            };

            match wakeup.target() {
                WakeupTarget::HostInputOpportunity => {
                    return Ok(DrainReport::new(
                        DrainOutcome::InputOpportunity {
                            wakeup: wakeup.id(),
                        },
                        processed,
                    ));
                }
                WakeupTarget::Process(_) | WakeupTarget::PassiveProcess(_) => {
                    if processed.len() >= request.budget().max_wakeups() {
                        return Ok(DrainReport::new(DrainOutcome::BudgetExceeded, processed));
                    }
                    let (draft, transition) =
                        ProcessRuntime::advance_wakeup(definitions, ids, model, &wakeup)?
                            .into_parts();
                    let invalidation =
                        match ProcessCommitInput::from_transition(&transition, wakeup.id()) {
                            Some(input) => apply_process_commit(
                                transaction_ids,
                                model,
                                draft.into_transaction_coupled_changes(),
                                input,
                                wakeup.order().time(),
                                wakeup.provenance(),
                            )?,
                            None => model
                                .apply_runtime_control_update(draft.accept_control_only()?)?
                                .invalidation(),
                        };
                    processed.push(ProcessedWakeup::new(
                        wakeup.id(),
                        wakeup_result(&transition),
                        invalidation,
                    ));
                }
                _ => {
                    return Err(RuntimeError::UnsupportedWakeupTarget {
                        target: wakeup.target().clone(),
                    });
                }
            }
        }
    }
}

fn apply_process_commit(
    transaction_ids: &mut CausalTransactionIdIssuer,
    model: &mut WorldModel,
    control_changes: RuntimeControlTransactionChanges,
    input: ProcessCommitInput<'_>,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
) -> Result<DerivedViewInvalidationReport, RuntimeError> {
    let transaction_id = transaction_ids
        .issue()
        .ok_or(RuntimeError::TransactionIdExhausted)?;
    let mut transaction = CausalTransactionBuilder::new(
        CausalTransactionHeader {
            id: transaction_id,
            source: RequestSource::Engine,
            cause: TransactionCause::ProcessTick {
                process: input.process.id(),
                process_definition: input.process.definition(),
                resolution: input.process.resolution(),
                wakeup: input.wakeup,
            },
            occurred_at,
            replay_level: ReplayLevel::AuditOnly,
            provenance,
        },
        InvalidationPackage::new(InvalidationSource::HardCommit(transaction_id)),
    );
    for change in control_changes.into_changes() {
        transaction.push_control_change(change);
    }

    let commit = CommitFinalizer::finalize_eventless_process_tick(transaction)?;
    Ok(model.apply_hard_commit(commit)?.invalidation())
}

struct ProcessCommitInput<'a> {
    process: &'a ProcessInstanceRecord,
    wakeup: ScheduledWakeupId,
}

impl<'a> ProcessCommitInput<'a> {
    fn from_transition(
        transition: &'a ProcessTransition,
        wakeup: ScheduledWakeupId,
    ) -> Option<Self> {
        transition.process().map(|process| Self { process, wakeup })
    }
}

fn wakeup_result(transition: &ProcessTransition) -> WakeupDrainResult {
    match transition {
        ProcessTransition::Started { .. } => WakeupDrainResult::Rescheduled,
        ProcessTransition::Rescheduled { .. } => WakeupDrainResult::Rescheduled,
        ProcessTransition::Completed { .. } => WakeupDrainResult::Completed,
        ProcessTransition::Failed { reason, .. } => WakeupDrainResult::Failed(reason.clone()),
        ProcessTransition::Waiting { .. } => WakeupDrainResult::Waiting,
        ProcessTransition::Paused { .. } => WakeupDrainResult::Paused,
        ProcessTransition::Interrupted { .. } => WakeupDrainResult::Interrupted,
        ProcessTransition::Resumed { .. } => WakeupDrainResult::Rescheduled,
        ProcessTransition::Abandoned { .. } => WakeupDrainResult::Abandoned,
        ProcessTransition::Skipped { reason, .. } => WakeupDrainResult::Skipped(reason.clone()),
    }
}
