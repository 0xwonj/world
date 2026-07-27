# Architecture

## Status

The greenfield target architecture is now authoritative under
[`target-architecture/`](target-architecture/README.md).

The earlier files listed below predate that redesign. They remain valuable as
detailed inputs and implementation history, but where they conflict with the
target package, the target package wins. Reconciliation and archival should
happen incrementally as implementation moves to the new contracts.

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
- [Cognition and Agency Lifecycles](target-architecture/cognition-and-agency.md):
  actor-relative context and the separate appraisal, intent, activity, action,
  and process contracts.
- [Runtime, Persistence, and Scale](target-architecture/runtime-persistence-and-scale.md):
  deterministic time, scheduling, commit, checkpoints, replay, randomness,
  multi-resolution, and scaling.
- [Extensibility and Research](target-architecture/extensibility-and-research.md): pack
  artifacts, executable definition families, extension trust, experiments,
  trace, and metrics.
- [Architecture Decisions](target-architecture/decisions.md): accepted target decisions.
- [Validation Scenarios](target-architecture/validation-scenarios.md): adversarial
  architecture acceptance cases.
- [Implementation Roadmap](target-architecture/implementation-roadmap.md):
  stable milestone order, outcomes, and exit gates.
- [Legacy Reconciliation](target-architecture/legacy-reconciliation.md):
  adopted, refined, and replaced earlier architecture and implementation.

Research support:

- [Architecture Redesign Research Synthesis](../research/architecture-redesign-synthesis.md)

Operational execution status and rolling milestone plans live under
[`docs/implementation/target-rewrite/`](../implementation/target-rewrite/README.md).

## Earlier architecture documents

- [Architecture Roadmap](roadmap.md): dependency order for stabilizing the
  architecture before crate design or implementation planning.
- [Architecture Decisions](ADR.md): compact decision notes for why the current
  architecture shape was selected.
- [Engine Architecture](engine.md): logical runtime components, ownership, and
  dependency direction before crate boundaries are chosen.
- [Runtime Pipeline Architecture](runtime-pipeline.md): request, process,
  effect, commit, observation, semantic, and resolution flow through the
  logical component structure.
- [Crate Boundary Architecture](crates.md): candidate Rust workspace and crate
  boundaries derived from authority ownership and dependency direction.
- [Project Conventions](project-conventions.md): stable Rust workspace,
  dependency, ID, error, diagnostics, serialization, async, and accelerator
  policies that should be fixed before implementation details.
- [Implementation Plan](implementation-plan.md): high-level phased build order,
  implementation principles, and phase exit conditions.
- [Implementation Execution Contract](implementation-execution-contract.md):
  mandatory agent workflow, public API guardrails, review gates, and phase
  completion checks for long-running implementation runs.

Earlier reading order:

```text
roadmap.md -> ADR.md -> engine.md -> runtime-pipeline.md -> crates.md -> project-conventions.md -> implementation-plan.md -> implementation-execution-contract.md
```

These documents should not be extended with new target-level decisions without
first checking the normative package.
