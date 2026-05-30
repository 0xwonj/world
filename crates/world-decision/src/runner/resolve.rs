use std::collections::{BTreeMap, BTreeSet};

use world_context::ContextProjectionKind;
use world_core::DefinitionId;

use crate::{
    DecisionError, DecisionInputRef, DecisionPassContract, DecisionProfile, InputBinding,
    InputRequirement, RepresentationInput, RepresentationRole, ResolvedDecisionInput,
    registry::flow::context_roles,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct ExecutionFlow {
    entries: Vec<FlowEntry>,
}

impl ExecutionFlow {
    pub(crate) fn from_context_inputs(
        context_inputs: impl Iterator<Item = ContextProjectionKind>,
    ) -> Self {
        let mut flow = Self::default();
        for context in context_inputs {
            for role in context_roles(context) {
                flow.entries.push(FlowEntry {
                    role,
                    kind: None,
                    source: DecisionInputRef::Context(context),
                });
            }
        }
        flow
    }

    pub(crate) fn insert_artifact(
        &mut self,
        role: RepresentationRole,
        kind: DefinitionId,
        artifact: crate::DecisionArtifactRef,
    ) {
        self.entries.push(FlowEntry {
            role,
            kind: Some(kind),
            source: DecisionInputRef::Artifact(artifact),
        });
    }

    pub(crate) fn resolve_pass_inputs(
        &self,
        profile: &DecisionProfile,
        pass: &DecisionPassContract,
    ) -> Result<Vec<ResolvedDecisionInput>, DecisionError> {
        let any_of_groups = collect_any_of_groups(pass.inputs());
        let mut visited_groups = BTreeSet::new();
        let mut resolved = Vec::new();

        for input in pass.inputs() {
            match input.requirement() {
                InputRequirement::Required => {
                    resolved.extend(self.resolve_single(profile, pass, input)?);
                }
                InputRequirement::Optional => {
                    resolved.extend(self.resolve_optional(profile, pass, input)?);
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
                        resolved.extend(self.resolve_any_of_group(
                            profile,
                            pass,
                            group,
                            group_inputs,
                        )?);
                    }
                }
            }
        }

        Ok(resolved)
    }

    pub(crate) fn resolve_exit(
        &self,
        profile: &DecisionProfile,
    ) -> Result<crate::DecisionArtifactRef, DecisionError> {
        let Some(output) = profile.exit().output() else {
            return Err(DecisionError::MissingProfileOutput {
                profile: profile.id(),
                role: RepresentationRole::Diagnostic,
                kind: None,
            });
        };

        let matches = self
            .entries
            .iter()
            .filter_map(|entry| entry.artifact_for(output.role(), output.kind_id()))
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(DecisionError::MissingProfileOutput {
                profile: profile.id(),
                role: output.role(),
                kind: output.kind_id(),
            }),
            [artifact] => Ok(*artifact),
            _ => Err(DecisionError::AmbiguousProfileOutput {
                profile: profile.id(),
                role: output.role(),
                kind: output.kind_id(),
                matches: matches.len(),
            }),
        }
    }

    fn resolve_single(
        &self,
        profile: &DecisionProfile,
        pass: &DecisionPassContract,
        input: &RepresentationInput,
    ) -> Result<Vec<ResolvedDecisionInput>, DecisionError> {
        let matches = self.matches_for_pass(input, pass);
        if matches.allowed.is_empty() {
            return Err(missing_or_disallowed_context(profile, pass, input, matches));
        }
        validate_binding(profile, pass, input, matches.allowed.len())?;
        Ok(matches.allowed)
    }

    fn resolve_optional(
        &self,
        profile: &DecisionProfile,
        pass: &DecisionPassContract,
        input: &RepresentationInput,
    ) -> Result<Vec<ResolvedDecisionInput>, DecisionError> {
        let matches = self.matches_for_pass(input, pass);
        if matches.allowed.is_empty() {
            Ok(Vec::new())
        } else {
            validate_binding(profile, pass, input, matches.allowed.len())?;
            Ok(matches.allowed)
        }
    }

    fn resolve_any_of_group(
        &self,
        profile: &DecisionProfile,
        pass: &DecisionPassContract,
        group: &str,
        inputs: &[&RepresentationInput],
    ) -> Result<Vec<ResolvedDecisionInput>, DecisionError> {
        let mut satisfiable = Vec::new();
        let mut first_missing = None;
        let mut first_disallowed = None;

        for input in inputs {
            let matches = self.matches_for_pass(input, pass);
            if matches.allowed.is_empty() {
                if first_missing.is_none() {
                    first_missing = Some(*input);
                }
                if first_disallowed.is_none() {
                    first_disallowed = matches.disallowed_context;
                }
                continue;
            }

            validate_binding(profile, pass, input, matches.allowed.len())?;
            satisfiable.push(matches.allowed);
        }

        match satisfiable.len() {
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
            1 => Ok(satisfiable.remove(0)),
            _ => Err(DecisionError::AmbiguousAnyOfInput {
                profile: profile.id(),
                pass: pass.id(),
                group: group.to_owned(),
            }),
        }
    }

    fn matches_for_pass(
        &self,
        input: &RepresentationInput,
        pass: &DecisionPassContract,
    ) -> ResolvedMatches {
        let mut matches = ResolvedMatches::default();
        for entry in self
            .entries
            .iter()
            .filter(|entry| entry.satisfies(input.role(), input.kind()))
        {
            match entry.source {
                DecisionInputRef::Context(context) if pass.allows_context(context) => {
                    matches
                        .allowed
                        .push(ResolvedDecisionInput::new(entry.source, input.role()));
                }
                DecisionInputRef::Context(context) => {
                    matches.disallowed_context.get_or_insert(context);
                }
                DecisionInputRef::Artifact(_) => {
                    matches
                        .allowed
                        .push(ResolvedDecisionInput::new(entry.source, input.role()));
                }
            }
        }
        matches
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FlowEntry {
    role: RepresentationRole,
    kind: Option<DefinitionId>,
    source: DecisionInputRef,
}

impl FlowEntry {
    fn satisfies(&self, role: RepresentationRole, kind: Option<DefinitionId>) -> bool {
        self.role == role
            && match kind {
                Some(kind) => self.kind == Some(kind),
                None => true,
            }
    }

    fn artifact_for(
        &self,
        role: RepresentationRole,
        kind: Option<DefinitionId>,
    ) -> Option<crate::DecisionArtifactRef> {
        if !self.satisfies(role, kind) {
            return None;
        }
        match self.source {
            DecisionInputRef::Artifact(artifact) => Some(artifact),
            DecisionInputRef::Context(_) => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct ResolvedMatches {
    allowed: Vec<ResolvedDecisionInput>,
    disallowed_context: Option<ContextProjectionKind>,
}

fn validate_binding(
    profile: &DecisionProfile,
    pass: &DecisionPassContract,
    input: &RepresentationInput,
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
    input: &RepresentationInput,
    matches: ResolvedMatches,
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

fn collect_any_of_groups<'a>(
    inputs: &'a [RepresentationInput],
) -> BTreeMap<&'a str, Vec<&'a RepresentationInput>> {
    let mut groups: BTreeMap<&'a str, Vec<&'a RepresentationInput>> = BTreeMap::new();
    for input in inputs {
        if let InputRequirement::AnyOf(group) = input.requirement() {
            groups.entry(group.as_str()).or_default().push(input);
        }
    }
    groups
}
