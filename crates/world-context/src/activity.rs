use core::fmt;

use world_core::{CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest};
use world_model::{
    Activity, ActivityId, ActivityState, ActivityStatus, ContainedInBelief,
    ContainmentAppraisalFingerprint, DesiredCondition, Intent, IntentId, IntentStatus,
    TravelActivityState, WorldSnapshot,
};

use crate::appraisal::write_belief;
use crate::intent::write_desired;
use crate::{
    ActivityAdvancementInputFingerprint, ActivityControllerSemanticsId,
    ActivityInitializationInputFingerprint,
};

const INPUT_SCHEMA_VERSION: u16 = 1;
const INITIALIZATION_INPUT_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("containment-activity-initialization-input-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("activity initialization input domain must be valid"),
    };
const ADVANCEMENT_INPUT_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("containment-activity-advancement-input-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("activity advancement input domain must be valid"),
    };
const TRAVEL_ADVANCEMENT_INPUT_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("travel-activity-advancement-input-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("travel activity advancement input domain must be valid"),
    };

/// Actor-safe semantic reason for invoking an activity controller.
///
/// Rich authoritative attempt outcomes, process identities, revisions, and
/// scheduler coordinates are deliberately not representable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityEvaluationCause {
    /// A sponsored action attempt reached a terminal boundary.
    AttemptedAction,
    /// Actor-relative appraisal material changed.
    AppraisalChanged {
        /// New material appraisal identity.
        appraisal: ContainmentAppraisalFingerprint,
    },
    /// A previously requested bounded recovery point became due.
    ScheduledRecovery,
}

/// Why an activity input could not be projected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityProjectionError {
    /// The requested accepted intent does not exist.
    MissingIntent {
        /// Missing intent identity.
        intent: IntentId,
    },
    /// The requested accepted activity does not exist.
    MissingActivity {
        /// Missing activity identity.
        activity: ActivityId,
    },
    /// A canonical input identity could not represent one of its fields.
    Canonical(CanonicalError),
}

impl fmt::Display for ActivityProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIntent { intent } => {
                write!(formatter, "accepted intent {intent} does not exist")
            }
            Self::MissingActivity { activity } => {
                write!(formatter, "accepted activity {activity} does not exist")
            }
            Self::Canonical(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ActivityProjectionError {}

impl From<CanonicalError> for ActivityProjectionError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

/// Actor-safe input for initializing one containment activity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainmentActivityInitializationPayload {
    intent: Intent,
    current_belief: Option<ContainedInBelief>,
    cause: ActivityEvaluationCause,
    controller_semantics: ActivityControllerSemanticsId,
    fingerprint: ActivityInitializationInputFingerprint,
}

impl ContainmentActivityInitializationPayload {
    /// Returns the accepted intent being initialized.
    #[must_use]
    pub const fn intent(&self) -> Intent {
        self.intent
    }

    /// Returns the actor's accepted current belief, if one exists.
    #[must_use]
    pub const fn current_belief(&self) -> Option<&ContainedInBelief> {
        self.current_belief.as_ref()
    }

    /// Returns the actor-safe invocation cause.
    #[must_use]
    pub const fn cause(&self) -> ActivityEvaluationCause {
        self.cause
    }

    /// Returns the exact activity-controller behavior identity.
    #[must_use]
    pub const fn controller_semantics(&self) -> ActivityControllerSemanticsId {
        self.controller_semantics
    }

    /// Returns the canonical identity of this complete controller input.
    #[must_use]
    pub const fn fingerprint(&self) -> ActivityInitializationInputFingerprint {
        self.fingerprint
    }
}

/// Actor-safe input for advancing one persistent containment activity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainmentActivityAdvancementPayload {
    intent: Intent,
    activity: Activity,
    current_belief: Option<ContainedInBelief>,
    cause: ActivityEvaluationCause,
    controller_semantics: ActivityControllerSemanticsId,
    fingerprint: ActivityAdvancementInputFingerprint,
}

/// Actor-safe input for advancing one persistent travel activity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TravelActivityAdvancementPayload {
    intent: Intent,
    activity: Activity,
    cause: ActivityEvaluationCause,
    controller_semantics: ActivityControllerSemanticsId,
    fingerprint: ActivityAdvancementInputFingerprint,
}

impl TravelActivityAdvancementPayload {
    #[must_use]
    pub const fn intent(&self) -> Intent {
        self.intent
    }

    #[must_use]
    pub const fn activity(&self) -> Activity {
        self.activity
    }

    #[must_use]
    pub fn state(&self) -> TravelActivityState {
        match self.activity.state() {
            ActivityState::Travel(state) => state,
            ActivityState::ContainmentTransfer(_) => {
                unreachable!("travel payload construction checks its activity method")
            }
        }
    }

    #[must_use]
    pub const fn cause(&self) -> ActivityEvaluationCause {
        self.cause
    }

    #[must_use]
    pub const fn controller_semantics(&self) -> ActivityControllerSemanticsId {
        self.controller_semantics
    }

    #[must_use]
    pub const fn fingerprint(&self) -> ActivityAdvancementInputFingerprint {
        self.fingerprint
    }
}

/// Closed actor-safe advancement input for the implemented activity methods.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivityAdvancementPayload {
    ContainmentTransfer(ContainmentActivityAdvancementPayload),
    Travel(TravelActivityAdvancementPayload),
}

impl ActivityAdvancementPayload {
    #[must_use]
    pub const fn activity(&self) -> Activity {
        match self {
            Self::ContainmentTransfer(input) => input.activity(),
            Self::Travel(input) => input.activity(),
        }
    }

    #[must_use]
    pub const fn intent(&self) -> Intent {
        match self {
            Self::ContainmentTransfer(input) => input.intent(),
            Self::Travel(input) => input.intent(),
        }
    }

    #[must_use]
    pub const fn cause(&self) -> ActivityEvaluationCause {
        match self {
            Self::ContainmentTransfer(input) => input.cause(),
            Self::Travel(input) => input.cause(),
        }
    }

    #[must_use]
    pub const fn controller_semantics(&self) -> ActivityControllerSemanticsId {
        match self {
            Self::ContainmentTransfer(input) => input.controller_semantics(),
            Self::Travel(input) => input.controller_semantics(),
        }
    }

    #[must_use]
    pub const fn fingerprint(&self) -> ActivityAdvancementInputFingerprint {
        match self {
            Self::ContainmentTransfer(input) => input.fingerprint(),
            Self::Travel(input) => input.fingerprint(),
        }
    }
}

impl ContainmentActivityAdvancementPayload {
    /// Returns the activity's accepted owning intent.
    #[must_use]
    pub const fn intent(&self) -> Intent {
        self.intent
    }

    /// Returns the accepted persistent activity state.
    #[must_use]
    pub const fn activity(&self) -> Activity {
        self.activity
    }

    /// Returns the actor's accepted current belief, if one exists.
    #[must_use]
    pub const fn current_belief(&self) -> Option<&ContainedInBelief> {
        self.current_belief.as_ref()
    }

    /// Returns the actor-safe invocation cause.
    #[must_use]
    pub const fn cause(&self) -> ActivityEvaluationCause {
        self.cause
    }

    /// Returns the exact activity-controller behavior identity.
    #[must_use]
    pub const fn controller_semantics(&self) -> ActivityControllerSemanticsId {
        self.controller_semantics
    }

    /// Returns the canonical identity of this complete controller input.
    #[must_use]
    pub const fn fingerprint(&self) -> ActivityAdvancementInputFingerprint {
        self.fingerprint
    }
}

/// Pure projector for containment activity initialization and advancement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContainmentActivityProjector {
    _private: (),
}

/// Pure projector for the closed implemented activity-method set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivityProjector {
    _private: (),
}

impl ActivityProjector {
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Projects one method-specific actor-safe advancement input.
    pub fn advancement(
        self,
        snapshot: &WorldSnapshot,
        activity: ActivityId,
        cause: ActivityEvaluationCause,
        controller_semantics: ActivityControllerSemanticsId,
    ) -> Result<ActivityAdvancementPayload, ActivityProjectionError> {
        let accepted = snapshot
            .accepted()
            .agency()
            .activity(activity)
            .copied()
            .ok_or(ActivityProjectionError::MissingActivity { activity })?;
        match accepted.state() {
            ActivityState::ContainmentTransfer(_) => ContainmentActivityProjector::new()
                .advancement(snapshot, activity, cause, controller_semantics)
                .map(ActivityAdvancementPayload::ContainmentTransfer),
            ActivityState::Travel(_) => {
                let intent_id = accepted.intent();
                let intent = snapshot
                    .accepted()
                    .agency()
                    .intent(intent_id)
                    .copied()
                    .ok_or(ActivityProjectionError::MissingIntent { intent: intent_id })?;
                let fingerprint =
                    travel_advancement_fingerprint(intent, accepted, cause, controller_semantics)?;
                Ok(ActivityAdvancementPayload::Travel(
                    TravelActivityAdvancementPayload {
                        intent,
                        activity: accepted,
                        cause,
                        controller_semantics,
                        fingerprint,
                    },
                ))
            }
        }
    }
}

impl ContainmentActivityProjector {
    /// Constructs the containment activity projector.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Projects one initialization input from accepted agency and epistemic
    /// semantic state.
    pub fn initialization(
        self,
        snapshot: &WorldSnapshot,
        intent: IntentId,
        cause: ActivityEvaluationCause,
        controller_semantics: ActivityControllerSemanticsId,
    ) -> Result<ContainmentActivityInitializationPayload, ActivityProjectionError> {
        let intent = snapshot
            .accepted()
            .agency()
            .intent(intent)
            .copied()
            .ok_or(ActivityProjectionError::MissingIntent { intent })?;
        let current_belief = current_belief(snapshot, intent);
        let fingerprint = initialization_fingerprint(
            intent,
            current_belief.as_ref(),
            cause,
            controller_semantics,
        )?;
        Ok(ContainmentActivityInitializationPayload {
            intent,
            current_belief,
            cause,
            controller_semantics,
            fingerprint,
        })
    }

    /// Projects one advancement input from accepted agency and epistemic
    /// semantic state.
    pub fn advancement(
        self,
        snapshot: &WorldSnapshot,
        activity: ActivityId,
        cause: ActivityEvaluationCause,
        controller_semantics: ActivityControllerSemanticsId,
    ) -> Result<ContainmentActivityAdvancementPayload, ActivityProjectionError> {
        let agency = snapshot.accepted().agency();
        let activity = agency
            .activity(activity)
            .copied()
            .ok_or(ActivityProjectionError::MissingActivity { activity })?;
        let intent_id = activity.intent();
        let intent = agency
            .intent(intent_id)
            .copied()
            .ok_or(ActivityProjectionError::MissingIntent { intent: intent_id })?;
        let current_belief = current_belief(snapshot, intent);
        let fingerprint = advancement_fingerprint(
            intent,
            activity,
            current_belief.as_ref(),
            cause,
            controller_semantics,
        )?;
        Ok(ContainmentActivityAdvancementPayload {
            intent,
            activity,
            current_belief,
            cause,
            controller_semantics,
            fingerprint,
        })
    }
}

fn current_belief(snapshot: &WorldSnapshot, intent: Intent) -> Option<ContainedInBelief> {
    match intent.desired() {
        DesiredCondition::ItemContainedIn { item, .. } => snapshot
            .accepted()
            .epistemic()
            .contained_in(intent.actor(), item)
            .cloned(),
        DesiredCondition::ActorAt { .. } => None,
    }
}

fn initialization_fingerprint(
    intent: Intent,
    current_belief: Option<&ContainedInBelief>,
    cause: ActivityEvaluationCause,
    controller_semantics: ActivityControllerSemanticsId,
) -> Result<ActivityInitializationInputFingerprint, CanonicalError> {
    let mut writer = CanonicalWriter::new(INITIALIZATION_INPUT_DOMAIN);
    writer.write_u16(INPUT_SCHEMA_VERSION);
    write_intent(&mut writer, intent)?;
    write_optional_belief(&mut writer, current_belief)?;
    write_cause(&mut writer, cause)?;
    writer.write_bytes(controller_semantics.as_bytes())?;
    Ok(ActivityInitializationInputFingerprint(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

fn advancement_fingerprint(
    intent: Intent,
    activity: Activity,
    current_belief: Option<&ContainedInBelief>,
    cause: ActivityEvaluationCause,
    controller_semantics: ActivityControllerSemanticsId,
) -> Result<ActivityAdvancementInputFingerprint, CanonicalError> {
    let mut writer = CanonicalWriter::new(ADVANCEMENT_INPUT_DOMAIN);
    writer.write_u16(INPUT_SCHEMA_VERSION);
    write_intent(&mut writer, intent)?;
    write_activity(&mut writer, activity)?;
    write_optional_belief(&mut writer, current_belief)?;
    write_cause(&mut writer, cause)?;
    writer.write_bytes(controller_semantics.as_bytes())?;
    Ok(ActivityAdvancementInputFingerprint(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

fn travel_advancement_fingerprint(
    intent: Intent,
    activity: Activity,
    cause: ActivityEvaluationCause,
    controller_semantics: ActivityControllerSemanticsId,
) -> Result<ActivityAdvancementInputFingerprint, CanonicalError> {
    let mut writer = CanonicalWriter::new(TRAVEL_ADVANCEMENT_INPUT_DOMAIN);
    writer.write_u16(INPUT_SCHEMA_VERSION);
    write_intent(&mut writer, intent)?;
    write_activity(&mut writer, activity)?;
    write_cause(&mut writer, cause)?;
    writer.write_bytes(controller_semantics.as_bytes())?;
    Ok(ActivityAdvancementInputFingerprint(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

fn write_intent(writer: &mut CanonicalWriter, intent: Intent) -> Result<(), CanonicalError> {
    writer.write_bytes(intent.id().as_bytes())?;
    writer.write_bytes(intent.actor().as_bytes())?;
    writer.write_u64(intent.generation().get());
    writer.write_u64(intent.version().get());
    write_desired(writer, intent.desired())?;
    writer.write_discriminant(intent_status_tag(intent.status()));
    Ok(())
}

fn write_activity(writer: &mut CanonicalWriter, activity: Activity) -> Result<(), CanonicalError> {
    writer.write_bytes(activity.id().as_bytes())?;
    writer.write_bytes(activity.actor().as_bytes())?;
    writer.write_bytes(activity.intent().as_bytes())?;
    writer.write_u64(activity.generation().get());
    writer.write_u64(activity.version().get());
    writer.write_bytes(activity.controller().as_bytes())?;
    writer.write_bytes(activity.state_schema().as_bytes())?;
    writer.write_discriminant(activity_status_tag(activity.status()));
    write_activity_state(writer, activity.state())
}

fn write_activity_state(
    writer: &mut CanonicalWriter,
    state: ActivityState,
) -> Result<(), CanonicalError> {
    match state {
        ActivityState::ContainmentTransfer(state) => {
            writer.write_discriminant(0);
            writer.write_bytes(state.item().as_bytes())?;
            writer.write_bytes(state.source().as_bytes())?;
            writer.write_bytes(state.destination().as_bytes())?;
            writer.write_u64(state.next_opportunity_generation().get());
            writer.write_u32(state.remaining_attempts());
            Ok(())
        }
        ActivityState::Travel(state) => {
            writer.write_discriminant(1);
            writer.write_bytes(state.source().as_bytes())?;
            writer.write_bytes(state.destination().as_bytes())?;
            writer.write_u64(state.next_opportunity_generation().get());
            writer.write_discriminant(match state.step() {
                world_model::TravelActivityStep::Pause => 0,
                world_model::TravelActivityStep::Resume => 1,
                world_model::TravelActivityStep::AwaitArrival => 2,
            });
            Ok(())
        }
    }
}

fn write_optional_belief(
    writer: &mut CanonicalWriter,
    belief: Option<&ContainedInBelief>,
) -> Result<(), CanonicalError> {
    match belief {
        Some(belief) => {
            writer.write_discriminant(1);
            write_belief(writer, belief)
        }
        None => {
            writer.write_discriminant(0);
            Ok(())
        }
    }
}

fn write_cause(
    writer: &mut CanonicalWriter,
    cause: ActivityEvaluationCause,
) -> Result<(), CanonicalError> {
    match cause {
        ActivityEvaluationCause::AttemptedAction => {
            writer.write_discriminant(0);
            Ok(())
        }
        ActivityEvaluationCause::AppraisalChanged { appraisal } => {
            writer.write_discriminant(1);
            writer.write_bytes(appraisal.as_bytes())
        }
        ActivityEvaluationCause::ScheduledRecovery => {
            writer.write_discriminant(2);
            Ok(())
        }
    }
}

const fn intent_status_tag(status: IntentStatus) -> u32 {
    match status {
        IntentStatus::Active => 0,
        IntentStatus::Suspended => 1,
        IntentStatus::Achieved => 2,
        IntentStatus::Abandoned => 3,
        IntentStatus::Failed => 4,
    }
}

const fn activity_status_tag(status: ActivityStatus) -> u32 {
    match status {
        ActivityStatus::Active => 0,
        ActivityStatus::Waiting => 1,
        ActivityStatus::Suspended => 2,
        ActivityStatus::Completed => 3,
        ActivityStatus::Failed => 4,
        ActivityStatus::Cancelled => 5,
    }
}
