# Formal System Model

## Status and purpose

This document defines the smallest normative model that explains the target
architecture. The other target documents refine its domain vocabulary,
protocols, and product surfaces.

The notation is a specification aid, not a mandate to build a generic
framework. In particular:

- the tuples below need not become one Rust struct;
- the transition relations need not become a registry of trait objects;
- the shared evaluator shape must not erase domain-specific request and result
  types;
- logical state partitions need not become separate databases or crates.

The implementation should use concrete types and private capabilities while
preserving the relations and invariants defined here.

## The explanatory kernel

The authoritative world-transition kernel reduces to five ideas:

```text
immutable execution semantics
  + one authoritative session state
  + capability-scoped immutable subsystem input
  -> bounded proposal or selected supplied ID
  -> verified atomic authority transition
  -> later typed causal work
```

Everything outside the authority transition either computes, proposes, routes,
observes, or changes only the explicit attempt-control protocol described
below. Only the kernel publishes a new authoritative world head.

The complete state of one controlled physical attempt is:

```text
Ωa = (Ca, Σ)

Ca = AttemptControlPlane for attempt a
Σ  = authoritative WorldSession state

AttemptControlPlane =
  RunAttemptControl
  append-only AttemptControlEventLog needed to derive the declared control
    verification trace
  durable AttemptArtifactPinLedger entries referenced by artifact-retention
    owner and transfer IDs
  retained StepPublicationReceipts required for reconciliation or verification
  content-addressed AttemptDisposition evidence referenced by a reservation or
    RunFinalization
```

`Ca` is a separately durable host protocol. It admits at most one world
transition at a time and freezes one terminal authority cursor. It chooses
whether execution continues, never how an admitted world transition is
interpreted. Attempt construction creates the initial bound pair
`Ωa,0 = (Ca, Σ0)` without advancing an existing world revision. After
construction, reserve, reconcile, cancel, retention, and compaction operations
change only `Ca`; the only transitions that can change an existing `Σ` remain
`Admit`, `Fire`, and `Manage`.

This model deliberately does not make context, cognition, planning, content
compilation, or analysis one universal pipeline. They are separate staged
transducers joined by typed records.

## Immutable execution semantics

For one session epoch, let the immutable execution environment be:

```text
Γ =
  ExecutionSpec
  EngineProtocolVersion
  RuntimeDefinitionSet
  SemanticImplementationSet
  LifecycleProfiles
  ExecutionConfigArtifact
```

`Γ` is formal notation, not an additional serialized artifact.
`requires(RuntimeDefinitionSet, LifecycleProfiles, ExecutionConfigArtifact)`
derives the exact required semantic-interface and implementation closure.
`ExecutionSemanticsManifest` is the sole normalized compatibility identity of
`EngineProtocolVersion`, `RuntimeDefinitionSet`, `SemanticImplementationSet`,
`LifecycleProfiles`, `ExecutionConfigArtifact`, and that derived closure;
component digests may be carried only as verified indexes into it. RNG/key
construction and any branch-affecting numerical or platform choices enter
through typed execution-configuration requirements and their exact semantic
implementation bindings, not as competing compatibility roots.
`ExecutionSpec` references that manifest and supplies the root seed,
`InitialStateRootId`, `TerminationContract`, and external-input
schedule/binding identity for this session.

`SemanticImplementationSet` means the exact behavior-affecting implementation
bindings selected for that closure: pack-required semantic primitives,
lifecycle implementations selected by `LifecycleProfiles`, and
configuration-selected algorithms whose code can change a logical result. It
is not the entire host build or every installed capability. Profiles and
configuration select requirements; the implementation set closes those
requirements against the engine distribution.

The process-local `ActivatedDefinitionRegistry` realizes the definition set
against the installed engine distribution. It may contain interned IDs,
indexes, caches, and dispatch tables. Those values are not durable identity
and must be reconstructible from `Γ`.

An engine build may contain unused primitive implementations and
semantics-preserving infrastructure. A pack binds only the exact semantic
interfaces it requires. Exact build identity remains provenance and may be
retained for strict reproduction, but unrelated installed capabilities change
neither `RuntimeDefinitionSetDigest` nor the normalized
`ExecutionSemanticsManifest`.

## Authoritative session state

Let a session state at revision `r` be:

```text
Σr =
  lineage
  mode
  now
  admission_frontier
  revision
  accepted_state
    domain
    epistemic
    social
    agency
  runtime_control
  scheduler
  authority_history_head
```

Where:

```text
mode
  Running | Paused | Quarantined | Failed

now
  last resolved SimMoment

admission_frontier
  earliest SimMoment at which new external input may become effective

runtime_control
  processes and reservations
  resolution representations and generations
  typed lifecycle continuations
  open action opportunities
  pending evaluator invocations
  coalescing and cancellation generations
  bounded retry and fallback dispositions
  input, management-request, and command deduplication ledgers

scheduler
  finite set of serialized typed triggers

authority_history_head
  distinguished epoch anchor or last AuthorityRecord identity,
  sequence, and cumulative hash
```

The semantic lineage embedded in the initial root and every authority-record
preimage is:

```text
EpochLineageBody =
    Origin(
      H("epoch-origin"
        || canonical initial-root semantic body with lineage and ID fields
           omitted)
    )
  | Child(
      parent ExecutionSpecId,
      parent AuthorityCursor,
      exact BranchTransformId or MigrationResetId
    )

EpochLineageId =
  H(canonical EpochLineageBody)
```

It identifies the semantic branch and epoch before execution begins.
`AttemptAuthorityDomainId`, `RunAttemptId`, host process identity, worker
identity, and storage location are excluded. A physical retry can therefore
reproduce the same semantic record identities, while a branch or migration
necessarily receives a new lineage.

The cursor used for compare-and-set, checkpoint binding, and finalization is:

```text
AuthorityCursor =
  EpochLineageId
  ExecutionSpecId
  world revision
  last RunRecordSeq, or 0 at the epoch root
  last AuthorityRecord hash, or epoch-record-anchor hash
  cumulative authority hash, or epoch-cumulative-anchor hash
```

Equality means canonical equality of the complete tuple, not merely revision
or sequence equality.

Runtime control is a storage partition, not a universal blackboard. It must
contain disjoint typed protocol records, each with one owner, state machine,
schema, and removal rule. A generic map of lifecycle values is forbidden.

Any subsystem state that can change the interpretation or result of a later
world transition must be in `Σ` or immutable `Γ`. `RunAttemptControl` may
change whether another transition is admitted and which already valid prefix
is retained, so it is modeled explicitly in `Ω`; it cannot alter transition
semantics. Everything else outside these declared planes is a reconstructible
cache, reliable-delivery obligation, or disposable telemetry and cannot
influence world behavior.

### Epistemic and social truth classes

The accepted partitions distinguish four value classes:

```text
Belief(a, proposition, support, confidence)
  ∈ accepted_state.epistemic

ActorSocialInterpretation(a, subject, meaning, support)
  ∈ accepted_state.social.actor_interpretations

IntersubjectiveClaim(claim_id, parties, act, proposition, provenance)
  ∈ accepted_state.social.claims

InstitutionalFact(scope, subject, status, constitutive_rule, evidence)
  ∈ accepted_state.social.institutional_facts
```

`Belief` is an actor-relative epistemic proposition and need not equal domain
truth. `ActorSocialInterpretation` is also actor-relative, but its predicate is
social meaning rather than a general descriptive proposition. Different
actors may hold incompatible interpretations of one event.

`IntersubjectiveClaim` makes the assertion, declaration, promise, or commitment
act authoritative history among its identified parties. It does not make the
embedded proposition a domain fact, belief, or institutional fact.
`InstitutionalFact` is authoritative only within `scope` because an installed
constitutive rule accepted it; it is not physical truth and need not be known
or accepted by every actor.

The transition constructors are disjoint:

```text
EpistemicGate accepts
  EpistemicTransitionProposal -> Belief delta

SocialGate accepts
  ActorSocialInterpretationProposal -> actor-interpretation delta
  | IntersubjectiveClaimProposal     -> claim delta
  | InstitutionalTransitionProposal -> institutional-fact delta
```

The optional `SocialInterpretationEvaluator` can produce only the first social
proposal family. Claim and institutional transitions require their own typed
social act or rule evidence. None of these transitions applies a physical
delta; a transaction that coordinates social and domain partitions declares
and verifies both gate receipts explicitly.

## Derived values

The following are derived artifacts rather than independent truth:

```text
ActorViewK =
  project(Γ, snapshot(Σ), actor, lifecycle K)

GroundedIntentCandidateSet =
  ground_intents(Γ, ActorViewIntent, permitted lifecycle values)

GroundedActionCandidateSet =
  ground_actions(Γ, ActorViewAction, ActionOpportunityView)

PolicyPayloadK =
  assemble(actor_safe_cause, permitted_time, ActorViewK,
           permitted lifecycle/controller values, grounded values)

PreparationReadEvidence =
  exact_authoritative_reads_and_versions(Γ, snapshot(Σ), admitted request)

PreparedTransaction =
  prepare(Γ, snapshot(Σ), admitted request, PreparationReadEvidence)
```

A derived artifact may be retained in a typed lifecycle continuation when it
must cross a microstep, external invocation, or checkpoint. Retention does not
turn it into domain, epistemic, social, or agency truth.
`PreparationReadEvidence` is runtime-owned same-step verifier input; only its
canonical digest may survive in authority history.

For every actor-facing lifecycle `K`, the trusted coordinator separates its
authority binding from its policy-visible meaning:

```text
InvocationEnvelopeK =
  authority head and global revision
  raw SimMoment and trigger provenance
  expected accepted-state versions
  LifecycleReadWitnessK, when retained or deferred
  private candidate-resolution and validation data
  private diagnostics
  PolicyPayloadK

PolicyPayloadK =
  actor and actor-safe semantic cause
  permitted actor-relative time projection
  typed actor-relative projections and availability
  permitted current semantic/controller state, where relevant
  actor-safe evidence, provenance, and diagnostics
  ActorInputFingerprint over the canonical visible payload body with this
    field omitted
```

These are conceptual layers refined by concrete port types, not one universal
envelope. The evaluator receives only `PolicyPayloadK`. The coordinator keeps
`InvocationEnvelopeK`, reattaches the result, and uses the private data for
freshness, selected-ID resolution, and commit validation. An inline evaluation
that completes inside one reserved prepared step is bound to that prepared
snapshot and expected versions and needs no dependency witness. A retained or
deferred evaluation carries its projector-owned `LifecycleReadWitnessK`
because its result survives the prepared step. The notation is refined by a
concrete port type rather than one universal runtime struct; M4 begins with
context-owned `ActionReadWitness`. Runtime's same-step
`PreparationReadEvidence` is a different value used to verify a prepared
authoritative transaction. Only the policy payload crosses the replaceable
evaluator boundary.

Grounded candidate IDs are computed before enclosing fingerprints and never
depend on them. A candidate-set fingerprint omits its own field from its
canonical preimage; the completed set may then enter the policy-payload
fingerprint, which also omits its own field. Checkpointed envelopes retain only
durable definition/entity references. Activation-local intern IDs,
implementation pointers, and dispatch handles are reconstructed after restore
and never enter durable identity.

### Grounding is not authoritative executability

For one open action opportunity `u`, actor `a`, action definition `d`, and
complete typed binding `β`, define actor-safe discovery separately from
runtime legality:

```text
CandidateΓ(V, u, d, β)
  iff InScope(u, d)
   ∧ CompleteTypedBinding(d.roles, β)
   ∧ DiscoverΓ(V, d, β)

ExecutableΓ(Σ, a, d, β)
  iff PermissionΓ(Σ, a, d, β)
   ∧ RequirementΓ(Σ, d, β)
   ∧ ResourceAvailabilityΓ(Σ, d, β)
   ∧ HardInvariantsΓ(Σ, d, β)
```

`V` is the action-lifecycle actor view, while `Σ` is authoritative state.
Grounding may evaluate only the first predicate. A candidate therefore proves
that every declared role is actor-safely and type-correctly bound; it does not
claim that hidden authoritative requirements hold. The private resolution
table has exactly one entry for every public candidate and cannot remove,
insert, or reorder candidates after consulting `Σ`.

For M3, let `BΓ(a, u, Σ)` be the primitive permitted action-view basis,
defined independently of projector output. Then:

```text
Σ1 ≈a,u Σ2
  iff canonical(BΓ(a, u, Σ1)) = canonical(BΓ(a, u, Σ2))
```

This non-circular basis is the fixture boundary for paired-state tests.

## Subsystem contract

Every replaceable trajectory-affecting semantic evaluator or controller is
conceptually a pure staged transducer:

```text
evaluateK:
  ΓK
  × immutable InputK
  -> Result<DecisionK, ErrorK>
```

`DecisionK` is the port's bounded semantic algebra and may include a
port-specific no-change or declined outcome where that outcome is meaningful.
`ErrorK` is a closed trusted-coordination failure algebra, not a simulation
choice. Proposed persistent implementation state, when present, is part of
the concrete decision and remains bound to the expected state version in the
private invocation envelope. For an actor-facing port, `InputK` is its
`PolicyPayloadK` and includes only the permitted semantic/controller state
view. This shape does not justify a universal `Subsystem<I, O>` trait, a
cross-lifecycle result envelope, or generic abstention and deferral variants.
Storage, publication, and external-I/O adapters instead refine their own
explicit protocols; they are not forced into this evaluator shape.
`ΓK` is likewise the exact capability-scoped semantic/configuration view
needed by the port, not ambient access to the full definition registry.

A conforming subsystem:

1. receives immutable, capability-scoped input;
2. cannot perform an outcome-affecting read after invocation begins;
3. returns a bounded algebraic result;
4. cannot carry the private commit capability;
5. selects supplied stable IDs instead of inventing executable structure where
   the boundary requires grounding;
6. includes any proposed persistent local-state value in its concrete decision
   while the coordinator binds it to the expected version retained in the
   private invocation envelope;
7. is deterministic under `Γ`, or uses the captured-external protocol;
8. has explicit no-change, failure, budget, and migration semantics where
   those concepts apply.

Execution mode is orthogonal to this semantic function. An inline deterministic
binding may complete during one moment evaluation. A captured-deferred binding
commits the private invocation and exact policy payload before dispatch, then
admits a canonical `DecisionK` at a later microstep. It cannot retroactively
join the creating moment. A `Manage` transition records cancellation, timeout,
or failure and schedules any declared fallback without fabricating a semantic
decision.

For retained or deferred work, invalidating a private
`LifecycleReadWitnessK` may cause trusted local projection, selected-ID
resolution, or runtime validation to run again. It creates a new logical
evaluator invocation only if the rebuilt `PolicyPayloadK` changes or a
separate actor-visible configured cause requests one. An explicit child
branch/epoch change to the evaluator semantic or configuration binding also
permits a new invocation; noninterference comparisons hold under fixed `ΓK`.
Transport retries keep the same invocation and idempotency identity.

The initial `FrontierBlocking` policy blocks the session frontier, not an
actor-local clock. When the creating `Fire` publication accepts a deferred
invocation, its pending control state stores
`blocked_at_frontier = resulting AdmissionFrontier` from that same record.
That creation publication may seal its own fired moment, but no later
transition may set a frontier greater than `blocked_at_frontier` while the
invocation remains unresolved. `Admit` atomically captures the result and
releases the barrier; `Manage` atomically records and releases or disposes it.
`HostScheduled` is the explicit nonblocking alternative.

### Retained action-evaluation product

The first concrete retained evaluator is action selection. Let `O` be one
accepted action opportunity and `I` one runtime invocation:

```text
O :=
  Open(v, generation)
  | Waiting(v, generation, invocation)
  | Consumed(v, generation, disposition)

I :=
  DispatchPending(request, artifacts, admission)
  | ResultCaptured(result, effective, scheduler key)
  | FallbackPending(cause, scheduler key)
  | Terminal(
      Applied(result, freshness)
      | Reinvoked(result, successor)
      | Failed(cause)
    )
```

The legal paired transitions are:

```text
begin:
  O: Open -> Waiting(i)
  I: absent -> DispatchPending

capture:
  O: Waiting(i) -> Waiting(i)
  I: DispatchPending -> ResultCaptured

apply:
  O: Waiting(i) -> Open -> Consumed
  I: ResultCaptured -> Terminal(Applied)

reinvoke:
  O: Waiting(i) -> Open(generation + 1) -> Waiting(successor)
  I: ResultCaptured -> Terminal(Reinvoked(successor))
  Isuccessor: absent -> DispatchPending

require fallback:
  O: Waiting(i) -> Waiting(i)
  I: DispatchPending | ResultCaptured -> FallbackPending

finish fallback:
  O: Waiting(i) -> Open -> Consumed(Failed)
  I: FallbackPending -> Terminal(Failed)
```

Each arrow in a two-edge opportunity chain is retained in canonical order in
the same authority publication. No transition skips directly from `Waiting`
to `Consumed`, and no terminal invocation can reopen an opportunity.

Freshness is the product of two independent checks:

```text
Freshness := PolicyProjection × ExecutionValidation

Current                            = unchanged × unchanged
ProjectionRebound                 = changed   × unchanged
ExecutionRevalidated              = unchanged × changed
ProjectionReboundAndExecutionRevalidated
                                   = changed   × changed
```

This product describes trusted reuse work, not evaluator-visible input. If the
rebuilt canonical policy payload is byte-identical, the original logical
decision remains eligible and private projection/resolution material is
rebound as needed. If the payload differs, the result cannot be applied: a
budgeted linked successor receives the next actor-visible generation, or the
fixed later fallback is scheduled. Execution legality is always revalidated
from current authority even when both witness components are unchanged.

Capture replay is a separate identity law. For capture identity `c` and
canonical capture fingerprint `fc`:

```text
ledger[c] absent       -> publish capture and retain (fc, outcome)
ledger[c] = (fc, out)  -> return out without publication
ledger[c] = (fc', _)   -> reject when fc != fc'
```

The exact-replay lookup precedes current invocation-state and frontier checks.
A different capture identity cannot affect an unknown, non-dispatch-pending,
or terminal invocation.

M4 realizes this authoritative invocation, captured-result, freshness, and
management protocol without requiring external transport. M5 realizes exact
restoration and replay from its captured state. M6 supplies authenticated
product adapters that dispatch already committed projection-safe requests.

## Authority records

Every authoritative revision transition publishes one outer record:

```text
AuthorityRecord :=
  Admission
    Commands(IngressBatchRecord)
      admitted CapturedInputRecord[]
      ingress-control delta
      scheduler delta

    | ActionEvaluation(ActionEvaluationAdmissionRecord)
      capture identity, fingerprint, and retained outcome
      invocation transition
      action-evaluation scheduler insertion
      capture-ledger and blocker delta

  | Moment(MomentBatchRecord)
    SimMoment
    resulting AdmissionFrontier
    consumed trigger IDs
    AttemptRecord[]
    accepted CommitRecord[]
    accepted-state delta
    optional session-mode delta
    runtime-control delta
    scheduler delta
    optional ReactionEnvelope

  | Management(ManagementBatchRecord)
    session pause, resume, quarantine, or failure
    invocation cancellation, timeout, or failure
    or admission sealing
    captured idempotent host request or deterministic kernel safety cause
    optional session-mode delta
    admission-frontier, runtime-control, and scheduler delta
    preserved unresolved-work frontier
```

Only the outer `AuthorityRecord` receives a `RunRecordSeq`, previous-record
hash, and cumulative hash. Inner attempt and commit IDs derive from the outer
record identity, record kind, and canonical local index. This avoids cyclic
identity among attempts, commits, and batches.

The canonical hash rule is non-recursive:

```text
record_preimage_body =
  canonical body with:
    outer identity/hash fields and serialized derived inner IDs omitted
    references to the enclosing record encoded as CurrentRecord
    references to same-record inner values encoded by kind and local index

record_id =
  H(EpochLineageId
    || RunRecordSeq
    || previous_record_hash
    || record_preimage_body)

cumulative_hash =
  H(previous_cumulative_hash || record_id)

attempt_id = H(record_id || "attempt" || canonical local index)
commit_id  = H(record_id || "commit"  || canonical local index)
```

The outer and derived inner IDs may be serialized for reference but are
normalized out of their own preimage and rechecked on load. Every reference to
the enclosing record—including a newly scheduled post-commit dispatch or an
idempotency outcome—is encoded as the distinguished `CurrentRecord` token.
Every same-record reference to an inner record—including references from
runtime-control or scheduler deltas and from other inner records—is encoded as
`(inner kind, canonical local index)`. Actual IDs are materialized only after
hashing. Canonical encoding, schema tags, map ordering, numeric encoding,
strings, optional values, and hash algorithms belong to the
persistence-format manifest. Record sequence or wall-clock admission order may
not act as a domain conflict tie-breaker. `AuthorityRecordId`,
`AttemptRecordId`, `CommitRecordId`, and every hash or ordinal derived from
persistence encoding are provenance only. Logical causality, random keys,
entity/process identity, conflict keys, wake generations, and scheduler
tie-breakers use format-independent semantic IDs whose construction belongs to
execution semantics.

## Kernel transitions

The transition relation has three authoritative actions.

### Durable idempotency

For each typed request family, the authenticated origin, request family, and
pre-run `EpochLineageId` select a non-reusable `IdempotencyNamespace`.
`RunAttemptId` and the not-yet-known `TrajectoryId` are excluded. Its request
ID contains a monotonically issued sequence number. Checkpointed deduplication
state contains both retained outcome entries and a compact non-reuse frontier:

```text
RequestFingerprintK =
  H(canonical request K with:
      request ID, retry/arrival metadata, and replaceable authentication-proof
        bytes omitted
      authenticated principal/scope and every authority- or effect-affecting
        field retained)

DedupStateK =
  retired_through
  retained[request sequence] ->
      Exact(RequestFingerprintK, original outcome)
    | Collision(canonical distinct-request-fingerprint set, original outcome)
        only when request family K defines same-barrier grouping
```

The retained fields include, as applicable, effective or target moment,
expected revision and witness, semantic bindings, actor/opportunity/cause,
management disposition, and typed body. Payload bytes alone never define an
idempotent request.

A namespace frontier advances only across a contiguous prefix whose IDs have
terminal outcomes or were explicitly closed as unused; reordered or unresolved
gaps remain retained. An authenticated exact duplicate in `retained`
short-circuits to its original outcome before current-state freshness,
frontier, or legality validation. A retained ID with another request
fingerprint fails closed. Any ID at or below `retired_through` returns `DuplicateExpired`
without an authoritative publication or effect; it is never treated as new.
This permits bounded outcome retention without permitting ID reuse. Exact
retry, mismatch, and retirement rules apply independently to input,
host-management, and command namespaces. `Collision` is an additional state
only for a family with a canonical same-barrier grouping operation—initially,
commands collected by one `Fire`.

Advancing `retired_through` is itself an authoritative ledger delta, selected
either by a manifest-bounded deterministic maintenance trigger or by an
explicit captured `Manage` disposition using durable acknowledgement/closure
evidence. It never occurs as wall-clock storage garbage collection. Physical
entry deletion may follow only after the authority record containing that
frontier advance commits.

Singular `Admit` and host-request `Manage` calls reserve an absent ID at their
linearization point. If different fingerprints race, one creates `Exact` and
the later request observes it and returns `IdReuseMismatch`; they do not invent
a batch collision.

For a canonical same-ID command group inside one `Fire`, ledger lookup and
reservation precede logical admission:

| Base ledger state | Canonical result |
|---|---|
| Retired | Every member returns `DuplicateExpired`; no request-specific attempt, effect, or ledger change |
| Retained `Exact(f, outcome)` | Members with fingerprint `f` return `outcome`; every other member fails `IdReuseMismatch`; no request-specific attempt, effect, or ledger change |
| Retained `Collision(_, outcome)` | Every member returns the stored collision outcome; no request-specific attempt, effect, or ledger change |
| Absent, one distinct fingerprint | One logical request may be admitted and create `Exact` |
| Absent, multiple distinct fingerprints | One group-level `IdCollision` attempt creates `Collision`; no request body is evaluated |

Thus a later mixed group can never replace an earlier exact outcome with a
collision tombstone or create another attempt for an existing ID. For commands
encountered inside `Fire`, “no request-specific change” does not suppress the
enclosing `MomentBatchRecord`: that publication still consumes its due triggers,
seals the moment, and records any deterministic control consequences.

### Admit

```text
Admit(Σ, external_input) -> Σ'
```

Preconditions:

- the kernel is at a serialized admission barrier;
- framing, authentication, basic decoding, and size limits have passed;
- after the idempotency short-circuit above, the input ID is admissibly new;
- an ordinary input's effective moment is not before
  `admission_frontier`;
- compatibility and authorization checks pass.

An accepted command input atomically publishes
`Admission(Commands(IngressBatchRecord))`, updates typed ingress-control and
input-deduplication state, and schedules its delivery. An exact retained
duplicate returns the original result without a second effect. Reuse of a
retained input ID with another request fingerprint fails closed; a retired ID
returns `DuplicateExpired`.

Capturing an external evaluator result may release a session-blocking frontier,
but the result cannot influence lifecycle policy until its scheduled delivery
is consumed.

Immediate session pause/resume/quarantine/failure, invocation
cancellation/timeout/failure, and admission-sealing requests use `Manage`, not
`Admit`: they target the current authoritative head rather than claiming a
simulation delivery moment. Cancelling the enclosing physical run attempt is a
separate `RunAttemptControl` disposition defined below.

Transport garbage rejected before admission belongs to operational or security
logs, not authoritative simulation history.

`submit_system_command` uses command admission.
`capture_action_evaluation_result` uses the distinct captured-decision
admission protocol and identity ledger defined above. A future external input
family must add its own concrete request, validation, identity, and delivery
contract rather than enter through a generic payload envelope. Actor action
selection begins from durable `ActionReady` work and never accepts an
arbitrary action key or binding set. `advance` and `drain_until` operate at
serialized barriers and return the resulting admission frontier.
`Fire` holds the exclusive admission/publication barrier while resolving its
due moment and seals that moment by advancing the frontier in the same atomic
publication. If `next(m)` is the least representable `SimMoment` strictly
after fired moment `m`, that publication must set
`admission_frontier' = max(admission_frontier, next(m))`. Advancing a frontier
across an interval with no fired moment uses an explicit admission-sealing
`ManagementBatchRecord` with a validated target. Once a publication has sealed
a moment, an input cannot be backdated into it.

### Fire

```text
Fire(Σ, least_due_moment) -> Σ'
```

Preconditions:

- the session is `Running`;
- the moment is the least due scheduler moment;
- the kernel holds the exclusive admission/publication barrier for that
  moment;
- no unresolved `FrontierBlocking` invocation forbids advancing the session
  frontier to `max(admission_frontier, next(moment))`;
- the current authoritative state satisfies its declared safety invariants.

The kernel:

1. drains all due triggers for the moment;
2. builds all eligible inputs from one immutable base snapshot;
3. evaluates pure work, potentially in parallel;
4. canonicalizes returned proposals;
5. computes declared read, write, resource, and invariant footprints;
6. runs a pure, terminating, permutation-invariant, total resolver;
7. verifies the combined accepted set;
8. constructs and seals one `MomentBatchRecord` containing every exact delta
   and, only when its reaction envelope is nonempty, the corresponding
   post-commit-dispatch scheduler delta;
9. calls `append_and_publish` once to atomically install that record, consume
   triggers, apply all state/control/scheduler deltas, seal the resulting
   admission frontier, and advance revision once.

Before parallel preparation, the kernel canonically groups due command
envelopes by `(source, CommandId)` and applies the base-ledger case table above.
Only an absent ID can form new work. Members with one distinct request
fingerprint form one logical command: one representative can be admitted,
producing one durable `AttemptRecord`, one outcome, and one `Exact` ledger delta
to which its duplicates resolve. An absent-ID group with multiple fingerprints
produces one durable group-level `IdCollision` attempt and `Collision` tombstone
before evaluation; no request becomes a winner. Every logical command admitted
after this step receives exactly one durable `AttemptRecord`.

If individually valid transactions violate a combined invariant, the resolver
must deterministically refine the accepted set and record every rejected
outcome. Rejecting the entire proposed accepted set while applying a safe,
declared trigger disposition is the mandatory total fallback; because the base
state is valid, this produces a valid `MomentBatchRecord` and preserves
`AttemptCoverage`. When declared failure policy requires it, that same moment
record may also set the session mode to `Quarantined` or `Failed`. The kernel
may not silently repeat the same uncommittable work.

### Manage

```text
Manage(Σ, management_cause, disposition) -> Σ'
```

Management is available even when ordinary scheduled work is blocked. It
accepts either an authenticated, bounded, idempotent host management request or
a deterministic kernel safety cause such as causal-budget exhaustion. It
validates and captures that cause in the same `ManagementBatchRecord` that
pauses, resumes, quarantines, or fails a session; records invocation
cancellation, timeout, or failure; or seals admission. Exact retained
host-request duplicates return the original outcome; retained ID reuse with
another request fingerprint fails, and a retired ID returns
`DuplicateExpired`.
The remaining work frontier is preserved unless the recorded disposition
explicitly disposes of it.

An admission-sealing disposition must name a target strictly greater than the
current frontier, may not skip scheduled work due before that target, and may
not cross an unresolved `FrontierBlocking` invocation. The same
`ManagementBatchRecord` may first resolve or explicitly dispose of that blocker
and then seal. Its postcondition is the exact validated target; other
management dispositions preserve the frontier unless their typed contract
declares and validates a change.

Checkpoint creation, restoration, verification, and branching are host
operations over authoritative heads, not hidden fourth mutation paths.

## Atomic publication

The storage backend is deferred; its abstract contract is not:

```text
append_and_publish(
  expected_head,
  sealed_record: AuthorityRecord,
  sealed_reservation: StepReservation
) ->
  Committed(
    resulting_head =
      apply_authority_record(expected_head, sealed_record),
    StepPublicationReceipt
  )
  | HeadConflict
  | ReservationMismatch
  | InvalidRecord
```

`apply_authority_record` is the canonical deterministic interpretation of the
record's exact deltas. Publication verifies the expected previous history link
(the last `AuthorityRecord` or the distinguished epoch anchor),
`RunRecordSeq`, derived IDs, revision increment, resulting admission frontier,
cumulative hash, and authority-history head; the resulting head is not an
independent caller-supplied value.

The runtime also verifies that the reservation's operation fingerprint matches
the exact transition request, due-work selector, or management cause from which
the sealed record was built. The publication linearization point stores a
`StepPublicationReceipt` binding the reservation, expected cursor, resulting
cursor, and record identity. This receipt is host-control provenance: it is
excluded from the authority-record preimage, `TrajectoryId`, random keys,
domain identities, conflicts, and scheduler ordering. It therefore proves
which reserved attempt step published a record without making physical
`RunAttemptId` part of world semantics.

The operation has one crash-safe linearization point. Recovery observes either
the complete old head or the complete uniquely derived new head. State,
scheduler, authority record, revision, and cumulative hash can never expose a
mixed publication.

A checkpoint is encoded from one immutable head. Its canonical
`checkpoint_state_fingerprint` hashes every authoritative checkpoint field,
complete cursor, and semantic/format identity while omitting that fingerprint
field itself. Installation requires both the exact revision/cursor and equality
with the canonical checkpoint projection recomputed from the expected head;
load recomputes the fingerprint from decoded fields. Compaction begins only
after that validated installation is durable.

## Run-attempt lifecycle

World-session health/control and run-attempt completion are different
lifecycles. `mode` remains the session's runtime condition; it does not acquire
a research-oriented `Completed` variant. Each physical attempt instead has a
separately durable host-plane state permanently bound to its world:

```text
AttemptBinding =
  AttemptAuthorityDomainId
  RunAttemptId
  ExecutionSpecId
  InitialStateRootId
  EpochLineageId

RunAttemptControl =
  binding: AttemptBinding
  creation_descriptor: AttemptCreationDescriptor
  creation_fingerprint =
    H(canonical AttemptCreationDescriptor)
  request_dedup: AttemptControlDedupState
  control_trace_head: ControlTransitionEventHash
  artifact_retention: AttemptArtifactRetention
  phase:
    Active(reconciled AuthorityCursor)
    | StepReserved(StepReservation, owner-local ReservationGrant)
    | Finalized(RunFinalization)

StepReservation =
  expected AuthorityCursor
  transition kind: Admit | Fire | Manage
  operation: canonical ReservedOperationDescriptor
  ReservedOperationFingerprint =
    H(canonical ReservedOperationDescriptor)
  AttemptStepId =
    H("attempt-step"
      || RunAttemptId
      || expected AuthorityCursor
      || ReservedOperationFingerprint)
  optional durable AttemptDisposition, initially absent

StepPublicationReceipt =
  AttemptBinding
  AttemptStepId
  ReservedOperationFingerprint
  expected AuthorityCursor
  resulting AuthorityCursor
  published AuthorityRecordId

AttemptDisposition =
    CancelRequested(CancelAttemptRequestId, request fingerprint, typed reason)
  | HostBudgetExceeded(disposition identity, typed evidence)
  | ExternalFailure(disposition identity, typed evidence)
  | EngineFailure(disposition identity, typed evidence)

RunFinalization =
  RunAttemptId
  terminal AuthorityCursor
  canonical finalization reason
  termination evidence:
    TerminationClauseId | AttemptDisposition digest
  TrajectoryId derived from that cursor

AttemptArtifactRetention =
    AttemptOwned(
      ResolvedExecutionClosureManifest digest,
      optional HandoffIntent
    )
  | RetainedBy(
      RunArtifactSet digest,
      root-relative ArtifactClosureManifest digest,
      AttemptArtifactRetentionRequestId,
      request fingerprint,
      owner-scoped transfer ID
    )
  | Discarded(
      AttemptArtifactDiscardRequestId,
      request fingerprint,
      former owned pin identities
    )

HandoffIntent =
  RunArtifactSet digest
  root-relative ArtifactClosureManifest digest
  AttemptArtifactRetentionRequestId
  request fingerprint
  owner-scoped transfer ID

AttemptArtifactPinLedger =
  idempotent owner-scoped source, provisional-target, retained-root, and
    release-pending pin records

AttemptControlEventLog =
  hash-chained ControlTransitionEvents
  distinguished initial previous hash =
    H("attempt-control-event-anchor" || RunAttemptId)

ControlTransitionEvent =
  monotonically issued control-event sequence
  previous control-event hash
  transition kind:
    Constructed
    | StepReserved
    | DispositionAttached
    | StepReconciled
    | CancelFinalized
  optional ReplayInput:
    Creation(AttemptCreationDescriptor, creation fingerprint)
    | HostStep(canonical HostStepIntent)
    | AnchoredControl(
        canonical CancelAttemptRequest or AttemptDisposition,
        AttemptControlInjectionAnchor
      )
  canonical ordered ExpectedObservations:
    InitialPhase(Active | Finalized)
    | Reserved(canonical ReservedOperationDescriptor, AttemptStepId)
    | Publication(Published(StepPublicationReceipt) | NoPublication)
    | Reconciled(Active | Finalized)
    | FinalizationEvaluated(
        TerminationClauseId or AttemptDisposition digest,
        terminal AuthorityCursor
      )
  resulting_control_state_digest =
    H(canonical resulting RunAttemptControl with control_trace_head omitted)

ControlTransitionEventHash =
  H("attempt-control-event" || canonical complete ControlTransitionEvent)

HostStepIntent =
  monotonically issued attempt-local input sequence
  Admit(canonical external-input reference and request fingerprint)
  | Fire(canonical advance or drain-until request and bounds)
  | Manage(canonical captured host-management reference and fingerprint)

AttemptControlInjectionAnchor =
    RootBarrier
  | BeforeStep(attempt-local input sequence)
  | ReservedStep(
      attempt-local input sequence,
      BeforePublication | AfterPublication
    )
  | AfterReconciledStep(attempt-local input sequence)
```

`AttemptStepId` is the semantic identity of the logical operation.
`ReservationGrant` is an owner-local authority epoch minted whenever control
enters `StepReserved`. If reconciliation releases an unpublished reservation
and the same logical operation is reserved again, the step ID may be equal but
the grant must differ. Every process capability for completing or failing a
step is bound to that exact grant, so a capability from the released
reservation is stale. The grant is control state, not world semantics, and is
excluded from operation fingerprints, step IDs, receipts, authority records,
cursors, cumulative history, trajectory identity, the canonical control-state
digest, and the control event log. Persistence stores the reservation; a
process keeps its current grant only while a corresponding process capability
can exist.

`Active` and `StepReserved` require `AttemptOwned` with no handoff intent.
`HandoffIntent`, `RetainedBy`, and `Discarded` are legal only after
`Finalized`; artifact retention never changes the selected terminal prefix.
Each `Constructed`, `StepReserved`, `DispositionAttached`, `StepReconciled`,
or `CancelFinalized` transition atomically appends one canonical composite
event and sets `control_trace_head` to its hash. Retention handoff/discard and
ledger compaction remain reconstructible control housekeeping and are excluded
from replay input. Rejected
mismatched/unauthorized calls do not mutate this authoritative control log;
optional security audit belongs to a separate trace whose presence is declared
by `TraceCompletenessManifest`. Control-log segments remain pinned until a
canonical `AttemptControlTraceArtifact` has durably taken ownership.

Omitting `control_trace_head` from `resulting_control_state_digest` is
mandatory: the event hash becomes that head, so including it would create a
self-referential identity cycle. Load recomputes the state projection, event
hash, and head equality.

That artifact preserves two typed partitions. `ReplayInputs` contains the
creation descriptor, the ordered `HostStepIntent`s, and captured accepted
exogenous control inputs such as cancellation or host/external failure
dispositions with their logical `AttemptControlInjectionAnchor`.
`ExpectedObservations` contains derived reservation descriptors and step IDs,
receipts, reconciliation outcomes, generated safety-management steps, semantic
termination-clause selection, and finalization barriers. Verification drives
only `ReplayInputs`; it regenerates and compares `ExpectedObservations` and
`RunFinalization`. Neither an expected cursor/selector nor an expected clause
or receipt is fed back as input.

`ReservedOperationDescriptor` is family-specific and concrete:

```text
Admit
  Command
    exact framed request identities/fingerprints and effective-moment intent
  | ActionEvaluationResult
    exact capture/invocation/request fingerprint and effective-moment intent

Fire
  exact advance/drain request plus the due-moment/trigger selector derived
  from the expected head

Manage
  exact authenticated request fingerprint or deterministic safety-cause
  identity and typed disposition
```

It contains enough information to prevent one reserved call from publishing a
different operation, but it is not a universal command envelope.

`StepPublicationReceipt`s are append-only members of `Ca`, although their
durable bytes may be co-located with the world publication for atomicity. The
receipt for an unresolved reservation is retained until reconciliation; any
receipt required for verification is retained through the referenced
`AttemptControlTraceArtifact`. Removing an unreferenced reconciled receipt
is storage compaction and cannot alter `RunAttemptControl` or world semantics.
The same rule retains the canonical bytes behind every referenced
`AttemptDisposition` digest; a finalization or trace never depends on a digest
whose evidence blob is unavailable.

`AttemptBinding` is checked whenever the control record is opened, reserved,
reconciled, archived, or restored. An active or reserved record cannot be
attached to another authority domain, specification, root, epoch, or attempt.

Attempt creation is a durable atomic create-or-open keyed by `RunAttemptId`.
`AttemptCreationDescriptor` contains the complete `AttemptBinding`,
runner-assigned attempt key, root cursor, exact resolved-execution
`ResolvedExecutionClosureManifest` digest, and format version. That immutable
manifest contains only pre-run execution dependencies; it excludes attempt
control, captured run inputs/results, history, receipts, finalization, and
downstream run/study artifacts. Load recomputes both
`RunAttemptId = H(AttemptAuthorityDomainId || ExecutionSpecId || attempt key)`
and the creation fingerprint from these canonical bytes. The resolved closure
is materialized and pinned before or atomically with creation, which
initializes `AttemptArtifactRetention::AttemptOwned(M0, None)`; no control
record may be published without that pin. Dynamic session/history dependencies
are pinned by their own live authority, scheduler, and delivery roots and by
frozen checkpoint/run closure manifests, never by mutating the creation
manifest. If no record exists, creation binds the world root, constructs its
`TerminationView`, and atomically installs the initial pair with either
`Active(root cursor)` or a root-level `Finalized` state. No mutation capability
is exposed before that check. An exact retry opens the same attempt; a
different creation fingerprint under the same ID fails closed. Coordinators
inside one attempt-authority
domain cannot create two physical sessions under one `RunAttemptId`; fresh
independent domains necessarily derive different IDs. Opening after a crash
addresses that same authoritative control record; it does not clone it. Every
portable copy is read-only. Without an exclusive fenced transfer protocol,
continuing from a copy requires a child root and new attempt identity.

The execution specification contains a pure, bounded, versioned,
stage-checked `TerminationContract`. It receives a capability-scoped view, not
the complete authority head:

```text
TerminationView =
  permitted session mode and SimMoment fields
  declared accepted-state projections and termination signals
  declared scheduler-quiescence facts
  declared deterministic simulation-budget counters
  declared external-input completion facts

terminate_project(Γ, Σ) -> TerminationView

evaluate_termination(Γ, TerminationView)
  -> Continue | Finalize(reason)
```

The contract declares its exact read set, and authoring verifies each read
against the termination-view schema. `TerminationView` does not expose world
revision, authority hashes or record IDs, deduplication ledgers, storage
metadata, raw scheduler entries, or private lifecycle state unless a
separately defined semantic termination signal deliberately projects their
meaning. Compaction and persistence provenance therefore cannot accidentally
become termination semantics.

A serialized `ExecutionSpec` or `TerminationContract` is untrusted input.
Before minting `ResolvedExecution`, the artifact resolver recomputes canonical
identity and reverifies schema/version, ordered-clause normalization,
boundedness, read-set and stage legality, and every required semantic-interface
binding. Session creation never accepts an authoring-time verification claim
without this load-time check.

Its clauses and precedence are canonical. A manifest-fixed
`RunFinalizationPolicy` combines the semantic termination decision with at most
one explicit durable `AttemptDisposition` and maps cancellation, host budget,
external failure, or engine failure to typed final reasons under a canonical
precedence rule. Wall-clock observation, an unrecorded exception, and other
uncaptured host state cannot select a prefix.

A non-cancellation disposition discovered while a step is reserved is
compare-and-set into that reservation and applied only by reconciliation at
the last receipt-validated cursor: expected if nothing published, or the exact
direct successor if publication completed. It cannot move the cursor.

Attempt-scoped `Admit`, `Fire`, and `Manage` use a durable single-step gate:

1. atomically reserve the active attempt at its exact current cursor;
2. perform at most one authority transition through `append_and_publish`,
   which atomically persists the matching `StepPublicationReceipt`;
3. while later attempt operations remain excluded, evaluate termination on the
   resulting immutable head's `TerminationView`;
4. compare-and-set the reservation either to `Active(new cursor)` or to one
   immutable `RunFinalization`.

Reconciliation is fail-closed:

| Observed world state | Required reconciliation |
|---|---|
| Head equals the reserved expected cursor and no receipt exists | No publication occurred. If the reservation has no disposition, atomically return to `Active(expected)`; the caller may resubmit through the normal idempotent surface. If it has a disposition, project the expected head and finalize there under the fixed policy |
| Head is the exact direct successor of expected and the atomic receipt matches the complete binding, step ID, operation fingerprint, record, and both cursors | Rerun only projection plus the pure termination/finalization rule and install `Active(successor)` or `Finalized` |
| Any missing/mismatched receipt, non-successor head, second successor, binding mismatch, or invalid cursor | Report storage/control corruption and retain `StepReserved`; issue no mutation capability and claim no automatic finalization |

Thus a crash after world publication but before the final reservation update
cannot cause another
effect or move the terminal cursor. Reconciliation does not call a lifecycle
evaluator or external service. No second world transition can begin from
`StepReserved`. Recovery never autonomously reconstructs and reruns an
unpublished operation from hashes; after release, a caller resubmits through
the normal typed idempotent surface. External invocation dispatch is already a
durable protocol action and never an uncaptured call inside publication.

A finalized attempt grants no further world-mutation capability. Read-only
inspection, checkpoint/archive creation, and idempotency lookup for a
previously handled exact retry remain legal; genuinely new input, management,
or scheduled execution under that `RunAttemptId` is rejected. Continuing from
the terminal cursor requires an explicit child root/branch and a new attempt.
`RunAttemptControl` never mutates `Σ`, so it is not a fourth world-authority
path; it durably gates calls to the three existing transitions and selects one
unique trajectory prefix.

Artifact retention is an orthogonal, durable control state. Creation installs
`AttemptOwned(M0, None)`, where `M0` is the immutable
`ResolvedExecutionClosureManifest`. Finalization does not release that pin.
After constructing a self-contained `RunArtifactSet` root `R` and frozen
root-relative closure `M1`, a typed, idempotent retention request performs:

1. compare-and-set `AttemptOwned(M0, None)` to
   `AttemptOwned(M0, HandoffIntent(R, M1, request, fingerprint, transfer))`;
2. durably acquire the `R`/`M1` pin under that owner-scoped transfer ID;
3. compare-and-set the prepared state to
   `RetainedBy(R, M1, request, fingerprint, transfer)`;
4. idempotently release the attempt-owned source pins.

Recovery resumes or aborts a prepared handoff and releases any orphaned
provisional target pin by inspecting the durable transfer ID and retention
state. A crash may therefore leave an extra pin, never zero pins. The reverse
ordering is forbidden.

An explicit terminal discard first resolves any prepared handoff, then
compare-and-sets the current owner to
`Discarded(request ID, fingerprint, former owned pin identities)` while
retaining the immutable binding, creation descriptor/fingerprint, finalization
identity, and request-dedup tombstone. Only then may this control owner's
attempt or `R`/`M1` pins be released; other independent retained roots remain
pinned. Exact handoff/discard retries return their original deduplicated
outcome while that exact entry is retained; retired request IDs return
`DuplicateExpired`, and same-ID/different-request reuse fails closed.
`start_attempt` against a discarded exact creation descriptor returns
`AttemptArtifactsDiscarded`; it never recreates the attempt.
Post-finalization retention housekeeping cannot alter `Σ`, the terminal
cursor, reason, or `TrajectoryId`.

`AttemptControlDedupState` uses the same exact-retry, mismatch, and
non-reuse-frontier semantics as singular `Admit`/`Manage` ledgers, but its
namespace additionally includes `RunAttemptId` and it is stored and compacted
with the attempt-control record. Cancellation, retention handoff, and terminal
discard are distinct singular request families; none has batch-collision
semantics. Each fingerprint covers the complete binding, authenticated
principal/scope, typed body and target/evidence where applicable, while
excluding only request ID, arrival/retry metadata, and replaceable proof
bytes.

An authenticated cancellation linearizes in one control-store compare-and-set:

```text
RunAttemptControl {
  phase = Active(cursor),
  request_dedup = base ledger
}
+ admissibly new CancelAttemptRequest
  -> RunAttemptControl {
       phase = Finalized(
         cursor,
         policy(AttemptDisposition::CancelRequested(...))
       ),
       request_dedup = updated exact ledger outcome
     }
```

Finalization, its disposition evidence, and the deduplication outcome publish
atomically. Exact retries return the original outcome; a different fingerprint
under the retained ID returns `IdReuseMismatch`; retired IDs return
`DuplicateExpired`. If different same-ID cancellations race, the first
linearized request creates `Exact` and the other observes that entry; there is
no artificial cancellation batch or collision tombstone.
Cancellation cannot pass `StepReserved`: if reservation wins first, the
cancellation receives only a transient retry-after-reconciliation response and
does not consume its request ID; if cancellation wins first, the step
reservation fails `AttemptFinalized`. This selects a serialized prefix without
a world transition. It cannot pause, fail, seal, or otherwise impersonate
session `Manage`.

The complete attempt relation is therefore explicit:

```text
construction lane, creates one bound Ω0 without advancing an existing Σ
  ConstructAttempt | OpenExactAttempt

world lane, changes Σ and atomically adds its receipt to Ca under one reservation
  Admit | Fire | Manage

post-construction attempt-control lane, changes only Ca
  ReserveStep
  ReconcileStep
  FinalizeFromDisposition
  InstallRunArtifactRetention
  DiscardAttemptArtifacts
  CompactAttemptControlLedger
```

`CancelAttempt` is the authenticated request form of
`FinalizeFromDisposition`. Other disposition forms require a typed,
independently captured control input or deterministic manifest-fixed cause.
Attempt-ledger retirement requires durable acknowledgement or explicit gap
closure and follows the same contiguous-frontier rule as world request
ledgers; wall-clock garbage collection cannot change it. These control
transitions explain permission and retained-prefix selection in `Ω` without
creating another way to change `Σ`.

## Reaction closure

The reaction payload of a moment batch is:

```text
ReactionEnvelope =
  observable domain and semantic events
  changed dependency keys
  explicit reliable integration events
```

The envelope is immutable and self-contained for routing. Its scheduled
`PostCommitDispatch` may retain the source batch ID for provenance but cannot
depend on compactable history for execution. In the source batch's canonical
preimage, that source reference is `CurrentRecord`; the actual batch ID is
materialized after hashing.

```text
needs_dispatch(batch) := reaction_envelope(batch) is nonempty
```

Consuming a dispatch is an ordinary `Fire` transition. The pure, engine-owned
`PostCommitRouter` maps the envelope to observation deliveries, invalidations,
and lifecycle-wake proposals. Runtime owns the scheduled dispatch and commits
the typed proposals through a later authority transition; it never depends on
context. A dispatch-consumption batch creates another dispatch only when it
produces a new nonempty reaction envelope; an empty reaction terminates.

## Agency kernel

The minimal persistent agency relations are:

```text
supports    EvidenceRecord -> Belief
owns        Activity -> Intent
focuses     Actor -> Activity?                     baseline
sponsors    ActionOpportunity -> Activity | ActorReaction
origin      ProcessInstance -> AttemptRecord
awaits      Activity <-> ProcessInstance
```

Evidence delivery and accepted evidence are different:

```text
EvidenceDelivery
  transient typed scheduler input

EvidenceRecord
  accepted actor-relative epistemic provenance

Belief
  accepted actor-relative claim
```

Assimilation produces one epistemic transition containing delivery disposition,
evidence-record delta, and belief delta.

The lifecycle state machines are:

```text
Intent
  Absent -> Active
  Active <-> Suspended
  Active | Suspended -> Achieved | Abandoned | Failed

Activity
  Absent -> Active
  Active -> Waiting | Suspended | Completed | Failed | Cancelled
  Waiting | Suspended -> Active | Completed | Failed | Cancelled

ActionOpportunity
  Absent -> Open
  Open -> WaitingForEvaluation
  WaitingForEvaluation -> Open
  Open -> Consumed
```

Intent adoption schedules activity initialization. The `ActivityController`
owns initialization and advancement. It may use planning, search, a behavior
tree, a script, or learned state internally.

Opening an action opportunity is an accepted agency/control transition.
`ActorReadyForAction` is only the scheduler trigger that references the open
opportunity. A ready `Select` or `NoApplicableAction` decision consumes it
exactly once. An inline policy error follows the declared engine-failure path
rather than becoming another action choice. A `DeferredCaptured` execution
binding moves the opportunity to `WaitingForEvaluation`; every completion
first returns it to `Open`. A result then consumes it, a visible-input change
opens a linked successor evaluation, and recorded cancellation, timeout,
failure, or exhausted fallback consumes it through the same checked
`Open -> Consumed` edge. There is no direct
`WaitingForEvaluation -> Consumed` transition.
Waiting, suspension, retry, and reconsideration are later activity or intent
directives. A bounded retry opens a causally linked successor with a new ID.

Required agency invariants:

- every live activity has exactly one nonterminal intent;
- a terminal intent has no live dependent activity;
- a suspended intent has no active foreground activity;
- the baseline has at most one open or evaluation-pending action opportunity
  per actor;
- every opportunity has one explicit sponsor;
- an activity-sponsored foreground opportunity matches the actor's focused
  active activity version and the exact action opening represented by its
  retained method state;
- process state changes only through runtime authority;
- activity termination never implicitly terminates a process;
- activity/process relationships are causal origin and explicit
  subscription, not ownership.

## Actor-relative noninterference

For fixed `Γ` and lifecycle-profile bindings, let:

```text
Σ1 ≈actor,role Σ2
```

mean that two authoritative states are indistinguishable under the actor and
role's permitted knowledge boundary.

Every actor-facing projector must satisfy:

```text
Σ1 ≈actor,role Σ2
  implies
project(Σ1, actor, role) = project(Σ2, actor, role)
```

This includes the complete policy payload; candidate presence, ordering,
actor-safe IDs, and fingerprints; perceived feasibility; score features;
diagnostics; missing-data behavior; the presence, timing, generation, and safe
identity of lifecycle wakes; and logical evaluator invocation/dispatch
presence, timing, and generation. Hidden authoritative facts may cause
different runtime acceptance after an attempt, but any actor-visible feedback
must pass observation and epistemic boundaries before it refines what the
actor can distinguish.

Global revision, raw `SimMoment`, dependency stamps and
`LifecycleReadWitnessK` when a retained lifecycle needs them, raw trigger or
authority-record IDs, private diagnostics, and private candidate-resolution
data remain in
`InvocationEnvelopeK`; they are not policy metadata. Actor-facing IDs and
fingerprints derive only from canonical actor-visible content or a
visibility-stable actor-local generation sponsored by an already actor-visible
lifecycle record. Thus:

```text
Σ1 ≈actor,role Σ2
  implies
PolicyPayloadK(Σ1) = PolicyPayloadK(Σ2)
and actor_visible_ids_and_order(Σ1) = actor_visible_ids_and_order(Σ2)
and logical_invocation_traceK(Σ1) = logical_invocation_traceK(Σ2)
```

This is semantic noninterference over simulation-visible values and logical
evaluator requests. Constant-time CPU, cache, and network-latency side-channel
resistance is a separate host-isolation concern; operational latency never
becomes policy input or authoritative causality.

The rich `AuthoritativeAttemptResolution`—including acceptance/rejection,
reason, retryability, revision, and attempt/commit/process references—is
engine-private. Replaceable controllers receive only a neutral
`AttemptResolved(ActionOpportunityId)` cause whose visible identity does not
depend on that resolution, plus separately projected actor-relative context
and accepted evidence. For the same submitted opportunity and actor-visible
sponsor state, acceptance and rejection schedule exactly one such cause with
the same profile-fixed effective microstep, generation rule, and visible
identity. Omission or cancellation may depend only on actor-visible sponsor
state. Protocol retryability cannot decide whether an actor-facing successor
opportunity exists.

Raw process progress/completion remains engine-private. A controller may
receive `ActivityMonitorTriggered(MonitorId)` only when the monitor identity,
predicate, cadence, and generation were predeclared over actor-visible input.
Otherwise process meaning must travel through reaction, observation, evidence
assimilation, and later actor-relative projection. Opaque IDs derived from
hidden authority records are never substituted for a safe cause.

## Compiler and activation model

Authoring is a partial deterministic staged transformation:

```text
parse_manifests
  -> resolve exact package/source graph
  -> compile sources topologically
  -> resolve names and type-check family-specific source forms
  -> lower to executable family IR
  -> ArtifactData
  -> validate(ArtifactData, SemanticInterfaceCatalog)
  -> deterministic artifact encoding
  -> sealed VerifiedPackArtifact
  -> finalize artifact-digest PackLock
  -> link immutable RuntimeDefinitionSet
  -> activate process-local ActivatedDefinitionRegistry

loaded ArtifactEnvelope
  -> format, version, length, and digest checks
  -> decode ArtifactData
  -> validate(ArtifactData, SemanticInterfaceCatalog)
  -> sealed VerifiedPackArtifact
  -> the same lock, link, and activation path
```

These are semantic phases, not a requirement for one public AST/HIR/IR type per
arrow.

The compiler obligations are:

```text
ResolutionClosure
  every import resolves to one exact selected package and exported definition

StageSafety
  an operation legal in one IR family or authority stage cannot execute in
  another

LoweringPreservation
  each accepted lowering preserves the declared meaning of its source
  definition under the same semantic-interface bindings; a non-obvious
  optimizer is admitted only with an enforced construction invariant or
  checkable translation validation

BoundedEvaluation
  accepted authoritative IR terminates structurally or under deterministic
  fuel without partial effects

CanonicalIdentity
  equivalent normalized input under one compiler protocol produces one
  canonical semantic fingerprint

ActivationValidation
  loaded serialized definitions cannot activate without decoding and the same
  owner validation applied to compiler-produced ArtifactData

RequiredInterfaceClosure
  activation matches every referenced semantic interface exactly; unused
  installed interfaces are irrelevant
```

Catalog-superset independence is:

```text
restrict(C, required(D)) = restrict(C', required(D))
  implies
validate(D, C) = validate(D, C')
```

Compiler passes and optimizers may evolve internally. A new source language
does not change runtime authority. A new semantic primitive does.

## Content, scenario, and root materialization

T0 content is classified by meaning rather than forced into one artifact:

```text
ReusableT0Declaration
  --compile/validate--> VerifiedPackArtifact
  --link-------------> RuntimeDefinitionSet

ScenarioArtifact                     owned by world-lab
  immutable study planning/provenance
  checked root-materialization recipe
  --world-lab materializer--> materialized root candidate

ProductInitialWorldSource            owned by a product composition root
  product-specific checked root-materialization input
  --product materializer---> materialized root candidate

RootMaterializationEnvironment =
  exact RuntimeDefinitionSet
  exact accepted-state and runtime-control schemas
  semantic requirements needed to validate starting values and work

verify_root(
  RootMaterializationEnvironment,
  materialized root candidate
) -> VerifiedInitialStateRoot
```

Reusable pack declarations may describe archetypes or checked instances, but
do not themselves create authoritative entity identity. `ScenarioArtifact`
and product source schemas are not runtime mutation protocols or alternative
definition sets. `VerifiedInitialStateRoot` is the runtime-owned canonical
boundary: leaf-owned materializers lower their source schemas before invoking
the runtime-owned root validator, so runtime never depends on `world-lab` or a
product. The verified root contains only the complete materialized state and
provenance required to begin the epoch, validated under the exact definitions
and semantics it references.

For a scenario artifact `q`:

```text
verify_root(
  RootMaterializationEnvironment,
  materialize_scenario(q)
) = root

execution_identity depends on InitialStateRootId(root)
study identity may additionally depend on digest(q)
trajectory identity does not depend on digest(q) independently of root
```

Thus two descriptive scenario artifacts that materialize the same root and
execution specification have the same execution identity, while their
`RunCase` provenance may remain different. Changing a source artifact after
materialization cannot change an active epoch.

## Execution and study identity

Identity is constructed without self-reference:

`AttemptAuthorityDomainId` is the durable, globally unique namespace of one
exclusive writable attempt-control domain. Replicas sharing one linearizable
control store share the ID. A newly initialized independent domain receives a
different ID; a portable archive retains the source ID only as provenance and
does not acquire its writer capability. The ID is host-control identity and is
excluded from world records, `TrajectoryId`, RNG keys, and semantic
configuration.

```text
InitialStateRootId =
  H(canonical InitialStateRoot body without its own ID or child
    ExecutionSpecId field)

ExecutionSpecId =
  H(canonical ExecutionSpec body without its own ID field)

RunCaseId =
  H(StudyDesignArtifact digest
    || canonical assignment key
       including factor, condition, block, replicate, and exact optional
         ScenarioArtifact provenance
    || ExecutionSpecId)

RunAttemptId =
  H(AttemptAuthorityDomainId
    || ExecutionSpecId
    || runner-assigned attempt key)

TrajectoryId =
  H(ExecutionSpecId || terminal-or-retained-prefix cumulative authority hash)
```

`InitialStateRoot` contains the exact starting session mode, current
`SimMoment`, admission frontier, accepted state, runtime-control state,
scheduler, and parent-only lineage/migration/reset references needed to begin
an epoch. It never contains its child `ExecutionSpecId`. A scenario
materialization, branch, or migration first produces this root and
`InitialStateRootId`; the canonical `ExecutionSpec` then references that ID.

Binding the two produces the initial authoritative head only after a
fail-closed compatibility check:

```text
RootCompatible(root, root_id, spec, spec_id) =
  H(canonical root body without its own ID or child ExecutionSpecId field)
    = root_id
  and root_id = spec.InitialStateRootId
  and H(canonical spec body without its own ID field) = spec_id
  and root.EpochLineageId =
      H(canonical root.EpochLineageBody)
  and every accepted-state, scheduler, runtime-control, and lifecycle-state
      value is valid under the RuntimeDefinitionSet, lifecycle state schemas,
      execution configuration, and semantic implementations named by the
      spec's ExecutionSemanticsManifest
  and every pending ingress, delivery, invocation, deadline, and termination
      reference is valid under the spec's external-input binding and effective
      termination contract
  and every migration/reset/parent reference satisfies its declared lineage
      and conversion contract

bind_initial_root(root, root_id, spec, spec_id) =
  undefined unless RootCompatible(root, root_id, spec, spec_id)
  otherwise Σ0 with:
    revision = 0
    authority_history_head = EpochAnchorCursor {
      EpochLineageId = root.EpochLineageId
      ExecutionSpecId = spec_id
      world revision = 0
      last RunRecordSeq = 0
      previous-record hash =
        H("epoch-record-anchor" || root_id || spec_id)
      previous-cumulative hash =
        H("epoch-cumulative-anchor" || root_id || spec_id)
    }
    next RunRecordSeq = 1
```

The distinguished epoch anchor cursor is not an `AuthorityRecord`; the first
record uses its two hashes as the previous-record and previous-cumulative
inputs. A root checkpoint may therefore encode a complete cursor while
referencing both the root and specification without a hash cycle. Branch
lineage names parents and source cursors only, never a not-yet-derived child
identity. A content-valid but semantically incompatible root/specification pair
cannot construct a session or epoch.

The canonical `ExecutionSpec` body is stored under `ExecutionSpecId`; there is
no independent `ExecutionSpecArtifactDigest`.

The `ExecutionSpec` body contains only pre-run settings that may affect the
trajectory and references one `InitialStateRootId` and one normalized
`ExecutionSemanticsManifest` digest.
Study design, intended metrics, capture policy, report layout, and analysis
method do not enter `ExecutionSpecId`. A metric change creates a new analysis
result, not a new execution specification.

`ExecutionSpecId` is a pre-run configuration identity, not by itself a
trajectory identity when admitted input, management, or deferred evaluator
results remain open. `TrajectoryId` identifies the resulting authoritative
trajectory prefix. A study-scoped immutable `RunCase` records the exact
assignment, including one `ScenarioArtifact` digest when used or an explicit
no-scenario/root-origin value. A separate immutable `RunCaseResult` maps that
case to the selected `RunAttemptId`, `RunArtifactSet`, and `TrajectoryId` or a
terminal no-trajectory status. Scenario provenance and study selection never
enter attempt or trajectory identity.

## Safety properties

Implementations must preserve:

```text
Authority
  only append_and_publish changes the authoritative head

AtomicPublication
  state, scheduler, history, revision, and matching publication receipt expose
  old or new together

AttemptCoverage
  every admitted command has exactly one durable outcome

TriggerConservation
  every consumed trigger appears in exactly one MomentBatchRecord

ReactionConservation
  every nonempty ReactionEnvelope is pending, consumed, or explicitly disposed

NoPastScheduling
  no new trigger precedes the sealed admission frontier

Idempotency
  duplicate InputId, ManagementRequestId, or CommandId cannot create a second
  effect; compacted IDs remain permanently non-reusable through their typed
  namespace frontier

RunFinalization
  one RunAttempt has at most one finalization cursor and no later world
  transition scoped to that attempt

AttemptControlIntegrity
  distinct writable attempt-authority domains have distinct IDs; one domain
  has one linearizable control owner; one RunAttemptId has one immutable
  AttemptBinding; every reserved successor is unchanged or receipt-proven;
  cancellation has one deduplicated control-plane outcome; at most one owner
  drives each reliable-delivery obligation; every portable copy is read-only

AttemptArtifactAvailability
  AttemptOwned pins the exact creation-time closure while independent
  authority, scheduler, and delivery roots pin dynamic recovery state;
  RetainedBy pins its complete frozen run root before releasing source pins;
  Discarded is reachable only after finalization and preserves a permanent
  non-reuse tombstone

AcceptedStateIntegrity
  every publication that changes accepted state preserves all hard state
  invariants

FailureContainment
  a publication may additionally enter an explicit non-running mode, but that
  mode never authorizes an invalid accepted-state delta

HistoryIntegrity
  authority records, revisions, and branch lineage form one valid hash-linked
  prefix

RecoveryClosure
  checkpoint + exact committed tail + artifact closure reconstructs one head

AttemptRecoveryClosure
  world head + bound AttemptControlPlane + a matching publication receipt
  exactly when the reserved phase observed a publication + retained
  control-input evidence reconstruct one permission/finalization state

KnowledgeNoninterference
  actor-facing projections, wakes, policy inputs, and logical evaluator
  invocation/dispatch behavior do not distinguish hidden-state differences
```

## Determinism and progress

For a fixed:

```text
initial checkpoint
immutable Γ and artifact closure
canonical admitted ordinary-input trace, including captured deferred results
canonical management trace, including admission sealing
AttemptControlTraceArtifact.ReplayInputs, containing only accepted captured
  host step intents and logically anchored exogenous control inputs
```

the logical transition relation is a partial function. Final accepted state and
authority-history fingerprints, regenerated
`AttemptControlTraceArtifact.ExpectedObservations`, and the earliest
finalization cursor are unique. Worker count, thread completion, allocation,
hash iteration, wall clock, and telemetry do not select a logical result.

Progress is conditional. Under weak host fairness, terminating bounded inline
work, and an eventually admitted result or recorded management disposition for
every session-blocking external invocation, plus eventual reconciliation of
any durable `StepReserved` attempt:

```text
Active RunAttempt over a Running session with due work
  eventually publishes a new revision
  or publishes Paused | Quarantined | Failed
  or durably finalizes at the current serialized barrier
```

The engine does not promise that an actor satisfies a desired condition, that a
domain policy eventually accepts an action, or that an external service
responds.

## Refinement obligations

Each implementation increment should demonstrate a refinement from concrete
state and operations to this model:

1. every concrete authoritative head maps to one valid `Σ`;
2. every concrete attempt-control plane and its retained evidence maps to one
   valid `Ca`, and the bound pair maps to one valid `Ωa`;
3. every concrete publication corresponds to one `Admit`, `Fire`, or `Manage`
   transition;
4. no concrete path changes mapped `Σ` without such a transition, and no path
   changes mapped `Ca` without a declared attempt-control transition or the
   receipt addition atomically coupled to that world transition;
5. serialization and restoration preserve both mapped states and their
   binding;
6. optimized projection/evaluation preserves observable results;
7. negative tests show that authority, stage, freshness, identity, and
   compatibility violations are rejected without a transition or partial
   effect;
8. every concrete attempt-scoped mutation of `Σ` is covered by one durable
   `RunAttemptControl` reservation, and recovery preserves its unique
   finalization cursor;
9. artifact handoff acquires and records a target pin before releasing the
   source, while discard records the permanent tombstone before releasing its
   former pins;
10. trace replay supplies only captured control inputs and compares, rather
    than supplies, derived reservations, receipts, reconciliation, semantic
    termination, and finalization.

Formal verification is not required for the first implementation. The
relations above are precise enough to drive state-machine tests,
property-based tests, deterministic trace comparison, and later executable
specification if the risk justifies it.

## Pattern correspondence

Several established patterns help explain the design, but none adds another
layer:

| Lens | Correspondence | Limit |
|---|---|---|
| Information hiding | Subsystems own likely-to-change algorithms and representations | Boundaries follow semantic ownership, not arbitrary file layers |
| Functional core / imperative shell | Projection, evaluation, routing, and preparation are pure; publication is narrow | Persistent controller state is explicit, not hidden in closures |
| Ports and adapters | Controllers, evaluators, storage, and external services sit behind typed boundaries | Ports do not receive generic mutation handles |
| Object capability | The private commit capability is the authority | A type named “state” is not authority |
| Compiler staging | Source, family IR, linked definitions, and activation have different legality rules | There is no universal IR or mandatory pass framework |
| Durable protocol state machine | Idempotency, deferred invocation, and run finalization expose explicit states and crash reconciliation | There is no generic workflow engine or shared blackboard |
| Transactional outbox | Reaction and reliable integration work are durable before dispatch | Telemetry remains lossy and non-authoritative |
| CQRS-like separation | Snapshots serve reads; commands request changes | The design is not pure event sourcing |

Pattern names are explanatory vocabulary, not reasons to create pattern-named
modules, traits, or abstractions.
