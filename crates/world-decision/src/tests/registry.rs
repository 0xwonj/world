use world_context::ContextProjectionKind;

use crate::{
    DecisionError, DecisionProfileStep, DecisionRegistry, DecisionRegistryBuilder,
    DeterminismPolicy, ImplementationMode, PassClass, PassWritePolicy, ProfileOraclePolicy,
    RepresentationAuthority, RepresentationInput, RepresentationOutput, RepresentationPersistence,
    RepresentationRole, RepresentationVisibility,
};

use super::helpers::{
    id, pass, pass_with_metadata, profile, profile_with_exit_and_policy, profile_with_policy,
    profile_with_terminal, representation, representation_with_metadata, seed_profile_registry,
    valid_two_step_registry,
};

#[test]
fn registry_rejects_duplicate_ids_across_declaration_kinds() {
    let representation = representation(1, "signal", [RepresentationRole::DecisionSignal]);
    let pass = pass(
        1,
        "same_id",
        PassClass::SemanticGrounding,
        [],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            representation.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );

    let mut builder = DecisionRegistryBuilder::new();
    assert!(builder.add_representation(representation).is_ok());

    assert!(matches!(
        builder.add_pass(pass),
        Err(DecisionError::DuplicateDefinitionId { id }) if id == crate::tests::helpers::id(1)
    ));
}

#[test]
fn registry_validates_pass_output_kind_roles() {
    let choice = representation(10, "choice", [RepresentationRole::Choice]);
    let pass = pass(
        20,
        "bad_output",
        PassClass::SemanticGrounding,
        [],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            choice.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );

    assert_eq!(
        DecisionRegistry::new([choice], [pass], []),
        Err(DecisionError::RepresentationRoleMismatch {
            owner: id(20),
            kind: id(10),
            role: RepresentationRole::DecisionSignal,
        })
    );
}

#[test]
fn registry_validates_pass_input_kind_roles() {
    let choice = representation(10, "choice", [RepresentationRole::Choice]);
    let signal = representation(11, "signal", [RepresentationRole::DecisionSignal]);
    let pass = pass(
        20,
        "bad_input",
        PassClass::Choice,
        [RepresentationInput::required_kind(
            RepresentationRole::DecisionSignal,
            choice.id(),
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );

    assert_eq!(
        DecisionRegistry::new([choice, signal], [pass], []),
        Err(DecisionError::RepresentationRoleMismatch {
            owner: id(20),
            kind: id(10),
            role: RepresentationRole::DecisionSignal,
        })
    );
}

#[test]
fn registry_validates_profile_pass_references_and_modes() {
    let signal = representation(10, "signal", [RepresentationRole::DecisionSignal]);
    let pass = pass(
        20,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let missing_pass_profile = profile(
        30,
        "missing",
        [ContextProjectionKind::Observation],
        [DecisionProfileStep::new(id(99), ImplementationMode::Rule)],
    );
    let bad_mode_profile = profile(
        31,
        "bad_mode",
        [ContextProjectionKind::Observation],
        [DecisionProfileStep::new(
            pass.id(),
            ImplementationMode::Heuristic,
        )],
    );

    assert_eq!(
        DecisionRegistry::new([signal.clone()], [pass.clone()], [missing_pass_profile]),
        Err(DecisionError::MissingPass {
            profile: id(30),
            pass: id(99),
        })
    );
    assert_eq!(
        DecisionRegistry::new([signal], [pass], [bad_mode_profile]),
        Err(DecisionError::UnsupportedMode {
            profile: id(31),
            pass: id(20),
            mode: ImplementationMode::Heuristic,
        })
    );
}

#[test]
fn profile_flow_accepts_context_inputs_and_prior_outputs() {
    let registry = valid_two_step_registry();

    assert!(registry.profile(id(300)).is_some());
    assert!(registry.pass(id(200)).is_some());
    assert!(registry.representation(id(100)).is_some());
}

#[test]
fn profile_exit_rejects_missing_terminal_output() {
    let signal = representation(40, "signal", [RepresentationRole::DecisionSignal]);
    let pass = pass(
        41,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let profile = profile_with_terminal(
        42,
        "missing_terminal",
        [ContextProjectionKind::Observation],
        [DecisionProfileStep::new(
            pass.id(),
            ImplementationMode::Rule,
        )],
        RepresentationRole::Choice,
        None,
    );

    assert_eq!(
        DecisionRegistry::new([signal], [pass], [profile]),
        Err(DecisionError::MissingProfileOutput {
            profile: id(42),
            role: RepresentationRole::Choice,
            kind: None,
        })
    );
}

#[test]
fn profile_exit_rejects_ambiguous_terminal_output() {
    let signal_a = representation(43, "signal_a", [RepresentationRole::DecisionSignal]);
    let signal_b = representation(44, "signal_b", [RepresentationRole::DecisionSignal]);
    let ground_a = pass(
        45,
        "ground_a",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal_a.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let ground_b = pass(
        46,
        "ground_b",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal_b.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let profile = profile_with_terminal(
        47,
        "ambiguous_terminal",
        [ContextProjectionKind::Observation],
        [
            DecisionProfileStep::new(ground_a.id(), ImplementationMode::Rule),
            DecisionProfileStep::new(ground_b.id(), ImplementationMode::Rule),
        ],
        RepresentationRole::DecisionSignal,
        None,
    );

    assert_eq!(
        DecisionRegistry::new([signal_a, signal_b], [ground_a, ground_b], [profile]),
        Err(DecisionError::AmbiguousProfileOutput {
            profile: id(47),
            role: RepresentationRole::DecisionSignal,
            kind: None,
            matches: 2,
        })
    );
}

#[test]
fn profile_flow_rejects_missing_context_input() {
    let signal = representation(10, "signal", [RepresentationRole::DecisionSignal]);
    let pass = pass(
        20,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let profile = profile(
        30,
        "no_context",
        [],
        [DecisionProfileStep::new(
            pass.id(),
            ImplementationMode::Rule,
        )],
    );

    assert_eq!(
        DecisionRegistry::new([signal], [pass], [profile]),
        Err(DecisionError::MissingProfileInput {
            profile: id(30),
            pass: id(20),
            role: RepresentationRole::ObservationView,
            kind: None,
            requirement: crate::InputRequirement::Required,
        })
    );
}

#[test]
fn profile_flow_rejects_context_input_not_allowed_by_pass() {
    let signal = representation(12, "signal", [RepresentationRole::DecisionSignal]);
    let pass = pass_with_metadata(
        22,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let profile = profile(
        32,
        "context_not_allowed",
        [ContextProjectionKind::Observation],
        [DecisionProfileStep::new(
            pass.id(),
            ImplementationMode::Rule,
        )],
    );

    assert_eq!(
        DecisionRegistry::new([signal], [pass], [profile]),
        Err(DecisionError::ContextInputNotAllowed {
            profile: id(32),
            pass: id(22),
            context: ContextProjectionKind::Observation,
            role: RepresentationRole::ObservationView,
        })
    );
}

#[test]
fn profile_flow_rejects_ambiguous_role_only_input() {
    let signal_a = representation(13, "signal_a", [RepresentationRole::DecisionSignal]);
    let signal_b = representation(14, "signal_b", [RepresentationRole::DecisionSignal]);
    let choice = representation(15, "choice", [RepresentationRole::Choice]);
    let ground_a = pass(
        23,
        "ground_a",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal_a.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let ground_b = pass(
        24,
        "ground_b",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal_b.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let choose = pass(
        25,
        "choose",
        PassClass::Choice,
        [RepresentationInput::required(
            RepresentationRole::DecisionSignal,
        )],
        [RepresentationOutput::new(
            RepresentationRole::Choice,
            choice.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let profile = profile(
        33,
        "ambiguous",
        [ContextProjectionKind::Observation],
        [
            DecisionProfileStep::new(ground_a.id(), ImplementationMode::Rule),
            DecisionProfileStep::new(ground_b.id(), ImplementationMode::Rule),
            DecisionProfileStep::new(choose.id(), ImplementationMode::Rule),
        ],
    );

    assert_eq!(
        DecisionRegistry::new(
            [signal_a, signal_b, choice],
            [ground_a, ground_b, choose],
            [profile]
        ),
        Err(DecisionError::AmbiguousProfileInput {
            profile: id(33),
            pass: id(25),
            role: RepresentationRole::DecisionSignal,
            kind: None,
            matches: 2,
        })
    );
}

#[test]
fn registry_validates_output_authority_against_write_policy() {
    let proposal = representation_with_metadata(
        16,
        "actor_truth_proposal",
        [RepresentationRole::NonHardUpdateProposal],
        RepresentationVisibility::ResearchTrace,
        RepresentationPersistence::ProposalOnly,
        RepresentationAuthority::ProposalTo(world_core::AuthorityClass::ActorTruth),
    );
    let pass = pass(
        26,
        "publish_social",
        PassClass::Publication,
        [],
        [RepresentationOutput::new(
            RepresentationRole::NonHardUpdateProposal,
            proposal.id(),
        )],
        super::helpers::proposal_policy(world_core::AuthorityClass::Social),
        [ImplementationMode::Rule],
    );
    let profile = profile(
        34,
        "publish",
        [],
        [DecisionProfileStep::new(
            pass.id(),
            ImplementationMode::Rule,
        )],
    );

    assert_eq!(
        DecisionRegistry::new([proposal], [pass], [profile]),
        Err(DecisionError::WritePolicyAuthorityMismatch {
            pass: id(26),
            kind: id(16),
            role: RepresentationRole::NonHardUpdateProposal,
            authority: RepresentationAuthority::ProposalTo(world_core::AuthorityClass::ActorTruth),
        })
    );
}

#[test]
fn disabled_pass_outputs_do_not_satisfy_downstream_inputs() {
    let signal = representation(10, "signal", [RepresentationRole::DecisionSignal]);
    let choice = representation(11, "choice", [RepresentationRole::Choice]);
    let grounding = pass(
        20,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule, ImplementationMode::Disabled],
    );
    let choose = pass(
        21,
        "choose",
        PassClass::Choice,
        [RepresentationInput::required_kind(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [RepresentationOutput::new(
            RepresentationRole::Choice,
            choice.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let profile = profile(
        30,
        "disabled",
        [ContextProjectionKind::Observation],
        [
            DecisionProfileStep::new(grounding.id(), ImplementationMode::Disabled),
            DecisionProfileStep::new(choose.id(), ImplementationMode::Rule),
        ],
    );

    assert_eq!(
        DecisionRegistry::new([signal, choice], [grounding, choose], [profile]),
        Err(DecisionError::MissingProfileInput {
            profile: id(30),
            pass: id(21),
            role: RepresentationRole::DecisionSignal,
            kind: Some(id(10)),
            requirement: crate::InputRequirement::Required,
        })
    );
}

#[test]
fn oracle_required_profile_requires_oracle_involvement() {
    let signal = representation(17, "signal", [RepresentationRole::DecisionSignal]);
    let pass = pass(
        27,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let profile = profile_with_policy(
        35,
        "requires_oracle",
        [ContextProjectionKind::Observation],
        [DecisionProfileStep::new(
            pass.id(),
            ImplementationMode::Rule,
        )],
        ProfileOraclePolicy::Require,
    );

    assert_eq!(
        DecisionRegistry::new([signal], [pass], [profile]),
        Err(DecisionError::OracleRequired { profile: id(35) })
    );
}

#[test]
fn normal_profile_rejects_oracle_mode_and_oracle_artifact() {
    let oracle_kind = representation_with_metadata(
        10,
        "oracle_model",
        [RepresentationRole::OtherModelView],
        RepresentationVisibility::OracleOnly,
        RepresentationPersistence::TraceRecorded,
        RepresentationAuthority::Oracle,
    );
    let pass = pass_with_metadata(
        20,
        "oracle",
        PassClass::OtherModeling,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::OtherModelView,
            oracle_kind.id(),
        )],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Oracle],
        DeterminismPolicy::Oracle,
    );
    let oracle_mode_profile = profile(
        30,
        "oracle_mode",
        [ContextProjectionKind::Observation],
        [DecisionProfileStep::new(
            pass.id(),
            ImplementationMode::Oracle,
        )],
    );

    assert_eq!(
        DecisionRegistry::new([oracle_kind.clone()], [pass.clone()], [oracle_mode_profile]),
        Err(DecisionError::OracleModeForbidden {
            profile: id(30),
            pass: id(20),
        })
    );

    let artifact_profile = profile_with_exit_and_policy(
        31,
        "oracle_artifact",
        [ContextProjectionKind::Observation],
        [DecisionProfileStep::new(
            pass.id(),
            ImplementationMode::Oracle,
        )],
        crate::DecisionProfileExit::terminal(crate::DecisionProfileOutput::new(
            RepresentationRole::OtherModelView,
            Some(oracle_kind.id()),
        )),
        ProfileOraclePolicy::Allow,
    );
    assert!(DecisionRegistry::new([oracle_kind], [pass], [artifact_profile]).is_ok());
}

#[test]
fn seed_profile_fixtures_cover_initial_ablation_matrix() {
    let registry = seed_profile_registry();

    for profile in [310, 311, 312, 313, 314] {
        assert!(
            registry.profile(id(profile)).is_some(),
            "seed profile {profile} should validate"
        );
    }
}
