# Target Architecture Execution Roadmap

## Status and purpose

This document owns the stable implementation sequence for replacing the
current code with the target architecture.

It deliberately specifies complete outcomes and objective gates without
freezing method-level plans for distant work. Detailed planning is rolling:
only the active milestone receives executable work packages. At each milestone
boundary, implementation evidence is reviewed before the next detailed plan is
accepted.

The normative architecture remains in the other documents in this package.
Operational status and active milestone plans live under
[`docs/implementation/target-rewrite/`](../../implementation/target-rewrite/README.md).

## Execution policy

### Clean replacement

The repository has no compatibility obligation to the current internal APIs or
formats. The rewrite introduces no:

- compatibility or legacy production module;
- deprecated alias for replaced types;
- old/new feature switch;
- current-format checkpoint or artifact importer;
- wrapper around `WorldModel`, `CausalRuntime`, or the generic decision runner;
- selectable dual authority pipeline.

Reusable algorithms and invariant tests are rewritten under their target
owners. The old implementation is retained only on the verified preservation
branch and in Git history.

### Rolling detail

The execution model has three planning horizons:

1. **Architecture** fixes authority, ownership, dependency direction, formal
   invariants, and final product boundaries.
2. **Roadmap** fixes milestone order, outcome, dependencies, and exit gates.
3. **Active milestone plan** fixes the next concrete work packages and may
   adapt local implementation choices as evidence arrives.

Future milestones must remain coarse. A milestone plan may not override a
normative architecture boundary. If implementation evidence requires changing
authority, package dependency direction, persistence ownership, or another
cross-system contract, work stops for an explicit architecture decision.

### Vertical delivery

Milestones deliver executable causal slices, not disconnected type catalogs.
A shared type is introduced only with a real producer, consumer, invariant,
and test. Empty placeholder crates and speculative public traits are forbidden.

The first target-state merge is intentionally larger than later merges:
Milestone 1 must contain both immutable definition/artifact foundations and one
minimal authoritative interaction. The target branch does not merge a state
that retains the old authority path alongside the new one.

## Milestone map

```mermaid
flowchart LR
    M0["M0<br/>Preservation and baseline"]
    M1["M1<br/>First authoritative slice"]
    M2["M2<br/>Deterministic kernel"]
    M3["M3<br/>Grounded action"]
    M4["M4<br/>Agency lifecycles"]
    M5["M5<br/>Durable execution"]
    M6["M6<br/>Product and research"]
    M7["M7<br/>Scale and optional semantics"]

    M0 --> M1 --> M2 --> M3 --> M4 --> M5 --> M6 --> M7
```

## M0: Preservation and clean rewrite baseline

### Outcome

The complete pre-redesign workspace is recoverable from a verified commit, the
rewrite proceeds on a clean branch from the original baseline, the target
architecture is tracked, and the first executable milestone has frozen inputs
and a detailed plan.

### Exit gate

- the preservation branch names an explicit verified commit;
- all selected tracked and untracked files exist in that commit;
- ignored build output is absent;
- the rewrite branch starts from the intended base and contains no uncommitted
  legacy implementation work;
- the target architecture and execution documents are tracked;
- baseline build, dependency, and superseded-symbol evidence is recorded;
- canonical identity protocol, first standard interaction, and minimum
  source/artifact surface are selected for M1;
- the detailed M1 plan defines work packages, deletion scope, decision
  triggers, and binary acceptance gates.

## M1: Immutable packs through one authoritative interaction

### Outcome

One standard controller request compiles from exact pack artifacts, resolves
into immutable execution semantics, and produces one atomic authoritative
transition through the new engine/runtime facade. The old authority, context,
and generic decision paths are absent from the merged target state.

### Major capabilities

- canonical identity and version primitives;
- minimal checked action, condition, effect, and event definitions;
- verified pack artifact, exact lock, definition-set linking, and activation;
- private runtime session head and in-memory atomic repository;
- `Admit`, staged `Fire`, and minimal `Manage`;
- sealed authority record, cursor, publication receipt, and attempt control;
- sealed `ResolvedExecution`;
- `Engine`, non-cloneable `RunAttempt`, and read-only `WorldSession`;
- one trusted standard primitive and one inspector query;
- dependency, privacy, canonicalization, and vertical conformance tests.

### Exit gate

- every world change is exactly one `Admit`, `Fire`, or `Manage`;
- state, scheduler, history, cursor, and receipt publish atomically;
- no external package can construct or replace the session head;
- artifact loading and linking fail closed on invalid identity or closure;
- repeated execution yields identical semantic fingerprints;
- no replaced public symbol or forbidden dependency edge remains;
- validation scenarios 13 and 18 pass;
- the runtime-authority portion of scenario 1 passes.

### Deferred

Actor-relative policy, rich process protocols, durable restoration, a database,
textual DSL design, CLI product work, and optional evaluators.

## M2: Complete deterministic runtime protocol

### Outcome

The minimal kernel becomes the complete deterministic authority protocol for
ingress, moments, management, conflicts, deduplication, causal routing, bounded
work, termination, and attempt reconciliation.

### Major capabilities

- complete authority-record families and identity rules;
- admission frontier and typed request ledgers;
- same-moment preparation from one base snapshot;
- explicit footprints and total deterministic conflict resolution;
- rejection-only valid fallback;
- complete reaction and lifecycle-control records;
- complete attempt reservation, disposition, receipt, cancellation, and
  reconciliation state machines;
- deterministic budgets, keyed randomness, and property-based invariant tests.

### Exit gate

- order, worker count, and collection representation cannot alter results;
- duplicate or retired request identities cannot create another effect;
- every admitted logical command has one durable outcome;
- invalid bindings, receipts, successors, and termination reads fail closed;
- validation scenarios 1 and 10 pass;
- kernel portions of scenarios 14, 15, and 19 pass.

## M3: Actor-relative context and grounded action

### Outcome

An actor chooses only from actor-visible, actually bindable action candidates,
while all freshness, authority, and lowering data remains private to the
trusted coordinator.

### Major capabilities

- lifecycle-specific actor-relative projections;
- explicit valid-empty and unavailable results;
- capability, affordance, and visibility projections;
- grounded candidate generation and private resolution tables;
- durable one-shot action opportunities;
- deterministic baseline `ActionPolicy`;
- private selected-candidate lowering and runtime revalidation;
- neutral attempt-resolution wakes and hidden-state noninterference tests.

### Exit gate

- policy cannot invent a definition, binding, or runtime command;
- hidden-only state cannot change actor-visible payloads or logical invocation
  timing;
- every opportunity reaches one typed terminal disposition or bounded,
  causally linked successor;
- validation scenarios 1, 2, 4, and 16 pass.

## M4: Independently scheduled agency lifecycles

### Outcome

Evidence assimilation, appraisal, optional social interpretation, intent,
activity, and action operate as distinct typed lifecycles with independent
cadence and explicit durable protocols.

### Major capabilities

- post-commit routing and coalescing generations;
- accepted evidence and belief transition;
- deterministic appraisal and intent baselines;
- persistent intent and versioned activity state machines;
- activity initialization and advancement;
- deferred evaluator invocation, freshness, cancellation, and failure;
- process instances and grounded actor-initiated control;
- normalized cross-lifecycle trace links.

### Exit gate

- no lifecycle recursively executes the full stack;
- intent, activity, action opportunity, and process remain distinct;
- basic actors operate without planning or rich appraisal;
- deferred work cannot be skipped by admission sealing;
- validation scenarios 2, 3, 5, 11, and 12 pass.

## M5: Checkpoint, restore, replay, branch, and delivery durability

### Outcome

A controlled attempt and its exact semantic closure survive interruption,
restore without re-executing external computation, support verification, and
produce read-only portable archives or explicit child epochs.

### Major capabilities

- durable checkpoint and typed history tail;
- exact artifact-closure manifests and retention;
- two-stage same-domain attempt restoration;
- verification replay and first-divergence diagnostics;
- immutable child lineage and offline target-schema migration boundary;
- reliable-delivery history leases and archive generation fencing;
- portable read-only archive import;
- crash-safe finalization, artifact handoff, discard, and compaction.

### Exit gate

- restoration invokes no evaluator or external service;
- active and reserved attempts retain their exact execution closure;
- portable data cannot create a second writer or delivery owner;
- incompatible definitions and schemas fail closed;
- verification identifies the first divergence;
- validation scenarios 5, 6, 8, 14, 15, and 19 pass.

An actual checkpoint transform is implemented only after a second supported
target checkpoint schema exists.

## M6: CLI, experiment, and inspection product

### Outcome

The architecture is usable as a headless engine and research product through
stable composition, scenario, run, trace, metric, comparison, and explanation
surfaces.

### Major capabilities

- `world-cli` composition root;
- `world-lab` scenario and experiment artifacts;
- deterministic run-case expansion and parallel independent execution;
- immutable run results and trajectory-bound reuse;
- normalized decision/provenance trace DAG;
- recomputable metrics and analysis manifests;
- pack checking, running, replay audit, experiment, and explanation commands.

### Exit gate

- one lifecycle can be replaced while others remain fixed;
- paired runs share declared exogenous inputs;
- metrics and telemetry cannot affect simulation behavior;
- run reuse requires exact trajectory identity and sufficient capture;
- validation scenario 9 passes.

## M7: Scale and evidence-gated extensions

### Outcome

The engine supports individual background resolution and only those optional
semantic implementations justified by concrete scenarios and measurements.

### Major capabilities

- mutually exclusive detailed/background/dormant entity representations;
- checked promotion and demotion;
- background scheduling with fidelity evidence and bounded hysteresis;
- optional planner, learned, remote, or Wasm evaluator implementations behind
  existing lifecycle ports.

### Exit gate

- resolution tiers never create double authority;
- hard process and resource invariants survive transitions;
- optional evaluators pass the same contracts as the deterministic baseline;
- replay never silently invokes external computation;
- validation scenarios 7 and 17 pass.

Population aggregation, database persistence, a server/editor, dynamic package
registry, intra-world parallel commit, and distributed simulation require
separate evidence-backed roadmap decisions.

## Validation scenario allocation

| Milestone | Primary scenarios |
|---|---|
| M1 | 13, 18, runtime-authority portion of 1 |
| M2 | 1, 10, kernel portions of 14, 15, 19 |
| M3 | 1, 2, 4, 16 |
| M4 | 2, 3, 5, 11, 12 |
| M5 | 5, 6, 8, 14, 15, 19 |
| M6 | 9 |
| M7 | 7, 17 |

Before the redesign is declared operational, every validation scenario must
have executable coverage or a documented later-scope justification.

## Gate applied to every milestone

For the contracts introduced by the milestone:

- formatting, workspace compilation, lints, tests, and whitespace checks pass;
- the exact direct dependency allowlist passes;
- focused unit, negative, state-machine, and compile-fail privacy tests pass;
- durable values round-trip through untrusted decoding and reverification;
- canonical identities and repeated-run fingerprints are stable;
- every concrete authoritative head refines a valid formal `Σ`;
- every concrete world publication maps to `Admit`, `Fire`, or `Manage`;
- documentation, public API, and implementation ownership agree;
- no forbidden dependency or superseded symbol has been introduced.

Completion evidence is recorded in the milestone plan before the next
milestone becomes active.
