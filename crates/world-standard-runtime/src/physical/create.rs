use world_defs::EffectPrimitiveId;
use world_model::HardStateChange;
use world_runtime::{
    PrimitiveInvocation, PrimitiveSemantics, PrimitiveSemanticsContract, PrimitiveStageContext,
    PrimitiveValidationContext, PrimitiveValidationFailure, RuntimeError,
};

use crate::events;

/// Trusted runtime semantics for the standard entity creation primitive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CreateEntitySemantics;

impl PrimitiveSemantics for CreateEntitySemantics {
    fn primitive(&self) -> EffectPrimitiveId {
        world_standard::ids::create_entity()
    }

    fn contract(&self) -> PrimitiveSemanticsContract {
        events::contract(&world_standard::primitives::physical::CreateEntity)
    }

    fn validate(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveValidationContext<'_>,
    ) -> Result<(), PrimitiveValidationFailure> {
        let role = invocation.required_role(&world_standard::ids::entity_param())?;
        let (role, entity) = context.required_role_entity(&role)?;
        if context.contains_entity(entity) {
            return Err(world_runtime::RejectedOutcome::new(
                context.action(),
                world_runtime::RejectionReason::EntityAlreadyPresent { role, entity },
            )
            .into());
        }

        context.insert_entity(entity);
        Ok(())
    }

    fn stage(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveStageContext<'_, '_, '_, '_>,
    ) -> Result<(), RuntimeError> {
        events::ensure_event_permission(invocation.primitive())?;
        let role = invocation.required_role(&world_standard::ids::entity_param())?;
        let (role, entity) = context.required_role_entity(&role)?;
        if context.contains_entity(entity) {
            return Err(RuntimeError::DuplicateVisibleEntity { role, entity });
        }

        context.stage_physical_change(
            invocation,
            HardStateChange::insert_entity(entity, None, context.provenance()),
        )?;
        context.emit_declared_events(invocation)
    }
}
