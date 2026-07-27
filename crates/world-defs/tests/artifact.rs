use core::fmt;

use world_core::SELECTED_DIGEST_ALGORITHM;
use world_defs::{
    ARTIFACT_FORMAT_VERSION, ActionBindingData, ActionData, ArtifactCodecError, ArtifactData,
    ArtifactDescriptor, ArtifactDigest, ArtifactEnvelope, ArtifactError, ArtifactMediaType,
    ArtifactValidator, BindingName, DefinitionKey, EffectCallData, EngineProtocolVersion,
    EventData, EventEmissionData, EventFieldBindingData, EventFieldData, EventFieldName,
    InterfaceVersion, LocalDefinitionName, MAX_ARTIFACT_BYTES, OperationCallData, OperationKind,
    OperationName, OperationParameter, PackCoordinate, PackDependency, PackExportDigest, PackKey,
    PackManifestData, PackVersion, ParameterName, RuntimeRequirementData, SemanticInterfaceCatalog,
    SemanticInterfaceDescriptor, SemanticInterfaceKey, SemanticOperationDescriptor, ValueKind,
    VerifiedPackArtifact,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputOrder {
    Canonical,
    Scrambled,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    None,
    WrongOperationStage,
    MissingBinding,
    DuplicateDefinitionNamespace,
    EmptyEffects,
    InvalidLocalEventMapping,
}

fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("artifact fixture must be valid: {error}"),
    }
}

fn rejected(result: Result<VerifiedPackArtifact, ArtifactError>) -> ArtifactError {
    match result {
        Ok(_) => panic!("invalid artifact fixture was accepted"),
        Err(error) => error,
    }
}

fn arranged<T>(mut values: Vec<T>, order: InputOrder) -> Vec<T> {
    if order == InputOrder::Scrambled {
        values.reverse();
    }
    values
}

fn pack_key(value: &str) -> PackKey {
    valid(PackKey::parse(value))
}

fn interface_key(value: &str) -> SemanticInterfaceKey {
    valid(SemanticInterfaceKey::parse(value))
}

fn local_name(value: &str) -> LocalDefinitionName {
    valid(LocalDefinitionName::parse(value))
}

fn operation_name(value: &str) -> OperationName {
    valid(OperationName::parse(value))
}

fn parameter_name(value: &str) -> ParameterName {
    valid(ParameterName::parse(value))
}

fn binding_name(value: &str) -> BindingName {
    valid(BindingName::parse(value))
}

fn event_field_name(value: &str) -> EventFieldName {
    valid(EventFieldName::parse(value))
}

fn coordinate(key: &str, version: (u32, u32, u32)) -> PackCoordinate {
    PackCoordinate::new(
        pack_key(key),
        PackVersion::new(version.0, version.1, version.2),
    )
}

fn operation(
    name: &str,
    kind: OperationKind,
    parameters: &[(&str, ValueKind)],
) -> SemanticOperationDescriptor {
    let parameters = parameters
        .iter()
        .map(|(name, kind)| OperationParameter::new(parameter_name(name), *kind))
        .collect();
    valid(SemanticOperationDescriptor::new(
        operation_name(name),
        kind,
        parameters,
    ))
}

fn audit_interface() -> SemanticInterfaceDescriptor {
    valid(SemanticInterfaceDescriptor::new(
        interface_key("example.audit"),
        valid(InterfaceVersion::new(1)),
        vec![
            operation(
                "record",
                OperationKind::Effect,
                &[("actor", ValueKind::Actor), ("item", ValueKind::Entity)],
            ),
            operation(
                "is-auditable",
                OperationKind::Predicate,
                &[("actor", ValueKind::Actor), ("item", ValueKind::Entity)],
            ),
        ],
    ))
}

fn inventory_interface() -> SemanticInterfaceDescriptor {
    let parameters = [
        ("actor", ValueKind::Actor),
        ("item", ValueKind::Entity),
        ("source", ValueKind::Entity),
        ("destination", ValueKind::Entity),
    ];
    valid(SemanticInterfaceDescriptor::new(
        interface_key("example.inventory"),
        valid(InterfaceVersion::new(1)),
        vec![
            operation("transfer", OperationKind::Effect, &parameters),
            operation("can-transfer", OperationKind::Predicate, &parameters),
        ],
    ))
}

fn unused_interface() -> SemanticInterfaceDescriptor {
    valid(SemanticInterfaceDescriptor::new(
        interface_key("example.unused"),
        valid(InterfaceVersion::new(1)),
        vec![operation(
            "inspect",
            OperationKind::Predicate,
            &[("item", ValueKind::Entity)],
        )],
    ))
}

fn call(interface: &str, operation: &str, arguments: &[&str]) -> OperationCallData {
    OperationCallData::new(
        interface_key(interface),
        operation_name(operation),
        arguments
            .iter()
            .map(|argument| binding_name(argument))
            .collect(),
    )
}

fn bindings(values: &[(&str, ValueKind)], order: InputOrder) -> Vec<ActionBindingData> {
    arranged(
        values
            .iter()
            .map(|(name, kind)| ActionBindingData::new(binding_name(name), *kind))
            .collect(),
        order,
    )
}

fn event_fields(values: &[(&str, ValueKind)], order: InputOrder) -> Vec<EventFieldData> {
    arranged(
        values
            .iter()
            .map(|(name, kind)| EventFieldData::new(event_field_name(name), *kind))
            .collect(),
        order,
    )
}

fn event_mappings(values: &[(&str, &str)], order: InputOrder) -> Vec<EventFieldBindingData> {
    arranged(
        values
            .iter()
            .map(|(field, binding)| {
                EventFieldBindingData::new(event_field_name(field), binding_name(binding))
            })
            .collect(),
        order,
    )
}

fn fixture(order: InputOrder, fault: Fault) -> (SemanticInterfaceCatalog, ArtifactData) {
    let audit = audit_interface();
    let inventory = inventory_interface();
    let catalog = valid(SemanticInterfaceCatalog::new(vec![
        inventory.clone(),
        audit.clone(),
    ]));
    let root_key = pack_key("example.pack");

    let dependencies = arranged(
        vec![
            PackDependency::new(
                coordinate("example.alpha", (2, 0, 0)),
                PackExportDigest::from_bytes([0x11; 32]),
            ),
            PackDependency::new(
                coordinate("example.zeta", (1, 3, 0)),
                PackExportDigest::from_bytes([0x22; 32]),
            ),
        ],
        order,
    );
    let interfaces = arranged(vec![audit.reference(), inventory.reference()], order);

    let audit_event_name = local_name("item-audited");
    let transfer_event_name = local_name("item-transferred");
    let events = arranged(
        vec![
            EventData::new(
                audit_event_name.clone(),
                event_fields(
                    &[("actor", ValueKind::Actor), ("item", ValueKind::Entity)],
                    order,
                ),
            ),
            EventData::new(
                transfer_event_name.clone(),
                event_fields(
                    &[
                        ("actor", ValueKind::Actor),
                        ("destination", ValueKind::Entity),
                        ("item", ValueKind::Entity),
                        ("source", ValueKind::Entity),
                    ],
                    order,
                ),
            ),
        ],
        order,
    );

    let audit_action_name = if fault == Fault::DuplicateDefinitionNamespace {
        audit_event_name.clone()
    } else {
        local_name("audit-item")
    };
    let audit_action = ActionData::new(
        audit_action_name,
        bindings(
            &[("actor", ValueKind::Actor), ("item", ValueKind::Entity)],
            order,
        ),
        vec![RuntimeRequirementData::new(call(
            "example.audit",
            "is-auditable",
            &["actor", "item"],
        ))],
        vec![EffectCallData::new(call(
            "example.audit",
            "record",
            &["actor", "item"],
        ))],
        vec![EventEmissionData::new(
            DefinitionKey::new(root_key.clone(), audit_event_name),
            event_mappings(&[("actor", "actor"), ("item", "item")], order),
        )],
    );

    let inventory_requirement = if fault == Fault::WrongOperationStage {
        RuntimeRequirementData::new(call(
            "example.inventory",
            "transfer",
            &["actor", "item", "source", "destination"],
        ))
    } else {
        RuntimeRequirementData::new(call(
            "example.inventory",
            "can-transfer",
            &["actor", "item", "source", "destination"],
        ))
    };
    let transfer_requirements = arranged(
        vec![
            RuntimeRequirementData::new(call("example.audit", "is-auditable", &["actor", "item"])),
            inventory_requirement,
        ],
        order,
    );
    let transfer_effects = if fault == Fault::EmptyEffects {
        Vec::new()
    } else {
        let arguments = if fault == Fault::MissingBinding {
            &["actor", "item", "source", "missing"][..]
        } else {
            &["actor", "item", "source", "destination"][..]
        };
        vec![EffectCallData::new(call(
            "example.inventory",
            "transfer",
            arguments,
        ))]
    };
    let transfer_mapping = if fault == Fault::InvalidLocalEventMapping {
        vec![
            ("carrier", "actor"),
            ("destination", "destination"),
            ("item", "item"),
            ("source", "source"),
        ]
    } else {
        vec![
            ("actor", "actor"),
            ("destination", "destination"),
            ("item", "item"),
            ("source", "source"),
        ]
    };
    let transfer_action = ActionData::new(
        local_name("transfer-item"),
        bindings(
            &[
                ("actor", ValueKind::Actor),
                ("destination", ValueKind::Entity),
                ("item", ValueKind::Entity),
                ("source", ValueKind::Entity),
            ],
            order,
        ),
        transfer_requirements,
        transfer_effects,
        vec![EventEmissionData::new(
            DefinitionKey::new(root_key.clone(), transfer_event_name),
            event_mappings(&transfer_mapping, order),
        )],
    );

    let manifest = PackManifestData::new(
        EngineProtocolVersion::new(1),
        PackCoordinate::new(root_key, PackVersion::new(1, 0, 0)),
        dependencies,
    );
    let actions = arranged(vec![audit_action, transfer_action], order);
    (
        catalog,
        ArtifactData::new(manifest, interfaces, actions, events),
    )
}

#[test]
fn validation_normalizes_nonsemantic_input_order() {
    let (catalog, canonical_data) = fixture(InputOrder::Canonical, Fault::None);
    let (_, scrambled_data) = fixture(InputOrder::Scrambled, Fault::None);
    let validator = ArtifactValidator::new(&catalog);
    let canonical = valid(validator.validate(canonical_data));
    let scrambled = valid(validator.validate(scrambled_data));

    assert_eq!(scrambled.envelope(), canonical.envelope());
    assert_eq!(scrambled.artifact_digest(), canonical.artifact_digest());
    assert_eq!(scrambled.export_digest(), canonical.export_digest());
    assert_eq!(
        scrambled.semantic_fingerprint(),
        canonical.semantic_fingerprint()
    );

    let dependencies: Vec<_> = scrambled
        .dependencies()
        .iter()
        .map(|dependency| dependency.coordinate().pack_key().as_str())
        .collect();
    assert_eq!(dependencies, ["example.alpha", "example.zeta"]);

    let interfaces: Vec<_> = scrambled
        .required_interfaces()
        .iter()
        .map(|reference| reference.key().as_str())
        .collect();
    assert_eq!(interfaces, ["example.audit", "example.inventory"]);

    let actions: Vec<_> = scrambled
        .actions()
        .iter()
        .map(|action| action.name().as_str())
        .collect();
    assert_eq!(actions, ["audit-item", "transfer-item"]);
    let events: Vec<_> = scrambled
        .events()
        .iter()
        .map(|event| event.name().as_str())
        .collect();
    assert_eq!(events, ["item-audited", "item-transferred"]);

    let transfer = &scrambled.actions()[1];
    let bindings: Vec<_> = transfer
        .bindings()
        .iter()
        .map(|binding| binding.name().as_str())
        .collect();
    assert_eq!(bindings, ["actor", "destination", "item", "source"]);
    let requirements: Vec<_> = transfer
        .requirements()
        .iter()
        .map(|requirement| {
            (
                requirement.call().interface().as_str(),
                requirement.call().operation().as_str(),
            )
        })
        .collect();
    assert_eq!(
        requirements,
        [
            ("example.audit", "is-auditable"),
            ("example.inventory", "can-transfer"),
        ]
    );
    let mappings: Vec<_> = transfer.success_events()[0]
        .field_bindings()
        .iter()
        .map(|mapping| (mapping.field().as_str(), mapping.binding().as_str()))
        .collect();
    assert_eq!(
        mappings,
        [
            ("actor", "actor"),
            ("destination", "destination"),
            ("item", "item"),
            ("source", "source"),
        ]
    );
}

#[test]
fn loading_directly_emitted_bytes_preserves_checked_semantics() {
    let (catalog, data) = fixture(InputOrder::Scrambled, Fault::None);
    let validator = ArtifactValidator::new(&catalog);
    let direct = valid(validator.validate(data));
    let loaded = valid(validator.load(direct.envelope().clone()));

    assert_eq!(loaded.coordinate(), direct.coordinate());
    assert_eq!(loaded.dependencies(), direct.dependencies());
    assert_eq!(loaded.required_interfaces(), direct.required_interfaces());
    assert_eq!(loaded.actions(), direct.actions());
    assert_eq!(loaded.events(), direct.events());
    assert_eq!(loaded.export_digest(), direct.export_digest());
    assert_eq!(loaded.semantic_fingerprint(), direct.semantic_fingerprint());
    assert_eq!(loaded.artifact_digest(), direct.artifact_digest());
}

#[test]
fn unused_catalog_entries_do_not_change_artifact_identity() {
    let (catalog, data) = fixture(InputOrder::Scrambled, Fault::None);
    let baseline = valid(ArtifactValidator::new(&catalog).validate(data.clone()));

    let mut descriptors = catalog.descriptors().to_vec();
    descriptors.push(unused_interface());
    let superset = valid(SemanticInterfaceCatalog::new(descriptors));
    let with_superset = valid(ArtifactValidator::new(&superset).validate(data));

    assert_eq!(with_superset.envelope(), baseline.envelope());
    assert_eq!(
        with_superset.semantic_fingerprint(),
        baseline.semantic_fingerprint()
    );
    assert_eq!(with_superset.export_digest(), baseline.export_digest());
}

#[test]
fn representative_semantic_failures_report_exact_errors() {
    let (catalog, data) = fixture(InputOrder::Canonical, Fault::WrongOperationStage);
    assert_eq!(
        rejected(ArtifactValidator::new(&catalog).validate(data)),
        ArtifactError::WrongOperationStage {
            action: local_name("transfer-item"),
            interface: interface_key("example.inventory"),
            operation: operation_name("transfer"),
            expected: OperationKind::Predicate,
            actual: OperationKind::Effect,
        }
    );

    let (catalog, data) = fixture(InputOrder::Canonical, Fault::MissingBinding);
    assert_eq!(
        rejected(ArtifactValidator::new(&catalog).validate(data)),
        ArtifactError::UnknownBinding {
            action: local_name("transfer-item"),
            binding: binding_name("missing"),
        }
    );

    let (catalog, data) = fixture(InputOrder::Canonical, Fault::DuplicateDefinitionNamespace);
    assert_eq!(
        rejected(ArtifactValidator::new(&catalog).validate(data)),
        ArtifactError::DuplicateDefinition {
            definition: local_name("item-audited"),
        }
    );

    let (catalog, data) = fixture(InputOrder::Canonical, Fault::EmptyEffects);
    assert_eq!(
        rejected(ArtifactValidator::new(&catalog).validate(data)),
        ArtifactError::EmptyEffects {
            action: local_name("transfer-item"),
        }
    );

    let (catalog, data) = fixture(InputOrder::Canonical, Fault::InvalidLocalEventMapping);
    assert_eq!(
        rejected(ArtifactValidator::new(&catalog).validate(data)),
        ArtifactError::EventFieldMismatch {
            action: local_name("transfer-item"),
            event: DefinitionKey::new(pack_key("example.pack"), local_name("item-transferred"),),
            expected: event_field_name("actor"),
            actual: event_field_name("carrier"),
        }
    );
}

#[test]
fn loading_rejects_descriptor_format_length_and_digest_mismatches() {
    let (catalog, data) = fixture(InputOrder::Canonical, Fault::None);
    let validator = ArtifactValidator::new(&catalog);
    let verified = valid(validator.validate(data));
    let descriptor = verified.envelope().descriptor();
    let blob = verified.envelope().blob().to_vec();
    let actual_length = blob.len() as u64;

    let format_descriptor = ArtifactDescriptor::new(
        descriptor.media_type(),
        ARTIFACT_FORMAT_VERSION + 1,
        descriptor.digest_algorithm(),
        actual_length,
        descriptor.blob_digest(),
    );
    assert_eq!(
        rejected(validator.load(ArtifactEnvelope::new(format_descriptor, blob.clone(),))),
        ArtifactError::UnsupportedFormatVersion {
            actual: ARTIFACT_FORMAT_VERSION + 1,
        }
    );

    let declared_length = actual_length + 1;
    let length_descriptor = ArtifactDescriptor::new(
        descriptor.media_type(),
        descriptor.format_version(),
        descriptor.digest_algorithm(),
        declared_length,
        descriptor.blob_digest(),
    );
    assert_eq!(
        rejected(validator.load(ArtifactEnvelope::new(length_descriptor, blob.clone()))),
        ArtifactError::LengthMismatch {
            declared: declared_length,
            actual: actual_length,
        }
    );

    let wrong_digest = ArtifactDigest::from_bytes([0xa5; 32]);
    assert_ne!(wrong_digest, descriptor.blob_digest());
    let digest_descriptor = ArtifactDescriptor::new(
        descriptor.media_type(),
        descriptor.format_version(),
        descriptor.digest_algorithm(),
        actual_length,
        wrong_digest,
    );
    assert_eq!(
        rejected(validator.load(ArtifactEnvelope::new(digest_descriptor, blob))),
        ArtifactError::DigestMismatch {
            declared: wrong_digest,
            actual: descriptor.blob_digest(),
        }
    );
}

#[test]
fn loading_accepts_longer_valid_integer_encoding_as_distinct_storage_identity() {
    let (catalog, data) = fixture(InputOrder::Canonical, Fault::None);
    let validator = ArtifactValidator::new(&catalog);
    let canonical = valid(validator.validate(data));
    let mut alternative_bytes = canonical.envelope().blob().to_vec();

    assert_eq!(alternative_bytes.get(1), Some(&0x01));
    alternative_bytes.splice(1..2, [0x18, 0x01]);
    let alternative_envelope = envelope_for_blob(alternative_bytes);
    let alternative = valid(validator.load(alternative_envelope));

    assert_ne!(
        alternative.artifact_digest(),
        canonical.artifact_digest(),
        "exact storage bytes have a distinct identity"
    );
    assert_eq!(alternative.coordinate(), canonical.coordinate());
    assert_eq!(alternative.dependencies(), canonical.dependencies());
    assert_eq!(
        alternative.required_interfaces(),
        canonical.required_interfaces()
    );
    assert_eq!(alternative.actions(), canonical.actions());
    assert_eq!(alternative.events(), canonical.events());
    assert_eq!(alternative.export_digest(), canonical.export_digest());
    assert_eq!(
        alternative.semantic_fingerprint(),
        canonical.semantic_fingerprint()
    );
}

#[test]
fn loader_rejects_schema_tag_and_arity_contract_violations() {
    let mut root_schema = minimal_artifact_blob();
    root_schema[1] = 2;
    assert!(matches!(
        load_error(root_schema),
        ArtifactError::Codec(ArtifactCodecError::UnsupportedSchema {
            schema: "artifact",
            expected: 1,
            actual: 2,
            ..
        })
    ));

    let mut manifest_schema = minimal_artifact_blob();
    manifest_schema[3] = 2;
    assert!(matches!(
        load_error(manifest_schema),
        ArtifactError::Codec(ArtifactCodecError::UnsupportedSchema {
            schema: "manifest",
            expected: 1,
            actual: 2,
            ..
        })
    ));

    let mut wrong_root_arity = minimal_artifact_blob();
    wrong_root_arity[0] = 0x83;
    assert!(matches!(
        load_error(wrong_root_arity),
        ArtifactError::Codec(ArtifactCodecError::WrongArrayLength {
            context: "artifact root",
            expected: 4,
            actual: 3,
            ..
        })
    ));

    let mut family_schema = minimal_artifact_prefix();
    family_schema.extend_from_slice(&[
        0x81, // one definition
        0x84, // event definition
        0x01, // event tag
        0x02, // unsupported family schema
    ]);
    assert!(matches!(
        load_error(family_schema),
        ArtifactError::Codec(ArtifactCodecError::UnsupportedSchema {
            schema: "event family",
            expected: 1,
            actual: 2,
            ..
        })
    ));

    let mut unknown_family = minimal_artifact_prefix();
    unknown_family.extend_from_slice(&[
        0x81, // one definition
        0x84, // definition array
        0x02, // unknown family tag
    ]);
    assert!(matches!(
        load_error(unknown_family),
        ArtifactError::Codec(ArtifactCodecError::UnknownDefinitionTag { actual: 2, .. })
    ));

    let mut unknown_value_kind = minimal_artifact_prefix();
    unknown_value_kind.extend_from_slice(&[
        0x81, // one definition
        0x84, // event definition
        0x01, // event tag
        0x01, // event family schema
        0x61, b'e', // event name
        0x81, // one field
        0x82, // event field
        0x61, b'f', // field name
        0x02, // unknown value-kind tag
    ]);
    assert!(matches!(
        load_error(unknown_value_kind),
        ArtifactError::Codec(ArtifactCodecError::UnknownValueKind { actual: 2, .. })
    ));
}

#[test]
fn loader_rejects_interface_slot_and_collection_limit_violations() {
    let mut out_of_range_slot = minimal_artifact_prefix();
    out_of_range_slot.extend_from_slice(&[
        0x81, // one definition
        0x87, // action definition
        0x00, // action tag
        0x01, // action family schema
        0x61, b'a', // action name
        0x80, // no bindings
        0x80, // no requirements
        0x81, // one effect
        0x83, // operation call
        0x00, // interface slot zero, but table is empty
        0x61, b'a', // operation name
        0x80, // no arguments
        0x80, // no success events
    ]);
    assert!(matches!(
        load_error(out_of_range_slot),
        ArtifactError::Codec(ArtifactCodecError::InterfaceSlotOutOfRange {
            slot: 0,
            available: 0,
            ..
        })
    ));

    let too_many_dependencies = vec![
        0x84, // artifact root
        0x01, // artifact schema
        0x84, // manifest
        0x01, // manifest schema
        0x00, // engine protocol
        0x82, 0x61, b'a', // coordinate and pack key
        0x83, 0x00, 0x00, 0x00, // version
        0x98, 0x81, // array length 129
    ];
    assert!(matches!(
        load_error(too_many_dependencies),
        ArtifactError::Codec(ArtifactCodecError::CollectionLimit {
            collection: "direct dependencies",
            actual: 129,
            maximum: 128,
            ..
        })
    ));
}

#[test]
fn loader_rejects_blob_larger_than_the_outer_protocol_limit() {
    let (catalog, data) = fixture(InputOrder::Canonical, Fault::None);
    let validator = ArtifactValidator::new(&catalog);
    let verified = valid(validator.validate(data));
    let template = verified.envelope().descriptor();
    let oversized = vec![0; MAX_ARTIFACT_BYTES + 1];
    let descriptor = ArtifactDescriptor::new(
        template.media_type(),
        template.format_version(),
        template.digest_algorithm(),
        oversized.len() as u64,
        ArtifactDigest::from_bytes([0; 32]),
    );

    assert_eq!(
        rejected(validator.load(ArtifactEnvelope::new(descriptor, oversized))),
        ArtifactError::ArtifactTooLarge {
            actual: MAX_ARTIFACT_BYTES + 1,
            maximum: MAX_ARTIFACT_BYTES,
        }
    );
}

fn minimal_artifact_prefix() -> Vec<u8> {
    vec![
        0x84, // artifact root
        0x01, // artifact schema
        0x84, // manifest
        0x01, // manifest schema
        0x00, // engine protocol
        0x82, 0x61, b'a', // coordinate and pack key
        0x83, 0x00, 0x00, 0x00, // version
        0x80, // no dependencies
        0x80, // no interfaces
    ]
}

fn minimal_artifact_blob() -> Vec<u8> {
    let mut blob = minimal_artifact_prefix();
    blob.push(0x80);
    blob
}

fn load_error(blob: Vec<u8>) -> ArtifactError {
    let catalog = SemanticInterfaceCatalog::default();
    let envelope = envelope_for_blob(blob);
    rejected(ArtifactValidator::new(&catalog).load(envelope))
}

fn envelope_for_blob(blob: Vec<u8>) -> ArtifactEnvelope {
    let descriptor = ArtifactDescriptor::new(
        ArtifactMediaType::WorldPackCbor,
        ARTIFACT_FORMAT_VERSION,
        SELECTED_DIGEST_ALGORITHM,
        blob.len() as u64,
        ArtifactDigest::of_blob_bytes(&blob),
    );
    ArtifactEnvelope::new(descriptor, blob)
}
