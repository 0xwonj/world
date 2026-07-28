use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Included};

use world_core::ActorId;
use world_model::{
    ActionEvaluationInvocationId, ActionOpportunity, ActionOpportunityDisposition,
    ActionOpportunityId, ActionOpportunityTransitionError, ActionOpportunityVersion,
    CommandAttemptOutcome, CommandId, CommandRequestFingerprint, CommandSource,
    ContainmentAppraisal, ContainmentAppraisalFingerprint, StableCommandRejection,
};

use crate::action_evaluation::{ActionEvaluationCaptureLedger, ActionEvaluationInvocationLedger};
use crate::authority::{AttemptRecordId, CapturedInputRecordId};
use crate::kernel::{
    AdmitOutcome, InputId, InputRequestFingerprint, LedgerRetirement, ManageOutcome,
    ManagementRequestFingerprint, ManagementRequestId,
};
use crate::lifecycle::{LifecycleCause, LifecycleGeneration, LifecycleRole};
use crate::relocation::RelocationProcessLedger;
use crate::scheduler::CommandTriggerId;

/// Classification shared by the two singular host-request ledgers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RequestLedgerLookup<T> {
    Absent,
    Retired,
    RetainedExact(T),
    IdReuseMismatch,
}

/// Closed result of one source-scoped command-ledger lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandLedgerLookup {
    Absent,
    Retired,
    RetainedExact {
        original_attempt: AttemptRecordId,
        outcome: CommandAttemptOutcome,
    },
    RetainedCollision {
        original_attempt: AttemptRecordId,
    },
    IdReuseMismatch {
        original_attempt: AttemptRecordId,
    },
}

/// Retained result of one published input request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InputLedgerEntry {
    fingerprint: InputRequestFingerprint,
    captured: CapturedInputRecordId,
    trigger: CommandTriggerId,
    outcome: AdmitOutcome,
}

impl InputLedgerEntry {
    #[must_use]
    pub(crate) const fn fingerprint(self) -> InputRequestFingerprint {
        self.fingerprint
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn captured(self) -> CapturedInputRecordId {
        self.captured
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn trigger(self) -> CommandTriggerId {
        self.trigger
    }

    #[must_use]
    pub(crate) const fn outcome(self) -> AdmitOutcome {
        self.outcome
    }
}

/// Retained result of one published session-management request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ManagementLedgerEntry {
    fingerprint: ManagementRequestFingerprint,
    outcome: ManageOutcome,
}

impl ManagementLedgerEntry {
    #[must_use]
    pub(crate) const fn fingerprint(self) -> ManagementRequestFingerprint {
        self.fingerprint
    }

    #[must_use]
    pub(crate) const fn outcome(self) -> ManageOutcome {
        self.outcome
    }
}

/// Retained terminal state of one source-scoped command sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CommandLedgerEntry {
    Exact {
        fingerprint: CommandRequestFingerprint,
        original_attempt: AttemptRecordId,
        outcome: CommandAttemptOutcome,
    },
    Collision {
        fingerprints: Box<[CommandRequestFingerprint]>,
        original_attempt: AttemptRecordId,
    },
}

impl CommandLedgerEntry {
    #[must_use]
    pub(crate) const fn original_attempt(&self) -> AttemptRecordId {
        match self {
            Self::Exact {
                original_attempt, ..
            }
            | Self::Collision {
                original_attempt, ..
            } => *original_attempt,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn attempt(&self) -> AttemptRecordId {
        self.original_attempt()
    }

    #[must_use]
    pub(crate) const fn outcome(&self) -> CommandAttemptOutcome {
        match self {
            Self::Exact { outcome, .. } => *outcome,
            Self::Collision { .. } => {
                CommandAttemptOutcome::Rejected(StableCommandRejection::IdCollision)
            }
        }
    }

    #[must_use]
    pub(crate) const fn exact_fingerprint(&self) -> Option<CommandRequestFingerprint> {
        match self {
            Self::Exact { fingerprint, .. } => Some(*fingerprint),
            Self::Collision { .. } => None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn collision_fingerprints(&self) -> Option<&[CommandRequestFingerprint]> {
        match self {
            Self::Exact { .. } => None,
            Self::Collision { fingerprints, .. } => Some(fingerprints),
        }
    }
}

/// Insertion of an input ID that is already retained or permanently retired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InputLedgerInsertError {
    id: InputId,
}

impl InputLedgerInsertError {
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn id(self) -> InputId {
        self.id
    }
}

/// Insertion of a management ID that is already retained or permanently retired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ManagementLedgerInsertError {
    id: ManagementRequestId,
}

impl ManagementLedgerInsertError {
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn id(self) -> ManagementRequestId {
        self.id
    }
}

/// Invalid insertion into the source-scoped command ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandLedgerInsertError {
    NotAbsent {
        source: CommandSource,
        id: CommandId,
    },
    CollisionRequiresDistinctFingerprints {
        source: CommandSource,
        id: CommandId,
        distinct_count: usize,
    },
    ExactCannotRepresentCollision {
        source: CommandSource,
        id: CommandId,
    },
}

impl CommandLedgerInsertError {
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn source(self) -> CommandSource {
        match self {
            Self::NotAbsent { source, .. }
            | Self::CollisionRequiresDistinctFingerprints { source, .. }
            | Self::ExactCannotRepresentCollision { source, .. } => source,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn id(self) -> CommandId {
        match self {
            Self::NotAbsent { id, .. } | Self::CollisionRequiresDistinctFingerprints { id, .. } => {
                id
            }
            Self::ExactCannotRepresentCollision { id, .. } => id,
        }
    }
}

/// Why a retained terminal prefix could not advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LedgerRetirementError {
    NotAdvancing {
        retired_through: u64,
        requested: u64,
    },
    Gap {
        missing: u64,
    },
    ManagementTargetNotBeforeRequest {
        target: ManagementRequestId,
        request: ManagementRequestId,
    },
}

/// Retained input outcomes plus a permanent non-reuse frontier.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct InputRequestLedger {
    retired_through: Option<u64>,
    entries: BTreeMap<InputId, InputLedgerEntry>,
}

impl InputRequestLedger {
    #[cfg(test)]
    #[must_use]
    pub(crate) fn retired_through(&self) -> Option<InputId> {
        self.retired_through.map(InputId::new)
    }

    #[must_use]
    pub(crate) fn get(&self, id: InputId) -> Option<InputLedgerEntry> {
        self.entries.get(&id).copied()
    }

    #[must_use]
    pub(crate) fn classify(
        &self,
        id: InputId,
        fingerprint: InputRequestFingerprint,
    ) -> RequestLedgerLookup<AdmitOutcome> {
        if self.is_retired(id) {
            return RequestLedgerLookup::Retired;
        }
        match self.get(id) {
            None => RequestLedgerLookup::Absent,
            Some(entry) if entry.fingerprint() == fingerprint => {
                RequestLedgerLookup::RetainedExact(entry.outcome())
            }
            Some(_) => RequestLedgerLookup::IdReuseMismatch,
        }
    }

    pub(crate) fn insert_exact(
        &mut self,
        id: InputId,
        fingerprint: InputRequestFingerprint,
        captured: CapturedInputRecordId,
        trigger: CommandTriggerId,
        outcome: AdmitOutcome,
    ) -> Result<(), InputLedgerInsertError> {
        if self.is_retired(id) {
            return Err(InputLedgerInsertError { id });
        }
        match self.entries.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(InputLedgerEntry {
                    fingerprint,
                    captured,
                    trigger,
                    outcome,
                });
                Ok(())
            }
            Entry::Occupied(_) => Err(InputLedgerInsertError { id }),
        }
    }

    pub(crate) fn can_retire_through(&self, target: InputId) -> Result<(), LedgerRetirementError> {
        let start = retirement_start(self.retired_through, target.get())?;
        for sequence in start..=target.get() {
            if !self.entries.contains_key(&InputId::new(sequence)) {
                return Err(LedgerRetirementError::Gap { missing: sequence });
            }
        }
        Ok(())
    }

    pub(crate) fn retire_through(&mut self, target: InputId) -> Result<(), LedgerRetirementError> {
        self.can_retire_through(target)?;

        // The frontier is the logical authority; payload removal follows only
        // after the complete next prefix has passed validation.
        self.retired_through = Some(target.get());
        self.entries.retain(|id, _| id.get() > target.get());
        Ok(())
    }

    fn is_retired(&self, id: InputId) -> bool {
        self.retired_through
            .is_some_and(|frontier| id.get() <= frontier)
    }

    #[cfg(test)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = (InputId, InputLedgerEntry)> + '_ {
        self.entries.iter().map(|(id, entry)| (*id, *entry))
    }
}

/// Retained management outcomes plus a permanent non-reuse frontier.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ManagementRequestLedger {
    retired_through: Option<u64>,
    entries: BTreeMap<ManagementRequestId, ManagementLedgerEntry>,
}

impl ManagementRequestLedger {
    #[cfg(test)]
    #[must_use]
    pub(crate) fn retired_through(&self) -> Option<ManagementRequestId> {
        self.retired_through.map(ManagementRequestId::new)
    }

    #[must_use]
    pub(crate) fn get(&self, id: ManagementRequestId) -> Option<ManagementLedgerEntry> {
        self.entries.get(&id).copied()
    }

    #[must_use]
    pub(crate) fn classify(
        &self,
        id: ManagementRequestId,
        fingerprint: ManagementRequestFingerprint,
    ) -> RequestLedgerLookup<ManageOutcome> {
        if self.is_retired(id) {
            return RequestLedgerLookup::Retired;
        }
        match self.get(id) {
            Some(entry) if entry.fingerprint() == fingerprint => {
                RequestLedgerLookup::RetainedExact(entry.outcome())
            }
            Some(_) => RequestLedgerLookup::IdReuseMismatch,
            None => RequestLedgerLookup::Absent,
        }
    }

    pub(crate) fn insert_exact(
        &mut self,
        id: ManagementRequestId,
        fingerprint: ManagementRequestFingerprint,
        outcome: ManageOutcome,
    ) -> Result<(), ManagementLedgerInsertError> {
        if self.is_retired(id) {
            return Err(ManagementLedgerInsertError { id });
        }
        match self.entries.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(ManagementLedgerEntry {
                    fingerprint,
                    outcome,
                });
                Ok(())
            }
            Entry::Occupied(_) => Err(ManagementLedgerInsertError { id }),
        }
    }

    pub(crate) fn can_retire_through(
        &self,
        target: ManagementRequestId,
    ) -> Result<(), LedgerRetirementError> {
        let start = retirement_start(self.retired_through, target.get())?;
        for sequence in start..=target.get() {
            let id = ManagementRequestId::new(sequence);
            if !self.entries.contains_key(&id) {
                return Err(LedgerRetirementError::Gap { missing: sequence });
            }
        }
        Ok(())
    }

    pub(crate) fn retire_through(
        &mut self,
        target: ManagementRequestId,
    ) -> Result<(), LedgerRetirementError> {
        self.can_retire_through(target)?;

        // The frontier is the logical authority; payload removal follows only
        // after the complete next prefix has passed validation.
        self.retired_through = Some(target.get());
        self.entries.retain(|id, _| id.get() > target.get());
        Ok(())
    }

    fn is_retired(&self, id: ManagementRequestId) -> bool {
        self.retired_through
            .is_some_and(|frontier| id.get() <= frontier)
    }

    #[cfg(test)]
    pub(crate) fn iter(
        &self,
    ) -> impl Iterator<Item = (ManagementRequestId, ManagementLedgerEntry)> + '_ {
        self.entries.iter().map(|(id, entry)| (*id, *entry))
    }
}

/// Typed retained command state and retirement frontiers scoped by source.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CommandRequestLedger {
    retired_through: BTreeMap<CommandSource, u64>,
    entries: BTreeMap<(CommandSource, CommandId), CommandLedgerEntry>,
}

impl CommandRequestLedger {
    #[cfg(test)]
    #[must_use]
    pub(crate) fn retired_through(&self, source: CommandSource) -> Option<CommandId> {
        self.retired_through
            .get(&source)
            .copied()
            .map(CommandId::new)
    }

    #[must_use]
    pub(crate) fn get(&self, source: CommandSource, id: CommandId) -> Option<&CommandLedgerEntry> {
        self.entries.get(&(source, id))
    }

    #[must_use]
    pub(crate) fn classify(
        &self,
        source: CommandSource,
        id: CommandId,
        fingerprint: CommandRequestFingerprint,
    ) -> CommandLedgerLookup {
        if self.is_retired(source, id) {
            return CommandLedgerLookup::Retired;
        }
        match self.get(source, id) {
            None => CommandLedgerLookup::Absent,
            Some(entry) if entry.exact_fingerprint() == Some(fingerprint) => {
                CommandLedgerLookup::RetainedExact {
                    original_attempt: entry.original_attempt(),
                    outcome: entry.outcome(),
                }
            }
            Some(entry @ CommandLedgerEntry::Exact { .. }) => {
                CommandLedgerLookup::IdReuseMismatch {
                    original_attempt: entry.original_attempt(),
                }
            }
            Some(entry @ CommandLedgerEntry::Collision { .. }) => {
                CommandLedgerLookup::RetainedCollision {
                    original_attempt: entry.original_attempt(),
                }
            }
        }
    }

    pub(crate) fn insert_exact(
        &mut self,
        source: CommandSource,
        id: CommandId,
        fingerprint: CommandRequestFingerprint,
        attempt: AttemptRecordId,
        outcome: CommandAttemptOutcome,
    ) -> Result<(), CommandLedgerInsertError> {
        if outcome == CommandAttemptOutcome::Rejected(StableCommandRejection::IdCollision) {
            return Err(CommandLedgerInsertError::ExactCannotRepresentCollision { source, id });
        }
        if self.is_retired(source, id) {
            return Err(CommandLedgerInsertError::NotAbsent { source, id });
        }
        match self.entries.entry((source, id)) {
            Entry::Vacant(entry) => {
                entry.insert(CommandLedgerEntry::Exact {
                    fingerprint,
                    original_attempt: attempt,
                    outcome,
                });
                Ok(())
            }
            Entry::Occupied(_) => Err(CommandLedgerInsertError::NotAbsent { source, id }),
        }
    }

    pub(crate) fn insert_collision(
        &mut self,
        source: CommandSource,
        id: CommandId,
        fingerprints: &[CommandRequestFingerprint],
        original_attempt: AttemptRecordId,
    ) -> Result<(), CommandLedgerInsertError> {
        let mut fingerprints = fingerprints.to_vec();
        fingerprints.sort_unstable();
        fingerprints.dedup();
        if fingerprints.len() < 2 {
            return Err(
                CommandLedgerInsertError::CollisionRequiresDistinctFingerprints {
                    source,
                    id,
                    distinct_count: fingerprints.len(),
                },
            );
        }
        if self.is_retired(source, id) {
            return Err(CommandLedgerInsertError::NotAbsent { source, id });
        }

        match self.entries.entry((source, id)) {
            Entry::Vacant(entry) => {
                entry.insert(CommandLedgerEntry::Collision {
                    fingerprints: fingerprints.into_boxed_slice(),
                    original_attempt,
                });
                Ok(())
            }
            Entry::Occupied(_) => Err(CommandLedgerInsertError::NotAbsent { source, id }),
        }
    }

    pub(crate) fn retire_through(
        &mut self,
        source: CommandSource,
        target: CommandId,
    ) -> Result<(), LedgerRetirementError> {
        self.can_retire_through(source, target)?;

        // The source frontier is the logical authority; payload removal
        // follows only after the complete next prefix has passed validation.
        self.retired_through.insert(source, target.get());
        self.entries
            .retain(|(entry_source, id), _| *entry_source != source || id.get() > target.get());
        Ok(())
    }

    pub(crate) fn can_retire_through(
        &self,
        source: CommandSource,
        target: CommandId,
    ) -> Result<(), LedgerRetirementError> {
        let current = self.retired_through.get(&source).copied();
        let start = retirement_start(current, target.get())?;
        for sequence in start..=target.get() {
            if !self
                .entries
                .contains_key(&(source, CommandId::new(sequence)))
            {
                return Err(LedgerRetirementError::Gap { missing: sequence });
            }
        }
        Ok(())
    }

    fn is_retired(&self, source: CommandSource, id: CommandId) -> bool {
        self.retired_through
            .get(&source)
            .is_some_and(|frontier| id.get() <= *frontier)
    }

    #[cfg(test)]
    pub(crate) fn iter(
        &self,
    ) -> impl Iterator<Item = ((CommandSource, CommandId), &CommandLedgerEntry)> + '_ {
        self.entries.iter().map(|(key, entry)| (*key, entry))
    }
}

/// Why an action opportunity could not complete its one-shot transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionOpportunityLedgerError {
    DuplicateOpportunity {
        opportunity: ActionOpportunityId,
    },
    ActorAlreadyHasActiveOpportunity {
        actor: ActorId,
        existing: ActionOpportunityId,
    },
    UnknownOpportunity {
        opportunity: ActionOpportunityId,
    },
    TransitionRejected {
        opportunity: ActionOpportunityId,
        error: ActionOpportunityTransitionError,
    },
}

/// Durable action opportunities indexed by their actor-safe identity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ActionOpportunityLedger {
    entries: BTreeMap<ActionOpportunityId, ActionOpportunity>,
}

impl ActionOpportunityLedger {
    #[must_use]
    fn from_root(opportunities: &[ActionOpportunity]) -> Self {
        let mut entries = BTreeMap::new();
        for opportunity in opportunities {
            if entries
                .insert(opportunity.id(), opportunity.clone())
                .is_some()
            {
                unreachable!("checked initial root cannot contain duplicate opportunities");
            }
        }
        Self { entries }
    }

    #[must_use]
    pub(crate) fn get(&self, id: ActionOpportunityId) -> Option<&ActionOpportunity> {
        self.entries.get(&id)
    }

    pub(crate) fn open(
        &mut self,
        opportunity: ActionOpportunity,
    ) -> Result<&ActionOpportunity, ActionOpportunityLedgerError> {
        if self.entries.contains_key(&opportunity.id()) {
            return Err(ActionOpportunityLedgerError::DuplicateOpportunity {
                opportunity: opportunity.id(),
            });
        }
        if let Some(existing) = self.entries.values().find(|existing| {
            existing.actor() == opportunity.actor()
                && !matches!(
                    existing.state(),
                    world_model::ActionOpportunityState::Consumed(_)
                )
        }) {
            return Err(
                ActionOpportunityLedgerError::ActorAlreadyHasActiveOpportunity {
                    actor: opportunity.actor(),
                    existing: existing.id(),
                },
            );
        }
        let id = opportunity.id();
        self.entries.insert(id, opportunity);
        Ok(self
            .entries
            .get(&id)
            .unwrap_or_else(|| unreachable!("inserted opportunity must remain indexed")))
    }

    pub(crate) fn consume(
        &mut self,
        id: ActionOpportunityId,
        expected_version: ActionOpportunityVersion,
        disposition: ActionOpportunityDisposition,
    ) -> Result<&ActionOpportunity, ActionOpportunityLedgerError> {
        let current = self
            .entries
            .get(&id)
            .ok_or(ActionOpportunityLedgerError::UnknownOpportunity { opportunity: id })?;
        let successor = current
            .consume(expected_version, disposition)
            .map_err(|error| ActionOpportunityLedgerError::TransitionRejected {
                opportunity: id,
                error,
            })?;
        let replaced = self.entries.insert(id, successor);
        if replaced.is_none() {
            unreachable!("successor replaces the opportunity read above");
        }
        Ok(self
            .entries
            .get(&id)
            .unwrap_or_else(|| unreachable!("consumed opportunity must remain indexed")))
    }

    pub(crate) fn begin_evaluation(
        &mut self,
        id: ActionOpportunityId,
        expected_version: ActionOpportunityVersion,
        policy_semantics: [u8; 32],
        action_input_fingerprint: [u8; 32],
    ) -> Result<(&ActionOpportunity, ActionEvaluationInvocationId), ActionOpportunityLedgerError>
    {
        let current = self
            .entries
            .get(&id)
            .ok_or(ActionOpportunityLedgerError::UnknownOpportunity { opportunity: id })?;
        let (successor, invocation) = current
            .begin_evaluation(expected_version, policy_semantics, action_input_fingerprint)
            .map_err(|error| ActionOpportunityLedgerError::TransitionRejected {
                opportunity: id,
                error,
            })?;
        self.entries.insert(id, successor);
        Ok((
            self.entries
                .get(&id)
                .unwrap_or_else(|| unreachable!("waiting opportunity must remain indexed")),
            invocation,
        ))
    }

    pub(crate) fn resume_evaluation(
        &mut self,
        id: ActionOpportunityId,
        expected_version: ActionOpportunityVersion,
        invocation: ActionEvaluationInvocationId,
    ) -> Result<&ActionOpportunity, ActionOpportunityLedgerError> {
        self.transition(id, |current| {
            current.resume_evaluation(expected_version, invocation)
        })
    }

    pub(crate) fn reopen_for_visible_reinvocation(
        &mut self,
        id: ActionOpportunityId,
        expected_version: ActionOpportunityVersion,
        invocation: ActionEvaluationInvocationId,
    ) -> Result<&ActionOpportunity, ActionOpportunityLedgerError> {
        self.transition(id, |current| {
            current.reopen_for_visible_reinvocation(expected_version, invocation)
        })
    }

    fn transition(
        &mut self,
        id: ActionOpportunityId,
        build: impl FnOnce(
            &ActionOpportunity,
        ) -> Result<ActionOpportunity, ActionOpportunityTransitionError>,
    ) -> Result<&ActionOpportunity, ActionOpportunityLedgerError> {
        let current = self
            .entries
            .get(&id)
            .ok_or(ActionOpportunityLedgerError::UnknownOpportunity { opportunity: id })?;
        let successor =
            build(current).map_err(|error| ActionOpportunityLedgerError::TransitionRejected {
                opportunity: id,
                error,
            })?;
        self.entries.insert(id, successor);
        Ok(self
            .entries
            .get(&id)
            .unwrap_or_else(|| unreachable!("transitioned opportunity must remain indexed")))
    }
}

/// Latest derived containment appraisal retained per actor and subject.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ContainmentAppraisalLedger {
    entries: BTreeMap<(ActorId, world_core::EntityId), ContainmentAppraisal>,
    by_material: BTreeMap<(ActorId, ContainmentAppraisalFingerprint), ContainmentAppraisal>,
}

impl ContainmentAppraisalLedger {
    #[must_use]
    pub(crate) fn get(
        &self,
        actor: ActorId,
        item: world_core::EntityId,
    ) -> Option<ContainmentAppraisal> {
        self.entries.get(&(actor, item)).copied()
    }

    #[must_use]
    pub(crate) fn find_material(
        &self,
        actor: ActorId,
        material: ContainmentAppraisalFingerprint,
    ) -> Option<ContainmentAppraisal> {
        self.by_material.get(&(actor, material)).copied()
    }

    pub(crate) fn retain(&mut self, appraisal: ContainmentAppraisal) {
        self.entries
            .insert((appraisal.actor(), appraisal.item()), appraisal);
        self.by_material.insert(
            (appraisal.actor(), appraisal.material_fingerprint()),
            appraisal,
        );
    }

    pub(crate) fn retract_exact(&mut self, expected: ContainmentAppraisal) -> bool {
        let key = (expected.actor(), expected.item());
        if self.entries.get(&key).copied() != Some(expected) {
            return false;
        }
        self.entries.remove(&key);
        self.by_material
            .remove(&(expected.actor(), expected.material_fingerprint()));
        true
    }
}

/// Why actor-local lifecycle scheduling control rejected a transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LifecycleControlError {
    EmptyCauseBatch,
    CauseRoleMismatch {
        role: LifecycleRole,
        cause: LifecycleCause,
    },
    GenerationOverflow {
        actor: ActorId,
        role: LifecycleRole,
    },
    UnknownControl {
        actor: ActorId,
        role: LifecycleRole,
    },
    EnqueuedGenerationMismatch {
        actor: ActorId,
        role: LifecycleRole,
        expected: Option<LifecycleGeneration>,
        supplied: LifecycleGeneration,
    },
}

/// Result of accepting one canonical batch of lifecycle wake causes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LifecycleWakeRequestOutcome {
    Duplicate {
        desired: LifecycleGeneration,
        enqueued: Option<LifecycleGeneration>,
    },
    Enqueue {
        generation: LifecycleGeneration,
    },
    Coalesced {
        enqueued: LifecycleGeneration,
        desired: LifecycleGeneration,
    },
}

/// Result of sealing one successfully processed lifecycle generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LifecycleCompletionOutcome {
    processed: LifecycleGeneration,
    successor: Option<LifecycleGeneration>,
}

impl LifecycleCompletionOutcome {
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn processed(self) -> LifecycleGeneration {
        self.processed
    }

    #[must_use]
    pub(crate) const fn successor(self) -> Option<LifecycleGeneration> {
        self.successor
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct LifecycleControlKey {
    actor: ActorId,
    role: LifecycleRole,
}

impl LifecycleControlKey {
    const fn new(actor: ActorId, role: LifecycleRole) -> Self {
        Self { actor, role }
    }
}

/// Generation cursor and retained causes for one actor and lifecycle role.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LifecycleControlRecord {
    desired: LifecycleGeneration,
    processed: LifecycleGeneration,
    enqueued: Option<LifecycleGeneration>,
    causes: BTreeMap<LifecycleGeneration, BTreeSet<LifecycleCause>>,
}

impl LifecycleControlRecord {
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn desired(&self) -> LifecycleGeneration {
        self.desired
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) const fn processed(&self) -> LifecycleGeneration {
        self.processed
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) const fn enqueued(&self) -> Option<LifecycleGeneration> {
        self.enqueued
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn causes(
        &self,
        generation: LifecycleGeneration,
    ) -> Option<&BTreeSet<LifecycleCause>> {
        self.causes.get(&generation)
    }

    #[must_use]
    pub(crate) fn enqueued_causes(
        &self,
        generation: LifecycleGeneration,
    ) -> Option<Vec<LifecycleCause>> {
        if self.enqueued != Some(generation) {
            return None;
        }
        Some(
            self.causes
                .range((Excluded(self.processed), Included(generation)))
                .flat_map(|(_, causes)| causes.iter().copied())
                .collect(),
        )
    }

    fn contains_cause(&self, cause: LifecycleCause) -> bool {
        self.causes
            .values()
            .any(|retained| retained.contains(&cause))
    }
}

/// Actor-local scheduling state for the concrete coalescing lifecycle roles.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LifecycleControlLedger {
    entries: BTreeMap<LifecycleControlKey, LifecycleControlRecord>,
}

impl LifecycleControlLedger {
    #[must_use]
    pub(crate) fn get(
        &self,
        actor: ActorId,
        role: LifecycleRole,
    ) -> Option<&LifecycleControlRecord> {
        self.entries.get(&LifecycleControlKey::new(actor, role))
    }

    pub(crate) fn request(
        &mut self,
        actor: ActorId,
        role: LifecycleRole,
        causes: &[LifecycleCause],
    ) -> Result<LifecycleWakeRequestOutcome, LifecycleControlError> {
        if causes.is_empty() {
            return Err(LifecycleControlError::EmptyCauseBatch);
        }
        let canonical = causes.iter().copied().collect::<BTreeSet<_>>();
        if let Some(cause) = canonical
            .iter()
            .copied()
            .find(|cause| !role.accepts(*cause))
        {
            return Err(LifecycleControlError::CauseRoleMismatch { role, cause });
        }

        let key = LifecycleControlKey::new(actor, role);
        let record = self.entries.entry(key).or_default();
        let new_causes = canonical
            .into_iter()
            .filter(|cause| !record.contains_cause(*cause))
            .collect::<BTreeSet<_>>();
        if new_causes.is_empty() {
            return Ok(LifecycleWakeRequestOutcome::Duplicate {
                desired: record.desired,
                enqueued: record.enqueued,
            });
        }

        let desired = record
            .desired
            .checked_next()
            .ok_or(LifecycleControlError::GenerationOverflow { actor, role })?;
        record.desired = desired;
        if record.causes.insert(desired, new_causes).is_some() {
            unreachable!("a strictly advancing generation cannot already retain causes");
        }

        match record.enqueued {
            Some(enqueued) => Ok(LifecycleWakeRequestOutcome::Coalesced { enqueued, desired }),
            None => {
                record.enqueued = Some(desired);
                Ok(LifecycleWakeRequestOutcome::Enqueue {
                    generation: desired,
                })
            }
        }
    }

    pub(crate) fn complete(
        &mut self,
        actor: ActorId,
        role: LifecycleRole,
        generation: LifecycleGeneration,
    ) -> Result<LifecycleCompletionOutcome, LifecycleControlError> {
        let key = LifecycleControlKey::new(actor, role);
        let record = self
            .entries
            .get_mut(&key)
            .ok_or(LifecycleControlError::UnknownControl { actor, role })?;
        if record.enqueued != Some(generation) {
            return Err(LifecycleControlError::EnqueuedGenerationMismatch {
                actor,
                role,
                expected: record.enqueued,
                supplied: generation,
            });
        }

        record.processed = generation;
        let successor = (record.desired > generation).then_some(record.desired);
        record.enqueued = successor;
        Ok(LifecycleCompletionOutcome {
            processed: generation,
            successor,
        })
    }
}

fn retirement_start(
    retired_through: Option<u64>,
    requested: u64,
) -> Result<u64, LedgerRetirementError> {
    match retired_through {
        None => Ok(0),
        Some(current) if requested > current => Ok(current + 1),
        Some(current) => Err(LedgerRetirementError::NotAdvancing {
            retired_through: current,
            requested,
        }),
    }
}

/// Authoritative retained request state installed by published records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RuntimeControlState {
    input: InputRequestLedger,
    management: ManagementRequestLedger,
    command: CommandRequestLedger,
    action_opportunities: ActionOpportunityLedger,
    action_evaluations: ActionEvaluationInvocationLedger,
    action_evaluation_captures: ActionEvaluationCaptureLedger,
    lifecycle: LifecycleControlLedger,
    appraisals: ContainmentAppraisalLedger,
    relocation_processes: RelocationProcessLedger,
}

impl RuntimeControlState {
    #[must_use]
    pub(crate) fn from_root(opportunities: &[ActionOpportunity]) -> Self {
        Self {
            action_opportunities: ActionOpportunityLedger::from_root(opportunities),
            ..Self::default()
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.input.retired_through.is_none()
            && self.input.entries.is_empty()
            && self.management.retired_through.is_none()
            && self.management.entries.is_empty()
            && self.command.retired_through.is_empty()
            && self.command.entries.is_empty()
            && self.action_opportunities.entries.is_empty()
            && self.action_evaluations.is_empty()
            && self.action_evaluation_captures.is_empty()
            && self.lifecycle.entries.is_empty()
            && self.appraisals.entries.is_empty()
            && self.appraisals.by_material.is_empty()
            && self.relocation_processes.is_empty()
    }

    #[must_use]
    pub(crate) const fn input(&self) -> &InputRequestLedger {
        &self.input
    }

    #[must_use]
    pub(crate) const fn management(&self) -> &ManagementRequestLedger {
        &self.management
    }

    #[must_use]
    pub(crate) const fn command(&self) -> &CommandRequestLedger {
        &self.command
    }

    #[must_use]
    pub(crate) const fn action_opportunities(&self) -> &ActionOpportunityLedger {
        &self.action_opportunities
    }

    #[must_use]
    pub(crate) const fn action_evaluations(&self) -> &ActionEvaluationInvocationLedger {
        &self.action_evaluations
    }

    #[must_use]
    pub(crate) const fn action_evaluation_captures(&self) -> &ActionEvaluationCaptureLedger {
        &self.action_evaluation_captures
    }

    pub(crate) const fn action_evaluations_mut(&mut self) -> &mut ActionEvaluationInvocationLedger {
        &mut self.action_evaluations
    }

    #[must_use]
    pub(crate) const fn action_evaluation_captures_mut(
        &mut self,
    ) -> &mut ActionEvaluationCaptureLedger {
        &mut self.action_evaluation_captures
    }

    #[must_use]
    pub(crate) const fn lifecycle(&self) -> &LifecycleControlLedger {
        &self.lifecycle
    }

    #[must_use]
    pub(crate) const fn appraisals(&self) -> &ContainmentAppraisalLedger {
        &self.appraisals
    }

    #[must_use]
    pub(crate) const fn relocation_processes(&self) -> &RelocationProcessLedger {
        &self.relocation_processes
    }

    #[must_use]
    pub(crate) fn input_mut(&mut self) -> &mut InputRequestLedger {
        &mut self.input
    }

    #[must_use]
    pub(crate) fn management_mut(&mut self) -> &mut ManagementRequestLedger {
        &mut self.management
    }

    #[must_use]
    pub(crate) fn command_mut(&mut self) -> &mut CommandRequestLedger {
        &mut self.command
    }

    #[must_use]
    pub(crate) fn lifecycle_mut(&mut self) -> &mut LifecycleControlLedger {
        &mut self.lifecycle
    }

    #[must_use]
    pub(crate) fn appraisals_mut(&mut self) -> &mut ContainmentAppraisalLedger {
        &mut self.appraisals
    }

    #[must_use]
    pub(crate) fn relocation_processes_mut(&mut self) -> &mut RelocationProcessLedger {
        &mut self.relocation_processes
    }

    pub(crate) fn consume_action_opportunity(
        &mut self,
        id: ActionOpportunityId,
        expected_version: ActionOpportunityVersion,
        disposition: ActionOpportunityDisposition,
    ) -> Result<&ActionOpportunity, ActionOpportunityLedgerError> {
        self.action_opportunities
            .consume(id, expected_version, disposition)
    }

    pub(crate) fn open_action_opportunity(
        &mut self,
        opportunity: ActionOpportunity,
    ) -> Result<&ActionOpportunity, ActionOpportunityLedgerError> {
        self.action_opportunities.open(opportunity)
    }

    pub(crate) fn begin_action_evaluation(
        &mut self,
        id: ActionOpportunityId,
        expected_version: ActionOpportunityVersion,
        policy_semantics: [u8; 32],
        action_input_fingerprint: [u8; 32],
    ) -> Result<(&ActionOpportunity, ActionEvaluationInvocationId), ActionOpportunityLedgerError>
    {
        self.action_opportunities.begin_evaluation(
            id,
            expected_version,
            policy_semantics,
            action_input_fingerprint,
        )
    }

    pub(crate) fn resume_action_evaluation(
        &mut self,
        id: ActionOpportunityId,
        expected_version: ActionOpportunityVersion,
        invocation: ActionEvaluationInvocationId,
    ) -> Result<&ActionOpportunity, ActionOpportunityLedgerError> {
        self.action_opportunities
            .resume_evaluation(id, expected_version, invocation)
    }

    pub(crate) fn reopen_action_evaluation(
        &mut self,
        id: ActionOpportunityId,
        expected_version: ActionOpportunityVersion,
        invocation: ActionEvaluationInvocationId,
    ) -> Result<&ActionOpportunity, ActionOpportunityLedgerError> {
        self.action_opportunities
            .reopen_for_visible_reinvocation(id, expected_version, invocation)
    }

    pub(crate) fn can_retire(
        &self,
        retirement: LedgerRetirement,
        current_management_request: ManagementRequestId,
    ) -> Result<(), LedgerRetirementError> {
        match retirement {
            LedgerRetirement::InputThrough(target) => self.input.can_retire_through(target),
            LedgerRetirement::ManagementThrough(target) if target >= current_management_request => {
                Err(LedgerRetirementError::ManagementTargetNotBeforeRequest {
                    target,
                    request: current_management_request,
                })
            }
            LedgerRetirement::ManagementThrough(target) => {
                self.management.can_retire_through(target)
            }
            LedgerRetirement::CommandThrough { source, command } => {
                self.command.can_retire_through(source, command)
            }
        }
    }

    pub(crate) fn retire(
        &mut self,
        retirement: LedgerRetirement,
        current_management_request: ManagementRequestId,
    ) -> Result<(), LedgerRetirementError> {
        self.can_retire(retirement, current_management_request)?;
        match retirement {
            LedgerRetirement::InputThrough(target) => self.input.retire_through(target),
            LedgerRetirement::ManagementThrough(target) => self.management.retire_through(target),
            LedgerRetirement::CommandThrough { source, command } => {
                self.command.retire_through(source, command)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use world_core::{ActorId, EntityId, Microstep, SimMoment, SimTime};
    use world_model::{
        ActionOpportunityGeneration, ActionOpportunityState, ActionSponsor, ActorReactionCause,
        CommandAttemptOutcome, ContainmentInteractionScope, StableCommandRejection,
    };

    use crate::authority::{
        AttemptLocalIndex, AttemptRecordId, AuthorityRecordId, CapturedInputRecordId,
    };
    use crate::kernel::fixtures;
    use crate::kernel::{
        AdmitRequest, InputId, LedgerRetirement, ManageRequest, ManagementRequestId,
        SessionManagement,
    };
    use crate::scheduler::CommandTriggerId;

    use super::*;

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(ticks), Microstep::new(microstep))
    }

    fn captured(byte: u8) -> CapturedInputRecordId {
        CapturedInputRecordId::from_bytes([byte; 32])
    }

    fn trigger(byte: u8) -> CommandTriggerId {
        CommandTriggerId::from_bytes([byte; 32])
    }

    fn attempt_id(owner_byte: u8, index: u32) -> AttemptRecordId {
        AttemptRecordId::derive(
            AuthorityRecordId::from_bytes([owner_byte; 32]),
            AttemptLocalIndex::new(index),
        )
    }

    fn action_opportunity(actor_byte: u8, generation: u64) -> ActionOpportunity {
        let scope = ContainmentInteractionScope::new(
            EntityId::from_bytes([0x21; 32]),
            vec![EntityId::from_bytes([0x22; 32])],
            vec![EntityId::from_bytes([0x23; 32])],
            8,
        )
        .unwrap_or_else(|error| panic!("ledger opportunity scope must be valid: {error}"));
        ActionOpportunity::open(
            ActorId::from_bytes([actor_byte; 32]),
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x31; 32])),
            world_model::ActionInteractionScope::containment(scope),
            ActionOpportunityGeneration::new(generation),
        )
    }

    #[test]
    fn action_opportunity_ledger_uses_exact_versioned_one_shot_consumption() {
        let opportunity = action_opportunity(0x11, 0);
        let id = opportunity.id();
        let mut state = RuntimeControlState::from_root(core::slice::from_ref(&opportunity));

        assert_eq!(state.action_opportunities().get(id), Some(&opportunity));
        assert_eq!(
            state
                .consume_action_opportunity(
                    id,
                    ActionOpportunityVersion::new(2)
                        .unwrap_or_else(|| panic!("version two must be valid")),
                    ActionOpportunityDisposition::ActionSubmitted,
                )
                .map(|value| value.state()),
            Err(ActionOpportunityLedgerError::TransitionRejected {
                opportunity: id,
                error: ActionOpportunityTransitionError::StaleVersion {
                    expected: ActionOpportunityVersion::new(2)
                        .unwrap_or_else(|| panic!("version two must be valid")),
                    actual: ActionOpportunityVersion::INITIAL,
                },
            })
        );
        assert_eq!(
            state
                .action_opportunities()
                .get(id)
                .map(ActionOpportunity::state),
            Some(ActionOpportunityState::Open)
        );

        let consumed = state
            .consume_action_opportunity(
                id,
                ActionOpportunityVersion::INITIAL,
                ActionOpportunityDisposition::ActionSubmitted,
            )
            .unwrap_or_else(|error| panic!("open opportunity must consume once: {error:?}"));
        assert_eq!(
            consumed.state(),
            ActionOpportunityState::Consumed(ActionOpportunityDisposition::ActionSubmitted)
        );
        let consumed_version = consumed.version();
        assert_eq!(consumed_version.get(), 2);
        assert_eq!(
            state
                .consume_action_opportunity(
                    id,
                    consumed_version,
                    ActionOpportunityDisposition::NoApplicableAction,
                )
                .map(|value| value.state()),
            Err(ActionOpportunityLedgerError::TransitionRejected {
                opportunity: id,
                error: ActionOpportunityTransitionError::AlreadyConsumed {
                    disposition: ActionOpportunityDisposition::ActionSubmitted,
                },
            })
        );
        let unknown = ActionOpportunityId::from_bytes([0x99; 32]);
        assert_eq!(
            state
                .consume_action_opportunity(
                    unknown,
                    ActionOpportunityVersion::INITIAL,
                    ActionOpportunityDisposition::Failed,
                )
                .map(|value| value.state()),
            Err(ActionOpportunityLedgerError::UnknownOpportunity {
                opportunity: unknown,
            })
        );
    }

    #[test]
    fn waiting_opportunity_remains_the_actors_unique_active_opportunity() {
        let first = action_opportunity(0x12, 0);
        let second = action_opportunity(0x12, 1);
        let first_id = first.id();
        let second_id = second.id();
        let actor = first.actor();
        let mut state = RuntimeControlState::from_root(core::slice::from_ref(&first));

        let (waiting_version, invocation) = {
            let (waiting, invocation) = state
                .begin_action_evaluation(first_id, first.version(), [0x41; 32], [0x42; 32])
                .unwrap_or_else(|error| panic!("open opportunity must begin waiting: {error:?}"));
            (waiting.version(), invocation)
        };
        assert!(matches!(
            state.open_action_opportunity(second.clone()),
            Err(ActionOpportunityLedgerError::ActorAlreadyHasActiveOpportunity {
                actor: actual,
                existing,
            }) if actual == actor && existing == first_id
        ));

        let reopened_version = state
            .resume_action_evaluation(first_id, waiting_version, invocation)
            .unwrap_or_else(|error| panic!("waiting opportunity must resume: {error:?}"))
            .version();
        state
            .consume_action_opportunity(
                first_id,
                reopened_version,
                ActionOpportunityDisposition::Failed,
            )
            .unwrap_or_else(|error| panic!("resumed opportunity must consume: {error:?}"));
        assert_eq!(
            state
                .open_action_opportunity(second)
                .unwrap_or_else(|error| {
                    panic!("a consumed predecessor must release actor uniqueness: {error:?}")
                })
                .id(),
            second_id
        );
    }

    #[test]
    fn ledgers_are_separate_and_preserve_first_publication() {
        let input_request =
            AdmitRequest::new(InputId::new(0), moment(2, 3), fixtures::command(0x11, 7));
        let management_request =
            ManageRequest::new(ManagementRequestId::new(0), SessionManagement::Pause);
        let input_record = AuthorityRecordId::from_bytes([0x21; 32]);
        let management_record = AuthorityRecordId::from_bytes([0x22; 32]);
        let input_outcome = AdmitOutcome::scheduled(input_record, input_request.effective());
        let management_outcome =
            ManageOutcome::applied(management_record, management_request.operation());
        let captured_id = captured(0x24);
        let trigger_id = trigger(0x25);
        let mut state = RuntimeControlState::default();

        assert_eq!(
            state.input_mut().insert_exact(
                input_request.id(),
                input_request.fingerprint(),
                captured_id,
                trigger_id,
                input_outcome,
            ),
            Ok(())
        );
        assert_eq!(
            state.management_mut().insert_exact(
                management_request.id(),
                management_request.fingerprint(),
                management_outcome,
            ),
            Ok(())
        );
        assert_eq!(
            state.input_mut().insert_exact(
                input_request.id(),
                input_request.fingerprint(),
                captured(0x26),
                trigger(0x27),
                AdmitOutcome::scheduled(
                    AuthorityRecordId::from_bytes([0x23; 32]),
                    input_request.effective(),
                ),
            ),
            Err(InputLedgerInsertError {
                id: input_request.id(),
            })
        );
        let retained_input = state
            .input()
            .get(input_request.id())
            .unwrap_or_else(|| panic!("published input must be retained"));
        assert_eq!(retained_input.captured(), captured_id);
        assert_eq!(retained_input.trigger(), trigger_id);
        assert_eq!(retained_input.outcome(), input_outcome);
        assert_eq!(
            state
                .management()
                .get(management_request.id())
                .unwrap_or_else(|| panic!("published management request must be retained"))
                .outcome(),
            management_outcome
        );
    }

    #[test]
    fn command_ledger_is_source_scoped_and_never_overwrites() {
        let first = fixtures::command(0x31, 4);
        let same_id_other_source = fixtures::command(0x32, 4);
        let owner = AuthorityRecordId::from_bytes([0x41; 32]);
        let attempt = AttemptRecordId::derive(owner, AttemptLocalIndex::new(0));
        let outcome =
            CommandAttemptOutcome::Rejected(StableCommandRejection::RequirementUnsatisfied);
        let mut ledger = CommandRequestLedger::default();

        assert_eq!(
            ledger.insert_exact(
                first.source(),
                first.id(),
                first.fingerprint(),
                attempt,
                outcome,
            ),
            Ok(())
        );
        assert_eq!(
            ledger.insert_exact(
                same_id_other_source.source(),
                same_id_other_source.id(),
                same_id_other_source.fingerprint(),
                attempt,
                outcome,
            ),
            Ok(())
        );
        assert_eq!(
            ledger.insert_exact(
                first.source(),
                first.id(),
                first.fingerprint(),
                attempt,
                CommandAttemptOutcome::Accepted,
            ),
            Err(CommandLedgerInsertError::NotAbsent {
                source: first.source(),
                id: first.id(),
            })
        );

        let retained = ledger
            .get(first.source(), first.id())
            .unwrap_or_else(|| panic!("first command result must be retained"));
        assert_eq!(retained.exact_fingerprint(), Some(first.fingerprint()));
        assert_eq!(retained.original_attempt(), attempt);
        assert_eq!(retained.outcome(), outcome);
    }

    #[test]
    fn ledgers_classify_absent_exact_and_mismatched_reuse() {
        let input_request =
            AdmitRequest::new(InputId::new(3), moment(4, 0), fixtures::command(0x41, 5));
        let changed_input = AdmitRequest::new(
            InputId::new(3),
            moment(4, 1),
            input_request.command().clone(),
        );
        let input_record = AuthorityRecordId::from_bytes([0x42; 32]);
        let input_outcome = AdmitOutcome::scheduled(input_record, input_request.effective());
        let mut input = InputRequestLedger::default();

        assert_eq!(
            input.classify(input_request.id(), input_request.fingerprint()),
            RequestLedgerLookup::Absent
        );
        input
            .insert_exact(
                input_request.id(),
                input_request.fingerprint(),
                captured(0x43),
                trigger(0x44),
                input_outcome,
            )
            .unwrap_or_else(|error| panic!("fixture input ID must be absent: {error:?}"));
        assert_eq!(
            input.classify(input_request.id(), input_request.fingerprint()),
            RequestLedgerLookup::RetainedExact(input_outcome)
        );
        assert_eq!(
            input.classify(changed_input.id(), changed_input.fingerprint()),
            RequestLedgerLookup::IdReuseMismatch
        );

        let pause = ManageRequest::new(ManagementRequestId::new(6), SessionManagement::Pause);
        let resume = ManageRequest::new(ManagementRequestId::new(6), SessionManagement::Resume);
        let management_record = AuthorityRecordId::from_bytes([0x45; 32]);
        let management_outcome = ManageOutcome::applied(management_record, pause.operation());
        let mut management = ManagementRequestLedger::default();
        management
            .insert_exact(pause.id(), pause.fingerprint(), management_outcome)
            .unwrap_or_else(|error| panic!("fixture management ID must be absent: {error:?}"));
        assert_eq!(
            management.classify(pause.id(), pause.fingerprint()),
            RequestLedgerLookup::RetainedExact(management_outcome)
        );
        assert_eq!(
            management.classify(resume.id(), resume.fingerprint()),
            RequestLedgerLookup::IdReuseMismatch
        );

        let command = fixtures::command(0x46, 7);
        let changed_fingerprint = fixtures::command(0x47, 7).fingerprint();
        let attempt = AttemptRecordId::derive(
            AuthorityRecordId::from_bytes([0x48; 32]),
            AttemptLocalIndex::new(0),
        );
        let command_outcome =
            CommandAttemptOutcome::Rejected(StableCommandRejection::RequirementUnsatisfied);
        let mut commands = CommandRequestLedger::default();
        commands
            .insert_exact(
                command.source(),
                command.id(),
                command.fingerprint(),
                attempt,
                command_outcome,
            )
            .unwrap_or_else(|error| panic!("fixture command key must be absent: {error:?}"));
        assert_eq!(
            commands.classify(command.source(), command.id(), command.fingerprint()),
            CommandLedgerLookup::RetainedExact {
                original_attempt: attempt,
                outcome: command_outcome,
            }
        );
        assert_eq!(
            commands.classify(command.source(), command.id(), changed_fingerprint),
            CommandLedgerLookup::IdReuseMismatch {
                original_attempt: attempt,
            }
        );
    }

    #[test]
    fn command_collision_is_canonical_and_permanent() {
        let first = fixtures::command_with_actor(0x51, 8, 0x61);
        let second = fixtures::command_with_actor(0x51, 8, 0x62);
        let third = fixtures::command_with_actor(0x51, 8, 0x63);
        let unrelated = fixtures::command_with_actor(0x51, 8, 0x64);
        let attempt = attempt_id(0x71, 0);
        let mut ledger = CommandRequestLedger::default();

        ledger
            .insert_collision(
                first.source(),
                first.id(),
                &[
                    third.fingerprint(),
                    first.fingerprint(),
                    second.fingerprint(),
                    first.fingerprint(),
                ],
                attempt,
            )
            .unwrap_or_else(|error| panic!("distinct collision must be retained: {error:?}"));

        let mut expected = vec![
            first.fingerprint(),
            second.fingerprint(),
            third.fingerprint(),
        ];
        expected.sort_unstable();
        let retained = ledger
            .get(first.source(), first.id())
            .unwrap_or_else(|| panic!("collision must be retained"));
        assert_eq!(retained.original_attempt(), attempt);
        assert_eq!(
            retained.outcome(),
            CommandAttemptOutcome::Rejected(StableCommandRejection::IdCollision)
        );
        assert_eq!(
            retained
                .collision_fingerprints()
                .unwrap_or_else(|| panic!("entry must be a collision")),
            expected
        );

        let collision = CommandLedgerLookup::RetainedCollision {
            original_attempt: attempt,
        };
        for fingerprint in [
            first.fingerprint(),
            second.fingerprint(),
            unrelated.fingerprint(),
        ] {
            assert_eq!(
                ledger.classify(first.source(), first.id(), fingerprint),
                collision
            );
        }
        assert_eq!(
            ledger.insert_exact(
                first.source(),
                first.id(),
                first.fingerprint(),
                attempt_id(0x72, 0),
                CommandAttemptOutcome::Accepted,
            ),
            Err(CommandLedgerInsertError::NotAbsent {
                source: first.source(),
                id: first.id(),
            })
        );
        assert_eq!(
            ledger.insert_collision(
                first.source(),
                first.id(),
                &[first.fingerprint(), unrelated.fingerprint()],
                attempt_id(0x73, 0),
            ),
            Err(CommandLedgerInsertError::NotAbsent {
                source: first.source(),
                id: first.id(),
            })
        );
        assert_eq!(
            ledger.classify(first.source(), first.id(), unrelated.fingerprint()),
            collision
        );
    }

    #[test]
    fn collision_insertion_requires_two_distinct_fingerprints() {
        let command = fixtures::command(0x52, 9);
        let attempt = attempt_id(0x74, 0);
        let mut ledger = CommandRequestLedger::default();

        for (fingerprints, distinct_count) in [
            (Vec::new(), 0),
            (vec![command.fingerprint(), command.fingerprint()], 1),
        ] {
            assert_eq!(
                ledger.insert_collision(command.source(), command.id(), &fingerprints, attempt),
                Err(
                    CommandLedgerInsertError::CollisionRequiresDistinctFingerprints {
                        source: command.source(),
                        id: command.id(),
                        distinct_count,
                    }
                )
            );
        }
        assert_eq!(
            ledger.classify(command.source(), command.id(), command.fingerprint()),
            CommandLedgerLookup::Absent
        );
        assert_eq!(
            ledger.insert_exact(
                command.source(),
                command.id(),
                command.fingerprint(),
                attempt,
                CommandAttemptOutcome::Rejected(StableCommandRejection::IdCollision),
            ),
            Err(CommandLedgerInsertError::ExactCannotRepresentCollision {
                source: command.source(),
                id: command.id(),
            })
        );
        assert_eq!(
            ledger.classify(command.source(), command.id(), command.fingerprint()),
            CommandLedgerLookup::Absent
        );
    }

    #[test]
    fn retained_exact_never_becomes_a_collision() {
        let exact = fixtures::command_with_actor(0x53, 10, 0x65);
        let changed = fixtures::command_with_actor(0x53, 10, 0x66);
        let original_attempt = attempt_id(0x75, 0);
        let outcome =
            CommandAttemptOutcome::Rejected(StableCommandRejection::RequirementUnsatisfied);
        let mut ledger = CommandRequestLedger::default();
        ledger
            .insert_exact(
                exact.source(),
                exact.id(),
                exact.fingerprint(),
                original_attempt,
                outcome,
            )
            .unwrap_or_else(|error| panic!("exact entry must be new: {error:?}"));

        assert_eq!(
            ledger.insert_collision(
                exact.source(),
                exact.id(),
                &[exact.fingerprint(), changed.fingerprint()],
                attempt_id(0x76, 0),
            ),
            Err(CommandLedgerInsertError::NotAbsent {
                source: exact.source(),
                id: exact.id(),
            })
        );
        assert_eq!(
            ledger.classify(exact.source(), exact.id(), exact.fingerprint()),
            CommandLedgerLookup::RetainedExact {
                original_attempt,
                outcome,
            }
        );
        assert_eq!(
            ledger.classify(exact.source(), exact.id(), changed.fingerprint()),
            CommandLedgerLookup::IdReuseMismatch { original_attempt }
        );
        let retained = ledger
            .get(exact.source(), exact.id())
            .unwrap_or_else(|| panic!("exact entry must remain"));
        assert_eq!(retained.exact_fingerprint(), Some(exact.fingerprint()));
        assert_eq!(retained.collision_fingerprints(), None);
    }

    #[test]
    fn singular_ledgers_retire_only_complete_terminal_prefixes() {
        let command = fixtures::command(0x54, 11);
        let input_fingerprint =
            AdmitRequest::new(InputId::new(0), moment(2, 0), command.clone()).fingerprint();
        let input_outcome =
            AdmitOutcome::scheduled(AuthorityRecordId::from_bytes([0x77; 32]), moment(2, 0));
        let mut input = InputRequestLedger::default();
        for sequence in [0, 2] {
            input
                .insert_exact(
                    InputId::new(sequence),
                    input_fingerprint,
                    captured(0x78),
                    trigger(0x79),
                    input_outcome,
                )
                .unwrap_or_else(|error| panic!("input sequence must be new: {error:?}"));
        }
        assert_eq!(
            input.retire_through(InputId::new(2)),
            Err(LedgerRetirementError::Gap { missing: 1 })
        );
        assert_eq!(input.retired_through(), None);
        assert!(input.get(InputId::new(0)).is_some());
        assert!(input.get(InputId::new(2)).is_some());

        input
            .insert_exact(
                InputId::new(1),
                input_fingerprint,
                captured(0x7a),
                trigger(0x7b),
                input_outcome,
            )
            .unwrap_or_else(|error| panic!("missing input sequence must be new: {error:?}"));
        input
            .retire_through(InputId::new(1))
            .unwrap_or_else(|error| panic!("complete input prefix must retire: {error:?}"));
        assert_eq!(input.retired_through(), Some(InputId::new(1)));
        for sequence in [0, 1] {
            let id = InputId::new(sequence);
            assert_eq!(
                input.classify(id, input_fingerprint),
                RequestLedgerLookup::Retired
            );
            assert_eq!(input.get(id), None);
            assert_eq!(
                input.insert_exact(
                    id,
                    input_fingerprint,
                    captured(0x7c),
                    trigger(0x7d),
                    input_outcome,
                ),
                Err(InputLedgerInsertError { id })
            );
        }
        assert_eq!(
            input.classify(InputId::new(2), input_fingerprint),
            RequestLedgerLookup::RetainedExact(input_outcome)
        );
        input
            .retire_through(InputId::new(2))
            .unwrap_or_else(|error| panic!("next retained sequence must retire: {error:?}"));
        assert_eq!(input.retired_through(), Some(InputId::new(2)));
        assert_eq!(
            input.classify(InputId::new(2), input_fingerprint),
            RequestLedgerLookup::Retired
        );
        assert_eq!(
            input.retire_through(InputId::new(1)),
            Err(LedgerRetirementError::NotAdvancing {
                retired_through: 2,
                requested: 1,
            })
        );

        let pause = ManageRequest::new(ManagementRequestId::new(0), SessionManagement::Pause);
        let retire = ManageRequest::new(
            ManagementRequestId::new(1),
            SessionManagement::Retire(LedgerRetirement::ManagementThrough(
                ManagementRequestId::new(0),
            )),
        );
        let mut management = ManagementRequestLedger::default();
        for request in [pause, retire] {
            management
                .insert_exact(
                    request.id(),
                    request.fingerprint(),
                    ManageOutcome::applied(
                        AuthorityRecordId::from_bytes([0x7e; 32]),
                        request.operation(),
                    ),
                )
                .unwrap_or_else(|error| panic!("management sequence must be new: {error:?}"));
        }
        management
            .retire_through(ManagementRequestId::new(0))
            .unwrap_or_else(|error| panic!("complete management prefix must retire: {error:?}"));
        assert_eq!(
            management.retired_through(),
            Some(ManagementRequestId::new(0))
        );
        assert_eq!(
            management.classify(pause.id(), pause.fingerprint()),
            RequestLedgerLookup::Retired
        );
        assert_eq!(
            management.classify(retire.id(), retire.fingerprint()),
            RequestLedgerLookup::RetainedExact(ManageOutcome::applied(
                AuthorityRecordId::from_bytes([0x7e; 32]),
                retire.operation(),
            ))
        );

        let next = ManageRequest::new(
            ManagementRequestId::new(2),
            SessionManagement::Retire(LedgerRetirement::ManagementThrough(
                ManagementRequestId::new(1),
            )),
        );
        management
            .insert_exact(
                next.id(),
                next.fingerprint(),
                ManageOutcome::applied(AuthorityRecordId::from_bytes([0x7f; 32]), next.operation()),
            )
            .unwrap_or_else(|error| panic!("next management request must be new: {error:?}"));
        management
            .retire_through(ManagementRequestId::new(1))
            .unwrap_or_else(|error| panic!("next complete prefix must retire: {error:?}"));
        assert_eq!(
            management.classify(retire.id(), retire.fingerprint()),
            RequestLedgerLookup::Retired
        );
        assert_eq!(
            management.classify(next.id(), next.fingerprint()),
            RequestLedgerLookup::RetainedExact(ManageOutcome::applied(
                AuthorityRecordId::from_bytes([0x7f; 32]),
                next.operation(),
            ))
        );
    }

    #[test]
    fn management_retirement_target_must_precede_the_carrying_request() {
        let prior = ManageRequest::new(ManagementRequestId::new(0), SessionManagement::Pause);
        let current = ManageRequest::new(
            ManagementRequestId::new(1),
            SessionManagement::Retire(LedgerRetirement::ManagementThrough(
                ManagementRequestId::new(0),
            )),
        );
        let record = AuthorityRecordId::from_bytes([0x80; 32]);
        let mut state = RuntimeControlState::default();
        state
            .management_mut()
            .insert_exact(
                prior.id(),
                prior.fingerprint(),
                ManageOutcome::applied(record, prior.operation()),
            )
            .unwrap_or_else(|error| panic!("prior management request must be new: {error:?}"));

        assert_eq!(
            state.can_retire(
                LedgerRetirement::ManagementThrough(ManagementRequestId::new(0)),
                current.id(),
            ),
            Ok(())
        );
        assert_eq!(
            state.retire(
                LedgerRetirement::ManagementThrough(current.id()),
                current.id(),
            ),
            Err(LedgerRetirementError::ManagementTargetNotBeforeRequest {
                target: current.id(),
                request: current.id(),
            })
        );
        assert_eq!(state.management().retired_through(), None);
        assert_eq!(
            state.management().classify(prior.id(), prior.fingerprint()),
            RequestLedgerLookup::RetainedExact(ManageOutcome::applied(record, prior.operation()))
        );
    }

    #[test]
    fn typed_control_retirement_advances_only_the_selected_frontier() {
        let input_request =
            AdmitRequest::new(InputId::new(0), moment(3, 0), fixtures::command(0x80, 8));
        let input_outcome = AdmitOutcome::scheduled(
            AuthorityRecordId::from_bytes([0x81; 32]),
            input_request.effective(),
        );
        let management_request =
            ManageRequest::new(ManagementRequestId::new(0), SessionManagement::Pause);
        let command = fixtures::command(0x82, 0);
        let mut state = RuntimeControlState::default();
        state
            .input_mut()
            .insert_exact(
                input_request.id(),
                input_request.fingerprint(),
                captured(0x83),
                trigger(0x84),
                input_outcome,
            )
            .unwrap_or_else(|error| panic!("input request must be new: {error:?}"));
        state
            .management_mut()
            .insert_exact(
                management_request.id(),
                management_request.fingerprint(),
                ManageOutcome::applied(
                    AuthorityRecordId::from_bytes([0x85; 32]),
                    management_request.operation(),
                ),
            )
            .unwrap_or_else(|error| panic!("management request must be new: {error:?}"));
        state
            .command_mut()
            .insert_exact(
                command.source(),
                command.id(),
                command.fingerprint(),
                attempt_id(0x86, 0),
                CommandAttemptOutcome::Accepted,
            )
            .unwrap_or_else(|error| panic!("command request must be new: {error:?}"));

        let retirement = LedgerRetirement::InputThrough(input_request.id());
        assert_eq!(
            state.can_retire(retirement, management_request.id()),
            Ok(())
        );
        state
            .retire(retirement, management_request.id())
            .unwrap_or_else(|error| panic!("selected input prefix must retire: {error:?}"));

        assert_eq!(state.input().retired_through(), Some(input_request.id()));
        assert_eq!(state.management().retired_through(), None);
        assert_eq!(state.command().retired_through(command.source()), None);
        assert_eq!(
            state
                .management()
                .classify(management_request.id(), management_request.fingerprint()),
            RequestLedgerLookup::RetainedExact(ManageOutcome::applied(
                AuthorityRecordId::from_bytes([0x85; 32]),
                management_request.operation(),
            ))
        );
        assert!(
            state
                .command()
                .get(command.source(), command.id())
                .is_some()
        );
    }

    #[test]
    fn command_retirement_is_contiguous_and_source_scoped() {
        let first_source = CommandSource::from_bytes([0x81; 32]);
        let second_source = CommandSource::from_bytes([0x82; 32]);
        let first_zero = fixtures::command_with_actor(0x81, 0, 0x11);
        let first_two = fixtures::command_with_actor(0x81, 2, 0x12);
        let second_zero = fixtures::command_with_actor(0x82, 0, 0x13);
        let outcome = CommandAttemptOutcome::Accepted;
        let original_attempt = attempt_id(0x83, 0);
        let mut ledger = CommandRequestLedger::default();
        for command in [&first_zero, &first_two, &second_zero] {
            ledger
                .insert_exact(
                    command.source(),
                    command.id(),
                    command.fingerprint(),
                    original_attempt,
                    outcome,
                )
                .unwrap_or_else(|error| panic!("command sequence must be new: {error:?}"));
        }

        assert_eq!(
            ledger.retire_through(first_source, CommandId::new(2)),
            Err(LedgerRetirementError::Gap { missing: 1 })
        );
        assert_eq!(ledger.retired_through(first_source), None);
        assert!(ledger.get(first_source, CommandId::new(0)).is_some());
        assert!(ledger.get(first_source, CommandId::new(2)).is_some());

        let first_one_a = fixtures::command_with_actor(0x81, 1, 0x14);
        let first_one_b = fixtures::command_with_actor(0x81, 1, 0x15);
        ledger
            .insert_collision(
                first_source,
                CommandId::new(1),
                &[first_one_b.fingerprint(), first_one_a.fingerprint()],
                attempt_id(0x84, 0),
            )
            .unwrap_or_else(|error| panic!("collision sequence must be new: {error:?}"));
        ledger
            .retire_through(first_source, CommandId::new(1))
            .unwrap_or_else(|error| panic!("complete command prefix must retire: {error:?}"));
        assert_eq!(
            ledger.retired_through(first_source),
            Some(CommandId::new(1))
        );
        for sequence in [0, 1] {
            assert_eq!(
                ledger.classify(
                    first_source,
                    CommandId::new(sequence),
                    first_zero.fingerprint(),
                ),
                CommandLedgerLookup::Retired
            );
            assert!(ledger.get(first_source, CommandId::new(sequence)).is_none());
        }
        assert_eq!(
            ledger.classify(first_source, first_two.id(), first_two.fingerprint()),
            CommandLedgerLookup::RetainedExact {
                original_attempt,
                outcome,
            }
        );
        assert_eq!(
            ledger.classify(second_source, second_zero.id(), second_zero.fingerprint()),
            CommandLedgerLookup::RetainedExact {
                original_attempt,
                outcome,
            }
        );
        assert_eq!(ledger.retired_through(second_source), None);
    }

    #[test]
    fn every_ledger_iterates_in_protocol_key_order() {
        let mut input = InputRequestLedger::default();
        let mut management = ManagementRequestLedger::default();
        let first_command = fixtures::command(0x51, 9);
        let second_command = fixtures::command(0x50, 10);
        let owner = AuthorityRecordId::from_bytes([0x61; 32]);
        let attempt = AttemptRecordId::derive(owner, AttemptLocalIndex::new(0));
        let input_record = AuthorityRecordId::from_bytes([0x62; 32]);
        let management_record = AuthorityRecordId::from_bytes([0x63; 32]);
        let input_request = AdmitRequest::new(InputId::new(1), moment(1, 0), first_command.clone());
        let management_request =
            ManageRequest::new(ManagementRequestId::new(1), SessionManagement::Resume);

        for id in [InputId::new(8), InputId::new(1)] {
            input
                .insert_exact(
                    id,
                    input_request.fingerprint(),
                    captured(0x64),
                    trigger(0x65),
                    AdmitOutcome::scheduled(input_record, input_request.effective()),
                )
                .unwrap_or_else(|error| panic!("fixture input IDs are unique: {error:?}"));
        }
        for id in [ManagementRequestId::new(7), ManagementRequestId::new(1)] {
            management
                .insert_exact(
                    id,
                    management_request.fingerprint(),
                    ManageOutcome::applied(management_record, SessionManagement::Resume),
                )
                .unwrap_or_else(|error| panic!("fixture management IDs are unique: {error:?}"));
        }
        let mut command = CommandRequestLedger::default();
        for envelope in [&first_command, &second_command] {
            command
                .insert_exact(
                    envelope.source(),
                    envelope.id(),
                    envelope.fingerprint(),
                    attempt,
                    CommandAttemptOutcome::Accepted,
                )
                .unwrap_or_else(|error| panic!("fixture command keys are unique: {error:?}"));
        }

        assert_eq!(
            input.iter().map(|(id, _)| id.get()).collect::<Vec<_>>(),
            vec![1, 8]
        );
        assert_eq!(
            management
                .iter()
                .map(|(id, _)| id.get())
                .collect::<Vec<_>>(),
            vec![1, 7]
        );
        assert_eq!(
            command
                .iter()
                .map(|((source, id), _)| (source, id.get()))
                .collect::<Vec<_>>(),
            vec![(second_command.source(), 10), (first_command.source(), 9)]
        );
    }

    #[test]
    fn duplicate_errors_retain_the_conflicting_key() {
        let input = InputLedgerInsertError {
            id: InputId::new(3),
        };
        let management = ManagementLedgerInsertError {
            id: ManagementRequestId::new(4),
        };
        let command = CommandLedgerInsertError::NotAbsent {
            source: CommandSource::from_bytes([0x71; 32]),
            id: CommandId::new(5),
        };

        assert_eq!(input.id(), InputId::new(3));
        assert_eq!(management.id(), ManagementRequestId::new(4));
        assert_eq!(command.source(), CommandSource::from_bytes([0x71; 32]));
        assert_eq!(command.id(), CommandId::new(5));
    }

    #[test]
    fn lifecycle_wake_batches_are_canonical_and_duplicate_safe() {
        let actor = ActorId::from_bytes([0x81; 32]);
        let first =
            LifecycleCause::Evidence(world_model::EvidenceDeliveryId::from_bytes([0x82; 32]));
        let second =
            LifecycleCause::Evidence(world_model::EvidenceDeliveryId::from_bytes([0x83; 32]));
        let generation = LifecycleGeneration::new(1);
        let mut state = RuntimeControlState::empty();

        assert_eq!(
            state.lifecycle_mut().request(
                actor,
                LifecycleRole::Appraisal,
                &[second, first, second],
            ),
            Ok(LifecycleWakeRequestOutcome::Enqueue { generation })
        );
        let record = state
            .lifecycle()
            .get(actor, LifecycleRole::Appraisal)
            .unwrap_or_else(|| panic!("accepted wake must create actor-role control"));
        assert_eq!(record.desired(), generation);
        assert_eq!(record.processed(), LifecycleGeneration::INITIAL);
        assert_eq!(record.enqueued(), Some(generation));
        assert_eq!(
            record
                .causes(generation)
                .map(|causes| causes.iter().copied().collect::<Vec<_>>()),
            Some(vec![first, second])
        );

        assert_eq!(
            state
                .lifecycle_mut()
                .request(actor, LifecycleRole::Appraisal, &[first, second]),
            Ok(LifecycleWakeRequestOutcome::Duplicate {
                desired: generation,
                enqueued: Some(generation),
            })
        );
        assert_eq!(
            state
                .lifecycle()
                .get(actor, LifecycleRole::Appraisal)
                .map(LifecycleControlRecord::desired),
            Some(generation)
        );
    }

    #[test]
    fn lifecycle_completion_enqueues_one_final_dirty_successor() {
        let actor = ActorId::from_bytes([0x91; 32]);
        let causes = [0x92, 0x93, 0x94].map(|byte| {
            LifecycleCause::Evidence(world_model::EvidenceDeliveryId::from_bytes([byte; 32]))
        });
        let mut state = RuntimeControlState::empty();

        assert_eq!(
            state
                .lifecycle_mut()
                .request(actor, LifecycleRole::Appraisal, &causes[..1]),
            Ok(LifecycleWakeRequestOutcome::Enqueue {
                generation: LifecycleGeneration::new(1),
            })
        );
        assert_eq!(
            state
                .lifecycle_mut()
                .request(actor, LifecycleRole::Appraisal, &causes[1..2]),
            Ok(LifecycleWakeRequestOutcome::Coalesced {
                enqueued: LifecycleGeneration::new(1),
                desired: LifecycleGeneration::new(2),
            })
        );
        assert_eq!(
            state
                .lifecycle_mut()
                .request(actor, LifecycleRole::Appraisal, &causes[2..]),
            Ok(LifecycleWakeRequestOutcome::Coalesced {
                enqueued: LifecycleGeneration::new(1),
                desired: LifecycleGeneration::new(3),
            })
        );

        let completion = state
            .lifecycle_mut()
            .complete(actor, LifecycleRole::Appraisal, LifecycleGeneration::new(1))
            .unwrap_or_else(|error| panic!("enqueued generation must complete: {error:?}"));
        assert_eq!(completion.processed(), LifecycleGeneration::new(1));
        assert_eq!(completion.successor(), Some(LifecycleGeneration::new(3)));
        let record = state
            .lifecycle()
            .get(actor, LifecycleRole::Appraisal)
            .unwrap_or_else(|| panic!("control remains retained after completion"));
        assert_eq!(record.desired(), LifecycleGeneration::new(3));
        assert_eq!(record.processed(), LifecycleGeneration::new(1));
        assert_eq!(record.enqueued(), Some(LifecycleGeneration::new(3)));
        assert_eq!(
            record.enqueued_causes(LifecycleGeneration::new(3)),
            Some(vec![causes[1], causes[2]])
        );

        let successor = state
            .lifecycle_mut()
            .complete(actor, LifecycleRole::Appraisal, LifecycleGeneration::new(3))
            .unwrap_or_else(|error| panic!("final dirty generation must complete: {error:?}"));
        assert_eq!(successor.processed(), LifecycleGeneration::new(3));
        assert_eq!(successor.successor(), None);
    }

    #[test]
    fn lifecycle_control_rejects_empty_or_role_incompatible_causes() {
        let actor = ActorId::from_bytes([0xa1; 32]);
        let evidence =
            LifecycleCause::Evidence(world_model::EvidenceDeliveryId::from_bytes([0xa2; 32]));
        let mut state = RuntimeControlState::empty();

        assert_eq!(
            state
                .lifecycle_mut()
                .request(actor, LifecycleRole::Appraisal, &[]),
            Err(LifecycleControlError::EmptyCauseBatch)
        );
        assert_eq!(
            state
                .lifecycle_mut()
                .request(actor, LifecycleRole::IntentReview, &[evidence]),
            Err(LifecycleControlError::CauseRoleMismatch {
                role: LifecycleRole::IntentReview,
                cause: evidence,
            })
        );
        assert_eq!(state.lifecycle().get(actor, LifecycleRole::Appraisal), None);
        assert_eq!(
            state.lifecycle().get(actor, LifecycleRole::IntentReview),
            None
        );
    }
}
