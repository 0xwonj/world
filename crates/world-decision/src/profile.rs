use std::collections::BTreeSet;

use world_context::ContextProjectionKind;
use world_core::{DefinitionId, VersionAnchor};
use world_defs::DefinitionName;

use crate::{DecisionError, ImplementationMode, TracePolicy, error::empty_definition_field};

/// Oracle policy for a checked decision profile.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProfileOraclePolicy {
    /// Oracle modes and oracle-only artifacts are forbidden.
    Forbid,
    /// Oracle modes and artifacts are allowed when declared by pass contracts.
    Allow,
    /// The profile is explicitly oracle-labeled.
    Require,
}

impl ProfileOraclePolicy {
    /// Returns whether oracle involvement is forbidden by this profile.
    #[must_use]
    pub const fn forbids_oracle(self) -> bool {
        matches!(self, Self::Forbid)
    }

    /// Returns whether this profile is explicitly oracle-labeled.
    #[must_use]
    pub const fn is_oracle_labeled(self) -> bool {
        matches!(self, Self::Allow | Self::Require)
    }
}

/// One pass invocation in a decision profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionProfileStep {
    pass: DefinitionId,
    mode: ImplementationMode,
}

impl DecisionProfileStep {
    /// Creates a profile step.
    #[must_use]
    pub const fn new(pass: DefinitionId, mode: ImplementationMode) -> Self {
        Self { pass, mode }
    }

    /// Returns the invoked pass id.
    #[must_use]
    pub const fn pass(self) -> DefinitionId {
        self.pass
    }

    /// Returns the selected implementation mode.
    #[must_use]
    pub const fn mode(self) -> ImplementationMode {
        self.mode
    }
}

/// Checked static decision-profile declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionProfile {
    id: DefinitionId,
    name: DefinitionName,
    context_inputs: BTreeSet<ContextProjectionKind>,
    steps: Vec<DecisionProfileStep>,
    oracle_policy: ProfileOraclePolicy,
    trace_policy: TracePolicy,
    version: VersionAnchor,
}

impl DecisionProfile {
    /// Creates a checked profile declaration with local invariants.
    pub fn new(
        id: DefinitionId,
        name: DefinitionName,
        context_inputs: impl IntoIterator<Item = ContextProjectionKind>,
        steps: impl IntoIterator<Item = DecisionProfileStep>,
        oracle_policy: ProfileOraclePolicy,
        trace_policy: TracePolicy,
        version: VersionAnchor,
    ) -> Result<Self, DecisionError> {
        let context_inputs = context_inputs.into_iter().collect::<BTreeSet<_>>();
        let steps = steps.into_iter().collect::<Vec<_>>();
        if steps.is_empty() {
            return Err(empty_definition_field(id, "DecisionProfile", "steps"));
        }

        Ok(Self {
            id,
            name,
            context_inputs,
            steps,
            oracle_policy,
            trace_policy,
            version,
        })
    }

    /// Returns the profile id.
    #[must_use]
    pub const fn id(&self) -> DefinitionId {
        self.id
    }

    /// Returns the profile name.
    #[must_use]
    pub const fn name(&self) -> &DefinitionName {
        &self.name
    }

    /// Returns required context projection families in deterministic order.
    pub fn context_inputs(&self) -> impl Iterator<Item = ContextProjectionKind> + '_ {
        self.context_inputs.iter().copied()
    }

    /// Returns profile steps in declared order.
    #[must_use]
    pub fn steps(&self) -> &[DecisionProfileStep] {
        &self.steps
    }

    /// Returns oracle policy metadata.
    #[must_use]
    pub const fn oracle_policy(&self) -> ProfileOraclePolicy {
        self.oracle_policy
    }

    /// Returns trace metadata.
    #[must_use]
    pub const fn trace_policy(&self) -> TracePolicy {
        self.trace_policy
    }

    /// Returns the version anchor.
    #[must_use]
    pub const fn version(&self) -> VersionAnchor {
        self.version
    }
}
