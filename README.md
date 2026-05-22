# world

Working repository for a simulation-first RPG world.

The goal is a headless, turn-based 2D grid world simulation that can support
multiple clients: terminal, web, native UI, automated tests, and AI agents.
The frontend is not the source of truth. The simulation core is.

## Direction

- 2D grid-based, turn-based RPG world.
- Simulation depth matters more than graphics.
- Human players and AI agents should use the same action interface.
- Entities perceive only what their senses, knowledge, memory, and position
  allow them to perceive.
- World changes should flow through explicit actions, validation, events, and
  state updates.
- Content should be system-driven: materials, body parts, factions, knowledge,
  environment, tools, rituals, mutations, and social relationships should
  combine into emergent behavior.

## Initial Questions

- What does `WorldState` own?
- What is the boundary between `Action`, `Event`, and `State`?
- How are perception, memory, and knowledge represented?
- What exact schema does an AI agent receive each turn?
- How can content be authored without hardcoding every special case?
- How do we keep the simulation deterministic and replayable?

## Docs

- [Vision](docs/vision.md)
- References
  - [Caves of Qud](docs/references/caves-of-qud.md)
- Brainstorming
  - [Content Systems](docs/brainstorming/content-systems.md)
  - [Agent Interface](docs/brainstorming/agent-interface.md)
  - [World Simulation](docs/brainstorming/world-simulation.md)
- Design
  - [Simulation Core](docs/design/simulation-core.md)
  - [Action and Event Model](docs/design/action-event-model.md)
  - [Perception and Knowledge](docs/design/perception-and-knowledge.md)

