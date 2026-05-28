use world_context::{
    ActionRepertoireEntry, ActorContextInput, ActorContextOptions, ActorContextPipeline,
    ActorContextProjection, ActorContextRequest, ContextDiagnostic, ContextProjectionCompleteness,
    ContextProjectionKind, ContextProjectionReport, ContextProvenanceSource, RepertoireStatus,
};
use world_core::{ActorId, AuthorityClass, DefinitionId, ReplayLevel, VersionAnchor};
use world_defs::{
    ActionDef, DefinitionName, DefinitionRegistry, EffectOp, EffectPrimitiveDef, EffectPrimitiveId,
    EffectProgramDef, EventContract, RoleDef, RoleName, RoleType, StagePermission,
};
use world_model::{
    AuthorityRead, InvalidationPackage, InvalidationSource, StoreFamily, WorldModel,
};

#[test]
fn empty_model_projects_value_context() {
    let model = WorldModel::new();
    let registry = empty_registry();
    let projection = project(&model, &registry, actor(1));
    let context = projection.context();

    assert_eq!(context.actor(), actor(1));
    assert!(context.observations().is_empty());
    assert!(context.epistemic().is_empty());
    assert!(context.social().is_empty());
    assert!(context.capabilities().is_empty());
    assert!(context.repertoire().is_empty());
    assert!(context.affordances().is_empty());

    assert!(
        projection
            .report()
            .reads()
            .contains_authority_read(AuthorityRead::epistemic_store())
    );
    assert!(
        projection
            .report()
            .reads()
            .contains_authority_read(AuthorityRead::derived_view(AuthorityClass::ActorTruth))
    );
    assert!(
        projection
            .report()
            .provenance()
            .sources()
            .contains(&ContextProvenanceSource::ActorScope(actor(1)))
    );
    assert_projection_status(
        projection.report(),
        ContextProjectionKind::Observation,
        ContextProjectionCompleteness::Unavailable,
    );
    assert_projection_status(
        projection.report(),
        ContextProjectionKind::Epistemic,
        ContextProjectionCompleteness::Complete,
    );
    assert_projection_status(
        projection.report(),
        ContextProjectionKind::Social,
        ContextProjectionCompleteness::Unavailable,
    );
    assert_projection_status(
        projection.report(),
        ContextProjectionKind::Capability,
        ContextProjectionCompleteness::Unavailable,
    );
    assert_projection_status(
        projection.report(),
        ContextProjectionKind::Repertoire,
        ContextProjectionCompleteness::Shallow,
    );
    assert_projection_status(
        projection.report(),
        ContextProjectionKind::Affordance,
        ContextProjectionCompleteness::Unavailable,
    );
}

#[test]
fn actor_repertoire_contains_definition_schema_candidates() {
    let model = WorldModel::new();
    let registry = action_registry();
    let projection = project(&model, &registry, actor(1));
    let entries = projection.context().repertoire().entries();

    assert_eq!(
        entries
            .iter()
            .map(ActionRepertoireEntry::action)
            .collect::<Vec<_>>(),
        vec![definition(3), definition(5)]
    );
    assert_eq!(entries[0].status(), RepertoireStatus::ActorFacingSchema);
    assert_eq!(entries[0].actor_role(), &role_name("actor"));
    assert_eq!(entries[0].effect_program(), definition(2));
    assert_eq!(
        entries[0]
            .roles()
            .iter()
            .map(|role| role.name().clone())
            .collect::<Vec<_>>(),
        vec![role_name("actor")]
    );
    assert!(entries.iter().all(|entry| entry.action() != definition(4)));
}

#[test]
fn definition_schema_candidates_do_not_populate_capabilities() {
    let model = WorldModel::new();
    let registry = action_registry();
    let projection = project(&model, &registry, actor(1));

    assert!(projection.context().capabilities().is_empty());
    assert!(!projection.context().repertoire().is_empty());
}

#[test]
fn report_records_definition_dependencies_and_invalidation_inputs() {
    let model = WorldModel::new();
    let registry = action_registry();
    let projection = project(&model, &registry, actor(1));
    let reads = projection.report().reads();

    for definition in [definition(2), definition(3), definition(5)] {
        assert!(reads.contains_definition(definition));
    }
    assert!(!reads.contains_definition(definition(4)));

    let mut actor_truth = InvalidationPackage::new(InvalidationSource::Manual);
    actor_truth
        .mark_authority_class(AuthorityClass::ActorTruth)
        .mark_store_family(StoreFamily::Epistemic);
    assert!(reads.is_invalidated_by_model(&actor_truth));

    let mut hard_world = InvalidationPackage::new(InvalidationSource::Manual);
    hard_world
        .mark_authority_class(AuthorityClass::Hard)
        .mark_store_family(StoreFamily::World);
    assert!(!reads.is_invalidated_by_model(&hard_world));
}

#[test]
fn debug_diagnostics_are_structured_and_source_free() {
    let model = WorldModel::new();
    let registry = empty_registry();
    let request = ActorContextRequest::with_options(
        actor(1),
        ActorContextOptions::new().with_debug_diagnostics(true),
    );
    let projection = match ActorContextPipeline::new()
        .project(ActorContextInput::new(&model, &registry), request)
    {
        Ok(projection) => projection,
        Err(error) => panic!("empty context projection should succeed: {error}"),
    };

    assert_eq!(
        projection.report().diagnostics(),
        &[
            ContextDiagnostic::ProjectionUnavailable {
                projection: ContextProjectionKind::Observation,
            },
            ContextDiagnostic::ProjectionUnavailable {
                projection: ContextProjectionKind::Social,
            },
            ContextDiagnostic::ProjectionUnavailable {
                projection: ContextProjectionKind::Affordance,
            },
        ]
    );
}

#[test]
fn unavailable_projection_status_is_not_debug_only() {
    let model = WorldModel::new();
    let registry = empty_registry();
    let projection = project(&model, &registry, actor(1));

    assert!(projection.report().diagnostics().is_empty());
    assert_projection_status(
        projection.report(),
        ContextProjectionKind::Observation,
        ContextProjectionCompleteness::Unavailable,
    );
    assert_projection_status(
        projection.report(),
        ContextProjectionKind::Social,
        ContextProjectionCompleteness::Unavailable,
    );
    assert_projection_status(
        projection.report(),
        ContextProjectionKind::Affordance,
        ContextProjectionCompleteness::Unavailable,
    );
}

fn assert_projection_status(
    report: &ContextProjectionReport,
    projection: ContextProjectionKind,
    completeness: ContextProjectionCompleteness,
) {
    assert_eq!(
        report
            .status(projection)
            .map(|status| status.completeness()),
        Some(completeness)
    );
}

fn project<'a>(
    model: &'a WorldModel,
    registry: &'a DefinitionRegistry,
    actor: ActorId,
) -> ActorContextProjection {
    match ActorContextPipeline::new().project(
        ActorContextInput::new(model, registry),
        ActorContextRequest::new(actor),
    ) {
        Ok(projection) => projection,
        Err(error) => panic!("context projection should succeed: {error}"),
    }
}

fn empty_registry() -> DefinitionRegistry {
    match DefinitionRegistry::new([], [], [], [], []) {
        Ok(registry) => registry,
        Err(error) => panic!("empty registry should be valid: {error}"),
    }
}

fn action_registry() -> DefinitionRegistry {
    let primitive = validate_primitive();
    let program = validate_program();
    let actor_action = action(3, "actor_action", true);
    let background_action = action(4, "background_action", false);
    let later_actor_action = action(5, "later_actor_action", true);

    match DefinitionRegistry::new(
        [primitive],
        [program],
        [later_actor_action, background_action, actor_action],
        [],
        [],
    ) {
        Ok(registry) => registry,
        Err(error) => panic!("action registry should be valid: {error}"),
    }
}

fn validate_primitive() -> EffectPrimitiveDef {
    match EffectPrimitiveDef::new(
        primitive(1),
        definition_name("validate_actor_action"),
        [],
        [StagePermission::Validate],
        EventContract::default(),
        ReplayLevel::AuditOnly,
        version(1),
    ) {
        Ok(primitive) => primitive,
        Err(error) => panic!("primitive should be valid: {error}"),
    }
}

fn validate_program() -> EffectProgramDef {
    let operation = match EffectOp::new(primitive(1), [], []) {
        Ok(operation) => operation,
        Err(error) => panic!("operation should be valid: {error}"),
    };

    match EffectProgramDef::new(
        definition(2),
        definition_name("validate_program"),
        [operation],
        EventContract::default(),
        ReplayLevel::AuditOnly,
        version(1),
    ) {
        Ok(program) => program,
        Err(error) => panic!("program should be valid: {error}"),
    }
}

fn action(id: u64, name: &'static str, actor_facing: bool) -> ActionDef {
    let action = match ActionDef::new(
        definition(id),
        definition_name(name),
        [role("actor")],
        [],
        [],
        definition(2),
        EventContract::default(),
        [StagePermission::Validate],
        version(1),
    ) {
        Ok(action) => action,
        Err(error) => panic!("action should be valid: {error}"),
    };

    if actor_facing {
        match action.with_actor_role(role_name("actor")) {
            Ok(action) => action,
            Err(error) => panic!("actor role should be valid: {error}"),
        }
    } else {
        action
    }
}

fn actor(value: u64) -> ActorId {
    let Some(value) = ActorId::new(value) else {
        panic!("test actor id must be nonzero");
    };
    value
}

fn definition(value: u64) -> DefinitionId {
    let Some(value) = DefinitionId::new(value) else {
        panic!("test definition id must be nonzero");
    };
    value
}

fn primitive(value: u64) -> EffectPrimitiveId {
    EffectPrimitiveId::new(definition(value))
}

fn version(value: u64) -> VersionAnchor {
    let Some(value) = VersionAnchor::new(value) else {
        panic!("test version must be nonzero");
    };
    value
}

fn definition_name(value: &'static str) -> DefinitionName {
    let Some(value) = DefinitionName::new(value) else {
        panic!("test definition name must be non-empty");
    };
    value
}

fn role(value: &'static str) -> RoleDef {
    RoleDef::new(role_name(value), role_type("entity"))
}

fn role_name(value: &'static str) -> RoleName {
    let Some(value) = RoleName::new(value) else {
        panic!("test role name must be non-empty");
    };
    value
}

fn role_type(value: &'static str) -> RoleType {
    let Some(value) = RoleType::new(value) else {
        panic!("test role type must be non-empty");
    };
    value
}
