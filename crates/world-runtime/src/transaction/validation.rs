use std::collections::BTreeSet;

use world_core::{DefinitionId, EntityId};
use world_defs::{ActionDef, DefinitionRegistry, EffectProgramDef, RoleName};
use world_model::{
    RelationKey, ReservationState, ReservationTarget, RuntimeControlRecordPayload, WorldModel,
};

use crate::{
    RuntimeError,
    outcome::{RejectedOutcome, RejectionReason},
    primitive::{PrimitiveInvocation, PrimitiveSemanticsRegistry, PrimitiveValidationFailure},
    request::BoundRuntimeRequest,
};

pub(crate) struct RuntimeValidator;

impl RuntimeValidator {
    pub(crate) fn validate(
        model: &WorldModel,
        action: &ActionDef,
        request: &BoundRuntimeRequest,
        program: &EffectProgramDef,
        definitions: &DefinitionRegistry,
        semantics: &PrimitiveSemanticsRegistry,
    ) -> Result<(), PrimitiveValidationFailure> {
        reject_unsupported_declarations(action)?;
        validate_actor_binding(action, request)?;
        let mut context = PrimitiveValidationContext::new(model, action.id(), request);
        for operation in program.operations() {
            let Some(primitive) = definitions.effect_primitive(operation.primitive()) else {
                return Err(RuntimeError::PrimitiveSemanticsForUnknownDefinition {
                    primitive: operation.primitive(),
                }
                .into());
            };
            let Some(handler) = semantics.handler(operation.primitive()) else {
                return Err(RuntimeError::MissingPrimitiveSemantics {
                    primitive: operation.primitive(),
                }
                .into());
            };
            handler.validate(PrimitiveInvocation::new(operation, primitive), &mut context)?;
        }

        Ok(())
    }
}

fn reject_unsupported_declarations(action: &ActionDef) -> Result<(), PrimitiveValidationFailure> {
    if let Some(requirement) = action.requirements().first() {
        return Err(RejectedOutcome::new(
            action.id(),
            RejectionReason::UnsupportedRequirement {
                requirement: requirement.kind().clone(),
            },
        )
        .into());
    }

    if let Some(binding_rule) = action.binding_rules().first() {
        return Err(RejectedOutcome::new(
            action.id(),
            RejectionReason::UnsupportedBindingRule {
                binding_rule: binding_rule.kind().clone(),
            },
        )
        .into());
    }

    Ok(())
}

fn validate_actor_binding(
    action: &ActionDef,
    request: &BoundRuntimeRequest,
) -> Result<(), PrimitiveValidationFailure> {
    let Some(actor) = request.actor() else {
        return Ok(());
    };
    let Some(actor_role) = action.actor_role() else {
        return Ok(());
    };
    let Some(bound_actor) = request.bound_role_entity(actor_role) else {
        return Ok(());
    };

    if actor != bound_actor {
        return Err(RejectedOutcome::new(
            action.id(),
            RejectionReason::ActorRoleMismatch { actor, bound_actor },
        )
        .into());
    }

    Ok(())
}

/// Current-world validation context exposed to trusted primitive semantics.
pub struct PrimitiveValidationContext<'model> {
    action: DefinitionId,
    request: &'model BoundRuntimeRequest,
    model: &'model WorldModel,
    entities: BTreeSet<EntityId>,
    relations: BTreeSet<RelationKey>,
    reservation_targets: BTreeSet<ReservationTarget>,
}

impl<'model> PrimitiveValidationContext<'model> {
    pub(crate) fn new(
        model: &'model WorldModel,
        action: DefinitionId,
        request: &'model BoundRuntimeRequest,
    ) -> Self {
        Self {
            action,
            request,
            model,
            entities: BTreeSet::new(),
            relations: BTreeSet::new(),
            reservation_targets: BTreeSet::new(),
        }
    }

    /// Returns the action currently being validated.
    pub const fn action(&self) -> DefinitionId {
        self.action
    }

    /// Resolves a required role binding to its entity.
    pub fn required_role_entity(
        &self,
        role: &RoleName,
    ) -> Result<(RoleName, EntityId), PrimitiveValidationFailure> {
        let Some(entity) = self.request.bound_role_entity(role) else {
            return Err(RejectedOutcome::new(
                self.action,
                RejectionReason::MissingRoleBinding { role: role.clone() },
            )
            .into());
        };

        Ok((role.clone(), entity))
    }

    /// Returns true when an entity is committed or staged by earlier validation.
    pub fn contains_entity(&self, entity: EntityId) -> bool {
        self.entities.contains(&entity) || self.model.world_store().contains_entity(entity)
    }

    /// Marks an entity as staged by validation for later operation visibility.
    pub fn insert_entity(&mut self, entity: EntityId) {
        self.entities.insert(entity);
    }

    /// Returns true when a relation is committed or staged by earlier validation.
    pub fn contains_relation(&self, relation: RelationKey) -> bool {
        self.relations.contains(&relation) || self.model.relation_store().contains(relation)
    }

    /// Marks a relation as staged by validation for later operation visibility.
    pub fn insert_relation(&mut self, relation: RelationKey) {
        self.relations.insert(relation);
    }

    /// Returns true when a reservation target is already held or staged as held.
    pub fn contains_active_reservation(&self, target: &ReservationTarget) -> bool {
        self.reservation_targets.contains(target)
            || self.model.runtime_control_store().records().any(|record| {
                let RuntimeControlRecordPayload::Reservation(reservation) = record.payload() else {
                    return false;
                };
                matches!(reservation.state(), ReservationState::Held { .. })
                    && reservation.target() == target
            })
    }

    /// Marks a reservation target as staged by validation.
    pub fn insert_reservation_target(&mut self, target: ReservationTarget) {
        self.reservation_targets.insert(target);
    }
}
