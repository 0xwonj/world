use core::{fmt, num::NonZeroU64};

use world_core::{ActorId, CanonicalDomain, CanonicalWriter, ContentDigest, EntityId, SimDuration};

/// Canonical schema version of [`RelocationRouteId`].
pub const RELOCATION_ROUTE_ID_SCHEMA_VERSION: u16 = 1;

const RELOCATION_ROUTE_ID_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("relocation-route-id-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("relocation route identity domain must be valid"),
    };

/// Why a directed relocation route could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectedRouteError {
    /// A route must connect two distinct locations.
    SameEndpoint {
        /// Location reused as both route endpoints.
        location: EntityId,
    },
    /// Relocation must consume positive virtual time.
    ZeroDuration,
}

impl fmt::Display for DirectedRouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameEndpoint { location } => {
                write!(formatter, "relocation route repeats endpoint {location:?}")
            }
            Self::ZeroDuration => formatter.write_str("relocation route duration must be positive"),
        }
    }
}

impl std::error::Error for DirectedRouteError {}

/// Semantic identity of one directed route and its exact duration.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelocationRouteId([u8; 32]);

impl RelocationRouteId {
    /// Derives the route identity from its complete accepted semantics.
    #[must_use]
    pub fn derive(source: EntityId, destination: EntityId, duration: SimDuration) -> Self {
        let mut writer = CanonicalWriter::new(RELOCATION_ROUTE_ID_DOMAIN);
        writer.write_u16(RELOCATION_ROUTE_ID_SCHEMA_VERSION);
        if writer.write_bytes(source.as_bytes()).is_err()
            || writer.write_bytes(destination.as_bytes()).is_err()
        {
            unreachable!("fixed-width route endpoints must fit canonical encoding");
        }
        writer.write_u64(duration.ticks());
        Self(ContentDigest::of_canonical(&writer.finish()).into_bytes())
    }

    /// Constructs an identity decoded by the route-state owner.
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

impl fmt::Display for RelocationRouteId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for RelocationRouteId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RelocationRouteId({self})")
    }
}

/// One accepted directed connection with positive exact travel duration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectedRoute {
    id: RelocationRouteId,
    source: EntityId,
    destination: EntityId,
    duration_ticks: NonZeroU64,
}

impl DirectedRoute {
    /// Constructs a checked directed route.
    pub fn new(
        source: EntityId,
        destination: EntityId,
        duration: SimDuration,
    ) -> Result<Self, DirectedRouteError> {
        if source == destination {
            return Err(DirectedRouteError::SameEndpoint { location: source });
        }
        let duration_ticks =
            NonZeroU64::new(duration.ticks()).ok_or(DirectedRouteError::ZeroDuration)?;
        Ok(Self {
            id: RelocationRouteId::derive(source, destination, duration),
            source,
            destination,
            duration_ticks,
        })
    }

    /// Returns the complete route identity.
    #[must_use]
    pub const fn id(self) -> RelocationRouteId {
        self.id
    }

    /// Returns the departure location.
    #[must_use]
    pub const fn source(self) -> EntityId {
        self.source
    }

    /// Returns the arrival location.
    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.destination
    }

    /// Returns the positive travel duration.
    #[must_use]
    pub const fn duration(self) -> SimDuration {
        SimDuration::from_ticks(self.duration_ticks.get())
    }
}

/// Accepted physical position of one actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorLocation {
    /// The actor is present at one exact location.
    At(EntityId),
    /// The actor is moving along one exact directed route.
    InTransit {
        /// Departure endpoint.
        source: EntityId,
        /// Arrival endpoint.
        destination: EntityId,
    },
}

impl ActorLocation {
    /// Constructs a stationary location.
    #[must_use]
    pub const fn at(location: EntityId) -> Self {
        Self::At(location)
    }

    /// Constructs an in-transit location from one accepted route.
    #[must_use]
    pub const fn in_transit(route: DirectedRoute) -> Self {
        Self::InTransit {
            source: route.source,
            destination: route.destination,
        }
    }
}

/// One actor's unique accepted physical position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorPosition {
    actor: ActorId,
    location: ActorLocation,
}

impl ActorPosition {
    /// Associates one actor with an accepted location.
    #[must_use]
    pub const fn new(actor: ActorId, location: ActorLocation) -> Self {
        Self { actor, location }
    }

    /// Returns the positioned actor.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the actor's exact accepted location.
    #[must_use]
    pub const fn location(self) -> ActorLocation {
        self.location
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(byte: u8) -> EntityId {
        EntityId::from_bytes([byte; 32])
    }

    #[test]
    fn route_identity_covers_direction_and_duration() {
        let forward = DirectedRoute::new(entity(0x11), entity(0x12), SimDuration::from_ticks(7))
            .unwrap_or_else(|error| panic!("route fixture must be valid: {error}"));
        let reverse = DirectedRoute::new(entity(0x12), entity(0x11), SimDuration::from_ticks(7))
            .unwrap_or_else(|error| panic!("route fixture must be valid: {error}"));
        let slower = DirectedRoute::new(entity(0x11), entity(0x12), SimDuration::from_ticks(8))
            .unwrap_or_else(|error| panic!("route fixture must be valid: {error}"));

        assert_ne!(forward.id(), reverse.id());
        assert_ne!(forward.id(), slower.id());
        assert_eq!(
            RelocationRouteId::derive(forward.source(), forward.destination(), forward.duration()),
            forward.id()
        );
        assert_eq!(
            DirectedRoute::new(entity(0x11), entity(0x11), SimDuration::from_ticks(1)),
            Err(DirectedRouteError::SameEndpoint {
                location: entity(0x11)
            })
        );
        assert_eq!(
            DirectedRoute::new(entity(0x11), entity(0x12), SimDuration::ZERO),
            Err(DirectedRouteError::ZeroDuration)
        );
    }
}
