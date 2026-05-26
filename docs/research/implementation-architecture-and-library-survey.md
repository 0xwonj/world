# Implementation Architecture And Library Survey

## Status

Draft implementation research.

## Purpose

This document researches implementation architecture choices that affect the
first engine structure.

It is not another domain-theory pass. Earlier research already shaped the
simulation, epistemic, social, appraisal, intent, process, and pack-authoring
design. This pass asks:

```text
If we now start turning the design into Rust crates, which architectural
choices would constrain everything else?
```

The focus is the early implementation order:

1. Authority / causal core
2. Typed world representation and identity
3. Typed effect program and transaction interpreter
4. Event history, replay, auditability, scheduler, and process state
5. Pack compiler, verifier, diagnostics, and definition registry
6. Derived views, semantic declaration evaluation, and later accelerators

Out of scope:

- engine code
- final crate layout
- final source syntax
- vertical-slice planning
- rendering, UI, networking, deployment, or editor integration
- concrete game-system content

## Existing Design Inputs

Relevant current documents:

- [Simulation Core](../design/simulation-core.md)
- [World Model](../design/world-model.md)
- [Causal Runtime](../design/causal-runtime.md)
- [Typed Effect Primitives](../design/typed-effect-primitives.md)
- [Time Model](../design/time-model.md)
- [Simulation Transition Compiler](../design/simulation-transition-compiler.md)
- [Pack Authoring And Semantic Declarations](../design/pack-authoring-and-semantic-declarations.md)
- [Multi-Resolution Simulation](../design/multi-resolution-simulation.md)
- [Engine Core And Game System Boundary](../design/engine-core-and-game-system-boundary.md)
- [World Representation / Query Model](world-representation-query-model.md)
- [Causal Runtime / Action-Effect-Event](causal-runtime-action-effect-event.md)
- [Time Model / Turn Scheduling](time-model-and-turn-scheduling.md)
- [Engine Architecture Entry](engine-architecture-entry.md)

The existing docs already imply several implementation constraints:

- `CausalTransaction` is the deepest hard-mutation boundary.
- `EventRecord` is a committed hard fact, not a command.
- `ActionRequest` is actor-facing attempt, not the whole runtime.
- `ProcessInstance` is durable runtime control state.
- Abstract simulation uses `ProcessInstance` / `ProcessTick`, not hidden local
  action spam.
- `Typed Effect Program` is a checked hard-mutation IR interpreted by the
  causal runtime.
- Semantic declarations are checked declarations that emit proposals,
  candidates, derived context, or non-hard records according to their kind.
- Semantic rules, scripts, event listeners, and external adapters must not
  mutate hard truth directly.

## Research Conclusion

Use a domain-owned core architecture.

```text
WorldModel authority:
  typed WorldStore
  typed RelationStore families
  RuntimeControlStore
  EventHistoryStore
  soft/actor/appraisal stores
  DerivedViewRegistry
  permissioned QueryLayer
  CausalTransactionGate

Optional hot projections:
  ECS view for concrete local simulation
  graph/pathfinding snapshots
  Datalog-like derived closures
  incremental compiler query database
```

The early engine should not make ECS, Datalog, a graph database, a scripting
VM, or an event-sourcing framework the source of truth.

Recommended implementation stance:

```text
Domain-owned stores first.
Compiler-like pack pipeline first.
Custom typed effect interpreter first.
Custom semantic declaration evaluator first.
ECS / graph / Datalog / scripting only as later projections, accelerators,
or tooling surfaces.
```

This keeps the current design's authority boundaries intact while still
allowing specialized libraries where they are useful.

## Decision 1: ECS Is A Storage Tool, Not The World Ontology

### What ECS Gives

ECS is attractive because it gives:

- entity/component storage
- fast iteration over hot component sets
- system scheduling
- declared data access
- change detection
- parallel execution

[Bevy ECS](https://docs.rs/bevy/latest/bevy/ecs/index.html) is a strong Rust
reference. Its docs describe entities, components, systems, resources,
schedules, change detection, and table versus sparse-set component storage.
It can also be used standalone outside the full Bevy engine.

[Flecs relationships](https://www.flecs.dev/flecs/md_docs_2Relationships.html)
are especially relevant as a design reference because they model edges as
relationship-target pairs and support relation queries. This maps well to
containment, equipment, membership, attachment, and topology pressure.

### Why ECS Should Not Be Source Of Truth

The project needs more than hot component iteration:

- stable story identity for memory, history, save/load, replay, and social
  references
- typed relation families with authority classes
- actor-relative query permission boundaries
- committed `EventRecord` history
- derived views with provenance
- soft truth and actor truth stores distinct from hard truth
- transaction-stage mutation and atomic commit

Raw ECS mutation would make it too easy for arbitrary systems to write
components directly. That conflicts with:

```text
No hard world mutation outside CausalTransaction.
```

It also risks turning semantic conditions into marker components and relation
facts into opaque component fields.

### Recommended Use

Use ECS only behind a domain-owned API.

```text
WorldStore
  may internally use ECS-like storage
  may later expose local concrete simulation as a materialized ECS view
  may use ECS schedules for non-authoritative projections

But:
  ECS entity ids are runtime handles, not durable story identity
  ECS systems cannot be public mutation authority
  ECS events/observers cannot bypass CausalTransaction
```

Implementation rule:

```text
The engine may optimize storage with ECS.
The engine API must remain WorldModel / QueryLayer / CausalTransaction shaped.
```

## Decision 2: Typed Hybrid World Model Is The Best Root

The strongest model remains the one already selected by
[World Model](../design/world-model.md):

```text
WorldModel
  WorldStore
  RelationStore
  EventHistoryStore
  RuntimeControlStore
  SocialInstitutionalStore
  ChronologyStore
  EpistemicStore
  AppraisalRecordStore
  DerivedViewRegistry
  QueryLayer
  CausalTransactionGate
```

Implementation implications:

- use newtyped ids instead of raw integers or raw ECS ids
- separate runtime handles from persistent ids
- store relation families explicitly
- index relations by subject, relation, object, and authority class where
  needed
- keep derived views rebuildable and explainable
- keep write paths behind commit surfaces

### Identity And Handles

Use two classes of identity:

```text
PersistentEntityId:
  stable across save, replay, memory, history, social references

RuntimeEntityHandle:
  efficient in-memory handle for current loaded simulation state
```

[slotmap](https://docs.rs/slotmap/) is a good candidate for runtime handles
because it provides persistent unique keys, O(1) insert/remove/access, custom
key types, and secondary maps. It is useful for dynamic game objects and graph
nodes.

Avoid relying on
[generational-arena](https://docs.rs/generational-arena/latest/generational_arena/)
as a default candidate because RustSec marks it
[unmaintained](https://rustsec.org/advisories/RUSTSEC-2024-0014.html) and
lists `slotmap` as an alternative.

Dense compiler registries may not need deletion-heavy handles. They can use
typed indices or dense vectors later. The architectural decision is only:

```text
Do not serialize runtime handles as durable world identity.
```

### Relation Storage

Use typed relation tables, not a universal graph:

```text
ContainedIn(subject, container)
EquippedInSlot(entity, actor, slot)
AttachedTo(child, parent)
EmbeddedIn(entity, target)
LocatedIn(entity, place)
PassageTo(from_place, to_place)
MemberOf(actor_or_group, faction)
SocialClaimOn(holder, object_or_right)
```

Each relation family declares:

- authority class
- cardinality
- inverse index policy
- allowed mutation gate
- provenance source
- invalidation dependencies

This lets the engine answer inverse and closure queries without making
everything `Fact(subject, predicate, object)`.

### Graph Libraries

[petgraph](https://github.com/petgraph/petgraph) is useful for graph
algorithms and snapshots. It should not be authority storage.

Good uses:

- route graphs
- pathfinding over a projected topology
- social network analysis snapshots
- dependency graphs inside the pack compiler

Bad uses:

- authoritative universal world graph
- untyped relation soup
- actor queries that bypass access filtering

## Decision 3: Use Command / Transaction Sourcing, Not Pure Event Sourcing

[Event Sourcing](https://www.martinfowler.com/eaaDev/EventSourcing.html)
stores state changes as a sequence of events and can rebuild historical state
from the event log. That vocabulary is useful, but pure event sourcing is not
the best runtime storage model for this engine.

The project needs current materialized world state for:

- local physical simulation
- capability validation
- perception projection
- process interruption
- derived views
- actor-facing affordance queries
- abstract resolution ticks

Recommended shape:

```text
ActionRequest / ProcessTick / ReactionRequest
  -> bind / validate
  -> Typed Effect Program
  -> CausalTransaction staging
  -> atomic commit
  -> update materialized stores
  -> append TransactionRecord + EventRecord set
  -> invalidate derived views
  -> project observations
```

That is closer to command/transaction sourcing:

- requests are commands or attempts
- transactions are committed mutation envelopes
- events are durable facts and evidence
- current stores remain materialized
- snapshots are normal save/load optimization

### Why Not An Event-Sourcing Framework

Typical CQRS/event-sourcing frameworks focus on aggregates and command
handlers. A simulation world has cross-cutting transactions:

- an actor moves an object between containers
- a wound changes capability
- smoke emits a sensory signal
- a process reservation changes conflict behavior
- a death creates observation evidence and later social meaning
- abstract travel changes resolution-aware location and future wakeups

These are not cleanly one-aggregate events.

Use event-sourcing principles, but keep the engine's own transaction
interpreter as the authority.

### Event-Driven Warning

Fowler's
[event-driven architecture note](https://martinfowler.com/articles/201701-event-driven.html)
warns that event notification can obscure larger flows, and that an event can
be misused as a passive-aggressive command.

For `world`:

```text
EventRecord:
  committed fact and evidence

ReactionRequest:
  later request caused by an event

ProcessTick:
  scheduled execution request

Semantic proposal:
  later interpretation, not hard mutation
```

Do not let `EventRecord` listeners mutate hard truth inside the original
transaction unless they are part of the same checked effect program.

## Decision 4: Replay, Auditability, And Commit Provenance

Replay has more than one meaning. The core architecture should not require
global deterministic recomputation of the whole simulation as the default.

Useful modes:

```text
Event replay:
  committed TransactionRecord / EventRecord history is used to inspect or
  rebuild consequences without rerunning every decision.

Audit replay:
  committed records, validation context, accepted ordering, and recorded random
  draws where relevant explain why an outcome happened.

Command replay:
  selected debug, test, or subsystem paths rerun from input logs and expect
  matching results.
```

Command replay is valuable, but it is a selected capability. The baseline
requirement is that committed outcomes have enough provenance for inspection,
save/load continuity, debugging, and explanation.

### Ordering

[ns-3's scheduler](https://www.nsnam.org/docs/release/3.45/manual/html/events.html)
handles events by increasing simulation time and uses a monotonically
increasing id as a same-time FIFO tie breaker. The exact structure need not be
copied, but the principle is useful when an execution path needs canonical
ordering:

```text
committed outcomes should record accepted ordering when ordering matters
```

Candidate scheduler key:

```text
(sim_time, phase, priority, sequence)
```

Rust's `HashMap` is randomly seeded by default, according to the
[standard library docs](https://doc.rust-lang.org/std/collections/struct.HashMap.html).
If a subsystem requires canonical ordering, use explicit sorting or ordered
collections instead of treating incidental map iteration as an explanation.
`BTreeMap` stores entries by key order and its iterators produce items in key
order according to the
[standard library docs](https://doc.rust-lang.org/std/collections/btree_map/struct.BTreeMap.html).

### RNG

Record random draw provenance when randomness affects committed outcomes:

```text
RngStream {
  stream_id
  seed
  draw_index
  purpose
}
```

Relevant stream families:

- world generation
- local combat/checks
- passive physical processes
- abstract simulation
- social/semantic stochastic choices where accepted into hard path
- debugging/fuzzing

`rand_chacha` documents its generators as deterministic and portable in the
[crate docs](https://docs.rs/rand_chacha/latest/x86_64-pc-windows-msvc/rand_chacha/).
It is a useful candidate for paths that opt into command replay. Whether or
not that exact crate is selected later, the architecture should record random
draw provenance in `CausalTransaction` where the draw affects accepted hard
outcomes.

### Parallelism

Parallel execution is acceptable only when:

- results are read-only, or
- writes are staged with explicit accepted ordering, or
- operations are provably commutative, or
- the runtime records enough provenance to explain the committed result

Parallelism is an implementation tradeoff. It should not be forbidden by the
architecture, but committed results still need an inspectable transaction
record.

## Decision 5: Process Runtime Should Be Explicit State Machines

[SimPy](https://simpy.readthedocs.io/en/latest/topical_guides/process_interaction.html)
is useful vocabulary for process interaction: sleep until woken, wait for
another process, interrupt another process, and shared resources.

Transfer the concepts, not the implementation style.

Do not save stackful coroutines as process state. Store serializable
`ProcessInstance` records:

```text
ProcessInstance {
  id
  process_def
  owner
  source_intent?
  activity?
  resolution
  target_roles
  progress
  local_state
  wait_condition
  reservations
  next_wakeup
  interrupt_policy
  resume_policy
  failure_policy
  rng_state_or_draw_refs
  provenance
}
```

Execution shape:

```text
ProcessInstance
  -> scheduled ProcessTick
  -> bind current context
  -> Typed Effect Program or runtime primitive
  -> CausalTransaction
  -> EventRecord
  -> update ProcessInstance / schedule next wakeup / complete / interrupt
```

This matches the multi-resolution rule:

```text
Concrete:
  selected Intent lowers through Activity to ActionRequest or ProcessInstance

Abstract:
  selected Intent lowers through Activity to ProcessInstance, not concrete
  action spam
```

## Decision 6: Typed Effect Program Needs A Custom Interpreter

`Typed Effect Program` is authority-sensitive enough that it should be a
custom IR plus custom interpreter.

Do not implement hard effects with:

- arbitrary Rust callbacks from content
- generic scripting APIs
- ECS `&mut Component` access from arbitrary systems
- stringly `SetField`
- Datalog rules that mutate stores

Use a small checked IR:

```text
EffectProgramIR
  parameters
  required_reads
  allowed_effects
  control_flow
  primitive_calls
  required_event_records
  replay_requirements
  stage_permissions
```

Primitive contract:

```text
primitive transfer_entity(entity, from, to):
  reads:
    current containment
    capacity / reachability where required
  validates:
    entity exists
    from contains entity
    to can contain entity
  stages:
    remove ContainedIn(entity, from)
    add ContainedIn(entity, to)
  emits:
    EntityTransferred
  invalidates:
    containment closure
    visibility
    possession-derived capability inputs
```

Interpreter modes:

```text
validate only
dry-run / preview
planning estimate
commit staging
replay audit
debug explanation
property-test model
```

This mirrors typed-effect and effect-handler ideas without importing a
general-purpose effect language.

## Decision 7: Pack Compiler Should Be Compiler-First, Syntax-Second

The stable artifact is not the first source syntax. The stable artifact is:

```text
pack source
  -> AST with spans
  -> declaration IR
  -> symbol/type/authority verification
  -> definition registry
  -> runtime indexes
```

Two IR families remain separate:

```text
Typed Effect Program IR:
  hard-mutation effect programs for actions, processes, reactions

Semantic Declaration IR:
  social_rule
  appraisal_rule
  intent_template
  semantic_view
```

### Parser Candidates

[chumsky](https://docs.rs/chumsky/latest/chumsky/index.html) is the best first
candidate for a custom authored DSL because it is Rust-native, expressive,
supports spans, has error recovery support, and is comfortable for syntax that
may still evolve.

[pest](https://docs.rs/pest/latest/pest/) is a reasonable backup if a simple
PEG grammar is preferred.

[LALRPOP](https://lalrpop.github.io/lalrpop/) is useful if the grammar becomes
stable and LR-shaped. It is less attractive while the language is still in
flux.

Recommended stance:

```text
Start with AST/IR/verifier design.
Use chumsky when source syntax begins.
Keep parser crate isolated so parser generic types do not leak into core.
```

### Diagnostics

Use an internal structured diagnostic model:

```text
Diagnostic {
  code
  severity
  primary_span
  labels
  related_spans
  help
  authority_boundary?
  declaration_id?
  source_chain
}
```

Then render it through a library.

[miette](https://docs.rs/miette/latest/miette/) is a strong first candidate
because it provides diagnostic codes, snippets, labels, help text, related
errors, and graphical terminal rendering.

[ariadne](https://docs.rs/ariadne/latest/ariadne/) is useful when hand-tuned
compiler-style reports are needed.

`codespan-reporting` remains a lower-level option. It is documented as
diagnostic reporting support for compiler-like tools in the
[crate docs](https://docs.rs/crate/codespan-reporting/0.9.5).

Implementation rule:

```text
Snapshot and test structured diagnostics.
Do not make terminal rendering the diagnostic source of truth.
```

### Incremental Queries

[Salsa](https://rustc-dev-guide.rust-lang.org/queries/salsa.html) is a library
for incremental recomputation. It is useful for compiler/tooling queries, not
runtime world mutation.

Good query candidates:

- file contents
- parsed AST
- module namespace
- symbol table
- resolved declarations
- typed declarations
- effect signatures
- rule indexes
- event-contract verification
- pack dependency graph

Bad query candidates:

- mutable `WorldModel`
- active `CausalTransaction`
- scheduler state
- RNG stream state
- `ProcessInstance` ticking

Use incremental compilation for pack authoring and editor-like tooling later.
Do not put the simulation runtime inside Salsa.

## Decision 8: Semantic Declarations Need A Custom Evaluator First

Datalog and rule engines are useful references, but semantic/appraisal/intent
declarations are not only logical derivations.

They need:

- actor-relative access
- declaration-kind authority rules
- provenance
- scoring features
- stage-specific outputs
- non-hard commit gates
- resolution-aware lowering metadata
- explanation

[datafrog](https://docs.rs/datafrog/latest/datafrog/) is a lightweight
Datalog engine in Rust whose docs describe static `Relation` sets and
monotonically increasing `Variable` sets. That is useful for closure-like
derived facts.

Good Datalog-like uses:

- containment ancestry
- reachability closure
- jurisdiction closure
- pack dependency closure
- rule dependency checks
- offline verification
- monotone derived views

Bad Datalog-like uses:

- final intent selection
- non-monotone motivational scoring
- hard mutation
- actor memory writes
- social truth commits
- process execution

Recommended implementation:

```text
SemanticDeclarationIR
  -> custom trigger-indexed matcher
  -> typed binding evaluator
  -> condition evaluator over QueryLayer
  -> output gate by declaration kind
  -> provenance / explanation record
```

Use Datalog-like tools only as internal accelerators for specific derived-view
subproblems.

## Decision 9: Scripting And Wasm Are Extension Boundaries

Initial pack architecture should stabilize checked IR and verifier behavior
before choosing scripting or plugin runtimes.

Scripting engines are good at flexible behavior. This project needs checked
authority boundaries, replayable effects, typed provenance, and staged
transactions. A scripting VM should not be the authority model for pack
definitions.

[Rhai](https://rhai.rs/) is a small Rust-friendly embedded scripting language.
It may be useful later for trusted tooling helpers or build-time generation.

[Wasmtime](https://docs.wasmtime.dev/api/wasmtime/) is a strong long-term
candidate for isolated third-party plugins. Its Rust API embeds a WebAssembly
engine and lets the host expose explicit imports. It is too much machinery for
the first core runtime.

Recommended rule:

```text
Scripts may generate declarations.
Scripts may run editor/tooling helpers.
Scripts may propose soft outputs through typed gates.
Scripts may not directly mutate WorldModel, EventHistoryStore,
EpistemicStore, SocialInstitutionalStore, AppraisalRecordStore, or final Intent.
```

## Early Implementation Architecture Implications

The implementation should start with a narrow, boring authority core.

### Phase A: Domain Model And Identity

Research-backed target:

```text
core ids
  PersistentEntityId
  RuntimeEntityHandle
  EventRecordId
  CausalTransactionId
  ProcessInstanceId
  DefinitionId

store shells
  WorldStore
  RelationStore
  RuntimeControlStore
  EventHistoryStore
```

Do not choose Bevy/hecs as root architecture yet.

### Phase B: Causal Transaction Skeleton

Build the transaction shape before broad simulation features:

```text
CausalTransaction {
  id
  sim_time
  phase
  sequence
  source
  staged_reads
  staged_rng
  staged_mutations
  staged_events
  invalidation_set
}
```

The key early test is architectural:

```text
Can every hard mutation be represented as staged data before commit?
```

### Phase C: Minimal Typed Effect IR

Add only a small effect set:

```text
move_entity
transfer_entity
set_open_state
set_lock_state
apply_damage
add_condition
remove_condition
emit_signal
schedule_process
cancel_process
```

The first implementation should prove:

- role binding
- validation
- staged writes
- mandatory `EventRecord`
- derived-view invalidation
- random draw provenance where it affects committed outcome

### Phase D: Scheduler And Process State

Implement ordered, inspectable wakeups:

```text
SchedulerKey = (sim_time, phase, priority, sequence)
ProcessTick -> CausalTransaction
```

Do not represent processes as stackful coroutines.

### Phase E: Pack Compiler Core Without Final Syntax

Before final DSL syntax:

```text
DefinitionRegistry
EffectProgramIR
SemanticDeclarationIR
Verifier
Diagnostic
```

The first authoring format may be a temporary structured format or host-side
builder. The stable work is the IR and verifier.

### Phase F: Projections And Accelerators

Only after the core is clear:

- ECS projection for hot local concrete simulation
- graph snapshots for routing
- Datalog-like closure engine for derived views
- Salsa-like incremental queries for pack tooling
- scripting/Wasm plugin boundary for trusted or sandboxed extensions

## Candidate Library Summary

| Area | Recommended stance | Candidate references |
| --- | --- | --- |
| ECS | Later internal storage/projection, not authority root | [Bevy ECS](https://docs.rs/bevy/latest/bevy/ecs/index.html), [Flecs relationships](https://www.flecs.dev/flecs/md_docs_2Relationships.html) |
| Runtime handles | Good early candidate for in-memory handles | [slotmap](https://docs.rs/slotmap/) |
| Graph algorithms | Derived snapshots only | [petgraph](https://github.com/petgraph/petgraph) |
| Event model | Use principles, not framework spine | [Event Sourcing](https://www.martinfowler.com/eaaDev/EventSourcing.html), [Event-Driven Architecture](https://martinfowler.com/articles/201701-event-driven.html) |
| Scheduler vocabulary | Stable time order and tie-breakers | [ns-3 events](https://www.nsnam.org/docs/release/3.45/manual/html/events.html) |
| Process vocabulary | Process interaction reference, not coroutine save model | [SimPy process interaction](https://simpy.readthedocs.io/en/latest/topical_guides/process_interaction.html) |
| Parser | Best first candidate once syntax begins | [chumsky](https://docs.rs/chumsky/latest/chumsky/index.html) |
| Parser backup | Simple PEG candidate | [pest](https://docs.rs/pest/latest/pest/) |
| Parser generator | Later stable grammar candidate | [LALRPOP](https://lalrpop.github.io/lalrpop/) |
| Diagnostics | First renderer candidate | [miette](https://docs.rs/miette/latest/miette/) |
| Compiler diagnostics | Hand-tuned renderer candidate | [ariadne](https://docs.rs/ariadne/latest/ariadne/) |
| Incremental pack tooling | Later compiler/query engine | [Salsa overview](https://rustc-dev-guide.rust-lang.org/queries/salsa.html) |
| Datalog-like closure | Specific derived-view subproblems only | [datafrog](https://docs.rs/datafrog/latest/datafrog/) |
| Serialization | Likely default serialization framework | [Serde](https://docs.rs/serde/latest) |
| Scripting | Later trusted helper, not authority language | [Rhai](https://rhai.rs/) |
| Sandboxed plugins | Later explicit capability boundary | [Wasmtime](https://docs.wasmtime.dev/api/wasmtime/) |

## Risks

### Risk: ECS Becomes The Authority Layer By Accident

Symptom:

```text
systems mutate components directly
```

Failure:

- transaction log is incomplete
- replay diverges
- semantic evidence is missing
- actor projection leaks hard truth

Mitigation:

```text
Only CausalTransaction writes hard stores.
ECS views are rebuilt or updated from committed transactions.
```

### Risk: EventRecord Becomes A Command

Symptom:

```text
listener sees EventRecord and mutates world immediately
```

Failure:

- event order becomes hidden flow control
- debugging requires live listener tracing
- events stop being facts

Mitigation:

```text
EventRecord may trigger ReactionRequest, ProcessTick, semantic appraisal,
or proposal. New hard mutation still enters the transaction path.
```

### Risk: Typed Effect Program Becomes A Generic Script

Symptom:

```text
SetField(entity, key, value)
CallRustCallback(...)
RunScript(...)
```

Failure:

- verifier cannot prove stage permissions
- event contracts become optional
- replay and provenance weaken

Mitigation:

```text
Small typed primitive vocabulary.
No raw store mutation primitive.
Mandatory EventRecord contract.
```

### Risk: Semantic Rule Engine Becomes A Universal Authority

Symptom:

```text
appraisal rule writes memory
social rule selects intent
intent template commits CausalTransaction
```

Failure:

- layer boundaries collapse
- rules or adapters gain hidden mutation authority
- explanations become unreliable

Mitigation:

```text
SemanticDeclarationIR kind controls reads and outputs.
All durable writes pass through matching commit gates.
```

### Risk: Auditability Is Added Too Late

Symptom:

```text
committed results cannot be explained from transaction records, event records,
validation context, ordering, or relevant random draws
```

Failure:

- replay cannot be trusted
- debugging becomes anecdotal
- save/load divergence is hard to diagnose

Mitigation:

```text
transaction sequence numbers, event versions, accepted ordering, relevant
random draw records, and optional state hashes for selected checkpoints
```

### Risk: Compiler Tooling Locks Onto Early Syntax

Symptom:

```text
parser and syntax become the architecture
```

Failure:

- IR evolves painfully
- diagnostics are brittle
- packs cannot migrate cleanly

Mitigation:

```text
AST and source syntax are replaceable.
Typed IR, verifier, registry, and diagnostics are the stable center.
```

## Open Questions For Target Architecture

Questions to answer before detailed crate boundaries:

- Which store families are in the first `WorldModel` crate?
- What is the exact persistent id and runtime handle split?
- What relation families are mandatory in the first architecture draft?
- What is the minimal `CausalTransaction` struct?
- What event/version/upcast policy is needed before the first saved world?
- Which derived views must exist for the first physical scenarios?
- Which effect primitives are core and which are standard-library pack
  definitions?
- Does the first pack compiler use a temporary structured format before the
  DSL parser?
- Should incremental pack compilation be designed into APIs early but
  implemented later?
- What is the first acceptable diagnostic model?

## Stable Recommendations

- Build the engine around domain-owned `WorldModel`, not ECS.
- Use ECS later as private storage or materialized local simulation view.
- Use typed relation stores, not a universal fact graph.
- Use current materialized state plus transaction/event history, not pure event
  sourcing as the only state store.
- Make transaction sequence, event versions, accepted ordering, and relevant
  random draw provenance part of the first core model.
- Implement `ProcessInstance` as explicit serializable state.
- Implement `Typed Effect Program` as custom checked IR interpreted by the
  causal runtime.
- Implement semantic declarations with a custom stage-gated evaluator first.
- Use Datalog-like tools only for specific monotone derived-view problems.
- Use compiler tooling around IR/verifier/diagnostics before final syntax.
- Treat Rhai/Lua/Wasm as later tooling or plugin surfaces, not initial
  authority languages.
