# M1: Immutable Packs Through One Authoritative Interaction

## Status

Complete. W1-W5 satisfy the M1 acceptance gates.

## Goal

Replace the current workspace authority path with a structurally final,
functionally minimal slice from exact pack input through one atomic runtime
publication and read-only inspection.

M1 is the first target-state merge. Review commits may stage compile-clean
subgraphs on the rewrite branch, but the merged result contains neither an old
selectable path nor a compatibility layer.

## Non-goals

- actor-relative context or decision policy;
- complete process and lifecycle protocols;
- durable checkpoint restoration;
- a database or public storage SPI;
- textual pack DSL design;
- CLI or experiment product work;
- optimization or intra-world parallel commit.

## Normative contracts

- [Target Rust Code Architecture](../../architecture/target-architecture/code-architecture.md)
- [Formal System Model](../../architecture/target-architecture/formal-model.md)
- [Extensibility and Research](../../architecture/target-architecture/extensibility-and-research.md)
- [Runtime, Persistence, and Scale](../../architecture/target-architecture/runtime-persistence-and-scale.md)
- [Validation Scenarios](../../architecture/target-architecture/validation-scenarios.md)

## Work packages

### W1: Workspace cutover and canonical core

- [Completed plan and evidence](milestone-01-work-package-01.md)
- remove the complete old dependency closure and every incoming manifest edge
  from the active workspace;
- retain only a compile-clean target foundation subgraph;
- rewrite `world-core` around canonical bytes, content digests,
  purpose-specific identities, virtual time, and revisions;
- add golden canonical vectors and dependency-allowlist checks.

### W2: Definitions, artifacts, and authoring

- [Completed plan and evidence](milestone-01-work-package-02.md)
- rewrite `world-defs` with pack-qualified keys and the minimum definition
  families selected in M0;
- implement unchecked envelope decoding plus shared catalog-aware
  `ArtifactData` validation, sealed `VerifiedPackArtifact`, exact lock, exact
  set, and definition linking;
- implement target-shaped programmatic authoring and diagnostics;
- add the standard transfer pack and declarative interface requirement.

### W3: Runtime authority

- [Completed plan and evidence](milestone-01-work-package-03.md)
- add private session head, cursor, root construction, record draft/seal/apply,
  and an in-memory atomic repository;
- implement attempt creation, reservation, receipt, finalization, and
  cancellation;
- implement minimal `Admit`, staged `Fire`, typed failed-fire disposition, and
  minimal `Manage`;
- keep the semantic repository and all publication-capable values private.

### W4: Engine and standard semantic implementation

- [Completed plan and evidence](milestone-01-work-package-04.md)
- implement `EngineDistribution`, artifact resolution, sealed
  `ResolvedExecution`, and runtime activation;
- implement `Engine`, `RunAttempt`, `WorldSession`, and one inspector query;
- implement trusted standard transfer semantics in `world-standard-runtime`;
- drive the complete interaction through a public `ControllerRequest`.

### W5: Conformance and absence proof

- [Completed plan and evidence](milestone-01-work-package-05.md)
- prove the old mutable model, runtime, context, and generic decision code
  removed during the workspace cutover have not re-entered;
- prove the target-shaped `world-engine` has no dependency on
  `world-authoring`;
- complete black-box conformance and owner-local privacy/authority tests;
- prove no replaced symbol or forbidden dependency edge remains.

Only the current implementation package receives lower-level task
decomposition. Its completion evidence may revise the next package's local
method, but not M1's outcome or authority boundaries.

## Deletion scope

The final M1 tree contains no:

- `DefinitionRegistry` or durable numeric `DefinitionId`;
- omnibus `VersionAnchor`;
- mutable `WorldModel` store or public `apply_*`;
- `CausalRuntime`;
- old accepted commit/update authority model;
- generic decision runner/profile/representation pipeline;
- broad legacy context pipeline;
- source-scan authority allowlist used in place of privacy;
- compatibility wrapper or old/new feature switch.

## Reference-game slice

M1 proves only the first physical verb from the
[Reference Game Vision](../../design/reference-game-vision.md): an exact
pack-defined containment transfer reaches the public engine/runtime authority
path and is visible through the read facade afterward.

Local grids, capability-derived object interaction, social state, cognition,
and multi-resolution execution remain later milestone work. M1 must not create
empty placeholders for them, but its engine, definition, runtime, and
inspection boundaries must permit those later consumers without another
authority path.

`world-context` and `world-decision` remain absent from active workspace
membership until M3. `world-lab` and `world-cli` are introduced in M6.

## Decision triggers

Stop for an explicit decision before:

- changing the canonical identity protocol fixed by M0;
- adding a major dependency not already approved;
- exposing a runtime repository or publication-capable value;
- adding a second persistence backend;
- expanding the initial pack family set without a vertical consumer;
- weakening exact package selection or introducing override order;
- moving artifact verification above `world-defs`;
- reintroducing context or decision work into M1.

## Acceptance gates

### Structural

- Cargo metadata contains only the intended target subgraph;
- dependency allowlist is exact;
- no old package is selected through an in-tree path dependency;
- public APIs expose no session-head constructor or publication token.

### Semantic

- one standard transfer executes through `Admit`, `Fire`, and one sealed
  authority publication;
- state, scheduler, history, cursor, and receipt expose old or new together;
- duplicate controller input cannot create a second transfer;
- missing ownership, stale source state, stale reservation/cursor evidence,
  same-moment slot contention, invalid interface, and altered artifact fail
  closed;
- termination is evaluated from the runtime-owned verified contract;
- the public engine facade exposes the final session only through
  `WorldSession`; its underlying cross-crate read capability remains
  `RuntimeSessionReader`.

### Determinism

- canonical golden vectors match;
- identical source produces identical artifact and definition-set digests;
- activation intern order does not alter durable identity;
- repeated runs produce identical record and trajectory fingerprints.

### Verification

```text
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
git diff --check
```

The M1-supported portion of validation scenario 13 and the single-slot
runtime-authority portion of scenario 1 must pass. M1 also proves deterministic
repetition, an exact dependency graph without a random provider, and absence
of the target random protocol; full keyed-randomness scenario 18 begins in M2
with the first real random consumer.

## Completion evidence

M1 now contains one complete target-shaped vertical slice:

```text
checked pack source
  -> deterministic compilation and verified artifact
  -> exact pack closure and linked definitions
  -> semantic installation and sealed resolution
  -> controller admission
  -> staged Fire evaluation
  -> one sealed atomic authority publication
  -> read-only world inspection
```

The completed workspace contains exactly nine packages and the dependency
direction recorded by W5. `world-runtime` is the sole owner of the session
head, reservation evidence, record sealing, and aggregate publication;
`world-engine` composes verified definitions and trusted semantics without
depending on authoring or a standard runtime implementation; callers receive
only opaque control capabilities and read-only session access.

All five work packages have recorded package-local evidence:

- [W1](milestone-01-work-package-01.md) replaces the workspace and freezes
  canonical core protocols;
- [W2](milestone-01-work-package-02.md) establishes artifacts, exact package
  closure, definitions, and structured authoring;
- [W3](milestone-01-work-package-03.md) establishes the private runtime
  authority and attempt protocol;
- [W4](milestone-01-work-package-04.md) composes the engine and executes the
  standard transfer through public APIs;
- [W5](milestone-01-work-package-05.md) makes dependency, absence, authority,
  failure, privacy, and deterministic behavior executable conformance facts.

The final verification passed 205 workspace unit and integration tests and 28
compile-fail doctests. Formatting, locked all-target dead-code-denied
compilation, locked workspace compilation, warning-denied Clippy, locked
workspace tests, explicit workspace doctests, exact metadata and dependency
inspection, and whitespace verification all passed.

No M1 closure fix changed the target authority boundary or dependency
direction. The only scope reconciliation was to state cumulative validation
scenarios truthfully: M1 proves the supported scenario 13 boundary set, the
single-slot atomic-authority subset of scenario 1, and the deterministic and
structural-absence baseline of scenario 18.

## Next milestone handoff

The [M1 exit review](milestone-01-exit-review.md) found the top-level
architecture fit for M2 and separated essential authority complexity from the
singular representation that must be replaced. The
[proposed M2 plan](milestone-02-deterministic-kernel.md) generalizes the
exercised single-slot authority into same-moment batching, footprints and
total conflict resolution, complete causal routing, bounded work, and keyed
randomness. It does not introduce cognition, actor-relative context, AI
integration, product adapters, or durable persistence.
