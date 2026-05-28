use thiserror::Error;
use world_core::DefinitionId;

use crate::effects::StagePermission;
use crate::events::EventRecordSpec;
use crate::keys::{EffectKind, RoleName, StateFieldName};
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
    /// A checked item that is not definition-scoped has an empty field.
    #[error("{type_name} has empty required field {field}")]
    EmptyItemField {
        /// Type that rejected the field.
        type_name: &'static str,
        /// Field that was empty.
        field: &'static str,
    },
    /// A mutating or event-emitting operation did not declare an emitted event.
    #[error("effect operation {operation} requires at least one emitted event")]
    OperationRequiresEvent {
        /// Operation kind that needs an event contract.
        operation: EffectKind,
    },
    /// An operation declared emitted events without event-emission permission.
    #[error(
        "effect operation {operation} declares emitted events without event-emission permission"
    )]
    EventPermissionNotDeclared {
        /// Operation kind that declared emitted events.
        operation: EffectKind,
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
