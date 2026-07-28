//! Structured pack authoring and deterministic compilation.
//!
//! Source syntax is deliberately programmatic in this slice. Richer parsers
//! can later lower into the same defs-owned artifact boundary.

mod compiler;
mod diagnostic;
mod source;

pub use compiler::{AuthoringCompiler, Compilation};
pub use diagnostic::{CompilationDiagnostic, DiagnosticSet, SourceGraphError};
pub use source::{CompileRequest, PackSource};

pub use world_defs::{EngineProtocolVersion, SourceSnapshotId};
