# Runtime Pipeline Architecture

## Status

Frozen legacy planning draft.

Superseded where it conflicts with
[`target-architecture/runtime-persistence-and-scale.md`](target-architecture/runtime-persistence-and-scale.md).

## Purpose

This document explains how runtime work flows through the engine's logical
components.

It answers:

```text
How do input, scheduler wakeups, intent, activity, action requests, process
ticks, typed effects, transactions, observations, semantic passes, and
resolution transitions connect?
```

It is not:

- a crate boundary document
- a final Rust API reference
- engine code
- a schema design
- a parser or source syntax design
- a vertical slice plan
- a gameplay-system content plan

The goal is to make the runtime path explicit enough that implementation can
preserve authority, actor perspective, process state, auditability,
explainability, and resolution boundaries without turning the engine into one
opaque planner or one generic ECS system graph.

## Inputs

Primary architecture inputs:

- [Architecture Roadmap](roadmap.md)
- [Architecture Decisions](ADR.md)
- [Engine Architecture](engine.md)

Primary design inputs:

- [Simulation Transition Compiler](../design/simulation-transition-compiler.md)
- [Causal Runtime](../design/causal-runtime.md)
- [Time Model](../design/time-model.md)
- [Typed Effect Primitives](../design/typed-effect-primitives.md)
- [Standard World Library And Primitive Semantics](../design/standard-world-library.md)
- [World Model](../design/world-model.md)
- [Perception And Observation](../design/perception-and-observation.md)
- [Capability, Affordance, And Actor Interface](../design/capability-affordance-and-actor-interface.md)
- [Epistemic State](../design/epistemic-state.md)
- [Social Institutional Model](../design/social-institutional-model.md)
- [Semantic Appraisal And Motivation](../design/semantic-appraisal-and-motivation.md)
- [Intent Templates And Planning](../design/intent-templates-and-planning.md)
- [Multi-Resolution Simulation](../design/multi-resolution-simulation.md)
- [Pack Authoring And Semantic Declarations](../design/pack-authoring-and-semantic-declarations.md)

Primary research inputs:

- [Runtime Pipeline Implementation Research](../research/runtime-pipeline-implementation-research.md)
- [Implementation Architecture And Library Survey](../research/implementation-architecture-and-library-survey.md)
- [Causal Runtime / Action-Effect-Event](../research/causal-runtime-action-effect-event.md)
- [Time Model / Turn Scheduling](../research/time-model-and-turn-scheduling.md)
- [World Representation / Query Model](../research/world-representation-query-model.md)
- [Semantic Appraisal, Intent, Activity, And Planning](../research/semantic-appraisal-intent-activity-planning.md)

## Thesis

The runtime pipeline is compiler-shaped but domain-named.

Use compiler concepts for:

```text
representation
pass
query dependency
projection
derivation
choice
lowering
legalization
binding
interpretation
publication
invalidation
diagnostics
provenance
```

Keep domain terms for the actual boundaries:

```text
Intent
Activity
ActionRequest
ProcessInstance
ProcessTick
Typed Effect Program
CausalTransaction
EventRecord
ObservedState
ObservedEvent
Thought
Pressure
GoalPressure
CandidateIntent
IntentScore
RuntimeControlUpdate
AcceptedRuntimeControlUpdate
ActivityTransition
ProcessTransition
DrainOutcome
```

The architecture should not rename the engine into a traditional compiler.
The useful structure is that each transition has typed inputs, typed outputs,
allowed reads, allowed writes, target contracts, provenance, invalidation, and
failure surfaces.

## Canonical Runtime Flow

The full runtime path:

```text
InputSource / ScheduledWakeup
  -> request or wakeup envelope
  -> query and context binding
  -> validation, derivation, choice, or lowering
  -> decision/control transition or executable work item
  -> durable ProcessInstance when work spans time or resolution
  -> Typed Effect Program instance when hard mutation is needed
  -> CausalTransaction staging
  -> invariant check
  -> atomic commit package
  -> TransactionRecord + EventRecord + store/control updates + invalidation
  -> observation projection
  -> accepted non-hard or runtime-control updates where applicable
  -> semantic/context work and future scheduling
  -> inspection, explanation, save/load, and replay surfaces
```

This flow is not always linear. A player command may enter at `ActionRequest`.
A process wakeup may enter at `ProcessTick`. A reaction may enqueue a later
`ReactionRequest`. A semantic pass may stop at pressure or candidate intent
without selecting anything.

The invariant is:

```text
Every accepted output uses the authority boundary for its state class.
```

Hard mutation uses:

```text
Typed Effect Program
  -> CausalTransaction
  -> EventRecord / store updates
```

Non-hard accepted state and control-only runtime state use accepted update
gates:

```text
AcceptedSocialUpdate
AcceptedChronologyRecord
AcceptedEpistemicUpdate
AcceptedAppraisalRecord
AcceptedRuntimeControlUpdate
```

Decision and interpretation outputs may bias, rank, suggest, or request. They
do not mutate hard truth directly.

## Runtime Control State

Runtime control state is durable engine control state that is not hard physical
truth and not semantic meaning.

It includes:

```text
selected or suggested Intent
active Activity
ActivityTransition
ProcessInstance
ProcessTransition
Reservation
ScheduledWakeup
interruption / resume / completion state
```

`RuntimeControlUpdate`, `AcceptedRuntimeControlUpdate`,
`ActivityTransition`, `ProcessTransition`, and `DrainOutcome` are
implementation-facing architecture terms, not final Rust APIs or persistence
schemas.

Runtime control state has two commit lanes:

```text
transaction-coupled:
  RuntimeControlUpdate staged with CausalTransaction

control-only:
  AcceptedRuntimeControlUpdate through the runtime-control gate
```

The first lane is for process, reservation, schedule, and activity changes that
must be accepted atomically with a hard outcome. The second lane is for durable
actor/control choices that do not themselves mutate hard truth, such as
selected intent, suggested intent, or continuing/interruption decisions.

## Runtime Representations

| Class | Examples | Owner | Runtime role |
| --- | --- | --- | --- |
| Input | player command, NPC policy output, AI-controlled actor policy output, scheduled wakeup, reaction trigger | `EngineHost`, `Scheduler`, actor policy boundary | supplies work to evaluate |
| Authority State | hard truth, soft truth, actor truth, accepted records, runtime control state | `WorldModel` store families | source data for queries and commits |
| Derived Context | `ObservedState`, `ObservedEvent`, `EpistemicWorkingSet`, `SocialContextView`, `CapabilitySet`, `ActionRepertoire`, `PerceivedAffordance` | query, observation, actor-context owners | actor-relative readable context |
| Appraisal Intermediate | `Thought`, `Pressure`, `GoalPressure` | semantic appraisal | interpreted meaning and motivational pressure |
| Intent Choice Intermediate | `CandidateIntent`, `IntentScore` | intent planning and selection gate | choice preparation and explainable ranking |
| Commitment | `Intent` | actor controller, NPC policy, AI-controlled actor policy, or selection gate | selected or suggested purpose and approach |
| Execution Frame | `Activity` | intent/activity runtime boundary | actor-facing temporal execution meaning |
| Runtime Control Transition | `ActivityTransition`, `ProcessTransition`, `RuntimeControlUpdate`, `AcceptedRuntimeControlUpdate` | intent/activity runtime, process runtime, scheduler, runtime-control gate | durable control-state change |
| Durable Progress Frame | `ProcessInstance` | process runtime and runtime-control store | serializable execution/progress frame |
| Executable Work Item | `ActionRequest`, `ProcessTick`, `ReactionRequest`, `ResolutionTransitionRequest` | actor interface, process runtime, resolution runtime, causal runtime | request-like work to bind, validate, or tick |
| Effect Program | `Typed Effect Program` instance | typed effect subsystem inside causal runtime | checked hard-mutation logic |
| Commit Envelope | `CommitEnvelope`, `CausalTransaction`, `RuntimeControlUpdate`, accepted update envelope, invalidation package | authority owners | staged or accepted state publication |
| Publication | `TransactionRecord`, `EventRecord`, accepted records, store mutations | `CausalRuntime` and accepted update gates | durable result applied to `WorldModel`-hosted store families |

These are representation roles, not a requirement for one generic
`Representation` trait.

## Runtime Pass Contract

Every major runtime stage should be describable by the same contract shape.
Shape blocks in this document are conceptual field lists, not final Rust
schemas or trait definitions.

```text
RuntimePassContract:
  name
  owner
  pass_class
  transformation_kind
  input_representation
  output_representation
  read_surface
  allowed_reads
  allowed_writes
  authority_gate
  target_contract
  provenance_output
  invalidation_dependencies
  failure_surface
  replay_level
```

Pass contracts are architecture documentation first. They do not require an
early generic pass executor.

### Pass Classes

| Pass class | Meaning | Examples |
| --- | --- | --- |
| Input | accepts external or scheduled work | player command, scheduled wakeup, policy output |
| Derivation | produces read-only or cache-only context | observation, capability, affordance, epistemic working set |
| Choice | ranks, chooses, or proposes | intent scoring, selected/suggested `Intent` |
| Translation | converts one runtime representation to another target | `Intent -> Activity`, `Activity -> executable work item or ProcessInstance` |
| Execution | validates and interprets checked effect logic into staged state | request binding, typed effect handling |
| Publication | publishes accepted state through authority gates | transaction commit, accepted non-hard/runtime-control update |
| Invalidation | marks derived views stale or schedules recomputation | invalidation package, cache dependency updates |

### Read Surfaces

Runtime passes should declare the read surface they use:

```text
KernelQuery:
  privileged engine validation and commit-time reads

ActorRelativeQuery:
  actor-facing perception, capability, affordance, and input context

SemanticContextQuery:
  actor-relative semantic, social, epistemic, appraisal, and intent context

DebugQuery:
  inspection-only reads, not gameplay policy input
```

A query must not write durable state. If a stage writes, it is not just a
query; it must use an accepted update or transaction gate.

## Scheduler Drain

The runtime uses an explicit simulation agenda, not wall-clock frame mutation.

```text
ScheduledWakeup {
  time
  phase
  priority
  sequence
  target
}
```

Canonical ordering key:

```text
(time, phase, priority, sequence)
```

Drain contract:

```text
while due wakeups exist:
  take next wakeup by scheduler key
  bind current context
  dispatch actor activation, process tick, passive process, reaction, or
  observation/semantic work
  stop when a drain outcome is reached
```

The scheduler owns ordering and wakeups. It does not own process state, action
validation, semantic choice, or hard mutation.

Drain outcomes:

```text
DrainOutcome:
  Quiescent
  InputOpportunity
  BoundaryReached
  BudgetExceeded
  MandatoryPrompt
```

The drain loop must have a budget or fairness guard so zero-time wakeups,
same-time reactions, process loops, or semantic follow-ups cannot starve the
host. `BudgetExceeded` is a diagnostic and scheduling safety surface, not a
global determinism requirement.

Ordering requirements are scoped:

- committed results must have inspectable ordering
- same-time ordering must be debuggable
- replay-sensitive paths must record the ordering they require
- storage and projections may use faster unordered structures when ordering
  does not affect committed outcomes or declared replay output

This is not a global determinism principle. The baseline requirement is audit,
explanation, and save/load continuity. Stronger command replay is selected by
`ReplayLevel` where it is worth the cost.

## Input And Wakeup Surfaces

Initial input sources:

```text
PlayerCommand
NPCPolicyOutput
AIControlledActorPolicyOutput
ScheduledActorActivation
ScheduledProcessWakeup
PassiveProcessWakeup
ReactionWakeup
ResolutionTransitionWork
DebugOrInspectionRequest
```

Input source rules:

- player and actor policies submit attempts or choices; they do not mutate
  stores directly
- scheduled wakeups resume runtime work; they do not contain arbitrary hidden
  callbacks
- reaction wakeups enqueue work after a committed source fact; they do not
  mutate the source transaction
- debug requests can inspect and explain but cannot commit unless routed
  through explicit debugging authority outside normal gameplay

## Actor Context Path

Actor-facing context is assembled before policy input.

```text
WorldModel-hosted stores
  -> QueryLayer
  -> ObservationPipeline
  -> ActorContextPipeline
  -> CapabilitySet
  -> ActionRepertoire
  -> EpistemicWorkingSet
  -> SocialContextView
  -> PerceivedAffordance
```

This path is not omniscient. It exposes what the actor can perceive, know,
infer, or be allowed to use.

Actor context passes may cache derived views, but durable actor truth,
epistemic records, social records, and appraisal records use their own accepted
update gates.

## Semantic Decision Path

Semantic decision work is the runtime middle-end between actor-relative context
and commitment.

```text
ObservedEvent / ObservedState
  + EpistemicWorkingSet
  + SocialContextView
  + CapabilitySet / ActionRepertoire
  + PerceivedAffordance
  + visible active Intent / Activity / ProcessInstance summary
  -> AppraisalVariableSet
  -> Thought
  -> Pressure
  -> GoalPressure
  -> IntentTemplate match / candidate seed
  -> CandidateIntent
  -> IntentScore
  -> selected or suggested Intent through selection gate
  -> ActivityTransition / Activity preparation
```

Boundaries:

- `Thought` is actor-relative interpreted meaning
- `Pressure` is motivational force or action-readiness pressure
- `GoalPressure` shapes pressure toward possible desired state
- stable `Goal` may come from role, policy, schedule, long-term objective, or
  prior commitment
- `CandidateIntent` is a possible commitment generated from templates
- `IntentScore` ranks candidates with explanation
- final `Intent` is selected or suggested by actor, policy, or selection gate
- `ActivityTransition` prepares or changes temporal execution state

Semantic passes do not:

- mutate hard truth
- directly write memory or social truth
- select final intent from appraisal alone
- produce `CausalTransaction`
- emit `EventRecord`
- bypass actor-relative access rules

They may produce proposals or accepted records only through the appropriate
non-hard gate.

## Intent And Activity Lowering

`Intent` is the commitment boundary.

`Activity` is the actor-facing temporal execution boundary.

`ActivityTransition` is the conceptual control-state transition for starting,
continuing, pausing, interrupting, resuming, completing, failing, or abandoning
an activity.

Durable intent and activity changes use runtime control gates:

```text
selected / suggested Intent
  -> AcceptedRuntimeControlUpdate when durable

ActivityTransition
  -> AcceptedRuntimeControlUpdate when control-only
  -> RuntimeControlUpdate staged with CausalTransaction when tied to a hard
     outcome
```

Lowering contract:

```text
Intent
  -> Activity
  -> ActionRequest or ProcessInstance
```

Concrete resolution:

```text
Intent
  -> Activity
  -> ActionRequest
  -> Typed Effect Program
  -> CausalTransaction
```

or:

```text
Intent
  -> Activity
  -> ProcessInstance
  -> ProcessTick
  -> Typed Effect Program
  -> CausalTransaction
```

Abstract resolution:

```text
Intent
  -> Activity
  -> ProcessInstance
  -> abstract ProcessTick
  -> Typed Effect Program when hard authoritative consequence is needed
  -> CausalTransaction
```

Strategic resolution:

```text
Intent
  -> Activity
  -> region / faction / world ProcessInstance where applicable
  -> strategic ProcessTick
  -> appropriate authority gate
```

Hard rules:

- `Intent` does not mutate truth
- `Activity` does not mutate truth
- concrete `ActionRequest` is actor-owned attempted change
- `ProcessInstance` is durable progress, not hidden action spam
- abstract execution must not synthesize repeated invisible concrete
  `ActionRequest`s
- hard outcomes still pass through `CausalTransaction`

## Request Binding And Validation

Executable work items enter the causal runtime through a request-like surface:

```text
ActionRequest
ProcessTick
ReactionRequest
ResolutionTransitionRequest
```

Binding contract:

```text
request / tick / reaction
  -> source validation
  -> definition lookup
  -> role binding
  -> current-state validation
  -> reservation availability check or reservation intent where needed
  -> Typed Effect Program instance
  -> InvalidActionFeedback or staged execution
```

Process tick metadata must separate definition-level capability from
transaction-level execution. A `ProcessDef` or resolution support entry may
declare effect programs that can implement a tick, but transaction history
should record only the implementation that actually ran. If ticks can be
handled by both built-in progress semantics and interpreted effect programs,
represent that as explicit execution-mode metadata instead of a mandatory
`ProcessTick.effect_program` field.

Validation may use privileged `KernelQuery` reads. It may also compare actor
submitted context against current truth when stale actor views matter.

Validation outcomes:

```text
Rejected:
  malformed, impossible, not actor-owned, unavailable before attempt

Blocked:
  current context, target, or reservation prevents meaningful effort

AttemptFailed:
  meaningful effort happened and may leave evidence

Interrupted:
  delayed action or process was stopped by typed cause

ConflictResolved:
  competing work was ordered, denied, preempted, or merged

Committed:
  transaction committed hard outcome

Aborted:
  invariant failure committed no hard mutation
```

Ordinary gameplay failure is a domain outcome, not necessarily a Rust error.
Infrastructure errors are for corrupted data, missing definitions, violated
engine invariants, IO, serialization, or version incompatibility.

## Typed Effect Handling

`Typed Effect Program` is the checked hard-mutation IR family.

Runtime handling shape:

```text
Typed Effect Program instance
  -> TypedEffectInterpreter
  -> PrimitiveSemanticsRegistry lookup
  -> CausalTransactionBuilder
  -> staged reads
  -> staged reservations
  -> staged runtime-control updates
  -> staged RNG draws
  -> staged mutations
  -> staged EventRecord candidates
  -> staged schedule changes
```

`TypedEffectInterpreter` is an internal `CausalRuntime` role. It owns dispatch
discipline and staging capability use. It does not own commit and does not
receive raw store mutation authority.

The runtime owns the primitive semantics registry, but not the growing standard
world vocabulary. Standard primitive definitions and trusted handlers are
installed from the standard world library or trusted extension packages.

Effect handling rules:

- effect programs call typed domain effects, not unchecked field writes
- primitive calls must resolve to installed semantics before execution
- effects stage mutation through transaction APIs
- later effects in the same transaction may see earlier staged changes
- required `EventRecord` contracts are checked before commit
- RNG draws that affect committed outcomes are recorded at the declared replay
  level
- schedules, reservations, and transaction-coupled runtime-control updates are
  staged with provenance
- semantic meaning is not emitted as a hard `EventRecord`

## Causal Commit

The causal commit path is the deepest hard-mutation waist.

```text
CausalTransactionBuilder
  -> invariant checks
  -> CausalTransactionGate
  -> atomic commit package:
       TransactionRecord
       EventRecord append
       hard WorldModel store updates
       transaction-coupled RuntimeControlUpdate entries
       schedule / reservation updates
       invalidation package
```

The commit package is published as one accepted unit. The listed outputs are
not independent side effects that can partially succeed.

Only `CausalRuntime` owns:

- atomic hard commit
- `EventHistoryStore` append for committed hard records
- hard-state publication
- final transaction sequencing

`EventHistoryStore` is the committed-history API/facade for transaction and
event records. It is not a generic generated-history owner and not a semantic
meaning store.

Commit output must preserve:

- source request or process reference
- simulation time, phase, priority, and sequence where relevant
- definition and content version anchors
- staged read and validation summary where required
- mutation summary or debug trace reference where required
- emitted `EventRecord` ids
- transaction-coupled `RuntimeControlUpdate` ids where present
- invalidation package
- replay/audit provenance required by the selected `ReplayLevel`

## Non-Hard Accepted Updates

Not every durable update is a hard physical transaction.

Non-hard state classes use their own accepted update gates:

```text
AcceptedSocialUpdate
AcceptedChronologyRecord
AcceptedEpistemicUpdate
AcceptedAppraisalRecord
AcceptedRuntimeControlUpdate
```

Rules:

- social, chronology, epistemic, and appraisal gates are outside
  `CausalTransaction`
- runtime control has both transaction-coupled `RuntimeControlUpdate` and
  control-only `AcceptedRuntimeControlUpdate` lanes
- accepted non-hard updates still leave durable envelopes
- accepted envelopes include provenance, source, ordering, version anchors,
  and invalidation output
- non-hard records may reference committed `EventRecord`s as evidence
- non-hard records do not retroactively rewrite hard facts
- runtime-control records do not become `EventRecord`s unless a hard
  transaction emits hard evidence

This keeps social, epistemic, chronology, appraisal, and control-only runtime
state durable without making the hard causal transaction responsible for every
kind of meaning or control choice.

## Process Runtime

`ProcessInstance` is explicit serializable runtime state.

```text
ProcessInstance {
  id
  definition
  owner
  roles
  state
  progress
  active_resolution
  wait_condition
  reservations
  interrupt_policy
  resume_policy
  failure_policy
  version
}
```

State set:

```text
created
scheduled
waiting
advancing
paused
interrupted
resumed
completed
failed
abandoned
```

`completed`, `failed`, and `abandoned` are terminal or near-terminal states
unless a process definition explicitly supports recovery.

Process runtime computes transitions:

```text
ProcessTick / process wakeup
  -> ProcessTransition
  -> RuntimeControlUpdate when durable
  -> optional executable work item
```

Process runtime rules:

- processes do not mutate hard state or `RuntimeControlStore` directly
- processes compute `ProcessTransition` and may emit executable work items
- concrete actor-facing continuations may emit `ActionRequest`
- abstract or strategic progress advances through `ProcessTick`, not hidden
  concrete action spam
- processes request reservation acquire/release through causal runtime or the
  runtime-control gate
- wait conditions and progress must survive save/load
- interrupt and resume decisions must be inspectable
- a process may tick at concrete, abstract, or strategic resolution only when
  its definition supports that resolution

## Invalidation And Derived Views

Commits publish invalidation, not silent cache mutation.

Invalidation package shape:

```text
InvalidationPackage:
  source envelope
  changed authority classes
  changed store families
  changed entity / relation / location / process ids
  changed EventRecord ranges
  affected query keys or dependency classes
  affected actor-relative views
  affected semantic context
  recommended eager recomputation?
```

`DerivedViewRegistry` consumes invalidation packages and decides whether a
view is:

```text
valid
stale
partially stale
needs rebuild
needs actor-relative refresh
```

Views may be recomputed eagerly or lazily. The architecture only requires that
derived views are not mutated as source truth and that stale actor-facing views
are handled explicitly.

## Observation And Post-Commit Meaning

After committed hard outcomes, the engine may project observations and later
meaning.

```text
EventRecord / changed hard state
  -> ObservationPipeline
  -> ObservedEvent / ObservedState
  -> dependency-ordered follow-up proposals:
       Epistemic persistence proposal
       Social update proposal
       AppraisalVariableSet / Thought / Pressure / GoalPressure
       future intent bias or candidate generation
```

This path is not part of the original hard transaction. It interprets committed
facts for holders and actors.

Rules:

- `EventRecord` is evidence, not social or emotional meaning
- observation is actor-relative
- epistemic persistence uses accepted epistemic update gates
- appraisal records use accepted appraisal gates
- social/institutional updates use social commit gates
- accepted social updates invalidate or refresh `SocialContextView` before
  dependent appraisal where needed
- future intent bias is not immediate execution

## Resolution Runtime

`ResolutionRuntime` decides execution detail level and materialization
boundaries. It does not own separate mutation authority.

Resolution targets:

```text
concrete:
  Activity -> ActionRequest or ProcessInstance

abstract:
  Activity -> ProcessInstance

strategic:
  Activity -> region / faction / world ProcessInstance where applicable
```

Promotion and demotion:

```text
abstract ProcessInstance
  -> promotion boundary
  -> refined local state and Activity view
  -> future concrete ActionRequest or local ProcessTick

concrete Activity / ProcessInstance
  -> demotion boundary
  -> coarser ProcessInstance state
  -> future abstract ProcessTick
```

If promotion or demotion creates, removes, relocates, refines, or coarsens hard
state, it commits through `CausalTransaction`.

## Diagnostics And Provenance

The runtime should produce structured explanation data rather than relying on
logs.

Diagnostic audiences:

```text
authoring:
  pack source, symbol, type, stage permission, verifier error

actor-facing:
  rejected, blocked, failed, interrupted, or continued explanation

debug / inspection:
  full provenance over queries, pass results, lowering, validation, staging,
  commit, invalidation, and observations
```

Provenance packet shape:

```text
ProvenancePacket:
  source definition id
  source span?
  actor / holder
  input source
  query keys read
  EventRecord refs
  rule or template match refs
  selected lowering target
  validation result
  staged mutation refs
  emitted record ids
  invalidation package id
  replay level
```

Not every runtime path needs to store full debug detail forever. The
architecture requires enough durable provenance for the selected audit,
explanation, save/load, and replay level.

## Replay Levels

Replay is tiered.

```text
AuditOnly:
  inspect committed records, ordering, validation summary, and provenance

EventRebuild:
  rebuild committed consequences from transaction and event history where the
  subsystem declares that support

DeterministicCommandReplay:
  rerun accepted input logs and expect matching transactions and events for
  selected subsystems, tests, or debug modes
```

Baseline requirement:

```text
auditability
explainability
save/load continuity
committed ordering where it affects meaning
version anchors
```

The runtime may choose faster or more flexible implementation strategies when
full deterministic command replay is not required. If ordering, RNG, thread
scheduling, batching, or cache timing can affect committed output, the selected
path must either record enough provenance to explain/rebuild the accepted
outcome or declare a stronger `ReplayLevel`.

## Accelerator Boundaries

Accelerators are allowed behind explicit boundaries.

Possible accelerators:

```text
ECS view
graph index
Datalog-like derived closure
incremental query database
dataflow projection
pathfinding cache
script/plugin sandbox
```

Allowed:

- optimize hot local queries
- store materialized projections
- batch read-only derivations
- accelerate semantic matching
- support inspection and tooling

Forbidden:

- direct hard mutation
- owning durable story identity
- bypassing actor-relative query access
- publishing `EventRecord`s directly
- treating cache state as source truth
- committing accepted records without their accepted update gates

## Rust Implementation Constraints

The core runtime should be sync-first. Async IO, network, asset loading, and
plugin host work belong at `EngineHost` or adapter boundaries.

Query passes should return ids, value snapshots, handles, or read tokens. A
pass should not hold a long-lived `&WorldModel` across staging or commit.
`CausalRuntime` should take short exclusive access only at the authority gate
where the accepted package is published.

## Runtime Pipeline Summary

The target architecture is:

```text
WorldModel-hosted stores
  -> QueryLayer
  -> actor-relative context and semantic passes
  -> Intent
  -> Activity
  -> ActivityTransition or ProcessInstance
  -> executable work item when needed
  -> Typed Effect Program
  -> CausalTransaction
  -> EventRecord + store/runtime-control updates + invalidation
  -> observation, accepted updates, semantic follow-up, scheduling
```

The structure is complex, but the complexity is contained by pass contracts.
Each pass has a small job. No pass owns the whole runtime.

## Deferred

Defer until crate architecture or implementation planning:

- exact Rust module boundaries
- final public APIs
- final source syntax
- concrete storage backend
- concrete ECS or graph integration
- exact persistence format
- exact diagnostic renderer
- first vertical slice
- standard game-system packs

## Next Document

The crate boundary architecture now lives in
[Crate Boundary Architecture](crates.md). The next architecture step should be
an implementation plan once the crate boundary is stable enough.
