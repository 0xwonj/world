use world_core::ActorId;

use crate::{
    ActionRepertoire, CapabilitySet, ContextDiagnostic, ContextProjectionCompleteness,
    ContextProjectionKind, ContextProjectionStatus, ContextProvenance, ContextProvenanceSource,
    ContextReadDependency, ContextReadSet, EpistemicWorkingSet, ObservationContext,
    PerceivedAffordance, SocialContextView,
};

/// Actor-relative, decision-safe context snapshot.
///
/// This type owns projected values. It does not borrow the model, expose kernel
/// query surfaces, carry runtime staging authority, or choose a final intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorContext {
    actor: ActorId,
    observations: ObservationContext,
    epistemic: EpistemicWorkingSet,
    social: SocialContextView,
    capabilities: CapabilitySet,
    repertoire: ActionRepertoire,
    affordances: Vec<PerceivedAffordance>,
}

impl ActorContext {
    pub(crate) fn new(
        actor: ActorId,
        observations: ObservationContext,
        epistemic: EpistemicWorkingSet,
        social: SocialContextView,
        capabilities: CapabilitySet,
        repertoire: ActionRepertoire,
        affordances: Vec<PerceivedAffordance>,
    ) -> Self {
        Self {
            actor,
            observations,
            epistemic,
            social,
            capabilities,
            repertoire,
            affordances,
        }
    }

    /// Returns the actor scope for this context.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns observed state and event context.
    #[must_use]
    pub const fn observations(&self) -> &ObservationContext {
        &self.observations
    }

    /// Returns actor-owned epistemic context.
    #[must_use]
    pub const fn epistemic(&self) -> &EpistemicWorkingSet {
        &self.epistemic
    }

    /// Returns actor-relative social context.
    #[must_use]
    pub const fn social(&self) -> &SocialContextView {
        &self.social
    }

    /// Returns capability evidence projected for the actor.
    #[must_use]
    pub const fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    /// Returns schema-level actor action candidates.
    #[must_use]
    pub const fn repertoire(&self) -> &ActionRepertoire {
        &self.repertoire
    }

    /// Returns perceived target/context affordances.
    #[must_use]
    pub fn affordances(&self) -> &[PerceivedAffordance] {
        &self.affordances
    }
}

/// Context plus projection metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorContextProjection {
    context: ActorContext,
    report: ContextProjectionReport,
}

impl ActorContextProjection {
    pub(crate) fn new(context: ActorContext, report: ContextProjectionReport) -> Self {
        Self { context, report }
    }

    /// Returns the projected actor context.
    #[must_use]
    pub const fn context(&self) -> &ActorContext {
        &self.context
    }

    /// Returns projection metadata.
    #[must_use]
    pub const fn report(&self) -> &ContextProjectionReport {
        &self.report
    }

    /// Splits the projection into owned context and report values.
    #[must_use]
    pub fn into_parts(self) -> (ActorContext, ContextProjectionReport) {
        (self.context, self.report)
    }
}

/// Read dependencies, provenance, and diagnostics emitted by context projection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextProjectionReport {
    reads: ContextReadSet,
    provenance: ContextProvenance,
    statuses: Vec<ContextProjectionStatus>,
    diagnostics: Vec<ContextDiagnostic>,
}

impl ContextProjectionReport {
    pub(crate) fn new(
        reads: ContextReadSet,
        provenance: ContextProvenance,
        statuses: Vec<ContextProjectionStatus>,
        diagnostics: Vec<ContextDiagnostic>,
    ) -> Self {
        Self {
            reads,
            provenance,
            statuses,
            diagnostics,
        }
    }

    /// Returns model and definition reads used by the projection.
    #[must_use]
    pub const fn reads(&self) -> &ContextReadSet {
        &self.reads
    }

    /// Returns coarse provenance anchors for the projection.
    #[must_use]
    pub const fn provenance(&self) -> &ContextProvenance {
        &self.provenance
    }

    /// Returns projection completeness records in pipeline order.
    #[must_use]
    pub fn statuses(&self) -> &[ContextProjectionStatus] {
        &self.statuses
    }

    /// Returns the status for one projection family.
    #[must_use]
    pub fn status(&self, projection: ContextProjectionKind) -> Option<ContextProjectionStatus> {
        self.statuses
            .iter()
            .copied()
            .find(|status| status.projection() == projection)
    }

    /// Returns structured projection diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[ContextDiagnostic] {
        &self.diagnostics
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ContextProjectionReportBuilder {
    reads: ContextReadSet,
    provenance: ContextProvenance,
    statuses: Vec<ContextProjectionStatus>,
    diagnostics: Vec<ContextDiagnostic>,
}

impl ContextProjectionReportBuilder {
    pub(crate) fn insert_read(&mut self, dependency: ContextReadDependency) {
        self.reads.insert(dependency);
    }

    pub(crate) fn insert_provenance(&mut self, source: ContextProvenanceSource) {
        self.provenance.push(source);
    }

    pub(crate) fn push_diagnostic(&mut self, diagnostic: ContextDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub(crate) fn push_status(
        &mut self,
        projection: ContextProjectionKind,
        completeness: ContextProjectionCompleteness,
    ) {
        self.statuses
            .push(ContextProjectionStatus::new(projection, completeness));
    }

    pub(crate) fn finish(self) -> ContextProjectionReport {
        ContextProjectionReport::new(self.reads, self.provenance, self.statuses, self.diagnostics)
    }
}
