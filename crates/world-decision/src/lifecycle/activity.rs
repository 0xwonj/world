use core::fmt;

use world_context::{
    ActivityAdvancementInputFingerprint, ActivityAdvancementPayload, ActivityControllerSemanticsId,
    ActivityEvaluationCause, ActivityInitializationInputFingerprint,
    ContainmentActivityAdvancementPayload, ContainmentActivityInitializationPayload,
    RelocationActionVerb, TravelActivityAdvancementPayload,
};
use world_core::{CanonicalDomain, EntityId};
use world_model::{
    ActionOpportunityGeneration, ActivityControllerId, ActivityStateSchemaId, ActivityStatus,
    ActivityTransition, ActivityTransitionError, ContainmentInteractionScope,
    ContainmentInteractionScopeError, ContainmentTransferActivityState,
    ContainmentTransferActivityStateError, DesiredCondition, IntentTransition,
    IntentTransitionError, TravelActivityState, TravelActivityStateError, TravelActivityStep,
};

use super::{ActivityController, identity};

const ACTIVITY_CONTROLLER_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("baseline-activity-controller-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("baseline activity-controller domain must be valid"),
    };
const ACTIVITY_STATE_SCHEMA_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("baseline-activity-state-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("activity-state schema domain must be valid"),
    };
const INITIAL_OPPORTUNITY_GENERATION: ActionOpportunityGeneration =
    ActionOpportunityGeneration::new(1);
const TOTAL_ACTION_ATTEMPTS: u32 = 2;
const ACTION_CANDIDATE_LIMIT: u32 = 1;

/// Returns the closed canonical schema of accepted baseline activity state.
#[must_use]
pub fn activity_state_schema() -> ActivityStateSchemaId {
    ActivityStateSchemaId::from_bytes(identity(ACTIVITY_STATE_SCHEMA_DOMAIN))
}

/// One checked actor-safe request to open a containment action opportunity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainmentActionDirective {
    generation: ActionOpportunityGeneration,
    scope: ContainmentInteractionScope,
}

/// One checked actor-safe request to open a relocation-control opportunity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelocationActionDirective {
    generation: ActionOpportunityGeneration,
    verb: RelocationActionVerb,
    source: EntityId,
    destination: EntityId,
}

impl RelocationActionDirective {
    #[must_use]
    pub const fn generation(self) -> ActionOpportunityGeneration {
        self.generation
    }

    #[must_use]
    pub const fn verb(self) -> RelocationActionVerb {
        self.verb
    }

    #[must_use]
    pub const fn source(self) -> EntityId {
        self.source
    }

    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.destination
    }
}

/// Closed action-opening directive of the implemented activity methods.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivityActionDirective {
    ContainmentTransfer(ContainmentActionDirective),
    Relocation(RelocationActionDirective),
}

impl ContainmentActionDirective {
    /// Returns the activity-local opportunity generation.
    #[must_use]
    pub const fn generation(&self) -> ActionOpportunityGeneration {
        self.generation
    }

    /// Returns exact interaction anchors derived from actor-relative state.
    #[must_use]
    pub const fn scope(&self) -> &ContainmentInteractionScope {
        &self.scope
    }
}

/// State and first action directive for one newly started activity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityInitializationStart {
    state: ContainmentTransferActivityState,
    directive: ContainmentActionDirective,
}

impl ActivityInitializationStart {
    /// Returns persistent controller state after opening the first action.
    #[must_use]
    pub const fn state(&self) -> ContainmentTransferActivityState {
        self.state
    }

    /// Returns the first action-opening directive.
    #[must_use]
    pub const fn directive(&self) -> &ContainmentActionDirective {
        &self.directive
    }
}

/// Closed deterministic result of containment activity initialization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivityInitializationDecision {
    /// Create the activity with the returned state and atomically open its
    /// first action opportunity.
    Start {
        /// Exact actor-safe input that produced this result.
        input: ActivityInitializationInputFingerprint,
        /// Paired persistent state and first action directive.
        start: Box<ActivityInitializationStart>,
    },
    /// The accepted belief already satisfies the owning intent.
    AlreadySatisfied {
        /// Exact actor-safe input that produced this result.
        input: ActivityInitializationInputFingerprint,
        /// Checked transition that marks the satisfied intent achieved.
        transition: IntentTransition,
    },
    /// No current actor belief supplies a source anchor, so the baseline
    /// suspends the owning intent rather than creating an implicit retry.
    SuspendIntent {
        /// Exact actor-safe input that produced this result.
        input: ActivityInitializationInputFingerprint,
        /// Checked transition that suspends the owning intent.
        transition: IntentTransition,
    },
}

impl ActivityInitializationDecision {
    /// Returns the exact actor-safe input fingerprint.
    #[must_use]
    pub const fn input_fingerprint(&self) -> ActivityInitializationInputFingerprint {
        match self {
            Self::Start { input, .. }
            | Self::AlreadySatisfied { input, .. }
            | Self::SuspendIntent { input, .. } => *input,
        }
    }

    /// Returns the state to persist for a started activity, if any.
    #[must_use]
    pub const fn initial_state(&self) -> Option<ContainmentTransferActivityState> {
        match self {
            Self::Start { start, .. } => Some(start.state),
            Self::AlreadySatisfied { .. } | Self::SuspendIntent { .. } => None,
        }
    }

    /// Returns the first action-opening directive, if initialization starts.
    #[must_use]
    pub const fn directive(&self) -> Option<&ContainmentActionDirective> {
        match self {
            Self::Start { start, .. } => Some(&start.directive),
            Self::AlreadySatisfied { .. } | Self::SuspendIntent { .. } => None,
        }
    }

    /// Returns a checked owning-intent transition for a non-start result.
    #[must_use]
    pub const fn intent_transition(&self) -> Option<IntentTransition> {
        match self {
            Self::AlreadySatisfied { transition, .. } | Self::SuspendIntent { transition, .. } => {
                Some(*transition)
            }
            Self::Start { .. } => None,
        }
    }
}

/// Closed deterministic result of activity advancement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivityAdvancementDecision {
    /// Persist a checked transition and atomically open one successor action.
    OpenAction {
        /// Exact actor-safe input that produced this result.
        input: ActivityAdvancementInputFingerprint,
        /// Version-checked transition for the current activity.
        transition: ActivityTransition,
        /// Exactly one actor-safe action-opening directive.
        directive: ActivityActionDirective,
    },
    /// Complete the activity because accepted actor belief satisfies intent.
    Complete {
        /// Exact actor-safe input that produced this result.
        input: ActivityAdvancementInputFingerprint,
        /// Checked terminal activity transition.
        transition: ActivityTransition,
        /// Checked achievement transition for the owning intent.
        intent_transition: IntentTransition,
    },
    /// Fail after exhausting the bounded action-attempt budget.
    Fail {
        /// Exact actor-safe input that produced this result.
        input: ActivityAdvancementInputFingerprint,
        /// Checked terminal activity transition.
        transition: ActivityTransition,
        /// Checked failure transition for the owning intent.
        intent_transition: IntentTransition,
    },
    /// Consume the current controller invocation without inventing knowledge.
    Await {
        /// Exact actor-safe input that produced this result.
        input: ActivityAdvancementInputFingerprint,
        /// Checked transition into waiting when the activity was active.
        ///
        /// An activity already waiting or suspended needs no repeated
        /// lifecycle transition.
        transition: Option<ActivityTransition>,
    },
}

impl ActivityAdvancementDecision {
    /// Returns the exact actor-safe input fingerprint.
    #[must_use]
    pub const fn input_fingerprint(&self) -> ActivityAdvancementInputFingerprint {
        match self {
            Self::OpenAction { input, .. }
            | Self::Complete { input, .. }
            | Self::Fail { input, .. }
            | Self::Await { input, .. } => *input,
        }
    }

    /// Returns the checked activity transition, if one was proposed.
    #[must_use]
    pub const fn transition(&self) -> Option<ActivityTransition> {
        match self {
            Self::OpenAction { transition, .. }
            | Self::Complete { transition, .. }
            | Self::Fail { transition, .. } => Some(*transition),
            Self::Await { transition, .. } => *transition,
        }
    }

    /// Returns the successor action-opening directive, if any.
    #[must_use]
    pub const fn directive(&self) -> Option<&ActivityActionDirective> {
        match self {
            Self::OpenAction { directive, .. } => Some(directive),
            Self::Complete { .. } | Self::Fail { .. } | Self::Await { .. } => None,
        }
    }
}

/// Why the baseline activity controller refused or could not construct an
/// output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityControllerError {
    /// The input was constructed for a different controller behavior.
    SemanticsMismatch {
        /// Behavior identity required by this implementation.
        expected: ActivityControllerSemanticsId,
        /// Behavior identity committed by the input.
        actual: ActivityControllerSemanticsId,
    },
    /// Actor-relative semantic fields could not form controller state.
    InvalidState(ContainmentTransferActivityStateError),
    /// Travel method state could not advance.
    InvalidTravelState(TravelActivityStateError),
    /// Actor-relative semantic fields could not form an action scope.
    InvalidScope(ContainmentInteractionScopeError),
    /// The travel activity and its owning intent disagree.
    TravelIntentMismatch,
    /// A travel controller was invoked without an attempted-action cause.
    TravelCauseMismatch,
    /// The proposed transition was invalid for the accepted activity.
    InvalidTransition(ActivityTransitionError),
    /// A non-start result was invalid for the accepted owning intent.
    InvalidIntentTransition(IntentTransitionError),
    /// The accepted activity belongs to a different controller.
    ControllerMismatch {
        /// Controller identity required by this implementation.
        expected: ActivityControllerId,
        /// Controller identity retained by the accepted activity.
        actual: ActivityControllerId,
    },
    /// The accepted activity state uses a different schema.
    StateSchemaMismatch {
        /// State schema required by this implementation.
        expected: ActivityStateSchemaId,
        /// State schema retained by the accepted activity.
        actual: ActivityStateSchemaId,
    },
    /// A terminal activity cannot be advanced.
    TerminalActivity {
        /// Existing terminal status.
        status: ActivityStatus,
    },
}

impl fmt::Display for ActivityControllerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticsMismatch { expected, actual } => write!(
                formatter,
                "activity input semantics {actual} do not match baseline semantics {expected}"
            ),
            Self::InvalidState(error) => error.fmt(formatter),
            Self::InvalidTravelState(error) => error.fmt(formatter),
            Self::InvalidScope(error) => error.fmt(formatter),
            Self::TravelIntentMismatch => {
                formatter.write_str("travel activity does not match its owning intent")
            }
            Self::TravelCauseMismatch => {
                formatter.write_str("travel control progression requires an attempted action")
            }
            Self::InvalidTransition(error) => error.fmt(formatter),
            Self::InvalidIntentTransition(error) => error.fmt(formatter),
            Self::ControllerMismatch { expected, actual } => write!(
                formatter,
                "activity controller {actual:?} does not match baseline controller {expected:?}"
            ),
            Self::StateSchemaMismatch { expected, actual } => write!(
                formatter,
                "activity state schema {actual:?} does not match baseline schema {expected:?}"
            ),
            Self::TerminalActivity { status } => {
                write!(formatter, "terminal activity {status:?} cannot advance")
            }
        }
    }
}

impl std::error::Error for ActivityControllerError {}

/// Deterministic bounded controller for the implemented activity methods.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BaselineActivityController {
    _private: (),
}

impl BaselineActivityController {
    /// Constructs the baseline activity controller.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Returns the typed behavior identity used by compatible context input.
    #[must_use]
    pub fn semantics_id(self) -> ActivityControllerSemanticsId {
        ActivityControllerSemanticsId::from_bytes(identity(ACTIVITY_CONTROLLER_DOMAIN))
    }

    /// Returns the catalog-facing implementation identity.
    #[must_use]
    pub fn implementation_id(self) -> [u8; 32] {
        self.semantics_id().into_bytes()
    }

    /// Returns the exact accepted state schema owned by this controller.
    #[must_use]
    pub fn state_schema(self) -> ActivityStateSchemaId {
        activity_state_schema()
    }

    /// Initializes a bounded restore-home activity from accepted actor belief.
    pub fn initialize(
        self,
        input: &ContainmentActivityInitializationPayload,
    ) -> Result<ActivityInitializationDecision, ActivityControllerError> {
        self.check_semantics(input.controller_semantics())?;
        let fingerprint = input.fingerprint();
        let DesiredCondition::ItemContainedIn {
            item,
            container: destination,
        } = input.intent().desired()
        else {
            return Err(ActivityControllerError::TravelIntentMismatch);
        };
        let Some(belief) = input.current_belief() else {
            let transition = IntentTransition::Suspend;
            check_intent_transition(input.intent(), transition)?;
            return Ok(ActivityInitializationDecision::SuspendIntent {
                input: fingerprint,
                transition,
            });
        };
        if belief.container() == destination {
            let transition = IntentTransition::Achieve;
            check_intent_transition(input.intent(), transition)?;
            return Ok(ActivityInitializationDecision::AlreadySatisfied {
                input: fingerprint,
                transition,
            });
        }

        let initial = ContainmentTransferActivityState::new(
            item,
            belief.container(),
            destination,
            INITIAL_OPPORTUNITY_GENERATION,
            TOTAL_ACTION_ATTEMPTS,
        )
        .map_err(ActivityControllerError::InvalidState)?;
        let (state, directive) = open_next_action(initial)?;
        Ok(ActivityInitializationDecision::Start {
            input: fingerprint,
            start: Box::new(ActivityInitializationStart { state, directive }),
        })
    }

    /// Advances a persistent activity from current actor belief.
    pub fn advance(
        self,
        input: &ActivityAdvancementPayload,
    ) -> Result<ActivityAdvancementDecision, ActivityControllerError> {
        match input {
            ActivityAdvancementPayload::ContainmentTransfer(input) => {
                self.advance_containment(input)
            }
            ActivityAdvancementPayload::Travel(input) => self.advance_travel(input),
        }
    }

    fn advance_containment(
        self,
        input: &ContainmentActivityAdvancementPayload,
    ) -> Result<ActivityAdvancementDecision, ActivityControllerError> {
        self.check_semantics(input.controller_semantics())?;
        let activity = input.activity();
        let expected_controller = ActivityControllerId::from_bytes(self.implementation_id());
        if activity.controller() != expected_controller {
            return Err(ActivityControllerError::ControllerMismatch {
                expected: expected_controller,
                actual: activity.controller(),
            });
        }
        let expected_schema = self.state_schema();
        if activity.state_schema() != expected_schema {
            return Err(ActivityControllerError::StateSchemaMismatch {
                expected: expected_schema,
                actual: activity.state_schema(),
            });
        }
        if activity.status().is_terminal() {
            return Err(ActivityControllerError::TerminalActivity {
                status: activity.status(),
            });
        }
        let fingerprint = input.fingerprint();
        let state = activity.state().containment_transfer().ok_or(
            ActivityControllerError::StateSchemaMismatch {
                expected: self.state_schema(),
                actual: activity.state_schema(),
            },
        )?;
        let Some(belief) = input.current_belief() else {
            let transition = match activity.status() {
                ActivityStatus::Active => Some(ActivityTransition::Wait(state.into())),
                ActivityStatus::Waiting | ActivityStatus::Suspended => None,
                ActivityStatus::Completed | ActivityStatus::Failed | ActivityStatus::Cancelled => {
                    unreachable!("terminal activity was rejected above")
                }
            };
            if let Some(transition) = transition {
                check_transition(activity, transition)?;
            }
            return Ok(ActivityAdvancementDecision::Await {
                input: fingerprint,
                transition,
            });
        };

        if belief.container() == state.destination() {
            let transition = ActivityTransition::Complete;
            let intent_transition = IntentTransition::Achieve;
            check_transition(activity, transition)?;
            check_intent_transition(input.intent(), intent_transition)?;
            return Ok(ActivityAdvancementDecision::Complete {
                input: fingerprint,
                transition,
                intent_transition,
            });
        }
        if state.remaining_attempts() == 0 {
            let transition = ActivityTransition::Fail;
            let intent_transition = IntentTransition::Fail;
            check_transition(activity, transition)?;
            check_intent_transition(input.intent(), intent_transition)?;
            return Ok(ActivityAdvancementDecision::Fail {
                input: fingerprint,
                transition,
                intent_transition,
            });
        }

        let current = state
            .with_source(belief.container())
            .map_err(ActivityControllerError::InvalidState)?;
        let (next, directive) = open_next_action(current)?;
        let transition = match activity.status() {
            ActivityStatus::Active => ActivityTransition::Continue(next.into()),
            ActivityStatus::Waiting | ActivityStatus::Suspended => {
                ActivityTransition::Resume(next.into())
            }
            ActivityStatus::Completed | ActivityStatus::Failed | ActivityStatus::Cancelled => {
                unreachable!("terminal activity was rejected above")
            }
        };
        check_transition(activity, transition)?;
        Ok(ActivityAdvancementDecision::OpenAction {
            input: fingerprint,
            transition,
            directive: ActivityActionDirective::ContainmentTransfer(directive),
        })
    }

    fn advance_travel(
        self,
        input: &TravelActivityAdvancementPayload,
    ) -> Result<ActivityAdvancementDecision, ActivityControllerError> {
        self.check_semantics(input.controller_semantics())?;
        let activity = input.activity();
        let expected_controller = ActivityControllerId::from_bytes(self.implementation_id());
        if activity.controller() != expected_controller {
            return Err(ActivityControllerError::ControllerMismatch {
                expected: expected_controller,
                actual: activity.controller(),
            });
        }
        let expected_schema = self.state_schema();
        if activity.state_schema() != expected_schema {
            return Err(ActivityControllerError::StateSchemaMismatch {
                expected: expected_schema,
                actual: activity.state_schema(),
            });
        }
        if activity.status().is_terminal() {
            return Err(ActivityControllerError::TerminalActivity {
                status: activity.status(),
            });
        }
        let state = input.state();
        if input.intent().desired()
            != (DesiredCondition::ActorAt {
                location: state.destination(),
            })
        {
            return Err(ActivityControllerError::TravelIntentMismatch);
        }
        let fingerprint = input.fingerprint();
        if state.step() == TravelActivityStep::AwaitArrival {
            let transition = match activity.status() {
                ActivityStatus::Active => Some(ActivityTransition::Wait(state.into())),
                ActivityStatus::Waiting | ActivityStatus::Suspended => None,
                ActivityStatus::Completed | ActivityStatus::Failed | ActivityStatus::Cancelled => {
                    unreachable!("terminal activity was rejected above")
                }
            };
            if let Some(transition) = transition {
                check_transition(activity, transition)?;
            }
            return Ok(ActivityAdvancementDecision::Await {
                input: fingerprint,
                transition,
            });
        }
        if input.cause() != ActivityEvaluationCause::AttemptedAction {
            return Err(ActivityControllerError::TravelCauseMismatch);
        }
        let (next, directive) = open_next_travel_action(state)?;
        let transition = match activity.status() {
            ActivityStatus::Active => ActivityTransition::Continue(next.into()),
            ActivityStatus::Waiting | ActivityStatus::Suspended => {
                ActivityTransition::Resume(next.into())
            }
            ActivityStatus::Completed | ActivityStatus::Failed | ActivityStatus::Cancelled => {
                unreachable!("terminal activity was rejected above")
            }
        };
        check_transition(activity, transition)?;
        Ok(ActivityAdvancementDecision::OpenAction {
            input: fingerprint,
            transition,
            directive: ActivityActionDirective::Relocation(directive),
        })
    }

    fn check_semantics(
        self,
        actual: ActivityControllerSemanticsId,
    ) -> Result<(), ActivityControllerError> {
        let expected = self.semantics_id();
        if actual == expected {
            Ok(())
        } else {
            Err(ActivityControllerError::SemanticsMismatch { expected, actual })
        }
    }
}

impl ActivityController for BaselineActivityController {
    fn implementation_id(&self) -> [u8; 32] {
        (*self).implementation_id()
    }

    fn initialize(
        &self,
        input: &ContainmentActivityInitializationPayload,
    ) -> Result<ActivityInitializationDecision, ActivityControllerError> {
        (*self).initialize(input)
    }

    fn advance(
        &self,
        input: &ActivityAdvancementPayload,
    ) -> Result<ActivityAdvancementDecision, ActivityControllerError> {
        (*self).advance(input)
    }
}

fn open_next_travel_action(
    state: TravelActivityState,
) -> Result<(TravelActivityState, RelocationActionDirective), ActivityControllerError> {
    let verb = match state.step() {
        TravelActivityStep::Pause => RelocationActionVerb::Pause,
        TravelActivityStep::Resume => RelocationActionVerb::Resume,
        TravelActivityStep::AwaitArrival => {
            return Err(ActivityControllerError::InvalidTravelState(
                TravelActivityStateError::AwaitingArrival,
            ));
        }
    };
    let generation = state.next_opportunity_generation();
    let next = state
        .after_opening_control()
        .map_err(ActivityControllerError::InvalidTravelState)?;
    Ok((
        next,
        RelocationActionDirective {
            generation,
            verb,
            source: state.source(),
            destination: state.destination(),
        },
    ))
}

fn open_next_action(
    state: ContainmentTransferActivityState,
) -> Result<(ContainmentTransferActivityState, ContainmentActionDirective), ActivityControllerError>
{
    let scope = ContainmentInteractionScope::new(
        state.source(),
        vec![state.destination()],
        vec![state.item()],
        ACTION_CANDIDATE_LIMIT,
    )
    .map_err(ActivityControllerError::InvalidScope)?;
    let generation = state.next_opportunity_generation();
    let next = state
        .after_opening_opportunity()
        .map_err(ActivityControllerError::InvalidState)?;
    Ok((next, ContainmentActionDirective { generation, scope }))
}

fn check_transition(
    activity: world_model::Activity,
    transition: ActivityTransition,
) -> Result<(), ActivityControllerError> {
    activity
        .transition(activity.version(), transition)
        .map(|_| ())
        .map_err(ActivityControllerError::InvalidTransition)
}

fn check_intent_transition(
    intent: world_model::Intent,
    transition: IntentTransition,
) -> Result<(), ActivityControllerError> {
    intent
        .transition(intent.version(), transition)
        .map(|_| ())
        .map_err(ActivityControllerError::InvalidIntentTransition)
}
