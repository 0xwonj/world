# Capability, Affordance, And Actor Interface

## Status

Current design draft.

## Source Ideas

- [Actor-Owned Capability-Derived Actions](../ideas/capability-derived-actions.md)
- [Kernel Primitives](../ideas/kernel-primitives.md)
- [Epistemic State / Agent Memory](../research/epistemic-state-and-agent-memory.md)
- [Engine Architecture Research Entry](../research/engine-architecture-entry.md)

## Related Design Owners

- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)

## Purpose

This document defines the actor-facing interface for what an actor can attempt,
what an actor can perceive, and how observed targets can be bound to actor-owned
action schemas.

It is the main bridge between hard simulation and AI-agent/native-NPC control.

## Core Principle

The actor's action space belongs to the actor.

```text
actor-owned state
  -> CapabilitySet
  -> ActionRepertoire

observed world
  -> PerceivedAffordance
  -> target/context binding

ActionRepertoire + PerceivedAffordance + intent/policy
  -> ActionRequest
```

External objects do not create the actor's action repertoire. They provide
signals, constraints, context, and perceived affordances.

## Interface Construction Order

The actor-facing interface is built in this order:

```text
actor-owned hard state + accessible actor truth + recognized social authority
  -> CapabilitySet, including perceptual capability
  -> ActionRepertoire
  -> PerceptionContext
  -> ObservedState / ObservedEvent
  -> Epistemic WorkingSet
  -> accessible SocialContextView
  -> PerceivedAffordance for observed targets and contexts
  -> Pressure / GoalPressure / stable Goal / current Intent
  -> policy or agent choice
  -> ActionRequest or intent choice
```

This order matters. Perception needs actor-owned perceptual capability, but the
resulting observations must not grant new actor-owned actions by themselves.
They only expose targets, signals, uncertainty, and affordance candidates for
binding.

## Boundary

This layer owns:

- actor-owned capability derivation
- `CapabilitySet`
- `ActionRepertoire`
- perceptual capability
- `PerceivedAffordance`
- actor-facing and AI-agent-facing interface shape
- binding distinction between owned schema and observed target/context

This layer does not own:

- hard state mutation
- typed effect execution
- event commit
- memory storage
- semantic appraisal
- final intent scoring
- UI phrasing

## Capability Sources

Capabilities are derived from actor-owned state:

- body: hands, eyes, legs, wings, lungs, tail, antennae, natural weapons,
  sensory organs
- equipment: tools, weapons, keys, lights, masks, books, implants, artifacts
  carried, worn, installed, or otherwise controlled by the actor
- skills: lockpicking, medicine, tracking, literacy, crafting, persuasion,
  ritual practice
- knowledge and procedures: recipes, maps, passwords, laws, monster weaknesses,
  names, rituals, taboos
- conditions: blindness, exhaustion, poison, silence, burning, fear, pain,
  broken limbs
- magic: teleportation, divination, telekinesis, fire shaping, warding, aura
  sight
- internalized social authority: rank, office, oath, permission token,
  recognized identity, debt relation, role known to the actor
- learned action schemas: techniques, spells, legal forms, recipes, procedures,
  combat maneuvers

This document owns the actor-facing mechanism: `CapabilitySet`,
`ActionRepertoire`, perceptual capability, `PerceivedAffordance`, and agent
I/O shape. Concrete stat, skill, procedure, spell, technique, recipe, and
social-authority vocabularies are pack-owned when they are specific to a game
system.

Capabilities are usually derived and cacheable, not stored as authoritative
truth. The cache must be rebuildable from authoritative state and holder-facing
epistemic access where relevant.

## Capability Effects

A capability may:

- grant an action schema
- block or degrade an action schema
- change allowed parameter kinds
- extend range
- alter cost, speed, risk, or difficulty
- provide an actor-owned resolution method
- modify perception
- affect validation or effect resolution

A capability must not:

- directly move actors
- directly transfer items
- directly apply damage
- directly create memory, pressure, or intent
- directly create social meaning

## ActionRepertoire

`ActionRepertoire` is the actor-owned set of action schemas this actor can try
in principle.

Examples:

```text
Move(direction)
Wait
Inspect(target)
Manipulate(target, mode)
ApplyTool(tool, target, mode)
Speak(target, speech_act)
StartProcess(kind, target, tools?)
InvokeKnownRitual(ritual, target)
AssertAuthority(target, authority_basis)
```

The repertoire should be stable enough for AI agents. It should not explode
into every target-specific option each turn.

## Perceptual Capability

Perception is also actor-owned.

```text
actor body + senses + conditions + controlled equipment + magic + knowledge
  -> perceptual capability

world signals + environmental constraints
  -> observed state / observed event
```

The outside world can emit light, sound, scent, heat, smoke, aura, traces, and
other signals. The actor's perceptual capability determines what can be
received, recognized, or misread.

## PerceivedAffordance

`PerceivedAffordance` is an actor-relative projection about an observed target
or context.

Conceptual shape:

```text
PerceivedAffordance:
  observer
  subject
  affordance_kind
  status: perceived | suspected | rumored | inferred
  confidence
  source: observation | recognition | epistemic_record | social_context
```

Examples:

```text
wooden door:
  closed
  openable
  burnable
  bashable
  maybe locked

lit torch:
  emits_light
  emits_heat
  can_ignite_flammable_targets

altar:
  inspectable
  accepts_offering maybe
  name_responsive maybe
```

An affordance does not grant a new action schema. It tells the engine how an
actor-owned schema may bind to a target and what validation/effect path may be
available.

Hidden affordances should not be projected as known options unless perception,
knowledge, investigation, or rumor has exposed them. Even then, the projection
should preserve whether the affordance is known, suspected, rumored, or
inferred.

## Binding, Validation, And Resolution

The boundaries are:

```text
Capability:
  why this actor owns this schema.

ActionRepertoire:
  what schemas the actor can attempt in principle.

PerceivedAffordance:
  what this observed target/context appears to support.

ActionRequest:
  what schema, roles, arguments, and mode the actor attempts now.

Validation:
  whether the attempt is valid against hard truth.

Resolution:
  which typed effects and events happen.
```

This keeps agent interfaces compact without sacrificing target-specific depth.

## Actor And AI-Agent Input

An actor-facing turn input should be non-omniscient and structured:

```text
AgentTurnInput:
  view_id
  actor
  sim_time
  observations: ObservedState / ObservedEvent
  epistemic_working_set: relevant EpistemicRecord views
  social_context_view: accessible social context only
  capability_set: derived capability explanations
  action_repertoire: owned action schemas
  perceived_affordances: observed target/context affordances
  pressures_goals: appraisal outputs when available
  current_intent?
  active_processes?
  recent_invalid_feedback?
```

The agent should submit `ActionRequest`s or intent choices:

```text
AgentTurnOutput:
  view_id
  command:
    SelectIntent(intent)
    SubmitActionRequest(action_request)
    ContinueActivity(activity_id)
    InterruptActivity(activity_id, reason)
    Wait
  actor_visible_explanation?
```

The command is a mutually exclusive union. The output is not authoritative
state. It is input to validation, planning, or the causal runtime. When an
`ActionRequest` is submitted, its `actor_view_version` should match the
`AgentTurnInput.view_id` that informed the choice.

An agent or NPC policy must not:

- mutate world state directly
- invent hidden target state as if observed
- add target-specific actions to its own repertoire
- commit soft truth, actor truth, or narrative framing without the appropriate
  proposal/commit gate

## Invalid Action Feedback

Invalid or failed choices should return structured actor-facing feedback rather
than raw engine errors.

```text
InvalidActionFeedback:
  request_id
  category: rejected | blocked | attempt_failed | interrupted
  actor_visible_reason
  perceived_missing_capability?
  perceived_failed_binding?
  perceived_unavailable_affordance?
  observed_consequence?
  actor_visible_retry_context?
```

Feedback must preserve hidden-truth boundaries. For example, an actor can be
told "the lock does not seem to respond" or "your wounded hand slips," but not
"there is an invisible ward" unless the actor has observed or inferred that
ward. This feedback helps AI agents recover without receiving omniscient state.

## Scenario: Door Interaction

```text
Actor-owned state:
  usable hand
  carried lit torch
  basic manipulation schema

CapabilitySet:
  ApplyTool(tool, target, mode)

Observed target:
  door appears wooden and closed

PerceivedAffordance:
  burnable
  openable
  bashable

ActionRequest:
  ApplyTool(lit_torch, door, ignite)

Validation and resolution:
  check reachability
  check torch heat
  check door material
  run typed effects
```

The door does not add `BurnDoor` to the actor. The actor owns `ApplyTool`; the
door makes one binding meaningful.

## Scenario: Wounded Hand

```text
Hard truth:
  actor has wounded right hand
  actor holds lockpick

Actor-owned skill/procedure input:
  actor knows lockpicking

CapabilitySet:
  fine manipulation degraded
  ApplyTool remains owned with higher risk or cost

ActionRepertoire:
  ApplyTool(tool, target, pick_lock)

Validation:
  may fail, become slower, or require a harder check
```

The wound affects capability derivation and validation. It does not require a
bespoke action list.

## Relationship To Other Documents

- [Physical Simulation Grammar](physical-simulation-grammar.md) supplies body,
  equipment, condition, signal, and target facts.
- [Perception And Observation](perception-and-observation.md) projects current
  actor-relative observations.
- [Epistemic State](epistemic-state.md) supplies knowledge, procedures,
  memories, rumors, and secrets where accessible.
- [Typed Effect Primitives](typed-effect-primitives.md) defines what a bound
  action can lower into.
- [Causal Runtime](causal-runtime.md) validates, stages, commits, and records
  the result.
- [Intent Templates And Planning](intent-templates-and-planning.md) will later
  choose among possible intents and action requests.

## Stable Decisions

- Action space is actor-owned.
- Perception capability is actor-owned.
- External targets expose perceived affordances and context, not repertoire
  ownership.
- `CapabilitySet` is derived and cacheable, not the source of hard truth.
- `AgentTurnInput` and `AgentTurnOutput` are actor-facing policy interfaces,
  not mutation authority.
- `InvalidActionFeedback` is actor-relative and must not leak hidden truth.
- AI agents receive actor-facing observations, repertoire, affordances, and
  working sets, not omniscient state.
- Concrete capability vocabularies such as named stats, skills, spells,
  recipes, techniques, and social forms may come from game-system packs.

## Deferred Decisions

- exact `CapabilitySet` schema
- exact `ActionRepertoire` schema
- exact `PerceivedAffordance` schema
- capability cache invalidation
- whether procedure knowledge grants repertoire directly or through a separate
  learned-schema family
