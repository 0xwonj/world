use crate::interface::{OperationName, SemanticInterfaceKey, ValueKind};
use crate::key::{BindingName, DefinitionKey, EventFieldName, LocalDefinitionName};

/// One named value supplied when an action is invoked.
///
/// The enclosing artifact validator checks uniqueness and any relationship
/// between bindings. This value records only the binding's name and declared
/// value kind.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActionBindingData {
    name: BindingName,
    value_kind: ValueKind,
}

impl ActionBindingData {
    /// Creates an action-binding declaration from checked leaf values.
    #[must_use]
    pub const fn new(name: BindingName, value_kind: ValueKind) -> Self {
        Self { name, value_kind }
    }

    /// Returns the binding name.
    #[must_use]
    pub const fn name(&self) -> &BindingName {
        &self.name
    }

    /// Returns the kind of value accepted by the binding.
    #[must_use]
    pub const fn value_kind(&self) -> &ValueKind {
        &self.value_kind
    }
}

/// A reference to one semantic-interface operation with ordered action
/// bindings as its arguments.
///
/// Argument order is part of the operation call. The artifact validator
/// resolves the interface and operation against a catalog and checks argument
/// names and kinds.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperationCallData {
    interface: SemanticInterfaceKey,
    operation: OperationName,
    arguments: Vec<BindingName>,
}

impl OperationCallData {
    /// Creates an operation call without resolving it against a catalog.
    #[must_use]
    pub fn new(
        interface: SemanticInterfaceKey,
        operation: OperationName,
        arguments: Vec<BindingName>,
    ) -> Self {
        Self {
            interface,
            operation,
            arguments,
        }
    }

    /// Returns the referenced semantic interface.
    #[must_use]
    pub const fn interface(&self) -> &SemanticInterfaceKey {
        &self.interface
    }

    /// Returns the referenced operation name.
    #[must_use]
    pub const fn operation(&self) -> &OperationName {
        &self.operation
    }

    /// Returns the ordered binding-name arguments.
    #[must_use]
    pub fn arguments(&self) -> &[BindingName] {
        &self.arguments
    }
}

/// An operation call evaluated as an authoritative runtime requirement.
///
/// This purpose-specific wrapper prevents a requirement call from being used
/// as an effect call without an explicit conversion by its owner.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeRequirementData {
    call: OperationCallData,
}

impl RuntimeRequirementData {
    /// Marks an operation call as a runtime requirement.
    #[must_use]
    pub const fn new(call: OperationCallData) -> Self {
        Self { call }
    }

    /// Returns the underlying operation call.
    #[must_use]
    pub const fn call(&self) -> &OperationCallData {
        &self.call
    }
}

/// An operation call evaluated as an authoritative effect.
///
/// The artifact validator checks that the referenced operation is legal in
/// the effect stage.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EffectCallData {
    call: OperationCallData,
}

impl EffectCallData {
    /// Marks an operation call as an effect.
    #[must_use]
    pub const fn new(call: OperationCallData) -> Self {
        Self { call }
    }

    /// Returns the underlying operation call.
    #[must_use]
    pub const fn call(&self) -> &OperationCallData {
        &self.call
    }
}

/// One named field in a physical event definition.
///
/// Field-name uniqueness is an event-level invariant checked when the
/// containing artifact is validated.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventFieldData {
    name: EventFieldName,
    value_kind: ValueKind,
}

impl EventFieldData {
    /// Creates an event-field declaration from checked leaf values.
    #[must_use]
    pub const fn new(name: EventFieldName, value_kind: ValueKind) -> Self {
        Self { name, value_kind }
    }

    /// Returns the field name.
    #[must_use]
    pub const fn name(&self) -> &EventFieldName {
        &self.name
    }

    /// Returns the field's declared value kind.
    #[must_use]
    pub const fn value_kind(&self) -> &ValueKind {
        &self.value_kind
    }
}

/// A mapping from an event field to an action binding.
///
/// The artifact validator checks that both names exist and their value kinds
/// agree.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventFieldBindingData {
    field: EventFieldName,
    binding: BindingName,
}

impl EventFieldBindingData {
    /// Creates an unresolved event-field mapping.
    #[must_use]
    pub const fn new(field: EventFieldName, binding: BindingName) -> Self {
        Self { field, binding }
    }

    /// Returns the target event field.
    #[must_use]
    pub const fn field(&self) -> &EventFieldName {
        &self.field
    }

    /// Returns the source action binding.
    #[must_use]
    pub const fn binding(&self) -> &BindingName {
        &self.binding
    }
}

/// One event emitted after an action succeeds.
///
/// Event existence, complete field coverage, duplicate mappings, and value
/// kinds are artifact-level invariants and are not claimed by construction.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventEmissionData {
    event: DefinitionKey,
    field_bindings: Vec<EventFieldBindingData>,
}

impl EventEmissionData {
    /// Creates an unresolved success-event declaration.
    #[must_use]
    pub fn new(event: DefinitionKey, field_bindings: Vec<EventFieldBindingData>) -> Self {
        Self {
            event,
            field_bindings,
        }
    }

    /// Returns the referenced event definition.
    #[must_use]
    pub const fn event(&self) -> &DefinitionKey {
        &self.event
    }

    /// Returns field mappings in their declared order.
    #[must_use]
    pub fn field_bindings(&self) -> &[EventFieldBindingData] {
        &self.field_bindings
    }

    pub(crate) fn into_parts(self) -> (DefinitionKey, Vec<EventFieldBindingData>) {
        (self.event, self.field_bindings)
    }
}

/// Input data for one action definition.
///
/// Construction preserves the declared order of requirements, effects, and
/// success events. It does not prove binding uniqueness, reference closure,
/// stage legality, or that the executable semantics are nonempty; those
/// invariants belong to whole-artifact validation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActionData {
    name: LocalDefinitionName,
    bindings: Vec<ActionBindingData>,
    requirements: Vec<RuntimeRequirementData>,
    effects: Vec<EffectCallData>,
    success_events: Vec<EventEmissionData>,
}

impl ActionData {
    /// Creates action input data without claiming cross-object validity.
    #[must_use]
    pub fn new(
        name: LocalDefinitionName,
        bindings: Vec<ActionBindingData>,
        requirements: Vec<RuntimeRequirementData>,
        effects: Vec<EffectCallData>,
        success_events: Vec<EventEmissionData>,
    ) -> Self {
        Self {
            name,
            bindings,
            requirements,
            effects,
            success_events,
        }
    }

    /// Returns the action's local definition name.
    #[must_use]
    pub const fn name(&self) -> &LocalDefinitionName {
        &self.name
    }

    /// Returns binding declarations in their declared order.
    #[must_use]
    pub fn bindings(&self) -> &[ActionBindingData] {
        &self.bindings
    }

    /// Returns runtime requirements in their declared order.
    #[must_use]
    pub fn requirements(&self) -> &[RuntimeRequirementData] {
        &self.requirements
    }

    /// Returns effects in execution order.
    #[must_use]
    pub fn effects(&self) -> &[EffectCallData] {
        &self.effects
    }

    /// Returns success-event declarations in emission order.
    #[must_use]
    pub fn success_events(&self) -> &[EventEmissionData] {
        &self.success_events
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        LocalDefinitionName,
        Vec<ActionBindingData>,
        Vec<RuntimeRequirementData>,
        Vec<EffectCallData>,
        Vec<EventEmissionData>,
    ) {
        (
            self.name,
            self.bindings,
            self.requirements,
            self.effects,
            self.success_events,
        )
    }
}

/// Input data for one physical event definition.
///
/// The enclosing artifact validator checks field uniqueness and definition
/// namespace rules.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventData {
    name: LocalDefinitionName,
    fields: Vec<EventFieldData>,
}

impl EventData {
    /// Creates event input data without claiming event-level validity.
    #[must_use]
    pub fn new(name: LocalDefinitionName, fields: Vec<EventFieldData>) -> Self {
        Self { name, fields }
    }

    /// Returns the event's local definition name.
    #[must_use]
    pub const fn name(&self) -> &LocalDefinitionName {
        &self.name
    }

    /// Returns fields in their declared order.
    #[must_use]
    pub fn fields(&self) -> &[EventFieldData] {
        &self.fields
    }

    pub(crate) fn into_parts(self) -> (LocalDefinitionName, Vec<EventFieldData>) {
        (self.name, self.fields)
    }
}
