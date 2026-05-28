use super::*;

pub(super) fn entity(value: u64) -> EntityId {
    let Some(id) = EntityId::new(value) else {
        panic!("test entity ids must be nonzero");
    };
    id
}

pub(super) fn definition(value: u64) -> DefinitionId {
    let Some(id) = DefinitionId::new(value) else {
        panic!("test definition ids must be nonzero");
    };
    id
}

pub(super) fn transaction(value: u64) -> CausalTransactionId {
    let Ok(mut issuer) = CausalTransactionIdIssuer::starting_at(value) else {
        panic!("test transaction id issuer must be valid");
    };
    let Some(id) = issuer.issue() else {
        panic!("test transaction id must be available");
    };
    id
}

pub(super) fn version(value: u64) -> VersionAnchor {
    let Some(id) = VersionAnchor::new(value) else {
        panic!("test version anchors must be nonzero");
    };
    id
}

pub(super) fn provenance(value: u64) -> ProvenanceKey {
    let Some(id) = ProvenanceKey::new(value) else {
        panic!("test provenance keys must be nonzero");
    };
    id
}

pub(super) fn role_name(value: &'static str) -> RoleName {
    let Some(name) = RoleName::new(value) else {
        panic!("test role names must be non-empty");
    };
    name
}

pub(super) fn role_type(value: &'static str) -> RoleType {
    let Some(name) = RoleType::new(value) else {
        panic!("test role types must be non-empty");
    };
    name
}

pub(super) fn policy_key(value: &'static str) -> PolicyKey {
    let Some(name) = PolicyKey::new(value) else {
        panic!("test policy keys must be non-empty");
    };
    name
}

pub(super) fn state_field_name(value: &'static str) -> StateFieldName {
    let Some(name) = StateFieldName::new(value) else {
        panic!("test state field names must be non-empty");
    };
    name
}

pub(super) fn state_value_type(value: &'static str) -> StateValueType {
    let Some(name) = StateValueType::new(value) else {
        panic!("test state value types must be non-empty");
    };
    name
}

pub(super) fn definition_name(value: &'static str) -> DefinitionName {
    let Some(name) = DefinitionName::new(value) else {
        panic!("test definition names must be non-empty");
    };
    name
}

pub(super) const PRIMITIVE_CREATE_ENTITY: u64 = 101;
pub(super) const PRIMITIVE_PLACE_ENTITY: u64 = 102;
pub(super) const PRIMITIVE_ACQUIRE_RESERVATION: u64 = 103;
pub(super) const PRIMITIVE_SCHEDULE_PROCESS: u64 = 104;
pub(super) const PRIMITIVE_UNKNOWN_SEMANTICS: u64 = 199;

pub(super) fn primitive_id(value: u64) -> EffectPrimitiveId {
    EffectPrimitiveId::new(definition(value))
}

pub(super) fn param_name(value: &'static str) -> EffectParamName {
    let Some(name) = EffectParamName::new(value) else {
        panic!("test primitive params must be non-empty");
    };
    name
}

pub(super) fn primitive_name(value: &'static str) -> DefinitionName {
    definition_name(value)
}

pub(super) fn arg(param: &'static str, role: &'static str) -> EffectArgBinding {
    EffectArgBinding::role(param_name(param), role_name(role))
}

pub(super) fn event_kind(value: &'static str) -> EventKind {
    let Some(name) = EventKind::new(value) else {
        panic!("test event kinds must be non-empty");
    };
    name
}

pub(super) fn event_spec() -> EventRecordSpec {
    let Ok(spec) = EventRecordSpec::new(
        event_kind("EntityPlaced"),
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

pub(super) fn primitive_def(
    id: u64,
    name: &'static str,
    params: impl IntoIterator<Item = EffectParamDef>,
    permissions: impl IntoIterator<Item = StagePermission>,
    event_contract: EventContract,
) -> EffectPrimitiveDef {
    primitive_def_with_replay(
        id,
        name,
        params,
        permissions,
        event_contract,
        ReplayLevel::EventRebuild,
    )
}

pub(super) fn primitive_def_with_replay(
    id: u64,
    name: &'static str,
    params: impl IntoIterator<Item = EffectParamDef>,
    permissions: impl IntoIterator<Item = StagePermission>,
    event_contract: EventContract,
    replay_level: ReplayLevel,
) -> EffectPrimitiveDef {
    let Ok(definition) = EffectPrimitiveDef::new(
        primitive_id(id),
        primitive_name(name),
        params,
        permissions,
        event_contract,
        replay_level,
        version(1),
    ) else {
        panic!("test primitive definition must be valid");
    };
    definition
}

pub(super) fn create_entity_primitive() -> EffectPrimitiveDef {
    primitive_def(
        PRIMITIVE_CREATE_ENTITY,
        "create_entity",
        [EffectParamDef::new(
            param_name("entity"),
            EffectParamKind::EntityRole,
        )],
        [
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        EventContract::new([event_spec()]),
    )
}

pub(super) fn place_entity_primitive() -> EffectPrimitiveDef {
    primitive_def(
        PRIMITIVE_PLACE_ENTITY,
        "place_entity",
        [
            EffectParamDef::new(param_name("item"), EffectParamKind::EntityRole),
            EffectParamDef::new(param_name("destination"), EffectParamKind::EntityRole),
        ],
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        EventContract::new([event_spec()]),
    )
}

pub(super) fn acquire_reservation_primitive() -> EffectPrimitiveDef {
    primitive_def(
        PRIMITIVE_ACQUIRE_RESERVATION,
        "acquire_reservation",
        [
            EffectParamDef::new(param_name("item"), EffectParamKind::EntityRole),
            EffectParamDef::new(param_name("holder"), EffectParamKind::OptionalEntityRole),
        ],
        [
            StagePermission::AcquireReservation,
            StagePermission::EmitPhysicalEventRecord,
        ],
        EventContract::new([event_spec()]),
    )
}

pub(super) fn schedule_process_primitive() -> EffectPrimitiveDef {
    primitive_def_with_replay(
        PRIMITIVE_SCHEDULE_PROCESS,
        "schedule_process",
        [],
        [StagePermission::ScheduleProcess],
        EventContract::default(),
        ReplayLevel::AuditOnly,
    )
}

pub(super) fn unknown_semantics_primitive() -> EffectPrimitiveDef {
    primitive_def(
        PRIMITIVE_UNKNOWN_SEMANTICS,
        "unknown_semantics",
        [
            EffectParamDef::new(param_name("item"), EffectParamKind::EntityRole),
            EffectParamDef::new(param_name("destination"), EffectParamKind::EntityRole),
        ],
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        EventContract::new([event_spec()]),
    )
}

pub(super) fn role(name: &'static str) -> RoleDef {
    RoleDef::new(role_name(name), role_type("entity"))
}

pub(super) fn process_state_schema() -> ProcessStateSchema {
    let Ok(schema) = ProcessStateSchema::new([ProcessStateField::new(
        state_field_name("progress"),
        state_value_type("u64"),
    )]) else {
        panic!("test process state schema must be valid");
    };
    schema
}

pub(super) fn process_policies() -> ProcessPolicies {
    ProcessPolicies::new(
        policy_key("tick"),
        policy_key("wait"),
        policy_key("interrupt"),
        policy_key("resume"),
        policy_key("failure"),
    )
}

pub(super) fn process_support(
    tier: ResolutionTier,
    effect_program: DefinitionId,
) -> ResolutionSupport {
    let policy = match tier {
        ResolutionTier::Concrete => policy_key("concrete"),
        ResolutionTier::Abstract => policy_key("abstract"),
        ResolutionTier::Strategic => policy_key("strategic"),
        _ => policy_key("other"),
    };
    let Ok(support) = ResolutionSupport::new(tier, policy, [effect_program]) else {
        panic!("test process support must be valid");
    };
    support
}

pub(super) fn op(
    primitive: EffectPrimitiveId,
    args: impl IntoIterator<Item = EffectArgBinding>,
    emitted_events: impl IntoIterator<Item = EventRecordSpec>,
) -> EffectOp {
    let Ok(op) = EffectOp::new(primitive, args, emitted_events) else {
        panic!("test effect op must be valid");
    };
    op
}

pub(super) fn place_program_with_id(id: u64, primitive: EffectPrimitiveId) -> EffectProgramDef {
    let event = event_spec();
    let Ok(program) = EffectProgramDef::new(
        definition(id),
        definition_name("place"),
        [op(
            primitive,
            [arg("item", "item"), arg("destination", "destination")],
            [event.clone()],
        )],
        EventContract::new([event]),
        ReplayLevel::EventRebuild,
        version(1),
    ) else {
        panic!("test effect program must be valid");
    };
    program
}

pub(super) fn place_program(primitive: EffectPrimitiveId) -> EffectProgramDef {
    place_program_with_id(1, primitive)
}

pub(super) fn reservation_program() -> EffectProgramDef {
    let event = event_spec();
    let Ok(program) = EffectProgramDef::new(
        definition(1),
        definition_name("reserve_item"),
        [op(
            primitive_id(PRIMITIVE_ACQUIRE_RESERVATION),
            [arg("item", "item"), arg("holder", "actor")],
            [event.clone()],
        )],
        EventContract::new([event]),
        ReplayLevel::EventRebuild,
        version(1),
    ) else {
        panic!("test reservation program must be valid");
    };
    program
}

pub(super) fn process_tick_program() -> EffectProgramDef {
    let Ok(program) = EffectProgramDef::new(
        definition(1),
        definition_name("process_tick"),
        [op(primitive_id(PRIMITIVE_SCHEDULE_PROCESS), [], [])],
        EventContract::default(),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("test process program must be valid");
    };
    program
}

pub(super) fn place_action_with_ids(
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
    )
    .and_then(|action| action.with_actor_role(role_name("actor"))) else {
        panic!("test action must be valid");
    };
    action
}

pub(super) fn place_action(stage_permissions: Vec<StagePermission>) -> ActionDef {
    place_action_with_ids(2, 1, stage_permissions)
}

pub(super) fn haul_process_with_resolutions(
    effect_program: DefinitionId,
    resolutions: impl IntoIterator<Item = ResolutionTier>,
) -> ProcessDef {
    let Ok(process) = ProcessDef::new(
        definition(3),
        definition_name("haul_supplies"),
        [role("actor"), role("item"), role("destination")],
        process_state_schema(),
        resolutions
            .into_iter()
            .map(|resolution| process_support(resolution, effect_program)),
        process_policies(),
        EventContract::default(),
        [StagePermission::ReadWorld, StagePermission::ScheduleProcess],
        version(3),
    ) else {
        panic!("test process must be valid");
    };
    process
}

pub(super) fn registry(
    primitive: EffectPrimitiveDef,
    program: EffectProgramDef,
    action: ActionDef,
) -> DefinitionRegistry {
    let Ok(registry) = DefinitionRegistry::new([primitive], [program], [action], [], []) else {
        panic!("test registry must be valid");
    };
    registry
}

pub(super) fn process_registry() -> DefinitionRegistry {
    process_registry_with_resolutions([ResolutionTier::Concrete, ResolutionTier::Abstract])
}

pub(super) fn process_registry_with_resolutions(
    resolutions: impl IntoIterator<Item = ResolutionTier>,
) -> DefinitionRegistry {
    let program = process_tick_program();
    let process = haul_process_with_resolutions(program.id(), resolutions);
    let Ok(registry) =
        DefinitionRegistry::new([schedule_process_primitive()], [program], [], [process], [])
    else {
        panic!("test process registry must be valid");
    };
    registry
}

pub(super) fn empty_registry() -> DefinitionRegistry {
    let Ok(registry) = DefinitionRegistry::new([], [], [], [], []) else {
        panic!("empty registry must be valid");
    };
    registry
}

pub(super) fn place_registry() -> DefinitionRegistry {
    registry(
        place_entity_primitive(),
        place_program(primitive_id(PRIMITIVE_PLACE_ENTITY)),
        place_action(vec![
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ]),
    )
}

pub(super) fn reservation_registry() -> DefinitionRegistry {
    registry(
        acquire_reservation_primitive(),
        reservation_program(),
        place_action(vec![
            StagePermission::AcquireReservation,
            StagePermission::EmitPhysicalEventRecord,
        ]),
    )
}

pub(super) fn insert_then_place_registry() -> DefinitionRegistry {
    let event = event_spec();
    let Ok(program) = EffectProgramDef::new(
        definition(1),
        definition_name("insert_then_place"),
        [
            op(
                primitive_id(PRIMITIVE_CREATE_ENTITY),
                [arg("entity", "entity")],
                [event.clone()],
            ),
            op(
                primitive_id(PRIMITIVE_PLACE_ENTITY),
                [arg("item", "item"), arg("destination", "destination")],
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
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(2),
    )
    .and_then(|action| action.with_actor_role(role_name("actor"))) else {
        panic!("test action must be valid");
    };

    let Ok(registry) = DefinitionRegistry::new(
        [create_entity_primitive(), place_entity_primitive()],
        [program],
        [action],
        [],
        [],
    ) else {
        panic!("test insert-then-place registry must be valid");
    };
    registry
}

pub(super) fn place_request_for(action: u64) -> RuntimeRequest {
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

pub(super) fn place_request() -> RuntimeRequest {
    place_request_for(2)
}

pub(super) fn insert_then_place_request() -> RuntimeRequest {
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

fn semantics_for(definitions: &DefinitionRegistry) -> PrimitiveSemanticsRegistry {
    let mut builder = PrimitiveSemanticsRegistryBuilder::new();
    if definitions
        .effect_primitive(primitive_id(PRIMITIVE_CREATE_ENTITY))
        .is_some()
    {
        add_handler(&mut builder, TestCreateEntity);
    }
    if definitions
        .effect_primitive(primitive_id(PRIMITIVE_PLACE_ENTITY))
        .is_some()
    {
        add_handler(&mut builder, TestPlaceEntity);
    }
    if definitions
        .effect_primitive(primitive_id(PRIMITIVE_ACQUIRE_RESERVATION))
        .is_some()
    {
        add_handler(&mut builder, TestAcquireReservation);
    }
    let Ok(registry) = builder.build_against(definitions) else {
        panic!("test semantics registry should match definitions");
    };
    registry
}

fn add_handler(builder: &mut PrimitiveSemanticsRegistryBuilder, handler: impl PrimitiveSemantics) {
    if let Err(error) = builder.add_handler(handler) {
        panic!("test primitive handler should install: {error}");
    }
}

fn contract(definition: EffectPrimitiveDef) -> PrimitiveSemanticsContract {
    PrimitiveSemanticsContract::new(
        definition.params().to_vec(),
        definition.required_permissions().copied(),
        definition.event_contract().clone(),
        definition.replay_level(),
        definition.version(),
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TestCreateEntity;

impl PrimitiveSemantics for TestCreateEntity {
    fn primitive(&self) -> EffectPrimitiveId {
        primitive_id(PRIMITIVE_CREATE_ENTITY)
    }

    fn contract(&self) -> PrimitiveSemanticsContract {
        contract(create_entity_primitive())
    }

    fn validate(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveValidationContext<'_>,
    ) -> Result<(), PrimitiveValidationFailure> {
        let role = invocation.required_role(&param_name("entity"))?;
        let (role, entity) = context.required_role_entity(&role)?;
        if context.contains_entity(entity) {
            return Err(RejectedOutcome::new(
                context.action(),
                RejectionReason::EntityAlreadyPresent { role, entity },
            )
            .into());
        }
        context.insert_entity(entity);
        Ok(())
    }

    fn stage(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveStageContext<'_, '_, '_, '_>,
    ) -> Result<(), RuntimeError> {
        let role = invocation.required_role(&param_name("entity"))?;
        let (role, entity) = context.required_role_entity(&role)?;
        if context.contains_entity(entity) {
            return Err(RuntimeError::DuplicateVisibleEntity { role, entity });
        }
        context.stage_physical_change(
            invocation,
            HardStateChange::insert_entity(entity, None, context.provenance()),
        )?;
        context.emit_declared_events(invocation)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TestPlaceEntity;

impl PrimitiveSemantics for TestPlaceEntity {
    fn primitive(&self) -> EffectPrimitiveId {
        primitive_id(PRIMITIVE_PLACE_ENTITY)
    }

    fn contract(&self) -> PrimitiveSemanticsContract {
        contract(place_entity_primitive())
    }

    fn validate(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveValidationContext<'_>,
    ) -> Result<(), PrimitiveValidationFailure> {
        let item_role = invocation.required_role(&param_name("item"))?;
        let destination_role = invocation.required_role(&param_name("destination"))?;
        let (item_role, item) = context.required_role_entity(&item_role)?;
        let (destination_role, destination) = context.required_role_entity(&destination_role)?;
        validate_visible_entity(context, item_role, item)?;
        validate_visible_entity(context, destination_role, destination)?;

        let relation = RelationKey::new(item, RelationFamily::ContainedIn, destination);
        if context.contains_relation(relation) {
            return Err(RejectedOutcome::new(
                context.action(),
                RejectionReason::RelationAlreadyPresent {
                    subject: item,
                    family: RelationFamily::ContainedIn,
                    object: destination,
                },
            )
            .into());
        }
        context.insert_relation(relation);
        Ok(())
    }

    fn stage(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveStageContext<'_, '_, '_, '_>,
    ) -> Result<(), RuntimeError> {
        let item_role = invocation.required_role(&param_name("item"))?;
        let destination_role = invocation.required_role(&param_name("destination"))?;
        let (item_role, item) = context.required_role_entity(&item_role)?;
        let (destination_role, destination) = context.required_role_entity(&destination_role)?;
        require_visible_entity(context, item_role, item)?;
        require_visible_entity(context, destination_role, destination)?;

        let relation = RelationKey::new(item, RelationFamily::ContainedIn, destination);
        if context.contains_relation(relation) {
            return Err(RuntimeError::DuplicateVisibleRelation {
                subject: item,
                family: RelationFamily::ContainedIn,
                object: destination,
            });
        }
        context.stage_physical_change(
            invocation,
            HardStateChange::insert_relation(
                item,
                RelationFamily::ContainedIn,
                destination,
                context.provenance(),
            ),
        )?;
        context.emit_declared_events(invocation)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TestAcquireReservation;

impl PrimitiveSemantics for TestAcquireReservation {
    fn primitive(&self) -> EffectPrimitiveId {
        primitive_id(PRIMITIVE_ACQUIRE_RESERVATION)
    }

    fn contract(&self) -> PrimitiveSemanticsContract {
        contract(acquire_reservation_primitive())
    }

    fn validate(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveValidationContext<'_>,
    ) -> Result<(), PrimitiveValidationFailure> {
        let item_role = invocation.required_role(&param_name("item"))?;
        let (_, item) = context.required_role_entity(&item_role)?;
        let target = ReservationTarget::Entity(item);
        if context.contains_active_reservation(&target) {
            return Err(RejectedOutcome::new(
                context.action(),
                RejectionReason::ReservationAlreadyHeld { target },
            )
            .into());
        }
        context.insert_reservation_target(target);
        Ok(())
    }

    fn stage(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveStageContext<'_, '_, '_, '_>,
    ) -> Result<(), RuntimeError> {
        let item_role = invocation.required_role(&param_name("item"))?;
        let (_, item) = context.required_role_entity(&item_role)?;
        let holder = match invocation.optional_role(&param_name("holder"))? {
            Some(role) => {
                let (_, entity) = context.required_role_entity(&role)?;
                world_model::ReservationHolder::Entity(entity)
            }
            None => world_model::ReservationHolder::Runtime,
        };
        context.stage_reservation_acquire(
            invocation,
            AcquireReservationRequest::new(
                holder,
                ReservationTarget::Entity(item),
                context.request_time(),
                context.provenance(),
            ),
        )?;
        context.emit_declared_events(invocation)
    }
}

fn validate_visible_entity(
    context: &PrimitiveValidationContext<'_>,
    role: RoleName,
    entity: EntityId,
) -> Result<(), PrimitiveValidationFailure> {
    if context.contains_entity(entity) {
        Ok(())
    } else {
        Err(RejectedOutcome::new(
            context.action(),
            RejectionReason::MissingEntity { role, entity },
        )
        .into())
    }
}

fn require_visible_entity(
    context: &PrimitiveStageContext<'_, '_, '_, '_>,
    role: RoleName,
    entity: EntityId,
) -> Result<(), RuntimeError> {
    if context.contains_entity(entity) {
        Ok(())
    } else {
        Err(RuntimeError::MissingVisibleEntity { role, entity })
    }
}

pub(super) fn runtime(definitions: DefinitionRegistry) -> CausalRuntime {
    let Ok(transaction_ids) = CausalTransactionIdIssuer::starting_at(1) else {
        panic!("test transaction id issuer must be valid");
    };
    let Ok(event_ids) = EventRecordIdIssuer::starting_at(1) else {
        panic!("test event id issuer must be valid");
    };
    let semantics = semantics_for(&definitions);
    match CausalRuntime::with_hard_issuers_for_empty_model(
        definitions,
        semantics,
        transaction_ids,
        event_ids,
    ) {
        Ok(runtime) => runtime,
        Err(error) => panic!("test runtime should be valid: {error}"),
    }
}

pub(super) fn runtime_for_model(
    definitions: DefinitionRegistry,
    model: &WorldModel,
) -> CausalRuntime {
    let Ok(transaction_ids) = CausalTransactionIdIssuer::starting_at(1) else {
        panic!("test transaction id issuer must be valid");
    };
    let Ok(event_ids) = EventRecordIdIssuer::starting_at(1) else {
        panic!("test event id issuer must be valid");
    };
    let semantics = semantics_for(&definitions);
    match CausalRuntime::with_hard_issuers_for_model(
        definitions,
        semantics,
        transaction_ids,
        event_ids,
        model,
    ) {
        Ok(runtime) => runtime,
        Err(error) => panic!("hydrated runtime should be valid: {error}"),
    }
}

pub(super) fn start_process_request(required_work: u64, first_tick: u64) -> StartProcessRequest {
    StartProcessRequest::new(
        definition(3),
        ResolutionTier::Concrete,
        world_model::ProcessWork::from_units(required_work),
        WakeupScheduleKey::new(SimulationTime::from_ticks(first_tick), 0, 0),
        SimulationTime::ZERO,
    )
    .with_owner(entity(1))
    .with_roles([
        world_model::ProcessRoleBinding::new(role_name("actor"), entity(1)),
        world_model::ProcessRoleBinding::new(role_name("item"), entity(2)),
        world_model::ProcessRoleBinding::new(role_name("destination"), entity(3)),
    ])
    .with_provenance(provenance(30))
}

pub(super) fn start_test_process(
    runtime: &mut CausalRuntime,
    model: &mut WorldModel,
) -> (world_core::ProcessInstanceId, world_core::ScheduledWakeupId) {
    let started = must_ok(runtime.start_process(model, start_process_request(3, 10)));
    let ProcessTransition::Started { process, wakeup } = started.transition() else {
        panic!("process start should report a started transition");
    };
    (process.id(), wakeup.id())
}

pub(super) fn seed_entities(model: &mut WorldModel, seed_transaction: u64, entities: &[EntityId]) {
    let transaction_id = transaction(seed_transaction);
    let mut invalidation = InvalidationPackage::new(InvalidationSource::HardCommit(transaction_id));
    invalidation
        .mark_authority_class(world_core::AuthorityClass::Hard)
        .mark_store_family(StoreFamily::EventHistory)
        .mark_store_family(StoreFamily::World);

    let commit = match AcceptedHardCommit::new(
        TransactionCommit::for_action(
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

pub(super) fn must_ok<T>(result: Result<T, RuntimeError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("unexpected runtime error: {error}"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EventHistoryCounts {
    transactions: usize,
    events: usize,
}

impl EventHistoryCounts {
    pub(super) fn capture(model: &WorldModel) -> Self {
        Self {
            transactions: model.event_history().transaction_count(),
            events: model.event_history().event_count(),
        }
    }

    pub(super) fn assert_delta(self, model: &WorldModel, transactions: usize, events: usize) {
        assert_eq!(
            EventHistoryCounts::capture(model),
            Self {
                transactions: self.transactions + transactions,
                events: self.events + events,
            }
        );
    }

    pub(super) fn assert_unchanged(self, model: &WorldModel) {
        self.assert_delta(model, 0, 0);
    }
}

pub(super) fn assert_single_wakeup_result(report: &DrainReport, expected: WakeupDrainResult) {
    assert_eq!(report.processed().len(), 1);
    assert_eq!(report.processed()[0].result(), &expected);
}

pub(super) fn held_reservation_for<'a>(
    model: &'a WorldModel,
    target: &ReservationTarget,
) -> &'a world_model::ReservationRecord {
    let mut reservations =
        model
            .runtime_control_store()
            .records()
            .filter_map(|record| match record.payload() {
                RuntimeControlRecordPayload::Reservation(reservation)
                    if reservation.target() == target
                        && matches!(reservation.state(), ReservationState::Held { .. }) =>
                {
                    Some(reservation)
                }
                _ => None,
            });
    let Some(reservation) = reservations.next() else {
        panic!("held reservation should exist for target {target:?}");
    };
    assert!(
        reservations.next().is_none(),
        "target should have exactly one held reservation"
    );
    reservation
}
