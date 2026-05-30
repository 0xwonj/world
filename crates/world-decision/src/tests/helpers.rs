use std::collections::BTreeSet;

use world_context::{ActorContextProjection, ContextProjectionKind};
use world_core::{ActorId, AuthorityClass, DefinitionId, VersionAnchor};
use world_defs::DefinitionName;

use crate::{
    DecisionPassContract, DecisionProfile, DecisionProfileExit, DecisionProfileOutput,
    DecisionProfileStep, DecisionRegistry, DecisionRegistryBuilder, DeterminismPolicy,
    ImplementationMode, PassClass, PassWritePolicy, ProfileOraclePolicy, RepresentationAuthority,
    RepresentationInput, RepresentationKindDef, RepresentationOutput, RepresentationPersistence,
    RepresentationRole, RepresentationVisibility,
};

pub(crate) fn id(value: u64) -> DefinitionId {
    let Some(id) = DefinitionId::new(value) else {
        panic!("test ids must be nonzero");
    };
    id
}

pub(crate) fn actor(value: u64) -> ActorId {
    let Some(actor) = ActorId::new(value) else {
        panic!("test actors must be nonzero");
    };
    actor
}

pub(crate) fn context_projection(actor_value: u64) -> ActorContextProjection {
    ActorContextProjection::empty(actor(actor_value))
}

pub(crate) fn version(value: u64) -> VersionAnchor {
    let Some(version) = VersionAnchor::new(value) else {
        panic!("test versions must be nonzero");
    };
    version
}

pub(crate) fn name(value: &'static str) -> DefinitionName {
    let Some(name) = DefinitionName::new(value) else {
        panic!("test names must be non-empty");
    };
    name
}

pub(crate) fn representation(
    value: u64,
    representation_name: &'static str,
    roles: impl IntoIterator<Item = RepresentationRole>,
) -> RepresentationKindDef {
    representation_with_metadata(
        value,
        representation_name,
        roles,
        RepresentationVisibility::EngineInternal,
        RepresentationPersistence::Ephemeral,
        RepresentationAuthority::Derived,
    )
}

pub(crate) fn representation_with_metadata(
    value: u64,
    representation_name: &'static str,
    roles: impl IntoIterator<Item = RepresentationRole>,
    visibility: RepresentationVisibility,
    persistence: RepresentationPersistence,
    authority: RepresentationAuthority,
) -> RepresentationKindDef {
    let Ok(representation) = RepresentationKindDef::new(
        id(value),
        name(representation_name),
        roles,
        visibility,
        persistence,
        authority,
        version(1),
    ) else {
        panic!("test representation should be valid");
    };
    representation
}

pub(crate) fn pass(
    value: u64,
    pass_name: &'static str,
    class: PassClass,
    inputs: impl IntoIterator<Item = RepresentationInput>,
    outputs: impl IntoIterator<Item = RepresentationOutput>,
    write_policy: PassWritePolicy,
    modes: impl IntoIterator<Item = ImplementationMode>,
) -> DecisionPassContract {
    let inputs = inputs.into_iter().collect::<Vec<_>>();
    let allowed_context = allowed_context_for_inputs(&inputs);
    pass_with_metadata(
        value,
        pass_name,
        class,
        inputs,
        outputs,
        allowed_context,
        [],
        [],
        write_policy,
        modes,
        DeterminismPolicy::Deterministic,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pass_with_metadata(
    value: u64,
    pass_name: &'static str,
    class: PassClass,
    inputs: impl IntoIterator<Item = RepresentationInput>,
    outputs: impl IntoIterator<Item = RepresentationOutput>,
    allowed_context: impl IntoIterator<Item = ContextProjectionKind>,
    allowed_authority_reads: impl IntoIterator<Item = AuthorityClass>,
    forbidden_authority_reads: impl IntoIterator<Item = AuthorityClass>,
    write_policy: PassWritePolicy,
    modes: impl IntoIterator<Item = ImplementationMode>,
    determinism: DeterminismPolicy,
) -> DecisionPassContract {
    let Ok(pass) = DecisionPassContract::new(
        id(value),
        name(pass_name),
        class,
        inputs,
        outputs,
        allowed_context,
        allowed_authority_reads,
        forbidden_authority_reads,
        write_policy,
        modes,
        determinism,
        version(1),
    ) else {
        panic!("test pass should be valid");
    };
    pass
}

fn allowed_context_for_inputs(inputs: &[RepresentationInput]) -> BTreeSet<ContextProjectionKind> {
    let mut contexts = BTreeSet::new();
    for input in inputs {
        match input.role() {
            RepresentationRole::ActorRelativeView => {
                contexts.insert(ContextProjectionKind::Observation);
                contexts.insert(ContextProjectionKind::Epistemic);
                contexts.insert(ContextProjectionKind::Social);
            }
            RepresentationRole::ObservationView => {
                contexts.insert(ContextProjectionKind::Observation);
            }
            RepresentationRole::EpistemicView => {
                contexts.insert(ContextProjectionKind::Epistemic);
            }
            RepresentationRole::SocialContextView => {
                contexts.insert(ContextProjectionKind::Social);
            }
            RepresentationRole::CapabilitySet => {
                contexts.insert(ContextProjectionKind::Capability);
            }
            RepresentationRole::ActionRepertoire => {
                contexts.insert(ContextProjectionKind::Repertoire);
            }
            RepresentationRole::AffordanceView => {
                contexts.insert(ContextProjectionKind::Affordance);
            }
            _ => {}
        }
    }
    contexts
}

pub(crate) fn profile(
    value: u64,
    profile_name: &'static str,
    context_inputs: impl IntoIterator<Item = ContextProjectionKind>,
    steps: impl IntoIterator<Item = DecisionProfileStep>,
) -> DecisionProfile {
    profile_with_exit_and_policy(
        value,
        profile_name,
        context_inputs,
        steps,
        DecisionProfileExit::terminal(DecisionProfileOutput::new(RepresentationRole::Choice, None)),
        ProfileOraclePolicy::Forbid,
    )
}

pub(crate) fn profile_with_terminal(
    value: u64,
    profile_name: &'static str,
    context_inputs: impl IntoIterator<Item = ContextProjectionKind>,
    steps: impl IntoIterator<Item = DecisionProfileStep>,
    role: RepresentationRole,
    kind: Option<DefinitionId>,
) -> DecisionProfile {
    profile_with_exit_and_policy(
        value,
        profile_name,
        context_inputs,
        steps,
        DecisionProfileExit::terminal(DecisionProfileOutput::new(role, kind)),
        ProfileOraclePolicy::Forbid,
    )
}

pub(crate) fn profile_with_policy(
    value: u64,
    profile_name: &'static str,
    context_inputs: impl IntoIterator<Item = ContextProjectionKind>,
    steps: impl IntoIterator<Item = DecisionProfileStep>,
    oracle_policy: ProfileOraclePolicy,
) -> DecisionProfile {
    profile_with_exit_and_policy(
        value,
        profile_name,
        context_inputs,
        steps,
        DecisionProfileExit::terminal(DecisionProfileOutput::new(RepresentationRole::Choice, None)),
        oracle_policy,
    )
}

pub(crate) fn profile_with_exit_and_policy(
    value: u64,
    profile_name: &'static str,
    context_inputs: impl IntoIterator<Item = ContextProjectionKind>,
    steps: impl IntoIterator<Item = DecisionProfileStep>,
    exit: DecisionProfileExit,
    oracle_policy: ProfileOraclePolicy,
) -> DecisionProfile {
    let Ok(profile) = DecisionProfile::new(
        id(value),
        name(profile_name),
        context_inputs,
        steps,
        exit,
        oracle_policy,
        version(1),
    ) else {
        panic!("test profile should be valid");
    };
    profile
}

pub(crate) fn proposal_policy(authority: AuthorityClass) -> PassWritePolicy {
    PassWritePolicy::ProposalOnly(BTreeSet::from([authority]))
}

pub(crate) fn valid_two_step_registry() -> DecisionRegistry {
    let signal_kind = representation(
        100,
        "observation_signal",
        [RepresentationRole::DecisionSignal],
    );
    let choice_kind = representation(101, "choice", [RepresentationRole::Choice]);
    let grounding = pass(
        200,
        "ground_observation",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal_kind.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let choose = pass(
        201,
        "choose",
        PassClass::Choice,
        [RepresentationInput::required_kind(
            RepresentationRole::DecisionSignal,
            signal_kind.id(),
        )],
        [RepresentationOutput::new(
            RepresentationRole::Choice,
            choice_kind.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let profile = profile(
        300,
        "direct_choice",
        [ContextProjectionKind::Observation],
        [
            DecisionProfileStep::new(grounding.id(), ImplementationMode::Rule),
            DecisionProfileStep::new(choose.id(), ImplementationMode::Rule),
        ],
    );

    let Ok(registry) =
        DecisionRegistry::new([signal_kind, choice_kind], [grounding, choose], [profile])
    else {
        panic!("test registry should be valid");
    };
    registry
}

pub(crate) fn seed_profile_registry() -> DecisionRegistry {
    let context_kind = representation(
        110,
        "structured_context",
        [RepresentationRole::ActorRelativeView],
    );
    let intent_kind = representation(111, "intent", [RepresentationRole::IntentCandidate]);
    let other_model_kind = representation(112, "other_model", [RepresentationRole::OtherModelView]);
    let oracle_other_model_kind = representation_with_metadata(
        113,
        "oracle_other_model",
        [RepresentationRole::OtherModelView],
        RepresentationVisibility::OracleOnly,
        RepresentationPersistence::TraceRecorded,
        RepresentationAuthority::Oracle,
    );
    let request_kind = representation_with_metadata(
        114,
        "request",
        [RepresentationRole::ExecutableRequest],
        RepresentationVisibility::EngineInternal,
        RepresentationPersistence::ProposalOnly,
        RepresentationAuthority::ExecutableRequest,
    );

    let structure_context = pass(
        210,
        "structure_context",
        PassClass::ContextDerivation,
        [RepresentationInput::required_all(
            RepresentationRole::ActorRelativeView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::ActorRelativeView,
            context_kind.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let intent = pass(
        211,
        "intent_candidate",
        PassClass::CandidateGeneration,
        [RepresentationInput::required_kind(
            RepresentationRole::ActorRelativeView,
            context_kind.id(),
        )],
        [RepresentationOutput::new(
            RepresentationRole::IntentCandidate,
            intent_kind.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
    );
    let other_model = pass(
        212,
        "other_model",
        PassClass::OtherModeling,
        [RepresentationInput::required_kind(
            RepresentationRole::IntentCandidate,
            intent_kind.id(),
        )],
        [RepresentationOutput::new(
            RepresentationRole::OtherModelView,
            other_model_kind.id(),
        )],
        PassWritePolicy::None,
        [ImplementationMode::Heuristic],
    );
    let oracle_other_model = pass_with_metadata(
        213,
        "oracle_other_model",
        PassClass::OtherModeling,
        [RepresentationInput::required_kind(
            RepresentationRole::IntentCandidate,
            intent_kind.id(),
        )],
        [RepresentationOutput::new(
            RepresentationRole::OtherModelView,
            oracle_other_model_kind.id(),
        )],
        [],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Oracle],
        DeterminismPolicy::Oracle,
    );
    let request = pass(
        214,
        "request_candidate",
        PassClass::ExecutionRequest,
        [RepresentationInput::required_kind(
            RepresentationRole::IntentCandidate,
            intent_kind.id(),
        )],
        [RepresentationOutput::new(
            RepresentationRole::ExecutableRequest,
            request_kind.id(),
        )],
        PassWritePolicy::ExecutableRequestOnly,
        [ImplementationMode::Rule],
    );
    let direct_request = pass(
        215,
        "direct_request_candidate",
        PassClass::ExecutionRequest,
        [RepresentationInput::required(
            RepresentationRole::ActionRepertoire,
        )],
        [RepresentationOutput::new(
            RepresentationRole::ExecutableRequest,
            request_kind.id(),
        )],
        PassWritePolicy::ExecutableRequestOnly,
        [ImplementationMode::Rule],
    );

    let direct_action_baseline = profile_with_terminal(
        310,
        "direct_action_baseline",
        [
            ContextProjectionKind::Observation,
            ContextProjectionKind::Repertoire,
        ],
        [DecisionProfileStep::new(
            direct_request.id(),
            ImplementationMode::Rule,
        )],
        RepresentationRole::ExecutableRequest,
        Some(request_kind.id()),
    );
    let intent_only_baseline = profile_with_terminal(
        311,
        "intent_only_baseline",
        [ContextProjectionKind::Observation],
        [
            DecisionProfileStep::new(structure_context.id(), ImplementationMode::Rule),
            DecisionProfileStep::new(intent.id(), ImplementationMode::Rule),
        ],
        RepresentationRole::IntentCandidate,
        Some(intent_kind.id()),
    );
    let structured_context_baseline = profile_with_terminal(
        312,
        "structured_context_baseline",
        [
            ContextProjectionKind::Observation,
            ContextProjectionKind::Epistemic,
        ],
        [
            DecisionProfileStep::new(structure_context.id(), ImplementationMode::Rule),
            DecisionProfileStep::new(intent.id(), ImplementationMode::Rule),
            DecisionProfileStep::new(request.id(), ImplementationMode::Rule),
        ],
        RepresentationRole::ExecutableRequest,
        Some(request_kind.id()),
    );
    let explicit_other_model_baseline = profile_with_terminal(
        313,
        "explicit_other_model_baseline",
        [
            ContextProjectionKind::Observation,
            ContextProjectionKind::Social,
        ],
        [
            DecisionProfileStep::new(structure_context.id(), ImplementationMode::Rule),
            DecisionProfileStep::new(intent.id(), ImplementationMode::Rule),
            DecisionProfileStep::new(other_model.id(), ImplementationMode::Heuristic),
        ],
        RepresentationRole::OtherModelView,
        Some(other_model_kind.id()),
    );
    let oracle_other_model_baseline = profile_with_exit_and_policy(
        314,
        "oracle_other_model_baseline",
        [ContextProjectionKind::Observation],
        [
            DecisionProfileStep::new(structure_context.id(), ImplementationMode::Rule),
            DecisionProfileStep::new(intent.id(), ImplementationMode::Rule),
            DecisionProfileStep::new(oracle_other_model.id(), ImplementationMode::Oracle),
        ],
        DecisionProfileExit::terminal(DecisionProfileOutput::new(
            RepresentationRole::OtherModelView,
            Some(oracle_other_model_kind.id()),
        )),
        ProfileOraclePolicy::Allow,
    );

    let mut builder = DecisionRegistryBuilder::new();
    for representation in [
        context_kind,
        intent_kind,
        other_model_kind,
        oracle_other_model_kind,
        request_kind,
    ] {
        if builder.add_representation(representation).is_err() {
            panic!("seed representation should be unique");
        }
    }
    for pass in [
        structure_context,
        intent,
        other_model,
        oracle_other_model,
        request,
        direct_request,
    ] {
        if builder.add_pass(pass).is_err() {
            panic!("seed pass should be unique");
        }
    }
    for profile in [
        direct_action_baseline,
        intent_only_baseline,
        structured_context_baseline,
        explicit_other_model_baseline,
        oracle_other_model_baseline,
    ] {
        if builder.add_profile(profile).is_err() {
            panic!("seed profile should be unique");
        }
    }

    let Ok(registry) = builder.build() else {
        panic!("seed profile registry should be valid");
    };
    registry
}
