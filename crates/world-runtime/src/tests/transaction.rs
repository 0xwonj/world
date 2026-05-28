use super::helpers::*;
use super::*;
use crate::{
    control::{AcquireReservationRequest, RuntimeControlIds},
    transaction::{EffectStager, PrimitiveStageContext},
};
use world_model::ReservationHolder;

#[test]
fn causal_runtime_commits_minimal_place_through_model_receiver() {
    let mut runtime = runtime(place_registry());
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(2), entity(3)]);
    let baseline = EventHistoryCounts::capture(&model);

    let outcome = must_ok(runtime.execute(&mut model, place_request()));

    let RuntimeOutcome::Committed(committed) = outcome else {
        panic!("place request should commit");
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
    let mut runtime = runtime(insert_then_place_registry());
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(3)]);

    let outcome = must_ok(runtime.execute(&mut model, insert_then_place_request()));

    let RuntimeOutcome::Committed(committed) = outcome else {
        panic!("insert-then-place request should commit");
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
    let mut runtime = runtime(place_registry());
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
    let mut runtime = runtime(place_registry());
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
    let mut runtime = runtime(place_registry());
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(3)]);
    let baseline = EventHistoryCounts::capture(&model);

    let outcome = must_ok(runtime.execute(&mut model, place_request()));

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
fn semantics_registry_rejects_duplicate_handlers() {
    let mut builder = PrimitiveSemanticsRegistryBuilder::new();
    if let Err(error) = builder.add_handler(TestPlaceEntity) {
        panic!("first handler should install: {error}");
    }

    assert_eq!(
        builder.add_handler(TestPlaceEntity).map(|_| ()),
        Err(RuntimeError::DuplicatePrimitiveSemantics {
            primitive: primitive_id(PRIMITIVE_PLACE_ENTITY),
        })
    );
}

#[test]
fn semantics_registry_rejects_unknown_and_mismatched_handlers() {
    let mut unknown = PrimitiveSemanticsRegistryBuilder::new();
    if let Err(error) = unknown.add_handler(TestPlaceEntity) {
        panic!("handler should install before definition matching: {error}");
    }
    assert_eq!(
        unknown.build_against(&empty_registry()).map(|_| ()),
        Err(RuntimeError::PrimitiveSemanticsForUnknownDefinition {
            primitive: primitive_id(PRIMITIVE_PLACE_ENTITY),
        })
    );

    let definitions = place_registry();
    let mut mismatched = PrimitiveSemanticsRegistryBuilder::new();
    if let Err(error) = mismatched.add_handler(MismatchedPlaceEntity) {
        panic!("mismatched handler should install before definition matching: {error}");
    }
    assert_eq!(
        mismatched.build_against(&definitions).map(|_| ()),
        Err(RuntimeError::PrimitiveSemanticsContractMismatch {
            primitive: primitive_id(PRIMITIVE_PLACE_ENTITY),
            field: "params",
        })
    );
}

#[test]
fn reservation_staging_requires_declared_capability() {
    let weak_primitive = primitive_def(
        PRIMITIVE_ACQUIRE_RESERVATION,
        "weak_reservation",
        [EffectParamDef::new(
            param_name("item"),
            EffectParamKind::EntityRole,
        )],
        [StagePermission::Validate],
        EventContract::default(),
    );
    let operation = op(weak_primitive.id(), [arg("item", "item")], []);
    let transaction_id = transaction(1);
    let mut transaction_builder = CausalTransactionBuilder::new(
        CausalTransactionHeader {
            id: transaction_id,
            source: RequestSource::Tooling,
            cause: TransactionCause::Action {
                action: definition(2),
                effect_program: definition(1),
            },
            occurred_at: SimulationTime::ZERO,
            replay_level: ReplayLevel::AuditOnly,
            provenance: None,
        },
        InvalidationPackage::new(InvalidationSource::HardCommit(transaction_id)),
    );
    let model = WorldModel::new();
    let action = place_action(vec![StagePermission::Validate]);
    let Ok(bound) = place_request().bind(&action) else {
        panic!("test request must bind");
    };

    {
        let mut stager = EffectStager::new(&model, &mut transaction_builder);
        let mut event_ids = EventRecordIdIssuer::new();
        let mut control_ids = RuntimeControlIds::new();
        let mut context =
            PrimitiveStageContext::new(&bound, &mut stager, &mut event_ids, &mut control_ids);

        assert_eq!(
            context.stage_reservation_acquire(
                PrimitiveInvocation::new(&operation, &weak_primitive),
                AcquireReservationRequest::new(
                    ReservationHolder::Runtime,
                    ReservationTarget::Entity(entity(2)),
                    SimulationTime::ZERO,
                    None,
                ),
            ),
            Err(RuntimeError::PermissionNotDeclared {
                primitive: weak_primitive.id(),
                permission: StagePermission::AcquireReservation,
            })
        );
    }

    assert!(transaction_builder.into_parts().control_changes.is_empty());
}

#[test]
fn missing_action_semantics_fails_registry_build() {
    let definitions = registry(
        unknown_semantics_primitive(),
        place_program(primitive_id(PRIMITIVE_UNKNOWN_SEMANTICS)),
        place_action(vec![
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ]),
    );

    assert_eq!(
        PrimitiveSemanticsRegistryBuilder::new()
            .build_against(&definitions)
            .map(|_| ()),
        Err(RuntimeError::MissingPrimitiveSemantics {
            primitive: primitive_id(PRIMITIVE_UNKNOWN_SEMANTICS),
        })
    );
}

#[test]
fn process_definition_programs_do_not_require_action_semantics() {
    let definitions = process_registry();

    let Ok(registry) = PrimitiveSemanticsRegistry::empty_checked(&definitions) else {
        panic!("process-only definitions should not require action primitive handlers");
    };
    assert!(
        registry
            .handler(primitive_id(PRIMITIVE_SCHEDULE_PROCESS))
            .is_none()
    );
}

#[test]
fn duplicate_relation_rejects_before_commit_application() {
    let mut runtime = runtime(place_registry());
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(2), entity(3)]);
    must_ok(runtime.execute(&mut model, place_request()));
    let baseline = EventHistoryCounts::capture(&model);

    assert_eq!(
        runtime.execute(&mut model, place_request()),
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
    let registry = place_registry();
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MismatchedPlaceEntity;

impl PrimitiveSemantics for MismatchedPlaceEntity {
    fn primitive(&self) -> EffectPrimitiveId {
        primitive_id(PRIMITIVE_PLACE_ENTITY)
    }

    fn contract(&self) -> PrimitiveSemanticsContract {
        PrimitiveSemanticsContract::new(
            [],
            [
                StagePermission::ReadWorld,
                StagePermission::MutatePhysical,
                StagePermission::EmitPhysicalEventRecord,
            ],
            EventContract::new([event_spec()]),
            ReplayLevel::EventRebuild,
            version(1),
        )
    }

    fn validate(
        &self,
        _invocation: PrimitiveInvocation<'_>,
        _context: &mut PrimitiveValidationContext<'_>,
    ) -> Result<(), PrimitiveValidationFailure> {
        unreachable!("mismatched handler should never execute")
    }

    fn stage(
        &self,
        _invocation: PrimitiveInvocation<'_>,
        _context: &mut PrimitiveStageContext<'_, '_, '_, '_>,
    ) -> Result<(), RuntimeError> {
        unreachable!("mismatched handler should never execute")
    }
}
