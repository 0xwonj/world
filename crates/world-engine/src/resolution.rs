use core::fmt;
use std::sync::Arc;

use world_context::{ContainmentTransferActionDefinitions, RelocationActionDefinitions};
use world_core::SimMoment;
use world_defs::{
    ArtifactError, ArtifactValidator, DefinitionKey, DefinitionLinker, EngineProtocolVersion,
    ExactPackSet, ExactPackageSelection, LinkError, PackCoordinate, PackLock, PackLockDigest,
    PackSetError, RuntimeDefinitionSet, RuntimeDefinitionSetDigest, SelectedPackage,
};
use world_model::{AcceptedState, ActionOpportunity};
use world_runtime::{
    ActionPolicyExecutionV1, ActivatedRuntimeExecution, DeferredActionControlV1, EpochLineageId,
    ExecutionConfigArtifactV3, ExecutionSemanticsManifestDigest, ExecutionSpecId,
    InitialStateRootId, LifecycleProfilesV2, OriginExecutionInput,
    ResolvedExecutionClosureManifestDigest, RootSeed, RuntimeActivationError,
    TerminationContractV1,
};

use crate::action::DeferredActionEvaluatorDescriptor;
use crate::artifact::{ArtifactResolveError, ArtifactResolver};
use crate::distribution::{InstalledLifecycleImplementations, LifecycleResolutionError};
use crate::engine::EngineBinding;
use crate::routing::PostCommitRouter;

/// Unactivated input for one exact origin execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionSpecInput {
    lock: PackLock,
    lifecycle_profiles: LifecycleProfilesV2,
    origin: OriginExecutionInput,
}

/// Checked world and lifecycle values from which an origin execution starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionOrigin {
    accepted: AcceptedState,
    action_opportunities: Vec<ActionOpportunity>,
    now: SimMoment,
    admission_frontier: SimMoment,
}

impl ExecutionOrigin {
    /// Groups the physical origin, open actor opportunities, and initial clock.
    #[must_use]
    pub fn new(
        accepted: AcceptedState,
        action_opportunities: Vec<ActionOpportunity>,
        now: SimMoment,
        admission_frontier: SimMoment,
    ) -> Self {
        Self {
            accepted,
            action_opportunities,
            now,
            admission_frontier,
        }
    }
}

impl ExecutionSpecInput {
    /// Binds an exact compiled package lock to checked initial world inputs.
    #[must_use]
    pub fn origin(
        lock: PackLock,
        origin: ExecutionOrigin,
        lifecycle_profiles: LifecycleProfilesV2,
        config: ExecutionConfigArtifactV3,
        root_seed: RootSeed,
        termination: TerminationContractV1,
    ) -> Self {
        let ExecutionOrigin {
            accepted,
            action_opportunities,
            now,
            admission_frontier,
        } = origin;
        Self {
            lock,
            lifecycle_profiles,
            origin: OriginExecutionInput::new(
                now,
                admission_frontier,
                accepted,
                action_opportunities,
                config,
                root_seed,
                termination,
            ),
        }
    }

    pub(crate) fn into_parts(self) -> (PackLock, LifecycleProfilesV2, OriginExecutionInput) {
        (self.lock, self.lifecycle_profiles, self.origin)
    }
}

/// Sealed immutable execution semantics resolved by one engine.
#[derive(Clone)]
pub struct ResolvedExecution {
    pub(crate) inner: Arc<ResolvedExecutionInner>,
}

pub(crate) struct ResolvedExecutionInner {
    pub(crate) engine: Arc<EngineBinding>,
    pub(crate) activation: ActivatedRuntimeExecution,
    pub(crate) post_commit_router: PostCommitRouter,
    pub(crate) definitions: RuntimeDefinitionSet,
    pub(crate) containment_actions: Option<ContainmentTransferActionDefinitions>,
    pub(crate) relocation_actions: Option<RelocationActionDefinitions>,
    pub(crate) lifecycle: InstalledLifecycleImplementations,
}

impl ResolvedExecution {
    /// Returns the exact execution-specification identity.
    #[must_use]
    pub fn execution_id(&self) -> ExecutionSpecId {
        self.inner.activation.execution_id()
    }

    /// Returns the exact initial-state-root identity.
    #[must_use]
    pub fn initial_root_id(&self) -> InitialStateRootId {
        self.inner.activation.initial_root_id()
    }

    /// Returns the semantic lineage identity of the resolved origin.
    #[must_use]
    pub fn epoch_lineage_id(&self) -> EpochLineageId {
        self.inner.activation.epoch_lineage_id()
    }

    /// Returns the normalized behavior-affecting semantics identity.
    #[must_use]
    pub fn semantics_digest(&self) -> ExecutionSemanticsManifestDigest {
        self.inner.activation.semantics_digest()
    }

    /// Returns the exact linked definition-set identity.
    #[must_use]
    pub fn definition_set_digest(&self) -> RuntimeDefinitionSetDigest {
        self.inner.activation.definition_set_digest()
    }

    /// Returns the exact package-lock identity used for resolution.
    #[must_use]
    pub fn pack_lock_digest(&self) -> PackLockDigest {
        self.inner.activation.pack_lock_digest()
    }

    /// Returns the complete resolved execution-closure identity.
    #[must_use]
    pub fn closure_digest(&self) -> ResolvedExecutionClosureManifestDigest {
        self.inner.activation.closure_digest()
    }

    /// Returns the immutable action-policy behavior identity selected for this execution.
    #[must_use]
    pub fn action_policy_semantics(&self) -> world_context::ActionPolicySemanticsId {
        self.inner.lifecycle.action_execution().semantics_id()
    }

    /// Returns how the selected action evaluator crosses its execution
    /// boundary.
    #[must_use]
    pub fn action_policy_execution(&self) -> ActionPolicyExecutionV1 {
        self.inner.lifecycle.action_execution().execution_class()
    }

    /// Returns the exact deferred protocol descriptor when this execution
    /// selected durable action evaluation.
    #[must_use]
    pub fn deferred_action_evaluator(&self) -> Option<DeferredActionEvaluatorDescriptor> {
        self.inner
            .lifecycle
            .action_execution()
            .deferred_descriptor()
    }

    /// Returns the exact closed lifecycle selection sealed into this execution.
    #[must_use]
    pub fn lifecycle_profiles(&self) -> LifecycleProfilesV2 {
        self.inner.lifecycle.profiles()
    }
}

/// Why exact artifact input could not become a sealed execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolveExecutionError {
    /// The host resolver failed for one exact package.
    ArtifactResolve {
        /// Package whose artifact was requested.
        package: PackCoordinate,
        /// Read-only resolver outcome.
        error: ArtifactResolveError,
    },
    /// Returned bytes failed defs-owned decoding or validation.
    ArtifactValidation {
        /// Package expected by the lock entry.
        package: PackCoordinate,
        /// Owner-local validation failure.
        error: Box<ArtifactError>,
    },
    /// Resolved artifacts did not form the exact selected package set.
    PackSet(Box<PackSetError>),
    /// Exact definitions did not link.
    Link(Box<LinkError>),
    /// Reconstructed package provenance did not reproduce the supplied lock.
    PackLockMismatch {
        /// Lock supplied to execution resolution.
        expected: PackLockDigest,
        /// Lock reconstructed from validated artifacts.
        actual: PackLockDigest,
    },
    /// One execution closure required more than one containment interface.
    MultipleContainmentInterfaces,
    /// One execution closure required more than one relocation interface.
    MultipleRelocationInterfaces,
    /// The selected lifecycle profile does not match installed capabilities.
    Lifecycle(LifecycleResolutionError),
    /// The exact root, semantics, or implementation binding could not be activated.
    Activation(ExecutionActivationError),
}

/// Stable engine-facing reason an exact execution closure could not be activated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionActivationError {
    /// Action-policy execution and deferred control select incompatible modes.
    ActionExecutionControlMismatch {
        /// Execution boundary selected by the lifecycle action profile.
        execution: ActionPolicyExecutionV1,
        /// Deferred action control selected by execution configuration.
        control: DeferredActionControlV1,
    },
    /// The linked definitions require an engine protocol this installation does not interpret.
    UnsupportedEngineProtocol {
        /// Protocol required by the exact definition set.
        required: EngineProtocolVersion,
        /// Protocol implemented by this installation.
        supported: EngineProtocolVersion,
    },
    /// The definition closure does not select exactly the installed semantic interface.
    SemanticClosureMismatch,
    /// A definition cannot be lowered into the installed semantic family.
    UnsupportedAction {
        /// Exact action rejected during activation.
        action: DefinitionKey,
    },
    /// More than one action invoked one closed relocation operation.
    DuplicateRelocationAction,
    /// One closed relocation operation had no authored action.
    MissingRelocationAction,
    /// Runtime's activated relocation family could not reproduce the
    /// context-owned checked action binding.
    RelocationActionBindingMismatch,
    /// Runtime's activated containment family could not reproduce the
    /// context-owned checked action binding.
    ContainmentActionBindingMismatch,
    /// The exact definition set contains no executable action.
    NoExecutableAction,
    /// The initial accepted state violates a runtime-owned invariant.
    InvalidInitialRoot,
    /// Installed implementations do not close the definition requirements.
    SemanticBindingMismatch,
    /// The root, semantics, and specification do not form one execution.
    InitialBindingMismatch,
}

impl fmt::Display for ExecutionActivationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "execution activation failed: {self:?}")
    }
}

impl std::error::Error for ExecutionActivationError {}

impl fmt::Display for ResolveExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArtifactResolve { package, error } => {
                write!(formatter, "cannot resolve artifact for {package}: {error}")
            }
            Self::ArtifactValidation { package, error } => {
                write!(formatter, "artifact for {package} is invalid: {error}")
            }
            Self::PackSet(error) => write!(formatter, "resolved package set is invalid: {error}"),
            Self::Link(error) => write!(formatter, "resolved definitions do not link: {error}"),
            Self::PackLockMismatch { expected, actual } => write!(
                formatter,
                "resolved package lock {actual} does not match requested lock {expected}"
            ),
            Self::MultipleContainmentInterfaces => {
                formatter.write_str("execution requires more than one containment interface")
            }
            Self::MultipleRelocationInterfaces => {
                formatter.write_str("execution requires more than one relocation interface")
            }
            Self::Lifecycle(error) => error.fmt(formatter),
            Self::Activation(error) => write!(formatter, "execution activation failed: {error:?}"),
        }
    }
}

impl std::error::Error for ResolveExecutionError {}

pub(crate) fn map_activation_error(error: RuntimeActivationError) -> ExecutionActivationError {
    match error {
        RuntimeActivationError::ActionExecutionControlMismatch { execution, control } => {
            ExecutionActivationError::ActionExecutionControlMismatch { execution, control }
        }
        RuntimeActivationError::UnsupportedEngineProtocol {
            required,
            supported,
        } => ExecutionActivationError::UnsupportedEngineProtocol {
            required,
            supported,
        },
        RuntimeActivationError::SemanticClosureMismatch => {
            ExecutionActivationError::SemanticClosureMismatch
        }
        RuntimeActivationError::UnsupportedAction { action } => {
            ExecutionActivationError::UnsupportedAction { action }
        }
        RuntimeActivationError::DuplicateRelocationAction { .. } => {
            ExecutionActivationError::DuplicateRelocationAction
        }
        RuntimeActivationError::MissingRelocationAction { .. } => {
            ExecutionActivationError::MissingRelocationAction
        }
        RuntimeActivationError::NoExecutableAction => ExecutionActivationError::NoExecutableAction,
        RuntimeActivationError::InitialRoot(_) => ExecutionActivationError::InvalidInitialRoot,
        RuntimeActivationError::SemanticBinding(_) => {
            ExecutionActivationError::SemanticBindingMismatch
        }
        RuntimeActivationError::InitialBinding(_) => {
            ExecutionActivationError::InitialBindingMismatch
        }
    }
}

pub(crate) fn resolve_definitions(
    resolver: &dyn ArtifactResolver,
    catalog: &world_defs::SemanticInterfaceCatalog,
    lock: PackLock,
) -> Result<RuntimeDefinitionSet, ResolveExecutionError> {
    let validator = ArtifactValidator::new(catalog);
    let mut artifacts = Vec::with_capacity(lock.entries().len());
    for entry in lock.entries() {
        let envelope =
            resolver
                .resolve(entry)
                .map_err(|error| ResolveExecutionError::ArtifactResolve {
                    package: entry.coordinate().clone(),
                    error,
                })?;
        let artifact = validator.load(envelope).map_err(|error| {
            ResolveExecutionError::ArtifactValidation {
                package: entry.coordinate().clone(),
                error: Box::new(error),
            }
        })?;
        artifacts.push(artifact);
    }

    let selection = ExactPackageSelection::new(
        lock.root().clone(),
        lock.entries()
            .iter()
            .map(|entry| {
                SelectedPackage::new(
                    entry.coordinate().clone(),
                    entry.source_snapshot(),
                    entry
                        .dependencies()
                        .iter()
                        .map(|dependency| dependency.coordinate().clone())
                        .collect(),
                )
            })
            .collect(),
    );
    let exact = ExactPackSet::finalize(selection, artifacts)
        .map_err(|error| ResolveExecutionError::PackSet(Box::new(error)))?;
    let definitions = DefinitionLinker::link(exact)
        .map_err(|error| ResolveExecutionError::Link(Box::new(error)))?;
    if definitions.lock() != &lock {
        return Err(ResolveExecutionError::PackLockMismatch {
            expected: lock.digest(),
            actual: definitions.lock().digest(),
        });
    }
    Ok(definitions)
}

#[cfg(test)]
mod tests {
    use core::fmt::Debug;
    use std::sync::Arc;

    use world_decision::BaselineActionPolicy;
    use world_defs::{
        ActionBindingData, ActionData, ArtifactData, ArtifactEnvelope, BindingName, CatalogError,
        DefinitionKey, DefinitionLinker, EffectCallData, EngineProtocolVersion, EventData,
        EventEmissionData, EventFieldBindingData, EventFieldData, EventFieldName, ExactPackSet,
        ExactPackageSelection, InterfaceVersion, LocalDefinitionName, OperationCallData,
        OperationKind, OperationName, OperationParameter, PackCoordinate, PackKey, PackLockEntry,
        PackManifestData, PackVersion, ParameterName, RuntimeRequirementData, SelectedPackage,
        SemanticInterfaceCatalog, SemanticInterfaceDescriptor, SemanticInterfaceKey,
        SemanticOperationDescriptor, SourceSnapshotId, ValueKind,
    };
    use world_model::{AgencyState, DomainState, EpistemicState, SocialState};
    use world_runtime::{
        ContainmentTransferEvaluation, ContainmentTransferImplementation, ContainmentTransferInput,
        DeferredActionAdmissionModeV1, LifecycleBindingV1, LifecycleImplementationId,
        LifecycleProfilesV2, RelocationActionImplementation, RuntimeService,
        SemanticImplementationId,
    };

    use crate::artifact::{ArtifactResolveError, ArtifactResolver};
    use crate::distribution::{
        ActionPolicyInstallation, EngineDistribution, LifecycleImplementationSet,
        baseline_lifecycle_profiles,
    };
    use crate::engine::EngineBuilder;

    use super::*;

    fn valid<T, E: Debug>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("resolution fixture must be valid: {error:?}"),
        }
    }

    #[test]
    fn activation_control_mismatch_preserves_exact_engine_facing_details() {
        let control = valid(DeferredActionControlV1::enabled(
            world_runtime::DeferredActionAdmissionModeV1::HostScheduled,
            2,
            512,
            256,
            128,
            64,
        ));
        let execution = ActionPolicyExecutionV1::InlineDeterministic;

        assert_eq!(
            map_activation_error(RuntimeActivationError::ActionExecutionControlMismatch {
                execution,
                control,
            }),
            ExecutionActivationError::ActionExecutionControlMismatch { execution, control }
        );
    }

    #[test]
    fn deferred_action_descriptor_survives_distribution_and_execution_resolution() {
        let interface = interface_descriptor("test.deferred.interface");
        let (coordinate, artifact) = required_artifact(&interface);
        let selection = ExactPackageSelection::new(
            coordinate.clone(),
            vec![SelectedPackage::new(
                coordinate,
                SourceSnapshotId::from_bytes([0x2f; 32]),
                Vec::new(),
            )],
        );
        let definitions = valid(DefinitionLinker::link(valid(ExactPackSet::finalize(
            selection,
            vec![artifact.clone()],
        ))));
        let transfer = valid(ContainmentTransferImplementation::new(
            interface,
            SemanticImplementationId::from_bytes([0x3f; 32]),
            never_accepts,
        ));
        let descriptor = DeferredActionEvaluatorDescriptor::new(
            world_context::ActionPolicySemanticsId::from_bytes([0x4f; 32]),
        );
        let action = ActionPolicyInstallation::deferred_captured(descriptor);
        let profiles = baseline_lifecycle_profiles(action.binding());
        let lifecycle = valid(LifecycleImplementationSet::baseline(vec![action]));
        let distribution = valid(EngineDistribution::new(
            vec![transfer],
            Vec::new(),
            lifecycle,
        ));
        let engine = valid(
            EngineBuilder::new(
                distribution,
                Arc::new(FixedArtifactResolver {
                    envelope: artifact.envelope().clone(),
                }),
                valid(RuntimeService::in_memory()),
            )
            .build(),
        );
        let accepted = AcceptedState::new(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );
        let control = valid(DeferredActionControlV1::enabled(
            DeferredActionAdmissionModeV1::FrontierBlocking,
            0,
            1024,
            512,
            2048,
            256,
        ));

        let resolved = valid(engine.resolve_execution(ExecutionSpecInput::origin(
            definitions.lock().clone(),
            ExecutionOrigin::new(accepted, Vec::new(), SimMoment::ORIGIN, SimMoment::ORIGIN),
            profiles,
            valid(ExecutionConfigArtifactV3::deferred(64, 32, 16, control)),
            RootSeed::from_bytes([0x5f; 32]),
            TerminationContractV1::never(),
        )));

        assert_eq!(
            resolved.action_policy_execution(),
            ActionPolicyExecutionV1::DeferredCaptured
        );
        assert_eq!(
            resolved.action_policy_semantics(),
            descriptor.semantics_id()
        );
        assert_eq!(resolved.deferred_action_evaluator(), Some(descriptor));
    }

    fn interface_descriptor(key: &str) -> SemanticInterfaceDescriptor {
        interface_descriptor_with_predicate(key, "allowed")
    }

    fn interface_descriptor_with_predicate(
        key: &str,
        predicate: &str,
    ) -> SemanticInterfaceDescriptor {
        let parameters = || {
            vec![
                OperationParameter::new(valid(ParameterName::parse("actor")), ValueKind::Actor),
                OperationParameter::new(valid(ParameterName::parse("item")), ValueKind::Entity),
                OperationParameter::new(valid(ParameterName::parse("source")), ValueKind::Entity),
                OperationParameter::new(
                    valid(ParameterName::parse("destination")),
                    ValueKind::Entity,
                ),
            ]
        };
        valid(SemanticInterfaceDescriptor::new(
            valid(SemanticInterfaceKey::parse(key)),
            valid(InterfaceVersion::new(1)),
            vec![
                valid(SemanticOperationDescriptor::new(
                    valid(OperationName::parse(predicate)),
                    OperationKind::Predicate,
                    parameters(),
                )),
                valid(SemanticOperationDescriptor::new(
                    valid(OperationName::parse("apply")),
                    OperationKind::Effect,
                    parameters(),
                )),
            ],
        ))
    }

    fn required_artifact(
        descriptor: &SemanticInterfaceDescriptor,
    ) -> (PackCoordinate, world_defs::VerifiedPackArtifact) {
        let coordinate = PackCoordinate::new(
            valid(PackKey::parse("test.required")),
            PackVersion::new(1, 0, 0),
        );
        let actor = valid(BindingName::parse("actor"));
        let item = valid(BindingName::parse("item"));
        let source = valid(BindingName::parse("source"));
        let destination = valid(BindingName::parse("destination"));
        let arguments = vec![
            actor.clone(),
            item.clone(),
            source.clone(),
            destination.clone(),
        ];
        let requirement = RuntimeRequirementData::new(OperationCallData::new(
            descriptor.key().clone(),
            valid(OperationName::parse("allowed")),
            arguments.clone(),
        ));
        let effect = EffectCallData::new(OperationCallData::new(
            descriptor.key().clone(),
            valid(OperationName::parse("apply")),
            arguments,
        ));
        let event_name = valid(LocalDefinitionName::parse("applied"));
        let event = EventData::new(
            event_name.clone(),
            vec![
                EventFieldData::new(valid(EventFieldName::parse("actor")), ValueKind::Actor),
                EventFieldData::new(valid(EventFieldName::parse("item")), ValueKind::Entity),
                EventFieldData::new(valid(EventFieldName::parse("source")), ValueKind::Entity),
                EventFieldData::new(
                    valid(EventFieldName::parse("destination")),
                    ValueKind::Entity,
                ),
            ],
        );
        let emission = EventEmissionData::new(
            DefinitionKey::new(coordinate.pack_key().clone(), event_name),
            vec![
                EventFieldBindingData::new(valid(EventFieldName::parse("actor")), actor.clone()),
                EventFieldBindingData::new(valid(EventFieldName::parse("item")), item.clone()),
                EventFieldBindingData::new(valid(EventFieldName::parse("source")), source.clone()),
                EventFieldBindingData::new(
                    valid(EventFieldName::parse("destination")),
                    destination.clone(),
                ),
            ],
        );
        let action = ActionData::new(
            valid(LocalDefinitionName::parse("apply")),
            vec![
                ActionBindingData::new(actor, ValueKind::Actor),
                ActionBindingData::new(item, ValueKind::Entity),
                ActionBindingData::new(source, ValueKind::Entity),
                ActionBindingData::new(destination, ValueKind::Entity),
            ],
            vec![requirement],
            vec![effect],
            vec![emission],
        );
        let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor.clone()]));
        let artifact = valid(ArtifactValidator::new(&catalog).validate(ArtifactData::new(
            PackManifestData::new(
                EngineProtocolVersion::new(1),
                coordinate.clone(),
                Vec::new(),
            ),
            vec![descriptor.reference()],
            vec![action],
            vec![event],
        )));
        (coordinate, artifact)
    }

    fn relocation_descriptor(key: &str) -> SemanticInterfaceDescriptor {
        let parameters = || {
            vec![
                OperationParameter::new(valid(ParameterName::parse("actor")), ValueKind::Actor),
                OperationParameter::new(
                    valid(ParameterName::parse("destination")),
                    ValueKind::Entity,
                ),
                OperationParameter::new(valid(ParameterName::parse("source")), ValueKind::Entity),
            ]
        };
        valid(SemanticInterfaceDescriptor::new(
            valid(SemanticInterfaceKey::parse(key)),
            valid(InterfaceVersion::new(1)),
            ["start", "pause", "resume"]
                .into_iter()
                .map(|operation| {
                    valid(SemanticOperationDescriptor::new(
                        valid(OperationName::parse(operation)),
                        OperationKind::Effect,
                        parameters(),
                    ))
                })
                .collect(),
        ))
    }

    fn relocation_artifact(
        descriptor: &SemanticInterfaceDescriptor,
    ) -> (PackCoordinate, world_defs::VerifiedPackArtifact) {
        let coordinate = PackCoordinate::new(
            valid(PackKey::parse("test.relocation")),
            PackVersion::new(1, 0, 0),
        );
        let actor = valid(BindingName::parse("actor"));
        let destination = valid(BindingName::parse("destination"));
        let source = valid(BindingName::parse("source"));
        let bindings = vec![
            ActionBindingData::new(actor.clone(), ValueKind::Actor),
            ActionBindingData::new(destination.clone(), ValueKind::Entity),
            ActionBindingData::new(source.clone(), ValueKind::Entity),
        ];
        let arguments = vec![actor.clone(), destination.clone(), source.clone()];
        let mut actions = Vec::new();
        let mut events = Vec::new();
        for operation in ["start", "pause", "resume"] {
            let event_name = valid(LocalDefinitionName::parse(&format!("{operation}-event")));
            events.push(EventData::new(
                event_name.clone(),
                vec![
                    EventFieldData::new(valid(EventFieldName::parse("actor")), ValueKind::Actor),
                    EventFieldData::new(
                        valid(EventFieldName::parse("destination")),
                        ValueKind::Entity,
                    ),
                    EventFieldData::new(valid(EventFieldName::parse("source")), ValueKind::Entity),
                ],
            ));
            actions.push(ActionData::new(
                valid(LocalDefinitionName::parse(operation)),
                bindings.clone(),
                Vec::new(),
                vec![EffectCallData::new(OperationCallData::new(
                    descriptor.key().clone(),
                    valid(OperationName::parse(operation)),
                    arguments.clone(),
                ))],
                vec![EventEmissionData::new(
                    DefinitionKey::new(coordinate.pack_key().clone(), event_name),
                    vec![
                        EventFieldBindingData::new(
                            valid(EventFieldName::parse("actor")),
                            actor.clone(),
                        ),
                        EventFieldBindingData::new(
                            valid(EventFieldName::parse("destination")),
                            destination.clone(),
                        ),
                        EventFieldBindingData::new(
                            valid(EventFieldName::parse("source")),
                            source.clone(),
                        ),
                    ],
                )],
            ));
        }
        let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor.clone()]));
        let artifact = valid(ArtifactValidator::new(&catalog).validate(ArtifactData::new(
            PackManifestData::new(
                EngineProtocolVersion::new(1),
                coordinate.clone(),
                Vec::new(),
            ),
            vec![descriptor.reference()],
            actions,
            events,
        )));
        (coordinate, artifact)
    }

    fn never_accepts(_: ContainmentTransferInput<'_>) -> ContainmentTransferEvaluation {
        ContainmentTransferEvaluation::RequirementUnsatisfied
    }

    fn lifecycle() -> (LifecycleImplementationSet, LifecycleProfilesV2) {
        let action =
            ActionPolicyInstallation::inline_deterministic(Arc::new(BaselineActionPolicy::new()));
        let profiles = baseline_lifecycle_profiles(action.binding());
        (
            valid(LifecycleImplementationSet::baseline(vec![action])),
            profiles,
        )
    }

    struct FixedArtifactResolver {
        envelope: ArtifactEnvelope,
    }

    impl ArtifactResolver for FixedArtifactResolver {
        fn resolve(
            &self,
            _reference: &PackLockEntry,
        ) -> Result<ArtifactEnvelope, ArtifactResolveError> {
            Ok(self.envelope.clone())
        }
    }

    struct PanicArtifactResolver;

    impl ArtifactResolver for PanicArtifactResolver {
        fn resolve(
            &self,
            _reference: &PackLockEntry,
        ) -> Result<ArtifactEnvelope, ArtifactResolveError> {
            panic!("artifact resolution must not run for an invalid lifecycle selection")
        }
    }

    #[test]
    fn invalid_lifecycle_selection_fails_before_artifact_resolution_or_activation() {
        let descriptor = interface_descriptor("test.required.interface");
        let (coordinate, artifact) = required_artifact(&descriptor);
        let selection = ExactPackageSelection::new(
            coordinate.clone(),
            vec![SelectedPackage::new(
                coordinate,
                SourceSnapshotId::from_bytes([0x30; 32]),
                Vec::new(),
            )],
        );
        let definitions = valid(DefinitionLinker::link(valid(ExactPackSet::finalize(
            selection,
            vec![artifact],
        ))));
        let installed = valid(ContainmentTransferImplementation::new(
            descriptor,
            SemanticImplementationId::from_bytes([0x40; 32]),
            never_accepts,
        ));
        let (lifecycle, baseline) = lifecycle();
        let distribution = valid(EngineDistribution::new(
            vec![installed],
            Vec::new(),
            lifecycle,
        ));
        let engine = valid(
            EngineBuilder::new(
                distribution,
                Arc::new(PanicArtifactResolver),
                valid(RuntimeService::in_memory()),
            )
            .build(),
        );
        let unknown = LifecycleImplementationId::from_bytes([0x50; 32]);
        let profiles = LifecycleProfilesV2::new(
            LifecycleBindingV1::stateless(unknown),
            baseline.appraisal(),
            baseline.social(),
            baseline.intent(),
            baseline.activity(),
            baseline.action(),
        );
        let accepted = AcceptedState::new(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );

        let result = engine.resolve_execution(ExecutionSpecInput::origin(
            definitions.lock().clone(),
            ExecutionOrigin::new(accepted, Vec::new(), SimMoment::ORIGIN, SimMoment::ORIGIN),
            profiles,
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            RootSeed::from_bytes([0x60; 32]),
            TerminationContractV1::never(),
        ));

        assert!(matches!(
            result,
            Err(ResolveExecutionError::Lifecycle(
                LifecycleResolutionError::UnknownImplementation {
                    port: crate::LifecyclePort::Evidence,
                    implementation,
                }
            )) if implementation == unknown
        ));
    }

    #[test]
    fn missing_installed_interface_fails_before_activation_or_attempt_construction() {
        let required_descriptor = interface_descriptor("test.required.interface");
        let required_reference = required_descriptor.reference();
        let (coordinate, artifact) = required_artifact(&required_descriptor);
        let selection = ExactPackageSelection::new(
            coordinate.clone(),
            vec![SelectedPackage::new(
                coordinate.clone(),
                SourceSnapshotId::from_bytes([0x31; 32]),
                Vec::new(),
            )],
        );
        let definitions = valid(DefinitionLinker::link(valid(ExactPackSet::finalize(
            selection,
            vec![artifact.clone()],
        ))));

        let installed = valid(ContainmentTransferImplementation::new(
            interface_descriptor("test.installed.interface"),
            SemanticImplementationId::from_bytes([0x41; 32]),
            never_accepts,
        ));
        let (lifecycle, profiles) = lifecycle();
        let distribution = valid(EngineDistribution::new(
            vec![installed],
            Vec::new(),
            lifecycle,
        ));
        let engine = valid(
            EngineBuilder::new(
                distribution,
                Arc::new(FixedArtifactResolver {
                    envelope: artifact.envelope().clone(),
                }),
                valid(RuntimeService::in_memory()),
            )
            .build(),
        );
        let accepted = AcceptedState::new(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );

        let result = engine.resolve_execution(ExecutionSpecInput::origin(
            definitions.lock().clone(),
            ExecutionOrigin::new(accepted, Vec::new(), SimMoment::ORIGIN, SimMoment::ORIGIN),
            profiles,
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            RootSeed::from_bytes([0x51; 32]),
            TerminationContractV1::never(),
        ));
        let error = match result {
            Ok(_) => panic!("execution resolved without its required installed interface"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            ResolveExecutionError::ArtifactValidation {
                package: coordinate,
                error: Box::new(ArtifactError::InterfaceCatalog(
                    CatalogError::MissingInterface {
                        key: required_reference.key().clone(),
                        version: required_reference.version(),
                    }
                )),
            }
        );
    }

    #[test]
    fn mismatched_installed_interface_fails_before_activation_or_attempt_construction() {
        let required_descriptor = interface_descriptor("test.required.interface");
        let required_reference = required_descriptor.reference();
        let (coordinate, artifact) = required_artifact(&required_descriptor);
        let selection = ExactPackageSelection::new(
            coordinate.clone(),
            vec![SelectedPackage::new(
                coordinate.clone(),
                SourceSnapshotId::from_bytes([0x32; 32]),
                Vec::new(),
            )],
        );
        let definitions = valid(DefinitionLinker::link(valid(ExactPackSet::finalize(
            selection,
            vec![artifact.clone()],
        ))));

        let installed_descriptor =
            interface_descriptor_with_predicate("test.required.interface", "permitted");
        let installed_reference = installed_descriptor.reference();
        assert_eq!(required_reference.key(), installed_reference.key());
        assert_eq!(required_reference.version(), installed_reference.version());
        assert_ne!(required_reference.digest(), installed_reference.digest());
        let installed = valid(ContainmentTransferImplementation::new(
            installed_descriptor,
            SemanticImplementationId::from_bytes([0x42; 32]),
            never_accepts,
        ));
        let (lifecycle, profiles) = lifecycle();
        let distribution = valid(EngineDistribution::new(
            vec![installed],
            Vec::new(),
            lifecycle,
        ));
        let engine = valid(
            EngineBuilder::new(
                distribution,
                Arc::new(FixedArtifactResolver {
                    envelope: artifact.envelope().clone(),
                }),
                valid(RuntimeService::in_memory()),
            )
            .build(),
        );
        let accepted = AcceptedState::new(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );

        let result = engine.resolve_execution(ExecutionSpecInput::origin(
            definitions.lock().clone(),
            ExecutionOrigin::new(accepted, Vec::new(), SimMoment::ORIGIN, SimMoment::ORIGIN),
            profiles,
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            RootSeed::from_bytes([0x52; 32]),
            TerminationContractV1::never(),
        ));
        let error = match result {
            Ok(_) => panic!("execution resolved against a mismatched interface descriptor"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            ResolveExecutionError::ArtifactValidation {
                package: coordinate,
                error: Box::new(ArtifactError::InterfaceCatalog(
                    CatalogError::DigestMismatch {
                        key: required_reference.key().clone(),
                        version: required_reference.version(),
                        expected: required_reference.digest(),
                        actual: installed_reference.digest(),
                    }
                )),
            }
        );
    }

    #[test]
    fn relocation_roles_are_resolved_once_and_sealed_into_execution_semantics() {
        let descriptor = relocation_descriptor("test.relocation.interface");
        let (coordinate, artifact) = relocation_artifact(&descriptor);
        let selection = ExactPackageSelection::new(
            coordinate.clone(),
            vec![SelectedPackage::new(
                coordinate,
                SourceSnapshotId::from_bytes([0x33; 32]),
                Vec::new(),
            )],
        );
        let definitions = valid(DefinitionLinker::link(valid(ExactPackSet::finalize(
            selection,
            vec![artifact.clone()],
        ))));
        let start = valid(OperationName::parse("start"));
        let pause = valid(OperationName::parse("pause"));
        let resume = valid(OperationName::parse("resume"));
        let resolve = |start, pause, resume| {
            let installed = valid(RelocationActionImplementation::new(
                descriptor.clone(),
                SemanticImplementationId::from_bytes([0x43; 32]),
                start,
                pause,
                resume,
            ));
            let (lifecycle, profiles) = lifecycle();
            let distribution = valid(EngineDistribution::new(
                Vec::new(),
                vec![installed],
                lifecycle,
            ));
            let engine = valid(
                EngineBuilder::new(
                    distribution,
                    Arc::new(FixedArtifactResolver {
                        envelope: artifact.envelope().clone(),
                    }),
                    valid(RuntimeService::in_memory()),
                )
                .build(),
            );
            let accepted = AcceptedState::new(
                valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
                EpistemicState::empty(),
                SocialState::empty(),
                AgencyState::empty(),
            );
            valid(engine.resolve_execution(ExecutionSpecInput::origin(
                definitions.lock().clone(),
                ExecutionOrigin::new(accepted, Vec::new(), SimMoment::ORIGIN, SimMoment::ORIGIN),
                profiles,
                valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
                RootSeed::from_bytes([0x53; 32]),
                TerminationContractV1::never(),
            )))
        };

        let conventional = resolve(start.clone(), pause.clone(), resume.clone());
        let swapped = resolve(pause, start, resume);
        let conventional_actions = conventional
            .inner
            .relocation_actions
            .as_ref()
            .unwrap_or_else(|| panic!("relocation closure must bind its checked actions"));
        let swapped_actions = swapped
            .inner
            .relocation_actions
            .as_ref()
            .unwrap_or_else(|| panic!("relocation closure must bind its checked actions"));

        assert_eq!(conventional_actions.start().local_name().as_str(), "start");
        assert_eq!(conventional_actions.pause().local_name().as_str(), "pause");
        assert_eq!(swapped_actions.start().local_name().as_str(), "pause");
        assert_eq!(swapped_actions.pause().local_name().as_str(), "start");
        assert_ne!(
            conventional.semantics_digest(),
            swapped.semantics_digest(),
            "normalized semantics must commit to the relocation role assignment"
        );
    }
}
