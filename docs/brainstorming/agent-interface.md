# Agent Interface

AI agents are a first-class target. The engine should make it easy for an AI to
control an NPC or player without receiving omniscient state or bypassing rules.

## Principle

Humans, scripted NPCs, and AI agents submit actions through the same validated
interface.

An agent receives:

- who it is
- what it can perceive
- what it remembers
- what it believes
- what goals or background it has
- what actor-owned action schemas it can use
- what affordances it perceives on observed targets

An agent returns:

- one selected `ActionRequest`
- optional speech
- optional short explanation or intent

## Input Shape

```text
AgentTurnInput
  actor_id
  turn_index
  self_state
  perceived_world
  remembered_facts
  beliefs
  goals
  background
  constraints
  action_repertoire
  perceived_affordances
  validation_hints?
```

`action_repertoire` should be stable and actor-owned. It is derived from the
actor's body, capacities, skills, knowledge, equipment, conditions, and learned
schemas.

`perceived_affordances` belongs to observed targets and context. It should not
secretly reveal hard truth. A hidden trap is not an affordance unless the actor
has perceived evidence for it.

Example:

```text
action_repertoire:
  - Move(direction)
  - Inspect(target)
  - Manipulate(target, mode)
  - ApplyTool(tool, target, mode)
  - Speak(target, speech_act)
  - StartProcess(kind, target, tools?)

perceived_affordances:
  bronze_gate:
    - closed
    - locked
    - metal
    - has_visible_seal

  bent_lockpick:
    - tool_quality(lockpick, 1)
```

## Output Shape

```text
AgentTurnOutput
  action_request
  speech?
  intent?
```

Example:

```text
action_request:
  schema: ApplyTool
  tool: bent_lockpick
  target: bronze_gate
  mode: pick_lock
intent: EnterRoomQuietly
```

## Hard Boundary

The agent never mutates world state directly.

The engine validates every submitted `ActionRequest`. Invalid actions become
either a rejection event, a failed attempt, or no-op depending on game design.

Affordance information is guidance, not authority. The engine must still check
hard truth at resolution time.

## Open Questions

- How much affordance information should the engine expose without making the
  agent omniscient?
- Should convenience hints ever include likely target-bound requests, or should
  agents always construct requests from schemas and perceived affordances?
- How much memory should be stored by the engine versus by the external agent?
- Should agents be allowed to maintain private notes outside the simulation?
- How do we represent lies, mistakes, hallucinated beliefs, and superstition?
- How do we keep LLM agent output compact enough for frequent turns?
