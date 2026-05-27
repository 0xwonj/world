use std::collections::{BTreeMap, BTreeSet};

use world_core::DefinitionId;

use crate::actions::ActionDef;
use crate::effects::{EffectProgramDef, StagePermission};
use crate::error::DefinitionError;
use crate::events::{EventContract, EventRecordSpec};
use crate::processes::ProcessDef;
use crate::semantics::{SemanticDeclarationDef, SemanticDeclarationKind};

/// Parser-free lookup table for checked runtime definitions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DefinitionRegistry {
    effect_programs: BTreeMap<DefinitionId, EffectProgramDef>,
    actions: BTreeMap<DefinitionId, ActionDef>,
    processes: BTreeMap<DefinitionId, ProcessDef>,
    semantic_declarations: BTreeMap<DefinitionId, SemanticDeclarationDef>,
}

impl DefinitionRegistry {
    /// Builds a registry and validates cross-definition contracts.
    pub fn new(
        effect_programs: impl IntoIterator<Item = EffectProgramDef>,
        actions: impl IntoIterator<Item = ActionDef>,
        processes: impl IntoIterator<Item = ProcessDef>,
        semantic_declarations: impl IntoIterator<Item = SemanticDeclarationDef>,
    ) -> Result<Self, DefinitionError> {
        let mut seen = BTreeSet::new();
        let effect_programs = collect_unique(effect_programs, &mut seen)?;
        let actions = collect_unique(actions, &mut seen)?;
        let processes = collect_unique(processes, &mut seen)?;
        let semantic_declarations = collect_unique(semantic_declarations, &mut seen)?;

        let registry = Self {
            effect_programs,
            actions,
            processes,
            semantic_declarations,
        };

        registry.validate_actions()?;
        registry.validate_processes()?;

        Ok(registry)
    }

    /// Looks up a checked effect program.
    pub fn effect_program(&self, id: DefinitionId) -> Option<&EffectProgramDef> {
        self.effect_programs.get(&id)
    }

    /// Looks up a checked action definition.
    pub fn action(&self, id: DefinitionId) -> Option<&ActionDef> {
        self.actions.get(&id)
    }

    /// Looks up a checked process definition.
    pub fn process(&self, id: DefinitionId) -> Option<&ProcessDef> {
        self.processes.get(&id)
    }

    /// Looks up a checked semantic declaration.
    pub fn semantic_declaration(&self, id: DefinitionId) -> Option<&SemanticDeclarationDef> {
        self.semantic_declarations.get(&id)
    }

    /// Returns checked actions.
    pub fn actions(&self) -> impl Iterator<Item = &ActionDef> {
        self.actions.values()
    }

    /// Returns checked effect programs.
    pub fn effect_programs(&self) -> impl Iterator<Item = &EffectProgramDef> {
        self.effect_programs.values()
    }

    /// Returns checked process definitions.
    pub fn processes(&self) -> impl Iterator<Item = &ProcessDef> {
        self.processes.values()
    }

    /// Returns checked semantic declarations.
    pub fn semantic_declarations(&self) -> impl Iterator<Item = &SemanticDeclarationDef> {
        self.semantic_declarations.values()
    }

    /// Returns checked semantic declarations for one declaration family.
    pub fn semantic_declarations_by_kind(
        &self,
        kind: SemanticDeclarationKind,
    ) -> impl Iterator<Item = &SemanticDeclarationDef> {
        self.semantic_declarations
            .values()
            .filter(move |declaration| declaration.kind() == kind)
    }

    fn validate_actions(&self) -> Result<(), DefinitionError> {
        for action in self.actions.values() {
            let Some(program) = self.effect_program(action.effect_program()) else {
                return Err(DefinitionError::MissingEffectProgram {
                    definition: action.id(),
                    effect_program: action.effect_program(),
                });
            };

            let declared_permissions = action.stage_permissions().copied().collect::<BTreeSet<_>>();
            validate_program_against_definition(
                action.id(),
                program,
                &declared_permissions,
                action.event_contract(),
            )?;
            validate_event_coverage(
                action.id(),
                action.event_contract(),
                &program.emitted_events(),
            )?;
        }

        Ok(())
    }

    fn validate_processes(&self) -> Result<(), DefinitionError> {
        for process in self.processes.values() {
            let declared_permissions = process
                .stage_permissions()
                .copied()
                .collect::<BTreeSet<_>>();

            for support in process
                .supported_resolutions()
                .filter_map(|resolution| process.resolution_support(*resolution))
            {
                let mut available_events = BTreeSet::new();

                for effect_program in support.effect_programs() {
                    let Some(program) = self.effect_program(*effect_program) else {
                        return Err(DefinitionError::MissingEffectProgram {
                            definition: process.id(),
                            effect_program: *effect_program,
                        });
                    };

                    validate_program_against_definition(
                        process.id(),
                        program,
                        &declared_permissions,
                        process.event_contract(),
                    )?;
                    available_events.extend(program.emitted_events());
                }

                validate_event_coverage(process.id(), process.event_contract(), &available_events)?;
            }
        }

        Ok(())
    }
}

trait RegistryItem {
    fn id(&self) -> DefinitionId;
}

impl RegistryItem for EffectProgramDef {
    fn id(&self) -> DefinitionId {
        self.id()
    }
}

impl RegistryItem for ActionDef {
    fn id(&self) -> DefinitionId {
        self.id()
    }
}

impl RegistryItem for ProcessDef {
    fn id(&self) -> DefinitionId {
        self.id()
    }
}

impl RegistryItem for SemanticDeclarationDef {
    fn id(&self) -> DefinitionId {
        self.id()
    }
}

fn collect_unique<T>(
    items: impl IntoIterator<Item = T>,
    seen: &mut BTreeSet<DefinitionId>,
) -> Result<BTreeMap<DefinitionId, T>, DefinitionError>
where
    T: RegistryItem,
{
    let mut out = BTreeMap::new();

    for item in items {
        let id = item.id();
        if !seen.insert(id) {
            return Err(DefinitionError::DuplicateDefinitionId { id });
        }
        out.insert(id, item);
    }

    Ok(out)
}

fn validate_program_against_definition(
    definition: DefinitionId,
    program: &EffectProgramDef,
    declared_permissions: &BTreeSet<StagePermission>,
    definition_contract: &EventContract,
) -> Result<(), DefinitionError> {
    validate_permission_coverage(
        definition,
        program.id(),
        declared_permissions,
        program.required_permissions().iter(),
    )?;
    validate_required_event_declaration(definition, definition_contract, program.event_contract())?;
    validate_permitted_events(definition, definition_contract, &program.emitted_events())
}

fn validate_permission_coverage<'a>(
    definition: DefinitionId,
    effect_program: DefinitionId,
    declared: &BTreeSet<StagePermission>,
    required: impl Iterator<Item = &'a StagePermission>,
) -> Result<(), DefinitionError> {
    for permission in required {
        if !declared.contains(permission) {
            return Err(DefinitionError::PermissionNotDeclared {
                definition,
                effect_program,
                permission: *permission,
            });
        }
    }

    Ok(())
}

fn validate_event_coverage(
    definition: DefinitionId,
    event_contract: &EventContract,
    available_events: &BTreeSet<EventRecordSpec>,
) -> Result<(), DefinitionError> {
    for event in event_contract.required_events() {
        if !available_events.contains(event) {
            return Err(DefinitionError::RequiredEventUnavailable {
                definition,
                event: event.clone(),
            });
        }
    }

    Ok(())
}

fn validate_permitted_events(
    definition: DefinitionId,
    event_contract: &EventContract,
    emitted_events: &BTreeSet<EventRecordSpec>,
) -> Result<(), DefinitionError> {
    for event in emitted_events {
        if !event_contract.permits_event(event) {
            return Err(DefinitionError::EventNotPermittedByContract {
                definition,
                event: event.clone(),
            });
        }
    }

    Ok(())
}

fn validate_required_event_declaration(
    definition: DefinitionId,
    definition_contract: &EventContract,
    program_contract: &EventContract,
) -> Result<(), DefinitionError> {
    for event in program_contract.required_events() {
        if !definition_contract.requires_event(event) {
            return Err(DefinitionError::RequiredEventNotDeclared {
                definition,
                event: event.clone(),
            });
        }
    }

    Ok(())
}
