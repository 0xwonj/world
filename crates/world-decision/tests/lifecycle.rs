use core::fmt::Debug;

use world_context::{
    ActivityAdvancementPayload, ActivityControllerSemanticsId, ActivityEvaluationCause,
    AppraisalEvaluatorSemanticsId, ContainmentActivityProjector, ContainmentAppraisalProjector,
    ContainmentIntentProjector, EvidenceAssimilationPayload, EvidenceAssimilationSemanticsId,
};
use world_core::{ActorId, EntityId, WorldRevision};
use world_decision::{
    ActivityActionDirective, ActivityAdvancementDecision, ActivityController,
    ActivityControllerError, ActivityInitializationDecision, AppraisalEvaluationError,
    AppraisalEvaluator, BaselineActivityController, BaselineAppraisalEvaluator,
    BaselineEvidenceAssimilator, BaselineIntentPolicy, ContainmentAppraisalEvaluation,
    EvidenceAssimilationError, EvidenceAssimilator, IntentDecision, IntentPolicy,
    IntentPolicyError,
};
use world_model::{
    AcceptedState, Activity, ActivityControllerId, ActivityGeneration, ActivityStateSchemaId,
    ActivityStatus, ActivityTransition, AgencyState, ContainmentAppraisal,
    ContainmentTransferDelta, DesiredCondition, DomainState, EpistemicState,
    EpistemicTransitionError, EpistemicVersion, EvidenceDeliveryGeneration, EvidenceRecord, Intent,
    IntentGeneration, IntentStatus, IntentTransition, PhysicalEvent, SocialState, WorldSnapshot,
};

fn valid<T, E: Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("fixture must be valid: {error:?}"))
}

fn actor(byte: u8) -> ActorId {
    ActorId::from_bytes([byte; 32])
}

fn entity(byte: u8) -> EntityId {
    EntityId::from_bytes([byte; 32])
}

fn evidence(
    observer: ActorId,
    generation: u64,
    event_actor: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
) -> EvidenceRecord {
    let delta = valid(ContainmentTransferDelta::new(
        event_actor,
        item,
        source,
        destination,
    ));
    let PhysicalEvent::ItemTransferred(event) = PhysicalEvent::item_transferred(delta) else {
        unreachable!("item-transfer constructor must produce an item-transfer event");
    };
    EvidenceRecord::direct_item_transfer(
        observer,
        EvidenceDeliveryGeneration::new(generation)
            .unwrap_or_else(|| panic!("test generation is nonzero")),
        event,
    )
}

fn assimilated(observer: ActorId, records: Vec<EvidenceRecord>) -> EpistemicState {
    valid(EpistemicState::empty().assimilate(observer, EpistemicVersion::EMPTY, records))
}

fn absence(
    observer: ActorId,
    generation: u64,
    item: EntityId,
    expected_container: EntityId,
) -> EvidenceRecord {
    EvidenceRecord::direct_item_absent(
        observer,
        EvidenceDeliveryGeneration::new(generation)
            .unwrap_or_else(|| panic!("test generation is nonzero")),
        item,
        expected_container,
    )
}

fn accepted_snapshot(epistemic: EpistemicState, agency: AgencyState) -> WorldSnapshot {
    WorldSnapshot::new(
        WorldRevision::ROOT,
        AcceptedState::new(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
            epistemic,
            SocialState::empty(),
            agency,
        ),
    )
}

fn intent(owner: ActorId, item: EntityId, destination: EntityId) -> Intent {
    Intent::adopt(
        owner,
        IntentGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
        DesiredCondition::item_contained_in(item, destination),
    )
}

fn agency(intents: Vec<Intent>, activities: Vec<Activity>) -> AgencyState {
    valid(AgencyState::new(intents, activities, Vec::new()))
}

#[test]
fn lifecycle_ports_remain_object_safe_with_concrete_methods() {
    let evidence = BaselineEvidenceAssimilator::new();
    let appraisal = BaselineAppraisalEvaluator::new();
    let intent = BaselineIntentPolicy::new();
    let activity = BaselineActivityController::new();

    let _: &dyn EvidenceAssimilator = &evidence;
    let _: &dyn AppraisalEvaluator = &appraisal;
    let _: &dyn IntentPolicy = &intent;
    let _: &dyn ActivityController = &activity;
}

#[test]
fn evidence_baseline_preserves_input_and_builds_checked_successor() {
    let owner = actor(0x10);
    let record = evidence(
        owner,
        1,
        actor(0x11),
        entity(0x20),
        entity(0x30),
        entity(0x40),
    );
    let baseline = BaselineEvidenceAssimilator::new();
    let input = valid(EvidenceAssimilationPayload::new(
        owner,
        EpistemicVersion::EMPTY,
        vec![record],
        baseline.semantics_id(),
    ));
    let proposal = valid(baseline.assimilate(&input));

    assert_eq!(proposal.input_fingerprint(), input.fingerprint());
    assert_eq!(proposal.actor(), owner);
    assert_eq!(proposal.expected_version(), EpistemicVersion::EMPTY);
    assert_eq!(proposal.evidence(), &[record]);

    let successor = valid(proposal.apply(&EpistemicState::empty()));
    assert_eq!(successor.actor_version(owner), EpistemicVersion::new(1));
    assert_eq!(successor.evidence_record(record.id()), Some(&record));
    assert_eq!(
        proposal.apply(&successor),
        Err(EpistemicTransitionError::StaleVersion {
            expected: EpistemicVersion::EMPTY,
            actual: EpistemicVersion::new(1),
        })
    );
}

#[test]
fn evidence_baseline_rejects_foreign_semantics() {
    let owner = actor(0x10);
    let record = evidence(
        owner,
        1,
        actor(0x11),
        entity(0x20),
        entity(0x30),
        entity(0x40),
    );
    let input = valid(EvidenceAssimilationPayload::new(
        owner,
        EpistemicVersion::EMPTY,
        vec![record],
        EvidenceAssimilationSemanticsId::from_bytes([0xff; 32]),
    ));
    let baseline = BaselineEvidenceAssimilator::new();

    assert_eq!(
        baseline.assimilate(&input),
        Err(EvidenceAssimilationError::SemanticsMismatch {
            expected: baseline.semantics_id(),
            actual: input.semantics(),
        })
    );
}

#[test]
fn appraisal_baseline_derives_restore_target_and_suppresses_equal_material() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let first = evidence(owner, 1, actor(0x11), item, home, away);
    let baseline = BaselineAppraisalEvaluator::new();
    let first_snapshot = accepted_snapshot(assimilated(owner, vec![first]), AgencyState::empty());
    let first_input = valid(ContainmentAppraisalProjector::new().build(
        &first_snapshot,
        owner,
        item,
        None,
        baseline.semantics_id(),
    ));
    let first_result = valid(baseline.evaluate(&first_input));
    let ContainmentAppraisalEvaluation::Present {
        appraisal: first_appraisal,
        material_changed: first_changed,
        ..
    } = first_result
    else {
        panic!("a positive containment belief must produce a present appraisal")
    };

    assert_eq!(first_result.input_fingerprint(), first_input.fingerprint());
    assert_eq!(first_appraisal.actor(), owner);
    assert_eq!(first_appraisal.item(), item);
    assert_eq!(first_appraisal.believed_current_container(), away);
    assert_eq!(first_appraisal.restore_container(), home);
    assert_eq!(first_appraisal.supporting_evidence(), first.id());
    assert!(first_changed);

    let refreshed = evidence(owner, 2, actor(0x11), item, home, away);
    let refreshed_snapshot = accepted_snapshot(
        assimilated(owner, vec![first, refreshed]),
        AgencyState::empty(),
    );
    let refreshed_input = valid(ContainmentAppraisalProjector::new().build(
        &refreshed_snapshot,
        owner,
        item,
        Some(first_appraisal),
        baseline.semantics_id(),
    ));
    let refreshed_result = valid(baseline.evaluate(&refreshed_input));
    let ContainmentAppraisalEvaluation::Present {
        appraisal: refreshed_appraisal,
        material_changed: refreshed_changed,
        ..
    } = refreshed_result
    else {
        panic!("a refreshed containment belief must produce a present appraisal")
    };

    assert_eq!(refreshed_appraisal.supporting_evidence(), refreshed.id());
    assert_eq!(
        refreshed_appraisal.material_fingerprint(),
        first_appraisal.material_fingerprint()
    );
    assert!(!refreshed_changed);
}

#[test]
fn appraisal_baseline_retracts_only_the_exact_absent_container() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let expected = entity(0x40);
    let other = entity(0x41);
    let transfer = evidence(owner, 1, actor(0x11), item, home, expected);
    let absent = absence(owner, 2, item, expected);
    let previous = ContainmentAppraisal::new(owner, item, expected, home, transfer.id());
    let baseline = BaselineAppraisalEvaluator::new();
    let snapshot = accepted_snapshot(
        assimilated(owner, vec![transfer, absent]),
        AgencyState::empty(),
    );
    let input = valid(ContainmentAppraisalProjector::new().build(
        &snapshot,
        owner,
        item,
        Some(previous),
        baseline.semantics_id(),
    ));

    assert_eq!(
        valid(baseline.evaluate(&input)),
        ContainmentAppraisalEvaluation::Retract {
            input: input.fingerprint(),
            before: previous,
            supporting_evidence: absent.id(),
        }
    );

    let no_previous = valid(ContainmentAppraisalProjector::new().build(
        &snapshot,
        owner,
        item,
        None,
        baseline.semantics_id(),
    ));
    assert_eq!(
        valid(baseline.evaluate(&no_previous)),
        ContainmentAppraisalEvaluation::NoChange {
            input: no_previous.fingerprint(),
        }
    );

    let unrelated_absence = absence(owner, 1, item, other);
    let unrelated_snapshot = accepted_snapshot(
        assimilated(owner, vec![unrelated_absence]),
        AgencyState::empty(),
    );
    let mismatched_previous = valid(ContainmentAppraisalProjector::new().build(
        &unrelated_snapshot,
        owner,
        item,
        Some(previous),
        baseline.semantics_id(),
    ));
    assert_eq!(
        valid(baseline.evaluate(&mismatched_previous)),
        ContainmentAppraisalEvaluation::NoChange {
            input: mismatched_previous.fingerprint(),
        }
    );
}

#[test]
fn appraisal_baseline_rejects_foreign_semantics() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let record = evidence(owner, 1, actor(0x11), item, entity(0x30), entity(0x40));
    let snapshot = accepted_snapshot(assimilated(owner, vec![record]), AgencyState::empty());
    let input = valid(ContainmentAppraisalProjector::new().build(
        &snapshot,
        owner,
        item,
        None,
        AppraisalEvaluatorSemanticsId::from_bytes([0xff; 32]),
    ));
    let baseline = BaselineAppraisalEvaluator::new();

    assert_eq!(
        baseline.evaluate(&input),
        Err(AppraisalEvaluationError::SemanticsMismatch {
            expected: baseline.semantics_id(),
            actual: input.evaluator_semantics(),
        })
    );
}

#[test]
fn intent_baseline_selects_only_a_supplied_grounded_candidate() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let record = evidence(owner, 1, actor(0x11), item, home, away);
    let appraisal = ContainmentAppraisal::new(owner, item, away, home, record.id());
    let policy = BaselineIntentPolicy::new();
    let build = valid(ContainmentIntentProjector::new().build(
        &accepted_snapshot(EpistemicState::empty(), AgencyState::empty()),
        appraisal,
        policy.semantics_id(),
    ));
    let decision = valid(policy.decide(build.payload()));
    let candidate = build.payload().candidates().candidates()[0].id();

    assert_eq!(
        decision,
        IntentDecision::Adopt {
            candidate,
            input: build.payload().fingerprint(),
        }
    );
    assert_eq!(decision.selected_candidate(), Some(candidate));
    assert!(build.resolution().resolve(candidate).is_some());

    let satisfied = ContainmentAppraisal::new(owner, item, home, home, record.id());
    let empty_build = valid(ContainmentIntentProjector::new().build(
        &accepted_snapshot(EpistemicState::empty(), AgencyState::empty()),
        satisfied,
        policy.semantics_id(),
    ));
    assert_eq!(
        valid(policy.decide(empty_build.payload())),
        IntentDecision::NoCandidate {
            input: empty_build.payload().fingerprint(),
        }
    );
}

#[test]
fn intent_baseline_rejects_foreign_semantics() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let record = evidence(owner, 1, actor(0x11), item, entity(0x30), entity(0x40));
    let build = valid(ContainmentIntentProjector::new().build(
        &accepted_snapshot(EpistemicState::empty(), AgencyState::empty()),
        ContainmentAppraisal::new(owner, item, entity(0x40), entity(0x30), record.id()),
        world_context::IntentPolicySemanticsId::from_bytes([0xff; 32]),
    ));
    let baseline = BaselineIntentPolicy::new();

    assert_eq!(
        baseline.decide(build.payload()),
        Err(IntentPolicyError::SemanticsMismatch {
            expected: baseline.semantics_id(),
            actual: build.payload().policy_semantics(),
        })
    );
}

#[test]
fn activity_initialization_opens_exactly_one_restore_home_attempt() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let record = evidence(owner, 1, actor(0x11), item, home, away);
    let intent = intent(owner, item, home);
    let snapshot = accepted_snapshot(
        assimilated(owner, vec![record]),
        agency(vec![intent], Vec::new()),
    );
    let controller = BaselineActivityController::new();
    let input = valid(
        ContainmentActivityProjector::new().initialization(
            &snapshot,
            intent.id(),
            ActivityEvaluationCause::AppraisalChanged {
                appraisal: ContainmentAppraisal::new(owner, item, away, home, record.id())
                    .material_fingerprint(),
            },
            controller.semantics_id(),
        ),
    );
    let decision = valid(controller.initialize(&input));

    assert_eq!(decision.input_fingerprint(), input.fingerprint());
    let state = decision
        .initial_state()
        .unwrap_or_else(|| panic!("displacement must start an activity"));
    let directive = decision
        .directive()
        .unwrap_or_else(|| panic!("started activity must open one action"));
    assert_eq!(state.item(), item);
    assert_eq!(state.source(), away);
    assert_eq!(state.destination(), home);
    assert_eq!(state.remaining_attempts(), 1);
    assert_eq!(state.next_opportunity_generation().get(), 2);
    assert_eq!(directive.generation().get(), 1);
    assert_eq!(directive.scope().source(), away);
    assert_eq!(directive.scope().destinations(), &[home]);
    assert_eq!(directive.scope().items(), &[item]);
    assert_eq!(directive.scope().candidate_limit(), 1);

    let activity = Activity::start(
        owner,
        intent.id(),
        ActivityGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
        ActivityControllerId::from_bytes(controller.implementation_id()),
        controller.state_schema(),
        state,
    );
    assert_eq!(activity.state(), state.into());
}

#[test]
fn activity_initialization_handles_satisfaction_absence_and_semantics() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let intent = intent(owner, item, home);
    let controller = BaselineActivityController::new();

    let satisfied_record = evidence(owner, 1, actor(0x11), item, away, home);
    let satisfied_snapshot = accepted_snapshot(
        assimilated(owner, vec![satisfied_record]),
        agency(vec![intent], Vec::new()),
    );
    let satisfied_input = valid(ContainmentActivityProjector::new().initialization(
        &satisfied_snapshot,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    let satisfied = valid(controller.initialize(&satisfied_input));
    assert!(matches!(
        satisfied,
        ActivityInitializationDecision::AlreadySatisfied { .. }
    ));
    let achieved = valid(
        intent.transition(
            intent.version(),
            satisfied
                .intent_transition()
                .unwrap_or_else(|| panic!("satisfaction must carry an intent transition")),
        ),
    );
    assert!(achieved.status().is_terminal());

    let absent_snapshot =
        accepted_snapshot(EpistemicState::empty(), agency(vec![intent], Vec::new()));
    let absent_input = valid(ContainmentActivityProjector::new().initialization(
        &absent_snapshot,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    let absent = valid(controller.initialize(&absent_input));
    assert!(matches!(
        absent,
        ActivityInitializationDecision::SuspendIntent { .. }
    ));
    let suspended = valid(
        intent.transition(
            intent.version(),
            absent
                .intent_transition()
                .unwrap_or_else(|| panic!("missing belief must carry an intent transition")),
        ),
    );
    assert_eq!(suspended.status(), world_model::IntentStatus::Suspended);

    let foreign_input = valid(ContainmentActivityProjector::new().initialization(
        &absent_snapshot,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        ActivityControllerSemanticsId::from_bytes([0xff; 32]),
    ));
    assert_eq!(
        controller.initialize(&foreign_input),
        Err(ActivityControllerError::SemanticsMismatch {
            expected: controller.semantics_id(),
            actual: foreign_input.controller_semantics(),
        })
    );
}

#[test]
fn activity_advancement_rejects_controller_and_state_schema_takeover() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let record = evidence(owner, 1, actor(0x11), item, home, away);
    let epistemic = assimilated(owner, vec![record]);
    let intent = intent(owner, item, home);
    let controller = BaselineActivityController::new();
    let init_snapshot = accepted_snapshot(epistemic.clone(), agency(vec![intent], Vec::new()));
    let init_input = valid(ContainmentActivityProjector::new().initialization(
        &init_snapshot,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    let state = valid(controller.initialize(&init_input))
        .initial_state()
        .unwrap_or_else(|| panic!("displacement must initialize state"));
    let generation =
        ActivityGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero"));

    let foreign_controller = ActivityControllerId::from_bytes([0xff; 32]);
    let wrong_controller_activity = Activity::start(
        owner,
        intent.id(),
        generation,
        foreign_controller,
        controller.state_schema(),
        state,
    );
    let wrong_controller_snapshot = accepted_snapshot(
        epistemic.clone(),
        agency(vec![intent], vec![wrong_controller_activity]),
    );
    let wrong_controller_input = valid(ContainmentActivityProjector::new().advancement(
        &wrong_controller_snapshot,
        wrong_controller_activity.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    assert_eq!(
        controller.advance(&ActivityAdvancementPayload::ContainmentTransfer(
            wrong_controller_input,
        )),
        Err(ActivityControllerError::ControllerMismatch {
            expected: ActivityControllerId::from_bytes(controller.implementation_id()),
            actual: foreign_controller,
        })
    );

    let foreign_schema = ActivityStateSchemaId::from_bytes([0xfe; 32]);
    let wrong_schema_activity = Activity::start(
        owner,
        intent.id(),
        generation,
        ActivityControllerId::from_bytes(controller.implementation_id()),
        foreign_schema,
        state,
    );
    let wrong_schema_snapshot =
        accepted_snapshot(epistemic, agency(vec![intent], vec![wrong_schema_activity]));
    let wrong_schema_input = valid(ContainmentActivityProjector::new().advancement(
        &wrong_schema_snapshot,
        wrong_schema_activity.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    assert_eq!(
        controller.advance(&ActivityAdvancementPayload::ContainmentTransfer(
            wrong_schema_input,
        )),
        Err(ActivityControllerError::StateSchemaMismatch {
            expected: controller.state_schema(),
            actual: foreign_schema,
        })
    );
}

#[test]
fn activity_advancement_is_bounded_checked_and_outcome_independent() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let record = evidence(owner, 1, actor(0x11), item, home, away);
    let epistemic = assimilated(owner, vec![record]);
    let intent = intent(owner, item, home);
    let controller = BaselineActivityController::new();
    let init_snapshot = accepted_snapshot(epistemic.clone(), agency(vec![intent], Vec::new()));
    let init_input = valid(ContainmentActivityProjector::new().initialization(
        &init_snapshot,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    let init_decision = valid(controller.initialize(&init_input));
    let initial_state = init_decision
        .initial_state()
        .unwrap_or_else(|| panic!("displacement must initialize state"));
    let activity = Activity::start(
        owner,
        intent.id(),
        ActivityGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
        ActivityControllerId::from_bytes(controller.implementation_id()),
        controller.state_schema(),
        initial_state,
    );
    let active_snapshot =
        accepted_snapshot(epistemic.clone(), agency(vec![intent], vec![activity]));
    let attempted_input = valid(ContainmentActivityProjector::new().advancement(
        &active_snapshot,
        activity.id(),
        ActivityEvaluationCause::AttemptedAction,
        controller.semantics_id(),
    ));
    let recovery_input = valid(ContainmentActivityProjector::new().advancement(
        &active_snapshot,
        activity.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    let attempted = valid(
        controller.advance(&ActivityAdvancementPayload::ContainmentTransfer(
            attempted_input,
        )),
    );
    let recovery = valid(
        controller.advance(&ActivityAdvancementPayload::ContainmentTransfer(
            recovery_input,
        )),
    );

    assert_ne!(attempted.input_fingerprint(), recovery.input_fingerprint());
    assert_eq!(attempted.transition(), recovery.transition());
    assert_eq!(attempted.directive(), recovery.directive());
    let retry_transition = attempted
        .transition()
        .unwrap_or_else(|| panic!("one recovery attempt must remain"));
    let retry_directive = attempted
        .directive()
        .unwrap_or_else(|| panic!("retry must open one action"));
    let ActivityActionDirective::ContainmentTransfer(retry_directive) = retry_directive else {
        panic!("containment activity must open a containment action");
    };
    assert_eq!(retry_directive.generation().get(), 2);
    assert_eq!(retry_directive.scope().items(), &[item]);
    let advanced = valid(activity.transition(activity.version(), retry_transition));
    let advanced_state = advanced
        .state()
        .containment_transfer()
        .unwrap_or_else(|| panic!("containment activity must retain containment state"));
    assert_eq!(advanced_state.remaining_attempts(), 0);
    assert_eq!(advanced_state.next_opportunity_generation().get(), 3);

    let exhausted_snapshot = accepted_snapshot(epistemic, agency(vec![intent], vec![advanced]));
    let exhausted_input = valid(ContainmentActivityProjector::new().advancement(
        &exhausted_snapshot,
        advanced.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    let exhausted = valid(
        controller.advance(&ActivityAdvancementPayload::ContainmentTransfer(
            exhausted_input,
        )),
    );
    let (activity_transition, intent_transition) = match exhausted {
        ActivityAdvancementDecision::Fail {
            transition,
            intent_transition,
            ..
        } => (transition, intent_transition),
        other => panic!("exhaustion must fail activity and intent: {other:?}"),
    };
    assert_eq!(activity_transition, ActivityTransition::Fail);
    assert_eq!(intent_transition, IntentTransition::Fail);
    let failed_activity = valid(advanced.transition(advanced.version(), activity_transition));
    let failed_intent = valid(intent.transition(intent.version(), intent_transition));
    assert_eq!(failed_activity.status(), ActivityStatus::Failed);
    assert_eq!(failed_intent.status(), IntentStatus::Failed);
}

#[test]
fn activity_advancement_completes_from_belief_and_waits_without_one() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let intent = intent(owner, item, home);
    let controller = BaselineActivityController::new();
    let displaced = evidence(owner, 1, actor(0x11), item, home, away);
    let init_snapshot = accepted_snapshot(
        assimilated(owner, vec![displaced]),
        agency(vec![intent], Vec::new()),
    );
    let init_input = valid(ContainmentActivityProjector::new().initialization(
        &init_snapshot,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    let state = valid(controller.initialize(&init_input))
        .initial_state()
        .unwrap_or_else(|| panic!("displacement must initialize state"));
    let activity = Activity::start(
        owner,
        intent.id(),
        ActivityGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
        ActivityControllerId::from_bytes(controller.implementation_id()),
        controller.state_schema(),
        state,
    );

    let restored = evidence(owner, 1, actor(0x11), item, away, home);
    let complete_snapshot = accepted_snapshot(
        assimilated(owner, vec![restored]),
        agency(vec![intent], vec![activity]),
    );
    let complete_input = valid(ContainmentActivityProjector::new().advancement(
        &complete_snapshot,
        activity.id(),
        ActivityEvaluationCause::AttemptedAction,
        controller.semantics_id(),
    ));
    let complete = valid(
        controller.advance(&ActivityAdvancementPayload::ContainmentTransfer(
            complete_input,
        )),
    );
    let (activity_transition, intent_transition) = match complete {
        ActivityAdvancementDecision::Complete {
            transition,
            intent_transition,
            ..
        } => (transition, intent_transition),
        other => panic!("satisfaction must complete activity and intent: {other:?}"),
    };
    assert_eq!(activity_transition, ActivityTransition::Complete);
    assert_eq!(intent_transition, IntentTransition::Achieve);
    let completed_activity = valid(activity.transition(activity.version(), activity_transition));
    let achieved_intent = valid(intent.transition(intent.version(), intent_transition));
    assert_eq!(completed_activity.status(), ActivityStatus::Completed);
    assert_eq!(achieved_intent.status(), IntentStatus::Achieved);

    let absent_snapshot = accepted_snapshot(
        EpistemicState::empty(),
        agency(vec![intent], vec![activity]),
    );
    let absent_input = valid(ContainmentActivityProjector::new().advancement(
        &absent_snapshot,
        activity.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    let absent = valid(
        controller.advance(&ActivityAdvancementPayload::ContainmentTransfer(
            absent_input,
        )),
    );
    assert!(matches!(absent, ActivityAdvancementDecision::Await { .. }));
    let waiting = valid(
        activity.transition(
            activity.version(),
            absent
                .transition()
                .unwrap_or_else(|| panic!("active activity must transition to waiting")),
        ),
    );
    assert_eq!(waiting.status(), ActivityStatus::Waiting);
}
