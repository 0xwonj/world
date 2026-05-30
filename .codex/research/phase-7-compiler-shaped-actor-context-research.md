# Phase 7 Compiler-Shaped Actor Context Research

## Status

Research note for Phase 7 preparation.

This is not a concrete implementation plan. It records local architecture
constraints, external compiler/runtime references, code-shape observations, and
Phase 8-9 co-design implications to carry into the next `.codex/plans`
document.

## Scope

Phase 7 concerns the actor-relative read boundary between authoritative world
state and later decision work:

- observation projection
- actor-facing context assembly
- capability derivation
- action repertoire derivation
- perceived affordance derivation
- epistemic working set projection
- accessible social context projection
- context provenance, diagnostics, and invalidation dependencies

The core research question is not whether to use "compiler ideas" in the
abstract. The question is which compiler methods should become real design
discipline for actor context without turning `world-context` into a generic
compiler framework, query engine, or planner.

## Main Conclusion

Phase 7 should treat actor context as a compiler-shaped derivation/projection
boundary:

```text
WorldModel + DefinitionRegistry + actor scope
  -> ActorContextPipeline
  -> actor-relative derived snapshot
  -> provenance + read dependencies + diagnostics
  -> world-decision
```

The useful compiler transfer is:

- explicit pass contracts;
- staged representations;
- checked registry inputs;
- query dependency and invalidation discipline;
- provenance and diagnostics;
- clear separation between derivation, decision, lowering, execution, and
  publication.

The wrong transfer is:

- an MLIR clone;
- a universal `Pass` trait or pass manager before there is duplication;
- calling every context object an IR;
- putting live `WorldModel`, scheduler, or transaction state into a Salsa-like
  database;
- making ECS, Datalog, scripting, or dataflow the source of truth;
- letting decision code read raw omniscient stores.

## Local Contract

The local docs already provide enough direction to constrain Phase 7.

`docs/design/simulation-transition-compiler.md` is the main architecture owner.
It says the engine "incrementally compiles actor-relative situations into
checked simulation transitions" and defines the core staged flow:

```text
pack declarations + current world/context + actor/policy choice
  -> checked candidates
  -> selected intent/action/process
  -> typed effect program instance
  -> CausalTransaction
  -> EventRecord + store mutations
```

The same document says every layer needs a concrete contract:

```text
input representation
output representation
transformation kind
allowed reads
allowed writes
verifier
pass boundary
provenance contract
invalidation dependencies
```

It also classifies actor context as `DerivedContext`, not hard truth and not
effect IR. That matters: `ObservedState`, `ObservedEvent`,
`EpistemicWorkingSet`, `SocialContextView`, `CapabilitySet`,
`ActionRepertoire`, and `PerceivedAffordance` are derived actor-relative
representations. They are not source syntax, transaction state, committed
records, or executable effect programs.

`docs/architecture/implementation-plan.md` aligns with this. Phase 7 locks:

- context is actor-relative, not omniscient;
- context generation does not mutate hard truth;
- context outputs are snapshots, ids, read handles, or derived views;
- derived context invalidates from model/runtime invalidation packages.

`docs/architecture/crates.md` defines the crate boundary:

```text
world-context
  -> world-core
  -> world-defs
  -> world-model
  -> world-standard where standard vocabulary is needed
```

`world-context` must not own:

- final intent selection;
- hard validation;
- transaction staging;
- `world-standard-runtime` or primitive semantics installers;
- durable memory/social/appraisal writes without accepted update gates.

This dependency shape is important. `world-standard` can provide pure reusable
vocabulary for context projection. `world-standard-runtime` is executable
trusted semantics and must stay out of context.

## Current Code Readiness

The current code is ready for Phase 7 without a major pre-refactor.

`world-defs` already looks like the parser-free checked-definition layer:

- `DefinitionRegistry` is a checked lookup table for runtime-facing
  definitions.
- `DefinitionRegistryBuilder` constructs the registry and calls verifier code.
- `registry::validate` verifies cross-definition contracts such as primitive
  existence, required args, unknown args, event declaration, replay strength,
  role coverage, permission coverage, and action/process owner coverage.

This maps well to a compiler symbol table plus verifier. Phase 7 should
consume this registry as checked input, not reintroduce ad hoc source/string
interpretation.

`world-runtime` and the standard crates already preserve the operation/handler
split:

- `EffectPrimitiveDescriptor` is pure primitive schema.
- `EffectPrimitiveDef` is the checked primitive definition.
- `PrimitiveSemantics` is the trusted executable handler boundary.
- `PrimitiveSemanticsRegistry` checks handler contracts against definitions.
- `TypedEffectInterpreter` interprets checked effect operations through
  capability-gated staging context.

This split should not be copied directly into `world-context`. Primitive
semantics are trusted execution handlers. Actor-context passes should begin as
concrete derivation code over read surfaces, not as a public trait-object
plugin registry.

`world-model` already has the first read-surface shape:

- `QueryLayer::kernel()`
- `QueryLayer::actor_relative(actor)`
- `QueryLayer::semantic_context(actor)`
- `QueryLayer::debug()`

The actor-relative surface is still shallow, but it already carries actor
scope and filters actor-held epistemic records. The semantic/debug/kernel split
is exactly the seam Phase 7 should build on.

`world-model` also already has invalidation vocabulary:

- `InvalidationPackage`
- `AuthorityRead`
- `DerivedViewDescriptor`
- `DerivedViewRegistry`

Phase 7 does not need a full cache engine, but it should return read
dependencies/provenance in a form that can later connect to this invalidation
model.

## External Reference Pressure

### MLIR: Dialects, Operations, Passes, And Legalization

MLIR is useful as a structural reference, not as a dependency.

Relevant references:

- [MLIR Language Reference](https://mlir.llvm.org/docs/LangRef/) describes
  dialects as modular namespaces for operations, attributes, and types.
- [MLIR Pass Infrastructure](https://mlir.llvm.org/docs/PassManagement/)
  treats passes as structured analysis/transformation units and supports pass
  instrumentation.
- [MLIR Dialect Conversion](https://mlir.llvm.org/docs/DialectConversion/)
  frames lowering as conversion from illegal source operations to legal target
  operations under explicit conversion targets and rewrite patterns.

Useful transfer:

```text
MLIR dialect
  -> standard vocabulary bundle or future trusted extension vocabulary

MLIR operation definition
  -> EffectPrimitiveDef / semantic declaration shape

MLIR verifier
  -> DefinitionRegistry validation

MLIR pass contract
  -> actor-context projection pass contract

MLIR conversion target
  -> target contract for later Intent -> Activity -> executable lowering
```

Phase 7 implication:

`world-context` should not be an arbitrary bag of queries. Its projection steps
should have named inputs, outputs, reads, provenance, and invalidation
dependencies. However, Phase 7 should not implement an MLIR-style pass manager.
The local docs already warn that `RepresentationClass` and `PassClass` are
design taxonomy labels, not mandatory runtime base classes.

### rustc Queries, Salsa, And rust-analyzer

rustc and rust-analyzer are useful for dependency tracking and recomputation
discipline.

Relevant references:

- [rustc incremental compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html)
  explains query dependency tracking and green/red revalidation.
- [rustc-dev-guide on Salsa](https://rustc-dev-guide.rust-lang.org/queries/salsa.html)
  notes Salsa's role as an incremental recomputation model used outside rustc,
  including rust-analyzer.
- [rust-analyzer architecture](https://rust-analyzer.github.io/book/contributing/architecture.html)
  describes a query-based analyzer where inputs are explicitly supplied facts
  and derived values are computed from those inputs.

Useful transfer:

- model actor-context projection around stable query keys;
- record which authority/read labels a projection consumed;
- distinguish input facts from derived outputs;
- make derived outputs value-like and recomputable;
- prepare for cache invalidation without adopting a query engine now.

Phase 7 implication:

The first implementation should not add Salsa. Instead it should define
lightweight types such as:

```text
ActorContextRequest
ContextReadDependency
ContextProvenance
ContextDiagnostic
ContextProjectionReport
```

Those types preserve the shape needed for future incremental recomputation.
They also keep the runtime model simple while still making Phase 8 decision
inputs auditable and cacheable later.

### Differential Dataflow And Incremental Derived Views

Differential dataflow is relevant to future high-volume derived projections,
not to the initial Phase 7 implementation.

Relevant reference:

- [Differential Dataflow overview](https://www.microsoft.com/en-us/research/video/incremental-iterative-and-interactive-computation-using-differential-dataflow/)
  describes reusing previous results when collections change, including loop
  feedback and new input data.

Useful transfer:

- derived context should be expressible as a function of input collections and
  deltas;
- invalidation dependencies should be explicit;
- full recomputation should not become the assumed final model;
- derived views may later become materialized or incremental.

Phase 7 implication:

Do not add differential dataflow. Do make context outputs explicit about:

- read dependencies;
- actor scope;
- authority class;
- source event/state anchors;
- whether the result is snapshot-like or cacheable.

### Datalog, Souffle, Provenance, And Rule Matching

Datalog-like systems are relevant to later semantic views, social/appraisal
rules, and authoring verification. Phase 7 should borrow provenance discipline
without adopting Datalog as the runtime core.

Relevant references:

- [Souffle provenance](https://souffle-lang.github.io/provenance) provides
  proof/explanation modes for derived facts.
- [Rete](https://www.sciencedirect.com/science/article/pii/B9780934613538500418)
  is a classic reference for many-pattern/many-object rule matching pressure.

Useful transfer:

- derived facts need explanation/provenance, not only values;
- repeated many-rule/many-object matching should later use indexes or rule
  networks;
- rule matching should not mutate authority state directly;
- context-derived affordances should be explainable by observed state,
  knowledge, social context, or standard vocabulary.

Phase 7 implication:

Initial `CapabilitySet`, `ActionRepertoire`, and `PerceivedAffordance` can be
simple typed vectors/sets with provenance. Avoid a Datalog engine until there
are enough semantic rules to justify it. Preserve the ability to explain:

```text
why this actor owns this action schema
why this target appears to afford this binding
which observation, knowledge, or social context supported it
```

### PDDL 2.1, Durative Actions, And Planning Pressure

PDDL 2.1 is relevant to Phase 8-9 and process/activity boundaries, especially
durative actions, temporal conditions, and numeric fluents.

Relevant reference:

- [PDDL 2.1 paper](https://www.cs.cmu.edu/afs/cs/project/jair/pub/volume20/fox03a-html/JAIRpddl.html)
  describes temporal and numeric planning domains, durative actions,
  continuous/discretized numeric change, and concurrent plan validation.

Useful transfer:

- actor context should not explode into fully grounded action choices;
- repertoire and affordance should stay separated;
- later intent/activity lowering needs a temporal target, not only immediate
  atomic action;
- process definitions are the durable execution bridge for long-running work.

Phase 7 implication:

`ActionRepertoire` should represent what the actor can try in principle.
`PerceivedAffordance` should represent what observed targets appear to support.
It should not enumerate every possible fully bound action every turn. This
keeps the actor interface compact and prepares Phase 8 for candidate intent
generation without making Phase 7 a planner.

### Rust API Guidelines

Rust API guidance supports the current direction:

- [Rust API Guidelines: Type Safety](https://rust-lang.github.io/api-guidelines/type-safety.html)
  favors domain-specific types and builders for complex construction.
- [Rust API Guidelines: Flexibility](https://rust-lang.github.io/api-guidelines/flexibility.html)
  recommends letting callers control ownership and using generic arguments only
  where they encode real assumptions.

Useful transfer:

- use newtypes and enums for actor/context/provenance distinctions;
- make public context outputs value-like and not tied to long-lived borrows;
- prefer concrete pipeline types over broad public traits until extension
  pressure is real;
- keep `lib.rs` as a thin re-export facade;
- use builders only when construction has real validation or optionality;
- avoid generic trait parameters that obscure authority/read boundaries.

Phase 7 implication:

The likely Rust shape is a concrete pipeline:

```text
ActorContextPipeline
ActorContextInput<'a>
ActorContextRequest
ActorContext
ContextProjectionReport
```

Internal projection modules can stay private and concrete:

```text
observation
epistemic
social
capability
repertoire
affordance
provenance
diagnostic
```

This gives Phase 7 a clear architecture without prematurely designing a plugin
or pass-manager API.

## Compiler Pattern Mapping For Phase 7-9

### Phase 7: Derived Context Front-End

Compiler analogy:

```text
frontend analysis over an actor-specific view
```

Engine meaning:

```text
AuthorityState + checked definitions + actor scope
  -> DerivedContext
```

Phase 7 should own:

- actor-relative projection;
- non-omniscient snapshots;
- read dependency reporting;
- context provenance;
- structured but lightweight context diagnostics;
- decision-safe inputs.

Phase 7 should not own:

- source parsing;
- semantic appraisal;
- final intent selection;
- hard validation;
- effect execution;
- durable memory/social/appraisal writes except through later accepted gates.

### Phase 8: Semantic Decision Middle-End

Compiler analogy:

```text
semantic analysis + candidate generation + ranking/selection
```

Engine meaning:

```text
ActorContext
  -> Thought / Pressure / GoalPressure
  -> CandidateIntent / IntentScore
  -> selected or suggested Intent
```

The main Phase 7 co-design requirement is that Phase 8 should consume
`ActorContext`, not raw `WorldModel`. If Phase 8 needs privileged query access,
Phase 7 has not established the correct boundary.

### Phase 9: Authoring Front-End And Diagnostics

Compiler analogy:

```text
source/structured authoring input
  -> parsed declarations
  -> typed definition IR / semantic declaration IR
  -> diagnostics
  -> checked DefinitionRegistry
```

Engine meaning:

```text
pack source or editor data
  -> checked definitions and indexes
```

Phase 7 should not pull source-span diagnostics or parser concerns forward.
It should keep output diagnostics source-free and runtime-facing. Phase 9 can
later attach source spans and richer diagnostic rendering to the same logical
contract.

## Recommended Research Constraints For The Phase 7 Plan

These are not implementation steps. They are constraints the implementation
plan should respect.

### 1. Actor Context Is A Boundary Object

`ActorContext` should be the value `world-decision` receives. It should not hold
a long-lived `&WorldModel`, a mutable model reference, or runtime staging
authority.

The context may contain:

- ids;
- snapshots;
- compact typed views;
- read handles if they are explicit and actor-scoped;
- provenance references;
- diagnostics or warnings.

It should not contain:

- kernel/debug query access;
- `CausalTransaction` or runtime-control mutation handles;
- primitive semantics handlers;
- source parser or authoring diagnostics state.

### 2. Pass Contracts Should Be Visible Without A Generic Pass Manager

Each major internal stage should have a local contract:

```text
name
input
output
allowed reads
allowed writes
provenance output
read dependencies
failure/diagnostic surface
```

This can be represented through concrete functions and result structs. A
public `dyn ContextPass` framework is not justified yet.

### 3. Read Dependencies Are Part Of The Output

Context projection must be able to answer:

```text
What stores or authority classes did this output depend on?
Which actor scope was used?
Which definitions influenced the result?
Which events or records explain the result?
What invalidation package would make this stale?
```

The first implementation can be coarse. The important thing is to make
dependency/provenance a typed part of the output, not an afterthought.

### 4. Repertoire And Affordance Must Stay Separate

The capability/affordance design docs make a key distinction:

```text
Capability:
  why this actor owns this schema.

ActionRepertoire:
  what schemas the actor can attempt in principle.

PerceivedAffordance:
  what this observed target/context appears to support.

ActionRequest:
  what concrete attempt the actor submits.

Validation:
  whether the attempt is valid against hard truth.
```

Phase 7 should preserve this split. It should not generate every target-bound
action as the repertoire, and it should not validate hard truth as if context
projection were runtime execution.

### 5. Standard Vocabulary May Be Read, Standard Runtime May Not

`world-context` may use pure `world-standard` vocabulary when it needs standard
physical categories or primitive ids for projection. It must not depend on
`world-standard-runtime`, because trusted primitive semantics are executable
mutation behavior.

This keeps the context crate a read/projection layer.

### 6. The First Version Should Be Honest About Missing Rich World Semantics

The current model has stores and query surfaces, but it does not yet have rich
body, sense, equipment, skill, social-rule, perception-channel, or content
schema families.

Phase 7 should establish the boundary and minimal projection shape without
pretending the full simulation grammar exists. Empty or shallow sets are
acceptable if they are typed, provenance-aware, and future-extensible.

### 7. Diagnostics Should Be Structured But Source-Free

Phase 7 diagnostics are runtime/context diagnostics:

```text
actor not found
definition missing
projection input unavailable
context truncated by budget
unsupported declaration family
```

They should not use source spans. Phase 9 owns source diagnostics and rich
authoring reports.

## Risk Register

### Risk: Ad Hoc Query Accretion

If Phase 7 becomes scattered `get_visible_*` helpers, later decision code will
start depending on raw model surfaces. That weakens actor-relative access,
provenance, and invalidation.

Mitigation:

- centralize public entry through `ActorContextPipeline`;
- return `ActorContext` plus projection report;
- keep raw model reads private to projection modules.

### Risk: Omniscient Truth Leakage

Actor context must not silently read kernel/debug surfaces and expose hard truth
as actor-visible information.

Mitigation:

- model allowed read surfaces explicitly;
- include read labels in provenance;
- add tests that actor-relative output excludes other actor records or debug
  data.

### Risk: Premature Framework Traits

A generic pass manager would create abstraction before extension pressure is
real. It may also obscure authority differences.

Mitigation:

- use concrete pipeline and private helpers first;
- introduce traits later only when there are multiple interchangeable pass
  families with real shared behavior.

### Risk: Misusing IR Terminology

Calling `ActorContext` an IR would blur the distinction between checked
transformable effect/definition IR and actor-facing derived snapshots.

Mitigation:

- call Phase 7 outputs "derived context", "snapshot", "view", or "working set";
- reserve IR for checked definition/effect/semantic declaration artifacts.

### Risk: Pulling Phase 8/9 Forward

Phase 7 can be tempted to perform appraisal, intent generation, semantic rule
execution, or source diagnostics.

Mitigation:

- keep Phase 7 to actor-relative readable context;
- treat semantic declarations as registry data only unless a Phase 7 projection
  truly needs a definition index;
- leave appraisal/intent to `world-decision`;
- leave parser/source diagnostics to `world-authoring`.

### Risk: Full Recompute Becomes The Final Model

The first version may recompute context directly. That is acceptable only if
outputs record enough dependency/provenance information for later incremental
maintenance.

Mitigation:

- add typed read dependencies now;
- keep context values rebuildable from `WorldModel` + `DefinitionRegistry`;
- avoid hidden mutable caches as the only source of projected truth.

## Research Summary

The current architecture and code are ready for Phase 7. The main work before
implementation is not another pre-refactor. It is a precise plan that treats
`world-context` as the actor-relative derived-context boundary:

```text
checked definitions and authoritative stores in
decision-safe actor context out
provenance and read dependencies alongside the output
no hard mutation, no final decision, no source compiler leakage
```

The strongest compiler lesson is not to build a compiler framework. It is to
make each representation and transformation honest about its inputs, outputs,
authority, reads, writes, provenance, invalidation, and diagnostics.

## References

Local:

- `docs/design/simulation-transition-compiler.md`
- `docs/architecture/runtime-pipeline.md`
- `docs/architecture/engine.md`
- `docs/architecture/crates.md`
- `docs/architecture/implementation-plan.md`
- `docs/design/capability-affordance-and-actor-interface.md`
- `docs/design/perception-and-observation.md`
- `docs/design/epistemic-state.md`
- `docs/design/semantic-appraisal-and-motivation.md`
- `docs/design/intent-templates-and-planning.md`
- `docs/research/runtime-pipeline-implementation-research.md`
- `docs/research/implementation-architecture-and-library-survey.md`
- `docs/research/world-representation-query-model.md`

External:

- [MLIR Language Reference](https://mlir.llvm.org/docs/LangRef/)
- [MLIR Pass Infrastructure](https://mlir.llvm.org/docs/PassManagement/)
- [MLIR Dialect Conversion](https://mlir.llvm.org/docs/DialectConversion/)
- [rustc-dev-guide: Incremental Compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html)
- [rustc-dev-guide: Salsa](https://rustc-dev-guide.rust-lang.org/queries/salsa.html)
- [rust-analyzer Architecture](https://rust-analyzer.github.io/book/contributing/architecture.html)
- [Differential Dataflow overview](https://www.microsoft.com/en-us/research/video/incremental-iterative-and-interactive-computation-using-differential-dataflow/)
- [Souffle Provenance](https://souffle-lang.github.io/provenance)
- [Rete: A Fast Algorithm for the Many Pattern/Many Object Pattern Match Problem](https://www.sciencedirect.com/science/article/pii/B9780934613538500418)
- [PDDL 2.1 paper](https://www.cs.cmu.edu/afs/cs/project/jair/pub/volume20/fox03a-html/JAIRpddl.html)
- [Rust API Guidelines: Type Safety](https://rust-lang.github.io/api-guidelines/type-safety.html)
- [Rust API Guidelines: Flexibility](https://rust-lang.github.io/api-guidelines/flexibility.html)
