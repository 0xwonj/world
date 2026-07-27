use world_core::{
    ActorId, EntityId, Microstep, NonZeroWorldRevision, SimDuration, SimMoment, SimTime,
    WorldRevision,
};

#[test]
fn cross_plane_identities_remain_purpose_specific() {
    let bytes = [0x42; 32];
    let entity = EntityId::from_bytes(bytes);
    let actor = ActorId::from_bytes(bytes);

    assert_eq!(entity.as_bytes(), &bytes);
    assert_eq!(actor.as_bytes(), &bytes);
    assert_eq!(entity.into_bytes(), bytes);
    assert_eq!(actor.into_bytes(), bytes);

    let zero = [0; 32];
    let maximum = [0xff; 32];
    assert_eq!(EntityId::from_bytes(zero).into_bytes(), zero);
    assert_eq!(ActorId::from_bytes(maximum).into_bytes(), maximum);
}

#[test]
fn virtual_time_uses_checked_integer_arithmetic() {
    let start = SimTime::from_ticks(1_000);
    let duration = SimDuration::from_ticks(250);
    let end = start.checked_add(duration);

    assert_eq!(end.map(SimTime::ticks), Some(1_250));
    assert_eq!(
        end.and_then(|time| time.checked_duration_since(start))
            .map(SimDuration::ticks),
        Some(250)
    );
    assert_eq!(
        SimTime::from_ticks(u64::MAX).checked_add(SimDuration::from_ticks(1)),
        None
    );
    assert_eq!(
        start.checked_duration_since(SimTime::from_ticks(1_001)),
        None
    );
    assert!(SimDuration::ZERO.is_zero());
    assert_eq!(
        SimDuration::from_ticks(u64::MAX).checked_add(SimDuration::from_ticks(1)),
        None
    );
}

#[test]
fn moments_order_by_time_then_microstep() {
    let mut moments = [
        SimMoment::new(SimTime::from_ticks(2), Microstep::ZERO),
        SimMoment::new(SimTime::from_ticks(1), Microstep::new(2)),
        SimMoment::new(SimTime::from_ticks(1), Microstep::new(1)),
    ];
    moments.sort();

    assert_eq!(moments[0].time().ticks(), 1);
    assert_eq!(moments[0].microstep().get(), 1);
    assert_eq!(moments[1].time().ticks(), 1);
    assert_eq!(moments[1].microstep().get(), 2);
    assert_eq!(moments[2], SimMoment::at(SimTime::from_ticks(2)));
    assert_eq!(
        SimMoment::new(SimTime::ZERO, Microstep::new(u64::MAX)).checked_next_microstep(),
        None
    );
    assert_eq!(SimMoment::ORIGIN, SimMoment::at(SimTime::ZERO));
}

#[test]
fn revisions_distinguish_the_root_from_published_values() {
    let Some(first) = WorldRevision::ROOT.checked_next() else {
        panic!("root must have a first successor");
    };
    assert_eq!(first.get(), 1);
    assert_eq!(first.previous(), WorldRevision::ROOT);
    assert_eq!(WorldRevision::from(first), WorldRevision::from_raw(1));
    assert_eq!(NonZeroWorldRevision::new(0), None);
    assert_eq!(
        WorldRevision::from_raw(u64::MAX).checked_next(),
        None,
        "world revision cannot wrap"
    );
    let Some(maximum) = NonZeroWorldRevision::new(u64::MAX) else {
        panic!("maximum u64 must be nonzero");
    };
    assert_eq!(maximum.checked_next(), None);
}
