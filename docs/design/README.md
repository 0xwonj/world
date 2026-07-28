# Design Document Index

## Role

Design documents describe product pressure, domain models, and subsystem
possibilities in more detail than the normative architecture. They are inputs,
not a second architecture.

For authority, lifecycle, time, persistence, extension, and package boundaries,
the [Target Architecture Package](../architecture/target-architecture/README.md)
wins. A design construct becomes an implementation requirement only after it
has a normative owner, validation scenario, and roadmap gate.

## Current direction

| Document | Status | Normative routing |
|---|---|---|
| [Reference Game Vision](reference-game-vision.md) | Current product and validation direction | Target roadmap and coverage |
| [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md) | Partially superseded design principle | System architecture; extensibility |
| [Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md) | Reconciled domain input | Context and agency; extensibility |
| [Perception And Observation](perception-and-observation.md) | Partially superseded domain input | Cognition and agency |
| [Epistemic State](epistemic-state.md) | Partially superseded domain input | Cognition and agency; formal state |
| [Social Institutional Model](social-institutional-model.md) | Active domain exploration | Cognition and agency; social gate |
| [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md) | Active evaluator-internal exploration | Cognition and agency |
| [Intent Templates And Planning](intent-templates-and-planning.md) | Partially superseded domain input | Cognition and agency |
| [Multi-Resolution Simulation](multi-resolution-simulation.md) | Partially superseded domain input | Runtime, persistence, and scale |
| [Physical Simulation Grammar](physical-simulation-grammar.md) | Active gameplay-domain exploration | System architecture; gameplay composition gate |
| [Standard World Library](standard-world-library.md) | Partially superseded design principle | Code architecture; extensibility |
| [Typed Effect Primitives](typed-effect-primitives.md) | Partially superseded vocabulary input | Extensibility; owner-specific preparation |
| [World Model](world-model.md) | Partially superseded domain input | Formal and system state owners |

“Active” here means useful for continued domain design. It does not mean
normative.

## Frozen pre-target models

The following files retain historical reasoning and useful examples, but their
cross-system contracts have been replaced. Do not extend or implement them as
the current architecture.

| Document | Replaced by |
|---|---|
| [Simulation Core](simulation-core.md) | Target package as a whole |
| [Simulation Transition Compiler](simulation-transition-compiler.md) | Formal model, system architecture, and family-specific compiler contracts |
| [Causal Runtime](causal-runtime.md) | Runtime/persistence model and runtime code architecture |
| [Time Model](time-model.md) | Normative superdense-time scheduler |
| [Pack Authoring And Semantic Declarations](pack-authoring-and-semantic-declarations.md) | Tiered, family-specific extensibility architecture |
| [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md) | Formal state partitions and typed gates |
| [Action And Event Model](action-event-model.md) | Authority records, domain events, actions, and processes in the target package |

These files remain in place because research and idea notes cite them as
historical inputs. Their status headers and this index prevent those links from
creating a second active architecture.

Partially superseded documents may likewise contain older names such as
`ActionRequest`, `CausalTransaction`, or `EventRecord`. Their domain examples
remain useful, but those names do not override the current grounded-candidate,
prepared-transition, domain-event, and authority-record contracts.

## Design rule

A subsystem may be arbitrarily rich internally while preserving a narrow outer
contract. Prefer concrete domain types and local algorithms. Introduce a shared
abstraction only after at least one vertical scenario supplies its producer,
consumer, invariant, failure model, and evidence that the abstraction belongs
across a boundary.
