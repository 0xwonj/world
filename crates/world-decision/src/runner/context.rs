use std::collections::BTreeSet;

use world_context::{
    ActionRepertoire, ActorContextProjection, CapabilitySet, ContextProjectionKind,
    EpistemicWorkingSet, ObservationContext, PerceivedAffordance, SocialContextView,
};

use crate::{
    DecisionArtifact, DecisionArtifactStore, DecisionInputRef, DecisionPassContract,
    DecisionProfile, ImplementationMode, RepresentationRole,
};

/// One resolved input made available to a pass executor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedDecisionInput {
    reference: DecisionInputRef,
    role: RepresentationRole,
}

impl ResolvedDecisionInput {
    /// Creates a resolved input.
    #[must_use]
    pub(crate) const fn new(reference: DecisionInputRef, role: RepresentationRole) -> Self {
        Self { reference, role }
    }

    /// Returns the input reference.
    #[must_use]
    pub const fn reference(self) -> DecisionInputRef {
        self.reference
    }

    /// Returns the role this input satisfies.
    #[must_use]
    pub const fn role(self) -> RepresentationRole {
        self.role
    }
}

/// Restricted actor-context view exposed to a decision pass executor.
#[derive(Clone, Copy)]
pub struct DecisionContextView<'a> {
    projection: &'a ActorContextProjection,
    allowed: &'a BTreeSet<ContextProjectionKind>,
}

impl<'a> DecisionContextView<'a> {
    pub(crate) fn new(
        projection: &'a ActorContextProjection,
        allowed: &'a BTreeSet<ContextProjectionKind>,
    ) -> Self {
        Self {
            projection,
            allowed,
        }
    }

    /// Returns observed state and event context when permitted.
    #[must_use]
    pub fn observations(&self) -> Option<&'a ObservationContext> {
        self.allowed(ContextProjectionKind::Observation)
            .then_some(self.projection.context().observations())
    }

    /// Returns actor-owned epistemic context when permitted.
    #[must_use]
    pub fn epistemic(&self) -> Option<&'a EpistemicWorkingSet> {
        self.allowed(ContextProjectionKind::Epistemic)
            .then_some(self.projection.context().epistemic())
    }

    /// Returns actor-relative social context when permitted.
    #[must_use]
    pub fn social(&self) -> Option<&'a SocialContextView> {
        self.allowed(ContextProjectionKind::Social)
            .then_some(self.projection.context().social())
    }

    /// Returns actor capability context when permitted.
    #[must_use]
    pub fn capabilities(&self) -> Option<&'a CapabilitySet> {
        self.allowed(ContextProjectionKind::Capability)
            .then_some(self.projection.context().capabilities())
    }

    /// Returns actor action repertoire when permitted.
    #[must_use]
    pub fn repertoire(&self) -> Option<&'a ActionRepertoire> {
        self.allowed(ContextProjectionKind::Repertoire)
            .then_some(self.projection.context().repertoire())
    }

    /// Returns perceived affordances when permitted.
    #[must_use]
    pub fn affordances(&self) -> Option<&'a [PerceivedAffordance]> {
        self.allowed(ContextProjectionKind::Affordance)
            .then_some(self.projection.context().affordances())
    }

    /// Returns whether a projection family is visible through this context.
    #[must_use]
    pub fn allowed(&self, projection: ContextProjectionKind) -> bool {
        self.allowed.contains(&projection)
    }
}

/// Restricted execution context passed to a trusted decision pass executor.
#[derive(Clone, Copy)]
pub struct DecisionPassExecutionContext<'a> {
    profile: &'a DecisionProfile,
    pass: &'a DecisionPassContract,
    mode: ImplementationMode,
    actor_context: DecisionContextView<'a>,
    inputs: &'a [ResolvedDecisionInput],
    artifacts: &'a DecisionArtifactStore,
}

impl<'a> DecisionPassExecutionContext<'a> {
    pub(crate) fn new(
        profile: &'a DecisionProfile,
        pass: &'a DecisionPassContract,
        mode: ImplementationMode,
        actor_context: DecisionContextView<'a>,
        inputs: &'a [ResolvedDecisionInput],
        artifacts: &'a DecisionArtifactStore,
    ) -> Self {
        Self {
            profile,
            pass,
            mode,
            actor_context,
            inputs,
            artifacts,
        }
    }

    /// Returns profile metadata.
    #[must_use]
    pub const fn profile(&self) -> &DecisionProfile {
        self.profile
    }

    /// Returns pass contract metadata.
    #[must_use]
    pub const fn pass(&self) -> &DecisionPassContract {
        self.pass
    }

    /// Returns selected implementation mode.
    #[must_use]
    pub const fn mode(&self) -> ImplementationMode {
        self.mode
    }

    /// Returns restricted actor-context view.
    #[must_use]
    pub const fn actor_context(&self) -> DecisionContextView<'a> {
        self.actor_context
    }

    /// Returns resolved pass inputs.
    #[must_use]
    pub const fn inputs(&self) -> &[ResolvedDecisionInput] {
        self.inputs
    }

    /// Looks up an artifact only when it was resolved as an input to this pass.
    #[must_use]
    pub fn artifact(&self, input: ResolvedDecisionInput) -> Option<&DecisionArtifact> {
        if !self.inputs.contains(&input) {
            return None;
        }
        match input.reference() {
            DecisionInputRef::Artifact(artifact) => self.artifacts.get(artifact),
            DecisionInputRef::Context(_) => None,
        }
    }
}
