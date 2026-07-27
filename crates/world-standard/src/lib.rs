//! Pure declarations for the engine's standard domain vocabulary.
//!
//! This crate owns portable interface and definition data only. Runtime
//! implementations and authoritative dispatch are composition concerns.

mod transfer;

pub use transfer::{
    STANDARD_PACK_KEY, STANDARD_TRANSFER_INTERFACE_KEY, transfer_artifact_data,
    transfer_interface_descriptor,
};
