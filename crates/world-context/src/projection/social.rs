use crate::{
    ContextDiagnostic, ContextProjectionCompleteness, ContextProjectionKind,
    context::ContextProjectionReportBuilder, request::ActorContextRequest,
};

/// Actor-relative social context view.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SocialContextView {
    records: Vec<SocialContextRecord>,
}

impl SocialContextView {
    /// Creates an empty social context view.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Returns whether no shallow social context was projected.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns social context records visible to this actor.
    #[must_use]
    pub fn records(&self) -> &[SocialContextRecord] {
        &self.records
    }
}

/// Minimal actor-relative social context record placeholder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocialContextRecord;

pub(crate) fn project(
    request: &ActorContextRequest,
    report: &mut ContextProjectionReportBuilder,
) -> SocialContextView {
    report.push_status(
        ContextProjectionKind::Social,
        ContextProjectionCompleteness::Unavailable,
    );

    if request.options().include_debug_diagnostics() {
        report.push_diagnostic(ContextDiagnostic::ProjectionUnavailable {
            projection: ContextProjectionKind::Social,
        });
    }

    SocialContextView::empty()
}
