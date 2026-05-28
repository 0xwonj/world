use super::helpers::*;
use super::*;

#[test]
fn runtime_control_ids_and_wakeup_sequence_hydrate_from_model_state() {
    let mut first_runtime = runtime(process_registry());
    let mut model = WorldModel::new();

    let first = must_ok(first_runtime.start_process(&mut model, start_process_request(2, 10)));
    let ProcessTransition::Started {
        process: first_process,
        wakeup: first_wakeup,
    } = first.transition()
    else {
        panic!("process start should report a started transition");
    };
    assert_eq!(first_process.id().get(), 1);
    assert_eq!(first_wakeup.id().get(), 1);
    assert_eq!(first_wakeup.order().sequence(), 0);

    let mut hydrated = runtime_for_model(process_registry(), &model);
    let second = must_ok(hydrated.start_process(&mut model, start_process_request(2, 11)));
    let ProcessTransition::Started {
        process: second_process,
        wakeup: second_wakeup,
    } = second.transition()
    else {
        panic!("hydrated process start should report a started transition");
    };
    assert_eq!(second_process.id().get(), 2);
    assert_eq!(second_wakeup.id().get(), 2);
    assert_eq!(second_wakeup.order().sequence(), 1);
}

#[test]
fn scheduler_assigns_same_time_sequence_and_drains_in_order() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();

    let first = must_ok(runtime.schedule_wakeup(
        &mut model,
        ScheduleWakeupRequest::new(
            WakeupScheduleKey::new(SimulationTime::from_ticks(5), 0, 0),
            WakeupTarget::HostInputOpportunity,
            SimulationTime::ZERO,
            None,
        ),
    ));
    let second = must_ok(runtime.schedule_wakeup(
        &mut model,
        ScheduleWakeupRequest::new(
            WakeupScheduleKey::new(SimulationTime::from_ticks(5), 0, 0),
            WakeupTarget::HostInputOpportunity,
            SimulationTime::ZERO,
            None,
        ),
    ));

    let due = model
        .runtime_control_store()
        .due_wakeups(SimulationTime::from_ticks(5))
        .map(|record| (record.id(), record.order().sequence()))
        .collect::<Vec<_>>();
    assert_eq!(due, vec![(first.wakeup(), 0), (second.wakeup(), 1)]);
}

#[test]
fn scheduler_drain_reschedules_then_completes_process() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();
    let started = must_ok(runtime.start_process(&mut model, start_process_request(2, 1)));
    let ProcessTransition::Started { process, wakeup } = started.transition() else {
        panic!("process start should report a started transition");
    };
    let process_id = process.id();
    let first_wakeup = wakeup.id();

    let first = must_ok(runtime.drain_scheduler(
        &mut model,
        DrainRequest::new(SimulationTime::from_ticks(1), DrainBudget::new(4)),
    ));
    assert_eq!(first.outcome(), &DrainOutcome::Quiescent);
    assert_single_wakeup_result(&first, WakeupDrainResult::Rescheduled);
    assert_eq!(
        model
            .event_history()
            .transaction(transaction(1))
            .map(|stored| stored.record().cause()),
        Some(TransactionCause::ProcessTick {
            process: process_id,
            process_definition: definition(3),
            resolution: ResolutionTier::Concrete,
            wakeup: first_wakeup,
        })
    );

    let second = must_ok(runtime.drain_scheduler(
        &mut model,
        DrainRequest::new(SimulationTime::from_ticks(2), DrainBudget::new(4)),
    ));
    assert_eq!(second.outcome(), &DrainOutcome::Quiescent);
    assert_single_wakeup_result(&second, WakeupDrainResult::Completed);
    assert!(matches!(
        model
            .runtime_control_store()
            .process(process_id)
            .map(|record| record.lifecycle()),
        Some(world_model::ProcessLifecycle::Completed)
    ));
    assert_eq!(
        model
            .runtime_control_store()
            .due_wakeups(SimulationTime::from_ticks(2))
            .count(),
        0
    );
    assert_eq!(model.event_history().transaction_count(), 2);
}

#[test]
fn scheduler_drain_persists_missing_definition_failure() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();
    let started = must_ok(runtime.start_process(&mut model, start_process_request(2, 1)));
    let ProcessTransition::Started { process, wakeup } = started.transition() else {
        panic!("process start should report a started transition");
    };

    let mut hydrated = runtime_for_model(empty_registry(), &model);
    let report = must_ok(hydrated.drain_scheduler(
        &mut model,
        DrainRequest::new(SimulationTime::from_ticks(1), DrainBudget::new(4)),
    ));

    assert_eq!(report.outcome(), &DrainOutcome::Quiescent);
    assert_single_wakeup_result(
        &report,
        WakeupDrainResult::Failed(ProcessFailureReason::MissingDefinition),
    );
    assert!(matches!(
        model
            .runtime_control_store()
            .process(process.id())
            .map(|record| record.lifecycle()),
        Some(world_model::ProcessLifecycle::Failed {
            reason: ProcessFailureReason::MissingDefinition,
        })
    ));
    assert!(matches!(
        model
            .runtime_control_store()
            .scheduled_wakeup(wakeup.id())
            .map(|record| record.status()),
        Some(world_model::ScheduledWakeupStatus::Consumed { .. })
    ));
    assert_eq!(model.event_history().transaction_count(), 1);
    assert_eq!(model.event_history().event_count(), 0);
}

#[test]
fn scheduler_drain_persists_unsupported_resolution_failure() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();
    let started = must_ok(runtime.start_process(&mut model, start_process_request(2, 1)));
    let ProcessTransition::Started { process, .. } = started.transition() else {
        panic!("process start should report a started transition");
    };

    let mut hydrated = runtime_for_model(
        process_registry_with_resolutions([ResolutionTier::Abstract]),
        &model,
    );
    let report = must_ok(hydrated.drain_scheduler(
        &mut model,
        DrainRequest::new(SimulationTime::from_ticks(1), DrainBudget::new(4)),
    ));

    assert_eq!(report.outcome(), &DrainOutcome::Quiescent);
    assert_single_wakeup_result(
        &report,
        WakeupDrainResult::Failed(ProcessFailureReason::UnsupportedResolution),
    );
    assert!(matches!(
        model
            .runtime_control_store()
            .process(process.id())
            .map(|record| record.lifecycle()),
        Some(world_model::ProcessLifecycle::Failed {
            reason: ProcessFailureReason::UnsupportedResolution,
        })
    ));
    assert_eq!(model.event_history().transaction_count(), 1);
    assert_eq!(model.event_history().event_count(), 0);
}

#[test]
fn scheduler_reports_host_input_without_consuming_wakeup() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();
    let scheduled = must_ok(runtime.schedule_wakeup(
        &mut model,
        ScheduleWakeupRequest::new(
            WakeupScheduleKey::new(SimulationTime::from_ticks(5), 0, 0),
            WakeupTarget::HostInputOpportunity,
            SimulationTime::ZERO,
            Some(provenance(70)),
        ),
    ));

    let report = must_ok(runtime.drain_scheduler(
        &mut model,
        DrainRequest::new(SimulationTime::from_ticks(5), DrainBudget::new(0)),
    ));

    assert_eq!(
        report.outcome(),
        &DrainOutcome::InputOpportunity {
            wakeup: scheduled.wakeup(),
        }
    );
    assert!(report.processed().is_empty());
    assert!(model
        .runtime_control_store()
        .scheduled_wakeup(scheduled.wakeup())
        .is_some_and(|record| record.status() == &world_model::ScheduledWakeupStatus::Scheduled));

    must_ok(runtime.acknowledge_host_input_wakeup(
        &mut model,
        scheduled.wakeup(),
        SimulationTime::from_ticks(5),
        Some(provenance(71)),
    ));
    assert!(
        model
            .runtime_control_store()
            .scheduled_wakeup(scheduled.wakeup())
            .is_some_and(|record| matches!(
                record.status(),
                world_model::ScheduledWakeupStatus::Consumed { .. }
            ))
    );
    assert_eq!(
        model
            .runtime_control_store()
            .due_wakeups(SimulationTime::from_ticks(5))
            .count(),
        0
    );
}

#[test]
fn scheduler_skips_stale_process_wakeup_with_accepted_state() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();
    let started = must_ok(runtime.start_process(&mut model, start_process_request(1, 1)));
    let ProcessTransition::Started { process, .. } = started.transition() else {
        panic!("process start should report a started transition");
    };
    let process_id = process.id();

    let completed = must_ok(runtime.drain_scheduler(
        &mut model,
        DrainRequest::new(SimulationTime::from_ticks(1), DrainBudget::new(4)),
    ));
    assert_single_wakeup_result(&completed, WakeupDrainResult::Completed);
    let stale = must_ok(runtime.schedule_wakeup(
        &mut model,
        ScheduleWakeupRequest::new(
            WakeupScheduleKey::new(SimulationTime::from_ticks(2), 0, 0),
            WakeupTarget::Process(process_id),
            SimulationTime::from_ticks(1),
            None,
        ),
    ));

    let report = must_ok(runtime.drain_scheduler(
        &mut model,
        DrainRequest::new(SimulationTime::from_ticks(2), DrainBudget::new(4)),
    ));

    assert_single_wakeup_result(
        &report,
        WakeupDrainResult::Skipped(world_model::StaleWakeupReason::TerminalProcess),
    );
    assert!(matches!(
        model
            .runtime_control_store()
            .scheduled_wakeup(stale.wakeup())
            .map(|record| record.status()),
        Some(world_model::ScheduledWakeupStatus::Skipped { .. })
    ));
}
