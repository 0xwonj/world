# Target Architecture Validation Scenarios

## Purpose

These scenarios test whether the target boundaries produce coherent behavior
under conflict, uncertainty, interruption, nondeterminism, persistence, and
scale.

They are architecture acceptance cases, not complete gameplay specifications.
Each should eventually exist as a small deterministic integration scenario
with causal-history and trace assertions.

Scenario names and setup nouns are fixtures. A milestone plan may replace
them with an equally small deterministic fixture, but the required flow,
owner boundaries, and assertions assigned to that milestone are mandatory.
Documenting a later-scope justification is valid only for an optional
capability or a contract that has no production consumer; it cannot waive a
roadmap-completion gate.

## 1. Two actors contend for one resource

### Setup

Two actors become ready at the same `SimMoment`. Their private invocation
envelopes are bound to revision `R`; both projection-safe policy payloads
contain a grounded `take(item)` candidate.
A second subcase uses two individually legal, disjoint writes whose combination
would violate a declared global capacity invariant.

### Required flow

1. Both action policies evaluate immutable projection-safe payloads whose
   private envelopes are based on `R`.
2. Both select their own grounded candidate ID.
3. The coordinator lowers two concrete commands.
4. Runtime detects that both require the same exclusive item.
5. A non-conflicting third command may also be present in the same batch.
6. Preparation declares read, write, resource, and global-invariant footprints.
7. A named, total domain conflict policy resolves the contention independent of
   proposal order.
8. The combined accepted set is rechecked against global invariants.
9. In the exclusive-resource subcase, one `MomentBatchRecord` publishes the
   `SameMomentCommitBatch`, moves revision `R` to `R+1`, accepts the winner and
   non-conflicting command, and rejects the loser.
10. In the combined-invariant subcase, the record publishes a deterministically
    refined valid subset, which may be the mandatory rejection-only fallback,
    and records every rejected outcome.
11. Every command receives one nested `AttemptRecord`.
12. Retry work and a self-contained `ReactionEnvelope`, when nonempty, are
    included atomically.

### Assertions

- insertion order and worker completion order do not decide the winner;
- the conflict policy and tie-break version appear in history;
- the item exists in exactly one place after commit;
- no losing event is published as if its action succeeded;
- the contender set, resolver version, rejection, accepted commits, trigger
  consumption, and scheduler consequences advance atomically;
- individually valid proposals that fail a combined invariant are
  deterministically refined, with a rejection-only total fallback and any
  configured non-running mode recorded in the same moment rather than an
  endless retry;
- non-conflicting same-moment work does not become stale after the first
  accepted subtransaction;
- the resolver terminates and is permutation-invariant and total;
- only the outer moment record receives the run-record sequence and hash-chain
  link; inner IDs are canonically derived without entering that preimage;
- rerunning with a different worker count produces the same logical result.

## 2. An actor acts on a false belief

Milestone ownership is intentionally split. M3 owns steps 1–5 and the paired
noninterference obligations through the neutral wake. M4 owns steps 6–8, where
modeled observation, accepted evidence, belief revision, and later appraisal
may finally distinguish the actor's view.

### Setup

The door is authoritatively locked. The actor's accepted belief says it is
unlocked.

### Required flow

1. `ActionContextPayload` reports the actor's permitted view, not hidden lock
   truth.
2. Candidate generation includes a grounded `open(door)` attempt.
3. The action policy selects it.
4. Runtime checks the current authoritative requirement and rejects or commits
   a modeled failed attempt according to the action contract.
5. Runtime retains the rich resolution privately and emits only
   `AttemptResolved(ActionOpportunityId)` to actor-facing lifecycle work; its
   visible identity does not depend on the rejection class or record hash.
6. Runtime rejection does not directly install actor knowledge. Only modeled
   observable failure information enters a nonempty `ReactionEnvelope`, then
   observation projection produces an `EvidenceDelivery` at a later microstep.
7. Evidence assimilation atomically accepts or rejects that delivery and may
   create an `EvidenceRecord` and revise the `Belief`.
8. Appraisal and activity/action wakes occur only if their dependencies changed.

### Assertions

- candidate omission or actor-safe metadata do not leak the lock;
- two authoritative states that differ only in actor-hidden lock details
  produce byte-identical policy payloads, actor-safe candidate IDs, ordering,
  and fingerprints even if their global revisions or hidden-only state differ;
- those paired states produce the same logical policy invocation/dispatch
  count, effective simulation timing, and generation;
- their terminal attempt resolutions schedule the same neutral-wake presence,
  effective moment/microstep, generation, and visible cause before separately
  accepted evidence differs;
- runtime legality is independent of perceived availability;
- raw acceptance/rejection, reason, retryability, revision, and
  attempt/commit/process references never enter `ActivityController`,
  `ActionPolicy`, or `ActorViewK`;
- global revision, raw `SimMoment`, raw trigger/cause identity,
  authority-derived IDs, and private build material remain outside every
  actor-facing policy input;
- M3 completes projection, decision, and lowering inside one reserved
  `ActionReady` step; the lowered command carries no stale projection claim,
  and runtime independently reevaluates current legality;
- a failed request cannot partially move the door;
- any actor-facing successor opportunity follows profile-fixed budgets and
  actor-relative context/evidence, never authoritative retryability;
- the actor may select an information-gathering or alternate candidate after
  its actor-relative inputs justify it;
- any redecision caused by accepted evidence occurs in a later microstep,
  although it may remain in the same simulation tick's causal wave;
- one `Fire` transition cannot recursively perform observation, assimilation,
  appraisal, and redecision.

## 3. Long activity and process are interrupted

### Setup

An actor has a persistent intent to reach another settlement. Its travel
activity has started an authoritative travel process scheduled to complete
later. A threat becomes observable midway.

### Required flow

1. The travel process advances under runtime control.
2. The threat `MomentBatchRecord` contains a nonempty `ReactionEnvelope` and
   atomically schedules a later `PostCommitDispatch`.
3. Dispatch consumption proposes observer-specific evidence and dependency
   wakes. Separate later moment records accept evidence and, when accepted
   dependencies changed, schedule appraisal and activity work.
4. A material urgency change proposes interruption.
5. Intent/focus policy keeps the travel intent but version-transitions and
   suspends or preempts its activity.
6. An accepted agency transition opens a new actor-reaction or focal-activity
   `ActionOpportunity`, with one explicit sponsor and state `Open`.
7. If process control is required, the actor selects a grounded control action
   whose runtime effect pauses, interrupts, or checkpoints travel.
8. When safe, the activity resumes or starts a replacement travel process from
   accepted progress.

### Assertions

- the intent is not regenerated merely because the next action changed;
- activity lifecycle transitions are recorded in authority history and linked
  explanatory trace;
- every activity state change checks the expected activity version;
- process progress is runtime truth rather than opaque planner state;
- raw `ProcessInstance`, `Progress`/`Completed` causes, and outcome references
  remain engine-private;
- any direct activity monitor wake uses a predeclared actor-visible
  `MonitorId`, predicate, cadence, and generation; otherwise process meaning
  reaches the controller only through projected evidence;
- monitor presence, timing, and visible cause satisfy actor-relative
  noninterference;
- obsolete process wakeups are invalidated by generation;
- no travel progress is counted twice after resumption;
- the reason for interruption is traceable from evidence to appraisal to
  activity transition.

## 4. A required context projection is unavailable

M3 establishes successful complete-empty semantics without inventing a
synthetic production provider. This scenario becomes an implementation gate
only when a real partial projection dependency supplies both an honest
producer and a production consumer of typed unavailability. It is not an M4
gate merely to force creation of that abstraction.

### Setup

The required projection provider is installed and activated, but the
projection cannot be produced for this actor and snapshot because a declared
runtime data dependency is unavailable.

### Required flow

1. The context build report marks the projection `Unavailable`.
2. It does not substitute an empty affordance set.
3. The coordinator follows the profile's explicit incomplete-context policy:
   abstain with `MissingProjectionKey`, use a declared fallback, or fail the
   lifecycle.
4. Any fallback is a separately traced invocation.

### Assertions

- the evaluator is never told the incomplete input is complete;
- `Complete(empty)` remains distinguishable and may legitimately yield
  `NoApplicableAction`;
- diagnostics and provider provenance survive into the trace;
- unavailable input cannot accidentally satisfy a declared requirement;
- an abstain, wait, fallback, or failure disposition that changes later
  behavior is proposed into typed agency/runtime-control state rather than
  remaining only in a trace.

## 5. A nonblocking external evaluator returns late

Milestone ownership is split. M4 owns the authoritative deferred-invocation,
captured-result ingress, freshness, rebind/discard, cancellation, and fallback
protocol below. M5 owns restoration and replay without service reinvocation.
M6 owns authenticated product transport and CLI/MCP/player/AI adapters. M4 may
exercise the protocol with a deterministic deferred evaluator and no network
service.

### Setup

A deferred action evaluator receives a projection-safe payload whose private
invocation envelope is bound to revision `R`, under the explicit nonblocking
`HostScheduled` policy. Before its response is available, another commit
changes a relevant target and the session reaches revision `R+1`.

### Required flow

1. The installed action execution is `DeferredCaptured`; the invoking moment
   atomically records the private envelope, exact policy payload, durable
   dispatch state, continuation, and `WaitingForEvaluation` opportunity
   without first calling an evaluator.
2. Only the policy payload leaves the process, and only after that state is
   durable.
3. `HostScheduled` permits unrelated authoritative work to advance the session.
4. The response is accepted at a serialized ingress barrier into an
   `ActionEvaluationAdmissionRecord` with a distinct capture identity,
   explicit effective `SimMoment`, and delivery trigger no earlier than the
   `AdmissionFrontier`.
5. Wall-clock completion order does not place it directly into simulation.
6. At a later delivery microstep, the coordinator checks the exact dependency
   witness. The response cannot retroactively join the invoking moment.
7. The trusted coordinator's stale-result protocol discards the result,
   privately rebinds/revalidates it, or creates a new logical invocation only
   when the policy payload or explicit evaluator binding changed according to
   recorded configuration.
8. A rule fallback, if used, appears as a separate lifecycle invocation and
   any command it submits receives an `AttemptRecord`.

### Assertions

- the external evaluator cannot submit a runtime command itself;
- it can select only a candidate from its original input;
- stale result handling is visible;
- M5 restoration replay uses the captured result and never calls the service;
- host timeout, cancellation, or failure uses an idempotent `Manage`
  transition that captures and applies the disposition atomically rather than
  becoming implicit simulation time;
- under the initial `FrontierBlocking` policy, the entire session frontier
  remains blocked instead, so the intervening `R+1` commit is impossible until
  a result is admitted or `Manage` records cancellation, timeout, or failure;
- any rule fallback is a later typed lifecycle invocation scheduled by that
  recorded disposition, not an unrecorded way to release the frontier.

## 6. Checkpoint, restoration, verification, and branch

### Setup

A checkpoint is taken while actors have active intentions, activities, a
retained appraisal continuation, processes, reservations, a pending external
invocation, and future scheduler work. More commits follow.

### Required flow

1. Restoration loads the checkpoint and applies the exact hash-linked
   `AuthorityRecord` tail, including admission, moment, and management records.
2. No lifecycle evaluator, effect primitive, or external service is invoked.
3. The restored state, lifecycle control, pending invocation, scheduler,
   process generations, and hashed history cursor match the original.
4. Verification replay regenerates deterministic attempts from captured
   inputs and the management/admission-sealing trace, then compares
   nested `AttemptRecord` and outer `AuthorityRecord` fingerprints, including
   `MomentBatchRecord` contents.
5. A direct branch with a different stateless action profile is allowed only
   at a quiescent cursor for that port. In the pending-invocation subcase, the
   test either resolves under the pinned old profile before branching or uses
   an explicit child-root reset to cancel/discard it before creating a new
   invocation.
6. The parent remains unchanged and the child records lineage, lifecycle
   profile, reset if any, and compatibility differences.

### Assertions

- the checkpoint cursor cannot describe a different state prefix;
- the admission frontier, input/management/command deduplication ledgers,
  pending reaction envelopes, and pending evaluator state are restored exactly;
- checkpoint bytes are encoded from one immutable head and installed only if
  revision and the complete history cursor (last sequence, record-or-epoch
  anchor hash, and cumulative hash) all match, and the canonical
  `checkpoint_state_fingerprint` equals the checkpoint projection recomputed
  from that head;
- crash injection before and after `append_and_publish` for every authority
  record kind exposes the complete old or new head, never a mixed head;
- a competing publication produces `HeadConflict` rather than overwriting a
  newer head;
- the exact artifact closure is present or restoration fails before activation;
- restoration and verification are named, separate APIs;
- the branch receives a new history and deterministic randomness policy;
- compacting the parent prefix is forbidden while a retained branch needs it;
- profile changes affect only the new branch;
- an old pending result is never interpreted by the replacement policy, and
  its old artifact closure remains retained while referenced;
- a stateless action-policy change is direct only at a quiescent port boundary,
  while pending or persistent-port state requires explicit resolution,
  migration/reset, or a pre-policy root.

## 7. A background actor is promoted during travel

### Setup

An individually identified actor's movement scope is in `Background`
resolution with a coarse travel representation and a scheduled arrival. An
external event makes that scope relevant before arrival.

### Required flow

1. Runtime analytically advances the coarse process to the interruption moment.
2. A `ResolutionTransition` cancels obsolete background wakes.
3. Promotion reconstructs detailed state using a declared policy and random
   keys where necessary.
4. Cross-resolution invariants are checked.
5. The actor resumes under detailed scheduling from the same identity and
   accepted progress.

### Assertions

- detailed and background representations never update concurrently;
- exactly one representation is active for the movement
  `ResolutionScopeId`, without forcing unrelated actor subsystems to the same
  tier;
- location, conserved resources, obligations, and process time remain valid;
- lost microstate is recorded as approximation rather than claimed history;
- stale arrival triggers cannot later fire;
- hysteresis prevents immediate demotion/promotion thrashing.

## 8. A checkpoint is loaded with different packs

### Setup

A checkpoint references `RuntimeDefinitionSetDigest` `A`. The host tries to
load it with `RuntimeDefinitionSet` `B`, which changes action or effect
semantics.

### Required flow

1. Compatibility validation fails before session activation.
2. The host may supply an explicit, versioned migration from `A` to `B`.
3. The offline checkpoint migration engine first produces a canonical
   execution-spec-independent `InitialStateRoot`, then the child
   `ExecutionSpec`, artifact closure, root checkpoint, provenance record, and
   definition-set digest.
4. The original checkpoint remains intact.

### Assertions

- source-level semantic version compatibility does not substitute for exact
  artifact identity;
- process-local numeric IDs are not silently reinterpreted;
- the initial-state root and child execution specification contain no
  self-reference;
- unknown IR operations or unsupported schemas fail closed;
- reproducible sessions never hot-reload the new semantics in place.

## 9. A paired action-policy experiment

### Setup

Two run cases use the same frozen scenario, definitions, partner policies,
initial state, and named exogenous random streams. Only `ActionPolicy` differs.
Until treatment-caused divergence, both use the same canonical admitted-input
trace and declared management/admission-sealing trace.

### Required flow

1. The study design deterministically creates two assignments, resolves each
   into an ID-free exact `ExecutionSpec` body, and hashes two
   `ExecutionSpecId`s.
2. Each `RunCaseId` combines its study assignment, the shared exact
   `ScenarioArtifact` provenance, and its `ExecutionSpecId`; scenario
   provenance remains outside execution and trajectory identity, and no ID
   appears in its own hash preimage.
3. Both policies receive equivalent actor views and candidate universes where
   their histories have not yet diverged.
4. Branch-specific endogenous randomness follows each causal history.
5. Run artifacts are retained immutably; each resulting `TrajectoryId` combines
   its `ExecutionSpecId` with the cumulative authority-history hash.
6. Each immutable `RunCaseResult` maps its assignment to the selected attempt,
   `RunArtifactSet`, and trajectory. A separate `AnalysisManifest` references
   those labeled mappings, and the `MetricSetArtifact` computes paired
   differences afterward.

### Assertions

- appraisal, intent, activity, and memory implementations are held fixed;
- no policy receives oracle-only inputs unless the condition is labeled;
- metrics cannot change decisions;
- changing a metric creates a new report, not a new run;
- assigning an exact retained trajectory in another study changes the
  study-scoped run-case identity but need not rerun simulation when the
  `RunArtifactSet` also satisfies capture requirements;
- matching only `ExecutionSpecId` cannot justify reuse when admitted inputs or
  deferred results differed;
- the inspector can identify the first causal or decision divergence.

## 10. A zero-duration causal cycle

M2 owns the generic same-tick budget and management escape protocol. No current
milestone claims an authored rule that reproduces this cycle: adding one only
for the gate would be a synthetic producer. The first real lifecycle or
process rule capable of forming the cycle must exercise this scenario without
adding a second recursion guard. M4 separately proves that management can
release or preserve a retained action-evaluation frontier blocker.

### Setup

A misconfigured rule causes an accepted update to schedule another equivalent
update at the same tick indefinitely.

### Required flow

1. Every consequence advances microstep.
2. The session-level same-tick budget is exhausted.
3. The kernel publishes a `ManagementBatchRecord` selecting `Paused`,
   `Quarantined`, or `Failed`. This emergency publication does not consume the
   exhausted ordinary-work budget.
4. The diagnostic includes the repeating triggers, commits, and causal links.
5. The remaining trigger frontier is preserved; resumption requires captured
   host intervention.
6. An authorized, idempotent host intervention is submitted directly through
   the scheduler-independent `Manage` path while ordinary `Fire` remains
   disabled.
7. One `ManagementBatchRecord` atomically captures and applies it.
8. A resume record preserves unresolved due work unless the captured
   disposition explicitly removes it.
9. A seal request that would cross the preserved due work or an unresolved
   `FrontierBlocking` invocation fails; one management batch may explicitly
   dispose of that blocker and then seal to its validated target.
10. Retrying the exact management request after mode or frontier changes
    returns its original outcome. Reusing its ID with a changed disposition or
    seal target changes the request fingerprint and fails closed.

### Assertions

- recursion cannot overflow the host stack;
- simulation time cannot silently remain stuck;
- partial accepted commits remain valid history;
- failure policy and terminal control state are explicit and deterministic;
- management can escape a scheduler-blocked or paused state without inventing
  wall-clock time or losing the due frontier;
- exact retained management-request retry returns the original outcome without
  a new revision, while retained ID reuse with another request fingerprint
  fails closed;
- after full outcome retention expires, the retired management ID returns
  `DuplicateExpired` without a revision or effect and can never become new.

## 11. Physical outcome and actor meaning diverge

M4 owns the physical-event, observer-specific evidence, belief, and appraisal
separation in steps 1–4. It keeps the social port explicitly disabled and does
not claim step 5 or the gift/coercion interpretation. A later evidence-gated
social implementation must complete that portion. M5 owns restoration and
verification assertions over the physical, epistemic, appraisal, and pending
reaction records already implemented.

### Setup

One actor gives an item to another. The physical transfer succeeds. The
recipient interprets it as a gift; a witness suspects coercion.

### Required flow

1. Runtime commits only the ownership/inventory transfer and its domain event
   in a moment record with a nonempty reaction envelope.
2. Post-commit routing proposes different observer-specific evidence
   deliveries for recipient and witness.
3. Separate epistemic transitions accept their evidence records and beliefs.
4. Their appraisal or optional social evaluators emit different hypotheses.
5. Social updates are separately validated and accepted per actor or
   institution.

### Assertions

- the physical effect does not write `gift`, `coercion`, gratitude, or trust;
- different actor-relative interpretations can coexist;
- every interpretation cites its own evidence and policy;
- raw domain outcome, commit, and process references never enter appraisal,
  activity, action, or social policy inputs;
- restoration applies already committed physical, epistemic, and social
  records without rerunning interpretation;
- verification may rerun compatible deterministic interpretation and compare
  resulting moment-record fingerprints;
- restoration from before interpretation recovers pending self-contained
  reaction work rather than inventing the social result.

## 12. Minimal actor without higher cognition

### Setup

An actor begins without an intent or activity. Its profile disables rich
appraisal and uses deterministic rule-based intent, activity, and action
implementations.

### Required flow

1. The actor receives evidence and a minimal actor view.
2. Rule intent policy selects a supplied intent candidate; acceptance creates
   an `Intent` containing its desired condition and schedules activity
   initialization.
3. `ActivityController.initialize` returns an `ActivityInitOutcome` proposing
   one versioned activity state and `OpenActionOpportunity(ActionScope)`.
4. The agency gate publishes a `MomentBatchRecord` that atomically creates the
   `Activity` and one explicitly sponsored `ActionOpportunity`.
5. `ActorReadyForAction` references that open opportunity.
6. Rule action policy selects one supplied grounded candidate ID.
7. A ready `Select` or `NoApplicableAction` decision consumes the opportunity
   exactly once. A captured-deferred execution binding instead moves it to
   `WaitingForEvaluation`; recorded cancellation, failure, or exhausted
   fallback returns it to `Open` before closing it through the checked
   invocation-control protocol.
8. Runtime validates and commits the selected command normally.

### Assertions

- no LLM, planner, emotion taxonomy, or theory-of-mind engine is required;
- authority, trace, scheduling, and runtime contracts are identical to richer
  profiles;
- disabling a higher layer cannot disable the lower execution path;
- there is no separate goal object, planner port, or duplicated action-need
  state;
- comparisons can replace one lifecycle implementation at a time.

## 13. An invalid pack is rejected before activation

### Setup

An activation set includes several ordinary invalid cases: corrupt artifact
bytes, a mismatched descriptor, conflicting dependency constraints for one
`PackKey`, an authority-illegal effect operation, an excessive semantic
collection, and a required semantic interface absent from the supplied
catalog or installed engine distribution.

### Required flow

1. The resolver selects an exact package/source graph before import/type
   checking; source packages do not pretend to have artifact digests yet.
2. Compiler-produced and decoded `ArtifactData` converge on the same
   catalog-aware domain validator.
3. Loading checks the outer byte limit, descriptor, length, digest, schema,
   and structure before domain validation.
4. Family validators enforce stage, references, semantic cardinality,
   structural termination, and deterministic execution limits.
5. Artifact digests are known before `PackLock` is finalized.
6. Linking verifies direct edges and produces a process-independent
   `RuntimeDefinitionSet`.
7. Activation compares the exact required semantic-interface closure with the
   installed `EngineDistribution` and produces an
   `ActivatedDefinitionRegistry`.
8. Every invalid case is rejected before session construction.

### Assertions

- a signature cannot bypass verification or grant execution authority;
- one runtime definition set contains at most one artifact per `PackKey`;
- digest, semantic fingerprint, and source-map sidecar identity are distinct;
- unused installed semantic interfaces and activation intern order do not
  change definition-set or execution identity;
- process-local intern IDs never appear as durable definition identity;
- budget exhaustion has no partial runtime effect;
- unknown or authority-illegal operations are rejected before activation.

## 14. Crash occurs before post-commit dispatch

### Setup

A domain moment batch with observable consequences commits a nonempty reaction
envelope, but the process crashes before observation projection or external
adapter delivery.

### Required flow

1. The source `MomentBatchRecord` atomically publishes a nonempty,
   self-contained `ReactionEnvelope` and corresponding
   `PostCommitDispatch` scheduler delta.
2. A checkpoint is installed before dispatch; the compactable source record is
   then removed.
3. Restoration recovers the trigger and envelope without loading the compacted
   source record.
4. One `Fire` transition consumes the trigger. `PostCommitRouter` emits bounded
   evidence-delivery, invalidation, and lifecycle-wake proposals, which a later
   `MomentBatchRecord` accepts or rejects.
5. After restored routing commits a self-contained delivery record, the
   compacted-source subcase durably materializes a transactional outbox entry
   before that later record is unpinned; the outbox then survives independently
   and redelivers at least once. In a separate persistent-cursor subcase, the
   unacknowledged cursor pins the later routing record, so its compaction is
   forbidden until acknowledgement.
6. A portable archive created while either obligation is pending includes a
   consistent, separately fingerprinted reliable-delivery snapshot, or archive
   creation fails.

### Assertions

- accepted state cannot exist without recoverable reaction work;
- observer/evidence work is not executed inside the original commit;
- the source history record need not remain available for routing;
- logical trigger consumption occurs exactly once;
- a batch with an empty `ReactionEnvelope` schedules no
  `PostCommitDispatch`;
- consuming a routing result with no new observable consequence schedules no
  further dispatch and reaches quiescence;
- adapter delivery is at-least-once and idempotent;
- compaction cannot remove the only copy of an unacknowledged adapter
  obligation;
- archive/restore cannot silently drop a pending reliable-delivery guarantee;
- lossy telemetry is not mistaken for reliable delivery.

## 15. An evaluator result is duplicated around a checkpoint

### Setup

An evaluator response is accepted in
`Admission(ActionEvaluation(ActionEvaluationAdmissionRecord))`, and its
delivery is still pending when a checkpoint is installed. The process crashes
before the caller receives acknowledgement. After restore, the host retries
the same result capture twice. A separate `HostScheduled` subcase races a
capture effective at moment `m` against a host call that seals `m`.

### Required flow

1. The restored `AdmissionFrontier`, invocation and request identities,
   `ActionEvaluationCaptureId`, capture fingerprint, typed result identity and
   schema, artifact binding, admission mode, effective moment, delivery
   trigger, and capture-ledger state match.
2. The retried response carries the original
   `ActionEvaluationInvocationId`, `ActionEvaluationRequestId`, and
   `ActionEvaluationCaptureId`.
3. Retaining the result, changing invocation control, scheduling the explicit
   effective `SimMoment`, and releasing only that invocation's blocker were
   one atomic action-evaluation admission publication.
4. Exact retained capture retries return the original capture outcome before
   current invocation or frontier validation and without a new revision.
5. Reusing the retained `ActionEvaluationCaptureId` with a changed effective
   moment, invocation/request binding, artifact binding, result identity or
   schema, admission mode, or payload changes its capture fingerprint and
   fails closed.
6. The restored delivery trigger fires exactly once.
7. Any downstream command retry resolves through the restored command ledger:
   an exact `(source, CommandId, request fingerprint)` duplicate returns its
   original outcome, while any authority/effect-bearing field mismatch fails.
8. The capture/sealing race linearizes either before sealing, admitting the
   `HostScheduled` result at `m`, or after sealing, rejecting a new capture as
   backdated. An exact already-retained capture retry still returns its
   original outcome before the frontier check.
9. After the complete downstream-command retention horizon, an authoritative
   command-ledger transition compacts a contiguous terminal prefix. Retrying a
   command ID at or below that frontier returns `DuplicateExpired` with no
   revision or effect.
10. A reordered gap above the frontier remains retained until it is resolved
    or explicitly closed; command-ledger compaction cannot make that ID
    reusable.
11. With a retained command entry `Exact(f, outcome)`, a later due batch
    containing fingerprints `f` and `g` returns the original outcome for `f`,
    returns `IdReuseMismatch` for `g`, emits no new attempt/effect/ledger delta,
    and leaves the exact entry unchanged. The enclosing `MomentBatchRecord`
    still consumes the due trigger and seals the moment. Reuse of a retained
    collision tombstone likewise returns its stored collision outcome without a
    request-specific delta.

### Assertions

- network arrival time never selects the simulation moment;
- no evaluator result is lost between action-evaluation admission and scheduler
  insertion;
- checkpoint and restoration retain the exact capture ledger, invocation
  transition, blocker disposition, and delivery work;
- evaluator results never enter `IngressBatchRecord`, use `InputId`, or share
  the command-input ledger;
- duplicate/reordered delivery does not duplicate an action;
- acknowledgement loss cannot duplicate a non-idempotent command effect;
- same-ID command envelopes in one due batch are grouped before parallel
  preparation: identical request fingerprints create one attempt, while
  differing fingerprints create one durable `IdCollision` attempt and no
  selected winner;
- existing exact and collision ledger entries are consulted before grouping
  can create new work;
- an all-duplicate due moment still consumes each trigger exactly once in its
  enclosing authority publication;
- malformed or oversized transport framing, failed authentication, or
  undecodable transport bytes rejected before an
  `ActionEvaluationCaptureRequest` is constructed create no authority record
  or revision;
- a structurally valid capture request whose evaluator artifact exceeds its
  role-bound limit publishes exactly one
  `Admission(ActionEvaluation(ActionEvaluationAdmissionRecord))`, retains
  bounded `ArtifactRejected` evidence, and schedules the declared fallback
  without a partial evaluator effect;
- restoration never calls the evaluator again.

## 16. A stale global revision has unchanged policy input

M3 owns the invariants that actor-safe fingerprints exclude global revision,
that synchronous work is completed within one exact prepared-step
reservation, and that runtime legality is checked authoritatively. M4 owns the
retained-evaluation flow below, including dependency witnesses, private rebind,
discard, and reinvocation.

### Setup

An action result was evaluated at revision `R`. At `R+1`, an unrelated actor's
private inventory changes. At `R+2`, two subcases change the selected target:
one changes actor-visible projected input, while the other changes only a
private runtime-legality condition.

### Required flow

1. The engine-private invocation/result envelope separates policy-input
   dependencies from execution-validation dependencies; neither section
   appears in the policy payload or candidate.
2. At `R+1`, the original narrow witness remains valid or trusted projection
   rebuilds locally. Because the rebuilt payload is byte-identical, the
   coordinator privately rebinds or retains the result without another
   logical policy invocation or dispatch.
3. In the actor-visible `R+2` subcase, the rebuilt policy payload fingerprint
   changes, so recorded discard and a new logical evaluation are permitted.
4. In the private-legality `R+2` subcase, policy payload and invocation trace
   remain unchanged; selected-ID lowering and runtime legality are revalidated
   privately.
5. Runtime reevaluates authoritative legality in every subcase.

### Assertions

- global revision remains engine/trace provenance, not policy input or the
  only validity test;
- safe reuse requires positive witness evidence, not an assumption;
- private legality dependencies remain in the execution-validation section
  rather than being omitted to avoid invalidation;
- hidden-only revision or witness changes cannot alter policy-visible
  metadata or logical evaluator invocation/dispatch behavior;
- action legality and behavioral-input freshness remain separate.

## 17. Resolution scopes differ and telemetry load changes

### Setup

One actor has detailed movement, background economic production, and dormant
social deliberation scopes. Host CPU load changes during two otherwise
identical reproducible runs.

### Required flow

1. Each `(entity, subsystem)` `ResolutionScopeId` has exactly one active
   representation.
2. Scope transitions modify only their declared operational fields.
3. Hard conservation/identity/deadline invariants run at transition.
4. Fidelity error is evaluated offline.
5. Manifest-fixed simulation budgets, not telemetry, drive resolution choice.

### Assertions

- one actor need not use one global tier;
- cross-scope fields have one canonical owner;
- wall-clock load cannot change the trajectory;
- an adaptive host intervention must enter through a recorded `Admit` or
  `Manage` transition rather than telemetry.

## 18. An unrelated random draw is added

### Setup

An unrelated authoritative subsystem introduces an additional random draw
without changing the logical keys of existing outcomes. Diagnostic and
analysis code remains non-authoritative and uses no trajectory-affecting
stream.

### Required flow

1. Existing draws use semantic `RandomKey`s rather than one global sequence.
2. The new draw receives its own namespace and purpose key.
3. Declared exogenous streams retain their paired identity independently of
   policy-branch history.
4. Endogenous streams follow branch-specific causal identity and decisions.

### Assertions

- existing keyed outcomes remain unchanged;
- worker order and object creation order do not allocate streams;
- RNG algorithm and key-policy versions appear in execution semantics;
- verification detects accidental key reuse or policy drift.

## 19. A run terminates after publication and crashes before finalization

### Setup

One ordered `TerminationContract` clause becomes true in the
`TerminationView` projected from the immutable head produced by a `Fire` at
moment `m`. That head still contains valid future scheduler work. The process
crashes after `append_and_publish` commits the head but before the attempt gate
records its finalization.

### Required flow

1. The attempt's permanent binding matches the exact authority domain, run,
   specification, initial root, and epoch.
   `RunAttemptControl::Active(R-1)` compare-and-sets to
   `StepReserved(R-1, kind, operation fingerprint, AttemptStepId)` before the
   world transition begins.
2. `Fire` publishes exactly one `MomentBatchRecord` and authoritative head
   `R`; the same linearization point stores a `StepPublicationReceipt` binding
   both cursors, the record, operation fingerprint, and attempt step.
3. The trusted projector builds the declared `TerminationView` from head `R`;
   the pure termination/finalization rule evaluates it at the serialized
   barrier and selects one canonical reason.
4. Crash occurs before the reservation compare-and-sets to `Finalized`.
5. Recovery observes the reserved old cursor and authoritative head `R`,
   verifies that `R` is the exact direct successor and that the atomic receipt
   matches every binding and identity, and reruns only termination projection
   plus the pure termination/finalization rule.
6. One `RunFinalization` records `RunAttemptId`, terminal cursor `R`, reason,
   and `TrajectoryId = H(ExecutionSpecId || cumulative hash at R)`.
7. A second recovery or competing coordinator reads the same finalization and
   cannot install another cursor or reason.
8. Exact idempotent lookup for a request handled at or before `R` may still
   return its recorded result. New ingress, management, or scheduled execution
   scoped to the finalized attempt fails `AttemptFinalized`.
9. Continuing the retained future work requires an explicit child root/branch
   and new `RunAttemptId`.
10. Separate subcases cover termination at the root—where atomic create-or-open
    exposes `Finalized` without an unchecked active capability—and simultaneous
    eligible reasons; the contract's canonical ordering and manifest-fixed
    policy choose the same result in every run.
11. A no-publication crash subcase observes head `R-1` with no receipt and
    safely returns to active when no disposition exists; the caller may then
    resubmit through the ordinary idempotent surface, but recovery never
    reconstructs a request from its fingerprint. A retained non-cancellation
    disposition finalizes at `R-1` under the fixed policy.
    Missing/mismatched receipt, a non-successor head, or a second successor
    remains reserved, reports corruption, and grants no mutation capability.
12. An idempotent attempt-cancellation subcase atomically records its typed
    disposition, finalization, and exact deduplication outcome at the
    reconciled cursor without a world record or session-mode change. Exact
    retry, same-ID/different-request, retired-ID, crash-after-CAS,
    cancellation-wins, and reservation-wins races follow the declared control
    ledger and compare-and-set rules. Different same-ID cancellations
    serialize as one retained exact winner plus `IdReuseMismatch`, not a
    synthetic batch collision. A separate requested pause still requires
    `Manage`.
13. Archive subcases accept only matching active, receipt-proven reserved, or
    finalized control/head pairs and preserve the complete binding, control
    ledger/log, artifact-retention and pin state, disposition evidence,
    receipt, exact archive-closure digest, and optional delivery snapshot
    under one archive fingerprint.
14. Same-domain restore opens the existing writable control and delivery
    owners. Every portable active, reserved, or finalized copy is read-only;
    it cannot create a second writable control plane or reliable-delivery
    owner. Continuing simulation from a portable snapshot requires an
    explicit child root and new `RunAttemptId`.
15. Two freshly initialized independent writable control domains use the same
    `ExecutionSpecId` and runner-assigned key but distinct
    `AttemptAuthorityDomainId`s, so they derive distinct `RunAttemptId`s.
    Copying an archive preserves the source domain only as provenance and
    confers no writer capability.
16. Artifact-retention crash subcases stop after handoff intent persistence,
    target-pin acquisition, `RetainedBy` compare-and-set, and source-pin
    release. Recovery uses the owner-scoped pin ledger to finish or abort
    idempotently; every state has at least one valid owner. Terminal discard
    subcases stop before and after tombstone installation and garbage
    collection: the descriptor/fingerprint tombstone always precedes release,
    a retained exact retry returns the stored outcome, a retired request ID
    returns `DuplicateExpired`, and mismatched reuse fails closed.
17. Archive creation reconciles or rejects an in-flight retention intent,
    includes every `RetainedBy` run root and closure in its own fingerprinted
    closure, and refuses to claim restore/verification support for an
    artifact-discarded attempt.
18. Verification drives only the trace's ordered host step intents and
    logically anchored accepted exogenous control inputs. It regenerates the
    due-work selector, reservation, step ID, receipt, reconciliation,
    termination clause, and finalization and compares them with
    `ExpectedObservations`; none of those expected values chooses replay
    behavior.

### Assertions

- session mode and run-attempt lifecycle remain distinct;
- a session may remain `Running` with pending work while its attempt has one
  frozen terminal prefix;
- crash timing cannot move the terminal cursor;
- recovery invokes no policy, domain effect, or external service;
- no attempt-scoped authority record can appear after finalization;
- physical attempt identity and publication receipts do not change the
  authority-record or trajectory hash;
- checkpoint compaction, deduplication layout, history metadata, and private
  lifecycle state cannot enter `TerminationView` unless represented by an
  explicitly declared semantic termination signal;
- wall-clock observation and analysis code cannot select the prefix;
- a finalized archive retains both the terminal authority cursor and
  `RunFinalization`;
- every live attempt pins its exact resolved execution artifact closure, and
  no portable archive import creates split-brain control or delivery
  ownership;
- finalized handoff/discard crashes may leave extra pins but never zero pins,
  a dangling archive reference, or a reusable discarded attempt ID;
- changing any packaged control-log segment, pin-ledger entry, receipt/evidence
  identity, or archive closure changes the canonical archive fingerprint.

## 20. CLI and MCP-style adapters share one control boundary

### Setup

Two otherwise identical sessions have deterministic work before the next
player input opportunity. The same authenticated principal and scope control
the actor through a CLI adapter in one session and an MCP-style adapter in the
other; their transport framing and replaceable authentication proofs differ.
A separate installed action evaluator uses `DeferredCaptured`.

### Required flow

1. CLI `advance` and the corresponding MCP-style `run` operation execute the
   same due causal work and stop at the same next player input opportunity.
2. Read-only inspection before and after the stop consumes no simulation time
   and cannot alter the next due work.
3. The inspector exposes the same `ActorControlFrame` and actor-safe candidate
   identities through both product adapters.
4. The principal selects the same supplied candidate. Principal and scope
   remain in the engine-private authority request, while transport framing,
   replaceable authentication-proof bytes, and host audit metadata remain
   outside the semantic actor-control input and request fingerprint.
5. Both adapters invoke the same engine-owned control operation; neither
   constructs a runtime command or receives the private resolution table.
6. The deferred-evaluation session durably records its exact projection-safe
   request before an authenticated dispatcher can read and send it.
7. The dispatcher returns only a result allowed by that request. The result
   enters through `ActionEvaluationAdmissionRecord` and follows the ordinary
   freshness and delivery protocol from scenario 5.
8. Product inspection shows the adapter, authentication, dispatch, admission,
   decision, attempt, and authority provenance without exposing hidden state
   as policy input.

### Assertions

- equivalent CLI and MCP-style choices under the same principal and scope
  produce the same canonical semantic input, request fingerprint, and
  trajectory;
- the turn shell stops at an input opportunity rather than advancing virtual
  time while waiting for wall-clock player input;
- repeated inspection changes neither the head nor the next returned control
  frame;
- authentication grants a narrow actor-control or evaluator-dispatch
  capability, never session mutation authority;
- adapters cannot invent definitions, bindings, candidates, effective
  simulation time, commands, or evaluator results;
- evaluator transport receives no private invocation envelope, hidden
  projection dependencies, or runtime command capability;
- dispatch cannot precede durable request publication, and duplicate delivery
  cannot duplicate an effect;
- changing transport latency or adapter implementation cannot change a
  reproducible trajectory.

## 21. AI-assisted authoring activates only through a child epoch

### Setup

An AI author proposes a small foundation-vocabulary T1 action or physical-event
definition using the existing embedded ordered typed effect-call
representation. Its first output is invalid; a repaired output compiles. An
isolated preview uses an already existing materialized root and content
fixture, while a parent session remains active under its exact artifact
closure.

### Required flow

1. AI output enters as untrusted source through the same compiler entry point
   used for human-authored source.
2. Structured diagnostics identify the invalid definition without producing
   an activatable artifact.
3. A repair produces checked source, and the compiler, linker, and activation
   validation build an exact candidate artifact closure.
4. Preview constructs an isolated non-authoritative or separately rooted
   session from the preexisting root and records the source, diagnostics,
   artifacts, execution specification, and preview result.
5. Acceptance creates an explicit child root and child semantic epoch pinned
   to the new closure. The parent retains its original state, semantics, and
   pending work.
6. Rejection or failed preview produces no child and no mutation of the
   parent.

### Assertions

- AI authors have no compiler, linker, preview, or activation bypass;
- the accepted source contains only foundation T1 action or physical-event
  definitions and embedded ordered typed effect calls;
- the scenario introduces no reusable T0 content-data, process-definition, or
  social-definition family;
- diagnostics are reproducible for the same source and toolchain;
- preview cannot hot-reload or mutate the parent session;
- behavior-relevant changes always produce new semantic identity and exact
  artifact closure;
- an already pending parent invocation remains bound to the old closure and is
  never interpreted by the child definitions;
- activation authority is an explicit product operation, not an AI decision
  or compiler side effect.

## 22. Resolution transitions replace tier-specific work

### Setup

One individually identified caravan has a movement scope with retained cargo,
elapsed travel, and a delivery obligation, plus a cognition scope that is
dormant until an external observation or deadline. The movement scope begins
Detailed, demotes to Background, is promoted by an encounter, and later
demotes again.

### Required flow

1. Detailed-to-Background demotion reconciles work through the transition
   moment, validates invariants, cancels detailed wakes, installs sparse
   background work, and records approximation evidence atomically.
2. Background advancement preserves elapsed time, identity, cargo, and the
   obligation without replaying detailed decisions.
3. Promotion at an encounter cancels the background arrival, materializes
   declared detail through the versioned conversion policy, and installs
   detailed work from the same causal progress.
4. The later demotion repeats the checked replacement and cannot reuse work
   canceled by either previous tier.
5. The dormant cognition scope has no recurring evaluation. A declared
   external observation or deadline activates it once and replaces the
   dormant trigger with the target-tier work.
6. Restoration at each tier and immediately after each transition reconstructs
   the same active representation and replacement-trigger set.

### Assertions

- exactly one tier-specific representation and work set is authoritative per
  resolution scope;
- promotion, demotion, and dormant activation are all positive tested paths,
  not merely type-level possibilities;
- stale detailed, background, and dormant triggers cannot fire later;
- no transition invents an observation, repeats a past decision, or resets
  elapsed causal time;
- identity, conserved cargo, obligation, process deadline, and declared hard
  bounds survive the full cycle;
- approximation is explicit evidence and hysteresis prevents transition
  thrashing.

## 23. Body, tool, lock, and door capabilities compose

### Setup

A locked door separates an actor from a needed item. The actor has typed body
state and may possess a compatible tool or an acquired skill. Another fixture
changes one hand, the tool, or the lock while leaving unrelated state equal.
At M8 entry no reusable T0 content-data family is assumed. This scenario
introduces the first minimal pack-owned object-archetype family required by the
door, lock, tool, item, and checked root-materialization fixtures.

### Required flow

1. The T0 family declares only reusable object archetypes with the exact typed
   definition references and parameters needed for the tool, lock, door,
   container, medicine, and material fixtures. It has canonical identity and
   encoding, bounded validation, no behavior, and no executable operation.
2. The game or scenario initial-world source references those archetypes to
   place concrete instances. Checked root materialization resolves them
   against one exact definition set and creates accepted object state plus
   containment or access relations.
3. T1 definitions declare capability requirements, grounded bindings,
   duration/risk inputs, embedded ordered typed effect calls, and emitted
   events using installed operations.
4. Actor-relative projection derives only currently perceivable and bindable
   open, unlock, manipulate, break, or use candidates.
5. The selected candidate is lowered privately and current body, tool, lock,
   reachability, and resource requirements are revalidated.
6. Success changes only state owned by declared effects; failure is atomic and
   later consequences are scheduled explicitly.
7. Acquiring the skill or tool, injuring a hand, or changing the lock changes
   candidate presence, duration, or risk through data and owner-local
   semantics.
8. After the baseline fixture passes, a separate child fixture adds one
   additional tool archetype and compatible T1 action using only the installed
   T0 fields, T1 families, and semantic operations. It is compiled, linked,
   materialized, and activated without changing the T0 family implementation.

### Assertions

- the actor cannot invent an action or binding absent from the grounded
  candidate set;
- capability changes alter candidate space without adding a bespoke branch to
  the central engine, scheduler, or action-policy result type;
- hidden lock details do not leak through omission metadata or diagnostics;
- definitions compose existing primitives rather than directly mutating an
  untyped property bag;
- the first T0 family has one real pack producer and root-materializer
  consumer; it is not a universal entity schema, property bag, content
  registry, or scripting boundary;
- introducing the first family is recorded as new authoring, artifact, and
  root-materializer capability and is not misreported as locality evidence;
- the later existing-vocabulary child fixture changes only source, checked
  artifacts, root data, activation data, and tests; it changes neither the T0
  family schema/checker nor standard primitive semantics or runtime authority
  protocols;
- adding disjoint state outside every declared derivation and transition
  dependency cannot change candidate derivation, duration, risk, or effect;
- an accepted transition changes only its declared write owners, and any
  retained incremental capability view introduced by the slice is equivalent
  to full recomputation.

## 24. Heat, fire, smoke, and material integrity contend atomically

### Setup

A combustible container holding medicine is exposed to heat. Fire propagation,
an actor's extinguishing action, and a structural interaction can become due at
the same moment. Material, integrity, heat, combustion, smoke, containment, and
health or exposure state have explicit owners.

### Required flow

1. A checked T1 process definition and its stage-specific condition roots
   produce or advance a typed fire process with versioned state and scheduled
   wakeups.
2. Fire proposes integrity loss, medicine damage, heat transfer, and smoke
   production through declared effects, footprints, and one new concrete T3
   primitive owned by the relevant domain.
3. Extinguishing and structural interaction independently prepare from the
   same base snapshot at the same moment.
4. Each participating owner prepares typed reads, writes, resources,
   invariants, and a gate receipt. The ordinary prepared-subtransaction and
   total conflict protocol accepts a compatible set, emits typed rejection for
   conflicts, and publishes one atomic commit.
5. Accepted outcomes route explicit physical events into later exposure,
   observation, evidence, and process work.
6. Process completion, interruption, or loss of fuel invalidates obsolete
   generations and cannot recursively execute the reaction stack.

### Assertions

- insertion order, worker completion order, and collection representation do
  not change the result;
- medicine cannot be both preserved and destroyed through incompatible
  accepted proposals at one moment;
- partial integrity, heat, containment, or inventory mutation is impossible;
- fire and smoke are domain processes, not special scheduler or authority
  modes;
- cross-primitive interaction occurs through declared reads, writes, effects,
  events, and later work rather than pair-specific coordinator logic;
- the new T3 primitive adds owner-local state, preparation, codecs, migration,
  and composition-root wiring only; authority laws, dependency direction,
  unrelated primitive semantics, and unrelated public APIs remain unchanged;
- disjoint extra state cannot change process derivation or transition outcome,
  and accepted effects change only declared write owners;
- any retained incremental heat, smoke, or exposure view introduced by the
  slice is equivalent to full recomputation;
- same-tick propagation remains bounded by the existing causal-work budget.

## 25. Witnessed physical loss changes social and agency state

### Setup

The fire scenario destroys or threatens medicine promised for a delivery.
Two actors receive different observations. A witness makes a typed claim.
One versioned institution consumes declared relationship or standing input,
accepts or rejects a finding under a checked rule, and changes the delivery
obligation on the accepted path. One actor's later behavior depends on the
accepted result.

### Required flow

1. The physical commit emits observer-scoped observation opportunities without
   assigning social meaning.
2. Perception produces unequal typed observations; evidence assimilation and
   belief revision remain actor-relative.
3. A captured natural-language utterance remains presentation material around
   the interaction. The witness separately proposes a typed claim whose
   provenance names only permitted evidence.
4. The social interpreter or institution evaluates the claim using a checked
   T1 declaration, permitted evidence, and a declared relationship or standing
   input. In one fixture it accepts the finding and submits a typed delivery
   obligation transition through the ordinary authority protocol. A paired
   fixture differs only in that declared social input and rejects the finding
   or produces a different typed disposition.
5. Appraisal and intent consume permitted belief and accepted social state.
   Activity then produces a later grounded action opportunity.
6. A non-witness or an actor whose claim was rejected follows a different
   causal path without changing the physical history.

### Assertions

- physical truth, private belief, claim, institutional finding, relationship,
  obligation, appraisal, and intent are distinct typed state;
- observation or captured dialogue cannot directly create accepted social
  truth;
- claim provenance reveals no evidence the claimant was not permitted to use;
- social acceptance is explicit, deterministic under its installed rules, and
  replayable without natural-language reevaluation;
- changing only the declared relationship or standing input can change the
  social, appraisal, and agency path while leaving physical truth unchanged;
- the institution rule is a checked declaration interpreted by installed
  social semantics, not hard-coded scenario orchestration;
- the physical-to-evidence-to-social-to-agency chain consists of bounded later
  work, not recursive cross-system calls;
- this scenario completes the social portion deliberately deferred by
  scenario 11.

## 26. The integrated headless scenario survives evolution and scale

### Setup

A small deterministic headless fixture combines a bounded local square
settlement grid, remote site, regional route, player actor, rule-controlled
actors, the three M8 slices, and a Background caravan with a delivery
obligation. One actor has an active long activity while an independently owned
physical process is running. The scenario is controlled through the public
product boundary and checkpointed while causal work is in progress.

### Required flow

1. The first minimal pack-owned object-archetype T0 family introduced by
   scenario 23 and one lab-owned `ScenarioArtifact` remain distinct planning
   inputs to a checked materializer, which produces a separately identified
   baseline `InitialStateRoot` before execution.
2. `advance` or `run` deterministically reaches the next player input
   opportunity. CLI or MCP-style control receives the same canonical
   `ActorControlFrame` and starts the fire/door/medicine chain while the
   caravan advances independently in Background.
3. The fire threat interrupts the long activity through an explicit wake and
   checked transition without implicitly completing or canceling the physical
   process. Unequal evidence, a typed social result, persistent intent,
   replacement activity, and a later grounded action follow the ordinary
   causal pipeline.
4. A checkpoint captures the exact new physical, process, epistemic, social,
   agency, scheduler, and resolution state.
5. Same-domain restoration and verification reproduce the prefix without
   invoking an evaluator or authoring service.
6. Only after that baseline has executed, an explicit child branch adds one
   additional object archetype and compatible existing-vocabulary T1
   treatment, tool action, or social rule. It reuses the installed T0 family,
   T1 families, and semantic operations without extending their schemas. The
   parent remains unchanged.
7. Approaching the caravan promotes it; leaving the area demotes it; a retained
   deadline later activates one dormant scope. Each transition replaces
   obsolete work.
8. The restored, branched, and uninterrupted executions expose their causal,
   definition, epoch, adapter, and resolution provenance through inspection.

### Assertions

- the public product drives the same engine boundary used by integration
  tests; no scenario-only mutation API exists;
- pack data, `ScenarioArtifact`, and `InitialStateRoot` retain distinct owners,
  identities, and retention roles; source or planning provenance cannot mutate
  the running session;
- the first reusable T0 family is validated as a new narrow content capability;
  only the later child change counts as the existing-vocabulary locality proof;
- `advance` and `run` stop at the same next input boundary, CLI and MCP-style
  adapters expose the same canonical frame, and inspection consumes no
  simulation time;
- the composed systems preserve atomic authority, actor-relative information,
  lifecycle separation, deterministic time, and exact semantic identity;
- local-grid interaction and regional travel use distinct domain
  representations while sharing canonical actor, place, route, and causal-time
  identity;
- activity interruption does not erase its intent or physical process, and
  process completion does not directly complete the replacement activity;
- new M8 state survives checkpoint, replay, branch, promotion, demotion, and
  dormant activation without loss or duplicate effect;
- the later T0/T1 child change requires no content-family schema/checker,
  engine, runtime, scheduler, persistence, primitive-owner, or actor-policy
  protocol modification;
- one continuous causal explanation crosses physical, epistemic, social,
  agency, product, persistence, and resolution records;
- fixture names, concrete layout within the selected local square-grid and
  regional topology, and narrative may change. The topology and capability
  assertions may not be waived at roadmap completion.

## Coverage matrix

This is a reverse navigation index from architectural concerns to scenarios.
The roadmap's
[vision-to-evidence traceability matrix](implementation-roadmap.md#vision-to-evidence-traceability)
remains the authoritative owner of milestone assignment and
roadmap-completion coverage.

| Architectural concern | Primary scenarios |
|---|---|
| Actor-relative knowledge and no leakage | 2, 4, 11, 20, 25, 26 |
| Lifecycle separation, initialization, and persistence | 3, 6, 12, 19, 25, 26 |
| Grounded action and runtime revalidation | 1, 2, 5, 16, 20, 23, 26 |
| Authority records, atomic publication, and conflict handling | 1, 6, 10, 11, 14, 15, 19, 24, 26 |
| Determinism and external computation | 1, 5, 6, 9, 15, 16, 18, 19 |
| Checkpoint, idempotency, replay, and migration | 6, 8, 14, 15, 19, 21, 22, 26 |
| Multi-resolution consistency | 3, 7, 17, 22, 26 |
| Pack, IR, and extension trust | 8, 13, 21, 23, 24, 25, 26 |
| Research isolation and ablation | 5, 9, 12, 18, 19 |
| Failure boundedness | 2, 4, 5, 10, 13, 19 |
| Product adapter parity and captured evaluator transport | 5, 20, 26 |
| Player turn shell and zero-time inspection | 20, 26 |
| Checked AI-assisted authoring and epoch evolution | 8, 13, 21 |
| Foundation T1 action/event authoring locality | 21 |
| First narrow reusable T0 content-data family | 23, 26 |
| Later existing-vocabulary T0/T1 change locality | 23, 26 |
| Concrete T3 owner locality and recorded change impact | 24, 26 |
| Dependency-frame stability, write confinement, and derived-view equivalence | 23, 24 |
| Pack content, `ScenarioArtifact`, and `InitialStateRoot` separation | 23, 26 |
| Positive promotion, demotion, dormant activation, and work replacement | 7, 17, 22, 26 |
| Capability-derived object and body interaction | 23, 26 |
| Cross-primitive physical process composition | 24, 26 |
| Typed social causality and obligation-driven agency | 11, 25, 26 |
| Integrated persistence and resolution of gameplay state | 22, 26 |
