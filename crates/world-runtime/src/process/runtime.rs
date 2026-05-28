use std::collections::BTreeSet;

use world_core::{
    DefinitionId, EntityId, ProcessInstanceId, ProvenanceKey, ReplayLevel, ScheduledWakeupId,
    SimulationDuration, SimulationTime, VersionAnchor,
};
use world_defs::{DefinitionRegistry, ResolutionTier, RoleName};
use world_model::{
    InterruptReason, PauseReason, ProcessFailureReason, ProcessInstanceInit, ProcessInstanceRecord,
    ProcessLifecycle, ProcessProgress, ProcessRoleBinding, ProcessWork, RuntimeControlSource,
    ScheduledWakeupRecord, ScheduledWakeupStatus, StaleWakeupReason, WaitCondition,
    WakeupConsumptionReason, WakeupTarget, WorldModel,
};

use crate::{
    RuntimeError, WakeupScheduleKey,
    control::{RuntimeControlDraft, RuntimeControlIds},
};

use super::{ProcessTick, ProcessTransition, StartProcessRequest};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProcessRuntimeUpdate {
    draft: RuntimeControlDraft,
    transition: ProcessTransition,
}

impl ProcessRuntimeUpdate {
    pub(crate) fn into_parts(self) -> (RuntimeControlDraft, ProcessTransition) {
        (self.draft, self.transition)
    }
}

struct NewProcessInstance {
    id: ProcessInstanceId,
    definition: DefinitionId,
    owner: Option<EntityId>,
    roles: Vec<ProcessRoleBinding>,
    resolution: ResolutionTier,
    required_work: ProcessWork,
    first_wakeup: ScheduledWakeupId,
    version: VersionAnchor,
    provenance: Option<ProvenanceKey>,
}

impl NewProcessInstance {
    fn into_record(self) -> ProcessInstanceRecord {
        let init = ProcessInstanceInit::new(
            self.id,
            self.definition,
            self.resolution,
            ProcessLifecycle::Scheduled {
                wakeup: self.first_wakeup,
            },
            ProcessProgress::Bounded {
                completed: ProcessWork::from_units(0),
                required: self.required_work,
            },
            self.version,
        )
        .with_roles(self.roles);
        let init = match self.owner {
            Some(owner) => init.with_owner(owner),
            None => init,
        };
        let init = match self.provenance {
            Some(provenance) => init.with_provenance(provenance),
            None => init,
        };
        ProcessInstanceRecord::new(init)
    }
}

pub(crate) struct ProcessRuntime;

impl ProcessRuntime {
    pub(crate) fn start(
        definitions: &DefinitionRegistry,
        ids: &mut RuntimeControlIds,
        request: StartProcessRequest,
    ) -> Result<ProcessRuntimeUpdate, RuntimeError> {
        let definition = definitions.process(request.definition).ok_or(
            RuntimeError::MissingProcessDefinition {
                definition: request.definition,
            },
        )?;
        if !definition.supports_resolution(request.resolution) {
            return Err(RuntimeError::UnsupportedProcessResolution {
                definition: request.definition,
                resolution: request.resolution,
            });
        }
        validate_roles(
            definition.roles().iter().map(|role| role.name()),
            &request.roles,
        )?;

        let process_id = ids.issue_process()?;
        let wakeup_id = ids.issue_wakeup()?;
        let wakeup_order = ids.issue_order(request.first_wakeup)?;
        let submitted_at = request.submitted_at;
        let provenance = request.provenance;
        let process = NewProcessInstance {
            id: process_id,
            definition: request.definition,
            owner: request.owner,
            roles: request.roles,
            resolution: request.resolution,
            required_work: request.required_work,
            first_wakeup: wakeup_id,
            version: definition.version(),
            provenance,
        }
        .into_record();
        let wakeup = ScheduledWakeupRecord::new(
            wakeup_id,
            wakeup_order,
            WakeupTarget::Process(process_id),
            ScheduledWakeupStatus::Scheduled,
            RuntimeControlSource::ProcessRuntime,
            provenance,
        );

        let mut draft = RuntimeControlDraft::new(
            RuntimeControlSource::ProcessRuntime,
            submitted_at,
            ReplayLevel::AuditOnly,
            provenance,
        );
        draft.create_process(submitted_at, process.clone());
        draft.schedule_wakeup(submitted_at, wakeup.clone());

        Ok(ProcessRuntimeUpdate {
            draft,
            transition: ProcessTransition::Started { process, wakeup },
        })
    }

    pub(crate) fn advance_wakeup(
        definitions: &DefinitionRegistry,
        ids: &mut RuntimeControlIds,
        model: &WorldModel,
        wakeup: &ScheduledWakeupRecord,
    ) -> Result<ProcessRuntimeUpdate, RuntimeError> {
        let (WakeupTarget::Process(process_id) | WakeupTarget::PassiveProcess(process_id)) =
            wakeup.target()
        else {
            return Err(RuntimeError::UnsupportedWakeupTarget {
                target: wakeup.target().clone(),
            });
        };
        let tick = ProcessTick::new(
            *process_id,
            wakeup.order().time(),
            wakeup.id(),
            wakeup.provenance(),
        );

        let Some(process) = model.runtime_control_store().process(tick.process()) else {
            return Ok(skip_wakeup_update(
                tick,
                StaleWakeupReason::MissingProcess,
                wakeup.provenance(),
            ));
        };

        match process.lifecycle() {
            ProcessLifecycle::Scheduled { wakeup: current } if *current == tick.source_wakeup() => {
            }
            lifecycle if lifecycle.is_terminal() => {
                return Ok(skip_wakeup_update(
                    tick,
                    StaleWakeupReason::TerminalProcess,
                    wakeup.provenance(),
                ));
            }
            _ => {
                return Ok(skip_wakeup_update(
                    tick,
                    StaleWakeupReason::Superseded,
                    wakeup.provenance(),
                ));
            }
        }

        let Some(definition) = definitions.process(process.definition()) else {
            return Ok(fail_process_update(
                process.clone(),
                tick,
                ProcessFailureReason::MissingDefinition,
            ));
        };
        if !definition.supports_resolution(process.resolution()) {
            return Ok(fail_process_update(
                process.clone(),
                tick,
                ProcessFailureReason::UnsupportedResolution,
            ));
        }

        let process = advance_process_progress(process)?;
        if process.progress().is_complete() {
            Ok(complete_process_update(process, tick))
        } else {
            reschedule_process_update(ids, process, wakeup, tick)
        }
    }

    pub(crate) fn wait(
        model: &WorldModel,
        process: ProcessInstanceId,
        condition: WaitCondition,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<ProcessRuntimeUpdate, RuntimeError> {
        transition_existing_process(
            model,
            process,
            occurred_at,
            provenance,
            ProcessLifecycle::Waiting {
                condition: condition.clone(),
            },
            ProcessTransitionKind::Waiting(condition),
        )
    }

    pub(crate) fn pause(
        model: &WorldModel,
        process: ProcessInstanceId,
        reason: PauseReason,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<ProcessRuntimeUpdate, RuntimeError> {
        transition_existing_process(
            model,
            process,
            occurred_at,
            provenance,
            ProcessLifecycle::Paused {
                reason: reason.clone(),
            },
            ProcessTransitionKind::Paused(reason),
        )
    }

    pub(crate) fn interrupt(
        model: &WorldModel,
        process: ProcessInstanceId,
        reason: InterruptReason,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<ProcessRuntimeUpdate, RuntimeError> {
        transition_existing_process(
            model,
            process,
            occurred_at,
            provenance,
            ProcessLifecycle::Interrupted {
                reason: reason.clone(),
            },
            ProcessTransitionKind::Interrupted(reason),
        )
    }

    pub(crate) fn abandon(
        model: &WorldModel,
        process: ProcessInstanceId,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<ProcessRuntimeUpdate, RuntimeError> {
        transition_existing_process(
            model,
            process,
            occurred_at,
            provenance,
            ProcessLifecycle::Abandoned,
            ProcessTransitionKind::Abandoned,
        )
    }

    pub(crate) fn resume(
        ids: &mut RuntimeControlIds,
        model: &WorldModel,
        process: ProcessInstanceId,
        schedule: WakeupScheduleKey,
        occurred_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Result<ProcessRuntimeUpdate, RuntimeError> {
        let existing = existing_process(model, process)?;
        ensure_transitionable(&existing)?;
        let wakeup_id = ids.issue_wakeup()?;
        let order = ids.issue_order(schedule)?;
        let updated = existing
            .clone()
            .with_lifecycle(ProcessLifecycle::Scheduled { wakeup: wakeup_id });
        let wakeup = ScheduledWakeupRecord::new(
            wakeup_id,
            order,
            WakeupTarget::Process(process),
            ScheduledWakeupStatus::Scheduled,
            RuntimeControlSource::ProcessRuntime,
            provenance,
        );
        let mut draft = RuntimeControlDraft::new(
            RuntimeControlSource::ProcessRuntime,
            occurred_at,
            ReplayLevel::AuditOnly,
            provenance,
        );
        draft.cancel_current_wakeup(existing.lifecycle(), occurred_at);
        draft.update_process(occurred_at, updated.clone());
        draft.schedule_wakeup(occurred_at, wakeup.clone());

        Ok(ProcessRuntimeUpdate {
            draft,
            transition: ProcessTransition::Resumed {
                process: updated,
                wakeup,
            },
        })
    }
}

enum ProcessTransitionKind {
    Waiting(WaitCondition),
    Paused(PauseReason),
    Interrupted(InterruptReason),
    Abandoned,
}

fn validate_roles<'a>(
    declared: impl Iterator<Item = &'a RoleName>,
    bindings: &[ProcessRoleBinding],
) -> Result<(), RuntimeError> {
    let declared = declared.cloned().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for binding in bindings {
        if !declared.contains(binding.role()) {
            return Err(RuntimeError::UnknownProcessRoleBinding {
                role: binding.role().clone(),
            });
        }
        if !seen.insert(binding.role().clone()) {
            return Err(RuntimeError::DuplicateProcessRoleBinding {
                role: binding.role().clone(),
            });
        }
    }

    for role in declared {
        if !seen.contains(&role) {
            return Err(RuntimeError::MissingBoundRole { role });
        }
    }

    Ok(())
}

fn existing_process(
    model: &WorldModel,
    process: ProcessInstanceId,
) -> Result<ProcessInstanceRecord, RuntimeError> {
    model
        .runtime_control_store()
        .process(process)
        .cloned()
        .ok_or(RuntimeError::MissingProcess { process })
}

fn ensure_transitionable(process: &ProcessInstanceRecord) -> Result<(), RuntimeError> {
    if process.lifecycle().is_terminal() {
        return Err(RuntimeError::InvalidProcessLifecycleTransition {
            process: process.id(),
            lifecycle: process.lifecycle().clone(),
        });
    }

    Ok(())
}

fn advance_process_progress(
    process: &ProcessInstanceRecord,
) -> Result<ProcessInstanceRecord, RuntimeError> {
    let progress = process
        .progress()
        .clone()
        .advance(ProcessWork::from_units(1))?;
    Ok(process.clone().with_progress(progress))
}

fn transition_existing_process(
    model: &WorldModel,
    process: ProcessInstanceId,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
    lifecycle: ProcessLifecycle,
    transition: ProcessTransitionKind,
) -> Result<ProcessRuntimeUpdate, RuntimeError> {
    let existing = existing_process(model, process)?;
    ensure_transitionable(&existing)?;
    let updated = existing.clone().with_lifecycle(lifecycle);
    let mut draft = RuntimeControlDraft::new(
        RuntimeControlSource::ProcessRuntime,
        occurred_at,
        ReplayLevel::AuditOnly,
        provenance,
    );
    draft.cancel_current_wakeup(existing.lifecycle(), occurred_at);
    draft.update_process(occurred_at, updated.clone());

    Ok(ProcessRuntimeUpdate {
        draft,
        transition: transition.into_transition(updated),
    })
}

impl ProcessTransitionKind {
    fn into_transition(self, process: ProcessInstanceRecord) -> ProcessTransition {
        match self {
            Self::Waiting(condition) => ProcessTransition::Waiting { process, condition },
            Self::Paused(reason) => ProcessTransition::Paused { process, reason },
            Self::Interrupted(reason) => ProcessTransition::Interrupted { process, reason },
            Self::Abandoned => ProcessTransition::Abandoned { process },
        }
    }
}

fn consume_wakeup(draft: &mut RuntimeControlDraft, tick: ProcessTick) {
    draft.consume_wakeup(
        tick.source_wakeup(),
        tick.occurred_at,
        WakeupConsumptionReason::Dispatched,
    );
}

fn skip_wakeup_update(
    tick: ProcessTick,
    reason: StaleWakeupReason,
    provenance: Option<ProvenanceKey>,
) -> ProcessRuntimeUpdate {
    let mut draft = RuntimeControlDraft::new(
        RuntimeControlSource::Scheduler,
        tick.occurred_at,
        ReplayLevel::AuditOnly,
        provenance,
    );
    draft.skip_wakeup(tick.source_wakeup(), tick.occurred_at, reason.clone());

    ProcessRuntimeUpdate {
        draft,
        transition: ProcessTransition::Skipped {
            wakeup: tick.source_wakeup(),
            reason,
        },
    }
}

fn fail_process_update(
    process: ProcessInstanceRecord,
    tick: ProcessTick,
    reason: ProcessFailureReason,
) -> ProcessRuntimeUpdate {
    let process = process.with_lifecycle(ProcessLifecycle::Failed {
        reason: reason.clone(),
    });
    let mut draft = RuntimeControlDraft::new(
        RuntimeControlSource::ProcessRuntime,
        tick.occurred_at,
        ReplayLevel::AuditOnly,
        tick.provenance,
    );
    consume_wakeup(&mut draft, tick);
    draft.update_process(tick.occurred_at, process.clone());

    ProcessRuntimeUpdate {
        draft,
        transition: ProcessTransition::Failed { process, reason },
    }
}

fn complete_process_update(
    process: ProcessInstanceRecord,
    tick: ProcessTick,
) -> ProcessRuntimeUpdate {
    let process = process.with_lifecycle(ProcessLifecycle::Completed);
    let mut draft = RuntimeControlDraft::new(
        RuntimeControlSource::ProcessRuntime,
        tick.occurred_at,
        ReplayLevel::AuditOnly,
        tick.provenance,
    );
    consume_wakeup(&mut draft, tick);
    draft.update_process(tick.occurred_at, process.clone());

    ProcessRuntimeUpdate {
        draft,
        transition: ProcessTransition::Completed { process },
    }
}

fn reschedule_process_update(
    ids: &mut RuntimeControlIds,
    process: ProcessInstanceRecord,
    wakeup: &ScheduledWakeupRecord,
    tick: ProcessTick,
) -> Result<ProcessRuntimeUpdate, RuntimeError> {
    let next_wakeup_id = ids.issue_wakeup()?;
    let next_schedule = next_schedule_after(wakeup.order())?;
    let next_order = ids.issue_order(next_schedule)?;
    let process = process.with_lifecycle(ProcessLifecycle::Scheduled {
        wakeup: next_wakeup_id,
    });
    let next_wakeup = ScheduledWakeupRecord::new(
        next_wakeup_id,
        next_order,
        next_wakeup_target(wakeup.target(), process.id()),
        ScheduledWakeupStatus::Scheduled,
        RuntimeControlSource::ProcessRuntime,
        tick.provenance,
    );

    let mut draft = RuntimeControlDraft::new(
        RuntimeControlSource::ProcessRuntime,
        tick.occurred_at,
        ReplayLevel::AuditOnly,
        tick.provenance,
    );
    consume_wakeup(&mut draft, tick);
    draft.update_process(tick.occurred_at, process.clone());
    draft.schedule_wakeup(tick.occurred_at, next_wakeup.clone());

    Ok(ProcessRuntimeUpdate {
        draft,
        transition: ProcessTransition::Rescheduled {
            process,
            wakeup: next_wakeup,
        },
    })
}

fn next_wakeup_target(previous: &WakeupTarget, process: ProcessInstanceId) -> WakeupTarget {
    match previous {
        WakeupTarget::PassiveProcess(_) => WakeupTarget::PassiveProcess(process),
        WakeupTarget::Process(_) | WakeupTarget::HostInputOpportunity => {
            WakeupTarget::Process(process)
        }
        _ => WakeupTarget::Process(process),
    }
}

fn next_schedule_after(
    order: world_core::WakeupOrderKey,
) -> Result<WakeupScheduleKey, RuntimeError> {
    let time = order
        .time()
        .checked_add(SimulationDuration::from_ticks(1))
        .ok_or(world_model::ModelError::RuntimeControlValueOverflow)?;

    Ok(WakeupScheduleKey::new(
        time,
        order.phase(),
        order.priority(),
    ))
}
