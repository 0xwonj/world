use core::fmt;
use core::num::{NonZeroU32, NonZeroU64};

use world_core::{
    ActorId, CanonicalBytes, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest,
    EntityId,
};

use crate::accepted::{
    Activity, ActivityId, ActivityState, ActivityVersion, RelocationRouteId, TravelActivityStep,
};

/// Canonical schema version of [`ActionOpportunityId`].
pub const ACTION_OPPORTUNITY_ID_SCHEMA_VERSION: u16 = 2;

/// Canonical schema version of [`ActionOpportunity`].
pub const ACTION_OPPORTUNITY_SCHEMA_VERSION: u16 = 5;

/// Canonical schema version of [`ActionEvaluationInvocationId`].
pub const ACTION_EVALUATION_INVOCATION_ID_SCHEMA_VERSION: u16 = 1;

const ACTION_OPPORTUNITY_ID_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-opportunity-id-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("action opportunity identity domain must be valid"),
    };

const ACTION_OPPORTUNITY_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-opportunity-v5") {
        Ok(domain) => domain,
        Err(_) => panic!("action opportunity domain must be valid"),
    };

const ACTION_EVALUATION_INVOCATION_ID_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-evaluation-invocation-id-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action evaluation invocation identity domain must be valid"),
    };

/// Actor-safe semantic cause of a reaction-sponsored action opportunity.
///
/// The reaction producer owns derivation of these bytes. They must identify
/// actor-visible semantics rather than an authority record, trigger, revision,
/// or another private execution value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorReactionCause([u8; 32]);

impl ActorReactionCause {
    /// Constructs a semantic cause decoded or derived by its owner.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the exact semantic-cause bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the cause and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Version binding for an activity-sponsored action opportunity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActivitySponsor {
    activity: ActivityId,
    expected_version: ActivityVersion,
}

impl ActivitySponsor {
    /// Binds an opportunity to the exact activity version that opened it.
    #[must_use]
    pub const fn new(activity: ActivityId, expected_version: ActivityVersion) -> Self {
        Self {
            activity,
            expected_version,
        }
    }

    /// Returns the sponsoring activity.
    #[must_use]
    pub const fn activity(self) -> ActivityId {
        self.activity
    }

    /// Returns the activity version expected by continuation handling.
    #[must_use]
    pub const fn expected_version(self) -> ActivityVersion {
        self.expected_version
    }
}

/// Explicit lifecycle sponsor of an action opportunity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionSponsor {
    /// A versioned persistent activity opened the opportunity.
    Activity(ActivitySponsor),
    /// An actor-visible semantic reaction opened the opportunity.
    ActorReaction(ActorReactionCause),
}

impl ActionSponsor {
    /// Constructs an activity-sponsored form.
    #[must_use]
    pub const fn activity(activity: ActivityId, expected_version: ActivityVersion) -> Self {
        Self::Activity(ActivitySponsor::new(activity, expected_version))
    }

    /// Constructs the initial reaction-sponsored form.
    #[must_use]
    pub const fn actor_reaction(cause: ActorReactionCause) -> Self {
        Self::ActorReaction(cause)
    }

    const fn canonical_tag(self) -> u32 {
        match self {
            Self::ActorReaction(_) => 0,
            Self::Activity(_) => 1,
        }
    }

    fn write_canonical(self, writer: &mut CanonicalWriter) -> Result<(), CanonicalError> {
        writer.write_discriminant(self.canonical_tag());
        match self {
            Self::Activity(sponsor) => {
                writer.write_bytes(sponsor.activity.as_bytes())?;
                writer.write_u64(sponsor.expected_version.get());
                Ok(())
            }
            Self::ActorReaction(cause) => writer.write_bytes(cause.as_bytes()),
        }
    }
}

/// Visibility-stable actor-local generation of one action opportunity.
///
/// The sponsoring lifecycle owns monotonicity. This scalar proves only the
/// exact representation used by the opportunity identity preimage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionOpportunityGeneration(u64);

impl ActionOpportunityGeneration {
    /// Constructs a sponsor-scoped generation.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the exact generation scalar.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Stable actor-safe identity of one action opportunity.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionOpportunityId([u8; 32]);

impl ActionOpportunityId {
    /// Constructs an identity decoded from durable model data.
    ///
    /// A decoder must verify it against the actor, sponsor, and generation
    /// before trusting the enclosing opportunity.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Derives the canonical actor-safe identity for an opportunity.
    #[must_use]
    pub fn derive(
        actor: ActorId,
        sponsor: ActionSponsor,
        generation: ActionOpportunityGeneration,
    ) -> Self {
        Self(
            ContentDigest::of_canonical(&action_opportunity_id_preimage(
                actor, sponsor, generation,
            ))
            .into_bytes(),
        )
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

impl fmt::Display for ActionOpportunityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for ActionOpportunityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ActionOpportunityId({self})")
    }
}

/// Actor-visible evaluation generation within one stable opportunity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionEvaluationGeneration(NonZeroU64);

impl ActionEvaluationGeneration {
    /// Generation of the first evaluation invocation.
    pub const INITIAL: Self = match NonZeroU64::new(1) {
        Some(value) => Self(value),
        None => unreachable!(),
    };

    /// Constructs a nonzero evaluation generation.
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the exact nonzero generation.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Returns the next evaluation generation, or `None` on overflow.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.get().checked_add(1) {
            Some(value) => Self::new(value),
            None => None,
        }
    }
}

/// Stable actor-safe identity of one action-policy evaluation invocation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionEvaluationInvocationId([u8; 32]);

impl ActionEvaluationInvocationId {
    /// Constructs an identity decoded from durable model data.
    ///
    /// A decoder must verify the complete derivation preimage before trusting
    /// an enclosing opportunity or invocation record.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Derives one actor-safe invocation identity.
    ///
    /// The fixed byte inputs are owned by higher layers. They must encode the
    /// selected policy semantics and complete actor-visible action input,
    /// never private runtime provenance.
    #[must_use]
    pub fn derive(
        opportunity: ActionOpportunityId,
        generation: ActionEvaluationGeneration,
        policy_semantics: [u8; 32],
        action_input_fingerprint: [u8; 32],
    ) -> Self {
        Self(
            ContentDigest::of_canonical(&action_evaluation_invocation_id_preimage(
                opportunity,
                generation,
                policy_semantics,
                action_input_fingerprint,
            ))
            .into_bytes(),
        )
    }

    /// Returns the exact invocation-identity bytes.
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

impl fmt::Display for ActionEvaluationInvocationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for ActionEvaluationInvocationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ActionEvaluationInvocationId({self})")
    }
}

/// Canonical identity of one complete action-opportunity value.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionOpportunityDigest(ContentDigest);

impl ActionOpportunityDigest {
    /// Returns the exact digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Consumes the digest and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0.into_bytes()
    }
}

impl fmt::Display for ActionOpportunityDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for ActionOpportunityDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ActionOpportunityDigest({self})")
    }
}

/// Version of one durable action-opportunity record.
///
/// Opportunities begin at [`Self::INITIAL`] and advance on every durable
/// evaluation-state or terminal-state transition. Runtime compare-and-set
/// provides authoritative transition serialization; this value provides the
/// checked protocol coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionOpportunityVersion(NonZeroU64);

impl ActionOpportunityVersion {
    /// Version of a newly opened opportunity.
    pub const INITIAL: Self = match NonZeroU64::new(1) {
        Some(value) => Self(value),
        None => unreachable!(),
    };

    /// Constructs a nonzero opportunity version.
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the exact nonzero version.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Returns the next opportunity version, or `None` on overflow.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.get().checked_add(1) {
            Some(value) => Self::new(value),
            None => None,
        }
    }
}

/// Exact relocation interaction that may be grounded by one opportunity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RelocationInteraction {
    /// Begin relocation along one directed route.
    Start(RelocationRouteId),
    /// Suspend progress along one directed route.
    Pause(RelocationRouteId),
    /// Continue paused progress along one directed route.
    Resume(RelocationRouteId),
}

impl RelocationInteraction {
    /// Returns the exact route whose start or control is permitted.
    #[must_use]
    pub const fn route(self) -> RelocationRouteId {
        match self {
            Self::Start(route) | Self::Pause(route) | Self::Resume(route) => route,
        }
    }
}

/// Actor-visible endpoint anchors for one scoped relocation interaction.
///
/// The anchors describe what the opportunity presents to the actor. They do
/// not assert that the route still exists or that its authoritative endpoints
/// match; runtime authority validates those facts when an interaction is
/// submitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelocationInteractionAnchor {
    interaction: RelocationInteraction,
    source: EntityId,
    destination: EntityId,
}

impl RelocationInteractionAnchor {
    /// Constructs one actor-visible relocation anchor.
    #[must_use]
    pub const fn new(
        interaction: RelocationInteraction,
        source: EntityId,
        destination: EntityId,
    ) -> Self {
        Self {
            interaction,
            source,
            destination,
        }
    }

    /// Returns the exact start or process-control interaction.
    #[must_use]
    pub const fn interaction(self) -> RelocationInteraction {
        self.interaction
    }

    /// Returns the actor-visible departure entity.
    #[must_use]
    pub const fn source(self) -> EntityId {
        self.source
    }

    /// Returns the actor-visible arrival entity.
    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.destination
    }
}

/// Why a relocation interaction scope could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocationInteractionScopeError {
    /// No route start or process control was supplied.
    Empty,
    /// One exact interaction appeared more than once.
    DuplicateInteraction {
        /// Repeated interaction.
        interaction: RelocationInteraction,
    },
    /// Candidate generation was given no positive work budget.
    ZeroCandidateLimit,
}

impl fmt::Display for RelocationInteractionScopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid relocation interaction scope: {self:?}")
    }
}

impl std::error::Error for RelocationInteractionScopeError {}

/// Bounded actor-visible frame for starting or controlling relocation.
///
/// Route and process legality remain private runtime checks. Pairing every
/// verb with its route prevents meaningless cross-products between starts and
/// controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelocationInteractionScope {
    anchors: Vec<RelocationInteractionAnchor>,
    candidate_limit: NonZeroU32,
}

impl RelocationInteractionScope {
    /// Validates and canonicalizes one relocation interaction frame.
    pub fn new(
        mut anchors: Vec<RelocationInteractionAnchor>,
        candidate_limit: u32,
    ) -> Result<Self, RelocationInteractionScopeError> {
        let candidate_limit = NonZeroU32::new(candidate_limit)
            .ok_or(RelocationInteractionScopeError::ZeroCandidateLimit)?;
        if anchors.is_empty() {
            return Err(RelocationInteractionScopeError::Empty);
        }
        anchors.sort_by_key(|anchor| anchor.interaction());
        if let Some(interaction) = anchors.windows(2).find_map(|pair| {
            (pair[0].interaction() == pair[1].interaction()).then_some(pair[0].interaction())
        }) {
            return Err(RelocationInteractionScopeError::DuplicateInteraction { interaction });
        }
        Ok(Self {
            anchors,
            candidate_limit,
        })
    }

    /// Returns actor-visible anchors in canonical kind-and-route order.
    #[must_use]
    pub fn anchors(&self) -> &[RelocationInteractionAnchor] {
        &self.anchors
    }

    /// Returns the positive upper bound on grounded candidates.
    #[must_use]
    pub const fn candidate_limit(&self) -> u32 {
        self.candidate_limit.get()
    }

    /// Returns whether this scope permits one exact interaction.
    #[must_use]
    pub fn permits(&self, interaction: RelocationInteraction) -> bool {
        self.anchors
            .binary_search_by_key(&interaction, |anchor| anchor.interaction())
            .is_ok()
    }
}

/// Why a containment interaction scope could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainmentInteractionScopeError {
    /// No exact item was supplied.
    EmptyItems,
    /// One exact item appeared more than once.
    DuplicateItem {
        /// Repeated item entity.
        item: EntityId,
    },
    /// No destination interaction anchor was supplied.
    EmptyDestinations,
    /// One destination anchor appeared more than once.
    DuplicateDestination {
        /// Repeated destination entity.
        destination: EntityId,
    },
    /// The source was also supplied as a destination.
    SourceIsDestination {
        /// Entity reused in both roles.
        container: EntityId,
    },
    /// Candidate generation was given no positive work budget.
    ZeroCandidateLimit,
}

impl fmt::Display for ContainmentInteractionScopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyItems => {
                formatter.write_str("containment interaction scope has no exact items")
            }
            Self::DuplicateItem { item } => write!(
                formatter,
                "containment interaction scope repeats item {}",
                Hex(item.as_bytes())
            ),
            Self::EmptyDestinations => {
                formatter.write_str("containment interaction scope has no destinations")
            }
            Self::DuplicateDestination { destination } => write!(
                formatter,
                "containment interaction scope repeats destination {}",
                Hex(destination.as_bytes())
            ),
            Self::SourceIsDestination { container } => write!(
                formatter,
                "containment interaction source {} is also a destination",
                Hex(container.as_bytes())
            ),
            Self::ZeroCandidateLimit => {
                formatter.write_str("containment interaction candidate limit must be positive")
            }
        }
    }
}

impl std::error::Error for ContainmentInteractionScopeError {}

/// Bounded interaction anchors for the initial containment-transfer grounder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainmentInteractionScope {
    source: EntityId,
    destinations: Vec<EntityId>,
    items: Vec<EntityId>,
    candidate_limit: NonZeroU32,
}

impl ContainmentInteractionScope {
    /// Validates and canonicalizes containment interaction anchors.
    pub fn new(
        source: EntityId,
        mut destinations: Vec<EntityId>,
        mut items: Vec<EntityId>,
        candidate_limit: u32,
    ) -> Result<Self, ContainmentInteractionScopeError> {
        let candidate_limit = NonZeroU32::new(candidate_limit)
            .ok_or(ContainmentInteractionScopeError::ZeroCandidateLimit)?;
        if items.is_empty() {
            return Err(ContainmentInteractionScopeError::EmptyItems);
        }
        if destinations.is_empty() {
            return Err(ContainmentInteractionScopeError::EmptyDestinations);
        }

        items.sort();
        if let Some(item) = items
            .windows(2)
            .find_map(|pair| (pair[0] == pair[1]).then_some(pair[0]))
        {
            return Err(ContainmentInteractionScopeError::DuplicateItem { item });
        }
        destinations.sort();
        if let Some(destination) = destinations
            .windows(2)
            .find_map(|pair| (pair[0] == pair[1]).then_some(pair[0]))
        {
            return Err(ContainmentInteractionScopeError::DuplicateDestination { destination });
        }
        if destinations.binary_search(&source).is_ok() {
            return Err(ContainmentInteractionScopeError::SourceIsDestination {
                container: source,
            });
        }

        Ok(Self {
            source,
            destinations,
            items,
            candidate_limit,
        })
    }

    /// Returns the actor-controlled source interaction anchor.
    #[must_use]
    pub const fn source(&self) -> EntityId {
        self.source
    }

    /// Returns destination anchors in canonical entity-identity order.
    #[must_use]
    pub fn destinations(&self) -> &[EntityId] {
        &self.destinations
    }

    /// Returns the exact allowed items in canonical entity-identity order.
    #[must_use]
    pub fn items(&self) -> &[EntityId] {
        &self.items
    }

    /// Returns whether the scope permits one exact item.
    #[must_use]
    pub fn permits_item(&self, item: EntityId) -> bool {
        self.items.binary_search(&item).is_ok()
    }

    /// Returns the positive upper bound on generated candidates.
    #[must_use]
    pub const fn candidate_limit(&self) -> u32 {
        self.candidate_limit.get()
    }
}

/// Closed interaction family owned by one action opportunity.
///
/// Each variant retains the exact bounded inputs needed by its concrete
/// grounder. Adding a new family requires an explicit model, context, engine,
/// and runtime authority path rather than an untyped action payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionInteractionScope {
    /// Exact item-transfer interaction anchors.
    Containment(ContainmentInteractionScope),
    /// Exact route starts and relocation-process controls.
    Relocation(RelocationInteractionScope),
}

impl ActionInteractionScope {
    /// Constructs a containment-transfer interaction.
    #[must_use]
    pub const fn containment(scope: ContainmentInteractionScope) -> Self {
        Self::Containment(scope)
    }

    /// Constructs a relocation interaction.
    #[must_use]
    pub const fn relocation(scope: RelocationInteractionScope) -> Self {
        Self::Relocation(scope)
    }

    /// Returns the contained transfer scope when this is a containment
    /// interaction.
    #[must_use]
    pub const fn containment_scope(&self) -> Option<&ContainmentInteractionScope> {
        match self {
            Self::Containment(scope) => Some(scope),
            Self::Relocation(_) => None,
        }
    }

    /// Returns the contained relocation scope when this is a relocation
    /// interaction.
    #[must_use]
    pub const fn relocation_scope(&self) -> Option<&RelocationInteractionScope> {
        match self {
            Self::Containment(_) => None,
            Self::Relocation(scope) => Some(scope),
        }
    }

    /// Returns the positive candidate budget declared by the concrete scope.
    #[must_use]
    pub const fn candidate_limit(&self) -> u32 {
        match self {
            Self::Containment(scope) => scope.candidate_limit(),
            Self::Relocation(scope) => scope.candidate_limit(),
        }
    }
}

/// Terminal semantic disposition of one action opportunity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionOpportunityDisposition {
    /// A selected candidate was privately lowered and submitted for attempt.
    ActionSubmitted,
    /// Complete actor-relative input contained no applicable selection.
    NoApplicableAction,
    /// The policy failed under its configured bounded failure rule.
    Failed,
}

/// Durable logical state of one action opportunity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionOpportunityState {
    /// Evaluation may begin or one terminal disposition may consume the opportunity.
    Open,
    /// One exact deferred evaluation owns the opportunity.
    WaitingForEvaluation(ActionEvaluationInvocationId),
    /// The opportunity has reached its one terminal disposition.
    Consumed(ActionOpportunityDisposition),
}

/// Why a checked action-opportunity transition could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionOpportunityTransitionError {
    /// The caller did not name the opportunity's current version.
    StaleVersion {
        /// Version required by the caller.
        expected: ActionOpportunityVersion,
        /// Version carried by the current value.
        actual: ActionOpportunityVersion,
    },
    /// A terminal opportunity cannot be consumed again.
    AlreadyConsumed {
        /// Existing terminal disposition.
        disposition: ActionOpportunityDisposition,
    },
    /// Evaluation already owns the opportunity.
    EvaluationAlreadyWaiting {
        /// Invocation currently retaining the opportunity.
        invocation: ActionEvaluationInvocationId,
    },
    /// The opportunity is open rather than waiting for a deferred result.
    EvaluationNotWaiting,
    /// The caller named an invocation other than the current waiting owner.
    EvaluationInvocationMismatch {
        /// Invocation required by the caller.
        expected: ActionEvaluationInvocationId,
        /// Invocation currently retaining the opportunity.
        actual: ActionEvaluationInvocationId,
    },
    /// Ordinary consumption cannot bypass a waiting evaluation.
    EvaluationPending {
        /// Invocation that must resume or reopen the opportunity.
        invocation: ActionEvaluationInvocationId,
    },
    /// The actor-visible evaluation generation cannot advance without wrapping.
    EvaluationGenerationOverflow,
    /// The version scalar cannot advance without wrapping.
    VersionOverflow,
}

impl fmt::Display for ActionOpportunityTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleVersion { expected, actual } => write!(
                formatter,
                "action opportunity expected version {} but is at {}",
                expected.get(),
                actual.get()
            ),
            Self::AlreadyConsumed { disposition } => {
                write!(
                    formatter,
                    "action opportunity is already consumed as {disposition:?}"
                )
            }
            Self::EvaluationAlreadyWaiting { invocation } => {
                write!(
                    formatter,
                    "action opportunity already waits for evaluation {invocation}"
                )
            }
            Self::EvaluationNotWaiting => {
                formatter.write_str("action opportunity is not waiting for evaluation")
            }
            Self::EvaluationInvocationMismatch { expected, actual } => write!(
                formatter,
                "action opportunity expected evaluation {expected} but waits for {actual}"
            ),
            Self::EvaluationPending { invocation } => write!(
                formatter,
                "action opportunity cannot be consumed while evaluation {invocation} is pending"
            ),
            Self::EvaluationGenerationOverflow => {
                formatter.write_str("action evaluation generation cannot advance")
            }
            Self::VersionOverflow => {
                formatter.write_str("action opportunity version cannot advance")
            }
        }
    }
}

impl std::error::Error for ActionOpportunityTransitionError {}

/// Immutable durable value for one checked one-shot action opportunity.
///
/// Constructing or transitioning this value does not mutate accepted state,
/// append history, schedule work, or grant publication authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionOpportunity {
    id: ActionOpportunityId,
    actor: ActorId,
    sponsor: ActionSponsor,
    interaction_scope: ActionInteractionScope,
    generation: ActionOpportunityGeneration,
    evaluation_generation: ActionEvaluationGeneration,
    version: ActionOpportunityVersion,
    state: ActionOpportunityState,
}

impl ActionOpportunity {
    /// Opens a reaction-sponsored one-shot opportunity.
    pub fn open(
        actor: ActorId,
        sponsor: ActionSponsor,
        interaction_scope: ActionInteractionScope,
        generation: ActionOpportunityGeneration,
    ) -> Self {
        let id = ActionOpportunityId::derive(actor, sponsor, generation);
        Self {
            id,
            actor,
            sponsor,
            interaction_scope,
            generation,
            evaluation_generation: ActionEvaluationGeneration::INITIAL,
            version: ActionOpportunityVersion::INITIAL,
            state: ActionOpportunityState::Open,
        }
    }

    /// Returns the stable actor-safe opportunity identity.
    #[must_use]
    pub const fn id(&self) -> ActionOpportunityId {
        self.id
    }

    /// Returns the actor receiving this opportunity.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the opportunity's one explicit sponsor.
    #[must_use]
    pub const fn sponsor(&self) -> ActionSponsor {
        self.sponsor
    }

    /// Returns the opportunity's one closed interaction scope.
    #[must_use]
    pub const fn interaction_scope(&self) -> &ActionInteractionScope {
        &self.interaction_scope
    }

    /// Returns the actor-local visibility-stable generation.
    #[must_use]
    pub const fn generation(&self) -> ActionOpportunityGeneration {
        self.generation
    }

    /// Returns the current actor-visible evaluation generation.
    #[must_use]
    pub const fn evaluation_generation(&self) -> ActionEvaluationGeneration {
        self.evaluation_generation
    }

    /// Returns the current expected-version coordinate.
    #[must_use]
    pub const fn version(&self) -> ActionOpportunityVersion {
        self.version
    }

    /// Returns the current one-shot protocol state.
    #[must_use]
    pub const fn state(&self) -> ActionOpportunityState {
        self.state
    }

    /// Returns whether this is the exact open action represented by one
    /// activity's retained post-opening state.
    ///
    /// This closes the semantic pairing between accepted agency state and
    /// runtime control. It does not establish current route, process, or
    /// containment legality; those remain authoritative runtime checks.
    #[must_use]
    pub fn matches_activity_opening(&self, activity: Activity) -> bool {
        if self.actor != activity.actor()
            || self.sponsor != ActionSponsor::activity(activity.id(), activity.version())
            || self.state != ActionOpportunityState::Open
        {
            return false;
        }

        match (activity.state(), &self.interaction_scope) {
            (
                ActivityState::ContainmentTransfer(state),
                ActionInteractionScope::Containment(scope),
            ) => {
                opened_generation(state.next_opportunity_generation()) == Some(self.generation)
                    && scope.source() == state.source()
                    && scope.destinations() == [state.destination()]
                    && scope.items() == [state.item()]
                    && scope.candidate_limit() == 1
            }
            (ActivityState::Travel(state), ActionInteractionScope::Relocation(scope)) => {
                let [anchor] = scope.anchors() else {
                    return false;
                };
                let expected_step = matches!(
                    (state.step(), anchor.interaction()),
                    (TravelActivityStep::Pause, RelocationInteraction::Start(_))
                        | (TravelActivityStep::Resume, RelocationInteraction::Pause(_))
                        | (
                            TravelActivityStep::AwaitArrival,
                            RelocationInteraction::Resume(_)
                        )
                );
                opened_generation(state.next_opportunity_generation()) == Some(self.generation)
                    && expected_step
                    && anchor.source() == state.source()
                    && anchor.destination() == state.destination()
                    && scope.candidate_limit() == 1
            }
            _ => false,
        }
    }

    /// Returns the canonical identity of the complete opportunity value.
    #[must_use]
    pub fn digest(&self) -> ActionOpportunityDigest {
        ActionOpportunityDigest(ContentDigest::of_canonical(&self.canonical_preimage()))
    }

    /// Constructs the waiting successor for a matching open version.
    ///
    /// The returned invocation identity excludes private runtime provenance.
    /// Runtime must atomically retain its request and private artifacts before
    /// exposing that invocation as pending.
    pub fn begin_evaluation(
        &self,
        expected_version: ActionOpportunityVersion,
        policy_semantics: [u8; 32],
        action_input_fingerprint: [u8; 32],
    ) -> Result<(Self, ActionEvaluationInvocationId), ActionOpportunityTransitionError> {
        self.require_version(expected_version)?;
        match self.state {
            ActionOpportunityState::Open => {}
            ActionOpportunityState::WaitingForEvaluation(invocation) => {
                return Err(ActionOpportunityTransitionError::EvaluationAlreadyWaiting {
                    invocation,
                });
            }
            ActionOpportunityState::Consumed(disposition) => {
                return Err(ActionOpportunityTransitionError::AlreadyConsumed { disposition });
            }
        }
        let version = self.next_version()?;
        let invocation = ActionEvaluationInvocationId::derive(
            self.id,
            self.evaluation_generation,
            policy_semantics,
            action_input_fingerprint,
        );
        Ok((
            self.successor(
                version,
                self.evaluation_generation,
                ActionOpportunityState::WaitingForEvaluation(invocation),
            ),
            invocation,
        ))
    }

    /// Reopens a matching waiting invocation for current-result resolution.
    ///
    /// The actor-visible evaluation generation is retained because the
    /// original request remains the result being interpreted.
    pub fn resume_evaluation(
        &self,
        expected_version: ActionOpportunityVersion,
        expected_invocation: ActionEvaluationInvocationId,
    ) -> Result<Self, ActionOpportunityTransitionError> {
        self.require_version(expected_version)?;
        self.require_waiting_invocation(expected_invocation)?;
        Ok(self.successor(
            self.next_version()?,
            self.evaluation_generation,
            ActionOpportunityState::Open,
        ))
    }

    /// Reopens a matching waiting invocation for actor-visible reinvocation.
    ///
    /// A later `begin_evaluation` derives a new invocation identity from the
    /// incremented actor-visible generation.
    pub fn reopen_for_visible_reinvocation(
        &self,
        expected_version: ActionOpportunityVersion,
        expected_invocation: ActionEvaluationInvocationId,
    ) -> Result<Self, ActionOpportunityTransitionError> {
        self.require_version(expected_version)?;
        self.require_waiting_invocation(expected_invocation)?;
        let evaluation_generation = self
            .evaluation_generation
            .checked_next()
            .ok_or(ActionOpportunityTransitionError::EvaluationGenerationOverflow)?;
        Ok(self.successor(
            self.next_version()?,
            evaluation_generation,
            ActionOpportunityState::Open,
        ))
    }

    /// Constructs the terminal successor for a matching open version.
    ///
    /// Runtime must still compare-and-set the authoritative record before this
    /// immutable successor has any effect.
    pub fn consume(
        &self,
        expected_version: ActionOpportunityVersion,
        disposition: ActionOpportunityDisposition,
    ) -> Result<Self, ActionOpportunityTransitionError> {
        self.require_version(expected_version)?;
        match self.state {
            ActionOpportunityState::Open => {}
            ActionOpportunityState::WaitingForEvaluation(invocation) => {
                return Err(ActionOpportunityTransitionError::EvaluationPending { invocation });
            }
            ActionOpportunityState::Consumed(disposition) => {
                return Err(ActionOpportunityTransitionError::AlreadyConsumed { disposition });
            }
        }
        Ok(self.successor(
            self.next_version()?,
            self.evaluation_generation,
            ActionOpportunityState::Consumed(disposition),
        ))
    }

    fn require_version(
        &self,
        expected_version: ActionOpportunityVersion,
    ) -> Result<(), ActionOpportunityTransitionError> {
        if expected_version == self.version {
            Ok(())
        } else {
            Err(ActionOpportunityTransitionError::StaleVersion {
                expected: expected_version,
                actual: self.version,
            })
        }
    }

    fn require_waiting_invocation(
        &self,
        expected_invocation: ActionEvaluationInvocationId,
    ) -> Result<(), ActionOpportunityTransitionError> {
        match self.state {
            ActionOpportunityState::Open => {
                Err(ActionOpportunityTransitionError::EvaluationNotWaiting)
            }
            ActionOpportunityState::WaitingForEvaluation(actual)
                if actual != expected_invocation =>
            {
                Err(
                    ActionOpportunityTransitionError::EvaluationInvocationMismatch {
                        expected: expected_invocation,
                        actual,
                    },
                )
            }
            ActionOpportunityState::WaitingForEvaluation(_) => Ok(()),
            ActionOpportunityState::Consumed(disposition) => {
                Err(ActionOpportunityTransitionError::AlreadyConsumed { disposition })
            }
        }
    }

    fn next_version(&self) -> Result<ActionOpportunityVersion, ActionOpportunityTransitionError> {
        self.version
            .checked_next()
            .ok_or(ActionOpportunityTransitionError::VersionOverflow)
    }

    fn successor(
        &self,
        version: ActionOpportunityVersion,
        evaluation_generation: ActionEvaluationGeneration,
        state: ActionOpportunityState,
    ) -> Self {
        Self {
            id: self.id,
            actor: self.actor,
            sponsor: self.sponsor,
            interaction_scope: self.interaction_scope.clone(),
            generation: self.generation,
            evaluation_generation,
            version,
            state,
        }
    }

    fn canonical_preimage(&self) -> CanonicalBytes {
        let encoded = (|| -> Result<_, CanonicalError> {
            let mut writer = CanonicalWriter::new(ACTION_OPPORTUNITY_DOMAIN);
            writer.write_u16(ACTION_OPPORTUNITY_SCHEMA_VERSION);
            writer.write_bytes(self.id.as_bytes())?;
            writer.write_bytes(self.actor.as_bytes())?;
            self.sponsor.write_canonical(&mut writer)?;
            match &self.interaction_scope {
                ActionInteractionScope::Containment(scope) => {
                    writer.write_discriminant(0);
                    writer.write_bytes(scope.source.as_bytes())?;
                    writer.write_sequence(&scope.destinations, |writer, destination| {
                        writer.write_bytes(destination.as_bytes())
                    })?;
                    writer.write_sequence(&scope.items, |writer, item| {
                        writer.write_bytes(item.as_bytes())
                    })?;
                    writer.write_u32(scope.candidate_limit.get());
                }
                ActionInteractionScope::Relocation(scope) => {
                    writer.write_discriminant(1);
                    writer.write_sequence(&scope.anchors, |writer, anchor| {
                        match anchor.interaction {
                            RelocationInteraction::Start(route) => {
                                writer.write_discriminant(0);
                                writer.write_bytes(route.as_bytes())?;
                            }
                            RelocationInteraction::Pause(route) => {
                                writer.write_discriminant(1);
                                writer.write_bytes(route.as_bytes())?;
                            }
                            RelocationInteraction::Resume(route) => {
                                writer.write_discriminant(2);
                                writer.write_bytes(route.as_bytes())?;
                            }
                        }
                        writer.write_bytes(anchor.source.as_bytes())?;
                        writer.write_bytes(anchor.destination.as_bytes())
                    })?;
                    writer.write_u32(scope.candidate_limit.get());
                }
            }
            writer.write_u64(self.generation.get());
            writer.write_u64(self.evaluation_generation.get());
            writer.write_u64(self.version.get());
            match self.state {
                ActionOpportunityState::Open => writer.write_discriminant(0),
                ActionOpportunityState::WaitingForEvaluation(invocation) => {
                    writer.write_discriminant(1);
                    writer.write_bytes(invocation.as_bytes())?;
                }
                ActionOpportunityState::Consumed(disposition) => {
                    writer.write_discriminant(2);
                    writer.write_discriminant(action_disposition_tag(disposition));
                }
            }
            Ok(writer.finish())
        })();
        match encoded {
            Ok(bytes) => bytes,
            Err(error) => {
                unreachable!("allocated action opportunity must be canonical: {error}")
            }
        }
    }
}

const fn action_disposition_tag(disposition: ActionOpportunityDisposition) -> u32 {
    match disposition {
        ActionOpportunityDisposition::ActionSubmitted => 0,
        ActionOpportunityDisposition::NoApplicableAction => 1,
        ActionOpportunityDisposition::Failed => 2,
    }
}

fn opened_generation(next: ActionOpportunityGeneration) -> Option<ActionOpportunityGeneration> {
    let current = next.get().checked_sub(1)?;
    (current > 0).then_some(ActionOpportunityGeneration::new(current))
}

fn action_opportunity_id_preimage(
    actor: ActorId,
    sponsor: ActionSponsor,
    generation: ActionOpportunityGeneration,
) -> CanonicalBytes {
    let encoded = (|| -> Result<_, CanonicalError> {
        let mut writer = CanonicalWriter::new(ACTION_OPPORTUNITY_ID_DOMAIN);
        writer.write_u16(ACTION_OPPORTUNITY_ID_SCHEMA_VERSION);
        writer.write_bytes(actor.as_bytes())?;
        sponsor.write_canonical(&mut writer)?;
        writer.write_u64(generation.get());
        Ok(writer.finish())
    })();
    match encoded {
        Ok(bytes) => bytes,
        Err(error) => unreachable!("fixed action-opportunity identity must be canonical: {error}"),
    }
}

fn action_evaluation_invocation_id_preimage(
    opportunity: ActionOpportunityId,
    generation: ActionEvaluationGeneration,
    policy_semantics: [u8; 32],
    action_input_fingerprint: [u8; 32],
) -> CanonicalBytes {
    let encoded = (|| -> Result<_, CanonicalError> {
        let mut writer = CanonicalWriter::new(ACTION_EVALUATION_INVOCATION_ID_DOMAIN);
        writer.write_u16(ACTION_EVALUATION_INVOCATION_ID_SCHEMA_VERSION);
        writer.write_bytes(opportunity.as_bytes())?;
        writer.write_u64(generation.get());
        writer.write_bytes(&policy_semantics)?;
        writer.write_bytes(&action_input_fingerprint)?;
        Ok(writer.finish())
    })();
    match encoded {
        Ok(bytes) => bytes,
        Err(error) => {
            unreachable!("fixed action-evaluation invocation identity must be canonical: {error}")
        }
    }
}

fn write_hex(formatter: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

struct Hex<'a>(&'a [u8]);

impl fmt::Display for Hex<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn waiting_fixture() -> (ActionOpportunity, ActionEvaluationInvocationId) {
        let actor = ActorId::from_bytes([0x11; 32]);
        let source = EntityId::from_bytes([0x21; 32]);
        let destination = EntityId::from_bytes([0x22; 32]);
        let item = EntityId::from_bytes([0x23; 32]);
        let scope = ContainmentInteractionScope::new(source, vec![destination], vec![item], 1)
            .unwrap_or_else(|error| panic!("opportunity fixture must be valid: {error}"));
        let open = ActionOpportunity::open(
            actor,
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x31; 32])),
            ActionInteractionScope::containment(scope),
            ActionOpportunityGeneration::new(1),
        );
        open.begin_evaluation(open.version(), [0x41; 32], [0x42; 32])
            .unwrap_or_else(|error| panic!("opportunity fixture must begin evaluation: {error}"))
    }

    #[test]
    fn checked_transitions_report_generation_and_version_overflow() {
        let (mut waiting, invocation) = waiting_fixture();
        waiting.evaluation_generation = ActionEvaluationGeneration::new(u64::MAX)
            .unwrap_or_else(|| unreachable!("maximum u64 is nonzero"));
        assert_eq!(
            waiting.reopen_for_visible_reinvocation(waiting.version(), invocation),
            Err(ActionOpportunityTransitionError::EvaluationGenerationOverflow)
        );

        waiting.evaluation_generation = ActionEvaluationGeneration::INITIAL;
        waiting.version = ActionOpportunityVersion::new(u64::MAX)
            .unwrap_or_else(|| unreachable!("maximum u64 is nonzero"));
        assert_eq!(
            waiting.resume_evaluation(waiting.version(), invocation),
            Err(ActionOpportunityTransitionError::VersionOverflow)
        );
    }
}
