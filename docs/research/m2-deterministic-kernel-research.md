# M2 Deterministic Kernel Research

## Purpose

This note records the research that selected the concrete M2 architecture. It
is an input to the normative target architecture and the executable M2 plan;
it is not a second specification.

The selection criterion was not similarity to any one game or infrastructure
system. A result was retained only when it strengthened the engine's existing
requirements:

- complete same-moment semantics over virtual time;
- one authoritative mutation waist;
- deterministic replay and permutation invariance;
- narrow subsystem boundaries;
- AI- and authored-policy extensibility without ambient nondeterminism; and
- minimal machinery with a current producer, consumer, invariant, and test.

## Result

M2 is best understood as a deterministic batch-certification kernel:

```text
freeze complete least-due deliveries and one base snapshot
  -> classify retained facts and create opaque evaluable work
  -> collect closed pure decisions
  -> prepare typed transactions and semantic footprints
  -> resolve conflicts and combined invariants totally
  -> seal one canonical authority record
  -> publish head, control, scheduler, history, and receipt once
```

The kernel is not a generic transaction manager, event-sourcing framework,
ECS scheduler, workflow engine, or distributed database. Those systems supply
useful principles, but importing their frameworks would obscure the smaller
domain protocol this engine needs.

## Simulation time and simultaneity

[Parallel DEVS](https://informs-sim.org/wsc94papers/1994_0104.pdf) replaces
insertion-order selection among simultaneous events with bags of simultaneous
input and an explicit confluent transition. This directly supports collecting
every delivery at the least `SimMoment` before resolution.

[Lingua Franca](https://www.lf-lang.org/docs/) and
[Ptolemy II discrete-event semantics](https://ptolemy.berkeley.edu/ptolemyII/ptII8.1/ptII/doc/codeDoc/ptolemy/domains/de/kernel/DEDirector.html)
use superdense time: a logical timestamp plus a microstep. Consequences that
must observe a commit occur at a later microstep without inventing elapsed
physical time. The existing `SimMoment = (SimTime, Microstep)` is therefore
the correct kernel coordinate.

[SimPy](https://simpy.readthedocs.io/en/4.0.2/topical_guides/time_and_scheduling.html)
uses a monotonically increasing event ID to make equal-time processing
reproducible. M2 retains an owner-local scheduler sequence for storage and
drain order, but deliberately excludes it from gameplay conflict resolution:
reproducible FIFO is still insertion-order semantics.

The
[Ptolemy distributed discrete-event discussion](https://ptolemy.berkeley.edu/publications/papers/99/HMAD/html/dde.html)
identifies zero-delay feedback as Zeno behavior that can prevent logical-time
progress. M2 therefore executes causal work iteratively, schedules consequences
at a later microstep, and bounds same-time waves deterministically.

## Game-system pressure

[Caves of Qud action costs](https://wiki.cavesofqud.com/wiki/Action_cost) and
its
[turn, segment, and action model](https://wiki.cavesofqud.com/wiki/Modding%3ATurns%2C_Segments%2C_and_Actions)
show why capability, equipment, status, and environment should compile into a
checked duration instead of becoming kernel-owned speed or energy fields.

[Cogmind's turn-time analysis](https://www.gridsagegames.com/blog/2019/04/turn-time-systems/)
shows the gameplay problems caused by pooled uninterrupted turns.
[Angband's energy loop](https://angband.readthedocs.io/en/latest/hacking/how-it-works.html)
is effective for a player-centered sequential game, but makes actor iteration
observable. Neither model should become M2 authority semantics.

The official
[Diplomacy rules](https://www.hasbro.com/common/instruct/diplomacy.pdf) are a
useful non-computational precedent: simultaneous orders must be adjudicated as
a set because support, standoffs, and dislodgement cannot be reproduced by
applying orders in list order.

Consequently:

- M2 owns checked integer time and scheduling, not actor speed policy;
- later action systems produce `SimDuration` through their own semantics;
- periodic world work is an explicit scheduled trigger, not a mandatory tick;
- zero-duration consequences still advance a microstep; and
- randomized initiative is a named, versioned gameplay policy, never ambient
  scheduler behavior.

## Deterministic evaluation and commit

[Calvin](https://ceres.cs.umd.edu/818/papers/calvin.pdf) separates
deterministic sequencing, scheduling, and storage execution and requires known
read/write sets. M2 adopts the separation:

```text
freeze -> evaluate -> certify -> publish
```

It does not adopt Calvin's serial transaction semantics, deterministic locks,
distributed replication, or worker architecture. Same-moment commands read one
shared base and model simultaneity; scheduler order must not decide domain
conflicts.

[FoundationDB simulation](https://apple.github.io/foundationdb/testing.html)
demonstrates the value of deterministic, single-threaded execution for exact
reproduction and aggressive failure exploration. M2 evaluates serially first
and treats future parallelism as an optimization that must erase worker count
and completion order from every canonical result.

[Bevy ECS system access](https://docs.rs/bevy_ecs/latest/bevy_ecs/system/index.html)
supports explicit read/write declarations for discovering independent work.
M2 adopts concrete semantic footprints for validation, conflict grouping, and
future parallel evaluation. It does not adopt an ECS system graph: Bevy also
documents that unspecified conflicting-system order is nondeterministic, which
is unsuitable for authority semantics.

Compiler architecture supplies a useful implementation discipline.
[MLIR pass infrastructure](https://mlir.llvm.org/docs/PassManagement/) uses
checked stage boundaries and isolation, while
[MLIR interfaces](https://mlir.llvm.org/docs/Interfaces/) avoid teaching every
pass every operation. M2 adopts explicit internal stages and owner-specific
closed transaction variants. It does not add a generic IR framework, dynamic
dialects, pass manager, or public transaction interface.

## Conflict and invariant certification

[Invariant Confluence](https://www.vldb.org/pvldb/vol8/p185-bailis.pdf) makes
application invariants, rather than storage conflicts alone, the criterion for
whether independently produced changes can combine. The
[snapshot-isolation critique](https://www.microsoft.com/en-us/research/publication/a-critique-of-ansi-sql-isolation-levels/)
is the corresponding warning: independent validity against one snapshot does
not imply a valid combined successor.

The M2 resolver therefore:

1. canonicalizes delivery identity and ledger classification;
2. prepares every genuinely new command against the same base;
3. creates concrete read, write, exclusive-resource, and invariant footprints;
4. forms connected conflict components from intersecting footprints;
5. ranks equal-policy contenders with keyed semantic randomness;
6. folds candidates canonically through the runtime-owned checked transition;
7. verifies the combined successor;
8. removes tentative acceptances monotonically if refinement is required; and
9. uses rejection-only as the finite valid fallback.

This function is total, bounded, and permutation-invariant. M2 does not search
for a maximum-cardinality accepted set and does not add SAT solving, generic
conflict graphs, CRDT merging, or a universal transaction trait.

## Idempotency, fencing, and recovery

The
[AWS idempotent API guidance](https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/)
supports caller-provided request identity, returning a retained result for
equivalent intent, and rejecting the same identity with different intent.

[RIFL](https://web.stanford.edu/~ouster/cgi-bin/papers/rifl.pdf) adds two
important requirements: the completion record and operation effect must become
durable atomically, and safe reclamation retains a contiguous
lowest-incomplete frontier. M2 maps these to typed request namespaces,
fingerprints, exact replay, mismatch, command collision, permanent retirement,
and one authority publication containing both effect and result.

[Chubby sequencers](https://research.google.com/archive/chubby-osdi06.pdf)
motivate a monotonically changing grant that fences stale capabilities.
M2's grant is operational evidence only. It is absent from semantic record
identity so receipt-free re-reservation of the same logical operation cannot
change the trajectory.

M2 does not add leases, wall-clock expiry, distributed locks, consensus, WAL,
sagas, or an external outbox. The in-memory aggregate has one linearization
point. M2 proves interruption-cut protocol behavior; M5 supplies actual
process-crash persistence.

Authority history is also not conventional event sourcing. The
[event-sourcing pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
makes the event stream the source of truth and carries substantial projection,
concurrency, and schema-evolution costs. Here, an authority record is immutable
commit and verification evidence published atomically with the materialized
head. Nested domain events never recursively mutate the world.

## Authoritative randomness

[Random123](https://www.thesalmons.org/john/random123/releases/1.11.2pre/docs/)
and the
[JAX functional PRNG design](https://docs.jax.dev/en/latest/jep/263-prng.html)
show why random results should be functions of explicit keys rather than
positions in a mutable stream.

M2 uses the existing pinned
[BLAKE3](https://github.com/BLAKE3-team/BLAKE3) dependency as a private
`Blake3KeyedPrf256V1`:

```text
master = BLAKE3 derive-key(fixed application context, RootSeed)
score  = BLAKE3 keyed-hash(master, canonical SemanticRandomKeyV1)
```

The key includes closed subsystem and purpose tags, semantic causal identity,
conflict resource identity, contender identity, an owner-local draw ordinal,
and key-policy version. Record, attempt, commit, scheduler-sequence, worker,
allocation, and collection-order identities are forbidden key material.

Equal-policy conflict selection uses
[Highest Random Weight](https://www.microsoft.com/en-us/research/wp-content/uploads/2017/02/HRW98.pdf):
each contender receives a stable score for the semantic conflict opportunity,
and the highest score wins. Adding a losing contender does not reroll existing
contenders.

Distinct semantic uses with one key are a typed key-reuse fault. Equal
256-bit scores from distinct keys are a typed score-collision safety fault;
no hidden fallback order is permitted. History retains reconstructible keys,
scores, membership, winner, and algorithm/policy versions.

No generic random API is added. If a later real consumer requires `[0, n)`,
[Lemire's multiply-high rejection mapping](https://arxiv.org/abs/1805.10941)
is the selected fixed-`u64` approach, with bounded attempts and typed
exhaustion. M2's HRW consumer does not need range mapping, so it remains
unimplemented.

## Bounded-progress correction

Research found one contradiction in the initial M2 plan. An oversized due set
cannot be repaired by plain `Resume` when immutable execution semantics and
the unresolved due set remain unchanged; preparation would deterministically
enter the same safety state again.

The corrected policy is:

- ordinary scheduler insertion validates the configured per-moment population
  bound, making oversized moments unreachable through valid ordinary records;
- preflight still detects population or evaluator-candidate excess as an
  integrity/safety backstop and enters `Quarantined` without consuming work;
- same-`SimTime` wave exhaustion enters `Paused` and preserves work;
- an idempotent host `Resume` from that specific pause starts a new explicitly
  recorded bounded wave tranche; and
- quarantine can be inspected, failed, cancelled, or handled by a future
  branch/recovery facility, but plain resume cannot pretend to repair it.

Safety evidence is bounded: configured limit, observed count, due-set
fingerprint, causal coordinate, and a capped diagnostic sample. It never
duplicates an unbounded due collection.

## Formal properties selected for M2

For canonicalized command collection `C`, immutable base `S_r`, and immutable
execution semantics `Gamma`:

```text
resolve_Gamma(S_r, C) is total
resolve_Gamma(S_r, permutation(C)) = resolve_Gamma(S_r, C)
apply(S_r, accepted(resolve(...))) satisfies every hard invariant
every new logical command occurs in exactly one outcome
```

For a typed retained ledger `L = (retired_through, retained)`:

```text
every retained sequence is greater than retired_through
every sequence at or below retired_through is permanently non-reusable
retirement advances only across a contiguous terminal or explicitly closed prefix
effect, retained result, and retirement delta publish atomically
```

For reservation and publication:

```text
Active(cursor_r) -> Reserved(exact operation, grant, cursor_r)
publish has one linearization point
  -> head cursor_(r+1) + matching direct-successor receipt
reconcile accepts only:
  cursor_r without a receipt, or
  the one receipt-proven direct cursor_(r+1)
```

Worker count and completion order are absent from `Gamma`, every canonical
preimage, conflict ranking, the accepted set, record bytes, and the successor.

## Verification program

M2 starts without another testing dependency:

- pure resolver and canonical-vector tests;
- exhaustive small-domain reference models;
- trigger, proposal, representation, and worker-count permutations;
- metamorphic tests for unrelated random draws and disjoint commands;
- exact, mismatch, collision, retired-prefix, and retirement-gap partitions;
- combined resource and invariant conflict shapes;
- interruption cuts before/after reservation, publication, receipt, reconcile,
  cancellation, and finalization; and
- a differential attempt-state-machine harness.

This follows the model-based testing principle described by
[QuickCheck's stateful work](https://doi.org/10.1145/636517.636527) while
keeping the first finite domain small and auditable. A property-testing
dependency is justified only if handwritten enumeration becomes the limiting
factor. Loom, Jepsen, TLA+, a worker pool, and distributed-system harnesses are
not M2 dependencies.

## Explicit non-transplants

M2 adds none of the following:

- FIFO or actor iteration as gameplay conflict policy;
- same-moment sequential reads;
- recursive event callbacks;
- mutable global RNG or library-owned distribution semantics;
- wall-clock simulation budgets or retention TTLs;
- generic ECS, transaction, repository, workflow, event-bus, or pass traits;
- optimistic/distributed simulation, locks, consensus, or a worker pool;
- maximum-independent-set or solver-based conflict optimization; or
- placeholder lifecycle, actor-context, storage, or AI types.
