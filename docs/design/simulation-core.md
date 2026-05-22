# Simulation Core

This is an initial sketch, not a locked implementation design.

## Ownership

`WorldState` should own the authoritative game state:

- map and tiles
- entities
- components
- inventories and equipment
- faction state
- knowledge state
- scheduled effects
- random seed/state
- event log cursor

Clients and agents should not own authoritative state.

## Determinism

The simulation should be reproducible from:

- engine version
- content version
- world seed
- initial scenario
- ordered action log

Any random result should come from the simulation RNG, not from clients.

## State Access

Different consumers need different views:

- engine systems: authoritative state
- debug tools: privileged state
- frontend clients: renderable observations
- human players: player observation
- AI agents: actor-specific observation plus available actions
- replay viewer: event stream and derived snapshots

## First Implementation Shape

Start simple:

```text
World
  state
  rules
  rng
  event_log

World::step(action_requests) -> StepResult
```

`StepResult` should include events and updated observations.

