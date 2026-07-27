# Implementation Execution Contract

## Status

Frozen legacy execution contract.

It is no longer mandatory for the redesign. Repository working rules remain in
`AGENTS.md`; the active replacement sequence and gates are in
[`target-architecture/implementation-roadmap.md`](target-architecture/implementation-roadmap.md).

## Purpose

This document defines how implementation agents should carry the current
architecture into code.

It is not a replacement for the architecture or design documents. It does not
redesign the engine. It records execution rules, review gates, public API
guardrails, and phase completion criteria that protect the architecture while
the Rust implementation is still young.

The central goal is to prevent an implementation from appearing complete while
leaving authority boundaries open through public APIs.

## Source Of Truth

Agents must read the relevant architecture and design documents before writing
phase code. Use this document as an execution policy, not as the only source of
architecture.

Primary architecture inputs:

- [Architecture Decisions](ADR.md): stable decisions and rejected directions.
- [Engine Architecture](engine.md): logical runtime ownership and component
  shape.
- [Runtime Pipeline Architecture](runtime-pipeline.md): intent, activity,
  action, effect, transaction, process, runtime-control, and observation flow.
- [Crate Boundary Architecture](crates.md): dependency direction and authority
  ownership by crate.
- [Project Conventions](project-conventions.md): workspace, dependency, ID,
  error, async, and accelerator policy.
- [Implementation Plan](implementation-plan.md): build order, phase goals, and
  phase exit conditions.

Primary design inputs:

- [Simulation Core](../design/simulation-core.md): simulation-first engine
  principles.
- [World Model](../design/world-model.md): materialized state, query layers,
  mutation boundary, and actor-relative visibility.
- [Causal Runtime](../design/causal-runtime.md): `ActionRequest`,
  `CausalTransaction`, staged effects, event records, process state, and
  permissions.
- [Typed Effect Primitives](../design/typed-effect-primitives.md): effect
  vocabulary and mutation authority.
- [Standard World Library And Primitive Semantics](../design/standard-world-library.md):
  reusable primitive definitions and trusted semantics outside runtime core.
- [Time Model](../design/time-model.md): simulation time and scheduler
  semantics.
- [Simulation Transition Compiler](../design/simulation-transition-compiler.md):
  transition lowering concepts and boundaries.

Primary research inputs:

- [Implementation Architecture And Library Survey](../research/implementation-architecture-and-library-survey.md)
- [Runtime Pipeline Implementation Research](../research/runtime-pipeline-implementation-research.md)

## Startup Protocol

Every implementation run must start with repository orientation.

1. Run `git status --short`.
2. Identify dirty files before editing.
3. Do not build on failed or unrelated dirty implementation work.
4. Do not revert unrelated user work.
5. If previous failed implementation changes are present in core runtime crates,
   either preserve them as a patch before restoring them or stop and ask.
6. Read `AGENTS.md` and this document before editing.

If a run begins from a post-Phase-0 workspace, the intended baseline is a clean
workspace skeleton and architecture documents, not a partially trusted runtime
implementation.

## Non-Negotiable Boundaries

These boundaries must be preserved in code, not only in documentation.

- Hard mutation must go through `CausalTransaction`.
- Runtime control state must go through validated
  `RuntimeControlUpdate` / `AcceptedRuntimeControlUpdate` style boundaries.
- `Intent` is the commitment boundary.
- `Activity` is the temporal execution boundary for selected intent lowering
  and ongoing actor-facing work.
- `ActionRequest` is the concrete actor-owned attempt boundary.
- `ProcessInstance` is durable progress and execution state, not hidden action
  spam.
- Decision, appraisal, planning, and semantic context code must not gain hard
  mutation authority.
- Standard primitive semantics must execute through runtime staging
  capabilities, not direct store mutation.
- Query APIs must be read-only.
- Actor-relative APIs must not leak privileged hard truth while labeling it as
  actor truth.

## Public API Guardrails

Early Rust code must use visibility to enforce authority where Rust can
represent the boundary, and must document and test unavoidable cross-crate
authority exceptions.

- Do not expose public constructors, public fields, or public enum variants that
  allow downstream crates to forge committed hard-state packages unless the
  constructor is an explicit accepted-package boundary for another engine crate.
- Do not expose public constructors, public fields, or public enum variants that
  allow downstream crates to forge accepted runtime-control updates unless the
  constructor is an explicit accepted-package boundary for another engine crate.
- If a type has invariants, prefer private fields plus constructors and
  accessors.
- Cross-crate authority cannot rely on friend visibility. If one crate should be
  the intended producer of a value consumed by another crate, enforce as much as
  Rust allows with private fields, narrow constructors, and higher-level facade
  ownership. If a public constructor is unavoidable because the producer lives
  in another crate, add rustdoc that names the intended producer and protect the
  call surface with source allowlist tests.
- `world-model` may own storage and atomic application of accepted updates, but
  model receiver methods are not a general mutation authority. They should
  validate accepted-package shape and storage invariants before mutation.
- `world-model` public APIs should remain read-first outside accepted-package
  receiver methods: model creation, read-only stores, read labels, query
  surfaces, and accessors. Do not add direct store mutators as a substitute for
  runtime authority.
- `world-runtime` may interpret checked definitions and produce accepted
  transaction or runtime-control outputs through narrow APIs.
- Effect handlers should receive staging capabilities, not raw store mutation
  authority.
- Decision and appraisal code should receive query/context capabilities, not
  mutation gates.
- Public APIs should be reviewed by asking: which crate should call this, and
  what authority does this expose?
- Avoid broad framework traits until a second concrete implementation needs the
  abstraction.

## Known Failure Modes To Prevent

The following patterns are architecture failures, even if the workspace
compiles and tests pass.

- A caller can construct hard-state mutations and a committed transaction
  package without passing through the causal runtime path.
- A caller can construct arbitrary runtime-control mutations and apply them
  without validation.
- `StagePermission` is stored on definitions but not enforced by execution.
- Later effects in one transaction cannot see earlier staged changes.
- Event contracts are checked only by matching event kind names.
- `ActorOwnsRole` only checks that a role exists.
- `ProcessMustSupportResolution` is exposed but ignored.
- Scheduler wakeups are consumed before the target work is accepted or
  recoverably handled.
- Actor-relative query surfaces return omniscient hard state as actor truth.
- `ProcessInstance` becomes a source of hidden concrete action spam instead of
  durable progress state.
- Public fields allow checked definitions, process progress, or event contracts
  to diverge from constructor validation.

## Agent Execution Workflow

For each implementation phase, follow the workflow below before continuing to
the next phase.

1. Research and context review.

   Read the relevant architecture, design, and research documents. Extract the
   contracts this phase must preserve. Use external research only when it
   materially improves a concrete implementation choice.

2. Local phase plan.

   Before writing code, briefly state:

   - what this phase should lock,
   - what remains intentionally open,
   - what adjacent concepts must be co-designed,
   - what is out of scope,
   - what tests will prove the important boundaries.

3. Implementation.

   Keep code minimal, concrete, and domain-shaped. Preserve crate dependency
   direction. Prefer narrow APIs over broad framework traits.

4. Verification.

   Run focused checks while developing and workspace checks before advancing.
   Add tests where they protect behavior, authority boundaries, or cross-crate
   contracts.

5. Review.

   Review in code-review style with findings first. Cover:

   - architecture boundary mismatches,
   - dependency direction,
   - public API bypass risks,
   - Rust API and invariant quality,
   - runtime correctness,
   - process and runtime-control semantics,
   - missing tests,
   - complexity and over-abstraction.

6. Fix.

   Fix real issues before phase handoff. Do not defer P0 or P1 authority
   boundary issues and still call the phase complete.

7. Phase handoff.

   Leave a concise handoff note before moving on.

Phase boundaries guide order. They are not rigid walls. Adjacent concepts may
be co-designed when that avoids artificial scaffolding, but co-design must not
skip the workflow gates.

Use subagents when available and useful, especially for architecture alignment,
Rust API surface review, runtime correctness, and test coverage.

## Review Severity

Use these severities for phase reviews.

- P0: The implementation permits a direct architecture violation or invalid
  authority bypass.
- P1: The implementation cannot satisfy the current phase exit criteria without
  fixing the issue.
- P2: The implementation is directionally valid but has weak API shape,
  incomplete validation, or missing important tests.
- P3: Cleanup, naming, organization, or documentation improvements.

Do not mark a phase complete with unresolved P0 or P1 findings.

## Phase Gates

Phase completion should be judged by enforceable behavior and API shape, not by
whether a type with the right name exists.

### Phase 1: Core Domain Substrate

Complete only when:

- later crates can use core identity, time, provenance, version, replay, and
  authority vocabulary without inventing local substitutes;
- durable identities and runtime handles remain distinct;
- constructors preserve invariants for non-trivial values;
- no broad framework trait or accelerator dependency becomes part of the core
  API.

### Phase 2: Checked Definition Model

Complete only when:

- runtime-facing definitions are separate from source syntax and authoring
  diagnostics;
- checked definitions cannot be trivially forged into inconsistent states
  through public fields;
- typed effect programs expose enough structure for later permission and event
  contract enforcement;
- definition lookup is possible without parser, renderer, scripting, async, or
  persistence dependencies.

### Phase 3: World Model And Query Surfaces

Complete only when:

- `WorldModel` can hold current state, committed history, runtime-control
  state, and read-only query surfaces;
- hard, non-hard, actor-relative, and runtime-control state remain distinct;
- arbitrary hard mutation is not exposed through model stores;
- query APIs are read-only;
- actor-relative query surfaces do not mislabel privileged hard truth;
- runtime-control storage and read surfaces exist without forcing Phase 5
  update validation or scheduler semantics forward;
- invalidation package vocabulary and derived-view staleness states exist
  without requiring final cache policy;
- public writes are intentionally absent until accepted package construction is
  owned by the proper runtime or engine facade;
- any later model-side apply surface is a narrow receiver for accepted packages,
  not a public causal transaction authority.

### Phase 4: Causal Mutation Waist

Complete only when:

- there is one visible hard-mutation waist;
- callers cannot commit hard world changes by directly mutating stores or
  forging committed packages;
- hard mutation is staged through causal transaction machinery;
- later effects can observe earlier staged effects in the same transaction;
- stage permissions are enforced;
- event contracts are checked against the action definition contract;
- successful hard commits append event history and publish invalidation through
  one accepted path.

### Phase 5: Runtime Control, Time, And Process

Complete only when:

- runtime-control state changes pass through validated update and accepted
  update boundaries;
- arbitrary runtime-control mutations cannot be forged and applied by general
  callers;
- scheduled wakeups are not lost when target work fails, blocks, or aborts;
- scheduler drains have ordering, provenance, and guard surfaces;
- `ProcessInstance` represents durable progress and execution state;
- process work can explain continued, paused, failed, completed, or interrupted
  outcomes;
- reservations, interruption, resume, completion, and failure state are
  represented minimally but concretely.

### Phase 6: Standard World Library And Primitive Semantics

Complete only when:

- primitive definitions and primitive semantics are visibly distinct;
- `world-runtime` owns registry lookup and staging capabilities without
  depending on standard primitive vocabulary;
- standard primitive definitions and trusted semantics are installed from the
  standard world library layer or a trusted extension;
- ordinary packs compose installed primitives rather than receiving raw staging
  callbacks;
- missing primitive semantics fails clearly at load or execution time;
- actor-context code can consume pure standard vocabulary without depending on
  runtime semantics installers.

## Required Tests

Add targeted tests where the corresponding behavior exists.

At minimum, the Phase 1-6 implementation should include coverage for:

- core value constructors and ordering where invariants exist;
- checked definition construction and rejected inconsistent definitions;
- query surfaces remaining read-only;
- staged effects seeing earlier staged changes;
- stage permission enforcement;
- event contract enforcement;
- actor-owned action requirement behavior;
- runtime-control update validation;
- scheduler wakeup handling on success and recoverable failure;
- process progress, pause, failure, and completion behavior;
- primitive definition/semantics registry mismatch behavior once the standard
  library layer exists;
- API privacy or compile-time unforgeability where feasible.

Tests should protect boundaries and behavior. Avoid tests that merely assert
that placeholder types can be constructed.

## Verification Commands

After meaningful Rust changes, run the relevant subset of:

```bash
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
git diff --check
```

For phase handoff after substantial implementation, run the full set unless a
known external blocker prevents it. Report any skipped check and why it was
skipped.

## Stop Conditions

Stop and ask before:

- changing crate dependency direction;
- changing core architecture boundaries;
- adding major dependencies;
- selecting a persistence backend;
- introducing ECS, async runtime, graph or Datalog systems, scripting, Wasm,
  parser, renderer, or persistence as core dependencies;
- making broad documentation rewrites unrelated to implementation;
- continuing past a phase that cannot satisfy its authority-boundary exit
  criteria.

## Phase Handoff Template

Use this shape at the end of each phase:

```text
Phase N Handoff

Context reviewed:
Implemented:
Deferred:
Tests added:
Checks run:
Review findings:
Fixes made:
Remaining risks:
Exit criteria verdict:
```

The verdict must say whether the phase truly satisfies its gate. If it does
not, do not proceed as though it does.

## Short Prompt For Future Runs

Long prompts should not duplicate this document. A future run can use a short
prompt like:

```text
Work in /Users/wonj/Projects/world.

Before implementing, read AGENTS.md,
docs/architecture/implementation-execution-contract.md, and the architecture,
design, and research documents referenced by that contract.

Carry the implementation cleanly from the post-Phase-0 workspace through
Phase 5 using the established architecture rather than redesigning it. Follow
the execution contract as mandatory policy. Do not mark any phase complete
unless its phase gate is satisfied.

Start with git status. If previous failed implementation changes are present,
do not build on them; preserve or discard them according to the execution
contract before restarting.
```
