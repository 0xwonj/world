//! Actor-relative observation and context projection crate.

mod context;
mod error;
mod evidence;
mod pipeline;
mod projection;
mod request;

pub use context::{ActorContext, ActorContextProjection, ContextProjectionReport};
pub use error::ContextError;
pub use evidence::{
    ContextDiagnostic, ContextProjectionCompleteness, ContextProjectionKind,
    ContextProjectionStatus, ContextProvenance, ContextProvenanceSource, ContextReadDependency,
    ContextReadSet,
};
pub use pipeline::ActorContextPipeline;
pub use projection::{
    ActionRepertoire, ActionRepertoireEntry, AffordanceStatus, CapabilityEntry, CapabilityKind,
    CapabilitySet, CapabilityStatus, EpistemicContextRecord, EpistemicWorkingSet,
    ObservationContext, ObservedEvent, ObservedState, PerceivedAffordance, RepertoireStatus,
    RoleProjection, SocialContextRecord, SocialContextView,
};
pub use request::{ActorContextInput, ActorContextOptions, ActorContextRequest};

#[cfg(test)]
mod tests;
