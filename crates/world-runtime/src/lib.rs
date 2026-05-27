//! Causal runtime, transaction, scheduler, process, and runtime-control crate.

mod builtin;
mod commit;
mod effects;
mod error;
mod outcome;
mod request;
mod runtime;
mod transaction;
mod validation;

pub use error::RuntimeError;
pub use outcome::{
    BlockedOutcome, CommittedOutcome, RejectedOutcome, RejectionReason, RuntimeOutcome,
};
pub use request::{RequestSource, RuntimeRequest, SubmittedRole};
pub use runtime::CausalRuntime;

#[cfg(test)]
mod tests;
