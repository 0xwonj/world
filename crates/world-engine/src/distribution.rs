use core::fmt;
use std::sync::Arc;

use world_core::{CanonicalDomain, CanonicalWriter, ContentDigest};
use world_decision::{
    ActionPolicy, ActivityController, AppraisalEvaluator, BaselineActivityController,
    BaselineAppraisalEvaluator, BaselineEvidenceAssimilator, BaselineIntentPolicy,
    EvidenceAssimilator, IntentPolicy, activity_state_schema,
};
use world_defs::{
    CatalogError, EngineProtocolVersion, SemanticInterfaceCatalog, SemanticInterfaceReference,
};
use world_runtime::{
    ActionPolicyBindingV1, ActionPolicyExecutionV1, ContainmentTransferImplementation,
    LifecycleBindingV1, LifecycleImplementationId, LifecycleProfilesV2, LifecycleStateBindingV1,
    LifecycleStateSchemaId, OptionalLifecycleBindingV1, RelocationActionImplementation,
    SUPPORTED_ENGINE_PROTOCOL_VERSION, SemanticImplementationId,
};

use crate::action::{DeferredActionEvaluatorDescriptor, InstalledActionExecution};

const ACTION_POLICY_INSTALLATION_SCHEMA_VERSION: u16 = 1;
const ACTION_POLICY_INSTALLATION_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-policy-installation-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action policy installation domain must be valid"),
    };

/// One installed action evaluator with a closed execution class.
#[derive(Clone)]
pub struct ActionPolicyInstallation {
    binding: ActionPolicyBindingV1,
    execution: InstalledActionExecution,
}

impl ActionPolicyInstallation {
    /// Installs a stateless policy evaluated inside one reserved engine step.
    #[must_use]
    pub fn inline_deterministic(policy: Arc<dyn ActionPolicy>) -> Self {
        let execution = InstalledActionExecution::inline(policy);
        Self::new(execution)
    }

    /// Installs a stateless evaluator whose requests and results cross a
    /// durable capture/ingress boundary.
    #[must_use]
    pub fn deferred_captured(descriptor: DeferredActionEvaluatorDescriptor) -> Self {
        Self::new(InstalledActionExecution::deferred(descriptor))
    }

    fn new(execution: InstalledActionExecution) -> Self {
        let implementation = action_policy_implementation_id(&execution);
        Self {
            binding: ActionPolicyBindingV1::new(
                LifecycleBindingV1::stateless(implementation),
                execution.execution_class(),
            ),
            execution,
        }
    }

    /// Returns the exact profile binding satisfied by this installation.
    #[must_use]
    pub const fn binding(&self) -> ActionPolicyBindingV1 {
        self.binding
    }

    const fn execution(&self) -> &InstalledActionExecution {
        &self.execution
    }
}

fn action_policy_implementation_id(
    execution: &InstalledActionExecution,
) -> LifecycleImplementationId {
    let mut writer = CanonicalWriter::new(ACTION_POLICY_INSTALLATION_DOMAIN);
    writer.write_u16(ACTION_POLICY_INSTALLATION_SCHEMA_VERSION);
    writer.write_discriminant(match execution.execution_class() {
        ActionPolicyExecutionV1::InlineDeterministic => 0,
        ActionPolicyExecutionV1::DeferredCaptured => 1,
    });
    write_action_policy_identity(&mut writer, execution.semantics_id().as_bytes());
    write_action_policy_identity(&mut writer, execution.request_payload_schema().as_bytes());
    write_action_policy_identity(&mut writer, execution.decision_result_schema().as_bytes());
    write_action_policy_identity(
        &mut writer,
        execution.candidate_table_continuation_schema().as_bytes(),
    );
    write_action_policy_identity(&mut writer, execution.read_witness_schema().as_bytes());
    LifecycleImplementationId::from_bytes(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    )
}

fn write_action_policy_identity(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width action policy identity must fit the canonical protocol");
    }
}

fn activity_lifecycle_schema() -> LifecycleStateSchemaId {
    LifecycleStateSchemaId::from_bytes(activity_state_schema().into_bytes())
}

#[derive(Clone)]
struct InstalledEvidenceAssimilator {
    binding: LifecycleBindingV1,
    implementation: Arc<dyn EvidenceAssimilator>,
}

impl InstalledEvidenceAssimilator {
    fn new(implementation: Arc<dyn EvidenceAssimilator>) -> Self {
        Self {
            binding: LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes(
                implementation.implementation_id(),
            )),
            implementation,
        }
    }
}

#[derive(Clone)]
struct InstalledAppraisalEvaluator {
    binding: LifecycleBindingV1,
    implementation: Arc<dyn AppraisalEvaluator>,
}

impl InstalledAppraisalEvaluator {
    fn new(implementation: Arc<dyn AppraisalEvaluator>) -> Self {
        Self {
            binding: LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes(
                implementation.implementation_id(),
            )),
            implementation,
        }
    }
}

#[derive(Clone)]
struct InstalledIntentPolicy {
    binding: LifecycleBindingV1,
    implementation: Arc<dyn IntentPolicy>,
}

impl InstalledIntentPolicy {
    fn new(implementation: Arc<dyn IntentPolicy>) -> Self {
        Self {
            binding: LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes(
                implementation.implementation_id(),
            )),
            implementation,
        }
    }
}

#[derive(Clone)]
struct InstalledActivityController {
    binding: LifecycleBindingV1,
    implementation: Arc<dyn ActivityController>,
}

impl InstalledActivityController {
    fn new(implementation: Arc<dyn ActivityController>) -> Self {
        Self {
            binding: LifecycleBindingV1::persistent(
                LifecycleImplementationId::from_bytes(implementation.implementation_id()),
                activity_lifecycle_schema(),
            ),
            implementation,
        }
    }
}

/// Closed installed capability set for concrete lifecycle ports.
///
/// Identity bytes select installed capabilities; they cannot substitute for
/// the port-specific capability objects:
///
/// ```compile_fail
/// use world_engine::{
///     ActionPolicyInstallation, LifecycleBindingV1, LifecycleImplementationId,
///     LifecycleImplementationSet,
/// };
///
/// let identity = LifecycleImplementationId::from_bytes([0x11; 32]);
/// let binding = LifecycleBindingV1::stateless(identity);
/// let _ = LifecycleImplementationSet::new(
///     vec![binding],
///     Vec::new(),
///     Vec::new(),
///     Vec::new(),
///     Vec::<ActionPolicyInstallation>::new(),
/// );
/// ```
#[derive(Clone)]
pub struct LifecycleImplementationSet {
    evidence: Vec<InstalledEvidenceAssimilator>,
    appraisal: Vec<InstalledAppraisalEvaluator>,
    intent: Vec<InstalledIntentPolicy>,
    activity: Vec<InstalledActivityController>,
    action: Vec<ActionPolicyInstallation>,
}

impl LifecycleImplementationSet {
    /// Validates and normalizes exact installations for every currently
    /// concrete lifecycle port.
    pub fn new(
        evidence: Vec<Arc<dyn EvidenceAssimilator>>,
        appraisal: Vec<Arc<dyn AppraisalEvaluator>>,
        intent: Vec<Arc<dyn IntentPolicy>>,
        activity: Vec<Arc<dyn ActivityController>>,
        mut action: Vec<ActionPolicyInstallation>,
    ) -> Result<Self, LifecycleInstallationError> {
        let mut evidence = evidence
            .into_iter()
            .map(InstalledEvidenceAssimilator::new)
            .collect::<Vec<_>>();
        let mut appraisal = appraisal
            .into_iter()
            .map(InstalledAppraisalEvaluator::new)
            .collect::<Vec<_>>();
        let mut intent = intent
            .into_iter()
            .map(InstalledIntentPolicy::new)
            .collect::<Vec<_>>();
        let mut activity = activity
            .into_iter()
            .map(InstalledActivityController::new)
            .collect::<Vec<_>>();
        evidence.sort_by_key(|installation| installation.binding.implementation());
        appraisal.sort_by_key(|installation| installation.binding.implementation());
        intent.sort_by_key(|installation| installation.binding.implementation());
        activity.sort_by_key(|installation| installation.binding.implementation());
        action.sort_by_key(|installation| installation.binding().binding().implementation());

        let identities = evidence
            .iter()
            .map(|installation| installation.binding.implementation())
            .chain(
                appraisal
                    .iter()
                    .map(|installation| installation.binding.implementation()),
            )
            .chain(
                intent
                    .iter()
                    .map(|installation| installation.binding.implementation()),
            )
            .chain(
                activity
                    .iter()
                    .map(|installation| installation.binding.implementation()),
            )
            .chain(
                action
                    .iter()
                    .map(|installation| installation.binding().binding().implementation()),
            );
        let mut seen = Vec::new();
        for implementation in identities {
            if seen.contains(&implementation) {
                return Err(
                    LifecycleInstallationError::DuplicateImplementationIdentity { implementation },
                );
            }
            seen.push(implementation);
        }

        Ok(Self {
            evidence,
            appraisal,
            intent,
            activity,
            action,
        })
    }

    /// Installs the deterministic baseline identities plus supplied action policies.
    pub fn baseline(
        action: Vec<ActionPolicyInstallation>,
    ) -> Result<Self, LifecycleInstallationError> {
        let activity = BaselineActivityController::new();
        Self::new(
            vec![Arc::new(BaselineEvidenceAssimilator::new())],
            vec![Arc::new(BaselineAppraisalEvaluator::new())],
            vec![Arc::new(BaselineIntentPolicy::new())],
            vec![Arc::new(activity)],
            action,
        )
    }

    fn resolve(
        &self,
        profiles: LifecycleProfilesV2,
    ) -> Result<InstalledLifecycleImplementations, LifecycleResolutionError> {
        let evidence = self.resolve_binding(
            LifecyclePort::Evidence,
            profiles.evidence(),
            &self.evidence,
            |installation| installation.binding,
        )?;
        let appraisal = self.resolve_binding(
            LifecyclePort::Appraisal,
            profiles.appraisal(),
            &self.appraisal,
            |installation| installation.binding,
        )?;
        if let OptionalLifecycleBindingV1::Enabled(_) = profiles.social() {
            return Err(LifecycleResolutionError::MissingInstallation {
                port: LifecyclePort::Social,
            });
        }
        let intent = self.resolve_binding(
            LifecyclePort::Intent,
            profiles.intent(),
            &self.intent,
            |installation| installation.binding,
        )?;
        let activity = self.resolve_binding(
            LifecyclePort::Activity,
            profiles.activity(),
            &self.activity,
            |installation| installation.binding,
        )?;
        let action = self.resolve_action(profiles.action())?;

        Ok(InstalledLifecycleImplementations {
            profiles,
            evidence_assimilator: Arc::clone(&evidence.implementation),
            appraisal_evaluator: Arc::clone(&appraisal.implementation),
            intent_policy: Arc::clone(&intent.implementation),
            activity_controller: Arc::clone(&activity.implementation),
            action_execution: action.execution().clone(),
        })
    }

    fn resolve_binding<'a, T>(
        &self,
        port: LifecyclePort,
        selected: LifecycleBindingV1,
        installed: &'a [T],
        binding: impl Fn(&T) -> LifecycleBindingV1,
    ) -> Result<&'a T, LifecycleResolutionError> {
        if installed.is_empty() {
            return Err(LifecycleResolutionError::MissingInstallation { port });
        }
        let Some(actual) = installed.iter().find(|installation| {
            binding(installation).implementation() == selected.implementation()
        }) else {
            return match self.port_for(selected.implementation()) {
                Some(installed) => Err(LifecycleResolutionError::WrongPort {
                    selected: port,
                    installed,
                    implementation: selected.implementation(),
                }),
                None => Err(LifecycleResolutionError::UnknownImplementation {
                    port,
                    implementation: selected.implementation(),
                }),
            };
        };
        let actual_binding = binding(actual);
        if actual_binding.state() != selected.state() {
            return Err(LifecycleResolutionError::StateBindingMismatch {
                port,
                implementation: selected.implementation(),
                selected: selected.state(),
                installed: actual_binding.state(),
            });
        }
        Ok(actual)
    }

    fn resolve_action(
        &self,
        selected: ActionPolicyBindingV1,
    ) -> Result<&ActionPolicyInstallation, LifecycleResolutionError> {
        if self.action.is_empty() {
            return Err(LifecycleResolutionError::MissingInstallation {
                port: LifecyclePort::Action,
            });
        }
        let implementation = selected.binding().implementation();
        let Some(actual) = self.action.iter().find(|installation| {
            installation.binding().binding().implementation() == implementation
        }) else {
            return match self.port_for(implementation) {
                Some(installed) => Err(LifecycleResolutionError::WrongPort {
                    selected: LifecyclePort::Action,
                    installed,
                    implementation,
                }),
                None => Err(LifecycleResolutionError::UnknownImplementation {
                    port: LifecyclePort::Action,
                    implementation,
                }),
            };
        };
        if actual.binding().binding().state() != selected.binding().state() {
            return Err(LifecycleResolutionError::StateBindingMismatch {
                port: LifecyclePort::Action,
                implementation,
                selected: selected.binding().state(),
                installed: actual.binding().binding().state(),
            });
        }
        if actual.binding().execution() != selected.execution() {
            return Err(LifecycleResolutionError::ActionExecutionMismatch {
                implementation,
                selected: selected.execution(),
                installed: actual.binding().execution(),
            });
        }
        Ok(actual)
    }

    fn port_for(&self, implementation: LifecycleImplementationId) -> Option<LifecyclePort> {
        self.evidence
            .iter()
            .any(|installation| installation.binding.implementation() == implementation)
            .then_some(LifecyclePort::Evidence)
            .or_else(|| {
                self.appraisal
                    .iter()
                    .any(|installation| installation.binding.implementation() == implementation)
                    .then_some(LifecyclePort::Appraisal)
            })
            .or_else(|| {
                self.intent
                    .iter()
                    .any(|installation| installation.binding.implementation() == implementation)
                    .then_some(LifecyclePort::Intent)
            })
            .or_else(|| {
                self.activity
                    .iter()
                    .any(|installation| installation.binding.implementation() == implementation)
                    .then_some(LifecyclePort::Activity)
            })
            .or_else(|| {
                self.action
                    .iter()
                    .any(|installation| {
                        installation.binding().binding().implementation() == implementation
                    })
                    .then_some(LifecyclePort::Action)
            })
    }
}

/// Constructs the baseline mandatory profile with social interpretation disabled.
#[must_use]
pub fn baseline_lifecycle_profiles(action: ActionPolicyBindingV1) -> LifecycleProfilesV2 {
    let activity = BaselineActivityController::new();
    LifecycleProfilesV2::new(
        LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes(
            BaselineEvidenceAssimilator::new().implementation_id(),
        )),
        LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes(
            BaselineAppraisalEvaluator::new().implementation_id(),
        )),
        OptionalLifecycleBindingV1::Disabled,
        LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes(
            BaselineIntentPolicy::new().implementation_id(),
        )),
        LifecycleBindingV1::persistent(
            LifecycleImplementationId::from_bytes(activity.implementation_id()),
            activity_lifecycle_schema(),
        ),
        action,
    )
}

/// Concrete lifecycle port named by an installation diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecyclePort {
    /// Evidence assimilation.
    Evidence,
    /// Appraisal.
    Appraisal,
    /// Optional social interpretation.
    Social,
    /// Intent review.
    Intent,
    /// Activity control.
    Activity,
    /// Foreground action policy.
    Action,
}

/// Why a closed lifecycle implementation set could not be installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleInstallationError {
    /// One behavior identity was installed more than once.
    DuplicateImplementationIdentity {
        /// Reused lifecycle implementation identity.
        implementation: LifecycleImplementationId,
    },
}

impl fmt::Display for LifecycleInstallationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateImplementationIdentity { implementation } => write!(
                formatter,
                "lifecycle implementation identity {implementation} is installed more than once"
            ),
        }
    }
}

impl std::error::Error for LifecycleInstallationError {}

/// Why one selected lifecycle profile did not match installed capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleResolutionError {
    /// A required port has no installed implementation.
    MissingInstallation {
        /// Required concrete port.
        port: LifecyclePort,
    },
    /// The selected identity is not installed on any port.
    UnknownImplementation {
        /// Port making the selection.
        port: LifecyclePort,
        /// Unknown behavior identity.
        implementation: LifecycleImplementationId,
    },
    /// The selected identity is installed for a different concrete port.
    WrongPort {
        /// Port making the selection.
        selected: LifecyclePort,
        /// Port on which the identity is installed.
        installed: LifecyclePort,
        /// Misapplied behavior identity.
        implementation: LifecycleImplementationId,
    },
    /// The implementation exists but its private-state contract differs.
    StateBindingMismatch {
        /// Port making the selection.
        port: LifecyclePort,
        /// Selected behavior identity.
        implementation: LifecycleImplementationId,
        /// State contract selected by the profile.
        selected: LifecycleStateBindingV1,
        /// State contract declared by the installation.
        installed: LifecycleStateBindingV1,
    },
    /// The action implementation exists but cannot use the selected execution class.
    ActionExecutionMismatch {
        /// Selected behavior identity.
        implementation: LifecycleImplementationId,
        /// Execution class selected by the profile.
        selected: ActionPolicyExecutionV1,
        /// Execution class declared by the installation.
        installed: ActionPolicyExecutionV1,
    },
}

impl fmt::Display for LifecycleResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "lifecycle profile does not match installation: {self:?}"
        )
    }
}

impl std::error::Error for LifecycleResolutionError {}

#[derive(Clone)]
pub(crate) struct InstalledLifecycleImplementations {
    profiles: LifecycleProfilesV2,
    evidence_assimilator: Arc<dyn EvidenceAssimilator>,
    appraisal_evaluator: Arc<dyn AppraisalEvaluator>,
    intent_policy: Arc<dyn IntentPolicy>,
    activity_controller: Arc<dyn ActivityController>,
    action_execution: InstalledActionExecution,
}

impl InstalledLifecycleImplementations {
    pub(crate) const fn profiles(&self) -> LifecycleProfilesV2 {
        self.profiles
    }

    pub(crate) fn evidence_assimilator(&self) -> &dyn EvidenceAssimilator {
        self.evidence_assimilator.as_ref()
    }

    pub(crate) fn appraisal_evaluator(&self) -> &dyn AppraisalEvaluator {
        self.appraisal_evaluator.as_ref()
    }

    pub(crate) fn intent_policy(&self) -> &dyn IntentPolicy {
        self.intent_policy.as_ref()
    }

    pub(crate) fn activity_controller(&self) -> &dyn ActivityController {
        self.activity_controller.as_ref()
    }

    pub(crate) const fn action_execution(&self) -> &InstalledActionExecution {
        &self.action_execution
    }
}

/// Immutable trusted semantic installation available to one engine.
#[derive(Clone)]
pub struct EngineDistribution {
    engine_protocol: EngineProtocolVersion,
    catalog: SemanticInterfaceCatalog,
    transfer: Vec<ContainmentTransferImplementation>,
    relocation: Vec<RelocationActionImplementation>,
    lifecycle: LifecycleImplementationSet,
}

impl EngineDistribution {
    /// Validates and normalizes the closed statically linked semantic
    /// installation.
    pub fn new(
        mut transfer: Vec<ContainmentTransferImplementation>,
        mut relocation: Vec<RelocationActionImplementation>,
        lifecycle: LifecycleImplementationSet,
    ) -> Result<Self, DistributionError> {
        let catalog = SemanticInterfaceCatalog::new(
            transfer
                .iter()
                .map(|implementation| implementation.descriptor().clone())
                .chain(
                    relocation
                        .iter()
                        .map(|implementation| implementation.descriptor().clone()),
                )
                .collect(),
        )
        .map_err(DistributionError::Catalog)?;

        transfer.sort_by(|left, right| {
            left.descriptor()
                .key()
                .cmp(right.descriptor().key())
                .then_with(|| left.interface().cmp(&right.interface()))
        });
        relocation.sort_by(|left, right| {
            left.descriptor()
                .key()
                .cmp(right.descriptor().key())
                .then_with(|| left.interface().cmp(&right.interface()))
        });
        let mut implementation_ids = transfer
            .iter()
            .map(ContainmentTransferImplementation::implementation_id)
            .chain(
                relocation
                    .iter()
                    .map(RelocationActionImplementation::implementation_id),
            )
            .collect::<Vec<_>>();
        implementation_ids.sort();
        if let Some(implementation) = implementation_ids
            .windows(2)
            .find_map(|pair| (pair[0] == pair[1]).then_some(pair[0]))
        {
            return Err(DistributionError::DuplicateImplementationIdentity { implementation });
        }

        Ok(Self {
            engine_protocol: SUPPORTED_ENGINE_PROTOCOL_VERSION,
            catalog,
            transfer,
            relocation,
            lifecycle,
        })
    }

    /// Returns the engine protocol implemented by this distribution's runtime.
    #[must_use]
    pub const fn engine_protocol(&self) -> EngineProtocolVersion {
        self.engine_protocol
    }

    /// Returns the exact descriptor catalog used by authoring and loading.
    #[must_use]
    pub const fn semantic_interfaces(&self) -> &SemanticInterfaceCatalog {
        &self.catalog
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.transfer.is_empty() && self.relocation.is_empty()
    }

    pub(crate) fn transfer_implementation(
        &self,
        interface: &SemanticInterfaceReference,
    ) -> Option<&ContainmentTransferImplementation> {
        self.transfer
            .binary_search_by(|implementation| implementation.interface().cmp(interface))
            .ok()
            .map(|index| &self.transfer[index])
    }

    pub(crate) fn relocation_implementation(
        &self,
        interface: &SemanticInterfaceReference,
    ) -> Option<&RelocationActionImplementation> {
        self.relocation
            .binary_search_by(|implementation| implementation.interface().cmp(interface))
            .ok()
            .map(|index| &self.relocation[index])
    }

    pub(crate) fn resolve_lifecycle(
        &self,
        profiles: LifecycleProfilesV2,
    ) -> Result<InstalledLifecycleImplementations, LifecycleResolutionError> {
        self.lifecycle.resolve(profiles)
    }
}

/// Why a trusted engine distribution could not be constructed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DistributionError {
    /// Installed descriptors conflict in the semantic-interface catalog.
    Catalog(CatalogError),
    /// Two interfaces claimed the same behavior-affecting implementation ID.
    DuplicateImplementationIdentity {
        /// Reused implementation identity.
        implementation: SemanticImplementationId,
    },
}

impl fmt::Display for DistributionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => error.fmt(formatter),
            Self::DuplicateImplementationIdentity { implementation } => write!(
                formatter,
                "semantic implementation identity {implementation} is installed more than once"
            ),
        }
    }
}

impl std::error::Error for DistributionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::DuplicateImplementationIdentity { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;
    use std::sync::Arc;

    use world_context::{
        ActionContextPayload, ActionPolicySemanticsId, ActivityAdvancementPayload,
        ContainmentActivityInitializationPayload, ContainmentAppraisalPayload,
        ContainmentIntentPayload, EvidenceAssimilationPayload, action_context_payload_schema,
        action_read_witness_schema, candidate_resolution_table_schema,
    };
    use world_decision::{
        ActionDecision, ActionPolicyError, ActivityAdvancementDecision, ActivityControllerError,
        ActivityInitializationDecision, AppraisalEvaluationError, BaselineActionPolicy,
        ContainmentAppraisalEvaluation, EvidenceAssimilationError, EvidenceAssimilationProposal,
        IntentDecision, IntentPolicyError, action_decision_schema,
    };
    use world_defs::{
        InterfaceVersion, OperationKind, OperationName, OperationParameter, ParameterName,
        SemanticInterfaceDescriptor, SemanticInterfaceKey, SemanticOperationDescriptor, ValueKind,
    };
    use world_runtime::{ContainmentTransferEvaluation, ContainmentTransferInput};

    use super::*;

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("distribution fixture must be valid: {error}"),
        }
    }

    fn descriptor(key: &str, version: u16) -> SemanticInterfaceDescriptor {
        let parameters = vec![
            OperationParameter::new(valid(ParameterName::parse("actor")), ValueKind::Actor),
            OperationParameter::new(valid(ParameterName::parse("item")), ValueKind::Entity),
            OperationParameter::new(valid(ParameterName::parse("source")), ValueKind::Entity),
            OperationParameter::new(
                valid(ParameterName::parse("destination")),
                ValueKind::Entity,
            ),
        ];
        valid(SemanticInterfaceDescriptor::new(
            valid(SemanticInterfaceKey::parse(key)),
            valid(InterfaceVersion::new(version)),
            vec![
                valid(SemanticOperationDescriptor::new(
                    valid(OperationName::parse("allowed")),
                    OperationKind::Predicate,
                    parameters.clone(),
                )),
                valid(SemanticOperationDescriptor::new(
                    valid(OperationName::parse("apply")),
                    OperationKind::Effect,
                    parameters,
                )),
            ],
        ))
    }

    fn never_accepts(_: ContainmentTransferInput<'_>) -> ContainmentTransferEvaluation {
        ContainmentTransferEvaluation::RequirementUnsatisfied
    }

    fn implementation(key: &str, version: u16, identity: u8) -> ContainmentTransferImplementation {
        valid(ContainmentTransferImplementation::new(
            descriptor(key, version),
            SemanticImplementationId::from_bytes([identity; 32]),
            never_accepts,
        ))
    }

    fn relocation_descriptor(key: &str, version: u16) -> SemanticInterfaceDescriptor {
        let parameters = vec![
            OperationParameter::new(valid(ParameterName::parse("actor")), ValueKind::Actor),
            OperationParameter::new(
                valid(ParameterName::parse("destination")),
                ValueKind::Entity,
            ),
            OperationParameter::new(valid(ParameterName::parse("source")), ValueKind::Entity),
        ];
        valid(SemanticInterfaceDescriptor::new(
            valid(SemanticInterfaceKey::parse(key)),
            valid(InterfaceVersion::new(version)),
            ["start", "pause", "resume"]
                .into_iter()
                .map(|name| {
                    valid(SemanticOperationDescriptor::new(
                        valid(OperationName::parse(name)),
                        OperationKind::Effect,
                        parameters.clone(),
                    ))
                })
                .collect(),
        ))
    }

    fn relocation_implementation(
        key: &str,
        version: u16,
        identity: u8,
    ) -> RelocationActionImplementation {
        valid(RelocationActionImplementation::new(
            relocation_descriptor(key, version),
            SemanticImplementationId::from_bytes([identity; 32]),
            valid(OperationName::parse("start")),
            valid(OperationName::parse("pause")),
            valid(OperationName::parse("resume")),
        ))
    }

    fn lifecycle() -> LifecycleImplementationSet {
        valid(LifecycleImplementationSet::baseline(vec![
            ActionPolicyInstallation::inline_deterministic(Arc::new(BaselineActionPolicy::new())),
        ]))
    }

    struct FixedEvidenceAssimilator {
        identity: [u8; 32],
    }

    impl EvidenceAssimilator for FixedEvidenceAssimilator {
        fn implementation_id(&self) -> [u8; 32] {
            self.identity
        }

        fn assimilate(
            &self,
            _input: &EvidenceAssimilationPayload,
        ) -> Result<EvidenceAssimilationProposal, EvidenceAssimilationError> {
            unreachable!("distribution identity fixture is never evaluated")
        }
    }

    struct FixedAppraisalEvaluator {
        identity: [u8; 32],
    }

    impl AppraisalEvaluator for FixedAppraisalEvaluator {
        fn implementation_id(&self) -> [u8; 32] {
            self.identity
        }

        fn evaluate(
            &self,
            _input: &ContainmentAppraisalPayload,
        ) -> Result<ContainmentAppraisalEvaluation, AppraisalEvaluationError> {
            unreachable!("distribution identity fixture is never evaluated")
        }
    }

    struct FixedIntentPolicy {
        identity: [u8; 32],
    }

    impl IntentPolicy for FixedIntentPolicy {
        fn implementation_id(&self) -> [u8; 32] {
            self.identity
        }

        fn decide(
            &self,
            _input: &ContainmentIntentPayload,
        ) -> Result<IntentDecision, IntentPolicyError> {
            unreachable!("distribution identity fixture is never evaluated")
        }
    }

    struct FixedActivityController {
        identity: [u8; 32],
    }

    impl ActivityController for FixedActivityController {
        fn implementation_id(&self) -> [u8; 32] {
            self.identity
        }

        fn initialize(
            &self,
            _input: &ContainmentActivityInitializationPayload,
        ) -> Result<ActivityInitializationDecision, ActivityControllerError> {
            unreachable!("distribution identity fixture is never evaluated")
        }

        fn advance(
            &self,
            _input: &ActivityAdvancementPayload,
        ) -> Result<ActivityAdvancementDecision, ActivityControllerError> {
            unreachable!("distribution identity fixture is never evaluated")
        }
    }

    struct FixedActionPolicy {
        semantics: ActionPolicySemanticsId,
    }

    impl ActionPolicy for FixedActionPolicy {
        fn semantics_id(&self) -> ActionPolicySemanticsId {
            self.semantics
        }

        fn decide(
            &self,
            input: &ActionContextPayload,
        ) -> Result<ActionDecision, ActionPolicyError> {
            Ok(ActionDecision::NoApplicableAction {
                input: input.input_fingerprint(),
            })
        }
    }

    fn baseline_set_with_evidence(
        evidence: Vec<Arc<dyn EvidenceAssimilator>>,
        action: Vec<ActionPolicyInstallation>,
    ) -> LifecycleImplementationSet {
        valid(LifecycleImplementationSet::new(
            evidence,
            vec![Arc::new(BaselineAppraisalEvaluator::new())],
            vec![Arc::new(BaselineIntentPolicy::new())],
            vec![Arc::new(BaselineActivityController::new())],
            action,
        ))
    }

    fn baseline_action() -> ActionPolicyInstallation {
        ActionPolicyInstallation::inline_deterministic(Arc::new(BaselineActionPolicy::new()))
    }

    #[test]
    fn action_installations_bind_execution_class_and_fixed_artifact_schemas() {
        let semantics = ActionPolicySemanticsId::from_bytes([0xa6; 32]);
        let inline = ActionPolicyInstallation::inline_deterministic(Arc::new(FixedActionPolicy {
            semantics,
        }));
        let descriptor = DeferredActionEvaluatorDescriptor::new(semantics);
        assert_eq!(
            descriptor.request_payload_schema(),
            action_context_payload_schema()
        );
        assert_eq!(
            descriptor.decision_result_schema(),
            action_decision_schema()
        );
        assert_eq!(
            descriptor.candidate_table_continuation_schema(),
            candidate_resolution_table_schema()
        );
        assert_eq!(
            descriptor.read_witness_schema(),
            action_read_witness_schema()
        );

        let deferred = ActionPolicyInstallation::deferred_captured(descriptor);
        let inline_binding = inline.binding();
        let deferred_binding = deferred.binding();
        assert_eq!(
            inline_binding.execution(),
            ActionPolicyExecutionV1::InlineDeterministic
        );
        assert_eq!(
            deferred_binding.execution(),
            ActionPolicyExecutionV1::DeferredCaptured
        );
        assert_ne!(
            inline_binding.binding().implementation(),
            deferred_binding.binding().implementation(),
            "execution class must be part of lifecycle implementation identity"
        );
        assert_eq!(
            inline_binding.binding().implementation().to_string(),
            "b848bc0cb40d90428aea52c7410de9be236348f20f5632ad25230bbd43ab6e55"
        );
        assert_eq!(
            deferred_binding.binding().implementation().to_string(),
            "219dc37a97fa6a9c42b44344abba4179bdd357da25f855096891d09eaa8263ef"
        );

        let profiles = baseline_lifecycle_profiles(deferred_binding);
        let selected = valid(
            valid(LifecycleImplementationSet::baseline(vec![inline, deferred])).resolve(profiles),
        );
        assert_eq!(
            selected.action_execution().execution_class(),
            ActionPolicyExecutionV1::DeferredCaptured
        );
        assert!(selected.action_execution().inline_policy().is_none());
        assert_eq!(
            selected.action_execution().deferred_descriptor(),
            Some(descriptor)
        );
    }

    #[test]
    fn distribution_rejects_duplicate_and_conflicting_descriptors() {
        let duplicate = EngineDistribution::new(
            vec![
                implementation("test.transfer", 1, 1),
                implementation("test.transfer", 1, 2),
            ],
            Vec::new(),
            lifecycle(),
        );
        assert!(matches!(
            duplicate,
            Err(DistributionError::Catalog(
                CatalogError::DuplicateEntry { .. }
            ))
        ));

        let conflict = EngineDistribution::new(
            vec![
                implementation("test.transfer", 1, 1),
                implementation("test.transfer", 2, 2),
            ],
            Vec::new(),
            lifecycle(),
        );
        assert!(matches!(
            conflict,
            Err(DistributionError::Catalog(
                CatalogError::ConflictingEntry { .. }
            ))
        ));
    }

    #[test]
    fn implementation_identity_is_unique_across_the_whole_distribution() {
        let result = EngineDistribution::new(
            vec![
                implementation("test.alpha", 1, 7),
                implementation("test.middle", 1, 8),
                implementation("test.zeta", 1, 7),
            ],
            Vec::new(),
            lifecycle(),
        );

        assert!(matches!(
            result,
            Err(DistributionError::DuplicateImplementationIdentity {
                implementation
            }) if implementation == SemanticImplementationId::from_bytes([7; 32])
        ));
    }

    #[test]
    fn semantic_identity_is_unique_across_different_families() {
        let relocation = relocation_implementation("test.relocation", 1, 9);
        let duplicate_identity = relocation.implementation_id();
        let transfer = valid(ContainmentTransferImplementation::new(
            descriptor("test.transfer", 1),
            duplicate_identity,
            never_accepts,
        ));

        let result = EngineDistribution::new(vec![transfer], vec![relocation], lifecycle());

        assert!(matches!(
            result,
            Err(DistributionError::DuplicateImplementationIdentity {
                implementation
            }) if implementation == duplicate_identity
        ));
    }

    #[test]
    fn lookup_is_exact_and_unaffected_by_order_or_unused_installations() {
        let alpha = implementation("test.alpha", 1, 1);
        let zeta = implementation("test.zeta", 1, 2);
        let alpha_reference = alpha.interface();
        let zeta_reference = zeta.interface();
        let minimal = valid(EngineDistribution::new(
            vec![alpha.clone()],
            Vec::new(),
            lifecycle(),
        ));
        let extended = valid(EngineDistribution::new(
            vec![zeta, alpha],
            Vec::new(),
            lifecycle(),
        ));

        assert_eq!(
            minimal
                .transfer_implementation(&alpha_reference)
                .map(ContainmentTransferImplementation::implementation_id),
            extended
                .transfer_implementation(&alpha_reference)
                .map(ContainmentTransferImplementation::implementation_id),
        );
        assert_eq!(
            extended
                .transfer_implementation(&alpha_reference)
                .map(ContainmentTransferImplementation::implementation_id),
            Some(SemanticImplementationId::from_bytes([1; 32])),
        );
        assert_eq!(
            extended
                .transfer_implementation(&zeta_reference)
                .map(ContainmentTransferImplementation::implementation_id),
            Some(SemanticImplementationId::from_bytes([2; 32]))
        );
    }

    #[test]
    fn relocation_lookup_is_exact() {
        let alpha = relocation_implementation("test.relocation.alpha", 1, 1);
        let zeta = relocation_implementation("test.relocation.zeta", 1, 2);
        let alpha_reference = alpha.interface();
        let zeta_reference = zeta.interface();
        let alpha_identity = alpha.implementation_id();
        let zeta_identity = zeta.implementation_id();
        let distribution = valid(EngineDistribution::new(
            Vec::new(),
            vec![zeta, alpha],
            lifecycle(),
        ));

        assert_eq!(
            distribution
                .relocation_implementation(&alpha_reference)
                .map(RelocationActionImplementation::implementation_id),
            Some(alpha_identity)
        );
        assert_eq!(
            distribution
                .relocation_implementation(&zeta_reference)
                .map(RelocationActionImplementation::implementation_id),
            Some(zeta_identity)
        );
    }

    #[test]
    fn exact_available_port_objects_are_retained() {
        let evidence: Arc<dyn EvidenceAssimilator> = Arc::new(FixedEvidenceAssimilator {
            identity: [0x11; 32],
        });
        let appraisal: Arc<dyn AppraisalEvaluator> = Arc::new(FixedAppraisalEvaluator {
            identity: [0x12; 32],
        });
        let intent: Arc<dyn IntentPolicy> = Arc::new(FixedIntentPolicy {
            identity: [0x14; 32],
        });
        let activity: Arc<dyn ActivityController> = Arc::new(FixedActivityController {
            identity: [0x15; 32],
        });
        let action = ActionPolicyInstallation::inline_deterministic(Arc::new(FixedActionPolicy {
            semantics: ActionPolicySemanticsId::from_bytes([0x16; 32]),
        }));
        let profiles = LifecycleProfilesV2::new(
            LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0x11; 32])),
            LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0x12; 32])),
            OptionalLifecycleBindingV1::Disabled,
            LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0x14; 32])),
            LifecycleBindingV1::persistent(
                LifecycleImplementationId::from_bytes([0x15; 32]),
                activity_lifecycle_schema(),
            ),
            action.binding(),
        );
        let set = valid(LifecycleImplementationSet::new(
            vec![Arc::clone(&evidence)],
            vec![Arc::clone(&appraisal)],
            vec![Arc::clone(&intent)],
            vec![Arc::clone(&activity)],
            vec![action],
        ));

        let selected = valid(set.resolve(profiles));

        assert_eq!(selected.profiles(), profiles);
        assert!(Arc::ptr_eq(&selected.evidence_assimilator, &evidence));
        assert!(Arc::ptr_eq(&selected.appraisal_evaluator, &appraisal));
        assert!(Arc::ptr_eq(&selected.intent_policy, &intent));
        assert!(Arc::ptr_eq(&selected.activity_controller, &activity));
        assert_eq!(
            selected.action_execution().semantics_id(),
            ActionPolicySemanticsId::from_bytes([0x16; 32])
        );
        assert!(selected.action_execution().inline_policy().is_some());
        assert!(selected.action_execution().deferred_descriptor().is_none());
    }

    #[test]
    fn disabled_social_requires_no_placeholder_installation() {
        let action = baseline_action();
        let profiles = baseline_lifecycle_profiles(action.binding());
        let selected =
            valid(valid(LifecycleImplementationSet::baseline(vec![action])).resolve(profiles));

        assert_eq!(
            selected.profiles().social(),
            OptionalLifecycleBindingV1::Disabled
        );
    }

    #[test]
    fn enabled_social_fails_until_a_concrete_evaluator_exists() {
        let action = baseline_action();
        let baseline = baseline_lifecycle_profiles(action.binding());
        let profiles = LifecycleProfilesV2::new(
            baseline.evidence(),
            baseline.appraisal(),
            OptionalLifecycleBindingV1::Enabled(LifecycleBindingV1::stateless(
                LifecycleImplementationId::from_bytes([0x13; 32]),
            )),
            baseline.intent(),
            baseline.activity(),
            baseline.action(),
        );
        let result = valid(LifecycleImplementationSet::baseline(vec![action])).resolve(profiles);

        assert!(matches!(
            result,
            Err(LifecycleResolutionError::MissingInstallation {
                port: LifecyclePort::Social
            })
        ));
    }

    #[test]
    fn unused_installations_do_not_change_the_selected_capability() {
        let selected_evidence: Arc<dyn EvidenceAssimilator> = Arc::new(FixedEvidenceAssimilator {
            identity: [0x31; 32],
        });
        let unused_evidence: Arc<dyn EvidenceAssimilator> = Arc::new(FixedEvidenceAssimilator {
            identity: [0x32; 32],
        });
        let minimal_action = baseline_action();
        let extended_action = baseline_action();
        let profiles = LifecycleProfilesV2::new(
            LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0x31; 32])),
            baseline_lifecycle_profiles(minimal_action.binding()).appraisal(),
            OptionalLifecycleBindingV1::Disabled,
            baseline_lifecycle_profiles(minimal_action.binding()).intent(),
            baseline_lifecycle_profiles(minimal_action.binding()).activity(),
            minimal_action.binding(),
        );
        let minimal =
            baseline_set_with_evidence(vec![Arc::clone(&selected_evidence)], vec![minimal_action]);
        let extended = baseline_set_with_evidence(
            vec![Arc::clone(&unused_evidence), Arc::clone(&selected_evidence)],
            vec![extended_action],
        );

        let minimal_selected = valid(minimal.resolve(profiles));
        let extended_selected = valid(extended.resolve(profiles));

        assert_eq!(minimal_selected.profiles(), extended_selected.profiles());
        assert!(Arc::ptr_eq(
            &minimal_selected.evidence_assimilator,
            &selected_evidence
        ));
        assert!(Arc::ptr_eq(
            &extended_selected.evidence_assimilator,
            &selected_evidence
        ));
    }

    #[test]
    fn lifecycle_installation_rejects_duplicate_behavior_identity() {
        let duplicate = LifecycleImplementationId::from_bytes([0x41; 32]);
        let result = LifecycleImplementationSet::new(
            vec![
                Arc::new(FixedEvidenceAssimilator {
                    identity: [0x41; 32],
                }),
                Arc::new(FixedEvidenceAssimilator {
                    identity: [0x41; 32],
                }),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );

        assert!(matches!(
            result,
            Err(LifecycleInstallationError::DuplicateImplementationIdentity {
                implementation
            }) if implementation == duplicate
        ));
    }

    #[test]
    fn lifecycle_resolution_reports_missing_unknown_and_wrong_port_selections() {
        let missing_action = baseline_action();
        let missing_profiles = baseline_lifecycle_profiles(missing_action.binding());
        let missing =
            baseline_set_with_evidence(Vec::new(), vec![missing_action]).resolve(missing_profiles);
        assert!(matches!(
            missing,
            Err(LifecycleResolutionError::MissingInstallation {
                port: LifecyclePort::Evidence
            })
        ));

        let unknown_action = baseline_action();
        let baseline = baseline_lifecycle_profiles(unknown_action.binding());
        let unknown_profiles = LifecycleProfilesV2::new(
            LifecycleBindingV1::stateless(LifecycleImplementationId::from_bytes([0x51; 32])),
            baseline.appraisal(),
            baseline.social(),
            baseline.intent(),
            baseline.activity(),
            baseline.action(),
        );
        let unknown = valid(LifecycleImplementationSet::baseline(vec![unknown_action]))
            .resolve(unknown_profiles);
        assert!(matches!(
            unknown,
            Err(LifecycleResolutionError::UnknownImplementation {
                port: LifecyclePort::Evidence,
                implementation
            }) if implementation == LifecycleImplementationId::from_bytes([0x51; 32])
        ));

        let wrong_action = baseline_action();
        let baseline = baseline_lifecycle_profiles(wrong_action.binding());
        let wrong_profiles = LifecycleProfilesV2::new(
            baseline.appraisal(),
            baseline.appraisal(),
            baseline.social(),
            baseline.intent(),
            baseline.activity(),
            baseline.action(),
        );
        let wrong =
            valid(LifecycleImplementationSet::baseline(vec![wrong_action])).resolve(wrong_profiles);
        assert!(matches!(
            wrong,
            Err(LifecycleResolutionError::WrongPort {
                selected: LifecyclePort::Evidence,
                installed: LifecyclePort::Appraisal,
                ..
            })
        ));
    }

    #[test]
    fn lifecycle_resolution_checks_state_schema_and_action_execution_class() {
        let state_action = baseline_action();
        let baseline = baseline_lifecycle_profiles(state_action.binding());
        let state_profiles = LifecycleProfilesV2::new(
            baseline.evidence(),
            baseline.appraisal(),
            baseline.social(),
            baseline.intent(),
            LifecycleBindingV1::persistent(
                baseline.activity().implementation(),
                LifecycleStateSchemaId::from_bytes([0x61; 32]),
            ),
            baseline.action(),
        );
        let state =
            valid(LifecycleImplementationSet::baseline(vec![state_action])).resolve(state_profiles);
        assert!(matches!(
            state,
            Err(LifecycleResolutionError::StateBindingMismatch {
                port: LifecyclePort::Activity,
                selected: LifecycleStateBindingV1::Persistent(schema),
                ..
            }) if schema == LifecycleStateSchemaId::from_bytes([0x61; 32])
        ));

        let execution_action = baseline_action();
        let baseline = baseline_lifecycle_profiles(execution_action.binding());
        let execution_profiles = LifecycleProfilesV2::new(
            baseline.evidence(),
            baseline.appraisal(),
            baseline.social(),
            baseline.intent(),
            baseline.activity(),
            ActionPolicyBindingV1::new(
                baseline.action().binding(),
                ActionPolicyExecutionV1::DeferredCaptured,
            ),
        );
        let execution = valid(LifecycleImplementationSet::baseline(vec![execution_action]))
            .resolve(execution_profiles);
        assert!(matches!(
            execution,
            Err(LifecycleResolutionError::ActionExecutionMismatch {
                selected: ActionPolicyExecutionV1::DeferredCaptured,
                installed: ActionPolicyExecutionV1::InlineDeterministic,
                ..
            })
        ));
    }
}
