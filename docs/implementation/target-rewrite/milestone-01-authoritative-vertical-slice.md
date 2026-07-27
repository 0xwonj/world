# M1: Immutable Packs Through One Authoritative Interaction

## Status

Active. W1 is complete and W2 is the active detailed work package.

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

- [Active detailed plan](milestone-01-work-package-02.md)
- rewrite `world-defs` with pack-qualified keys and the minimum definition
  families selected in M0;
- implement unchecked envelope decoding plus shared catalog-aware
  `ArtifactData` validation, sealed `VerifiedPackArtifact`, exact lock, exact
  set, and definition linking;
- implement target-shaped programmatic authoring and diagnostics;
- add the standard transfer pack and declarative interface requirement.

### W3: Runtime authority

- add private session head, cursor, root construction, record draft/seal/apply,
  and an in-memory atomic repository;
- implement attempt creation, reservation, receipt, finalization, and
  cancellation;
- implement minimal `Admit`, staged `Fire`, typed failed-fire disposition, and
  minimal `Manage`;
- keep the semantic repository and all publication-capable values private.

### W4: Engine and standard semantic implementation

- implement `EngineDistribution`, artifact resolution, sealed
  `ResolvedExecution`, and runtime activation;
- implement `Engine`, `RunAttempt`, `WorldSession`, and one inspector query;
- implement trusted standard transfer semantics in `world-standard-runtime`;
- drive the complete interaction through a public `ControllerRequest`.

### W5: Conformance and absence proof

- prove the old mutable model, runtime, context, and generic decision code
  removed during the workspace cutover have not re-entered;
- prove the target-shaped `world-engine` has no dependency on
  `world-authoring`;
- add black-box conformance and owner-local privacy/authority tests;
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
- missing ownership, stale witness, conflicting resource, invalid interface,
  and altered artifact fail closed;
- termination is evaluated from the runtime-owned verified contract;
- the final session is inspectable only through `WorldSession`.

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

Validation scenarios 13 and 18, plus the runtime-authority portion of scenario
1, must pass.

## Completion evidence

To be filled at milestone close.

## Next milestone handoff

M2 generalizes only the runtime contracts exercised by this slice. It does not
introduce cognition or product work.
