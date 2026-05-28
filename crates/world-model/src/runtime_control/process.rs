use std::collections::BTreeMap;

use world_core::{
    DefinitionId, EntityId, ProcessInstanceId, ProvenanceKey, ReservationId, VersionAnchor,
};
use world_defs::{ResolutionTier, RoleName, StateFieldName};

use crate::ModelError;

use super::lifecycle::ProcessLifecycle;

/// Entity bound to one process role.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessRoleBinding {
    role: RoleName,
    entity: EntityId,
}

impl ProcessRoleBinding {
    /// Creates a process role binding.
    #[must_use]
    pub fn new(role: RoleName, entity: EntityId) -> Self {
        Self { role, entity }
    }

    /// Returns the role name.
    pub fn role(&self) -> &RoleName {
        &self.role
    }

    /// Returns the entity bound to the role.
    pub const fn entity(&self) -> EntityId {
        self.entity
    }
}

/// Small closed process state value set used before a process policy language exists.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProcessStateValue {
    /// Unsigned integer state.
    Unsigned(u64),
    /// Boolean state.
    Flag(bool),
    /// Opaque checked label.
    Label(String),
}

/// Durable process state snapshot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProcessStateSnapshot {
    fields: BTreeMap<StateFieldName, ProcessStateValue>,
}

impl ProcessStateSnapshot {
    /// Creates a process state snapshot.
    #[must_use]
    pub fn new(fields: impl IntoIterator<Item = (StateFieldName, ProcessStateValue)>) -> Self {
        Self {
            fields: fields.into_iter().collect(),
        }
    }

    /// Returns state fields in key order.
    pub fn fields(&self) -> impl Iterator<Item = (&StateFieldName, &ProcessStateValue)> {
        self.fields.iter()
    }
}

/// Process work unit, distinct from simulation time ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessWork {
    units: u64,
}

impl ProcessWork {
    /// Creates a process work value.
    #[must_use]
    pub const fn from_units(units: u64) -> Self {
        Self { units }
    }

    /// Returns raw work units.
    #[must_use]
    pub const fn units(self) -> u64 {
        self.units
    }

    /// Adds work units, returning `None` on overflow.
    #[must_use]
    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.units.checked_add(other.units) {
            Some(units) => Some(Self { units }),
            None => None,
        }
    }
}

/// Durable process progress.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessProgress {
    /// Process has a known completion threshold.
    Bounded {
        completed: ProcessWork,
        required: ProcessWork,
    },
    /// Process records progress but has no fixed completion threshold.
    OpenEnded { completed: ProcessWork },
}

impl ProcessProgress {
    /// Returns completed work units.
    #[must_use]
    pub const fn completed(&self) -> ProcessWork {
        match self {
            Self::Bounded { completed, .. } | Self::OpenEnded { completed } => *completed,
        }
    }

    /// Returns whether bounded work has reached the required threshold.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        match self {
            Self::Bounded {
                completed,
                required,
            } => completed.units() >= required.units(),
            Self::OpenEnded { .. } => false,
        }
    }

    /// Returns progress after adding work units.
    pub fn advance(self, amount: ProcessWork) -> Result<Self, ModelError> {
        let completed = self
            .completed()
            .checked_add(amount)
            .ok_or(ModelError::RuntimeControlValueOverflow)?;
        Ok(match self {
            Self::Bounded { required, .. } => Self::Bounded {
                completed,
                required,
            },
            Self::OpenEnded { .. } => Self::OpenEnded { completed },
        })
    }
}

/// Named initialization data for a durable process instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessInstanceInit {
    id: ProcessInstanceId,
    definition: DefinitionId,
    owner: Option<EntityId>,
    roles: Vec<ProcessRoleBinding>,
    resolution: ResolutionTier,
    lifecycle: ProcessLifecycle,
    progress: ProcessProgress,
    state: ProcessStateSnapshot,
    reservations: Vec<ReservationId>,
    version: VersionAnchor,
    provenance: Option<ProvenanceKey>,
}

impl ProcessInstanceInit {
    /// Creates process instance initialization data with empty optional state.
    #[must_use]
    pub fn new(
        id: ProcessInstanceId,
        definition: DefinitionId,
        resolution: ResolutionTier,
        lifecycle: ProcessLifecycle,
        progress: ProcessProgress,
        version: VersionAnchor,
    ) -> Self {
        Self {
            id,
            definition,
            owner: None,
            roles: Vec::new(),
            resolution,
            lifecycle,
            progress,
            state: ProcessStateSnapshot::default(),
            reservations: Vec::new(),
            version,
            provenance: None,
        }
    }

    /// Returns this initialization data with an owner.
    #[must_use]
    pub fn with_owner(mut self, owner: EntityId) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Returns this initialization data with role bindings.
    #[must_use]
    pub fn with_roles(mut self, roles: impl IntoIterator<Item = ProcessRoleBinding>) -> Self {
        self.roles = roles.into_iter().collect();
        self
    }

    /// Returns this initialization data with a state snapshot.
    #[must_use]
    pub fn with_state(mut self, state: ProcessStateSnapshot) -> Self {
        self.state = state;
        self
    }

    /// Returns this initialization data with reservation references.
    #[must_use]
    pub fn with_reservations(
        mut self,
        reservations: impl IntoIterator<Item = ReservationId>,
    ) -> Self {
        self.reservations = reservations.into_iter().collect();
        self
    }

    /// Returns this initialization data with provenance.
    #[must_use]
    pub fn with_provenance(mut self, provenance: ProvenanceKey) -> Self {
        self.provenance = Some(provenance);
        self
    }
}

/// Durable process instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessInstanceRecord {
    id: ProcessInstanceId,
    definition: DefinitionId,
    owner: Option<EntityId>,
    roles: Vec<ProcessRoleBinding>,
    resolution: ResolutionTier,
    lifecycle: ProcessLifecycle,
    progress: ProcessProgress,
    state: ProcessStateSnapshot,
    reservations: Vec<ReservationId>,
    version: VersionAnchor,
    provenance: Option<ProvenanceKey>,
}

impl ProcessInstanceRecord {
    /// Creates a durable process instance record from named initialization data.
    #[must_use]
    pub fn new(init: ProcessInstanceInit) -> Self {
        init.into()
    }

    /// Returns the process id.
    pub const fn id(&self) -> ProcessInstanceId {
        self.id
    }

    /// Returns the checked process definition id.
    pub const fn definition(&self) -> DefinitionId {
        self.definition
    }

    /// Returns the owner entity, if any.
    pub const fn owner(&self) -> Option<EntityId> {
        self.owner
    }

    /// Returns role bindings.
    pub fn roles(&self) -> &[ProcessRoleBinding] {
        &self.roles
    }

    /// Returns the active resolution tier.
    pub const fn resolution(&self) -> ResolutionTier {
        self.resolution
    }

    /// Returns lifecycle state.
    pub const fn lifecycle(&self) -> &ProcessLifecycle {
        &self.lifecycle
    }

    /// Returns progress.
    pub const fn progress(&self) -> &ProcessProgress {
        &self.progress
    }

    /// Returns state snapshot.
    pub const fn state(&self) -> &ProcessStateSnapshot {
        &self.state
    }

    /// Returns held or needed reservation ids.
    pub fn reservations(&self) -> &[ReservationId] {
        &self.reservations
    }

    /// Returns the process definition version anchor.
    pub const fn version(&self) -> VersionAnchor {
        self.version
    }

    /// Returns process provenance, if known.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }

    /// Returns this process with a new lifecycle.
    #[must_use]
    pub fn with_lifecycle(mut self, lifecycle: ProcessLifecycle) -> Self {
        self.lifecycle = lifecycle;
        self
    }

    /// Returns this process with new progress.
    #[must_use]
    pub fn with_progress(mut self, progress: ProcessProgress) -> Self {
        self.progress = progress;
        self
    }
}

impl From<ProcessInstanceInit> for ProcessInstanceRecord {
    fn from(init: ProcessInstanceInit) -> Self {
        Self {
            id: init.id,
            definition: init.definition,
            owner: init.owner,
            roles: init.roles,
            resolution: init.resolution,
            lifecycle: init.lifecycle,
            progress: init.progress,
            state: init.state,
            reservations: init.reservations,
            version: init.version,
            provenance: init.provenance,
        }
    }
}
