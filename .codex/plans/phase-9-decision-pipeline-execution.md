# Phase 9 Plan: Decision Pipeline Execution And Trace

## Status

Draft implementation plan for Phase 9.

This plan turns the checked Phase 8 decision substrate into a small executable
profile runner. It should stay domain-shaped: execute static checked profiles,
record typed traces, and produce handoff artifacts without becoming a generic
workflow engine.

## Purpose

Phase 9 target:

```text
ActorContextProjection
  + DecisionRegistry
  + trusted pass executor registry
  -> DecisionRunner
  -> DecisionRunReport
     - trace
     - terminal artifact / request proposal / abstention / failure
```

The runner lets research code compare small decision profiles over the same
actor-context input. It does not select or mutate hard truth directly.

## Research Inputs

Primary local research:

- `.codex/research/phase-9-decision-pipeline-execution-research.md`
- `.codex/research/phase-8-decision-substrate-research.md`
- `.codex/research/phase-7-compiler-shaped-actor-context-research.md`

Primary local architecture/design:

- `docs/architecture/implementation-plan.md`
- `docs/architecture/configurable-decision-pipeline.md`
- `docs/architecture/crates.md`
- `docs/architecture/engine.md`
- `docs/architecture/runtime-pipeline.md`
- `docs/design/simulation-transition-compiler.md`
- `docs/design/perception-and-observation.md`
- `docs/design/epistemic-state.md`
- `docs/design/social-institutional-model.md`
- `docs/design/intent-templates-and-planning.md`
- `docs/research/cognitive-agent-research-map.md`
- `docs/research/social-strategic-evaluation-program.md`
- `docs/research/social-strategic-benchmark-methodology.md`

External patterns to apply selectively:

- MLIR pass management: static pass pipelines, pass failure, pass
  instrumentation, and trace-before/after/failure structure.
- LLVM new pass manager: explicit pass scope, analysis/dependency discipline,
  and validation after each pass.
- rustc/Salsa query systems: pure deterministic derivations with explicit
  dependency edges, while marking nondeterministic/LLM/oracle stages.
- W3C PROV and OpenTelemetry: entity/activity/provenance and span-like trace
  structure.
- HELM/SOTOPIA/Concordia: comparable profiles, social-agent traceability, and
  explicit LLM/oracle metadata.

## Baseline Decisions

### Static Checked Runner, Not Workflow Engine

Run the ordered steps already declared by `DecisionProfile`. Do not add a
dynamic graph executor, textual pipeline parser, plugin loader, scheduling
system, or general workflow DSL.

### Trusted Executor Boundary

Add a trait boundary for installed pass semantics. The trait is public because
`world-engine` and tests need to install executors, but it is not a security
boundary and not ordinary pack-authoring authority.

Executors receive a restricted decision context, not `WorldModel`,
`CausalRuntime`, or primitive staging APIs.

### Decision Artifacts Are Handoff Values, Not Authority

Artifacts can represent selected intent, activity plan, executable request
candidate, non-hard proposal, diagnostic output, or intermediate
social-cognitive values. They still do not commit hard truth or accepted
non-hard truth.

### Explicit Terminal Output

Do not infer profile output from the last pass by convention. Add a profile
exit declaration so comparable profiles can state which terminal role/kind
counts as the run outcome and whether abstention is allowed.

### Metadata-Only LLM And Oracle Support

Phase 9 should represent LLM, hybrid, nondeterministic, seeded, replay, and
oracle execution metadata. It should not call external models or oracle
providers.

## Proposed Module Structure

Keep `world-decision` as the owner of declarations, execution, and trace
vocabulary:

```text
crates/world-decision/src/
  lib.rs
  error.rs
  pass.rs
  profile.rs
  representation.rs

  registry/
    mod.rs
    builder.rs
    flow.rs
    validate.rs

  trace/
    mod.rs
    artifact.rs
    builder.rs
    metadata.rs
    step.rs

  runner/
    mod.rs
    artifact_store.rs
    context.rs
    executor.rs
    registry.rs
    request.rs
    report.rs
    resolve.rs
    validate.rs

  tests/
    mod.rs
    helpers.rs
    representation.rs
    pass.rs
    profile.rs
    registry.rs
    trace.rs
    runner.rs
    guardrails.rs
```

`lib.rs` remains a thin facade. The new `runner/` module exposes only the
runner, request/report, executor trait, executor registry, and domain value
types needed to install and run trusted executors.

`trace.rs` may be split into `trace/` because Phase 9 adds enough trace
builder, input-ref, status, and metadata vocabulary that a flat file will stop
being clear.

## Core Type Shape

These are target shapes, not exact final APIs.

### Profile Exit

```rust
pub struct DecisionProfileExit {
    output: Option<DecisionProfileOutput>,
    abstention_allowed: bool,
}

pub struct DecisionProfileOutput {
    role: RepresentationRole,
    kind: Option<DefinitionId>,
}
```

Rules:

- `output` is a single terminal handoff, unless the profile only allows abstention;
- hard proposals remain forbidden by representation authority rules;
- exit output role/kind must be satisfiable by context/pass flow validation;
- ambiguous terminal handoff is rejected by profile flow validation.

`DecisionProfile::new` should gain an exit argument. Tests can use helpers to
keep constructors readable.

### Input References

```rust
pub enum DecisionInputRef {
    Context(ContextProjectionKind),
    Artifact(DecisionArtifactRef),
}
```

Use this in trace steps and runner-resolved inputs. This avoids fake artifact
ids for actor-context projections.

### Artifact Envelope

```rust
pub struct DecisionArtifact {
    record: DecisionArtifactRecord,
    payload: DecisionArtifactPayload,
}

pub struct DecisionArtifactPayload {
    // Opaque runtime-local payload wrapper.
}
```

Recommended implementation:

- use `Arc<dyn DecisionArtifactBody>` internally for cheap cloning;
- provide `marker()` for tests and metadata-only passes;
- provide `downcast_ref<T>()` for trusted representation-family executors;
- do not serialize payloads or treat them as accepted authority.

The runner validates artifact metadata against `RepresentationKindDef`; the
payload belongs to the representation family.

### Executor Trait

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

Executor registry:

```rust
pub struct DecisionExecutorRegistry {
    executors: BTreeMap<(DefinitionId, ImplementationMode), Box<dyn DecisionPassExecutor>>,
}
```

Rules:

- duplicate `(pass_id, mode)` registration fails;
- a missing executor for a non-disabled profile step is recorded as a failed
  run report;
- executor registration is checked against the `DecisionRegistry` before run or
  at first execution;
- no executor gets mutation authority through the decision API.

### Restricted Execution Context

```rust
pub struct DecisionPassExecutionContext<'a> {
    profile: &'a DecisionProfile,
    pass: &'a DecisionPassContract,
    mode: ImplementationMode,
    actor_context: DecisionContextView<'a>,
    inputs: &'a [ResolvedDecisionInput],
    artifacts: &'a DecisionArtifactStore, // internal, only input-scoped lookup is exposed
}
```

`DecisionContextView` exposes projection families only when both the profile
declared them and the pass contract allows them:

```rust
fn observations(&self) -> Option<&ObservationContext>;
fn epistemic(&self) -> Option<&EpistemicWorkingSet>;
fn social(&self) -> Option<&SocialContextView>;
fn capabilities(&self) -> Option<&CapabilitySet>;
fn repertoire(&self) -> Option<&ActionRepertoire>;
fn affordances(&self) -> Option<&[PerceivedAffordance]>;
```

Do not expose `&ActorContextProjection` directly.
Do not expose the full artifact store through the execution context. Executors
can inspect only runner-issued `ResolvedDecisionInput` handles.

### Execution Result

```rust
pub struct DecisionPassExecution {
    disposition: DecisionPassDisposition,
    outputs: Vec<ProducedDecisionArtifact>,
    diagnostics: Vec<DecisionPassDiagnostic>,
    metadata: DecisionExecutionMetadata,
}

pub enum DecisionPassDisposition {
    Completed,
    Abstained,
}
```

Executor failures use `Result::Err`, but once a checked run has started the
runner records them as failed trace steps and returns a failed report.
Intentional no-action uses `Abstained`.

### Execution Metadata

```rust
pub struct DecisionExecutionMetadata {
    mode: ImplementationMode,
    determinism: DeterminismPolicy,
    seed: Option<DecisionRunSeed>,
    model: Option<ModelInvocationMetadata>,
    oracle: Option<OracleInvocationMetadata>,
    replay: Option<ReplayInvocationMetadata>,
}
```

Validation rules:

- `Rule` and deterministic `Heuristic` runs should not require model metadata;
- `Seeded` determinism requires a seed;
- `Llm` and `Hybrid` modes require model/prompt/sampling metadata;
- `Oracle` mode requires oracle metadata and compatible profile oracle policy;
- `Replay` mode requires replay source metadata;
- `Disabled` steps are skipped by the runner and do not call executors.

### Trace Builder

```rust
pub struct DecisionTraceBuilder { ... }
```

Responsibilities:

- allocate `DecisionArtifactRef`;
- record context/artifact input refs;
- record outputs and diagnostics;
- record verifier result and execution metadata;
- reject steps that refer to missing artifacts;
- reject duplicate artifacts;
- finalize as completed, abstained, or failed.

`DecisionTraceStatus` should grow beyond `Started/Completed/Failed` with an
`Abstained` variant. Step records should also distinguish completed, skipped,
abstained, and failed steps.

### Runner Request And Report

```rust
pub struct DecisionRunRequest<'a> {
    profile: DefinitionId,
    actor_context: &'a ActorContextProjection,
}

pub struct DecisionRunReport {
    outcome: DecisionRunOutcome,
    trace: DecisionTrace,
    artifacts: DecisionArtifactStore,
}

pub enum DecisionRunOutcome {
    TerminalArtifact(DecisionArtifactRef),
    Abstained,
    Failed,
}
```

The terminal artifact can be classified by its `RepresentationRole` and
`RepresentationAuthority`:

- `ExecutableRequest` means a later engine facade may submit a runtime request;
- `NonHardUpdateProposal` means a later engine facade may route it to an
  accepted non-hard gate;
- `Choice`, `IntentCandidate`, or `ActivityPlan` are decision-level handoffs;
- `Diagnostic` cannot be a normal terminal output unless explicitly allowed by
  the profile exit.

Reports own the run-local artifact store. Public access is read-only, with
`artifact(ref)` and `terminal_artifact()` as the primary handoff APIs.

## Flow Algorithm

At a high level, `DecisionRunner::run` should:

1. Look up the profile in `DecisionRegistry`.
2. Create `DecisionTraceHeader::from_projection`.
3. Initialize a flow state with profile context inputs.
4. For each profile step:
   - look up pass contract;
   - if mode is `Disabled`, record skipped step and continue;
   - resolve each pass input to `DecisionInputRef` values according to
     `InputBinding`;
   - create `DecisionContextView` restricted by profile context inputs and
     pass `allowed_context`;
   - find matching executor by `(pass_id, mode)`;
   - call executor;
   - validate execution metadata against mode and determinism;
   - if executor abstained, validate profile allows abstention, finalize trace,
     and return `DecisionRunOutcome::Abstained`;
   - validate output count, role, kind, authority, and duplicate constraints
     against the pass contract and registry;
   - allocate artifact refs, insert artifact store entries, and record step;
   - make produced artifacts available to downstream inputs.
5. Resolve the profile exit output from available artifacts.
6. Finalize the trace and return the terminal artifact report.

## Implementation Sequence

### 1. Refactor Trace Vocabulary

- Move `trace.rs` into `trace/`.
- Add `DecisionInputRef`.
- Add trace step status and verifier result types.
- Add execution metadata records.
- Add `DecisionTraceStatus::Abstained`.
- Preserve existing public re-exports from `lib.rs`.

Tests:

- input refs can represent context and artifact inputs;
- trace rejects duplicate artifact refs;
- trace builder rejects missing artifact refs;
- status and diagnostics remain value-like.

### 2. Add Profile Exit Contract

- Add `DecisionProfileExit` and `DecisionProfileOutput`.
- Update `DecisionProfile`.
- Update profile helpers/tests.
- Extend registry validation so profile exit is satisfiable by the profile
  flow.

Tests:

- missing terminal output fails registry build;
- ambiguous terminal output fails;
- terminal output can target executable request, non-hard proposal, choice, or
  intent-like artifact roles;
- diagnostic terminal output is rejected unless explicitly declared.

### 3. Extract Shared Flow Resolution

- Move context-role mapping and available-representation tracking out of
  `registry/validate.rs` into a reusable internal `registry/flow.rs`.
- Keep static validation and runtime resolution aligned.
- Support context sources and artifact sources in one representation-flow
  model.

Tests:

- existing profile validation tests continue to pass;
- role-only ambiguity behavior is unchanged;
- `AllAvailable` still allows multiple sources;
- disabled pass outputs are not available downstream.

### 4. Add Runner Types

- Add `runner/request.rs`.
- Add `runner/report.rs`.
- Add `runner/context.rs`.
- Add `runner/artifact_store.rs`.
- Add `runner/executor.rs`.
- Add `runner/registry.rs`.

Keep `DecisionRunner` small and facade-like.

Tests:

- duplicate executor registration fails;
- missing executor fails the run with a failed trace;
- executor registered for a pass/mode not in the decision registry fails
  preflight or run validation.

### 5. Add Restricted Context Access

- Implement `DecisionContextView`.
- Expose only explicitly permitted projection accessors.
- Avoid direct `ActorContextProjection` exposure to executors.

Tests:

- a pass allowed only `Observation` sees observations and not social/epistemic
  context through normal API;
- a pass with `ActorRelativeView` and `AllAvailable` can receive multiple
  declared context input refs without receiving hidden model access.

### 6. Implement Runner Execution

- Implement `DecisionRunner::new`.
- Implement `DecisionRunner::run`.
- Add contract validation around executor outputs and metadata.
- Record successful, skipped, abstained, and failed steps.
- Return terminal outcome by explicit profile exit.

Tests:

- one-step context-to-choice profile runs;
- two-step profile passes artifact output into the next step;
- `Disabled` pass is skipped and downstream required input fails if no
  alternate source exists;
- `AnyOf` and `AllAvailable` behavior matches registry validation;
- executor output with wrong kind/role fails;
- executor output not declared by pass fails;
- pass failure records failed trace status;
- abstention returns an abstained report only when profile allows it.

### 7. Add Metadata Validation

- Add seed/model/oracle/replay metadata structs.
- Validate metadata against `ImplementationMode` and `DeterminismPolicy`.
- Preserve metadata in trace steps.

Tests:

- seeded pass without seed fails;
- LLM/hybrid pass without model metadata fails;
- oracle mode without oracle metadata fails;
- oracle mode under `ProfileOraclePolicy::Forbid` fails before executor call
  or at metadata validation;
- process records metadata without adding LLM/oracle provider dependencies.

### 8. Guardrails And Docs

- Keep the dependency guardrail current after module moves.
- Add tests that `world-decision` still does not import privileged crates.
- Update crate docs/rustdoc for runner and executor trust boundary.
- Avoid broad architecture rewrites.

## Test Plan

Run:

```bash
cargo fmt --all
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
git diff --check
```

Additional checks:

```bash
rg -n "world_model|world_runtime|world_standard_runtime" crates/world-decision/src
rg -n "Phase 9|temporary|handoff" crates/world-decision/src
```

The second command should not find planning-stage comments in production code.
Domain terms like `handoff` are acceptable only if they describe the actual
decision-output boundary, not project-management state.

## Acceptance Criteria

- `world-decision` can run a checked static decision profile over an
  `ActorContextProjection`.
- The runner records context input refs, artifact refs, step status,
  diagnostics, verifier result, implementation mode, and execution metadata.
- Terminal output is explicit through profile exit configuration.
- Disabled, abstained, failed, deterministic, seeded, LLM, hybrid, replay, and
  oracle-shaped steps have distinct metadata/status handling.
- Decision executors cannot access model/runtime authority through normal
  decision APIs.
- No dependency edge from `world-decision` to `world-model`, `world-runtime`,
  `world-standard-runtime`, or `world-engine` is added.
- No LLM provider, oracle provider, persistence backend, or scenario runner is
  introduced.
- Existing Phase 8 validation behavior is preserved unless explicitly expanded
  by profile exit validation.

## Deferred Work

Phase 10:

- concrete social-cognitive representation payload schemas;
- typed speech, commitment, other-model, motivation, and strategic assessment
  representation families;
- representation-family verifiers beyond substrate output checks.

Phase 11:

- engine facade that projects context, runs decision profiles, submits runtime
  requests, drains work, and exports traces.

Phase 12:

- scenario/evaluation substrate and benchmark run traces.

Phase 13:

- source authoring for pass contracts, decision profiles, and scenario
  declarations.

## Assumptions

- The logical crate separation remains correct.
- `world-decision` owns decision execution over actor-context values.
- `world-engine` will later orchestrate decision results with runtime
  execution.
- The executor trait is trusted infrastructure, not a sandbox.
- Concrete artifact schemas should wait for the social-cognitive
  representation slice.
