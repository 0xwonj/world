use crate::keys::{EffectParamName, RoleName};

/// Parameter accepted by a checked primitive effect.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EffectParamDef {
    name: EffectParamName,
    kind: EffectParamKind,
}

impl EffectParamDef {
    /// Creates a primitive parameter declaration.
    pub fn new(name: EffectParamName, kind: EffectParamKind) -> Self {
        Self { name, kind }
    }

    /// Returns the parameter name.
    pub fn name(&self) -> &EffectParamName {
        &self.name
    }

    /// Returns the parameter kind.
    pub const fn kind(&self) -> EffectParamKind {
        self.kind
    }
}

/// Kind of value a primitive parameter accepts.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EffectParamKind {
    /// Required entity role binding.
    EntityRole,
    /// Optional entity role binding.
    OptionalEntityRole,
}

impl EffectParamKind {
    /// Returns true when the argument kind can satisfy this parameter.
    pub const fn accepts(self, arg: EffectArgKind) -> bool {
        matches!(
            (self, arg),
            (
                Self::EntityRole | Self::OptionalEntityRole,
                EffectArgKind::Role
            )
        )
    }

    /// Returns true when the parameter must be bound by each operation.
    pub const fn is_required(self) -> bool {
        matches!(self, Self::EntityRole)
    }
}

/// Binding from one primitive parameter to one effect-program argument.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EffectArgBinding {
    param: EffectParamName,
    value: EffectArgValue,
}

impl EffectArgBinding {
    /// Creates an argument binding.
    pub fn new(param: EffectParamName, value: EffectArgValue) -> Self {
        Self { param, value }
    }

    /// Creates a binding from a primitive parameter to an action or process role.
    pub fn role(param: EffectParamName, role: RoleName) -> Self {
        Self::new(param, EffectArgValue::Role(role))
    }

    /// Returns the parameter being bound.
    pub fn param(&self) -> &EffectParamName {
        &self.param
    }

    /// Returns the argument value.
    pub fn value(&self) -> &EffectArgValue {
        &self.value
    }
}

/// Runtime-independent argument value carried by an effect operation.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EffectArgValue {
    /// Use the entity bound to the named action or process role.
    Role(RoleName),
}

impl EffectArgValue {
    /// Returns the value shape used for parameter signature checks.
    pub const fn kind(&self) -> EffectArgKind {
        match self {
            Self::Role(_) => EffectArgKind::Role,
        }
    }

    /// Returns the role name when this value references a role binding.
    pub fn role(&self) -> Option<&RoleName> {
        match self {
            Self::Role(role) => Some(role),
        }
    }
}

/// Coarse shape of an effect operation argument.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EffectArgKind {
    /// A role reference.
    Role,
}
