# Runtime, Persistence, and Scaling Architecture

## Purpose

This document defines authoritative execution, virtual time, scheduling,
atomic commit, run-attempt gating and recovery, durable history, checkpoints
and archives, deterministic replay, branching, randomness, multi-resolution,
and the order in which scaling techniques should be added.

## Runtime thesis

The runtime is a deterministic, checkpoint-centered discrete-event state
machine:

```text
external admission | due SimMoment | management disposition
    -> typed proposal or command
    -> authoritative validation and preparation
    -> total deterministic resolution
    -> invariant verification
    -> one atomic AuthorityRecord publication
         accepted state
         runtime control
         scheduler
         authority-history head
    -> optional later causal work
```

It is not:

- a mandatory global tick over every actor;
- an event-sourced database rebuilt from domain events since genesis;
- an actor system whose message arrival order defines causality;
- a distributed simulation kernel;
- a callback registry with shared mutable world access.

The only authoritative transition surfaces are `Admit`, `Fire`, and `Manage`.
They publish ingress, moment, and management records respectively through the
same private atomic publication capability.

## Virtual time

### Time representation

Simulation time is independent of wall-clock time:

```text
SimTick
  quantized integer time chosen by ExecutionConfigArtifact

Microstep
  causal index within one SimTick

SimMoment
  (SimTick, Microstep)

SimDuration
  checked integer duration
```

Floating-point values are not used as scheduler keys. The time quantum is fixed
by the session's immutable `ExecutionConfigArtifact`. Authoring normalizes
source durations to a canonical, configuration-independent exact unit.
Session creation validates that each used duration is exactly representable by
the selected quantum and materializes the integer `SimDuration`; the initial
contract fails closed rather than rounding.

Microsteps order consequences that consume no modeled time:

```text
(t, 0) action commit
(t, 1) resulting observation delivery
(t, 2) appraisal wake
(t, 3) accepted interruption and activity wake
```

Rules:

- no work may be scheduled in the past;
- zero-duration consequences advance to a later microstep;
- handlers may prepare work but cannot recursively commit;
- every moment has a configured work and microstep budget;
- exceeding the budget produces a causal-loop diagnostic and a controlled
  session outcome.

### Fixed-step subsystems

A subsystem such as physics may use a fixed step internally. It participates as
a scheduled component:

```text
PhysicsStep trigger
  -> advance one bounded physics island step
  -> produce a prepared result
  -> commit
  -> schedule another step only while active
```

Its local step does not force cognitive, social, economic, or background
systems onto the same frequency.

## Scheduler

The scheduler is authoritative, serializable scheduler state alongside the
separate runtime-control partition.

The kernel also maintains a monotonic `AdmissionFrontier`: the earliest
`SimMoment` at which a newly admitted external input may become effective.
External input is accepted only at serialized admission barriers. `Fire` holds
the exclusive admission/publication barrier while resolving one due moment,
then seals that moment by advancing the frontier in the same atomic
publication. No host input races into the moment while the barrier is held. If
`advance`/`drain_until` moves the frontier across an interval without firing a
moment, that seal is itself an explicit `ManagementBatchRecord`.

```text
ScheduledTrigger
  trigger id
  target actor, process, component, or session
  SimMoment
  engine-owned lane
  trigger kind
  typed payload
  format-independent semantic causal key
  engine-private provenance record reference
  cancellation generation
  canonical sequence
```

The ordering key is:

```text
(SimTick, Microstep, engine lane, canonical sequence)
```

Engine lanes are a small, named, versioned set used for causal protocol, not a
broad user-assigned priority mechanism. The final sequence is a mechanical
tie-breaker; it must not decide domain conflicts that require an explicit rule.

All lanes at one `SimMoment` share the same start snapshot. A later lane does
not observe a change produced by an earlier lane at that same moment. Any
causal consequence that must observe the result is scheduled at a later
microstep. Lane is therefore a canonical classification inside one batch, not
a hidden isolation level.

Canonical sequence numbers are assigned only at the serialized kernel commit
boundary. The kernel sorts proposed scheduling operations by engine lane,
target, trigger kind, semantic causal key, canonical payload digest, and
producer-local ordinal before assigning sequences. Worker completion and
concurrent insertion order never participate.

The semantic causal key used here is a format-independent cause ID, not an
`AuthorityRecordId`, `AttemptRecordId`, `CommitRecordId`, record sequence, or
hash derived from persistence encoding. Persistence-derived identities may
populate the separate provenance reference, but cannot become scheduler
ordering keys, conflict keys,
randomness keys, entity/process identity, wake generations, or any other
logical tie-breaker.
When that provenance points into the creating publication, its canonical
preimage uses `CurrentRecord` or `(inner kind, canonical local index)` under
the normal nonrecursive reference rule.

The scheduler supports:

- schedule;
- invalidate or cancel;
- reschedule;
- batch-drain one moment;
- deterministic checkpoint and restore;
- metrics and compaction for lazily canceled work;
- diagnostics for stale generations, past scheduling, and same-time loops.

Common trigger families include:

```text
ExternalInputReady
ProcessWake
ProcessDeadline
EvidenceDelivery
AppraisalNeeded
IntentReviewNeeded
ActivityInitNeeded
ActivityAdvanceNeeded
ActorReadyForAction
ResolutionReview
Maintenance
```

Closures, function pointers, and host object references are not scheduled.

## Same-moment work and conflicts

One resolution of the due work at a `SimMoment` is one atomic
`MomentBatchRecord` and one authoritative revision transition. Revisions may
also advance through explicitly typed ingress or management records; those do
not pretend to be simulation work at the current or effective moment.

When multiple actors or processes wake at one due moment while the exclusive
admission/publication barrier is held:

1. drain the due trigger batch;
2. build immutable inputs against a declared snapshot;
3. evaluate and prepare eligible pure work, potentially in parallel;
4. canonicalize returned proposals;
5. complete declared read, write, resource, and invariant footprints;
6. group prepared requests that contend for the same resources or invariants;
7. apply explicit domain conflict policy;
8. use a pure, terminating, permutation-invariant, total resolver to choose a
   candidate accepted set;
9. verify and, where required, deterministically refine the combined set;
10. atomically move revision `R` to `R+1`, consume the due triggers, append one
    `MomentBatchRecord`, and schedule later consequences.

Incidental insertion order must not answer questions such as:

- which actor obtains the last item;
- whether two attacks are simultaneous;
- which reservation wins;
- whether a transaction observes another same-moment transaction.

The responsible domain declares the conflict semantics. The scheduler only
provides a total execution order after that policy has resolved the modeled
meaning.

Every request in the batch is evaluated against the same base revision `R`.
There is no sequential illusion in which the second same-moment request reads
the first request's result. If the modeled semantics require that visibility,
the second operation belongs at a later microstep.

If a candidate accepted set violates a combined invariant, the total resolver
deterministically refines it. Rejecting every proposed acceptance with one
durable outcome per attempt is the mandatory safe fallback, since the base head
already satisfies its safety invariants. If this reveals a configuration or
engine fault, declared failure policy may set `Quarantined` or `Failed` in that
same moment record. It does not retry the same impossible batch forever.

## Runtime requests

The runtime receives closed, typed request families:

```text
InvokeAction
StartProcess
ControlProcess
SubmitEpistemicProposal
SubmitSocialProposal
SubmitAgencyProposal
SubmitLifecycleWakeProposal
TransitionResolution
```

Internal process wakes use serialized scheduler payloads, not host requests.
External inputs use the atomic ingress protocol defined below.
`StartProcess` and `ControlProcess` are trusted lowered/runtime-internal
families; an actor or activity controller reaches them through a grounded
action in the initial architecture.

The trusted coordinator/lowerer binds each authoritative request to the
relevant subset of:

```text
command id
source/controller
actor, where relevant
action opportunity or sponsoring activity, where relevant
requested SimMoment
input world revision
ReadWitness
runtime-definition-set digest
decision or external-input provenance
typed request payload
```

These are engine-private authority/freshness fields, not policy input. Each
request family has a concrete type; this list does not mandate one universal
request-envelope struct or meaningless optional fields.

Every request family defines a canonical `RequestFingerprintK` over its whole
authority/effect-bearing envelope, excluding only the request ID,
retry/arrival metadata, and replaceable authentication-proof bytes. The
authenticated principal and scope, effective/target moment, expected
revision/witness, semantic bindings, actor/opportunity/cause, disposition, and
typed body are included wherever present. Idempotency never compares payload
bytes alone.

The command states what is requested. It does not assert that the action is
valid or that its predicted effects occurred.

After authentication, framing, basic decoding, and size checks, due envelopes
are canonically grouped and reserved by idempotency identity. Each resulting
logical command or ID-collision group then crosses the admitted-command
boundary and receives one `AttemptRecord`, including domain-invalid requests.
Exact duplicate envelopes are retry copies of that one logical command, not
additional admitted commands. Arbitrary malformed transport bytes remain
operational/security diagnostics and cannot churn the authoritative revision.

Runtime control contains a checkpointed command ledger:

```text
CommandLedger
  non-reusable namespace and compact retired-through frontier
  retained (source identity, command id)
    -> Exact(command request fingerprint, original AttemptRecordId and outcome)
     | Collision(sorted distinct request fingerprints,
                 original AttemptRecordId and collision outcome)
```

An exact retained duplicate returns the original outcome without staging
another effect. Reusing a retained exact ID with another request fingerprint
fails closed. Request IDs are monotonically issued inside an authenticated,
`EpochLineageId`-scoped, non-reusable namespace. Physical `RunAttemptId` and
the not-yet-known `TrajectoryId` are excluded. After the protocol's complete
retry/acknowledgement horizon, a contiguous prefix of terminal IDs may compact
to a permanent `retired_through` frontier; a later request at or below that
frontier returns `DuplicateExpired` and never becomes new work. Reordered or
unclosed sequence gaps and entries reachable from a retained checkpoint or
history root cannot be discarded. The same namespace/frontier protocol applies
to the input and host-management ledgers.

The compact frontier changes a later retry outcome and is therefore
authoritative runtime-control state. It advances only in a
`MomentBatchRecord` from a manifest-bounded deterministic maintenance trigger,
or in an explicit captured `ManagementBatchRecord`, using durable
acknowledgement or gap-closure evidence. Wall-clock garbage collection cannot
advance it. Physical entry deletion follows the committed ledger delta.

Deduplication lookup occurs after authentication and request fingerprinting
but before state-dependent freshness, frontier, or legality validation. Thus
an exact retry returns its original outcome even if the current revision or
admission frontier has moved; only a genuinely new ID enters normal validation.
For commands collected in one due batch, the kernel first groups
`(source, CommandId)` canonically and consults the base ledger:

| Base ledger state | Result before parallel preparation |
|---|---|
| Retired | All members return `DuplicateExpired` |
| Retained `Exact(f, outcome)` | Fingerprint `f` returns `outcome`; other fingerprints fail `IdReuseMismatch` |
| Retained `Collision(_, outcome)` | All members return the stored collision outcome |
| Absent with one distinct fingerprint | One logical command and one `AttemptRecord` |
| Absent with multiple fingerprints | One durable `IdCollision` attempt and collision tombstone; no request is selected |

The first three rows create no request-specific `AttemptRecord`, effect, or
ledger mutation. The enclosing `Fire` still publishes its `MomentBatchRecord`
to consume due triggers, seal the moment, and record deterministic control
consequences. In particular, a mixed retry/mismatch group cannot replace an
existing exact entry with a collision. Retirement eventually changes either
retained state only to `DuplicateExpired`.

When a ledger delta refers to an attempt created in the same outer
`AuthorityRecord`, its canonical preimage uses
`(AttemptRecord, canonical local index)`; the derived `AttemptRecordId` is
materialized only after the outer record is hashed.

The same nonrecursive rule applies to all same-publication references. A
post-commit dispatch, input-ledger outcome, or management-ledger outcome that
refers to its enclosing record uses the canonical `CurrentRecord` token in the
preimage; the actual outer ID is materialized after hashing and rechecked on
load.

`ReadWitness` is the dependency evidence used for narrow freshness checks:

```text
ReadWitness
  provenance revision
  runtime-definition-set and execution-semantics digests
  policy-input section
    ActorInputFingerprint and candidate-set fingerprints
    entity/component and query-index stamps read while building visible input
  execution-validation section
    authoritative entity/resource versions used for lowering and legality
    explicit private applicability dependencies
```

An unrelated session revision need not invalidate a result when the witness
proves its inputs unchanged. If the policy-input section no longer validates,
trusted projection rebuilds the complete policy payload. A byte-identical
`ActorInputFingerprint` permits private rebinding or result reuse without a
new logical evaluator invocation; only a changed payload, an explicit
child-branch/epoch evaluator-binding change, or another actor-visible
configured cause permits reinvocation.

The execution-validation section is never policy input. A change there causes
the coordinator and runtime to re-resolve the selected safe ID and revalidate
authoritative legality; it cannot alter policy invocation/dispatch behavior.
Runtime legality is reevaluated authoritatively even when both witness sections
remain valid.

## Prepare and atomic commit

### Validation sequence

For an action or process request, runtime performs:

1. definition and protocol resolution;
2. command-schema and role-binding validation;
3. source authority and permission validation;
4. current-state requirement and resource validation;
5. staleness and reservation checks;
6. trusted primitive evaluation and side-effect-free staging;
7. domain-event contract and per-transaction invariant validation;
8. footprint completion and canonical prepared-transaction construction;
9. explicit same-moment conflict resolution plus combined-invariant
   verification/refinement;
10. atomic commit.

Context and decision validation improve explanations and avoid useless
requests. They never replace runtime validation.

### Prepared transaction

Effect evaluation constructs a side-effect-free internal value:

```text
PreparedTransaction
  base revision
  ReadWitness
  transaction kind
  participating gate receipts
  read footprint
  write footprint
  resource/conflict footprint
  invariant footprint
  state delta
  process and reservation delta
  lifecycle-control delta
  scheduler delta
  domain events
  command outcome
  random draw evidence
  invariant evidence
```

It is not a public pack or evaluator interface. Observers, adapters, and
external services do not run while it is being prepared or committed.

### Atomic same-moment commit batch

The kernel combines all requests evaluated from one base snapshot into:

```text
SameMomentCommitBatch
  SimMoment
  base and resulting revision
  resulting AdmissionFrontier
  consumed trigger ids
  resolver and policy receipts
  AttemptRecord[]
  accepted PreparedTransaction[]
  combined accepted-state delta
  optional session-mode delta
  combined runtime-control and scheduler delta
  optional self-contained ReactionEnvelope
```

Every submitted runtime attempt receives a durable record, including attempts
that have crossed the admitted-command boundary and are stale, contended,
illegal, rejected before preparation, or accepted:

```text
AttemptRecord
  attempt id
  input identity =
      ExactCommandRequestFingerprint
    | IdCollisionGroup(sorted distinct request fingerprints and canonical digest)
  source
  actor and action opportunity, for an exact command
  base revision and ReadWitness digest
  contender/conflict group, where relevant
  resolver and policy version
  canonical ordering evidence
  accepted or rejected outcome
  stable rejection class
  resulting CommitRecord reference, if accepted
```

An accepted subtransaction also produces:

```text
CommitRecord
  command or accepted-proposal reference
  transaction kind and gate receipts
  exact committed delta
  domain events
  random evidence
  execution-semantics fingerprints
```

`SameMomentCommitBatch` is an internal resolved aggregate. Its authoritative
encoding is one outer record; attempts and commits have only one canonical
location in that record:

```text
AuthorityRecord
  common sequence, identity, previous hash, and cumulative hash
  body:
    IngressBatchRecord
    MomentBatchRecord {
      SimMoment, resulting AdmissionFrontier, and consumed trigger IDs
      canonically indexed AttemptRecord[]
      canonically indexed CommitRecord[]
      exact accepted-state/mode/runtime-control/scheduler delta
      optional self-contained ReactionEnvelope
    }
    ManagementBatchRecord
```

Inner IDs derive from the outer record identity, kind, and canonical local
index. The outer record-ID preimage omits those derived IDs, so attempts do not
point around a cyclic record graph.

The batch atomically:

- consumes the due scheduler triggers;
- applies all verified accepted-state deltas;
- applies process, reservation, resolution, lifecycle-control, and scheduler
  changes;
- records every accepted and rejected attempt;
- appends one `MomentBatchRecord` with accepted commit subrecords;
- enqueues a serialized `PostCommitDispatch` if and only if the resulting
  `ReactionEnvelope` is nonempty;
- advances the authoritative revision;
- makes the new immutable snapshot visible.

If any part fails, none becomes visible. A batch with no successful domain
change may still advance control/history state when it records rejections and a
bounded retry, wait, pause, or failure transition.

A `DomainEvent` is a semantic fact for model consumers. A committed delta is
the exact versioned recovery representation. They are linked but not forced to
be the same schema forever.

## Post-commit causal waves

Commit handlers do not recursively run perception or cognition.

For a batch with downstream semantic consequences, the original publication
atomically enqueues a serialized `PostCommitDispatch` at a later microstep. A
crash after durability therefore cannot lose required reaction work.

```text
ReactionEnvelope
  observable domain and semantic events
  ChangedDependencyKey[]
  explicit reliable integration events
```

The envelope is immutable and self-contained for execution. It may retain the
source authority-record ID for provenance, but restoration does not need to
load compactable history to route it. In the publishing record's canonical
preimage, that source reference is `CurrentRecord`; the actual ID is
materialized after hashing.

When that trigger is consumed:

1. the engine-owned `PostCommitRouter` reads the envelope;
2. observation projection emits actor-addressed `EvidenceDelivery` proposals;
3. context dependency analysis emits typed lifecycle-wake proposals;
4. deterministic coalescing proposes generation-aware cancel/reschedule
   operations;
5. runtime validates and commits those evidence/control/scheduler changes in
   another atomic same-moment batch.

Runtime never calls context code and never directly guesses cognitive
dependencies. `ChangedDependencyKey` vocabulary lives below runtime and
context; the engine joins the two through `PostCommitRouter`.

All work occurs at later microsteps and is bounded. Every trigger retains the
commit or input that caused it as engine-private provenance. Actor-facing
causes expose only projection-safe semantic identities. Wait, retry, fallback,
appraisal, and activity-action wakes are scheduler proposals and become
authoritative only through a committed runtime batch. Router invalidation may
use hidden dependency keys internally, but actor-facing wake presence, timing,
generation, and cause must satisfy knowledge noninterference. A hidden-only
invalidation may mark a cache or private envelope stale, but cannot schedule
another logical policy invocation when rebuilt policy-visible input is
unchanged.

Consuming a dispatch creates another dispatch only if that consumption batch
produces a new nonempty reaction envelope. An empty routing result terminates;
it cannot create an infinite no-op dispatch wave.

The rich accepted/rejected attempt resolution is returned only to the trusted
coordinator protocol and retained in its `AttemptRecord`. For every submitted
opportunity with the same live actor-visible sponsor state, the resolving batch
atomically schedules exactly one profile-timed neutral
`AttemptResolved(ActionOpportunityId)` wake regardless of outcome. The
resolution is not a domain event unless the domain explicitly models an
observable attempt. Actor-facing retry or successor work depends only on
profile-fixed budgets and actor-relative input, not the private rejection
class.

Reliable external adapters consume durable history through persistent cursors
or a transactional outbox. Delivery is at-least-once and consumers use stable
IDs for idempotency. An unacknowledged cursor pins every record it still needs;
an outbox entry instead carries a self-contained delivery payload and remains a
durable retention root until acknowledgement. The cursor/outbox and
acknowledgement live in a separately durable adapter-delivery plane, not in
authoritative `Σ`: an outbox payload is deterministically materialized from a
committed self-contained reaction/routing record, and its acknowledgement
cannot change simulation state or ordering. Until materialization is durable,
the source record remains pinned and recovery may rebuild the identical outbox
entry; only then may the outbox become the independent retention root.

The physical protocol is an opaque runtime-owned `HistoryRetentionLease`.
Runtime verifies the committed source and pins it before the delivery service
can materialize an entry. One retention compare-and-set installs the verified
self-contained `DurableDeliveryRoot` before releasing the source pin. A crash
before that compare-and-set leaves the source lease; a crash after it leaves
the independent root. Runtime history compaction refuses ranges covered by
either state. Adapter cursor advancement, dispatch, and acknowledgement remain
owned by the engine delivery service; it receives no raw pin-release
capability. Lossy wall-clock telemetry uses a separate path.

## Durable state and record planes

### Materialized authoritative state

Normal queries read materialized current state. The engine does not reconstruct
the world by replaying semantic events before each session.

### Deterministic run history

The durable run history contains:

```text
AuthorityRecord
  IngressBatchRecord
    CapturedInputRecord[]
    ingress-control delta
    delivery scheduler delta

  MomentBatchRecord
    SimMoment, resulting AdmissionFrontier, and consumed triggers
    AttemptRecord[]
    accepted CommitRecord[]
    exact accepted-state/mode/control/scheduler delta
    optional self-contained ReactionEnvelope

  ManagementBatchRecord
    session pause/resume/quarantine/failure, invocation
      cancellation/timeout/failure, or admission sealing
    captured idempotent host request or deterministic kernel safety cause
    exact mode/frontier/control/scheduler delta
    preserved unresolved-work frontier
```

Only the outer authority record receives a monotonic `RunRecordSeq`, stable
content identity, branch identity, previous-record hash, and cumulative hash.
Inner identities derive from its canonical local order. Captured
nondeterministic inputs are replay data, not automatically world facts. They
are referenced by later attempts or commits.

`EpochLineageId` and the complete `AuthorityCursor` tuple are defined by the
[formal model](formal-model.md#authoritative-session-state). Runtime encoding
uses those exact fields; a revision or sequence alone is never a cursor, and
physical `RunAttemptId` is never semantic lineage.

The hash chain is non-recursive:

```text
record_preimage_body =
  canonical authority-record body with:
    outer identity/hash fields and serialized derived inner IDs omitted
    references to the enclosing record encoded as CurrentRecord
    references to same-record inner values encoded by kind and local index

record_id =
  H(EpochLineageId
    || RunRecordSeq
    || previous record hash
    || record_preimage_body)

cumulative_hash =
  H(previous cumulative hash || record_id)

inner_id =
  H(record_id || inner record kind || canonical local index)
```

Serialized outer and derived inner IDs are rechecked on load. Enclosing-record
references use `CurrentRecord`; all same-record inner references use
`(inner kind, canonical local index)`. Canonical encoding and hash policy are
versioned in the persistence-format manifest. Attempt-step IDs and
`StepPublicationReceipt`s are separate control provenance and are normalized
out of this preimage.

Policy abstention, incomplete context, candidate scoring, and explanatory
fallback reasoning remain decision-trace material. Once a fallback submits a
runtime command or schedules outcome-affecting work, the resulting attempt or
control transition is durable history.

### Atomic external ingress

Every host or external evaluator result enters through:

```text
CapturedInputRecord
  InputId
  origin and idempotency key
  payload and payload hash
  canonical input request fingerprint
  interface and implementation compatibility
  explicit effective SimMoment
  causal invocation, where relevant
```

The checkpointed `InputLedger` contains the namespace's compact non-reuse
frontier plus retained `(origin, InputId)` request fingerprints and original
ingress outcomes.

At a serialized admission barrier, the kernel atomically publishes an
`IngressBatchRecord` containing the input and its delivery trigger. Exact
retained `(origin, InputId, request fingerprint)` duplicates resolve to the
original outcome; reusing a retained ID with another request fingerprint fails
closed. Delivery remains safe after crash and retry. Network completion time
never becomes simulation
time implicitly. This deduplication short-circuit precedes the
current-admission-frontier check; a retry of a once-valid input does not become
a new backdated rejection merely because the world advanced. An ID whose full
outcome has been compacted receives `DuplicateExpired` with no publication.

Ingress uses the same atomic authority-publication machinery and advances the
authoritative control revision, but the payload is not semantically visible to
the simulation until its delivery trigger is consumed at the recorded
effective moment. Capture may update pending-invocation control state and
release a `FrontierBlocking` barrier; policies still cannot consume the result
before delivery. An effective moment behind the monotonic
`AdmissionFrontier` is rejected rather than backdated.

External evaluator invocation has accepted runtime-control state:

```text
DispatchPending
  admission policy
  blocked_at_frontier, exactly when FrontierBlocking
  -> ResultCaptured
     -> Fresh | Stale
     -> Applied | Reinvoked | Fallback | Discarded
  -> TimedOut | Cancelled | Failed
     -> FallbackPending | Discarded
```

The exact request, durable dispatch obligation, response, and every
outcome-affecting transition survive checkpoints. `Requested` is only the
pre-commit proposal; accepted invocation state begins at `DispatchPending`.
External I/O begins only after that obligation is authoritative. Adapter send
attempts are at-least-once and operational: `Dispatched` is not a second
authoritative simulation state. Duplicate requests and results use stable
idempotency keys. Two deterministic admission policies are initially
supported:

- `FrontierBlocking`: the creating `Fire` record stores
  `blocked_at_frontier` equal to that record's resulting
  `AdmissionFrontier`; no later transition may set a frontier greater than
  that value until `Admit` atomically captures a result or `Manage` records
  cancellation, timeout, failure, or explicit disposal;
- `HostScheduled`: the host supplies an explicit effective simulation moment
  as captured input; other simulation may advance, and normal freshness rules
  apply when delivery occurs.

The primary reproducible mode is session-wide `FrontierBlocking`. Actor-local
frontiers are deferred because the initial architecture has one global virtual
clock. A wall-clock timeout, cancellation, or host failure is an
authenticated, idempotent management request, never a captured ordinary input
or implicit simulated deadline.

Evaluator implementations declare one execution class:

```text
InlineDeterministic
  runs as bounded pure work and is recomputed during verification

DeferredCaptured
  proposes an invocation; exact results enter through ingress and are replayed
```

Native versus isolated execution does not decide this class. Any
nondeterministic implementation uses `DeferredCaptured` even when it runs in
the host process.

`Manage` is a scheduler-independent atomic path. It validates, deduplicates,
captures, and applies an authorized host management request in one
`ManagementBatchRecord`, so a blocked or paused session can be resumed,
quarantined, failed, or admission-sealed, and a pending invocation can be
cancelled, without pretending ordinary simulation time advanced. A
deterministic kernel safety cause may directly produce the same record kind.
Unresolved scheduled work remains intact unless
the recorded disposition explicitly removes it. A cancellation, timeout, or
failure disposition may schedule a typed lifecycle fallback trigger at the
preserved frontier; the later fallback is ordinary `Fire` work, not the
mechanism that releases the barrier.

Host management requests have their own typed request ID and checkpointed
deduplication ledger and non-reuse frontier. After authentication, its
deduplication short-circuit also precedes current-state validation. An exact
retained retry returns the original management outcome; a retired retry returns
`DuplicateExpired`; reusing a retained ID with another request fingerprint
fails closed.

Admission sealing names an exact target strictly greater than the current
frontier. Validation rejects a target that skips scheduled work due before it
or crosses an unresolved `FrontierBlocking` invocation, unless the same
`ManagementBatchRecord` resolves or explicitly disposes of that blocker before
setting the target. Other management operations preserve the admission
frontier unless their typed contract explicitly validates a change.

### Decision and lifecycle trace

The decision trace records context construction, lifecycle invocations,
candidate scoring, proposal support, fallback, rejection, and links to commits.
It is explanatory and research-facing, not recovery authority.

### Performance telemetry

Wall-clock latency, allocations, queue depth, cache behavior, token usage, and
other operational measurements are disposable telemetry. They never influence
simulation semantics.

## World checkpoints

A `WorldCheckpoint` is state-complete but depends on an exact immutable
artifact closure:

```text
WorldCheckpoint
  checkpoint format and schema versions
  ExecutionSpecId, which content-addresses the canonical ExecutionSpec body
  InitialStateRootId referenced by that ExecutionSpec
  execution-semantics and persistence-format manifests
  ArtifactClosureManifest reference
  canonical EpochLineageBody and EpochLineageId
  session mode, SimMoment, AdmissionFrontier, and world revision
  complete accepted state partitions
  process, reservation, resolution, and typed lifecycle-control state
  input, management-request, and command deduplication ledgers, including
    non-reuse namespace frontiers
  pending external evaluator invocations and captured-result state
  complete scheduler state, including self-contained pending reaction envelopes
  root seed and branch RNG namespace
  randomness algorithm and key-policy versions
  history cursor:
    last RunRecordSeq (0 at an epoch root)
    last record hash or distinguished epoch-record-anchor hash
    cumulative run-history or epoch-cumulative-anchor hash
  checkpoint_state_fingerprint
```

`checkpoint_state_fingerprint` is the hash of the canonical checkpoint body
with that field omitted. Its preimage includes every authoritative state,
control, scheduler, deduplication, lineage, cursor, and
semantic/persistence-format identity listed above. The trusted encoder derives
that projection from one immutable head. Installation recomputes the same
projection from the expected head and requires both fingerprints to match;
load recomputes it from decoded fields. A body cannot claim a valid cursor
while carrying state from another head.

Recovery loads the checkpoint and applies committed recovery deltas after its
cursor. It does not invoke policies, effects, language models, external
services, or domain-event handlers.

The root-relative, acyclic `ArtifactClosureManifest` defined by the
[extensibility and research architecture](extensibility-and-research.md#artifact-closure-and-retention)
names every content-addressed dependency required for restore or verification:
the checkpoint, archive, or run root that references it is excluded. It
includes the immutable `ResolvedExecutionClosureManifest` plus the dynamic
captured inputs/results, history, and control trace required by that frozen
root. The resolved closure contains the canonical `ExecutionSpec`, its
execution-spec-independent `InitialStateRoot`, runtime packs, lifecycle
profiles and state schemas, required semantic-interface descriptors and their
matching semantic implementations, execution configuration, external modules,
and the exact engine build or reproducible build recipe. A portable
`SessionArchive` packages the checkpoint, retained history, control evidence,
and its exact root-relative closure.
When a run attempt is retained, it also packages the complete durable
`RunAttemptControl`: permanent authority-domain/attempt/specification/root/
epoch binding, canonical creation descriptor and fingerprint, control trace
head and required log segment, attempt-control deduplication ledger and
non-reuse frontier, phase, artifact-retention state and its owner-scoped pin
ledger entries, final disposition evidence, and any matching
`StepPublicationReceipt`.

Archive creation uses one attempt/archive snapshot fence. It reads the
immutable world head and attempt-control record under a stable control-store
generation and accepts only:

```text
Active(cursor)
  checkpoint AuthorityCursor = cursor

StepReserved(expected)
  checkpoint AuthorityCursor = expected and no publication receipt exists
  | checkpoint AuthorityCursor is the exact direct successor of expected
      and one matching atomic StepPublicationReceipt is packaged

Finalized(terminal)
  checkpoint AuthorityCursor = terminal
```

The same fence validates artifact retention. `AttemptOwned(M0, None)` must
package the source closure and all live recovery roots.
`RetainedBy(R, M1, ...)` must package `R`, `M1`, and their complete
reachability. An in-flight `HandoffIntent` is reconciled before snapshot or
causes archive creation to retry; it is never serialized with a dangling
provisional target. `Discarded` cannot produce a restorable/verification
`SessionArchive`, because that state deliberately withdrew the required
artifacts; its permanent control tombstone remains separately inspectable.

Archive identity is canonical rather than an informal list:

```text
AttemptControlPlaneSnapshotDigest =
  H(canonical(
    RunAttemptControl,
    exact AttemptControlEventLog segment through control_trace_head,
    referenced AttemptArtifactPinLedger entries,
    required StepPublicationReceipt identities,
    required AttemptDisposition evidence identities
  ))

ArchiveFingerprint =
  H(canonical SessionArchive body with ArchiveFingerprint omitted)
```

The `SessionArchive` body includes the archive-format identity, checkpoint
fingerprint, exact root-relative `SessionArchive`
`ArtifactClosureManifest` digest, optional
`AttemptControlPlaneSnapshotDigest`, matching receipt if present, and the
digest of the `ReliableDeliverySnapshot` or explicit absence with its format
identity. The closure supplies the referenced log, pin, receipt, evidence, and
history bytes. A concurrent change causes snapshot retry. Any other
control/head pair, binding mismatch, or generation change fails archive
creation; it is never normalized by guessing.

`restore_attempt` within the original persistence domain atomically opens the
one existing control record by `RunAttemptId` and reconciles any reserved
world step or prepared retention handoff; it never clones the record. A
portable import of an `Active` or `StepReserved` archive is observation-only
and issues no mutation capability. Continuing from that snapshot requires
materializing an explicit child root/branch and creating a new
`RunAttemptId`. A finalized archive is freely cloneable only as a read-only
inspection or verification snapshot. Its terminal cursor and reason cannot
change, but the original owner may still compact control ledgers,
unreferenced receipts/evidence, and outcomes covered by the permanent non-reuse
frontier, and may still own pending delivery obligations. Evidence referenced
by `RunFinalization` or a retained control trace remains pinned unless an
explicit terminal discard removes that supported root. Preserving the same
writable control plane across storage domains would require a future
exclusive, fenced transfer protocol that revokes the source; it is not part
of the initial architecture.

If reliable adapter delivery remains pending, it also packages a separately
fingerprinted `ReliableDeliverySnapshot` containing the necessary persistent
cursors, pinned records, and/or self-contained outbox entries; archive creation
fails if that plane cannot be frozen consistently. Portable import verifies
their epoch, checkpoint/history cursor, record references, and delivery-plane
binding, then stores them only in an inert archive/evidence namespace; it
never enqueues or activates them. Same-domain restoration reopens the existing
delivery owner. A portable copy is observation-only for delivery as well:
resuming obligations in another storage domain requires a future exclusive
fenced transfer, so cloning an archive cannot create two active outbox owners.
A checkpoint without the reachable closure is not independently restorable.

The attempt/archive snapshot fence and delivery snapshot form one composite
generation protocol. Runtime's opaque `DeliveryArchiveFence` covers the
session cursor, history leases, and durable delivery roots; the engine delivery
service freezes its cursor/acknowledgement generation against that fence.
Archive installation succeeds only if runtime revalidates the unchanged fence
and both canonical plane digests. Any concurrent acknowledgement,
materialization, compaction, or head/control change forces a retry.

The storage backend is deferred, but its abstract publication contract is
fixed:

```text
append_and_publish(
  expected authoritative head,
  sealed_record: AuthorityRecord,
  sealed_reservation: StepReservation
) ->
  Committed(
    resulting head =
      apply_authority_record(expected head, sealed_record),
    StepPublicationReceipt
  )
  | HeadConflict
  | ReservationMismatch
  | InvalidRecord
```

`apply_authority_record` deterministically applies the record's exact deltas.
Publication validates the previous history link (the last authority record or
distinguished epoch anchor), sequence, derived identities, revision increment,
frontier, cumulative hash, and resulting history head; a caller cannot pair a
valid record with an unrelated resulting state.

The runtime verifies the reservation binding, expected cursor, transition
kind, and `ReservedOperationFingerprint` against the exact operation that
produced the sealed record. The same linearization point stores a
`StepPublicationReceipt` with expected/resulting cursors and record identity.
The receipt is excluded from the authority-record hash and every semantic
identity, so a different physical `RunAttemptId` cannot perturb a reproduced
trajectory.

It has one crash-safe linearization point. Recovery sees the complete old head
or complete uniquely derived new head, never state from one revision with
scheduler or history from another. A checkpoint is encoded from one immutable
head and installed only if its revision, complete history cursor—last sequence,
last record or epoch-anchor hash, and cumulative hash—and canonical
checkpoint-state fingerprint still match that head.

After a checkpoint is durably installed, earlier history may be compacted
according to retention policy, provided no retained branch or audit artifact
depends on it. Artifact garbage collection likewise preserves every blob
reachable from a live attempt or retained checkpoint, run, branch, or report.

Pending scheduler work is self-contained or is itself a retention root. In
particular, a pending post-commit dispatch carries its `ReactionEnvelope`, so
compacting its source batch cannot make restoration unable to route it.
Unacknowledged reliable-adapter obligations follow the same rule: either their
persistent cursor pins the required history, or a self-contained transactional
outbox entry survives independently. Compaction is forbidden until every such
obligation is acknowledged or durably preserved by one of those mechanisms.

Changing a session's `RuntimeDefinitionSet` is not hot reload. It creates a new
session epoch through an explicit offline checkpoint migration with a new
compatibility and artifact-closure manifest. The migrator first emits an
execution-spec-independent `InitialStateRoot`, then the child
`ExecutionSpec`, then a root checkpoint that references both.

## Three replay meanings

The term replay is qualified everywhere.

### Restoration

```text
checkpoint + committed tail -> materialized session state
```

Recorded results are applied. Decisions and effect code are not rerun.
Restoring an active attempt also reconciles any durable `StepReserved` state
against the restored authority cursor before another mutation capability is
issued.

### Verification

```text
checkpoint + scheduler frontier + captured ordinary inputs
  + management/admission-sealing trace
  + AttemptControlTraceArtifact.ReplayInputs
  -> regenerate deterministic lifecycle decisions and runtime attempts
  -> compare AttemptControlTraceArtifact.ExpectedObservations,
     nested AttemptRecords, every AuthorityRecord fingerprint,
     and RunFinalization
```

This is a regression and audit mode. A mismatch is reported at the earliest
divergent durable record. Captured external evaluator results are delivered
from history and never requested again. Decision traces may diagnose a
divergence but are not required for verification. The pure termination rule is
also regenerated at each serialized barrier and the finalization cursor/reason
must match. The expected `RunFinalization` is comparison output, never an input
substituted for the attempt-control trace.

### Counterfactual branch

```text
materialize retained cursor
  -> create immutable branch lineage
  -> change LifecycleProfiles for an allowed stateless policy
     or a declared state-compatible ExecutionConfigArtifact section
     or migrate/reset the child root for a representation-affecting change
  -> produce a new history
```

The parent is not rewritten. Undo is normally a branch or a compensating
command, not destructive history mutation.

A branch cannot directly reinterpret persistent policy-owned state:

- replacing a stateless `ActionPolicy` may be allowed by new child
  `LifecycleProfiles` and normalized execution-semantics identity, but only at
  a quiescent boundary for that port;
- pending work remains pinned to the old implementation and protocol; the host
  must resolve it before branching or explicitly cancel/discard it through a
  child-root reset before invoking the new implementation;
- replacing an activity controller, intent policy, belief policy, or
  another port with persistent state requires declared state compatibility,
  an explicit state migration, or an explicit reset from a pre-policy scenario
  root;
- changing runtime definitions or trusted primitive semantics requires an
  offline checkpoint migration that creates a new child epoch and root
  checkpoint.
- changing an execution-configuration section directly is allowed only when
  its typed compatibility predicate accepts the materialized root and all
  pending work; otherwise it requires an explicit child-root migration or
  reset. In particular, a time quantum cannot reinterpret existing
  `SimMoment`s or compiled durations, and scheduler, admission, conflict, or
  resolution changes cannot reinterpret pending control state.

Every migration or reset is part of branch provenance. Parent history remains
immutable. A branch that continues from a finalized attempt creates a child
root and new attempt; it never reactivates the parent's control record.

## Run-attempt finalization

`WorldSession` mode describes runtime health and administrative control.
`RunAttemptControl` separately decides which authoritative prefix belongs to
one physical execution attempt. The durable record is keyed by
`RunAttemptId` and permanently binds the exact
`AttemptAuthorityDomainId`, `ExecutionSpecId`, `InitialStateRootId`, and
`EpochLineageId` in every phase:

```text
Active(reconciled AuthorityCursor, attempt-control deduplication state)
  -> StepReserved(
       expected cursor,
       transition kind,
       canonical ReservedOperationDescriptor,
       ReservedOperationFingerprint,
       AttemptStepId,
       optional durable non-cancellation AttemptDisposition
     )
  -> Active(new cursor)
   | Finalized {
       RunAttemptId,
       terminal AuthorityCursor,
       canonical reason,
       TerminationClauseId or AttemptDisposition digest,
       TrajectoryId
     }
```

`start_attempt` is an atomic create-or-open by `RunAttemptId`. An exact
creation retry returns the existing bound session/control pair; a different
binding, creation descriptor, or fingerprint under that ID fails closed. Load
recomputes the attempt ID and fingerprint from the canonical descriptor,
which retains the complete binding, raw runner-assigned key, root cursor,
`ResolvedExecutionClosureManifest` digest, and control format. The store-owned
domain is part of the derivation:
`RunAttemptId = H(domain || ExecutionSpecId || attempt key)`. Fresh
independent writable domains have distinct IDs. The immutable referenced
closure excludes run-produced state, trace, and result artifacts and is
materialized and pinned before or atomically with creating the control record
in `AttemptOwned(M0, None)`. Initial creation binds the root, constructs its
`TerminationView`, and installs the initial world/control pair with either
`Active(root cursor)` or root-level `Finalized`; it never exposes an unchecked
active capability.

The attempt gate is the only host surface that can invoke session `Admit`,
`Fire`, or `Manage`. It reserves one exact head, permits at most one authority
publication, which atomically writes the matching `StepPublicationReceipt`.
It then constructs the declared, projection-safe `TerminationView` from the
resulting immutable head and evaluates the pure versioned
`TerminationContract` plus manifest-fixed `RunFinalizationPolicy` before
releasing another step. Termination code cannot read authority hashes, revisions,
deduplication state, raw scheduler entries, storage metadata, or private
lifecycle state except through an explicitly declared semantic projection.
Ordered contract clauses resolve simultaneous semantic conditions; explicit
durable cancellation, host-budget, external-failure, and engine-failure
dispositions map to distinct final reasons.

A non-cancellation disposition arising during a reserved operation is attached
durably to that reservation. Reconciliation applies it only at the
receipt-validated cursor—expected if no publication occurred, otherwise the
exact direct successor—so a failure report cannot choose a later prefix.

The reservation is durable. Recovery has three cases:

1. if the world cursor is still the expected cursor and no receipt exists, no
   publication occurred; without a disposition it may release the reservation
   to `Active` so the caller can resubmit through the normal idempotent surface,
   while a retained disposition finalizes at the expected cursor under the
   fixed policy;
2. if the world cursor is the exact direct successor and the receipt matches
   the attempt binding, step ID, operation fingerprint, record, and both
   cursors, recovery recomputes only `TerminationView` and the pure finalization
   decision and completes the same `Active` or `Finalized` compare-and-set;
3. every other observation is a storage/control integrity failure.

The third case remains `StepReserved`, reports corruption, and grants no
mutation capability; it does not invent a terminal cursor. Recovery never
reruns domain effects, policies, or external services, and another world step
cannot pass the reservation. Recovery never reconstructs a typed request from
an operation hash or autonomously reruns an unpublished operation. External
service dispatch is separately durable and never hidden inside the
publication.

Finalization may intentionally select a prefix whose session remains `Running`
or has pending future work; this is why it is not a session mode. New
attempt-scoped ingress, management, or firing then fails `AttemptFinalized`.
Read-only inspection, checkpoint/archive, reliable-adapter delivery, and
lookup of an already handled idempotent retry remain legal. Continuing
simulation requires an explicit child root/branch and new `RunAttemptId`.

An authenticated, idempotent attempt-cancellation request finalizes the current
reconciled cursor entirely in the run-control plane. Its request ledger uses
the same exact-retry/mismatch/retired-frontier protocol as singular world
request ledgers.
One control-store compare-and-set atomically stores the typed cancellation
disposition, finalization, and deduplication outcome. Exact retries return that
outcome after finalization; mismatched reuse and retired IDs fail closed.

```text
AttemptControlDedupState
  namespace =
    H(RunAttemptId
      || EpochLineageId
      || authenticated origin
      || AttemptControl request family)
  retired_through
  retained[request sequence] ->
      Exact(AttemptControlRequestFingerprint, typed original outcome)
```

Cancellation, retention handoff, and terminal discard use distinct singular
request families in this ledger. Their fingerprints cover the complete
`AttemptBinding`, authenticated principal/scope, typed body, and any target or
evidence, excluding only the request ID, arrival/retry metadata, and
replaceable proof bytes. The base ledger is consulted before current phase
validation. Retirement advances only across a durably acknowledged or
explicitly closed contiguous prefix; it is a control-record transition, never
wall-clock garbage collection.

If different same-ID cancellations race, the first linearized request creates
the exact entry and the other returns `IdReuseMismatch`; cancellation has no
batch-grouping phase or collision tombstone.

Cancellation races by compare-and-set. If cancellation linearizes first, a
step cannot reserve. If `StepReserved` linearizes first, cancellation returns a
transient retry-after-reconciliation response without consuming its request
ID, then retries against the reconciled cursor. Cancellation does not call
`Manage` or change session mode; a host that also wants the retained session
paused, quarantined, or failed must request that separate world transition
before cancelling the attempt.

### Crash-safe attempt artifact retention

`AttemptArtifactRetention` is orthogonal to the
active/reserved/finalized phase:

```text
AttemptOwned(M0, no handoff)
  -> AttemptOwned(M0, HandoffIntent(R, M1, request, fingerprint, transfer))
  -> RetainedBy(R, M1, request, fingerprint, transfer)

AttemptOwned | RetainedBy
  -> Discarded(request, fingerprint, former owned pin identities)
```

`M0` is the immutable `ResolvedExecutionClosureManifest`; `R` is the
finalized `RunArtifactSet`; `M1` is its frozen root-relative
`ArtifactClosureManifest`. Active or reserved attempts require the first state
with no handoff. Only finalized attempts may prepare a handoff or discard.

Handoff first persists the intent while source pins remain, then idempotently
pins `R`/`M1` under the transfer ID in the durable
`AttemptArtifactPinLedger`, compare-and-sets to `RetainedBy`, and only then
marks and releases source pins. Recovery resumes or aborts a prepared intent
from the retention state plus those owner-scoped pin records. Crashes may leak
an extra pin until reconciliation but can never create a zero-pin interval.

Terminal discard resolves any prepared handoff, installs `Discarded` and its
permanent descriptor/fingerprint non-reuse tombstone, and only then releases
the former attempt-owned or retained-run-owned pins. Independent roots keep
their own references. Exact handoff/discard retries return their stored
outcome while their ledger entry is retained; retired request IDs return
`DuplicateExpired`, and mismatched reuse fails closed. An exact
`start_attempt` retry after discard returns `AttemptArtifactsDiscarded`, never
a new session. These transitions and control-ledger compaction may mutate the
original control-plane owner after finalization, but cannot change the world,
terminal cursor, reason, or trajectory.

The append-only `AttemptControlEventLog` stores one composite, canonically
ordered `ControlTransitionEvent` atomically with each accepted construction,
step reservation, disposition attachment, step reconciliation, or
cancellation-finalization transition. The event carries its optional replay
input, all derived observations from that same transition, and the resulting
control-state digest. That digest omits `control_trace_head`; the event hash is
computed next and installed as the new head, avoiding self-reference.
Cancellation-finalization and reconciliation-finalization therefore never
depend on an implicit order among separate events. A host step input captures
the exact `Admit`/`Manage` reference or `advance`/`drain_until` request before
deriving a reservation. Every accepted exogenous control input records a
logical root/before-step/reserved-step/after-reconciliation injection anchor.
The hash-chained segments and referenced receipts/evidence remain pinned until
the canonical `AttemptControlTraceArtifact` takes ownership. Rejected or
unauthorized requests and post-finalization retention housekeeping are
optional operational audit, not deterministic replay input. Verification
drives only those captured intents and anchored exogenous inputs;
reservations, due-work selectors, step IDs, receipts, reconciliation, semantic
termination selection, and finalization are regenerated comparison targets.

## Determinism contract

The engine promises identical logical results under one declared execution
semantics. It does not claim universal bit-for-bit identity across arbitrary
compilers, hardware, native extensions, or numerical libraries.

Compatibility is typed rather than one equality test:

```text
ExecutionSemanticsManifest
  must match for verification execution

PersistenceFormatManifest
  must be decodable or explicitly migrated for restoration

AnalysisFormatManifest
  trace/report/export formats; may be converted without changing trajectory
```

`ExecutionSemanticsManifest` references the exact:

- `EngineProtocolVersion`;
- `RuntimeDefinitionSet` digest;
- `LifecycleProfiles`, including each enabled port's implementation
  requirement and persistent-state schema;
- `ExecutionConfigArtifact`, whose typed sections carry RNG/key and any
  branch-affecting numerical or platform requirements;
- bindings from
  `requires(RuntimeDefinitionSet, LifecycleProfiles, ExecutionConfigArtifact)`
  to matching behavior-affecting implementations, which form the
  `SemanticImplementationSet`.

This manifest is the normalized semantic root. Checkpoints, authority records,
branches, and run artifacts reference its digest. They may carry verified
denormalized fields for indexing, but cannot construct competing semantic
identities independently. Exact whole-build and infrastructure identities are
retained as run provenance unless they are declared trajectory-affecting.

`ExecutionConfigArtifact` is the canonical sealed manifest for configurable
state-affecting policy. It is composed of owner-typed sections rather than a
string-keyed universal bag:

```text
KernelTimeConfig
SchedulerBudgetConfig
ConflictPolicyBindings
LifecycleControlConfig
RunFinalizationConfig
ExternalAdmissionConfig
ResolutionConfig
RandomnessPolicy
SemanticExtensionConfigBindings
```

Hard protocol meaning, including what scheduler lanes and microsteps mean,
belongs to the engine protocol. Configuration selects declared policies and
budgets; it cannot redefine causal semantics through data.

One `Engine` may resolve many execution configurations. Within one
`ResolvedExecution`, session, checkpoint, branch epoch, and execution
specification, however, the exact artifact digest agrees. Trace schema and
report schema changes do not make an otherwise compatible world trajectory
different.

All state-affecting iteration uses canonical order. Authoritative behavior may
not depend on:

- hash-map iteration;
- filesystem order;
- thread completion order;
- pointer or allocation address;
- wall-clock time;
- random UUID generation;
- unspecified floating-point equality.

## Randomness

The baseline uses keyed or counter-based randomness rather than one mutable
global draw stream:

```text
RandomKey
  run seed
  namespace
  actor, entity, process, or execution-owned semantic stream identity
  semantic causal identity
  semantic draw label
  ordinal
  key-policy version
```

Properties:

- unrelated new draws do not shift existing results;
- pure work may execute in parallel without changing outcomes;
- draw purpose is traceable;
- a branch can explicitly preserve or change selected streams.

An exogenous stream identity is resolved into the `InitialStateRoot` or the
trajectory-affecting external-input binding referenced by `ExecutionSpec`.
Planning-level `ScenarioArtifact` identity is never a `RandomKey` input, so a
descriptive scenario edit cannot silently change a trajectory.
Persistence-format record, attempt, and commit identities are likewise never
random-key material; `semantic causal identity` is constructed independently
under the execution semantics.

Experiments distinguish:

```text
exogenous randomness
  scenario and environment streams shared by paired conditions

endogenous randomness
  streams whose identity follows branch-specific decisions
```

Random keys and results needed for verification are referenced by commit or
decision records.

## Multi-resolution simulation

### Initial resolution model

Resolution changes computation, not identity or truth authority.

Every relevant entity retains a canonical core:

```text
identity
accepted invariant-bearing state
ownership and obligations
coarse location
conserved resources
active intent, activity, and process references
important deadlines and wakeups
```

Resolution is selected per explicit scope:

```text
ResolutionScopeId
  (entity or phenomenon identity, subsystem kind)
```

An actor may therefore have detailed movement, background cognition, and
dormant economic production when those are separate declared scopes. Exactly
one tier-specific authoritative representation is active per scope:

```text
Detailed
  full context and action interaction

Background
  individual identity retained, activity/process summarized, sparse wakes

Dormant
  no recurring evaluation, only external or deadline activation
```

Detailed and coarse implementations may not update the same scope's
authoritative operational fields concurrently. Each component declares which
fields are canonical core and which are exclusively owned by its active tier.

### Promotion and demotion

Every transition is a checked runtime transaction:

```text
ResolutionTransition
  source and target tier
  cause
  conversion implementation and version
  preserved invariants
  canceled and replacement triggers
  approximation evidence
  resulting tier state
```

A resolution-capable component eventually implements:

```text
demote(detailed, moment) -> background
advance_background(background, until) -> background + proposed outcomes
promote(background, moment, RandomOracle) -> detailed
validate_cross_resolution_invariants
```

The component owns the internal representation. The shared contract owns
identity, time, invariants, causal ordering, and explicit loss of detail.

Required declared invariants may include:

- entity identity and membership;
- conserved currency or resources;
- health and inventory bounds;
- obligation and relationship continuity;
- process deadlines and interruptibility;
- spatial reachability;
- causal ordering;
- deterministic structural and conservation checks.

Runtime transition checks distinguish:

```text
HardTransitionInvariant
  identity, conservation, bounds, deadlines, ownership, causal order
  checked on every transition

FidelityMetric
  approximation error against detailed or empirical baselines
  calibrated and evaluated offline
```

The runtime does not claim to prove a statistical error bound when no detailed
counterfactual exists.

Promotion never claims that discarded microstate was preserved. Any
reconstruction uses an explicit, versioned policy and records approximation
evidence.

### Scope order

The first scaling implementation keeps individual actors and summarizes their
background activities and processes. It does not aggregate populations.

Population aggregation is deferred until a domain supplies:

- a membership ledger;
- conservation rules;
- aggregation and disaggregation maps;
- cross-tier interaction semantics;
- an error metric;
- a policy for recovering an individually important member.

Resolution policy uses hysteresis, minimum residency, and manifest-fixed
simulation budgets to prevent promotion/demotion thrashing. Wall-clock load,
latency, and telemetry cannot choose a tier in reproducible modes. A host may
change policy only through an explicit captured controller input permitted by
the execution configuration.

## Scaling order

Scaling work proceeds in this order:

1. **Event skipping** — do not wake actors without meaningful work.
2. **Indexes and incremental invalidation** — rebuild only affected context.
3. **Background individual simulation** — sparse activity/process wakes.
4. **Parallel independent runs** — scenarios, seeds, profiles, and branches.
5. **Parallel pure evaluation** — snapshot-isolated context and policies,
   followed by canonical collection.
6. **Profile-guided optimization** — storage, query, and commit internals.
7. **Only with evidence: intra-world parallel or distributed simulation.**

The final authoritative conflict resolution and commit lane remains
deterministic. Distributed rollback, speculative event execution, and
cross-node causal protocols are not first-kernel requirements.

If a same-tick work or microstep budget is exhausted, the kernel commits a
deterministic session-control transition such as `Paused`, `Quarantined`, or
`Failed`. The record preserves the remaining trigger frontier, repeating causal
evidence, and configured disposition. Resumption requires an authenticated,
idempotent host management request and cannot silently discard due work.

Progress guarantees are conditional. Under weak host fairness, terminating
bounded inline work, and an eventually admitted result or recorded management
disposition for every session-blocking external invocation, plus eventual
reconciliation of any durable `StepReserved` attempt, a running session with
due work under an active attempt eventually publishes another revision, enters
an explicit non-running management state, or durably finalizes that attempt at
a serialized barrier. The runtime does not promise that a domain eventually
accepts an actor's action or that an external service responds.

## Runtime failure invariants

The runtime test strategy must prove:

1. state, scheduler, and commit history cannot partially diverge;
2. no event or external notification escapes a rolled-back transaction;
3. stale commands are never accepted without explicit safe revalidation;
4. two requests cannot consume one exclusive resource;
5. canceled process generations cannot wake obsolete work;
6. same-time work cannot depend on thread completion order;
7. zero-time loops terminate with a diagnostic;
8. checkpoint cursors and canonical state fingerprints exactly match the
   included authoritative projection;
9. restoration never invokes nondeterministic computation;
10. definition or protocol incompatibility fails closed;
11. resolution transitions conserve their declared invariants;
12. rendering rate, telemetry, and wall time cannot change simulation outcome;
13. post-commit reaction work survives a crash before dispatch;
14. rejected attempts and conflict decisions remain in durable history;
15. lifecycle-control and pending external-invocation state survive
    checkpoints;
16. causal-wave exhaustion leaves an explicit committed session state;
17. every authoritative publication is one hash-linked ingress, moment, or
    management record with one crash-safe linearization point;
18. duplicate input, management-request, and command IDs cannot produce a
    second effect, including after retained outcomes compact to non-reuse
    frontiers;
19. a nonempty reaction envelope is pending, consumed, or explicitly disposed,
    while an empty envelope cannot schedule itself;
20. compaction cannot make any pending scheduler trigger or unacknowledged
    reliable-delivery obligation unrecoverable;
21. combined-invariant failure deterministically refines the accepted set,
    falling back to a rejection-only moment record with complete attempt
    coverage before any later management transition;
22. no input becomes effective before the sealed admission frontier;
23. the published head equals deterministic application of the authority
    record to its exact expected head;
24. selecting `Paused`, `Quarantined`, or `Failed` never permits an invalid
    accepted-state delta;
25. actor-facing wake payload, presence, timing, generation, and cause cannot
    reveal a private attempt/process outcome or hidden dependency;
26. hidden-only witness or legality changes cannot alter logical policy
    invocation/dispatch presence, timing, or generation; an explicit
    child-branch/epoch evaluator-binding change is a new execution semantics,
    not a hidden-state difference;
27. one `RunAttemptId` finalizes at most once at its deterministic terminal
    cursor, and crash recovery cannot permit another attempt-scoped world
    transition past that cursor.
