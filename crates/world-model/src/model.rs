use std::collections::BTreeSet;

use crate::{
    AcceptedHardCommit, AppraisalRecordStore, ChronologyStore, DerivedViewDescriptor,
    DerivedViewKey, DerivedViewRegistry, EpistemicStore, EventHistoryStore, HardCommitApplication,
    HardStateChange, ModelError, QueryLayer, RelationKey, RelationStore, RuntimeControlApplication,
    RuntimeControlStore, SocialInstitutionalStore, WorldStore,
};
#[cfg(test)]
use crate::{
    AppraisalRecord, ChronologyRecord, DerivedViewInvalidationReport, EntitySnapshot,
    EpistemicRecord, EventRecord, InvalidationPackage, RelationRecord, RuntimeControlRecord,
    SocialRecord, TransactionRecord,
};
use crate::{
    history::EventHistoryAppendPlan, invalidation::DerivedViewInvalidationPlan,
    runtime_control::RuntimeControlChangeApplyPlan,
};
#[cfg(test)]
use world_core::StoreCursor;

/// Root owner of materialized model state and read surfaces.
#[derive(Debug, Default)]
pub struct WorldModel {
    world: WorldStore,
    relations: RelationStore,
    event_history: EventHistoryStore,
    runtime_control: RuntimeControlStore,
    social: SocialInstitutionalStore,
    chronology: ChronologyStore,
    epistemic: EpistemicStore,
    appraisal: AppraisalRecordStore,
    derived_views: DerivedViewRegistry,
}

impl WorldModel {
    /// Creates an empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current hard world store.
    pub const fn world_store(&self) -> &WorldStore {
        &self.world
    }

    /// Returns the relation store.
    pub const fn relation_store(&self) -> &RelationStore {
        &self.relations
    }

    /// Returns the committed event-history store.
    pub const fn event_history(&self) -> &EventHistoryStore {
        &self.event_history
    }

    /// Returns the runtime-control store.
    pub const fn runtime_control_store(&self) -> &RuntimeControlStore {
        &self.runtime_control
    }

    /// Returns the social/institutional store.
    pub const fn social_store(&self) -> &SocialInstitutionalStore {
        &self.social
    }

    /// Returns the chronology store.
    pub const fn chronology_store(&self) -> &ChronologyStore {
        &self.chronology
    }

    /// Returns the epistemic store.
    pub const fn epistemic_store(&self) -> &EpistemicStore {
        &self.epistemic
    }

    /// Returns the appraisal store.
    pub const fn appraisal_store(&self) -> &AppraisalRecordStore {
        &self.appraisal
    }

    /// Returns the derived-view registry.
    pub const fn derived_view_registry(&self) -> &DerivedViewRegistry {
        &self.derived_views
    }

    /// Returns model read surfaces.
    pub const fn query_layer(&self) -> QueryLayer<'_> {
        QueryLayer::new(self)
    }

    /// Returns a derived view descriptor.
    pub fn derived_view(&self, key: DerivedViewKey) -> Option<&DerivedViewDescriptor> {
        self.derived_views.view(key)
    }

    /// Applies a hard commit package accepted by the causal runtime.
    ///
    /// This is the model-side receiver for accepted packages. It verifies
    /// storage invariants and applies the package atomically; it does not grant
    /// callers causal transaction authority.
    pub fn apply_hard_commit(
        &mut self,
        commit: AcceptedHardCommit,
    ) -> Result<HardCommitApplication, ModelError> {
        let plan = self.plan_hard_commit(&commit)?;
        Ok(self.apply_planned_hard_commit(commit, plan))
    }

    /// Applies an accepted runtime-control update package.
    ///
    /// This is the model-side receiver for runtime-control packages produced by
    /// runtime authority. General callers should schedule, start, wait, resume,
    /// or acknowledge through runtime APIs instead of calling this directly.
    pub fn apply_runtime_control_update(
        &mut self,
        update: crate::AcceptedRuntimeControlUpdate,
    ) -> Result<RuntimeControlApplication, ModelError> {
        let control = self.runtime_control.plan_control_update(&update)?;
        let derived_views = self
            .derived_views
            .plan_invalidation(update.invalidation())?;
        let invalidation_package = update.invalidation().clone();
        let (update_cursor, changed_records) = self
            .runtime_control
            .apply_planned_control_update(update, control);
        let invalidation = self
            .derived_views
            .apply_planned_invalidation(&invalidation_package, derived_views);

        Ok(RuntimeControlApplication::new(
            update_cursor,
            changed_records,
            invalidation,
        ))
    }

    fn plan_hard_commit(
        &self,
        commit: &AcceptedHardCommit,
    ) -> Result<HardCommitApplyPlan, ModelError> {
        for change in commit.changes() {
            change.validate_hard_authority()?;
        }
        validate_hard_commit_invalidation(commit)?;
        validate_hard_commit_changes(self, commit)?;

        let event_ids = commit
            .events()
            .iter()
            .map(|event| event.id())
            .collect::<Vec<_>>();
        let history = self
            .event_history
            .plan_append(commit.transaction().id(), &event_ids)?;
        let runtime_control = if commit.control_changes().is_empty() {
            None
        } else {
            Some(
                self.runtime_control
                    .plan_transaction_coupled_changes(commit.control_changes())?,
            )
        };
        let derived_views = self
            .derived_views
            .plan_invalidation(commit.invalidation())?;

        Ok(HardCommitApplyPlan {
            history,
            runtime_control,
            derived_views,
        })
    }

    fn apply_planned_hard_commit(
        &mut self,
        commit: AcceptedHardCommit,
        plan: HardCommitApplyPlan,
    ) -> HardCommitApplication {
        let (transaction, events, changes, _control_changes, invalidation) = commit.into_parts();
        let transaction_id = transaction.id();
        let event_records = events
            .into_iter()
            .map(|event| event.into_record(transaction_id))
            .collect::<Vec<_>>();
        self.event_history
            .append_planned(transaction.into_record(), event_records, &plan.history);

        for change in changes {
            match change {
                HardStateChange::InsertEntity {
                    entity,
                    runtime_handle,
                    provenance,
                } => {
                    self.world.insert_planned(crate::EntitySnapshot::new(
                        entity,
                        runtime_handle,
                        provenance,
                    ));
                }
                HardStateChange::InsertRelation {
                    subject,
                    family,
                    object,
                    provenance,
                } => {
                    self.relations.insert_planned(crate::RelationRecord::new(
                        subject, family, object, provenance,
                    ));
                }
            }
        }

        if let Some(runtime_control) = plan.runtime_control {
            self.runtime_control
                .apply_planned_transaction_changes(runtime_control);
        }

        let invalidation = self
            .derived_views
            .apply_planned_invalidation(&invalidation, plan.derived_views);
        HardCommitApplication::new(
            plan.history.transaction_cursor(),
            plan.history.event_cursors().to_vec(),
            invalidation,
        )
    }
}

struct HardCommitApplyPlan {
    history: EventHistoryAppendPlan,
    runtime_control: Option<RuntimeControlChangeApplyPlan>,
    derived_views: DerivedViewInvalidationPlan,
}

fn validate_hard_commit_invalidation(commit: &AcceptedHardCommit) -> Result<(), ModelError> {
    let transaction = commit.transaction().id();
    let invalidation = commit.invalidation();

    if !invalidation.contains_authority_class(world_core::AuthorityClass::Hard) {
        return Err(ModelError::MissingHardCommitAuthorityInvalidation {
            transaction,
            authority: world_core::AuthorityClass::Hard,
        });
    }

    if !invalidation.contains_store_family(crate::StoreFamily::EventHistory) {
        return Err(ModelError::MissingHardCommitStoreInvalidation {
            transaction,
            store: crate::StoreFamily::EventHistory,
        });
    }

    if !commit.control_changes().is_empty() {
        if !invalidation.contains_authority_class(world_core::AuthorityClass::RuntimeControl) {
            return Err(ModelError::MissingHardCommitAuthorityInvalidation {
                transaction,
                authority: world_core::AuthorityClass::RuntimeControl,
            });
        }

        if !invalidation.contains_store_family(crate::StoreFamily::RuntimeControl) {
            return Err(ModelError::MissingHardCommitStoreInvalidation {
                transaction,
                store: crate::StoreFamily::RuntimeControl,
            });
        }
    }

    for change in commit.changes() {
        let store = change.changed_store_family();
        if !invalidation.contains_store_family(store) {
            return Err(ModelError::MissingHardCommitStoreInvalidation { transaction, store });
        }
    }

    Ok(())
}

fn validate_hard_commit_changes(
    model: &WorldModel,
    commit: &AcceptedHardCommit,
) -> Result<(), ModelError> {
    let mut inserted_entities = BTreeSet::new();
    let mut inserted_relations = BTreeSet::new();

    for change in commit.changes() {
        match change {
            HardStateChange::InsertEntity { entity, .. } => {
                if model.world.contains_entity(*entity) || !inserted_entities.insert(*entity) {
                    return Err(ModelError::DuplicateEntity { entity: *entity });
                }
            }
            HardStateChange::InsertRelation {
                subject,
                family,
                object,
                ..
            } => {
                let key = RelationKey::new(*subject, *family, *object);
                if model.relations.contains(key) || !inserted_relations.insert(key) {
                    return Err(ModelError::DuplicateRelation {
                        subject: *subject,
                        family: *family,
                        object: *object,
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
impl WorldModel {
    pub(crate) fn insert_entity(&mut self, snapshot: EntitySnapshot) -> Result<(), ModelError> {
        self.world.insert(snapshot)
    }

    pub(crate) fn insert_relation(&mut self, record: RelationRecord) -> Result<(), ModelError> {
        self.relations.insert(record)
    }

    pub(crate) fn append_transaction(
        &mut self,
        record: TransactionRecord,
    ) -> Result<StoreCursor, ModelError> {
        self.event_history.append_transaction(record)
    }

    pub(crate) fn append_event(&mut self, record: EventRecord) -> Result<StoreCursor, ModelError> {
        self.event_history.append_event(record)
    }

    pub(crate) fn insert_runtime_control(
        &mut self,
        record: RuntimeControlRecord,
    ) -> Result<(), ModelError> {
        self.runtime_control.insert(record)
    }

    pub(crate) fn insert_social_record(&mut self, record: SocialRecord) -> Result<(), ModelError> {
        self.social.insert(record)
    }

    pub(crate) fn insert_chronology_record(
        &mut self,
        record: ChronologyRecord,
    ) -> Result<(), ModelError> {
        self.chronology.insert(record)
    }

    pub(crate) fn insert_epistemic_record(
        &mut self,
        record: EpistemicRecord,
    ) -> Result<(), ModelError> {
        self.epistemic.insert(record)
    }

    pub(crate) fn insert_appraisal_record(
        &mut self,
        record: AppraisalRecord,
    ) -> Result<(), ModelError> {
        self.appraisal.insert(record)
    }

    pub(crate) fn register_derived_view(
        &mut self,
        descriptor: DerivedViewDescriptor,
    ) -> Result<(), ModelError> {
        self.derived_views.register(descriptor)
    }

    pub(crate) fn apply_invalidation(
        &mut self,
        package: &InvalidationPackage,
    ) -> Result<DerivedViewInvalidationReport, ModelError> {
        self.derived_views.apply_invalidation(package)
    }
}
