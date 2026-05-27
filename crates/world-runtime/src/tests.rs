use world_core::{
    CausalSource, CausalTransactionId, CausalTransactionIdIssuer, DefinitionId, EntityId,
    EventRecordIdIssuer, ProvenanceKey, ReplayLevel, SimulationTime, VersionAnchor,
};
use world_defs::{
    ActionDef, DefinitionName, DefinitionRegistry, EffectKind, EffectOp, EffectProgramDef,
    EventContract, EventKind, EventRecordSpec, RoleDef, RoleName, RoleType, StagePermission,
};
use world_model::{
    AcceptedHardCommit, HardStateChange, InvalidationPackage, InvalidationSource, RelationFamily,
    RelationKey, StoreFamily, TransactionCommit, WorldModel,
};

use super::*;
use crate::{
    commit::CommitFinalizer,
    transaction::{CausalTransactionBuilder, CausalTransactionHeader},
};

fn entity(value: u64) -> EntityId {
    let Some(id) = EntityId::new(value) else {
        panic!("test entity ids must be nonzero");
    };
    id
}

fn definition(value: u64) -> DefinitionId {
    let Some(id) = DefinitionId::new(value) else {
        panic!("test definition ids must be nonzero");
    };
    id
}

fn transaction(value: u64) -> CausalTransactionId {
    let Ok(mut issuer) = CausalTransactionIdIssuer::starting_at(value) else {
        panic!("test transaction id issuer must be valid");
    };
    let Some(id) = issuer.issue() else {
        panic!("test transaction id must be available");
    };
    id
}

fn version(value: u64) -> VersionAnchor {
    let Some(id) = VersionAnchor::new(value) else {
        panic!("test version anchors must be nonzero");
    };
    id
}

fn provenance(value: u64) -> ProvenanceKey {
    let Some(id) = ProvenanceKey::new(value) else {
        panic!("test provenance keys must be nonzero");
    };
    id
}

fn role_name(value: &'static str) -> RoleName {
    let Some(name) = RoleName::new(value) else {
        panic!("test role names must be non-empty");
    };
    name
}

fn role_type(value: &'static str) -> RoleType {
    let Some(name) = RoleType::new(value) else {
        panic!("test role types must be non-empty");
    };
    name
}

fn definition_name(value: &'static str) -> DefinitionName {
    let Some(name) = DefinitionName::new(value) else {
        panic!("test definition names must be non-empty");
    };
    name
}

fn effect_kind(value: &'static str) -> EffectKind {
    let Some(name) = EffectKind::new(value) else {
        panic!("test effect kinds must be non-empty");
    };
    name
}

fn event_kind(value: &'static str) -> EventKind {
    let Some(name) = EventKind::new(value) else {
        panic!("test event kinds must be non-empty");
    };
    name
}

fn event_spec() -> EventRecordSpec {
    let Ok(spec) = EventRecordSpec::new(
        event_kind("EntityTransferred"),
        [
            role_name("actor"),
            role_name("item"),
            role_name("destination"),
        ],
        version(1),
    ) else {
        panic!("test event specs must be valid");
    };
    spec
}

fn role(name: &'static str) -> RoleDef {
    RoleDef::new(role_name(name), role_type("entity"))
}

fn op(
    kind: &'static str,
    permissions: impl IntoIterator<Item = StagePermission>,
    emitted_events: impl IntoIterator<Item = EventRecordSpec>,
) -> EffectOp {
    let Ok(op) = EffectOp::new(effect_kind(kind), permissions, emitted_events) else {
        panic!("test effect op must be valid");
    };
    op
}

fn transfer_program_with_id(
    id: u64,
    kind: &'static str,
    permissions: Vec<StagePermission>,
) -> EffectProgramDef {
    let event = event_spec();
    let Ok(program) = EffectProgramDef::new(
        definition(id),
        definition_name("transfer"),
        [op(kind, permissions, [event.clone()])],
        EventContract::new([event]),
        ReplayLevel::EventRebuild,
        version(1),
    ) else {
        panic!("test effect program must be valid");
    };
    program
}

fn transfer_program(kind: &'static str, permissions: Vec<StagePermission>) -> EffectProgramDef {
    transfer_program_with_id(1, kind, permissions)
}

fn transfer_action_with_ids(
    id: u64,
    effect_program: u64,
    stage_permissions: Vec<StagePermission>,
) -> ActionDef {
    let event = event_spec();
    let Ok(action) = ActionDef::new(
        definition(id),
        definition_name("move_item"),
        [role("actor"), role("item"), role("destination")],
        [],
        [],
        definition(effect_program),
        EventContract::new([event]),
        stage_permissions,
        version(2),
    ) else {
        panic!("test action must be valid");
    };
    action
}

fn transfer_action(stage_permissions: Vec<StagePermission>) -> ActionDef {
    transfer_action_with_ids(2, 1, stage_permissions)
}

fn registry(program: EffectProgramDef, action: ActionDef) -> DefinitionRegistry {
    let Ok(registry) = DefinitionRegistry::new([program], [action], [], []) else {
        panic!("test registry must be valid");
    };
    registry
}

fn transfer_registry() -> DefinitionRegistry {
    registry(
        transfer_program(
            "transfer_entity",
            vec![
                StagePermission::MutatePhysical,
                StagePermission::EmitPhysicalEventRecord,
            ],
        ),
        transfer_action(vec![
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ]),
    )
}

fn insert_then_transfer_registry() -> DefinitionRegistry {
    let event = event_spec();
    let Ok(program) = EffectProgramDef::new(
        definition(1),
        definition_name("insert_then_transfer"),
        [
            op(
                "insert_entity",
                [
                    StagePermission::MutatePhysical,
                    StagePermission::EmitPhysicalEventRecord,
                ],
                [event.clone()],
            ),
            op(
                "transfer_entity",
                [
                    StagePermission::MutatePhysical,
                    StagePermission::EmitPhysicalEventRecord,
                ],
                [event.clone()],
            ),
        ],
        EventContract::new([event.clone()]),
        ReplayLevel::EventRebuild,
        version(1),
    ) else {
        panic!("test effect program must be valid");
    };
    let Ok(action) = ActionDef::new(
        definition(2),
        definition_name("create_and_move_item"),
        [
            role("actor"),
            role("entity"),
            role("item"),
            role("destination"),
        ],
        [],
        [],
        definition(1),
        EventContract::new([event]),
        [
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(2),
    ) else {
        panic!("test action must be valid");
    };

    registry(program, action)
}

fn transfer_request_for(action: u64) -> RuntimeRequest {
    RuntimeRequest::new(
        RequestSource::Player,
        Some(entity(1)),
        definition(action),
        SimulationTime::from_ticks(12),
        [
            SubmittedRole::new(role_name("actor"), entity(1)),
            SubmittedRole::new(role_name("item"), entity(2)),
            SubmittedRole::new(role_name("destination"), entity(3)),
        ],
        Some(provenance(1)),
    )
}

fn transfer_request() -> RuntimeRequest {
    transfer_request_for(2)
}

fn insert_then_transfer_request() -> RuntimeRequest {
    RuntimeRequest::new(
        RequestSource::Player,
        Some(entity(1)),
        definition(2),
        SimulationTime::from_ticks(13),
        [
            SubmittedRole::new(role_name("actor"), entity(1)),
            SubmittedRole::new(role_name("entity"), entity(2)),
            SubmittedRole::new(role_name("item"), entity(2)),
            SubmittedRole::new(role_name("destination"), entity(3)),
        ],
        Some(provenance(2)),
    )
}

fn runtime(definitions: DefinitionRegistry) -> CausalRuntime {
    let Ok(transaction_ids) = CausalTransactionIdIssuer::starting_at(1) else {
        panic!("test transaction id issuer must be valid");
    };
    let Ok(event_ids) = EventRecordIdIssuer::starting_at(1) else {
        panic!("test event id issuer must be valid");
    };
    CausalRuntime::with_issuers(definitions, transaction_ids, event_ids)
}

fn seed_entities(model: &mut WorldModel, seed_transaction: u64, entities: &[EntityId]) {
    let transaction_id = transaction(seed_transaction);
    let mut invalidation = InvalidationPackage::new(InvalidationSource::HardCommit(transaction_id));
    invalidation
        .mark_authority_class(world_core::AuthorityClass::Hard)
        .mark_store_family(StoreFamily::EventHistory)
        .mark_store_family(StoreFamily::World);

    let commit = match AcceptedHardCommit::new(
        TransactionCommit::new(
            transaction_id,
            CausalSource::Tooling,
            definition(90),
            definition(91),
            ReplayLevel::EventRebuild,
            SimulationTime::ZERO,
            None,
        ),
        [],
        entities
            .iter()
            .map(|entity| HardStateChange::insert_entity(*entity, None, None)),
        invalidation,
    ) {
        Ok(commit) => commit,
        Err(error) => panic!("seed hard commit should be valid: {error}"),
    };
    if let Err(error) = model.apply_hard_commit(commit) {
        panic!("seed hard commit should apply: {error}");
    }
}

fn must_ok<T>(result: Result<T, RuntimeError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("unexpected runtime error: {error}"),
    }
}

#[test]
fn causal_runtime_commits_minimal_transfer_through_model_receiver() {
    let mut runtime = runtime(transfer_registry());
    let mut model = WorldModel::new();
    seed_entities(&mut model, 100, &[entity(1), entity(2), entity(3)]);

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
    assert_eq!(model.event_history().transaction_count(), 2);
    assert_eq!(model.event_history().event_count(), 1);
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
    assert_eq!(model.event_history().transaction_count(), 1);
    assert_eq!(model.event_history().event_count(), 0);
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

    assert_eq!(
        runtime.execute(&mut model, transfer_request()),
        Err(RuntimeError::PermissionNotDeclared {
            operation: effect_kind("transfer_entity"),
            permission: StagePermission::MutatePhysical,
        })
    );
    assert_eq!(model.event_history().transaction_count(), 1);
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
    assert_eq!(model.event_history().transaction_count(), 2);
    assert_eq!(model.event_history().event_count(), 1);
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
            action: definition(2),
            effect_program: definition(1),
            occurred_at: SimulationTime::ZERO,
            replay_level: ReplayLevel::EventRebuild,
            provenance: None,
        },
        InvalidationPackage::new(InvalidationSource::HardCommit(transaction_id)),
    );

    assert_eq!(
        CommitFinalizer::finalize(transaction, program),
        Err(RuntimeError::RequiredEventMissing {
            effect_program: definition(1),
            event: event_spec(),
        })
    );
}
