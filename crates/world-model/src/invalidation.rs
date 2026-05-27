use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;

use world_core::{AuthorityClass, CausalTransactionId, InvalidCoreValue, QueryEpoch};

#[cfg(test)]
use crate::ModelError;
use crate::{AuthorityRead, StoreFamily};

/// Local identity for derived views registered with the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DerivedViewKey(NonZeroU64);

impl DerivedViewKey {
    /// Creates a derived view key when the raw value is nonzero.
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the underlying stable numeric value.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl TryFrom<u64> for DerivedViewKey {
    type Error = InvalidCoreValue;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(InvalidCoreValue::Zero {
            type_name: "DerivedViewKey",
        })
    }
}

impl From<DerivedViewKey> for u64 {
    fn from(value: DerivedViewKey) -> Self {
        value.get()
    }
}

/// Origin of an invalidation package.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InvalidationSource {
    /// Accepted hard commit.
    HardCommit(CausalTransactionId),
    /// Accepted runtime-control change.
    RuntimeControl,
    /// Accepted non-hard authority commit.
    AcceptedAuthorityCommit(AuthorityClass),
    /// Explicit maintenance or tooling invalidation.
    Manual,
}

/// Package describing model changes that can affect derived views.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidationPackage {
    source: InvalidationSource,
    changed_authority_classes: BTreeSet<AuthorityClass>,
    changed_store_families: BTreeSet<StoreFamily>,
    affected_views: BTreeSet<DerivedViewKey>,
}

impl InvalidationPackage {
    /// Returns the invalidation source.
    pub const fn source(&self) -> InvalidationSource {
        self.source
    }

    /// Iterates changed authority classes.
    pub fn changed_authority_classes(&self) -> impl Iterator<Item = AuthorityClass> + '_ {
        self.changed_authority_classes.iter().copied()
    }

    /// Iterates changed store families.
    pub fn changed_store_families(&self) -> impl Iterator<Item = StoreFamily> + '_ {
        self.changed_store_families.iter().copied()
    }

    /// Iterates directly affected derived views.
    pub fn affected_views(&self) -> impl Iterator<Item = DerivedViewKey> + '_ {
        self.affected_views.iter().copied()
    }
}

#[cfg(test)]
impl InvalidationPackage {
    pub(crate) fn new(source: InvalidationSource) -> Self {
        Self {
            source,
            changed_authority_classes: BTreeSet::new(),
            changed_store_families: BTreeSet::new(),
            affected_views: BTreeSet::new(),
        }
    }

    pub(crate) fn mark_authority_class(&mut self, authority: AuthorityClass) -> &mut Self {
        self.changed_authority_classes.insert(authority);
        self
    }

    pub(crate) fn mark_store_family(&mut self, family: StoreFamily) -> &mut Self {
        self.changed_store_families.insert(family);
        self
    }

    pub(crate) fn mark_derived_view(&mut self, key: DerivedViewKey) -> &mut Self {
        self.affected_views.insert(key);
        self
    }

    fn touches_read(&self, read: AuthorityRead) -> bool {
        self.changed_authority_classes
            .contains(&read.authority_class())
            || self.changed_store_families.contains(&read.store_family())
    }
}

/// Current staleness state of a derived view.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DerivedViewStatus {
    /// View is valid for the current query epoch.
    Valid,
    /// View depends on changed state.
    Stale,
    /// View should be rebuilt before use.
    NeedsRebuild,
}

/// Derived view registration metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedViewDescriptor {
    key: DerivedViewKey,
    reads: BTreeSet<AuthorityRead>,
    status: DerivedViewStatus,
}

impl DerivedViewDescriptor {
    /// Returns the derived view key.
    pub const fn key(&self) -> DerivedViewKey {
        self.key
    }

    /// Returns the current view status.
    pub const fn status(&self) -> DerivedViewStatus {
        self.status
    }

    /// Iterates authority read labels declared by this view.
    pub fn reads(&self) -> impl Iterator<Item = AuthorityRead> + '_ {
        self.reads.iter().copied()
    }
}

#[cfg(test)]
impl DerivedViewDescriptor {
    pub(crate) fn new(
        key: DerivedViewKey,
        reads: impl IntoIterator<Item = AuthorityRead>,
    ) -> Result<Self, ModelError> {
        let reads = reads.into_iter().collect::<BTreeSet<_>>();
        if reads.is_empty() {
            return Err(ModelError::EmptyItemField {
                type_name: "DerivedViewDescriptor",
                field: "reads",
            });
        }

        Ok(Self {
            key,
            reads,
            status: DerivedViewStatus::Valid,
        })
    }

    pub(crate) fn set_status(&mut self, status: DerivedViewStatus) {
        self.status = status;
    }
}

/// Result of applying invalidation to the derived-view registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DerivedViewInvalidationReport {
    touched_views: usize,
    epoch: QueryEpoch,
}

impl DerivedViewInvalidationReport {
    /// Returns how many registered views changed staleness state.
    pub const fn touched_views(self) -> usize {
        self.touched_views
    }

    /// Returns the registry epoch after invalidation.
    pub const fn epoch(self) -> QueryEpoch {
        self.epoch
    }
}

#[cfg(test)]
impl DerivedViewInvalidationReport {
    fn new(touched_views: usize, epoch: QueryEpoch) -> Self {
        Self {
            touched_views,
            epoch,
        }
    }
}

/// Registry for derived-view dependency metadata and staleness state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DerivedViewRegistry {
    epoch: QueryEpoch,
    views: BTreeMap<DerivedViewKey, DerivedViewDescriptor>,
}

impl DerivedViewRegistry {
    /// Returns a registered derived view descriptor.
    pub fn view(&self, key: DerivedViewKey) -> Option<&DerivedViewDescriptor> {
        self.views.get(&key)
    }

    /// Returns the current registry epoch.
    pub const fn epoch(&self) -> QueryEpoch {
        self.epoch
    }

    /// Returns the number of registered views.
    pub fn len(&self) -> usize {
        self.views.len()
    }

    /// Returns whether no views are registered.
    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }

    /// Iterates registered views in key order.
    pub fn views(&self) -> impl Iterator<Item = &DerivedViewDescriptor> {
        self.views.values()
    }
}

#[cfg(test)]
impl DerivedViewRegistry {
    pub(crate) fn register(&mut self, descriptor: DerivedViewDescriptor) -> Result<(), ModelError> {
        let key = descriptor.key();
        if self.views.contains_key(&key) {
            return Err(ModelError::DuplicateDerivedView { key });
        }

        self.views.insert(key, descriptor);
        Ok(())
    }

    pub(crate) fn apply_invalidation(
        &mut self,
        package: &InvalidationPackage,
    ) -> Result<DerivedViewInvalidationReport, ModelError> {
        let mut touched = 0;

        for (key, view) in &mut self.views {
            let direct = package.affected_views.contains(key);
            let dependency = view.reads.iter().any(|read| package.touches_read(*read));

            let next_status = if direct {
                Some(DerivedViewStatus::NeedsRebuild)
            } else if dependency {
                Some(DerivedViewStatus::Stale)
            } else {
                None
            };

            if let Some(status) = next_status
                && view.status() != status
            {
                view.set_status(status);
                touched += 1;
            }
        }

        if touched > 0 {
            let Some(next) = self.epoch.next() else {
                return Err(ModelError::QueryEpochExhausted);
            };
            self.epoch = next;
        }

        Ok(DerivedViewInvalidationReport::new(touched, self.epoch))
    }
}
