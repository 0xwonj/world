use std::collections::BTreeSet;

use world_core::{DefinitionId, EntityId};
use world_defs::{ActionDef, EffectProgramDef, RoleName};
use world_model::{
    RelationKey, ReservationState, ReservationTarget, RuntimeControlRecordPayload, WorldModel,
};

use crate::{
    RuntimeError,
    builtin::{BuiltinEffect, BuiltinRole},
    outcome::{RejectedOutcome, RejectionReason},
    request::BoundRuntimeRequest,
};

pub(crate) struct RuntimeValidator;

impl RuntimeValidator {
    pub(crate) fn validate(
        model: &WorldModel,
        action: &ActionDef,
        request: &BoundRuntimeRequest,
        program: &EffectProgramDef,
    ) -> Result<(), RuntimeValidationFailure> {
        reject_unsupported_declarations(action)?;
        validate_actor_binding(action.id(), request)?;
        let mut context = ValidationContext::new(model, action.id(), request);
        for operation in program.operations() {
            BuiltinEffect::from_operation(operation)?.validate(&mut context, operation)?;
        }

        Ok(())
    }
}

pub(crate) enum RuntimeValidationFailure {
    Rejected(RejectedOutcome),
    Runtime(RuntimeError),
}

impl From<RejectedOutcome> for RuntimeValidationFailure {
    fn from(value: RejectedOutcome) -> Self {
        Self::Rejected(value)
    }
}

impl From<RuntimeError> for RuntimeValidationFailure {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

fn reject_unsupported_declarations(action: &ActionDef) -> Result<(), RuntimeValidationFailure> {
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
    action: DefinitionId,
    request: &BoundRuntimeRequest,
) -> Result<(), RuntimeValidationFailure> {
    let Some(actor) = request.actor() else {
        return Ok(());
    };
    let actor_role = BuiltinRole::Actor.name()?;
    let Some(bound_actor) = request.bound_role_entity(&actor_role) else {
        return Ok(());
    };

    if actor != bound_actor {
        return Err(RejectedOutcome::new(
            action,
            RejectionReason::ActorRoleMismatch { actor, bound_actor },
        )
        .into());
    }

    Ok(())
}

pub(crate) struct ValidationContext<'model> {
    action: DefinitionId,
    request: &'model BoundRuntimeRequest,
    model: &'model WorldModel,
    entities: BTreeSet<EntityId>,
    relations: BTreeSet<RelationKey>,
    reservation_targets: BTreeSet<ReservationTarget>,
}

impl<'model> ValidationContext<'model> {
    fn new(
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

    pub(crate) const fn action(&self) -> DefinitionId {
        self.action
    }

    pub(crate) fn required_role(
        &self,
        role: BuiltinRole,
    ) -> Result<(RoleName, EntityId), RuntimeValidationFailure> {
        let role = role.name()?;
        let Some(entity) = self.request.bound_role_entity(&role) else {
            return Err(RejectedOutcome::new(
                self.action,
                RejectionReason::MissingRoleBinding { role },
            )
            .into());
        };

        Ok((role, entity))
    }

    pub(crate) fn contains_entity(&self, entity: EntityId) -> bool {
        self.entities.contains(&entity) || self.model.world_store().contains_entity(entity)
    }

    pub(crate) fn insert_entity(&mut self, entity: EntityId) {
        self.entities.insert(entity);
    }

    pub(crate) fn contains_relation(&self, relation: RelationKey) -> bool {
        self.relations.contains(&relation) || self.model.relation_store().contains(relation)
    }

    pub(crate) fn insert_relation(&mut self, relation: RelationKey) {
        self.relations.insert(relation);
    }

    pub(crate) fn contains_active_reservation(&self, target: &ReservationTarget) -> bool {
        self.reservation_targets.contains(target)
            || self.model.runtime_control_store().records().any(|record| {
                let RuntimeControlRecordPayload::Reservation(reservation) = record.payload() else {
                    return false;
                };
                matches!(reservation.state(), ReservationState::Held { .. })
                    && reservation.target() == target
            })
    }

    pub(crate) fn insert_reservation_target(&mut self, target: ReservationTarget) {
        self.reservation_targets.insert(target);
    }
}
