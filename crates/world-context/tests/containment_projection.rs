use core::fmt;
use std::collections::BTreeSet;

use world_context::{
    ActionPolicySemanticsId, CandidateCoverage, ContainmentTransferActionDefinitions,
    ContainmentTransferActionDefinitionsError, ContainmentTransferProjector,
    GroundedActionCandidateId, GroundedActionInteraction,
};
use world_core::{ActorId, EntityId, WorldRevision};
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
    AcceptedState, ActionOpportunityId, AgencyState, ContainerAuthorityRecord, ContainerRecord,
    ContainmentInteractionScope, ContainmentRecord, ContainmentTransferDelta, DomainState,
    EpistemicState, EpistemicVersion, EvidenceDeliveryGeneration, EvidenceRecord, PhysicalEvent,
    SocialState, WorldSnapshot,
};

const POLICY_SEMANTICS: ActionPolicySemanticsId = ActionPolicySemanticsId::from_bytes([0x71; 32]);

fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("fixture must be valid: {error}"),
    }
}

fn actor(byte: u8) -> ActorId {
    ActorId::from_bytes([byte; 32])
}

fn entity(byte: u8) -> EntityId {
    EntityId::from_bytes([byte; 32])
}

fn checked_name<T, E: fmt::Display>(result: Result<T, E>) -> T {
    valid(result)
}

fn checked_definitions(binding_names: [&str; 4], action_names: &[&str]) -> RuntimeDefinitionSet {
    let interface_key = checked_name(SemanticInterfaceKey::parse("example.containment-transfer"));
    let operation_name = checked_name(OperationName::parse("apply-transfer"));
    let parameters = vec![
        OperationParameter::new(
            checked_name(ParameterName::parse("actor")),
            ValueKind::Actor,
        ),
        OperationParameter::new(
            checked_name(ParameterName::parse("destination")),
            ValueKind::Entity,
        ),
        OperationParameter::new(
            checked_name(ParameterName::parse("item")),
            ValueKind::Entity,
        ),
        OperationParameter::new(
            checked_name(ParameterName::parse("source")),
            ValueKind::Entity,
        ),
    ];
    let operation = checked_name(SemanticOperationDescriptor::new(
        operation_name.clone(),
        OperationKind::Effect,
        parameters,
    ));
    let descriptor = checked_name(SemanticInterfaceDescriptor::new(
        interface_key.clone(),
        checked_name(InterfaceVersion::new(1)),
        vec![operation],
    ));
    let bindings = binding_names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            ActionBindingData::new(
                checked_name(BindingName::parse(name)),
                if index == 0 {
                    ValueKind::Actor
                } else {
                    ValueKind::Entity
                },
            )
        })
        .collect::<Vec<_>>();
    let arguments: Vec<BindingName> = binding_names
        .into_iter()
        .map(|name| checked_name(BindingName::parse(name)))
        .collect();
    let event_name = checked_name(LocalDefinitionName::parse("item-moved"));
    let event_fields = ["actor", "destination", "item", "source"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            EventFieldData::new(
                checked_name(EventFieldName::parse(name)),
                if index == 0 {
                    ValueKind::Actor
                } else {
                    ValueKind::Entity
                },
            )
        })
        .collect();
    let pack_key = checked_name(PackKey::parse("example.pack"));
    let actions = action_names
        .iter()
        .map(|name| {
            ActionData::new(
                checked_name(LocalDefinitionName::parse(name)),
                bindings.clone(),
                Vec::new(),
                vec![EffectCallData::new(OperationCallData::new(
                    interface_key.clone(),
                    operation_name.clone(),
                    arguments.clone(),
                ))],
                vec![EventEmissionData::new(
                    DefinitionKey::new(pack_key.clone(), event_name.clone()),
                    ["actor", "destination", "item", "source"]
                        .into_iter()
                        .zip(binding_names)
                        .map(|(field, binding)| {
                            EventFieldBindingData::new(
                                checked_name(EventFieldName::parse(field)),
                                checked_name(BindingName::parse(binding)),
                            )
                        })
                        .collect(),
                )],
            )
        })
        .collect();
    let coordinate = PackCoordinate::new(pack_key, PackVersion::new(1, 0, 0));
    let manifest = PackManifestData::new(
        EngineProtocolVersion::new(1),
        coordinate.clone(),
        Vec::new(),
    );
    let data = ArtifactData::new(
        manifest,
        if action_names.is_empty() {
            Vec::new()
        } else {
            vec![descriptor.reference()]
        },
        actions,
        vec![EventData::new(event_name, event_fields)],
    );
    let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor]));
    let artifact = valid(ArtifactValidator::new(&catalog).validate(data));
    let selection = ExactPackageSelection::new(
        coordinate.clone(),
        vec![SelectedPackage::new(
            coordinate,
            SourceSnapshotId::from_bytes([0x51; 32]),
            Vec::new(),
        )],
    );
    let exact = valid(ExactPackSet::finalize(selection, vec![artifact]));
    valid(DefinitionLinker::link(exact))
}

fn transfer_definitions() -> RuntimeDefinitionSet {
    checked_definitions(["actor", "destination", "item", "source"], &["move-item"])
}

fn action_key(definitions: &RuntimeDefinitionSet, local_name: &str) -> DefinitionKey {
    DefinitionKey::new(
        definitions.root().pack_key().clone(),
        checked_name(LocalDefinitionName::parse(local_name)),
    )
}

fn activated_transfer_actions(
    definitions: &RuntimeDefinitionSet,
) -> ContainmentTransferActionDefinitions {
    let actions = definitions
        .artifacts()
        .iter()
        .flat_map(|artifact| {
            artifact.actions().iter().map(|action| {
                DefinitionKey::new(
                    artifact.coordinate().pack_key().clone(),
                    action.name().clone(),
                )
            })
        })
        .collect();
    valid(ContainmentTransferActionDefinitions::new(
        definitions,
        actions,
    ))
}

fn accepted(
    acting: ActorId,
    containers: Vec<ContainerRecord>,
    containment: Vec<ContainmentRecord>,
    controls_source: bool,
) -> AcceptedState {
    let beliefs = containment
        .iter()
        .map(|record| (record.item(), record.container()))
        .collect();
    accepted_with_beliefs(acting, containers, containment, controls_source, beliefs)
}

fn accepted_with_beliefs(
    acting: ActorId,
    containers: Vec<ContainerRecord>,
    containment: Vec<ContainmentRecord>,
    controls_source: bool,
    beliefs: Vec<(EntityId, EntityId)>,
) -> AcceptedState {
    let source = entity(0x30);
    let authority = if controls_source {
        vec![ContainerAuthorityRecord::new(acting, source)]
    } else {
        Vec::new()
    };
    AcceptedState::new(
        valid(DomainState::new(containers, containment, authority)),
        epistemic(acting, &beliefs),
        SocialState::empty(),
        AgencyState::empty(),
    )
}

fn epistemic(actor: ActorId, beliefs: &[(EntityId, EntityId)]) -> EpistemicState {
    if beliefs.is_empty() {
        return EpistemicState::empty();
    }
    let evidence = beliefs
        .iter()
        .enumerate()
        .map(|(index, (item, container))| {
            let source = if *container == entity(0xef) {
                entity(0xee)
            } else {
                entity(0xef)
            };
            transfer_evidence(
                actor,
                u64::try_from(index).unwrap_or_else(|_| panic!("fixture index must fit u64")) + 1,
                *item,
                source,
                *container,
            )
        })
        .collect();
    valid(EpistemicState::empty().assimilate(actor, EpistemicVersion::EMPTY, evidence))
}

fn transfer_evidence(
    observer: ActorId,
    generation: u64,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
) -> EvidenceRecord {
    let delta = valid(ContainmentTransferDelta::new(
        observer,
        item,
        source,
        destination,
    ));
    let PhysicalEvent::ItemTransferred(event) = PhysicalEvent::item_transferred(delta) else {
        unreachable!("containment transfer must produce item-transfer evidence")
    };
    EvidenceRecord::direct_item_transfer(
        observer,
        EvidenceDeliveryGeneration::new(generation)
            .unwrap_or_else(|| panic!("fixture evidence generation is nonzero")),
        event,
    )
}

fn scope(
    items: Vec<EntityId>,
    destinations: Vec<EntityId>,
    candidate_limit: u32,
) -> ContainmentInteractionScope {
    valid(ContainmentInteractionScope::new(
        entity(0x30),
        destinations,
        items,
        candidate_limit,
    ))
}

#[test]
fn projector_builds_complete_actor_safe_candidates_and_exact_private_resolutions() {
    let acting = actor(0x10);
    let source = entity(0x30);
    let destination_a = entity(0x40);
    let destination_b = entity(0x41);
    let item_a = entity(0x20);
    let item_b = entity(0x21);
    let state = accepted(
        acting,
        vec![
            ContainerRecord::new(destination_b, 4),
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination_a, 4),
        ],
        vec![
            ContainmentRecord::new(item_b, source),
            ContainmentRecord::new(item_a, source),
        ],
        true,
    );
    let snapshot = WorldSnapshot::new(WorldRevision::from_raw(7), state);
    let definitions = transfer_definitions();
    let actions = activated_transfer_actions(&definitions);
    let key = action_key(&definitions, "move-item");
    let opportunity = ActionOpportunityId::from_bytes([0x61; 32]);
    let interaction_scope = scope(vec![item_b, item_a], vec![destination_b, destination_a], 16);

    let build = valid(ContainmentTransferProjector::new(&actions).build(
        &snapshot,
        acting,
        opportunity,
        &interaction_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));
    let payload = build.payload();
    let candidates = payload.candidates();

    assert_eq!(payload.actor(), acting);
    assert_eq!(payload.opportunity(), opportunity);
    assert_eq!(candidates.coverage(), CandidateCoverage::Complete);
    assert_eq!(candidates.candidates().len(), 4);
    let interaction = payload
        .interaction()
        .containment()
        .unwrap_or_else(|| panic!("containment projector must return a containment view"));
    assert_eq!(interaction.destinations().len(), 2);
    assert_eq!(interaction.items().len(), 2);
    assert_eq!(build.resolution().len(), candidates.candidates().len());

    let expected_binding_names = ["actor", "destination", "item", "source"];
    let mut exact_bindings = BTreeSet::new();
    for candidate in candidates.candidates() {
        assert_eq!(candidate.opportunity(), opportunity);
        assert_eq!(candidate.action(), &key);
        assert_eq!(
            candidate.interaction(),
            GroundedActionInteraction::ContainmentTransfer
        );
        assert!(candidates.contains(candidate.id()));
        assert_eq!(
            candidate
                .bindings()
                .iter()
                .map(|binding| binding.name().as_str())
                .collect::<Vec<_>>(),
            expected_binding_names
        );

        let resolved = build
            .resolution()
            .resolve(candidate.id())
            .unwrap_or_else(|| panic!("supplied candidate must resolve"));
        let resolved = resolved
            .containment()
            .unwrap_or_else(|| panic!("containment candidate must resolve as containment"));
        assert_eq!(resolved.action(), &key);
        assert_eq!(resolved.actor(), acting);
        assert_eq!(resolved.source(), source);
        exact_bindings.insert((resolved.item(), resolved.destination()));
    }

    assert_eq!(
        exact_bindings,
        BTreeSet::from([
            (item_a, destination_a),
            (item_a, destination_b),
            (item_b, destination_a),
            (item_b, destination_b),
        ])
    );
}

#[test]
fn projector_excludes_source_items_outside_the_exact_scope() {
    let acting = actor(0x10);
    let source = entity(0x30);
    let destination = entity(0x40);
    let allowed_item = entity(0x20);
    let excluded_item = entity(0x21);
    let state = accepted(
        acting,
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 4),
        ],
        vec![
            ContainmentRecord::new(allowed_item, source),
            ContainmentRecord::new(excluded_item, source),
        ],
        true,
    );
    let definitions = transfer_definitions();
    let actions = activated_transfer_actions(&definitions);
    let build = valid(ContainmentTransferProjector::new(&actions).build(
        &WorldSnapshot::new(WorldRevision::ROOT, state),
        acting,
        ActionOpportunityId::from_bytes([0x61; 32]),
        &scope(vec![allowed_item], vec![destination], 4),
        &definitions,
        POLICY_SEMANTICS,
    ));

    assert_eq!(build.payload().candidates().candidates().len(), 1);
    assert_eq!(
        build
            .payload()
            .interaction()
            .containment()
            .unwrap_or_else(|| panic!("containment projector must return a containment view"))
            .items()
            .len(),
        1
    );
    for candidate in build.payload().candidates().candidates() {
        let resolved = build
            .resolution()
            .resolve(candidate.id())
            .unwrap_or_else(|| panic!("supplied candidate must resolve"));
        let resolved = resolved
            .containment()
            .unwrap_or_else(|| panic!("containment candidate must resolve as containment"));
        assert_eq!(resolved.item(), allowed_item);
        assert_ne!(resolved.item(), excluded_item);
    }
}

#[test]
fn complete_empty_is_a_successful_projection() {
    let acting = actor(0x10);
    let source = entity(0x30);
    let destination = entity(0x40);
    let state = accepted(
        acting,
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 4),
        ],
        Vec::new(),
        true,
    );
    let snapshot = WorldSnapshot::new(WorldRevision::ROOT, state);
    let definitions = transfer_definitions();
    let actions = activated_transfer_actions(&definitions);
    let build = valid(ContainmentTransferProjector::new(&actions).build(
        &snapshot,
        acting,
        ActionOpportunityId::from_bytes([0x61; 32]),
        &scope(vec![entity(0x20)], vec![destination], 4),
        &definitions,
        POLICY_SEMANTICS,
    ));

    assert_eq!(
        build.payload().candidates().coverage(),
        CandidateCoverage::Complete
    );
    assert!(build.payload().candidates().candidates().is_empty());
    assert!(
        build
            .payload()
            .interaction()
            .containment()
            .unwrap_or_else(|| panic!("containment projector must return a containment view"))
            .items()
            .is_empty()
    );
    assert!(build.resolution().is_empty());
}

#[test]
fn hidden_source_authority_does_not_gate_actor_belief_grounding() {
    let acting = actor(0x10);
    let source = entity(0x30);
    let destination = entity(0x40);
    let item = entity(0x20);
    let without_authority = accepted(
        acting,
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 4),
        ],
        vec![ContainmentRecord::new(item, source)],
        false,
    );
    let with_authority = accepted(
        acting,
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 4),
        ],
        vec![ContainmentRecord::new(item, source)],
        true,
    );
    let definitions = transfer_definitions();
    let actions = activated_transfer_actions(&definitions);
    let opportunity = ActionOpportunityId::from_bytes([0x61; 32]);
    let interaction_scope = scope(vec![item], vec![destination], 4);
    let without_authority = valid(ContainmentTransferProjector::new(&actions).build(
        &WorldSnapshot::new(WorldRevision::ROOT, without_authority),
        acting,
        opportunity,
        &interaction_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));
    let with_authority = valid(ContainmentTransferProjector::new(&actions).build(
        &WorldSnapshot::new(WorldRevision::from_raw(900), with_authority),
        acting,
        opportunity,
        &interaction_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));

    assert_eq!(without_authority.payload(), with_authority.payload());
    assert_eq!(without_authority.resolution(), with_authority.resolution());
    assert_eq!(
        without_authority.read_witness().projection(),
        with_authority.read_witness().projection()
    );
    assert_ne!(
        without_authority.read_witness().execution(),
        with_authority.read_witness().execution()
    );
    assert_eq!(
        without_authority.payload().candidates().candidates().len(),
        1
    );
}

#[test]
fn hidden_destination_state_does_not_change_projected_action_context() {
    let acting = actor(0x10);
    let source = entity(0x30);
    let destination = entity(0x40);
    let visible_item = entity(0x20);
    let hidden_item = entity(0x2f);
    let containers = vec![
        ContainerRecord::new(source, 4),
        ContainerRecord::new(destination, 1),
    ];
    let open = accepted(
        acting,
        containers.clone(),
        vec![ContainmentRecord::new(visible_item, source)],
        true,
    );
    let full = accepted(
        acting,
        containers,
        vec![
            ContainmentRecord::new(visible_item, source),
            ContainmentRecord::new(hidden_item, destination),
        ],
        true,
    );
    let definitions = transfer_definitions();
    let actions = activated_transfer_actions(&definitions);
    let opportunity = ActionOpportunityId::from_bytes([0x61; 32]);
    let interaction_scope = scope(vec![visible_item], vec![destination], 4);
    let open_build = valid(ContainmentTransferProjector::new(&actions).build(
        &WorldSnapshot::new(WorldRevision::from_raw(3), open),
        acting,
        opportunity,
        &interaction_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));
    let full_build = valid(ContainmentTransferProjector::new(&actions).build(
        &WorldSnapshot::new(WorldRevision::from_raw(900), full),
        acting,
        opportunity,
        &interaction_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));

    assert_eq!(open_build.payload(), full_build.payload());
    assert_eq!(
        open_build.payload().input_fingerprint(),
        full_build.payload().input_fingerprint()
    );
    assert_eq!(open_build.resolution(), full_build.resolution());
    assert_eq!(
        open_build.read_witness().projection(),
        full_build.read_witness().projection()
    );
    assert_ne!(
        open_build.read_witness().execution(),
        full_build.read_witness().execution()
    );
    assert_eq!(
        open_build.payload().candidates().candidates()[0].id(),
        full_build.payload().candidates().candidates()[0].id()
    );
}

#[test]
fn hidden_authoritative_item_location_does_not_change_belief_grounded_payload() {
    let acting = actor(0x10);
    let source = entity(0x30);
    let destination = entity(0x40);
    let item = entity(0x20);
    let containers = vec![
        ContainerRecord::new(source, 4),
        ContainerRecord::new(destination, 4),
    ];
    let believed_source = vec![(item, source)];
    let actually_present = accepted_with_beliefs(
        acting,
        containers.clone(),
        vec![ContainmentRecord::new(item, source)],
        true,
        believed_source.clone(),
    );
    let actually_elsewhere = accepted_with_beliefs(
        acting,
        containers,
        vec![ContainmentRecord::new(item, destination)],
        true,
        believed_source,
    );
    let definitions = transfer_definitions();
    let actions = activated_transfer_actions(&definitions);
    let opportunity = ActionOpportunityId::from_bytes([0x61; 32]);
    let interaction_scope = scope(vec![item], vec![destination], 4);

    let present = valid(ContainmentTransferProjector::new(&actions).build(
        &WorldSnapshot::new(WorldRevision::from_raw(3), actually_present),
        acting,
        opportunity,
        &interaction_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));
    let elsewhere = valid(ContainmentTransferProjector::new(&actions).build(
        &WorldSnapshot::new(WorldRevision::from_raw(900), actually_elsewhere),
        acting,
        opportunity,
        &interaction_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));

    assert_eq!(present.payload(), elsewhere.payload());
    assert_eq!(present.resolution(), elsewhere.resolution());
    assert_eq!(
        present.read_witness().projection(),
        elsewhere.read_witness().projection()
    );
    assert_ne!(
        present.read_witness().execution(),
        elsewhere.read_witness().execution()
    );
    assert_eq!(present.payload().candidates().candidates().len(), 1);
}

#[test]
fn changed_evidence_provenance_does_not_change_projection_witness() {
    let acting = actor(0x10);
    let source = entity(0x30);
    let destination = entity(0x40);
    let item = entity(0x20);
    let domain = valid(DomainState::new(
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 4),
        ],
        vec![ContainmentRecord::new(item, source)],
        vec![ContainerAuthorityRecord::new(acting, source)],
    ));
    let first_evidence = transfer_evidence(acting, 1, item, entity(0x50), source);
    let later_evidence = transfer_evidence(acting, 2, item, entity(0x51), source);
    let first_epistemic = valid(EpistemicState::empty().assimilate(
        acting,
        EpistemicVersion::EMPTY,
        vec![first_evidence],
    ));
    let later_epistemic = valid(EpistemicState::empty().assimilate(
        acting,
        EpistemicVersion::EMPTY,
        vec![first_evidence, later_evidence],
    ));
    let first_state = AcceptedState::new(
        domain.clone(),
        first_epistemic,
        SocialState::empty(),
        AgencyState::empty(),
    );
    let later_state = AcceptedState::new(
        domain,
        later_epistemic,
        SocialState::empty(),
        AgencyState::empty(),
    );
    let definitions = transfer_definitions();
    let actions = activated_transfer_actions(&definitions);
    let opportunity = ActionOpportunityId::from_bytes([0x61; 32]);
    let interaction_scope = scope(vec![item], vec![destination], 4);

    let first = valid(ContainmentTransferProjector::new(&actions).build(
        &WorldSnapshot::new(WorldRevision::from_raw(3), first_state),
        acting,
        opportunity,
        &interaction_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));
    let later = valid(ContainmentTransferProjector::new(&actions).build(
        &WorldSnapshot::new(WorldRevision::from_raw(900), later_state),
        acting,
        opportunity,
        &interaction_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));

    assert_eq!(first.payload(), later.payload());
    assert_eq!(first.resolution(), later.resolution());
    assert_eq!(
        first.read_witness().projection(),
        later.read_witness().projection()
    );
    assert_eq!(
        first.read_witness().execution(),
        later.read_witness().execution()
    );
}

#[test]
fn candidate_budget_is_explicit_and_canonical() {
    let acting = actor(0x10);
    let source = entity(0x30);
    let destinations = vec![entity(0x41), entity(0x40)];
    let state = accepted(
        acting,
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destinations[0], 4),
            ContainerRecord::new(destinations[1], 4),
        ],
        vec![
            ContainmentRecord::new(entity(0x21), source),
            ContainmentRecord::new(entity(0x20), source),
        ],
        true,
    );
    let definitions = transfer_definitions();
    let actions = activated_transfer_actions(&definitions);
    let opportunity = ActionOpportunityId::from_bytes([0x61; 32]);
    let snapshot = WorldSnapshot::new(WorldRevision::ROOT, state);
    let items = vec![entity(0x21), entity(0x20)];
    let canonical_scope = scope(items.clone(), destinations.clone(), 2);
    let reversed_scope = scope(
        items.into_iter().rev().collect(),
        destinations.into_iter().rev().collect(),
        2,
    );

    let canonical = valid(ContainmentTransferProjector::new(&actions).build(
        &snapshot,
        acting,
        opportunity,
        &canonical_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));
    let reversed = valid(ContainmentTransferProjector::new(&actions).build(
        &snapshot,
        acting,
        opportunity,
        &reversed_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));

    assert_eq!(
        canonical.payload().candidates().coverage(),
        CandidateCoverage::BudgetLimited
    );
    assert_eq!(canonical.payload().candidates().candidates().len(), 2);
    assert_eq!(canonical.payload(), reversed.payload());
    assert_eq!(canonical.resolution(), reversed.resolution());
}

#[test]
fn multiple_checked_actions_share_one_budget_in_definition_key_order() {
    let acting = actor(0x10);
    let source = entity(0x30);
    let destination = entity(0x40);
    let state = accepted(
        acting,
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 4),
        ],
        vec![ContainmentRecord::new(entity(0x20), source)],
        true,
    );
    let definitions = checked_definitions(
        ["actor", "destination", "item", "source"],
        &["z-transfer", "a-transfer"],
    );
    let actions = activated_transfer_actions(&definitions);
    let snapshot = WorldSnapshot::new(WorldRevision::ROOT, state);
    let opportunity = ActionOpportunityId::from_bytes([0x61; 32]);

    let complete = valid(ContainmentTransferProjector::new(&actions).build(
        &snapshot,
        acting,
        opportunity,
        &scope(vec![entity(0x20)], vec![destination], 2),
        &definitions,
        POLICY_SEMANTICS,
    ));
    assert_eq!(
        complete
            .payload()
            .candidates()
            .candidates()
            .iter()
            .map(|candidate| candidate.action().local_name().as_str())
            .collect::<Vec<_>>(),
        ["a-transfer", "z-transfer"]
    );
    for candidate in complete.payload().candidates().candidates() {
        let resolved = complete
            .resolution()
            .resolve(candidate.id())
            .unwrap_or_else(|| panic!("every supplied action candidate must resolve"));
        let resolved = resolved
            .containment()
            .unwrap_or_else(|| panic!("containment candidate must resolve as containment"));
        assert_eq!(resolved.action(), candidate.action());
    }

    let limited = valid(ContainmentTransferProjector::new(&actions).build(
        &snapshot,
        acting,
        opportunity,
        &scope(vec![entity(0x20)], vec![destination], 1),
        &definitions,
        POLICY_SEMANTICS,
    ));
    assert_eq!(
        limited.payload().candidates().coverage(),
        CandidateCoverage::BudgetLimited
    );
    assert_eq!(limited.payload().candidates().candidates().len(), 1);
    assert_eq!(
        limited.payload().candidates().candidates()[0]
            .action()
            .local_name()
            .as_str(),
        "a-transfer"
    );

    let exact_family = valid(ContainmentTransferActionDefinitions::new(
        &definitions,
        vec![action_key(&definitions, "z-transfer")],
    ));
    let exact = valid(ContainmentTransferProjector::new(&exact_family).build(
        &snapshot,
        acting,
        opportunity,
        &scope(vec![entity(0x20)], vec![destination], 2),
        &definitions,
        POLICY_SEMANTICS,
    ));
    assert_eq!(exact.payload().candidates().candidates().len(), 1);
    assert_eq!(
        exact.payload().candidates().candidates()[0]
            .action()
            .local_name()
            .as_str(),
        "z-transfer",
        "projection must use only the exact activated family, not scan definitions"
    );
}

#[test]
fn fabricated_candidate_ids_have_no_private_resolution() {
    let acting = actor(0x10);
    let source = entity(0x30);
    let destination = entity(0x40);
    let state = accepted(
        acting,
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 4),
        ],
        vec![ContainmentRecord::new(entity(0x20), source)],
        true,
    );
    let definitions = transfer_definitions();
    let actions = activated_transfer_actions(&definitions);
    let build = valid(ContainmentTransferProjector::new(&actions).build(
        &WorldSnapshot::new(WorldRevision::ROOT, state),
        acting,
        ActionOpportunityId::from_bytes([0x61; 32]),
        &scope(vec![entity(0x20)], vec![destination], 4),
        &definitions,
        POLICY_SEMANTICS,
    ));
    let fabricated = GroundedActionCandidateId::from_bytes([0xff; 32]);

    assert!(!build.payload().candidates().contains(fabricated));
    assert_eq!(build.resolution().resolve(fabricated), None);
}

#[test]
fn action_family_rejects_an_action_without_the_exact_typed_role_contract() {
    let definitions = checked_definitions(["actor", "target", "item", "source"], &["move-item"]);
    let action = action_key(&definitions, "move-item");
    let result = ContainmentTransferActionDefinitions::new(&definitions, vec![action.clone()]);

    assert_eq!(
        result,
        Err(ContainmentTransferActionDefinitionsError::BindingShapeMismatch { action })
    );
}

#[test]
fn action_family_rejects_an_activated_set_without_a_transfer_action() {
    let definitions = checked_definitions(["actor", "destination", "item", "source"], &[]);
    let result = ContainmentTransferActionDefinitions::new(&definitions, Vec::new());

    assert_eq!(
        result,
        Err(ContainmentTransferActionDefinitionsError::NoActions)
    );
}

#[test]
fn fingerprints_bind_policy_semantics_without_changing_candidates() {
    let acting = actor(0x10);
    let source = entity(0x30);
    let destination = entity(0x40);
    let state = accepted(
        acting,
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 4),
        ],
        vec![ContainmentRecord::new(entity(0x20), source)],
        true,
    );
    let snapshot = WorldSnapshot::new(WorldRevision::ROOT, state);
    let definitions = transfer_definitions();
    let actions = activated_transfer_actions(&definitions);
    let opportunity = ActionOpportunityId::from_bytes([0x61; 32]);
    let interaction_scope = scope(vec![entity(0x20)], vec![destination], 4);
    let first = valid(ContainmentTransferProjector::new(&actions).build(
        &snapshot,
        acting,
        opportunity,
        &interaction_scope,
        &definitions,
        POLICY_SEMANTICS,
    ));
    let second = valid(ContainmentTransferProjector::new(&actions).build(
        &snapshot,
        acting,
        opportunity,
        &interaction_scope,
        &definitions,
        ActionPolicySemanticsId::from_bytes([0x72; 32]),
    ));

    assert_eq!(first.payload().candidates(), second.payload().candidates());
    assert_ne!(
        first.payload().input_fingerprint(),
        second.payload().input_fingerprint()
    );
}
