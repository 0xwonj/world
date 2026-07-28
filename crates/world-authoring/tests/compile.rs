use core::fmt;

use world_authoring::{
    AuthoringCompiler, CompilationDiagnostic, CompileRequest, PackSource, SourceGraphError,
};
use world_defs::{
    ActionBindingData, ActionData, ArtifactError, ArtifactValidator, BindingName, DefinitionKey,
    EffectCallData, EngineProtocolVersion, EventData, EventEmissionData, EventFieldBindingData,
    EventFieldData, EventFieldName, InterfaceVersion, LocalDefinitionName, OperationCallData,
    OperationKind, OperationName, OperationParameter, PackCoordinate, PackKey, PackVersion,
    ParameterName, RuntimeRequirementData, SemanticInterfaceCatalog, SemanticInterfaceDescriptor,
    SemanticInterfaceKey, SemanticOperationDescriptor, SourceSnapshotId, ValueKind,
};

fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("authoring fixture must be valid: {error}"),
    }
}

fn transfer_descriptor() -> SemanticInterfaceDescriptor {
    let parameters = || {
        vec![
            OperationParameter::new(valid(ParameterName::parse("actor")), ValueKind::Actor),
            OperationParameter::new(valid(ParameterName::parse("item")), ValueKind::Entity),
            OperationParameter::new(valid(ParameterName::parse("source")), ValueKind::Entity),
            OperationParameter::new(
                valid(ParameterName::parse("destination")),
                ValueKind::Entity,
            ),
        ]
    };
    let predicate = valid(SemanticOperationDescriptor::new(
        valid(OperationName::parse("can-transfer-item")),
        OperationKind::Predicate,
        parameters(),
    ));
    let effect = valid(SemanticOperationDescriptor::new(
        valid(OperationName::parse("transfer-item")),
        OperationKind::Effect,
        parameters(),
    ));
    valid(SemanticInterfaceDescriptor::new(
        valid(SemanticInterfaceKey::parse("world.standard.transfer")),
        valid(InterfaceVersion::new(1)),
        vec![effect, predicate],
    ))
}

fn transfer_source(include_effect: bool) -> PackSource {
    let pack_key = valid(PackKey::parse("world.standard"));
    let coordinate = PackCoordinate::new(pack_key.clone(), PackVersion::new(1, 0, 0));
    let interface = valid(SemanticInterfaceKey::parse("world.standard.transfer"));

    let actor = valid(BindingName::parse("actor"));
    let destination = valid(BindingName::parse("destination"));
    let item = valid(BindingName::parse("item"));
    let source = valid(BindingName::parse("source"));
    let arguments = vec![
        actor.clone(),
        item.clone(),
        source.clone(),
        destination.clone(),
    ];
    let requirement = RuntimeRequirementData::new(OperationCallData::new(
        interface.clone(),
        valid(OperationName::parse("can-transfer-item")),
        arguments.clone(),
    ));
    let effects = if include_effect {
        vec![EffectCallData::new(OperationCallData::new(
            interface,
            valid(OperationName::parse("transfer-item")),
            arguments,
        ))]
    } else {
        Vec::new()
    };

    let event_name = valid(LocalDefinitionName::parse("item-transferred"));
    let event = EventData::new(
        event_name.clone(),
        vec![
            EventFieldData::new(valid(EventFieldName::parse("source")), ValueKind::Entity),
            EventFieldData::new(valid(EventFieldName::parse("item")), ValueKind::Entity),
            EventFieldData::new(
                valid(EventFieldName::parse("destination")),
                ValueKind::Entity,
            ),
            EventFieldData::new(valid(EventFieldName::parse("actor")), ValueKind::Actor),
        ],
    );
    let emission = EventEmissionData::new(
        DefinitionKey::new(pack_key, event_name),
        vec![
            EventFieldBindingData::new(valid(EventFieldName::parse("source")), source.clone()),
            EventFieldBindingData::new(valid(EventFieldName::parse("item")), item.clone()),
            EventFieldBindingData::new(
                valid(EventFieldName::parse("destination")),
                destination.clone(),
            ),
            EventFieldBindingData::new(valid(EventFieldName::parse("actor")), actor.clone()),
        ],
    );
    let action = ActionData::new(
        valid(LocalDefinitionName::parse("transfer-item")),
        vec![
            ActionBindingData::new(source, ValueKind::Entity),
            ActionBindingData::new(item, ValueKind::Entity),
            ActionBindingData::new(destination, ValueKind::Entity),
            ActionBindingData::new(actor, ValueKind::Actor),
        ],
        vec![requirement],
        effects,
        vec![emission],
    );

    PackSource::new(
        SourceSnapshotId::from_bytes([0x53; 32]),
        EngineProtocolVersion::new(1),
        coordinate,
        Vec::new(),
        vec![action],
        vec![event],
    )
}

#[test]
fn independent_authoring_matches_the_standard_frozen_vectors() {
    let descriptor = transfer_descriptor();
    let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor]));
    let source = transfer_source(true);
    let root = source.coordinate().clone();
    let compilation =
        valid(AuthoringCompiler::new(&catalog).compile(CompileRequest::new(root, vec![source])));

    assert_eq!(compilation.envelopes().len(), 1);
    let envelope = &compilation.envelopes()[0];
    assert_eq!(envelope.blob().len(), 403);
    assert_eq!(
        envelope.descriptor().blob_digest().to_string(),
        "e66fb079d4c9716ab4307a2d30c09eb5cb4cb491dacd29f3a337f2576ffe3321"
    );
    assert_eq!(
        compilation.definitions().lock().digest().to_string(),
        "afddcc97a203fe8933ec2ed9417bcbad1df82d96693a7bce4b94048f8b0b78c2"
    );
    assert_eq!(
        compilation.definitions().digest().to_string(),
        "38fae8323c548dfd14e38e3b42485bf54cb41d9a531bf2f70d6bb51f644b67a3"
    );

    let loaded = valid(ArtifactValidator::new(&catalog).load(envelope.clone()));
    assert_eq!(
        loaded.semantic_fingerprint().to_string(),
        "95e414f0ad40e23c032fa991e6c142f506422da3aebe29e311c414c0322c1d8c"
    );
    assert_eq!(
        loaded.export_digest().to_string(),
        "a88a5915d1d488aea65d5175ef5e1cc705cb77f5e6c4d820f14de3de98ea6d42"
    );
}

#[test]
fn invalid_source_returns_one_diagnostic_and_no_partial_compilation() {
    let descriptor = transfer_descriptor();
    let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor]));
    let source = transfer_source(false);
    let root = source.coordinate().clone();
    let result = AuthoringCompiler::new(&catalog).compile(CompileRequest::new(root, vec![source]));
    let diagnostics = match result {
        Ok(_) => panic!("invalid authoring source unexpectedly compiled"),
        Err(diagnostics) => diagnostics,
    };

    assert_eq!(diagnostics.len(), 1);
    assert!(matches!(
        diagnostics.iter().next(),
        Some(CompilationDiagnostic::Artifact { error, .. })
            if matches!(error.as_ref(), ArtifactError::EmptyEffects { .. })
    ));
}

#[test]
fn conflicting_source_coordinates_are_rejected_independent_of_input_order() {
    let catalog = SemanticInterfaceCatalog::default();
    let pack = valid(PackKey::parse("world.conflict"));
    let first = PackCoordinate::new(pack.clone(), PackVersion::new(1, 0, 0));
    let second = PackCoordinate::new(pack.clone(), PackVersion::new(2, 0, 0));
    let first_source = PackSource::new(
        SourceSnapshotId::from_bytes([0x61; 32]),
        EngineProtocolVersion::new(1),
        first.clone(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let second_source = PackSource::new(
        SourceSnapshotId::from_bytes([0x62; 32]),
        EngineProtocolVersion::new(1),
        second.clone(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let compiler = AuthoringCompiler::new(&catalog);

    let forward = compiler.compile(CompileRequest::new(
        first.clone(),
        vec![first_source.clone(), second_source.clone()],
    ));
    let reverse = compiler.compile(CompileRequest::new(
        first.clone(),
        vec![second_source, first_source],
    ));
    assert_eq!(forward, reverse);

    let diagnostics = match forward {
        Ok(_) => panic!("conflicting source coordinates unexpectedly compiled"),
        Err(diagnostics) => diagnostics,
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics.iter().next(),
        Some(&CompilationDiagnostic::SourceGraph(
            SourceGraphError::ConflictingPackages {
                pack,
                first: Box::new(first),
                second: Box::new(second),
            }
        ))
    );
}
