use world_core::{CausalTransactionId, DefinitionId, EntityId, EventRecordId};
use world_defs::{BindingRuleKind, RequirementKind, RoleName};
use world_model::{DerivedViewInvalidationReport, RelationFamily};

/// Result of accepted runtime work that reached a domain outcome.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeOutcome {
    /// Work committed a hard causal transaction.
    Committed(CommittedOutcome),
    /// Work was rejected before hard mutation.
    Rejected(RejectedOutcome),
    /// Work could not proceed now but may be retried later.
    Blocked(BlockedOutcome),
}

impl RuntimeOutcome {
    /// Returns committed outcome data when this outcome committed.
    pub const fn committed(&self) -> Option<&CommittedOutcome> {
        match self {
            Self::Committed(outcome) => Some(outcome),
            Self::Rejected(_) | Self::Blocked(_) => None,
        }
    }
}

/// Committed hard transaction result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedOutcome {
    transaction: CausalTransactionId,
    events: Vec<EventRecordId>,
    invalidation: DerivedViewInvalidationReport,
}

impl CommittedOutcome {
    pub(crate) fn new(
        transaction: CausalTransactionId,
        events: Vec<EventRecordId>,
        invalidation: DerivedViewInvalidationReport,
    ) -> Self {
        Self {
            transaction,
            events,
            invalidation,
        }
    }

    /// Returns the committed transaction id.
    pub const fn transaction(&self) -> CausalTransactionId {
        self.transaction
    }

    /// Returns committed event ids in commit order.
    pub fn events(&self) -> &[EventRecordId] {
        &self.events
    }

    /// Returns model invalidation caused by the commit.
    pub const fn invalidation(&self) -> DerivedViewInvalidationReport {
        self.invalidation
    }
}

/// Request rejection before hard mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RejectedOutcome {
    action: DefinitionId,
    reason: RejectionReason,
}

impl RejectedOutcome {
    pub(crate) const fn new(action: DefinitionId, reason: RejectionReason) -> Self {
        Self { action, reason }
    }

    /// Returns the action definition requested.
    pub const fn action(&self) -> DefinitionId {
        self.action
    }

    /// Returns why the request was rejected.
    pub const fn reason(&self) -> &RejectionReason {
        &self.reason
    }
}

/// Domain-level rejection reason.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RejectionReason {
    /// The requested action is not present in the definition registry.
    UnknownAction { action: DefinitionId },
    /// The request submitted the same role more than once.
    DuplicateRoleBinding { role: RoleName },
    /// The request submitted a role the action does not declare.
    UnknownRoleBinding { role: RoleName },
    /// The request omitted a role the action requires.
    MissingRoleBinding { role: RoleName },
    /// The request actor does not match the bound actor role.
    ActorRoleMismatch {
        /// Actor supplied with the request.
        actor: EntityId,
        /// Entity bound to the actor role.
        bound_actor: EntityId,
    },
    /// Runtime has no evaluator for this declared requirement.
    UnsupportedRequirement {
        /// Requirement kind that cannot be evaluated.
        requirement: RequirementKind,
    },
    /// Runtime has no evaluator for this declared binding rule.
    UnsupportedBindingRule {
        /// Binding rule kind that cannot be evaluated.
        binding_rule: BindingRuleKind,
    },
    /// Runtime validation could not see an entity required by an effect operation.
    MissingEntity {
        /// Role whose entity was missing.
        role: RoleName,
        /// Missing entity id.
        entity: EntityId,
    },
    /// Runtime validation found an entity insertion that would duplicate visible state.
    EntityAlreadyPresent {
        /// Role whose entity already exists.
        role: RoleName,
        /// Existing entity id.
        entity: EntityId,
    },
    /// Runtime validation found a relation insertion that would duplicate visible state.
    RelationAlreadyPresent {
        /// Relation subject.
        subject: EntityId,
        /// Relation family.
        family: RelationFamily,
        /// Relation object.
        object: EntityId,
    },
}

/// Work is valid but blocked before hard mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockedOutcome {
    reason: &'static str,
}

impl BlockedOutcome {
    /// Creates a blocked outcome.
    pub const fn new(reason: &'static str) -> Self {
        Self { reason }
    }

    /// Returns the blocking reason label.
    pub const fn reason(&self) -> &'static str {
        self.reason
    }
}
