# Phase 8 Plan: Decision Substrate

## Status

Draft implementation plan for Phase 8.

This plan is intentionally concrete about architecture, module shape, important
types, validation rules, and tests. It is not a final API reference and does
not implement the decision runner, social-cognitive representation slice, or
authoring DSL.

## Purpose

Build the checked decision-substrate layer in `world-decision`.

The phase target is:

```text
ActorContextProjection + checked decision declarations
  -> DecisionRegistry validation
  -> profile/pass/representation compatibility
  -> trace artifact vocabulary
```

Phase 8 should make later configurable decision pipelines possible without
hardcoding appraisal as the only middle representation and without giving
decision code mutation authority.

## Research Inputs

Primary local research:

- `.codex/research/phase-8-decision-substrate-research.md`
- `.codex/research/phase-7-compiler-shaped-actor-context-research.md`
- `.codex/research/phase-6-standard-primitive-semantics-research.md`

Primary local architecture/design:

- `docs/architecture/implementation-plan.md`
- `docs/architecture/configurable-decision-pipeline.md`
- `docs/architecture/crates.md`
- `docs/architecture/engine.md`
- `docs/architecture/runtime-pipeline.md`
- `docs/architecture/implementation-execution-contract.md`
- `docs/design/simulation-transition-compiler.md`
- `docs/design/perception-and-observation.md`
- `docs/design/epistemic-state.md`
- `docs/design/social-institutional-model.md`
- `docs/design/intent-templates-and-planning.md`
- `docs/design/semantic-appraisal-and-motivation.md`

External patterns to apply selectively:

- MLIR interfaces/traits: roles as broad compatibility interfaces over
  concrete representation kinds.
- MLIR pass management: explicit pass contracts, pass failure vocabulary, and
  instrumentation-ready trace shape.
- LLVM pass manager: analysis dependency and preservation discipline, but not
  the full manager machinery.
- rustc/Salsa query systems: explicit artifact references and dependency
  recording without adding a query database.
- HELM/SOTOPIA/Concordia: profile and trace metadata should support later
  social-agent evaluation and comparisons.

## Baseline Decisions

### Decision Declarations Live In `world-decision`

`world-defs` remains the parser-free checked-definition crate for actions,
processes, typed effect programs, primitive definitions, and broad semantic
declaration envelopes.

Phase 8 should not move decision-profile-specific declarations into
`world-defs`. `world-decision` owns:

- representation kinds used by decision passes;
- pass contracts;
- decision profiles;
- decision traces;
- profile validation.

Later `world-authoring` can produce these checked `world-decision`
declarations because `world-authoring` is allowed to depend on
`world-decision`.

### Static Checked Profiles First

Implement static checked declarations and registry validation, not an
executable pass manager.

Do not add:

- dynamic pass graph execution;
- public `dyn DecisionPass`;
- plugin loading;
- LLM calls;
- oracle providers;
- source parser or source diagnostics;
- query cache or incremental database.

Phase 9 can execute small static profiles once Phase 8 has checked contracts.

### Appraisal Is One Representation Family

Do not make `Thought`, `Pressure`, or `GoalPressure` the root substrate.

Phase 8 should define role/kind machinery that can later host:

- typed speech;
- commitment candidates;
- bounded other-model views;
- strategic assessments;
- motivational/appraisal-like signals;
- executable request candidates;
- non-hard update proposals.

### Decision Artifacts Are Not Authority

Decision artifacts are typed trace/proposal values. They do not mutate hard
truth and do not create accepted social, epistemic, appraisal, chronology, or
runtime-control records.

If a future pass wants to persist non-hard state, the artifact is only a
proposal routed to the proper accepted update gate.

### Reuse Actor Context, Do Not Re-query The Model

`world-decision` should consume `world-context` outputs and checked decision
declarations. It should not depend on `world-model`, `world-runtime`, or
`world-standard-runtime`.

The input boundary should be value-like:

```text
ActorContextProjection
DecisionRegistry
DecisionProfile
```

not:

```text
WorldModel
CausalRuntime
QueryLayer
PrimitiveSemanticsRegistry
```

## Proposed Module Structure

```text
crates/world-decision/src/
  lib.rs
  error.rs
  representation.rs
  pass.rs
  profile.rs
  registry/
    mod.rs
    builder.rs
    validate.rs
  trace.rs
  tests/
    mod.rs
    helpers.rs
    representation.rs
    pass.rs
    profile.rs
    registry.rs
```

`lib.rs` should remain a thin re-export facade.

No `tests/` integration test is required unless Phase 8 exposes a public
black-box behavior that is better tested outside the crate. Most Phase 8 tests
can live in `src/tests/` because they protect constructor and registry
invariants without forcing helper APIs public.

## Core Types

### `RepresentationRole`

Broad compatibility category used by profile and pass validation.

Initial variants:

```text
ActorRelativeView
ObservationView
EpistemicView
SocialContextView
ActionRepertoire
CapabilitySet
AffordanceView
SpeechSurface
SpeechAct
DecisionSignal
MotivationalSignal
StrategicAssessment
OtherModelView
CommitmentCandidate
IntentCandidate
Choice
ActivityPlan
ExecutableRequest
NonHardUpdateProposal
Diagnostic
```

Use `#[non_exhaustive]` so later dialects can grow without pretending the
first list is complete.

### `RepresentationKindDef`

Concrete decision artifact kind.

Recommended fields:

```text
id: DefinitionId
name: DefinitionName
roles: BTreeSet<RepresentationRole>
visibility: RepresentationVisibility
persistence: RepresentationPersistence
authority: RepresentationAuthority
version: VersionAnchor
```

Important rules:

- roles must be non-empty;
- a kind may have multiple roles;
- actor-facing visibility must be explicit;
- proposal kinds must declare which authority class they propose toward;
- diagnostic-only kinds must not satisfy executable or proposal requirements.

### `RepresentationVisibility`

Controls whether the artifact is actor-facing or only engine/research-facing.

Initial variants:

```text
ActorVisible
EngineInternal
ResearchTrace
OracleOnly
DiagnosticOnly
```

`OracleOnly` should force profile/trace oracle labeling later.

### `RepresentationPersistence`

Describes artifact lifetime:

```text
Ephemeral
TraceRecorded
ProposalOnly
AcceptedElsewhere
```

No Phase 8 artifact is committed authority by itself.

### `RepresentationAuthority`

Describes authority relation without granting authority:

```text
Derived
ProposalTo(AuthorityClass)
ExecutableRequest
ControlProposal
Diagnostic
Oracle
```

`ProposalTo` is only a declaration that later execution must route through the
proper gate.

### `PassClass`

Architecture label for validation, diagnostics, and later execution.

Initial variants:

```text
ContextDerivation
SemanticGrounding
CognitiveSignal
OtherModeling
CandidateGeneration
Choice
ActivityBinding
ExecutionRequest
Validation
Publication
Diagnostic
```

Phase 8 should not model these as trait inheritance roots.

### `ImplementationMode`

Pass implementation mode for ablations and trace metadata:

```text
Rule
Heuristic
Llm
Hybrid
Oracle
Replay
Disabled
```

Use explicit `Oracle` rather than overloading `Llm` or `Hybrid`.

### `DeterminismPolicy`

Metadata for later trace/replay:

```text
Deterministic
Seeded
ExternalNondeterministic
Oracle
```

`Llm`, `Hybrid`, and `Oracle` modes should require a non-deterministic or
oracle-compatible policy in profile validation.

### `RepresentationInput` And `RepresentationOutput`

Pass edges should distinguish broad roles from concrete kinds.

Recommended shape:

```text
RepresentationInput {
  role: RepresentationRole,
  kind: Option<DefinitionId>,
  requirement: InputRequirement,
}

RepresentationOutput {
  role: RepresentationRole,
  kind: DefinitionId,
}
```

`InputRequirement`:

```text
Required
Optional
AnyOf(group)
```

Keep `AnyOf` minimal if it adds too much complexity. Required and optional are
enough for the first implementation.

### `DecisionPassContract`

Checked pass declaration.

Recommended fields:

```text
id: DefinitionId
name: DefinitionName
class: PassClass
inputs: Vec<RepresentationInput>
outputs: Vec<RepresentationOutput>
allowed_context: BTreeSet<ContextProjectionKind>
allowed_authority_reads: BTreeSet<AuthorityClass>
forbidden_authority_reads: BTreeSet<AuthorityClass>
write_policy: PassWritePolicy
implementation_modes: BTreeSet<ImplementationMode>
determinism: DeterminismPolicy
trace_policy: TracePolicy
version: VersionAnchor
```

Validation rules:

- outputs must be non-empty unless the pass class is explicitly diagnostic or
  validation-only;
- every output kind must exist and provide the declared role;
- input kind, when specified, must exist and provide the declared role;
- implementation modes must be non-empty;
- hard writes are never allowed;
- non-hard writes are proposal-only in Phase 8;
- oracle modes require oracle visibility or oracle trace labeling.

### `PassWritePolicy`

Initial variants:

```text
None
ProposalOnly(BTreeSet<AuthorityClass>)
ExecutableRequestOnly
ControlProposalOnly
DiagnosticOnly
```

Do not add a general write-capability object.

### `TracePolicy`

Initial fields:

```text
record_inputs: bool
record_outputs: bool
record_diagnostics: bool
record_provenance: bool
record_model_metadata: bool
```

Phase 8 only validates trace requirements. Phase 9 fills trace entries during
execution.

### `DecisionProfile`

Checked profile declaration.

Recommended fields:

```text
id: DefinitionId
name: DefinitionName
context_inputs: BTreeSet<ContextProjectionKind>
steps: Vec<DecisionProfileStep>
oracle_policy: ProfileOraclePolicy
trace_policy: TracePolicy
version: VersionAnchor
```

`DecisionProfileStep`:

```text
pass: DefinitionId
mode: ImplementationMode
```

Validation rules:

- steps must be non-empty;
- every pass exists;
- every step mode is allowed by the pass;
- pass input roles/kinds are available from the profile's context inputs or
  earlier pass outputs;
- disabled passes cannot be required by downstream steps;
- normal profiles cannot include oracle-only artifacts or oracle modes;
- context inputs must be explicit, even when using the full actor context.

### `DecisionRegistry`

Checked collection:

```text
DecisionRegistry {
  representations: BTreeMap<DefinitionId, RepresentationKindDef>
  passes: BTreeMap<DefinitionId, DecisionPassContract>
  profiles: BTreeMap<DefinitionId, DecisionProfile>
}
```

Implement a builder that preserves deterministic ordering and returns
`Result<DecisionRegistry, DecisionError>`.

The registry verifier should be separate from constructors:

- constructors validate local invariants;
- registry validation checks cross-reference and profile flow invariants.

This mirrors `world-defs` and keeps future authoring clean.

### `DecisionTrace`

Phase 8 should define the value vocabulary but not require real execution.

Recommended types:

```text
DecisionTrace
DecisionTraceHeader
DecisionTraceStep
DecisionArtifactRef
DecisionArtifactRecord
DecisionPassDiagnostic
DecisionTraceStatus
```

Trace header should include:

```text
actor: ActorId
profile: DefinitionId
profile_version: VersionAnchor
context_reads: ContextReadSet
context_provenance: ContextProvenance
oracle_policy: ProfileOraclePolicy
```

Artifact records should include:

```text
artifact: DecisionArtifactRef
kind: DefinitionId
role: RepresentationRole
producer: Option<DefinitionId>
provenance: ContextProvenance
```

Do not store opaque LLM prose as authoritative state. If natural-language
rationale appears later, it should be diagnostic-only or trace-only.

## Implementation Steps

### Step 1: Replace The Stub Crate Shape

Create the module tree and update `lib.rs` docs from semantic appraisal only
to configurable social-cognitive decision substrate.

Add `thiserror` to `world-decision` if `DecisionError` is rich enough to
benefit from derives.

### Step 2: Add Representation Definitions

Implement:

- `RepresentationRole`
- `RepresentationVisibility`
- `RepresentationPersistence`
- `RepresentationAuthority`
- `RepresentationKindDef`

Add tests for:

- empty role rejection;
- duplicate role normalization through `BTreeSet`;
- proposal authority declaration;
- oracle visibility classification.

### Step 3: Add Pass Contract Definitions

Implement:

- `PassClass`
- `ImplementationMode`
- `DeterminismPolicy`
- `InputRequirement`
- `RepresentationInput`
- `RepresentationOutput`
- `PassWritePolicy`
- `TracePolicy`
- `DecisionPassContract`

Add tests for:

- empty implementation mode rejection;
- hard write rejection;
- outputs requiring declared roles;
- LLM/oracle metadata compatibility where local invariants can check it.

### Step 4: Add Profile Definitions

Implement:

- `DecisionProfile`
- `DecisionProfileStep`
- `ProfileOraclePolicy`

Add tests for:

- empty step rejection;
- explicit context input preservation;
- oracle policy constructors;
- deterministic step ordering.

### Step 5: Add Decision Registry And Cross-Validation

Implement:

- `DecisionRegistry`
- `DecisionRegistryBuilder`
- `registry::validate`

Validation should cover:

- duplicate id rejection;
- pass input/output representation references;
- output kind role compatibility;
- profile pass references;
- profile mode compatibility;
- profile input availability from context or earlier pass outputs;
- disabled pass downstream failure;
- normal profile rejecting oracle modes and oracle-only artifacts.

### Step 6: Add Trace Vocabulary

Implement:

- `DecisionTraceHeader`
- `DecisionTrace`
- `DecisionTraceStep`
- `DecisionArtifactRef`
- `DecisionArtifactRecord`
- `DecisionPassDiagnostic`
- `DecisionTraceStatus`

Keep constructors narrow and value-like. Phase 9 will add trace building during
execution.

Tests should prove:

- artifact refs are trace-local and deterministic;
- trace header can carry `ContextReadSet` and `ContextProvenance`;
- oracle traces are explicit.

### Step 7: Add Seed Profile Fixtures

Add test-only fixtures for profiles that Phase 9 can execute later:

```text
direct_action_baseline
intent_only_baseline
structured_context_baseline
explicit_other_model_baseline
oracle_other_model_baseline
```

These should be test fixtures, not production standard profiles yet.

The fixtures should validate the shape of the first ablation matrix without
pretending typed speech or other-model semantics already exist.

### Step 8: Guardrails

Ensure existing dependency-direction guardrails still pass.

Add a crate-local guardrail if needed to prevent `world-decision` source from
importing:

```text
world_model
world_runtime
world_standard_runtime
```

Do not duplicate the workspace manifest guardrail unless a source-level import
guard catches a real risk not already covered.

## Out Of Scope

Do not implement:

- decision pass execution;
- action request lowering;
- intent/activity runtime-control commits;
- typed speech semantics;
- commitment lifecycle semantics;
- bounded other-model algorithms;
- appraisal variable derivation;
- LLM adapters;
- oracle providers;
- source syntax or diagnostics;
- scenario runner;
- benchmark metrics;
- persistence.

## Test Plan

Focused during development:

```bash
cargo test -p world-decision
cargo check -p world-decision
```

Before completion:

```bash
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
git diff --check
```

## Acceptance Criteria

- `world-decision` is no longer a stub.
- `world-decision` still depends only on `world-core`, `world-defs`, and
  `world-context`, plus `thiserror` if used.
- `DecisionRegistry` validates representation, pass, and profile references.
- Normal profiles cannot include oracle modes or oracle-only artifacts.
- Pass contracts cannot declare hard mutation authority.
- Profile validation can prove pass input availability over context inputs and
  prior outputs.
- Trace vocabulary can reference context reads/provenance and typed decision
  artifacts.
- Appraisal vocabulary is not hardcoded as the root of the decision substrate.
- No source-authoring, model, runtime, standard-runtime, LLM, or scenario
  dependency enters `world-decision`.

## Future Handoff

Phase 9 should take the checked registry from this phase and add a small static
profile runner:

```text
ActorContextProjection
  -> DecisionProfile
  -> validated concrete pass execution
  -> DecisionTrace
  -> selected intent, request, abstention, or proposal
```

Phase 10 should then add the first social-cognitive representation slice,
using this substrate instead of changing the substrate around one theory.
