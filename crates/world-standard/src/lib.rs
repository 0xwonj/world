//! Pure declarations for the engine's standard domain vocabulary.
//!
//! This crate owns portable interface and definition data only. Runtime
//! implementations and authoritative dispatch are composition concerns.

mod relocation;
mod transfer;

pub use relocation::{
    STANDARD_RELOCATION_INTERFACE_KEY, STANDARD_RELOCATION_PACK_KEY, pause_relocation_action_key,
    relocation_artifact_data, relocation_interface_descriptor, resume_relocation_action_key,
    start_relocation_action_key,
};
pub use transfer::{
    STANDARD_PACK_KEY, STANDARD_TRANSFER_INTERFACE_KEY, transfer_action_key,
    transfer_artifact_data, transfer_interface_descriptor,
};
