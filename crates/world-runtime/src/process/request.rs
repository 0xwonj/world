use world_core::{DefinitionId, EntityId, ProvenanceKey, SimulationTime};
use world_defs::ResolutionTier;
use world_model::{ProcessRoleBinding, ProcessWork};

use crate::WakeupScheduleKey;

/// Request to create a durable process instance and schedule its first wakeup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartProcessRequest {
    pub(super) definition: DefinitionId,
    pub(super) owner: Option<EntityId>,
    pub(super) roles: Vec<ProcessRoleBinding>,
    pub(super) resolution: ResolutionTier,
    pub(super) required_work: ProcessWork,
    pub(super) first_wakeup: WakeupScheduleKey,
    pub(super) submitted_at: SimulationTime,
    pub(super) provenance: Option<ProvenanceKey>,
}

impl StartProcessRequest {
    /// Creates a process start request.
    #[must_use]
    pub fn new(
        definition: DefinitionId,
        resolution: ResolutionTier,
        required_work: ProcessWork,
        first_wakeup: WakeupScheduleKey,
        submitted_at: SimulationTime,
    ) -> Self {
        Self {
            definition,
            owner: None,
            roles: Vec::new(),
            resolution,
            required_work,
            first_wakeup,
            submitted_at,
            provenance: None,
        }
    }

    /// Returns this request with an owning entity.
    #[must_use]
    pub const fn with_owner(mut self, owner: EntityId) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Returns this request with process role bindings.
    #[must_use]
    pub fn with_roles(mut self, roles: impl IntoIterator<Item = ProcessRoleBinding>) -> Self {
        self.roles = roles.into_iter().collect();
        self
    }

    /// Returns this request with provenance.
    #[must_use]
    pub const fn with_provenance(mut self, provenance: ProvenanceKey) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Returns the checked process definition id.
    pub const fn definition(&self) -> DefinitionId {
        self.definition
    }

    /// Returns the requested process resolution tier.
    pub const fn resolution(&self) -> ResolutionTier {
        self.resolution
    }
}
