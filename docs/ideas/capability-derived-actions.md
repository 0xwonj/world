# Capability-Derived Actions

## Status

Candidate

## Core Idea

Every world-changing interaction should converge into an `ActionRequest`.
Capabilities do not mutate world state directly. They expand, restrict, or
modify the action space available to an actor.

```text
Capability = why or how an actor can do something
ActionRequest = what the actor attempts this turn
Event = what actually happened
```

This keeps interaction paths unified while still allowing body parts, skills,
magic, conditions, knowledge, equipment, social status, and environment to
produce varied possibilities.

## Motivation

`world` should support actors whose possible actions depend on more than a
fixed class or command list. A character may pick up an item with a hand, move
it with telekinesis, command a servant to carry it, or use a tool arm. Those are
different capability sources, but the final interaction should still pass
through the same validation, resolution, event, and state update path.

This model is attractive because it gives depth without letting each subsystem
invent its own mutation path.

## Model Sketch

```text
ActorState
  + Body
  + Equipment
  + Skills
  + Magic
  + Conditions
  + Knowledge
  + SocialState
  + LocalEnvironment
    |
    v
CapabilitySet
    |
    v
AvailableActionSet
    |
    v
ActionRequest
    |
    v
Validation
    |
    v
Resolution
    |
    v
Events
    |
    v
State Update
```

`CapabilitySet` is derived from authoritative state and local context. It is not
itself authoritative world change.

`AvailableActionSet` is the actor-facing projection of capabilities into
semantic actions that can be attempted now.

`ActionRequest` is the only way an actor tries to change the world.

## Capability Sources

Capabilities can come from many sources:

- `Body`: hands, eyes, legs, wings, lungs, tail, antennae, natural weapons,
  sensory organs.
- `Equipment`: tools, weapons, keys, lights, books, containers, masks,
  implants, artifacts.
- `Skills`: lockpicking, medicine, tracking, literacy, ritual practice,
  persuasion, crafting.
- `Magic`: teleportation, divination, fire shaping, warding, mind reading,
  telekinesis.
- `Conditions`: blindness, exhaustion, blessing, poison, silence, burning,
  fear, pain, broken limbs.
- `Knowledge`: passwords, recipes, maps, laws, monster weaknesses, names,
  rituals, taboos.
- `SocialState`: faction rank, permission, disguise, reputation, debt, oath,
  ownership.
- `Environment`: darkness, water, fire, metal floor, nearby wall, sacred site,
  hostile territory.

These sources should be composable. No source should receive a special shortcut
around the action/event model.

## Capability Effects

A capability can affect action space in several ways:

- grant a semantic action
- unlock a target
- extend range
- reduce or increase cost
- change risk
- add a resolution path
- block an action
- modify observation
- modify event resolution

Examples:

- A usable hand grants manual manipulation.
- Telekinesis grants distant manipulation without a hand.
- Blindness blocks visual inspection but may leave hearing-based inspection
  available.
- A known passphrase grants a social or magical opening action on a sealed door.
- Faction permission grants legal access where physical access already exists.
- A crowbar adds a force-open resolution path for some locked objects.

## State Implications

Capabilities should usually be derived, not stored directly.

Authoritative state should store the facts that capabilities derive from:

- actor body structure and body-part condition
- equipped and carried items
- learned skills and known facts
- active effects and conditions
- social standing and permissions
- local terrain, objects, light, sound, ownership, and hazards

The engine can cache a `CapabilitySet` for performance, but the cache must be
rebuildable from authoritative state.

## Action Boundary

All world-changing interaction should become an `ActionRequest`.

Good boundaries:

- `Capability` answers: why can this actor attempt this?
- `ActionRequest` answers: what is this actor attempting now?
- `Rule` answers: how is this attempt resolved?
- `Event` answers: what actually happened?

Capabilities must not:

- directly move actors
- directly damage objects
- directly add knowledge
- directly change reputation
- directly modify inventory
- directly create UI messages as the record of truth

Capabilities may:

- make an action available
- make an action unavailable
- change action parameters
- provide a resolution option
- affect action cost or risk
- affect what the actor can perceive

## Event Implications

Resolution should emit structured events that preserve both the attempt and the
consequence.

Example lock interaction:

```text
ActionRequest
  actor: thief
  action: PickLock
  target: bronze_gate
  tool: bent_lockpick

Events
  LockpickAttempted
  LockpickSucceeded
  DoorUnlocked
  NoiseEmitted
```

Example failed force interaction:

```text
ActionRequest
  actor: wounded_guard
  action: ForceOpen
  target: stuck_door

Events
  ForceOpenAttempted
  ForceOpenFailed
  PainIncreased
  NoiseEmitted
```

Keeping attempt events explicit is useful for memory, social consequences,
debugging, replay, and AI explanation.

## Observation And Agent Implications

Actors should not receive raw omniscient state. They should receive an
actor-specific action projection.

An agent turn input might include:

```text
available_actions:
  - action: OpenDoor
    target: bronze_gate
    enabled_by:
      - body.usable_hand

  - action: PickLock
    target: bronze_gate
    tool: bent_lockpick
    enabled_by:
      - skill.lockpicking
      - item.lockpick
      - body.fine_manipulation

  - action: RecitePassphrase
    target: bronze_gate
    enabled_by:
      - knowledge.gate_passphrase
      - capability.speech
```

This format helps humans, scripted NPCs, and language-model agents use the same
action path while still explaining why each option exists.

## Examples

### Door

The same door can produce different actions depending on capability sources:

- `OpenDoor`: hand or telekinesis, unlocked door.
- `UnlockDoor`: matching key, passphrase, authority, or lock manipulation.
- `PickLock`: lockpicking skill plus tool or equivalent fine manipulation.
- `ForceOpen`: strength, tool, magic, or body size.
- `MeltLock`: acid item, heat spell, or corrosive body fluid.
- `PhaseThrough`: magic, mutation-like state, or artifact effect.
- `AskToOpen`: speech plus social relation to someone who can open it.

All of these still become action requests.

### Reading

Reading is not just a UI command. It can depend on:

- sight or tactile reading
- literacy
- known language
- translation spell
- scholar background
- item condition
- local light
- whether the actor knows the text is forbidden

The final action may be `ReadObject`, but the available readings and resulting
events depend on capabilities and knowledge.

### Combat

Attacking can derive from:

- held weapon
- natural weapon
- spell
- thrown object
- environmental hazard
- command over an ally
- knowledge of a weakness

The capability model should avoid a hard split between "combat action" and
"non-combat interaction". Both are action requests resolved by rules.

## Design Risks

- If every trait becomes a capability, the model turns into a vague tag soup.
- If actions are too generic, debugging and agent reasoning become opaque.
- If actions are too specific, action types explode.
- If capability derivation is not deterministic, replay becomes fragile.
- If capability explanations are not preserved, available actions become hard
  to debug.
- If subsystems bypass action requests, source-of-truth boundaries collapse.

## Open Questions

- Should `Capability` be a typed enum, data-driven tag, rule-derived fact, or a
  mix?
- Should `AvailableActionSet` enumerate all valid actions, or expose semantic
  action schemas plus local affordances?
- What is the right granularity for action types?
- How should conflicting capabilities resolve? For example, one condition grants
  speech while another silences the actor.
- Should cost and risk be part of capability derivation, action validation, or
  action resolution?
- How much of the enabled-by explanation should be exposed to human UI and AI
  agents?
- Are some interactions automatic reactions rather than actions? If so, do they
  still emit action-like attempts or only events?

## Related References

- [Caves of Qud](../references/caves-of-qud.md)
- [Agent Interface](../brainstorming/agent-interface.md)
- [Action and Event Model](../design/action-event-model.md)
- [Perception and Knowledge](../design/perception-and-knowledge.md)

