# M1/W3: Runtime Authority

## Status

Complete.

W3 begins from the sealed immutable definition set completed in W2. It
implements the M1 authority boundary with its final ownership and dependency
direction before engine composition or trusted transfer semantics are added.

## Goal

Introduce one complete M1 reserved-step and publication waist:

```text
immutable runtime binding
  + private SessionHead
  + repository-owned RunAttemptControl
    -> reserve one exact authority step
    -> build and seal one typed AuthorityRecord
    -> atomically publish head + history + receipt
    -> return to Active at the new cursor or finalize there
```

`Admit`, `Fire`, and `Manage` are the only operations that may change the
authoritative session state `Σ`. Attempt creation, reservation,
reconciliation, explicit failed-Fire disposition, cancellation, and
finalization may change only the attempt-control plane `Ca`.

The organizing abstraction is a **reserved authority step**:

```text
Active(cursor)
  -> Reserved(step bound to cursor)
  -> Reserved(step, published record + receipt + resulting cursor)
  -> Active(resulting cursor) | Finalized(resulting cursor)
```

Publication advances `Σ` but deliberately leaves the repository-owned attempt
gate reserved. Runtime projects termination from the immutable resulting head
and only then compare-and-sets `Ca` to `Active` or `Finalized`. No later
attempt operation may enter between those steps. Every other W3 type exists
to construct, validate, publish, read, finalize, or reconcile that protocol.

## Trust and failure assumption

W3 protects correctness under ordinary concurrency, interrupted or dropped
drivers and prepared steps while the repository remains alive, stale local
handles, duplicate requests, and mismatched repository state. These are part
of the engine's authority and reproducibility model, not a hypothetical
hostile-input boundary. Its in-memory repository models the reservation,
receipt, and reconciliation protocol; surviving process failure requires a
later persistent backend.

Keep:

- private construction of the session head, sealed records, reservations, and
  publication arguments;
- exact cursor, binding, operation-fingerprint, and receipt correspondence;
- one atomic old-or-new repository publication point;
- deterministic record, cursor, and cumulative-history identities;
- typed attempt states and closed step/record families;
- exact request deduplication needed to prevent a second logical command;
- receipt-based interrupted-step reconciliation and single terminal
  finalization.

Do not add:

- authentication, authorization policy, encryption, signatures, process
  isolation, or hostile resource-exhaustion defenses;
- a database, async persistence abstraction, public repository trait, or
  distributed consensus protocol;
- repeated validation of facts carried by sealed private values;
- generalized retry, retention, compaction, or archival frameworks before
  their later consumers exist.

## Non-goals

- `EngineDistribution`, semantic implementation installation, runtime
  activation, or sealed `ResolvedExecution`;
- `world-engine`, `world-standard-runtime`, public `ControllerRequest`, or
  execution of the standard transfer primitive;
- complete same-moment batching, footprints, conflict resolution, post-commit
  reaction routing, or full lifecycle scheduling;
- actor-relative context, cognition, decisions, intent, activity, process, or
  observation;
- the complete verified `TerminationContract` interpreter;
- checkpoints, restoration, replay, branches, archives, delivery leases,
  post-finalization artifact-retention handoff, or a second persistence
  backend;
- the persistent attempt-control event log and trace head, control-trace
  artifact, artifact-pin ledger, and the complete retention algebra;
- same-barrier command collision grouping, permanent deduplication retirement
  frontiers, or full M2 kernel protocols;
- compatibility with the deleted runtime or authority APIs.

## Normative contracts

- [Target Rust Code Architecture](../../architecture/target-architecture/code-architecture.md)
- [Formal System Model](../../architecture/target-architecture/formal-model.md)
- [Target System Architecture](../../architecture/target-architecture/system-architecture.md)
- [Runtime, Persistence, and Scale](../../architecture/target-architecture/runtime-persistence-and-scale.md)
- [Architecture Decisions](../../architecture/target-architecture/decisions.md)
- [Validation Scenarios](../../architecture/target-architecture/validation-scenarios.md)
- [M1 authoritative vertical slice](milestone-01-authoritative-vertical-slice.md)
- [Completed W2 plan and evidence](milestone-01-work-package-02.md)

## W2 input

W2 supplies:

- `RuntimeDefinitionSet` with exact root, engine protocol, required semantic
  interfaces, artifact closure, source lock, and deterministic identity;
- checked actions, runtime requirements, effects, and physical events;
- pack-qualified durable definition keys;
- exact artifact and definition-set vectors;
- no executable semantic implementation, activation index, mutable state, or
  publication authority.

W3 may bind immutable runtime state to the definition-set identity. It may not
reinterpret, rebuild, or bypass the sealed W2 values.

## Package boundaries

The selected production graph adds:

```text
world-model   -> world-core, world-defs
world-runtime -> world-core, world-defs, world-model
```

The complete selected local graph after W3 is:

```text
world-defs      -> world-core
world-model     -> world-core, world-defs
world-runtime   -> world-core, world-defs, world-model
world-authoring -> world-core, world-defs
world-standard  -> world-defs
```

No new registry dependency is expected. The in-memory repository uses the
standard library's synchronization primitives.

| Concern | Owner |
|---|---|
| Immutable accepted values and model-facing deltas | `world-model` |
| Immutable admitted command envelopes and execution outcomes shared with runtime | `world-model` |
| Cloneable, non-authoritative read snapshots | `world-model` |
| Session head, record sealing/application, and scheduler state | `world-runtime` |
| Attempt binding, initial closure ownership, reservation, receipt, disposition, and finalization | `world-runtime` |
| Atomic repository and reconciliation | `world-runtime` |
| `MomentWorkInput`, `MomentWorkProposals`, and `PreparedFire` protocol wrappers | `world-runtime` |
| `Admit`, staged `Fire`, and `Manage` | `world-runtime` |
| Semantic implementation activation and dispatch | deferred W4 |
| Engine facade and controller request | deferred W4 |

`world-model` contains no mutable aggregate root, store, history append,
scheduler mutation, `apply_*`, or publication-capable value.

## Minimum model surface

W3 introduces only immutable model values that have both a runtime consumer
and the already-selected W4 transfer consumer. Because these values enter the
initial-root and authority-record preimages, their complete M1 meaning is
fixed here rather than represented by empty placeholder families, reserved
tags, or a generic property bag.

The minimum public shape is:

```text
ContainerRecord
  container EntityId
  item_capacity u32

ContainmentRecord
  item EntityId
  container EntityId

ContainerAuthorityRecord
  actor ActorId
  container EntityId

AcceptedState
  containers sorted uniquely by container
  containment sorted uniquely by item
  container authority sorted uniquely by (actor, container)

ContainmentTransferDelta
  actor
  item
  expected source
  destination

PhysicalEvent
  ItemTransferred { actor, item, source, destination }

WorldSnapshot
  authoritative WorldRevision
  immutable accepted model state

CommandEnvelope
  exact command source, identity, and fingerprint
  actor and selected action definition
  typed role bindings

CommandAttemptOutcome
  Accepted
  | Rejected(DefinitionUnavailable | BindingMismatch | Stale
             | RequirementUnsatisfied | Conflict)
```

`AdmitOutcome` is separate from `CommandAttemptOutcome`. `Admit` captures and
schedules a `CommandEnvelope`; only a later `Fire` produces the command's
accepted or rejected execution outcome. The admitted representation is not a
second logical command identity.

`ContainerAuthorityRecord` is hard authority to perform the physical transfer;
it is not social or legal ownership. `ContainmentTransferDelta` and
`PhysicalEvent` are inert immutable values and grant no mutation authority.
M1 containment is deliberately flat: container IDs are unique, item IDs are
unique and disjoint from container IDs, every containment and authority record
references a declared container, authority pairs are unique, and the number
of directly contained items never exceeds that container's `item_capacity`.
`world-runtime` privately validates and applies the delta: the expected source
matches, source and destination exist and differ, the actor controls the
source, destination capacity remains valid, and every item remains in at most
one container. W4 supplies the trusted predicate/effect implementation and
runtime-owned lowering from exact activated roles into this checked proposal
path; it does not add another durable transfer variant.

The minimum runtime record schema correspondingly contains:

```text
RecordedCommandResolution
  Accepted { commit: SameRecordCommitRef }
  | Rejected(StableCommandRejection)

CommitRecord
  ContainmentTransfer { delta, derived ItemTransferred event }

ReactionEnvelope
  ordered nonempty PhysicalEvent[]
```

The sealer derives `ItemTransferred` from the accepted
`ContainmentTransferDelta`; W4 cannot supply an independently different event.
The `MomentBatchRecord` freezes its moment/frontier, consumed triggers, typed
attempts, typed commits, optional combined containment delta, control and
scheduler deltas, and optional reaction envelope. Accepted resolution, local
commit reference, delta, derived event, and nonempty reaction envelope must
appear together; rejection contains none of them. Public
`CommandAttemptOutcome` is derived from the recorded resolution rather than
stored as an independently disagreeing field. W3's stable-rejection fixture
uses the rejection form. W4's accepted transfer uses the already frozen
delta, commit, event, and envelope forms. Neither package may add a universal
component map, dynamic value tree, or generic mutation list merely to bridge
the two packages.

A `WorldSnapshot` is safe to clone. Constructing one for a fixture does not
construct an authoritative session or cursor. Runtime derives its
`WorldRevision` from the same locked `SessionHead` as its accepted state; an
aggregate read must satisfy `snapshot.revision == head.cursor.revision`.
`AuthorityCursor` remains runtime-owned and is not embedded in `world-model`.

## Minimum execution binding

Identity-bearing runtime values belong to `world-runtime`, not to W4. W3
therefore freezes and implements the minimum canonical forms required before
it records root, attempt, finalization, or trajectory vectors:

```text
InitialStateRoot
  starting SessionMode, SimMoment, and admission frontier
  accepted model state
  concrete empty V1 runtime-control state
  concrete empty V1 scheduler state
  EpochLineageId and declared parent/reset origin

ExecutionSpec
  InitialStateRootId
  ExecutionSemanticsManifest digest
  root seed
  minimum closed TerminationContract
  external-input binding digest

ResolvedExecutionClosureManifest
  exact root and specification references
  RuntimeDefinitionSet and artifact closure
  execution-semantics/configuration references
  exact semantic/lifecycle implementation binding slots
```

The first closed protocol values are:

| Contract | M1 form |
|---|---|
| `SessionMode` | `Running | Paused | Quarantined | Failed` |
| lifecycle profiles | retired singleton profile with lifecycle evaluation disabled |
| execution configuration | `ExecutionConfigArtifactV1 { finalization_policy: DispositionFirstV1 }` |
| semantic implementation selection | exactly one implementation binding for each required `SemanticInterfaceReference` |
| external input | `ExternalInputBindingV1::HostSerialized` |
| termination | `Never | AtOrAfterMoment { moment, reason: ReachedConfiguredMoment }` |
| root runtime control | `EmptyInitialRuntimeControlV1` |
| root scheduler | `EmptyInitialSchedulerV1` |

`DispositionFirstV1` maps a present durable attempt disposition to its matching
typed final reason; otherwise a satisfied `AtOrAfterMoment` clause finalizes
with `ReachedConfiguredMoment`; otherwise execution continues. The timed
clause has its own derived `TerminationClauseId`. `HostSerialized` is a real
closed tag, not a caller-supplied digest. Later request namespaces derive from
`(EpochLineageId, request family, HostSerialized)` and cannot be chosen by an
input request.

Implementation and lifecycle selections may be empty only when the derived
requirement closure is empty. The transfer-shaped owner-local fixture instead
supplies one exact private deterministic rejection-only implementation binding
for every required interface reference. W4 constructs a separate production
manifest with production bindings in the same format. Slots are never absent,
replaced by an undefined "fixture digest," or added to the same format later.

The origin lineage is derived without a root/specification cycle. Runtime
first canonicalizes an `initial-root-semantic-body-v1` containing mode, time,
frontier, accepted-state digest, and the concrete empty control/scheduler
values but no lineage or identity. `EpochOriginId` commits that body.
`EpochLineageId` then commits either that origin or the exact child lineage
body. `InitialStateRootId` commits the complete lineage and semantic root body
while excluding its own ID and any child `ExecutionSpecId`. `ExecutionSpecId`
is derived only after the specification names that root.

`AcceptedStateDigest` is derived by `world-model` from the complete three
sorted accepted-state vectors under `accepted-state-v1`. `InitialStateRoot`
owns that complete accepted state and writes its recomputed digest into the
semantic body; it never accepts an independently supplied digest.
Root-compatibility checking recomputes the digest from the retained state.

`ExecutionSemanticsManifestV1` contains the engine protocol, definition-set
digest, disabled lifecycle-profile value and identity, complete V1
configuration value and identity, canonically sorted implementation bindings,
and the exact required-interface closure derived from the sealed definition
set. `ExecutionSpecV1` references its manifest digest, root ID, root seed,
closed termination contract, and closed external-input binding. Constructors
receive and retain the owned values from which identities are recomputed; they
do not accept unattached identity bytes as proof. `Disabled` is an
execution-binding value, not authority to erase or consume post-commit work.

`ResolvedExecutionClosureManifestV1` likewise retains the checked root,
specification, and semantics values required to reopen the execution. Its own
identity preimage writes their verified IDs/digest plus the closure-specific
component and artifact references; it does not embed those three canonical
bodies again after writing their identities.

W3 privately checks and binds a canonical root/specification pair and derives
their real identities. Owner-local tests construct that complete minimum
fixture binding. W4 uses the same frozen formats and preimages to construct and
reverify a distinct production manifest, specification, and closure against an
installed distribution, then mints the first externally reachable activation.
The production binding may have different identities because its
implementation selection is different; W3's fixture vectors remain valid
fixture vectors. W4 does not redefine an identity preimage or append fields to
an existing format.

## Controlled-attempt model

The W3 state is the M1 projection of the formal model:

```text
Ωa = (Ca, Σ)

Ca = attempt control for physical attempt a
Σ  = one private authoritative session head
```

This projection contains the live attempt record, retained closure reference,
reservation, disposition, receipt, and finalization needed by one in-memory
authority step. M2 completes the in-memory attempt state machines. The
persistent control-event log and trace head, artifact-pin ledger, verification
trace, retention transitions, and reconstructible replay protocol remain M5
work. Their final owner is still `world-runtime`; W3 does not create a
competing representation or public extension point.

The minimum private head is:

```text
SessionHead
  AuthorityCursor
  SessionMode
  SessionClock { now, admission frontier }
  accepted model state
  minimum typed runtime-control state
  minimum typed scheduler state
```

`SessionHead` has no public constructor, clone-to-authority conversion,
replacement method, or setter. `SessionHead::root` projects one already
validated immutable closure into `Σ` and creates the distinguished root cursor
without publishing an ordinary revision. The closure remains in attempt-owned
`Γ`; W3 sealing borrows it, and canonical apply does not. W4 replaces that
borrow with the private activated runtime execution rather than adding
configuration fields to `SessionHead`.

The cursor uses structurally distinct positions:

```text
AuthorityCursor
  epoch/execution binding
  Root {
    record anchor
    cumulative anchor
  }
  | Record {
    nonzero world revision
    nonzero record sequence
    AuthorityRecordId
    cumulative hash
  }
```

No optional-field combination represents both root and post-record state.

The M1 scheduler is one ordered map rather than several synchronized indexes:

```text
SchedulerKey
  SimMoment
  SchedulerLaneV1
  SchedulerSequence

SchedulerLaneV1
  Command = 0
  | PostCommit = 1

ScheduledWork
  Command(ScheduledCommand)
  | PostCommit(PostCommitDispatch)
```

The lane tags are canonical V1 protocol values. M1 admits exactly one
`ScheduledWork` at an exact moment; sequence remains a deterministic insertion
coordinate and does not become a domain conflict tie-breaker. This is the
deliberate M1 projection of the formal model's one atomic moment batch. M2 may
generalize the value at one moment to an all-due batch without changing the
authority boundary.

Post-commit work is scheduled at the earliest unoccupied representable moment
strictly after the committing moment: increment the microstep when possible,
otherwise advance the simulation time by one tick and reset the microstep to
zero, continuing until a vacant moment is found. Exhausting the representable
moment space is a typed seal failure. `SchedulerKey` alone owns the scheduled
moment; the work value does not duplicate it. A `PostCommitDispatchId` is
derived for the one complete reaction envelope produced by a source batch.
Cursor succession and one batch per source moment make a permanent consumed-ID
tombstone unnecessary. The global sequence orders installations and is not a
second logical dispatch identity.

`PostCommitDispatchId` is a semantic scheduler identity, not persistence
provenance:

```text
PostCommitDispatchId =
  H(canonical "post-commit-dispatch-v1" {
    schema = 1
    EpochLineageId
    source SimMoment
  })
```

The preimage identifies the single reaction envelope emitted by one source
moment in one lineage. It excludes command identity, event ordinal,
`AuthorityRecordId`, inner record IDs, scheduler destination, reaction bytes,
record sequence, revision, cumulative history, and attempt-control identities.
The applied `PostCommitDispatch` retains the dispatch ID, derived
`ReactionEnvelopeId`, and complete envelope as checkpoint-safe work.
`SchedulerKey` is the sole owner of its destination.

W3 does not route post-commit work. If the globally least scheduler entry is a
`PostCommitDispatch`, `prepare_fire` returns the typed
`RuntimeDriveError::PostCommitRoutingRequired { moment }` before creating a
reservation. The blocked moment is sufficient at the public boundary; raw
scheduler keys and dispatch identities remain private runtime evidence. The
entry remains unchanged, and no later command may be selected around it. M2
replaces this deliberate command-only projection with whole-moment,
all-lanes preparation and engine-owned post-commit routing over the durable
runtime dispatch protocol.

## Authority-record model

History has one closed outer algebra:

```text
AuthorityRecord
  common header and predecessor identity
  Ingress(IngressBatchRecord)
  | Moment(MomentBatchRecord)
  | Management(ManagementBatchRecord)
```

The private build sequence is:

```text
DraftAuthorityRecord
  -> canonicalize owner-selected unordered collections
  -> validate the new transition invariants
  -> derive nonrecursive record and nested identities
  -> derive cumulative history and resulting cursor
  -> SealedAuthorityRecord
  -> canonical apply to the expected SessionHead
```

Only the sealed value enters publication. Record application derives the
successor; no caller supplies a resulting head.

The concrete private header retains the complete checked predecessor context:

```text
AuthorityRecordHeader
  EpochLineageId
  resulting NonZeroWorldRevision
  NonZeroRunRecordSeq
  PreviousAuthorityHash
  previous CumulativeAuthorityHash
  derived AuthorityRecordId
  derived resulting CumulativeAuthorityHash
```

`PreviousAuthorityHash` is a private role type populated from either the root
record anchor or the previous record identity. Its canonical field is exactly
the 32 hash bytes with no root/record discriminant. The record-ID preimage is
exactly schema, lineage, sequence, previous-authority bytes, and canonical
body. It excludes execution specification, revision, both cumulative hashes,
the outer ID, materialized inner IDs, cursor, reservation, receipt, and
attempt-control data. The cumulative preimage contains only its schema,
previous cumulative hash, and new record ID.

M1 represents the legal moment shapes directly:

```text
MomentRecordShape
  NewRejected {
    CommandDeliveryRecord
    AttemptRecord(Rejected)
    command-ledger insertion
  }
  | NewAcceptedTransfer {
      CommandDeliveryRecord
      AttemptRecord(Accepted { SameRecordCommitRef(0) })
      ContainmentTransferCommit
      nonempty ReactionEnvelope
      command-ledger insertion
      post-commit dispatch
    }
  | RetainedExact {
      CommandDeliveryRecord
      original AttemptRecordId
      original CommandAttemptOutcome
    }
  | IdReuseMismatch {
      CommandDeliveryRecord
      original AttemptRecordId
    }
```

The canonical writer projects this correlated algebra into the target
attempts, commits, accepted delta, reaction, control delta, and scheduler delta
field order. Those projections are not independently stored `Vec`/`Option`
fields, so an accepted attempt cannot exist without its commit, delta, derived
event, reaction, ledger entry, and dispatch. The four command shapes consume
exactly their captured command trigger. Only `New*` inserts the command
ledger; only `NewAcceptedTransfer` changes accepted state or schedules
post-commit work. No W3 authority record consumes a post-commit dispatch.

W3 freezes byte-complete canonical preimages and golden vectors for:

- the initial root and root cursor;
- each minimum outer record family;
- `AuthorityRecordId`;
- cumulative authority hash;
- `ReservedOperationFingerprint` and `AttemptStepId`;
- `StepPublicationReceipt`;
- attempt creation fingerprint, finalization representation, and
  `TrajectoryId`.

The minimum derivations are fixed by the formal model:

```text
InitialStateRootId =
  H(canonical InitialStateRoot body without its own ID or child
    ExecutionSpecId field)

ExecutionSpecId =
  H(canonical ExecutionSpec body without its own ID field)

RunAttemptId =
  H(AttemptAuthorityDomainId
    || ExecutionSpecId
    || runner-assigned attempt key)

AttemptCreationFingerprint =
  H(canonical AttemptCreationDescriptor)

ReservedOperationFingerprint =
  H(canonical ReservedOperationDescriptor)

AttemptStepId =
  H("attempt-step"
    || RunAttemptId
    || expected AuthorityCursor
    || ReservedOperationFingerprint)

TrajectoryId =
  H(ExecutionSpecId || terminal cumulative authority hash)
```

The root is derived before the child `ExecutionSpec`; binding occurs only
after the spec references that exact `InitialStateRootId` and both canonical
identities have been recomputed.

Self-references use the canonical local-index/current-record rules already
fixed by the target architecture. Rust memory layout, debug output, and
general serialization do not enter identity.

W3 freezes the local-reference tags used by its V1 authority-record writer:

| Local value kind | Tag |
|---|---:|
| captured input | `0` |
| command attempt | `1` |
| accepted commit | `2` |
| reaction envelope | `3` |

Each local reference writes its kind tag and a `u32` canonical local index.
The enclosing-record role has its own one-value V1 protocol whose
`CurrentRecord` token is tag `0`; it is not a generic graph reference or a
sentinel identity. M1's correlated record variants contain zero or one value
of each local kind, but the indexed encoding remains the durable format.

## Attempt authority domain

One `RuntimeService` repository/control domain supplies and permanently owns
one `AttemptAuthorityDomainId`; an attempt request, `ExecutionSpec`, activated
execution, root, or semantic implementation cannot choose it. W3's in-memory
repository mints the ID from a process-global monotonic repository ordinal and
retains it for the repository lifetime. This guarantees distinct writable
domains among all live repositories in the process without adding entropy or
persistence machinery.

W3 deliberately makes no cross-process or crash-survival claim: no in-memory
attempt survives process loss, so reusing an ordinal in another process cannot
collide with live authority. A later persistent backend stores the same
logical field and replaces the minting rule with durable store initialization.
Tests may inject a deterministic owner-local value. The ID is never accepted
in an attempt, activation, or execution request.

The domain ID participates in `RunAttemptId` only. It is excluded from root and
specification identities, authority records and cursors, `TrajectoryId`, RNG
keys, and semantic configuration. Consequently, running the same semantic
execution and inputs in two independent domains produces different
`RunAttemptId`s but identical authority records, cursors, and trajectory
identity.

## Attempt-control algebra

The M1 attempt projection is a closed algebra, not related booleans:

```text
AttemptBinding
  AttemptAuthorityDomainId
  RunAttemptId
  ExecutionSpecId
  InitialStateRootId
  EpochLineageId

RunAttemptControl
  immutable AttemptBinding
  immutable AttemptCreationDescriptor
  exact creation fingerprint
  retained attempt-control request deduplication
  artifact retention:
    AttemptOwned(AttemptOwnedClosure, no handoff)
  phase:
    Active(AuthorityCursor)
    | Reserved(StepReservation)
    | Finalized(RunFinalization)
```

`AttemptCreationDescriptor` contains the complete binding, raw
runner-assigned attempt key, root cursor, exact
`ResolvedExecutionClosureManifest` digest, and control-format version. The
receipt contains the complete binding, `AttemptStepId`,
`ReservedOperationFingerprint`, expected and resulting cursors, and published
`AuthorityRecordId`.

The closure manifest is materialized and retained before, or atomically with,
attempt creation. Artifact retention and attempt phase are orthogonal fields:
finalization changes only the phase and W3 keeps the same
`AttemptOwnedClosure` with no handoff. Later handoff or discard changes only
artifact retention, is legal only after `Finalized`, and remains deferred.
Owner-local fixtures construct the complete minimum canonical
root/specification/closure binding and exercise the runtime-owned termination
projection. W4 activation becomes the first external producer of production
bindings in those frozen formats.

Attempt creation evaluates the runtime-owned root termination rule and
installs either `Active(root)` or root-level `Finalized`. It never exposes an
unchecked active capability.

`StepReservation` binds:

- attempt and immutable execution binding;
- a private repository-local `ReservationGrant`, minted each time control
  enters `Reserved`;
- exact expected cursor;
- `Admit`, `Fire`, or `Manage` step kind;
- the complete canonical family-specific `ReservedOperationDescriptor`;
- its exact `ReservedOperationFingerprint`;
- the deterministically derived `AttemptStepId`;
- an optional retained `AttemptDispositionId`, initially absent.

The descriptor is retained control evidence. Reconciliation never attempts to
reconstruct it from its fingerprint.

`AttemptStepId` identifies the logical operation. `ReservationGrant` identifies
one physical grant of authority to perform it. Reconciliation may release an
unpublished reservation and a later retry may derive the same logical step;
the new grant then makes every `PreparedFire` from the prior reservation
stale. The grant is process-local control state and is excluded from the
reserved-operation fingerprint, step ID, receipt, authority record, cursor,
and trajectory identity.

The M1 Fire descriptor binds the exact least command selected from the
expected head:

```text
Fire {
  exact SchedulerKey
  CommandTriggerId
}
```

The key freezes ordering and the trigger freezes logical work. This descriptor
can be created only after the globally least entry has been proven to be that
command. A globally least post-commit dispatch produces
`PostCommitRoutingRequired` before reservation, so W3 cannot reserve one work
family and publish the other and needs no generic scheduler-token interface.

`StepPublicationReceipt` binds that reservation to the exact outer record,
expected cursor, and resulting cursor. A receipt is stored atomically with
publication and is not world semantics.

The M1 disposition algebra is:

```text
AttemptDisposition
  CancelRequested {
    CancelAttemptRequestId
    CancelAttemptRequestFingerprint
    CancelReason::HostRequested
  }
  | HostBudgetExceeded
  | ExternalFailure
  | EngineFailure

AttemptDispositionId =
  H("world-attempt-disposition-v1"
    || canonical AttemptDisposition)
```

The private `AttemptDispositionStore` retains the exact canonical value under
that identity. A prepared-Fire disposition is attached to the reservation
that already identifies its `AttemptStepId`;
the cancellation disposition is created only by the `Active -> Finalized`
control compare-and-set. `RunFinalization` references the retained
`AttemptDispositionId`. The cancellation request fingerprint covers its
binding and typed reason while omitting the request ID and retry metadata.
The three failure variants are the complete unit-valued V1 evidence. Adding
payload fields requires a V2 disposition schema and a new canonical domain;
the reservation supplies the step binding.

`RunFinalization` installs exactly once at one reconciled terminal cursor.
Finalization revokes further attempt-scoped publication without changing the
already-published session head. Cancellation similarly changes only `Ca`; a
desired pause or failure of the session remains an explicit `Manage`.
Cancellation cannot pass an unresolved reservation and must reconcile it
first.

The minimum authoritative request ledgers live in `Σ.runtime_control`:

```text
InputRequestLedger
  InputId -> (InputRequestFingerprint, AdmitOutcome)

ManagementRequestLedger
  ManagementRequestId -> (ManagementRequestFingerprint, ManageOutcome)

CommandRequestLedger
  (CommandSource, CommandId)
    -> Exact {
         CommandRequestFingerprint
         original AttemptRecordId
         original CommandAttemptOutcome
       }
```

`InputId` and `ManagementRequestId` are distinct `u64` protocol wrappers.
Zero is representable; issuance monotonicity and the later retirement frontier
belong to their namespace owner rather than these scalar constructors.

The singular request fingerprints omit their request ID:

```text
InputRequestFingerprint =
  H("input-request-v1"
    || effective SimMoment
    || CommandSource
    || CommandId
    || CommandRequestFingerprint)

ManagementRequestFingerprint =
  H("management-request-v1"
    || SessionManagement)
```

The checked `CommandEnvelope` retained by an ingress or moment record remains
the source of the command fields. The authority-record identity writes its
source, command ID, and already-checked request fingerprint; `world-model`
does not gain a general serialization trait or a second command identity.

The retained successful outcomes are:

```text
AdmitOutcome::Scheduled {
  record: AuthorityRecordId
  effective: SimMoment
}

ManageOutcome
  Paused { record: AuthorityRecordId }
  | Resumed { record: AuthorityRecordId }
```

Their record preimages encode the record field as `CurrentRecord`. The
internal input-ledger entry additionally retains the derived
`CapturedInputRecordId` and semantic `CommandTriggerId`; these are encoded as
`SameRecordCapturedInputRef(0)` and the trigger value, respectively. The
public outcome remains narrow and returns only record provenance and the
effective moment. `MomentSlotUnavailable`, illegal mode transitions, and
request-ID reuse mismatches are nonpublishing results and do not create
ledger entries.

The host-serialized input namespace and scheduled-command trigger are semantic
identities:

```text
ExternalInputNamespaceId =
  H("input-request-namespace-v1"
    || EpochLineageId
    || ExternalInputBindingDigest)

CommandTriggerId =
  H("command-delivery-trigger-v1"
    || ExternalInputNamespaceId
    || InputId
    || InputRequestFingerprint)
```

The canonical domains distinguish the request family, so callers supply
neither identity. One scheduled command retains its `CommandTriggerId`, the
materialized `CapturedInputRecordId`, and the complete checked
`CommandEnvelope`; its `SchedulerKey` retains the effective moment. The
record-ID preimage writes the captured-input provenance as
`SameRecordCapturedInputRef(0)`, while canonical apply installs the derived
identity. `SchedulerSequence` is independently allocated, checked, and
advanced by `SchedulerState` at the serialized commit boundary. It is never
borrowed from an input or idempotency identity.

A consumed moment has exactly one `MomentRecordShape`, as defined by the
authority-record algebra above. `NewRejected` and `NewAcceptedTransfer` own a
new `CommandDeliveryRecord` and same-record attempt reference; only the
accepted form also owns its commit, reaction, and post-commit dispatch.
`RetainedExact` owns the delivery plus the original attempt and outcome;
`IdReuseMismatch` owns the delivery plus the original attempt. There is no
separate delivery-resolution enum that could disagree with the enclosing
shape.

The new attempt points back to the delivery through the consumed trigger,
prior `CapturedInputRecordId`, and exact command tuple rather than inventing
another same-record inner kind. The command's authority-record commitment is
`(CommandSource, CommandId, CommandRequestFingerprint)`. The complete checked
envelope remains owned for replay and must reproduce that fingerprint when
decoded, but the record identity does not introduce a second command
canonicalizer. The enclosing correlated algebra is the sole
new/retained/mismatch classification.

They are separate concrete ordered maps with no common ledger trait and no
retirement frontier in W3.

For either public singular request family, retained lookup precedes
attempt-phase, cursor-freshness, and current-state checks. An exact retry
therefore returns the original outcome without another record even after
attempt finalization. Reusing a retained identity with another fingerprint
returns a typed mismatch. Only an absent identity may enter the single-step
reservation. The `Active -> Reserved` compare-and-set rechecks both the
expected head and ledger absence; this is the absent-ID linearization point,
and the reservation binds the exact request identity and fingerprint.
Successful publication installs its exact ledger entry and domain outcome in
the same authority record. If no record publishes and reconciliation releases
the reservation, the identity remains absent and may be resubmitted.

The command ledger is consulted inside `prepare_fire` against the immutable
base head before any semantic evaluation:

1. an absent `(source, CommandId)` exposes new command work; successful moment
   publication creates one `AttemptRecord` and installs its exact fingerprint,
   record identity, and outcome;
2. a retained exact fingerprint exposes the original attempt-record identity
   and retained outcome as no-new-work;
3. a retained identity with another fingerprint exposes a stable
   `IdReuseMismatch` plus the original attempt-record reference as
   no-new-work.

Cases 2 and 3 still publish the enclosing `MomentBatchRecord` to consume the
due trigger and record the replay/mismatch resolution, but they mint no second
request-specific `AttemptRecord` and perform no semantic evaluation, model
effect, or command-ledger mutation. W3 needs no same-barrier collision entry
because its scheduler admits at most one work item at an exact moment.
Collision grouping, retirement, and compaction are M2 work. Attempt creation
and cancellation use their separate `Ca` request-deduplication records.

Cancellation consults its `Ca` deduplication record before the attempt phase.
A retained exact request returns its original `CancelAttemptOutcome`, including
after finalization. The ledger retains the general fingerprint classifier, but
V1 exposes only `CancelReason::HostRequested`; every legal cancellation body
for one attempt therefore has the same fingerprint. The mismatch branch is
reserved for a later schema with a genuinely distinct request body and is not
exposed as a fabricated V1 outcome. For an absent request:

1. `Reserved` returns a transient reconcile-and-retry result without consuming
   the request identity;
2. `Active` atomically stores the canonical `CancelRequested` disposition,
   finalization, and exact cancellation-dedup outcome at the current cursor;
3. `Finalized` returns `AttemptFinalized` without consuming the request
   identity.

Cancellation never creates a reservation and never changes `Σ`.

## Atomic repository

The repository is crate-private and concrete. One aggregate in-memory lock is
preferred initially because it makes the required linearization point
obvious.

Conceptually, it owns:

```text
MemoryRepositoryState
  AttemptAuthorityDomainId
  attempts: RunAttemptId -> AttemptAggregate {
    control
    dispositions
    private session head
    authority history
    receipts
  }
```

Its central operation is:

```text
append_and_publish(SealedStepPublication)
  -> atomically:
       verify exact live reservation and expected head
       apply the sealed record to derive the successor head
       append the record
       store the matching receipt
       retain the attempt gate as Reserved with publication evidence
```

Readers observe either the complete pre-publication aggregate or the complete
published aggregate. The published aggregate contains the successor session
state, scheduler/control state, cursor, history, matching receipt, and the
same still-reserved attempt gate. These parts cannot become independently
visible. After publication, the runtime-owned termination projection and
pure finalization rule perform a separate `Ca` compare-and-set to
`Active(resulting cursor)` or `Finalized`.

No public repository SPI is introduced. A future backend remains internal to
`world-runtime` or sits below a runtime-owned byte/CAS validation wrapper.

Minimum reconciliation distinguishes:

1. unchanged head, no receipt, and no disposition: release the reservation to
   `Active(expected)`;
2. unchanged head, no receipt, and an explicit disposition: project the
   expected head and finalize there under the fixed runtime policy;
3. exact direct successor with the exact receipt: rerun only the runtime-owned
   termination projection, then combine that decision with any attached
   disposition under the canonical finalization precedence and install
   `Active(successor)` or `Finalized`;
4. any missing or mismatched receipt, non-successor, second successor, binding
   mismatch, or invalid cursor: retain the reservation, grant no mutation
   capability, and return a typed integrity failure.

Reconciliation never re-executes effects, policies, or external services.

## Runtime capability surface

The M1 public capabilities already have their final authority direction:

```text
RuntimeService                 cloneable service capability
RuntimeAttemptDriver           non-Clone, attempt-bound mutation capability
RuntimeSessionReader           cloneable read-only capability
PreparedFire                   non-Clone, single-use staged-Fire token
```

The W3 read surface is exactly:

```text
RuntimeSessionReader::cursor()
  -> Result<AuthorityCursor, RuntimeReadError>

RuntimeSessionReader::snapshot()
  -> Result<WorldSnapshot, RuntimeReadError>
```

Each method copies its result from one aggregate read. The reader exposes no
history, reservation, receipt, disposition, attempt-control, checkpoint, or
mutation operation.

The driver surface is:

```text
session_reader(&self) -> RuntimeSessionReader

admit(&mut self, AdmitRequest)
  -> Result<AdmitOutcome, RuntimeDriveError>

prepare_fire(&mut self, FireRequest)
  -> Result<PreparedFire, RuntimeDriveError>

complete_fire(&mut self, PreparedFire, MomentWorkProposals)
  -> Result<FireOutcome, RuntimeDriveError>

fail_prepared_fire(&mut self, PreparedFire, PreparedFireFailure)
  -> Result<PreparedFireFailureOutcome, RuntimeControlError>

manage(&mut self, ManageRequest)
  -> Result<ManageOutcome, RuntimeDriveError>

cancel_attempt(&mut self, CancelAttemptRequest)
  -> Result<CancelAttemptOutcome, RuntimeControlError>
```

`PreparedFire` exposes only immutable due-work input. It contains no callback,
repository, mutable snapshot, record builder, or publication method.

The staged W3/W4 seam is deliberately narrow:

```text
PreparedFire::input() -> MomentWorkInput<'_>

MomentWorkInput
  EvaluateCommand {
    due SimMoment
    borrowed base WorldSnapshot
    borrowed CommandEnvelope
  }
  | ResolvedCommand {
      due SimMoment
      borrowed CommandEnvelope
      original AttemptRecordId
      Retained(CommandAttemptOutcome) | IdReuseMismatch
    }

MomentWorkProposals
  opaque private-field value with exact identity/fingerprint coverage for
  that one due command
```

W3 prepares one due command per moment; complete same-moment, all-lanes
batching is M2 work. `PreparedFire` owns the captured snapshot and due
selection, so the input borrows them and cannot outlive the token. Runtime
rejects a missing, duplicate, unknown, or differently bound proposal before
record sealing. A globally least post-commit entry instead returns
`PostCommitRoutingRequired` without a token or reservation.

`RuntimeSessionReader` is not part of staged input and is never passed to an
evaluator. After preparation, proposal construction may depend only on the
token's immutable `EvaluateCommand` input and execution capabilities selected
before evaluation; a later reader call cannot affect that proposal.

W3 can privately construct only the stable-rejection proposal needed to prove
new-command publication. A narrow checked no-new-work constructor accepts a
`ResolvedCommand` input. There is no public generic delta, event, map,
callback, or arbitrary proposal constructor. W4 connects the checked
stable-rejection and accepted-transfer paths to typed activation. The trusted
implementation returns only a permission result; runtime derives the exact
containment delta from activated roles and uses this already frozen proposal
constructor. That addition does not change a durable variant, the
non-cloneable token, immutable input, or runtime-owned sealing/publication
boundary.

W3 constructs the immutable execution/start binding only inside
`world-runtime`. Owner-local tests exercise the complete M1 authority waist.
W4 activation becomes its first external producer. W3 must not expose a
temporary public session-seed or pre-activation bypass solely to make the
intermediate package callable.

## Minimum transition behavior

### `Admit`

- accepts one `AdmitRequest` containing an `InputId`, effective `SimMoment`,
  and `CommandEnvelope`;
- performs exact request-ID/fingerprint lookup before requiring an active
  attempt or checking the current cursor;
- returns a retained exact outcome or typed mismatch without reservation;
- for an absent ID, reserves the exact active cursor with that request
  identity and fingerprint while rechecking ledger absence;
- validates the effective moment and other current-head conditions only after
  that short circuit;
- enforces the M1 scheduler invariant of at most one scheduled item globally
  at an exact `SimMoment`; a moment occupied by either lane, or already sealed,
  returns the stable typed nonpublishing `MomentSlotUnavailable` result and
  leaves the input ID absent;
- captures one typed `CommandEnvelope` and schedules its later delivery;
- seals one `IngressBatchRecord` containing the request-ledger outcome;
- exact retry returns the original outcome without a second publication.

Pause/resume cannot remove pending work, and the admission frontier never moves
backward. Once an exact moment is occupied, it remains occupied until that
work is handled; afterward the moment is sealed. Therefore
`MomentSlotUnavailable` cannot later become an acceptance for the same
effective moment.

### `Manage`

- remains scheduler-independent;
- accepts `ManageRequest { id: ManagementRequestId, operation:
  SessionManagement }`, where W3's closed `SessionManagement` is `Pause |
  Resume`;
- performs the management-ledger exact/mismatch lookup before requiring an
  active attempt, reserving a cursor, or validating the current mode;
- returns a retained exact outcome or typed mismatch without reservation;
- for an absent ID, reserves the exact active cursor and seals one
  `ManagementBatchRecord` containing the ledger outcome and mode delta, while
  rechecking ledger absence at reservation;
- does not smuggle a domain mutation through administrative control.

### `Fire`

- `prepare_fire` first inspects the globally least due work; if it is
  post-commit, it returns `PostCommitRoutingRequired` without reservation,
  consumption, rescheduling, or selection of any later command;
- if that globally least work is a command, `prepare_fire` reserves the exact
  cursor and command key/trigger;
- because W3 admits at most one work item at an exact `SimMoment`, that command
  moment contains exactly one due item and therefore already satisfies the M1
  drain-all-due rule;
- the returned token exposes either one immutable new-command evaluation input
  or one no-evaluation retained/mismatch command resolution;
- `complete_fire` consumes that exact token and a closed proposal value,
  revalidates the reservation and command-ledger classification, and seals one
  `MomentBatchRecord`;
- W3 proves successful staged publication with a legitimate stable command
  rejection before W4 adds accepted transfer semantics; it must not expose a
  generic mutation bag for the test;
- retained exact or mismatched command identities consume their due trigger
  and record their resolution without another semantic evaluation or model
  effect;
- no W3 Fire consumes a post-commit dispatch;
- dropping a prepared token performs no implicit cleanup. The
  repository-retained reservation is released or completed only by
  reconciliation.

`PreparedFireFailure` is closed:

```text
HostBudgetExceeded
ExternalFailure
EngineFailure
```

This category is the complete typed evidence available in M1: it carries no
arbitrary string, source error, byte payload, or extensible diagnostic.
`fail_prepared_fire` consumes the prepared token, maps its failure to the
matching exact canonical `HostBudgetExceeded`, `ExternalFailure`, or
`EngineFailure` disposition, attaches its identity to the reservation, and
reconciles/finalizes `Ca` at the last receipt-validated cursor without changing
the session head or authority cursor. Richer family-specific evidence is added
only with a concrete later producer and a corresponding canonical schema.

W3 implements the pure evaluator for its minimum closed termination contract
and proves root/post-record finalization with checked contract values. W4 adds
load-time reverification and the externally reachable activated binding; it
does not supply a termination Boolean or callback.

## Validation ownership

Each boundary checks only the invariant it introduces:

| Boundary | New proof |
|---|---|
| Root construction | immutable binding and distinguished root consistency |
| Attempt creation | exact binding, creation fingerprint, retained closure, root cursor, and initial Active or Finalized phase |
| Reservation | live attempt, exact cursor, canonical operation descriptor, and its fingerprint |
| Record sealing | closed body legality, canonical identity, and derived successor |
| Publication | reservation/head correspondence and atomic installation of successor head, history, receipt, and an unchanged Reserved gate |
| Reconciliation | disposition-aware unchanged/successor relation and fail-closed mismatch handling |
| Finalization/cancellation | unique legal `Ca` transition at the reconciled cursor |

Private sealed values carry earlier facts forward. Publication does not rerun
source/artifact validation, and readers do not revalidate sealed history.

## Target source layout

Files appear only with concrete implementation:

```text
crates/
  world-model/
    src/
      lib.rs
      accepted.rs
      command.rs
      snapshot.rs

  world-runtime/
    src/
      lib.rs
      service.rs
      execution/
        ids.rs
        config.rs
        semantics.rs
        termination.rs
        external_input.rs
        lineage.rs
        spec.rs
        initial_root.rs
        closure.rs
        binding.rs
      session/
        head.rs
        mode.rs
      authority/
        cursor.rs
        record.rs
        seal.rs
        apply.rs
      attempt/
        control.rs
        reservation.rs
        receipt.rs
        disposition.rs
        finalization.rs
      kernel/
        admit.rs
        fire.rs
        manage.rs
      control/
        ledger.rs
      scheduler.rs
      persistence/
        memory.rs
```

Directories or modules are omitted when the slice does not yet need them.
There is no empty scheduler, transaction, checkpoint, archive, random,
retention, process, or lifecycle framework.

The first implementation slice creates only `world-model`, the execution
modules above, `session/mode.rs`, and `authority/cursor.rs`. Session heads,
records, attempt control, service, kernel fronts, and persistence appear only
after the root/specification/cursor protocol has passing byte vectors.

## Work sequence

### 1. Freeze the minimum authority protocols

- select the exact W3 model records required by the transfer slice;
- freeze the complete M1 `InitialStateRoot`, `ExecutionSpec`, minimum
  semantics/configuration/termination values, and closure-manifest slots;
- freeze root, record, cursor, reservation, receipt, and attempt identity
  preimages, plus the canonical `RunFinalization` representation and
  `TrajectoryId` preimage;
- freeze the authority-domain, reserved-operation, prepared-Fire disposition,
  and cancellation request/disposition contracts;
- freeze the minimum closed record and error algebras;
- record deterministic golden vectors before implementation.

### 2. Add immutable model values

- add `world-model` with exact final dependency edges;
- implement the concrete container, containment, hard-authority, transfer
  delta, transfer-event, closed command-outcome, `CommandEnvelope`, and
  cloneable read-snapshot values used by M1;
- prove model values carry no store or publication authority.

### 3. Implement root and record derivation

- add `world-runtime`;
- first implement canonical execution values, checked
  root/specification/closure binding, and structurally distinct root/record
  cursor positions;
- then implement the private head, the concrete transfer-shaped
  attempt/commit/event/reaction record slots, record draft/seal/apply, and
  cumulative history;
- test exact vectors and nonrecursive identity derivation.

### 4. Implement attempt control and the atomic repository

- implement attempt create/open, initial closure ownership,
  active/reserved/finalized states, reservations, receipts, dispositions,
  separate exact input, host-management, and singular command deduplication,
  and the one aggregate in-memory publication point;
- test competing publication and atomic reader visibility.

### 5. Add opaque runtime capabilities

- put `RuntimeService`, `RuntimeAttemptDriver`, `RuntimeSessionReader`, and
  `PreparedFire` over the private repository;
- keep production root/start construction private until W4 activation;
- add privacy and non-Clone compile-fail tests.

### 6. Implement the three transition fronts

- implement exact command capture in `Admit`;
- enforce one scheduled item across both lanes per exact M1 moment;
- implement minimum pause/resume `Manage` with singular host-request
  deduplication;
- implement staged `prepare_fire`, `complete_fire`, and explicit failed-Fire
  disposition with the one-command evaluation/resolved input algebra and
  without a generic mutation surface.

### 7. Reconcile and finalize

- implement unchanged-head and receipt-proven-success reconciliation;
- implement unique finalization and the serialized `Ca`-only cancellation
  lookup/deduplication/compare-and-set;
- prove dropped prepared tokens, explicit failure, cancellation, and
  finalization have distinct repository-retained effects.

### 8. Close the package

- run the full verification and dependency-allowlist matrix;
- audit public API reachability and internal atomicity;
- record exact completion evidence;
- detail W4 only after every W3 gate passes.

## Acceptance gates

### Cargo graph

```text
selected local packages =
  { world-core, world-defs, world-model, world-runtime,
    world-authoring, world-standard }

world-model local dependencies =
  { world-core, world-defs }

world-runtime local dependencies =
  { world-core, world-defs, world-model }

new registry dependencies = {}
world-engine selected = false
world-standard-runtime selected = false
```

### Authority and atomicity

- only sealed canonical application advances revision, sequence, history, and
  cursor;
- root and record cursor positions cannot be confused;
- competing publications at one cursor produce exactly one committed record;
- readers see either the complete pre-publication aggregate or the complete
  published aggregate with successor state, history, receipt, and the gate
  still `Reserved`;
- stale cursor, wrong reservation, missing/wrong canonical operation
  descriptor, wrong operation fingerprint, or wrong pre-publication binding
  causes zero session mutation;
- reconciliation corruption causes zero additional session mutation, retains
  the `Reserved` phase, and grants no mutation capability even when the head has
  already advanced;
- no external crate can construct or replace the head, sealed record,
  reservation, receipt, or publication argument.

### Attempt control

- exact attempt-creation retry opens the same attempt; another creation
  fingerprint fails;
- creation retains the exact resolved-execution closure and may install either
  root-level `Active` or root-level `Finalized`;
- exact input retry returns the retained outcome without another record; the
  same ID with another fingerprint fails;
- exact management retry returns the retained outcome without another record;
  the same ID with another fingerprint fails;
- retained input and management lookups remain legal after finalization and
  precede freshness or mode validation;
- the same `(CommandSource, CommandId, fingerprint)` admitted through another
  input or moment reuses its retained command outcome and performs no second
  effect; another fingerprint produces `IdReuseMismatch`;
- retained and mismatched command deliveries are not semantically evaluated,
  mint no second request-specific `AttemptRecord`, and retain the original
  attempt-record reference, yet their enclosing Fire still consumes the due
  trigger in one `MomentBatchRecord`;
- two admissions targeting one exact `SimMoment` leave at most one scheduled
  item there; a post-commit item also occupies the entire M1 moment. Any
  conflicting admission receives `MomentSlotUnavailable`, publishes no
  record, and leaves its input ID absent;
- when a post-commit dispatch is globally least, `prepare_fire` returns its
  exact blocked moment as `PostCommitRoutingRequired`, creates no reservation,
  leaves scheduler state unchanged, and does not skip to a later command;
- one attempt has at most one reserved world step at a time;
- `PreparedFire` is non-Clone and single-use;
- a released and re-granted logical step receives a new private reservation
  grant, so a `PreparedFire` from the prior grant cannot complete or fail the
  new reservation even when both reservations derive the same
  `AttemptStepId`;
- a dropped token leaves a repository-retained reservation and reconciliation
  follows the declared cases while that repository remains alive;
- explicit failed Fire and cancellation do not change the session head or
  authority cursor;
- cancellation linearizes only from `Active` and atomically installs its
  disposition, finalization, and `Ca` deduplication outcome;
- exact cancellation retry returns its retained outcome even after
  finalization; V1 adds no artificial second cancellation body merely to make
  fingerprint mismatch reachable;
- reservation-first makes cancellation return a transient retry without
  consuming its request identity; cancellation-first makes reservation return
  `AttemptFinalized`;
- finalization installs once, freezes one terminal cursor, and blocks later
  attempt-scoped publication.

### Determinism and API

- frozen root, record, cursor, reservation, receipt, and finalization vectors
  match;
- two authority domains running the same semantic execution and inputs have
  distinct `RunAttemptId`s but identical authority records, cursors, and
  `TrajectoryId`;
- repeated identical operation sequences produce identical authority records,
  cursors, and cumulative history;
- owner-declared unordered input collections cannot alter a sealed record;
- concurrent publication affects only which already-sealed contender wins the
  compare-and-set, never record contents or partial visibility;
- the read capability exposes no mutation path;
- the read capability exposes only one-current-aggregate `cursor` and
  `snapshot` copies;
- every returned snapshot carries the `WorldRevision` of the same aggregate
  head from which its accepted state was copied;
- W3's staged Fire input contains exactly one due command; only an absent
  command-ledger entry receives the immutable base snapshot for evaluation,
  while retained/mismatch command inputs permit only their checked no-new-work
  proposal;
- every completed Fire has exact identity/fingerprint coverage for that due
  command, with no missing, duplicate, unknown, or differently bound
  proposal;
- `RuntimeSessionReader` is absent from staged evaluation input, and proposal
  construction has no outcome-affecting read path after preparation;
- no public repository trait, generic state map, callback, mutation bag,
  engine activation bypass, or temporary session-seed API exists.

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
    ([.packages[] | select(.source == null) | .name] | sort == [
      "world-authoring", "world-core", "world-defs", "world-model",
      "world-runtime", "world-standard"
    ]) and
    (.workspace_members | length == 6) and
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
      {name: "world-model", dependencies: [
        {name: "world-core", source: "local",
         uses_default_features: true, features: []},
        {name: "world-defs", source: "local",
         uses_default_features: true, features: []}
      ]},
      {name: "world-runtime", dependencies: [
        {name: "world-core", source: "local",
         uses_default_features: true, features: []},
        {name: "world-defs", source: "local",
         uses_default_features: true, features: []},
        {name: "world-model", source: "local",
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
        "world-defs", "world-model", "world-runtime", "world-standard"
      ])
  '
```

## Decision triggers

Stop before:

- exposing a repository, session-head constructor, record sealer, reservation,
  receipt constructor, or publication-capable value;
- moving record application, attempt control, or atomic publication outside
  `world-runtime`;
- adding a second backend, async persistence API, or registry dependency;
- adding a generic model map, delta framework, scheduler trait, callback, or
  extensible record family;
- pulling engine activation, standard transfer implementation, or lifecycle
  coordination into W3;
- allowing cancellation, disposition, or finalization to mutate `Σ`;
- changing a W1/W2 identity or package boundary.

## Completion evidence

```text
selected local packages:
  world-core
  world-defs
  world-model
  world-runtime
  world-authoring
  world-standard

direct local dependency graph:
  world-defs      -> world-core
  world-model     -> world-core, world-defs
  world-runtime   -> world-core, world-defs, world-model
  world-authoring -> world-core, world-defs
  world-standard  -> world-defs

new direct registry dependencies in W3:
  none

resolved registry closure:
  arrayref, arrayvec, blake3, cc, cfg-if, constant_time_eq,
  cpufeatures, find-msvc-tools, libc, minicbor, shlex

workspace Cargo manifests:
  Cargo.toml
  crates/world-core/Cargo.toml
  crates/world-defs/Cargo.toml
  crates/world-model/Cargo.toml
  crates/world-runtime/Cargo.toml
  crates/world-authoring/Cargo.toml
  crates/world-standard/Cargo.toml
```

Verified results:

- `world-model` contains only immutable, checked accepted-state, command,
  transfer, event, outcome, and snapshot values; it owns no aggregate root,
  store, history, scheduler mutation, or publication authority;
- `world-runtime` owns the one private authority waist: immutable execution
  binding plus `SessionHead` and `RunAttemptControl`, one exact reserved
  operation, one sealed record application, one aggregate publication point,
  and receipt-based reconciliation;
- `MomentRecordShape` is the sole new/rejected/accepted/retained/mismatch
  classifier; accepted transfer, commit, derived event, reaction envelope,
  command-ledger insertion, and post-commit dispatch cannot vary
  independently;
- the M1 scheduler owns one global sequence and at most one work item at an
  exact `SimMoment`; Fire is command-only, and globally least post-commit work
  returns `PostCommitRoutingRequired` without reservation, mutation, or
  skipping;
- `AttemptStepId` remains the semantic logical-operation identity, while the
  private `ReservationGrant` fences each physical authority grant; stale
  tokens can neither complete nor fail a re-granted step;
- publication and reconciliation verify the exact cursor-to-history-tail
  correspondence; truncated or mismatched prior history causes zero
  additional mutation and leaves the reservation fail-closed;
- exact request lookup precedes phase checks, retained commands are never
  reevaluated, cancellation changes only `Ca`, and finalization selects one
  immutable terminal cursor without changing `Σ`;
- service, driver, reader, prepared-Fire, and proposal capabilities expose no
  repository or reservation evidence through construction, cloning,
  formatting, equality, or a second scheduler-mutation path;
- 175 workspace unit/integration tests and 17 independent compile-fail
  doctests passed; these include 114 runtime tests covering concurrent
  admission, atomic publication, history mismatch, retry/reuse, dropped-token
  recovery, stale completion/failure fencing, explicit failure,
  cancellation, finalization, and the post-commit boundary;
- formatting, locked workspace check, warning-free Clippy, locked workspace
  tests, warning-free API documentation, metadata, dependency tree, manifest
  scan, and `git diff --check` passed;
- the executable metadata allowlist returned `true`, the dependency tree
  matched the graph above, exactly seven Cargo manifests were present, and
  the superseded-symbol scan returned no match.

## W4 handoff

W4's [completed plan](milestone-01-work-package-04.md) receives a sealed
immutable definition set and the complete M1 private
authority waist. It verifies and activates the runtime binding whose canonical
shape W3 owns, then supplies the trusted standard transfer implementation
binding, predicate/effect evaluation, runtime-owned role-bound delta
derivation, engine facade, public controller request, and one read-only
inspector query. W3 schedules each resulting post-commit dispatch at its exact
strictly later moment but never consumes it. W4 preserves the typed
`PostCommitRoutingRequired` boundary and neither drains, reschedules, skips,
nor invents a lifecycle result for that dispatch. W4 adds no new model state,
delta, command outcome, commit, event, or reaction form and cannot construct a
head, sealed record, reservation, receipt, or publication argument. M2
generalizes preparation to the complete same-moment, all-lanes batch and adds
engine-owned post-commit routing over the runtime-owned dispatch protocol
without changing the minimum W3 reservation, reconciliation, termination, or
finalization protocol. M5 adds persistent
control traces, pins, retention, replay, and durable recovery without changing
this authority boundary.
