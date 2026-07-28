use std::collections::BTreeSet;

use world_core::{ActorId, EntityId};
use world_model::{
    AcceptedState, ActorLocation, ActorPosition, ContainmentRecord, ContainmentTransferDelta,
    DomainState, DomainStateError, RelocationProcess,
};

/// Why a typed containment delta set cannot produce an authoritative successor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContainmentTransitionError {
    ItemNotContained {
        item: EntityId,
    },
    SourceMismatch {
        item: EntityId,
        actual: EntityId,
        expected: EntityId,
    },
    DestinationContainerMissing {
        container: EntityId,
    },
    SourceAuthorityMissing {
        actor: world_core::ActorId,
        container: EntityId,
    },
    DuplicateItemClaim {
        item: EntityId,
    },
    InvalidSuccessor(DomainStateError),
}

/// Why one exact relocation position transition could not be applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RelocationPositionTransitionError {
    ActorPositionMissing {
        actor: ActorId,
    },
    PositionMismatch {
        actor: ActorId,
        actual: Box<ActorLocation>,
        expected: Box<ActorLocation>,
    },
    InvalidSuccessor(DomainStateError),
}

/// Applies a complete typed containment delta set from one immutable base.
///
/// The function is pure and all-or-nothing. Candidate certification and
/// authority-record application share it so their physical semantics cannot
/// drift.
pub(crate) fn apply_containment_transfers(
    base: &AcceptedState,
    deltas: &[ContainmentTransferDelta],
) -> Result<AcceptedState, ContainmentTransitionError> {
    let mut claimed_items = BTreeSet::new();
    for delta in deltas {
        if !claimed_items.insert(delta.item()) {
            return Err(ContainmentTransitionError::DuplicateItemClaim { item: delta.item() });
        }
        let current = base
            .domain()
            .containment_for(delta.item())
            .ok_or(ContainmentTransitionError::ItemNotContained { item: delta.item() })?;
        if current.container() != delta.expected_source() {
            return Err(ContainmentTransitionError::SourceMismatch {
                item: delta.item(),
                actual: current.container(),
                expected: delta.expected_source(),
            });
        }
        if base.domain().container(delta.destination()).is_none() {
            return Err(ContainmentTransitionError::DestinationContainerMissing {
                container: delta.destination(),
            });
        }
        if !base
            .domain()
            .actor_controls(delta.actor(), delta.expected_source())
        {
            return Err(ContainmentTransitionError::SourceAuthorityMissing {
                actor: delta.actor(),
                container: delta.expected_source(),
            });
        }
    }

    let mut containment = base.domain().containment().to_vec();
    for delta in deltas {
        let index = containment
            .binary_search_by_key(&delta.item(), |record| record.item())
            .map_err(|_| ContainmentTransitionError::ItemNotContained { item: delta.item() })?;
        containment[index] = ContainmentRecord::new(delta.item(), delta.destination());
    }

    let domain = DomainState::new(
        base.domain().containers().to_vec(),
        containment,
        base.domain().container_authority().to_vec(),
    )
    .and_then(|domain| {
        domain.with_mobility(
            base.domain().routes().to_vec(),
            base.domain().actor_positions().to_vec(),
        )
    })
    .map_err(ContainmentTransitionError::InvalidSuccessor)?;

    Ok(AcceptedState::new(
        domain,
        base.epistemic().clone(),
        *base.social(),
        base.agency().clone(),
    ))
}

/// Applies the accepted departure caused by one newly started process.
pub(crate) fn apply_relocation_departure(
    base: &AcceptedState,
    process: RelocationProcess,
) -> Result<AcceptedState, RelocationPositionTransitionError> {
    let route = process.route();
    replace_actor_location(
        base,
        process.actor(),
        ActorLocation::at(route.source()),
        ActorLocation::in_transit(route),
    )
}

/// Applies the accepted arrival caused by one completed process.
pub(crate) fn apply_relocation_arrival(
    base: &AcceptedState,
    process: RelocationProcess,
) -> Result<AcceptedState, RelocationPositionTransitionError> {
    let route = process.route();
    replace_actor_location(
        base,
        process.actor(),
        ActorLocation::in_transit(route),
        ActorLocation::at(route.destination()),
    )
}

fn replace_actor_location(
    base: &AcceptedState,
    actor: ActorId,
    expected: ActorLocation,
    replacement: ActorLocation,
) -> Result<AcceptedState, RelocationPositionTransitionError> {
    let actual = base
        .domain()
        .actor_location(actor)
        .ok_or(RelocationPositionTransitionError::ActorPositionMissing { actor })?;
    if actual != expected {
        return Err(RelocationPositionTransitionError::PositionMismatch {
            actor,
            actual: Box::new(actual),
            expected: Box::new(expected),
        });
    }

    let mut positions = base.domain().actor_positions().to_vec();
    let index = positions
        .binary_search_by_key(&actor, |position| position.actor())
        .map_err(|_| RelocationPositionTransitionError::ActorPositionMissing { actor })?;
    positions[index] = ActorPosition::new(actor, replacement);

    let domain = DomainState::new(
        base.domain().containers().to_vec(),
        base.domain().containment().to_vec(),
        base.domain().container_authority().to_vec(),
    )
    .and_then(|domain| domain.with_mobility(base.domain().routes().to_vec(), positions))
    .map_err(RelocationPositionTransitionError::InvalidSuccessor)?;

    Ok(AcceptedState::new(
        domain,
        base.epistemic().clone(),
        *base.social(),
        base.agency().clone(),
    ))
}

#[cfg(test)]
mod tests {
    use world_core::{ActorId, EntityId};
    use world_model::{
        AgencyState, ContainerAuthorityRecord, ContainerRecord, ContainmentRecord, DomainState,
        EpistemicState, SocialState,
    };

    use super::*;

    fn entity(byte: u8) -> EntityId {
        EntityId::from_bytes([byte; 32])
    }

    fn actor(byte: u8) -> ActorId {
        ActorId::from_bytes([byte; 32])
    }

    fn state(capacity: u32) -> AcceptedState {
        AcceptedState::new(
            DomainState::new(
                vec![
                    ContainerRecord::new(entity(0x31), 4),
                    ContainerRecord::new(entity(0x32), capacity),
                ],
                vec![
                    ContainmentRecord::new(entity(0x21), entity(0x31)),
                    ContainmentRecord::new(entity(0x22), entity(0x31)),
                ],
                vec![ContainerAuthorityRecord::new(actor(0x11), entity(0x31))],
            )
            .unwrap_or_else(|error| panic!("fixture must be valid: {error}")),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        )
    }

    fn transfer(item: u8) -> ContainmentTransferDelta {
        ContainmentTransferDelta::new(actor(0x11), entity(item), entity(0x31), entity(0x32))
            .unwrap_or_else(|error| panic!("fixture delta must be valid: {error}"))
    }

    #[test]
    fn complete_delta_set_is_applied_atomically() {
        let base = state(2);
        let successor = apply_containment_transfers(&base, &[transfer(0x21), transfer(0x22)])
            .unwrap_or_else(|error| panic!("combined transfer must be valid: {error:?}"));

        for item in [entity(0x21), entity(0x22)] {
            assert_eq!(
                successor
                    .domain()
                    .containment_for(item)
                    .map(|record| record.container()),
                Some(entity(0x32))
            );
            assert_eq!(
                base.domain()
                    .containment_for(item)
                    .map(|record| record.container()),
                Some(entity(0x31))
            );
        }
        assert_eq!(successor.epistemic(), base.epistemic());
        assert_eq!(successor.social(), base.social());
        assert_eq!(successor.agency(), base.agency());
    }

    #[test]
    fn invalid_combined_successor_leaves_the_base_unchanged() {
        let base = state(1);
        let result = apply_containment_transfers(&base, &[transfer(0x21), transfer(0x22)]);

        assert!(matches!(
            result,
            Err(ContainmentTransitionError::InvalidSuccessor(
                DomainStateError::ContainerCapacityExceeded { .. }
            ))
        ));
        assert!(
            base.domain()
                .containment()
                .iter()
                .all(|record| record.container() == entity(0x31))
        );
    }

    #[test]
    fn duplicate_item_claim_is_explicit() {
        assert_eq!(
            apply_containment_transfers(&state(2), &[transfer(0x21), transfer(0x21)]),
            Err(ContainmentTransitionError::DuplicateItemClaim { item: entity(0x21) })
        );
    }
}
