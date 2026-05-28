use std::collections::{BTreeMap, BTreeSet};

use world_core::DefinitionId;

use crate::actions::ActionDef;
use crate::effects::{EffectPrimitiveDef, EffectPrimitiveId, EffectProgramDef};
use crate::error::DefinitionError;
use crate::processes::ProcessDef;
use crate::semantics::SemanticDeclarationDef;

use super::{DefinitionRegistry, validate};

/// Incremental builder for parser-free checked runtime definitions.
#[derive(Clone, Debug, Default)]
pub struct DefinitionRegistryBuilder {
    seen: BTreeSet<DefinitionId>,
    effect_primitives: BTreeMap<EffectPrimitiveId, EffectPrimitiveDef>,
    effect_programs: BTreeMap<DefinitionId, EffectProgramDef>,
    actions: BTreeMap<DefinitionId, ActionDef>,
    processes: BTreeMap<DefinitionId, ProcessDef>,
    semantic_declarations: BTreeMap<DefinitionId, SemanticDeclarationDef>,
}

/// Pure definition bundle that installs checked definitions into a registry builder.
pub trait DefinitionBundle {
    /// Installs definitions without receiving runtime mutation authority.
    fn install_definitions(
        &self,
        builder: &mut DefinitionRegistryBuilder,
    ) -> Result<(), DefinitionError>;
}

impl DefinitionRegistryBuilder {
    /// Creates an empty registry builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Installs a pure checked definition bundle.
    pub fn install<B>(&mut self, bundle: &B) -> Result<&mut Self, DefinitionError>
    where
        B: DefinitionBundle + ?Sized,
    {
        bundle.install_definitions(self)?;
        Ok(self)
    }

    /// Adds a checked primitive effect definition.
    pub fn add_primitive(
        &mut self,
        primitive: EffectPrimitiveDef,
    ) -> Result<&mut Self, DefinitionError> {
        insert_unique(&mut self.effect_primitives, &mut self.seen, primitive)?;
        Ok(self)
    }

    /// Adds a checked effect program definition.
    pub fn add_effect_program(
        &mut self,
        program: EffectProgramDef,
    ) -> Result<&mut Self, DefinitionError> {
        insert_unique(&mut self.effect_programs, &mut self.seen, program)?;
        Ok(self)
    }

    /// Adds a checked action definition.
    pub fn add_action(&mut self, action: ActionDef) -> Result<&mut Self, DefinitionError> {
        insert_unique(&mut self.actions, &mut self.seen, action)?;
        Ok(self)
    }

    /// Adds a checked process definition.
    pub fn add_process(&mut self, process: ProcessDef) -> Result<&mut Self, DefinitionError> {
        insert_unique(&mut self.processes, &mut self.seen, process)?;
        Ok(self)
    }

    /// Adds a checked semantic declaration definition.
    pub fn add_semantic_declaration(
        &mut self,
        declaration: SemanticDeclarationDef,
    ) -> Result<&mut Self, DefinitionError> {
        insert_unique(&mut self.semantic_declarations, &mut self.seen, declaration)?;
        Ok(self)
    }

    /// Builds an immutable registry and validates cross-definition contracts.
    pub fn build(self) -> Result<DefinitionRegistry, DefinitionError> {
        let registry = DefinitionRegistry {
            effect_primitives: self.effect_primitives,
            effect_programs: self.effect_programs,
            actions: self.actions,
            processes: self.processes,
            semantic_declarations: self.semantic_declarations,
        };
        validate::registry(&registry)?;
        Ok(registry)
    }
}

trait RegistryItem {
    type Key: Ord;

    fn key(&self) -> Self::Key;

    fn definition_id(&self) -> DefinitionId;
}

impl RegistryItem for EffectPrimitiveDef {
    type Key = EffectPrimitiveId;

    fn key(&self) -> Self::Key {
        self.id()
    }

    fn definition_id(&self) -> DefinitionId {
        self.id().definition()
    }
}

impl RegistryItem for EffectProgramDef {
    type Key = DefinitionId;

    fn key(&self) -> Self::Key {
        self.id()
    }

    fn definition_id(&self) -> DefinitionId {
        self.id()
    }
}

impl RegistryItem for ActionDef {
    type Key = DefinitionId;

    fn key(&self) -> Self::Key {
        self.id()
    }

    fn definition_id(&self) -> DefinitionId {
        self.id()
    }
}

impl RegistryItem for ProcessDef {
    type Key = DefinitionId;

    fn key(&self) -> Self::Key {
        self.id()
    }

    fn definition_id(&self) -> DefinitionId {
        self.id()
    }
}

impl RegistryItem for SemanticDeclarationDef {
    type Key = DefinitionId;

    fn key(&self) -> Self::Key {
        self.id()
    }

    fn definition_id(&self) -> DefinitionId {
        self.id()
    }
}

fn insert_unique<T>(
    items: &mut BTreeMap<T::Key, T>,
    seen: &mut BTreeSet<DefinitionId>,
    item: T,
) -> Result<(), DefinitionError>
where
    T: RegistryItem,
{
    let id = item.definition_id();
    if !seen.insert(id) {
        return Err(DefinitionError::DuplicateDefinitionId { id });
    }
    items.insert(item.key(), item);
    Ok(())
}
