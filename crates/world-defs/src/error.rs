use thiserror::Error;
use world_core::{DefinitionId, ReplayLevel};

use crate::effects::{EffectArgKind, EffectParamKind, EffectPrimitiveId, StagePermission};
use crate::events::EventRecordSpec;
use crate::keys::{DefinitionName, EffectParamName, RoleName, StateFieldName};
use crate::processes::ResolutionTier;
use crate::semantics::{SemanticDeclarationKind, SemanticOutputKind};

/// Error returned when checked definitions violate local or registry invariants.
#[non_exhaustive]
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DefinitionError {
    /// A field that must have at least one value was empty.
    #[error("{type_name} {} has empty required field {field}", .definition.get())]
    EmptyDefinitionField {
        /// Definition that owns the invalid field.
        definition: DefinitionId,
        /// Type that rejected the field.
        type_name: &'static str,
        /// Field that was empty.
        field: &'static str,
    },
    /// A registry contains the same definition id more than once.
    #[error("definition id {} is declared more than once", .id.get())]
    DuplicateDefinitionId {
        /// Duplicated definition id.
        id: DefinitionId,
    },
    /// A registry contains the same primitive name more than once.
    #[error("effect primitive name {name} is declared more than once")]
    DuplicatePrimitiveName {
        /// Duplicated primitive name.
        name: DefinitionName,
    },
    /// A primitive definition contains the same parameter name more than once.
    #[error("effect primitive {primitive} declares parameter {param} more than once")]
    DuplicateEffectParam {
        /// Primitive with the duplicated parameter.
        primitive: EffectPrimitiveId,
        /// Duplicated parameter.
        param: EffectParamName,
    },
    /// An operation binds the same primitive parameter more than once.
    #[error("effect primitive {primitive} argument {param} is bound more than once")]
    DuplicateEffectArg {
        /// Invoked primitive.
        primitive: EffectPrimitiveId,
        /// Duplicated argument parameter.
        param: EffectParamName,
    },
    /// An operation declares the same emitted event more than once.
    #[error("effect primitive {primitive} declares emitted event {event} more than once")]
    DuplicateEffectEvent {
        /// Invoked primitive.
        primitive: EffectPrimitiveId,
        /// Duplicated event.
        event: EventRecordSpec,
    },
    /// A definition contains the same role name more than once.
    #[error("definition {} declares role {role} more than once", .definition.get())]
    DuplicateRole {
        /// Definition that declared the duplicate role.
        definition: DefinitionId,
        /// Duplicated role.
        role: RoleName,
    },
    /// A process state schema contains the same field name more than once.
    #[error("process state field {field} is declared more than once")]
    DuplicateStateField {
        /// Duplicated state field.
        field: StateFieldName,
    },
    /// A process definition declares the same resolution more than once.
    #[error("process definition {} declares resolution support for {resolution:?} more than once", .definition.get())]
    DuplicateResolutionSupport {
        /// Definition that declared the duplicate resolution support.
        definition: DefinitionId,
        /// Duplicated resolution.
        resolution: ResolutionTier,
    },
    /// A requirement or binding rule references a role the definition did not declare.
    #[error("definition {} references undeclared role {role}", .definition.get())]
    UnknownRole {
        /// Definition that contains the dangling role reference.
        definition: DefinitionId,
        /// Missing role.
        role: RoleName,
    },
    /// An action or process references an effect program missing from the registry.
    #[error("definition {} references missing effect program {}", .definition.get(), .effect_program.get())]
    MissingEffectProgram {
        /// Definition that references the effect program.
        definition: DefinitionId,
        /// Missing effect program id.
        effect_program: DefinitionId,
    },
    /// An effect operation references a primitive missing from the registry.
    #[error("definition {} references missing primitive {}", .definition.get(), .primitive)]
    MissingEffectPrimitive {
        /// Definition that owns the effect program.
        definition: DefinitionId,
        /// Effect program that contains the operation.
        effect_program: DefinitionId,
        /// Missing primitive id.
        primitive: EffectPrimitiveId,
    },
    /// An effect operation does not bind a required primitive parameter.
    #[error("definition {} effect program {} primitive {} is missing argument {param}", .definition.get(), .effect_program.get(), .primitive)]
    MissingEffectArg {
        /// Definition that owns the effect program.
        definition: DefinitionId,
        /// Effect program that contains the operation.
        effect_program: DefinitionId,
        /// Invoked primitive.
        primitive: EffectPrimitiveId,
        /// Missing argument parameter.
        param: EffectParamName,
    },
    /// An effect operation binds a parameter the primitive does not declare.
    #[error("definition {} effect program {} primitive {} has unknown argument {param}", .definition.get(), .effect_program.get(), .primitive)]
    UnknownEffectArg {
        /// Definition that owns the effect program.
        definition: DefinitionId,
        /// Effect program that contains the operation.
        effect_program: DefinitionId,
        /// Invoked primitive.
        primitive: EffectPrimitiveId,
        /// Unknown argument parameter.
        param: EffectParamName,
    },
    /// An effect operation binds an argument whose kind cannot satisfy its parameter.
    #[error("definition {} effect program {} primitive {} argument {param} has kind {actual:?}, expected {expected:?}", .definition.get(), .effect_program.get(), .primitive)]
    EffectArgKindMismatch {
        /// Definition that owns the effect program.
        definition: DefinitionId,
        /// Effect program that contains the operation.
        effect_program: DefinitionId,
        /// Invoked primitive.
        primitive: EffectPrimitiveId,
        /// Bound parameter.
        param: EffectParamName,
        /// Expected parameter kind.
        expected: EffectParamKind,
        /// Actual argument kind.
        actual: EffectArgKind,
    },
    /// A definition references an effect program but does not declare one of its permissions.
    #[error("definition {} references effect program {} without declaring permission {permission:?}", .definition.get(), .effect_program.get())]
    PermissionNotDeclared {
        /// Definition that references the effect program.
        definition: DefinitionId,
        /// Referenced effect program id.
        effect_program: DefinitionId,
        /// Missing permission.
        permission: StagePermission,
    },
    /// A definition requires an event that its referenced effect programs cannot emit.
    #[error("definition {} requires event {event} that its effect programs cannot emit", .definition.get())]
    RequiredEventUnavailable {
        /// Definition that requires the event.
        definition: DefinitionId,
        /// Required event.
        event: EventRecordSpec,
    },
    /// An effect program requires an event that none of its operations emit.
    #[error("effect program {} requires event {event} that no operation emits", .definition.get())]
    RequiredEventNotEmitted {
        /// Effect program that requires the event.
        definition: DefinitionId,
        /// Required event.
        event: EventRecordSpec,
    },
    /// An action or process did not declare an event required by a referenced effect program.
    #[error("definition {} does not declare required event {event}", .definition.get())]
    RequiredEventNotDeclared {
        /// Definition that references the effect program.
        definition: DefinitionId,
        /// Required event missing from the definition contract.
        event: EventRecordSpec,
    },
    /// An operation emits an event that its definition contract does not permit.
    #[error("definition {} can emit event {event} that is not permitted by its contract", .definition.get())]
    EventNotPermittedByContract {
        /// Definition that owns the operation or referenced effect program.
        definition: DefinitionId,
        /// Emitted event missing from the contract.
        event: EventRecordSpec,
    },
    /// A primitive requires an operation to emit an event but the operation does not.
    #[error("definition {} effect program {} primitive {} requires event {event}", .definition.get(), .effect_program.get(), .primitive)]
    PrimitiveRequiredEventNotEmitted {
        /// Definition that owns the effect program.
        definition: DefinitionId,
        /// Effect program that contains the operation.
        effect_program: DefinitionId,
        /// Invoked primitive.
        primitive: EffectPrimitiveId,
        /// Required event.
        event: EventRecordSpec,
    },
    /// An effect program does not require an event required by one of its primitives.
    #[error("effect program {} invokes primitive {} without requiring event {event}", .effect_program.get(), .primitive)]
    PrimitiveRequiredEventNotDeclared {
        /// Effect program that contains the operation.
        effect_program: DefinitionId,
        /// Invoked primitive.
        primitive: EffectPrimitiveId,
        /// Required event missing from the program contract.
        event: EventRecordSpec,
    },
    /// An operation emits an event not permitted by the primitive definition.
    #[error("definition {} effect program {} primitive {} cannot emit event {event}", .definition.get(), .effect_program.get(), .primitive)]
    OperationEventNotPermittedByPrimitive {
        /// Definition that owns the effect program.
        definition: DefinitionId,
        /// Effect program that contains the operation.
        effect_program: DefinitionId,
        /// Invoked primitive.
        primitive: EffectPrimitiveId,
        /// Event missing from the primitive contract.
        event: EventRecordSpec,
    },
    /// An effect program declares weaker replay support than one of its primitives.
    #[error("effect program {} declares replay level {program_replay:?}, weaker than primitive {} replay level {primitive_replay:?}", .effect_program.get(), .primitive)]
    EffectProgramReplayTooWeak {
        /// Effect program that contains the operation.
        effect_program: DefinitionId,
        /// Invoked primitive.
        primitive: EffectPrimitiveId,
        /// Program replay level.
        program_replay: ReplayLevel,
        /// Primitive replay level.
        primitive_replay: ReplayLevel,
    },
    /// A checked item that is not definition-scoped has an empty field.
    #[error("{type_name} has empty required field {field}")]
    EmptyItemField {
        /// Type that rejected the field.
        type_name: &'static str,
        /// Field that was empty.
        field: &'static str,
    },
    /// A primitive with mutating permissions did not declare any event contract.
    #[error("effect primitive {primitive} requires at least one event contract")]
    PrimitiveRequiresEvent {
        /// Primitive that requires event evidence.
        primitive: EffectPrimitiveId,
    },
    /// A primitive declared event contracts without event-emission permission.
    #[error("effect primitive {primitive} declares events without event-emission permission")]
    PrimitiveEventPermissionNotDeclared {
        /// Primitive that declared events.
        primitive: EffectPrimitiveId,
    },
    /// A semantic declaration tries to output a semantic family its kind cannot own.
    #[error("semantic declaration {} of kind {kind:?} cannot output {output:?}", .definition.get())]
    ForbiddenSemanticOutput {
        /// Semantic declaration that requested the output.
        definition: DefinitionId,
        /// Declaration kind that rejected the output.
        kind: SemanticDeclarationKind,
        /// Forbidden semantic output.
        output: SemanticOutputKind,
    },
}

pub(crate) fn empty_definition_field(
    definition: DefinitionId,
    type_name: &'static str,
    field: &'static str,
) -> DefinitionError {
    DefinitionError::EmptyDefinitionField {
        definition,
        type_name,
        field,
    }
}
