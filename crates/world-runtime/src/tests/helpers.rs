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

pub(super) fn effect_kind(value: &'static str) -> EffectKind {
    let Some(name) = EffectKind::new(value) else {
        panic!("test effect kinds must be non-empty");
    };
    name
}

pub(super) fn event_kind(value: &'static str) -> EventKind {
    let Some(name) = EventKind::new(value) else {
        panic!("test event kinds must be non-empty");
    };
    name
}

pub(super) fn event_spec() -> EventRecordSpec {
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
    kind: &'static str,
    permissions: impl IntoIterator<Item = StagePermission>,
    emitted_events: impl IntoIterator<Item = EventRecordSpec>,
) -> EffectOp {
    let Ok(op) = EffectOp::new(effect_kind(kind), permissions, emitted_events) else {
        panic!("test effect op must be valid");
    };
    op
}

pub(super) fn transfer_program_with_id(
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

pub(super) fn transfer_program(
    kind: &'static str,
    permissions: Vec<StagePermission>,
) -> EffectProgramDef {
    transfer_program_with_id(1, kind, permissions)
}

pub(super) fn process_tick_program() -> EffectProgramDef {
    let Ok(program) = EffectProgramDef::new(
        definition(1),
        definition_name("process_tick"),
        [op(
            "schedule_process",
            [StagePermission::ScheduleProcess],
            [],
        )],
        EventContract::default(),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("test process program must be valid");
    };
    program
}

pub(super) fn transfer_action_with_ids(
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

pub(super) fn transfer_action(stage_permissions: Vec<StagePermission>) -> ActionDef {
    transfer_action_with_ids(2, 1, stage_permissions)
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

pub(super) fn registry(program: EffectProgramDef, action: ActionDef) -> DefinitionRegistry {
    let Ok(registry) = DefinitionRegistry::new([program], [action], [], []) else {
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
    let Ok(registry) = DefinitionRegistry::new([program], [], [process], []) else {
        panic!("test process registry must be valid");
    };
    registry
}

pub(super) fn empty_registry() -> DefinitionRegistry {
    let Ok(registry) = DefinitionRegistry::new([], [], [], []) else {
        panic!("empty registry must be valid");
    };
    registry
}

pub(super) fn transfer_registry() -> DefinitionRegistry {
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

pub(super) fn reservation_registry() -> DefinitionRegistry {
    registry(
        transfer_program(
            "acquire_reservation",
            vec![
                StagePermission::AcquireReservation,
                StagePermission::EmitPhysicalEventRecord,
            ],
        ),
        transfer_action(vec![
            StagePermission::AcquireReservation,
            StagePermission::EmitPhysicalEventRecord,
        ]),
    )
}

pub(super) fn insert_then_transfer_registry() -> DefinitionRegistry {
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

pub(super) fn transfer_request_for(action: u64) -> RuntimeRequest {
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

pub(super) fn transfer_request() -> RuntimeRequest {
    transfer_request_for(2)
}

pub(super) fn insert_then_transfer_request() -> RuntimeRequest {
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

pub(super) fn runtime(definitions: DefinitionRegistry) -> CausalRuntime {
    let Ok(transaction_ids) = CausalTransactionIdIssuer::starting_at(1) else {
        panic!("test transaction id issuer must be valid");
    };
    let Ok(event_ids) = EventRecordIdIssuer::starting_at(1) else {
        panic!("test event id issuer must be valid");
    };
    CausalRuntime::with_hard_issuers_for_empty_model(definitions, transaction_ids, event_ids)
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
    match CausalRuntime::with_hard_issuers_for_model(definitions, transaction_ids, event_ids, model)
    {
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
