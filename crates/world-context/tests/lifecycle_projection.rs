use core::fmt::Debug;

use world_context::{
    ActivityControllerSemanticsId, ActivityEvaluationCause, AppraisalEvaluatorSemanticsId,
    ContainmentActivityProjector, ContainmentAppraisalProjector, ContainmentAppraisalSubject,
    ContainmentIntentProjector, EvidenceAssimilationPayload, EvidenceAssimilationPayloadError,
    EvidenceAssimilationSemanticsId, GroundedIntentCandidateId, IntentPolicySemanticsId,
};
use world_core::{ActorId, EntityId, WorldRevision};
use world_model::{
    AcceptedState, ActionOpportunityGeneration, Activity, ActivityControllerId, ActivityGeneration,
    ActivityStateSchemaId, AgencyState, ContainerRecord, ContainmentAppraisal, ContainmentRecord,
    ContainmentTransferActivityState, ContainmentTransferDelta, DesiredCondition, DomainState,
    EpistemicState, EpistemicVersion, EvidenceDeliveryGeneration, EvidenceRecord, Intent,
    IntentGeneration, IntentTransition, PhysicalEvent, SocialState, WorldSnapshot,
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
        panic!("item transfer constructor must produce its typed event")
    };
    EvidenceRecord::direct_item_transfer(
        observer,
        EvidenceDeliveryGeneration::new(generation)
            .unwrap_or_else(|| panic!("test generation is nonzero")),
        event,
    )
}

fn epistemic(observer: ActorId, records: Vec<EvidenceRecord>) -> EpistemicState {
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

fn empty_domain() -> DomainState {
    valid(DomainState::new(Vec::new(), Vec::new(), Vec::new()))
}

fn domain_with_item(item: EntityId, container: EntityId) -> DomainState {
    valid(DomainState::new(
        vec![ContainerRecord::new(container, 4)],
        vec![ContainmentRecord::new(item, container)],
        Vec::new(),
    ))
}

fn snapshot(
    revision: WorldRevision,
    domain: DomainState,
    epistemic: EpistemicState,
    agency: AgencyState,
) -> WorldSnapshot {
    WorldSnapshot::new(
        revision,
        AcceptedState::new(domain, epistemic, SocialState::empty(), agency),
    )
}

fn intent_fixture(
    owner: ActorId,
    generation: u64,
    item: EntityId,
    destination: EntityId,
) -> Intent {
    Intent::adopt(
        owner,
        IntentGeneration::new(generation).unwrap_or_else(|| panic!("test generation is nonzero")),
        DesiredCondition::item_contained_in(item, destination),
    )
}

fn activity_fixture(
    intent: Intent,
    generation: u64,
    source: EntityId,
    remaining_attempts: u32,
) -> Activity {
    let DesiredCondition::ItemContainedIn {
        item,
        container: destination,
    } = intent.desired()
    else {
        panic!("fixture intent must be containment-shaped");
    };
    let state = valid(ContainmentTransferActivityState::new(
        item,
        source,
        destination,
        ActionOpportunityGeneration::new(1),
        remaining_attempts,
    ));
    Activity::start(
        intent.actor(),
        intent.id(),
        ActivityGeneration::new(generation).unwrap_or_else(|| panic!("test generation is nonzero")),
        ActivityControllerId::from_bytes([0xa1; 32]),
        ActivityStateSchemaId::from_bytes([0xa2; 32]),
        state,
    )
}

fn agency_fixture(intents: Vec<Intent>, activities: Vec<Activity>) -> AgencyState {
    valid(AgencyState::new(intents, activities, Vec::new()))
}

fn displaced_appraisal(
    owner: ActorId,
    item: EntityId,
    current: EntityId,
    restore: EntityId,
    support: EvidenceRecord,
) -> ContainmentAppraisal {
    ContainmentAppraisal::new(owner, item, current, restore, support.id())
}

#[test]
fn evidence_assimilation_is_nonempty_actor_bound_and_canonical() {
    let observer = actor(0x10);
    let item = entity(0x20);
    let destination = entity(0x40);
    let first = evidence(observer, 1, actor(0x11), item, entity(0x30), destination);
    let second = evidence(observer, 2, actor(0x11), item, entity(0x31), destination);
    let semantics = EvidenceAssimilationSemanticsId::from_bytes([0x51; 32]);
    let payload = valid(EvidenceAssimilationPayload::new(
        observer,
        EpistemicVersion::new(7),
        vec![second, first],
        semantics,
    ));

    assert_eq!(payload.actor(), observer);
    assert_eq!(payload.expected_version(), EpistemicVersion::new(7));
    assert_eq!(payload.evidence(), &[first, second]);
    assert_eq!(payload.semantics(), semantics);

    assert_eq!(
        EvidenceAssimilationPayload::new(observer, EpistemicVersion::EMPTY, Vec::new(), semantics,),
        Err(EvidenceAssimilationPayloadError::EmptyEvidence)
    );
    let foreign = evidence(actor(0x12), 1, actor(0x11), item, entity(0x30), destination);
    assert_eq!(
        EvidenceAssimilationPayload::new(
            observer,
            EpistemicVersion::EMPTY,
            vec![foreign],
            semantics,
        ),
        Err(EvidenceAssimilationPayloadError::WrongObserver {
            evidence: foreign.id(),
        })
    );
}

#[test]
fn evidence_assimilation_fingerprint_covers_every_input_class() {
    let observer = actor(0x10);
    let item = entity(0x20);
    let baseline_record = evidence(observer, 1, actor(0x11), item, entity(0x30), entity(0x40));
    let baseline = valid(EvidenceAssimilationPayload::new(
        observer,
        EpistemicVersion::new(2),
        vec![baseline_record],
        EvidenceAssimilationSemanticsId::from_bytes([0x51; 32]),
    ));

    let other_actor = actor(0x12);
    let actor_variant = valid(EvidenceAssimilationPayload::new(
        other_actor,
        EpistemicVersion::new(2),
        vec![evidence(
            other_actor,
            1,
            actor(0x11),
            item,
            entity(0x30),
            entity(0x40),
        )],
        EvidenceAssimilationSemanticsId::from_bytes([0x51; 32]),
    ));
    let version_variant = valid(EvidenceAssimilationPayload::new(
        observer,
        EpistemicVersion::new(3),
        vec![baseline_record],
        EvidenceAssimilationSemanticsId::from_bytes([0x51; 32]),
    ));
    let evidence_variant = valid(EvidenceAssimilationPayload::new(
        observer,
        EpistemicVersion::new(2),
        vec![evidence(
            observer,
            1,
            actor(0x11),
            item,
            entity(0x31),
            entity(0x40),
        )],
        EvidenceAssimilationSemanticsId::from_bytes([0x51; 32]),
    ));
    let semantics_variant = valid(EvidenceAssimilationPayload::new(
        observer,
        EpistemicVersion::new(2),
        vec![baseline_record],
        EvidenceAssimilationSemanticsId::from_bytes([0x52; 32]),
    ));

    for variant in [
        actor_variant,
        version_variant,
        evidence_variant,
        semantics_variant,
    ] {
        assert_ne!(baseline.fingerprint(), variant.fingerprint());
    }
}

#[test]
fn appraisal_uses_latest_belief_support_and_ignores_hidden_domain_truth() {
    let observer = actor(0x10);
    let item = entity(0x20);
    let believed_container = entity(0x40);
    let first = evidence(
        observer,
        1,
        actor(0x11),
        item,
        entity(0x30),
        believed_container,
    );
    let latest = evidence(
        observer,
        2,
        actor(0x11),
        item,
        entity(0x31),
        believed_container,
    );
    let epistemic = epistemic(observer, vec![latest, first]);
    let other_observer = actor(0x12);
    let other_evidence = evidence(
        other_observer,
        1,
        actor(0x13),
        entity(0x21),
        entity(0x32),
        entity(0x42),
    );
    let epistemic_with_other_actor = valid(epistemic.clone().assimilate(
        other_observer,
        EpistemicVersion::EMPTY,
        vec![other_evidence],
    ));
    let semantics = AppraisalEvaluatorSemanticsId::from_bytes([0x61; 32]);
    let first_snapshot = snapshot(
        WorldRevision::from_raw(3),
        domain_with_item(item, entity(0x50)),
        epistemic.clone(),
        AgencyState::empty(),
    );
    let second_snapshot = snapshot(
        WorldRevision::from_raw(900),
        domain_with_item(item, entity(0x51)),
        epistemic_with_other_actor,
        AgencyState::empty(),
    );

    let first_payload = valid(ContainmentAppraisalProjector::new().build(
        &first_snapshot,
        observer,
        item,
        None,
        semantics,
    ));
    let second_payload = valid(ContainmentAppraisalProjector::new().build(
        &second_snapshot,
        observer,
        item,
        None,
        semantics,
    ));

    assert_eq!(first_payload, second_payload);
    let ContainmentAppraisalSubject::Present {
        belief,
        supporting_evidence,
    } = first_payload.subject()
    else {
        panic!("a retained containment belief must project a present subject")
    };
    assert_eq!(belief.container(), believed_container);
    assert_eq!(*supporting_evidence, latest);
    assert_ne!(
        belief.container(),
        entity(0x50),
        "hidden authoritative containment must not replace actor belief"
    );
}

#[test]
fn appraisal_projects_exact_absence_without_disclosing_an_actual_container() {
    let observer = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let expected = entity(0x40);
    let hidden_actual = entity(0x50);
    let transfer = evidence(observer, 1, actor(0x11), item, home, expected);
    let absent = absence(observer, 2, item, expected);
    let previous = displaced_appraisal(observer, item, expected, home, transfer);
    let semantics = AppraisalEvaluatorSemanticsId::from_bytes([0x61; 32]);
    let projected = valid(ContainmentAppraisalProjector::new().build(
        &snapshot(
            WorldRevision::ROOT,
            domain_with_item(item, hidden_actual),
            epistemic(observer, vec![transfer, absent]),
            AgencyState::empty(),
        ),
        observer,
        item,
        Some(previous),
        semantics,
    ));

    assert_eq!(projected.previous(), Some(previous));
    let ContainmentAppraisalSubject::Absent {
        item: projected_item,
        expected_container,
        supporting_evidence,
    } = projected.subject()
    else {
        panic!("matching absence evidence must project an absent subject")
    };
    assert_eq!(*projected_item, item);
    assert_eq!(*expected_container, expected);
    assert_eq!(*supporting_evidence, absent);
    assert_ne!(*expected_container, hidden_actual);
}

#[test]
fn appraisal_retains_a_belief_when_absence_names_another_container() {
    let observer = actor(0x10);
    let item = entity(0x20);
    let home = entity(0x30);
    let believed = entity(0x40);
    let other = entity(0x41);
    let transfer = evidence(observer, 1, actor(0x11), item, home, believed);
    let unrelated_absence = absence(observer, 2, item, other);
    let projected = valid(ContainmentAppraisalProjector::new().build(
        &snapshot(
            WorldRevision::ROOT,
            empty_domain(),
            epistemic(observer, vec![transfer, unrelated_absence]),
            AgencyState::empty(),
        ),
        observer,
        item,
        None,
        AppraisalEvaluatorSemanticsId::from_bytes([0x61; 32]),
    ));

    let ContainmentAppraisalSubject::Present { belief, .. } = projected.subject() else {
        panic!("absence from another container must not retract the current belief")
    };
    assert_eq!(belief.container(), believed);
}

#[test]
fn appraisal_fingerprint_covers_belief_evidence_history_and_port_semantics() {
    let observer = actor(0x10);
    let item = entity(0x20);
    let first = evidence(observer, 1, actor(0x11), item, entity(0x30), entity(0x40));
    let baseline_snapshot = snapshot(
        WorldRevision::ROOT,
        empty_domain(),
        epistemic(observer, vec![first]),
        AgencyState::empty(),
    );
    let semantics = AppraisalEvaluatorSemanticsId::from_bytes([0x61; 32]);
    let baseline = valid(ContainmentAppraisalProjector::new().build(
        &baseline_snapshot,
        observer,
        item,
        None,
        semantics,
    ));

    let changed_record = evidence(observer, 1, actor(0x11), item, entity(0x31), entity(0x41));
    let changed_snapshot = snapshot(
        WorldRevision::ROOT,
        empty_domain(),
        epistemic(observer, vec![changed_record]),
        AgencyState::empty(),
    );
    let belief_variant = valid(ContainmentAppraisalProjector::new().build(
        &changed_snapshot,
        observer,
        item,
        None,
        semantics,
    ));
    let previous = displaced_appraisal(observer, item, entity(0x40), entity(0x30), first);
    let previous_variant = valid(ContainmentAppraisalProjector::new().build(
        &baseline_snapshot,
        observer,
        item,
        Some(previous),
        semantics,
    ));
    let semantics_variant = valid(ContainmentAppraisalProjector::new().build(
        &baseline_snapshot,
        observer,
        item,
        None,
        AppraisalEvaluatorSemanticsId::from_bytes([0x62; 32]),
    ));

    for variant in [belief_variant, previous_variant, semantics_variant] {
        assert_ne!(baseline.fingerprint(), variant.fingerprint());
    }
}

#[test]
fn displaced_appraisal_supplies_one_resolvable_intent_candidate() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let current = entity(0x40);
    let restore = entity(0x30);
    let support = evidence(owner, 1, actor(0x11), item, restore, current);
    let appraisal = displaced_appraisal(owner, item, current, restore, support);
    let build = valid(ContainmentIntentProjector::new().build(
        &snapshot(
            WorldRevision::ROOT,
            empty_domain(),
            EpistemicState::empty(),
            AgencyState::empty(),
        ),
        appraisal,
        IntentPolicySemanticsId::from_bytes([0x72; 32]),
    ));
    let candidates = build.payload().candidates();

    assert_eq!(candidates.candidates().len(), 1);
    let candidate = candidates.candidates()[0];
    assert!(candidates.contains(candidate.id()));
    assert_eq!(candidate.actor(), owner);
    assert_eq!(
        candidate.desired(),
        DesiredCondition::item_contained_in(item, restore)
    );
    assert_eq!(
        candidate.supporting_appraisal(),
        appraisal.material_fingerprint()
    );
    assert_eq!(candidate.supporting_evidence(), support.id());

    let resolved = build
        .resolution()
        .resolve(candidate.id())
        .unwrap_or_else(|| panic!("supplied candidate must resolve"));
    assert_eq!(resolved.candidate(), candidate.id());
    assert_eq!(resolved.actor(), owner);
    assert_eq!(resolved.desired(), candidate.desired());

    let fabricated = GroundedIntentCandidateId::from_bytes([0xff; 32]);
    assert!(!candidates.contains(fabricated));
    assert_eq!(build.resolution().resolve(fabricated), None);
}

#[test]
fn live_intent_and_non_displacement_suppress_intent_candidates() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let current = entity(0x40);
    let restore = entity(0x30);
    let support = evidence(owner, 1, actor(0x11), item, restore, current);
    let displaced = displaced_appraisal(owner, item, current, restore, support);
    let live = intent_fixture(owner, 1, item, restore);
    let live_build = valid(ContainmentIntentProjector::new().build(
        &snapshot(
            WorldRevision::ROOT,
            empty_domain(),
            EpistemicState::empty(),
            agency_fixture(vec![live], Vec::new()),
        ),
        displaced,
        IntentPolicySemanticsId::from_bytes([0x72; 32]),
    ));
    assert!(live_build.payload().candidates().candidates().is_empty());
    assert!(live_build.resolution().is_empty());

    let terminal = valid(live.transition(live.version(), IntentTransition::Achieve));
    let terminal_build = valid(ContainmentIntentProjector::new().build(
        &snapshot(
            WorldRevision::ROOT,
            empty_domain(),
            EpistemicState::empty(),
            agency_fixture(vec![terminal], Vec::new()),
        ),
        displaced,
        IntentPolicySemanticsId::from_bytes([0x72; 32]),
    ));
    assert_eq!(terminal_build.payload().candidates().candidates().len(), 1);

    let satisfied = ContainmentAppraisal::new(owner, item, restore, restore, support.id());
    let satisfied_build = valid(ContainmentIntentProjector::new().build(
        &snapshot(
            WorldRevision::ROOT,
            empty_domain(),
            EpistemicState::empty(),
            AgencyState::empty(),
        ),
        satisfied,
        IntentPolicySemanticsId::from_bytes([0x72; 32]),
    ));
    assert!(
        satisfied_build
            .payload()
            .candidates()
            .candidates()
            .is_empty()
    );
}

#[test]
fn intent_projection_is_hidden_truth_independent_and_semantics_sensitive() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let current = entity(0x40);
    let restore = entity(0x30);
    let support = evidence(owner, 1, actor(0x11), item, restore, current);
    let appraisal = displaced_appraisal(owner, item, current, restore, support);
    let first_snapshot = snapshot(
        WorldRevision::from_raw(2),
        domain_with_item(item, current),
        EpistemicState::empty(),
        AgencyState::empty(),
    );
    let second_snapshot = snapshot(
        WorldRevision::from_raw(700),
        domain_with_item(item, restore),
        EpistemicState::empty(),
        agency_fixture(
            vec![intent_fixture(actor(0x12), 1, entity(0x21), entity(0x31))],
            Vec::new(),
        ),
    );
    let policy = IntentPolicySemanticsId::from_bytes([0x72; 32]);

    let first = valid(ContainmentIntentProjector::new().build(&first_snapshot, appraisal, policy));
    let second =
        valid(ContainmentIntentProjector::new().build(&second_snapshot, appraisal, policy));
    assert_eq!(first, second);

    let policy_variant = valid(ContainmentIntentProjector::new().build(
        &first_snapshot,
        appraisal,
        IntentPolicySemanticsId::from_bytes([0x73; 32]),
    ));
    assert_eq!(
        first.payload().candidates(),
        policy_variant.payload().candidates()
    );
    assert_ne!(
        first.payload().fingerprint(),
        policy_variant.payload().fingerprint()
    );

    assert_eq!(
        first.payload().candidates().grounding_semantics(),
        ContainmentIntentProjector::new().semantics_id()
    );
}

#[test]
fn activity_payloads_use_only_semantic_agency_belief_and_actor_safe_cause() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let current = entity(0x40);
    let destination = entity(0x30);
    let support = evidence(owner, 1, actor(0x11), item, destination, current);
    let epistemic = epistemic(owner, vec![support]);
    let intent = intent_fixture(owner, 1, item, destination);
    let activity = activity_fixture(intent, 1, current, 2);
    let agency = agency_fixture(vec![intent], vec![activity]);
    let other_owner = actor(0x12);
    let other_evidence = evidence(
        other_owner,
        1,
        actor(0x13),
        entity(0x21),
        entity(0x31),
        entity(0x41),
    );
    let epistemic_with_other_actor = valid(epistemic.clone().assimilate(
        other_owner,
        EpistemicVersion::EMPTY,
        vec![other_evidence],
    ));
    let agency_with_other_actor = agency_fixture(
        vec![
            intent,
            intent_fixture(other_owner, 1, entity(0x21), entity(0x31)),
        ],
        vec![activity],
    );
    let appraisal =
        displaced_appraisal(owner, item, current, destination, support).material_fingerprint();
    let cause = ActivityEvaluationCause::AppraisalChanged { appraisal };
    let semantics = ActivityControllerSemanticsId::from_bytes([0x81; 32]);
    let first_snapshot = snapshot(
        WorldRevision::from_raw(5),
        domain_with_item(item, entity(0x50)),
        epistemic.clone(),
        agency.clone(),
    );
    let second_snapshot = snapshot(
        WorldRevision::from_raw(900),
        domain_with_item(item, entity(0x51)),
        epistemic_with_other_actor,
        agency_with_other_actor,
    );

    let first_initialization = valid(ContainmentActivityProjector::new().initialization(
        &first_snapshot,
        intent.id(),
        cause,
        semantics,
    ));
    let second_initialization = valid(ContainmentActivityProjector::new().initialization(
        &second_snapshot,
        intent.id(),
        cause,
        semantics,
    ));
    assert_eq!(first_initialization, second_initialization);
    assert_eq!(first_initialization.intent(), intent);
    assert_eq!(
        first_initialization
            .current_belief()
            .map(|belief| belief.container()),
        Some(current)
    );
    assert_eq!(first_initialization.cause(), cause);
    assert_eq!(first_initialization.controller_semantics(), semantics);

    let first_advancement = valid(ContainmentActivityProjector::new().advancement(
        &first_snapshot,
        activity.id(),
        ActivityEvaluationCause::AttemptedAction,
        semantics,
    ));
    let second_advancement = valid(ContainmentActivityProjector::new().advancement(
        &second_snapshot,
        activity.id(),
        ActivityEvaluationCause::AttemptedAction,
        semantics,
    ));
    assert_eq!(first_advancement, second_advancement);
    assert_eq!(first_advancement.intent(), intent);
    assert_eq!(first_advancement.activity(), activity);
    assert_eq!(
        first_advancement
            .current_belief()
            .map(|belief| belief.container()),
        Some(current)
    );
}

#[test]
fn missing_actor_belief_is_valid_activity_input() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let source = entity(0x40);
    let destination = entity(0x30);
    let intent = intent_fixture(owner, 1, item, destination);
    let activity = activity_fixture(intent, 1, source, 2);
    let snapshot = snapshot(
        WorldRevision::ROOT,
        empty_domain(),
        EpistemicState::empty(),
        agency_fixture(vec![intent], vec![activity]),
    );
    let semantics = ActivityControllerSemanticsId::from_bytes([0x81; 32]);

    let initialization = valid(ContainmentActivityProjector::new().initialization(
        &snapshot,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        semantics,
    ));
    let advancement = valid(ContainmentActivityProjector::new().advancement(
        &snapshot,
        activity.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        semantics,
    ));

    assert_eq!(initialization.current_belief(), None);
    assert_eq!(advancement.current_belief(), None);
}

#[test]
fn activity_fingerprints_cover_cause_semantics_belief_and_persistent_state() {
    let owner = actor(0x10);
    let item = entity(0x20);
    let current = entity(0x40);
    let destination = entity(0x30);
    let support = evidence(owner, 1, actor(0x11), item, destination, current);
    let intent = intent_fixture(owner, 1, item, destination);
    let activity = activity_fixture(intent, 1, current, 2);
    let with_belief = snapshot(
        WorldRevision::ROOT,
        empty_domain(),
        epistemic(owner, vec![support]),
        agency_fixture(vec![intent], vec![activity]),
    );
    let without_belief = snapshot(
        WorldRevision::ROOT,
        empty_domain(),
        EpistemicState::empty(),
        agency_fixture(vec![intent], vec![activity]),
    );
    let changed_activity = activity_fixture(intent, 1, entity(0x41), 1);
    let changed_activity_snapshot = snapshot(
        WorldRevision::ROOT,
        empty_domain(),
        epistemic(owner, vec![support]),
        agency_fixture(vec![intent], vec![changed_activity]),
    );
    let semantics = ActivityControllerSemanticsId::from_bytes([0x81; 32]);

    let baseline_initialization = valid(ContainmentActivityProjector::new().initialization(
        &with_belief,
        intent.id(),
        ActivityEvaluationCause::AttemptedAction,
        semantics,
    ));
    let missing_belief_initialization = valid(ContainmentActivityProjector::new().initialization(
        &without_belief,
        intent.id(),
        ActivityEvaluationCause::AttemptedAction,
        semantics,
    ));
    let cause_initialization = valid(ContainmentActivityProjector::new().initialization(
        &with_belief,
        intent.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        semantics,
    ));
    let semantics_initialization = valid(ContainmentActivityProjector::new().initialization(
        &with_belief,
        intent.id(),
        ActivityEvaluationCause::AttemptedAction,
        ActivityControllerSemanticsId::from_bytes([0x82; 32]),
    ));
    for fingerprint in [
        missing_belief_initialization.fingerprint(),
        cause_initialization.fingerprint(),
        semantics_initialization.fingerprint(),
    ] {
        assert_ne!(baseline_initialization.fingerprint(), fingerprint);
    }

    let baseline_advancement = valid(ContainmentActivityProjector::new().advancement(
        &with_belief,
        activity.id(),
        ActivityEvaluationCause::AttemptedAction,
        semantics,
    ));
    let changed_state_advancement = valid(ContainmentActivityProjector::new().advancement(
        &changed_activity_snapshot,
        changed_activity.id(),
        ActivityEvaluationCause::AttemptedAction,
        semantics,
    ));
    let cause_advancement = valid(ContainmentActivityProjector::new().advancement(
        &with_belief,
        activity.id(),
        ActivityEvaluationCause::ScheduledRecovery,
        semantics,
    ));
    let semantics_advancement = valid(ContainmentActivityProjector::new().advancement(
        &with_belief,
        activity.id(),
        ActivityEvaluationCause::AttemptedAction,
        ActivityControllerSemanticsId::from_bytes([0x82; 32]),
    ));
    for fingerprint in [
        changed_state_advancement.fingerprint(),
        cause_advancement.fingerprint(),
        semantics_advancement.fingerprint(),
    ] {
        assert_ne!(baseline_advancement.fingerprint(), fingerprint);
    }
}
