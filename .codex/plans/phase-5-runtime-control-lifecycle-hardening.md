# Phase 5 Follow-Up Plan: Runtime-Control Lifecycle Hardening

## Status

Proposed implementation plan.

This plan addresses the Phase 5 review findings after the initial runtime
control, time, and process implementation.

It deliberately excludes the accepted-package forgeability issue. Public
constructor and crate-boundary authority hardening requires a separate
architecture decision. This plan focuses on the problems that can be fixed
inside the current crate direction:

- runtime-control changes should be domain transitions, not raw store patches;
- model-side validation should prove runtime-control state-machine invariants;
- runtime-owned ids and scheduler ordering must survive restart/load;
- process wakeups must connect cleanly to the causal transaction path;
- reservations and host input wakeups need durable, explicit lifecycle
  semantics;
- the API, module shape, and tests should reflect the durable runtime-control
  model without half-implemented surfaces.

## Purpose

Turn the first Phase 5 implementation into a durable runtime-control substrate
that can be safely built on by later standard primitive, actor-context, and
semantic decision phases.

The target shape is:

```text
control-only runtime transition
  -> typed RuntimeControlChange
  -> model verifier
  -> AcceptedRuntimeControlUpdate receiver
  -> current runtime-control state + update history + invalidation

hard outcome with coupled control state
  -> CausalRuntime::execute
  -> EffectStager / transaction builder
  -> hard state + event history + RuntimeControlChange set
  -> one atomic WorldModel application
```

The goal is not to make process semantics expressive yet. The goal is to make
the minimal Phase 5 semantics true, typed, replayable at the declared level,
and hard to accidentally misuse.

## Source Inputs

Primary local documents:

- `docs/architecture/implementation-plan.md`
- `docs/architecture/implementation-execution-contract.md`
- `docs/architecture/runtime-pipeline.md`
- `docs/architecture/crates.md`
- `docs/design/causal-runtime.md`
- `docs/design/time-model.md`
- `docs/design/typed-effect-primitives.md`
- `docs/design/standard-world-library.md`
- `.codex/plans/phase-5-runtime-control-time-and-process.md`
- `.codex/research/phase-4-5-runtime-research.md`

External reference pressure:

- [MLIR defining dialects](https://mlir.llvm.org/docs/DefiningDialects/):
  operations and types carry verifier hooks. Apply this as a typed transition
  IR plus model verifier, not as a generic pass framework.
- [SimPy time and scheduling](https://simpy.readthedocs.io/en/4.0.2/topical_guides/time_and_scheduling.html):
  deterministic event processing uses explicit same-time ordering. Apply this
  to scheduler-owned wakeup sequence assignment.
- [ns-3 events and simulator](https://www.nsnam.org/docs/manual/html/events.html):
  simulated time is separate from wall-clock time, and cancellation/removal have
  explicit scheduler meanings. Apply this to wakeup terminal states and host
  input acknowledgement.
- [Erlang `gen_statem`](https://www.erlang.org/doc/system/statem.html):
  state machines are event-driven transitions from state and event to actions
  plus next state. Apply this to process, reservation, and wakeup transition
  matrices.
- [Temporal history-service architecture](https://github.com/temporalio/temporal/blob/main/docs/architecture/history-service.md):
  mutable workflow state, history events, and generated tasks are coupled as
  one transition. Apply this to hard commits with runtime-control changes.
- [Rust API Guidelines: type safety](https://rust-lang.github.io/api-guidelines/type-safety.html):
  encode invariants with concrete types and narrow APIs. Apply this to request
  shapes, id seeds, transition enums, and test-only raw counter access.

These references are pressure, not dependencies. Do not add a scheduler,
workflow, state-machine, or compiler framework dependency.

## Non-Scope

Do not solve in this plan:

- cross-crate compile-time prevention of forged accepted packages;
- a new kernel crate or crate-boundary redesign;
- a persistence backend, save-file schema, serde policy, or migration policy;
- a full process policy interpreter;
- the standard world primitive library split;
- actor context projection, protagonist UI turn shell, observation projection,
  semantic appraisal, or AI-agent input loop;
- complete activity, RNG, reaction, passive physics, or resource-lock systems.

This plan may narrow public APIs that are not needed, but it must not claim to
solve the larger accepted-package authority issue.

## Design Direction

### 1. Replace Store Patch Semantics With Typed Transitions

`RuntimeControlChange::PutRecord(RuntimeControlRecord)` is too broad for a
durable control-state IR. It allows callers and runtime code to think in terms
of storage replacement rather than lifecycle transitions.

Move to typed changes whose variants encode domain intent:

```rust
pub enum RuntimeControlChange {
    CreateProcess(ProcessInstanceRecord),
    UpdateProcess(ProcessInstanceRecord),
    ScheduleWakeup(ScheduledWakeupRecord),
    TransitionWakeup {
        wakeup: ScheduledWakeupId,
        transition: WakeupTerminalTransition,
    },
    AcquireReservation(ReservationRecord),
    TransitionReservation(ReservationTransition),
}
```

`UpdateProcess` is acceptable as a first step only if model validation proves
the transition from current lifecycle/progress to new lifecycle/progress. If
that remains too wide during implementation, split it into narrower variants
such as `AdvanceProcess`, `WaitProcess`, `PauseProcess`, `InterruptProcess`,
`ResumeProcess`, `CompleteProcess`, `FailProcess`, and `AbandonProcess`.

Storage upsert should become an internal materialization detail of the model
apply plan. Runtime code should construct transitions, not store writes.

### 2. Add A Model-Side Verifier Layer

Model application should validate in this order:

```text
accepted envelope validation
  -> structural change validation
  -> current-state transition validation
  -> cross-record validation
  -> apply plan construction
  -> atomic application
```

Required invariants:

- create process rejects an existing process id;
- update process rejects a missing process id;
- terminal process states cannot be reopened by a generic update;
- scheduled process records must have a matching scheduled wakeup when the
  transition says work is scheduled;
- wakeup schedule rejects an existing wakeup id;
- wakeup terminal transition is only allowed from `Scheduled`;
- terminal wakeup cannot be reactivated;
- reservation acquire rejects an existing reservation id;
- only one held exclusive reservation may exist per target;
- reservation release/cancel requires the current stored reservation to be held;
- reservation holder and target are immutable across transition;
- stale caller-owned reservation clones cannot release current model state;
- transaction-coupled runtime-control validation failure leaves hard state,
  event history, runtime-control state, and invalidation unchanged.

The verifier may live inside `RuntimeControlStore` at first, but it should be
structured as small validation functions rather than one broad `plan_changes`
body.

### 3. Make Runtime Id And Order Issuance Durable

`CausalRuntime` should still own runtime-control issuers. `WorldModel` should
not become an id allocator.

Add a seed shape derived from current model state:

```rust
pub struct RuntimeControlIdSeed {
    next_process: u64,
    next_reservation: u64,
    next_wakeup: u64,
    next_wakeup_sequence: u64,
}
```

`RuntimeControlStore` should expose read-only seed derivation by scanning
current process, reservation, and scheduled wakeup records. The runtime should
construct `RuntimeControlIds` with `*_Issuer::starting_at`.

The scheduler should own wakeup sequence assignment. Replace request shapes
that accept a complete `WakeupOrderKey` with a request that omits `sequence`:

```rust
pub struct WakeupScheduleKey {
    time: SimulationTime,
    phase: u16,
    priority: i32,
}

pub struct ScheduleWakeupRequest {
    schedule: WakeupScheduleKey,
    target: WakeupTarget,
    submitted_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
}
```

The runtime assigns:

```text
WakeupOrderKey(time, phase, priority, runtime_owned_sequence)
```

Do not expose raw `next_*_id_value()` as normal public API. Tests should inspect
issued ids through outcomes, or use explicitly test-scoped helpers.

### 4. Split Control-Only Scheduler Work From Causal Process Work

Scheduler drain should distinguish maintenance/control transitions from process
work that produces hard outcomes.

Control-only lane:

- cancel wakeup;
- skip stale wakeup;
- host input opportunity presentation and acknowledgement;
- administrative pause/interruption/abandonment that does not validate or
  unlock a hard outcome.

Causal lane:

- process tick that advances hard world state;
- process completion/failure that must emit hard evidence;
- reservation acquire/release that gates a hard outcome;
- future process scheduling that must be atomic with a hard outcome.

The drain path should produce one of these internal dispatch results:

```rust
enum WakeupDispatch {
    ControlOnly(AcceptedRuntimeControlUpdate),
    CausalProcess(ProcessTickRequest),
    HostInputOpportunity(ScheduledWakeupId),
}
```

For Phase 5, the process tick may remain a minimal built-in progress rule, but
when it claims a causal outcome it should pass through `CausalRuntime` commit
machinery and produce event history plus transaction-coupled control changes.

If a process transition is explicitly control-only, name and test it as such.
Do not let control-only progress accidentally stand in for the documented
`ProcessTick -> CausalTransaction` path.

### 5. Give Host Input Wakeups An Explicit Acknowledgement Path

`DrainOutcome::InputOpportunity { wakeup }` should be a host-facing stop reason,
not a hidden consumption.

Add a runtime method with accepted provenance, for example:

```rust
pub fn acknowledge_host_input_wakeup(
    &mut self,
    model: &mut WorldModel,
    wakeup: ScheduledWakeupId,
    acknowledged_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
) -> Result<RuntimeControlApplication, RuntimeError>
```

The method should terminal-transition the wakeup, most likely using
`WakeupTerminalTransition::Consumed`. `cancel_wakeup` remains the withdrawal or
discard path.

Drain budget should count processed work. Merely discovering a due host input
opportunity should not consume budget or cause the wakeup to disappear.

### 6. Move Reservations Into Staging Capabilities

Reservation acquire/release that gates execution should be staged through the
transaction path, not handed around as a loose `RuntimeControlChange`.

Add narrow staging methods:

```rust
impl EffectStager<'_, '_> {
    fn acquire_reservation(
        &mut self,
        request: AcquireReservationRequest,
        ids: &mut RuntimeControlIds,
    ) -> Result<ReservationId, RuntimeError>;

    fn release_reservation(
        &mut self,
        reservation: ReservationId,
        released_at: SimulationTime,
    ) -> Result<(), RuntimeError>;
}
```

The exact signature may change to avoid borrow issues, but the ownership rule
should not: effect semantics stage reservations through runtime capabilities.

Add at least one real `CausalRuntime::execute` path that stages a reservation
change and commits it atomically with hard state. A minimal built-in effect is
acceptable for this phase if it is explicitly scoped:

- `AcquireReservation` requires `StagePermission::AcquireReservation`;
- validation checks current model state and same-transaction staged conflicts;
- staging pushes a typed `RuntimeControlChange::AcquireReservation`;
- `CommitFinalizer` carries it into the accepted hard commit;
- model hard-commit preflight validates the control change before publishing
  any hard state.

Do not implement a full reservation/resource policy system here.

### 7. Remove Or Defer Half-Implemented Runtime-Control Families

If `Activity`, `RngStream`, or `RngDraw` runtime-control record kinds do not
have payload records, apply semantics, and tests in this phase, remove them
from the current public record-kind surface or mark them as unsupported through
an explicit non-public placeholder.

Do not expose public enum variants for state families that cannot be stored or
validated yet.

### 8. Split The Largest Runtime-Control File Only Where It Reduces Risk

The current model runtime-control file mixes record definitions, accepted
update envelopes, store state, indexes, verifier logic, and apply logic.

After typed transitions are introduced, split only if the code has stabilized
enough to avoid churn. Suggested shape:

```text
crates/world-model/src/runtime_control/
  mod.rs
  record.rs
  change.rs
  store.rs
  update.rs
  validate.rs
```

If Rust module churn becomes a distraction, keep a single file during the
semantic refactor and split immediately after tests pass.

Runtime-side modules can stay close to the current Phase 5 shape:

```text
crates/world-runtime/src/
  runtime_control.rs
  scheduler.rs
  process.rs
  reservation.rs
  transaction.rs
  effects.rs
  builtin.rs
```

## Implementation Order

1. Introduce typed runtime-control changes and model verifier structure.

   Replace runtime construction of `PutRecord` with domain transition variants.
   Keep behavior equivalent where possible, but make invalid replacement paths
   impossible or rejected.

2. Add runtime-control id seed derivation and runtime hydration.

   Derive next process/reservation/wakeup ids and wakeup sequence from the
   model. Add a runtime constructor that consumes the seed or derives it from
   `&WorldModel`.

3. Move scheduler sequence assignment into runtime.

   Update `ScheduleWakeupRequest`, process start/resume scheduling, and tests
   so callers provide time/phase/priority and runtime assigns sequence.

4. Fix host input acknowledgement and drain budget semantics.

   Add an explicit acknowledgement method. Ensure zero-budget and host-input
   drain behavior are intentional and tested.

5. Strengthen process lifecycle validation.

   Add an explicit transition matrix for the minimal lifecycle. Ensure
   scheduled/waiting/paused/interrupted/terminal states cannot be replaced
   incoherently.

6. Move reservation release/acquire validation to current model state.

   Stop accepting stale caller-owned `ReservationRecord` clones as release
   authority. Stage transitions by id and validate against stored current
   state.

7. Connect one reservation staging path through `CausalRuntime::execute`.

   Thread runtime-control ids/staging capability into the effect interpreter
   carefully. Prove that hard state, event history, and runtime-control changes
   are applied atomically.

8. Clean unsupported runtime-control family surface and module shape.

   Remove or hide activity/RNG variants that are not backed by records. Split
   modules only after semantic tests are stable.

## Test Plan

Add focused tests before broad cleanup.

Model receiver and verifier:

- process create with existing id is rejected;
- process update for missing id is rejected;
- process terminal state cannot be reopened;
- wakeup schedule with existing id is rejected;
- terminal wakeup cannot be transitioned again or reactivated;
- reservation acquire with existing id is rejected;
- duplicate active reservation target is rejected;
- reservation release/cancel requires current held state;
- reservation holder and target are immutable;
- invalid transaction-coupled control change leaves hard state, event history,
  runtime-control state, and invalidation unchanged.

Runtime ids and ordering:

- runtime constructed from a model with existing control records issues ids
  after the maximum stored ids;
- same-time wakeups receive monotonic runtime-owned sequence values;
- drain order follows `(time, phase, priority, sequence, wakeup id)`;
- callers cannot choose scheduler sequence through public scheduling requests.

Scheduler and host input:

- stale process wakeup is skipped through an accepted transition and not
  processed again;
- host input wakeup remains due until acknowledged or canceled;
- host input with zero budget returns `InputOpportunity`;
- budget `1` with two due process wakeups processes one and reports
  `BudgetExceeded`;
- processed wakeups are reported in scheduler order.

Process and transaction coupling:

- causal process tick appends a transaction record and event history when it
  claims a hard outcome;
- process control-only transitions are explicitly named and do not pretend to
  be hard causal outcomes;
- process start/resume schedules wakeups with runtime-owned sequence;
- invalid lifecycle transitions are rejected.

Reservation and effect staging:

- reservation acquire staged by `CausalRuntime::execute` appears in the hard
  commit control changes;
- duplicate reservation acquire rejects before hard publication;
- reservation release by stale clone is impossible or rejected;
- model preflight failure during coupled reservation staging leaves all stores
  unchanged.

Verification:

```bash
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
git diff --check
```

## Completion Criteria

This follow-up is complete when:

- runtime-control current-state changes are expressed as typed transitions;
- model apply preflight rejects invalid process, wakeup, and reservation
  transitions before any partial application;
- runtime id and wakeup sequence issuance can be hydrated from model state;
- scheduler request APIs no longer let callers choose sequence;
- host input wakeups have explicit accepted acknowledgement or cancellation;
- reservation acquire/release can be staged through the causal transaction path
  when it gates hard execution;
- unsupported activity/RNG runtime-control surfaces are not exposed as if they
  were implemented;
- tests cover the behavior and atomicity risks listed above;
- the accepted-package forgeability issue remains clearly deferred rather than
  accidentally claimed as solved.
