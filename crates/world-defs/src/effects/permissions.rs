use std::collections::BTreeSet;

/// Coarse authority marker a typed effect operation needs from the causal runtime stage.
///
/// Fine-grained read/write/resource effects and event-channel facets are intentionally
/// left to future primitive families instead of being partially modeled here.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StagePermission {
    /// Read committed world state.
    ReadWorld,
    /// Read actor-owned or holder-relative state.
    ReadActorOwnedState,
    /// Read a derived engine view.
    ReadDerivedEngineView,
    /// Read a submitted role binding.
    ReadSubmittedBinding,
    /// Run validation without staging mutation.
    Validate,
    /// Acquire a runtime reservation.
    AcquireReservation,
    /// Release a runtime reservation.
    ReleaseReservation,
    /// Draw from an engine-owned random stream.
    Rng,
    /// Stage a hard physical mutation.
    MutatePhysical,
    /// Stage process progress or process lifecycle mutation.
    MutateProcess,
    /// Emit a hard physical event record.
    EmitPhysicalEventRecord,
    /// Emit a sensory event record.
    EmitSensoryEventRecord,
    /// Schedule durable process work.
    ScheduleProcess,
    /// Schedule a reaction request.
    ScheduleReaction,
}

pub(super) fn permissions_require_event(permissions: &BTreeSet<StagePermission>) -> bool {
    permissions.iter().any(|permission| {
        matches!(
            permission,
            StagePermission::MutatePhysical
                | StagePermission::EmitPhysicalEventRecord
                | StagePermission::EmitSensoryEventRecord
        )
    })
}

pub(super) fn permissions_allow_event_emission(permissions: &BTreeSet<StagePermission>) -> bool {
    permissions.iter().any(|permission| {
        matches!(
            permission,
            StagePermission::EmitPhysicalEventRecord | StagePermission::EmitSensoryEventRecord
        )
    })
}
