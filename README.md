# world

Working repository for a simulation-first RPG engine.

The goal is a headless, turn-based world simulation with bounded local
two-dimensional square grids and a regional topology. It can support multiple
clients: terminal, web, native UI, automated tests, and AI agents. The
frontend is not the source of truth. The simulation core is.

## Direction

- Turn-based RPG play over local square grids and a regional world topology.
- Simulation depth matters more than graphics.
- Human players and AI agents should use the same action interface.
- Entities perceive only what their senses, knowledge, memory, and position
  allow them to perceive.
- World changes flow through typed admission or scheduled work, verified atomic
  authority transitions, domain events, and explicit later causal work.
- Content should be system-driven: materials, body parts, factions, knowledge,
  environment, tools, rituals, mutations, and social relationships should
  combine into emergent behavior.

## Docs

Start with the [Documentation Guide](docs/README.md). It identifies the role
and authority of every documentation layer.

- [Vision](docs/vision.md) and
  [Reference Game Vision](docs/design/reference-game-vision.md) define product
  direction and the gameplay pressure the architecture must survive.
- [Target Architecture](docs/architecture/target-architecture/README.md) is the
  only normative cross-system architecture.
- [Execution Roadmap](docs/architecture/target-architecture/implementation-roadmap.md)
  maps required capabilities to milestones and validation evidence.
- [Rewrite Status](docs/implementation/target-rewrite/README.md) records
  completed milestones and whether a detailed milestone plan is currently
  active.
- [Design Index](docs/design/README.md) classifies current, partially
  superseded, and frozen design inputs.
- [Research Index](docs/research/README.md) separates accepted dispositions
  from evidence and open hypotheses.

M1 through M7 build and operationalize the engine foundation. The target
roadmap is architecture-complete only after the M8 gameplay-composition suite
has demonstrated cross-system depth without adding a second authority path or
a speculative universal framework.
