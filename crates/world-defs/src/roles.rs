use std::collections::BTreeSet;

use world_core::DefinitionId;

use crate::error::DefinitionError;
use crate::keys::{BindingRuleKind, RequirementKind, RoleName, RoleType};

/// Typed role declaration for checked definitions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleDef {
    name: RoleName,
    role_type: RoleType,
}

impl RoleDef {
    /// Creates a role declaration.
    pub fn new(name: RoleName, role_type: RoleType) -> Self {
        Self { name, role_type }
    }

    /// Returns the role name.
    pub fn name(&self) -> &RoleName {
        &self.name
    }

    /// Returns the checked role type.
    pub fn role_type(&self) -> &RoleType {
        &self.role_type
    }
}

/// Checked validation requirement declared by an action or process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequirementDef {
    kind: RequirementKind,
    roles: Vec<RoleName>,
}

impl RequirementDef {
    /// Creates a requirement declaration.
    pub fn new(kind: RequirementKind, roles: impl IntoIterator<Item = RoleName>) -> Self {
        Self {
            kind,
            roles: roles.into_iter().collect(),
        }
    }

    /// Returns the requirement kind.
    pub fn kind(&self) -> &RequirementKind {
        &self.kind
    }

    /// Returns the roles this requirement references.
    pub fn roles(&self) -> &[RoleName] {
        &self.roles
    }
}

/// Checked role-binding rule declared by an action or process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingRuleDef {
    kind: BindingRuleKind,
    roles: Vec<RoleName>,
}

impl BindingRuleDef {
    /// Creates a binding rule declaration.
    pub fn new(kind: BindingRuleKind, roles: impl IntoIterator<Item = RoleName>) -> Self {
        Self {
            kind,
            roles: roles.into_iter().collect(),
        }
    }

    /// Returns the binding rule kind.
    pub fn kind(&self) -> &BindingRuleKind {
        &self.kind
    }

    /// Returns the roles this binding rule references.
    pub fn roles(&self) -> &[RoleName] {
        &self.roles
    }
}

pub(crate) fn ensure_unique_roles(
    definition: DefinitionId,
    roles: &[RoleDef],
) -> Result<(), DefinitionError> {
    let mut names = BTreeSet::new();

    for role in roles {
        if !names.insert(role.name().clone()) {
            return Err(DefinitionError::DuplicateRole {
                definition,
                role: role.name().clone(),
            });
        }
    }

    Ok(())
}

pub(crate) fn declared_role_names(roles: &[RoleDef]) -> BTreeSet<RoleName> {
    roles.iter().map(|role| role.name().clone()).collect()
}

pub(crate) fn validate_role_refs<'a>(
    definition: DefinitionId,
    declared_roles: &BTreeSet<RoleName>,
    roles: impl Iterator<Item = &'a RoleName>,
) -> Result<(), DefinitionError> {
    for role in roles {
        if !declared_roles.contains(role) {
            return Err(DefinitionError::UnknownRole {
                definition,
                role: role.clone(),
            });
        }
    }

    Ok(())
}
