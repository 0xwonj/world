use world_core::{
    CausalTransactionIdIssuer, DefinitionId, EntityId, EventRecordIdIssuer, ReplayLevel,
    SimulationTime, VersionAnchor,
};
use world_defs::{
    ActionDef, DefinitionName, DefinitionRegistryBuilder, EffectArgBinding, EffectOp,
    EffectProgramDef, EventContract, RoleDef, RoleName, RoleType, StagePermission,
};
use world_model::{RelationFamily, RelationKey, ReservationTarget, WorldModel};
use world_runtime::{
    CausalRuntime, PrimitiveSemanticsRegistryBuilder, RejectionReason, RequestSource,
    RuntimeOutcome, RuntimeRequest, SubmittedRole,
};
use world_standard_runtime::StandardPrimitiveSemantics;

#[test]
fn standard_definitions_and_semantics_execute_place() {
    let definitions = definitions_with_actions();
    let mut semantics = PrimitiveSemanticsRegistryBuilder::new();
    if let Err(error) = semantics.install(&StandardPrimitiveSemantics) {
        panic!("standard semantics should install: {error}");
    }
    let Ok(semantics) = semantics.build_against(&definitions) else {
        panic!("standard semantics should match standard definitions");
    };
    let mut runtime = match CausalRuntime::with_hard_issuers_for_empty_model(
        definitions,
        semantics,
        transaction_ids(),
        event_ids(),
    ) {
        Ok(runtime) => runtime,
        Err(error) => panic!("standard runtime should construct: {error}"),
    };
    let mut model = WorldModel::new();
    for entity in [entity(1), entity(2), entity(3)] {
        match runtime.execute(&mut model, create_request(entity)) {
            Ok(RuntimeOutcome::Committed(_)) => {}
            Ok(other) => panic!("standard create should commit, got {other:?}"),
            Err(error) => panic!("standard create should execute: {error}"),
        }
    }

    let outcome = match runtime.execute(&mut model, place_request()) {
        Ok(outcome) => outcome,
        Err(error) => panic!("standard place should execute: {error}"),
    };

    let RuntimeOutcome::Committed(committed) = outcome else {
        panic!("standard place should commit");
    };
    assert_eq!(committed.transaction().get(), 4);
    assert!(model.relation_store().contains(RelationKey::new(
        entity(2),
        RelationFamily::ContainedIn,
        entity(3),
    )));
    let Some(placed) = model
        .event_history()
        .events()
        .last()
        .map(|event| event.record())
    else {
        panic!("place should commit an event");
    };
    assert_eq!(placed.spec(), &world_standard::events::entity_placed());
    assert_event_roles(
        placed,
        &[
            (world_standard::ids::actor_role(), entity(1)),
            (world_standard::ids::destination_role(), entity(3)),
            (world_standard::ids::item_role(), entity(2)),
        ],
    );
}

#[test]
fn standard_reservation_semantics_commit_and_reject_duplicate() {
    let definitions = definitions_with_actions();
    let mut semantics = PrimitiveSemanticsRegistryBuilder::new();
    if let Err(error) = semantics.install(&StandardPrimitiveSemantics) {
        panic!("standard semantics should install: {error}");
    }
    let Ok(semantics) = semantics.build_against(&definitions) else {
        panic!("standard semantics should match standard definitions");
    };
    let mut runtime = match CausalRuntime::with_hard_issuers_for_empty_model(
        definitions,
        semantics,
        transaction_ids(),
        event_ids(),
    ) {
        Ok(runtime) => runtime,
        Err(error) => panic!("standard runtime should construct: {error}"),
    };
    let mut model = WorldModel::new();
    for entity in [entity(1), entity(2)] {
        match runtime.execute(&mut model, create_request(entity)) {
            Ok(RuntimeOutcome::Committed(_)) => {}
            Ok(other) => panic!("standard create should commit, got {other:?}"),
            Err(error) => panic!("standard create should execute: {error}"),
        }
    }

    match runtime.execute(&mut model, reservation_request()) {
        Ok(RuntimeOutcome::Committed(_)) => {}
        Ok(other) => panic!("standard reservation should commit, got {other:?}"),
        Err(error) => panic!("standard reservation should execute: {error}"),
    }
    let Some(acquired) = model
        .event_history()
        .events()
        .last()
        .map(|event| event.record())
    else {
        panic!("reservation should commit an event");
    };
    assert_eq!(
        acquired.spec(),
        &world_standard::events::reservation_acquired()
    );
    assert_event_roles(acquired, &[(world_standard::ids::item_role(), entity(2))]);

    let duplicate = match runtime.execute(&mut model, reservation_request()) {
        Ok(outcome) => outcome,
        Err(error) => panic!("standard duplicate reservation should reject: {error}"),
    };
    assert_eq!(
        duplicate,
        RuntimeOutcome::Rejected(world_runtime::RejectedOutcome::new(
            definition(22),
            RejectionReason::ReservationAlreadyHeld {
                target: ReservationTarget::Entity(entity(2)),
            },
        ))
    );
}

fn definitions_with_actions() -> world_defs::DefinitionRegistry {
    let create_event = world_standard::events::entity_created();
    let create_operation = match EffectOp::new(
        world_standard::ids::create_entity(),
        [EffectArgBinding::role(
            world_standard::ids::entity_param(),
            world_standard::ids::entity_role(),
        )],
        [create_event.clone()],
    ) {
        Ok(operation) => operation,
        Err(error) => panic!("standard create operation should be valid: {error}"),
    };
    let Ok(create_program) = EffectProgramDef::new(
        definition(11),
        definition_name("standard_create_program"),
        [create_operation],
        EventContract::new([create_event.clone()]),
        ReplayLevel::EventRebuild,
        version(1),
    ) else {
        panic!("standard create program should be valid");
    };
    let Ok(create_action) = ActionDef::new(
        definition(21),
        definition_name("standard_create"),
        [role("entity")],
        [],
        [],
        create_program.id(),
        EventContract::new([create_event]),
        [
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    ) else {
        panic!("standard create action should be valid");
    };

    let place_event = world_standard::events::entity_placed();
    let operation = match EffectOp::new(
        world_standard::ids::place_entity(),
        [
            EffectArgBinding::role(
                world_standard::ids::item_param(),
                world_standard::ids::item_role(),
            ),
            EffectArgBinding::role(
                world_standard::ids::destination_param(),
                world_standard::ids::destination_role(),
            ),
        ],
        [place_event.clone()],
    ) {
        Ok(operation) => operation,
        Err(error) => panic!("standard operation should be valid: {error}"),
    };
    let Ok(program) = EffectProgramDef::new(
        definition(10),
        definition_name("standard_place_program"),
        [operation],
        EventContract::new([place_event.clone()]),
        ReplayLevel::EventRebuild,
        version(1),
    ) else {
        panic!("standard place program should be valid");
    };
    let Ok(action) = ActionDef::new(
        definition(20),
        definition_name("standard_place"),
        [role("actor"), role("item"), role("destination")],
        [],
        [],
        program.id(),
        EventContract::new([place_event]),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    )
    .and_then(|action| action.with_actor_role(world_standard::ids::actor_role())) else {
        panic!("standard place action should be valid");
    };

    let reservation_event = world_standard::events::reservation_acquired();
    let reservation_operation = match EffectOp::new(
        world_standard::ids::acquire_reservation(),
        [
            EffectArgBinding::role(
                world_standard::ids::item_param(),
                world_standard::ids::item_role(),
            ),
            EffectArgBinding::role(
                world_standard::ids::holder_param(),
                world_standard::ids::actor_role(),
            ),
        ],
        [reservation_event.clone()],
    ) {
        Ok(operation) => operation,
        Err(error) => panic!("standard reservation operation should be valid: {error}"),
    };
    let Ok(reservation_program) = EffectProgramDef::new(
        definition(12),
        definition_name("standard_reservation_program"),
        [reservation_operation],
        EventContract::new([reservation_event.clone()]),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("standard reservation program should be valid");
    };
    let Ok(reservation_action) = ActionDef::new(
        definition(22),
        definition_name("standard_reserve"),
        [role("actor"), role("item")],
        [],
        [],
        reservation_program.id(),
        EventContract::new([reservation_event]),
        [
            StagePermission::AcquireReservation,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    )
    .and_then(|action| action.with_actor_role(world_standard::ids::actor_role())) else {
        panic!("standard reservation action should be valid");
    };

    let mut builder = DefinitionRegistryBuilder::new();
    if let Err(error) = builder.install(&world_standard::StandardWorldDefinitions) {
        panic!("standard definitions should install: {error}");
    }
    if let Err(error) = builder.add_effect_program(create_program) {
        panic!("standard create program should install: {error}");
    }
    if let Err(error) = builder.add_effect_program(program) {
        panic!("standard place program should install: {error}");
    }
    if let Err(error) = builder.add_effect_program(reservation_program) {
        panic!("standard reservation program should install: {error}");
    }
    if let Err(error) = builder.add_action(create_action) {
        panic!("standard create action should install: {error}");
    }
    if let Err(error) = builder.add_action(action) {
        panic!("standard place action should install: {error}");
    }
    if let Err(error) = builder.add_action(reservation_action) {
        panic!("standard reservation action should install: {error}");
    }
    let Ok(registry) = builder.build() else {
        panic!("standard place registry should build");
    };
    registry
}

fn create_request(entity: EntityId) -> RuntimeRequest {
    RuntimeRequest::new(
        RequestSource::Tooling,
        None,
        definition(21),
        SimulationTime::from_ticks(1),
        [SubmittedRole::new(
            world_standard::ids::entity_role(),
            entity,
        )],
        None,
    )
}

fn place_request() -> RuntimeRequest {
    RuntimeRequest::new(
        RequestSource::Player,
        Some(entity(1)),
        definition(20),
        SimulationTime::from_ticks(10),
        [
            SubmittedRole::new(world_standard::ids::actor_role(), entity(1)),
            SubmittedRole::new(world_standard::ids::item_role(), entity(2)),
            SubmittedRole::new(world_standard::ids::destination_role(), entity(3)),
        ],
        None,
    )
}

fn reservation_request() -> RuntimeRequest {
    RuntimeRequest::new(
        RequestSource::Player,
        Some(entity(1)),
        definition(22),
        SimulationTime::from_ticks(20),
        [
            SubmittedRole::new(world_standard::ids::actor_role(), entity(1)),
            SubmittedRole::new(world_standard::ids::item_role(), entity(2)),
        ],
        None,
    )
}

fn assert_event_roles(event: &world_model::EventRecord, expected: &[(RoleName, EntityId)]) {
    let actual = event
        .roles()
        .iter()
        .map(|binding| (binding.role().clone(), binding.entity()))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

fn transaction_ids() -> CausalTransactionIdIssuer {
    match CausalTransactionIdIssuer::starting_at(1) {
        Ok(issuer) => issuer,
        Err(error) => panic!("transaction id issuer should be valid: {error}"),
    }
}

fn event_ids() -> EventRecordIdIssuer {
    match EventRecordIdIssuer::starting_at(1) {
        Ok(issuer) => issuer,
        Err(error) => panic!("event id issuer should be valid: {error}"),
    }
}

fn entity(value: u64) -> EntityId {
    let Some(id) = EntityId::new(value) else {
        panic!("test entity ids should be nonzero");
    };
    id
}

fn definition(value: u64) -> DefinitionId {
    let Some(id) = DefinitionId::new(value) else {
        panic!("test definition ids should be nonzero");
    };
    id
}

fn version(value: u64) -> VersionAnchor {
    let Some(version) = VersionAnchor::new(value) else {
        panic!("test versions should be nonzero");
    };
    version
}

fn definition_name(value: &'static str) -> DefinitionName {
    let Some(name) = DefinitionName::new(value) else {
        panic!("test definition names should be non-empty");
    };
    name
}

fn role(value: &'static str) -> RoleDef {
    RoleDef::new(role_name(value), role_type("entity"))
}

fn role_name(value: &'static str) -> RoleName {
    let Some(name) = RoleName::new(value) else {
        panic!("test role names should be non-empty");
    };
    name
}

fn role_type(value: &'static str) -> RoleType {
    let Some(name) = RoleType::new(value) else {
        panic!("test role types should be non-empty");
    };
    name
}
