use std::collections::{BTreeMap, BTreeSet};

use world_core::DefinitionId;

use crate::{
    DecisionError, DecisionPassContract, DecisionProfile, DecisionRegistry, RepresentationKindDef,
};

use super::validate;

/// Incremental builder for checked decision declarations.
#[derive(Clone, Debug, Default)]
pub struct DecisionRegistryBuilder {
    seen: BTreeSet<DefinitionId>,
    representations: BTreeMap<DefinitionId, RepresentationKindDef>,
    passes: BTreeMap<DefinitionId, DecisionPassContract>,
    profiles: BTreeMap<DefinitionId, DecisionProfile>,
}

impl DecisionRegistryBuilder {
    /// Creates an empty registry builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a checked representation kind.
    pub fn add_representation(
        &mut self,
        representation: RepresentationKindDef,
    ) -> Result<&mut Self, DecisionError> {
        insert_unique(&mut self.representations, &mut self.seen, representation)?;
        Ok(self)
    }

    /// Adds a checked pass contract.
    pub fn add_pass(&mut self, pass: DecisionPassContract) -> Result<&mut Self, DecisionError> {
        insert_unique(&mut self.passes, &mut self.seen, pass)?;
        Ok(self)
    }

    /// Adds a checked decision profile.
    pub fn add_profile(&mut self, profile: DecisionProfile) -> Result<&mut Self, DecisionError> {
        insert_unique(&mut self.profiles, &mut self.seen, profile)?;
        Ok(self)
    }

    /// Builds an immutable registry and validates cross-declaration contracts.
    pub fn build(self) -> Result<DecisionRegistry, DecisionError> {
        let registry = DecisionRegistry {
            representations: self.representations,
            passes: self.passes,
            profiles: self.profiles,
        };
        validate::registry(&registry)?;
        Ok(registry)
    }
}

trait RegistryItem {
    fn id(&self) -> DefinitionId;
}

impl RegistryItem for RepresentationKindDef {
    fn id(&self) -> DefinitionId {
        self.id()
    }
}

impl RegistryItem for DecisionPassContract {
    fn id(&self) -> DefinitionId {
        self.id()
    }
}

impl RegistryItem for DecisionProfile {
    fn id(&self) -> DefinitionId {
        self.id()
    }
}

fn insert_unique<T>(
    items: &mut BTreeMap<DefinitionId, T>,
    seen: &mut BTreeSet<DefinitionId>,
    item: T,
) -> Result<(), DecisionError>
where
    T: RegistryItem,
{
    let id = item.id();
    if !seen.insert(id) {
        return Err(DecisionError::DuplicateDefinitionId { id });
    }
    items.insert(id, item);
    Ok(())
}
