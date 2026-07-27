use core::fmt;
use std::fmt::Write;

use world_defs::{
    ArtifactValidator, DefinitionKey, DefinitionLinker, ExactPackSet, ExactPackageSelection,
    SelectedPackage, SemanticInterfaceCatalog, SourceSnapshotId, ValueKind,
};
use world_standard::{STANDARD_PACK_KEY, transfer_artifact_data, transfer_interface_descriptor};

fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("standard transfer fixture must be valid: {error}"),
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        assert!(write!(&mut encoded, "{byte:02x}").is_ok());
    }
    encoded
}

#[test]
fn transfer_declaration_matches_the_frozen_protocol_vector() {
    let descriptor = transfer_interface_descriptor();
    assert_eq!(
        descriptor.digest().to_string(),
        "70f1b02ad7847bada652d0631a1385ff997dcefe9674d0ac7566a190cc3f2067"
    );
    let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor]));
    let validator = ArtifactValidator::new(&catalog);
    let verified = valid(validator.validate(transfer_artifact_data()));
    let loaded = valid(validator.load(verified.envelope().clone()));
    let selection = ExactPackageSelection::new(
        verified.coordinate().clone(),
        vec![SelectedPackage::new(
            verified.coordinate().clone(),
            SourceSnapshotId::from_bytes([0x53; 32]),
            Vec::new(),
        )],
    );
    let exact = valid(ExactPackSet::finalize(selection, vec![verified.clone()]));
    assert_eq!(
        exact.lock().digest().to_string(),
        "afddcc97a203fe8933ec2ed9417bcbad1df82d96693a7bce4b94048f8b0b78c2"
    );
    let definitions = valid(DefinitionLinker::link(exact));
    assert_eq!(
        definitions.digest().to_string(),
        "38fae8323c548dfd14e38e3b42485bf54cb41d9a531bf2f70d6bb51f644b67a3"
    );

    assert_eq!(
        hex(verified.envelope().blob()),
        "8401840101826e776f726c642e7374616e646172648301000080818377776f726c642e7374616e646172642e7472616e7366657201582070f1b02ad7847bada652d0631a1385ff997dcefe9674d0ac7566a190cc3f206782840101706974656d2d7472616e736665727265648482656163746f7200826b64657374696e6174696f6e0182646974656d018266736f75726365018700016d7472616e736665722d6974656d8482656163746f7200826b64657374696e6174696f6e0182646974656d018266736f75726365018183007163616e2d7472616e736665722d6974656d84656163746f72646974656d66736f757263656b64657374696e6174696f6e8183006d7472616e736665722d6974656d84656163746f72646974656d66736f757263656b64657374696e6174696f6e8182826e776f726c642e7374616e64617264706974656d2d7472616e736665727265648482656163746f72656163746f72826b64657374696e6174696f6e6b64657374696e6174696f6e82646974656d646974656d8266736f7572636566736f75726365"
    );
    assert_eq!(
        verified.artifact_digest().to_string(),
        "e66fb079d4c9716ab4307a2d30c09eb5cb4cb491dacd29f3a337f2576ffe3321"
    );
    assert_eq!(
        verified.export_digest().to_string(),
        "a88a5915d1d488aea65d5175ef5e1cc705cb77f5e6c4d820f14de3de98ea6d42"
    );
    assert_eq!(
        verified.semantic_fingerprint().to_string(),
        "95e414f0ad40e23c032fa991e6c142f506422da3aebe29e311c414c0322c1d8c"
    );

    assert_eq!(loaded.artifact_digest(), verified.artifact_digest());
    assert_eq!(
        loaded.semantic_fingerprint(),
        verified.semantic_fingerprint()
    );
    assert_eq!(verified.coordinate().pack_key().as_str(), STANDARD_PACK_KEY);
    assert_eq!(verified.dependencies(), []);
    assert_eq!(verified.required_interfaces().len(), 1);
    assert_eq!(verified.actions().len(), 1);
    assert_eq!(verified.events().len(), 1);

    let action = &verified.actions()[0];
    let binding_shape: Vec<_> = action
        .bindings()
        .iter()
        .map(|binding| (binding.name().as_str(), *binding.value_kind()))
        .collect();
    assert_eq!(
        binding_shape,
        [
            ("actor", ValueKind::Actor),
            ("destination", ValueKind::Entity),
            ("item", ValueKind::Entity),
            ("source", ValueKind::Entity),
        ]
    );
    assert_eq!(action.requirements().len(), 1);
    assert_eq!(action.effects().len(), 1);
    assert_eq!(action.success_events().len(), 1);

    let event = &verified.events()[0];
    let event_key = DefinitionKey::new(
        verified.coordinate().pack_key().clone(),
        event.name().clone(),
    );
    assert!(definitions.event(&event_key).is_some());
    assert_eq!(action.success_events()[0].event(), &event_key);
    let field_shape: Vec<_> = event
        .fields()
        .iter()
        .map(|field| (field.name().as_str(), *field.value_kind()))
        .collect();
    assert_eq!(
        field_shape,
        [
            ("actor", ValueKind::Actor),
            ("destination", ValueKind::Entity),
            ("item", ValueKind::Entity),
            ("source", ValueKind::Entity),
        ]
    );
}
