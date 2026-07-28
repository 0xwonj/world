# M1/W5: Conformance and Absence Proof

## Status

Complete.

W1-W4 already provide the complete M1 path from checked pack source through
one authoritative transfer and read-only inspection. W5 closes M1 by making
the intended package graph, authority boundaries, failure behavior, and
absence of superseded mechanisms executable facts.

## Planning posture

W5 is a proof and cleanup package. It does not add another architectural
layer, generalize the runtime, or pull later milestone concepts forward.
Where the target validation scenarios describe future facilities, W5 records
the exact M1-applicable claim instead of creating placeholder implementations.

Implementation remains bounded to:

- durable structural checks over the workspace and dependency graph;
- owner-local tests at compile, resolution, activation, and runtime authority
  boundaries;
- black-box conformance through the public engine facade;
- removal or narrow test-scoping of unreachable implementation scaffolding;
- milestone documentation and full-workspace verification.

A change to crate dependency direction, runtime authority, persistence,
canonical identity, or the public engine/runtime seam remains a decision
trigger rather than W5 cleanup.

## Entry review

The W5 entry audit found the architecture coherent and the public vertical
slice already complete. It also identified four closure needs.

### Structural evidence must be durable

The current nine-package graph and forbidden-edge checks pass, but command
output recorded in an implementation note can drift. `world-conformance`
therefore needs an executable repository-shape test that owns the exact
workspace membership and dependency allowlist and rejects reintroduced legacy
paths or symbols.

### Runtime proof must exercise the authority backstop

The standard evaluator rejects ordinary transfer-policy failures, while
runtime independently validates the authoritative source, authority record,
and destination capacity before sealing. W5 tests that private validator
directly and proves rejection leaves the aggregate unchanged.

The runtime-authority slice also needs to show that competing ingress for one
M1 moment slot publishes exactly one complete record, not merely that one
caller returns an error.

### Resolution and compiler failures need their owner tests

Resolution must reject an absent or mismatched installed semantic interface
before activation or attempt construction. Exact package selection must reject
two artifacts for one `PackKey` without returning a partial definition set.
These tests stay with the crates that own those boundaries.

### Validation-scenario scope must be truthful

The full target scenarios are cumulative architecture scenarios, not a demand
to create unused subsystems in M1:

- Scenario 1's complete same-moment batching, footprints, total conflict
  policy, and permutation invariance belong to M2. M1 proves single-slot
  exclusion, atomic publication, and no losing event or partial aggregate.
- Scenario 13 is exercised for every M1-supported artifact, dependency,
  interface, budget, and activation boundary. Artifact signatures and
  source-map sidecars are explicitly absent from the M1 artifact contract, so
  their scenario clauses are not claimed as implemented.
- Scenario 18 requires authoritative randomness. M1 deliberately has no
  random consumer or RNG abstraction. W5 proves deterministic repetition,
  an exact dependency graph without a randomness provider, and the absence of
  the target `RandomKey`/`RandomStream` protocol. Full keyed randomness moves
  to M2 with the first real random consumer.
- M1's stale-witness proof means exact source-state validation plus
  reservation and authority-cursor freshness. General actor-relative
  `ReadWitness` values enter with retained/deferred agency evaluation in M4;
  M3's grounded action path is synchronous and has no cross-revision reuse
  interval.

## Package ownership

### `world-conformance`

Owns checks that require the complete repository or public composition:

- exact workspace membership and direct dependency edges;
- no production dependency on the conformance leaf;
- no legacy package directory, path-selected package, superseded symbol, or
  selected random dependency or target random protocol;
- public compile, resolve, start, controller submission, advancement, and
  inspection behavior;
- deterministic equality of repeated compiled artifacts and executions.

The structural test reads repository source and manifests only. It does not
invoke a mutation API or infer architecture from runtime behavior.

### `world-authoring` and `world-defs`

Own deterministic compiler and exact-set rejection:

- conflicting coordinates for one `PackKey` produce deterministic
  diagnostics and no compilation;
- duplicate or conflicting verified artifacts cannot become an
  `ExactPackSet` or partial linked definition set.

### `world-engine`

Owns resolution and composition failures:

- a required semantic interface absent from the installed distribution, or
  present at the same key/version with another descriptor digest, fails before
  runtime activation;
- a descriptor/implementation mismatch cannot construct a valid
  distribution;
- failed resolution cannot start an attempt.

### `world-runtime`

Owns authority and atomicity proofs:

- source mismatch, missing source authority, and exhausted destination
  capacity are rejected by runtime even if trusted semantics propose the
  transfer;
- the failed operation changes no head, history, scheduler, ledger, cursor, or
  receipt;
- competing admission to the M1 single-command moment slot installs one exact
  successor aggregate;
- `HostBudgetExceeded` records the typed disposition and finalizes at the
  unchanged cursor without world publication;
- production reachability is warning-free without a crate-wide dead-code
  exemption.

## Work sequence

### 1. Freeze the W5 scope

- record the scenario allocations above;
- keep W5 free of RNG, batching, context, persistence, or product scaffolding;
- retain frozen lineage and canonical record representations even when their
  later constructors are test-only in M1.

### 2. Make architecture absence executable

- add a repository-shape integration test under `world-conformance`;
- encode exact members and dependency edges as data;
- scan source and manifests for the frozen superseded set and forbidden
  package paths;
- prove `world-engine` has no authoring or standard-runtime dependency and no
  production crate depends on `world-conformance`.

M1 also has no crate feature tables: there is one selectable build shape. The
structural test freezes that milestone-specific fact and rejects dotted
dependency/feature forms and package overrides that could bypass the flat
allowlist. This is not a permanent target prohibition on evidence-backed
features in later milestones.

### 3. Close owner-local authority and resolution gaps

- add conflicting-pack-coordinate and exact-set collision tests;
- add missing/mismatched semantic installation tests at engine resolution;
- add runtime validator rejection tests with unchanged-state assertions;
- strengthen competing-admission and budget-finalization tests.

### 4. Complete public conformance

- compare repeated compilation envelopes and artifact identities directly;
- add malformed controller binding and no-work-due cases when they increase
  public contract coverage without duplicating owner tests;
- preserve the supported standard composition as the only black-box product
  path.

### 5. Close M1

- run all package-local tests while implementing;
- run formatting, check, clippy, tests, doctests, dependency inspection, and
  whitespace verification across the workspace;
- record exact evidence and limitations in this document and the milestone
  plan;
- mark M1 complete only when every binary gate below is satisfied.

## Acceptance gates

### Structure

- the workspace contains exactly the nine intended packages;
- direct dependency edges equal the target allowlist;
- only `world-conformance` is a leaf test package and no production package
  depends on it;
- `world-engine` depends on definitions, model, core, and runtime, but not
  authoring or a concrete standard implementation;
- no forbidden legacy package directory, path dependency, feature-selected
  compatibility path, or superseded symbol exists;
- M1 exposes no crate feature table or manifest override; adding a legitimate
  later feature requires an explicit update to this current-state gate.

### Authority and failure

- runtime rejects invalid transfer authority independently of evaluator
  policy and publishes nothing;
- competing M1 ingress publishes one complete successor while the losing call
  returns one typed inert failure and installs no loser record or partial
  aggregate;
- budget failure finalizes the physical attempt at the last published cursor
  without changing the world trajectory;
- missing or descriptor-mismatched semantic installation and conflicting pack
  identity fail before activation;
- opaque mutation, preparation, activation, and publication capabilities
  remain non-constructible through public APIs.

### Public behavior and determinism

- the standard transfer still traverses the complete public engine path;
- malformed input, no-work-due, retry, reuse, and altered-artifact behavior is
  fail-closed and state-preserving where applicable;
- identical sources yield byte-identical compilation artifacts and definition
  identities;
- repeated public executions yield identical semantic history, cursor, and
  trajectory fingerprints;
- no random dependency or target `RandomKey`/`RandomStream` API exists in M1,
  and execution identity is unaffected by host collection or activation
  intern order.

### Verification

```text
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test --workspace --doc
git diff --check
```

The normal production build must be warning-free. Test-only protocol fixtures
may remain behind `cfg(test)`; a crate-wide dead-code allowance may not.

## Completion evidence

```text
rewrite branch:
  codex/target-architecture-rewrite

selected local packages:
  world-authoring
  world-conformance
  world-core
  world-defs
  world-engine
  world-model
  world-runtime
  world-standard
  world-standard-runtime

direct local dependency graph:
  world-authoring        -> world-core, world-defs
  world-conformance      -[dev]-> world-authoring, world-engine,
                                  world-standard, world-standard-runtime
  world-defs             -> world-core
  world-engine           -> world-core, world-defs, world-model, world-runtime
  world-model            -> world-core, world-defs
  world-runtime          -> world-core, world-defs, world-model
  world-standard         -> world-defs
  world-standard-runtime -> world-core, world-defs, world-model,
                            world-runtime, world-standard

new direct registry dependencies in W5:
  none
```

Verified results:

- the executable repository-shape test selects exactly the nine packages and
  direct edges above, resolves every path dependency to an active package,
  rejects production dependencies on the conformance leaf, and rejects
  additional crate manifests and directories;
- the same test rejects dependency subtables, target-specific and dotted
  dependency keys, inline dependency tables, feature tables and dotted feature
  keys, package overrides, quoted or escaped structural-key disguises, and
  structural headers with trailing comments or inline target/workspace maps;
- no legacy package path, superseded public symbol, random dependency, or
  target `RandomKey`/`RandomStream` protocol exists in the active tree;
- conflicting source coordinates and duplicate or conflicting verified
  artifacts fail deterministically before a compilation or exact pack set can
  be returned;
- absent and descriptor-digest-mismatched semantic interfaces fail in engine
  resolution before activation or attempt construction;
- runtime independently rejects source mismatch, missing authority, and full
  destination capacity, with every session-head component, history, and
  receipt unchanged;
- competing same-moment admission publishes exactly one complete aggregate;
  the losing caller receives `MomentSlotUnavailable` and installs no record;
- `HostBudgetExceeded` preserves the world cursor and snapshot while recording
  the typed failure and finalizing the physical attempt;
- the public conformance suite covers the complete transfer, malformed
  binding, no-work-due, missing and altered artifacts, unsupported protocol,
  cross-engine use, modeled rejections, retry and ID reuse, same-moment
  contention, failure finalization, and deterministic repeated execution;
- opaque runtime and engine capabilities retain direct compile-fail privacy
  proofs, and production builds require no crate-wide dead-code exemption;
- 205 workspace unit and integration tests and 28 compile-fail doctests
  passed, including 12 public behavior tests, 3 repository-structure tests,
  5 engine tests, 8 authoring tests, 31 definition tests, and 120 runtime
  tests;
- formatting, locked all-target dead-code-denied check, locked workspace
  check, warning-denied Clippy, locked workspace tests, explicit workspace
  doctests, metadata inspection, dependency-tree inspection, and
  `git diff --check` passed.

The scenario claims remain deliberately bounded: M1 closes the single-slot
atomic-authority subset of scenario 1, every scenario 13 boundary represented
by the M1 artifact contract, and the deterministic/absence baseline for
scenario 18. Complete batching and keyed randomness remain M2 work.

## M2 handoff

M2 receives one proven private authority waist, exact semantic closure,
atomic in-memory publication, typed attempt reconciliation, and a public
engine path. It generalizes these exercised contracts into complete
same-moment batching, footprints and total conflict resolution, complete
post-commit causal routing, bounded work, and keyed randomness. It does not
reopen package identity, create another mutation path, or introduce context,
agency, AI, product, or persistence work.
