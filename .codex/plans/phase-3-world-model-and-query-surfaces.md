# Phase 3 Local Plan: World Model And Query Surfaces

## Status

Implementation-aligned local phase plan.

## Purpose

Implement `world-model` as the materialized state owner and read-surface crate
without pulling the causal runtime, scheduler, semantic evaluator, persistence
backend, or external storage framework forward.

The target is a small, typed, authority-preserving model substrate:

```text
WorldModel
  stores authoritative and holder-relative state families
  exposes read-only query capabilities
  keeps accepted-apply plumbing out of its public API
  tracks derived-view staleness
```

## Research Inputs

Internal documents reviewed:

- `docs/architecture/implementation-plan.md`
- `docs/architecture/implementation-execution-contract.md`
- `docs/architecture/crates.md`
- `docs/architecture/runtime-pipeline.md`
- `docs/architecture/project-conventions.md`
- `docs/design/world-model.md`
- `docs/design/causal-runtime.md`
- `docs/design/truth-authority-and-layer-boundaries.md`
- `docs/research/world-representation-query-model.md`
- `docs/research/implementation-architecture-and-library-survey.md`

External primary references checked:

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/): API
  shape, common traits, validation, private fields, newtypes, future-proofing,
  and sealed traits.
- [Rust Book module/privacy rules](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html):
  crate/module visibility is the enforcement primitive; there is no
  friend-crate visibility.
- [`thiserror` docs](https://docs.rs/thiserror/latest/thiserror/): derives
  standard `Error` and `Display` without exposing `thiserror` in the public API.
- [Martin Fowler CQRS](https://martinfowler.com/bliki/CQRS.html):
  command/write and query/read models can be split, but the pattern adds
  complexity and should be applied only where the domain benefits.
- [Martin Fowler Event Sourcing](https://www.martinfowler.com/eaaDev/EventSourcing.html):
  event logs support audit, rebuild, temporal query, and replay, but current
  state can still be stored as a materialized model.
- [Martin Fowler Unit of Work](https://martinfowler.com/eaaCatalog/unitOfWork.html):
  staged changes should be collected and written as one business transaction.
- [Bevy ECS query docs](https://docs.rs/bevy/latest/bevy/ecs/system/struct.Query.html)
  and [Flecs relationship docs](https://www.flecs.dev/flecs/md_docs_2Relationships.html):
  query access and typed relationship pairs are useful references, but
  ECS/graph identity should not become the public world ontology.

## Research Method

The review used two independent read-only sub-agent passes:

- architecture/document-boundary review for Phase 3 versus Phase 4/5 ownership
- Rust/API convention review for current crate style, dependency direction,
  error handling, tests, and quality gates

The local plan below integrates those findings with direct document/code
inspection and external primary-source checks.

## Architecture Decision

Phase 3 should implement a domain-owned typed model, not an ECS, Datalog,
event-sourcing framework, graph database, or cache engine.

Use standard-library collections first. Do not add new dependencies in this
phase. `thiserror` is already available in the workspace and may be used by
`world-model` if the crate needs a domain-rich error enum; `world-core` remains
manual-error only.

The critical architectural split is:

```text
world-model:
  owns stores, indexes, read surfaces, and staleness state
  keeps local storage helpers crate-internal until accepted package boundaries
  are designed

world-runtime:
  owns CausalTransaction construction, staging, invariant checks, effect
  interpretation, EventRecord production, accepted hard commit construction,
  scheduler/process semantics, and runtime-control update validation
```

## Scope

Implement in `crates/world-model`:

- `WorldModel` as the root owner with private store fields.
- Authority-family store containers:
  - hard state placeholder store
  - hard relation store families
  - `EventHistoryStore`
  - `RuntimeControlStore`
  - `SocialInstitutionalStore`
  - `ChronologyStore`
  - `EpistemicStore`
  - `AppraisalRecordStore`
- Explicit store-family and authority labels for reads and invalidation.
- Minimal committed-history facade types for `TransactionRecord` and
  `EventRecord` references, without producing events.
- Minimal runtime-control storage records, without scheduler/process update
  validation.
- Minimal authority-family accepted-record envelopes or record references for
  non-hard stores, without domain-specific commit-gate validation.
- `DerivedViewRegistry` with staleness states and invalidation consumption.
- Read-only query wrappers:
  - `KernelQuery`
  - `ActorRelativeQuery`
  - `SemanticContextQuery`
  - `DebugQuery`
- A `ModelError` if invariants need typed failure reporting.

Allow small `world-core` additions only when Phase 3 needs durable ids or
ordering values that clearly belong below `world-model`.

## Explicit Non-Scope

Do not implement:

- `ActionRequest` binding.
- `TypedEffectInterpreter`.
- `CausalTransaction` staging.
- invariant checking for hard mutation.
- committed hard package construction.
- real `EventRecord` emission.
- scheduler drains.
- process ticks or process lifecycle semantics.
- `RuntimeControlUpdate` / `AcceptedRuntimeControlUpdate` validation.
- social, chronology, epistemic, or appraisal commit-gate validation.
- full actor-context projection.
- final semantic context assembly.
- persistence backend, serialization format, async runtime, scripting runtime,
  ECS backend, graph database, Datalog engine, or cache engine.

## API Rules

- Store fields are private.
- Public constructors exist for ids, labels, and read-only value objects.
  Authority-bearing committed, runtime-control, or accepted records keep private
  fields and non-public constructors until Phase 4/5 accepted package
  construction is designed.
- Public enums that are likely to grow should be `#[non_exhaustive]`.
- Use newtyped ids and domain enums instead of raw integers, strings, or
  catch-all maps when values cross crate boundaries.
- Query wrappers borrow `WorldModel` immutably and expose read-only methods.
- Query results should be ids, snapshots, read labels, or small value records,
  not mutable store references.
- Actor-relative queries require an actor or holder scope at construction.
- Debug queries may be omniscient, but their type and method names must make
  that explicit.
- Public writes are intentionally absent. Local write helpers may exist only as
  crate-internal fixture/storage plumbing; they must not become a public commit
  surface or pretend to be causal runtime authority.
- Committed and accepted value forge prevention is not solved by public
  constructors in this phase. The durable boundary is private fields, narrow
  APIs, and later runtime/engine ownership.

## Implementation Order

1. Create module structure:
   - `error`
   - `model`
   - `store`
   - `history`
   - `runtime_control`
   - `records`
   - `relations`
   - `invalidation`
   - `query`
2. Add domain labels:
   - `StoreFamily`
   - `AuthorityRead`
   - `RelationFamily`
   - `DerivedViewKey`
   - `DerivedViewStatus`
3. Implement `WorldModel` with private store fields and read-only accessors.
4. Implement store containers as concrete structs with crate-internal
   insertion/apply helpers that preserve local shape invariants for tests and
   later accepted package receivers.
5. Implement `EventHistoryStore` as an append/read facade over committed record
   snapshots, not as an event producer.
6. Implement `RuntimeControlStore` as storage/read surface only.
7. Implement non-hard accepted-record containers as authority-family envelopes
   with provenance references, leaving validation to later authority gates.
8. Implement `InvalidationPackage` and
   `DerivedViewRegistry::apply_invalidation`.
9. Implement query wrappers and ensure actor-relative construction requires
   scope.
10. Add tests for privacy-relevant behavior and read-only query contracts.

## Test Plan

Add unit tests in `world-model` for:

- `WorldModel::default` or `new` creates empty valid stores.
- store families stay separate and expose authority labels.
- query wrappers do not expose mutable store references.
- actor-relative queries cannot be constructed without actor scope.
- debug query access is explicitly labeled as debug/omniscient.
- crate-internal event history append/read preserves committed ordering and
  public query APIs cannot construct or append events.
- runtime-control store can hold minimal records but has no broad arbitrary
  public mutation API.
- invalidation packages can mark derived views stale without recomputing them.
- same logical id in different store families is not treated as the same
  authority record.
- duplicate derived-view keys or duplicate relation entries are rejected if the
  chosen container promises uniqueness.

Run after implementation:

```bash
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
git diff --check
```

## Code Quality Checklist

- Keep modules concrete and domain-shaped.
- Add traits only after there are multiple real implementations or a sealed
  public extension point is required.
- Prefer `BTreeMap` / `BTreeSet` while deterministic iteration is valuable and
  performance pressure is unknown.
- Avoid exposing `&mut WorldModel` or `&mut Store` from query paths.
- Keep `world-model` dependency-free beyond `world-core` and `world-defs`
  unless a later phase-local review justifies an addition.
- Document authority-relevant public methods with concise rustdoc.
- Keep gameplay outcomes separate from infrastructure/model errors.

## Open Questions For Implementation

- Should `DerivedViewKey` live in `world-core` as a durable id, or stay a
  `world-model` local key until cross-crate usage appears?
- Which accepted package receiver should Phase 4/5 expose first, and should it
  live directly in `world-model` or behind a higher engine/runtime facade?
