use world_core::DefinitionId;
use world_defs::{ActionDef, DefinitionName, DefinitionRegistry, RoleName, RoleType};

use crate::{
    ContextProvenanceSource, ContextReadDependency, context::ContextProjectionReportBuilder,
};

/// Schema-level actions an actor can consider before target binding and runtime validation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionRepertoire {
    entries: Vec<ActionRepertoireEntry>,
}

impl ActionRepertoire {
    /// Creates a repertoire from deterministic entries.
    #[must_use]
    pub(crate) fn new(entries: Vec<ActionRepertoireEntry>) -> Self {
        Self { entries }
    }

    /// Returns whether no actor-facing action schemas were projected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns repertoire entries in definition order.
    #[must_use]
    pub fn entries(&self) -> &[ActionRepertoireEntry] {
        &self.entries
    }
}

/// One actor-facing action schema candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionRepertoireEntry {
    action: DefinitionId,
    name: DefinitionName,
    actor_role: RoleName,
    roles: Vec<RoleProjection>,
    effect_program: DefinitionId,
    status: RepertoireStatus,
}

impl ActionRepertoireEntry {
    fn from_action(action: &ActionDef, actor_role: RoleName) -> Self {
        let roles = action
            .roles()
            .iter()
            .map(|role| RoleProjection::new(role.name().clone(), role.role_type().clone()))
            .collect();

        Self {
            action: action.id(),
            name: action.name().clone(),
            actor_role,
            roles,
            effect_program: action.effect_program(),
            status: RepertoireStatus::ActorFacingSchema,
        }
    }

    /// Returns the action definition id.
    #[must_use]
    pub const fn action(&self) -> DefinitionId {
        self.action
    }

    /// Returns the action definition name.
    #[must_use]
    pub const fn name(&self) -> &DefinitionName {
        &self.name
    }

    /// Returns the declared role that carries actor identity.
    #[must_use]
    pub const fn actor_role(&self) -> &RoleName {
        &self.actor_role
    }

    /// Returns declared action roles.
    #[must_use]
    pub fn roles(&self) -> &[RoleProjection] {
        &self.roles
    }

    /// Returns the effect program used by this action schema.
    #[must_use]
    pub const fn effect_program(&self) -> DefinitionId {
        self.effect_program
    }

    /// Returns schema-level availability status.
    #[must_use]
    pub const fn status(&self) -> RepertoireStatus {
        self.status
    }
}

/// Role metadata projected into action repertoire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleProjection {
    name: RoleName,
    role_type: RoleType,
}

impl RoleProjection {
    /// Creates a role projection.
    #[must_use]
    pub fn new(name: RoleName, role_type: RoleType) -> Self {
        Self { name, role_type }
    }

    /// Returns the role name.
    #[must_use]
    pub const fn name(&self) -> &RoleName {
        &self.name
    }

    /// Returns the checked role type.
    #[must_use]
    pub const fn role_type(&self) -> &RoleType {
        &self.role_type
    }
}

/// Schema-level availability status for an action repertoire entry.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RepertoireStatus {
    /// Actor role metadata makes this a definition-level actor-facing schema candidate.
    ActorFacingSchema,
}

pub(crate) fn derive(
    definitions: &DefinitionRegistry,
    report: &mut ContextProjectionReportBuilder,
) -> ActionRepertoire {
    report.push_status(
        crate::ContextProjectionKind::Repertoire,
        crate::ContextProjectionCompleteness::Shallow,
    );

    let entries = definitions
        .actions()
        .filter_map(|action| {
            let actor_role = action.actor_role()?.clone();
            let definition = action.id();
            report.insert_read(ContextReadDependency::Definition(definition));
            report.insert_read(ContextReadDependency::Definition(action.effect_program()));
            report.insert_provenance(ContextProvenanceSource::Definition(definition));
            report.insert_provenance(ContextProvenanceSource::Definition(action.effect_program()));
            Some(ActionRepertoireEntry::from_action(action, actor_role))
        })
        .collect();

    ActionRepertoire::new(entries)
}
