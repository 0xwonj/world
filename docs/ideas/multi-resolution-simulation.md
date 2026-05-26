# Multi-Resolution Simulation

## Status

Promoted source history.

## Promotion Note

The current design owner is
[Multi-Resolution Simulation](../design/multi-resolution-simulation.md).

This idea document remains as source history. Some wording below predates the
current design boundary. In particular, the promoted design now says:

- resolution is execution/detail policy, not truth authority
- do not split process into separate abstract and concrete systems
- concrete simulation supports `ActionRequest` and `ProcessTick`
- abstract simulation uses the shared `ProcessInstance` / `ProcessTick` system
  instead of hidden per-turn concrete actions
- abstract movement updates authoritative coarse location and route progress
- hard state changes at any resolution commit through `CausalTransaction` and
  `EventRecord`

## Dependency Chain

This idea builds on lower-level source-of-truth documents:

1. [Action and Event Model](../design/action-event-model.md)
2. [Actor Intent And Activity](actor-intent-and-activity.md)
3. Multi-Resolution Simulation

Action/event rules define how the world changes. Actor intent defines how an
actor keeps purpose across turns. Multi-resolution simulation decides how much
of that machinery is active at different distances and relevance levels.

## Core Idea

`world` should not simulate the entire world at full grid/action resolution all
the time. It should use different simulation resolutions depending on distance,
relevance, observability, and player history.

The useful shape is three layers:

```text
Distant / Strategic
  far from the protagonist
  region pressure, faction state, rumors, economy, danger
  no per-actor intent by default

Nearby / Abstract
  close enough to become relevant soon
  tracked actors or groups can hold AbstractIntent
  progresses through time, risk, traces, and outcome events

Local / Concrete
  interactable range
  concrete actors, grid positions, perception, combat, dialogue, items
  ConcreteIntent produces ActionRequest
```

The protagonist's immediate world remains precise and inspectable. The far
world still changes, but at a coarser level.

## Motivation

The project is currently framed as a single-protagonist RPG, not a RimWorld-like
colony manager. That means the player should usually control one main actor
directly, and the most important moment-to-moment loop should stay close to:

```text
player chooses action
engine validates action
rules resolve action
events update state
observations are derived
```

However, the surrounding world should not feel frozen. Guards should patrol,
caravans should travel, injured enemies should recover or flee, factions should
react, rumors should spread, rituals should progress, and threats should build.

Full simulation for every offscreen actor is too expensive and too noisy. It
also creates false precision: the game would commit to many details the player
never observed and may never care about.

Multi-resolution simulation keeps the local game deep while letting the wider
world move in a controlled, explainable way.

## Layer 1: Distant / Strategic

The distant layer represents places, factions, and pressures that are too far
from the protagonist to need tracked individual activity.

This layer should usually store:

- regional danger
- faction tension
- trade flow
- food scarcity
- disease pressure
- weather pressure
- rumor pools
- broad migration or patrol density
- important unresolved historical events
- scenario or director pressure

Example:

```text
RegionState
  region: north_road
  bandit_activity: high
  trade_flow: disrupted
  patrol_density: low
  rumor_pool:
    - caravans have been delayed near the old bridge
```

This layer does not need per-actor action selection. It can advance through
coarse time steps and produce abstract pressure changes:

```text
BanditActivityIncreased
TradeFlowReduced
RumorEnteredPool
StormFrontMoved
FactionTensionRaised
```

## Layer 2: Nearby / Abstract

The nearby layer starts when something is close enough, relevant enough, or
connected enough to the protagonist that it should become individually tracked.

This is where the shared intent system begins. The intent is abstract: it does
not produce concrete movement or attack actions yet. It advances progress,
checks risk, leaves traces, and may produce abstract outcome events.

Example:

```text
AbstractIntent
  subject: red_road_caravan
  kind: TravelToMarket
  route: village_a -> old_bridge -> market_town
  progress: 0.63
  urgency: normal
  risk_factors:
    - bandit_activity_high
    - storm_front_nearby
  traces:
    - wagon_tracks
    - campfire_remains
  materialization_hint:
    location: old_bridge_approach
    state: delayed_and_wary
```

Possible abstract outcomes:

```text
CaravanProgressed
CaravanDelayed
CaravanChangedRoute
CaravanAmbushed
TracksLeft
RumorGenerated
```

The point is that the caravan is meaningfully progressing, but the engine is not
calculating every wagon step on the grid.

## Layer 3: Local / Concrete

The local layer is the protagonist's interactable world. It should use the
highest resolution:

- concrete entities
- grid positions
- field of view
- hearing and other senses
- precise items and containers
- dialogue targets
- combat state
- concrete intent
- validated action requests
- structured events

In this layer, intent resolves into actual action requests:

```text
ConcreteIntent
  actor: caravan_guard_1
  kind: ProtectMerchant
  target: merchant_1
  next_action_policy: hold_cover_between_threat_and_target
  interrupt_conditions:
    - merchant_down
    - guard_panic
    - line_of_sight_lost

ActionRequest
  actor: caravan_guard_1
  action: MoveToCover
  target: broken_cart

Events
  ActorMoved
  CoverOccupied
```

The local layer should be where exact combat, conversation, item use, stealth,
ritual interruption, and environmental manipulation happen.

## Intent Resolution Across Layers

This document does not define the intent model itself. Current ownership is
[Intent Templates And Planning](../design/intent-templates-and-planning.md),
with [Actor Intent And Activity](actor-intent-and-activity.md) as source
history. The multi-resolution question is how intent resolution changes across
simulation layers.

Abstract resolution:

```text
AbstractIntent
  -> advance progress
  -> update risk
  -> emit traces
  -> emit abstract outcome events
  -> prepare materialization state
```

Concrete resolution:

```text
ConcreteIntent
  -> choose next ActionRequest
  -> validate against authoritative state
  -> resolve through rules
  -> emit events
  -> update or clear intent
```

This keeps source-of-truth boundaries intact. Intent explains and guides action
selection. It does not directly mutate world truth.

## Activation Criteria

Simulation level should not be based on distance alone.

Useful inputs:

- `distance`: how close this is to the protagonist
- `relevance`: whether this actor, group, place, or event matters
- `observability`: whether the protagonist can observe traces or effects
- `player_history`: whether the protagonist caused, witnessed, or learned about
  this thread
- `future_contact`: whether this thread may soon intersect the protagonist

Example:

```text
simulation_level = f(distance, relevance, observability, player_history)
```

A distant named assassin hunting the protagonist may deserve nearby/abstract
treatment. A nearby anonymous crowd may remain strategic pressure until the
player interacts with it.

## Promotion

Promotion increases simulation resolution.

Examples:

```text
DistantPressure
  north_road.bandit_activity = high

Player approaches north road

AbstractIntent
  subject: bandit_party
  kind: PrepareAmbush
  target: red_road_caravan
  traces: hidden_tracks, quiet_road

Player reaches old bridge

ConcreteIntents
  bandit_1: HoldCover
  bandit_2: FlankMerchant
  guard_1: ProtectWagon
```

Promotion should preserve causality. The local scene should be explainable from
the abstract state that produced it.

## Demotion

Demotion summarizes concrete detail back into abstract or strategic state when
the protagonist leaves or the thread becomes less relevant.

Example:

```text
Concrete
  bandit_chief at tile (12, 8)
  left_arm_cut
  morale_low
  stolen_relic in inventory
  fleeing north

Abstract
  subject: bandit_chief
  kind: FleeAndRecover
  last_seen: north_gate
  condition: wounded
  carried_items:
    - stolen_relic
  likely_destination: old_mill
  pursuit_risk: high
```

Demotion should keep durable facts and meaningful consequences. It should not
preserve unobserved trivia unless it matters later.

## RPG Use Cases

### Travel

Abstract:

```text
AbstractIntent: TravelToMarket
  progress increases by route and time
  risk checks can delay, reroute, or create traces
```

Concrete:

```text
ConcreteIntent: ContinueTravel
  actor moves on map
  reacts to player, danger, terrain, and weather
```

### Pursuit

Abstract:

```text
AbstractIntent: HuntPlayer
  subject: escaped_assassin
  clues: last_known_player_location
  progress: closing_distance
  risk: may lose trail
```

Concrete:

```text
ConcreteIntent: TrackPlayer
  inspect tracks
  question witness
  move toward trail
  attack if contact is made
```

### Combat

Concrete combat benefits strongly from intent:

```text
CombatIntent
  FlankPlayer
  HoldDoorway
  ProtectCaster
  RetreatToCover
  LureIntoTrap
  InterruptRitual
```

Each intent can span multiple turns but still produce one action request per
turn. This makes enemies more strategic without making action resolution
opaque.

### Rituals

Abstract:

```text
AbstractIntent: PrepareRitual
  collect participants
  increase ritual_readiness
  produce rumors or omens
```

Concrete:

```text
ConcreteIntent: PerformRitual
  chant
  hold position
  spend components
  resist interruption
```

### Recovery

Abstract:

```text
AbstractIntent: RecoverFromWound
  condition improves or worsens over time
  may need medicine, shelter, or allies
```

Concrete:

```text
ConcreteIntent: TreatWound
  move to safe place
  use bandage
  call healer
```

### Rumor And Knowledge Flow

Distant:

```text
RegionPressure
  rumor_pool gains "caravans delayed near old bridge"
```

Nearby:

```text
AbstractIntent: SpreadRumor
  subject: traveling_merchant
  target_region: market_town
  progress: on_route
```

Concrete:

```text
ConcreteIntent: TellRumor
  talk to tavern keeper
  trade secret for favor
```

## Action And Event Boundary

The simulation layers must preserve the existing action/event principle:

```text
Actions are requests.
Events are facts.
```

Distant and nearby layers may emit abstract events, but those events should
still be structured facts. They should not be UI-only narration.

Examples:

```text
AbstractEvent
  CaravanDelayed
  RumorGenerated
  FactionPatrolRerouted
  RitualReadinessIncreased
  BanditPartyMovedRegion

ConcreteEvent
  ActorMoved
  AttackHit
  DoorOpened
  ItemPickedUp
  WitnessObservedActorAction
```

Some abstract events can later materialize into concrete state:

```text
CaravanAmbushed
  -> local scene includes damaged wagon, wounded guard, missing goods
```

## Observation Implications

Actors should not observe simulation layers directly. They observe evidence.

Examples:

- distant pressure creates rumor
- abstract intent creates tracks, smoke, delayed arrivals, missing patrols
- concrete intent creates visible tells, gestures, combat positioning, speech

An actor-specific observation should expose what the actor can plausibly know:

```text
Observation
  visible_entities
  heard_sounds
  remembered_traces
  known_rumors
  inferred_danger
  action_repertoire
  perceived_affordances
  candidate_activities
```

This lets AI agents use the same partial-information model as human players.

## Agent Implications

AI agents should be able to choose at multiple levels.

Local agent:

```text
choose next ActionRequest or ConcreteIntent
```

Nearby agent:

```text
choose or update AbstractIntent
```

Director or scenario agent:

```text
adjust pressure or propose incidents
```

However, every level must pass through validation. No agent gets to mutate
authoritative state directly.

## State Implications

The engine may need explicit state for:

- region pressures
- abstract actors or groups
- abstract intents
- concrete actors
- concrete intents
- promotion records
- demotion summaries
- materialization seeds
- traces and rumors
- event provenance

One possible model:

```text
WorldState
  regions
  entities
  actors
  facts
  event_log

RegionState
  pressures
  rumor_pool
  faction_state
  pending_incidents

AbstractActorState
  identity
  last_known_region
  condition_summary
  carried_important_items
  intent

ConcreteActorState
  position
  body
  inventory
  perception
  current_intent
```

The same actor may move between abstract and concrete representations, but
durable identity and important facts should remain stable.

## Determinism And Replay

Multi-resolution simulation must be deterministic.

Replay should be based on:

- seed
- content version
- starting world state
- player action log
- abstract event log
- promotion and demotion records
- deterministic random choices

Promotion must not invent arbitrary facts. If a caravan materializes wounded,
the abstract event or risk resolution should explain why.

Demotion must not lose important facts. If an NPC saw the protagonist commit a
crime locally, that witness memory must survive summarization.

## Design Risks

- If distant simulation is too vague, the world feels fake.
- If distant simulation is too detailed, the game wastes effort on invisible
  trivia and may create unfair outcomes.
- If promotion invents too much, local scenes feel arbitrary.
- If demotion loses too much, consequences disappear.
- If abstract intent bypasses events, replay and debugging break.
- If every actor gets abstract intent, the model becomes too heavy.
- If distance is the only criterion, important remote threads may be lost.
- If hidden offscreen events are too decisive, the player may feel punished for
  things they could not perceive or influence.

## Open Questions

- What exact distance or relevance thresholds should control promotion?
- Which actors are important enough to have abstract identities?
- Should anonymous groups become named only when they interact with the
  protagonist?
- How much detail should demotion preserve for inventory, wounds, relationships,
  and memories?
- Should abstract events be stored in the same event log as concrete events?
- How should abstract intent interact with scheduled events and director
  pressure?
- Can traces be generated from abstract intent without revealing hidden truth?
- What should happen when the protagonist interrupts an abstract event during
  materialization?
- How should saves represent actors that are halfway between abstract and
  concrete states?

## Related References

- [Action and Event Model](../design/action-event-model.md)
- [Actor Intent And Activity](actor-intent-and-activity.md)
- [RimWorld](../references/rimworld.md)
- [Caves of Qud](../references/caves-of-qud.md)
- [Perception And Observation](../design/perception-and-observation.md)
- [Knowledge, History, And Belief](knowledge-history-and-belief.md)
- [Actor-Owned Capability-Derived Actions](capability-derived-actions.md)
