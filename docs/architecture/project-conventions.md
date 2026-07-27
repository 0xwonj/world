# Project Conventions

## Status

Supporting convention draft.

These conventions remain active only where they do not conflict with
`AGENTS.md` or the normative
[`target-architecture/`](target-architecture/README.md) package.

## Purpose

This document records implementation choices that are safe to decide before
Phase 0 code begins.

It is intentionally narrow. It should capture conventions that protect the
whole architecture, not library choices that are better made during later
phase-local planning.

## Decide Now

### Rust And Workspace

Use stable Rust and a Cargo workspace.

Current workspace convention:

```text
edition:
  Rust 2024

toolchain:
  stable

workspace:
  one root Cargo workspace
  one root Cargo.lock
  package crates under crates/
```

Reason:

The crate graph is part of the architecture. The workspace should make
dependency direction visible before implementation details accumulate.

### Crate Dependency Direction

Follow [Crate Boundary Architecture](crates.md).

Foundational crates should stay small. Core crates must not gain default
dependencies on:

```text
ECS
graph database / graph algorithm stack
Datalog / incremental query engine
scripting runtime
Wasm runtime
async runtime
parser stack
renderer / UI framework
```

Reason:

Those tools may be valuable later, but they must not define the source of
truth, mutation authority, public ontology, or public API shape.

### ID Policy

Use newtyped identifiers for architecturally distinct identities.

Examples:

```text
EntityId
ActorId
DefinitionId
EventRecordId
CausalTransactionId
ProcessInstanceId
ActivityId
QueryEpoch
StoreCursor
```

Rules:

- do not pass raw integers or raw strings across crate boundaries when the
  value has domain meaning
- keep durable story identity separate from runtime handles
- do not serialize ECS entity handles as durable world identity
- add ids as they become necessary; do not invent a complete universe of ids
  before code needs them

Reason:

ID confusion is expensive to fix after APIs exist. Newtypes keep authority,
history, process, and definition references distinct.

### Error And Outcome Policy

Keep gameplay outcomes separate from infrastructure errors.

Preferred shape:

```text
Result<DomainOutcome, InfrastructureError>
```

Examples of domain outcomes:

```text
Rejected
Blocked
AttemptFailed
Interrupted
ConflictResolved
Committed
AbortedWithNoCommit
```

Examples of infrastructure errors:

```text
missing checked definition
malformed checked data
engine invariant violation
IO or serialization failure
version incompatibility
corrupted save or registry
```

Library error enums should use typed errors. `thiserror` is the preferred
candidate when an error crate is needed.

`world-core` should avoid an error helper crate while its error surface remains
small. Manual `Display` and `Error` implementations keep the foundational crate
lean and make dependency additions explicit. Higher-level crates can use
`thiserror` once the error enum is domain-rich enough to benefit from derives.

Reason:

Failed gameplay attempts are part of the simulation. Treating them as Rust
errors would make normal runtime behavior harder to inspect and compose.

### Serialization Policy

`serde` is the preferred serialization framework candidate.

Rules:

- persistence backend is not selected yet
- binary/text format is not selected yet
- derive support may be added where stable data types need it
- serialization attributes should not leak persistence policy into every
  domain type before the shape is stable

Reason:

Save/load, event history, process state, diagnostics snapshots, and authoring
artifacts will need serialization, but the backend should be selected after
core data shapes are clearer.

### Diagnostics Policy

Runtime/library errors and authoring diagnostics are separate concerns.

Preferred candidates:

```text
thiserror:
  typed library errors

miette:
  authoring/source diagnostics with spans, labels, help, and related context
```

Rules:

- do not put source-diagnostic renderer types in foundational runtime APIs
- diagnostics should preserve phase, symbol, source, and authority context
  when available
- exact renderer choice can wait until authoring implementation

Reason:

Pack authoring and verification need source-aware diagnostics. Runtime crates
need typed errors and inspectable outcomes, not parser-specific renderer
types.

### Test Placement Policy

Place tests by the boundary they protect, not by current convenience.

Use crate `tests/` for black-box tests that should see the crate like an
external user or neighboring crate.

Put these in `tests/`:

- public API contract tests
- public re-export surface tests
- cross-crate behavior tests that use only public APIs
- crate dependency-direction guardrails
- public authority-boundary guardrails
- manifest or source guardrails that protect workspace-level architecture

Use `src/tests.rs` or `src/tests/` for white-box tests that need crate-private
or module-private access to protect internal invariants.

Put these in `src/tests`:

- private store and verifier invariants
- internal apply-plan or preflight atomicity checks
- `pub(crate)` constructor and receiver discipline
- crate-local implementation guardrails tied to private module layout
- behavior tests that require private fixtures and would become weaker through
  only public API observation

Do not add inline `#[cfg(test)] mod tests` blocks inside production modules.
Even leaf-module tests should live in `src/tests.rs` or `src/tests/<topic>.rs`.
This keeps production modules focused on runtime/library code and makes test
ownership visible at the crate level.

Root `#[cfg(test)] mod tests;` declarations are allowed to load `src/tests.rs`
or `src/tests/mod.rs`. Test-only support code inside production modules should
be rare, minimal, and used only when a private invariant cannot be tested
without it.

Guardrail placement follows the same rule:

- public or cross-crate boundary guardrail: `tests/`
- private implementation discipline guardrail: `src/tests/guardrails.rs`

Source or manifest scanners must include focused tests for the scanner itself,
such as comment/string masking, renamed dependency handling, or token
normalization. A scanner guardrail without parser/scanner tests gives false
confidence.

Test fixtures should not force production APIs to become public. Use
`src/tests/helpers.rs` for private fixtures and `tests/helpers.rs` only for
fixtures that can be built through public APIs.

Test names should describe behavior, not project-management state.

Prefer:

```text
hard_commit_application_is_atomic_when_late_storage_checks_fail
definition_schema_candidates_do_not_populate_capabilities
crate_dependency_direction_matches_architecture
```

Avoid:

```text
works
test_new_logic
test_fix
```

Reason:

Tests are part of the architecture boundary. Black-box tests protect public
contracts, while white-box tests protect invariants that should not become
public API just to be tested.

### Runtime Handle Policy

`slotmap` is the preferred candidate for runtime handles when handles are
needed.

Rules:

- runtime handles are not durable story identity
- durable ids remain domain-owned newtypes
- handle storage is an implementation detail of the owning crate

Reason:

Runtime storage needs efficient dynamic handles, but the engine's durable
history, memory, social claims, and event records must not depend on an
in-memory container id.

### Parser Policy

Do not choose the final parser yet.

Current preferred candidate:

```text
chumsky:
  first candidate for pack / semantic DSL parsing once source syntax begins
```

Rules:

- parser dependencies belong in `world-authoring`
- runtime crates consume checked definitions, not parser ASTs
- final parser selection waits until syntax and diagnostics needs are known

Reason:

Choosing a parser before syntax is shaped risks leaking parser types into the
definition or runtime model.

### Async Policy

Keep the core runtime synchronous.

Rules:

- async runtime is not a dependency of core crates
- async may appear in `world-engine` or later host/adapter crates
- no transaction staging path should require holding runtime state across an
  `.await`

Reason:

Core simulation is primarily state validation, staging, commit, invalidation,
and scheduler drain. Synchronous boundaries keep borrow, replay, save/load,
and inspection behavior simpler.

### Accelerator Policy

ECS, graph, Datalog-like, scripting, and Wasm systems are allowed later as
adapters, projections, tooling, or extension boundaries.

Rules:

- they are not the root source of truth
- they do not publish `EventRecord`s directly
- they do not mutate hard state directly
- accepted outputs return through `ActionRequest`, `ProcessTick`,
  `RuntimeControlUpdate`, accepted non-hard updates, or `CausalTransaction`
- deep game-system packs may use ECS-backed local projections for high-volume
  concrete simulation after the core authority path exists

Reason:

Deep simulation may benefit from specialized storage and execution tools, but
the reusable simulation core must keep authority and causality domain-owned.

## Do Not Decide Yet

Defer:

- exact dependency versions except what Cargo resolves during Phase 0
- exact persistence backend
- exact parser crate
- exact ECS, graph, Datalog, scripting, or Wasm library
- exact async runtime
- benchmark and fuzzing stack
- CI matrix
- final public API names
- final module tree

## Phase 0 Checks

Initial workspace checks should be simple:

```text
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
```

More expensive checks should be added when there is code that justifies them.
