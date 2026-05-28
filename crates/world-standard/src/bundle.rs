use world_defs::{
    DefinitionBundle, DefinitionError, DefinitionRegistryBuilder, EffectPrimitiveDescriptor,
};

/// Pure installer for the standard primitive definition vocabulary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StandardWorldDefinitions;

impl DefinitionBundle for StandardWorldDefinitions {
    fn install_definitions(
        &self,
        builder: &mut DefinitionRegistryBuilder,
    ) -> Result<(), DefinitionError> {
        builder.add_primitive(crate::primitives::physical::CreateEntity.definition()?)?;
        builder.add_primitive(crate::primitives::physical::PlaceEntity.definition()?)?;
        builder.add_primitive(crate::primitives::reservation::AcquireReservation.definition()?)?;
        builder.add_primitive(crate::primitives::process::ScheduleProcess.definition()?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_core::ReplayLevel;
    use world_defs::{
        DefinitionRegistryBuilder, EffectParamKind, EffectPrimitiveDef, EventContract,
        StagePermission,
    };

    #[test]
    fn standard_definition_bundle_builds_a_valid_registry() {
        let mut builder = DefinitionRegistryBuilder::new();
        if let Err(error) = builder.install(&StandardWorldDefinitions) {
            panic!("standard definitions should install: {error}");
        }
        let Ok(registry) = builder.build() else {
            panic!("standard definitions should build");
        };

        assert!(
            registry
                .effect_primitive(crate::ids::create_entity())
                .is_some()
        );
        assert!(
            registry
                .effect_primitive(crate::ids::place_entity())
                .is_some()
        );
        assert!(
            registry
                .effect_primitive(crate::ids::acquire_reservation())
                .is_some()
        );
    }

    #[test]
    fn standard_primitive_descriptors_pin_schema() {
        assert_primitive(
            crate::primitives::physical::CreateEntity.definition(),
            crate::ids::create_entity(),
            "create_entity",
            &[(crate::ids::entity_param(), EffectParamKind::EntityRole)],
            &[
                StagePermission::MutatePhysical,
                StagePermission::EmitPhysicalEventRecord,
            ],
            EventContract::new([crate::events::entity_created()]),
            ReplayLevel::EventRebuild,
        );
        assert_primitive(
            crate::primitives::physical::PlaceEntity.definition(),
            crate::ids::place_entity(),
            "place_entity",
            &[
                (crate::ids::item_param(), EffectParamKind::EntityRole),
                (crate::ids::destination_param(), EffectParamKind::EntityRole),
            ],
            &[
                StagePermission::ReadWorld,
                StagePermission::MutatePhysical,
                StagePermission::EmitPhysicalEventRecord,
            ],
            EventContract::new([crate::events::entity_placed()]),
            ReplayLevel::EventRebuild,
        );
        assert_primitive(
            crate::primitives::reservation::AcquireReservation.definition(),
            crate::ids::acquire_reservation(),
            "acquire_reservation",
            &[
                (crate::ids::item_param(), EffectParamKind::EntityRole),
                (
                    crate::ids::holder_param(),
                    EffectParamKind::OptionalEntityRole,
                ),
            ],
            &[
                StagePermission::AcquireReservation,
                StagePermission::EmitPhysicalEventRecord,
            ],
            EventContract::new([crate::events::reservation_acquired()]),
            ReplayLevel::AuditOnly,
        );
        assert_primitive(
            crate::primitives::process::ScheduleProcess.definition(),
            crate::ids::schedule_process(),
            "schedule_process",
            &[],
            &[StagePermission::ScheduleProcess],
            EventContract::default(),
            ReplayLevel::AuditOnly,
        );
    }

    fn assert_primitive(
        definition: Result<EffectPrimitiveDef, DefinitionError>,
        id: world_defs::EffectPrimitiveId,
        name: &'static str,
        params: &[(world_defs::EffectParamName, EffectParamKind)],
        permissions: &[StagePermission],
        event_contract: EventContract,
        replay_level: ReplayLevel,
    ) {
        let Ok(definition) = definition else {
            panic!("standard primitive descriptor should materialize");
        };
        assert_eq!(definition.id(), id);
        assert_eq!(definition.name().as_ref(), name);
        assert_eq!(definition.params().len(), params.len());
        for (actual, (name, kind)) in definition.params().iter().zip(params) {
            assert_eq!(actual.name(), name);
            assert_eq!(actual.kind(), *kind);
        }
        assert_eq!(
            definition
                .required_permissions()
                .copied()
                .collect::<Vec<_>>(),
            permissions
        );
        assert_eq!(definition.event_contract(), &event_contract);
        assert_eq!(definition.replay_level(), replay_level);
        assert_eq!(definition.version(), crate::ids::primitive_version());
    }
}
