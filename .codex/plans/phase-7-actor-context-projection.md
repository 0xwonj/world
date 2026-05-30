# Phase 7 Local Plan: Actor Context Projection

## Status

Draft implementation plan for Phase 7.

This plan should be reviewed before implementation. It is intentionally concrete
about module shape, public API direction, verification gates, and design
constraints. It is not a final permanent API reference.

## Purpose

Add the actor-relative context boundary between authoritative world state and
later semantic decision work.

The phase target is:

```text
WorldModel + DefinitionRegistry + actor scope
  -> ActorContextPipeline
  -> actor-relative derived context snapshot
  -> provenance + read dependencies + diagnostics
  -> decision-safe input for world-decision
```

The phase is complete when:

- `world-context` exposes a concrete `ActorContextPipeline`;
- pipeline input is read-only and uses `WorldModel` plus checked
  `DefinitionRegistry`;
- pipeline output is actor-relative, value-like, and does not hold privileged
  model/runtime authority;
- context contains first-class slots for observation, epistemic working set,
  social context view, capability set, action repertoire, and perceived
  affordances;
- context projection records read dependencies and provenance in a typed form;
- missing rich perception/capability/social semantics are represented honestly
  as empty or shallow derived outputs, not hidden hardcoded gameplay logic;
- later `world-decision` can consume `ActorContextProjection` or a decision
  input wrapper that keeps context and projection evidence together, without
  raw `WorldModel` access;
- dependency direction remains unchanged: no runtime, standard-runtime,
  mutation, or authoring/parser dependency enters `world-context`.

## Research Inputs

Primary local research:

- `.codex/research/phase-7-compiler-shaped-actor-context-research.md`
- `.codex/research/phase-6-standard-primitive-semantics-research.md`
- `.codex/research/phase-4-5-runtime-research.md`

Primary local architecture/design:

- `docs/architecture/implementation-plan.md`
- `docs/architecture/crates.md`
- `docs/architecture/engine.md`
- `docs/architecture/runtime-pipeline.md`
- `docs/design/simulation-transition-compiler.md`
- `docs/design/capability-affordance-and-actor-interface.md`
- `docs/design/perception-and-observation.md`
- `docs/design/epistemic-state.md`
- `docs/design/social-institutional-model.md`
- `docs/design/semantic-appraisal-and-motivation.md`
- `docs/design/intent-templates-and-planning.md`
- `docs/design/standard-world-library.md`

External patterns to apply selectively:

- MLIR pass discipline: projection steps should have typed inputs, outputs,
  read/write contracts, and verification points.
- MLIR dialect/operation separation: pure vocabulary can be consumed without
  executable runtime semantics.
- rustc/Salsa query discipline: record stable keys, read dependencies, and
  invalidation inputs without adding a query engine now.
- Differential dataflow pressure: derived context should be rebuildable and
  later incrementally maintainable.
- Souffle/Datalog provenance pressure: derived facts should carry explanation
  anchors.
- PDDL 2.1 planning pressure: keep repertoire, affordance, concrete request,
  validation, and execution separate.
- Rust API Guidelines: use domain newtypes/enums, value-like outputs, thin
  re-export facades, deterministic collection choices, and narrow concrete
  APIs before framework traits.

## Baseline Decisions

### Compiler-Shaped, Not Compiler-Framework

Phase 7 should apply compiler methodology as design discipline:

```text
input representation
output representation
transformation kind
allowed reads
allowed writes
provenance output
invalidation dependencies
diagnostic surface
```

Do not implement:

```text
PassManager
dyn ContextPass
generic CompilerPipeline
Salsa database
Datalog engine
ECS projection engine
script/plugin callback system
```

The first implementation should use one concrete public pipeline and private
module-level helper functions. Traits can be introduced later only after real
extension pressure exists.

### Actor Context Is Derived Context

`ActorContext` is not:

- hard truth;
- source syntax;
- checked effect IR;
- committed event history;
- runtime-control state;
- final intent;
- a mutable session handle.

It is a derived, actor-relative, decision-safe snapshot or view assembled from
authoritative stores and checked definitions.

Use "context", "view", "snapshot", "working set", and "projection" terminology.
Reserve "IR" for checked definition, semantic declaration, or effect-program
artifacts.

### Preserve Crate Direction

Current dependency direction for this phase:

```text
world-context
  -> world-core
  -> world-defs
  -> world-model
```

Optional later dependency, only when a concrete projection needs pure standard
vocabulary:

```text
world-context
  -> world-standard
```

Forbidden dependencies:

```text
world-context -> world-runtime
world-context -> world-standard-runtime
world-context -> world-decision
world-context -> world-authoring
```

`world-context` may read pure definition/vocabulary data. It must not install
or call executable primitive semantics, stage transactions, validate hard
effects, or publish accepted records.

### Do Not Fake Rich World Semantics

The current code has the right authority substrate, but it does not yet have
complete body, sense, equipment, skill, material, social-rule, perception
channel, or content schema families.

Phase 7 should establish the boundary and minimal projection shape. It should
not pretend the full simulation grammar exists.

Acceptable first outputs:

- empty observations where no perception model exists;
- shallow epistemic working sets from actor-held records;
- social context summaries from available accepted social records;
- definition-derived action repertoire candidates with explicit provenance;
- empty affordance sets when no affordance derivation exists.

Unacceptable first outputs:

- hardcoded gameplay taxonomies such as stats, spells, skills, combat styles,
  or recipes;
- omniscient hard-world facts exposed as actor-visible observations;
- a fallback that treats every `ActionDef` as fully available and valid;
- hidden validation that duplicates runtime hard-truth checks;
- decision/appraisal output produced by context projection.

### No Legacy Completion State

By the Phase 7 exit gate, there should be one canonical actor-context entry
point. Do not leave parallel public context/query shortcuts that let later
decision code bypass `ActorContextPipeline`.

Internal `WorldModel::query_layer()` remains the model read surface. The new
boundary is that decision-facing code should consume `ActorContextProjection`
or a wrapper that includes both context and projection evidence, not raw
kernel/debug model queries.

## Current Baseline

The current `world-context` crate is a stub:

```text
crates/world-context/
  Cargo.toml
  src/lib.rs
```

It already depends only on:

```text
world-core
world-defs
world-model
```

This is the right starting point.

Existing code to preserve and build on:

- `world-defs::DefinitionRegistry` is a parser-free checked registry.
- `world-defs::ActionDef` has actor-role metadata.
- `world-defs::SemanticDeclarationDef` is currently an envelope only; Phase 7
  should not execute semantic declarations.
- `world-model::WorldModel::query_layer()` exposes distinct read surfaces:
  kernel, actor-relative, semantic-context, and debug.
- `world-model::ActorRelativeQuery` already scopes epistemic reads by actor.
- `world-model::AuthorityRead`, `InvalidationPackage`,
  `DerivedViewDescriptor`, and `DerivedViewRegistry` provide the first
  invalidation/read-dependency vocabulary.

No major pre-Phase-7 refactor is required.

## Target Module Shape

Target `world-context` structure:

```text
crates/world-context/src/
  lib.rs
  error.rs
  request.rs
  pipeline.rs
  context.rs
  dependency.rs
  provenance.rs
  diagnostic.rs
  observation.rs
  epistemic.rs
  social.rs
  capability.rs
  repertoire.rs
  affordance.rs
  tests.rs
```

Ownership:

- `lib.rs`: thin public re-export facade.
- `error.rs`: context projection errors.
- `request.rs`: input/request/options types.
- `pipeline.rs`: public `ActorContextPipeline` and orchestration.
- `context.rs`: `ActorContext` and `ActorContextProjection`.
- `dependency.rs`: read dependencies, definition dependencies, staleness
  matching helpers.
- `provenance.rs`: context provenance records and source anchors.
- `diagnostic.rs`: source-free runtime/context diagnostics.
- `observation.rs`: `ObservedState`, `ObservedEvent`, observation projection.
- `epistemic.rs`: `EpistemicWorkingSet` from actor-held records.
- `social.rs`: `SocialContextView` from accessible social/semantic context.
- `capability.rs`: `CapabilitySet` and capability evidence.
- `repertoire.rs`: `ActionRepertoire` and definition-derived action entries.
- `affordance.rs`: `PerceivedAffordance` and affordance evidence.
- `tests.rs`: crate-local behavioral and boundary tests.

Keep leaf modules small. If one module grows significantly during
implementation, split it by domain, not by generic pass-framework concepts.

## Public API Direction

Public facade should be centered on these types:

```rust
pub struct ActorContextPipeline;

pub struct ActorContextInput<'a> {
    pub model: &'a WorldModel,
    pub definitions: &'a DefinitionRegistry,
}

pub struct ActorContextRequest {
    actor: ActorId,
    options: ActorContextOptions,
}

pub struct ActorContextProjection {
    context: ActorContext,
    report: ContextProjectionReport,
}

impl ActorContextPipeline {
    pub fn project(
        &self,
        input: ActorContextInput<'_>,
        request: ActorContextRequest,
    ) -> Result<ActorContextProjection, ContextError>;
}
```

This shape is preferred over:

```rust
pub trait ContextPass { ... }
pub struct PassManager { ... }
pub fn get_visible_everything(model: &WorldModel, actor: ActorId) -> ...
```

Reasons:

- the public boundary is explicit and easy for `world-decision` to depend on;
- implementation can remain concrete while pass contracts are still visible;
- no plugin or generic pass abstraction leaks into the first API;
- output can carry context plus projection metadata together.

### Request And Options

Suggested request shape:

```rust
pub struct ActorContextRequest {
    actor: ActorId,
    options: ActorContextOptions,
}

pub struct ActorContextOptions {
    include_debug_diagnostics: bool,
}
```

Keep options minimal. Add budgets, focus targets, resolution scopes, or
projection modes only when the first implementation actually uses them. Avoid
"future option bags" with many inert fields.

### Context Output

Suggested output shape:

```rust
pub struct ActorContext {
    actor: ActorId,
    observations: ObservationContext,
    epistemic: EpistemicWorkingSet,
    social: SocialContextView,
    capabilities: CapabilitySet,
    repertoire: ActionRepertoire,
    affordances: Vec<PerceivedAffordance>,
}
```

Rules:

- `ActorContext` should not borrow `WorldModel`.
- `ActorContext` should not expose kernel/debug read handles.
- `ActorContext` should not contain runtime semantics handlers.
- `ActorContext` alone is not the decision-stage handoff; downstream decision
  code should receive projection evidence together with the value snapshot.
- output collections should have deterministic ordering.
- prefer `Vec<T>` for ordered projection output and `BTreeSet`/`BTreeMap` for
  deterministic membership/indexing.

### Projection Report

Suggested report shape:

```rust
pub struct ContextProjectionReport {
    reads: ContextReadSet,
    provenance: ContextProvenance,
    diagnostics: Vec<ContextDiagnostic>,
}
```

The report is not source diagnostics. It is runtime/context evidence for
debugging, explanation, future incremental invalidation, and the later decision
handoff. Decision code should not silently discard it.

### Dependency Types

Suggested dependency shape:

```rust
pub struct ContextReadSet {
    reads: BTreeSet<AuthorityRead>,
    definitions: BTreeSet<DefinitionId>,
}

pub enum ContextReadDependency {
    Authority(AuthorityRead),
    Definition(DefinitionId),
    DerivedView(DerivedViewKey),
}
```

If `DerivedViewKey` is not publicly usable enough for `world-context`, keep the
first implementation to `AuthorityRead` plus `DefinitionId` and document the
future derived-view hook. Do not make model internals public only to satisfy a
premature dependency design.

### Provenance Types

Suggested provenance shape:

```rust
pub enum ContextProvenanceSource {
    ActorScope(ActorId),
    Definition(DefinitionId),
    AcceptedRecord(AcceptedRecordId),
    EventRecord(EventRecordId),
    QueryRead(AuthorityRead),
}

pub struct ContextProvenance {
    sources: Vec<ContextProvenanceSource>,
}
```

Keep this coarse. It should be enough to explain why a context element exists
without pretending to be a final proof tree.

### Diagnostics

Suggested diagnostic shape:

```rust
pub enum ContextDiagnostic {
    ProjectionUnavailable { projection: ContextProjectionKind },
    UnsupportedSemanticDeclaration { definition: DefinitionId },
    ContextTruncated { projection: ContextProjectionKind },
}
```

Use structured enum variants, not strings as the primary diagnostic source.
Do not add source spans or terminal diagnostic renderers in this phase.
Projection availability/completeness must be reported as normal projection
metadata, not only as optional debug diagnostics.

## Projection Flow

Target orchestration:

```text
ActorContextPipeline::project
  -> validate request and create projection state
  -> collect actor-relative query labels
  -> project observations
  -> retrieve epistemic working set
  -> build social context view or explicit unavailable status
  -> derive actor-specific capability evidence
  -> derive action repertoire
  -> derive perceived affordances
  -> assemble ActorContext
  -> return ActorContextProjection { context, report }
```

This order is conceptual. Implementation may share intermediate data directly
inside `pipeline.rs`, but each stage should remain visible in module ownership
and tests.

### Observation Projection

Initial responsibility:

- create `ObservationContext` with typed slots for observed state and observed
  events;
- record read dependencies for any model surfaces actually used;
- preserve hidden-truth boundary;
- return empty observations when no perception data exists.

Do not:

- expose hard world/entity/relation state as known observations unless there is
  an explicit actor-relative basis;
- persist memory;
- create social meaning;
- create pressure or intent.

### Epistemic Working Set

Initial responsibility:

- use `WorldModel::query_layer().actor_relative(actor)` to retrieve actor-held
  epistemic records;
- project record ids, definitions, and provenance into a value-like working set;
- record `AuthorityRead::epistemic_store()` and actor scope in the report.

Do not:

- read other actors' epistemic records;
- write new epistemic records;
- interpret records into appraisal or final action choice.

### Social Context View

Initial responsibility:

- keep `SocialContextView` as the actor-relative social slot;
- report the slot as unavailable/shallow when there is no meaningful social
  projection;
- record social/chronology/epistemic/appraisal read labels only when used;
- keep the view actor-scoped even if current model stores are coarse.

Do not:

- execute `SemanticDeclarationDef::SocialRule`;
- write `SocialRecord`s;
- fill the view with count-only data that looks semantically complete;
- infer laws, debts, reputation, or obligations beyond stored/shallow data.

### Capability Set

Initial responsibility:

- represent actor-owned capability evidence in a typed set;
- derive only capabilities that the current checked definitions and actor scope
  can justify;
- record definition dependencies for any action/process/semantic definitions
  used.

Because the current model lacks rich body/skill/equipment semantics, the first
implementation should be conservative. `ActionDef::actor_role()` is schema
metadata, not capability evidence. Do not populate `CapabilitySet` from
definition-derived action schemas alone. Leave it empty unless an entry has
actor-specific evidence.

Do not:

- hardcode stats, skills, spells, body parts, equipment, or professions;
- treat capability as hard validation;
- grant every action unconditionally without actor-specific evidence.

### Action Repertoire

Initial responsibility:

- produce definition-derived actor-facing action-schema candidates from checked
  `ActionDef`s;
- use `ActionDef::actor_role()` as the minimum definition-level signal that an
  action is actor-facing;
- keep entries schema-level, not fully target-bound;
- include action id, name, actor role, declared roles, and a status that cannot
  be confused with actor-owned capability or runtime validation.

Do not:

- enumerate every target-specific action choice;
- bind concrete roles beyond actor scope;
- call runtime validation or primitive semantics;
- produce final `ActionRequest`.

Recommended status vocabulary:

```text
actor_facing_schema
definition_candidate
blocked_by_missing_actor_role
requires_context_not_yet_modelled
```

Use exact enum names during implementation only after checking local naming.
The important design is that repertoire entries distinguish "actor-facing
schema candidate" from "this actor has the capability/evidence to attempt it"
and from "will pass runtime validation now".

### Perceived Affordance

Initial responsibility:

- provide a typed place for target/context affordances;
- preserve known/suspected/rumored/inferred status when data exists;
- return an empty set when no observation/affordance data exists.

Do not:

- make affordance grant new action schemas;
- use the action id as the core identity of the perceived affordance;
- expose hidden hard truth;
- validate final effects;
- infer all physical possibilities from raw relation/world stores without an
  actor-visible basis.

## Implementation Sequence

### 1. Establish Module Skeleton And Facade

Add the target `world-context/src` modules and keep `lib.rs` as a thin
re-export layer.

Public modules should expose stable domain types only. Internal helper
functions should stay private to their modules.

Acceptance:

- crate compiles with empty/minimal types;
- no dependency changes unless a concrete need appears;
- public API is centered on `ActorContextPipeline`.

### 2. Add Request, Input, Error, Report, Dependency, And Provenance Types

Implement:

- `ActorContextInput<'a>`;
- `ActorContextRequest`;
- `ActorContextOptions`;
- `ContextError`;
- `ContextProjectionReport`;
- `ContextReadSet` or equivalent;
- `ContextProvenance` and source enum;
- `ContextDiagnostic`.

Rust practices to use:

- `#[non_exhaustive]` on public enums that are expected to grow;
- `#[must_use]` on constructors and accessors where missed values are likely
  bugs;
- private fields with accessor methods for public structs;
- deterministic collections;
- `Default` only when the default is semantically meaningful.

Acceptance:

- no stringly primary diagnostics;
- no source-span diagnostics;
- no mutable model/runtime authority in any type.

### 3. Add Context Value Types

Implement first value-like context outputs:

- `ActorContext`;
- `ActorContextProjection`;
- `ObservationContext`;
- `ObservedState`;
- `ObservedEvent`;
- `EpistemicWorkingSet`;
- `SocialContextView`;
- `CapabilitySet`;
- `ActionRepertoire`;
- `ActionRepertoireEntry`;
- `PerceivedAffordance`.

Rust practices to use:

- keep constructors validating only real invariants;
- prefer slices/iterators for accessors;
- avoid exposing mutable collection references;
- keep `Clone`, `Debug`, `PartialEq`, and `Eq` derives where useful for tests;
- use newtypes/enums for domain meaning instead of raw strings.

Acceptance:

- `ActorContext` contains no long-lived borrow of `WorldModel`;
- context output can be cloned/compared in tests;
- empty/shallow outputs are explicit and typed.

### 4. Implement Pipeline Orchestration

Implement `ActorContextPipeline::project(...)` as the single public entry point.

Internal orchestration can use a private accumulator:

```rust
struct ProjectionState {
    reads: ContextReadSet,
    provenance: ContextProvenance,
    diagnostics: Vec<ContextDiagnostic>,
}
```

Do not expose this as a public pass framework.

Acceptance:

- projecting an empty model returns an `ActorContextProjection`, not a panic;
- report includes actor scope and relevant read dependencies;
- every internal projection stage has one obvious owning module.

### 5. Implement Epistemic Projection First

Use the existing actor-relative query surface to project actor-held epistemic
records.

Tests should prove:

- actor A sees actor A epistemic records;
- actor A does not see actor B epistemic records;
- projected records preserve record id, definition, and provenance where
  available;
- report includes the actor-truth/epistemic read dependency.

This is the first concrete proof that `world-context` is actor-relative rather
than omniscient.

### 6. Implement Social And Observation Skeletons

Implement shallow `ObservationContext` and `SocialContextView`.

These should be honest:

- no perception model means no invented observations;
- no social rule execution means explicit unavailable/shallow status, not a
  count-only view that looks complete;
- projection availability/completeness is always visible in the report, not only
  in debug diagnostics.

Tests should prove:

- observation projection does not expose kernel/debug hard-world data by
  default;
- unavailable/shallow projection families are reported explicitly;
- social context reads use semantic/social read labels only when a real shallow
  projection is used;
- no writes occur.

### 7. Implement Actor-Facing Repertoire And Evidence-Backed Capability

Use `DefinitionRegistry` to inspect checked actions.

Initial rule:

- actions with `actor_role()` can produce actor-facing schema candidates in the
  repertoire;
- entries remain schema-level and definition-derived;
- `CapabilitySet` remains empty unless an entry has actor-specific evidence;
- unsupported requirements/binding rules are metadata or diagnostics, not
  runtime validation;
- actions without actor-role metadata are not silently treated as actor-owned.

Tests should prove:

- an action with actor role appears in repertoire as a definition-derived
  actor-facing schema candidate;
- actor-facing schema candidates do not populate `CapabilitySet` by themselves;
- an action without actor role is excluded or marked non-actor-facing according
  to the chosen status model;
- report includes definition dependencies for included entries;
- repertoire does not produce concrete `ActionRequest`s.

### 8. Implement Affordance Placeholder Honestly

Add `PerceivedAffordance` and return an empty set until there is real
observation/standard vocabulary data to derive from. Its identity should be the
perceived target/context opportunity, not an action id.

This is still useful because it locks the Phase 8 input shape without lying
about current simulation richness.

Tests should prove:

- empty affordance output is stable and typed;
- unavailable/shallow affordance projection is visible in the projection report;
- no hard truth is exposed as perceived affordance without an actor-visible
  source.

### 9. Add Boundary And Dependency Tests

Add focused tests in `world-context` for:

- empty model projection;
- actor-relative epistemic filtering;
- report read dependencies;
- definition-derived actor-facing action repertoire;
- empty/evidence-backed capability semantics;
- no decision/runtime mutation types in public API usage;
- no debug/kernel query usage in context projection;
- deterministic output ordering.

If existing workspace guardrails do not cover the new crate adequately, extend
the manifest dependency-direction test to ensure:

```text
world-context !-> world-runtime
world-context !-> world-standard-runtime
world-context !-> world-decision
world-context !-> world-authoring
world-decision !-> world-model
```

Do not add a new dependency just for guardrail parsing.

### 10. Update Minimal Documentation And Rustdoc

Add rustdoc on public types explaining:

- actor-relative derived context;
- no mutation authority;
- no final decision;
- source-free diagnostics;
- future cache/incremental-read dependency purpose.

Do not broadly rewrite architecture docs unless implementation reveals a
specific mismatch.

## Future-Phase Co-Design Constraints

### Phase 8: Semantic Decision Middle-End

Phase 8 should depend on `world-context` and consume `ActorContextProjection`
or a `DecisionContextInput` wrapper that keeps `ActorContext` and
`ContextProjectionReport` together.

It should not require:

- `WorldModel`;
- `world-runtime`;
- `PrimitiveSemanticsRegistry`;
- hard validation APIs;
- kernel/debug query surfaces.

If Phase 8 cannot work without raw model access, Phase 7 did not establish the
right boundary.

Phase 7 should therefore keep the actor-context projection broad enough to
carry the major decision inputs:

- observed state/events;
- epistemic working set;
- social context view;
- capabilities;
- repertoire;
- affordances;
- provenance and diagnostics.

It should not implement appraisal, pressure, candidate intent, or intent
selection.

### Phase 9: Authoring And Verification

Phase 9 will provide parser/source diagnostics, source spans, pack dependency
graphs, semantic declaration verification, and richer registry construction.

Phase 7 should not import that work early. It should use checked
`DefinitionRegistry` as input and source-free runtime/context diagnostics as
output.

Later, Phase 9 can attach source spans to the definitions that Phase 7 already
references through `DefinitionId`.

### Phase 10: Engine Facade

The engine facade should later wire:

```text
definition loading
world state
runtime drain
actor context projection
decision
runtime submission
inspection
```

Phase 7 should expose a clean enough `ActorContextPipeline` that `world-engine`
can orchestrate it without merging context and runtime authority.

### Later Projection Acceleration

Future derived-view cache, incremental query, ECS-backed local projection,
Datalog-style rule matching, or differential/dataflow maintenance can be added
behind the same boundary if Phase 7 records:

- stable actor/context request;
- read dependencies;
- provenance;
- deterministic output order;
- value-like context output.

Do not build these accelerators now.

## Rust And Design Patterns To Use

Use these patterns deliberately:

- **Concrete facade:** `ActorContextPipeline` is the public entry point.
- **Value object output:** `ActorContext` and nested views are owned snapshots or
  explicit read handles, not live model borrows.
- **Private fields:** preserve invariants and allow API evolution.
- **Narrow constructors:** validate real invariants; do not create ceremonial
  builders.
- **Deterministic collections:** prefer `Vec` for source/projection order and
  `BTreeSet`/`BTreeMap` for sorted membership.
- **Structured diagnostics:** enum variants first, rendering later.
- **Typed provenance:** record ids, definition ids, event ids, authority reads,
  and actor scope as typed anchors.
- **Thin re-export facade:** keep `lib.rs` small.
- **Source-free context errors:** Phase 7 errors should not know parser spans.
- **No generic pass trait yet:** pass contracts are visible through module
  names, result types, and tests.
- **No mutation handles:** no `&mut WorldModel`, no runtime staging context, no
  accepted package constructors in context APIs.

Avoid these patterns:

- broad `WorldSystem`/`RuntimeSystem` style traits;
- opaque string diagnostics as the primary API;
- long-lived references to model stores inside output context;
- public mutable collection fields;
- convenience APIs that expose debug/kernel truth to decision code;
- compatibility aliases for abandoned context entry points.

## Test Plan

During implementation, run the relevant subset after each meaningful slice:

```bash
cargo fmt --all
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
git diff --check
```

Targeted test coverage:

- `ActorContextPipeline` projects an empty context without panic.
- actor-relative epistemic projection filters by actor.
- context report records read dependencies.
- definition-derived repertoire includes actor-facing schema candidates.
- definition-derived action schemas do not populate `CapabilitySet` without
  actor-specific evidence.
- non-actor-facing actions do not become actor-owned actions silently.
- unavailable/shallow projection families are visible in normal projection
  metadata.
- context projection does not expose debug/kernel hard truth as observation.
- context output does not require `world-runtime`.
- `world-decision` cannot depend on `world-model`.
- projection ordering is deterministic.
- diagnostics are structured and source-free.

Additional acceptance criteria:

- no new dependency on `world-runtime`, `world-standard-runtime`,
  `world-decision`, or `world-authoring`;
- no new major dependency;
- no generic pass manager;
- no source parser or source diagnostic renderer;
- no runtime primitive semantics execution;
- no hard mutation, accepted package construction, or model receiver calls;
- no final intent, pressure, appraisal, or action selection.

## Accepted Decisions

These decisions are fixed for this implementation line. Do not leave parallel
compatibility behavior for the rejected alternatives.

### Capability And Repertoire

`CapabilitySet` is actor-specific evidence only. It must not be populated from
global action definitions merely because an action declares `actor_role()`.

`ActionRepertoire` may contain definition-derived actor-facing action-schema
candidates. Those entries are not runtime validation results and are not
capability grants.

### Social Context

Do not fill `SocialContextView` with count-only data. Keep the slot, but report
unavailable/shallow projection status until social context can carry meaningful
record references, access mode, and provenance.

### Decision Handoff

Phase 8 consumes context and projection evidence together. The accepted handoff
is `ActorContextProjection` or a dedicated decision input wrapper, not bare
`ActorContext`.

### Affordance Identity

`PerceivedAffordance` should not be identified by an action id. It should
represent perceived target/context opportunity. Action binding belongs to a
later matching step.

### Projection Completeness

Unavailable or shallow projection families must be visible in normal projection
metadata. Do not hide this only behind debug diagnostics.

### Context Input Surface

External callers may construct context input and call the pipeline, but they
should not use `world-context` as a public transport for raw `WorldModel`
authority. Narrow or crate-private access is preferred.

### Dependency Guardrails

Forbid `world-decision -> world-model` before decision code is implemented.
Decision code should depend on `world-context`, not raw model query surfaces.

## Risk Register

### Risk: Actor Context Becomes Convenience Queries

If Phase 7 exposes many direct helper functions instead of a central pipeline,
Phase 8 will likely depend on raw model queries.

Mitigation:

- keep `ActorContextPipeline` as the public decision-facing entry point;
- keep internal query helpers private;
- write tests against `ActorContextProjection`, not scattered helpers.

### Risk: Omniscient Truth Leakage

Observation and affordance projection can accidentally read hard stores and
present hidden truth as actor-visible context.

Mitigation:

- start conservative;
- record read labels;
- test actor filtering;
- keep debug/kernel query surfaces out of public context output.

### Risk: Capability/Repertoire Overclaiming

Current model lacks rich actor capability semantics. The pipeline may be tempted
to mark too much as available.

Mitigation:

- keep `CapabilitySet` actor-specific and evidence-backed;
- keep definition-derived action schemas in repertoire, not capability;
- use evidence/status fields;
- separate capability, repertoire, affordance, request, validation, and
  resolution;
- avoid concrete target binding in repertoire.

### Risk: Framework Abstraction Too Early

Compiler language can push the implementation toward a generic pass manager.

Mitigation:

- concrete modules first;
- private helpers;
- introduce traits only after multiple real implementations need interchange.

### Risk: Phase 8/9 Leakage

Context code may start executing semantic declarations, appraisal rules, intent
templates, or source diagnostics.

Mitigation:

- allow reading checked definition metadata only;
- keep semantic/appraisal/intent outputs absent;
- keep source spans and parser dependencies out.

### Risk: Cache Design Locks In Too Soon

Full cache/incremental systems are attractive but premature.

Mitigation:

- record dependencies/provenance now;
- keep outputs rebuildable;
- defer materialized/incremental cache policy.

## Summary

Phase 7 should make `world-context` the actor-relative derived-context waist.
The first implementation should be concrete and modest, but architecturally
strict:

```text
checked definitions + authoritative read surfaces
  -> actor-relative projection pipeline
  -> value-like decision context
  -> provenance/read dependencies/diagnostics
```

This sets up Phase 8 to build semantic decision work on a clean input boundary,
sets up Phase 9 to attach source-aware authoring diagnostics later, and keeps
the runtime mutation authority untouched.
