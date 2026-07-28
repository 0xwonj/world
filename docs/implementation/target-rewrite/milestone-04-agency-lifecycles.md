# M4 Plan: Independently Scheduled Agency Lifecycles

## Status

Complete and exit-reviewed.

This document records the M4 implementation order and acceptance criteria.
Completion evidence and the final architecture review are recorded in the
[M4 exit review](milestone-04-exit-review.md).

## Goal

Extend the completed M3 actor-control waist into one deterministic causal
spine in which evidence, belief, appraisal, intent, activity, action
opportunity, action, and process have different owners and lifetimes.

For fixed execution semantics `Γ`, authoritative state `Σ`, and actor `a`, the
principal flow is:

```text
committed occurrence
  -> later post-commit routing
  -> actor-addressed evidence delivery
  -> accepted evidence and belief transition
  -> later appraisal
  -> later intent adoption or reconsideration
  -> activity initialization or advancement
  -> activity-sponsored M3 action opportunity
  -> existing M3 projection, selection, lowering, and runtime validation
  -> authoritative action or process transition
  -> later causal continuation
```

Every arrow that changes authoritative state is a separate prepared,
proposed, sealed, and applied transition. No lifecycle recursively executes
the rest of the stack.

M4 also adds the semantic protocol for retained and deferred action
evaluation. The request is committed before dispatch, the result is captured
before use, and freshness is decided from positive dependencies rather than
global revision alone.

## Non-goals

M4 does not introduce:

- a universal lifecycle runner, global cognition tick, pass graph, or
  synchronous evidence-to-action call chain;
- a generic `Disposition<T>`, generic lifecycle state map, mutable context
  blackboard, string-keyed property bag, or universal provider registry;
- a planner port or a mandated behavior-selection algorithm;
- GOAP, HTN, behavior trees, utility AI, language-model prompting, or scripts
  as cross-system architecture;
- a capability, action, process, social, intent, activity, or cognition DSL;
- a complete belief-revision system, probabilistic fusion, truth-maintenance
  graph, emotion taxonomy, theory of mind, relationship model, or norm engine;
- a fake partial projection, synthetic failure switch, or unused
  `Unavailable` branch;
- a generic process framework without one concrete time-bearing world
  mechanic;
- automatic travel-intent adoption or travel-activity initialization from
  appraisal; the first travel method is a checked execution-origin seed;
- arrival appraisal or automatic completion of an awaiting travel activity;
  M4 routes arrival into accepted evidence but leaves the agency transition
  for a later concrete semantic slice;
- changing the existing immediate containment-transfer action into a delayed
  action without a separately authored semantic contract;
- checkpoint restoration, verification replay, branching, or persistent
  backend support for the new records; those are M5;
- CLI, MCP, network queues, authentication, evaluator sessions, or product
  transport; those are M6;
- background population, actor-local clocks, multi-resolution simulation, or
  promotion and demotion; those are M7;
- compatibility aliases or a parallel legacy path for the pre-M4 state
  shape.

Concrete policy implementations may use richer algorithms internally later.
M4 keeps their state bounded, canonical, versioned, and private to the
specific lifecycle implementation.

## Normative contracts

- [Formal System Model](../../architecture/target-architecture/formal-model.md)
- [System Architecture](../../architecture/target-architecture/system-architecture.md)
- [Target Rust Code Architecture](../../architecture/target-architecture/code-architecture.md)
- [Cognition and Agency](../../architecture/target-architecture/cognition-and-agency.md)
- [Architecture Decisions](../../architecture/target-architecture/decisions.md)
- [Validation Scenarios](../../architecture/target-architecture/validation-scenarios.md)
- [Target Architecture Execution Roadmap](../../architecture/target-architecture/implementation-roadmap.md)
- [Reference Game Vision](../../design/reference-game-vision.md)
- [M4 research synthesis](../../research/m4-agency-lifecycles-research.md)
- [M3 exit review](milestone-03-exit-review.md)

The target package owns authority, dependency direction, state partitioning,
and lifecycle separation. This plan selects the smallest M4 implementation
that proves those contracts.

## Entry-state evidence

M3 closes with one working action path:

```text
ScheduledWork
  -> PreparedDelivery and MomentWorkInput
  -> engine-private evaluation
  -> MomentWorkDecision and MomentWorkProposals
  -> runtime seal and atomic apply
  -> optional later ScheduledWork
```

The repository already provides:

- immutable execution resolution and a normalized execution-semantics
  manifest;
- a typed scheduler and complete least-due moment batching;
- post-commit dispatch work and bounded same-tick causal waves;
- authoritative compare-and-set sealing and atomic publication;
- durable reaction-sponsored action opportunities;
- actor-safe action payloads and private candidate-resolution tables;
- ID-only action selection, private lowering, and runtime revalidation;
- neutral `AttemptResolved` wakes that do not reveal rich runtime outcomes;
- same-moment deterministic conflict resolution and hidden-state
  noninterference tests.

The M4 entry audit found these gaps.

| Area | Current state | Required correction |
|---|---|---|
| Lifecycle semantics | The pre-M4 singleton profile denotes one synchronous mode while an arbitrary installed action policy can still change behavior | Bind every enabled behavior-affecting lifecycle implementation and state schema into canonical `Γ` before adding persistent lifecycle state |
| Accepted state | `AcceptedState` currently denotes only containment/domain state | Make `AcceptedState` the aggregate of typed domain, epistemic, social, and agency partitions |
| Routing | `PostCommitDispatch` is consumed without semantic follow-up | Route self-contained reaction material into typed later lifecycle work |
| Agency | Intent and activity do not exist; action opportunities have only reaction sponsorship | Add persistent versioned intent/activity and activity sponsorship without another action path |
| Evidence | No accepted evidence or belief transition exists | Add actor-addressed delivery, assimilation, provenance, and belief change |
| Appraisal | No material-change gate exists | Add a derived, actor-relative appraisal continuation and later agency wake |
| Processes | No process family or authoritative time-bearing mechanic exists | Add exactly one real relocation/travel process and grounded actor control |
| Deferred action | M3 evaluation is stack-local and synchronous | Add one concrete pending-action-evaluation state machine, captured ingress, witness, freshness, and fallback |
| Partial projection | All current checked in-memory projections are total | Do not add `Unavailable` until a real partial provider and consumer exist |

The correction order is architectural, not cosmetic. Persistent evaluator
state cannot be interpreted honestly until its implementation identity belongs
to `Γ`, and new accepted records cannot be placed correctly until
`AcceptedState` is partitioned.

## Fixed architectural shape

### Canonical semantics before lifecycle state

`Γ` must close over the exact behavior-affecting implementations:

```text
Γ =
  ExecutionSpec
  EngineProtocolVersion
  RuntimeDefinitionSet
  SemanticImplementationSet
  LifecycleProfiles
  ExecutionConfigArtifact
```

M4 replaces the singleton lifecycle marker with explicit bindings for each
enabled concrete port. A binding identifies:

- lifecycle interface and protocol version;
- selected semantic implementation identity;
- persistent implementation-state schema when that port has state;
- deterministic mode and fallback policy where those choices can change
  logical results.

An optional lifecycle is explicitly `Disabled` or exactly bound. Missing,
unknown, duplicate, or incompatible bindings fail during execution
resolution. A host may install an implementation catalog, but it cannot
replace a selected implementation after `ResolvedExecution` has been sealed.

Port protocol identity is positional in `LifecycleProfilesV2`: the evidence,
appraisal, social, intent, activity, and action fields name six distinct
interfaces. Any incompatible change to one port's payload, result, or state
contract therefore requires a new outer lifecycle-profile schema and canonical
domain. A field cannot silently acquire a different interface meaning while
retaining the V2 profile identity.

Changing an action policy, evidence assimilator, appraisal evaluator, intent
policy, activity controller, or enabled social evaluator changes the
`SemanticImplementationSet` and normalized
`ExecutionSemanticsManifest`. Transport retries, worker identity, allocation,
and telemetry do not.

### Accepted-state partitions

The physical-only state becomes a domain partition. The aggregate shape is:

```text
AcceptedState
  domain
  epistemic
  social
  agency
```

Ownership is fixed as follows.

| Value | Partition |
|---|---|
| containment and later physical facts | accepted `domain` |
| `EvidenceRecord` and `Belief` | accepted `epistemic` |
| accepted actor-relative or institutional social state | accepted `social` |
| `Intent` and `Activity` | accepted `agency` |
| `ActionOpportunity` | typed `runtime_control` protocol |
| `ProcessInstance` and process generation | typed `runtime_control` protocol |
| pending evaluator invocation, captured result, retry, cancellation, and fallback | typed `runtime_control` protocol |
| scheduler triggers and coalescing work | `scheduler` and typed `runtime_control` cursors |
| appraisal output | derived invocation result or typed continuation, never independent accepted truth |

Each partition has concrete checked records, canonical encoding, digest
coverage, and owner-local transitions. Empty initial partitions are real typed
empty states, not a generic map.

The old physical `AcceptedState` name is not retained as a compatibility
alias. Existing containment constructors, queries, initial-root encoding,
snapshot projection, authority application, and tests move to the new domain
partition deliberately.

### One execution waist

Every new work family extends the existing M2/M3 path:

```text
scheduled typed input
  -> prepared delivery
  -> immutable snapshot and private context construction
  -> concrete evaluator or deterministic coordinator
  -> typed proposal
  -> one authority seal
  -> canonical apply
  -> later typed work
```

There is no second lifecycle executor and no evaluator receives mutation
authority. `world-runtime` continues to own scheduling, compare-and-set,
validation, publication, and protocol state transitions.

The scheduler may share identity and ordering primitives, but every lifecycle
work variant has its own payload, preparation rule, proposal, seal validation,
apply rule, and removal rule.

The scheduler algebra is closed without making lifecycle evaluation generic:

```text
ScheduledWork
  Command
  PostCommit
  Lifecycle(LifecycleWork)
  ActionReady

LifecycleWork
  EvidenceDelivery
  Appraisal
  IntentReview
  ActivityInit
  AttemptResolved
  ActivityAdvance
```

`LifecycleWork` uses one stable scheduler lane and a canonical variant order.
The nesting limits top-level scheduler and authority-record growth; it is not a
universal runner. Each variant still has its own concrete input, proposal,
gate, transition record, and apply path. In particular, `ActionReady` remains
the existing M3 waist rather than becoming a second lifecycle abstraction.
Lane or variant order never establishes state visibility: a complete
least-due moment is evaluated against one shared base snapshot, so every
causal dependency that must observe a predecessor is scheduled at a later
microstep.

### Distinct lifetimes

```text
Intent
  why a commitment persists across attempts

Activity
  how one selected implementation pursues that commitment

ActionOpportunity
  one bounded actor-relative decision boundary

Action
  one immediate grounded attempt

ProcessInstance
  authoritative time-bearing world execution
```

An intent does not disappear because one action is rejected. An activity may
open several causally linked opportunities. An activity never mutates a
process directly. Suspending or terminating an activity does not implicitly
suspend or terminate a process.

The implemented persistent method state is one closed sum with two real
consumers, not a generic state map:

```text
ActivityState
  ContainmentTransfer(ContainmentTransferActivityState)
  Travel(TravelActivityState)

TravelActivityState
  source
  destination
  next opportunity generation
  Pause | Resume | AwaitArrival
```

An activity-sponsored relocation start traverses the ordinary action waist.
Its neutral `AttemptResolved` continuation lets the travel method offer Pause,
then Resume, then enter Waiting while the process continues independently.
The controller retains no route-process identity or progress coordinate; the
engine privately carries the exact attempted route into the next grounded
scope and runtime revalidates current process legality.

M4 seeds the first travel intent, focused activity, and Start opportunity as
one checked execution-origin tuple. The origin validator requires the
opportunity to name that exact focused active activity and to match the
retained post-opening method state. General appraisal-to-travel
initialization is deliberately not claimed.

### Concrete lifecycle ports

`world-decision` owns one object-safe port and one concrete result vocabulary
per implemented lifecycle:

```text
EvidenceAssimilator
AppraisalEvaluator
IntentPolicy
ActivityController
ActionPolicy
SocialInterpretationEvaluator (future optional port; disabled in M4)
```

The deterministic baseline implementations are completed before deferred or
replaceable variants.

Action evaluation remains semantically:

```text
Select(GroundedActionCandidateId)
NoApplicableAction(...)
```

Inline or deferred execution is selected by the resolved action-policy
binding before an opportunity is evaluated. Dispatch, wait, timeout,
cancellation, failure, fallback, discard, and reinvocation are states or
transitions of the invocation-control protocol. They are not extra
action-policy answers and are not unified into a public generic disposition.

### Causal spine and later microsteps

A lifecycle result never calls the next lifecycle recursively. Instead, its
accepted proposal may schedule one or more typed successors at a strictly
later microstep:

```text
domain event
  -> PostCommitDispatch
  -> EvidenceDelivery
  -> EvidenceAssimilationNeeded
  -> AppraisalNeeded
  -> IntentReviewNeeded
  -> ActivityInitNeeded or ActivityAdvanceNeeded
  -> ActorReadyForAction
  -> existing M3 action path
  -> AttemptResolved
  -> ActivityAdvanceNeeded
```

Coalescing removes redundant wake requests, not semantic input or provenance.
Each concrete lifecycle scheduling key tracks the equivalent of:

```text
desired_generation
processed_generation
enqueued_generation?
```

The exact coalescing law is:

1. A nonempty canonical set of distinct material causes accepted for one key
   in one authority batch advances `desired_generation` exactly once and
   retains every cause. Duplicate wake requests do neither.
2. If no generation is enqueued, runtime schedules the current desired
   generation at a strictly later microstep and records it as
   `enqueued_generation`. Otherwise the wake coalesces into the existing
   entry.
3. Work for generation `g` seals only when
   `enqueued_generation == g`. A successful transition sets
   `processed_generation = g`; it never copies the then-current desired
   generation.
4. Seal computes the final desired generation after incorporating every
   same-batch cause. If it is newer than `g`, runtime schedules exactly one
   successor carrying that final generation at a later microstep.
5. Failed or stale evaluation does not advance the processed generation.
   Materially equal appraisal may advance it, but does not create an intent
   wake.

Thus a cause committed while generation `g` is being evaluated remains dirty
after `g` commits and cannot be lost. Evidence deliveries themselves do not
coalesce; only recomputation requests for appraisal, intent review, activity
initialization, and activity advancement do.

M2's complete least-due batching defines same-moment order. The existing
same-tick causal budget remains the only recursion/cycle bound.

For the deterministic W3/W4 spine, activity recovery uses an
outcome-independent profile-fixed offset:

```text
μ0  authoritative action attempt resolves
μ1  PostCommitDispatch, when nonempty, and neutral AttemptResolved
μ2  EvidenceDelivery is assimilated into accepted evidence and belief
μ3  appraisal processes the accepted epistemic state
μ4  intent review processes material appraisal
μ5  activity initialization or attempt recovery
μ6  ActionReady for an opportunity opened at μ5
```

At `μ1`, `AttemptResolved` only retains an actor-safe activity cause; it never
invokes `ActivityController` directly. For an activity-sponsored opportunity,
runtime schedules recovery at `μ5` for accepted, rejected, and unobservable
outcomes alike, including when no evidence delivery exists. The selected
lifecycle timing profile and its four-microstep recovery offset are part of
`Γ`. Consequently recovery observes every deterministic upstream transition
without leaking a hidden attempt outcome through wake presence or timing.
Future deferred upstream lifecycles must replace the bounded offset with an
explicit typed dependency fence rather than silently lengthening it.

### Deferred action evaluation

Deferred action evaluation is added only after the deterministic causal spine
works. It is a concrete action protocol, not a generic asynchronous evaluator.
The installed action policy is closed:

```text
Inline(ActionPolicy)
Deferred(DeferredActionEvaluatorDescriptor)
```

Runtime never retains or calls a policy callback for the deferred case.

#### Opportunity and invocation control

An action opportunity has exactly these states:

```text
Open
WaitingForEvaluation(ActionEvaluationInvocationId)
Consumed
```

`begin_evaluation` checks `Open -> WaitingForEvaluation`. `resume_evaluation`
checks `WaitingForEvaluation -> Open`; ordinary resolution then performs the
existing checked `Open -> Consumed` transition. This preserves the M3
single-consumption law. A result publication may therefore contain the ordered
two-edge chain `Waiting -> Open -> Consumed`. A visible-input change instead
publishes `Waiting(old) -> Open -> Waiting(successor)`.

The corresponding action-specific invocation record has exactly these control
states:

```text
DispatchPending
ResultCaptured {
  result, effective, scheduler_key, capture_record
}
FallbackPending {
  cause, scheduler_key
}
Terminal(
  Applied { result, freshness }
  | Reinvoked { result, successor }
  | Failed { cause }
)
```

Creating a deferred invocation in `ActionReady` is one atomic runtime
transition: consume the ready work, move the opportunity to waiting, install
`DispatchPending` with its exact artifacts, and, for `FrontierBlocking`, store
the resulting frontier as `blocked_at_frontier`. Nothing is dispatch-visible
before that transition commits. There is no operational `Dispatched` state or
dispatch acknowledgement in M4.

Action-evaluation work is one closed scheduler family:

```text
ResultReady(invocation)
Fallback(invocation, cause)
```

Both carry an exact due moment and expected waiting version, sort between
`ActionReady` and `AttemptResolved`, and execute through ordinary `Fire`. They
always occur after the invocation-creating moment.

#### Closed identity and artifact boundary

The invocation record retains the opportunity, pre-wait and waiting versions,
predecessor/successor evaluation generation, selected action implementation,
fixed schema identities, exact request artifact and fingerprint, private
continuation and read-witness artifacts, admission mode, remaining
reinvocation budget, fixed fallback, and the creation moment and source cursor
as private provenance.
`blocked_at_frontier` exists only for `FrontierBlocking`.

Identity is closed and attempt-scoped:

```text
ActionEvaluationInvocationId =
  H(opportunity, actor-visible evaluation generation,
    selected policy semantics, ActionInputFingerprint)

ActionEvaluationRequestId =
  H(invocation, request schema, request artifact digest)

ActionEvaluationResultId =
  H(request, result schema, result artifact digest)
```

The invocation identity excludes authority revision, authority-record
identity, and private witness material so hidden-state changes cannot alter an
evaluator-visible identity. A distinct host `ActionEvaluationCaptureId`
identifies a result capture; its fingerprint covers invocation, request and
result identities, effective moment/mode, schema, and result artifact digest.
Repeating the same capture identity and fingerprint is idempotent across later
revisions; reusing that identity for different content fails. Command
`InputId` and action-evaluation capture identity are separate replay
namespaces.

Runtime cannot depend on `world-context` or `world-decision`. It stores four
role-specific, bounded canonical wrappers:

```text
dispatch-safe request artifact
captured result artifact
engine-private continuation artifact
engine-private read-witness artifact
```

Every wrapper carries a schema identity, byte length, and digest. Runtime
checks role, schema, bound, digest, and invocation bindings only.
`world-context` owns the canonical request-payload, private
candidate-resolution continuation, and combined positive read-witness codecs.
`world-decision` owns the canonical `ActionDecision` codec. The engine
composes and interprets those owner-local artifacts but does not define a
second encoding. Exact byte bounds are required because the destination set
is not bounded by the candidate limit; oversize artifacts produce a recorded
failure and later fallback, not an engine error with an implicit retry.

#### Public in-process boundary and simulation time

M4 adds only these engine-facing operations:

```text
RunAttempt::pending_action_evaluations(&self) -> sorted request views
RunAttempt::capture_action_evaluation_result(&mut self, capture)
  -> CaptureOutcome
```

A pending view contains only invocation/request/implementation/schema
identities, the typed actor-safe payload, and admission mode. It never exposes
revision, source cursor, witness, or private candidate-resolution material.
The capture operation accepts a typed `ActionDecision`; raw bytes and
authenticated transport belong to M6. Ingress is the closed sum
`Command | ActionEvaluationResultCapture`, not a command-envelope convention.

For `FrontierBlocking`, runtime fixes the capture's effective moment to
`blocked_at_frontier`. For `HostScheduled`, the caller supplies an effective
moment at or after the current admission frontier and strictly after the
creation moment. Capture atomically changes `DispatchPending` to
`ResultCaptured`, schedules `ResultReady`, and releases only that invocation's
blocker. After checking replay of an already accepted
`ActionEvaluationCaptureId`, a capture with a new ID for an unknown, terminal,
or late invocation fails without a record; reuse of a capture ID with a
different fingerprint also fails. Concurrent capture and management
serialize; the first accepted transition wins.

The frontier laws are:

- while a `FrontierBlocking` invocation is `DispatchPending`, no `Fire`,
  ordinary `Admit`, or admission-seal publication may produce a frontier
  greater than the minimum live `blocked_at_frontier`; the creating `Fire` may
  reach that frontier;
- capture or management releases its exact blocker, and multiple blockers use
  their minimum frontier;
- `HostScheduled` never blocks unrelated work, and arrival order never defines
  simulation order.

There is no automatic wall-clock timeout. A host records timeout explicitly
through management.

#### Containment witness and freshness

Action projection first reads the exact actor/source authority relation and
reads source membership only when that authority exists. Its positive
containment witness has two concrete sections:

- policy input: the exact authority-relation observation plus the complete,
  canonical direct-item identities returned by the source query, including an
  empty/absence token;
- bounded candidate execution validation: selected-item container including
  absence, source existence, actor/source authority, destination capacity
  including absence, and destination direct-item count/query token.

Definitions, opportunity, scope, destinations, and policy semantics are fixed
inputs, not witnessed reads. The global authority revision and private source
cursor are provenance only. No generic read graph, string dependency key, or
provider/unavailability abstraction is introduced.

At `ResultReady`, the engine decodes the original artifacts, validates the
result against the original request, and rebuilds action projection from the
current snapshot:

| Observation | Required outcome |
|---|---|
| Policy witness still valid | Reuse the result |
| Policy witness changed, visible payload byte-identical | Keep the invocation and privately rebind |
| Visible payload changed and budget remains | Record discard, increment actor-visible generation, and create a linked successor |
| Visible payload changed and budget is exhausted | Schedule the fixed fallback |
| Only execution-validation witness changed | Rebuild private resolution material and revalidate during lowering |
| Unknown candidate, wrong input fingerprint, or invalid typed result | Record failure and schedule the later fallback |

Destination capacity and occupancy affect private legality, never actor-visible
policy freshness. An unrelated actor/source change leaves the narrow witness
valid.

#### Management, fallback, and milestone boundary

Existing idempotent management ingress gains action-specific cancel, timeout,
and host-failure operations keyed by invocation. They may target
`DispatchPending` or `ResultCaptured`; management of a captured invocation
removes its exact `ResultReady` key, releases its blocker, and schedules
fallback at the preserved frontier. Cancellation means the invocation can no
longer affect simulation; it does not claim an external worker stopped, and a
late result is rejected.

If configured, cancellation, timeout, host failure, invalid result, exhausted
freshness reinvocation, and fallback-required management first enter
`FallbackPending`; a later recorded `Fallback` fire terminalizes the
invocation and publishes the checked
`Waiting -> Open -> Consumed(Failed)` chain. M4 has one fixed fallback:
`FinishFailedOnLaterWake`. It never fabricates `NoApplicableAction`.

`DeferredActionControlV1` is required exactly when the selected action profile
is deferred. It fixes admission mode, maximum visible-payload reinvocations,
per-role artifact byte bounds, and the fallback above. Inline profiles carry
`Disabled`, so irrelevant deferred settings cannot alter their semantic
identity.

M4 deliberately rejects a generic invocation framework, dispatch
acknowledgements, an outbox worker protocol, real transport, retry counters,
actor-local frontiers, and automatic wall-clock policy. Pending reads are
observation, not proof of send. M5 owns checkpoint restoration and
evaluator-free replay; M6 owns CLI/MCP/player/AI adapters, authenticated raw
transport, dispatch delivery, and retry policy. The M4 record is nevertheless
self-contained so those later layers do not require a semantic redesign.

### No synthetic unavailability

`Complete(empty)` remains a valid projection result. `Unavailable` is added
only when a real production projection depends on data that can genuinely be
missing or temporarily inaccessible and a current lifecycle consumes that
distinction.

The in-memory M4 projection path is total over checked inputs. A validation
scenario does not justify inventing a provider or failure flag. Scenario 4
remains unassigned unless a real provider appears.

### One honest process vertical

M4 introduces process machinery only through one separately authored
time-bearing relocation/travel mechanic:

```text
grounded start-relocation action
  -> runtime-owned ProcessInstance with start, destination, duration, and generation
  -> scheduled process wake at a later SimMoment
  -> runtime progress or completion transition
  -> accepted domain relocation at completion
  -> domain event and ordinary later reaction routing
```

The existing immediate item-transfer semantic remains immediate. Relocation
has a distinct definition and action contract; it cannot masquerade as a
delayed version of the old action.

Grounded pause and resume actions control the relocation process. They enter
the normal M3 candidate-ID, lowering, command, and runtime-validation path.
Process progress and completion remain runtime authority. An activity may
await or observe the process only through explicit subscription/monitoring or
modeled evidence.

The first duration may be a simple canonical authored value. Skill,
equipment, condition, terrain, and environment modifiers remain future
domain logic behind the same time-bearing contract.

#### M4 relocation contract

The accepted domain slice is deliberately smaller than a spatial framework:

```text
DirectedRoute
  source
  destination
  positive duration

ActorPosition
  At(place)
  InTransit { source, destination }
```

Routes are directed and unique by `(source, destination)`. Accepted domain
state records only physical position. It contains neither process identity nor
progress.

Runtime control owns one concrete relocation process:

```text
RelocationProcess
  actor
  source
  destination
  total duration
  elapsed duration
  version
  wake generation
  Active { active since, due }
    | Paused
    | Completed { completed at }
```

Starting requires `At(source)`, a matching route, no live relocation for the
actor, and positive elapsed simulation time. It atomically changes position
to `InTransit`, creates the process, and schedules its generation-guarded
completion wake. Pausing before the due time accumulates exactly the current
active segment and advances the generation. Resuming schedules a new due time
from the remaining duration and advances the generation again. Completion
with the expected generation atomically installs `At(destination)` and emits
the typed completion event. A wake for an older generation is consumed as
obsolete and cannot change progress.

The authored action family is the closed M4 set:

```text
StartRelocation(actor, source, destination)
PauseRelocation(actor, source, destination)
ResumeRelocation(actor, source, destination)
```

`ActionScope` gains concrete relocation start and control forms rather than a
generic effect request. Actor-facing candidates contain actor-safe source and
destination references only. Exact process identity and expected generation
for pause or resume remain in engine-private lowering material and runtime
validation; they never enter `ActionPolicy` or `ActivityController`.

An activity may retain semantic method state such as source, destination, and
whether it is awaiting arrival. It does not own process identity, elapsed
progress, due time, or wake generation. Waiting or terminating the activity
therefore has no implicit process effect. Process meaning returns to
the epistemic partition through modeled evidence. M4 does not yet appraise an
arrival into an activity transition, so the travel activity remains
`Waiting/AwaitArrival` after the process completes. Changing process state
still requires a grounded action or an internal runtime wake.

M4 explicitly defers generic process definitions or DSLs, cancellation and
its physical placement policy, continuous or interpolated route position,
terrain and skill duration modifiers, reservations, and a general spatial
model. Those additions must preserve this activity/process separation and the
same authority path.

## Formal commitments

### State relation

For each lifecycle family `K`, evaluation is a staged transducer:

```text
prepareK(Γ, snapshot(Σr), scheduled input)
  -> private input envelope + actor-safe payload

evaluateK(selected implementation, actor-safe payload)
  -> concrete resultK

proposeK(private envelope, resultK)
  -> typed proposalK

sealK(Γ, Σr, proposalK, expected versions)
  -> AuthorityRecord | rejection

apply(Σr, AuthorityRecord)
  -> Σr+1 + later ScheduledWork
```

Only `seal` and canonical `apply` can accept state. The evaluator neither
reads unrestricted `Σ` nor emits a runtime command except through the
existing private action lowering boundary.

### Persistent agency

The required relations are:

```text
supports    EvidenceRecord -> Belief
owns        Activity -> Intent
focuses     Actor -> Activity?                  baseline
sponsors    ActionOpportunity -> Activity | ActorReaction
origin      ProcessInstance -> AttemptRecord
awaits      Activity <-> ProcessInstance
```

Minimum invariants are:

- every live activity owns exactly one nonterminal intent;
- a terminal intent has no live dependent activity;
- a suspended intent has no active foreground activity;
- the baseline has at most one open or evaluation-pending action opportunity
  per actor;
- every opportunity has exactly one explicit sponsor;
- every accepted intent, activity, belief, evidence, process, opportunity, and
  invocation transition checks its expected version or generation;
- process state changes only through runtime authority;
- activity termination never implicitly terminates a process;
- a terminal invocation cannot be reopened by duplicate or late results.

### Action continuation

`ActionSponsor` gains an activity form only when an activity can produce and
consume it:

```text
ActivitySponsor
  activity identity
  expected activity version
  actor-visible opportunity generation
```

The containment activity also narrows its directive to the intended object.
W4 replaces the current source-and-destinations-only
`ContainmentInteractionScope` with a canonical nonempty exact-item allowlist:

```text
ContainmentInteractionScope
  source
  destinations
  items
  candidate limit
```

The context projector may ground only supplied items. This prevents an
activity pursuing one desired condition from selecting another item that
happens to share the source container. The changed canonical scope and action
opportunity identity receive new schema/domain versions; no compatibility
alias preserves the broader pre-W4 meaning.

An `AttemptResolved` delivery stays neutral. Engine-private preparation
reattaches its consumed opportunity sponsor and rich authoritative outcome.
It schedules or directly proposes one version-checked
`ActivityAdvanceNeeded`; the controller still receives only the actor-safe
cause and separately projected accepted knowledge.

An activity transition may:

- open exactly one successor opportunity;
- wait until an explicit simulation moment;
- suspend;
- complete;
- fail or cancel; or
- request intent reconsideration.

These are concrete activity results, not a shared public disposition algebra.

### Actor-relative noninterference

For states indistinguishable to actor `a` in lifecycle role `K`:

```text
Σ1 ≈a,K Σ2
```

M4 must preserve:

```text
PolicyPayloadK(Σ1) = PolicyPayloadK(Σ2)
actor-visible identities and ordering are equal
logical invocation and dispatch traces are equal
actor-facing wake presence, timing, and generation are equal
```

Raw revision, raw `SimMoment`, authority-record identity, dependency witness,
private candidate table, process identity derived from hidden authority, and
rich attempt resolution remain engine-private.

### Progress and causal cycles

Every accepted successor work item advances microstep. M4 preserves and tests
the existing same-tick budget and management escape path for every new work
lane, but does not invent an authored self-generating lifecycle or relocation
cycle merely to exercise it. The first real authored cycle must use the same
budget. M4 adds no recursive runner, host-stack recursion guard, or second
cycle budget.

## Reference vertical slices

### Evidence to action

The first conformance fixture uses existing containment semantics to prove the
whole deterministic spine:

1. a committed transfer or modeled failed interaction emits a typed domain
   occurrence;
2. post-commit routing creates actor-addressed evidence deliveries;
3. evidence assimilation creates provenance-bearing `EvidenceRecord` values
   and updates actor-keyed `Belief` values;
4. a materially changed appraisal schedules intent review;
5. the deterministic intent baseline selects one supplied grounded intent
   candidate;
6. intent adoption schedules activity initialization;
7. the activity baseline creates versioned activity state and one
   activity-sponsored containment-transfer opportunity;
8. the existing M3 policy selects a supplied candidate ID and runtime
   validates the command;
9. neutral attempt resolution advances the same activity rather than
   regenerating the intent.

The fixture must include one false or stale belief so hidden authoritative
truth is not accidentally substituted for actor knowledge.

### Minimal actor

The same path works with optional social interpretation, planning, rich
appraisal, and external evaluation disabled. A deterministic rule profile is
a complete production semantics binding, not a testing shortcut.

### Social interpretation

Social state is added only with one real typed interpretation in the
reference fixture. The preferred narrow slice uses explicit giver, recipient,
and witness roles for one transfer occurrence so recipient and witness may
accept different hypotheses while the physical record contains neither.

If the required role relation and authored meaning cannot be represented
without inventing a generic social ontology, implementation stops at the
epistemic/appraisal boundary and records an architecture decision. M4 cannot
claim the social portion of scenario 11 until a concrete slice exists; it may
not satisfy the gate with labels attached directly to the physical effect.

### Relocation process

Combined public-facade and focused runtime conformance proves:

1. a grounded start action changes `At(source)` to `InTransit`, creates one
   active process, consumes positive virtual time, and completes at the
   directed route's due moment;
2. a grounded pause records elapsed progress, a grounded resume schedules the
   remaining duration, the superseded wake is harmless, and arrival occurs
   exactly once;
3. a waiting activity does not stop its live process, terminal activity
   completion or failure does not alter that process, and process completion
   does not mutate the waiting activity;
4. paired actor-indistinguishable inputs produce identical action candidates
   even when hidden route or process legality makes runtime accept one command
   and reject the other; and
5. immediate containment transfer remains immediate and creates no process.

## Crate ownership

### `world-core`

- only identities, time, canonical encoding, and checked scalars that are
  genuinely shared across multiple lower packages;
- no behavior, registry, or lifecycle runner.

Lifecycle, process, invocation, evidence, belief, intent, and activity
identities remain beside their owning protocols unless a second lower-package
consumer proves that moving one into `world-core` is necessary.

### `world-defs`

- checked concrete declaration types required by the selected intent,
  activity, social, and relocation slices;
- exact semantic-interface requirements for new authored action/process
  definitions;
- no speculative DSL or general planner representation.

### `world-model`

- aggregate `AcceptedState` and typed domain, epistemic, social, and agency
  partitions;
- immutable checked `EvidenceRecord`, `Belief`, `Intent`, and `Activity`
  values;
- concrete relocation-process protocol values that do not grant mutation
  authority;
- narrow read-only query views.

### `world-context`

- concrete actor-relative builders for evidence, appraisal, intent, activity,
  social interpretation when enabled, action freshness rebuild, and
  relocation actions;
- engine-private dependency tables, candidate-resolution tables, and rebuild
  products;
- no runtime, engine, persistence, or host dependency.

### `world-decision`

- one concrete object-safe port per implemented lifecycle;
- deterministic evidence, appraisal, intent, activity, and action baselines;
- optional social evaluator only with the selected concrete slice;
- no authority, unrestricted snapshot access, transport, or generic
  lifecycle framework.

### `world-runtime`

- partition-integrity validation and canonical state application;
- typed scheduler work, delivery preparation, proposals, seal checks, and
  authority records;
- concrete action-opportunity, process, pending-action-evaluation,
  coalescing, cancellation, timeout, and fallback state machines;
- result-capture ingress and idempotency;
- no dependency on `world-context` or `world-decision`.

### `world-engine`

- exact lifecycle implementation registration, resolution, and binding;
- lifecycle-specific coordinators and post-commit routing;
- actor-safe/private envelope separation;
- deferred request encoding, result decoding, witness validation, projection
  rebuild, private rebind, and selected-ID lowering;
- no mutation outside runtime proposals.

### `world-standard-runtime`

- concrete trusted transfer and relocation operation implementations bound to
  their authored semantic interfaces;
- no product adapter or generic game framework.

Deterministic evidence, appraisal, intent, activity, and action baselines live
in `world-decision`; relocation-process transitions and action-evaluation
artifacts remain owned by `world-runtime`.

### `world-conformance`

- public-facade vertical scenarios and paired noninterference fixtures;
- no privileged mutation or direct internal state setup after an equivalent
  public setup path exists.

## Work packages

The dependency order is:

```text
W1 semantic closure
  -> W2 accepted-state partition
  -> W3 deterministic lifecycle substrate and evidence spine
  -> W4 persistent intent/activity continuation
  -> W5 real relocation process
  -> W6 deferred action evaluation
  -> W7 conformance, deletion, and exit review
```

W3 and local model work for W4 may be prepared in parallel only after W1 and
W2 contracts are fixed. W6 does not begin before the synchronous deterministic
spine passes.

### W1: Close lifecycle implementation identity

- replace the singleton lifecycle marker with exact per-port requirements;
- include enabled implementation IDs and state-schema IDs in
  `SemanticImplementationSet` and the normalized manifest;
- make execution resolution bind installed implementations by those exact
  identities;
- remove arbitrary post-resolution controller substitution;
- preserve explicit `Disabled` for optional ports;
- add canonical vectors and tests proving a behavior-affecting implementation
  change changes execution semantics identity;
- prove missing, unknown, duplicate, and schema-incompatible bindings fail
  before session creation.

W1 is complete only when every evaluator capable of changing a logical result
is part of `Γ`.

### W2: Partition accepted state

- rename the current physical state to the domain partition;
- introduce the aggregate `AcceptedState` with domain, epistemic, social, and
  agency partitions;
- update initial roots, snapshots, canonical encoding, digests, authority
  application, queries, and tests;
- establish cross-partition integrity checks without a generic state bag;
- keep action opportunities, processes, pending invocations, and scheduler
  control outside accepted truth;
- add canonical round-trip, insertion-order, digest, and invalid-reference
  tests;
- remove the old physical-only accepted-state constructors and aliases once
  callers migrate.

### W3: Deterministic lifecycle substrate and evidence spine

- replace consume-without-follow-up routing with a pure typed
  `PostCommitRouter`;
- retain a self-contained reaction envelope so dispatch never depends on
  compactable history;
- add actor-addressed evidence delivery and concrete coalescing generations;
- add evidence assimilation with acceptance/rejection, provenance, and belief
  addition, supersession, or retraction;
- add actor-relative deterministic appraisal and material-change detection;
- schedule every successor through the existing prepared-work/proposal/seal
  pipeline at a later microstep;
- prove work dirtied during evaluation is not lost;
- prove equal appraisal does not cause unnecessary intent churn;
- add the false-belief evidence/appraisal completion of scenario 2;
- add a narrow social interpretation only if the concrete scenario-11 role
  fixture is selected and validated.

W3 introduces no universal context, generic lifecycle result, or fake
unavailable provider.

### W4: Persistent intent, activity, and action continuation

- add immutable versioned intent and activity records to accepted agency
  state;
- add grounded intent candidates built from actor-safe accepted knowledge;
- implement deterministic intent adoption, reconsideration, achievement,
  abandonment, suspension, and failure transitions needed by the fixture;
- implement `ActivityController.initialize` and `advance` with concrete input
  and result types;
- extend `ActionSponsor` with activity identity and expected version;
- let initialization atomically create one activity and one sponsored open
  action opportunity;
- route that opportunity through the unchanged M3 candidate projection,
  action policy, private lowering, and runtime validation;
- transform neutral `AttemptResolved` into later version-checked activity
  advancement using engine-private resolution material;
- prove bounded recovery can open at most one causally linked successor;
- prove an action rejection does not regenerate or implicitly terminate the
  intent;
- complete scenario 12 with optional higher cognition disabled.

### W5: Real relocation/travel process

- seed one checked travel intent, focused activity, and grounded Start
  opportunity at the execution origin without claiming a general travel
  initializer;
- add checked directed routes and `At | InTransit` actor position;
- define the closed start, pause, and resume relocation action family;
- add one authoritative relocation process with exact duration, elapsed
  progress, due moment, version, and wake generation;
- schedule completion through the ordinary runtime work path;
- make pause/resume preserve elapsed progress and invalidate older wakes;
- apply departure and arrival only through validated authority transitions;
- emit typed relocation events and reenter the ordinary post-commit evidence
  spine;
- keep exact process control bindings private while grounding pause and resume
  through the M3 action boundary;
- prove waiting or terminating an activity and controlling a process are
  distinct transitions;
- prove activity termination does not alter process state;
- prove a paused and resumed process completes once without double-counting
  progress;
- retain the M2 same-tick causal budget and management escape for future
  authored lifecycle/process cycles;
- prove the scenario-3 foundations: authoritative travel, grounded
  pause/resume, and separation of activity state from process state.

No generic process DSL, cancellation rule, interpolated position, service
framework, or silent semantic change to the existing immediate transfer
action is allowed.

### W6: Retained and deferred action evaluation

- add the exact opportunity and action-invocation control states specified
  above, with checked two-edge resumption before ordinary consumption;
- atomically retain the bounded request, private continuation, and positive
  witness before a request becomes visible, then retain the bounded result at
  capture;
- expose sorted typed pending views and idempotent typed result capture through
  the public engine facade;
- enforce the minimum-blocker frontier law for `FrontierBlocking` and explicit
  serialized effective time for nonblocking `HostScheduled`;
- classify current, private-rebind, visible-change reinvocation,
  private-legality revalidation, and invalid-result outcomes from the narrow
  containment witness;
- add idempotent cancel, timeout, and host-failure management, one later-wake
  failure fallback, blocker release, and terminal late-result rejection;
- keep action-policy output limited to `Select | NoApplicableAction` and prove
  an external result can neither submit a command nor select an unknown
  candidate;
- prove result/fallback work executes in a later microstep and blocking
  invocations cannot be skipped by admission sealing;
- complete M4's semantic portions of scenarios 5 and 16 with an in-process
  deterministic deferred evaluator.

W6 does not implement checkpoint restoration/replay, dispatch delivery, real
transport, authentication, or retry policy.

### W7: Conformance, simplification, and exit review

- run the M4 validation-scenario allocation through the public engine facade;
- add same-moment permutation and cross-lifecycle causal-order tests;
- verify crate dependency, privacy, and authority boundaries;
- remove superseded routing, state shapes, controller installation paths,
  reaction-only sponsorship assumptions, and action-only exhaustive matches;
- remove any wrapper, broad trait, generic disposition, or placeholder type
  left without two concrete semantically identical uses;
- retain only the final `ExecutionConfigArtifactV3` shape, whose schema-v3
  identity includes the deferred-action control contract, with no
  compatibility alias for either superseded configuration shape;
- reconcile target documents with the implemented names and actual ownership;
- run the full locked workspace gate;
- write the M4 exit review with exact code and test evidence;
- detail M5 only after M4 closes.

## Deletion scope

After each replacement is proven:

- remove pre-M4 lifecycle-profile shapes that fail to identify selected
  lifecycle behavior;
- remove arbitrary controller installation that can alter resolved semantics;
- remove the physical-only meaning and constructors of `AcceptedState`;
- remove `ConsumeWithoutFollowup` once typed post-commit routing is live;
- remove reaction-only assumptions from `ActionSponsor`;
- remove the consume-only `AttemptResolved` path once activity advancement is
  its real successor;
- remove action-only scheduler, delivery, authority-record, and proposal
  matches that bypass explicit handling of new typed work;
- remove duplicate private continuation or generation stores;
- remove any temporary direct evaluator callback or command path used during
  development;
- remove placeholder `Unavailable`, social, process, planner, or generic
  lifecycle abstractions with no real producer and consumer.

No compatibility layer preserves the old model. Deletion follows replacement
behavior and focused tests so the repository remains reviewable.

## Decision triggers

Stop and record an architecture decision before:

- changing crate dependency direction or moving mutation authority out of
  runtime;
- adding lifecycle state before its implementation and schema identities are
  in `Γ`;
- treating implementation selection as host configuration outside normalized
  execution semantics;
- putting an action opportunity, process, or pending invocation into accepted
  domain/epistemic/social/agency truth;
- allowing policy to read unrestricted snapshots, raw revision, raw moment,
  dependency witnesses, private process state, or rich attempt resolution;
- adding a generic lifecycle runner, context bag, disposition, planner port,
  provider registry, or state map;
- extracting shared lifecycle protocol machinery before two concrete
  lifecycles prove identical transition semantics;
- adding `Unavailable` without a real partial provider and current consumer;
- selecting a social slice that requires a general social ontology;
- weakening scenario 11 instead of either implementing one concrete social
  interpretation or amending its milestone allocation;
- changing existing immediate transfer semantics to satisfy the process gate;
- introducing a second process family before the relocation vertical closes;
- allowing evaluator output or external callbacks to issue runtime commands;
- using global revision as the sole retained-result freshness check;
- treating transport delivery as result capture or cancellation as proof that
  a worker stopped;
- moving checkpoint restoration/replay into M4 or product transport into M4;
- changing canonical identities without explicit version/domain changes.

Local concrete names, module splits, private algorithms, and bounded error
vocabularies may change without a new architecture decision when ownership
and observable semantics stay fixed.

## Acceptance gates

### Semantic closure

- every enabled lifecycle implementation is selected by the resolved profile
  and included in the normalized execution-semantics identity;
- changing a selected implementation or persistent-state schema changes that
  identity;
- unrelated installed implementations do not;
- no engine builder call can silently replace a selected implementation;
- missing or incompatible bindings fail before session creation.

### State ownership and integrity

- accepted state has explicit domain, epistemic, social, and agency
  partitions;
- canonical encoding and digest cover all partitions exactly once;
- every accepted cross-reference is validated;
- action opportunities, processes, pending invocations, and coalescing state
  remain typed runtime control;
- derived appraisal is not silently promoted to accepted truth;
- invalid cross-partition or stale-version proposals publish no mutation.

### Lifecycle separation

- every lifecycle uses scheduled input, preparation, proposal, seal, apply,
  and later work;
- one transition cannot recursively execute the full stack;
- independent lifecycle keys and actors do not share a global cognition
  cadence;
- coalescing cannot lose a cause committed during evaluation;
- causal links and generations are canonical and traceable;
- same-moment insertion and evaluation permutations produce the same result.

### Evidence and agency

- domain outcomes do not directly write actor belief, appraisal, intent, or
  social meaning;
- evidence carries actor, source, semantic content, and provenance;
- beliefs may remain false until modeled evidence changes them;
- appraisal reads accepted actor-relative knowledge, not hidden truth;
- intent persists until an explicit accepted transition;
- activity persists across individual action attempts;
- one activity-sponsored opportunity uses exactly the M3 action path;
- neutral attempt outcome shape does not leak rich runtime resolution;
- the minimal deterministic actor works without planning, social
  interpretation, rich appraisal, or external evaluation.

### Process

- relocation consumes positive simulation time;
- directed route duration and `At | InTransit` position are accepted domain
  truth while elapsed progress and wake generation remain runtime control;
- domain relocation occurs only through runtime authority;
- process progress is neither activity state nor planner state;
- waiting, completing, or failing an activity does not implicitly alter the
  process, and process completion does not mutate the waiting activity;
- grounded pause and resume use the ordinary action boundary without exposing
  process identity or generation to actor-facing controllers;
- stale process wakes cannot duplicate progress;
- pause followed by resume preserves elapsed progress and completes exactly
  once;
- existing immediate containment transfer creates no process and remains
  immediate;
- the existing same-tick budget and management escape cover every M4 work
  lane; an authored lifecycle/process cycle remains unclaimed until one
  exists.

### Deferred evaluation

- `DispatchPending` and the exact canonical request exist before dispatch;
- result capture is serialized, idempotent, and distinct from delivery;
- a result cannot join its invoking moment;
- duplicate and late results cannot reopen terminal state;
- actor-visible payload and logical dispatch behavior satisfy
  noninterference;
- byte-identical rebuilt payload reuses or privately rebinds without a new
  logical invocation;
- visible payload change records discard and a linked successor invocation;
- hidden-legality-only change causes private revalidation, not policy
  reinvocation;
- absence-sensitive dependencies are positively witnessed;
- `FrontierBlocking` blocks the frontier and `HostScheduled` uses explicit
  simulation-time ingress;
- action-policy output remains `Select | NoApplicableAction`;
- fallback is a later recorded invocation, never an implicit callback;
- admission sealing cannot skip unresolved blocking work.

### Validation-scenario allocation

- Scenario 2: modeled rejection observation, evidence, belief, and later
  appraisal complete the M3 false-belief slice.
- Scenario 3: an origin-seeded persistent travel intent, authoritative
  relocation, activity/process separation, and grounded pause/resume pass.
  General travel initialization, arrival appraisal, and threat-driven
  interruption remain unclaimed until concrete semantic producers exist.
- Scenario 5: M4's semantic nonblocking invocation, capture, freshness,
  cancellation, and fallback pass without claiming M5 replay or M6 transport.
- Scenario 10: the kernel's causal budget and management escape remain proven,
  but M4 does not claim an authored self-generating rule cycle.
- Scenario 11: physical outcome, observer-specific evidence, and appraisal are
  distinct and proven. Social interpretation remains disabled and unclaimed
  until one concrete social semantic slice exists.
- Scenario 12: the deterministic minimal actor completes the entire path.
- Scenario 16: retained evaluation proves positive freshness, private rebind,
  visible reinvocation, and hidden-legality-only revalidation.
- Scenario 4 remains unassigned unless a genuine partial provider with a
  production consumer appears.

### Quality and verification

```text
cargo fmt --all --check
cargo check --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
git diff --check
```

Focused tests also cover canonical vectors, state-machine transitions,
expected-version failures, duplicate ingress, terminal one-shot behavior,
coalescing generations, actor-relative paired states, dependency allowlists,
and compile-time crate privacy.

## Completion evidence

### Entry review

Recorded in this plan:

- current M3 implementation and public action path inspected;
- target formal, system, code, cognition, roadmap, and scenario contracts
  cross-checked;
- M4 research implications reconciled with the current code;
- lifecycle semantic identity and accepted-state partitioning identified as
  mandatory first corrections;
- deterministic causal, process, and deferred-evaluation verticals selected;
- milestone boundary with M5 replay and M6 transport fixed.

### Implementation and exit review

Accepted in the [M4 exit review](milestone-04-exit-review.md).

The completed implementation includes:

- exact lifecycle implementation and state-schema closure in `Γ`;
- accepted domain, epistemic, social, and agency partitions;
- separately scheduled evidence, appraisal, intent, activity, action, process,
  and deferred-evaluation lifecycles;
- a closed `ContainmentTransfer | Travel` activity-state sum;
- an origin-seeded, activity-driven Start/Pause/Resume relocation cycle with
  stale-wake protection and activity/process separation;
- retained and deferred action evaluation with positive witnesses, private
  rebind, visible reinvocation, cancellation, and later fallback;
- exact origin validation for focused activity sponsorship and retained
  method opening;
- public causal-order, actor-relative, relocation, deferred-evaluation,
  scheduler-permutation, authority-vector, and dependency-boundary evidence.

The complete locked workspace gate passed on 2026-07-28.

## Next milestone handoff

M5 should receive:

- normalized execution semantics that include every active lifecycle
  implementation and state schema;
- canonically encoded accepted-state partitions;
- persistent intent, activity, evidence, belief, process, opportunity, and
  pending-action-evaluation records;
- exact typed scheduler and runtime-control state for all M4 lifecycles;
- self-contained post-commit reaction work;
- captured deferred requests and results that never require service
  reinvocation to interpret;
- deterministic authority records and causal links suitable for checkpoint
  restoration and verification;
- no transport, adapter, or legacy execution path embedded in the semantic
  core.

M5 then makes that complete state restorable and replay-verifiable. M6 adds
CLI, MCP, player, AI-agent, and external evaluator adapters without changing
the M4 semantic boundaries.
