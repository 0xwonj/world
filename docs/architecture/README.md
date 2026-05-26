# Architecture

## Status

Architecture planning.

## Purpose

This folder turns the research and design documents into an implementation
architecture shape.

It does not replace:

- `docs/research/`, which records why options were selected or rejected
- `docs/design/`, which owns subsystem concepts, boundaries, and terminology
- future implementation plans, which will own concrete tasks and code work

## Documents

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

Recommended reading order:

```text
roadmap.md -> ADR.md -> engine.md -> runtime-pipeline.md -> crates.md -> project-conventions.md -> implementation-plan.md
```

Planned later:

- detailed phase-local plans as implementation begins.
