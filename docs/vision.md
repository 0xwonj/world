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

No client owns game truth. Actor-facing clients receive capability-scoped,
actor-relative projections and select supplied grounded actions or submit
another explicitly authorized request through the same engine boundary.
Operator and research inspectors may receive broader read-only views without
gaining mutation authority.

## Design Principles

- Determinism first: the same semantic epoch, initial root, admitted and
  captured inputs, and management trace should reproduce the same authority
  history.
- Causal records first: accepted changes, rejections, future work, and their
  provenance should be inspectable, replay-verifiable, and testable. Domain
  events describe occurrences; they are not a substitute for atomic authority
  history.
- Partial observation: actors do not receive omniscient state.
- Symmetric control: human players, scripted NPCs, and AI agents use the same
  actor-control and runtime-validation path.
- Systemic content: prefer reusable rules over one-off scripted exceptions.
- Debuggability: every surprising result should be explainable from state,
  exact definitions, captured inputs, decisions, authority records, and later
  causal work.
- Explicit evolution: AI- or human-authored behavior is compiled, diagnosed,
  previewed, and activated only in a new semantic epoch; it never hot-patches a
  reproducible live world.

## Reference game

The [Reference Game Vision](design/reference-game-vision.md) turns these
principles into one small frontier-settlement game and a milestone-by-milestone
architecture pressure test. It is deliberately narrower than the engine and
deeper than a synthetic API fixture.
