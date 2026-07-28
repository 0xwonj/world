use core::fmt;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest};

mod agency;
mod domain;
mod epistemic;
mod mobility;
mod social;

pub use agency::{
    AGENCY_STATE_SCHEMA_VERSION, Activity, ActivityControllerId, ActivityFocus, ActivityGeneration,
    ActivityId, ActivityState, ActivityStateSchemaId, ActivityStateTransitionError, ActivityStatus,
    ActivityTransition, ActivityTransitionError, ActivityTransitionKind, ActivityVersion,
    AgencyState, AgencyStateDigest, AgencyStateError, AgencyTransitionError,
    ContainmentTransferActivityState, ContainmentTransferActivityStateError, DesiredCondition,
    Intent, IntentGeneration, IntentId, IntentStatus, IntentTransition, IntentTransitionError,
    IntentVersion, TravelActivityState, TravelActivityStateError, TravelActivityStep,
};
pub use domain::{
    ActorArrivedEvent, ActorDepartedEvent, ContainerAuthorityRecord, ContainerRecord,
    ContainmentRecord, ContainmentTransferDelta, ContainmentTransferError,
    DOMAIN_STATE_SCHEMA_VERSION, DomainState, DomainStateDigest, DomainStateError,
    ItemTransferredEvent, PhysicalEvent,
};
pub use epistemic::{
    ActorEpistemicRecord, ActorRelocationObservation, ContainedInBelief, ContainedInBeliefError,
    EPISTEMIC_STATE_SCHEMA_VERSION, EpistemicState, EpistemicStateDigest, EpistemicStateError,
    EpistemicTransitionError, EpistemicVersion, EvidenceDeliveryGeneration, EvidenceDeliveryId,
    EvidenceProvenance, EvidenceRecord, ItemAbsentFromContainerObservation,
};
pub use mobility::{
    ActorLocation, ActorPosition, DirectedRoute, DirectedRouteError,
    RELOCATION_ROUTE_ID_SCHEMA_VERSION, RelocationRouteId,
};
pub use social::{SOCIAL_STATE_SCHEMA_VERSION, SocialState, SocialStateDigest};

/// Canonical schema version of [`AcceptedState`].
///
/// Version two is the first aggregate schema. Version one denoted only the
/// physical containment state that now belongs to [`DomainState`].
pub const ACCEPTED_STATE_SCHEMA_VERSION: u16 = 2;

const ACCEPTED_STATE_CANONICAL_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("accepted-state-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("accepted-state identity domain must be valid"),
    };

/// Canonical identity of one complete accepted model state.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AcceptedStateDigest(ContentDigest);

impl AcceptedStateDigest {
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

impl fmt::Display for AcceptedStateDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for AcceptedStateDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "AcceptedStateDigest({self})")
    }
}

/// Immutable aggregate of all accepted semantic state partitions.
///
/// Runtime-control protocols such as action opportunities and processes are
/// deliberately not accepted semantic truth and therefore do not appear here.
///
/// ```compile_fail
/// use world_model::{
///     AcceptedState, AgencyState, DomainState, EpistemicState, SocialState,
/// };
///
/// let domain = DomainState::new(Vec::new(), Vec::new(), Vec::new()).unwrap();
/// let _ = AcceptedState {
///     domain,
///     epistemic: EpistemicState::empty(),
///     social: SocialState::empty(),
///     agency: AgencyState::empty(),
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedState {
    domain: DomainState,
    epistemic: EpistemicState,
    social: SocialState,
    agency: AgencyState,
}

impl AcceptedState {
    /// Composes checked owner-local partitions into one accepted state.
    #[must_use]
    pub const fn new(
        domain: DomainState,
        epistemic: EpistemicState,
        social: SocialState,
        agency: AgencyState,
    ) -> Self {
        Self {
            domain,
            epistemic,
            social,
            agency,
        }
    }

    /// Returns accepted physical and other world-domain facts.
    #[must_use]
    pub const fn domain(&self) -> &DomainState {
        &self.domain
    }

    /// Returns accepted actor-relative evidence and belief state.
    #[must_use]
    pub const fn epistemic(&self) -> &EpistemicState {
        &self.epistemic
    }

    /// Returns accepted social state.
    #[must_use]
    pub const fn social(&self) -> &SocialState {
        &self.social
    }

    /// Returns accepted intent and activity state.
    #[must_use]
    pub const fn agency(&self) -> &AgencyState {
        &self.agency
    }

    /// Returns the canonical identity of all four accepted partitions.
    #[must_use]
    pub fn digest(&self) -> AcceptedStateDigest {
        AcceptedStateDigest(ContentDigest::of_canonical(&accepted_state_preimage(self)))
    }
}

fn accepted_state_preimage(state: &AcceptedState) -> CanonicalBytes {
    let encoded = (|| -> Result<_, CanonicalError> {
        let mut writer = CanonicalWriter::new(ACCEPTED_STATE_CANONICAL_DOMAIN);
        writer.write_u16(ACCEPTED_STATE_SCHEMA_VERSION);
        writer.write_bytes(state.domain.digest().as_bytes())?;
        writer.write_bytes(state.epistemic.digest().as_bytes())?;
        writer.write_bytes(state.social.digest().as_bytes())?;
        writer.write_bytes(state.agency.digest().as_bytes())?;
        Ok(writer.finish())
    })();
    match encoded {
        Ok(bytes) => bytes,
        Err(error) => unreachable!("fixed accepted-state identity must be canonical: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;

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
    fn accepted_state_preimage_is_byte_complete() {
        let state = AcceptedState::new(
            DomainState::new(Vec::new(), Vec::new(), Vec::new())
                .unwrap_or_else(|error| panic!("empty domain state must be valid: {error}")),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );

        assert_eq!(
            hex(accepted_state_preimage(&state).as_bytes()),
            "776f726c642d63616e6f6e6963616c2d7631000000000000001161636365707465642d73746174652d763200020000000000000020e70e495b6799c899670d8bb645b2868665c80a55c8cfd13526d994815db7d3a50000000000000020bf0115831a7b526be11c7e9b8b2cae9a74e076c3c799ddfce810c571ef9637200000000000000020c400400c7239599ffa2adccfcb723c3418abce261de44c8238c393a5e42a63240000000000000020795cbbd500486dd226807b0b2fc9b171eb52ae1f4cb7b06f0e9f25d84b88d267"
        );
    }
}
