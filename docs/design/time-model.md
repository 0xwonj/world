# Time Model

## Status

Current design draft

## Source Research

- [Time Model / Turn Scheduling](../research/time-model-and-turn-scheduling.md)
- [Causal Runtime / Action-Effect-Event](../research/causal-runtime-action-effect-event.md)

## Related Design Owners

- [Simulation Transition Compiler](simulation-transition-compiler.md)
- [Causal Runtime](causal-runtime.md)
- [Typed Effect Primitives](typed-effect-primitives.md)
- [Physical Simulation Grammar](physical-simulation-grammar.md)
- [Multi-Resolution Simulation](multi-resolution-simulation.md)

## Purpose

The time model defines how actor actions, process wakeups, passive simulation,
and player input advance simulation time.

The player-facing experience should remain turn-based. Internally, the engine
uses integer simulation time and an explicit priority scheduler.

## Core Principle

```text
Player feel:
  turn-based single-protagonist RPG

Internal model:
  integer discrete-event scheduler with explicit ordering
```

The word `turn` is player-facing language. Internally, use:

```text
simulation time
activation
action duration
process wakeup
scheduled wakeup
```

## Time Units

Use integer ticks.

Initial balance scale:

```text
1000 ticks = one normal action
100 ticks = one segment
0-100 ticks = free or near-free action
1500-3000 ticks = slow action
```

Rules:

- Do not use floating point for hard simulation time.
- Do not use wall-clock time in hard simulation.
- Raw ticks are engine/debug data, not the default player-facing display.
- Store `sim_time` and relevant `duration` in committed transaction records and
  `EventRecord`s.

## Scheduler

Use a priority scheduler.

```text
ScheduledWakeup {
  time
  phase
  priority
  sequence
  target: actor | process | reaction
}
```

Canonical ordering key:

```text
(time, phase, priority, sequence)
```

Rules:

- `time` is integer simulation time.
- `phase` is a broad same-time ordering bucket.
- `priority` orders work within a phase.
- `sequence` is monotonic and assigned when scheduled.
- Ordering must not depend on hash map iteration, thread timing, or memory
  layout.
- Same-time ordering must be visible in debug tools.

Initial phases:

```text
0. transaction cleanup / due finalizers
1. actor activation
2. process wakeup
3. passive physical process
4. reaction request
5. observation projection
6. semantic interpretation
7. snapshot / debug checkpoint
```

The exact phase list may change. The requirement is explicit ordering that can
be inspected and replayed at the declared replay level.

## Actor Scheduling

Actors become ready at scheduled times.

```text
when actor acts:
  duration = action_duration(actor, action, context)
  actor.next_ready_time = current_time + duration
  schedule actor activation at next_ready_time
```

The protagonist receives input when their activation is due and no selected
automation should continue.

Fast actors act more often because their actions produce shorter durations or
their readiness advances faster. Slow actors act less often.

## Action Duration

Each action schema has a base duration.

Examples:

```text
Move one tile:
  base_duration = 1000

Open door:
  base_duration = 800

Quick dagger stab:
  base_duration = 700

Longsword attack:
  base_duration = 1200

Heavy ritual step:
  base_duration = 1500
```

Duration calculation separates global speed from action-specific modifiers.

```text
duration = base_duration
duration = apply_action_cost_modifiers(duration, action, context)
duration = apply_actor_speed(duration, actor)
duration = clamp_and_round(duration)
```

Examples:

```text
Quickness or haste:
  affects broad readiness or most action durations

Movement speed:
  affects movement duration

Lockpicking skill:
  affects lockpicking process duration, progress, or risk

Wounded hand:
  affects manipulation duration and possibly success/risk

Heavy armor:
  affects movement and some physical actions
```

Rules:

- Duration and success are separate concepts.
- A bad condition may affect duration, success chance, risk, or all three.
- Avoid random duration jitter by default.
- Clamp minimum duration so meaningful actions cannot create infinite loops.

## Action Timing Shapes

### Instant Effect, After-Delay

Most short actions resolve immediately, then the actor waits.

```text
current_time:
  commit movement, attack, pickup, drop, open, inspect, or speech

after:
  actor ready again at current_time + duration
```

Use for:

- movement
- ordinary attacks
- item pickup/drop
- short manipulation
- speech acts
- inspection

### Windup, Then Effect

Some actions start now and commit later.

```text
current_time:
  ActionWindupStarted
  schedule completion

completion_time:
  revalidate target/context
  commit effect, failure, or interruption
```

Use for:

- heavy attacks
- charged spells
- precise shots
- force-opening a door
- interruptible ritual steps

### Continuous Process

Long activities use process wakeups.

```text
ProcessStarted
ProcessTick at each scheduled interval
ProcessCompleted or interrupted
```

Use for:

- reading
- crafting
- treating wounds
- searching
- travel/rest automation
- fire spread
- healing

### Free / Zero-Time Action

Use sparingly.

```text
duration = 0
```

Rules:

- Zero-time actions must not allow unbounded loops.
- They may need per-phase count limits, cooldowns, or UI-only classification.
- Debug and UI actions can be zero-time because they are not hard world
  mutation.

## Player Turn Shell

Player input happens when:

```text
protagonist is ready
and no selected player process should continue automatically
and no mandatory prompt or interrupt is pending
```

The player should not need to think in scheduler events. They experience:

```text
choose action
world resolves consequences
receive next input opportunity
```

Between player input opportunities, the scheduler may resolve:

- fast enemy activations
- passive physical processes
- process ticks
- reaction requests
- observation and semantic updates

## Player Automation

Player automation is represented as `ProcessInstance`.

Examples:

```text
RestUntilRecovered
TravelToKnownSite
SearchRoom
ReadUntilInterrupted
FollowTrail
TreatWound
```

Automation stops on actor-relative interrupts:

```text
ObservedDanger
ActorDamaged
ToolLost
TargetMoved
ReservationLost
NoiseHeard
PlayerInput
ProcessCompleted
```

Rules:

- Hidden hard truth should not stop automation unless it creates an observable
  or physically forced signal.
- Interrupts should preserve enough state for the player to continue, cancel,
  or switch activity when appropriate.

## Passive Processes

Passive simulation is scheduled independently from player turns.

Examples:

```text
FireProcess:
  next_wakeup = sim_time + 250

BleedingProcess:
  next_wakeup = sim_time + 500

PoisonProcess:
  next_wakeup = sim_time + 1000

ScentDiffusion:
  next_wakeup = sim_time + 2000 or coarser
```

Rules:

- Processes schedule their own next wakeup.
- Dormant processes should not tick.
- Local processes can use fine intervals.
- Abstract or distant processes can use coarser intervals.

## Multi-Resolution Time Constraints

Multi-resolution promotion, demotion, and abstract or strategic process
scheduling are owned by
[Multi-Resolution Simulation](multi-resolution-simulation.md). The time model
owns the shared clock and ordering constraints.

Use one shared time axis with different wakeup granularity.

```text
Concrete:
  exact integer simulation time
  actor activations and process wakeups

Abstract:
  same time axis, coarser wakeups
  process wakeups with progress, route position, risk, and traces

Strategic:
  coarse calendar or strategic ticks
  region/faction pressure updates
```

Promotion:

```text
coarse state refines at a specific sim_time
with provenance from abstract process state and `EventRecord`s
```

Demotion:

```text
concrete actors/processes change active resolution at a specific sim_time
while preserving durable facts, process identity, and provenance
```

Rules:

- Abstract and strategic processes must still schedule explicit wakeups.
- Coarser wakeups reduce detail; they do not escape simulation time.
- Promotion must not create a temporal discontinuity.
- Demotion must record enough timing provenance for replay and explanation.

## AI-Agent Interface

Actor-facing AI agents should receive timing information through their
projection, not omniscient scheduler truth.

Agent timing context may include:

```text
current_sim_time
protagonist_ready_state
last_action_duration
observed_recent_events
visible_actor_readiness_estimates
active_processes
interrupts
available_action_schemas with estimated duration/cost
```

Useful player/agent-facing language:

```text
quick
normal
slow
will finish soon
still several moments away
enemy is recovering
enemy is winding up
```

Debug tools may show exact ticks and scheduler order.

## Relationship To Causal Runtime

Time scheduling does not mutate hard state directly.

The scheduler wakes a target:

```text
ScheduledWakeup
  -> ActionRequest / ProcessTick / ReactionRequest
  -> CausalTransaction
  -> EventHistoryStore append
  -> schedule future wakeups
```

The causal runtime records:

- `sim_time`
- phase
- sequence
- action duration where relevant
- scheduled future work

## Current Open Questions

- Are `1000 ticks = one normal action` and `100 ticks = one segment` stable
  enough for the baseline design?
- Which actions are truly zero-time?
- What is the first concrete phase list?
- Which actor stats modify global speed versus specific action duration?
- How much exact duration should the normal UI expose?
- Which passive processes need fine intervals in the first implementation?
