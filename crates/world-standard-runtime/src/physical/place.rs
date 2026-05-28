use world_core::EntityId;
use world_defs::{EffectPrimitiveId, RoleName};
use world_model::{HardStateChange, RelationFamily, RelationKey};
use world_runtime::{
    PrimitiveInvocation, PrimitiveSemantics, PrimitiveSemanticsContract, PrimitiveStageContext,
    PrimitiveValidationContext, PrimitiveValidationFailure, RuntimeError,
};

use crate::events;

/// Trusted runtime semantics for the standard containment placement primitive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlaceEntitySemantics;

impl PrimitiveSemantics for PlaceEntitySemantics {
    fn primitive(&self) -> EffectPrimitiveId {
        world_standard::ids::place_entity()
    }

    fn contract(&self) -> PrimitiveSemanticsContract {
        events::contract(&world_standard::primitives::physical::PlaceEntity)
    }

    fn validate(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveValidationContext<'_>,
    ) -> Result<(), PrimitiveValidationFailure> {
        let item_role = invocation.required_role(&world_standard::ids::item_param())?;
        let destination_role =
            invocation.required_role(&world_standard::ids::destination_param())?;
        let (item_role, item) = context.required_role_entity(&item_role)?;
        let (destination_role, destination) = context.required_role_entity(&destination_role)?;
        validate_visible_entity(context, item_role, item)?;
        validate_visible_entity(context, destination_role, destination)?;

        let relation = RelationKey::new(item, RelationFamily::ContainedIn, destination);
        if context.contains_relation(relation) {
            return Err(world_runtime::RejectedOutcome::new(
                context.action(),
                world_runtime::RejectionReason::RelationAlreadyPresent {
                    subject: item,
                    family: RelationFamily::ContainedIn,
                    object: destination,
                },
            )
            .into());
        }

        context.insert_relation(relation);
        Ok(())
    }

    fn stage(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveStageContext<'_, '_, '_, '_>,
    ) -> Result<(), RuntimeError> {
        events::ensure_event_permission(invocation.primitive())?;
        let item_role = invocation.required_role(&world_standard::ids::item_param())?;
        let destination_role =
            invocation.required_role(&world_standard::ids::destination_param())?;
        let (item_role, item) = context.required_role_entity(&item_role)?;
        let (destination_role, destination) = context.required_role_entity(&destination_role)?;
        require_visible_entity(context, item_role, item)?;
        require_visible_entity(context, destination_role, destination)?;

        let relation = RelationKey::new(item, RelationFamily::ContainedIn, destination);
        if context.contains_relation(relation) {
            return Err(RuntimeError::DuplicateVisibleRelation {
                subject: item,
                family: RelationFamily::ContainedIn,
                object: destination,
            });
        }

        context.stage_physical_change(
            invocation,
            HardStateChange::insert_relation(
                item,
                RelationFamily::ContainedIn,
                destination,
                context.provenance(),
            ),
        )?;
        context.emit_declared_events(invocation)
    }
}

fn validate_visible_entity(
    context: &PrimitiveValidationContext<'_>,
    role: RoleName,
    entity: EntityId,
) -> Result<(), PrimitiveValidationFailure> {
    if context.contains_entity(entity) {
        Ok(())
    } else {
        Err(world_runtime::RejectedOutcome::new(
            context.action(),
            world_runtime::RejectionReason::MissingEntity { role, entity },
        )
        .into())
    }
}

fn require_visible_entity(
    context: &PrimitiveStageContext<'_, '_, '_, '_>,
    role: RoleName,
    entity: EntityId,
) -> Result<(), RuntimeError> {
    if context.contains_entity(entity) {
        Ok(())
    } else {
        Err(RuntimeError::MissingVisibleEntity { role, entity })
    }
}
