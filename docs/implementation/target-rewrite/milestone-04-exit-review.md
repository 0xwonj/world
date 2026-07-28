# M4 Exit Review: Independently Scheduled Agency Lifecycles

## Status

Accepted.

M4 extends the M3 actor-control waist into one deterministic causal spine:

```text
committed event
  -> post-commit routing
  -> actor-addressed evidence
  -> accepted belief
  -> appraisal
  -> intent
  -> activity
  -> activity-sponsored ActionOpportunity
  -> grounded action
  -> authoritative action or process transition
  -> later causal work
```

Every arrow that changes authoritative state remains a separate scheduled,
prepared, proposed, sealed, and applied transition. No evaluator mutates
state, no lifecycle recursively runs the stack, and no second command or
publication path was introduced.

## Semantic closure

`LifecycleProfilesV2` binds every enabled evidence, appraisal, intent,
activity, and action port by exact implementation identity, while its social
port is explicitly `Disabled`. Persistent port state also binds its schema
identity. Action execution independently selects inline or deferred-captured
evaluation, and deferred behavior is closed by `ExecutionConfigArtifactV3`.

Execution resolution proves that:

- missing, unknown, duplicate, wrong-port, or state-schema-incompatible
  implementations fail before activation;
- changing a selected implementation changes normalized execution semantics;
- unrelated installed implementations do not;
- social interpretation is explicitly disabled and requires no placeholder
  evaluator;
- a resolved attempt cannot replace a selected lifecycle implementation.

This closes behavior-affecting implementation choice inside `Γ` instead of
leaving it as mutable host configuration.

## State and ownership

`AcceptedState` is the checked aggregate of four explicit partitions:

```text
domain      physical containment, routes, and actor position
epistemic   evidence and belief
social      accepted social truth, empty in the M4 baseline
agency      intent, activity, and foreground focus
```

Action opportunities, relocation processes, lifecycle coalescing, deferred
invocations, scheduler work, and management state remain typed runtime
control. Appraisal remains derived lifecycle material rather than accepted
truth.

Canonical encoding and digests cover every partition exactly once. The
aggregate and each changed protocol use explicit versioned domains. Frozen
vectors cover the accepted-state roots, execution configuration, lifecycle
profiles, closure identities, scheduler identities, and authority records.

## Lifecycle substrate

The scheduler has concrete lanes for commands, post-commit work, process
wakes, lifecycle work, action readiness, deferred result/fallback work, and
neutral attempt resolution. A complete least-due moment still evaluates
against one immutable base snapshot. Visibility therefore comes only from a
later microstep, never from evaluation order inside a batch.

Lifecycle coalescing retains canonical material causes and separate desired,
processed, and enqueued generations. A cause committed while an older
generation is being evaluated remains dirty and produces exactly one later
successor. Evidence deliveries themselves do not coalesce.

The final runtime retains only causes with production paths: evidence,
appraisal, intent, and neutral attempt resolution. The speculative
activity-originated wake cause was removed.

## Evidence and persistent agency

A self-contained post-commit reaction can produce observer-specific evidence.
Evidence assimilation version-transitions accepted evidence and belief;
appraisal reads that accepted actor-relative state rather than hidden domain
truth. Material appraisal may later adopt or reconsider an intent.

Accepted intent and activity have independent identities, versions, statuses,
and lifetimes. The baseline containment method can initialize one focused
activity and one exact sponsored opportunity. Neutral `AttemptResolved` work
advances that activity later, independent of whether runtime accepted or
rejected the attempted action.

Persistent activity method state is one closed sum:

```text
ActivityState
  ContainmentTransfer(ContainmentTransferActivityState)
  Travel(TravelActivityState)
```

There is no generic activity-state map. Method-family changes are invalid.
Initial-root validation requires the sponsor to be the actor's focused active
activity at the exact version. Both root validation and later authority
sealing then use one model-owned predicate for the family, verb, scope,
endpoints, candidate bound, and generation represented by the retained
post-opening state.

## Relocation process

The first time-bearing mechanic is a concrete directed relocation:

- accepted domain state owns `At | InTransit` position and directed route
  duration;
- runtime control owns process identity, elapsed duration, due time, version,
  wake generation, and `Active | Paused | Completed` status;
- Start, Pause, and Resume cross the ordinary grounded-action boundary;
- runtime revalidates route and process legality without exposing it to the
  policy or activity controller;
- pausing records exactly the elapsed active segment;
- resuming schedules only the remaining duration;
- an obsolete wake is consumed without effect;
- arrival changes accepted position exactly once.

The travel activity retains only source, destination, next opportunity
generation, and `Pause | Resume | AwaitArrival`. Engine-private coordination
recovers the exact route from the immediately preceding activity-sponsored
opportunity. Process identity, progress, due time, version, wake generation,
and rich attempt outcome never enter the controller input.

The public cycle captures Start at tick 1, Pause at tick 4, Resume at tick 8,
consumes the obsolete original wake at tick 11, and arrives once at tick 15.
The activity is already `Waiting/AwaitArrival` before arrival and remains so
afterward. Focused authority tests also prove that process transitions preserve
agency and activity completion or failure preserves a live process.

M4 seeds the travel intent, activity, focus, and Start opportunity as one
checked execution origin. It does not claim general appraisal-to-travel
initialization. Arrival re-enters accepted evidence but does not yet produce
arrival appraisal or complete the waiting activity.

## Retained and deferred action evaluation

Deferred action evaluation is an action-specific protocol rather than a
generic asynchronous evaluator framework:

```text
ActionOpportunity
  Open -> WaitingForEvaluation -> Open -> Consumed

ActionEvaluationInvocation
  DispatchPending
  ResultCaptured | FallbackPending
  Terminal(Applied | Reinvoked | Failed)
```

Runtime commits the bounded request, engine-private continuation, positive
read witness, and dispatch state before the request becomes visible. Typed
capture is serialized and idempotent. `FrontierBlocking` fixes and holds the
simulation frontier; `HostScheduled` requires an explicit later simulation
moment.

At result use, the engine rebuilds current projection and classifies only the
dependencies that matter:

- unchanged policy witness reuses the result;
- changed witness with a byte-identical actor payload privately rebinds;
- changed visible payload creates one linked successor while budget remains;
- hidden-legality-only change revalidates private lowering without policy
  reinvocation;
- an invalid or unknown selection records failure and enters the fixed later
  fallback.

Cancellation, timeout, and host failure use the same checked invocation
protocol. Late results cannot reopen terminal state, and evaluator output can
never submit a command directly.

Authority-record schema v3 has byte-complete vectors for deferred admission,
management resolution, ActionReady invocation opening, and ResultReady
resumption and terminalization.

## Conformance evidence

The public and focused suites prove:

- `rejected_false_belief_retracts_only_after_modeled_absence_reaches_appraisal`:
  physical rejection, evidence, belief, and appraisal remain separate;
- `committed_transfer_drives_evidence_intent_activity_and_a_restoring_action`:
  one exact causal order crosses evidence, intent, activity, and the existing
  action waist;
- `travel_activity_owns_start_pause_resume_and_awaits_one_rescheduled_arrival`:
  activity-driven process control, stale-wake safety, exactly-once arrival,
  and bidirectional activity/process non-mutation;
- the seven deferred-action conformance cases: blocking and host-scheduled
  capture, cancellation, later fallback, private rebind, visible
  reinvocation, and hidden-legality revalidation;
- `same_moment_work_is_permutation_invariant_across_every_scheduler_lane`:
  insertion permutations do not change the complete-moment result;
- initial-root and model tests: focused sponsor, exact method opening,
  generation, scope, and endpoint pairing;
- workspace-structure tests: the crate dependency allowlist and removal of
  superseded symbols.

Scenario allocation is intentionally exact:

- M4 completes the evidence/appraisal portion of scenario 2;
- it completes the origin-seeded relocation and grounded-control foundations
  of scenario 3, not threat-driven interruption or general travel
  initialization;
- it completes the semantic in-process portion of scenarios 5 and 16, not
  restoration or transport;
- it proves the physical/evidence/appraisal portion of scenario 11 while
  social interpretation remains disabled;
- it completes the deterministic minimal actor in scenario 12;
- it retains the kernel budget and management escape for scenario 10 without
  claiming an authored self-generating rule cycle;
- scenario 4 remains unassigned because no real partial provider exists.

## Simplification review

The final M4 shape removes or declines:

- pre-M4 lifecycle identity and physical-only accepted-state shapes;
- reaction-only sponsorship assumptions;
- consume-only post-commit and attempt-resolution paths;
- arbitrary post-resolution controller replacement;
- a generic lifecycle runner, provider registry, mutable context bag, generic
  disposition, planner port, or activity-state map;
- a synthetic `Unavailable` result for total in-memory projections;
- a placeholder social evaluator;
- a generic process framework or process DSL;
- a speculative activity-originated lifecycle cause;
- public runtime test digests and unused action-evaluation ledger surfaces.

The remaining public traits are the concrete replaceable lifecycle ports and
artifact resolver. The remaining activity projection surface names the closed
multi-method advancement path accurately, while containment-specific
initialization remains explicitly containment-specific.

## Explicit deferrals

- General travel intent adoption, activity initialization, and arrival
  appraisal wait for concrete actor-relative semantic producers.
- Social interpretation waits for one real social relation and scenario.
- Partial projection waits for a genuine unavailable provider and consumer.
- Checkpoint, restoration, verification replay, branching, and durable
  delivery remain M5.
- CLI, MCP, player/AI-agent adapters, authenticated transport, and external
  evaluator delivery remain M6.
- Background simulation and multi-resolution promotion/demotion remain M7.
- Rich action DSLs, planner algorithms, social ontologies, spatial systems,
  and game mechanics remain local subsystem evolution behind the established
  boundaries.

None of these deferrals requires another authority path or a change to the
actor-safe action waist.

## Verification gate

The accepted gate is:

```text
cargo fmt --all --check
cargo check --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
git diff --check
```

All gates passed on 2026-07-28. The workspace run includes unit, integration,
public conformance, frozen-vector, compile-fail, dependency-structure, and
documentation tests.

## Exit decision

M4 is complete. Its stable abstraction is one scheduled authority waist around
distinct owned lifetimes: truth, evidence, belief, intent, activity,
opportunity, action, process, and evaluator invocation. The two complex new
subsystems—time-bearing relocation and deferred evaluation—keep their private
state inside runtime and engine boundaries while sharing only narrow,
actor-safe typed contracts with the rest of the architecture.

M5 can add durable checkpoint and replay semantics over this complete state
without invoking evaluators again or redesigning agency, process, or action
boundaries.
