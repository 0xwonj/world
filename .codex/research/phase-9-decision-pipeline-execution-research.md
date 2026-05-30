# Phase 9 Decision Pipeline Execution Research

## Purpose

This memo supports the Phase 9 implementation plan. It records the research and
architecture pressure for making checked decision profiles executable and
traceable without turning `world-decision` into a generic workflow engine or
giving decision code mutation authority.

Phase 9 should build:

```text
ActorContextProjection
  + DecisionRegistry
  + trusted pass executors
  -> checked static profile run
  -> typed artifact handoff
  -> decision trace
```

It should not yet build:

- concrete social-cognitive representation families;
- LLM provider adapters;
- oracle provider adapters;
- engine session orchestration;
- scenario/evaluation runners;
- source authoring;
- persistence or replay storage.

## Local State

The current roadmap defines Phase 9 as `Decision Pipeline Execution And Trace`.
The exit condition is that the same actor-context input can run through small
comparable decision profiles and produce an explanatory trace without granting
decision code mutation authority.

The implemented Phase 8 substrate already provides:

- `DecisionRegistry` for representation kinds, pass contracts, and profiles;
- `DecisionPassContract` with input/output roles, context access,
  implementation modes, determinism, write policy, and trace policy;
- `DecisionProfile` with static ordered steps and oracle policy;
- `RepresentationKindDef` with role, visibility, persistence, and authority
  metadata;
- `DecisionTrace`, `DecisionTraceStep`, `DecisionArtifactRecord`, and
  trace-local artifact refs;
- profile validation for missing inputs, unsupported modes, ambiguous role-only
  inputs, disallowed context access, oracle labeling, and authority/write-policy
  mismatch;
- a dependency guardrail keeping `world-decision` behind `world-context` and
  out of `world-model`, `world-runtime`, and `world-standard-runtime`.

The main gap is that Phase 8 validates profile shape but does not execute a
profile. Phase 9 should add a small runner and trace builder while reusing the
existing registry contracts.

## Main Conclusion

Use a compiler-style static pass runner, not a workflow engine.

The runner should execute the profile order already validated by
`DecisionRegistry`. It should resolve typed inputs, call trusted executors
through a narrow context API, validate executor outputs against pass contracts,
record verifier results and execution metadata, and return a typed handoff.

The runner should not own world mutation, query the model, submit runtime
requests, or run arbitrary user code with privileged handles.

The strongest shape is:

```text
DecisionRunner
  borrows DecisionRegistry
  owns/borrows DecisionExecutorRegistry
  consumes ActorContextProjection by reference
  resolves context/artifact inputs
  runs pass executors in profile order
  records trace with context refs, artifact refs, metadata, diagnostics
  returns DecisionRunReport
```

## Compiler Infrastructure Lessons

### MLIR Pass Management

MLIR's pass infrastructure is useful because it separates pass declarations,
pass pipelines, pass failures, pass timing, analysis computation, and
instrumentation. MLIR also exposes before/after/failure instrumentation hooks
around pass execution and analysis computation.

Relevant reference:

- https://mlir.llvm.org/docs/PassManagement/

Transfer to Phase 9:

- the profile is the checked pass pipeline;
- the runner should be small and static;
- pass execution should record before/after/failure-shaped trace events;
- trace building should be centralized rather than hand-built at every call
  site;
- runner failures should point to the profile step, pass id, and failed
  contract;
- timing/export hooks can be left for later, but the trace shape should be
  instrumentation-ready.

Do not transfer:

- dynamic textual pass-pipeline parsing;
- multi-threaded pass scheduling;
- TableGen-style pass declarations;
- mutable IR rewriting machinery.

### LLVM New Pass Manager

LLVM's new pass manager separates pass execution from analysis managers and
requires passes to communicate which analyses they preserve or invalidate.
It also makes nesting and pass scope explicit.

Relevant reference:

- https://llvm.org/docs/NewPassManager.html

Transfer to Phase 9:

- keep pass scope explicit: Phase 9 passes run over one actor-context decision
  run, not over `WorldModel` or `CausalRuntime`;
- treat context projection and prior artifacts as the only runner inputs;
- record dependencies and metadata for future invalidation/replay;
- validate outputs after each pass before making them available downstream.

Do not transfer:

- nested analysis-manager proxies;
- mutable analysis invalidation machinery;
- cross-IR adaptors.

### rustc Queries And Salsa

rustc's query model and Salsa both emphasize pure computations, explicit
dependencies, and dependency graphs for incremental recomputation. rustc's
incremental compilation records which queries depend on which inputs and can
reuse cached results when those inputs have not changed.

Relevant references:

- https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html
- https://rustc-dev-guide.rust-lang.org/queries/salsa.html

Transfer to Phase 9:

- deterministic decision passes should behave like pure derivations of
  actor-context inputs and prior artifacts;
- trace records should capture dependency edges: context input refs and artifact
  refs;
- nondeterministic, LLM, replay, and oracle modes must be explicitly marked so
  future replay/evaluation does not treat them as pure deterministic queries;
- Phase 9 should avoid a query database, but should leave artifact dependency
  structure clean enough for one later.

Do not transfer:

- automatic incremental recomputation;
- global query storage;
- placing world/session mutation inside a query engine.

## Trace And Provenance Lessons

### PROV

W3C PROV defines provenance as information about entities, activities, and
people involved in producing data, useful for assessing quality, reliability,
and trustworthiness. It also emphasizes derivation, versioning,
reproducibility, and procedure representation.

Relevant reference:

- https://www.w3.org/TR/prov-overview/

Transfer to Phase 9:

- model context projections and artifacts as produced entities;
- model pass execution as activities;
- model executor/policy metadata as agent-like attribution;
- make derivation edges explicit in trace input refs;
- keep provenance compact and engine-native, not full PROV serialization.

### OpenTelemetry Traces

OpenTelemetry models traces as DAGs of spans, with span attributes, events,
links, and parent/child relationships.

Relevant reference:

- https://opentelemetry.io/docs/specs/otel/overview/

Transfer to Phase 9:

- a decision run should be one trace;
- each pass execution is a step/span-like record;
- context refs and artifact refs are links/dependencies;
- metadata should be structured, not embedded in diagnostic strings;
- timing and export can wait, but step status should distinguish success,
  skip, abstention, and failure.

Do not transfer:

- distributed tracing IDs;
- network propagation;
- external telemetry dependencies.

## Evaluation And Agent Lessons

### HELM

HELM argues for evaluating models across scenarios and metrics, standardizing
conditions, exposing tradeoffs, and releasing raw prompts/completions for
transparency.

Relevant reference:

- https://arxiv.org/abs/2211.09110

Transfer to Phase 9:

- decision traces should preserve enough metadata for later comparison across
  profiles and implementation modes;
- LLM and sampling metadata belong in the trace vocabulary before provider
  integration exists;
- raw text can be recorded later, but should not become authority or hidden
  state.

### SOTOPIA

SOTOPIA evaluates social intelligence through multi-agent role-play scenarios
and holistic social-interaction evaluation.

Relevant reference:

- https://arxiv.org/abs/2310.11667

Transfer to Phase 9:

- decision profiles are experimental conditions, not merely implementation
  knobs;
- traces must expose social-cognitive process evidence, not only final action
  outcomes;
- oracle and normal actor-facing runs must be separable from the start.

### Concordia

Concordia is a neighboring generative agent-based modeling library. It uses
LLMs, memory, grounded actions, and a game-master-like environment component.

Relevant reference:

- https://arxiv.org/abs/2312.03664

Transfer to Phase 9:

- the project should support flexible agent behavior, but differentiate through
  typed authority boundaries and traceable pass contracts;
- natural-language action attempts can be a future policy input, but Phase 9
  should return typed artifacts and requests/proposals rather than rely on a
  game-master translation layer hidden inside the decision runner.

## Architecture Implications

### 1. Add A Trusted Executor Boundary

Phase 9 needs a trait boundary for trusted pass semantics:

```rust
pub trait DecisionPassExecutor {
    fn pass_id(&self) -> DefinitionId;
    fn mode(&self) -> ImplementationMode;
    fn execute(
        &self,
        context: DecisionPassExecutionContext<'_>,
    ) -> Result<DecisionPassExecution, DecisionError>;
}
```

This is an engine-installed execution boundary, not an ordinary pack-authoring
API and not a security sandbox. The important protection is that the context
does not expose `WorldModel`, `CausalRuntime`, or commit authority.

### 2. Use A Restricted Pass Context

Executors should not receive `&ActorContextProjection` directly. They should
receive a restricted context that exposes:

- pass contract metadata;
- selected implementation mode;
- resolved input refs;
- artifact payload lookup for resolved artifact refs;
- actor-context projection families allowed by the pass contract and profile;
- context provenance/report metadata;
- no world model, no runtime, no primitive staging context.

This prevents accidental use of disallowed context families through convenience
accessors.

### 3. Add Context Input Refs

Current `DecisionTraceStep.inputs` stores only `DecisionArtifactRef`. That is
not enough because first-step inputs come from actor-context projections, which
do not have concrete `RepresentationKindDef` ids.

Phase 9 should add:

```rust
pub enum DecisionInputRef {
    Context(ContextProjectionKind),
    Artifact(DecisionArtifactRef),
}
```

The runner can then record both actor-context inputs and prior artifact inputs
without inventing fake representation ids.

### 4. Add A Trace Builder

`DecisionTrace::from_parts` is useful for low-level tests but the runner should
not hand-assemble traces. A `DecisionTraceBuilder` should:

- allocate trace-local artifact refs;
- reject duplicate artifacts;
- reject steps that reference missing artifacts;
- ensure step outputs are produced by the step being recorded;
- finalize as completed, abstained, or failed;
- centralize future instrumentation metadata.

### 5. Add Runtime Artifact Storage

The registry verifies representation kinds, but execution needs values. Phase 9
should introduce an in-memory `DecisionArtifact` envelope:

```rust
DecisionArtifactRecord
DecisionArtifactPayload
```

The runner should understand only the envelope metadata. Concrete payload
schemas belong to representation families in later phases. A small opaque,
type-erased payload wrapper is acceptable if it is clearly runtime-local,
trusted, and non-authoritative. Marker payloads are enough for Phase 9 smoke
tests.

### 6. Add A Profile Exit Contract

The current profile validates pass flow but does not say which terminal
artifact is the decision handoff. Phase 9 should add a small profile exit
declaration:

```rust
DecisionProfileExit
  terminal_outputs: allowed terminal role/kind specs
  abstention_allowed: bool
```

The runner should not infer the final decision from "last output" when multiple
profiles are meant to be comparable. Explicit exits make profile comparison
and trace interpretation cleaner.

### 7. Keep LLM/Oracle As Metadata-Only

Phase 9 should not add LLM clients, prompt templates, oracle providers, or
replay stores. It should add enough metadata types and validation to prevent
future ambiguity:

- LLM/hybrid mode must carry model/prompt/sampling metadata in executor output;
- seeded mode must carry a seed;
- oracle mode must carry oracle metadata and profile oracle labeling;
- disabled mode records a skipped step and produces no outputs.

### 8. Preserve Crate Direction

`world-decision` should continue depending only on:

```text
world-core
world-defs
world-context
thiserror
```

The runner lives in `world-decision` because it executes decision declarations
over actor-context values. `world-engine` will later orchestrate context
projection, decision runs, runtime submission, and trace export.

## Risks And Mitigations

### Risk: Executor Trait Becomes A Plugin Framework

Mitigation:

- document it as a trusted engine-installed boundary;
- do not add dynamic loading;
- do not pass mutation authority into the context;
- keep registry installation explicit and in-memory.

### Risk: Artifact Payload Becomes Untyped State

Mitigation:

- keep the runner type-checking against `RepresentationKindDef`;
- treat payloads as representation-family owned;
- forbid payloads from granting authority;
- keep raw text/rationale non-authoritative unless a future typed verifier
  turns it into a checked artifact.

### Risk: Trace Records Are Too Weak For Evaluation

Mitigation:

- record context refs, artifact refs, implementation mode, metadata,
  diagnostics, verifier result, and final outcome;
- preserve profile id/version and context read/provenance metadata;
- do not rely on diagnostic strings for semantics.

### Risk: Hidden Truth Leakage Through Context API

Mitigation:

- pass executors receive a restricted context view;
- runner resolves context access from `DecisionProfile.context_inputs` and
  `DecisionPassContract.allowed_context`;
- tests should prove a pass cannot access disallowed context through the normal
  context API.

## Recommended Phase 9 Scope

Implement:

- `runner/` module in `world-decision`;
- executor registry and executor trait;
- restricted pass execution context;
- input resolver shared with or aligned to profile validation;
- context/artifact input refs;
- trace builder;
- in-memory artifact store;
- metadata and verifier result capture;
- explicit profile exit contract;
- runner report and outcome handoff;
- focused tests for successful execution, skip, abstention, failure,
  metadata, context gating, output mismatch, and dependency guardrails.

Defer:

- concrete speech, commitment, other-model, motivation, and strategic
  representation schemas;
- actual LLM/oracle/replay providers;
- runtime request submission;
- engine session facade;
- scenario and benchmark runners;
- source authoring and diagnostics.
