use core::fmt::Debug;

use world_context::{
    ActivityAdvancementPayload, ActivityEvaluationCause, ContainmentActivityProjector,
    ContainmentAppraisalPayload, ContainmentAppraisalProjector, ContainmentIntentContextBuild,
    ContainmentIntentProjector, EvidenceAssimilationPayload, EvidenceAssimilationSemanticsId,
    GroundedIntentCandidateId, IntentPolicySemanticsId,
};
use world_core::{ActorId, EntityId, WorldRevision};
use world_decision::{
    AppraisalEvaluationError, AppraisalEvaluator, BaselineActivityController,
    BaselineAppraisalEvaluator, BaselineEvidenceAssimilator, BaselineIntentPolicy,
    ContainmentAppraisalEvaluation, EvidenceAssimilationError, EvidenceAssimilationProposal,
    EvidenceAssimilator, IntentDecision, IntentPolicy, IntentPolicyError, activity_state_schema,
};
use world_model::{
    AcceptedState, ActionSponsor, ActivityControllerId, ActivityGeneration, ActivityStatus,
    AgencyState, ContainmentAppraisal, ContainmentTransferDelta, DesiredCondition, DomainState,
    EpistemicState, EpistemicVersion, EvidenceDeliveryGeneration, EvidenceRecord, Intent,
    IntentGeneration, IntentStatus, PhysicalEvent, SocialState, WorldSnapshot,
};

use super::{
    ActivityCoordinationError, ActivityCoordinator, AppraisalCoordinationError,
    AppraisalCoordinator, CoordinatedActivityAdvancement, CoordinatedActivityInitialization,
    CoordinatedAppraisal, CoordinatedIntentReview, EvidenceCoordinationError, EvidenceCoordinator,
    IntentCoordinationError, IntentCoordinator,
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
    let event = match PhysicalEvent::item_transferred(delta) {
        PhysicalEvent::ItemTransferred(event) => event,
        PhysicalEvent::ActorDeparted(_) | PhysicalEvent::ActorArrived(_) => {
            unreachable!("item transfer constructor returned another event family")
        }
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

fn agency(intents: Vec<Intent>) -> AgencyState {
    valid(AgencyState::new(intents, Vec::new(), Vec::new()))
}

fn snapshot(epistemic: EpistemicState, agency: AgencyState) -> WorldSnapshot {
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

#[derive(Clone)]
struct StoredEvidenceAssimilator {
    semantics: EvidenceAssimilationSemanticsId,
    proposal: EvidenceAssimilationProposal,
}

impl EvidenceAssimilator for StoredEvidenceAssimilator {
    fn implementation_id(&self) -> [u8; 32] {
        self.semantics.into_bytes()
    }

    fn assimilate(
        &self,
        _input: &EvidenceAssimilationPayload,
    ) -> Result<EvidenceAssimilationProposal, EvidenceAssimilationError> {
        Ok(self.proposal.clone())
    }
}

#[derive(Clone, Copy)]
struct StoredAppraisalEvaluator {
    semantics: world_context::AppraisalEvaluatorSemanticsId,
    evaluation: ContainmentAppraisalEvaluation,
}

impl AppraisalEvaluator for StoredAppraisalEvaluator {
    fn implementation_id(&self) -> [u8; 32] {
        self.semantics.into_bytes()
    }

    fn evaluate(
        &self,
        _input: &ContainmentAppraisalPayload,
    ) -> Result<ContainmentAppraisalEvaluation, AppraisalEvaluationError> {
        Ok(self.evaluation)
    }
}

#[derive(Clone, Copy)]
struct FixedIntentPolicy {
    semantics: IntentPolicySemanticsId,
    decision: IntentDecision,
}

impl IntentPolicy for FixedIntentPolicy {
    fn implementation_id(&self) -> [u8; 32] {
        self.semantics.into_bytes()
    }

    fn decide(
        &self,
        _input: &world_context::ContainmentIntentPayload,
    ) -> Result<IntentDecision, IntentPolicyError> {
        Ok(self.decision)
    }
}

fn intent_build(
    owner: ActorId,
    item: EntityId,
    home: EntityId,
    away: EntityId,
    support: EvidenceRecord,
    policy: IntentPolicySemanticsId,
) -> ContainmentIntentContextBuild {
    valid(ContainmentIntentProjector::new().build(
        &snapshot(EpistemicState::empty(), AgencyState::empty()),
        ContainmentAppraisal::new(owner, item, away, home, support.id()),
        policy,
    ))
}

#[test]
fn evidence_coordination_builds_a_checked_successor_and_rejects_replayed_output() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let baseline = BaselineEvidenceAssimilator::new();
    let record = evidence(owner, 1, actor(0x11), item, entity(0x30), entity(0x40));
    let input = valid(EvidenceAssimilationPayload::new(
        owner,
        EpistemicVersion::EMPTY,
        vec![record],
        baseline.semantics_id(),
    ));
    let coordinated = valid(EvidenceCoordinator::coordinate(
        &EpistemicState::empty(),
        &input,
        &baseline,
    ));
    let (fingerprint, coordinated_actor, version, successor) = coordinated.into_parts();

    assert_eq!(fingerprint, input.fingerprint());
    assert_eq!(coordinated_actor, owner);
    assert_eq!(version, EpistemicVersion::EMPTY);
    assert_eq!(successor.evidence_record(record.id()), Some(&record));

    let foreign = evidence(owner, 1, actor(0x11), item, entity(0x31), entity(0x40));
    let foreign_input = valid(EvidenceAssimilationPayload::new(
        owner,
        EpistemicVersion::EMPTY,
        vec![foreign],
        baseline.semantics_id(),
    ));
    let forged = StoredEvidenceAssimilator {
        semantics: baseline.semantics_id(),
        proposal: valid(baseline.assimilate(&foreign_input)),
    };
    assert!(matches!(
        EvidenceCoordinator::coordinate(&EpistemicState::empty(), &input, &forged),
        Err(EvidenceCoordinationError::InputFingerprintMismatch { .. })
    ));
}

#[test]
fn appraisal_coordination_recomputes_meaning_and_material_change() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let record = evidence(owner, 1, actor(0x11), item, home, away);
    let baseline = BaselineAppraisalEvaluator::new();
    let accepted = snapshot(assimilated(owner, vec![record]), AgencyState::empty());
    let input = valid(ContainmentAppraisalProjector::new().build(
        &accepted,
        owner,
        item,
        None,
        baseline.semantics_id(),
    ));
    let coordinated = valid(AppraisalCoordinator::coordinate(&input, &baseline));
    let CoordinatedAppraisal::Present {
        input: fingerprint,
        appraisal,
        material_changed: changed,
    } = coordinated
    else {
        panic!("a positive containment belief must coordinate as present")
    };

    assert_eq!(fingerprint, input.fingerprint());
    assert_eq!(appraisal.restore_container(), home);
    assert_eq!(appraisal.believed_current_container(), away);
    assert!(changed);

    let equal_input = valid(ContainmentAppraisalProjector::new().build(
        &accepted,
        owner,
        item,
        Some(appraisal),
        baseline.semantics_id(),
    ));
    let forged = StoredAppraisalEvaluator {
        semantics: baseline.semantics_id(),
        evaluation: valid(baseline.evaluate(&equal_input)),
    };
    assert!(matches!(
        AppraisalCoordinator::coordinate(&input, &forged),
        Err(AppraisalCoordinationError::InputFingerprintMismatch { .. })
    ));
    let CoordinatedAppraisal::Present {
        material_changed, ..
    } = valid(AppraisalCoordinator::coordinate(&equal_input, &baseline))
    else {
        panic!("an unchanged positive belief must remain present")
    };
    assert!(!material_changed);
}

#[test]
fn appraisal_coordination_accepts_only_an_exact_retraction() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let expected = entity(0x40);
    let other = entity(0x41);
    let transfer = evidence(owner, 1, actor(0x11), item, home, expected);
    let absent = absence(owner, 2, item, expected);
    let previous = ContainmentAppraisal::new(owner, item, expected, home, transfer.id());
    let baseline = BaselineAppraisalEvaluator::new();
    let accepted = snapshot(
        assimilated(owner, vec![transfer, absent]),
        AgencyState::empty(),
    );
    let input = valid(ContainmentAppraisalProjector::new().build(
        &accepted,
        owner,
        item,
        Some(previous),
        baseline.semantics_id(),
    ));

    assert_eq!(
        valid(AppraisalCoordinator::coordinate(&input, &baseline)),
        CoordinatedAppraisal::Retract {
            input: input.fingerprint(),
            before: previous,
            supporting_evidence: absent.id(),
        }
    );

    let forged_before = ContainmentAppraisal::new(owner, item, other, home, transfer.id());
    let forged = StoredAppraisalEvaluator {
        semantics: baseline.semantics_id(),
        evaluation: ContainmentAppraisalEvaluation::Retract {
            input: input.fingerprint(),
            before: forged_before,
            supporting_evidence: absent.id(),
        },
    };
    assert_eq!(
        AppraisalCoordinator::coordinate(&input, &forged),
        Err(AppraisalCoordinationError::AppraisalMismatch)
    );

    let unrelated_absence = absence(owner, 1, item, other);
    let unrelated_snapshot = snapshot(
        assimilated(owner, vec![unrelated_absence]),
        AgencyState::empty(),
    );
    let unrelated_input = valid(ContainmentAppraisalProjector::new().build(
        &unrelated_snapshot,
        owner,
        item,
        Some(previous),
        baseline.semantics_id(),
    ));
    assert_eq!(
        valid(AppraisalCoordinator::coordinate(
            &unrelated_input,
            &baseline,
        )),
        CoordinatedAppraisal::NoChange {
            input: unrelated_input.fingerprint(),
        }
    );
}

#[test]
fn intent_coordination_resolves_only_supplied_private_material() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let support = evidence(owner, 1, actor(0x11), item, home, away);
    let baseline = BaselineIntentPolicy::new();
    let build = intent_build(owner, item, home, away, support, baseline.semantics_id());
    let input_fingerprint = build.payload().fingerprint();
    let coordinated = valid(IntentCoordinator::coordinate(
        &AgencyState::empty(),
        build,
        IntentGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
        &baseline,
    ));
    let adopted = coordinated
        .adopted_intent()
        .unwrap_or_else(|| panic!("displaced item must produce one intent"));

    assert_eq!(coordinated.input_fingerprint(), input_fingerprint);
    assert_eq!(adopted.actor(), owner);
    assert_eq!(
        adopted.desired(),
        DesiredCondition::item_contained_in(item, home)
    );

    let forged_build = intent_build(owner, item, home, away, support, baseline.semantics_id());
    let forged_candidate = GroundedIntentCandidateId::from_bytes([0xff; 32]);
    let forged_policy = FixedIntentPolicy {
        semantics: baseline.semantics_id(),
        decision: IntentDecision::Adopt {
            candidate: forged_candidate,
            input: forged_build.payload().fingerprint(),
        },
    };
    assert_eq!(
        IntentCoordinator::coordinate(
            &AgencyState::empty(),
            forged_build,
            IntentGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
            &forged_policy,
        ),
        Err(IntentCoordinationError::CandidateUnavailable {
            candidate: forged_candidate,
        })
    );
}

#[test]
fn intent_coordination_rejects_no_candidate_when_grounding_supplied_one() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let support = evidence(owner, 1, actor(0x11), item, entity(0x30), entity(0x40));
    let baseline = BaselineIntentPolicy::new();
    let build = intent_build(
        owner,
        item,
        entity(0x30),
        entity(0x40),
        support,
        baseline.semantics_id(),
    );
    let policy = FixedIntentPolicy {
        semantics: baseline.semantics_id(),
        decision: IntentDecision::NoCandidate {
            input: build.payload().fingerprint(),
        },
    };

    assert_eq!(
        IntentCoordinator::coordinate(
            &AgencyState::empty(),
            build,
            IntentGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
            &policy,
        ),
        Err(IntentCoordinationError::UnexpectedNoCandidate)
    );
}

#[test]
fn activity_coordination_opens_one_version_bound_opportunity_and_closes_exhaustion() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let record = evidence(owner, 1, actor(0x11), item, home, away);
    let epistemic = assimilated(owner, vec![record]);
    let intent = intent(owner, item, home);
    let agency = agency(vec![intent]);
    let controller = BaselineActivityController::new();
    let initial_snapshot = snapshot(epistemic.clone(), agency.clone());
    let initial_input = valid(ContainmentActivityProjector::new().initialization(
        &initial_snapshot,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));

    let initialized = valid(ActivityCoordinator::initialize(
        &agency,
        &initial_input,
        ActivityGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
        &controller,
    ));
    assert_eq!(initialized.input_fingerprint(), initial_input.fingerprint());
    let CoordinatedActivityInitialization::Start {
        activity,
        opportunity,
        ..
    } = initialized
    else {
        panic!("displacement must start an activity");
    };
    let activity = *activity;
    assert_eq!(
        activity.controller(),
        ActivityControllerId::from_bytes(controller.implementation_id())
    );
    assert_eq!(activity.state_schema(), activity_state_schema());
    assert_eq!(
        opportunity.sponsor(),
        ActionSponsor::activity(activity.id(), activity.version())
    );
    let initial_scope = opportunity
        .interaction_scope()
        .containment_scope()
        .unwrap_or_else(|| panic!("containment activity must open containment scope"));
    assert_eq!(initial_scope.source(), away);
    assert_eq!(initial_scope.destinations(), &[home]);
    assert_eq!(initial_scope.items(), &[item]);
    assert_eq!(opportunity.generation().get(), 1);

    let active_agency = valid(agency.start_activity(activity, true));
    let active_snapshot = snapshot(epistemic.clone(), active_agency.clone());
    let advance_input = ActivityAdvancementPayload::ContainmentTransfer(valid(
        ContainmentActivityProjector::new().advancement(
            &active_snapshot,
            activity.id(),
            ActivityEvaluationCause::AttemptedAction,
            controller.semantics_id(),
        ),
    ));
    let advanced = valid(ActivityCoordinator::advance(
        &active_agency,
        &advance_input,
        &[],
        &controller,
    ));
    assert_eq!(advanced.input_fingerprint(), advance_input.fingerprint());
    let CoordinatedActivityAdvancement::OpenAction {
        expected_version,
        successor,
        opportunity,
        ..
    } = advanced
    else {
        panic!("one bounded recovery step must remain");
    };
    assert_eq!(expected_version, activity.version());
    assert_eq!(
        successor
            .state()
            .containment_transfer()
            .unwrap_or_else(|| panic!("containment activity must retain containment state"))
            .remaining_attempts(),
        0
    );
    assert_eq!(
        opportunity.sponsor(),
        ActionSponsor::activity(successor.id(), successor.version())
    );
    assert_eq!(opportunity.generation().get(), 2);
    assert_eq!(
        opportunity
            .interaction_scope()
            .containment_scope()
            .unwrap_or_else(|| panic!("containment activity must open containment scope"))
            .items(),
        &[item]
    );

    let exhausted_agency = valid(active_agency.transition_activity(
        activity.id(),
        activity.version(),
        world_model::ActivityTransition::Continue(successor.state()),
    ));
    let exhausted_snapshot = snapshot(epistemic, exhausted_agency.clone());
    let exhausted_input = ActivityAdvancementPayload::ContainmentTransfer(valid(
        ContainmentActivityProjector::new().advancement(
            &exhausted_snapshot,
            successor.id(),
            ActivityEvaluationCause::ScheduledRecovery,
            controller.semantics_id(),
        ),
    ));
    let failed = valid(ActivityCoordinator::advance(
        &exhausted_agency,
        &exhausted_input,
        &[],
        &controller,
    ));
    let CoordinatedActivityAdvancement::Terminal {
        expected_activity_version,
        activity_successor,
        expected_intent_version,
        intent_successor,
        ..
    } = failed
    else {
        panic!("exhaustion must terminalize both activity and intent");
    };
    assert_eq!(
        expected_activity_version,
        exhausted_input.activity().version()
    );
    assert_eq!(activity_successor.status(), ActivityStatus::Failed);
    assert_eq!(expected_intent_version, intent.version());
    assert_eq!(intent_successor.status(), IntentStatus::Failed);
}

#[test]
fn activity_coordination_closes_satisfied_activity_and_intent_together() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let displaced = evidence(owner, 1, actor(0x11), item, home, away);
    let intent = intent(owner, item, home);
    let controller = BaselineActivityController::new();
    let base_agency = agency(vec![intent]);
    let initial_snapshot = snapshot(assimilated(owner, vec![displaced]), base_agency.clone());
    let initial_input = valid(ContainmentActivityProjector::new().initialization(
        &initial_snapshot,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    let initialized = valid(ActivityCoordinator::initialize(
        &base_agency,
        &initial_input,
        ActivityGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
        &controller,
    ));
    let CoordinatedActivityInitialization::Start { activity, .. } = initialized else {
        panic!("displacement must start an activity");
    };
    let activity = *activity;
    let active_agency = valid(base_agency.start_activity(activity, true));
    let restored = evidence(owner, 2, actor(0x11), item, away, home);
    let complete_snapshot = snapshot(
        assimilated(owner, vec![displaced, restored]),
        active_agency.clone(),
    );
    let complete_input = ActivityAdvancementPayload::ContainmentTransfer(valid(
        ContainmentActivityProjector::new().advancement(
            &complete_snapshot,
            activity.id(),
            ActivityEvaluationCause::AttemptedAction,
            controller.semantics_id(),
        ),
    ));

    let completed = valid(ActivityCoordinator::advance(
        &active_agency,
        &complete_input,
        &[],
        &controller,
    ));
    let CoordinatedActivityAdvancement::Terminal {
        expected_activity_version,
        activity_successor,
        expected_intent_version,
        intent_successor,
        ..
    } = completed
    else {
        panic!("satisfaction must terminalize both activity and intent");
    };
    assert_eq!(expected_activity_version, activity.version());
    assert_eq!(activity_successor.status(), ActivityStatus::Completed);
    assert_eq!(expected_intent_version, intent.version());
    assert_eq!(intent_successor.status(), IntentStatus::Achieved);
}

#[test]
fn activity_coordination_rejects_an_activity_owned_by_another_controller() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let record = evidence(owner, 1, actor(0x11), item, home, away);
    let epistemic = assimilated(owner, vec![record]);
    let intent = intent(owner, item, home);
    let base_agency = agency(vec![intent]);
    let controller = BaselineActivityController::new();
    let init_snapshot = snapshot(epistemic.clone(), base_agency.clone());
    let init_input = valid(ContainmentActivityProjector::new().initialization(
        &init_snapshot,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    let state = valid(controller.initialize(&init_input))
        .initial_state()
        .unwrap_or_else(|| panic!("displacement must create state"));
    let foreign = world_model::Activity::start(
        owner,
        intent.id(),
        ActivityGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
        ActivityControllerId::from_bytes([0xff; 32]),
        activity_state_schema(),
        state,
    );
    let accepted = valid(base_agency.start_activity(foreign, true));
    let accepted_snapshot = snapshot(epistemic, accepted.clone());
    let input = ActivityAdvancementPayload::ContainmentTransfer(valid(
        ContainmentActivityProjector::new().advancement(
            &accepted_snapshot,
            foreign.id(),
            ActivityEvaluationCause::ScheduledRecovery,
            controller.semantics_id(),
        ),
    ));

    assert_eq!(
        ActivityCoordinator::advance(&accepted, &input, &[], &controller),
        Err(ActivityCoordinationError::ControllerMismatch {
            expected: ActivityControllerId::from_bytes(controller.implementation_id()),
            actual: foreign.controller(),
        })
    );
}

#[test]
fn satisfied_activity_initialization_returns_a_checked_intent_successor() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let away = entity(0x40);
    let record = evidence(owner, 1, actor(0x11), item, away, home);
    let intent = intent(owner, item, home);
    let agency = agency(vec![intent]);
    let controller = BaselineActivityController::new();
    let accepted = snapshot(assimilated(owner, vec![record]), agency.clone());
    let input = valid(ContainmentActivityProjector::new().initialization(
        &accepted,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        controller.semantics_id(),
    ));
    let result = valid(ActivityCoordinator::initialize(
        &agency,
        &input,
        ActivityGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
        &controller,
    ));

    assert!(matches!(
        result,
        CoordinatedActivityInitialization::TransitionIntent {
            expected_version,
            successor,
            ..
        } if expected_version == intent.version() && successor.status() == IntentStatus::Achieved
    ));
}

#[test]
fn no_candidate_is_a_concrete_no_change_result() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let support = evidence(owner, 1, actor(0x11), item, entity(0x40), home);
    let baseline = BaselineIntentPolicy::new();
    let build = valid(ContainmentIntentProjector::new().build(
        &snapshot(EpistemicState::empty(), AgencyState::empty()),
        ContainmentAppraisal::new(owner, item, home, home, support.id()),
        baseline.semantics_id(),
    ));
    let input = build.payload().fingerprint();
    assert_eq!(
        valid(IntentCoordinator::coordinate(
            &AgencyState::empty(),
            build,
            IntentGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
            &baseline,
        )),
        CoordinatedIntentReview::NoChange { input }
    );
}
