use std::collections::BTreeSet;

use crate::{
    DecisionError, DecisionPassContract, DecisionProfile, ImplementationMode, InputBinding,
    InputRequirement, PassWritePolicy, ProfileOraclePolicy, RepresentationAuthority,
    RepresentationKindDef,
};

use super::{
    DecisionRegistry,
    flow::{AvailableRepresentations, InputMatchCounts, collect_any_of_groups},
};

pub(super) fn registry(registry: &DecisionRegistry) -> Result<(), DecisionError> {
    validate_passes(registry)?;
    validate_profiles(registry)
}

fn validate_passes(registry: &DecisionRegistry) -> Result<(), DecisionError> {
    for pass in registry.passes.values() {
        validate_pass_inputs(registry, pass)?;
        validate_pass_outputs(registry, pass)?;
    }

    Ok(())
}

fn validate_pass_inputs(
    registry: &DecisionRegistry,
    pass: &DecisionPassContract,
) -> Result<(), DecisionError> {
    for input in pass.inputs() {
        let Some(kind) = input.kind() else {
            continue;
        };
        let Some(representation) = registry.representation(kind) else {
            return Err(DecisionError::MissingRepresentationKind {
                owner: pass.id(),
                kind,
            });
        };
        if !representation.can_satisfy(input.role()) {
            return Err(DecisionError::RepresentationRoleMismatch {
                owner: pass.id(),
                kind,
                role: input.role(),
            });
        }
    }

    Ok(())
}

fn validate_pass_outputs(
    registry: &DecisionRegistry,
    pass: &DecisionPassContract,
) -> Result<(), DecisionError> {
    for output in pass.outputs() {
        let Some(representation) = registry.representation(output.kind()) else {
            return Err(DecisionError::MissingRepresentationKind {
                owner: pass.id(),
                kind: output.kind(),
            });
        };
        if !representation.can_satisfy(output.role()) {
            return Err(DecisionError::RepresentationRoleMismatch {
                owner: pass.id(),
                kind: output.kind(),
                role: output.role(),
            });
        }
        validate_output_write_policy(pass, *output, representation)?;
    }

    Ok(())
}

fn validate_profiles(registry: &DecisionRegistry) -> Result<(), DecisionError> {
    for profile in registry.profiles.values() {
        validate_profile(registry, profile)?;
    }

    Ok(())
}

fn validate_profile(
    registry: &DecisionRegistry,
    profile: &DecisionProfile,
) -> Result<(), DecisionError> {
    let mut available = AvailableRepresentations::from_context_inputs(profile.context_inputs());
    let mut oracle_involved = false;

    for step in profile.steps() {
        let Some(pass) = registry.pass(step.pass()) else {
            return Err(DecisionError::MissingPass {
                profile: profile.id(),
                pass: step.pass(),
            });
        };
        if !pass.supports_mode(step.mode()) {
            return Err(DecisionError::UnsupportedMode {
                profile: profile.id(),
                pass: pass.id(),
                mode: step.mode(),
            });
        }
        if step.mode() == ImplementationMode::Disabled {
            continue;
        }
        let oracle_step = step.mode() == ImplementationMode::Oracle
            || pass.determinism() == crate::DeterminismPolicy::Oracle;
        if profile.oracle_policy().forbids_oracle() && oracle_step {
            return Err(DecisionError::OracleModeForbidden {
                profile: profile.id(),
                pass: pass.id(),
            });
        }
        if oracle_step {
            oracle_involved = true;
        }

        validate_profile_inputs(profile, pass, &available)?;
        if validate_profile_outputs(registry, profile, pass, &mut available)? {
            oracle_involved = true;
        }
    }

    if profile.oracle_policy() == ProfileOraclePolicy::Require && !oracle_involved {
        return Err(DecisionError::OracleRequired {
            profile: profile.id(),
        });
    }
    validate_profile_exit(profile, &available)?;

    Ok(())
}

fn validate_profile_inputs(
    profile: &DecisionProfile,
    pass: &DecisionPassContract,
    available: &AvailableRepresentations,
) -> Result<(), DecisionError> {
    let any_of_groups = collect_any_of_groups(pass.inputs());
    let mut visited_groups = BTreeSet::new();

    for input in pass.inputs() {
        match input.requirement() {
            InputRequirement::Required => {
                validate_single_input(profile, pass, available, input)?;
            }
            InputRequirement::Optional => {
                let matches = available.matches_for_pass(input, pass);
                validate_optional_input(profile, pass, input, matches)?;
            }
            InputRequirement::AnyOf(group) => {
                if visited_groups.insert(group.as_str()) {
                    let Some(group_inputs) = any_of_groups.get(group.as_str()) else {
                        return Err(DecisionError::MissingProfileInput {
                            profile: profile.id(),
                            pass: pass.id(),
                            role: input.role(),
                            kind: input.kind(),
                            requirement: input.requirement().clone(),
                        });
                    };
                    validate_any_of_group(profile, pass, available, group, group_inputs)?;
                }
            }
        }
    }

    Ok(())
}

fn validate_single_input(
    profile: &DecisionProfile,
    pass: &DecisionPassContract,
    available: &AvailableRepresentations,
    input: &crate::RepresentationInput,
) -> Result<(), DecisionError> {
    let matches = available.matches_for_pass(input, pass);
    if matches.allowed == 0 {
        return Err(missing_or_disallowed_context(profile, pass, input, matches));
    }
    validate_binding(profile, pass, input, matches.allowed)
}

fn validate_optional_input(
    profile: &DecisionProfile,
    pass: &DecisionPassContract,
    input: &crate::RepresentationInput,
    matches: InputMatchCounts,
) -> Result<(), DecisionError> {
    if matches.allowed == 0 {
        Ok(())
    } else {
        validate_binding(profile, pass, input, matches.allowed)
    }
}

fn validate_any_of_group(
    profile: &DecisionProfile,
    pass: &DecisionPassContract,
    available: &AvailableRepresentations,
    group: &str,
    inputs: &[&crate::RepresentationInput],
) -> Result<(), DecisionError> {
    let mut satisfiable = 0;
    let mut first_missing = None;
    let mut first_disallowed = None;

    for input in inputs {
        let matches = available.matches_for_pass(input, pass);
        if matches.allowed == 0 {
            if first_missing.is_none() {
                first_missing = Some(*input);
            }
            if first_disallowed.is_none() {
                first_disallowed = matches.disallowed_context;
            }
            continue;
        }

        validate_optional_input(profile, pass, input, matches)?;
        satisfiable += 1;
    }

    match satisfiable {
        0 => {
            let input = first_missing.unwrap_or(inputs[0]);
            if let Some(context) = first_disallowed {
                Err(DecisionError::ContextInputNotAllowed {
                    profile: profile.id(),
                    pass: pass.id(),
                    context,
                    role: input.role(),
                })
            } else {
                Err(DecisionError::MissingProfileInput {
                    profile: profile.id(),
                    pass: pass.id(),
                    role: input.role(),
                    kind: input.kind(),
                    requirement: InputRequirement::AnyOf(group.to_owned()),
                })
            }
        }
        1 => Ok(()),
        _ => Err(DecisionError::AmbiguousAnyOfInput {
            profile: profile.id(),
            pass: pass.id(),
            group: group.to_owned(),
        }),
    }
}

fn validate_binding(
    profile: &DecisionProfile,
    pass: &DecisionPassContract,
    input: &crate::RepresentationInput,
    matches: usize,
) -> Result<(), DecisionError> {
    match input.binding() {
        InputBinding::ExactlyOne if matches == 1 => Ok(()),
        InputBinding::ExactlyOne => Err(DecisionError::AmbiguousProfileInput {
            profile: profile.id(),
            pass: pass.id(),
            role: input.role(),
            kind: input.kind(),
            matches,
        }),
        InputBinding::AllAvailable => Ok(()),
    }
}

fn missing_or_disallowed_context(
    profile: &DecisionProfile,
    pass: &DecisionPassContract,
    input: &crate::RepresentationInput,
    matches: InputMatchCounts,
) -> DecisionError {
    if let Some(context) = matches.disallowed_context {
        DecisionError::ContextInputNotAllowed {
            profile: profile.id(),
            pass: pass.id(),
            context,
            role: input.role(),
        }
    } else {
        DecisionError::MissingProfileInput {
            profile: profile.id(),
            pass: pass.id(),
            role: input.role(),
            kind: input.kind(),
            requirement: input.requirement().clone(),
        }
    }
}

fn validate_profile_outputs(
    registry: &DecisionRegistry,
    profile: &DecisionProfile,
    pass: &DecisionPassContract,
    available: &mut AvailableRepresentations,
) -> Result<bool, DecisionError> {
    let mut oracle_involved = false;
    for output in pass.outputs() {
        let Some(representation) = registry.representation(output.kind()) else {
            return Err(DecisionError::MissingRepresentationKind {
                owner: pass.id(),
                kind: output.kind(),
            });
        };
        if profile.oracle_policy().forbids_oracle() && representation.is_oracle_only() {
            return Err(DecisionError::OracleArtifactForbidden {
                profile: profile.id(),
                kind: output.kind(),
            });
        }
        if representation.is_oracle_only() {
            oracle_involved = true;
        }
        available.insert_output(output.role(), output.kind());
    }

    Ok(oracle_involved)
}

fn validate_output_write_policy(
    pass: &DecisionPassContract,
    output: crate::RepresentationOutput,
    representation: &RepresentationKindDef,
) -> Result<(), DecisionError> {
    let compatible = match pass.write_policy() {
        PassWritePolicy::None => matches!(
            representation.authority(),
            RepresentationAuthority::Derived
                | RepresentationAuthority::Diagnostic
                | RepresentationAuthority::Oracle
        ),
        PassWritePolicy::ProposalOnly(targets) => match representation.authority() {
            RepresentationAuthority::ProposalTo(target) => targets.contains(&target),
            _ => false,
        },
        PassWritePolicy::ExecutableRequestOnly => {
            representation.authority() == RepresentationAuthority::ExecutableRequest
        }
        PassWritePolicy::ControlProposalOnly => {
            representation.authority() == RepresentationAuthority::ControlProposal
        }
        PassWritePolicy::DiagnosticOnly => {
            representation.authority() == RepresentationAuthority::Diagnostic
        }
    };

    if compatible {
        Ok(())
    } else {
        Err(DecisionError::WritePolicyAuthorityMismatch {
            pass: pass.id(),
            kind: output.kind(),
            role: output.role(),
            authority: representation.authority(),
        })
    }
}

fn validate_profile_exit(
    profile: &DecisionProfile,
    available: &AvailableRepresentations,
) -> Result<(), DecisionError> {
    if let Some(output) = profile.exit().output() {
        let matches = available.matches_for_output(output.role(), output.kind_id());
        match matches {
            0 => {
                return Err(DecisionError::MissingProfileOutput {
                    profile: profile.id(),
                    role: output.role(),
                    kind: output.kind_id(),
                });
            }
            1 => {}
            _ => {
                return Err(DecisionError::AmbiguousProfileOutput {
                    profile: profile.id(),
                    role: output.role(),
                    kind: output.kind_id(),
                    matches,
                });
            }
        }
    }

    Ok(())
}
