# Phase 5 Local Plan: Runtime Control, Time, And Process

## Status

Reviewed local implementation plan.

This plan is ready to drive Phase 5 implementation. The first implementation
step should preserve the baseline decisions below unless current code proves a
specific decision unworkable.

## Purpose

Add durable runtime-control authority on top of the Phase 4 causal mutation
waist.

The phase target is:

```text
ScheduledWakeup / host control request / process continuation
  -> runtime-owned validation and transition computation
  -> RuntimeControlUpdate staging
  -> AcceptedRuntimeControlUpdate or transaction-coupled RuntimeControlChange
  -> WorldModel receiver applies runtime-control state and invalidation
     atomically
```

The phase is complete when the runtime can:

- store and query durable runtime-control records;
- schedule, cancel, consume, and skip wakeups with provenance;
- drain due wakeups in deterministic order with bounded progress;
- represent a process as explicit saved state, not an execution stack;
- advance a minimal process through scheduling, progress, completion, waiting,
  pause, interruption, failure, or abandonment;
- apply runtime-control changes through accepted gates;
- stage runtime-control changes with hard transactions when atomicity is
  required;
- explain why work continued, stopped, blocked, skipped, failed, completed, or
  produced a host input opportunity.

## Local Contract

Phase 5 extends the same authority shape used by Phase 4:

```text
Hard truth:
  CausalRuntime
    -> AcceptedHardCommit
    -> WorldModel::apply_hard_commit

Runtime control:
  RuntimeControlGate / Scheduler / ProcessRuntime
    -> AcceptedRuntimeControlUpdate
    -> WorldModel::apply_runtime_control_update
```

Runtime-control state is authority-bearing engine state. It affects scheduling,
validation, interruption, replay, and future mutation. It is not hard physical
truth and not semantic/social/appraisal meaning.

The core rule:

```text
Runtime components compute transitions.
WorldModel applies accepted packages.
No scheduler, process, reservation, or activity path receives raw mutable store
authority.
```

## Baseline Decisions

### Crate Direction

This is the Phase 5 touched-crate subset:

```text
world-core
  <- world-defs
  <- world-model
  <- world-runtime
```

Do not add these edges:

```text
world-model -> world-runtime
world-runtime -> world-standard
world-runtime -> world-context
world-runtime -> world-decision
```

Do not add a new scheduler/runtime framework dependency. Use standard library
ordered collections first.

### Accepted Update Identity

Use model-assigned append cursors for accepted runtime-control update history
in the first implementation.

Do not add `RuntimeControlUpdateId` unless model-assigned `StoreCursor` proves
insufficient for a real cross-crate reference. Runtime-control current-state
records continue to use domain ids such as `ProcessInstanceId`,
`ReservationId`, and `ScheduledWakeupId`.

### Accepted Package Visibility

The same Rust caveat from Phase 4 applies here. Rust has no friend-crate
visibility, so a type that lives in `world-model` and can be constructed by
`world-runtime` cannot be made constructible only by `world-runtime` without a
crate-boundary change.

Phase 5 should therefore enforce the strongest boundary available in the
current crate layout:

- no public raw runtime-control store mutators;
- no public partial wakeup/reservation/process mutation API;
- one model receiver for accepted runtime-control updates;
- model constructors validate package shape and storage invariants;
- runtime code owns transition semantics and is the only normal producer of
  accepted packages;
- tests exercise the runtime path rather than constructing model records as the
  main behavior surface.

Changing this compile-time forge-resistance story is an architecture decision,
not a Phase 5 coding detail.

### Runtime-Control Update History

Add a runtime-control update history surface in `world-model`, either inside
`RuntimeControlStore` or as a sibling store if that keeps the file cleaner.

The history stores accepted update envelopes and append cursors. It is not
`EventHistoryStore`, and runtime-control records are not hard `EventRecord`s
unless a hard transaction explicitly emits hard evidence.

### Transaction-Coupled Control Changes

Do not nest a control-only `AcceptedRuntimeControlUpdate` inside
`AcceptedHardCommit`.

For transaction-coupled runtime-control state, extend the hard commit package
with a `Vec<RuntimeControlChange>` or a small `RuntimeControlChangeSet`.

The hard commit keeps one invalidation source:

```text
InvalidationSource::HardCommit(transaction_id)
```

When a hard commit includes runtime-control changes, its single invalidation
package must mark both:

```text
AuthorityClass::Hard
AuthorityClass::RuntimeControl
StoreFamily::EventHistory
StoreFamily::RuntimeControl
```

plus any hard store families changed by the transaction.

### Id Issuer Ownership

The Phase 5 runtime facade owns runtime-control id issuers, following the same
pattern as `CausalRuntime` owning transaction and event ids.

Initial issuer owner:

```rust
pub struct CausalRuntime {
    transaction_ids: CausalTransactionIdIssuer,
    event_ids: EventRecordIdIssuer,
    process_ids: ProcessInstanceIdIssuer,
    activity_ids: ActivityIdIssuer,
    reservation_ids: ReservationIdIssuer,
    wakeup_ids: ScheduledWakeupIdIssuer,
}
```

If the file gets too broad, factor the runtime-control issuers into an internal
`RuntimeControlIds` value owned by `CausalRuntime`. Do not move issuer ownership
into `WorldModel`.

### Process Scope

Keep `ProcessRuntime` minimal.

`world-defs::ProcessDef` currently provides roles, state schema, resolution
support, policy keys, effect programs, event contracts, and stage permissions.
It does not provide executable tick/wait/interruption policy semantics.

Therefore Phase 5 implements:

- definition lookup;
- resolution support validation;
- role/state envelope storage;
- a tiny built-in progress rule for tests and baseline process mechanics;
- status transitions that are explicit and durable;
- optional production of `RuntimeRequest` only through narrow testable paths.

Do not implement a process policy interpreter in Phase 5.

### Reservation Lane

Reservation acquire/release that gates executable work defaults to the
transaction-coupled lane.

Control-only reservation updates are allowed only for clearly non-hard
administrative lifecycle changes, such as cancellation, expiry, cleanup, or
host-directed abandonment that does not validate or unlock a hard outcome.

The first conflict rule is deliberately narrow:

```text
one active exclusive reservation per ReservationTarget
```

Full resource semantics, shared locks, priority preemption, deadlock handling,
and rich reservation policies are later work.

### Wakeup Skip Semantics

A due wakeup must not disappear without accepted provenance.

Add `WakeupTerminalTransition::Skipped` and a durable `Skipped` wakeup status,
or implement an equivalent accepted state transition. Use it for stale due work
such as a wakeup targeting a completed, canceled, or missing process.

`WakeupDrainResult::Skipped` is a report item backed by accepted provenance,
not a report-only disappearance.

### Activity Scope

Activity state is optional and minimal in Phase 5.

Only add `ActivityRecord` if it is needed to link a process to an actor-facing
activity in tests. Do not implement the full activity lifecycle, intent
selection, or activity planning in this phase.

### RNG Scope

Add RNG stream/draw records only for paths that claim
`ReplayLevel::DeterministicCommandReplay` or need committed RNG provenance.

For ordinary Phase 5 scheduler/process tests, prefer `ReplayLevel::AuditOnly`
and avoid RNG entirely.

### Host-Facing Stops

`DrainOutcome::InputOpportunity` and `DrainOutcome::MandatoryPrompt` are
host-facing stop reasons. They are not the player UI turn shell and should not
pull actor context, protagonist UI, observation projection, or input handling
into Phase 5.

Use a generic host/input wakeup target for tests rather than protagonist-
specific behavior unless the current code already has the actor-context needed
to support it cleanly.

### Durability Scope

Phase 5 creates model-stored durable value shapes. It does not define a
save-file format, serde representation, migration policy, or long-term schema
compatibility contract.

## Reference Pressure

The implementation should use these references as pressure, not as frameworks
to copy.

- SimPy and ns-3 support a deterministic discrete-event agenda with explicit
  same-time tie-breaking. This maps to `WakeupOrderKey`.
- ns-3 also demonstrates cancel/remove semantics, but the local engine should
  not use delayed callbacks as mutation authority.
- CDDA activities support the need for saveable, interruptible, resumable
  long-running work. Adapt this into explicit `ProcessInstance` state rather
  than subclasses or callback logic.
- Temporal-style durable workflows reinforce replay-safe history and recorded
  non-determinism, while also showing why saved coroutine stacks are the wrong
  persistence unit here.
- MLIR, rustc, and Salsa reinforce typed stage contracts, query invalidation,
  stable keys, and narrow failure surfaces. Do not turn Phase 5 into a generic
  pass framework.
- Event sourcing and CQRS support accepted envelopes and read/write
  separation, but the world model remains materialized state plus history, not
  a pure event-sourced store.
- Rust privacy and API guidelines support private fields, newtypes, narrow
  constructors, and conservative public surfaces as the practical authority
  tools available in the current crate layout.

## Scope

Implement in `crates/world-model`:

- runtime-control update history with model-assigned append cursors;
- accepted runtime-control update package and model receiver;
- runtime-control current-state payload records for:
  - process instances;
  - reservations;
  - scheduled wakeups;
  - activity records only if needed;
  - RNG records only when required by declared replay/provenance;
- atomic application for:
  - accepted update envelope append;
  - process upsert/status transition;
  - reservation upsert/status transition;
  - wakeup schedule/consume/cancel/skip;
  - optional activity link/status update;
  - runtime-control invalidation;
- query helpers for:
  - runtime-control record by kind/id;
  - process by id;
  - reservation by id;
  - scheduled wakeup by id;
  - due active wakeups in canonical order;
- a due-wakeup secondary index target:
  - key: `(WakeupOrderKey, ScheduledWakeupId)`;
  - value: `ScheduledWakeupId`;
  - scanning the record map is acceptable only as a short first step if tests
    prove the same semantics and the index target remains documented;
- storage validation for:
  - store key / derived record kind mismatch;
  - duplicate active records;
  - invalid status transitions;
  - stale wakeup consumption/cancellation/skip;
  - missing referenced process/activity/reservation/wakeup;
  - invalid runtime-control invalidation;
  - current-state/history partial application.

Implement in `crates/world-runtime`:

- runtime-control update builder and gate;
- runtime-control id issuer ownership;
- scheduler due-wakeup selection and drain;
- drain budget and stop outcomes;
- wakeup scheduling, cancellation, consumption, and skipping;
- minimal process runtime;
- minimal reservation acquire/release/cancel/conflict logic;
- transaction-coupled runtime-control staging in `CausalTransactionBuilder`;
- hard commit finalization and model application for hard plus control changes;
- focused tests for model receiver, scheduler ordering, drain behavior,
  process lifecycle, reservation conflict, and hard/control atomicity.

Small `world-core` additions are allowed only when a value clearly belongs
below both model and runtime. Avoid adding a new id unless the model cursor
approach fails.

Small `world-defs` additions are allowed only for validating current checked
process definitions against stored process state. Do not add parser, DSL,
policy interpreter, or standard process library code.

## Explicit Non-Scope

Do not implement:

- standard world primitive library split;
- `PrimitiveSemanticsRegistry` or standard primitive semantics installers;
- actor context projection;
- observation projection;
- semantic appraisal, intent scoring, or final actor policy selection;
- complete activity/intent framework;
- process policy interpreter;
- full game-system process vocabulary;
- full reservation/resource conflict model;
- passive physical simulation beyond minimal process wakeup support;
- reaction runtime that listens to events and mutates immediately;
- scripting, Wasm, plugin loading, ECS system graph, async task runtime, or
  callback scheduler;
- persistence backend, save-file format, serde policy, migrations, or durable
  schema compatibility;
- deterministic command replay beyond the data needed by declared replay
  levels;
- player UI turn shell;
- AI-agent input loop.

## Target Module Shape

Suggested `world-model` shape:

```text
crates/world-model/src/
  lib.rs
  error.rs
  model.rs
  store.rs
  relations.rs
  history.rs
  invalidation.rs
  commit.rs
  runtime_control.rs
  runtime_control_commit.rs
  records.rs
  query.rs
  tests.rs
```

`runtime_control.rs` should own current-state records and indexes.
`runtime_control_commit.rs` should own accepted update packages, apply plans,
and application output if that keeps the model boundary readable.

Suggested `world-runtime` shape:

```text
crates/world-runtime/src/
  lib.rs
  error.rs
  request.rs
  outcome.rs
  runtime.rs
  transaction.rs
  effects.rs
  commit.rs
  runtime_control.rs
  scheduler.rs
  process.rs
  reservation.rs
  tests.rs
```

Add `activity.rs` only if an activity link becomes necessary. Do not add
`reaction.rs`, `context.rs`, `semantic.rs`, `standard.rs`, `plugin.rs`, or
`system.rs` in Phase 5.

## Required Type Sketches

These sketches are implementation anchors, not final public APIs.

### Runtime-Control Update History

```rust
pub struct RuntimeControlUpdateHeader {
    source: RuntimeControlSource,
    occurred_at: SimulationTime,
    replay_level: ReplayLevel,
    provenance: Option<ProvenanceKey>,
}

pub struct RuntimeControlUpdateRecord {
    header: RuntimeControlUpdateHeader,
    changed: Vec<RuntimeControlRecordKind>,
}

pub struct RuntimeControlApplication {
    update_cursor: StoreCursor,
    changed_records: Vec<RuntimeControlRecordKind>,
    invalidation: DerivedViewInvalidationReport,
}
```

The store assigns `StoreCursor` when the accepted update is appended.

### Accepted Runtime-Control Update

```rust
pub struct AcceptedRuntimeControlUpdate {
    header: RuntimeControlUpdateHeader,
    changes: Vec<RuntimeControlChange>,
    invalidation: InvalidationPackage,
}

pub enum RuntimeControlChange {
    PutRecord(RuntimeControlRecord),
    TransitionWakeup {
        wakeup: ScheduledWakeupId,
        transition: WakeupTerminalTransition,
    },
}

pub enum WakeupTerminalTransition {
    Consumed {
        at: SimulationTime,
        reason: WakeupConsumptionReason,
    },
    Canceled {
        at: SimulationTime,
        reason: WakeupCancellationReason,
    },
    Skipped {
        at: SimulationTime,
        reason: StaleWakeupReason,
    },
}
```

`RuntimeControlUpdateHeader` mirrors the Phase 4 transaction-header pattern.
It keeps source, time, replay level, and provenance from being repeated across
accepted update and history record types.

Add activity payload support only if activity records are introduced.

### Runtime-Control Records

```rust
pub struct RuntimeControlRecord {
    payload: RuntimeControlRecordPayload,
    updated_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}

pub enum RuntimeControlRecordPayload {
    Process(ProcessInstanceRecord),
    Reservation(ReservationRecord),
    ScheduledWakeup(ScheduledWakeupRecord),
}

impl RuntimeControlRecord {
    pub fn kind(&self) -> RuntimeControlRecordKind {
        self.payload.kind()
    }
}

impl RuntimeControlRecordPayload {
    pub fn kind(&self) -> RuntimeControlRecordKind {
        match self {
            Self::Process(record) => RuntimeControlRecordKind::Process(record.id()),
            Self::Reservation(record) => RuntimeControlRecordKind::Reservation(record.id()),
            Self::ScheduledWakeup(record) => {
                RuntimeControlRecordKind::ScheduledWakeup(record.id())
            }
        }
    }
}
```

Do not store `kind` beside `payload` inside the record. Derive it from the
payload so the type shape cannot represent `RuntimeControlRecordKind::Process`
with a wakeup payload. Store application should still validate that the map key
matches `record.kind()`.

```text
store key == RuntimeControlRecord::kind()
```

### Scheduled Wakeup

```rust
pub struct ScheduledWakeupRecord {
    id: ScheduledWakeupId,
    order: WakeupOrderKey,
    target: WakeupTarget,
    status: ScheduledWakeupStatus,
    source: RuntimeControlSource,
    provenance: Option<ProvenanceKey>,
}

pub enum WakeupTarget {
    HostInputOpportunity,
    Process(ProcessInstanceId),
    PassiveProcess(ProcessInstanceId),
}

pub enum ScheduledWakeupStatus {
    Scheduled,
    Consumed {
        at: SimulationTime,
        reason: WakeupConsumptionReason,
    },
    Canceled {
        at: SimulationTime,
        reason: WakeupCancellationReason,
    },
    Skipped {
        at: SimulationTime,
        reason: StaleWakeupReason,
    },
}
```

Terminal wakeup statuses carry their time and reason. This keeps accepted
provenance on the durable record instead of scattering it across update-only
values.

Actor-specific activation can be added later when actor context exists. If an
actor id is needed for a narrow test, add it deliberately without pulling in
actor context projection.

### Scheduler Drain

```rust
pub struct Scheduler {
    default_budget: DrainBudget,
}

pub struct DrainRequest {
    until: Option<SimulationTime>,
    budget: DrainBudget,
}

pub struct DrainReport {
    outcome: DrainOutcome,
    processed: Vec<ProcessedWakeup>,
}

pub enum DrainOutcome {
    Quiescent,
    InputOpportunity,
    BoundaryReached,
    BudgetExceeded,
    MandatoryPrompt,
}

pub struct ProcessedWakeup {
    wakeup: ScheduledWakeupId,
    result: WakeupDrainResult,
}

pub enum WakeupDrainResult {
    Consumed,
    Blocked(BlockedReason),
    Canceled,
    Rescheduled,
    Completed,
    Failed(ProcessFailureReason),
    Skipped(StaleWakeupReason),
}
```

`InputOpportunity` and `MandatoryPrompt` are host-facing stop reasons.

### Process Instance

```rust
pub struct ProcessInstanceRecord {
    id: ProcessInstanceId,
    definition: DefinitionId,
    owner: Option<EntityId>,
    roles: Vec<ProcessRoleBinding>,
    resolution: ResolutionTier,
    lifecycle: ProcessLifecycle,
    progress: ProcessProgress,
    state: ProcessStateSnapshot,
    reservations: Vec<ReservationId>,
    version: VersionAnchor,
    provenance: Option<ProvenanceKey>,
}

pub enum ProcessLifecycle {
    Created,
    Scheduled {
        wakeup: ScheduledWakeupId,
    },
    Waiting {
        condition: WaitCondition,
    },
    Advancing,
    Paused {
        reason: PauseReason,
    },
    Interrupted {
        reason: InterruptReason,
    },
    Completed,
    Failed {
        reason: ProcessFailureReason,
    },
    Abandoned,
}

pub struct ProcessWork(u64);

pub enum ProcessProgress {
    Bounded {
        completed: ProcessWork,
        required: ProcessWork,
    },
    OpenEnded {
        completed: ProcessWork,
    },
}
```

`ProcessLifecycle` carries the data that is required for each lifecycle state.
Avoid a `status` field plus `wait: Option<_>` or `wakeup: Option<_>` because
that allows invalid combinations such as `Waiting` with no wait condition or
`Completed` with an active wakeup.

For `ProcessStateSnapshot`, use the smallest closed value model that supports
the first tests. If schema validation is added, keep it limited to simple
built-in value labels and defer policy execution.

Terminal lifecycles do not advance through ordinary ticks.

### Process Transition

```rust
pub struct ProcessTick {
    process: ProcessInstanceId,
    occurred_at: SimulationTime,
    source_wakeup: Option<ScheduledWakeupId>,
    provenance: Option<ProvenanceKey>,
}

pub enum ProcessTransition {
    Started(ProcessInstanceRecord),
    Scheduled {
        process: ProcessInstanceId,
        wakeup: ScheduledWakeupRecord,
    },
    Advanced {
        process: ProcessInstanceRecord,
        next_wakeup: Option<ScheduledWakeupRecord>,
    },
    Waiting {
        process: ProcessInstanceRecord,
        condition: WaitCondition,
    },
    Paused(ProcessInstanceRecord),
    Interrupted(ProcessInstanceRecord, InterruptReason),
    Resumed(ProcessInstanceRecord),
    Completed(ProcessInstanceRecord),
    Failed(ProcessInstanceRecord, ProcessFailureReason),
    Abandoned(ProcessInstanceRecord),
    ProducedRuntimeRequest(RuntimeRequest),
}
```

Phase 5 may store interruption reasons, but observation-relative interrupt
policy is out of scope.

### Reservations

```rust
pub struct ReservationRecord {
    id: ReservationId,
    holder: ReservationHolder,
    target: ReservationTarget,
    state: ReservationState,
    provenance: Option<ProvenanceKey>,
}

pub enum ReservationState {
    Held {
        acquired_at: SimulationTime,
    },
    Released {
        acquired_at: SimulationTime,
        released_at: SimulationTime,
    },
    Canceled {
        acquired_at: SimulationTime,
        canceled_at: SimulationTime,
        reason: ReservationCancelReason,
    },
}

pub enum ReservationTransition {
    Acquired(ReservationRecord),
    Released {
        reservation: ReservationId,
        released_at: SimulationTime,
    },
    Canceled {
        reservation: ReservationId,
        canceled_at: SimulationTime,
        reason: ReservationCancelReason,
    },
    Conflict {
        requested: ReservationTarget,
        blocker: ReservationId,
    },
}
```

Acquire/release that gates executable work is transaction-coupled by default.

## Optional Type Sketches

### Activity Link

Add only if a Phase 5 test needs actor-facing activity linkage.

```rust
pub struct ActivityRecord {
    id: ActivityId,
    actor: Option<EntityId>,
    process: Option<ProcessInstanceId>,
    state: ActivityState,
    provenance: Option<ProvenanceKey>,
}

pub enum ActivityState {
    Selected,
    Active,
    Paused {
        reason: PauseReason,
    },
    Interrupted {
        reason: InterruptReason,
    },
    Completed,
    Failed {
        reason: ProcessFailureReason,
    },
    Abandoned,
}
```

Keep it a link/state record. Do not implement intent planning or full
activity lifecycle semantics.

### RNG Records

Add only for deterministic replay or committed RNG provenance.

```rust
pub struct RngDrawRecord {
    id: RngDrawId,
    stream: RngStreamId,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}
```

Audit-only scheduler/process tests should not need RNG.

## Transaction-Coupled Integration

Extend the Phase 4 transaction builder conceptually:

```rust
pub(crate) struct CausalTransactionBuilder {
    // existing hard transaction fields
    control_changes: Vec<RuntimeControlChange>,
}

impl EffectStager<'_, '_> {
    pub(crate) fn stage_runtime_control(
        &mut self,
        change: RuntimeControlChange,
    ) -> Result<(), RuntimeError>;
}
```

Finalization produces one accepted hard commit:

```rust
pub struct AcceptedHardCommit {
    transaction: TransactionCommit,
    events: Vec<EventCommit>,
    changes: Vec<HardStateChange>,
    control_changes: Vec<RuntimeControlChange>,
    invalidation: InvalidationPackage,
}
```

The model preflights hard changes and control changes before mutating either
store family. If any part fails, no transaction history, event history, hard
state, runtime-control state, update history, or invalidation is published.

## Design Patterns To Use

- **Accepted package receiver:** model applies accepted runtime-control
  packages as one operation.
- **Update builder:** runtime collects control changes, source, replay level,
  provenance, and invalidation before model application.
- **Capability-based staging:** process/effect code stages schedule,
  reservation, and process changes through narrow APIs.
- **Discrete-event agenda:** ordering is explicit through
  `(time, phase, priority, sequence)`.
- **Serializable state machine:** process, reservation, and wakeup records
  store state, not executable stacks.
- **State-carrying enums:** lifecycle/status enums carry the data required by
  that state instead of pairing a status enum with optional side fields.
- **Validate-then-apply:** model receivers preflight all changes before
  mutating current-state indexes or history.
- **Outcome/error split:** lifecycle states are domain transitions; corrupted
  data, exhausted issuers, missing definitions, and impossible engine states
  are errors.
- **Ordered index for due work:** use an explicit due-wakeup index or a
  semantics-preserving scan while building toward the index.
- **Closed runtime semantics first:** no public extension trait until there is
  a concrete extension boundary.

Avoid:

- `System::run(&mut WorldModel)` style APIs;
- scheduler callbacks or boxed closures;
- saved async/coroutine stacks;
- public raw runtime-control mutation;
- hidden concrete `ActionRequest` spam for abstract process progress;
- event listeners that mutate the source transaction;
- generic field-patch runtime-control updates;
- global deterministic replay claims without recorded inputs;
- broad public traits.

## Implementation Sequence

### 1. Model Accepted Runtime-Control Receiver

Build the model-side authority receiver first.

Tasks:

- add accepted update envelope and update history with model-assigned cursors;
- expand runtime-control current-state records for process, reservation, and
  wakeup payloads;
- add runtime-control change application with preflight planning;
- add runtime-control invalidation validation;
- add store key / derived record kind consistency checks;
- keep direct mutators private or test-only.

Tests:

- accepted update inserts process/reservation/wakeup records;
- duplicate or mismatched store key / record kind is rejected before mutation;
- invalidation must include runtime-control authority and store family;
- multi-change update is atomic when one change is invalid;
- update history cursor is assigned by model application;
- read APIs expose current records without write authority.

### 2. Runtime-Control Gate And Issuers

Build the runtime-side accepted update path.

Tasks:

- add runtime-control id issuers to the runtime facade;
- add `RuntimeControlUpdateBuilder`;
- validate source, time, replay level, changes, and invalidation;
- produce `AcceptedRuntimeControlUpdate`;
- apply control-only updates through `WorldModel::apply_runtime_control_update`;
- ensure failed preflight does not publish records or consume model cursors.

Tests:

- valid control-only update applies;
- missing invalidation fails;
- gate cannot stage hard changes;
- id issuers are owned by runtime, not model;
- model cursor advances only on accepted application.

### 3. Wakeup Records And Ordering

Implement durable scheduled wakeups.

Tasks:

- add `ScheduledWakeupRecord`, target, and status;
- implement schedule, consume, cancel, and skip changes;
- add active due-wakeup query in `WakeupOrderKey` order;
- build or document the secondary index target;
- preserve provenance for all wakeup status changes.

Tests:

- due wakeups order by time, phase, priority, sequence, id;
- canceled/consumed/skipped wakeups are not returned as active due work;
- stale consume/cancel/skip attempts fail clearly;
- same-time ordering is independent of map/hash iteration;
- skipped stale wakeup is backed by accepted state.

### 4. Scheduler Drain

Implement bounded drain over due wakeups.

Tasks:

- add scheduler drain owner;
- add `DrainRequest`, `DrainBudget`, `DrainOutcome`, and `DrainReport`;
- stop on quiescence, host input opportunity, boundary, mandatory prompt, or
  budget exhaustion;
- record a result for every processed wakeup;
- avoid protagonist/UI-specific behavior.

Tests:

- empty agenda drains to `Quiescent`;
- generic host input target returns `InputOpportunity`;
- budget exhaustion stops without hidden mutation;
- stale process wakeup is accepted as skipped;
- drain report preserves processed order.

### 5. Minimal Process Runtime

Implement explicit process state and tiny progress semantics.

Tasks:

- add process records, lifecycle, progress, and state snapshot;
- add process tick and transition values;
- validate process definition existence and resolution support;
- advance progress with a tiny built-in rule;
- reschedule incomplete processes;
- complete processes when progress reaches required work;
- reject ordinary ticks for terminal lifecycles;
- keep policy key interpretation out of scope.

Tests:

- process starts and schedules a wakeup;
- tick advances and reschedules incomplete process;
- tick completes process when progress is sufficient;
- paused/interrupted/completed/failed/abandoned process does not advance;
- unsupported resolution or missing definition fails clearly;
- abstract process progress does not synthesize hidden action spam.

### 6. Minimal Reservation Boundary

Implement one narrow reservation model.

Tasks:

- add reservation holder, target, state, and transition values;
- implement one active exclusive reservation per target;
- route acquire/release that gates executable work through
  transaction-coupled changes;
- allow control-only cancel/expiry/cleanup where no hard outcome is gated;
- preserve conflict explanations.

Tests:

- transaction-coupled acquire creates held reservation only with committed hard
  outcome;
- exclusive duplicate reports conflict with blocker;
- release removes active conflict only when accepted;
- cancel/cleanup can use control-only lane;
- hard commit failure leaves reservation state unchanged.

### 7. Hard Commit Integration

Wire transaction-coupled runtime-control changes into the Phase 4 hard commit
waist.

Tasks:

- add control changes to `CausalTransactionBuilder`;
- add staging capability methods;
- update `CommitFinalizer`;
- extend `AcceptedHardCommit`;
- update `WorldModel::plan_hard_commit` and apply logic to preflight and apply
  hard/control changes atomically;
- ensure hard commit invalidation marks both hard and runtime-control
  authority when both are changed.

Tests:

- hard state and control state apply together;
- model preflight failure leaves both hard and control state unchanged;
- scheduled follow-up exists only after commit;
- process progress tied to a hard outcome is atomic with transaction history;
- invalidation reports all changed authority and store families.

### 8. Focused Integration Scenarios

Add end-to-end tests without pulling future layers forward.

Scenarios:

- schedule process wakeup, drain, advance, and reschedule;
- drain a process to completion;
- drain to generic host input opportunity;
- skip stale wakeup after target process is terminal;
- commit hard event plus scheduled process follow-up atomically;
- reservation conflict blocks the requested runtime transition with a clear
  reason.

## API Rules

- Public names describe domain meaning, not implementation stages.
- Public enums likely to grow should be `#[non_exhaustive]`.
- Direct runtime-control store mutation remains private or test-only.
- Runtime-control writes go through accepted updates or transaction staging.
- Scheduler entries are typed targets, not callbacks.
- Runtime code may read model state but cannot bypass accepted receivers.
- Use `SimulationTime` and `SimulationDuration`, never wall-clock time or
  floats, for scheduler/runtime-control time.
- Runtime-control invalidation is part of accepted publication.
- Domain lifecycle outcomes are not infrastructure errors.
- Keep values crate-private until cross-crate use is real.

## Invariants To Protect

- No active wakeup is processed twice.
- No due wakeup disappears without accepted consume/cancel/skip provenance.
- Same-time order is inspectable and deterministic.
- Drain cannot spin forever on zero-time work.
- Process terminal states do not advance through ordinary ticks.
- Runtime-control updates cannot mutate hard stores.
- Transaction-coupled control changes are atomic with hard commits.
- Reservation conflicts preserve blocker identity.
- Store key / derived record kind mismatches are rejected before mutation.
- Invalidation marks every changed authority and store family.
- Failed preflight publishes no partial history, state, or invalidation.

## Verification Plan

After meaningful Rust changes, run:

```bash
cargo fmt --all --check
cargo check --workspace
cargo test -p world-model --all-targets
cargo test -p world-runtime --all-targets
cargo clippy --workspace --all-targets
git diff --check
```

Before committing the complete phase, run:

```bash
cargo test --workspace
```

For this planning document alone, `git diff --check` and a trailing-whitespace
scan are sufficient.

## Review Checklist

Before marking Phase 5 complete:

- `WorldModel` has one accepted runtime-control update receiver.
- Runtime-control update history uses model-assigned cursors.
- Runtime-control store has no public raw mutator.
- Wakeups are durable typed targets.
- Due wakeup ordering uses `WakeupOrderKey`.
- Skipped wakeups have accepted provenance.
- Drain outcomes explain why work stopped.
- Process instances are explicit saved state.
- Process runtime uses tiny built-in progress, not policy interpretation.
- Reservation acquire/release that gates work is transaction-coupled.
- Hard/control atomicity is tested.
- Activity and RNG stayed optional unless their entry criteria were met.
- No actor context, semantic appraisal, standard primitive library, parser,
  scripting, UI, or AI-agent loop leaked into the phase.
