#[cfg(test)]
use crate::{
    AppraisalRecord, ChronologyRecord, DerivedViewInvalidationReport, EntitySnapshot,
    EpistemicRecord, EventRecord, InvalidationPackage, ModelError, RelationRecord,
    RuntimeControlRecord, SocialRecord, TransactionRecord,
};
use crate::{
    AppraisalRecordStore, ChronologyStore, DerivedViewDescriptor, DerivedViewKey,
    DerivedViewRegistry, EpistemicStore, EventHistoryStore, QueryLayer, RelationStore,
    RuntimeControlStore, SocialInstitutionalStore, WorldStore,
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
