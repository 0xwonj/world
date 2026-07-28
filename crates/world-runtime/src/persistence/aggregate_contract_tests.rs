use core::fmt::Debug;
use std::sync::Arc;

use world_core::{ActorId, EntityId, Microstep, SimMoment, SimTime};
use world_model::{
    AcceptedState, AgencyState, CommandAttemptOutcome, ContainerAuthorityRecord, ContainerRecord,
    ContainmentRecord, ContainmentTransferDelta, DomainState, EpistemicState, SocialState,
    StableCommandRejection,
};

use crate::attempt::{
    AttemptDisposition, AttemptKey, AttemptPhase, CancelAttemptRequest, CancelAttemptRequestId,
    CancelReason, CancellationLookup, RunFinalizationCause,
};
use crate::authority::{
    AuthorityAdmissionRecord, AuthorityRecordBody, DeliveryResolutionRecord, DraftAuthorityRecord,
    DraftMomentBatch, seal_authority_record,
};
use crate::execution::{
    CanonicalExecutionSpecV1, ExecutionConfigArtifactV3, ExecutionSemanticsManifestV1,
    ExternalInputBindingV1, InitialStateRootV1, ResolvedExecutionClosureManifestV1, RootSeed,
    SemanticImplementationBinding, SemanticImplementationId, TerminationContractV1,
};
use crate::kernel::fixtures;
use crate::kernel::{
    AdmitRequest, CommandProposal, ContainmentCandidate, ContainmentCandidateProposal,
    ContainmentCandidateSet, ContainmentCommandIdentity, FirePreparation, FireRequest, InputId,
    KernelSafetyCause, KernelSafetyDisposition, LedgerRetirement, ManageRequest,
    ManagementRequestId, MomentWorkDecision, MomentWorkInput, MomentWorkProposals,
    PostCommitRoutingDecision, PreparedCommandResolution, PreparedDelivery, PreparedFire,
    PreparedFireFailure, PreparedKernelSafety, SessionManagement, WorkProposal,
    resolve_containment_candidates,
};
use crate::randomness::Blake3KeyedPrf256V1;
use crate::scheduler::ScheduledWork;
use crate::service::{RuntimeControlError, RuntimeDriveError, RuntimeService};
use crate::session::SessionMode;

use super::{MemoryRepository, append_and_publish, reconcile};

fn must<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("contract fixture must be valid: {error:?}"),
    }
}

fn ready(result: Result<FirePreparation, RuntimeDriveError>) -> PreparedFire {
    match must(result) {
        FirePreparation::Ready(prepared) => prepared,
        FirePreparation::KernelSafety(_) => {
            panic!("ordinary contract fixture must pass deterministic safety preflight")
        }
    }
}

fn safety(result: Result<FirePreparation, RuntimeDriveError>) -> PreparedKernelSafety {
    match must(result) {
        FirePreparation::KernelSafety(prepared) => prepared,
        FirePreparation::Ready(_) => {
            panic!("safety contract fixture must exceed its deterministic preflight bound")
        }
    }
}

fn proposals(prepared: &PreparedFire, command_proposal: CommandProposal) -> MomentWorkProposals {
    let decisions = prepared
        .work()
        .map(|input| match input {
            MomentWorkInput::EvaluateCommand { work, .. } => {
                MomentWorkDecision::command(work, command_proposal)
            }
            input @ MomentWorkInput::PostCommitDispatch { .. } => {
                must(MomentWorkDecision::route_post_commit(
                    input,
                    PostCommitRoutingDecision::DeliverEvidence(Vec::new()),
                ))
            }
            MomentWorkInput::EvidenceAssimilation { .. }
            | MomentWorkInput::Appraisal { .. }
            | MomentWorkInput::IntentReview { .. }
            | MomentWorkInput::ActivityInitialization { .. }
            | MomentWorkInput::ActionReady { .. }
            | MomentWorkInput::ActionEvaluationResultReady { .. }
            | MomentWorkInput::AttemptResolved { .. }
            | MomentWorkInput::ActivityAdvance { .. }
            | MomentWorkInput::RelocationProcessWake { .. } => {
                unreachable!("command-only contract fixtures contain no action lifecycle work")
            }
        })
        .collect();
    must(MomentWorkProposals::from_decisions(prepared, decisions))
}

fn reject(prepared: &PreparedFire, reason: StableCommandRejection) -> MomentWorkProposals {
    proposals(prepared, CommandProposal::Rejected(reason))
}

fn accept(prepared: &PreparedFire, delta: ContainmentTransferDelta) -> MomentWorkProposals {
    proposals(prepared, CommandProposal::AcceptedTransfer(delta))
}

fn resolved_moment(
    prepared: &PreparedFire,
    proposals: &MomentWorkProposals,
    closure: &ResolvedExecutionClosureManifestV1,
) -> crate::kernel::ContainmentMomentResolution {
    let candidates = prepared
        .work()
        .filter_map(|input| match input {
            MomentWorkInput::EvaluateCommand { work, command, .. } => {
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
                        panic!("command work must have one correlated command proposal")
                    }
                };
                Some(ContainmentCandidate::new(
                    ContainmentCommandIdentity::from_command(command),
                    command.actor(),
                    proposal,
                ))
            }
            MomentWorkInput::PostCommitDispatch { .. } => None,
            MomentWorkInput::ActionReady { .. }
            | MomentWorkInput::ActionEvaluationResultReady { .. }
            | MomentWorkInput::EvidenceAssimilation { .. }
            | MomentWorkInput::Appraisal { .. }
            | MomentWorkInput::IntentReview { .. }
            | MomentWorkInput::ActivityInitialization { .. }
            | MomentWorkInput::AttemptResolved { .. }
            | MomentWorkInput::ActivityAdvance { .. }
            | MomentWorkInput::RelocationProcessWake { .. } => {
                unreachable!("command-only contract fixtures contain no action lifecycle work")
            }
        })
        .collect();
    let candidates = must(ContainmentCandidateSet::new(candidates));
    let oracle = Blake3KeyedPrf256V1::from_root_seed(closure.specification().root_seed());
    resolve_containment_candidates(
        prepared.moment(),
        prepared.base_snapshot().accepted(),
        &candidates,
        &oracle,
    )
}

fn draft_moment(
    prepared: &PreparedFire,
    proposals: &MomentWorkProposals,
    closure: &ResolvedExecutionClosureManifestV1,
) -> DraftMomentBatch {
    let resolution = resolved_moment(prepared, proposals, closure);
    must(DraftMomentBatch::from_prepared(
        prepared,
        proposals,
        &resolution,
    ))
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
        must(DomainState::new(containers, containment, authority)),
        EpistemicState::empty(),
        SocialState::empty(),
        AgencyState::empty(),
    )
}

fn empty_state() -> AcceptedState {
    accepted_state(Vec::new(), Vec::new(), Vec::new())
}

fn closure(seed_byte: u8, accepted: AcceptedState) -> ResolvedExecutionClosureManifestV1 {
    closure_with_termination(seed_byte, accepted, TerminationContractV1::never())
}

fn closure_with_termination(
    seed_byte: u8,
    accepted: AcceptedState,
    termination: TerminationContractV1,
) -> ResolvedExecutionClosureManifestV1 {
    closure_with_config(
        seed_byte,
        accepted,
        termination,
        must(ExecutionConfigArtifactV3::inline(64, 32, 16)),
    )
}

fn closure_with_config(
    seed_byte: u8,
    accepted: AcceptedState,
    termination: TerminationContractV1,
    config: ExecutionConfigArtifactV3,
) -> ResolvedExecutionClosureManifestV1 {
    let definitions = fixtures::command_definitions();
    let interface = match definitions.required_interfaces().first() {
        Some(interface) => interface.clone(),
        None => panic!("command fixture must require one semantic interface"),
    };
    let semantics = must(ExecutionSemanticsManifestV1::new(
        definitions,
        crate::execution::fixture_lifecycle_profiles(),
        config,
        vec![SemanticImplementationBinding::new(
            interface,
            SemanticImplementationId::from_bytes([seed_byte.wrapping_add(1); 32]),
        )],
    ));
    let root = must(InitialStateRootV1::origin(
        SessionMode::Running,
        SimMoment::ORIGIN,
        SimMoment::ORIGIN,
        accepted,
        Vec::new(),
    ));
    let specification = CanonicalExecutionSpecV1::new(
        &root,
        &semantics,
        RootSeed::from_bytes([seed_byte; 32]),
        termination,
        ExternalInputBindingV1::HostSerialized,
    );
    must(ResolvedExecutionClosureManifestV1::bind(
        root,
        specification,
        semantics,
    ))
}

#[test]
fn root_and_published_moment_termination_select_exact_cursors() {
    let repository = must(MemoryRepository::new());
    let root_closure = closure_with_termination(
        0x18,
        empty_state(),
        TerminationContractV1::at_or_after_moment(SimMoment::ORIGIN),
    );
    let root_attempt =
        must(repository.create_or_open(root_closure.clone(), AttemptKey::from_bytes([0x28; 32])))
            .attempt();
    {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&root_attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        assert!(matches!(
            aggregate.control.phase(),
            AttemptPhase::Finalized(finalization)
                if finalization.terminal() == root_closure.root_cursor()
                    && matches!(
                        finalization.cause(),
                        RunFinalizationCause::ReachedConfiguredMoment { .. }
                    )
        ));
    }

    let due = moment(10, 0);
    let running_closure = closure_with_termination(
        0x19,
        empty_state(),
        TerminationContractV1::at_or_after_moment(due),
    );
    let running_attempt =
        must(repository.create_or_open(running_closure, AttemptKey::from_bytes([0x29; 32])))
            .attempt();
    must(repository.admit(
        running_attempt,
        AdmitRequest::new(InputId::new(1), due, fixtures::command(0x39, 1)),
    ));
    let prepared = ready(repository.prepare_fire(running_attempt, FireRequest::through(due)));
    let proposals = reject(&prepared, StableCommandRejection::RequirementUnsatisfied);
    let fired = must(repository.complete_fire(running_attempt, prepared, proposals));

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&running_attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Finalized(finalization)
            if finalization.terminal() == fired.cursor()
                && matches!(
                    finalization.cause(),
                    RunFinalizationCause::ReachedConfiguredMoment { .. }
                )
    ));
}

#[test]
fn independent_authority_domains_share_semantic_history_and_trajectory() {
    let first_service = must(RuntimeService::in_memory());
    let second_service = must(RuntimeService::in_memory());
    let closure = closure(0x1a, empty_state());
    let key = AttemptKey::from_bytes([0x2a; 32]);
    let mut first = must(first_service.start_attempt_with_closure(closure.clone(), key));
    let mut second = must(second_service.start_attempt_with_closure(closure, key));
    assert_ne!(first.attempt_id(), second.attempt_id());

    let due = moment(11, 0);
    let request = AdmitRequest::new(InputId::new(1), due, fixtures::command(0x3a, 1));
    let first_admission = must(first.admit(request.clone()));
    let second_admission = must(second.admit(request));
    assert_eq!(first_admission, second_admission);
    assert_eq!(
        must(first.session_reader().cursor()),
        must(second.session_reader().cursor())
    );

    let first_prepared = ready(first.prepare_fire(FireRequest::through(due)));
    let second_prepared = ready(second.prepare_fire(FireRequest::through(due)));
    let first_proposals = reject(
        &first_prepared,
        StableCommandRejection::RequirementUnsatisfied,
    );
    let second_proposals = reject(
        &second_prepared,
        StableCommandRejection::RequirementUnsatisfied,
    );
    let first_fire = must(first.complete_fire(first_prepared, first_proposals));
    let second_fire = must(second.complete_fire(second_prepared, second_proposals));
    assert_eq!(first_fire, second_fire);

    let cancellation =
        CancelAttemptRequest::new(CancelAttemptRequestId::new(1), CancelReason::HostRequested);
    let first_finalization = must(first.cancel_attempt(cancellation)).finalization();
    let second_finalization = must(second.cancel_attempt(cancellation)).finalization();
    assert_ne!(first_finalization.attempt(), second_finalization.attempt());
    assert_eq!(
        first_finalization.terminal(),
        second_finalization.terminal()
    );
    assert_eq!(
        first_finalization.trajectory(),
        second_finalization.trajectory()
    );
}

#[test]
fn competing_admissions_publish_two_atomic_entries_at_one_moment() {
    let repository = Arc::new(must(MemoryRepository::new()));
    let attempt = must(repository.create_or_open(
        closure(0x1b, empty_state()),
        AttemptKey::from_bytes([0x2b; 32]),
    ))
    .attempt();
    let initial_cursor = must(repository.cursor(attempt));
    let due = moment(12, 0);
    let first_request = AdmitRequest::new(InputId::new(1), due, fixtures::command(0x3b, 1));
    let second_request = AdmitRequest::new(InputId::new(2), due, fixtures::command(0x3c, 2));

    let first_repository = Arc::clone(&repository);
    let first_work = first_request.clone();
    let first_worker = std::thread::spawn(move || {
        first_repository
            .admit(attempt, first_work)
            .map_err(Box::new)
    });
    let second_repository = Arc::clone(&repository);
    let second_work = second_request.clone();
    let second_worker = std::thread::spawn(move || {
        second_repository
            .admit(attempt, second_work)
            .map_err(Box::new)
    });
    let first_result = match first_worker.join() {
        Ok(result) => result,
        Err(_) => panic!("first admission worker must not panic"),
    };
    let second_result = match second_worker.join() {
        Ok(result) => result,
        Err(_) => panic!("second admission worker must not panic"),
    };
    let first_admission = must(first_result);
    let second_admission = must(second_result);

    let published_cursor = must(repository.cursor(attempt));
    assert_ne!(published_cursor, initial_cursor);
    assert_eq!(published_cursor.revision().get(), 2);

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.history.len(), 2);
    assert_eq!(aggregate.receipts.len(), 2);
    assert_eq!(aggregate.history[0].header().revision().get(), 1);
    assert_eq!(aggregate.history[1].header().revision().get(), 2);
    assert_eq!(aggregate.history[0].header().sequence().get(), 1);
    assert_eq!(aggregate.history[1].header().sequence().get(), 2);
    assert_eq!(
        aggregate.history[1].header().cumulative(),
        published_cursor.cumulative()
    );

    let mut captured = aggregate
        .history
        .iter()
        .map(|record| match record.body() {
            AuthorityRecordBody::Admission(AuthorityAdmissionRecord::Commands(batch)) => {
                match batch.entries() {
                    [entry] => (
                        record.header().id(),
                        entry.captured().input(),
                        entry.captured().request_fingerprint(),
                        entry.captured().command().clone(),
                    ),
                    _ => panic!("each singular Admit must publish one ingress entry"),
                }
            }
            AuthorityRecordBody::Admission(AuthorityAdmissionRecord::ActionEvaluation(_))
            | AuthorityRecordBody::Moment(_)
            | AuthorityRecordBody::Management(_) => {
                panic!("both concurrent operations must remain ingress records")
            }
        })
        .collect::<Vec<_>>();
    captured.sort_by_key(|entry| entry.1);
    assert_eq!(
        captured,
        vec![
            (
                first_admission.record(),
                first_request.id(),
                first_request.fingerprint(),
                first_request.command().clone(),
            ),
            (
                second_admission.record(),
                second_request.id(),
                second_request.fingerprint(),
                second_request.command().clone(),
            ),
        ]
    );

    let mut receipt_records = aggregate
        .receipts
        .values()
        .map(|receipt| receipt.record())
        .collect::<Vec<_>>();
    receipt_records.sort_unstable();
    let mut admission_records = vec![first_admission.record(), second_admission.record()];
    admission_records.sort_unstable();
    assert_eq!(receipt_records, admission_records);

    let input_entries: Vec<_> = aggregate.head.runtime_control().input().iter().collect();
    assert_eq!(input_entries.len(), 2);
    assert_eq!(input_entries[0].0, first_request.id());
    assert_eq!(input_entries[1].0, second_request.id());

    let due_set = match aggregate.head.scheduler().clone_least_due() {
        Some(due_set) => due_set,
        None => panic!("both admitted commands must remain scheduled"),
    };
    assert_eq!(due_set.moment(), due);
    let mut scheduled_inputs = due_set
        .entries()
        .iter()
        .map(|(_, work)| match work {
            ScheduledWork::Command(command) => command.input(),
            ScheduledWork::PostCommit(_) => {
                panic!("concurrent admissions must schedule commands")
            }
            ScheduledWork::ActionReady(_)
            | ScheduledWork::ActionEvaluation(_)
            | ScheduledWork::Lifecycle(_)
            | ScheduledWork::Process(_) => {
                panic!("concurrent admissions must schedule commands")
            }
        })
        .collect::<Vec<_>>();
    scheduled_inputs.sort_unstable();
    assert_eq!(
        scheduled_inputs,
        vec![Some(first_request.id()), Some(second_request.id())]
    );
    assert_eq!(aggregate.head.cursor(), published_cursor);
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Active(cursor) if *cursor == published_cursor
    ));
}

#[test]
fn whole_moment_groups_exact_duplicates_and_publishes_independent_attempts_atomically() {
    let (repository, attempt) = repository_with_attempt(0x1f, 0x2f, empty_state());
    let due = moment(12, 1);
    let duplicate = fixtures::command(0x4b, 1);
    let independent = fixtures::command(0x4c, 2);

    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), due, duplicate.clone()),
    ));
    must(repository.admit(attempt, AdmitRequest::new(InputId::new(2), due, duplicate)));
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(3), due, independent),
    ));

    let prepared = ready(repository.prepare_fire(attempt, FireRequest::through(due)));
    assert_eq!(prepared.deliveries().len(), 3);
    assert_eq!(
        prepared.work().len(),
        2,
        "exact copies share one evaluator decision while independent commands do not"
    );
    assert!(prepared.work().all(|input| matches!(
        input,
        MomentWorkInput::EvaluateCommand {
            due: selected,
            snapshot,
            ..
        } if selected == due && snapshot == prepared.base_snapshot()
    )));

    let proposals = reject(&prepared, StableCommandRejection::Conflict);
    let outcome = must(repository.complete_fire(attempt, prepared, proposals));
    assert_eq!(outcome.command_resolutions().len(), 3);
    assert!(outcome.command_resolutions().iter().all(|resolution| {
        resolution.classification()
            == crate::kernel::CommandFireClassification::New(CommandAttemptOutcome::Rejected(
                StableCommandRejection::Conflict,
            ))
    }));

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    let batch = match aggregate.history.last().map(|record| record.body()) {
        Some(AuthorityRecordBody::Moment(batch)) => batch,
        _ => panic!("the complete due set must publish as one moment batch"),
    };
    assert_eq!(batch.moment(), due);
    assert_eq!(batch.consumed_keys().len(), 3);
    assert_eq!(batch.command_deliveries().len(), 3);
    assert_eq!(batch.attempts().len(), 2);
    assert_eq!(batch.resolutions().len(), 3);
    assert!(batch.attempts().iter().all(|attempt| {
        attempt.resolution().outcome()
            == CommandAttemptOutcome::Rejected(StableCommandRejection::Conflict)
    }));

    let mut referenced_attempts = batch
        .resolutions()
        .iter()
        .map(|resolution| match resolution {
            DeliveryResolutionRecord::NewCommand { attempt, .. } => *attempt,
            _ => panic!("each absent command delivery must resolve through a new attempt"),
        })
        .collect::<Vec<_>>();
    referenced_attempts.sort_unstable();
    referenced_attempts.dedup();
    assert_eq!(referenced_attempts.len(), 2);
    assert!(aggregate.head.scheduler().is_empty());
    assert_eq!(aggregate.history.len(), 4);
    assert_eq!(aggregate.receipts.len(), 4);
}

#[test]
fn management_retry_and_reuse_classification_precede_current_phase() {
    let (repository, attempt) = repository_with_attempt(0x1c, 0x2c, empty_state());
    let pause = ManageRequest::new(ManagementRequestId::new(4), SessionManagement::Pause);
    let first = must(repository.manage(attempt, pause));
    assert_eq!(must(repository.manage(attempt, pause)), first);
    assert_eq!(
        repository.manage(
            attempt,
            ManageRequest::new(ManagementRequestId::new(4), SessionManagement::Resume),
        ),
        Err(RuntimeDriveError::ManagementIdReuse)
    );

    must(repository.cancel_attempt(
        attempt,
        CancelAttemptRequest::new(CancelAttemptRequestId::new(5), CancelReason::HostRequested),
    ));
    assert_eq!(must(repository.manage(attempt, pause)), first);
}

fn assert_host_terminal_management(
    seed_byte: u8,
    key_byte: u8,
    operation: SessionManagement,
    expected_mode: SessionMode,
    pause_first: bool,
) {
    let actor = ActorId::from_bytes([0x41; 32]);
    let container = EntityId::from_bytes([0x51; 32]);
    let accepted = accepted_state(
        vec![ContainerRecord::new(container, 2)],
        Vec::new(),
        vec![ContainerAuthorityRecord::new(actor, container)],
    );
    let (repository, attempt) = repository_with_attempt(seed_byte, key_byte, accepted);
    let due = moment(17, 0);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(0), due, fixtures::command(0x61, 0)),
    ));
    must(repository.manage(
        attempt,
        ManageRequest::new(
            ManagementRequestId::new(0),
            SessionManagement::SealAdmissionThrough(due),
        ),
    ));

    let terminal_id = if pause_first {
        must(repository.manage(
            attempt,
            ManageRequest::new(ManagementRequestId::new(1), SessionManagement::Pause),
        ));
        ManagementRequestId::new(2)
    } else {
        ManagementRequestId::new(1)
    };

    let (before_cursor, before_frontier, before_accepted, before_due) = {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        assert_eq!(
            aggregate.head.mode(),
            if pause_first {
                SessionMode::Paused
            } else {
                SessionMode::Running
            }
        );
        (
            aggregate.head.cursor(),
            aggregate.head.clock().frontier(),
            aggregate.head.accepted().clone(),
            aggregate.head.scheduler().clone_least_due(),
        )
    };

    let request = ManageRequest::new(terminal_id, operation);
    let outcome = must(repository.manage(attempt, request));
    assert_eq!(outcome.resulting_mode(), Some(expected_mode));
    assert_ne!(must(repository.cursor(attempt)), before_cursor);

    let terminal_cursor = must(repository.cursor(attempt));
    assert_eq!(must(repository.manage(attempt, request)), outcome);
    assert_eq!(must(repository.cursor(attempt)), terminal_cursor);

    let mismatched_operation = match operation {
        SessionManagement::Quarantine => SessionManagement::Fail,
        SessionManagement::Fail => SessionManagement::Quarantine,
        _ => panic!("terminal management fixture requires quarantine or failure"),
    };
    assert_eq!(
        repository.manage(
            attempt,
            ManageRequest::new(terminal_id, mismatched_operation),
        ),
        Err(RuntimeDriveError::ManagementIdReuse)
    );
    assert_eq!(
        repository.manage(
            attempt,
            ManageRequest::new(
                ManagementRequestId::new(terminal_id.get() + 1),
                SessionManagement::Resume,
            ),
        ),
        Err(RuntimeDriveError::IllegalManagement {
            current: expected_mode,
        })
    );

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.head.mode(), expected_mode);
    assert_eq!(aggregate.head.clock().frontier(), before_frontier);
    assert_eq!(aggregate.head.accepted(), &before_accepted);
    assert_eq!(aggregate.head.scheduler().clone_least_due(), before_due);
    assert_eq!(aggregate.head.safety_blocker(), None);
    let batch = match aggregate.history.last().map(|record| record.body()) {
        Some(AuthorityRecordBody::Management(batch)) => batch,
        _ => panic!("host terminal transition must publish a management record"),
    };
    assert_eq!(batch.kernel_safety_cause(), None);
    assert_eq!(batch.resulting_mode(), expected_mode);
    assert_eq!(batch.preserved_frontier(), before_frontier);
    assert!(matches!(
        batch.entries(),
        [entry] if entry.operation() == operation && entry.outcome() == outcome
    ));
}

#[test]
fn host_quarantine_from_running_is_authoritative_and_non_resumable() {
    assert_host_terminal_management(
        0x1d,
        0x2d,
        SessionManagement::Quarantine,
        SessionMode::Quarantined,
        false,
    );
}

#[test]
fn host_failure_from_paused_is_authoritative_and_non_resumable() {
    assert_host_terminal_management(
        0x1e,
        0x2e,
        SessionManagement::Fail,
        SessionMode::Failed,
        true,
    );
}

#[test]
fn runtime_service_exposes_retirement_and_admission_sealing_through_manage() {
    let service = must(RuntimeService::in_memory());
    let mut driver = must(service.start_attempt_with_closure(
        closure(0x40, empty_state()),
        AttemptKey::from_bytes([0x50; 32]),
    ));
    let due = moment(9, 0);
    must(driver.admit(AdmitRequest::new(
        InputId::new(0),
        due,
        fixtures::command(0x60, 0),
    )));

    let retirement = LedgerRetirement::InputThrough(InputId::new(0));
    let retire = ManageRequest::new(
        ManagementRequestId::new(0),
        SessionManagement::Retire(retirement),
    );
    let retired = must(driver.manage(retire));
    assert_eq!(retired.retirement(), Some(retirement));
    assert_eq!(must(driver.manage(retire)), retired);

    let seal = ManageRequest::new(
        ManagementRequestId::new(1),
        SessionManagement::SealAdmissionThrough(due),
    );
    let sealed = must(driver.manage(seal));
    assert_eq!(sealed.admission_frontier(), Some(due));
    assert_eq!(must(driver.manage(seal)), sealed);
    assert_eq!(
        must(driver.session_reader().read()).admission_frontier(),
        due
    );
}

#[test]
fn input_retirement_is_a_recorded_delta_and_preserves_scheduled_work() {
    let (repository, attempt) = repository_with_attempt(0x41, 0x51, empty_state());
    let due = moment(10, 0);
    let input = AdmitRequest::new(InputId::new(0), due, fixtures::command(0x61, 0));
    must(repository.admit(attempt, input.clone()));

    let gap = LedgerRetirement::InputThrough(InputId::new(1));
    let before_gap = must(repository.cursor(attempt));
    assert_eq!(
        repository.manage(
            attempt,
            ManageRequest::new(ManagementRequestId::new(0), SessionManagement::Retire(gap)),
        ),
        Err(RuntimeDriveError::RetirementGap {
            retirement: gap,
            missing: 1,
        })
    );
    assert_eq!(must(repository.cursor(attempt)), before_gap);

    let retirement = LedgerRetirement::InputThrough(InputId::new(0));
    let request = ManageRequest::new(
        ManagementRequestId::new(0),
        SessionManagement::Retire(retirement),
    );
    let outcome = must(repository.manage(attempt, request));
    assert_eq!(outcome.retirement(), Some(retirement));
    assert_eq!(outcome.resulting_mode(), None);
    let retired_cursor = must(repository.cursor(attempt));
    assert_ne!(retired_cursor, before_gap);
    assert_eq!(must(repository.manage(attempt, request)), outcome);
    assert_eq!(must(repository.cursor(attempt)), retired_cursor);
    assert_eq!(
        repository.admit(attempt, input),
        Err(RuntimeDriveError::InputRetired {
            id: InputId::new(0),
        })
    );

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(
        aggregate.head.scheduler().least_due_moment(),
        Some(due),
        "retiring the request result must not remove its scheduled command"
    );
    assert_eq!(
        aggregate.head.runtime_control().input().retired_through(),
        Some(InputId::new(0))
    );
    let batch = match aggregate.history.last().map(|record| record.body()) {
        Some(AuthorityRecordBody::Management(batch)) => batch,
        _ => panic!("retirement must publish an authority management record"),
    };
    assert!(matches!(
        batch.entries(),
        [entry] if entry.operation() == SessionManagement::Retire(retirement)
            && entry.outcome() == outcome
    ));
}

#[test]
fn management_retirement_keeps_one_exactly_retriable_tail_request() {
    let (repository, attempt) = repository_with_attempt(0x42, 0x52, empty_state());
    let pause = ManageRequest::new(ManagementRequestId::new(0), SessionManagement::Pause);
    let resume = ManageRequest::new(ManagementRequestId::new(1), SessionManagement::Resume);
    must(repository.manage(attempt, pause));
    must(repository.manage(attempt, resume));

    let invalid = ManageRequest::new(
        ManagementRequestId::new(2),
        SessionManagement::Retire(LedgerRetirement::ManagementThrough(
            ManagementRequestId::new(2),
        )),
    );
    let before_invalid = must(repository.cursor(attempt));
    assert_eq!(
        repository.manage(attempt, invalid),
        Err(
            RuntimeDriveError::ManagementRetirementTargetNotBeforeRequest {
                target: ManagementRequestId::new(2),
                request: ManagementRequestId::new(2),
            }
        )
    );
    assert_eq!(must(repository.cursor(attempt)), before_invalid);

    let first_retirement = LedgerRetirement::ManagementThrough(ManagementRequestId::new(1));
    let first_carrier = ManageRequest::new(
        ManagementRequestId::new(2),
        SessionManagement::Retire(first_retirement),
    );
    let first_outcome = must(repository.manage(attempt, first_carrier));
    let first_cursor = must(repository.cursor(attempt));
    assert_eq!(first_outcome.retirement(), Some(first_retirement));
    assert_eq!(
        must(repository.manage(attempt, first_carrier)),
        first_outcome
    );
    assert_eq!(must(repository.cursor(attempt)), first_cursor);
    assert_eq!(
        repository.manage(attempt, pause),
        Err(RuntimeDriveError::ManagementRetired {
            id: ManagementRequestId::new(0),
        })
    );
    assert_eq!(
        repository.manage(
            attempt,
            ManageRequest::new(ManagementRequestId::new(2), SessionManagement::Resume),
        ),
        Err(RuntimeDriveError::ManagementIdReuse)
    );

    let second_retirement = LedgerRetirement::ManagementThrough(ManagementRequestId::new(2));
    let second_carrier = ManageRequest::new(
        ManagementRequestId::new(3),
        SessionManagement::Retire(second_retirement),
    );
    let second_outcome = must(repository.manage(attempt, second_carrier));
    assert_eq!(
        repository.manage(attempt, first_carrier),
        Err(RuntimeDriveError::ManagementRetired {
            id: ManagementRequestId::new(2),
        })
    );
    assert_eq!(
        must(repository.manage(attempt, second_carrier)),
        second_outcome
    );

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(
        aggregate
            .head
            .runtime_control()
            .management()
            .retired_through(),
        Some(ManagementRequestId::new(2))
    );
    assert_eq!(
        aggregate
            .head
            .runtime_control()
            .management()
            .iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        vec![ManagementRequestId::new(3)]
    );
}

#[test]
fn command_retirement_advances_only_the_selected_source_namespace() {
    let (repository, attempt) = repository_with_attempt(0x43, 0x53, empty_state());
    let due = moment(11, 0);
    let first = fixtures::command(0x63, 0);
    let second = fixtures::command(0x64, 0);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(0), due, first.clone()),
    ));
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), due, second.clone()),
    ));
    let prepared = ready(repository.prepare_fire(attempt, FireRequest::through(due)));
    let proposals = reject(&prepared, StableCommandRejection::Stale);
    must(repository.complete_fire(attempt, prepared, proposals));

    let retirement = LedgerRetirement::CommandThrough {
        source: first.source(),
        command: first.id(),
    };
    let outcome = must(repository.manage(
        attempt,
        ManageRequest::new(
            ManagementRequestId::new(0),
            SessionManagement::Retire(retirement),
        ),
    ));
    assert_eq!(outcome.retirement(), Some(retirement));

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert!(matches!(
        aggregate.head.runtime_control().command().classify(
            first.source(),
            first.id(),
            first.fingerprint(),
        ),
        crate::control::CommandLedgerLookup::Retired
    ));
    assert!(matches!(
        aggregate.head.runtime_control().command().classify(
            second.source(),
            second.id(),
            second.fingerprint(),
        ),
        crate::control::CommandLedgerLookup::RetainedExact { .. }
    ));
    assert_eq!(
        aggregate
            .head
            .runtime_control()
            .command()
            .retired_through(first.source()),
        Some(first.id())
    );
    assert_eq!(
        aggregate
            .head
            .runtime_control()
            .command()
            .retired_through(second.source()),
        None
    );
}

#[test]
fn admission_sealing_is_retriable_and_cannot_skip_due_work() {
    let (repository, attempt) = repository_with_attempt(0x44, 0x54, empty_state());
    let due = moment(12, 0);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(0), due, fixtures::command(0x65, 0)),
    ));

    let skipped = moment(12, 1);
    let before_rejection = must(repository.cursor(attempt));
    assert_eq!(
        repository.manage(
            attempt,
            ManageRequest::new(
                ManagementRequestId::new(0),
                SessionManagement::SealAdmissionThrough(skipped),
            ),
        ),
        Err(RuntimeDriveError::AdmissionSealCrossesScheduledWork {
            requested: skipped,
            scheduled: due,
        })
    );
    assert_eq!(must(repository.cursor(attempt)), before_rejection);

    let request = ManageRequest::new(
        ManagementRequestId::new(0),
        SessionManagement::SealAdmissionThrough(due),
    );
    let outcome = must(repository.manage(attempt, request));
    assert_eq!(outcome.admission_frontier(), Some(due));
    assert_eq!(outcome.resulting_mode(), None);
    let sealed_cursor = must(repository.cursor(attempt));
    assert_eq!(must(repository.manage(attempt, request)), outcome);
    assert_eq!(must(repository.cursor(attempt)), sealed_cursor);
    assert_eq!(must(repository.read(attempt)).admission_frontier(), due);
    assert_eq!(
        repository.manage(
            attempt,
            ManageRequest::new(
                ManagementRequestId::new(0),
                SessionManagement::SealAdmissionThrough(skipped),
            ),
        ),
        Err(RuntimeDriveError::ManagementIdReuse)
    );
    assert_eq!(
        repository.manage(
            attempt,
            ManageRequest::new(
                ManagementRequestId::new(1),
                SessionManagement::SealAdmissionThrough(due),
            ),
        ),
        Err(RuntimeDriveError::AdmissionFrontierNotAdvancing {
            current: due,
            requested: due,
        })
    );

    let before_due = moment(11, 9);
    assert_eq!(
        repository.admit(
            attempt,
            AdmitRequest::new(InputId::new(1), before_due, fixtures::command(0x66, 1)),
        ),
        Err(RuntimeDriveError::EffectiveMomentBeforeFrontier {
            effective: before_due,
            frontier: due,
        })
    );
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), due, fixtures::command(0x66, 1)),
    ));
}

#[test]
fn retained_and_mismatched_commands_are_consumed_without_reevaluation() {
    let (repository, attempt) = repository_with_attempt(0x1d, 0x2d, empty_state());
    let first_due = moment(13, 0);
    let retained_due = moment(14, 0);
    let mismatch_due = moment(15, 0);
    let original = fixtures::command_with_actor(0x3d, 1, 0x41);
    let changed = fixtures::command_with_actor(0x3d, 1, 0x42);

    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), first_due, original.clone()),
    ));
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(2), retained_due, original),
    ));
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(3), mismatch_due, changed),
    ));

    let first = ready(repository.prepare_fire(attempt, FireRequest::through(first_due)));
    let first_proposals = reject(&first, StableCommandRejection::Stale);
    must(repository.complete_fire(attempt, first, first_proposals));

    let retained = ready(repository.prepare_fire(attempt, FireRequest::through(retained_due)));
    assert_eq!(retained.work().count(), 0);
    let (original_attempt, original_outcome) = match retained.deliveries() {
        [
            PreparedDelivery::ResolvedCommand {
                resolution:
                    PreparedCommandResolution::Retained {
                        original_attempt,
                        outcome,
                    },
                ..
            },
        ] => (*original_attempt, *outcome),
        _ => panic!("an exact repeated command must remain a private retained delivery"),
    };
    assert_eq!(
        original_outcome,
        CommandAttemptOutcome::Rejected(StableCommandRejection::Stale)
    );
    let retained_proposals = reject(&retained, StableCommandRejection::Conflict);
    let retained_outcome = must(repository.complete_fire(attempt, retained, retained_proposals));
    assert!(matches!(
        retained_outcome.command_resolutions(),
        [resolution]
            if resolution.classification()
                == crate::kernel::CommandFireClassification::Retained(original_outcome)
    ));

    let mismatch = ready(repository.prepare_fire(attempt, FireRequest::through(mismatch_due)));
    assert_eq!(mismatch.work().count(), 0);
    let mismatch_original = match mismatch.deliveries() {
        [
            PreparedDelivery::ResolvedCommand {
                resolution: PreparedCommandResolution::IdReuseMismatch { original_attempt },
                ..
            },
        ] => *original_attempt,
        _ => panic!("changed content under one command ID must remain a private mismatch"),
    };
    assert_eq!(mismatch_original, original_attempt);
    let mismatch_proposals = reject(&mismatch, StableCommandRejection::Conflict);
    let mismatch_outcome = must(repository.complete_fire(attempt, mismatch, mismatch_proposals));
    assert!(matches!(
        mismatch_outcome.command_resolutions(),
        [resolution]
            if resolution.classification()
                == crate::kernel::CommandFireClassification::IdReuseMismatch
    ));

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    let moment_resolutions: Vec<&DeliveryResolutionRecord> = aggregate
        .history
        .iter()
        .filter_map(|record| match record.body() {
            AuthorityRecordBody::Moment(batch) => batch.resolutions().first(),
            AuthorityRecordBody::Admission(_) | AuthorityRecordBody::Management(_) => None,
        })
        .collect();
    assert!(matches!(
        moment_resolutions.as_slice(),
        [
            DeliveryResolutionRecord::NewCommand { .. },
            DeliveryResolutionRecord::RetainedCommand {
                original_attempt: retained_attempt,
                original_outcome: CommandAttemptOutcome::Rejected(
                    StableCommandRejection::Stale
                ),
                ..
            },
            DeliveryResolutionRecord::CommandIdReuseMismatch {
                original_attempt: mismatch_attempt,
                ..
            }
        ] if *retained_attempt == original_attempt && *mismatch_attempt == original_attempt
    ));
}

#[test]
fn same_barrier_command_collision_is_resolved_once_without_evaluator_work() {
    let (repository, attempt) = repository_with_attempt(0x1e, 0x2e, empty_state());
    let due = moment(16, 0);
    let first = fixtures::command_with_actor(0x4d, 1, 0x41);
    let second = fixtures::command_with_actor(0x4d, 1, 0x42);
    assert_ne!(first.fingerprint(), second.fingerprint());

    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), due, first.clone()),
    ));
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(2), due, second.clone()),
    ));

    let prepared = ready(repository.prepare_fire(attempt, FireRequest::through(due)));
    assert_eq!(prepared.deliveries().len(), 2);
    assert_eq!(prepared.work().count(), 0);
    assert!(prepared.deliveries().iter().all(|delivery| matches!(
        delivery,
        PreparedDelivery::ResolvedCommand {
            resolution: PreparedCommandResolution::NewCollision,
            ..
        }
    )));

    let proposals = reject(&prepared, StableCommandRejection::Conflict);
    let outcome = must(repository.complete_fire(attempt, prepared, proposals));
    assert_eq!(outcome.command_resolutions().len(), 2);
    assert!(outcome.command_resolutions().iter().all(|resolution| {
        resolution.source() == first.source()
            && resolution.command() == first.id()
            && resolution.classification() == crate::kernel::CommandFireClassification::IdCollision
    }));

    let original_attempt = {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        let batch = match aggregate.history.last().map(|record| record.body()) {
            Some(AuthorityRecordBody::Moment(batch)) => batch,
            _ => panic!("collision resolution must publish one moment batch"),
        };
        assert!(matches!(
            batch.attempts(),
            [attempt]
                if attempt.resolution().outcome()
                    == CommandAttemptOutcome::Rejected(StableCommandRejection::IdCollision)
        ));
        assert_eq!(batch.resolutions().len(), 2);
        let original_attempt = batch.attempts()[0].id();
        assert!(matches!(
            aggregate.head.runtime_control().command().classify(
                first.source(),
                first.id(),
                first.fingerprint(),
            ),
            crate::control::CommandLedgerLookup::RetainedCollision {
                original_attempt: retained,
            } if retained == original_attempt
        ));
        assert!(matches!(
            aggregate.head.runtime_control().command().classify(
                second.source(),
                second.id(),
                second.fingerprint(),
            ),
            crate::control::CommandLedgerLookup::RetainedCollision {
                original_attempt: retained,
            } if retained == original_attempt
        ));
        original_attempt
    };

    let retry_due = moment(17, 0);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(3), retry_due, first),
    ));
    let retained = ready(repository.prepare_fire(attempt, FireRequest::through(retry_due)));
    assert_eq!(retained.work().count(), 0);
    assert!(matches!(
        retained.deliveries(),
        [PreparedDelivery::ResolvedCommand {
            resolution: PreparedCommandResolution::RetainedCollision {
                original_attempt: retained,
            },
            ..
        }] if *retained == original_attempt
    ));
    let proposals = reject(&retained, StableCommandRejection::Conflict);
    let outcome = must(repository.complete_fire(attempt, retained, proposals));
    assert!(matches!(
        outcome.command_resolutions(),
        [resolution]
            if resolution.classification()
                == crate::kernel::CommandFireClassification::IdCollision
    ));

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    let batch = match aggregate.history.last().map(|record| record.body()) {
        Some(AuthorityRecordBody::Moment(batch)) => batch,
        _ => panic!("retained collision consumption must publish one moment batch"),
    };
    assert!(batch.attempts().is_empty());
    assert!(matches!(
        batch.resolutions(),
        [DeliveryResolutionRecord::RetainedCollision {
            original_attempt: retained,
            ..
        }] if *retained == original_attempt
    ));
}

fn repository_with_attempt(
    seed_byte: u8,
    key_byte: u8,
    accepted: AcceptedState,
) -> (MemoryRepository, crate::attempt::RunAttemptId) {
    let repository = must(MemoryRepository::new());
    let attempt = must(repository.create_or_open(
        closure(seed_byte, accepted),
        AttemptKey::from_bytes([key_byte; 32]),
    ))
    .attempt();
    (repository, attempt)
}

#[test]
fn admission_retry_is_stable_and_identity_reuse_is_rejected() {
    let (repository, attempt) = repository_with_attempt(0x11, 0x21, empty_state());
    let initial_cursor = must(repository.cursor(attempt));
    let due = moment(2, 0);
    let request = AdmitRequest::new(InputId::new(7), due, fixtures::command(0x31, 1));

    let first = must(repository.admit(attempt, request.clone()));
    let published_cursor = must(repository.cursor(attempt));
    assert_ne!(published_cursor, initial_cursor);
    assert_eq!(first.effective(), due);

    {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        assert_eq!(aggregate.history.len(), 1);
        assert_eq!(aggregate.receipts.len(), 1);
        assert_eq!(
            aggregate.history[0].header().id(),
            first.record(),
            "the retained outcome must name the appended record"
        );
        let receipt = match aggregate.receipts.values().next() {
            Some(receipt) => *receipt,
            None => panic!("publication must retain its receipt"),
        };
        assert_eq!(receipt.expected(), initial_cursor);
        assert_eq!(receipt.resulting(), published_cursor);
        assert_eq!(receipt.record(), first.record());
        assert!(matches!(
            aggregate.control.phase(),
            AttemptPhase::Active(cursor) if *cursor == published_cursor
        ));
    }

    let retry = must(repository.admit(attempt, request));
    assert_eq!(retry, first);
    assert_eq!(must(repository.cursor(attempt)), published_cursor);

    let reused = AdmitRequest::new(InputId::new(7), due, fixtures::command(0x31, 2));
    assert_eq!(
        repository.admit(attempt, reused),
        Err(RuntimeDriveError::InputIdReuse)
    );

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.history.len(), 1);
    assert_eq!(aggregate.receipts.len(), 1);
    assert_eq!(aggregate.head.cursor(), published_cursor);
}

#[test]
fn rejected_fire_publishes_one_correlated_successor() {
    let (repository, attempt) = repository_with_attempt(0x12, 0x22, empty_state());
    let due = moment(3, 0);
    let command = fixtures::command(0x32, 1);
    let admission = must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), due, command.clone()),
    ));
    let base_cursor = must(repository.cursor(attempt));
    let base_snapshot = must(repository.snapshot(attempt));

    let prepared = ready(repository.prepare_fire(attempt, FireRequest::through(due)));
    match prepared.work().collect::<Vec<_>>().as_slice() {
        [
            MomentWorkInput::EvaluateCommand {
                due: selected,
                snapshot,
                command: selected_command,
                ..
            },
        ] => {
            assert_eq!(*selected, due);
            assert_eq!(*snapshot, &base_snapshot);
            assert_eq!(*selected_command, &command);
        }
        _ => panic!("the first delivery must expose exactly one evaluable command"),
    }
    let proposals = reject(&prepared, StableCommandRejection::RequirementUnsatisfied);
    let outcome = must(repository.complete_fire(attempt, prepared, proposals));

    assert_eq!(must(repository.cursor(attempt)), outcome.cursor());
    let snapshot = must(repository.snapshot(attempt));
    assert_eq!(snapshot.revision(), outcome.cursor().revision());
    assert_eq!(snapshot.accepted(), base_snapshot.accepted());

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.history.len(), 2);
    assert_eq!(aggregate.receipts.len(), 2);
    assert_eq!(aggregate.history[0].header().id(), admission.record());
    assert_eq!(aggregate.history[1].header().id(), outcome.record());
    assert!(aggregate.head.scheduler().is_empty());
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Active(cursor) if *cursor == outcome.cursor()
    ));
    match aggregate.history[1].body() {
        AuthorityRecordBody::Moment(batch) => match batch.attempts() {
            [attempt] => assert_eq!(
                attempt.resolution().outcome(),
                world_model::CommandAttemptOutcome::Rejected(
                    StableCommandRejection::RequirementUnsatisfied
                )
            ),
            _ => panic!("rejection must materialize the correlated rejected shape"),
        },
        _ => panic!("command delivery must append a moment record"),
    }
    let fire_receipt = match aggregate
        .receipts
        .values()
        .find(|receipt| receipt.record() == outcome.record())
    {
        Some(receipt) => receipt,
        None => panic!("fire publication must retain its receipt"),
    };
    assert_eq!(fire_receipt.expected(), base_cursor);
    assert_eq!(fire_receipt.resulting(), outcome.cursor());
}

#[test]
fn publication_moves_head_history_and_receipt_before_releasing_the_gate() {
    let (repository, attempt) = repository_with_attempt(0x13, 0x23, empty_state());
    let due = moment(4, 0);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), due, fixtures::command(0x33, 1)),
    ));
    let prepared = ready(repository.prepare_fire(attempt, FireRequest::through(due)));

    let mut state = must(repository.state.lock());
    let aggregate = match state.attempts.get_mut(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    let expected = aggregate.head.cursor();
    let history_len = aggregate.history.len();
    let receipt_len = aggregate.receipts.len();
    let step = prepared.step();

    let unrelated = must(seal_authority_record(
        &aggregate.head,
        aggregate.control.closure(),
        DraftAuthorityRecord::management(
            expected,
            vec![ManageRequest::new(
                ManagementRequestId::new(99),
                SessionManagement::Pause,
            )],
        ),
    ));
    assert_eq!(
        append_and_publish(aggregate, unrelated),
        Err(RuntimeDriveError::Integrity)
    );
    assert_eq!(aggregate.head.cursor(), expected);
    assert_eq!(aggregate.history.len(), history_len);
    assert_eq!(aggregate.receipts.len(), receipt_len);
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Reserved(reservation) if reservation.step() == step
    ));

    let proposals = reject(&prepared, StableCommandRejection::Stale);
    let draft = draft_moment(&prepared, &proposals, aggregate.control.closure());
    let sealed = must(seal_authority_record(
        &aggregate.head,
        aggregate.control.closure(),
        DraftAuthorityRecord::moment(expected, draft),
    ));
    let (record, resulting) = must(append_and_publish(aggregate, sealed));

    assert_eq!(aggregate.head.cursor(), resulting);
    assert_eq!(aggregate.history.len(), history_len + 1);
    assert_eq!(aggregate.receipts.len(), receipt_len + 1);
    assert_eq!(
        aggregate.history.last().map(|entry| entry.header().id()),
        Some(record)
    );
    let receipt = match aggregate.receipts.get(&step) {
        Some(receipt) => *receipt,
        None => panic!("published step must retain its matching receipt"),
    };
    assert_eq!(receipt.expected(), expected);
    assert_eq!(receipt.resulting(), resulting);
    assert_eq!(receipt.record(), record);
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Reserved(reservation) if reservation.step() == step
    ));

    must(reconcile(aggregate));
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Active(cursor) if *cursor == resulting
    ));
}

#[test]
fn mismatched_history_tail_blocks_reconciliation_and_publication() {
    let (repository, attempt) = repository_with_attempt(0x1e, 0x2e, empty_state());
    let due = moment(4, 1);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), due, fixtures::command(0x3e, 1)),
    ));
    let prepared = ready(repository.prepare_fire(attempt, FireRequest::through(due)));

    let mut state = must(repository.state.lock());
    let aggregate = match state.attempts.get_mut(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    let expected = aggregate.head.cursor();
    let receipt_len = aggregate.receipts.len();
    let removed = aggregate.history.pop();
    assert!(removed.is_some());

    assert!(reconcile(aggregate).is_err());
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Reserved(reservation) if reservation.step() == prepared.step()
    ));

    let proposals = reject(&prepared, StableCommandRejection::Stale);
    let draft = draft_moment(&prepared, &proposals, aggregate.control.closure());
    let sealed = must(seal_authority_record(
        &aggregate.head,
        aggregate.control.closure(),
        DraftAuthorityRecord::moment(expected, draft),
    ));
    assert_eq!(
        append_and_publish(aggregate, sealed),
        Err(RuntimeDriveError::Integrity)
    );
    assert_eq!(aggregate.head.cursor(), expected);
    assert!(aggregate.history.is_empty());
    assert_eq!(aggregate.receipts.len(), receipt_len);
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Reserved(reservation) if reservation.step() == prepared.step()
    ));
}

#[test]
fn reopening_releases_an_unpublished_reservation_and_fences_its_old_token() {
    let service = must(RuntimeService::in_memory());
    let execution = closure(0x14, empty_state());
    let key = AttemptKey::from_bytes([0x24; 32]);
    let mut driver = must(service.start_attempt_with_closure(execution.clone(), key));
    let due = moment(5, 0);
    must(driver.admit(AdmitRequest::new(
        InputId::new(1),
        due,
        fixtures::command(0x34, 1),
    )));
    let reader = driver.session_reader();
    let cursor = must(reader.cursor());
    let first = ready(driver.prepare_fire(FireRequest::through(due)));
    let step = first.step();

    drop(driver);

    let mut reopened = must(service.start_attempt_with_closure(execution.clone(), key));
    assert_eq!(must(reopened.session_reader().cursor()), cursor);
    let second = ready(reopened.prepare_fire(FireRequest::through(due)));
    assert_eq!(second.step(), step);
    assert_ne!(second.grant(), first.grant());

    let stale_proposals = reject(&first, StableCommandRejection::Stale);
    assert_eq!(
        reopened.complete_fire(first, stale_proposals),
        Err(RuntimeDriveError::PreparedFireMismatch)
    );

    drop(reopened);

    let mut reopened_again = must(service.start_attempt_with_closure(execution, key));
    let third = ready(reopened_again.prepare_fire(FireRequest::through(due)));
    assert_eq!(third.step(), step);
    assert_ne!(third.grant(), second.grant());
    assert_eq!(
        reopened_again.fail_prepared_fire(second, PreparedFireFailure::EngineFailure),
        Err(RuntimeControlError::PreparedFireMismatch)
    );

    let proposals = reject(&third, StableCommandRejection::Conflict);
    must(reopened_again.complete_fire(third, proposals));
}

#[test]
fn terminal_due_set_publishes_failed_safety_without_consuming_work() {
    let actor = ActorId::from_bytes([0x41; 32]);
    let source = EntityId::from_bytes([0x51; 32]);
    let destination = EntityId::from_bytes([0x52; 32]);
    let item = EntityId::from_bytes([0x53; 32]);
    let accepted = accepted_state(
        vec![
            ContainerRecord::new(source, 2),
            ContainerRecord::new(destination, 2),
        ],
        vec![ContainmentRecord::new(item, source)],
        vec![ContainerAuthorityRecord::new(actor, source)],
    );
    let (repository, attempt) = repository_with_attempt(0x1f, 0x2f, accepted);
    let due = moment(u64::MAX, u64::MAX);
    must(repository.admit(
        attempt,
        AdmitRequest::new(
            InputId::new(1),
            due,
            fixtures::command_with_actor(0x3f, 1, 0x41),
        ),
    ));
    let expected = must(repository.cursor(attempt));
    let snapshot = must(repository.snapshot(attempt));
    let (frontier, due_keys) = {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        let due_set = match aggregate.head.scheduler().clone_least_due() {
            Some(due_set) => due_set,
            None => panic!("terminal command must remain scheduled"),
        };
        (
            aggregate.head.clock().frontier(),
            due_set
                .entries()
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
        )
    };

    let prepared = safety(repository.prepare_fire(attempt, FireRequest::through(due)));
    let step = prepared.step();
    let cause = prepared.cause();
    let evidence = match cause {
        KernelSafetyCause::TerminalClockExhausted { evidence } => evidence,
        _ => panic!("terminal virtual time must select terminal-clock safety"),
    };
    assert_eq!(evidence.due(), due);
    assert_eq!(evidence.preserved_frontier(), frontier);
    assert_eq!(evidence.due_count().get(), 1);
    assert_eq!(must(repository.cursor(attempt)), expected);
    assert_eq!(must(repository.snapshot(attempt)), snapshot);

    let outcome = must(repository.complete_kernel_safety(attempt, prepared));
    assert_eq!(outcome.cause(), cause);
    assert_eq!(outcome.disposition(), KernelSafetyDisposition::Failed);
    assert_ne!(outcome.cursor(), expected);
    assert_eq!(
        repository.manage(
            attempt,
            ManageRequest::new(ManagementRequestId::new(1), SessionManagement::Resume),
        ),
        Err(RuntimeDriveError::IllegalManagement {
            current: SessionMode::Failed,
        })
    );

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.head.mode(), SessionMode::Failed);
    assert_eq!(
        aggregate
            .head
            .safety_blocker()
            .map(|blocker| blocker.cause()),
        Some(cause)
    );
    let retained_due = match aggregate.head.scheduler().clone_least_due() {
        Some(due_set) => due_set,
        None => panic!("terminal safety must preserve its unresolved due set"),
    };
    assert_eq!(
        retained_due
            .entries()
            .iter()
            .map(|(key, _)| *key)
            .collect::<Vec<_>>(),
        due_keys
    );
    assert_eq!(aggregate.head.clock().frontier(), frontier);
    assert_eq!(aggregate.head.accepted(), snapshot.accepted());
    let receipt = match aggregate.receipts.get(&step) {
        Some(receipt) => *receipt,
        None => panic!("published safety step must retain its recovery receipt"),
    };
    assert_eq!(receipt.expected(), expected);
    assert_eq!(receipt.resulting(), outcome.cursor());
    assert_eq!(receipt.record(), outcome.record());
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Active(cursor) if *cursor == outcome.cursor()
    ));
    assert!(matches!(
        aggregate.history.last().map(|record| record.body()),
        Some(AuthorityRecordBody::Management(batch))
            if batch.kernel_safety_cause() == Some(cause)
                && batch.resulting_mode() == SessionMode::Failed
                && batch.preserved_frontier() == frontier
    ));
    assert_eq!(aggregate.history.len(), 2);
    assert_eq!(aggregate.receipts.len(), 2);
}

#[test]
fn evaluable_population_quarantine_preserves_due_set_frontier_and_receipt() {
    let repository = must(MemoryRepository::new());
    let execution = closure_with_config(
        0x4d,
        empty_state(),
        TerminationContractV1::never(),
        must(ExecutionConfigArtifactV3::inline(4, 1, 16)),
    );
    let attempt =
        must(repository.create_or_open(execution, AttemptKey::from_bytes([0x5d; 32]))).attempt();
    let due = moment(18, 0);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), due, fixtures::command(0x6d, 1)),
    ));
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(2), due, fixtures::command(0x6e, 2)),
    ));

    let expected = must(repository.cursor(attempt));
    let snapshot = must(repository.snapshot(attempt));
    let (frontier, due_keys) = {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        let due_set = match aggregate.head.scheduler().clone_least_due() {
            Some(due_set) => due_set,
            None => panic!("both admitted commands must remain scheduled"),
        };
        (
            aggregate.head.clock().frontier(),
            due_set
                .entries()
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
        )
    };

    let prepared = safety(repository.prepare_fire(attempt, FireRequest::through(due)));
    let step = prepared.step();
    let cause = prepared.cause();
    let evidence = match cause {
        KernelSafetyCause::EvaluableCommandPopulationExceeded {
            limit,
            observed,
            evidence,
        } => {
            assert_eq!(limit.get(), 1);
            assert_eq!(observed, 2);
            evidence
        }
        _ => panic!("evaluable-command excess must select population safety"),
    };
    assert_eq!(evidence.due(), due);
    assert_eq!(evidence.preserved_frontier(), frontier);
    assert_eq!(evidence.due_count().get(), 2);

    let operation_fingerprint = {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        assert_eq!(aggregate.head.cursor(), expected);
        assert_eq!(aggregate.head.clock().frontier(), frontier);
        let reservation = match aggregate.control.phase() {
            AttemptPhase::Reserved(reservation) if reservation.step() == step => reservation,
            _ => panic!("kernel safety preparation must retain its exact reservation"),
        };
        assert!(matches!(
            reservation.operation(),
            crate::attempt::ReservedOperationDescriptor::KernelSafety {
                cause: retained,
            } if retained == cause
        ));
        reservation.operation_fingerprint()
    };

    let outcome = must(repository.complete_kernel_safety(attempt, prepared));
    assert_eq!(outcome.cause(), cause);
    assert_eq!(outcome.disposition(), KernelSafetyDisposition::Quarantined);
    assert_ne!(outcome.cursor(), expected);
    assert_eq!(
        repository.manage(
            attempt,
            ManageRequest::new(ManagementRequestId::new(1), SessionManagement::Resume),
        ),
        Err(RuntimeDriveError::IllegalManagement {
            current: SessionMode::Quarantined,
        })
    );
    assert!(matches!(
        repository.prepare_fire(attempt, FireRequest::through(due)),
        Err(RuntimeDriveError::SessionNotRunning {
            current: SessionMode::Quarantined,
        })
    ));
    let read = must(repository.read(attempt));
    assert_eq!(read.cursor(), outcome.cursor());
    assert_eq!(read.mode(), SessionMode::Quarantined);
    assert_eq!(
        read.safety_blocker().map(|blocker| blocker.cause()),
        Some(cause)
    );
    assert_eq!(
        read.safety_blocker().map(|blocker| blocker.disposition()),
        Some(KernelSafetyDisposition::Quarantined)
    );
    assert_eq!(read.snapshot().accepted(), snapshot.accepted());

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.head.mode(), SessionMode::Quarantined);
    assert_eq!(
        aggregate
            .head
            .safety_blocker()
            .map(|blocker| blocker.cause()),
        Some(cause)
    );
    let retained_due = match aggregate.head.scheduler().clone_least_due() {
        Some(due_set) => due_set,
        None => panic!("quarantine must preserve its unresolved due set"),
    };
    assert_eq!(
        retained_due
            .entries()
            .iter()
            .map(|(key, _)| *key)
            .collect::<Vec<_>>(),
        due_keys
    );
    assert_eq!(aggregate.head.clock().frontier(), frontier);
    assert_eq!(aggregate.head.accepted(), snapshot.accepted());
    let receipt = match aggregate.receipts.get(&step) {
        Some(receipt) => *receipt,
        None => panic!("published safety step must retain its recovery receipt"),
    };
    assert_eq!(receipt.expected(), expected);
    assert_eq!(receipt.resulting(), outcome.cursor());
    assert_eq!(receipt.record(), outcome.record());
    assert_eq!(receipt.operation_fingerprint(), operation_fingerprint);
    assert_eq!(aggregate.head.cursor(), outcome.cursor());
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Active(cursor) if *cursor == outcome.cursor()
    ));
    assert!(matches!(
        aggregate.history.last().map(|record| record.body()),
        Some(AuthorityRecordBody::Management(batch))
            if batch.kernel_safety_cause() == Some(cause)
                && batch.resulting_mode() == SessionMode::Quarantined
                && batch.preserved_frontier() == frontier
    ));
    assert_eq!(aggregate.history.len(), 3);
    assert_eq!(aggregate.receipts.len(), 3);
}

#[test]
fn reconciliation_fences_a_stale_kernel_safety_capability_without_changing_its_step() {
    let repository = must(MemoryRepository::new());
    let execution = closure_with_config(
        0x50,
        empty_state(),
        TerminationContractV1::never(),
        must(ExecutionConfigArtifactV3::inline(4, 1, 16)),
    );
    let attempt =
        must(repository.create_or_open(execution, AttemptKey::from_bytes([0x60; 32]))).attempt();
    let due = moment(18, 1);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), due, fixtures::command(0x71, 1)),
    ));
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(2), due, fixtures::command(0x72, 2)),
    ));

    let stale = safety(repository.prepare_fire(attempt, FireRequest::through(due)));
    let step = stale.step();
    let grant = stale.grant();
    must(repository.reconcile_for_open(attempt));

    let live = safety(repository.prepare_fire(attempt, FireRequest::through(due)));
    assert_eq!(live.step(), step);
    assert_ne!(live.grant(), grant);
    assert_eq!(live.cause(), stale.cause());
    assert_eq!(
        repository.complete_kernel_safety(attempt, stale),
        Err(RuntimeDriveError::PreparedKernelSafetyMismatch)
    );

    let outcome = must(repository.complete_kernel_safety(attempt, live));
    assert_eq!(outcome.disposition(), KernelSafetyDisposition::Quarantined);
    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.head.cursor(), outcome.cursor());
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Active(cursor) if *cursor == outcome.cursor()
    ));
    assert!(aggregate.receipts.contains_key(&step));
}

#[test]
fn same_time_wave_pause_resumes_a_new_tranche_and_fires_the_preserved_due_set() {
    let repository = must(MemoryRepository::new());
    let execution = closure_with_config(
        0x4f,
        empty_state(),
        TerminationContractV1::never(),
        must(ExecutionConfigArtifactV3::inline(4, 4, 1)),
    );
    let attempt =
        must(repository.create_or_open(execution, AttemptKey::from_bytes([0x5f; 32]))).attempt();
    let first_due = moment(19, 0);
    let second_due = moment(19, 1);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), first_due, fixtures::command(0x6f, 1)),
    ));
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(2), second_due, fixtures::command(0x70, 2)),
    ));

    let first = ready(repository.prepare_fire(attempt, FireRequest::through(first_due)));
    let proposals = reject(&first, StableCommandRejection::Conflict);
    must(repository.complete_fire(attempt, first, proposals));

    let (expected, frontier, due_keys) = {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        assert_eq!(
            aggregate.head.clock().same_time_tranche().completed_waves(),
            1
        );
        let due_set = match aggregate.head.scheduler().clone_least_due() {
            Some(due_set) => due_set,
            None => panic!("second same-time wave must remain scheduled"),
        };
        (
            aggregate.head.cursor(),
            aggregate.head.clock().frontier(),
            due_set
                .entries()
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(frontier, second_due);

    let prepared = safety(repository.prepare_fire(attempt, FireRequest::through(second_due)));
    let cause = prepared.cause();
    let evidence = match cause {
        KernelSafetyCause::SameTimeWaveExhausted {
            limit,
            attempted_wave,
            evidence,
        } => {
            assert_eq!(limit.get(), 1);
            assert_eq!(attempted_wave, 2);
            evidence
        }
        _ => panic!("the second same-time wave must pause at a one-wave tranche"),
    };
    assert_eq!(evidence.due(), second_due);
    assert_eq!(evidence.preserved_frontier(), frontier);
    assert_eq!(evidence.due_count().get(), 1);

    let paused = must(repository.complete_kernel_safety(attempt, prepared));
    assert_eq!(paused.cause(), cause);
    assert_eq!(paused.disposition(), KernelSafetyDisposition::Paused);
    assert_ne!(paused.cursor(), expected);
    let paused_read = must(repository.read(attempt));
    assert_eq!(paused_read.mode(), SessionMode::Paused);
    assert_eq!(
        paused_read.safety_blocker().map(|blocker| blocker.cause()),
        Some(cause)
    );
    assert_eq!(paused_read.same_time_wave_tranche().completed_waves(), 1);

    {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        assert_eq!(aggregate.head.mode(), SessionMode::Paused);
        assert_eq!(
            aggregate
                .head
                .safety_blocker()
                .map(|blocker| blocker.cause()),
            Some(cause)
        );
        let due_set = match aggregate.head.scheduler().clone_least_due() {
            Some(due_set) => due_set,
            None => panic!("wave pause must preserve its unresolved due set"),
        };
        assert_eq!(
            due_set
                .entries()
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
            due_keys
        );
        assert_eq!(aggregate.head.clock().frontier(), frontier);
    }

    let resume = ManageRequest::new(ManagementRequestId::new(1), SessionManagement::Resume);
    let resumed = must(repository.manage(attempt, resume));
    assert_eq!(resumed.resulting_mode(), Some(SessionMode::Running));
    let resumed_cursor = must(repository.cursor(attempt));
    assert_eq!(must(repository.manage(attempt, resume)), resumed);
    assert_eq!(must(repository.cursor(attempt)), resumed_cursor);
    let resumed_read = must(repository.read(attempt));
    assert_eq!(resumed_read.mode(), SessionMode::Running);
    assert_eq!(resumed_read.safety_blocker(), None);
    assert_eq!(
        resumed_read.same_time_wave_tranche().time(),
        second_due.time()
    );
    assert_eq!(resumed_read.same_time_wave_tranche().completed_waves(), 0);

    {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        assert_eq!(aggregate.head.mode(), SessionMode::Running);
        assert_eq!(aggregate.head.safety_blocker(), None);
        assert_eq!(aggregate.head.clock().frontier(), frontier);
        assert_eq!(
            aggregate.head.clock().same_time_tranche().time(),
            second_due.time()
        );
        assert_eq!(
            aggregate.head.clock().same_time_tranche().completed_waves(),
            0
        );
        let due_set = match aggregate.head.scheduler().clone_least_due() {
            Some(due_set) => due_set,
            None => panic!("resume must preserve the paused due set"),
        };
        assert_eq!(
            due_set
                .entries()
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
            due_keys
        );
    }

    let second = ready(repository.prepare_fire(attempt, FireRequest::through(second_due)));
    let proposals = reject(&second, StableCommandRejection::Conflict);
    let fired = must(repository.complete_fire(attempt, second, proposals));
    assert_eq!(fired.moment(), second_due);

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.head.mode(), SessionMode::Running);
    assert_eq!(aggregate.head.safety_blocker(), None);
    assert!(aggregate.head.scheduler().is_empty());
    assert_eq!(
        aggregate.head.clock().same_time_tranche().completed_waves(),
        1
    );
    assert_eq!(aggregate.history.len(), 6);
    assert_eq!(aggregate.receipts.len(), 6);
}

#[test]
fn host_terminal_mode_replaces_a_live_pause_blocker_without_discarding_due_work() {
    let repository = must(MemoryRepository::new());
    let execution = closure_with_config(
        0x51,
        empty_state(),
        TerminationContractV1::never(),
        must(ExecutionConfigArtifactV3::inline(4, 4, 1)),
    );
    let attempt =
        must(repository.create_or_open(execution, AttemptKey::from_bytes([0x61; 32]))).attempt();
    let first_due = moment(21, 0);
    let blocked_due = moment(21, 1);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), first_due, fixtures::command(0x71, 1)),
    ));
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(2), blocked_due, fixtures::command(0x72, 2)),
    ));

    let first = ready(repository.prepare_fire(attempt, FireRequest::through(first_due)));
    let proposals = reject(&first, StableCommandRejection::Conflict);
    must(repository.complete_fire(attempt, first, proposals));
    let prepared = safety(repository.prepare_fire(attempt, FireRequest::through(blocked_due)));
    must(repository.complete_kernel_safety(attempt, prepared));

    let (frontier, accepted, due) = {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        assert_eq!(aggregate.head.mode(), SessionMode::Paused);
        assert!(aggregate.head.safety_blocker().is_some());
        (
            aggregate.head.clock().frontier(),
            aggregate.head.accepted().clone(),
            aggregate.head.scheduler().clone_least_due(),
        )
    };

    let request = ManageRequest::new(ManagementRequestId::new(0), SessionManagement::Quarantine);
    let outcome = must(repository.manage(attempt, request));
    assert_eq!(outcome.resulting_mode(), Some(SessionMode::Quarantined));

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.head.mode(), SessionMode::Quarantined);
    assert_eq!(aggregate.head.safety_blocker(), None);
    assert_eq!(aggregate.head.clock().frontier(), frontier);
    assert_eq!(aggregate.head.accepted(), &accepted);
    assert_eq!(aggregate.head.scheduler().clone_least_due(), due);
    assert!(matches!(
        aggregate.history.last().map(|record| record.body()),
        Some(AuthorityRecordBody::Management(batch))
            if batch.kernel_safety_cause().is_none()
                && batch.resulting_mode() == SessionMode::Quarantined
                && matches!(
                    batch.entries(),
                    [entry] if entry.operation() == SessionManagement::Quarantine
                        && entry.outcome() == outcome
                )
    ));
}

fn assert_invalid_transfer_is_stably_rejected(
    seed_byte: u8,
    key_byte: u8,
    accepted: AcceptedState,
    delta: ContainmentTransferDelta,
    expected: StableCommandRejection,
) {
    let (repository, attempt) = repository_with_attempt(seed_byte, key_byte, accepted);
    let due = moment(13, 0);
    let admission = must(repository.admit(
        attempt,
        AdmitRequest::new(
            InputId::new(1),
            due,
            fixtures::command_with_actor(seed_byte, 1, 0x41),
        ),
    ));
    let base_cursor = must(repository.cursor(attempt));
    let base_snapshot = must(repository.snapshot(attempt));
    let prepared = ready(repository.prepare_fire(attempt, FireRequest::through(due)));
    let proposal = accept(&prepared, delta);
    let outcome = must(repository.complete_fire(attempt, prepared, proposal));

    assert_ne!(outcome.cursor(), base_cursor);
    assert_eq!(
        outcome.command_resolutions(),
        [crate::kernel::CommandFireResolution::new(
            fixtures::command_with_actor(seed_byte, 1, 0x41).source(),
            fixtures::command_with_actor(seed_byte, 1, 0x41).id(),
            crate::kernel::CommandFireClassification::New(CommandAttemptOutcome::Rejected(
                expected
            ),),
        )]
    );
    let snapshot = must(repository.snapshot(attempt));
    assert_eq!(snapshot.accepted(), base_snapshot.accepted());

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.head.cursor(), outcome.cursor());
    assert_eq!(aggregate.head.accepted(), base_snapshot.accepted());
    assert_eq!(aggregate.history.len(), 2);
    assert_eq!(aggregate.receipts.len(), 2);
    assert_eq!(aggregate.history[0].header().id(), admission.record());
    assert_eq!(aggregate.history[1].header().id(), outcome.record());
    assert!(aggregate.head.scheduler().is_empty());
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Active(cursor) if *cursor == outcome.cursor()
    ));
    assert!(matches!(
        aggregate.history[1].body(),
        AuthorityRecordBody::Moment(batch)
            if matches!(
                batch.attempts(),
                [attempt]
                    if attempt.resolution().outcome()
                        == CommandAttemptOutcome::Rejected(expected)
            )
    ));
}

#[test]
fn transfer_revalidation_turns_source_mismatch_into_stable_rejection() {
    let actor = ActorId::from_bytes([0x41; 32]);
    let actual_source = EntityId::from_bytes([0x61; 32]);
    let claimed_source = EntityId::from_bytes([0x62; 32]);
    let destination = EntityId::from_bytes([0x63; 32]);
    let item = EntityId::from_bytes([0x64; 32]);
    let accepted = accepted_state(
        vec![
            ContainerRecord::new(actual_source, 2),
            ContainerRecord::new(claimed_source, 2),
            ContainerRecord::new(destination, 2),
        ],
        vec![ContainmentRecord::new(item, actual_source)],
        vec![ContainerAuthorityRecord::new(actor, claimed_source)],
    );
    let delta = must(ContainmentTransferDelta::new(
        actor,
        item,
        claimed_source,
        destination,
    ));

    assert_invalid_transfer_is_stably_rejected(
        0x40,
        0x50,
        accepted,
        delta,
        StableCommandRejection::Stale,
    );
}

#[test]
fn transfer_revalidation_turns_missing_authority_into_stable_rejection() {
    let actor = ActorId::from_bytes([0x41; 32]);
    let source = EntityId::from_bytes([0x65; 32]);
    let destination = EntityId::from_bytes([0x66; 32]);
    let item = EntityId::from_bytes([0x67; 32]);
    let accepted = accepted_state(
        vec![
            ContainerRecord::new(source, 2),
            ContainerRecord::new(destination, 2),
        ],
        vec![ContainmentRecord::new(item, source)],
        Vec::new(),
    );
    let delta = must(ContainmentTransferDelta::new(
        actor,
        item,
        source,
        destination,
    ));

    assert_invalid_transfer_is_stably_rejected(
        0x42,
        0x52,
        accepted,
        delta,
        StableCommandRejection::RequirementUnsatisfied,
    );
}

#[test]
fn transfer_revalidation_turns_full_destination_into_stable_rejection() {
    let actor = ActorId::from_bytes([0x41; 32]);
    let source = EntityId::from_bytes([0x68; 32]);
    let destination = EntityId::from_bytes([0x69; 32]);
    let item = EntityId::from_bytes([0x6a; 32]);
    let occupant = EntityId::from_bytes([0x6b; 32]);
    let accepted = accepted_state(
        vec![
            ContainerRecord::new(source, 2),
            ContainerRecord::new(destination, 1),
        ],
        vec![
            ContainmentRecord::new(item, source),
            ContainmentRecord::new(occupant, destination),
        ],
        vec![ContainerAuthorityRecord::new(actor, source)],
    );
    let delta = must(ContainmentTransferDelta::new(
        actor,
        item,
        source,
        destination,
    ));

    assert_invalid_transfer_is_stably_rejected(
        0x43,
        0x53,
        accepted,
        delta,
        StableCommandRejection::RequirementUnsatisfied,
    );
}

#[test]
fn prepared_fire_failure_finalizes_without_world_publication() {
    let (repository, attempt) = repository_with_attempt(0x15, 0x25, empty_state());
    let due = moment(6, 0);
    let request = AdmitRequest::new(InputId::new(1), due, fixtures::command(0x35, 1));
    let admission = must(repository.admit(attempt, request.clone()));
    let terminal = must(repository.cursor(attempt));
    let snapshot = must(repository.snapshot(attempt));
    let prepared = ready(repository.prepare_fire(attempt, FireRequest::through(due)));

    let outcome =
        must(repository.fail_prepared_fire(attempt, prepared, PreparedFireFailure::EngineFailure));
    let finalization = outcome.finalization();
    assert_eq!(finalization.attempt(), attempt);
    assert_eq!(finalization.terminal(), terminal);
    let disposition = match finalization.cause() {
        RunFinalizationCause::EngineFailure { disposition } => disposition,
        _ => panic!("engine failure must select the matching terminal cause"),
    };
    assert_eq!(must(repository.cursor(attempt)), terminal);
    assert_eq!(must(repository.snapshot(attempt)), snapshot);

    {
        let state = must(repository.state.lock());
        let aggregate = match state.attempts.get(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        assert_eq!(aggregate.history.len(), 1);
        assert_eq!(aggregate.history[0].header().id(), admission.record());
        assert_eq!(aggregate.receipts.len(), 1);
        assert_eq!(
            aggregate.dispositions.get(disposition),
            Some(AttemptDisposition::EngineFailure)
        );
        assert!(matches!(
            aggregate.control.phase(),
            AttemptPhase::Finalized(retained) if *retained == finalization
        ));
    }

    assert_eq!(
        must(repository.admit(attempt, request)),
        admission,
        "an exact retained retry remains readable after finalization"
    );
    assert!(matches!(
        repository.prepare_fire(attempt, FireRequest::through(due)),
        Err(RuntimeDriveError::AttemptFinalized { finalization: retained })
            if retained == finalization
    ));
}

#[test]
fn host_budget_failure_finalizes_without_world_publication() {
    let (repository, attempt) = repository_with_attempt(0x44, 0x54, empty_state());
    let due = moment(14, 0);
    let admission = must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), due, fixtures::command(0x45, 1)),
    ));
    let terminal = must(repository.cursor(attempt));
    let snapshot = must(repository.snapshot(attempt));
    let prepared = ready(repository.prepare_fire(attempt, FireRequest::through(due)));

    let outcome = must(repository.fail_prepared_fire(
        attempt,
        prepared,
        PreparedFireFailure::HostBudgetExceeded,
    ));
    let finalization = outcome.finalization();
    assert_eq!(finalization.attempt(), attempt);
    assert_eq!(finalization.terminal(), terminal);
    let disposition = match finalization.cause() {
        RunFinalizationCause::HostBudgetExceeded { disposition } => disposition,
        _ => panic!("host budget failure must select the matching terminal cause"),
    };
    assert_eq!(must(repository.cursor(attempt)), terminal);
    assert_eq!(must(repository.snapshot(attempt)), snapshot);

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.history.len(), 1);
    assert_eq!(aggregate.history[0].header().id(), admission.record());
    assert_eq!(aggregate.receipts.len(), 1);
    assert_eq!(
        aggregate.dispositions.get(disposition),
        Some(AttemptDisposition::HostBudgetExceeded)
    );
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Finalized(retained) if *retained == finalization
    ));
}

#[test]
fn cancellation_waits_for_reserved_fire_and_retains_exact_retry() {
    let (repository, attempt) = repository_with_attempt(0x16, 0x26, empty_state());
    let due = moment(7, 0);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(1), due, fixtures::command(0x36, 1)),
    ));
    let terminal = must(repository.cursor(attempt));
    let prepared = ready(repository.prepare_fire(attempt, FireRequest::through(due)));
    let request =
        CancelAttemptRequest::new(CancelAttemptRequestId::new(8), CancelReason::HostRequested);

    assert_eq!(
        repository.cancel_attempt(attempt, request),
        Err(RuntimeControlError::StepReserved)
    );
    drop(prepared);
    must(repository.reconcile_for_open(attempt));

    let first = must(repository.cancel_attempt(attempt, request));
    let retry = must(repository.cancel_attempt(attempt, request));
    assert_eq!(retry, first);
    assert_eq!(first.finalization().terminal(), terminal);
    let disposition = match first.finalization().cause() {
        RunFinalizationCause::Cancelled { disposition } => disposition,
        _ => panic!("host cancellation must select the cancellation cause"),
    };

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert_eq!(aggregate.head.cursor(), terminal);
    assert_eq!(aggregate.history.len(), 1);
    assert_eq!(aggregate.receipts.len(), 1);
    assert_eq!(aggregate.dispositions.len(), 1);
    assert!(matches!(
        aggregate.dispositions.get(disposition),
        Some(AttemptDisposition::CancelRequested { request: id, .. })
            if id == CancelAttemptRequestId::new(8)
    ));
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Finalized(finalization) if *finalization == first.finalization()
    ));
}

#[test]
fn cancellation_ledger_distinguishes_exact_and_mismatched_replay() {
    let repository = must(MemoryRepository::new());
    let attempt = must(repository.create_or_open(
        closure(0x48, empty_state()),
        AttemptKey::from_bytes([0x58; 32]),
    ))
    .attempt();
    let other = must(repository.create_or_open(
        closure(0x49, empty_state()),
        AttemptKey::from_bytes([0x59; 32]),
    ))
    .attempt();
    let request =
        CancelAttemptRequest::new(CancelAttemptRequestId::new(0), CancelReason::HostRequested);
    let mismatched = {
        let state = must(repository.state.lock());
        let other = match state.attempts.get(&other) {
            Some(aggregate) => aggregate,
            None => panic!("second attempt must remain retained"),
        };
        request.bind(other.control.binding())
    };

    let first = must(repository.cancel_attempt(attempt, request));
    assert_eq!(must(repository.cancel_attempt(attempt, request)), first);

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("cancelled attempt must remain retained"),
    };
    assert_eq!(
        aggregate.control.classify_cancellation(mismatched),
        CancellationLookup::IdReuseMismatch
    );
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Finalized(finalization) if *finalization == first.finalization()
    ));
}

#[test]
fn post_commit_work_is_consumed_before_later_command_delivery() {
    let actor = ActorId::from_bytes([0x41; 32]);
    let source = EntityId::from_bytes([0x51; 32]);
    let destination = EntityId::from_bytes([0x52; 32]);
    let item = EntityId::from_bytes([0x53; 32]);
    let accepted = accepted_state(
        vec![
            ContainerRecord::new(source, 2),
            ContainerRecord::new(destination, 2),
        ],
        vec![ContainmentRecord::new(item, source)],
        vec![ContainerAuthorityRecord::new(actor, source)],
    );
    let (repository, attempt) = repository_with_attempt(0x17, 0x27, accepted);
    let due = moment(8, 0);
    must(repository.admit(
        attempt,
        AdmitRequest::new(
            InputId::new(1),
            due,
            fixtures::command_with_actor(0x37, 1, 0x41),
        ),
    ));
    let prepared = ready(repository.prepare_fire(attempt, FireRequest::through(due)));
    let delta = must(ContainmentTransferDelta::new(
        actor,
        item,
        source,
        destination,
    ));

    let post_commit_moment = {
        let mut state = must(repository.state.lock());
        let aggregate = match state.attempts.get_mut(&attempt) {
            Some(aggregate) => aggregate,
            None => panic!("created attempt must remain retained"),
        };
        let proposals = accept(&prepared, delta);
        let draft = draft_moment(&prepared, &proposals, aggregate.control.closure());
        let sealed = must(seal_authority_record(
            &aggregate.head,
            aggregate.control.closure(),
            DraftAuthorityRecord::moment(aggregate.head.cursor(), draft),
        ));
        must(append_and_publish(aggregate, sealed));
        must(reconcile(aggregate));
        assert_eq!(
            aggregate
                .head
                .accepted()
                .domain()
                .containment_for(item)
                .map(|record| record.container()),
            Some(destination)
        );
        match aggregate.head.scheduler().first() {
            Some((key, ScheduledWork::PostCommit(_))) => key.moment(),
            _ => panic!("accepted transfer must schedule its reaction dispatch"),
        }
    };

    let later = moment(9, 0);
    must(repository.admit(
        attempt,
        AdmitRequest::new(InputId::new(2), later, fixtures::command(0x38, 2)),
    ));
    let cursor = must(repository.cursor(attempt));
    let routing = ready(repository.prepare_fire(attempt, FireRequest::through(later)));
    assert!(matches!(
        routing.work().collect::<Vec<_>>().as_slice(),
        [MomentWorkInput::PostCommitDispatch { due, .. }] if *due == post_commit_moment
    ));
    let routing_proposals = reject(&routing, StableCommandRejection::Conflict);
    let routing_outcome = must(repository.complete_fire(attempt, routing, routing_proposals));
    assert_eq!(routing_outcome.post_commit_consumed(), 1);
    assert!(routing_outcome.command_resolutions().is_empty());
    assert_ne!(routing_outcome.cursor(), cursor);

    let command = ready(repository.prepare_fire(attempt, FireRequest::through(later)));
    assert!(matches!(
        command.work().collect::<Vec<_>>().as_slice(),
        [MomentWorkInput::EvaluateCommand { due, .. }] if *due == later
    ));
    drop(command);

    let state = must(repository.state.lock());
    let aggregate = match state.attempts.get(&attempt) {
        Some(aggregate) => aggregate,
        None => panic!("created attempt must remain retained"),
    };
    assert!(matches!(
        aggregate.control.phase(),
        AttemptPhase::Reserved(_)
    ));
    assert!(matches!(
        aggregate.head.scheduler().first(),
        Some((key, ScheduledWork::Command(_))) if key.moment() == later
    ));
}
