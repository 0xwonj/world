use world_core::{EntityId, EventRecordIdIssuer, ProvenanceKey};
use world_defs::{EffectOp, EffectProgramDef, EventRecordSpec, RoleName, StagePermission};
use world_model::{EventRoleBinding, HardStateChange, RelationKey};

use crate::{
    RuntimeError,
    builtin::{BuiltinEffect, BuiltinRole},
    request::BoundRuntimeRequest,
    transaction::{EffectStager, PendingEventRecord},
};

pub(crate) struct TypedEffectInterpreter;

impl TypedEffectInterpreter {
    pub(crate) fn interpret(
        &self,
        program: &EffectProgramDef,
        request: &BoundRuntimeRequest,
        stager: &mut EffectStager<'_, '_>,
        event_ids: &mut EventRecordIdIssuer,
    ) -> Result<(), RuntimeError> {
        let mut context = StageContext::new(request, stager, event_ids);
        for operation in program.operations() {
            BuiltinEffect::from_operation(operation)?.stage(&mut context, operation)?;
        }

        Ok(())
    }
}

fn event_roles(
    spec: &EventRecordSpec,
    request: &BoundRuntimeRequest,
) -> Result<Vec<EventRoleBinding>, RuntimeError> {
    spec.roles()
        .map(|role| {
            request
                .bound_role_entity(role)
                .map(|entity| EventRoleBinding::new(role.clone(), entity))
                .ok_or_else(|| RuntimeError::MissingBoundRole { role: role.clone() })
        })
        .collect()
}

pub(crate) struct StageContext<'request, 'stager, 'model, 'tx> {
    request: &'request BoundRuntimeRequest,
    stager: &'stager mut EffectStager<'model, 'tx>,
    event_ids: &'stager mut EventRecordIdIssuer,
}

impl<'request, 'stager, 'model, 'tx> StageContext<'request, 'stager, 'model, 'tx> {
    fn new(
        request: &'request BoundRuntimeRequest,
        stager: &'stager mut EffectStager<'model, 'tx>,
        event_ids: &'stager mut EventRecordIdIssuer,
    ) -> Self {
        Self {
            request,
            stager,
            event_ids,
        }
    }

    pub(crate) fn required_role(
        &self,
        role: BuiltinRole,
    ) -> Result<(RoleName, EntityId), RuntimeError> {
        let role = role.name()?;
        let entity = self
            .request
            .bound_role_entity(&role)
            .ok_or_else(|| RuntimeError::MissingBoundRole { role: role.clone() })?;
        Ok((role, entity))
    }

    pub(crate) const fn provenance(&self) -> Option<ProvenanceKey> {
        self.request.provenance()
    }

    pub(crate) fn contains_entity(&self, entity: EntityId) -> bool {
        self.stager.contains_entity(entity)
    }

    pub(crate) fn contains_relation(&self, key: RelationKey) -> bool {
        self.stager.contains_relation(key)
    }

    pub(crate) fn push_change(&mut self, change: HardStateChange) {
        self.stager.push_change(change);
    }

    pub(crate) fn emit_declared_events(
        &mut self,
        operation: &EffectOp,
    ) -> Result<(), RuntimeError> {
        for spec in operation.emitted_events() {
            self.emit_event(operation, spec)?;
        }

        Ok(())
    }

    pub(crate) fn emit_event(
        &mut self,
        operation: &EffectOp,
        spec: &EventRecordSpec,
    ) -> Result<(), RuntimeError> {
        require_event_permission(operation)?;
        if !operation.emits_event(spec) {
            return Err(RuntimeError::EventNotDeclaredForOperation {
                operation: operation.kind().clone(),
                event: spec.clone(),
            });
        }

        let Some(event_id) = self.event_ids.issue() else {
            return Err(RuntimeError::EventIdExhausted);
        };
        let roles = event_roles(spec, self.request)?;
        let event = PendingEventRecord::new(event_id, spec.clone(), roles, self.provenance());
        self.stager.push_event(event)
    }
}

fn require_event_permission(operation: &EffectOp) -> Result<(), RuntimeError> {
    if operation.requires_permission(StagePermission::EmitPhysicalEventRecord)
        || operation.requires_permission(StagePermission::EmitSensoryEventRecord)
        || operation.emits_no_events()
    {
        Ok(())
    } else {
        Err(RuntimeError::PermissionNotDeclared {
            operation: operation.kind().clone(),
            permission: StagePermission::EmitPhysicalEventRecord,
        })
    }
}
