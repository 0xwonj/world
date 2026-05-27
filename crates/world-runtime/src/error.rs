use thiserror::Error;
use world_core::{CausalTransactionId, DefinitionId, EntityId, EventRecordId};
use world_defs::{EffectKind, EventRecordSpec, RoleName, StagePermission};
use world_model::{ModelError, RelationFamily};

/// Error returned by runtime infrastructure while executing causal work.
#[non_exhaustive]
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RuntimeError {
    /// A checked action referenced an effect program missing from the registry.
    #[error(
        "action {} references missing effect program {}",
        .action.get(),
        .effect_program.get()
    )]
    MissingEffectProgram {
        /// Action definition being executed.
        action: DefinitionId,
        /// Missing effect program.
        effect_program: DefinitionId,
    },
    /// An effect operation had no runtime-owned handler.
    #[error("no runtime handler is registered for effect {kind}")]
    MissingEffectHandler {
        /// Effect operation family.
        kind: EffectKind,
    },
    /// An effect handler required a permission the operation did not declare.
    #[error("effect {operation} did not declare required permission {permission:?}")]
    PermissionNotDeclared {
        /// Effect operation family.
        operation: EffectKind,
        /// Missing stage permission.
        permission: StagePermission,
    },
    /// An effect handler required a role that binding did not provide.
    #[error("bound request is missing role {role}")]
    MissingBoundRole {
        /// Missing role.
        role: RoleName,
    },
    /// A built-in runtime handler declared an invalid static role name.
    #[error("built-in runtime handler used invalid role name {name}")]
    InvalidStaticRole {
        /// Invalid role name.
        name: &'static str,
    },
    /// An effect handler could not see an entity needed after validation.
    #[error("visible transaction state is missing entity {} for role {role}", .entity.get())]
    MissingVisibleEntity {
        /// Role whose entity was missing.
        role: RoleName,
        /// Missing entity.
        entity: EntityId,
    },
    /// An effect handler attempted to insert an entity that is already visible.
    #[error("visible transaction state already contains entity {} for role {role}", .entity.get())]
    DuplicateVisibleEntity {
        /// Role whose entity already exists.
        role: RoleName,
        /// Existing entity.
        entity: EntityId,
    },
    /// An effect handler attempted to insert a relation that is already visible.
    #[error(
        "visible transaction state already contains relation {family:?} from entity {} to entity {}",
        .subject.get(),
        .object.get()
    )]
    DuplicateVisibleRelation {
        /// Relation subject.
        subject: EntityId,
        /// Relation family.
        family: RelationFamily,
        /// Relation object.
        object: EntityId,
    },
    /// Required event contract was not satisfied by staged events.
    #[error("effect program {} did not emit required event {event}", .effect_program.get())]
    RequiredEventMissing {
        /// Effect program being finalized.
        effect_program: DefinitionId,
        /// Missing event.
        event: EventRecordSpec,
    },
    /// An effect handler attempted to emit an event not declared by its operation.
    #[error("effect {operation} attempted to emit undeclared event {event}")]
    EventNotDeclaredForOperation {
        /// Effect operation family.
        operation: EffectKind,
        /// Undeclared event.
        event: EventRecordSpec,
    },
    /// Transaction id issuer is exhausted.
    #[error("causal transaction id issuer is exhausted")]
    TransactionIdExhausted,
    /// Event id issuer is exhausted.
    #[error("event record id issuer is exhausted")]
    EventIdExhausted,
    /// Event id appeared more than once in a staged transaction.
    #[error("event {} was emitted more than once in transaction {}", .event.get(), .transaction.get())]
    DuplicateStagedEvent {
        /// Transaction being staged.
        transaction: CausalTransactionId,
        /// Duplicated event id.
        event: EventRecordId,
    },
    /// Model storage rejected an accepted commit package.
    #[error(transparent)]
    Model(#[from] ModelError),
}
