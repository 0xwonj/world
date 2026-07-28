use core::fmt;
use std::sync::Arc;

use world_runtime::{AttemptKey, RuntimeService, RuntimeStartError};

use crate::artifact::ArtifactResolver;
use crate::attempt::RunAttempt;
use crate::distribution::EngineDistribution;
use crate::resolution::{
    ExecutionSpecInput, ResolveExecutionError, ResolvedExecution, ResolvedExecutionInner,
    map_activation_error, resolve_definitions,
};
use crate::routing::PostCommitRouter;

/// Checked construction inputs for the public engine facade.
pub struct EngineBuilder {
    distribution: EngineDistribution,
    artifacts: Arc<dyn ArtifactResolver>,
    runtime: RuntimeService,
}

impl EngineBuilder {
    /// Binds trusted semantics, artifact reads, and one runtime authority domain.
    #[must_use]
    pub fn new(
        distribution: EngineDistribution,
        artifacts: Arc<dyn ArtifactResolver>,
        runtime: RuntimeService,
    ) -> Self {
        Self {
            distribution,
            artifacts,
            runtime,
        }
    }

    /// Constructs an engine only when its semantic installation is usable.
    pub fn build(self) -> Result<Engine, EngineBuildError> {
        if self.distribution.is_empty() {
            return Err(EngineBuildError::NoSemanticImplementations);
        }
        Ok(Engine {
            inner: Arc::new(EngineInner {
                distribution: self.distribution,
                artifacts: self.artifacts,
                runtime: self.runtime,
                binding: Arc::new(EngineBinding),
            }),
        })
    }
}

/// Why an engine facade could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineBuildError {
    /// No executable interaction exists without a trusted semantic family.
    NoSemanticImplementations,
}

impl fmt::Display for EngineBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSemanticImplementations => {
                formatter.write_str("engine distribution installs no semantic implementation")
            }
        }
    }
}

impl std::error::Error for EngineBuildError {}

/// Cloneable composition facade over immutable installation and runtime services.
#[derive(Clone)]
pub struct Engine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    distribution: EngineDistribution,
    artifacts: Arc<dyn ArtifactResolver>,
    runtime: RuntimeService,
    binding: Arc<EngineBinding>,
}

pub(crate) struct EngineBinding;

impl Engine {
    /// Resolves exact stored artifacts and asks runtime to mint one activation.
    pub fn resolve_execution(
        &self,
        input: ExecutionSpecInput,
    ) -> Result<ResolvedExecution, ResolveExecutionError> {
        let (lock, lifecycle_profiles, origin) = input.into_parts();
        let lifecycle = self
            .inner
            .distribution
            .resolve_lifecycle(lifecycle_profiles)
            .map_err(ResolveExecutionError::Lifecycle)?;
        let definitions = resolve_definitions(
            self.inner.artifacts.as_ref(),
            self.inner.distribution.semantic_interfaces(),
            lock,
        )?;
        let mut transfer = None;
        let mut relocation = None;
        for required in definitions.required_interfaces() {
            if let Some(implementation) = self.inner.distribution.transfer_implementation(required)
            {
                if transfer.replace(implementation.clone()).is_some() {
                    return Err(ResolveExecutionError::MultipleContainmentInterfaces);
                }
                continue;
            }
            if let Some(implementation) =
                self.inner.distribution.relocation_implementation(required)
            {
                if relocation.replace(implementation.clone()).is_some() {
                    return Err(ResolveExecutionError::MultipleRelocationInterfaces);
                }
                continue;
            }
            unreachable!(
                "validated artifacts can require only implementations in the distribution catalog"
            );
        }
        let activation = self
            .inner
            .runtime
            .activate_origin(
                definitions.clone(),
                transfer,
                relocation,
                lifecycle_profiles,
                origin,
            )
            .map_err(|error| ResolveExecutionError::Activation(map_activation_error(error)))?;
        let containment_actions = activation
            .containment_transfer_actions()
            .map(|family| {
                world_context::ContainmentTransferActionDefinitions::new(
                    &definitions,
                    family.actions().to_vec(),
                )
            })
            .transpose()
            .map_err(|_| {
                ResolveExecutionError::Activation(
                    crate::resolution::ExecutionActivationError::ContainmentActionBindingMismatch,
                )
            })?;
        let relocation_actions = activation
            .relocation_action_family()
            .map(|family| {
                world_context::RelocationActionDefinitions::new(
                    &definitions,
                    family.start().clone(),
                    family.pause().clone(),
                    family.resume().clone(),
                )
            })
            .transpose()
            .map_err(|_| {
                ResolveExecutionError::Activation(
                    crate::resolution::ExecutionActivationError::RelocationActionBindingMismatch,
                )
            })?;
        let post_commit_router = PostCommitRouter::new(activation.post_commit_routing_policy());

        Ok(ResolvedExecution {
            inner: Arc::new(ResolvedExecutionInner {
                engine: Arc::clone(&self.inner.binding),
                activation,
                post_commit_router,
                definitions,
                containment_actions,
                relocation_actions,
                lifecycle,
            }),
        })
    }

    /// Starts one non-cloneable attempt bound to an execution from this engine.
    pub fn start_attempt(
        &self,
        execution: &ResolvedExecution,
        key: AttemptKey,
    ) -> Result<RunAttempt, StartAttemptError> {
        if !Arc::ptr_eq(&self.inner.binding, &execution.inner.engine) {
            return Err(StartAttemptError::EngineMismatch);
        }
        let driver = self
            .inner
            .runtime
            .start_attempt(&execution.inner.activation, key)
            .map_err(map_start_error)?;
        Ok(RunAttempt::new(driver, Arc::clone(&execution.inner)))
    }
}

/// Why a sealed execution could not become a driven attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartAttemptError {
    /// The execution was resolved by another engine composition.
    EngineMismatch,
    /// The process exhausted its in-memory authority-domain identities.
    AuthorityDomainExhausted,
    /// The same attempt identity was reopened with another creation value.
    AttemptCreationConflict,
    /// No attempt with the requested identity exists.
    AttemptNotFound,
    /// Retained authority and control state violate the engine protocol.
    Integrity,
    /// The authority service could not be accessed.
    Unavailable,
}

impl fmt::Display for StartAttemptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EngineMismatch => {
                formatter.write_str("resolved execution belongs to another engine")
            }
            other => write!(formatter, "engine could not start attempt: {other:?}"),
        }
    }
}

impl std::error::Error for StartAttemptError {}

fn map_start_error(error: RuntimeStartError) -> StartAttemptError {
    match error {
        RuntimeStartError::AuthorityDomainExhausted => StartAttemptError::AuthorityDomainExhausted,
        RuntimeStartError::AttemptCreationConflict => StartAttemptError::AttemptCreationConflict,
        RuntimeStartError::AttemptNotFound => StartAttemptError::AttemptNotFound,
        RuntimeStartError::Integrity => StartAttemptError::Integrity,
        RuntimeStartError::Unavailable => StartAttemptError::Unavailable,
    }
}
