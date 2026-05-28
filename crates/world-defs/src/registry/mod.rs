use std::collections::BTreeMap;

use world_core::DefinitionId;

use crate::actions::ActionDef;
use crate::effects::{EffectPrimitiveDef, EffectPrimitiveId, EffectProgramDef};
use crate::error::DefinitionError;
use crate::processes::ProcessDef;
use crate::semantics::{SemanticDeclarationDef, SemanticDeclarationKind};

mod builder;
mod validate;

pub use builder::{DefinitionBundle, DefinitionRegistryBuilder};

/// Parser-free lookup table for checked runtime definitions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DefinitionRegistry {
    effect_primitives: BTreeMap<EffectPrimitiveId, EffectPrimitiveDef>,
    effect_programs: BTreeMap<DefinitionId, EffectProgramDef>,
    actions: BTreeMap<DefinitionId, ActionDef>,
    processes: BTreeMap<DefinitionId, ProcessDef>,
    semantic_declarations: BTreeMap<DefinitionId, SemanticDeclarationDef>,
}

impl DefinitionRegistry {
    /// Builds a registry and validates cross-definition contracts.
    pub fn new(
        effect_primitives: impl IntoIterator<Item = EffectPrimitiveDef>,
        effect_programs: impl IntoIterator<Item = EffectProgramDef>,
        actions: impl IntoIterator<Item = ActionDef>,
        processes: impl IntoIterator<Item = ProcessDef>,
        semantic_declarations: impl IntoIterator<Item = SemanticDeclarationDef>,
    ) -> Result<Self, DefinitionError> {
        let mut builder = DefinitionRegistryBuilder::new();

        for primitive in effect_primitives {
            builder.add_primitive(primitive)?;
        }
        for program in effect_programs {
            builder.add_effect_program(program)?;
        }
        for action in actions {
            builder.add_action(action)?;
        }
        for process in processes {
            builder.add_process(process)?;
        }
        for declaration in semantic_declarations {
            builder.add_semantic_declaration(declaration)?;
        }

        builder.build()
    }

    /// Looks up a checked primitive effect definition.
    pub fn effect_primitive(&self, id: EffectPrimitiveId) -> Option<&EffectPrimitiveDef> {
        self.effect_primitives.get(&id)
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

    /// Returns checked primitive effect definitions.
    pub fn effect_primitives(&self) -> impl Iterator<Item = &EffectPrimitiveDef> {
        self.effect_primitives.values()
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
}
