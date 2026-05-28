use world_core::ActorId;
use world_defs::DefinitionRegistry;
use world_model::WorldModel;

/// Read-only input used to project an actor-relative context.
#[derive(Clone, Copy, Debug)]
pub struct ActorContextInput<'a> {
    model: &'a WorldModel,
    definitions: &'a DefinitionRegistry,
}

impl<'a> ActorContextInput<'a> {
    /// Creates read-only context projection input.
    #[must_use]
    pub const fn new(model: &'a WorldModel, definitions: &'a DefinitionRegistry) -> Self {
        Self { model, definitions }
    }

    /// Returns the authoritative model read source.
    #[must_use]
    pub(crate) const fn model(self) -> &'a WorldModel {
        self.model
    }

    /// Returns the checked definition registry.
    #[must_use]
    pub(crate) const fn definitions(self) -> &'a DefinitionRegistry {
        self.definitions
    }
}

/// Actor-relative context projection request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorContextRequest {
    actor: ActorId,
    options: ActorContextOptions,
}

impl ActorContextRequest {
    /// Creates a request with default projection options.
    #[must_use]
    pub const fn new(actor: ActorId) -> Self {
        Self {
            actor,
            options: ActorContextOptions::new(),
        }
    }

    /// Creates a request with explicit projection options.
    #[must_use]
    pub const fn with_options(actor: ActorId, options: ActorContextOptions) -> Self {
        Self { actor, options }
    }

    /// Returns the actor whose context is being projected.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns projection options.
    #[must_use]
    pub const fn options(&self) -> &ActorContextOptions {
        &self.options
    }
}

/// Options for actor-context projection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActorContextOptions {
    include_debug_diagnostics: bool,
}

impl ActorContextOptions {
    /// Creates default context projection options.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            include_debug_diagnostics: false,
        }
    }

    /// Enables or disables conservative debug diagnostics for shallow projections.
    #[must_use]
    pub const fn with_debug_diagnostics(mut self, include: bool) -> Self {
        self.include_debug_diagnostics = include;
        self
    }

    /// Returns whether shallow projection diagnostics should be included.
    #[must_use]
    pub const fn include_debug_diagnostics(self) -> bool {
        self.include_debug_diagnostics
    }
}
