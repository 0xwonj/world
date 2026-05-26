# Simulation Transition Compiler

## Status

Current architecture model.

This document defines the compiler-shaped architecture that connects pack
declarations, actor-relative context, semantic interpretation, intent lowering,
typed effects, and transactional world mutation.

It is not a claim that the game is a traditional source-code compiler. It is a
design model for organizing staged simulation transitions.

## Source Context

- [Simulation Core](simulation-core.md)
- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Pack Authoring And Semantic Declarations](pack-authoring-and-semantic-declarations.md)
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
- [World Model](world-model.md)
- [Causal Runtime](causal-runtime.md)
- [Typed Effect Primitives](typed-effect-primitives.md)
- [Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md)
- [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md)
- [Intent Templates And Planning](intent-templates-and-planning.md)
- [Multi-Resolution Simulation](multi-resolution-simulation.md)

External foundations:

- [MLIR Language Reference](https://mlir.llvm.org/docs/LangRef/), for
  multi-level IR, dialects, extensible operations, verification, and lowering
  discipline.
- [MLIR Passes](https://mlir.llvm.org/docs/Passes/), for pass-oriented
  transformation structure.
- [Rust incremental compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html)
  and [Salsa](https://rustc-dev-guide.rust-lang.org/queries/salsa.html), for
  dependency-tracked query recomputation.
- [Abstract Interpretation](https://cs.nyu.edu/~pcousot/COUSOTpapers/POPL77.shtml),
  for abstract execution, approximation, and refinement pressure.
- [Handling Algebraic Effects](https://lmcs.episciences.org/705), for the
  operation/handler split behind typed effect execution.
- [Souffle](https://souffle-lang.github.io/cav-paper), for Datalog-shaped rule
  and derived-fact analysis.
- [Differential Dataflow](https://timelydataflow.github.io/differential-dataflow/),
  for incremental maintenance of derived views as inputs change.
- [Rete](https://www.sciencedirect.com/science/article/pii/0004370282900200),
  for many-pattern / many-object rule matching pressure.
- [PDDL 2.1](https://planning.wiki/ref/pddl21/domain), for durative actions,
  temporal conditions, numeric fluents, and process-definition pressure.

These are methodological references. The design does not require adopting any
specific framework directly.

## Thesis

The engine incrementally compiles actor-relative situations into checked
simulation transitions.

More precisely:

```text
Pack declarations are compiled ahead of time.
Runtime situations are projected, analyzed, selected, and lowered where needed.
Effects are interpreted transactionally.
```

The runtime does not compile source code into a binary. It repeatedly turns the
current world, actor-relative context, available declarations, and policy choice
into a checked state transition.

```text
pack declarations + current world/context + actor/policy choice
  -> checked candidates
  -> selected intent/action/process
  -> typed effect program instance
  -> CausalTransaction
  -> EventRecord + store mutations
```

This model matters because it gives every design layer a concrete contract:

```text
input representation
output representation
transformation kind
allowed reads
allowed writes
verifier
pass boundary
provenance contract
invalidation dependencies
```

## Not A Traditional Compiler

A traditional compiler usually looks like this:

```text
source program
  -> parse
  -> analyze
  -> optimize
  -> lower
  -> codegen
  -> executable artifact
```

This engine looks like this:

```text
pack source
  -> ahead-of-time declaration checking
  -> registered definitions and indexes

world state + actor-relative context
  -> runtime query / analysis / candidate generation
  -> actor, player, NPC policy, or AI choice
  -> resolution-aware lowering
  -> transactional effect execution
  -> committed world mutation and records
```

Important differences:

- The input changes every turn.
- The engine cannot see omniscient truth from an actor's point of view.
- Runtime choice can come from a player, NPC policy, AI-controlled actor, or
  scheduled process.
- Failed validation is a normal simulation result, not merely a compile error.
- `EventRecord`s and accepted non-hard records are durable gameplay artifacts,
  not disposable compiler logs.
- Soft truth and non-hard proposals may be less deterministic, but accepted
  gameplay-relevant writes still need explicit provenance and commit gates.

The useful compiler analogy is therefore staged representation, analysis,
lowering, and checking, not batch compilation.

## Three Time Scales

### 1. Authoring / Pack Compile Time

Pack declarations are checked before they become usable runtime definitions.

Inputs:

```text
ActionSchema
ActionDef
ProcessDef
AppraisalRule
IntentTemplate
SocialRule
ContentSchema
DerivedView
Typed Effect Program
```

Outputs:

```text
DefinitionRegistry
SymbolTable
EffectSignatureSet
RuleIndex
QueryDefinitionIndex
EventRecordContractIndex
PackDependencyGraph
VersionMigrationTable?
```

Checks:

- referenced symbols exist
- declared types and roles match
- stage permissions are legal
- hard effects only use allowed primitive effects
- required `EventRecord` contracts are declared
- semantic rules do not mutate hard truth
- appraisal rules do not write memory, social truth, or final intent directly
- intent templates do not mutate truth
- process definitions declare supported resolutions
- pack dependencies and extension points are explicit

This is closest to ordinary compilation. It turns authored declarations into
verified, indexed runtime definitions.

[Pack Authoring And Semantic Declarations](pack-authoring-and-semantic-declarations.md)
owns the IR-family split: `Typed Effect Program` as a separate hard-mutation
IR family, and one semantic declaration IR framework for social, appraisal,
intent, and semantic-view declarations.

### 2. Runtime Context Compilation

At runtime, the engine projects, analyzes, and selectively lowers the current
situation into candidates and executable requests.

Inputs:

```text
hard truth
soft truth
actor truth
current resolution
scheduled wakeups
active process state
recent EventRecord history
registered pack definitions
actor / player / NPC / AI policy boundary
```

Actor context:

```text
ObservedState
ObservedEvent
EpistemicWorkingSet
SocialContextView
CapabilitySet
ActionRepertoire
PerceivedAffordance
```

Decision intermediates:

```text
Pressure / GoalPressure
CandidateIntent
```

Outputs:

```text
selected Intent
Activity
ActionRequest
ProcessInstance
ProcessTick
ReactionRequest
InvalidActionFeedback
```

Runtime compilation is partial and actor-relative. It should only expose what
the actor can access, and it should preserve uncertainty instead of silently
reading omniscient truth.

### 3. Transactional Execution

After an executable request or process tick exists, hard mutation uses the
causal runtime.

```text
ActionRequest / ProcessTick / ReactionRequest
  -> ActionDef / ProcessDef binding
  -> Typed Effect Program instance
  -> CausalTransaction staging
  -> invariant checks
  -> atomic commit
  -> EventRecord append
  -> derived view invalidation
```

The transaction interpreter is analogous to an effect handler. Pack-authored
definitions request typed operations; the runtime decides whether those
operations are valid, how they stage mutation, which records are emitted, and
whether the result commits.

## Representation Ladder

Not every boundary object in this architecture is an IR.

`IR` is reserved for checked, transformable intermediate artifacts such as
definition IR and effect IR. Candidate forms may be IR-like inside a pass, but
they should not be confused with durable state, actor-facing requests,
transactions, or committed records. The broader word is representation.

The pipeline uses two levels of classification:

```text
RepresentationClass:
  broad role family used for design, authority, visibility, provenance,
  caching, diagnostics, and extension boundaries

representation kind:
  concrete role at a particular pipeline boundary
```

Representation classes are design taxonomy labels, not mandatory runtime base
types. They do not require a universal `Representation` trait or one generic
storage model. A value's class can also change by lifecycle role: an
`EventRecord` is a publication output when emitted, then becomes durable
authority state when later queried; a `ProcessInstance` is durable
runtime-control state that can schedule executable work while ticking.

```text
Authoring:
  pack source, definition IR, rule declarations, schemas, registries

AuthorityState:
  hard truth, soft truth, actor truth, durable stores, accepted records

DerivedContext:
  actor-relative views, working sets, access-filtered context, analysis results

Decision:
  appraisal candidates, pressures, goals, intent candidates, selected intent

Executable:
  actor/process/reaction work items

RuntimeControl:
  durable selected intent, activity, process, reservation, and wakeup control
  state

EffectProgram:
  checked typed mutation program instance

CommitEnvelope:
  staged transaction, runtime-control update, accepted update envelope,
  invalidation package
```

| Level | Class | Kind | Representation | Owner | Transition role |
| --- | --- | --- | --- | --- | --- |
| Pack source | Authoring | source | authored pack declarations | Game system packs | human-authored vocabulary and rules |
| Definition | Authoring | definition IR | `ActionDef`, `ProcessDef`, `AppraisalRule`, `IntentTemplate`, `SocialRule`, `ContentSchema`, `DerivedView`, `DefinitionRegistry` | Pack compiler / relevant domain docs | compile, verify, index |
| World/query input | AuthorityState | state / durable record | hard truth, soft truth, actor truth, event history, accepted non-hard records | World model and truth owners | authoritative and holder-relative inputs |
| Actor context | DerivedContext | view / working set / analysis result | `ObservedState`, `ObservedEvent`, `EpistemicWorkingSet`, `SocialContextView`, `CapabilitySet`, `ActionRepertoire`, `PerceivedAffordance` | perception, epistemic, social, capability docs | project, retrieve, filter, analyze |
| Appraisal | Decision | appraisal candidate / accepted proposal | `Thought`, `Pressure`, `GoalPressure`, appraisal candidate, accepted `AppraisalRecord` proposal | semantic appraisal | interpret and propose motivational pressure |
| Intent | Decision | candidate / selected purpose | `CandidateIntent`, `IntentScore`, `Intent` | intent planning | generate, rank, select |
| Runtime control | RuntimeControl | control state / durable progress | `Activity`, `ActivityTransition`, `ProcessInstance`, `ProcessTransition`, `RuntimeControlUpdate` | intent/activity runtime, process runtime, runtime-control gate | persist selected work, progress, scheduling, and reservation state |
| Execution | Executable | request / tick / reaction | `ActionRequest`, `ProcessTick`, `ReactionRequest`, `ResolutionTransitionRequest`, `InvalidActionFeedback` | actor interface, multi-resolution, causal runtime | bind, validate, tick, react |
| Effect | EffectProgram | effect IR | `Typed Effect Program` instance | typed effects | bind and specialize checked mutation logic |
| Commit | CommitEnvelope | transaction / runtime-control / accepted update envelope | `CausalTransaction`, `RuntimeControlUpdate`, `AcceptedRuntimeControlUpdate`, `AcceptedSocialUpdate`, `AcceptedChronologyRecord`, `AcceptedEpistemicUpdate`, `AcceptedAppraisalRecord`, invalidation set | causal runtime and truth-boundary owners | stage, validate, publish through authority gate |
| Output | CommitEnvelope -> AuthorityState | record / state update | `EventRecord`, accepted soft/actor/appraisal/runtime-control records, store mutations | truth-boundary owners | persist facts and invalidate derived views |

### Decision Middle-End Pass Group

Semantic appraisal and intent planning are the decision middle-end of this
pipeline. They do not add a new top-level compiler phase. They refine the
existing `DerivedContext` -> `Decision` -> executable-preparation portion of
the ladder.

Folded compiler view:

```text
DerivedContext
  -> Appraisal
  -> Intent Choice
  -> Activity / lowering target preparation
  -> RuntimeControl where durable progress or control state is needed
  -> Executable
```

Expanded decision view:

```text
ObservedEvent / ObservedState
  + EpistemicWorkingSet
  + SocialContextView
  + CapabilitySet / ActionRepertoire
  + PerceivedAffordance
  -> AppraisalVariableSet
  -> Thought
  -> Pressure
  -> GoalPressure
  -> CandidateIntent
  -> IntentScore
  -> selected or suggested Intent
  -> Activity
  -> ActionRequest or ProcessInstance target
```

This structure exists to contain complexity inside pass contracts. It should
not be implemented as one opaque "agent brain" that directly maps context to
action. The engine may physically combine some of these steps in an early
implementation, but the logical pass boundaries must remain visible for
provenance, authority checks, AI proposal gates, and resolution-aware lowering.

Non-negotiable boundaries:

- appraisal may produce `Thought`, `Pressure`, and `GoalPressure`, but not
  final `Intent`
- intent choice may select or suggest `Intent`, but not mutate truth
- `Activity` may prepare execution over time, but hard effects still require
  `ActionRequest` or `ProcessTick`
- abstract execution must lower through `ProcessInstance`, not hidden concrete
  action spam
- accepted hard mutation still passes through `Typed Effect Program`,
  `CausalTransaction`, and `EventRecord`

The ladder is not always linear. Some passes can skip levels. For example, a
player may submit an `ActionRequest` directly; a process wakeup may start at
`ProcessTick`; an AI proposal may target a soft commit gate instead of hard
mutation. The invariant is that each accepted output still uses the correct
authority boundary.

## Transformation Kinds

Each edge in the pipeline should name what kind of transformation it performs.
Calling every edge "lowering" hides important authority differences.

Transformation also uses two levels of classification:

```text
PassClass:
  broad operational family used for scheduling, extension points, default
  authority expectations, cacheability, provenance, and diagnostics

transformation kind:
  precise semantic operation performed by a pass
```

`PassClass` is not a permission by itself. A derivation pass can still leak
hidden truth if its read/output contract is wrong; a publication pass can still
be invalid if it writes through the wrong gate. The individual pass contract
remains authoritative.

Default expectations:

| Pass class | Default expectation |
| --- | --- |
| Declaration | authoring/load-time only; may write definition registries, not gameplay state |
| Derivation | read-only or cache-only; produces derived context with provenance and invalidation dependencies |
| Choice | may involve player, NPC policy, AI, scoring, or controlled nondeterminism; must not mutate truth |
| Translation | maps decisions or attempts to target executable/effect representations; must preserve binding provenance |
| Execution | validates and interprets checked effects into staged runtime state; durable writes are still forbidden |
| Publication | publishes durable state only through accepted authority gates and invalidates derived views |

Useful compiler and runtime analogies:

| Pass class | Transformation kind | Meaning in this engine | Typical edge |
| --- | --- | --- | --- |
| Declaration | Compile / verify / index | Turn authored declarations into checked reusable definitions and indexes. | pack source -> definition registry |
| Derivation | Projection | Produce an actor-relative view from hard state and signal/event surfaces. | hard truth -> `ObservedState` / `ObservedEvent` |
| Derivation | Retrieval | Select relevant holder-relative records without changing them. | `EpistemicStore` -> `EpistemicWorkingSet` |
| Derivation | Access filtering | Restrict social, institutional, or epistemic context to what the actor may use. | social state -> `SocialContextView` |
| Derivation | Analysis | Derive capabilities, affordances, interpretations, or pressures from available inputs. | context -> `CapabilitySet`, `PerceivedAffordance`, `Pressure` |
| Derivation | Rule matching | Find applicable declarations or template matches without committing their consequences. | facts/views -> appraisal, social, affordance, or intent-template matches |
| Choice | Generation / ranking / selection / proposal | Generate, score, choose, or propose one option among candidates. | matches/candidates -> `CandidateIntent`, selected `Intent`, or proposal |
| Translation | Lowering | Translate a higher-level purpose to an execution or runtime-control target. | `Intent` -> `Activity` -> `ActionRequest` or `ProcessInstance` |
| Translation | Binding / specialization / legalization | Attach concrete roles, targets, definitions, and current-world facts to an executable request. | `ActionRequest` / `ProcessTick` -> `Typed Effect Program` instance |
| Execution | Validation | Check current-world legality, invariants, reservations, randomness, and failure contracts. | effect instance -> staged transaction or feedback |
| Execution | Interpretation / handling / staging | Execute typed effect operations through runtime handlers without bypassing authority. | `Typed Effect Program` -> `CausalTransaction` staging |
| Publication | Commit / accepted update | Atomically publish hard mutations or accepted non-hard state. | `CausalTransaction` or accepted update envelope -> durable state |
| Publication | Invalidation | Mark derived views and caches stale after accepted changes. | committed records -> affected queries |

Only some of these are true lowering. Observation is projection, epistemic
context is retrieval, social context is access-filtered view construction,
appraisal is semantic analysis, intent is generation and selection, effects are
binding plus interpretation, and final state change is commit.

## Canonical Ladder, Extensible Pass Graph

The representation ladder defines the canonical conceptual order. It is not a
mandatory linear execution sequence.

Actual execution may be a typed pass graph organized by
`RepresentationClass`, `PassClass`, and explicit pass contracts:

```text
shortcut pass:
  A -> D

expansion pass:
  A -> B1 -> B2 -> B3 -> C

side analysis pass:
  A -> AnalysisResult
  AnalysisResult is consumed later without replacing A
```

This is useful because different gameplay situations need different amounts of
analysis. A direct player command may produce an `ActionRequest` without
explicit intent generation. A complex social scene may insert threat,
norm-violation, reputation-risk, or faction-context analyses before appraisal.

A pass may skip or insert stages only if:

- its output representation satisfies the target contract
- required provenance is preserved
- actor-facing outputs do not leak hidden truth
- authority gates are not bypassed
- required validation and commit boundaries still run
- invalidation dependencies are declared

Non-skippable gates:

- hard mutation must pass through `CausalTransaction`
- required physical or sensory evidence must satisfy `EventRecord` contracts
- accepted non-hard state must pass through its accepted update gate
- actor-facing information must respect perception and access filtering
- effect execution must pass through binding, validation, and runtime
  interpretation

Therefore packs should extend the engine through declared extension points and
typed pass contracts, not by inserting arbitrary code between layers.

## Pass Contract

Every major pass should be designed with the same shape.

```text
PassContract:
  name
  owner_doc
  pass_class
  transformation_kind
  input_representation_class
  input_representation
  output_representation_class
  output_representation
  allowed_reads
  allowed_writes
  authority_class
  verifier
  target_contract?
  provenance_output
  invalidation_dependencies
  failure_surface
  replay_requirements
```

This prevents "systems" from becoming vague boxes. A pass is allowed to do only
what its contract says.

### Pack Declaration Pass

```text
pass class:
  Declaration

transformation kind:
  compile / verify / index

input:
  pack source declarations

output:
  checked DefinitionRegistry entries
  rule indexes
  effect signatures
  event record contracts

allowed writes:
  definition registry / pack metadata only

forbidden:
  runtime world mutation
  accepted gameplay state
```

Verifier:

- type and symbol resolution
- effect permission checking
- stage-permission checking
- required provenance and `EventRecord` contracts
- pack dependency consistency

### Observation Projection Pass

```text
pass class:
  Derivation

transformation kind:
  projection

input:
  hard truth
  EventRecord history
  actor perceptual capability
  environmental signals

output:
  ObservedState
  ObservedEvent

allowed writes:
  none, unless explicitly caching derived observations

forbidden:
  memory creation
  semantic meaning
  intent selection
```

This pass is like frontend analysis over an actor-specific view of the world.
It must preserve uncertainty and hidden-truth boundaries.

### Epistemic Retrieval Pass

```text
pass class:
  Derivation

transformation kind:
  retrieval / filtering

input:
  actor holder
  accessible EpistemicRecord set
  current observation
  query focus

output:
  EpistemicWorkingSet

allowed writes:
  none for retrieval

forbidden:
  direct pressure creation
  direct action selection
```

Epistemic persistence is a separate accepted update path. Retrieval constructs
the working set for later passes.

### Social Context Pass

```text
pass class:
  Derivation

transformation kind:
  access filtering / derived view construction

input:
  accessible social/institutional state
  actor identity and recognized authority
  current observation and epistemic context

output:
  SocialContextView

allowed writes:
  none for query

forbidden:
  physical mutation
  direct appraisal output
```

This is a derived context pass, not a social-state commit pass.

### Semantic Appraisal Pass

```text
pass class:
  Derivation

transformation kind:
  semantic analysis / rule matching / proposal

input:
  ObservedEvent
  EpistemicRecord / EpistemicWorkingSet
  SocialContextView
  AppraisalRule definitions

output:
  Thought
  Pressure
  GoalPressure
  AcceptedAppraisalRecord proposal or accepted record

allowed writes:
  AppraisalRecordStore only through AcceptedAppraisalRecord

forbidden:
  hard truth mutation
  direct EpistemicStore writes
  direct SocialInstitutionalStore writes
  direct final intent selection
```

This is the closest layer to semantic analysis in a compiler. It gives meaning
to facts and memories under context.

### Intent Choice Pass

```text
pass class:
  Choice

transformation kind:
  generation / ranking / selection

input:
  Pressure
  GoalPressure
  CapabilitySet
  ActionRepertoire
  PerceivedAffordance
  EpistemicWorkingSet
  SocialContextView
  IntentTemplate definitions

output:
  CandidateIntent
  IntentScore
  selected or suggested Intent

allowed writes:
  intent state or suggestion state only where the later intent design permits

forbidden:
  hard truth mutation
  effect execution
```

This is the folded compiler view of intent choice. The detailed owner is
[Intent Templates And Planning](intent-templates-and-planning.md), which splits
this folded pass into candidate generation, scoring/selection, and activity
preparation. It should be explainable through typed features, not opaque
story-specific scripts.

### Resolution Lowering Pass

```text
pass class:
  Translation

transformation kind:
  target lowering

input:
  Intent
  active resolution
  CapabilitySet
  PerceivedAffordance
  current process state

output:
  concrete: Activity -> ActionRequest or ProcessInstance
  abstract: Activity -> ProcessInstance
  strategic: Activity -> region/faction/process request where applicable

allowed writes:
  no hard mutation directly

forbidden:
  hidden concrete ActionRequest generation during abstract execution
```

This is target lowering. The target is not a CPU. The target is a simulation
resolution.

### Definition Binding Pass

```text
pass class:
  Translation

transformation kind:
  binding / specialization

input:
  ActionRequest / ProcessTick / ReactionRequest
  ActionDef / ProcessDef / reaction definition
  current hard truth
  actor-relative binding provenance

output:
  Typed Effect Program instance
  InvalidActionFeedback where binding or validation fails

allowed writes:
  none before transaction staging
```

Binding turns a high-level attempt into a specific effect program instance, but
it still does not mutate truth.

### Effect Verification And Execution Pass

```text
pass class:
  Execution

transformation kind:
  validation / interpretation / handling

input:
  Typed Effect Program instance
  allowed effects
  hard truth query surface
  RNG stream

output:
  CausalTransaction

allowed writes:
  staged transaction state only

forbidden:
  direct store mutation before commit
  semantic writes
  untracked randomness
```

This is the low-level effect interpreter.

### Transaction Commit Pass

```text
pass class:
  Publication

transformation kind:
  commit / invalidation

input:
  CausalTransaction
  invariant results

output:
  committed hard state
  EventRecord set
  runtime control updates
  schedule updates
  derived-view invalidations

allowed writes:
  hard truth stores and EventHistoryStore

forbidden:
  rewriting prior committed EventRecord entries
  implicit semantic meaning
```

This is the only hard mutation commit point.

## Dialects, Not One Giant Language

The architecture should avoid one universal language that tries to represent
everything.

For pack authoring, this means source files may be organized by theme or
extension while compiled registries are organized by declaration kind, stage,
and trigger.

Instead, use dialect-like boundaries:

```text
physical dialect:
  material, substance, body, wound, signal, field, residue

effect dialect:
  transfer_entity, apply_damage, emit_signal, schedule_process

process dialect:
  supported resolutions, tick policy, progress, interrupt, resume

appraisal dialect:
  observed event + epistemic/social context -> thought/pressure

intent dialect:
  pressure/goal + capability/affordance -> candidate intent

social dialect:
  SocialClaim, norm, law, taboo, permission, debt, oath

epistemic dialect:
  holder-relative records, confidence, source, access, disclosure
```

The social, appraisal, intent, and semantic-view dialects share the semantic
declaration framework. They are not separate unrelated DSLs, and they are not
allowed to collapse into one unrestricted semantic language.

Each dialect needs:

- owned vocabulary
- type rules
- stage permissions
- verifier
- provenance expectations
- lowering or query boundaries

Dialect boundaries match the engine/game-pack boundary:

```text
core owns mechanism;
packs define vocabulary inside checked dialects;
game content instantiates pack vocabularies.
```

## Verifier Versus Runtime Validator

The architecture needs both.

### Verifier

Runs at authoring/load time.

Questions:

- Is this declaration well-typed?
- Are all symbols resolvable?
- Does this effect program use only allowed effects?
- Does the rule declare required provenance?
- Does the process definition cover its supported resolutions?
- Does this pack try to write stores it cannot own?

Verifier failure means the pack or declaration is invalid.

### Runtime Validator

Runs against current world/context.

Questions:

- Can this actor currently attempt this schema?
- Does the perceived binding still correspond to valid hard truth?
- Is the target reachable?
- Are reservations available?
- Did the skill, resistance, or random check pass?
- Are transaction invariants still satisfied?

Runtime validation failure is simulation output. It may become
`InvalidActionFeedback`, a failed attempt record, or no public record depending
on the failure contract.

## Incremental Queries And Invalidation

The compiler model makes derived views explicit.

Candidate derived queries:

```text
CapabilitySet(actor)
ActionRepertoire(actor)
PerceptionContext(actor)
ObservedState(actor, focus)
ObservedEvent(actor, event_ref)
EpistemicWorkingSet(holder, focus)
SocialContextView(actor, focus)
PerceivedAffordance(actor, subject)
CandidateAppraisals(actor, focus)
CandidateIntents(actor, focus)
```

Each query should declare:

- stable key
- input dependencies
- authority class of inputs
- whether output is cacheable
- whether output may be shown to actor or only used internally
- invalidation conditions
- explanation/provenance output

Post-commit invalidation should be precise enough to avoid full recomputation,
but not so clever that derived views become hidden truth.

```text
CausalTransaction commits EntityTransferred(item, from, to)
  -> invalidate containment-derived views for item/from/to
  -> invalidate observers' possible ObservedState near relevant places
  -> invalidate affordances involving item/from/to
  -> notify epistemic persistence and appraisal candidate passes
```

## Rule Matching

Semantic appraisal, social rules, affordance inference, and intent templates
will create a many-rules / many-facts problem.

The design should support rule indexing rather than full scans.

Useful approaches:

- Datalog-shaped derived relations for queryable context and rule
  preconditions.
- Rete-like production matching when many rules need to stay partially matched
  across changing working memory.
- Differential/incremental dataflow when derived relations are large and input
  deltas are small.
- Hand-written indexes for hot physical and perception paths.

Rule matching should produce candidates, not direct mutation.

```text
facts and views
  -> candidate rule matches
  -> typed proposal
  -> correct commit gate
```

## Multi-Resolution As Abstract Interpretation

Multi-resolution simulation is not only optimization. It is a controlled
change of semantic domain.

```text
concrete simulation:
  detailed state surface and local action semantics

abstract simulation:
  coarser state surface and process semantics

strategic simulation:
  aggregate pressures, region/faction processes, chronology, and summaries
```

Demotion is abstraction:

```text
concrete body / inventory / route / conflict details
  -> preserved durable state
  -> summarized process state
  -> abstract location and progress
```

Promotion is refinement:

```text
abstract process state + provenance + nearby context
  -> concrete state sufficient for interaction
```

The design should not require perfect semantic equivalence. It should require
the right preservation contract:

- stable identity
- authority class
- important hard state
- active intent/process identity
- durable consequences
- reservations where still valid
- event provenance
- actor-observable evidence
- no contradiction with committed `EventRecord`s

This is the practical game version of an abstract interpretation contract.

## Process Definitions As Temporal Lowering Targets

`ProcessDef` is the key bridge between compiler architecture and RPG activity.

Useful pressure from temporal planning:

```text
ProcessDef:
  roles
  supported_resolutions
  start_condition
  over_time_invariant
  tick_policy
  completion_condition
  start_effect?
  tick_effect?
  completion_effect?
  interruption_policy
  resume_policy
  failure_policy
  event_record_contract
```

Concrete resolution may lower an intent through `Activity` to atomic
`ActionRequest`s or a long-running `ProcessInstance`. Abstract resolution
lowers intent through `Activity` to `ProcessInstance`; the process later wakes
through `ProcessTick`, not hidden concrete action spam. Strategic resolution
usually uses region/faction/scenario processes rather than individual intent.

The process definition is authored by a pack. The process instance is runtime
state. The process tick commits through the causal runtime.

## Diagnostics And Explanation

Compiler architecture is valuable only if it produces diagnostics.

The engine should eventually answer:

- Why did this actor see the blood but not identify the killer?
- Why did this memory become revenge pressure?
- Why was `PickLock` available yesterday but unavailable now?
- Why did the wounded hand increase lockpick difficulty?
- Why did the abstract travel process materialize here?
- Why did shrine item removal count as taboo violation to this guard but not to
  another actor?
- Why did an AI proposal get accepted, rejected, or constrained?

Each pass should emit explanation fragments:

```text
query provenance:
  which facts and records were read

rule provenance:
  which rule matched and why

lowering provenance:
  why this target representation or executable request was chosen

validation provenance:
  which checks passed or failed

commit provenance:
  which mutations and EventRecords were committed
```

This is not only debugging. It is part of making AI-native play and authoring
safe: agents and tools need stable, inspectable reasons.

## Scenario Walkthroughs

### Wounded Hand And Lockpick

```text
hard truth:
  actor has wounded right hand
  actor carries lockpick
  door has lock

queries:
  CapabilitySet(actor)
  ActionRepertoire(actor)
  PerceivedAffordance(actor, door)

intent/action:
  Intent(OpenLockedContainer)
  -> Activity(OpeningLockedContainer)
  -> ActionRequest(ApplyTool(lockpick, door.lock, pick))

binding:
  ActionDef(ApplyTool)
  lock affords pick attempt
  wounded hand degrades manipulation precision

effect program:
  check_can_manipulate(actor, fine)
  check_skill(actor, lockpicking, increased_difficulty)
  set_lock_state(lock, unlocked) on success

transaction:
  CausalTransaction
  EventRecord(LockPickAttempted)
  EventRecord(LockUnlocked?) on success
```

No special `WoundedHandCannotPickLock` action is needed. The capability,
affordance, binding, and effect passes explain the result.

### Torch And Wooden Door

```text
pack definitions:
  ApplyTool(tool, target, mode)
  material flammability vocabulary
  ignite / heat / smoke effect contracts

runtime context:
  actor has lit torch
  door material is wood
  door is observed as burnable

lowering:
  Intent(DestroyOrOpenBarrier)
  -> Activity(OpeningBarrier)
  -> ActionRequest(ApplyTool(torch, door, ignite))
  -> ActionDef(ApplyTool)
  -> Typed Effect Program

effect program:
  check_reachable(actor, door)
  check_material_property(door, flammable)
  change_temperature(door, delta)
  ignite(door)?
  emit_signal(smoke)
  apply_damage(door, fire_damage)

commit:
  CausalTransaction
  EventRecord(TemperatureChanged)
  EventRecord(FireStarted?)
  EventRecord(ObjectDamaged?)
```

The physical layer does not declare arson, crime, sacrilege, or cleverness.
Those meanings belong to later appraisal passes.

### Mentor Killed By Bandit

```text
hard fact:
  EventRecord(ActorDied(mentor, cause, source_actor=bandit))

actor-relative projection:
  player observed the death, or later hears testimony

epistemic:
  EpistemicRecord(holder=player, content=EventRecordRef(...))

appraisal:
  relationship(player, mentor).emotional_weight = high
  AppraisalRule(close relation harmed by known actor)
  -> Thought(Grief)
  -> Pressure(Retaliation, target=bandit)
  -> GoalPressure(FindOrConfrontBandit)

intent:
  IntentTemplate(LocateActor)
  IntentTemplate(AskInformationSource)
  IntentTemplate(TrackPhysicalTrace)
  IntentTemplate(ConfrontActor)

lowering:
  concrete -> Activity -> ActionRequest or local ProcessInstance
  abstract -> Activity -> ProcessInstance
```

The engine does not hardcode a scenario-specific pursuit command. It compiles
the situation into generic appraisals and intent templates.

### Abstract Travel

```text
intent:
  TravelToMarket

abstract lowering:
  Activity(TravelingToMarket)
  -> ProcessInstance(TravelToMarket, active_resolution=abstract)

ticks:
  ProcessTick
  -> CausalTransaction
  -> EventRecord(RouteProgressed)
  -> EventRecord(CaravanDelayed?) if risk triggers

promotion:
  route progress + provenance + local terrain
  -> concrete position near route anchor
```

No hidden concrete `Move` actions are generated while abstract. The process
uses its abstract tick semantics and still commits hard outcomes through the
causal runtime.

## AI And Coauthoring

AI can participate in this architecture in three places.

### Policy Choice

An AI-controlled actor may choose among actor-facing candidates.

```text
AgentTurnInput
  -> SelectIntent(...)
  -> SubmitActionRequest(...)
```

The AI does not gain direct mutation authority. Its choice still goes through
the same selection, lowering, binding, and validation boundaries as any other
actor decision.

### Proposal Generation

AI may propose soft truth, chronology, epistemic content, appraisal records, or
materialization details where the relevant design permits it.

```text
AI proposal
  -> proposal verifier / gate
  -> accepted non-hard record
```

If an AI proposal affects hard truth, it must become a checked plan whose hard
effects commit through `CausalTransaction`.

### Explanation And Tooling

AI can help summarize traces, explain rule matches, suggest pack definitions,
or diagnose why a transition failed. It should consume provenance, not invent
authority.

## Anti-Goals

- Do not rename every game object into compiler jargon.
- Do not build one giant universal source language.
- Do not let Datalog, Rete, or AI become mutation authority.
- Do not require perfect equivalence between concrete and abstract simulation.
- Do not treat `EventRecord` as compiler log text; it is committed hard fact.
- Do not optimize away gameplay evidence just because a compiler would remove
  redundant work.
- Do not make pack authors write low-level effects when a checked higher-level
  declaration can lower safely.
- Do not turn `RepresentationClass` or `PassClass` into a universal runtime
  base type or arbitrary plugin system.

## Stable Decisions

- The engine uses compiler architecture internally: multi-level
  representations, narrow IRs, checked passes, effect-typed lowering,
  incremental queries, and transactional interpretation.
- The engine does not compile source code into a binary. It incrementally
  compiles actor-relative situations into checked simulation transitions.
- Pack declarations are checked ahead of time; runtime situations are
  projected, analyzed, selected, and lowered where needed; hard effects are
  interpreted transactionally.
- Every major layer should define input representation, output
  representation, representation class, pass class, transformation kind,
  allowed reads, allowed writes, verifier, pass boundary, provenance, and
  invalidation dependencies.
- `RepresentationClass` and `PassClass` are design taxonomy labels for
  authority, visibility, cacheability, provenance, diagnostics, and extension
  boundaries. They are not a requirement for one generic runtime abstraction.
- `CausalTransaction` remains the hard mutation boundary. The compiler model
  does not create another path around it.
- Multi-resolution simulation should be treated as resolution-aware lowering
  plus abstract/refined execution contracts, not as hidden concrete simulation.
- PL/tooling should check, index, explain, and migrate declarations. It should
  not own truth.
- AI may choose, propose, and explain through explicit gates. It must not
  silently acquire mutation authority.

## Deferred Decisions

- exact serialized representation of each durable representation and internal
  IR
- exact pack compiler phases and registry layout
- exact query dependency and invalidation engine
- exact rule matching strategy for appraisal, social, and intent rules
- exact diagnostic/explanation trace format
- exact correctness criteria for abstract/concrete lowering
- exact relation between future PL syntax and internal definition IRs
