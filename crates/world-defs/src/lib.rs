//! Checked definition model consumed by runtime and tooling crates.

mod actions;
mod effects;
mod error;
mod events;
mod keys;
mod processes;
mod registry;
mod roles;
mod semantics;

pub use actions::ActionDef;
pub use effects::{EffectOp, EffectProgramDef, StagePermission};
pub use error::DefinitionError;
pub use events::{EventContract, EventRecordSpec};
pub use keys::{
    BindingRuleKind, DefinitionName, EffectKind, EventKind, PolicyKey, RequirementKind, RoleName,
    RoleType, StateFieldName, StateValueType,
};
pub use processes::{
    ProcessDef, ProcessPolicies, ProcessStateField, ProcessStateSchema, ResolutionSupport,
    ResolutionTier,
};
pub use registry::DefinitionRegistry;
pub use roles::{BindingRuleDef, RequirementDef, RoleDef};
pub use semantics::{
    SemanticDeclarationDef, SemanticDeclarationKind, SemanticInputKind, SemanticOutputKind,
};

#[cfg(test)]
mod tests;
