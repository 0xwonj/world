# Actor-Owned Capability-Derived Actions

## Status

Promoted source history.

## Promotion Note

Stable design content from this idea has been promoted into
[Capability, Affordance, And Actor Interface](../design/capability-affordance-and-actor-interface.md).
This file remains as source history for actor-owned action-space design.

## Core Idea

Every world-changing interaction should converge into an `ActionRequest`.

The actor's action space should belong to the actor. It should be derived from
actor-owned state and capabilities, not from external objects.

External objects do not create the actor's action space. They expose perceived
affordances and context that determine how an actor-owned action schema can be
bound, validated, and resolved.

```text
Capability = why or how an actor owns an action schema
ActionRequest = what the actor attempts this turn
Affordance = what an observed target appears to support
Event = what actually happened
```

This keeps interaction paths unified while still allowing body parts, skills,
magic, conditions, knowledge, equipment, and internalized social authority to
produce varied possibilities.

## Motivation

`world` should support actors whose possible action schemas depend on more than
a fixed class or command list. A character may manipulate objects with a hand,
move them with telekinesis, command a servant, or use a tool arm. Those are
different capability sources, but the final interaction should still pass
through the same validation, resolution, event, and state update path.

This model is attractive because it gives depth without letting each subsystem
invent its own mutation path.

It also gives AI agents a stable interface. The agent does not need a wholly
new action list for every nearby object. It receives the actor's stable action
repertoire, the actor's observation of the world, and the perceived affordances
of relevant targets.

## Model Sketch

```text
ActorOwnedState
  + Body
  + Equipment
  + Skills
  + Magic
  + Conditions
  + Knowledge
  + InternalizedSocialAuthority
  + LearnedActionSchemas
    |
    v
CapabilitySet
    |
    v
ActionRepertoire
    |
    v
ObservedTargets + PerceivedAffordance + Context
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

`CapabilitySet` is derived from actor-owned authoritative state. It is not
itself authoritative world change.

`ActionRepertoire` is the actor-owned set of action schemas the actor can
attempt in principle.

`ObservedTargets`, `PerceivedAffordance`, and `Context` do not create the
repertoire. They determine whether a particular `ActionRequest` can bind to a
target, what checks are required, and which typed effects may result.

`ActionRequest` is the only way an actor tries to change the world.

## Actor-Owned Capability Sources

Capabilities should come from actor-owned sources:

- `Body`: hands, eyes, legs, wings, lungs, tail, antennae, natural weapons,
  sensory organs.
- `Equipment`: tools, weapons, keys, lights, books, containers, masks,
  implants, artifacts carried, worn, installed, or otherwise controlled by the
  actor.
- `Skills`: lockpicking, medicine, tracking, literacy, ritual practice,
  persuasion, crafting.
- `Magic`: teleportation, divination, fire shaping, warding, mind reading,
  telekinesis.
- `Conditions`: blindness, exhaustion, blessing, poison, silence, burning,
  fear, pain, broken limbs.
- `Knowledge`: passwords, recipes, maps, laws, monster weaknesses, names,
  rituals, taboos.
- `InternalizedSocialAuthority`: rank, office, oath, permission token,
  recognized identity, disguise, debt claim, or role known to the actor.
- `LearnedActionSchemas`: practiced techniques, rituals, recipes, procedures,
  legal forms, combat maneuvers, spells, and trained interactions.

These sources should be composable. No source should receive a special shortcut
around the action/event model.

External objects, terrain, other actors, local law, light, sound, hazards, and
places are not actor-owned capability sources. They may expose affordances or
context that an actor-owned action schema can use.

## Capability Effects

A capability can affect action space in several ways:

- grant an action schema
- alter the parameter space of an action schema
- extend range
- reduce or increase cost
- change risk
- add an actor-owned resolution method
- block or degrade an action schema
- modify observation
- modify event resolution

Examples:

- A usable hand grants manual manipulation.
- Telekinesis grants distant manipulation without a hand.
- Blindness blocks visual inspection but may leave hearing-based inspection
  available.
- A known passphrase grants the ability to perform a ritualized speech action.
- A recognized office grants the ability to assert authority.
- A carried crowbar grants a tool-application method.

Those capabilities do not say which target will accept the action. A sealed
door, suspicious guard, wooden wall, or ritual altar provides that through
perceived affordances and later validation.

## State Implications

Capabilities should usually be derived, not stored directly.

Capability derivation can have multiple typed families:

- physical body and senses
- controlled equipment and tools
- learned skills and procedures
- knowledge and language
- conditions and status effects
- internalized social authority
- magic, ritual attunement, and pacts

Physical capacity scores such as grip, balance, or fine manipulation may be
useful inside the physical derivation family. They should not be treated as the
general model for all capability.

Authoritative state should store the facts that capabilities derive from:

- actor body structure and body-part condition
- equipped and carried items
- learned skills and known facts
- active effects and conditions
- internalized social standing, roles, permissions, oaths, and claims
- learned procedures, rituals, recipes, and action schemas

The engine can cache a `CapabilitySet` for performance, but the cache must be
rebuildable from authoritative state.

External state should be stored where it belongs:

- target physical properties
- target affordances
- place facts
- observed hazards
- light, sound, smell, and material fields
- ownership and social claims
- local law, taboo, and institutional context

These may affect binding, validation, resolution, or semantic interpretation.
They should not be folded into the actor's owned action repertoire.

## Affordance And Context

Affordances belong to observed targets and contexts, not to the actor.

Examples:

```text
wooden door:
  closed
  openable
  burnable
  bashable
  maybe locked
  maybe ritually sealed

lit torch:
  emits_light
  emits_heat
  can_ignite_flammable_targets

altar:
  inspectable
  writable_surface
  maybe name_responsive
  maybe accepts_offering
```

An affordance does not grant the actor a new action schema. It tells the engine
how an actor-owned schema can bind to a target and what effect path may be
available.

Example:

```text
Actor repertoire:
  ApplyTool(tool, target, mode)

Observed state:
  actor carries lit torch
  door appears wooden and burnable

ActionRequest:
  ApplyTool(torch, door, ignite)

Validation and resolution:
  torch emits heat
  door material can ignite
  actor can reach door
  -> ignite / smoke / heat / sound events
```

The actor owned `ApplyTool`. The door did not add `BurnDoor` to the actor. The
door made one binding of `ApplyTool` meaningful.

Hidden affordances should not be projected as known options. If a door has a
hidden trap, the actor should not see `DisarmTrap` unless perception, knowledge,
or investigation produced enough evidence. The actor may still attempt a
generic inspection or probe action.

## Action Boundary

All world-changing interaction should become an `ActionRequest`.

Good boundaries:

- `Capability` answers: why does this actor own this action schema?
- `Affordance` answers: what does this observed target appear to support?
- `ActionRequest` answers: what schema, arguments, and mode is this actor
  attempting now?
- `Validation` answers: is this attempt valid against hard truth?
- `Resolution` answers: how is this attempt resolved?
- `Event` answers: what actually happened?

Capabilities must not:

- directly move actors
- directly damage objects
- directly add knowledge
- directly change reputation
- directly modify inventory
- directly create UI messages as the record of truth

Capabilities may:

- grant or block an actor-owned action schema
- change the parameters an actor can supply
- provide an actor-owned method
- affect action cost or risk
- affect what the actor can perceive

Affordances and context may:

- make a target appear bindable or unbindable for a schema
- provide target-specific requirements
- select a typed effect path
- reveal or hide possible target interactions
- affect success, cost, risk, and consequences

## Event Implications

Resolution should emit structured events that preserve both the attempt and the
consequence.

Example lock interaction:

```text
ActionRequest
  actor: thief
  schema: ApplyTool
  target: bronze_gate
  tool: bent_lockpick
  mode: pick_lock

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
  schema: Attack
  target: stuck_door
  mode: force_open

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
actor-specific interface that separates owned action schemas from perceived
target affordances.

An agent turn input might include:

```text
action_repertoire:
  - Move(direction)
  - Inspect(target)
  - Manipulate(target, mode)
  - ApplyTool(tool, target, mode)
  - Speak(target, speech_act)
  - StartProcess(kind, target, tools?)
  - InvokeKnownRitual(ritual, target)

perceived_entities:
  bronze_gate:
    perceived_affordances:
      - closed
      - openable
      - locked
      - metal
      - has_visible_seal

  bent_lockpick:
    perceived_affordances:
      - tool_quality(lockpick, 1)

known_actor_capabilities:
  - fine_manipulation
  - speech
  - lockpicking
  - known_ritual(gate_passphrase)
```

The agent then submits an `ActionRequest`:

```text
ActionRequest:
  schema: ApplyTool
  actor: thief
  tool: bent_lockpick
  target: bronze_gate
  mode: pick_lock
```

The engine validates the request against hard truth and resolves it through
typed effects. The agent interface may include convenience hints or blocked
reasons, but the canonical interface should not depend on enumerating every
concrete target-bound action each turn.

This format helps humans, scripted NPCs, and language-model agents use the same
action path while keeping the actor's action space stable and explainable.

## Examples

### Door

The actor owns stable action schemas. The door provides perceived affordances.

Actor-owned repertoire:

- `Manipulate(target, mode)`
- `ApplyTool(tool, target, mode)`
- `Attack(target, mode)`
- `Speak(target, speech_act)`
- `InvokeKnownRitual(ritual, target)`

Door affordances:

- `openable`
- `lockable`
- `bashable`
- `burnable`
- `ritually_sealed`

Possible requests:

- `Manipulate(door, open)`
- `ApplyTool(key, door, unlock)`
- `ApplyTool(lockpick, door, pick_lock)`
- `Attack(door, bash)`
- `ApplyTool(torch, door, ignite)`
- `Speak(door, request_opening)`
- `InvokeKnownRitual(opening_phrase, door)`

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

The actor-owned schema may be `Inspect` or `ReadObject`. The text's perceived
affordances and the actor's knowledge determine what reading modes are possible
and what events follow.

### Combat

The actor may own combat schemas through:

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
- If external objects create actor action schemas directly, the action space
  stops belonging to the actor and becomes unstable for agents.
- If actions are too generic, debugging and agent reasoning become opaque.
- If actions are too specific, action types explode.
- If capability derivation is not deterministic, replay becomes fragile.
- If capability explanations are not preserved, action repertoire becomes hard
  to debug.
- If perceived affordances leak hidden truth, actor knowledge and hard truth
  collapse together.
- If target affordances are not typed, resolution becomes a stringly dispatch
  problem.
- If subsystems bypass action requests, source-of-truth boundaries collapse.

## Open Questions

- Should `Capability` be a typed enum, data-driven tag, rule-derived fact, or a
  mix?
- What is the right split between actor-owned action schemas and target-owned
  affordances?
- What is the right granularity for action types?
- How should conflicting capabilities resolve? For example, one condition grants
  speech while another silences the actor.
- Should cost and risk be part of capability derivation, action validation, or
  action resolution?
- How much of the capability and affordance explanation should be exposed to
  human UI and AI agents?
- Should convenience hints ever enumerate likely requests, or should the agent
  always construct `ActionRequest`s from schemas and observations?
- Are some interactions automatic reactions rather than actions? If so, do they
  still emit action-like attempts or only events?

## Related References

- [Caves of Qud](../references/caves-of-qud.md)
- [Cataclysm: Dark Days Ahead](../references/cataclysm-dda.md)
- [Agent Interface](../brainstorming/agent-interface.md)
- [Action and Event Model](../design/action-event-model.md)
- [Perception And Observation](../design/perception-and-observation.md)
- [Typed Action Effects](typed-action-effects.md)
- [Kernel Primitives](kernel-primitives.md)
