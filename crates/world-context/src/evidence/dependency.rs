use std::collections::BTreeSet;

use world_core::DefinitionId;
use world_model::{AuthorityRead, InvalidationPackage};

/// Typed read dependency used by actor-context projection reports.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContextReadDependency {
    /// A model authority/store read label.
    Authority(AuthorityRead),
    /// A checked definition read by id.
    Definition(DefinitionId),
}

/// Deterministic set of model and definition inputs used by a projection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextReadSet {
    authority_reads: BTreeSet<AuthorityRead>,
    definitions: BTreeSet<DefinitionId>,
}

impl ContextReadSet {
    /// Creates an empty read set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            authority_reads: BTreeSet::new(),
            definitions: BTreeSet::new(),
        }
    }

    /// Records one authority read label.
    pub fn insert_authority_read(&mut self, read: AuthorityRead) -> bool {
        self.authority_reads.insert(read)
    }

    /// Records one checked definition dependency.
    pub fn insert_definition(&mut self, definition: DefinitionId) -> bool {
        self.definitions.insert(definition)
    }

    /// Records one typed read dependency.
    pub fn insert(&mut self, dependency: ContextReadDependency) -> bool {
        match dependency {
            ContextReadDependency::Authority(read) => self.insert_authority_read(read),
            ContextReadDependency::Definition(definition) => self.insert_definition(definition),
        }
    }

    /// Returns true when no dependencies have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.authority_reads.is_empty() && self.definitions.is_empty()
    }

    /// Iterates authority read labels in deterministic order.
    pub fn authority_reads(&self) -> impl Iterator<Item = AuthorityRead> + '_ {
        self.authority_reads.iter().copied()
    }

    /// Iterates checked definition dependencies in deterministic order.
    pub fn definitions(&self) -> impl Iterator<Item = DefinitionId> + '_ {
        self.definitions.iter().copied()
    }

    /// Returns whether the set contains the authority read label.
    #[must_use]
    pub fn contains_authority_read(&self, read: AuthorityRead) -> bool {
        self.authority_reads.contains(&read)
    }

    /// Returns whether the set contains the checked definition id.
    #[must_use]
    pub fn contains_definition(&self, definition: DefinitionId) -> bool {
        self.definitions.contains(&definition)
    }

    /// Returns whether a model invalidation may stale this context projection.
    ///
    /// This checks only model-state invalidation packages. Definition and
    /// registry changes need a separate definition invalidation input.
    #[must_use]
    pub fn is_invalidated_by_model(&self, invalidation: &InvalidationPackage) -> bool {
        self.authority_reads.iter().any(|read| {
            invalidation.contains_authority_class(read.authority_class())
                || invalidation.contains_store_family(read.store_family())
        })
    }
}
