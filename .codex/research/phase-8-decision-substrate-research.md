# Phase 8 Decision Substrate Research

## Purpose

This note records the research and architecture pressure for the Phase 8
implementation plan. It is not a final API reference and not a broad literature
review.

Phase 8 should build the typed substrate for configurable social-cognitive
decision work:

```text
ActorContextProjection
  -> checked decision profile and pass contracts
  -> typed decision artifacts
  -> decision trace skeleton
```

It should not yet build the full decision runner, benchmark runner, LLM
integration, source authoring DSL, or social-cognitive representation slice.

## Main Conclusion

Use compiler-shaped discipline, not a compiler framework.

The first implementation should create:

- concrete checked declarations;
- role/kind compatibility checks;
- pass contracts;
- decision profiles;
- trace artifact vocabulary;
- registry-style validation.

It should not create:

- a dynamic plugin host;
- an arbitrary workflow graph engine;
- a generic public `dyn Pass` execution API;
- a query cache or incremental engine;
- LLM or oracle execution;
- appraisal as the mandatory middle representation.

The strongest shape is a small `world-decision` registry, analogous to the
existing `world-defs` checked registry but owned by the decision crate:

```text
DecisionRegistry
  representations: RepresentationKindDef
  passes: DecisionPassContract
  profiles: DecisionProfile
```

`world-defs` continues to own action/process/effect definitions and broad
semantic declaration envelopes. `world-decision` owns decision-profile and
decision-artifact contracts because those contracts are consumed by decision
logic, not by the causal runtime.

## Local Architecture Pressure

The revised implementation roadmap says Phase 8 is `Decision Substrate`, not
`Semantic Appraisal`. This matters. Appraisal-like variables remain useful, but
they should be one standard representation family, not the kernel of the
decision architecture.

`docs/architecture/configurable-decision-pipeline.md` defines the durable
split:

```text
fixed authority kernel
  + configurable decision middle-end
```

The kernel owns hard truth, accepted non-hard gates, actor-relative access,
typed effects, causal commit, provenance, event records, profile validation,
and leakage checks. The configurable middle-end owns which actor-relative
context views, semantic representations, cognitive signals, implementation
modes, and candidate-selection stages are used.

`docs/design/simulation-transition-compiler.md` already gives the pass-contract
shape that should become concrete:

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

Phase 8 should translate that into decision-specific types without inventing a
large runtime pass manager.

The current code is ready for this:

- `world-context` already returns value-like `ActorContextProjection` plus
  `ContextProjectionReport`, read dependencies, provenance, status, and
  diagnostics.
- `world-defs` already has `DefinitionId`, `DefinitionName`,
  `VersionAnchor`, `SemanticDeclarationDef`, action definitions, registry
  validation, and descriptor precedent from primitive definitions.
- `world-decision` is currently empty and can be shaped cleanly.

## Compiler References

### MLIR Interfaces And Traits

MLIR interfaces provide a generic way for analyses/transforms to reason about
operations without hardcoding every dialect. Traits abstract common properties
across operation/type/attribute definitions.

Transfer to Phase 8:

- `RepresentationRole` should be the broad interface-like category.
- `RepresentationKindDef` should be the concrete dialect/kind declaration.
- Pass validation should match concrete kinds through declared roles, not
  through ad hoc string names or one blessed base type.
- Appraisal, speech acts, commitments, and other-model artifacts should be
  different representation families that can share roles where appropriate.

Do not transfer:

- dynamic dialect loading;
- TableGen-like code generation;
- a full MLIR-style operation hierarchy.

References:

- https://mlir.llvm.org/docs/Interfaces/
- https://mlir.llvm.org/docs/Traits/
- https://mlir.llvm.org/docs/DefiningDialects/

### MLIR Pass Management

MLIR pass infrastructure emphasizes a current operation/unit, declared
dependent dialects, analysis management, pass failure, pass registration,
instrumentation, and reproducer support.

Transfer to Phase 8:

- `DecisionPassContract` should declare input kinds/roles, output kinds/roles,
  allowed context reads, authority reads, forbidden writes, implementation
  modes, determinism, and trace obligations.
- Phase 8 should validate contracts before execution exists.
- Trace metadata should be designed as future pass instrumentation, not as an
  afterthought.

Do not transfer:

- an executable pass manager yet;
- dynamic textual pass pipeline parsing;
- pass plugins.

Reference:

- https://mlir.llvm.org/docs/PassManagement/

### LLVM New Pass Manager

LLVM's new pass manager separates pass execution from analysis management and
uses preserved analyses/invalidation to avoid stale analysis results.

Transfer to Phase 8:

- profile validation should know which artifacts each pass consumes and
  produces;
- pass contracts should declare invalidation/read dependencies even before a
  cache exists;
- decision traces should preserve enough artifact references for future
  invalidation and replay analysis.

Do not transfer:

- mutable cross-level analysis proxies;
- fine-grained invalidation implementation before there is a decision runner.

Reference:

- https://llvm.org/docs/NewPassManager.html

### rustc Queries And Salsa

rustc's incremental model stores query results plus a query dependency DAG.
Salsa's model similarly pushes deterministic derived computations with explicit
inputs.

Transfer to Phase 8:

- `DecisionArtifactRef` should be explicit, stable within a trace, and typed by
  representation kind;
- pass contracts should record read dependencies and output artifact kinds;
- deterministic derivation should be separated from LLM/oracle/external
  proposal modes.

Do not transfer:

- a database abstraction;
- automatic recomputation;
- putting mutable world/session state inside a query engine.

References:

- https://rustc-dev-guide.rust-lang.org/query.html
- https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html
- https://salsa-rs.github.io/salsa/overview.html

## Evaluation And Agent References

HELM argues for evaluating across scenarios and multiple metrics rather than a
single score. Phase 8 is not evaluation yet, but it must leave room for later
scenario/profile/metric comparison.

SOTOPIA shows the value of interactive social-intelligence evaluation, while
Concordia shows the neighboring space of LLM generative agent-based modeling.
`world` should differentiate through typed authority boundaries, actor-relative
context, causal commits, and process-level traces.

Transfer to Phase 8:

- `DecisionProfile` should be first-class because experiments compare
  profiles, not only agents;
- implementation mode should be explicit: rule, heuristic, LLM, hybrid, oracle,
  replay, disabled;
- oracle usage should be marked in the profile/trace vocabulary from the
  beginning;
- traces should distinguish typed artifact evidence from untrusted rationale.

References:

- https://arxiv.org/abs/2211.09110
- https://arxiv.org/abs/2310.11667
- https://arxiv.org/abs/2312.03664

## Rust Shape

Use concrete domain types first:

- private fields;
- constructor validation;
- non-exhaustive enums for extension pressure;
- `BTreeMap` / `BTreeSet` for deterministic ordering;
- thin `lib.rs` re-export facade;
- `thiserror` for a domain-rich `DecisionError`;
- no public trait execution surface in Phase 8.

A small descriptor trait may be useful later for standard representation or
pass declarations, similar to `EffectPrimitiveDescriptor`, but Phase 8 should
not require it unless implementation duplication appears immediately.

## Design Risks

### Too Dynamic Too Early

If Phase 8 builds a generic graph executor, profile validation and ablation
comparability become harder. Keep profiles static checked declarations.

### Appraisal Becomes Kernel

If Phase 8 starts from `Thought`, `Pressure`, and `GoalPressure`, future social
and cognitive structures become forced through appraisal. Start with role/kind
contracts instead.

### Trace Is Added Later

If trace shape is postponed, pass outputs may become hard to compare. Include
trace artifact references now, even if Phase 9 fills them during execution.

### Authoring Leaks In

If source diagnostics or parser ASTs enter Phase 8, `world-decision` becomes an
authoring crate. Keep source syntax deferred to Phase 13.

### Hidden Authority Through Convenience

Decision code must not gain raw model/runtime access. Phase 8 should only
accept actor-context values and checked declarations.
