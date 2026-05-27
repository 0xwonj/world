/// Authority layer for gameplay-relevant state.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthorityClass {
    /// Physical and causal truth committed through causal runtime.
    Hard,
    /// Durable runtime control state updated through runtime-control authority gates.
    RuntimeControl,
    /// Social or institutional soft truth committed through a social gate.
    Social,
    /// Authored, generated, or accepted world-context chronology.
    Chronology,
    /// Holder-relative perception, memory, belief, or knowledge.
    ActorTruth,
    /// Accepted appraisal and motivation state.
    Appraisal,
}

/// Runtime source class preserved on causal transaction records.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CausalSource {
    /// Player-facing command source.
    Player,
    /// Actor policy source.
    ActorPolicy,
    /// Engine-owned source.
    Engine,
    /// Tooling or test harness source.
    Tooling,
}

/// Declared replay strength for committed runtime records.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReplayLevel {
    /// Inspect committed records, ordering, validation context, and provenance.
    AuditOnly,
    /// Rebuild consequences from committed transaction and event history.
    EventRebuild,
    /// Rerun accepted input logs and expect matching transactions and events.
    DeterministicCommandReplay,
}
