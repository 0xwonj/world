use std::collections::{BTreeMap, BTreeSet};

use world_core::{
    AuthorityClass, ProcessInstanceId, ProvenanceKey, ReservationId, ScheduledWakeupId,
    SimulationTime, StoreCursor, WakeupOrderKey,
};

use crate::{InvalidationPackage, InvalidationSource, ModelError, StoreFamily};

use super::{
    lifecycle::ProcessLifecycle,
    process::ProcessInstanceRecord,
    record::{RuntimeControlRecord, RuntimeControlRecordKind, RuntimeControlRecordPayload},
    reservation::{ReservationRecord, ReservationState},
    update::{
        AcceptedRuntimeControlUpdate, RuntimeControlChange, RuntimeControlUpdateRecord,
        StoredRuntimeControlUpdate,
    },
    wakeup::{
        ScheduledWakeupRecord, ScheduledWakeupStatus, WakeupTarget, WakeupTerminalTransition,
    },
};

/// Store for durable runtime-control state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeControlStore {
    next_update_cursor: StoreCursor,
    records: BTreeMap<RuntimeControlRecordKind, RuntimeControlRecord>,
    update_order: BTreeMap<StoreCursor, StoredRuntimeControlUpdate>,
    due_wakeups: BTreeMap<(WakeupOrderKey, ScheduledWakeupId), ScheduledWakeupId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeControlUpdateApplyPlan {
    update_cursor: StoreCursor,
    next_update_cursor: StoreCursor,
    changes: RuntimeControlChangeApplyPlan,
}

impl RuntimeControlUpdateApplyPlan {
    pub(crate) fn new(
        update_cursor: StoreCursor,
        next_update_cursor: StoreCursor,
        changes: RuntimeControlChangeApplyPlan,
    ) -> Self {
        Self {
            update_cursor,
            next_update_cursor,
            changes,
        }
    }

    pub(crate) const fn update_cursor(&self) -> StoreCursor {
        self.update_cursor
    }

    pub(crate) const fn next_update_cursor(&self) -> StoreCursor {
        self.next_update_cursor
    }

    pub(crate) fn into_changes(self) -> RuntimeControlChangeApplyPlan {
        self.changes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeControlChangeApplyPlan {
    deltas: Vec<RuntimeControlDelta>,
    changed_records: Vec<RuntimeControlRecordKind>,
}

impl RuntimeControlChangeApplyPlan {
    fn new(deltas: Vec<RuntimeControlDelta>) -> Self {
        let changed_records = deltas
            .iter()
            .map(RuntimeControlDelta::kind)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Self {
            deltas,
            changed_records,
        }
    }

    fn into_deltas(self) -> Vec<RuntimeControlDelta> {
        self.deltas
    }
}

impl RuntimeControlUpdateApplyPlan {
    pub(crate) fn changed_records(&self) -> &[RuntimeControlRecordKind] {
        self.changes.changed_records()
    }
}

impl RuntimeControlChangeApplyPlan {
    pub(crate) fn changed_records(&self) -> &[RuntimeControlRecordKind] {
        &self.changed_records
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeControlDelta {
    kind: RuntimeControlRecordKind,
    record: RuntimeControlRecord,
}

impl RuntimeControlDelta {
    fn new(kind: RuntimeControlRecordKind, record: RuntimeControlRecord) -> Self {
        Self { kind, record }
    }

    fn materialize(
        payload: RuntimeControlRecordPayload,
        updated_at: SimulationTime,
        provenance: Option<ProvenanceKey>,
    ) -> Self {
        let kind = payload.kind();
        Self::new(
            kind,
            RuntimeControlRecord::new(payload, updated_at, provenance),
        )
    }

    const fn kind(&self) -> RuntimeControlRecordKind {
        self.kind
    }

    const fn record(&self) -> &RuntimeControlRecord {
        &self.record
    }
}

struct RuntimeControlPlanningOverlay<'store> {
    base: &'store BTreeMap<RuntimeControlRecordKind, RuntimeControlRecord>,
    planned: BTreeMap<RuntimeControlRecordKind, RuntimeControlRecord>,
    transitioned_wakeups: BTreeMap<ScheduledWakeupId, WakeupTerminalTransition>,
    required_wakeup_transitions: BTreeSet<ScheduledWakeupId>,
}

impl<'store> RuntimeControlPlanningOverlay<'store> {
    fn new(base: &'store BTreeMap<RuntimeControlRecordKind, RuntimeControlRecord>) -> Self {
        Self {
            base,
            planned: BTreeMap::new(),
            transitioned_wakeups: BTreeMap::new(),
            required_wakeup_transitions: BTreeSet::new(),
        }
    }

    fn contains(&self, kind: RuntimeControlRecordKind) -> bool {
        self.planned.contains_key(&kind) || self.base.contains_key(&kind)
    }

    fn record(&self, kind: RuntimeControlRecordKind) -> Option<&RuntimeControlRecord> {
        self.planned.get(&kind).or_else(|| self.base.get(&kind))
    }

    fn process(&self, id: ProcessInstanceId) -> Result<&ProcessInstanceRecord, ModelError> {
        let kind = RuntimeControlRecordKind::Process(id);
        match self.record(kind).map(RuntimeControlRecord::payload) {
            Some(RuntimeControlRecordPayload::Process(process)) => Ok(process),
            _ => Err(ModelError::MissingRuntimeControlRecord { kind }),
        }
    }

    fn wakeup(&self, id: ScheduledWakeupId) -> Result<&ScheduledWakeupRecord, ModelError> {
        let kind = RuntimeControlRecordKind::ScheduledWakeup(id);
        match self.record(kind).map(RuntimeControlRecord::payload) {
            Some(RuntimeControlRecordPayload::ScheduledWakeup(wakeup)) => Ok(wakeup),
            _ => Err(ModelError::MissingRuntimeControlRecord { kind }),
        }
    }

    fn reservation(&self, id: ReservationId) -> Result<&ReservationRecord, ModelError> {
        let kind = RuntimeControlRecordKind::Reservation(id);
        match self.record(kind).map(RuntimeControlRecord::payload) {
            Some(RuntimeControlRecordPayload::Reservation(reservation)) => Ok(reservation),
            _ => Err(ModelError::MissingRuntimeControlRecord { kind }),
        }
    }

    fn insert(&mut self, delta: &RuntimeControlDelta) {
        self.planned.insert(delta.kind, delta.record.clone());
    }

    fn transition_wakeup(
        &mut self,
        wakeup: ScheduledWakeupId,
        transition: WakeupTerminalTransition,
    ) {
        self.transitioned_wakeups.insert(wakeup, transition);
    }

    fn require_wakeup_transition(&mut self, wakeup: ScheduledWakeupId) {
        self.required_wakeup_transitions.insert(wakeup);
    }

    fn has_transitioned_wakeup(&self, wakeup: ScheduledWakeupId) -> bool {
        self.transitioned_wakeups.contains_key(&wakeup)
    }

    fn for_each_effective_record(
        &self,
        mut visit: impl FnMut(&RuntimeControlRecord) -> Result<(), ModelError>,
    ) -> Result<(), ModelError> {
        for (kind, record) in self.base {
            if !self.planned.contains_key(kind) {
                visit(record)?;
            }
        }
        for record in self.planned.values() {
            visit(record)?;
        }
        Ok(())
    }
}

impl Default for RuntimeControlStore {
    fn default() -> Self {
        Self {
            next_update_cursor: StoreCursor::INITIAL,
            records: BTreeMap::new(),
            update_order: BTreeMap::new(),
            due_wakeups: BTreeMap::new(),
        }
    }
}

impl RuntimeControlStore {
    /// Returns whether a runtime-control record exists.
    pub fn contains(&self, kind: RuntimeControlRecordKind) -> bool {
        self.records.contains_key(&kind)
    }

    /// Returns a runtime-control record.
    pub fn record(&self, kind: RuntimeControlRecordKind) -> Option<&RuntimeControlRecord> {
        self.records.get(&kind)
    }

    /// Returns a process record.
    pub fn process(&self, id: ProcessInstanceId) -> Option<&ProcessInstanceRecord> {
        match self
            .record(RuntimeControlRecordKind::Process(id))?
            .payload()
        {
            RuntimeControlRecordPayload::Process(record) => Some(record),
            RuntimeControlRecordPayload::Reservation(_)
            | RuntimeControlRecordPayload::ScheduledWakeup(_) => None,
        }
    }

    /// Returns a reservation record.
    pub fn reservation(&self, id: ReservationId) -> Option<&ReservationRecord> {
        match self
            .record(RuntimeControlRecordKind::Reservation(id))?
            .payload()
        {
            RuntimeControlRecordPayload::Reservation(record) => Some(record),
            RuntimeControlRecordPayload::Process(_)
            | RuntimeControlRecordPayload::ScheduledWakeup(_) => None,
        }
    }

    /// Returns a scheduled wakeup record.
    pub fn scheduled_wakeup(&self, id: ScheduledWakeupId) -> Option<&ScheduledWakeupRecord> {
        match self
            .record(RuntimeControlRecordKind::ScheduledWakeup(id))?
            .payload()
        {
            RuntimeControlRecordPayload::ScheduledWakeup(record) => Some(record),
            RuntimeControlRecordPayload::Process(_)
            | RuntimeControlRecordPayload::Reservation(_) => None,
        }
    }

    /// Returns the number of current runtime-control records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns the number of accepted runtime-control updates.
    pub fn update_count(&self) -> usize {
        self.update_order.len()
    }

    /// Iterates runtime-control records in key order.
    pub fn records(&self) -> impl Iterator<Item = &RuntimeControlRecord> {
        self.records.values()
    }

    /// Iterates accepted runtime-control updates in append order.
    pub fn updates(&self) -> impl Iterator<Item = &StoredRuntimeControlUpdate> {
        self.update_order.values()
    }

    /// Iterates active wakeups due at or before `until`, in scheduler order.
    pub fn due_wakeups(
        &self,
        until: SimulationTime,
    ) -> impl Iterator<Item = &ScheduledWakeupRecord> {
        self.due_wakeups
            .iter()
            .take_while(move |((order, _), _)| order.time() <= until)
            .filter_map(|(_, id)| self.scheduled_wakeup(*id))
    }
}

impl RuntimeControlStore {
    pub(crate) fn plan_control_update(
        &self,
        update: &AcceptedRuntimeControlUpdate,
    ) -> Result<RuntimeControlUpdateApplyPlan, ModelError> {
        validate_control_invalidation(update.invalidation())?;
        let changes = self.plan_changes(update.changes())?;
        let update_cursor = self.next_update_cursor;
        let Some(next_update_cursor) = update_cursor.next() else {
            return Err(ModelError::StoreCursorExhausted);
        };

        Ok(RuntimeControlUpdateApplyPlan::new(
            update_cursor,
            next_update_cursor,
            changes,
        ))
    }

    pub(crate) fn plan_transaction_coupled_changes(
        &self,
        changes: &[RuntimeControlChange],
    ) -> Result<RuntimeControlChangeApplyPlan, ModelError> {
        self.plan_changes(changes)
    }

    fn plan_changes(
        &self,
        changes: &[RuntimeControlChange],
    ) -> Result<RuntimeControlChangeApplyPlan, ModelError> {
        if changes.is_empty() {
            return Ok(RuntimeControlChangeApplyPlan::new(Vec::new()));
        }

        let mut planned = RuntimeControlPlanningOverlay::new(&self.records);
        let mut deltas = Vec::new();
        for change in changes {
            let delta = plan_change(&mut planned, change)?;
            planned.insert(&delta);
            deltas.push(delta);
        }

        validate_process_wakeup_links(&planned)?;

        Ok(RuntimeControlChangeApplyPlan::new(deltas))
    }

    pub(crate) fn apply_planned_control_update(
        &mut self,
        update: AcceptedRuntimeControlUpdate,
        plan: RuntimeControlUpdateApplyPlan,
    ) -> (StoreCursor, Vec<RuntimeControlRecordKind>) {
        let (header, _, _) = update.into_parts();
        let changed = plan.changed_records().to_vec();
        let update_cursor = plan.update_cursor();
        let next_update_cursor = plan.next_update_cursor();
        self.apply_changes(plan.into_changes());
        self.update_order.insert(
            update_cursor,
            StoredRuntimeControlUpdate::new(
                RuntimeControlUpdateRecord::new(header, changed.clone()),
                update_cursor,
            ),
        );
        self.next_update_cursor = next_update_cursor;
        (update_cursor, changed)
    }

    pub(crate) fn apply_planned_transaction_changes(
        &mut self,
        plan: RuntimeControlChangeApplyPlan,
    ) -> Vec<RuntimeControlRecordKind> {
        let changed = plan.changed_records().to_vec();
        self.apply_changes(plan);
        changed
    }

    fn apply_changes(&mut self, plan: RuntimeControlChangeApplyPlan) {
        for delta in plan.into_deltas() {
            self.insert_materialized_record(delta.kind, delta.record);
        }
    }

    fn insert_materialized_record(
        &mut self,
        kind: RuntimeControlRecordKind,
        record: RuntimeControlRecord,
    ) {
        self.remove_due_wakeup(kind);
        if let RuntimeControlRecordPayload::ScheduledWakeup(wakeup) = record.payload()
            && wakeup.status().is_scheduled()
        {
            self.due_wakeups
                .insert((wakeup.order(), wakeup.id()), wakeup.id());
        }
        self.records.insert(kind, record);
    }

    fn remove_due_wakeup(&mut self, kind: RuntimeControlRecordKind) {
        let RuntimeControlRecordKind::ScheduledWakeup(id) = kind else {
            return;
        };
        if let Some(existing) = self.scheduled_wakeup(id) {
            self.due_wakeups.remove(&(existing.order(), id));
        }
    }

    #[cfg(test)]
    pub(crate) fn insert(&mut self, record: RuntimeControlRecord) -> Result<(), ModelError> {
        let kind = record.kind();
        let planned = RuntimeControlPlanningOverlay::new(&self.records);
        validate_create_record(&planned, kind, &record)?;
        if let RuntimeControlRecordPayload::Reservation(reservation) = record.payload()
            && reservation.state().is_held()
        {
            validate_held_reservation_conflict(&planned, reservation)?;
        }
        self.insert_materialized_record(kind, record);
        Ok(())
    }
}

fn validate_control_invalidation(package: &InvalidationPackage) -> Result<(), ModelError> {
    if package.source() != InvalidationSource::RuntimeControl {
        return Err(ModelError::InvalidRuntimeControlInvalidation {
            invalidation_source: package.source(),
        });
    }

    if !package.contains_authority_class(AuthorityClass::RuntimeControl) {
        return Err(ModelError::MissingRuntimeControlAuthorityInvalidation {
            authority: AuthorityClass::RuntimeControl,
        });
    }

    if !package.contains_store_family(StoreFamily::RuntimeControl) {
        return Err(ModelError::MissingRuntimeControlStoreInvalidation {
            store: StoreFamily::RuntimeControl,
        });
    }

    Ok(())
}

fn transition_time(transition: &WakeupTerminalTransition) -> SimulationTime {
    match transition {
        WakeupTerminalTransition::Consumed { at, .. }
        | WakeupTerminalTransition::Canceled { at, .. }
        | WakeupTerminalTransition::Skipped { at, .. } => *at,
    }
}

fn plan_change(
    planned: &mut RuntimeControlPlanningOverlay<'_>,
    change: &RuntimeControlChange,
) -> Result<RuntimeControlDelta, ModelError> {
    match change {
        RuntimeControlChange::CreateProcess {
            process,
            updated_at,
            provenance,
        } => {
            let delta = RuntimeControlDelta::materialize(
                RuntimeControlRecordPayload::Process(process.clone()),
                *updated_at,
                *provenance,
            );
            validate_create_record(planned, delta.kind(), delta.record())?;
            Ok(delta)
        }
        RuntimeControlChange::UpdateProcess {
            process,
            updated_at,
            provenance,
        } => {
            let existing = planned.process(process.id())?;
            if let Some(wakeup) = validate_process_update(existing, process)? {
                planned.require_wakeup_transition(wakeup);
            }
            Ok(RuntimeControlDelta::materialize(
                RuntimeControlRecordPayload::Process(process.clone()),
                *updated_at,
                *provenance,
            ))
        }
        RuntimeControlChange::ScheduleWakeup {
            wakeup,
            updated_at,
            provenance,
        } => {
            let delta = RuntimeControlDelta::materialize(
                RuntimeControlRecordPayload::ScheduledWakeup(wakeup.clone()),
                *updated_at,
                *provenance,
            );
            validate_create_record(planned, delta.kind(), delta.record())?;
            if !wakeup.status().is_scheduled() {
                return Err(ModelError::InvalidWakeupTransition {
                    wakeup: wakeup.id(),
                });
            }
            Ok(delta)
        }
        RuntimeControlChange::TransitionWakeup { wakeup, transition } => {
            let kind = RuntimeControlRecordKind::ScheduledWakeup(*wakeup);
            let existing = planned.wakeup(*wakeup)?;
            if !existing.status().is_scheduled() {
                return Err(ModelError::InvalidWakeupTransition { wakeup: *wakeup });
            }
            let provenance = planned
                .record(kind)
                .and_then(RuntimeControlRecord::provenance);
            let updated = existing
                .clone()
                .with_status(ScheduledWakeupStatus::from(transition.clone()));
            planned.transition_wakeup(*wakeup, transition.clone());
            Ok(RuntimeControlDelta::materialize(
                RuntimeControlRecordPayload::ScheduledWakeup(updated),
                transition_time(transition),
                provenance,
            ))
        }
        RuntimeControlChange::AcquireReservation {
            reservation,
            updated_at,
            provenance,
        } => {
            let delta = RuntimeControlDelta::materialize(
                RuntimeControlRecordPayload::Reservation(reservation.clone()),
                *updated_at,
                *provenance,
            );
            validate_create_record(planned, delta.kind(), delta.record())?;
            validate_held_reservation_conflict(planned, reservation)?;
            Ok(delta)
        }
        RuntimeControlChange::TransitionReservation {
            reservation,
            transition,
        } => {
            let existing = planned.reservation(*reservation)?;
            let ReservationState::Held { acquired_at } = existing.state() else {
                return Err(ModelError::InvalidReservationTransition {
                    reservation: *reservation,
                });
            };
            let updated = ReservationRecord::new(
                existing.id(),
                existing.holder().clone(),
                existing.target().clone(),
                transition.clone().into_state(*acquired_at),
                existing.provenance(),
            );
            Ok(RuntimeControlDelta::materialize(
                RuntimeControlRecordPayload::Reservation(updated),
                transition.transition_time(),
                existing.provenance(),
            ))
        }
    }
}

fn validate_create_record(
    planned: &RuntimeControlPlanningOverlay<'_>,
    kind: RuntimeControlRecordKind,
    record: &RuntimeControlRecord,
) -> Result<(), ModelError> {
    if planned.contains(kind) {
        return Err(ModelError::DuplicateRuntimeControlRecord { kind });
    }
    if record.kind() != kind {
        return Err(ModelError::MissingRuntimeControlRecord { kind });
    }
    Ok(())
}

fn validate_process_update(
    existing: &ProcessInstanceRecord,
    next: &ProcessInstanceRecord,
) -> Result<Option<ScheduledWakeupId>, ModelError> {
    if existing.definition() != next.definition()
        || existing.owner() != next.owner()
        || existing.roles() != next.roles()
        || existing.resolution() != next.resolution()
        || existing.version() != next.version()
        || existing.provenance() != next.provenance()
    {
        return Err(ModelError::InvalidProcessTransition { process: next.id() });
    }

    if existing.lifecycle().is_terminal() {
        return Err(ModelError::InvalidProcessTransition { process: next.id() });
    }

    if !is_allowed_process_lifecycle_transition(existing.lifecycle(), next.lifecycle()) {
        return Err(ModelError::InvalidProcessTransition { process: next.id() });
    }

    let required_wakeup_transition = if let ProcessLifecycle::Scheduled { wakeup } =
        existing.lifecycle()
        && existing.lifecycle() != next.lifecycle()
    {
        Some(*wakeup)
    } else {
        None
    };

    Ok(required_wakeup_transition)
}

fn is_allowed_process_lifecycle_transition(from: &ProcessLifecycle, to: &ProcessLifecycle) -> bool {
    match lifecycle_class(from) {
        ProcessLifecycleClass::Created => matches!(
            to,
            ProcessLifecycle::Scheduled { .. }
                | ProcessLifecycle::Waiting { .. }
                | ProcessLifecycle::Paused { .. }
                | ProcessLifecycle::Interrupted { .. }
                | ProcessLifecycle::Failed { .. }
                | ProcessLifecycle::Abandoned
        ),
        ProcessLifecycleClass::Scheduled => matches!(
            to,
            ProcessLifecycle::Scheduled { .. }
                | ProcessLifecycle::Advancing
                | ProcessLifecycle::Waiting { .. }
                | ProcessLifecycle::Paused { .. }
                | ProcessLifecycle::Interrupted { .. }
                | ProcessLifecycle::Completed
                | ProcessLifecycle::Failed { .. }
                | ProcessLifecycle::Abandoned
        ),
        ProcessLifecycleClass::Waiting => matches!(
            to,
            ProcessLifecycle::Scheduled { .. }
                | ProcessLifecycle::Interrupted { .. }
                | ProcessLifecycle::Failed { .. }
                | ProcessLifecycle::Abandoned
        ),
        ProcessLifecycleClass::Paused => matches!(
            to,
            ProcessLifecycle::Scheduled { .. }
                | ProcessLifecycle::Interrupted { .. }
                | ProcessLifecycle::Abandoned
        ),
        ProcessLifecycleClass::Interrupted => matches!(
            to,
            ProcessLifecycle::Scheduled { .. }
                | ProcessLifecycle::Failed { .. }
                | ProcessLifecycle::Abandoned
        ),
        ProcessLifecycleClass::Advancing => matches!(
            to,
            ProcessLifecycle::Scheduled { .. }
                | ProcessLifecycle::Waiting { .. }
                | ProcessLifecycle::Paused { .. }
                | ProcessLifecycle::Interrupted { .. }
                | ProcessLifecycle::Completed
                | ProcessLifecycle::Failed { .. }
                | ProcessLifecycle::Abandoned
        ),
        ProcessLifecycleClass::Terminal => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcessLifecycleClass {
    Created,
    Scheduled,
    Waiting,
    Advancing,
    Paused,
    Interrupted,
    Terminal,
}

fn lifecycle_class(lifecycle: &ProcessLifecycle) -> ProcessLifecycleClass {
    match lifecycle {
        ProcessLifecycle::Created => ProcessLifecycleClass::Created,
        ProcessLifecycle::Scheduled { .. } => ProcessLifecycleClass::Scheduled,
        ProcessLifecycle::Waiting { .. } => ProcessLifecycleClass::Waiting,
        ProcessLifecycle::Advancing => ProcessLifecycleClass::Advancing,
        ProcessLifecycle::Paused { .. } => ProcessLifecycleClass::Paused,
        ProcessLifecycle::Interrupted { .. } => ProcessLifecycleClass::Interrupted,
        ProcessLifecycle::Completed
        | ProcessLifecycle::Failed { .. }
        | ProcessLifecycle::Abandoned => ProcessLifecycleClass::Terminal,
    }
}

fn validate_process_wakeup_links(
    planned: &RuntimeControlPlanningOverlay<'_>,
) -> Result<(), ModelError> {
    planned.for_each_effective_record(|record| {
        let RuntimeControlRecordPayload::Process(process) = record.payload() else {
            return Ok(());
        };
        let ProcessLifecycle::Scheduled { wakeup } = process.lifecycle() else {
            return Ok(());
        };
        let wakeup_record = planned.wakeup(*wakeup)?;
        if !wakeup_record.status().is_scheduled() {
            return Err(ModelError::InvalidWakeupTransition { wakeup: *wakeup });
        }
        match wakeup_record.target() {
            WakeupTarget::Process(target) | WakeupTarget::PassiveProcess(target)
                if *target == process.id() => {}
            _ => {
                return Err(ModelError::InvalidProcessTransition {
                    process: process.id(),
                });
            }
        }
        Ok(())
    })?;

    for wakeup in &planned.required_wakeup_transitions {
        if !planned.has_transitioned_wakeup(*wakeup) {
            return Err(ModelError::InvalidWakeupTransition { wakeup: *wakeup });
        }
    }

    for wakeup in planned.transitioned_wakeups.keys() {
        let wakeup_record = planned.wakeup(*wakeup)?;
        if wakeup_record.status().is_scheduled() {
            return Err(ModelError::InvalidWakeupTransition { wakeup: *wakeup });
        }
    }

    Ok(())
}

fn validate_held_reservation_conflict(
    planned: &RuntimeControlPlanningOverlay<'_>,
    reservation: &ReservationRecord,
) -> Result<(), ModelError> {
    if !reservation.state().is_held() {
        return Err(ModelError::InvalidReservationTransition {
            reservation: reservation.id(),
        });
    }

    planned.for_each_effective_record(|record| {
        let RuntimeControlRecordPayload::Reservation(existing) = record.payload() else {
            return Ok(());
        };
        if existing.id() != reservation.id()
            && existing.state().is_held()
            && existing.target() == reservation.target()
        {
            return Err(ModelError::DuplicateActiveReservation {
                reservation: existing.id(),
                target: reservation.target().clone(),
            });
        }
        Ok(())
    })?;

    Ok(())
}
