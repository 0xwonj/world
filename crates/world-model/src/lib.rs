//! Immutable accepted-world values and command protocol records.
//!
//! This crate owns checked model data and its canonical identities. It does
//! not own an aggregate root, a store, record publication, or mutation
//! authority.

mod accepted;
mod action_opportunity;
mod appraisal;
mod command;
mod process;
mod snapshot;
mod view;

pub use accepted::{
    ACCEPTED_STATE_SCHEMA_VERSION, AGENCY_STATE_SCHEMA_VERSION, AcceptedState, AcceptedStateDigest,
    Activity, ActivityControllerId, ActivityFocus, ActivityGeneration, ActivityId, ActivityState,
    ActivityStateSchemaId, ActivityStateTransitionError, ActivityStatus, ActivityTransition,
    ActivityTransitionError, ActivityTransitionKind, ActivityVersion, ActorArrivedEvent,
    ActorDepartedEvent, ActorEpistemicRecord, ActorLocation, ActorPosition,
    ActorRelocationObservation, AgencyState, AgencyStateDigest, AgencyStateError,
    AgencyTransitionError, ContainedInBelief, ContainedInBeliefError, ContainerAuthorityRecord,
    ContainerRecord, ContainmentRecord, ContainmentTransferActivityState,
    ContainmentTransferActivityStateError, ContainmentTransferDelta, ContainmentTransferError,
    DOMAIN_STATE_SCHEMA_VERSION, DesiredCondition, DirectedRoute, DirectedRouteError, DomainState,
    DomainStateDigest, DomainStateError, EPISTEMIC_STATE_SCHEMA_VERSION, EpistemicState,
    EpistemicStateDigest, EpistemicStateError, EpistemicTransitionError, EpistemicVersion,
    EvidenceDeliveryGeneration, EvidenceDeliveryId, EvidenceProvenance, EvidenceRecord, Intent,
    IntentGeneration, IntentId, IntentStatus, IntentTransition, IntentTransitionError,
    IntentVersion, ItemAbsentFromContainerObservation, ItemTransferredEvent, PhysicalEvent,
    RELOCATION_ROUTE_ID_SCHEMA_VERSION, RelocationRouteId, SOCIAL_STATE_SCHEMA_VERSION,
    SocialState, SocialStateDigest, TravelActivityState, TravelActivityStateError,
    TravelActivityStep,
};
pub use action_opportunity::{
    ACTION_EVALUATION_INVOCATION_ID_SCHEMA_VERSION, ACTION_OPPORTUNITY_ID_SCHEMA_VERSION,
    ACTION_OPPORTUNITY_SCHEMA_VERSION, ActionEvaluationGeneration, ActionEvaluationInvocationId,
    ActionInteractionScope, ActionOpportunity, ActionOpportunityDigest,
    ActionOpportunityDisposition, ActionOpportunityGeneration, ActionOpportunityId,
    ActionOpportunityState, ActionOpportunityTransitionError, ActionOpportunityVersion,
    ActionSponsor, ActivitySponsor, ActorReactionCause, ContainmentInteractionScope,
    ContainmentInteractionScopeError, RelocationInteraction, RelocationInteractionAnchor,
    RelocationInteractionScope, RelocationInteractionScopeError,
};
pub use appraisal::{
    CONTAINMENT_APPRAISAL_MATERIAL_SCHEMA_VERSION, ContainmentAppraisal,
    ContainmentAppraisalFingerprint,
};
pub use command::{
    COMMAND_REQUEST_SCHEMA_VERSION, COMMAND_SOURCE_SCHEMA_VERSION, CommandAttemptOutcome,
    CommandBinding, CommandEnvelope, CommandEnvelopeError, CommandId, CommandRequestFingerprint,
    CommandSource, CommandValue, StableCommandRejection, SystemCommandSourceId,
};
pub use process::{
    RELOCATION_PROCESS_ID_SCHEMA_VERSION, RELOCATION_PROCESS_SCHEMA_VERSION, RelocationProcess,
    RelocationProcessDigest, RelocationProcessError, RelocationProcessGeneration,
    RelocationProcessId, RelocationProcessStatus, RelocationProcessVersion,
    RelocationWakeGeneration,
};
pub use snapshot::WorldSnapshot;
pub use view::ContainmentTransferReadView;
