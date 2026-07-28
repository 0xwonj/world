use core::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use world_core::{ActorId, EntityId, SimMoment};
use world_model::{
    ActionEvaluationInvocationId, ActionOpportunity, ActionOpportunityDisposition,
    ActionOpportunityId, ActionOpportunityState, ActionSponsor, Activity, ActivityStatus,
    ActivityTransition, ActorLocation, CommandEnvelope, CommandId, CommandSource, CommandValue,
    ContainmentAppraisal, DomainStateError, EvidenceDeliveryGeneration, EvidenceProvenance, Intent,
    IntentStatus, IntentTransition, PhysicalEvent, RelocationInteraction, RelocationProcessError,
    RelocationProcessStatus, StableCommandRejection,
};

use crate::action_evaluation::{
    ActionEvaluationArtifactError, ActionEvaluationArtifactFailure, ActionEvaluationCaptureLookup,
    ActionEvaluationCapturePayload, ActionEvaluationCaptureRequest, ActionEvaluationFallbackCause,
    ActionEvaluationInvocationRecord, ActionEvaluationInvocationState,
    ActionEvaluationPrivateContinuationArtifact, ActionEvaluationPrivateReadWitnessArtifact,
    ActionEvaluationRequestArtifact, ActionEvaluationTerminal, ActionEvaluationWork,
};
use crate::control::{
    CommandLedgerLookup, LedgerRetirementError, LifecycleWakeRequestOutcome, RequestLedgerLookup,
};
use crate::execution::ResolvedExecutionClosureManifestV1;
use crate::kernel::{
    ActionEvaluationDecision, ActionEvaluationResultFailure, ActionProposal, ActivityAdvanceResult,
    ActivityInitializationResult, ContainmentCommandIdentity, ContainmentResolutionEvidence,
    ContainmentResolutionFallback, DeferredActionInvocationInput, EvaluatedAction,
    IntentReviewResult, KernelSafetyCause, KernelSafetyDisposition, derive_input_request_namespace,
    select_kernel_safety_cause,
};
use crate::lifecycle::{
    ActivityAdvanceWork, ActivityInitializationWork, AppraisalWork, EvidenceDeliveryWork,
    IntentReviewWork, LifecycleCause, LifecycleGeneration, LifecycleRole, LifecycleWork,
};
use crate::randomness::{
    Blake3KeyedPrf256V1, ContainmentConflictContenderV1, ContainmentRandomRankError,
};
use crate::relocation::{
    RelocationProcessLedger, RelocationProcessLedgerError, RelocationProcessWake,
    RelocationWakeClassification,
};
use crate::scheduler::{
    AttemptResolved, PostCommitScheduleError, PreparedPostCommitDispatch, PreparedScheduledCommand,
    ScheduledCommand, ScheduledCommandCause, ScheduledWork, SchedulerBatchPlanError,
    SchedulerInsertion, SchedulerKey, SchedulerPlanError, SchedulerProducerOrdinal, SchedulerState,
    strictly_later_moment,
};
#[cfg(test)]
use crate::session::SessionClock;
use crate::session::{SessionHead, SessionMode};

use super::{
    ActionEvaluationAdmissionRecord, ActionEvaluationDeliveryRecord, ActionEvaluationDeliveryRef,
    ActionEvaluationInvocationOpeningCause, ActionEvaluationInvocationOpeningRecord,
    ActionEvaluationInvocationTransitionCause, ActionEvaluationInvocationTransitionRecord,
    ActionEvaluationManagementRecord, ActionOpportunityOpeningRecord,
    ActionOpportunityTransitionRecord, ActionReadyDeliveryRecord, ActionReadyDeliveryRef,
    ActionResolutionDeliveryRef, ActivityStartRecord, ActivityTerminalTransitionRecord,
    ActivityTransitionRecord, AttemptLocalIndex, AttemptRecord, AttemptRecordId, AttemptRecordRef,
    AttemptResolvedDeliveryRecord, AttemptResolvedDeliveryRef, AttemptSubjectRecord,
    AuthorityAdmissionRecord, AuthorityCursor, AuthorityCursorAdvanceError, AuthorityRecord,
    AuthorityRecordBody, AuthorityRecordHeader, AuthorityRecordId, CapturedInputLocalIndex,
    CapturedInputRecord, CapturedInputRecordId, CommandDeliveryRecord, CommandDeliveryRef,
    CommitLocalIndex, CommitRecordId, ContainmentAppraisalTransitionRecord,
    ContainmentTransferCommitRecord, ContainmentTransitionError, CumulativeAuthorityHash,
    DeliveryResolutionRecord, DraftAttemptOutcome, DraftAttemptRecord, DraftAttemptSubject,
    DraftAuthorityAdmission, DraftAuthorityRecord, DraftAuthorityRecordBody,
    DraftDeliveryResolution, DraftMomentBatch, DraftMomentDelivery, EvidenceAssimilationRecord,
    EvidenceRoutingRecord, EvidenceRoutingSource, IngressBatchRecord, IngressRecord,
    IntentAdoptionRecord, IntentTransitionRecord, LifecycleControlMutationRecord,
    LifecycleDeliveryRecord, LifecycleDeliveryRef, ManagementBatchRecord, ManagementRecord,
    MomentBatchRecord, MomentCommitRef, MomentReactionRef, NormalizedActionEvaluationAdmission,
    NormalizedActionEvaluationManagement, NormalizedAttemptRecord, NormalizedAttemptResolution,
    NormalizedAttemptSubject, NormalizedAuthorityAdmission, NormalizedAuthorityRecordBody,
    NormalizedDeliveryResolution, NormalizedIngressRecord, NormalizedManagementCause,
    NormalizedManagementRecord, NormalizedMomentBatch, NormalizedSchedulerInsertion,
    PostCommitDeliveryRecord, PostCommitDeliveryRef, ReactionEnvelopeId, ReactionEnvelopeRecord,
    ReactionLocalIndex, RecordedCommandResolution, RelocationAttemptRecord,
    RelocationAttemptRejection, RelocationAttemptResolution, RelocationPositionTransitionError,
    RelocationProcessDeliveryRecord, RelocationProcessDeliveryRef,
    RelocationProcessTransitionCause, RelocationProcessTransitionRecord, SchedulerInsertionRecord,
    SchedulerRemovalRecord, apply_containment_transfers, apply_relocation_arrival,
    apply_relocation_departure, authority_record_preimage, cumulative_authority_preimage,
};

/// Why a private record draft could not be sealed against an exact head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AuthorityRecordSealError {
    StaleDraft {
        expected: Box<AuthorityCursor>,
        actual: Box<AuthorityCursor>,
    },
    CursorAdvance(AuthorityCursorAdvanceError),
    InputRequestAlreadyRetained,
    CommandDefinitionSetMismatch {
        expected: world_defs::RuntimeDefinitionSetDigest,
        actual: world_defs::RuntimeDefinitionSetDigest,
    },
    ManagementRequestAlreadyRetained,
    LedgerRetirement {
        retirement: crate::kernel::LedgerRetirement,
        error: LedgerRetirementError,
    },
    AdmissionFrontierNotAdvancing {
        current: SimMoment,
        requested: SimMoment,
    },
    AdmissionSealCrossesScheduledWork {
        requested: SimMoment,
        scheduled: SimMoment,
    },
    ActionEvaluationFrontierBlocked {
        blocked_at: SimMoment,
    },
    EffectiveMomentBeforeFrontier {
        effective: SimMoment,
        frontier: SimMoment,
    },
    SchedulerSequenceExhausted,
    SchedulerKeyOccupied,
    EmptyBatch,
    DuplicateInputRequest,
    DuplicateManagementRequest,
    CollectionTooLarge,
    IncompleteDueSet {
        expected: usize,
        supplied: usize,
    },
    ScheduledWorkPayloadMismatch {
        key: SchedulerKey,
    },
    WorkPopulationExceeded {
        moment: SimMoment,
        maximum: u32,
        actual: usize,
    },
    ResolutionEvidenceMismatch,
    InvalidNormalizedGraph,
    IllegalManagementTransition {
        current: SessionMode,
        requested: SessionMode,
    },
    ScheduledWorkMissing,
    ScheduledWorkBeforeClock {
        scheduled: SimMoment,
        now: SimMoment,
    },
    MomentRequiresRunning {
        current: SessionMode,
    },
    KernelSafetyRequiresRunning {
        current: SessionMode,
    },
    KernelSafetyCauseMismatch,
    ScheduledWorkNotLeast {
        least: SchedulerKey,
        supplied: SchedulerKey,
    },
    CommandLedgerClassificationMismatch,
    AcceptedTransfer(ContainmentTransferSealError),
    RelocationProcess(RelocationProcessLedgerError),
    RelocationPosition(RelocationPositionTransitionError),
    NoStrictlyLaterMoment {
        source: SimMoment,
    },
}

impl From<AuthorityCursorAdvanceError> for AuthorityRecordSealError {
    fn from(error: AuthorityCursorAdvanceError) -> Self {
        Self::CursorAdvance(error)
    }
}

/// Why a proposed containment transfer cannot be an accepted transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContainmentTransferSealError {
    ActorMismatch,
    ItemNotContained,
    SourceMismatch {
        actual: EntityId,
        expected: EntityId,
    },
    DestinationContainerMissing,
    SourceAuthorityMissing,
    DestinationCapacityExceeded,
    InvalidTransition(ContainmentTransitionError),
}

/// One checked record and the only cursor transition it can later publish.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SealedAuthorityRecord {
    expected_cursor: AuthorityCursor,
    resulting_cursor: AuthorityCursor,
    record: AuthorityRecord,
}

impl SealedAuthorityRecord {
    #[must_use]
    pub(crate) const fn expected_cursor(&self) -> AuthorityCursor {
        self.expected_cursor
    }

    #[must_use]
    pub(crate) const fn resulting_cursor(&self) -> AuthorityCursor {
        self.resulting_cursor
    }

    #[must_use]
    pub(crate) const fn record(&self) -> &AuthorityRecord {
        &self.record
    }

    #[must_use]
    pub(crate) fn into_record(self) -> AuthorityRecord {
        self.record
    }
}

/// Seals one closed record body against the exact immutable base head.
pub(crate) fn seal_authority_record(
    head: &SessionHead,
    closure: &ResolvedExecutionClosureManifestV1,
    draft: DraftAuthorityRecord,
) -> Result<SealedAuthorityRecord, AuthorityRecordSealError> {
    let expected_cursor = draft.expected_cursor();
    let actual_cursor = head.cursor();
    if expected_cursor != actual_cursor {
        return Err(AuthorityRecordSealError::StaleDraft {
            expected: Box::new(expected_cursor),
            actual: Box::new(actual_cursor),
        });
    }
    let plan = expected_cursor.successor_plan()?;
    let normalized = normalize(head, closure, plan.lineage(), draft.into_body())?;
    let preimage = authority_record_preimage(
        plan.lineage(),
        plan.sequence(),
        plan.previous_authority(),
        &normalized,
    );
    let record_id = AuthorityRecordId::of_canonical(&preimage);
    let cumulative = CumulativeAuthorityHash::of_canonical(&cumulative_authority_preimage(
        plan.previous_cumulative(),
        record_id,
    ));
    let body = materialize(record_id, normalized);
    let header = AuthorityRecordHeader::new(
        plan.lineage(),
        plan.revision(),
        plan.sequence(),
        plan.previous_authority(),
        plan.previous_cumulative(),
        record_id,
        cumulative,
    );
    let resulting_cursor = plan.finish(record_id, cumulative);

    Ok(SealedAuthorityRecord {
        expected_cursor,
        resulting_cursor,
        record: AuthorityRecord::new(header, body),
    })
}

fn normalize(
    head: &SessionHead,
    closure: &ResolvedExecutionClosureManifestV1,
    lineage: crate::execution::EpochLineageId,
    draft: DraftAuthorityRecordBody,
) -> Result<NormalizedAuthorityRecordBody, AuthorityRecordSealError> {
    match draft {
        DraftAuthorityRecordBody::Admission(DraftAuthorityAdmission::Commands(requests)) => {
            normalize_command_admission(head, closure, lineage, requests)
        }
        DraftAuthorityRecordBody::Admission(DraftAuthorityAdmission::ActionEvaluation(request)) => {
            normalize_action_evaluation_admission(head, closure, *request)
        }
        DraftAuthorityRecordBody::Management { requests } => {
            normalize_management(head, closure, requests)
        }
        DraftAuthorityRecordBody::KernelSafety { cause } => {
            normalize_kernel_safety(head, closure, cause)
        }
        DraftAuthorityRecordBody::Moment { batch } => {
            normalize_moment(head, closure, lineage, batch)
        }
    }
}

fn normalize_command_admission(
    head: &SessionHead,
    closure: &ResolvedExecutionClosureManifestV1,
    lineage: crate::execution::EpochLineageId,
    mut requests: Vec<crate::kernel::AdmitRequest>,
) -> Result<NormalizedAuthorityRecordBody, AuthorityRecordSealError> {
    if requests.is_empty() {
        return Err(AuthorityRecordSealError::EmptyBatch);
    }
    requests.sort_by(compare_admit_requests);

    let input_namespace =
        derive_input_request_namespace(lineage, closure.specification().external_input_digest());
    let expected_definition_set = closure.semantics().definition_set_digest();
    let placeholder_owner = AuthorityRecordId::from_bytes([0; 32]);
    let mut seen_requests = BTreeSet::new();
    let mut additions_by_moment = BTreeMap::<SimMoment, usize>::new();
    let mut prepared_by_trigger = BTreeMap::new();
    let mut insertions = Vec::with_capacity(requests.len());

    for (position, request) in requests.into_iter().enumerate() {
        if !seen_requests.insert(request.id()) {
            return Err(AuthorityRecordSealError::DuplicateInputRequest);
        }
        if !matches!(
            head.runtime_control()
                .input()
                .classify(request.id(), request.fingerprint()),
            RequestLedgerLookup::Absent
        ) {
            return Err(AuthorityRecordSealError::InputRequestAlreadyRetained);
        }
        if request.command().definition_set_digest() != expected_definition_set {
            return Err(AuthorityRecordSealError::CommandDefinitionSetMismatch {
                expected: expected_definition_set,
                actual: request.command().definition_set_digest(),
            });
        }
        if request.effective() < head.clock().frontier() {
            return Err(AuthorityRecordSealError::EffectiveMomentBeforeFrontier {
                effective: request.effective(),
                frontier: head.clock().frontier(),
            });
        }
        let additions = additions_by_moment.entry(request.effective()).or_default();
        *additions = additions
            .checked_add(1)
            .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;

        let index = local_u32(position)?;
        let prepared = PreparedScheduledCommand::prepare(input_namespace, &request);
        let trigger = prepared.trigger();
        let placeholder =
            CapturedInputRecordId::derive(placeholder_owner, CapturedInputLocalIndex::new(index));
        if prepared_by_trigger
            .insert(trigger, prepared.clone())
            .is_some()
        {
            return Err(AuthorityRecordSealError::DuplicateInputRequest);
        }
        insertions.push(SchedulerInsertion::new(
            SchedulerProducerOrdinal::new(index),
            ScheduledWork::command(prepared.materialize(placeholder)),
        ));
    }
    for (moment, additions) in additions_by_moment {
        ensure_work_population(head, closure, moment, additions)?;
    }

    let plan = head
        .scheduler()
        .plan_batch(insertions)
        .map_err(map_scheduler_batch_plan_error)?;
    let mut entries = Vec::with_capacity(plan.entries().len());
    for (scheduler_key, work) in plan.entries() {
        let ScheduledWork::Command(scheduled) = work else {
            unreachable!("ingress normalization plans only command insertions");
        };
        let ScheduledCommandCause::CapturedExternal { trigger, .. } = scheduled.cause() else {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        };
        let prepared = prepared_by_trigger
            .remove(&trigger)
            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
        entries.push(NormalizedIngressRecord {
            prepared,
            scheduler_key: *scheduler_key,
        });
    }
    if !prepared_by_trigger.is_empty() {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    Ok(NormalizedAuthorityRecordBody::Admission(
        NormalizedAuthorityAdmission::Commands(entries),
    ))
}

fn normalize_action_evaluation_admission(
    head: &SessionHead,
    closure: &ResolvedExecutionClosureManifestV1,
    request: ActionEvaluationCaptureRequest,
) -> Result<NormalizedAuthorityRecordBody, AuthorityRecordSealError> {
    if !matches!(
        head.runtime_control()
            .action_evaluation_captures()
            .classify(
                request.capture(),
                request.invocation(),
                request.fingerprint(),
            ),
        ActionEvaluationCaptureLookup::Absent
    ) {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    let before = head
        .runtime_control()
        .action_evaluations()
        .get(request.invocation())
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
    request
        .validate_new(before, head.clock().frontier())
        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;

    let work = match request.payload() {
        ActionEvaluationCapturePayload::Result { .. } => ActionEvaluationWork::result_ready(
            request.invocation(),
            before.opportunity(),
            before.waiting_version(),
            request.effective(),
        ),
        ActionEvaluationCapturePayload::ArtifactRejected { failure, .. } => {
            ActionEvaluationWork::fallback(
                request.invocation(),
                before.opportunity(),
                before.waiting_version(),
                ActionEvaluationFallbackCause::ArtifactRejected(*failure),
                request.effective(),
            )
        }
    };
    ensure_work_population(head, closure, request.effective(), 1)?;
    let scheduled = ScheduledWork::action_evaluation(work);
    let plan = head
        .scheduler()
        .plan_batch(vec![SchedulerInsertion::new(
            SchedulerProducerOrdinal::new(0),
            scheduled.clone(),
        )])
        .map_err(map_scheduler_batch_plan_error)?;
    let [(scheduler_key, planned)] = plan.entries() else {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    };
    if planned != &scheduled {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }

    let control = closure.semantics().config().deferred_action_control();
    let mut invocations = head.runtime_control().action_evaluations().clone();
    let after = match request.payload() {
        ActionEvaluationCapturePayload::Result { artifact, .. } => invocations.capture_result(
            request.invocation(),
            before.waiting_version(),
            request.capture(),
            request.fingerprint(),
            artifact.clone(),
            request.effective(),
            *scheduler_key,
            control,
        ),
        ActionEvaluationCapturePayload::ArtifactRejected { failure, .. } => invocations
            .capture_artifact_rejection(
                request.invocation(),
                before.waiting_version(),
                request.fingerprint(),
                *failure,
                request.effective(),
                *scheduler_key,
                control,
            ),
    }
    .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?
    .clone();
    let transition = ActionEvaluationInvocationTransitionRecord::new(
        ActionEvaluationInvocationTransitionCause::ResultCapture(request.capture()),
        before.digest(),
        after,
    );
    Ok(NormalizedAuthorityRecordBody::Admission(
        NormalizedAuthorityAdmission::ActionEvaluation(Box::new(
            NormalizedActionEvaluationAdmission {
                request,
                transition,
                scheduler_key: *scheduler_key,
                work: scheduled,
            },
        )),
    ))
}

fn normalize_management(
    head: &SessionHead,
    closure: &ResolvedExecutionClosureManifestV1,
    mut requests: Vec<crate::kernel::ManageRequest>,
) -> Result<NormalizedAuthorityRecordBody, AuthorityRecordSealError> {
    if requests.is_empty() {
        return Err(AuthorityRecordSealError::EmptyBatch);
    }
    requests.sort_by(compare_manage_requests);

    let mut seen_requests = BTreeSet::new();
    let mut current = head.mode();
    let mut current_frontier = head.clock().frontier();
    let mut runtime_control = head.runtime_control().clone();
    let mut scheduler = head.scheduler().clone();
    let placeholder_record = AuthorityRecordId::from_bytes([0; 32]);
    let mut entries = Vec::with_capacity(requests.len());
    for request in requests {
        if !seen_requests.insert(request.id()) {
            return Err(AuthorityRecordSealError::DuplicateManagementRequest);
        }
        if !matches!(
            runtime_control
                .management()
                .classify(request.id(), request.fingerprint()),
            RequestLedgerLookup::Absent
        ) {
            return Err(AuthorityRecordSealError::ManagementRequestAlreadyRetained);
        }

        let operation = request.operation();
        if let crate::kernel::SessionManagement::SealAdmissionThrough(requested) = operation {
            if requested <= current_frontier {
                return Err(AuthorityRecordSealError::AdmissionFrontierNotAdvancing {
                    current: current_frontier,
                    requested,
                });
            }
            if let Some(blocked_at) = runtime_control
                .action_evaluations()
                .minimum_blocked_frontier()
                && requested > blocked_at
            {
                return Err(AuthorityRecordSealError::ActionEvaluationFrontierBlocked {
                    blocked_at,
                });
            }
            if let Some(scheduled) = scheduler.least_due_moment()
                && scheduled < requested
            {
                return Err(
                    AuthorityRecordSealError::AdmissionSealCrossesScheduledWork {
                        requested,
                        scheduled,
                    },
                );
            }
            current_frontier = requested;
        }
        let resulting_mode = match operation.resulting_mode() {
            None => current,
            Some(requested) => match (current, requested) {
                (SessionMode::Running, SessionMode::Paused) => SessionMode::Paused,
                (SessionMode::Paused, SessionMode::Running) => SessionMode::Running,
                (SessionMode::Running | SessionMode::Paused, SessionMode::Quarantined) => {
                    SessionMode::Quarantined
                }
                (SessionMode::Running | SessionMode::Paused, SessionMode::Failed) => {
                    SessionMode::Failed
                }
                _ => {
                    return Err(AuthorityRecordSealError::IllegalManagementTransition {
                        current,
                        requested,
                    });
                }
            },
        };
        let action_evaluation = match operation {
            crate::kernel::SessionManagement::ResolveActionEvaluation {
                invocation,
                disposition,
            } => Some(normalize_action_evaluation_management(
                &mut runtime_control,
                &mut scheduler,
                closure,
                request.id(),
                invocation,
                disposition,
                current_frontier,
            )?),
            crate::kernel::SessionManagement::Pause
            | crate::kernel::SessionManagement::Resume
            | crate::kernel::SessionManagement::Retire(_)
            | crate::kernel::SessionManagement::SealAdmissionThrough(_)
            | crate::kernel::SessionManagement::Quarantine
            | crate::kernel::SessionManagement::Fail => None,
        };

        runtime_control
            .management_mut()
            .insert_exact(
                request.id(),
                request.fingerprint(),
                crate::kernel::ManageOutcome::applied(placeholder_record, operation),
            )
            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
        if let crate::kernel::SessionManagement::Retire(retirement) = operation {
            runtime_control
                .retire(retirement, request.id())
                .map_err(|error| AuthorityRecordSealError::LedgerRetirement {
                    retirement,
                    error,
                })?;
        }

        current = resulting_mode;
        entries.push(NormalizedManagementRecord {
            request,
            resulting_mode: current,
            action_evaluation,
        });
    }
    Ok(NormalizedAuthorityRecordBody::Management {
        cause: Box::new(NormalizedManagementCause::HostRequests(entries)),
        resulting_mode: current,
        preserved_frontier: head.clock().frontier(),
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "management normalization binds one complete invocation and scheduler replacement"
)]
fn normalize_action_evaluation_management(
    runtime_control: &mut crate::control::RuntimeControlState,
    scheduler: &mut SchedulerState,
    closure: &ResolvedExecutionClosureManifestV1,
    request: crate::kernel::ManagementRequestId,
    invocation: ActionEvaluationInvocationId,
    disposition: crate::kernel::ActionEvaluationManagementDisposition,
    frontier: SimMoment,
) -> Result<NormalizedActionEvaluationManagement, AuthorityRecordSealError> {
    let before = runtime_control
        .action_evaluations()
        .get(invocation)
        .cloned()
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
    let (due, removed) = match before.state() {
        ActionEvaluationInvocationState::DispatchPending => {
            let due = match before.admission_mode() {
                crate::execution::DeferredActionAdmissionModeV1::FrontierBlocking => before
                    .blocked_at_frontier()
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?,
                crate::execution::DeferredActionAdmissionModeV1::HostScheduled => frontier.max(
                    strictly_later_moment(before.creation_moment())
                        .map_err(map_post_commit_error)?,
                ),
            };
            (due, None)
        }
        ActionEvaluationInvocationState::ResultCaptured {
            effective,
            scheduler_key,
            ..
        } => {
            let expected = ScheduledWork::action_evaluation(ActionEvaluationWork::result_ready(
                invocation,
                before.opportunity(),
                before.waiting_version(),
                *effective,
            ));
            let removed = scheduler
                .remove_exact(*scheduler_key)
                .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
            if removed != expected {
                return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
            }
            (frontier.max(*effective), Some((*scheduler_key, removed)))
        }
        ActionEvaluationInvocationState::FallbackPending { .. }
        | ActionEvaluationInvocationState::Terminal(_) => {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    };
    ensure_scheduler_work_population(scheduler, closure, due, 1)?;
    let cause = disposition.fallback_cause();
    let insertion_work = ScheduledWork::action_evaluation(ActionEvaluationWork::fallback(
        invocation,
        before.opportunity(),
        before.waiting_version(),
        cause,
        due,
    ));
    let plan = scheduler
        .plan_batch(vec![SchedulerInsertion::new(
            SchedulerProducerOrdinal::new(0),
            insertion_work.clone(),
        )])
        .map_err(map_scheduler_batch_plan_error)?;
    let [(insertion_key, planned)] = plan.entries() else {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    };
    if planned != &insertion_work {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    let insertion_key = *insertion_key;
    scheduler
        .install_batch_exact(plan)
        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;

    let expected_before = before.digest();
    let after = runtime_control
        .action_evaluations_mut()
        .begin_managed_fallback(invocation, before.waiting_version(), cause, insertion_key)
        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?
        .clone();
    Ok(NormalizedActionEvaluationManagement {
        transition: ActionEvaluationInvocationTransitionRecord::new(
            ActionEvaluationInvocationTransitionCause::Management(request),
            expected_before,
            after,
        ),
        removed,
        insertion_key,
        insertion_work,
    })
}

fn normalize_kernel_safety(
    head: &SessionHead,
    closure: &ResolvedExecutionClosureManifestV1,
    cause: KernelSafetyCause,
) -> Result<NormalizedAuthorityRecordBody, AuthorityRecordSealError> {
    if head.mode() != SessionMode::Running {
        return Err(AuthorityRecordSealError::KernelSafetyRequiresRunning {
            current: head.mode(),
        });
    }
    let due = head
        .scheduler()
        .clone_least_due()
        .ok_or(AuthorityRecordSealError::ScheduledWorkMissing)?;
    let due_keys = due
        .entries()
        .iter()
        .map(|(key, _)| *key)
        .collect::<Vec<_>>();
    let mut absent_fingerprints = BTreeMap::<
        (world_model::CommandSource, world_model::CommandId),
        BTreeSet<world_model::CommandRequestFingerprint>,
    >::new();
    for (_, work) in due.entries() {
        let ScheduledWork::Command(scheduled) = work else {
            continue;
        };
        let command = scheduled.command();
        if matches!(
            head.runtime_control().command().classify(
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
    let evaluable_commands = u64::try_from(
        absent_fingerprints
            .values()
            .filter(|fingerprints| fingerprints.len() == 1)
            .count(),
    )
    .map_err(|_| AuthorityRecordSealError::CollectionTooLarge)?;
    let selected = select_kernel_safety_cause(
        closure.semantics().config(),
        head.clock().frontier(),
        &due_keys,
        evaluable_commands,
        head.clock().attempted_wave(due.moment()),
    )
    .map_err(|_| AuthorityRecordSealError::KernelSafetyCauseMismatch)?;
    if selected != Some(cause) {
        return Err(AuthorityRecordSealError::KernelSafetyCauseMismatch);
    }
    let resulting_mode = match cause.disposition() {
        KernelSafetyDisposition::Paused => SessionMode::Paused,
        KernelSafetyDisposition::Quarantined => SessionMode::Quarantined,
        KernelSafetyDisposition::Failed => SessionMode::Failed,
    };
    Ok(NormalizedAuthorityRecordBody::Management {
        cause: Box::new(NormalizedManagementCause::KernelSafety(cause)),
        resulting_mode,
        preserved_frontier: head.clock().frontier(),
    })
}

struct CheckedDeferredDispatchArtifacts {
    request: ActionEvaluationRequestArtifact,
    result_schema: crate::action_evaluation::ActionEvaluationArtifactSchemaId,
    private_continuation: ActionEvaluationPrivateContinuationArtifact,
    private_read_witness: ActionEvaluationPrivateReadWitnessArtifact,
}

enum CheckedDeferredArtifacts {
    Dispatchable(Box<CheckedDeferredDispatchArtifacts>),
    Rejected(ActionEvaluationArtifactFailure),
}

fn build_deferred_artifacts(
    input: DeferredActionInvocationInput,
    control: crate::execution::DeferredActionControlV1,
) -> Result<CheckedDeferredArtifacts, AuthorityRecordSealError> {
    let (_, _, request, result_schema, private_continuation, private_read_witness) =
        input.into_parts();

    let (schema, bytes) = request.into_parts();
    let request = match ActionEvaluationRequestArtifact::new(schema, bytes, control) {
        Ok(artifact) => artifact,
        Err(ActionEvaluationArtifactError::LengthExceeded { failure, .. }) => {
            return Ok(CheckedDeferredArtifacts::Rejected(failure));
        }
        Err(_) => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
    };

    let (schema, bytes) = private_continuation.into_parts();
    let private_continuation =
        match ActionEvaluationPrivateContinuationArtifact::new(schema, bytes, control) {
            Ok(artifact) => artifact,
            Err(ActionEvaluationArtifactError::LengthExceeded { failure, .. }) => {
                return Ok(CheckedDeferredArtifacts::Rejected(failure));
            }
            Err(_) => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
        };

    let (schema, bytes) = private_read_witness.into_parts();
    let private_read_witness =
        match ActionEvaluationPrivateReadWitnessArtifact::new(schema, bytes, control) {
            Ok(artifact) => artifact,
            Err(ActionEvaluationArtifactError::LengthExceeded { failure, .. }) => {
                return Ok(CheckedDeferredArtifacts::Rejected(failure));
            }
            Err(_) => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
        };

    Ok(CheckedDeferredArtifacts::Dispatchable(Box::new(
        CheckedDeferredDispatchArtifacts {
            request,
            result_schema,
            private_continuation,
            private_read_witness,
        },
    )))
}

enum PendingRejectedInvocationKind {
    Initial {
        implementation: crate::execution::LifecycleImplementationId,
    },
    VisibleReinvocation {
        predecessor: Box<ActionEvaluationInvocationRecord>,
    },
}

struct PendingRejectedInvocation {
    cause: ActionEvaluationInvocationOpeningCause,
    kind: PendingRejectedInvocationKind,
    invocation: ActionEvaluationInvocationId,
    opportunity: world_model::ActionOpportunityId,
    pre_wait_version: world_model::ActionOpportunityVersion,
    waiting_version: world_model::ActionOpportunityVersion,
    evaluation_generation: world_model::ActionEvaluationGeneration,
    policy_semantics: [u8; 32],
    action_input_fingerprint: [u8; 32],
    failure: ActionEvaluationArtifactFailure,
    creation_moment: SimMoment,
    source_cursor: AuthorityCursor,
    blocked_at_frontier: Option<SimMoment>,
    control: crate::execution::DeferredActionControlV1,
}

struct PendingFallbackTransition {
    delivery: ActionEvaluationDeliveryRef,
    before: ActionEvaluationInvocationRecord,
    expected_waiting_version: world_model::ActionOpportunityVersion,
    cause: ActionEvaluationFallbackCause,
}

impl PendingRejectedInvocation {
    fn materialize(
        self,
        scheduler_key: SchedulerKey,
    ) -> Result<ActionEvaluationInvocationOpeningRecord, AuthorityRecordSealError> {
        let record = match self.kind {
            PendingRejectedInvocationKind::Initial { implementation } => {
                ActionEvaluationInvocationRecord::artifact_rejected(
                    self.invocation,
                    self.opportunity,
                    self.pre_wait_version,
                    self.waiting_version,
                    self.evaluation_generation,
                    self.policy_semantics,
                    self.action_input_fingerprint,
                    implementation,
                    self.failure,
                    self.creation_moment,
                    self.source_cursor,
                    self.blocked_at_frontier,
                    scheduler_key,
                    self.control,
                )
            }
            PendingRejectedInvocationKind::VisibleReinvocation { predecessor } => {
                ActionEvaluationInvocationRecord::visible_reinvocation_artifact_rejected(
                    predecessor.as_ref(),
                    self.invocation,
                    self.pre_wait_version,
                    self.waiting_version,
                    self.evaluation_generation,
                    self.policy_semantics,
                    self.action_input_fingerprint,
                    self.failure,
                    self.creation_moment,
                    self.source_cursor,
                    self.blocked_at_frontier,
                    scheduler_key,
                    self.control,
                )
            }
        }
        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
        Ok(ActionEvaluationInvocationOpeningRecord::new(
            self.cause, record,
        ))
    }
}

fn normalize_moment(
    head: &SessionHead,
    closure: &ResolvedExecutionClosureManifestV1,
    lineage: crate::execution::EpochLineageId,
    batch: DraftMomentBatch,
) -> Result<NormalizedAuthorityRecordBody, AuthorityRecordSealError> {
    if head.mode() != SessionMode::Running {
        return Err(AuthorityRecordSealError::MomentRequiresRunning {
            current: head.mode(),
        });
    }
    if batch.deliveries().is_empty() {
        return Err(AuthorityRecordSealError::EmptyBatch);
    }
    let mut deliveries = batch.deliveries().to_vec();
    deliveries.sort_by_key(DraftMomentDelivery::key);

    let due = head
        .scheduler()
        .clone_least_due()
        .ok_or(AuthorityRecordSealError::ScheduledWorkMissing)?;
    if let (Some((least, _)), Some(supplied)) = (due.entries().first(), deliveries.first())
        && *least != supplied.key()
    {
        return Err(AuthorityRecordSealError::ScheduledWorkNotLeast {
            least: *least,
            supplied: supplied.key(),
        });
    }
    if due.entries().len() != deliveries.len() {
        return Err(AuthorityRecordSealError::IncompleteDueSet {
            expected: due.entries().len(),
            supplied: deliveries.len(),
        });
    }
    for ((expected_key, expected_work), supplied) in due.entries().iter().zip(&deliveries) {
        if *expected_key != supplied.key() {
            return Err(AuthorityRecordSealError::IncompleteDueSet {
                expected: due.entries().len(),
                supplied: deliveries.len(),
            });
        }
        if expected_work != &supplied.scheduled_work() {
            return Err(AuthorityRecordSealError::ScheduledWorkPayloadMismatch {
                key: *expected_key,
            });
        }
    }

    let fired_moment = due.moment();
    if fired_moment < head.clock().now() {
        return Err(AuthorityRecordSealError::ScheduledWorkBeforeClock {
            scheduled: fired_moment,
            now: head.clock().now(),
        });
    }
    let strictly_later = strictly_later_moment(fired_moment).map_err(map_post_commit_error)?;
    let resulting_frontier = head.clock().frontier().max(strictly_later);
    if let Some(blocked_at) = head
        .runtime_control()
        .action_evaluations()
        .minimum_blocked_frontier()
        && resulting_frontier > blocked_at
    {
        return Err(AuthorityRecordSealError::ActionEvaluationFrontierBlocked { blocked_at });
    }

    verify_resolution_evidence(
        batch.resolution_evidence(),
        closure,
        fired_moment,
        batch.attempts(),
    )?;

    let mut draft_attempts = batch.attempts().to_vec();
    draft_attempts.sort_by_key(|attempt| attempt.identity());
    let mut attempt_refs = BTreeMap::new();
    let mut attempts = Vec::with_capacity(draft_attempts.len());
    let mut commits = Vec::new();
    for attempt in draft_attempts {
        let identity = attempt.identity();
        if attempt_refs.contains_key(&identity) {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        let attempt_ref = AttemptRecordRef::from_position(attempts.len())
            .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
        let (subject, resolution) = match (attempt.subject(), attempt.outcome()) {
            (
                DraftAttemptSubject::EvaluatedCommand(command),
                DraftAttemptOutcome::Accepted(delta),
            ) => {
                if !matches!(
                    head.runtime_control().command().classify(
                        command.source(),
                        command.id(),
                        command.fingerprint(),
                    ),
                    CommandLedgerLookup::Absent
                ) {
                    return Err(AuthorityRecordSealError::CommandLedgerClassificationMismatch);
                }
                if delta.actor() != command.actor() {
                    return Err(AuthorityRecordSealError::AcceptedTransfer(
                        ContainmentTransferSealError::ActorMismatch,
                    ));
                }
                let commit = MomentCommitRef::from_position(commits.len())
                    .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                commits.push(delta);
                (
                    NormalizedAttemptSubject::EvaluatedCommand(command.clone()),
                    NormalizedAttemptResolution::Accepted { commit },
                )
            }
            (
                DraftAttemptSubject::EvaluatedCommand(command),
                DraftAttemptOutcome::Rejected(reason),
            ) => {
                if !matches!(
                    head.runtime_control().command().classify(
                        command.source(),
                        command.id(),
                        command.fingerprint(),
                    ),
                    CommandLedgerLookup::Absent
                ) {
                    return Err(AuthorityRecordSealError::CommandLedgerClassificationMismatch);
                }
                (
                    NormalizedAttemptSubject::EvaluatedCommand(command.clone()),
                    NormalizedAttemptResolution::Rejected(reason),
                )
            }
            (
                DraftAttemptSubject::CommandIdCollision {
                    source,
                    command,
                    fingerprints,
                },
                DraftAttemptOutcome::CommandIdCollision,
            ) => {
                let Some(first) = fingerprints.first().copied() else {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                };
                if fingerprints.len() < 2
                    || !is_strictly_sorted(fingerprints)
                    || !matches!(
                        head.runtime_control()
                            .command()
                            .classify(*source, *command, first),
                        CommandLedgerLookup::Absent
                    )
                {
                    return Err(AuthorityRecordSealError::CommandLedgerClassificationMismatch);
                }
                (
                    NormalizedAttemptSubject::CommandIdCollision {
                        source: *source,
                        command: *command,
                        fingerprints: fingerprints.clone(),
                    },
                    NormalizedAttemptResolution::CommandIdCollision,
                )
            }
            _ => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
        };
        attempt_refs.insert(identity, attempt_ref);
        attempts.push(NormalizedAttemptRecord {
            subject,
            resolution,
        });
    }

    let mut accepted = if commits.is_empty() {
        head.accepted().clone()
    } else {
        apply_containment_transfers(head.accepted(), &commits)
            .map_err(map_containment_transition_error)
            .map_err(AuthorityRecordSealError::AcceptedTransfer)?
    };
    let containment_delta = commits.clone();
    let mut physical_events = commits
        .iter()
        .copied()
        .map(PhysicalEvent::item_transferred)
        .collect::<Vec<_>>();
    let mut relocation_processes = head.runtime_control().relocation_processes().clone();
    let mut lifecycle_control = head.runtime_control().lifecycle().clone();
    let mut appraisal_ledger = head.runtime_control().appraisals().clone();
    let mut action_evaluation_ledger = head.runtime_control().action_evaluations().clone();
    let mut evidence_generation_cursor = head
        .accepted()
        .epistemic()
        .actors()
        .iter()
        .map(|record| (record.actor(), record.last_delivery_generation().get()))
        .collect::<BTreeMap<_, _>>();
    for delivery in &deliveries {
        if let DraftMomentDelivery::EvidenceDelivery { delivery, .. } = delivery {
            evidence_generation_cursor
                .entry(delivery.evidence().observer())
                .and_modify(|generation| {
                    *generation = (*generation).max(delivery.evidence().generation().get());
                })
                .or_insert(delivery.evidence().generation().get());
        }
    }
    let mut due_evidence_by_actor = BTreeMap::<ActorId, Vec<world_model::EvidenceRecord>>::new();
    for delivery in &deliveries {
        if let DraftMomentDelivery::EvidenceDelivery { delivery, .. } = delivery {
            due_evidence_by_actor
                .entry(delivery.evidence().observer())
                .or_default()
                .push(delivery.evidence());
        }
    }
    let consumed_keys = deliveries.iter().map(DraftMomentDelivery::key).collect();
    let mut command_deliveries = Vec::new();
    let mut post_commit_deliveries = Vec::new();
    let mut lifecycle_deliveries = Vec::new();
    let mut action_ready_deliveries = Vec::new();
    let mut action_evaluation_deliveries = Vec::new();
    let mut attempt_resolved_deliveries = Vec::new();
    let mut relocation_process_deliveries = Vec::new();
    let mut action_opportunity_transitions = Vec::new();
    let mut action_evaluation_invocation_openings = Vec::new();
    let mut action_evaluation_invocation_transitions = Vec::new();
    let mut pending_rejected_invocations =
        BTreeMap::<ActionEvaluationInvocationId, PendingRejectedInvocation>::new();
    let mut pending_fallback_transitions =
        BTreeMap::<ActionEvaluationInvocationId, PendingFallbackTransition>::new();
    let mut action_opportunity_openings = Vec::new();
    let mut evidence_routing = Vec::new();
    let mut evidence_assimilations = Vec::new();
    let mut appraisal_transitions = Vec::new();
    let mut intent_adoptions = Vec::new();
    let mut intent_transitions = Vec::new();
    let mut activity_starts = Vec::new();
    let mut activity_transitions = Vec::new();
    let mut activity_terminal_transitions = Vec::new();
    let mut lifecycle_mutations =
        BTreeMap::<(ActorId, LifecycleRole), PendingLifecycleMutation>::new();
    let mut relocation_attempts = Vec::new();
    let mut relocation_process_transitions = Vec::new();
    let mut transitioned_opportunities = BTreeSet::new();
    let mut transitioned_action_evaluations = BTreeSet::new();
    let mut neutral_wakes = BTreeSet::new();
    let mut routed_feedback_attempts = BTreeSet::new();
    let mut scheduled_work = Vec::new();
    let mut resolutions = Vec::with_capacity(deliveries.len());

    for item in deliveries {
        match item {
            DraftMomentDelivery::Command {
                key,
                scheduled,
                resolution,
            } => {
                let delivery = CommandDeliveryRef::from_position(command_deliveries.len())
                    .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                command_deliveries.push(CommandDeliveryRecord::new(key, &scheduled));
                let mut new_attempt = None;
                match resolution {
                    DraftDeliveryResolution::NewCommand { attempt } => {
                        let identity =
                            ContainmentCommandIdentity::from_command(scheduled.command());
                        if identity != attempt.identity() {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        let attempt = *attempt_refs
                            .get(&identity)
                            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        let normalized_attempt =
                            attempts
                                .get(usize::try_from(attempt.position()).map_err(|_| {
                                    AuthorityRecordSealError::InvalidNormalizedGraph
                                })?)
                                .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        if normalized_attempt.subject
                            != NormalizedAttemptSubject::EvaluatedCommand(
                                scheduled.command().clone(),
                            )
                        {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        new_attempt = Some(attempt);
                        resolutions
                            .push(NormalizedDeliveryResolution::NewCommand { delivery, attempt });
                    }
                    DraftDeliveryResolution::RetainedCommand {
                        original_attempt,
                        original_outcome,
                    } => {
                        match head.runtime_control().command().classify(
                            scheduled.command().source(),
                            scheduled.command().id(),
                            scheduled.command().fingerprint(),
                        ) {
                            CommandLedgerLookup::RetainedExact {
                                original_attempt: retained_attempt,
                                outcome: retained_outcome,
                            } if retained_attempt == original_attempt
                                && retained_outcome == original_outcome => {}
                            _ => {
                                return Err(
                                    AuthorityRecordSealError::CommandLedgerClassificationMismatch,
                                );
                            }
                        }
                        resolutions.push(NormalizedDeliveryResolution::RetainedCommand {
                            delivery,
                            original_attempt,
                            original_outcome,
                        });
                    }
                    DraftDeliveryResolution::CommandIdReuseMismatch { original_attempt } => {
                        match head.runtime_control().command().classify(
                            scheduled.command().source(),
                            scheduled.command().id(),
                            scheduled.command().fingerprint(),
                        ) {
                            CommandLedgerLookup::IdReuseMismatch {
                                original_attempt: retained_attempt,
                            } if retained_attempt == original_attempt => {}
                            _ => {
                                return Err(
                                    AuthorityRecordSealError::CommandLedgerClassificationMismatch,
                                );
                            }
                        }
                        resolutions.push(NormalizedDeliveryResolution::CommandIdReuseMismatch {
                            delivery,
                            original_attempt,
                        });
                    }
                    DraftDeliveryResolution::NewCollision { attempt } => {
                        let identity =
                            ContainmentCommandIdentity::from_command(scheduled.command());
                        if identity != attempt.identity() {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        let attempt = *attempt_refs
                            .get(&identity)
                            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        let normalized_attempt =
                            attempts
                                .get(usize::try_from(attempt.position()).map_err(|_| {
                                    AuthorityRecordSealError::InvalidNormalizedGraph
                                })?)
                                .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        match &normalized_attempt.subject {
                            NormalizedAttemptSubject::CommandIdCollision {
                                source,
                                command,
                                fingerprints,
                            } if *source == scheduled.command().source()
                                && *command == scheduled.command().id()
                                && fingerprints
                                    .binary_search(&scheduled.command().fingerprint())
                                    .is_ok() => {}
                            _ => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
                        }
                        resolutions
                            .push(NormalizedDeliveryResolution::NewCollision { delivery, attempt });
                    }
                    DraftDeliveryResolution::RetainedCollision { original_attempt } => {
                        match head.runtime_control().command().classify(
                            scheduled.command().source(),
                            scheduled.command().id(),
                            scheduled.command().fingerprint(),
                        ) {
                            CommandLedgerLookup::RetainedCollision {
                                original_attempt: retained_attempt,
                            } if retained_attempt == original_attempt => {}
                            _ => {
                                return Err(
                                    AuthorityRecordSealError::CommandLedgerClassificationMismatch,
                                );
                            }
                        }
                        resolutions.push(NormalizedDeliveryResolution::RetainedCollision {
                            delivery,
                            original_attempt,
                        });
                    }
                    DraftDeliveryResolution::RetiredCommand => {
                        if !matches!(
                            head.runtime_control().command().classify(
                                scheduled.command().source(),
                                scheduled.command().id(),
                                scheduled.command().fingerprint(),
                            ),
                            CommandLedgerLookup::Retired
                        ) {
                            return Err(
                                AuthorityRecordSealError::CommandLedgerClassificationMismatch,
                            );
                        }
                        resolutions.push(NormalizedDeliveryResolution::RetiredCommand { delivery });
                    }
                }
                if let Some(opportunity) = scheduled.action_opportunity() {
                    validate_action_command(head, opportunity, scheduled.command(), closure)?;
                    if let Some(attempt) = new_attempt
                        && let Some(feedback) = rejected_containment_feedback(
                            head,
                            closure,
                            &attempts,
                            attempt,
                            opportunity,
                            scheduled.command(),
                        )?
                        && routed_feedback_attempts.insert(attempt)
                    {
                        let generation = next_evidence_generation(
                            &mut evidence_generation_cursor,
                            feedback.actor,
                        )?;
                        let evidence = world_model::EvidenceRecord::direct_item_absent(
                            feedback.actor,
                            generation,
                            feedback.item,
                            feedback.expected_container,
                        );
                        evidence_routing.push(EvidenceRoutingRecord::rejected_containment_attempt(
                            attempt, evidence,
                        ));
                        scheduled_work.push(ScheduledWork::lifecycle(
                            LifecycleWork::EvidenceDelivery(EvidenceDeliveryWork::new(
                                evidence,
                                strictly_later,
                            )),
                        ));
                    }
                    if !neutral_wakes.insert(opportunity) {
                        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                    }
                    scheduled_work.push(ScheduledWork::attempt_resolved(AttemptResolved::new(
                        opportunity,
                        strictly_later,
                    )));
                }
            }
            DraftMomentDelivery::PostCommit {
                key,
                dispatch,
                observations,
            } => {
                let delivery = PostCommitDeliveryRef::from_position(post_commit_deliveries.len())
                    .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                for observation in observations {
                    let event = dispatch
                        .reaction()
                        .events()
                        .get(
                            usize::try_from(observation.event_index())
                                .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
                        )
                        .copied()
                        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                    if observation.observer() != event.actor() {
                        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                    }
                    let next = next_evidence_generation(
                        &mut evidence_generation_cursor,
                        observation.observer(),
                    )?;
                    let evidence = world_model::EvidenceRecord::direct_physical_event(
                        observation.observer(),
                        next,
                        event,
                    );
                    evidence_routing.push(EvidenceRoutingRecord::physical_event(
                        delivery,
                        observation.event_index(),
                        evidence,
                    ));
                    scheduled_work.push(ScheduledWork::lifecycle(LifecycleWork::EvidenceDelivery(
                        EvidenceDeliveryWork::new(evidence, strictly_later),
                    )));
                }
                post_commit_deliveries.push(PostCommitDeliveryRecord::new(key, dispatch));
                resolutions.push(NormalizedDeliveryResolution::PostCommitConsumed { delivery });
            }
            DraftMomentDelivery::EvidenceDelivery {
                key,
                delivery: work,
                assimilation,
            } => {
                let delivery = LifecycleDeliveryRef::from_position(lifecycle_deliveries.len())
                    .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                lifecycle_deliveries.push(LifecycleDeliveryRecord::new(
                    key,
                    LifecycleWork::EvidenceDelivery(work),
                ));
                resolutions.push(NormalizedDeliveryResolution::LifecycleConsumed { delivery });
                if let Some((actor, expected_version, successor)) = assimilation {
                    let evidence = due_evidence_by_actor
                        .get(&actor)
                        .cloned()
                        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                    let expected = head
                        .accepted()
                        .epistemic()
                        .assimilate(actor, expected_version, evidence.clone())
                        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                    if expected != *successor {
                        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                    }
                    let next_epistemic = accepted
                        .epistemic()
                        .assimilate(actor, expected_version, evidence.clone())
                        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                    accepted = accepted_with_epistemic(&accepted, next_epistemic);
                    evidence_assimilations.push(EvidenceAssimilationRecord::new(
                        actor,
                        expected_version,
                        evidence.clone(),
                    ));
                    let appraisal_causes = evidence
                        .iter()
                        .filter_map(|record| match record.provenance() {
                            EvidenceProvenance::DirectItemTransfer(_) => {
                                Some(LifecycleCause::Evidence(record.id()))
                            }
                            EvidenceProvenance::DirectItemAbsent(_) => {
                                Some(LifecycleCause::Evidence(record.id()))
                            }
                            EvidenceProvenance::DirectActorDeparture(_)
                            | EvidenceProvenance::DirectActorArrival(_) => None,
                        })
                        .collect::<Vec<_>>();
                    request_lifecycle(
                        &mut lifecycle_mutations,
                        actor,
                        LifecycleRole::Appraisal,
                        &appraisal_causes,
                        strictly_later,
                    )?;
                }
            }
            DraftMomentDelivery::Appraisal { key, work, results } => {
                let delivery = LifecycleDeliveryRef::from_position(lifecycle_deliveries.len())
                    .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                lifecycle_deliveries.push(LifecycleDeliveryRecord::new(
                    key,
                    LifecycleWork::Appraisal(work),
                ));
                resolutions.push(NormalizedDeliveryResolution::LifecycleConsumed { delivery });

                for result in results {
                    match result {
                        crate::kernel::AppraisalResult::Present {
                            appraisal,
                            material_changed: reported_change,
                        } => {
                            validate_appraisal(&accepted, work.actor(), appraisal)?;
                            let before = appraisal_ledger.get(appraisal.actor(), appraisal.item());
                            let material_changed = before
                                .map(ContainmentAppraisal::material_fingerprint)
                                != Some(appraisal.material_fingerprint());
                            if reported_change != material_changed {
                                return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                            }
                            appraisal_ledger.retain(appraisal);
                            appraisal_transitions.push(
                                ContainmentAppraisalTransitionRecord::Present {
                                    before,
                                    after: appraisal,
                                },
                            );
                            if material_changed {
                                request_lifecycle(
                                    &mut lifecycle_mutations,
                                    appraisal.actor(),
                                    LifecycleRole::IntentReview,
                                    &[LifecycleCause::Appraisal {
                                        generation: work.generation(),
                                        material: appraisal.material_fingerprint(),
                                    }],
                                    strictly_later,
                                )?;
                            }
                        }
                        crate::kernel::AppraisalResult::Retract {
                            before,
                            supporting_evidence,
                        } => {
                            validate_appraisal_retraction(
                                &accepted,
                                work.actor(),
                                before,
                                supporting_evidence,
                            )?;
                            if !appraisal_ledger.retract_exact(before) {
                                return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                            }
                            appraisal_transitions.push(
                                ContainmentAppraisalTransitionRecord::Retracted {
                                    before,
                                    supporting_evidence,
                                },
                            );
                        }
                    }
                }
                complete_lifecycle(
                    &mut lifecycle_mutations,
                    work.actor(),
                    LifecycleRole::Appraisal,
                    work.generation(),
                    strictly_later,
                )?;
            }
            DraftMomentDelivery::IntentReview { key, work, result } => {
                let delivery = LifecycleDeliveryRef::from_position(lifecycle_deliveries.len())
                    .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                lifecycle_deliveries.push(LifecycleDeliveryRecord::new(
                    key,
                    LifecycleWork::IntentReview(work),
                ));
                resolutions.push(NormalizedDeliveryResolution::LifecycleConsumed { delivery });

                if let IntentReviewResult::Adopt(intent) = result {
                    if intent.actor() != work.actor() {
                        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                    }
                    let next_agency = accepted
                        .agency()
                        .adopt_intent(intent)
                        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                    accepted = accepted_with_agency(&accepted, next_agency);
                    intent_adoptions.push(IntentAdoptionRecord::new(intent));
                    request_lifecycle(
                        &mut lifecycle_mutations,
                        intent.actor(),
                        LifecycleRole::ActivityInitialization,
                        &[LifecycleCause::Intent {
                            intent: intent.id(),
                            version: intent.version(),
                        }],
                        strictly_later,
                    )?;
                }
                complete_lifecycle(
                    &mut lifecycle_mutations,
                    work.actor(),
                    LifecycleRole::IntentReview,
                    work.generation(),
                    strictly_later,
                )?;
            }
            DraftMomentDelivery::ActivityInitialization { key, work, result } => {
                let delivery = LifecycleDeliveryRef::from_position(lifecycle_deliveries.len())
                    .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                lifecycle_deliveries.push(LifecycleDeliveryRecord::new(
                    key,
                    LifecycleWork::ActivityInitialization(work),
                ));
                resolutions.push(NormalizedDeliveryResolution::LifecycleConsumed { delivery });
                match result {
                    ActivityInitializationResult::Start {
                        activity,
                        opportunity,
                    } => {
                        let activity = *activity;
                        validate_opened_opportunity(activity, &opportunity)?;
                        let next_agency = accepted
                            .agency()
                            .start_activity(activity, true)
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        accepted = accepted_with_agency(&accepted, next_agency);
                        activity_starts.push(ActivityStartRecord::new(activity));
                        action_opportunity_openings
                            .push(ActionOpportunityOpeningRecord::new(opportunity.clone()));
                        scheduled_work.push(ScheduledWork::action_ready(
                            crate::scheduler::ActionReady::new(
                                opportunity.id(),
                                opportunity.version(),
                                strictly_later,
                            ),
                        ));
                    }
                    ActivityInitializationResult::TransitionIntent {
                        expected_version,
                        successor,
                    } => {
                        let before = accepted
                            .agency()
                            .intent(successor.id())
                            .copied()
                            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        if before.version() != expected_version {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        let transition = intent_transition_between(before, successor)?;
                        let next_agency = accepted
                            .agency()
                            .transition_intent(before.id(), expected_version, transition)
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        if next_agency.intent(before.id()).copied() != Some(successor) {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        accepted = accepted_with_agency(&accepted, next_agency);
                        intent_transitions.push(IntentTransitionRecord::new(before, successor));
                    }
                }
                complete_lifecycle(
                    &mut lifecycle_mutations,
                    work.actor(),
                    LifecycleRole::ActivityInitialization,
                    work.generation(),
                    strictly_later,
                )?;
            }
            DraftMomentDelivery::ActionReady {
                key,
                ready,
                opportunity,
                proposal,
            } => {
                if ready.opportunity() != opportunity.id()
                    || ready.expected_version() != opportunity.version()
                    || opportunity.state() != ActionOpportunityState::Open
                    || head
                        .runtime_control()
                        .action_opportunities()
                        .get(opportunity.id())
                        != Some(&opportunity)
                    || !transitioned_opportunities.insert(opportunity.id())
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }

                let delivery = ActionReadyDeliveryRef::from_position(action_ready_deliveries.len())
                    .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                action_ready_deliveries.push(ActionReadyDeliveryRecord::new(key, ready));
                resolutions.push(NormalizedDeliveryResolution::ActionReadyConsumed { delivery });

                if let ActionProposal::BeginDeferred {
                    expected_version,
                    input,
                } = proposal
                {
                    if expected_version != opportunity.version() {
                        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                    }
                    let input = *input;
                    let policy_semantics = input.policy_semantics();
                    let action_input_fingerprint = input.action_input_fingerprint();
                    let (waiting, invocation) = opportunity
                        .begin_evaluation(
                            expected_version,
                            policy_semantics,
                            action_input_fingerprint,
                        )
                        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                    let control = closure.semantics().config().deferred_action_control();
                    let blocked_at_frontier = match control.admission_mode() {
                        Some(crate::execution::DeferredActionAdmissionModeV1::FrontierBlocking) => {
                            Some(resulting_frontier)
                        }
                        Some(crate::execution::DeferredActionAdmissionModeV1::HostScheduled) => {
                            None
                        }
                        None => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
                    };
                    let implementation = closure
                        .semantics()
                        .lifecycle_profiles()
                        .action()
                        .binding()
                        .implementation();
                    let artifacts = build_deferred_artifacts(input, control)?;
                    let opening_cause =
                        ActionEvaluationInvocationOpeningCause::ActionReady(delivery);
                    match artifacts {
                        CheckedDeferredArtifacts::Dispatchable(artifacts) => {
                            let CheckedDeferredDispatchArtifacts {
                                request,
                                result_schema,
                                private_continuation,
                                private_read_witness,
                            } = *artifacts;
                            let record = ActionEvaluationInvocationRecord::dispatch_pending(
                                invocation,
                                opportunity.id(),
                                opportunity.version(),
                                waiting.version(),
                                waiting.evaluation_generation(),
                                policy_semantics,
                                action_input_fingerprint,
                                implementation,
                                request,
                                result_schema,
                                private_continuation,
                                private_read_witness,
                                fired_moment,
                                head.cursor(),
                                blocked_at_frontier,
                                control,
                            )
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                            action_evaluation_invocation_openings.push(
                                ActionEvaluationInvocationOpeningRecord::new(opening_cause, record),
                            );
                        }
                        CheckedDeferredArtifacts::Rejected(failure) => {
                            let due = blocked_at_frontier.unwrap_or(strictly_later);
                            scheduled_work.push(ScheduledWork::action_evaluation(
                                ActionEvaluationWork::fallback(
                                    invocation,
                                    opportunity.id(),
                                    waiting.version(),
                                    ActionEvaluationFallbackCause::ArtifactRejected(failure),
                                    due,
                                ),
                            ));
                            if pending_rejected_invocations
                                .insert(
                                    invocation,
                                    PendingRejectedInvocation {
                                        cause: opening_cause,
                                        kind: PendingRejectedInvocationKind::Initial {
                                            implementation,
                                        },
                                        invocation,
                                        opportunity: opportunity.id(),
                                        pre_wait_version: opportunity.version(),
                                        waiting_version: waiting.version(),
                                        evaluation_generation: waiting.evaluation_generation(),
                                        policy_semantics,
                                        action_input_fingerprint,
                                        failure,
                                        creation_moment: fired_moment,
                                        source_cursor: head.cursor(),
                                        blocked_at_frontier,
                                        control,
                                    },
                                )
                                .is_some()
                            {
                                return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                            }
                        }
                    }
                    action_opportunity_transitions.push(ActionOpportunityTransitionRecord::new(
                        *opportunity,
                        waiting,
                    ));
                } else {
                    let (disposition, command, relocation) = match proposal {
                        ActionProposal::Submit {
                            expected_version,
                            command,
                        } if expected_version == opportunity.version() => {
                            validate_action_command(head, opportunity.id(), &command, closure)?;
                            (
                                ActionOpportunityDisposition::ActionSubmitted,
                                Some(command),
                                None,
                            )
                        }
                        ActionProposal::Finish {
                            expected_version,
                            disposition,
                        } if expected_version == opportunity.version()
                            && disposition != ActionOpportunityDisposition::ActionSubmitted =>
                        {
                            (disposition, None, None)
                        }
                        ActionProposal::Relocation {
                            expected_version,
                            interaction,
                        } if expected_version == opportunity.version()
                            && opportunity
                                .interaction_scope()
                                .relocation_scope()
                                .is_some_and(|scope| scope.permits(interaction)) =>
                        {
                            (
                                ActionOpportunityDisposition::ActionSubmitted,
                                None,
                                Some(interaction),
                            )
                        }
                        ActionProposal::BeginDeferred { .. } => {
                            unreachable!("deferred proposal was handled above")
                        }
                        _ => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
                    };
                    let successor = opportunity
                        .consume(opportunity.version(), disposition)
                        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;

                    action_opportunity_transitions.push(ActionOpportunityTransitionRecord::new(
                        *opportunity.clone(),
                        successor,
                    ));

                    match (command, relocation) {
                        (Some(command), None) => {
                            scheduled_work.push(ScheduledWork::command(
                                ScheduledCommand::from_action_opportunity(
                                    opportunity.id(),
                                    strictly_later,
                                    *command,
                                ),
                            ));
                        }
                        (None, Some(interaction)) => {
                            let resolution = match apply_relocation_interaction(
                                &accepted,
                                &relocation_processes,
                                opportunity.actor(),
                                interaction,
                                fired_moment,
                                RelocationProcessTransitionCause::Action(
                                    ActionResolutionDeliveryRef::Ready(delivery),
                                ),
                            ) {
                                Ok((next_accepted, next_processes, transition, wake)) => {
                                    accepted = next_accepted;
                                    relocation_processes = next_processes;
                                    if let Some(event) = transition.event() {
                                        physical_events.push(event);
                                    }
                                    if let Some(wake) = wake {
                                        scheduled_work.push(ScheduledWork::process(wake));
                                    }
                                    let process = transition.after().id();
                                    relocation_process_transitions.push(transition);
                                    RelocationAttemptResolution::Accepted { process }
                                }
                                Err(RelocationInteractionApplyError::Rejected(reason)) => {
                                    RelocationAttemptResolution::Rejected(reason)
                                }
                                Err(RelocationInteractionApplyError::Authority(error)) => {
                                    return Err(error);
                                }
                            };
                            relocation_attempts.push(RelocationAttemptRecord::new(
                                ActionResolutionDeliveryRef::Ready(delivery),
                                interaction,
                                resolution,
                            ));
                            if !neutral_wakes.insert(opportunity.id()) {
                                return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                            }
                            scheduled_work.push(ScheduledWork::attempt_resolved(
                                AttemptResolved::new(opportunity.id(), strictly_later),
                            ));
                        }
                        (None, None) => {
                            if !neutral_wakes.insert(opportunity.id()) {
                                return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                            }
                            scheduled_work.push(ScheduledWork::attempt_resolved(
                                AttemptResolved::new(opportunity.id(), strictly_later),
                            ));
                        }
                        (Some(_), Some(_)) => {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                    }
                }
            }
            DraftMomentDelivery::ActionEvaluation {
                key,
                evaluation,
                opportunity,
                invocation,
                proposal,
            } => {
                let opportunity = *opportunity;
                let invocation = *invocation;
                if evaluation.invocation() != invocation.invocation()
                    || evaluation.opportunity() != opportunity.id()
                    || evaluation.expected_waiting_version() != opportunity.version()
                    || opportunity.state()
                        != ActionOpportunityState::WaitingForEvaluation(invocation.invocation())
                    || head
                        .runtime_control()
                        .action_opportunities()
                        .get(opportunity.id())
                        != Some(&opportunity)
                    || head
                        .runtime_control()
                        .action_evaluations()
                        .get(invocation.invocation())
                        != Some(&invocation)
                    || !transitioned_action_evaluations.insert(invocation.invocation())
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }

                let delivery =
                    ActionEvaluationDeliveryRef::from_position(action_evaluation_deliveries.len())
                        .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                action_evaluation_deliveries
                    .push(ActionEvaluationDeliveryRecord::new(key, evaluation));
                resolutions
                    .push(NormalizedDeliveryResolution::ActionEvaluationConsumed { delivery });
                let transition_cause =
                    ActionEvaluationInvocationTransitionCause::EvaluationDelivery(delivery);

                match evaluation {
                    ActionEvaluationWork::ResultReady { .. } => {
                        let Some(proposal) = proposal else {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        };
                        let ActionEvaluationInvocationState::ResultCaptured { result, .. } =
                            invocation.state()
                        else {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        };
                        let result = *result;
                        match proposal {
                            ActionEvaluationDecision::Apply { freshness, action } => {
                                if !transitioned_opportunities.insert(opportunity.id()) {
                                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                                }
                                let reopened = opportunity
                                    .resume_evaluation(
                                        opportunity.version(),
                                        invocation.invocation(),
                                    )
                                    .map_err(|_| {
                                        AuthorityRecordSealError::InvalidNormalizedGraph
                                    })?;
                                action_opportunity_transitions.push(
                                    ActionOpportunityTransitionRecord::new(
                                        opportunity.clone(),
                                        reopened.clone(),
                                    ),
                                );

                                let (disposition, command, relocation) = match action {
                                    EvaluatedAction::Submit(command) => {
                                        validate_action_command(
                                            head,
                                            opportunity.id(),
                                            &command,
                                            closure,
                                        )?;
                                        (
                                            ActionOpportunityDisposition::ActionSubmitted,
                                            Some(command),
                                            None,
                                        )
                                    }
                                    EvaluatedAction::Relocate(interaction)
                                        if opportunity
                                            .interaction_scope()
                                            .relocation_scope()
                                            .is_some_and(|scope| scope.permits(interaction)) =>
                                    {
                                        (
                                            ActionOpportunityDisposition::ActionSubmitted,
                                            None,
                                            Some(interaction),
                                        )
                                    }
                                    EvaluatedAction::NoApplicableAction => (
                                        ActionOpportunityDisposition::NoApplicableAction,
                                        None,
                                        None,
                                    ),
                                    EvaluatedAction::Relocate(_) => {
                                        return Err(
                                            AuthorityRecordSealError::InvalidNormalizedGraph,
                                        );
                                    }
                                };
                                let consumed =
                                    reopened.consume(reopened.version(), disposition).map_err(
                                        |_| AuthorityRecordSealError::InvalidNormalizedGraph,
                                    )?;
                                action_opportunity_transitions.push(
                                    ActionOpportunityTransitionRecord::new(reopened, consumed),
                                );

                                match (command, relocation) {
                                    (Some(command), None) => {
                                        scheduled_work.push(ScheduledWork::command(
                                            ScheduledCommand::from_action_opportunity(
                                                opportunity.id(),
                                                strictly_later,
                                                *command,
                                            ),
                                        ));
                                    }
                                    (None, Some(interaction)) => {
                                        let resolution = match apply_relocation_interaction(
                                            &accepted,
                                            &relocation_processes,
                                            opportunity.actor(),
                                            interaction,
                                            fired_moment,
                                            RelocationProcessTransitionCause::Action(
                                                ActionResolutionDeliveryRef::Evaluation(delivery),
                                            ),
                                        ) {
                                            Ok((
                                                next_accepted,
                                                next_processes,
                                                transition,
                                                wake,
                                            )) => {
                                                accepted = next_accepted;
                                                relocation_processes = next_processes;
                                                if let Some(event) = transition.event() {
                                                    physical_events.push(event);
                                                }
                                                if let Some(wake) = wake {
                                                    scheduled_work
                                                        .push(ScheduledWork::process(wake));
                                                }
                                                let process = transition.after().id();
                                                relocation_process_transitions.push(transition);
                                                RelocationAttemptResolution::Accepted { process }
                                            }
                                            Err(RelocationInteractionApplyError::Rejected(
                                                reason,
                                            )) => RelocationAttemptResolution::Rejected(reason),
                                            Err(RelocationInteractionApplyError::Authority(
                                                error,
                                            )) => return Err(error),
                                        };
                                        relocation_attempts.push(RelocationAttemptRecord::new(
                                            ActionResolutionDeliveryRef::Evaluation(delivery),
                                            interaction,
                                            resolution,
                                        ));
                                        if !neutral_wakes.insert(opportunity.id()) {
                                            return Err(
                                                AuthorityRecordSealError::InvalidNormalizedGraph,
                                            );
                                        }
                                        scheduled_work.push(ScheduledWork::attempt_resolved(
                                            AttemptResolved::new(opportunity.id(), strictly_later),
                                        ));
                                    }
                                    (None, None) => {
                                        if !neutral_wakes.insert(opportunity.id()) {
                                            return Err(
                                                AuthorityRecordSealError::InvalidNormalizedGraph,
                                            );
                                        }
                                        scheduled_work.push(ScheduledWork::attempt_resolved(
                                            AttemptResolved::new(opportunity.id(), strictly_later),
                                        ));
                                    }
                                    (Some(_), Some(_)) => {
                                        return Err(
                                            AuthorityRecordSealError::InvalidNormalizedGraph,
                                        );
                                    }
                                }

                                let expected_before = invocation.digest();
                                let after = action_evaluation_ledger
                                    .finish_applied(
                                        invocation.invocation(),
                                        opportunity.version(),
                                        result,
                                        freshness,
                                    )
                                    .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?
                                    .clone();
                                action_evaluation_invocation_transitions.push(
                                    ActionEvaluationInvocationTransitionRecord::new(
                                        transition_cause,
                                        expected_before,
                                        after,
                                    ),
                                );
                            }
                            ActionEvaluationDecision::Reinvoke(input) => {
                                let input = *input;
                                let Some(retained_request) = invocation.request() else {
                                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                                };
                                let Some(retained_result_schema) = invocation.result_schema()
                                else {
                                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                                };
                                let Some(retained_continuation) = invocation.private_continuation()
                                else {
                                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                                };
                                let Some(retained_witness) = invocation.private_read_witness()
                                else {
                                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                                };
                                if input.policy_semantics() != *invocation.policy_semantics()
                                    || input.request().schema() != retained_request.schema()
                                    || input.result_schema() != retained_result_schema
                                    || input.private_continuation().schema()
                                        != retained_continuation.schema()
                                    || input.private_read_witness().schema()
                                        != retained_witness.schema()
                                    || !transitioned_opportunities.insert(opportunity.id())
                                {
                                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                                }

                                let reopened = opportunity
                                    .reopen_for_visible_reinvocation(
                                        opportunity.version(),
                                        invocation.invocation(),
                                    )
                                    .map_err(|_| {
                                        AuthorityRecordSealError::InvalidNormalizedGraph
                                    })?;
                                let policy_semantics = input.policy_semantics();
                                let action_input_fingerprint = input.action_input_fingerprint();
                                let (waiting, successor) = reopened
                                    .begin_evaluation(
                                        reopened.version(),
                                        policy_semantics,
                                        action_input_fingerprint,
                                    )
                                    .map_err(|_| {
                                        AuthorityRecordSealError::InvalidNormalizedGraph
                                    })?;
                                action_opportunity_transitions.push(
                                    ActionOpportunityTransitionRecord::new(
                                        opportunity.clone(),
                                        reopened.clone(),
                                    ),
                                );
                                action_opportunity_transitions.push(
                                    ActionOpportunityTransitionRecord::new(
                                        reopened.clone(),
                                        waiting.clone(),
                                    ),
                                );

                                let expected_before = invocation.digest();
                                let predecessor = action_evaluation_ledger
                                    .finish_reinvoked(
                                        invocation.invocation(),
                                        opportunity.version(),
                                        result,
                                        successor,
                                    )
                                    .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?
                                    .clone();
                                action_evaluation_invocation_transitions.push(
                                    ActionEvaluationInvocationTransitionRecord::new(
                                        transition_cause,
                                        expected_before,
                                        predecessor.clone(),
                                    ),
                                );

                                let control =
                                    closure.semantics().config().deferred_action_control();
                                let blocked_at_frontier = match control.admission_mode() {
                                    Some(
                                        crate::execution::DeferredActionAdmissionModeV1::FrontierBlocking,
                                    ) => Some(resulting_frontier),
                                    Some(
                                        crate::execution::DeferredActionAdmissionModeV1::HostScheduled,
                                    ) => None,
                                    None => {
                                        return Err(
                                            AuthorityRecordSealError::InvalidNormalizedGraph,
                                        );
                                    }
                                };
                                let artifacts = build_deferred_artifacts(input, control)?;
                                let opening_cause =
                                    ActionEvaluationInvocationOpeningCause::VisibleReinvocation(
                                        delivery,
                                    );
                                match artifacts {
                                    CheckedDeferredArtifacts::Dispatchable(artifacts) => {
                                        let CheckedDeferredDispatchArtifacts {
                                            request,
                                            private_continuation,
                                            private_read_witness,
                                            ..
                                        } = *artifacts;
                                        let successor_record =
                                            ActionEvaluationInvocationRecord::visible_reinvocation_dispatch_pending(
                                                &predecessor,
                                                successor,
                                                reopened.version(),
                                                waiting.version(),
                                                waiting.evaluation_generation(),
                                                policy_semantics,
                                                action_input_fingerprint,
                                                request,
                                                private_continuation,
                                                private_read_witness,
                                                fired_moment,
                                                head.cursor(),
                                                blocked_at_frontier,
                                                control,
                                            )
                                            .map_err(|_| {
                                                AuthorityRecordSealError::InvalidNormalizedGraph
                                            })?;
                                        action_evaluation_invocation_openings.push(
                                            ActionEvaluationInvocationOpeningRecord::new(
                                                opening_cause,
                                                successor_record,
                                            ),
                                        );
                                    }
                                    CheckedDeferredArtifacts::Rejected(failure) => {
                                        let due = blocked_at_frontier.unwrap_or(strictly_later);
                                        scheduled_work.push(ScheduledWork::action_evaluation(
                                            ActionEvaluationWork::fallback(
                                                successor,
                                                opportunity.id(),
                                                waiting.version(),
                                                ActionEvaluationFallbackCause::ArtifactRejected(
                                                    failure,
                                                ),
                                                due,
                                            ),
                                        ));
                                        if pending_rejected_invocations
                                            .insert(
                                                successor,
                                                PendingRejectedInvocation {
                                                    cause: opening_cause,
                                                    kind: PendingRejectedInvocationKind::VisibleReinvocation {
                                                        predecessor: Box::new(predecessor),
                                                    },
                                                    invocation: successor,
                                                    opportunity: opportunity.id(),
                                                    pre_wait_version: reopened.version(),
                                                    waiting_version: waiting.version(),
                                                    evaluation_generation: waiting
                                                        .evaluation_generation(),
                                                    policy_semantics,
                                                    action_input_fingerprint,
                                                    failure,
                                                    creation_moment: fired_moment,
                                                    source_cursor: head.cursor(),
                                                    blocked_at_frontier,
                                                    control,
                                                },
                                            )
                                            .is_some()
                                        {
                                            return Err(
                                                AuthorityRecordSealError::InvalidNormalizedGraph,
                                            );
                                        }
                                    }
                                }
                            }
                            ActionEvaluationDecision::RequireFallback(failure) => {
                                let cause = match failure {
                                    ActionEvaluationResultFailure::InvalidResult => {
                                        ActionEvaluationFallbackCause::InvalidResult
                                    }
                                    ActionEvaluationResultFailure::VisibleReinvocationExhausted => {
                                        ActionEvaluationFallbackCause::VisibleReinvocationExhausted
                                    }
                                };
                                scheduled_work.push(ScheduledWork::action_evaluation(
                                    ActionEvaluationWork::fallback(
                                        invocation.invocation(),
                                        opportunity.id(),
                                        opportunity.version(),
                                        cause,
                                        strictly_later,
                                    ),
                                ));
                                if pending_fallback_transitions
                                    .insert(
                                        invocation.invocation(),
                                        PendingFallbackTransition {
                                            delivery,
                                            before: invocation,
                                            expected_waiting_version: opportunity.version(),
                                            cause,
                                        },
                                    )
                                    .is_some()
                                {
                                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                                }
                            }
                        }
                    }
                    ActionEvaluationWork::Fallback { cause, .. } => {
                        if proposal.is_some()
                            || !transitioned_opportunities.insert(opportunity.id())
                        {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        let reopened = opportunity
                            .resume_evaluation(opportunity.version(), invocation.invocation())
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        let consumed = reopened
                            .consume(reopened.version(), ActionOpportunityDisposition::Failed)
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        action_opportunity_transitions.push(
                            ActionOpportunityTransitionRecord::new(
                                opportunity.clone(),
                                reopened.clone(),
                            ),
                        );
                        action_opportunity_transitions
                            .push(ActionOpportunityTransitionRecord::new(reopened, consumed));
                        if !neutral_wakes.insert(opportunity.id()) {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        scheduled_work.push(ScheduledWork::attempt_resolved(AttemptResolved::new(
                            opportunity.id(),
                            strictly_later,
                        )));

                        let expected_before = invocation.digest();
                        let after = action_evaluation_ledger
                            .finish_fallback(invocation.invocation(), opportunity.version())
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?
                            .clone();
                        if after.state()
                            != &ActionEvaluationInvocationState::Terminal(
                                ActionEvaluationTerminal::Failed { cause },
                            )
                        {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        action_evaluation_invocation_transitions.push(
                            ActionEvaluationInvocationTransitionRecord::new(
                                transition_cause,
                                expected_before,
                                after,
                            ),
                        );
                    }
                }
            }
            DraftMomentDelivery::AttemptResolved { key, resolved } => {
                let delivery =
                    AttemptResolvedDeliveryRef::from_position(attempt_resolved_deliveries.len())
                        .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                attempt_resolved_deliveries.push(AttemptResolvedDeliveryRecord::new(key, resolved));
                resolutions
                    .push(NormalizedDeliveryResolution::AttemptResolvedConsumed { delivery });
                let opportunity = head
                    .runtime_control()
                    .action_opportunities()
                    .get(resolved.opportunity())
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                if let ActionSponsor::Activity(sponsor) = opportunity.sponsor() {
                    let activity = accepted
                        .agency()
                        .activity(sponsor.activity())
                        .copied()
                        .filter(|activity| activity.version() == sponsor.expected_version())
                        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                    if activity.actor() != opportunity.actor() {
                        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                    }
                    let recovery_due = later_microsteps(fired_moment, 4)?;
                    request_lifecycle(
                        &mut lifecycle_mutations,
                        opportunity.actor(),
                        LifecycleRole::ActivityAdvance,
                        &[LifecycleCause::AttemptResolved(opportunity.id())],
                        recovery_due,
                    )?;
                }
            }
            DraftMomentDelivery::ActivityAdvance { key, work, result } => {
                let delivery = LifecycleDeliveryRef::from_position(lifecycle_deliveries.len())
                    .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                lifecycle_deliveries.push(LifecycleDeliveryRecord::new(
                    key,
                    LifecycleWork::ActivityAdvance(work),
                ));
                resolutions.push(NormalizedDeliveryResolution::LifecycleConsumed { delivery });
                match result {
                    ActivityAdvanceResult::OpenAction {
                        expected_version,
                        successor,
                        opportunity,
                    } => {
                        let successor = *successor;
                        if successor.status().is_terminal() {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        let before = accepted
                            .agency()
                            .activity(successor.id())
                            .copied()
                            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        if before.version() != expected_version {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        let transition = activity_transition_between(before, successor)?;
                        let next_agency = accepted
                            .agency()
                            .transition_activity(before.id(), expected_version, transition)
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        if next_agency.activity(before.id()).copied() != Some(successor) {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        validate_opened_opportunity(successor, &opportunity)?;
                        accepted = accepted_with_agency(&accepted, next_agency);
                        activity_transitions.push(ActivityTransitionRecord::new(before, successor));
                        action_opportunity_openings
                            .push(ActionOpportunityOpeningRecord::new(opportunity.clone()));
                        scheduled_work.push(ScheduledWork::action_ready(
                            crate::scheduler::ActionReady::new(
                                opportunity.id(),
                                opportunity.version(),
                                strictly_later,
                            ),
                        ));
                    }
                    ActivityAdvanceResult::Transition {
                        expected_version,
                        successor,
                    } => {
                        let successor = *successor;
                        if successor.status().is_terminal() {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        let before = accepted
                            .agency()
                            .activity(successor.id())
                            .copied()
                            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        if before.version() != expected_version {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        let transition = activity_transition_between(before, successor)?;
                        let next_agency = accepted
                            .agency()
                            .transition_activity(before.id(), expected_version, transition)
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        if next_agency.activity(before.id()).copied() != Some(successor) {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        accepted = accepted_with_agency(&accepted, next_agency);
                        activity_transitions.push(ActivityTransitionRecord::new(before, successor));
                    }
                    ActivityAdvanceResult::Terminal {
                        expected_activity_version,
                        activity_successor,
                        expected_intent_version,
                        intent_successor,
                    } => {
                        let activity_after = *activity_successor;
                        let activity_before = accepted
                            .agency()
                            .activity(activity_after.id())
                            .copied()
                            .filter(|activity| activity.version() == expected_activity_version)
                            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        let intent_before = accepted
                            .agency()
                            .intent(intent_successor.id())
                            .copied()
                            .filter(|intent| intent.version() == expected_intent_version)
                            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        let (activity_transition, intent_transition) =
                            activity_terminal_transitions_between(
                                activity_before,
                                activity_after,
                                intent_before,
                                intent_successor,
                            )?;
                        let next_agency = accepted
                            .agency()
                            .transition_activity(
                                activity_before.id(),
                                expected_activity_version,
                                activity_transition,
                            )
                            .and_then(|agency| {
                                agency.transition_intent(
                                    intent_before.id(),
                                    expected_intent_version,
                                    intent_transition,
                                )
                            })
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                        if next_agency.activity(activity_before.id()).copied()
                            != Some(activity_after)
                            || next_agency.intent(intent_before.id()).copied()
                                != Some(intent_successor)
                        {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        accepted = accepted_with_agency(&accepted, next_agency);
                        activity_terminal_transitions.push(ActivityTerminalTransitionRecord::new(
                            activity_before,
                            activity_after,
                            intent_before,
                            intent_successor,
                        ));
                    }
                    ActivityAdvanceResult::NoChange {
                        activity,
                        expected_version,
                    } => {
                        if accepted
                            .agency()
                            .activity(activity)
                            .is_none_or(|current| current.version() != expected_version)
                        {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                    }
                }
                complete_lifecycle(
                    &mut lifecycle_mutations,
                    work.actor(),
                    LifecycleRole::ActivityAdvance,
                    work.generation(),
                    strictly_later,
                )?;
            }
            DraftMomentDelivery::RelocationProcess {
                key,
                wake,
                classification,
            } => {
                let delivery = RelocationProcessDeliveryRef::from_position(
                    relocation_process_deliveries.len(),
                )
                .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
                relocation_process_deliveries.push(RelocationProcessDeliveryRecord::new(key, wake));
                let current = relocation_processes.classify_wake(wake);
                if current != classification {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
                match current {
                    RelocationWakeClassification::Obsolete => {
                        resolutions.push(NormalizedDeliveryResolution::ObsoleteRelocationWake {
                            delivery,
                        });
                    }
                    RelocationWakeClassification::Current(process) => {
                        let (actual_before, after) = relocation_processes
                            .complete(wake, fired_moment.time())
                            .map_err(AuthorityRecordSealError::RelocationProcess)?;
                        if actual_before.id() != process
                            || !matches!(after.status(), RelocationProcessStatus::Completed { .. })
                        {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        accepted = apply_relocation_arrival(&accepted, after)
                            .map_err(AuthorityRecordSealError::RelocationPosition)?;
                        let route = after.route();
                        let event = PhysicalEvent::actor_arrived(
                            after.id(),
                            after.actor(),
                            route.source(),
                            route.destination(),
                        );
                        physical_events.push(event);
                        relocation_process_transitions.push(
                            RelocationProcessTransitionRecord::new(
                                RelocationProcessTransitionCause::Wake(delivery),
                                Some(actual_before),
                                after,
                                Some(event),
                            ),
                        );
                        resolutions.push(
                            NormalizedDeliveryResolution::RelocationProcessCompleted { delivery },
                        );
                    }
                }
            }
        }
    }

    let lifecycle_control_mutations = finalize_lifecycle_mutations(
        &mut lifecycle_control,
        lifecycle_mutations,
        &mut scheduled_work,
    )?;

    let mut reactions = Vec::new();
    let mut post_commit_normalization = None;
    if let Some(reaction) = crate::scheduler::ReactionEnvelope::from_events(physical_events) {
        let reaction_ref = MomentReactionRef::from_position(0)
            .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
        let prepared_dispatch =
            PreparedPostCommitDispatch::prepare(lineage, fired_moment, reaction.clone());
        let placeholder_reaction = ReactionEnvelopeId::derive(
            AuthorityRecordId::from_bytes([0; 32]),
            ReactionLocalIndex::new(0),
        );
        let placeholder = prepared_dispatch.clone().materialize(placeholder_reaction);
        scheduled_work.push(ScheduledWork::PostCommit(placeholder.clone()));
        post_commit_normalization = Some((reaction_ref, prepared_dispatch, placeholder));
        reactions.push(reaction);
    }

    ensure_work_population(head, closure, strictly_later, scheduled_work.len())?;
    let mut scheduler_insertions = Vec::with_capacity(scheduled_work.len());
    if !scheduled_work.is_empty() {
        let plan = head
            .scheduler()
            .plan_batch(
                scheduled_work
                    .into_iter()
                    .enumerate()
                    .map(|(position, work)| {
                        SchedulerInsertion::new(
                            SchedulerProducerOrdinal::new(u32::try_from(position).unwrap_or_else(
                                |_| unreachable!("checked work population must fit u32"),
                            )),
                            work,
                        )
                    })
                    .collect(),
            )
            .map_err(map_scheduler_batch_plan_error)?;
        for (scheduler_key, work) in plan.entries() {
            let insertion = match work {
                ScheduledWork::Command(scheduled) => {
                    let opportunity = scheduled
                        .action_opportunity()
                        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                    NormalizedSchedulerInsertion::action_command(
                        *scheduler_key,
                        opportunity,
                        scheduled.effective(),
                        scheduled.command().clone(),
                    )
                }
                ScheduledWork::Lifecycle(LifecycleWork::AttemptResolved(resolved)) => {
                    NormalizedSchedulerInsertion::attempt_resolved(*scheduler_key, *resolved)
                }
                ScheduledWork::Lifecycle(work) => {
                    NormalizedSchedulerInsertion::lifecycle(*scheduler_key, *work)
                }
                ScheduledWork::PostCommit(dispatch) => {
                    let Some((reaction, prepared, placeholder)) =
                        post_commit_normalization.as_ref()
                    else {
                        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                    };
                    if dispatch != placeholder {
                        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                    }
                    NormalizedSchedulerInsertion::post_commit(
                        *scheduler_key,
                        *reaction,
                        prepared.clone(),
                    )
                }
                ScheduledWork::Process(wake) => {
                    NormalizedSchedulerInsertion::relocation_process(*scheduler_key, *wake)
                }
                ScheduledWork::ActionReady(ready) => {
                    NormalizedSchedulerInsertion::action_ready(*scheduler_key, *ready)
                }
                ScheduledWork::ActionEvaluation(work) => {
                    let invocation = work.invocation();
                    if let Some(pending) = pending_rejected_invocations.remove(&invocation) {
                        if *work
                            != ActionEvaluationWork::fallback(
                                pending.invocation,
                                pending.opportunity,
                                pending.waiting_version,
                                ActionEvaluationFallbackCause::ArtifactRejected(pending.failure),
                                scheduler_key.moment(),
                            )
                        {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        action_evaluation_invocation_openings
                            .push(pending.materialize(*scheduler_key)?);
                    } else if let Some(pending) = pending_fallback_transitions.remove(&invocation) {
                        if pending.before.invocation() != invocation
                            || *work
                                != ActionEvaluationWork::fallback(
                                    invocation,
                                    pending.before.opportunity(),
                                    pending.expected_waiting_version,
                                    pending.cause,
                                    scheduler_key.moment(),
                                )
                        {
                            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                        }
                        let expected_before = pending.before.digest();
                        let after = action_evaluation_ledger
                            .begin_fallback(
                                invocation,
                                pending.expected_waiting_version,
                                pending.cause,
                                *scheduler_key,
                            )
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?
                            .clone();
                        action_evaluation_invocation_transitions.push(
                            ActionEvaluationInvocationTransitionRecord::new(
                                ActionEvaluationInvocationTransitionCause::EvaluationDelivery(
                                    pending.delivery,
                                ),
                                expected_before,
                                after,
                            ),
                        );
                    } else {
                        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                    }
                    NormalizedSchedulerInsertion::action_evaluation(*scheduler_key, *work)
                }
            };
            scheduler_insertions.push(insertion);
        }
    }
    if !pending_rejected_invocations.is_empty() || !pending_fallback_transitions.is_empty() {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }

    let mut opportunity_control = head.runtime_control().clone();
    for transition in &action_opportunity_transitions {
        let before = transition.before();
        let after = transition.after();
        if opportunity_control.action_opportunities().get(before.id()) != Some(before) {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        let applied = match (before.state(), after.state()) {
            (
                ActionOpportunityState::Open,
                ActionOpportunityState::WaitingForEvaluation(invocation),
            ) => {
                let opening = action_evaluation_invocation_openings
                    .iter()
                    .find(|opening| opening.invocation().invocation() == invocation)
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                let (applied, derived) = opportunity_control
                    .begin_action_evaluation(
                        before.id(),
                        before.version(),
                        *opening.invocation().policy_semantics(),
                        *opening.invocation().action_input_fingerprint(),
                    )
                    .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                if derived != invocation {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
                applied
            }
            (
                ActionOpportunityState::WaitingForEvaluation(invocation),
                ActionOpportunityState::Open,
            ) if before.evaluation_generation() == after.evaluation_generation() => {
                opportunity_control
                    .resume_action_evaluation(before.id(), before.version(), invocation)
                    .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?
            }
            (
                ActionOpportunityState::WaitingForEvaluation(invocation),
                ActionOpportunityState::Open,
            ) => opportunity_control
                .reopen_action_evaluation(before.id(), before.version(), invocation)
                .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
            (ActionOpportunityState::Open, ActionOpportunityState::Consumed(disposition)) => {
                opportunity_control
                    .consume_action_opportunity(before.id(), before.version(), disposition)
                    .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?
            }
            _ => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
        };
        if applied != after {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }
    for transition in &action_evaluation_invocation_transitions {
        let applied = opportunity_control
            .action_evaluations_mut()
            .install_transition_exact(transition.expected_before(), transition.after().clone())
            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
        if applied != transition.after() {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }
    for opening in &action_evaluation_invocation_openings {
        let invocation = opening.invocation();
        let waiting = opportunity_control
            .action_opportunities()
            .get(invocation.opportunity())
            .cloned()
            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
        let installed = match invocation.state() {
            ActionEvaluationInvocationState::DispatchPending => opportunity_control
                .action_evaluations_mut()
                .install_dispatch(invocation.clone(), &waiting),
            ActionEvaluationInvocationState::FallbackPending {
                cause: ActionEvaluationFallbackCause::ArtifactRejected(_),
                ..
            } => opportunity_control
                .action_evaluations_mut()
                .install_artifact_rejection(invocation.clone(), &waiting),
            ActionEvaluationInvocationState::ResultCaptured { .. }
            | ActionEvaluationInvocationState::FallbackPending { .. }
            | ActionEvaluationInvocationState::Terminal(_) => {
                return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
            }
        }
        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
        if installed != invocation {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }
    for opening in &action_opportunity_openings {
        opportunity_control
            .open_action_opportunity(opening.opportunity().clone())
            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
    }

    let normalized = NormalizedMomentBatch {
        moment: fired_moment,
        resulting_frontier: head.clock().frontier().max(strictly_later),
        consumed_keys,
        command_deliveries,
        post_commit_deliveries,
        lifecycle_deliveries,
        action_ready_deliveries,
        action_evaluation_deliveries,
        attempt_resolved_deliveries,
        relocation_process_deliveries,
        action_opportunity_transitions,
        action_evaluation_invocation_openings,
        action_evaluation_invocation_transitions,
        action_opportunity_openings,
        evidence_routing,
        evidence_assimilations,
        appraisal_transitions,
        intent_adoptions,
        intent_transitions,
        activity_starts,
        activity_transitions,
        activity_terminal_transitions,
        lifecycle_control_mutations,
        relocation_attempts,
        relocation_process_transitions,
        attempts,
        commits,
        containment_delta,
        reactions,
        scheduler_insertions,
        resolutions,
        resolution_evidence: batch.resolution_evidence().clone(),
    };
    verify_normalized_moment(head, closure, &normalized)?;
    Ok(NormalizedAuthorityRecordBody::Moment(Box::new(normalized)))
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RelocationInteractionApplyError {
    Rejected(RelocationAttemptRejection),
    Authority(AuthorityRecordSealError),
}

fn apply_relocation_interaction(
    accepted: &world_model::AcceptedState,
    processes: &RelocationProcessLedger,
    actor: ActorId,
    interaction: RelocationInteraction,
    moment: SimMoment,
    cause: RelocationProcessTransitionCause,
) -> Result<
    (
        world_model::AcceptedState,
        RelocationProcessLedger,
        RelocationProcessTransitionRecord,
        Option<RelocationProcessWake>,
    ),
    RelocationInteractionApplyError,
> {
    let mut successor_processes = processes.clone();
    match interaction {
        RelocationInteraction::Start(route_id) => {
            let route = accepted.domain().route(route_id).ok_or(
                RelocationInteractionApplyError::Rejected(
                    RelocationAttemptRejection::RouteUnavailable,
                ),
            )?;
            if accepted.domain().actor_location(actor) != Some(ActorLocation::at(route.source())) {
                return Err(RelocationInteractionApplyError::Rejected(
                    RelocationAttemptRejection::PositionMismatch,
                ));
            }
            let process = successor_processes
                .start(actor, route, moment.time())
                .map_err(relocation_attempt_ledger_error)?;
            let accepted =
                apply_relocation_departure(accepted, process).map_err(|error| match error {
                    RelocationPositionTransitionError::ActorPositionMissing { .. }
                    | RelocationPositionTransitionError::PositionMismatch { .. } => {
                        RelocationInteractionApplyError::Rejected(
                            RelocationAttemptRejection::PositionMismatch,
                        )
                    }
                    RelocationPositionTransitionError::InvalidSuccessor(_) => {
                        RelocationInteractionApplyError::Authority(
                            AuthorityRecordSealError::RelocationPosition(error),
                        )
                    }
                })?;
            let event = PhysicalEvent::actor_departed(
                process.id(),
                actor,
                route.source(),
                route.destination(),
            );
            let wake = RelocationProcessWake::for_active(process).ok_or(
                RelocationInteractionApplyError::Authority(
                    AuthorityRecordSealError::InvalidNormalizedGraph,
                ),
            )?;
            Ok((
                accepted,
                successor_processes,
                RelocationProcessTransitionRecord::new(cause, None, process, Some(event)),
                Some(wake),
            ))
        }
        RelocationInteraction::Pause(route_id) => {
            let before = successor_processes.live_for(actor).ok_or(
                RelocationInteractionApplyError::Rejected(
                    RelocationAttemptRejection::ProcessUnavailable,
                ),
            )?;
            if before.route().id() != route_id {
                return Err(RelocationInteractionApplyError::Rejected(
                    RelocationAttemptRejection::ProcessUnavailable,
                ));
            }
            if accepted.domain().actor_location(actor)
                != Some(ActorLocation::in_transit(before.route()))
            {
                return Err(RelocationInteractionApplyError::Rejected(
                    RelocationAttemptRejection::PositionMismatch,
                ));
            }
            let (actual_before, after) = successor_processes
                .pause(before.id(), before.version(), moment.time())
                .map_err(relocation_attempt_ledger_error)?;
            if actual_before != before {
                return Err(RelocationInteractionApplyError::Authority(
                    AuthorityRecordSealError::InvalidNormalizedGraph,
                ));
            }
            Ok((
                accepted.clone(),
                successor_processes,
                RelocationProcessTransitionRecord::new(cause, Some(before), after, None),
                None,
            ))
        }
        RelocationInteraction::Resume(route_id) => {
            let before = successor_processes.live_for(actor).ok_or(
                RelocationInteractionApplyError::Rejected(
                    RelocationAttemptRejection::ProcessUnavailable,
                ),
            )?;
            if before.route().id() != route_id {
                return Err(RelocationInteractionApplyError::Rejected(
                    RelocationAttemptRejection::ProcessUnavailable,
                ));
            }
            if accepted.domain().actor_location(actor)
                != Some(ActorLocation::in_transit(before.route()))
            {
                return Err(RelocationInteractionApplyError::Rejected(
                    RelocationAttemptRejection::PositionMismatch,
                ));
            }
            let (actual_before, after) = successor_processes
                .resume(before.id(), before.version(), moment.time())
                .map_err(relocation_attempt_ledger_error)?;
            if actual_before != before {
                return Err(RelocationInteractionApplyError::Authority(
                    AuthorityRecordSealError::InvalidNormalizedGraph,
                ));
            }
            let wake = RelocationProcessWake::for_active(after).ok_or(
                RelocationInteractionApplyError::Authority(
                    AuthorityRecordSealError::InvalidNormalizedGraph,
                ),
            )?;
            Ok((
                accepted.clone(),
                successor_processes,
                RelocationProcessTransitionRecord::new(cause, Some(before), after, None),
                Some(wake),
            ))
        }
    }
}

fn relocation_attempt_ledger_error(
    error: RelocationProcessLedgerError,
) -> RelocationInteractionApplyError {
    let rejection = match error {
        RelocationProcessLedgerError::LiveProcessExists { .. } => {
            RelocationAttemptRejection::ProcessStateConflict
        }
        RelocationProcessLedgerError::UnknownProcess { .. }
        | RelocationProcessLedgerError::ProcessNotLive { .. } => {
            RelocationAttemptRejection::ProcessUnavailable
        }
        RelocationProcessLedgerError::ProcessValueMismatch { .. } => {
            RelocationAttemptRejection::ProcessStateConflict
        }
        RelocationProcessLedgerError::ActorGenerationOverflow { .. } => {
            RelocationAttemptRejection::LimitReached
        }
        RelocationProcessLedgerError::InvalidTransition { error, .. } => match error {
            RelocationProcessError::TimeOverflow | RelocationProcessError::GenerationOverflow => {
                RelocationAttemptRejection::LimitReached
            }
            RelocationProcessError::StaleVersion { .. }
            | RelocationProcessError::NotActive
            | RelocationProcessError::NotPaused
            | RelocationProcessError::AlreadyCompleted
            | RelocationProcessError::PauseBeforeSegment
            | RelocationProcessError::CompletionAlreadyDue
            | RelocationProcessError::StaleWake { .. }
            | RelocationProcessError::WrongCompletionTime { .. } => {
                RelocationAttemptRejection::ProcessStateConflict
            }
        },
    };
    RelocationInteractionApplyError::Rejected(rejection)
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

fn validate_appraisal(
    accepted: &world_model::AcceptedState,
    actor: ActorId,
    appraisal: ContainmentAppraisal,
) -> Result<(), AuthorityRecordSealError> {
    if appraisal.actor() != actor {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    let evidence = accepted
        .epistemic()
        .evidence_record(appraisal.supporting_evidence())
        .copied()
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
    let EvidenceProvenance::DirectItemTransfer(event) = evidence.provenance() else {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    };
    let belief = accepted
        .epistemic()
        .contained_in(actor, appraisal.item())
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
    let expected = ContainmentAppraisal::new(
        actor,
        belief.item(),
        belief.container(),
        event.source(),
        evidence.id(),
    );
    if expected != appraisal {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    Ok(())
}

fn validate_appraisal_retraction(
    accepted: &world_model::AcceptedState,
    actor: ActorId,
    before: ContainmentAppraisal,
    supporting_evidence: world_model::EvidenceDeliveryId,
) -> Result<(), AuthorityRecordSealError> {
    let evidence = accepted
        .epistemic()
        .evidence_record(supporting_evidence)
        .copied()
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
    let EvidenceProvenance::DirectItemAbsent(observation) = evidence.provenance() else {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    };
    if before.actor() != actor
        || evidence.observer() != actor
        || observation.item() != before.item()
        || observation.expected_container() != before.believed_current_container()
        || accepted
            .epistemic()
            .contained_in(actor, before.item())
            .is_some()
    {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    Ok(())
}

fn validate_opened_opportunity(
    activity: Activity,
    opportunity: &ActionOpportunity,
) -> Result<(), AuthorityRecordSealError> {
    if !opportunity.matches_activity_opening(activity) {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    Ok(())
}

fn intent_transition_between(
    before: Intent,
    after: Intent,
) -> Result<IntentTransition, AuthorityRecordSealError> {
    if before.id() != after.id()
        || before.actor() != after.actor()
        || before.generation() != after.generation()
        || before.desired() != after.desired()
    {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
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
        _ => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
    };
    if before.transition(before.version(), transition).ok() != Some(after) {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    Ok(transition)
}

fn activity_transition_between(
    before: Activity,
    after: Activity,
) -> Result<ActivityTransition, AuthorityRecordSealError> {
    if before.id() != after.id()
        || before.actor() != after.actor()
        || before.intent() != after.intent()
        || before.generation() != after.generation()
        || before.controller() != after.controller()
        || before.state_schema() != after.state_schema()
    {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
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
        _ => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
    };
    if before.transition(before.version(), transition).ok() != Some(after) {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    Ok(transition)
}

fn activity_terminal_transitions_between(
    activity_before: Activity,
    activity_after: Activity,
    intent_before: Intent,
    intent_after: Intent,
) -> Result<(ActivityTransition, IntentTransition), AuthorityRecordSealError> {
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
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    Ok((
        activity_transition_between(activity_before, activity_after)?,
        intent_transition_between(intent_before, intent_after)?,
    ))
}

#[derive(Default)]
struct PendingLifecycleMutation {
    requested: BTreeSet<LifecycleCause>,
    request_due: Option<SimMoment>,
    completed: Option<LifecycleGeneration>,
    completion_due: Option<SimMoment>,
}

fn request_lifecycle(
    mutations: &mut BTreeMap<(ActorId, LifecycleRole), PendingLifecycleMutation>,
    actor: ActorId,
    role: LifecycleRole,
    causes: &[LifecycleCause],
    due: SimMoment,
) -> Result<(), AuthorityRecordSealError> {
    if causes.is_empty() {
        return Ok(());
    }
    if causes.iter().copied().any(|cause| !role.accepts(cause)) {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    let mutation = mutations.entry((actor, role)).or_default();
    mutation.requested.extend(causes.iter().copied());
    mutation.request_due = Some(mutation.request_due.map_or(due, |current| current.max(due)));
    Ok(())
}

fn complete_lifecycle(
    mutations: &mut BTreeMap<(ActorId, LifecycleRole), PendingLifecycleMutation>,
    actor: ActorId,
    role: LifecycleRole,
    generation: LifecycleGeneration,
    due: SimMoment,
) -> Result<(), AuthorityRecordSealError> {
    let mutation = mutations.entry((actor, role)).or_default();
    if mutation.completed.replace(generation).is_some() {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    mutation.completion_due = Some(due);
    Ok(())
}

fn finalize_lifecycle_mutations(
    ledger: &mut crate::control::LifecycleControlLedger,
    mutations: BTreeMap<(ActorId, LifecycleRole), PendingLifecycleMutation>,
    scheduled: &mut Vec<ScheduledWork>,
) -> Result<Vec<LifecycleControlMutationRecord>, AuthorityRecordSealError> {
    let mut records = Vec::with_capacity(mutations.len());
    for ((actor, role), mutation) in mutations {
        let requested = mutation.requested.into_iter().collect::<Vec<_>>();
        if !requested.is_empty() {
            match ledger
                .request(actor, role, &requested)
                .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?
            {
                LifecycleWakeRequestOutcome::Enqueue { generation } => {
                    let due = mutation
                        .request_due
                        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                    scheduled.push(lifecycle_work(role, actor, generation, due));
                }
                LifecycleWakeRequestOutcome::Duplicate { .. }
                | LifecycleWakeRequestOutcome::Coalesced { .. } => {}
            }
        }
        if let Some(generation) = mutation.completed {
            let outcome = ledger
                .complete(actor, role, generation)
                .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
            if let Some(successor) = outcome.successor() {
                let completion_due = mutation
                    .completion_due
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                let due = mutation.request_due.map_or(completion_due, |request_due| {
                    request_due.max(completion_due)
                });
                scheduled.push(lifecycle_work(role, actor, successor, due));
            }
        }
        records.push(LifecycleControlMutationRecord::new(
            actor,
            role,
            requested,
            mutation.completed,
        ));
    }
    Ok(records)
}

fn lifecycle_work(
    role: LifecycleRole,
    actor: ActorId,
    generation: LifecycleGeneration,
    due: SimMoment,
) -> ScheduledWork {
    let work = match role {
        LifecycleRole::Appraisal => {
            LifecycleWork::Appraisal(AppraisalWork::new(actor, generation, due))
        }
        LifecycleRole::IntentReview => {
            LifecycleWork::IntentReview(IntentReviewWork::new(actor, generation, due))
        }
        LifecycleRole::ActivityInitialization => LifecycleWork::ActivityInitialization(
            ActivityInitializationWork::new(actor, generation, due),
        ),
        LifecycleRole::ActivityAdvance => {
            LifecycleWork::ActivityAdvance(ActivityAdvanceWork::new(actor, generation, due))
        }
    };
    ScheduledWork::lifecycle(work)
}

fn later_microsteps(
    mut moment: SimMoment,
    count: u32,
) -> Result<SimMoment, AuthorityRecordSealError> {
    for _ in 0..count {
        moment = strictly_later_moment(moment).map_err(map_post_commit_error)?;
    }
    Ok(moment)
}

fn ensure_work_population(
    head: &SessionHead,
    closure: &ResolvedExecutionClosureManifestV1,
    moment: SimMoment,
    additions: usize,
) -> Result<(), AuthorityRecordSealError> {
    let actual = head
        .scheduler()
        .entry_count_at(moment)
        .checked_add(additions)
        .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
    let maximum = closure.semantics().config().maximum_work_per_moment().get();
    if actual > usize::try_from(maximum).unwrap_or(usize::MAX) {
        return Err(AuthorityRecordSealError::WorkPopulationExceeded {
            moment,
            maximum,
            actual,
        });
    }
    Ok(())
}

fn ensure_scheduler_work_population(
    scheduler: &SchedulerState,
    closure: &ResolvedExecutionClosureManifestV1,
    moment: SimMoment,
    additions: usize,
) -> Result<(), AuthorityRecordSealError> {
    let actual = scheduler
        .entry_count_at(moment)
        .checked_add(additions)
        .ok_or(AuthorityRecordSealError::CollectionTooLarge)?;
    let maximum = closure.semantics().config().maximum_work_per_moment().get();
    if actual > usize::try_from(maximum).unwrap_or(usize::MAX) {
        return Err(AuthorityRecordSealError::WorkPopulationExceeded {
            moment,
            maximum,
            actual,
        });
    }
    Ok(())
}

fn validate_action_command(
    head: &SessionHead,
    opportunity: world_model::ActionOpportunityId,
    command: &world_model::CommandEnvelope,
    closure: &ResolvedExecutionClosureManifestV1,
) -> Result<(), AuthorityRecordSealError> {
    if command.source() != CommandSource::derive_action(opportunity)
        || command.id() != CommandId::new(0)
        || command.definition_set_digest() != closure.semantics().definition_set_digest()
    {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    let Some(authoritative) = head
        .runtime_control()
        .action_opportunities()
        .get(opportunity)
    else {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    };
    let Some(transfer) = ScopedContainmentTransfer::resolve(command, closure) else {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    };
    if !transfer.is_authorized_by(authoritative) {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    Ok(())
}

/// Exact opportunity-relevant roles of one activated containment transfer.
///
/// Runtime reconstructs this narrow view from the sealed definition closure
/// instead of trusting engine-supplied scope or family metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScopedContainmentTransfer {
    actor: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
}

impl ScopedContainmentTransfer {
    fn resolve(
        command: &CommandEnvelope,
        closure: &ResolvedExecutionClosureManifestV1,
    ) -> Option<Self> {
        const ACTOR_ROLE: &str = "actor";
        const ITEM_ROLE: &str = "item";
        const SOURCE_ROLE: &str = "source";
        const DESTINATION_ROLE: &str = "destination";

        let semantics = closure.semantics();
        let [family] = semantics.required_interfaces() else {
            return None;
        };
        let action = semantics.definitions().action(command.action())?;
        let [requirement] = action.requirements() else {
            return None;
        };
        let [effect] = action.effects() else {
            return None;
        };
        let requirement = requirement.call();
        let effect = effect.call();
        if requirement.interface() != family.key()
            || effect.interface() != family.key()
            || requirement.arguments() != effect.arguments()
            || !matches!(
                effect.arguments(),
                [actor, item, source, destination]
                    if actor.as_str() == ACTOR_ROLE
                        && item.as_str() == ITEM_ROLE
                        && source.as_str() == SOURCE_ROLE
                        && destination.as_str() == DESTINATION_ROLE
            )
        {
            return None;
        }

        let actor = actor_command_binding(command, ACTOR_ROLE)?;
        let item = entity_command_binding(command, ITEM_ROLE)?;
        let source = entity_command_binding(command, SOURCE_ROLE)?;
        let destination = entity_command_binding(command, DESTINATION_ROLE)?;
        (actor == command.actor()).then_some(Self {
            actor,
            item,
            source,
            destination,
        })
    }

    fn is_authorized_by(self, opportunity: &ActionOpportunity) -> bool {
        let Some(scope) = opportunity.interaction_scope().containment_scope() else {
            return false;
        };
        self.actor == opportunity.actor()
            && scope.permits_item(self.item)
            && self.source == scope.source()
            && scope
                .destinations()
                .binary_search(&self.destination)
                .is_ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RejectedContainmentFeedback {
    actor: ActorId,
    item: EntityId,
    expected_container: EntityId,
}

fn rejected_containment_feedback(
    head: &SessionHead,
    closure: &ResolvedExecutionClosureManifestV1,
    attempts: &[NormalizedAttemptRecord],
    attempt: AttemptRecordRef,
    opportunity: world_model::ActionOpportunityId,
    command: &CommandEnvelope,
) -> Result<Option<RejectedContainmentFeedback>, AuthorityRecordSealError> {
    let normalized = attempts
        .get(
            usize::try_from(attempt.index())
                .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
        )
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
    if normalized.subject != NormalizedAttemptSubject::EvaluatedCommand(command.clone()) {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    if !matches!(
        normalized.resolution,
        NormalizedAttemptResolution::Rejected(
            StableCommandRejection::Stale | StableCommandRejection::RequirementUnsatisfied
        )
    ) {
        return Ok(None);
    }
    let authoritative = head
        .runtime_control()
        .action_opportunities()
        .get(opportunity)
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
    let transfer = ScopedContainmentTransfer::resolve(command, closure)
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
    if !transfer.is_authorized_by(authoritative) {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    let Some(belief) = head
        .accepted()
        .epistemic()
        .contained_in(transfer.actor, transfer.item)
    else {
        return Ok(None);
    };
    if belief.container() != transfer.source
        || head
            .accepted()
            .domain()
            .containment_for(transfer.item)
            .map(|record| record.container())
            == Some(transfer.source)
    {
        return Ok(None);
    }
    Ok(Some(RejectedContainmentFeedback {
        actor: transfer.actor,
        item: transfer.item,
        expected_container: belief.container(),
    }))
}

fn next_evidence_generation(
    cursor: &mut BTreeMap<ActorId, u64>,
    actor: ActorId,
) -> Result<EvidenceDeliveryGeneration, AuthorityRecordSealError> {
    let next = cursor
        .get(&actor)
        .copied()
        .unwrap_or(0)
        .checked_add(1)
        .and_then(EvidenceDeliveryGeneration::new)
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
    cursor.insert(actor, next.get());
    Ok(next)
}

fn actor_command_binding(command: &CommandEnvelope, role: &str) -> Option<ActorId> {
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

fn entity_command_binding(command: &CommandEnvelope, role: &str) -> Option<EntityId> {
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

fn verify_resolution_evidence(
    evidence: &ContainmentResolutionEvidence,
    closure: &ResolvedExecutionClosureManifestV1,
    moment: SimMoment,
    attempts: &[DraftAttemptRecord],
) -> Result<(), AuthorityRecordSealError> {
    let config = closure.semantics().config();
    if evidence.resolution_policy() != config.moment_resolution_policy()
        || evidence.conflict_policy() != config.containment_conflict_policy()
        || evidence.random_oracle_policy() != config.random_oracle_policy()
        || evidence.random_key_policy() != config.random_key_policy()
    {
        return Err(AuthorityRecordSealError::ResolutionEvidenceMismatch);
    }
    let oracle = Blake3KeyedPrf256V1::from_root_seed(closure.specification().root_seed());

    let attempt_contenders = attempts
        .iter()
        .filter_map(|attempt| {
            attempt.command().map(|command| {
                ContainmentConflictContenderV1::new(command.actor(), command.source(), command.id())
            })
        })
        .collect::<BTreeSet<_>>();
    let mut represented_contenders = BTreeSet::new();
    let mut previous_component = None;
    for component in evidence.components() {
        let Some(first) = component.contenders().first().copied() else {
            return Err(AuthorityRecordSealError::ResolutionEvidenceMismatch);
        };
        if previous_component.is_some_and(|previous| previous >= first)
            || !is_strictly_sorted(component.contenders())
        {
            return Err(AuthorityRecordSealError::ResolutionEvidenceMismatch);
        }
        previous_component = Some(first);
        for contender in component.contenders() {
            if !attempt_contenders.contains(contender) || !represented_contenders.insert(*contender)
            {
                return Err(AuthorityRecordSealError::ResolutionEvidenceMismatch);
            }
        }

        let mut previous_group = None;
        if component.resources().is_empty() {
            return Err(AuthorityRecordSealError::ResolutionEvidenceMismatch);
        }
        for resource in component.resources() {
            let group = resource.group();
            if group.moment() != moment || previous_group.is_some_and(|previous| previous >= group)
            {
                return Err(AuthorityRecordSealError::ResolutionEvidenceMismatch);
            }
            previous_group = Some(group);

            let entries = resource.ranking().entries();
            if entries.is_empty()
                || entries.len()
                    <= usize::try_from(resource.admission_limit()).unwrap_or(usize::MAX)
            {
                return Err(AuthorityRecordSealError::ResolutionEvidenceMismatch);
            }
            let mut actual_contenders = BTreeSet::new();
            let mut key_ids = BTreeSet::new();
            let mut scores = BTreeSet::new();
            let mut winning = None;
            let mut previous_contender = None;
            for entry in entries {
                let key = entry.key();
                let contender = key.contender();
                if key.group() != group
                    || key.draw_ordinal() != 0
                    || key.key_policy_version() != 1
                    || key.id() != entry.key_id()
                    || oracle.score(key) != entry.score()
                    || !component.contenders().contains(&contender)
                    || !actual_contenders.insert(contender)
                    || !key_ids.insert(entry.key_id())
                    || !scores.insert(entry.score())
                    || previous_contender.is_some_and(|previous| previous >= contender)
                {
                    return Err(AuthorityRecordSealError::ResolutionEvidenceMismatch);
                }
                previous_contender = Some(contender);
                if winning.is_none_or(|(score, _)| entry.score() > score) {
                    winning = Some((entry.score(), contender));
                }
            }
            if actual_contenders.len() != entries.len()
                || winning.map(|(_, contender)| contender) != Some(resource.ranking().winner())
            {
                return Err(AuthorityRecordSealError::ResolutionEvidenceMismatch);
            }
        }
    }

    match evidence.fallback() {
        Some(ContainmentResolutionFallback::RandomEvidence {
            group,
            error:
                ContainmentRandomRankError::EmptyConflictGroup
                | ContainmentRandomRankError::SemanticKeyReuse { .. }
                | ContainmentRandomRankError::ScoreCollision { .. },
            ..
        }) if group.moment() == moment => {}
        Some(ContainmentResolutionFallback::CombinedTransition { .. }) | None => {}
        Some(ContainmentResolutionFallback::RandomEvidence { .. }) => {
            return Err(AuthorityRecordSealError::ResolutionEvidenceMismatch);
        }
    }
    Ok(())
}

fn is_strictly_sorted<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn verify_normalized_moment(
    head: &SessionHead,
    closure: &ResolvedExecutionClosureManifestV1,
    batch: &NormalizedMomentBatch,
) -> Result<(), AuthorityRecordSealError> {
    if batch.resolutions.len()
        != batch.command_deliveries.len()
            + batch.post_commit_deliveries.len()
            + batch.lifecycle_deliveries.len()
            + batch.action_ready_deliveries.len()
            + batch.action_evaluation_deliveries.len()
            + batch.attempt_resolved_deliveries.len()
            + batch.relocation_process_deliveries.len()
    {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }

    let mut command_coverage = vec![false; batch.command_deliveries.len()];
    let mut post_commit_coverage = vec![false; batch.post_commit_deliveries.len()];
    let mut lifecycle_coverage = vec![false; batch.lifecycle_deliveries.len()];
    let mut action_ready_coverage = vec![false; batch.action_ready_deliveries.len()];
    let mut action_evaluation_coverage = vec![false; batch.action_evaluation_deliveries.len()];
    let mut attempt_resolved_coverage = vec![false; batch.attempt_resolved_deliveries.len()];
    let mut relocation_process_coverage = vec![false; batch.relocation_process_deliveries.len()];
    let mut attempt_coverage = vec![false; batch.attempts.len()];
    let mut collision_fingerprint_coverage = vec![BTreeSet::new(); batch.attempts.len()];
    let mut attempt_identities = BTreeSet::new();
    for attempt in &batch.attempts {
        if !attempt_identities.insert(attempt.subject.identity())
            || !matches!(
                (&attempt.subject, attempt.resolution),
                (
                    NormalizedAttemptSubject::EvaluatedCommand(_),
                    NormalizedAttemptResolution::Accepted { .. }
                        | NormalizedAttemptResolution::Rejected(_)
                ) | (
                    NormalizedAttemptSubject::CommandIdCollision { .. },
                    NormalizedAttemptResolution::CommandIdCollision
                )
            )
        {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        if let NormalizedAttemptSubject::CommandIdCollision { fingerprints, .. } = &attempt.subject
            && (fingerprints.len() < 2 || !is_strictly_sorted(fingerprints))
        {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }
    for resolution in &batch.resolutions {
        match *resolution {
            NormalizedDeliveryResolution::NewCommand { delivery, attempt } => {
                mark_once(&mut command_coverage, delivery.index())?;
                mark_present(&mut attempt_coverage, attempt.position())?;
                let delivery = batch
                    .command_deliveries
                    .get(
                        usize::try_from(delivery.index())
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
                    )
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                let attempt = batch
                    .attempts
                    .get(
                        usize::try_from(attempt.position())
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
                    )
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                if attempt.subject
                    != NormalizedAttemptSubject::EvaluatedCommand(delivery.command().clone())
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
            NormalizedDeliveryResolution::RetainedCommand { delivery, .. }
            | NormalizedDeliveryResolution::CommandIdReuseMismatch { delivery, .. }
            | NormalizedDeliveryResolution::RetainedCollision { delivery, .. }
            | NormalizedDeliveryResolution::RetiredCommand { delivery } => {
                mark_once(&mut command_coverage, delivery.index())?;
            }
            NormalizedDeliveryResolution::NewCollision { delivery, attempt } => {
                mark_once(&mut command_coverage, delivery.index())?;
                mark_present(&mut attempt_coverage, attempt.position())?;
                let delivery = batch
                    .command_deliveries
                    .get(
                        usize::try_from(delivery.index())
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
                    )
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                let attempt_position = usize::try_from(attempt.position())
                    .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?;
                let attempt = batch
                    .attempts
                    .get(attempt_position)
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                match &attempt.subject {
                    NormalizedAttemptSubject::CommandIdCollision {
                        source,
                        command,
                        fingerprints,
                    } if *source == delivery.command().source()
                        && *command == delivery.command().id()
                        && fingerprints
                            .binary_search(&delivery.command().fingerprint())
                            .is_ok() =>
                    {
                        collision_fingerprint_coverage[attempt_position]
                            .insert(delivery.command().fingerprint());
                    }
                    _ => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
                }
            }
            NormalizedDeliveryResolution::PostCommitConsumed { delivery } => {
                mark_once(&mut post_commit_coverage, delivery.index())?;
            }
            NormalizedDeliveryResolution::ActionReadyConsumed { delivery } => {
                mark_once(&mut action_ready_coverage, delivery.index())?;
            }
            NormalizedDeliveryResolution::ActionEvaluationConsumed { delivery } => {
                mark_once(&mut action_evaluation_coverage, delivery.index())?;
            }
            NormalizedDeliveryResolution::AttemptResolvedConsumed { delivery } => {
                mark_once(&mut attempt_resolved_coverage, delivery.index())?;
            }
            NormalizedDeliveryResolution::LifecycleConsumed { delivery } => {
                mark_once(&mut lifecycle_coverage, delivery.index())?;
            }
            NormalizedDeliveryResolution::RelocationProcessCompleted { delivery }
            | NormalizedDeliveryResolution::ObsoleteRelocationWake { delivery } => {
                mark_once(&mut relocation_process_coverage, delivery.index())?;
            }
        }
    }
    if command_coverage.iter().any(|covered| !covered)
        || post_commit_coverage.iter().any(|covered| !covered)
        || lifecycle_coverage.iter().any(|covered| !covered)
        || action_ready_coverage.iter().any(|covered| !covered)
        || action_evaluation_coverage.iter().any(|covered| !covered)
        || attempt_resolved_coverage.iter().any(|covered| !covered)
        || relocation_process_coverage.iter().any(|covered| !covered)
        || attempt_coverage.iter().any(|covered| !covered)
    {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    for (attempt, covered) in batch.attempts.iter().zip(&collision_fingerprint_coverage) {
        if let NormalizedAttemptSubject::CommandIdCollision { fingerprints, .. } = &attempt.subject
            && covered.iter().copied().ne(fingerprints.iter().copied())
        {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }

    let mut commit_coverage = vec![false; batch.commits.len()];
    for attempt in &batch.attempts {
        if let NormalizedAttemptResolution::Accepted { commit } = attempt.resolution {
            mark_present(&mut commit_coverage, commit.position())?;
        }
    }
    if commit_coverage.iter().any(|covered| !covered) || batch.containment_delta != batch.commits {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }

    let mut delivery_keys = batch
        .command_deliveries
        .iter()
        .map(CommandDeliveryRecord::scheduler_key)
        .chain(
            batch
                .lifecycle_deliveries
                .iter()
                .copied()
                .map(LifecycleDeliveryRecord::scheduler_key),
        )
        .chain(
            batch
                .post_commit_deliveries
                .iter()
                .map(PostCommitDeliveryRecord::scheduler_key),
        )
        .chain(
            batch
                .action_ready_deliveries
                .iter()
                .copied()
                .map(ActionReadyDeliveryRecord::scheduler_key),
        )
        .chain(
            batch
                .action_evaluation_deliveries
                .iter()
                .copied()
                .map(ActionEvaluationDeliveryRecord::scheduler_key),
        )
        .chain(
            batch
                .attempt_resolved_deliveries
                .iter()
                .copied()
                .map(AttemptResolvedDeliveryRecord::scheduler_key),
        )
        .chain(
            batch
                .relocation_process_deliveries
                .iter()
                .copied()
                .map(RelocationProcessDeliveryRecord::scheduler_key),
        )
        .collect::<Vec<_>>();
    delivery_keys.sort();
    if delivery_keys != batch.consumed_keys {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }

    for transition in &batch.activity_transitions {
        if transition.after().status().is_terminal() {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        activity_transition_between(transition.before(), transition.after())?;
    }
    for transition in &batch.activity_terminal_transitions {
        activity_terminal_transitions_between(
            transition.activity_before(),
            transition.activity_after(),
            transition.intent_before(),
            transition.intent_after(),
        )?;
    }

    verify_action_transition_graph(head, batch)?;
    verify_relocation_transition_graph(batch)?;
    verify_evidence_routing_graph(head, closure, batch)?;
    verify_scheduler_consequences(batch)
}

fn verify_action_transition_graph(
    head: &SessionHead,
    batch: &NormalizedMomentBatch,
) -> Result<(), AuthorityRecordSealError> {
    let mut opening_invocations = BTreeSet::new();
    for opening in &batch.action_evaluation_invocation_openings {
        let invocation = opening.invocation();
        if !opening_invocations.insert(invocation.invocation()) {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        match opening.cause() {
            ActionEvaluationInvocationOpeningCause::ActionReady(delivery) => {
                let source = batch
                    .action_ready_deliveries
                    .get(
                        usize::try_from(delivery.index())
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
                    )
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                if source.ready().opportunity() != invocation.opportunity() {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
            ActionEvaluationInvocationOpeningCause::VisibleReinvocation(delivery) => {
                let source = batch
                    .action_evaluation_deliveries
                    .get(
                        usize::try_from(delivery.index())
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
                    )
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                if source.work().opportunity() != invocation.opportunity() {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
                let predecessor = batch
                    .action_evaluation_invocation_transitions
                    .iter()
                    .find(|transition| {
                        transition.cause()
                            == ActionEvaluationInvocationTransitionCause::EvaluationDelivery(
                                delivery,
                            )
                    })
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                if !matches!(
                    predecessor.after().state(),
                    ActionEvaluationInvocationState::Terminal(
                        ActionEvaluationTerminal::Reinvoked { successor, .. }
                    ) if *successor == invocation.invocation()
                ) {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
        }
    }

    let mut transitioned_invocations = BTreeSet::new();
    for transition in &batch.action_evaluation_invocation_transitions {
        let ActionEvaluationInvocationTransitionCause::EvaluationDelivery(delivery) =
            transition.cause()
        else {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        };
        let source = batch
            .action_evaluation_deliveries
            .get(
                usize::try_from(delivery.index())
                    .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
            )
            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
        let invocation = source.work().invocation();
        let before = head
            .runtime_control()
            .action_evaluations()
            .get(invocation)
            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
        if !transitioned_invocations.insert(invocation)
            || transition.expected_before() != before.digest()
            || transition.after().invocation() != invocation
        {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        match (source.work(), transition.after().state()) {
            (
                ActionEvaluationWork::ResultReady { .. },
                ActionEvaluationInvocationState::Terminal(
                    ActionEvaluationTerminal::Applied { .. }
                    | ActionEvaluationTerminal::Reinvoked { .. },
                )
                | ActionEvaluationInvocationState::FallbackPending { .. },
            ) => {}
            (
                ActionEvaluationWork::Fallback { cause, .. },
                ActionEvaluationInvocationState::Terminal(ActionEvaluationTerminal::Failed {
                    cause: retained,
                }),
            ) if cause == *retained => {}
            _ => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
        }
    }
    if transitioned_invocations.len() != batch.action_evaluation_deliveries.len() {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }

    let mut current = BTreeMap::<ActionOpportunityId, ActionOpportunity>::new();
    let mut waiting_openings = BTreeSet::new();
    for transition in &batch.action_opportunity_transitions {
        let before = transition.before();
        let after = transition.after();
        let expected = current.entry(before.id()).or_insert_with(|| {
            head.runtime_control()
                .action_opportunities()
                .get(before.id())
                .cloned()
                .unwrap_or_else(|| before.clone())
        });
        if expected != before || before.id() != after.id() {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        let valid = match (before.state(), after.state()) {
            (
                ActionOpportunityState::Open,
                ActionOpportunityState::WaitingForEvaluation(invocation),
            ) => {
                let Some(opening) = batch
                    .action_evaluation_invocation_openings
                    .iter()
                    .find(|opening| opening.invocation().invocation() == invocation)
                else {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                };
                waiting_openings.insert(invocation)
                    && before
                        .begin_evaluation(
                            before.version(),
                            *opening.invocation().policy_semantics(),
                            *opening.invocation().action_input_fingerprint(),
                        )
                        .is_ok_and(|(candidate, derived)| {
                            candidate == *after && derived == invocation
                        })
            }
            (
                ActionOpportunityState::WaitingForEvaluation(invocation),
                ActionOpportunityState::Open,
            ) if before.evaluation_generation() == after.evaluation_generation() => {
                before
                    .resume_evaluation(before.version(), invocation)
                    .as_ref()
                    == Ok(after)
            }
            (
                ActionOpportunityState::WaitingForEvaluation(invocation),
                ActionOpportunityState::Open,
            ) => {
                before
                    .reopen_for_visible_reinvocation(before.version(), invocation)
                    .as_ref()
                    == Ok(after)
            }
            (ActionOpportunityState::Open, ActionOpportunityState::Consumed(disposition)) => {
                before.consume(before.version(), disposition).as_ref() == Ok(after)
            }
            _ => false,
        };
        if !valid {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        *expected = after.clone();
    }
    if waiting_openings != opening_invocations {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }

    for delivery in &batch.action_ready_deliveries {
        let ready = delivery.ready();
        let Some(before) = head
            .runtime_control()
            .action_opportunities()
            .get(ready.opportunity())
        else {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        };
        if before.version() != ready.expected_version()
            || !batch
                .action_opportunity_transitions
                .iter()
                .any(|transition| transition.before() == before)
        {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }
    for (position, delivery) in batch.action_evaluation_deliveries.iter().enumerate() {
        let reference = ActionEvaluationDeliveryRef::from_position(position)
            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
        let transition = batch
            .action_evaluation_invocation_transitions
            .iter()
            .find(|transition| {
                transition.cause()
                    == ActionEvaluationInvocationTransitionCause::EvaluationDelivery(reference)
            })
            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
        let opportunity = current
            .get(&delivery.work().opportunity())
            .or_else(|| {
                head.runtime_control()
                    .action_opportunities()
                    .get(delivery.work().opportunity())
            })
            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
        let valid = match transition.after().state() {
            ActionEvaluationInvocationState::Terminal(ActionEvaluationTerminal::Applied {
                ..
            }) => matches!(
                opportunity.state(),
                ActionOpportunityState::Consumed(
                    ActionOpportunityDisposition::ActionSubmitted
                        | ActionOpportunityDisposition::NoApplicableAction
                )
            ),
            ActionEvaluationInvocationState::Terminal(ActionEvaluationTerminal::Reinvoked {
                successor,
                ..
            }) => opportunity.state() == ActionOpportunityState::WaitingForEvaluation(*successor),
            ActionEvaluationInvocationState::Terminal(ActionEvaluationTerminal::Failed {
                ..
            }) => {
                opportunity.state()
                    == ActionOpportunityState::Consumed(ActionOpportunityDisposition::Failed)
            }
            ActionEvaluationInvocationState::FallbackPending { .. } => {
                opportunity.state()
                    == ActionOpportunityState::WaitingForEvaluation(delivery.work().invocation())
            }
            ActionEvaluationInvocationState::DispatchPending
            | ActionEvaluationInvocationState::ResultCaptured { .. } => false,
        };
        if !valid {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }
    Ok(())
}

fn verify_relocation_transition_graph(
    batch: &NormalizedMomentBatch,
) -> Result<(), AuthorityRecordSealError> {
    let mut expected_action_attempts = BTreeSet::new();
    let mut source_by_opportunity = BTreeMap::new();
    for (position, delivery) in batch.action_ready_deliveries.iter().enumerate() {
        let reference = ActionReadyDeliveryRef::from_position(position)
            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
        if source_by_opportunity
            .insert(
                delivery.ready().opportunity(),
                ActionResolutionDeliveryRef::Ready(reference),
            )
            .is_some()
        {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }
    for (position, delivery) in batch.action_evaluation_deliveries.iter().enumerate() {
        let reference = ActionEvaluationDeliveryRef::from_position(position)
            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
        if source_by_opportunity
            .insert(
                delivery.work().opportunity(),
                ActionResolutionDeliveryRef::Evaluation(reference),
            )
            .is_some()
        {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }
    for transition in &batch.action_opportunity_transitions {
        if matches!(
            transition.after().state(),
            ActionOpportunityState::Consumed(ActionOpportunityDisposition::ActionSubmitted)
        ) && transition
            .before()
            .interaction_scope()
            .relocation_scope()
            .is_some()
        {
            let delivery = source_by_opportunity
                .get(&transition.before().id())
                .copied()
                .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
            if !expected_action_attempts.insert(delivery) {
                return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
            }
        }
    }

    let mut accepted_action_attempts = BTreeMap::new();
    for attempt in &batch.relocation_attempts {
        let delivery = attempt.resolution_delivery();
        if !expected_action_attempts.remove(&delivery) {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        let opportunity_id = match delivery {
            ActionResolutionDeliveryRef::Ready(reference) => batch
                .action_ready_deliveries
                .get(
                    usize::try_from(reference.index())
                        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
                )
                .map(|record| record.ready().opportunity()),
            ActionResolutionDeliveryRef::Evaluation(reference) => batch
                .action_evaluation_deliveries
                .get(
                    usize::try_from(reference.index())
                        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
                )
                .map(|record| record.work().opportunity()),
        }
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
        let opportunity = batch
            .action_opportunity_transitions
            .iter()
            .find(|transition| {
                transition.before().id() == opportunity_id
                    && matches!(
                        transition.after().state(),
                        ActionOpportunityState::Consumed(
                            ActionOpportunityDisposition::ActionSubmitted
                        )
                    )
            })
            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?
            .before();
        if !opportunity
            .interaction_scope()
            .relocation_scope()
            .is_some_and(|scope| scope.permits(attempt.interaction()))
        {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        if let RelocationAttemptResolution::Accepted { process } = attempt.resolution()
            && accepted_action_attempts
                .insert(process, (delivery, attempt.interaction()))
                .is_some()
        {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }
    if !expected_action_attempts.is_empty() {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }

    let mut completion_wakes = BTreeMap::new();
    for resolution in &batch.resolutions {
        if let NormalizedDeliveryResolution::RelocationProcessCompleted { delivery } = *resolution {
            let record = batch
                .relocation_process_deliveries
                .get(
                    usize::try_from(delivery.index())
                        .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
                )
                .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
            if completion_wakes
                .insert(record.wake().process(), (delivery, record.wake()))
                .is_some()
            {
                return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
            }
        }
    }

    let mut transitioned = BTreeSet::new();
    for transition in &batch.relocation_process_transitions {
        let after = transition.after();
        if !transitioned.insert(after.id()) {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        match (transition.cause(), transition.before(), after.status()) {
            (
                RelocationProcessTransitionCause::Action(delivery),
                None,
                RelocationProcessStatus::Active { .. },
            ) => {
                if accepted_action_attempts.remove(&after.id())
                    != Some((delivery, RelocationInteraction::Start(after.route().id())))
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
                if world_model::RelocationProcess::start(
                    after.actor(),
                    after.route(),
                    after.generation(),
                    batch.moment.time(),
                )
                .as_ref()
                    != Ok(&after)
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
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
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
            (
                RelocationProcessTransitionCause::Action(delivery),
                Some(before),
                RelocationProcessStatus::Paused { .. },
            ) if before.id() == after.id()
                && before.pause(before.version(), batch.moment.time()).as_ref() == Ok(&after) =>
            {
                if accepted_action_attempts.remove(&after.id())
                    != Some((delivery, RelocationInteraction::Pause(after.route().id())))
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
                if transition.event().is_some() {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
            (
                RelocationProcessTransitionCause::Action(delivery),
                Some(before),
                RelocationProcessStatus::Active { .. },
            ) if before.id() == after.id()
                && before
                    .resume(before.version(), batch.moment.time())
                    .as_ref()
                    == Ok(&after) =>
            {
                if accepted_action_attempts.remove(&after.id())
                    != Some((delivery, RelocationInteraction::Resume(after.route().id())))
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
                if transition.event().is_some() {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
            (
                RelocationProcessTransitionCause::Wake(delivery),
                Some(before),
                RelocationProcessStatus::Completed { .. },
            ) if before.id() == after.id() => {
                let (expected_delivery, wake) = completion_wakes
                    .remove(&after.id())
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                if delivery != expected_delivery
                    || before
                        .complete(
                            wake.expected_version(),
                            wake.wake_generation(),
                            batch.moment.time(),
                        )
                        .as_ref()
                        != Ok(&after)
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
                let route = after.route();
                if transition.event()
                    != Some(PhysicalEvent::actor_arrived(
                        after.id(),
                        after.actor(),
                        route.source(),
                        route.destination(),
                    ))
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
            _ => return Err(AuthorityRecordSealError::InvalidNormalizedGraph),
        }
    }
    if !accepted_action_attempts.is_empty() || !completion_wakes.is_empty() {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    Ok(())
}

fn verify_evidence_routing_graph(
    head: &SessionHead,
    closure: &ResolvedExecutionClosureManifestV1,
    batch: &NormalizedMomentBatch,
) -> Result<(), AuthorityRecordSealError> {
    let mut expected_feedback = BTreeMap::new();
    for resolution in &batch.resolutions {
        let NormalizedDeliveryResolution::NewCommand { delivery, attempt } = *resolution else {
            continue;
        };
        let scheduled = batch
            .command_deliveries
            .get(
                usize::try_from(delivery.index())
                    .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
            )
            .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?
            .scheduled();
        let Some(opportunity) = scheduled.action_opportunity() else {
            continue;
        };
        if let Some(feedback) = rejected_containment_feedback(
            head,
            closure,
            &batch.attempts,
            attempt,
            opportunity,
            scheduled.command(),
        )? && expected_feedback
            .insert(attempt, feedback)
            .is_some_and(|previous| previous != feedback)
        {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }

    let mut sources = BTreeSet::new();
    let mut evidence_ids = BTreeSet::new();
    for routing in &batch.evidence_routing {
        let source = routing.source();
        let evidence = routing.evidence();
        if !sources.insert(source) || !evidence_ids.insert(evidence.id()) {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
        match source {
            EvidenceRoutingSource::PhysicalEvent {
                dispatch,
                event_index,
            } => {
                let event = batch
                    .post_commit_deliveries
                    .get(
                        usize::try_from(dispatch.index())
                            .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
                    )
                    .and_then(|delivery| {
                        delivery
                            .dispatch()
                            .reaction()
                            .events()
                            .get(usize::try_from(event_index).ok()?)
                    })
                    .copied()
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                if evidence.observer() != event.actor()
                    || world_model::EvidenceRecord::direct_physical_event(
                        evidence.observer(),
                        evidence.generation(),
                        event,
                    ) != evidence
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
            EvidenceRoutingSource::RejectedContainmentAttempt { attempt } => {
                let feedback = expected_feedback
                    .remove(&attempt)
                    .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
                if world_model::EvidenceRecord::direct_item_absent(
                    feedback.actor,
                    evidence.generation(),
                    feedback.item,
                    feedback.expected_container,
                ) != evidence
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
        }
    }
    if !expected_feedback.is_empty() {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    Ok(())
}

fn verify_scheduler_consequences(
    batch: &NormalizedMomentBatch,
) -> Result<(), AuthorityRecordSealError> {
    let later = strictly_later_moment(batch.moment).map_err(map_post_commit_error)?;
    if !batch
        .scheduler_insertions
        .windows(2)
        .all(|pair| pair[0].scheduler_key() < pair[1].scheduler_key())
    {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }

    let mut submitted = BTreeMap::new();
    let mut expected_wakes = BTreeSet::new();
    let mut expected_process_wakes = BTreeSet::new();
    let mut expected_action_evaluations = BTreeMap::new();
    for transition in &batch.action_opportunity_transitions {
        let ActionOpportunityState::Consumed(disposition) = transition.after().state() else {
            continue;
        };
        match disposition {
            ActionOpportunityDisposition::ActionSubmitted => {
                if transition
                    .before()
                    .interaction_scope()
                    .relocation_scope()
                    .is_some()
                {
                    expected_wakes.insert(transition.before().id());
                } else {
                    submitted.insert(transition.before().id(), transition.before().actor());
                }
            }
            ActionOpportunityDisposition::NoApplicableAction
            | ActionOpportunityDisposition::Failed => {
                expected_wakes.insert(transition.before().id());
            }
        }
    }
    for delivery in &batch.command_deliveries {
        if let Some(opportunity) = delivery.scheduled().action_opportunity()
            && !expected_wakes.insert(opportunity)
        {
            return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
        }
    }
    for transition in &batch.relocation_process_transitions {
        let after = transition.after();
        if matches!(after.status(), RelocationProcessStatus::Active { .. })
            && let Some(wake) = RelocationProcessWake::for_active(after)
        {
            expected_process_wakes.insert((
                wake.process(),
                wake.process_generation(),
                wake.expected_version(),
                wake.wake_generation(),
                wake.due(),
            ));
        }
    }
    for opening in &batch.action_evaluation_invocation_openings {
        let invocation = opening.invocation();
        if let ActionEvaluationInvocationState::FallbackPending {
            cause,
            scheduler_key,
        } = invocation.state()
        {
            let work = ActionEvaluationWork::fallback(
                invocation.invocation(),
                invocation.opportunity(),
                invocation.waiting_version(),
                *cause,
                scheduler_key.moment(),
            );
            if expected_action_evaluations
                .insert(*scheduler_key, work)
                .is_some()
            {
                return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
            }
        }
    }
    for transition in &batch.action_evaluation_invocation_transitions {
        let invocation = transition.after();
        if let ActionEvaluationInvocationState::FallbackPending {
            cause,
            scheduler_key,
        } = invocation.state()
        {
            let work = ActionEvaluationWork::fallback(
                invocation.invocation(),
                invocation.opportunity(),
                invocation.waiting_version(),
                *cause,
                scheduler_key.moment(),
            );
            if expected_action_evaluations
                .insert(*scheduler_key, work)
                .is_some()
            {
                return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
            }
        }
    }

    let mut actual_commands = BTreeSet::new();
    let mut actual_wakes = BTreeSet::new();
    let mut actual_process_wakes = BTreeSet::new();
    let mut actual_evidence = BTreeSet::new();
    let mut actual_action_ready = BTreeSet::new();
    let mut post_commit = None;
    for insertion in &batch.scheduler_insertions {
        match insertion {
            NormalizedSchedulerInsertion::ActionCommand {
                opportunity,
                effective,
                command,
                ..
            } => {
                let Some(actor) = submitted.get(opportunity) else {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                };
                if *effective != later
                    || command.source() != CommandSource::derive_action(*opportunity)
                    || command.id() != CommandId::new(0)
                    || command.actor() != *actor
                    || !actual_commands.insert(*opportunity)
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
            NormalizedSchedulerInsertion::AttemptResolved { resolved, .. } => {
                if resolved.due() != later || !actual_wakes.insert(resolved.opportunity()) {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
            NormalizedSchedulerInsertion::PostCommit {
                reaction, dispatch, ..
            } => {
                if post_commit.replace((*reaction, dispatch)).is_some() {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
            NormalizedSchedulerInsertion::RelocationProcess { wake, .. } => {
                actual_process_wakes.insert((
                    wake.process(),
                    wake.process_generation(),
                    wake.expected_version(),
                    wake.wake_generation(),
                    wake.due(),
                ));
            }
            NormalizedSchedulerInsertion::Lifecycle { work, .. } => {
                if work.due() <= batch.moment {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
                if let LifecycleWork::EvidenceDelivery(delivery) = work {
                    actual_evidence.insert(delivery.evidence().id());
                }
            }
            NormalizedSchedulerInsertion::ActionReady { ready, .. } => {
                if ready.due() != later || !actual_action_ready.insert(ready.opportunity()) {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
            NormalizedSchedulerInsertion::ActionEvaluation {
                scheduler_key,
                work,
            } => {
                if work.due() <= batch.moment
                    || expected_action_evaluations.remove(scheduler_key) != Some(*work)
                {
                    return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
                }
            }
        }
    }
    if actual_commands != submitted.keys().copied().collect()
        || actual_wakes != expected_wakes
        || actual_process_wakes != expected_process_wakes
        || !expected_action_evaluations.is_empty()
    {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    if actual_evidence
        != batch
            .evidence_routing
            .iter()
            .map(|routing| routing.evidence().id())
            .collect()
        || actual_action_ready
            != batch
                .action_opportunity_openings
                .iter()
                .map(|opening| opening.opportunity().id())
                .collect()
    {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    let expected_events = batch
        .commits
        .iter()
        .copied()
        .map(PhysicalEvent::item_transferred)
        .chain(
            batch
                .relocation_process_transitions
                .iter()
                .filter_map(|transition| transition.event()),
        )
        .collect::<Vec<_>>();
    match (expected_events.is_empty(), post_commit) {
        (true, None) if batch.reactions.is_empty() => Ok(()),
        (false, Some((reaction, dispatch)))
            if batch.reactions.len() == 1
                && reaction.position() == 0
                && dispatch.reaction() == &batch.reactions[0]
                && batch.reactions[0].events() == expected_events =>
        {
            Ok(())
        }
        _ => Err(AuthorityRecordSealError::InvalidNormalizedGraph),
    }
}

fn mark_once(coverage: &mut [bool], position: u32) -> Result<(), AuthorityRecordSealError> {
    let entry = coverage
        .get_mut(
            usize::try_from(position)
                .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
        )
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
    if *entry {
        return Err(AuthorityRecordSealError::InvalidNormalizedGraph);
    }
    *entry = true;
    Ok(())
}

fn mark_present(coverage: &mut [bool], position: u32) -> Result<(), AuthorityRecordSealError> {
    let entry = coverage
        .get_mut(
            usize::try_from(position)
                .map_err(|_| AuthorityRecordSealError::InvalidNormalizedGraph)?,
        )
        .ok_or(AuthorityRecordSealError::InvalidNormalizedGraph)?;
    *entry = true;
    Ok(())
}

fn local_u32(position: usize) -> Result<u32, AuthorityRecordSealError> {
    u32::try_from(position).map_err(|_| AuthorityRecordSealError::CollectionTooLarge)
}

fn compare_admit_requests(
    left: &crate::kernel::AdmitRequest,
    right: &crate::kernel::AdmitRequest,
) -> Ordering {
    left.effective()
        .cmp(&right.effective())
        .then_with(|| left.command().source().cmp(&right.command().source()))
        .then_with(|| left.command().actor().cmp(&right.command().actor()))
        .then_with(|| left.command().id().cmp(&right.command().id()))
        .then_with(|| {
            left.command()
                .fingerprint()
                .cmp(&right.command().fingerprint())
        })
        .then_with(|| left.id().cmp(&right.id()))
        .then_with(|| left.fingerprint().cmp(&right.fingerprint()))
}

fn compare_manage_requests(
    left: &crate::kernel::ManageRequest,
    right: &crate::kernel::ManageRequest,
) -> Ordering {
    left.id()
        .cmp(&right.id())
        .then_with(|| left.fingerprint().cmp(&right.fingerprint()))
        .then_with(|| left.operation().cmp(&right.operation()))
}

fn materialize(
    record_id: AuthorityRecordId,
    normalized: NormalizedAuthorityRecordBody,
) -> AuthorityRecordBody {
    match normalized {
        NormalizedAuthorityRecordBody::Admission(admission) => {
            let admission = match admission {
                NormalizedAuthorityAdmission::Commands(entries) => {
                    let entries = entries
                        .into_iter()
                        .enumerate()
                        .map(|(position, entry)| {
                            let index = u32::try_from(position).unwrap_or_else(|_| {
                                unreachable!("normalized command admission index must fit u32")
                            });
                            let captured_id = CapturedInputRecordId::derive(
                                record_id,
                                CapturedInputLocalIndex::new(index),
                            );
                            let captured = CapturedInputRecord::new(captured_id, &entry.prepared);
                            let trigger = entry.prepared.trigger();
                            let outcome = crate::kernel::AdmitOutcome::scheduled(
                                record_id,
                                entry.prepared.effective(),
                            );
                            let scheduled_command = entry.prepared.materialize(captured_id);
                            IngressRecord::new(
                                captured,
                                trigger,
                                entry.scheduler_key,
                                scheduled_command,
                                outcome,
                            )
                        })
                        .collect();
                    AuthorityAdmissionRecord::Commands(IngressBatchRecord::new(entries))
                }
                NormalizedAuthorityAdmission::ActionEvaluation(admission) => {
                    let admission = *admission;
                    let outcome = admission.request.outcome(record_id);
                    AuthorityAdmissionRecord::ActionEvaluation(Box::new(
                        ActionEvaluationAdmissionRecord::new(
                            admission.request,
                            outcome,
                            admission.transition,
                            SchedulerInsertionRecord::new(admission.scheduler_key, admission.work),
                        ),
                    ))
                }
            };
            AuthorityRecordBody::Admission(admission)
        }
        NormalizedAuthorityRecordBody::Management {
            cause,
            resulting_mode,
            preserved_frontier,
        } => {
            let batch = match *cause {
                NormalizedManagementCause::HostRequests(entries) => {
                    let entries = entries
                        .into_iter()
                        .map(|entry| {
                            let outcome = crate::kernel::ManageOutcome::applied(
                                record_id,
                                entry.request.operation(),
                            );
                            let action_evaluation = entry.action_evaluation.map(|effect| {
                                ActionEvaluationManagementRecord::new(
                                    effect.transition,
                                    effect
                                        .removed
                                        .map(|(key, work)| SchedulerRemovalRecord::new(key, work)),
                                    SchedulerInsertionRecord::new(
                                        effect.insertion_key,
                                        effect.insertion_work,
                                    ),
                                )
                            });
                            ManagementRecord::new(entry.request, outcome, action_evaluation)
                        })
                        .collect();
                    ManagementBatchRecord::host_requests(
                        entries,
                        resulting_mode,
                        preserved_frontier,
                    )
                }
                NormalizedManagementCause::KernelSafety(cause) => {
                    ManagementBatchRecord::kernel_safety(cause, resulting_mode, preserved_frontier)
                }
            };
            AuthorityRecordBody::Management(Box::new(batch))
        }
        NormalizedAuthorityRecordBody::Moment(batch) => {
            AuthorityRecordBody::Moment(Box::new(materialize_moment(record_id, *batch)))
        }
    }
}

fn materialize_moment(
    record_id: AuthorityRecordId,
    batch: NormalizedMomentBatch,
) -> MomentBatchRecord {
    let commits = batch
        .commits
        .iter()
        .copied()
        .enumerate()
        .map(|(position, delta)| {
            let index = u32::try_from(position)
                .unwrap_or_else(|_| unreachable!("normalized commit index must fit u32"));
            ContainmentTransferCommitRecord::new(
                CommitRecordId::derive(record_id, CommitLocalIndex::new(index)),
                delta,
            )
        })
        .collect::<Vec<_>>();

    let attempts = batch
        .attempts
        .into_iter()
        .enumerate()
        .map(|(position, attempt)| {
            let index = u32::try_from(position)
                .unwrap_or_else(|_| unreachable!("normalized attempt index must fit u32"));
            let resolution = match attempt.resolution {
                NormalizedAttemptResolution::Accepted { commit } => {
                    RecordedCommandResolution::Accepted {
                        commit: CommitRecordId::derive(
                            record_id,
                            CommitLocalIndex::new(commit.position()),
                        ),
                    }
                }
                NormalizedAttemptResolution::Rejected(reason) => {
                    RecordedCommandResolution::Rejected(reason)
                }
                NormalizedAttemptResolution::CommandIdCollision => {
                    RecordedCommandResolution::CommandIdCollision
                }
            };
            let subject = match attempt.subject {
                NormalizedAttemptSubject::EvaluatedCommand(command) => {
                    AttemptSubjectRecord::EvaluatedCommand(command)
                }
                NormalizedAttemptSubject::CommandIdCollision {
                    source,
                    command,
                    fingerprints,
                } => AttemptSubjectRecord::CommandIdCollision {
                    source,
                    command,
                    fingerprints: fingerprints.into_boxed_slice(),
                },
            };
            AttemptRecord::new(
                AttemptRecordId::derive(record_id, AttemptLocalIndex::new(index)),
                subject,
                resolution,
            )
        })
        .collect::<Vec<_>>();

    let reactions = batch
        .reactions
        .into_iter()
        .enumerate()
        .map(|(position, reaction)| {
            let index = u32::try_from(position)
                .unwrap_or_else(|_| unreachable!("normalized reaction index must fit u32"));
            ReactionEnvelopeRecord::new(
                ReactionEnvelopeId::derive(record_id, ReactionLocalIndex::new(index)),
                reaction,
            )
        })
        .collect::<Vec<_>>();

    let scheduler_insertions = batch
        .scheduler_insertions
        .into_iter()
        .map(|insertion| match insertion {
            NormalizedSchedulerInsertion::PostCommit {
                scheduler_key,
                reaction,
                dispatch,
            } => {
                let reaction_id = ReactionEnvelopeId::derive(
                    record_id,
                    ReactionLocalIndex::new(reaction.position()),
                );
                SchedulerInsertionRecord::new(
                    scheduler_key,
                    ScheduledWork::PostCommit(dispatch.materialize(reaction_id)),
                )
            }
            NormalizedSchedulerInsertion::ActionCommand {
                scheduler_key,
                opportunity,
                effective,
                command,
            } => SchedulerInsertionRecord::new(
                scheduler_key,
                ScheduledWork::command(ScheduledCommand::from_action_opportunity(
                    opportunity,
                    effective,
                    command,
                )),
            ),
            NormalizedSchedulerInsertion::AttemptResolved {
                scheduler_key,
                resolved,
            } => SchedulerInsertionRecord::new(
                scheduler_key,
                ScheduledWork::attempt_resolved(resolved),
            ),
            NormalizedSchedulerInsertion::RelocationProcess {
                scheduler_key,
                wake,
            } => SchedulerInsertionRecord::new(scheduler_key, ScheduledWork::process(wake)),
            NormalizedSchedulerInsertion::Lifecycle {
                scheduler_key,
                work,
            } => SchedulerInsertionRecord::new(scheduler_key, ScheduledWork::lifecycle(work)),
            NormalizedSchedulerInsertion::ActionReady {
                scheduler_key,
                ready,
            } => SchedulerInsertionRecord::new(scheduler_key, ScheduledWork::action_ready(ready)),
            NormalizedSchedulerInsertion::ActionEvaluation {
                scheduler_key,
                work,
            } => {
                SchedulerInsertionRecord::new(scheduler_key, ScheduledWork::action_evaluation(work))
            }
        })
        .collect();

    let resolutions = batch
        .resolutions
        .into_iter()
        .map(|resolution| match resolution {
            NormalizedDeliveryResolution::NewCommand { delivery, attempt } => {
                DeliveryResolutionRecord::NewCommand {
                    delivery,
                    attempt: AttemptRecordId::derive(
                        record_id,
                        AttemptLocalIndex::new(attempt.position()),
                    ),
                }
            }
            NormalizedDeliveryResolution::RetainedCommand {
                delivery,
                original_attempt,
                original_outcome,
            } => DeliveryResolutionRecord::RetainedCommand {
                delivery,
                original_attempt,
                original_outcome,
            },
            NormalizedDeliveryResolution::CommandIdReuseMismatch {
                delivery,
                original_attempt,
            } => DeliveryResolutionRecord::CommandIdReuseMismatch {
                delivery,
                original_attempt,
            },
            NormalizedDeliveryResolution::NewCollision { delivery, attempt } => {
                DeliveryResolutionRecord::NewCollision {
                    delivery,
                    attempt: AttemptRecordId::derive(
                        record_id,
                        AttemptLocalIndex::new(attempt.position()),
                    ),
                }
            }
            NormalizedDeliveryResolution::RetainedCollision {
                delivery,
                original_attempt,
            } => DeliveryResolutionRecord::RetainedCollision {
                delivery,
                original_attempt,
            },
            NormalizedDeliveryResolution::RetiredCommand { delivery } => {
                DeliveryResolutionRecord::RetiredCommand { delivery }
            }
            NormalizedDeliveryResolution::PostCommitConsumed { delivery } => {
                DeliveryResolutionRecord::PostCommitConsumed { delivery }
            }
            NormalizedDeliveryResolution::ActionReadyConsumed { delivery } => {
                DeliveryResolutionRecord::ActionReadyConsumed { delivery }
            }
            NormalizedDeliveryResolution::ActionEvaluationConsumed { delivery } => {
                DeliveryResolutionRecord::ActionEvaluationConsumed { delivery }
            }
            NormalizedDeliveryResolution::AttemptResolvedConsumed { delivery } => {
                DeliveryResolutionRecord::AttemptResolvedConsumed { delivery }
            }
            NormalizedDeliveryResolution::LifecycleConsumed { delivery } => {
                DeliveryResolutionRecord::LifecycleConsumed { delivery }
            }
            NormalizedDeliveryResolution::RelocationProcessCompleted { delivery } => {
                DeliveryResolutionRecord::RelocationProcessCompleted { delivery }
            }
            NormalizedDeliveryResolution::ObsoleteRelocationWake { delivery } => {
                DeliveryResolutionRecord::ObsoleteRelocationWake { delivery }
            }
        })
        .collect();

    MomentBatchRecord::new(
        batch.moment,
        batch.resulting_frontier,
        batch.consumed_keys,
        batch.command_deliveries,
        batch.post_commit_deliveries,
        batch.lifecycle_deliveries,
        batch.action_ready_deliveries,
        batch.action_evaluation_deliveries,
        batch.attempt_resolved_deliveries,
        batch.relocation_process_deliveries,
        batch.action_opportunity_transitions,
        batch.action_evaluation_invocation_openings,
        batch.action_evaluation_invocation_transitions,
        batch.action_opportunity_openings,
        batch.evidence_routing,
        batch.evidence_assimilations,
        batch.appraisal_transitions,
        batch.intent_adoptions,
        batch.intent_transitions,
        batch.activity_starts,
        batch.activity_transitions,
        batch.activity_terminal_transitions,
        batch.lifecycle_control_mutations,
        batch.relocation_attempts,
        batch.relocation_process_transitions,
        attempts,
        commits,
        batch.containment_delta,
        reactions,
        scheduler_insertions,
        resolutions,
        batch.resolution_evidence,
    )
}

fn map_containment_transition_error(
    error: ContainmentTransitionError,
) -> ContainmentTransferSealError {
    match error {
        ContainmentTransitionError::ItemNotContained { .. } => {
            ContainmentTransferSealError::ItemNotContained
        }
        ContainmentTransitionError::SourceMismatch {
            actual, expected, ..
        } => ContainmentTransferSealError::SourceMismatch { actual, expected },
        ContainmentTransitionError::DestinationContainerMissing { .. } => {
            ContainmentTransferSealError::DestinationContainerMissing
        }
        ContainmentTransitionError::SourceAuthorityMissing { .. } => {
            ContainmentTransferSealError::SourceAuthorityMissing
        }
        ContainmentTransitionError::InvalidSuccessor(
            DomainStateError::ContainerCapacityExceeded { .. },
        ) => ContainmentTransferSealError::DestinationCapacityExceeded,
        other @ (ContainmentTransitionError::DuplicateItemClaim { .. }
        | ContainmentTransitionError::InvalidSuccessor(_)) => {
            ContainmentTransferSealError::InvalidTransition(other)
        }
    }
}

fn map_scheduler_plan_error(error: SchedulerPlanError) -> AuthorityRecordSealError {
    match error {
        SchedulerPlanError::NoStrictlyLaterMoment { source } => {
            AuthorityRecordSealError::NoStrictlyLaterMoment { source }
        }
        SchedulerPlanError::SequenceExhausted => {
            AuthorityRecordSealError::SchedulerSequenceExhausted
        }
        SchedulerPlanError::KeyOccupied { .. } => AuthorityRecordSealError::SchedulerKeyOccupied,
    }
}

fn map_scheduler_batch_plan_error(error: SchedulerBatchPlanError) -> AuthorityRecordSealError {
    match error {
        SchedulerBatchPlanError::DuplicateProducerOrdinal { .. } => {
            AuthorityRecordSealError::InvalidNormalizedGraph
        }
        SchedulerBatchPlanError::Scheduler(error) => map_scheduler_plan_error(error),
    }
}

fn map_post_commit_error(error: PostCommitScheduleError) -> AuthorityRecordSealError {
    match error {
        PostCommitScheduleError::NoStrictlyLaterMoment { source } => {
            AuthorityRecordSealError::NoStrictlyLaterMoment { source }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use world_core::{ActorId, EntityId, Microstep, SimDuration, SimMoment, SimTime};
    use world_defs::{
        ActionBindingData, ActionData, ArtifactData, ArtifactValidator, BindingName, DefinitionKey,
        DefinitionLinker, EffectCallData, EngineProtocolVersion, EventData, EventEmissionData,
        EventFieldBindingData, EventFieldData, EventFieldName, ExactPackSet, ExactPackageSelection,
        InterfaceVersion, LocalDefinitionName, OperationCallData, OperationKind, OperationName,
        OperationParameter, PackCoordinate, PackKey, PackManifestData, PackVersion, ParameterName,
        RuntimeRequirementData, SelectedPackage, SemanticInterfaceCatalog,
        SemanticInterfaceDescriptor, SemanticInterfaceKey, SemanticOperationDescriptor,
        SourceSnapshotId, ValueKind,
    };
    use world_model::{
        AcceptedState, ActionInteractionScope, ActionOpportunity, ActionOpportunityGeneration,
        ActionSponsor, Activity, ActivityControllerId, ActivityGeneration, ActivityStateSchemaId,
        ActorLocation, ActorPosition, ActorReactionCause, AgencyState, CommandBinding,
        ContainerAuthorityRecord, ContainerRecord, ContainmentInteractionScope, ContainmentRecord,
        ContainmentTransferActivityState, ContainmentTransferDelta, DesiredCondition,
        DirectedRoute, DomainState, EpistemicState, Intent, IntentGeneration,
        RelocationInteraction, RelocationInteractionAnchor, RelocationInteractionScope,
        RelocationProcessStatus, SocialState, StableCommandRejection,
    };

    use crate::action_evaluation::{
        ActionEvaluationArtifactRole, ActionEvaluationArtifactSchemaId, ActionEvaluationCaptureId,
        ActionEvaluationCaptureLookup, ActionEvaluationCaptureOutcome,
        ActionEvaluationCaptureRequest, ActionEvaluationFallbackCause,
        ActionEvaluationInvocationPayload, ActionEvaluationInvocationState,
        ActionEvaluationResultFreshness, ActionEvaluationResultSubmission,
        ActionEvaluationTerminal, ActionEvaluationWork,
    };
    use crate::attempt::{AttemptAuthorityDomainId, AttemptStepId, ReservationGrant, RunAttemptId};
    use crate::control::RuntimeControlState;
    use crate::execution::{
        ActionPolicyBindingV1, ActionPolicyExecutionV1, CanonicalExecutionSpecV1,
        DeferredActionAdmissionModeV1, DeferredActionControlV1, ExecutionConfigArtifactV3,
        ExecutionSemanticsManifestV1, ExternalInputBindingV1, InitialStateRootV1,
        LifecycleBindingV1, LifecycleImplementationId, LifecycleProfilesV2, RootSeed,
        SemanticImplementationBinding, SemanticImplementationId, TerminationContractV1,
    };
    use crate::kernel::{
        ActionEvaluationDecision, ActionEvaluationManagementDisposition, ActivityAdvanceResult,
        AppraisalResult, CommandProposal, ContainmentCandidate, ContainmentCandidateProposal,
        ContainmentCandidateSet, ContainmentCommandIdentity, DeferredActionArtifactInput,
        DeferredActionInvocationInput, EvaluatedAction, InputId, ManageRequest,
        ManagementRequestId, MomentWorkDecision, MomentWorkInput, MomentWorkProposals,
        PostCommitRoutingDecision, PreparedDelivery, PreparedFire, SessionManagement,
        resolve_containment_candidates,
    };
    use crate::randomness::Blake3KeyedPrf256V1;
    use crate::scheduler::{PreparedScheduledCommand, ScheduledWork};

    use super::*;

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("authority fixture must be valid: {error}"),
        }
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

    fn empty_accepted_state() -> AcceptedState {
        accepted_state(Vec::new(), Vec::new(), Vec::new())
    }

    struct HeadFixture {
        closure: ResolvedExecutionClosureManifestV1,
        head: SessionHead,
    }

    fn root_fixture_with(seed_byte: u8, mode: SessionMode, accepted: AcceptedState) -> HeadFixture {
        let definitions = crate::control::test_support::definitions();
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            definitions,
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            Vec::new(),
        ));
        let root = valid(InitialStateRootV1::origin(
            mode,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted,
            Vec::new(),
        ));
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &semantics,
            RootSeed::from_bytes([seed_byte; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        let closure = valid(crate::execution::ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ));
        let head = SessionHead::root(&closure);
        HeadFixture { closure, head }
    }

    fn root_fixture() -> HeadFixture {
        root_fixture_with(0x81, SessionMode::Running, empty_accepted_state())
    }

    fn deferred_action_fixture(
        opportunity: ActionOpportunity,
        control: DeferredActionControlV1,
    ) -> HeadFixture {
        let inline = crate::execution::fixture_lifecycle_profiles();
        let profiles = LifecycleProfilesV2::new(
            inline.evidence(),
            inline.appraisal(),
            inline.social(),
            inline.intent(),
            inline.activity(),
            ActionPolicyBindingV1::new(
                LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0xa2; 32])),
                ActionPolicyExecutionV1::DeferredCaptured,
            ),
        );
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            crate::control::test_support::definitions(),
            profiles,
            valid(ExecutionConfigArtifactV3::deferred(64, 32, 16, control)),
            Vec::new(),
        ));
        let root = valid(InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            empty_accepted_state(),
            vec![opportunity],
        ));
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &semantics,
            RootSeed::from_bytes([0x83; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        let closure = valid(ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ));
        let head = SessionHead::root(&closure);
        HeadFixture { closure, head }
    }

    fn deferred_invocation_input(request_bytes: Vec<u8>) -> DeferredActionInvocationInput {
        DeferredActionInvocationInput::new(
            [0x91; 32],
            [0x92; 32],
            DeferredActionArtifactInput::new(
                ActionEvaluationArtifactSchemaId::from_bytes([0x93; 32]),
                request_bytes,
            ),
            ActionEvaluationArtifactSchemaId::from_bytes([0x94; 32]),
            DeferredActionArtifactInput::new(
                ActionEvaluationArtifactSchemaId::from_bytes([0x95; 32]),
                vec![0x51],
            ),
            DeferredActionArtifactInput::new(
                ActionEvaluationArtifactSchemaId::from_bytes([0x96; 32]),
                vec![0x52],
            ),
        )
    }

    fn publish_deferred_opening(
        fixture: &HeadFixture,
        opportunity: &ActionOpportunity,
        input: DeferredActionInvocationInput,
    ) -> crate::authority::AppliedAuthorityRecord {
        let due = fixture
            .head
            .scheduler()
            .clone_least_due()
            .unwrap_or_else(|| panic!("root opportunity must schedule action-ready work"));
        let [(key, ScheduledWork::ActionReady(ready))] = due.entries() else {
            panic!("deferred fixture must contain one action-ready delivery");
        };
        let prepared = PreparedFire::new(
            AttemptAuthorityDomainId::from_bytes([0x84; 32]),
            RunAttemptId::from_bytes([0x85; 32]),
            fixture.closure.specification().id(),
            AttemptStepId::from_bytes([0x86; 32]),
            ReservationGrant::FIRST,
            strictly_later_moment(ready.due())
                .unwrap_or_else(|error| panic!("action-ready moment must advance: {error:?}")),
            fixture.head.snapshot(),
            vec![PreparedDelivery::action_ready(
                *key,
                *ready,
                opportunity.clone(),
            )],
        )
        .unwrap_or_else(|error| panic!("deferred opening must prepare: {error:?}"));
        let work_items = prepared.work().collect::<Vec<_>>();
        let [work] = work_items.as_slice() else {
            panic!("one action-ready delivery must expose one work item");
        };
        let decision = MomentWorkDecision::begin_deferred_action(*work, input)
            .unwrap_or_else(|error| panic!("deferred decision must correlate: {error:?}"));
        let proposals = MomentWorkProposals::from_decisions(&prepared, vec![decision])
            .unwrap_or_else(|error| panic!("deferred proposals must be complete: {error:?}"));
        let draft = DraftMomentBatch::from_prepared(
            &prepared,
            &proposals,
            &empty_resolution(fixture, prepared.moment()),
        )
        .unwrap_or_else(|error| panic!("deferred opening draft must be valid: {error:?}"));
        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(fixture.head.cursor(), draft),
        )
        .unwrap_or_else(|error| panic!("deferred opening must seal: {error:?}"));
        crate::authority::apply_authority_record(&fixture.head, sealed)
            .unwrap_or_else(|error| panic!("deferred opening must replay: {error:?}"))
    }

    fn scheduled_work_fixture(
        accepted: AcceptedState,
        runtime_control: RuntimeControlState,
        work: Vec<ScheduledWork>,
    ) -> (HeadFixture, Vec<(SchedulerKey, ScheduledWork)>) {
        let root = root_fixture();
        let mut scheduler = SchedulerState::empty();
        let insertions = work
            .into_iter()
            .enumerate()
            .map(|(position, work)| {
                SchedulerInsertion::new(
                    SchedulerProducerOrdinal::new(
                        u32::try_from(position)
                            .unwrap_or_else(|error| panic!("fixture work must fit u32: {error}")),
                    ),
                    work,
                )
            })
            .collect();
        let plan = scheduler
            .plan_batch(insertions)
            .unwrap_or_else(|error| panic!("fixture work must plan: {error:?}"));
        let entries = plan.entries().to_vec();
        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("fixture work must install: {error:?}"));
        let head = SessionHead::from_authority_projection(
            root.head.cursor(),
            SessionMode::Running,
            SessionClock::from_coordinates(SimMoment::ORIGIN, SimMoment::ORIGIN),
            accepted,
            runtime_control,
            scheduler,
            None,
        );
        (
            HeadFixture {
                closure: root.closure,
                head,
            },
            entries,
        )
    }

    #[test]
    fn deferred_action_opening_and_captured_result_cross_the_authority_waist() {
        let actor = ActorId::from_bytes([0x87; 32]);
        let source = EntityId::from_bytes([0x88; 32]);
        let destination = EntityId::from_bytes([0x89; 32]);
        let item = EntityId::from_bytes([0x8a; 32]);
        let opportunity = containment_opportunity(
            actor,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x8b; 32])),
            item,
            source,
            destination,
        );
        let control = valid(DeferredActionControlV1::enabled(
            DeferredActionAdmissionModeV1::FrontierBlocking,
            1,
            8,
            8,
            8,
            8,
        ));
        let fixture = deferred_action_fixture(opportunity.clone(), control);
        let opened = publish_deferred_opening(
            &fixture,
            &opportunity,
            deferred_invocation_input(vec![0x41]),
        );
        let (opened_head, opening_record) = opened.into_parts();

        let pending = opened_head
            .runtime_control()
            .action_evaluations()
            .pending_raw();
        let [pending] = pending.as_slice() else {
            panic!("one dispatchable invocation must be externally visible");
        };
        assert_eq!(
            pending.result_schema(),
            ActionEvaluationArtifactSchemaId::from_bytes([0x94; 32])
        );
        let invocation_id = pending.invocation();
        let waiting = opened_head
            .runtime_control()
            .action_opportunities()
            .get(opportunity.id())
            .cloned()
            .unwrap_or_else(|| panic!("deferred opportunity must remain retained"));
        assert_eq!(
            waiting.state(),
            ActionOpportunityState::WaitingForEvaluation(invocation_id)
        );
        let invocation = opened_head
            .runtime_control()
            .action_evaluations()
            .get(invocation_id)
            .cloned()
            .unwrap_or_else(|| panic!("deferred invocation must be retained"));
        assert!(matches!(
            invocation.state(),
            ActionEvaluationInvocationState::DispatchPending
        ));
        let AuthorityRecordBody::Moment(opening_batch) = opening_record.body() else {
            panic!("deferred opening must publish a moment");
        };
        assert!(matches!(
            opening_batch.action_evaluation_invocation_openings(),
            [opening]
                if opening.invocation() == &invocation
                    && matches!(
                        opening.cause(),
                        ActionEvaluationInvocationOpeningCause::ActionReady(_)
                    )
        ));
        assert!(opened_head.scheduler().is_empty());

        let effective = invocation
            .blocked_at_frontier()
            .unwrap_or_else(|| panic!("frontier-blocking invocation must retain its barrier"));
        let capture = ActionEvaluationCaptureId::new(1);
        let request = ActionEvaluationCaptureRequest::resolve(
            ActionEvaluationResultSubmission::at_invocation_frontier(
                capture,
                invocation_id,
                invocation
                    .result_schema()
                    .unwrap_or_else(|| panic!("dispatchable invocation must retain result schema")),
                vec![0x42],
            ),
            &invocation,
            control,
        )
        .unwrap_or_else(|error| panic!("result capture must resolve: {error:?}"));
        let capture_fingerprint = request.fingerprint();
        let capture_sealed = seal_authority_record(
            &opened_head,
            &fixture.closure,
            DraftAuthorityRecord::admit_action_evaluation(opened_head.cursor(), request.clone()),
        )
        .unwrap_or_else(|error| panic!("result capture admission must seal: {error:?}"));
        let capture_record_id = capture_sealed.record().header().id();
        let AuthorityRecordBody::Admission(AuthorityAdmissionRecord::ActionEvaluation(
            capture_admission,
        )) = capture_sealed.record().body()
        else {
            panic!("result capture must publish an action-evaluation admission");
        };
        let outcome = capture_admission.outcome();
        let ActionEvaluationCaptureOutcome::ResultCaptured {
            record,
            invocation: retained_invocation,
            result,
            effective: retained_effective,
        } = outcome
        else {
            panic!("bounded result bytes must be retained");
        };
        assert_eq!(record, capture_record_id);
        assert_eq!(retained_invocation, invocation_id);
        assert_eq!(retained_effective, effective);
        let capture_applied =
            crate::authority::apply_authority_record(&opened_head, capture_sealed)
                .unwrap_or_else(|error| panic!("result capture must replay: {error:?}"));
        let (captured_head, _) = capture_applied.into_parts();
        assert_eq!(
            captured_head
                .runtime_control()
                .action_evaluation_captures()
                .classify(capture, invocation_id, capture_fingerprint),
            ActionEvaluationCaptureLookup::RetainedExact(outcome)
        );
        assert_eq!(
            captured_head
                .runtime_control()
                .action_evaluations()
                .minimum_blocked_frontier(),
            None
        );
        let scheduled = captured_head
            .scheduler()
            .clone_least_due()
            .unwrap_or_else(|| panic!("result capture must schedule later interpretation"));
        let [(key, ScheduledWork::ActionEvaluation(work))] = scheduled.entries() else {
            panic!("result capture must schedule exactly one action-evaluation delivery");
        };
        let key = *key;
        let work = *work;
        assert_eq!(
            work,
            ActionEvaluationWork::result_ready(
                invocation_id,
                opportunity.id(),
                waiting.version(),
                effective,
            )
        );
        let captured = captured_head
            .runtime_control()
            .action_evaluations()
            .get(invocation_id)
            .cloned()
            .unwrap_or_else(|| panic!("captured invocation must remain retained"));
        let prepared = PreparedFire::new(
            AttemptAuthorityDomainId::from_bytes([0x8c; 32]),
            RunAttemptId::from_bytes([0x8d; 32]),
            fixture.closure.specification().id(),
            AttemptStepId::from_bytes([0x8e; 32]),
            ReservationGrant::FIRST,
            strictly_later_moment(effective)
                .unwrap_or_else(|error| panic!("result moment must advance: {error:?}")),
            captured_head.snapshot(),
            vec![PreparedDelivery::action_evaluation(
                key, work, waiting, captured,
            )],
        )
        .unwrap_or_else(|error| panic!("captured result must prepare: {error:?}"));
        let work_items = prepared.work().collect::<Vec<_>>();
        let [input] = work_items.as_slice() else {
            panic!("captured result must expose one engine decision");
        };
        let decision = MomentWorkDecision::resolve_action_evaluation(
            *input,
            ActionEvaluationDecision::Apply {
                freshness: ActionEvaluationResultFreshness::Current,
                action: EvaluatedAction::NoApplicableAction,
            },
        )
        .unwrap_or_else(|error| panic!("result decision must correlate: {error:?}"));
        let proposals = MomentWorkProposals::from_decisions(&prepared, vec![decision])
            .unwrap_or_else(|error| panic!("result proposals must be complete: {error:?}"));
        let draft = DraftMomentBatch::from_prepared(
            &prepared,
            &proposals,
            &empty_resolution(&fixture, prepared.moment()),
        )
        .unwrap_or_else(|error| panic!("captured-result draft must be valid: {error:?}"));
        let sealed = seal_authority_record(
            &captured_head,
            &fixture.closure,
            DraftAuthorityRecord::moment(captured_head.cursor(), draft),
        )
        .unwrap_or_else(|error| panic!("captured result must seal: {error:?}"));
        let applied = crate::authority::apply_authority_record(&captured_head, sealed)
            .unwrap_or_else(|error| panic!("captured result must replay: {error:?}"));
        let (resulting_head, result_record) = applied.into_parts();

        assert_eq!(
            resulting_head
                .runtime_control()
                .action_opportunities()
                .get(opportunity.id())
                .map(ActionOpportunity::state),
            Some(ActionOpportunityState::Consumed(
                ActionOpportunityDisposition::NoApplicableAction
            ))
        );
        assert!(matches!(
            resulting_head
                .runtime_control()
                .action_evaluations()
                .get(invocation_id)
                .map(|record| record.state()),
            Some(ActionEvaluationInvocationState::Terminal(
                ActionEvaluationTerminal::Applied {
                    result: retained,
                    freshness: ActionEvaluationResultFreshness::Current,
                }
            )) if *retained == result
        ));
        let AuthorityRecordBody::Moment(result_batch) = result_record.body() else {
            panic!("captured result must publish a moment");
        };
        assert_eq!(result_batch.action_evaluation_deliveries().len(), 1);
        assert_eq!(
            result_batch
                .action_evaluation_invocation_transitions()
                .len(),
            1
        );
    }

    #[test]
    fn frontier_blocking_management_reports_and_releases_the_exact_barrier() {
        let actor = ActorId::from_bytes([0xb1; 32]);
        let opportunity = containment_opportunity(
            actor,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0xb2; 32])),
            EntityId::from_bytes([0xb3; 32]),
            EntityId::from_bytes([0xb4; 32]),
            EntityId::from_bytes([0xb5; 32]),
        );
        let control = valid(DeferredActionControlV1::enabled(
            DeferredActionAdmissionModeV1::FrontierBlocking,
            1,
            8,
            8,
            8,
            8,
        ));
        let fixture = deferred_action_fixture(opportunity.clone(), control);
        let opened = publish_deferred_opening(
            &fixture,
            &opportunity,
            deferred_invocation_input(vec![0xb6]),
        );
        let (opened_head, _) = opened.into_parts();
        let invocation = opened_head
            .runtime_control()
            .action_evaluations()
            .pending_dispatches()
            .next()
            .cloned()
            .unwrap_or_else(|| panic!("frontier-blocking invocation must remain pending"));
        let blocked_at = invocation
            .blocked_at_frontier()
            .unwrap_or_else(|| panic!("frontier-blocking invocation must retain its barrier"));
        let requested = strictly_later_moment(blocked_at)
            .unwrap_or_else(|error| panic!("barrier fixture must advance: {error:?}"));
        let crossing = ManageRequest::new(
            ManagementRequestId::new(31),
            SessionManagement::SealAdmissionThrough(requested),
        );
        assert_eq!(
            seal_authority_record(
                &opened_head,
                &fixture.closure,
                DraftAuthorityRecord::management(opened_head.cursor(), vec![crossing]),
            ),
            Err(AuthorityRecordSealError::ActionEvaluationFrontierBlocked { blocked_at })
        );

        let resolution = ManageRequest::new(
            ManagementRequestId::new(32),
            SessionManagement::ResolveActionEvaluation {
                invocation: invocation.invocation(),
                disposition: ActionEvaluationManagementDisposition::Timeout,
            },
        );
        let sealed = seal_authority_record(
            &opened_head,
            &fixture.closure,
            DraftAuthorityRecord::management(opened_head.cursor(), vec![resolution]),
        )
        .unwrap_or_else(|error| panic!("action-evaluation management must seal: {error:?}"));
        let AuthorityRecordBody::Management(batch) = sealed.record().body() else {
            panic!("action-evaluation resolution must publish management");
        };
        let [entry] = batch.entries() else {
            panic!("one resolution request must publish one management entry");
        };
        let effect = entry
            .action_evaluation()
            .unwrap_or_else(|| panic!("resolution must retain its exact action-evaluation effect"));
        assert_eq!(effect.scheduler_removal(), None);
        assert_eq!(
            effect.transition().cause(),
            ActionEvaluationInvocationTransitionCause::Management(resolution.id())
        );
        assert!(matches!(
            effect.fallback_insertion().work(),
            ScheduledWork::ActionEvaluation(ActionEvaluationWork::Fallback {
                invocation: retained,
                cause: ActionEvaluationFallbackCause::TimedOut,
                due,
                ..
            }) if *retained == invocation.invocation() && *due == blocked_at
        ));
        let applied = crate::authority::apply_authority_record(&opened_head, sealed)
            .unwrap_or_else(|error| panic!("action-evaluation management must replay: {error:?}"));
        let managed = applied.resulting_head();
        assert_eq!(
            managed
                .runtime_control()
                .action_evaluations()
                .minimum_blocked_frontier(),
            None
        );
        assert!(matches!(
            managed
                .runtime_control()
                .action_evaluations()
                .get(invocation.invocation())
                .map(|record| record.state()),
            Some(ActionEvaluationInvocationState::FallbackPending {
                cause: ActionEvaluationFallbackCause::TimedOut,
                ..
            })
        ));
    }

    #[test]
    fn rejected_deferred_artifact_retains_only_evidence_and_finishes_on_later_fallback() {
        let actor = ActorId::from_bytes([0x97; 32]);
        let opportunity = containment_opportunity(
            actor,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x98; 32])),
            EntityId::from_bytes([0x99; 32]),
            EntityId::from_bytes([0x9a; 32]),
            EntityId::from_bytes([0x9b; 32]),
        );
        let control = valid(DeferredActionControlV1::enabled(
            DeferredActionAdmissionModeV1::FrontierBlocking,
            1,
            1,
            8,
            8,
            8,
        ));
        let fixture = deferred_action_fixture(opportunity.clone(), control);
        let opened = publish_deferred_opening(
            &fixture,
            &opportunity,
            deferred_invocation_input(vec![0x41, 0x42]),
        );
        let (opened_head, opening_record) = opened.into_parts();
        assert!(
            opened_head
                .runtime_control()
                .action_evaluations()
                .pending_raw()
                .is_empty(),
            "rejected raw bytes must never become dispatchable"
        );
        let waiting = opened_head
            .runtime_control()
            .action_opportunities()
            .get(opportunity.id())
            .cloned()
            .unwrap_or_else(|| panic!("rejected invocation must retain its waiting opportunity"));
        let ActionOpportunityState::WaitingForEvaluation(invocation_id) = waiting.state() else {
            panic!("rejected invocation must own the waiting opportunity");
        };
        let invocation = opened_head
            .runtime_control()
            .action_evaluations()
            .get(invocation_id)
            .cloned()
            .unwrap_or_else(|| panic!("rejected invocation must be retained"));
        assert!(matches!(
            invocation.payload(),
            ActionEvaluationInvocationPayload::ArtifactRejected { failure }
                if failure.role() == ActionEvaluationArtifactRole::Request
                    && failure.actual_length() == 2
        ));
        let ActionEvaluationInvocationState::FallbackPending {
            cause,
            scheduler_key,
        } = invocation.state()
        else {
            panic!("rejected invocation must bind a later fallback");
        };
        assert!(matches!(
            cause,
            ActionEvaluationFallbackCause::ArtifactRejected(_)
        ));
        let AuthorityRecordBody::Moment(opening_batch) = opening_record.body() else {
            panic!("rejected opening must publish a moment");
        };
        assert_eq!(
            opening_batch.action_evaluation_invocation_openings().len(),
            1
        );
        assert_eq!(opening_batch.scheduler_insertions().len(), 1);

        let scheduled = opened_head
            .scheduler()
            .clone_least_due()
            .unwrap_or_else(|| panic!("artifact rejection must schedule fallback work"));
        let [(key, ScheduledWork::ActionEvaluation(work))] = scheduled.entries() else {
            panic!("artifact rejection must schedule exactly one evaluation fallback");
        };
        assert_eq!(key, scheduler_key);
        let prepared = PreparedFire::new(
            AttemptAuthorityDomainId::from_bytes([0x9c; 32]),
            RunAttemptId::from_bytes([0x9d; 32]),
            fixture.closure.specification().id(),
            AttemptStepId::from_bytes([0x9e; 32]),
            ReservationGrant::FIRST,
            strictly_later_moment(work.due())
                .unwrap_or_else(|error| panic!("fallback moment must advance: {error:?}")),
            opened_head.snapshot(),
            vec![PreparedDelivery::action_evaluation(
                *key, *work, waiting, invocation,
            )],
        )
        .unwrap_or_else(|error| panic!("fallback work must prepare: {error:?}"));
        assert!(prepared.work().next().is_none());
        let proposals = MomentWorkProposals::from_decisions(&prepared, Vec::new())
            .unwrap_or_else(|error| panic!("fallback needs no engine proposal: {error:?}"));
        let draft = DraftMomentBatch::from_prepared(
            &prepared,
            &proposals,
            &empty_resolution(&fixture, prepared.moment()),
        )
        .unwrap_or_else(|error| panic!("fallback draft must be valid: {error:?}"));
        let sealed = seal_authority_record(
            &opened_head,
            &fixture.closure,
            DraftAuthorityRecord::moment(opened_head.cursor(), draft),
        )
        .unwrap_or_else(|error| panic!("fallback must seal: {error:?}"));
        let applied = crate::authority::apply_authority_record(&opened_head, sealed)
            .unwrap_or_else(|error| panic!("fallback must replay: {error:?}"));
        let (resulting_head, record) = applied.into_parts();

        assert_eq!(
            resulting_head
                .runtime_control()
                .action_opportunities()
                .get(opportunity.id())
                .map(ActionOpportunity::state),
            Some(ActionOpportunityState::Consumed(
                ActionOpportunityDisposition::Failed
            ))
        );
        assert!(matches!(
            resulting_head
                .runtime_control()
                .action_evaluations()
                .get(invocation_id)
                .map(|record| record.state()),
            Some(ActionEvaluationInvocationState::Terminal(
                ActionEvaluationTerminal::Failed {
                    cause: ActionEvaluationFallbackCause::ArtifactRejected(_)
                }
            ))
        ));
        let AuthorityRecordBody::Moment(batch) = record.body() else {
            panic!("fallback must publish a moment");
        };
        assert_eq!(batch.action_evaluation_deliveries().len(), 1);
        assert_eq!(batch.action_evaluation_invocation_transitions().len(), 1);
        assert!(batch.scheduler_insertions().iter().any(|insertion| {
            matches!(
                insertion.work(),
                ScheduledWork::Lifecycle(LifecycleWork::AttemptResolved(_))
            )
        }));
    }

    fn empty_resolution(
        fixture: &HeadFixture,
        moment: SimMoment,
    ) -> crate::kernel::ContainmentMomentResolution {
        let candidates = ContainmentCandidateSet::new(Vec::new())
            .unwrap_or_else(|error| panic!("empty candidate set must be valid: {error:?}"));
        resolve_containment_candidates(
            moment,
            fixture.head.accepted(),
            &candidates,
            &Blake3KeyedPrf256V1::from_root_seed(fixture.closure.specification().root_seed()),
        )
    }

    #[test]
    fn same_batch_dirty_causes_advance_one_generation_and_keep_the_latest_due() {
        let actor = ActorId::from_bytes([0x01; 32]);
        let initial =
            LifecycleCause::Evidence(world_model::EvidenceDeliveryId::from_bytes([0x02; 32]));
        let first =
            LifecycleCause::Evidence(world_model::EvidenceDeliveryId::from_bytes([0x03; 32]));
        let second =
            LifecycleCause::Evidence(world_model::EvidenceDeliveryId::from_bytes([0x04; 32]));
        let mut ledger = crate::control::LifecycleControlLedger::default();
        assert_eq!(
            ledger.request(actor, LifecycleRole::Appraisal, &[initial]),
            Ok(LifecycleWakeRequestOutcome::Enqueue {
                generation: LifecycleGeneration::new(1),
            })
        );

        let early = moment(2, 1);
        let late = moment(2, 4);
        let mut mutations = BTreeMap::new();
        request_lifecycle(
            &mut mutations,
            actor,
            LifecycleRole::Appraisal,
            &[second, first],
            early,
        )
        .unwrap_or_else(|error| panic!("first same-batch request must be valid: {error:?}"));
        request_lifecycle(
            &mut mutations,
            actor,
            LifecycleRole::Appraisal,
            &[first],
            late,
        )
        .unwrap_or_else(|error| panic!("duplicate same-batch cause must be valid: {error:?}"));
        complete_lifecycle(
            &mut mutations,
            actor,
            LifecycleRole::Appraisal,
            LifecycleGeneration::new(1),
            early,
        )
        .unwrap_or_else(|error| panic!("current generation must complete: {error:?}"));

        let mut scheduled = Vec::new();
        let records = finalize_lifecycle_mutations(&mut ledger, mutations, &mut scheduled)
            .unwrap_or_else(|error| panic!("same-batch mutations must finalize: {error:?}"));

        let control = ledger
            .get(actor, LifecycleRole::Appraisal)
            .unwrap_or_else(|| panic!("lifecycle control must remain retained"));
        assert_eq!(control.desired(), LifecycleGeneration::new(2));
        assert_eq!(control.processed(), LifecycleGeneration::new(1));
        assert_eq!(control.enqueued(), Some(LifecycleGeneration::new(2)));
        assert_eq!(
            records.as_slice(),
            [LifecycleControlMutationRecord::new(
                actor,
                LifecycleRole::Appraisal,
                vec![first, second],
                Some(LifecycleGeneration::new(1)),
            )]
        );
        assert!(matches!(
            scheduled.as_slice(),
            [ScheduledWork::Lifecycle(LifecycleWork::Appraisal(work))]
                if work.actor() == actor
                    && work.generation() == LifecycleGeneration::new(2)
                    && work.due() == late
        ));
    }

    #[test]
    fn all_due_evidence_for_one_actor_assimilates_in_one_authority_transition() {
        let actor = ActorId::from_bytes([0x11; 32]);
        let source = EntityId::from_bytes([0x12; 32]);
        let destination = EntityId::from_bytes([0x13; 32]);
        let first_delta = valid(ContainmentTransferDelta::new(
            actor,
            EntityId::from_bytes([0x14; 32]),
            source,
            destination,
        ));
        let second_delta = valid(ContainmentTransferDelta::new(
            actor,
            EntityId::from_bytes([0x15; 32]),
            source,
            destination,
        ));
        let first = world_model::EvidenceRecord::direct_physical_event(
            actor,
            EvidenceDeliveryGeneration::new(1)
                .unwrap_or_else(|| panic!("one is a valid evidence generation")),
            PhysicalEvent::item_transferred(first_delta),
        );
        let second = world_model::EvidenceRecord::direct_physical_event(
            actor,
            EvidenceDeliveryGeneration::new(2)
                .unwrap_or_else(|| panic!("two is a valid evidence generation")),
            PhysicalEvent::item_transferred(second_delta),
        );
        let due = moment(1, 1);
        let (fixture, entries) = scheduled_work_fixture(
            empty_accepted_state(),
            RuntimeControlState::empty(),
            vec![
                ScheduledWork::lifecycle(LifecycleWork::EvidenceDelivery(
                    EvidenceDeliveryWork::new(first, due),
                )),
                ScheduledWork::lifecycle(LifecycleWork::EvidenceDelivery(
                    EvidenceDeliveryWork::new(second, due),
                )),
            ],
        );
        let deliveries = entries
            .iter()
            .map(|(key, work)| match work {
                ScheduledWork::Lifecycle(LifecycleWork::EvidenceDelivery(delivery)) => {
                    PreparedDelivery::evidence_delivery(*key, *delivery)
                }
                _ => unreachable!("fixture contains only evidence deliveries"),
            })
            .collect();
        let prepared = PreparedFire::new(
            AttemptAuthorityDomainId::from_bytes([0x21; 32]),
            RunAttemptId::from_bytes([0x22; 32]),
            fixture.closure.specification().id(),
            AttemptStepId::from_bytes([0x23; 32]),
            ReservationGrant::FIRST,
            strictly_later_moment(due)
                .unwrap_or_else(|error| panic!("fixture moment must advance: {error:?}")),
            fixture.head.snapshot(),
            deliveries,
        )
        .unwrap_or_else(|error| panic!("evidence batch must prepare: {error:?}"));
        let inputs = prepared.work().collect::<Vec<_>>();
        let [
            input @ MomentWorkInput::EvidenceAssimilation {
                actor: input_actor,
                evidence,
                ..
            },
        ] = inputs.as_slice()
        else {
            panic!("one actor must produce one assimilation input");
        };
        assert_eq!(*input_actor, actor);
        assert_eq!(*evidence, [first, second]);
        let successor = fixture
            .head
            .accepted()
            .epistemic()
            .assimilate(
                actor,
                fixture.head.accepted().epistemic().actor_version(actor),
                evidence.to_vec(),
            )
            .unwrap_or_else(|error| panic!("complete evidence batch must assimilate: {error:?}"));
        let decision = MomentWorkDecision::assimilate_evidence(*input, successor)
            .unwrap_or_else(|error| panic!("assimilation decision must correlate: {error:?}"));
        let proposals = MomentWorkProposals::from_decisions(&prepared, vec![decision])
            .unwrap_or_else(|error| panic!("assimilation proposal must be complete: {error:?}"));
        let draft = DraftMomentBatch::from_prepared(
            &prepared,
            &proposals,
            &empty_resolution(&fixture, due),
        )
        .unwrap_or_else(|error| panic!("assimilation draft must be valid: {error:?}"));
        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(fixture.head.cursor(), draft),
        )
        .unwrap_or_else(|error| panic!("assimilation draft must seal: {error:?}"));
        let applied = crate::authority::apply_authority_record(&fixture.head, sealed)
            .unwrap_or_else(|error| panic!("assimilation record must replay: {error:?}"));
        let (head, record) = applied.into_parts();

        assert_eq!(head.accepted().epistemic().evidence(), [first, second]);
        assert_eq!(
            head.accepted().epistemic().actor_version(actor).get(),
            1,
            "one actor batch advances epistemic version exactly once"
        );
        let control = head
            .runtime_control()
            .lifecycle()
            .get(actor, LifecycleRole::Appraisal)
            .unwrap_or_else(|| panic!("material evidence must request appraisal"));
        assert_eq!(control.desired(), LifecycleGeneration::new(1));
        let AuthorityRecordBody::Moment(batch) = record.body() else {
            panic!("assimilation must publish a moment batch");
        };
        let mut canonical_evidence = [first, second];
        canonical_evidence.sort_by_key(|evidence| evidence.id());
        assert!(matches!(
            batch.evidence_assimilations(),
            [assimilation] if assimilation.actor() == actor
                && assimilation.evidence() == canonical_evidence
        ));
    }

    #[test]
    fn materially_equal_appraisal_completes_without_intent_wake() {
        let actor = ActorId::from_bytes([0x31; 32]);
        let item = EntityId::from_bytes([0x32; 32]);
        let source = EntityId::from_bytes([0x33; 32]);
        let destination = EntityId::from_bytes([0x34; 32]);
        let evidence = world_model::EvidenceRecord::direct_physical_event(
            actor,
            EvidenceDeliveryGeneration::new(1)
                .unwrap_or_else(|| panic!("one is a valid evidence generation")),
            PhysicalEvent::item_transferred(valid(ContainmentTransferDelta::new(
                actor,
                item,
                source,
                destination,
            ))),
        );
        let epistemic = EpistemicState::empty()
            .assimilate(actor, world_model::EpistemicVersion::EMPTY, vec![evidence])
            .unwrap_or_else(|error| {
                panic!("appraisal fixture evidence must assimilate: {error:?}")
            });
        let accepted = AcceptedState::new(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
            epistemic,
            SocialState::empty(),
            AgencyState::empty(),
        );
        let appraisal = ContainmentAppraisal::new(actor, item, destination, source, evidence.id());
        let generation = LifecycleGeneration::new(1);
        let due = moment(2, 1);
        let cause = LifecycleCause::Evidence(evidence.id());
        let mut control = RuntimeControlState::empty();
        assert_eq!(
            control
                .lifecycle_mut()
                .request(actor, LifecycleRole::Appraisal, &[cause]),
            Ok(LifecycleWakeRequestOutcome::Enqueue { generation })
        );
        control.appraisals_mut().retain(appraisal);
        let work = AppraisalWork::new(actor, generation, due);
        let (fixture, entries) = scheduled_work_fixture(
            accepted,
            control,
            vec![ScheduledWork::lifecycle(LifecycleWork::Appraisal(work))],
        );
        let [(key, ScheduledWork::Lifecycle(LifecycleWork::Appraisal(_)))] = entries.as_slice()
        else {
            panic!("fixture must contain one appraisal");
        };
        let prepared = PreparedFire::new(
            AttemptAuthorityDomainId::from_bytes([0x41; 32]),
            RunAttemptId::from_bytes([0x42; 32]),
            fixture.closure.specification().id(),
            AttemptStepId::from_bytes([0x43; 32]),
            ReservationGrant::FIRST,
            strictly_later_moment(due)
                .unwrap_or_else(|error| panic!("fixture moment must advance: {error:?}")),
            fixture.head.snapshot(),
            vec![PreparedDelivery::appraisal(
                *key,
                work,
                vec![evidence],
                vec![appraisal],
            )],
        )
        .unwrap_or_else(|error| panic!("appraisal must prepare: {error:?}"));
        let inputs = prepared.work().collect::<Vec<_>>();
        let [input] = inputs.as_slice() else {
            panic!("fixture must expose one appraisal input");
        };
        let decision = MomentWorkDecision::publish_appraisals(
            *input,
            vec![AppraisalResult::present(appraisal, false)],
        )
        .unwrap_or_else(|error| panic!("appraisal decision must correlate: {error:?}"));
        let proposals = MomentWorkProposals::from_decisions(&prepared, vec![decision])
            .unwrap_or_else(|error| panic!("appraisal proposal must be complete: {error:?}"));
        let draft = DraftMomentBatch::from_prepared(
            &prepared,
            &proposals,
            &empty_resolution(&fixture, due),
        )
        .unwrap_or_else(|error| panic!("appraisal draft must be valid: {error:?}"));
        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(fixture.head.cursor(), draft),
        )
        .unwrap_or_else(|error| panic!("equal appraisal must seal: {error:?}"));
        let applied = crate::authority::apply_authority_record(&fixture.head, sealed)
            .unwrap_or_else(|error| panic!("equal appraisal must replay: {error:?}"));
        let (head, record) = applied.into_parts();

        assert!(
            head.runtime_control()
                .lifecycle()
                .get(actor, LifecycleRole::IntentReview)
                .is_none()
        );
        let AuthorityRecordBody::Moment(batch) = record.body() else {
            panic!("appraisal must publish a moment batch");
        };
        assert!(batch.scheduler_insertions().is_empty());
        assert!(matches!(
            batch.lifecycle_control_mutations(),
            [mutation] if mutation.actor() == actor
                && mutation.role() == LifecycleRole::Appraisal
                && mutation.requested().is_empty()
                && mutation.completed() == Some(generation)
        ));
    }

    #[test]
    fn exact_absence_evidence_retracts_and_replays_one_retained_appraisal() {
        let actor = ActorId::from_bytes([0x35; 32]);
        let item = EntityId::from_bytes([0x36; 32]);
        let source = EntityId::from_bytes([0x37; 32]);
        let believed_container = EntityId::from_bytes([0x38; 32]);
        let present = world_model::EvidenceRecord::direct_physical_event(
            actor,
            EvidenceDeliveryGeneration::new(1).unwrap_or_else(|| unreachable!()),
            PhysicalEvent::item_transferred(valid(ContainmentTransferDelta::new(
                actor,
                item,
                source,
                believed_container,
            ))),
        );
        let epistemic = EpistemicState::empty()
            .assimilate(actor, world_model::EpistemicVersion::EMPTY, vec![present])
            .unwrap_or_else(|error| panic!("present evidence must assimilate: {error:?}"));
        let absent = world_model::EvidenceRecord::direct_item_absent(
            actor,
            EvidenceDeliveryGeneration::new(2).unwrap_or_else(|| unreachable!()),
            item,
            believed_container,
        );
        let epistemic = epistemic
            .assimilate(actor, world_model::EpistemicVersion::new(1), vec![absent])
            .unwrap_or_else(|error| panic!("absence evidence must assimilate: {error:?}"));
        assert!(epistemic.contained_in(actor, item).is_none());
        let accepted = AcceptedState::new(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
            epistemic,
            SocialState::empty(),
            AgencyState::empty(),
        );
        let appraisal =
            ContainmentAppraisal::new(actor, item, believed_container, source, present.id());
        let generation = LifecycleGeneration::new(1);
        let due = moment(2, 1);
        let mut control = RuntimeControlState::empty();
        assert_eq!(
            control.lifecycle_mut().request(
                actor,
                LifecycleRole::Appraisal,
                &[LifecycleCause::Evidence(absent.id())],
            ),
            Ok(LifecycleWakeRequestOutcome::Enqueue { generation })
        );
        control.appraisals_mut().retain(appraisal);
        let work = AppraisalWork::new(actor, generation, due);
        let (fixture, entries) = scheduled_work_fixture(
            accepted,
            control,
            vec![ScheduledWork::lifecycle(LifecycleWork::Appraisal(work))],
        );
        let [(key, ScheduledWork::Lifecycle(LifecycleWork::Appraisal(_)))] = entries.as_slice()
        else {
            panic!("fixture must contain one appraisal");
        };
        let prepared = PreparedFire::new(
            AttemptAuthorityDomainId::from_bytes([0x45; 32]),
            RunAttemptId::from_bytes([0x46; 32]),
            fixture.closure.specification().id(),
            AttemptStepId::from_bytes([0x47; 32]),
            ReservationGrant::FIRST,
            strictly_later_moment(due)
                .unwrap_or_else(|error| panic!("fixture moment must advance: {error:?}")),
            fixture.head.snapshot(),
            vec![PreparedDelivery::appraisal(
                *key,
                work,
                vec![absent],
                vec![appraisal],
            )],
        )
        .unwrap_or_else(|error| panic!("appraisal must prepare: {error:?}"));
        let inputs = prepared.work().collect::<Vec<_>>();
        let [input] = inputs.as_slice() else {
            panic!("fixture must expose one appraisal input");
        };

        let invalid_decision = MomentWorkDecision::publish_appraisals(
            *input,
            vec![AppraisalResult::retract(appraisal, present.id())],
        )
        .unwrap_or_else(|error| panic!("retraction decision must correlate: {error:?}"));
        let invalid_proposals =
            MomentWorkProposals::from_decisions(&prepared, vec![invalid_decision])
                .unwrap_or_else(|error| panic!("invalid proposal must be complete: {error:?}"));
        let invalid_draft = DraftMomentBatch::from_prepared(
            &prepared,
            &invalid_proposals,
            &empty_resolution(&fixture, due),
        )
        .unwrap_or_else(|error| panic!("invalid retraction draft must be shaped: {error:?}"));
        assert_eq!(
            seal_authority_record(
                &fixture.head,
                &fixture.closure,
                DraftAuthorityRecord::moment(fixture.head.cursor(), invalid_draft),
            ),
            Err(AuthorityRecordSealError::InvalidNormalizedGraph)
        );

        let decision = MomentWorkDecision::publish_appraisals(
            *input,
            vec![AppraisalResult::retract(appraisal, absent.id())],
        )
        .unwrap_or_else(|error| panic!("retraction decision must correlate: {error:?}"));
        let proposals = MomentWorkProposals::from_decisions(&prepared, vec![decision])
            .unwrap_or_else(|error| panic!("retraction proposal must be complete: {error:?}"));
        let draft = DraftMomentBatch::from_prepared(
            &prepared,
            &proposals,
            &empty_resolution(&fixture, due),
        )
        .unwrap_or_else(|error| panic!("retraction draft must be valid: {error:?}"));
        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(fixture.head.cursor(), draft),
        )
        .unwrap_or_else(|error| panic!("retraction must seal: {error:?}"));
        let applied = crate::authority::apply_authority_record(&fixture.head, sealed)
            .unwrap_or_else(|error| panic!("retraction must replay: {error:?}"));
        let (head, record) = applied.into_parts();

        assert!(
            head.runtime_control()
                .appraisals()
                .get(actor, item)
                .is_none()
        );
        let AuthorityRecordBody::Moment(batch) = record.body() else {
            panic!("retraction must publish a moment");
        };
        assert!(matches!(
            batch.appraisal_transitions(),
            [ContainmentAppraisalTransitionRecord::Retracted {
                before,
                supporting_evidence,
            }] if *before == appraisal && *supporting_evidence == absent.id()
        ));
    }

    #[test]
    fn attempt_resolution_routes_only_sponsor_and_uses_fixed_recovery_timing() {
        let actor = ActorId::from_bytes([0x51; 32]);
        let item = EntityId::from_bytes([0x52; 32]);
        let source = EntityId::from_bytes([0x53; 32]);
        let destination = EntityId::from_bytes([0x54; 32]);
        let intent = Intent::adopt(
            actor,
            IntentGeneration::new(1).unwrap_or_else(|| panic!("one is nonzero")),
            DesiredCondition::item_contained_in(item, destination),
        );
        let activity = Activity::start(
            actor,
            intent.id(),
            ActivityGeneration::new(1).unwrap_or_else(|| panic!("one is nonzero")),
            ActivityControllerId::from_bytes([0x55; 32]),
            ActivityStateSchemaId::from_bytes([0x56; 32]),
            valid(ContainmentTransferActivityState::new(
                item,
                source,
                destination,
                ActionOpportunityGeneration::new(1),
                2,
            )),
        );
        let accepted = AcceptedState::new(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
            EpistemicState::empty(),
            SocialState::empty(),
            valid(AgencyState::new(vec![intent], vec![activity], Vec::new())),
        );
        let opportunity = ActionOpportunity::open(
            actor,
            ActionSponsor::activity(activity.id(), activity.version()),
            ActionInteractionScope::containment(valid(ContainmentInteractionScope::new(
                source,
                vec![destination],
                vec![item],
                1,
            ))),
            ActionOpportunityGeneration::new(1),
        );
        let due = moment(0, 1);
        let recovery_due = moment(0, 5);
        let mut routed = Vec::new();

        for disposition in [
            ActionOpportunityDisposition::NoApplicableAction,
            ActionOpportunityDisposition::Failed,
        ] {
            let mut control = RuntimeControlState::empty();
            control
                .open_action_opportunity(opportunity.clone())
                .unwrap_or_else(|error| panic!("opportunity must open: {error:?}"));
            let consumed = control
                .consume_action_opportunity(opportunity.id(), opportunity.version(), disposition)
                .unwrap_or_else(|error| panic!("opportunity must consume: {error:?}"))
                .clone();
            let resolved = AttemptResolved::new(opportunity.id(), due);
            let (fixture, entries) = scheduled_work_fixture(
                accepted.clone(),
                control,
                vec![ScheduledWork::attempt_resolved(resolved)],
            );
            let [(key, ScheduledWork::Lifecycle(LifecycleWork::AttemptResolved(_)))] =
                entries.as_slice()
            else {
                panic!("fixture must contain one neutral attempt-resolution wake");
            };
            let prepared = PreparedFire::new(
                AttemptAuthorityDomainId::from_bytes([0x61; 32]),
                RunAttemptId::from_bytes([0x62; 32]),
                fixture.closure.specification().id(),
                AttemptStepId::from_bytes([0x63; 32]),
                ReservationGrant::FIRST,
                strictly_later_moment(due)
                    .unwrap_or_else(|error| panic!("fixture moment must advance: {error:?}")),
                fixture.head.snapshot(),
                vec![PreparedDelivery::attempt_resolved(*key, resolved, consumed)],
            )
            .unwrap_or_else(|error| panic!("attempt resolution must prepare: {error:?}"));
            let inputs = prepared.work().collect::<Vec<_>>();
            let [input] = inputs.as_slice() else {
                panic!("fixture must expose one neutral continuation input");
            };
            let decision = MomentWorkDecision::consume_attempt_resolution(*input)
                .unwrap_or_else(|error| panic!("neutral decision must correlate: {error:?}"));
            let proposals = MomentWorkProposals::from_decisions(&prepared, vec![decision])
                .unwrap_or_else(|error| panic!("neutral proposal must be complete: {error:?}"));
            let draft = DraftMomentBatch::from_prepared(
                &prepared,
                &proposals,
                &empty_resolution(&fixture, due),
            )
            .unwrap_or_else(|error| panic!("neutral draft must be valid: {error:?}"));
            let sealed = seal_authority_record(
                &fixture.head,
                &fixture.closure,
                DraftAuthorityRecord::moment(fixture.head.cursor(), draft),
            )
            .unwrap_or_else(|error| panic!("neutral continuation must seal: {error:?}"));
            let applied = crate::authority::apply_authority_record(&fixture.head, sealed)
                .unwrap_or_else(|error| panic!("neutral continuation must replay: {error:?}"));
            let (head, record) = applied.into_parts();
            let AuthorityRecordBody::Moment(batch) = record.body() else {
                panic!("neutral continuation must publish a moment batch");
            };
            let [insertion] = batch.scheduler_insertions() else {
                panic!("activity sponsor must produce one recovery wake");
            };
            assert!(matches!(
                insertion.work(),
                ScheduledWork::Lifecycle(LifecycleWork::ActivityAdvance(work))
                    if work.actor() == actor
                        && work.generation() == LifecycleGeneration::new(1)
                        && work.due() == recovery_due
            ));
            let control = head
                .runtime_control()
                .lifecycle()
                .get(actor, LifecycleRole::ActivityAdvance)
                .unwrap_or_else(|| panic!("activity recovery control must be retained"));
            assert_eq!(control.desired(), LifecycleGeneration::new(1));
            routed.push(insertion.work().clone());
        }

        assert_eq!(
            routed[0], routed[1],
            "terminal action disposition cannot affect neutral sponsor routing"
        );
    }

    #[test]
    fn terminal_activity_advancement_atomically_transitions_its_owning_intent() {
        let actor = ActorId::from_bytes([0x71; 32]);
        let item = EntityId::from_bytes([0x72; 32]);
        let source = EntityId::from_bytes([0x73; 32]);
        let destination = EntityId::from_bytes([0x74; 32]);
        let route = valid(DirectedRoute::new(
            source,
            destination,
            SimDuration::from_ticks(10),
        ));
        let intent = Intent::adopt(
            actor,
            IntentGeneration::new(1).unwrap_or_else(|| panic!("one is nonzero")),
            DesiredCondition::item_contained_in(item, destination),
        );
        let activity = Activity::start(
            actor,
            intent.id(),
            ActivityGeneration::new(1).unwrap_or_else(|| panic!("one is nonzero")),
            ActivityControllerId::from_bytes([0x75; 32]),
            ActivityStateSchemaId::from_bytes([0x76; 32]),
            valid(ContainmentTransferActivityState::new(
                item,
                source,
                destination,
                ActionOpportunityGeneration::new(1),
                1,
            )),
        );
        let accepted = AcceptedState::new(
            valid(
                valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())).with_mobility(
                    vec![route],
                    vec![ActorPosition::new(actor, ActorLocation::in_transit(route))],
                ),
            ),
            EpistemicState::empty(),
            SocialState::empty(),
            valid(AgencyState::new(vec![intent], vec![activity], Vec::new())),
        );
        let due = moment(3, 1);

        for (activity_operation, intent_operation) in [
            (
                world_model::ActivityTransition::Complete,
                world_model::IntentTransition::Achieve,
            ),
            (
                world_model::ActivityTransition::Fail,
                world_model::IntentTransition::Fail,
            ),
        ] {
            let activity_successor = activity
                .transition(activity.version(), activity_operation)
                .unwrap_or_else(|error| panic!("terminal activity must transition: {error:?}"));
            let intent_successor = intent
                .transition(intent.version(), intent_operation)
                .unwrap_or_else(|error| panic!("owning intent must transition: {error:?}"));
            let generation = LifecycleGeneration::new(1);
            let cause =
                LifecycleCause::AttemptResolved(ActionOpportunityId::from_bytes([0x76; 32]));
            let mut control = RuntimeControlState::empty();
            let process = control
                .relocation_processes_mut()
                .start(actor, route, SimTime::from_ticks(1))
                .unwrap_or_else(|error| panic!("live relocation must start: {error:?}"));
            assert_eq!(
                control
                    .lifecycle_mut()
                    .request(actor, LifecycleRole::ActivityAdvance, &[cause]),
                Ok(LifecycleWakeRequestOutcome::Enqueue { generation })
            );
            let work = ActivityAdvanceWork::new(actor, generation, due);
            let (fixture, entries) = scheduled_work_fixture(
                accepted.clone(),
                control,
                vec![ScheduledWork::lifecycle(LifecycleWork::ActivityAdvance(
                    work,
                ))],
            );
            let [(key, ScheduledWork::Lifecycle(LifecycleWork::ActivityAdvance(_)))] =
                entries.as_slice()
            else {
                panic!("fixture must contain one activity advancement");
            };
            let prepared = PreparedFire::new(
                AttemptAuthorityDomainId::from_bytes([0x77; 32]),
                RunAttemptId::from_bytes([0x78; 32]),
                fixture.closure.specification().id(),
                AttemptStepId::from_bytes([0x79; 32]),
                ReservationGrant::FIRST,
                strictly_later_moment(due)
                    .unwrap_or_else(|error| panic!("fixture moment must advance: {error:?}")),
                fixture.head.snapshot(),
                vec![PreparedDelivery::activity_advance(
                    *key,
                    work,
                    vec![activity],
                    Vec::new(),
                )],
            )
            .unwrap_or_else(|error| panic!("activity advancement must prepare: {error:?}"));
            let inputs = prepared.work().collect::<Vec<_>>();
            let [input] = inputs.as_slice() else {
                panic!("fixture must expose one activity-advancement input");
            };
            let decision = MomentWorkDecision::advance_activity(
                *input,
                ActivityAdvanceResult::Terminal {
                    expected_activity_version: activity.version(),
                    activity_successor: Box::new(activity_successor),
                    expected_intent_version: intent.version(),
                    intent_successor,
                },
            )
            .unwrap_or_else(|error| panic!("terminal decision must correlate: {error:?}"));
            let proposals = MomentWorkProposals::from_decisions(&prepared, vec![decision])
                .unwrap_or_else(|error| panic!("terminal proposal must be complete: {error:?}"));
            let draft = DraftMomentBatch::from_prepared(
                &prepared,
                &proposals,
                &empty_resolution(&fixture, due),
            )
            .unwrap_or_else(|error| panic!("terminal draft must be valid: {error:?}"));
            let sealed = seal_authority_record(
                &fixture.head,
                &fixture.closure,
                DraftAuthorityRecord::moment(fixture.head.cursor(), draft),
            )
            .unwrap_or_else(|error| panic!("terminal transition must seal: {error:?}"));
            let applied = crate::authority::apply_authority_record(&fixture.head, sealed)
                .unwrap_or_else(|error| panic!("terminal transition must replay: {error:?}"));
            let (head, record) = applied.into_parts();

            assert_eq!(
                head.accepted().agency().activity(activity.id()).copied(),
                Some(activity_successor)
            );
            assert_eq!(
                head.accepted().agency().intent(intent.id()).copied(),
                Some(intent_successor)
            );
            assert_eq!(
                head.runtime_control()
                    .relocation_processes()
                    .live_for(actor),
                Some(process),
                "terminating an activity must not mutate its actor's live relocation process"
            );
            let AuthorityRecordBody::Moment(batch) = record.body() else {
                panic!("terminal transition must publish a moment batch");
            };
            assert!(batch.activity_transitions().is_empty());
            assert!(batch.intent_transitions().is_empty());
            assert!(matches!(
                batch.activity_terminal_transitions(),
                [transition]
                    if transition.activity_before() == activity
                        && transition.activity_after() == activity_successor
                        && transition.intent_before() == intent
                        && transition.intent_after() == intent_successor
            ));
        }
    }

    fn relocation_fixture(
        accepted: AcceptedState,
        opportunity: ActionOpportunity,
        now: SimMoment,
    ) -> HeadFixture {
        let definitions = crate::control::test_support::definitions();
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            definitions,
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            Vec::new(),
        ));
        let root = valid(InitialStateRootV1::origin(
            SessionMode::Running,
            now,
            now,
            accepted,
            vec![opportunity],
        ));
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &semantics,
            RootSeed::from_bytes([0x82; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        let closure = valid(crate::execution::ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ));
        let head = SessionHead::root(&closure);
        HeadFixture { closure, head }
    }

    fn apply_relocation_action(
        fixture: &HeadFixture,
        opportunity: &ActionOpportunity,
        interaction: RelocationInteraction,
    ) -> crate::authority::AppliedAuthorityRecord {
        let due = fixture
            .head
            .scheduler()
            .clone_least_due()
            .unwrap_or_else(|| panic!("root opportunity must schedule action-ready work"));
        let [(key, ScheduledWork::ActionReady(ready))] = due.entries() else {
            panic!("relocation fixture must contain one action-ready delivery");
        };
        let prepared = PreparedFire::new(
            AttemptAuthorityDomainId::from_bytes([0x71; 32]),
            RunAttemptId::from_bytes([0x72; 32]),
            fixture.closure.specification().id(),
            AttemptStepId::from_bytes([0x73; 32]),
            ReservationGrant::FIRST,
            strictly_later_moment(SimMoment::ORIGIN)
                .unwrap_or_else(|error| panic!("origin must have a successor: {error:?}")),
            fixture.head.snapshot(),
            vec![PreparedDelivery::action_ready(
                *key,
                *ready,
                opportunity.clone(),
            )],
        )
        .unwrap_or_else(|error| panic!("relocation action must prepare: {error:?}"));
        let inputs = prepared.work().collect::<Vec<_>>();
        let [input] = inputs.as_slice() else {
            panic!("one current relocation action must expose one work input");
        };
        let decision = MomentWorkDecision::submit_relocation_action(*input, interaction)
            .unwrap_or_else(|error| panic!("relocation decision must correlate: {error:?}"));
        let proposals = MomentWorkProposals::from_decisions(&prepared, vec![decision])
            .unwrap_or_else(|error| panic!("relocation proposals must be complete: {error:?}"));
        let candidates = ContainmentCandidateSet::new(Vec::new())
            .unwrap_or_else(|error| panic!("empty containment candidates are valid: {error:?}"));
        let resolution = resolve_containment_candidates(
            prepared.moment(),
            prepared.base_snapshot().accepted(),
            &candidates,
            &Blake3KeyedPrf256V1::from_root_seed(fixture.closure.specification().root_seed()),
        );
        let batch = DraftMomentBatch::from_prepared(&prepared, &proposals, &resolution)
            .unwrap_or_else(|error| panic!("relocation draft must be checked: {error:?}"));
        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(fixture.head.cursor(), batch),
        )
        .unwrap_or_else(|error| panic!("relocation draft must seal: {error:?}"));
        crate::authority::apply_authority_record(&fixture.head, sealed)
            .unwrap_or_else(|error| panic!("relocation record must apply: {error:?}"))
    }

    #[test]
    fn grounded_start_relocation_crosses_the_complete_authority_waist() {
        let actor = ActorId::from_bytes([0x11; 32]);
        let source = EntityId::from_bytes([0x21; 32]);
        let destination = EntityId::from_bytes([0x22; 32]);
        let route = valid(DirectedRoute::new(
            source,
            destination,
            SimDuration::from_ticks(6),
        ));
        let domain = valid(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())).with_mobility(
                vec![route],
                vec![ActorPosition::new(actor, ActorLocation::at(source))],
            ),
        );
        let accepted = AcceptedState::new(
            domain,
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );
        let opportunity = ActionOpportunity::open(
            actor,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x31; 32])),
            ActionInteractionScope::relocation(valid(RelocationInteractionScope::new(
                vec![RelocationInteractionAnchor::new(
                    RelocationInteraction::Start(route.id()),
                    route.source(),
                    route.destination(),
                )],
                1,
            ))),
            ActionOpportunityGeneration::new(0),
        );
        let fixture = relocation_fixture(accepted.clone(), opportunity.clone(), SimMoment::ORIGIN);
        let applied = apply_relocation_action(
            &fixture,
            &opportunity,
            RelocationInteraction::Start(route.id()),
        );
        let (head, record) = applied.into_parts();

        assert_eq!(
            head.accepted().domain().actor_location(actor),
            Some(ActorLocation::in_transit(route))
        );
        assert_eq!(head.accepted().epistemic(), accepted.epistemic());
        assert_eq!(head.accepted().social(), accepted.social());
        assert_eq!(head.accepted().agency(), accepted.agency());
        let process = head
            .runtime_control()
            .relocation_processes()
            .live_for(actor)
            .unwrap_or_else(|| panic!("departure must install one live process"));
        assert_eq!(process.route(), route);
        let AuthorityRecordBody::Moment(batch) = record.body() else {
            panic!("relocation must publish a moment record");
        };
        assert!(
            batch
                .scheduler_insertions()
                .iter()
                .any(|insertion| matches!(
                    insertion.work(),
                    ScheduledWork::Process(wake) if wake.process() == process.id()
                ))
        );
        assert!(matches!(
            batch.relocation_attempts(),
            [attempt]
                if attempt.interaction() == RelocationInteraction::Start(route.id())
                    && attempt.resolution()
                        == RelocationAttemptResolution::Accepted {
                            process: process.id(),
                        }
        ));
        let attempt_delivery = batch.relocation_attempts()[0].resolution_delivery();
        assert!(matches!(
            batch.relocation_process_transitions(),
            [transition]
                if transition.cause()
                    == RelocationProcessTransitionCause::Action(attempt_delivery)
                    && matches!(transition.event(), Some(PhysicalEvent::ActorDeparted(_)))
        ));
    }

    #[test]
    fn rejected_relocation_consumes_the_opportunity_without_partial_effects() {
        let actor = ActorId::from_bytes([0x11; 32]);
        let source = EntityId::from_bytes([0x21; 32]);
        let destination = EntityId::from_bytes([0x22; 32]);
        let elsewhere = EntityId::from_bytes([0x23; 32]);
        let route = valid(DirectedRoute::new(
            source,
            destination,
            SimDuration::from_ticks(6),
        ));
        let domain = valid(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())).with_mobility(
                vec![route],
                vec![ActorPosition::new(actor, ActorLocation::at(elsewhere))],
            ),
        );
        let accepted = AcceptedState::new(
            domain,
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );
        let opportunity = ActionOpportunity::open(
            actor,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x31; 32])),
            ActionInteractionScope::relocation(valid(RelocationInteractionScope::new(
                vec![RelocationInteractionAnchor::new(
                    RelocationInteraction::Start(route.id()),
                    route.source(),
                    route.destination(),
                )],
                1,
            ))),
            ActionOpportunityGeneration::new(0),
        );
        let fixture = relocation_fixture(accepted.clone(), opportunity.clone(), SimMoment::ORIGIN);
        let applied = apply_relocation_action(
            &fixture,
            &opportunity,
            RelocationInteraction::Start(route.id()),
        );
        let (head, record) = applied.into_parts();

        assert_eq!(head.accepted(), &accepted);
        assert!(
            head.runtime_control()
                .relocation_processes()
                .live_for(actor)
                .is_none()
        );
        let AuthorityRecordBody::Moment(batch) = record.body() else {
            panic!("rejected relocation must publish a moment record");
        };
        assert!(matches!(
            batch.relocation_attempts(),
            [attempt]
                if attempt.interaction() == RelocationInteraction::Start(route.id())
                    && attempt.resolution()
                        == RelocationAttemptResolution::Rejected(
                            RelocationAttemptRejection::PositionMismatch,
                        )
        ));
        assert!(batch.relocation_process_transitions().is_empty());
        assert!(
            !batch
                .scheduler_insertions()
                .iter()
                .any(|insertion| { matches!(insertion.work(), ScheduledWork::Process(_)) })
        );
        assert!(batch.scheduler_insertions().iter().any(|insertion| {
            matches!(
                insertion.work(),
                ScheduledWork::Lifecycle(LifecycleWork::AttemptResolved(resolved))
                    if resolved.opportunity() == opportunity.id()
            )
        }));
    }

    #[test]
    fn conflicting_relocation_start_leaves_the_process_ledger_unchanged() {
        let actor = ActorId::from_bytes([0x11; 32]);
        let source = EntityId::from_bytes([0x21; 32]);
        let destination = EntityId::from_bytes([0x22; 32]);
        let route = valid(DirectedRoute::new(
            source,
            destination,
            SimDuration::from_ticks(6),
        ));
        let domain = valid(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())).with_mobility(
                vec![route],
                vec![ActorPosition::new(actor, ActorLocation::at(source))],
            ),
        );
        let accepted = AcceptedState::new(
            domain,
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );
        let empty = RelocationProcessLedger::default();
        let first_delivery =
            ActionReadyDeliveryRef::from_position(0).unwrap_or_else(|| unreachable!());
        let (_, processes, _, _) = apply_relocation_interaction(
            &accepted,
            &empty,
            actor,
            RelocationInteraction::Start(route.id()),
            SimMoment::ORIGIN,
            RelocationProcessTransitionCause::Action(ActionResolutionDeliveryRef::Ready(
                first_delivery,
            )),
        )
        .unwrap_or_else(|error| panic!("first start must be authoritative: {error:?}"));
        let before_conflict = processes.clone();
        let conflicting_delivery =
            ActionReadyDeliveryRef::from_position(1).unwrap_or_else(|| unreachable!());

        let conflict = apply_relocation_interaction(
            &accepted,
            &processes,
            actor,
            RelocationInteraction::Start(route.id()),
            SimMoment::at(SimTime::from_ticks(1)),
            RelocationProcessTransitionCause::Action(ActionResolutionDeliveryRef::Ready(
                conflicting_delivery,
            )),
        );

        assert_eq!(
            conflict,
            Err(RelocationInteractionApplyError::Rejected(
                RelocationAttemptRejection::ProcessStateConflict,
            ))
        );
        assert_eq!(processes, before_conflict);
    }

    #[test]
    fn relocation_completion_cites_the_exact_current_wake_delivery() {
        let actor = ActorId::from_bytes([0x11; 32]);
        let source = EntityId::from_bytes([0x21; 32]);
        let destination = EntityId::from_bytes([0x22; 32]);
        let route = valid(DirectedRoute::new(
            source,
            destination,
            SimDuration::from_ticks(6),
        ));
        let domain = valid(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())).with_mobility(
                vec![route],
                vec![ActorPosition::new(actor, ActorLocation::at(source))],
            ),
        );
        let accepted = AcceptedState::new(
            domain,
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );
        let mut control = RuntimeControlState::empty();
        let active = control
            .relocation_processes_mut()
            .start(actor, route, SimTime::ZERO)
            .unwrap_or_else(|error| panic!("fixture process must start: {error:?}"));
        let accepted = apply_relocation_departure(&accepted, active)
            .unwrap_or_else(|error| panic!("fixture departure must apply: {error:?}"));
        let wake = RelocationProcessWake::for_active(active)
            .unwrap_or_else(|| panic!("active process must expose one completion wake"));
        let (fixture, entries) =
            scheduled_work_fixture(accepted, control, vec![ScheduledWork::process(wake)]);
        let [(key, ScheduledWork::Process(scheduled_wake))] = entries.as_slice() else {
            panic!("fixture must contain one process delivery");
        };
        let prepared = PreparedFire::new(
            AttemptAuthorityDomainId::from_bytes([0x71; 32]),
            RunAttemptId::from_bytes([0x72; 32]),
            fixture.closure.specification().id(),
            AttemptStepId::from_bytes([0x73; 32]),
            ReservationGrant::FIRST,
            strictly_later_moment(wake.due())
                .unwrap_or_else(|error| panic!("wake moment must advance: {error:?}")),
            fixture.head.snapshot(),
            vec![PreparedDelivery::process(
                *key,
                *scheduled_wake,
                RelocationWakeClassification::Current(active.id()),
            )],
        )
        .unwrap_or_else(|error| panic!("process completion must prepare: {error:?}"));
        let inputs = prepared.work().collect::<Vec<_>>();
        let [input] = inputs.as_slice() else {
            panic!("one current process wake must expose one work input");
        };
        let decision = MomentWorkDecision::complete_relocation_process(*input)
            .unwrap_or_else(|error| panic!("completion decision must correlate: {error:?}"));
        let proposals = MomentWorkProposals::from_decisions(&prepared, vec![decision])
            .unwrap_or_else(|error| panic!("completion proposal must be complete: {error:?}"));
        let batch = DraftMomentBatch::from_prepared(
            &prepared,
            &proposals,
            &empty_resolution(&fixture, prepared.moment()),
        )
        .unwrap_or_else(|error| panic!("completion draft must be valid: {error:?}"));
        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(fixture.head.cursor(), batch),
        )
        .unwrap_or_else(|error| panic!("completion draft must seal: {error:?}"));
        let applied = crate::authority::apply_authority_record(&fixture.head, sealed)
            .unwrap_or_else(|error| panic!("completion record must replay: {error:?}"));
        let (head, record) = applied.into_parts();

        assert_eq!(
            head.accepted().domain().actor_location(actor),
            Some(ActorLocation::at(destination))
        );
        let AuthorityRecordBody::Moment(batch) = record.body() else {
            panic!("completion must publish a moment record");
        };
        let [delivery] = batch.relocation_process_deliveries() else {
            panic!("completion must retain one process delivery");
        };
        let [transition] = batch.relocation_process_transitions() else {
            panic!("completion must retain one process transition");
        };
        let RelocationProcessTransitionCause::Wake(reference) = transition.cause() else {
            panic!("completion must cite a wake delivery");
        };
        assert_eq!(
            batch
                .relocation_process_deliveries()
                .get(usize::try_from(reference.index()).unwrap_or_else(|_| unreachable!())),
            Some(delivery)
        );
        assert_eq!(delivery.wake(), wake);
        assert!(matches!(
            transition.event(),
            Some(PhysicalEvent::ActorArrived(_))
        ));
    }

    #[test]
    fn relocation_start_pause_resume_preserves_progress_and_agency() {
        let actor = ActorId::from_bytes([0x11; 32]);
        let source = EntityId::from_bytes([0x21; 32]);
        let destination = EntityId::from_bytes([0x22; 32]);
        let route = valid(DirectedRoute::new(
            source,
            destination,
            SimDuration::from_ticks(10),
        ));

        let item = EntityId::from_bytes([0x31; 32]);
        let desired_container = EntityId::from_bytes([0x32; 32]);
        let activity_source = EntityId::from_bytes([0x33; 32]);
        let intent = Intent::adopt(
            actor,
            IntentGeneration::new(1).unwrap_or_else(|| unreachable!()),
            DesiredCondition::item_contained_in(item, desired_container),
        );
        let activity = Activity::start(
            actor,
            intent.id(),
            ActivityGeneration::new(1).unwrap_or_else(|| unreachable!()),
            ActivityControllerId::from_bytes([0x41; 32]),
            ActivityStateSchemaId::from_bytes([0x42; 32]),
            valid(ContainmentTransferActivityState::new(
                item,
                activity_source,
                desired_container,
                ActionOpportunityGeneration::new(1),
                3,
            )),
        );
        let agency = valid(AgencyState::new(vec![intent], vec![activity], Vec::new()));
        let domain = valid(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())).with_mobility(
                vec![route],
                vec![ActorPosition::new(actor, ActorLocation::at(source))],
            ),
        );
        let accepted = AcceptedState::new(
            domain,
            EpistemicState::empty(),
            SocialState::empty(),
            agency,
        );
        let original_agency = accepted.agency().clone();
        let processes = RelocationProcessLedger::default();

        let start_delivery =
            ActionReadyDeliveryRef::from_position(0).unwrap_or_else(|| unreachable!());
        let (departed, processes, start, first_wake) = apply_relocation_interaction(
            &accepted,
            &processes,
            actor,
            RelocationInteraction::Start(route.id()),
            SimMoment::at(SimTime::from_ticks(4)),
            RelocationProcessTransitionCause::Action(ActionResolutionDeliveryRef::Ready(
                start_delivery,
            )),
        )
        .unwrap_or_else(|error| panic!("relocation start must be authoritative: {error:?}"));
        let first_wake = first_wake.unwrap_or_else(|| panic!("start must schedule one wake"));
        assert_eq!(
            departed.domain().actor_location(actor),
            Some(ActorLocation::in_transit(route))
        );
        assert!(matches!(
            start.event(),
            Some(PhysicalEvent::ActorDeparted(_))
        ));
        assert_eq!(departed.agency(), &original_agency);

        let pause_delivery =
            ActionReadyDeliveryRef::from_position(1).unwrap_or_else(|| unreachable!());
        let (paused_state, processes, pause, no_wake) = apply_relocation_interaction(
            &departed,
            &processes,
            actor,
            RelocationInteraction::Pause(route.id()),
            SimMoment::at(SimTime::from_ticks(7)),
            RelocationProcessTransitionCause::Action(ActionResolutionDeliveryRef::Ready(
                pause_delivery,
            )),
        )
        .unwrap_or_else(|error| panic!("relocation pause must be authoritative: {error:?}"));
        let RelocationProcessStatus::Paused { elapsed, .. } = pause.after().status() else {
            panic!("pause must retain paused process state");
        };
        assert_eq!(elapsed, SimDuration::from_ticks(3));
        assert_eq!(no_wake, None);
        assert_eq!(paused_state.agency(), &original_agency);
        assert_eq!(
            processes.classify_wake(first_wake),
            RelocationWakeClassification::Obsolete
        );

        let resume_delivery =
            ActionReadyDeliveryRef::from_position(2).unwrap_or_else(|| unreachable!());
        let (resumed_state, mut processes, _resume, current_wake) = apply_relocation_interaction(
            &paused_state,
            &processes,
            actor,
            RelocationInteraction::Resume(route.id()),
            SimMoment::at(SimTime::from_ticks(20)),
            RelocationProcessTransitionCause::Action(ActionResolutionDeliveryRef::Ready(
                resume_delivery,
            )),
        )
        .unwrap_or_else(|error| panic!("relocation resume must be authoritative: {error:?}"));
        let current_wake =
            current_wake.unwrap_or_else(|| panic!("resume must schedule one current wake"));
        assert_eq!(current_wake.due(), SimMoment::at(SimTime::from_ticks(27)));
        assert_eq!(resumed_state.agency(), &original_agency);
        assert_eq!(
            processes.classify_wake(first_wake),
            RelocationWakeClassification::Obsolete
        );

        let (_, completed) = processes
            .complete(current_wake, current_wake.due().time())
            .unwrap_or_else(|error| panic!("current wake must complete exactly once: {error:?}"));
        let arrived = apply_relocation_arrival(&resumed_state, completed)
            .unwrap_or_else(|error| panic!("completion must install arrival: {error:?}"));
        assert_eq!(
            arrived.domain().actor_location(actor),
            Some(ActorLocation::at(destination))
        );
        assert_eq!(arrived.agency(), &original_agency);
    }

    #[test]
    fn containment_action_scope_accepts_only_its_actor_and_anchors() {
        let actor = ActorId::from_bytes([0x11; 32]);
        let item = EntityId::from_bytes([0x22; 32]);
        let source = EntityId::from_bytes([0x21; 32]);
        let destination = EntityId::from_bytes([0x31; 32]);
        let alternate = EntityId::from_bytes([0x32; 32]);
        let opportunity = ActionOpportunity::open(
            actor,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x41; 32])),
            world_model::ActionInteractionScope::containment(valid(
                ContainmentInteractionScope::new(
                    source,
                    vec![alternate, destination],
                    vec![item],
                    8,
                ),
            )),
            ActionOpportunityGeneration::new(0),
        );

        assert!(
            ScopedContainmentTransfer {
                actor,
                item,
                source,
                destination,
            }
            .is_authorized_by(&opportunity)
        );
        assert!(
            !ScopedContainmentTransfer {
                actor: ActorId::from_bytes([0x12; 32]),
                item,
                source,
                destination,
            }
            .is_authorized_by(&opportunity)
        );
        assert!(
            !ScopedContainmentTransfer {
                actor,
                item,
                source: EntityId::from_bytes([0x22; 32]),
                destination,
            }
            .is_authorized_by(&opportunity)
        );
        assert!(
            !ScopedContainmentTransfer {
                actor,
                item,
                source,
                destination: EntityId::from_bytes([0x33; 32]),
            }
            .is_authorized_by(&opportunity)
        );
        assert!(
            !ScopedContainmentTransfer {
                actor,
                item: EntityId::from_bytes([0x23; 32]),
                source,
                destination,
            }
            .is_authorized_by(&opportunity)
        );
    }

    struct ContainmentActionDefinitionFixture {
        semantics: ExecutionSemanticsManifestV1,
        action: DefinitionKey,
        actor_binding: BindingName,
        item_binding: BindingName,
        source_binding: BindingName,
        destination_binding: BindingName,
    }

    fn containment_action_definitions() -> ContainmentActionDefinitionFixture {
        let actor_binding = valid(BindingName::parse("actor"));
        let item_binding = valid(BindingName::parse("item"));
        let source_binding = valid(BindingName::parse("source"));
        let destination_binding = valid(BindingName::parse("destination"));
        let interface_key = valid(SemanticInterfaceKey::parse("test.containment-transfer"));
        let allowed = valid(OperationName::parse("allowed"));
        let apply = valid(OperationName::parse("apply"));
        let parameters = || {
            vec![
                OperationParameter::new(valid(ParameterName::parse("actor")), ValueKind::Actor),
                OperationParameter::new(valid(ParameterName::parse("item")), ValueKind::Entity),
                OperationParameter::new(valid(ParameterName::parse("source")), ValueKind::Entity),
                OperationParameter::new(
                    valid(ParameterName::parse("destination")),
                    ValueKind::Entity,
                ),
            ]
        };
        let descriptor = valid(SemanticInterfaceDescriptor::new(
            interface_key.clone(),
            valid(InterfaceVersion::new(1)),
            vec![
                valid(SemanticOperationDescriptor::new(
                    allowed.clone(),
                    OperationKind::Predicate,
                    parameters(),
                )),
                valid(SemanticOperationDescriptor::new(
                    apply.clone(),
                    OperationKind::Effect,
                    parameters(),
                )),
            ],
        ));
        let coordinate = PackCoordinate::new(
            valid(PackKey::parse("test.containment-feedback")),
            PackVersion::new(1, 0, 0),
        );
        let action_name = valid(LocalDefinitionName::parse("transfer"));
        let action = DefinitionKey::new(coordinate.pack_key().clone(), action_name.clone());
        let event_name = valid(LocalDefinitionName::parse("transferred"));
        let item_field = valid(EventFieldName::parse("item"));
        let arguments = vec![
            actor_binding.clone(),
            item_binding.clone(),
            source_binding.clone(),
            destination_binding.clone(),
        ];
        let artifact = valid(
            ArtifactValidator::new(&valid(SemanticInterfaceCatalog::new(vec![
                descriptor.clone(),
            ])))
            .validate(ArtifactData::new(
                PackManifestData::new(
                    EngineProtocolVersion::new(1),
                    coordinate.clone(),
                    Vec::new(),
                ),
                vec![descriptor.reference()],
                vec![ActionData::new(
                    action_name,
                    vec![
                        ActionBindingData::new(actor_binding.clone(), ValueKind::Actor),
                        ActionBindingData::new(item_binding.clone(), ValueKind::Entity),
                        ActionBindingData::new(source_binding.clone(), ValueKind::Entity),
                        ActionBindingData::new(destination_binding.clone(), ValueKind::Entity),
                    ],
                    vec![RuntimeRequirementData::new(OperationCallData::new(
                        interface_key.clone(),
                        allowed,
                        arguments.clone(),
                    ))],
                    vec![EffectCallData::new(OperationCallData::new(
                        interface_key,
                        apply,
                        arguments,
                    ))],
                    vec![EventEmissionData::new(
                        DefinitionKey::new(coordinate.pack_key().clone(), event_name.clone()),
                        vec![EventFieldBindingData::new(
                            item_field.clone(),
                            item_binding.clone(),
                        )],
                    )],
                )],
                vec![EventData::new(
                    event_name,
                    vec![EventFieldData::new(item_field, ValueKind::Entity)],
                )],
            )),
        );
        let definitions = valid(DefinitionLinker::link(valid(ExactPackSet::finalize(
            ExactPackageSelection::new(
                coordinate.clone(),
                vec![SelectedPackage::new(
                    coordinate,
                    SourceSnapshotId::from_bytes([0x61; 32]),
                    Vec::new(),
                )],
            ),
            vec![artifact],
        ))));
        let [interface] = definitions.required_interfaces() else {
            panic!("transfer fixture must require one interface");
        };
        let interface = interface.clone();
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            definitions,
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            vec![SemanticImplementationBinding::new(
                interface,
                SemanticImplementationId::from_bytes([0x71; 32]),
            )],
        ));
        ContainmentActionDefinitionFixture {
            semantics,
            action,
            actor_binding,
            item_binding,
            source_binding,
            destination_binding,
        }
    }

    fn scheduled_containment_action_fixture(
        accepted: AcceptedState,
        opportunity: ActionOpportunity,
        item: EntityId,
        source: EntityId,
        destination: EntityId,
    ) -> (HeadFixture, SchedulerKey, CommandEnvelope) {
        let definitions = containment_action_definitions();
        let root = valid(InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted.clone(),
            Vec::new(),
        ));
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &definitions.semantics,
            RootSeed::from_bytes([0x81; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        let closure = valid(ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            definitions.semantics,
        ));
        let command = valid(CommandEnvelope::new(
            closure.semantics().definitions(),
            CommandSource::derive_action(opportunity.id()),
            CommandId::new(0),
            opportunity.actor(),
            definitions.action,
            vec![
                CommandBinding::new(
                    definitions.actor_binding,
                    CommandValue::Actor(opportunity.actor()),
                ),
                CommandBinding::new(definitions.item_binding, CommandValue::Entity(item)),
                CommandBinding::new(definitions.source_binding, CommandValue::Entity(source)),
                CommandBinding::new(
                    definitions.destination_binding,
                    CommandValue::Entity(destination),
                ),
            ],
        ));
        let effective = moment(1, 0);
        let scheduled = crate::scheduler::ScheduledCommand::from_action_opportunity(
            opportunity.id(),
            effective,
            command.clone(),
        );
        let mut scheduler = SchedulerState::empty();
        let plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(scheduled),
            )])
            .unwrap_or_else(|error| panic!("action command must plan: {error:?}"));
        let key = plan.entries()[0].0;
        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("action command must install: {error:?}"));
        let mut control = RuntimeControlState::empty();
        control
            .open_action_opportunity(opportunity.clone())
            .unwrap_or_else(|error| panic!("opportunity must open: {error:?}"));
        control
            .consume_action_opportunity(
                opportunity.id(),
                opportunity.version(),
                ActionOpportunityDisposition::ActionSubmitted,
            )
            .unwrap_or_else(|error| panic!("opportunity must be consumed: {error:?}"));
        let cursor = SessionHead::root(&closure).cursor();
        let head = SessionHead::from_authority_projection(
            cursor,
            SessionMode::Running,
            SessionClock::from_coordinates(SimMoment::ORIGIN, SimMoment::ORIGIN),
            accepted,
            control,
            scheduler,
            None,
        );
        (HeadFixture { closure, head }, key, command)
    }

    fn false_containment_belief(
        actor: ActorId,
        item: EntityId,
        actual_source: EntityId,
        believed_source: EntityId,
        destination: EntityId,
    ) -> AcceptedState {
        let domain = valid(DomainState::new(
            vec![
                ContainerRecord::new(actual_source, 2),
                ContainerRecord::new(believed_source, 2),
                ContainerRecord::new(destination, 2),
            ],
            vec![ContainmentRecord::new(item, actual_source)],
            vec![ContainerAuthorityRecord::new(actor, believed_source)],
        ));
        let belief = world_model::EvidenceRecord::direct_physical_event(
            actor,
            EvidenceDeliveryGeneration::new(1).unwrap_or_else(|| unreachable!()),
            PhysicalEvent::item_transferred(valid(ContainmentTransferDelta::new(
                actor,
                item,
                actual_source,
                believed_source,
            ))),
        );
        let epistemic = EpistemicState::empty()
            .assimilate(actor, world_model::EpistemicVersion::EMPTY, vec![belief])
            .unwrap_or_else(|error| panic!("false belief must assimilate: {error:?}"));
        AcceptedState::new(
            domain,
            epistemic,
            SocialState::empty(),
            AgencyState::empty(),
        )
    }

    fn containment_opportunity(
        actor: ActorId,
        sponsor: ActionSponsor,
        item: EntityId,
        source: EntityId,
        destination: EntityId,
    ) -> ActionOpportunity {
        ActionOpportunity::open(
            actor,
            sponsor,
            ActionInteractionScope::containment(valid(ContainmentInteractionScope::new(
                source,
                vec![destination],
                vec![item],
                1,
            ))),
            ActionOpportunityGeneration::new(0),
        )
    }

    fn apply_rejected_containment_action(
        accepted: AcceptedState,
        opportunity: ActionOpportunity,
        item: EntityId,
        source: EntityId,
        destination: EntityId,
        reason: StableCommandRejection,
    ) -> crate::authority::AppliedAuthorityRecord {
        let (fixture, key, command) =
            scheduled_containment_action_fixture(accepted, opportunity, item, source, destination);
        let identity = ContainmentCommandIdentity::from_command(&command);
        let draft = checked_draft(
            &fixture,
            vec![evaluable_delivery(&fixture, key)],
            &[(identity, CommandProposal::Rejected(reason))],
            false,
        );
        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(fixture.head.cursor(), draft),
        )
        .unwrap_or_else(|error| panic!("rejected action attempt must seal: {error:?}"));
        crate::authority::apply_authority_record(&fixture.head, sealed)
            .unwrap_or_else(|error| panic!("rejected action attempt must replay: {error:?}"))
    }

    #[test]
    fn rejected_containment_attempt_routes_only_private_absence_feedback() {
        let actor = ActorId::from_bytes([0x11; 32]);
        let item = EntityId::from_bytes([0x12; 32]);
        let actual_source = EntityId::from_bytes([0x13; 32]);
        let believed_source = EntityId::from_bytes([0x14; 32]);
        let destination = EntityId::from_bytes([0x15; 32]);
        let sponsors = [
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x21; 32])),
            ActionSponsor::activity(
                world_model::ActivityId::from_bytes([0x22; 32]),
                world_model::ActivityVersion::INITIAL,
            ),
        ];

        for sponsor in sponsors {
            let accepted =
                false_containment_belief(actor, item, actual_source, believed_source, destination);
            let opportunity =
                containment_opportunity(actor, sponsor, item, believed_source, destination);
            let applied = apply_rejected_containment_action(
                accepted,
                opportunity,
                item,
                believed_source,
                destination,
                StableCommandRejection::Stale,
            );
            let (head, record) = applied.into_parts();
            let AuthorityRecordBody::Moment(batch) = record.body() else {
                panic!("rejected attempt must publish a moment");
            };
            let [routing] = batch.evidence_routing() else {
                panic!("exact mismatch must route one feedback record");
            };
            assert!(matches!(
                routing.source(),
                EvidenceRoutingSource::RejectedContainmentAttempt { attempt }
                    if attempt.index() == 0
            ));
            let evidence = routing.evidence();
            let EvidenceProvenance::DirectItemAbsent(observation) = evidence.provenance() else {
                panic!("feedback must contain only absence evidence");
            };
            assert_eq!(evidence.observer(), actor);
            assert_eq!(observation.item(), item);
            assert_eq!(observation.expected_container(), believed_source);
            assert_ne!(observation.expected_container(), actual_source);
            assert!(batch.scheduler_insertions().iter().any(|insertion| {
                matches!(
                    insertion.work(),
                    ScheduledWork::Lifecycle(LifecycleWork::EvidenceDelivery(delivery))
                        if delivery.evidence() == evidence && delivery.due() > batch.moment()
                )
            }));
            assert_eq!(
                head.accepted()
                    .domain()
                    .containment_for(item)
                    .map(|record| record.container()),
                Some(actual_source)
            );
        }
    }

    #[test]
    fn containment_feedback_excludes_unrelated_rejections_and_nonmatching_beliefs() {
        let actor = ActorId::from_bytes([0x31; 32]);
        let item = EntityId::from_bytes([0x32; 32]);
        let actual_source = EntityId::from_bytes([0x33; 32]);
        let believed_source = EntityId::from_bytes([0x34; 32]);
        let attempted_source = EntityId::from_bytes([0x35; 32]);
        let destination = EntityId::from_bytes([0x36; 32]);

        for (reason, source) in [
            (StableCommandRejection::Conflict, believed_source),
            (StableCommandRejection::BindingMismatch, believed_source),
            (StableCommandRejection::Stale, attempted_source),
        ] {
            let accepted =
                false_containment_belief(actor, item, actual_source, believed_source, destination);
            let opportunity = containment_opportunity(
                actor,
                ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x41; 32])),
                item,
                source,
                destination,
            );
            let applied = apply_rejected_containment_action(
                accepted,
                opportunity,
                item,
                source,
                destination,
                reason,
            );
            let (_, record) = applied.into_parts();
            let AuthorityRecordBody::Moment(batch) = record.body() else {
                panic!("rejected attempt must publish a moment");
            };
            assert!(batch.evidence_routing().is_empty());
            assert!(!batch.scheduler_insertions().iter().any(|insertion| {
                matches!(
                    insertion.work(),
                    ScheduledWork::Lifecycle(LifecycleWork::EvidenceDelivery(_))
                )
            }));
        }
    }

    #[test]
    fn retained_rejected_containment_attempt_does_not_repeat_feedback() {
        let actor = ActorId::from_bytes([0x51; 32]);
        let item = EntityId::from_bytes([0x52; 32]);
        let actual_source = EntityId::from_bytes([0x53; 32]);
        let believed_source = EntityId::from_bytes([0x54; 32]);
        let destination = EntityId::from_bytes([0x55; 32]);
        let accepted =
            false_containment_belief(actor, item, actual_source, believed_source, destination);
        let opportunity = containment_opportunity(
            actor,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x56; 32])),
            item,
            believed_source,
            destination,
        );
        let (mut fixture, key, command) = scheduled_containment_action_fixture(
            accepted,
            opportunity,
            item,
            believed_source,
            destination,
        );
        let original_attempt = AttemptRecordId::from_bytes([0x57; 32]);
        let outcome = world_model::CommandAttemptOutcome::Rejected(StableCommandRejection::Stale);
        let mut control = fixture.head.runtime_control().clone();
        control
            .command_mut()
            .insert_exact(
                command.source(),
                command.id(),
                command.fingerprint(),
                original_attempt,
                outcome,
            )
            .unwrap_or_else(|error| panic!("retained attempt must install: {error:?}"));
        fixture.head = SessionHead::from_authority_projection(
            fixture.head.cursor(),
            fixture.head.mode(),
            fixture.head.clock(),
            fixture.head.accepted().clone(),
            control,
            fixture.head.scheduler().clone(),
            None,
        );
        let scheduled = match fixture.head.scheduler().get(key) {
            Some(ScheduledWork::Command(scheduled)) => scheduled.as_ref().clone(),
            _ => panic!("fixture key must contain one command"),
        };
        let draft = checked_draft(
            &fixture,
            vec![PreparedDelivery::resolved_command(
                key,
                scheduled,
                crate::kernel::PreparedCommandResolution::Retained {
                    original_attempt,
                    outcome,
                },
            )],
            &[],
            false,
        );
        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(fixture.head.cursor(), draft),
        )
        .unwrap_or_else(|error| panic!("retained attempt must seal: {error:?}"));
        let applied = crate::authority::apply_authority_record(&fixture.head, sealed)
            .unwrap_or_else(|error| panic!("retained attempt must replay: {error:?}"));
        let (_, record) = applied.into_parts();
        let AuthorityRecordBody::Moment(batch) = record.body() else {
            panic!("retained attempt must publish a moment");
        };

        assert!(batch.attempts().is_empty());
        assert!(batch.evidence_routing().is_empty());
        assert!(!batch.scheduler_insertions().iter().any(|insertion| {
            matches!(
                insertion.work(),
                ScheduledWork::Lifecycle(LifecycleWork::EvidenceDelivery(_))
            )
        }));
    }

    #[test]
    fn action_command_validation_rejects_an_item_outside_the_exact_scope() {
        let actor = ActorId::from_bytes([0x11; 32]);
        let permitted_item = EntityId::from_bytes([0x22; 32]);
        let out_of_scope_item = EntityId::from_bytes([0x23; 32]);
        let source = EntityId::from_bytes([0x31; 32]);
        let destination = EntityId::from_bytes([0x41; 32]);
        let opportunity = ActionOpportunity::open(
            actor,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x51; 32])),
            world_model::ActionInteractionScope::containment(valid(
                ContainmentInteractionScope::new(
                    source,
                    vec![destination],
                    vec![permitted_item],
                    8,
                ),
            )),
            ActionOpportunityGeneration::new(0),
        );

        let actor_binding = valid(BindingName::parse("actor"));
        let item_binding = valid(BindingName::parse("item"));
        let source_binding = valid(BindingName::parse("source"));
        let destination_binding = valid(BindingName::parse("destination"));
        let interface_key = valid(SemanticInterfaceKey::parse("test.containment-transfer"));
        let allowed = valid(OperationName::parse("allowed"));
        let apply = valid(OperationName::parse("apply"));
        let parameters = || {
            vec![
                OperationParameter::new(valid(ParameterName::parse("actor")), ValueKind::Actor),
                OperationParameter::new(valid(ParameterName::parse("item")), ValueKind::Entity),
                OperationParameter::new(valid(ParameterName::parse("source")), ValueKind::Entity),
                OperationParameter::new(
                    valid(ParameterName::parse("destination")),
                    ValueKind::Entity,
                ),
            ]
        };
        let descriptor = valid(SemanticInterfaceDescriptor::new(
            interface_key.clone(),
            valid(InterfaceVersion::new(1)),
            vec![
                valid(SemanticOperationDescriptor::new(
                    allowed.clone(),
                    OperationKind::Predicate,
                    parameters(),
                )),
                valid(SemanticOperationDescriptor::new(
                    apply.clone(),
                    OperationKind::Effect,
                    parameters(),
                )),
            ],
        ));
        let coordinate = PackCoordinate::new(
            valid(PackKey::parse("test.containment")),
            PackVersion::new(1, 0, 0),
        );
        let action_name = valid(LocalDefinitionName::parse("transfer"));
        let action = DefinitionKey::new(coordinate.pack_key().clone(), action_name.clone());
        let event_name = valid(LocalDefinitionName::parse("transferred"));
        let item_field = valid(EventFieldName::parse("item"));
        let arguments = vec![
            actor_binding.clone(),
            item_binding.clone(),
            source_binding.clone(),
            destination_binding.clone(),
        ];
        let artifact = valid(
            ArtifactValidator::new(&valid(SemanticInterfaceCatalog::new(vec![
                descriptor.clone(),
            ])))
            .validate(ArtifactData::new(
                PackManifestData::new(
                    EngineProtocolVersion::new(1),
                    coordinate.clone(),
                    Vec::new(),
                ),
                vec![descriptor.reference()],
                vec![ActionData::new(
                    action_name,
                    vec![
                        ActionBindingData::new(actor_binding.clone(), ValueKind::Actor),
                        ActionBindingData::new(item_binding.clone(), ValueKind::Entity),
                        ActionBindingData::new(source_binding.clone(), ValueKind::Entity),
                        ActionBindingData::new(destination_binding.clone(), ValueKind::Entity),
                    ],
                    vec![RuntimeRequirementData::new(OperationCallData::new(
                        interface_key.clone(),
                        allowed,
                        arguments.clone(),
                    ))],
                    vec![EffectCallData::new(OperationCallData::new(
                        interface_key,
                        apply,
                        arguments,
                    ))],
                    vec![EventEmissionData::new(
                        DefinitionKey::new(coordinate.pack_key().clone(), event_name.clone()),
                        vec![EventFieldBindingData::new(
                            item_field.clone(),
                            item_binding.clone(),
                        )],
                    )],
                )],
                vec![EventData::new(
                    event_name,
                    vec![EventFieldData::new(item_field, ValueKind::Entity)],
                )],
            )),
        );
        let definitions = valid(DefinitionLinker::link(valid(ExactPackSet::finalize(
            ExactPackageSelection::new(
                coordinate.clone(),
                vec![SelectedPackage::new(
                    coordinate,
                    SourceSnapshotId::from_bytes([0x61; 32]),
                    Vec::new(),
                )],
            ),
            vec![artifact],
        ))));
        let [interface] = definitions.required_interfaces() else {
            panic!("transfer fixture must require one interface");
        };
        let interface = interface.clone();
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            definitions,
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            vec![SemanticImplementationBinding::new(
                interface,
                SemanticImplementationId::from_bytes([0x71; 32]),
            )],
        ));
        let root = valid(InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            empty_accepted_state(),
            vec![opportunity.clone()],
        ));
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &semantics,
            RootSeed::from_bytes([0x81; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        let closure = valid(ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ));
        let head = SessionHead::root(&closure);
        let command = |item| {
            valid(CommandEnvelope::new(
                closure.semantics().definitions(),
                CommandSource::derive_action(opportunity.id()),
                CommandId::new(0),
                actor,
                action.clone(),
                vec![
                    CommandBinding::new(actor_binding.clone(), CommandValue::Actor(actor)),
                    CommandBinding::new(item_binding.clone(), CommandValue::Entity(item)),
                    CommandBinding::new(source_binding.clone(), CommandValue::Entity(source)),
                    CommandBinding::new(
                        destination_binding.clone(),
                        CommandValue::Entity(destination),
                    ),
                ],
            ))
        };
        let permitted = command(permitted_item);
        let out_of_scope = command(out_of_scope_item);

        assert_eq!(
            validate_action_command(&head, opportunity.id(), &permitted, &closure),
            Ok(())
        );
        assert_eq!(
            validate_action_command(&head, opportunity.id(), &out_of_scope, &closure),
            Err(AuthorityRecordSealError::InvalidNormalizedGraph)
        );
    }

    #[test]
    fn action_command_validation_rejects_a_non_containment_family() {
        let actor = ActorId::from_bytes([0x11; 32]);
        let source = EntityId::from_bytes([0x21; 32]);
        let destination = EntityId::from_bytes([0x31; 32]);
        let opportunity = ActionOpportunity::open(
            actor,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x41; 32])),
            world_model::ActionInteractionScope::containment(valid(
                ContainmentInteractionScope::new(
                    source,
                    vec![destination],
                    vec![EntityId::from_bytes([0x22; 32])],
                    8,
                ),
            )),
            ActionOpportunityGeneration::new(0),
        );

        let definitions = crate::kernel::fixtures::command_definitions();
        let artifact = definitions
            .artifacts()
            .first()
            .unwrap_or_else(|| panic!("command fixture must contain one artifact"));
        let definition = artifact
            .actions()
            .first()
            .unwrap_or_else(|| panic!("command fixture must contain one action"));
        let action = world_defs::DefinitionKey::new(
            artifact.coordinate().pack_key().clone(),
            definition.name().clone(),
        );
        let actor_binding = definition
            .bindings()
            .first()
            .unwrap_or_else(|| panic!("command fixture action must bind its actor"))
            .name()
            .clone();
        let interface = definitions
            .required_interfaces()
            .first()
            .unwrap_or_else(|| panic!("command fixture must require one interface"))
            .clone();
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            definitions,
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            vec![SemanticImplementationBinding::new(
                interface,
                SemanticImplementationId::from_bytes([0x51; 32]),
            )],
        ));
        let root = valid(InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            empty_accepted_state(),
            vec![opportunity.clone()],
        ));
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &semantics,
            RootSeed::from_bytes([0x61; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        let closure = valid(ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ));
        let head = SessionHead::root(&closure);
        let command = valid(CommandEnvelope::new(
            closure.semantics().definitions(),
            CommandSource::derive_action(opportunity.id()),
            CommandId::new(0),
            actor,
            action,
            vec![CommandBinding::new(
                actor_binding,
                CommandValue::Actor(actor),
            )],
        ));

        assert_eq!(
            validate_action_command(&head, opportunity.id(), &command, &closure),
            Err(AuthorityRecordSealError::InvalidNormalizedGraph)
        );
    }

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(ticks), Microstep::new(microstep))
    }

    fn head_with_commands(
        mode: SessionMode,
        now: SimMoment,
        frontier: SimMoment,
        accepted: AcceptedState,
        effective_moments: &[SimMoment],
    ) -> (HeadFixture, Vec<SchedulerKey>) {
        let root = root_fixture();
        let namespace = derive_input_request_namespace(
            root.head.cursor().epoch().lineage(),
            root.closure.specification().external_input_digest(),
        );
        let mut insertions = Vec::with_capacity(effective_moments.len());

        for (index, effective) in effective_moments.iter().copied().enumerate() {
            let fixture_byte = u8::try_from(index).unwrap_or_else(|error| {
                panic!("authority fixture command count must fit u8: {error}")
            });
            let request = crate::kernel::AdmitRequest::new(
                InputId::new(u64::try_from(index).unwrap_or_else(|error| {
                    panic!("authority fixture command count must fit u64: {error}")
                })),
                effective,
                crate::kernel::fixtures::command(
                    0x90_u8.wrapping_add(fixture_byte),
                    100_u64
                        .checked_add(u64::try_from(index).unwrap_or_else(|error| {
                            panic!("authority fixture command count must fit u64: {error}")
                        }))
                        .unwrap_or_else(|| {
                            panic!("authority fixture command identity must fit u64")
                        }),
                ),
            );
            let command = PreparedScheduledCommand::prepare(namespace, &request).materialize(
                CapturedInputRecordId::from_bytes([0xa0_u8.wrapping_add(fixture_byte); 32]),
            );
            let ordinal = u32::try_from(index).unwrap_or_else(|error| {
                panic!("authority fixture command count must fit u32: {error}")
            });
            insertions.push(SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(ordinal),
                ScheduledWork::command(command),
            ));
        }

        let mut scheduler = SchedulerState::empty();
        let plan = scheduler
            .plan_batch(insertions)
            .unwrap_or_else(|error| panic!("authority fixture commands must plan: {error:?}"));
        let keys = plan.entries().iter().map(|(key, _)| *key).collect();
        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("authority fixture commands must install: {error:?}"));

        let head = SessionHead::from_authority_projection(
            root.head.cursor(),
            mode,
            SessionClock::from_coordinates(now, frontier),
            accepted,
            RuntimeControlState::empty(),
            scheduler,
            None,
        );
        (
            HeadFixture {
                closure: root.closure,
                head,
            },
            keys,
        )
    }

    fn head_with_command_batch(
        effective: SimMoment,
        count: usize,
    ) -> (
        HeadFixture,
        Vec<(SchedulerKey, crate::scheduler::ScheduledCommand)>,
    ) {
        let root = root_fixture();
        let namespace = derive_input_request_namespace(
            root.head.cursor().epoch().lineage(),
            root.closure.specification().external_input_digest(),
        );
        let mut insertions = Vec::with_capacity(count);
        for index in 0..count {
            let local = u32::try_from(index)
                .unwrap_or_else(|error| panic!("batch fixture index must fit u32: {error}"));
            let input = u64::try_from(index)
                .unwrap_or_else(|error| panic!("batch fixture input must fit u64: {error}"));
            let fixture_byte = u8::try_from(index)
                .unwrap_or_else(|error| panic!("batch fixture count must fit u8: {error}"));
            let request = crate::kernel::AdmitRequest::new(
                InputId::new(input),
                effective,
                crate::kernel::fixtures::command(0xd0_u8.wrapping_add(fixture_byte), input),
            );
            let captured =
                CapturedInputRecordId::from_bytes([0xe0_u8.wrapping_add(fixture_byte); 32]);
            let scheduled =
                PreparedScheduledCommand::prepare(namespace, &request).materialize(captured);
            insertions.push(SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(local),
                ScheduledWork::command(scheduled),
            ));
        }

        let mut scheduler = SchedulerState::empty();
        let plan = scheduler
            .plan_batch(insertions)
            .unwrap_or_else(|error| panic!("command batch fixture must plan: {error:?}"));
        let entries = plan
            .entries()
            .iter()
            .map(|(key, work)| match work {
                ScheduledWork::Command(command) => (*key, command.as_ref().clone()),
                ScheduledWork::PostCommit(_) => {
                    panic!("command batch fixture must contain only commands")
                }
                ScheduledWork::ActionReady(_)
                | ScheduledWork::ActionEvaluation(_)
                | ScheduledWork::Lifecycle(_)
                | ScheduledWork::Process(_) => {
                    unreachable!("command batch fixture cannot contain action work")
                }
            })
            .collect();
        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("command batch fixture must install: {error:?}"));
        let accepted = empty_accepted_state();
        let head = SessionHead::from_authority_projection(
            root.head.cursor(),
            SessionMode::Running,
            SessionClock::from_coordinates(SimMoment::ORIGIN, SimMoment::ORIGIN),
            accepted,
            RuntimeControlState::empty(),
            scheduler,
            None,
        );
        (
            HeadFixture {
                closure: root.closure,
                head,
            },
            entries,
        )
    }

    fn head_with_post_commit(
        source: SimMoment,
    ) -> (
        HeadFixture,
        SchedulerKey,
        crate::scheduler::PostCommitDispatchId,
        ReactionEnvelopeId,
    ) {
        let root = root_fixture();
        let reaction = crate::scheduler::ReactionEnvelope::from_transfers(&[transfer_fixture().1])
            .unwrap_or_else(|| panic!("one transfer must produce one reaction envelope"));
        let prepared = PreparedPostCommitDispatch::prepare(
            root.head.cursor().epoch().lineage(),
            source,
            reaction,
        );
        let dispatch_id = prepared.id();
        let reaction_id = ReactionEnvelopeId::from_bytes([0xd1; 32]);
        let dispatch = prepared.materialize(reaction_id);
        let mut scheduler = SchedulerState::empty();
        let plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::PostCommit(dispatch),
            )])
            .unwrap_or_else(|error| panic!("post-commit fixture must plan: {error:?}"));
        let key = plan
            .entries()
            .first()
            .map(|(key, _)| *key)
            .unwrap_or_else(|| panic!("post-commit fixture plan must contain one entry"));
        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("post-commit fixture must install: {error:?}"));
        let accepted = empty_accepted_state();

        let head = SessionHead::from_authority_projection(
            root.head.cursor(),
            SessionMode::Running,
            SessionClock::from_coordinates(SimMoment::ORIGIN, SimMoment::ORIGIN),
            accepted,
            RuntimeControlState::empty(),
            scheduler,
            None,
        );
        (
            HeadFixture {
                closure: root.closure,
                head,
            },
            key,
            dispatch_id,
            reaction_id,
        )
    }

    fn transfer_fixture() -> (AcceptedState, ContainmentTransferDelta) {
        let actor = ActorId::from_bytes([0x41; 32]);
        let source = EntityId::from_bytes([0xb1; 32]);
        let destination = EntityId::from_bytes([0xb2; 32]);
        let item = EntityId::from_bytes([0xb3; 32]);
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

    fn capacity_conflict_fixture() -> (AcceptedState, [ContainmentTransferDelta; 2]) {
        let actor = ActorId::from_bytes([0x41; 32]);
        let source = EntityId::from_bytes([0xc1; 32]);
        let destination = EntityId::from_bytes([0xc2; 32]);
        let first_item = EntityId::from_bytes([0xc3; 32]);
        let second_item = EntityId::from_bytes([0xc4; 32]);
        let accepted = accepted_state(
            vec![
                ContainerRecord::new(source, 2),
                ContainerRecord::new(destination, 1),
            ],
            vec![
                ContainmentRecord::new(first_item, source),
                ContainmentRecord::new(second_item, source),
            ],
            vec![ContainerAuthorityRecord::new(actor, source)],
        );
        (
            accepted,
            [
                valid(ContainmentTransferDelta::new(
                    actor,
                    first_item,
                    source,
                    destination,
                )),
                valid(ContainmentTransferDelta::new(
                    actor,
                    second_item,
                    source,
                    destination,
                )),
            ],
        )
    }

    fn evaluable_delivery(fixture: &HeadFixture, key: SchedulerKey) -> PreparedDelivery {
        match fixture.head.scheduler().get(key) {
            Some(ScheduledWork::Command(scheduled)) => {
                PreparedDelivery::evaluable_command(key, scheduled.as_ref().clone())
            }
            _ => panic!("fixture key must contain a command"),
        }
    }

    fn checked_draft(
        fixture: &HeadFixture,
        deliveries: Vec<PreparedDelivery>,
        command_proposals: &[(ContainmentCommandIdentity, CommandProposal)],
        reverse_decisions: bool,
    ) -> DraftMomentBatch {
        checked_draft_resolved_at(
            fixture,
            deliveries,
            command_proposals,
            reverse_decisions,
            None,
        )
    }

    fn checked_draft_resolved_at(
        fixture: &HeadFixture,
        deliveries: Vec<PreparedDelivery>,
        command_proposals: &[(ContainmentCommandIdentity, CommandProposal)],
        reverse_decisions: bool,
        resolution_moment: Option<SimMoment>,
    ) -> DraftMomentBatch {
        let due = deliveries
            .first()
            .map(PreparedDelivery::key)
            .unwrap_or_else(|| panic!("authority draft fixture must contain one delivery"))
            .moment();
        let resulting_frontier =
            fixture
                .head
                .clock()
                .frontier()
                .max(strictly_later_moment(due).unwrap_or_else(|error| {
                    panic!("authority fixture moment must have a successor: {error:?}")
                }));
        let prepared = PreparedFire::new(
            AttemptAuthorityDomainId::from_bytes([0x71; 32]),
            RunAttemptId::from_bytes([0x72; 32]),
            fixture.closure.specification().id(),
            AttemptStepId::from_bytes([0x73; 32]),
            ReservationGrant::FIRST,
            resulting_frontier,
            fixture.head.snapshot(),
            deliveries,
        )
        .unwrap_or_else(|error| panic!("prepared authority fixture must be valid: {error:?}"));

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
                    let candidate = match proposal {
                        CommandProposal::Rejected(reason) => {
                            ContainmentCandidateProposal::Rejected(reason)
                        }
                        CommandProposal::AcceptedTransfer(delta) => {
                            ContainmentCandidateProposal::Transfer(delta)
                        }
                    };
                    candidates.push(ContainmentCandidate::new(
                        identity,
                        command.actor(),
                        candidate,
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
        if reverse_decisions {
            decisions.reverse();
        }
        let proposals = MomentWorkProposals::from_decisions(&prepared, decisions)
            .unwrap_or_else(|error| panic!("authority decisions must be complete: {error:?}"));
        let candidates = ContainmentCandidateSet::new(candidates)
            .unwrap_or_else(|error| panic!("authority candidates must be unique: {error:?}"));
        let oracle =
            Blake3KeyedPrf256V1::from_root_seed(fixture.closure.specification().root_seed());
        let resolution = resolve_containment_candidates(
            resolution_moment.unwrap_or(prepared.moment()),
            prepared.base_snapshot().accepted(),
            &candidates,
            &oracle,
        );
        DraftMomentBatch::from_prepared(&prepared, &proposals, &resolution)
            .unwrap_or_else(|error| panic!("authority draft must be checked: {error:?}"))
    }

    #[test]
    fn ingress_rejects_a_command_bound_to_another_definition_set() {
        let fixture = root_fixture();
        let request = crate::kernel::AdmitRequest::new(
            InputId::new(4),
            SimMoment::ORIGIN,
            crate::kernel::fixtures::command(0x82, 5),
        );
        assert!(matches!(
            seal_authority_record(
                &fixture.head,
                &fixture.closure,
                DraftAuthorityRecord::admit_commands(fixture.head.cursor(), vec![request]),
            ),
            Err(AuthorityRecordSealError::CommandDefinitionSetMismatch { .. })
        ));
    }

    #[test]
    fn ingress_materialization_derives_inner_provenance_from_outer_record() {
        let namespace = crate::execution::ExternalInputNamespaceId::from_bytes([0x83; 32]);
        let request = crate::kernel::AdmitRequest::new(
            InputId::new(4),
            SimMoment::ORIGIN,
            crate::kernel::fixtures::command(0x82, 5),
        );
        let prepared = PreparedScheduledCommand::prepare(namespace, &request);
        let plan = crate::scheduler::SchedulerState::empty()
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                ScheduledWork::command(
                    prepared
                        .clone()
                        .materialize(CapturedInputRecordId::from_bytes([0x85; 32])),
                ),
            )])
            .unwrap_or_else(|error| panic!("ingress schedule must plan: {error:?}"));
        let scheduler_key = plan
            .entries()
            .first()
            .map(|(key, _)| *key)
            .unwrap_or_else(|| panic!("ingress schedule must contain one entry"));
        let record = AuthorityRecordId::from_bytes([0x84; 32]);
        let body = materialize(
            record,
            NormalizedAuthorityRecordBody::Admission(NormalizedAuthorityAdmission::Commands(vec![
                NormalizedIngressRecord {
                    prepared,
                    scheduler_key,
                },
            ])),
        );
        let ingress = match &body {
            AuthorityRecordBody::Admission(AuthorityAdmissionRecord::Commands(ingress)) => ingress,
            _ => panic!("ingress draft must produce an ingress body"),
        };
        let entry = ingress
            .entries()
            .first()
            .unwrap_or_else(|| panic!("materialized ingress must contain one entry"));
        assert_eq!(
            entry.captured().id(),
            CapturedInputRecordId::derive(record, CapturedInputLocalIndex::new(0))
        );
        assert_eq!(entry.outcome().record(), record);
        assert_eq!(entry.scheduler_key().moment(), entry.captured().effective());
        assert_eq!(
            entry.scheduled_command().captured(),
            Some(entry.captured().id())
        );
        assert_eq!(
            entry.captured().command(),
            entry.scheduled_command().command()
        );
    }

    #[test]
    fn management_seal_materializes_current_record_outcome() {
        let fixture = root_fixture();
        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::management(
                fixture.head.cursor(),
                vec![crate::kernel::ManageRequest::new(
                    ManagementRequestId::new(9),
                    SessionManagement::Pause,
                )],
            ),
        )
        .unwrap_or_else(|error| panic!("pause fixture must seal: {error:?}"));
        assert_eq!(sealed.expected_cursor(), fixture.head.cursor());
        assert_eq!(sealed.resulting_cursor().revision().get(), 1);
        assert_eq!(sealed.record().header().revision().get(), 1);
        assert_eq!(sealed.record().header().sequence().get(), 1);
        assert_eq!(
            sealed.record().header().cumulative(),
            sealed.resulting_cursor().cumulative()
        );
        let management = match sealed.record().body() {
            AuthorityRecordBody::Management(management) => management,
            _ => panic!("management draft must produce a management body"),
        };
        assert_eq!(management.resulting_mode(), SessionMode::Paused);
        let entry = management
            .entries()
            .first()
            .unwrap_or_else(|| panic!("management batch must contain one entry"));
        assert_eq!(entry.outcome().record(), sealed.record().header().id());

        assert_eq!(
            seal_authority_record(
                &fixture.head,
                &fixture.closure,
                DraftAuthorityRecord::management(
                    fixture.head.cursor(),
                    vec![crate::kernel::ManageRequest::new(
                        ManagementRequestId::new(10),
                        SessionManagement::Resume,
                    )],
                ),
            ),
            Err(AuthorityRecordSealError::IllegalManagementTransition {
                current: SessionMode::Running,
                requested: SessionMode::Running,
            })
        );
    }

    #[test]
    fn management_batch_is_input_permutation_invariant() {
        let fixture = root_fixture();
        let pause = crate::kernel::ManageRequest::new(
            ManagementRequestId::new(1),
            SessionManagement::Pause,
        );
        let resume = crate::kernel::ManageRequest::new(
            ManagementRequestId::new(2),
            SessionManagement::Resume,
        );

        let forward = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::management(fixture.head.cursor(), vec![pause, resume]),
        )
        .unwrap_or_else(|error| panic!("canonical management batch must seal: {error:?}"));
        let reversed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::management(fixture.head.cursor(), vec![resume, pause]),
        )
        .unwrap_or_else(|error| panic!("permuted management batch must seal: {error:?}"));

        assert_eq!(forward, reversed);
        let AuthorityRecordBody::Management(batch) = forward.record().body() else {
            panic!("management draft must produce a management batch");
        };
        assert_eq!(batch.entries().len(), 2);
        assert_eq!(batch.resulting_mode(), SessionMode::Running);
    }

    #[test]
    fn moment_seal_requires_and_canonicalizes_the_complete_due_set() {
        let due = moment(7, 0);
        let (fixture, entries) = head_with_command_batch(due, 2);
        let proposals = entries
            .iter()
            .map(|(_, scheduled)| {
                (
                    ContainmentCommandIdentity::from_command(scheduled.command()),
                    CommandProposal::Rejected(StableCommandRejection::Conflict),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            seal_authority_record(
                &fixture.head,
                &fixture.closure,
                DraftAuthorityRecord::moment(
                    fixture.head.cursor(),
                    checked_draft(
                        &fixture,
                        vec![evaluable_delivery(&fixture, entries[0].0)],
                        &proposals[..1],
                        false,
                    ),
                ),
            ),
            Err(AuthorityRecordSealError::IncompleteDueSet {
                expected: 2,
                supplied: 1,
            })
        );

        let forward = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(
                fixture.head.cursor(),
                checked_draft(
                    &fixture,
                    entries
                        .iter()
                        .map(|(key, _)| evaluable_delivery(&fixture, *key))
                        .collect(),
                    &proposals,
                    false,
                ),
            ),
        )
        .unwrap_or_else(|error| panic!("complete due set must seal: {error:?}"));
        let reversed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(
                fixture.head.cursor(),
                checked_draft(
                    &fixture,
                    entries
                        .iter()
                        .rev()
                        .map(|(key, _)| evaluable_delivery(&fixture, *key))
                        .collect(),
                    &proposals,
                    true,
                ),
            ),
        )
        .unwrap_or_else(|error| panic!("permuted complete due set must seal: {error:?}"));

        assert_eq!(forward, reversed);
        let AuthorityRecordBody::Moment(batch) = forward.record().body() else {
            panic!("moment draft must produce a moment batch");
        };
        assert_eq!(batch.moment(), due);
        assert_eq!(
            batch.consumed_keys(),
            entries.iter().map(|(key, _)| *key).collect::<Vec<_>>()
        );
        assert_eq!(batch.command_deliveries().len(), 2);
        assert_eq!(batch.attempts().len(), 2);
    }

    #[test]
    fn accepted_transfer_schedules_one_batch_dispatch_at_the_first_vacant_later_moment() {
        let (accepted, delta) = transfer_fixture();
        let source = moment(4, 0);
        let occupied = moment(4, 1);
        let (fixture, keys) = head_with_commands(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted,
            &[source, occupied],
        );
        let scheduled = match fixture.head.scheduler().get(keys[0]) {
            Some(ScheduledWork::Command(scheduled)) => scheduled,
            _ => panic!("fixture key must contain a command"),
        };
        let proposal = [(
            ContainmentCommandIdentity::from_command(scheduled.command()),
            CommandProposal::AcceptedTransfer(delta),
        )];
        let draft = checked_draft(
            &fixture,
            vec![evaluable_delivery(&fixture, keys[0])],
            &proposal,
            false,
        );

        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(fixture.head.cursor(), draft),
        )
        .unwrap_or_else(|error| panic!("accepted transfer must seal: {error:?}"));
        let predecessor = fixture
            .head
            .cursor()
            .successor_plan()
            .unwrap_or_else(|error| panic!("fixture cursor must advance: {error:?}"));
        assert_eq!(sealed.record().header().lineage(), predecessor.lineage());
        assert_eq!(
            sealed.record().header().previous_authority_bytes(),
            predecessor.previous_authority().as_bytes()
        );
        assert_eq!(
            sealed.record().header().previous_cumulative(),
            predecessor.previous_cumulative()
        );
        let batch = match sealed.record().body() {
            AuthorityRecordBody::Moment(batch) => batch,
            _ => panic!("accepted transfer must produce a moment body"),
        };
        let delivery = &batch.command_deliveries()[0];
        let attempt = &batch.attempts()[0];
        let commit = batch.commits()[0];
        let reaction = &batch.reactions()[0];
        let insertion = &batch.scheduler_insertions()[0];
        let dispatch_key = insertion.scheduler_key();
        let ScheduledWork::PostCommit(dispatch) = insertion.work() else {
            panic!("accepted transfer must schedule one post-commit dispatch");
        };

        assert_eq!(delivery.scheduler_key(), keys[0]);
        assert_eq!(attempt.command(), Some(delivery.command()));
        assert_eq!(
            attempt.id(),
            AttemptRecordId::derive(sealed.record().header().id(), AttemptLocalIndex::new(0),)
        );
        assert!(matches!(
            attempt.resolution(),
            RecordedCommandResolution::Accepted { commit: id } if id == commit.id()
        ));
        assert_eq!(commit.delta(), delta);
        assert_eq!(reaction.envelope().events(), &[commit.event()]);
        assert_eq!(dispatch_key.moment(), moment(4, 1));
        assert_eq!(dispatch.source_moment(), source);
        assert_eq!(dispatch.reaction_id(), reaction.id());
        assert_eq!(dispatch.reaction(), reaction.envelope());
    }

    #[test]
    fn post_commit_work_is_consumed_through_its_typed_delivery() {
        let (post_commit_fixture, post_commit_key, _, _) = head_with_post_commit(moment(3, 0));
        let dispatch = match post_commit_fixture.head.scheduler().get(post_commit_key) {
            Some(ScheduledWork::PostCommit(dispatch)) => dispatch.clone(),
            _ => panic!("fixture key must contain post-commit work"),
        };
        let sealed = seal_authority_record(
            &post_commit_fixture.head,
            &post_commit_fixture.closure,
            DraftAuthorityRecord::moment(
                post_commit_fixture.head.cursor(),
                checked_draft(
                    &post_commit_fixture,
                    vec![PreparedDelivery::post_commit(post_commit_key, dispatch)],
                    &[],
                    false,
                ),
            ),
        )
        .unwrap_or_else(|error| panic!("exact post-commit delivery must seal: {error:?}"));
        let batch = match sealed.record().body() {
            AuthorityRecordBody::Moment(batch) => batch,
            _ => panic!("post-commit draft must produce a moment body"),
        };
        assert_eq!(batch.consumed_keys(), &[post_commit_key]);
        assert!(batch.command_deliveries().is_empty());
        assert_eq!(batch.post_commit_deliveries().len(), 1);
    }

    #[test]
    fn checked_graph_canonicalizes_proposal_completion_order() {
        let due = moment(7, 0);
        let (fixture, entries) = head_with_command_batch(due, 2);
        let proposals = entries
            .iter()
            .map(|(_, scheduled)| {
                (
                    ContainmentCommandIdentity::from_command(scheduled.command()),
                    CommandProposal::Rejected(StableCommandRejection::RequirementUnsatisfied),
                )
            })
            .collect::<Vec<_>>();
        let deliveries = || {
            entries
                .iter()
                .map(|(key, _)| evaluable_delivery(&fixture, *key))
                .collect()
        };

        let forward = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(
                fixture.head.cursor(),
                checked_draft(&fixture, deliveries(), &proposals, false),
            ),
        )
        .unwrap_or_else(|error| panic!("forward graph must seal: {error:?}"));
        let reversed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(
                fixture.head.cursor(),
                checked_draft(&fixture, deliveries(), &proposals, true),
            ),
        )
        .unwrap_or_else(|error| panic!("reverse-completed graph must seal: {error:?}"));

        assert_eq!(forward, reversed);
    }

    #[test]
    fn conflict_evidence_is_complete_and_wrong_moment_evidence_is_rejected() {
        let due = moment(8, 0);
        let (accepted, deltas) = capacity_conflict_fixture();
        let (mut fixture, entries) = head_with_command_batch(due, 2);
        fixture.head = SessionHead::from_authority_projection(
            fixture.head.cursor(),
            fixture.head.mode(),
            fixture.head.clock(),
            accepted,
            fixture.head.runtime_control().clone(),
            fixture.head.scheduler().clone(),
            fixture.head.safety_blocker(),
        );
        let proposals = entries
            .iter()
            .zip(deltas)
            .map(|((_, scheduled), delta)| {
                (
                    ContainmentCommandIdentity::from_command(scheduled.command()),
                    CommandProposal::AcceptedTransfer(delta),
                )
            })
            .collect::<Vec<_>>();
        let deliveries = || {
            entries
                .iter()
                .map(|(key, _)| evaluable_delivery(&fixture, *key))
                .collect()
        };

        let sealed = seal_authority_record(
            &fixture.head,
            &fixture.closure,
            DraftAuthorityRecord::moment(
                fixture.head.cursor(),
                checked_draft(&fixture, deliveries(), &proposals, true),
            ),
        )
        .unwrap_or_else(|error| panic!("conflict evidence must seal: {error:?}"));
        let AuthorityRecordBody::Moment(batch) = sealed.record().body() else {
            panic!("conflict resolution must produce a moment record");
        };
        let evidence = batch.resolution_evidence();
        assert_eq!(evidence.components().len(), 1);
        assert_eq!(evidence.components()[0].contenders().len(), 2);
        assert_eq!(evidence.components()[0].resources().len(), 1);
        let resource = &evidence.components()[0].resources()[0];
        assert_eq!(resource.group().moment(), due);
        assert_eq!(resource.admission_limit(), 1);
        assert_eq!(resource.ranking().entries().len(), 2);
        assert!(resource.ranking().entries().iter().all(|entry| {
            entry.key().id() == entry.key_id() && entry.key().group() == resource.group()
        }));

        let wrong_moment = moment(80, 0);
        assert_eq!(
            seal_authority_record(
                &fixture.head,
                &fixture.closure,
                DraftAuthorityRecord::moment(
                    fixture.head.cursor(),
                    checked_draft_resolved_at(
                        &fixture,
                        deliveries(),
                        &proposals,
                        false,
                        Some(wrong_moment),
                    ),
                ),
            ),
            Err(AuthorityRecordSealError::ResolutionEvidenceMismatch)
        );
    }
}
