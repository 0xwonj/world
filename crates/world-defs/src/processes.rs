use std::collections::{BTreeMap, BTreeSet};

use world_core::{DefinitionId, VersionAnchor};

use crate::effects::StagePermission;
use crate::error::{DefinitionError, empty_definition_field};
use crate::events::EventContract;
use crate::keys::{DefinitionName, PolicyKey, StateFieldName, StateValueType};
use crate::roles::{RoleDef, declared_role_names, ensure_unique_roles, validate_role_refs};

/// Resolution tier a process definition can support.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolutionTier {
    /// Local, concrete execution.
    Concrete,
    /// Abstract progress execution.
    Abstract,
    /// Strategic large-scale execution.
    Strategic,
}

/// Checked process support for one resolution tier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionSupport {
    tier: ResolutionTier,
    lowering_policy: PolicyKey,
    effect_programs: BTreeSet<DefinitionId>,
}

impl ResolutionSupport {
    /// Creates a resolution support declaration with the effect programs legal at this tier.
    pub fn new(
        tier: ResolutionTier,
        lowering_policy: PolicyKey,
        effect_programs: impl IntoIterator<Item = DefinitionId>,
    ) -> Result<Self, DefinitionError> {
        let effect_programs = effect_programs.into_iter().collect::<BTreeSet<_>>();
        if effect_programs.is_empty() {
            return Err(DefinitionError::EmptyItemField {
                type_name: "ResolutionSupport",
                field: "effect_programs",
            });
        }

        Ok(Self {
            tier,
            lowering_policy,
            effect_programs,
        })
    }

    /// Returns the supported resolution tier.
    pub fn tier(&self) -> ResolutionTier {
        self.tier
    }

    /// Returns the lowering policy for this resolution tier.
    pub fn lowering_policy(&self) -> &PolicyKey {
        &self.lowering_policy
    }

    /// Returns effect programs legal for this process resolution tier.
    ///
    /// Process programs are checked as definitions here; executable process-program
    /// semantics are supplied by a later runtime path, not by action primitive handlers.
    pub fn effect_programs(&self) -> impl Iterator<Item = &DefinitionId> {
        self.effect_programs.iter()
    }
}

/// Checked durable process state field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessStateField {
    name: StateFieldName,
    value_type: StateValueType,
}

impl ProcessStateField {
    /// Creates a process state field declaration.
    pub fn new(name: StateFieldName, value_type: StateValueType) -> Self {
        Self { name, value_type }
    }

    /// Returns the field name.
    pub fn name(&self) -> &StateFieldName {
        &self.name
    }

    /// Returns the checked field value type.
    pub fn value_type(&self) -> &StateValueType {
        &self.value_type
    }
}

/// Checked durable process state schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessStateSchema {
    fields: Vec<ProcessStateField>,
}

impl ProcessStateSchema {
    /// Creates a state schema when declared fields have unique names.
    pub fn new(
        fields: impl IntoIterator<Item = ProcessStateField>,
    ) -> Result<Self, DefinitionError> {
        let fields = fields.into_iter().collect::<Vec<_>>();
        let mut names = BTreeSet::new();
        for field in &fields {
            if !names.insert(field.name().clone()) {
                return Err(DefinitionError::DuplicateStateField {
                    field: field.name().clone(),
                });
            }
        }

        Ok(Self { fields })
    }

    /// Returns checked state fields.
    pub fn fields(&self) -> &[ProcessStateField] {
        &self.fields
    }
}

/// Checked process policy keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessPolicies {
    tick: PolicyKey,
    wait: PolicyKey,
    interrupt: PolicyKey,
    resume: PolicyKey,
    failure: PolicyKey,
}

impl ProcessPolicies {
    /// Creates process lifecycle and resolution policy keys.
    pub fn new(
        tick: PolicyKey,
        wait: PolicyKey,
        interrupt: PolicyKey,
        resume: PolicyKey,
        failure: PolicyKey,
    ) -> Self {
        Self {
            tick,
            wait,
            interrupt,
            resume,
            failure,
        }
    }

    /// Returns the tick policy key.
    pub fn tick(&self) -> &PolicyKey {
        &self.tick
    }

    /// Returns the wait policy key.
    pub fn wait(&self) -> &PolicyKey {
        &self.wait
    }

    /// Returns the interrupt policy key.
    pub fn interrupt(&self) -> &PolicyKey {
        &self.interrupt
    }

    /// Returns the resume policy key.
    pub fn resume(&self) -> &PolicyKey {
        &self.resume
    }

    /// Returns the failure policy key.
    pub fn failure(&self) -> &PolicyKey {
        &self.failure
    }
}

/// Checked durable process definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessDef {
    id: DefinitionId,
    name: DefinitionName,
    roles: Vec<RoleDef>,
    state_schema: ProcessStateSchema,
    resolution_support: BTreeMap<ResolutionTier, ResolutionSupport>,
    policies: ProcessPolicies,
    event_contract: EventContract,
    stage_permissions: BTreeSet<StagePermission>,
    version: VersionAnchor,
}

impl ProcessDef {
    /// Creates a checked process definition.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: DefinitionId,
        name: DefinitionName,
        roles: impl IntoIterator<Item = RoleDef>,
        state_schema: ProcessStateSchema,
        resolution_support: impl IntoIterator<Item = ResolutionSupport>,
        policies: ProcessPolicies,
        event_contract: EventContract,
        stage_permissions: impl IntoIterator<Item = StagePermission>,
        version: VersionAnchor,
    ) -> Result<Self, DefinitionError> {
        let roles = roles.into_iter().collect::<Vec<_>>();
        if roles.is_empty() {
            return Err(empty_definition_field(id, "ProcessDef", "roles"));
        }
        ensure_unique_roles(id, &roles)?;
        validate_role_refs(id, &declared_role_names(&roles), event_contract.role_refs())?;

        let resolution_support = collect_resolution_support(id, resolution_support)?;
        if resolution_support.is_empty() {
            return Err(empty_definition_field(
                id,
                "ProcessDef",
                "resolution_support",
            ));
        }

        let stage_permissions = stage_permissions.into_iter().collect::<BTreeSet<_>>();
        if stage_permissions.is_empty() {
            return Err(empty_definition_field(
                id,
                "ProcessDef",
                "stage_permissions",
            ));
        }

        Ok(Self {
            id,
            name,
            roles,
            state_schema,
            resolution_support,
            policies,
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

    /// Returns the durable process state schema.
    pub fn state_schema(&self) -> &ProcessStateSchema {
        &self.state_schema
    }

    /// Returns true when this process can execute at the resolution tier.
    pub fn supports_resolution(&self, resolution: ResolutionTier) -> bool {
        self.resolution_support.contains_key(&resolution)
    }

    /// Returns supported resolution tiers.
    pub fn supported_resolutions(&self) -> impl Iterator<Item = &ResolutionTier> {
        self.resolution_support.keys()
    }

    /// Returns the lowering policy for a supported resolution tier.
    pub fn resolution_policy(&self, resolution: ResolutionTier) -> Option<&PolicyKey> {
        self.resolution_support
            .get(&resolution)
            .map(ResolutionSupport::lowering_policy)
    }

    /// Returns checked resolution support for a supported tier.
    pub fn resolution_support(&self, resolution: ResolutionTier) -> Option<&ResolutionSupport> {
        self.resolution_support.get(&resolution)
    }

    /// Returns checked process policies.
    pub fn policies(&self) -> &ProcessPolicies {
        &self.policies
    }

    /// Returns all referenced effect program ids across supported resolution tiers.
    pub fn effect_programs(&self) -> impl Iterator<Item = &DefinitionId> {
        self.resolution_support
            .values()
            .flat_map(ResolutionSupport::effect_programs)
    }

    /// Returns the event contract.
    pub fn event_contract(&self) -> &EventContract {
        &self.event_contract
    }

    /// Returns declared stage permissions.
    pub fn stage_permissions(&self) -> impl Iterator<Item = &StagePermission> {
        self.stage_permissions.iter()
    }

    /// Returns true when the process declares the permission.
    pub fn declares_permission(&self, permission: StagePermission) -> bool {
        self.stage_permissions.contains(&permission)
    }

    /// Returns the version anchor.
    pub fn version(&self) -> VersionAnchor {
        self.version
    }
}

fn collect_resolution_support(
    definition: DefinitionId,
    items: impl IntoIterator<Item = ResolutionSupport>,
) -> Result<BTreeMap<ResolutionTier, ResolutionSupport>, DefinitionError> {
    let mut out = BTreeMap::new();

    for item in items {
        let tier = item.tier();
        if out.insert(tier, item).is_some() {
            return Err(DefinitionError::DuplicateResolutionSupport {
                definition,
                resolution: tier,
            });
        }
    }

    Ok(out)
}
