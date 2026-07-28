use core::fmt;

use world_core::{ActorId, EntityId};
use world_defs::{
    ActionBindingData, ActionData, ArtifactData, ArtifactValidator, BindingName, DefinitionKey,
    DefinitionLinker, EffectCallData, EngineProtocolVersion, EventData, EventEmissionData,
    EventFieldBindingData, EventFieldData, EventFieldName, ExactPackSet, ExactPackageSelection,
    InterfaceVersion, LocalDefinitionName, OperationCallData, OperationKind, OperationName,
    OperationParameter, PackCoordinate, PackKey, PackManifestData, PackVersion, ParameterName,
    RuntimeDefinitionSet, RuntimeRequirementData, SelectedPackage, SemanticInterfaceCatalog,
    SemanticInterfaceDescriptor, SemanticInterfaceKey, SemanticOperationDescriptor,
    SourceSnapshotId, ValueKind,
};
use world_model::{
    CommandAttemptOutcome, CommandBinding, CommandEnvelope, CommandEnvelopeError, CommandId,
    CommandSource, CommandValue, StableCommandRejection,
};

fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("command fixture must be valid: {error}"),
    }
}

fn binding(value: &str) -> BindingName {
    valid(BindingName::parse(value))
}

fn fixture(version: PackVersion) -> (RuntimeDefinitionSet, DefinitionKey) {
    let (definitions, action, _) = fixture_with_optional_second_action(version, false);
    (definitions, action)
}

fn fixture_with_optional_second_action(
    version: PackVersion,
    include_second_action: bool,
) -> (RuntimeDefinitionSet, DefinitionKey, Option<DefinitionKey>) {
    let pack = valid(PackKey::parse("test.commands"));
    let coordinate = PackCoordinate::new(pack.clone(), version);
    let interface_key = valid(SemanticInterfaceKey::parse("test.containment"));
    let actor_parameter =
        OperationParameter::new(valid(ParameterName::parse("actor")), ValueKind::Actor);
    let item_parameter =
        OperationParameter::new(valid(ParameterName::parse("item")), ValueKind::Entity);
    let requirement_name = valid(OperationName::parse("can-move"));
    let effect_name = valid(OperationName::parse("move"));
    let descriptor = valid(SemanticInterfaceDescriptor::new(
        interface_key.clone(),
        valid(InterfaceVersion::new(1)),
        vec![
            valid(SemanticOperationDescriptor::new(
                requirement_name.clone(),
                OperationKind::Predicate,
                vec![actor_parameter.clone(), item_parameter.clone()],
            )),
            valid(SemanticOperationDescriptor::new(
                effect_name.clone(),
                OperationKind::Effect,
                vec![actor_parameter, item_parameter],
            )),
        ],
    ));
    let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor.clone()]));

    let actor_name = binding("actor");
    let item_name = binding("item");
    let arguments = vec![actor_name.clone(), item_name.clone()];
    let event_name = valid(LocalDefinitionName::parse("item-moved"));
    let event = EventData::new(
        event_name.clone(),
        vec![
            EventFieldData::new(valid(EventFieldName::parse("actor")), ValueKind::Actor),
            EventFieldData::new(valid(EventFieldName::parse("item")), ValueKind::Entity),
        ],
    );
    let event_key = DefinitionKey::new(pack.clone(), event_name);
    let make_action = |name: &str| {
        let action_name = valid(LocalDefinitionName::parse(name));
        let action_key = DefinitionKey::new(pack.clone(), action_name.clone());
        let action = ActionData::new(
            action_name,
            vec![
                ActionBindingData::new(actor_name.clone(), ValueKind::Actor),
                ActionBindingData::new(item_name.clone(), ValueKind::Entity),
            ],
            vec![RuntimeRequirementData::new(OperationCallData::new(
                interface_key.clone(),
                requirement_name.clone(),
                arguments.clone(),
            ))],
            vec![EffectCallData::new(OperationCallData::new(
                interface_key.clone(),
                effect_name.clone(),
                arguments.clone(),
            ))],
            vec![EventEmissionData::new(
                event_key.clone(),
                vec![
                    EventFieldBindingData::new(
                        valid(EventFieldName::parse("actor")),
                        actor_name.clone(),
                    ),
                    EventFieldBindingData::new(
                        valid(EventFieldName::parse("item")),
                        item_name.clone(),
                    ),
                ],
            )],
        );
        (action_key, action)
    };
    let (action_key, action) = make_action("move-item");
    let second = include_second_action.then(|| make_action("move-other"));
    let mut actions = vec![action];
    if let Some((_, action)) = &second {
        actions.push(action.clone());
    }
    let artifact = valid(ArtifactValidator::new(&catalog).validate(ArtifactData::new(
        PackManifestData::new(
            EngineProtocolVersion::new(1),
            coordinate.clone(),
            Vec::new(),
        ),
        vec![descriptor.reference()],
        actions,
        vec![event],
    )));
    let selection = ExactPackageSelection::new(
        coordinate.clone(),
        vec![SelectedPackage::new(
            coordinate,
            SourceSnapshotId::from_bytes([0x77; 32]),
            Vec::new(),
        )],
    );
    let exact = valid(ExactPackSet::finalize(selection, vec![artifact]));
    (
        valid(DefinitionLinker::link(exact)),
        action_key,
        second.map(|(key, _)| key),
    )
}

fn actor(byte: u8) -> ActorId {
    ActorId::from_bytes([byte; 32])
}

fn entity(byte: u8) -> EntityId {
    EntityId::from_bytes([byte; 32])
}

fn bindings(actor_id: ActorId, item: EntityId) -> Vec<CommandBinding> {
    vec![
        CommandBinding::new(binding("item"), CommandValue::Entity(item)),
        CommandBinding::new(binding("actor"), CommandValue::Actor(actor_id)),
    ]
}

#[test]
fn envelope_resolves_and_canonicalizes_exact_definition_bindings() {
    let (definitions, action) = fixture(PackVersion::new(1, 0, 0));
    let source = CommandSource::from_bytes([0x31; 32]);
    let actor_id = actor(0x41);
    let item = entity(0x51);
    let first = valid(CommandEnvelope::new(
        &definitions,
        source,
        CommandId::new(7),
        actor_id,
        action.clone(),
        bindings(actor_id, item),
    ));
    let mut reversed = bindings(actor_id, item);
    reversed.reverse();
    let retry_identity = valid(CommandEnvelope::new(
        &definitions,
        source,
        CommandId::new(8),
        actor_id,
        action,
        reversed,
    ));

    assert_eq!(first.bindings()[0].name().as_str(), "actor");
    assert_eq!(first.bindings()[1].name().as_str(), "item");
    assert_eq!(first.source(), source);
    assert_eq!(first.id(), CommandId::new(7));
    assert_eq!(first.actor(), actor_id);
    assert_eq!(first.definition_set_digest(), definitions.digest());
    assert_eq!(first.fingerprint(), retry_identity.fingerprint());
    assert_eq!(
        first.fingerprint().to_string(),
        "5d4e27e64ea4af1a7da8b3d5f0cb5abd2b0ca4d9227abd7f10dbb7e3ccdf659b"
    );
}

#[test]
fn request_fingerprint_covers_effect_fields_but_omits_command_id() {
    let (definitions, action) = fixture(PackVersion::new(1, 0, 0));
    let (changed_definitions, changed_action) = fixture(PackVersion::new(2, 0, 0));
    let actor_id = actor(0x41);
    let item = entity(0x51);
    let make = |definitions: &RuntimeDefinitionSet,
                source: CommandSource,
                id: u64,
                actor_id: ActorId,
                action: DefinitionKey,
                bindings: Vec<CommandBinding>| {
        valid(CommandEnvelope::new(
            definitions,
            source,
            CommandId::new(id),
            actor_id,
            action,
            bindings,
        ))
    };
    let original = make(
        &definitions,
        CommandSource::from_bytes([0x31; 32]),
        7,
        actor_id,
        action.clone(),
        bindings(actor_id, item),
    );
    let changed_id = make(
        &definitions,
        CommandSource::from_bytes([0x31; 32]),
        99,
        actor_id,
        action.clone(),
        bindings(actor_id, item),
    );
    let changed_source = make(
        &definitions,
        CommandSource::from_bytes([0x32; 32]),
        7,
        actor_id,
        action.clone(),
        bindings(actor_id, item),
    );
    let changed_actor = make(
        &definitions,
        CommandSource::from_bytes([0x31; 32]),
        7,
        actor(0x42),
        action.clone(),
        bindings(actor_id, item),
    );
    let changed_item = make(
        &definitions,
        CommandSource::from_bytes([0x31; 32]),
        7,
        actor_id,
        action,
        bindings(actor_id, entity(0x52)),
    );
    let changed_set = make(
        &changed_definitions,
        CommandSource::from_bytes([0x31; 32]),
        7,
        actor_id,
        changed_action,
        bindings(actor_id, item),
    );

    assert_eq!(original.fingerprint(), changed_id.fingerprint());
    assert_ne!(original.fingerprint(), changed_source.fingerprint());
    assert_ne!(original.fingerprint(), changed_actor.fingerprint());
    assert_ne!(original.fingerprint(), changed_item.fingerprint());
    assert_ne!(original.fingerprint(), changed_set.fingerprint());

    let (same_set, first_action, second_action) =
        fixture_with_optional_second_action(PackVersion::new(1, 0, 0), true);
    let second_action = second_action
        .unwrap_or_else(|| panic!("two-action fixture must contain its second action"));
    let first_selection = make(
        &same_set,
        CommandSource::from_bytes([0x31; 32]),
        7,
        actor_id,
        first_action,
        bindings(actor_id, item),
    );
    let second_selection = make(
        &same_set,
        CommandSource::from_bytes([0x31; 32]),
        7,
        actor_id,
        second_action,
        bindings(actor_id, item),
    );
    assert_eq!(
        first_selection.definition_set_digest(),
        second_selection.definition_set_digest()
    );
    assert_ne!(
        first_selection.fingerprint(),
        second_selection.fingerprint()
    );
}

#[test]
fn envelope_reports_definition_and_binding_contract_failures() {
    let (definitions, action) = fixture(PackVersion::new(1, 0, 0));
    let source = CommandSource::from_bytes([0x31; 32]);
    let actor_id = actor(0x41);
    let item = entity(0x51);
    let missing_action = DefinitionKey::new(
        valid(PackKey::parse("test.commands")),
        valid(LocalDefinitionName::parse("missing")),
    );
    assert_eq!(
        CommandEnvelope::new(
            &definitions,
            source,
            CommandId::new(1),
            actor_id,
            missing_action.clone(),
            bindings(actor_id, item),
        ),
        Err(CommandEnvelopeError::DefinitionUnavailable {
            action: missing_action,
        })
    );

    let actor_binding = CommandBinding::new(binding("actor"), CommandValue::Actor(actor_id));
    assert_eq!(
        CommandEnvelope::new(
            &definitions,
            source,
            CommandId::new(1),
            actor_id,
            action.clone(),
            vec![actor_binding.clone(), actor_binding],
        ),
        Err(CommandEnvelopeError::DuplicateBinding {
            binding: binding("actor"),
        })
    );
    assert_eq!(
        CommandEnvelope::new(
            &definitions,
            source,
            CommandId::new(1),
            actor_id,
            action.clone(),
            vec![CommandBinding::new(
                binding("actor"),
                CommandValue::Actor(actor_id),
            )],
        ),
        Err(CommandEnvelopeError::MissingBinding {
            binding: binding("item"),
        })
    );
    assert_eq!(
        CommandEnvelope::new(
            &definitions,
            source,
            CommandId::new(1),
            actor_id,
            action.clone(),
            vec![
                CommandBinding::new(binding("actor"), CommandValue::Actor(actor_id)),
                CommandBinding::new(binding("item"), CommandValue::Entity(item)),
                CommandBinding::new(binding("other"), CommandValue::Entity(item)),
            ],
        ),
        Err(CommandEnvelopeError::UnexpectedBinding {
            binding: binding("other"),
        })
    );
    assert_eq!(
        CommandEnvelope::new(
            &definitions,
            source,
            CommandId::new(1),
            actor_id,
            action,
            vec![
                CommandBinding::new(binding("actor"), CommandValue::Entity(entity(0x41))),
                CommandBinding::new(binding("item"), CommandValue::Entity(item)),
            ],
        ),
        Err(CommandEnvelopeError::BindingKindMismatch {
            binding: binding("actor"),
            expected: ValueKind::Actor,
            actual: ValueKind::Entity,
        })
    );
}

#[test]
fn attempt_outcome_and_stable_rejection_variants_are_closed_and_distinct() {
    let reasons = [
        StableCommandRejection::DefinitionUnavailable,
        StableCommandRejection::BindingMismatch,
        StableCommandRejection::Stale,
        StableCommandRejection::RequirementUnsatisfied,
        StableCommandRejection::Conflict,
        StableCommandRejection::IdCollision,
    ];

    assert_ne!(
        CommandAttemptOutcome::Accepted,
        CommandAttemptOutcome::Rejected(StableCommandRejection::RequirementUnsatisfied)
    );
    for (index, reason) in reasons.iter().enumerate() {
        assert!(!reasons[..index].contains(reason));
    }
}
