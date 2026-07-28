use core::fmt;
use std::collections::BTreeMap;

use world_core::{
    ActorId, CanonicalBytes, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest,
    EntityId,
};

use crate::process::RelocationProcessId;

use super::{ActorLocation, ActorPosition, DirectedRoute, RelocationRouteId};

/// Canonical schema version of [`DomainState`].
pub const DOMAIN_STATE_SCHEMA_VERSION: u16 = 2;

const DOMAIN_STATE_CANONICAL_DOMAIN: CanonicalDomain = match CanonicalDomain::new("domain-state-v2")
{
    Ok(domain) => domain,
    Err(_) => panic!("domain-state identity domain must be valid"),
};

/// One container and its direct item capacity.
///
/// Capacity counts direct [`ContainmentRecord`] entries only. Recursive
/// containment and aggregate resource budgets are separate model contracts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainerRecord {
    container: EntityId,
    item_capacity: u32,
}

impl ContainerRecord {
    /// Describes a container with a fixed direct-item capacity.
    #[must_use]
    pub const fn new(container: EntityId, item_capacity: u32) -> Self {
        Self {
            container,
            item_capacity,
        }
    }

    /// Returns the container entity.
    #[must_use]
    pub const fn container(self) -> EntityId {
        self.container
    }

    /// Returns the maximum number of directly contained items.
    #[must_use]
    pub const fn item_capacity(self) -> u32 {
        self.item_capacity
    }
}

/// One item's direct container relation.
///
/// Construction records input data. [`DomainState::new`] establishes
/// uniqueness, container existence, direct non-self-containment, and capacity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainmentRecord {
    item: EntityId,
    container: EntityId,
}

impl ContainmentRecord {
    /// Describes one direct containment relation.
    #[must_use]
    pub const fn new(item: EntityId, container: EntityId) -> Self {
        Self { item, container }
    }

    /// Returns the contained item.
    #[must_use]
    pub const fn item(self) -> EntityId {
        self.item
    }

    /// Returns the direct container.
    #[must_use]
    pub const fn container(self) -> EntityId {
        self.container
    }
}

/// Hard authority for one actor to transfer items out of one container.
///
/// This record is physical command authority, not social or legal ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainerAuthorityRecord {
    actor: ActorId,
    container: EntityId,
}

impl ContainerAuthorityRecord {
    /// Binds one actor to one controlled container.
    #[must_use]
    pub const fn new(actor: ActorId, container: EntityId) -> Self {
        Self { actor, container }
    }

    /// Returns the controlling actor.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the controlled container.
    #[must_use]
    pub const fn container(self) -> EntityId {
        self.container
    }
}

/// Why a domain-state value could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainStateError {
    /// More than one container record used the same entity identity.
    DuplicateContainer {
        /// Reused container identity.
        container: EntityId,
    },
    /// More than one containment record used the same item identity.
    DuplicateContainment {
        /// Multiply contained item.
        item: EntityId,
    },
    /// The same actor/container authority pair appeared more than once.
    DuplicateContainerAuthority {
        /// Reused actor identity.
        actor: ActorId,
        /// Reused container identity.
        container: EntityId,
    },
    /// A containment record referenced a container absent from the state.
    MissingContainmentContainer {
        /// Contained item.
        item: EntityId,
        /// Missing container identity.
        container: EntityId,
    },
    /// An authority record referenced a container absent from the state.
    MissingAuthorityContainer {
        /// Actor named by the authority record.
        actor: ActorId,
        /// Missing container identity.
        container: EntityId,
    },
    /// An item directly named itself as its container.
    DirectSelfContainment {
        /// Self-contained item identity.
        item: EntityId,
    },
    /// Flat containment used one entity as both an item and a container.
    ContainerUsedAsItem {
        /// Identity present in both roles.
        item: EntityId,
    },
    /// Direct membership exceeded a container's declared capacity.
    ContainerCapacityExceeded {
        /// Over-capacity container.
        container: EntityId,
        /// Declared direct-item capacity.
        capacity: u32,
        /// Actual number of direct containment records.
        actual: u64,
    },
    /// More than one route used the same complete route identity.
    DuplicateRoute {
        /// Reused route identity.
        route: RelocationRouteId,
    },
    /// More than one route connected the same directed endpoints.
    DuplicateDirectedEndpoints {
        /// Departure endpoint.
        source: EntityId,
        /// Arrival endpoint.
        destination: EntityId,
    },
    /// More than one accepted position named the same actor.
    DuplicateActorPosition {
        /// Multiply positioned actor.
        actor: ActorId,
    },
    /// An in-transit position referenced a route absent from accepted state.
    MissingTransitRoute {
        /// Positioned actor.
        actor: ActorId,
        /// Departure endpoint.
        source: EntityId,
        /// Arrival endpoint.
        destination: EntityId,
    },
}

impl fmt::Display for DomainStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateContainer { container } => {
                write!(
                    formatter,
                    "duplicate container {}",
                    Hex(container.as_bytes())
                )
            }
            Self::DuplicateContainment { item } => {
                write!(
                    formatter,
                    "duplicate containment for item {}",
                    Hex(item.as_bytes())
                )
            }
            Self::DuplicateContainerAuthority { actor, container } => write!(
                formatter,
                "duplicate authority for actor {} and container {}",
                Hex(actor.as_bytes()),
                Hex(container.as_bytes())
            ),
            Self::MissingContainmentContainer { item, container } => write!(
                formatter,
                "item {} references missing container {}",
                Hex(item.as_bytes()),
                Hex(container.as_bytes())
            ),
            Self::MissingAuthorityContainer { actor, container } => write!(
                formatter,
                "actor {} controls missing container {}",
                Hex(actor.as_bytes()),
                Hex(container.as_bytes())
            ),
            Self::DirectSelfContainment { item } => {
                write!(
                    formatter,
                    "item {} directly contains itself",
                    Hex(item.as_bytes())
                )
            }
            Self::ContainerUsedAsItem { item } => write!(
                formatter,
                "flat containment item {} is also declared as a container",
                Hex(item.as_bytes())
            ),
            Self::ContainerCapacityExceeded {
                container,
                capacity,
                actual,
            } => write!(
                formatter,
                "container {} has capacity {capacity} but {actual} direct items",
                Hex(container.as_bytes())
            ),
            Self::DuplicateRoute { route } => {
                write!(formatter, "duplicate relocation route {route}")
            }
            Self::DuplicateDirectedEndpoints {
                source,
                destination,
            } => write!(
                formatter,
                "duplicate directed relocation endpoints {} -> {}",
                Hex(source.as_bytes()),
                Hex(destination.as_bytes())
            ),
            Self::DuplicateActorPosition { actor } => {
                write!(
                    formatter,
                    "duplicate position for actor {}",
                    Hex(actor.as_bytes())
                )
            }
            Self::MissingTransitRoute {
                actor,
                source,
                destination,
            } => write!(
                formatter,
                "actor {} is in transit on missing route {} -> {}",
                Hex(actor.as_bytes()),
                Hex(source.as_bytes()),
                Hex(destination.as_bytes())
            ),
        }
    }
}

impl std::error::Error for DomainStateError {}

/// Canonical identity of one complete accepted domain state.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DomainStateDigest(ContentDigest);

impl DomainStateDigest {
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

impl fmt::Display for DomainStateDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for DomainStateDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "DomainStateDigest({self})")
    }
}

/// Immutable accepted domain state for direct containment transfer.
///
/// ```compile_fail
/// use world_model::DomainState;
///
/// let _ = DomainState {
///     containers: Vec::new(),
///     containment: Vec::new(),
///     container_authority: Vec::new(),
///     routes: Vec::new(),
///     actor_positions: Vec::new(),
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainState {
    containers: Vec<ContainerRecord>,
    containment: Vec<ContainmentRecord>,
    container_authority: Vec<ContainerAuthorityRecord>,
    routes: Vec<DirectedRoute>,
    actor_positions: Vec<ActorPosition>,
}

impl DomainState {
    /// Validates and canonicalizes a complete accepted domain-state value.
    pub fn new(
        mut containers: Vec<ContainerRecord>,
        mut containment: Vec<ContainmentRecord>,
        mut container_authority: Vec<ContainerAuthorityRecord>,
    ) -> Result<Self, DomainStateError> {
        containers.sort_by_key(|record| record.container);
        if let Some(container) = adjacent_duplicate_by(&containers, |record| record.container) {
            return Err(DomainStateError::DuplicateContainer { container });
        }

        containment.sort_by_key(|record| record.item);
        if let Some(item) = adjacent_duplicate_by(&containment, |record| record.item) {
            return Err(DomainStateError::DuplicateContainment { item });
        }

        container_authority.sort_by_key(|record| (record.actor, record.container));
        if let Some((actor, container)) = adjacent_duplicate_by(&container_authority, |record| {
            (record.actor, record.container)
        }) {
            return Err(DomainStateError::DuplicateContainerAuthority { actor, container });
        }

        for record in &containment {
            if record.item == record.container {
                return Err(DomainStateError::DirectSelfContainment { item: record.item });
            }
            if find_container(&containers, record.item).is_some() {
                return Err(DomainStateError::ContainerUsedAsItem { item: record.item });
            }
            if find_container(&containers, record.container).is_none() {
                return Err(DomainStateError::MissingContainmentContainer {
                    item: record.item,
                    container: record.container,
                });
            }
        }

        for record in &container_authority {
            if find_container(&containers, record.container).is_none() {
                return Err(DomainStateError::MissingAuthorityContainer {
                    actor: record.actor,
                    container: record.container,
                });
            }
        }

        let mut direct_counts = BTreeMap::<EntityId, u64>::new();
        for record in &containment {
            let count = direct_counts.entry(record.container).or_default();
            *count += 1;
        }
        for (container, actual) in direct_counts {
            let record = match find_container(&containers, container) {
                Some(record) => record,
                None => unreachable!("container existence was checked above"),
            };
            if actual > u64::from(record.item_capacity) {
                return Err(DomainStateError::ContainerCapacityExceeded {
                    container,
                    capacity: record.item_capacity,
                    actual,
                });
            }
        }

        Ok(Self {
            containers,
            containment,
            container_authority,
            routes: Vec::new(),
            actor_positions: Vec::new(),
        })
    }

    /// Installs and validates the accepted mobility facts for this domain.
    ///
    /// Empty mobility is a valid domain shape, so containment-only callers do
    /// not need placeholder route or position values.
    pub fn with_mobility(
        mut self,
        mut routes: Vec<DirectedRoute>,
        mut actor_positions: Vec<ActorPosition>,
    ) -> Result<Self, DomainStateError> {
        routes.sort_by_key(|route| route.id());
        if let Some(route) = adjacent_duplicate_by(&routes, |route| route.id()) {
            return Err(DomainStateError::DuplicateRoute { route });
        }

        let mut directed_endpoints = routes.iter().collect::<Vec<_>>();
        directed_endpoints.sort_by_key(|route| (route.source(), route.destination()));
        if let Some((source, destination)) = adjacent_duplicate_by(&directed_endpoints, |route| {
            (route.source(), route.destination())
        }) {
            return Err(DomainStateError::DuplicateDirectedEndpoints {
                source,
                destination,
            });
        }

        actor_positions.sort_by_key(|position| position.actor());
        if let Some(actor) = adjacent_duplicate_by(&actor_positions, |position| position.actor()) {
            return Err(DomainStateError::DuplicateActorPosition { actor });
        }
        for position in &actor_positions {
            let ActorLocation::InTransit {
                source,
                destination,
            } = position.location()
            else {
                continue;
            };
            if !routes
                .iter()
                .any(|route| route.source() == source && route.destination() == destination)
            {
                return Err(DomainStateError::MissingTransitRoute {
                    actor: position.actor(),
                    source,
                    destination,
                });
            }
        }

        self.routes = routes;
        self.actor_positions = actor_positions;
        Ok(self)
    }

    /// Returns containers in canonical container-identity order.
    #[must_use]
    pub fn containers(&self) -> &[ContainerRecord] {
        &self.containers
    }

    /// Returns containment records in canonical item-identity order.
    #[must_use]
    pub fn containment(&self) -> &[ContainmentRecord] {
        &self.containment
    }

    /// Returns authority records in canonical `(actor, container)` order.
    #[must_use]
    pub fn container_authority(&self) -> &[ContainerAuthorityRecord] {
        &self.container_authority
    }

    /// Returns directed relocation routes in canonical route-identity order.
    #[must_use]
    pub fn routes(&self) -> &[DirectedRoute] {
        &self.routes
    }

    /// Returns actor positions in canonical actor-identity order.
    #[must_use]
    pub fn actor_positions(&self) -> &[ActorPosition] {
        &self.actor_positions
    }

    /// Finds a container by exact identity.
    #[must_use]
    pub fn container(&self, container: EntityId) -> Option<&ContainerRecord> {
        find_container(&self.containers, container)
    }

    /// Finds an item's one direct containment record.
    #[must_use]
    pub fn containment_for(&self, item: EntityId) -> Option<&ContainmentRecord> {
        self.containment
            .binary_search_by_key(&item, |record| record.item)
            .ok()
            .map(|index| &self.containment[index])
    }

    /// Returns whether an exact actor/container authority record exists.
    #[must_use]
    pub fn actor_controls(&self, actor: ActorId, container: EntityId) -> bool {
        self.container_authority
            .binary_search_by_key(&(actor, container), |record| {
                (record.actor, record.container)
            })
            .is_ok()
    }

    /// Finds a route by its complete semantic identity.
    #[must_use]
    pub fn route(&self, route: RelocationRouteId) -> Option<DirectedRoute> {
        self.routes
            .binary_search_by_key(&route, |candidate| candidate.id())
            .ok()
            .map(|index| self.routes[index])
    }

    /// Finds the unique route for one ordered pair of endpoints.
    #[must_use]
    pub fn directed_route(&self, source: EntityId, destination: EntityId) -> Option<DirectedRoute> {
        self.routes
            .iter()
            .copied()
            .find(|route| route.source() == source && route.destination() == destination)
    }

    /// Returns one actor's accepted physical location.
    #[must_use]
    pub fn actor_location(&self, actor: ActorId) -> Option<ActorLocation> {
        self.actor_positions
            .binary_search_by_key(&actor, |position| position.actor())
            .ok()
            .map(|index| self.actor_positions[index].location())
    }

    /// Returns the canonical domain-state identity.
    #[must_use]
    pub fn digest(&self) -> DomainStateDigest {
        compute_domain_state_digest(
            &self.containers,
            &self.containment,
            &self.container_authority,
            &self.routes,
            &self.actor_positions,
        )
    }
}

fn find_container(containers: &[ContainerRecord], container: EntityId) -> Option<&ContainerRecord> {
    containers
        .binary_search_by_key(&container, |record| record.container)
        .ok()
        .map(|index| &containers[index])
}

fn adjacent_duplicate_by<T, K: Copy + PartialEq>(values: &[T], key: impl Fn(&T) -> K) -> Option<K> {
    values.windows(2).find_map(|pair| {
        let previous = key(&pair[0]);
        let current = key(&pair[1]);
        (previous == current).then_some(current)
    })
}

fn compute_domain_state_digest(
    containers: &[ContainerRecord],
    containment: &[ContainmentRecord],
    container_authority: &[ContainerAuthorityRecord],
    routes: &[DirectedRoute],
    actor_positions: &[ActorPosition],
) -> DomainStateDigest {
    DomainStateDigest(ContentDigest::of_canonical(&domain_state_preimage(
        containers,
        containment,
        container_authority,
        routes,
        actor_positions,
    )))
}

fn domain_state_preimage(
    containers: &[ContainerRecord],
    containment: &[ContainmentRecord],
    container_authority: &[ContainerAuthorityRecord],
    routes: &[DirectedRoute],
    actor_positions: &[ActorPosition],
) -> CanonicalBytes {
    let encoded = (|| -> Result<_, CanonicalError> {
        let mut writer = CanonicalWriter::new(DOMAIN_STATE_CANONICAL_DOMAIN);
        writer.write_u16(DOMAIN_STATE_SCHEMA_VERSION);
        writer.write_sequence(containers, |writer, record| {
            writer.write_bytes(record.container.as_bytes())?;
            writer.write_u32(record.item_capacity);
            Ok(())
        })?;
        writer.write_sequence(containment, |writer, record| {
            writer.write_bytes(record.item.as_bytes())?;
            writer.write_bytes(record.container.as_bytes())
        })?;
        writer.write_sequence(container_authority, |writer, record| {
            writer.write_bytes(record.actor.as_bytes())?;
            writer.write_bytes(record.container.as_bytes())
        })?;
        writer.write_sequence(routes, |writer, route| {
            writer.write_bytes(route.id().as_bytes())?;
            writer.write_bytes(route.source().as_bytes())?;
            writer.write_bytes(route.destination().as_bytes())?;
            writer.write_u64(route.duration().ticks());
            Ok(())
        })?;
        writer.write_sequence(actor_positions, |writer, position| {
            writer.write_bytes(position.actor().as_bytes())?;
            match position.location() {
                ActorLocation::At(location) => {
                    writer.write_discriminant(0);
                    writer.write_bytes(location.as_bytes())
                }
                ActorLocation::InTransit {
                    source,
                    destination,
                } => {
                    writer.write_discriminant(1);
                    writer.write_bytes(source.as_bytes())?;
                    writer.write_bytes(destination.as_bytes())
                }
            }
        })?;
        Ok(writer.finish())
    })();
    match encoded {
        Ok(bytes) => bytes,
        Err(error) => unreachable!(
            "allocated domain-state collections must fit the canonical protocol: {error}"
        ),
    }
}

/// A concrete proposed change to one direct containment relation.
///
/// This value is inert. Runtime remains responsible for validating it against
/// one immutable base state and for deciding whether to publish it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainmentTransferDelta {
    actor: ActorId,
    item: EntityId,
    expected_source: EntityId,
    destination: EntityId,
}

impl ContainmentTransferDelta {
    /// Constructs a structurally legal direct-containment transfer.
    pub fn new(
        actor: ActorId,
        item: EntityId,
        expected_source: EntityId,
        destination: EntityId,
    ) -> Result<Self, ContainmentTransferError> {
        if expected_source == destination {
            return Err(ContainmentTransferError::SourceEqualsDestination {
                container: expected_source,
            });
        }
        if item == expected_source {
            return Err(ContainmentTransferError::DirectSelfContainment {
                item,
                container: expected_source,
            });
        }
        if item == destination {
            return Err(ContainmentTransferError::DirectSelfContainment {
                item,
                container: destination,
            });
        }

        Ok(Self {
            actor,
            item,
            expected_source,
            destination,
        })
    }

    /// Returns the actor proposing the transfer.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the transferred item.
    #[must_use]
    pub const fn item(self) -> EntityId {
        self.item
    }

    /// Returns the source container expected in the base state.
    #[must_use]
    pub const fn expected_source(self) -> EntityId {
        self.expected_source
    }

    /// Returns the proposed destination container.
    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.destination
    }
}

/// Why a containment-transfer value was structurally invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainmentTransferError {
    /// Source and destination named the same container.
    SourceEqualsDestination {
        /// Reused container identity.
        container: EntityId,
    },
    /// The transfer would directly place an item inside itself.
    DirectSelfContainment {
        /// Transferred item.
        item: EntityId,
        /// Equal source or destination.
        container: EntityId,
    },
}

impl fmt::Display for ContainmentTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceEqualsDestination { container } => write!(
                formatter,
                "transfer source and destination are both {}",
                Hex(container.as_bytes())
            ),
            Self::DirectSelfContainment { item, container } => write!(
                formatter,
                "transfer item {} directly names itself as container {}",
                Hex(item.as_bytes()),
                Hex(container.as_bytes())
            ),
        }
    }
}

impl std::error::Error for ContainmentTransferError {}

/// One physical event emitted by the initial model slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalEvent {
    /// A direct containment transfer completed.
    ItemTransferred(ItemTransferredEvent),
    /// An actor began one authoritative relocation.
    ActorDeparted(ActorDepartedEvent),
    /// An actor completed one authoritative relocation.
    ActorArrived(ActorArrivedEvent),
}

impl PhysicalEvent {
    /// Returns the actor whose physical action or movement produced the event.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        match self {
            Self::ItemTransferred(event) => event.actor(),
            Self::ActorDeparted(event) => event.actor(),
            Self::ActorArrived(event) => event.actor(),
        }
    }

    /// Derives the physical event corresponding exactly to a transfer delta.
    #[must_use]
    pub const fn item_transferred(delta: ContainmentTransferDelta) -> Self {
        Self::ItemTransferred(ItemTransferredEvent {
            actor: delta.actor,
            item: delta.item,
            source: delta.expected_source,
            destination: delta.destination,
        })
    }

    /// Derives the physical event for one real accepted departure.
    #[must_use]
    pub const fn actor_departed(
        process: RelocationProcessId,
        actor: ActorId,
        source: EntityId,
        destination: EntityId,
    ) -> Self {
        Self::ActorDeparted(ActorDepartedEvent {
            process,
            actor,
            source,
            destination,
        })
    }

    /// Derives the physical event for one real accepted arrival.
    #[must_use]
    pub const fn actor_arrived(
        process: RelocationProcessId,
        actor: ActorId,
        source: EntityId,
        destination: EntityId,
    ) -> Self {
        Self::ActorArrived(ActorArrivedEvent {
            process,
            actor,
            source,
            destination,
        })
    }
}

/// Exact fields of an item-transferred physical event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemTransferredEvent {
    actor: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
}

impl ItemTransferredEvent {
    /// Returns the actor that held source-container authority.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the transferred item.
    #[must_use]
    pub const fn item(self) -> EntityId {
        self.item
    }

    /// Returns the preceding direct container.
    #[must_use]
    pub const fn source(self) -> EntityId {
        self.source
    }

    /// Returns the new direct container.
    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.destination
    }
}

/// Exact physical provenance of an actor entering transit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorDepartedEvent {
    process: RelocationProcessId,
    actor: ActorId,
    source: EntityId,
    destination: EntityId,
}

impl ActorDepartedEvent {
    /// Returns the relocation process that caused the departure.
    #[must_use]
    pub const fn process(self) -> RelocationProcessId {
        self.process
    }

    /// Returns the relocating actor.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the accepted location the actor left.
    #[must_use]
    pub const fn source(self) -> EntityId {
        self.source
    }

    /// Returns the accepted destination of the transit.
    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.destination
    }
}

/// Exact physical provenance of an actor leaving transit at its destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorArrivedEvent {
    process: RelocationProcessId,
    actor: ActorId,
    source: EntityId,
    destination: EntityId,
}

impl ActorArrivedEvent {
    /// Returns the relocation process that caused the arrival.
    #[must_use]
    pub const fn process(self) -> RelocationProcessId {
        self.process
    }

    /// Returns the relocating actor.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the accepted source of the completed transit.
    #[must_use]
    pub const fn source(self) -> EntityId {
        self.source
    }

    /// Returns the accepted location where the actor arrived.
    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.destination
    }
}

struct Hex<'a>(&'a [u8]);

impl fmt::Display for Hex<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;

    fn actor(byte: u8) -> ActorId {
        ActorId::from_bytes([byte; 32])
    }

    fn entity(byte: u8) -> EntityId {
        EntityId::from_bytes([byte; 32])
    }

    fn hex(bytes: &[u8]) -> String {
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            if write!(&mut encoded, "{byte:02x}").is_err() {
                unreachable!("writing to String cannot fail");
            }
        }
        encoded
    }

    #[test]
    fn domain_state_preimage_is_byte_complete() {
        let containers = [
            ContainerRecord::new(entity(0x10), 2),
            ContainerRecord::new(entity(0x20), 3),
        ];
        let containment = [
            ContainmentRecord::new(entity(0x30), entity(0x10)),
            ContainmentRecord::new(entity(0x40), entity(0x20)),
        ];
        let authority = [
            ContainerAuthorityRecord::new(actor(0x50), entity(0x10)),
            ContainerAuthorityRecord::new(actor(0x50), entity(0x20)),
            ContainerAuthorityRecord::new(actor(0x51), entity(0x20)),
        ];

        assert_eq!(
            hex(domain_state_preimage(&containers, &containment, &authority, &[], &[]).as_bytes()),
            "776f726c642d63616e6f6e6963616c2d7631000000000000000f646f6d61696e2d73746174652d76320002000000000000000200000000000000201010101010101010101010101010101010101010101010101010101010101010000000020000000000000020202020202020202020202020202020202020202020202020202020202020202000000003000000000000000200000000000000203030303030303030303030303030303030303030303030303030303030303030000000000000002010101010101010101010101010101010101010101010101010101010101010100000000000000020404040404040404040404040404040404040404040404040404040404040404000000000000000202020202020202020202020202020202020202020202020202020202020202020000000000000000300000000000000205050505050505050505050505050505050505050505050505050505050505050000000000000002010101010101010101010101010101010101010101010101010101010101010100000000000000020505050505050505050505050505050505050505050505050505050505050505000000000000000202020202020202020202020202020202020202020202020202020202020202020000000000000002051515151515151515151515151515151515151515151515151515151515151510000000000000020202020202020202020202020202020202020202020202020202020202020202000000000000000000000000000000000"
        );
    }

    #[test]
    fn mobility_is_canonical_and_transit_must_match_an_accepted_route() {
        let forward = DirectedRoute::new(
            entity(0x61),
            entity(0x62),
            world_core::SimDuration::from_ticks(4),
        )
        .unwrap_or_else(|error| panic!("route fixture must be valid: {error}"));
        let reverse = DirectedRoute::new(
            entity(0x62),
            entity(0x61),
            world_core::SimDuration::from_ticks(5),
        )
        .unwrap_or_else(|error| panic!("route fixture must be valid: {error}"));
        let actor_a = actor(0x70);
        let actor_b = actor(0x71);
        let positions = vec![
            ActorPosition::new(actor_b, ActorLocation::at(entity(0x62))),
            ActorPosition::new(actor_a, ActorLocation::in_transit(forward)),
        ];

        let state = DomainState::new(Vec::new(), Vec::new(), Vec::new())
            .and_then(|state| state.with_mobility(vec![reverse, forward], positions))
            .unwrap_or_else(|error| panic!("mobility fixture must be valid: {error}"));

        assert_eq!(
            state
                .routes()
                .iter()
                .map(|route| route.id())
                .collect::<Vec<_>>(),
            {
                let mut ids = vec![forward.id(), reverse.id()];
                ids.sort();
                ids
            }
        );
        assert_eq!(
            state.actor_location(actor_a),
            Some(ActorLocation::in_transit(forward))
        );
        assert_eq!(
            state.directed_route(entity(0x61), entity(0x62)),
            Some(forward)
        );

        let missing = DirectedRoute::new(
            entity(0x63),
            entity(0x64),
            world_core::SimDuration::from_ticks(1),
        )
        .unwrap_or_else(|error| panic!("route fixture must be valid: {error}"));
        assert_eq!(
            DomainState::new(Vec::new(), Vec::new(), Vec::new()).and_then(|state| {
                state.with_mobility(
                    vec![forward],
                    vec![ActorPosition::new(
                        actor_a,
                        ActorLocation::in_transit(missing),
                    )],
                )
            }),
            Err(DomainStateError::MissingTransitRoute {
                actor: actor_a,
                source: missing.source(),
                destination: missing.destination(),
            })
        );
    }
}
