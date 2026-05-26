# Causal Runtime / Action-Effect-Event

## Status

Draft research

## Axis

[Causal Runtime / Action-Effect-Event](engine-architecture-entry.md)

## Design Outputs

- [Causal Runtime](../design/causal-runtime.md)
- [Typed Effect Primitives](../design/typed-effect-primitives.md)
- [Time Model](../design/time-model.md)

## Design Resolution Note

The current design docs supersede several candidate terms in this research
draft:

- actor commands from an AI-controlled NPC are `AI-controlled actor policy`, not
  `AI proposal`; `AI proposal` is reserved for soft truth, actor truth, or
  narrative proposals.
- semantic appraisal may create `Thought`, `Pressure`, `GoalPressure`, proposed
  epistemic updates, proposed intent bias, and semantic records; it does not
  directly mutate memory, belief, or final intent.
- `EventRecord` is the hard causal event record. Semantic/social meaning
  uses typed soft-truth or appraisal records.

## Core Question

```text
How does an attempted change become validated world mutation and recorded fact?
```

## Why This Must Be Researched Together

The causal runtime is the engine's mutation discipline.

Action granularity, typed effect execution, process progress, interruption,
event granularity, deterministic ordering, RNG, replay, and provenance cannot
be designed independently. If an action can mutate state without event
contracts, the semantic layer loses its evidence. If a long activity keeps
progress outside the scheduler, interruption and save/load become fragile. If
events are too coarse, memory and debugging become weak. If events are too
fine, the log becomes an unusable mutation trace.

For `world`, this axis must answer one coupled problem:

```text
What path must every hard world change follow, and what record proves it
happened?
```

## Scope

In scope:

- action request submission and lifecycle
- validation, binding, failure, and attempt semantics
- typed effect programs over kernel primitives
- staged mutation and atomic causal transaction commit
- turn, tick, and discrete-event scheduling
- long-running process/activity runtime
- reservations, conflict, interruption, and reaction scheduling
- event schema, event granularity, provenance, and observation projection hooks
- deterministic RNG, replay, snapshots, and save/load implications
- PL-inspired effect checking and interpreter boundaries

Out of scope:

- final physical simulation vocabulary
- actor-owned capability derivation
- target affordance derivation
- semantic/social interpretation of events
- AI soft-truth gates
- final storage/backend choice
- final surface syntax for an action DSL

Those topics depend on this axis, but they are not settled here.

## Terminology Alignment

This document is aligned with
[World Representation / Query Model](world-representation-query-model.md).

Use these names consistently:

- `CausalTransaction`: one staged and committed mutation attempt.
- `CausalTransactionGate`: the world representation boundary that accepts only
  typed effect programs and commits through `CausalTransaction`.
- `Reservation`: temporary hard runtime state for process/action conflict
  resolution. This replaces the ambiguous runtime use of `Claim`.
- `SocialClaim`: social, legal, institutional, or customary assertion. Causal
  runtime may read it through context/query layers, but physical actions should
  not create social meaning directly.
- `EventHistoryStore`: the hard event/history family that stores transaction
  records, `EventRecord`s, generated historical records, and long-term history
  references.

Do not use plain `Claim` in this axis. Runtime conflict control is
`Reservation`; social meaning is `SocialClaim`.

## Research Inputs

This pass used three parallel research lenses:

- PL and formal semantics: typed effects, effect handlers, rewrite semantics,
  transaction boundaries, planning/action formalisms, and verification.
- Simulation runtime: discrete-event scheduling, process interaction, event
  sourcing, deterministic replay, rollback, RNG, snapshots, and provenance.
- Game/runtime pressure: CDDA activities, RimWorld-style jobs and
  reservations, roguelike action time/interruption, Qud-style event reactions,
  and long-running RPG activities.

## Theory Baseline

### Discrete-Event And Process Simulation

Discrete-event simulation gives a useful time model:

- simulation time is separate from wall-clock time
- future work is represented by scheduled events
- events execute in deterministic time order
- tied events need a stable tie-breaker
- long-duration work should schedule later completion, not consume hidden
  wall-clock time

ns-3 is a useful reference because its manual makes the scheduler explicit:
events are stored in a scheduler data structure, handled in increasing
simulation time, and same-time events are handled by a stable insertion order.
It also distinguishes cancelling a scheduled event from removing it.

SimPy is useful for process vocabulary:

- events have states: not triggered, triggered, processed
- processes can wait on events and can themselves be waited on
- conditions such as any-of/all-of model waiting for multiple causes
- interrupts resume a process early with an explicit cause
- resources model queues, priority, capacity, and preemption

Transfer:

- Use a deterministic event agenda for delayed effects, passive processes,
  travel, wounds, rituals, weather, fire, scent, and abstract offscreen work.
- Use process-style waiting, interruption, and resource pressure.
- Treat "same time" as a design problem requiring explicit ordering.

Adapt:

- SimPy-like processes should become serializable `ProcessInstance` records,
  not stackful generator/coroutine saves.
- ns-3-style scheduler events should be typed wakeups or requests, not opaque
  callback mutation.

Reject:

- hidden callback events as the domain model
- stackful coroutine state as save data
- same-time execution that depends on map iteration, thread timing, or object
  memory layout

### Event Sourcing And Command/Event Models

Event sourcing provides a useful command/event separation:

```text
command / request:
  asks to change state

event:
  records a committed fact

projection:
  read model derived from committed `EventRecord`s
```

The useful lesson is not that `world` must store only events. The useful lesson
is that committed change needs immutable, replayable evidence, and projections
must be rebuildable or explainable.

Transfer:

- Actions are attempts; events are facts.
- A command/action handler should not mutate current state as its first act.
- Committed events need versioning and provenance.
- Snapshots are checkpoints, not replacements for causal history.
- Event design should preserve domain intent, not only final field deltas.

Adapt:

- Use events as the authoritative causal record of committed changes.
- Use snapshots for normal save/load performance.
- Keep read-side projections and derived views separate from mutation.
- Consider compensating events for corrections instead of rewriting history.

Reject:

- pure event sourcing as the only runtime state representation
- events that are merely `field changed from x to y`
- event streams without schema/version/upcast policy

### Deterministic Replay And Rollback

Deterministic lockstep and rollback systems are useful because they expose what
replay actually requires:

- fixed simulation inputs
- stable ordering
- deterministic logic
- deterministic RNG
- no untracked external side effects
- fast snapshot/load when rollback or debugging is needed

`world` does not need multiplayer rollback as a primary constraint, but it does
need the same discipline for debugging, AI audit, save verification, and
simulation trust.

Transfer:

- Define the deterministic hard path explicitly.
- Record enough input, version, seed, RNG, and event provenance to replay.
- Use state hashes or checkpoints to detect divergence.

Adapt:

- Keep two replay modes:
  - command replay: recompute from genesis, seed, content version, and
    action/process inputs
  - event replay: rebuild state from committed `EventRecord`s without rerunning
    stochastic rules
- Treat rollback as a debug/speculative capability, not the main game model.

Reject:

- replay that depends on current wall-clock time, nondeterministic iteration, or
  live AI calls
- a single global RNG stream whose draw order changes whenever any rule changes

### Typed Effects And Effect Handlers

Algebraic effects and typed effect systems are useful as design theory, not
necessarily as a literal implementation language.

The central idea to transfer is:

```text
An authored rule should declare which effects it is allowed to perform.
An interpreter/handler decides what those effects mean in a given mode.
```

Example stage permissions:

```text
PhysicalAction:
  ReadWorld
  Validate
  Reserve
  Rng
  MutatePhysical
  EmitPhysicalEvent
  EmitSensoryEvent
  Schedule

SemanticRule:
  ReadObservedEvents
  ReadEpistemicWorkingSet
  ReadSocialContextView
CreateThought
CreatePressure
ProposeEpistemicUpdate
ProposeIntentBias
ProposeAppraisalRecord

AIProposal:
  ReadActorView
  ProposeAction
```

This is valuable because it makes layer violations checkable. A physical
action should not directly mutate belief, relationship, guilt, revenge,
narrative progress, or legal meaning. A semantic rule should not directly move
an actor, unlock a door, or apply damage.

Effect handlers also suggest multiple interpreters for the same action
definition:

```text
ActionDef
  -> validation / explanation interpreter
  -> dry-run preview interpreter
  -> commit interpreter
  -> abstract-planning interpreter
  -> replay / audit interpreter
  -> property-test or model-check interpreter
```

Transfer:

- Use typed effect permissions as the PL-shaped boundary checker.
- Lower authored actions and processes into typed effect IR.
- Keep the IR interpretable in multiple modes.

Adapt:

- Use algebraic-effect-inspired IR, not full unrestricted continuations.
- Prefer domain-specific kernel effects over generic mutation effects.
- Let effect types express capabilities such as `MutatePhysical` or
  `EmitEvent`, not vague "side effect" buckets.

Reject:

- content DSLs with arbitrary callbacks into engine internals
- multishot/full continuation power in ordinary gameplay content
- generic `SetField(entity, field, value)` as a gameplay primitive

### Rewrite Semantics, Planning, And Verification

K, TLA+, Verse calculus, and PDDL are useful as research pressure:

- K shows that a language can have executable rewrite semantics, with explicit
  state cells and rules.
- TLA+ shows why the transition relation and invariants should be specified
  before building complex concurrent/stateful machinery.
- Verse calculus is a relevant PL/game-adjacent example because it gives a
  deterministic small-step semantics for a richer logic-like language.
- PDDL shows a simple action form with typed parameters, preconditions, and
  effects.

Transfer:

- Specify `CausalTransaction` as a transition relation before implementing many
  features.
- Keep action definitions close to typed parameters, requirements, and effects.
- Make invalid states and invariant checks first-class.

Adapt:

- Use formal methods selectively on small core protocols:
  - scheduler ordering
  - transaction commit
  - reservation conflict
  - replay determinism
  - event/version invariants

Reject:

- making the whole game a theorem-proving project before the engine exists
- PDDL-style planning as the full runtime action model

## Reference Pressure

### CDDA Activities

CDDA activities are long-term actions that can be interrupted and sometimes
continued. Its activity documentation names the important lifecycle pieces:

- activity definition
- activity actor state
- `start`
- per-turn `do_turn`
- `finish`
- `canceled`
- `can_resume`
- backlog/resume behavior
- serialization for save/load
- time-based, speed-based, or custom progress

Transfer:

- Long-running work must be first-class serializable state.
- Interruption and resume policy must be explicit.
- Per-turn advancement must avoid infinite loops.

Adapt:

- Replace subclass-per-activity opacity with typed `ProcessDef` /
  `ProcessInstance`.
- Require every process mutation to pass through typed effects and events.

Reject:

- activity code that directly mutates arbitrary world state without event
  contracts
- progress stored only in UI/animation/conversation state

### CDDA Effect-On-Condition And Events

CDDA's effect-on-condition system demonstrates both power and danger. It can
run effects when conditions pass, can run on recurrence, and can trigger from
events. Its event docs expose event context variables and typed event fields.

Transfer:

- Event-triggered rules are useful.
- Recurring rules and event rules need explicit context variables.
- Event data should have typed fields.

Adapt:

- Use event-triggered rules as reaction request producers.
- Route event-triggered hard mutation back through `CausalTransaction`.

Reject:

- event listeners that mutate hard truth inside the original transaction
- stringly effect scripts with unclear stage permissions

### RimWorld Jobs And Long Work

The local RimWorld reference note pressures this axis toward a middle layer
between intent and atomic action:

```text
standing assignment / need / duty
  -> job
  -> steps
  -> world-changing attempts
```

The transferable lesson is not colony management. The transferable lesson is
that long work needs decomposition, reservations, fail conditions, and
interruptions.

Transfer:

- `Task` or `Process` owns progress and next-step selection.
- Actor actions stay granular and replayable.
- Reservations are required once multiple actors can target the same
  resource, place, patient, workstation, conversation target, or ritual role.

Adapt:

- Use standing work requests for guards, priests, monsters, factions, and
  offscreen groups, but keep the single-protagonist RPG as the main interface.

Reject:

- a full colony job scheduler as the core player experience
- AI or director jobs that bypass the action/effect/event boundary

### Roguelike Turns, Delays, And Interruption

Roguelikes pressure the model toward:

- action cost
- delayed actions
- rest/travel automation
- interruption by new danger or player input
- action-point or energy scheduling
- deterministic turn ordering

Transfer:

- Treat "rest until interrupted", "travel to known site", "search room",
  "continue aiming", "read until danger", and "perform ritual" as processes,
  not single actions.
- Interruption should be caused by typed signals: observed danger, damage,
  keypress, resource loss, target moved, reservation conflict, status change,
  or process-specific condition.

Adapt:

- The player may use process conveniences without losing direct turn-by-turn
  control.

Reject:

- automation that continues through visible danger
- interruption logic that leaks unseen hard truth

### Qud-Style Event Reactions

Caves of Qud's object parts and event style are useful because they show how
rich object behavior can emerge from reacting to events.

Transfer:

- Many systems should be able to react to typed facts.
- Object/item/condition-specific behavior can be modular.

Adapt:

- Event subscribers should enqueue reaction requests, process ticks, or
  semantic interpretations.
- They should not mutate the original transaction directly unless they are
  explicitly part of the same effect program.

Reject:

- arbitrary listener mutation with unclear order
- unbounded event cascades without depth/time limits

## Observations

### Observation: `ActionRequest` Is Not The Only Narrow Waist

`ActionRequest` is the actor-facing attempted-change interface. It is not the
deepest mutation boundary.

Better shape:

```text
Actor / AI / Process / Reaction
  -> ActionRequest or ProcessTick
  -> Typed Effect Program
  -> CausalTransaction
  -> Event Log + Observations + Reactions
```

Inference:

The real mutation waist should be `CausalTransaction`. Actor actions, passive
processes, magic, crafting, fire spread, reaction effects, and abstract
simulation materialization all need to commit through the same transaction
discipline.

### Observation: Effects Need Staging

An effect program should not directly mutate authoritative state.

Better:

```text
run effect program
  -> staged reads
  -> staged reservations
  -> staged RNG draws
  -> staged mutations
  -> staged events
  -> invariant checks
  -> atomic commit or abort
```

Inside a transaction, later effects may see earlier staged changes. Outside
the transaction, nothing has happened until commit.

Inference:

This gives validation, dry-run, explanation, rollback, replay, and invariant
checking a shared structure.

### Observation: Process Is Serializable Continuation

A long activity is not a big action. It is a state machine that emits or
continues ordinary attempts over time.

Inference:

`ProcessInstance` should store:

- process id
- process kind
- owner actor or source
- target roles
- local state
- progress
- wait condition
- reservations
- interrupt policy
- resume policy
- failure policy
- provenance
- deterministic RNG state or draw references

### Observation: Reservations Are Causal State

Reservation data should not be a UI-only convenience.

Inference:

Reservations need to be part of the causal runtime because they affect
validation, conflict, task selection, interruption, and replay.

Candidate:

```text
Reservation {
  id
  owner_process_or_action
  scope: actor | entity | location | resource | role | time_slot
  mode: exclusive | shared | capacity(n)
  priority
  expires_at_or_revalidates
  release_on: complete | fail | interrupt | abandon
  provenance
}
```

### Observation: Events Need A Layered Granularity

One event per raw primitive mutation is often too noisy. One event per
user-facing action is often too coarse.

Inference:

Use layered records:

```text
TransactionRecord:
  committed batch envelope

EventRecord:
  meaningful fact for replay, observation, semantic interpretation, memory,
  and debugging

MutationTrace:
  optional/internal primitive-level trace for audit and tooling

ObservationEvent:
  actor-relative projection produced after commit

SemanticEvent:
  later interpretation, not emitted by physical action effects
```

The exact storage split can change, but the conceptual split matters.

### Observation: Failure Is Not One Thing

Failure semantics need typed distinctions:

```text
Rejected:
  request malformed, impossible, not actor-owned, or unavailable before any
  attempt enters the world

Blocked:
  target/context/reservation prevents the attempt before meaningful effort

AttemptFailed:
  actor tried; effort, sound, damage, time, or observation may result

Interrupted:
  process or delayed action stopped by a typed interrupt signal

ConflictResolved:
  simultaneous or competing actions were ordered, denied, preempted, or merged

Aborted:
  transaction failed invariant checks and committed no hard mutation
```

Inference:

Only some failures should produce public world events. All should produce
debug/provenance records.

## Proposed Model

### Core Flow

```text
InputSource
  player command | NPC policy | AI-controlled actor policy | process wakeup
  passive process | reaction request | resolution transition work
    |
    v
ActionRequest or ProcessTick
    |
    v
Bind roles and context
    |
    v
Preflight validation
    |
    v
Typed Effect Program
    |
    v
CausalTransaction
  staged reads
  staged reservations
  staged RNG
  staged mutations
  staged events
  staged schedules
    |
    v
Invariant check
    |
    v
Atomic commit
    |
    v
EventHistoryStore append
  + scheduler update
  + process update
  + reservation update
  + state update
    |
    v
Observation projection
    |
    v
Epistemic persistence / semantic appraisal / pressure / future intent bias
```

### Core Records

```text
ActionRequest {
  id
  actor
  schema
  roles
  mode
  declared_intent?
  source: player | npc_policy | ai_controlled_actor | process | reaction
  submitted_at
  actor_view_version?
}

ActionDef {
  id
  typed_roles
  requirements
  binding_rules
  effect_program
  event_record_contract
  stage_permissions
  version
}

EffectProgram {
  typed_ir
  allowed_effects
  required_event_records
  deterministic_control_flow
}

CausalTransaction {
  id
  sim_time
  phase
  sequence
  source_request_or_process
  read_set?
  reservation_set
  rng_draws
  mutation_set
  event_record_set
  schedule_set
  invariant_results
}

TransactionRecord {
  id
  sim_time
  phase
  sequence
  source_request_or_process
  event_record_ids
  mutation_trace_ref?
  snapshot_hash_after?
}

EventRecord {
  id
  tx_id
  time
  sequence
  kind
  roles
  data
  causal_parents
  source_action?
  source_process?
  rng_refs?
  schema_version
  content_version
}

ProcessInstance {
  id
  kind
  owner
  roles
  state
  progress
  wait_condition
  reservations
  interrupt_policy
  resume_policy
  failure_policy
  source_event_record_or_action?
  version
}

ReactionRequest {
  id
  source_event
  target_rule
  priority
  scheduled_time
  permissions
}
```

### Scheduler

Detailed time-model research is tracked separately:

- [Time Model / Turn Scheduling](time-model-and-turn-scheduling.md)

Candidate deterministic key:

```text
(sim_time, phase, priority, insertion_sequence)
```

Possible phases:

```text
input collection
actor action resolution
process advancement
passive physical processes
reaction scheduling
observation projection
semantic interpretation
cleanup / snapshot
```

Rules:

- Time is integer or fixed-point simulation time.
- Scheduler ordering must not depend on hash-map iteration.
- Same-time ordering must be visible in debug tools.
- Zero-time reactions need bounded depth or delayed scheduling to prevent loops.
- Long-duration work schedules future wakeups rather than blocking a turn.

### Effect IR Permissions

Candidate hard-action effect permissions:

```text
ReadWorld
ReadActorOwnedState
ReadDerivedEngineView
ReadSubmittedBinding
Validate
AcquireReservation
ReleaseReservation
Rng
MutatePhysical
MutateProcess
EmitPhysicalEvent
EmitSensoryEvent
ScheduleProcess
ScheduleReaction
```

Forbidden in hard physical actions:

```text
MutateMemory
MutateBelief
MutateRelationship
MutatePressure
DeclareCrime
DeclareTheft
EmitNarrativeMeaning
CallAI
WriteUIMessageAsTruth
SetFieldUnchecked
```

Candidate semantic permissions:

```text
ReadObservedEvents
ReadEpistemicWorkingSet
ReadSocialContextView
CreateThought
CreatePressure
CreateGoalPressure
ProposeEpistemicUpdate
ProposeIntentBias
ProposeAppraisalRecord
```

Forbidden in semantic rules:

```text
move_entity
transfer_entity
apply_damage
set_open_state
set_lock_state
MutatePhysical
```

### Process Lifecycle

```text
ProcessStarted
ReservationAcquired
ProcessAdvanced
ProcessPaused
ProcessInterrupted
ProcessResumed
ProcessCompleted
ProcessFailed
ProcessAbandoned
ReservationReleased
```

Lifecycle functions:

```text
start:
  create process state, acquire initial reservations, schedule first wakeup

advance:
  validate continuation, submit ActionRequest or ContinueProcessRequest,
  update progress through typed effects

finish:
  validate completion, commit final mutations and release reservations

cancel:
  release reservations, optionally preserve resumable state

interrupt:
  consume InterruptSignal, pause/fail/continue according to policy

resume:
  revalidate state and reservations, then schedule or advance
```

Processes should be able to represent:

- reading
- searching
- treating wounds
- crafting
- building
- rest/travel automation
- tracking
- aiming or charging an attack
- ritual performance
- hauling or standing work
- passive physical processes such as fire spread or healing

### Interrupt Signals

Candidate interrupt sources:

```text
PlayerInput
ObservedDanger
ActorDamaged
TargetMoved
TargetDestroyed
ReservationLost
ToolLost
ResourceMissing
StatusChanged
NoiseHeard
LightChanged
PathInvalidated
ScheduleExpired
SemanticControlStateChanged
```

Rules:

- Interrupts should be actor-relative when they depend on perception.
- Hidden hard truth should not interrupt player automation unless it produces
  an observable or physically forced signal.
- Interrupt results should be structured events or private feedback.

### Reaction Rules

Event reactions should not mutate the original transaction directly.

Preferred flow:

```text
CommittedEvent
  -> reaction rule reads event and context
  -> enqueues ReactionRequest or ProcessTick
  -> later CausalTransaction validates and commits effects
```

This prevents:

- order-dependent hidden listener mutation
- infinite event cascades inside one commit
- semantic rules bypassing physical action boundaries

Some purely derived projection updates may happen immediately after commit, but
they should be rebuildable and should not be new hard truth unless committed by
a later transaction.

## Replay And Save Contract

Hard deterministic replay requires:

- engine version
- content/schema version
- genesis state or snapshot hash
- EventHistoryStore cursor
- stable entity ids
- deterministic scheduler order
- deterministic RNG streams
- action/process input log
- committed EventHistoryStore transaction records and `EventRecord`s
- optional read/write sets or dependency summaries
- state hash checkpoints

Candidate RNG model:

```text
RngStream {
  stream_id: actor | process | transaction | system
  seed
  counter
}

RngDraw {
  stream_id
  counter_before
  distribution
  result
  purpose
}
```

Use named streams or substreams so adding an unrelated random draw does not
silently change the whole world.

Replay modes:

```text
Command replay:
  genesis + seed + content version + action/process inputs
  -> recompute transactions and events

Event replay:
  snapshot or genesis + committed event_record_set
  -> rebuild state and projections without rerunning resolution logic

Debug replay:
  command replay + event comparison + state hash checkpoints
```

Save/load:

- Normal saves can store snapshot plus EventHistoryStore cursor.
- EventHistoryStore remains the causal audit trail.
- Snapshots must include schema/content version and state hash.
- Loading should know whether it is resuming, migrating, or replaying.

## Candidate Models

### Candidate A: Direct Resolver Runtime

Sketch:

```text
ActionRequest -> resolver mutates WorldState -> emit optional log
```

Makes easy:

- simple first implementation
- straightforward imperative code

Makes hard:

- replay
- debugging
- long activities
- event granularity
- typed stage permissions
- semantic interpretation
- AI auditing

Assessment:

Reject as architecture. It can exist only behind a typed transaction/effect
interface.

### Candidate B: Pure Event Sourcing Runtime

Sketch:

```text
ActionRequest -> event_record_set -> replay event_record_set to get state
```

Makes easy:

- audit log
- temporal queries
- rebuild projections
- event-driven semantics

Makes hard:

- hot simulation queries
- derived state performance
- event schema evolution
- large-world save/load
- partial replay during local simulation

Assessment:

Adapt the event-sourcing discipline, not the pure storage model. Use snapshots,
typed world stores, and committed `EventRecord`s together.

### Candidate C: Coroutine Process Simulation

Sketch:

```text
process coroutine yields event/time/resource waits
```

Makes easy:

- readable long-running behavior
- natural wait/interrupt code

Makes hard:

- save/load
- versioning
- replay
- inspection
- deterministic debugging
- content migration

Assessment:

Use the vocabulary but reject stackful coroutine persistence. Store explicit
process state machines.

### Candidate D: Typed Effect Transaction Runtime

Sketch:

```text
ActionRequest / ProcessTick
  -> typed effect IR
  -> staged CausalTransaction
  -> committed `EventRecord`s and state
```

Makes easy:

- unified hard mutation path
- stage permission checking
- long process integration
- deterministic replay
- event provenance
- AI/action audit
- dry-run/explanation interpreters

Makes hard:

- more up-front design
- effect vocabulary discipline
- `EventRecord` contract design
- transaction staging complexity
- schema/versioning work

Assessment:

Strongest candidate for `world`.

## Test Scenarios

### Lockpick Interrupted By Guard

Initial state:

- player starts `PickLockProcess` on a bronze gate
- process has a lockpick tool reservation and gate interaction reservation
- progress is 2 / 5
- guard enters visible range

Expected:

- guard movement commits as an event
- observation projection creates `ObservedDanger` for the player
- process receives an interrupt signal
- player may continue, pause, cancel, or switch action
- lockpick progress, noise, and reservations remain explainable

Reveals:

- process interruption must be event/observation-driven
- hidden guard outside perception should not leak as automation stop

### Two Healers, One Patient

Initial state:

- two NPCs decide to treat the same wounded actor
- one bed and one clean bandage exist

Expected:

- first healer acquires patient/bed/bandage reservations
- second healer receives `ReservationDenied` or chooses alternate support
- if first healer is attacked, reservations release or transfer according to
  policy

Reveals:

- reservations are causal state, not UI hints
- conflict resolution must be deterministic

### Ritual With Observable Preparation

Initial state:

- cultist starts a four-turn ritual
- ritual reserves circle, relic, voice, and participant role
- each turn emits sensory tells

Expected:

- player sees chanting, symbol glow, or component use
- stun/silence/damage/reservation loss may interrupt
- completion commits physical effects and emits events
- semantic interpretation of taboo or sacrilege happens later

Reveals:

- delayed actions need preparation, progress, interruption, and `EventRecord`
  contracts

### Passive Fire Spread

Initial state:

- torch ignites dry cloth
- fire process is scheduled
- nearby smoke and heat fields are observable

Expected:

- fire spreads through the same effect/transaction path as actions
- RNG draw for spread is recorded
- smoke/heat sensory events feed observation
- no passive process mutates world outside transaction

Reveals:

- passive physics and actor actions need the same causal discipline

### Failed Forced Door

Initial state:

- wounded actor attempts to force a stuck door

Expected:

- if impossible before effort: private `Blocked` feedback may be enough
- if effort is made: emit `ForceOpenAttempted`, `ForceOpenFailed`, possible
  pain and noise events

Reveals:

- validation failure and attempted failure need different semantics

### Replay With RNG

Initial state:

- actor attacks monster
- hit chance and damage use RNG

Expected:

- command replay produces the same hit/damage with same versions and RNG
  streams
- event replay can rebuild state from committed hit/damage event without
  rerolling
- adding an unrelated weather RNG draw should not change combat result

Reveals:

- RNG stream policy is part of the causal runtime

### Event Reaction Without Cascade Mutation

Initial state:

- `ActorDied` commits
- nearby beast has a blood-scent reaction
- faction rule has a witness-report reaction

Expected:

- reactions enqueue later requests or semantic rules
- original death transaction does not mutate beast intent or faction pressure
  directly
- reaction order is deterministic and inspectable

Reveals:

- event listeners must not become hidden mutation paths

### Abstract Caravan Materializes

Initial state:

- abstract caravan process has `CaravanDelayed` and `TracksLeft`
- player approaches local area

Expected:

- materialization uses a causal transaction
- local entities/events point back to abstract event provenance
- generated scene is explainable from abstract state

Reveals:

- multi-resolution simulation depends on causal transaction provenance

## Design Decisions

This axis must eventually settle:

- Is `CausalTransaction` the deepest mandatory mutation waist?
- What is the exact `ActionRequest` lifecycle?
- What is the minimum typed effect IR?
- What effect permissions are checked first?
- What mutations require mandatory `EventRecord` contracts?
- What is the event granularity rule?
- Which failures become public events, private feedback, or debug-only records?
- What scheduler key and phase model are canonical?
- How are same-time actions resolved?
- Are reservations kernel runtime state?
- What is the first `ProcessInstance` shape?
- How are process wait conditions, interruption, and resume serialized?
- How do event reactions enqueue work without hidden mutation?
- What replay mode is authoritative for debugging?
- How much RNG provenance is stored in normal saves?
- How are event schemas versioned and migrated?
- Which invariants are checked at transaction commit?
- Which parts deserve formal specification first?

## Takeaways For `world`

Keep:

- `ActionRequest` as the actor/AI/process-facing attempted-change interface
- actions as typed effect programs over kernel primitives
- events as committed facts, not UI text
- process/activity as serializable state machines
- deterministic scheduler ordering
- staged transactions before atomic commit
- reservations as causal runtime state
- RNG streams and provenance
- event reactions that enqueue later work
- observation projection after commit

Adapt:

- SimPy process/event/resource vocabulary into explicit process state
- ns-3 event agenda and tie-breaking into scheduler semantics
- event sourcing into committed `EventRecord`s, projections, snapshots, and
  schema/version policy
- algebraic effects into checked effect permissions and multiple interpreters
- K/TLA+ into small formal specs for scheduler/transaction invariants
- CDDA activity lifecycle into process lifecycle
- RimWorld job/reservation pressure into process reservations

Reject:

- hidden mutation in action resolvers, activities, or event listeners
- generic unchecked `SetField`
- pure event sourcing as the only runtime state model
- stackful coroutine save state
- global RNG draw streams
- event/history records that are only prose or UI text
- semantic meaning emitted by physical action effects
- automation interruption based on unobserved hidden truth

Defer:

- final implementation language
- external action DSL syntax
- exact event storage backend
- exact snapshot format
- complete effect vocabulary
- combat as privileged resolver versus effect program
- full formal verification scope

## Sources

Primary or official sources:

- [SimPy overview](https://simpy.readthedocs.io/en/latest/)
- [SimPy events](https://simpy.readthedocs.io/en/latest/topical_guides/events.html)
- [SimPy process interaction](https://simpy.readthedocs.io/en/latest/topical_guides/process_interaction.html)
- [SimPy shared resources](https://simpy.readthedocs.io/en/latest/topical_guides/resources.html)
- [SimPy time and scheduling](https://simpy.readthedocs.io/en/latest/topical_guides/time_and_scheduling.html)
- [ns-3 events and simulator](https://www.nsnam.org/docs/manual/html/events.html)
- [Microsoft Event Sourcing pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
- [Akka event sourcing](https://doc.akka.io/libraries/akka-core/current/typed/persistence.html)
- [Martin Fowler: Event Sourcing](https://www.martinfowler.com/eaaDev/EventSourcing.html)
- [Gaffer On Games: Deterministic Lockstep](https://gafferongames.com/post/deterministic_lockstep/)
- [GGPO rollback networking](https://www.ggpo.net/)
- [Unity Netcode prediction and rollback][unity-netcode-rollback]
- [Koka language book](https://koka-lang.github.io/koka/doc/book.html)
- [Unison abilities and ability handlers][unison-abilities]
- [Eff language](https://www.eff-lang.org/)
- [Handlers of Algebraic Effects](https://www.research.ed.ac.uk/en/publications/handlers-of-algebraic-effects)
- [The Verse Calculus](https://simon.peytonjones.org/verse-calculus/)
- [K Framework](https://kframework.org/exports/K.html)
- [Specifying Systems / TLA+](https://lamport.org/pubs/lamport-spec-tla-plus.pdf)
- [PDDL domain reference](https://planning.wiki/ref/pddl/domain)
- [CDDA activities](https://docs.cataclysmdda.org/PLAYER_ACTIVITY.html)
- [CDDA Effect On Condition](https://docs.cataclysmdda.org/JSON/EFFECT_ON_CONDITION.html)
- [CDDA event source](https://raw.githubusercontent.com/CleverRaven/Cataclysm-DDA/master/src/event.h)
- [CDDA item docs](https://docs.cataclysmdda.org/JSON/ITEM.html)
- [Caves of Qud events](https://wiki.cavesofqud.com/wiki/Modding%3AEvents)
- [Unreal Gameplay Ability System][unreal-gas]
- [Unity Entity Command Buffers][unity-ecb]

Local anchors:

- [Action and Event Model](../design/action-event-model.md)
- [Typed Action Effects](../ideas/typed-action-effects.md)
- [Kernel Primitives](../ideas/kernel-primitives.md)
- [Actor-Owned Capability-Derived Actions](../ideas/capability-derived-actions.md)
- [Actor Intent And Activity](../ideas/actor-intent-and-activity.md)
- [Multi-Resolution Simulation](../design/multi-resolution-simulation.md)
- [Semantic Kernel And PL Boundary](../ideas/semantic-kernel-and-pl-boundary.md)
- [CDDA reference note](../references/cataclysm-dda.md)
- [RimWorld reference note](../references/rimworld.md)

[unison-abilities]: https://www.unison-lang.org/docs/language-reference/abilities-and-ability-handlers/
[unity-netcode-rollback]: https://docs.unity.cn/Packages/com.unity.netcode%401.0/manual/prediction.html
[unreal-gas]: https://dev.epicgames.com/documentation/unreal-engine/gameplay-ability-system-for-unreal-engine
[unity-ecb]: https://docs.unity.cn/Packages/com.unity.entities%401.0/manual/systems-entity-command-buffers.html
