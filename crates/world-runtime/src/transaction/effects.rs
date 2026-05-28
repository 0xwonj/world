use world_core::{EntityId, EventRecordIdIssuer, ProvenanceKey};
use world_defs::{EffectProgramDef, EventRecordSpec, RoleName, StagePermission};
use world_model::{EventRoleBinding, HardStateChange, RelationKey};

use crate::{
    RuntimeError,
    control::{AcquireReservationRequest, ReservationRuntime, RuntimeControlIds},
    primitive::{PrimitiveInvocation, PrimitiveSemanticsRegistry},
    request::BoundRuntimeRequest,
    transaction::{EffectStager, PendingEventRecord},
};

pub(crate) struct TypedEffectInterpreter;

pub(crate) struct EffectInterpretation<'request, 'stager, 'model, 'tx> {
    pub(crate) definitions: &'request world_defs::DefinitionRegistry,
    pub(crate) semantics: &'request PrimitiveSemanticsRegistry,
    pub(crate) request: &'request BoundRuntimeRequest,
    pub(crate) stager: &'stager mut EffectStager<'model, 'tx>,
    pub(crate) event_ids: &'stager mut EventRecordIdIssuer,
    pub(crate) control_ids: &'stager mut RuntimeControlIds,
}

impl TypedEffectInterpreter {
    pub(crate) fn interpret(
        &self,
        program: &EffectProgramDef,
        input: EffectInterpretation<'_, '_, '_, '_>,
    ) -> Result<(), RuntimeError> {
        let mut context = PrimitiveStageContext::new(
            input.request,
            input.stager,
            input.event_ids,
            input.control_ids,
        );
        for operation in program.operations() {
            let Some(primitive) = input.definitions.effect_primitive(operation.primitive()) else {
                return Err(RuntimeError::PrimitiveSemanticsForUnknownDefinition {
                    primitive: operation.primitive(),
                });
            };
            let Some(handler) = input.semantics.handler(operation.primitive()) else {
                return Err(RuntimeError::MissingPrimitiveSemantics {
                    primitive: operation.primitive(),
                });
            };
            handler.stage(PrimitiveInvocation::new(operation, primitive), &mut context)?;
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

/// Capability-gated staging context exposed to trusted primitive semantics.
pub struct PrimitiveStageContext<'request, 'stager, 'model, 'tx> {
    request: &'request BoundRuntimeRequest,
    stager: &'stager mut EffectStager<'model, 'tx>,
    event_ids: &'stager mut EventRecordIdIssuer,
    control_ids: &'stager mut RuntimeControlIds,
}

impl<'request, 'stager, 'model, 'tx> PrimitiveStageContext<'request, 'stager, 'model, 'tx> {
    pub(crate) fn new(
        request: &'request BoundRuntimeRequest,
        stager: &'stager mut EffectStager<'model, 'tx>,
        event_ids: &'stager mut EventRecordIdIssuer,
        control_ids: &'stager mut RuntimeControlIds,
    ) -> Self {
        Self {
            request,
            stager,
            event_ids,
            control_ids,
        }
    }

    /// Resolves a required role binding to its entity.
    pub fn required_role_entity(
        &self,
        role: &RoleName,
    ) -> Result<(RoleName, EntityId), RuntimeError> {
        let entity = self
            .request
            .bound_role_entity(role)
            .ok_or_else(|| RuntimeError::MissingBoundRole { role: role.clone() })?;
        Ok((role.clone(), entity))
    }

    /// Resolves an optional role binding to its entity.
    pub fn optional_role_entity(&self, role: &RoleName) -> Option<EntityId> {
        self.request.bound_role_entity(role)
    }

    /// Returns request provenance.
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.request.provenance()
    }

    /// Returns request submission time.
    pub const fn request_time(&self) -> world_core::SimulationTime {
        self.request.submitted_at()
    }

    /// Returns true when an entity is committed or staged in this transaction.
    pub fn contains_entity(&self, entity: EntityId) -> bool {
        self.stager.contains_entity(entity)
    }

    /// Returns true when a relation is committed or staged in this transaction.
    pub fn contains_relation(&self, key: RelationKey) -> bool {
        self.stager.contains_relation(key)
    }

    /// Stages a hard physical change after checking primitive authority.
    pub fn stage_physical_change(
        &mut self,
        invocation: PrimitiveInvocation<'_>,
        change: HardStateChange,
    ) -> Result<(), RuntimeError> {
        require_permission(invocation, StagePermission::MutatePhysical)?;
        self.stager.push_change(change);
        Ok(())
    }

    /// Stages a reservation acquisition after checking primitive authority.
    pub fn stage_reservation_acquire(
        &mut self,
        invocation: PrimitiveInvocation<'_>,
        request: AcquireReservationRequest,
    ) -> Result<(), RuntimeError> {
        require_permission(invocation, StagePermission::AcquireReservation)?;
        let change = ReservationRuntime::acquire(self.control_ids, request)?;
        self.stager.push_control_change(change);
        Ok(())
    }

    /// Emits every event selected by the primitive operation.
    pub fn emit_declared_events(
        &mut self,
        invocation: PrimitiveInvocation<'_>,
    ) -> Result<(), RuntimeError> {
        for spec in invocation.operation().emitted_events() {
            self.emit_event(invocation, spec)?;
        }

        Ok(())
    }

    /// Emits a selected event after checking primitive and operation authority.
    pub fn emit_event(
        &mut self,
        invocation: PrimitiveInvocation<'_>,
        spec: &EventRecordSpec,
    ) -> Result<(), RuntimeError> {
        require_event_permission(invocation)?;
        if !invocation.operation().emits_event(spec) {
            return Err(RuntimeError::EventNotDeclaredForOperation {
                primitive: invocation.primitive().id(),
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

fn require_event_permission(invocation: PrimitiveInvocation<'_>) -> Result<(), RuntimeError> {
    if invocation
        .primitive()
        .requires_permission(StagePermission::EmitPhysicalEventRecord)
        || invocation
            .primitive()
            .requires_permission(StagePermission::EmitSensoryEventRecord)
        || invocation.operation().emits_no_events()
    {
        Ok(())
    } else {
        Err(RuntimeError::PermissionNotDeclared {
            primitive: invocation.primitive().id(),
            permission: StagePermission::EmitPhysicalEventRecord,
        })
    }
}

fn require_permission(
    invocation: PrimitiveInvocation<'_>,
    permission: StagePermission,
) -> Result<(), RuntimeError> {
    if invocation.primitive().requires_permission(permission) {
        Ok(())
    } else {
        Err(RuntimeError::PermissionNotDeclared {
            primitive: invocation.primitive().id(),
            permission,
        })
    }
}
