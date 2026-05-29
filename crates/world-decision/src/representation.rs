use std::collections::BTreeSet;

use world_core::{AuthorityClass, DefinitionId, VersionAnchor};
use world_defs::DefinitionName;

use crate::error::{DecisionError, empty_definition_field};

/// Broad compatibility role for a decision representation kind.
///
/// Roles are intentionally wider than concrete artifact schemas. They let
/// checked profiles reason about whether one pass can feed another while
/// keeping theory-specific schemas outside the substrate.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RepresentationRole {
    /// Full actor-relative context view.
    ActorRelativeView,
    /// Actor-visible observation view.
    ObservationView,
    /// Actor-owned epistemic view.
    EpistemicView,
    /// Actor-relative social context view.
    SocialContextView,
    /// Actor capability evidence set.
    CapabilitySet,
    /// Actor action-repertoire view.
    ActionRepertoire,
    /// Actor-visible affordance view.
    AffordanceView,
    /// Speech options or speech context available to the actor.
    SpeechSurface,
    /// A typed speech-act artifact.
    SpeechAct,
    /// Generic decision signal.
    DecisionSignal,
    /// Motivation or appraisal-like signal.
    MotivationalSignal,
    /// Strategic assessment artifact.
    StrategicAssessment,
    /// Bounded model of another actor or institution.
    OtherModelView,
    /// Candidate commitment or promise artifact.
    CommitmentCandidate,
    /// Candidate intent artifact.
    IntentCandidate,
    /// Selected choice artifact.
    Choice,
    /// Activity-level plan artifact.
    ActivityPlan,
    /// Runtime-executable request artifact.
    ExecutableRequest,
    /// Proposal for an accepted non-hard authority gate.
    NonHardUpdateProposal,
    /// Diagnostic-only artifact.
    Diagnostic,
}

/// Actor/research visibility of a decision artifact.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RepresentationVisibility {
    /// Safe to expose to the acting actor.
    ActorVisible,
    /// Internal engine artifact.
    EngineInternal,
    /// Research trace artifact.
    ResearchTrace,
    /// Oracle-only artifact that must be explicitly labeled by profiles.
    OracleOnly,
    /// Diagnostic-only artifact.
    DiagnosticOnly,
}

impl RepresentationVisibility {
    /// Returns whether this visibility requires oracle-aware profile labeling.
    #[must_use]
    pub const fn is_oracle_only(self) -> bool {
        matches!(self, Self::OracleOnly)
    }

    /// Returns whether this visibility is diagnostic-only.
    #[must_use]
    pub const fn is_diagnostic_only(self) -> bool {
        matches!(self, Self::DiagnosticOnly)
    }
}

/// Lifetime and persistence expectation for a decision artifact.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RepresentationPersistence {
    /// Artifact exists only while evaluating one decision.
    Ephemeral,
    /// Artifact is intended to be recorded in a research/runtime trace.
    TraceRecorded,
    /// Artifact is a proposal, not accepted authority.
    ProposalOnly,
    /// Accepted elsewhere through a separate authority gate.
    AcceptedElsewhere,
}

/// Authority relation declared by a representation kind.
///
/// This declaration never grants mutation authority. It only lets validators
/// reason about where an artifact must be routed if later execution wants to
/// publish it.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RepresentationAuthority {
    /// Derived value with no publication authority.
    Derived,
    /// Proposal for a specific authority class.
    ProposalTo(AuthorityClass),
    /// Candidate request to be routed through runtime execution.
    ExecutableRequest,
    /// Proposal for runtime-control authority.
    ControlProposal,
    /// Diagnostic-only artifact.
    Diagnostic,
    /// Oracle-sourced or oracle-only artifact.
    Oracle,
}

impl RepresentationAuthority {
    /// Returns whether this authority relation is oracle-sourced.
    #[must_use]
    pub const fn is_oracle(self) -> bool {
        matches!(self, Self::Oracle)
    }

    /// Returns whether this authority relation is proposal-shaped.
    #[must_use]
    pub const fn is_proposal(self) -> bool {
        matches!(self, Self::ProposalTo(_) | Self::ControlProposal)
    }
}

/// Concrete representation kind used by decision passes and profiles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepresentationKindDef {
    id: DefinitionId,
    name: DefinitionName,
    roles: BTreeSet<RepresentationRole>,
    visibility: RepresentationVisibility,
    persistence: RepresentationPersistence,
    authority: RepresentationAuthority,
    version: VersionAnchor,
}

impl RepresentationKindDef {
    /// Creates a checked representation-kind declaration with local invariants.
    pub fn new(
        id: DefinitionId,
        name: DefinitionName,
        roles: impl IntoIterator<Item = RepresentationRole>,
        visibility: RepresentationVisibility,
        persistence: RepresentationPersistence,
        authority: RepresentationAuthority,
        version: VersionAnchor,
    ) -> Result<Self, DecisionError> {
        let roles = roles.into_iter().collect::<BTreeSet<_>>();
        if roles.is_empty() {
            return Err(empty_definition_field(id, "RepresentationKindDef", "roles"));
        }
        validate_authority_role(id, &roles, authority)?;

        Ok(Self {
            id,
            name,
            roles,
            visibility,
            persistence,
            authority,
            version,
        })
    }

    /// Returns the representation kind id.
    #[must_use]
    pub const fn id(&self) -> DefinitionId {
        self.id
    }

    /// Returns the representation name.
    #[must_use]
    pub const fn name(&self) -> &DefinitionName {
        &self.name
    }

    /// Returns broad compatibility roles in deterministic order.
    pub fn roles(&self) -> impl Iterator<Item = RepresentationRole> + '_ {
        self.roles.iter().copied()
    }

    /// Returns whether this representation declares a role.
    #[must_use]
    pub fn has_role(&self, role: RepresentationRole) -> bool {
        self.roles.contains(&role)
    }

    /// Returns whether this representation may satisfy a pass input or output role.
    #[must_use]
    pub fn can_satisfy(&self, role: RepresentationRole) -> bool {
        if self.visibility.is_diagnostic_only()
            && matches!(
                role,
                RepresentationRole::ExecutableRequest | RepresentationRole::NonHardUpdateProposal
            )
        {
            return false;
        }

        self.has_role(role)
    }

    /// Returns visibility metadata.
    #[must_use]
    pub const fn visibility(&self) -> RepresentationVisibility {
        self.visibility
    }

    /// Returns persistence metadata.
    #[must_use]
    pub const fn persistence(&self) -> RepresentationPersistence {
        self.persistence
    }

    /// Returns authority-relation metadata.
    #[must_use]
    pub const fn authority(&self) -> RepresentationAuthority {
        self.authority
    }

    /// Returns whether this representation requires oracle-aware profile labeling.
    #[must_use]
    pub const fn is_oracle_only(&self) -> bool {
        self.visibility.is_oracle_only() || self.authority.is_oracle()
    }

    /// Returns the version anchor.
    #[must_use]
    pub const fn version(&self) -> VersionAnchor {
        self.version
    }
}

fn validate_authority_role(
    representation: DefinitionId,
    roles: &BTreeSet<RepresentationRole>,
    authority: RepresentationAuthority,
) -> Result<(), DecisionError> {
    match authority {
        RepresentationAuthority::ProposalTo(AuthorityClass::Hard) => {
            return Err(DecisionError::HardProposalAuthority { representation });
        }
        RepresentationAuthority::ProposalTo(_) | RepresentationAuthority::ControlProposal => {
            require_role(
                representation,
                authority,
                roles,
                RepresentationRole::NonHardUpdateProposal,
            )?;
        }
        RepresentationAuthority::ExecutableRequest => {
            require_role(
                representation,
                authority,
                roles,
                RepresentationRole::ExecutableRequest,
            )?;
        }
        RepresentationAuthority::Diagnostic => {
            require_role(
                representation,
                authority,
                roles,
                RepresentationRole::Diagnostic,
            )?;
        }
        RepresentationAuthority::Derived | RepresentationAuthority::Oracle => {}
    }

    Ok(())
}

fn require_role(
    representation: DefinitionId,
    authority: RepresentationAuthority,
    roles: &BTreeSet<RepresentationRole>,
    role: RepresentationRole,
) -> Result<(), DecisionError> {
    if roles.contains(&role) {
        Ok(())
    } else {
        Err(DecisionError::RepresentationAuthorityRoleMismatch {
            representation,
            authority,
            role,
        })
    }
}
