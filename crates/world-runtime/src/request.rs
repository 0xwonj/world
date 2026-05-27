use std::collections::{BTreeMap, BTreeSet};

use world_core::{CausalSource, DefinitionId, EntityId, ProvenanceKey, SimulationTime};
use world_defs::{ActionDef, RoleName};

use crate::outcome::{RejectedOutcome, RejectionReason};

/// Source that submitted runtime work to the causal runtime.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RequestSource {
    /// Player-facing command source.
    Player,
    /// Actor policy source.
    ActorPolicy,
    /// Engine-owned source.
    Engine,
    /// Tooling or test harness source.
    Tooling,
}

impl From<RequestSource> for CausalSource {
    fn from(source: RequestSource) -> Self {
        match source {
            RequestSource::Player => Self::Player,
            RequestSource::ActorPolicy => Self::ActorPolicy,
            RequestSource::Engine => Self::Engine,
            RequestSource::Tooling => Self::Tooling,
        }
    }
}

/// Submitted role binding for one runtime request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmittedRole {
    name: RoleName,
    entity: EntityId,
}

impl SubmittedRole {
    /// Creates a submitted role binding.
    pub fn new(name: RoleName, entity: EntityId) -> Self {
        Self { name, entity }
    }

    /// Returns the role name.
    pub fn name(&self) -> &RoleName {
        &self.name
    }

    /// Returns the bound entity.
    pub const fn entity(&self) -> EntityId {
        self.entity
    }
}

/// Request submitted to the causal runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeRequest {
    source: RequestSource,
    actor: Option<EntityId>,
    action: DefinitionId,
    submitted_at: SimulationTime,
    roles: Vec<SubmittedRole>,
    provenance: Option<ProvenanceKey>,
}

impl RuntimeRequest {
    /// Creates a runtime request for an action definition.
    pub fn new(
        source: RequestSource,
        actor: Option<EntityId>,
        action: DefinitionId,
        submitted_at: SimulationTime,
        roles: impl IntoIterator<Item = SubmittedRole>,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        Self {
            source,
            actor,
            action,
            submitted_at,
            roles: roles.into_iter().collect(),
            provenance,
        }
    }

    /// Returns the request source.
    pub const fn source(&self) -> RequestSource {
        self.source
    }

    /// Returns the actor entity, if the source supplied one.
    pub const fn actor(&self) -> Option<EntityId> {
        self.actor
    }

    /// Returns the requested action definition id.
    pub const fn action(&self) -> DefinitionId {
        self.action
    }

    /// Returns submitted simulation time.
    pub const fn submitted_at(&self) -> SimulationTime {
        self.submitted_at
    }

    /// Returns submitted role bindings.
    pub fn roles(&self) -> &[SubmittedRole] {
        &self.roles
    }

    /// Returns request provenance, if known.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }

    pub(crate) fn bind(self, action: &ActionDef) -> Result<BoundRuntimeRequest, RejectedOutcome> {
        let mut roles = BTreeMap::new();
        for submitted in self.roles {
            let role_name = submitted.name;
            if roles.insert(role_name.clone(), submitted.entity).is_some() {
                return Err(RejectedOutcome::new(
                    self.action,
                    RejectionReason::DuplicateRoleBinding { role: role_name },
                ));
            }
        }

        let declared = action
            .roles()
            .iter()
            .map(|role| role.name().clone())
            .collect::<BTreeSet<_>>();

        for role in roles.keys() {
            if !declared.contains(role) {
                return Err(RejectedOutcome::new(
                    self.action,
                    RejectionReason::UnknownRoleBinding { role: role.clone() },
                ));
            }
        }

        for role in action.roles() {
            if !roles.contains_key(role.name()) {
                return Err(RejectedOutcome::new(
                    self.action,
                    RejectionReason::MissingRoleBinding {
                        role: role.name().clone(),
                    },
                ));
            }
        }

        Ok(BoundRuntimeRequest {
            source: self.source,
            actor: self.actor,
            action: self.action,
            effect_program: action.effect_program(),
            submitted_at: self.submitted_at,
            roles,
            provenance: self.provenance,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoundRuntimeRequest {
    source: RequestSource,
    actor: Option<EntityId>,
    action: DefinitionId,
    effect_program: DefinitionId,
    submitted_at: SimulationTime,
    roles: BTreeMap<RoleName, EntityId>,
    provenance: Option<ProvenanceKey>,
}

impl BoundRuntimeRequest {
    pub(crate) const fn source(&self) -> RequestSource {
        self.source
    }

    pub(crate) const fn actor(&self) -> Option<EntityId> {
        self.actor
    }

    pub(crate) const fn action(&self) -> DefinitionId {
        self.action
    }

    pub(crate) const fn effect_program(&self) -> DefinitionId {
        self.effect_program
    }

    pub(crate) const fn submitted_at(&self) -> SimulationTime {
        self.submitted_at
    }

    pub(crate) const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }

    pub(crate) fn bound_role_entity(&self, role: &RoleName) -> Option<EntityId> {
        self.roles.get(role).copied()
    }
}
