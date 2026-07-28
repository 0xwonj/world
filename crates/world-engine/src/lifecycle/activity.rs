use core::fmt;

use world_context::{
    ActivityAdvancementInputFingerprint, ActivityAdvancementPayload, ActivityControllerSemanticsId,
    ActivityInitializationInputFingerprint, ContainmentActivityInitializationPayload,
    RelocationActionVerb,
};
use world_decision::{
    ActivityActionDirective, ActivityAdvancementDecision, ActivityController,
    ActivityControllerError, ActivityInitializationDecision, ContainmentActionDirective,
    RelocationActionDirective, activity_state_schema,
};
use world_model::{
    ActionInteractionScope, ActionOpportunity, ActionSponsor, Activity, ActivityControllerId,
    ActivityGeneration, ActivityId, ActivityState, ActivityStateSchemaId, ActivityStatus,
    ActivityTransition, ActivityVersion, AgencyState, AgencyTransitionError,
    ContainmentTransferActivityState, DesiredCondition, Intent, IntentId, IntentStatus,
    IntentTransition, IntentVersion, RelocationInteraction, RelocationInteractionAnchor,
    RelocationInteractionScope, TravelActivityState, TravelActivityStep,
};

/// Why an activity-controller result could not become a checked agency operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ActivityCoordinationError {
    SemanticsMismatch {
        expected: ActivityControllerSemanticsId,
        actual: ActivityControllerSemanticsId,
    },
    Controller(ActivityControllerError),
    InitializationInputMismatch {
        expected: ActivityInitializationInputFingerprint,
        actual: ActivityInitializationInputFingerprint,
    },
    AdvancementInputMismatch {
        expected: ActivityAdvancementInputFingerprint,
        actual: ActivityAdvancementInputFingerprint,
    },
    AcceptedIntentMismatch {
        intent: IntentId,
    },
    AcceptedActivityMismatch {
        activity: ActivityId,
    },
    ControllerMismatch {
        expected: ActivityControllerId,
        actual: ActivityControllerId,
    },
    StateSchemaMismatch {
        expected: ActivityStateSchemaId,
        actual: ActivityStateSchemaId,
    },
    InvalidInitializationDecision,
    InvalidAdvancementDecision,
    ActivityStateMismatch,
    DirectiveScopeMismatch,
    AttemptedActionMismatch,
    DirectiveGenerationMismatch {
        expected: u64,
        actual: u64,
    },
    Agency(AgencyTransitionError),
}

impl fmt::Display for ActivityCoordinationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticsMismatch { expected, actual } => write!(
                formatter,
                "activity input semantics {actual} do not match resolved controller semantics {expected}"
            ),
            Self::Controller(error) => error.fmt(formatter),
            Self::InitializationInputMismatch { expected, actual } => write!(
                formatter,
                "activity initialization result input {actual} does not match prepared input {expected}"
            ),
            Self::AdvancementInputMismatch { expected, actual } => write!(
                formatter,
                "activity advancement result input {actual} does not match prepared input {expected}"
            ),
            Self::AcceptedIntentMismatch { intent } => write!(
                formatter,
                "prepared intent {intent} does not match accepted agency state"
            ),
            Self::AcceptedActivityMismatch { activity } => write!(
                formatter,
                "prepared activity {activity} does not match accepted agency state"
            ),
            Self::ControllerMismatch { expected, actual } => write!(
                formatter,
                "activity controller {actual:?} does not match resolved controller {expected:?}"
            ),
            Self::StateSchemaMismatch { expected, actual } => write!(
                formatter,
                "activity state schema {actual:?} does not match containment schema {expected:?}"
            ),
            Self::InvalidInitializationDecision => {
                formatter.write_str("activity initialization result contradicts its input")
            }
            Self::InvalidAdvancementDecision => {
                formatter.write_str("activity advancement result contradicts its result variant")
            }
            Self::ActivityStateMismatch => formatter.write_str(
                "activity controller state does not preserve the prepared intent and belief anchors",
            ),
            Self::DirectiveScopeMismatch => formatter.write_str(
                "activity action directive does not name the exact controller-state anchors",
            ),
            Self::AttemptedActionMismatch => formatter.write_str(
                "activity action directive cannot be bound to one exact attempted interaction",
            ),
            Self::DirectiveGenerationMismatch { expected, actual } => write!(
                formatter,
                "activity action generation {actual} does not precede retained generation {expected}"
            ),
            Self::Agency(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ActivityCoordinationError {}

/// Engine-private checked result of one activity initialization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CoordinatedActivityInitialization {
    Start {
        input: ActivityInitializationInputFingerprint,
        activity: Box<Activity>,
        opportunity: ActionOpportunity,
    },
    TransitionIntent {
        input: ActivityInitializationInputFingerprint,
        expected_version: IntentVersion,
        successor: Intent,
    },
}

impl CoordinatedActivityInitialization {
    #[cfg(test)]
    pub(crate) const fn input_fingerprint(&self) -> ActivityInitializationInputFingerprint {
        match self {
            Self::Start { input, .. } | Self::TransitionIntent { input, .. } => *input,
        }
    }
}

/// Engine-private checked result of one activity advancement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CoordinatedActivityAdvancement {
    OpenAction {
        input: ActivityAdvancementInputFingerprint,
        expected_version: ActivityVersion,
        successor: Box<Activity>,
        opportunity: ActionOpportunity,
    },
    Transition {
        input: ActivityAdvancementInputFingerprint,
        expected_version: ActivityVersion,
        successor: Box<Activity>,
    },
    Terminal {
        input: ActivityAdvancementInputFingerprint,
        expected_activity_version: ActivityVersion,
        activity_successor: Box<Activity>,
        expected_intent_version: IntentVersion,
        intent_successor: Intent,
    },
    NoChange {
        input: ActivityAdvancementInputFingerprint,
        activity: ActivityId,
        expected_version: ActivityVersion,
    },
}

impl CoordinatedActivityAdvancement {
    #[cfg(test)]
    pub(crate) const fn input_fingerprint(&self) -> ActivityAdvancementInputFingerprint {
        match self {
            Self::OpenAction { input, .. }
            | Self::Transition { input, .. }
            | Self::Terminal { input, .. }
            | Self::NoChange { input, .. } => *input,
        }
    }
}

/// Coordinates concrete activity initialization and advancement results.
pub(crate) struct ActivityCoordinator;

impl ActivityCoordinator {
    pub(crate) fn initialize(
        current: &AgencyState,
        input: &ContainmentActivityInitializationPayload,
        generation: ActivityGeneration,
        controller: &dyn ActivityController,
    ) -> Result<CoordinatedActivityInitialization, ActivityCoordinationError> {
        validate_semantics(input.controller_semantics(), controller)?;
        validate_intent(current, input.intent())?;

        let decision = controller
            .initialize(input)
            .map_err(ActivityCoordinationError::Controller)?;
        if decision.input_fingerprint() != input.fingerprint() {
            return Err(ActivityCoordinationError::InitializationInputMismatch {
                expected: input.fingerprint(),
                actual: decision.input_fingerprint(),
            });
        }

        match decision {
            ActivityInitializationDecision::Start { start, .. } => {
                let state = start.state();
                validate_initial_state(input, state)?;
                validate_containment_directive(start.directive(), state)?;

                let intent = input.intent();
                let activity = Activity::start(
                    intent.actor(),
                    intent.id(),
                    generation,
                    ActivityControllerId::from_bytes(controller.implementation_id()),
                    activity_state_schema(),
                    state,
                );
                current
                    .start_activity(activity, true)
                    .map_err(ActivityCoordinationError::Agency)?;
                let opportunity = ActionOpportunity::open(
                    activity.actor(),
                    ActionSponsor::activity(activity.id(), activity.version()),
                    ActionInteractionScope::containment(start.directive().scope().clone()),
                    start.directive().generation(),
                );
                Ok(CoordinatedActivityInitialization::Start {
                    input: input.fingerprint(),
                    activity: Box::new(activity),
                    opportunity,
                })
            }
            ActivityInitializationDecision::AlreadySatisfied { transition, .. } => {
                if transition != IntentTransition::Achieve || !initialization_is_satisfied(input) {
                    return Err(ActivityCoordinationError::InvalidInitializationDecision);
                }
                coordinate_intent_transition(current, input, transition)
            }
            ActivityInitializationDecision::SuspendIntent { transition, .. } => {
                if transition != IntentTransition::Suspend
                    || input.current_belief().is_some()
                    || input.intent().status() != IntentStatus::Active
                {
                    return Err(ActivityCoordinationError::InvalidInitializationDecision);
                }
                coordinate_intent_transition(current, input, transition)
            }
        }
    }

    pub(crate) fn advance(
        current: &AgencyState,
        input: &ActivityAdvancementPayload,
        attempted: &[ActionOpportunity],
        controller: &dyn ActivityController,
    ) -> Result<CoordinatedActivityAdvancement, ActivityCoordinationError> {
        validate_semantics(input.controller_semantics(), controller)?;
        validate_intent(current, input.intent())?;
        validate_activity(current, input.activity(), controller)?;

        let decision = controller
            .advance(input)
            .map_err(ActivityCoordinationError::Controller)?;
        if decision.input_fingerprint() != input.fingerprint() {
            return Err(ActivityCoordinationError::AdvancementInputMismatch {
                expected: input.fingerprint(),
                actual: decision.input_fingerprint(),
            });
        }

        match decision {
            ActivityAdvancementDecision::OpenAction {
                transition,
                directive,
                ..
            } => {
                if !matches!(
                    (input.activity().status(), transition),
                    (ActivityStatus::Active, ActivityTransition::Continue(_))
                        | (
                            ActivityStatus::Waiting | ActivityStatus::Suspended,
                            ActivityTransition::Resume(_)
                        )
                ) {
                    return Err(ActivityCoordinationError::InvalidAdvancementDecision);
                }
                let successor = coordinate_activity_transition(current, input, transition)?;
                let interaction_scope =
                    coordinate_directive(input, &directive, successor.state(), attempted)?;
                let opportunity = ActionOpportunity::open(
                    successor.actor(),
                    ActionSponsor::activity(successor.id(), successor.version()),
                    interaction_scope,
                    directive_generation(&directive),
                );
                Ok(CoordinatedActivityAdvancement::OpenAction {
                    input: input.fingerprint(),
                    expected_version: input.activity().version(),
                    successor: Box::new(successor),
                    opportunity,
                })
            }
            ActivityAdvancementDecision::Complete {
                transition,
                intent_transition,
                ..
            } => {
                if transition != ActivityTransition::Complete
                    || intent_transition != IntentTransition::Achieve
                {
                    return Err(ActivityCoordinationError::InvalidAdvancementDecision);
                }
                coordinate_terminal_activity(current, input, transition, intent_transition)
            }
            ActivityAdvancementDecision::Fail {
                transition,
                intent_transition,
                ..
            } => {
                if transition != ActivityTransition::Fail
                    || intent_transition != IntentTransition::Fail
                {
                    return Err(ActivityCoordinationError::InvalidAdvancementDecision);
                }
                coordinate_terminal_activity(current, input, transition, intent_transition)
            }
            ActivityAdvancementDecision::Await { transition, .. } => match transition {
                Some(ActivityTransition::Wait(state))
                    if input.activity().status() == ActivityStatus::Active
                        && state == input.activity().state()
                        && advancement_can_await(input) =>
                {
                    let successor = coordinate_activity_transition(
                        current,
                        input,
                        ActivityTransition::Wait(state),
                    )?;
                    Ok(CoordinatedActivityAdvancement::Transition {
                        input: input.fingerprint(),
                        expected_version: input.activity().version(),
                        successor: Box::new(successor),
                    })
                }
                None if matches!(
                    input.activity().status(),
                    ActivityStatus::Waiting | ActivityStatus::Suspended
                ) && advancement_can_await(input) =>
                {
                    Ok(CoordinatedActivityAdvancement::NoChange {
                        input: input.fingerprint(),
                        activity: input.activity().id(),
                        expected_version: input.activity().version(),
                    })
                }
                Some(_) | None => Err(ActivityCoordinationError::InvalidAdvancementDecision),
            },
        }
    }
}

fn validate_semantics(
    actual: ActivityControllerSemanticsId,
    controller: &dyn ActivityController,
) -> Result<(), ActivityCoordinationError> {
    let expected = controller.semantics_id();
    if actual == expected {
        Ok(())
    } else {
        Err(ActivityCoordinationError::SemanticsMismatch { expected, actual })
    }
}

fn validate_intent(current: &AgencyState, intent: Intent) -> Result<(), ActivityCoordinationError> {
    if current.intent(intent.id()).copied() == Some(intent) {
        Ok(())
    } else {
        Err(ActivityCoordinationError::AcceptedIntentMismatch {
            intent: intent.id(),
        })
    }
}

fn validate_activity(
    current: &AgencyState,
    activity: Activity,
    controller: &dyn ActivityController,
) -> Result<(), ActivityCoordinationError> {
    if current.activity(activity.id()).copied() != Some(activity) {
        return Err(ActivityCoordinationError::AcceptedActivityMismatch {
            activity: activity.id(),
        });
    }
    let expected_controller = ActivityControllerId::from_bytes(controller.implementation_id());
    if activity.controller() != expected_controller {
        return Err(ActivityCoordinationError::ControllerMismatch {
            expected: expected_controller,
            actual: activity.controller(),
        });
    }
    let expected_schema = activity_state_schema();
    if activity.state_schema() != expected_schema {
        return Err(ActivityCoordinationError::StateSchemaMismatch {
            expected: expected_schema,
            actual: activity.state_schema(),
        });
    }
    Ok(())
}

fn validate_initial_state(
    input: &ContainmentActivityInitializationPayload,
    state: ContainmentTransferActivityState,
) -> Result<(), ActivityCoordinationError> {
    let Some(belief) = input.current_belief() else {
        return Err(ActivityCoordinationError::InvalidInitializationDecision);
    };
    let DesiredCondition::ItemContainedIn {
        item,
        container: destination,
    } = input.intent().desired()
    else {
        return Err(ActivityCoordinationError::ActivityStateMismatch);
    };
    if belief.actor() != input.intent().actor()
        || belief.item() != item
        || belief.container() == destination
        || state.item() != item
        || state.source() != belief.container()
        || state.destination() != destination
    {
        return Err(ActivityCoordinationError::ActivityStateMismatch);
    }
    Ok(())
}

fn initialization_is_satisfied(input: &ContainmentActivityInitializationPayload) -> bool {
    let DesiredCondition::ItemContainedIn { item, container } = input.intent().desired() else {
        return false;
    };
    input.current_belief().is_some_and(|belief| {
        belief.actor() == input.intent().actor()
            && belief.item() == item
            && belief.container() == container
    })
}

fn validate_containment_directive(
    directive: &ContainmentActionDirective,
    retained_state: ContainmentTransferActivityState,
) -> Result<(), ActivityCoordinationError> {
    let scope = directive.scope();
    if scope.source() != retained_state.source()
        || scope.destinations() != [retained_state.destination()]
        || scope.items() != [retained_state.item()]
    {
        return Err(ActivityCoordinationError::DirectiveScopeMismatch);
    }
    let expected = retained_state.next_opportunity_generation().get();
    let actual = directive.generation().get();
    if actual.checked_add(1) != Some(expected) {
        return Err(ActivityCoordinationError::DirectiveGenerationMismatch { expected, actual });
    }
    Ok(())
}

fn validate_relocation_directive(
    directive: RelocationActionDirective,
    retained_state: TravelActivityState,
) -> Result<(), ActivityCoordinationError> {
    let expected_verb = match retained_state.step() {
        TravelActivityStep::Resume => RelocationActionVerb::Pause,
        TravelActivityStep::AwaitArrival => RelocationActionVerb::Resume,
        TravelActivityStep::Pause => {
            return Err(ActivityCoordinationError::DirectiveScopeMismatch);
        }
    };
    if directive.verb() != expected_verb
        || directive.source() != retained_state.source()
        || directive.destination() != retained_state.destination()
    {
        return Err(ActivityCoordinationError::DirectiveScopeMismatch);
    }
    let expected = retained_state.next_opportunity_generation().get();
    let actual = directive.generation().get();
    if actual.checked_add(1) != Some(expected) {
        return Err(ActivityCoordinationError::DirectiveGenerationMismatch { expected, actual });
    }
    Ok(())
}

fn coordinate_directive(
    input: &ActivityAdvancementPayload,
    directive: &ActivityActionDirective,
    retained_state: ActivityState,
    attempted: &[ActionOpportunity],
) -> Result<ActionInteractionScope, ActivityCoordinationError> {
    match (directive, retained_state) {
        (
            ActivityActionDirective::ContainmentTransfer(directive),
            ActivityState::ContainmentTransfer(state),
        ) => {
            validate_containment_directive(directive, state)?;
            Ok(ActionInteractionScope::containment(
                directive.scope().clone(),
            ))
        }
        (ActivityActionDirective::Relocation(directive), ActivityState::Travel(state)) => {
            validate_relocation_directive(*directive, state)?;
            coordinate_relocation_scope(input, *directive, attempted)
        }
        _ => Err(ActivityCoordinationError::DirectiveScopeMismatch),
    }
}

fn coordinate_relocation_scope(
    input: &ActivityAdvancementPayload,
    directive: RelocationActionDirective,
    attempted: &[ActionOpportunity],
) -> Result<ActionInteractionScope, ActivityCoordinationError> {
    let activity = input.activity();
    let mut matching = attempted.iter().filter(|opportunity| {
        opportunity.actor() == activity.actor()
            && matches!(
                opportunity.sponsor(),
                ActionSponsor::Activity(sponsor)
                    if sponsor.activity() == activity.id()
                        && sponsor.expected_version() == activity.version()
            )
    });
    let attempted = matching
        .next()
        .ok_or(ActivityCoordinationError::AttemptedActionMismatch)?;
    if matching.next().is_some() {
        return Err(ActivityCoordinationError::AttemptedActionMismatch);
    }
    let scope = attempted
        .interaction_scope()
        .relocation_scope()
        .ok_or(ActivityCoordinationError::AttemptedActionMismatch)?;
    let [anchor] = scope.anchors() else {
        return Err(ActivityCoordinationError::AttemptedActionMismatch);
    };
    if anchor.source() != directive.source() || anchor.destination() != directive.destination() {
        return Err(ActivityCoordinationError::AttemptedActionMismatch);
    }
    let expected_predecessor = match directive.verb() {
        RelocationActionVerb::Pause => RelocationInteraction::Start(anchor.interaction().route()),
        RelocationActionVerb::Resume => RelocationInteraction::Pause(anchor.interaction().route()),
        RelocationActionVerb::Start => {
            return Err(ActivityCoordinationError::DirectiveScopeMismatch);
        }
    };
    if anchor.interaction() != expected_predecessor {
        return Err(ActivityCoordinationError::AttemptedActionMismatch);
    }
    let interaction = match directive.verb() {
        RelocationActionVerb::Pause => RelocationInteraction::Pause(anchor.interaction().route()),
        RelocationActionVerb::Resume => RelocationInteraction::Resume(anchor.interaction().route()),
        RelocationActionVerb::Start => {
            return Err(ActivityCoordinationError::DirectiveScopeMismatch);
        }
    };
    let scope = RelocationInteractionScope::new(
        vec![RelocationInteractionAnchor::new(
            interaction,
            directive.source(),
            directive.destination(),
        )],
        1,
    )
    .map_err(|_| ActivityCoordinationError::DirectiveScopeMismatch)?;
    Ok(ActionInteractionScope::relocation(scope))
}

fn directive_generation(
    directive: &ActivityActionDirective,
) -> world_model::ActionOpportunityGeneration {
    match directive {
        ActivityActionDirective::ContainmentTransfer(directive) => directive.generation(),
        ActivityActionDirective::Relocation(directive) => directive.generation(),
    }
}

fn advancement_can_await(input: &ActivityAdvancementPayload) -> bool {
    match input {
        ActivityAdvancementPayload::ContainmentTransfer(input) => input.current_belief().is_none(),
        ActivityAdvancementPayload::Travel(input) => {
            input.state().step() == TravelActivityStep::AwaitArrival
        }
    }
}

fn coordinate_intent_transition(
    current: &AgencyState,
    input: &ContainmentActivityInitializationPayload,
    transition: IntentTransition,
) -> Result<CoordinatedActivityInitialization, ActivityCoordinationError> {
    let intent = input.intent();
    let successor = intent
        .transition(intent.version(), transition)
        .map_err(|error| ActivityCoordinationError::Agency(AgencyTransitionError::Intent(error)))?;
    let agency = current
        .transition_intent(intent.id(), intent.version(), transition)
        .map_err(ActivityCoordinationError::Agency)?;
    if agency.intent(intent.id()).copied() != Some(successor) {
        return Err(ActivityCoordinationError::AcceptedIntentMismatch {
            intent: intent.id(),
        });
    }
    Ok(CoordinatedActivityInitialization::TransitionIntent {
        input: input.fingerprint(),
        expected_version: intent.version(),
        successor,
    })
}

fn coordinate_activity_transition(
    current: &AgencyState,
    input: &ActivityAdvancementPayload,
    transition: ActivityTransition,
) -> Result<Activity, ActivityCoordinationError> {
    let activity = input.activity();
    let successor = activity
        .transition(activity.version(), transition)
        .map_err(|error| {
            ActivityCoordinationError::Agency(AgencyTransitionError::Activity(error))
        })?;
    let agency = current
        .transition_activity(activity.id(), activity.version(), transition)
        .map_err(ActivityCoordinationError::Agency)?;
    if agency.activity(activity.id()).copied() != Some(successor) {
        return Err(ActivityCoordinationError::AcceptedActivityMismatch {
            activity: activity.id(),
        });
    }
    Ok(successor)
}

fn coordinate_terminal_activity(
    current: &AgencyState,
    input: &ActivityAdvancementPayload,
    activity_transition: ActivityTransition,
    intent_transition: IntentTransition,
) -> Result<CoordinatedActivityAdvancement, ActivityCoordinationError> {
    if !matches!(
        (activity_transition, intent_transition),
        (ActivityTransition::Complete, IntentTransition::Achieve)
            | (ActivityTransition::Fail, IntentTransition::Fail)
    ) {
        return Err(ActivityCoordinationError::InvalidAdvancementDecision);
    }

    let activity = input.activity();
    let intent = input.intent();
    if activity.intent() != intent.id() || activity.actor() != intent.actor() {
        return Err(ActivityCoordinationError::InvalidAdvancementDecision);
    }

    let activity_successor = activity
        .transition(activity.version(), activity_transition)
        .map_err(|error| {
            ActivityCoordinationError::Agency(AgencyTransitionError::Activity(error))
        })?;
    let intent_successor = intent
        .transition(intent.version(), intent_transition)
        .map_err(|error| ActivityCoordinationError::Agency(AgencyTransitionError::Intent(error)))?;
    let agency = current
        .transition_activity(activity.id(), activity.version(), activity_transition)
        .map_err(ActivityCoordinationError::Agency)?
        .transition_intent(intent.id(), intent.version(), intent_transition)
        .map_err(ActivityCoordinationError::Agency)?;
    if agency.activity(activity.id()).copied() != Some(activity_successor) {
        return Err(ActivityCoordinationError::AcceptedActivityMismatch {
            activity: activity.id(),
        });
    }
    if agency.intent(intent.id()).copied() != Some(intent_successor) {
        return Err(ActivityCoordinationError::AcceptedIntentMismatch {
            intent: intent.id(),
        });
    }

    Ok(CoordinatedActivityAdvancement::Terminal {
        input: input.fingerprint(),
        expected_activity_version: activity.version(),
        activity_successor: Box::new(activity_successor),
        expected_intent_version: intent.version(),
        intent_successor,
    })
}
