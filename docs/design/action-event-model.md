# Action and Event Model

Actions are requests. Events are facts.

## Action

An action represents what an actor attempts to do.

Examples:

- move north
- wait
- inspect target
- pick up item
- drop item
- use item on target
- talk to actor
- attack target
- open door
- read object

Actions can fail validation.

## Event

An event represents what actually happened in the world.

Examples:

- actor moved
- movement blocked
- item picked up
- attack missed
- attack hit
- actor heard sound
- door opened
- fire started
- faction reputation changed

Events are the basis for:

- replay
- logs
- debugging
- observation derivation
- memory updates
- UI messages
- AI context updates

## Resolution Flow

```text
ActionRequest
  -> validate against authoritative state
  -> resolve through rules
  -> emit events
  -> apply events to WorldState
  -> derive observations
```

## Rule

A rule should not directly produce UI text. It should produce structured events.
Presentation can translate events into text later.

## Open Questions

- Should events be stored before or after state application?
- Should failed validation create visible events?
- How do simultaneous actions resolve in a turn-based model?
- How granular should events be for replay and memory?

