# Architecture

## Status

The target architecture is the only active cross-system architecture:
[`target-architecture/`](target-architecture/README.md).

The other Markdown files directly in this directory predate the redesign.
They are frozen historical inputs retained at stable paths because older
research and design notes cite them. They must not be continued as plans or
implemented as current contracts. Their dispositions are recorded in
[Legacy Reconciliation](target-architecture/legacy-reconciliation.md).

Start from the repository-wide [Documentation Guide](../README.md), not from a
legacy file reached through an old link.

## Purpose

This folder turns the research and design documents into an implementation
architecture shape.

The documentation responsibilities are:

- `docs/research/` records why options were selected or rejected;
- `docs/design/` retains detailed subsystem concepts and terminology unless
  the target package supersedes them;
- `docs/architecture/target-architecture/` owns the target's cross-system
  boundaries and decisions;
- implementation plans own concrete code tasks and verification work.

## Target architecture

- [Target Architecture](target-architecture/README.md): status, scope, thesis, and reading
  order for the normative redesign.
- [System Architecture](target-architecture/system-architecture.md): authority, ownership,
  component structure, dependency direction, and public surfaces.
- [Formal System Model](target-architecture/formal-model.md): minimal state,
  transition, subsystem, compiler, determinism, and refinement model.
- [Target Rust Code Architecture](target-architecture/code-architecture.md):
  physical crate/module ownership, visibility, public APIs, and clean-rewrite
  cuts.
- [Cognition and Agency Lifecycles](target-architecture/cognition-and-agency.md):
  actor-relative context and the separate appraisal, intent, activity, action,
  and process contracts.
- [Runtime, Persistence, and Scale](target-architecture/runtime-persistence-and-scale.md):
  deterministic time, scheduling, commit, checkpoints, replay, randomness,
  multi-resolution, and scaling.
- [Extensibility and Research](target-architecture/extensibility-and-research.md): pack
  artifacts, executable definition families, extension trust, experiments,
  trace, and metrics.
- [ArtifactBlobV1 Protocol](target-architecture/artifact-blob-v1.md):
  byte-complete foundation compiled-pack appendix.
- [Architecture Decisions](target-architecture/decisions.md): accepted target decisions.
- [Validation Scenarios](target-architecture/validation-scenarios.md): adversarial
  architecture acceptance cases.
- [Implementation Roadmap](target-architecture/implementation-roadmap.md):
  stable milestone order, outcomes, and exit gates.
- [Legacy Reconciliation](target-architecture/legacy-reconciliation.md):
  adopted, refined, and replaced earlier architecture and implementation.

Primary research support:

- [Architecture Redesign Research Synthesis](../research/architecture-redesign-synthesis.md)
- [Gameplay Composition And Evolution Research](../research/gameplay-composition-and-evolution-research.md)

Operational execution status and rolling milestone plans live under
[`docs/implementation/target-rewrite/`](../implementation/target-rewrite/README.md).

## Frozen pre-target documents

- [Architecture Roadmap](roadmap.md)
- [Architecture Decisions](ADR.md)
- [Engine Architecture](engine.md)
- [Runtime Pipeline Architecture](runtime-pipeline.md)
- [Crate Boundary Architecture](crates.md)
- [Configurable Decision Pipeline](configurable-decision-pipeline.md)
- [Implementation Plan](implementation-plan.md)
- [Implementation Execution Contract](implementation-execution-contract.md)
- [Project Conventions](project-conventions.md)

Do not use these as a reading sequence. Their status headers route to the
current owner. New target-level decisions belong only in the normative package.
