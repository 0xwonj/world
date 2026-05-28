use thiserror::Error;
use world_core::{
    CausalTransactionId, DefinitionId, EntityId, EventRecordId, ProcessInstanceId, ReservationId,
    ScheduledWakeupId,
};
use world_defs::{EffectKind, EventRecordSpec, ResolutionTier, RoleName, StagePermission};
use world_model::{ModelError, ProcessLifecycle, RelationFamily, TransactionCause, WakeupTarget};

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
    /// Process instance id issuer is exhausted.
    #[error("process instance id issuer is exhausted")]
    ProcessInstanceIdExhausted,
    /// Reservation id issuer is exhausted.
    #[error("reservation id issuer is exhausted")]
    ReservationIdExhausted,
    /// Scheduled wakeup id issuer is exhausted.
    #[error("scheduled wakeup id issuer is exhausted")]
    ScheduledWakeupIdExhausted,
    /// Event id appeared more than once in a staged transaction.
    #[error("event {} was emitted more than once in transaction {}", .event.get(), .transaction.get())]
    DuplicateStagedEvent {
        /// Transaction being staged.
        transaction: CausalTransactionId,
        /// Duplicated event id.
        event: EventRecordId,
    },
    /// Process transaction finalization received a non-process transaction cause.
    #[error("process transaction finalizer received non-process cause {cause:?}")]
    InvalidProcessTransactionCause {
        /// Unexpected transaction cause.
        cause: TransactionCause,
    },
    /// Eventless process tick finalization received staged events.
    #[error("eventless process tick transaction staged event records")]
    EventlessProcessTickEmittedEvents,
    /// A process definition is missing from the definition registry.
    #[error("process definition {} is missing", .definition.get())]
    MissingProcessDefinition {
        /// Missing process definition.
        definition: DefinitionId,
    },
    /// A process start request repeated the same role binding.
    #[error("process start request carries duplicate role binding {role}")]
    DuplicateProcessRoleBinding {
        /// Duplicated role.
        role: RoleName,
    },
    /// A process start request supplied a role the process definition does not declare.
    #[error("process start request carries unknown role binding {role}")]
    UnknownProcessRoleBinding {
        /// Unknown role.
        role: RoleName,
    },
    /// A process definition does not support the requested resolution tier.
    #[error("process definition {} does not support resolution {resolution:?}", .definition.get())]
    UnsupportedProcessResolution {
        /// Process definition being started or advanced.
        definition: DefinitionId,
        /// Requested resolution tier.
        resolution: ResolutionTier,
    },
    /// A scheduled process wakeup referenced a process that is not stored.
    #[error("process {} is missing", .process.get())]
    MissingProcess {
        /// Missing process instance.
        process: ProcessInstanceId,
    },
    /// A scheduled wakeup referenced by runtime control is missing.
    #[error("scheduled wakeup {} is missing", .wakeup.get())]
    MissingScheduledWakeup {
        /// Missing scheduled wakeup.
        wakeup: ScheduledWakeupId,
    },
    /// Scheduler cannot dispatch this wakeup target.
    #[error("unsupported wakeup target {target:?}")]
    UnsupportedWakeupTarget {
        /// Unsupported target.
        target: WakeupTarget,
    },
    /// A process cannot accept the requested lifecycle transition.
    #[error("process {} cannot transition from lifecycle {lifecycle:?}", .process.get())]
    InvalidProcessLifecycleTransition {
        /// Process instance being transitioned.
        process: ProcessInstanceId,
        /// Current lifecycle state.
        lifecycle: ProcessLifecycle,
    },
    /// A reservation lifecycle transition requires an active held reservation.
    #[error("reservation {} is not held", .reservation.get())]
    ReservationNotHeld {
        /// Reservation being transitioned.
        reservation: ReservationId,
    },
    /// Model storage rejected an accepted commit package.
    #[error(transparent)]
    Model(#[from] ModelError),
}
