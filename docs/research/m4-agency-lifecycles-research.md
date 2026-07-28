# M4 Agency-Lifecycle Research

## Purpose

This note records the research used to enter M4. It asks one architectural
question:

> What is the smallest durable agency model that lets actors maintain unequal
> knowledge, pursue persistent purposes, interrupt and resume time-bearing
> behavior, and optionally use deferred external evaluators without collapsing
> cognition into runtime authority?

The answer must extend the completed M3 actor-control waist rather than create
a second action path. It must also preserve deterministic replay, actor-relative
information boundaries, independently scheduled lifecycles, and basic actors
that require neither planning nor external AI.

This document is a research input. The normative contract remains under
`docs/architecture/target-architecture/`, and the executable milestone plan
should select the smallest vertical slices that prove that contract.

## Result

M4 should implement one narrow causal spine:

```text
committed occurrence
  -> post-commit route and coalesced lifecycle wake
  -> actor-relative evidence
  -> belief transition
  -> appraisal
  -> persistent intent
  -> persistent activity
  -> activity-sponsored M3 action opportunity
  -> grounded action
  -> authoritative action or process transition
```

Each arrow is an explicit committed transition. No lifecycle recursively calls
the remaining stack, and no evaluator can directly mutate another lifecycle or
the authoritative world.

The cross-system architecture should standardize only:

- stable lifecycle and invocation identities;
- actor identity and immutable source authority;
- expected state version and coalescing generation;
- typed causes, inputs, dispositions, and outputs;
- dependency evidence for retained results;
- explicit scheduling and causal trace edges; and
- durable invocation, timeout, cancellation, and fallback control.

Behavior trees, utility systems, GOAP, HTN, social reasoning, emotion models,
and language-model prompting are possible implementations inside those
boundaries. They are not additional cross-system layers.

## Existing architectural constraints

The current target architecture already fixes the most important boundaries:

- evidence, appraisal, social interpretation, intent, activity, action, and
  process are distinct lifecycle concepts;
- intent and activity persist until an accepted transition changes them;
- activity sponsors the existing M3 action-opportunity protocol;
- a process is authoritative, time-bearing runtime state rather than cognitive
  state;
- policy inputs are actor-relative and cannot contain hidden authority
  metadata;
- external computation begins as a committed `DispatchPending` record;
- replay consumes captured results and never silently invokes an external
  service;
- deferred completion cannot retroactively join its invoking moment;
- `FrontierBlocking` is the initial reproducible policy and `HostScheduled` is
  an explicit nonblocking alternative; and
- `Unavailable` and a valid empty projection are different results.

Research mostly confirms these choices. It materially sharpens lifecycle
coalescing, commitment and reconsideration, dependency witnesses, terminal
invocation control, and the amount of common machinery M4 should introduce.

## Independently scheduled lifecycle machines

### Evidence

Classic DEVS describes an atomic model in terms of typed input and output
events, durable state, internal and external transition functions, and a time
advance function. Coupled atomic models remain compositional, and a model is
piecewise stable between relevant events. See Van Tendeloo and Vangheluwe,
[*An Introduction to Classic
DEVS*](https://arxiv.org/pdf/1701.07697).

Game simulation practice reaches the same operational conclusion from a
different direction: event-based simulation avoids repeatedly scanning all
entities by scheduling work when a state-changing event makes it relevant.
See Dickinson,
[*Efficient Event-Based
Simulations*](https://www.gameaipro.com/GameAIProOnlineEdition2021/GameAIProOnlineEdition2021_Chapter02_Efficient_Event_Based_Simulations.pdf).

### Inference for M4

Each M4 lifecycle should be auditable as a small transition system with:

```text
typed accepted input
current durable state and version
deterministic transition
typed committed output
optional next SimMoment
```

The existing scheduler, authority records, and post-commit routing are already
the coupling mechanism. M4 does not need a generic DEVS framework, global
cognitive update loop, per-turn actor scan, or synchronous call chain.

When a scheduled internal deadline and an external input share a simulation
moment, the milestone must define their normal batch semantics rather than let
delivery order become policy. M2's complete least-due batch and later
microsteps are the appropriate substrate.

## Coalescing without lost work

### Evidence

Kubernetes documents its work queue as “stingy”: multiple additions of the
same key coalesce, but an item marked dirty while it is being processed is
queued again after processing completes. See the official
[`client-go/workqueue` documentation](https://pkg.go.dev/k8s.io/client-go/util/workqueue).

This is implementation evidence rather than a domain model. Its useful
property is that coalescing redundant work never loses a change that arrives
during execution.

### Inference for M4

Every independently scheduled lifecycle should distinguish:

```text
desired_generation
  newest committed input generation known for the lifecycle key

processed_generation
  generation represented by the currently accepted lifecycle state
```

A worker evaluates a captured desired generation and commits its result with
an expected lifecycle-state version. If another cause advances
`desired_generation` before that commit, the lifecycle remains dirty and is
scheduled again.

Coalescing applies to wake requests, not to semantic inputs:

- accepted evidence and its provenance remain durable;
- distinct raw causes remain traceable;
- only redundant requests to recompute the same lifecycle key are collapsed;
  and
- a generation is never advanced merely because work was attempted.

A single global “cognition generation” would couple unrelated actors and
lifecycles. Generations should be scoped to the smallest stable scheduling key,
such as actor plus lifecycle family or another concrete domain owner.

## Commitment, reconsideration, and persistent agency

### Evidence

PRS and BDI work distinguishes beliefs or information, goals or motivations,
and intentions or deliberative commitments. It identifies two bad extremes:
continually reconsidering every intention and blindly executing an obsolete
plan. Commitment is useful precisely because it persists while explicit
success, failure, or reconsideration conditions determine when it changes.
See Rao and Georgeff,
[*BDI Agents: From Theory to
Practice*](https://cdn.aaai.org/ICMAS/1995/ICMAS95-042.pdf), and Georgeff and
Lansky,
[*Reactive Reasoning and
Planning*](https://aaai.org/Papers/AAAI/1987/AAAI87-121.pdf).

PRS also permits several intention stacks and interleaves execution with
responses to new events. Its concrete global interpreter is one historical
implementation, not a required architectural form.

### Inference for M4

Intent and activity must be durable state rather than values regenerated for
every action opportunity.

Intent reconsideration should have explicit causes, for example:

- newly accepted material evidence;
- materially changed appraisal;
- satisfaction or invalidation of the intent;
- activity completion or exhausted recovery policy;
- an explicit scheduled reconsideration deadline; or
- actor or scenario control that is itself accepted through the relevant
  authority boundary.

Routine action success, rejection, or delay need not destroy an intent. The
activity owns local recovery decisions and may sponsor a later opportunity,
wait, suspend, choose another method, or report exhaustion to the intent
lifecycle.

Several intentions or activities may be represented as active, waiting, or
suspended when a scenario needs them. M4 should not introduce a universal
priority-stack algorithm. Focus and foreground-opportunity policy remain typed
agency policy.

## Modular cognition rather than a universal framework

### Evidence

FAtiMA Modular reports that accumulating emotion and cognition features in one
agent architecture increased complexity, motivating a small core with
independent optional components. See Dias, Mascarenhas, and Paiva,
[*FAtiMA Modular: Towards an Agent Architecture with a Generic Appraisal
Framework*](https://www.researchgate.net/publication/265033357_FAtiMA_Modular_Towards_an_Agent_Architecture_with_a_Generic_Appraisal_Framework).

Game-AI behavior selection has no single dominant algorithm. Finite-state
machines, behavior trees, utility selection, GOAP, and HTN planning make
different tradeoffs in authoring, reactivity, search, and persistent execution
state. See Dill,
[*Behavior Selection
Algorithms*](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter04_Behavior_Selection_Algorithms.pdf).

### Inference for M4

M4 should not add:

- a universal `CognitionContext`;
- a generic string-keyed cognitive blackboard;
- a universal planner port;
- a public generic lifecycle result envelope; or
- shared framework traits whose only consumer is one concrete baseline.

Concrete ports should own concrete projection-safe payloads, proposal types,
and accepted state. An activity controller may privately store a behavior-tree
cursor, utility hysteresis, a GOAP plan, an HTN task network, an LLM dialogue,
or no plan at all. That private state is sealed, schema-versioned
implementation state.

Common Rust abstractions should be extracted only after at least two concrete
lifecycles demonstrate identical semantics. Similar-looking dispositions in
the formal model do not by themselves justify one generic Rust enum.

## Evidence, belief, and appraisal

### Evidence

*Talk of the Town* models knowledge as information propagated by discrete
character interactions. Characters can hold false beliefs, and the system
retains evidence, predecessor history, and provenance so beliefs have
trajectories rather than appearing as uncaused facts. See Ryan and Mateas,
[*Simulating Character Knowledge Phenomena in Talk of the
Town*](https://www.gameaipro.com/GameAIPro3/GameAIPro3_Chapter37_Simulating_Character_Knowledge_Phenomena_in_Talk_of_the_Town.pdf).

Truth-maintenance systems likewise record justifications so a conclusion can
be revised, retracted, or explained. See Doyle,
[*A Truth Maintenance
System*](https://www.sciencedirect.com/science/article/abs/pii/0004370279900080),
and de Kleer,
[*An Assumption-Based
TMS*](https://www.researchgate.net/publication/220546361_An_assumption-based_TMS).

EMA treats appraisal as a subjective interpretation of the
person-environment relationship. It uses a common appraisal account rather
than requiring separate “fast,” “slow,” social, and tactical appraisal
architectures; differing dynamics can emerge from perception, inference, and
action unfolding at different times. See Marsella and Gratch,
[*EMA: A Process Model of Appraisal Dynamics*](https://people.ict.usc.edu/~gratch/CSCI534/Readings/COGSYS-RS-EMOTION-2008-6.pdf).

### Inference for M4

The minimal accepted model should retain:

- an evidence identity, actor, source, observation and acceptance time,
  semantic payload, and derivation or provenance;
- an actor-and-claim-keyed belief with version, current value or status, and
  supporting evidence references;
- explicit addition, supersession, and retraction transitions; and
- a narrow appraisal result plus enough structured support to explain which
  beliefs affected it.

Appraisal consumes the actor's beliefs and interpretations. It must not read
physical truth that the actor has not learned. A world-level fact may be
recorded as correct or incorrect for audit, but that classification must not
become actor input merely because the engine can compute it.

M4 does not need probabilistic belief fusion, a general inference graph,
Bayesian filtering, an ATMS, or a universal uncertainty algebra. Those are
possible internal implementations once a concrete game system requires them.

A downstream intent wake is needed only when accepted appraisal changes
materially. Recomputing an equal appraisal may update operational provenance
without causing an unnecessary new deliberation.

## Social interpretation remains advisory

### Evidence

Versu models social practices as reactive joint plans that provide roles,
affordances, and suggestions but do not directly control an individual
character. Character-agnostic role binding also permits human and AI
participants to occupy the same authored role. See Evans and Short,
[*Versu—A Simulationist Storytelling
System*](https://cs.engr.uky.edu/~sgware/reading/papers/evans2014versu.pdf),
and the
[*Versu architecture overview*](https://versu.com/wp-content/uploads/2014/05/versu.pdf).

Comme il Faut demonstrates how reusable social norms and interaction rules can
avoid authoring every social situation as a special case. See McCoy et al.,
[*Comme il Faut: A System for Authoring Playable Social
Models*](https://ojs.aaai.org/index.php/AIIDE/article/download/12454/12313/15982).

### Inference for M4

Social interpretation may produce actor-relative meaning, evidence, appraisal
signals, or candidate activities. It may not:

- mutate physical state;
- issue a runtime command;
- bypass intent, activity, or action selection; or
- assert that every participant interpreted one occurrence identically.

The M4 vertical slice should prove that a recipient and a witness can derive
different accepted interpretations from the same occurrence. A social-practice
DSL, relationship simulation, norm engine, theory of mind, and conversation
planner are deferred implementation strategies behind the same boundary.

## Activity, action, and process

### Evidence

The BDI evidence supports persistent commitments and interruptible execution,
while behavior-selection literature shows that internal execution state varies
substantially by algorithm. Neither body of work treats the cognitive plan as
authoritative physical state.

### Inference for M4

The following distinctions should remain structural:

```text
Intent
  why the actor remains committed

Activity
  how one implementation is pursuing that commitment

Action opportunity
  one bounded actor-relative selection boundary inherited from M3

Action
  one immediate grounded attempt

Process
  authoritative time-bearing runtime mechanism
```

An activity can survive several action attempts and process transitions. A
process can continue while cognition is idle. Interrupting, pausing, or
redirecting a process is a grounded actor action or an explicit runtime
control transition, not an activity mutating process state.

This separation is what permits long travel, crafting, treatment, study, or
conversation to share a runtime process model without forcing their
domain-specific planning logic into the kernel.

## Durable deferred evaluation

### Evidence

Temporal's asynchronous activity completion allows a worker function to
return before the external operation completes; later completion is correlated
through a task token or workflow/activity identity. Temporal records activity
schedule, start, completion, retry, timeout, and cancellation behavior, and
requires implementations to tolerate retries. Cancellation is cooperative:
requesting it does not prove the external worker has stopped. See:

- [Temporal asynchronous activity
  completion](https://docs.temporal.io/develop/go/activities/asynchronous-activity);
- [Temporal activity
  execution](https://docs.temporal.io/develop/go/activities/execution); and
- [Temporal workflow
  cancellation](https://docs.temporal.io/develop/go/workflows/cancellation).

Azure Durable Task external events are durably awaited and may be delivered
more than once, so applications need stable event identity and deduplication.
See
[*Handling External Events in Durable Task
SDKs*](https://learn.microsoft.com/en-us/azure/durable-task/common/durable-task-external-events).

AWS Step Functions callback tasks similarly use task tokens and explicit
timeouts or heartbeats; after a timeout, a retry receives a different callback
token. See
[*Run a Job with Step Functions Using a Callback
Token*](https://docs.aws.amazon.com/step-functions/latest/dg/connect-to-resource.html).

These are workflow-system precedents, not a reason to adopt a workflow engine.
They establish the failure semantics expected whenever local durable state and
external execution are separated.

### Inference for M4

The target invocation protocol is appropriate:

```text
DispatchPending
  -> ResultCaptured
     -> Fresh | Stale
     -> Applied | Reinvoked | Fallback | Discarded
  -> TimedOut | Cancelled | Failed
     -> FallbackPending | Discarded
```

The required invariants are:

1. `DispatchPending` and the exact request are committed before external I/O.
2. Logical invocation identity is distinct from transport-attempt identity.
3. A transport retry uses the same request fingerprint and idempotency key.
4. Adapter delivery and result capture are treated as at-least-once.
5. External callbacks enter serialized ingress and never mutate simulation
   state directly.
6. Replay consumes the captured response and performs no external request.
7. Timeout, cancellation, failure, fallback, and discard are durable
   management dispositions.
8. Cancellation is a terminal local authority decision plus best-effort
   adapter cancellation; it is not evidence that remote work stopped.
9. A late duplicate result cannot reopen a cancelled, timed-out, failed, or
   consumed invocation.
10. A retry that semantically asks a new question receives a successor logical
    invocation linked to the earlier terminal one.

Kubernetes' versioned update and finalizer models reinforce two narrower
points: expected versions prevent lost updates, and terminal cleanup is a
state transition rather than resurrection of deleted work. See
[*Kubernetes API
concepts*](https://kubernetes.io/docs/reference/using-api/api-concepts/) and
[*Finalizers*](https://kubernetes.io/docs/concepts/overview/working-with-objects/finalizers/).

`FrontierBlocking` remains the initial reproducible mode because it prevents
wall-clock completion from changing which simulation work occurs first.
`HostScheduled` may allow unrelated simulation work to advance, but the host
must provide an explicit effective `SimMoment`. Raw completion order is never
simulation order.

## Freshness, rebind, and reinvocation

### Evidence

FoundationDB validates optimistic transactions at commit against the exact
read-conflict ranges. Its documentation explicitly notes that an external
process inside an optimistic loop requires equivalent validation at a higher
layer. See the
[*FoundationDB Developer
Guide*](https://apple.github.io/foundationdb/developer-guide.html) and
[*Known Limitations*](https://apple.github.io/foundationdb/known-limitations.html).

Salsa tracks the dependencies and revisions used by a derived query. If an
input changed, it reexecutes the query; if the output is nevertheless equal,
it can “backdate” the result so unchanged meaning does not force downstream
work. See the
[*Salsa red-green
algorithm*](https://salsa-rs.github.io/salsa/reference/algorithm.html).

These systems validate storage transactions or incremental queries rather than
game-agent decisions. The shared principle is narrow semantic dependency
tracking instead of invalidating everything on every global revision.

### Inference for M4

A retained evaluator invocation needs two dependency classes:

```text
policy-input dependencies
  actor-visible facts whose change can alter the evaluator's semantic input

execution-validation dependencies
  private facts needed to resolve or authoritatively validate the selected
  action, without changing what the evaluator was permitted to choose
```

The resulting behavior should be:

| Dependency outcome | Required disposition |
|---|---|
| No witnessed dependency changed | Reuse or apply the result, followed by normal authoritative legality checks |
| Policy dependency changed, rebuilt projection-safe payload is byte-identical | Privately rebind or reuse; do not invoke the evaluator again |
| Policy dependency changed, rebuilt payload differs | Discard the stale result and create an explicitly linked new logical invocation |
| Only execution-validation dependency changed | Re-resolve the selected candidate and revalidate legality; do not invoke policy again |

A global authority revision is useful provenance but is too coarse to decide
freshness.

The witness must also represent negative or absence dependencies. If an
evaluator saw “no visible threats,” recording only the empty set of returned
entity keys would remain falsely fresh when a new threat entered the queried
domain. A projection dependency should therefore be able to include a stable
generation for the bounded query domain, index partition, spatial bucket, or
other owner that establishes both presence and absence.

This is an inference from range-conflict and incremental-query systems. It is
not a requirement to expose a database range abstraction in domain APIs.
Domain projection owners should provide concrete, narrow dependency tokens.

## Typed unavailability

### Evidence

The external and incremental-computation evidence distinguishes absence of a
result, failed computation, and a valid result whose collection is empty.
Conflating these cases hides recovery policy and makes downstream behavior
depend on implementation accidents.

### Inference for M4

The first genuinely partial provider should return a typed disposition such
as:

```text
Complete(value)
Unavailable(reason)
```

`Complete(empty)` is valid semantic input. `Unavailable` cannot satisfy a
required projection and must drive an explicit abstain, wait, fallback, or
failure transition. The reason vocabulary should be concrete and bounded; M4
does not need a universal exception transport or generic error graph.

## Adopted implications

The following implications are sufficiently supported and fit the current
target architecture:

1. Implement lifecycle work as independently scheduled, version-checked
   transitions over durable state.
2. Use desired-versus-processed generations so coalescing cannot lose a wake
   that arrives during processing.
3. Coalesce work requests while retaining exact semantic evidence and causal
   provenance.
4. Persist intent and activity independently and require explicit
   reconsideration, completion, suspension, or abandonment transitions.
5. Keep appraisal actor-relative and narrow; materially equal output should
   not churn downstream agency.
6. Let activity implementations privately own planning or behavior-selection
   state.
7. Route activity-sponsored actions through the unchanged M3 candidate-ID
   selection and runtime-validation path.
8. Model process state as authoritative runtime truth and actor process control
   as grounded action.
9. Treat social interpretation as actor-relative evidence or advice rather
   than authority.
10. Commit deferred requests before I/O and treat transport and completion as
    at-least-once.
11. Make cancellation and timeout durable terminal dispositions that late
    results cannot reopen.
12. Separate policy-input freshness from private execution validation.
13. Reuse byte-identical rebuilt policy input without a new logical evaluator
    invocation.
14. Represent absence-sensitive query dependencies with bounded domain
    generations or equivalent concrete tokens.
15. Introduce the first real typed `Unavailable` only with a genuinely partial
    provider.

## Rejected implications

The research does not justify the following M4 designs:

- one global cognition tick or recursive evidence-to-action call stack;
- a generic agent framework, cognitive blackboard, or universal context bag;
- a single planner or behavior-selection algorithm at the architecture waist;
- policy access to authoritative world state, dependency versions, or
  execution-validation facts;
- regenerating intent for every action or abandoning it after every rejected
  attempt;
- evaluator-produced raw commands or direct world mutation;
- social rules directly commanding participants;
- wall-clock callback order determining simulation order;
- “cancelled” meaning that an external worker certainly stopped;
- a global revision alone deciding freshness;
- witnessing only returned entity keys for absence-sensitive queries;
- reinvoking policy because only a hidden legality dependency changed;
- probabilistic belief fusion, a universal TMS, or rich emotion machinery in
  the cross-system core; or
- extracting shared generic Rust lifecycle machinery before concrete repeated
  semantics exist.

## Deferred implications

These directions are compatible with the architecture but lack an M4
vertical-slice requirement:

- Bayesian or weighted belief revision;
- full truth-maintenance or assumption-environment reasoning;
- social-practice, dialogue, norm, quest, or activity DSLs;
- recursive theory of mind and rich emotion dynamics;
- utility, behavior-tree, GOAP, HTN, learned, remote, or language-model
  activity implementations beyond one bounded demonstration;
- actor-local virtual clocks;
- general speculative evaluation;
- distributed evaluator queues or workflow-engine integration;
- reusable dependency algebra beyond the first concrete retained evaluator;
- broad query indexes introduced solely for freshness;
- automatic evaluator-state migration; and
- background population or multi-resolution agency, which remains M7 work.

Deferral means the M4 boundaries must not preclude these features. It does not
mean M4 should install placeholder traits, empty enums, or generic storage for
them.

## Recommended implementation sequence

The research supports entering M4 through deterministic causal slices before
adding nondeterministic adapters:

1. Add post-commit lifecycle routing, scoped generations, expected-version
   transitions, and normalized trace links.
2. Implement accepted evidence, belief transition, and deterministic appraisal
   for the false-belief scenario.
3. Implement persistent intent and versioned activity state, then let one
   activity sponsor the existing M3 action opportunity.
4. Add one time-bearing process and grounded interruption, cancellation, or
   resumption through the action/runtime boundary.
5. Add one optional social interpretation in which recipient and witness
   derive different meanings from the same committed occurrence.
6. Introduce one genuine partial projection and prove
   `Complete(empty) != Unavailable`.
7. Add one deferred evaluator end to end: committed dispatch, at-least-once
   adapter, captured ingress, narrow witness, freshness classification,
   private rebind, explicit reinvocation, timeout, cancellation, fallback, and
   replay-safe result use.

The sequence is architectural rather than a promise of crate or work-package
shape. Each slice should be detailed from the repository state at entry and
should add only machinery with a current producer, consumer, invariant, and
validation scenario.

## Research-derived exit questions

Before M4 closes, its implementation and tests should answer:

1. Can a lifecycle be dirtied while processing without losing the later work?
2. Can two unrelated actors or lifecycle families progress without sharing a
   global cognition cadence?
3. Can an actor retain a false belief and choose a meaningful action without
   policy observing the hidden truth?
4. Does materially unchanged appraisal avoid unnecessary intent churn?
5. Can an intent survive action rejection while its activity applies bounded
   local recovery?
6. Does an activity use exactly the M3 action-opportunity boundary rather than
   constructing commands?
7. Can a process continue, suspend, or terminate independently of activity
   evaluation?
8. Can recipient and witness interpretations diverge without either becoming
   physical authority?
9. Does an empty projection remain distinguishable from an unavailable one?
10. Is every external request committed before I/O and every response captured
    before admission?
11. Can duplicate delivery, timeout, cancellation, and late completion occur
    without double application or terminal-state resurrection?
12. Does an actor-visible payload change cause reinvocation while a
    hidden-legality-only change causes private revalidation?
13. Does insertion into a previously empty query domain invalidate a retained
    “nothing observed” result?
14. Can the deterministic baseline operate with social interpretation,
    planning, and external evaluation all disabled?

If these questions have small, concrete answers, M4 will have established the
durable agency architecture without prematurely designing every future game
system.
