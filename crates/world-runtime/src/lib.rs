//! Causal runtime, transaction, scheduler, process, and runtime-control crate.

mod control;
mod error;
mod outcome;
mod primitive;
mod process;
mod request;
mod runtime;
mod scheduler;
mod transaction;

pub use control::{AcquireReservationRequest, WakeupScheduleKey};
pub use error::RuntimeError;
pub use outcome::{
    BlockedOutcome, CommittedOutcome, RejectedOutcome, RejectionReason, RuntimeOutcome,
};
pub use process::{ProcessControlOutcome, ProcessTransition, StartProcessRequest};
pub use request::{RequestSource, RuntimeRequest, SubmittedRole};
pub use runtime::CausalRuntime;
pub use scheduler::{
    DrainBudget, DrainOutcome, DrainReport, DrainRequest, ProcessedWakeup, ScheduleWakeupRequest,
    ScheduledWakeupOutcome, WakeupDrainResult,
};

#[cfg(test)]
mod tests;
pub use primitive::{
    PrimitiveInvocation, PrimitiveSemantics, PrimitiveSemanticsContract,
    PrimitiveSemanticsInstaller, PrimitiveSemanticsRegistry, PrimitiveSemanticsRegistryBuilder,
    PrimitiveStageContext, PrimitiveValidationContext, PrimitiveValidationFailure,
};
