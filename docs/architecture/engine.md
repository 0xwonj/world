# Engine Architecture

## Status

Frozen legacy planning draft.

Superseded where it conflicts with
[`target-architecture/system-architecture.md`](target-architecture/system-architecture.md).

## Purpose

This document describes the target engine structure before detailed Rust crate
boundaries are chosen.

It answers:

```text
What are the major engine components?
What does each component own?
What does each component explicitly not own?
How should data and authority flow between them?
```

It is not:

- a crate boundary document
- an implementation plan
- a vertical slice plan
- a final API reference
- a parser or source syntax design
- a game-system content plan

The goal is to make the target architecture clear enough that later crate
boundaries can follow ownership and dependency direction instead of temporary
feature grouping.

## Inputs

Primary design inputs:

- [Simulation Core](../design/simulation-core.md)
- [Engine Core And Game System Boundary](../design/engine-core-and-game-system-boundary.md)
- [World Model](../design/world-model.md)
- [Causal Runtime](../design/causal-runtime.md)
- [Typed Effect Primitives](../design/typed-effect-primitives.md)
- [Standard World Library And Primitive Semantics](../design/standard-world-library.md)
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

Primary architecture inputs:

- [Architecture Roadmap](roadmap.md)
- [Architecture Decisions](ADR.md)
- [Implementation Architecture And Library Survey](../research/implementation-architecture-and-library-survey.md)

## Architectural Thesis

The engine is a domain-owned simulation runtime with checked authoring inputs.

```text
PackCompiler / DefinitionRegistry
  supplies checked definitions

StandardWorldLibrary
  supplies reusable primitive definitions and trusted primitive semantics

WorldModel
  owns authoritative and holder-relative store families

QueryLayer
  exposes typed, permissioned read surfaces

CausalRuntime
  stages and commits hard world mutation through installed primitive semantics

ProcessRuntime / Scheduler
  advances durable time and long-running work

ActorContextPipeline
  projects non-omniscient actor-facing context

SemanticDecisionMiddleEnd
  interprets context into pressure, candidates, and selected or suggested
  intent without bypassing runtime boundaries

ResolutionRuntime
  controls concrete, abstract, and strategic execution detail
```

Specialized libraries may appear behind these components, but they do not own
truth or mutation authority.

## Top-Level Shape

```text
Authoring side:

pack source or structured definitions
  -> PackCompiler
  -> DefinitionRegistry

Runtime side:

WorldModel-hosted stores
  including RuntimeControlStore and EventHistoryStore
  -> QueryLayer
  -> ActorContextPipeline
  -> SemanticDecisionMiddleEnd
  -> Intent
  -> Activity
  -> ActionRequest or ProcessInstance
  -> CausalRuntime
      request binding / validation
      TypedEffectInterpreter over transaction staging APIs
      CausalTransaction commit
  -> EventRecord + store updates + invalidation
```

The architecture is not a single planner and not an ECS application with
gameplay systems directly mutating components.

The component names in this document are logical roles. They are not final
crate names. Early implementation may co-locate tightly coupled roles as long
as authority and dependency boundaries remain visible.

## Architecture Planes

The component map can be read as six cooperating planes:

```text
Authoring:
  PackCompiler, DefinitionRegistry, StandardWorldLibrary

State and query:
  WorldModel, QueryLayer, DerivedViewRegistry

Execution and commit:
  CausalRuntime including TypedEffectInterpreter, EventHistoryStore

Time and process:
  Scheduler, ProcessRuntime, ResolutionRuntime

Actor and decision:
  ObservationPipeline, ActorContextPipeline, SemanticDecisionMiddleEnd

Inspection and adapters:
  InspectionSurface, AcceleratorAdapters, EngineHost
```

The planes are a readability aid, not an extra runtime hierarchy.

## Component Map

```text
EngineHost
  coordinates runtime services and external adapters

DefinitionRegistry
  checked pack, action, process, effect, semantic, and content definitions

StandardWorldLibrary
  reusable RPG-world grammar, standard primitive definitions, and trusted
  semantics installers

WorldModel
  authority-class stores, relation families, identity, derived-view registry

QueryLayer
  kernel, actor-relative, semantic, and debug read surfaces

Scheduler
  simulation time agenda and wakeups

ProcessRuntime
  ProcessInstance lifecycle, interruption, progress, reservation interaction

CausalRuntime
  ActionRequest / ProcessTick / ReactionRequest validation and transaction
  staging

TypedEffectInterpreter
  internal causal-runtime role for checked effect IR execution over
  transaction staging APIs

PrimitiveSemanticsRegistry
  runtime-owned lookup table for trusted primitive handlers installed by the
  standard world library or trusted extensions

EventHistoryStore
  TransactionRecord, EventRecord, audit data, history cursor, version anchors

VersioningPolicy
  cross-cutting schema, definition, and save/version anchors

ObservationPipeline
  actor-relative ObservedState and ObservedEvent projection

ActorContextPipeline
  CapabilitySet, ActionRepertoire, EpistemicWorkingSet, SocialContextView,
  PerceivedAffordance

SemanticDecisionMiddleEnd
  semantic views, appraisal, pressure, candidate intent, scoring, intent
  selection/suggestion, activity preparation

ResolutionRuntime
  concrete, abstract, and strategic execution detail; promotion, demotion, and
  materialization boundaries

InspectionSurface
  provenance, explanation, diagnostics, replay/audit views

AcceleratorAdapters
  optional ECS, graph, Datalog-like, incremental query, scripting, or plugin
  adapters behind explicit boundaries
```

## EngineHost

Owns:

- runtime service wiring
- top-level world/session lifecycle
- external adapter entry points
- high-level orchestration of load, tick, save, and inspect operations

Does not own:

- world truth
- pack definition semantics
- typed effect semantics
- primitive semantics implementations
- transaction commit rules
- actor-relative visibility

The host should coordinate components, not become the hidden place where
gameplay rules live.

Typical responsibilities:

```text
load checked definitions
load or create world state
route controller input to runtime request surfaces
advance scheduled work
expose inspection and save/load operations
```

## DefinitionRegistry

Owns:

- checked `ActionDef`
- checked `ProcessDef`
- checked `ReactionDef`
- checked primitive effect definitions
- checked `Typed Effect Program` definitions
- checked semantic declaration definitions
- content schemas and ids
- pack metadata needed by runtime lookup
- source span and diagnostic references for tooling

Does not own:

- current world state
- committed events
- process progress
- actor memory
- final runtime validation
- source syntax details after compilation

The registry is the runtime-facing output of pack compilation.

```text
pack source or structured authoring input
  -> parse/load
  -> declaration AST
  -> typed declaration IR
  -> verification
  -> DefinitionRegistry
```

The registry should support lookup by runtime need:

```text
action_defs_by_schema
process_defs_by_kind
effect_programs_by_definition
social_rules_by_trigger
appraisal_rules_by_focus
intent_templates_by_pressure_or_goal
semantic_views_by_input
```

## StandardWorldLibrary

Owns reusable world-simulation definition bundles and primitive semantics
installers.

Owns:

- standard primitive definitions
- reusable event family definitions
- reusable physical/topological value categories
- trusted primitive semantics installers
- version anchors for bundled standard primitives

Does not own:

- `CausalTransaction` authority
- primitive dispatch registry ownership
- pack source parsing
- actor decision
- concrete game taxonomies and balance

The standard world library is a layer between runtime mechanism and
game-system packs. Runtime owns staging, dispatch, commit, and replay. The
standard library supplies common primitives such as transfer, damage,
condition, signal, field, and process hooks without making ordinary packs
executable mutation callbacks.

## PackCompiler

Owns:

- source loading or temporary structured definition loading
- symbol resolution
- type checking
- authority and stage permission checking
- event/effect contract checking
- diagnostic production
- registry construction

Does not own:

- hard world mutation
- active runtime decisions
- process ticking
- actor observation
- save/load state migration policy beyond definition/version metadata

The first implementation does not need final surface syntax. It needs stable
IR, verifier behavior, diagnostics, and registry shape.

Compiler phases:

```text
source or structured input
  -> AST with spans
  -> raw declarations
  -> symbol table
  -> typed definitions
  -> authority checks
  -> domain-specific verification
  -> DefinitionRegistry
```

## WorldModel

Owns storage families and indexes. It does not by itself grant write authority.

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

`WorldModel` hosts these store families. Components such as
`EventHistoryStore` and `RuntimeControlStore` may expose focused APIs, but they
do not become sibling truth owners that can bypass the relevant commit gate.
`CausalTransactionGate` denotes the model-adjacent apply boundary; the causal
runtime owns transaction construction, validation, and accepted hard commit
authority.
Before that commit boundary is implemented, `WorldModel` remains a read-first
substrate: public callers can create a model, inspect read-only stores, and use
query surfaces, but they cannot construct committed records or apply accepted
packages through public model APIs.

Owns:

- persistent ids and runtime handles
- typed hard state stores
- typed relation families
- store indexes
- derived-view registration and invalidation hooks
- store-family boundaries
- query plumbing

Does not own:

- the meaning of every domain record
- the validation semantics of every action
- direct mutation APIs for arbitrary callers
- actor-facing access policy by itself

Write surfaces:

```text
CausalTransaction:
  writes hard truth stores and EventHistoryStore

AcceptedSocialUpdate:
  writes social/institutional soft truth

AcceptedChronologyRecord:
  writes chronology/world-context soft truth

AcceptedEpistemicUpdate:
  writes holder-relative actor truth

AcceptedAppraisalRecord:
  writes appraisal/motivation records
```

The WorldModel may later use ECS-like storage internally. That remains a
storage detail, not the public ontology.

## Identity Model

The engine should separate durable identity from loaded-runtime handles.

```text
PersistentEntityId:
  stable across save, history, memory, social references, and event records

RuntimeEntityHandle:
  efficient handle for currently loaded state

DefinitionId:
  stable checked definition reference

EventRecordId:
  committed event reference

CausalTransactionId:
  committed transaction reference

ProcessInstanceId:
  durable process/progress reference
```

Rules:

- durable records should refer to persistent ids or definition ids
- runtime handles may be optimized for loaded state
- relation stores should record authority class and provenance source
- no accelerator-specific entity id should leak into durable records

## QueryLayer

Owns typed read surfaces over store families.

Read classes:

```text
KernelQuery:
  privileged runtime validation and effect execution reads

ActorRelativeQuery:
  reads filtered by actor access, perception, memory, and current context

SemanticContextQuery:
  reads prepared context for social, appraisal, and intent passes

DebugQuery:
  privileged inspection and provenance reads
```

Does not own:

- write authority
- final intent selection
- durable truth records
- semantic meaning by itself

The QueryLayer should make read authority visible in API shape. A pass should
not accidentally use omniscient hard truth when it needs actor-relative input.

## DerivedViewRegistry

Owns:

- derived-view definitions
- dependency declarations
- invalidation hooks
- cache ownership metadata
- provenance requirements for derived facts

Does not own:

- durable authoritative facts
- direct mutation of source stores
- actor-relative access exceptions

Examples:

```text
containment closure
reachability
visibility support views
passability
material exposure
capability inputs
social relevance views
semantic context views
```

Derived views may be cached. Cached views must remain rebuildable or
explainable from their declared inputs.

## Scheduler

Owns:

- simulation-time agenda
- scheduled wakeups
- ordering metadata for committed wakeups
- wakeup dispatch to runtime components

Does not own:

- process local state
- transaction commit semantics
- actor decision policy
- event meaning

Core shape:

```text
ScheduledWakeup {
  sim_time
  phase
  priority
  sequence
  target
  reason
}
```

The scheduler should preserve enough ordering information that committed
outcomes can be inspected and explained. Selected subsystems may require
canonical replay behavior, but global deterministic recomputation is not the
default architecture requirement.

## ProcessRuntime

Owns `ProcessInstance` lifecycle.

Owns:

- process creation
- process progress state
- wait conditions
- interrupt handling
- resume handling
- completion and failure transitions
- process-owned reservation interactions
- process wakeup scheduling

Does not own:

- hard mutation outside `CausalTransaction`
- final semantic meaning of events
- actor capability derivation
- pack verification

Core shape:

```text
ProcessInstance
  -> scheduled ProcessTick
  -> bind current context
  -> ProcessTransition
  -> Typed Effect Program or runtime primitive
  -> CausalTransaction
  -> EventRecord + RuntimeControlUpdate
  -> accepted process state update or completion
```

`ProcessInstance` is explicit serializable state, not a saved coroutine stack.
`ProcessRuntime` computes transitions; it does not mutate
`RuntimeControlStore` directly.

## CausalRuntime

Owns hard mutation discipline.

Owns:

- `ActionRequest` lifecycle
- `ProcessTick` lifecycle
- `ReactionRequest` lifecycle
- binding and preflight validation
- transaction staging
- invariant checks
- atomic hard-state commit
- event append through `EventHistoryStore`
- derived-view invalidation
- audit/provenance anchors

Does not own:

- final actor choice
- semantic appraisal
- social interpretation
- pack source syntax
- arbitrary store mutation outside transaction staging

Core flow:

```text
ActionRequest / ProcessTick / ReactionRequest
  -> bind roles and context
  -> preflight validation
  -> Typed Effect Program
  -> TypedEffectInterpreter stages checked effects
  -> CausalTransaction
  -> invariant check
  -> atomic commit
  -> EventRecord append
  -> invalidation and observation projection hooks
```

The causal runtime is the deepest hard-mutation waist. `ActionRequest` is an
actor-facing attempt boundary, not the deepest mutation boundary.

## TypedEffectInterpreter

Owns execution of checked hard-effect IR as an internal role of
`CausalRuntime`.

Owns:

- primitive effect dispatch
- lookup through `PrimitiveSemanticsRegistry`
- staged reads
- staged writes
- required `EventRecord` contract enforcement
- random draw provenance where it affects committed outcome
- dry-run, validation, audit, and selected replay interpretation modes

Does not own:

- atomic commit
- `EventHistoryStore` append
- hard-state publication
- raw store access outside transaction staging
- semantic meaning
- social claims
- actor memory
- final intent

Effect program shape:

```text
Typed Effect Program
  parameters
  required reads
  allowed primitive effects
  bounded control flow
  required event records
  replay/audit requirements
  stage permissions
```

Primitive effects should be domain-specific enough to preserve semantics and
contracts. The interpreter should not become a generic field mutation engine.

The interpreter owns dispatch discipline, not the whole standard primitive
vocabulary. Standard primitive semantics are installed from the standard world
library or trusted extension packages and execute only through staging
capabilities.

## EventHistoryStore

Owns committed causal records and audit anchors.

Owns:

- `TransactionRecord`
- `EventRecord`
- stored event schema version references
- transaction sequence metadata
- source request or process references
- random draw records where needed
- optional read/write summaries
- optional state hash checkpoints

Does not own:

- current physical state by itself
- semantic interpretation by itself
- social truth by itself
- actor belief by itself
- generated or authored chronology by itself

Records:

```text
CausalTransaction:
  committed mutation envelope

EventRecord:
  meaningful committed hard fact

MutationTrace:
  optional lower-level audit/debug trace

ObservationEvent:
  actor-relative projection after commit
```

Event history supports inspection, replay where needed, semantic evidence, and
debugging. It should not become the only materialized state representation.
Authored or generated chronology belongs to `ChronologyStore` unless a later
transition materializes hard state through `CausalRuntime` and commits an
`EventRecord`.

## VersioningPolicy

Owns cross-cutting version anchors and migration policy boundaries.

Owns:

- event schema version anchors
- definition and content version anchors
- save/snapshot version anchors
- accepted-record version metadata
- migration policy registration points

Does not own:

- exact persistence backend
- migration implementation mechanics
- runtime mutation authority
- diagnostic rendering

The architecture needs a visible owner for version policy before crate
boundaries are chosen. Exact persistence and migration mechanics remain
deferred.

## ObservationPipeline

Owns actor-relative projection from committed state and event surfaces.

Owns:

- `ObservedState`
- `ObservedEvent`
- sense-channel projection
- access filtering
- hidden information preservation
- observation provenance

Does not own:

- hard mutation
- durable memory writes by itself
- social meaning
- final action choice

Core flow:

```text
WorldModel + EventRecord + actor sensing context
  -> ObservedState / ObservedEvent
  -> EpistemicWorkingSet and later persistence gates
```

Observation is current perception, not memory. Persistence belongs to
epistemic commit surfaces.

## ActorContextPipeline

Owns construction of the actor-facing decision surface.

Owns:

- `CapabilitySet`
- `ActionRepertoire`
- `PerceivedAffordance`
- `EpistemicWorkingSet`
- accessible `SocialContextView`
- invalid action feedback context
- turn input for policy or controller

Does not own:

- final hard validation
- hard mutation
- final intent selection by itself
- omniscient debug state

Core shape:

```text
actor-owned hard state
  + accessible actor truth
  + observation
  + recognized authority/context
  -> CapabilitySet
  -> ActionRepertoire
  -> PerceivedAffordance
  -> turn input for policy or controller
```

The action space belongs to the actor. External objects and places contribute
signals, constraints, and affordances; they do not directly own the actor's
repertoire.

## Non-Hard Commit Gates

Non-hard gameplay state is still committed through explicit gates.

```text
SocialCommitGate:
  AcceptedSocialUpdate -> SocialInstitutionalStore

ChronologyCommitGate:
  AcceptedChronologyRecord -> ChronologyStore

EpistemicCommitGate:
  AcceptedEpistemicUpdate -> EpistemicStore

AppraisalCommitGate:
  AcceptedAppraisalRecord -> AppraisalRecordStore
```

These gates are not part of `CausalTransaction`, but they should preserve
authority class, provenance, source inputs, accepted ordering, invalidation
behavior, and a durable accepted-update envelope.

The hard runtime may create evidence that later supports non-hard commits. It
should not directly smuggle social meaning, memory, belief, or motivation into
hard physical effects.

The exact accepted-update record shape belongs in the runtime pipeline
architecture document.

## SemanticDecisionMiddleEnd

Owns the decision middle-end between actor-relative context and executable
runtime requests.

Owns:

- semantic views
- consumption of accessible `SocialContextView`
- appraisal variable sets
- `Thought`
- `Pressure`
- `GoalPressure`
- candidate intent generation
- `IntentScore` features
- selected or suggested `Intent`
- `Activity` preparation metadata

Does not own:

- hard mutation
- direct memory writes
- direct social truth writes
- `SocialContextView` assembly or access filtering
- direct process ticks
- direct `ActionRequest` submission without the appropriate choice boundary

Core flow:

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

This is not one monolithic planner. It is a staged set of representation
passes. Pressure biases later selection; it does not execute anything.

## Intent And Activity Runtime Boundary

`Intent` and `Activity` are runtime control boundaries.

```text
Intent:
  selected or suggested commitment to a purpose and approach

Activity:
  temporal execution frame that gives actor-facing meaning to ongoing
  process/plan execution
```

Lowering depends on resolution:

```text
Concrete:
  Intent -> Activity -> ActionRequest or ProcessInstance

Abstract:
  Intent -> Activity -> ProcessInstance

Strategic:
  Intent -> Activity -> region/faction/world process
```

Neither `Intent` nor `Activity` mutates hard truth directly. Hard outcomes
still enter through `ActionRequest`, `ProcessTick`, or `ReactionRequest` and
then through `CausalTransaction`.

Durable selected intent and activity lifecycle changes are runtime control
state. They are accepted as `AcceptedRuntimeControlUpdate` when control-only,
or as transaction-coupled `RuntimeControlUpdate` when tied to a hard outcome.

## ResolutionRuntime

Owns active simulation resolution.

Owns:

- concrete, abstract, and strategic execution policies
- promotion and demotion
- resolution-aware location semantics
- materialization constraints
- abstract event provenance
- process tick policy by resolution

Does not own:

- separate mutation authority
- separate process identity system
- hidden concrete action loops for abstract work
- actor-relative access policy by itself

Core rule:

```text
Resolution changes detail, not authority.
```

Abstract execution should advance through shared `ProcessInstance` and
`ProcessTick` machinery. It should not secretly generate repeated local
concrete requests just to preserve an illusion of detail.

## InspectionSurface

Owns inspection and explanation views.

Owns:

- provenance traces
- transaction/event inspection
- diagnostic presentation hooks
- rule match explanations
- effect execution traces where recorded
- replay/audit views
- derived-view explanation

Does not own:

- gameplay truth
- commit authority
- authoring verification
- controller decisions

The inspection surface consumes records produced by the rest of the engine. It
should not become an alternate mutation or correction path.

## AcceleratorAdapters

Own optional integration boundaries for specialized libraries.

Allowed roles:

```text
ECS:
  private hot storage or local materialized projection

Graph algorithms:
  pathfinding, topology snapshots, dependency graphs

Datalog-like engines:
  monotone derived closures and offline verification subproblems

Incremental query engines:
  pack compiler and editor-like tooling

Scripting:
  tooling helpers, declaration generation, explicit extension points

Sandboxed plugins:
  explicit capability boundary for later extension systems
```

Not allowed roles:

- root source of truth
- hidden hard mutation authority
- opaque bypass around `CausalTransaction`
- untyped access to actor-hidden truth
- durable record owner without a commit surface

## Dependency Direction

The architecture should keep dependencies pointing toward the core substrate.

Allowed conceptual dependency direction:

```text
authoring/compiler
  -> definition registry

runtime controllers/policies
  -> actor context
  -> request surfaces

semantic decision middle-end
  -> query layer and definition registry
  -> intent/activity outputs

causal runtime
  -> world model, definition registry, typed effects, scheduler hooks

world model
  -> store families, indexes, event history, query plumbing

inspection
  -> records and provenance from other components
```

Avoid:

```text
WorldModel depending on semantic appraisal.
CausalRuntime depending on final intent scoring.
TypedEffectInterpreter depending on actor memory internals.
PackCompiler depending on live world mutation.
ECS projection owning durable identity.
Inspection tools mutating gameplay state directly.
```

## Boot And Load Flow

High-level boot:

```text
load engine configuration
  -> install selected standard definition bundles and primitive semantics
  -> load pack sources or structured definitions
  -> PackCompiler verifies definitions
  -> DefinitionRegistry is built
  -> load or create WorldModel state
  -> rebuild or validate derived view registry
  -> initialize Scheduler and RuntimeControlStore
  -> expose controller and inspection surfaces
```

Save/load should preserve:

- world state stores
- relation stores
- runtime control state
- scheduler state
- process instances
- event/history cursor
- definition/content version anchors
- enough provenance for committed outcomes to remain inspectable

## Runtime Flow

One generic runtime cycle:

```text
external input, actor policy, process wakeup, reaction, or resolution work
  -> request surface
  -> request binding and validation
  -> typed effect interpretation
  -> CausalTransaction staging
  -> commit or failure result
  -> EventRecord append when committed
  -> derived-view invalidation
  -> observation projection
  -> non-hard context updates where accepted
  -> future scheduler/process/semantic work
```

Decision cycle:

```text
WorldModel-hosted stores / actor truth stores
  -> QueryLayer
  -> ObservationPipeline
  -> ActorContextPipeline
  -> SemanticDecisionMiddleEnd
  -> Intent
  -> Activity
  -> ActionRequest or ProcessInstance
```

## Component Ownership Table

| Component | Owns | Must Not Own |
| --- | --- | --- |
| EngineHost | orchestration and adapters | gameplay truth or hidden rules |
| PackCompiler | verification and registry construction | live runtime mutation |
| DefinitionRegistry | checked reusable definitions | current world state |
| StandardWorldLibrary | reusable primitive definitions and trusted semantics installers | causal commit authority or concrete game content |
| WorldModel | stores, identity, indexes, query plumbing | arbitrary write authority |
| QueryLayer | typed read surfaces | durable writes |
| DerivedViewRegistry | cache dependencies and invalidation | source truth |
| Scheduler | agenda and wakeups | process local state or mutation semantics |
| ProcessRuntime | `ProcessInstance` lifecycle | hard mutation outside transactions |
| CausalRuntime | hard mutation discipline | semantic meaning or final choice |
| PrimitiveSemanticsRegistry | trusted primitive handler lookup | standard vocabulary ownership |
| TypedEffectInterpreter | checked effect execution inside causal runtime | commit or raw store mutation |
| EventHistoryStore | committed causal records | social meaning or actor belief |
| VersioningPolicy | schema and content version anchors | persistence backend or mutation |
| ObservationPipeline | actor-relative projection | durable memory writes |
| ActorContextPipeline | actor-facing decision context | final hard validation |
| Non-Hard Commit Gates | accepted soft/actor/appraisal records | hard physical mutation |
| SemanticDecisionMiddleEnd | interpretation and intent preparation | direct execution |
| ResolutionRuntime | execution detail and materialization | separate mutation authority |
| InspectionSurface | explanation and audit views | correction or commit authority |
| AcceleratorAdapters | optional optimized views/tools | truth ownership |

## Stable Architecture Decisions

The canonical architecture decisions live in [Architecture Decisions](ADR.md)
and the dependency order lives in [Architecture Roadmap](roadmap.md). This
document applies those decisions to runtime component ownership and flow.

## Deferred

Defer until crate architecture or implementation planning:

- exact module boundaries
- exact role co-location strategy
- final public API names
- final parser choice and source syntax
- exact persistence backend
- migration mechanics and persistence backend details
- exact diagnostic renderer
- exact first standard primitive bundle
- concrete ECS or graph integration
- concrete scripting or plugin runtime
- specific game-system standard packs
- first vertical slice

## Next Document

The crate boundary architecture now lives in
[Crate Boundary Architecture](crates.md). The next architecture step should be
an implementation plan once the crate boundary is stable enough.
