# Multi-Resolution Simulation

## Status

Current design draft.

## Source Ideas

- [Multi-Resolution Simulation](../ideas/multi-resolution-simulation.md)
- [Actor Intent And Activity](../ideas/actor-intent-and-activity.md)
- [Causal Runtime / Action-Effect-Event](../research/causal-runtime-action-effect-event.md)
- [Time Model / Turn Scheduling](../research/time-model-and-turn-scheduling.md)

## Related Design Owners

- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
- [World Model](world-model.md)
- [Causal Runtime](causal-runtime.md)
- [Time Model](time-model.md)
- [Intent Templates And Planning](intent-templates-and-planning.md)
- [Perception And Observation](perception-and-observation.md)
- [Epistemic State](epistemic-state.md)

## Purpose

Multi-resolution simulation defines how the same world can progress at
different levels of detail without changing truth authority.

The local game should remain a deep single-protagonist RPG with concrete
actions, perception, affordances, combat, dialogue, and physical interaction.
The wider world should still move, but it should not run invisible concrete
turns for every offscreen actor.

This document owns:

- resolution tiers
- how intent lowers at each tier
- how the existing process system behaves across tiers
- promotion from coarse state to concrete state
- demotion from concrete state to coarser state
- resolution-aware location and route progress
- observation boundaries for abstract state
- authority constraints for hard and soft state across resolution changes

Multi-resolution execution is a reusable mechanism. Concrete `ProcessDef`
families for travel, recovery, crafting, construction, faction projects,
rituals, patrols, trade routes, and strategic conflicts may be game-system pack
definitions, but they advance through the shared process, transaction, event,
and truth-authority boundaries.

The [Simulation Transition Compiler](simulation-transition-compiler.md) frames
multi-resolution as target-specific lowering plus abstract/refined execution
contracts. Concrete and abstract execution may use different state surfaces,
but both must preserve authority, provenance, and durable consequences.

## Core Principle

Resolution is an execution and detail policy. It is not truth authority.

```text
resolution:
  how much detail is represented and which execution policy is active

authority:
  which truth layer owns a record and which commit surface may write it
```

A low-resolution fact can still be hard truth. A high-resolution story detail
can still be soft truth.

Examples:

```text
Hard abstract truth:
  caravan_1 is on north_road_segment_3 at route_progress 0.53.

Not yet hard truth:
  caravan_1 wagon is exactly at tile (42, 17).

Soft chronology:
  old_tower_1 was betrayed by its captain generations ago.
```

Any hard fact created or changed at any resolution must commit through
`CausalTransaction` and produce `EventRecord` evidence.

## Resolution Tiers

### Concrete

Concrete simulation is the local, interactable tier.

It supports:

- exact local position
- local topology and obstacles
- field of view, hearing, smell, touch, and magical detection
- concrete inventory and equipment
- body parts, wounds, conditions, and materials where relevant
- affordance binding
- dialogue targets
- combat positioning
- actor `ActionRequest`s
- concrete `ProcessTick`s

Concrete execution:

```text
Intent
  -> Activity
  -> ActionRequest for discrete attempts
  -> ProcessInstance for long-running activities
  -> ProcessTick for activity continuation
  -> CausalTransaction
  -> EventRecord
```

### Abstract

Abstract simulation tracks a relevant actor, group, place, or thread without
running per-turn concrete actions.

It supports:

- stable identity
- current coarse location
- route progress
- important inventory or cargo
- important wounds and conditions
- active `Intent`
- active `ProcessInstance`
- risk, delay, and progress state
- trace generation
- provenance

Abstract execution:

```text
Intent
  -> Activity
  -> ProcessInstance
  -> ProcessTick at abstract resolution
  -> CausalTransaction
  -> EventRecord
```

Abstract simulation should not synthesize hidden concrete `ActionRequest`s such
as repeated offscreen moves, attacks, waits, and searches. It advances the same
process system over a coarser state surface.

### Strategic

Strategic simulation tracks regions, factions, institutions, markets, weather,
threats, rumors, and scenario pressure.

It usually does not track individual actor intent. It may run region, faction,
or scenario processes.

Examples:

```text
RegionPressure(north_road)
  bandit_activity: high
  trade_flow: disrupted
  patrol_density: low

FactionProcess(shrine_order)
  investigate_relic_theft
  pressure: increasing
```

Strategic state can be:

- hard aggregate state when gameplay-authoritative
- soft chronology or world-context state when generated or authored
- social/institutional soft truth when it concerns norms, authority, claims,
  faction membership, reputation, or obligations

The authority class decides the store and commit gate.

## Shared Model Across Tiers

Do not split `Process` into `AbstractProcess` and `ConcreteProcess`.

Use one `ProcessInstance` model:

```text
ProcessInstance
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
  active_resolution
  version
```

Each process definition declares which resolutions it supports:

```text
ProcessDef
  supported_resolutions: concrete | abstract | strategic
  concrete_tick?
  abstract_tick?
  strategic_tick?
  promotion_policy?
  demotion_policy?
```

The tick system is shared:

```text
ScheduledWakeup
  -> ProcessTick
  -> process definition resolves tick for active_resolution
  -> CausalTransaction
  -> EventRecord
```

The difference is the state surface and tick policy, not the existence of a
separate process system.

## Intent Lowering

Intent can exist at concrete and abstract resolution.

```text
Concrete:
  Intent lowers through Activity to ActionRequest or ProcessInstance.

Abstract:
  Intent lowers through Activity to ProcessInstance.

Strategic:
  individual intent is not active by default; processes and pressure usually
  drive the tier.
```

Examples:

```text
Intent(FleeAndRecover)
  concrete:
    ActionRequest(MoveToCover)
    ActionRequest(BandageWound)
    ProcessInstance(RestAndRecover)

  abstract:
    ProcessInstance(FleeAndRecover)
    route_progress, pursuit_risk, condition_summary
```

Intent owns purpose and reusable goal shape. It does not mutate truth. Process
and action lowering decide how the purpose executes at the active resolution.

## Resolution-Aware Location

Movement must continue in abstract simulation. Otherwise promotion cannot know
where to materialize the actor or group.

The location relation is hard truth, but its target can have different
granularity.

```text
Concrete:
  LocatedIn(caravan_1, tile_42_17)

Abstract:
  LocatedIn(caravan_1, north_road_segment_3)
  RoutePosition(caravan_1, route=north_road_to_market, progress=0.53)

Strategic:
  LocatedIn(caravan_1, north_road_region)
```

The current authoritative location is the best resolution the simulation is
currently claiming. A demoted actor should not continue to claim an exact tile
as current truth if the engine is no longer maintaining tile-level movement.

Previous exact information can remain as provenance:

```text
last_concrete_position
last_observed_position
EventRecordRef
EpistemicRecord
```

Those are not the same as current exact position.

## Abstract Movement

Movement processes own route progress.

```text
ProcessInstance(TravelToMarket)
  owner: caravan_1
  route: north_road_to_market
  progress: 0.42
  active_resolution: abstract
  risk: storm, bandit_activity
```

An abstract `ProcessTick` may:

- advance route progress
- update coarse `LocatedIn`
- update `RoutePosition`
- apply delay or hazard risk
- update important condition summaries
- emit traces
- schedule future wakeups
- request promotion if the protagonist or another concrete observer approaches

Example:

```text
Before:
  LocatedIn(caravan_1, north_road_segment_2)
  RoutePosition(caravan_1, progress=0.42)

ProcessTick:
  progress += 0.11
  risk check: storm delay

After:
  LocatedIn(caravan_1, north_road_segment_3)
  RoutePosition(caravan_1, progress=0.53)
  EventRecord(RouteProgressed)
  EventRecord(CaravanDelayed)
```

This is hard state mutation. It goes through `CausalTransaction`.

## Promotion

Promotion increases active resolution.

It does not replace identity. It refines state.

```text
Abstract state:
  caravan_1 on north_road_segment_3
  route_progress=0.53
  delayed=true
  trace=broken_wagon_tracks

Promotion:
  protagonist enters concrete range

Concrete state after commit:
  wagon entity placed near road bend
  wounded_guard placed beside wagon
  tracks placed toward forest
  missing_goods absent
```

Promotion may use a materialization plan, but the plan is not authoritative
truth. If the promotion creates or refines hard state, it commits through
`CausalTransaction`.

Promotion should preserve:

- stable identity
- current coarse location
- route progress
- important items
- wounds and conditions
- active process and intent
- provenance from prior `EventRecord`s
- actor truth and social consequences

## Demotion

Demotion decreases active resolution.

It does not replace the entity or process:

```text
bad:
  delete ConcreteActorState
  create AbstractActorState

good:
  keep actor identity
  keep ProcessInstance identity
  change active_resolution
  change current location granularity where needed
  release or summarize local-only reservations
  invalidate local derived views
```

Demotion may mutate hard state when the current representation must become
coarser.

Example:

```text
Concrete:
  LocatedIn(bandit_chief, tile_12_08)
  Condition(left_arm_cut)
  ContainedIn(stolen_relic, bandit_chief_inventory)
  ProcessInstance(FleeAndRecover, active_resolution=concrete)

Demoted:
  LocatedIn(bandit_chief, north_road_segment_1)
  RoutePosition(bandit_chief, route=north_gate_to_old_mill, progress=0.18)
  Condition(left_arm_cut)
  ContainedIn(stolen_relic, bandit_chief_inventory)
  ProcessInstance(FleeAndRecover, active_resolution=abstract)
```

The exact tile may remain as `last_concrete_position` or observation
provenance, but not as maintained current tile truth.

Demotion should preserve:

- stable identity
- alive/dead state
- important conditions
- important inventory and cargo
- ongoing process
- selected or active intent where still meaningful
- witness memory and actor truth
- social claims, obligations, and reputation consequences
- `EventRecord` provenance

It may discard or summarize:

- unobserved exact tile path
- local-only tactical stance
- minor unimportant carried objects
- transient local reservations
- cached field-of-view and pathfinding views

## Observation Boundary

Actors do not observe resolution tiers directly.

They observe evidence:

- visible entities
- tracks
- smoke
- sounds
- delayed arrivals
- missing patrols
- rumors
- testimony
- remembered last positions
- role-granted reports

An AI agent controlling an actor should not receive omniscient abstract state.
It should receive `ObservedState`, `ObservedEvent`, `EpistemicRecord` working
sets, and actor-accessible social context.

## Store And Authority Mapping

Resolution does not choose the store by itself. Authority class does.

```text
Concrete hard state:
  WorldStore / hard RelationStore / RuntimeControlStore / EventHistoryStore

Abstract hard state:
  WorldStore / hard RelationStore / RuntimeControlStore / EventHistoryStore

Strategic hard aggregate:
  WorldStore / hard RelationStore / RuntimeControlStore / EventHistoryStore

Generated or authored background:
  ChronologyStore through AcceptedChronologyRecord

Memory, rumor, belief, or known last position:
  EpistemicStore through AcceptedEpistemicUpdate

Faction, law, ownership, rank, reputation, or obligation:
  SocialInstitutionalStore through AcceptedSocialUpdate

Meaning, thought, pressure, or goal pressure:
  AppraisalRecordStore through AcceptedAppraisalRecord
```

## Time Model

All resolutions share one `sim_time` axis.

Concrete work usually uses finer wakeups. Abstract and strategic processes may
schedule coarser wakeups.

```text
Concrete:
  actor activation, concrete process wakeups, reactions

Abstract:
  process wakeups over route progress, risk, traces, recovery, pursuit

Strategic:
  region, faction, economy, weather, and scenario pressure wakeups
```

Promotion and demotion happen at specific simulation times and keep provenance
from the state they refine or summarize.

## AI Authority

AI may propose:

- materialization details
- soft chronology
- rumors
- appraisal interpretations
- summaries of distant situations

AI does not directly commit hard state.

If an AI proposal affects hard state, it must become a checked plan whose hard
effects commit through `CausalTransaction`. If it affects soft chronology,
epistemic state, social state, or appraisal state, it must use the
corresponding accepted update surface.

## Scenario: Caravan Travel

```text
Strategic:
  north_road.bandit_activity = high
  trade_flow = disrupted

Abstract:
  ProcessInstance(TravelToMarket)
    owner=red_road_caravan
    route=north_road_to_market
    progress=0.53
    active_resolution=abstract
    risk=bandit_activity_high, storm

ProcessTick:
  delay risk triggers
  LocatedIn updates to north_road_segment_3
  broken_wagon_tracks trace is created
  EventRecord(CaravanDelayed)

Promotion:
  player reaches north_road_segment_3
  concrete placement is refined from route position and trace state
  CausalTransaction commits wagon, guard, tracks, and missing cargo facts
```

## Scenario: Bandit With Stolen Relic

```text
Concrete:
  bandit_chief steals shrine_relic
  player wounds bandit_chief
  bandit_chief starts fleeing

Demotion:
  bandit_chief leaves concrete area
  active_resolution changes to abstract
  LocatedIn becomes north_road_segment_1
  ProcessInstance(FleeAndRecover) continues
  stolen_relic and wound remain hard facts

Abstract:
  ProcessTick advances route progress toward old_mill
  pursuit risk remains high
  blood trail may be emitted as trace

Promotion:
  player follows trail
  bandit_chief materializes near a plausible route anchor with wound and relic
  preserved
```

## Stable Decisions

- Resolution is detail/execution policy, not truth authority.
- Do not create separate abstract and concrete process systems.
- `ProcessInstance` is shared across resolutions.
- A process definition declares which resolutions it supports.
- Concrete process definition vocabularies may be pack-owned, but
  `ProcessInstance` execution is shared engine mechanism.
- Concrete simulation supports `ActionRequest` and `ProcessTick`.
- Abstract simulation uses `ProcessTick`, not hidden per-turn concrete
  `ActionRequest`s.
- Intent can exist in concrete and abstract resolution.
- Concrete intent lowers through `Activity` to `ActionRequest` or
  `ProcessInstance`.
- Abstract intent lowers through `Activity` to `ProcessInstance`.
- Abstract movement updates authoritative coarse location and route progress.
- Promotion refines state; demotion changes active resolution and coarsens only
  state whose current truth requires coarser representation.
- Hard state changes at every resolution use `CausalTransaction` and
  `EventRecord`.
- Actors observe evidence, not omniscient resolution state.

## Deferred Decisions

- exact resolution activation thresholds
- exact process definition schema for supported resolutions
- first process effect vocabulary for abstract and strategic ticks
- materialization event taxonomy
- demotion summarization policy per process family
- how much strategic state should be hard aggregate versus soft chronology
- AI proposal review policy for materialization details
- debug UI for current resolution, route progress, and provenance
