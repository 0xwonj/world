# Archived Implementation Plan

Archive note:

This document is historical context for the earlier simulation-first RPG engine
roadmap. The active roadmap is
[Target Architecture Execution Roadmap](../target-architecture/implementation-roadmap.md).

## Status

Archived high-level implementation plan.

## Purpose

This document turns the architecture documents into a build order.

It answers:

```text
What should be implemented first?
What principles should guide early code?
What phase boundaries are stable enough to rely on?
What should stay flexible until the phase is planned in detail?
```

It is not:

- a task tracker
- a final API design
- a crate-by-crate module tree
- a schema or persistence format
- a parser/source syntax plan
- a vertical slice or playable scenario plan

The goal is to preserve the architecture's core boundaries while leaving
implementation details open where early code will teach the best shape.

## Inputs

Primary architecture inputs:

- [Architecture Decisions](../ADR.md)
- [Engine Architecture](../engine.md)
- [Runtime Pipeline Architecture](../runtime-pipeline.md)
- [Crate Boundary Architecture](../crates.md)
- [Project Conventions](../project-conventions.md)

Primary design inputs:

- [Simulation Core](../../design/simulation-core.md)
- [World Model](../../design/world-model.md)
- [Causal Runtime](../../design/causal-runtime.md)
- [Typed Effect Primitives](../../design/typed-effect-primitives.md)
- [Standard World Library And Primitive Semantics](../../design/standard-world-library.md)
- [Time Model](../../design/time-model.md)
- [Pack Authoring And Semantic Declarations](../../design/pack-authoring-and-semantic-declarations.md)
- [Simulation Transition Compiler](../../design/simulation-transition-compiler.md)
- [Intent Templates And Planning](../../design/intent-templates-and-planning.md)

Primary research inputs:

- [Implementation Architecture And Library Survey](../../research/implementation-architecture-and-library-survey.md)
- [Runtime Pipeline Implementation Research](../../research/runtime-pipeline-implementation-research.md)

## Planning Rules

The plan should lock what is already architecturally stable:

- domain-owned core
- materialized stores plus transaction/event history
- `CausalTransaction` as the hard mutation gate
- `Typed Effect Program` as checked hard-mutation IR
- standard world library as the reusable primitive-definition and trusted
  primitive-semantics layer outside runtime core
- `RuntimeControlUpdate` as runtime control-state update boundary
- `Intent` as commitment boundary
- `Activity` as temporal execution boundary
- `ActionRequest` as actor-facing attempt boundary
- `ProcessInstance` as durable execution/progress frame
- semantic decision work as proposal, appraisal, choice, or preparation, not
  hard mutation
- accelerators as projections, adapters, or tooling surfaces

The plan should not lock details that are better decided during phase design:

- exact Rust module tree
- exact public function names
- exact storage backend
- exact serialization format
- exact parser and source syntax
- exact ECS, graph, Datalog, scripting, or Wasm integration
- exact diagnostics renderer
- final gameplay content packs
- exact first standard primitive bundle and concrete taxonomies
- first playable scenario

Each implementation phase should begin with a short local design note if the
next step needs API, storage, or test-shape choices that this document
intentionally leaves open.

## Implementation Principles

### Authority First

Build mutation authority before expressive gameplay systems.

The early engine must make it difficult to mutate hard truth outside
`CausalRuntime` and difficult to persist runtime control state outside the
runtime-control gate.

### Type-Rich Core

Use concrete types, enums, and newtyped identifiers before broad public traits.

Avoid generic framework shapes that hide authority:

```text
trait RuntimeSystem {
  fn run(&mut self, world: &mut WorldModel);
}
```

Prefer narrow role-specific surfaces:

```text
KernelQuery
ActorRelativeQuery
SemanticContextQuery
TransactionStager
DefinitionLookup
DiagnosticSink
```

### Sync Core, Host Flexibility

Keep the core simulation runtime synchronous.

Async IO, editor integration, networking, asset loading, and plugin execution
belong at host or adapter boundaries unless a later phase proves otherwise.

### Outcomes Are Not Errors

Gameplay outcomes are domain results. Infrastructure failures are Rust
errors.

The implementation should keep room for shapes like:

```text
Result<RuntimeOutcome, RuntimeError>
```

without forcing exact enum variants in this plan.

### Replay Without Overfitting To Determinism

The baseline is auditability, explainability, and save/load continuity.

When ordering, RNG, batching, or concurrency affects committed results, record
enough provenance for the declared replay level. Do not make full deterministic
command replay a global requirement.

### Delay Accelerators

ECS, graph indexes, Datalog-like closures, scripting, Wasm plugins, and
incremental query engines are useful later. They should not become early
sources of truth.

## Agent Execution Workflow

This section is the durable workflow contract for long-running agent runs.
`AGENTS.md` should stay as a short repository router. A `/goal` prompt should
name the objective and stopping condition, then refer back to this plan instead
of copying architecture details.

For each implementation phase:

1. Research and context review.

   Read `AGENTS.md`, inspect the current architecture/design/research indexes,
   and use `rg` to find the relevant design owner. Use external research only
   when it materially improves an important implementation choice.

2. Local phase plan.

   Briefly state what this phase should lock, what should stay open, and what
   is out of scope. Treat phase boundaries as ordering guidance, not rigid
   slices. Co-design adjacent concepts when that avoids artificial seams or
   predictable rework.

3. Implement.

   Keep code minimal, domain-shaped, and aligned with crate dependency
   direction. Prefer concrete types and narrow APIs over broad framework traits.
   Do not introduce optional accelerators or backend commitments just to make an
   early phase feel complete.

4. Verify.

   Run focused checks while developing and the relevant workspace checks before
   advancing. Add targeted tests when they protect behavior, authority
   boundaries, or cross-crate contracts.

5. Review.

   Review from these perspectives:

   - architecture boundaries and dependency direction
   - Rust/API shape, ownership, and public surface size
   - runtime correctness, mutation authority, process/control-state boundaries,
     auditability, and explainability
   - complexity, over-abstraction, and future expression limits

   Use separate review passes or subagents when available and useful, especially
   for phases that define shared runtime authority.

6. Fix and hand off.

   Fix real issues, drop non-issues, and leave a concise phase note covering
   changes, verification, review findings, fixes, and intentionally deferred
   work before moving on.

Execution intensity should vary by phase. Lightweight substrate phases can move
quickly. Definition, world-model, causal-runtime, and process/runtime-control
phases should spend more time on context review, local planning, and review.

Stop and ask before changing core architecture boundaries, crate dependency
direction, persistence/backend choices, major dependency policy, optional
accelerator adoption, or broad documentation structure.

## Phase Overview

Recommended order:

```text
Phase 0: Workspace Foundation
Phase 1: Core Domain Substrate
Phase 2: Checked Definition Model
Phase 3: World Model And Query Surfaces
Phase 4: Causal Mutation Waist
Phase 5: Runtime Control, Time, And Process
Phase 6: Standard World Library And Primitive Semantics
Phase 7: Actor Context Projection
Phase 8: Semantic Decision Middle-End
Phase 9: Authoring And Verification
Phase 10: Engine Facade And Integration
Phase 11: Scenario And Adapter Planning
```

The order matters. Later phases can be implemented with more confidence when
authority, identity, definitions, state, and mutation gates already exist.

## Phase 0: Workspace Foundation

Goal:

Create the Rust workspace shape and project conventions without implementing
engine behavior.

Primary focus:

- Cargo workspace
- crate skeletons from [Crate Boundary Architecture](../crates.md)
- workspace dependency policy from [Project Conventions](../project-conventions.md)
- lint and formatting baseline
- minimal documentation for how to run checks

Crates touched:

```text
world-core
world-defs
world-model
world-runtime
world-standard
world-standard-runtime
world-context
world-decision
world-authoring
world-engine
```

Lock now:

- initial workspace membership for foundational crates
- dependency direction
- root lint/profile/dependency policy
- no optional accelerator dependency in foundational crates

Leave open:

- exact module trees
- exact APIs
- later boundary crates when a later phase makes a new stable layer explicit
- exact CI matrix
- exact test-support crate

Exit condition:

The workspace can compile empty or near-empty crates in the intended dependency
direction, and dependency policy prevents obvious reverse edges.

## Phase 1: Core Domain Substrate

Goal:

Establish shared vocabulary that every later crate can depend on without
creating cycles.

Primary focus:

- newtyped ids
- time primitives
- version anchors
- replay level
- provenance keys
- authority class tags
- stable ordering keys

Crate focus:

```text
world-core
```

Lock now:

- ID categories that are architecturally distinct
- durable id versus runtime handle distinction
- shared ordering and version anchor vocabulary

Leave open:

- final numeric representation
- persistence encoding
- complete id list
- exact replay implementation

Exit condition:

Later crates can refer to core identities, time, provenance, version, and
replay vocabulary without inventing local substitutes.

## Phase 2: Checked Definition Model

Goal:

Create the runtime-facing checked definition model before live runtime
execution.

Primary focus:

- definition ids and registry shape
- `EffectPrimitiveDef`
- `ActionDef`
- `ProcessDef`
- `Typed Effect Program` definition model
- semantic declaration definition model
- definition version anchors

Crate focus:

```text
world-defs
```

Lock now:

- checked definitions are separate from source syntax
- runtime consumes normalized definitions
- authoring produces definitions, not callbacks with hidden authority
- primitive signatures are checked data, separate from runtime semantics

Leave open:

- final DSL syntax
- parser choice
- full verifier implementation
- exact effect operation set
- exact standard primitive bundle
- exact semantic declaration format

Exit condition:

Runtime and model crates can look up checked definitions without depending on
pack source parsing or authoring diagnostics.

## Phase 3: World Model And Query Surfaces

Goal:

Build materialized state ownership and read surfaces before mutation runtime.

Primary focus:

- `WorldModel`
- authority-class store families
- relation store families
- `EventHistoryStore`
- `RuntimeControlStore`
- `SocialInstitutionalStore`
- `ChronologyStore`
- `EpistemicStore`
- `AppraisalRecordStore`
- `DerivedViewRegistry`
- invalidation package vocabulary and derived-view staleness states
- `QueryLayer`

Crate focus:

```text
world-model
```

Small `world-core` additions are allowed only when the model needs new durable
ids or ordering values. `world-defs` should remain a consumed input, not a
Phase 3 implementation target.

Lock now:

- stores are hosted by `WorldModel`
- hard, non-hard, actor-relative, and runtime-control state stay distinct
- queries are read surfaces, not mutation paths
- `EventHistoryStore` is a committed-history facade and does not produce
  `EventRecord`s
- `RuntimeControlStore` hosts durable runtime-control state and read surfaces;
  runtime-control update validation and lifecycle belong to Phase 5
- social, chronology, epistemic, and appraisal stores are separate containers;
  their domain-specific validation and commit gates stay with their authority
  owners
- `DerivedViewRegistry` can consume invalidation vocabulary and mark staleness,
  but cache policy and eager recomputation strategy stay open
- public writes are intentionally absent from `world-model` in this phase;
  local write helpers are crate-internal storage/fixture plumbing, not public
  authority
- model-side apply surfaces, when introduced later, are narrow receivers for
  accepted packages; construction, validation, and causal commit authority stay
  outside `world-model`
- committed and accepted package forge prevention is not solved by public
  constructors in this phase. It relies on private fields, narrow APIs, and
  later runtime or engine facade ownership.
- actor-relative and semantic query surfaces carry explicit scope and authority
  labels even before full actor-context projection is implemented

Leave open:

- `CausalTransaction` construction, transaction staging, effect interpretation,
  and committed hard package production
- runtime-control update semantics, scheduler drain behavior, and process
  lifecycle
- social, chronology, epistemic, and appraisal commit-gate validation rules
- full actor context assembly and semantic context derivation
- concrete storage backend
- index layout
- cache strategy
- persistence format
- exact query API names

Exit condition:

The model can hold current state, committed history, runtime control state, and
read-only query surfaces without exposing public write authority, while later
runtime phases retain a clear place to introduce accepted package receivers.

## Phase 4: Causal Mutation Waist

Goal:

Establish the hard mutation gate before process, semantic, or gameplay
expressiveness.

Primary focus:

- `ActionRequest` binding shape
- `TypedEffectInterpreter`
- transaction staging
- invariant check boundary
- atomic commit package
- `TransactionRecord`
- `EventRecord`
- invalidation package
- runtime outcome versus runtime error split

Crate focus:

```text
world-runtime
```

Lock now:

- hard mutation passes through `CausalTransaction`
- effect handling receives staging APIs, not raw store mutation authority
- committed hard outcomes append `EventRecord`s
- commit publishes state updates and invalidation as one accepted package
- `world-runtime` owns `CausalTransactionGate` semantics, transaction staging,
  invariant checks, and committed package construction; `world-model` only
  provides the storage and narrow application surface needed by accepted
  commits
- model-side apply surfaces are accepted-package receivers only; they do not
  construct, validate, or partially publish causal commits

Leave open:

- complete effect vocabulary
- standard world library crate split and complete primitive semantics registry
- full validation rule set
- exact transaction builder API
- storage optimization
- deterministic command replay details

Exit condition:

There is one visible hard-mutation waist, and callers cannot commit hard world
changes by directly mutating stores or bypassing the accepted-package receiver.

## Phase 5: Runtime Control, Time, And Process

Goal:

Add durable time, scheduling, process progress, and runtime control state on
top of the mutation waist.

Primary focus:

- `ScheduledWakeup`
- scheduler drain
- `DrainOutcome`
- `ProcessInstance`
- `ProcessTick`
- `ProcessTransition`
- `ActivityTransition`
- `RuntimeControlUpdate`
- `AcceptedRuntimeControlUpdate`
- reservation acquire/release boundary
- interruption, resume, completion, and failure state

Crate focus:

```text
world-runtime
```

Lock now:

- `ProcessInstance` is serializable state, not a saved coroutine stack
- processes compute transitions rather than mutating `RuntimeControlStore`
  directly
- abstract execution uses process progress, not hidden concrete action spam
- scheduler has ordering, provenance, and drain guard surfaces
- Phase 5 uses the Phase 3 `RuntimeControlStore` as storage, but owns
  `RuntimeControlUpdate` validation, accepted update construction, scheduler
  semantics, and process lifecycle rules
- runtime control keeps its two commit lanes visible: transaction-coupled
  `RuntimeControlUpdate` for control changes that must be atomic with a hard
  outcome, and control-only `AcceptedRuntimeControlUpdate` for durable control
  changes that do not themselves mutate hard truth

Leave open:

- exact process state enum
- exact scheduler data structure
- exact reservation conflict algorithm
- exact budget values
- exact process definition schema

Exit condition:

The runtime can represent long-running work, schedule wakeups, update runtime
control state through gates, and explain why process work continued, paused,
failed, or completed.

## Phase 6: Standard World Library And Primitive Semantics

Goal:

Move reusable world-simulation primitive vocabulary out of the runtime core and
make primitive definitions and trusted semantics explicit before actor context
depends on standard physical grammar.

Primary focus:

- `EffectPrimitiveDef` completion where Phase 2 left room
- `EffectOp` references to checked primitive definitions rather than only raw
  operation names
- descriptor-based primitive definitions in `world-defs`
- `PrimitiveSemanticsRegistry`
- standard primitive definition bundle
- standard primitive semantics installer
- runtime bootstrap wiring for definitions and semantics
- definition/semantics compatibility checks
- migration of Phase 4 seed primitive handling into the registry shape

Crate focus:

```text
world-defs
world-runtime
world-standard
world-standard-runtime
world-engine as wiring only if needed
```

If `world-standard` or `world-standard-runtime` were not created during the
workspace foundation pass, introduce them here with narrow empty surfaces before
moving semantics out of `world-runtime`.

Lock now:

- runtime owns semantics lookup, staging capabilities, and causal authority
- the standard world library owns reusable primitive definitions and trusted
  primitive semantics outside the runtime core
- `world-runtime` does not depend on `world-standard` or
  `world-standard-runtime`
- ordinary game-system packs compose installed primitives rather than
  receiving raw staging callbacks
- missing primitive semantics fails load or execution clearly instead of
  falling back to generic field mutation
- process effect programs are definition-verified here, but process-program
  execution remains a separate runtime path
- actor-context code may depend on pure standard vocabulary, but not on
  runtime semantics installers

Leave open:

- final standard primitive set
- final damage, wound, condition, material, resource, signal, and field
  taxonomies
- trusted extension package loading/signing policy
- Wasm, scripting, or sandboxed primitive semantics
- final pack manifest and dependency-resolution policy

Exit condition:

The engine can distinguish runtime mechanism from standard world semantics:
checked effect programs call installed primitive definitions, runtime dispatch
uses trusted semantics through staging capabilities, and reusable standard
primitive behavior no longer needs to grow inside the runtime core.

## Phase 7: Actor Context Projection

Goal:

Create actor-relative readable context after state and runtime authority are
stable.

Primary focus:

- observation projection
- actor-facing context assembly
- capability derivation
- action repertoire derivation
- perceived affordance derivation
- epistemic working set projection
- social context view projection
- context provenance

Crate focus:

```text
world-context
```

Lock now:

- context is actor-relative, not omniscient
- context generation does not mutate hard truth
- context outputs are snapshots, ids, read handles, or derived views
- derived context invalidates from model/runtime invalidation packages

Leave open:

- exact projection algorithms
- cache policy
- complete capability vocabulary
- complete affordance vocabulary
- social/epistemic storage optimization

Exit condition:

Decision code can receive actor-visible context without privileged world-store
access or direct mutation authority.

## Phase 8: Semantic Decision Middle-End

Goal:

Add semantic interpretation and intent preparation after actor context exists.

Primary focus:

- appraisal variable sets
- `Thought`
- `Pressure`
- `GoalPressure`
- candidate intent generation
- `IntentScore`
- selected or suggested `Intent`
- activity preparation metadata
- decision explanation

Crate focus:

```text
world-decision
```

Lock now:

- appraisal does not execute
- memory, social context, and appraisal do not choose actions directly
- pressure biases later selection but does not mutate state
- final selected or suggested intent remains a commitment/control boundary
- hard outcomes still require later runtime validation and commit

Leave open:

- exact scoring model
- exact intent template syntax
- exact AI integration point
- complete semantic rule vocabulary
- scheduling policy for decision work

Exit condition:

The engine can produce explainable decision intermediates and selected or
suggested intents without giving decision code hard mutation authority.

## Phase 9: Authoring And Verification

Goal:

Introduce pack authoring and diagnostics after the runtime-facing definition
model is stable.

Primary focus:

- pack compiler structure
- definition verification
- semantic declaration verification
- typed effect verification
- source diagnostics
- definition registry construction

Crate focus:

```text
world-authoring
```

Lock now:

- authoring produces checked definitions
- parser/source syntax remains isolated from runtime crates
- diagnostics preserve enough source and stage context
- authored content cannot become arbitrary Rust callbacks with mutation
  authority

Leave open:

- final DSL syntax
- parser library
- exact diagnostic renderer
- incremental compilation
- editor tooling

Exit condition:

Definitions can be constructed and checked through an authoring path without
leaking parser or source-diagnostic dependencies into runtime authority crates.

## Phase 10: Engine Facade And Integration

Goal:

Provide the public engine-facing orchestration layer once inner boundaries are
usable.

Primary focus:

- session lifecycle
- pack loading and registry wiring
- world creation/load/save coordination
- controller input surface
- scheduler drain entry point
- context/decision/runtime orchestration
- inspection entry points

Crate focus:

```text
world-engine
```

Lock now:

- application users enter through facade APIs
- facade orchestrates decision and runtime without merging their authority
- host/adapters live outside the core mutation waist
- facade may re-export stable types selectively

Leave open:

- exact application API names
- async host integration
- persistence backend
- UI/editor integration
- plugin host

Exit condition:

An application or later tool can create a session, load checked definitions,
hold world state, submit inputs, drain runtime work, inspect outcomes, and save
or explain state through stable high-level surfaces.

## Phase 11: Scenario And Adapter Planning

Goal:

Plan the first concrete scenario and optional adapter work after the core
architecture has a real implementation shape.

Primary focus:

- first buildable scenario plan
- persistence backend decision
- test-support crate decision
- optional ECS/graph/Datalog/scripting/Wasm adapter decision
- benchmark and fuzzing strategy
- CI command matrix

For deep concrete game systems, ECS-backed local projections are an expected
adapter candidate once the core authority path exists. They should accelerate
simulation work without becoming the source of truth.

Lock then:

- concrete acceptance scenarios
- feature flag matrix
- adapter boundaries
- persistence and migration policy
- test levels

Leave open until then:

- playable scenario details
- UI/client architecture
- editor architecture
- standard game-system packs
- network or multiplayer authority

Exit condition:

The project has enough implemented substrate to choose a scenario and adapter
plan based on real code rather than speculative crate boundaries.

## Global Exit Criteria

This high-level plan has served its purpose when:

- the workspace dependency direction matches [Crate Boundary Architecture](../crates.md)
- hard mutation authority is visibly centralized in runtime code
- current state, event history, runtime control state, and accepted non-hard
  state are represented as distinct authority classes
- checked definitions are consumed by runtime without depending on parser
  internals
- standard world primitive definitions and trusted semantics stay outside the
  runtime core while executing only through runtime staging capabilities
- actor context and semantic decision code cannot bypass runtime authority
- long-running and abstract work use `ProcessInstance` and `ProcessTick`
  rather than hidden concrete action spam
- optional accelerators remain adapters or projections
- implementation-specific choices are documented in local phase plans instead
  of being guessed globally here

## Deferred Beyond This Plan

These topics should not be solved by this document:

- exact parser and DSL syntax
- exact persistence backend and migration format
- exact ECS, graph, Datalog, scripting, or Wasm adapter
- first playable scenario
- game-system standard library packs
- final standard world taxonomies beyond the first reusable primitive surface
- UI/editor/client architecture
- multiplayer/network authority
- production performance budget
- release packaging

## Summary

The implementation order is:

```text
workspace
  -> core vocabulary
  -> checked definitions
  -> world model and query surfaces
  -> causal mutation waist
  -> runtime control, time, and process
  -> standard world library and primitive semantics
  -> actor context
  -> semantic decision middle-end
  -> authoring and verification
  -> engine facade
  -> scenario and adapter planning
```

This sequence favors foundational architecture over early feature breadth. It
should be refined phase by phase as code reveals the right internal APIs, but
it should not relax the authority boundaries that make the engine coherent.
