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

At most one milestone is active. A milestone boundary may have no active
implementation plan while the completed evidence is reviewed and the next
plan is accepted.

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

**Active milestone:** none.

**Next candidate:** M5 — Checkpoint, restore, replay, branch, and delivery
durability. Its detailed plan has not yet been researched, reviewed, or
accepted.

| Milestone | Status | Plan |
|---|---|---|
| M0 — Preservation and baseline | Complete | [Plan and evidence](milestone-00-preservation-and-baseline.md) |
| M1 — First authoritative slice | Complete and exit-reviewed | [Plan and evidence](milestone-01-authoritative-vertical-slice.md), [exit review](milestone-01-exit-review.md) |
| M2 — Deterministic kernel | Complete and exit-reviewed | [Plan and evidence](milestone-02-deterministic-kernel.md), [research](../../research/m2-deterministic-kernel-research.md), [exit review](milestone-02-exit-review.md) |
| M3 — Grounded action | Complete and exit-reviewed | [Plan and evidence](milestone-03-grounded-action.md), [research](../../research/m3-grounded-action-research.md), [exit review](milestone-03-exit-review.md) |
| M4 — Agency lifecycles | Complete and exit-reviewed | [Plan and evidence](milestone-04-agency-lifecycles.md), [research](../../research/m4-agency-lifecycles-research.md), [exit review](milestone-04-exit-review.md) |
| M5–M8 | Roadmap only | [Roadmap](../../architecture/target-architecture/implementation-roadmap.md) |

M1–M7 establish and operationalize the engine foundations in dependency
order. The target roadmap is complete only after M8 has falsified the
gameplay-composition boundaries with the mandatory capability evidence and
named validation scenarios in the roadmap. M8 is not a request for broad
content or a generic framework; it is the first evidence point at which
gameplay-facing composition APIs may be stabilized.

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
