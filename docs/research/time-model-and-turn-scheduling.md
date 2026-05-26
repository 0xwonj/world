# Time Model / Turn Scheduling

## Status

Draft research

## Parent Axis

[Causal Runtime / Action-Effect-Event](causal-runtime-action-effect-event.md)

## Design Output

- [Time Model](../design/time-model.md)

## Core Question

```text
What does "turn" mean in `world`, and how should actor actions, long
processes, passive simulation, and player input advance time?
```

## Why This Is Separate

The causal runtime document establishes that every hard mutation flows through
typed effects and `CausalTransaction`. It intentionally does not settle the
time policy.

Time policy is worth isolating because it affects:

- player feel
- speed and action-cost balance
- NPC behavior frequency
- passive process frequency
- long activity interruption
- replay determinism
- multi-resolution promotion and demotion
- AI-agent turn interface

This document should remain about time and scheduling. It should not pull in
the full conflict, effect, event, or reservation policy.

## Terminology

Use these terms distinctly:

```text
Simulation time:
  Monotonic integer or fixed-point engine time.

Activation:
  The moment an actor or process is allowed to do work.

Player turn:
  A UI/input opportunity when the protagonist is ready and no mandatory
  resolution is pending.

Action duration:
  How much simulation time an action consumes.

Process wakeup:
  A scheduled time when a process advances or checks conditions.

Round:
  Optional presentation/balance unit, usually "about one normal action".
  It should not be the primitive engine clock unless deliberately chosen.
```

The word `turn` should be treated as player-facing language. Internally, use
simulation time, activation, duration, and scheduled wakeups.

## Theory Baseline

### Discrete-Event Time

Discrete-event simulation treats time as an ordered event agenda:

```text
ScheduledItem(time, tie_breaker, payload)
```

The engine repeatedly takes the earliest item, sets simulation time to that
time, executes it, and schedules future work.

Transfer:

- Simulation time is not wall-clock time.
- Work should be scheduled at explicit future times.
- Same-time events need deterministic ordering.
- Long work should schedule a future completion/wakeup, not block.

Adapt:

- Use typed actor/process wakeups instead of opaque callbacks.
- Use integer or fixed-point time for deterministic replay.

Reject:

- floating point time as core hard-simulation time
- wall-clock time in hard simulation
- ordering tied to hash map iteration or thread scheduling

### Energy / Action-Point Time

Many roguelikes model speed by giving actors action points over time. Actions
spend points; faster actors gain points faster or spend fewer points.

Useful vocabulary:

```text
speed / quickness:
  how quickly an actor becomes ready

action cost:
  how much readiness or time an action consumes

carryover:
  unspent readiness that lets fast actors act more often
```

Transfer:

- Speed and action cost should be distinct.
- Movement speed can be a specialization of action cost, not global speed.
- Some actions may be free, cheap, normal, slow, or long-running.

Adapt:

- Use action points as a design/balance vocabulary if useful.
- Implement internally as next-ready simulation time if that is cleaner.

Reject:

- hidden random speed jitter by default
- free actions that can loop indefinitely
- one global "turn" where every actor always acts exactly once

### Player-Facing Turn Shell

The game is a single-protagonist RPG. The player should experience it as
turn-based:

```text
player chooses an action
world resolves consequences
player receives the next input opportunity
```

Internally, many actor activations and process wakeups may happen before the
player is ready again. That should usually be invisible except through
observations and event messages.

Transfer:

- The protagonist's readiness defines when the UI asks for input.
- Other actors and processes run until the next player input boundary.
- Automation such as rest, travel, reading, or searching can continue across
  many internal activations until an interrupt condition occurs.

Reject:

- exposing every non-player activation as a "turn" to the player
- forcing all simulation to wait for player input when the protagonist is not
  ready

## Reference Pressure

### SimPy

SimPy is useful for deterministic queue semantics. Its time guide emphasizes
that even same-time events are processed sequentially and deterministically,
with an event id used to break ties.

Transfer:

- Use a stable agenda key.
- Same-time events are not truly simultaneous unless the engine explicitly
  models them as a batch.
- Determinism needs a tie-breaker.

### ns-3

ns-3 is a discrete-event simulator. Its event model stores future events in a
scheduler, runs them in increasing simulation time, and uses FIFO ordering for
same-time events. It also notes that event execution is modeled as zero-time;
if work takes time, schedule another event for completion.

Transfer:

- Separate "start work" from "finish work" for delayed actions.
- Use explicit cancel/remove semantics for scheduled wakeups.
- Treat scheduler performance as a later implementation concern.

Adapt:

- `ScheduleNow`-style behavior can become "same-time next phase" work.
- Event context should include actor/process/source for debugging.

Reject:

- opaque callback events as the gameplay model

### CDDA

CDDA activity docs distinguish long-term activities from one-turn actions.
Activities can be interrupted and resumed, and progress can be based on time,
actor speed, or custom per-turn logic.

Transfer:

- Long activities need explicit time basis:
  - fixed elapsed time
  - actor-speed-adjusted work
  - custom process tick
- Activities need saveable state.
- Infinite progress loops must be guarded against.

Adapt:

- Use `ProcessInstance` rather than activity subclasses.
- Use typed effect programs for process ticks.

### Caves Of Qud

Qud is a useful RPG reference for action cost. Its public mechanics distinguish
quickness, action points, action cost, movement speed, free actions, and
action-cost modifiers.

Transfer:

- Most actions can share a default cost.
- Quickness/global speed and movement-specific speed are different concepts.
- Equipment, skills, mutation, environment, and status can modify action cost.
- Cheap actions and free actions are useful but need loop guards.

Adapt:

- Random action-cost variation should be optional or avoided initially because
  it complicates explainability and replay.
- Use "action cost" as player-facing language, even if the internal scheduler
  uses next-ready times.

### Dungeon Crawl Stone Soup

DCSS exposes elapsed time and previous action duration, and actions may be
slower or quicker depending on action type, equipment, status, or skill. Its
run/rest options show that delayed actions need explicit interrupt categories:
monster seen, damage, keypress, message, teleport, and other causes.

Transfer:

- Show or explain previous action duration when useful.
- Rest/travel/search automation should stop on actor-relative interrupt
  signals.
- Interrupt categories should be inspectable and configurable later.

Adapt:

- Interrupts should come from observation/projection, not hidden omniscient
  truth.

## Candidate Models

### Candidate A: Alternating Player/World Turns

Sketch:

```text
player acts
all other actors act
passive processes tick
repeat
```

Makes easy:

- simple player mental model
- easy first implementation
- predictable pacing

Makes hard:

- speed differences
- quick/slow actions
- process timing
- passive simulation
- AI-agent explanation for "how much time passed"
- multi-resolution time sync

Assessment:

Reject as core model. It is too coarse for the intended simulation depth.

### Candidate B: Round-Based Initiative

Sketch:

```text
round starts
actors act in initiative order
round ends
```

Makes easy:

- tabletop-like combat
- clear grouping
- easy visible initiative UI

Makes hard:

- non-combat simulation
- unequal action duration
- passive processes and long activities
- very fast or very slow actors
- "world-like" continuous time pressure

Assessment:

Useful as a combat presentation mode, not the engine's base time model.

### Candidate C: Action Point Pool Per Round

Sketch:

```text
each round:
  actor gains speed-based points
  actor spends points on one or more actions
  leftover may carry over
```

Makes easy:

- speed/action cost math
- fast actors taking extra actions
- slow/cheap/expensive actions
- player-facing cost language

Makes hard:

- defining a global round
- process wakeups not tied to actors
- same-time ordering
- avoiding UI feeling like an AP tactics game

Assessment:

Strong as a balance vocabulary. Less ideal as the internal scheduler if the
world also needs many independent process wakeups.

### Candidate D: Energy Accumulation Queue

Sketch:

```text
actors accumulate readiness
when readiness >= action cost, actor activates
action spends readiness
leftover readiness carries over
```

Makes easy:

- organic speed differences
- extra actions over time
- action-cost modifiers
- Qud-like behavior

Makes hard:

- explaining exact timing
- handling non-actor processes
- same-readiness ties
- preventing carryover exploits

Assessment:

Good model, especially for actor readiness. It can be represented as an
equivalent next-ready-time scheduler.

### Candidate E: Time-Cost Priority Scheduler

Sketch:

```text
ScheduledWakeup(time, phase, priority, sequence, target)

when actor acts:
  resolve action now
  next_ready_time = current_time + action_duration(actor, action, context)
  schedule actor at next_ready_time

when process ticks:
  resolve process tick
  schedule next wakeup
```

Makes easy:

- deterministic discrete-event scheduling
- independent actor and process wakeups
- passive processes
- long activity wakeups
- player turn as protagonist activation
- multi-resolution sync
- replay/debug with explicit times

Makes hard:

- exact duration math needs care
- fast actors may feel like "extra turns"
- simultaneous events need phase/tie policy
- action duration has to be explainable

Assessment:

Best core model for `world`.

### Candidate F: Simultaneous Declaration / WEGO

Sketch:

```text
all relevant actors declare
engine resolves batch simultaneously
next turn
```

Makes easy:

- tactical prediction
- simultaneous conflict drama
- less first-mover advantage

Makes hard:

- single-protagonist RPG flow
- AI-agent input
- long processes
- passive simulation
- debugging and player comprehension

Assessment:

Reject as core model. Could be used for special subsystems, not the base
runtime.

## Recommended Model

Use a deterministic time-cost priority scheduler with a player-facing
turn-based shell.

Core:

```text
Time:
  integer simulation ticks

BaseActionDuration:
  1000 ticks

ScheduledWakeup:
  time
  phase
  priority
  sequence
  target: actor | process | reaction | abstract thread

PlayerTurn:
  occurs when protagonist wakeup is next and the engine needs input
```

Conceptual flow:

```text
while game_running:
  item = scheduler.pop_next()
  sim_time = item.time

  if item is protagonist activation and no automation is active:
    ask player for input
  else:
    produce ActionRequest / ProcessTick / ReactionRequest

  resolve through CausalTransaction
  schedule future wakeups
  project observations

  if protagonist is ready and an interrupt requires input:
    stop automation and ask player
```

Player-facing feel:

```text
The game is turn-based.
The player acts when the protagonist is ready.
Fast enemies, fire, bleeding, rituals, and travel advance according to
simulation time between player input opportunities.
```

## Time Units

Recommended initial scale:

```text
1 normal action = 1000 ticks
1 segment = 100 ticks
1 light/free-adjacent action = 0-100 ticks
1 slow action = 1500-3000 ticks
long activities = repeated process wakeups or delayed completion
```

Why 1000:

- matches common action-cost vocabulary
- supports tenths/segments cleanly
- allows speed modifiers without floats
- maps well to Qud-like action-cost thinking

Rules:

- Use integers only.
- Do not expose raw ticks everywhere in UI.
- Store both `sim_time` and `duration` in transaction records and
  `EventRecord`s where
  relevant.
- Use fixed named constants for common costs.

## Duration Model

Separate:

```text
base_action_duration:
  default duration of the action schema

actor_speed:
  global readiness modifier from body/status/magic/etc.

action_cost_modifiers:
  tool, skill, environment, condition, mode

movement_cost:
  movement-specific duration component

process_work_rate:
  rate at which actor/process advances long work
```

Candidate formula:

```text
duration = base_duration
duration = apply_action_cost_modifiers(duration, action, context)
duration = apply_actor_speed(duration, actor)
duration = clamp_and_round(duration)
```

Alternative actor-readiness formula:

```text
ready_at += cost * BASE_SPEED / effective_speed
```

Avoid for now:

- random action-cost jitter by default
- hidden fractional leftovers
- arbitrary per-feature time formulas

## Action Timing Shapes

Not all actions should resolve at the same timing point.

### Instant Effect, After-Delay

Most roguelike actions:

```text
at current_time:
  move, attack, pick up item, open door
then:
  actor is unavailable until current_time + duration
```

Use for:

- movement
- ordinary attacks
- item pickup/drop
- short manipulation
- speech acts
- inspection

### Windup, Then Effect

Delayed committed effect:

```text
current_time:
  ActionWindupStarted
  schedule completion

completion_time:
  validate still possible
  commit effect or interruption/failure
```

Use for:

- heavy attack
- charged spell
- long door forcing
- precise shot
- interruptible ritual step

### Continuous Process

Repeated progress:

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

Use sparingly:

```text
duration = 0
must not allow unbounded loops
may be limited by phase, count, cooldown, or UI-only status
```

Use for:

- look/examine
- inventory inspection
- maybe some trained instant abilities
- UI-only decisions

## Scheduler Key

Recommended:

```text
(time, phase, priority, sequence)
```

Where:

```text
time:
  simulation tick

phase:
  broad ordering bucket

priority:
  local ordering within the phase

sequence:
  monotonic id assigned at scheduling time
```

Initial phase candidates:

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

This phase list is not final. The important decision is that same-time ordering
is explicit and deterministic.

## Player Turn And Automation

The player should not need to think in scheduler events.

Player input happens when:

```text
protagonist is ready
and no selected player process should continue automatically
and no mandatory prompt/interrupt is pending
```

Player automation can be represented as a `ProcessInstance`:

```text
RestUntilRecovered
TravelToKnownSite
SearchRoom
ReadUntilInterrupted
FollowTrail
TreatWound
```

Automation stops when an actor-relative interrupt is produced:

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

Hidden hard truth should not stop automation unless it creates an observable or
physically forced signal.

## Passive Processes

Passive simulation should not depend on "every actor turn".

Instead:

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

- Processes schedule their next wakeup explicitly.
- Dormant processes should not tick.
- Nearby/local processes can use fine intervals.
- Abstract/distant processes can use coarser intervals.

## Multi-Resolution Time

Recommended layering:

```text
Local:
  exact integer simulation time
  actor activations and process wakeups

Nearby / abstract:
  same time axis, coarser wakeups
  abstract processes with progress and risk

Distant / strategic:
  coarse calendar or strategic ticks
  region/faction pressure updates
```

Promotion rule:

```text
abstract thread materializes at a concrete sim_time
with provenance from abstract progress events
```

Demotion rule:

```text
concrete actors/processes summarize into abstract process state
with last concrete sim_time and durable consequences
```

This keeps one time axis while allowing different resolution.

## AI-Agent Interface Implications

For an AI-agent-native RPG, the agent should receive:

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

Do not expose omniscient scheduler truth by default. Enemy readiness should be
observed approximately unless the actor has a reason to know exact timing.

Useful public language:

```text
quick
normal
slow
will finish soon
still several moments away
enemy is recovering
enemy is winding up
```

Debug tools can show exact ticks.

## Design Decisions

This time-model axis should eventually settle:

- What is the base tick scale?
- Is `1000 ticks = one normal action` the canonical balance unit?
- Is action cost public-facing while duration is engine-facing?
- Which actor stats modify global readiness versus specific action costs?
- How are minimum and maximum action durations clamped?
- Are random action-cost jitters ever allowed?
- Which actions are truly zero-time?
- What is the exact scheduler phase list?
- Can actor activations and process wakeups share the same phase?
- How does player automation decide whether to continue?
- Which passive processes need fine local intervals first?
- How does abstract time synchronize with local concrete time?
- How much exact timing is exposed to AI agents and human UI?

## Recommended Defaults

Initial default stance:

```text
Internal model:
  deterministic integer discrete-event scheduler

Player feel:
  turn-based single-protagonist RPG

Base unit:
  1000 ticks = one normal action

Actor scheduling:
  next_ready_time += action_duration

Process scheduling:
  explicit ScheduledWakeup records

Same-time ordering:
  (time, phase, priority, sequence)

Speed:
  modifies action duration or readiness, using integer math

Movement speed:
  action-cost modifier for movement, not universal speed

Free actions:
  allowed only with loop guards

Random jitter:
  off by default; only explicit deterministic effect if later desired

Replay:
  record sim_time, duration, scheduler sequence, and RNG draws
```

## Test Scenarios

### Fast Wolf, Slow Guard, Player

Initial:

- player normal action duration 1000
- wolf move duration 750
- armored guard attack duration 1400

Expected:

- player still experiences turns
- wolf acts more often over time
- guard acts less often
- exact order is deterministic

Reveals:

- speed and action cost need clear arithmetic

### Reading Until Danger

Initial:

- player starts reading process
- reading wakes every 1000 ticks
- monster enters visible range at tick 2500

Expected:

- process progresses twice
- observation interrupt stops automation
- hidden monster outside observation does not stop reading

Reveals:

- player turn shell and process wakeups must cooperate

### Charged Heavy Attack

Initial:

- player starts heavy attack with 500 tick windup and 1000 tick recovery
- enemy moves before completion

Expected:

- windup event is visible
- completion revalidates target/context
- miss, retarget, cancel, or partial effect is explicit

Reveals:

- instant actions and windup actions need different timing shapes

### Fire And Bleeding

Initial:

- actor is bleeding every 500 ticks
- nearby cloth fire spreads every 250 ticks
- player action duration is 1000

Expected:

- several passive wakeups may happen between player input opportunities
- events remain ordered by sim_time
- player receives observations, not raw internal ticks

Reveals:

- passive processes cannot be tied to player turns

### Abstract Caravan

Initial:

- caravan is far away
- abstract travel process wakes every 6 hours
- player approaches region

Expected:

- abstract progress maps onto the same time axis
- promotion creates local state with provenance

Reveals:

- multi-resolution simulation needs shared time coordinates

## Takeaways For `world`

Keep:

- turn-based player feel
- deterministic integer simulation time
- action duration/cost per action
- priority scheduler for actors and processes
- explicit scheduled wakeups
- player input as protagonist activation boundary
- process automation with actor-relative interrupts

Adapt:

- SimPy/ns-3 agenda ordering and tie-breakers
- Qud-style action cost vocabulary
- CDDA-style time/speed/custom process basis
- DCSS-style previous action duration and interruption categories

Reject:

- player/world alternating turns as core engine time
- strict global rounds as core engine time
- floating-point hard-simulation time
- random action-cost jitter by default
- hidden global ticks that update everything every player turn
- omniscient automation interrupts

Defer:

- exact base constants beyond the recommended default
- final phase list
- exact UI timing display
- detailed action-cost formulas
- detailed process interval table

## Sources

Primary or official sources:

- [SimPy time and scheduling](https://simpy.readthedocs.io/en/latest/topical_guides/time_and_scheduling.html)
- [SimPy events](https://simpy.readthedocs.io/en/latest/topical_guides/events.html)
- [ns-3 events and simulator](https://www.nsnam.org/docs/manual/html/events.html)
- [CDDA activities](https://docs.cataclysmdda.org/PLAYER_ACTIVITY.html)
- [Caves of Qud action cost](https://wiki.cavesofqud.com/wiki/Action_cost)
- [Caves of Qud quickness](https://wiki.cavesofqud.com/wiki/Quickness)
- [Caves of Qud movement speed](https://wiki.cavesofqud.com/wiki/Movement_Speed)
- [Dungeon Crawl Stone Soup manual][dcss-manual]
- [Dungeon Crawl Stone Soup options guide][dcss-options]

Local anchors:

- [Causal Runtime / Action-Effect-Event](causal-runtime-action-effect-event.md)
- [Kernel Primitives](../ideas/kernel-primitives.md)
- [Actor Intent And Activity](../ideas/actor-intent-and-activity.md)
- [Multi-Resolution Simulation](../design/multi-resolution-simulation.md)
- [CDDA reference note](../references/cataclysm-dda.md)

[dcss-manual]: https://raw.githubusercontent.com/crawl/crawl/master/crawl-ref/docs/crawl_manual.rst
[dcss-options]: https://raw.githubusercontent.com/crawl/crawl/master/crawl-ref/docs/options_guide.txt
