use world_defs::EventRecordSpec;

use crate::ids;

/// Event emitted when a hard entity is created.
pub fn entity_created() -> EventRecordSpec {
    let Ok(spec) = EventRecordSpec::new(
        ids::event_kind("EntityCreated"),
        [ids::entity_role()],
        ids::primitive_version(),
    ) else {
        unreachable!("standard entity-created event spec is valid");
    };
    spec
}

/// Event emitted when a hard containment placement is created.
pub fn entity_placed() -> EventRecordSpec {
    let Ok(spec) = EventRecordSpec::new(
        ids::event_kind("EntityPlaced"),
        [ids::actor_role(), ids::item_role(), ids::destination_role()],
        ids::primitive_version(),
    ) else {
        unreachable!("standard entity-placed event spec is valid");
    };
    spec
}

/// Event emitted when runtime authority acquires a reservation.
pub fn reservation_acquired() -> EventRecordSpec {
    let Ok(spec) = EventRecordSpec::new(
        ids::event_kind("ReservationAcquired"),
        [ids::item_role()],
        ids::primitive_version(),
    ) else {
        unreachable!("standard reservation-acquired event spec is valid");
    };
    spec
}
