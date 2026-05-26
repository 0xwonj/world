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
- What is the boundary between `ActionRequest`, `CausalTransaction`,
  `EventRecord`, and state?
- How are perception, epistemic state, memory, and knowledge represented?
- What exact schema does an AI agent receive each turn?
- How can content be authored without hardcoding every special case?
- How do we keep committed outcomes auditable, explainable, and replayable
  where needed?

## Docs

- [Vision](docs/vision.md)
- Architecture
  - [Architecture README](docs/architecture/README.md)
  - [Architecture Roadmap](docs/architecture/roadmap.md)
  - [Architecture Decisions](docs/architecture/ADR.md)
  - [Engine Architecture](docs/architecture/engine.md)
  - [Runtime Pipeline Architecture](docs/architecture/runtime-pipeline.md)
  - [Crate Boundary Architecture](docs/architecture/crates.md)
  - [Project Conventions](docs/architecture/project-conventions.md)
  - [Implementation Plan](docs/architecture/implementation-plan.md)
- Design
  - [Simulation Core](docs/design/simulation-core.md)
  - [Engine Core And Game System Boundary](docs/design/engine-core-and-game-system-boundary.md)
  - [Pack Authoring And Semantic Declarations](docs/design/pack-authoring-and-semantic-declarations.md)
  - [Simulation Transition Compiler](docs/design/simulation-transition-compiler.md)
  - [Truth, Authority, And Layer Boundaries](docs/design/truth-authority-and-layer-boundaries.md)
  - [World Model](docs/design/world-model.md)
  - [Physical Simulation Grammar](docs/design/physical-simulation-grammar.md)
  - [Typed Effect Primitives](docs/design/typed-effect-primitives.md)
  - [Causal Runtime](docs/design/causal-runtime.md)
  - [Action And Event Model](docs/design/action-event-model.md) terminology sketch
  - [Time Model](docs/design/time-model.md)
  - [Capability, Affordance, And Actor Interface](docs/design/capability-affordance-and-actor-interface.md)
  - [Perception And Observation](docs/design/perception-and-observation.md)
  - [Epistemic State](docs/design/epistemic-state.md)
  - [Social Institutional Model](docs/design/social-institutional-model.md)
  - [Semantic Appraisal And Motivation](docs/design/semantic-appraisal-and-motivation.md)
  - [Intent Templates And Planning](docs/design/intent-templates-and-planning.md)
  - [Multi-Resolution Simulation](docs/design/multi-resolution-simulation.md)
- Research
  - [Engine Architecture Research Entry](docs/research/engine-architecture-entry.md)
  - [Implementation Architecture And Library Survey](docs/research/implementation-architecture-and-library-survey.md)
  - [Causal Runtime / Action-Effect-Event](docs/research/causal-runtime-action-effect-event.md)
  - [World Representation / Query Model](docs/research/world-representation-query-model.md)
  - [Time Model / Turn Scheduling](docs/research/time-model-and-turn-scheduling.md)
  - [Epistemic State / Agent Memory](docs/research/epistemic-state-and-agent-memory.md)
  - [Semantic Appraisal, Intent, Activity, And Planning](docs/research/semantic-appraisal-intent-activity-planning.md)
  - [Reference Research Questions](docs/research/reference-questions.md)
- References
  - [Caves of Qud](docs/references/caves-of-qud.md)
  - [RimWorld](docs/references/rimworld.md)
  - [Cataclysm: Dark Days Ahead](docs/references/cataclysm-dda.md)
- Ideas
  - [Design Ideas](docs/ideas/README.md)
  - [Capability-Derived Actions](docs/ideas/capability-derived-actions.md)
  - [Knowledge, History, And Belief](docs/ideas/knowledge-history-and-belief.md)
  - [Kernel Primitives](docs/ideas/kernel-primitives.md)
  - [Typed Action Effects](docs/ideas/typed-action-effects.md)
  - [Actor Intent And Activity](docs/ideas/actor-intent-and-activity.md)
  - [Actor Pressure And Interpretation](docs/ideas/actor-pressure-and-interpretation.md)
  - [Layered Truth And AI Co-Authority](docs/ideas/layered-truth-and-ai-coauthority.md)
  - [Multi-Resolution Simulation](docs/ideas/multi-resolution-simulation.md)
  - [Semantic Kernel And PL Boundary](docs/ideas/semantic-kernel-and-pl-boundary.md)
- Brainstorming
  - [Content Systems](docs/brainstorming/content-systems.md)
  - [Agent Interface](docs/brainstorming/agent-interface.md)
  - [World Simulation](docs/brainstorming/world-simulation.md)
