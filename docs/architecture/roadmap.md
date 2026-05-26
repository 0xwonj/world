# Architecture Roadmap

## Status

Current architecture planning draft.

## Purpose

This document defines the order in which the engine architecture should be
stabilized before detailed crate design or implementation work begins.

It is not:

- a vertical slice plan
- a code task list
- a test plan
- a content roadmap
- a final crate boundary document

The goal is to prevent early implementation choices from accidentally
overriding the design's authority, causality, replay, actor-perspective, and
pack-authoring boundaries.

## Inputs

Primary design inputs:

- [Simulation Core](../design/simulation-core.md)
- [World Model](../design/world-model.md)
- [Causal Runtime](../design/causal-runtime.md)
- [Typed Effect Primitives](../design/typed-effect-primitives.md)
- [Time Model](../design/time-model.md)
- [Pack Authoring And Semantic Declarations](../design/pack-authoring-and-semantic-declarations.md)
- [Simulation Transition Compiler](../design/simulation-transition-compiler.md)
- [Capability, Affordance, And Actor Interface](../design/capability-affordance-and-actor-interface.md)
- [Perception And Observation](../design/perception-and-observation.md)
- [Epistemic State](../design/epistemic-state.md)
- [Social Institutional Model](../design/social-institutional-model.md)
- [Semantic Appraisal And Motivation](../design/semantic-appraisal-and-motivation.md)
- [Intent Templates And Planning](../design/intent-templates-and-planning.md)
- [Multi-Resolution Simulation](../design/multi-resolution-simulation.md)

Primary research inputs:

- [Engine Architecture Research Entry](../research/engine-architecture-entry.md)
- [Implementation Architecture And Library Survey](../research/implementation-architecture-and-library-survey.md)
- [World Representation / Query Model](../research/world-representation-query-model.md)
- [Causal Runtime / Action-Effect-Event](../research/causal-runtime-action-effect-event.md)
- [Time Model / Turn Scheduling](../research/time-model-and-turn-scheduling.md)
- [Runtime Pipeline Implementation Research](../research/runtime-pipeline-implementation-research.md)

## North Star

The implementation architecture should preserve this shape:

```text
pack definitions + current world/context + actor/policy choice
  -> checked representations
  -> actor-relative projection and semantic analysis
  -> selected or suggested intent
  -> Activity
  -> ActionRequest or ProcessInstance
  -> Typed Effect Program
  -> CausalTransaction
  -> EventRecord
  -> updated stores, invalidated views, observations, later meaning
```

The source of truth is not a renderer, client adapter, ECS world, rule engine,
scripting VM, or graph database.

The source of truth is the domain-owned simulation core:

```text
WorldModel
  authority-class stores
  typed relation families
  EventHistoryStore
  RuntimeControlStore
  DerivedViewRegistry
  QueryLayer
  CausalTransactionGate
```

## Architecture Principles

### Authority Before Behavior

Before implementing interesting features, the architecture must define which
layer is allowed to write which state.

Hard truth changes through `CausalTransaction`.

Non-hard state changes through their own accepted commit surfaces:

```text
AcceptedSocialUpdate
AcceptedChronologyRecord
AcceptedEpistemicUpdate
AcceptedAppraisalRecord
```

No adapter, rule evaluator, event listener, content script, ECS system, or
debugging tool may bypass the relevant commit surface.

### Domain Stores Before Accelerators

The root architecture is a typed hybrid `WorldModel`.

ECS, graph libraries, Datalog-like engines, scripting, and Wasm may be useful
later, but they are not the root source of truth.

Use them as:

- private storage optimizations
- materialized projections
- derived-view accelerators
- compiler/tooling helpers
- explicit plugin boundaries

### IR Before Syntax

Pack authoring should stabilize around checked IR and verifier behavior before
finalizing source syntax.

The important early artifacts are:

```text
Typed Effect Program IR
SemanticDeclarationIR
DefinitionRegistry
Verifier
Diagnostic model
Runtime indexes
```

The parser and surface syntax can evolve after those contracts are clear.

### Process Before Automation

Long-running work should be explicit `ProcessInstance` state.

Automation, abstract execution, travel, rituals, recovery, crafting, weather,
and passive processes should not become hidden loops of concrete action spam.

### Actor Perspective Before Policy Input

Actors and decision policies should receive actor-relative context. They should
not get omniscient hard truth and then be trusted to ignore it.

The actor-facing path should stay:

```text
CapabilitySet
  -> ActionRepertoire
  -> ObservedState / ObservedEvent
  -> EpistemicWorkingSet
  -> SocialContextView
  -> PerceivedAffordance
  -> turn input for policy or controller
```

## Roadmap Shape

The architecture should be stabilized in this order:

```text
0. Architecture framing
1. Authority and mutation core
2. WorldModel, identity, and relation storage
3. Typed Effect Program and transaction interpreter
4. Time, EventHistoryStore, replay, and ProcessInstance
5. Pack compiler, DefinitionRegistry, and diagnostics
6. Actor context and access-controlled query surfaces
7. Semantic decision middle-end
8. Multi-resolution execution
9. Accelerator and plugin boundaries
10. Crate boundary architecture
```

The ordering is not a claim that later systems are less important. It is a
dependency order. Later layers become safer when earlier authority and state
contracts are already stable.

## Stage 0: Architecture Framing

Purpose:

Establish what architecture documents own, and keep them separate from
research, design, and implementation plans.

Architecture outputs:

- this roadmap
- [Architecture Decisions](ADR.md)
- [Engine Architecture](engine.md)
- [Runtime Pipeline Architecture](runtime-pipeline.md)
- [Crate Boundary Architecture](crates.md)

Decisions to lock:

- `docs/research/` explains why
- `docs/design/` defines concepts and boundaries
- `docs/architecture/` defines implementation structure and dependency order
- future implementation planning owns concrete tasks

Do not solve yet:

- exact crate names
- final APIs
- final source syntax
- first playable scenario

## Stage 1: Authority And Mutation Core

Purpose:

Define the narrowest mutation authority before building feature systems.

Core shape:

```text
ActionRequest / ProcessTick / ReactionRequest
  -> bind roles and context
  -> validate
  -> Typed Effect Program
  -> CausalTransaction staging
  -> invariant checks
  -> atomic commit
  -> EventRecord append
```

Decisions to lock:

- what `CausalTransaction` owns
- what `EventRecord` proves
- how transaction staging works
- how validation failure differs from committed failure
- how hard mutation invalidates derived views
- how non-hard commit surfaces stay separate

Architecture risk if skipped:

Feature code will start writing directly to stores, and the transaction/event
model will become a logging afterthought instead of the source of causal
truth.

Exit condition:

The architecture can explain how any hard mutation enters the same commit
path, regardless of whether it came from a player command, NPC policy, passive
process, reaction, abstract tick, or resolution transition.

## Stage 2: WorldModel, Identity, And Relation Storage

Purpose:

Define what authoritative state is and how it can be queried without making
derived views or projections into hidden truth.

Core shape:

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

Decisions to lock:

- persistent identity versus runtime handles
- first typed relation families
- relation cardinality and inverse indexes
- authority class per store and relation family
- derived-view invalidation and provenance policy
- kernel, actor-relative, semantic, and debug query surfaces

Implementation-oriented conclusion:

Start with domain-owned typed stores and relation indexes. Do not make ECS the
root engine abstraction. ECS can later be an internal local-simulation storage
tool or materialized projection.

Architecture risk if skipped:

The engine will drift toward either raw ECS-as-ontology or an untyped fact
graph. Both make authority, provenance, and actor-relative access harder.

Exit condition:

The architecture can represent physical possession, social claim, actor
belief, location, containment, equipment, body state, and event history as
separate but queryable facts without collapsing them into one store.

## Stage 3: Typed Effect Program And Transaction Interpreter

Purpose:

Define how authored hard behavior becomes checked mutation without granting
packs arbitrary write authority.

Core shape:

```text
ActionDef / ProcessDef / ReactionDef
  -> typed roles and requirements
  -> Typed Effect Program IR
  -> effect permission checks
  -> transaction interpreter
  -> staged mutation and EventRecord contracts
```

Decisions to lock:

- minimal primitive effect families
- effect permission model
- required read/write declarations
- required `EventRecord` contracts
- RNG provenance in effects
- dry-run, validation, commit, replay, and explanation interpreter modes
- forbidden semantic effects inside hard effect programs

Implementation-oriented conclusion:

Use a custom checked IR and custom interpreter. Do not implement hard effects
as arbitrary callbacks, general scripts, or direct store writes.

Architecture risk if skipped:

Pack behavior will become bespoke resolver code. That makes replay,
inspection, migration, and authority checking much harder.

Exit condition:

The architecture can explain how a transfer, wound, lock change, signal
emission, process schedule, and process cancellation would each stage reads,
stage writes, emit records, and commit atomically.

## Stage 4: Time, EventHistoryStore, Replay, And ProcessInstance

Purpose:

Define durable temporal execution before higher-level activity and abstract
simulation rely on it.

Core shape:

```text
SchedulerKey = (sim_time, phase, priority, sequence)

ProcessInstance
  -> scheduled ProcessTick
  -> bind current context
  -> effect program or runtime primitive
  -> CausalTransaction
  -> EventRecord
  -> continue, sleep, interrupt, complete, or fail
```

Decisions to lock:

- simulation time representation
- same-time ordering
- scheduler phases
- replay levels, inputs, and outputs
- event schema/version policy
- transaction sequence and state hash policy
- process wait, interrupt, resume, and failure policy
- reservation lifecycle

Implementation-oriented conclusion:

Use explicit serializable process state. Do not make processes stackful
coroutines or hidden callbacks.

Architecture risk if skipped:

Travel, rest, rituals, crafting, recovery, passive hazards, and abstract
simulation will each invent their own timing and interruption model.

Exit condition:

The architecture can replay or explain a long-running activity across save/load
and resolution changes without relying on hidden runtime call stacks.

## Stage 5: Pack Compiler, DefinitionRegistry, And Diagnostics

Purpose:

Define how game-system packs enter the runtime as checked definitions rather
than arbitrary code.

Core shape:

```text
pack source or temporary structured input
  -> AST with spans
  -> declaration IR
  -> symbol resolution
  -> type checking
  -> authority and stage permission checks
  -> DefinitionRegistry
  -> runtime indexes
```

Decisions to lock:

- `Typed Effect Program IR`
- `SemanticDeclarationIR`
- `DefinitionRegistry`
- verifier boundaries
- diagnostic model
- pack namespace and dependency model
- runtime index shape
- which source syntax decisions remain deferred

Implementation-oriented conclusion:

Stabilize IR and verifier behavior before final source syntax. Parser and
diagnostic libraries are implementation choices around the compiler, not the
compiler's source of truth.

Architecture risk if skipped:

The first syntax or scripting convenience will become the architecture, and
authority checks will be retrofitted later.

Exit condition:

The architecture can describe how an action definition, process definition,
social rule, appraisal rule, intent template, and semantic view become
verified registry entries with source spans and diagnostics.

## Stage 6: Actor Context And Access-Controlled Query Surfaces

Purpose:

Define the non-omniscient bridge from hard truth to actor-facing action space.

Core shape:

```text
WorldModel / RuntimeControlStore / EpistemicStore / SocialInstitutionalStore
  -> permissioned QueryLayer
  -> CapabilitySet
  -> ActionRepertoire
  -> ObservedState / ObservedEvent
  -> EpistemicWorkingSet
  -> SocialContextView
  -> PerceivedAffordance
```

Decisions to lock:

- kernel query versus actor-relative query
- capability derivation inputs
- action repertoire derivation
- perceived affordance derivation
- observation projection inputs and outputs
- epistemic working set construction
- social context view construction
- invalid action and failed action feedback boundary

Architecture risk if skipped:

Decision policies will either receive omniscient truth or a shallow handwritten
action list disconnected from the simulation model.

Exit condition:

The architecture can explain why a wounded hand changes lockpicking validation
and scoring without hardcoding the action as globally unavailable.

## Stage 7: Semantic Decision Middle-End

Purpose:

Define how interpreted meaning biases decisions without directly executing
anything.

Core shape:

```text
ObservedEvent / ObservedState / EpistemicWorkingSet / SocialContextView
  -> AppraisalVariableSet
  -> Thought
  -> Pressure
  -> GoalPressure
  -> CandidateIntent
  -> IntentScore
  -> selected or suggested Intent
  -> Activity preparation
```

Decisions to lock:

- semantic declaration evaluator boundary
- social rule outputs versus appraisal outputs
- appraisal records versus durable memory/social updates
- `Pressure` and `GoalPressure` as non-commitment motivation
- candidate generation from intent templates
- `IntentScore` explanation inputs
- final intent selection gate
- `Activity` preparation and lifecycle boundary

Implementation-oriented conclusion:

Use a custom stage-gated semantic evaluator first. Datalog-like tools may help
with specific derived closures, but they should not own appraisal, scoring,
final intent, or state mutation.

Architecture risk if skipped:

Semantic rules will either become passive flavor text or gain too much hidden
authority over memory, social truth, and action execution.

Exit condition:

The architecture can explain a mentor death flowing from `EventRecord` to
observation, belief/context, `Thought`, `Pressure`, `GoalPressure`,
`CandidateIntent`, selected `Intent`, `Activity`, and either local
`ActionRequest` or abstract `ProcessInstance`.

## Stage 8: Multi-Resolution Execution

Purpose:

Define how the same causal world progresses at concrete, abstract, and
strategic resolution without duplicating runtime systems.

Core shape:

```text
Concrete:
  Intent -> Activity -> ActionRequest or ProcessInstance

Abstract:
  Intent -> Activity -> ProcessInstance

Strategic:
  Intent -> Activity -> region / faction / world process

All hard outcomes:
  ProcessTick or ActionRequest
    -> Typed Effect Program
    -> CausalTransaction
    -> EventRecord
```

Decisions to lock:

- resolution tier contracts
- promotion and demotion records
- resolution-aware location
- abstract event provenance
- materialization constraints
- what abstract simulation may summarize
- what abstract simulation must preserve
- how process identity survives resolution changes

Architecture risk if skipped:

Abstract simulation will either become fake narration with no causal force, or
it will secretly execute concrete action loops that cannot be inspected or
controlled.

Exit condition:

The architecture can explain abstract travel as `ProcessInstance` progress,
not repeated hidden movement requests, while still producing durable
consequences and promotion-ready provenance.

## Stage 9: Accelerator And Plugin Boundaries

Purpose:

Define where specialized libraries may help without taking authority.

Candidate boundaries:

```text
ECS:
  private hot local storage or materialized projection

Graph library:
  pathfinding and topology snapshots

Datalog-like engine:
  monotone derived closures and offline verification

Salsa-like incremental queries:
  pack compiler and editor tooling

Scripting:
  trusted tooling helper or declaration generator

Wasm:
  later sandboxed plugin boundary with explicit host capabilities
```

Decisions to lock:

- which accelerators may hold cached views
- how caches invalidate
- which accelerators may run during authoritative commit
- which accelerators are tooling-only
- plugin capability boundary
- script and proposal gates

Architecture risk if skipped:

Convenient libraries will become hidden sources of truth and will slowly erode
the transaction and authority model.

Exit condition:

The architecture can say for each library class whether it is authority,
projection, compiler tooling, runtime accelerator, or plugin boundary.

## Stage 10: Crate Boundary Architecture

Purpose:

Translate the stable architecture into candidate Rust crate boundaries.

This should happen after [Engine Architecture](engine.md) and the runtime
pipeline architecture are clear enough to avoid splitting crates around
temporary concepts.

Decisions to lock:

- public API versus internal modules
- core dependency direction
- definition/compiler/runtime separation
- feature flags and optional accelerators
- serialization and versioning ownership
- test/support crate boundaries if needed later

Do not lock too early:

- final module hierarchy
- exact package metadata
- parser crate dependencies
- ECS dependency
- scripting or Wasm dependency

Architecture risk if done too early:

The crate graph will encode speculative boundaries before the engine's actual
authority and runtime pipeline are settled.

Exit condition:

The crate boundary document can justify each crate by ownership and dependency
direction, not by surface feature grouping.

## Recommended Document Sequence

Write architecture documents in this order:

1. [Architecture Roadmap](roadmap.md)
2. [Architecture Decisions](ADR.md)
3. [Engine Architecture](engine.md)
4. [Runtime Pipeline Architecture](runtime-pipeline.md)
5. [Crate Boundary Architecture](crates.md)
6. [Project Conventions](project-conventions.md)
7. [Implementation Plan](implementation-plan.md), only after the crate
   boundary is stable enough

The engine architecture should answer:

- what the major runtime components are
- what each component owns
- how data moves between components
- what is authoritative versus derived
- what remains deferred

The runtime pipeline architecture should answer:

- how requests, processes, effects, transactions, observations, appraisal,
  intent, and multi-resolution execution connect
- where validation happens
- where durable state changes
- where proposals are accepted or rejected
- where derived views invalidate

The crate boundary architecture should answer:

- which crates are foundational
- which crates are optional
- which dependencies point inward or outward
- which crates expose public APIs
- which crates are only tooling or adapters

## Stable Direction

The current stable direction is:

- Build around domain-owned `WorldModel`, not ECS.
- Keep `CausalTransaction` as the hard mutation gate.
- Keep `EventRecord` as committed hard fact and evidence.
- Keep current state materialized; use event history for replay, audit,
  observation, and semantic evidence.
- Keep `Typed Effect Program` as a separate checked hard-mutation IR.
- Keep semantic declarations stage-gated and non-authoritative over hard truth.
- Keep `Intent` as commitment boundary.
- Keep `Activity` as temporal execution boundary.
- Keep `ActionRequest` as actor-facing concrete attempt boundary.
- Keep `ProcessInstance` as durable execution/progress state.
- Use abstract simulation through shared process execution, not hidden concrete
  action spam.
- Treat ECS, graph, Datalog-like engines, scripting, and Wasm as later
  projections, accelerators, tooling surfaces, or plugin boundaries.

## Deferred Until After This Roadmap

- final Rust crate names
- final parser and source syntax
- package manager and manifest format
- editor tooling
- concrete game-system packs
- first playable scenario
- vertical-slice implementation plan
- production persistence backend
- networked or multiplayer authority
- UI/client architecture
