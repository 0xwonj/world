use std::collections::BTreeSet;

use world_core::{DefinitionId, ReplayLevel, VersionAnchor};

use crate::error::DefinitionError;
use crate::events::{EventContract, EventRecordSpec};
use crate::keys::{DefinitionName, EffectKind};

/// Permission a typed effect operation needs from the causal runtime stage.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StagePermission {
    /// Read committed world state.
    ReadWorld,
    /// Read actor-owned or holder-relative state.
    ReadActorOwnedState,
    /// Read a derived engine view.
    ReadDerivedEngineView,
    /// Read a submitted role binding.
    ReadSubmittedBinding,
    /// Run validation without staging mutation.
    Validate,
    /// Acquire a runtime reservation.
    AcquireReservation,
    /// Release a runtime reservation.
    ReleaseReservation,
    /// Draw from an engine-owned random stream.
    Rng,
    /// Stage a hard physical mutation.
    MutatePhysical,
    /// Stage process progress or process lifecycle mutation.
    MutateProcess,
    /// Emit a hard physical event record.
    EmitPhysicalEventRecord,
    /// Emit a sensory event record.
    EmitSensoryEventRecord,
    /// Schedule durable process work.
    ScheduleProcess,
    /// Schedule a reaction request.
    ScheduleReaction,
}

/// One checked primitive operation in a typed effect program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectOp {
    kind: EffectKind,
    permissions: BTreeSet<StagePermission>,
    emitted_events: BTreeSet<EventRecordSpec>,
}

impl EffectOp {
    /// Creates an operation when it declares at least one stage permission.
    pub fn new(
        kind: EffectKind,
        permissions: impl IntoIterator<Item = StagePermission>,
        emitted_events: impl IntoIterator<Item = EventRecordSpec>,
    ) -> Result<Self, DefinitionError> {
        let permissions = permissions.into_iter().collect::<BTreeSet<_>>();
        let emitted_events = emitted_events.into_iter().collect::<BTreeSet<_>>();

        if permissions.is_empty() {
            return Err(DefinitionError::EmptyItemField {
                type_name: "EffectOp",
                field: "permissions",
            });
        }

        if permissions_require_event(&permissions) && emitted_events.is_empty() {
            return Err(DefinitionError::OperationRequiresEvent { operation: kind });
        }

        if !emitted_events.is_empty() && !permissions_allow_event_emission(&permissions) {
            return Err(DefinitionError::EventPermissionNotDeclared { operation: kind });
        }

        Ok(Self {
            kind,
            permissions,
            emitted_events,
        })
    }

    /// Returns whether this operation needs an emitted event contract.
    pub fn requires_event(&self) -> bool {
        permissions_require_event(&self.permissions)
    }

    /// Returns whether this operation can emit no events.
    pub fn emits_no_events(&self) -> bool {
        self.emitted_events.is_empty()
    }

    /// Returns true when this operation emits the event spec.
    pub fn emits_event(&self, event: &EventRecordSpec) -> bool {
        self.emitted_events.contains(event)
    }

    /// Returns true when this operation requires the permission.
    pub fn requires_permission(&self, permission: StagePermission) -> bool {
        self.permissions.contains(&permission)
    }

    /// Returns the operation kind.
    pub fn kind(&self) -> &EffectKind {
        &self.kind
    }

    /// Returns permissions required by this operation.
    pub fn permissions(&self) -> impl Iterator<Item = &StagePermission> {
        self.permissions.iter()
    }

    /// Returns event specs this operation can emit.
    pub fn emitted_events(&self) -> impl Iterator<Item = &EventRecordSpec> {
        self.emitted_events.iter()
    }
}

/// Checked typed effect program definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectProgramDef {
    id: DefinitionId,
    name: DefinitionName,
    operations: Vec<EffectOp>,
    event_contract: EventContract,
    replay_level: ReplayLevel,
    version: VersionAnchor,
}

impl EffectProgramDef {
    /// Creates a checked effect program definition.
    pub fn new(
        id: DefinitionId,
        name: DefinitionName,
        operations: impl IntoIterator<Item = EffectOp>,
        event_contract: EventContract,
        replay_level: ReplayLevel,
        version: VersionAnchor,
    ) -> Result<Self, DefinitionError> {
        let operations = operations.into_iter().collect::<Vec<_>>();
        if operations.is_empty() {
            return Err(DefinitionError::EmptyDefinitionField {
                definition: id,
                type_name: "EffectProgramDef",
                field: "operations",
            });
        }

        let emitted_events = collect_emitted_events(&operations);
        for event in event_contract.required_events() {
            if !emitted_events.contains(event) {
                return Err(DefinitionError::RequiredEventNotEmitted {
                    definition: id,
                    event: event.clone(),
                });
            }
        }
        for event in operations.iter().flat_map(EffectOp::emitted_events) {
            if !event_contract.permits_event(event) {
                return Err(DefinitionError::EventNotPermittedByContract {
                    definition: id,
                    event: event.clone(),
                });
            }
        }

        Ok(Self {
            id,
            name,
            operations,
            event_contract,
            replay_level,
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

    /// Returns checked operations.
    pub fn operations(&self) -> &[EffectOp] {
        &self.operations
    }

    /// Returns the event contract.
    pub fn event_contract(&self) -> &EventContract {
        &self.event_contract
    }

    /// Returns declared replay level.
    pub fn replay_level(&self) -> ReplayLevel {
        self.replay_level
    }

    /// Returns the version anchor.
    pub fn version(&self) -> VersionAnchor {
        self.version
    }

    /// Returns the union of permissions required by all operations.
    pub fn required_permissions(&self) -> BTreeSet<StagePermission> {
        self.operations
            .iter()
            .flat_map(|operation| operation.permissions().copied())
            .collect()
    }

    /// Returns the union of event specs that operations can emit.
    pub fn emitted_events(&self) -> BTreeSet<EventRecordSpec> {
        collect_emitted_events(&self.operations)
    }
}

fn collect_emitted_events(operations: &[EffectOp]) -> BTreeSet<EventRecordSpec> {
    operations
        .iter()
        .flat_map(EffectOp::emitted_events)
        .cloned()
        .collect()
}

fn permissions_require_event(permissions: &BTreeSet<StagePermission>) -> bool {
    permissions.iter().any(|permission| {
        matches!(
            permission,
            StagePermission::MutatePhysical
                | StagePermission::EmitPhysicalEventRecord
                | StagePermission::EmitSensoryEventRecord
        )
    })
}

fn permissions_allow_event_emission(permissions: &BTreeSet<StagePermission>) -> bool {
    permissions.iter().any(|permission| {
        matches!(
            permission,
            StagePermission::EmitPhysicalEventRecord | StagePermission::EmitSensoryEventRecord
        )
    })
}
