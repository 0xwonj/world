use core::fmt;

use world_defs::{
    ActionBindingData, ActionData, ArtifactData, BindingName, DefinitionKey, EffectCallData,
    EngineProtocolVersion, EventData, EventEmissionData, EventFieldBindingData, EventFieldData,
    EventFieldName, InterfaceVersion, LocalDefinitionName, OperationCallData, OperationKind,
    OperationName, OperationParameter, PackCoordinate, PackKey, PackManifestData, PackVersion,
    ParameterName, RuntimeRequirementData, SemanticInterfaceDescriptor, SemanticInterfaceKey,
    SemanticOperationDescriptor, ValueKind,
};

/// Durable key of the built-in standard pack.
pub const STANDARD_PACK_KEY: &str = "world.standard";

/// Durable key of the standard physical-transfer interface.
pub const STANDARD_TRANSFER_INTERFACE_KEY: &str = "world.standard.transfer";

const TRANSFER_ACTION: &str = "transfer-item";
const TRANSFER_EVENT: &str = "item-transferred";
const CAN_TRANSFER_OPERATION: &str = "can-transfer-item";
const TRANSFER_OPERATION: &str = "transfer-item";

/// Constructs the standard transfer semantic-interface contract.
#[must_use]
pub fn transfer_interface_descriptor() -> SemanticInterfaceDescriptor {
    let parameters = transfer_parameters();
    let predicate = declared(SemanticOperationDescriptor::new(
        declared(OperationName::parse(CAN_TRANSFER_OPERATION)),
        OperationKind::Predicate,
        parameters.clone(),
    ));
    let effect = declared(SemanticOperationDescriptor::new(
        declared(OperationName::parse(TRANSFER_OPERATION)),
        OperationKind::Effect,
        parameters,
    ));
    declared(SemanticInterfaceDescriptor::new(
        declared(SemanticInterfaceKey::parse(STANDARD_TRANSFER_INTERFACE_KEY)),
        declared(InterfaceVersion::new(1)),
        vec![predicate, effect],
    ))
}

/// Constructs the pure standard transfer artifact data.
///
/// Whole-artifact validity is deliberately established by
/// [`world_defs::ArtifactValidator`], just as it is for authored packs.
#[must_use]
pub fn transfer_artifact_data() -> ArtifactData {
    let descriptor = transfer_interface_descriptor();
    let pack_key = declared(PackKey::parse(STANDARD_PACK_KEY));
    let coordinate = PackCoordinate::new(pack_key.clone(), PackVersion::new(1, 0, 0));
    let manifest = PackManifestData::new(EngineProtocolVersion::new(1), coordinate, Vec::new());

    let actor = declared(BindingName::parse("actor"));
    let destination = declared(BindingName::parse("destination"));
    let item = declared(BindingName::parse("item"));
    let source = declared(BindingName::parse("source"));

    let bindings = vec![
        ActionBindingData::new(actor.clone(), ValueKind::Actor),
        ActionBindingData::new(destination.clone(), ValueKind::Entity),
        ActionBindingData::new(item.clone(), ValueKind::Entity),
        ActionBindingData::new(source.clone(), ValueKind::Entity),
    ];
    let arguments = vec![
        actor.clone(),
        item.clone(),
        source.clone(),
        destination.clone(),
    ];
    let requirement = RuntimeRequirementData::new(OperationCallData::new(
        descriptor.key().clone(),
        declared(OperationName::parse(CAN_TRANSFER_OPERATION)),
        arguments.clone(),
    ));
    let effect = EffectCallData::new(OperationCallData::new(
        descriptor.key().clone(),
        declared(OperationName::parse(TRANSFER_OPERATION)),
        arguments,
    ));

    let event_name = declared(LocalDefinitionName::parse(TRANSFER_EVENT));
    let event = EventData::new(
        event_name.clone(),
        vec![
            EventFieldData::new(declared(EventFieldName::parse("actor")), ValueKind::Actor),
            EventFieldData::new(
                declared(EventFieldName::parse("destination")),
                ValueKind::Entity,
            ),
            EventFieldData::new(declared(EventFieldName::parse("item")), ValueKind::Entity),
            EventFieldData::new(declared(EventFieldName::parse("source")), ValueKind::Entity),
        ],
    );
    let emission = EventEmissionData::new(
        DefinitionKey::new(pack_key, event_name),
        vec![
            EventFieldBindingData::new(declared(EventFieldName::parse("actor")), actor),
            EventFieldBindingData::new(declared(EventFieldName::parse("destination")), destination),
            EventFieldBindingData::new(declared(EventFieldName::parse("item")), item),
            EventFieldBindingData::new(declared(EventFieldName::parse("source")), source),
        ],
    );
    let action = ActionData::new(
        declared(LocalDefinitionName::parse(TRANSFER_ACTION)),
        bindings,
        vec![requirement],
        vec![effect],
        vec![emission],
    );

    ArtifactData::new(
        manifest,
        vec![descriptor.reference()],
        vec![action],
        vec![event],
    )
}

fn transfer_parameters() -> Vec<OperationParameter> {
    vec![
        OperationParameter::new(declared(ParameterName::parse("actor")), ValueKind::Actor),
        OperationParameter::new(declared(ParameterName::parse("item")), ValueKind::Entity),
        OperationParameter::new(declared(ParameterName::parse("source")), ValueKind::Entity),
        OperationParameter::new(
            declared(ParameterName::parse("destination")),
            ValueKind::Entity,
        ),
    ]
}

fn declared<T, E: fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("invalid built-in standard declaration: {error}"),
    }
}
