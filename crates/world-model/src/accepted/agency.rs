use core::fmt;
use core::num::NonZeroU64;

use world_core::{
    ActorId, CanonicalBytes, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest,
    EntityId,
};

use crate::action_opportunity::ActionOpportunityGeneration;

/// Canonical schema version of [`AgencyState`].
pub const AGENCY_STATE_SCHEMA_VERSION: u16 = 3;

const AGENCY_STATE_CANONICAL_DOMAIN: CanonicalDomain = match CanonicalDomain::new("agency-state-v3")
{
    Ok(domain) => domain,
    Err(_) => panic!("agency-state identity domain must be valid"),
};
const INTENT_ID_CANONICAL_DOMAIN: CanonicalDomain = match CanonicalDomain::new("intent-id-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("intent identity domain must be valid"),
};
const ACTIVITY_ID_CANONICAL_DOMAIN: CanonicalDomain = match CanonicalDomain::new("activity-id-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("activity identity domain must be valid"),
};

macro_rules! nonzero_coordinate {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(NonZeroU64);

        impl $name {
            /// Constructs a nonzero coordinate.
            #[must_use]
            pub const fn new(value: u64) -> Option<Self> {
                match NonZeroU64::new(value) {
                    Some(value) => Some(Self(value)),
                    None => None,
                }
            }

            /// Returns the exact coordinate scalar.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0.get()
            }

            /// Returns the following coordinate, or `None` on overflow.
            #[must_use]
            pub const fn checked_next(self) -> Option<Self> {
                match self.get().checked_add(1) {
                    Some(value) => Self::new(value),
                    None => None,
                }
            }
        }
    };
}

nonzero_coordinate!(
    /// Actor-local generation of one intent.
    IntentGeneration
);
nonzero_coordinate!(
    /// Version of one accepted intent record.
    IntentVersion
);
nonzero_coordinate!(
    /// Actor-local generation of one activity.
    ActivityGeneration
);
nonzero_coordinate!(
    /// Version of one accepted activity record.
    ActivityVersion
);

impl IntentVersion {
    /// Version of a newly adopted intent.
    pub const INITIAL: Self = match NonZeroU64::new(1) {
        Some(value) => Self(value),
        None => unreachable!(),
    };
}

impl ActivityVersion {
    /// Version of a newly started activity.
    pub const INITIAL: Self = match NonZeroU64::new(1) {
        Some(value) => Self(value),
        None => unreachable!(),
    };
}

/// Concrete desired world condition supported by the first agency slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DesiredCondition {
    /// The actor wants one item to have one direct container.
    ItemContainedIn {
        /// Desired item.
        item: EntityId,
        /// Desired direct container.
        container: EntityId,
    },
    /// The actor wants to arrive at one location.
    ActorAt {
        /// Desired destination.
        location: EntityId,
    },
}

impl DesiredCondition {
    /// Constructs the initial containment condition.
    #[must_use]
    pub const fn item_contained_in(item: EntityId, container: EntityId) -> Self {
        Self::ItemContainedIn { item, container }
    }

    /// Constructs a travel destination condition.
    #[must_use]
    pub const fn actor_at(location: EntityId) -> Self {
        Self::ActorAt { location }
    }

    fn write_canonical(self, writer: &mut CanonicalWriter) -> Result<(), CanonicalError> {
        match self {
            Self::ItemContainedIn { item, container } => {
                writer.write_discriminant(0);
                writer.write_bytes(item.as_bytes())?;
                writer.write_bytes(container.as_bytes())
            }
            Self::ActorAt { location } => {
                writer.write_discriminant(1);
                writer.write_bytes(location.as_bytes())
            }
        }
    }
}

/// Stable semantic identity of one actor-owned intent.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntentId([u8; 32]);

impl IntentId {
    /// Constructs an identity decoded from durable model data.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Derives an intent identity from actor-local semantic coordinates.
    #[must_use]
    pub fn derive(actor: ActorId, generation: IntentGeneration, desired: DesiredCondition) -> Self {
        let encoded = (|| -> Result<_, CanonicalError> {
            let mut writer = CanonicalWriter::new(INTENT_ID_CANONICAL_DOMAIN);
            writer.write_u16(1);
            writer.write_bytes(actor.as_bytes())?;
            writer.write_u64(generation.get());
            desired.write_canonical(&mut writer)?;
            Ok(writer.finish())
        })();
        match encoded {
            Ok(bytes) => Self(ContentDigest::of_canonical(&bytes).into_bytes()),
            Err(error) => unreachable!("fixed intent identity must be canonical: {error}"),
        }
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for IntentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for IntentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "IntentId({self})")
    }
}

/// Durable lifecycle status of an intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IntentStatus {
    Active,
    Suspended,
    Achieved,
    Abandoned,
    Failed,
}

impl IntentStatus {
    /// Returns whether no later lifecycle transition is legal.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Achieved | Self::Abandoned | Self::Failed)
    }
}

/// One legal requested intent transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentTransition {
    Suspend,
    Resume,
    Achieve,
    Abandon,
    Fail,
}

/// Why an intent transition could not construct a successor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentTransitionError {
    StaleVersion {
        expected: IntentVersion,
        actual: IntentVersion,
    },
    InvalidStatus {
        current: IntentStatus,
        transition: IntentTransition,
    },
    VersionOverflow,
}

impl fmt::Display for IntentTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for IntentTransitionError {}

/// Immutable accepted intent record.
///
/// Intent adoption provenance belongs to the authority transition that
/// accepts this record. The intent deliberately contains no evidence ID:
/// evidence supports actor-relative beliefs, while an appraisal or authored
/// objective may adopt the same desired condition. This avoids an unchecked
/// cross-partition reference and keeps the accepted intent meaningful after
/// supporting evidence is superseded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intent {
    id: IntentId,
    actor: ActorId,
    generation: IntentGeneration,
    version: IntentVersion,
    desired: DesiredCondition,
    status: IntentStatus,
}

impl Intent {
    /// Adopts a new active intent.
    #[must_use]
    pub fn adopt(actor: ActorId, generation: IntentGeneration, desired: DesiredCondition) -> Self {
        Self {
            id: IntentId::derive(actor, generation, desired),
            actor,
            generation,
            version: IntentVersion::INITIAL,
            desired,
            status: IntentStatus::Active,
        }
    }

    #[must_use]
    pub const fn id(self) -> IntentId {
        self.id
    }

    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    #[must_use]
    pub const fn generation(self) -> IntentGeneration {
        self.generation
    }

    #[must_use]
    pub const fn version(self) -> IntentVersion {
        self.version
    }

    #[must_use]
    pub const fn desired(self) -> DesiredCondition {
        self.desired
    }

    #[must_use]
    pub const fn status(self) -> IntentStatus {
        self.status
    }

    /// Constructs a version-checked legal intent successor.
    pub fn transition(
        self,
        expected_version: IntentVersion,
        transition: IntentTransition,
    ) -> Result<Self, IntentTransitionError> {
        if expected_version != self.version {
            return Err(IntentTransitionError::StaleVersion {
                expected: expected_version,
                actual: self.version,
            });
        }
        let status = match (self.status, transition) {
            (IntentStatus::Active, IntentTransition::Suspend) => IntentStatus::Suspended,
            (IntentStatus::Suspended, IntentTransition::Resume) => IntentStatus::Active,
            (IntentStatus::Active | IntentStatus::Suspended, IntentTransition::Achieve) => {
                IntentStatus::Achieved
            }
            (IntentStatus::Active | IntentStatus::Suspended, IntentTransition::Abandon) => {
                IntentStatus::Abandoned
            }
            (IntentStatus::Active | IntentStatus::Suspended, IntentTransition::Fail) => {
                IntentStatus::Failed
            }
            (current, transition) => {
                return Err(IntentTransitionError::InvalidStatus {
                    current,
                    transition,
                });
            }
        };
        let version = self
            .version
            .checked_next()
            .ok_or(IntentTransitionError::VersionOverflow)?;
        Ok(Self {
            status,
            version,
            ..self
        })
    }

    fn has_valid_id(self) -> bool {
        self.id == IntentId::derive(self.actor, self.generation, self.desired)
    }
}

/// Stable semantic identity of one actor-owned activity.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActivityId([u8; 32]);

impl ActivityId {
    /// Constructs an identity decoded from durable model data.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Derives an activity identity from its owner, intent, and generation.
    #[must_use]
    pub fn derive(actor: ActorId, intent: IntentId, generation: ActivityGeneration) -> Self {
        let encoded = (|| -> Result<_, CanonicalError> {
            let mut writer = CanonicalWriter::new(ACTIVITY_ID_CANONICAL_DOMAIN);
            writer.write_u16(1);
            writer.write_bytes(actor.as_bytes())?;
            writer.write_bytes(intent.as_bytes())?;
            writer.write_u64(generation.get());
            Ok(writer.finish())
        })();
        match encoded {
            Ok(bytes) => Self(ContentDigest::of_canonical(&bytes).into_bytes()),
            Err(error) => unreachable!("fixed activity identity must be canonical: {error}"),
        }
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ActivityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for ActivityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ActivityId({self})")
    }
}

macro_rules! opaque_activity_identity {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Constructs an identity decoded or derived by its semantic
            /// owner.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Returns the exact identity bytes.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }

            /// Consumes the identity and returns its exact bytes.
            #[must_use]
            pub const fn into_bytes(self) -> [u8; 32] {
                self.0
            }
        }
    };
}

opaque_activity_identity!(
    /// Semantic implementation identity of an activity controller.
    ActivityControllerId
);
opaque_activity_identity!(
    /// Schema identity of persistent activity-controller state.
    ActivityStateSchemaId
);

/// Why containment-transfer activity state was structurally invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainmentTransferActivityStateError {
    /// Source and destination named the same container.
    SourceEqualsDestination { container: EntityId },
    /// The item was also used as a containment anchor.
    ItemEqualsContainer { item: EntityId, container: EntityId },
    /// Opportunity generations begin at one for a live activity.
    ZeroOpportunityGeneration,
    /// No bounded recovery attempt remains.
    AttemptsExhausted,
    /// An update changed the item or desired destination.
    ChangedDesiredCondition,
    /// Opportunity generation and remaining attempts did not advance
    /// together by exactly one.
    InvalidOpportunityProgress,
}

impl fmt::Display for ContainmentTransferActivityStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ContainmentTransferActivityStateError {}

/// Concrete semantic state retained by a containment-transfer activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainmentTransferActivityState {
    item: EntityId,
    source: EntityId,
    destination: EntityId,
    next_opportunity_generation: ActionOpportunityGeneration,
    remaining_attempts: u32,
}

impl ContainmentTransferActivityState {
    /// Constructs the exact item and containment anchors pursued by an
    /// activity together with its bounded recovery cursor.
    pub fn new(
        item: EntityId,
        source: EntityId,
        destination: EntityId,
        next_opportunity_generation: ActionOpportunityGeneration,
        remaining_attempts: u32,
    ) -> Result<Self, ContainmentTransferActivityStateError> {
        if source == destination {
            return Err(
                ContainmentTransferActivityStateError::SourceEqualsDestination {
                    container: source,
                },
            );
        }
        if item == source {
            return Err(ContainmentTransferActivityStateError::ItemEqualsContainer {
                item,
                container: source,
            });
        }
        if item == destination {
            return Err(ContainmentTransferActivityStateError::ItemEqualsContainer {
                item,
                container: destination,
            });
        }
        if next_opportunity_generation.get() == 0 {
            return Err(ContainmentTransferActivityStateError::ZeroOpportunityGeneration);
        }
        Ok(Self {
            item,
            source,
            destination,
            next_opportunity_generation,
            remaining_attempts,
        })
    }

    #[must_use]
    pub const fn item(self) -> EntityId {
        self.item
    }

    #[must_use]
    pub const fn source(self) -> EntityId {
        self.source
    }

    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.destination
    }

    /// Returns the generation to assign to the next causally linked
    /// opportunity.
    #[must_use]
    pub const fn next_opportunity_generation(self) -> ActionOpportunityGeneration {
        self.next_opportunity_generation
    }

    /// Returns the bounded number of opportunities that may still be opened.
    #[must_use]
    pub const fn remaining_attempts(self) -> u32 {
        self.remaining_attempts
    }

    /// Constructs the exact state after opening one opportunity.
    pub fn after_opening_opportunity(self) -> Result<Self, ContainmentTransferActivityStateError> {
        let remaining_attempts = self
            .remaining_attempts
            .checked_sub(1)
            .ok_or(ContainmentTransferActivityStateError::AttemptsExhausted)?;
        let generation = self
            .next_opportunity_generation
            .get()
            .checked_add(1)
            .ok_or(ContainmentTransferActivityStateError::InvalidOpportunityProgress)?;
        Ok(Self {
            next_opportunity_generation: ActionOpportunityGeneration::new(generation),
            remaining_attempts,
            ..self
        })
    }

    /// Changes only the presently believed source anchor.
    pub fn with_source(
        self,
        source: EntityId,
    ) -> Result<Self, ContainmentTransferActivityStateError> {
        Self::new(
            self.item,
            source,
            self.destination,
            self.next_opportunity_generation,
            self.remaining_attempts,
        )
    }

    fn validates_successor(
        self,
        successor: Self,
    ) -> Result<(), ContainmentTransferActivityStateError> {
        if successor.item != self.item || successor.destination != self.destination {
            return Err(ContainmentTransferActivityStateError::ChangedDesiredCondition);
        }
        let unchanged = successor.next_opportunity_generation == self.next_opportunity_generation
            && successor.remaining_attempts == self.remaining_attempts;
        let advanced = self
            .next_opportunity_generation
            .get()
            .checked_add(1)
            .is_some_and(|generation| {
                successor.next_opportunity_generation.get() == generation
                    && self
                        .remaining_attempts
                        .checked_sub(1)
                        .is_some_and(|remaining| successor.remaining_attempts == remaining)
            });
        if !unchanged && !advanced {
            return Err(ContainmentTransferActivityStateError::InvalidOpportunityProgress);
        }
        Ok(())
    }

    fn write_canonical(self, writer: &mut CanonicalWriter) -> Result<(), CanonicalError> {
        writer.write_bytes(self.item.as_bytes())?;
        writer.write_bytes(self.source.as_bytes())?;
        writer.write_bytes(self.destination.as_bytes())?;
        writer.write_u64(self.next_opportunity_generation.get());
        writer.write_u32(self.remaining_attempts);
        Ok(())
    }
}

/// Next grounded control step in the first bounded travel method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TravelActivityStep {
    /// Offer one opportunity to pause the active relocation.
    Pause,
    /// Offer one opportunity to resume the paused relocation.
    Resume,
    /// Await modeled arrival meaning without owning process progress.
    AwaitArrival,
}

/// Why persistent travel-method state was structurally invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TravelActivityStateError {
    /// Travel requires distinct endpoints.
    SameEndpoint { location: EntityId },
    /// Opportunity generations begin at one.
    ZeroOpportunityGeneration,
    /// No further control action follows the awaiting-arrival state.
    AwaitingArrival,
    /// An update changed the travel endpoints.
    ChangedEndpoints,
    /// Step and generation did not advance together.
    InvalidOpportunityProgress,
}

impl fmt::Display for TravelActivityStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TravelActivityStateError {}

/// Concrete semantic method state retained by one travel activity.
///
/// Process identity, elapsed time, due moments, versions, and wake generations
/// deliberately remain absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TravelActivityState {
    source: EntityId,
    destination: EntityId,
    next_opportunity_generation: ActionOpportunityGeneration,
    step: TravelActivityStep,
}

impl TravelActivityState {
    /// Constructs the state retained after the activity's initial start
    /// opportunity has been opened.
    pub fn after_start_opened(
        source: EntityId,
        destination: EntityId,
        next_opportunity_generation: ActionOpportunityGeneration,
    ) -> Result<Self, TravelActivityStateError> {
        if source == destination {
            return Err(TravelActivityStateError::SameEndpoint { location: source });
        }
        if next_opportunity_generation.get() == 0 {
            return Err(TravelActivityStateError::ZeroOpportunityGeneration);
        }
        Ok(Self {
            source,
            destination,
            next_opportunity_generation,
            step: TravelActivityStep::Pause,
        })
    }

    #[must_use]
    pub const fn source(self) -> EntityId {
        self.source
    }

    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.destination
    }

    #[must_use]
    pub const fn next_opportunity_generation(self) -> ActionOpportunityGeneration {
        self.next_opportunity_generation
    }

    #[must_use]
    pub const fn step(self) -> TravelActivityStep {
        self.step
    }

    /// Advances the bounded method after opening its next control
    /// opportunity.
    pub fn after_opening_control(self) -> Result<Self, TravelActivityStateError> {
        let step = match self.step {
            TravelActivityStep::Pause => TravelActivityStep::Resume,
            TravelActivityStep::Resume => TravelActivityStep::AwaitArrival,
            TravelActivityStep::AwaitArrival => {
                return Err(TravelActivityStateError::AwaitingArrival);
            }
        };
        let generation = self
            .next_opportunity_generation
            .get()
            .checked_add(1)
            .ok_or(TravelActivityStateError::InvalidOpportunityProgress)?;
        Ok(Self {
            next_opportunity_generation: ActionOpportunityGeneration::new(generation),
            step,
            ..self
        })
    }

    fn validates_successor(self, successor: Self) -> Result<(), TravelActivityStateError> {
        if successor.source != self.source || successor.destination != self.destination {
            return Err(TravelActivityStateError::ChangedEndpoints);
        }
        if successor == self {
            return Ok(());
        }
        if self.after_opening_control().ok() == Some(successor) {
            Ok(())
        } else {
            Err(TravelActivityStateError::InvalidOpportunityProgress)
        }
    }

    fn write_canonical(self, writer: &mut CanonicalWriter) -> Result<(), CanonicalError> {
        writer.write_bytes(self.source.as_bytes())?;
        writer.write_bytes(self.destination.as_bytes())?;
        writer.write_u64(self.next_opportunity_generation.get());
        writer.write_discriminant(match self.step {
            TravelActivityStep::Pause => 0,
            TravelActivityStep::Resume => 1,
            TravelActivityStep::AwaitArrival => 2,
        });
        Ok(())
    }
}

/// Closed persistent state of the two implemented activity methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityState {
    /// Bounded containment restoration.
    ContainmentTransfer(ContainmentTransferActivityState),
    /// Bounded travel control followed by awaiting arrival.
    Travel(TravelActivityState),
}

impl ActivityState {
    #[must_use]
    pub const fn containment_transfer(self) -> Option<ContainmentTransferActivityState> {
        match self {
            Self::ContainmentTransfer(state) => Some(state),
            Self::Travel(_) => None,
        }
    }

    #[must_use]
    pub const fn travel(self) -> Option<TravelActivityState> {
        match self {
            Self::ContainmentTransfer(_) => None,
            Self::Travel(state) => Some(state),
        }
    }

    fn validates_successor(self, successor: Self) -> Result<(), ActivityStateTransitionError> {
        match (self, successor) {
            (Self::ContainmentTransfer(before), Self::ContainmentTransfer(after)) => before
                .validates_successor(after)
                .map_err(ActivityStateTransitionError::ContainmentTransfer),
            (Self::Travel(before), Self::Travel(after)) => before
                .validates_successor(after)
                .map_err(ActivityStateTransitionError::Travel),
            _ => Err(ActivityStateTransitionError::MethodChanged),
        }
    }

    fn write_canonical(self, writer: &mut CanonicalWriter) -> Result<(), CanonicalError> {
        match self {
            Self::ContainmentTransfer(state) => {
                writer.write_discriminant(0);
                state.write_canonical(writer)
            }
            Self::Travel(state) => {
                writer.write_discriminant(1);
                state.write_canonical(writer)
            }
        }
    }
}

impl From<ContainmentTransferActivityState> for ActivityState {
    fn from(state: ContainmentTransferActivityState) -> Self {
        Self::ContainmentTransfer(state)
    }
}

impl From<TravelActivityState> for ActivityState {
    fn from(state: TravelActivityState) -> Self {
        Self::Travel(state)
    }
}

/// Why an activity changed or invalidated its persistent method state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityStateTransitionError {
    /// The activity changed method families.
    MethodChanged,
    /// A containment method successor was invalid.
    ContainmentTransfer(ContainmentTransferActivityStateError),
    /// A travel method successor was invalid.
    Travel(TravelActivityStateError),
}

/// Durable lifecycle status of an activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActivityStatus {
    Active,
    Waiting,
    Suspended,
    Completed,
    Failed,
    Cancelled,
}

impl ActivityStatus {
    /// Returns whether no later lifecycle transition is legal.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// One legal requested transition of a persistent activity method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityTransition {
    Continue(ActivityState),
    Wait(ActivityState),
    Suspend,
    Resume(ActivityState),
    Complete,
    Fail,
    Cancel,
}

/// Compact diagnostic kind of an activity transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityTransitionKind {
    Continue,
    Wait,
    Suspend,
    Resume,
    Complete,
    Fail,
    Cancel,
}

impl ActivityTransition {
    const fn kind(self) -> ActivityTransitionKind {
        match self {
            Self::Continue(_) => ActivityTransitionKind::Continue,
            Self::Wait(_) => ActivityTransitionKind::Wait,
            Self::Suspend => ActivityTransitionKind::Suspend,
            Self::Resume(_) => ActivityTransitionKind::Resume,
            Self::Complete => ActivityTransitionKind::Complete,
            Self::Fail => ActivityTransitionKind::Fail,
            Self::Cancel => ActivityTransitionKind::Cancel,
        }
    }
}

/// Why an activity transition could not construct a successor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityTransitionError {
    StaleVersion {
        expected: ActivityVersion,
        actual: ActivityVersion,
    },
    InvalidStatus {
        current: ActivityStatus,
        transition: ActivityTransitionKind,
    },
    InvalidControllerState(ActivityStateTransitionError),
    VersionOverflow,
}

impl fmt::Display for ActivityTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ActivityTransitionError {}

/// Immutable accepted persistent activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Activity {
    id: ActivityId,
    actor: ActorId,
    intent: IntentId,
    generation: ActivityGeneration,
    version: ActivityVersion,
    controller: ActivityControllerId,
    state_schema: ActivityStateSchemaId,
    status: ActivityStatus,
    state: ActivityState,
}

impl Activity {
    /// Starts one active activity owned by an existing intent.
    #[must_use]
    pub fn start(
        actor: ActorId,
        intent: IntentId,
        generation: ActivityGeneration,
        controller: ActivityControllerId,
        state_schema: ActivityStateSchemaId,
        state: impl Into<ActivityState>,
    ) -> Self {
        let state = state.into();
        Self {
            id: ActivityId::derive(actor, intent, generation),
            actor,
            intent,
            generation,
            version: ActivityVersion::INITIAL,
            controller,
            state_schema,
            status: ActivityStatus::Active,
            state,
        }
    }

    #[must_use]
    pub const fn id(self) -> ActivityId {
        self.id
    }

    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    #[must_use]
    pub const fn intent(self) -> IntentId {
        self.intent
    }

    #[must_use]
    pub const fn generation(self) -> ActivityGeneration {
        self.generation
    }

    #[must_use]
    pub const fn version(self) -> ActivityVersion {
        self.version
    }

    /// Returns the selected activity-controller implementation.
    #[must_use]
    pub const fn controller(self) -> ActivityControllerId {
        self.controller
    }

    /// Returns the schema governing persistent controller state.
    #[must_use]
    pub const fn state_schema(self) -> ActivityStateSchemaId {
        self.state_schema
    }

    #[must_use]
    pub const fn status(self) -> ActivityStatus {
        self.status
    }

    #[must_use]
    pub const fn state(self) -> ActivityState {
        self.state
    }

    /// Constructs a version-checked legal activity successor.
    pub fn transition(
        self,
        expected_version: ActivityVersion,
        transition: ActivityTransition,
    ) -> Result<Self, ActivityTransitionError> {
        if expected_version != self.version {
            return Err(ActivityTransitionError::StaleVersion {
                expected: expected_version,
                actual: self.version,
            });
        }
        let (status, state) = match (self.status, transition) {
            (ActivityStatus::Active, ActivityTransition::Continue(state)) => {
                (ActivityStatus::Active, state)
            }
            (ActivityStatus::Active, ActivityTransition::Wait(state)) => {
                (ActivityStatus::Waiting, state)
            }
            (ActivityStatus::Active, ActivityTransition::Suspend) => {
                (ActivityStatus::Suspended, self.state)
            }
            (
                ActivityStatus::Waiting | ActivityStatus::Suspended,
                ActivityTransition::Resume(state),
            ) => (ActivityStatus::Active, state),
            (
                ActivityStatus::Active | ActivityStatus::Waiting | ActivityStatus::Suspended,
                ActivityTransition::Complete,
            ) => (ActivityStatus::Completed, self.state),
            (
                ActivityStatus::Active | ActivityStatus::Waiting | ActivityStatus::Suspended,
                ActivityTransition::Fail,
            ) => (ActivityStatus::Failed, self.state),
            (
                ActivityStatus::Active | ActivityStatus::Waiting | ActivityStatus::Suspended,
                ActivityTransition::Cancel,
            ) => (ActivityStatus::Cancelled, self.state),
            (current, transition) => {
                return Err(ActivityTransitionError::InvalidStatus {
                    current,
                    transition: transition.kind(),
                });
            }
        };
        self.state
            .validates_successor(state)
            .map_err(ActivityTransitionError::InvalidControllerState)?;
        let version = self
            .version
            .checked_next()
            .ok_or(ActivityTransitionError::VersionOverflow)?;
        Ok(Self {
            version,
            status,
            state,
            ..self
        })
    }

    fn has_valid_id(self) -> bool {
        self.id == ActivityId::derive(self.actor, self.intent, self.generation)
    }
}

/// The single foreground activity, when any, selected for one actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivityFocus {
    actor: ActorId,
    activity: ActivityId,
}

impl ActivityFocus {
    #[must_use]
    pub const fn new(actor: ActorId, activity: ActivityId) -> Self {
        Self { actor, activity }
    }

    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    #[must_use]
    pub const fn activity(self) -> ActivityId {
        self.activity
    }
}

/// Why a complete agency state violated ownership or lifecycle integrity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgencyStateError {
    DuplicateIntent {
        intent: IntentId,
    },
    DuplicateIntentGeneration {
        actor: ActorId,
        generation: IntentGeneration,
    },
    InvalidIntentId {
        intent: IntentId,
    },
    DuplicateActivity {
        activity: ActivityId,
    },
    DuplicateActivityGeneration {
        actor: ActorId,
        generation: ActivityGeneration,
    },
    InvalidActivityId {
        activity: ActivityId,
    },
    MissingOwningIntent {
        activity: ActivityId,
        intent: IntentId,
    },
    ActivityActorMismatch {
        activity: ActivityId,
    },
    ActivityDesiredConditionMismatch {
        activity: ActivityId,
    },
    LiveActivityHasTerminalIntent {
        activity: ActivityId,
        intent: IntentId,
    },
    DuplicateFocus {
        actor: ActorId,
    },
    MissingFocusedActivity {
        actor: ActorId,
        activity: ActivityId,
    },
    FocusActorMismatch {
        actor: ActorId,
        activity: ActivityId,
    },
    FocusedActivityNotActive {
        actor: ActorId,
        activity: ActivityId,
    },
    FocusedIntentNotActive {
        actor: ActorId,
        intent: IntentId,
    },
}

impl fmt::Display for AgencyStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AgencyStateError {}

/// Why an owner-local agency successor could not be constructed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgencyTransitionError {
    MissingIntent {
        intent: IntentId,
    },
    MissingActivity {
        activity: ActivityId,
    },
    DuplicateIntentGeneration {
        actor: ActorId,
        generation: IntentGeneration,
    },
    DuplicateActivityGeneration {
        actor: ActorId,
        generation: ActivityGeneration,
    },
    StaleFocus {
        actor: ActorId,
        expected: Option<ActivityId>,
        actual: Option<ActivityId>,
    },
    Intent(IntentTransitionError),
    Activity(ActivityTransitionError),
    InvalidSuccessor(AgencyStateError),
}

impl fmt::Display for AgencyTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AgencyTransitionError {}

/// Canonical identity of accepted agency state.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgencyStateDigest(ContentDigest);

impl AgencyStateDigest {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0.into_bytes()
    }
}

impl fmt::Display for AgencyStateDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for AgencyStateDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "AgencyStateDigest({self})")
    }
}

/// Immutable accepted intents, activities, and foreground focus.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgencyState {
    intents: Vec<Intent>,
    activities: Vec<Activity>,
    focus: Vec<ActivityFocus>,
}

impl AgencyState {
    /// Constructs the canonical empty agency state.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            intents: Vec::new(),
            activities: Vec::new(),
            focus: Vec::new(),
        }
    }

    /// Validates and canonicalizes a complete agency state.
    pub fn new(
        mut intents: Vec<Intent>,
        mut activities: Vec<Activity>,
        mut focus: Vec<ActivityFocus>,
    ) -> Result<Self, AgencyStateError> {
        intents.sort_by_key(|intent| intent.id);
        if let Some(intent) = adjacent_duplicate_by(&intents, |intent| intent.id) {
            return Err(AgencyStateError::DuplicateIntent { intent });
        }
        let mut intent_generations: Vec<_> = intents
            .iter()
            .map(|intent| (intent.actor, intent.generation))
            .collect();
        intent_generations.sort();
        if let Some((actor, generation)) = adjacent_duplicate(&intent_generations) {
            return Err(AgencyStateError::DuplicateIntentGeneration { actor, generation });
        }
        for intent in &intents {
            if !intent.has_valid_id() {
                return Err(AgencyStateError::InvalidIntentId { intent: intent.id });
            }
        }

        activities.sort_by_key(|activity| activity.id);
        if let Some(activity) = adjacent_duplicate_by(&activities, |activity| activity.id) {
            return Err(AgencyStateError::DuplicateActivity { activity });
        }
        let mut activity_generations: Vec<_> = activities
            .iter()
            .map(|activity| (activity.actor, activity.generation))
            .collect();
        activity_generations.sort();
        if let Some((actor, generation)) = adjacent_duplicate(&activity_generations) {
            return Err(AgencyStateError::DuplicateActivityGeneration { actor, generation });
        }
        for activity in &activities {
            if !activity.has_valid_id() {
                return Err(AgencyStateError::InvalidActivityId {
                    activity: activity.id,
                });
            }
            let Some(intent) = find_intent(&intents, activity.intent) else {
                return Err(AgencyStateError::MissingOwningIntent {
                    activity: activity.id,
                    intent: activity.intent,
                });
            };
            if activity.actor != intent.actor {
                return Err(AgencyStateError::ActivityActorMismatch {
                    activity: activity.id,
                });
            }
            let state_matches_intent = match (intent.desired, activity.state) {
                (
                    DesiredCondition::ItemContainedIn { item, container },
                    ActivityState::ContainmentTransfer(state),
                ) => state.item == item && state.destination == container,
                (DesiredCondition::ActorAt { location }, ActivityState::Travel(state)) => {
                    state.destination == location
                }
                _ => false,
            };
            if !state_matches_intent {
                return Err(AgencyStateError::ActivityDesiredConditionMismatch {
                    activity: activity.id,
                });
            }
            if !activity.status.is_terminal() && intent.status.is_terminal() {
                return Err(AgencyStateError::LiveActivityHasTerminalIntent {
                    activity: activity.id,
                    intent: intent.id,
                });
            }
        }

        focus.sort_by_key(|focus| focus.actor);
        if let Some(actor) = adjacent_duplicate_by(&focus, |focus| focus.actor) {
            return Err(AgencyStateError::DuplicateFocus { actor });
        }
        for focus in &focus {
            let Some(activity) = find_activity(&activities, focus.activity) else {
                return Err(AgencyStateError::MissingFocusedActivity {
                    actor: focus.actor,
                    activity: focus.activity,
                });
            };
            if activity.actor != focus.actor {
                return Err(AgencyStateError::FocusActorMismatch {
                    actor: focus.actor,
                    activity: focus.activity,
                });
            }
            if activity.status != ActivityStatus::Active {
                return Err(AgencyStateError::FocusedActivityNotActive {
                    actor: focus.actor,
                    activity: focus.activity,
                });
            }
            let intent = find_intent(&intents, activity.intent)
                .unwrap_or_else(|| unreachable!("activity ownership was checked above"));
            if intent.status != IntentStatus::Active {
                return Err(AgencyStateError::FocusedIntentNotActive {
                    actor: focus.actor,
                    intent: intent.id,
                });
            }
        }

        Ok(Self {
            intents,
            activities,
            focus,
        })
    }

    #[must_use]
    pub fn intents(&self) -> &[Intent] {
        &self.intents
    }

    #[must_use]
    pub fn activities(&self) -> &[Activity] {
        &self.activities
    }

    #[must_use]
    pub fn focus(&self) -> &[ActivityFocus] {
        &self.focus
    }

    #[must_use]
    pub fn intent(&self, id: IntentId) -> Option<&Intent> {
        find_intent(&self.intents, id)
    }

    #[must_use]
    pub fn activity(&self, id: ActivityId) -> Option<&Activity> {
        find_activity(&self.activities, id)
    }

    #[must_use]
    pub fn focused_activity(&self, actor: ActorId) -> Option<ActivityId> {
        self.focus
            .binary_search_by_key(&actor, |focus| focus.actor)
            .ok()
            .map(|index| self.focus[index].activity)
    }

    /// Constructs a successor with one newly adopted intent.
    pub fn adopt_intent(&self, intent: Intent) -> Result<Self, AgencyTransitionError> {
        if self.intents.iter().any(|existing| {
            existing.actor == intent.actor && existing.generation == intent.generation
        }) {
            return Err(AgencyTransitionError::DuplicateIntentGeneration {
                actor: intent.actor,
                generation: intent.generation,
            });
        }
        let mut intents = self.intents.clone();
        intents.push(intent);
        Self::new(intents, self.activities.clone(), self.focus.clone())
            .map_err(AgencyTransitionError::InvalidSuccessor)
    }

    /// Constructs a successor with one newly started activity and optional
    /// foreground focus.
    pub fn start_activity(
        &self,
        activity: Activity,
        take_focus: bool,
    ) -> Result<Self, AgencyTransitionError> {
        if self.activities.iter().any(|existing| {
            existing.actor == activity.actor && existing.generation == activity.generation
        }) {
            return Err(AgencyTransitionError::DuplicateActivityGeneration {
                actor: activity.actor,
                generation: activity.generation,
            });
        }
        let mut activities = self.activities.clone();
        activities.push(activity);
        let mut focus = self.focus.clone();
        if take_focus {
            match focus.binary_search_by_key(&activity.actor, |focus| focus.actor) {
                Ok(index) => focus[index] = ActivityFocus::new(activity.actor, activity.id),
                Err(index) => {
                    focus.insert(index, ActivityFocus::new(activity.actor, activity.id));
                }
            }
        }
        Self::new(self.intents.clone(), activities, focus)
            .map_err(AgencyTransitionError::InvalidSuccessor)
    }

    /// Constructs a checked successor for one intent transition.
    pub fn transition_intent(
        &self,
        id: IntentId,
        expected_version: IntentVersion,
        transition: IntentTransition,
    ) -> Result<Self, AgencyTransitionError> {
        let Some(index) = self
            .intents
            .binary_search_by_key(&id, |intent| intent.id)
            .ok()
        else {
            return Err(AgencyTransitionError::MissingIntent { intent: id });
        };
        let next = self.intents[index]
            .transition(expected_version, transition)
            .map_err(AgencyTransitionError::Intent)?;
        let mut intents = self.intents.clone();
        intents[index] = next;
        let mut focus = self.focus.clone();
        if next.status != IntentStatus::Active {
            focus.retain(|entry| {
                self.activity(entry.activity)
                    .is_none_or(|activity| activity.intent != id)
            });
        }
        Self::new(intents, self.activities.clone(), focus)
            .map_err(AgencyTransitionError::InvalidSuccessor)
    }

    /// Constructs a checked successor for one activity transition.
    pub fn transition_activity(
        &self,
        id: ActivityId,
        expected_version: ActivityVersion,
        transition: ActivityTransition,
    ) -> Result<Self, AgencyTransitionError> {
        let Some(index) = self
            .activities
            .binary_search_by_key(&id, |activity| activity.id)
            .ok()
        else {
            return Err(AgencyTransitionError::MissingActivity { activity: id });
        };
        let next = self.activities[index]
            .transition(expected_version, transition)
            .map_err(AgencyTransitionError::Activity)?;
        let mut activities = self.activities.clone();
        activities[index] = next;
        let mut focus = self.focus.clone();
        if next.status != ActivityStatus::Active {
            focus.retain(|entry| entry.activity != id);
        }
        Self::new(self.intents.clone(), activities, focus)
            .map_err(AgencyTransitionError::InvalidSuccessor)
    }

    /// Compare-and-sets one actor's optional foreground activity.
    pub fn set_focus(
        &self,
        actor: ActorId,
        expected: Option<ActivityId>,
        replacement: Option<ActivityId>,
    ) -> Result<Self, AgencyTransitionError> {
        let actual = self.focused_activity(actor);
        if actual != expected {
            return Err(AgencyTransitionError::StaleFocus {
                actor,
                expected,
                actual,
            });
        }
        let mut focus = self.focus.clone();
        match (
            focus.binary_search_by_key(&actor, |entry| entry.actor),
            replacement,
        ) {
            (Ok(index), Some(activity)) => {
                focus[index] = ActivityFocus::new(actor, activity);
            }
            (Ok(index), None) => {
                focus.remove(index);
            }
            (Err(index), Some(activity)) => {
                focus.insert(index, ActivityFocus::new(actor, activity));
            }
            (Err(_), None) => {}
        }
        Self::new(self.intents.clone(), self.activities.clone(), focus)
            .map_err(AgencyTransitionError::InvalidSuccessor)
    }

    /// Returns the canonical agency-state identity.
    #[must_use]
    pub fn digest(&self) -> AgencyStateDigest {
        AgencyStateDigest(ContentDigest::of_canonical(&self.canonical_preimage()))
    }

    fn canonical_preimage(&self) -> CanonicalBytes {
        let encoded = (|| -> Result<_, CanonicalError> {
            let mut writer = CanonicalWriter::new(AGENCY_STATE_CANONICAL_DOMAIN);
            writer.write_u16(AGENCY_STATE_SCHEMA_VERSION);
            writer.write_sequence(&self.intents, |writer, intent| {
                writer.write_bytes(intent.id.as_bytes())?;
                writer.write_bytes(intent.actor.as_bytes())?;
                writer.write_u64(intent.generation.get());
                writer.write_u64(intent.version.get());
                intent.desired.write_canonical(writer)?;
                writer.write_discriminant(intent_status_tag(intent.status));
                Ok(())
            })?;
            writer.write_sequence(&self.activities, |writer, activity| {
                writer.write_bytes(activity.id.as_bytes())?;
                writer.write_bytes(activity.actor.as_bytes())?;
                writer.write_bytes(activity.intent.as_bytes())?;
                writer.write_u64(activity.generation.get());
                writer.write_u64(activity.version.get());
                writer.write_bytes(activity.controller.as_bytes())?;
                writer.write_bytes(activity.state_schema.as_bytes())?;
                writer.write_discriminant(activity_status_tag(activity.status));
                activity.state.write_canonical(writer)
            })?;
            writer.write_sequence(&self.focus, |writer, focus| {
                writer.write_bytes(focus.actor.as_bytes())?;
                writer.write_bytes(focus.activity.as_bytes())
            })?;
            Ok(writer.finish())
        })();
        match encoded {
            Ok(bytes) => bytes,
            Err(error) => unreachable!("allocated agency state must be canonical: {error}"),
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

fn find_intent(intents: &[Intent], id: IntentId) -> Option<&Intent> {
    intents
        .binary_search_by_key(&id, |intent| intent.id)
        .ok()
        .map(|index| &intents[index])
}

fn find_activity(activities: &[Activity], id: ActivityId) -> Option<&Activity> {
    activities
        .binary_search_by_key(&id, |activity| activity.id)
        .ok()
        .map(|index| &activities[index])
}

fn adjacent_duplicate<T: Copy + PartialEq>(values: &[T]) -> Option<T> {
    adjacent_duplicate_by(values, |value| *value)
}

fn adjacent_duplicate_by<T, K: Copy + PartialEq>(values: &[T], key: impl Fn(&T) -> K) -> Option<K> {
    values.windows(2).find_map(|pair| {
        let previous = key(&pair[0]);
        let current = key(&pair[1]);
        (previous == current).then_some(current)
    })
}

fn write_hex(formatter: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}
