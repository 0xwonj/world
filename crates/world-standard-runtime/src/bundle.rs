use world_runtime::{PrimitiveSemanticsInstaller, PrimitiveSemanticsRegistryBuilder, RuntimeError};

/// Trusted installer for standard primitive runtime semantics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StandardPrimitiveSemantics;

impl PrimitiveSemanticsInstaller for StandardPrimitiveSemantics {
    fn install_semantics(
        &self,
        builder: &mut PrimitiveSemanticsRegistryBuilder,
    ) -> Result<(), RuntimeError> {
        builder.add_handler(crate::physical::CreateEntitySemantics)?;
        builder.add_handler(crate::physical::PlaceEntitySemantics)?;
        builder.add_handler(crate::reservation::AcquireReservationSemantics)?;
        Ok(())
    }
}
