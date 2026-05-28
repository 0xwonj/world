use crate::{
    ActorContext, ActorContextInput, ActorContextProjection, ActorContextRequest, ContextError,
    ContextProvenanceSource,
    context::ContextProjectionReportBuilder,
    projection::{affordance, capability, epistemic, observation, repertoire, social},
};

/// Concrete entry point for actor-relative context projection.
///
/// The pipeline reads checked definitions and model query surfaces, then
/// returns an owned context snapshot plus projection metadata. It does not
/// mutate the model, stage runtime effects, install primitive semantics, or
/// choose an intent.
#[derive(Clone, Copy, Debug, Default)]
pub struct ActorContextPipeline;

impl ActorContextPipeline {
    /// Creates a context projection pipeline.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Projects an actor-relative context snapshot.
    pub fn project(
        &self,
        input: ActorContextInput<'_>,
        request: ActorContextRequest,
    ) -> Result<ActorContextProjection, ContextError> {
        let mut report = ContextProjectionReportBuilder::default();
        report.insert_provenance(ContextProvenanceSource::ActorScope(request.actor()));

        let observations = observation::project(&request, &mut report);
        let epistemic = epistemic::project(input, &request, &mut report);
        let social = social::project(&request, &mut report);
        let capabilities = capability::derive(&mut report);
        let repertoire = repertoire::derive(input.definitions(), &mut report);
        let affordances = affordance::derive(&request, &mut report);

        let context = ActorContext::new(
            request.actor(),
            observations,
            epistemic,
            social,
            capabilities,
            repertoire,
            affordances,
        );

        Ok(ActorContextProjection::new(context, report.finish()))
    }
}
