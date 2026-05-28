use world_core::{DefinitionId, EntityId, EventRecordId, ProvenanceKey, SimulationTime};

use crate::{
    ContextDiagnostic, ContextProjectionCompleteness, ContextProjectionKind,
    context::ContextProjectionReportBuilder, request::ActorContextRequest,
};

/// Actor-visible observation context.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObservationContext {
    states: Vec<ObservedState>,
    events: Vec<ObservedEvent>,
}

impl ObservationContext {
    /// Creates an empty observation context.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            states: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Returns whether no observations were projected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.states.is_empty() && self.events.is_empty()
    }

    /// Returns observed state records.
    #[must_use]
    pub fn states(&self) -> &[ObservedState] {
        &self.states
    }

    /// Returns observed event records.
    #[must_use]
    pub fn events(&self) -> &[ObservedEvent] {
        &self.events
    }
}

/// Actor-visible state observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedState {
    subject: EntityId,
    definition: Option<DefinitionId>,
    provenance: Option<ProvenanceKey>,
}

impl ObservedState {
    /// Returns the observed entity.
    #[must_use]
    pub const fn subject(&self) -> EntityId {
        self.subject
    }

    /// Returns the definition associated with this observation, if known.
    #[must_use]
    pub const fn definition(&self) -> Option<DefinitionId> {
        self.definition
    }

    /// Returns provenance associated with this observation, if known.
    #[must_use]
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }
}

/// Actor-visible event observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedEvent {
    source_event: EventRecordId,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}

impl ObservedEvent {
    /// Returns the committed event id that backs this observation.
    #[must_use]
    pub const fn source_event(&self) -> EventRecordId {
        self.source_event
    }

    /// Returns the simulation time of the observed event.
    #[must_use]
    pub const fn occurred_at(&self) -> SimulationTime {
        self.occurred_at
    }

    /// Returns provenance associated with this event, if known.
    #[must_use]
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }
}

pub(crate) fn project(
    request: &ActorContextRequest,
    report: &mut ContextProjectionReportBuilder,
) -> ObservationContext {
    report.push_status(
        ContextProjectionKind::Observation,
        ContextProjectionCompleteness::Unavailable,
    );

    if request.options().include_debug_diagnostics() {
        report.push_diagnostic(ContextDiagnostic::ProjectionUnavailable {
            projection: ContextProjectionKind::Observation,
        });
    }

    ObservationContext::empty()
}
