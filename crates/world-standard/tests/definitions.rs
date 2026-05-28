use world_core::ReplayLevel;
use world_defs::{
    DefinitionError, DefinitionRegistryBuilder, EffectParamKind, EffectPrimitiveDef,
    EffectPrimitiveDescriptor, EventContract, StagePermission,
};
use world_standard::StandardWorldDefinitions;

#[test]
fn standard_definition_bundle_builds_a_valid_registry() {
    let mut builder = DefinitionRegistryBuilder::new();
    if let Err(error) = builder.install(&StandardWorldDefinitions) {
        panic!("standard definitions should install: {error}");
    }
    let Ok(registry) = builder.build() else {
        panic!("standard definitions should build");
    };

    assert!(
        registry
            .effect_primitive(world_standard::ids::create_entity())
            .is_some()
    );
    assert!(
        registry
            .effect_primitive(world_standard::ids::place_entity())
            .is_some()
    );
    assert!(
        registry
            .effect_primitive(world_standard::ids::acquire_reservation())
            .is_some()
    );
}

#[test]
fn standard_primitive_descriptors_pin_schema() {
    assert_primitive(
        world_standard::primitives::physical::CreateEntity.definition(),
        world_standard::ids::create_entity(),
        "create_entity",
        &[(
            world_standard::ids::entity_param(),
            EffectParamKind::EntityRole,
        )],
        &[
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        EventContract::new([world_standard::events::entity_created()]),
        ReplayLevel::EventRebuild,
    );
    assert_primitive(
        world_standard::primitives::physical::PlaceEntity.definition(),
        world_standard::ids::place_entity(),
        "place_entity",
        &[
            (
                world_standard::ids::item_param(),
                EffectParamKind::EntityRole,
            ),
            (
                world_standard::ids::destination_param(),
                EffectParamKind::EntityRole,
            ),
        ],
        &[
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        EventContract::new([world_standard::events::entity_placed()]),
        ReplayLevel::EventRebuild,
    );
    assert_primitive(
        world_standard::primitives::reservation::AcquireReservation.definition(),
        world_standard::ids::acquire_reservation(),
        "acquire_reservation",
        &[
            (
                world_standard::ids::item_param(),
                EffectParamKind::EntityRole,
            ),
            (
                world_standard::ids::holder_param(),
                EffectParamKind::OptionalEntityRole,
            ),
        ],
        &[
            StagePermission::AcquireReservation,
            StagePermission::EmitPhysicalEventRecord,
        ],
        EventContract::new([world_standard::events::reservation_acquired()]),
        ReplayLevel::AuditOnly,
    );
    assert_primitive(
        world_standard::primitives::process::ScheduleProcess.definition(),
        world_standard::ids::schedule_process(),
        "schedule_process",
        &[],
        &[StagePermission::ScheduleProcess],
        EventContract::default(),
        ReplayLevel::AuditOnly,
    );
}

fn assert_primitive(
    definition: Result<EffectPrimitiveDef, DefinitionError>,
    id: world_defs::EffectPrimitiveId,
    name: &'static str,
    params: &[(world_defs::EffectParamName, EffectParamKind)],
    permissions: &[StagePermission],
    event_contract: EventContract,
    replay_level: ReplayLevel,
) {
    let Ok(definition) = definition else {
        panic!("standard primitive descriptor should materialize");
    };
    assert_eq!(definition.id(), id);
    assert_eq!(definition.name().as_ref(), name);
    assert_eq!(definition.params().len(), params.len());
    for (actual, (name, kind)) in definition.params().iter().zip(params) {
        assert_eq!(actual.name(), name);
        assert_eq!(actual.kind(), *kind);
    }
    assert_eq!(
        definition
            .required_permissions()
            .copied()
            .collect::<Vec<_>>(),
        permissions
    );
    assert_eq!(definition.event_contract(), &event_contract);
    assert_eq!(definition.replay_level(), replay_level);
    assert_eq!(
        definition.version(),
        world_standard::ids::primitive_version()
    );
}
