use world_core::ReplayLevel;
use world_defs::{
    EffectParamDef, EffectParamKind, EffectPrimitiveDescriptor, EffectPrimitiveId, EventContract,
    StagePermission,
};

use crate::{events, ids};

/// Descriptor for acquiring a runtime reservation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AcquireReservation;

impl EffectPrimitiveDescriptor for AcquireReservation {
    fn id(&self) -> EffectPrimitiveId {
        ids::acquire_reservation()
    }

    fn name(&self) -> world_defs::DefinitionName {
        ids::primitive_name("acquire_reservation")
    }

    fn params(&self) -> Vec<EffectParamDef> {
        vec![
            EffectParamDef::new(ids::item_param(), EffectParamKind::EntityRole),
            EffectParamDef::new(ids::holder_param(), EffectParamKind::OptionalEntityRole),
        ]
    }

    fn required_permissions(&self) -> Vec<StagePermission> {
        vec![
            StagePermission::AcquireReservation,
            StagePermission::EmitPhysicalEventRecord,
        ]
    }

    fn event_contract(&self) -> EventContract {
        EventContract::new([events::reservation_acquired()])
    }

    fn replay_level(&self) -> ReplayLevel {
        ReplayLevel::AuditOnly
    }

    fn version(&self) -> world_core::VersionAnchor {
        ids::primitive_version()
    }
}
