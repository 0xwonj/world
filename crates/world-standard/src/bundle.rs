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
