# Phase 4 Local Plan: Causal Mutation Waist

## Status

Implementation-aligned local phase plan.

## Purpose

Implement the first hard-mutation runtime path without pulling process
scheduling, semantic appraisal, actor context projection, persistence, or a full
gameplay effect vocabulary forward.

The target is a small, auditable causal runtime:

```text
ActionRequest / runtime request
  -> definition lookup and binding
  -> checked typed effect interpretation
  -> CausalTransaction staging
  -> invariant checks
  -> one accepted hard commit package
  -> WorldModel receiver applies committed history, state changes, and
     invalidation atomically
```

The phase is successful when there is one visible hard-mutation waist and no
runtime path needs raw mutable store access.

## Research Inputs

Internal documents reviewed:

- `docs/architecture/implementation-plan.md`
- `docs/architecture/implementation-execution-contract.md`
- `docs/architecture/crates.md`
- `docs/architecture/runtime-pipeline.md`
- `docs/design/causal-runtime.md`
- `docs/design/time-model.md`
- `docs/design/world-model.md`
- `docs/design/truth-authority-and-layer-boundaries.md`
- `docs/design/typed-effect-primitives.md`
- `docs/design/multi-resolution-simulation.md`
- `docs/research/runtime-pipeline-implementation-research.md`
- `docs/research/causal-runtime-action-effect-event.md`
- `docs/research/time-model-and-turn-scheduling.md`
- `.codex/research/phase-4-5-runtime-research.md`

External references checked:

- [PostgreSQL transaction isolation](https://www.postgresql.org/docs/current/transaction-iso.html):
  staged writes are not public facts before commit, and commit can fail.
- [Fowler Unit of Work](https://martinfowler.com/eaaCatalog/unitOfWork.html):
  collect affected state during one business transaction and coordinate final
  write/concurrency handling.
- [Azure Event Sourcing](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
  and [Fowler Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html):
  event history is useful for audit/rebuild/replay, but full event sourcing is
  costly and not required for every store.
- [Azure CQRS](https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs):
  command/write paths and query/read paths should stay distinct.
- [MLIR Pass Management](https://mlir.llvm.org/docs/PassManagement/):
  pass scope, failure, preservation, and invalidation rules are useful
  discipline, not a reason to add a generic pass framework.
- [rustc incremental compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html)
  and [Salsa](https://salsa-rs.github.io/salsa/overview.html):
  stable keys and projection invalidation are useful for derived-view
  boundaries.
- [Rust visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html)
  and [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):
  private fields, newtypes, narrow constructors, and sealed extension points
  are the available API enforcement tools.

## Architecture Decision

Phase 4 implements the hard causal runtime in `world-runtime`, with a narrow
model-side accepted commit receiver in `world-model`.

Use the current dependency direction:

```text
world-core
  <- world-defs
  <- world-model
  <- world-runtime
```

Do not merge crates or add a new authority crate in this phase.

The practical boundary is:

```text
world-runtime:
  constructs, validates, and finalizes causal commits

world-model:
  stores current state, committed history, and invalidation metadata
  applies one accepted hard commit package through a narrow receiver
  does not bind actions, interpret effects, validate causal semantics, or
  publish partial side effects
```

Rust-specific caveat:

The current crate split cannot express true friend-crate visibility. If an
accepted package type is constructible by `world-runtime`, it cannot be made
constructible by only `world-runtime` while also living in `world-model` under
normal Rust visibility rules. Phase 4 should therefore enforce the boundary
that is possible without changing crate architecture:

- no public raw store mutators;
- no public partial event append separate from commit application;
- no public direct `EventHistoryStore` or `WorldStore` mutation path;
- one package receiver that applies transaction, events, state changes, and
  invalidation as one operation;
- causal validation and package construction are implemented and used in
  `world-runtime`;
- model-side validation is limited to storage invariants such as duplicates,
  missing transaction references, authority-family mismatch, and cursor
  exhaustion.

If stronger compile-time forge resistance against arbitrary downstream crates is
required, that is an architecture-boundary decision and should be handled
explicitly before implementation. It likely requires colocating runtime
authority and model apply internals in the same crate/module boundary or
changing which crates are treated as public API.

## Scope

Implement in `crates/world-model`:

- An accepted hard commit package/value shape for model application.
- A single `WorldModel` receiver for accepted hard commits.
- Atomic application behavior for:
  - transaction metadata append;
  - event metadata append;
  - minimal hard state changes supported by this phase;
  - derived-view invalidation.
- Public read surfaces remain read-only.
- Store-level write helpers remain non-public except through the accepted
  commit receiver.
- Structural storage errors needed by accepted commit application.

Implement in `crates/world-runtime`:

- Module structure for the causal runtime waist.
- `ActionRequest` or equivalent runtime request value for Phase 4 tests and
  first hard-mutation execution.
- `CausalRuntime` facade for executing one request/effect program against a
  mutable `WorldModel`.
- `CausalTransactionBuilder` or equivalent staging owner.
- Typed staging surfaces for reads, hard mutations, event emission, and
  invalidation.
- `TypedEffectInterpreter` that executes checked `EffectProgramDef` operations
  through staging APIs, not raw store mutation.
- Runtime-owned mapping from checked `EffectKind` values to typed handlers.
- Runtime outcome versus runtime infrastructure error split.
- Event contract enforcement before commit.
- Minimal transaction/event record production.
- Minimal invalidation production for accepted hard commits.
- Unit tests that prove failed runtime work does not publish partial history,
  state, or invalidation.

The Phase 4 effect handler mapping is a causal-waist seed, not the permanent
home for reusable physical, damage, condition, signal, or process primitives.
The high-level implementation plan now moves the standard primitive definition
and trusted semantics boundary to the standard world library phase after
runtime control and process are in place.

Allow small `world-core` additions only when Phase 4 needs shared values that
clearly belong below both model and runtime, such as a request/source reference
or stable outcome tag. Prefer keeping request and transaction staging types in
`world-runtime` until cross-crate use is real.

## Explicit Non-Scope

Do not implement:

- scheduler drain;
- `ScheduledWakeup`;
- `ProcessInstance`;
- `ProcessTick`;
- `ProcessTransition`;
- `ActivityTransition`;
- `RuntimeControlUpdate`;
- `AcceptedRuntimeControlUpdate`;
- reservation conflict algorithm;
- interruption, resume, completion, or failure lifecycle;
- actor-context projection;
- semantic appraisal;
- social, chronology, epistemic, or appraisal accepted gates;
- full physical simulation grammar;
- complete effect vocabulary;
- deterministic command replay beyond preserving the declared replay level and
  enough audit provenance for this phase;
- persistence backend or serialization format;
- scripting runtime, ECS backend, graph database, Datalog engine, async runtime,
  or cache engine.

## Target Module Shape

The module shape should make the waist visible in the source tree. Keep it
small enough that Phase 4 does not look like the full engine.

Suggested `world-model` shape after this phase:

```text
crates/world-model/src/
  lib.rs
  error.rs
  model.rs
  store.rs
  relations.rs
  history.rs
  invalidation.rs
  commit.rs          accepted hard commit package and receiver support
  runtime_control.rs storage shell only, not Phase 5 semantics
  records.rs
  query.rs
  tests.rs
```

`commit.rs` should only contain storage-facing accepted hard commit types and
application helpers. It should not contain action binding, effect
interpretation, causal validation, scheduler behavior, process behavior, or
runtime-control lane semantics.

Suggested `world-runtime` shape after this phase:

```text
crates/world-runtime/src/
  lib.rs
  error.rs
  request.rs         request/source/bound request values
  outcome.rs         committed/rejected/blocked outcome values
  runtime.rs         CausalRuntime facade
  transaction.rs     staging owner and transaction finalization data
  effects.rs         typed effect interpreter and internal dispatcher
  commit.rs          conversion from staged transaction to accepted package
  tests.rs
```

Do not add these modules in Phase 4:

```text
scheduler.rs
process.rs
runtime_control.rs
reaction.rs
resolution.rs
```

Those names belong to Phase 5 or later unless Phase 4 needs a tiny source enum
variant. If a variant is needed, keep it as source provenance, not as lifecycle
logic.

## Design Patterns To Use

Use these patterns deliberately:

- **Unit of Work / transaction builder:** collect reads, hard changes, emitted
  events, replay metadata, and invalidation before anything is applied to the
  model.
- **Accepted package receiver:** `WorldModel` receives one accepted hard commit
  package and applies it as one storage operation. It is not a causal validator.
- **Capability-based staging:** effect handlers receive a narrow staging API,
  not `&mut WorldModel` or mutable store references.
- **Closed dispatcher first:** map checked `EffectKind` values to
  runtime-owned handlers internally. Do not introduce a public plugin trait or
  scripting surface in this phase.
- **Validate-then-apply:** the model receiver should perform structural
  preflight checks before mutating stores. If a check can fail after mutation,
  use a temporary affected-store copy or another local staging strategy rather
  than rollback-by-convention.
- **Outcome/error split:** domain results are runtime outcomes; infrastructure,
  storage, exhausted id issuers, and impossible internal states are errors.
- **Read/write split:** query surfaces remain immutable; write authority is
  represented by request execution and accepted commit application.
- **Fixture-only builders:** if tests need constructors that production callers
  should not use, keep them under `#[cfg(test)]` or local test helpers.

Avoid these patterns:

- broad `System::run(&mut WorldModel)` APIs;
- public `EffectHandler` extension traits before there is a real extension
  boundary;
- generic `SetField` mutation;
- event listeners that mutate the source transaction;
- direct event-history append exposed independently from commit application;
- callback-shaped scheduler/process abstractions.

## Core Type Sketches

The sketches below are design anchors, not final public API. Names should
remain domain-shaped during implementation.

Model-side accepted commit shape:

```rust
pub struct AcceptedHardCommit {
    transaction: TransactionRecord,
    events: Vec<EventRecord>,
    changes: Vec<HardStateChange>,
    invalidation: InvalidationPackage,
}

pub enum HardStateChange {
    InsertEntity(EntitySnapshot),
    InsertRelation(RelationRecord),
}

pub struct HardCommitApplication {
    transaction_cursor: StoreCursor,
    event_cursors: Vec<StoreCursor>,
    invalidation: DerivedViewInvalidationReport,
}
```

Important meaning:

- `AcceptedHardCommit` is accepted by the runtime, not by the model.
- The model constructor, if public, can only validate package structure.
- `WorldModel::apply_hard_commit` should be the only public model-side write
  path added in this phase.
- `HardStateChange` starts narrow and should not become a generic field patch
  language.

Runtime request shape:

```rust
pub struct RuntimeRequest {
    source: RequestSource,
    actor: Option<EntityId>,
    effect_program: DefinitionId,
    submitted_at: SimulationTime,
    roles: Vec<SubmittedRole>,
    provenance: Option<ProvenanceKey>,
}

pub enum RequestSource {
    Player,
    ActorPolicy,
    Engine,
    Tooling,
}

pub struct SubmittedRole {
    name: RoleName,
    entity: EntityId,
}
```

Important meaning:

- This is enough to test the causal waist without actor-context projection.
- Process/reaction-like causes should remain provenance/source labels in Phase
  4, not process or reaction runtimes.
- If durable request identity is needed later, add it intentionally rather than
  treating every transient request as a committed fact.

Runtime facade and outcome shape:

```rust
pub struct CausalRuntime {
    transaction_ids: CausalTransactionIdIssuer,
    event_ids: EventRecordIdIssuer,
    effects: EffectDispatcher,
}

pub enum RuntimeOutcome {
    Committed(CommittedOutcome),
    Rejected(RejectedOutcome),
    Blocked(BlockedOutcome),
}

pub struct CommittedOutcome {
    transaction: CausalTransactionId,
    events: Vec<EventRecordId>,
    invalidation: DerivedViewInvalidationReport,
}
```

Important meaning:

- `CausalRuntime` is the public waist, not the effect interpreter.
- The committed outcome reports committed ids and model-visible invalidation.
- Rejected/blocked outcomes must not append history or mutate state.

Transaction staging shape:

```rust
pub struct CausalTransactionBuilder {
    id: CausalTransactionId,
    source: RequestSource,
    occurred_at: SimulationTime,
    replay_level: ReplayLevel,
    provenance: Option<ProvenanceKey>,
    changes: Vec<HardStateChange>,
    events: Vec<PendingEventRecord>,
    invalidation: InvalidationPackage,
}

pub struct EffectStager<'tx> {
    transaction: &'tx mut CausalTransactionBuilder,
}
```

Important meaning:

- `CausalTransactionBuilder` owns staged hard outputs.
- `EffectStager` is the capability passed to handlers.
- Neither type should expose mutable model stores.

Effect interpreter shape:

```rust
pub(crate) struct TypedEffectInterpreter {
    dispatcher: EffectDispatcher,
}

pub(crate) struct EffectDispatcher {
    handlers: BTreeMap<EffectKind, EffectHandler>,
}

type EffectHandler =
    fn(&EffectOp, &mut EffectStager<'_>) -> Result<(), RuntimeError>;
```

Important meaning:

- A private function-pointer dispatcher is enough for Phase 4.
- Public handler registration can wait until there is a concrete pack/runtime
  extension story.
- Permission checking wraps handler execution; handlers do not grant
  themselves authority.

Commit finalization shape:

```rust
pub(crate) struct CommitFinalizer;

impl CommitFinalizer {
    pub(crate) fn finalize(
        transaction: CausalTransactionBuilder,
    ) -> Result<AcceptedHardCommit, RuntimeError> {
        // validate event contracts, seal records, build accepted package
    }
}
```

Important meaning:

- Finalization checks that staged effects satisfy the checked definition
  contract before model application.
- The finalizer produces the only package that `CausalRuntime` applies to the
  model.
- The model remains responsible for structural storage checks.

## API Rules

- Runtime stages do not receive `&mut WorldModel` except at the final commit
  application boundary.
- Effect handlers receive staging capabilities, not raw store references.
- Public request and outcome types use domain names, not phase names.
- Public enums likely to grow should be `#[non_exhaustive]`.
- IDs and source references use newtyped values rather than raw integers or
  strings when crossing crate boundaries.
- `EffectKind` is a checked operation-family key, not the whole interpreter
  semantics.
- `StagePermission` gates what an operation is allowed to do during
  interpretation.
- Event records are committed hard evidence, not commands and not semantic
  meaning.
- Reactions are not implemented in this phase; no event listener may mutate the
  source transaction.
- Invalidation is published with the accepted commit package, not as a later
  independent side effect.
- Runtime errors indicate infrastructure or impossible internal failures.
  Runtime outcomes describe domain-level rejected, blocked, failed, or
  committed work.
- Avoid broad public traits. Add a trait only when a sealed extension point or
  multiple real implementations are needed.

## Minimal Commit Shape

The first accepted hard commit package should be small but complete enough to
exercise the authority path:

```text
Accepted hard commit package
  transaction record
  event record set
  hard state change set
  invalidation package
```

The first hard state change set can be intentionally narrow. It only needs to
prove that state change, event append, and invalidation are published together.
Candidates already present in `world-model`:

- insert or replace a minimal `EntitySnapshot`;
- insert a hard `RelationRecord`;
- later expand to richer physical mutation records.

The commit package must reject or fail atomically when:

- the transaction id is duplicated;
- an event references a missing transaction;
- an event id is duplicated;
- a relation change uses a non-hard relation family on the hard path;
- a state change violates model structural invariants;
- invalidation cannot be applied.

The exact package and delta type names can be chosen during implementation, but
they should not expose broad store mutation or imply that the model validates
causal semantics.

## Runtime Request Shape

Phase 4 needs a request shape that can drive tests and future action binding
without pulling actor-context projection forward.

The request should capture:

- source kind: player, actor policy, process-like source, reaction-like source,
  or tooling/test harness;
- actor or actor-like source when available;
- action/effect definition id;
- submitted simulation time;
- role bindings or submitted target references;
- optional provenance key;
- optional actor-view/version anchor if already available from lower crates.

The first implementation can keep binding shallow:

```text
request
  -> lookup ActionDef / EffectProgramDef
  -> verify submitted roles satisfy the local definition shape where available
  -> create a bound runtime request
```

Full actor affordance, capability, observed context, and semantic target
selection remain later-phase work.

## Transaction Staging Rules

`CausalTransactionBuilder` should be the only owner of staged hard outputs.

It should stage:

- source request/process/reaction reference;
- simulation time and ordering context available in Phase 4;
- checked effect program definition;
- read labels or read summary where available;
- hard state changes;
- emitted event records;
- invalidation package;
- replay level and provenance required by the effect program.

It should not:

- mutate `WorldModel` during effect interpretation;
- append events before invariant checks;
- publish invalidation before state/history commit;
- let event handlers run inside the source commit;
- own scheduler or process lifecycle semantics.

## Interpreter Rules

`TypedEffectInterpreter` should interpret checked `EffectProgramDef` operations
against staging capabilities.

Initial behavior:

- verify every operation has declared permissions;
- reject an operation when a handler requires a permission the operation did
  not declare;
- stage hard mutations only through mutation staging APIs;
- stage events only through event staging APIs;
- enforce that required event contracts are satisfied before final commit;
- preserve `ReplayLevel` on the transaction output;
- produce invalidation based on changed authority/store families.

Do not build a generic scripting language. Do not let a string `EffectKind`
directly decide arbitrary store writes. Runtime-owned handler registration or
matching must map checked keys to concrete domain behavior.

## Implementation Order

1. Add the model-side accepted hard commit receiver.
   - Define the smallest accepted package and hard state change shape.
   - Add `WorldModel` apply method for the package.
   - Internally apply transaction, events, hard state changes, and invalidation
     as one operation.
   - Keep existing read-only query surfaces unchanged.
2. Make storage application atomic.
   - Validate structural constraints before mutating when possible.
   - If a later step can fail, stage enough local changes first or order checks
     so the receiver cannot leave partial history/state.
   - Add tests for duplicate transaction/event and invalid relation authority.
3. Add `world-runtime` module structure:
   - `error`
   - `request`
   - `outcome`
   - `transaction`
   - `effects`
   - `commit`
   - `runtime`
4. Add runtime error and outcome types.
   - Infrastructure/internal errors remain separate from domain outcomes.
   - Domain outcomes include at least committed and rejected/blocked style
     results needed by tests.
5. Add minimal request and source values.
   - Keep them runtime-owned unless a lower crate need appears.
   - Include provenance and simulation time.
6. Add transaction staging.
   - Builder owns staged hard changes, emitted events, replay level,
     provenance, and invalidation.
   - Builder exposes typed staging methods, not store handles.
7. Add typed effect interpretation.
   - Resolve `EffectProgramDef` operations.
   - Check `StagePermission`.
   - Invoke runtime-owned handlers.
   - Stage declared events and hard changes.
8. Add event contract validation.
   - Required events must be emitted before commit.
   - Missing required events produce a runtime outcome or validation error
     before model application.
9. Add commit finalization.
   - Allocate or accept transaction/event ids through clear issuers.
   - Build the accepted hard commit package.
   - Apply it through the model receiver.
   - Return committed outcome with transaction/event ids and invalidation
     summary.
10. Add negative tests for partial publication.
    - Failed validation does not append transaction.
    - Failed event contract does not append event.
    - Failed hard state structural apply does not append history.
    - Failed invalidation does not leave committed state behind.
11. Re-run formatting, workspace checks, clippy, tests, and diff whitespace.

## Test Plan

Add focused tests for `world-model`:

- accepted hard commit applies transaction, events, state changes, and
  invalidation together.
- direct store mutation remains unavailable from public query surfaces.
- duplicate transaction id rejects the whole package.
- duplicate event id rejects the whole package.
- event referencing a missing transaction is impossible through the receiver or
  rejected before mutation.
- non-hard relation family is rejected on the hard commit path.
- invalidation marks matching derived views stale as part of commit.
- failed package application leaves event history and hard state unchanged.

Add focused tests for `world-runtime`:

- a minimal request/effect program can commit through `CausalRuntime`.
- effect handlers do not receive raw mutable stores.
- missing required event contract fails before model application.
- undeclared permission fails before model application.
- emitted event records reference the committed transaction.
- committed outcome reports transaction and event ids.
- runtime infrastructure errors are not confused with domain rejection/blocked
  outcomes.
- a failed transaction does not publish history, hard state, or invalidation.
- `ReplayLevel` from the effect program is preserved in the runtime output or
  transaction metadata chosen for this phase.

Run after implementation:

```bash
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
git diff --check
```

## Code Quality Checklist

- Keep the runtime concrete and domain-shaped.
- Prefer standard-library collections first, especially `BTreeMap` and
  `BTreeSet` where deterministic iteration is useful.
- Do not add dependencies in this phase unless a local review justifies them.
- Keep model storage errors separate from runtime execution outcomes.
- Keep rustdoc concise on authority-bearing public methods.
- Do not expose `&mut WorldModel`, `&mut WorldStore`, `&mut EventHistoryStore`,
  or `&mut DerivedViewRegistry` from runtime staging paths.
- Do not encode planning terms in code names, comments, tests, or diagnostics.
- Keep tests focused on authority, atomicity, and cross-crate contracts.

## Open Questions For Implementation

- Is the pragmatic accepted-package receiver boundary enough for this repo's
  current crate split, or should stronger compile-time forge resistance trigger
  a crate-boundary discussion before coding?
- Should transaction/event id issuance live inside `CausalRuntime`, be supplied
  by the engine host, or be passed as explicit issuers for deterministic tests?
- What is the smallest hard state change vocabulary that proves the commit
  waist without inventing premature physical simulation semantics?
- Should missing event contracts be modeled as domain rejected outcomes or
  runtime validation errors?
- Does Phase 4 need a durable source/request id, or is a provenance/source
  envelope enough until process and scheduler work begins?
- Should the accepted package receiver live directly on `WorldModel`, or behind
  a small receiver type borrowed from `WorldModel`?
