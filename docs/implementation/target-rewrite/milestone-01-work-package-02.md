# M1/W2: Definitions, Artifacts, and Structured Authoring

## Status

Complete.

The deterministic CBOR storage decision and the correctness-focused validation
model are now implemented under the frozen
[ArtifactBlobV1 Protocol](../../architecture/target-architecture/artifact-blob-v1.md).

## Goal

Introduce the smallest complete immutable-content path:

```text
compiler path:
  structured PackSource
    -> exact source graph
    -> ArtifactData
    -> shared catalog-aware validation
    -> deterministic ArtifactEnvelope + VerifiedPackArtifact

loader path:
  ArtifactEnvelope
    -> descriptor/length/digest checks
    -> decode ArtifactData
    -> the same catalog-aware validation
    -> VerifiedPackArtifact

shared continuation:
  ExactPackageSelection + VerifiedPackArtifact[]
    -> private matching PackLock + ExactPackSet
    -> RuntimeDefinitionSet
```

The slice also defines the pure standard transfer vocabulary required by later
runtime work. It does not execute or activate that vocabulary.

## Trust and validation assumption

The current pack author and local authoring toolchain are host-trusted.
Artifacts may still be corrupt, stale, mismatched, or incompatible. Validation
therefore protects domain correctness and reproducibility; W2 does not build a
hostile-input sandbox.

Keep:

- format, version, length, and digest checks;
- exact identifiers, references, dependency closure, and interface digests;
- family, stage, binding, event, and semantic collection invariants;
- private construction of sealed values;
- deterministic project-owned encoding and frozen vectors;
- runtime revalidation and atomic publication in later work.

Defer:

- repeated compiler serialize/decode round trips;
- load-time re-encoding equality;
- per-node, per-string, depth, or total-allocation accounting;
- mutation fuzzing as a completion gate;
- signatures, hostile streaming ingestion, and process isolation.

If a real third-party or network ingestion boundary appears, it receives its
own hardened adapter without changing the domain model.

## Non-goals

- textual syntax, a parser API, or a final pack DSL;
- a universal expression tree, compiler-pass framework, definition-family
  trait, or configurable optimizer;
- process definitions, authored duration, observation, projection, appraisal,
  intent, scheduler, or decision representations;
- semantic-version ranges, registries, aliases, overrides, patch artifacts,
  signatures, source-map sidecars, or artifact upgraders;
- runtime activation, process-local interning, semantic implementation
  dispatch, or trusted transfer execution;
- persistence or transport framing for envelopes, locks, or definition sets;
- compatibility with deleted definitions or authoring APIs.

## Normative contracts

- [Target Rust Code Architecture](../../architecture/target-architecture/code-architecture.md)
- [Formal System Model](../../architecture/target-architecture/formal-model.md)
- [Extensibility and Research](../../architecture/target-architecture/extensibility-and-research.md)
- [ArtifactBlobV1 Protocol](../../architecture/target-architecture/artifact-blob-v1.md)
- [Architecture Decisions](../../architecture/target-architecture/decisions.md)
- [Validation Scenarios](../../architecture/target-architecture/validation-scenarios.md)
- [M0 preservation and baseline](milestone-00-preservation-and-baseline.md)
- [Completed W1 plan and evidence](milestone-01-work-package-01.md)

## W1 input

W1 leaves one selected local package, `world-core`, with:

- the byte-complete `world-canonical-v1` identity protocol;
- BLAKE3-256 content digests;
- purpose-specific actor and entity identities;
- integer virtual time and microsteps;
- checked revisions.

W2 may depend only on that public surface. It does not move pack-specific
identities, diagnostics, limits, or serialization into core.

## Approved registry dependency

Add one direct registry dependency to `world-defs`:

```toml
minicbor = { version = "2.3.0", default-features = false, features = ["alloc"] }
```

Encoding and decoding use explicit owner-written calls. W2 enables no derive,
Serde, `std`, or floating-point feature and introduces no second serialization
framework.

## Package boundaries

The selected production graph becomes:

```text
world-defs      -> world-core, minicbor
world-authoring -> world-core, world-defs
world-standard  -> world-defs
```

`world-standard` may add a direct `world-core` edge only if its implementation
actually imports a core type. There is no dependency in either direction
between `world-authoring` and `world-standard`.

| Concern | Owner |
|---|---|
| Pack-qualified keys and checked leaf values | `world-defs` |
| Semantic-interface descriptors and catalogs | `world-defs` |
| `ArtifactData`, storage codec, validation, sealed artifacts | `world-defs` |
| Exact selection, private lock, exact set, linking | `world-defs` |
| Structured source, exact source graph, compilation diagnostics | `world-authoring` |
| Stable transfer keys, descriptor, declaration, physical event vocabulary | `world-standard` |
| Installed transfer implementation and runtime dispatch | deferred `world-standard-runtime` in W4 |
| Activation and process-local IDs | deferred `world-runtime` in W4 |

## Core APIs

The public operations stay concrete:

```text
ArtifactValidator::new(&SemanticInterfaceCatalog)
  .validate(ArtifactData)
  -> Result<VerifiedPackArtifact, ArtifactError>

ArtifactValidator::new(&SemanticInterfaceCatalog)
  .load(ArtifactEnvelope)
  -> Result<VerifiedPackArtifact, ArtifactError>

ExactPackSet::finalize(ExactPackageSelection, VerifiedPackArtifact[])
  -> Result<ExactPackSet, PackSetError>

DefinitionLinker::link(ExactPackSet)
  -> Result<RuntimeDefinitionSet, LinkError>

AuthoringCompiler::new(&SemanticInterfaceCatalog)
  .compile(CompileRequest)
  -> Result<Compilation, DiagnosticSet>
```

One private semantic function validates compiler-produced and decoded
`ArtifactData`. The direct path validates and encodes once. The load path
checks the envelope, decodes, and calls the same function. Each later boundary
checks only the new invariant it introduces.

`VerifiedPackArtifact`, `PackLock`, `ExactPackSet`, and
`RuntimeDefinitionSet` have no public constructors or deserializers.

## Definition vocabulary

### Durable keys

W2 implements the exact grammars and widths in the artifact protocol for:

- `PackKey`;
- `PackVersion { major, minor, patch }`;
- `PackCoordinate`;
- `LocalDefinitionName`;
- `DefinitionKey = PackKey + LocalDefinitionName`;
- `SemanticInterfaceKey` and nonzero interface version;
- operation, parameter, binding, and event-field names.

Every semantic role has a distinct public type. A pack has one local
definition namespace across families. Durable numeric definition IDs do not
exist.

### Semantic interfaces

`SemanticInterfaceDescriptor` contains:

- exact key and version;
- sorted operation descriptors;
- operation kind: `Predicate` or `Effect`;
- ordered named parameters of `Actor` or `Entity` kind;
- exact descriptor digest.

`SemanticInterfaceCatalog` is an immutable sorted collection with no duplicate
key or conflicting descriptor. It contains declarations only—no callback,
compiler hook, or runtime implementation.

Artifacts store only exact interface key/version/digest references. Validation
resolves those references against the supplied catalog. An unused catalog
superset changes neither emitted bytes nor semantic identity.

### Minimum definition families

`ArtifactData` contains only:

- action definitions with named typed bindings;
- `RuntimeRequirement` predicate calls;
- a nonempty ordered effect-call sequence;
- physical event definitions;
- nonempty success-event mappings.

There is no universal expression type. Requirement order is normalized because
it is conjunction; effect and success-event order remains semantic.

The first standard declaration uses:

- `world.standard@1.0.0`;
- interface `world.standard.transfer@1`;
- predicate `can-transfer-item`;
- effect `transfer-item`;
- action `transfer-item`;
- physical event `item-transferred`;
- actor, item, source, and destination bindings.

The later trusted implementation owns current containment, authority, capacity,
conflict handling, accepted mutation, and event staging. Pack data contains no
callback, social interpretation, or authored duration.

## Artifact model

`ArtifactData` is the compiler/decoder domain representation. Its leaf
identifiers and descriptors are checked, but construction does not claim
whole-artifact validity.

`ArtifactValidator`:

1. normalizes semantically unordered collections;
2. validates identifiers, duplicates, references, bindings, stages, and
   semantic limits;
3. resolves exact interface references against the supplied catalog;
4. proves that every resolved interface-table entry is used exactly by the
   artifact's calls;
5. computes `PackExportDigest` and `RuntimeSemanticFingerprint`;
6. deterministically emits `ArtifactBlobV1` for direct data, or retains the
   exact loaded envelope;
7. privately constructs `VerifiedPackArtifact`.

`ArtifactValidator::load` first checks:

- media type, format version, and digest algorithm;
- the single 16 MiB outer byte limit;
- declared versus actual length;
- exact BLAKE3-256 blob digest;
- exact structural schema and end of input.

No general public artifact encoder is exposed.

Exact blob identity and normalized semantic identity remain separate:

```text
ArtifactDigest
  = BLAKE3(exact stored ArtifactBlobV1 bytes)

RuntimeSemanticFingerprint
  = BLAKE3(world-canonical-v1 normalized pack semantics)
```

## Exact package closure

`ExactPackageSelection` is a constructible description containing:

- one root coordinate;
- one selected coordinate per `PackKey`;
- an owner-supplied `SourceSnapshotId`;
- exact direct dependency coordinates.

`ExactPackSet::finalize` proves:

- one selected entry and artifact per `PackKey`;
- exact coordinate and manifest-edge agreement;
- one closed, reachable, acyclic root graph;
- no missing, duplicate, conflicting, or extra package;
- expected dependency export digests match;
- the exact required-interface union.

It privately constructs the matching `PackLock`. Callers cannot pair a lock
with arbitrary artifacts.

`DefinitionLinker` checks only definition-level cross-pack references and
constructs `RuntimeDefinitionSet`.

The identities intentionally separate provenance from runtime definition
identity:

- source changes with identical exact artifacts change `PackLockDigest`;
- exact artifact closure and semantic fingerprints determine
  `RuntimeDefinitionSetDigest`;
- process-local activation indexes have no durable identity.

## Structured authoring

W2 source is programmatic:

```text
PackSource
  source snapshot identity
  exact coordinate
  EngineProtocolVersion
  exact dependency coordinates
  defs-owned action and event input data
```

Compilation:

1. validates the exact source graph before definition references;
2. processes packages in deterministic topological order;
3. obtains dependency export digests;
4. constructs and validates `ArtifactData`;
5. derives `ExactPackageSelection`;
6. finalizes and links the exact set;
7. returns all envelopes plus `RuntimeDefinitionSet`, or one deterministic
   nonempty `DiagnosticSet` and no partial output.

Text spans, warnings, loaders, resolver traits, source maps, and public compiler
passes are deferred.

## Standard/authoring seam

`world-standard` constructs defs-owned declarative data and its interface
descriptor. It does not construct an authoring-owned source, depend on the
compiler, or provide executable semantics.

The two independent tests compare against the same frozen vector:

- `world-authoring` compiles an equivalent transfer-shaped fixture;
- `world-standard` validates its direct declaration.

No Cargo edge between the two crates is needed.

## Target source layout

Files appear only when their responsibility has code:

```text
crates/
  world-core/

  world-defs/
    src/
      lib.rs
      key.rs
      interface.rs
      definition.rs
      artifact/
        mod.rs
        codec.rs
      package.rs
      link.rs

  world-authoring/
    src/
      lib.rs
      source.rs
      compiler.rs
      diagnostic.rs

  world-standard/
    src/
      lib.rs
      transfer.rs
```

Modules split further only after an ownership or file-pressure reason exists.

## Work sequence

### 1. Freeze the W2 protocols

- accept the correctness-focused validation model;
- record D-031;
- freeze identifier grammar, versions, artifact CDDL, tag table, semantic
  normalization, identity preimages, limits, and standard transfer vector.

Status: complete, including the frozen implementation vectors.

### 2. Introduce checked definition values

- add `world-defs` and its exact dependency edges;
- implement keys, versions, semantic-interface descriptors/catalogs, action
  input, requirement calls, effect calls, events, and reusable errors;
- protect local collection and signature invariants with targeted tests.

Status: complete.

### 3. Implement the artifact boundary

- implement private deterministic CBOR emission and ordinary decoding;
- add descriptor, envelope, `ArtifactData`, shared validation, derived
  identities, and sealed artifact;
- compute the frozen standard descriptor and artifact vectors;
- test direct validation and load convergence without compiler byte
  round-tripping.

Status: complete.

### 4. Implement exact closure and linking

- add selection, private lock construction, and exact set finalization;
- add definition linking and read-only lookup;
- compute lock and definition-set vectors;
- test order independence and graph/reference failures.

Status: complete.

### 5. Add structured authoring and the standard transfer

- add deterministic programmatic compilation and diagnostics;
- add pure standard interface and declaration values;
- prove both leaves independently match the same frozen artifact identities.

Status: complete.

### 6. Close the package

- run the full verification matrix and executable dependency allowlist;
- inspect public APIs and docs for authority or dependency leaks;
- record exact completion evidence;
- detail W3 only after every gate passes.

Status: complete.

## Acceptance gates

### Cargo graph

```text
selected local packages =
  { world-core, world-defs, world-authoring, world-standard }

world-defs local dependencies =
  { world-core }

world-authoring local dependencies =
  { world-core, world-defs }

world-standard local dependencies =
  { world-defs }
  or { world-core, world-defs } only if core is directly imported

world-standard -> world-authoring = forbidden
world-authoring -> world-standard = forbidden
```

The only new direct registry dependency is `minicbor` on `world-defs`, with
default features disabled and only `alloc` enabled. Serde and a second
serialization framework are forbidden.

### Domain and API

- distinct identifiers accept exactly their frozen grammars;
- descriptor and catalog construction reject duplicate/conflicting entries;
- direct and decoded `ArtifactData` produce the same normalized definitions
  and semantic fingerprint;
- compiler output is encoded once, not decoded again;
- loaded input receives descriptor, length, digest, schema, and domain checks;
- missing catalog entries, digest mismatch, wrong operation stage/signature,
  invalid binding, empty effect, and invalid event mapping are rejected;
- `VerifiedPackArtifact`, `PackLock`, `ExactPackSet`, and
  `RuntimeDefinitionSet` cannot be publicly constructed;
- exact-set construction cannot accept a caller-supplied lock;
- no durable numeric definition ID or runtime implementation enters an
  artifact or definition set.

### Determinism and closure

- identical structured source produces identical project-emitted bytes,
  artifact digest, semantic fingerprint, lock, and definition-set digest;
- package and definition input order do not alter identity;
- unused catalog entries do not alter validation or identity;
- loader rejects descriptor mismatch, malformed structure, unknown
  schema/tag, wrong arity, trailing bytes, outer-size excess, and semantic
  collection-limit excess;
- missing dependencies, cycles, extra packages, version conflicts, duplicate
  definitions, export mismatch, and interface conflicts are rejected;
- source-snapshot changes alter lock provenance but not identical artifact or
  runtime-definition-set identity;
- the standard artifact contains exactly its transfer requirement, effect, and
  physical event contract.

### Commands

```text
cargo fmt --all --check
cargo check --locked --workspace
cargo clippy --locked --workspace --all-targets
cargo test --locked --workspace
cargo metadata --locked --all-features --format-version 1
cargo tree --locked --workspace --all-features --target all
rg --files -g Cargo.toml
rg -n 'DefinitionId|DefinitionRegistry|VersionAnchor|WorldModel|CausalRuntime|DecisionRunner' crates
git diff --check
```

The full metadata output must also pass this executable allowlist:

```bash
cargo metadata --locked --all-features --format-version 1 |
  jq -e '
    ([.packages[] | select(.source == null) | .name] | sort ==
      ["world-authoring", "world-core", "world-defs", "world-standard"]) and
    (.workspace_members | length == 4) and
    (.workspace_default_members == .workspace_members) and
    ([.packages[] | select(.source == null) |
      {name, dependencies: ([.dependencies[] |
        {name,
         source: (if .source == null then "local" else .source end),
         uses_default_features,
         features}] | sort_by(.name))}] | sort_by(.name) == [
      {name: "world-authoring", dependencies: [
        {name: "world-core", source: "local",
         uses_default_features: true, features: []},
        {name: "world-defs", source: "local",
         uses_default_features: true, features: []}
      ]},
      {name: "world-core", dependencies: [
        {name: "blake3",
         source: "registry+https://github.com/rust-lang/crates.io-index",
         uses_default_features: false, features: []}
      ]},
      {name: "world-defs", dependencies: [
        {name: "minicbor",
         source: "registry+https://github.com/rust-lang/crates.io-index",
         uses_default_features: false, features: ["alloc"]},
        {name: "world-core", source: "local",
         uses_default_features: true, features: []}
      ]},
      {name: "world-standard", dependencies: [
        {name: "world-defs", source: "local",
         uses_default_features: true, features: []}
      ]}
    ]) and
    ([.resolve.nodes[].id as $id |
      .packages[] |
      select(.id == $id) |
      .name] | sort == [
        "arrayref", "arrayvec", "blake3", "cc", "cfg-if",
        "constant_time_eq", "cpufeatures", "find-msvc-tools", "libc",
        "minicbor", "shlex", "world-authoring", "world-core",
        "world-defs", "world-standard"
      ])
  '
```

## Decision triggers

Stop before:

- changing `world-canonical-v1`, the artifact protocol, or any frozen identity
  preimage;
- adding another serialization or compiler-framework dependency;
- adding a hardened hostile-ingestion boundary without a real external
  consumer and explicit policy;
- adding a Cargo edge between `world-authoring` and `world-standard`;
- moving artifact validation outside `world-defs`;
- adding a public sealed-value constructor or deserializer;
- adding definition families or semantic operations without a slice consumer;
- introducing runtime activation or executable transfer authority;
- expanding exact M1 dependencies into ranges, aliases, override order, or a
  registry.

## Completion evidence

```text
implementation commit:
  bdb0e5638a5ac54d1702ad4718b479298d9dc4dc

selected local packages:
  world-core
  world-defs
  world-authoring
  world-standard

direct local dependency graph:
  world-defs      -> world-core
  world-authoring -> world-core, world-defs
  world-standard  -> world-defs

new registry dependency:
  minicbor 2.3.0, default features disabled, alloc only

resolved registry closure:
  arrayref, arrayvec, blake3, cc, cfg-if, constant_time_eq,
  cpufeatures, find-msvc-tools, libc, minicbor, shlex

tracked Cargo manifests:
  Cargo.toml
  crates/world-core/Cargo.toml
  crates/world-defs/Cargo.toml
  crates/world-authoring/Cargo.toml
  crates/world-standard/Cargo.toml
```

Verified results:

- checked domain inputs converge through one catalog-aware artifact validator;
  direct validation encodes once, while loading checks the envelope, decodes,
  and invokes the same semantic function without re-encoding;
- project-emitted deterministic CBOR and an accepted longer-form CBOR integer
  have different exact artifact digests but identical normalized definitions,
  export digests, and runtime semantic fingerprints;
- format, length, digest, outer-size, schema, tag, structural arity, interface
  slot, collection, reference, stage, binding, event, and namespace failures
  are covered at their owning boundaries;
- exact selection proves coordinate correspondence, graph closure, dependency
  exports, engine protocol, and interface union before the linker adds
  cross-pack event existence and signature checks;
- source ordering does not affect compilation, source identity affects only
  lock provenance, and compiler failure returns one deterministic nonempty
  diagnostic set with no partial compilation;
- independent `world-authoring` and `world-standard` fixtures match the same
  403-byte transfer artifact and all seven frozen protocol vectors;
- sealed-value compile-fail tests prove that `VerifiedPackArtifact`,
  `PackLock`, `ExactPackSet`, and `RuntimeDefinitionSet` cannot be publicly
  constructed;
- 50 unit/integration tests and five compile-fail doctests passed;
- locked metadata passed the executable exact allowlist, the dependency tree
  matched the graph above, exactly five Cargo manifests were present, and the
  superseded-symbol scan returned no match;
- formatting, locked workspace check, warning-free Clippy, locked workspace
  tests, warning-free API documentation, metadata, dependency tree, manifest
  scan, and `git diff --check` passed.

The final balance audit removed recoverable error paths for infallible
in-memory encoding and built-in declarations, repeated package-count and
compiler-finalization checks, an undocumented interface-parameter limit, and
unused public DTO decomposition methods. Decoder pre-allocation limits,
semantic collection limits, exact closure checks, and cross-pack link checks
remain because each protects a distinct correctness boundary.

## W3 handoff

The [W3 plan and evidence](milestone-01-work-package-03.md) receives immutable,
sealed, process-independent definition values. It may read checked actions,
requirements, effects, and events from
`RuntimeDefinitionSet`, but cannot bypass artifact validation, exact-set
finalization, or semantic-interface identity. Activation and the trusted
standard implementation remain W4 composition-root work.
