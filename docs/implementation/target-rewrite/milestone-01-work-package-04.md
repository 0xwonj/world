# M1/W4: Engine Composition and Standard Transfer

## Status

Complete.

W1-W3 established canonical identity, verified definitions, immutable model
values, and the private runtime authority waist. W4 connects those owners
through the target engine facade and one trusted standard semantic
implementation. It does not generalize the deterministic kernel or introduce
actor-relative cognition.

## Planning posture

This plan fixes W4's outcome, package ownership, public authority boundaries,
minimum vertical interaction, and acceptance evidence. It does not freeze the
order of private helper modules, the internal representation of dispatch
indexes, or the exact decomposition of implementation commits.

Implementation should proceed in the smallest compile-clean vertical slices.
Local method choices may change as Rust privacy, error shape, and tests expose
a simpler design. A change to package dependency direction, runtime authority,
semantic-extension trust, execution identity, or the public engine/runtime
seam requires an explicit architecture review.

## Goal

Complete the first target-shaped public interaction:

```text
trusted standard installation
  + exact verified RuntimeDefinitionSet
  + checked initial execution inputs
    -> sealed ResolvedExecution
    -> non-cloneable RunAttempt
    -> public ControllerRequest
    -> Admit one exact command
    -> evaluate one standard containment transfer
    -> complete one Fire publication
    -> inspect the resulting WorldSession
```

The successful path must use the same artifact verification, execution
identity, attempt reservation, record sealing, atomic publication, scheduler,
history, and read capabilities that future game systems will use. There is no
test-only mutation path or standard-specific session constructor.

## Completed inputs

### W1

- canonical preimage writer and selected digest protocol;
- purpose-specific IDs, `SimMoment`, revision, and deterministic value
  conventions;
- exact workspace and dependency-allowlist foundation.

### W2

- checked semantic-interface descriptors and exact catalog lookup;
- checked action, runtime-requirement, effect, event, and event-emission
  definitions;
- sealed `VerifiedPackArtifact`, exact `PackLock`, exact pack set, and linked
  `RuntimeDefinitionSet`;
- deterministic programmatic authoring and diagnostics;
- the standard transfer pack and its exact interface descriptor.

### W3

- immutable accepted containment state and checked `CommandEnvelope`;
- canonical initial root, execution specification, semantic manifest, closure,
  attempt binding, and trajectory identities;
- private session head, scheduler, authority-record sealing/application, and
  one atomic in-memory repository;
- attempt create/open, reservation, publication receipt, reconciliation,
  cancellation, and finalization;
- minimal `Admit`, command-only staged `Fire`, and `Manage`;
- accepted containment-transfer record shape, derived physical event,
  reaction envelope, and strictly later post-commit scheduling;
- opaque `RuntimeService`, non-cloneable `RuntimeAttemptDriver`,
  non-cloneable `PreparedFire`, and read-only `RuntimeSessionReader`.

W4 consumes these values. It must not rebuild their identities or create a
second owner for their invariants.

## Non-goals

- same-moment multi-command batching, footprints, or conflict resolution;
- consuming or routing post-commit work;
- actor-relative context, candidate generation, action opportunities, or
  lifecycle policy;
- processes, appraisal, intent, activity, social state, or dialogue;
- local grids, capability derivation, or other reference-game systems beyond
  containment transfer;
- checkpoints, persistent restoration, archives, delivery, or artifact-pin
  protocols;
- a second runtime repository or public storage SPI;
- a textual pack DSL, dynamic native plugin ABI, Wasm, MCP, CLI, or lab;
- compatibility wrappers or a selectable direct-runtime product path;
- a universal semantic value tree, callback registry, effect bag, or
  subsystem trait.

## Package graph

W4 adds the first real consumers that justify these packages:

```text
world-standard-runtime
  -> world-core, world-defs, world-model, world-runtime, world-standard

world-engine
  -> world-core, world-defs, world-model, world-runtime

world-conformance (test-only)
  -> world-authoring, world-engine, world-standard, world-standard-runtime
```

`world-engine` does not depend on `world-authoring` or
`world-standard-runtime`. The application/conformance composition root selects
trusted standard semantics and supplies already verified execution artifacts.
`world-standard-runtime` does not depend on `world-engine`.

`world-context` and `world-decision` remain absent until M3. `world-lab` and
`world-cli` remain absent until M6.

No new registry dependency is expected.

## Ownership boundaries

### `world-runtime`

Runtime owns:

- the sealed process-local `ActivatedRuntimeExecution`;
- exact activation checks against the installed implementation binding;
- semantic dispatch input shapes that expose only immutable staged evidence;
- the checked accepted-transfer proposal constructor;
- all session, scheduler, reservation, sealing, application, and publication
  authority.

Runtime may expose a public opaque factory or activation operation required by
`world-engine`. It must not expose `SessionHead`, repository operations,
record sealing, reservation grants, or publication arguments.

### `world-engine`

Engine owns:

- immutable `EngineDistribution`;
- the read-only `ArtifactResolver` host port;
- execution resolution and sealed `ResolvedExecution`;
- `Engine`, non-cloneable `RunAttempt`, cloneable read-only `WorldSession`,
  and the minimum `Inspector`;
- translation of a checked public controller request into runtime ingress;
- staged orchestration of prepare, trusted evaluation, and completion.

Engine coordinates authority but cannot publish without the runtime-owned
driver and sealed prepared token.

### `world-standard-runtime`

The standard runtime package owns:

- the behavior-affecting identity of the standard transfer implementation;
- the trusted implementation corresponding exactly to
  `world.standard.transfer`;
- requirement evaluation over the exact immutable transfer roles projected by
  runtime;
- a composition value that can be installed without giving the implementation
  a world pointer, repository handle, or transaction builder.

It does not own standard definitions, accepted state, transfer-delta
construction, proposal construction, or publication.

### `world-conformance`

The conformance leaf owns the first cross-package black-box composition:

- standard catalog and implementation installation;
- programmatic pack compilation through normal W2 authoring;
- execution resolution and attempt construction;
- public request, advancement, and read inspection;
- dependency and privacy assertions that require several package surfaces.

W5 expands this leaf into the complete M1 absence and conformance proof. W4
creates it only because the first real cross-package consumer now exists.

## Semantic implementation boundary

The implementation port must be narrow and typed.

For W4, only the standard containment-transfer family has a producer and
consumer. Do not introduce:

- `dyn Subsystem`;
- a generic string-keyed operation callback;
- a universal dynamic argument/value representation;
- a public mutable transaction or effect collector;
- a semantic plugin interface that claims to cover future physical, social,
  agency, or process families.

The exact Rust representation is selected during the first implementation
slice. It may use a runtime-owned family-specific trait, sealed callable, or
another narrow typed value if that is the smallest form that permits
`world-standard-runtime` and `world-engine` to depend in the target direction.
Whichever form is selected must:

- bind one exact semantic-interface reference and implementation identity;
- receive only the prepared immutable transfer input it needs;
- return only the bounded permission result or typed failure, while runtime
  derives the exact write from the activated roles;
- be safe to store behind the immutable distribution/activation boundary;
- enter execution semantics when it can change a logical result;
- add no mutation capability outside runtime publication.

Future semantic families add their own typed ports only with a real vertical
consumer. They need not fit an abstraction invented for transfer.

## Installation and activation

`EngineDistribution` is constructed from trusted, statically linked bundles.
Its checked construction must establish:

- interface keys are unique;
- exact descriptor references match their implementations;
- implementation identities are unique where required by the binding model;
- the exported `SemanticInterfaceCatalog` and installed implementations cannot
  disagree;
- installed-but-unused interfaces do not alter a definition set or normalized
  execution-semantics identity.

Resolution then performs one direction of trust:

```text
unchecked or host-supplied resolution input
  -> resolve exact artifact references
  -> owner-validate loaded artifacts
  -> verify RuntimeDefinitionSet closure
  -> match every required interface to one exact installed implementation
  -> runtime verifies engine-protocol compatibility and typed action contracts
  -> verify initial root, spec, config, termination, and lineage correspondence
  -> runtime mints ActivatedRuntimeExecution
  -> engine seals ResolvedExecution
```

W4 may use an in-memory `ArtifactResolver` in conformance tests. The port is
read-only; artifact retention and durable availability begin in later
milestones.

`ResolvedExecution` has private fields and no public `Default`,
serialization, deserialization, or constructor. Callers may inspect only
stable identity and provenance needed to start and understand an attempt.

## Controller boundary

M1 has no actor-relative context or grounded candidate set. Its
`ControllerRequest` therefore represents one explicitly trusted host
controller's exact request to invoke a checked pack action with complete
bindings and ingress identity.

This is not an actor-visible claim that the action was perceived or available.
The engine:

1. resolves the action and binding schema against the linked definition set
   while constructing the exact `CommandEnvelope`;
2. submits only the resulting `AdmitRequest` to runtime, which derives the
   ingress identity and fingerprint;
3. at Fire, the activated runtime resolves the typed action and validates its
   action-specific actor-role correspondence;
4. reports actor-role inequality as the stable modeled `BindingMismatch`
   outcome rather than inventing a controller-wide binding name.

The public request accepts no state delta, effect program, runtime command,
authority cursor, record, repository handle, or publication capability.

Before M3 exposes actor-facing control, its context/candidate layer will become
the ordinary constructor of controller selections and may narrow this M1
host-control surface. W4 must keep the request representation small enough
that this change does not create a second authority or compatibility path.

The exact public field and constructor shape is an implementation-time choice
subject to these constraints and the M1 black-box scenario.

## Standard transfer path

For a genuinely new prepared transfer command:

1. the runtime-owned activated registry resolves the exact action and trusted
   semantic implementation;
2. the standard implementation reads only the supplied immutable snapshot and
   exact bound roles;
3. `can-transfer-item` checks the selected W3 containment invariants;
4. failure produces the closed stable rejection already owned by
   `world-model`;
5. success grants permission for those roles but supplies no write;
6. runtime derives the checked `ContainmentTransferDelta` from every activated
   role and binds it to the exact `PreparedFire`;
7. W3 sealing derives the `ItemTransferred` event and reaction envelope;
8. W3 publication atomically updates accepted state, command ledger,
   scheduler, history, cursor, and receipt.

The standard implementation cannot supply an independent event, reaction
envelope, ledger update, scheduler delta, record, cursor, or receipt.

Retained and command-ID-reuse inputs are never reevaluated. Engine uses the
existing `PreparedFire::retain_resolution` path.

## Engine and attempt facade

The minimum W4 facade follows the target ownership:

```text
EngineBuilder
  -> Engine

Engine::resolve_execution
  -> ResolvedExecution

Engine::start_attempt
  -> RunAttempt

RunAttempt
  submit_controller_request
  advance one bounded step
  status / binding / id
  session

WorldSession
  cursor
  snapshot
  inspector

Inspector
  one containment-oriented read query
```

`RunAttempt` is non-cloneable because it owns the one mutation driver.
`WorldSession` is cloneable because it owns only the runtime read capability.
The inspector copies or projects immutable read data and cannot recover a
driver.

W4 advancement is deliberately narrow. It may admit and fire the one-command
W3 protocol, but it does not pretend to implement M2 whole-moment draining,
post-commit routing, lifecycle coordination, or complete `drain_until`.

If post-commit work becomes globally least after an accepted transfer, the
facade preserves the typed `PostCommitRoutingRequired` boundary. It does not
consume, skip, reschedule, or synthesize a lifecycle result.

## Failure model

Expected modeled outcomes remain typed values:

- exact request retry;
- stable runtime requirement rejection;
- retained command result;
- input-ID reuse mismatch;
- no work due within a bound;
- post-commit routing required;
- legal session-management outcomes.

Errors cover:

- invalid or conflicting distribution construction;
- unsupported engine protocol or incompatible semantic closure/binding;
- artifact resolution or owner validation failure;
- invalid initial execution binding;
- unsupported action or malformed controller binding;
- runtime integrity, authority, or storage failure;
- trusted standard implementation failure outside a modeled rejection.

No error path may publish a partial state change. A verified pre-publication
failure after Fire reservation retains `EngineFailure` and finalizes at the
unchanged cursor. Reopening the same attempt key reconciles an unpublished
reservation and fences the old grant. Failures whose publication outcome is
ambiguous remain fail-closed rather than being guessed successful or retried.

## Initial implementation sequence

This sequence is a starting order, not a permanent internal architecture:

1. Add the three justified package manifests and exact dependency checks.
2. Add the minimal runtime activation and typed accepted-transfer seam.
3. Implement the standard transfer semantic bundle and its focused tests.
4. Implement distribution construction and exact resolution in
   `world-engine`.
5. Implement the attempt, controller, advance, session, and inspector facades.
6. Add the cross-package transfer scenario in `world-conformance`.
7. Review the public surface and dependency graph before recording completion
   evidence.

If a thinner compile-clean order appears during implementation, use it without
changing the fixed boundaries above.

## Acceptance gates

### Package and privacy

- the local dependency graph matches the target direction exactly;
- `world-engine` has no normal or test dependency on `world-authoring`;
- `world-engine` has no dependency on `world-standard-runtime`;
- `world-standard-runtime` has no dependency on `world-engine`;
- `world-conformance` is test-only and no production package depends on it;
- no public value can construct a session head, activation, sealed record,
  reservation, receipt, or publication argument;
- `RunAttempt` and every mutation/prepared capability remain non-cloneable;
- `WorldSession` and inspector expose no mutation path.

### Installation and resolution

- duplicate or conflicting installed interfaces fail before engine creation;
- an incompatible required/installed implementation closure fails before
  attempt construction;
- artifact decode and owner validation precede activation;
- activation order and unused installed implementations cannot change the
  normalized execution identity;
- `ResolvedExecution` cannot be forged, deserialized, or assembled from loose
  component digests.

### Standard semantics

- the exact transfer descriptor resolves to the exact standard implementation;
- requirement failure produces one stable rejection and no containment change;
- accepted transfer changes only the checked containment relation;
- accepted commit, derived event, reaction envelope, command-ledger insertion,
  and post-commit dispatch cannot disagree;
- the standard implementation receives no authority or unrestricted session
  read;
- an exact retry is retained and never reevaluated;
- request-ID reuse cannot create another effect.

### Facade and vertical scenario

- one public controller request compiles/resolves through the exact standard
  pack and publishes one accepted transfer;
- the resulting `WorldSession` snapshot and inspector report the same revision
  and containment result;
- the attempt binding matches the sealed resolved execution;
- post-commit work remains present and the next unsupported advance reports
  `PostCommitRoutingRequired`;
- repeated construction and execution produce identical definition,
  execution, record, and trajectory fingerprints;
- no direct runtime product path is needed by the conformance consumer.

### Workspace verification

```text
cargo fmt --all --check
cargo check --locked --workspace
cargo clippy --locked --workspace --all-targets
cargo test --locked --workspace
RUSTDOCFLAGS="-Dwarnings" cargo doc --locked --workspace --no-deps
cargo metadata --locked --all-features --format-version 1
cargo tree --locked --workspace --all-features --target all
rg --files -g Cargo.toml
git diff --check
```

The package allowlist must compare complete local and registry dependency
metadata, not only package names.

```bash
cargo metadata --locked --all-features --format-version 1 |
  jq -e '
    def dep:
      {name,
       source: (.source // "local"),
       req,
       kind: (.kind // "normal"),
       rename,
       optional,
       uses_default_features,
       features,
       target,
       registry};
    def local_edge($name; $kind):
      {name: $name, source: "local", req: "*", kind: $kind,
       rename: null, optional: false, uses_default_features: true,
       features: [], target: null, registry: null};
    def registry_edge($name; $req; $defaults; $features):
      {name: $name,
       source: "registry+https://github.com/rust-lang/crates.io-index",
       req: $req, kind: "normal", rename: null, optional: false,
       uses_default_features: $defaults, features: $features,
       target: null, registry: null};

    ([.packages[] | select(.source == null) | .name] | sort == [
      "world-authoring", "world-conformance", "world-core", "world-defs",
      "world-engine", "world-model", "world-runtime", "world-standard",
      "world-standard-runtime"
    ]) and
    (.workspace_members | length == 9) and
    (.workspace_default_members == .workspace_members) and
    ([.packages[] | select(.source == null) |
      {name, dependencies: ([.dependencies[] | dep] | sort_by(.name))}] |
      sort_by(.name) == [
        {name: "world-authoring", dependencies: [
          local_edge("world-core"; "normal"),
          local_edge("world-defs"; "normal")
        ]},
        {name: "world-conformance", dependencies: [
          local_edge("world-authoring"; "dev"),
          local_edge("world-engine"; "dev"),
          local_edge("world-standard"; "dev"),
          local_edge("world-standard-runtime"; "dev")
        ]},
        {name: "world-core", dependencies: [
          registry_edge("blake3"; "^1.8.5"; false; [])
        ]},
        {name: "world-defs", dependencies: [
          registry_edge("minicbor"; "^2.3.0"; false; ["alloc"]),
          local_edge("world-core"; "normal")
        ]},
        {name: "world-engine", dependencies: [
          local_edge("world-core"; "normal"),
          local_edge("world-defs"; "normal"),
          local_edge("world-model"; "normal"),
          local_edge("world-runtime"; "normal")
        ]},
        {name: "world-model", dependencies: [
          local_edge("world-core"; "normal"),
          local_edge("world-defs"; "normal")
        ]},
        {name: "world-runtime", dependencies: [
          local_edge("world-core"; "normal"),
          local_edge("world-defs"; "normal"),
          local_edge("world-model"; "normal")
        ]},
        {name: "world-standard", dependencies: [
          local_edge("world-defs"; "normal")
        ]},
        {name: "world-standard-runtime", dependencies: [
          local_edge("world-core"; "normal"),
          local_edge("world-defs"; "normal"),
          local_edge("world-model"; "normal"),
          local_edge("world-runtime"; "normal"),
          local_edge("world-standard"; "normal")
        ]}
      ]) and
    ([.resolve.nodes[].id as $id |
      .packages[] |
      select(.id == $id) |
      {name, version, source}] | sort_by(.name) == [
        {name: "arrayref", version: "0.3.9",
         source: "registry+https://github.com/rust-lang/crates.io-index"},
        {name: "arrayvec", version: "0.7.8",
         source: "registry+https://github.com/rust-lang/crates.io-index"},
        {name: "blake3", version: "1.8.5",
         source: "registry+https://github.com/rust-lang/crates.io-index"},
        {name: "cc", version: "1.4.0",
         source: "registry+https://github.com/rust-lang/crates.io-index"},
        {name: "cfg-if", version: "1.0.4",
         source: "registry+https://github.com/rust-lang/crates.io-index"},
        {name: "constant_time_eq", version: "0.4.2",
         source: "registry+https://github.com/rust-lang/crates.io-index"},
        {name: "cpufeatures", version: "0.3.0",
         source: "registry+https://github.com/rust-lang/crates.io-index"},
        {name: "find-msvc-tools", version: "0.1.9",
         source: "registry+https://github.com/rust-lang/crates.io-index"},
        {name: "libc", version: "0.2.189",
         source: "registry+https://github.com/rust-lang/crates.io-index"},
        {name: "minicbor", version: "2.3.0",
         source: "registry+https://github.com/rust-lang/crates.io-index"},
        {name: "shlex", version: "2.0.1",
         source: "registry+https://github.com/rust-lang/crates.io-index"},
        {name: "world-authoring", version: "0.0.0", source: null},
        {name: "world-conformance", version: "0.0.0", source: null},
        {name: "world-core", version: "0.0.0", source: null},
        {name: "world-defs", version: "0.0.0", source: null},
        {name: "world-engine", version: "0.0.0", source: null},
        {name: "world-model", version: "0.0.0", source: null},
        {name: "world-runtime", version: "0.0.0", source: null},
        {name: "world-standard", version: "0.0.0", source: null},
        {name: "world-standard-runtime", version: "0.0.0", source: null}
      ])
  '
```

## Decision triggers

Stop for explicit architecture review before:

- adding an engine-to-authoring or engine-to-standard-runtime dependency;
- moving activation, session authority, sealing, or publication out of
  `world-runtime`;
- making a semantic implementation port a universal subsystem or dynamic
  mutation API;
- exposing a raw command, delta, effect, callback, repository, or session-head
  constructor through `world-engine`;
- allowing a standard implementation to create events, scheduler deltas,
  records, or receipts independently;
- consuming post-commit work before M2;
- adding context, decision, process, social, grid, CLI, MCP, or persistence
  scope to W4;
- changing a W1-W3 canonical identity or package boundary;
- adding a registry dependency.

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

new direct registry dependencies in W4:
  none

resolved registry closure:
  arrayref 0.3.9, arrayvec 0.7.8, blake3 1.8.5, cc 1.4.0,
  cfg-if 1.0.4, constant_time_eq 0.4.2, cpufeatures 0.3.0,
  find-msvc-tools 0.1.9, libc 0.2.189, minicbor 2.3.0, shlex 2.0.1

workspace Cargo manifests:
  Cargo.toml
  crates/world-authoring/Cargo.toml
  crates/world-conformance/Cargo.toml
  crates/world-core/Cargo.toml
  crates/world-defs/Cargo.toml
  crates/world-engine/Cargo.toml
  crates/world-model/Cargo.toml
  crates/world-runtime/Cargo.toml
  crates/world-standard/Cargo.toml
  crates/world-standard-runtime/Cargo.toml
```

Verified results:

- runtime owns the supported engine protocol, activation, action table,
  action-specific actor-role check, exact transfer-delta derivation, proposal
  construction, sealing, and publication;
- the trusted standard evaluator receives only an immutable snapshot and four
  exact roles and returns only `Accepted` or `RequirementUnsatisfied`; its
  behavior-affecting implementation identity has a frozen canonical vector;
- `EngineDistribution` rejects catalog conflicts and implementation-ID reuse
  across the whole installation, and exact lookup is invariant under
  installation order and unrelated installed interfaces;
- the unused-installation identity guarantee is compositional: W2 proves
  unused catalog entries do not change artifacts or definitions, distribution
  lookup selects the same exact implementation, runtime admits only the exact
  required binding into normalized execution semantics, and the public repeat
  scenario compares the resulting semantics and closure identities;
- artifact resolution, owner validation, exact-set reconstruction, lock
  comparison, protocol compatibility, semantic closure, and typed activation
  all precede construction of `ResolvedExecution`;
- `ResolvedExecution`, `RunAttempt`, and prepared/publication capabilities
  cannot be caller-constructed; `RunAttempt` is non-cloneable and
  `WorldSession` is read-only;
- the public conformance path compiles the standard pack, resolves an origin,
  admits and fires one transfer, and observes the same revision and
  containment through snapshot and inspector reads;
- conformance also covers missing and altered artifacts, unsupported engine
  protocol, cross-engine capability rejection, actor-role mismatch, standard
  requirement rejection, exact ingress retry, input-ID reuse, retained
  command results, command-ID content mismatch, deterministic independent
  execution, post-commit blocking, and pre-publication failure finalization;
- verified pre-publication completion failure publishes no partial state,
  retains `EngineFailure`, finalizes at the unchanged cursor, and same-key
  reopen reconciliation fences abandoned reservation grants;
- 191 workspace unit/integration tests and 24 compile-fail doctests passed,
  including 9 public `world-conformance` scenarios, 3 engine tests, 2 standard
  runtime tests, and 116 runtime tests;
- formatting, locked workspace check, warning-free Clippy, locked workspace
  tests, warning-free API documentation, full metadata, all-target dependency
  tree, manifest scan, superseded-symbol scan, and `git diff --check` passed;
- the executable metadata allowlist returned `true`, selected exactly nine
  local packages and the registry versions listed above, and compared every
  direct dependency's source, requirement, kind, rename, optionality, default
  features, explicit features, target, and registry metadata.

## W5 handoff

W5 receives the complete executable M1 vertical slice and the initial
`world-conformance` consumer. It expands black-box failure, privacy,
dependency, reproducibility, and superseded-symbol coverage; removes any
test-only scaffolding that is not part of the target surface; and closes M1.
It adds no new gameplay system or authority path.
