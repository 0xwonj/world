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
- what actions are currently legal or plausible

An agent returns:

- one selected action
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
  available_actions
```

## Output Shape

```text
AgentTurnOutput
  action
  speech?
  intent?
```

## Hard Boundary

The agent never mutates world state directly.

The engine validates every submitted action. Invalid actions become either a
rejection event, a failed attempt, or no-op depending on game design.

## Open Questions

- Should the engine enumerate every valid action, or provide action schemas plus
  local affordances?
- How much memory should be stored by the engine versus by the external agent?
- Should agents be allowed to maintain private notes outside the simulation?
- How do we represent lies, mistakes, hallucinated beliefs, and superstition?
- How do we keep LLM agent output compact enough for frequent turns?

