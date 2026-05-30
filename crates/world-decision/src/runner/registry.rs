use std::collections::BTreeMap;

use world_core::DefinitionId;

use crate::{DecisionError, DecisionPassExecutor, DecisionRegistry, ImplementationMode};

/// In-memory registry of trusted decision pass executors.
#[derive(Default)]
pub struct DecisionExecutorRegistry {
    executors: BTreeMap<(DefinitionId, ImplementationMode), Box<dyn DecisionPassExecutor>>,
}

impl DecisionExecutorRegistry {
    /// Creates an empty executor registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an executor for its `(pass_id, mode)` pair.
    pub fn add_executor(
        &mut self,
        executor: Box<dyn DecisionPassExecutor>,
    ) -> Result<&mut Self, DecisionError> {
        let key = (executor.pass_id(), executor.mode());
        if self.executors.contains_key(&key) {
            return Err(DecisionError::DuplicateExecutor {
                pass: key.0,
                mode: key.1,
            });
        }
        self.executors.insert(key, executor);
        Ok(self)
    }

    /// Looks up an executor.
    #[must_use]
    pub fn executor(
        &self,
        pass: DefinitionId,
        mode: ImplementationMode,
    ) -> Option<&dyn DecisionPassExecutor> {
        self.executors.get(&(pass, mode)).map(Box::as_ref)
    }

    /// Validates registered executors against a checked decision registry.
    pub fn validate_against(&self, registry: &DecisionRegistry) -> Result<(), DecisionError> {
        for (pass, mode) in self.executors.keys().copied() {
            let Some(contract) = registry.pass(pass) else {
                return Err(DecisionError::MissingExecutorPass { pass });
            };
            if !contract.supports_mode(mode) {
                return Err(DecisionError::ExecutorModeUnsupported { pass, mode });
            }
        }

        Ok(())
    }
}
