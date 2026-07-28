# Legacy Architecture Reconciliation

## Purpose

This ledger prevents the repository from carrying two active architectures
after the redesign. It records which earlier concepts survive, which are
refined, which are replaced, and how the current implementation should be
treated.

It is a routing document, not a migration-by-type specification or a
compatibility promise. It remains useful while frozen documents stay at stable
paths. If those files are eventually removed, this ledger may move with them
to the archive; Git history remains the source of the superseded
implementation.

## Adopted

The target keeps these earlier decisions:

- a domain-owned simulation core;
- materialized current state plus causal history;
- actor-relative context rather than omniscient decisions;
- runtime-only accepted mutation authority;
- typed action, effect, event, and process boundaries;
- compiler-shaped authoring and verification;
- ordinary packs without arbitrary mutation callbacks;
- standard-world definitions separated from trusted primitive semantics;
- headless-first execution and narrow product facades;
- explanation and provenance as first-class outputs.

Existing algorithms and tests that directly protect these invariants are
rewrite inputs. Their current public type shapes are not reuse requirements.

## Refined

| Earlier concept | Target refinement |
|---|---|
| Compiler-shaped decision passes | Stable outer lifecycle ports; a future pass graph may be written anew inside one port only when a real experiment requires it |
| Tiered replay | Explicit restoration, verification, and counterfactual branch protocols |
| Transaction/event history | One hash-linked `AuthorityRecord` per revision, with internal same-moment resolution, nested attempts/commits, and optional self-contained reaction dispatch |
| Definition registry | Verified artifacts link into an exact process-independent `RuntimeDefinitionSet`; activation binds only its required semantic interfaces into a reconstructible process-local registry |
| Context completeness | Valid empty and unavailable are distinct; dependency witnesses are explicit |
| Target/action selection | Grounded intent/action candidates with canonical identity and trusted lowering |
| Planning and execution | One `ActivityController` owns initialization/advancement; planning remains internal and action opportunities are accepted one-shot control records |
| Multi-resolution | One active representation per explicit resolution scope with checked transitions |
| Research profiles | Profiles compose typed lifecycle ports rather than one cross-lifecycle runner |

## Replaced

The following earlier structures are not target constraints:

- one configurable decision pass graph spanning appraisal, intent, activity,
  and action cadence;
- broad representation enums whose variants lack an operational producer,
  consumer, or scenario;
- treating a declared context source as available without successful
  projection;
- one target enum for immediate action, process continuation, and waiting;
- action choice that does not ground real definition roles and binding rules;
- mutation authority protected mainly by cross-crate convention;
- one opaque numeric version anchor for semantic, schema, and exact identity;
- session-local numeric definition IDs as durable pack identity;
- pure event reconstruction or a debug log without atomic scheduler/state
  history;
- dynamic native Rust plugins or generic mutable callbacks.

These internal structures receive no compatibility facade, alias, format
importer, feature-selected old path, or dual execution path.

## Preserved pre-redesign baseline

The preservation branch contains the former `world-context` and
`world-decision` implementation. It remains evidence and a source of reusable
test ideas, typed values, projection logic, and trace concepts, but it is not
the normative production control plane and is absent from the rewrite tree.

For that preserved baseline:

- the generic runner was deleted rather than migrated; future
  pass-graph research starts as a new implementation behind one lifecycle
  port;
- unused representations should not be migrated automatically;
- preserved action/context work should be evaluated against grounded candidate,
  availability, and dependency-witness contracts;
- the former `world-engine -> world-authoring` Cargo dependency was known
  implementation drift and was removed with the legacy graph; the future
  target engine consumes verified/linked artifacts and must not depend on
  authoring;
- `world-lab` and `world-cli` are target product boundaries, not current
  workspace crates; they are introduced only with the experiment/CLI vertical
  slices in the roadmap;
- the future `world-standard` remains an optional composition-root choice
  rather than a dependency of generic engine, context, or authoring contracts;
- no compatibility facade preserves the superseded pipeline.

The rewrite cutover removed the complete selectable pre-redesign crate graph
before constructing the target slice. The first target-state merge combines
that clean foundation with the definition/artifact foundation and minimal
runtime vertical slice. Later slices extend only target structures. The exact
cutover is defined by the active
[rewrite roadmap](implementation-roadmap.md) and
[Target Rust Code Architecture](code-architecture.md).

## Document routing

The repository-wide [Documentation Guide](../../README.md), [Design
Index](../../design/README.md), and [Research Index](../../research/README.md)
are the entry points for non-normative material. Legacy files retain stable
paths for historical links, not because they remain active.

| Earlier document | Status |
|---|---|
| `ADR.md` | Frozen decision history; target decision record wins |
| `engine.md` | Frozen logical architecture input |
| `runtime-pipeline.md` | Frozen runtime input |
| `crates.md` | Frozen crate-boundary input |
| `configurable-decision-pipeline.md` | Replaced as production control plane |
| `implementation-plan.md` | Frozen; do not continue as active roadmap |
| `implementation-execution-contract.md` | Frozen; `AGENTS.md` and target roadmap govern |
| `project-conventions.md` | Frozen pre-target conventions; `AGENTS.md` and target code architecture govern |
| `docs/design/*.md` | Individually classified nonnormative input; frozen pre-target cross-system models must not be continued |
| `docs/research/*.md` | Individually classified rationale, evidence, or falsification proposal; not runtime authority |

## Unresolved by design

The following remain intentionally open inside their named boundaries:

- pack source syntax;
- final action/effect expression language;
- `ActivityController` planning/search algorithm and internal plan
  representation;
- detailed appraisal, belief-revision, and social models;
- storage backend;
- trace/export encoding;
- resolution conversion algorithms and population aggregation;
- Wasm interfaces and runtime;
- server, editor, registry, and distributed execution.

An unresolved internal choice does not reopen the authority, lifecycle,
scheduling, persistence, or trust decisions in the target package.
