# Architecture Decisions

## Status

Current architecture decision notes.

This file records short architecture decisions that explain why the engine
shape is the way it is. It is not a full research survey, implementation plan,
or crate boundary document.

Detailed rationale lives in the linked research and design documents. These
notes only record the durable decision shape.

## ADR-001: Domain-Owned Simulation Core

Decision:

The root engine architecture is a domain-owned simulation core, not an
ECS-owned, planner-owned, script-owned, graph-owned, or client-owned runtime.

Reason:

The engine needs typed truth layers, actor-relative access, transaction
boundaries, event records, process state, semantic interpretation, and pack
verification to fit together without any one specialized tool becoming hidden
authority.

Consequence:

ECS, graph, Datalog-like, scripting, and plugin systems may be used as storage,
projection, tooling, or extension boundaries, but not as the default source of
truth or mutation authority.

## ADR-002: Materialized Stores Plus Transaction History

Decision:

Keep current world state materialized in typed stores, and record committed
changes through `CausalTransaction`, `TransactionRecord`, and `EventRecord`.

Reason:

Runtime simulation needs fast current-state queries for capability,
perception, process interruption, abstract execution, and validation. A pure
event-log model would make those paths unnecessarily heavy.

Consequence:

Event history supports audit, explanation, history, save/load continuity, and
selected replay paths. It is not the only representation of current world
state.

## ADR-003: Compiler-Shaped Passes, Not One Planner

Decision:

Use compiler-shaped authoring and runtime passes:

```text
pack declarations
  -> checked definitions

actor-relative context
  -> semantic analysis
  -> intent/activity
  -> action/process request
  -> typed effects
  -> transaction/event commit
```

Reason:

The engine needs staged representations, authority checks, diagnostics,
provenance, and resolution-aware lowering. A single planner would mix
motivation, selection, execution, and mutation too tightly.

Consequence:

Semantic appraisal, social interpretation, intent scoring, planning, process
execution, and hard mutation stay as separate pass families with explicit
input and output boundaries.

## ADR-004: Logical Components Before Crates

Decision:

Architecture component names describe logical roles before they describe Rust
crate boundaries.

Reason:

Early implementation should follow ownership and dependency direction, but it
should not split every logical role into a separate crate too early.

Consequence:

Later crate architecture may co-locate tightly coupled roles such as request
binding, causal runtime, and typed effect interpretation while preserving the
same public authority boundaries.

## ADR-005: Tiered Replay

Decision:

Replay is tiered. The baseline requirement is auditable committed outcomes,
not global deterministic recomputation of the whole simulation.

Reason:

The engine needs inspectable transaction/event history and save/load
continuity everywhere, while only selected subsystems, tests, or debug modes
need command replay that recomputes the same result from inputs.

Consequence:

Subsystems may declare stronger replay requirements, but the architecture
should distinguish audit replay, event rebuild, and deterministic command
replay instead of treating them as one global property.

## ADR-006: Standard World Library Outside Runtime Core

Decision:

Keep reusable RPG-world primitive definitions and trusted primitive semantics
in a standard world library layer outside the causal runtime core.

Reason:

The runtime core should own transaction authority, staging capabilities,
registry dispatch, commit, replay, and runtime-control gates. Common mechanics
such as containment transfer, damage, wounds, conditions, fields, signals,
resources, and passive physical process hooks need meaningful reusable
semantics, but placing all of them directly in the runtime crate would make
the runtime a hardcoded game system. Pushing them entirely into ordinary packs
would force packs either to compose overly generic mutations or to gain unsafe
callback authority.

Consequence:

`world-runtime` owns the primitive semantics registry and staging APIs without
depending on standard primitive vocabulary. The standard world library supplies
definition bundles and trusted semantics installers. Ordinary game-system
packs compose installed primitives through checked `Typed Effect Program`s;
future primitive extensions are trusted engine extensions, not arbitrary pack
scripts.

## ADR-007: Accepted Package Authority Under Current Crate Split

Decision:

Keep `world-model` and `world-runtime` as separate crates. `world-model` owns
materialized stores, accepted package types, model-side verifier checks, and
the receiver methods that apply accepted packages. `world-runtime` remains the
normal producer of `AcceptedHardCommit` and `AcceptedRuntimeControlUpdate`.

Because Rust has no friend-crate visibility, a value that lives in
`world-model` and is constructed by `world-runtime` cannot be made
constructible only by `world-runtime` without changing crate boundaries. The
current implementation therefore treats accepted-package constructors as
runtime-authority constructors by convention, rustdoc, verifier checks, facade
ownership, and source allowlist tests rather than as an absolute external
compile-time security boundary.

Reason:

The crate split is logically correct: storage and query receivers belong in
`world-model`, while transaction, scheduler, process, and effect authority
belong in `world-runtime`. Collapsing them only to hide constructors would make
the conceptual boundary less clear and would move the design away from the
long-term crate graph.

Consequence:

General game-system, standard-library, authoring, and decision code must not
construct accepted packages or call model receiver methods directly. They
compose checked definitions and typed effect programs that execute through the
runtime facade. If strict external forge prevention becomes necessary for
untrusted extension packages, revisit the crate architecture with a dedicated
kernel/facade/capability boundary instead of treating the current public
constructors as a complete security mechanism.
