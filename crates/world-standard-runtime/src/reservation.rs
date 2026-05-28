use world_defs::EffectPrimitiveId;
use world_model::{ReservationHolder, ReservationTarget};
use world_runtime::{
    AcquireReservationRequest, PrimitiveInvocation, PrimitiveSemantics, PrimitiveSemanticsContract,
    PrimitiveStageContext, PrimitiveValidationContext, PrimitiveValidationFailure, RuntimeError,
};

use crate::events;

/// Trusted runtime semantics for the standard reservation acquisition primitive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AcquireReservationSemantics;

impl PrimitiveSemantics for AcquireReservationSemantics {
    fn primitive(&self) -> EffectPrimitiveId {
        world_standard::ids::acquire_reservation()
    }

    fn contract(&self) -> PrimitiveSemanticsContract {
        events::contract(&world_standard::primitives::reservation::AcquireReservation)
    }

    fn validate(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveValidationContext<'_>,
    ) -> Result<(), PrimitiveValidationFailure> {
        let item_role = invocation.required_role(&world_standard::ids::item_param())?;
        let (_, item) = context.required_role_entity(&item_role)?;
        let target = ReservationTarget::Entity(item);
        if context.contains_active_reservation(&target) {
            return Err(world_runtime::RejectedOutcome::new(
                context.action(),
                world_runtime::RejectionReason::ReservationAlreadyHeld { target },
            )
            .into());
        }

        context.insert_reservation_target(target);
        Ok(())
    }

    fn stage(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveStageContext<'_, '_, '_, '_>,
    ) -> Result<(), RuntimeError> {
        events::ensure_event_permission(invocation.primitive())?;
        let item_role = invocation.required_role(&world_standard::ids::item_param())?;
        let (_, item) = context.required_role_entity(&item_role)?;
        let holder = match invocation.optional_role(&world_standard::ids::holder_param())? {
            Some(role) => {
                let (_, entity) = context.required_role_entity(&role)?;
                ReservationHolder::Entity(entity)
            }
            None => ReservationHolder::Runtime,
        };

        context.stage_reservation_acquire(
            invocation,
            AcquireReservationRequest::new(
                holder,
                ReservationTarget::Entity(item),
                context.request_time(),
                context.provenance(),
            ),
        )?;
        context.emit_declared_events(invocation)
    }
}
