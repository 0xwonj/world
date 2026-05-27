use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;

use world_core::{AuthorityClass, CausalTransactionId, InvalidCoreValue, QueryEpoch};

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

    /// Returns whether this package marks an authority class as changed.
    pub fn contains_authority_class(&self, authority: AuthorityClass) -> bool {
        self.changed_authority_classes.contains(&authority)
    }

    /// Iterates changed store families.
    pub fn changed_store_families(&self) -> impl Iterator<Item = StoreFamily> + '_ {
        self.changed_store_families.iter().copied()
    }

    /// Returns whether this package marks a store family as changed.
    pub fn contains_store_family(&self, family: StoreFamily) -> bool {
        self.changed_store_families.contains(&family)
    }

    /// Iterates directly affected derived views.
    pub fn affected_views(&self) -> impl Iterator<Item = DerivedViewKey> + '_ {
        self.affected_views.iter().copied()
    }
}

impl InvalidationPackage {
    /// Creates an empty invalidation package for an accepted authority update.
    pub fn new(source: InvalidationSource) -> Self {
        Self {
            source,
            changed_authority_classes: BTreeSet::new(),
            changed_store_families: BTreeSet::new(),
            affected_views: BTreeSet::new(),
        }
    }

    /// Marks an authority class as changed.
    pub fn mark_authority_class(&mut self, authority: AuthorityClass) -> &mut Self {
        self.changed_authority_classes.insert(authority);
        self
    }

    /// Marks a store family as changed.
    pub fn mark_store_family(&mut self, family: StoreFamily) -> &mut Self {
        self.changed_store_families.insert(family);
        self
    }

    /// Marks a derived view as directly affected.
    pub fn mark_derived_view(&mut self, key: DerivedViewKey) -> &mut Self {
        self.affected_views.insert(key);
        self
    }

    pub(crate) fn touches_read(&self, read: AuthorityRead) -> bool {
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
}

impl DerivedViewDescriptor {
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

impl DerivedViewInvalidationReport {
    fn new(touched_views: usize, epoch: QueryEpoch) -> Self {
        Self {
            touched_views,
            epoch,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DerivedViewInvalidationPlan {
    touched_views: usize,
    epoch: QueryEpoch,
}

impl DerivedViewInvalidationPlan {
    fn new(touched_views: usize, epoch: QueryEpoch) -> Self {
        Self {
            touched_views,
            epoch,
        }
    }

    fn report(self) -> DerivedViewInvalidationReport {
        DerivedViewInvalidationReport::new(self.touched_views, self.epoch)
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

impl DerivedViewRegistry {
    #[cfg(test)]
    pub(crate) fn register(&mut self, descriptor: DerivedViewDescriptor) -> Result<(), ModelError> {
        let key = descriptor.key();
        if self.views.contains_key(&key) {
            return Err(ModelError::DuplicateDerivedView { key });
        }

        self.views.insert(key, descriptor);
        Ok(())
    }

    pub(crate) fn plan_invalidation(
        &self,
        package: &InvalidationPackage,
    ) -> Result<DerivedViewInvalidationPlan, ModelError> {
        let touched = self
            .views
            .iter()
            .filter(|(key, view)| {
                planned_status(package, key, view).is_some_and(|status| view.status() != status)
            })
            .count();

        let epoch = if touched > 0 {
            self.epoch.next().ok_or(ModelError::QueryEpochExhausted)?
        } else {
            self.epoch
        };

        Ok(DerivedViewInvalidationPlan::new(touched, epoch))
    }

    pub(crate) fn apply_planned_invalidation(
        &mut self,
        package: &InvalidationPackage,
        plan: DerivedViewInvalidationPlan,
    ) -> DerivedViewInvalidationReport {
        let mut touched = 0;
        for (key, view) in &mut self.views {
            if let Some(status) = planned_status(package, key, view)
                && view.status() != status
            {
                view.set_status(status);
                touched += 1;
            }
        }

        debug_assert_eq!(touched, plan.touched_views);
        self.epoch = plan.epoch;

        plan.report()
    }

    #[cfg(test)]
    pub(crate) fn apply_invalidation(
        &mut self,
        package: &InvalidationPackage,
    ) -> Result<DerivedViewInvalidationReport, ModelError> {
        let plan = self.plan_invalidation(package)?;
        Ok(self.apply_planned_invalidation(package, plan))
    }
}

fn planned_status(
    package: &InvalidationPackage,
    key: &DerivedViewKey,
    view: &DerivedViewDescriptor,
) -> Option<DerivedViewStatus> {
    let direct = package.affected_views.contains(key);
    let dependency = view.reads.iter().any(|read| package.touches_read(*read));

    if direct {
        Some(DerivedViewStatus::NeedsRebuild)
    } else if dependency {
        Some(DerivedViewStatus::Stale)
    } else {
        None
    }
}
