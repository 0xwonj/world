# ArtifactBlobV1 Protocol

## Status and scope

This document is the byte-complete W2 storage and identity contract for
compiled pack artifacts. It refines the artifact boundary owned by
[Extensibility and Research](extensibility-and-research.md) and the validation
policy accepted in [D-031](decisions.md).

The current environment is a trusted local authoring toolchain. Artifact
validation detects ordinary corruption, incompatible schemas, invalid domain
data, and exact-closure mismatches. It is not a hostile-input sandbox.

## Separation of representations

Three representations have different meanings:

```text
ArtifactData
  normalized domain data produced by the compiler or decoder

ArtifactBlobV1
  deterministic CBOR emitted from validated ArtifactData

world-canonical-v1 preimages
  purpose-specific semantic and closure identities
```

`ArtifactBlobV1` is a storage format. It does not implement or extend the
canonical identity writer.

The two construction paths converge on one semantic validator:

```text
compiler:
  ArtifactData
    -> validate(data, catalog)
    -> deterministic encode
    -> descriptor + envelope + VerifiedPackArtifact

loader:
  descriptor + bytes
    -> format/length/digest checks
    -> decode ArtifactData
    -> validate(data, catalog)
    -> VerifiedPackArtifact
```

The compiler does not decode its own output. The loader does not re-encode an
accepted blob merely to prove one possible representation.

## Identifiers

Identifiers contain already-normalized lowercase ASCII. No Unicode
normalization occurs.

```text
segment  = [a-z][a-z0-9]*( "-" [a-z0-9]+ )*
pack-key = segment *( "." segment )
```

| Type | Grammar | Byte length |
|---|---|---:|
| `PackKey` | `pack-key` | 1–127 |
| `SemanticInterfaceKey` | `pack-key` | 1–127 |
| `LocalDefinitionName` | `segment` | 1–63 |
| Operation name | `segment` | 1–63 |
| Binding name | `segment` | 1–63 |
| Event-field name | `segment` | 1–63 |
| Interface-parameter name | `segment` | 1–63 |

Ordering is unsigned ASCII byte order. A prefix sorts before a longer value.
Each public semantic role has its own Rust newtype even when two roles share
the same grammar.

`DefinitionKey` is structural:

```text
DefinitionKey = (PackKey, LocalDefinitionName)
```

It contains no version and has no numeric durable representation.

## Versions and widths

| Value | Domain width |
|---|---:|
| `PackVersion.major`, `.minor`, `.patch` | `u32` |
| Artifact, manifest, family, resolver, linker schema versions | `u16` |
| Engine protocol version | `u16` |
| Semantic-interface version | nonzero `u16` |
| Artifact-local interface slot | `u16` |
| Canonical enum discriminant | `u32` |

CBOR uses the shortest unsigned representation emitted by `minicbor`. The
widths above define the accepted domain range. Canonical identity preimages
use their fixed-width `world-canonical-v1` representations.

W2 pack versions have exactly three numeric components. Prerelease labels,
build metadata, ranges, aliases, and parallel versions of one `PackKey` are
outside this protocol.

## Artifact envelope

The envelope is an in-memory boundary, not a second serialized format.

```text
ArtifactDescriptor
  media type: application/vnd.world.pack+cbor
  artifact format version: u16 = 1
  digest algorithm: blake3-256
  exact blob length: u64
  exact blob digest: 32 bytes

ArtifactEnvelope
  descriptor
  ArtifactBlobV1 bytes
```

The media type is represented by a closed enum in Rust rather than storing an
arbitrary repeated string.

Loading checks:

1. supported media type, format version, and digest algorithm;
2. declared length equals the actual byte length;
3. the byte length does not exceed `MAX_ARTIFACT_BYTES`;
4. BLAKE3-256 of the exact bytes equals the declared digest;
5. one complete `ArtifactBlobV1` value decodes with no trailing bytes;
6. decoded `ArtifactData` satisfies the domain invariants below.

## CBOR profile

The project-owned emitter uses RFC 8949 Core Deterministic CBOR. The schema
uses only:

- definite arrays;
- unsigned integers;
- byte strings;
- UTF-8 strings.

Maps, tags, floating point, negative integers, indefinite items, ignored
fields, and implicit enum representations are outside the schema.

The loader checks exact structural array arity, known schema and family tags,
known value kinds, valid UTF-8, collection limits, and end of input. It does
not reject an otherwise schema-valid value solely because another producer
used a longer valid CBOR integer representation or a different order for a
semantically unordered collection. Validation normalizes those collections.

Consequently, two accepted blobs may have different exact blob digests and
the same runtime semantic fingerprint. Exact storage identity and normalized
behavior identity are intentionally different.

## CDDL

```cddl
uint16 = 0..65535
uint32 = 0..4294967295
digest = bytes .size 32

artifact-blob-v1 = [
  artifact-schema: 1,
  manifest: pack-manifest-v1,
  interfaces: [* interface-ref-v1],
  definitions: [* definition-v1]
]

pack-manifest-v1 = [
  manifest-schema: 1,
  engine-protocol: uint16,
  coordinate: pack-coordinate,
  dependencies: [* dependency-ref-v1]
]

pack-coordinate = [
  pack-key,
  [major: uint32, minor: uint32, patch: uint32]
]

dependency-ref-v1 = [
  coordinate: pack-coordinate,
  expected-export-digest: digest
]

interface-ref-v1 = [
  interface-key,
  interface-version: uint16,
  descriptor-digest: digest
]

definition-v1 = action-v1 / event-v1

action-v1 = [
  family-tag: 0,
  family-schema: 1,
  local-name,
  bindings: [* action-binding-v1],
  requirements: [* operation-call-v1],
  effects: [1* operation-call-v1],
  success-events: [1* event-emission-v1]
]

event-v1 = [
  family-tag: 1,
  family-schema: 1,
  local-name,
  fields: [1* event-field-v1]
]

action-binding-v1 = [binding-name, value-kind]
event-field-v1 = [field-name, value-kind]

operation-call-v1 = [
  interface-slot: uint16,
  operation-name,
  arguments: [* binding-name]
]

event-emission-v1 = [
  event: definition-key,
  fields: [1* event-field-binding-v1]
]

definition-key = [pack-key, local-name]
event-field-binding-v1 = [event-field-name, action-binding-name]

value-kind = 0 / 1
```

## Tags

| Meaning | CBOR value |
|---|---:|
| Action definition | 0 |
| Physical event definition | 1 |
| Actor value | 0 |
| Entity value | 1 |

Semantic-interface operation kinds are not stored on each call. The supplied
catalog declares:

| Operation kind | Meaning |
|---|---|
| `Predicate` | Legal only in `RuntimeRequirement`; returns Boolean |
| `Effect` | Legal only in `EffectProgram`; returns success or a concrete runtime failure |

## Normalization and domain validation

The deterministic emitter writes:

- dependencies sorted and unique by dependency `PackKey`;
- interface references sorted and unique by interface key;
- definitions sorted and unique by local definition name;
- action bindings, event fields, and event field mappings sorted and unique by
  name;
- requirement calls sorted by their normalized representation;
- effect calls and success events in semantic execution order;
- call arguments in descriptor parameter order.

Decoded data is normalized to the same representation. Duplicate names remain
errors. Artifact-local interface slots are resolved to the complete
key/version/digest tuple before normalization.

The shared validator proves:

- the coordinate, identifiers, versions, and collections are valid;
- one local definition namespace is shared across families;
- every direct dependency has a distinct `PackKey`;
- every interface reference resolves to the same key, version, and digest in
  the supplied `SemanticInterfaceCatalog`;
- every interface table entry is used;
- every requirement call resolves to a `Predicate` operation;
- every effect call resolves to an `Effect` operation;
- call arity and binding value kinds match the operation signature;
- every referenced binding exists;
- effects and success-event lists are nonempty;
- every referenced event is exported by this pack or a declared dependency;
- every local event field is mapped exactly once with the matching value kind;
- every collection remains within its semantic limit.

Cross-pack event existence and expected dependency export digests are checked
when the exact artifact set is finalized and linked.

## Semantic-interface descriptors

Descriptors live in the supplied `SemanticInterfaceCatalog`. Artifacts store
only exact references:

```text
SemanticInterfaceReference
  key
  nonzero u16 version
  SemanticInterfaceDigest
```

A descriptor contains:

```text
SemanticInterfaceDescriptor
  key
  version
  operations sorted and unique by name

SemanticOperationDescriptor
  operation name
  Predicate | Effect
  ordered parameters:
    parameter name
    Actor | Entity
```

Descriptors contain no callback, runtime implementation, compiler hook,
generic value tree, or embedded copy inside an artifact. W2 bounds operation
count and effect-call count. Generalized execution cost metadata is deferred
until runtime work budgeting has a concrete consumer.

An unused catalog superset has no effect:

```text
restrict(C, required(D)) = restrict(C', required(D))
  implies
validate(D, C) = validate(D, C')
```

## Canonical identities

Every semantic identity below uses `world-canonical-v1` and BLAKE3-256. The
artifact blob digest alone hashes exact storage bytes directly.

### Semantic-interface digest

Domain:

```text
semantic-interface-v1
```

Preimage fields:

```text
u16 descriptor schema = 1
interface key
u16 interface version
operations sorted by name:
  operation name
  u32 operation-kind tag
  ordered parameters:
    parameter name
    u32 value-kind tag
```

### Pack export digest

Domain:

```text
pack-exports-v1
```

Preimage fields:

```text
u16 export schema = 1
pack coordinate
definitions sorted by local name:
  u32 family tag
  u16 family schema
  local name
  public signature:
    Action -> sorted bindings (name, value kind)
    Event  -> sorted fields (name, value kind)
```

All W2 definitions are exported. Requirements, effects, and event mappings are
not part of the export surface.

### Runtime semantic fingerprint

Domain:

```text
pack-semantics-v1
```

Preimage fields:

```text
u16 semantic schema = 1
u16 engine protocol version
pack coordinate
dependency references sorted by PackKey:
  exact coordinate
  expected export digest
interface references sorted by key:
  key
  version
  descriptor digest
definitions sorted by local name:
  complete normalized family body
```

Calls write the complete interface tuple rather than an artifact-local slot.
Source identity, CBOR bytes, and the artifact descriptor are excluded.

### Pack-lock digest

Domain:

```text
pack-lock-v1
```

Preimage fields:

```text
u16 lock schema = 1
u16 resolver version = 1
root coordinate
entries sorted by PackKey:
  coordinate
  source snapshot ID bytes
  u16 artifact format version
  u64 artifact byte length
  artifact blob digest
  export digest
  direct dependencies sorted by PackKey:
    exact coordinate
    dependency artifact digest
    dependency export digest
required interface closure sorted by key:
  key
  version
  descriptor digest
```

### Runtime-definition-set digest

Domain:

```text
runtime-definition-set-v1
```

Preimage fields:

```text
u16 linker schema = 1
u16 engine protocol version
root coordinate
artifacts sorted by PackKey:
  coordinate
  u16 artifact format version
  artifact blob digest
  runtime semantic fingerprint
required interface closure sorted by key:
  key
  version
  descriptor digest
```

The definition-set digest intentionally excludes source snapshots and the
pack-lock digest. Obtaining the same exact artifact closure from another source
changes resolution provenance and `PackLockDigest`, not runtime definition
identity.

### Blob and source identities

```text
ArtifactDigest = BLAKE3(exact ArtifactBlobV1 bytes)
```

`SourceSnapshotId` is a distinct 32-byte content identity supplied by the
source owner. W2 does not invent a universal source serialization solely to
derive it.

## Exact selection, lock, and set

Before compilation:

```text
ExactPackageSelection
  root coordinate
  packages sorted by PackKey:
    coordinate
    SourceSnapshotId
    direct dependency coordinates
```

After every artifact digest exists, `ExactPackSet::finalize` checks:

- one selected entry and one artifact per selected `PackKey`;
- exact coordinate and direct-dependency agreement;
- one root and a closed, reachable, acyclic graph;
- no missing, duplicate, conflicting, or extra package;
- each expected dependency export digest matches the selected dependency;
- the required-interface closure equals the union of artifact requirements.

It then privately constructs:

```text
PackLock
  schema and resolver versions
  exact source and artifact closure
  required interface closure
  PackLockDigest

ExactPackSet
  private PackLock
  verified artifacts sorted by PackKey
```

`ExactPackSet` has no independent digest and accepts no caller-supplied lock.

`DefinitionLinker` checks definition-level cross-pack references and constructs
an immutable `RuntimeDefinitionSet`. Process-local numeric IDs and dispatch
tables remain activation concerns.

## Practical limits

These are format sanity and execution-shape limits, not an adversarial
resource-accounting system:

| Limit | Value |
|---|---:|
| Artifact bytes | 16 MiB |
| Packages in one exact selection | 256 |
| Direct dependencies per pack | 128 |
| Required interfaces per pack | 128 |
| Operations per interface | 256 |
| Definitions per artifact | 4,096 |
| Bindings per action | 32 |
| Requirements per action | 64 |
| Effects per action | 256 |
| Success events per action | 32 |
| Fields per event | 32 |
| Arguments per operation call | 32 |

There is no recursive expression representation in W2, so there is no generic
depth, node, fuel, or total-allocation budget. A decoded array length is
checked against its corresponding semantic limit before collection
allocation.

## Standard transfer vector

The first standard declaration is:

```text
pack: world.standard@1.0.0
engine protocol: 1
dependencies: none

required interface: world.standard.transfer@1

Predicate can-transfer-item(
  actor: Actor,
  item: Entity,
  source: Entity,
  destination: Entity
)

Effect transfer-item(
  actor: Actor,
  item: Entity,
  source: Entity,
  destination: Entity
)

Event item-transferred
  actor: Actor
  destination: Entity
  item: Entity
  source: Entity

Action transfer-item
  bindings:
    actor: Actor
    destination: Entity
    item: Entity
    source: Entity
  requirements:
    can-transfer-item(actor, item, source, destination)
  effects:
    transfer-item(actor, item, source, destination)
  success event:
    item-transferred {
      actor <- actor
      destination <- destination
      item <- item
      source <- source
    }
```

The later trusted implementation determines current containment, actor
authority, destination validity, capacity, and conflict behavior. It
revalidates and atomically changes containment before emitting the physical
event. The artifact contains no gift, theft, coercion, gratitude, trust,
social interpretation, or authored duration.

## Required vectors and tests

W2 freezes:

- one semantic-interface descriptor preimage and digest;
- one deterministic `ArtifactBlobV1` byte vector and blob digest;
- the standard pack export digest and runtime semantic fingerprint;
- one single-pack lock digest;
- one runtime-definition-set digest.

Tests cover deterministic emission, ordinary encode/load round trips,
descriptor length/digest mismatch, unsupported schema/tag, wrong array arity,
trailing bytes, semantic collection boundaries, invalid identifiers,
references, stages, duplicates, graph closure, and linking.

Mutation fuzzing, hostile allocation accounting, signature policy, and
hardened streaming ingestion are not W2 completion gates.
