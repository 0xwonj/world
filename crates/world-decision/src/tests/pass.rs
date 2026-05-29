use world_context::ContextProjectionKind;
use world_core::AuthorityClass;

use crate::{
    DecisionError, DecisionPassContract, DeterminismPolicy, ImplementationMode, PassClass,
    PassWritePolicy, RepresentationInput, RepresentationOutput, RepresentationRole, TracePolicy,
};

use super::helpers::{id, name, pass_with_metadata, proposal_policy, version};

#[test]
fn pass_rejects_empty_implementation_modes() {
    let result = DecisionPassContract::new(
        id(1),
        name("grounding"),
        PassClass::SemanticGrounding,
        [],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            id(10),
        )],
        [],
        [],
        [],
        PassWritePolicy::None,
        [],
        DeterminismPolicy::Deterministic,
        TracePolicy::default(),
        version(1),
    );

    assert_eq!(
        result,
        Err(DecisionError::EmptyDefinitionField {
            definition: id(1),
            type_name: "DecisionPassContract",
            field: "implementation_modes",
        })
    );
}

#[test]
fn pass_rejects_hard_proposal_write_policy() {
    let result = DecisionPassContract::new(
        id(2),
        name("hard_proposal"),
        PassClass::Publication,
        [],
        [RepresentationOutput::new(
            RepresentationRole::NonHardUpdateProposal,
            id(10),
        )],
        [],
        [],
        [],
        proposal_policy(AuthorityClass::Hard),
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
        TracePolicy::default(),
        version(1),
    );

    assert_eq!(
        result,
        Err(DecisionError::HardMutationAuthority { pass: id(2) })
    );
}

#[test]
fn pass_rejects_output_role_incompatible_with_write_policy() {
    let result = DecisionPassContract::new(
        id(3),
        name("request_without_policy"),
        PassClass::ExecutionRequest,
        [],
        [RepresentationOutput::new(
            RepresentationRole::ExecutableRequest,
            id(10),
        )],
        [],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
        TracePolicy::default(),
        version(1),
    );

    assert_eq!(
        result,
        Err(DecisionError::WritePolicyRoleMismatch {
            pass: id(3),
            role: RepresentationRole::ExecutableRequest,
        })
    );
}

#[test]
fn pass_rejects_write_policy_incompatible_with_class() {
    let result = DecisionPassContract::new(
        id(30),
        name("semantic_request"),
        PassClass::SemanticGrounding,
        [],
        [RepresentationOutput::new(
            RepresentationRole::ExecutableRequest,
            id(10),
        )],
        [],
        [],
        [],
        PassWritePolicy::ExecutableRequestOnly,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
        TracePolicy::default(),
        version(1),
    );

    assert_eq!(
        result,
        Err(DecisionError::WritePolicyClassMismatch {
            pass: id(30),
            class: PassClass::SemanticGrounding,
        })
    );
}

#[test]
fn pass_rejects_conflicting_authority_read_contract() {
    let result = DecisionPassContract::new(
        id(4),
        name("conflict"),
        PassClass::SemanticGrounding,
        [],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            id(10),
        )],
        [],
        [AuthorityClass::ActorTruth],
        [AuthorityClass::ActorTruth],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
        TracePolicy::default(),
        version(1),
    );

    assert_eq!(
        result,
        Err(DecisionError::ConflictingAuthorityRead {
            pass: id(4),
            authority: AuthorityClass::ActorTruth,
        })
    );
}

#[test]
fn llm_and_oracle_modes_require_matching_determinism_metadata() {
    let llm_result = DecisionPassContract::new(
        id(5),
        name("llm"),
        PassClass::OtherModeling,
        [],
        [RepresentationOutput::new(
            RepresentationRole::OtherModelView,
            id(10),
        )],
        [],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Llm],
        DeterminismPolicy::Deterministic,
        TracePolicy::default(),
        version(1),
    );
    let oracle_result = DecisionPassContract::new(
        id(6),
        name("oracle"),
        PassClass::OtherModeling,
        [],
        [RepresentationOutput::new(
            RepresentationRole::OtherModelView,
            id(11),
        )],
        [],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Oracle],
        DeterminismPolicy::ExternalNondeterministic,
        TracePolicy::default(),
        version(1),
    );

    assert_eq!(
        llm_result,
        Err(DecisionError::DeterminismMismatch {
            pass: id(5),
            mode: ImplementationMode::Llm,
            determinism: DeterminismPolicy::Deterministic,
        })
    );
    assert_eq!(
        oracle_result,
        Err(DecisionError::DeterminismMismatch {
            pass: id(6),
            mode: ImplementationMode::Oracle,
            determinism: DeterminismPolicy::ExternalNondeterministic,
        })
    );
}

#[test]
fn validation_and_diagnostic_passes_may_have_no_outputs() {
    let pass = pass_with_metadata(
        7,
        "validator",
        PassClass::Validation,
        [RepresentationInput::required(
            RepresentationRole::DecisionSignal,
        )],
        [],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );

    assert_eq!(pass.outputs(), []);
}
