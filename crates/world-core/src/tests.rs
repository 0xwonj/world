use super::*;

#[test]
fn public_ids_reject_zero_values() {
    assert_eq!(EntityId::new(0), None);
    assert_eq!(ActorId::new(0), None);
    assert_eq!(DefinitionId::new(0), None);
    assert_eq!(ProvenanceKey::new(0), None);
    assert_eq!(VersionAnchor::new(0), None);

    assert_eq!(EntityId::new(7).map(EntityId::get), Some(7));
    assert_eq!(ActorId::try_from(9).map(ActorId::get), Ok(9));
    assert_eq!(
        DefinitionId::try_from(0).map(DefinitionId::get),
        Err(InvalidCoreValue::Zero {
            type_name: "DefinitionId"
        })
    );
}

#[test]
fn issued_ids_come_from_monotonic_issuers() {
    let mut event_ids = EventRecordIdIssuer::new();
    let mut tx_ids = CausalTransactionIdIssuer::new();
    let mut process_ids = ProcessInstanceIdIssuer::new();
    let mut activity_ids = ActivityIdIssuer::new();
    let mut handles = RuntimeEntityHandleIssuer::new();

    assert_eq!(event_ids.issue().map(EventRecordId::get), Some(1));
    assert_eq!(event_ids.issue().map(EventRecordId::get), Some(2));
    assert_eq!(tx_ids.issue().map(CausalTransactionId::get), Some(1));
    assert_eq!(process_ids.issue().map(ProcessInstanceId::get), Some(1));
    assert_eq!(activity_ids.issue().map(ActivityId::get), Some(1));
    assert_eq!(handles.issue().map(RuntimeEntityHandle::get), Some(1));

    assert_eq!(
        RuntimeEntityHandleIssuer::starting_at(0),
        Err(InvalidCoreValue::Zero {
            type_name: "RuntimeEntityHandle"
        })
    );

    let Ok(mut nearly_exhausted) = CausalTransactionIdIssuer::starting_at(u64::MAX) else {
        panic!("u64::MAX is a valid nonzero transaction id");
    };
    assert_eq!(
        nearly_exhausted.issue().map(CausalTransactionId::get),
        Some(u64::MAX)
    );
    assert_eq!(nearly_exhausted.issue(), None);
}

#[test]
fn issued_ids_are_allocation_labels_not_authority_tokens() {
    let mut first_store = EventRecordIdIssuer::new();
    let mut second_store = EventRecordIdIssuer::new();

    assert_eq!(first_store.issue(), second_store.issue());
    assert_eq!(first_store.next_value(), Some(2));
    assert_eq!(second_store.next_value(), Some(2));
}

#[test]
fn runtime_handles_and_durable_entity_ids_have_separate_types() {
    let mut handles = RuntimeEntityHandleIssuer::new();

    assert_ne!(
        core::any::type_name::<EntityId>(),
        core::any::type_name::<RuntimeEntityHandle>()
    );
    assert_eq!(EntityId::new(1).map(EntityId::get), Some(1));
    assert_eq!(handles.issue().map(RuntimeEntityHandle::get), Some(1));
}

#[test]
fn simulation_time_uses_checked_integer_arithmetic() {
    let start = SimulationTime::from_ticks(1_000);
    let duration = SimulationDuration::from_ticks(250);
    let end = start.checked_add(duration);

    assert_eq!(end.map(SimulationTime::ticks), Some(1_250));
    assert_eq!(
        end.and_then(|time| time.checked_duration_since(start))
            .map(SimulationDuration::ticks),
        Some(250)
    );
    assert_eq!(
        SimulationTime::from_ticks(u64::MAX).checked_add(SimulationDuration::from_ticks(1)),
        None
    );
    assert_eq!(
        start.checked_duration_since(SimulationTime::from_ticks(1_001)),
        None
    );
    assert!(SimulationDuration::ZERO.is_zero());
    assert_eq!(
        SimulationDuration::from_ticks(u64::MAX).checked_add(SimulationDuration::from_ticks(1)),
        None
    );
}

#[test]
fn wakeup_order_key_sorts_by_declared_scheduler_order() {
    let mut keys = [
        WakeupOrderKey::new(SimulationTime::from_ticks(10), 1, 0, 1),
        WakeupOrderKey::new(SimulationTime::from_ticks(9), 9, 9, 9),
        WakeupOrderKey::new(SimulationTime::from_ticks(10), 0, 9, 9),
        WakeupOrderKey::new(SimulationTime::from_ticks(10), 1, -1, 9),
        WakeupOrderKey::new(SimulationTime::from_ticks(10), 1, -1, 8),
    ];

    keys.sort();

    assert_eq!(keys[0].time().ticks(), 9);
    assert_eq!(keys[1].phase(), 0);
    assert_eq!(keys[2].priority(), -1);
    assert_eq!(keys[2].sequence(), 8);
    assert_eq!(keys[3].priority(), -1);
    assert_eq!(keys[3].sequence(), 9);
    assert_eq!(keys[4].sequence(), 1);
}

#[test]
fn cursors_and_epochs_advance_checked_values() {
    assert_eq!(StoreCursor::INITIAL.next().map(StoreCursor::get), Some(1));
    assert_eq!(QueryEpoch::INITIAL.next().map(QueryEpoch::get), Some(1));
    assert_eq!(StoreCursor::new(u64::MAX).next(), None);
    assert_eq!(QueryEpoch::new(u64::MAX).next(), None);
}
