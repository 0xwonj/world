# M1/W2: Definitions, Verified Artifacts, and Structured Authoring

## Status

Proposed. W1 is complete. W2 begins after the artifact-format dependency
decision in this plan is approved.

## Goal

Introduce the smallest complete immutable-content path:

```text
structured pack source
  -> exact source selection + checked pack declarations
  -> deterministic artifact bytes
  -> untrusted ArtifactEnvelope
  -> sealed VerifiedPackArtifact
  -> exact PackLock and ExactPackSet
  -> immutable RuntimeDefinitionSet
```

The slice also defines the pure standard transfer vocabulary needed by later
runtime work. It does not execute or activate that vocabulary.

## Non-goals

- textual syntax, a public parser API, or a final pack DSL;
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
- [Cognition and Agency](../../architecture/target-architecture/cognition-and-agency.md)
- [Validation Scenarios](../../architecture/target-architecture/validation-scenarios.md)
- [M0 preservation and baseline](milestone-00-preservation-and-baseline.md)
- [Completed W1 plan and evidence](milestone-01-work-package-01.md)

## Current-state evidence

W1 leaves one selected local package, `world-core`, with the frozen
`world-canonical-v1` identity-preimage protocol and BLAKE3-256 content
digests. There is no selected definition, authoring, standard, artifact, or
runtime package.

The target documents already fix the ownership and trust transitions, but do
not yet fix:

- identifier grammars and widths;
- the exact `ArtifactBlobV1` byte format;
- artifact decoder limits and unknown-field behavior;
- canonical identity schemas for descriptors, fingerprints, locks, and sets;
- the exact minimum action, condition, effect, event, and transfer schemas;
- the source-snapshot identity carried into an exact lock.

Those are W2 decisions. They must become byte-complete before artifact code is
treated as stable.

## Package boundaries

The selected production graph becomes:

```text
world-defs      -> world-core, minicbor
world-authoring -> world-core, world-defs
world-standard  -> world-core, world-defs
```

There is no dependency between `world-authoring` and `world-standard`.

| Concern | Owner |
|---|---|
| Pack-qualified keys and checked definition values | `world-defs` |
| Semantic-interface descriptors and exact catalog values | `world-defs` |
| Artifact bytes, reverification, sealed artifacts, locks, exact sets, linking | `world-defs` |
| Structured source, exact source graph, lowering, and authoring diagnostics | `world-authoring` |
| Stable transfer keys, descriptor, declarations, and physical event vocabulary | `world-standard` |
| Installed transfer implementation and runtime dispatch | deferred `world-standard-runtime` in W4 |
| Activation and process-local IDs | deferred `world-runtime` in W4 |

`world-defs` owns the full untrusted-to-trusted artifact boundary. The
authoring plane can produce bytes but cannot mint a trusted artifact through a
second path. The standard package contains no callback, mutable runtime
access, or implementation registry.

## Core abstractions

The architecture is explained by four irreversible validation transitions:

```text
PackSource
  --compile--> ArtifactEnvelope (still untrusted)

ArtifactEnvelope
  --reverify--> VerifiedPackArtifact

ExactPackageSelection + VerifiedPackArtifact[]
  --finalize--> ExactPackSet (including its private PackLock)

ExactPackSet
  --link--> RuntimeDefinitionSet
```

Only the transitions that add a real invariant receive a distinct type.
Parsing, name resolution, family checking, and lowering remain private
functions until one of them has an independently useful producer, consumer,
or test boundary.

The public operations remain narrow:

```text
AuthoringCompiler::compile(CompileRequest) -> Compilation

ArtifactVerifier::reverify(ArtifactEnvelope)
  -> Result<VerifiedPackArtifact, ArtifactError>

ExactPackSet::finalize(exact_selection, verified_artifacts)
  -> Result<ExactPackSet, LinkError>

DefinitionLinker::link(ExactPackSet)
  -> Result<RuntimeDefinitionSet, LinkError>
```

`ExactPackageSelection` is an untrusted, process-independent description of
the resolved root, exact package coordinates, exact source-snapshot
identities, and direct dependency edges. `ExactPackSet::finalize` proves that
selection against the artifacts and privately creates the matching
`PackLock`; callers cannot pair an arbitrary lock with artifacts.
`DefinitionLinker::link` consumes that proof-carrying set. The compiler's
encoder must feed its output through `ArtifactVerifier::reverify`; compiler
output receives no privileged trust path.

## Proposed artifact-format decision

### Recommendation

Use a strict profile of RFC 8949 Core Deterministic CBOR for
`ArtifactBlobV1`, implemented with explicit, owner-written `minicbor` calls:

```toml
minicbor = { version = "2.3.0", default-features = false, features = ["alloc"] }
```

The dependency has no default features. W2 enables neither derives, Serde,
`std`, nor half-precision floating point. The artifact codec remains private
to `world-defs`; no general encoding trait is added to domain types.

The selected profile permits only:

- definite-length arrays;
- UTF-8 text and byte strings;
- schema-selected bounded unsigned integers;
- booleans;
- explicit array-shaped variant and option encodings;
- owner-validated ordered sequences.

It forbids:

- CBOR maps;
- floating-point values;
- negative integers;
- tags;
- indefinite-length items;
- unknown variants or fields;
- ignored trailing bytes.

Every structural array has an exact arity. The decoder checks the outer byte
limit before allocation, enforces fixed depth, node, string-byte, sequence,
and total-allocation budgets, rejects duplicate or incorrectly ordered owner
collections, requires exact end of input, then re-encodes and compares the
original bytes byte-for-byte. Semantic verification follows only after those
checks.

The structural schema is specified in checked-in CDDL plus normative profile
text and language-independent golden vectors. CDDL documents the accepted
tree; the profile text fixes the additional deterministic and rejection rules.

### Why this boundary

RFC 8949 defines shortest-form integers and lengths, definite-length items,
and deterministic ordering. Restricting the application profile to arrays and
non-floating scalar values removes the format's remaining ordering and numeric
ambiguities. `minicbor` provides a small, type-directed, non-allocating decoder
over borrowed input while leaving domain limits and validation under
`world-defs`.

This is preferable to:

- a bespoke codec, which offers control but creates a new cross-language wire
  standard and parser burden;
- Postcard, whose stable format still accepts non-minimal integer encodings
  and delegates schema evolution outside the format;
- Protocol Buffers, whose own documentation says deterministic serialization
  is not canonical across implementations or versions;
- `rkyv`, whose archived representation is Rust- and schema-layout-oriented;
- `bincode`, whose current release line is explicitly unmaintained.

Primary research:

- [RFC 8949: CBOR, including deterministic encoding](https://www.rfc-editor.org/rfc/rfc8949.html)
- [RFC 8610: CDDL](https://datatracker.ietf.org/doc/html/rfc8610)
- [`minicbor` 2.3.0 package metadata](https://crates.io/crates/minicbor/2.3.0)
- [`minicbor` decoder API](https://docs.rs/minicbor/2.3.0/minicbor/decode/struct.Decoder.html)
- [Postcard wire-format specification](https://postcard.jamesmunns.com/wire-format)
- [Protocol Buffers canonicalization warning](https://protobuf.dev/programming-guides/serialization-not-canonical/)
- [`rkyv` format-control and compatibility documentation](https://docs.rs/rkyv/latest/rkyv/)
- [`bincode` project status](https://docs.rs/crate/bincode/latest)

### Identity separation

The three byte domains remain distinct:

```text
artifact blob digest
  = BLAKE3(exact accepted ArtifactBlobV1 CBOR bytes)

runtime semantic fingerprint
  = BLAKE3(world-canonical-v1 preimage over normalized behavior)

definition-set and related semantic digests
  = BLAKE3(world-canonical-v1 owner-defined preimages)
```

The external `ArtifactDescriptor` carries the artifact format version,
algorithm, blob length, and blob digest. It is not included in its own digest.
W2 does not define a second wire encoding for the envelope.

Unknown material is rejected within a format version. Future evolution uses a
new explicit container or family schema version and a new verifier; an old
verifier never seals behavior it cannot interpret.

This decision changes the artifact storage format and adds a dependency, so it
requires explicit approval before Cargo or normative architecture documents
are changed.

## Definition vocabulary

### Durable keys

W2 freezes checked, bounded ASCII forms for:

- `PackKey`;
- `PackVersion { major, minor, patch }`;
- `PackCoordinate`;
- `LocalDefinitionName`;
- `DefinitionKey = PackKey + LocalDefinitionName`;
- `SemanticInterfaceKey` and operation names;
- action binding and event field names.

Version components have fixed integer widths and no prerelease or build
syntax. Dependencies select exact `PackCoordinate` values. A pack has one
local definition namespace across families, and durable numeric definition
IDs do not exist.

The schema specification must state each alphabet, maximum length, ordering,
and canonical identity field order before constructors are implemented.

### Semantic interfaces

`SemanticInterfaceDescriptor` is immutable declarative data:

- exact interface key and version;
- typed operation signatures;
- legal definition family and authority stage;
- read/predicate or domain-effect authority classification;
- deterministic fixed cost and structural limits;
- exact descriptor digest.

W2 needs only actor/entity binding values and the operations exercised by the
standard transfer. Descriptors contain no compiler hook or runtime callback.
An artifact embeds the exact verifier-complete descriptors it uses, allowing
independent reverification. Later activation must still bind every descriptor
digest to an installed trusted implementation.

A `SemanticInterfaceCatalog` is an immutable checked collection used during
authoring. Only the exact transitive descriptor closure actually referenced by
a pack enters its artifact identity; an unused catalog superset changes
nothing.

### Minimum checked families

Only four definition responsibilities enter W2:

- named typed action bindings and one checked action definition;
- stage-specific `RuntimeRequirement`, with no stage-erased condition root;
- a nonempty, ordered, bounded effect program of semantic-interface calls;
- a physical event schema and checked success-field mapping.

The first standard declaration binds actor, item, source, and destination,
checks the required binding relations, invokes the exact transfer interface,
and declares an item-transferred physical event. It records no gift,
coercion, gratitude, trust, or other actor-relative interpretation.

Ownership, capacity, resource conflict, accepted mutation, and event staging
remain the responsibility of the trusted W4 transfer implementation and
runtime revalidation. W2 defines the contract they must later implement; it
does not simulate that authority in pack data.

The transfer has no real authored-time consumer in M1, so W2 does not add an
unused timing discriminator or duration field.

## Artifact, lock, and set invariants

### Artifact verification

`ArtifactEnvelope` is always untrusted. Reverification performs, in order:

1. descriptor media-type, algorithm, version, and outer-length checks;
2. exact BLAKE3 verification over the original blob;
3. bounded strict `ArtifactBlobV1` decode;
4. exact CBOR re-encoding equality;
5. identifier, manifest, family, reference, stage, cost, and limit checks;
6. embedded semantic-interface descriptor and closure checks;
7. runtime semantic fingerprint recomputation;
8. private construction of `VerifiedPackArtifact`.

A signature, cached origin, or compiler origin cannot skip a step.

### Exact package closure

M1 uses one exact artifact per `PackKey`, exact three-part versions, exact
dependency coordinates, one root, and an acyclic closed graph. Missing,
duplicate, conflicting, unreachable extra, or cyclic artifacts fail.
Load/input order has no semantic meaning.

`PackLock` records:

- resolver and schema versions;
- the root coordinate;
- sorted package coordinates and exact source-snapshot identities;
- exact artifact and export digests;
- exact resolved direct dependency edges;
- exact required semantic-interface digests.

All W2 definitions are explicitly exported. Import aliases, visibility rules,
ranges, and partial overrides are deferred.

Source-snapshot identity is graph provenance, not executable behavior. It
enters `ExactPackageSelection` and `PackLock`, not `ArtifactBlobV1` or the
runtime semantic fingerprint. This lets independently obtained identical
artifact bytes retain distinct resolution provenance without changing their
behavior-equivalence fingerprint.

### Linking

`ExactPackSet::finalize` rechecks root closure and creates the lock only after
all blob digests are known. `DefinitionLinker` rechecks lock/artifact
correspondence, direct edges, exported-symbol closure, global duplicate keys,
cross-pack interface consistency, and the exact required-interface union.

`RuntimeDefinitionSet` privately owns the immutable linked body and exposes
only read-only identity and lookup operations. Its digest is:

```text
BLAKE3(
  world-canonical-v1 domain
  + canonical RuntimeDefinitionSet body without its digest
)
```

Activation tables, numeric intern IDs, caches, dispatch functions, and mutable
registries are absent.

## Target source layout

Files appear only when their responsibility is implemented:

```text
crates/
  world-core/

  world-defs/
    src/
      lib.rs
      key.rs
      interface.rs
      condition.rs
      effect.rs
      action.rs
      event.rs
      artifact/
        mod.rs
        codec.rs
        verify.rs
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

Modules may remain combined while small. This layout is an ownership map, not
a requirement to create empty files.

## Work sequence

### 1. Freeze the W2 protocols

- accept the artifact-format decision;
- add an architecture decision separating deterministic artifact storage from
  canonical semantic identity;
- specify identifier grammars, integer widths, sort orders, domain labels,
  artifact tag tables, CDDL, and fixed verifier limits;
- add one manually reviewable golden artifact byte vector and expected digest
  before implementing its decoder.

### 2. Introduce checked definition values

- add `world-defs` and its exact dependency edges;
- implement keys, exact versions and dependencies, semantic-interface
  descriptors/catalogs, and the four minimum definition responsibilities;
- protect every collection and reference invariant with checked constructors
  and targeted tests.

### 3. Implement the artifact trust boundary

- add private explicit CBOR encoding and bounded decoding;
- add the untrusted descriptor/envelope and sealed verified artifact;
- force emitted artifacts through the same reverification path as loaded
  bytes;
- add canonicality, tamper, limit, unknown-value, and no-trailing-byte tests.

### 4. Implement exact closure and linking

- validate a defs-owned exact package selection supplied by authoring or a
  later artifact resolver;
- finalize a proof-carrying exact set and private matching lock;
- link exact symbols and semantic-interface closure;
- compute frozen lock, export, semantic fingerprint, and definition-set
  identity vectors;
- add graph-order, duplicate, conflict, cycle, missing, and extra-package
  tests.

### 5. Add structured authoring and the standard transfer

- add the generic programmatic source and deterministic compiler;
- return complete output or deterministic owner-specific diagnostics, never a
  partial trusted set;
- add pure standard transfer keys, interface descriptor, checked declaration,
  and physical event vocabulary;
- prove authoring and standard construction reach the same frozen artifact
  identity by comparing each side with the same golden vector, without adding
  any dependency between their packages.

### 6. Close the package

- run the full verification matrix;
- inspect public documentation and Cargo metadata for forbidden authority or
  dependency edges;
- reconcile the implementation with the formal model and target code
  architecture;
- record exact completion evidence and only then detail W3.

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
  { world-core, world-defs }

world-standard -> world-authoring = forbidden
world-authoring -> world-standard = forbidden
```

The only new direct registry dependency is approved `minicbor` with default
features disabled and only `alloc` enabled. The resolved transitive closure is
recorded and allowlisted after `Cargo.lock` is generated. Serde and another
serialization framework are forbidden.

### Trust and API

- `VerifiedPackArtifact`, `PackLock`, `ExactPackSet`, and
  `RuntimeDefinitionSet` cannot be publicly constructed or deserialized;
- compiler-produced bytes and loaded bytes use one reverification path;
- exact-set construction cannot accept a caller-supplied mismatched lock;
- source diagnostics, artifact failures, and link failures remain concrete
  owner-specific types;
- no numeric durable definition ID or runtime implementation object enters an
  artifact or definition set.

### Determinism and rejection

- identical structured source produces identical artifact bytes, artifact
  digest, semantic fingerprint, lock, and definition-set digest;
- package, definition, and catalog input order do not alter identity;
- unused installed/catalog interfaces do not alter identity;
- tampered bytes, descriptor mismatch, non-shortest CBOR, indefinite items,
  wrong arity, trailing bytes, unknown values, excessive depth/count/bytes,
  duplicates, and incorrect ordering fail closed;
- missing dependencies, cycles, extra packages, version conflicts, duplicate
  definitions, missing exports, and interface key/digest conflicts fail;
- invalid binding references, illegal stages, empty effects, and invalid event
  mappings fail before sealing;
- the standard artifact references exactly its transfer interface and physical
  event contract.

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

The metadata and tree outputs receive executable exact allowlists after the
dependency lock is created.

## Decision triggers

Stop before:

- adding `minicbor` or accepting deterministic CBOR without explicit approval;
- changing `world-canonical-v1` or using it as the artifact storage codec;
- adding another serialization or compiler-framework dependency;
- adding a production `world-authoring`/`world-standard` edge;
- moving artifact verification outside `world-defs`;
- adding a public trusted-value constructor or deserializer;
- adding definition families or semantic operations without a transfer-slice
  consumer;
- introducing runtime activation or executable transfer authority;
- expanding exact M1 dependencies into ranges, aliases, override order, or a
  registry.

## Completion evidence

To be filled after every gate passes.

## W3 handoff

W3 receives only immutable, sealed, process-independent definition values. It
may read checked actions, requirements, effects, and events from
`RuntimeDefinitionSet`, but cannot bypass exact artifacts, link proof, or
semantic-interface identity. Activation and the trusted standard
implementation remain W4 composition-root work.
