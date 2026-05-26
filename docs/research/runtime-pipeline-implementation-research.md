# Runtime Pipeline Implementation Research

## Status

Draft research.

## Purpose

This document researches implementation references for the future runtime
pipeline architecture document.

It is not:

- the runtime pipeline architecture document
- a crate boundary document
- engine code
- a schema design
- a test plan
- a vertical slice plan
- final public API design

The immediate goal is to answer:

```text
Which implementation patterns, runtime models, and Rust practices should guide
the runtime pipeline architecture?
```

## Current Design Anchors

The current design already fixes several important boundaries:

- `WorldModel` hosts authoritative and holder-relative store families.
- `QueryLayer` exposes typed, permissioned read surfaces.
- `CausalRuntime` is the hard mutation owner.
- `TypedEffectInterpreter` is an internal causal-runtime role, not a separate
  mutation authority.
- `CausalTransaction` is the atomic hard-simulation commit boundary.
- `EventRecord` is a committed hard causal fact.
- `Intent` is the commitment boundary.
- `Activity` is the temporal execution boundary.
- `ActionRequest` is the actor-facing concrete attempt boundary.
- `ProcessInstance` is the durable execution/progress frame.
- Concrete execution may lower `Intent -> Activity` to `ActionRequest` or
  `ProcessInstance`.
- Abstract execution lowers `Intent -> Activity` to `ProcessInstance`, not
  hidden concrete action spam.
- Non-hard accepted updates use their own commit surfaces and durable envelopes.
- Replay is tiered: baseline audit/explainability and save/load continuity,
  with stronger deterministic command replay only where declared.

The runtime pipeline architecture should preserve those terms. Compiler
language is useful for pass discipline, query invalidation, lowering, and
diagnostics; it should not rename domain boundaries into generic compiler
objects.

## Research Conclusion

Use a domain-owned staged runtime pipeline, not a generic framework where
arbitrary systems mutate state.

Recommended shape:

```text
InputSource / ScheduledWakeup
  -> request or wakeup envelope
  -> actor/world context binding
  -> validation and legalization
  -> Activity / ActionRequest / ProcessTick / ProcessInstance path
  -> Typed Effect Program instance when hard mutation is needed
  -> CausalTransaction staging
  -> invariant check
  -> atomic commit
  -> EventRecord + store updates + invalidation package
  -> observation, semantic, inspection, and future scheduling hooks
```

The pipeline should look compiler-shaped in its contracts:

```text
input representation
output representation
allowed reads
allowed writes
target contract
provenance output
invalidation dependencies
failure surface
replay level
```

But its implementation should be Rust-domain-shaped:

```text
concrete owner types
narrow internal traits
newtyped ids
value snapshots across borrow boundaries
serializable process state machines
explicit commit gates
structured diagnostics and provenance
```

## Reference Axis 1: Compiler Pass Discipline

### MLIR Pass Management

MLIR is useful less as a literal IR model and more as a pass-discipline
reference.

Relevant lessons:

- A pass should operate on a clear current unit.
- A pass should not inspect or mutate arbitrary sibling state.
- Analysis results need preservation and invalidation rules.
- Pass scheduling and diagnostics become tractable when pass contracts are
  explicit.

Transfer to `world`:

```text
RuntimePassContract:
  name
  owner
  input representation
  output representation
  allowed reads
  allowed writes
  target contract
  provenance output
  invalidation dependencies
  failure surface
  replay level
```

This does not mean the runtime needs a universal `dyn Pass` system on day one.
It means every logical stage should have a contract precise enough that later
optimization, tracing, plugin boundaries, and diagnostics do not depend on
tribal knowledge.

Useful runtime examples:

- observation projection is a derivation pass
- social context assembly is an access-filtered view pass
- appraisal is semantic analysis
- intent selection is a choice pass
- `Intent -> Activity -> ActionRequest / ProcessInstance` is lowering and
  legalization
- `Typed Effect Program -> CausalTransaction` is interpretation / handling /
  staging
- `CausalTransaction -> EventRecord + store updates` is publication

### rustc / Salsa Query Model

rustc incremental compilation and Salsa-style systems are useful references
for dependency-tracked query recomputation.

Relevant lessons:

- Query results should depend on stable query keys.
- Query dependencies should be explicit or discoverable.
- Cached results should be invalidated by dependency changes, not by vague
  "world changed" flags everywhere.
- Query systems work best when queries are pure reads, not hidden writes.
- Ordering matters when replaying or re-evaluating dependency paths.

Transfer to `world`:

```text
QueryKey:
  query kind
  actor or holder
  focus
  authority class
  visibility / epistemic epoch
  definition version
  store cursor or event-history cursor
  resolution
```

The first implementation does not need full automatic incremental computation.
It does need the architecture to separate:

```text
pure/cacheable query:
  reads state and returns a value, view, id list, or explanation

write-capable pass:
  creates accepted updates only through an authority gate

commit gate:
  publishes durable state and emits invalidation packages
```

This keeps `QueryLayer`, `DerivedViewRegistry`, and semantic context passes
from drifting into hidden mutation authority.

### Lowering / Legalization

The compiler view is most valuable when a higher-level representation must
prove that it can become a lower-level representation without violating
authority.

For runtime pipeline design, "lowering" should mean:

```text
source representation satisfies enough information and permissions
target representation satisfies its contract
provenance is preserved
failure has a typed surface
```

Important legalization edges:

```text
Intent -> Activity:
  preserves purpose, actor ownership, and selected approach

Activity -> ActionRequest:
  legal only for concrete attempts available to the actor

Activity -> ProcessInstance:
  legal for long-running, abstract, or strategic progress

ActionRequest / ProcessTick -> Typed Effect Program instance:
  legal only after role binding, current-state validation, and definition
  matching
```

The multi-resolution rule follows directly:

```text
abstract Activity -> ProcessInstance
not:
  abstract Activity -> repeated hidden concrete ActionRequest spam
```

### Algebraic Effects And Handlers

Algebraic effects are a useful theoretical analogy for `Typed Effect Program`.
The authored effect program describes typed operations. The runtime handler
decides how each operation validates, stages mutation, emits records, and
commits.

Transfer to `world`:

```text
Typed Effect Program:
  operation requests, permissions, required records

TypedEffectInterpreter:
  internal handler role
  validates operations
  stages mutation through transaction APIs
  records RNG provenance
  enforces required EventRecord contracts

CausalRuntime:
  owns transaction staging and commit
```

Do not transfer:

- treating `ProcessInstance` as a saved coroutine stack
- letting an effect handler mutate raw stores directly
- treating every semantic rule as a hard effect

### Abstract Interpretation

Abstract interpretation is a useful reference for multi-resolution execution.
It justifies the idea that abstract execution can preserve selected properties
without simulating every concrete step.

Transfer to `world`:

```text
abstract execution must preserve:
  stable identity where needed
  authority class
  durable consequence
  EventRecord provenance
  no contradiction with committed hard facts
```

Do not require exact equivalence between abstract and concrete histories.
Require declared preservation contracts and materialization boundaries.

## Reference Axis 2: Discrete-Event And Process Runtime

### Discrete-Event Scheduling

SimPy and ns-3 both support the current time model: an explicit simulation
agenda ordered by simulation time and a tie-breaker.

Transfer to `world`:

```text
ScheduledWakeup:
  time
  phase
  priority
  sequence
  target
```

The runtime pipeline should include a scheduler drain contract:

```text
drain due wakeups in canonical order
stop at protagonist input opportunity or declared boundary
make same-time ordering inspectable
record scheduling provenance when it affects committed outcomes
```

This is stronger than a frame loop and more suitable for:

- turn-based player feel
- actor activations
- passive processes
- delayed effects
- process wakeups
- reaction requests
- abstract travel and offscreen simulation

It should not imply global deterministic recomputation as a core principle.
It only means the scheduler must expose enough ordering for audit, save/load,
and declared replay levels.

### Process Interaction

SimPy process interaction is useful for vocabulary:

- wait until woken
- wait for another process
- wait for multiple events
- interrupt with cause
- shared resources and queues

Transfer to `world`:

```text
ProcessInstance:
  state
  progress
  wait condition
  reservations
  interrupt policy
  resume policy
  failure policy
  active resolution
```

The important adaptation for Rust and save/load is:

```text
durable serializable state machine
not stackful coroutine persistence
```

Processes should submit `ProcessTick`, `ContinueProcessRequest`, or ordinary
`ActionRequest`s. They should not mutate hard state directly.

## Reference Axis 3: Transaction History, CQRS, And Event Sourcing

Event sourcing and CQRS are useful references but should not become the root
architecture.

Useful lessons:

- Commands/requests are not events.
- Events record accepted facts.
- Read models/projections are separate from write authority.
- Materialized views are useful when replaying an event log for every query is
  too expensive.
- Event sourcing has real complexity in schema evolution, consistency, and
  migration.

Transfer to `world`:

```text
ActionRequest / ProcessTick:
  request or command-like attempt

CausalTransaction:
  checked commit envelope

EventRecord:
  committed hard causal fact

DerivedViewRegistry:
  materialized projection owner
```

Reject:

- pure event sourcing as the whole world model
- `EventRecord` as a low-level mutation diff
- event listeners that mutate the original transaction
- read projections that become hidden truth

The current design's materialized stores plus transaction/event history is a
better fit:

```text
current state:
  fast query and validation

history:
  audit, explanation, rebuild, replay tiers, semantic evidence
```

## Reference Axis 4: Rust Implementation Practices

### Domain Types Before Framework Traits

Rust favors explicit data ownership, concrete types, enums, and newtypes for
domain boundaries.

Recommended stance:

```text
concrete owner types first
narrow traits for internal capability surfaces
sealed or pub(crate) traits where extensibility is not yet intended
public plugin traits only after the extension contract is stable
```

Early runtime architecture should avoid a large public trait framework such as:

```text
trait RuntimeSystem {
  fn run(&mut self, world: &mut WorldModel);
}
```

That shape is too permissive. It hides allowed reads, allowed writes,
authority, provenance, and invalidation.

Prefer narrow surfaces:

```text
KernelQuery
ActorRelativeQuery
SemanticContextQuery
TransactionStager
EffectHandler
DefinitionLookup
DiagnosticSink
```

### ID Taxonomy

Use newtypes rather than aliases for IDs that have different meanings.

Likely categories:

```text
PersistentEntityId:
  durable story identity

RuntimeEntityHandle:
  in-memory loaded-world handle

DefinitionId:
  checked authored definition identity

EventRecordId:
  committed hard fact identity

CausalTransactionId:
  committed transaction envelope identity

ProcessInstanceId:
  durable process/progress identity

ActivityId:
  actor-facing execution-frame identity, when persisted or referenced

QueryEpoch / StoreCursor:
  invalidation and cache boundary marker
```

Use storage helpers such as `slotmap` only for runtime handles where their
semantics match. Durable identities should remain independent of any in-memory
container.

### Borrowing And Transaction Staging

The runtime should not pass a broad `&mut WorldModel` through every stage.
That makes it easy to bypass boundaries and hard to compose reads with staged
writes.

Recommended shape:

```text
CausalRuntime:
  owns the mutable world borrow only at narrow points

QueryLayer:
  returns ids, value snapshots, read tokens, or derived views

TypedEffectInterpreter:
  receives a transaction staging API, not raw store access

CausalTransactionBuilder:
  accumulates reads, reservations, RNG draws, mutations, schedules, and
  EventRecord candidates

CausalTransactionGate:
  validates staged mutation and commits atomically
```

This aligns with Rust's borrowing model: long-lived immutable views and
wide mutable access do not mix well. The architecture should use short mutable
borrows at commit boundaries and value-like intermediate records between
passes.

### Outcomes Versus Errors

Do not encode ordinary gameplay failures as infrastructure errors.

Recommended split:

```text
RuntimeOutcome:
  Rejected
  Blocked
  AttemptFailed
  Interrupted
  ConflictResolved
  Committed
  AbortedWithNoCommit

RuntimeError:
  missing definition
  malformed checked data
  violated engine invariant
  IO / serialization failure
  version incompatibility
  corrupted save or pack registry
```

In Rust terms:

```text
Result<RuntimeOutcome, RuntimeError>
```

`thiserror` is a good fit for library error enums. `miette` is a good fit for
pack/source diagnostics with spans, labels, related errors, and help text.

### Serializable State Machines

`ProcessInstance` should be represented as explicit state, not a stackful
async task or generator.

Rust enums and Serde tagged enum representations are a natural fit for:

```text
ProcessState:
  Starting
  Waiting
  Advancing
  Paused
  Interrupted
  Completed
  Failed
```

The architecture should keep process transition behavior explicit:

```text
ProcessTick(input, state) -> TickOutcome
Interrupt(input, state) -> InterruptOutcome
Resume(input, state) -> ResumeOutcome
```

The exact code shape can wait, but the future architecture should avoid
requiring a persisted async stack.

### Core Sync, Host Async

Async Rust is useful for IO, networking, editor integration, plugin sandboxes,
asset loading, remote tools, and host orchestration.

The core simulation runtime should remain synchronous unless a concrete
subsystem proves otherwise:

```text
execute_request(...)
tick_process(...)
drain_scheduler(...)
commit_transaction(...)
```

This keeps save/load, audit, debugging, and borrow boundaries simpler. An
`EngineHost` can call synchronous runtime APIs from async services.

### Replay-Aware Ordering Without Overstating Determinism

Rust's standard `HashMap` has arbitrary iteration order and randomized seeding.
That is fine for storage and lookup, but replay-relevant ordering should not
depend on it.

Recommended rule:

```text
If ordering affects committed results or declared replay output, make ordering
explicit.
```

Use:

- explicit `sequence`
- sorted vectors
- `BTreeMap`
- stable registry order
- declared tie-breakers
- recorded RNG stream/draw provenance

Do not turn this into a global "determinism above all" principle. It is a
contract selected per path by `ReplayLevel`.

## Reference Axis 5: ECS And Other Accelerators

ECS libraries such as Bevy ECS and hecs are useful references for storage,
iteration, schedules, change detection, and system access declarations.

They should not become the root source of truth.

Recommended architecture stance:

```text
WorldModel:
  owns domain truth and store families

ECS / graph / Datalog / incremental query / dataflow:
  optional projection, cache, accelerator, batch executor, or tooling surface
```

Allowed uses:

- hot local spatial/component iteration
- derived projection storage
- batch validation acceleration
- debug visualization indexes
- pathfinding or graph snapshots
- semantic derived-view acceleration

Forbidden uses:

- ECS systems directly commit hard truth
- ECS entity ids as durable story identity
- component marker tags as the semantic model
- ECS events as `EventRecord`s
- query accelerators bypassing actor-relative access control

This matches the current ADR: domain-owned simulation core first,
accelerators behind explicit boundaries later.

## Reference Axis 6: Diagnostics, Provenance, And Explanation

The runtime pipeline needs diagnostics at three levels:

```text
authoring diagnostic:
  pack source, symbol, type, stage permission, verifier error

runtime validation feedback:
  actor-facing rejected/blocked/failed explanation

debug/provenance trace:
  query reads, rule matches, lowering choice, effect staging, committed
  mutations, emitted EventRecord ids, invalidated views
```

These should share a provenance attachment model even if their audiences
differ.

Likely provenance packet fields:

```text
ProvenancePacket:
  source definition id
  pack source span?
  actor / holder
  query keys read
  EventRecord refs
  rule/template match refs
  selected lowering target
  validation result
  staged mutation refs
  emitted record ids
  invalidation package
  replay level
```

This is important because the engine is explicitly actor-relative and
compiler-shaped. Without structured provenance, the runtime will be difficult
to debug once semantic appraisal, social context, process execution, and
multi-resolution lowering interact.

## Candidate Runtime Pipeline Contracts

These are research outputs for the future architecture doc. They are not final
APIs.

### Scheduler Drain Contract

```text
Input:
  current sim time
  scheduled wakeups
  protagonist readiness / input boundary policy

Output:
  ordered execution of due wakeups
  possible input opportunity
  scheduling provenance

Must preserve:
  explicit same-time ordering
  no hidden wall-clock dependency in hard simulation
  bounded zero-time loops
```

### Query Discipline Contract

```text
Pure query:
  no writes
  stable key
  declared authority and visibility class
  dependency / invalidation source
  actor-visible or debug-only classification

Write-capable operation:
  not a query
  must use an accepted update or transaction gate
```

### Request Binding Contract

```text
Input:
  ActionRequest / ProcessTick / ReactionRequest
  relevant definition ids
  actor-relative or kernel query context

Output:
  bound executable attempt
  InvalidActionFeedback
  or diagnostic/runtime error

Must preserve:
  actor ownership where applicable
  role binding provenance
  definition version
  current-world validation context
```

### Lowering / Legalization Contract

```text
Input:
  selected Intent
  Activity preparation
  active resolution
  CapabilitySet / ActionRepertoire / PerceivedAffordance
  active process state

Output:
  Activity continuation
  ActionRequest
  ProcessInstance
  InvalidActionFeedback

Must preserve:
  Intent -> Activity boundary
  abstract execution through ProcessInstance
  no hard mutation during lowering
```

### Effect Handling Contract

```text
Input:
  Typed Effect Program instance
  transaction staging API
  current validation context

Output:
  staged reads
  staged reservations
  staged RNG draws
  staged mutations
  staged EventRecord candidates
  staged schedule changes

Forbidden:
  raw WorldModel mutation
  direct non-hard state mutation
  final EventHistoryStore append outside commit
```

### Commit And Invalidation Contract

```text
Input:
  staged CausalTransaction

Output:
  committed TransactionRecord
  EventRecord ids
  store updates
  RuntimeControlStore updates
  invalidation package
  observation / semantic scheduling hooks

Must preserve:
  atomic commit
  EventHistoryStore append through CausalRuntime
  replay/audit provenance required by ReplayLevel
```

## Recommended Architecture Implications

The future `runtime-pipeline.md` should probably include:

- one canonical pipeline diagram
- one table of runtime representations by owner
- one table of pass classes and transformation kinds
- a `RuntimePassContract` shape
- query/read-surface discipline
- scheduler drain contract
- request binding and validation path
- `Intent -> Activity -> ActionRequest / ProcessInstance` lowering contract
- `ProcessInstance` lifecycle contract
- `Typed Effect Program` handling contract
- commit, invalidation, and publication contract
- diagnostics/provenance surface
- replay-level requirements
- accelerator boundaries

It should not include:

- final crate names
- final Rust type definitions
- parser/source syntax
- vertical slice order
- gameplay-specific action libraries
- AI policy implementation

## Anti-Patterns To Avoid

### One Generic System Mutates World

```text
System::run(&mut WorldModel)
```

This hides authority, invalidation, provenance, and failure surfaces.

### ECS As Authority

ECS may store or accelerate projections. It should not own `EventRecord`,
`CausalTransaction`, actor-relative access, or durable story identity.

### Saved Coroutine Processes

Long-running gameplay work should be a serializable `ProcessInstance`, not a
persisted async task, stack, or generator.

### Event Listeners As Mutation Backdoors

Reactions should enqueue `ReactionRequest` or `ProcessTick`. They should not
mutate the transaction that emitted the source `EventRecord`.

### Pure Event Sourcing Everywhere

Materialized stores are necessary for fast validation, actor context,
capability queries, process interruption, and abstract resolution.

### Everything Async

Async belongs in host/adapters unless a concrete runtime subsystem proves it
needs async. The simulation core should remain synchronous and inspectable.

### Semantic Policy As Runtime Authority

Social, appraisal, intent, AI, and planning outputs may propose, score, select,
or request through typed gates. They should not bypass hard mutation or
non-hard accepted update boundaries.

## Open Questions For The Architecture Document

These should be answered in `docs/architecture/runtime-pipeline.md`, not here.

1. How much dependency tracking is manual in the first implementation, and
   which derived views need a query-database-like mechanism later?
2. What is the minimum `ProvenancePacket` / explanation envelope that every
   runtime boundary should carry?
3. Which runtime passes are always present, and which are optional extension
   points?
4. How does `EngineHost` drive scheduler drain, protagonist input, save/load,
   and inspection without owning simulation authority?
5. What exact non-hard accepted update envelope names should appear in the
   runtime pipeline architecture?
6. Which replay fields belong in `CommitEnvelope`, `TransactionRecord`,
   `EventRecord`, and process state?
7. Which parts of semantic decision evaluation are synchronous runtime passes
   versus deferred or cached derived views?
8. What is the first crate/module grouping that preserves boundaries without
   oversplitting logical components?

## Sources

Compiler and query references:

- [MLIR Pass Management](https://mlir.llvm.org/docs/PassManagement/)
- [MLIR Language Reference](https://mlir.llvm.org/docs/LangRef/)
- [rustc incremental compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html)
- [rustc guide: Salsa](https://rustc-dev-guide.rust-lang.org/queries/salsa.html)
- [Cousot and Cousot, Abstract Interpretation](https://cs.nyu.edu/~pcousot/COUSOTpapers/POPL77.shtml)
- [Plotkin and Pretnar, Handling Algebraic Effects](https://lmcs.episciences.org/705)

Simulation and process references:

- [SimPy Time and Scheduling](https://simpy.readthedocs.io/en/latest/topical_guides/time_and_scheduling.html)
- [SimPy Process Interaction](https://simpy.readthedocs.io/en/latest/topical_guides/process_interaction.html)
- [ns-3 Events and Simulator](https://www.nsnam.org/docs/release/3.38/manual/html/events.html)

Transaction, projection, and history references:

- [Azure Architecture Center: Event Sourcing pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
- [Azure Architecture Center: CQRS pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs)
- [Martin Fowler: Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html)
- [Martin Fowler: CQRS](https://martinfowler.com/bliki/CQRS.html)

Rust implementation references:

- [Rust API Guidelines: Type safety](https://rust-lang.github.io/api-guidelines/type-safety.html)
- [Rust Book: Advanced Types / Newtype](https://doc.rust-lang.org/stable/book/ch20-03-advanced-types.html)
- [Rust Book: Enums](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)
- [Rust Book: References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)
- [Rust Book: Recoverable Errors with Result](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)
- [Serde enum representations](https://serde.rs/enum-representations.html)
- [thiserror](https://docs.rs/thiserror/latest/thiserror/)
- [miette](https://docs.rs/miette/latest/miette/)
- [slotmap](https://docs.rs/slotmap/latest/slotmap/)
- [Rust HashMap](https://doc.rust-lang.org/std/collections/struct.HashMap.html)
- [rand_chacha](https://docs.rs/rand_chacha/latest/rand_chacha/)
- [Async Rust Book](https://rust-lang.github.io/async-book/)

ECS and accelerator references:

- [Bevy ECS schedule module](https://docs.rs/bevy/latest/bevy/ecs/schedule/)
- [hecs](https://docs.rs/hecs/latest/hecs/)
