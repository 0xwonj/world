# M0: Preservation and Clean Rewrite Baseline

## Status

Complete.

## Goal

Preserve the complete pre-redesign workspace, establish a clean rewrite branch
from the original baseline, make the target architecture durable in Git,
record the legacy baseline without repairing it, and make M1 executable.

## Non-goals

- changing Rust production code;
- deleting the legacy implementation;
- changing the Cargo workspace graph;
- selecting a persistence backend;
- implementing target crates or compatibility paths.

## Normative contracts

- [Target Architecture Package](../../architecture/target-architecture/README.md)
- [Target Rust Code Architecture](../../architecture/target-architecture/code-architecture.md)
- [Execution Roadmap](../../architecture/target-architecture/implementation-roadmap.md)

## Current-state evidence

Original baseline:

```text
branch: main
commit: 4bc71b7
remote relation: origin/main, zero commits ahead or behind
workspace packages: 10
```

The complete command output summary, package graph, and superseded-symbol
inventory are recorded in
[`baseline-2026-07-27.md`](baseline-2026-07-27.md).

The pre-redesign working tree contained:

```text
27 modified tracked files
51 untracked files
78 changed paths in the complete snapshot
no staged files before preservation
no non-ignored binary, symlink, executable, or file above 1 MiB
ignored target/ build output excluded
```

The targeted credential scan found no private-key header, common provider
token, `.env` file, or literal credential assignment. This was not a
full entropy-based secret audit because `gitleaks` was unavailable.

## Decisions fixed for M1

### Canonical identity protocol

M1 uses a versioned, purpose-built canonical byte protocol rather than a
general serialization format.

```text
protocol: world-canonical-v1
digest: BLAKE3-256
domain separation: mandatory ASCII identity-domain label in every preimage
integers: fixed-width unsigned big-endian
booleans: one byte, 0 or 1
enums: checked u32 discriminant
bytes and UTF-8 strings: u64 byte length followed by exact bytes
optional values: explicit absent/present tag
sequences: u64 element count followed by ordered elements
maps: forbidden in identity preimages; callers canonicalize to sorted sequences
floats: forbidden in identity preimages
Unicode normalization: none inside the protocol; owning identifier types
  validate their accepted alphabet before encoding
```

The owner writes fields explicitly. Identity is never derived from Rust memory
layout, `Hash`, debug formatting, or a convenience serializer. Golden vectors
must be language-independent. Adding the Rust `blake3` crate is a dependency
decision executed in M1 under the repository approval rule.

### First standard interaction

The first slice transfers one exclusively owned item from a source container
to a destination container for one actor.

It must exercise:

```text
exact standard pack
  -> action and authoritative condition
  -> trusted transfer primitive
  -> controller admission and delivery trigger
  -> staged Fire
  -> ownership/resource revalidation
  -> accepted relation change
  -> domain event
  -> nonempty reaction envelope
  -> atomic authority publication
  -> read-only inspector query
```

The public input is a target-shaped `ControllerRequest`. No public raw runtime
command or test-only feature is introduced.

### Minimum source and artifact surface

M1 uses a programmatic/structured source model; textual syntax is deferred.

The first artifact schema contains only:

- `PackManifestV1`;
- exact `PackKey` and three-component semantic version;
- exact dependency requirements for M1;
- action definitions;
- stage-specific authoritative conditions;
- effect programs over installed semantic interfaces;
- event definitions;
- required interface and export digests;
- deterministic size, nesting, and evaluation limits.

Process definitions, general version ranges, patch/override artifacts, a
package registry, signing service, and custom compiler hooks are deferred until
they have an immediate consumer.

## Work packages

1. Preserve the complete non-ignored workspace on a dedicated branch and
   verify its commit.
2. Create a rewrite branch from `4bc71b7`.
3. Restore only the target architecture, architecture-status updates, and
   redesign research.
4. Record baseline build, test, dependency, and superseded-symbol evidence.
5. Replace the distant detailed implementation plan with the coarse execution
   roadmap and establish this operational planning directory.
6. Write the draft M1 plan with work packages, deletion set, decisions, and
   binary gates.
7. Commit and verify the clean M0 result.

## Acceptance gates

- preservation commit is reachable on its named branch;
- rewrite branch is based on `4bc71b7`;
- target documents are tracked and local links resolve;
- the legacy baseline result is recorded exactly;
- M1 decisions and plan are present;
- `git diff --check` passes;
- the M0 commit leaves a clean working tree.

## Completion evidence

```text
preservation branch:
  codex/preserve-pre-redesign-2026-07-27

preservation commit:
  6571dc8442f4450e067d60c8a2a71257370512df

rewrite branch:
  codex/target-architecture-rewrite

rewrite base:
  4bc71b7

architecture and roadmap commit:
  5590eca
```

Verified results:

- the preservation commit contains all 78 selected changed paths;
- ignored `target/` output is absent;
- `git fsck --no-dangling --no-progress` passed;
- the clean committed baseline passed formatting, workspace check, clippy,
  155 tests, and whitespace validation;
- the rewrite branch contains the architecture and execution documents but no
  preserved context/decision implementation WIP;
- Markdown fence, trailing-whitespace, relative-link, and staged-diff checks
  passed;
- the three M1 input decisions and its binary acceptance gates are recorded.

## Next milestone handoff

M1 is active. It begins with a compile-clean workspace cutover and canonical
`world-core`. Only W1 receives method-level implementation planning before
work begins.
