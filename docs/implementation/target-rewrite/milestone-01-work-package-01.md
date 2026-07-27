# M1/W1: Workspace Cutover and Canonical Core

## Status

Active.

## Goal

Make the selected Cargo graph contain only a compile-clean target-shaped
`world-core`, and replace the old allocation-label foundation with the exact
canonical bytes, digest, cross-plane identity, virtual-time, and revision
primitives required by later M1 work.

## Non-goals

- retaining compatibility with current core names;
- implementing definitions, artifacts, model records, runtime authority, or
  engine facades;
- adding dependency witnesses, authoring diagnostics, provenance records, or
  budget families before their first cross-package consumer;
- defining storage or wire serialization;
- creating empty target crates.

## Current evidence

- rewrite branch base: `4bc71b7`;
- M0 completion commit: `068cf47`;
- the legacy workspace selects ten packages;
- the current `world-core` exports durable numeric IDs, monotonic ID issuers,
  `VersionAnchor`, broad authority enums, store cursors, and the old
  `SimulationTime` vocabulary;
- downstream legacy crates still path-depend on those surfaces.

The old downstream packages remain recoverable on the preservation branch.
W1 deletes their directories from the rewrite tree; target-shaped packages
return only with their first real producer, consumer, and invariant test.

## Approved dependency

Add the official `blake3` crate at the workspace boundary:

```toml
blake3 = { version = "1.8.5", default-features = false }
```

The lockfile pins the resolved implementation. Durable semantics are pinned by
`world-canonical-v1`, `blake3-256`, and language-independent golden vectors,
not by a crate version string.

No other dependency is added.

## Target module surface

```text
world-core/src/
  lib.rs
  canonical.rs
  content.rs
  identity.rs
  time.rs
  revision.rs
```

Modules listed in the final ownership map but lacking a W1 producer/consumer
are deferred:

- `dependency.rs` until projection/runtime freshness contracts;
- `provenance.rs` and `diagnostic.rs` until artifacts/authoring;
- `budget.rs` until T1 verification or runtime work limits.

### Canonical bytes

`CanonicalDomain` validates a 1–64 byte ASCII label matching
`[a-z][a-z0-9-]{0,63}`.

`CanonicalWriter`:

- starts every preimage with the literal ASCII bytes `world-canonical-v1`,
  followed by the checked domain's `u64` big-endian byte length and exact
  bytes;
- writes only explicit `u8`/`u16`/`u32`/`u64`/`u128` values in big-endian
  order, one-byte booleans, owner-selected `u32` enum discriminants,
  `u64`-byte-length-framed bytes/UTF-8, one-byte option tags, and ordered
  slices whose `u64` element counts are derived by the writer;
- performs no implicit ordering or Unicode normalization;
- yields private-constructor `CanonicalBytes`;
- exposes no blanket serialization or universal `CanonicalEncode` trait.

Callers own schema field order, versioned domain labels, and
map-to-validated-sorted-sequence conversion.

### Content digests

`DigestAlgorithm::Blake3_256` exposes the stable artifact identifier
`blake3-256`.

`ContentDigest`:

- is exactly 32 bytes;
- hashes arbitrary exact blob bytes or completed canonical bytes with
  standard unkeyed BLAKE3 and its default 32-byte output;
- supports byte access and lowercase hexadecimal display;
- carries no authority and has no self-referential constructor.

### Cross-plane identities

`EntityId` and `ActorId` are distinct 32-byte representation types. They
replace numeric allocation labels and issuers, but do not claim to verify
their own semantic derivation: future entity and actor schemas own that rule.
They expose no conversion from each other or from a generic content digest.
Definition, attempt, record, process, lifecycle, and research IDs stay with
their future owners.

### Virtual time

Use one vocabulary consistently:

```text
SimTime       integer coordinate in session-selected ticks
SimDuration   checked nonnegative tick distance
Microstep     same-time causal index
SimMoment     ordered pair (SimTime, Microstep)
```

`SimTick` in the runtime architecture document is renamed to `SimTime`.
Floating point and wall time are absent. Configuration-independent authored
duration is deferred to W2, where it has a compiler producer and activation
consumer.

### Revision

`WorldRevision` includes the root revision and advances only through checked
successor construction. `NonZeroWorldRevision` proves only that the numeric
coordinate is nonzero; a future sealed runtime record proves publication.
Authority-record sequence types remain runtime-owned and are introduced with
their first record consumer.

## Work

1. Replace the root workspace membership with `world-core` only and delete
   every old downstream package directory.
2. Add the approved BLAKE3 workspace dependency and regenerate `Cargo.lock`.
3. Delete the old `world-core` modules and public exports.
4. Implement the six target modules above.
5. Add black-box integration tests for canonical bytes, official BLAKE3
   vectors, domain separation, ID type separation, time arithmetic, moment
   ordering, and revision overflow.
6. Align the `SimTime` terminology in the normative runtime document.
7. Verify Cargo metadata selects only `world-core` and BLAKE3's registry
   dependency closure.
8. Record completion evidence and draft W2 only after the gates pass.

## Acceptance gates

### Cargo graph

```text
selected local packages = { world-core }
world-core local dependencies = {}
old path dependencies selected = 0
```

The registry dependency closure may contain only BLAKE3 and its transitive
implementation dependencies.

### API and invariant tests

- invalid canonical domains fail;
- distinct domains produce distinct bytes and digests for equal fields;
- lengths and unsigned integers use the frozen big-endian representation;
- exact UTF-8 bytes are preserved without normalization;
- the official BLAKE3 empty-input vector matches;
- a frozen complete canonical preimage byte vector and digest match;
- `EntityId` and `ActorId` are not interchangeable;
- time and revision arithmetic fail on overflow;
- `SimMoment` ordering is lexicographic by time then microstep;
- no old public symbol remains in selected source.

### Commands

```text
cargo fmt --all --check
cargo check --locked --workspace
cargo clippy --locked --workspace --all-targets
cargo test --locked --workspace
cargo metadata --locked --no-deps --format-version 1
cargo metadata --locked --all-features --format-version 1
cargo tree --locked --workspace --all-features --target all
rg --files -g Cargo.toml
rg -n 'DefinitionId|VersionAnchor|WorldModel|CausalRuntime|DecisionRunner|StoreCursor|QueryEpoch|SimulationTime' crates
git diff --check
```

The full metadata output must also pass this executable allowlist:

```bash
cargo metadata --locked --all-features --format-version 1 |
  jq -e '
    ([.packages[] | select(.source == null) | .name] |
      sort == ["world-core"]) and
    (.workspace_members | length == 1) and
    (.workspace_default_members == .workspace_members) and
    ([.packages[] |
      select(.source == null and .name == "world-core") |
      .dependencies[] |
      {name, source, uses_default_features, path: (.path // null)}] == [{
        name: "blake3",
        source: "registry+https://github.com/rust-lang/crates.io-index",
        uses_default_features: false,
        path: null
      }]) and
    ([.resolve.nodes[].id as $id |
      .packages[] |
      select(.id == $id) |
      .name] | sort == [
        "arrayref", "arrayvec", "blake3", "cc", "cfg-if",
        "constant_time_eq", "cpufeatures", "find-msvc-tools",
        "libc", "shlex", "world-core"
      ])
  '
```

## Decision triggers

Stop before:

- changing `world-canonical-v1` or BLAKE3-256;
- introducing a general serialization dependency or canonical trait;
- retaining a legacy alias or numeric issuer;
- moving a downstream identity into `world-core`;
- adding another dependency;
- restoring a deleted legacy package instead of introducing its target-shaped
  replacement.

## Completion evidence

To be filled after verification.

## W2 handoff

W2 may depend only on the final public surface demonstrated here. It will add
the first real producers and consumers for pack keys, definition identities,
artifact provenance/diagnostics, exact authored duration, and deterministic T1
limits.
