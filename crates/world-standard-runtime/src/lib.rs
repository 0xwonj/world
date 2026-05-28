//! Trusted runtime semantics for standard world primitive definitions.

mod bundle;
pub mod events;
pub mod physical;
pub mod reservation;

pub use bundle::StandardPrimitiveSemantics;
