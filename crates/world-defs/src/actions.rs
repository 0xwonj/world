use std::collections::BTreeSet;

use world_core::{DefinitionId, VersionAnchor};

use crate::effects::StagePermission;
use crate::error::{DefinitionError, require_not_empty};
use crate::events::EventContract;
use crate::keys::DefinitionName;
use crate::roles::{
    BindingRuleDef, RequirementDef, RoleDef, declared_role_names, ensure_unique_roles,
    validate_role_refs,
};

/// Checked action definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionDef {
    id: DefinitionId,
    name: DefinitionName,
    roles: Vec<RoleDef>,
    requirements: Vec<RequirementDef>,
    binding_rules: Vec<BindingRuleDef>,
    effect_program: DefinitionId,
    event_contract: EventContract,
    stage_permissions: BTreeSet<StagePermission>,
    version: VersionAnchor,
}

impl ActionDef {
    /// Creates a checked action definition with local role and permission invariants.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: DefinitionId,
        name: DefinitionName,
        roles: impl IntoIterator<Item = RoleDef>,
        requirements: impl IntoIterator<Item = RequirementDef>,
        binding_rules: impl IntoIterator<Item = BindingRuleDef>,
        effect_program: DefinitionId,
        event_contract: EventContract,
        stage_permissions: impl IntoIterator<Item = StagePermission>,
        version: VersionAnchor,
    ) -> Result<Self, DefinitionError> {
        let roles = roles.into_iter().collect::<Vec<_>>();
        require_not_empty(id, "ActionDef", "roles", &roles)?;
        ensure_unique_roles(id, &roles)?;

        let role_names = declared_role_names(&roles);
        let requirements = requirements.into_iter().collect::<Vec<_>>();
        let binding_rules = binding_rules.into_iter().collect::<Vec<_>>();
        validate_role_refs(
            id,
            &role_names,
            requirements.iter().flat_map(RequirementDef::roles),
        )?;
        validate_role_refs(
            id,
            &role_names,
            binding_rules.iter().flat_map(BindingRuleDef::roles),
        )?;
        validate_role_refs(id, &role_names, event_contract.role_refs())?;

        let stage_permissions = stage_permissions.into_iter().collect::<BTreeSet<_>>();
        require_not_empty(id, "ActionDef", "stage_permissions", &stage_permissions)?;

        Ok(Self {
            id,
            name,
            roles,
            requirements,
            binding_rules,
            effect_program,
            event_contract,
            stage_permissions,
            version,
        })
    }

    /// Returns the definition id.
    pub fn id(&self) -> DefinitionId {
        self.id
    }

    /// Returns the definition name.
    pub fn name(&self) -> &DefinitionName {
        &self.name
    }

    /// Returns declared roles.
    pub fn roles(&self) -> &[RoleDef] {
        &self.roles
    }

    /// Returns declared requirements.
    pub fn requirements(&self) -> &[RequirementDef] {
        &self.requirements
    }

    /// Returns declared binding rules.
    pub fn binding_rules(&self) -> &[BindingRuleDef] {
        &self.binding_rules
    }

    /// Returns the referenced typed effect program id.
    pub fn effect_program(&self) -> DefinitionId {
        self.effect_program
    }

    /// Returns the event contract.
    pub fn event_contract(&self) -> &EventContract {
        &self.event_contract
    }

    /// Returns declared stage permissions.
    pub fn stage_permissions(&self) -> impl Iterator<Item = &StagePermission> {
        self.stage_permissions.iter()
    }

    /// Returns true when the action declares the permission.
    pub fn declares_permission(&self, permission: StagePermission) -> bool {
        self.stage_permissions.contains(&permission)
    }

    /// Returns the version anchor.
    pub fn version(&self) -> VersionAnchor {
        self.version
    }
}
