use std::collections::BTreeSet;

use world_core::{DefinitionId, ReplayLevel, VersionAnchor};

use crate::error::DefinitionError;
use crate::events::{EventContract, EventRecordSpec};
use crate::keys::{DefinitionName, EffectParamName};

use super::{EffectArgBinding, EffectPrimitiveId};

/// One checked primitive operation in a typed effect program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectOp {
    primitive: EffectPrimitiveId,
    args: Vec<EffectArgBinding>,
    emitted_events: Vec<EventRecordSpec>,
}

impl EffectOp {
    /// Creates an operation that calls a checked primitive definition.
    pub fn new(
        primitive: EffectPrimitiveId,
        args: impl IntoIterator<Item = EffectArgBinding>,
        emitted_events: impl IntoIterator<Item = EventRecordSpec>,
    ) -> Result<Self, DefinitionError> {
        let args = args.into_iter().collect::<Vec<_>>();
        let emitted_events = emitted_events.into_iter().collect::<Vec<_>>();
        validate_unique_args(primitive, &args)?;
        validate_unique_events(primitive, &emitted_events)?;

        Ok(Self {
            primitive,
            args,
            emitted_events,
        })
    }

    /// Returns the primitive this operation invokes.
    pub const fn primitive(&self) -> EffectPrimitiveId {
        self.primitive
    }

    /// Returns argument bindings in source order.
    pub fn args(&self) -> &[EffectArgBinding] {
        &self.args
    }

    /// Looks up an argument binding by parameter name.
    pub fn arg(&self, param: &EffectParamName) -> Option<&EffectArgBinding> {
        self.args.iter().find(|arg| arg.param() == param)
    }

    /// Returns whether this operation can emit no events.
    pub fn emits_no_events(&self) -> bool {
        self.emitted_events.is_empty()
    }

    /// Returns true when this operation emits the event spec.
    pub fn emits_event(&self, event: &EventRecordSpec) -> bool {
        self.emitted_events.contains(event)
    }

    /// Returns event specs this operation can emit in declaration order.
    pub fn emitted_events(&self) -> impl Iterator<Item = &EventRecordSpec> {
        self.emitted_events.iter()
    }
}

/// Checked typed effect program definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectProgramDef {
    id: DefinitionId,
    name: DefinitionName,
    operations: Vec<EffectOp>,
    event_contract: EventContract,
    replay_level: ReplayLevel,
    version: VersionAnchor,
}

impl EffectProgramDef {
    /// Creates a checked effect program definition.
    pub fn new(
        id: DefinitionId,
        name: DefinitionName,
        operations: impl IntoIterator<Item = EffectOp>,
        event_contract: EventContract,
        replay_level: ReplayLevel,
        version: VersionAnchor,
    ) -> Result<Self, DefinitionError> {
        let operations = operations.into_iter().collect::<Vec<_>>();
        if operations.is_empty() {
            return Err(DefinitionError::EmptyDefinitionField {
                definition: id,
                type_name: "EffectProgramDef",
                field: "operations",
            });
        }

        let emitted_events = collect_emitted_events(&operations);
        for event in event_contract.required_events() {
            if !emitted_events.contains(event) {
                return Err(DefinitionError::RequiredEventNotEmitted {
                    definition: id,
                    event: event.clone(),
                });
            }
        }
        for event in operations.iter().flat_map(EffectOp::emitted_events) {
            if !event_contract.permits_event(event) {
                return Err(DefinitionError::EventNotPermittedByContract {
                    definition: id,
                    event: event.clone(),
                });
            }
        }

        Ok(Self {
            id,
            name,
            operations,
            event_contract,
            replay_level,
            version,
        })
    }

    /// Returns the definition id.
    pub fn id(&self) -> DefinitionId {
        self.id
    }

    /// Returns the definition name.
    pub fn name(&self) -> &DefinitionName {
        &self.name
    }

    /// Returns checked operations.
    pub fn operations(&self) -> &[EffectOp] {
        &self.operations
    }

    /// Returns the event contract.
    pub fn event_contract(&self) -> &EventContract {
        &self.event_contract
    }

    /// Returns declared replay level.
    pub fn replay_level(&self) -> ReplayLevel {
        self.replay_level
    }

    /// Returns the version anchor.
    pub fn version(&self) -> VersionAnchor {
        self.version
    }

    /// Returns the union of event specs that operations can emit.
    pub fn emitted_events(&self) -> BTreeSet<EventRecordSpec> {
        collect_emitted_events(&self.operations)
    }
}

fn collect_emitted_events(operations: &[EffectOp]) -> BTreeSet<EventRecordSpec> {
    operations
        .iter()
        .flat_map(EffectOp::emitted_events)
        .cloned()
        .collect()
}

fn validate_unique_args(
    primitive: EffectPrimitiveId,
    args: &[EffectArgBinding],
) -> Result<(), DefinitionError> {
    let mut seen = BTreeSet::new();
    for arg in args {
        if !seen.insert(arg.param().clone()) {
            return Err(DefinitionError::DuplicateEffectArg {
                primitive,
                param: arg.param().clone(),
            });
        }
    }

    Ok(())
}

fn validate_unique_events(
    primitive: EffectPrimitiveId,
    events: &[EventRecordSpec],
) -> Result<(), DefinitionError> {
    let mut seen = BTreeSet::new();
    for event in events {
        if !seen.insert(event.clone()) {
            return Err(DefinitionError::DuplicateEffectEvent {
                primitive,
                event: event.clone(),
            });
        }
    }

    Ok(())
}
