use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use world_core::{
    ActorId, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest, EntityId,
};
use world_defs::{
    ActionBindingData, ActionDefinition, BindingName, DefinitionKey, RuntimeDefinitionSet,
    RuntimeDefinitionSetDigest, ValueKind,
};
use world_model::{
    ActionOpportunityId, ContainmentInteractionScope, RelocationInteraction,
    RelocationInteractionScope, WorldSnapshot,
};

use crate::identity::{
    ActionInputFingerprint, ActionPolicySemanticsId, ActorSafeObjectRef, GroundedActionCandidateId,
    GroundedCandidateSetFingerprint, GroundingSemanticsId,
};

mod codec;

pub use codec::{
    ActionArtifactCodecError, action_context_payload_schema, action_execution_witness_schema,
    action_projection_witness_schema, action_read_witness_schema,
    candidate_resolution_table_schema, decode_action_context_payload,
    decode_action_execution_witness, decode_action_projection_witness, decode_action_read_witness,
    decode_candidate_resolution_table, encode_action_context_payload,
    encode_action_execution_witness, encode_action_projection_witness, encode_action_read_witness,
    encode_candidate_resolution_table,
};

const IDENTITY_SCHEMA_VERSION: u16 = 2;

const OBJECT_REFERENCE_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("actor-safe-object-ref-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("actor-safe object reference domain must be valid"),
    };
const CANDIDATE_DOMAIN: CanonicalDomain = match CanonicalDomain::new("grounded-action-candidate-v2")
{
    Ok(domain) => domain,
    Err(_) => panic!("grounded action candidate domain must be valid"),
};
const CANDIDATE_SET_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("grounded-candidate-set-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("grounded candidate set domain must be valid"),
    };
const ACTION_INPUT_DOMAIN: CanonicalDomain = match CanonicalDomain::new("grounded-action-input-v2")
{
    Ok(domain) => domain,
    Err(_) => panic!("grounded action input domain must be valid"),
};
const GROUNDING_SEMANTICS_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("containment-transfer-grounding-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("containment transfer grounding domain must be valid"),
    };
const RELOCATION_GROUNDING_SEMANTICS_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("relocation-grounding-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("relocation grounding domain must be valid"),
    };
const ACTOR_ROLE: &str = "actor";
const DESTINATION_ROLE: &str = "destination";
const ITEM_ROLE: &str = "item";
const SOURCE_ROLE: &str = "source";

/// Actor-visible relocation operation selected by one grounded candidate.
///
/// The exact route and any live process binding remain outside this value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RelocationActionVerb {
    /// Begin progress along an accepted directed route.
    Start,
    /// Suspend a currently active relocation.
    Pause,
    /// Continue a currently paused relocation.
    Resume,
}

impl RelocationActionVerb {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Start => 0,
            Self::Pause => 1,
            Self::Resume => 2,
        }
    }
}

impl From<RelocationInteraction> for RelocationActionVerb {
    fn from(interaction: RelocationInteraction) -> Self {
        match interaction {
            RelocationInteraction::Start(_) => Self::Start,
            RelocationInteraction::Pause(_) => Self::Pause,
            RelocationInteraction::Resume(_) => Self::Resume,
        }
    }
}

/// Why the exact authored relocation action family could not be bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelocationActionDefinitionsError {
    /// Two relocation verbs named the same authored action.
    DuplicateAction {
        /// Repeated durable action key.
        action: DefinitionKey,
    },
    /// One exact authored action was absent from the resolved definition set.
    ActionUnavailable {
        /// Missing durable action key.
        action: DefinitionKey,
    },
    /// One action did not declare exactly `actor`, `destination`, and `source`.
    BindingShapeMismatch {
        /// Incompatible durable action key.
        action: DefinitionKey,
    },
}

impl fmt::Display for RelocationActionDefinitionsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid relocation action family: {self:?}")
    }
}

impl std::error::Error for RelocationActionDefinitionsError {}

/// Why exact authored containment-transfer actions could not be bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainmentTransferActionDefinitionsError {
    /// The activated containment-transfer family contains no authored action.
    NoActions,
    /// One durable action key appeared more than once.
    DuplicateAction {
        /// Repeated durable action key.
        action: DefinitionKey,
    },
    /// One exact authored action was absent from the resolved definition set.
    ActionUnavailable {
        /// Missing durable action key.
        action: DefinitionKey,
    },
    /// One action did not declare exactly the transfer actor, destination,
    /// item, and source bindings.
    BindingShapeMismatch {
        /// Incompatible durable action key.
        action: DefinitionKey,
    },
}

impl fmt::Display for ContainmentTransferActionDefinitionsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid containment-transfer action family: {self:?}"
        )
    }
}

impl std::error::Error for ContainmentTransferActionDefinitionsError {}

/// Exact checked authored actions for one activated containment-transfer
/// semantic family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainmentTransferActionDefinitions {
    definition_set: RuntimeDefinitionSetDigest,
    actions: Vec<DefinitionKey>,
}

impl ContainmentTransferActionDefinitions {
    /// Binds the exact activated actions after validating their typed grounding
    /// roles against one linked definition set.
    pub fn new(
        definitions: &RuntimeDefinitionSet,
        mut actions: Vec<DefinitionKey>,
    ) -> Result<Self, ContainmentTransferActionDefinitionsError> {
        if actions.is_empty() {
            return Err(ContainmentTransferActionDefinitionsError::NoActions);
        }
        actions.sort();
        if let Some(action) = actions
            .windows(2)
            .find_map(|pair| (pair[0] == pair[1]).then(|| pair[0].clone()))
        {
            return Err(ContainmentTransferActionDefinitionsError::DuplicateAction { action });
        }
        for action in &actions {
            let Some(definition) = definitions.action(action) else {
                return Err(
                    ContainmentTransferActionDefinitionsError::ActionUnavailable {
                        action: action.clone(),
                    },
                );
            };
            if !has_containment_transfer_binding_shape(definition) {
                return Err(
                    ContainmentTransferActionDefinitionsError::BindingShapeMismatch {
                        action: action.clone(),
                    },
                );
            }
        }
        Ok(Self {
            definition_set: definitions.digest(),
            actions,
        })
    }

    /// Returns the normalized exact action keys in this family.
    #[must_use]
    pub fn actions(&self) -> &[DefinitionKey] {
        &self.actions
    }

    /// Returns the exact linked definition set against which this family was
    /// checked.
    #[must_use]
    pub const fn definition_set(&self) -> RuntimeDefinitionSetDigest {
        self.definition_set
    }
}

/// Exact checked authored actions for the closed relocation family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelocationActionDefinitions {
    definition_set: RuntimeDefinitionSetDigest,
    start: DefinitionKey,
    pause: DefinitionKey,
    resume: DefinitionKey,
}

impl RelocationActionDefinitions {
    /// Binds three distinct exact actions after validating their typed roles.
    pub fn new(
        definitions: &RuntimeDefinitionSet,
        start: DefinitionKey,
        pause: DefinitionKey,
        resume: DefinitionKey,
    ) -> Result<Self, RelocationActionDefinitionsError> {
        let mut keys = [&start, &pause, &resume];
        keys.sort();
        if let Some(action) = keys
            .windows(2)
            .find_map(|pair| (pair[0] == pair[1]).then(|| pair[0].clone()))
        {
            return Err(RelocationActionDefinitionsError::DuplicateAction { action });
        }
        for action in [&start, &pause, &resume] {
            let Some(definition) = definitions.action(action) else {
                return Err(RelocationActionDefinitionsError::ActionUnavailable {
                    action: action.clone(),
                });
            };
            if !has_relocation_binding_shape(definition) {
                return Err(RelocationActionDefinitionsError::BindingShapeMismatch {
                    action: action.clone(),
                });
            }
        }
        Ok(Self {
            definition_set: definitions.digest(),
            start,
            pause,
            resume,
        })
    }

    /// Returns the exact authored start action.
    #[must_use]
    pub const fn start(&self) -> &DefinitionKey {
        &self.start
    }

    /// Returns the exact authored pause action.
    #[must_use]
    pub const fn pause(&self) -> &DefinitionKey {
        &self.pause
    }

    /// Returns the exact authored resume action.
    #[must_use]
    pub const fn resume(&self) -> &DefinitionKey {
        &self.resume
    }

    /// Returns the exact linked definition set against which this family was
    /// checked.
    #[must_use]
    pub const fn definition_set(&self) -> RuntimeDefinitionSetDigest {
        self.definition_set
    }

    fn action(&self, verb: RelocationActionVerb) -> &DefinitionKey {
        match verb {
            RelocationActionVerb::Start => &self.start,
            RelocationActionVerb::Pause => &self.pause,
            RelocationActionVerb::Resume => &self.resume,
        }
    }
}

/// Why the concrete containment-transfer projection could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainmentProjectionError {
    /// The checked action family belongs to another linked definition set.
    DefinitionSetMismatch {
        /// Definition set used to check the action family.
        expected: RuntimeDefinitionSetDigest,
        /// Definition set supplied to projection.
        actual: RuntimeDefinitionSetDigest,
    },
    /// Two distinct exact entities produced the same actor-safe reference.
    ObjectReferenceCollision {
        /// Colliding actor-safe reference.
        reference: ActorSafeObjectRef,
    },
    /// Two distinct grounded bindings produced the same candidate identity.
    CandidateIdentityCollision {
        /// Colliding candidate identity.
        candidate: GroundedActionCandidateId,
    },
    /// A canonical identity preimage could not represent one of its fields.
    Canonical(CanonicalError),
}

impl fmt::Display for ContainmentProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefinitionSetMismatch { expected, actual } => write!(
                formatter,
                "containment-transfer actions belong to definition set {expected}, not {actual}"
            ),
            Self::ObjectReferenceCollision { reference } => {
                write!(
                    formatter,
                    "actor-safe object reference collision at {reference}"
                )
            }
            Self::CandidateIdentityCollision { candidate } => {
                write!(
                    formatter,
                    "grounded candidate identity collision at {candidate}"
                )
            }
            Self::Canonical(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ContainmentProjectionError {}

impl From<CanonicalError> for ContainmentProjectionError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

/// Why actor-safe relocation projection could not be completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocationProjectionError {
    /// The checked action family belongs to another linked definition set.
    DefinitionSetMismatch {
        /// Definition set used to check the action family.
        expected: RuntimeDefinitionSetDigest,
        /// Definition set supplied to projection.
        actual: RuntimeDefinitionSetDigest,
    },
    /// Two exact entities produced the same actor-safe reference.
    ObjectReferenceCollision {
        /// Colliding actor-safe reference.
        reference: ActorSafeObjectRef,
    },
    /// Two distinct grounded interactions produced the same candidate identity.
    CandidateIdentityCollision {
        /// Colliding candidate identity.
        candidate: GroundedActionCandidateId,
    },
    /// A canonical identity preimage could not represent one of its fields.
    Canonical(CanonicalError),
}

impl fmt::Display for RelocationProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "relocation projection failed: {self:?}")
    }
}

impl std::error::Error for RelocationProjectionError {}

impl From<CanonicalError> for RelocationProjectionError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

/// One actor-safe value bound to a declared action role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActorSafeBindingValue {
    /// The actor receiving the policy input.
    Actor(ActorId),
    /// An opportunity-local reference to a visible object.
    Object(ActorSafeObjectRef),
}

/// One complete actor-safe action-role binding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorSafeBinding {
    name: BindingName,
    value: ActorSafeBindingValue,
}

impl ActorSafeBinding {
    fn new(name: BindingName, value: ActorSafeBindingValue) -> Self {
        Self { name, value }
    }

    /// Returns the checked action-binding name.
    #[must_use]
    pub const fn name(&self) -> &BindingName {
        &self.name
    }

    /// Returns the actor-safe bound value.
    #[must_use]
    pub const fn value(&self) -> ActorSafeBindingValue {
        self.value
    }
}

/// Closed semantic family of one actor-safe grounded candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GroundedActionInteraction {
    /// Immediate containment transfer.
    ContainmentTransfer,
    /// Relocation start or process-control operation.
    Relocation(RelocationActionVerb),
}

impl GroundedActionInteraction {
    fn write_canonical(self, writer: &mut CanonicalWriter) {
        match self {
            Self::ContainmentTransfer => writer.write_discriminant(0),
            Self::Relocation(verb) => {
                writer.write_discriminant(1);
                writer.write_discriminant(verb.canonical_tag());
            }
        }
    }
}

/// One fully bound action candidate exposed to a controller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundedActionCandidate {
    id: GroundedActionCandidateId,
    opportunity: ActionOpportunityId,
    action: DefinitionKey,
    interaction: GroundedActionInteraction,
    bindings: Vec<ActorSafeBinding>,
}

impl GroundedActionCandidate {
    /// Returns the stable candidate identity.
    #[must_use]
    pub const fn id(&self) -> GroundedActionCandidateId {
        self.id
    }

    /// Returns the action opportunity that owns this candidate.
    #[must_use]
    pub const fn opportunity(&self) -> ActionOpportunityId {
        self.opportunity
    }

    /// Returns the durable checked action definition key.
    #[must_use]
    pub const fn action(&self) -> &DefinitionKey {
        &self.action
    }

    /// Returns the candidate's closed actor-visible interaction kind.
    #[must_use]
    pub const fn interaction(&self) -> GroundedActionInteraction {
        self.interaction
    }

    /// Returns complete role bindings in canonical binding-name order.
    #[must_use]
    pub fn bindings(&self) -> &[ActorSafeBinding] {
        &self.bindings
    }
}

/// Whether the bounded candidate set exhausts the actor-safe universe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateCoverage {
    /// Every discoverable binding in the supplied scope was included.
    Complete,
    /// More discoverable bindings existed than the declared candidate limit.
    BudgetLimited,
}

impl CandidateCoverage {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Complete => 0,
            Self::BudgetLimited => 1,
        }
    }
}

/// Bounded, canonically ordered candidates for one action opportunity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundedActionCandidateSet {
    opportunity: ActionOpportunityId,
    grounding_semantics: GroundingSemanticsId,
    candidate_limit: u32,
    coverage: CandidateCoverage,
    candidates: Vec<GroundedActionCandidate>,
    fingerprint: GroundedCandidateSetFingerprint,
}

impl GroundedActionCandidateSet {
    /// Returns the owning action opportunity.
    #[must_use]
    pub const fn opportunity(&self) -> ActionOpportunityId {
        self.opportunity
    }

    /// Returns the exact grounding behavior identity.
    #[must_use]
    pub const fn grounding_semantics(&self) -> GroundingSemanticsId {
        self.grounding_semantics
    }

    /// Returns the configured positive candidate limit.
    #[must_use]
    pub const fn candidate_limit(&self) -> u32 {
        self.candidate_limit
    }

    /// Returns whether the set is exhaustive under its actor-safe basis.
    #[must_use]
    pub const fn coverage(&self) -> CandidateCoverage {
        self.coverage
    }

    /// Returns candidates in canonical actor-safe binding order.
    #[must_use]
    pub fn candidates(&self) -> &[GroundedActionCandidate] {
        &self.candidates
    }

    /// Returns the canonical candidate-set fingerprint.
    #[must_use]
    pub const fn fingerprint(&self) -> GroundedCandidateSetFingerprint {
        self.fingerprint
    }

    /// Returns whether this exact set supplied a candidate identity.
    #[must_use]
    pub fn contains(&self, candidate: GroundedActionCandidateId) -> bool {
        self.candidates
            .iter()
            .any(|supplied| supplied.id == candidate)
    }
}

/// Actor-safe objects participating in the containment interaction frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorSafeContainmentInteraction {
    source: ActorSafeObjectRef,
    destinations: Vec<ActorSafeObjectRef>,
    items: Vec<ActorSafeObjectRef>,
}

impl ActorSafeContainmentInteraction {
    /// Returns the actor-visible source reference.
    #[must_use]
    pub const fn source(&self) -> ActorSafeObjectRef {
        self.source
    }

    /// Returns destination references in canonical actor-safe order.
    #[must_use]
    pub fn destinations(&self) -> &[ActorSafeObjectRef] {
        &self.destinations
    }

    /// Returns item references represented by the bounded candidate set.
    #[must_use]
    pub fn items(&self) -> &[ActorSafeObjectRef] {
        &self.items
    }
}

/// One actor-visible relocation operation and its endpoint references.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorSafeRelocationInteractionEntry {
    verb: RelocationActionVerb,
    source: ActorSafeObjectRef,
    destination: ActorSafeObjectRef,
}

impl ActorSafeRelocationInteractionEntry {
    /// Returns the start or process-control verb.
    #[must_use]
    pub const fn verb(self) -> RelocationActionVerb {
        self.verb
    }

    /// Returns the actor-safe departure reference.
    #[must_use]
    pub const fn source(self) -> ActorSafeObjectRef {
        self.source
    }

    /// Returns the actor-safe arrival reference.
    #[must_use]
    pub const fn destination(self) -> ActorSafeObjectRef {
        self.destination
    }
}

/// Bounded actor-safe relocation frame represented by the candidate set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorSafeRelocationInteraction {
    interactions: Vec<ActorSafeRelocationInteractionEntry>,
}

impl ActorSafeRelocationInteraction {
    /// Returns interactions in the same canonical order as their candidates.
    #[must_use]
    pub fn interactions(&self) -> &[ActorSafeRelocationInteractionEntry] {
        &self.interactions
    }
}

/// Closed actor-safe interaction view supplied to the action policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActorSafeActionInteraction {
    /// Visible object anchors for immediate containment transfer.
    Containment(ActorSafeContainmentInteraction),
    /// Visible endpoints and verbs for relocation control.
    Relocation(ActorSafeRelocationInteraction),
}

impl ActorSafeActionInteraction {
    /// Returns the containment frame when this payload grounds containment.
    #[must_use]
    pub const fn containment(&self) -> Option<&ActorSafeContainmentInteraction> {
        match self {
            Self::Containment(interaction) => Some(interaction),
            Self::Relocation(_) => None,
        }
    }

    /// Returns the relocation frame when this payload grounds relocation.
    #[must_use]
    pub const fn relocation(&self) -> Option<&ActorSafeRelocationInteraction> {
        match self {
            Self::Containment(_) => None,
            Self::Relocation(interaction) => Some(interaction),
        }
    }
}

/// Complete immutable payload supplied to an action policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionContextPayload {
    actor: ActorId,
    opportunity: ActionOpportunityId,
    interaction: ActorSafeActionInteraction,
    candidates: GroundedActionCandidateSet,
    policy_semantics: ActionPolicySemanticsId,
    input_fingerprint: ActionInputFingerprint,
}

impl ActionContextPayload {
    /// Returns the actor receiving this input.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the action opportunity being decided.
    #[must_use]
    pub const fn opportunity(&self) -> ActionOpportunityId {
        self.opportunity
    }

    /// Returns the closed actor-safe interaction view.
    #[must_use]
    pub const fn interaction(&self) -> &ActorSafeActionInteraction {
        &self.interaction
    }

    /// Returns the bounded grounded candidates.
    #[must_use]
    pub const fn candidates(&self) -> &GroundedActionCandidateSet {
        &self.candidates
    }

    /// Returns the policy behavior identity bound into this input.
    #[must_use]
    pub const fn policy_semantics(&self) -> ActionPolicySemanticsId {
        self.policy_semantics
    }

    /// Returns the canonical fingerprint of the complete visible payload.
    #[must_use]
    pub const fn input_fingerprint(&self) -> ActionInputFingerprint {
        self.input_fingerprint
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PrivateObjectResolution {
    actor_safe: ActorSafeObjectRef,
    exact: EntityId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PrivateCandidateResolution {
    Containment {
        candidate: GroundedActionCandidateId,
        action: DefinitionKey,
        actor: ActorId,
        item: ActorSafeObjectRef,
        source: ActorSafeObjectRef,
        destination: ActorSafeObjectRef,
    },
    Relocation {
        candidate: GroundedActionCandidateId,
        action: DefinitionKey,
        actor: ActorId,
        interaction: RelocationInteraction,
    },
}

impl PrivateCandidateResolution {
    const fn candidate(&self) -> GroundedActionCandidateId {
        match self {
            Self::Containment { candidate, .. } | Self::Relocation { candidate, .. } => *candidate,
        }
    }
}

/// Exact containment-transfer bindings recovered after trusted selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedContainmentTransfer {
    action: DefinitionKey,
    actor: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
}

impl ResolvedContainmentTransfer {
    /// Returns the durable action key.
    #[must_use]
    pub const fn action(&self) -> &DefinitionKey {
        &self.action
    }

    /// Returns the exact actor binding.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the exact item binding.
    #[must_use]
    pub const fn item(&self) -> EntityId {
        self.item
    }

    /// Returns the exact source binding.
    #[must_use]
    pub const fn source(&self) -> EntityId {
        self.source
    }

    /// Returns the exact destination binding.
    #[must_use]
    pub const fn destination(&self) -> EntityId {
        self.destination
    }
}

/// Exact private relocation selection recovered after policy selection.
///
/// This value carries no authoritative route body, process identity, process
/// version, wake generation, or current process status. Runtime authority
/// resolves and validates those values from current state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedRelocationAction {
    action: DefinitionKey,
    actor: ActorId,
    interaction: RelocationInteraction,
}

impl ResolvedRelocationAction {
    /// Returns the durable authored action key selected for the verb.
    #[must_use]
    pub const fn action(&self) -> &DefinitionKey {
        &self.action
    }

    /// Returns the actor who owned the action opportunity.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the exact scoped interaction, including its route identity.
    #[must_use]
    pub const fn interaction(&self) -> RelocationInteraction {
        self.interaction
    }
}

/// Closed private result of resolving a selected grounded candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedActionSelection {
    /// Immediate command lowering for containment transfer.
    Containment(ResolvedContainmentTransfer),
    /// Relocation operation for runtime-owned process validation.
    Relocation(ResolvedRelocationAction),
}

impl ResolvedActionSelection {
    /// Returns exact transfer bindings for a containment selection.
    #[must_use]
    pub const fn containment(&self) -> Option<&ResolvedContainmentTransfer> {
        match self {
            Self::Containment(selection) => Some(selection),
            Self::Relocation(_) => None,
        }
    }

    /// Returns exact route material for a relocation selection.
    #[must_use]
    pub const fn relocation(&self) -> Option<&ResolvedRelocationAction> {
        match self {
            Self::Containment(_) => None,
            Self::Relocation(selection) => Some(selection),
        }
    }
}

/// Opaque private map from supplied candidate IDs to exact model references.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateResolutionTable {
    references: Vec<PrivateObjectResolution>,
    candidates: Vec<PrivateCandidateResolution>,
}

impl CandidateResolutionTable {
    /// Returns the number of supplied candidates with private resolutions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    /// Returns whether the table contains no candidate resolutions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Resolves one supplied candidate to exact lowering bindings.
    ///
    /// Unknown or fabricated IDs return `None`; callers cannot add bindings
    /// through this boundary.
    #[must_use]
    pub fn resolve(&self, candidate: GroundedActionCandidateId) -> Option<ResolvedActionSelection> {
        let resolution = self
            .candidates
            .iter()
            .find(|resolution| resolution.candidate() == candidate)?;
        match resolution {
            PrivateCandidateResolution::Containment {
                action,
                actor,
                item,
                source,
                destination,
                ..
            } => Some(ResolvedActionSelection::Containment(
                ResolvedContainmentTransfer {
                    action: action.clone(),
                    actor: *actor,
                    item: self.resolve_object(*item)?,
                    source: self.resolve_object(*source)?,
                    destination: self.resolve_object(*destination)?,
                },
            )),
            PrivateCandidateResolution::Relocation {
                action,
                actor,
                interaction,
                ..
            } => Some(ResolvedActionSelection::Relocation(
                ResolvedRelocationAction {
                    action: action.clone(),
                    actor: *actor,
                    interaction: *interaction,
                },
            )),
        }
    }

    fn resolve_object(&self, actor_safe: ActorSafeObjectRef) -> Option<EntityId> {
        self.references
            .binary_search_by_key(&actor_safe, |entry| entry.actor_safe)
            .ok()
            .map(|index| self.references[index].exact)
    }
}

/// One exact actor-local belief observation used during containment
/// projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainmentBeliefObservation {
    item: EntityId,
    believed_container: Option<EntityId>,
}

impl ContainmentBeliefObservation {
    /// Returns the exact scoped item that was observed.
    #[must_use]
    pub const fn item(&self) -> EntityId {
        self.item
    }

    /// Returns the believed direct container, or `None` when the actor had no
    /// direct containment belief for this item.
    #[must_use]
    pub const fn believed_container(&self) -> Option<EntityId> {
        self.believed_container
    }
}

/// Complete narrow policy-read witness for containment projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainmentPolicyWitness {
    actor: ActorId,
    observations: Vec<ContainmentBeliefObservation>,
}

impl ContainmentPolicyWitness {
    /// Returns the actor whose epistemic partition was observed.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns one exact belief-or-absence observation for every scoped item.
    #[must_use]
    pub fn observations(&self) -> &[ContainmentBeliefObservation] {
        &self.observations
    }
}

/// Closed accepted-state read witness for actor-safe action projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionProjectionWitness {
    /// Exact actor-local containment beliefs and absences used by grounding.
    Containment(ContainmentPolicyWitness),
    /// Relocation grounding reads only opportunity anchors and definitions.
    RelocationNoRead,
}

/// Hidden containment facts used only to detect a need for fresh runtime
/// legality validation before lowering one retained candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainmentCandidateExecutionWitness {
    candidate: GroundedActionCandidateId,
    item_container: Option<EntityId>,
    source_exists: bool,
    actor_controls_source: bool,
    destination_capacity: Option<u32>,
    destination_direct_item_count: u64,
}

impl ContainmentCandidateExecutionWitness {
    /// Returns the grounded candidate observed by this witness.
    #[must_use]
    pub const fn candidate(self) -> GroundedActionCandidateId {
        self.candidate
    }

    /// Returns the item's authoritative direct container, when present.
    #[must_use]
    pub const fn item_container(self) -> Option<EntityId> {
        self.item_container
    }

    /// Returns whether the exact source existed as a container.
    #[must_use]
    pub const fn source_exists(self) -> bool {
        self.source_exists
    }

    /// Returns whether the actor had hard authority at the exact source.
    #[must_use]
    pub const fn actor_controls_source(self) -> bool {
        self.actor_controls_source
    }

    /// Returns the exact destination capacity, when the destination existed.
    #[must_use]
    pub const fn destination_capacity(self) -> Option<u32> {
        self.destination_capacity
    }

    /// Returns the accepted direct-item count at the destination.
    #[must_use]
    pub const fn destination_direct_item_count(self) -> u64 {
        self.destination_direct_item_count
    }
}

/// Closed private legality-read witness for retained action evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionExecutionWitness {
    /// Per-candidate containment facts that runtime semantics will revalidate.
    Containment(Vec<ContainmentCandidateExecutionWitness>),
    /// Relocation legality is entirely runtime-owned and has no context read.
    RelocationNoRead,
}

/// One context-owned private record of every accepted-state read used while
/// preparing an action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionReadWitness {
    projection: ActionProjectionWitness,
    execution: ActionExecutionWitness,
}

impl ActionReadWitness {
    /// Borrows the semantic reads that determined actor-safe policy input.
    #[must_use]
    pub const fn projection(&self) -> &ActionProjectionWitness {
        &self.projection
    }

    /// Borrows hidden reads used to classify fresh execution validation.
    #[must_use]
    pub const fn execution(&self) -> &ActionExecutionWitness {
        &self.execution
    }

    /// Separates the two closed read families retained by this artifact.
    #[must_use]
    pub fn into_parts(self) -> (ActionProjectionWitness, ActionExecutionWitness) {
        (self.projection, self.execution)
    }
}

/// Actor-safe payload paired with private lowering and one narrow read record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionContextBuild {
    payload: ActionContextPayload,
    resolution: CandidateResolutionTable,
    read_witness: ActionReadWitness,
}

impl ActionContextBuild {
    /// Borrows the only value that may cross the action-policy boundary.
    #[must_use]
    pub const fn payload(&self) -> &ActionContextPayload {
        &self.payload
    }

    /// Borrows private exact-reference material for trusted lowering.
    #[must_use]
    pub const fn resolution(&self) -> &CandidateResolutionTable {
        &self.resolution
    }

    /// Borrows the complete private record of accepted-state reads.
    #[must_use]
    pub const fn read_witness(&self) -> &ActionReadWitness {
        &self.read_witness
    }

    /// Separates policy input from private lowering and read material.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        ActionContextPayload,
        CandidateResolutionTable,
        ActionReadWitness,
    ) {
        (self.payload, self.resolution, self.read_witness)
    }
}

/// Pure projector for the checked containment-transfer action family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainmentTransferProjector<'actions> {
    actions: &'actions ContainmentTransferActionDefinitions,
}

impl<'actions> ContainmentTransferProjector<'actions> {
    /// Constructs the concrete containment-transfer grounder for one checked
    /// authored action family.
    #[must_use]
    pub const fn new(actions: &'actions ContainmentTransferActionDefinitions) -> Self {
        Self { actions }
    }

    /// Returns the exact behavior identity of this projector.
    #[must_use]
    pub fn semantics_id(self) -> GroundingSemanticsId {
        let mut writer = CanonicalWriter::new(GROUNDING_SEMANTICS_DOMAIN);
        writer.write_u16(IDENTITY_SCHEMA_VERSION);
        GroundingSemanticsId(ContentDigest::of_canonical(&writer.finish()).into_bytes())
    }

    /// Builds one bounded actor-safe frame and its private resolution table.
    ///
    /// Scoped items are discovered only from the actor's accepted
    /// containment beliefs. Authoritative location, control, destination
    /// capacity, and occupancy remain private runtime legality concerns.
    pub fn build(
        self,
        snapshot: &WorldSnapshot,
        actor: ActorId,
        opportunity: ActionOpportunityId,
        scope: &ContainmentInteractionScope,
        definitions: &RuntimeDefinitionSet,
        policy_semantics: ActionPolicySemanticsId,
    ) -> Result<ActionContextBuild, ContainmentProjectionError> {
        if self.actions.definition_set != definitions.digest() {
            return Err(ContainmentProjectionError::DefinitionSetMismatch {
                expected: self.actions.definition_set,
                actual: definitions.digest(),
            });
        }
        let actions = self
            .actions
            .actions
            .iter()
            .map(|key| {
                let action = definitions.action(key).unwrap_or_else(|| {
                    unreachable!(
                        "containment-transfer family was checked against this definition set"
                    )
                });
                (key.clone(), action)
            })
            .collect::<Vec<_>>();

        let grounding_semantics = self.semantics_id();
        let source = scope.source();
        let source_ref = derive_object_ref(actor, opportunity, source, grounding_semantics)?;

        let mut destinations = scope
            .destinations()
            .iter()
            .copied()
            .map(|exact| {
                derive_object_ref(actor, opportunity, exact, grounding_semantics)
                    .map(|actor_safe| (actor_safe, exact))
            })
            .collect::<Result<Vec<_>, CanonicalError>>()?;
        destinations.sort_by_key(|(actor_safe, _)| *actor_safe);
        reject_reference_collisions(&destinations)?;

        let epistemic = snapshot.accepted().epistemic();
        let projection_witness = ActionProjectionWitness::Containment(ContainmentPolicyWitness {
            actor,
            observations: scope
                .items()
                .iter()
                .copied()
                .map(|item| ContainmentBeliefObservation {
                    item,
                    believed_container: epistemic
                        .contained_in(actor, item)
                        .map(|belief| belief.container()),
                })
                .collect(),
        });
        let mut items = scope
            .items()
            .iter()
            .copied()
            .filter(|item| {
                epistemic
                    .contained_in(actor, *item)
                    .is_some_and(|belief| belief.container() == source)
            })
            .map(|exact| {
                derive_object_ref(actor, opportunity, exact, grounding_semantics)
                    .map(|actor_safe| (actor_safe, exact))
            })
            .collect::<Result<Vec<_>, CanonicalError>>()?;
        items.sort_by_key(|(actor_safe, _)| *actor_safe);
        reject_reference_collisions(&items)?;

        let candidate_limit = scope.candidate_limit();
        let limit = usize::try_from(candidate_limit)
            .map_err(|_| CanonicalError::LengthOverflow { length: usize::MAX })?;
        let universe_size =
            (actions.len() as u128) * (destinations.len() as u128) * (items.len() as u128);
        let coverage = if universe_size > u128::from(candidate_limit) {
            CandidateCoverage::BudgetLimited
        } else {
            CandidateCoverage::Complete
        };

        let mut public_candidates = Vec::with_capacity(
            limit.min(
                actions
                    .len()
                    .saturating_mul(destinations.len())
                    .saturating_mul(items.len()),
            ),
        );
        let mut private_candidates = Vec::with_capacity(public_candidates.capacity());
        let mut execution_observations = Vec::with_capacity(public_candidates.capacity());
        let mut selected_items = BTreeSet::new();
        let mut candidate_ids = BTreeSet::new();

        'actions: for (action_key, action) in &actions {
            for (destination_ref, destination) in &destinations {
                for (item_ref, item) in &items {
                    if public_candidates.len() == limit {
                        break 'actions;
                    }
                    let bindings = actor_safe_bindings(
                        action.bindings(),
                        actor,
                        *item_ref,
                        source_ref,
                        *destination_ref,
                    );
                    let candidate = derive_candidate_id(
                        opportunity,
                        action_key,
                        GroundedActionInteraction::ContainmentTransfer,
                        &bindings,
                        grounding_semantics,
                    )?;
                    if !candidate_ids.insert(candidate) {
                        return Err(ContainmentProjectionError::CandidateIdentityCollision {
                            candidate,
                        });
                    }
                    public_candidates.push(GroundedActionCandidate {
                        id: candidate,
                        opportunity,
                        action: action_key.clone(),
                        interaction: GroundedActionInteraction::ContainmentTransfer,
                        bindings,
                    });
                    private_candidates.push(PrivateCandidateResolution::Containment {
                        candidate,
                        action: action_key.clone(),
                        actor,
                        item: *item_ref,
                        source: source_ref,
                        destination: *destination_ref,
                    });
                    let view = snapshot.accepted().domain().containment_transfer_view(
                        actor,
                        *item,
                        source,
                        *destination,
                    );
                    execution_observations.push(ContainmentCandidateExecutionWitness {
                        candidate,
                        item_container: view.item_container(),
                        source_exists: view.source_exists(),
                        actor_controls_source: view.actor_controls_source(),
                        destination_capacity: view.destination_capacity(),
                        destination_direct_item_count: view.destination_direct_item_count(),
                    });
                    selected_items.insert(*item_ref);
                }
            }
        }

        let public_destinations = destinations
            .iter()
            .map(|(actor_safe, _)| *actor_safe)
            .collect();
        let public_items = selected_items.iter().copied().collect();
        let interaction =
            ActorSafeActionInteraction::Containment(ActorSafeContainmentInteraction {
                source: source_ref,
                destinations: public_destinations,
                items: public_items,
            });

        let candidate_set_fingerprint = candidate_set_fingerprint(
            opportunity,
            grounding_semantics,
            candidate_limit,
            coverage,
            &public_candidates,
        )?;
        let candidate_set = GroundedActionCandidateSet {
            opportunity,
            grounding_semantics,
            candidate_limit,
            coverage,
            candidates: public_candidates,
            fingerprint: candidate_set_fingerprint,
        };
        let input_fingerprint = action_input_fingerprint(
            actor,
            opportunity,
            &interaction,
            &candidate_set,
            policy_semantics,
        )?;

        let mut references = BTreeMap::new();
        register_reference(&mut references, source_ref, source)?;
        for (actor_safe, exact) in &destinations {
            register_reference(&mut references, *actor_safe, *exact)?;
        }
        for (actor_safe, exact) in &items {
            if selected_items.contains(actor_safe) {
                register_reference(&mut references, *actor_safe, *exact)?;
            }
        }
        let references = references
            .into_iter()
            .map(|(actor_safe, exact)| PrivateObjectResolution { actor_safe, exact })
            .collect();

        Ok(ActionContextBuild {
            payload: ActionContextPayload {
                actor,
                opportunity,
                interaction,
                candidates: candidate_set,
                policy_semantics,
                input_fingerprint,
            },
            resolution: CandidateResolutionTable {
                references,
                candidates: private_candidates,
            },
            read_witness: ActionReadWitness {
                projection: projection_witness,
                execution: ActionExecutionWitness::Containment(execution_observations),
            },
        })
    }
}

/// Pure projector for the checked start, pause, and resume relocation family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelocationProjector<'actions> {
    actions: &'actions RelocationActionDefinitions,
}

impl<'actions> RelocationProjector<'actions> {
    /// Constructs the concrete relocation grounder for one checked authored
    /// action family.
    #[must_use]
    pub const fn new(actions: &'actions RelocationActionDefinitions) -> Self {
        Self { actions }
    }

    /// Returns the exact behavior identity of this projector.
    #[must_use]
    pub fn semantics_id(self) -> GroundingSemanticsId {
        let mut writer = CanonicalWriter::new(RELOCATION_GROUNDING_SEMANTICS_DOMAIN);
        writer.write_u16(IDENTITY_SCHEMA_VERSION);
        GroundingSemanticsId(ContentDigest::of_canonical(&writer.finish()).into_bytes())
    }

    /// Builds actor-safe relocation candidates and exact private interaction
    /// resolutions from the opportunity's actor-visible anchors.
    pub fn build(
        self,
        actor: ActorId,
        opportunity: ActionOpportunityId,
        scope: &RelocationInteractionScope,
        definitions: &RuntimeDefinitionSet,
        policy_semantics: ActionPolicySemanticsId,
    ) -> Result<ActionContextBuild, RelocationProjectionError> {
        if self.actions.definition_set != definitions.digest() {
            return Err(RelocationProjectionError::DefinitionSetMismatch {
                expected: self.actions.definition_set,
                actual: definitions.digest(),
            });
        }

        let grounding_semantics = self.semantics_id();
        let mut reference_owners = BTreeMap::new();
        let mut candidate_ids = BTreeSet::new();
        let mut candidates = Vec::with_capacity(scope.anchors().len());

        for anchor in scope.anchors().iter().copied() {
            let interaction = anchor.interaction();
            let source =
                derive_object_ref(actor, opportunity, anchor.source(), grounding_semantics)?;
            let destination = derive_object_ref(
                actor,
                opportunity,
                anchor.destination(),
                grounding_semantics,
            )?;
            register_relocation_reference(&mut reference_owners, source, anchor.source())?;
            register_relocation_reference(
                &mut reference_owners,
                destination,
                anchor.destination(),
            )?;

            let verb = RelocationActionVerb::from(interaction);
            let action_key = self.actions.action(verb);
            let action = definitions.action(action_key).unwrap_or_else(|| {
                unreachable!("relocation family was checked against this exact definition set")
            });
            let bindings =
                relocation_actor_safe_bindings(action.bindings(), actor, source, destination);
            let candidate = derive_candidate_id(
                opportunity,
                action_key,
                GroundedActionInteraction::Relocation(verb),
                &bindings,
                grounding_semantics,
            )?;
            if !candidate_ids.insert(candidate) {
                return Err(RelocationProjectionError::CandidateIdentityCollision { candidate });
            }
            candidates.push((
                GroundedActionCandidate {
                    id: candidate,
                    opportunity,
                    action: action_key.clone(),
                    interaction: GroundedActionInteraction::Relocation(verb),
                    bindings,
                },
                ActorSafeRelocationInteractionEntry {
                    verb,
                    source,
                    destination,
                },
                PrivateCandidateResolution::Relocation {
                    candidate,
                    action: action_key.clone(),
                    actor,
                    interaction,
                },
            ));
        }

        // Budget selection depends only on actor-safe candidate identity, not
        // on private route identity or runtime process state.
        candidates.sort_by_key(|(candidate, _, _)| candidate.id);
        let candidate_limit = scope.candidate_limit();
        let coverage = if candidates.len() > candidate_limit as usize {
            CandidateCoverage::BudgetLimited
        } else {
            CandidateCoverage::Complete
        };
        candidates.truncate(candidate_limit as usize);

        let mut public_candidates = Vec::with_capacity(candidates.len());
        let mut public_interactions = Vec::with_capacity(candidates.len());
        let mut private_candidates = Vec::with_capacity(candidates.len());
        for (candidate, interaction, resolution) in candidates {
            public_candidates.push(candidate);
            public_interactions.push(interaction);
            private_candidates.push(resolution);
        }
        let interaction = ActorSafeActionInteraction::Relocation(ActorSafeRelocationInteraction {
            interactions: public_interactions,
        });
        let candidate_set_fingerprint = candidate_set_fingerprint(
            opportunity,
            grounding_semantics,
            candidate_limit,
            coverage,
            &public_candidates,
        )?;
        let candidate_set = GroundedActionCandidateSet {
            opportunity,
            grounding_semantics,
            candidate_limit,
            coverage,
            candidates: public_candidates,
            fingerprint: candidate_set_fingerprint,
        };
        let input_fingerprint = action_input_fingerprint(
            actor,
            opportunity,
            &interaction,
            &candidate_set,
            policy_semantics,
        )?;

        Ok(ActionContextBuild {
            payload: ActionContextPayload {
                actor,
                opportunity,
                interaction,
                candidates: candidate_set,
                policy_semantics,
                input_fingerprint,
            },
            resolution: CandidateResolutionTable {
                references: Vec::new(),
                candidates: private_candidates,
            },
            read_witness: ActionReadWitness {
                projection: ActionProjectionWitness::RelocationNoRead,
                execution: ActionExecutionWitness::RelocationNoRead,
            },
        })
    }
}

fn has_containment_transfer_binding_shape(action: &ActionDefinition) -> bool {
    let expected = [
        (ACTOR_ROLE, ValueKind::Actor),
        (DESTINATION_ROLE, ValueKind::Entity),
        (ITEM_ROLE, ValueKind::Entity),
        (SOURCE_ROLE, ValueKind::Entity),
    ];
    action.bindings().len() == expected.len()
        && action
            .bindings()
            .iter()
            .zip(expected)
            .all(|(actual, (name, kind))| {
                actual.name().as_str() == name && *actual.value_kind() == kind
            })
}

fn has_relocation_binding_shape(action: &ActionDefinition) -> bool {
    let expected = [
        (ACTOR_ROLE, ValueKind::Actor),
        (DESTINATION_ROLE, ValueKind::Entity),
        (SOURCE_ROLE, ValueKind::Entity),
    ];
    action.bindings().len() == expected.len()
        && action
            .bindings()
            .iter()
            .zip(expected)
            .all(|(actual, (name, kind))| {
                actual.name().as_str() == name && *actual.value_kind() == kind
            })
}

fn actor_safe_bindings(
    declarations: &[ActionBindingData],
    actor: ActorId,
    item: ActorSafeObjectRef,
    source: ActorSafeObjectRef,
    destination: ActorSafeObjectRef,
) -> Vec<ActorSafeBinding> {
    declarations
        .iter()
        .map(|declaration| {
            let value = match declaration.name().as_str() {
                ACTOR_ROLE => ActorSafeBindingValue::Actor(actor),
                DESTINATION_ROLE => ActorSafeBindingValue::Object(destination),
                ITEM_ROLE => ActorSafeBindingValue::Object(item),
                SOURCE_ROLE => ActorSafeBindingValue::Object(source),
                _ => unreachable!("validated transfer roles are exhaustive"),
            };
            ActorSafeBinding::new(declaration.name().clone(), value)
        })
        .collect()
}

fn relocation_actor_safe_bindings(
    declarations: &[ActionBindingData],
    actor: ActorId,
    source: ActorSafeObjectRef,
    destination: ActorSafeObjectRef,
) -> Vec<ActorSafeBinding> {
    declarations
        .iter()
        .map(|declaration| {
            let value = match declaration.name().as_str() {
                ACTOR_ROLE => ActorSafeBindingValue::Actor(actor),
                DESTINATION_ROLE => ActorSafeBindingValue::Object(destination),
                SOURCE_ROLE => ActorSafeBindingValue::Object(source),
                _ => unreachable!("checked relocation roles are exhaustive"),
            };
            ActorSafeBinding::new(declaration.name().clone(), value)
        })
        .collect()
}

fn derive_object_ref(
    actor: ActorId,
    opportunity: ActionOpportunityId,
    entity: EntityId,
    grounding_semantics: GroundingSemanticsId,
) -> Result<ActorSafeObjectRef, CanonicalError> {
    let mut writer = CanonicalWriter::new(OBJECT_REFERENCE_DOMAIN);
    writer.write_u16(IDENTITY_SCHEMA_VERSION);
    writer.write_bytes(actor.as_bytes())?;
    writer.write_bytes(opportunity.as_bytes())?;
    writer.write_bytes(entity.as_bytes())?;
    writer.write_bytes(grounding_semantics.as_bytes())?;
    Ok(ActorSafeObjectRef(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

fn derive_candidate_id(
    opportunity: ActionOpportunityId,
    action: &DefinitionKey,
    interaction: GroundedActionInteraction,
    bindings: &[ActorSafeBinding],
    grounding_semantics: GroundingSemanticsId,
) -> Result<GroundedActionCandidateId, CanonicalError> {
    let mut writer = CanonicalWriter::new(CANDIDATE_DOMAIN);
    writer.write_u16(IDENTITY_SCHEMA_VERSION);
    writer.write_bytes(opportunity.as_bytes())?;
    write_definition_key(&mut writer, action)?;
    interaction.write_canonical(&mut writer);
    writer.write_sequence(bindings, write_binding)?;
    writer.write_bytes(grounding_semantics.as_bytes())?;
    Ok(GroundedActionCandidateId(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

fn candidate_set_fingerprint(
    opportunity: ActionOpportunityId,
    grounding_semantics: GroundingSemanticsId,
    candidate_limit: u32,
    coverage: CandidateCoverage,
    candidates: &[GroundedActionCandidate],
) -> Result<GroundedCandidateSetFingerprint, CanonicalError> {
    let mut writer = CanonicalWriter::new(CANDIDATE_SET_DOMAIN);
    write_candidate_set_body(
        &mut writer,
        opportunity,
        grounding_semantics,
        candidate_limit,
        coverage,
        candidates,
    )?;
    Ok(GroundedCandidateSetFingerprint(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

fn action_input_fingerprint(
    actor: ActorId,
    opportunity: ActionOpportunityId,
    interaction: &ActorSafeActionInteraction,
    candidates: &GroundedActionCandidateSet,
    policy_semantics: ActionPolicySemanticsId,
) -> Result<ActionInputFingerprint, CanonicalError> {
    let mut writer = CanonicalWriter::new(ACTION_INPUT_DOMAIN);
    writer.write_u16(IDENTITY_SCHEMA_VERSION);
    writer.write_bytes(actor.as_bytes())?;
    writer.write_bytes(opportunity.as_bytes())?;
    write_actor_safe_interaction(&mut writer, interaction)?;
    write_candidate_set_body(
        &mut writer,
        candidates.opportunity,
        candidates.grounding_semantics,
        candidates.candidate_limit,
        candidates.coverage,
        &candidates.candidates,
    )?;
    writer.write_bytes(candidates.fingerprint.as_bytes())?;
    writer.write_bytes(policy_semantics.as_bytes())?;
    Ok(ActionInputFingerprint(
        ContentDigest::of_canonical(&writer.finish()).into_bytes(),
    ))
}

fn write_candidate_set_body(
    writer: &mut CanonicalWriter,
    opportunity: ActionOpportunityId,
    grounding_semantics: GroundingSemanticsId,
    candidate_limit: u32,
    coverage: CandidateCoverage,
    candidates: &[GroundedActionCandidate],
) -> Result<(), CanonicalError> {
    writer.write_u16(IDENTITY_SCHEMA_VERSION);
    writer.write_bytes(opportunity.as_bytes())?;
    writer.write_bytes(grounding_semantics.as_bytes())?;
    writer.write_u32(candidate_limit);
    writer.write_discriminant(coverage.canonical_tag());
    writer.write_sequence(candidates, |writer, candidate| {
        writer.write_bytes(candidate.id.as_bytes())?;
        writer.write_bytes(candidate.opportunity.as_bytes())?;
        write_definition_key(writer, &candidate.action)?;
        candidate.interaction.write_canonical(writer);
        writer.write_sequence(&candidate.bindings, write_binding)
    })
}

fn write_actor_safe_interaction(
    writer: &mut CanonicalWriter,
    interaction: &ActorSafeActionInteraction,
) -> Result<(), CanonicalError> {
    match interaction {
        ActorSafeActionInteraction::Containment(interaction) => {
            writer.write_discriminant(0);
            writer.write_bytes(interaction.source.as_bytes())?;
            writer.write_sequence(&interaction.destinations, |writer, destination| {
                writer.write_bytes(destination.as_bytes())
            })?;
            writer.write_sequence(&interaction.items, |writer, item| {
                writer.write_bytes(item.as_bytes())
            })
        }
        ActorSafeActionInteraction::Relocation(interaction) => {
            writer.write_discriminant(1);
            writer.write_sequence(&interaction.interactions, |writer, interaction| {
                writer.write_discriminant(interaction.verb.canonical_tag());
                writer.write_bytes(interaction.source.as_bytes())?;
                writer.write_bytes(interaction.destination.as_bytes())
            })
        }
    }
}

fn write_definition_key(
    writer: &mut CanonicalWriter,
    key: &DefinitionKey,
) -> Result<(), CanonicalError> {
    writer.write_str(key.pack_key().as_str())?;
    writer.write_str(key.local_name().as_str())
}

fn write_binding(
    writer: &mut CanonicalWriter,
    binding: &ActorSafeBinding,
) -> Result<(), CanonicalError> {
    writer.write_str(binding.name.as_str())?;
    match binding.value {
        ActorSafeBindingValue::Actor(actor) => {
            writer.write_discriminant(0);
            writer.write_bytes(actor.as_bytes())
        }
        ActorSafeBindingValue::Object(object) => {
            writer.write_discriminant(1);
            writer.write_bytes(object.as_bytes())
        }
    }
}

fn reject_reference_collisions(
    references: &[(ActorSafeObjectRef, EntityId)],
) -> Result<(), ContainmentProjectionError> {
    if let Some(reference) = references
        .windows(2)
        .find_map(|pair| (pair[0].0 == pair[1].0 && pair[0].1 != pair[1].1).then_some(pair[0].0))
    {
        Err(ContainmentProjectionError::ObjectReferenceCollision { reference })
    } else {
        Ok(())
    }
}

fn register_reference(
    references: &mut BTreeMap<ActorSafeObjectRef, EntityId>,
    actor_safe: ActorSafeObjectRef,
    exact: EntityId,
) -> Result<(), ContainmentProjectionError> {
    if let Some(previous) = references.insert(actor_safe, exact)
        && previous != exact
    {
        return Err(ContainmentProjectionError::ObjectReferenceCollision {
            reference: actor_safe,
        });
    }
    Ok(())
}

fn register_relocation_reference(
    references: &mut BTreeMap<ActorSafeObjectRef, EntityId>,
    actor_safe: ActorSafeObjectRef,
    exact: EntityId,
) -> Result<(), RelocationProjectionError> {
    if let Some(previous) = references.insert(actor_safe, exact)
        && previous != exact
    {
        return Err(RelocationProjectionError::ObjectReferenceCollision {
            reference: actor_safe,
        });
    }
    Ok(())
}
