//! Authoritative state storage and query surface crate.

mod error;
mod history;
mod invalidation;
mod model;
mod query;
mod records;
mod relations;
mod runtime_control;
mod store;

pub use error::ModelError;
pub use history::{
    EventHistoryStore, EventRecord, StoredEventRecord, StoredTransactionRecord, TransactionRecord,
};
pub use invalidation::{
    DerivedViewDescriptor, DerivedViewInvalidationReport, DerivedViewKey, DerivedViewRegistry,
    DerivedViewStatus, InvalidationPackage, InvalidationSource,
};
pub use model::WorldModel;
pub use query::{ActorRelativeQuery, DebugQuery, KernelQuery, QueryLayer, SemanticContextQuery};
pub use records::{
    AcceptedRecordId, AppraisalRecord, AppraisalRecordStore, ChronologyRecord, ChronologyStore,
    EpistemicHolder, EpistemicRecord, EpistemicStore, SocialInstitutionalStore, SocialRecord,
};
pub use relations::{RelationFamily, RelationKey, RelationRecord, RelationStore};
pub use runtime_control::{RuntimeControlRecord, RuntimeControlRecordKind, RuntimeControlStore};
pub use store::{AuthorityRead, EntitySnapshot, StoreFamily, WorldStore};

#[cfg(test)]
mod tests;
