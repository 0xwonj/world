use world_core::{
    AuthorityClass, CausalTransactionIdIssuer, EventRecordIdIssuer, ProcessInstanceId,
    ProvenanceKey, ScheduledWakeupId, SimulationTime,
};
use world_defs::DefinitionRegistry;
use world_model::{
    InterruptReason, InvalidationPackage, InvalidationSource, PauseReason,
    RuntimeControlApplication, TransactionCause, WaitCondition, WakeupCancellationReason,
    WorldModel,
};

use crate::{
    RuntimeError,
    control::RuntimeControlIds,
    outcome::{CommittedOutcome, RejectedOutcome, RejectionReason, RuntimeOutcome},
    primitive::{PrimitiveSemanticsRegistry, PrimitiveValidationFailure},
    process::{ProcessControlOutcome, ProcessRuntime, ProcessRuntimeUpdate, StartProcessRequest},
    request::RuntimeRequest,
    scheduler::{
        DrainReport, DrainRequest, ScheduleWakeupRequest, ScheduledWakeupOutcome, Scheduler,
    },
    transaction::{
        CausalTransactionBuilder, CausalTransactionHeader, CommitFinalizer, EffectInterpretation,
        EffectStager, RuntimeValidator, TypedEffectInterpreter,
    },
};

/// Public causal mutation waist.
pub struct CausalRuntime {
    definitions: DefinitionRegistry,
    semantics: PrimitiveSemanticsRegistry,
    transaction_ids: CausalTransactionIdIssuer,
    event_ids: EventRecordIdIssuer,
    control_ids: RuntimeControlIds,
    interpreter: TypedEffectInterpreter,
}

impl CausalRuntime {
    fn new(
        definitions: DefinitionRegistry,
        semantics: PrimitiveSemanticsRegistry,
        transaction_ids: CausalTransactionIdIssuer,
        event_ids: EventRecordIdIssuer,
        control_ids: RuntimeControlIds,
    ) -> Result<Self, RuntimeError> {
        semantics.validate_against(&definitions)?;
        Ok(Self {
            definitions,
            semantics,
            transaction_ids,
            event_ids,
            control_ids,
            interpreter: TypedEffectInterpreter,
        })
    }

    /// Creates a causal runtime for a new empty model with fresh id issuers.
    pub fn for_empty_model(
        definitions: DefinitionRegistry,
        semantics: PrimitiveSemanticsRegistry,
    ) -> Result<Self, RuntimeError> {
        Self::new(
            definitions,
            semantics,
            CausalTransactionIdIssuer::new(),
            EventRecordIdIssuer::new(),
            RuntimeControlIds::new(),
        )
    }

    /// Creates a causal runtime for a new empty model with explicit hard-state
    /// id issuers.
    pub fn with_hard_issuers_for_empty_model(
        definitions: DefinitionRegistry,
        semantics: PrimitiveSemanticsRegistry,
        transaction_ids: CausalTransactionIdIssuer,
        event_ids: EventRecordIdIssuer,
    ) -> Result<Self, RuntimeError> {
        Self::new(
            definitions,
            semantics,
            transaction_ids,
            event_ids,
            RuntimeControlIds::new(),
        )
    }

    /// Creates a causal runtime whose runtime-control issuers continue from
    /// existing model state.
    pub fn for_model(
        definitions: DefinitionRegistry,
        semantics: PrimitiveSemanticsRegistry,
        model: &WorldModel,
    ) -> Result<Self, RuntimeError> {
        Self::new(
            definitions,
            semantics,
            CausalTransactionIdIssuer::new(),
            EventRecordIdIssuer::new(),
            RuntimeControlIds::from_store(model.runtime_control_store())?,
        )
    }

    /// Creates a causal runtime with explicit hard-state issuers and hydrated
    /// runtime-control issuers.
    pub fn with_hard_issuers_for_model(
        definitions: DefinitionRegistry,
        semantics: PrimitiveSemanticsRegistry,
        transaction_ids: CausalTransactionIdIssuer,
        event_ids: EventRecordIdIssuer,
        model: &WorldModel,
    ) -> Result<Self, RuntimeError> {
        Self::new(
            definitions,
            semantics,
            transaction_ids,
            event_ids,
            RuntimeControlIds::from_store(model.runtime_control_store())?,
        )
    }

    /// Executes one runtime request against the model through the hard commit waist.
    pub fn execute(
        &mut self,
        model: &mut WorldModel,
        request: RuntimeRequest,
    ) -> Result<RuntimeOutcome, RuntimeError> {
        let action_id = request.action();
        let Some(action) = self.definitions.action(action_id) else {
            return Ok(RuntimeOutcome::Rejected(RejectedOutcome::new(
                action_id,
                RejectionReason::UnknownAction { action: action_id },
            )));
        };

        let bound = match request.bind(action) {
            Ok(bound) => bound,
            Err(rejected) => return Ok(RuntimeOutcome::Rejected(rejected)),
        };

        let Some(program) = self.definitions.effect_program(bound.effect_program()) else {
            return Err(RuntimeError::MissingEffectProgram {
                action: action.id(),
                effect_program: bound.effect_program(),
            });
        };

        if let Err(failure) = RuntimeValidator::validate(
            model,
            action,
            &bound,
            program,
            &self.definitions,
            &self.semantics,
        ) {
            return match failure {
                PrimitiveValidationFailure::Rejected(rejected) => {
                    Ok(RuntimeOutcome::Rejected(rejected))
                }
                PrimitiveValidationFailure::Runtime(error) => Err(error),
            };
        }

        let Some(transaction_id) = self.transaction_ids.issue() else {
            return Err(RuntimeError::TransactionIdExhausted);
        };

        let mut invalidation =
            InvalidationPackage::new(InvalidationSource::HardCommit(transaction_id));
        invalidation.mark_authority_class(AuthorityClass::Hard);

        let mut transaction = CausalTransactionBuilder::new(
            CausalTransactionHeader {
                id: transaction_id,
                source: bound.source(),
                cause: TransactionCause::Action {
                    action: bound.action(),
                    effect_program: bound.effect_program(),
                },
                occurred_at: bound.submitted_at(),
                replay_level: program.replay_level(),
                provenance: bound.provenance(),
            },
            invalidation,
        );

        {
            let mut stager = EffectStager::new(model, &mut transaction);
            self.interpreter.interpret(
                program,
                EffectInterpretation {
                    definitions: &self.definitions,
                    semantics: &self.semantics,
                    request: &bound,
                    stager: &mut stager,
                    event_ids: &mut self.event_ids,
                    control_ids: &mut self.control_ids,
                },
            )?;
        }

        let accepted = CommitFinalizer::finalize_action(transaction, program)?;
        let event_ids = accepted
            .events()
            .iter()
            .map(|event| event.id())
            .collect::<Vec<_>>();
        let application = model.apply_hard_commit(accepted)?;

        Ok(RuntimeOutcome::Committed(CommittedOutcome::new(
            transaction_id,
            event_ids,
            application.invalidation(),
        )))
    }

    /// Starts a durable process and schedules its first wakeup through the
    /// runtime-control gate.
    pub fn start_process(
        &mut self,
        model: &mut WorldModel,
        request: StartProcessRequest,
    ) -> Result<ProcessControlOutcome, RuntimeError> {
        apply_process_control_update(
            model,
            ProcessRuntime::start(&self.definitions, &mut self.control_ids, request)?,
        )
    }

    /// Drains due scheduler wakeups through the runtime-control gate.
    pub fn drain_scheduler(
        &mut self,
        model: &mut WorldModel,
        request: DrainRequest,
    ) -> Result<DrainReport, RuntimeError> {
        Scheduler::drain(
            &self.definitions,
            &mut self.transaction_ids,
            &mut self.control_ids,
            model,
            request,
        )
    }

    /// Schedules a wakeup through the runtime-control gate.
    pub fn schedule_wakeup(
        &mut self,
        model: &mut WorldModel,
        request: ScheduleWakeupRequest,
    ) -> Result<ScheduledWakeupOutcome, RuntimeError> {
        let (wakeup, update) = Scheduler::schedule(&mut self.control_ids, request)?.into_parts();
        let application = model.apply_runtime_control_update(update)?;

        Ok(ScheduledWakeupOutcome::new(wakeup, application))
    }

    /// Cancels a scheduled wakeup through the runtime-control gate.
    pub fn cancel_wakeup(
        &mut self,
        model: &mut WorldModel,
        wakeup: ScheduledWakeupId,
        canceled_at: SimulationTime,
        reason: WakeupCancellationReason,
        provenance: Option<ProvenanceKey>,
    ) -> Result<RuntimeControlApplication, RuntimeError> {
        let update = Scheduler::cancel(wakeup, canceled_at, reason, provenance)?;
        Ok(model.apply_runtime_control_update(update)?)
    }

    /// Acknowledges a due host input wakeup through the runtime-control gate.
    pub fn acknowledge_host_input_wakeup(
        &mut self,
        model: &mut WorldModel,
        wakeup: ScheduledWakeupId,
        acknowledged_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<RuntimeControlApplication, RuntimeError> {
        let update = Scheduler::acknowledge_host_input(wakeup, acknowledged_at, provenance)?;
        Ok(model.apply_runtime_control_update(update)?)
    }

    /// Moves a process into a wait state through the runtime-control gate.
    pub fn wait_process(
        &mut self,
        model: &mut WorldModel,
        process: ProcessInstanceId,
        condition: WaitCondition,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<ProcessControlOutcome, RuntimeError> {
        apply_process_control_update(
            model,
            ProcessRuntime::wait(model, process, condition, occurred_at, provenance)?,
        )
    }

    /// Pauses a process through the runtime-control gate.
    pub fn pause_process(
        &mut self,
        model: &mut WorldModel,
        process: ProcessInstanceId,
        reason: PauseReason,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<ProcessControlOutcome, RuntimeError> {
        apply_process_control_update(
            model,
            ProcessRuntime::pause(model, process, reason, occurred_at, provenance)?,
        )
    }

    /// Interrupts a process through the runtime-control gate.
    pub fn interrupt_process(
        &mut self,
        model: &mut WorldModel,
        process: ProcessInstanceId,
        reason: InterruptReason,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<ProcessControlOutcome, RuntimeError> {
        apply_process_control_update(
            model,
            ProcessRuntime::interrupt(model, process, reason, occurred_at, provenance)?,
        )
    }

    /// Abandons a process through the runtime-control gate.
    pub fn abandon_process(
        &mut self,
        model: &mut WorldModel,
        process: ProcessInstanceId,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<ProcessControlOutcome, RuntimeError> {
        apply_process_control_update(
            model,
            ProcessRuntime::abandon(model, process, occurred_at, provenance)?,
        )
    }

    /// Resumes a process by scheduling a future wakeup through runtime control.
    pub fn resume_process(
        &mut self,
        model: &mut WorldModel,
        process: ProcessInstanceId,
        schedule: crate::WakeupScheduleKey,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<ProcessControlOutcome, RuntimeError> {
        apply_process_control_update(
            model,
            ProcessRuntime::resume(
                &mut self.control_ids,
                model,
                process,
                schedule,
                occurred_at,
                provenance,
            )?,
        )
    }
}

fn apply_process_control_update(
    model: &mut WorldModel,
    update: ProcessRuntimeUpdate,
) -> Result<ProcessControlOutcome, RuntimeError> {
    let (draft, transition) = update.into_parts();
    let application = model.apply_runtime_control_update(draft.accept_control_only()?)?;

    Ok(ProcessControlOutcome::new(transition, application))
}
