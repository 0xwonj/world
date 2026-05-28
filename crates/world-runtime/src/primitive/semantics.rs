use world_core::{ReplayLevel, VersionAnchor};
use world_defs::{
    EffectParamDef, EffectPrimitiveDescriptor, EffectPrimitiveId, EventContract, StagePermission,
};

use crate::{
    RuntimeError,
    outcome::RejectedOutcome,
    primitive::PrimitiveInvocation,
    transaction::{PrimitiveStageContext, PrimitiveValidationContext},
};

/// Pure descriptor used to verify trusted handler contracts against definitions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimitiveSemanticsContract {
    params: Vec<EffectParamDef>,
    required_permissions: Vec<StagePermission>,
    event_contract: EventContract,
    replay_level: ReplayLevel,
    version: VersionAnchor,
}

impl PrimitiveSemanticsContract {
    /// Creates a handler contract descriptor.
    pub fn new(
        params: impl IntoIterator<Item = EffectParamDef>,
        required_permissions: impl IntoIterator<Item = StagePermission>,
        event_contract: EventContract,
        replay_level: ReplayLevel,
        version: VersionAnchor,
    ) -> Self {
        let mut required_permissions = required_permissions.into_iter().collect::<Vec<_>>();
        required_permissions.sort();
        required_permissions.dedup();

        Self {
            params: params.into_iter().collect(),
            required_permissions,
            event_contract,
            replay_level,
            version,
        }
    }

    /// Creates a handler contract from the same descriptor used to materialize the definition.
    pub fn from_descriptor(descriptor: &impl EffectPrimitiveDescriptor) -> Self {
        Self::new(
            descriptor.params(),
            descriptor.required_permissions(),
            descriptor.event_contract(),
            descriptor.replay_level(),
            descriptor.version(),
        )
    }

    pub(crate) fn matches_definition(
        &self,
        definition: &world_defs::EffectPrimitiveDef,
    ) -> Result<(), &'static str> {
        if self.params != definition.params() {
            return Err("params");
        }
        let permissions = definition
            .required_permissions()
            .copied()
            .collect::<Vec<_>>();
        if self.required_permissions != permissions {
            return Err("required_permissions");
        }
        if &self.event_contract != definition.event_contract() {
            return Err("event_contract");
        }
        if self.replay_level != definition.replay_level() {
            return Err("replay_level");
        }
        if self.version != definition.version() {
            return Err("version");
        }

        Ok(())
    }
}

/// Trusted engine extension boundary for one checked primitive.
///
/// Ordinary definition packs compose checked primitive definitions and effect
/// programs; they do not receive raw staging callbacks.
pub trait PrimitiveSemantics: Send + Sync + 'static {
    /// Returns the primitive id this handler implements.
    fn primitive(&self) -> EffectPrimitiveId;

    /// Returns the definition contract this handler expects.
    fn contract(&self) -> PrimitiveSemanticsContract;

    /// Performs current-world validation before staging begins.
    fn validate(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveValidationContext<'_>,
    ) -> Result<(), PrimitiveValidationFailure>;

    /// Stages hard/control changes through capability-gated context methods.
    fn stage(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveStageContext<'_, '_, '_, '_>,
    ) -> Result<(), RuntimeError>;
}

/// Trusted installer for one primitive semantics bundle.
///
/// Installers add engine-owned handlers to a runtime registry after their pure
/// contracts are checked against definitions.
pub trait PrimitiveSemanticsInstaller {
    /// Installs trusted primitive handlers into a registry builder.
    fn install_semantics(
        &self,
        builder: &mut crate::primitive::PrimitiveSemanticsRegistryBuilder,
    ) -> Result<(), RuntimeError>;
}

/// Validation can either reject a request as gameplay or fail runtime infrastructure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrimitiveValidationFailure {
    /// Request-level validation rejection.
    Rejected(RejectedOutcome),
    /// Runtime infrastructure or registry failure.
    Runtime(RuntimeError),
}

impl From<RejectedOutcome> for PrimitiveValidationFailure {
    fn from(value: RejectedOutcome) -> Self {
        Self::Rejected(value)
    }
}

impl From<RuntimeError> for PrimitiveValidationFailure {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}
