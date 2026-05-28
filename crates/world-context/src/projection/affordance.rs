use world_core::{DefinitionId, EntityId};

use crate::{
    ContextDiagnostic, ContextProjectionCompleteness, ContextProjectionKind, ContextProvenance,
    context::ContextProjectionReportBuilder, request::ActorContextRequest,
};

/// Actor-visible target or context affordance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerceivedAffordance {
    definition: Option<DefinitionId>,
    target: Option<EntityId>,
    status: AffordanceStatus,
    provenance: ContextProvenance,
}

impl PerceivedAffordance {
    /// Returns the checked affordance definition, if known.
    #[must_use]
    pub const fn definition(&self) -> Option<DefinitionId> {
        self.definition
    }

    /// Returns the perceived target, if this is target-specific.
    #[must_use]
    pub const fn target(&self) -> Option<EntityId> {
        self.target
    }

    /// Returns affordance evidence status.
    #[must_use]
    pub const fn status(&self) -> AffordanceStatus {
        self.status
    }

    /// Returns provenance anchors for this affordance.
    #[must_use]
    pub const fn provenance(&self) -> &ContextProvenance {
        &self.provenance
    }
}

/// Evidence quality for a perceived affordance.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AffordanceStatus {
    /// Directly observed by actor-visible perception.
    Observed,
    /// Inferred from actor-accessible context.
    Inferred,
    /// Suspected from incomplete actor-accessible context.
    Suspected,
}

pub(crate) fn derive(
    request: &ActorContextRequest,
    report: &mut ContextProjectionReportBuilder,
) -> Vec<PerceivedAffordance> {
    report.push_status(
        ContextProjectionKind::Affordance,
        ContextProjectionCompleteness::Unavailable,
    );

    if request.options().include_debug_diagnostics() {
        report.push_diagnostic(ContextDiagnostic::ProjectionUnavailable {
            projection: ContextProjectionKind::Affordance,
        });
    }

    Vec::new()
}
