use std::collections::BTreeMap;

use world_core::{ActorId, EntityId};
use world_model::{
    ActionOpportunityId, ActionOpportunityState, Activity, ActivityStatus, ActivityTransition,
    CommandEnvelope, CommandValue, ContainmentAppraisal, EvidenceProvenance, Intent, IntentStatus,
    IntentTransition, PhysicalEvent, RelocationInteraction, RelocationProcessStatus,
    StableCommandRejection,
};

use crate::action_evaluation::{
    ActionEvaluationCaptureLedgerError, ActionEvaluationFallbackCause,
    ActionEvaluationInvocationLedgerError, ActionEvaluationInvocationState, ActionEvaluationWork,
};
use crate::control::{
    ActionOpportunityLedgerError, CommandLedgerInsertError, InputLedgerInsertError,
    LedgerRetirementError, ManagementLedgerInsertError, RuntimeControlState,
};
use crate::relocation::{RelocationProcessLedgerError, RelocationWakeClassification};
use crate::scheduler::{
    ScheduledWork, SchedulerBatchPlanError, SchedulerInsertion, SchedulerInstallError,
    SchedulerKey, SchedulerProducerOrdinal, SchedulerState,
};
use crate::session::{SessionClockProjectionError, SessionHead, SessionResumeProjectionError};

use super::{
    ActionEvaluationInvocationOpeningRecord, ActionEvaluationInvocationTransitionCause,
    ActionOpportunityTransitionRecord, AttemptRecord, AttemptSubjectRecord,
    AuthorityAdmissionRecord, AuthorityCursor, AuthorityRecord, AuthorityRecordBody,
    ContainmentAppraisalTransitionRecord, ContainmentTransitionError, DeliveryResolutionRecord,
    EvidenceRoutingRecord, EvidenceRoutingSource, ManagementCauseRecord, MomentBatchRecord,
    RelocationAttemptRecord, RelocationAttemptResolution, RelocationPositionTransitionError,
    RelocationProcessTransitionCause, RelocationProcessTransitionRecord, SealedAuthorityRecord,
    apply_containment_transfers, apply_relocation_arrival, apply_relocation_departure,
};

/// Why a sealed authority transition cannot be applied to an exact head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AuthorityRecordApplyError {
    StaleHead {
        expected: Box<AuthorityCursor>,
        actual: Box<AuthorityCursor>,
    },
    InputLedgerConflict(InputLedgerInsertError),
    ManagementLedgerConflict(ManagementLedgerInsertError),
    CommandLedgerConflict(CommandLedgerInsertError),
    ActionOpportunityLedger(ActionOpportunityLedgerError),
    ActionOpportunityTransitionMismatch {
        opportunity: ActionOpportunityId,
    },
    ActionEvaluationLedger(ActionEvaluationInvocationLedgerError),
    ActionEvaluationCaptureLedger(ActionEvaluationCaptureLedgerError),
    ActionEvaluationTransitionMismatch,
    LedgerRetirement(LedgerRetirementError),
    ScheduledWorkMissing {
        key: SchedulerKey,
    },
    ScheduledWorkMismatch {
        key: SchedulerKey,
    },
    SchedulerInstall(SchedulerInstallError),
    SchedulerPlan(SchedulerBatchPlanError),
    SchedulerPlanMismatch,
    ContainmentTransition(ContainmentTransitionError),
    RelocationProcess(RelocationProcessLedgerError),
    RelocationPosition(RelocationPositionTransitionError),
    RelocationTransitionMismatch,
    LifecycleTransitionMismatch,
    ClockProjection(SessionClockProjectionError),
    ResumeProjection(SessionResumeProjectionError),
    ManagementFrontierMismatch,
    ManagementTransitionMismatch,
}

impl From<InputLedgerInsertError> for AuthorityRecordApplyError {
    fn from(error: InputLedgerInsertError) -> Self {
        Self::InputLedgerConflict(error)
    }
}

impl From<ManagementLedgerInsertError> for AuthorityRecordApplyError {
    fn from(error: ManagementLedgerInsertError) -> Self {
        Self::ManagementLedgerConflict(error)
    }
}

impl From<CommandLedgerInsertError> for AuthorityRecordApplyError {
    fn from(error: CommandLedgerInsertError) -> Self {
        Self::CommandLedgerConflict(error)
    }
}

impl From<ActionOpportunityLedgerError> for AuthorityRecordApplyError {
    fn from(error: ActionOpportunityLedgerError) -> Self {
        Self::ActionOpportunityLedger(error)
    }
}

impl From<ActionEvaluationInvocationLedgerError> for AuthorityRecordApplyError {
    fn from(error: ActionEvaluationInvocationLedgerError) -> Self {
        Self::ActionEvaluationLedger(error)
    }
}

impl From<ActionEvaluationCaptureLedgerError> for AuthorityRecordApplyError {
    fn from(error: ActionEvaluationCaptureLedgerError) -> Self {
        Self::ActionEvaluationCaptureLedger(error)
    }
}

impl From<LedgerRetirementError> for AuthorityRecordApplyError {
    fn from(error: LedgerRetirementError) -> Self {
        Self::LedgerRetirement(error)
    }
}

impl From<SchedulerInstallError> for AuthorityRecordApplyError {
    fn from(error: SchedulerInstallError) -> Self {
        Self::SchedulerInstall(error)
    }
}

impl From<SchedulerBatchPlanError> for AuthorityRecordApplyError {
    fn from(error: SchedulerBatchPlanError) -> Self {
        Self::SchedulerPlan(error)
    }
}

impl From<ContainmentTransitionError> for AuthorityRecordApplyError {
    fn from(error: ContainmentTransitionError) -> Self {
        Self::ContainmentTransition(error)
    }
}

impl From<SessionClockProjectionError> for AuthorityRecordApplyError {
    fn from(error: SessionClockProjectionError) -> Self {
        Self::ClockProjection(error)
    }
}

impl From<SessionResumeProjectionError> for AuthorityRecordApplyError {
    fn from(error: SessionResumeProjectionError) -> Self {
        Self::ResumeProjection(error)
    }
}

/// One successfully interpreted record and the successor it produced.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AppliedAuthorityRecord {
    resulting_head: SessionHead,
    record: AuthorityRecord,
}

impl AppliedAuthorityRecord {
    pub(crate) const fn resulting_head(&self) -> &SessionHead {
        &self.resulting_head
    }

    pub(crate) const fn record(&self) -> &AuthorityRecord {
        &self.record
    }

    pub(crate) fn into_parts(self) -> (SessionHead, AuthorityRecord) {
        (self.resulting_head, self.record)
    }
}

/// Interprets one sealed delta against its exact authoritative predecessor.
///
/// Sealing owns semantic validation. Apply checks only that the sealed delta
/// is applicable to the supplied aggregate and constructs a new aggregate
/// without mutating the predecessor.
pub(crate) fn apply_authority_record(
    expected: &SessionHead,
    sealed: SealedAuthorityRecord,
) -> Result<AppliedAuthorityRecord, AuthorityRecordApplyError> {
    if expected.cursor() != sealed.expected_cursor() {
        return Err(AuthorityRecordApplyError::StaleHead {
            expected: Box::new(sealed.expected_cursor()),
            actual: Box::new(expected.cursor()),
        });
    }

    let resulting_cursor = sealed.resulting_cursor();
    let record = sealed.into_record();
    let mut mode = expected.mode();
    let mut clock = expected.clock();
    let mut accepted = expected.accepted().clone();
    let mut runtime_control = expected.runtime_control().clone();
    let mut scheduler = expected.scheduler().clone();
    let mut safety_blocker = expected.safety_blocker();

    match record.body() {
        AuthorityRecordBody::Admission(AuthorityAdmissionRecord::Commands(batch)) => {
            let mut insertions = Vec::with_capacity(batch.entries().len());
            let mut expected_insertions = Vec::with_capacity(batch.entries().len());
            for (position, entry) in batch.entries().iter().enumerate() {
                let captured = entry.captured();
                runtime_control.input_mut().insert_exact(
                    captured.input(),
                    captured.request_fingerprint(),
                    captured.id(),
                    entry.trigger(),
                    entry.outcome(),
                )?;
                let work = ScheduledWork::command(entry.scheduled_command().clone());
                insertions.push(SchedulerInsertion::new(
                    producer_ordinal(position),
                    work.clone(),
                ));
                expected_insertions.push((entry.scheduler_key(), work));
            }
            install_scheduler_delta(&mut scheduler, insertions, &expected_insertions)?;
        }
        AuthorityRecordBody::Admission(AuthorityAdmissionRecord::ActionEvaluation(admission)) => {
            let request = admission.request();
            let transition = admission.transition();
            if transition.cause()
                != ActionEvaluationInvocationTransitionCause::ResultCapture(request.capture())
                || transition.after().invocation() != request.invocation()
                || admission.outcome() != request.outcome(record.header().id())
            {
                return Err(AuthorityRecordApplyError::ActionEvaluationTransitionMismatch);
            }
            runtime_control
                .action_evaluation_captures_mut()
                .insert_exact(request, admission.outcome())?;
            let installed = runtime_control
                .action_evaluations_mut()
                .install_transition_exact(
                    transition.expected_before(),
                    transition.after().clone(),
                )?;
            if installed != transition.after() {
                return Err(AuthorityRecordApplyError::ActionEvaluationTransitionMismatch);
            }
            let insertion = admission.scheduler_insertion();
            let expected_insertion = [(insertion.scheduler_key(), insertion.work().clone())];
            install_scheduler_delta(
                &mut scheduler,
                vec![SchedulerInsertion::new(
                    SchedulerProducerOrdinal::new(0),
                    insertion.work().clone(),
                )],
                &expected_insertion,
            )?;
        }
        AuthorityRecordBody::Management(batch) => {
            if batch.preserved_frontier() != expected.clock().frontier() {
                return Err(AuthorityRecordApplyError::ManagementFrontierMismatch);
            }
            match batch.cause() {
                ManagementCauseRecord::HostRequests(entries) => {
                    for entry in entries {
                        runtime_control.management_mut().insert_exact(
                            entry.request(),
                            entry.fingerprint(),
                            entry.outcome(),
                        )?;
                        match entry.operation() {
                            crate::kernel::SessionManagement::ResolveActionEvaluation {
                                invocation,
                                disposition,
                            } => {
                                let effect = entry.action_evaluation().ok_or(
                                    AuthorityRecordApplyError::ManagementTransitionMismatch,
                                )?;
                                apply_action_evaluation_management(
                                    &mut runtime_control,
                                    &mut scheduler,
                                    entry.request(),
                                    invocation,
                                    disposition,
                                    effect,
                                )?;
                            }
                            crate::kernel::SessionManagement::Retire(retirement) => {
                                if entry.action_evaluation().is_some() {
                                    return Err(
                                        AuthorityRecordApplyError::ManagementTransitionMismatch,
                                    );
                                }
                                runtime_control.retire(retirement, entry.request())?;
                            }
                            crate::kernel::SessionManagement::SealAdmissionThrough(frontier) => {
                                if entry.action_evaluation().is_some() {
                                    return Err(
                                        AuthorityRecordApplyError::ManagementTransitionMismatch,
                                    );
                                }
                                clock = clock.seal_admission_through(frontier)?;
                            }
                            crate::kernel::SessionManagement::Pause
                            | crate::kernel::SessionManagement::Resume
                            | crate::kernel::SessionManagement::Quarantine
                            | crate::kernel::SessionManagement::Fail => {
                                if entry.action_evaluation().is_some() {
                                    return Err(
                                        AuthorityRecordApplyError::ManagementTransitionMismatch,
                                    );
                                }
                            }
                        }
                    }
                    if entries
                        .iter()
                        .any(|entry| entry.operation() == crate::kernel::SessionManagement::Resume)
                    {
                        let sealed_frontier = clock.frontier();
                        (clock, safety_blocker) = expected.resume_projection()?;
                        if sealed_frontier > clock.frontier() {
                            clock = clock.seal_admission_through(sealed_frontier)?;
                        }
                    }
                    if entries.iter().any(|entry| {
                        matches!(
                            entry.operation(),
                            crate::kernel::SessionManagement::Quarantine
                                | crate::kernel::SessionManagement::Fail
                        )
                    }) {
                        safety_blocker = None;
                    }
                }
                ManagementCauseRecord::KernelSafety(cause) => {
                    safety_blocker = Some(crate::kernel::KernelSafetyBlocker::new(*cause));
                }
            }
            mode = batch.resulting_mode();
        }
        AuthorityRecordBody::Moment(batch) => {
            let mut expected_consumed = BTreeMap::new();
            for delivery in batch.command_deliveries() {
                record_expected_consumption(
                    &mut expected_consumed,
                    delivery.scheduler_key(),
                    ScheduledWork::command(delivery.scheduled().clone()),
                )?;
            }
            for delivery in batch.post_commit_deliveries() {
                record_expected_consumption(
                    &mut expected_consumed,
                    delivery.scheduler_key(),
                    ScheduledWork::PostCommit(delivery.dispatch().clone()),
                )?;
            }
            for delivery in batch.lifecycle_deliveries() {
                record_expected_consumption(
                    &mut expected_consumed,
                    delivery.scheduler_key(),
                    ScheduledWork::lifecycle(delivery.work()),
                )?;
            }
            for delivery in batch.action_ready_deliveries() {
                record_expected_consumption(
                    &mut expected_consumed,
                    delivery.scheduler_key(),
                    ScheduledWork::action_ready(delivery.ready()),
                )?;
            }
            for delivery in batch.action_evaluation_deliveries() {
                record_expected_consumption(
                    &mut expected_consumed,
                    delivery.scheduler_key(),
                    ScheduledWork::action_evaluation(delivery.work()),
                )?;
            }
            for delivery in batch.attempt_resolved_deliveries() {
                record_expected_consumption(
                    &mut expected_consumed,
                    delivery.scheduler_key(),
                    ScheduledWork::attempt_resolved(delivery.resolved()),
                )?;
            }
            for delivery in batch.relocation_process_deliveries() {
                record_expected_consumption(
                    &mut expected_consumed,
                    delivery.scheduler_key(),
                    ScheduledWork::process(delivery.wake()),
                )?;
            }
            if expected_consumed.keys().copied().collect::<Vec<_>>() != batch.consumed_keys() {
                return Err(AuthorityRecordApplyError::SchedulerPlanMismatch);
            }
            for key in batch.consumed_keys() {
                let expected_work = expected_consumed
                    .get(key)
                    .ok_or(AuthorityRecordApplyError::SchedulerPlanMismatch)?;
                match scheduler.consume_exact(*key) {
                    None => {
                        return Err(AuthorityRecordApplyError::ScheduledWorkMissing { key: *key });
                    }
                    Some(actual) if &actual == expected_work => {}
                    Some(_) => {
                        return Err(AuthorityRecordApplyError::ScheduledWorkMismatch { key: *key });
                    }
                }
            }
            for routing in batch.evidence_routing() {
                validate_evidence_routing(&accepted, &runtime_control, batch, *routing)?;
            }
            for transition in batch.action_opportunity_transitions() {
                apply_action_opportunity_transition(
                    &mut runtime_control,
                    transition,
                    batch.action_evaluation_invocation_openings(),
                )?;
            }
            for transition in batch.action_evaluation_invocation_transitions() {
                let actual = runtime_control
                    .action_evaluations_mut()
                    .install_transition_exact(
                        transition.expected_before(),
                        transition.after().clone(),
                    )?;
                if actual != transition.after() {
                    return Err(AuthorityRecordApplyError::ActionEvaluationTransitionMismatch);
                }
            }
            for opening in batch.action_evaluation_invocation_openings() {
                let invocation = opening.invocation();
                let waiting = runtime_control
                    .action_opportunities()
                    .get(invocation.opportunity())
                    .cloned()
                    .ok_or(AuthorityRecordApplyError::ActionEvaluationTransitionMismatch)?;
                let installed = match invocation.state() {
                    ActionEvaluationInvocationState::DispatchPending => runtime_control
                        .action_evaluations_mut()
                        .install_dispatch(invocation.clone(), &waiting),
                    ActionEvaluationInvocationState::FallbackPending {
                        cause: ActionEvaluationFallbackCause::ArtifactRejected(_),
                        ..
                    } => runtime_control
                        .action_evaluations_mut()
                        .install_artifact_rejection(invocation.clone(), &waiting),
                    ActionEvaluationInvocationState::ResultCaptured { .. }
                    | ActionEvaluationInvocationState::FallbackPending { .. }
                    | ActionEvaluationInvocationState::Terminal(_) => {
                        return Err(AuthorityRecordApplyError::ActionEvaluationTransitionMismatch);
                    }
                }?;
                if installed != invocation {
                    return Err(AuthorityRecordApplyError::ActionEvaluationTransitionMismatch);
                }
            }
            for opening in batch.action_opportunity_openings() {
                let actual = runtime_control
                    .open_action_opportunity(opening.opportunity().clone())
                    .map_err(AuthorityRecordApplyError::ActionOpportunityLedger)?;
                if actual != opening.opportunity() {
                    return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
                }
            }
            for attempt in batch.attempts() {
                retain_command_result(&mut runtime_control, attempt)?;
            }
            accepted = apply_containment_transfers(&accepted, batch.containment_delta())?;
            for transition in batch.relocation_process_transitions() {
                accepted = apply_relocation_process_transition(
                    &mut runtime_control,
                    &accepted,
                    transition,
                    batch.relocation_attempts(),
                    batch.relocation_process_deliveries(),
                    batch.moment(),
                )?;
            }
            for assimilation in batch.evidence_assimilations() {
                let epistemic = accepted
                    .epistemic()
                    .assimilate(
                        assimilation.actor(),
                        assimilation.expected_version(),
                        assimilation.evidence().to_vec(),
                    )
                    .map_err(|_| AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
                accepted = accepted_with_epistemic(&accepted, epistemic);
            }
            for transition in batch.appraisal_transitions() {
                match *transition {
                    ContainmentAppraisalTransitionRecord::Present { before, after } => {
                        if runtime_control
                            .appraisals()
                            .get(after.actor(), after.item())
                            != before
                        {
                            return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
                        }
                        runtime_control.appraisals_mut().retain(after);
                    }
                    ContainmentAppraisalTransitionRecord::Retracted {
                        before,
                        supporting_evidence,
                    } => {
                        validate_appraisal_retraction(&accepted, before, supporting_evidence)?;
                        if !runtime_control.appraisals_mut().retract_exact(before) {
                            return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
                        }
                    }
                }
            }
            for adoption in batch.intent_adoptions() {
                let agency = accepted
                    .agency()
                    .adopt_intent(adoption.intent())
                    .map_err(|_| AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
                accepted = accepted_with_agency(&accepted, agency);
            }
            for transition in batch.intent_transitions() {
                let before = transition.before();
                if accepted.agency().intent(before.id()).copied() != Some(before) {
                    return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
                }
                let operation = intent_transition_between(before, transition.after())?;
                let agency = accepted
                    .agency()
                    .transition_intent(before.id(), before.version(), operation)
                    .map_err(|_| AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
                accepted = accepted_with_agency(&accepted, agency);
            }
            for start in batch.activity_starts() {
                let agency = accepted
                    .agency()
                    .start_activity(start.activity(), true)
                    .map_err(|_| AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
                accepted = accepted_with_agency(&accepted, agency);
            }
            for transition in batch.activity_transitions() {
                let before = transition.before();
                if accepted.agency().activity(before.id()).copied() != Some(before) {
                    return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
                }
                let operation = activity_transition_between(before, transition.after())?;
                let agency = accepted
                    .agency()
                    .transition_activity(before.id(), before.version(), operation)
                    .map_err(|_| AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
                accepted = accepted_with_agency(&accepted, agency);
            }
            for transition in batch.activity_terminal_transitions() {
                let activity_before = transition.activity_before();
                let activity_after = transition.activity_after();
                let intent_before = transition.intent_before();
                let intent_after = transition.intent_after();
                if accepted.agency().activity(activity_before.id()).copied()
                    != Some(activity_before)
                    || accepted.agency().intent(intent_before.id()).copied() != Some(intent_before)
                {
                    return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
                }
                let (activity_operation, intent_operation) = activity_terminal_transitions_between(
                    activity_before,
                    activity_after,
                    intent_before,
                    intent_after,
                )?;
                let agency = accepted
                    .agency()
                    .transition_activity(
                        activity_before.id(),
                        activity_before.version(),
                        activity_operation,
                    )
                    .and_then(|agency| {
                        agency.transition_intent(
                            intent_before.id(),
                            intent_before.version(),
                            intent_operation,
                        )
                    })
                    .map_err(|_| AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
                if agency.activity(activity_before.id()).copied() != Some(activity_after)
                    || agency.intent(intent_before.id()).copied() != Some(intent_after)
                {
                    return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
                }
                accepted = accepted_with_agency(&accepted, agency);
            }
            for mutation in batch.lifecycle_control_mutations() {
                if !mutation.requested().is_empty() {
                    runtime_control
                        .lifecycle_mut()
                        .request(mutation.actor(), mutation.role(), mutation.requested())
                        .map_err(|_| AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
                }
                if let Some(generation) = mutation.completed() {
                    runtime_control
                        .lifecycle_mut()
                        .complete(mutation.actor(), mutation.role(), generation)
                        .map_err(|_| AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
                }
            }

            let mut insertions = Vec::with_capacity(batch.scheduler_insertions().len());
            let mut expected_insertions = Vec::with_capacity(batch.scheduler_insertions().len());
            for (position, insertion) in batch.scheduler_insertions().iter().enumerate() {
                let work = insertion.work().clone();
                insertions.push(SchedulerInsertion::new(
                    producer_ordinal(position),
                    work.clone(),
                ));
                expected_insertions.push((insertion.scheduler_key(), work));
            }
            install_scheduler_delta(&mut scheduler, insertions, &expected_insertions)?;
            clock = clock.after_fire(batch.moment(), batch.resulting_frontier())?;
        }
    }

    let resulting_head = SessionHead::from_authority_projection(
        resulting_cursor,
        mode,
        clock,
        accepted,
        runtime_control,
        scheduler,
        safety_blocker,
    );
    Ok(AppliedAuthorityRecord {
        resulting_head,
        record,
    })
}

fn record_expected_consumption(
    expected: &mut BTreeMap<SchedulerKey, ScheduledWork>,
    key: SchedulerKey,
    work: ScheduledWork,
) -> Result<(), AuthorityRecordApplyError> {
    if expected.insert(key, work).is_some() {
        return Err(AuthorityRecordApplyError::SchedulerPlanMismatch);
    }
    Ok(())
}

fn apply_action_opportunity_transition(
    runtime_control: &mut RuntimeControlState,
    transition: &ActionOpportunityTransitionRecord,
    evaluation_openings: &[ActionEvaluationInvocationOpeningRecord],
) -> Result<(), AuthorityRecordApplyError> {
    let before = transition.before();
    let after = transition.after();
    let opportunity = before.id();
    if after.id() != opportunity
        || runtime_control.action_opportunities().get(opportunity) != Some(before)
    {
        return Err(AuthorityRecordApplyError::ActionOpportunityTransitionMismatch { opportunity });
    }
    let successor = match (before.state(), after.state()) {
        (
            ActionOpportunityState::Open,
            ActionOpportunityState::WaitingForEvaluation(invocation),
        ) => {
            let opening = evaluation_openings
                .iter()
                .find(|opening| opening.invocation().invocation() == invocation)
                .ok_or(
                    AuthorityRecordApplyError::ActionOpportunityTransitionMismatch { opportunity },
                )?;
            let (successor, derived) = runtime_control.begin_action_evaluation(
                opportunity,
                before.version(),
                *opening.invocation().policy_semantics(),
                *opening.invocation().action_input_fingerprint(),
            )?;
            if derived != invocation {
                return Err(
                    AuthorityRecordApplyError::ActionOpportunityTransitionMismatch { opportunity },
                );
            }
            successor
        }
        (
            ActionOpportunityState::WaitingForEvaluation(invocation),
            ActionOpportunityState::Open,
        ) if before.evaluation_generation() == after.evaluation_generation() => {
            runtime_control.resume_action_evaluation(opportunity, before.version(), invocation)?
        }
        (
            ActionOpportunityState::WaitingForEvaluation(invocation),
            ActionOpportunityState::Open,
        ) => runtime_control.reopen_action_evaluation(opportunity, before.version(), invocation)?,
        (ActionOpportunityState::Open, ActionOpportunityState::Consumed(disposition)) => {
            runtime_control.consume_action_opportunity(
                opportunity,
                before.version(),
                disposition,
            )?
        }
        _ => {
            return Err(
                AuthorityRecordApplyError::ActionOpportunityTransitionMismatch { opportunity },
            );
        }
    };
    if successor != after {
        return Err(AuthorityRecordApplyError::ActionOpportunityTransitionMismatch { opportunity });
    }
    Ok(())
}

fn validate_evidence_routing(
    accepted: &world_model::AcceptedState,
    runtime_control: &RuntimeControlState,
    batch: &MomentBatchRecord,
    routing: EvidenceRoutingRecord,
) -> Result<(), AuthorityRecordApplyError> {
    let evidence = routing.evidence();
    match routing.source() {
        EvidenceRoutingSource::PhysicalEvent {
            dispatch,
            event_index,
        } => {
            let event = batch
                .post_commit_deliveries()
                .get(
                    usize::try_from(dispatch.index())
                        .map_err(|_| AuthorityRecordApplyError::LifecycleTransitionMismatch)?,
                )
                .and_then(|delivery| {
                    delivery
                        .dispatch()
                        .reaction()
                        .events()
                        .get(usize::try_from(event_index).ok()?)
                })
                .copied()
                .ok_or(AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
            if evidence.observer() != event.actor()
                || world_model::EvidenceRecord::direct_physical_event(
                    evidence.observer(),
                    evidence.generation(),
                    event,
                ) != evidence
            {
                return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
            }
        }
        EvidenceRoutingSource::RejectedContainmentAttempt { attempt } => {
            let attempt = batch
                .attempts()
                .get(
                    usize::try_from(attempt.index())
                        .map_err(|_| AuthorityRecordApplyError::LifecycleTransitionMismatch)?,
                )
                .ok_or(AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
            if !matches!(
                attempt.resolution(),
                super::RecordedCommandResolution::Rejected(
                    StableCommandRejection::Stale | StableCommandRejection::RequirementUnsatisfied
                )
            ) {
                return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
            }
            let AttemptSubjectRecord::EvaluatedCommand(command) = attempt.subject() else {
                return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
            };
            let scheduled = batch
                .resolutions()
                .iter()
                .filter_map(|resolution| match *resolution {
                    DeliveryResolutionRecord::NewCommand {
                        delivery,
                        attempt: candidate,
                    } if candidate == attempt.id() => batch
                        .command_deliveries()
                        .get(usize::try_from(delivery.index()).ok()?),
                    _ => None,
                })
                .find(|delivery| delivery.command() == command)
                .ok_or(AuthorityRecordApplyError::LifecycleTransitionMismatch)?
                .scheduled();
            let opportunity = scheduled
                .action_opportunity()
                .and_then(|opportunity| runtime_control.action_opportunities().get(opportunity))
                .ok_or(AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
            let (actor, item, source, destination) = containment_command_subject(command)
                .ok_or(AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
            let scope = opportunity
                .interaction_scope()
                .containment_scope()
                .filter(|scope| {
                    opportunity.actor() == actor
                        && scope.source() == source
                        && scope.permits_item(item)
                        && scope.destinations().binary_search(&destination).is_ok()
                })
                .ok_or(AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
            let belief = accepted
                .epistemic()
                .contained_in(actor, item)
                .filter(|belief| belief.container() == scope.source())
                .ok_or(AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
            if accepted
                .domain()
                .containment_for(item)
                .map(|record| record.container())
                == Some(source)
                || world_model::EvidenceRecord::direct_item_absent(
                    actor,
                    evidence.generation(),
                    item,
                    belief.container(),
                ) != evidence
            {
                return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
            }
        }
    }
    Ok(())
}

fn containment_command_subject(
    command: &CommandEnvelope,
) -> Option<(ActorId, EntityId, EntityId, EntityId)> {
    let actor = command_binding_actor(command, "actor")?;
    let item = command_binding_entity(command, "item")?;
    let source = command_binding_entity(command, "source")?;
    let destination = command_binding_entity(command, "destination")?;
    (actor == command.actor()).then_some((actor, item, source, destination))
}

fn command_binding_actor(command: &CommandEnvelope, role: &str) -> Option<ActorId> {
    match command
        .bindings()
        .iter()
        .find(|binding| binding.name().as_str() == role)?
        .value()
    {
        CommandValue::Actor(actor) => Some(actor),
        CommandValue::Entity(_) => None,
    }
}

fn command_binding_entity(command: &CommandEnvelope, role: &str) -> Option<EntityId> {
    match command
        .bindings()
        .iter()
        .find(|binding| binding.name().as_str() == role)?
        .value()
    {
        CommandValue::Entity(entity) => Some(entity),
        CommandValue::Actor(_) => None,
    }
}

fn validate_appraisal_retraction(
    accepted: &world_model::AcceptedState,
    before: ContainmentAppraisal,
    supporting_evidence: world_model::EvidenceDeliveryId,
) -> Result<(), AuthorityRecordApplyError> {
    let evidence = accepted
        .epistemic()
        .evidence_record(supporting_evidence)
        .copied()
        .ok_or(AuthorityRecordApplyError::LifecycleTransitionMismatch)?;
    let EvidenceProvenance::DirectItemAbsent(observation) = evidence.provenance() else {
        return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
    };
    if evidence.observer() != before.actor()
        || observation.item() != before.item()
        || observation.expected_container() != before.believed_current_container()
        || accepted
            .epistemic()
            .contained_in(before.actor(), before.item())
            .is_some()
    {
        return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
    }
    Ok(())
}

fn apply_relocation_process_transition(
    runtime_control: &mut RuntimeControlState,
    accepted: &world_model::AcceptedState,
    transition: &RelocationProcessTransitionRecord,
    attempts: &[RelocationAttemptRecord],
    deliveries: &[super::RelocationProcessDeliveryRecord],
    moment: world_core::SimMoment,
) -> Result<world_model::AcceptedState, AuthorityRecordApplyError> {
    let after = transition.after();
    match (transition.before(), after.status()) {
        (None, RelocationProcessStatus::Active { .. }) => {
            require_relocation_action_cause(
                transition,
                attempts,
                RelocationInteraction::Start(after.route().id()),
            )?;
            let actual = runtime_control
                .relocation_processes_mut()
                .start(after.actor(), after.route(), moment.time())
                .map_err(AuthorityRecordApplyError::RelocationProcess)?;
            if actual != after {
                return Err(AuthorityRecordApplyError::RelocationTransitionMismatch);
            }
            let route = after.route();
            if transition.event()
                != Some(PhysicalEvent::actor_departed(
                    after.id(),
                    after.actor(),
                    route.source(),
                    route.destination(),
                ))
            {
                return Err(AuthorityRecordApplyError::RelocationTransitionMismatch);
            }
            apply_relocation_departure(accepted, after)
                .map_err(AuthorityRecordApplyError::RelocationPosition)
        }
        (Some(before), RelocationProcessStatus::Paused { .. }) => {
            require_relocation_action_cause(
                transition,
                attempts,
                RelocationInteraction::Pause(after.route().id()),
            )?;
            let (actual_before, actual_after) = runtime_control
                .relocation_processes_mut()
                .pause(before.id(), before.version(), moment.time())
                .map_err(AuthorityRecordApplyError::RelocationProcess)?;
            if (actual_before, actual_after, transition.event()) != (before, after, None) {
                return Err(AuthorityRecordApplyError::RelocationTransitionMismatch);
            }
            Ok(accepted.clone())
        }
        (Some(before), RelocationProcessStatus::Active { .. }) => {
            require_relocation_action_cause(
                transition,
                attempts,
                RelocationInteraction::Resume(after.route().id()),
            )?;
            let (actual_before, actual_after) = runtime_control
                .relocation_processes_mut()
                .resume(before.id(), before.version(), moment.time())
                .map_err(AuthorityRecordApplyError::RelocationProcess)?;
            if (actual_before, actual_after, transition.event()) != (before, after, None) {
                return Err(AuthorityRecordApplyError::RelocationTransitionMismatch);
            }
            Ok(accepted.clone())
        }
        (Some(before), RelocationProcessStatus::Completed { .. }) => {
            let RelocationProcessTransitionCause::Wake(delivery) = transition.cause() else {
                return Err(AuthorityRecordApplyError::RelocationTransitionMismatch);
            };
            let wake = deliveries
                .get(
                    usize::try_from(delivery.index())
                        .map_err(|_| AuthorityRecordApplyError::RelocationTransitionMismatch)?,
                )
                .map(|delivery| delivery.wake())
                .ok_or(AuthorityRecordApplyError::RelocationTransitionMismatch)?;
            if runtime_control.relocation_processes().classify_wake(wake)
                != RelocationWakeClassification::Current(before.id())
            {
                return Err(AuthorityRecordApplyError::RelocationTransitionMismatch);
            }
            let (actual_before, actual_after) = runtime_control
                .relocation_processes_mut()
                .complete(wake, moment.time())
                .map_err(AuthorityRecordApplyError::RelocationProcess)?;
            let route = after.route();
            if (actual_before, actual_after) != (before, after)
                || transition.event()
                    != Some(PhysicalEvent::actor_arrived(
                        after.id(),
                        after.actor(),
                        route.source(),
                        route.destination(),
                    ))
            {
                return Err(AuthorityRecordApplyError::RelocationTransitionMismatch);
            }
            apply_relocation_arrival(accepted, after)
                .map_err(AuthorityRecordApplyError::RelocationPosition)
        }
        _ => Err(AuthorityRecordApplyError::RelocationTransitionMismatch),
    }
}

fn require_relocation_action_cause(
    transition: &RelocationProcessTransitionRecord,
    attempts: &[RelocationAttemptRecord],
    interaction: RelocationInteraction,
) -> Result<(), AuthorityRecordApplyError> {
    let RelocationProcessTransitionCause::Action(delivery) = transition.cause() else {
        return Err(AuthorityRecordApplyError::RelocationTransitionMismatch);
    };
    let attempt = attempts
        .iter()
        .find(|attempt| attempt.resolution_delivery() == delivery)
        .ok_or(AuthorityRecordApplyError::RelocationTransitionMismatch)?;
    if attempt.interaction() != interaction
        || attempt.resolution()
            != (RelocationAttemptResolution::Accepted {
                process: transition.after().id(),
            })
    {
        return Err(AuthorityRecordApplyError::RelocationTransitionMismatch);
    }
    Ok(())
}

fn install_scheduler_delta(
    scheduler: &mut SchedulerState,
    insertions: Vec<SchedulerInsertion>,
    expected: &[(SchedulerKey, ScheduledWork)],
) -> Result<(), AuthorityRecordApplyError> {
    if insertions.is_empty() {
        return Ok(());
    }
    let plan = scheduler.plan_batch(insertions)?;
    if plan.entries() != expected {
        return Err(AuthorityRecordApplyError::SchedulerPlanMismatch);
    }
    scheduler.install_batch_exact(plan)?;
    Ok(())
}

fn apply_action_evaluation_management(
    runtime_control: &mut RuntimeControlState,
    scheduler: &mut SchedulerState,
    request: crate::kernel::ManagementRequestId,
    invocation: world_model::ActionEvaluationInvocationId,
    disposition: crate::kernel::ActionEvaluationManagementDisposition,
    effect: &super::ActionEvaluationManagementRecord,
) -> Result<(), AuthorityRecordApplyError> {
    let transition = effect.transition();
    if transition.cause() != ActionEvaluationInvocationTransitionCause::Management(request)
        || transition.after().invocation() != invocation
    {
        return Err(AuthorityRecordApplyError::ManagementTransitionMismatch);
    }
    let before = runtime_control
        .action_evaluations()
        .get(invocation)
        .ok_or(AuthorityRecordApplyError::ManagementTransitionMismatch)?;
    if before.digest() != transition.expected_before() {
        return Err(AuthorityRecordApplyError::ManagementTransitionMismatch);
    }
    match before.state() {
        ActionEvaluationInvocationState::DispatchPending => {
            if effect.scheduler_removal().is_some() {
                return Err(AuthorityRecordApplyError::ManagementTransitionMismatch);
            }
        }
        ActionEvaluationInvocationState::ResultCaptured {
            effective,
            scheduler_key,
            ..
        } => {
            let removal = effect
                .scheduler_removal()
                .ok_or(AuthorityRecordApplyError::ManagementTransitionMismatch)?;
            let expected_work =
                ScheduledWork::action_evaluation(ActionEvaluationWork::result_ready(
                    invocation,
                    before.opportunity(),
                    before.waiting_version(),
                    *effective,
                ));
            if removal.scheduler_key() != *scheduler_key || removal.work() != &expected_work {
                return Err(AuthorityRecordApplyError::ManagementTransitionMismatch);
            }
            let actual = scheduler.remove_exact(removal.scheduler_key()).ok_or(
                AuthorityRecordApplyError::ScheduledWorkMissing {
                    key: removal.scheduler_key(),
                },
            )?;
            if actual != *removal.work() {
                return Err(AuthorityRecordApplyError::ScheduledWorkMismatch {
                    key: removal.scheduler_key(),
                });
            }
        }
        ActionEvaluationInvocationState::FallbackPending { .. }
        | ActionEvaluationInvocationState::Terminal(_) => {
            return Err(AuthorityRecordApplyError::ManagementTransitionMismatch);
        }
    }

    let cause = disposition.fallback_cause();
    let insertion = effect.fallback_insertion();
    let ActionEvaluationInvocationState::FallbackPending {
        cause: after_cause,
        scheduler_key,
    } = transition.after().state()
    else {
        return Err(AuthorityRecordApplyError::ManagementTransitionMismatch);
    };
    let expected_work = ScheduledWork::action_evaluation(ActionEvaluationWork::fallback(
        invocation,
        before.opportunity(),
        before.waiting_version(),
        cause,
        insertion.scheduler_key().moment(),
    ));
    if *after_cause != cause
        || *scheduler_key != insertion.scheduler_key()
        || insertion.work() != &expected_work
    {
        return Err(AuthorityRecordApplyError::ManagementTransitionMismatch);
    }
    let installed = runtime_control
        .action_evaluations_mut()
        .install_transition_exact(transition.expected_before(), transition.after().clone())?;
    if installed != transition.after() {
        return Err(AuthorityRecordApplyError::ManagementTransitionMismatch);
    }
    let expected_insertion = [(insertion.scheduler_key(), insertion.work().clone())];
    install_scheduler_delta(
        scheduler,
        vec![SchedulerInsertion::new(
            SchedulerProducerOrdinal::new(0),
            insertion.work().clone(),
        )],
        &expected_insertion,
    )
}

fn producer_ordinal(position: usize) -> SchedulerProducerOrdinal {
    SchedulerProducerOrdinal::new(
        u32::try_from(position)
            .unwrap_or_else(|_| unreachable!("sealed scheduler insertion index must fit u32")),
    )
}

fn retain_command_result(
    runtime_control: &mut RuntimeControlState,
    attempt: &AttemptRecord,
) -> Result<(), AuthorityRecordApplyError> {
    match attempt.subject() {
        AttemptSubjectRecord::EvaluatedCommand(command) => {
            runtime_control.command_mut().insert_exact(
                command.source(),
                command.id(),
                command.fingerprint(),
                attempt.id(),
                attempt.resolution().outcome(),
            )?;
        }
        AttemptSubjectRecord::CommandIdCollision {
            source,
            command,
            fingerprints,
        } => {
            runtime_control.command_mut().insert_collision(
                *source,
                *command,
                fingerprints,
                attempt.id(),
            )?;
        }
    }
    Ok(())
}

fn accepted_with_epistemic(
    accepted: &world_model::AcceptedState,
    epistemic: world_model::EpistemicState,
) -> world_model::AcceptedState {
    world_model::AcceptedState::new(
        accepted.domain().clone(),
        epistemic,
        *accepted.social(),
        accepted.agency().clone(),
    )
}

fn accepted_with_agency(
    accepted: &world_model::AcceptedState,
    agency: world_model::AgencyState,
) -> world_model::AcceptedState {
    world_model::AcceptedState::new(
        accepted.domain().clone(),
        accepted.epistemic().clone(),
        *accepted.social(),
        agency,
    )
}

fn intent_transition_between(
    before: Intent,
    after: Intent,
) -> Result<IntentTransition, AuthorityRecordApplyError> {
    let transition = match (before.status(), after.status()) {
        (IntentStatus::Active, IntentStatus::Suspended) => IntentTransition::Suspend,
        (IntentStatus::Suspended, IntentStatus::Active) => IntentTransition::Resume,
        (IntentStatus::Active | IntentStatus::Suspended, IntentStatus::Achieved) => {
            IntentTransition::Achieve
        }
        (IntentStatus::Active | IntentStatus::Suspended, IntentStatus::Abandoned) => {
            IntentTransition::Abandon
        }
        (IntentStatus::Active | IntentStatus::Suspended, IntentStatus::Failed) => {
            IntentTransition::Fail
        }
        _ => return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch),
    };
    if before.transition(before.version(), transition).ok() != Some(after) {
        return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
    }
    Ok(transition)
}

fn activity_transition_between(
    before: Activity,
    after: Activity,
) -> Result<ActivityTransition, AuthorityRecordApplyError> {
    let transition = match (before.status(), after.status()) {
        (ActivityStatus::Active, ActivityStatus::Active) => {
            ActivityTransition::Continue(after.state())
        }
        (ActivityStatus::Active, ActivityStatus::Waiting) => {
            ActivityTransition::Wait(after.state())
        }
        (ActivityStatus::Active, ActivityStatus::Suspended) => ActivityTransition::Suspend,
        (ActivityStatus::Waiting | ActivityStatus::Suspended, ActivityStatus::Active) => {
            ActivityTransition::Resume(after.state())
        }
        (
            ActivityStatus::Active | ActivityStatus::Waiting | ActivityStatus::Suspended,
            ActivityStatus::Completed,
        ) => ActivityTransition::Complete,
        (
            ActivityStatus::Active | ActivityStatus::Waiting | ActivityStatus::Suspended,
            ActivityStatus::Failed,
        ) => ActivityTransition::Fail,
        (
            ActivityStatus::Active | ActivityStatus::Waiting | ActivityStatus::Suspended,
            ActivityStatus::Cancelled,
        ) => ActivityTransition::Cancel,
        _ => return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch),
    };
    if before.transition(before.version(), transition).ok() != Some(after) {
        return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
    }
    Ok(transition)
}

fn activity_terminal_transitions_between(
    activity_before: Activity,
    activity_after: Activity,
    intent_before: Intent,
    intent_after: Intent,
) -> Result<(ActivityTransition, IntentTransition), AuthorityRecordApplyError> {
    if activity_before.actor() != intent_before.actor()
        || activity_after.actor() != intent_after.actor()
        || activity_before.intent() != intent_before.id()
        || activity_after.intent() != intent_after.id()
        || !matches!(
            (activity_after.status(), intent_after.status()),
            (ActivityStatus::Completed, IntentStatus::Achieved)
                | (ActivityStatus::Failed, IntentStatus::Failed)
        )
    {
        return Err(AuthorityRecordApplyError::LifecycleTransitionMismatch);
    }
    Ok((
        activity_transition_between(activity_before, activity_after)?,
        intent_transition_between(intent_before, intent_after)?,
    ))
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use world_core::{ActorId, EntityId, Microstep, SimMoment, SimTime};
    use world_model::{
        AcceptedState, AgencyState, CommandAttemptOutcome, ContainerAuthorityRecord,
        ContainerRecord, ContainmentRecord, ContainmentTransferDelta, DomainState, EpistemicState,
        SocialState, StableCommandRejection,
    };

    use crate::attempt::{AttemptAuthorityDomainId, AttemptStepId, ReservationGrant, RunAttemptId};
    use crate::control::RuntimeControlState;
    use crate::execution::{
        CanonicalExecutionSpecV1, ExecutionConfigArtifactV3, ExecutionSemanticsManifestV1,
        ExternalInputBindingV1, InitialStateRootV1, ResolvedExecutionClosureManifestV1, RootSeed,
        SemanticImplementationBinding, SemanticImplementationId, TerminationContractV1,
    };
    use crate::kernel::{
        AdmitRequest, CommandProposal, ContainmentCandidate, ContainmentCandidateProposal,
        ContainmentCandidateSet, ContainmentCommandIdentity, InputId, ManageRequest,
        ManagementRequestId, MomentWorkDecision, MomentWorkInput, MomentWorkProposals,
        PostCommitRoutingDecision, PreparedCommandResolution, PreparedDelivery, PreparedFire,
        SessionManagement, derive_input_request_namespace, resolve_containment_candidates,
    };
    use crate::randomness::Blake3KeyedPrf256V1;
    use crate::scheduler::{PreparedScheduledCommand, ScheduledWork, SchedulerState};
    use crate::session::SessionMode;

    use super::*;
    use crate::authority::{
        AuthorityRecordBody, CapturedInputRecordId, DraftAuthorityRecord, DraftMomentBatch,
        seal_authority_record,
    };

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("authority apply fixture must be valid: {error}"),
        }
    }

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(ticks), Microstep::new(microstep))
    }

    fn accepted_state(
        containers: Vec<ContainerRecord>,
        containment: Vec<ContainmentRecord>,
        authority: Vec<ContainerAuthorityRecord>,
    ) -> AcceptedState {
        AcceptedState::new(
            valid(DomainState::new(containers, containment, authority)),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        )
    }

    struct TestExecution {
        closure: ResolvedExecutionClosureManifestV1,
        head: SessionHead,
    }

    fn root_execution(accepted: AcceptedState) -> TestExecution {
        let definitions = crate::kernel::fixtures::command_definitions();
        let interface = definitions
            .required_interfaces()
            .first()
            .unwrap_or_else(|| panic!("command fixture must require one semantic interface"))
            .clone();
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            definitions,
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            vec![SemanticImplementationBinding::new(
                interface,
                SemanticImplementationId::from_bytes([0xd0; 32]),
            )],
        ));
        let root = valid(InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted,
            Vec::new(),
        ));
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &semantics,
            RootSeed::from_bytes([0xd1; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        let closure = valid(ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ));
        let head = SessionHead::root(&closure);
        TestExecution { closure, head }
    }

    fn empty_execution() -> TestExecution {
        root_execution(accepted_state(Vec::new(), Vec::new(), Vec::new()))
    }

    fn transfer_fixture() -> (AcceptedState, ContainmentTransferDelta) {
        let actor = ActorId::from_bytes([0x41; 32]);
        let source = EntityId::from_bytes([0xe1; 32]);
        let destination = EntityId::from_bytes([0xe2; 32]);
        let item = EntityId::from_bytes([0xe3; 32]);
        let accepted = accepted_state(
            vec![
                ContainerRecord::new(source, 2),
                ContainerRecord::new(destination, 2),
            ],
            vec![ContainmentRecord::new(item, source)],
            vec![ContainerAuthorityRecord::new(actor, source)],
        );
        let delta = valid(ContainmentTransferDelta::new(
            actor,
            item,
            source,
            destination,
        ));
        (accepted, delta)
    }

    fn apply_ingress(
        head: &SessionHead,
        closure: &ResolvedExecutionClosureManifestV1,
        input: u64,
        effective: SimMoment,
        command: world_model::CommandEnvelope,
    ) -> SessionHead {
        let sealed = seal_authority_record(
            head,
            closure,
            DraftAuthorityRecord::admit_commands(
                head.cursor(),
                vec![AdmitRequest::new(InputId::new(input), effective, command)],
            ),
        )
        .unwrap_or_else(|error| panic!("ingress fixture must seal: {error:?}"));
        apply_authority_record(head, sealed)
            .unwrap_or_else(|error| panic!("ingress fixture must apply: {error:?}"))
            .into_parts()
            .0
    }

    fn evaluable_delivery(head: &SessionHead, key: SchedulerKey) -> PreparedDelivery {
        let scheduled = match head.scheduler().get(key) {
            Some(ScheduledWork::Command(scheduled)) => scheduled.as_ref().clone(),
            _ => panic!("fixture key must contain command work"),
        };
        PreparedDelivery::evaluable_command(key, scheduled)
    }

    fn resolved_delivery(
        head: &SessionHead,
        key: SchedulerKey,
        resolution: PreparedCommandResolution,
    ) -> PreparedDelivery {
        let scheduled = match head.scheduler().get(key) {
            Some(ScheduledWork::Command(scheduled)) => scheduled.as_ref().clone(),
            _ => panic!("fixture key must contain command work"),
        };
        PreparedDelivery::resolved_command(key, scheduled, resolution)
    }

    fn checked_draft(
        head: &SessionHead,
        closure: &ResolvedExecutionClosureManifestV1,
        deliveries: Vec<PreparedDelivery>,
        command_proposals: &[(ContainmentCommandIdentity, CommandProposal)],
    ) -> DraftMomentBatch {
        let due = deliveries
            .first()
            .map(PreparedDelivery::key)
            .unwrap_or_else(|| panic!("apply draft fixture must contain one delivery"))
            .moment();
        let resulting_frontier = head.clock().frontier().max(
            crate::scheduler::strictly_later_moment(due).unwrap_or_else(|error| {
                panic!("apply fixture moment must have a successor: {error:?}")
            }),
        );
        let prepared = PreparedFire::new(
            AttemptAuthorityDomainId::from_bytes([0x71; 32]),
            RunAttemptId::from_bytes([0x72; 32]),
            closure.specification().id(),
            AttemptStepId::from_bytes([0x73; 32]),
            ReservationGrant::FIRST,
            resulting_frontier,
            head.snapshot(),
            deliveries,
        )
        .unwrap_or_else(|error| panic!("prepared apply fixture must be valid: {error:?}"));
        let proposals_by_identity = command_proposals
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>();
        let mut decisions = Vec::new();
        let mut candidates = Vec::new();
        for input in prepared.work() {
            match input {
                MomentWorkInput::EvaluateCommand { work, command, .. } => {
                    let identity = ContainmentCommandIdentity::from_command(command);
                    let proposal = proposals_by_identity
                        .get(&identity)
                        .copied()
                        .unwrap_or(CommandProposal::Rejected(StableCommandRejection::Conflict));
                    decisions.push(MomentWorkDecision::command(work, proposal));
                    candidates.push(ContainmentCandidate::new(
                        identity,
                        command.actor(),
                        match proposal {
                            CommandProposal::Rejected(reason) => {
                                ContainmentCandidateProposal::Rejected(reason)
                            }
                            CommandProposal::AcceptedTransfer(delta) => {
                                ContainmentCandidateProposal::Transfer(delta)
                            }
                        },
                    ));
                }
                input @ MomentWorkInput::PostCommitDispatch { .. } => {
                    decisions.push(
                        MomentWorkDecision::route_post_commit(
                            input,
                            PostCommitRoutingDecision::DeliverEvidence(Vec::new()),
                        )
                        .unwrap_or_else(|error| {
                            panic!("post-commit fixture decision must be valid: {error:?}")
                        }),
                    );
                }
                MomentWorkInput::ActionReady { .. }
                | MomentWorkInput::ActionEvaluationResultReady { .. }
                | MomentWorkInput::EvidenceAssimilation { .. }
                | MomentWorkInput::Appraisal { .. }
                | MomentWorkInput::IntentReview { .. }
                | MomentWorkInput::ActivityInitialization { .. }
                | MomentWorkInput::AttemptResolved { .. }
                | MomentWorkInput::ActivityAdvance { .. }
                | MomentWorkInput::RelocationProcessWake { .. } => {
                    unreachable!("command and post-commit fixture cannot contain action work")
                }
            }
        }
        let proposals = MomentWorkProposals::from_decisions(&prepared, decisions)
            .unwrap_or_else(|error| panic!("apply proposals must be complete: {error:?}"));
        let candidates = ContainmentCandidateSet::new(candidates)
            .unwrap_or_else(|error| panic!("apply candidates must be unique: {error:?}"));
        let oracle = Blake3KeyedPrf256V1::from_root_seed(closure.specification().root_seed());
        let resolution = resolve_containment_candidates(
            prepared.moment(),
            prepared.base_snapshot().accepted(),
            &candidates,
            &oracle,
        );
        DraftMomentBatch::from_prepared(&prepared, &proposals, &resolution)
            .unwrap_or_else(|error| panic!("apply authority draft must be checked: {error:?}"))
    }

    #[test]
    fn ingress_application_installs_one_correlated_ledger_and_scheduler_entry() {
        let fixture = empty_execution();
        let command = crate::kernel::fixtures::command(0xd2, 7);
        let request = AdmitRequest::new(InputId::new(3), moment(2, 4), command);
        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::admit_commands(fixture.head.cursor(), vec![request.clone()]),
        )
        .unwrap_or_else(|error| panic!("ingress must seal: {error:?}"));
        let expected_cursor = sealed.expected_cursor();
        let resulting_cursor = sealed.resulting_cursor();
        let successor = apply_authority_record(&fixture.head, sealed)
            .unwrap_or_else(|error| panic!("ingress must apply: {error:?}"))
            .into_parts()
            .0;

        assert_eq!(successor.cursor(), resulting_cursor);
        let retained = successor
            .runtime_control()
            .input()
            .get(request.id())
            .unwrap_or_else(|| panic!("input outcome must be retained"));
        assert_eq!(retained.outcome().effective(), request.effective());
        assert!(matches!(
            successor.scheduler().first(),
            Some((key, ScheduledWork::Command(command)))
                if key.moment() == request.effective()
                    && command.input() == Some(request.id())
        ));

        assert_eq!(fixture.head.cursor(), expected_cursor);
        assert!(fixture.head.runtime_control().is_empty());
        assert!(fixture.head.scheduler().is_empty());
    }

    #[test]
    fn management_application_changes_only_mode_and_management_control() {
        let fixture = empty_execution();
        let request = ManageRequest::new(ManagementRequestId::new(8), SessionManagement::Pause);
        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::management(fixture.head.cursor(), vec![request]),
        )
        .unwrap_or_else(|error| panic!("management fixture must seal: {error:?}"));
        let resulting_cursor = sealed.resulting_cursor();
        let record_id = sealed.record().header().id();
        let applied = apply_authority_record(&fixture.head, sealed)
            .unwrap_or_else(|error| panic!("management fixture must apply: {error:?}"));

        assert_eq!(applied.record().header().id(), record_id);
        let successor = applied.resulting_head();
        assert_eq!(successor.cursor(), resulting_cursor);
        assert_eq!(successor.snapshot().revision(), resulting_cursor.revision());
        assert_eq!(successor.mode(), SessionMode::Paused);
        assert_eq!(successor.clock(), fixture.head.clock());
        assert_eq!(successor.accepted(), fixture.head.accepted());
        assert_eq!(successor.scheduler(), fixture.head.scheduler());
        assert!(successor.runtime_control().input().iter().next().is_none());
        assert!(
            successor
                .runtime_control()
                .command()
                .iter()
                .next()
                .is_none()
        );
        assert_eq!(
            successor
                .runtime_control()
                .management()
                .get(request.id())
                .unwrap_or_else(|| panic!("management result must be retained"))
                .outcome()
                .record(),
            record_id
        );
    }

    #[test]
    fn command_application_covers_rejected_retained_and_id_reuse_shapes() {
        let root = empty_execution();
        let command = crate::kernel::fixtures::command(0xd3, 8);
        let admitted = apply_ingress(
            &root.head,
            &root.closure,
            1,
            SimMoment::ORIGIN,
            command.clone(),
        );
        let first_key = admitted
            .scheduler()
            .first()
            .unwrap_or_else(|| panic!("admitted command must be scheduled"))
            .0;
        let identity = ContainmentCommandIdentity::from_command(&command);
        let rejected = seal_authority_record(
            &admitted,
            &root.closure,
            DraftAuthorityRecord::moment(
                admitted.cursor(),
                checked_draft(
                    &admitted,
                    &root.closure,
                    vec![evaluable_delivery(&admitted, first_key)],
                    &[(
                        identity,
                        CommandProposal::Rejected(StableCommandRejection::RequirementUnsatisfied),
                    )],
                ),
            ),
        )
        .unwrap_or_else(|error| panic!("rejected command must seal: {error:?}"));
        let original_attempt = match rejected.record().body() {
            AuthorityRecordBody::Moment(batch) => batch
                .attempts()
                .first()
                .unwrap_or_else(|| panic!("rejected record must contain one attempt"))
                .id(),
            _ => panic!("moment draft must materialize a moment record"),
        };
        let rejected_head = apply_authority_record(&admitted, rejected)
            .unwrap_or_else(|error| panic!("rejected command must apply: {error:?}"))
            .into_parts()
            .0;
        let retained = rejected_head
            .runtime_control()
            .command()
            .get(command.source(), command.id())
            .unwrap_or_else(|| panic!("new rejection must be retained"));
        assert_eq!(retained.attempt(), original_attempt);
        assert_eq!(
            retained.outcome(),
            CommandAttemptOutcome::Rejected(StableCommandRejection::RequirementUnsatisfied)
        );
        assert!(admitted.scheduler().get(first_key).is_some());
        assert!(rejected_head.scheduler().get(first_key).is_none());

        let replay = apply_ingress(
            &rejected_head,
            &root.closure,
            2,
            moment(1, 0),
            command.clone(),
        );
        let replay_key = replay
            .scheduler()
            .first()
            .unwrap_or_else(|| panic!("replay must be scheduled"))
            .0;
        let retained_record = seal_authority_record(
            &replay,
            &root.closure,
            DraftAuthorityRecord::moment(
                replay.cursor(),
                checked_draft(
                    &replay,
                    &root.closure,
                    vec![resolved_delivery(
                        &replay,
                        replay_key,
                        PreparedCommandResolution::Retained {
                            original_attempt,
                            outcome: retained.outcome(),
                        },
                    )],
                    &[],
                ),
            ),
        )
        .unwrap_or_else(|error| panic!("exact replay must seal: {error:?}"));
        let retained_head = apply_authority_record(&replay, retained_record)
            .unwrap_or_else(|error| panic!("exact replay must apply: {error:?}"))
            .into_parts()
            .0;
        assert_eq!(
            retained_head
                .runtime_control()
                .command()
                .get(command.source(), command.id()),
            Some(retained)
        );

        let mismatched_command = crate::kernel::fixtures::command_with_actor(0xd3, 8, 0x42);
        let mismatch = apply_ingress(
            &retained_head,
            &root.closure,
            3,
            moment(2, 0),
            mismatched_command,
        );
        let mismatch_key = mismatch
            .scheduler()
            .first()
            .unwrap_or_else(|| panic!("mismatched reuse must be scheduled"))
            .0;
        let mismatch_record = seal_authority_record(
            &mismatch,
            &root.closure,
            DraftAuthorityRecord::moment(
                mismatch.cursor(),
                checked_draft(
                    &mismatch,
                    &root.closure,
                    vec![resolved_delivery(
                        &mismatch,
                        mismatch_key,
                        PreparedCommandResolution::IdReuseMismatch { original_attempt },
                    )],
                    &[],
                ),
            ),
        )
        .unwrap_or_else(|error| panic!("ID reuse mismatch must seal: {error:?}"));
        let mismatch_head = apply_authority_record(&mismatch, mismatch_record)
            .unwrap_or_else(|error| panic!("ID reuse mismatch must apply: {error:?}"))
            .into_parts()
            .0;
        assert_eq!(
            mismatch_head
                .runtime_control()
                .command()
                .get(command.source(), command.id()),
            Some(retained)
        );
        assert!(mismatch_head.scheduler().is_empty());
    }

    #[test]
    fn duplicate_exact_deliveries_share_one_logical_attempt() {
        let root = empty_execution();
        let command = crate::kernel::fixtures::command(0xd7, 12);
        let first = apply_ingress(
            &root.head,
            &root.closure,
            11,
            SimMoment::ORIGIN,
            command.clone(),
        );
        let admitted = apply_ingress(
            &first,
            &root.closure,
            12,
            SimMoment::ORIGIN,
            command.clone(),
        );
        let due = admitted
            .scheduler()
            .clone_least_due()
            .unwrap_or_else(|| panic!("duplicate exact commands must be due together"));
        let deliveries = due
            .entries()
            .iter()
            .map(|(key, _)| evaluable_delivery(&admitted, *key))
            .collect();
        let draft = checked_draft(
            &admitted,
            &root.closure,
            deliveries,
            &[(
                ContainmentCommandIdentity::from_command(&command),
                CommandProposal::Rejected(StableCommandRejection::RequirementUnsatisfied),
            )],
        );
        let sealed = seal_authority_record(
            &admitted,
            &root.closure,
            DraftAuthorityRecord::moment(admitted.cursor(), draft),
        )
        .unwrap_or_else(|error| panic!("duplicate exact commands must seal: {error:?}"));

        let AuthorityRecordBody::Moment(batch) = sealed.record().body() else {
            panic!("duplicate exact commands must produce a moment record");
        };
        assert_eq!(batch.command_deliveries().len(), 2);
        assert_eq!(batch.attempts().len(), 1);
        let attempt = batch.attempts()[0].id();
        assert!(batch.resolutions().iter().all(|resolution| matches!(
            resolution,
            crate::authority::DeliveryResolutionRecord::NewCommand {
                attempt: referenced,
                ..
            } if *referenced == attempt
        )));

        let successor = apply_authority_record(&admitted, sealed)
            .unwrap_or_else(|error| panic!("shared attempt must apply once: {error:?}"))
            .into_parts()
            .0;
        assert!(successor.scheduler().is_empty());
        assert_eq!(
            successor
                .runtime_control()
                .command()
                .get(command.source(), command.id())
                .map(|entry| entry.attempt()),
            Some(attempt)
        );
    }

    #[test]
    fn collision_attempt_is_retained_once_and_later_deliveries_are_dispositions_only() {
        let root = empty_execution();
        let first_command = crate::kernel::fixtures::command(0xd8, 0);
        let second_command = crate::kernel::fixtures::command_with_actor(0xd8, 0, 0x42);
        let first = apply_ingress(
            &root.head,
            &root.closure,
            21,
            SimMoment::ORIGIN,
            first_command.clone(),
        );
        let admitted = apply_ingress(
            &first,
            &root.closure,
            22,
            SimMoment::ORIGIN,
            second_command.clone(),
        );
        let due = admitted
            .scheduler()
            .clone_least_due()
            .unwrap_or_else(|| panic!("colliding commands must be due together"));
        let deliveries = || {
            due.entries()
                .iter()
                .map(|(key, _)| {
                    resolved_delivery(&admitted, *key, PreparedCommandResolution::NewCollision)
                })
                .collect::<Vec<_>>()
        };
        let collision = seal_authority_record(
            &admitted,
            &root.closure,
            DraftAuthorityRecord::moment(
                admitted.cursor(),
                checked_draft(&admitted, &root.closure, deliveries(), &[]),
            ),
        )
        .unwrap_or_else(|error| panic!("new command collision must seal: {error:?}"));
        let mut reversed_deliveries = deliveries();
        reversed_deliveries.reverse();
        let reversed_collision = seal_authority_record(
            &admitted,
            &root.closure,
            DraftAuthorityRecord::moment(
                admitted.cursor(),
                checked_draft(&admitted, &root.closure, reversed_deliveries, &[]),
            ),
        )
        .unwrap_or_else(|error| panic!("permuted collision must seal: {error:?}"));
        assert_eq!(collision, reversed_collision);
        let AuthorityRecordBody::Moment(batch) = collision.record().body() else {
            panic!("collision must produce a moment record");
        };
        assert_eq!(batch.attempts().len(), 1);
        let collision_attempt = batch.attempts()[0].id();
        let (source, command, fingerprints) = batch.attempts()[0]
            .subject()
            .collision()
            .unwrap_or_else(|| panic!("collision attempt must carry a collision subject"));
        assert_eq!(source, first_command.source());
        assert_eq!(command, first_command.id());
        assert_eq!(
            fingerprints,
            &[
                first_command
                    .fingerprint()
                    .min(second_command.fingerprint()),
                first_command
                    .fingerprint()
                    .max(second_command.fingerprint()),
            ]
        );
        assert!(batch.resolutions().iter().all(|resolution| matches!(
            resolution,
            crate::authority::DeliveryResolutionRecord::NewCollision {
                attempt,
                ..
            } if *attempt == collision_attempt
        )));

        let collided = apply_authority_record(&admitted, collision)
            .unwrap_or_else(|error| panic!("collision tombstone must apply: {error:?}"))
            .into_parts()
            .0;
        assert!(matches!(
            collided.runtime_control().command().classify(
                first_command.source(),
                first_command.id(),
                first_command.fingerprint(),
            ),
            crate::control::CommandLedgerLookup::RetainedCollision {
                original_attempt,
            } if original_attempt == collision_attempt
        ));

        let replay = apply_ingress(
            &collided,
            &root.closure,
            23,
            moment(1, 0),
            first_command.clone(),
        );
        let replay_key = replay
            .scheduler()
            .first()
            .unwrap_or_else(|| panic!("collision replay must be scheduled"))
            .0;
        let retained_collision = seal_authority_record(
            &replay,
            &root.closure,
            DraftAuthorityRecord::moment(
                replay.cursor(),
                checked_draft(
                    &replay,
                    &root.closure,
                    vec![resolved_delivery(
                        &replay,
                        replay_key,
                        PreparedCommandResolution::RetainedCollision {
                            original_attempt: collision_attempt,
                        },
                    )],
                    &[],
                ),
            ),
        )
        .unwrap_or_else(|error| panic!("retained collision must seal: {error:?}"));
        let AuthorityRecordBody::Moment(batch) = retained_collision.record().body() else {
            panic!("retained collision must produce a moment record");
        };
        assert!(batch.attempts().is_empty());
        assert!(matches!(
            batch.resolutions(),
            [crate::authority::DeliveryResolutionRecord::RetainedCollision {
                original_attempt,
                ..
            }] if *original_attempt == collision_attempt
        ));
        let retained = apply_authority_record(&replay, retained_collision)
            .unwrap_or_else(|error| panic!("retained collision must apply: {error:?}"))
            .into_parts()
            .0;

        let mut runtime_control = retained.runtime_control().clone();
        runtime_control
            .command_mut()
            .retire_through(first_command.source(), first_command.id())
            .unwrap_or_else(|error| panic!("complete collision prefix must retire: {error:?}"));
        let retired_base = SessionHead::from_authority_projection(
            retained.cursor(),
            retained.mode(),
            retained.clock(),
            retained.accepted().clone(),
            runtime_control,
            retained.scheduler().clone(),
            retained.safety_blocker(),
        );
        let retired_delivery = apply_ingress(
            &retired_base,
            &root.closure,
            24,
            moment(2, 0),
            first_command,
        );
        let retired_key = retired_delivery
            .scheduler()
            .first()
            .unwrap_or_else(|| panic!("retired command delivery must be scheduled"))
            .0;
        let retired_record = seal_authority_record(
            &retired_delivery,
            &root.closure,
            DraftAuthorityRecord::moment(
                retired_delivery.cursor(),
                checked_draft(
                    &retired_delivery,
                    &root.closure,
                    vec![resolved_delivery(
                        &retired_delivery,
                        retired_key,
                        PreparedCommandResolution::Retired,
                    )],
                    &[],
                ),
            ),
        )
        .unwrap_or_else(|error| panic!("retired delivery must seal: {error:?}"));
        let AuthorityRecordBody::Moment(batch) = retired_record.record().body() else {
            panic!("retired delivery must produce a moment record");
        };
        assert!(batch.attempts().is_empty());
        assert!(matches!(
            batch.resolutions(),
            [crate::authority::DeliveryResolutionRecord::RetiredCommand { .. }]
        ));
        let after_retired = apply_authority_record(&retired_delivery, retired_record)
            .unwrap_or_else(|error| panic!("retired disposition must apply: {error:?}"))
            .into_parts()
            .0;
        assert!(after_retired.scheduler().is_empty());
    }

    #[test]
    fn accepted_transfer_updates_state_and_enqueues_but_does_not_consume_post_commit_work() {
        let (accepted, delta) = transfer_fixture();
        let root = root_execution(accepted);
        let admitted = apply_ingress(
            &root.head,
            &root.closure,
            4,
            SimMoment::ORIGIN,
            crate::kernel::fixtures::command(0xd4, 9),
        );
        let command_key = admitted
            .scheduler()
            .first()
            .unwrap_or_else(|| panic!("accepted command must be scheduled"))
            .0;
        let command = match admitted.scheduler().get(command_key) {
            Some(ScheduledWork::Command(scheduled)) => scheduled.command(),
            _ => panic!("accepted command key must contain command work"),
        };
        let sealed = seal_authority_record(
            &admitted,
            &root.closure,
            DraftAuthorityRecord::moment(
                admitted.cursor(),
                checked_draft(
                    &admitted,
                    &root.closure,
                    vec![evaluable_delivery(&admitted, command_key)],
                    &[(
                        ContainmentCommandIdentity::from_command(command),
                        CommandProposal::AcceptedTransfer(delta),
                    )],
                ),
            ),
        )
        .unwrap_or_else(|error| panic!("accepted transfer must seal: {error:?}"));
        let dispatch_key = match sealed.record().body() {
            AuthorityRecordBody::Moment(batch) => batch
                .scheduler_insertions()
                .first()
                .unwrap_or_else(|| panic!("accepted record must schedule one dispatch"))
                .scheduler_key(),
            _ => panic!("moment draft must materialize a moment record"),
        };
        let successor = apply_authority_record(&admitted, sealed)
            .unwrap_or_else(|error| panic!("accepted transfer must apply: {error:?}"))
            .into_parts()
            .0;

        assert_eq!(
            successor
                .accepted()
                .domain()
                .containment_for(delta.item())
                .unwrap_or_else(|| panic!("transferred item must remain contained"))
                .container(),
            delta.destination()
        );
        assert_eq!(successor.clock().now(), command_key.moment());
        assert!(matches!(
            successor.scheduler().get(dispatch_key),
            Some(ScheduledWork::PostCommit(dispatch))
                if dispatch.source_moment() == command_key.moment()
        ));
        assert_eq!(
            successor.scheduler().first().map(|(key, _)| key),
            Some(dispatch_key)
        );

        let dispatch = match successor.scheduler().get(dispatch_key) {
            Some(ScheduledWork::PostCommit(dispatch)) => dispatch.clone(),
            _ => panic!("accepted transition must schedule post-commit work"),
        };
        let consumed = seal_authority_record(
            &successor,
            &root.closure,
            DraftAuthorityRecord::moment(
                successor.cursor(),
                checked_draft(
                    &successor,
                    &root.closure,
                    vec![PreparedDelivery::post_commit(dispatch_key, dispatch)],
                    &[],
                ),
            ),
        )
        .unwrap_or_else(|error| panic!("post-commit consumption must seal: {error:?}"));
        let after_dispatch = apply_authority_record(&successor, consumed)
            .unwrap_or_else(|error| panic!("post-commit consumption must apply: {error:?}"))
            .into_parts()
            .0;
        assert!(matches!(
            successor.scheduler().get(dispatch_key),
            Some(ScheduledWork::PostCommit(_))
        ));
        assert!(after_dispatch.scheduler().is_empty());
    }

    #[test]
    fn failed_candidate_application_cannot_leak_partial_state() {
        let root = empty_execution();
        let request = AdmitRequest::new(
            InputId::new(5),
            moment(3, 0),
            crate::kernel::fixtures::command(0xd5, 10),
        );
        let sealed = seal_authority_record(
            &root.head,
            &root.closure,
            DraftAuthorityRecord::admit_commands(root.head.cursor(), vec![request.clone()]),
        )
        .unwrap_or_else(|error| panic!("ingress must seal: {error:?}"));
        let expected_cursor = sealed.expected_cursor();

        let conflicting_request = AdmitRequest::new(
            InputId::new(6),
            moment(4, 0),
            crate::kernel::fixtures::command(0xd6, 11),
        );
        let mut scheduler = SchedulerState::empty();
        let namespace = derive_input_request_namespace(
            root.head.cursor().epoch().lineage(),
            root.closure.specification().external_input_digest(),
        );
        let conflicting = PreparedScheduledCommand::prepare(namespace, &conflicting_request)
            .materialize(CapturedInputRecordId::from_bytes([0xf1; 32]));
        let plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(conflicting),
            )])
            .unwrap_or_else(|error| panic!("conflicting fixture must plan: {error:?}"));
        let conflicting_key = plan
            .entries()
            .first()
            .map(|(key, _)| *key)
            .unwrap_or_else(|| panic!("conflicting fixture plan must contain one entry"));
        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("conflicting fixture must install: {error:?}"));
        let inconsistent = SessionHead::from_authority_projection(
            root.head.cursor(),
            root.head.mode(),
            root.head.clock(),
            root.head.accepted().clone(),
            RuntimeControlState::empty(),
            scheduler,
            root.head.safety_blocker(),
        );

        assert_eq!(
            apply_authority_record(&inconsistent, sealed),
            Err(AuthorityRecordApplyError::SchedulerPlanMismatch)
        );
        assert!(inconsistent.runtime_control().is_empty());
        assert_eq!(
            inconsistent.scheduler().first().map(|(key, _)| key),
            Some(conflicting_key)
        );
        assert_eq!(inconsistent.cursor(), expected_cursor);
    }
}
