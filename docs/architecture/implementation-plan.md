# Implementation Plan

## Status

Frozen legacy implementation roadmap.

Do not continue this sequence as the active plan. The replacement is
[`target-architecture/implementation-roadmap.md`](target-architecture/implementation-roadmap.md).

The previous roadmap is archived at
[Archived RPG Engine Roadmap](archive/implementation-plan-rpg-engine-roadmap.md).
It remains useful historical context, but this document is the active build
order.

## Direction

The project direction is now:

```text
authority-bounded social-cognitive simulation substrate
  -> configurable decision and evaluation substrate
  -> later game, scenario, and adapter layers
```

The future game layer is still important. The priority is different: build the
simulation core, actor-relative context, social-cognitive decision substrate,
and evaluation surfaces before committing to a particular playable loop.

This roadmap is intentionally thin. Detailed implementation plans belong in
phase-local files under `.codex/plans/`. Research background, benchmark
methodology, concrete APIs, and module layouts belong in the linked
architecture, design, research, and phase-local documents.

## Primary Inputs

Architecture:

- [Architecture Decisions](ADR.md)
- [Engine Architecture](engine.md)
- [Runtime Pipeline Architecture](runtime-pipeline.md)
- [Crate Boundary Architecture](crates.md)
- [Project Conventions](project-conventions.md)
- [Configurable Decision Pipeline](configurable-decision-pipeline.md)

Design:

- [Simulation Core](../design/simulation-core.md)
- [Truth, Authority, And Layer Boundaries](../design/truth-authority-and-layer-boundaries.md)
- [World Model](../design/world-model.md)
- [Causal Runtime](../design/causal-runtime.md)
- [Typed Effect Primitives](../design/typed-effect-primitives.md)
- [Standard World Library And Primitive Semantics](../design/standard-world-library.md)
- [Perception And Observation](../design/perception-and-observation.md)
- [Epistemic State](../design/epistemic-state.md)
- [Social Institutional Model](../design/social-institutional-model.md)
- [Pack Authoring And Semantic Declarations](../design/pack-authoring-and-semantic-declarations.md)
- [Intent Templates And Planning](../design/intent-templates-and-planning.md)
- [Simulation Transition Compiler](../design/simulation-transition-compiler.md)

Research:

- [Cognitive And Agent Research Map](../research/cognitive-agent-research-map.md)
- [Social Strategic Evaluation Program](../research/social-strategic-evaluation-program.md)
- [Social-Strategic Benchmark Methodology](../research/social-strategic-benchmark-methodology.md)
- [Implementation Architecture And Library Survey](../research/implementation-architecture-and-library-survey.md)
- [Runtime Pipeline Implementation Research](../research/runtime-pipeline-implementation-research.md)

## Planning Rules

Stable boundaries:

- hard truth mutates only through typed runtime execution and causal commit
- accepted non-hard state enters through accepted update gates
- actor-facing context is actor-relative, provenance-bearing, and non-mutating
- decision code produces typed artifacts, requests, or proposals; it does not
  own mutation authority
- LLMs, heuristics, rules, and oracles are pass implementation modes, not
  authority owners
- game-specific systems enter through checked definitions, semantic dialects,
  standard libraries, scenario content, or later adapter layers

Flexible choices:

- exact pass graph representation
- exact source syntax and parser
- persistence backend
- ECS, graph, Datalog, scripting, or Wasm adapters
- first benchmark suite
- first playable game scenario
- UI/editor/network architecture

Every future phase should begin with a local plan that states what it locks,
what it leaves open, and how it preserves authority boundaries.

## Completed Foundation

Phases 0-7 remain valid under the revised direction. They established the
authority, identity, definition, model, runtime, primitive, and actor-context
substrate that social-cognitive simulation needs.

| Phase | Foundation |
| --- | --- |
| 0 | Workspace, crate skeletons, dependency direction, conventions. |
| 1 | Shared ids, time, version, provenance, replay, and authority vocabulary. |
| 2 | Checked runtime-facing definitions and registry validation. |
| 3 | World model stores, event history, query surfaces, receivers, and invalidation. |
| 4 | Causal transaction waist, typed effect staging, validation, and hard commit. |
| 5 | Runtime control, scheduler drain, process state, wakeups, reservations, and time/progress boundaries. |
| 6 | Pure standard primitive definitions and trusted standard primitive semantics. |
| 7 | Actor-relative context projection, action repertoire, capability placeholders, affordances, and provenance. |

## Revised Roadmap

### Phase 8: Decision Substrate

Create the typed substrate for configurable social-cognitive decision work in
`world-decision`.

Key work:

- decision profiles
- representation roles and concrete representation kinds
- pass contracts and implementation modes
- decision artifacts and artifact references
- decision trace skeleton
- profile validation skeleton

Exit condition:

The project can define checked decision profiles and typed pass contracts over
actor-context inputs without executing a full decision pipeline or hardcoding a
single cognitive theory.

### Phase 9: Decision Pipeline Execution And Trace

Make a small, validated decision pipeline executable and traceable.

Key work:

- static profile runner
- concrete pass execution boundary
- typed pass input/output handling
- verifier result capture
- trace recording
- deterministic, nondeterministic, LLM, and oracle metadata
- selected intent, request, abstention, or non-hard proposal handoff

Exit condition:

The project can run the same actor-context input through small comparable
profiles and produce a decision trace that explains the produced output without
granting decision code mutation authority.

### Phase 10: Social-Cognitive Representation Slice

Add the first research-relevant representation families on top of the decision
substrate.

Key work:

- typed speech surfaces and speech-act candidates
- commitment candidates and commitment lifecycle inputs
- bounded other-model views
- strategic or motivation signals
- optional appraisal-like variables as one standard dialect
- paired ablation examples

Exit condition:

The project can express at least one meaningful social-strategic contrast, such
as direct action versus typed speech or no other-model versus bounded
other-model, with comparable typed traces.

### Phase 11: Engine Facade And Research Session Skeleton

Provide the first public orchestration layer for simulation sessions, decision
runs, runtime execution, traces, and inspection.

Key work:

- session lifecycle
- registry and library wiring
- world creation/load/save coordination
- actor context projection entry point
- decision profile execution entry point
- runtime request submission and scheduler drain
- trace and metric input export
- inspection and explanation surfaces

Exit condition:

An application or research runner can create a session, load checked content,
project actor context, run a decision profile, submit runtime requests, drain
work, inspect outcomes, and export traces through stable high-level surfaces.

### Phase 12: Scenario And Evaluation Substrate

Introduce the minimal substrate needed to run controlled social-strategic
evaluation scenarios on top of the session facade.

Key work:

- scenario family and scenario instance vocabulary
- actor specs and information partitions
- agent policy boundary
- decision profile assignment
- run configuration and run trace collection
- metric report vocabulary
- smoke, development, calibration, and held-out split vocabulary

Exit condition:

The project can run small controlled scenario instances, collect typed traces,
and compute or prepare the first outcome, process, validity, and trace-support
metrics.

### Phase 13: Authoring And Verification

Introduce source-facing authoring and diagnostics after the runtime,
definition, context, decision, session, and evaluation shapes are visible in
code.

Key work:

- pack compiler structure
- checked definition construction
- semantic dialect declarations
- decision profile declarations
- pass contract declarations
- scenario family declarations
- source-aware diagnostics
- registry and profile verification

Exit condition:

Definitions, semantic declarations, decision profiles, and scenario-facing
declarations can be authored through a checked path without leaking parser or
diagnostic dependencies into runtime authority crates.

### Phase 14: Game Layer And Adapter Planning

Plan the first concrete game/content layer and optional adapters after the
social-cognitive substrate is real enough to guide those choices.

Key work:

- first playable or scenario-facing content slice
- game-layer package strategy
- persistence backend decision
- optional ECS, graph, Datalog, scripting, or Wasm adapter decision
- UI/editor/client planning
- CI and benchmark command matrix

Exit condition:

The project has enough implemented substrate to choose a game/content layer and
adapter strategy based on real authority, context, decision, and evaluation
code rather than speculative engine boundaries.

## Global Exit Criteria

This roadmap has served its purpose when:

- hard mutation authority remains centralized in runtime code
- actor context and decision code cannot bypass runtime authority
- accepted non-hard state remains gated and inspectable
- configurable decision profiles can be validated, executed, traced, and
  compared
- social-cognitive representations remain typed artifacts rather than hidden
  prompt state
- scenario runs can support paired profile comparisons with leakage checks
- authoring produces checked declarations without granting mutation authority
- the engine facade can orchestrate sessions without collapsing crate
  boundaries
- a later game layer can build on the same substrate without replacing the
  research/evaluation architecture

## Deferred Beyond This Roadmap

- exact source language syntax
- exact persistence backend and migration format
- final ECS, graph, Datalog, scripting, or Wasm adapter
- first public benchmark release
- first playable game
- UI/editor/client architecture
- multiplayer/network authority
- hosted evaluation service
- production performance budget
- release packaging

## Summary

The revised implementation order is:

```text
completed foundation:
  workspace
  -> core vocabulary
  -> checked definitions
  -> world model and query surfaces
  -> causal mutation waist
  -> runtime control, time, and process
  -> standard world library and primitive semantics
  -> actor context

revised roadmap:
  decision substrate
  -> decision pipeline execution and trace
  -> social-cognitive representation slice
  -> engine facade and research session skeleton
  -> scenario and evaluation substrate
  -> authoring and verification
  -> game layer and adapter planning
```

The revised order keeps the completed engine substrate, but moves future work
toward a stronger social-cognitive simulation and evaluation foundation before
committing to a particular game layer.
