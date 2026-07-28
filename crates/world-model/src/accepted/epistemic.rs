use core::fmt;
use core::num::NonZeroU64;
use std::collections::{BTreeMap, BTreeSet};

use world_core::{
    ActorId, CanonicalBytes, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest,
    EntityId,
};

use super::{ActorArrivedEvent, ActorDepartedEvent, ItemTransferredEvent, PhysicalEvent};

/// Canonical schema version of [`EpistemicState`].
pub const EPISTEMIC_STATE_SCHEMA_VERSION: u16 = 4;

const EPISTEMIC_STATE_CANONICAL_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("epistemic-state-v4") {
        Ok(domain) => domain,
        Err(_) => panic!("epistemic-state identity domain must be valid"),
    };
const EVIDENCE_DELIVERY_ID_CANONICAL_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("evidence-delivery-id-v3") {
        Ok(domain) => domain,
        Err(_) => panic!("evidence-delivery identity domain must be valid"),
    };

/// Actor-local sequence coordinate of one visible evidence delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceDeliveryGeneration(NonZeroU64);

impl EvidenceDeliveryGeneration {
    /// Constructs a nonzero actor-local delivery generation.
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the exact generation scalar.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Returns the following generation, or `None` on overflow.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.get().checked_add(1) {
            Some(value) => Self::new(value),
            None => None,
        }
    }
}

/// Actor-local version of accepted epistemic state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpistemicVersion(u64);

impl EpistemicVersion {
    /// Version observed before an actor has accepted evidence.
    pub const EMPTY: Self = Self(0);

    /// Constructs an exact actor-local version.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the exact version scalar.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the following version, or `None` on overflow.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// Actor-safe semantic source of accepted evidence.
///
/// This provenance contains only modeled observation meaning. Authority
/// records, revisions, scheduler coordinates, and host identifiers cannot be
/// represented here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceProvenance {
    /// The observing actor directly perceived a completed item transfer.
    DirectItemTransfer(ItemTransferredEvent),
    /// The observing actor established only that an item was absent from the
    /// exact container in which they expected to interact with it.
    DirectItemAbsent(ItemAbsentFromContainerObservation),
    /// The observing actor perceived an actor enter transit.
    DirectActorDeparture(ActorRelocationObservation),
    /// The observing actor perceived an actor arrive at a destination.
    DirectActorArrival(ActorRelocationObservation),
}

impl EvidenceProvenance {
    fn write_canonical(self, writer: &mut CanonicalWriter) -> Result<(), CanonicalError> {
        match self {
            Self::DirectItemTransfer(event) => {
                writer.write_discriminant(0);
                writer.write_bytes(event.actor().as_bytes())?;
                writer.write_bytes(event.item().as_bytes())?;
                writer.write_bytes(event.source().as_bytes())?;
                writer.write_bytes(event.destination().as_bytes())
            }
            Self::DirectItemAbsent(observation) => {
                writer.write_discriminant(3);
                writer.write_bytes(observation.item.as_bytes())?;
                writer.write_bytes(observation.expected_container.as_bytes())
            }
            Self::DirectActorDeparture(observation) => {
                writer.write_discriminant(1);
                observation.write_canonical(writer)
            }
            Self::DirectActorArrival(observation) => {
                writer.write_discriminant(2);
                observation.write_canonical(writer)
            }
        }
    }

    const fn contained_in_claim(self) -> Option<(EntityId, EntityId)> {
        match self {
            Self::DirectItemTransfer(event) => Some((event.item(), event.destination())),
            Self::DirectItemAbsent(_)
            | Self::DirectActorDeparture(_)
            | Self::DirectActorArrival(_) => None,
        }
    }

    /// Returns the containment item directly addressed by this evidence.
    #[must_use]
    pub const fn containment_item(self) -> Option<EntityId> {
        match self {
            Self::DirectItemTransfer(event) => Some(event.item()),
            Self::DirectItemAbsent(observation) => Some(observation.item),
            Self::DirectActorDeparture(_) | Self::DirectActorArrival(_) => None,
        }
    }

    const fn containment_effect(self) -> Option<ContainmentEvidenceEffect> {
        match self {
            Self::DirectItemTransfer(event) => Some(ContainmentEvidenceEffect::Present {
                item: event.item(),
                container: event.destination(),
            }),
            Self::DirectItemAbsent(observation) => Some(ContainmentEvidenceEffect::Absent {
                item: observation.item,
                expected_container: observation.expected_container,
            }),
            Self::DirectActorDeparture(_) | Self::DirectActorArrival(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContainmentEvidenceEffect {
    Present {
        item: EntityId,
        container: EntityId,
    },
    Absent {
        item: EntityId,
        expected_container: EntityId,
    },
}

/// Actor-safe meaning of a failed interaction at one expected container.
///
/// This observation intentionally contains no actual item location, runtime
/// rejection reason, command identity, or authority coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemAbsentFromContainerObservation {
    item: EntityId,
    expected_container: EntityId,
}

impl ItemAbsentFromContainerObservation {
    /// Returns the item that was not available for the attempted interaction.
    #[must_use]
    pub const fn item(self) -> EntityId {
        self.item
    }

    /// Returns the exact container contradicted by the observation.
    #[must_use]
    pub const fn expected_container(self) -> EntityId {
        self.expected_container
    }
}

/// Actor-safe physical meaning of a relocation observation.
///
/// Runtime process identity and wake generation are deliberately absent: they
/// are causal authority provenance, not something a modeled observer learns
/// merely by perceiving departure or arrival.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorRelocationObservation {
    actor: ActorId,
    source: EntityId,
    destination: EntityId,
}

impl ActorRelocationObservation {
    /// Returns the actor whose movement was observed.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the observed departure endpoint.
    #[must_use]
    pub const fn source(self) -> EntityId {
        self.source
    }

    /// Returns the observed arrival endpoint.
    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.destination
    }

    fn write_canonical(self, writer: &mut CanonicalWriter) -> Result<(), CanonicalError> {
        writer.write_bytes(self.actor.as_bytes())?;
        writer.write_bytes(self.source.as_bytes())?;
        writer.write_bytes(self.destination.as_bytes())
    }
}

/// Stable semantic identity of one actor-addressed evidence delivery.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceDeliveryId([u8; 32]);

impl EvidenceDeliveryId {
    /// Constructs an identity decoded from durable model data.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Derives an identity from the observer, actor-local generation, and the
    /// complete actor-safe observation body.
    #[must_use]
    pub fn derive(
        observer: ActorId,
        generation: EvidenceDeliveryGeneration,
        provenance: EvidenceProvenance,
    ) -> Self {
        Self(
            ContentDigest::of_canonical(&evidence_delivery_id_preimage(
                observer, generation, provenance,
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

impl fmt::Display for EvidenceDeliveryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for EvidenceDeliveryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "EvidenceDeliveryId({self})")
    }
}

/// Immutable accepted provenance for one actor-relative observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceRecord {
    id: EvidenceDeliveryId,
    observer: ActorId,
    generation: EvidenceDeliveryGeneration,
    provenance: EvidenceProvenance,
}

impl EvidenceRecord {
    /// Creates actor-safe direct evidence from one real physical event.
    ///
    /// Relocation process identity is removed by the concrete relocation
    /// constructors before the evidence crosses the actor-facing boundary.
    #[must_use]
    pub fn direct_physical_event(
        observer: ActorId,
        generation: EvidenceDeliveryGeneration,
        event: PhysicalEvent,
    ) -> Self {
        match event {
            PhysicalEvent::ItemTransferred(event) => {
                Self::direct_item_transfer(observer, generation, event)
            }
            PhysicalEvent::ActorDeparted(event) => {
                Self::direct_actor_departure(observer, generation, event)
            }
            PhysicalEvent::ActorArrived(event) => {
                Self::direct_actor_arrival(observer, generation, event)
            }
        }
    }

    /// Creates direct-observation evidence with its derived semantic identity.
    #[must_use]
    pub fn direct_item_transfer(
        observer: ActorId,
        generation: EvidenceDeliveryGeneration,
        event: ItemTransferredEvent,
    ) -> Self {
        let provenance = EvidenceProvenance::DirectItemTransfer(event);
        Self {
            id: EvidenceDeliveryId::derive(observer, generation, provenance),
            observer,
            generation,
            provenance,
        }
    }

    /// Creates a non-locating observation that contradicts one exact
    /// contained-in belief.
    #[must_use]
    pub fn direct_item_absent(
        observer: ActorId,
        generation: EvidenceDeliveryGeneration,
        item: EntityId,
        expected_container: EntityId,
    ) -> Self {
        let provenance = EvidenceProvenance::DirectItemAbsent(ItemAbsentFromContainerObservation {
            item,
            expected_container,
        });
        Self {
            id: EvidenceDeliveryId::derive(observer, generation, provenance),
            observer,
            generation,
            provenance,
        }
    }

    /// Creates actor-safe departure evidence from a real physical event.
    #[must_use]
    pub fn direct_actor_departure(
        observer: ActorId,
        generation: EvidenceDeliveryGeneration,
        event: ActorDepartedEvent,
    ) -> Self {
        let provenance = EvidenceProvenance::DirectActorDeparture(ActorRelocationObservation {
            actor: event.actor(),
            source: event.source(),
            destination: event.destination(),
        });
        Self {
            id: EvidenceDeliveryId::derive(observer, generation, provenance),
            observer,
            generation,
            provenance,
        }
    }

    /// Creates actor-safe arrival evidence from a real physical event.
    #[must_use]
    pub fn direct_actor_arrival(
        observer: ActorId,
        generation: EvidenceDeliveryGeneration,
        event: ActorArrivedEvent,
    ) -> Self {
        let provenance = EvidenceProvenance::DirectActorArrival(ActorRelocationObservation {
            actor: event.actor(),
            source: event.source(),
            destination: event.destination(),
        });
        Self {
            id: EvidenceDeliveryId::derive(observer, generation, provenance),
            observer,
            generation,
            provenance,
        }
    }

    /// Returns the semantic delivery identity.
    #[must_use]
    pub const fn id(self) -> EvidenceDeliveryId {
        self.id
    }

    /// Returns the actor whose epistemic state owns this evidence.
    #[must_use]
    pub const fn observer(self) -> ActorId {
        self.observer
    }

    /// Returns the actor-local delivery generation.
    #[must_use]
    pub const fn generation(self) -> EvidenceDeliveryGeneration {
        self.generation
    }

    /// Returns actor-safe semantic provenance.
    #[must_use]
    pub const fn provenance(self) -> EvidenceProvenance {
        self.provenance
    }

    fn has_valid_id(self) -> bool {
        self.id == EvidenceDeliveryId::derive(self.observer, self.generation, self.provenance)
    }
}

/// Why a contained-in belief could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainedInBeliefError {
    /// A belief was supplied without accepted evidence.
    EmptySupport,
    /// The same evidence delivery was supplied more than once.
    DuplicateSupport {
        /// Repeated evidence identity.
        evidence: EvidenceDeliveryId,
    },
}

impl fmt::Display for ContainedInBeliefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySupport => {
                formatter.write_str("contained-in belief has no evidence support")
            }
            Self::DuplicateSupport { evidence } => {
                write!(formatter, "contained-in belief repeats evidence {evidence}")
            }
        }
    }
}

impl std::error::Error for ContainedInBeliefError {}

/// One actor's current belief about an item's direct container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainedInBelief {
    actor: ActorId,
    item: EntityId,
    container: EntityId,
    support: Vec<EvidenceDeliveryId>,
}

impl ContainedInBelief {
    /// Validates and canonicalizes nonempty evidence support.
    pub fn new(
        actor: ActorId,
        item: EntityId,
        container: EntityId,
        mut support: Vec<EvidenceDeliveryId>,
    ) -> Result<Self, ContainedInBeliefError> {
        if support.is_empty() {
            return Err(ContainedInBeliefError::EmptySupport);
        }
        support.sort();
        if let Some(evidence) = adjacent_duplicate(&support) {
            return Err(ContainedInBeliefError::DuplicateSupport { evidence });
        }
        Ok(Self {
            actor,
            item,
            container,
            support,
        })
    }

    /// Returns the actor holding this belief.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the item named by the belief.
    #[must_use]
    pub const fn item(&self) -> EntityId {
        self.item
    }

    /// Returns the believed direct container.
    #[must_use]
    pub const fn container(&self) -> EntityId {
        self.container
    }

    /// Returns canonical nonempty supporting evidence identities.
    #[must_use]
    pub fn support(&self) -> &[EvidenceDeliveryId] {
        &self.support
    }
}

/// Version coordinates for one actor's accepted epistemic history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorEpistemicRecord {
    actor: ActorId,
    version: EpistemicVersion,
    last_delivery_generation: EvidenceDeliveryGeneration,
}

impl ActorEpistemicRecord {
    /// Records a nonempty actor-local epistemic history.
    #[must_use]
    pub const fn new(
        actor: ActorId,
        version: EpistemicVersion,
        last_delivery_generation: EvidenceDeliveryGeneration,
    ) -> Self {
        Self {
            actor,
            version,
            last_delivery_generation,
        }
    }

    /// Returns the actor owning the history.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns its current actor-local version.
    #[must_use]
    pub const fn version(self) -> EpistemicVersion {
        self.version
    }

    /// Returns the latest accepted delivery generation.
    #[must_use]
    pub const fn last_delivery_generation(self) -> EvidenceDeliveryGeneration {
        self.last_delivery_generation
    }
}

/// Why an epistemic state failed structural or provenance validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EpistemicStateError {
    DuplicateActor {
        actor: ActorId,
    },
    EmptyActorVersion {
        actor: ActorId,
    },
    VersionExceedsEvidence {
        actor: ActorId,
    },
    DuplicateEvidenceGeneration {
        actor: ActorId,
        generation: EvidenceDeliveryGeneration,
    },
    InvalidEvidenceId {
        evidence: EvidenceDeliveryId,
    },
    MissingActorRecord {
        actor: ActorId,
    },
    NoncontiguousEvidence {
        actor: ActorId,
        expected: u64,
        actual: u64,
    },
    ActorGenerationMismatch {
        actor: ActorId,
        declared: u64,
        actual: u64,
    },
    DuplicateBelief {
        actor: ActorId,
        item: EntityId,
    },
    MissingBeliefSupport {
        evidence: EvidenceDeliveryId,
    },
    MismatchedBeliefSupport {
        evidence: EvidenceDeliveryId,
    },
    StaleBelief {
        actor: ActorId,
        item: EntityId,
    },
    MissingCurrentBelief {
        actor: ActorId,
        item: EntityId,
    },
}

impl fmt::Display for EpistemicStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EpistemicStateError {}

/// Why an evidence assimilation successor could not be constructed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EpistemicTransitionError {
    EmptyAssimilation,
    StaleVersion {
        expected: EpistemicVersion,
        actual: EpistemicVersion,
    },
    WrongObserver {
        evidence: EvidenceDeliveryId,
    },
    UnexpectedGeneration {
        expected: u64,
        actual: u64,
    },
    VersionOverflow,
    InvalidSuccessor(EpistemicStateError),
}

impl fmt::Display for EpistemicTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EpistemicTransitionError {}

/// Canonical identity of accepted epistemic state.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpistemicStateDigest(ContentDigest);

impl EpistemicStateDigest {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0.into_bytes()
    }
}

impl fmt::Display for EpistemicStateDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for EpistemicStateDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "EpistemicStateDigest({self})")
    }
}

/// Immutable accepted actor-relative evidence and contained-in beliefs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EpistemicState {
    actors: Vec<ActorEpistemicRecord>,
    evidence: Vec<EvidenceRecord>,
    contained_in: Vec<ContainedInBelief>,
}

impl EpistemicState {
    /// Constructs the canonical empty epistemic state.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            actors: Vec::new(),
            evidence: Vec::new(),
            contained_in: Vec::new(),
        }
    }

    /// Validates and canonicalizes a complete epistemic state.
    pub fn new(
        mut actors: Vec<ActorEpistemicRecord>,
        mut evidence: Vec<EvidenceRecord>,
        mut contained_in: Vec<ContainedInBelief>,
    ) -> Result<Self, EpistemicStateError> {
        actors.sort_by_key(|record| record.actor);
        if let Some(actor) = adjacent_duplicate_by(&actors, |record| record.actor) {
            return Err(EpistemicStateError::DuplicateActor { actor });
        }
        for record in &actors {
            if record.version == EpistemicVersion::EMPTY {
                return Err(EpistemicStateError::EmptyActorVersion {
                    actor: record.actor,
                });
            }
            if record.version.get() > record.last_delivery_generation.get() {
                return Err(EpistemicStateError::VersionExceedsEvidence {
                    actor: record.actor,
                });
            }
        }

        evidence.sort_by_key(|record| (record.observer, record.generation));
        if let Some((actor, generation)) =
            adjacent_duplicate_by(&evidence, |record| (record.observer, record.generation))
        {
            return Err(EpistemicStateError::DuplicateEvidenceGeneration { actor, generation });
        }

        let mut next_generation = BTreeMap::<ActorId, u64>::new();
        let mut evidence_by_id = BTreeMap::<EvidenceDeliveryId, EvidenceRecord>::new();
        let mut current_claim = BTreeMap::<(ActorId, EntityId), EntityId>::new();
        for record in &evidence {
            if !record.has_valid_id() {
                return Err(EpistemicStateError::InvalidEvidenceId {
                    evidence: record.id,
                });
            }
            if find_actor(&actors, record.observer).is_none() {
                return Err(EpistemicStateError::MissingActorRecord {
                    actor: record.observer,
                });
            }
            let expected = next_generation.entry(record.observer).or_insert(1);
            if record.generation.get() != *expected {
                return Err(EpistemicStateError::NoncontiguousEvidence {
                    actor: record.observer,
                    expected: *expected,
                    actual: record.generation.get(),
                });
            }
            *expected = expected
                .checked_add(1)
                .unwrap_or_else(|| record.generation.get());
            evidence_by_id.insert(record.id, *record);
            match record.provenance.containment_effect() {
                Some(ContainmentEvidenceEffect::Present { item, container }) => {
                    current_claim.insert((record.observer, item), container);
                }
                Some(ContainmentEvidenceEffect::Absent {
                    item,
                    expected_container,
                }) => {
                    let key = (record.observer, item);
                    if current_claim.get(&key) == Some(&expected_container) {
                        current_claim.remove(&key);
                    }
                }
                None => {}
            }
        }

        for actor in &actors {
            let actual = next_generation
                .get(&actor.actor)
                .copied()
                .unwrap_or(1)
                .saturating_sub(1);
            if actual != actor.last_delivery_generation.get() {
                return Err(EpistemicStateError::ActorGenerationMismatch {
                    actor: actor.actor,
                    declared: actor.last_delivery_generation.get(),
                    actual,
                });
            }
        }

        contained_in.sort_by_key(|belief| (belief.actor, belief.item));
        if let Some((actor, item)) =
            adjacent_duplicate_by(&contained_in, |belief| (belief.actor, belief.item))
        {
            return Err(EpistemicStateError::DuplicateBelief { actor, item });
        }

        let mut represented = BTreeSet::new();
        for belief in &contained_in {
            for support in &belief.support {
                let Some(record) = evidence_by_id.get(support) else {
                    return Err(EpistemicStateError::MissingBeliefSupport { evidence: *support });
                };
                let claim = record.provenance.contained_in_claim();
                if record.observer != belief.actor || claim != Some((belief.item, belief.container))
                {
                    return Err(EpistemicStateError::MismatchedBeliefSupport {
                        evidence: *support,
                    });
                }
            }
            if current_claim.get(&(belief.actor, belief.item)) != Some(&belief.container) {
                return Err(EpistemicStateError::StaleBelief {
                    actor: belief.actor,
                    item: belief.item,
                });
            }
            represented.insert((belief.actor, belief.item));
        }
        if let Some(((actor, item), _)) = current_claim
            .iter()
            .find(|(key, _)| !represented.contains(key))
        {
            return Err(EpistemicStateError::MissingCurrentBelief {
                actor: *actor,
                item: *item,
            });
        }

        Ok(Self {
            actors,
            evidence,
            contained_in,
        })
    }

    #[must_use]
    pub fn actors(&self) -> &[ActorEpistemicRecord] {
        &self.actors
    }

    #[must_use]
    pub fn evidence(&self) -> &[EvidenceRecord] {
        &self.evidence
    }

    /// Finds accepted evidence by exact semantic delivery identity.
    #[must_use]
    pub fn evidence_record(&self, id: EvidenceDeliveryId) -> Option<&EvidenceRecord> {
        self.evidence.iter().find(|record| record.id == id)
    }

    #[must_use]
    pub fn contained_in_beliefs(&self) -> &[ContainedInBelief] {
        &self.contained_in
    }

    #[must_use]
    pub fn actor_record(&self, actor: ActorId) -> Option<&ActorEpistemicRecord> {
        find_actor(&self.actors, actor)
    }

    #[must_use]
    pub fn actor_version(&self, actor: ActorId) -> EpistemicVersion {
        self.actor_record(actor)
            .map_or(EpistemicVersion::EMPTY, |record| record.version)
    }

    #[must_use]
    pub fn next_delivery_generation(&self, actor: ActorId) -> Option<EvidenceDeliveryGeneration> {
        match self.actor_record(actor) {
            Some(record) => record.last_delivery_generation.checked_next(),
            None => EvidenceDeliveryGeneration::new(1),
        }
    }

    #[must_use]
    pub fn contained_in(&self, actor: ActorId, item: EntityId) -> Option<&ContainedInBelief> {
        self.contained_in
            .binary_search_by_key(&(actor, item), |belief| (belief.actor, belief.item))
            .ok()
            .map(|index| &self.contained_in[index])
    }

    /// Constructs the checked successor after one nonempty actor-local
    /// assimilation batch.
    pub fn assimilate(
        &self,
        actor: ActorId,
        expected_version: EpistemicVersion,
        mut records: Vec<EvidenceRecord>,
    ) -> Result<Self, EpistemicTransitionError> {
        if records.is_empty() {
            return Err(EpistemicTransitionError::EmptyAssimilation);
        }
        let actual = self.actor_version(actor);
        if expected_version != actual {
            return Err(EpistemicTransitionError::StaleVersion {
                expected: expected_version,
                actual,
            });
        }
        records.sort_by_key(|record| record.generation);
        let mut expected_generation = self
            .next_delivery_generation(actor)
            .ok_or(EpistemicTransitionError::UnexpectedGeneration {
                expected: u64::MAX,
                actual: u64::MAX,
            })?
            .get();
        for record in &records {
            if record.observer != actor {
                return Err(EpistemicTransitionError::WrongObserver {
                    evidence: record.id,
                });
            }
            if record.generation.get() != expected_generation {
                return Err(EpistemicTransitionError::UnexpectedGeneration {
                    expected: expected_generation,
                    actual: record.generation.get(),
                });
            }
            expected_generation = expected_generation.checked_add(1).ok_or(
                EpistemicTransitionError::UnexpectedGeneration {
                    expected: u64::MAX,
                    actual: record.generation.get(),
                },
            )?;
        }
        let next_version = actual
            .checked_next()
            .ok_or(EpistemicTransitionError::VersionOverflow)?;

        let mut evidence = self.evidence.clone();
        evidence.extend(records.iter().copied());
        let mut contained_in = self.contained_in.clone();
        for record in &records {
            match record.provenance.containment_effect() {
                Some(ContainmentEvidenceEffect::Present { item, container }) => {
                    match contained_in
                        .binary_search_by_key(&(actor, item), |belief| (belief.actor, belief.item))
                    {
                        Ok(index) if contained_in[index].container == container => {
                            contained_in[index].support.push(record.id);
                            contained_in[index].support.sort();
                            contained_in[index].support.dedup();
                        }
                        Ok(index) => {
                            contained_in[index] =
                                ContainedInBelief::new(actor, item, container, vec![record.id])
                                    .unwrap_or_else(|_| unreachable!("single support is nonempty"));
                        }
                        Err(index) => contained_in.insert(
                            index,
                            ContainedInBelief::new(actor, item, container, vec![record.id])
                                .unwrap_or_else(|_| unreachable!("single support is nonempty")),
                        ),
                    }
                }
                Some(ContainmentEvidenceEffect::Absent {
                    item,
                    expected_container,
                }) => {
                    if let Ok(index) = contained_in
                        .binary_search_by_key(&(actor, item), |belief| (belief.actor, belief.item))
                        && contained_in[index].container == expected_container
                    {
                        contained_in.remove(index);
                    }
                }
                None => {}
            }
        }

        let mut actors = self.actors.clone();
        let last_delivery_generation = records
            .last()
            .map(|record| record.generation)
            .unwrap_or_else(|| unreachable!("empty assimilation was rejected"));
        match actors.binary_search_by_key(&actor, |record| record.actor) {
            Ok(index) => {
                actors[index] =
                    ActorEpistemicRecord::new(actor, next_version, last_delivery_generation);
            }
            Err(index) => actors.insert(
                index,
                ActorEpistemicRecord::new(actor, next_version, last_delivery_generation),
            ),
        }

        Self::new(actors, evidence, contained_in)
            .map_err(EpistemicTransitionError::InvalidSuccessor)
    }

    #[must_use]
    pub fn digest(&self) -> EpistemicStateDigest {
        EpistemicStateDigest(ContentDigest::of_canonical(&self.canonical_preimage()))
    }

    fn canonical_preimage(&self) -> CanonicalBytes {
        let encoded = (|| -> Result<_, CanonicalError> {
            let mut writer = CanonicalWriter::new(EPISTEMIC_STATE_CANONICAL_DOMAIN);
            writer.write_u16(EPISTEMIC_STATE_SCHEMA_VERSION);
            writer.write_sequence(&self.actors, |writer, record| {
                writer.write_bytes(record.actor.as_bytes())?;
                writer.write_u64(record.version.get());
                writer.write_u64(record.last_delivery_generation.get());
                Ok(())
            })?;
            writer.write_sequence(&self.evidence, |writer, record| {
                writer.write_bytes(record.id.as_bytes())?;
                writer.write_bytes(record.observer.as_bytes())?;
                writer.write_u64(record.generation.get());
                record.provenance.write_canonical(writer)
            })?;
            writer.write_sequence(&self.contained_in, |writer, belief| {
                writer.write_bytes(belief.actor.as_bytes())?;
                writer.write_bytes(belief.item.as_bytes())?;
                writer.write_bytes(belief.container.as_bytes())?;
                writer.write_sequence(&belief.support, |writer, evidence| {
                    writer.write_bytes(evidence.as_bytes())
                })
            })?;
            Ok(writer.finish())
        })();
        match encoded {
            Ok(bytes) => bytes,
            Err(error) => {
                unreachable!("allocated epistemic state must be canonical: {error}")
            }
        }
    }
}

fn evidence_delivery_id_preimage(
    observer: ActorId,
    generation: EvidenceDeliveryGeneration,
    provenance: EvidenceProvenance,
) -> CanonicalBytes {
    let encoded = (|| -> Result<_, CanonicalError> {
        let mut writer = CanonicalWriter::new(EVIDENCE_DELIVERY_ID_CANONICAL_DOMAIN);
        writer.write_u16(3);
        writer.write_bytes(observer.as_bytes())?;
        writer.write_u64(generation.get());
        provenance.write_canonical(&mut writer)?;
        Ok(writer.finish())
    })();
    match encoded {
        Ok(bytes) => bytes,
        Err(error) => unreachable!("fixed evidence identity must be canonical: {error}"),
    }
}

fn find_actor(actors: &[ActorEpistemicRecord], actor: ActorId) -> Option<&ActorEpistemicRecord> {
    actors
        .binary_search_by_key(&actor, |record| record.actor)
        .ok()
        .map(|index| &actors[index])
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
