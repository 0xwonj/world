use super::helpers::*;
use super::*;

#[test]
fn reservation_acquire_effect_commits_through_execute_atomically() {
    let mut runtime = runtime(reservation_registry());
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(2), entity(3)]);
    let baseline = EventHistoryCounts::capture(&model);

    let outcome = must_ok(runtime.execute(&mut model, transfer_request()));

    let RuntimeOutcome::Committed(committed) = outcome else {
        panic!("reservation request should commit");
    };
    assert_eq!(committed.transaction().get(), 1);
    baseline.assert_delta(&model, 1, 1);
    let after_commit = EventHistoryCounts::capture(&model);
    let target = ReservationTarget::Entity(entity(2));
    let reservation = held_reservation_for(&model, &target);
    assert_eq!(
        reservation.state(),
        &ReservationState::Held {
            acquired_at: SimulationTime::from_ticks(12),
        }
    );

    let duplicate = must_ok(runtime.execute(&mut model, transfer_request()));
    assert_eq!(
        duplicate,
        RuntimeOutcome::Rejected(RejectedOutcome::new(
            definition(2),
            RejectionReason::ReservationAlreadyHeld {
                target: ReservationTarget::Entity(entity(2)),
            },
        ))
    );
    after_commit.assert_unchanged(&model);
}
