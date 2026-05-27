use world_core::EntityId;
use world_defs::{EffectKind, EffectOp, RoleName, StagePermission};
use world_model::{HardStateChange, RelationFamily, RelationKey};

use crate::{
    RuntimeError,
    effects::StageContext,
    outcome::{RejectedOutcome, RejectionReason},
    validation::{RuntimeValidationFailure, ValidationContext},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuiltinRole {
    Actor,
    Entity,
    Item,
    Destination,
}

impl BuiltinRole {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::Entity => "entity",
            Self::Item => "item",
            Self::Destination => "destination",
        }
    }

    pub(crate) fn name(self) -> Result<RoleName, RuntimeError> {
        RoleName::new(self.as_str()).ok_or(RuntimeError::InvalidStaticRole {
            name: self.as_str(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuiltinEffect {
    InsertEntity,
    TransferEntity,
    RecordEvent,
}

impl BuiltinEffect {
    pub(crate) fn from_kind(kind: &EffectKind) -> Option<Self> {
        match kind.as_str() {
            "insert_entity" => Some(Self::InsertEntity),
            "transfer_entity" => Some(Self::TransferEntity),
            "record_event" => Some(Self::RecordEvent),
            _ => None,
        }
    }

    pub(crate) fn from_operation(operation: &EffectOp) -> Result<Self, RuntimeError> {
        Self::from_kind(operation.kind()).ok_or_else(|| RuntimeError::MissingEffectHandler {
            kind: operation.kind().clone(),
        })
    }

    pub(crate) fn validate(
        self,
        context: &mut ValidationContext<'_>,
        operation: &EffectOp,
    ) -> Result<(), RuntimeValidationFailure> {
        match self {
            Self::InsertEntity => validate_insert_entity(context),
            Self::TransferEntity => validate_transfer_entity(context),
            Self::RecordEvent => {
                validate_event_permission(operation)?;
                Ok(())
            }
        }
    }

    pub(crate) fn stage(
        self,
        context: &mut StageContext<'_, '_, '_, '_>,
        operation: &EffectOp,
    ) -> Result<(), RuntimeError> {
        match self {
            Self::InsertEntity => stage_insert_entity(context, operation),
            Self::TransferEntity => stage_transfer_entity(context, operation),
            Self::RecordEvent => {
                validate_event_permission(operation)?;
                context.emit_declared_events(operation)
            }
        }
    }
}

fn validate_insert_entity(
    context: &mut ValidationContext<'_>,
) -> Result<(), RuntimeValidationFailure> {
    let (role, entity) = context.required_role(BuiltinRole::Entity)?;
    if context.contains_entity(entity) {
        return Err(RejectedOutcome::new(
            context.action(),
            RejectionReason::EntityAlreadyPresent { role, entity },
        )
        .into());
    }

    context.insert_entity(entity);
    Ok(())
}

fn validate_transfer_entity(
    context: &mut ValidationContext<'_>,
) -> Result<(), RuntimeValidationFailure> {
    let (item_role, item) = context.required_role(BuiltinRole::Item)?;
    let (destination_role, destination) = context.required_role(BuiltinRole::Destination)?;
    validate_visible_entity(context, item_role, item)?;
    validate_visible_entity(context, destination_role, destination)?;

    let relation = RelationKey::new(item, RelationFamily::ContainedIn, destination);
    if context.contains_relation(relation) {
        return Err(RejectedOutcome::new(
            context.action(),
            RejectionReason::RelationAlreadyPresent {
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

fn validate_visible_entity(
    context: &ValidationContext<'_>,
    role: RoleName,
    entity: EntityId,
) -> Result<(), RuntimeValidationFailure> {
    if context.contains_entity(entity) {
        Ok(())
    } else {
        Err(RejectedOutcome::new(
            context.action(),
            RejectionReason::MissingEntity { role, entity },
        )
        .into())
    }
}

fn stage_insert_entity(
    context: &mut StageContext<'_, '_, '_, '_>,
    operation: &EffectOp,
) -> Result<(), RuntimeError> {
    require_permission(operation, StagePermission::MutatePhysical)?;
    let (role, entity) = context.required_role(BuiltinRole::Entity)?;
    if context.contains_entity(entity) {
        return Err(RuntimeError::DuplicateVisibleEntity { role, entity });
    }

    context.push_change(HardStateChange::insert_entity(
        entity,
        None,
        context.provenance(),
    ));
    context.emit_declared_events(operation)
}

fn stage_transfer_entity(
    context: &mut StageContext<'_, '_, '_, '_>,
    operation: &EffectOp,
) -> Result<(), RuntimeError> {
    require_permission(operation, StagePermission::MutatePhysical)?;
    let (item_role, item) = context.required_role(BuiltinRole::Item)?;
    let (destination_role, destination) = context.required_role(BuiltinRole::Destination)?;
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

    context.push_change(HardStateChange::insert_relation(
        item,
        RelationFamily::ContainedIn,
        destination,
        context.provenance(),
    ));
    context.emit_declared_events(operation)
}

fn require_visible_entity(
    context: &StageContext<'_, '_, '_, '_>,
    role: RoleName,
    entity: EntityId,
) -> Result<(), RuntimeError> {
    if context.contains_entity(entity) {
        Ok(())
    } else {
        Err(RuntimeError::MissingVisibleEntity { role, entity })
    }
}

fn validate_event_permission(operation: &EffectOp) -> Result<(), RuntimeError> {
    if operation.requires_permission(StagePermission::EmitPhysicalEventRecord)
        || operation.requires_permission(StagePermission::EmitSensoryEventRecord)
    {
        Ok(())
    } else {
        Err(RuntimeError::PermissionNotDeclared {
            operation: operation.kind().clone(),
            permission: StagePermission::EmitPhysicalEventRecord,
        })
    }
}

fn require_permission(
    operation: &EffectOp,
    permission: StagePermission,
) -> Result<(), RuntimeError> {
    if operation.requires_permission(permission) {
        Ok(())
    } else {
        Err(RuntimeError::PermissionNotDeclared {
            operation: operation.kind().clone(),
            permission,
        })
    }
}
