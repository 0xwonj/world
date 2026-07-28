use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::action_evaluation::{ActionEvaluationCaptureOutcome, ActionEvaluationResultSubmission};
use crate::attempt::{AttemptAuthorityDomainId, AttemptKey, RunAttemptId};
use crate::execution::ResolvedExecutionClosureManifestV1;
use crate::kernel::{
    AdmitOutcome, AdmitRequest, FireOutcome, FirePreparation, FireRequest, KernelSafetyOutcome,
    ManageOutcome, ManageRequest, MomentWorkProposals, PreparedFire, PreparedFireFailure,
    PreparedFireFailureOutcome, PreparedKernelSafety,
};
use crate::service::{
    RuntimeActionEvaluationCaptureError, RuntimeControlError, RuntimeDriveError, RuntimeReadError,
    RuntimeStartError,
};

use super::aggregate::{AggregateRead, AttemptAggregate, OpenedAttempt};

static NEXT_REPOSITORY_ORDINAL: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub(crate) struct MemoryRepository {
    domain: AttemptAuthorityDomainId,
    state: Mutex<MemoryRepositoryState>,
}

#[derive(Debug, Default)]
struct MemoryRepositoryState {
    attempts: BTreeMap<RunAttemptId, AttemptAggregate>,
}

#[allow(
    clippy::result_large_err,
    reason = "repository transitions preserve complete terminal replay evidence in the runtime error"
)]
impl MemoryRepository {
    pub(crate) fn new() -> Result<Self, RuntimeStartError> {
        let ordinal = NEXT_REPOSITORY_ORDINAL
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| RuntimeStartError::AuthorityDomainExhausted)?;
        Ok(Self {
            domain: AttemptAuthorityDomainId::from_repository_ordinal(ordinal),
            state: Mutex::new(MemoryRepositoryState::default()),
        })
    }

    pub(crate) fn create_or_open(
        &self,
        closure: ResolvedExecutionClosureManifestV1,
        key: AttemptKey,
    ) -> Result<OpenedAttempt, RuntimeStartError> {
        let creation = AttemptAggregate::derive_creation(self.domain, key, &closure);
        let attempt = creation.binding().attempt();
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStartError::Unavailable)?;
        let aggregate = match state.attempts.entry(attempt) {
            Entry::Occupied(entry) => {
                let aggregate = entry.into_mut();
                aggregate.verify_creation(creation)?;
                aggregate
            }
            Entry::Vacant(entry) => entry.insert(AttemptAggregate::create(creation, closure)?),
        };
        aggregate.open()
    }

    pub(crate) fn read(&self, attempt: RunAttemptId) -> Result<AggregateRead, RuntimeReadError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RuntimeReadError::Unavailable)?;
        state
            .attempts
            .get(&attempt)
            .ok_or(RuntimeReadError::AttemptNotFound)
            .map(AttemptAggregate::read)
    }

    #[cfg(test)]
    pub(crate) fn cursor(
        &self,
        attempt: RunAttemptId,
    ) -> Result<crate::authority::AuthorityCursor, RuntimeReadError> {
        self.read(attempt).map(|read| read.cursor())
    }

    #[cfg(test)]
    pub(crate) fn snapshot(
        &self,
        attempt: RunAttemptId,
    ) -> Result<world_model::WorldSnapshot, RuntimeReadError> {
        self.read(attempt).map(|read| read.snapshot().clone())
    }

    #[cfg(test)]
    pub(crate) fn reconcile_for_open(
        &self,
        attempt: RunAttemptId,
    ) -> Result<(), RuntimeStartError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeStartError::Unavailable)?;
        state
            .attempts
            .get_mut(&attempt)
            .ok_or(RuntimeStartError::AttemptNotFound)?
            .reconcile_for_open()
    }

    pub(crate) fn admit(
        &self,
        attempt: RunAttemptId,
        request: AdmitRequest,
    ) -> Result<AdmitOutcome, RuntimeDriveError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeDriveError::Unavailable)?;
        state
            .attempts
            .get_mut(&attempt)
            .ok_or(RuntimeDriveError::AttemptNotFound)?
            .admit(request)
    }

    pub(crate) fn manage(
        &self,
        attempt: RunAttemptId,
        request: ManageRequest,
    ) -> Result<ManageOutcome, RuntimeDriveError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeDriveError::Unavailable)?;
        state
            .attempts
            .get_mut(&attempt)
            .ok_or(RuntimeDriveError::AttemptNotFound)?
            .manage(request)
    }

    pub(crate) fn capture_action_evaluation_result(
        &self,
        attempt: RunAttemptId,
        submission: ActionEvaluationResultSubmission,
    ) -> Result<ActionEvaluationCaptureOutcome, RuntimeActionEvaluationCaptureError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeActionEvaluationCaptureError::Unavailable)?;
        state
            .attempts
            .get_mut(&attempt)
            .ok_or(RuntimeActionEvaluationCaptureError::AttemptNotFound)?
            .capture_action_evaluation_result(submission)
    }

    pub(crate) fn prepare_fire(
        &self,
        attempt: RunAttemptId,
        request: FireRequest,
    ) -> Result<FirePreparation, RuntimeDriveError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeDriveError::Unavailable)?;
        state
            .attempts
            .get_mut(&attempt)
            .ok_or(RuntimeDriveError::AttemptNotFound)?
            .prepare_fire(self.domain, attempt, request)
    }

    pub(crate) fn complete_kernel_safety(
        &self,
        attempt: RunAttemptId,
        prepared: PreparedKernelSafety,
    ) -> Result<KernelSafetyOutcome, RuntimeDriveError> {
        AttemptAggregate::verify_prepared_safety_target(self.domain, attempt, &prepared)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeDriveError::Unavailable)?;
        state
            .attempts
            .get_mut(&attempt)
            .ok_or(RuntimeDriveError::AttemptNotFound)?
            .complete_kernel_safety(prepared)
    }

    pub(crate) fn complete_fire(
        &self,
        attempt: RunAttemptId,
        prepared: PreparedFire,
        proposals: MomentWorkProposals,
    ) -> Result<FireOutcome, RuntimeDriveError> {
        AttemptAggregate::verify_prepared_target(self.domain, attempt, &prepared)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeDriveError::Unavailable)?;
        state
            .attempts
            .get_mut(&attempt)
            .ok_or(RuntimeDriveError::AttemptNotFound)?
            .complete_fire(prepared, proposals)
    }

    pub(crate) fn fail_prepared_fire(
        &self,
        attempt: RunAttemptId,
        prepared: PreparedFire,
        failure: PreparedFireFailure,
    ) -> Result<PreparedFireFailureOutcome, RuntimeControlError> {
        AttemptAggregate::verify_prepared_control_target(self.domain, attempt, &prepared)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeControlError::Unavailable)?;
        state
            .attempts
            .get_mut(&attempt)
            .ok_or(RuntimeControlError::AttemptNotFound)?
            .fail_prepared_fire(prepared, failure)
    }

    pub(crate) fn cancel_attempt(
        &self,
        attempt: RunAttemptId,
        request: crate::attempt::CancelAttemptRequest,
    ) -> Result<crate::attempt::CancelAttemptOutcome, RuntimeControlError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RuntimeControlError::Unavailable)?;
        state
            .attempts
            .get_mut(&attempt)
            .ok_or(RuntimeControlError::AttemptNotFound)?
            .cancel_attempt(request)
    }
}

#[cfg(test)]
use super::aggregate::{append_and_publish, reconcile};

#[cfg(test)]
#[path = "aggregate_contract_tests.rs"]
mod aggregate_contract_tests;
