use world_core::{DefinitionId, VersionAnchor};
use world_defs::{DefinitionName, EffectParamName, EffectPrimitiveId, EventKind, RoleName};

/// Standard primitive id for creating an entity.
pub fn create_entity() -> EffectPrimitiveId {
    primitive_id(1_001)
}

/// Standard primitive id for placing an entity into a hard containment relation.
pub fn place_entity() -> EffectPrimitiveId {
    primitive_id(1_002)
}

/// Standard primitive id for acquiring an exclusive runtime reservation.
pub fn acquire_reservation() -> EffectPrimitiveId {
    primitive_id(1_003)
}

/// Definition-only process scheduling primitive used by process declarations.
pub fn schedule_process() -> EffectPrimitiveId {
    primitive_id(1_004)
}

/// Standard primitive schema version for the initial primitive surface.
pub fn primitive_version() -> VersionAnchor {
    let Some(version) = VersionAnchor::new(1) else {
        unreachable!("standard primitive version is nonzero");
    };
    version
}

/// Role parameter used by entity-creating primitives.
pub fn entity_param() -> EffectParamName {
    param("entity")
}

/// Role parameter used by placement and reservation primitives.
pub fn item_param() -> EffectParamName {
    param("item")
}

/// Role parameter used by placement primitives.
pub fn destination_param() -> EffectParamName {
    param("destination")
}

/// Optional holder role parameter for reservation primitives.
pub fn holder_param() -> EffectParamName {
    param("holder")
}

/// Standard action/process role for the acting entity.
pub fn actor_role() -> RoleName {
    role("actor")
}

/// Standard action/process role for a created entity.
pub fn entity_role() -> RoleName {
    role("entity")
}

/// Standard action/process role for an item-like subject entity.
pub fn item_role() -> RoleName {
    role("item")
}

/// Standard action/process role for a destination entity.
pub fn destination_role() -> RoleName {
    role("destination")
}

pub(crate) fn primitive_name(value: &'static str) -> DefinitionName {
    let Some(name) = DefinitionName::new(value) else {
        unreachable!("standard primitive names are non-empty");
    };
    name
}

pub(crate) fn event_kind(value: &'static str) -> EventKind {
    let Some(kind) = EventKind::new(value) else {
        unreachable!("standard event kinds are non-empty");
    };
    kind
}

fn primitive_id(value: u64) -> EffectPrimitiveId {
    let Some(id) = DefinitionId::new(value) else {
        unreachable!("standard primitive ids are nonzero");
    };
    EffectPrimitiveId::new(id)
}

fn param(value: &'static str) -> EffectParamName {
    let Some(name) = EffectParamName::new(value) else {
        unreachable!("standard parameter names are non-empty");
    };
    name
}

fn role(value: &'static str) -> RoleName {
    let Some(name) = RoleName::new(value) else {
        unreachable!("standard role names are non-empty");
    };
    name
}
