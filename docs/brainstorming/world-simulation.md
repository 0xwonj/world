# World Simulation

The world should be modeled as an evolving simulation rather than as a set of
screen-specific scenes.

## Simulation Axes

- Space: 2D grid, regions, rooms, zones, overmap.
- Time: deterministic turns, scheduled events, delayed effects.
- Matter: items, materials, terrain, fluids, gases, temperature.
- Bodies: anatomy, wounds, equipment, senses, needs.
- Mind: goals, memory, fear, loyalty, beliefs, personality.
- Society: factions, laws, reputation, rumors, trade.
- History: ruins, artifacts, generated lineages, old conflicts.

## Core Loop

```text
collect intents
validate actions
resolve actions
emit events
update state
derive observations
ask agents for next actions
```

## Desired Properties

- Replayable from seed plus action/event log.
- Inspectable by debug tools.
- Testable without a frontend.
- Capable of running faster than real time for AI-only simulations.
- Capable of pausing for human input.

## Early Non-goals

- High-fidelity graphics.
- Realtime combat.
- Large content database before core rules exist.
- A complex UI before the simulation loop is proven.

