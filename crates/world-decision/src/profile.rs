use std::collections::BTreeSet;

use world_context::ContextProjectionKind;
use world_core::{DefinitionId, VersionAnchor};
use world_defs::DefinitionName;

use crate::{
    DecisionError, ImplementationMode, RepresentationRole,
    error::{empty_definition_field, empty_item_field},
};

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

/// Terminal output declaration for a checked decision profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionProfileOutput {
    role: RepresentationRole,
    kind: Option<DefinitionId>,
}

impl DecisionProfileOutput {
    /// Creates a terminal output declaration.
    #[must_use]
    pub const fn new(role: RepresentationRole, kind: Option<DefinitionId>) -> Self {
        Self { role, kind }
    }

    /// Creates a terminal output declaration for a concrete representation kind.
    #[must_use]
    pub const fn kind(role: RepresentationRole, kind: DefinitionId) -> Self {
        Self {
            role,
            kind: Some(kind),
        }
    }

    /// Returns the terminal broad role.
    #[must_use]
    pub const fn role(self) -> RepresentationRole {
        self.role
    }

    /// Returns the terminal concrete kind, if required.
    #[must_use]
    pub const fn kind_id(self) -> Option<DefinitionId> {
        self.kind
    }
}

/// Exit contract for a checked decision profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionProfileExit {
    output: Option<DecisionProfileOutput>,
    abstention_allowed: bool,
}

impl DecisionProfileExit {
    /// Creates a profile exit contract.
    pub fn new(
        output: Option<DecisionProfileOutput>,
        abstention_allowed: bool,
    ) -> Result<Self, DecisionError> {
        if output.is_none() && !abstention_allowed {
            return Err(empty_item_field("DecisionProfileExit", "output"));
        }

        Ok(Self {
            output,
            abstention_allowed,
        })
    }

    /// Creates an exit contract with one required terminal output.
    #[must_use]
    pub fn terminal(output: DecisionProfileOutput) -> Self {
        Self {
            output: Some(output),
            abstention_allowed: false,
        }
    }

    /// Creates an exit contract that accepts either one terminal output or abstention.
    #[must_use]
    pub fn terminal_or_abstain(output: DecisionProfileOutput) -> Self {
        Self {
            output: Some(output),
            abstention_allowed: true,
        }
    }

    /// Creates an exit contract that allows abstention without terminal output.
    #[must_use]
    pub const fn abstention() -> Self {
        Self {
            output: None,
            abstention_allowed: true,
        }
    }

    /// Returns the terminal output declaration, if this profile can complete with one.
    #[must_use]
    pub const fn output(&self) -> Option<DecisionProfileOutput> {
        self.output
    }

    /// Returns whether the profile may intentionally abstain.
    #[must_use]
    pub const fn abstention_allowed(&self) -> bool {
        self.abstention_allowed
    }
}

/// Checked static decision-profile declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionProfile {
    id: DefinitionId,
    name: DefinitionName,
    context_inputs: BTreeSet<ContextProjectionKind>,
    steps: Vec<DecisionProfileStep>,
    exit: DecisionProfileExit,
    oracle_policy: ProfileOraclePolicy,
    version: VersionAnchor,
}

impl DecisionProfile {
    /// Creates a checked profile declaration with local invariants.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: DefinitionId,
        name: DefinitionName,
        context_inputs: impl IntoIterator<Item = ContextProjectionKind>,
        steps: impl IntoIterator<Item = DecisionProfileStep>,
        exit: DecisionProfileExit,
        oracle_policy: ProfileOraclePolicy,
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
            exit,
            oracle_policy,
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

    /// Returns profile exit contract.
    #[must_use]
    pub const fn exit(&self) -> &DecisionProfileExit {
        &self.exit
    }

    /// Returns oracle policy metadata.
    #[must_use]
    pub const fn oracle_policy(&self) -> ProfileOraclePolicy {
        self.oracle_policy
    }

    /// Returns the version anchor.
    #[must_use]
    pub const fn version(&self) -> VersionAnchor {
        self.version
    }
}
