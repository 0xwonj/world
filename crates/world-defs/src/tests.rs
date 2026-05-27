use super::*;
use world_core::{DefinitionId, ReplayLevel, VersionAnchor};

fn id(value: u64) -> DefinitionId {
    let Some(id) = DefinitionId::new(value) else {
        panic!("test ids must be nonzero");
    };
    id
}

fn version(value: u64) -> VersionAnchor {
    let Some(version) = VersionAnchor::new(value) else {
        panic!("test versions must be nonzero");
    };
    version
}

fn role(name: &'static str) -> RoleDef {
    RoleDef::new(RoleName::from_static(name), RoleType::from_static("entity"))
}

fn event(kind: &'static str, roles: impl IntoIterator<Item = &'static str>) -> EventRecordSpec {
    event_with_version(kind, roles, 1)
}

fn event_with_version(
    kind: &'static str,
    roles: impl IntoIterator<Item = &'static str>,
    version_value: u64,
) -> EventRecordSpec {
    let Ok(event) = EventRecordSpec::new(
        EventKind::from_static(kind),
        roles.into_iter().map(RoleName::from_static),
        version(version_value),
    ) else {
        panic!("test events must declare roles");
    };
    event
}

fn state_schema() -> ProcessStateSchema {
    let Ok(schema) = ProcessStateSchema::new([ProcessStateField::new(
        StateFieldName::from_static("progress"),
        StateValueType::from_static("u32"),
    )]) else {
        panic!("test process state schemas must have fields");
    };
    schema
}

fn empty_state_schema() -> ProcessStateSchema {
    let Ok(schema) = ProcessStateSchema::new([]) else {
        panic!("empty process state schemas are valid");
    };
    schema
}

fn policies() -> ProcessPolicies {
    ProcessPolicies::new(
        PolicyKey::from_static("tick"),
        PolicyKey::from_static("wait"),
        PolicyKey::from_static("interrupt"),
        PolicyKey::from_static("resume"),
        PolicyKey::from_static("failure"),
    )
}

fn support(tier: ResolutionTier, effect_program: DefinitionId) -> ResolutionSupport {
    let name = match tier {
        ResolutionTier::Concrete => "concrete_lowering",
        ResolutionTier::Abstract => "abstract_lowering",
        ResolutionTier::Strategic => "strategic_lowering",
    };
    let Ok(support) = ResolutionSupport::new(tier, PolicyKey::from_static(name), [effect_program])
    else {
        panic!("test resolution support must reference effect programs");
    };
    support
}

fn op(
    kind: &'static str,
    permissions: impl IntoIterator<Item = StagePermission>,
    emitted_events: impl IntoIterator<Item = EventRecordSpec>,
) -> EffectOp {
    let Ok(operation) = EffectOp::new(EffectKind::from_static(kind), permissions, emitted_events)
    else {
        panic!("test operations declare permissions");
    };
    operation
}

fn transfer_program() -> EffectProgramDef {
    let Ok(program) = EffectProgramDef::new(
        id(1),
        DefinitionName::from_static("transfer"),
        [op(
            "transfer_entity",
            [
                StagePermission::ReadWorld,
                StagePermission::MutatePhysical,
                StagePermission::EmitPhysicalEventRecord,
            ],
            [event("EntityTransferred", ["actor", "item", "destination"])],
        )],
        EventContract::new([event("EntityTransferred", ["actor", "item", "destination"])]),
        ReplayLevel::EventRebuild,
        version(1),
    ) else {
        panic!("test program should be valid");
    };
    program
}

fn action(effect_program: DefinitionId) -> ActionDef {
    let Ok(action) = ActionDef::new(
        id(2),
        DefinitionName::from_static("move_item"),
        [role("actor"), role("item"), role("destination")],
        [RequirementDef::new(
            RequirementKind::from_static("reachable"),
            [
                RoleName::from_static("actor"),
                RoleName::from_static("item"),
            ],
        )],
        [BindingRuleDef::new(
            BindingRuleKind::from_static("holds"),
            [
                RoleName::from_static("actor"),
                RoleName::from_static("item"),
            ],
        )],
        effect_program,
        EventContract::new([event("EntityTransferred", ["actor", "item", "destination"])]),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(2),
    ) else {
        panic!("test action should be valid");
    };
    action
}

fn process(effect_program: DefinitionId) -> ProcessDef {
    let Ok(process) = ProcessDef::new(
        id(3),
        DefinitionName::from_static("haul_supplies"),
        [role("actor"), role("item"), role("destination")],
        state_schema(),
        [
            support(ResolutionTier::Concrete, effect_program),
            support(ResolutionTier::Abstract, effect_program),
        ],
        policies(),
        EventContract::new([event("EntityTransferred", ["actor", "item", "destination"])]),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(3),
    ) else {
        panic!("test process should be valid");
    };
    process
}

#[test]
fn string_keys_reject_blank_values() {
    assert_eq!(
        RoleName::new(" actor ").map(|name| name.to_string()),
        Some("actor".to_owned())
    );
    assert_eq!(DefinitionName::new(" "), None);
}

#[test]
fn action_defs_reject_duplicate_roles_and_unknown_role_refs() {
    let Err(error) = ActionDef::new(
        id(10),
        DefinitionName::from_static("bad_roles"),
        [role("actor"), role("actor")],
        [],
        [],
        id(1),
        EventContract::default(),
        [StagePermission::ReadWorld],
        version(1),
    ) else {
        panic!("duplicate roles must be rejected");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateRole {
            definition: id(10),
            role: RoleName::from_static("actor"),
        }
    );

    let Err(error) = ActionDef::new(
        id(11),
        DefinitionName::from_static("bad_requirement"),
        [role("actor")],
        [RequirementDef::new(
            RequirementKind::from_static("owns"),
            [RoleName::from_static("item")],
        )],
        [],
        id(1),
        EventContract::default(),
        [StagePermission::ReadWorld],
        version(1),
    ) else {
        panic!("unknown role references must be rejected");
    };
    assert_eq!(
        error,
        DefinitionError::UnknownRole {
            definition: id(11),
            role: RoleName::from_static("item"),
        }
    );
}

#[test]
fn effect_programs_expose_permissions_and_event_contracts() {
    let program = transfer_program();

    assert!(
        program
            .required_permissions()
            .contains(&StagePermission::MutatePhysical)
    );
    assert!(program.emitted_events().contains(&event(
        "EntityTransferred",
        ["actor", "item", "destination"],
    )));

    let Err(error) = EffectProgramDef::new(
        id(12),
        DefinitionName::from_static("bad_event_contract"),
        [op(
            "validate",
            [StagePermission::ReadWorld],
            std::iter::empty::<EventRecordSpec>(),
        )],
        EventContract::new([event("Committed", ["actor"])]),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("effect programs must be able to emit required events");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventNotEmitted {
            definition: id(12),
            event: event("Committed", ["actor"]),
        }
    );

    let Err(error) = EffectOp::new(
        EffectKind::from_static("mutate_without_event"),
        [StagePermission::MutatePhysical],
        std::iter::empty::<EventRecordSpec>(),
    ) else {
        panic!("mutating operations must declare emitted events");
    };
    assert_eq!(
        error,
        DefinitionError::OperationRequiresEvent {
            operation: EffectKind::from_static("mutate_without_event"),
        }
    );

    let Ok(control_operation) = EffectOp::new(
        EffectKind::from_static("schedule_without_event"),
        [StagePermission::ScheduleProcess],
        std::iter::empty::<EventRecordSpec>(),
    ) else {
        panic!("runtime-control effect permissions must not force event records");
    };
    assert!(control_operation.emits_no_events());
    assert!(!control_operation.requires_event());

    let Err(error) = EffectOp::new(
        EffectKind::from_static("read_with_event"),
        [StagePermission::ReadWorld],
        [event("ReadOnlyEvent", ["actor"])],
    ) else {
        panic!("event-emitting operations must declare event permission");
    };
    assert_eq!(
        error,
        DefinitionError::EventPermissionNotDeclared {
            operation: EffectKind::from_static("read_with_event"),
        }
    );

    let Err(error) = EffectProgramDef::new(
        id(13),
        DefinitionName::from_static("uncontracted_event"),
        [op(
            "emit_uncontracted",
            [StagePermission::EmitPhysicalEventRecord],
            [event("Uncontracted", ["actor"])],
        )],
        EventContract::default(),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("event-emitting operations must be covered by the program contract");
    };
    assert_eq!(
        error,
        DefinitionError::EventNotPermittedByContract {
            definition: id(13),
            event: event("Uncontracted", ["actor"]),
        }
    );

    let Ok(optional_event_program) = EffectProgramDef::new(
        id(14),
        DefinitionName::from_static("optional_event"),
        [op(
            "emit_optional",
            [StagePermission::EmitSensoryEventRecord],
            [event("SmokeEmitted", ["actor"])],
        )],
        EventContract::with_allowed([], [event("SmokeEmitted", ["actor"])]),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("allowed events should be permitted without becoming required");
    };
    assert!(
        optional_event_program
            .event_contract()
            .allowed_events()
            .any(|event| event.kind().as_str() == "SmokeEmitted")
    );

    let Err(error) = EffectProgramDef::new(
        id(15),
        DefinitionName::from_static("wrong_event_shape"),
        [op(
            "emit_wrong_shape",
            [StagePermission::EmitPhysicalEventRecord],
            [event("EntityTransferred", ["actor", "item", "destination"])],
        )],
        EventContract::new([event("EntityTransferred", ["actor", "item"])]),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("event contracts must match role sets, not only event kind");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventNotEmitted {
            definition: id(15),
            event: event("EntityTransferred", ["actor", "item"]),
        }
    );

    let Err(error) = EffectProgramDef::new(
        id(16),
        DefinitionName::from_static("wrong_event_version"),
        [op(
            "emit_wrong_version",
            [StagePermission::EmitPhysicalEventRecord],
            [event_with_version("EntityTransferred", ["actor"], 2)],
        )],
        EventContract::with_allowed([], [event_with_version("EntityTransferred", ["actor"], 1)]),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("event contracts must match version anchors, not only event kind");
    };
    assert_eq!(
        error,
        DefinitionError::EventNotPermittedByContract {
            definition: id(16),
            event: event_with_version("EntityTransferred", ["actor"], 2),
        }
    );
}

#[test]
fn registry_validates_action_effect_reference_permissions_and_events() {
    let program = transfer_program();

    let Err(error) = DefinitionRegistry::new([], [action(id(99))], [], []) else {
        panic!("missing effect references must be rejected");
    };
    assert_eq!(
        error,
        DefinitionError::MissingEffectProgram {
            definition: id(2),
            effect_program: id(99),
        }
    );

    let Ok(under_permitted) = ActionDef::new(
        id(4),
        DefinitionName::from_static("under_permitted"),
        [role("actor")],
        [],
        [],
        id(1),
        EventContract::default(),
        [StagePermission::ReadWorld, StagePermission::MutatePhysical],
        version(1),
    ) else {
        panic!("local action shape should be valid");
    };
    let Err(error) = DefinitionRegistry::new([program.clone()], [under_permitted], [], []) else {
        panic!("registry must reject missing stage permissions");
    };
    assert_eq!(
        error,
        DefinitionError::PermissionNotDeclared {
            definition: id(4),
            effect_program: id(1),
            permission: StagePermission::EmitPhysicalEventRecord,
        }
    );

    let Ok(weak_contract) = ActionDef::new(
        id(8),
        DefinitionName::from_static("weak_contract"),
        [role("actor"), role("item"), role("destination")],
        [],
        [],
        id(1),
        EventContract::default(),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    ) else {
        panic!("local action shape should be valid");
    };
    let Err(error) = DefinitionRegistry::new([program.clone()], [weak_contract], [], []) else {
        panic!("registry must reject action contracts weaker than the effect program");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventNotDeclared {
            definition: id(8),
            event: event("EntityTransferred", ["actor", "item", "destination"]),
        }
    );

    let Ok(bad_event) = ActionDef::new(
        id(5),
        DefinitionName::from_static("bad_event"),
        [role("actor"), role("item"), role("destination")],
        [],
        [],
        id(1),
        EventContract::new([
            event("EntityTransferred", ["actor", "item", "destination"]),
            event("OtherEvent", ["actor"]),
        ]),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    ) else {
        panic!("local action shape should be valid");
    };
    let Err(error) = DefinitionRegistry::new([program.clone()], [bad_event], [], []) else {
        panic!("registry must reject unavailable required events");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventUnavailable {
            definition: id(5),
            event: event("OtherEvent", ["actor"]),
        }
    );

    let Ok(registry) = DefinitionRegistry::new(
        [program.clone()],
        [action(program.id())],
        [process(program.id())],
        [],
    ) else {
        panic!("registry should accept matching definitions");
    };
    assert!(registry.effect_program(program.id()).is_some());
    assert!(registry.action(id(2)).is_some());
    assert!(registry.process(id(3)).is_some());
}

#[test]
fn registry_validates_process_effect_references_permissions_and_events() {
    let program = transfer_program();

    let Err(error) = DefinitionRegistry::new([], [], [process(id(99))], []) else {
        panic!("missing process effect references must be rejected");
    };
    assert_eq!(
        error,
        DefinitionError::MissingEffectProgram {
            definition: id(3),
            effect_program: id(99),
        }
    );

    let Ok(under_permitted) = ProcessDef::new(
        id(6),
        DefinitionName::from_static("under_permitted_process"),
        [role("actor")],
        state_schema(),
        [support(ResolutionTier::Concrete, id(1))],
        policies(),
        EventContract::default(),
        [StagePermission::ReadWorld, StagePermission::MutatePhysical],
        version(1),
    ) else {
        panic!("local process shape should be valid");
    };
    let Err(error) = DefinitionRegistry::new([program.clone()], [], [under_permitted], []) else {
        panic!("registry must reject missing process stage permissions");
    };
    assert_eq!(
        error,
        DefinitionError::PermissionNotDeclared {
            definition: id(6),
            effect_program: id(1),
            permission: StagePermission::EmitPhysicalEventRecord,
        }
    );

    let Ok(weak_contract) = ProcessDef::new(
        id(9),
        DefinitionName::from_static("weak_process_contract"),
        [role("actor"), role("item"), role("destination")],
        state_schema(),
        [support(ResolutionTier::Concrete, id(1))],
        policies(),
        EventContract::default(),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    ) else {
        panic!("local process shape should be valid");
    };
    let Err(error) = DefinitionRegistry::new([program.clone()], [], [weak_contract], []) else {
        panic!("registry must reject process contracts weaker than the effect program");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventNotDeclared {
            definition: id(9),
            event: event("EntityTransferred", ["actor", "item", "destination"]),
        }
    );

    let Ok(bad_event) = ProcessDef::new(
        id(7),
        DefinitionName::from_static("bad_process_event"),
        [role("actor"), role("item"), role("destination")],
        state_schema(),
        [support(ResolutionTier::Concrete, id(1))],
        policies(),
        EventContract::new([
            event("EntityTransferred", ["actor", "item", "destination"]),
            event("OtherEvent", ["actor"]),
        ]),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    ) else {
        panic!("local process shape should be valid");
    };
    let Err(error) = DefinitionRegistry::new([program.clone()], [], [bad_event], []) else {
        panic!("registry must reject unavailable process event contracts");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventUnavailable {
            definition: id(7),
            event: event("OtherEvent", ["actor"]),
        }
    );

    let Ok(read_only_program) = EffectProgramDef::new(
        id(40),
        DefinitionName::from_static("abstract_observe"),
        [op(
            "observe",
            [StagePermission::ReadWorld],
            std::iter::empty::<EventRecordSpec>(),
        )],
        EventContract::default(),
        ReplayLevel::AuditOnly,
        version(1),
    ) else {
        panic!("read-only program should be locally valid");
    };

    let Ok(resolution_mismatch) = ProcessDef::new(
        id(41),
        DefinitionName::from_static("resolution_mismatch"),
        [role("actor"), role("item"), role("destination")],
        state_schema(),
        [
            support(ResolutionTier::Concrete, id(1)),
            support(ResolutionTier::Abstract, id(40)),
        ],
        policies(),
        EventContract::new([event("EntityTransferred", ["actor", "item", "destination"])]),
        [
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ],
        version(1),
    ) else {
        panic!("local process shape should be valid");
    };
    let Err(error) =
        DefinitionRegistry::new([program, read_only_program], [], [resolution_mismatch], [])
    else {
        panic!("registry must validate process events per supported resolution");
    };
    assert_eq!(
        error,
        DefinitionError::RequiredEventUnavailable {
            definition: id(41),
            event: event("EntityTransferred", ["actor", "item", "destination"]),
        }
    );
}

#[test]
fn process_defs_require_supported_resolution_and_preserve_lookup() {
    assert!(empty_state_schema().fields().is_empty());

    let Err(error) = ProcessStateSchema::new([
        ProcessStateField::new(
            StateFieldName::from_static("progress"),
            StateValueType::from_static("u32"),
        ),
        ProcessStateField::new(
            StateFieldName::from_static("progress"),
            StateValueType::from_static("u64"),
        ),
    ]) else {
        panic!("process state schemas must reject duplicate fields");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateStateField {
            field: StateFieldName::from_static("progress"),
        }
    );

    let Err(error) = ProcessDef::new(
        id(20),
        DefinitionName::from_static("bad_process"),
        [role("actor")],
        state_schema(),
        [],
        policies(),
        EventContract::default(),
        [StagePermission::ReadWorld],
        version(1),
    ) else {
        panic!("process definitions must declare supported resolutions");
    };
    assert_eq!(
        error,
        DefinitionError::EmptyDefinitionField {
            definition: id(20),
            type_name: "ProcessDef",
            field: "resolution_support",
        }
    );

    let Err(error) = ProcessDef::new(
        id(21),
        DefinitionName::from_static("duplicate_resolution"),
        [role("actor")],
        state_schema(),
        [
            support(ResolutionTier::Concrete, id(1)),
            support(ResolutionTier::Concrete, id(1)),
        ],
        policies(),
        EventContract::default(),
        [StagePermission::ReadWorld],
        version(1),
    ) else {
        panic!("process definitions must reject duplicate resolution support");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateResolutionSupport {
            definition: id(21),
            resolution: ResolutionTier::Concrete,
        }
    );

    let process = process(id(1));
    assert!(process.supports_resolution(ResolutionTier::Concrete));
    assert!(process.supports_resolution(ResolutionTier::Abstract));
    assert!(!process.supports_resolution(ResolutionTier::Strategic));
    assert_eq!(
        process
            .resolution_policy(ResolutionTier::Abstract)
            .map(PolicyKey::as_str),
        Some("abstract_lowering")
    );
}

#[test]
fn semantic_declarations_validate_kind_owned_outputs() {
    let Err(error) = SemanticDeclarationDef::new(
        id(30),
        DefinitionName::from_static("bad_intent_output"),
        SemanticDeclarationKind::IntentTemplate,
        [
            SemanticInputKind::Pressure,
            SemanticInputKind::ActionRepertoire,
        ],
        [SemanticOutputKind::SocialUpdateProposal],
        version(1),
    ) else {
        panic!("intent templates must not output social updates");
    };
    assert_eq!(
        error,
        DefinitionError::ForbiddenSemanticOutput {
            definition: id(30),
            kind: SemanticDeclarationKind::IntentTemplate,
            output: SemanticOutputKind::SocialUpdateProposal,
        }
    );

    let Err(error) = SemanticDeclarationDef::new(
        id(32),
        DefinitionName::from_static("bad_social_output"),
        SemanticDeclarationKind::SocialRule,
        [
            SemanticInputKind::HardEventEvidence,
            SemanticInputKind::SocialContext,
        ],
        [SemanticOutputKind::Pressure],
        version(1),
    ) else {
        panic!("social rules must not output appraisal pressure directly");
    };
    assert_eq!(
        error,
        DefinitionError::ForbiddenSemanticOutput {
            definition: id(32),
            kind: SemanticDeclarationKind::SocialRule,
            output: SemanticOutputKind::Pressure,
        }
    );

    let Ok(declaration) = SemanticDeclarationDef::new(
        id(31),
        DefinitionName::from_static("intent_candidate"),
        SemanticDeclarationKind::IntentTemplate,
        [
            SemanticInputKind::Pressure,
            SemanticInputKind::CapabilitySet,
            SemanticInputKind::ActionRepertoire,
            SemanticInputKind::PerceivedAffordance,
        ],
        [
            SemanticOutputKind::CandidateIntent,
            SemanticOutputKind::IntentScoreFeature,
            SemanticOutputKind::LoweringContract,
            SemanticOutputKind::ActivityPreparation,
        ],
        version(1),
    ) else {
        panic!("intent templates may output intent preparation artifacts");
    };
    assert_eq!(declaration.kind(), SemanticDeclarationKind::IntentTemplate);
    assert!(
        declaration
            .outputs()
            .contains(&SemanticOutputKind::CandidateIntent)
    );

    let Ok(appraisal) = SemanticDeclarationDef::new(
        id(33),
        DefinitionName::from_static("fear_appraisal"),
        SemanticDeclarationKind::AppraisalRule,
        [
            SemanticInputKind::HardEventEvidence,
            SemanticInputKind::ActorContext,
        ],
        [
            SemanticOutputKind::Thought,
            SemanticOutputKind::Pressure,
            SemanticOutputKind::GoalPressure,
            SemanticOutputKind::AppraisalRecordProposal,
        ],
        version(1),
    ) else {
        panic!("appraisal rules may output appraisal artifacts");
    };
    assert!(appraisal.outputs().contains(&SemanticOutputKind::Pressure));
}

#[test]
fn registry_rejects_duplicate_definition_ids_across_families() {
    let program = transfer_program();
    let duplicate_program = program.clone();
    let Err(error) = DefinitionRegistry::new([program.clone(), duplicate_program], [], [], [])
    else {
        panic!("registry must reject ids reused within a definition family");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateDefinitionId { id: program.id() }
    );

    let semantic = {
        let Ok(declaration) = SemanticDeclarationDef::new(
            program.id(),
            DefinitionName::from_static("duplicate"),
            SemanticDeclarationKind::SemanticView,
            [SemanticInputKind::ActorContext],
            [SemanticOutputKind::DerivedActorContext],
            version(1),
        ) else {
            panic!("semantic declaration should be locally valid");
        };
        declaration
    };

    let Err(error) = DefinitionRegistry::new([program.clone()], [], [], [semantic]) else {
        panic!("registry must reject ids reused across definition families");
    };
    assert_eq!(
        error,
        DefinitionError::DuplicateDefinitionId { id: program.id() }
    );
}
