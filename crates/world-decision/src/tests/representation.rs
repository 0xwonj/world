use world_core::AuthorityClass;

use crate::{
    DecisionError, RepresentationAuthority, RepresentationKindDef, RepresentationPersistence,
    RepresentationRole, RepresentationVisibility,
};

use super::helpers::{id, name, representation_with_metadata, version};

#[test]
fn representation_rejects_empty_roles() {
    let result = RepresentationKindDef::new(
        id(1),
        name("empty"),
        [],
        RepresentationVisibility::EngineInternal,
        RepresentationPersistence::Ephemeral,
        RepresentationAuthority::Derived,
        version(1),
    );

    assert_eq!(
        result,
        Err(DecisionError::EmptyDefinitionField {
            definition: id(1),
            type_name: "RepresentationKindDef",
            field: "roles",
        })
    );
}

#[test]
fn representation_roles_are_deterministic_and_deduplicated() {
    let representation = representation_with_metadata(
        2,
        "signal",
        [
            RepresentationRole::DecisionSignal,
            RepresentationRole::DecisionSignal,
            RepresentationRole::MotivationalSignal,
        ],
        RepresentationVisibility::EngineInternal,
        RepresentationPersistence::Ephemeral,
        RepresentationAuthority::Derived,
    );

    assert_eq!(
        representation.roles().collect::<Vec<_>>(),
        [
            RepresentationRole::DecisionSignal,
            RepresentationRole::MotivationalSignal,
        ]
    );
}

#[test]
fn proposal_authority_preserves_target_authority_class() {
    let representation = representation_with_metadata(
        3,
        "social_proposal",
        [RepresentationRole::NonHardUpdateProposal],
        RepresentationVisibility::ResearchTrace,
        RepresentationPersistence::ProposalOnly,
        RepresentationAuthority::ProposalTo(AuthorityClass::Social),
    );

    assert_eq!(
        representation.authority(),
        RepresentationAuthority::ProposalTo(AuthorityClass::Social)
    );
    assert!(representation.authority().is_proposal());
}

#[test]
fn representation_rejects_hard_proposal_authority() {
    let result = RepresentationKindDef::new(
        id(6),
        name("hard_proposal"),
        [RepresentationRole::NonHardUpdateProposal],
        RepresentationVisibility::ResearchTrace,
        RepresentationPersistence::ProposalOnly,
        RepresentationAuthority::ProposalTo(AuthorityClass::Hard),
        version(1),
    );

    assert_eq!(
        result,
        Err(DecisionError::HardProposalAuthority {
            representation: id(6),
        })
    );
}

#[test]
fn representation_rejects_authority_without_required_role() {
    let result = RepresentationKindDef::new(
        id(7),
        name("request"),
        [RepresentationRole::Choice],
        RepresentationVisibility::EngineInternal,
        RepresentationPersistence::ProposalOnly,
        RepresentationAuthority::ExecutableRequest,
        version(1),
    );

    assert_eq!(
        result,
        Err(DecisionError::RepresentationAuthorityRoleMismatch {
            representation: id(7),
            authority: RepresentationAuthority::ExecutableRequest,
            role: RepresentationRole::ExecutableRequest,
        })
    );
}

#[test]
fn oracle_visibility_is_explicitly_classified() {
    let representation = representation_with_metadata(
        4,
        "oracle_other_model",
        [RepresentationRole::OtherModelView],
        RepresentationVisibility::OracleOnly,
        RepresentationPersistence::TraceRecorded,
        RepresentationAuthority::Oracle,
    );

    assert!(representation.is_oracle_only());
}

#[test]
fn diagnostic_only_representation_cannot_satisfy_request_role() {
    let representation = representation_with_metadata(
        5,
        "diagnostic_request",
        [RepresentationRole::ExecutableRequest],
        RepresentationVisibility::DiagnosticOnly,
        RepresentationPersistence::TraceRecorded,
        RepresentationAuthority::Derived,
    );

    assert!(representation.has_role(RepresentationRole::ExecutableRequest));
    assert!(!representation.can_satisfy(RepresentationRole::ExecutableRequest));
}
