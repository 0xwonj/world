use world_defs::{EffectPrimitiveDef, EffectPrimitiveDescriptor, StagePermission};
use world_runtime::PrimitiveSemanticsContract;

pub(crate) fn contract(descriptor: &impl EffectPrimitiveDescriptor) -> PrimitiveSemanticsContract {
    PrimitiveSemanticsContract::from_descriptor(descriptor)
}

pub(crate) fn ensure_event_permission(
    definition: &EffectPrimitiveDef,
) -> Result<(), world_runtime::RuntimeError> {
    if definition.requires_permission(StagePermission::EmitPhysicalEventRecord)
        || definition.requires_permission(StagePermission::EmitSensoryEventRecord)
    {
        Ok(())
    } else {
        Err(world_runtime::RuntimeError::PermissionNotDeclared {
            primitive: definition.id(),
            permission: StagePermission::EmitPhysicalEventRecord,
        })
    }
}
