//! Foundational domain vocabulary shared by the simulation workspace.

mod authority;
mod error;
mod ids;
mod ordering;
mod time;

pub use authority::{AuthorityClass, ReplayLevel};
pub use error::InvalidCoreValue;
pub use ids::{
    ActivityId, ActivityIdIssuer, ActorId, CausalTransactionId, CausalTransactionIdIssuer,
    DefinitionId, EntityId, EventRecordId, EventRecordIdIssuer, ProcessInstanceId,
    ProcessInstanceIdIssuer, ProvenanceKey, RuntimeEntityHandle, RuntimeEntityHandleIssuer,
    VersionAnchor,
};
pub use ordering::{QueryEpoch, StoreCursor, WakeupOrderKey};
pub use time::{SimulationDuration, SimulationTime};

#[cfg(test)]
mod tests;
