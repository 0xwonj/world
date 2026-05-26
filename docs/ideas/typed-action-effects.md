# Typed Action Effects

## Status

Promoted source history.

## Promotion Note

Stable design content from this idea has been promoted into:

- [Typed Effect Primitives](../design/typed-effect-primitives.md)
- [Causal Runtime](../design/causal-runtime.md)
- [Capability, Affordance, And Actor Interface](../design/capability-affordance-and-actor-interface.md)

This file remains as source history for action/effect design pressure. Current
primitive naming and `EventRecord` contracts are owned by the design docs.
Any older `ConcreteIntent` wording below should be read as source-history
shorthand; the promoted design uses `Intent`, `Activity`, `ActionRequest`, and
`ProcessInstance` boundaries.

## Core Idea

An action should remain the basic unit of attempted world change, but actions
can be defined as typed effect programs over kernel primitives instead of one
hardcoded resolver function per action.

```text
ActionDef = typed effect program over kernel primitives
```

This keeps `ActionRequest` as the narrow waist of the simulation while making
action definitions more composable, checkable, and explainable.

## Relationship To Kernel And PL Boundary

This idea depends on the boundary from
[Semantic Kernel And PL Boundary](semantic-kernel-and-pl-boundary.md):

```text
Kernel owns causality.
PL owns meaning.
```

Typed action effects belong on the kernel/action side. They can call kernel
primitive effects and emit typed events. They cannot create social,
psychological, legal, or narrative meaning directly.

Semantic interpretation happens later:

```text
ActionDef
  -> kernel effects
  -> typed events
  -> observation
  -> semantic interpretation
```

## Why Action Still Matters

Action is not the base unit of meaning. It is the base unit of attempted world
change.

```text
Intent
  what the actor is trying to accomplish

ActionRequest
  what the actor attempts now

Effect program
  how that attempt changes physical state through kernel primitives

Event
  what actually happened

Semantic interpretation
  what the event means to observers
```

This keeps all world-changing interaction on a validated path.

## Why Not Hardcode Every Action

Handwritten resolvers are simple at first:

```text
resolve_take_item(...)
resolve_open_door(...)
resolve_pick_lock(...)
resolve_attack(...)
```

But over time they risk becoming scattered special cases.

Problems:

- repeated validation logic
- repeated event emission logic
- inconsistent action repertoire or binding explanations
- hidden state mutation without semantic/appraisal records
- hard-to-compose content interactions
- difficult checker/tooling support

Typed action effects make the structure of each action visible.

## Why Not One Generic Action

The opposite extreme is also bad:

```text
Action::Use(actor, target, mode: string)
```

Risks:

- weak typing
- unclear roles
- opaque resolver dispatch
- poor AI/action schema generation
- hard-to-debug availability

The better model is typed action definitions with typed roles and typed effect
bodies.

## Action Definition Shape

An action definition should include:

- name
- typed roles
- parameters
- requirements
- checks
- kernel effects
- emitted `EventRecord` contract
- optional affordance or availability metadata

Example:

```text
action TakeItem(actor: Actor, item: Item):
  require reachable(actor, item)
  require can_manipulate(actor)
  require movable(item)

  let from = location(item)

  effect transfer_item(item, from, inventory(actor))
  emit ItemTaken(actor, item, from)
```

Example:

```text
action OpenDoor(actor: Actor, door: Door):
  require adjacent(actor, door)
  require can_manipulate(actor)
  require door.closed
  require not door.locked

  effect set_open_state(door, open)
  emit DoorOpened(actor, door)
  emit SoundEmitted(source: door, volume: quiet)
```

Example:

```text
action PickLock(actor: Actor, tool: Item, lock: Lock):
  require adjacent(actor, lock)
  require can_manipulate(actor)
  require tool.has(Lockpick)
  require lock.locked

  emit LockpickAttempted(actor, tool, lock)

  let success = check_skill(actor, Lockpicking, lock.difficulty)

  if success:
    effect set_lock_state(lock, Unlocked)
    emit LockpickSucceeded(actor, lock)
  else:
    emit LockpickFailed(actor, lock)
    maybe effect damage_item(tool)
    maybe emit SoundEmitted(source: lock, volume: quiet)
```

These examples are sketches of an action effect language, not a finalized
syntax.

## Kernel Primitive Effects

Action definitions should only use kernel primitive effects and checks.

Candidate primitive families:

```text
query:
  location(entity)
  adjacent(actor, target)
  reachable(actor, target)
  component(entity, kind)
  inventory(actor)
  body_capacity(actor, kind)

check:
  check_skill(actor, skill, difficulty)
  check_attack(actor, target, instrument)
  check_random(table)

mutation:
  move_actor(actor, to)
  transfer_item(item, from, to)
  apply_damage(target, damage)
  set_open_state(door, state)
  set_lock_state(lock, state)
  damage_item(item, amount)
  spend_time(actor, cost)

event:
  emit Event
  emit SoundEmitted
```

The actual primitive set should stay small and typed.

## Typed Effect IR

The action language can lower into a typed effect IR.

Example:

```text
Require(Reachable(actor, item))
Require(CanManipulate(actor))
transfer_entity(item, from, to)
set_lock_state(lock, unlocked)
apply_damage(target, damage)
emit_event(ItemTaken(...))
```

The IR should be typed. Avoid generic mutation effects like:

```text
SetField(entity, "locked", false)
ModifyStat(actor, "mood", -10)
AddTag(actor, "angry")
```

Prefer domain-specific kernel effects:

```text
set_lock_state(lock, unlocked)
apply_damage(actor, damage)
add_condition(actor, bleeding)
emit_event(ActorWounded(...))
```

## Event Contract

Every physical mutation should produce a corresponding typed event.

Examples:

```text
transfer_item -> ItemTaken or ItemTransferred
move_actor -> ActorMoved
apply_damage -> ActorWounded, DamageResolved, or ActorDied
set_open_state -> DoorOpened or DoorClosed
set_lock_state -> DoorUnlocked or LockStateChanged
```

This prevents hidden mutation. Semantic interpretation must read events, not
private side effects.

Bad:

```text
effect transfer_item(item, ground, inventory(actor))
```

Good:

```text
effect transfer_item(item, ground, inventory(actor))
emit ItemTaken(actor, item, from: ground)
```

## Stage Permissions

Typed action effects are allowed to perform physical kernel work.

Allowed:

- requirements and checks
- physical state mutation through kernel primitives
- resource and time cost
- typed event emission
- sensory event emission

Forbidden:

- creating grief, fear, revenge, guilt, or other pressure
- declaring theft, taboo, insult, or crime
- changing relationship because of social meaning
- creating belief or rumor
- directly advancing quest or narrative meaning

Those belong to semantic interpretation rules over observed events.

## Action Binding And Validation

Action definitions should not be the source of the actor's action repertoire.
That repertoire comes from actor-owned capabilities.

Action definitions can still describe how a submitted action schema binds to
roles, checks target affordances, validates context, and lowers into typed
effects.

Example:

```text
Actor-owned schema:
  ApplyTool(tool, target, mode)

ActionRequest:
  ApplyTool(lockpick, bronze_gate, pick_lock)

ActionDef:
  require actor can manipulate
  require tool provides ToolQuality(lockpick)
  require target exposes Lock
  require target is reachable
  check lockpicking skill
  emit lockpick events
```

This is useful for:

- player UI
- AI agent input
- NPC policy
- debugging
- replay explanation

Validation should explain both `enabled_by` and `blocked_by` where possible,
but the canonical interface should not require enumerating every concrete
target-bound action each turn.

## Relationship To Actor-Owned Capability-Derived Actions

Actor-owned capability-derived actions answer:

```text
What action schemas does this actor own, and why?
```

Typed action effects answer:

```text
Given an ActionRequest, target affordances, and context, what typed kernel
effects and events define that attempt?
```

Example:

```text
Actor-owned capability:
  actor has fine manipulation and lockpicking skill
  actor holds a lockpick

Action repertoire:
  ApplyTool(tool, target, mode)

Observed target affordance:
  bronze_gate appears to have a lock

ActionRequest:
  ApplyTool(lockpick, bronze_gate, pick_lock)

ActionDef:
  require adjacent
  require tool.has(Lockpick)
  require target exposes Lock
  check skill
  set_lock_state on success
  emit lockpick events
```

Capabilities and action definitions meet when an actor submits an
`ActionRequest`. Affordances and context then determine whether the request is
valid and which typed effect path applies.

## Relationship To Actor Intent

Intent chooses or biases action instances. It should not directly emit typed
effects.

```text
Intent: EnterSealedRoomQuietly
  candidate requests:
    - ApplyTool(lockpick, door, pick_lock)
    - Speak(door, recite_passphrase)
    - Speak(guard, ask_to_open)
```

Each candidate action has a typed effect definition. The kernel still validates
and executes the selected action.

## Relationship To Semantic Interpretation

The action effect language and semantic interpretation language are separate.

```text
Action language:
  ActionRequest -> kernel effects -> typed events

Semantic language:
  ObservedEvents + SocialContextView -> appraisal records and proposed intent
  bias
```

The connection point is the event stream.

Example:

```text
TakeItem
  -> ItemTaken

Semantic interpretation:
  ItemTaken + owner != actor + no permission
    -> theft interpretation
    -> guard duty pressure
```

## Abstract Simulation

The promoted multi-resolution design does not use hidden abstract
`ActionRequest`s. Abstract resolution uses the shared `ProcessInstance` /
`ProcessTick` system.

Local/concrete:

```text
ConcreteIntent
  -> ActionDef instance
  -> kernel effects
  -> concrete events
```

Nearby/abstract:

```text
Intent
  -> ProcessInstance
  -> ProcessTick at abstract resolution
  -> progress, risk, traces, EventRecord entries
```

Process ticks may need process-oriented effect vocabulary, but they should keep
the same principles:

- typed effects
- stage permissions
- `EventRecord` contracts
- provenance
- no hidden mutation

## Open Questions

- What is the smallest useful action effect vocabulary?
- How much control flow should action definitions allow?
- Which mutations require mandatory `EventRecord` contracts?
- Should attack be defined as an action effect program or privileged kernel
  logic?
- How should random checks be typed for deterministic replay?
- How should perceived affordances and validation errors avoid leaking hidden
  information?
- How should action definitions refer to components without becoming stringly?
- Should the first action language be Rust-embedded IR or an external syntax?

## Related References

- [Action and Event Model](../design/action-event-model.md)
- [Actor-Owned Capability-Derived Actions](capability-derived-actions.md)
- [Kernel Primitives](kernel-primitives.md)
- [Actor Intent And Activity](actor-intent-and-activity.md)
- [Actor Pressure And Interpretation](actor-pressure-and-interpretation.md)
- [Multi-Resolution Simulation](../design/multi-resolution-simulation.md)
- [Semantic Kernel And PL Boundary](semantic-kernel-and-pl-boundary.md)
