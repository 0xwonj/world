use std::collections::BTreeMap;

use world_context::ContextProjectionKind;
use world_core::DefinitionId;

use crate::{DecisionPassContract, InputRequirement, RepresentationInput, RepresentationRole};

#[derive(Clone, Debug, Default)]
pub(super) struct AvailableRepresentations {
    entries: Vec<AvailableRepresentation>,
}

impl AvailableRepresentations {
    pub(super) fn from_context_inputs(
        context_inputs: impl Iterator<Item = ContextProjectionKind>,
    ) -> Self {
        let mut available = Self::default();
        for context in context_inputs {
            for role in context_roles(context) {
                available.entries.push(AvailableRepresentation {
                    role,
                    kind: None,
                    source: AvailableSource::Context(context),
                });
            }
        }
        available
    }

    pub(super) fn insert_output(&mut self, role: RepresentationRole, kind: DefinitionId) {
        self.entries.push(AvailableRepresentation {
            role,
            kind: Some(kind),
            source: AvailableSource::PassOutput,
        });
    }

    pub(super) fn matches_for_pass(
        &self,
        input: &RepresentationInput,
        pass: &DecisionPassContract,
    ) -> InputMatchCounts {
        let mut counts = InputMatchCounts::default();
        for entry in self
            .entries
            .iter()
            .filter(|entry| entry.satisfies(input.role(), input.kind()))
        {
            match entry.source {
                AvailableSource::Context(context) if pass.allows_context(context) => {
                    counts.allowed += 1;
                }
                AvailableSource::Context(context) => {
                    counts.disallowed_context.get_or_insert(context);
                }
                AvailableSource::PassOutput => {
                    counts.allowed += 1;
                }
            }
        }
        counts
    }

    pub(super) fn matches_for_output(
        &self,
        role: RepresentationRole,
        kind: Option<DefinitionId>,
    ) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.source == AvailableSource::PassOutput)
            .filter(|entry| entry.satisfies(role, kind))
            .count()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AvailableRepresentation {
    role: RepresentationRole,
    kind: Option<DefinitionId>,
    source: AvailableSource,
}

impl AvailableRepresentation {
    fn satisfies(&self, role: RepresentationRole, kind: Option<DefinitionId>) -> bool {
        self.role == role
            && match kind {
                Some(kind) => self.kind == Some(kind),
                None => true,
            }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AvailableSource {
    Context(ContextProjectionKind),
    PassOutput,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct InputMatchCounts {
    pub(super) allowed: usize,
    pub(super) disallowed_context: Option<ContextProjectionKind>,
}

pub(super) fn collect_any_of_groups<'a>(
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

pub(crate) fn context_roles(
    context: ContextProjectionKind,
) -> impl Iterator<Item = RepresentationRole> {
    match context {
        ContextProjectionKind::Observation => [
            RepresentationRole::ActorRelativeView,
            RepresentationRole::ObservationView,
        ]
        .as_slice(),
        ContextProjectionKind::Epistemic => [
            RepresentationRole::ActorRelativeView,
            RepresentationRole::EpistemicView,
        ]
        .as_slice(),
        ContextProjectionKind::Social => [
            RepresentationRole::ActorRelativeView,
            RepresentationRole::SocialContextView,
        ]
        .as_slice(),
        ContextProjectionKind::Capability => [RepresentationRole::CapabilitySet].as_slice(),
        ContextProjectionKind::Repertoire => [RepresentationRole::ActionRepertoire].as_slice(),
        ContextProjectionKind::Affordance => [RepresentationRole::AffordanceView].as_slice(),
        _ => [].as_slice(),
    }
    .iter()
    .copied()
}
