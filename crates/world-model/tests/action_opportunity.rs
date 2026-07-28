use world_core::{ActorId, EntityId, SimDuration};
use world_model::{
    ActionEvaluationGeneration, ActionEvaluationInvocationId, ActionInteractionScope,
    ActionOpportunity, ActionOpportunityDisposition, ActionOpportunityGeneration,
    ActionOpportunityId, ActionOpportunityState, ActionOpportunityTransitionError,
    ActionOpportunityVersion, ActionSponsor, Activity, ActivityControllerId, ActivityGeneration,
    ActivityId, ActivityStateSchemaId, ActivityVersion, ActorReactionCause,
    ContainmentInteractionScope, ContainmentInteractionScopeError,
    ContainmentTransferActivityState, DirectedRoute, IntentId, RelocationInteraction,
    RelocationInteractionAnchor, RelocationInteractionScope, RelocationInteractionScopeError,
    TravelActivityState,
};

fn actor(byte: u8) -> ActorId {
    ActorId::from_bytes([byte; 32])
}

fn entity(byte: u8) -> EntityId {
    EntityId::from_bytes([byte; 32])
}

fn sponsor(byte: u8) -> ActionSponsor {
    ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([byte; 32]))
}

fn scope(destinations: Vec<EntityId>) -> ContainmentInteractionScope {
    ContainmentInteractionScope::new(entity(0x10), destinations, vec![entity(0x60)], 32)
        .unwrap_or_else(|error| panic!("interaction scope fixture must be valid: {error}"))
}

fn opportunity(generation: u64) -> ActionOpportunity {
    ActionOpportunity::open(
        actor(0x40),
        sponsor(0x50),
        ActionInteractionScope::containment(scope(vec![entity(0x30), entity(0x20)])),
        ActionOpportunityGeneration::new(generation),
    )
}

#[test]
fn containment_scope_is_bounded_unique_and_canonical() {
    let original = scope(vec![entity(0x30), entity(0x20)]);
    let reversed = scope(vec![entity(0x20), entity(0x30)]);

    assert_eq!(original, reversed);
    assert_eq!(original.source(), entity(0x10));
    assert_eq!(original.destinations(), &[entity(0x20), entity(0x30)]);
    assert_eq!(original.items(), &[entity(0x60)]);
    assert!(original.permits_item(entity(0x60)));
    assert!(!original.permits_item(entity(0x61)));
    assert_eq!(original.candidate_limit(), 32);

    assert_eq!(
        ContainmentInteractionScope::new(entity(0x10), Vec::new(), vec![entity(0x60)], 1),
        Err(ContainmentInteractionScopeError::EmptyDestinations)
    );
    assert_eq!(
        ContainmentInteractionScope::new(
            entity(0x10),
            vec![entity(0x20), entity(0x20)],
            vec![entity(0x60)],
            1,
        ),
        Err(ContainmentInteractionScopeError::DuplicateDestination {
            destination: entity(0x20),
        })
    );
    assert_eq!(
        ContainmentInteractionScope::new(
            entity(0x10),
            vec![entity(0x10), entity(0x20)],
            vec![entity(0x60)],
            1,
        ),
        Err(ContainmentInteractionScopeError::SourceIsDestination {
            container: entity(0x10),
        })
    );
    assert_eq!(
        ContainmentInteractionScope::new(entity(0x10), vec![entity(0x20)], vec![entity(0x60)], 0,),
        Err(ContainmentInteractionScopeError::ZeroCandidateLimit)
    );
    assert_eq!(
        ContainmentInteractionScope::new(entity(0x10), vec![entity(0x20)], Vec::new(), 1),
        Err(ContainmentInteractionScopeError::EmptyItems)
    );
    assert_eq!(
        ContainmentInteractionScope::new(
            entity(0x10),
            vec![entity(0x20)],
            vec![entity(0x60), entity(0x60)],
            1,
        ),
        Err(ContainmentInteractionScopeError::DuplicateItem { item: entity(0x60) })
    );
}

#[test]
fn relocation_scope_is_bounded_unique_and_canonical() {
    let source = entity(0x10);
    let destination_a = entity(0x20);
    let destination_b = entity(0x30);
    let route_a = DirectedRoute::new(source, destination_a, SimDuration::from_ticks(3))
        .unwrap_or_else(|error| panic!("route fixture must be valid: {error}"))
        .id();
    let route_b = DirectedRoute::new(source, destination_b, SimDuration::from_ticks(4))
        .unwrap_or_else(|error| panic!("route fixture must be valid: {error}"))
        .id();
    let scope = RelocationInteractionScope::new(
        vec![
            RelocationInteractionAnchor::new(
                RelocationInteraction::Resume(route_b),
                source,
                destination_b,
            ),
            RelocationInteractionAnchor::new(
                RelocationInteraction::Start(route_a),
                source,
                destination_a,
            ),
            RelocationInteractionAnchor::new(
                RelocationInteraction::Pause(route_b),
                source,
                destination_b,
            ),
        ],
        3,
    )
    .unwrap_or_else(|error| panic!("relocation scope fixture must be valid: {error}"));

    assert_eq!(
        scope.anchors(),
        &[
            RelocationInteractionAnchor::new(
                RelocationInteraction::Start(route_a),
                source,
                destination_a,
            ),
            RelocationInteractionAnchor::new(
                RelocationInteraction::Pause(route_b),
                source,
                destination_b,
            ),
            RelocationInteractionAnchor::new(
                RelocationInteraction::Resume(route_b),
                source,
                destination_b,
            ),
        ]
    );
    assert_eq!(scope.candidate_limit(), 3);
    assert!(scope.permits(RelocationInteraction::Start(route_a)));
    assert!(scope.permits(RelocationInteraction::Resume(route_b)));

    assert_eq!(
        RelocationInteractionScope::new(Vec::new(), 1),
        Err(RelocationInteractionScopeError::Empty)
    );
    assert_eq!(
        RelocationInteractionScope::new(
            vec![
                RelocationInteractionAnchor::new(
                    RelocationInteraction::Pause(route_a),
                    source,
                    destination_a,
                ),
                RelocationInteractionAnchor::new(
                    RelocationInteraction::Pause(route_a),
                    entity(0x11),
                    destination_a,
                ),
            ],
            1,
        ),
        Err(RelocationInteractionScopeError::DuplicateInteraction {
            interaction: RelocationInteraction::Pause(route_a),
        })
    );
    assert_eq!(
        RelocationInteractionScope::new(
            vec![RelocationInteractionAnchor::new(
                RelocationInteraction::Start(route_a),
                source,
                destination_a,
            )],
            0,
        ),
        Err(RelocationInteractionScopeError::ZeroCandidateLimit)
    );
}

#[test]
fn opportunity_identity_uses_actor_safe_sponsor_and_generation() {
    let original = opportunity(7);
    let equivalent = ActionOpportunity::open(
        actor(0x40),
        sponsor(0x50),
        ActionInteractionScope::containment(scope(vec![entity(0x20), entity(0x30)])),
        ActionOpportunityGeneration::new(7),
    );
    let changed_actor = ActionOpportunityId::derive(
        actor(0x41),
        sponsor(0x50),
        ActionOpportunityGeneration::new(7),
    );
    let changed_sponsor = ActionOpportunityId::derive(
        actor(0x40),
        sponsor(0x51),
        ActionOpportunityGeneration::new(7),
    );
    let changed_generation = ActionOpportunityId::derive(
        actor(0x40),
        sponsor(0x50),
        ActionOpportunityGeneration::new(8),
    );

    assert_eq!(original.id(), equivalent.id());
    assert_ne!(original.id(), changed_actor);
    assert_ne!(original.id(), changed_sponsor);
    assert_ne!(original.id(), changed_generation);
    assert_eq!(
        original.id().to_string(),
        "85599e23ef71be21a15d3ad021b8bf36c61af68fe45d8d96dffdb67eab303b35"
    );
}

#[test]
fn evaluation_invocation_identity_covers_only_actor_safe_inputs() {
    let opportunity = opportunity(7);
    let generation = ActionEvaluationGeneration::INITIAL;
    let original =
        ActionEvaluationInvocationId::derive(opportunity.id(), generation, [0x71; 32], [0x72; 32]);
    let equivalent =
        ActionEvaluationInvocationId::derive(opportunity.id(), generation, [0x71; 32], [0x72; 32]);

    assert_eq!(original, equivalent);
    assert_ne!(
        original,
        ActionEvaluationInvocationId::derive(
            ActionOpportunityId::from_bytes([0x70; 32]),
            generation,
            [0x71; 32],
            [0x72; 32],
        )
    );
    assert_ne!(
        original,
        ActionEvaluationInvocationId::derive(
            opportunity.id(),
            ActionEvaluationGeneration::new(2)
                .unwrap_or_else(|| panic!("two is a valid evaluation generation")),
            [0x71; 32],
            [0x72; 32],
        )
    );
    assert_ne!(
        original,
        ActionEvaluationInvocationId::derive(opportunity.id(), generation, [0x73; 32], [0x72; 32],)
    );
    assert_ne!(
        original,
        ActionEvaluationInvocationId::derive(opportunity.id(), generation, [0x71; 32], [0x74; 32],)
    );
    assert_eq!(
        original.to_string(),
        "601091e0f9c93efefa1a82e3550baaecd9c8059c8a0d3e04f5fed302e376243c"
    );
}

#[test]
fn opportunity_retains_actor_sponsor_interaction_generation_and_version() {
    let current = opportunity(7);
    let Some(scope) = current.interaction_scope().containment_scope() else {
        panic!("fixture must retain a containment interaction");
    };

    assert_eq!(current.actor(), actor(0x40));
    assert_eq!(current.sponsor(), sponsor(0x50));
    assert_eq!(scope.source(), entity(0x10));
    assert_eq!(scope.destinations(), &[entity(0x20), entity(0x30)]);
    assert_eq!(current.generation(), ActionOpportunityGeneration::new(7));
    assert_eq!(
        current.evaluation_generation(),
        ActionEvaluationGeneration::INITIAL
    );
    assert_eq!(current.version(), ActionOpportunityVersion::INITIAL);
    assert_eq!(current.state(), ActionOpportunityState::Open);
    assert_eq!(ActionOpportunityVersion::new(0), None);
    let maximum = ActionOpportunityVersion::new(u64::MAX)
        .unwrap_or_else(|| panic!("maximum u64 is a valid nonzero opportunity version"));
    assert_eq!(maximum.checked_next(), None);
}

#[test]
fn opportunity_consumption_is_versioned_and_terminal() {
    let open = opportunity(7);
    let consumed = open
        .consume(
            ActionOpportunityVersion::INITIAL,
            ActionOpportunityDisposition::ActionSubmitted,
        )
        .unwrap_or_else(|error| panic!("open opportunity must consume once: {error}"));

    assert_eq!(consumed.id(), open.id());
    assert_eq!(consumed.version().get(), 2);
    assert_eq!(
        consumed.state(),
        ActionOpportunityState::Consumed(ActionOpportunityDisposition::ActionSubmitted)
    );
    assert_eq!(
        open.consume(
            ActionOpportunityVersion::new(2).unwrap_or_else(|| panic!("version two must be valid")),
            ActionOpportunityDisposition::NoApplicableAction,
        ),
        Err(ActionOpportunityTransitionError::StaleVersion {
            expected: ActionOpportunityVersion::new(2)
                .unwrap_or_else(|| panic!("version two must be valid")),
            actual: ActionOpportunityVersion::INITIAL,
        })
    );
    assert_eq!(
        consumed.consume(
            consumed.version(),
            ActionOpportunityDisposition::NoApplicableAction,
        ),
        Err(ActionOpportunityTransitionError::AlreadyConsumed {
            disposition: ActionOpportunityDisposition::ActionSubmitted,
        })
    );

    assert_ne!(open.digest(), consumed.digest());
}

#[test]
fn evaluation_wait_resume_and_visible_reinvocation_are_checked_transitions() {
    let open = opportunity(7);
    let stable_id = open.id();
    let (waiting, invocation) = open
        .begin_evaluation(open.version(), [0x71; 32], [0x72; 32])
        .unwrap_or_else(|error| panic!("open opportunity must begin evaluation: {error}"));
    assert_eq!(waiting.id(), stable_id);
    assert_eq!(waiting.version().get(), 2);
    assert_eq!(
        waiting.evaluation_generation(),
        ActionEvaluationGeneration::INITIAL
    );
    assert_eq!(
        waiting.state(),
        ActionOpportunityState::WaitingForEvaluation(invocation)
    );
    assert_eq!(
        waiting.begin_evaluation(waiting.version(), [0x71; 32], [0x72; 32]),
        Err(ActionOpportunityTransitionError::EvaluationAlreadyWaiting { invocation })
    );
    assert_eq!(
        waiting.consume(
            waiting.version(),
            ActionOpportunityDisposition::ActionSubmitted,
        ),
        Err(ActionOpportunityTransitionError::EvaluationPending { invocation })
    );

    let wrong_invocation = ActionEvaluationInvocationId::from_bytes([0xff; 32]);
    assert_eq!(
        waiting.resume_evaluation(waiting.version(), wrong_invocation),
        Err(
            ActionOpportunityTransitionError::EvaluationInvocationMismatch {
                expected: wrong_invocation,
                actual: invocation,
            }
        )
    );

    let resumed = waiting
        .resume_evaluation(waiting.version(), invocation)
        .unwrap_or_else(|error| panic!("current result must reopen the opportunity: {error}"));
    assert_eq!(resumed.id(), stable_id);
    assert_eq!(resumed.version().get(), 3);
    assert_eq!(
        resumed.evaluation_generation(),
        ActionEvaluationGeneration::INITIAL
    );
    assert_eq!(resumed.state(), ActionOpportunityState::Open);
    assert_eq!(
        resumed.resume_evaluation(resumed.version(), invocation),
        Err(ActionOpportunityTransitionError::EvaluationNotWaiting)
    );
    let consumed = resumed
        .consume(
            resumed.version(),
            ActionOpportunityDisposition::ActionSubmitted,
        )
        .unwrap_or_else(|error| panic!("resumed opportunity must consume normally: {error}"));
    assert_eq!(consumed.id(), stable_id);
    assert_eq!(consumed.version().get(), 4);

    let reopened = waiting
        .reopen_for_visible_reinvocation(waiting.version(), invocation)
        .unwrap_or_else(|error| panic!("visible change must reopen for reinvocation: {error}"));
    let second_generation = ActionEvaluationGeneration::new(2)
        .unwrap_or_else(|| panic!("two is a valid evaluation generation"));
    assert_eq!(reopened.id(), stable_id);
    assert_eq!(reopened.version().get(), 3);
    assert_eq!(reopened.evaluation_generation(), second_generation);
    assert_eq!(reopened.state(), ActionOpportunityState::Open);
    assert_ne!(reopened.digest(), resumed.digest());

    let (second_waiting, second_invocation) = reopened
        .begin_evaluation(reopened.version(), [0x71; 32], [0x72; 32])
        .unwrap_or_else(|error| panic!("reopened opportunity must begin again: {error}"));
    assert_ne!(second_invocation, invocation);
    assert_eq!(second_waiting.id(), stable_id);
    assert_eq!(second_waiting.version().get(), 4);
    assert_eq!(
        second_waiting.state(),
        ActionOpportunityState::WaitingForEvaluation(second_invocation)
    );
}

#[test]
fn evaluation_transitions_reject_stale_versions_and_generation_overflow_is_checked() {
    let open = opportunity(7);
    let version_two =
        ActionOpportunityVersion::new(2).unwrap_or_else(|| panic!("two is a valid version"));
    assert_eq!(
        open.begin_evaluation(version_two, [0x71; 32], [0x72; 32]),
        Err(ActionOpportunityTransitionError::StaleVersion {
            expected: version_two,
            actual: ActionOpportunityVersion::INITIAL,
        })
    );

    let (waiting, invocation) = open
        .begin_evaluation(open.version(), [0x71; 32], [0x72; 32])
        .unwrap_or_else(|error| panic!("open opportunity must begin evaluation: {error}"));
    assert_eq!(
        waiting.resume_evaluation(ActionOpportunityVersion::INITIAL, invocation),
        Err(ActionOpportunityTransitionError::StaleVersion {
            expected: ActionOpportunityVersion::INITIAL,
            actual: version_two,
        })
    );
    assert_eq!(
        waiting.reopen_for_visible_reinvocation(ActionOpportunityVersion::INITIAL, invocation,),
        Err(ActionOpportunityTransitionError::StaleVersion {
            expected: ActionOpportunityVersion::INITIAL,
            actual: version_two,
        })
    );

    assert_eq!(ActionEvaluationGeneration::new(0), None);
    let maximum = ActionEvaluationGeneration::new(u64::MAX)
        .unwrap_or_else(|| panic!("maximum u64 is a valid evaluation generation"));
    assert_eq!(maximum.checked_next(), None);
}

#[test]
fn opportunity_digest_covers_complete_semantics() {
    let baseline = opportunity(7);
    let Some(baseline_scope) = baseline.interaction_scope().containment_scope() else {
        panic!("fixture must retain a containment interaction");
    };
    let changed_scope = ActionOpportunity::open(
        baseline.actor(),
        baseline.sponsor(),
        ActionInteractionScope::containment(
            ContainmentInteractionScope::new(
                baseline_scope.source(),
                vec![entity(0x21), entity(0x30)],
                baseline_scope.items().to_vec(),
                baseline_scope.candidate_limit() + 1,
            )
            .unwrap_or_else(|error| panic!("changed scope must be valid: {error}")),
        ),
        baseline.generation(),
    );

    assert_eq!(baseline.id(), changed_scope.id());
    assert_ne!(baseline.digest(), changed_scope.digest());
    assert_eq!(
        baseline.digest().to_string(),
        "0c1c3e61b56486d1b13609e9f73c06f8912c8b65dc9cd77620fd51b2be34e5e5"
    );

    let route = DirectedRoute::new(entity(0x10), entity(0x20), SimDuration::from_ticks(3))
        .unwrap_or_else(|error| panic!("route fixture must be valid: {error}"));
    let relocation = ActionOpportunity::open(
        baseline.actor(),
        baseline.sponsor(),
        ActionInteractionScope::relocation(
            RelocationInteractionScope::new(
                vec![RelocationInteractionAnchor::new(
                    RelocationInteraction::Start(route.id()),
                    route.source(),
                    route.destination(),
                )],
                1,
            )
            .unwrap_or_else(|error| panic!("relocation scope must be valid: {error}")),
        ),
        baseline.generation(),
    );
    assert_eq!(baseline.id(), relocation.id());
    assert_ne!(baseline.digest(), relocation.digest());
    assert!(relocation.interaction_scope().containment_scope().is_none());
    assert_eq!(
        relocation
            .interaction_scope()
            .relocation_scope()
            .map(RelocationInteractionScope::candidate_limit),
        Some(1)
    );
}

#[test]
fn activity_sponsor_binds_exact_activity_version() {
    let activity = ActivityId::from_bytes([0x70; 32]);
    let sponsor = ActionSponsor::activity(activity, ActivityVersion::INITIAL);
    let opportunity = ActionOpportunity::open(
        actor(0x40),
        sponsor,
        ActionInteractionScope::containment(scope(vec![entity(0x20)])),
        ActionOpportunityGeneration::new(1),
    );

    let ActionSponsor::Activity(binding) = opportunity.sponsor() else {
        panic!("activity sponsor must remain typed");
    };
    assert_eq!(binding.activity(), activity);
    assert_eq!(binding.expected_version(), ActivityVersion::INITIAL);
}

#[test]
fn retained_activity_state_identifies_its_exact_open_opportunity() {
    let acting = actor(0x40);
    let item = entity(0x60);
    let source = entity(0x10);
    let destination = entity(0x20);
    let containment_state = ContainmentTransferActivityState::new(
        item,
        source,
        destination,
        ActionOpportunityGeneration::new(1),
        2,
    )
    .and_then(ContainmentTransferActivityState::after_opening_opportunity)
    .unwrap_or_else(|error| panic!("containment state must retain one opening: {error}"));
    let containment_activity = Activity::start(
        acting,
        IntentId::from_bytes([0x71; 32]),
        ActivityGeneration::new(1).unwrap_or_else(|| panic!("one is nonzero")),
        ActivityControllerId::from_bytes([0x72; 32]),
        ActivityStateSchemaId::from_bytes([0x73; 32]),
        containment_state,
    );
    let containment = ActionOpportunity::open(
        acting,
        ActionSponsor::activity(containment_activity.id(), containment_activity.version()),
        ActionInteractionScope::containment(
            ContainmentInteractionScope::new(source, vec![destination], vec![item], 1)
                .unwrap_or_else(|error| panic!("containment scope must be valid: {error}")),
        ),
        ActionOpportunityGeneration::new(1),
    );
    assert!(containment.matches_activity_opening(containment_activity));

    let wrong_generation = ActionOpportunity::open(
        acting,
        containment.sponsor(),
        containment.interaction_scope().clone(),
        ActionOpportunityGeneration::new(2),
    );
    assert!(!wrong_generation.matches_activity_opening(containment_activity));
    let wrong_bound = ActionOpportunity::open(
        acting,
        containment.sponsor(),
        ActionInteractionScope::containment(
            ContainmentInteractionScope::new(source, vec![destination], vec![item], 2)
                .unwrap_or_else(|error| panic!("bounded scope must be valid: {error}")),
        ),
        containment.generation(),
    );
    assert!(!wrong_bound.matches_activity_opening(containment_activity));

    let route = DirectedRoute::new(source, destination, SimDuration::from_ticks(3))
        .unwrap_or_else(|error| panic!("route must be valid: {error}"));
    let travel_state = TravelActivityState::after_start_opened(
        source,
        destination,
        ActionOpportunityGeneration::new(2),
    )
    .unwrap_or_else(|error| panic!("travel state must retain its start opening: {error}"));
    let travel_activity = Activity::start(
        acting,
        IntentId::from_bytes([0x74; 32]),
        ActivityGeneration::new(2).unwrap_or_else(|| panic!("two is nonzero")),
        ActivityControllerId::from_bytes([0x72; 32]),
        ActivityStateSchemaId::from_bytes([0x73; 32]),
        travel_state,
    );
    let relocation_scope = |interaction, anchored_destination| {
        ActionInteractionScope::relocation(
            RelocationInteractionScope::new(
                vec![RelocationInteractionAnchor::new(
                    interaction,
                    source,
                    anchored_destination,
                )],
                1,
            )
            .unwrap_or_else(|error| panic!("relocation scope must be valid: {error}")),
        )
    };
    let start = ActionOpportunity::open(
        acting,
        ActionSponsor::activity(travel_activity.id(), travel_activity.version()),
        relocation_scope(RelocationInteraction::Start(route.id()), destination),
        ActionOpportunityGeneration::new(1),
    );
    assert!(start.matches_activity_opening(travel_activity));

    let wrong_verb = ActionOpportunity::open(
        acting,
        start.sponsor(),
        relocation_scope(RelocationInteraction::Pause(route.id()), destination),
        start.generation(),
    );
    assert!(!wrong_verb.matches_activity_opening(travel_activity));
    let wrong_endpoint = ActionOpportunity::open(
        acting,
        start.sponsor(),
        relocation_scope(RelocationInteraction::Start(route.id()), entity(0x21)),
        start.generation(),
    );
    assert!(!wrong_endpoint.matches_activity_opening(travel_activity));
}
