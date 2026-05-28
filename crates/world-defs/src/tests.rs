use super::*;
use world_core::{DefinitionId, ReplayLevel, VersionAnchor};

fn id(value: u64) -> DefinitionId {
    let Some(id) = DefinitionId::new(value) else {
        panic!("test ids must be nonzero");
    };
    id
}

fn version(value: u64) -> VersionAnchor {
    let Some(version) = VersionAnchor::new(value) else {
        panic!("test versions must be nonzero");
    };
    version
}

macro_rules! test_key {
    ($fn_name:ident, $type_name:ident, $message:literal) => {
        fn $fn_name(value: &'static str) -> $type_name {
            let Some(value) = $type_name::new(value) else {
                panic!($message);
            };
            value
        }
    };
}

test_key!(
    definition_name,
    DefinitionName,
    "test definition names must be non-empty"
);
test_key!(role_name, RoleName, "test role names must be non-empty");
test_key!(role_type, RoleType, "test role types must be non-empty");
test_key!(
    effect_param,
    EffectParamName,
    "test effect params must be non-empty"
);
test_key!(event_kind, EventKind, "test event kinds must be non-empty");
test_key!(
    requirement_kind,
    RequirementKind,
    "test requirement kinds must be non-empty"
);
test_key!(
    binding_rule_kind,
    BindingRuleKind,
    "test binding rule kinds must be non-empty"
);
test_key!(policy_key, PolicyKey, "test policy keys must be non-empty");
test_key!(
    state_field_name,
    StateFieldName,
    "test state field names must be non-empty"
);
test_key!(
    state_value_type,
    StateValueType,
    "test state value types must be non-empty"
);

fn role(name: &'static str) -> RoleDef {
    RoleDef::new(role_name(name), role_type("entity"))
}

fn event(kind: &'static str, roles: impl IntoIterator<Item = &'static str>) -> EventRecordSpec {
    event_with_version(kind, roles, 1)
}

fn event_with_version(
    kind: &'static str,
    roles: impl IntoIterator<Item = &'static str>,
    version_value: u64,
) -> EventRecordSpec {
    let Ok(event) = EventRecordSpec::new(
        event_kind(kind),
        roles.into_iter().map(role_name),
        version(version_value),
    ) else {
        panic!("test events must declare roles");
    };
    event
}

#[test]
fn event_record_spec_display_includes_version_and_role_shape() {
    assert_eq!(
        event_with_version("EntityPlaced", ["item", "actor", "destination"], 7).to_string(),
        "EntityPlaced@7(actor,destination,item)"
    );
}

fn state_schema() -> ProcessStateSchema {
    let Ok(schema) = ProcessStateSchema::new([ProcessStateField::new(
        state_field_name("progress"),
        state_value_type("u32"),
    )]) else {
        panic!("test process state schemas must have fields");
    };
    schema
}

fn empty_state_schema() -> ProcessStateSchema {
    let Ok(schema) = ProcessStateSchema::new([]) else {
        panic!("empty process state schemas are valid");
    };
    schema
}

fn policies() -> ProcessPolicies {
    ProcessPolicies::new(
        policy_key("tick"),
        policy_key("wait"),
        policy_key("interrupt"),
        policy_key("resume"),
        policy_key("failure"),
    )
}

fn support(tier: ResolutionTier, effect_program: DefinitionId) -> ResolutionSupport {
    let name = match tier {
        ResolutionTier::Concrete => "concrete_lowering",
        ResolutionTier::Abstract => "abstract_lowering",
        ResolutionTier::Strategic => "strategic_lowering",
    };
    let Ok(support) = ResolutionSupport::new(tier, policy_key(name), [effect_program]) else {
        panic!("test resolution support must reference effect programs");
    };
    support
}

fn primitive_id(value: u64) -> EffectPrimitiveId {
    EffectPrimitiveId::new(id(value))
}

fn arg(param: &'static str, role: &'static str) -> EffectArgBinding {
    EffectArgBinding::role(effect_param(param), role_name(role))
}

fn primitive(
    value: u64,
    name: &'static str,
    params: impl IntoIterator<Item = EffectParamDef>,
    permissions: impl IntoIterator<Item = StagePermission>,
    event_contract: EventContract,
) -> EffectPrimitiveDef {
    primitive_with_replay(
        value,
        name,
        params,
        permissions,
        event_contract,
        ReplayLevel::EventRebuild,
    )
}

fn primitive_with_replay(
    value: u64,
    name: &'static str,
    params: impl IntoIterator<Item = EffectParamDef>,
    permissions: impl IntoIterator<Item = StagePermission>,
    event_contract: EventContract,
    replay_level: ReplayLevel,
) -> EffectPrimitiveDef {
    let Ok(primitive) = EffectPrimitiveDef::new(
        primitive_id(value),
        definition_name(name),
        params,
        permissions,
        event_contract,
        replay_level,
        version(1),
    ) else {
        panic!("test primitive should be valid");
    };
    primitive
}

fn place_event() -> EventRecordSpec {
    event("EntityPlaced", ["actor", "item", "destination"])
}

fn place_primitive() -> EffectPrimitiveDef {
    primitive(
        101,
        "place_entity",
        [
            EffectParamDef::new(effect_param("item"), EffectParamKind::EntityRole),
            EffectParamDef::new(effect_param("destination"), EffectParamKind::EntityRole),
        ],
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        EventContract::new([place_event()]),
    )
}

fn schedule_primitive() -> EffectPrimitiveDef {
    primitive_with_replay(
        102,
        "schedule_process",
        [],
        [StagePermission::ScheduleProcess],
        EventContract::default(),
        ReplayLevel::AuditOnly,
    )
}

fn observe_primitive() -> EffectPrimitiveDef {
    primitive_with_replay(
        103,
        "observe",
        [],
        [StagePermission::ReadWorld],
        EventContract::default(),
        ReplayLevel::AuditOnly,
    )
}

fn op(
    primitive: EffectPrimitiveId,
    args: impl IntoIterator<Item = EffectArgBinding>,
    emitted_events: impl IntoIterator<Item = EventRecordSpec>,
) -> EffectOp {
    let Ok(operation) = EffectOp::new(primitive, args, emitted_events) else {
        panic!("test operations should be valid");
    };
    operation
}

fn place_op() -> EffectOp {
    op(
        place_primitive().id(),
        [arg("item", "item"), arg("destination", "destination")],
        [place_event()],
    )
}

fn program_with_ops(
    value: u64,
    operations: impl IntoIterator<Item = EffectOp>,
    event_contract: EventContract,
) -> EffectProgramDef {
    let Ok(program) = EffectProgramDef::new(
        id(value),
        definition_name("test_program"),
        operations,
        event_contract,
        ReplayLevel::EventRebuild,
        version(1),
    ) else {
        panic!("test program should be valid");
    };
    program
}

fn place_program() -> EffectProgramDef {
    let Ok(program) = EffectProgramDef::new(
        id(1),
        definition_name("place"),
        [place_op()],
        EventContract::new([place_event()]),
        ReplayLevel::EventRebuild,
        version(1),
    ) else {
        panic!("test program should be valid");
    };
    program
}

fn action(effect_program: DefinitionId) -> ActionDef {
    let Ok(action) = ActionDef::new(
        id(2),
        definition_name("move_item"),
        [role("actor"), role("item"), role("destination")],
        [RequirementDef::new(
            requirement_kind("reachable"),
            [role_name("actor"), role_name("item")],
        )],
        [BindingRuleDef::new(
            binding_rule_kind("holds"),
            [role_name("actor"), role_name("item")],
        )],
        effect_program,
        EventContract::new([place_event()]),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(2),
    ) else {
        panic!("test action should be valid");
    };
    action
}

fn process(effect_program: DefinitionId) -> ProcessDef {
    let Ok(process) = ProcessDef::new(
        id(3),
        definition_name("haul_supplies"),
        [role("actor"), role("item"), role("destination")],
        state_schema(),
        [
            support(ResolutionTier::Concrete, effect_program),
            support(ResolutionTier::Abstract, effect_program),
        ],
        policies(),
        EventContract::new([place_event()]),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(3),
    ) else {
        panic!("test process should be valid");
    };
    process
}

#[test]
fn string_keys_reject_blank_values() {
    assert_eq!(
        RoleName::new(" actor ").map(|name| name.to_string()),
        Some("actor".to_owned())
    );
    let Ok(role) = RoleName::try_from(" actor ") else {
        panic!("trimmed key should be valid");
    };
    assert_eq!(role.as_ref(), "actor");
    assert_eq!(
        DefinitionName::try_from(" "),
        Err(DefinitionError::EmptyItemField {
            type_name: "DefinitionName",
            field: "value",
        })
    );
    assert_eq!(DefinitionName::new(" "), None);
}

#[test]
fn action_defs_reject_duplicate_roles_and_unknown_role_refs() {
    let Err(error) = ActionDef::new(
        id(10),
        definition_name("bad_roles"),
        [role("actor"), role("actor")],
        [],
        [],
        id(1),
        EventContract::default(),
        [StagePermission::ReadWorld],
        version(1),
    ) else {
        panic!("duplicate roles must be rejected");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateRole {
            definition: id(10),
            role: role_name("actor"),
        }
    );

    let Err(error) = ActionDef::new(
        id(11),
        definition_name("bad_requirement"),
        [role("actor")],
        [RequirementDef::new(
            requirement_kind("owns"),
            [role_name("item")],
        )],
        [],
        id(1),
        EventContract::default(),
        [StagePermission::ReadWorld],
        version(1),
    ) else {
        panic!("unknown role references must be rejected");
    };
    assert_eq!(
        error,
        DefinitionError::UnknownRole {
            definition: id(11),
            role: role_name("item"),
        }
    );
}

#[test]
fn primitive_definitions_and_effect_programs_validate_local_contracts() {
    let program = place_program();

    assert!(program.emitted_events().contains(&place_event()));

    let Err(error) = EffectPrimitiveDef::new(
        primitive_id(12),
        definition_name("duplicate_param"),
        [
            EffectParamDef::new(effect_param("entity"), EffectParamKind::EntityRole),
            EffectParamDef::new(effect_param("entity"), EffectParamKind::EntityRole),
        ],
        [StagePermission::ReadWorld],
        EventContract::default(),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("primitive params must be unique");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateEffectParam {
            primitive: primitive_id(12),
            param: effect_param("entity"),
        }
    );

    let Err(error) = EffectPrimitiveDef::new(
        primitive_id(13),
        definition_name("mutate_without_event"),
        [],
        [StagePermission::MutatePhysical],
        EventContract::default(),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("mutating primitives must declare event evidence");
    };
    assert_eq!(
        error,
        DefinitionError::PrimitiveRequiresEvent {
            primitive: primitive_id(13),
        }
    );

    let Err(error) = EffectPrimitiveDef::new(
        primitive_id(14),
        definition_name("read_with_event"),
        [],
        [StagePermission::ReadWorld],
        EventContract::new([event("ReadOnlyEvent", ["actor"])]),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("event-contract primitives must declare event permission");
    };
    assert_eq!(
        error,
        DefinitionError::PrimitiveEventPermissionNotDeclared {
            primitive: primitive_id(14),
        }
    );

    let Err(error) = EffectOp::new(
        place_primitive().id(),
        [arg("item", "item"), arg("item", "other")],
        [place_event()],
    ) else {
        panic!("operation args must be unique");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateEffectArg {
            primitive: place_primitive().id(),
            param: effect_param("item"),
        }
    );

    let Err(error) = EffectOp::new(
        place_primitive().id(),
        [arg("item", "item"), arg("destination", "destination")],
        [place_event(), place_event()],
    ) else {
        panic!("operation events must be unique");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateEffectEvent {
            primitive: place_primitive().id(),
            event: place_event(),
        }
    );

    let secondary_event = event("SecondaryEvent", ["actor"]);
    let Ok(ordered_events) = EffectOp::new(
        place_primitive().id(),
        [arg("item", "item"), arg("destination", "destination")],
        [secondary_event.clone(), place_event()],
    ) else {
        panic!("operation with distinct events should be valid");
    };
    assert_eq!(
        ordered_events.emitted_events().cloned().collect::<Vec<_>>(),
        vec![secondary_event, place_event()]
    );

    let Ok(control_operation) = EffectOp::new(
        schedule_primitive().id(),
        [],
        std::iter::empty::<EventRecordSpec>(),
    ) else {
        panic!("runtime-control primitive calls can be eventless");
    };
    assert!(control_operation.emits_no_events());

    let Err(error) = EffectProgramDef::new(
        id(15),
        definition_name("bad_event_contract"),
        [op(
            observe_primitive().id(),
            [],
            std::iter::empty::<EventRecordSpec>(),
        )],
        EventContract::new([event("Committed", ["actor"])]),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("effect programs must be able to emit required events");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventNotEmitted {
            definition: id(15),
            event: event("Committed", ["actor"]),
        }
    );

    let Err(error) = EffectProgramDef::new(
        id(16),
        definition_name("uncontracted_event"),
        [op(
            place_primitive().id(),
            [arg("item", "item"), arg("destination", "destination")],
            [event("Uncontracted", ["actor"])],
        )],
        EventContract::default(),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("event-emitting operations must be covered by the program contract");
    };
    assert_eq!(
        error,
        DefinitionError::EventNotPermittedByContract {
            definition: id(16),
            event: event("Uncontracted", ["actor"]),
        }
    );

    let Ok(optional_event_program) = EffectProgramDef::new(
        id(17),
        definition_name("optional_event"),
        [op(
            place_primitive().id(),
            [arg("item", "item"), arg("destination", "destination")],
            [event("SmokeEmitted", ["actor"])],
        )],
        EventContract::with_allowed([], [event("SmokeEmitted", ["actor"])]),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("allowed events should be permitted without becoming required");
    };
    assert!(
        optional_event_program
            .event_contract()
            .allowed_events()
            .any(|event| event.kind().as_str() == "SmokeEmitted")
    );

    let Err(error) = EffectProgramDef::new(
        id(18),
        definition_name("wrong_event_shape"),
        [op(
            place_primitive().id(),
            [arg("item", "item"), arg("destination", "destination")],
            [place_event()],
        )],
        EventContract::new([event("EntityPlaced", ["actor", "item"])]),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("event contracts must match role sets, not only event kind");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventNotEmitted {
            definition: id(18),
            event: event("EntityPlaced", ["actor", "item"]),
        }
    );

    let Err(error) = EffectProgramDef::new(
        id(19),
        definition_name("wrong_event_version"),
        [op(
            place_primitive().id(),
            [arg("item", "item"), arg("destination", "destination")],
            [event_with_version("EntityPlaced", ["actor"], 2)],
        )],
        EventContract::with_allowed([], [event_with_version("EntityPlaced", ["actor"], 1)]),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("event contracts must match version anchors, not only event kind");
    };
    assert_eq!(
        error,
        DefinitionError::EventNotPermittedByContract {
            definition: id(19),
            event: event_with_version("EntityPlaced", ["actor"], 2),
        }
    );
}

#[test]
fn registry_validates_action_effect_reference_permissions_and_events() {
    let program = place_program();

    let Err(error) = DefinitionRegistry::new([], [], [action(id(99))], [], []) else {
        panic!("missing effect references must be rejected");
    };
    assert_eq!(
        error,
        DefinitionError::MissingEffectProgram {
            definition: id(2),
            effect_program: id(99),
        }
    );

    let Ok(under_permitted) = ActionDef::new(
        id(4),
        definition_name("under_permitted"),
        [role("actor"), role("item"), role("destination")],
        [],
        [],
        id(1),
        EventContract::default(),
        [StagePermission::ReadWorld, StagePermission::MutatePhysical],
        version(1),
    ) else {
        panic!("local action shape should be valid");
    };
    let Err(error) = DefinitionRegistry::new(
        [place_primitive()],
        [program.clone()],
        [under_permitted],
        [],
        [],
    ) else {
        panic!("registry must reject missing stage permissions");
    };
    assert_eq!(
        error,
        DefinitionError::PermissionNotDeclared {
            definition: id(4),
            effect_program: id(1),
            permission: StagePermission::EmitPhysicalEventRecord,
        }
    );

    let Ok(weak_contract) = ActionDef::new(
        id(8),
        definition_name("weak_contract"),
        [role("actor"), role("item"), role("destination")],
        [],
        [],
        id(1),
        EventContract::default(),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    ) else {
        panic!("local action shape should be valid");
    };
    let Err(error) = DefinitionRegistry::new(
        [place_primitive()],
        [program.clone()],
        [weak_contract],
        [],
        [],
    ) else {
        panic!("registry must reject action contracts weaker than the effect program");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventNotDeclared {
            definition: id(8),
            event: event("EntityPlaced", ["actor", "item", "destination"]),
        }
    );

    let Ok(bad_event) = ActionDef::new(
        id(5),
        definition_name("bad_event"),
        [role("actor"), role("item"), role("destination")],
        [],
        [],
        id(1),
        EventContract::new([
            event("EntityPlaced", ["actor", "item", "destination"]),
            event("OtherEvent", ["actor"]),
        ]),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    ) else {
        panic!("local action shape should be valid");
    };
    let Err(error) =
        DefinitionRegistry::new([place_primitive()], [program.clone()], [bad_event], [], [])
    else {
        panic!("registry must reject unavailable required events");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventUnavailable {
            definition: id(5),
            event: event("OtherEvent", ["actor"]),
        }
    );

    let Ok(registry) = DefinitionRegistry::new(
        [place_primitive()],
        [program.clone()],
        [action(program.id())],
        [process(program.id())],
        [],
    ) else {
        panic!("registry should accept matching definitions");
    };
    assert!(registry.effect_program(program.id()).is_some());
    assert!(registry.action(id(2)).is_some());
    assert!(registry.process(id(3)).is_some());
}

#[test]
fn registry_validates_primitive_references_args_and_operation_events() {
    let program = place_program();
    let Err(error) = DefinitionRegistry::new([], [program.clone()], [], [], []) else {
        panic!("registry must reject missing primitive references");
    };
    assert_eq!(
        error,
        DefinitionError::MissingEffectPrimitive {
            definition: id(1),
            effect_program: id(1),
            primitive: place_primitive().id(),
        }
    );

    let missing_arg = program_with_ops(
        50,
        [op(
            place_primitive().id(),
            [arg("item", "item")],
            [place_event()],
        )],
        EventContract::new([place_event()]),
    );
    let Err(error) = DefinitionRegistry::new(
        [place_primitive()],
        [missing_arg.clone()],
        [action(missing_arg.id())],
        [],
        [],
    ) else {
        panic!("registry must reject missing required primitive args");
    };
    assert_eq!(
        error,
        DefinitionError::MissingEffectArg {
            definition: id(50),
            effect_program: id(50),
            primitive: place_primitive().id(),
            param: effect_param("destination"),
        }
    );

    let unknown_arg = program_with_ops(
        51,
        [op(
            place_primitive().id(),
            [
                arg("item", "item"),
                arg("destination", "destination"),
                arg("extra", "actor"),
            ],
            [place_event()],
        )],
        EventContract::new([place_event()]),
    );
    let Err(error) = DefinitionRegistry::new(
        [place_primitive()],
        [unknown_arg.clone()],
        [action(unknown_arg.id())],
        [],
        [],
    ) else {
        panic!("registry must reject unknown primitive args");
    };
    assert_eq!(
        error,
        DefinitionError::UnknownEffectArg {
            definition: id(51),
            effect_program: id(51),
            primitive: place_primitive().id(),
            param: effect_param("extra"),
        }
    );

    let unknown_role = program_with_ops(
        52,
        [op(
            place_primitive().id(),
            [arg("item", "missing"), arg("destination", "destination")],
            [place_event()],
        )],
        EventContract::new([place_event()]),
    );
    let Err(error) = DefinitionRegistry::new(
        [place_primitive()],
        [unknown_role.clone()],
        [action(unknown_role.id())],
        [],
        [],
    ) else {
        panic!("registry must reject args that reference undeclared roles");
    };
    assert_eq!(
        error,
        DefinitionError::UnknownRole {
            definition: id(2),
            role: role_name("missing"),
        }
    );

    let missing_event = program_with_ops(
        53,
        [op(
            place_primitive().id(),
            [arg("item", "item"), arg("destination", "destination")],
            [],
        )],
        EventContract::default(),
    );
    let Err(error) = DefinitionRegistry::new(
        [place_primitive()],
        [missing_event.clone()],
        [action(missing_event.id())],
        [],
        [],
    ) else {
        panic!("registry must reject omitted primitive-required events");
    };
    assert_eq!(
        error,
        DefinitionError::PrimitiveRequiredEventNotEmitted {
            definition: id(53),
            effect_program: id(53),
            primitive: place_primitive().id(),
            event: place_event(),
        }
    );

    let undeclared_required_event = program_with_ops(
        55,
        [op(
            place_primitive().id(),
            [arg("item", "item"), arg("destination", "destination")],
            [place_event()],
        )],
        EventContract::with_allowed([], [place_event()]),
    );
    let Err(error) = DefinitionRegistry::new(
        [place_primitive()],
        [undeclared_required_event.clone()],
        [action(undeclared_required_event.id())],
        [],
        [],
    ) else {
        panic!("registry must reject primitive-required events not required by the program");
    };
    assert_eq!(
        error,
        DefinitionError::PrimitiveRequiredEventNotDeclared {
            effect_program: id(55),
            primitive: place_primitive().id(),
            event: place_event(),
        }
    );

    let Ok(weak_replay) = EffectProgramDef::new(
        id(56),
        definition_name("weak_replay"),
        [op(
            place_primitive().id(),
            [arg("item", "item"), arg("destination", "destination")],
            [place_event()],
        )],
        EventContract::new([place_event()]),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("weak replay program should be locally valid");
    };
    let Err(error) = DefinitionRegistry::new(
        [place_primitive()],
        [weak_replay.clone()],
        [action(weak_replay.id())],
        [],
        [],
    ) else {
        panic!("registry must reject effect programs below primitive replay level");
    };
    assert_eq!(
        error,
        DefinitionError::EffectProgramReplayTooWeak {
            effect_program: id(56),
            primitive: place_primitive().id(),
            program_replay: ReplayLevel::AuditOnly,
            primitive_replay: ReplayLevel::EventRebuild,
        }
    );

    let other_event = event("OtherEvent", ["actor"]);
    let unpermitted_event = program_with_ops(
        54,
        [op(
            place_primitive().id(),
            [arg("item", "item"), arg("destination", "destination")],
            [place_event(), other_event.clone()],
        )],
        EventContract::with_allowed([place_event()], [other_event.clone()]),
    );
    let Err(error) = DefinitionRegistry::new(
        [place_primitive()],
        [unpermitted_event.clone()],
        [action(unpermitted_event.id())],
        [],
        [],
    ) else {
        panic!("registry must reject events outside the primitive contract");
    };
    assert_eq!(
        error,
        DefinitionError::OperationEventNotPermittedByPrimitive {
            definition: id(54),
            effect_program: id(54),
            primitive: place_primitive().id(),
            event: other_event,
        }
    );
}

#[test]
fn registry_builder_uses_the_same_cross_definition_validation_path() {
    let program = place_program();
    let action_def = action(program.id());
    let process = process(program.id());

    let Ok(direct) = DefinitionRegistry::new(
        [place_primitive()],
        [program.clone()],
        [action_def.clone()],
        [process.clone()],
        [],
    ) else {
        panic!("direct registry constructor should accept matching definitions");
    };

    let mut builder = DefinitionRegistryBuilder::new();
    if let Err(error) = builder.add_primitive(place_primitive()) {
        panic!("builder should accept primitive: {error}");
    }
    if let Err(error) = builder.add_effect_program(program.clone()) {
        panic!("builder should accept effect program: {error}");
    }
    if let Err(error) = builder.add_action(action_def.clone()) {
        panic!("builder should accept action: {error}");
    }
    if let Err(error) = builder.add_process(process.clone()) {
        panic!("builder should accept process: {error}");
    }
    let Ok(built) = builder.build() else {
        panic!("builder should build the same valid registry");
    };

    assert_eq!(built, direct);

    let mut missing_reference = DefinitionRegistryBuilder::new();
    if let Err(error) = missing_reference.add_action(action(id(99))) {
        panic!("builder should defer cross-reference validation to build: {error}");
    }
    let Err(error) = missing_reference.build() else {
        panic!("builder must reject missing effect references at build");
    };
    assert_eq!(
        error,
        DefinitionError::MissingEffectProgram {
            definition: id(2),
            effect_program: id(99),
        }
    );

    let mut duplicate = DefinitionRegistryBuilder::new();
    if let Err(error) = duplicate.add_effect_program(program.clone()) {
        panic!("builder should accept first definition: {error}");
    }
    let Err(error) = duplicate.add_effect_program(program.clone()) else {
        panic!("builder must reject duplicate ids before build");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateDefinitionId { id: program.id() }
    );
}

#[test]
fn registry_validates_process_effect_references_permissions_and_events() {
    let program = place_program();

    let Err(error) = DefinitionRegistry::new([], [], [], [process(id(99))], []) else {
        panic!("missing process effect references must be rejected");
    };
    assert_eq!(
        error,
        DefinitionError::MissingEffectProgram {
            definition: id(3),
            effect_program: id(99),
        }
    );

    let Ok(under_permitted) = ProcessDef::new(
        id(6),
        definition_name("under_permitted_process"),
        [role("actor"), role("item"), role("destination")],
        state_schema(),
        [support(ResolutionTier::Concrete, id(1))],
        policies(),
        EventContract::default(),
        [StagePermission::ReadWorld, StagePermission::MutatePhysical],
        version(1),
    ) else {
        panic!("local process shape should be valid");
    };
    let Err(error) = DefinitionRegistry::new(
        [place_primitive()],
        [program.clone()],
        [],
        [under_permitted],
        [],
    ) else {
        panic!("registry must reject missing process stage permissions");
    };
    assert_eq!(
        error,
        DefinitionError::PermissionNotDeclared {
            definition: id(6),
            effect_program: id(1),
            permission: StagePermission::EmitPhysicalEventRecord,
        }
    );

    let Ok(weak_contract) = ProcessDef::new(
        id(9),
        definition_name("weak_process_contract"),
        [role("actor"), role("item"), role("destination")],
        state_schema(),
        [support(ResolutionTier::Concrete, id(1))],
        policies(),
        EventContract::default(),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    ) else {
        panic!("local process shape should be valid");
    };
    let Err(error) = DefinitionRegistry::new(
        [place_primitive()],
        [program.clone()],
        [],
        [weak_contract],
        [],
    ) else {
        panic!("registry must reject process contracts weaker than the effect program");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventNotDeclared {
            definition: id(9),
            event: event("EntityPlaced", ["actor", "item", "destination"]),
        }
    );

    let Ok(bad_event) = ProcessDef::new(
        id(7),
        definition_name("bad_process_event"),
        [role("actor"), role("item"), role("destination")],
        state_schema(),
        [support(ResolutionTier::Concrete, id(1))],
        policies(),
        EventContract::new([
            event("EntityPlaced", ["actor", "item", "destination"]),
            event("OtherEvent", ["actor"]),
        ]),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    ) else {
        panic!("local process shape should be valid");
    };
    let Err(error) =
        DefinitionRegistry::new([place_primitive()], [program.clone()], [], [bad_event], [])
    else {
        panic!("registry must reject unavailable process event contracts");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventUnavailable {
            definition: id(7),
            event: event("OtherEvent", ["actor"]),
        }
    );

    let Ok(read_only_program) = EffectProgramDef::new(
        id(40),
        definition_name("abstract_observe"),
        [op(
            observe_primitive().id(),
            [],
            std::iter::empty::<EventRecordSpec>(),
        )],
        EventContract::default(),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("read-only program should be locally valid");
    };

    let Ok(resolution_mismatch) = ProcessDef::new(
        id(41),
        definition_name("resolution_mismatch"),
        [role("actor"), role("item"), role("destination")],
        state_schema(),
        [
            support(ResolutionTier::Concrete, id(1)),
            support(ResolutionTier::Abstract, id(40)),
        ],
        policies(),
        EventContract::new([event("EntityPlaced", ["actor", "item", "destination"])]),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    ) else {
        panic!("local process shape should be valid");
    };
    let Err(error) = DefinitionRegistry::new(
        [place_primitive(), observe_primitive()],
        [program, read_only_program],
        [],
        [resolution_mismatch],
        [],
    ) else {
        panic!("registry must validate process events per supported resolution");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventUnavailable {
            definition: id(41),
            event: event("EntityPlaced", ["actor", "item", "destination"]),
        }
    );
}

#[test]
fn process_defs_require_supported_resolution_and_preserve_lookup() {
    assert!(empty_state_schema().fields().is_empty());

    let Err(error) = ProcessStateSchema::new([
        ProcessStateField::new(state_field_name("progress"), state_value_type("u32")),
        ProcessStateField::new(state_field_name("progress"), state_value_type("u64")),
    ]) else {
        panic!("process state schemas must reject duplicate fields");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateStateField {
            field: state_field_name("progress"),
        }
    );

    let Err(error) = ProcessDef::new(
        id(20),
        definition_name("bad_process"),
        [role("actor")],
        state_schema(),
        [],
        policies(),
        EventContract::default(),
        [StagePermission::ReadWorld],
        version(1),
    ) else {
        panic!("process definitions must declare supported resolutions");
    };
    assert_eq!(
        error,
        DefinitionError::EmptyDefinitionField {
            definition: id(20),
            type_name: "ProcessDef",
            field: "resolution_support",
        }
    );

    let Err(error) = ProcessDef::new(
        id(21),
        definition_name("duplicate_resolution"),
        [role("actor")],
        state_schema(),
        [
            support(ResolutionTier::Concrete, id(1)),
            support(ResolutionTier::Concrete, id(1)),
        ],
        policies(),
        EventContract::default(),
        [StagePermission::ReadWorld],
        version(1),
    ) else {
        panic!("process definitions must reject duplicate resolution support");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateResolutionSupport {
            definition: id(21),
            resolution: ResolutionTier::Concrete,
        }
    );

    let process = process(id(1));
    assert!(process.supports_resolution(ResolutionTier::Concrete));
    assert!(process.supports_resolution(ResolutionTier::Abstract));
    assert!(!process.supports_resolution(ResolutionTier::Strategic));
    assert_eq!(
        process
            .resolution_policy(ResolutionTier::Abstract)
            .map(PolicyKey::as_str),
        Some("abstract_lowering")
    );
}

#[test]
fn semantic_declarations_validate_kind_owned_outputs() {
    let Err(error) = SemanticDeclarationDef::new(
        id(30),
        definition_name("bad_intent_output"),
        SemanticDeclarationKind::IntentTemplate,
        [
            SemanticInputKind::Pressure,
            SemanticInputKind::ActionRepertoire,
        ],
        [SemanticOutputKind::SocialUpdateProposal],
        version(1),
    ) else {
        panic!("intent templates must not output social updates");
    };
    assert_eq!(
        error,
        DefinitionError::ForbiddenSemanticOutput {
            definition: id(30),
            kind: SemanticDeclarationKind::IntentTemplate,
            output: SemanticOutputKind::SocialUpdateProposal,
        }
    );

    let Err(error) = SemanticDeclarationDef::new(
        id(32),
        definition_name("bad_social_output"),
        SemanticDeclarationKind::SocialRule,
        [
            SemanticInputKind::HardEventEvidence,
            SemanticInputKind::SocialContext,
        ],
        [SemanticOutputKind::Pressure],
        version(1),
    ) else {
        panic!("social rules must not output appraisal pressure directly");
    };
    assert_eq!(
        error,
        DefinitionError::ForbiddenSemanticOutput {
            definition: id(32),
            kind: SemanticDeclarationKind::SocialRule,
            output: SemanticOutputKind::Pressure,
        }
    );

    let Ok(declaration) = SemanticDeclarationDef::new(
        id(31),
        definition_name("intent_candidate"),
        SemanticDeclarationKind::IntentTemplate,
        [
            SemanticInputKind::Pressure,
            SemanticInputKind::CapabilitySet,
            SemanticInputKind::ActionRepertoire,
            SemanticInputKind::PerceivedAffordance,
        ],
        [
            SemanticOutputKind::CandidateIntent,
            SemanticOutputKind::IntentScoreFeature,
            SemanticOutputKind::LoweringContract,
            SemanticOutputKind::ActivityPreparation,
        ],
        version(1),
    ) else {
        panic!("intent templates may output intent preparation artifacts");
    };
    assert_eq!(declaration.kind(), SemanticDeclarationKind::IntentTemplate);
    assert!(
        declaration
            .outputs()
            .contains(&SemanticOutputKind::CandidateIntent)
    );

    let Ok(appraisal) = SemanticDeclarationDef::new(
        id(33),
        definition_name("fear_appraisal"),
        SemanticDeclarationKind::AppraisalRule,
        [
            SemanticInputKind::HardEventEvidence,
            SemanticInputKind::ActorContext,
        ],
        [
            SemanticOutputKind::Thought,
            SemanticOutputKind::Pressure,
            SemanticOutputKind::GoalPressure,
            SemanticOutputKind::AppraisalRecordProposal,
        ],
        version(1),
    ) else {
        panic!("appraisal rules may output appraisal artifacts");
    };
    assert!(appraisal.outputs().contains(&SemanticOutputKind::Pressure));
}

#[test]
fn registry_rejects_duplicate_definition_ids_across_families() {
    let program = place_program();
    let duplicate_program = program.clone();
    let Err(error) = DefinitionRegistry::new([], [program.clone(), duplicate_program], [], [], [])
    else {
        panic!("registry must reject ids reused within a definition family");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateDefinitionId { id: program.id() }
    );

    let semantic = {
        let Ok(declaration) = SemanticDeclarationDef::new(
            program.id(),
            definition_name("duplicate"),
            SemanticDeclarationKind::SemanticView,
            [SemanticInputKind::ActorContext],
            [SemanticOutputKind::DerivedActorContext],
            version(1),
        ) else {
            panic!("semantic declaration should be locally valid");
        };
        declaration
    };

    let Err(error) = DefinitionRegistry::new([], [program.clone()], [], [], [semantic]) else {
        panic!("registry must reject ids reused across definition families");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateDefinitionId { id: program.id() }
    );
}
