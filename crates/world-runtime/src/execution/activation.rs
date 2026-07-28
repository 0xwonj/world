use core::fmt;
use std::sync::Arc;

use world_core::{ActorId, CanonicalDomain, CanonicalWriter, ContentDigest, EntityId, SimMoment};
use world_defs::{
    ActionDefinition, BindingName, DefinitionKey, EngineProtocolVersion, OperationKind,
    OperationName, PackLockDigest, RuntimeDefinitionSet, RuntimeDefinitionSetDigest,
    SemanticInterfaceDescriptor, SemanticInterfaceReference, ValueKind,
};
use world_model::{
    AcceptedState, ActionOpportunity, CommandEnvelope, CommandValue, ContainmentTransferDelta,
    ContainmentTransferReadView, StableCommandRejection, WorldSnapshot,
};

use crate::session::SessionMode;

use super::{
    ActionPolicyExecutionV1, CanonicalExecutionSpecV1, DeferredActionControlV1,
    ExecutionConfigArtifactV3, ExecutionSemanticsManifestV1, ExecutionSpecId,
    ExternalInputBindingV1, InitialExecutionBindingError, InitialRootError, InitialStateRootId,
    InitialStateRootV1, LifecycleProfilesV2, PostCommitRoutingPolicyV1,
    ResolvedExecutionClosureManifestDigest, ResolvedExecutionClosureManifestV1, RootSeed,
    SemanticBindingError, SemanticImplementationBinding, SemanticImplementationId,
    TerminationContractV1,
};

const ACTOR_ROLE: &str = "actor";
const ITEM_ROLE: &str = "item";
const SOURCE_ROLE: &str = "source";
const DESTINATION_ROLE: &str = "destination";
const RELOCATION_IMPLEMENTATION_BINDING_SCHEMA_VERSION: u16 = 1;
const RELOCATION_IMPLEMENTATION_BINDING_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("relocation-implementation-binding-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("relocation implementation binding domain must be valid"),
    };

/// Engine protocol interpreted by this runtime implementation.
pub const SUPPORTED_ENGINE_PROTOCOL_VERSION: EngineProtocolVersion = EngineProtocolVersion::new(1);

/// Checked origin inputs whose canonical execution values are minted by runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginExecutionInput {
    now: SimMoment,
    admission_frontier: SimMoment,
    accepted_state: AcceptedState,
    action_opportunities: Vec<ActionOpportunity>,
    config: ExecutionConfigArtifactV3,
    root_seed: RootSeed,
    termination: TerminationContractV1,
}

impl OriginExecutionInput {
    /// Describes one origin execution without constructing its authoritative root.
    #[must_use]
    pub const fn new(
        now: SimMoment,
        admission_frontier: SimMoment,
        accepted_state: AcceptedState,
        action_opportunities: Vec<ActionOpportunity>,
        config: ExecutionConfigArtifactV3,
        root_seed: RootSeed,
        termination: TerminationContractV1,
    ) -> Self {
        Self {
            now,
            admission_frontier,
            accepted_state,
            action_opportunities,
            config,
            root_seed,
            termination,
        }
    }
}

/// Immutable, family-specific input supplied to trusted containment semantics.
///
/// Runtime constructs this value only after resolving a checked action through
/// the activated definition registry. It contains no repository, session head,
/// scheduler, record builder, or publication capability.
#[derive(Clone, Copy)]
pub struct ContainmentTransferInput<'world> {
    view: ContainmentTransferReadView<'world>,
}

impl<'world> ContainmentTransferInput<'world> {
    /// Returns the bounded immutable containment facts for this transfer.
    #[must_use]
    pub const fn view(self) -> ContainmentTransferReadView<'world> {
        self.view
    }

    /// Returns the actor whose hard source-container authority is required.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.view.actor()
    }

    /// Returns the item selected for transfer.
    #[must_use]
    pub const fn item(self) -> EntityId {
        self.view.item()
    }

    /// Returns the direct container expected in the base state.
    #[must_use]
    pub const fn source(self) -> EntityId {
        self.view.source()
    }

    /// Returns the proposed destination container.
    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.view.destination()
    }
}

/// Total result of trusted containment-transfer semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainmentTransferEvaluation {
    /// The declared authoritative requirement did not hold.
    RequirementUnsatisfied,
    /// The requirement held for the exact transfer roles supplied by runtime.
    Accepted,
}

/// Statically linked evaluator for the containment-transfer semantic family.
pub type ContainmentTransferEvaluator =
    for<'world> fn(ContainmentTransferInput<'world>) -> ContainmentTransferEvaluation;

/// Why an installation does not implement the typed containment contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainmentTransferInstallationError {
    /// The interface does not declare exactly one predicate and one effect.
    OperationFamilyMismatch,
    /// An operation does not use the canonical transfer role signature.
    OperationSignatureMismatch {
        /// Operation whose signature is incompatible.
        operation: OperationName,
    },
}

impl fmt::Display for ContainmentTransferInstallationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationFamilyMismatch => {
                formatter.write_str("the interface is not a containment-transfer operation family")
            }
            Self::OperationSignatureMismatch { operation } => write!(
                formatter,
                "operation {operation} does not use the containment-transfer signature"
            ),
        }
    }
}

impl std::error::Error for ContainmentTransferInstallationError {}

/// One immutable trusted implementation of a containment-transfer interface.
#[derive(Clone)]
pub struct ContainmentTransferImplementation {
    descriptor: SemanticInterfaceDescriptor,
    implementation: SemanticImplementationId,
    predicate: OperationName,
    effect: OperationName,
    evaluator: ContainmentTransferEvaluator,
}

impl ContainmentTransferImplementation {
    /// Checks and installs one statically linked typed implementation.
    pub fn new(
        descriptor: SemanticInterfaceDescriptor,
        implementation: SemanticImplementationId,
        evaluator: ContainmentTransferEvaluator,
    ) -> Result<Self, ContainmentTransferInstallationError> {
        let mut predicate = None;
        let mut effect = None;

        for operation in descriptor.operations() {
            if !is_transfer_signature(operation.parameters()) {
                return Err(
                    ContainmentTransferInstallationError::OperationSignatureMismatch {
                        operation: operation.name().clone(),
                    },
                );
            }
            let slot = match operation.kind() {
                OperationKind::Predicate => &mut predicate,
                OperationKind::Effect => &mut effect,
            };
            if slot.replace(operation.name().clone()).is_some() {
                return Err(ContainmentTransferInstallationError::OperationFamilyMismatch);
            }
        }

        let (Some(predicate), Some(effect)) = (predicate, effect) else {
            return Err(ContainmentTransferInstallationError::OperationFamilyMismatch);
        };
        if descriptor.operations().len() != 2 {
            return Err(ContainmentTransferInstallationError::OperationFamilyMismatch);
        }

        Ok(Self {
            descriptor,
            implementation,
            predicate,
            effect,
            evaluator,
        })
    }

    /// Returns the complete interface descriptor supplied to pack validation.
    #[must_use]
    pub const fn descriptor(&self) -> &SemanticInterfaceDescriptor {
        &self.descriptor
    }

    /// Returns the exact artifact-facing interface reference.
    #[must_use]
    pub fn interface(&self) -> SemanticInterfaceReference {
        self.descriptor.reference()
    }

    /// Returns the behavior-affecting implementation identity.
    #[must_use]
    pub const fn implementation_id(&self) -> SemanticImplementationId {
        self.implementation
    }

    fn evaluate(&self, input: ContainmentTransferInput<'_>) -> ContainmentTransferEvaluation {
        (self.evaluator)(input)
    }
}

/// Why an installation does not identify the closed relocation operation
/// family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelocationActionInstallationError {
    /// Start, pause, and resume must name three different operations.
    DuplicateOperation {
        /// Operation assigned to more than one relocation role.
        operation: OperationName,
    },
    /// One named relocation operation is absent from the descriptor.
    OperationUnavailable {
        /// Missing operation.
        operation: OperationName,
    },
    /// A relocation operation is not an effect with the exact typed role
    /// signature.
    OperationContractMismatch {
        /// Incompatible operation.
        operation: OperationName,
    },
    /// The descriptor contains operations outside the closed relocation
    /// family.
    OperationFamilyMismatch,
}

impl fmt::Display for RelocationActionInstallationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid relocation semantic installation: {self:?}"
        )
    }
}

impl std::error::Error for RelocationActionInstallationError {}

/// Statically installed semantics for the closed relocation action family.
///
/// The role assignment is included in the exposed implementation identity, so
/// swapping two operations changes normalized execution semantics even when
/// the interface descriptor itself is unchanged.
#[derive(Clone)]
pub struct RelocationActionImplementation {
    descriptor: SemanticInterfaceDescriptor,
    implementation: SemanticImplementationId,
    start: OperationName,
    pause: OperationName,
    resume: OperationName,
}

impl RelocationActionImplementation {
    /// Validates and binds exact interface operations to relocation roles.
    pub fn new(
        descriptor: SemanticInterfaceDescriptor,
        implementation: SemanticImplementationId,
        start: OperationName,
        pause: OperationName,
        resume: OperationName,
    ) -> Result<Self, RelocationActionInstallationError> {
        let mut operations = [&start, &pause, &resume];
        operations.sort();
        if let Some(operation) = operations
            .windows(2)
            .find_map(|pair| (pair[0] == pair[1]).then(|| pair[0].clone()))
        {
            return Err(RelocationActionInstallationError::DuplicateOperation { operation });
        }
        if descriptor.operations().len() != operations.len() {
            return Err(RelocationActionInstallationError::OperationFamilyMismatch);
        }
        for operation in [&start, &pause, &resume] {
            let Some(declaration) = descriptor
                .operations()
                .iter()
                .find(|declaration| declaration.name() == operation)
            else {
                return Err(RelocationActionInstallationError::OperationUnavailable {
                    operation: operation.clone(),
                });
            };
            if declaration.kind() != OperationKind::Effect
                || !is_relocation_signature(declaration.parameters())
            {
                return Err(
                    RelocationActionInstallationError::OperationContractMismatch {
                        operation: operation.clone(),
                    },
                );
            }
        }
        let implementation = relocation_implementation_binding_id(
            implementation,
            &descriptor,
            &start,
            &pause,
            &resume,
        );
        Ok(Self {
            descriptor,
            implementation,
            start,
            pause,
            resume,
        })
    }

    /// Returns the complete interface descriptor supplied to pack validation.
    #[must_use]
    pub const fn descriptor(&self) -> &SemanticInterfaceDescriptor {
        &self.descriptor
    }

    /// Returns the exact artifact-facing interface reference.
    #[must_use]
    pub fn interface(&self) -> SemanticInterfaceReference {
        self.descriptor.reference()
    }

    /// Returns the behavior identity including the exact operation-role
    /// assignment.
    #[must_use]
    pub const fn implementation_id(&self) -> SemanticImplementationId {
        self.implementation
    }

    fn role_for(&self, operation: &OperationName) -> Option<RelocationActionRole> {
        if operation == &self.start {
            Some(RelocationActionRole::Start)
        } else if operation == &self.pause {
            Some(RelocationActionRole::Pause)
        } else if operation == &self.resume {
            Some(RelocationActionRole::Resume)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RelocationActionRole {
    Start,
    Pause,
    Resume,
}

fn relocation_implementation_binding_id(
    implementation: SemanticImplementationId,
    descriptor: &SemanticInterfaceDescriptor,
    start: &OperationName,
    pause: &OperationName,
    resume: &OperationName,
) -> SemanticImplementationId {
    let mut writer = CanonicalWriter::new(RELOCATION_IMPLEMENTATION_BINDING_DOMAIN);
    writer.write_u16(RELOCATION_IMPLEMENTATION_BINDING_SCHEMA_VERSION);
    writer.write_u16(descriptor.version().get());
    if writer.write_bytes(implementation.as_bytes()).is_err()
        || writer.write_str(descriptor.key().as_str()).is_err()
        || writer.write_bytes(descriptor.digest().as_bytes()).is_err()
        || writer.write_str(start.as_str()).is_err()
        || writer.write_str(pause.as_str()).is_err()
        || writer.write_str(resume.as_str()).is_err()
    {
        unreachable!("checked relocation installation must fit canonical encoding");
    }
    SemanticImplementationId::from_bytes(ContentDigest::of_canonical(&writer.finish()).into_bytes())
}

/// Why verified execution inputs could not be activated by runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeActivationError {
    /// Action-policy execution and deferred control select incompatible modes.
    ActionExecutionControlMismatch {
        /// Execution boundary selected by the lifecycle action profile.
        execution: ActionPolicyExecutionV1,
        /// Deferred action control selected by execution configuration.
        control: DeferredActionControlV1,
    },
    /// The linked definitions require an engine protocol this runtime does not interpret.
    UnsupportedEngineProtocol {
        /// Protocol required by the exact definition set.
        required: EngineProtocolVersion,
        /// Protocol implemented by this runtime.
        supported: EngineProtocolVersion,
    },
    /// The definition closure does not select exactly the installed interface.
    SemanticClosureMismatch,
    /// A definition cannot be lowered into the installed transfer family.
    UnsupportedAction {
        /// Exact action rejected during activation.
        action: DefinitionKey,
    },
    /// More than one authored action invokes one relocation role.
    DuplicateRelocationAction {
        /// Repeated semantic operation.
        operation: OperationName,
    },
    /// No authored action invokes one required relocation role.
    MissingRelocationAction {
        /// Unbound semantic operation.
        operation: OperationName,
    },
    /// The exact definition set contains no executable transfer action.
    NoExecutableAction,
    /// The initial root violates a runtime-owned invariant.
    InitialRoot(InitialRootError),
    /// Implementation bindings do not close the definition requirements.
    SemanticBinding(SemanticBindingError),
    /// The root, semantics, and specification do not form one execution.
    InitialBinding(InitialExecutionBindingError),
}

impl fmt::Display for RuntimeActivationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActionExecutionControlMismatch { execution, control } => write!(
                formatter,
                "action execution {execution:?} is incompatible with deferred control {control:?}"
            ),
            Self::UnsupportedEngineProtocol {
                required,
                supported,
            } => write!(
                formatter,
                "definition set requires engine protocol {required}; runtime supports {supported}"
            ),
            Self::SemanticClosureMismatch => {
                formatter.write_str("definition and implementation closures do not match")
            }
            Self::UnsupportedAction { action } => {
                write!(formatter, "action {action} cannot be activated")
            }
            Self::DuplicateRelocationAction { operation } => {
                write!(
                    formatter,
                    "more than one action invokes relocation operation {operation}"
                )
            }
            Self::MissingRelocationAction { operation } => {
                write!(
                    formatter,
                    "no action invokes relocation operation {operation}"
                )
            }
            Self::NoExecutableAction => {
                formatter.write_str("definition set contains no executable transfer action")
            }
            Self::InitialRoot(error) => error.fmt(formatter),
            Self::SemanticBinding(error) => error.fmt(formatter),
            Self::InitialBinding(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RuntimeActivationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InitialRoot(error) => Some(error),
            Self::SemanticBinding(error) => Some(error),
            Self::InitialBinding(error) => Some(error),
            Self::ActionExecutionControlMismatch { .. }
            | Self::UnsupportedEngineProtocol { .. }
            | Self::SemanticClosureMismatch
            | Self::UnsupportedAction { .. }
            | Self::DuplicateRelocationAction { .. }
            | Self::MissingRelocationAction { .. }
            | Self::NoExecutableAction => None,
        }
    }
}

/// Exact authored actions activated for one containment-transfer semantic
/// family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivatedContainmentTransferActions {
    actions: Vec<DefinitionKey>,
}

impl ActivatedContainmentTransferActions {
    /// Returns normalized exact action keys validated against the activated
    /// containment implementation.
    #[must_use]
    pub fn actions(&self) -> &[DefinitionKey] {
        &self.actions
    }
}

/// Exact authored actions activated for the closed relocation family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivatedRelocationActionFamily {
    start: DefinitionKey,
    pause: DefinitionKey,
    resume: DefinitionKey,
}

impl ActivatedRelocationActionFamily {
    /// Returns the exact action invoking the installed start operation.
    #[must_use]
    pub const fn start(&self) -> &DefinitionKey {
        &self.start
    }

    /// Returns the exact action invoking the installed pause operation.
    #[must_use]
    pub const fn pause(&self) -> &DefinitionKey {
        &self.pause
    }

    /// Returns the exact action invoking the installed resume operation.
    #[must_use]
    pub const fn resume(&self) -> &DefinitionKey {
        &self.resume
    }
}

/// Sealed process-local execution semantics minted by `world-runtime`.
///
/// The value has no public constructor and exposes only stable identities.
pub struct ActivatedRuntimeExecution {
    inner: Arc<ActivatedRuntimeExecutionInner>,
}

struct ActivatedRuntimeExecutionInner {
    closure: ResolvedExecutionClosureManifestV1,
    actions: ActivatedActionTable,
    transfer: Option<ContainmentTransferImplementation>,
    containment: Option<ActivatedContainmentTransferActions>,
    relocation: Option<ActivatedRelocationActionFamily>,
}

impl ActivatedRuntimeExecution {
    pub(crate) fn origin(
        definitions: RuntimeDefinitionSet,
        transfer: Option<ContainmentTransferImplementation>,
        relocation: Option<RelocationActionImplementation>,
        lifecycle_profiles: LifecycleProfilesV2,
        input: OriginExecutionInput,
    ) -> Result<Self, RuntimeActivationError> {
        validate_action_execution_control(lifecycle_profiles, input.config)?;
        if definitions.engine_protocol() != SUPPORTED_ENGINE_PROTOCOL_VERSION {
            return Err(RuntimeActivationError::UnsupportedEngineProtocol {
                required: definitions.engine_protocol(),
                supported: SUPPORTED_ENGINE_PROTOCOL_VERSION,
            });
        }
        let mut selected_interfaces = transfer
            .iter()
            .map(ContainmentTransferImplementation::interface)
            .chain(
                relocation
                    .iter()
                    .map(RelocationActionImplementation::interface),
            )
            .collect::<Vec<_>>();
        selected_interfaces
            .sort_by(|left, right| left.key().cmp(right.key()).then_with(|| left.cmp(right)));
        if selected_interfaces.is_empty()
            || definitions.required_interfaces() != selected_interfaces
        {
            return Err(RuntimeActivationError::SemanticClosureMismatch);
        }

        let actions = ActivatedActionTable::new(&definitions, transfer.as_ref())?;
        let containment = transfer
            .as_ref()
            .map(|_| ActivatedContainmentTransferActions {
                actions: actions
                    .actions
                    .iter()
                    .map(|action| action.key.clone())
                    .collect(),
            });
        let relocation_actions = relocation
            .as_ref()
            .map(|implementation| {
                ActivatedRelocationActionFamily::new(&definitions, implementation)
            })
            .transpose()?;
        let bindings = transfer
            .iter()
            .map(|implementation| {
                SemanticImplementationBinding::new(
                    implementation.interface(),
                    implementation.implementation_id(),
                )
            })
            .chain(relocation.iter().map(|implementation| {
                SemanticImplementationBinding::new(
                    implementation.interface(),
                    implementation.implementation_id(),
                )
            }))
            .collect();
        let semantics = ExecutionSemanticsManifestV1::new(
            definitions,
            lifecycle_profiles,
            input.config,
            bindings,
        )
        .map_err(RuntimeActivationError::SemanticBinding)?;
        let root = InitialStateRootV1::origin(
            SessionMode::Running,
            input.now,
            input.admission_frontier,
            input.accepted_state,
            input.action_opportunities,
        )
        .map_err(RuntimeActivationError::InitialRoot)?;
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &semantics,
            input.root_seed,
            input.termination,
            ExternalInputBindingV1::host_serialized(),
        );
        let closure = ResolvedExecutionClosureManifestV1::bind(root, specification, semantics)
            .map_err(RuntimeActivationError::InitialBinding)?;

        Ok(Self {
            inner: Arc::new(ActivatedRuntimeExecutionInner {
                closure,
                actions,
                transfer,
                containment,
                relocation: relocation_actions,
            }),
        })
    }

    /// Returns the exact execution-specification identity.
    #[must_use]
    pub fn execution_id(&self) -> ExecutionSpecId {
        self.inner.closure.specification().id()
    }

    /// Returns the exact initial-state-root identity.
    #[must_use]
    pub fn initial_root_id(&self) -> InitialStateRootId {
        self.inner.closure.initial_root().id()
    }

    /// Returns the semantic lineage identity of the activated origin.
    #[must_use]
    pub fn epoch_lineage_id(&self) -> super::EpochLineageId {
        self.inner.closure.initial_root().lineage_id()
    }

    /// Returns the exact linked definition-set identity.
    #[must_use]
    pub fn definition_set_digest(&self) -> RuntimeDefinitionSetDigest {
        self.inner.closure.semantics().definition_set_digest()
    }

    /// Returns the normalized behavior-affecting execution-semantics identity.
    #[must_use]
    pub fn semantics_digest(&self) -> super::ExecutionSemanticsManifestDigest {
        self.inner.closure.semantics().digest()
    }

    /// Returns the pure post-commit routing policy bound into this execution.
    #[must_use]
    pub fn post_commit_routing_policy(&self) -> PostCommitRoutingPolicyV1 {
        self.inner
            .closure
            .semantics()
            .config()
            .post_commit_routing_policy()
    }

    /// Returns the exact package-lock identity retained by the definition set.
    #[must_use]
    pub fn pack_lock_digest(&self) -> PackLockDigest {
        self.inner.closure.semantics().definitions().lock().digest()
    }

    /// Returns the complete resolved execution-closure identity.
    #[must_use]
    pub fn closure_digest(&self) -> ResolvedExecutionClosureManifestDigest {
        self.inner.closure.digest()
    }

    /// Returns exact activated containment-transfer actions when this
    /// execution includes that semantic family.
    #[must_use]
    pub fn containment_transfer_actions(&self) -> Option<&ActivatedContainmentTransferActions> {
        self.inner.containment.as_ref()
    }

    /// Returns exact activated relocation actions when this execution includes
    /// the relocation semantic family.
    #[must_use]
    pub fn relocation_action_family(&self) -> Option<&ActivatedRelocationActionFamily> {
        self.inner.relocation.as_ref()
    }

    pub(crate) fn closure(&self) -> &ResolvedExecutionClosureManifestV1 {
        &self.inner.closure
    }

    pub(crate) fn evaluate(
        &self,
        snapshot: &WorldSnapshot,
        command: &CommandEnvelope,
    ) -> ActivatedCommandEvaluation {
        if command.definition_set_digest() != self.definition_set_digest() {
            return ActivatedCommandEvaluation::Rejected(
                StableCommandRejection::DefinitionUnavailable,
            );
        }
        let Some(action) = self.inner.actions.action(command.action()) else {
            return ActivatedCommandEvaluation::Rejected(
                StableCommandRejection::DefinitionUnavailable,
            );
        };
        let Some(input) = action.bind(snapshot.accepted(), command) else {
            return ActivatedCommandEvaluation::Rejected(StableCommandRejection::BindingMismatch);
        };

        let Some(transfer) = &self.inner.transfer else {
            return ActivatedCommandEvaluation::Rejected(
                StableCommandRejection::DefinitionUnavailable,
            );
        };
        classify_transfer_evaluation(transfer.evaluate(input), input)
    }
}

fn validate_action_execution_control(
    lifecycle_profiles: LifecycleProfilesV2,
    config: ExecutionConfigArtifactV3,
) -> Result<(), RuntimeActivationError> {
    let execution = lifecycle_profiles.action().execution();
    let control = config.deferred_action_control();
    match (execution, control) {
        (ActionPolicyExecutionV1::InlineDeterministic, DeferredActionControlV1::Disabled)
        | (ActionPolicyExecutionV1::DeferredCaptured, DeferredActionControlV1::Enabled { .. }) => {
            Ok(())
        }
        _ => Err(RuntimeActivationError::ActionExecutionControlMismatch { execution, control }),
    }
}

pub(crate) enum ActivatedCommandEvaluation {
    Rejected(StableCommandRejection),
    AcceptedTransfer(ContainmentTransferDelta),
    ImplementationContractViolation,
}

fn classify_transfer_evaluation(
    evaluation: ContainmentTransferEvaluation,
    input: ContainmentTransferInput<'_>,
) -> ActivatedCommandEvaluation {
    match evaluation {
        ContainmentTransferEvaluation::RequirementUnsatisfied => {
            ActivatedCommandEvaluation::Rejected(StableCommandRejection::RequirementUnsatisfied)
        }
        ContainmentTransferEvaluation::Accepted => match ContainmentTransferDelta::new(
            input.actor(),
            input.item(),
            input.source(),
            input.destination(),
        ) {
            Ok(delta) => ActivatedCommandEvaluation::AcceptedTransfer(delta),
            Err(_) => ActivatedCommandEvaluation::ImplementationContractViolation,
        },
    }
}

struct ActivatedActionTable {
    actions: Vec<ActivatedTransferAction>,
}

impl ActivatedActionTable {
    fn new(
        definitions: &RuntimeDefinitionSet,
        implementation: Option<&ContainmentTransferImplementation>,
    ) -> Result<Self, RuntimeActivationError> {
        let mut actions = Vec::new();
        if let Some(implementation) = implementation {
            for artifact in definitions.artifacts() {
                for action in artifact.actions() {
                    if !action_uses_interface(action, implementation.descriptor.key()) {
                        continue;
                    }
                    let key = DefinitionKey::new(
                        artifact.coordinate().pack_key().clone(),
                        action.name().clone(),
                    );
                    actions.push(ActivatedTransferAction::new(key, action, implementation)?);
                }
            }
            if actions.is_empty() {
                return Err(RuntimeActivationError::NoExecutableAction);
            }
        }
        actions.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(Self { actions })
    }

    fn action(&self, key: &DefinitionKey) -> Option<&ActivatedTransferAction> {
        self.actions
            .binary_search_by(|action| action.key.cmp(key))
            .ok()
            .map(|index| &self.actions[index])
    }
}

impl ActivatedRelocationActionFamily {
    fn new(
        definitions: &RuntimeDefinitionSet,
        implementation: &RelocationActionImplementation,
    ) -> Result<Self, RuntimeActivationError> {
        let mut start = None;
        let mut pause = None;
        let mut resume = None;
        for artifact in definitions.artifacts() {
            for action in artifact.actions() {
                if !action_uses_interface(action, implementation.descriptor.key()) {
                    continue;
                }
                let key = DefinitionKey::new(
                    artifact.coordinate().pack_key().clone(),
                    action.name().clone(),
                );
                let role = validate_relocation_action(&key, action, implementation)?;
                let (slot, operation) = match role {
                    RelocationActionRole::Start => (&mut start, &implementation.start),
                    RelocationActionRole::Pause => (&mut pause, &implementation.pause),
                    RelocationActionRole::Resume => (&mut resume, &implementation.resume),
                };
                if slot.replace(key).is_some() {
                    return Err(RuntimeActivationError::DuplicateRelocationAction {
                        operation: operation.clone(),
                    });
                }
            }
        }

        Ok(Self {
            start: start.ok_or_else(|| RuntimeActivationError::MissingRelocationAction {
                operation: implementation.start.clone(),
            })?,
            pause: pause.ok_or_else(|| RuntimeActivationError::MissingRelocationAction {
                operation: implementation.pause.clone(),
            })?,
            resume: resume.ok_or_else(|| RuntimeActivationError::MissingRelocationAction {
                operation: implementation.resume.clone(),
            })?,
        })
    }
}

struct ActivatedTransferAction {
    key: DefinitionKey,
    actor: BindingName,
    item: BindingName,
    source: BindingName,
    destination: BindingName,
}

fn validate_relocation_action(
    key: &DefinitionKey,
    action: &ActionDefinition,
    implementation: &RelocationActionImplementation,
) -> Result<RelocationActionRole, RuntimeActivationError> {
    let [] = action.requirements() else {
        return Err(RuntimeActivationError::UnsupportedAction {
            action: key.clone(),
        });
    };
    let [effect] = action.effects() else {
        return Err(RuntimeActivationError::UnsupportedAction {
            action: key.clone(),
        });
    };
    let effect = effect.call();
    if effect.interface() != implementation.descriptor.key() {
        return Err(RuntimeActivationError::UnsupportedAction {
            action: key.clone(),
        });
    }
    let Some(role) = implementation.role_for(effect.operation()) else {
        return Err(RuntimeActivationError::UnsupportedAction {
            action: key.clone(),
        });
    };
    let [actor, destination, source] = effect.arguments() else {
        return Err(RuntimeActivationError::UnsupportedAction {
            action: key.clone(),
        });
    };
    if !has_relocation_action_bindings(action)
        || actor.as_str() != ACTOR_ROLE
        || destination.as_str() != DESTINATION_ROLE
        || source.as_str() != SOURCE_ROLE
        || !event_matches_relocation_roles(action, actor, source, destination)
    {
        return Err(RuntimeActivationError::UnsupportedAction {
            action: key.clone(),
        });
    }
    Ok(role)
}

fn action_uses_interface(
    action: &ActionDefinition,
    interface: &world_defs::SemanticInterfaceKey,
) -> bool {
    action
        .requirements()
        .iter()
        .any(|requirement| requirement.call().interface() == interface)
        || action
            .effects()
            .iter()
            .any(|effect| effect.call().interface() == interface)
}

impl ActivatedTransferAction {
    fn new(
        key: DefinitionKey,
        action: &ActionDefinition,
        implementation: &ContainmentTransferImplementation,
    ) -> Result<Self, RuntimeActivationError> {
        let [requirement] = action.requirements() else {
            return Err(RuntimeActivationError::UnsupportedAction { action: key });
        };
        let [effect] = action.effects() else {
            return Err(RuntimeActivationError::UnsupportedAction { action: key });
        };
        let requirement = requirement.call();
        let effect = effect.call();
        if requirement.interface() != implementation.descriptor.key()
            || effect.interface() != implementation.descriptor.key()
            || requirement.operation() != &implementation.predicate
            || effect.operation() != &implementation.effect
            || requirement.arguments() != effect.arguments()
        {
            return Err(RuntimeActivationError::UnsupportedAction { action: key });
        }
        let [actor, item, source, destination] = effect.arguments() else {
            return Err(RuntimeActivationError::UnsupportedAction { action: key });
        };
        if !event_matches_transfer_roles(action, actor, item, source, destination) {
            return Err(RuntimeActivationError::UnsupportedAction { action: key });
        }

        Ok(Self {
            key,
            actor: actor.clone(),
            item: item.clone(),
            source: source.clone(),
            destination: destination.clone(),
        })
    }

    fn bind<'world>(
        &self,
        accepted: &'world AcceptedState,
        command: &CommandEnvelope,
    ) -> Option<ContainmentTransferInput<'world>> {
        let bound_actor = actor_binding(command, &self.actor)?;
        if bound_actor != command.actor() {
            return None;
        }
        let item = entity_binding(command, &self.item)?;
        let source = entity_binding(command, &self.source)?;
        let destination = entity_binding(command, &self.destination)?;
        Some(ContainmentTransferInput {
            view: accepted.domain().containment_transfer_view(
                bound_actor,
                item,
                source,
                destination,
            ),
        })
    }
}

fn is_transfer_signature(parameters: &[world_defs::OperationParameter]) -> bool {
    matches!(
        parameters,
        [actor, item, source, destination]
            if actor.name().as_str() == ACTOR_ROLE
                && actor.value_kind() == ValueKind::Actor
                && item.name().as_str() == ITEM_ROLE
                && item.value_kind() == ValueKind::Entity
                && source.name().as_str() == SOURCE_ROLE
                && source.value_kind() == ValueKind::Entity
                && destination.name().as_str() == DESTINATION_ROLE
                && destination.value_kind() == ValueKind::Entity
    )
}

fn is_relocation_signature(parameters: &[world_defs::OperationParameter]) -> bool {
    matches!(
        parameters,
        [actor, destination, source]
            if actor.name().as_str() == ACTOR_ROLE
                && actor.value_kind() == ValueKind::Actor
                && destination.name().as_str() == DESTINATION_ROLE
                && destination.value_kind() == ValueKind::Entity
                && source.name().as_str() == SOURCE_ROLE
                && source.value_kind() == ValueKind::Entity
    )
}

fn has_relocation_action_bindings(action: &ActionDefinition) -> bool {
    matches!(
        action.bindings(),
        [actor, destination, source]
            if actor.name().as_str() == ACTOR_ROLE
                && actor.value_kind() == &ValueKind::Actor
                && destination.name().as_str() == DESTINATION_ROLE
                && destination.value_kind() == &ValueKind::Entity
                && source.name().as_str() == SOURCE_ROLE
                && source.value_kind() == &ValueKind::Entity
    )
}

fn event_matches_transfer_roles(
    action: &ActionDefinition,
    actor: &BindingName,
    item: &BindingName,
    source: &BindingName,
    destination: &BindingName,
) -> bool {
    let [event] = action.success_events() else {
        return false;
    };
    let mut actor_matches = false;
    let mut item_matches = false;
    let mut source_matches = false;
    let mut destination_matches = false;
    for mapping in event.field_bindings() {
        match mapping.field().as_str() {
            ACTOR_ROLE => actor_matches = mapping.binding() == actor,
            ITEM_ROLE => item_matches = mapping.binding() == item,
            SOURCE_ROLE => source_matches = mapping.binding() == source,
            DESTINATION_ROLE => destination_matches = mapping.binding() == destination,
            _ => return false,
        }
    }
    event.field_bindings().len() == 4
        && actor_matches
        && item_matches
        && source_matches
        && destination_matches
}

fn event_matches_relocation_roles(
    action: &ActionDefinition,
    actor: &BindingName,
    source: &BindingName,
    destination: &BindingName,
) -> bool {
    let [event] = action.success_events() else {
        return false;
    };
    let mut actor_matches = false;
    let mut source_matches = false;
    let mut destination_matches = false;
    for mapping in event.field_bindings() {
        match mapping.field().as_str() {
            ACTOR_ROLE => actor_matches = mapping.binding() == actor,
            SOURCE_ROLE => source_matches = mapping.binding() == source,
            DESTINATION_ROLE => destination_matches = mapping.binding() == destination,
            _ => return false,
        }
    }
    event.field_bindings().len() == 3 && actor_matches && source_matches && destination_matches
}

fn actor_binding(command: &CommandEnvelope, name: &BindingName) -> Option<ActorId> {
    match command
        .bindings()
        .binary_search_by(|binding| binding.name().cmp(name))
        .ok()
        .map(|index| command.bindings()[index].value())?
    {
        CommandValue::Actor(actor) => Some(actor),
        CommandValue::Entity(_) => None,
    }
}

fn entity_binding(command: &CommandEnvelope, name: &BindingName) -> Option<EntityId> {
    match command
        .bindings()
        .binary_search_by(|binding| binding.name().cmp(name))
        .ok()
        .map(|index| command.bindings()[index].value())?
    {
        CommandValue::Entity(entity) => Some(entity),
        CommandValue::Actor(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use world_model::{AgencyState, DomainState, EpistemicState, SocialState};

    use super::*;

    fn valid<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("activation fixture must be valid: {error:?}"),
        }
    }

    fn deferred_profiles() -> LifecycleProfilesV2 {
        let inline = crate::execution::fixture_lifecycle_profiles();
        LifecycleProfilesV2::new(
            inline.evidence(),
            inline.appraisal(),
            inline.social(),
            inline.intent(),
            inline.activity(),
            crate::execution::ActionPolicyBindingV1::new(
                inline.action().binding(),
                ActionPolicyExecutionV1::DeferredCaptured,
            ),
        )
    }

    #[test]
    fn activation_rejects_action_execution_and_control_mismatches_before_origin() {
        let inline_profiles = crate::execution::fixture_lifecycle_profiles();
        let deferred_profiles = deferred_profiles();
        let inline_config = valid(ExecutionConfigArtifactV3::inline(64, 32, 16));
        let control = valid(DeferredActionControlV1::enabled(
            crate::execution::DeferredActionAdmissionModeV1::FrontierBlocking,
            0,
            1024,
            512,
            2048,
            256,
        ));
        let deferred_config = valid(ExecutionConfigArtifactV3::deferred(64, 32, 16, control));

        assert_eq!(
            validate_action_execution_control(inline_profiles, inline_config),
            Ok(())
        );
        assert_eq!(
            validate_action_execution_control(deferred_profiles, deferred_config),
            Ok(())
        );
        assert_eq!(
            validate_action_execution_control(inline_profiles, deferred_config),
            Err(RuntimeActivationError::ActionExecutionControlMismatch {
                execution: ActionPolicyExecutionV1::InlineDeterministic,
                control,
            })
        );
        assert_eq!(
            validate_action_execution_control(deferred_profiles, inline_config),
            Err(RuntimeActivationError::ActionExecutionControlMismatch {
                execution: ActionPolicyExecutionV1::DeferredCaptured,
                control: DeferredActionControlV1::Disabled,
            })
        );
    }

    #[test]
    fn accepted_transfer_delta_is_derived_from_every_bound_role() {
        let accepted = AcceptedState::new(
            valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );
        let actor = ActorId::from_bytes([0x11; 32]);
        let item = EntityId::from_bytes([0x21; 32]);
        let source = EntityId::from_bytes([0x31; 32]);
        let destination = EntityId::from_bytes([0x41; 32]);
        let input = ContainmentTransferInput {
            view: accepted
                .domain()
                .containment_transfer_view(actor, item, source, destination),
        };
        assert!(matches!(
            classify_transfer_evaluation(ContainmentTransferEvaluation::Accepted, input),
            ActivatedCommandEvaluation::AcceptedTransfer(delta)
                if delta.actor() == input.actor()
                    && delta.item() == input.item()
                    && delta.expected_source() == input.source()
                    && delta.destination() == input.destination()
        ));

        let invalid = ContainmentTransferInput {
            view: accepted
                .domain()
                .containment_transfer_view(actor, item, source, source),
        };
        assert!(matches!(
            classify_transfer_evaluation(ContainmentTransferEvaluation::Accepted, invalid),
            ActivatedCommandEvaluation::ImplementationContractViolation
        ));
    }
}
