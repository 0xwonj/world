use world_core::ReplayLevel;
use world_defs::{
    EffectParamDef, EffectParamKind, EffectPrimitiveDescriptor, EffectPrimitiveId, EventContract,
    StagePermission,
};

use crate::{events, ids};

/// Descriptor for creating a hard entity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CreateEntity;

impl EffectPrimitiveDescriptor for CreateEntity {
    fn id(&self) -> EffectPrimitiveId {
        ids::create_entity()
    }

    fn name(&self) -> world_defs::DefinitionName {
        ids::primitive_name("create_entity")
    }

    fn params(&self) -> Vec<EffectParamDef> {
        vec![EffectParamDef::new(
            ids::entity_param(),
            EffectParamKind::EntityRole,
        )]
    }

    fn required_permissions(&self) -> Vec<StagePermission> {
        vec![
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ]
    }

    fn event_contract(&self) -> EventContract {
        EventContract::new([events::entity_created()])
    }

    fn replay_level(&self) -> ReplayLevel {
        ReplayLevel::EventRebuild
    }

    fn version(&self) -> world_core::VersionAnchor {
        ids::primitive_version()
    }
}

/// Descriptor for placing an entity into a hard containment relation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlaceEntity;

impl EffectPrimitiveDescriptor for PlaceEntity {
    fn id(&self) -> EffectPrimitiveId {
        ids::place_entity()
    }

    fn name(&self) -> world_defs::DefinitionName {
        ids::primitive_name("place_entity")
    }

    fn params(&self) -> Vec<EffectParamDef> {
        vec![
            EffectParamDef::new(ids::item_param(), EffectParamKind::EntityRole),
            EffectParamDef::new(ids::destination_param(), EffectParamKind::EntityRole),
        ]
    }

    fn required_permissions(&self) -> Vec<StagePermission> {
        vec![
            StagePermission::ReadWorld,
            StagePermission::MutatePhysical,
            StagePermission::EmitPhysicalEventRecord,
        ]
    }

    fn event_contract(&self) -> EventContract {
        EventContract::new([events::entity_placed()])
    }

    fn replay_level(&self) -> ReplayLevel {
        ReplayLevel::EventRebuild
    }

    fn version(&self) -> world_core::VersionAnchor {
        ids::primitive_version()
    }
}
