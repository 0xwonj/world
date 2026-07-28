use world_core::{ActorId, EntityId};
use world_model::{
    ActionOpportunityGeneration, Activity, ActivityControllerId, ActivityGeneration,
    ActivityStateSchemaId, ActivityStatus, ActivityTransition, ActivityVersion, AgencyState,
    AgencyStateError, AgencyTransitionError, ContainmentTransferActivityState, DesiredCondition,
    Intent, IntentGeneration, IntentStatus, IntentTransition, IntentVersion,
};

fn actor(byte: u8) -> ActorId {
    ActorId::from_bytes([byte; 32])
}

fn entity(byte: u8) -> EntityId {
    EntityId::from_bytes([byte; 32])
}

fn intent_generation(value: u64) -> IntentGeneration {
    IntentGeneration::new(value).unwrap_or_else(|| panic!("intent generation must be nonzero"))
}

fn activity_generation(value: u64) -> ActivityGeneration {
    ActivityGeneration::new(value).unwrap_or_else(|| panic!("activity generation must be nonzero"))
}

fn fixture() -> (Intent, Activity) {
    let actor = actor(0x40);
    let intent = Intent::adopt(
        actor,
        intent_generation(1),
        DesiredCondition::item_contained_in(entity(0x50), entity(0x20)),
    );
    let activity = Activity::start(
        actor,
        intent.id(),
        activity_generation(1),
        ActivityControllerId::from_bytes([0x70; 32]),
        ActivityStateSchemaId::from_bytes([0x71; 32]),
        ContainmentTransferActivityState::new(
            entity(0x50),
            entity(0x10),
            entity(0x20),
            ActionOpportunityGeneration::new(1),
            2,
        )
        .unwrap_or_else(|error| panic!("activity state must be valid: {error}")),
    );
    (intent, activity)
}

#[test]
fn intent_and_activity_successors_are_versioned_state_machines() {
    let (intent, activity) = fixture();
    let suspended = intent
        .transition(IntentVersion::INITIAL, IntentTransition::Suspend)
        .unwrap_or_else(|error| panic!("active intent must suspend: {error}"));
    let resumed = suspended
        .transition(suspended.version(), IntentTransition::Resume)
        .unwrap_or_else(|error| panic!("suspended intent must resume: {error}"));
    assert_eq!(resumed.status(), IntentStatus::Active);
    assert_eq!(resumed.version().get(), 3);

    let waiting = activity
        .transition(
            ActivityVersion::INITIAL,
            ActivityTransition::Wait(activity.state()),
        )
        .unwrap_or_else(|error| panic!("active activity must wait: {error}"));
    let resumed_activity = waiting
        .transition(
            waiting.version(),
            ActivityTransition::Resume(waiting.state()),
        )
        .unwrap_or_else(|error| panic!("waiting activity must resume: {error}"));
    let completed = resumed_activity
        .transition(resumed_activity.version(), ActivityTransition::Complete)
        .unwrap_or_else(|error| panic!("active activity must complete: {error}"));
    assert_eq!(completed.status(), ActivityStatus::Completed);
    assert_eq!(completed.version().get(), 4);
}

#[test]
fn agency_successors_preserve_ownership_and_focus_integrity() {
    let (intent, activity) = fixture();
    let with_intent = AgencyState::empty()
        .adopt_intent(intent)
        .unwrap_or_else(|error| panic!("intent adoption must succeed: {error}"));
    let active = with_intent
        .start_activity(activity, true)
        .unwrap_or_else(|error| panic!("activity start must succeed: {error}"));
    assert_eq!(active.focused_activity(actor(0x40)), Some(activity.id()));

    let waiting = active
        .transition_activity(
            activity.id(),
            ActivityVersion::INITIAL,
            ActivityTransition::Wait(activity.state()),
        )
        .unwrap_or_else(|error| panic!("waiting transition must succeed: {error}"));
    assert_eq!(waiting.focused_activity(actor(0x40)), None);

    let waiting_activity = *waiting
        .activity(activity.id())
        .unwrap_or_else(|| panic!("activity must remain retained"));
    let resumed = waiting
        .transition_activity(
            activity.id(),
            waiting_activity.version(),
            ActivityTransition::Resume(waiting_activity.state()),
        )
        .unwrap_or_else(|error| panic!("resume must succeed: {error}"));
    let focused = resumed
        .set_focus(actor(0x40), None, Some(activity.id()))
        .unwrap_or_else(|error| panic!("active activity may take focus: {error}"));
    assert_eq!(focused.focused_activity(actor(0x40)), Some(activity.id()));
}

#[test]
fn activity_binds_controller_schema_and_bounded_opportunity_progress() {
    let (_, activity) = fixture();
    let state = activity
        .state()
        .containment_transfer()
        .unwrap_or_else(|| panic!("fixture activity must use containment state"));
    assert_eq!(
        activity.controller(),
        ActivityControllerId::from_bytes([0x70; 32])
    );
    assert_eq!(
        activity.state_schema(),
        ActivityStateSchemaId::from_bytes([0x71; 32])
    );
    assert_eq!(
        state.next_opportunity_generation(),
        ActionOpportunityGeneration::new(1)
    );
    assert_eq!(state.remaining_attempts(), 2);

    let after_open = state
        .after_opening_opportunity()
        .unwrap_or_else(|error| panic!("bounded attempt must advance: {error}"));
    let waiting = activity
        .transition(
            ActivityVersion::INITIAL,
            ActivityTransition::Wait(after_open.into()),
        )
        .unwrap_or_else(|error| panic!("coupled opportunity progress must be accepted: {error}"));
    assert_eq!(
        waiting
            .state()
            .containment_transfer()
            .unwrap_or_else(|| panic!("waiting activity must use containment state"))
            .next_opportunity_generation(),
        ActionOpportunityGeneration::new(2)
    );
    assert_eq!(
        waiting
            .state()
            .containment_transfer()
            .unwrap_or_else(|| panic!("waiting activity must use containment state"))
            .remaining_attempts(),
        1
    );

    let illegally_replenished = ContainmentTransferActivityState::new(
        state.item(),
        state.source(),
        state.destination(),
        state.next_opportunity_generation(),
        3,
    )
    .unwrap_or_else(|error| panic!("structural state may be compared by transition: {error}"));
    assert!(
        activity
            .transition(
                ActivityVersion::INITIAL,
                ActivityTransition::Continue(illegally_replenished.into()),
            )
            .is_err()
    );
}

#[test]
fn live_activity_prevents_terminal_intent_until_activity_terminates() {
    let (intent, activity) = fixture();
    let state = AgencyState::empty()
        .adopt_intent(intent)
        .and_then(|state| state.start_activity(activity, true))
        .unwrap_or_else(|error| panic!("agency fixture must be valid: {error}"));

    assert!(matches!(
        state.transition_intent(
            intent.id(),
            IntentVersion::INITIAL,
            IntentTransition::Achieve,
        ),
        Err(AgencyTransitionError::InvalidSuccessor(
            AgencyStateError::LiveActivityHasTerminalIntent { .. }
        ))
    ));

    let completed = state
        .transition_activity(
            activity.id(),
            ActivityVersion::INITIAL,
            ActivityTransition::Complete,
        )
        .unwrap_or_else(|error| panic!("activity completion must succeed: {error}"));
    let achieved = completed
        .transition_intent(
            intent.id(),
            IntentVersion::INITIAL,
            IntentTransition::Achieve,
        )
        .unwrap_or_else(|error| panic!("intent may terminate after its activity: {error}"));
    assert_eq!(
        achieved
            .intent(intent.id())
            .unwrap_or_else(|| panic!("intent remains retained"))
            .status(),
        IntentStatus::Achieved
    );
    assert_eq!(achieved.focused_activity(actor(0x40)), None);
}

#[test]
fn activity_must_implement_its_owning_intent_condition() {
    let (intent, _) = fixture();
    let mismatched = Activity::start(
        actor(0x40),
        intent.id(),
        activity_generation(1),
        ActivityControllerId::from_bytes([0x70; 32]),
        ActivityStateSchemaId::from_bytes([0x71; 32]),
        ContainmentTransferActivityState::new(
            entity(0x51),
            entity(0x10),
            entity(0x20),
            ActionOpportunityGeneration::new(1),
            2,
        )
        .unwrap_or_else(|error| panic!("activity state must be structurally valid: {error}")),
    );

    assert_eq!(
        AgencyState::new(vec![intent], vec![mismatched], Vec::new()),
        Err(AgencyStateError::ActivityDesiredConditionMismatch {
            activity: mismatched.id(),
        })
    );
}

#[test]
fn owner_local_generations_and_canonical_order_are_enforced() {
    let first = Intent::adopt(
        actor(0x40),
        intent_generation(1),
        DesiredCondition::item_contained_in(entity(0x50), entity(0x20)),
    );
    let reused = Intent::adopt(
        actor(0x40),
        intent_generation(1),
        DesiredCondition::item_contained_in(entity(0x51), entity(0x20)),
    );
    assert_eq!(
        AgencyState::new(vec![first, reused], Vec::new(), Vec::new()),
        Err(AgencyStateError::DuplicateIntentGeneration {
            actor: actor(0x40),
            generation: intent_generation(1),
        })
    );

    let second = Intent::adopt(
        actor(0x41),
        intent_generation(1),
        DesiredCondition::item_contained_in(entity(0x51), entity(0x20)),
    );
    let canonical = AgencyState::new(vec![first, second], Vec::new(), Vec::new())
        .unwrap_or_else(|error| panic!("distinct actors must be valid: {error}"));
    let reversed = AgencyState::new(vec![second, first], Vec::new(), Vec::new())
        .unwrap_or_else(|error| panic!("input order must not matter: {error}"));
    assert_eq!(canonical, reversed);
    assert_eq!(canonical.digest(), reversed.digest());
}
