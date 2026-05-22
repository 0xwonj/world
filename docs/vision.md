# Vision

`world` is a simulation-first RPG project.

The long-term direction is closer to Caves of Qud or Dwarf Fortress than to a
graphics-led RPG. The interesting part is not visual fidelity. The interesting
part is a world where entities, materials, knowledge, factions, tools,
environmental rules, and agent decisions create varied outcomes.

## Core Idea

Build a headless simulation engine first.

Clients are adapters on top of the simulation:

- terminal client
- web client
- native graphical client
- replay viewer
- debug inspector
- AI-agent runner
- automated simulation test harness

No client should own game truth. Clients observe state and submit actions.

## Design Principles

- Determinism first: the same seed and action log should reproduce the same
  result.
- Events first: world changes should be inspectable, replayable, and testable.
- Partial observation: actors do not receive omniscient state.
- Symmetric control: human players, scripted NPCs, and AI agents use the same
  action path.
- Systemic content: prefer reusable rules over one-off scripted exceptions.
- Debuggability: every surprising result should be explainable from state,
  rules, actions, and events.

## Initial Vertical Slice

The first playable slice should be intentionally small:

- 20x20 grid map.
- One human-controlled player.
- Three NPCs.
- Movement, inspection, pickup, use, talk, attack, wait.
- Basic field-of-view and hearing.
- Basic inventory and item tags.
- Basic faction/reputation relation.
- Rule-based NPC agent.
- LLM-agent adapter behind the same agent interface.
- Event log replay.

