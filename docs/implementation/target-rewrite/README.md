# Target Rewrite Execution

## Purpose

This directory records operational plans and completion evidence for the
target-architecture rewrite.

The stable milestone sequence is
[`implementation-roadmap.md`](../../architecture/target-architecture/implementation-roadmap.md).
The normative system contracts remain under
[`target-architecture/`](../../architecture/target-architecture/README.md).
An execution plan can refine work inside those contracts but cannot redefine
authority, ownership, or dependency direction.

## Planning model

Exactly one milestone is active.

- The active milestone has concrete work packages, deletion scope, decision
  triggers, and acceptance commands.
- The next milestone may have a draft plan sufficient to expose dependencies.
- Later milestones remain outcome-and-gate descriptions in the roadmap.
- Local implementation choices may change as tests expose better methods.
- Cross-system boundary changes require an explicit architecture decision
  before implementation proceeds.

At milestone close:

1. record exact completion evidence;
2. compare implementation against the formal and physical architecture;
3. record any assumption that changed;
4. update the coarse roadmap only when the intended outcome or order changed;
5. accept the next milestone plan;
6. mark only that milestone active.

## Status

| Milestone | Status | Plan |
|---|---|---|
| M0 — Preservation and baseline | Complete | [Plan and evidence](milestone-00-preservation-and-baseline.md) |
| M1 — First authoritative slice | Active | [Plan](milestone-01-authoritative-vertical-slice.md) |
| M2–M7 | Roadmap only | [Roadmap](../../architecture/target-architecture/implementation-roadmap.md) |

## Plan template

Each detailed plan uses:

```text
Status
Goal
Non-goals
Normative contracts
Current-state evidence
Decisions fixed for this milestone
Work packages
Deletion scope
Decision triggers
Acceptance gates
Completion evidence
Next milestone handoff
```

Plans describe repository work, not code vocabulary. Project-management terms
must not appear in production type names, comments, diagnostics, or tests.
