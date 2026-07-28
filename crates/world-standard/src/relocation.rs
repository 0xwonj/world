use core::fmt;

use world_defs::{
    ActionBindingData, ActionData, ArtifactData, BindingName, DefinitionKey, EffectCallData,
    EngineProtocolVersion, EventData, EventEmissionData, EventFieldBindingData, EventFieldData,
    EventFieldName, InterfaceVersion, LocalDefinitionName, OperationCallData, OperationKind,
    OperationName, OperationParameter, PackCoordinate, PackKey, PackManifestData, PackVersion,
    ParameterName, SemanticInterfaceDescriptor, SemanticInterfaceKey, SemanticOperationDescriptor,
    ValueKind,
};

/// Durable pack key of the built-in relocation vocabulary.
pub const STANDARD_RELOCATION_PACK_KEY: &str = "world.standard.relocation";

/// Durable key of the standard relocation operation interface.
pub const STANDARD_RELOCATION_INTERFACE_KEY: &str = "world.standard.relocation";

const START_ACTION: &str = "start-relocation";
const PAUSE_ACTION: &str = "pause-relocation";
const RESUME_ACTION: &str = "resume-relocation";

/// Returns the durable definition key of the relocation-start action.
#[must_use]
pub fn start_relocation_action_key() -> DefinitionKey {
    action_key(START_ACTION)
}

/// Returns the durable definition key of the relocation-pause action.
#[must_use]
pub fn pause_relocation_action_key() -> DefinitionKey {
    action_key(PAUSE_ACTION)
}

/// Returns the durable definition key of the relocation-resume action.
#[must_use]
pub fn resume_relocation_action_key() -> DefinitionKey {
    action_key(RESUME_ACTION)
}

/// Constructs the standard relocation semantic-interface contract.
#[must_use]
pub fn relocation_interface_descriptor() -> SemanticInterfaceDescriptor {
    declared(SemanticInterfaceDescriptor::new(
        declared(SemanticInterfaceKey::parse(
            STANDARD_RELOCATION_INTERFACE_KEY,
        )),
        declared(InterfaceVersion::new(1)),
        [START_ACTION, PAUSE_ACTION, RESUME_ACTION]
            .into_iter()
            .map(|operation| {
                declared(SemanticOperationDescriptor::new(
                    declared(OperationName::parse(operation)),
                    OperationKind::Effect,
                    relocation_parameters(),
                ))
            })
            .collect(),
    ))
}

/// Constructs the pure standard relocation artifact data.
///
/// Runtime activation for these declarations is intentionally a separate
/// composition step. This value contains no evaluator or process authority.
#[must_use]
pub fn relocation_artifact_data() -> ArtifactData {
    let descriptor = relocation_interface_descriptor();
    let pack = declared(PackKey::parse(STANDARD_RELOCATION_PACK_KEY));
    let coordinate = PackCoordinate::new(pack.clone(), PackVersion::new(1, 0, 0));
    let actor = declared(BindingName::parse("actor"));
    let destination = declared(BindingName::parse("destination"));
    let source = declared(BindingName::parse("source"));
    let bindings = vec![
        ActionBindingData::new(actor.clone(), ValueKind::Actor),
        ActionBindingData::new(destination.clone(), ValueKind::Entity),
        ActionBindingData::new(source.clone(), ValueKind::Entity),
    ];

    let mut actions = Vec::new();
    let mut events = Vec::new();
    for (action_name, event_name) in [
        (START_ACTION, "relocation-started"),
        (PAUSE_ACTION, "relocation-paused"),
        (RESUME_ACTION, "relocation-resumed"),
    ] {
        let event_name = declared(LocalDefinitionName::parse(event_name));
        events.push(EventData::new(
            event_name.clone(),
            relocation_event_fields(),
        ));
        actions.push(ActionData::new(
            declared(LocalDefinitionName::parse(action_name)),
            bindings.clone(),
            Vec::new(),
            vec![EffectCallData::new(OperationCallData::new(
                descriptor.key().clone(),
                declared(OperationName::parse(action_name)),
                vec![actor.clone(), destination.clone(), source.clone()],
            ))],
            vec![EventEmissionData::new(
                DefinitionKey::new(pack.clone(), event_name),
                vec![
                    EventFieldBindingData::new(
                        declared(EventFieldName::parse("actor")),
                        actor.clone(),
                    ),
                    EventFieldBindingData::new(
                        declared(EventFieldName::parse("destination")),
                        destination.clone(),
                    ),
                    EventFieldBindingData::new(
                        declared(EventFieldName::parse("source")),
                        source.clone(),
                    ),
                ],
            )],
        ));
    }

    ArtifactData::new(
        PackManifestData::new(EngineProtocolVersion::new(1), coordinate, Vec::new()),
        vec![descriptor.reference()],
        actions,
        events,
    )
}

fn action_key(name: &str) -> DefinitionKey {
    DefinitionKey::new(
        declared(PackKey::parse(STANDARD_RELOCATION_PACK_KEY)),
        declared(LocalDefinitionName::parse(name)),
    )
}

fn relocation_parameters() -> Vec<OperationParameter> {
    vec![
        OperationParameter::new(declared(ParameterName::parse("actor")), ValueKind::Actor),
        OperationParameter::new(
            declared(ParameterName::parse("destination")),
            ValueKind::Entity,
        ),
        OperationParameter::new(declared(ParameterName::parse("source")), ValueKind::Entity),
    ]
}

fn relocation_event_fields() -> Vec<EventFieldData> {
    vec![
        EventFieldData::new(declared(EventFieldName::parse("actor")), ValueKind::Actor),
        EventFieldData::new(
            declared(EventFieldName::parse("destination")),
            ValueKind::Entity,
        ),
        EventFieldData::new(declared(EventFieldName::parse("source")), ValueKind::Entity),
    ]
}

fn declared<T, E: fmt::Display>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("invalid built-in relocation declaration: {error}"))
}

#[cfg(test)]
mod tests {
    use world_defs::{ArtifactValidator, SemanticInterfaceCatalog};

    use super::*;

    #[test]
    fn standard_relocation_family_is_distinct_and_artifact_valid() {
        let descriptor = relocation_interface_descriptor();
        let catalog = declared(SemanticInterfaceCatalog::new(vec![descriptor]));
        let artifact =
            declared(ArtifactValidator::new(&catalog).validate(relocation_artifact_data()));

        assert_eq!(artifact.actions().len(), 3);
        assert_ne!(start_relocation_action_key(), pause_relocation_action_key());
        assert_ne!(
            pause_relocation_action_key(),
            resume_relocation_action_key()
        );
    }
}
