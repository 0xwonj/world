use std::collections::{BTreeMap, BTreeSet};

use world_model::{CommandId, CommandRequestFingerprint, CommandSource, WorldSnapshot};

use crate::action_evaluation::{
    ActionEvaluationCaptureLookup, ActionEvaluationCaptureOutcome, ActionEvaluationCaptureRequest,
    ActionEvaluationCaptureRequestError, ActionEvaluationResultSubmission,
    PendingActionEvaluationRaw,
};
use crate::attempt::{
    AttemptAuthorityDomainId, AttemptBinding, AttemptCreation, AttemptDisposition,
    AttemptDispositionStore, AttemptKey, AttemptPhase, AttemptPhaseError, AttemptStepId,
    CancelAttemptOutcome, CancelAttemptRequest, CancellationLookup, ReservedOperationDescriptor,
    RunAttemptControl, RunAttemptId, StepPublicationReceipt,
};
use crate::authority::{
    AuthorityAdmissionRecord, AuthorityPosition, AuthorityRecord, AuthorityRecordApplyError,
    AuthorityRecordBody, AuthorityRecordSealError, DraftAuthorityRecord, DraftMomentBatch,
    SealedAuthorityRecord, apply_authority_record, seal_authority_record,
};
use crate::control::{CommandLedgerLookup, LedgerRetirementError, RequestLedgerLookup};
use crate::execution::ResolvedExecutionClosureManifestV1;
use crate::kernel::{
    ActionEvaluationDecision, ActionProposal, AdmitOutcome, AdmitRequest,
    CommandFireClassification, CommandFireResolution, CommandProposal, ContainmentCandidate,
    ContainmentCandidateOutcome, ContainmentCandidateProposal, ContainmentCandidateSet,
    ContainmentCommandIdentity, FireOutcome, FirePreparation, FireRequest, KernelSafetyCause,
    KernelSafetyOutcome, ManageOutcome, ManageRequest, MomentWorkProposals,
    PreparedCommandResolution, PreparedDelivery, PreparedFire, PreparedFireFailure,
    PreparedFireFailureOutcome, PreparedKernelSafety, WorkProposal, resolve_containment_candidates,
    select_kernel_safety_cause,
};
use crate::lifecycle::LifecycleWork;
use crate::randomness::Blake3KeyedPrf256V1;
use crate::scheduler::{ScheduledWork, strictly_later_moment};
use crate::service::{
    RuntimeActionEvaluationCaptureError, RuntimeAttemptStatus, RuntimeControlError,
    RuntimeDriveError, RuntimeStartError,
};
use crate::session::SessionHead;

#[derive(Debug)]
pub(super) struct AttemptAggregate {
    pub(super) control: RunAttemptControl,
    pub(super) dispositions: AttemptDispositionStore,
    pub(super) head: SessionHead,
    pub(super) history: Vec<AuthorityRecord>,
    pub(super) receipts: BTreeMap<AttemptStepId, StepPublicationReceipt>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OpenedAttempt {
    attempt: RunAttemptId,
    binding: AttemptBinding,
}

impl OpenedAttempt {
    pub(crate) const fn attempt(self) -> RunAttemptId {
        self.attempt
    }

    pub(crate) const fn binding(self) -> AttemptBinding {
        self.binding
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AggregateRead {
    cursor: crate::authority::AuthorityCursor,
    mode: crate::session::SessionMode,
    admission_frontier: world_core::SimMoment,
    snapshot: WorldSnapshot,
    status: RuntimeAttemptStatus,
    safety_blocker: Option<crate::kernel::KernelSafetyBlocker>,
    same_time_wave_tranche: crate::session::SameTimeWaveTranche,
    pending_action_evaluations: Vec<PendingActionEvaluationRaw>,
}

impl AggregateRead {
    pub(crate) const fn cursor(&self) -> crate::authority::AuthorityCursor {
        self.cursor
    }

    pub(crate) const fn mode(&self) -> crate::session::SessionMode {
        self.mode
    }

    pub(crate) const fn admission_frontier(&self) -> world_core::SimMoment {
        self.admission_frontier
    }

    pub(crate) const fn snapshot(&self) -> &WorldSnapshot {
        &self.snapshot
    }

    pub(crate) const fn status(&self) -> &RuntimeAttemptStatus {
        &self.status
    }

    pub(crate) const fn safety_blocker(&self) -> Option<crate::kernel::KernelSafetyBlocker> {
        self.safety_blocker
    }

    pub(crate) const fn same_time_wave_tranche(&self) -> crate::session::SameTimeWaveTranche {
        self.same_time_wave_tranche
    }

    pub(crate) fn pending_action_evaluations(&self) -> &[PendingActionEvaluationRaw] {
        &self.pending_action_evaluations
    }
}

#[allow(
    clippy::result_large_err,
    reason = "aggregate transitions preserve complete terminal replay evidence in the runtime error"
)]
impl AttemptAggregate {
    pub(super) fn derive_creation(
        domain: AttemptAuthorityDomainId,
        key: AttemptKey,
        closure: &ResolvedExecutionClosureManifestV1,
    ) -> AttemptCreation {
        AttemptCreation::derive(domain, key, closure)
    }

    pub(super) fn create(
        creation: AttemptCreation,
        closure: ResolvedExecutionClosureManifestV1,
    ) -> Result<Self, RuntimeStartError> {
        let head = SessionHead::root(&closure);
        let control = RunAttemptControl::new(creation, closure, &head)
            .map_err(|_| RuntimeStartError::Integrity)?;
        Ok(Self {
            control,
            dispositions: AttemptDispositionStore::default(),
            head,
            history: Vec::new(),
            receipts: BTreeMap::new(),
        })
    }

    pub(super) fn verify_creation(
        &self,
        creation: AttemptCreation,
    ) -> Result<(), RuntimeStartError> {
        if self.control.creation() == creation {
            Ok(())
        } else {
            Err(RuntimeStartError::AttemptCreationConflict)
        }
    }

    pub(super) fn open(&mut self) -> Result<OpenedAttempt, RuntimeStartError> {
        reconcile(self).map_err(|_| RuntimeStartError::Integrity)?;
        Ok(OpenedAttempt {
            attempt: self.control.binding().attempt(),
            binding: self.control.binding(),
        })
    }

    pub(super) fn read(&self) -> AggregateRead {
        let status = match self.control.phase() {
            AttemptPhase::Active(_) => RuntimeAttemptStatus::Active,
            AttemptPhase::Reserved(_) => RuntimeAttemptStatus::Reserved,
            AttemptPhase::Finalized(finalization) => RuntimeAttemptStatus::Finalized(*finalization),
        };
        AggregateRead {
            cursor: self.head.cursor(),
            mode: self.head.mode(),
            admission_frontier: self.head.clock().frontier(),
            snapshot: self.head.snapshot(),
            status,
            safety_blocker: self.head.safety_blocker(),
            same_time_wave_tranche: self.head.clock().same_time_tranche(),
            pending_action_evaluations: self
                .head
                .runtime_control()
                .action_evaluations()
                .pending_raw(),
        }
    }

    #[cfg(test)]
    pub(super) fn reconcile_for_open(&mut self) -> Result<(), RuntimeStartError> {
        reconcile(self).map_err(|_| RuntimeStartError::Integrity)
    }

    pub(super) fn admit(
        &mut self,
        request: AdmitRequest,
    ) -> Result<AdmitOutcome, RuntimeDriveError> {
        match self
            .head
            .runtime_control()
            .input()
            .classify(request.id(), request.fingerprint())
        {
            RequestLedgerLookup::RetainedExact(outcome) => return Ok(outcome),
            RequestLedgerLookup::Retired => {
                return Err(RuntimeDriveError::InputRetired { id: request.id() });
            }
            RequestLedgerLookup::IdReuseMismatch => {
                return Err(RuntimeDriveError::InputIdReuse);
            }
            RequestLedgerLookup::Absent => {}
        }

        let operation = ReservedOperationDescriptor::admit_command(&request);
        reserve(self, operation)?;
        let step = reserved_step(self)?;
        let effective = request.effective();
        let sealed = match seal_authority_record(
            &self.head,
            self.control.closure(),
            DraftAuthorityRecord::admit_commands(self.head.cursor(), vec![request]),
        ) {
            Ok(sealed) => sealed,
            Err(error) => {
                release_unpublished(self, step)?;
                return Err(map_admit_seal_error(error));
            }
        };
        let (record, _) = publish_and_reconcile(self, sealed)?;
        Ok(AdmitOutcome::scheduled(record, effective))
    }

    pub(super) fn manage(
        &mut self,
        request: ManageRequest,
    ) -> Result<ManageOutcome, RuntimeDriveError> {
        match self
            .head
            .runtime_control()
            .management()
            .classify(request.id(), request.fingerprint())
        {
            RequestLedgerLookup::RetainedExact(outcome) => return Ok(outcome),
            RequestLedgerLookup::Retired => {
                return Err(RuntimeDriveError::ManagementRetired { id: request.id() });
            }
            RequestLedgerLookup::IdReuseMismatch => {
                return Err(RuntimeDriveError::ManagementIdReuse);
            }
            RequestLedgerLookup::Absent => {}
        }

        let operation = ReservedOperationDescriptor::manage(request);
        reserve(self, operation)?;
        let step = reserved_step(self)?;
        let sealed = match seal_authority_record(
            &self.head,
            self.control.closure(),
            DraftAuthorityRecord::management(self.head.cursor(), vec![request]),
        ) {
            Ok(sealed) => sealed,
            Err(error) => {
                release_unpublished(self, step)?;
                return Err(map_manage_seal_error(error));
            }
        };
        let (record, _) = publish_and_reconcile(self, sealed)?;
        Ok(ManageOutcome::applied(record, request.operation()))
    }

    pub(super) fn capture_action_evaluation_result(
        &mut self,
        submission: ActionEvaluationResultSubmission,
    ) -> Result<ActionEvaluationCaptureOutcome, RuntimeActionEvaluationCaptureError> {
        let capture = submission.capture();
        let invocation = submission.invocation();
        let retained = self
            .head
            .runtime_control()
            .action_evaluation_captures()
            .get(capture);
        let invocation_record = self
            .head
            .runtime_control()
            .action_evaluations()
            .get(invocation);

        if let Some(entry) = retained {
            if entry.invocation() != invocation {
                return Err(RuntimeActionEvaluationCaptureError::CaptureIdReuse { capture });
            }
            let Some(invocation_record) = invocation_record else {
                return Err(RuntimeActionEvaluationCaptureError::Integrity);
            };
            let request = ActionEvaluationCaptureRequest::resolve(
                submission,
                invocation_record,
                self.control
                    .closure()
                    .semantics()
                    .config()
                    .deferred_action_control(),
            )
            .map_err(|_| RuntimeActionEvaluationCaptureError::CaptureIdReuse { capture })?;
            return match self
                .head
                .runtime_control()
                .action_evaluation_captures()
                .classify(capture, invocation, request.fingerprint())
            {
                ActionEvaluationCaptureLookup::RetainedExact(outcome) => Ok(outcome),
                ActionEvaluationCaptureLookup::IdReuseMismatch => {
                    Err(RuntimeActionEvaluationCaptureError::CaptureIdReuse { capture })
                }
                ActionEvaluationCaptureLookup::Absent => {
                    Err(RuntimeActionEvaluationCaptureError::Integrity)
                }
            };
        }

        require_active(self).map_err(map_capture_drive_error)?;

        let invocation_record = invocation_record
            .ok_or(RuntimeActionEvaluationCaptureError::UnknownInvocation { invocation })?;
        let request = ActionEvaluationCaptureRequest::resolve(
            submission,
            invocation_record,
            self.control
                .closure()
                .semantics()
                .config()
                .deferred_action_control(),
        )
        .map_err(map_capture_request_error)?;
        match self
            .head
            .runtime_control()
            .action_evaluation_captures()
            .classify(capture, invocation, request.fingerprint())
        {
            ActionEvaluationCaptureLookup::Absent => {}
            ActionEvaluationCaptureLookup::RetainedExact(outcome) => return Ok(outcome),
            ActionEvaluationCaptureLookup::IdReuseMismatch => {
                return Err(RuntimeActionEvaluationCaptureError::CaptureIdReuse { capture });
            }
        }
        request
            .validate_new(invocation_record, self.head.clock().frontier())
            .map_err(map_capture_request_error)?;

        let operation = ReservedOperationDescriptor::admit_action_evaluation(&request);
        reserve(self, operation).map_err(map_capture_drive_error)?;
        let step = reserved_step(self).map_err(map_capture_drive_error)?;
        let sealed = match seal_authority_record(
            &self.head,
            self.control.closure(),
            DraftAuthorityRecord::admit_action_evaluation(self.head.cursor(), request.clone()),
        ) {
            Ok(sealed) => sealed,
            Err(error) => {
                release_unpublished(self, step).map_err(map_capture_drive_error)?;
                return Err(map_capture_seal_error(error));
            }
        };
        let (record, _) = publish_and_reconcile(self, sealed).map_err(map_capture_drive_error)?;
        Ok(request.outcome(record))
    }

    pub(super) fn prepare_fire(
        &mut self,
        domain: AttemptAuthorityDomainId,
        attempt: RunAttemptId,
        request: FireRequest,
    ) -> Result<FirePreparation, RuntimeDriveError> {
        require_active(self)?;
        if self.head.mode() != crate::session::SessionMode::Running {
            return Err(RuntimeDriveError::SessionNotRunning {
                current: self.head.mode(),
            });
        }
        if let Some(blocked_at) = self
            .head
            .runtime_control()
            .action_evaluations()
            .minimum_blocked_frontier()
            && request.through_moment() > blocked_at
        {
            return Err(RuntimeDriveError::ActionEvaluationFrontierBlocked { blocked_at });
        }
        let due = self
            .head
            .scheduler()
            .clone_least_due()
            .ok_or(RuntimeDriveError::NoScheduledWork)?;
        let moment = due.moment();
        if moment > request.through_moment() {
            return Err(RuntimeDriveError::NoWorkDue {
                next: moment,
                through: request.through_moment(),
            });
        }

        let mut absent_fingerprints =
            BTreeMap::<(CommandSource, CommandId), BTreeSet<CommandRequestFingerprint>>::new();
        for (_, work) in due.entries() {
            let ScheduledWork::Command(scheduled) = work else {
                continue;
            };
            let command = scheduled.command();
            if matches!(
                self.head.runtime_control().command().classify(
                    command.source(),
                    command.id(),
                    command.fingerprint(),
                ),
                CommandLedgerLookup::Absent
            ) {
                absent_fingerprints
                    .entry((command.source(), command.id()))
                    .or_default()
                    .insert(command.fingerprint());
            }
        }
        let due_keys = due
            .entries()
            .iter()
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        let evaluable_commands = absent_fingerprints
            .values()
            .filter(|fingerprints| fingerprints.len() == 1)
            .count();
        let evaluable_commands =
            u64::try_from(evaluable_commands).map_err(|_| RuntimeDriveError::Integrity)?;
        let safety_cause = select_kernel_safety_cause(
            self.control.closure().semantics().config(),
            self.head.clock().frontier(),
            &due_keys,
            evaluable_commands,
            self.head.clock().attempted_wave(moment),
        )
        .map_err(|_| RuntimeDriveError::Integrity)?;
        if let Some(cause) = safety_cause {
            reserve(self, ReservedOperationDescriptor::kernel_safety(cause))?;
            let reservation = self
                .control
                .reservation()
                .ok_or(RuntimeDriveError::Integrity)?;
            return Ok(FirePreparation::KernelSafety(PreparedKernelSafety::new(
                domain,
                attempt,
                self.control.closure().specification().id(),
                reservation.step(),
                reservation.grant(),
                self.head.cursor(),
                cause,
            )));
        }

        let resulting_frontier = self
            .head
            .clock()
            .frontier()
            .max(strictly_later_moment(moment).map_err(|_| RuntimeDriveError::Integrity)?);
        if let Some(blocked_at) = self
            .head
            .runtime_control()
            .action_evaluations()
            .minimum_blocked_frontier()
            && resulting_frontier > blocked_at
        {
            return Err(RuntimeDriveError::ActionEvaluationFrontierBlocked { blocked_at });
        }

        let mut deliveries = Vec::with_capacity(due.entries().len());
        for (key, work) in due.entries() {
            let delivery = match work {
                ScheduledWork::PostCommit(dispatch) => {
                    PreparedDelivery::post_commit(*key, dispatch.clone())
                }
                ScheduledWork::Process(wake) => PreparedDelivery::process(
                    *key,
                    *wake,
                    self.head
                        .runtime_control()
                        .relocation_processes()
                        .classify_wake(*wake),
                ),
                ScheduledWork::ActionReady(ready) => {
                    let opportunity = self
                        .head
                        .runtime_control()
                        .action_opportunities()
                        .get(ready.opportunity())
                        .ok_or(RuntimeDriveError::Integrity)?;
                    if opportunity.version() != ready.expected_version()
                        || opportunity.state() != world_model::ActionOpportunityState::Open
                    {
                        return Err(RuntimeDriveError::Integrity);
                    }
                    PreparedDelivery::action_ready(*key, *ready, opportunity.clone())
                }
                ScheduledWork::ActionEvaluation(evaluation) => {
                    let opportunity = self
                        .head
                        .runtime_control()
                        .action_opportunities()
                        .get(evaluation.opportunity())
                        .ok_or(RuntimeDriveError::Integrity)?;
                    if opportunity.version() != evaluation.expected_waiting_version()
                        || opportunity.state()
                            != world_model::ActionOpportunityState::WaitingForEvaluation(
                                evaluation.invocation(),
                            )
                    {
                        return Err(RuntimeDriveError::Integrity);
                    }
                    let invocation = self
                        .head
                        .runtime_control()
                        .action_evaluations()
                        .get(evaluation.invocation())
                        .ok_or(RuntimeDriveError::Integrity)?;
                    PreparedDelivery::action_evaluation(
                        *key,
                        *evaluation,
                        opportunity.clone(),
                        invocation.clone(),
                    )
                }
                ScheduledWork::Lifecycle(LifecycleWork::EvidenceDelivery(delivery)) => {
                    PreparedDelivery::evidence_delivery(*key, *delivery)
                }
                ScheduledWork::Lifecycle(LifecycleWork::Appraisal(appraisal)) => {
                    let causes = self
                        .head
                        .runtime_control()
                        .lifecycle()
                        .get(
                            appraisal.actor(),
                            crate::lifecycle::LifecycleRole::Appraisal,
                        )
                        .and_then(|record| record.enqueued_causes(appraisal.generation()))
                        .ok_or(RuntimeDriveError::Integrity)?;
                    let mut evidence = Vec::with_capacity(causes.len());
                    let mut previous = Vec::new();
                    for cause in causes {
                        let crate::lifecycle::LifecycleCause::Evidence(evidence_id) = cause else {
                            return Err(RuntimeDriveError::Integrity);
                        };
                        let record = self
                            .head
                            .accepted()
                            .epistemic()
                            .evidence_record(evidence_id)
                            .copied()
                            .ok_or(RuntimeDriveError::Integrity)?;
                        if let Some(item) = record.provenance().containment_item()
                            && let Some(retained) = self
                                .head
                                .runtime_control()
                                .appraisals()
                                .get(appraisal.actor(), item)
                            && !previous.contains(&retained)
                        {
                            previous.push(retained);
                        }
                        evidence.push(record);
                    }
                    PreparedDelivery::appraisal(*key, *appraisal, evidence, previous)
                }
                ScheduledWork::Lifecycle(LifecycleWork::IntentReview(review)) => {
                    let causes = self
                        .head
                        .runtime_control()
                        .lifecycle()
                        .get(
                            review.actor(),
                            crate::lifecycle::LifecycleRole::IntentReview,
                        )
                        .and_then(|record| record.enqueued_causes(review.generation()))
                        .ok_or(RuntimeDriveError::Integrity)?;
                    let mut appraisals = Vec::with_capacity(causes.len());
                    for cause in causes {
                        let crate::lifecycle::LifecycleCause::Appraisal { material, .. } = cause
                        else {
                            return Err(RuntimeDriveError::Integrity);
                        };
                        let appraisal = self
                            .head
                            .runtime_control()
                            .appraisals()
                            .find_material(review.actor(), material)
                            .ok_or(RuntimeDriveError::Integrity)?;
                        if !appraisals.contains(&appraisal) {
                            appraisals.push(appraisal);
                        }
                    }
                    PreparedDelivery::intent_review(*key, *review, appraisals)
                }
                ScheduledWork::Lifecycle(LifecycleWork::ActivityInitialization(initialization)) => {
                    let causes = self
                        .head
                        .runtime_control()
                        .lifecycle()
                        .get(
                            initialization.actor(),
                            crate::lifecycle::LifecycleRole::ActivityInitialization,
                        )
                        .and_then(|record| record.enqueued_causes(initialization.generation()))
                        .ok_or(RuntimeDriveError::Integrity)?;
                    let mut intents = Vec::with_capacity(causes.len());
                    for cause in causes {
                        let crate::lifecycle::LifecycleCause::Intent { intent, version } = cause
                        else {
                            return Err(RuntimeDriveError::Integrity);
                        };
                        let accepted = self
                            .head
                            .accepted()
                            .agency()
                            .intent(intent)
                            .copied()
                            .filter(|intent| intent.version() == version)
                            .ok_or(RuntimeDriveError::Integrity)?;
                        if !intents.contains(&accepted) {
                            intents.push(accepted);
                        }
                    }
                    PreparedDelivery::activity_initialization(*key, *initialization, intents)
                }
                ScheduledWork::Lifecycle(LifecycleWork::AttemptResolved(resolved)) => {
                    let opportunity = self
                        .head
                        .runtime_control()
                        .action_opportunities()
                        .get(resolved.opportunity())
                        .cloned()
                        .ok_or(RuntimeDriveError::Integrity)?;
                    PreparedDelivery::attempt_resolved(*key, *resolved, opportunity)
                }
                ScheduledWork::Lifecycle(LifecycleWork::ActivityAdvance(advance)) => {
                    let causes = self
                        .head
                        .runtime_control()
                        .lifecycle()
                        .get(
                            advance.actor(),
                            crate::lifecycle::LifecycleRole::ActivityAdvance,
                        )
                        .and_then(|record| record.enqueued_causes(advance.generation()))
                        .ok_or(RuntimeDriveError::Integrity)?;
                    let mut attempted = Vec::new();
                    for cause in causes {
                        let crate::lifecycle::LifecycleCause::AttemptResolved(opportunity) = cause
                        else {
                            return Err(RuntimeDriveError::Integrity);
                        };
                        let consumed = self
                            .head
                            .runtime_control()
                            .action_opportunities()
                            .get(opportunity)
                            .cloned()
                            .ok_or(RuntimeDriveError::Integrity)?;
                        if !attempted.contains(&consumed) {
                            attempted.push(consumed);
                        }
                    }
                    PreparedDelivery::activity_advance(*key, *advance, Vec::new(), attempted)
                }
                ScheduledWork::Command(scheduled) => {
                    let command = scheduled.command();
                    let resolution = match self.head.runtime_control().command().classify(
                        command.source(),
                        command.id(),
                        command.fingerprint(),
                    ) {
                        CommandLedgerLookup::Absent => {
                            let fingerprints = absent_fingerprints
                                .get(&(command.source(), command.id()))
                                .ok_or(RuntimeDriveError::Integrity)?;
                            if fingerprints.len() == 1 {
                                deliveries.push(PreparedDelivery::evaluable_command(
                                    *key,
                                    scheduled.as_ref().clone(),
                                ));
                                continue;
                            }
                            PreparedCommandResolution::NewCollision
                        }
                        CommandLedgerLookup::Retired => PreparedCommandResolution::Retired,
                        CommandLedgerLookup::RetainedExact {
                            original_attempt,
                            outcome,
                        } => PreparedCommandResolution::Retained {
                            original_attempt,
                            outcome,
                        },
                        CommandLedgerLookup::RetainedCollision {
                            original_attempt, ..
                        } => PreparedCommandResolution::RetainedCollision { original_attempt },
                        CommandLedgerLookup::IdReuseMismatch { original_attempt } => {
                            PreparedCommandResolution::IdReuseMismatch { original_attempt }
                        }
                    };
                    PreparedDelivery::resolved_command(*key, scheduled.as_ref().clone(), resolution)
                }
            };
            deliveries.push(delivery);
        }

        let operation = ReservedOperationDescriptor::fire(moment, resulting_frontier, &due_keys)
            .map_err(|_| RuntimeDriveError::Integrity)?;
        let snapshot = self.head.snapshot();
        let execution = self.control.closure().specification().id();
        reserve(self, operation)?;
        let reservation = self
            .control
            .reservation()
            .ok_or(RuntimeDriveError::Integrity)?;
        let step = reservation.step();
        let prepared = PreparedFire::new(
            domain,
            attempt,
            execution,
            step,
            reservation.grant(),
            resulting_frontier,
            snapshot,
            deliveries,
        );
        match prepared {
            Ok(prepared) => Ok(FirePreparation::Ready(prepared)),
            Err(_) => {
                release_unpublished(self, step)?;
                Err(RuntimeDriveError::Integrity)
            }
        }
    }

    pub(super) fn verify_prepared_target(
        domain: AttemptAuthorityDomainId,
        attempt: RunAttemptId,
        prepared: &PreparedFire,
    ) -> Result<(), RuntimeDriveError> {
        if prepared.domain() == domain && prepared.attempt() == attempt {
            Ok(())
        } else {
            Err(RuntimeDriveError::PreparedFireMismatch)
        }
    }

    pub(super) fn verify_prepared_safety_target(
        domain: AttemptAuthorityDomainId,
        attempt: RunAttemptId,
        prepared: &PreparedKernelSafety,
    ) -> Result<(), RuntimeDriveError> {
        if prepared.domain() == domain && prepared.attempt() == attempt {
            Ok(())
        } else {
            Err(RuntimeDriveError::PreparedKernelSafetyMismatch)
        }
    }

    pub(super) fn complete_kernel_safety(
        &mut self,
        prepared: PreparedKernelSafety,
    ) -> Result<KernelSafetyOutcome, RuntimeDriveError> {
        verify_prepared_safety(self, &prepared)?;
        let cause = prepared.cause();
        let sealed = seal_authority_record(
            &self.head,
            self.control.closure(),
            DraftAuthorityRecord::kernel_safety(self.head.cursor(), cause),
        )
        .map_err(|_| RuntimeDriveError::Integrity)?;
        let (record, cursor) = append_and_publish(self, sealed)?;
        reconcile(self).map_err(|_| RuntimeDriveError::Integrity)?;
        Ok(KernelSafetyOutcome::published(record, cursor, cause))
    }

    pub(super) fn complete_fire(
        &mut self,
        prepared: PreparedFire,
        proposals: MomentWorkProposals,
    ) -> Result<FireOutcome, RuntimeDriveError> {
        verify_prepared(self, &prepared)?;
        if prepared.validate_proposals(&proposals).is_err() {
            retain_completion_failure(self)?;
            return Err(RuntimeDriveError::ProposalMismatch);
        }

        let mut seen_work = BTreeSet::new();
        let mut candidates = Vec::new();
        for (position, delivery) in prepared.deliveries().iter().enumerate() {
            let PreparedDelivery::EvaluableCommand { scheduled, .. } = delivery else {
                continue;
            };
            let work = prepared
                .work_id_for_delivery(position)
                .ok_or(RuntimeDriveError::Integrity)?;
            if !seen_work.insert(work) {
                continue;
            }
            let proposal = match proposals.proposal(work) {
                Some(WorkProposal::Command(CommandProposal::Rejected(reason))) => {
                    ContainmentCandidateProposal::Rejected(*reason)
                }
                Some(WorkProposal::Command(CommandProposal::AcceptedTransfer(delta))) => {
                    ContainmentCandidateProposal::Transfer(*delta)
                }
                Some(
                    WorkProposal::PostCommit(_)
                    | WorkProposal::EvidenceAssimilation { .. }
                    | WorkProposal::Appraisal(_)
                    | WorkProposal::IntentReview(_)
                    | WorkProposal::ActivityInitialization(_)
                    | WorkProposal::Action(_)
                    | WorkProposal::ActionEvaluation(_)
                    | WorkProposal::AttemptResolvedConsumed
                    | WorkProposal::ActivityAdvance(_)
                    | WorkProposal::RelocationProcessCompleted,
                )
                | None => {
                    retain_completion_failure(self)?;
                    return Err(RuntimeDriveError::ProposalMismatch);
                }
            };
            candidates.push(ContainmentCandidate::new(
                ContainmentCommandIdentity::from_command(scheduled.command()),
                scheduled.command().actor(),
                proposal,
            ));
        }
        let candidates = match ContainmentCandidateSet::new(candidates) {
            Ok(candidates) => candidates,
            Err(_) => {
                retain_completion_failure(self)?;
                return Err(RuntimeDriveError::Integrity);
            }
        };
        let oracle =
            Blake3KeyedPrf256V1::from_root_seed(self.control.closure().specification().root_seed());
        let resolution = resolve_containment_candidates(
            prepared.moment(),
            self.head.accepted(),
            &candidates,
            &oracle,
        );
        let ProjectedFireOutcome {
            command_resolutions,
            post_commit_consumed,
            action_opportunities_consumed,
            attempt_resolved,
        } = match project_fire_outcome(&prepared, &proposals, &resolution) {
            Ok(outcome) => outcome,
            Err(()) => {
                retain_completion_failure(self)?;
                return Err(RuntimeDriveError::Integrity);
            }
        };
        let draft = match DraftMomentBatch::from_prepared(&prepared, &proposals, &resolution) {
            Ok(draft) => draft,
            Err(_) => {
                retain_completion_failure(self)?;
                return Err(RuntimeDriveError::ProposalMismatch);
            }
        };

        let sealed = match seal_authority_record(
            &self.head,
            self.control.closure(),
            DraftAuthorityRecord::moment(self.head.cursor(), draft),
        ) {
            Ok(sealed) => sealed,
            Err(error) => {
                let error = map_fire_seal_error(error);
                retain_completion_failure(self)?;
                return Err(error);
            }
        };
        let (record, cursor) = match append_and_publish(self, sealed) {
            Ok(publication) => publication,
            Err(error) => {
                retain_completion_failure(self)?;
                return Err(error);
            }
        };
        reconcile(self).map_err(|_| RuntimeDriveError::Integrity)?;
        Ok(FireOutcome::published(
            record,
            cursor,
            prepared.moment(),
            command_resolutions,
            post_commit_consumed,
            action_opportunities_consumed,
            attempt_resolved,
        ))
    }

    pub(super) fn verify_prepared_control_target(
        domain: AttemptAuthorityDomainId,
        attempt: RunAttemptId,
        prepared: &PreparedFire,
    ) -> Result<(), RuntimeControlError> {
        if prepared.domain() == domain && prepared.attempt() == attempt {
            Ok(())
        } else {
            Err(RuntimeControlError::PreparedFireMismatch)
        }
    }

    pub(super) fn fail_prepared_fire(
        &mut self,
        prepared: PreparedFire,
        failure: PreparedFireFailure,
    ) -> Result<PreparedFireFailureOutcome, RuntimeControlError> {
        verify_prepared_control(self, &prepared)?;
        retain_prepared_failure(self, failure)
    }

    pub(super) fn cancel_attempt(
        &mut self,
        request: CancelAttemptRequest,
    ) -> Result<CancelAttemptOutcome, RuntimeControlError> {
        let bound = request.bind(self.control.binding());
        match self.control.classify_cancellation(bound) {
            CancellationLookup::RetainedExact(outcome) => return Ok(*outcome),
            CancellationLookup::IdReuseMismatch => {
                return Err(RuntimeControlError::CancellationIdReuse { id: request.id() });
            }
            CancellationLookup::Absent => {}
        }

        let cursor = match self.control.phase() {
            AttemptPhase::Active(cursor) if *cursor == self.head.cursor() => *cursor,
            AttemptPhase::Active(_) => return Err(RuntimeControlError::Integrity),
            AttemptPhase::Reserved(_) => return Err(RuntimeControlError::StepReserved),
            AttemptPhase::Finalized(finalization) => {
                return Err(RuntimeControlError::AttemptFinalized {
                    finalization: *finalization,
                });
            }
        };
        let disposition = bound.into_disposition();
        let finalization = self
            .control
            .project_finalization(&self.head, Some(disposition))
            .map_err(|_| RuntimeControlError::Integrity)?
            .ok_or(RuntimeControlError::Integrity)?;
        self.dispositions
            .retain(disposition)
            .map_err(|_| RuntimeControlError::Integrity)?;
        let outcome = CancelAttemptOutcome::cancelled(finalization);
        self.control
            .retain_cancellation(bound, outcome)
            .map_err(|_| RuntimeControlError::Integrity)?;
        self.control
            .finalize_active(cursor, finalization)
            .map_err(|_| RuntimeControlError::Integrity)?;
        Ok(outcome)
    }
}

struct ProjectedFireOutcome {
    command_resolutions: Vec<CommandFireResolution>,
    post_commit_consumed: usize,
    action_opportunities_consumed: Vec<world_model::ActionOpportunityId>,
    attempt_resolved: Vec<world_model::ActionOpportunityId>,
}

fn project_fire_outcome(
    prepared: &PreparedFire,
    proposals: &MomentWorkProposals,
    resolution: &crate::kernel::ContainmentMomentResolution,
) -> Result<ProjectedFireOutcome, ()> {
    let outcomes = resolution
        .outcomes()
        .iter()
        .map(|resolved| {
            let outcome = match resolved.outcome() {
                ContainmentCandidateOutcome::Accepted { .. } => {
                    world_model::CommandAttemptOutcome::Accepted
                }
                ContainmentCandidateOutcome::Rejected(reason) => {
                    world_model::CommandAttemptOutcome::Rejected(*reason)
                }
            };
            (resolved.identity(), outcome)
        })
        .collect::<BTreeMap<_, _>>();

    let mut command_resolutions = Vec::new();
    let mut post_commit_consumed = 0usize;
    let mut action_opportunities_consumed = Vec::new();
    let mut attempt_resolved = Vec::new();
    for (position, delivery) in prepared.deliveries().iter().enumerate() {
        match delivery {
            PreparedDelivery::PostCommit { .. } => {
                post_commit_consumed = post_commit_consumed.checked_add(1).ok_or(())?;
            }
            PreparedDelivery::EvaluableCommand { scheduled, .. } => {
                let command = scheduled.command();
                let outcome = outcomes
                    .get(&ContainmentCommandIdentity::from_command(command))
                    .copied()
                    .ok_or(())?;
                if scheduled.action_opportunity().is_none() {
                    command_resolutions.push(CommandFireResolution::new(
                        command.source(),
                        command.id(),
                        CommandFireClassification::New(outcome),
                    ));
                }
            }
            PreparedDelivery::ResolvedCommand {
                scheduled,
                resolution,
                ..
            } => {
                let command = scheduled.command();
                let classification = match resolution {
                    PreparedCommandResolution::Retained { outcome, .. } => {
                        CommandFireClassification::Retained(*outcome)
                    }
                    PreparedCommandResolution::IdReuseMismatch { .. } => {
                        CommandFireClassification::IdReuseMismatch
                    }
                    PreparedCommandResolution::NewCollision
                    | PreparedCommandResolution::RetainedCollision { .. } => {
                        CommandFireClassification::IdCollision
                    }
                    PreparedCommandResolution::Retired => CommandFireClassification::Retired,
                };
                if scheduled.action_opportunity().is_none() {
                    command_resolutions.push(CommandFireResolution::new(
                        command.source(),
                        command.id(),
                        classification,
                    ));
                }
            }
            PreparedDelivery::ActionReady { opportunity, .. } => {
                let work = prepared.work_id_for_delivery(position).ok_or(())?;
                match proposals.proposal(work) {
                    Some(WorkProposal::Action(ActionProposal::BeginDeferred { .. })) => {}
                    Some(WorkProposal::Action(_)) => {
                        action_opportunities_consumed.push(opportunity.id());
                    }
                    _ => return Err(()),
                }
            }
            PreparedDelivery::ActionEvaluation {
                evaluation,
                opportunity,
                ..
            } => match evaluation {
                crate::action_evaluation::ActionEvaluationWork::ResultReady { .. } => {
                    let work = prepared.work_id_for_delivery(position).ok_or(())?;
                    match proposals.proposal(work) {
                        Some(WorkProposal::ActionEvaluation(ActionEvaluationDecision::Apply {
                            ..
                        })) => action_opportunities_consumed.push(opportunity.id()),
                        Some(WorkProposal::ActionEvaluation(
                            ActionEvaluationDecision::Reinvoke(_)
                            | ActionEvaluationDecision::RequireFallback(_),
                        )) => {}
                        _ => return Err(()),
                    }
                }
                crate::action_evaluation::ActionEvaluationWork::Fallback { .. } => {
                    if prepared.work_id_for_delivery(position).is_some() {
                        return Err(());
                    }
                    action_opportunities_consumed.push(opportunity.id());
                }
            },
            PreparedDelivery::AttemptResolved { resolved, .. } => {
                attempt_resolved.push(resolved.opportunity());
            }
            PreparedDelivery::EvidenceDelivery { .. }
            | PreparedDelivery::Appraisal { .. }
            | PreparedDelivery::IntentReview { .. }
            | PreparedDelivery::ActivityInitialization { .. }
            | PreparedDelivery::ActivityAdvance { .. }
            | PreparedDelivery::Process { .. } => {}
        }
    }
    Ok(ProjectedFireOutcome {
        command_resolutions,
        post_commit_consumed,
        action_opportunities_consumed,
        attempt_resolved,
    })
}

#[allow(
    clippy::result_large_err,
    reason = "the runtime error retains complete terminal replay evidence"
)]
fn retain_completion_failure(aggregate: &mut AttemptAggregate) -> Result<(), RuntimeDriveError> {
    retain_prepared_failure(aggregate, PreparedFireFailure::EngineFailure)
        .map(|_| ())
        .map_err(|_| RuntimeDriveError::Integrity)
}

#[allow(
    clippy::result_large_err,
    reason = "the runtime error retains complete terminal replay evidence"
)]
fn retain_prepared_failure(
    aggregate: &mut AttemptAggregate,
    failure: PreparedFireFailure,
) -> Result<PreparedFireFailureOutcome, RuntimeControlError> {
    let disposition = match failure {
        PreparedFireFailure::HostBudgetExceeded => AttemptDisposition::HostBudgetExceeded,
        PreparedFireFailure::ExternalFailure => AttemptDisposition::ExternalFailure,
        PreparedFireFailure::EngineFailure => AttemptDisposition::EngineFailure,
    };
    let disposition_id = aggregate
        .dispositions
        .retain(disposition)
        .map_err(|_| RuntimeControlError::Integrity)?;
    aggregate
        .control
        .reservation_mut()
        .ok_or(RuntimeControlError::PreparedFireMismatch)?
        .attach_failure(disposition_id)
        .map_err(|_| RuntimeControlError::Integrity)?;
    reconcile(aggregate).map_err(|_| RuntimeControlError::Integrity)?;
    match aggregate.control.phase() {
        AttemptPhase::Finalized(finalization) => {
            Ok(PreparedFireFailureOutcome::finalized(*finalization))
        }
        AttemptPhase::Active(_) | AttemptPhase::Reserved(_) => Err(RuntimeControlError::Integrity),
    }
}

#[allow(
    clippy::result_large_err,
    reason = "the runtime error retains complete terminal replay evidence"
)]
fn reserve(
    aggregate: &mut AttemptAggregate,
    operation: ReservedOperationDescriptor,
) -> Result<(), RuntimeDriveError> {
    aggregate
        .control
        .reserve(aggregate.head.cursor(), operation)
        .map_err(map_phase_drive_error)
}

#[allow(
    clippy::result_large_err,
    reason = "the runtime error retains complete terminal replay evidence"
)]
fn require_active(aggregate: &AttemptAggregate) -> Result<(), RuntimeDriveError> {
    match aggregate.control.phase() {
        AttemptPhase::Active(cursor) if *cursor == aggregate.head.cursor() => Ok(()),
        AttemptPhase::Active(_) => Err(RuntimeDriveError::Integrity),
        AttemptPhase::Reserved(_) => Err(RuntimeDriveError::StepReserved),
        AttemptPhase::Finalized(finalization) => Err(RuntimeDriveError::AttemptFinalized {
            finalization: *finalization,
        }),
    }
}

#[allow(
    clippy::result_large_err,
    reason = "the runtime error retains complete terminal replay evidence"
)]
fn reserved_step(aggregate: &AttemptAggregate) -> Result<AttemptStepId, RuntimeDriveError> {
    aggregate
        .control
        .reservation()
        .map(|reservation| reservation.step())
        .ok_or(RuntimeDriveError::Integrity)
}

#[allow(
    clippy::result_large_err,
    reason = "the runtime error retains complete terminal replay evidence"
)]
fn release_unpublished(
    aggregate: &mut AttemptAggregate,
    step: AttemptStepId,
) -> Result<(), RuntimeDriveError> {
    aggregate
        .control
        .activate_reserved(step, aggregate.head.cursor())
        .map_err(map_phase_drive_error)
}

#[allow(
    clippy::result_large_err,
    reason = "the runtime error retains complete terminal replay evidence"
)]
fn publish_and_reconcile(
    aggregate: &mut AttemptAggregate,
    sealed: SealedAuthorityRecord,
) -> Result<
    (
        crate::authority::AuthorityRecordId,
        crate::authority::AuthorityCursor,
    ),
    RuntimeDriveError,
> {
    let (record, cursor) = append_and_publish(aggregate, sealed)?;
    reconcile(aggregate).map_err(|_| RuntimeDriveError::Integrity)?;
    Ok((record, cursor))
}

#[allow(
    clippy::result_large_err,
    reason = "the runtime error retains complete terminal replay evidence"
)]
pub(super) fn append_and_publish(
    aggregate: &mut AttemptAggregate,
    sealed: SealedAuthorityRecord,
) -> Result<
    (
        crate::authority::AuthorityRecordId,
        crate::authority::AuthorityCursor,
    ),
    RuntimeDriveError,
> {
    let reservation = aggregate
        .control
        .reservation()
        .ok_or(RuntimeDriveError::Integrity)?;
    if !history_matches_head(aggregate)
        || reservation.expected() != aggregate.head.cursor()
        || sealed.expected_cursor() != reservation.expected()
        || !operation_matches_record(reservation.operation(), sealed.record().body())
        || aggregate.receipts.contains_key(&reservation.step())
    {
        return Err(RuntimeDriveError::Integrity);
    }

    let publication = apply_authority_record(&aggregate.head, sealed).map_err(map_apply_error)?;
    let receipt = StepPublicationReceipt::from_publication(reservation, &publication);
    let record = publication.record().header().id();
    let cursor = publication.resulting_head().cursor();
    let (head, authority_record) = publication.into_parts();
    aggregate.head = head;
    aggregate.history.push(authority_record);
    aggregate.receipts.insert(receipt.step(), receipt);
    Ok((record, cursor))
}

pub(super) fn reconcile(aggregate: &mut AttemptAggregate) -> Result<(), ReconciliationError> {
    let Some(reservation) = aggregate.control.reservation() else {
        return Ok(());
    };
    if !history_matches_head(aggregate) {
        return Err(ReconciliationError);
    }
    let step = reservation.step();
    let expected = reservation.expected();
    let disposition = reservation.disposition();
    let head_cursor = aggregate.head.cursor();

    if head_cursor == expected {
        if aggregate.receipts.contains_key(&step) {
            return Err(ReconciliationError);
        }
        match disposition {
            None => aggregate
                .control
                .activate_reserved(step, expected)
                .map_err(|_| ReconciliationError),
            Some(disposition_id) => {
                let disposition = aggregate
                    .dispositions
                    .get(disposition_id)
                    .ok_or(ReconciliationError)?;
                let finalization = aggregate
                    .control
                    .project_finalization(&aggregate.head, Some(disposition))
                    .map_err(|_| ReconciliationError)?
                    .ok_or(ReconciliationError)?;
                aggregate
                    .control
                    .finalize_reserved(step, finalization)
                    .map_err(|_| ReconciliationError)
            }
        }
    } else {
        let receipt = aggregate.receipts.get(&step).ok_or(ReconciliationError)?;
        if receipt.binding() != aggregate.control.binding()
            || receipt.step() != step
            || receipt.operation_fingerprint() != reservation.operation_fingerprint()
            || receipt.expected() != expected
            || receipt.resulting() != head_cursor
        {
            return Err(ReconciliationError);
        }
        let plan = expected.successor_plan().map_err(|_| ReconciliationError)?;
        let expected_successor = plan.finish(receipt.record(), head_cursor.cumulative());
        if expected_successor != head_cursor {
            return Err(ReconciliationError);
        }
        let retained_disposition = match disposition {
            None => None,
            Some(id) => Some(aggregate.dispositions.get(id).ok_or(ReconciliationError)?),
        };
        match aggregate
            .control
            .project_finalization(&aggregate.head, retained_disposition)
            .map_err(|_| ReconciliationError)?
        {
            Some(finalization) => aggregate
                .control
                .finalize_reserved(step, finalization)
                .map_err(|_| ReconciliationError),
            None => aggregate
                .control
                .activate_reserved(step, head_cursor)
                .map_err(|_| ReconciliationError),
        }
    }
}

fn history_matches_head(aggregate: &AttemptAggregate) -> bool {
    match aggregate.head.cursor().position() {
        AuthorityPosition::Root { .. } => aggregate.history.is_empty(),
        AuthorityPosition::Record {
            revision,
            sequence,
            record,
            cumulative,
        } => {
            u64::try_from(aggregate.history.len()) == Ok(sequence.get())
                && aggregate.history.last().is_some_and(|entry| {
                    let header = entry.header();
                    header.lineage() == aggregate.head.cursor().epoch().lineage()
                        && header.revision() == revision
                        && header.sequence() == sequence
                        && header.id() == record
                        && header.cumulative() == cumulative
                })
        }
    }
}

#[allow(
    clippy::result_large_err,
    reason = "the runtime error retains complete terminal replay evidence"
)]
fn verify_prepared(
    aggregate: &AttemptAggregate,
    prepared: &PreparedFire,
) -> Result<(), RuntimeDriveError> {
    verify_prepared_inner(aggregate, prepared).map_err(|_| RuntimeDriveError::PreparedFireMismatch)
}

#[allow(
    clippy::result_large_err,
    reason = "the runtime error retains complete terminal replay evidence"
)]
fn verify_prepared_safety(
    aggregate: &AttemptAggregate,
    prepared: &PreparedKernelSafety,
) -> Result<(), RuntimeDriveError> {
    let reservation = aggregate
        .control
        .reservation()
        .ok_or(RuntimeDriveError::PreparedKernelSafetyMismatch)?;
    let current_cause = current_kernel_safety_cause(aggregate)
        .ok_or(RuntimeDriveError::PreparedKernelSafetyMismatch)?;
    if reservation.step() != prepared.step()
        || reservation.grant() != prepared.grant()
        || reservation.binding().attempt() != prepared.attempt()
        || reservation.expected() != aggregate.head.cursor()
        || prepared.expected() != aggregate.head.cursor()
        || prepared.execution() != aggregate.control.closure().specification().id()
        || reservation.operation() != ReservedOperationDescriptor::kernel_safety(prepared.cause())
        || prepared.cause() != current_cause
    {
        return Err(RuntimeDriveError::PreparedKernelSafetyMismatch);
    }
    Ok(())
}

fn current_kernel_safety_cause(aggregate: &AttemptAggregate) -> Option<KernelSafetyCause> {
    if aggregate.head.mode() != crate::session::SessionMode::Running {
        return None;
    }
    let due = aggregate.head.scheduler().clone_least_due()?;
    let mut absent_fingerprints =
        BTreeMap::<(CommandSource, CommandId), BTreeSet<CommandRequestFingerprint>>::new();
    for (_, work) in due.entries() {
        let ScheduledWork::Command(scheduled) = work else {
            continue;
        };
        let command = scheduled.command();
        if matches!(
            aggregate.head.runtime_control().command().classify(
                command.source(),
                command.id(),
                command.fingerprint(),
            ),
            CommandLedgerLookup::Absent
        ) {
            absent_fingerprints
                .entry((command.source(), command.id()))
                .or_default()
                .insert(command.fingerprint());
        }
    }
    let due_keys = due
        .entries()
        .iter()
        .map(|(key, _)| *key)
        .collect::<Vec<_>>();
    let evaluable_commands = u64::try_from(
        absent_fingerprints
            .values()
            .filter(|fingerprints| fingerprints.len() == 1)
            .count(),
    )
    .ok()?;
    select_kernel_safety_cause(
        aggregate.control.closure().semantics().config(),
        aggregate.head.clock().frontier(),
        &due_keys,
        evaluable_commands,
        aggregate.head.clock().attempted_wave(due.moment()),
    )
    .ok()
    .flatten()
}

#[allow(
    clippy::result_large_err,
    reason = "the runtime error retains complete terminal replay evidence"
)]
fn verify_prepared_control(
    aggregate: &AttemptAggregate,
    prepared: &PreparedFire,
) -> Result<(), RuntimeControlError> {
    verify_prepared_inner(aggregate, prepared)
        .map_err(|_| RuntimeControlError::PreparedFireMismatch)
}

fn verify_prepared_inner(
    aggregate: &AttemptAggregate,
    prepared: &PreparedFire,
) -> Result<(), PreparedMismatch> {
    let reservation = aggregate.control.reservation().ok_or(PreparedMismatch)?;
    let due_keys = prepared
        .deliveries()
        .iter()
        .map(PreparedDelivery::key)
        .collect::<Vec<_>>();
    let expected_operation = ReservedOperationDescriptor::fire(
        prepared.moment(),
        prepared.resulting_frontier(),
        &due_keys,
    )
    .map_err(|_| PreparedMismatch)?;
    if reservation.step() != prepared.step()
        || reservation.grant() != prepared.grant()
        || reservation.binding().attempt() != prepared.attempt()
        || reservation.expected() != aggregate.head.cursor()
        || reservation.operation() != expected_operation
        || aggregate.head.scheduler().least_due_moment() != Some(prepared.moment())
        || aggregate.head.scheduler().entry_count_at(prepared.moment())
            != prepared.deliveries().len()
        || prepared.base_snapshot().revision() != aggregate.head.cursor().revision()
        || prepared.base_snapshot().accepted() != aggregate.head.accepted()
        || prepared.deliveries().iter().any(|delivery| {
            aggregate.head.scheduler().get(delivery.key()) != Some(&delivery.scheduled_work())
        })
    {
        return Err(PreparedMismatch);
    }
    Ok(())
}

fn operation_matches_record(
    operation: ReservedOperationDescriptor,
    body: &AuthorityRecordBody,
) -> bool {
    match (operation, body) {
        (
            ReservedOperationDescriptor::AdmitCommand {
                id,
                fingerprint,
                effective,
            },
            AuthorityRecordBody::Admission(AuthorityAdmissionRecord::Commands(batch)),
        ) => {
            matches!(
                batch.entries(),
                [entry]
                    if entry.captured().input() == id
                        && entry.captured().request_fingerprint() == fingerprint
                        && entry.captured().effective() == effective
            )
        }
        (
            ReservedOperationDescriptor::AdmitActionEvaluation {
                capture,
                fingerprint,
                invocation,
                effective,
            },
            AuthorityRecordBody::Admission(AuthorityAdmissionRecord::ActionEvaluation(admission)),
        ) => {
            admission.request().capture() == capture
                && admission.request().fingerprint() == fingerprint
                && admission.request().invocation() == invocation
                && admission.request().effective() == effective
        }
        (
            operation @ ReservedOperationDescriptor::Fire {
                fired,
                resulting_frontier,
                due_count,
                ..
            },
            AuthorityRecordBody::Moment(batch),
        ) => {
            batch.moment() == fired
                && batch.resulting_frontier() == resulting_frontier
                && usize::try_from(due_count.get()) == Ok(batch.consumed_keys().len())
                && ReservedOperationDescriptor::fire(
                    batch.moment(),
                    batch.resulting_frontier(),
                    batch.consumed_keys(),
                ) == Ok(operation)
        }
        (
            ReservedOperationDescriptor::Manage {
                id,
                fingerprint,
                operation,
            },
            AuthorityRecordBody::Management(batch),
        ) => {
            matches!(
                batch.entries(),
                [entry]
                    if entry.request() == id
                        && entry.fingerprint() == fingerprint
                        && entry.operation() == operation
            )
        }
        (
            ReservedOperationDescriptor::KernelSafety { cause },
            AuthorityRecordBody::Management(batch),
        ) => batch.kernel_safety_cause() == Some(cause),
        _ => false,
    }
}

fn map_phase_drive_error(error: AttemptPhaseError) -> RuntimeDriveError {
    match error {
        AttemptPhaseError::StepReserved => RuntimeDriveError::StepReserved,
        AttemptPhaseError::Finalized(finalization) => RuntimeDriveError::AttemptFinalized {
            finalization: *finalization,
        },
        AttemptPhaseError::NoReservation
        | AttemptPhaseError::ReservationMismatch
        | AttemptPhaseError::ReservationGrantExhausted
        | AttemptPhaseError::CursorMismatch { .. } => RuntimeDriveError::Integrity,
    }
}

fn map_admit_seal_error(error: AuthorityRecordSealError) -> RuntimeDriveError {
    match error {
        AuthorityRecordSealError::EffectiveMomentBeforeFrontier {
            effective,
            frontier,
        } => RuntimeDriveError::EffectiveMomentBeforeFrontier {
            effective,
            frontier,
        },
        AuthorityRecordSealError::WorkPopulationExceeded {
            moment,
            maximum,
            actual,
        } => RuntimeDriveError::MomentPopulationExceeded {
            moment,
            maximum,
            actual,
        },
        _ => RuntimeDriveError::Integrity,
    }
}

fn map_capture_request_error(
    error: ActionEvaluationCaptureRequestError,
) -> RuntimeActionEvaluationCaptureError {
    match error {
        ActionEvaluationCaptureRequestError::InvocationNotPending { invocation }
        | ActionEvaluationCaptureRequestError::InvocationNotDispatchable { invocation } => {
            RuntimeActionEvaluationCaptureError::LateInvocation { invocation }
        }
        ActionEvaluationCaptureRequestError::TimingMismatch {
            admission_mode,
            supplied,
        } => RuntimeActionEvaluationCaptureError::TimingModeMismatch {
            expected: admission_mode,
            supplied,
        },
        ActionEvaluationCaptureRequestError::EffectiveMomentNotAfterCreation {
            effective,
            creation,
        } => RuntimeActionEvaluationCaptureError::EffectiveMomentNotAfterCreation {
            effective,
            creation,
        },
        ActionEvaluationCaptureRequestError::EffectiveMomentBeforeFrontier {
            effective,
            frontier,
        } => RuntimeActionEvaluationCaptureError::EffectiveMomentBeforeFrontier {
            effective,
            frontier,
        },
        ActionEvaluationCaptureRequestError::ResultSchemaMismatch { expected, actual } => {
            RuntimeActionEvaluationCaptureError::ResultSchemaMismatch { expected, actual }
        }
        ActionEvaluationCaptureRequestError::InvocationMismatch { .. }
        | ActionEvaluationCaptureRequestError::MissingBlockingFrontier { .. }
        | ActionEvaluationCaptureRequestError::Artifact(_) => {
            RuntimeActionEvaluationCaptureError::Integrity
        }
    }
}

fn map_capture_seal_error(error: AuthorityRecordSealError) -> RuntimeActionEvaluationCaptureError {
    match error {
        AuthorityRecordSealError::WorkPopulationExceeded {
            moment,
            maximum,
            actual,
        } => RuntimeActionEvaluationCaptureError::MomentPopulationExceeded {
            moment,
            maximum,
            actual,
        },
        _ => RuntimeActionEvaluationCaptureError::Integrity,
    }
}

fn map_capture_drive_error(error: RuntimeDriveError) -> RuntimeActionEvaluationCaptureError {
    match error {
        RuntimeDriveError::AttemptNotFound => RuntimeActionEvaluationCaptureError::AttemptNotFound,
        RuntimeDriveError::AttemptFinalized { finalization } => {
            RuntimeActionEvaluationCaptureError::AttemptFinalized { finalization }
        }
        RuntimeDriveError::StepReserved => RuntimeActionEvaluationCaptureError::StepReserved,
        RuntimeDriveError::MomentPopulationExceeded {
            moment,
            maximum,
            actual,
        } => RuntimeActionEvaluationCaptureError::MomentPopulationExceeded {
            moment,
            maximum,
            actual,
        },
        RuntimeDriveError::EffectiveMomentBeforeFrontier {
            effective,
            frontier,
        } => RuntimeActionEvaluationCaptureError::EffectiveMomentBeforeFrontier {
            effective,
            frontier,
        },
        RuntimeDriveError::Unavailable => RuntimeActionEvaluationCaptureError::Unavailable,
        RuntimeDriveError::InputIdReuse
        | RuntimeDriveError::InputRetired { .. }
        | RuntimeDriveError::ManagementIdReuse
        | RuntimeDriveError::ManagementRetired { .. }
        | RuntimeDriveError::RetirementNotAdvancing { .. }
        | RuntimeDriveError::RetirementGap { .. }
        | RuntimeDriveError::ManagementRetirementTargetNotBeforeRequest { .. }
        | RuntimeDriveError::AdmissionFrontierNotAdvancing { .. }
        | RuntimeDriveError::AdmissionSealCrossesScheduledWork { .. }
        | RuntimeDriveError::ActionEvaluationFrontierBlocked { .. }
        | RuntimeDriveError::IllegalManagement { .. }
        | RuntimeDriveError::NoScheduledWork
        | RuntimeDriveError::NoWorkDue { .. }
        | RuntimeDriveError::PreparedFireMismatch
        | RuntimeDriveError::PreparedKernelSafetyMismatch
        | RuntimeDriveError::ProposalMismatch
        | RuntimeDriveError::SessionNotRunning { .. }
        | RuntimeDriveError::Integrity => RuntimeActionEvaluationCaptureError::Integrity,
    }
}

fn map_manage_seal_error(error: AuthorityRecordSealError) -> RuntimeDriveError {
    match error {
        AuthorityRecordSealError::IllegalManagementTransition { current, .. } => {
            RuntimeDriveError::IllegalManagement { current }
        }
        AuthorityRecordSealError::LedgerRetirement {
            retirement,
            error:
                LedgerRetirementError::NotAdvancing {
                    retired_through, ..
                },
        } => RuntimeDriveError::RetirementNotAdvancing {
            retirement,
            retired_through,
        },
        AuthorityRecordSealError::LedgerRetirement {
            retirement,
            error: LedgerRetirementError::Gap { missing },
        } => RuntimeDriveError::RetirementGap {
            retirement,
            missing,
        },
        AuthorityRecordSealError::LedgerRetirement {
            error: LedgerRetirementError::ManagementTargetNotBeforeRequest { target, request },
            ..
        } => RuntimeDriveError::ManagementRetirementTargetNotBeforeRequest { target, request },
        AuthorityRecordSealError::AdmissionFrontierNotAdvancing { current, requested } => {
            RuntimeDriveError::AdmissionFrontierNotAdvancing { current, requested }
        }
        AuthorityRecordSealError::AdmissionSealCrossesScheduledWork {
            requested,
            scheduled,
        } => RuntimeDriveError::AdmissionSealCrossesScheduledWork {
            requested,
            scheduled,
        },
        AuthorityRecordSealError::ActionEvaluationFrontierBlocked { blocked_at } => {
            RuntimeDriveError::ActionEvaluationFrontierBlocked { blocked_at }
        }
        _ => RuntimeDriveError::Integrity,
    }
}

fn map_fire_seal_error(error: AuthorityRecordSealError) -> RuntimeDriveError {
    match error {
        AuthorityRecordSealError::MomentRequiresRunning { current } => {
            RuntimeDriveError::SessionNotRunning { current }
        }
        AuthorityRecordSealError::CommandLedgerClassificationMismatch => {
            RuntimeDriveError::ProposalMismatch
        }
        AuthorityRecordSealError::ActionEvaluationFrontierBlocked { blocked_at } => {
            RuntimeDriveError::ActionEvaluationFrontierBlocked { blocked_at }
        }
        _ => RuntimeDriveError::Integrity,
    }
}

fn map_apply_error(_error: AuthorityRecordApplyError) -> RuntimeDriveError {
    RuntimeDriveError::Integrity
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ReconciliationError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PreparedMismatch;
