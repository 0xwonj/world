use core::fmt;

use world_context::{
    ActionPolicySemanticsId, CandidateCoverage, GroundedActionInteraction,
    RelocationActionDefinitions, RelocationActionDefinitionsError, RelocationActionVerb,
    RelocationProjector,
};
use world_core::{ActorId, EntityId, SimDuration};
use world_defs::{
    ActionBindingData, ActionData, ArtifactData, ArtifactValidator, BindingName, DefinitionKey,
    DefinitionLinker, EffectCallData, EngineProtocolVersion, EventData, EventEmissionData,
    EventFieldBindingData, EventFieldData, EventFieldName, ExactPackSet, ExactPackageSelection,
    InterfaceVersion, LocalDefinitionName, OperationCallData, OperationKind, OperationName,
    OperationParameter, PackCoordinate, PackKey, PackManifestData, PackVersion, ParameterName,
    RuntimeDefinitionSet, SelectedPackage, SemanticInterfaceCatalog, SemanticInterfaceDescriptor,
    SemanticInterfaceKey, SemanticOperationDescriptor, SourceSnapshotId, ValueKind,
};
use world_model::{
    ActionOpportunityId, DirectedRoute, RelocationInteraction, RelocationInteractionAnchor,
    RelocationInteractionScope,
};

const POLICY_SEMANTICS: ActionPolicySemanticsId = ActionPolicySemanticsId::from_bytes([0x71; 32]);

fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("fixture must be valid: {error}"))
}

fn actor(byte: u8) -> ActorId {
    ActorId::from_bytes([byte; 32])
}

fn entity(byte: u8) -> EntityId {
    EntityId::from_bytes([byte; 32])
}

fn key(definitions: &RuntimeDefinitionSet, name: &str) -> DefinitionKey {
    DefinitionKey::new(
        definitions.root().pack_key().clone(),
        valid(LocalDefinitionName::parse(name)),
    )
}

fn definitions(extra_resume_binding: bool) -> RuntimeDefinitionSet {
    let interface = valid(SemanticInterfaceKey::parse("example.relocation"));
    let operation = valid(OperationName::parse("relocate"));
    let descriptor = valid(SemanticInterfaceDescriptor::new(
        interface.clone(),
        valid(InterfaceVersion::new(1)),
        vec![valid(SemanticOperationDescriptor::new(
            operation.clone(),
            OperationKind::Effect,
            vec![
                OperationParameter::new(valid(ParameterName::parse("actor")), ValueKind::Actor),
                OperationParameter::new(
                    valid(ParameterName::parse("destination")),
                    ValueKind::Entity,
                ),
                OperationParameter::new(valid(ParameterName::parse("source")), ValueKind::Entity),
            ],
        ))],
    ));
    let actor = valid(BindingName::parse("actor"));
    let destination = valid(BindingName::parse("destination"));
    let source = valid(BindingName::parse("source"));
    let item = valid(BindingName::parse("item"));
    let event_name = valid(LocalDefinitionName::parse("relocation-requested"));
    let pack = valid(PackKey::parse("example.relocation-pack"));
    let actions = ["start-relocation", "pause-relocation", "resume-relocation"]
        .into_iter()
        .map(|name| {
            let mut bindings = vec![
                ActionBindingData::new(actor.clone(), ValueKind::Actor),
                ActionBindingData::new(destination.clone(), ValueKind::Entity),
                ActionBindingData::new(source.clone(), ValueKind::Entity),
            ];
            if extra_resume_binding && name == "resume-relocation" {
                bindings.push(ActionBindingData::new(item.clone(), ValueKind::Entity));
            }
            ActionData::new(
                valid(LocalDefinitionName::parse(name)),
                bindings,
                Vec::new(),
                vec![EffectCallData::new(OperationCallData::new(
                    interface.clone(),
                    operation.clone(),
                    vec![actor.clone(), destination.clone(), source.clone()],
                ))],
                vec![EventEmissionData::new(
                    DefinitionKey::new(pack.clone(), event_name.clone()),
                    vec![
                        EventFieldBindingData::new(
                            valid(EventFieldName::parse("actor")),
                            actor.clone(),
                        ),
                        EventFieldBindingData::new(
                            valid(EventFieldName::parse("destination")),
                            destination.clone(),
                        ),
                        EventFieldBindingData::new(
                            valid(EventFieldName::parse("source")),
                            source.clone(),
                        ),
                    ],
                )],
            )
        })
        .collect();
    let coordinate = PackCoordinate::new(pack, PackVersion::new(1, 0, 0));
    let artifact = valid(
        ArtifactValidator::new(&valid(SemanticInterfaceCatalog::new(vec![
            descriptor.clone(),
        ])))
        .validate(ArtifactData::new(
            PackManifestData::new(
                EngineProtocolVersion::new(1),
                coordinate.clone(),
                Vec::new(),
            ),
            vec![descriptor.reference()],
            actions,
            vec![EventData::new(
                event_name,
                vec![
                    EventFieldData::new(valid(EventFieldName::parse("actor")), ValueKind::Actor),
                    EventFieldData::new(
                        valid(EventFieldName::parse("destination")),
                        ValueKind::Entity,
                    ),
                    EventFieldData::new(valid(EventFieldName::parse("source")), ValueKind::Entity),
                ],
            )],
        )),
    );
    valid(DefinitionLinker::link(valid(ExactPackSet::finalize(
        ExactPackageSelection::new(
            coordinate.clone(),
            vec![SelectedPackage::new(
                coordinate,
                SourceSnapshotId::from_bytes([0x51; 32]),
                Vec::new(),
            )],
        ),
        vec![artifact],
    ))))
}

fn family(definitions: &RuntimeDefinitionSet) -> RelocationActionDefinitions {
    valid(RelocationActionDefinitions::new(
        definitions,
        key(definitions, "start-relocation"),
        key(definitions, "pause-relocation"),
        key(definitions, "resume-relocation"),
    ))
}

fn route(source: EntityId, destination: EntityId, ticks: u64) -> DirectedRoute {
    valid(DirectedRoute::new(
        source,
        destination,
        SimDuration::from_ticks(ticks),
    ))
}

fn anchor(interaction: RelocationInteraction, route: DirectedRoute) -> RelocationInteractionAnchor {
    RelocationInteractionAnchor::new(interaction, route.source(), route.destination())
}

#[test]
fn action_family_requires_three_distinct_exact_typed_definitions() {
    let definition_set = definitions(false);
    let start = key(&definition_set, "start-relocation");
    let pause = key(&definition_set, "pause-relocation");
    let resume = key(&definition_set, "resume-relocation");

    assert_eq!(
        RelocationActionDefinitions::new(
            &definition_set,
            start.clone(),
            start.clone(),
            resume.clone(),
        ),
        Err(RelocationActionDefinitionsError::DuplicateAction { action: start })
    );
    let missing = DefinitionKey::new(
        definition_set.root().pack_key().clone(),
        valid(LocalDefinitionName::parse("missing-relocation")),
    );
    assert_eq!(
        RelocationActionDefinitions::new(
            &definition_set,
            key(&definition_set, "start-relocation"),
            pause,
            missing.clone(),
        ),
        Err(RelocationActionDefinitionsError::ActionUnavailable { action: missing })
    );

    let malformed = definitions(true);
    let malformed_resume = key(&malformed, "resume-relocation");
    assert_eq!(
        RelocationActionDefinitions::new(
            &malformed,
            key(&malformed, "start-relocation"),
            key(&malformed, "pause-relocation"),
            malformed_resume.clone(),
        ),
        Err(RelocationActionDefinitionsError::BindingShapeMismatch {
            action: malformed_resume,
        })
    );
}

#[test]
fn projector_exposes_only_verbs_action_keys_and_actor_safe_endpoints() {
    let acting = actor(0x10);
    let route_a = route(entity(0x20), entity(0x21), 3);
    let route_b = route(entity(0x30), entity(0x31), 5);
    let definitions = definitions(false);
    let actions = family(&definitions);
    let opportunity = ActionOpportunityId::from_bytes([0x61; 32]);
    let scope = valid(RelocationInteractionScope::new(
        vec![
            anchor(RelocationInteraction::Start(route_a.id()), route_a),
            anchor(RelocationInteraction::Pause(route_b.id()), route_b),
            anchor(RelocationInteraction::Resume(route_b.id()), route_b),
        ],
        3,
    ));
    let build = valid(RelocationProjector::new(&actions).build(
        acting,
        opportunity,
        &scope,
        &definitions,
        POLICY_SEMANTICS,
    ));

    assert_eq!(build.payload().actor(), acting);
    assert_eq!(
        build.payload().candidates().coverage(),
        CandidateCoverage::Complete
    );
    let interactions = build
        .payload()
        .interaction()
        .relocation()
        .unwrap_or_else(|| panic!("relocation projector must return a relocation view"))
        .interactions();
    assert_eq!(
        interactions.len(),
        build.payload().candidates().candidates().len()
    );
    assert_eq!(build.resolution().len(), interactions.len());

    for (candidate, public) in build
        .payload()
        .candidates()
        .candidates()
        .iter()
        .zip(interactions)
    {
        assert_eq!(
            candidate.interaction(),
            GroundedActionInteraction::Relocation(public.verb())
        );
        assert_eq!(
            candidate
                .bindings()
                .iter()
                .map(|binding| binding.name().as_str())
                .collect::<Vec<_>>(),
            ["actor", "destination", "source"]
        );
        let resolved = build
            .resolution()
            .resolve(candidate.id())
            .and_then(|selection| selection.relocation().cloned())
            .unwrap_or_else(|| panic!("relocation candidate must resolve privately"));
        assert_eq!(resolved.actor(), acting);
        assert_eq!(resolved.action(), candidate.action());
        assert_eq!(
            RelocationActionVerb::from(resolved.interaction()),
            public.verb()
        );
        assert!(
            [route_a.id(), route_b.id()].contains(&resolved.interaction().route()),
            "private resolution must retain one exact scoped interaction"
        );
        let public_source = candidate
            .bindings()
            .iter()
            .find(|binding| binding.name().as_str() == "source")
            .map(|binding| binding.value());
        let public_destination = candidate
            .bindings()
            .iter()
            .find(|binding| binding.name().as_str() == "destination")
            .map(|binding| binding.value());
        assert_eq!(
            public_source,
            Some(world_context::ActorSafeBindingValue::Object(
                public.source()
            ))
        );
        assert_eq!(
            public_destination,
            Some(world_context::ActorSafeBindingValue::Object(
                public.destination()
            ))
        );
    }
}

#[test]
fn pause_and_resume_ground_without_reading_runtime_process_control() {
    let acting = actor(0x10);
    let route = route(entity(0x20), entity(0x21), 3);
    let definitions = definitions(false);
    let actions = family(&definitions);
    let scope = valid(RelocationInteractionScope::new(
        vec![
            anchor(RelocationInteraction::Pause(route.id()), route),
            anchor(RelocationInteraction::Resume(route.id()), route),
        ],
        1,
    ));
    let build = valid(RelocationProjector::new(&actions).build(
        acting,
        ActionOpportunityId::from_bytes([0x61; 32]),
        &scope,
        &definitions,
        POLICY_SEMANTICS,
    ));

    assert_eq!(
        build.payload().candidates().coverage(),
        CandidateCoverage::BudgetLimited
    );
    assert_eq!(build.payload().candidates().candidates().len(), 1);
}

#[test]
fn actor_visible_anchors_determine_relocation_input_without_a_route_snapshot() {
    let acting = actor(0x10);
    let visible = route(entity(0x20), entity(0x21), 3);
    let definitions = definitions(false);
    let actions = family(&definitions);
    let opportunity = ActionOpportunityId::from_bytes([0x61; 32]);
    let scope = valid(RelocationInteractionScope::new(
        vec![anchor(RelocationInteraction::Start(visible.id()), visible)],
        1,
    ));
    let build = valid(RelocationProjector::new(&actions).build(
        acting,
        opportunity,
        &scope,
        &definitions,
        POLICY_SEMANTICS,
    ));

    let candidate = build.payload().candidates().candidates()[0].id();
    let resolution = build
        .resolution()
        .resolve(candidate)
        .and_then(|selection| selection.relocation().cloned())
        .unwrap_or_else(|| panic!("scoped interaction must resolve"));
    assert_eq!(
        resolution.interaction(),
        RelocationInteraction::Start(visible.id())
    );
}

#[test]
fn an_exact_scoped_route_need_not_exist_during_actor_safe_projection() {
    let definitions = definitions(false);
    let actions = family(&definitions);
    let missing = route(entity(0x20), entity(0x21), 3);
    let scope = valid(RelocationInteractionScope::new(
        vec![anchor(RelocationInteraction::Start(missing.id()), missing)],
        1,
    ));
    let build = valid(RelocationProjector::new(&actions).build(
        actor(0x10),
        ActionOpportunityId::from_bytes([0x61; 32]),
        &scope,
        &definitions,
        POLICY_SEMANTICS,
    ));

    assert_eq!(build.payload().candidates().candidates().len(), 1);
    let candidate = build.payload().candidates().candidates()[0].id();
    let selection = build
        .resolution()
        .resolve(candidate)
        .and_then(|selection| selection.relocation().cloned())
        .unwrap_or_else(|| panic!("missing authoritative route must remain actor-projectable"));
    assert_eq!(
        selection.interaction(),
        RelocationInteraction::Start(missing.id())
    );
}
