use thiserror::Error;
use world_context::ContextProjectionKind;
use world_core::{AuthorityClass, DefinitionId};

use crate::{
    DecisionArtifactRef, DeterminismPolicy, ImplementationMode, InputRequirement, PassClass,
    RepresentationAuthority, RepresentationRole,
};

/// Error returned when decision declarations violate local or registry invariants.
#[non_exhaustive]
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DecisionError {
    /// A definition-scoped field that must have at least one value was empty.
    #[error("{type_name} {} has empty required field {field}", .definition.get())]
    EmptyDefinitionField {
        /// Definition that owns the invalid field.
        definition: DefinitionId,
        /// Type that rejected the field.
        type_name: &'static str,
        /// Field that was empty.
        field: &'static str,
    },
    /// A non-definition-scoped field that must have a value was empty.
    #[error("{type_name} has empty required field {field}")]
    EmptyItemField {
        /// Type that rejected the field.
        type_name: &'static str,
        /// Field that was empty.
        field: &'static str,
    },
    /// A registry contains the same definition id more than once.
    #[error("decision definition id {} is declared more than once", .id.get())]
    DuplicateDefinitionId {
        /// Duplicated definition id.
        id: DefinitionId,
    },
    /// A trace contains the same artifact ref more than once.
    #[error("decision trace artifact {} is declared more than once", .artifact.get())]
    DuplicateArtifactRef {
        /// Duplicated trace-local artifact ref.
        artifact: DecisionArtifactRef,
    },
    /// A pass or trace references an unknown representation kind.
    #[error("decision definition {} references missing representation kind {}", .owner.get(), .kind.get())]
    MissingRepresentationKind {
        /// Definition or profile that owns the dangling reference.
        owner: DefinitionId,
        /// Missing representation kind id.
        kind: DefinitionId,
    },
    /// A profile references an unknown pass.
    #[error("decision profile {} references missing pass {}", .profile.get(), .pass.get())]
    MissingPass {
        /// Profile that owns the dangling reference.
        profile: DefinitionId,
        /// Missing pass id.
        pass: DefinitionId,
    },
    /// A representation kind cannot satisfy the requested broad role.
    #[error("decision definition {} requires representation kind {} to provide role {role:?}", .owner.get(), .kind.get())]
    RepresentationRoleMismatch {
        /// Definition that requested the role.
        owner: DefinitionId,
        /// Referenced representation kind.
        kind: DefinitionId,
        /// Required role.
        role: RepresentationRole,
    },
    /// A pass reads and forbids the same authority class.
    #[error("decision pass {} both allows and forbids authority read {authority:?}", .pass.get())]
    ConflictingAuthorityRead {
        /// Pass that declared the conflict.
        pass: DefinitionId,
        /// Conflicting authority class.
        authority: AuthorityClass,
    },
    /// A pass write policy tries to write hard truth directly.
    #[error("decision pass {} cannot declare hard mutation authority", .pass.get())]
    HardMutationAuthority {
        /// Pass that declared the forbidden authority.
        pass: DefinitionId,
    },
    /// A representation kind is declared as a hard-authority proposal.
    #[error("decision representation {} cannot propose hard mutation authority", .representation.get())]
    HardProposalAuthority {
        /// Representation kind that declared the forbidden authority.
        representation: DefinitionId,
    },
    /// A representation kind authority is incompatible with its role set.
    #[error("decision representation {} authority {authority:?} is incompatible with role {role:?}", .representation.get())]
    RepresentationAuthorityRoleMismatch {
        /// Representation kind with the incompatible declaration.
        representation: DefinitionId,
        /// Declared authority metadata.
        authority: RepresentationAuthority,
        /// Required role missing from the role set.
        role: RepresentationRole,
    },
    /// A pass write policy is incompatible with its pass class.
    #[error("decision pass {} write policy is incompatible with pass class {class:?}", .pass.get())]
    WritePolicyClassMismatch {
        /// Pass with the incompatible declaration.
        pass: DefinitionId,
        /// Declared pass class.
        class: PassClass,
    },
    /// A pass output contract is incompatible with its write policy.
    #[error("decision pass {} output role {role:?} is incompatible with its write policy", .pass.get())]
    WritePolicyRoleMismatch {
        /// Pass with the incompatible output.
        pass: DefinitionId,
        /// Output role rejected by the write policy.
        role: RepresentationRole,
    },
    /// A pass output representation authority is incompatible with its write policy.
    #[error("decision pass {} output kind {} has incompatible authority {authority:?} for role {role:?}", .pass.get(), .kind.get())]
    WritePolicyAuthorityMismatch {
        /// Pass with the incompatible output.
        pass: DefinitionId,
        /// Output representation kind.
        kind: DefinitionId,
        /// Output role.
        role: RepresentationRole,
        /// Declared representation authority.
        authority: RepresentationAuthority,
    },
    /// A pass implementation mode is incompatible with its determinism policy.
    #[error("decision pass {} mode {mode:?} is incompatible with determinism policy {determinism:?}", .pass.get())]
    DeterminismMismatch {
        /// Pass with the incompatible mode.
        pass: DefinitionId,
        /// Mode rejected by the determinism policy.
        mode: ImplementationMode,
        /// Declared determinism policy.
        determinism: DeterminismPolicy,
    },
    /// A pass uses an `AnyOf` input group without a group name.
    #[error("decision pass {} has an empty AnyOf input group", .pass.get())]
    EmptyInputGroup {
        /// Pass with the invalid input requirement.
        pass: DefinitionId,
    },
    /// A profile step selected a mode that the pass does not declare.
    #[error("decision profile {} invokes pass {} with unsupported mode {mode:?}", .profile.get(), .pass.get())]
    UnsupportedMode {
        /// Profile that selected the mode.
        profile: DefinitionId,
        /// Pass invoked by the profile.
        pass: DefinitionId,
        /// Unsupported mode.
        mode: ImplementationMode,
    },
    /// A profile step requires an input that is not available from context or earlier outputs.
    #[error("decision profile {} cannot satisfy pass {} input role {role:?}", .profile.get(), .pass.get())]
    MissingProfileInput {
        /// Profile being validated.
        profile: DefinitionId,
        /// Pass whose input is unavailable.
        pass: DefinitionId,
        /// Required broad role.
        role: RepresentationRole,
        /// Required concrete representation kind, if any.
        kind: Option<DefinitionId>,
        /// Required input policy.
        requirement: InputRequirement,
    },
    /// A role-only profile input has more than one possible source.
    #[error("decision profile {} pass {} input role {role:?} is ambiguous across {matches} sources", .profile.get(), .pass.get())]
    AmbiguousProfileInput {
        /// Profile being validated.
        profile: DefinitionId,
        /// Pass whose input is ambiguous.
        pass: DefinitionId,
        /// Required broad role.
        role: RepresentationRole,
        /// Required concrete representation kind, if any.
        kind: Option<DefinitionId>,
        /// Number of matching sources.
        matches: usize,
    },
    /// A profile input is available only through a context projection the pass did not allow.
    #[error("decision profile {} pass {} input role {role:?} requires disallowed context {context:?}", .profile.get(), .pass.get())]
    ContextInputNotAllowed {
        /// Profile being validated.
        profile: DefinitionId,
        /// Pass whose context access is too narrow.
        pass: DefinitionId,
        /// Context projection that could satisfy the input.
        context: ContextProjectionKind,
        /// Required broad role.
        role: RepresentationRole,
    },
    /// An AnyOf input group has more than one satisfiable alternative.
    #[error("decision profile {} pass {} AnyOf group {group} is ambiguous", .profile.get(), .pass.get())]
    AmbiguousAnyOfInput {
        /// Profile being validated.
        profile: DefinitionId,
        /// Pass whose input group is ambiguous.
        pass: DefinitionId,
        /// Input group name.
        group: String,
    },
    /// A non-oracle profile selected an oracle implementation mode.
    #[error("decision profile {} cannot invoke pass {} with oracle mode", .profile.get(), .pass.get())]
    OracleModeForbidden {
        /// Profile that forbids oracle involvement.
        profile: DefinitionId,
        /// Pass selected with oracle mode.
        pass: DefinitionId,
    },
    /// A non-oracle profile produces or depends on an oracle-only artifact.
    #[error("decision profile {} cannot use oracle representation kind {}", .profile.get(), .kind.get())]
    OracleArtifactForbidden {
        /// Profile that forbids oracle artifacts.
        profile: DefinitionId,
        /// Oracle-only representation kind.
        kind: DefinitionId,
    },
    /// An oracle-required profile contains no oracle mode or oracle artifact.
    #[error("decision profile {} requires oracle involvement but none was declared", .profile.get())]
    OracleRequired {
        /// Profile that requires oracle involvement.
        profile: DefinitionId,
    },
}

pub(crate) fn empty_definition_field(
    definition: DefinitionId,
    type_name: &'static str,
    field: &'static str,
) -> DecisionError {
    DecisionError::EmptyDefinitionField {
        definition,
        type_name,
        field,
    }
}

pub(crate) fn empty_item_field(type_name: &'static str, field: &'static str) -> DecisionError {
    DecisionError::EmptyItemField { type_name, field }
}
