use std::collections::BTreeSet;

use world_context::ContextProjectionKind;
use world_core::{AuthorityClass, DefinitionId, VersionAnchor};
use world_defs::DefinitionName;

use crate::{
    DecisionError, RepresentationRole,
    error::{empty_definition_field, empty_item_field},
};

/// Coarse architecture label for a decision pass contract.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PassClass {
    /// Derives decision-safe values from actor context.
    ContextDerivation,
    /// Grounds raw context into semantic or social-cognitive artifacts.
    SemanticGrounding,
    /// Produces cognitive, motivational, or appraisal-like signals.
    CognitiveSignal,
    /// Produces bounded models of another actor or institution.
    OtherModeling,
    /// Generates candidate choices, intents, or commitments.
    CandidateGeneration,
    /// Selects among candidates.
    Choice,
    /// Binds a choice into an activity-level plan.
    ActivityBinding,
    /// Produces a runtime-executable request candidate.
    ExecutionRequest,
    /// Validates artifacts without producing new domain state.
    Validation,
    /// Publishes a proposal or trace-bound artifact.
    Publication,
    /// Emits diagnostics only.
    Diagnostic,
}

impl PassClass {
    const fn allows_empty_outputs(self) -> bool {
        matches!(self, Self::Validation | Self::Diagnostic)
    }
}

/// Declared implementation mode for ablation and trace metadata.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImplementationMode {
    /// Hand-written deterministic rules.
    Rule,
    /// Heuristic implementation.
    Heuristic,
    /// External language-model implementation.
    Llm,
    /// Hybrid of structured code and external model/tooling.
    Hybrid,
    /// Oracle or ground-truth-assisted implementation.
    Oracle,
    /// Replay from recorded artifacts.
    Replay,
    /// Pass is intentionally skipped in the profile.
    Disabled,
}

/// Determinism and replay metadata for a pass contract.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeterminismPolicy {
    /// Fully deterministic.
    Deterministic,
    /// Deterministic when given an explicit seed.
    Seeded,
    /// Uses an external nondeterministic source.
    ExternalNondeterministic,
    /// Oracle-assisted or oracle-sourced.
    Oracle,
}

impl DeterminismPolicy {
    const fn accepts_mode(self, mode: ImplementationMode) -> bool {
        match mode {
            ImplementationMode::Llm | ImplementationMode::Hybrid => {
                matches!(self, Self::ExternalNondeterministic | Self::Oracle)
            }
            ImplementationMode::Oracle => matches!(self, Self::Oracle),
            ImplementationMode::Rule | ImplementationMode::Heuristic => {
                !matches!(self, Self::Oracle)
            }
            ImplementationMode::Replay | ImplementationMode::Disabled => true,
        }
    }
}

/// Requiredness policy for a pass input edge.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InputRequirement {
    /// Input must be available.
    Required,
    /// Input may be absent.
    Optional,
    /// One required input from the named group must be available.
    AnyOf(String),
}

impl InputRequirement {
    /// Returns whether this input must be satisfied by profile flow validation.
    #[must_use]
    pub fn is_required(&self) -> bool {
        matches!(self, Self::Required | Self::AnyOf(_))
    }
}

/// Binding semantics for a pass input once profile validation has found sources.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InputBinding {
    /// Exactly one source must satisfy the input.
    ExactlyOne,
    /// Every currently available matching source is passed to the step.
    AllAvailable,
}

/// One input role/kind expected by a decision pass.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepresentationInput {
    role: RepresentationRole,
    kind: Option<DefinitionId>,
    requirement: InputRequirement,
    binding: InputBinding,
}

impl RepresentationInput {
    /// Creates an input edge.
    pub fn new(
        role: RepresentationRole,
        kind: Option<DefinitionId>,
        requirement: InputRequirement,
    ) -> Result<Self, DecisionError> {
        Self::with_binding(role, kind, requirement, InputBinding::ExactlyOne)
    }

    /// Creates an input edge with explicit binding semantics.
    pub fn with_binding(
        role: RepresentationRole,
        kind: Option<DefinitionId>,
        requirement: InputRequirement,
        binding: InputBinding,
    ) -> Result<Self, DecisionError> {
        if let InputRequirement::AnyOf(group) = &requirement
            && group.trim().is_empty()
        {
            return Err(empty_item_field("RepresentationInput", "requirement"));
        }

        Ok(Self {
            role,
            kind,
            requirement,
            binding,
        })
    }

    /// Creates a required input for a broad role.
    pub fn required(role: RepresentationRole) -> Self {
        Self {
            role,
            kind: None,
            requirement: InputRequirement::Required,
            binding: InputBinding::ExactlyOne,
        }
    }

    /// Creates a required input that consumes every available source with the role.
    pub fn required_all(role: RepresentationRole) -> Self {
        Self {
            role,
            kind: None,
            requirement: InputRequirement::Required,
            binding: InputBinding::AllAvailable,
        }
    }

    /// Creates an optional input for a broad role.
    pub fn optional(role: RepresentationRole) -> Self {
        Self {
            role,
            kind: None,
            requirement: InputRequirement::Optional,
            binding: InputBinding::ExactlyOne,
        }
    }

    /// Creates an optional input that consumes every available source with the role.
    pub fn optional_all(role: RepresentationRole) -> Self {
        Self {
            role,
            kind: None,
            requirement: InputRequirement::Optional,
            binding: InputBinding::AllAvailable,
        }
    }

    /// Creates a required input for a concrete representation kind.
    pub fn required_kind(role: RepresentationRole, kind: DefinitionId) -> Self {
        Self {
            role,
            kind: Some(kind),
            requirement: InputRequirement::Required,
            binding: InputBinding::ExactlyOne,
        }
    }

    /// Creates an optional input for a concrete representation kind.
    pub fn optional_kind(role: RepresentationRole, kind: DefinitionId) -> Self {
        Self {
            role,
            kind: Some(kind),
            requirement: InputRequirement::Optional,
            binding: InputBinding::ExactlyOne,
        }
    }

    /// Returns the required broad role.
    #[must_use]
    pub const fn role(&self) -> RepresentationRole {
        self.role
    }

    /// Returns the concrete kind requirement, if present.
    #[must_use]
    pub const fn kind(&self) -> Option<DefinitionId> {
        self.kind
    }

    /// Returns the input requiredness policy.
    #[must_use]
    pub const fn requirement(&self) -> &InputRequirement {
        &self.requirement
    }

    /// Returns binding semantics used after profile validation resolves sources.
    #[must_use]
    pub const fn binding(&self) -> InputBinding {
        self.binding
    }
}

/// One output role/kind produced by a decision pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepresentationOutput {
    role: RepresentationRole,
    kind: DefinitionId,
}

impl RepresentationOutput {
    /// Creates an output edge.
    #[must_use]
    pub const fn new(role: RepresentationRole, kind: DefinitionId) -> Self {
        Self { role, kind }
    }

    /// Returns the produced broad role.
    #[must_use]
    pub const fn role(self) -> RepresentationRole {
        self.role
    }

    /// Returns the produced concrete representation kind.
    #[must_use]
    pub const fn kind(self) -> DefinitionId {
        self.kind
    }
}

/// Write/publication declaration for a pass.
///
/// This is a checked declaration for later routing. It does not grant direct
/// authority to mutate world state.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PassWritePolicy {
    /// The pass writes nothing.
    None,
    /// The pass may produce proposals for accepted non-hard authority gates.
    ProposalOnly(BTreeSet<AuthorityClass>),
    /// The pass may produce an executable request candidate.
    ExecutableRequestOnly,
    /// The pass may produce a runtime-control proposal candidate.
    ControlProposalOnly,
    /// The pass emits diagnostics only.
    DiagnosticOnly,
}

impl PassWritePolicy {
    /// Returns whether this policy declares no writes.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns whether this policy is compatible with an output role.
    #[must_use]
    pub fn accepts_role(&self, role: RepresentationRole) -> bool {
        match self {
            Self::None => !matches!(
                role,
                RepresentationRole::ExecutableRequest | RepresentationRole::NonHardUpdateProposal
            ),
            Self::ProposalOnly(_) | Self::ControlProposalOnly => {
                matches!(role, RepresentationRole::NonHardUpdateProposal)
            }
            Self::ExecutableRequestOnly => matches!(role, RepresentationRole::ExecutableRequest),
            Self::DiagnosticOnly => matches!(role, RepresentationRole::Diagnostic),
        }
    }
}

/// Checked declaration for a pass that may later be executed by a runner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionPassContract {
    id: DefinitionId,
    name: DefinitionName,
    class: PassClass,
    inputs: Vec<RepresentationInput>,
    outputs: Vec<RepresentationOutput>,
    allowed_context: BTreeSet<ContextProjectionKind>,
    allowed_authority_reads: BTreeSet<AuthorityClass>,
    forbidden_authority_reads: BTreeSet<AuthorityClass>,
    write_policy: PassWritePolicy,
    implementation_modes: BTreeSet<ImplementationMode>,
    determinism: DeterminismPolicy,
    version: VersionAnchor,
}

impl DecisionPassContract {
    /// Creates a checked pass declaration with local invariants.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: DefinitionId,
        name: DefinitionName,
        class: PassClass,
        inputs: impl IntoIterator<Item = RepresentationInput>,
        outputs: impl IntoIterator<Item = RepresentationOutput>,
        allowed_context: impl IntoIterator<Item = ContextProjectionKind>,
        allowed_authority_reads: impl IntoIterator<Item = AuthorityClass>,
        forbidden_authority_reads: impl IntoIterator<Item = AuthorityClass>,
        write_policy: PassWritePolicy,
        implementation_modes: impl IntoIterator<Item = ImplementationMode>,
        determinism: DeterminismPolicy,
        version: VersionAnchor,
    ) -> Result<Self, DecisionError> {
        let inputs = inputs.into_iter().collect::<Vec<_>>();
        let outputs = outputs.into_iter().collect::<Vec<_>>();
        if outputs.is_empty() && !class.allows_empty_outputs() {
            return Err(empty_definition_field(
                id,
                "DecisionPassContract",
                "outputs",
            ));
        }

        let allowed_context = allowed_context.into_iter().collect::<BTreeSet<_>>();
        let allowed_authority_reads = allowed_authority_reads.into_iter().collect::<BTreeSet<_>>();
        let forbidden_authority_reads = forbidden_authority_reads
            .into_iter()
            .collect::<BTreeSet<_>>();
        if let Some(authority) = allowed_authority_reads
            .intersection(&forbidden_authority_reads)
            .next()
        {
            return Err(DecisionError::ConflictingAuthorityRead {
                pass: id,
                authority: *authority,
            });
        }

        validate_write_policy(id, class, &write_policy, &outputs)?;

        let implementation_modes = implementation_modes.into_iter().collect::<BTreeSet<_>>();
        if implementation_modes.is_empty() {
            return Err(empty_definition_field(
                id,
                "DecisionPassContract",
                "implementation_modes",
            ));
        }
        for mode in &implementation_modes {
            if !determinism.accepts_mode(*mode) {
                return Err(DecisionError::DeterminismMismatch {
                    pass: id,
                    mode: *mode,
                    determinism,
                });
            }
        }
        for input in &inputs {
            if let InputRequirement::AnyOf(group) = input.requirement()
                && group.trim().is_empty()
            {
                return Err(DecisionError::EmptyInputGroup { pass: id });
            }
        }

        Ok(Self {
            id,
            name,
            class,
            inputs,
            outputs,
            allowed_context,
            allowed_authority_reads,
            forbidden_authority_reads,
            write_policy,
            implementation_modes,
            determinism,
            version,
        })
    }

    /// Returns the pass id.
    #[must_use]
    pub const fn id(&self) -> DefinitionId {
        self.id
    }

    /// Returns the pass name.
    #[must_use]
    pub const fn name(&self) -> &DefinitionName {
        &self.name
    }

    /// Returns the pass class.
    #[must_use]
    pub const fn class(&self) -> PassClass {
        self.class
    }

    /// Returns declared input edges.
    #[must_use]
    pub fn inputs(&self) -> &[RepresentationInput] {
        &self.inputs
    }

    /// Returns declared output edges.
    #[must_use]
    pub fn outputs(&self) -> &[RepresentationOutput] {
        &self.outputs
    }

    /// Returns actor-context projection families this pass may inspect.
    pub fn allowed_context(&self) -> impl Iterator<Item = ContextProjectionKind> + '_ {
        self.allowed_context.iter().copied()
    }

    /// Returns whether this pass may inspect a context projection family.
    #[must_use]
    pub fn allows_context(&self, context: ContextProjectionKind) -> bool {
        self.allowed_context.contains(&context)
    }

    /// Returns allowed authority read classes.
    pub fn allowed_authority_reads(&self) -> impl Iterator<Item = AuthorityClass> + '_ {
        self.allowed_authority_reads.iter().copied()
    }

    /// Returns forbidden authority read classes.
    pub fn forbidden_authority_reads(&self) -> impl Iterator<Item = AuthorityClass> + '_ {
        self.forbidden_authority_reads.iter().copied()
    }

    /// Returns write/publication metadata.
    #[must_use]
    pub const fn write_policy(&self) -> &PassWritePolicy {
        &self.write_policy
    }

    /// Returns implementation modes in deterministic order.
    pub fn implementation_modes(&self) -> impl Iterator<Item = ImplementationMode> + '_ {
        self.implementation_modes.iter().copied()
    }

    /// Returns whether this pass supports the implementation mode.
    #[must_use]
    pub fn supports_mode(&self, mode: ImplementationMode) -> bool {
        self.implementation_modes.contains(&mode)
    }

    /// Returns determinism metadata.
    #[must_use]
    pub const fn determinism(&self) -> DeterminismPolicy {
        self.determinism
    }

    /// Returns the version anchor.
    #[must_use]
    pub const fn version(&self) -> VersionAnchor {
        self.version
    }
}

fn validate_write_policy(
    pass: DefinitionId,
    class: PassClass,
    policy: &PassWritePolicy,
    outputs: &[RepresentationOutput],
) -> Result<(), DecisionError> {
    let class_matches_policy = match policy {
        PassWritePolicy::None => true,
        PassWritePolicy::ExecutableRequestOnly => class == PassClass::ExecutionRequest,
        PassWritePolicy::ProposalOnly(_) | PassWritePolicy::ControlProposalOnly => {
            class == PassClass::Publication
        }
        PassWritePolicy::DiagnosticOnly => class == PassClass::Diagnostic,
    };
    if !class_matches_policy {
        return Err(DecisionError::WritePolicyClassMismatch { pass, class });
    }

    if let PassWritePolicy::ProposalOnly(targets) = policy {
        if targets.is_empty() {
            return Err(empty_definition_field(
                pass,
                "DecisionPassContract",
                "write_policy",
            ));
        }
        if targets.contains(&AuthorityClass::Hard) {
            return Err(DecisionError::HardMutationAuthority { pass });
        }
    }

    for output in outputs {
        if !policy.accepts_role(output.role()) {
            return Err(DecisionError::WritePolicyRoleMismatch {
                pass,
                role: output.role(),
            });
        }
    }

    Ok(())
}
