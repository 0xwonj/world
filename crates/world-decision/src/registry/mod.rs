use std::collections::BTreeMap;

use world_core::DefinitionId;

use crate::{DecisionError, DecisionPassContract, DecisionProfile, RepresentationKindDef};

mod builder;
mod validate;

pub use builder::DecisionRegistryBuilder;

/// Parser-free lookup table for checked decision declarations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DecisionRegistry {
    representations: BTreeMap<DefinitionId, RepresentationKindDef>,
    passes: BTreeMap<DefinitionId, DecisionPassContract>,
    profiles: BTreeMap<DefinitionId, DecisionProfile>,
}

impl DecisionRegistry {
    /// Builds a registry and validates cross-declaration contracts.
    pub fn new(
        representations: impl IntoIterator<Item = RepresentationKindDef>,
        passes: impl IntoIterator<Item = DecisionPassContract>,
        profiles: impl IntoIterator<Item = DecisionProfile>,
    ) -> Result<Self, DecisionError> {
        let mut builder = DecisionRegistryBuilder::new();

        for representation in representations {
            builder.add_representation(representation)?;
        }
        for pass in passes {
            builder.add_pass(pass)?;
        }
        for profile in profiles {
            builder.add_profile(profile)?;
        }

        builder.build()
    }

    /// Looks up a checked representation kind.
    #[must_use]
    pub fn representation(&self, id: DefinitionId) -> Option<&RepresentationKindDef> {
        self.representations.get(&id)
    }

    /// Looks up a checked pass contract.
    #[must_use]
    pub fn pass(&self, id: DefinitionId) -> Option<&DecisionPassContract> {
        self.passes.get(&id)
    }

    /// Looks up a checked decision profile.
    #[must_use]
    pub fn profile(&self, id: DefinitionId) -> Option<&DecisionProfile> {
        self.profiles.get(&id)
    }

    /// Returns checked representation kinds in deterministic order.
    pub fn representations(&self) -> impl Iterator<Item = &RepresentationKindDef> {
        self.representations.values()
    }

    /// Returns checked pass contracts in deterministic order.
    pub fn passes(&self) -> impl Iterator<Item = &DecisionPassContract> {
        self.passes.values()
    }

    /// Returns checked profiles in deterministic order.
    pub fn profiles(&self) -> impl Iterator<Item = &DecisionProfile> {
        self.profiles.values()
    }
}
