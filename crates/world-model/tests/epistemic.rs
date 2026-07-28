use world_core::{ActorId, EntityId};
use world_model::{
    ActorDepartedEvent, ActorEpistemicRecord, ContainedInBelief, EpistemicState,
    EpistemicStateError, EpistemicTransitionError, EpistemicVersion, EvidenceDeliveryGeneration,
    EvidenceDeliveryId, EvidenceRecord, ItemTransferredEvent, PhysicalEvent, RelocationProcessId,
};

fn actor(byte: u8) -> ActorId {
    ActorId::from_bytes([byte; 32])
}

fn entity(byte: u8) -> EntityId {
    EntityId::from_bytes([byte; 32])
}

fn event(
    actor: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
) -> ItemTransferredEvent {
    let delta = world_model::ContainmentTransferDelta::new(actor, item, source, destination)
        .unwrap_or_else(|error| panic!("observation fixture must be valid: {error}"));
    let PhysicalEvent::ItemTransferred(event) = PhysicalEvent::item_transferred(delta) else {
        unreachable!("item transfer constructor must produce an item-transfer event");
    };
    event
}

fn generation(value: u64) -> EvidenceDeliveryGeneration {
    EvidenceDeliveryGeneration::new(value)
        .unwrap_or_else(|| panic!("fixture generation must be nonzero"))
}

fn departure(
    process_byte: u8,
    moving: ActorId,
    source: EntityId,
    destination: EntityId,
) -> ActorDepartedEvent {
    let PhysicalEvent::ActorDeparted(event) = PhysicalEvent::actor_departed(
        RelocationProcessId::from_bytes([process_byte; 32]),
        moving,
        source,
        destination,
    ) else {
        unreachable!("departure constructor must produce a departure event");
    };
    event
}

#[test]
fn assimilation_is_actor_local_versioned_and_provenance_bearing() {
    let observer = actor(0x40);
    let item = entity(0x50);
    let first_event = event(actor(0x41), item, entity(0x10), entity(0x20));
    let first = EvidenceRecord::direct_item_transfer(observer, generation(1), first_event);
    let empty = EpistemicState::empty();

    assert_eq!(empty.actor_version(observer), EpistemicVersion::EMPTY);
    assert_eq!(
        empty.next_delivery_generation(observer),
        Some(generation(1))
    );

    let accepted = empty
        .assimilate(observer, EpistemicVersion::EMPTY, vec![first])
        .unwrap_or_else(|error| panic!("first evidence must assimilate: {error}"));
    let belief = accepted
        .contained_in(observer, item)
        .unwrap_or_else(|| panic!("transfer evidence must establish a contained-in belief"));

    assert_eq!(accepted.actor_version(observer), EpistemicVersion::new(1));
    assert_eq!(
        accepted.next_delivery_generation(observer),
        Some(generation(2))
    );
    assert_eq!(accepted.evidence(), &[first]);
    assert_eq!(accepted.evidence_record(first.id()), Some(&first));
    assert_eq!(
        accepted.evidence_record(EvidenceDeliveryId::from_bytes([0xff; 32])),
        None
    );
    assert_eq!(belief.container(), entity(0x20));
    assert_eq!(belief.support(), &[first.id()]);
    assert_eq!(
        first.provenance(),
        world_model::EvidenceProvenance::DirectItemTransfer(first_event)
    );
}

#[test]
fn current_belief_support_accumulates_then_resets_on_contradicting_evidence() {
    let observer = actor(0x40);
    let item = entity(0x50);
    let first = EvidenceRecord::direct_item_transfer(
        observer,
        generation(1),
        event(actor(0x41), item, entity(0x10), entity(0x20)),
    );
    let second = EvidenceRecord::direct_item_transfer(
        observer,
        generation(2),
        event(actor(0x41), item, entity(0x10), entity(0x20)),
    );
    let third = EvidenceRecord::direct_item_transfer(
        observer,
        generation(3),
        event(actor(0x41), item, entity(0x20), entity(0x30)),
    );

    let first_two = EpistemicState::empty()
        .assimilate(observer, EpistemicVersion::EMPTY, vec![second, first])
        .unwrap_or_else(|error| panic!("canonicalized batch must assimilate: {error}"));
    let mut expected_support = vec![first.id(), second.id()];
    expected_support.sort();
    assert_eq!(
        first_two
            .contained_in(observer, item)
            .unwrap_or_else(|| panic!("belief must exist"))
            .support(),
        expected_support
    );

    let changed = first_two
        .assimilate(observer, EpistemicVersion::new(1), vec![third])
        .unwrap_or_else(|error| panic!("newer evidence must supersede the claim: {error}"));
    let belief = changed
        .contained_in(observer, item)
        .unwrap_or_else(|| panic!("belief must remain present"));
    assert_eq!(belief.container(), entity(0x30));
    assert_eq!(belief.support(), &[third.id()]);
    assert_eq!(changed.evidence().len(), 3);
}

#[test]
fn item_absence_retracts_only_the_matching_containment_belief() {
    let observer = actor(0x40);
    let item = entity(0x50);
    let other_item = entity(0x51);
    let expected_container = entity(0x20);
    let other_container = entity(0x30);
    let first = EvidenceRecord::direct_item_transfer(
        observer,
        generation(1),
        event(actor(0x41), item, entity(0x10), expected_container),
    );
    let other = EvidenceRecord::direct_item_transfer(
        observer,
        generation(2),
        event(actor(0x41), other_item, entity(0x11), other_container),
    );
    let absent =
        EvidenceRecord::direct_item_absent(observer, generation(3), item, expected_container);
    let initial = EpistemicState::empty()
        .assimilate(observer, EpistemicVersion::EMPTY, vec![first, other])
        .unwrap_or_else(|error| panic!("initial beliefs must assimilate: {error}"));

    let retracted = initial
        .assimilate(observer, EpistemicVersion::new(1), vec![absent])
        .unwrap_or_else(|error| panic!("absence evidence must assimilate: {error}"));

    assert_eq!(retracted.contained_in(observer, item), None);
    assert_eq!(
        retracted
            .contained_in(observer, other_item)
            .map(ContainedInBelief::container),
        Some(other_container)
    );
    let world_model::EvidenceProvenance::DirectItemAbsent(observation) = absent.provenance() else {
        panic!("absence constructor must retain non-locating absence meaning");
    };
    assert_eq!(observation.item(), item);
    assert_eq!(observation.expected_container(), expected_container);
}

#[test]
fn item_absence_for_another_container_leaves_the_belief_unchanged() {
    let observer = actor(0x40);
    let item = entity(0x50);
    let believed_container = entity(0x20);
    let first = EvidenceRecord::direct_item_transfer(
        observer,
        generation(1),
        event(actor(0x41), item, entity(0x10), believed_container),
    );
    let absent_elsewhere =
        EvidenceRecord::direct_item_absent(observer, generation(2), item, entity(0x30));
    let initial = EpistemicState::empty()
        .assimilate(observer, EpistemicVersion::EMPTY, vec![first])
        .unwrap_or_else(|error| panic!("initial belief must assimilate: {error}"));

    let unchanged = initial
        .assimilate(observer, EpistemicVersion::new(1), vec![absent_elsewhere])
        .unwrap_or_else(|error| panic!("unrelated absence evidence must assimilate: {error}"));

    let belief = unchanged
        .contained_in(observer, item)
        .unwrap_or_else(|| panic!("another-container absence must not retract this belief"));
    assert_eq!(belief.container(), believed_container);
    assert_eq!(belief.support(), &[first.id()]);
}

#[test]
fn assimilation_rejects_stale_versions_wrong_observers_and_generation_gaps() {
    let observer = actor(0x40);
    let evidence = EvidenceRecord::direct_item_transfer(
        observer,
        generation(1),
        event(actor(0x41), entity(0x50), entity(0x10), entity(0x20)),
    );
    let state = EpistemicState::empty();

    assert_eq!(
        state.assimilate(observer, EpistemicVersion::new(1), vec![evidence]),
        Err(EpistemicTransitionError::StaleVersion {
            expected: EpistemicVersion::new(1),
            actual: EpistemicVersion::EMPTY,
        })
    );

    let wrong_observer = EvidenceRecord::direct_item_transfer(
        actor(0x42),
        generation(1),
        event(actor(0x41), entity(0x50), entity(0x10), entity(0x20)),
    );
    assert_eq!(
        state.assimilate(observer, EpistemicVersion::EMPTY, vec![wrong_observer]),
        Err(EpistemicTransitionError::WrongObserver {
            evidence: wrong_observer.id(),
        })
    );

    let skipped = EvidenceRecord::direct_item_transfer(
        observer,
        generation(2),
        event(actor(0x41), entity(0x50), entity(0x10), entity(0x20)),
    );
    assert_eq!(
        state.assimilate(observer, EpistemicVersion::EMPTY, vec![skipped]),
        Err(EpistemicTransitionError::UnexpectedGeneration {
            expected: 1,
            actual: 2,
        })
    );
}

#[test]
fn evidence_identity_commits_the_complete_observation_body() {
    let observer = actor(0x40);
    let baseline = EvidenceRecord::direct_item_transfer(
        observer,
        generation(1),
        event(actor(0x41), entity(0x50), entity(0x10), entity(0x20)),
    );
    let changed_actor = EvidenceRecord::direct_item_transfer(
        observer,
        generation(1),
        event(actor(0x42), entity(0x50), entity(0x10), entity(0x20)),
    );
    let changed_item = EvidenceRecord::direct_item_transfer(
        observer,
        generation(1),
        event(actor(0x41), entity(0x51), entity(0x10), entity(0x20)),
    );
    let changed_source = EvidenceRecord::direct_item_transfer(
        observer,
        generation(1),
        event(actor(0x41), entity(0x50), entity(0x11), entity(0x20)),
    );
    let changed_destination = EvidenceRecord::direct_item_transfer(
        observer,
        generation(1),
        event(actor(0x41), entity(0x50), entity(0x10), entity(0x21)),
    );

    assert_ne!(baseline.id(), changed_actor.id());
    assert_ne!(baseline.id(), changed_item.id());
    assert_ne!(baseline.id(), changed_source.id());
    assert_ne!(baseline.id(), changed_destination.id());
}

#[test]
fn relocation_evidence_enters_epistemic_history_without_fabricating_a_belief() {
    let observer = actor(0x40);
    let moving = actor(0x41);
    let source = entity(0x10);
    let destination = entity(0x20);
    let first = EvidenceRecord::direct_actor_departure(
        observer,
        generation(1),
        departure(0x61, moving, source, destination),
    );
    let same_visible_observation = EvidenceRecord::direct_actor_departure(
        observer,
        generation(1),
        departure(0x62, moving, source, destination),
    );

    assert_eq!(first, same_visible_observation);

    let state = EpistemicState::empty()
        .assimilate(observer, EpistemicVersion::EMPTY, vec![first])
        .unwrap_or_else(|error| panic!("departure evidence must assimilate: {error}"));
    assert_eq!(state.evidence(), &[first]);
    assert!(state.contained_in_beliefs().is_empty());
    assert_eq!(state.actor_version(observer), EpistemicVersion::new(1));
}

#[test]
fn complete_state_validation_rejects_missing_belief_support() {
    let observer = actor(0x40);
    let evidence = EvidenceRecord::direct_item_transfer(
        observer,
        generation(1),
        event(actor(0x41), entity(0x50), entity(0x10), entity(0x20)),
    );
    let belief = ContainedInBelief::new(
        observer,
        entity(0x50),
        entity(0x20),
        vec![EvidenceDeliveryId::from_bytes([0x99; 32])],
    )
    .unwrap_or_else(|error| panic!("structural belief fixture must be valid: {error}"));

    assert_eq!(
        EpistemicState::new(
            vec![ActorEpistemicRecord::new(
                observer,
                EpistemicVersion::new(1),
                generation(1),
            )],
            vec![evidence],
            vec![belief],
        ),
        Err(EpistemicStateError::MissingBeliefSupport {
            evidence: EvidenceDeliveryId::from_bytes([0x99; 32]),
        })
    );
}
