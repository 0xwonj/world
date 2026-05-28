use std::{collections::BTreeSet, fmt};

use world_core::{DefinitionId, ReplayLevel, VersionAnchor};

use crate::{
    error::{DefinitionError, empty_definition_field},
    events::EventContract,
    keys::{DefinitionName, EffectParamName},
};

use super::{
    EffectParamDef, StagePermission,
    permissions::{permissions_allow_event_emission, permissions_require_event},
};

/// Stable identity of a checked primitive effect definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EffectPrimitiveId(DefinitionId);

impl EffectPrimitiveId {
    /// Wraps a definition id as a primitive effect id.
    pub const fn new(id: DefinitionId) -> Self {
        Self(id)
    }

    /// Returns the underlying definition id.
    pub const fn definition(self) -> DefinitionId {
        self.0
    }
}

impl From<EffectPrimitiveId> for DefinitionId {
    fn from(value: EffectPrimitiveId) -> Self {
        value.definition()
    }
}

impl fmt::Display for EffectPrimitiveId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.definition().get())
    }
}

/// Checked primitive effect definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectPrimitiveDef {
    id: EffectPrimitiveId,
    name: DefinitionName,
    params: Vec<EffectParamDef>,
    required_permissions: BTreeSet<StagePermission>,
    event_contract: EventContract,
    replay_level: ReplayLevel,
    version: VersionAnchor,
}

impl EffectPrimitiveDef {
    /// Creates a primitive definition with local signature and capability checks.
    pub fn new(
        id: EffectPrimitiveId,
        name: DefinitionName,
        params: impl IntoIterator<Item = EffectParamDef>,
        required_permissions: impl IntoIterator<Item = StagePermission>,
        event_contract: EventContract,
        replay_level: ReplayLevel,
        version: VersionAnchor,
    ) -> Result<Self, DefinitionError> {
        let params = params.into_iter().collect::<Vec<_>>();
        validate_unique_params(id, &params)?;

        let required_permissions = required_permissions.into_iter().collect::<BTreeSet<_>>();
        if required_permissions.is_empty() {
            return Err(empty_definition_field(
                id.definition(),
                "EffectPrimitiveDef",
                "required_permissions",
            ));
        }
        if permissions_require_event(&required_permissions) && event_contract.is_empty() {
            return Err(DefinitionError::PrimitiveRequiresEvent { primitive: id });
        }
        if !event_contract.is_empty() && !permissions_allow_event_emission(&required_permissions) {
            return Err(DefinitionError::PrimitiveEventPermissionNotDeclared { primitive: id });
        }

        Ok(Self {
            id,
            name,
            params,
            required_permissions,
            event_contract,
            replay_level,
            version,
        })
    }

    /// Returns the primitive id.
    pub const fn id(&self) -> EffectPrimitiveId {
        self.id
    }

    /// Returns the primitive display name.
    pub fn name(&self) -> &DefinitionName {
        &self.name
    }

    /// Returns primitive parameters in declaration order.
    pub fn params(&self) -> &[EffectParamDef] {
        &self.params
    }

    /// Looks up a primitive parameter by name.
    pub fn param(&self, name: &EffectParamName) -> Option<&EffectParamDef> {
        self.params.iter().find(|param| param.name() == name)
    }

    /// Returns required staging permissions.
    pub fn required_permissions(&self) -> impl Iterator<Item = &StagePermission> {
        self.required_permissions.iter()
    }

    /// Returns true when the primitive requires the permission.
    pub fn requires_permission(&self, permission: StagePermission) -> bool {
        self.required_permissions.contains(&permission)
    }

    /// Returns the primitive event contract.
    pub fn event_contract(&self) -> &EventContract {
        &self.event_contract
    }

    /// Returns declared replay level.
    pub const fn replay_level(&self) -> ReplayLevel {
        self.replay_level
    }

    /// Returns primitive schema version.
    pub const fn version(&self) -> VersionAnchor {
        self.version
    }
}

/// Pure schema descriptor for one primitive effect definition.
///
/// Descriptors are compile-time vocabulary objects. They define the checked
/// primitive surface without carrying executable runtime semantics.
pub trait EffectPrimitiveDescriptor {
    /// Returns the stable primitive id.
    fn id(&self) -> EffectPrimitiveId;

    /// Returns the primitive display name.
    fn name(&self) -> DefinitionName;

    /// Returns primitive parameters in declaration order.
    fn params(&self) -> Vec<EffectParamDef>;

    /// Returns required staging permissions.
    fn required_permissions(&self) -> Vec<StagePermission>;

    /// Returns the primitive event contract.
    fn event_contract(&self) -> EventContract;

    /// Returns declared replay level.
    fn replay_level(&self) -> ReplayLevel;

    /// Returns primitive schema version.
    fn version(&self) -> VersionAnchor;

    /// Materializes a checked primitive definition from this pure descriptor.
    fn definition(&self) -> Result<EffectPrimitiveDef, DefinitionError> {
        EffectPrimitiveDef::new(
            self.id(),
            self.name(),
            self.params(),
            self.required_permissions(),
            self.event_contract(),
            self.replay_level(),
            self.version(),
        )
    }
}

fn validate_unique_params(
    primitive: EffectPrimitiveId,
    params: &[EffectParamDef],
) -> Result<(), DefinitionError> {
    let mut seen = BTreeSet::new();
    for param in params {
        if !seen.insert(param.name().clone()) {
            return Err(DefinitionError::DuplicateEffectParam {
                primitive,
                param: param.name().clone(),
            });
        }
    }

    Ok(())
}
