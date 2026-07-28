use core::fmt;
use core::num::NonZeroU32;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter};

use super::{
    ExecutionConfigArtifactDigest, LifecycleImplementationId, LifecycleProfilesDigest,
    LifecycleStateSchemaId,
};

/// Canonical schema of the lifecycle-profile selection.
pub const LIFECYCLE_PROFILES_SCHEMA_VERSION: u16 = 2;

/// Canonical schema of the execution-configuration artifact.
pub const EXECUTION_CONFIG_SCHEMA_VERSION: u16 = 3;

const LIFECYCLE_PROFILES_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("lifecycle-profiles-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("lifecycle profile domain must be valid"),
    };

const EXECUTION_CONFIG_DOMAIN: CanonicalDomain = match CanonicalDomain::new("execution-config-v3") {
    Ok(domain) => domain,
    Err(_) => panic!("execution configuration domain must be valid"),
};

/// State owned privately by one lifecycle implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleStateBindingV1 {
    /// The implementation owns no persistent state.
    Stateless,
    /// The implementation owns state encoded under this exact schema.
    Persistent(LifecycleStateSchemaId),
}

impl LifecycleStateBindingV1 {
    fn write_canonical(self, writer: &mut CanonicalWriter) {
        match self {
            Self::Stateless => writer.write_discriminant(0),
            Self::Persistent(schema) => {
                writer.write_discriminant(1);
                write_fixed_identity(writer, schema.as_bytes());
            }
        }
    }
}

/// Exact implementation and private-state schema selected for one lifecycle port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifecycleBindingV1 {
    implementation: LifecycleImplementationId,
    state: LifecycleStateBindingV1,
}

impl LifecycleBindingV1 {
    /// Selects one stateless lifecycle implementation.
    #[must_use]
    pub const fn stateless(implementation: LifecycleImplementationId) -> Self {
        Self {
            implementation,
            state: LifecycleStateBindingV1::Stateless,
        }
    }

    /// Selects one lifecycle implementation with private persistent state.
    #[must_use]
    pub const fn persistent(
        implementation: LifecycleImplementationId,
        schema: LifecycleStateSchemaId,
    ) -> Self {
        Self {
            implementation,
            state: LifecycleStateBindingV1::Persistent(schema),
        }
    }

    /// Returns the exact selected behavior identity.
    #[must_use]
    pub const fn implementation(self) -> LifecycleImplementationId {
        self.implementation
    }

    /// Returns the implementation-owned state contract.
    #[must_use]
    pub const fn state(self) -> LifecycleStateBindingV1 {
        self.state
    }

    fn write_canonical(self, writer: &mut CanonicalWriter) {
        write_fixed_identity(writer, self.implementation.as_bytes());
        self.state.write_canonical(writer);
    }
}

/// Selection state of an optional lifecycle port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionalLifecycleBindingV1 {
    /// The port is absent and produces no lifecycle work.
    Disabled,
    /// The port uses this exact implementation.
    Enabled(LifecycleBindingV1),
}

impl OptionalLifecycleBindingV1 {
    fn write_canonical(self, writer: &mut CanonicalWriter) {
        match self {
            Self::Disabled => writer.write_discriminant(0),
            Self::Enabled(binding) => {
                writer.write_discriminant(1);
                binding.write_canonical(writer);
            }
        }
    }
}

/// How one selected action policy crosses its execution boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionPolicyExecutionV1 {
    /// Evaluate deterministically inside the reserved engine step.
    InlineDeterministic,
    /// Capture a durable request and accept its result through ingress.
    DeferredCaptured,
}

impl ActionPolicyExecutionV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::InlineDeterministic => 0,
            Self::DeferredCaptured => 1,
        }
    }
}

/// Exact action-policy selection and execution class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionPolicyBindingV1 {
    binding: LifecycleBindingV1,
    execution: ActionPolicyExecutionV1,
}

impl ActionPolicyBindingV1 {
    /// Selects a stateless policy evaluated inside the reserved engine step.
    #[must_use]
    pub const fn inline_deterministic(implementation: LifecycleImplementationId) -> Self {
        Self {
            binding: LifecycleBindingV1::stateless(implementation),
            execution: ActionPolicyExecutionV1::InlineDeterministic,
        }
    }

    /// Selects an exact binding and action execution class.
    #[must_use]
    pub const fn new(binding: LifecycleBindingV1, execution: ActionPolicyExecutionV1) -> Self {
        Self { binding, execution }
    }

    /// Returns the selected implementation and state contract.
    #[must_use]
    pub const fn binding(self) -> LifecycleBindingV1 {
        self.binding
    }

    /// Returns the selected action execution class.
    #[must_use]
    pub const fn execution(self) -> ActionPolicyExecutionV1 {
        self.execution
    }

    fn write_canonical(self, writer: &mut CanonicalWriter) {
        self.binding.write_canonical(writer);
        writer.write_discriminant(self.execution.canonical_tag());
    }
}

/// Exact closed lifecycle selection understood by this protocol version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifecycleProfilesV2 {
    evidence: LifecycleBindingV1,
    appraisal: LifecycleBindingV1,
    social: OptionalLifecycleBindingV1,
    intent: LifecycleBindingV1,
    activity: LifecycleBindingV1,
    action: ActionPolicyBindingV1,
}

impl LifecycleProfilesV2 {
    /// Constructs one complete port-by-port lifecycle selection.
    #[must_use]
    pub const fn new(
        evidence: LifecycleBindingV1,
        appraisal: LifecycleBindingV1,
        social: OptionalLifecycleBindingV1,
        intent: LifecycleBindingV1,
        activity: LifecycleBindingV1,
        action: ActionPolicyBindingV1,
    ) -> Self {
        Self {
            evidence,
            appraisal,
            social,
            intent,
            activity,
            action,
        }
    }

    /// Returns the selected evidence-assimilation binding.
    #[must_use]
    pub const fn evidence(self) -> LifecycleBindingV1 {
        self.evidence
    }

    /// Returns the selected appraisal binding.
    #[must_use]
    pub const fn appraisal(self) -> LifecycleBindingV1 {
        self.appraisal
    }

    /// Returns the optional social-interpretation binding.
    #[must_use]
    pub const fn social(self) -> OptionalLifecycleBindingV1 {
        self.social
    }

    /// Returns the selected intent-policy binding.
    #[must_use]
    pub const fn intent(self) -> LifecycleBindingV1 {
        self.intent
    }

    /// Returns the selected activity-controller binding.
    #[must_use]
    pub const fn activity(self) -> LifecycleBindingV1 {
        self.activity
    }

    /// Returns the selected action-policy binding.
    #[must_use]
    pub const fn action(self) -> ActionPolicyBindingV1 {
        self.action
    }

    /// Returns the canonical lifecycle-profile identity.
    #[must_use]
    pub fn digest(self) -> LifecycleProfilesDigest {
        LifecycleProfilesDigest::of_canonical(&self.canonical_bytes())
    }

    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        let mut writer = CanonicalWriter::new(LIFECYCLE_PROFILES_DOMAIN);
        writer.write_u16(LIFECYCLE_PROFILES_SCHEMA_VERSION);
        self.evidence.write_canonical(&mut writer);
        self.appraisal.write_canonical(&mut writer);
        self.social.write_canonical(&mut writer);
        self.intent.write_canonical(&mut writer);
        self.activity.write_canonical(&mut writer);
        self.action.write_canonical(&mut writer);
        writer.finish()
    }
}

fn write_fixed_identity(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width lifecycle identity must fit the canonical protocol");
    }
}

#[cfg(test)]
pub(crate) const fn fixture_lifecycle_profiles() -> LifecycleProfilesV2 {
    LifecycleProfilesV2::new(
        LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0xe1; 32])),
        LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0xe2; 32])),
        OptionalLifecycleBindingV1::Disabled,
        LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0xe3; 32])),
        LifecycleBindingV1::persistent(
            LifecycleImplementationId::from_bytes([0xe4; 32]),
            LifecycleStateSchemaId::from_bytes([0x54; 32]),
        ),
        ActionPolicyBindingV1::inline_deterministic(LifecycleImplementationId::from_bytes(
            [0xa1; 32],
        )),
    )
}

/// Deterministic candidate-set resolver selected by execution semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MomentResolutionPolicyV2 {
    /// Order each connected component from its resource-local evidence,
    /// greedily admit a maximal feasible set, and refine combined-invariant
    /// failures without retrying the moment.
    CanonicalComponentGreedy,
}

impl MomentResolutionPolicyV2 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::CanonicalComponentGreedy => 1,
        }
    }
}

/// Domain policy for equal-priority containment conflicts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainmentConflictPolicyV1 {
    /// Rank every contender by a semantic-keyed highest-random-weight score;
    /// selecting the maximum score is the complete conflict-choice mapping.
    EqualHighestRandomWeight,
}

impl ContainmentConflictPolicyV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::EqualHighestRandomWeight => 0,
        }
    }
}

/// Authoritative pseudorandom function selected by execution semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomOraclePolicyV1 {
    /// BLAKE3 keyed PRF with a 256-bit result.
    Blake3KeyedPrf256,
}

impl RandomOraclePolicyV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Blake3KeyedPrf256 => 0,
        }
    }
}

/// Canonical policy for constructing authoritative random keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomKeyPolicyV1 {
    /// Use format-independent semantic causality, resource, and contender
    /// identities.
    SemanticContainmentConflict,
}

impl RandomKeyPolicyV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::SemanticContainmentConflict => 0,
        }
    }
}

/// Closed finalization precedence selected by execution semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalizationPolicyV1 {
    /// A retained disposition wins, followed by semantic termination.
    DispositionFirst,
}

impl FinalizationPolicyV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::DispositionFirst => 0,
        }
    }
}

/// Closed routing policy for work emitted by an accepted commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostCommitRoutingPolicyV1 {
    /// Route each physical event as direct evidence to the event actor.
    DirectActorEvidence,
}

impl PostCommitRoutingPolicyV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::DirectActorEvidence => 0,
        }
    }
}

/// Safety transition selected when one same-time causal tranche is exhausted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SameTimeWaveExhaustionPolicyV1 {
    /// Pause with unresolved work preserved for an explicit bounded resume.
    Pause,
}

impl SameTimeWaveExhaustionPolicyV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Pause => 0,
        }
    }
}

/// Safety transition selected when due or evaluable work exceeds its bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkPopulationExhaustionPolicyV1 {
    /// Quarantine because unchanged semantics would deterministically exceed
    /// the same bound again.
    Quarantine,
}

impl WorkPopulationExhaustionPolicyV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Quarantine => 0,
        }
    }
}

/// How a captured deferred action result enters simulation time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferredActionAdmissionModeV1 {
    /// Hold the session frontier until this invocation is captured or managed.
    FrontierBlocking,
    /// Admit the result at an explicit host-selected simulation moment.
    HostScheduled,
}

impl DeferredActionAdmissionModeV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::FrontierBlocking => 0,
            Self::HostScheduled => 1,
        }
    }
}

/// Recorded outcome used when deferred action evaluation cannot continue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferredActionFallbackV1 {
    /// Finish the waiting opportunity as failed in a later scheduled wake.
    FinishFailedOnLaterWake,
}

impl DeferredActionFallbackV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::FinishFailedOnLaterWake => 0,
        }
    }
}

/// Why bounded deferred-action control could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferredActionControlError {
    /// A dispatch-safe request was configured with no byte capacity.
    ZeroMaximumRequestBytes,
    /// A captured result was configured with no byte capacity.
    ZeroMaximumResultBytes,
    /// An engine-private continuation was configured with no byte capacity.
    ZeroMaximumPrivateContinuationBytes,
    /// An engine-private read witness was configured with no byte capacity.
    ZeroMaximumPrivateWitnessBytes,
}

impl fmt::Display for DeferredActionControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroMaximumRequestBytes => {
                formatter.write_str("maximum deferred request bytes must be nonzero")
            }
            Self::ZeroMaximumResultBytes => {
                formatter.write_str("maximum deferred result bytes must be nonzero")
            }
            Self::ZeroMaximumPrivateContinuationBytes => {
                formatter.write_str("maximum deferred private-continuation bytes must be nonzero")
            }
            Self::ZeroMaximumPrivateWitnessBytes => {
                formatter.write_str("maximum deferred private-witness bytes must be nonzero")
            }
        }
    }
}

impl std::error::Error for DeferredActionControlError {}

/// Closed execution control for deferred action-policy evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferredActionControlV1 {
    /// Inline action evaluation carries no deferred settings.
    Disabled,
    /// Exact bounds and timing law for retained deferred evaluation.
    Enabled {
        /// How a captured result enters simulation time.
        admission_mode: DeferredActionAdmissionModeV1,
        /// Maximum actor-visible payload changes that may cause reinvocation.
        maximum_visible_reinvocations: u32,
        /// Maximum dispatch-safe request artifact size.
        maximum_request_bytes: NonZeroU32,
        /// Maximum captured result artifact size.
        maximum_result_bytes: NonZeroU32,
        /// Maximum engine-private continuation artifact size.
        maximum_private_continuation_bytes: NonZeroU32,
        /// Maximum engine-private read-witness artifact size.
        maximum_private_witness_bytes: NonZeroU32,
        /// Fixed later-wake failure behavior.
        fallback: DeferredActionFallbackV1,
    },
}

impl DeferredActionControlV1 {
    /// Constructs enabled deferred action control with checked byte bounds.
    pub const fn enabled(
        admission_mode: DeferredActionAdmissionModeV1,
        maximum_visible_reinvocations: u32,
        maximum_request_bytes: u32,
        maximum_result_bytes: u32,
        maximum_private_continuation_bytes: u32,
        maximum_private_witness_bytes: u32,
    ) -> Result<Self, DeferredActionControlError> {
        let Some(maximum_request_bytes) = NonZeroU32::new(maximum_request_bytes) else {
            return Err(DeferredActionControlError::ZeroMaximumRequestBytes);
        };
        let Some(maximum_result_bytes) = NonZeroU32::new(maximum_result_bytes) else {
            return Err(DeferredActionControlError::ZeroMaximumResultBytes);
        };
        let Some(maximum_private_continuation_bytes) =
            NonZeroU32::new(maximum_private_continuation_bytes)
        else {
            return Err(DeferredActionControlError::ZeroMaximumPrivateContinuationBytes);
        };
        let Some(maximum_private_witness_bytes) = NonZeroU32::new(maximum_private_witness_bytes)
        else {
            return Err(DeferredActionControlError::ZeroMaximumPrivateWitnessBytes);
        };
        Ok(Self::Enabled {
            admission_mode,
            maximum_visible_reinvocations,
            maximum_request_bytes,
            maximum_result_bytes,
            maximum_private_continuation_bytes,
            maximum_private_witness_bytes,
            fallback: DeferredActionFallbackV1::FinishFailedOnLaterWake,
        })
    }

    /// Returns whether retained deferred evaluation is enabled.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled { .. })
    }

    /// Returns the configured result-admission mode when enabled.
    #[must_use]
    pub const fn admission_mode(self) -> Option<DeferredActionAdmissionModeV1> {
        match self {
            Self::Disabled => None,
            Self::Enabled { admission_mode, .. } => Some(admission_mode),
        }
    }

    /// Returns the actor-visible reinvocation bound when enabled.
    #[must_use]
    pub const fn maximum_visible_reinvocations(self) -> Option<u32> {
        match self {
            Self::Disabled => None,
            Self::Enabled {
                maximum_visible_reinvocations,
                ..
            } => Some(maximum_visible_reinvocations),
        }
    }

    /// Returns the dispatch-safe request byte bound when enabled.
    #[must_use]
    pub const fn maximum_request_bytes(self) -> Option<NonZeroU32> {
        match self {
            Self::Disabled => None,
            Self::Enabled {
                maximum_request_bytes,
                ..
            } => Some(maximum_request_bytes),
        }
    }

    /// Returns the captured-result byte bound when enabled.
    #[must_use]
    pub const fn maximum_result_bytes(self) -> Option<NonZeroU32> {
        match self {
            Self::Disabled => None,
            Self::Enabled {
                maximum_result_bytes,
                ..
            } => Some(maximum_result_bytes),
        }
    }

    /// Returns the engine-private continuation byte bound when enabled.
    #[must_use]
    pub const fn maximum_private_continuation_bytes(self) -> Option<NonZeroU32> {
        match self {
            Self::Disabled => None,
            Self::Enabled {
                maximum_private_continuation_bytes,
                ..
            } => Some(maximum_private_continuation_bytes),
        }
    }

    /// Returns the engine-private read-witness byte bound when enabled.
    #[must_use]
    pub const fn maximum_private_witness_bytes(self) -> Option<NonZeroU32> {
        match self {
            Self::Disabled => None,
            Self::Enabled {
                maximum_private_witness_bytes,
                ..
            } => Some(maximum_private_witness_bytes),
        }
    }

    /// Returns the fixed failure fallback when enabled.
    #[must_use]
    pub const fn fallback(self) -> Option<DeferredActionFallbackV1> {
        match self {
            Self::Disabled => None,
            Self::Enabled { fallback, .. } => Some(fallback),
        }
    }

    fn write_canonical(self, writer: &mut CanonicalWriter) {
        match self {
            Self::Disabled => writer.write_discriminant(0),
            Self::Enabled {
                admission_mode,
                maximum_visible_reinvocations,
                maximum_request_bytes,
                maximum_result_bytes,
                maximum_private_continuation_bytes,
                maximum_private_witness_bytes,
                fallback,
            } => {
                writer.write_discriminant(1);
                writer.write_discriminant(admission_mode.canonical_tag());
                writer.write_u32(maximum_visible_reinvocations);
                writer.write_u32(maximum_request_bytes.get());
                writer.write_u32(maximum_result_bytes.get());
                writer.write_u32(maximum_private_continuation_bytes.get());
                writer.write_u32(maximum_private_witness_bytes.get());
                writer.write_discriminant(fallback.canonical_tag());
            }
        }
    }
}

/// Why deterministic execution limits could not form a configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionConfigError {
    /// A moment was configured to permit no due work.
    ZeroMaximumWorkPerMoment,
    /// A moment was configured to permit no command evaluation.
    ZeroMaximumEvaluableCommands,
    /// A same-time causal tranche was configured to permit no wave.
    ZeroMaximumSameTimeWaves,
    /// The deferred constructor was given disabled action control.
    DeferredActionControlRequired,
}

impl fmt::Display for ExecutionConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroMaximumWorkPerMoment => {
                formatter.write_str("maximum work per moment must be nonzero")
            }
            Self::ZeroMaximumEvaluableCommands => {
                formatter.write_str("maximum evaluable commands must be nonzero")
            }
            Self::ZeroMaximumSameTimeWaves => {
                formatter.write_str("maximum same-time waves must be nonzero")
            }
            Self::DeferredActionControlRequired => {
                formatter.write_str("deferred execution requires enabled action control")
            }
        }
    }
}

impl std::error::Error for ExecutionConfigError {}

/// Exact behavior-affecting execution configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionConfigArtifactV3 {
    maximum_work_per_moment: NonZeroU32,
    maximum_evaluable_commands: NonZeroU32,
    maximum_same_time_waves: NonZeroU32,
    moment_resolution_policy: MomentResolutionPolicyV2,
    containment_conflict_policy: ContainmentConflictPolicyV1,
    random_oracle_policy: RandomOraclePolicyV1,
    random_key_policy: RandomKeyPolicyV1,
    finalization_policy: FinalizationPolicyV1,
    post_commit_routing_policy: PostCommitRoutingPolicyV1,
    same_time_wave_exhaustion_policy: SameTimeWaveExhaustionPolicyV1,
    work_population_exhaustion_policy: WorkPopulationExhaustionPolicyV1,
    deferred_action_control: DeferredActionControlV1,
}

impl ExecutionConfigArtifactV3 {
    /// Constructs complete execution configuration for inline action policy.
    #[must_use = "execution configuration must be checked before origin activation"]
    pub const fn inline(
        maximum_work_per_moment: u32,
        maximum_evaluable_commands: u32,
        maximum_same_time_waves: u32,
    ) -> Result<Self, ExecutionConfigError> {
        Self::build(
            maximum_work_per_moment,
            maximum_evaluable_commands,
            maximum_same_time_waves,
            DeferredActionControlV1::Disabled,
        )
    }

    /// Constructs complete execution configuration for deferred action policy.
    #[must_use = "execution configuration must be checked before origin activation"]
    pub const fn deferred(
        maximum_work_per_moment: u32,
        maximum_evaluable_commands: u32,
        maximum_same_time_waves: u32,
        control: DeferredActionControlV1,
    ) -> Result<Self, ExecutionConfigError> {
        if !control.is_enabled() {
            return Err(ExecutionConfigError::DeferredActionControlRequired);
        }
        Self::build(
            maximum_work_per_moment,
            maximum_evaluable_commands,
            maximum_same_time_waves,
            control,
        )
    }

    const fn build(
        maximum_work_per_moment: u32,
        maximum_evaluable_commands: u32,
        maximum_same_time_waves: u32,
        deferred_action_control: DeferredActionControlV1,
    ) -> Result<Self, ExecutionConfigError> {
        let Some(maximum_work_per_moment) = NonZeroU32::new(maximum_work_per_moment) else {
            return Err(ExecutionConfigError::ZeroMaximumWorkPerMoment);
        };
        let Some(maximum_evaluable_commands) = NonZeroU32::new(maximum_evaluable_commands) else {
            return Err(ExecutionConfigError::ZeroMaximumEvaluableCommands);
        };
        let Some(maximum_same_time_waves) = NonZeroU32::new(maximum_same_time_waves) else {
            return Err(ExecutionConfigError::ZeroMaximumSameTimeWaves);
        };

        Ok(Self {
            maximum_work_per_moment,
            maximum_evaluable_commands,
            maximum_same_time_waves,
            moment_resolution_policy: MomentResolutionPolicyV2::CanonicalComponentGreedy,
            containment_conflict_policy: ContainmentConflictPolicyV1::EqualHighestRandomWeight,
            random_oracle_policy: RandomOraclePolicyV1::Blake3KeyedPrf256,
            random_key_policy: RandomKeyPolicyV1::SemanticContainmentConflict,
            finalization_policy: FinalizationPolicyV1::DispositionFirst,
            post_commit_routing_policy: PostCommitRoutingPolicyV1::DirectActorEvidence,
            same_time_wave_exhaustion_policy: SameTimeWaveExhaustionPolicyV1::Pause,
            work_population_exhaustion_policy: WorkPopulationExhaustionPolicyV1::Quarantine,
            deferred_action_control,
        })
    }

    /// Returns the maximum complete due set admitted at one moment.
    #[must_use]
    pub const fn maximum_work_per_moment(self) -> NonZeroU32 {
        self.maximum_work_per_moment
    }

    /// Returns the maximum command candidates evaluated at one moment.
    #[must_use]
    pub const fn maximum_evaluable_commands(self) -> NonZeroU32 {
        self.maximum_evaluable_commands
    }

    /// Returns the maximum causal waves in one same-time tranche.
    #[must_use]
    pub const fn maximum_same_time_waves(self) -> NonZeroU32 {
        self.maximum_same_time_waves
    }

    /// Returns the selected deterministic moment resolver.
    #[must_use]
    pub const fn moment_resolution_policy(self) -> MomentResolutionPolicyV2 {
        self.moment_resolution_policy
    }

    /// Returns the selected containment conflict policy.
    #[must_use]
    pub const fn containment_conflict_policy(self) -> ContainmentConflictPolicyV1 {
        self.containment_conflict_policy
    }

    /// Returns the selected keyed pseudorandom function.
    #[must_use]
    pub const fn random_oracle_policy(self) -> RandomOraclePolicyV1 {
        self.random_oracle_policy
    }

    /// Returns the selected semantic random-key policy.
    #[must_use]
    pub const fn random_key_policy(self) -> RandomKeyPolicyV1 {
        self.random_key_policy
    }

    /// Returns the selected finalization precedence.
    #[must_use]
    pub const fn finalization_policy(self) -> FinalizationPolicyV1 {
        self.finalization_policy
    }

    /// Returns the selected routing policy for accepted post-commit work.
    #[must_use]
    pub const fn post_commit_routing_policy(self) -> PostCommitRoutingPolicyV1 {
        self.post_commit_routing_policy
    }

    /// Returns the same-time wave-exhaustion transition.
    #[must_use]
    pub const fn same_time_wave_exhaustion_policy(self) -> SameTimeWaveExhaustionPolicyV1 {
        self.same_time_wave_exhaustion_policy
    }

    /// Returns the due/evaluable population-exhaustion transition.
    #[must_use]
    pub const fn work_population_exhaustion_policy(self) -> WorkPopulationExhaustionPolicyV1 {
        self.work_population_exhaustion_policy
    }

    /// Returns the closed deferred action-policy control selection.
    #[must_use]
    pub const fn deferred_action_control(self) -> DeferredActionControlV1 {
        self.deferred_action_control
    }

    /// Returns the canonical execution-configuration identity.
    #[must_use]
    pub fn digest(self) -> ExecutionConfigArtifactDigest {
        ExecutionConfigArtifactDigest::of_canonical(&self.canonical_bytes())
    }

    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        execution_config_bytes(self)
    }
}

fn execution_config_bytes(config: ExecutionConfigArtifactV3) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(EXECUTION_CONFIG_DOMAIN);
    writer.write_u16(EXECUTION_CONFIG_SCHEMA_VERSION);
    writer.write_u32(config.maximum_work_per_moment.get());
    writer.write_u32(config.maximum_evaluable_commands.get());
    writer.write_u32(config.maximum_same_time_waves.get());
    writer.write_discriminant(config.moment_resolution_policy.canonical_tag());
    writer.write_discriminant(config.containment_conflict_policy.canonical_tag());
    writer.write_discriminant(config.random_oracle_policy.canonical_tag());
    writer.write_discriminant(config.random_key_policy.canonical_tag());
    writer.write_discriminant(config.finalization_policy.canonical_tag());
    writer.write_discriminant(config.post_commit_routing_policy.canonical_tag());
    writer.write_discriminant(config.same_time_wave_exhaustion_policy.canonical_tag());
    writer.write_discriminant(config.work_population_exhaustion_policy.canonical_tag());
    config.deferred_action_control.write_canonical(&mut writer);
    writer.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_lifecycle_profile_matches_the_frozen_vector() {
        let profiles = fixture_lifecycle_profiles();

        assert_eq!(
            hex(profiles.canonical_bytes().as_bytes()),
            "776f726c642d63616e6f6e6963616c2d763100000000000000156c6966656379636c652d70726f66696c65732d763200020000000000000020e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1000000000000000000000020e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e200000000000000000000000000000020e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3000000000000000000000020e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e400000001000000000000002054545454545454545454545454545454545454545454545454545454545454540000000000000020a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a10000000000000000"
        );
        assert_eq!(
            profiles.digest().to_string(),
            "3aaea302d4a6ff6b178dee51269baf56134f09612c178498abce3dc13005e6ee"
        );
        assert_eq!(
            profiles.activity().state(),
            LifecycleStateBindingV1::Persistent(LifecycleStateSchemaId::from_bytes([0x54; 32]))
        );
        assert_eq!(profiles.social(), OptionalLifecycleBindingV1::Disabled);
        assert_eq!(
            profiles.action().execution(),
            ActionPolicyExecutionV1::InlineDeterministic
        );
    }

    #[test]
    fn every_port_and_private_state_schema_changes_the_profile_identity() {
        let baseline = fixture_lifecycle_profiles();
        let changed = [
            LifecycleProfilesV2::new(
                LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0xf1; 32])),
                baseline.appraisal(),
                baseline.social(),
                baseline.intent(),
                baseline.activity(),
                baseline.action(),
            ),
            LifecycleProfilesV2::new(
                LifecycleBindingV1::persistent(
                    baseline.evidence().implementation(),
                    LifecycleStateSchemaId::from_bytes([0x51; 32]),
                ),
                baseline.appraisal(),
                baseline.social(),
                baseline.intent(),
                baseline.activity(),
                baseline.action(),
            ),
            LifecycleProfilesV2::new(
                baseline.evidence(),
                LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0xf2; 32])),
                baseline.social(),
                baseline.intent(),
                baseline.activity(),
                baseline.action(),
            ),
            LifecycleProfilesV2::new(
                baseline.evidence(),
                LifecycleBindingV1::persistent(
                    baseline.appraisal().implementation(),
                    LifecycleStateSchemaId::from_bytes([0x52; 32]),
                ),
                baseline.social(),
                baseline.intent(),
                baseline.activity(),
                baseline.action(),
            ),
            LifecycleProfilesV2::new(
                baseline.evidence(),
                baseline.appraisal(),
                OptionalLifecycleBindingV1::Enabled(LifecycleBindingV1::stateless(
                    LifecycleImplementationId::from_bytes([0xf3; 32]),
                )),
                baseline.intent(),
                baseline.activity(),
                baseline.action(),
            ),
            LifecycleProfilesV2::new(
                baseline.evidence(),
                baseline.appraisal(),
                OptionalLifecycleBindingV1::Enabled(LifecycleBindingV1::persistent(
                    LifecycleImplementationId::from_bytes([0xf3; 32]),
                    LifecycleStateSchemaId::from_bytes([0x53; 32]),
                )),
                baseline.intent(),
                baseline.activity(),
                baseline.action(),
            ),
            LifecycleProfilesV2::new(
                baseline.evidence(),
                baseline.appraisal(),
                baseline.social(),
                LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0xf4; 32])),
                baseline.activity(),
                baseline.action(),
            ),
            LifecycleProfilesV2::new(
                baseline.evidence(),
                baseline.appraisal(),
                baseline.social(),
                LifecycleBindingV1::persistent(
                    baseline.intent().implementation(),
                    LifecycleStateSchemaId::from_bytes([0x54; 32]),
                ),
                baseline.activity(),
                baseline.action(),
            ),
            LifecycleProfilesV2::new(
                baseline.evidence(),
                baseline.appraisal(),
                baseline.social(),
                baseline.intent(),
                LifecycleBindingV1::persistent(
                    LifecycleImplementationId::from_bytes([0xf5; 32]),
                    LifecycleStateSchemaId::from_bytes([0x54; 32]),
                ),
                baseline.action(),
            ),
            LifecycleProfilesV2::new(
                baseline.evidence(),
                baseline.appraisal(),
                baseline.social(),
                baseline.intent(),
                LifecycleBindingV1::persistent(
                    baseline.activity().implementation(),
                    LifecycleStateSchemaId::from_bytes([0x55; 32]),
                ),
                baseline.action(),
            ),
            LifecycleProfilesV2::new(
                baseline.evidence(),
                baseline.appraisal(),
                baseline.social(),
                baseline.intent(),
                baseline.activity(),
                ActionPolicyBindingV1::inline_deterministic(LifecycleImplementationId::from_bytes(
                    [0xf6; 32],
                )),
            ),
            LifecycleProfilesV2::new(
                baseline.evidence(),
                baseline.appraisal(),
                baseline.social(),
                baseline.intent(),
                baseline.activity(),
                ActionPolicyBindingV1::new(
                    LifecycleBindingV1::persistent(
                        baseline.action().binding().implementation(),
                        LifecycleStateSchemaId::from_bytes([0x56; 32]),
                    ),
                    baseline.action().execution(),
                ),
            ),
            LifecycleProfilesV2::new(
                baseline.evidence(),
                baseline.appraisal(),
                baseline.social(),
                baseline.intent(),
                baseline.activity(),
                ActionPolicyBindingV1::new(
                    baseline.action().binding(),
                    ActionPolicyExecutionV1::DeferredCaptured,
                ),
            ),
        ];

        for profile in changed {
            assert_ne!(profile.canonical_bytes(), baseline.canonical_bytes());
            assert_ne!(profile.digest(), baseline.digest());
        }
    }

    #[test]
    fn lifecycle_binding_constructors_retain_exact_state_contracts() {
        let implementation = LifecycleImplementationId::from_bytes([0xc1; 32]);
        let schema = LifecycleStateSchemaId::from_bytes([0xc2; 32]);
        let stateless = LifecycleBindingV1::stateless(implementation);
        let persistent = LifecycleBindingV1::persistent(implementation, schema);

        assert_eq!(stateless.implementation(), implementation);
        assert_eq!(stateless.state(), LifecycleStateBindingV1::Stateless);
        assert_eq!(persistent.implementation(), implementation);
        assert_eq!(
            persistent.state(),
            LifecycleStateBindingV1::Persistent(schema)
        );
    }

    #[test]
    fn complete_configuration_matches_the_frozen_vector() {
        let config = valid(ExecutionConfigArtifactV3::inline(64, 32, 16));

        assert_eq!(
            hex(config.canonical_bytes().as_bytes()),
            "776f726c642d63616e6f6e6963616c2d76310000000000000013657865637574696f6e2d636f6e6669672d76330003000000400000002000000010000000010000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(
            config.digest().to_string(),
            "49937ac98d56333b9b4341f2686d96324e6a45767682d91c3f03cd62040c2aaa"
        );
        assert_eq!(config.maximum_work_per_moment().get(), 64);
        assert_eq!(config.maximum_evaluable_commands().get(), 32);
        assert_eq!(config.maximum_same_time_waves().get(), 16);
        assert_eq!(
            config.moment_resolution_policy(),
            MomentResolutionPolicyV2::CanonicalComponentGreedy
        );
        assert_eq!(
            config.containment_conflict_policy(),
            ContainmentConflictPolicyV1::EqualHighestRandomWeight
        );
        assert_eq!(
            config.random_oracle_policy(),
            RandomOraclePolicyV1::Blake3KeyedPrf256
        );
        assert_eq!(
            config.random_key_policy(),
            RandomKeyPolicyV1::SemanticContainmentConflict
        );
        assert_eq!(
            config.finalization_policy(),
            FinalizationPolicyV1::DispositionFirst
        );
        assert_eq!(
            config.post_commit_routing_policy(),
            PostCommitRoutingPolicyV1::DirectActorEvidence
        );
        assert_eq!(
            config.same_time_wave_exhaustion_policy(),
            SameTimeWaveExhaustionPolicyV1::Pause
        );
        assert_eq!(
            config.work_population_exhaustion_policy(),
            WorkPopulationExhaustionPolicyV1::Quarantine
        );
        assert_eq!(
            config.deferred_action_control(),
            DeferredActionControlV1::Disabled
        );
    }

    #[test]
    fn every_limit_is_checked_and_identity_bearing() {
        assert_eq!(
            ExecutionConfigArtifactV3::inline(0, 1, 1),
            Err(ExecutionConfigError::ZeroMaximumWorkPerMoment)
        );
        assert_eq!(
            ExecutionConfigArtifactV3::inline(1, 0, 1),
            Err(ExecutionConfigError::ZeroMaximumEvaluableCommands)
        );
        assert_eq!(
            ExecutionConfigArtifactV3::inline(1, 1, 0),
            Err(ExecutionConfigError::ZeroMaximumSameTimeWaves)
        );

        let baseline = valid(ExecutionConfigArtifactV3::inline(64, 32, 16)).digest();
        for variant in [
            valid(ExecutionConfigArtifactV3::inline(65, 32, 16)),
            valid(ExecutionConfigArtifactV3::inline(64, 33, 16)),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 17)),
        ] {
            assert_ne!(variant.digest(), baseline);
        }
    }

    #[test]
    fn deferred_control_checks_each_artifact_bound_and_allows_zero_reinvocations() {
        assert_eq!(
            DeferredActionControlV1::enabled(
                DeferredActionAdmissionModeV1::FrontierBlocking,
                0,
                0,
                1,
                1,
                1,
            ),
            Err(DeferredActionControlError::ZeroMaximumRequestBytes)
        );
        assert_eq!(
            DeferredActionControlV1::enabled(
                DeferredActionAdmissionModeV1::FrontierBlocking,
                0,
                1,
                0,
                1,
                1,
            ),
            Err(DeferredActionControlError::ZeroMaximumResultBytes)
        );
        assert_eq!(
            DeferredActionControlV1::enabled(
                DeferredActionAdmissionModeV1::FrontierBlocking,
                0,
                1,
                1,
                0,
                1,
            ),
            Err(DeferredActionControlError::ZeroMaximumPrivateContinuationBytes)
        );
        assert_eq!(
            DeferredActionControlV1::enabled(
                DeferredActionAdmissionModeV1::FrontierBlocking,
                0,
                1,
                1,
                1,
                0,
            ),
            Err(DeferredActionControlError::ZeroMaximumPrivateWitnessBytes)
        );

        let control = valid(DeferredActionControlV1::enabled(
            DeferredActionAdmissionModeV1::FrontierBlocking,
            0,
            1024,
            512,
            2048,
            256,
        ));
        assert_eq!(control.maximum_visible_reinvocations(), Some(0));
        assert_eq!(
            control.maximum_request_bytes().map(NonZeroU32::get),
            Some(1024)
        );
        assert_eq!(
            control.maximum_result_bytes().map(NonZeroU32::get),
            Some(512)
        );
        assert_eq!(
            control
                .maximum_private_continuation_bytes()
                .map(NonZeroU32::get),
            Some(2048)
        );
        assert_eq!(
            control.maximum_private_witness_bytes().map(NonZeroU32::get),
            Some(256)
        );
        assert_eq!(
            control.fallback(),
            Some(DeferredActionFallbackV1::FinishFailedOnLaterWake)
        );
    }

    #[test]
    fn deferred_control_is_required_and_every_setting_changes_configuration_identity() {
        assert_eq!(
            ExecutionConfigArtifactV3::deferred(64, 32, 16, DeferredActionControlV1::Disabled,),
            Err(ExecutionConfigError::DeferredActionControlRequired)
        );

        let control = valid(DeferredActionControlV1::enabled(
            DeferredActionAdmissionModeV1::FrontierBlocking,
            0,
            1024,
            512,
            2048,
            256,
        ));
        let baseline = valid(ExecutionConfigArtifactV3::deferred(64, 32, 16, control));
        assert_eq!(
            baseline.deferred_action_control().admission_mode(),
            Some(DeferredActionAdmissionModeV1::FrontierBlocking)
        );
        assert_ne!(
            baseline.digest(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)).digest()
        );

        let variants = [
            valid(DeferredActionControlV1::enabled(
                DeferredActionAdmissionModeV1::HostScheduled,
                0,
                1024,
                512,
                2048,
                256,
            )),
            valid(DeferredActionControlV1::enabled(
                DeferredActionAdmissionModeV1::FrontierBlocking,
                1,
                1024,
                512,
                2048,
                256,
            )),
            valid(DeferredActionControlV1::enabled(
                DeferredActionAdmissionModeV1::FrontierBlocking,
                0,
                1025,
                512,
                2048,
                256,
            )),
            valid(DeferredActionControlV1::enabled(
                DeferredActionAdmissionModeV1::FrontierBlocking,
                0,
                1024,
                513,
                2048,
                256,
            )),
            valid(DeferredActionControlV1::enabled(
                DeferredActionAdmissionModeV1::FrontierBlocking,
                0,
                1024,
                512,
                2049,
                256,
            )),
            valid(DeferredActionControlV1::enabled(
                DeferredActionAdmissionModeV1::FrontierBlocking,
                0,
                1024,
                512,
                2048,
                257,
            )),
        ];
        for control in variants {
            let variant = valid(ExecutionConfigArtifactV3::deferred(64, 32, 16, control));
            assert_ne!(variant.digest(), baseline.digest());
        }
    }

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("configuration fixture must be valid: {error}"),
        }
    }

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
            encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        encoded
    }
}
