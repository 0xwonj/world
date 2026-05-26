# Causal Runtime

## Status

Current design draft

## Source Research

- [Causal Runtime / Action-Effect-Event](../research/causal-runtime-action-effect-event.md)
- [World Representation / Query Model](../research/world-representation-query-model.md)
- [Time Model / Turn Scheduling](../research/time-model-and-turn-scheduling.md)

## Related Design Owners

- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
- [Typed Effect Primitives](typed-effect-primitives.md)
- [Physical Simulation Grammar](physical-simulation-grammar.md)
- [World Model](world-model.md)
- [Time Model](time-model.md)
- [Multi-Resolution Simulation](multi-resolution-simulation.md)

## Purpose

The causal runtime defines how hard world state may change.

It connects actor attempts, process ticks, passive simulation, reactions,
typed effect programs, committed `EventRecord`s, observations, and replay.

`Causal` means cause/effect, not casual. A `CausalTransaction` records the
source cause, checked execution, staged mutations, and committed `EventRecord`s
as one hard-simulation unit.

The core rule:

```text
No hard world mutation outside CausalTransaction.
```

[Simulation Transition Compiler](simulation-transition-compiler.md) treats the
causal runtime as the final transactional interpreter for checked effect IR.
Higher-level context, appraisal, intent, and process passes may lower into
runtime requests, but they do not bypass this commit boundary.

## Core Flow

```text
InputSource
  player command | NPC policy | AI-controlled actor policy | process wakeup
  passive process | reaction request | resolution transition work
    |
    v
ActionRequest or ProcessTick or ReactionRequest
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
  staged event_record_set
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
  + authoritative state update
  + runtime control update
  + derived-view invalidation
    |
    v
Observation projection
    |
    v
Epistemic persistence / semantic appraisal / pressure / future intent bias
```

## Narrow Waists

There are several important boundaries. They are not interchangeable.

```text
ActionRequest:
  actor-facing attempted-change interface

Typed Effect Program:
  authored rule/effect definition boundary

CausalTransaction:
  atomic hard-simulation cause/effect commit boundary

EventRecord:
  committed hard fact boundary

Observation:
  actor-relative knowledge boundary
```

The deepest mutation waist is `CausalTransaction`, not `ActionRequest`.
Actor actions, process ticks, passive physics, reaction requests, magic,
crafting, and resolution transition work all commit hard state through the
same transaction discipline.

The runtime is a reusable mechanism. It does not own combat, crafting, magic,
economy, or other named RPG systems as features. Game-system packs may define
the action and process vocabularies for those systems, but accepted hard
outcomes still execute as checked typed effect programs inside
`CausalTransaction` and append `EventRecord`s.

## Core Records

### ActionRequest

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
```

Meaning:

- what an actor or actor-like source attempts now
- not a fact
- can be rejected, blocked, attempted, failed, or committed

Rules:

- Actor-owned capability determines which action schemas the actor can attempt.
- Observed affordances and context determine how schemas bind to targets.
- Hard validation happens inside the causal runtime.

### ActionDef

```text
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
```

Meaning:

- typed definition of an action schema
- lowers a valid request into a typed effect program
- declares which `EventRecord`s must be emitted on meaningful paths

Rules:

- Avoid generic `Use(target, mode: string)` as the primary model.
- Avoid action-specific hidden resolver mutation.
- Action definitions should call typed kernel effects, not raw store mutation.
- Action definitions may be authored by game-system packs, but pack ownership
  does not grant mutation authority outside the runtime.

### Typed Effect Program

```text
EffectProgram {
  typed_ir
  allowed_effects
  required_event_records
  deterministic_control_flow
}
```

Meaning:

- checked sequence or graph of primitive effects
- interpretable for validation, dry-run, commit, replay, audit, and debugging

Rules:

- Prefer domain-specific effects such as `transfer_entity`, `apply_damage`, and
  `set_lock_state`.
- Avoid unchecked `SetField`.
- Effect permissions enforce layer boundaries.

### CausalTransaction

```text
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
```

Meaning:

- one staged and committed cause/effect unit
- the only boundary that can alter authoritative hard state

Rules:

- Effects build staged mutations first.
- Later effects inside the same transaction may see earlier staged changes.
- Outside the transaction, nothing changes until commit.
- If invariant checks fail, hard mutation does not commit.
- Some failure outcomes may still commit attempt/failure `EventRecord`s if
  they are valid gameplay facts.

### TransactionRecord

```text
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
```

Meaning:

- committed transaction envelope
- audit and replay anchor

### EventRecord

```text
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
```

Meaning:

- meaningful committed hard causal fact
- input to observation, memory, semantic interpretation, logs, and replay

Rules:

- `EventRecord`s are not UI text.
- `EventRecord`s do not declare social, legal, emotional, or narrative meaning.
- Semantic appraisal may reference `EventRecord`s, but its accepted outputs are
  semantic/appraisal records, not `EventRecord`s.
- Social or institutional changes are typed soft-truth records owned by the
  social model, not hard physical `EventRecord`s.
- Primitive mutation traces may exist separately for debug/audit.

## Effect Permissions

Initial hard-action permissions:

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
EmitPhysicalEventRecord
EmitSensoryEventRecord
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

Semantic appraisal permissions:

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
MutateMemoryDirectly
MutateBeliefDirectly
SelectIntentDirectly
EmitEventRecord
```

## Process Runtime

A long activity is not a large action. It is a serializable state machine that
emits or continues ordinary attempts over time.

```text
ProcessInstance {
  id
  kind
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
  source_event_record_or_action?
  version
}
```

Lifecycle:

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

Rules:

- Processes do not mutate world state directly.
- Processes handle `ProcessTick`s, compute `ProcessTransition`s, and may emit
  continuation work or ordinary `ActionRequest`s.
- Durable progress, wait, reservation, interruption, resume, completion, and
  failure changes become `RuntimeControlUpdate`s rather than direct
  `RuntimeControlStore` mutation.
- A process may tick at concrete, abstract, or strategic resolution when its
  process definition supports that resolution.
- Abstract process ticks must not synthesize hidden per-turn concrete
  `ActionRequest`s. They advance the same process over a coarser state
  surface.
- Progress, wait conditions, reservations, interruption, and resume state must
  be serializable.
- Passive physical processes use the same causal transaction path.

## Reservation

`Reservation` is temporary runtime conflict-control state.

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

Rules:

- A reservation is not ownership.
- A reservation is not a social claim.
- Reservations affect validation and replay, so they are hard runtime state.
- Reservations are acquired and released through causal transactions.

## Reaction Requests

`EventRecord` reactions do not mutate the original transaction directly.

```text
ReactionRequest {
  id
  source_event_record
  target_rule
  priority
  scheduled_time
  permissions
}
```

Preferred flow:

```text
Committed EventRecord
  -> reaction rule reads EventRecord and context
  -> enqueues ReactionRequest or ProcessTick
  -> later CausalTransaction validates and commits effects
```

This prevents hidden listener mutation, order-dependent cascades, and semantic
rules bypassing physical action boundaries.

## Failure Semantics

Use typed failure categories:

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

Rules:

- Not every rejection is a public `EventRecord`.
- Meaningful attempts should leave structured evidence.
- Debug/provenance should record more than actor-facing output.

## Replay And Save Contract

Replay has tiers. All committed hard outcomes need enough provenance for audit
and save/load continuity. Only selected subsystems, tests, or debug modes need
deterministic command replay.

`ReplayLevel` values:

```text
AuditOnly:
  inspect committed records, ordering, validation context, and provenance

EventRebuild:
  rebuild committed consequences from transaction and event history

DeterministicCommandReplay:
  rerun accepted input logs and expect matching transactions and events
```

Hard replay records may include:

- engine version
- content/schema version
- genesis state or snapshot hash
- EventHistoryStore cursor
- stable entity ids
- explicit scheduler ordering
- RNG draw provenance where needed
- deterministic RNG streams for command replay paths
- action/process input log
- committed transaction records and `EventRecord`s
- optional read/write sets or dependency summaries
- state hash checkpoints

RNG model:

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

Replay level usage:

```text
DeterministicCommandReplay:
  genesis + seed + content version + action/process inputs
  -> recompute transactions and event_record_set

EventRebuild:
  snapshot or genesis + committed event_record_set
  -> rebuild state and projections without rerunning resolution logic

Debug command replay:
  DeterministicCommandReplay + EventRecord comparison + state hash checkpoints
```

## Relationship To Time Model

The causal runtime does not define the whole time policy. It requires only:

```text
CausalTransaction.sim_time
CausalTransaction.phase
CausalTransaction.sequence
ScheduledWakeup records
explicit scheduler ordering
```

Detailed timing is defined in [Time Model](time-model.md).

## Relationship To Multi-Resolution Simulation

[Multi-Resolution Simulation](multi-resolution-simulation.md) decides which
resolution is active for an actor, group, process, place, or thread. The causal
runtime does not decide resolution policy.

The causal runtime enforces the hard mutation contract:

```text
Concrete ActionRequest
  -> CausalTransaction
  -> EventRecord

Concrete ProcessTick
  -> CausalTransaction
  -> EventRecord

Abstract ProcessTick
  -> CausalTransaction
  -> EventRecord

Strategic ProcessTick, when hard-gameplay authoritative
  -> CausalTransaction
  -> EventRecord
```

Promotion and demotion are not special mutation backdoors. If they create,
remove, refine, coarsen, or relocate hard state, they commit through
`CausalTransaction`.

AI-proposed materialization details are also not hard truth until checked and
committed through the proper authority surface.

## Relationship To World Model

The causal runtime writes through the
[World Model](world-model.md)'s `CausalTransactionGate`.

It may write:

- `WorldStore`
- hard-truth `RelationStore` families
- `RuntimeControlStore`
- `EventHistoryStore`

It must not write:

- `SocialInstitutionalStore`
- soft-truth `RelationStore` families
- `ChronologyStore`
- `EpistemicStore`
- `AppraisalRecordStore`

Hard physical action effects must not smuggle social meaning, memory, belief,
or appraisal state through generic relation writes.

Committed `EventRecord`s may become provenance for later non-hard commit
surfaces:

```text
EventRecord
  -> AcceptedSocialUpdate
  -> AcceptedEpistemicUpdate
  -> AcceptedAppraisalRecord
  -> AcceptedChronologyRecord
```

Those later commits are owned by their respective truth-layer gates, not by the
causal runtime.

It may invalidate:

- `DerivedViewRegistry` views
- actor projections
- debug/provenance views

It must not directly write semantic meaning into hard physical action effects.

## Relationship To Typed Effect Primitives

The causal runtime executes `Typed Effect Program`s, stages their writes, checks
transaction invariants, commits successful transactions, and appends
`EventRecord`s.

[Typed Effect Primitives](typed-effect-primitives.md) defines the primitive
effect vocabulary and `EventRecord` contract expectations. The causal runtime
defines execution, failure, replay, process, reservation, and reaction
semantics.

## Current Open Questions

- Which primitive effects from [Typed Effect Primitives](typed-effect-primitives.md)
  are mandatory in the minimal design surface?
- Which `EventRecord` contracts must the runtime enforce versus lint?
- Which failed validations produce public `EventRecord`s?
- What is the minimum useful invariant set for transaction commit?
- How much read/write provenance is retained in normal saves?
- Which reactions may be same-time next-phase, and which must be delayed?
