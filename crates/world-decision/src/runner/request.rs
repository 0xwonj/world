use world_context::ActorContextProjection;
use world_core::DefinitionId;

/// Request to execute a checked decision profile.
#[derive(Clone, Copy, Debug)]
pub struct DecisionRunRequest<'a> {
    profile: DefinitionId,
    actor_context: &'a ActorContextProjection,
}

impl<'a> DecisionRunRequest<'a> {
    /// Creates a decision run request.
    #[must_use]
    pub const fn new(profile: DefinitionId, actor_context: &'a ActorContextProjection) -> Self {
        Self {
            profile,
            actor_context,
        }
    }

    /// Returns profile id.
    #[must_use]
    pub const fn profile(self) -> DefinitionId {
        self.profile
    }

    /// Returns actor context projection.
    #[must_use]
    pub const fn actor_context(self) -> &'a ActorContextProjection {
        self.actor_context
    }
}
