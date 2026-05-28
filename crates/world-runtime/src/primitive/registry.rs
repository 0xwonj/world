use std::collections::BTreeMap;

use world_defs::{DefinitionRegistry, EffectPrimitiveId};

use crate::{
    RuntimeError,
    primitive::{PrimitiveSemantics, PrimitiveSemanticsInstaller},
};

/// Mutable builder for trusted primitive semantics handlers.
#[derive(Default)]
pub struct PrimitiveSemanticsRegistryBuilder {
    handlers: BTreeMap<EffectPrimitiveId, Box<dyn PrimitiveSemantics>>,
}

impl PrimitiveSemanticsRegistryBuilder {
    /// Creates an empty semantics registry builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Installs all handlers from a trusted semantics installer.
    pub fn install<I>(&mut self, installer: &I) -> Result<&mut Self, RuntimeError>
    where
        I: PrimitiveSemanticsInstaller + ?Sized,
    {
        installer.install_semantics(self)?;
        Ok(self)
    }

    /// Adds one trusted primitive handler.
    pub fn add_handler<H>(&mut self, handler: H) -> Result<&mut Self, RuntimeError>
    where
        H: PrimitiveSemantics,
    {
        self.add_boxed_handler(Box::new(handler))
    }

    /// Adds one boxed trusted primitive handler.
    pub fn add_boxed_handler(
        &mut self,
        handler: Box<dyn PrimitiveSemantics>,
    ) -> Result<&mut Self, RuntimeError> {
        let primitive = handler.primitive();
        if self.handlers.contains_key(&primitive) {
            return Err(RuntimeError::DuplicatePrimitiveSemantics { primitive });
        }
        self.handlers.insert(primitive, handler);
        Ok(self)
    }

    /// Builds an immutable registry after checking it against definitions.
    pub fn build_against(
        self,
        definitions: &DefinitionRegistry,
    ) -> Result<PrimitiveSemanticsRegistry, RuntimeError> {
        let registry = PrimitiveSemanticsRegistry {
            handlers: self.handlers,
        };
        registry.validate_against(definitions)?;
        Ok(registry)
    }
}

/// Immutable lookup table for trusted primitive semantics.
pub struct PrimitiveSemanticsRegistry {
    handlers: BTreeMap<EffectPrimitiveId, Box<dyn PrimitiveSemantics>>,
}

impl PrimitiveSemanticsRegistry {
    /// Returns an empty semantics registry checked against definitions.
    pub fn empty_checked(definitions: &DefinitionRegistry) -> Result<Self, RuntimeError> {
        PrimitiveSemanticsRegistryBuilder::new().build_against(definitions)
    }

    /// Looks up a primitive handler.
    pub fn handler(&self, primitive: EffectPrimitiveId) -> Option<&dyn PrimitiveSemantics> {
        self.handlers.get(&primitive).map(Box::as_ref)
    }

    pub(crate) fn validate_against(
        &self,
        definitions: &DefinitionRegistry,
    ) -> Result<(), RuntimeError> {
        for (primitive, handler) in &self.handlers {
            let Some(definition) = definitions.effect_primitive(*primitive) else {
                return Err(RuntimeError::PrimitiveSemanticsForUnknownDefinition {
                    primitive: *primitive,
                });
            };
            if let Err(field) = handler.contract().matches_definition(definition) {
                return Err(RuntimeError::PrimitiveSemanticsContractMismatch {
                    primitive: *primitive,
                    field,
                });
            }
        }

        // Action programs are interpreted by this registry. Process programs are
        // checked as definitions and executed by a separate process runtime path.
        for action in definitions.actions() {
            let Some(program) = definitions.effect_program(action.effect_program()) else {
                continue;
            };
            for operation in program.operations() {
                if !self.handlers.contains_key(&operation.primitive()) {
                    return Err(RuntimeError::MissingPrimitiveSemantics {
                        primitive: operation.primitive(),
                    });
                }
            }
        }

        Ok(())
    }
}
