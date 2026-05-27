use thiserror::Error;
use world_core::{AuthorityClass, CausalTransactionId, EntityId, EventRecordId};
use world_defs::RoleName;

use crate::{
    AcceptedRecordId, DerivedViewKey, InvalidationSource, RelationFamily, RuntimeControlRecordKind,
    StoreFamily,
};

/// Error returned when model storage or query-surface invariants are violated.
#[non_exhaustive]
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ModelError {
    /// A store already contains the entity.
    #[error("entity {} is already present in the world store", .entity.get())]
    DuplicateEntity {
        /// Duplicated entity.
        entity: EntityId,
    },
    /// A relation store already contains the relation key.
    #[error(
        "relation {family:?} from entity {} to entity {} is already present",
        .subject.get(),
        .object.get()
    )]
    DuplicateRelation {
        /// Relation subject.
        subject: EntityId,
        /// Relation family.
        family: RelationFamily,
        /// Relation object.
        object: EntityId,
    },
    /// A hard commit package attempted to write a non-hard relation family.
    #[error(
        "non-hard relation {family:?} from entity {} to entity {} cannot be applied by a hard commit",
        .subject.get(),
        .object.get()
    )]
    NonHardRelationInHardCommit {
        /// Relation subject.
        subject: EntityId,
        /// Relation family.
        family: RelationFamily,
        /// Relation object.
        object: EntityId,
    },
    /// A transaction id is already present in event history.
    #[error("transaction {} is already present in event history", .transaction.get())]
    DuplicateTransaction {
        /// Duplicated transaction.
        transaction: CausalTransactionId,
    },
    /// An event references a transaction that is not present in event history.
    #[error("event references missing transaction {}", .transaction.get())]
    MissingTransaction {
        /// Missing transaction.
        transaction: CausalTransactionId,
    },
    /// An event id is already present in event history.
    #[error("event {} is already present in event history", .event.get())]
    DuplicateEvent {
        /// Duplicated event.
        event: EventRecordId,
    },
    /// The append-only store cursor cannot advance.
    #[error("store cursor is exhausted")]
    StoreCursorExhausted,
    /// A hard commit package carried an invalidation source for another authority path.
    #[error(
        "hard commit {} cannot use invalidation source {invalidation_source:?}",
        .transaction.get()
    )]
    InvalidHardCommitInvalidation {
        /// Transaction being applied.
        transaction: CausalTransactionId,
        /// Invalid invalidation source.
        invalidation_source: InvalidationSource,
    },
    /// A hard commit did not mark a required changed authority class.
    #[error(
        "hard commit {} did not invalidate changed authority {authority:?}",
        .transaction.get()
    )]
    MissingHardCommitAuthorityInvalidation {
        /// Transaction being applied.
        transaction: CausalTransactionId,
        /// Missing authority class.
        authority: AuthorityClass,
    },
    /// A hard commit did not mark a required changed store family.
    #[error(
        "hard commit {} did not invalidate changed store {store:?}",
        .transaction.get()
    )]
    MissingHardCommitStoreInvalidation {
        /// Transaction being applied.
        transaction: CausalTransactionId,
        /// Missing store family.
        store: StoreFamily,
    },
    /// A committed event role binding repeated the same role name.
    #[error("event {} carries duplicate role binding {role}", .event.get())]
    DuplicateEventRoleBinding {
        /// Event being applied.
        event: EventRecordId,
        /// Duplicated role.
        role: RoleName,
    },
    /// A committed event is missing a role required by its event record spec.
    #[error("event {} is missing required role binding {role}", .event.get())]
    MissingEventRoleBinding {
        /// Event being applied.
        event: EventRecordId,
        /// Missing role.
        role: RoleName,
    },
    /// A runtime-control record key is already present.
    #[error("runtime-control record {kind:?} is already present")]
    DuplicateRuntimeControlRecord {
        /// Duplicated runtime-control record key.
        kind: RuntimeControlRecordKind,
    },
    /// An accepted record id is already present in its store.
    #[error("accepted record {} is already present", .record.get())]
    DuplicateAcceptedRecord {
        /// Duplicated record id.
        record: AcceptedRecordId,
    },
    /// A derived view key is already registered.
    #[error("derived view {} is already registered", .key.get())]
    DuplicateDerivedView {
        /// Duplicated derived view key.
        key: DerivedViewKey,
    },
    /// A derived view key is not registered.
    #[error("derived view {} is not registered", .key.get())]
    UnknownDerivedView {
        /// Missing derived view key.
        key: DerivedViewKey,
    },
    /// A required collection was empty.
    #[error("{type_name} has empty required field {field}")]
    EmptyItemField {
        /// Type that rejected the empty field.
        type_name: &'static str,
        /// Field that was empty.
        field: &'static str,
    },
    /// A query epoch cannot advance.
    #[error("query epoch is exhausted")]
    QueryEpochExhausted,
}
