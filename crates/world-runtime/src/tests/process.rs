use super::helpers::*;
use super::*;

#[test]
fn process_start_schedules_first_wakeup_through_runtime_control() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();

    let outcome = must_ok(runtime.start_process(&mut model, start_process_request(2, 10)));

    let ProcessTransition::Started { process, wakeup } = outcome.transition() else {
        panic!("process start should report a started transition");
    };
    assert_eq!(process.id().get(), 1);
    assert_eq!(wakeup.id().get(), 1);
    assert!(
        model
            .runtime_control_store()
            .process(process.id())
            .is_some()
    );
    assert_eq!(
        model
            .runtime_control_store()
            .due_wakeups(SimulationTime::from_ticks(10))
            .map(|record| record.id())
            .collect::<Vec<_>>(),
        vec![wakeup.id()]
    );
    assert_eq!(outcome.application().update_cursor().get(), 0);
}

#[test]
fn wait_process_cancels_current_wakeup() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();
    let (process_id, first_wakeup) = start_test_process(&mut runtime, &mut model);

    let waiting = must_ok(runtime.wait_process(
        &mut model,
        process_id,
        WaitCondition::Host,
        SimulationTime::from_ticks(1),
        Some(provenance(90)),
    ));

    assert!(matches!(
        waiting.transition(),
        ProcessTransition::Waiting {
            condition: WaitCondition::Host,
            ..
        }
    ));
    assert!(matches!(
        model
            .runtime_control_store()
            .scheduled_wakeup(first_wakeup)
            .map(|record| record.status()),
        Some(world_model::ScheduledWakeupStatus::Canceled { .. })
    ));
}

#[test]
fn resume_process_schedules_next_wakeup_after_wait() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();
    let (process_id, _) = start_test_process(&mut runtime, &mut model);
    must_ok(runtime.wait_process(
        &mut model,
        process_id,
        WaitCondition::Host,
        SimulationTime::from_ticks(1),
        None,
    ));

    let resumed = must_ok(runtime.resume_process(
        &mut model,
        process_id,
        WakeupScheduleKey::new(SimulationTime::from_ticks(11), 0, 0),
        SimulationTime::from_ticks(2),
        Some(provenance(91)),
    ));

    assert!(matches!(
        resumed.transition(),
        ProcessTransition::Resumed { .. }
    ));
    assert_eq!(
        model
            .runtime_control_store()
            .due_wakeups(SimulationTime::from_ticks(11))
            .count(),
        1
    );
}

#[test]
fn pause_process_records_pause_reason() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();
    let (process_id, _) = start_test_process(&mut runtime, &mut model);

    let paused = must_ok(runtime.pause_process(
        &mut model,
        process_id,
        PauseReason::Host,
        SimulationTime::from_ticks(3),
        None,
    ));

    assert!(matches!(
        paused.transition(),
        ProcessTransition::Paused {
            reason: PauseReason::Host,
            ..
        }
    ));
}

#[test]
fn interrupt_process_records_interrupt_reason() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();
    let (process_id, _) = start_test_process(&mut runtime, &mut model);

    let interrupted = must_ok(runtime.interrupt_process(
        &mut model,
        process_id,
        InterruptReason::ReservationLost,
        SimulationTime::from_ticks(4),
        None,
    ));

    assert!(matches!(
        interrupted.transition(),
        ProcessTransition::Interrupted {
            reason: InterruptReason::ReservationLost,
            ..
        }
    ));
}

#[test]
fn abandon_process_prevents_later_resume() {
    let mut runtime = runtime(process_registry());
    let mut model = WorldModel::new();
    let (process_id, _) = start_test_process(&mut runtime, &mut model);

    let abandoned = must_ok(runtime.abandon_process(
        &mut model,
        process_id,
        SimulationTime::from_ticks(5),
        None,
    ));

    assert!(matches!(
        abandoned.transition(),
        ProcessTransition::Abandoned { .. }
    ));
    assert_eq!(
        runtime.resume_process(
            &mut model,
            process_id,
            WakeupScheduleKey::new(SimulationTime::from_ticks(12), 0, 0),
            SimulationTime::from_ticks(6),
            None,
        ),
        Err(RuntimeError::InvalidProcessLifecycleTransition {
            process: process_id,
            lifecycle: world_model::ProcessLifecycle::Abandoned,
        })
    );
}
