use world_defs::{EffectArgValue, EffectOp, EffectParamName, EffectPrimitiveDef, RoleName};

use crate::RuntimeError;

/// Resolved view of one primitive operation and the definition it invokes.
#[derive(Clone, Copy, Debug)]
pub struct PrimitiveInvocation<'a> {
    operation: &'a EffectOp,
    primitive: &'a EffectPrimitiveDef,
}

impl<'a> PrimitiveInvocation<'a> {
    pub(crate) fn new(operation: &'a EffectOp, primitive: &'a EffectPrimitiveDef) -> Self {
        Self {
            operation,
            primitive,
        }
    }

    /// Returns the operation call site.
    pub const fn operation(self) -> &'a EffectOp {
        self.operation
    }

    /// Returns the resolved primitive definition.
    pub const fn primitive(self) -> &'a EffectPrimitiveDef {
        self.primitive
    }

    /// Returns the required role argument for a primitive parameter.
    pub fn required_role(&self, param: &EffectParamName) -> Result<RoleName, RuntimeError> {
        self.role_binding(param)?
            .ok_or_else(|| RuntimeError::MissingPrimitiveArgument {
                primitive: self.primitive.id(),
                param: param.clone(),
            })
    }

    /// Returns the optional role argument for a primitive parameter.
    pub fn optional_role(&self, param: &EffectParamName) -> Result<Option<RoleName>, RuntimeError> {
        self.role_binding(param)
    }

    fn role_binding(&self, param: &EffectParamName) -> Result<Option<RoleName>, RuntimeError> {
        let Some(arg) = self.operation.arg(param) else {
            return Ok(None);
        };

        match arg.value() {
            EffectArgValue::Role(role) => Ok(Some(role.clone())),
            _ => Err(RuntimeError::UnsupportedPrimitiveArgument {
                primitive: self.primitive.id(),
                param: param.clone(),
            }),
        }
    }
}
