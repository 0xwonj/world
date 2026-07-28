use world_core::{ActorId, EntityId, WorldRevision};
use world_model::{
    AcceptedState, AgencyState, ContainerAuthorityRecord, ContainerRecord, ContainmentRecord,
    ContainmentTransferDelta, ContainmentTransferError, DomainState, DomainStateError,
    EpistemicState, PhysicalEvent, SocialState, WorldSnapshot,
};

fn actor(byte: u8) -> ActorId {
    ActorId::from_bytes([byte; 32])
}

fn entity(byte: u8) -> EntityId {
    EntityId::from_bytes([byte; 32])
}

fn domain_in_input_order(reversed: bool) -> DomainState {
    let mut containers = vec![
        ContainerRecord::new(entity(0x10), 2),
        ContainerRecord::new(entity(0x20), 3),
    ];
    let mut containment = vec![
        ContainmentRecord::new(entity(0x30), entity(0x10)),
        ContainmentRecord::new(entity(0x40), entity(0x20)),
    ];
    let mut authority = vec![
        ContainerAuthorityRecord::new(actor(0x51), entity(0x20)),
        ContainerAuthorityRecord::new(actor(0x50), entity(0x20)),
        ContainerAuthorityRecord::new(actor(0x50), entity(0x10)),
    ];
    if reversed {
        containers.reverse();
        containment.reverse();
        authority.reverse();
    }

    DomainState::new(containers, containment, authority)
        .unwrap_or_else(|error| panic!("domain-state fixture must be valid: {error}"))
}

fn accepted_in_input_order(reversed: bool) -> AcceptedState {
    AcceptedState::new(
        domain_in_input_order(reversed),
        EpistemicState::empty(),
        SocialState::empty(),
        AgencyState::empty(),
    )
}

#[test]
fn domain_state_normalizes_owner_declared_order_and_freezes_a_digest() {
    let original = domain_in_input_order(false);
    let reversed = domain_in_input_order(true);

    assert_eq!(original, reversed);
    assert_eq!(original.containers()[0].container(), entity(0x10));
    assert_eq!(original.containment()[0].item(), entity(0x30));
    assert_eq!(original.container_authority()[0].actor(), actor(0x50));
    assert_eq!(original.container_authority()[0].container(), entity(0x10));
    assert_eq!(original.container_authority()[1].actor(), actor(0x50));
    assert_eq!(original.container_authority()[1].container(), entity(0x20));
    assert_eq!(original.container_authority()[2].actor(), actor(0x51));
    assert_eq!(
        original.digest().to_string(),
        "d1f7e0e6977df57cd4084fa763cc712c55a34fa4db1cbc50de478e4569f80b61"
    );
}

#[test]
fn domain_state_digest_covers_capacity_containment_and_authority() {
    let original = domain_in_input_order(false);
    let changed_capacity = DomainState::new(
        vec![
            ContainerRecord::new(entity(0x10), 3),
            ContainerRecord::new(entity(0x20), 3),
        ],
        original.containment().to_vec(),
        original.container_authority().to_vec(),
    )
    .unwrap_or_else(|error| panic!("capacity variant must be valid: {error}"));
    let changed_containment = DomainState::new(
        original.containers().to_vec(),
        vec![
            ContainmentRecord::new(entity(0x30), entity(0x20)),
            ContainmentRecord::new(entity(0x40), entity(0x20)),
        ],
        original.container_authority().to_vec(),
    )
    .unwrap_or_else(|error| panic!("containment variant must be valid: {error}"));
    let changed_authority = DomainState::new(
        original.containers().to_vec(),
        original.containment().to_vec(),
        vec![
            ContainerAuthorityRecord::new(actor(0x50), entity(0x10)),
            ContainerAuthorityRecord::new(actor(0x50), entity(0x20)),
            ContainerAuthorityRecord::new(actor(0x52), entity(0x20)),
        ],
    )
    .unwrap_or_else(|error| panic!("authority variant must be valid: {error}"));

    assert_ne!(original.digest(), changed_capacity.digest());
    assert_ne!(original.digest(), changed_containment.digest());
    assert_ne!(original.digest(), changed_authority.digest());
    assert_ne!(changed_capacity.digest(), changed_containment.digest());
    assert_ne!(changed_containment.digest(), changed_authority.digest());
}

#[test]
fn domain_state_rejects_duplicate_or_invalid_relations() {
    let source = ContainerRecord::new(entity(0x10), 1);
    let destination = ContainerRecord::new(entity(0x20), 1);

    assert_eq!(
        DomainState::new(vec![source, source], Vec::new(), Vec::new()),
        Err(DomainStateError::DuplicateContainer {
            container: entity(0x10),
        })
    );
    assert_eq!(
        DomainState::new(
            vec![source, destination],
            vec![
                ContainmentRecord::new(entity(0x30), entity(0x10)),
                ContainmentRecord::new(entity(0x30), entity(0x20)),
            ],
            Vec::new(),
        ),
        Err(DomainStateError::DuplicateContainment { item: entity(0x30) })
    );
    assert_eq!(
        DomainState::new(
            vec![source],
            Vec::new(),
            vec![
                ContainerAuthorityRecord::new(actor(0x50), entity(0x10)),
                ContainerAuthorityRecord::new(actor(0x50), entity(0x10)),
            ],
        ),
        Err(DomainStateError::DuplicateContainerAuthority {
            actor: actor(0x50),
            container: entity(0x10),
        })
    );
    assert_eq!(
        DomainState::new(
            vec![source],
            vec![ContainmentRecord::new(entity(0x30), entity(0x20))],
            Vec::new(),
        ),
        Err(DomainStateError::MissingContainmentContainer {
            item: entity(0x30),
            container: entity(0x20),
        })
    );
    assert_eq!(
        DomainState::new(
            vec![source],
            Vec::new(),
            vec![ContainerAuthorityRecord::new(actor(0x50), entity(0x20),)],
        ),
        Err(DomainStateError::MissingAuthorityContainer {
            actor: actor(0x50),
            container: entity(0x20),
        })
    );
    assert_eq!(
        DomainState::new(
            vec![source],
            vec![ContainmentRecord::new(entity(0x10), entity(0x10))],
            Vec::new(),
        ),
        Err(DomainStateError::DirectSelfContainment { item: entity(0x10) })
    );
    assert_eq!(
        DomainState::new(
            vec![source, destination],
            vec![ContainmentRecord::new(entity(0x10), entity(0x20))],
            Vec::new(),
        ),
        Err(DomainStateError::ContainerUsedAsItem { item: entity(0x10) })
    );
    assert_eq!(
        DomainState::new(
            vec![source],
            vec![
                ContainmentRecord::new(entity(0x30), entity(0x10)),
                ContainmentRecord::new(entity(0x40), entity(0x10)),
            ],
            Vec::new(),
        ),
        Err(DomainStateError::ContainerCapacityExceeded {
            container: entity(0x10),
            capacity: 1,
            actual: 2,
        })
    );
}

#[test]
fn accepted_state_exposes_and_identifies_each_owned_partition() {
    let state = accepted_in_input_order(false);

    assert_eq!(state.domain(), &domain_in_input_order(false));
    assert_eq!(state.epistemic(), &EpistemicState::empty());
    assert_eq!(state.social(), &SocialState::empty());
    assert_eq!(state.agency(), &AgencyState::empty());

    assert_eq!(
        state.domain().digest().to_string(),
        "d1f7e0e6977df57cd4084fa763cc712c55a34fa4db1cbc50de478e4569f80b61"
    );
    assert_eq!(
        state.epistemic().digest().to_string(),
        "bf0115831a7b526be11c7e9b8b2cae9a74e076c3c799ddfce810c571ef963720"
    );
    assert_eq!(
        state.social().digest().to_string(),
        "c400400c7239599ffa2adccfcb723c3418abce261de44c8238c393a5e42a6324"
    );
    assert_eq!(
        state.agency().digest().to_string(),
        "795cbbd500486dd226807b0b2fc9b171eb52ae1f4cb7b06f0e9f25d84b88d267"
    );
    assert_eq!(
        state.digest().to_string(),
        "192ac19c00631100133672ddcc52a80cc19cd7da356956bcd0c57ca21d8838a3"
    );

    let partition_digests = [
        state.domain().digest().into_bytes(),
        state.epistemic().digest().into_bytes(),
        state.social().digest().into_bytes(),
        state.agency().digest().into_bytes(),
    ];
    for (index, digest) in partition_digests.iter().enumerate() {
        assert!(
            partition_digests[..index]
                .iter()
                .all(|earlier| earlier != digest),
            "accepted partitions must have domain-separated identities"
        );
    }
}

#[test]
fn transfer_and_event_are_typed_but_do_not_mutate_state() {
    let state = domain_in_input_order(false);
    let before = state.digest();
    let delta =
        ContainmentTransferDelta::new(actor(0x50), entity(0x30), entity(0x10), entity(0x20))
            .unwrap_or_else(|error| panic!("transfer fixture must be valid: {error}"));
    let event = PhysicalEvent::item_transferred(delta);

    assert_eq!(delta.actor(), actor(0x50));
    assert_eq!(delta.item(), entity(0x30));
    assert_eq!(delta.expected_source(), entity(0x10));
    assert_eq!(delta.destination(), entity(0x20));
    let PhysicalEvent::ItemTransferred(event) = event else {
        unreachable!("item transfer constructor must produce an item-transfer event");
    };
    assert_eq!(event.actor(), delta.actor());
    assert_eq!(event.item(), delta.item());
    assert_eq!(event.source(), delta.expected_source());
    assert_eq!(event.destination(), delta.destination());
    assert_eq!(state.digest(), before);

    assert_eq!(
        ContainmentTransferDelta::new(actor(0x50), entity(0x30), entity(0x10), entity(0x10),),
        Err(ContainmentTransferError::SourceEqualsDestination {
            container: entity(0x10),
        })
    );
    assert_eq!(
        ContainmentTransferDelta::new(actor(0x50), entity(0x30), entity(0x10), entity(0x30),),
        Err(ContainmentTransferError::DirectSelfContainment {
            item: entity(0x30),
            container: entity(0x30),
        })
    );
}

#[test]
fn snapshot_is_a_cloneable_read_value_not_an_authority_cursor() {
    let state = accepted_in_input_order(false);
    let snapshot = WorldSnapshot::new(WorldRevision::from_raw(7), state.clone());
    let cloned = snapshot.clone();

    assert_eq!(snapshot.revision().get(), 7);
    assert_eq!(snapshot.accepted(), &state);
    assert_eq!(cloned, snapshot);
    assert_eq!(cloned.into_accepted(), state);
}

#[test]
fn containment_transfer_view_is_bound_to_only_the_required_facts() {
    let state = domain_in_input_order(false);
    let view =
        state.containment_transfer_view(actor(0x50), entity(0x30), entity(0x10), entity(0x20));

    assert_eq!(view.actor(), actor(0x50));
    assert_eq!(view.item(), entity(0x30));
    assert_eq!(view.source(), entity(0x10));
    assert_eq!(view.destination(), entity(0x20));
    assert_eq!(view.item_container(), Some(entity(0x10)));
    assert!(view.source_exists());
    assert!(view.actor_controls_source());
    assert_eq!(view.destination_capacity(), Some(3));
    assert_eq!(view.destination_direct_item_count(), 1);

    let absent =
        state.containment_transfer_view(actor(0x52), entity(0x60), entity(0x70), entity(0x80));
    assert_eq!(absent.item_container(), None);
    assert!(!absent.source_exists());
    assert!(!absent.actor_controls_source());
    assert_eq!(absent.destination_capacity(), None);
    assert_eq!(absent.destination_direct_item_count(), 0);
}
