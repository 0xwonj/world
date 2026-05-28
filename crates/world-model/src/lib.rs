//! Authoritative state storage and query surface crate.

mod commit;
mod error;
mod history;
mod invalidation;
mod model;
mod query;
mod records;
mod relations;
mod runtime_control;
mod store;

pub use commit::{
    AcceptedHardCommit, EventCommit, HardCommitApplication, HardStateChange, TransactionCommit,
};
pub use error::ModelError;
pub use history::{
    EventHistoryStore, EventRecord, EventRoleBinding, StoredEventRecord, StoredTransactionRecord,
    TransactionCause, TransactionRecord,
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
pub use runtime_control::{
    AcceptedRuntimeControlUpdate, InterruptReason, PauseReason, ProcessFailureReason,
    ProcessInstanceInit, ProcessInstanceRecord, ProcessLifecycle, ProcessProgress,
    ProcessRoleBinding, ProcessStateSnapshot, ProcessStateValue, ProcessWork,
    ReservationCancelReason, ReservationHolder, ReservationRecord, ReservationState,
    ReservationTarget, ReservationTransition, RuntimeControlApplication, RuntimeControlChange,
    RuntimeControlRecord, RuntimeControlRecordKind, RuntimeControlRecordPayload,
    RuntimeControlSource, RuntimeControlStore, RuntimeControlUpdateHeader,
    RuntimeControlUpdateRecord, ScheduledWakeupRecord, ScheduledWakeupStatus, StaleWakeupReason,
    StoredRuntimeControlUpdate, WaitCondition, WakeupCancellationReason, WakeupConsumptionReason,
    WakeupTarget, WakeupTerminalTransition,
};
pub use store::{AuthorityRead, EntitySnapshot, StoreFamily, WorldStore};

#[cfg(test)]
mod tests;
