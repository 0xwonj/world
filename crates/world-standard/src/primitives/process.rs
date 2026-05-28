use world_core::ReplayLevel;
use world_defs::{
    EffectParamDef, EffectPrimitiveDescriptor, EffectPrimitiveId, EventContract, StagePermission,
};

use crate::ids;

/// Descriptor for a definition-only process primitive that schedules durable work.
///
/// This primitive is checked by process definitions but is not executed by the
/// action primitive interpreter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScheduleProcess;

impl EffectPrimitiveDescriptor for ScheduleProcess {
    fn id(&self) -> EffectPrimitiveId {
        ids::schedule_process()
    }

    fn name(&self) -> world_defs::DefinitionName {
        ids::primitive_name("schedule_process")
    }

    fn params(&self) -> Vec<EffectParamDef> {
        Vec::new()
    }

    fn required_permissions(&self) -> Vec<StagePermission> {
        vec![StagePermission::ScheduleProcess]
    }

    fn event_contract(&self) -> EventContract {
        EventContract::default()
    }

    fn replay_level(&self) -> ReplayLevel {
        ReplayLevel::AuditOnly
    }

    fn version(&self) -> world_core::VersionAnchor {
        ids::primitive_version()
    }
}
