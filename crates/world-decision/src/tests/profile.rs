use world_context::ContextProjectionKind;

use crate::{
    DecisionError, DecisionProfile, DecisionProfileStep, ImplementationMode, ProfileOraclePolicy,
    TracePolicy,
};

use super::helpers::{id, name, profile, version};

#[test]
fn profile_rejects_empty_steps() {
    let result = DecisionProfile::new(
        id(1),
        name("empty"),
        [ContextProjectionKind::Observation],
        [],
        ProfileOraclePolicy::Forbid,
        TracePolicy::default(),
        version(1),
    );

    assert_eq!(
        result,
        Err(DecisionError::EmptyDefinitionField {
            definition: id(1),
            type_name: "DecisionProfile",
            field: "steps",
        })
    );
}

#[test]
fn profile_preserves_explicit_context_inputs_in_ordered_form() {
    let profile = profile(
        2,
        "context_inputs",
        [
            ContextProjectionKind::Social,
            ContextProjectionKind::Observation,
            ContextProjectionKind::Social,
        ],
        [DecisionProfileStep::new(id(10), ImplementationMode::Rule)],
    );

    assert_eq!(
        profile.context_inputs().collect::<Vec<_>>(),
        [
            ContextProjectionKind::Observation,
            ContextProjectionKind::Social
        ]
    );
}

#[test]
fn profile_preserves_step_order() {
    let profile = profile(
        3,
        "ordered",
        [ContextProjectionKind::Observation],
        [
            DecisionProfileStep::new(id(10), ImplementationMode::Rule),
            DecisionProfileStep::new(id(11), ImplementationMode::Heuristic),
        ],
    );

    assert_eq!(
        profile.steps(),
        [
            DecisionProfileStep::new(id(10), ImplementationMode::Rule),
            DecisionProfileStep::new(id(11), ImplementationMode::Heuristic),
        ]
    );
}

#[test]
fn oracle_policy_marks_oracle_labeled_profiles() {
    assert!(ProfileOraclePolicy::Forbid.forbids_oracle());
    assert!(!ProfileOraclePolicy::Allow.forbids_oracle());
    assert!(ProfileOraclePolicy::Require.is_oracle_labeled());
}
