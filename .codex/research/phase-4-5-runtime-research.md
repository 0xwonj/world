# Phase 4/5 Runtime Research

## Status

Research note for Phase 4 and Phase 5 preparation.

This is not a concrete implementation plan. It records the current local
architecture contract, external reference pressure, risks, and high-level
research conclusions to carry into the phase-specific `.codex/plans` documents.

## Scope

Phase 4:

- causal mutation waist
- `ActionRequest` binding and typed effect interpretation
- transaction staging and invariant checks
- accepted hard commit package
- `TransactionRecord`, `EventRecord`, and invalidation publication

Phase 5:

- runtime-control state
- integer simulation time
- durable scheduler wakeups
- process/activity progress
- reservation, interruption, resume, completion, and failure semantics

The shared concern is authority: which code is allowed to construct accepted
state changes, which code is only allowed to apply accepted packages, and what
records must exist for audit, replay, debugging, and derived-view invalidation.

## Local Contract Summary

The current architecture is coherent around one main idea:

```text
Hard world mutation flows through CausalTransaction.
Runtime-control mutation flows through runtime-control authority gates.
WorldModel stores and applies accepted records, but does not decide authority.
```

The relevant local documents already agree on most of the shape:

- `docs/architecture/implementation-plan.md` makes Phase 4 the hard mutation
  waist and Phase 5 the runtime-control/time/process layer above it.
- `docs/architecture/runtime-pipeline.md` defines the full pipeline as
  compiler-shaped but domain-named.
- `docs/design/causal-runtime.md` makes `CausalTransaction` the deepest hard
  mutation boundary.
- `docs/design/time-model.md` commits to integer simulation time and explicit
  scheduler ordering.
- `docs/design/world-model.md` keeps `WorldModel` read-first and store-owned,
  with commit gates still mostly conceptual.
- `crates/world-core` already has authority classes, replay levels, IDs,
  integer time, and wakeup ordering primitives.
- `crates/world-defs` already has checked action/process/effect definition
  records, stage permissions, event contracts, and replay-level declarations.
- `crates/world-model` already has read-only model stores, event history,
  runtime-control record envelopes, and derived-view invalidation metadata.
- `crates/world-runtime` is still effectively empty, so Phase 4 and Phase 5
  are the first places where runtime authority will become real code.

The existing code also preserves an important precondition: public model APIs
are read-oriented, and committed/runtime-control record constructors are not
public forge paths.

## Main Research Conclusions

### Keep Phase 4 Before Phase 5

The current ordering should stand. Process runtime, scheduler drains,
reservations, interruption, and long-running activity all need a way to publish
accepted state changes. If Phase 5 comes before Phase 4, process code will need
temporary write paths, and those paths will be hard to remove cleanly.

The external references support this sequencing:

- database transaction literature favors staging and atomic commit before
  public visibility;
- event-sourcing and CQRS guidance separates command handling from read models;
- discrete-event simulation frameworks separate scheduling order from domain
  state transition meaning;
- Temporal-style workflow replay separates durable command history from
  arbitrary side effects.

Phase 4 should therefore establish the hard commit waist first. Phase 5 should
then use that waist for process work that produces hard outcomes and use
runtime-control gates for durable control changes.

### Treat Wakeups As Work Selection, Not Mutation

SimPy and ns-3 both model a discrete-event clock that jumps to the next event
and processes same-time events deterministically. That maps well to
`ScheduledWakeup { time, phase, priority, sequence }`.

The part not to copy literally is "event as delayed function call." In this
engine, a wakeup should awaken a target such as an actor activation,
`ProcessTick`, or `ReactionRequest`. It should not be a callback that directly
mutates stores.

The safer high-level contract is:

```text
ScheduledWakeup
  -> due work envelope
  -> runtime validation / process transition / action binding
  -> accepted transaction or accepted runtime-control update
```

This keeps scheduler order visible while preserving the mutation boundary.

### Keep ProcessInstance As Explicit Serializable State

CDDA's activity model is the closest game reference: long activities can be
interrupted, resumed, serialized, advanced per turn, completed, or canceled.
The local architecture should keep that lesson but avoid copying its direct
subclass-heavy shape.

For this repo, the durable unit is `ProcessInstance`:

```text
ProcessInstance
  serializable state
  progress
  wait condition
  reservations
  interruption / resume / failure policy
```

Do not save coroutine stacks, async tasks, closures, or language call frames.
Temporal and Azure Durable Functions are useful negative references here: they
show the value of replay-safe workflow histories, but their stack-like workflow
authoring style is not the right persistence shape for this engine.

### Keep Materialized Stores Plus Event History

Pure event sourcing should not become the whole world model. The better fit is
materialized authoritative stores plus committed event and transaction history.

Event sourcing is still useful as an audit and replay lens:

- events should capture domain intent and hard facts, not just low-level diffs;
- event history can rebuild selected projections or support temporal debugging;
- versioned event envelopes and upcasting strategies matter once records become
  durable;
- external side effects must not run just because history is replayed.

For `world`, this argues for `EventHistoryStore` as committed hard evidence,
not as the only source for all present state and not as a generic generated
history store.

### Use Compiler Ideas As Discipline, Not As A Framework

MLIR, rustc queries, and Salsa support the repo's "compiler-shaped,
domain-named" thesis. The useful transfer is not a generic pass framework. The
useful transfer is contract discipline:

- each stage has typed inputs and outputs;
- each stage has allowed reads and writes;
- mutation is staged or accepted through an owner;
- analyses/projections declare invalidation dependencies;
- diagnostics and failure surfaces are explicit;
- deterministic replay claims require stable keys, ordering, and recorded
  non-determinism.

Do not introduce a universal `Pass` trait or a broad `System::run(&mut
WorldModel)` mechanism just because the design uses compiler language. The
local domain nouns are already stronger than a generic runtime abstraction.

### Runtime Control Needs Two Visible Lanes

The current Phase 5 adjustment is directionally correct: runtime control needs
both transaction-coupled updates and control-only accepted updates.

Transaction-coupled examples:

- process progress caused by a hard effect;
- reservation acquire/release that must be atomic with an action outcome;
- scheduler updates that must exist only if a hard outcome committed;
- RNG draw records tied to a committed outcome.

Control-only examples:

- selected or suggested intent;
- player automation continue/pause choice;
- process interruption decision that does not itself change hard truth;
- durable control state used to resume a future prompt or process.

This distinction is important enough to keep in the high-level architecture.
The concrete struct names can wait for the phase plan.

### Reservations Are The Main Open Classification Pressure

Reservation language is split across the current docs. Older research calls
reservation state hard causal state. Newer architecture separates
`AuthorityClass::RuntimeControl` from `Hard` and allows a control-only lane.

The research recommendation is to stop calling reservations "hard physical
truth" while still treating them as authority-bearing runtime state. They affect
future validation and replay, so they are not UI hints. But they are also not
physical facts like entity position or item containment.

High-level lane rule to carry forward:

```text
Reservation changes may be transaction-coupled when they must be atomic with
hard outcomes. Reservation-only changes may use runtime-control authority, but
only if the accepted envelope preserves ordering, provenance, invalidation, and
conflict explanation.
```

The exact split belongs in the Phase 5 implementation plan, not here.

### RuntimeControlStore Wording Needs Cleanup Later

`docs/design/world-model.md` currently says `RuntimeControlStore` owns "hard
runtime control state" and that runtime control is hard state because it affects
validation, replay, interruption, and future mutation. Current code and newer
pipeline docs represent runtime control as its own `AuthorityClass`.

The conceptual fix is:

```text
Runtime control is authority-bearing durable engine state.
It is not hard physical truth.
It can be transaction-coupled with hard commits when atomicity is required.
```

This is a high-level wording adjustment, not an implementation boundary change.

### Generated History Should Stay Out Of EventHistoryStore

Some older research still placed generated historical records in
`EventHistoryStore`. The newer world-model design is better: authored or
generated chronology belongs to `ChronologyStore` unless a scenario transition
materializes a hard outcome through causal runtime.

This matters for Phase 4 because `EventHistoryStore` should not become a
catch-all log. Its role should stay narrow:

```text
Committed hard transaction and event evidence.
```

Chronology can reference committed events as evidence, but chronology is not
itself a hard causal event.

### Keep Abstract Execution On The Same Runtime Axis

The current no-spam rule is correct:

```text
Abstract execution lowers through ProcessInstance / ProcessTick,
not hidden repeated concrete ActionRequest spam.
```

DEVS and multi-resolution simulation literature support modular hierarchical
models, but parallel/distributed simulation literature also warns that
independent logical processes introduce causality, lookahead, and rollback
problems. The repo should first treat multi-resolution as coarser state and
coarser wakeup granularity on the same authority-preserving time axis, not as a
parallel abstract runtime.

### Determinism Should Be Claimed Selectively

The local `ReplayLevel` enum is a good design pressure valve:

- `AuditOnly` can preserve explanation, ordering, and provenance without
  promising command replay.
- `EventRebuild` can rebuild selected materialized consequences from committed
  events.
- `DeterministicCommandReplay` should be reserved for paths that record all
  relevant ordering, RNG, version anchors, and external inputs.

Temporal's replay constraints are useful here: non-determinism must either be
kept outside replay or recorded through replay-safe APIs. For `world`, this
means wall-clock time, hash iteration order, thread timing, unrecorded RNG, and
AI/model calls cannot influence hard outcomes on paths that claim deterministic
command replay.

### Forge Resistance Is An API Shape, Not Just A Convention

Rust privacy and API design references reinforce the current direction:

- private fields preserve invariants;
- newtypes distinguish IDs and domains;
- sealed traits can prevent downstream implementations when a trait is needed;
- scoped constructors are more meaningful than broad public mutation handles;
- public fields and broad public traits make authority hard to recover later.

Rust does not have friend-crate visibility. Therefore the accepted-package
receiver boundary cannot rely on "runtime can construct it, model can apply it,
and no one else can see it" unless crate boundaries and module privacy are
chosen carefully. This is the main Rust-specific pressure on Phase 4.

The high-level contract remains:

```text
world-runtime constructs accepted packages.
world-model applies accepted packages through narrow receivers.
callers cannot forge committed records or split publication side effects.
```

The exact crate/module shape belongs in the Phase 4 implementation plan.

## Reference Synthesis

### Discrete-Event Simulation

[SimPy time and scheduling](https://simpy.readthedocs.io/en/4.1.1/topical_guides/time_and_scheduling.html)
uses a single-threaded deterministic event queue and FIFO tie-breaking for
same-time events. The local scheduler should keep explicit tie-breakers rather
than relying on collection order.

[ns-3 events and simulator](https://www.nsnam.org/docs/manual/html/events.html)
uses simulation time, an event queue, monotonic IDs for same-time ordering,
cancel/remove semantics, and integer-backed time. The local design should copy
the ordering discipline but not the direct delayed-callback mutation model.

DEVS literature supports modular hierarchical discrete-event models, especially
for multi-resolution thinking. The practical caution is that distributed
discrete-event simulation introduces causality-management problems. Keep Phase
5 single-authority and deterministic before considering any parallel logical
process model.

### Game Runtime References

[CDDA player activities](https://docs.cataclysmdda.org/PLAYER_ACTIVITY.html)
are the best concrete game reference for long-running work. Useful lessons:
activities can be interrupted, resumed, serialized, advanced per turn,
completed, canceled, and protected against infinite progress loops.

[CDDA Effect On Condition](https://docs.cataclysmdda.org/JSON/EFFECT_ON_CONDITION.html)
shows the power and danger of event-triggered scripting. It is useful as
reference pressure for context-bearing reaction rules, but `world` should keep
reaction work behind accepted runtime requests rather than letting event
listeners mutate original commits.

[Game Programming Patterns: Game Loop](https://gameprogrammingpatterns.com/game-loop.html)
and [Fix Your Timestep](https://gafferongames.com/post/fix_your_timestep/)
reinforce the local rule that hard simulation should not depend on wall-clock
frame cadence. For this repo, the primary model is not a rendering loop; it is
integer simulation time and scheduler drain.

Unity's [Entity Command Buffer](https://docs.unity.cn/Packages/com.unity.entities@1.0/manual/systems-entity-command-buffers.html)
is a useful staging analogy: commands can be recorded and played back later.
The limit of the analogy is that ECS structural change is not domain authority.
`world` needs domain-shaped causal transactions, not a generic component
command queue as its deepest boundary.

### Compiler And Incremental Runtime References

[MLIR pass management](https://mlir.llvm.org/docs/PassManagement/) is useful for
scope restrictions, pass failure, and analysis preservation/invalidation. The
best transfer is explicit stage contracts, not a pass framework.

[rustc incremental compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html)
and [Salsa](https://salsa-rs.github.io/salsa/overview.html) support the
derived-view side of the architecture: deterministic inputs, tracked
dependencies, stable keys, projection queries, and invalidation firewalls.

[Cousot and Cousot's abstract interpretation](https://www.di.ens.fr/~cousot/COUSOTpapers/POPL77.shtml)
is useful for multi-resolution discipline: abstract execution can be a sound
summary of concrete execution for a chosen purpose, but it is intentionally less
precise. This supports "coarser process progress with provenance" rather than
pretending abstract execution secretly performed every concrete action.

[Algebraic effects and handlers](https://www.eff-lang.org/handlers-tutorial.pdf)
support the typed effect interpretation analogy: authored effect programs can
name operations, while the runtime handler controls what those operations are
allowed to read, stage, emit, and publish.

### Transactions, Event History, And Replay

[PostgreSQL transaction isolation](https://www.postgresql.org/docs/current/transaction-iso.html)
is a useful analogy for staged mutation and commit failure. A staged
transaction is not a public fact, and commit can fail.

[Azure Event Sourcing](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
and [Fowler's Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html)
support immutable event history, audit, replay, versioning, and snapshots. They
also warn that event sourcing is costly and should be adopted where audit and
reconstruction justify it.

[Azure CQRS](https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs)
supports the local read/write separation: commands express business tasks,
queries do not alter data, and read models can be optimized separately.

[Fowler's Unit of Work](https://martinfowler.com/eaaCatalog/unitOfWork.html)
maps to the transaction builder idea: track affected state during a business
transaction and coordinate the final write plus concurrency handling.

[Temporal workflow definitions](https://docs.temporal.io/workflow-definition)
and [workflow execution](https://docs.temporal.io/workflow-execution) are useful
for deterministic replay and durable command history. The local conclusion is
not to copy stackful workflow code, but to record enough history and
non-determinism for the replay level being claimed.

### Rust API Authority References

[Rust visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html)
sets the real boundary mechanics. Since Rust lacks friend crates, public API
shape and crate layout matter for accepted-package receivers.

[Rust API Guidelines: future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)
supports private fields, newtypes, and sealed traits as authority tools.

[Rust API Guidelines: type safety](https://rust-lang.github.io/api-guidelines/type-safety.html)
supports domain-specific newtypes and argument types instead of primitive
tokens with ambiguous meaning.

[Rust HashMap](https://doc.rust-lang.org/std/collections/struct.HashMap.html)
documents arbitrary iteration order, which is enough reason to keep scheduler
ordering on explicit keys and ordered collections.

[Serde enum representations](https://serde.rs/enum-representations.html) is
relevant once durable process/runtime-control payloads become serialized. The
high-level caution is to choose versionable tagged envelopes for long-lived
records, not untagged formats whose interpretation depends on variant order.

### Formal Methods References

TLA+ is worth using selectively for small authority invariants, not for the
whole game. Good candidates:

- commit atomicity;
- same-time scheduler ordering;
- reservation conflict and release rules;
- lost wakeup prevention;
- zero-time loop guards.

This is most useful when a rule has a small state machine and a high cost of
getting it wrong.

## Structural Pressure On The Current Design

### No Major Architecture Reversal

The research does not suggest abandoning the current architecture. The strong
direction remains:

```text
domain-shaped runtime
materialized world model
typed effect programs
causal transaction waist
accepted package receivers
runtime-control gates
integer discrete-event scheduler
explicit serializable process state
```

The risky alternatives are less suitable:

- pure event sourcing as the whole state model;
- ECS/system graph as the authority model;
- direct delayed callbacks as scheduler events;
- saved async/coroutine stacks as process persistence;
- generic scripting with broad store mutation authority;
- hidden concrete `ActionRequest` spam for abstract progress;
- deterministic replay promised globally without recording enough inputs.

### Accepted Package Receiver Is The Sharpest Phase 4 Boundary

The local docs now say `world-runtime` owns accepted package construction and
`world-model` provides narrow application surfaces. This is the right boundary,
but it is the hardest one to express in Rust crate/module mechanics.

The important research conclusion is not "make a specific type now." It is:

```text
Applying an accepted package is storage work.
Constructing an accepted package is authority work.
Validating an accepted package is runtime work.
Publishing a package must be atomic from the caller's perspective.
```

If model-side APIs ever allow callers to construct a committed package, append
events separately from state, or publish invalidation separately from state,
the Phase 4 contract has failed.

### RuntimeControlStore Is Payload-Thin By Design, But Phase 5 Must Choose

Current `RuntimeControlRecord` stores kind plus provenance only. That was a good
Phase 3 storage shell. Phase 5 will need durable payload decisions for process
state, scheduled wakeups, reservations, RNG state, and activity state.

The high-level pressure is ownership, not fields:

```text
world-model can store runtime-control records.
world-runtime should own transition semantics and accepted update construction.
payload formats must be durable enough for save/load and replay claims.
```

### EffectKind Is A Checked Key, Not The Runtime Language

`EffectKind` is currently a normalized string key on checked effect operations.
That is acceptable for definition indexing and pack-facing vocabulary. It
should not become the interpreter's entire semantic model.

Phase 4 should preserve the distinction:

```text
EffectKind identifies an operation family.
StagePermission and event contracts constrain it.
Runtime-owned typed handlers define what it actually means.
```

The risk is a stringly generic interpreter where any pack can smuggle arbitrary
meaning through a key and broad permissions.

### Action Request Provenance Still Needs A Durable Story

The design docs show an `ActionRequest { id, source, submitted_at, ... }`
shape, but current core IDs do not yet include an `ActionRequestId`. That is
not automatically a problem. `ActionRequest` may be transient if committed
transactions preserve a source envelope that is stable enough for audit and
debugging.

The high-level question for Phase 4 is:

```text
What identity or source envelope is required for a committed transaction to
explain the accepted request, process tick, or reaction that caused it?
```

This should be answered before hard commit records become durable.

### EventRecord Should Be Evidence, Not A Command

Events should describe committed hard facts and provide evidence to semantic,
social, epistemic, and appraisal layers. Reactions may listen to events, but
the reaction should enqueue later work rather than mutate the original commit.

This prevents a common event-driven failure:

```text
event emitted
listener mutates world immediately
causal source becomes invisible or circular
```

The safer flow is:

```text
EventRecord
  -> reaction candidate
  -> ReactionRequest or ProcessTick
  -> new accepted transaction or accepted control update
```

### Scheduler Drain Needs Guardrails

Discrete-event references and CDDA both expose common failure modes:

- same-time ambiguity;
- lost wakeups;
- duplicate completion;
- stale canceled wakeups;
- zero-time loops;
- callback mutation that bypasses commit;
- work consumed before it is accepted, blocked, or rescheduled;
- wall-clock or collection-order leakage into hard outcomes.

The high-level Phase 5 contract should keep scheduler drains explainable:

```text
Every due wakeup has an outcome:
accepted, blocked, canceled, rescheduled, completed, failed, or skipped with
auditable reason.
```

The exact `DrainOutcome` shape can wait.

## High-Level Questions To Carry Into Phase Plans

These are not implementation tasks. They are the questions the concrete
Phase 4/5 plans should answer before code changes.

1. What is the accepted hard commit package taxonomy?
2. How does `world-runtime` construct accepted packages while `world-model`
   applies them without exposing forge paths?
3. What source identity is required for committed transactions caused by
   action requests, process ticks, passive processes, and reactions?
4. What minimum `TransactionRecord` and `EventRecord` data is required for each
   `ReplayLevel`?
5. Which runtime-control changes must be transaction-coupled, which may be
   control-only, and which are valid in both lanes?
6. Are reservation-only changes allowed through the control-only lane, and if
   so what accepted envelope proves ordering and conflict outcome?
7. What scheduler outcomes prevent lost wakeups and duplicate completion?
8. What guard prevents zero-time process or reaction loops?
9. Which runtime-control payloads must be versioned/serialized in Phase 5, and
   which can remain opaque or minimal?
10. What derived-view invalidation data is required from accepted hard commits
    versus accepted runtime-control updates?
11. Which non-deterministic inputs are recorded for deterministic command
    replay, and which paths should only claim audit or event rebuild?
12. Which stale research/design wording should be cleaned up once the Phase 4/5
    contracts are concretely settled?

## Recommended High-Level Adjustments

The implementation plan already has the right top-level direction after the
Phase 4/5 edits. The remaining high-level adjustments are doc-contract cleanup,
not new implementation detail:

1. Use "authority-bearing runtime control state" instead of "hard runtime
   control state" where the text is trying to distinguish runtime control from
   physical hard truth.
2. Keep reservation classification explicit: not a UI hint, not a social claim,
   not physical truth, but authority-bearing control state that may need
   transaction-coupled atomicity.
3. Keep generated/authored chronology out of `EventHistoryStore` unless it is
   materialized through causal runtime.
4. Keep `CausalTransactionGate` language precise: runtime owns gate semantics;
   model exposes accepted-package receiver behavior.
5. Keep compiler vocabulary limited to contracts, dependency tracking,
   invalidation, diagnostics, and provenance. Do not add a generic pass
   framework as a Phase 4/5 goal.
6. Keep `ProcessInstance` as the shared execution/progress frame across local
   and abstract resolution. Do not add a separate abstract runtime.

## Source Index

Local sources:

- `docs/architecture/implementation-plan.md`
- `docs/architecture/runtime-pipeline.md`
- `docs/architecture/implementation-execution-contract.md`
- `docs/design/causal-runtime.md`
- `docs/design/time-model.md`
- `docs/design/world-model.md`
- `docs/design/truth-authority-and-layer-boundaries.md`
- `docs/design/multi-resolution-simulation.md`
- `docs/design/intent-templates-and-planning.md`
- `docs/design/pack-authoring-and-semantic-declarations.md`
- `docs/research/runtime-pipeline-implementation-research.md`
- `docs/research/causal-runtime-action-effect-event.md`
- `docs/research/time-model-and-turn-scheduling.md`
- `docs/research/world-representation-query-model.md`
- `crates/world-core`
- `crates/world-defs`
- `crates/world-model`
- `crates/world-runtime`

External sources:

- [SimPy: Time and Scheduling](https://simpy.readthedocs.io/en/4.1.1/topical_guides/time_and_scheduling.html)
- [ns-3: Events and Simulator](https://www.nsnam.org/docs/manual/html/events.html)
- [Cataclysm: DDA Activities](https://docs.cataclysmdda.org/PLAYER_ACTIVITY.html)
- [Cataclysm: DDA Effect On Condition](https://docs.cataclysmdda.org/JSON/EFFECT_ON_CONDITION.html)
- [Game Programming Patterns: Game Loop](https://gameprogrammingpatterns.com/game-loop.html)
- [Fix Your Timestep](https://gafferongames.com/post/fix_your_timestep/)
- [Unity Entity Command Buffer](https://docs.unity.cn/Packages/com.unity.entities@1.0/manual/systems-entity-command-buffers.html)
- [MLIR Pass Management](https://mlir.llvm.org/docs/PassManagement/)
- [rustc incremental compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html)
- [Salsa overview](https://salsa-rs.github.io/salsa/overview.html)
- [Cousot and Cousot 1977 abstract interpretation](https://www.di.ens.fr/~cousot/COUSOTpapers/POPL77.shtml)
- [Pretnar: Algebraic Effects and Handlers](https://www.eff-lang.org/handlers-tutorial.pdf)
- [PostgreSQL transaction isolation](https://www.postgresql.org/docs/current/transaction-iso.html)
- [Azure Event Sourcing pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
- [Azure CQRS pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs)
- [Fowler: Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html)
- [Fowler: Unit of Work](https://martinfowler.com/eaaCatalog/unitOfWork.html)
- [Temporal Workflow Definition](https://docs.temporal.io/workflow-definition)
- [Temporal Workflow Execution](https://docs.temporal.io/workflow-execution)
- [Rust Reference: Visibility and Privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html)
- [Rust API Guidelines: Future Proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)
- [Rust API Guidelines: Type Safety](https://rust-lang.github.io/api-guidelines/type-safety.html)
- [Rust std HashMap](https://doc.rust-lang.org/std/collections/struct.HashMap.html)
- [Serde enum representations](https://serde.rs/enum-representations.html)
- [TLA+ Hyperbook](https://lamport.azurewebsites.net/tla/hyperbook.html)
