use std::collections::BTreeSet;

use world_core::DefinitionId;

use crate::effects::{EffectOp, EffectPrimitiveDef, EffectProgramDef, StagePermission};
use crate::error::DefinitionError;
use crate::events::{EventContract, EventRecordSpec};
use crate::keys::RoleName;
use crate::roles::RoleDef;

use super::DefinitionRegistry;

pub(super) fn registry(registry: &DefinitionRegistry) -> Result<(), DefinitionError> {
    validate_primitives(registry)?;
    validate_effect_programs(registry)?;
    validate_actions(registry)?;
    validate_processes(registry)
}

fn validate_primitives(registry: &DefinitionRegistry) -> Result<(), DefinitionError> {
    let mut names = BTreeSet::new();
    for primitive in registry.effect_primitives.values() {
        if !names.insert(primitive.name().clone()) {
            return Err(DefinitionError::DuplicatePrimitiveName {
                name: primitive.name().clone(),
            });
        }
    }

    Ok(())
}

fn validate_effect_programs(registry: &DefinitionRegistry) -> Result<(), DefinitionError> {
    for program in registry.effect_programs.values() {
        validate_program_schema(registry, program)?;
    }

    Ok(())
}

fn validate_actions(registry: &DefinitionRegistry) -> Result<(), DefinitionError> {
    for action in registry.actions.values() {
        let Some(program) = registry.effect_program(action.effect_program()) else {
            return Err(DefinitionError::MissingEffectProgram {
                definition: action.id(),
                effect_program: action.effect_program(),
            });
        };

        let declared_permissions = action.stage_permissions().copied().collect::<BTreeSet<_>>();
        let declared_roles = role_set(action.roles());
        validate_program_against_definition(
            registry,
            action.id(),
            program,
            &declared_permissions,
            action.event_contract(),
            &declared_roles,
        )?;
        validate_event_coverage(
            action.id(),
            action.event_contract(),
            &program.emitted_events(),
        )?;
    }

    Ok(())
}

fn validate_processes(registry: &DefinitionRegistry) -> Result<(), DefinitionError> {
    for process in registry.processes.values() {
        let declared_permissions = process
            .stage_permissions()
            .copied()
            .collect::<BTreeSet<_>>();
        let declared_roles = role_set(process.roles());

        for support in process
            .supported_resolutions()
            .filter_map(|resolution| process.resolution_support(*resolution))
        {
            let mut available_events = BTreeSet::new();

            for effect_program in support.effect_programs() {
                let Some(program) = registry.effect_program(*effect_program) else {
                    return Err(DefinitionError::MissingEffectProgram {
                        definition: process.id(),
                        effect_program: *effect_program,
                    });
                };

                validate_program_against_definition(
                    registry,
                    process.id(),
                    program,
                    &declared_permissions,
                    process.event_contract(),
                    &declared_roles,
                )?;
                available_events.extend(program.emitted_events());
            }

            validate_event_coverage(process.id(), process.event_contract(), &available_events)?;
        }
    }

    Ok(())
}

fn validate_program_against_definition(
    registry: &DefinitionRegistry,
    definition: DefinitionId,
    program: &EffectProgramDef,
    declared_permissions: &BTreeSet<StagePermission>,
    definition_contract: &EventContract,
    declared_roles: &BTreeSet<RoleName>,
) -> Result<(), DefinitionError> {
    validate_role_coverage(definition, program, declared_roles)?;
    let required_permissions = required_permissions(registry, program);
    validate_permission_coverage(
        definition,
        program.id(),
        declared_permissions,
        required_permissions.iter(),
    )?;
    validate_required_event_declaration(definition, definition_contract, program.event_contract())?;
    validate_permitted_events(definition, definition_contract, &program.emitted_events())
}

fn validate_program_schema(
    registry: &DefinitionRegistry,
    program: &EffectProgramDef,
) -> Result<(), DefinitionError> {
    for operation in program.operations() {
        let Some(primitive) = registry.effect_primitive(operation.primitive()) else {
            return Err(DefinitionError::MissingEffectPrimitive {
                definition: program.id(),
                effect_program: program.id(),
                primitive: operation.primitive(),
            });
        };

        validate_operation_args(program.id(), operation, primitive)?;
        validate_operation_events(program, operation, primitive)?;
        validate_program_replay(program, primitive)?;
    }

    Ok(())
}

fn validate_operation_args(
    effect_program: DefinitionId,
    operation: &EffectOp,
    primitive: &EffectPrimitiveDef,
) -> Result<(), DefinitionError> {
    for arg in operation.args() {
        let Some(param) = primitive.param(arg.param()) else {
            return Err(DefinitionError::UnknownEffectArg {
                definition: effect_program,
                effect_program,
                primitive: primitive.id(),
                param: arg.param().clone(),
            });
        };
        let actual = arg.value().kind();
        if !param.kind().accepts(actual) {
            return Err(DefinitionError::EffectArgKindMismatch {
                definition: effect_program,
                effect_program,
                primitive: primitive.id(),
                param: arg.param().clone(),
                expected: param.kind(),
                actual,
            });
        }
    }

    for param in primitive.params() {
        if param.kind().is_required() && operation.arg(param.name()).is_none() {
            return Err(DefinitionError::MissingEffectArg {
                definition: effect_program,
                effect_program,
                primitive: primitive.id(),
                param: param.name().clone(),
            });
        }
    }

    Ok(())
}

fn validate_operation_events(
    program: &EffectProgramDef,
    operation: &EffectOp,
    primitive: &EffectPrimitiveDef,
) -> Result<(), DefinitionError> {
    for event in primitive.event_contract().required_events() {
        if !operation.emits_event(event) {
            return Err(DefinitionError::PrimitiveRequiredEventNotEmitted {
                definition: program.id(),
                effect_program: program.id(),
                primitive: primitive.id(),
                event: event.clone(),
            });
        }
        if !program.event_contract().requires_event(event) {
            return Err(DefinitionError::PrimitiveRequiredEventNotDeclared {
                effect_program: program.id(),
                primitive: primitive.id(),
                event: event.clone(),
            });
        }
    }
    for event in operation.emitted_events() {
        if !primitive.event_contract().permits_event(event) {
            return Err(DefinitionError::OperationEventNotPermittedByPrimitive {
                definition: program.id(),
                effect_program: program.id(),
                primitive: primitive.id(),
                event: event.clone(),
            });
        }
    }

    Ok(())
}

fn validate_program_replay(
    program: &EffectProgramDef,
    primitive: &EffectPrimitiveDef,
) -> Result<(), DefinitionError> {
    if program.replay_level() < primitive.replay_level() {
        return Err(DefinitionError::EffectProgramReplayTooWeak {
            effect_program: program.id(),
            primitive: primitive.id(),
            program_replay: program.replay_level(),
            primitive_replay: primitive.replay_level(),
        });
    }

    Ok(())
}

fn required_permissions(
    registry: &DefinitionRegistry,
    program: &EffectProgramDef,
) -> BTreeSet<StagePermission> {
    program
        .operations()
        .iter()
        .filter_map(|operation| registry.effect_primitive(operation.primitive()))
        .flat_map(EffectPrimitiveDef::required_permissions)
        .copied()
        .collect()
}

fn validate_role_coverage(
    definition: DefinitionId,
    program: &EffectProgramDef,
    declared_roles: &BTreeSet<RoleName>,
) -> Result<(), DefinitionError> {
    for role in program
        .operations()
        .iter()
        .flat_map(EffectOp::args)
        .filter_map(|arg| arg.value().role())
    {
        if !declared_roles.contains(role) {
            return Err(DefinitionError::UnknownRole {
                definition,
                role: role.clone(),
            });
        }
    }

    Ok(())
}

fn validate_permission_coverage<'a>(
    definition: DefinitionId,
    effect_program: DefinitionId,
    declared: &BTreeSet<StagePermission>,
    required: impl Iterator<Item = &'a StagePermission>,
) -> Result<(), DefinitionError> {
    for permission in required {
        if !declared.contains(permission) {
            return Err(DefinitionError::PermissionNotDeclared {
                definition,
                effect_program,
                permission: *permission,
            });
        }
    }

    Ok(())
}

fn role_set(roles: &[RoleDef]) -> BTreeSet<RoleName> {
    roles.iter().map(|role| role.name().clone()).collect()
}

fn validate_event_coverage(
    definition: DefinitionId,
    event_contract: &EventContract,
    available_events: &BTreeSet<EventRecordSpec>,
) -> Result<(), DefinitionError> {
    for event in event_contract.required_events() {
        if !available_events.contains(event) {
            return Err(DefinitionError::RequiredEventUnavailable {
                definition,
                event: event.clone(),
            });
        }
    }

    Ok(())
}

fn validate_permitted_events(
    definition: DefinitionId,
    event_contract: &EventContract,
    emitted_events: &BTreeSet<EventRecordSpec>,
) -> Result<(), DefinitionError> {
    for event in emitted_events {
        if !event_contract.permits_event(event) {
            return Err(DefinitionError::EventNotPermittedByContract {
                definition,
                event: event.clone(),
            });
        }
    }

    Ok(())
}

fn validate_required_event_declaration(
    definition: DefinitionId,
    definition_contract: &EventContract,
    program_contract: &EventContract,
) -> Result<(), DefinitionError> {
    for event in program_contract.required_events() {
        if !definition_contract.requires_event(event) {
            return Err(DefinitionError::RequiredEventNotDeclared {
                definition,
                event: event.clone(),
            });
        }
    }

    Ok(())
}
