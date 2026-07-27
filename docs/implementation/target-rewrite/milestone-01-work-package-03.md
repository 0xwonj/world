# M1/W3: Runtime Authority

## Status

Active.

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
- complete same-moment batching, footprints, conflict resolution, reaction
  routing, or lifecycle scheduling;
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
| Semantic implementation dispatch and transfer legality | deferred W4 |
| Engine facade and controller request | deferred W4 |

`world-model` contains no mutable aggregate root, store, history append,
scheduler mutation, `apply_*`, or publication-capable value.

## Minimum model surface

W3 introduces only immutable model values that have both a runtime consumer
and the already-selected W4 transfer consumer. The exact initial physical
records are chosen while implementing the first root and snapshot; empty
placeholder families and a generic property bag are forbidden.

The minimum public shape is:

```text
WorldSnapshot
  authoritative WorldRevision
  immutable accepted model state

CommandEnvelope
  exact command source, identity, and fingerprint
  actor and selected action definition
  typed role bindings

CommandAttemptOutcome
  stable rejected Fire result in W3
  first concrete accepted transfer result added with its W4 producer
```

`AdmitOutcome` is separate from `CommandAttemptOutcome`. `Admit` captures and
schedules a `CommandEnvelope`; only a later `Fire` produces the command's
accepted or rejected execution outcome. The admitted representation is not a
second logical command identity.

W3 adds only the accepted-state records needed to construct its root,
snapshot, and stable-rejection path. W4 introduces the concrete
ownership/containment record and typed transfer delta together with the trusted
transfer producer. Neither package may add a universal component map, dynamic
value tree, or generic mutation list merely to bridge the two packages.

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
  input, management, and command ledgers
  minimum scheduler state
  EpochLineageId and declared parent/reset origin

ExecutionSpec
  schema and canonicalization versions
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

The M1 termination contract contains only the concrete root/record conditions
needed by the slice; its tags and canonical fields are frozen before the first
vector. Implementation and lifecycle selections may be empty only when the
derived requirement closure is empty. The transfer-shaped owner-local fixture
instead supplies exact private deterministic rejection-only bindings for every
required interface operation. W4 constructs a separate production manifest
with production bindings in the same format. Slots are never absent, replaced
by an undefined "fixture digest," or added to the same format later.

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
  SessionClock and admission frontier
  accepted model state
  minimum typed runtime-control state
  minimum typed scheduler state
```

`SessionHead` has no public constructor, clone-to-authority conversion,
replacement method, or setter. The root constructor validates its immutable
binding and creates the distinguished root cursor without publishing an
ordinary revision.

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

## Attempt authority domain

One `RuntimeService` repository/control domain supplies and permanently owns
one `AttemptAuthorityDomainId`; an attempt request, `ExecutionSpec`, activated
execution, root, or semantic implementation cannot choose it. W3's in-memory
repository retains that identity for its lifetime. A later persistent backend
stores the same logical field, and independently writable authority domains
must receive distinct values.

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
  AttemptOwnedClosure
  Live {
    Active(AuthorityCursor)
    | Reserved(StepReservation)
  }
  | Finalized(RunFinalization)
```

`AttemptCreationDescriptor` contains the complete binding, raw
runner-assigned attempt key, root cursor, exact
`ResolvedExecutionClosureManifest` digest, and control-format version. The
receipt contains the complete binding, `AttemptStepId`,
`ReservedOperationFingerprint`, expected and resulting cursors, and published
`AuthorityRecordId`.

The closure manifest is materialized and retained before, or atomically with,
attempt creation. W3 implements only the initial `AttemptOwnedClosure` state
needed by a live attempt; post-finalization handoff and discard remain
deferred. Owner-local fixtures construct the complete minimum canonical
root/specification/closure binding and exercise the runtime-owned termination
projection. W4 activation becomes the first external producer of production
bindings in those frozen formats.

Attempt creation evaluates the runtime-owned root termination rule and
installs either `Active(root)` or root-level `Finalized`. It never exposes an
unchecked active capability.

`StepReservation` binds:

- attempt and immutable execution binding;
- exact expected cursor;
- `Admit`, `Fire`, or `Manage` step kind;
- the complete canonical family-specific `ReservedOperationDescriptor`;
- its exact `ReservedOperationFingerprint`;
- the deterministically derived `AttemptStepId`;
- an optional retained `AttemptDispositionId`, initially absent.

The descriptor is retained control evidence. Reconciliation never attempts to
reconstruct it from its fingerprint.

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
  | PreparedFireFailed {
      AttemptStepId
      PreparedFireFailure
    }

AttemptDispositionId =
  H("world-attempt-disposition-v1"
    || canonical AttemptDisposition)
```

The private `AttemptDispositionStore` retains the exact canonical value under
that identity. A prepared-Fire disposition is attached to its reservation;
the cancellation disposition is created only by the `Active -> Finalized`
control compare-and-set. `RunFinalization` references the retained
`AttemptDispositionId`. The cancellation request fingerprint covers its
binding and typed reason while omitting the request ID and retry metadata.

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
because its scheduler admits at most one command at an exact moment. Collision
grouping, retirement, and compaction are M2 work. Attempt creation and
cancellation use their separate `Ca` request-deduplication records.

Cancellation consults its `Ca` deduplication record before the attempt phase.
A retained exact request returns its original `CancelAttemptOutcome`, including
after finalization; the same ID with another fingerprint returns a typed
mismatch. For an absent request:

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
attempt control + dispositions + private session head + authority history
  + receipts
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

W3 admits one due command per prepared moment; complete same-moment batching
is M2 work. `PreparedFire` owns the captured snapshot and due selection, so
the input borrows them and cannot outlive the token. Runtime rejects a
missing, duplicate, unknown, or differently bound proposal before record
sealing.

`RuntimeSessionReader` is not part of staged input and is never passed to an
evaluator. After preparation, proposal construction may depend only on the
token's immutable `EvaluateCommand` input and execution capabilities selected
before evaluation; a later reader call cannot affect that proposal.

W3 can privately construct only the stable-rejection proposal needed to prove
new-command publication. A narrow checked no-new-work constructor accepts only
a `ResolvedCommand` input, allowing W4 coordination to complete and consume
that due trigger without evaluating it. There is no public generic delta,
event, map, callback, or arbitrary proposal constructor. W4 exposes the
checked stable-rejection construction and adds the first public accepted
proposal together with its concrete containment delta and trusted transfer
producer. That addition does not change the non-cloneable token, immutable
input, or runtime-owned sealing/publication boundary.

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
- enforces the M1 scheduler invariant of at most one pending command at an
  exact `SimMoment`; an occupied or already sealed moment returns the stable
  typed nonpublishing `MomentSlotUnavailable` result and leaves the input ID
  absent;
- captures one typed `CommandEnvelope` and schedules its later delivery;
- seals one `IngressBatchRecord` containing the request-ledger outcome;
- exact retry returns the original outcome without a second publication.

Pause/resume cannot remove pending work, and the admission frontier never moves
backward. Once an exact moment is occupied, it remains occupied until that
single command fires and the moment becomes sealed, so
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

- `prepare_fire` reserves the exact cursor and selects the least due typed
  work;
- because W3 admits at most one command at an exact `SimMoment`, the selected
  moment contains exactly one due command and therefore already satisfies the
  M1 drain-all-due rule;
- the returned token exposes either one immutable new-command evaluation input
  or one no-evaluation retained/mismatch command-ledger resolution;
- `complete_fire` consumes that exact token and a closed proposal value,
  revalidates the reservation and command-ledger classification, and seals one
  `MomentBatchRecord`;
- W3 proves successful staged publication with a legitimate stable command
  rejection before W4 adds accepted transfer semantics; it must not expose a
  generic mutation bag for the test;
- retained exact or mismatched command identities consume their due trigger
  and record their resolution without another semantic evaluation or model
  effect;
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
`fail_prepared_fire` consumes the prepared token, constructs and stores the
exact canonical `PreparedFireFailed { AttemptStepId, failure }` disposition,
attaches its identity to the reservation, and reconciles/finalizes `Ca` at the
last receipt-validated cursor without changing the session head or authority
cursor. Richer family-specific evidence is added only with a concrete later
producer and a corresponding canonical schema.

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
        config.rs
        semantics.rs
        termination.rs
        spec.rs
        initial_root.rs
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

## Work sequence

### 1. Freeze the minimum authority protocols

- select the exact W3 model records required by the transfer slice;
- freeze the complete M1 `InitialStateRoot`, `ExecutionSpec`, minimum
  semantics/configuration/termination values, and closure-manifest slots;
- freeze root, record, cursor, reservation, receipt, attempt, and finalization
  identity preimages;
- freeze the authority-domain, reserved-operation, prepared-Fire disposition,
  and cancellation request/disposition contracts;
- freeze the minimum closed record and error algebras;
- record deterministic golden vectors before implementation.

### 2. Add immutable model values

- add `world-model` with exact final dependency edges;
- implement only concrete accepted records, `CommandEnvelope`,
  the stable command-attempt outcome, and cloneable read snapshots used by W3;
- prove model values carry no store or publication authority.

### 3. Implement root and record derivation

- add `world-runtime`;
- implement canonical execution values, checked root/specification binding,
  private head, structurally distinct cursor positions, record
  draft/seal/apply, and cumulative history;
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
- enforce one pending command per exact M1 moment;
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
  `StepReserved`, and grants no mutation capability even when the head has
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
- two admissions targeting one exact `SimMoment` schedule at most one command;
  the other receives `MomentSlotUnavailable`, publishes no record, and leaves
  its input ID absent;
- one attempt has at most one reserved world step at a time;
- `PreparedFire` is non-Clone and single-use;
- a dropped token leaves a repository-retained reservation and reconciliation
  follows the declared cases while that repository remains alive;
- explicit failed Fire and cancellation do not change the session head or
  authority cursor;
- cancellation linearizes only from `Active` and atomically installs its
  disposition, finalization, and `Ca` deduplication outcome;
- exact cancellation retry returns its retained outcome even after
  finalization, while same-ID/different-fingerprint reuse fails without
  mutation;
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
  while retained/mismatch inputs permit only the checked no-new-work proposal;
- every completed Fire has exact identity/fingerprint coverage for that due
  command, with no missing, duplicate, unknown, or differently bound proposal;
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

The metadata and dependency tree receive executable exact allowlists after
the W3 manifests are added.

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

To be filled after every gate passes.

## W4 handoff

W4 receives a sealed immutable definition set and the complete M1 private
authority waist. It verifies and activates the runtime binding whose canonical
shape W3 owns, then supplies the trusted standard transfer implementation,
engine facade, public controller request, and one read-only inspector query.
It adds the first concrete ownership/containment delta and accepted
transfer-proposal constructor behind the
`MomentWorkInput`/`MomentWorkProposals` seam selected here, but cannot
construct a head, sealed record, reservation, receipt, or publication
argument. M2 later generalizes the reserved-step subset with the complete
in-memory reservation, reconciliation, termination, and finalization
protocols. M5 adds persistent control traces, pins, retention, replay, and
durable recovery without changing this authority boundary.
