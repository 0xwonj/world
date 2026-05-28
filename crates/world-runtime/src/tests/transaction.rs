use super::helpers::*;
use super::*;

#[test]
fn causal_runtime_commits_minimal_transfer_through_model_receiver() {
    let mut runtime = runtime(transfer_registry());
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(2), entity(3)]);
    let baseline = EventHistoryCounts::capture(&model);

    let outcome = must_ok(runtime.execute(&mut model, transfer_request()));

    let RuntimeOutcome::Committed(committed) = outcome else {
        panic!("transfer request should commit");
    };
    assert_eq!(committed.transaction().get(), 1);
    assert_eq!(
        committed
            .events()
            .iter()
            .map(|event| event.get())
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert!(model.relation_store().contains(RelationKey::new(
        entity(2),
        RelationFamily::ContainedIn,
        entity(3),
    )));
    baseline.assert_delta(&model, 1, 1);
    let Some(stored_event) = model.event_history().event(committed.events()[0]) else {
        panic!("committed event should be stored");
    };
    let stored_event = stored_event.record();
    assert_eq!(stored_event.spec(), &event_spec());
    assert_eq!(
        stored_event
            .roles()
            .iter()
            .map(|binding| (binding.role().as_str(), binding.entity()))
            .collect::<Vec<_>>(),
        vec![
            ("actor", entity(1)),
            ("destination", entity(3)),
            ("item", entity(2)),
        ]
    );
}

#[test]
fn staged_effect_reads_see_prior_changes_in_the_same_transaction() {
    let mut runtime = runtime(insert_then_transfer_registry());
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(3)]);

    let outcome = must_ok(runtime.execute(&mut model, insert_then_transfer_request()));

    let RuntimeOutcome::Committed(committed) = outcome else {
        panic!("insert-then-transfer request should commit");
    };
    assert_eq!(committed.events().len(), 2);
    assert!(model.world_store().contains_entity(entity(2)));
    assert!(model.relation_store().contains(RelationKey::new(
        entity(2),
        RelationFamily::ContainedIn,
        entity(3),
    )));
}

#[test]
fn unknown_action_rejects_without_mutating_model() {
    let mut runtime = runtime(transfer_registry());
    let mut model = WorldModel::new();
    let request = RuntimeRequest::new(
        RequestSource::Player,
        Some(entity(1)),
        definition(99),
        SimulationTime::ZERO,
        [],
        None,
    );

    let outcome = must_ok(runtime.execute(&mut model, request));

    assert_eq!(
        outcome,
        RuntimeOutcome::Rejected(RejectedOutcome::new(
            definition(99),
            RejectionReason::UnknownAction {
                action: definition(99),
            },
        ))
    );
    assert!(model.event_history().is_empty());
}

#[test]
fn missing_role_rejects_without_mutating_model() {
    let mut runtime = runtime(transfer_registry());
    let mut model = WorldModel::new();
    let request = RuntimeRequest::new(
        RequestSource::Player,
        Some(entity(1)),
        definition(2),
        SimulationTime::ZERO,
        [SubmittedRole::new(role_name("actor"), entity(1))],
        None,
    );

    let outcome = must_ok(runtime.execute(&mut model, request));

    let RuntimeOutcome::Rejected(rejected) = outcome else {
        panic!("request with missing roles should reject");
    };
    assert_eq!(
        rejected.reason(),
        &RejectionReason::MissingRoleBinding {
            role: role_name("item"),
        }
    );
    assert!(model.event_history().is_empty());
}

#[test]
fn missing_visible_entity_rejects_without_issuing_commit() {
    let mut runtime = runtime(transfer_registry());
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(3)]);
    let baseline = EventHistoryCounts::capture(&model);

    let outcome = must_ok(runtime.execute(&mut model, transfer_request()));

    assert_eq!(
        outcome,
        RuntimeOutcome::Rejected(RejectedOutcome::new(
            definition(2),
            RejectionReason::MissingEntity {
                role: role_name("item"),
                entity: entity(2),
            },
        ))
    );
    baseline.assert_unchanged(&model);
}

#[test]
fn undeclared_handler_permission_fails_before_model_application() {
    let mut runtime = runtime(registry(
        transfer_program(
            "transfer_entity",
            vec![StagePermission::EmitPhysicalEventRecord],
        ),
        transfer_action(vec![StagePermission::EmitPhysicalEventRecord]),
    ));
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(2), entity(3)]);
    let baseline = EventHistoryCounts::capture(&model);

    assert_eq!(
        runtime.execute(&mut model, transfer_request()),
        Err(RuntimeError::PermissionNotDeclared {
            operation: effect_kind("transfer_entity"),
            permission: StagePermission::MutatePhysical,
        })
    );
    baseline.assert_unchanged(&model);
    assert!(model.relation_store().is_empty());
}

#[test]
fn missing_effect_handler_fails_before_model_application() {
    let mut runtime = runtime(registry(
        transfer_program(
            "unknown_effect",
            vec![
                StagePermission::MutatePhysical,
                StagePermission::EmitPhysicalEventRecord,
            ],
        ),
        transfer_action(vec![
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ]),
    ));
    let mut model = WorldModel::new();

    assert_eq!(
        runtime.execute(&mut model, transfer_request()),
        Err(RuntimeError::MissingEffectHandler {
            kind: effect_kind("unknown_effect"),
        })
    );
    assert!(model.event_history().is_empty());
    assert!(model.relation_store().is_empty());
}

#[test]
fn missing_effect_handler_fails_before_transaction_id_is_issued() {
    let permissions = vec![
        StagePermission::MutatePhysical,
        StagePermission::EmitPhysicalEventRecord,
    ];
    let Ok(definitions) = DefinitionRegistry::new(
        [
            transfer_program_with_id(1, "unknown_effect", permissions.clone()),
            transfer_program_with_id(3, "transfer_entity", permissions.clone()),
        ],
        [
            transfer_action_with_ids(2, 1, permissions.clone()),
            transfer_action_with_ids(4, 3, permissions),
        ],
        [],
        [],
    ) else {
        panic!("test registry must be valid");
    };
    let mut runtime = runtime(definitions);
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(2), entity(3)]);

    assert_eq!(
        runtime.execute(&mut model, transfer_request()),
        Err(RuntimeError::MissingEffectHandler {
            kind: effect_kind("unknown_effect"),
        })
    );

    let outcome = must_ok(runtime.execute(&mut model, transfer_request_for(4)));
    let RuntimeOutcome::Committed(committed) = outcome else {
        panic!("valid transfer should still commit");
    };
    assert_eq!(committed.transaction().get(), 1);
}

#[test]
fn duplicate_relation_rejects_before_commit_application() {
    let mut runtime = runtime(transfer_registry());
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(2), entity(3)]);
    must_ok(runtime.execute(&mut model, transfer_request()));
    let baseline = EventHistoryCounts::capture(&model);

    assert_eq!(
        runtime.execute(&mut model, transfer_request()),
        Ok(RuntimeOutcome::Rejected(RejectedOutcome::new(
            definition(2),
            RejectionReason::RelationAlreadyPresent {
                subject: entity(2),
                family: RelationFamily::ContainedIn,
                object: entity(3),
            },
        )))
    );
    baseline.assert_unchanged(&model);
}

#[test]
fn finalizer_rejects_missing_required_event_before_model_application() {
    let registry = transfer_registry();
    let Some(program) = registry.effect_program(definition(1)) else {
        panic!("test program must exist");
    };
    let mut transaction_ids = CausalTransactionIdIssuer::new();
    let Some(transaction_id) = transaction_ids.issue() else {
        panic!("test transaction id must be available");
    };
    let transaction = CausalTransactionBuilder::new(
        CausalTransactionHeader {
            id: transaction_id,
            source: RequestSource::Tooling,
            cause: TransactionCause::Action {
                action: definition(2),
                effect_program: definition(1),
            },
            occurred_at: SimulationTime::ZERO,
            replay_level: ReplayLevel::EventRebuild,
            provenance: None,
        },
        InvalidationPackage::new(InvalidationSource::HardCommit(transaction_id)),
    );

    assert_eq!(
        CommitFinalizer::finalize_action(transaction, program),
        Err(RuntimeError::RequiredEventMissing {
            effect_program: definition(1),
            event: event_spec(),
        })
    );
}

#[test]
fn process_tick_finalizer_requires_process_tick_cause() {
    let mut transaction_ids = CausalTransactionIdIssuer::new();
    let Some(transaction_id) = transaction_ids.issue() else {
        panic!("test transaction id must be available");
    };
    let cause = TransactionCause::Action {
        action: definition(2),
        effect_program: definition(1),
    };
    let transaction = CausalTransactionBuilder::new(
        CausalTransactionHeader {
            id: transaction_id,
            source: RequestSource::Engine,
            cause,
            occurred_at: SimulationTime::ZERO,
            replay_level: ReplayLevel::AuditOnly,
            provenance: None,
        },
        InvalidationPackage::new(InvalidationSource::HardCommit(transaction_id)),
    );

    assert_eq!(
        CommitFinalizer::finalize_eventless_process_tick(transaction),
        Err(RuntimeError::InvalidProcessTransactionCause { cause })
    );
}
