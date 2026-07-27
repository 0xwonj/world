use core::fmt;

use world_defs::{
    ActionBindingData, ActionData, ArtifactData, ArtifactValidator, BindingName, DefinitionKey,
    DefinitionLinker, EffectCallData, EngineProtocolVersion, EventData, EventEmissionData,
    EventFieldBindingData, EventFieldData, EventFieldName, ExactPackSet, ExactPackageSelection,
    InterfaceVersion, LinkError, LocalDefinitionName, OperationCallData, OperationKind,
    OperationName, OperationParameter, PackCoordinate, PackDependency, PackKey, PackManifestData,
    PackVersion, ParameterName, RuntimeDefinitionSet, SelectedPackage, SemanticInterfaceCatalog,
    SemanticInterfaceDescriptor, SemanticInterfaceKey, SemanticInterfaceReference,
    SemanticOperationDescriptor, SourceSnapshotId, ValueKind, VerifiedPackArtifact,
};

struct LinkFixture {
    root_coordinate: PackCoordinate,
    leaf_coordinate: PackCoordinate,
    root: VerifiedPackArtifact,
    leaf: VerifiedPackArtifact,
}

fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("link fixture must be valid: {error}"),
    }
}

fn rejected(result: Result<RuntimeDefinitionSet, LinkError>) -> LinkError {
    match result {
        Ok(_) => panic!("invalid link fixture was accepted"),
        Err(error) => error,
    }
}

fn pack_key(value: &str) -> PackKey {
    valid(PackKey::parse(value))
}

fn local_name(value: &str) -> LocalDefinitionName {
    valid(LocalDefinitionName::parse(value))
}

fn binding_name(value: &str) -> BindingName {
    valid(BindingName::parse(value))
}

fn field_name(value: &str) -> EventFieldName {
    valid(EventFieldName::parse(value))
}

fn coordinate(value: &str) -> PackCoordinate {
    PackCoordinate::new(pack_key(value), PackVersion::new(1, 0, 0))
}

fn effect_interface() -> (
    SemanticInterfaceCatalog,
    SemanticInterfaceReference,
    SemanticInterfaceKey,
    OperationName,
) {
    let interface_key = valid(SemanticInterfaceKey::parse("test.effects"));
    let operation_name = valid(OperationName::parse("apply"));
    let parameter =
        OperationParameter::new(valid(ParameterName::parse("subject")), ValueKind::Entity);
    let operation = valid(SemanticOperationDescriptor::new(
        operation_name.clone(),
        OperationKind::Effect,
        vec![parameter],
    ));
    let descriptor = valid(SemanticInterfaceDescriptor::new(
        interface_key.clone(),
        valid(InterfaceVersion::new(1)),
        vec![operation],
    ));
    let reference = descriptor.reference();
    let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor]));
    (catalog, reference, interface_key, operation_name)
}

fn fixture(target_event: &str, event_value_kind: ValueKind, mapped_fields: &[&str]) -> LinkFixture {
    let (catalog, interface, interface_key, operation_name) = effect_interface();
    let validator = ArtifactValidator::new(&catalog);
    let root_coordinate = coordinate("test.root");
    let leaf_coordinate = coordinate("test.leaf");

    let leaf_event_name = local_name("changed");
    let leaf = valid(validator.validate(ArtifactData::new(
        PackManifestData::new(
            EngineProtocolVersion::new(1),
            leaf_coordinate.clone(),
            Vec::new(),
        ),
        Vec::new(),
        Vec::new(),
        vec![EventData::new(
            leaf_event_name,
            vec![EventFieldData::new(field_name("subject"), event_value_kind)],
        )],
    )));

    let subject = binding_name("subject");
    let effect = EffectCallData::new(OperationCallData::new(
        interface_key,
        operation_name,
        vec![subject.clone()],
    ));
    let emission = EventEmissionData::new(
        DefinitionKey::new(leaf_coordinate.pack_key().clone(), local_name(target_event)),
        mapped_fields
            .iter()
            .map(|field| EventFieldBindingData::new(field_name(field), subject.clone()))
            .collect(),
    );
    let root = valid(validator.validate(ArtifactData::new(
        PackManifestData::new(
            EngineProtocolVersion::new(1),
            root_coordinate.clone(),
            vec![PackDependency::new(
                leaf_coordinate.clone(),
                leaf.export_digest(),
            )],
        ),
        vec![interface],
        vec![ActionData::new(
            local_name("change"),
            vec![ActionBindingData::new(subject, ValueKind::Entity)],
            Vec::new(),
            vec![effect],
            vec![emission],
        )],
        Vec::new(),
    )));

    LinkFixture {
        root_coordinate,
        leaf_coordinate,
        root,
        leaf,
    }
}

fn exact_set(fixture: &LinkFixture, root_source: u8, leaf_source: u8) -> ExactPackSet {
    let selection = ExactPackageSelection::new(
        fixture.root_coordinate.clone(),
        vec![
            SelectedPackage::new(
                fixture.root_coordinate.clone(),
                SourceSnapshotId::from_bytes([root_source; 32]),
                vec![fixture.leaf_coordinate.clone()],
            ),
            SelectedPackage::new(
                fixture.leaf_coordinate.clone(),
                SourceSnapshotId::from_bytes([leaf_source; 32]),
                Vec::new(),
            ),
        ],
    );
    valid(ExactPackSet::finalize(
        selection,
        vec![fixture.root.clone(), fixture.leaf.clone()],
    ))
}

fn link_error(fixture: &LinkFixture) -> LinkError {
    rejected(DefinitionLinker::link(exact_set(fixture, 1, 2)))
}

#[test]
fn cross_pack_events_link_and_runtime_identity_excludes_source_provenance() {
    let fixture = fixture("changed", ValueKind::Entity, &["subject"]);
    let original = exact_set(&fixture, 1, 2);
    let changed_source = exact_set(&fixture, 9, 2);

    assert_eq!(original.root(), &fixture.root_coordinate);
    assert_eq!(original.lock().entries().len(), 2);
    assert_ne!(original.lock().digest(), changed_source.lock().digest());

    let original = valid(DefinitionLinker::link(original));
    let changed_source = valid(DefinitionLinker::link(changed_source));
    assert_ne!(original.lock().digest(), changed_source.lock().digest());
    assert_eq!(original.digest(), changed_source.digest());

    let root_action = DefinitionKey::new(
        fixture.root_coordinate.pack_key().clone(),
        local_name("change"),
    );
    let leaf_event = DefinitionKey::new(
        fixture.leaf_coordinate.pack_key().clone(),
        local_name("changed"),
    );
    assert!(
        original
            .artifact(fixture.root_coordinate.pack_key())
            .is_some()
    );
    assert!(
        original
            .artifact(fixture.leaf_coordinate.pack_key())
            .is_some()
    );
    assert_eq!(
        original.action(&root_action).map(|action| action.name()),
        Some(root_action.local_name())
    );
    assert_eq!(
        original.event(&leaf_event).map(|event| event.name()),
        Some(leaf_event.local_name())
    );
    assert_eq!(
        original
            .action(&root_action)
            .map(|action| { action.success_events()[0].event() }),
        Some(&leaf_event)
    );
}

#[test]
fn missing_cross_pack_event_definition_is_reported_by_the_linker() {
    let fixture = fixture("missing", ValueKind::Entity, &["subject"]);
    let set = exact_set(&fixture, 1, 2);
    let action = DefinitionKey::new(
        fixture.root_coordinate.pack_key().clone(),
        local_name("change"),
    );
    let event = DefinitionKey::new(
        fixture.leaf_coordinate.pack_key().clone(),
        local_name("missing"),
    );

    assert_eq!(
        rejected(DefinitionLinker::link(set)),
        LinkError::MissingEventDefinition {
            action: Box::new(action),
            event: Box::new(event),
        }
    );
}

#[test]
fn cross_pack_event_contract_mismatches_are_reported_by_the_linker() {
    let action = DefinitionKey::new(pack_key("test.root"), local_name("change"));
    let event = DefinitionKey::new(pack_key("test.leaf"), local_name("changed"));

    let wrong_arity = fixture("changed", ValueKind::Entity, &["extra", "subject"]);
    assert_eq!(
        link_error(&wrong_arity),
        LinkError::EventMappingArityMismatch {
            action: Box::new(action.clone()),
            event: Box::new(event.clone()),
            expected: 1,
            actual: 2,
        }
    );

    let wrong_field = fixture("changed", ValueKind::Entity, &["other"]);
    assert_eq!(
        link_error(&wrong_field),
        LinkError::EventFieldMismatch {
            action: Box::new(action.clone()),
            event: Box::new(event.clone()),
            expected: field_name("subject"),
            actual: field_name("other"),
        }
    );

    let wrong_kind = fixture("changed", ValueKind::Actor, &["subject"]);
    assert_eq!(
        link_error(&wrong_kind),
        LinkError::EventFieldKindMismatch {
            action: Box::new(action),
            event: Box::new(event),
            field: field_name("subject"),
            binding: binding_name("subject"),
            expected: ValueKind::Actor,
            actual: ValueKind::Entity,
        }
    );
}
