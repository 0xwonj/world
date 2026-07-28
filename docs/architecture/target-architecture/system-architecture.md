# Target System Architecture

## Purpose

This document defines the target system shape: what the engine is, which
components exist, what each component owns, and which dependency and authority
directions must remain true.

It does not prescribe the internal algorithm of every component.

## Product definition

`world` is primarily a production-quality, headless, simulation-first RPG
engine. It is secondarily a platform for comparing decision and cognition
implementations under controlled simulation conditions.

The same engine must support:

- player-, AI-, script-, and experiment-controlled actors through symmetric
  request boundaries;
- partial and incorrect actor knowledge;
- persistent activities and long-running world processes;
- deterministic headless execution and branching;
- explainable decisions without making explanations authoritative;
- declarative game content without granting packs arbitrary code execution;
- richer future action, planning, cognition, and resolution implementations
  without broadening the core authority boundary.

The first product surfaces are a Rust library, a headless command-line tool,
an experiment runner, and trace inspection/export. Rendering, networking,
editors, services, and registries are adapters outside the simulation kernel.

## Design forces

The architecture optimizes for the following, in order:

1. correct authority and causal ordering;
2. deterministic, inspectable simulation;
3. domain-shaped interfaces;
4. replaceable cognition and content implementations;
5. efficient inactive-world handling;
6. controlled research comparison;
7. operational simplicity.

It does not optimize first for arbitrary plugins, maximal genericity,
distributed execution, or a universal cognitive theory.

## Four planes

The system is easier to reason about as four planes with one-way authority:

### Authoring plane

Parses and verifies source packs, resolves dependencies, and produces immutable
artifacts and a `RuntimeDefinitionSet`.

It has no live-session mutation authority.

### Simulation plane

Owns the authoritative session head, virtual time, processes, scheduler,
accepted state, runtime validation, atomic commit, and causal history.

This is the only plane that can accept world/session state changes.

### Cognition plane

Builds actor-relative read models and invokes independently scheduled evidence,
appraisal, optional social interpretation, intent, activity, and action
lifecycles. It emits typed proposals and selections.

It cannot mutate state and cannot read unrestricted authoritative state.

### Research and host plane

Resolves executions, owns durable run-attempt gating/finalization, supplies
external input, runs experiments, captures traces, computes metrics, and
exposes product APIs.

It may choose what to run and when to stop an attempt, but cannot bypass
runtime validation or mutate world state outside the three kernel transitions.

## Canonical concepts

The following concepts are shared architecture vocabulary.

| Concept | Meaning | Lifetime |
|---|---|---|
| `VerifiedPackArtifact` | Sealed, verified immutable compiled content unit | Across sessions |
| `RuntimeDefinitionSet` | Exact process-independent linked definition closure | Session epoch |
| `ActivatedDefinitionRegistry` | Reconstructible process-local indexes, intern IDs, and semantic dispatch | Process/session |
| `EngineDistribution` | Installed trusted semantic implementations, per-interface identities, and build provenance | Engine |
| `SemanticInterfaceCatalog` | Serializable pack-visible primitive and verifier contracts | Engine/compiler |
| `ExecutionConfigArtifact` | Exact state-affecting policies, budgets, and time rules | Session/branch |
| `LifecycleProfiles` | Per-port lifecycle implementation requirements and state schemas | Session/branch |
| `SemanticImplementationSet` | Exact behavior-affecting implementations bound for definitions, profiles, and configuration | Session epoch |
| `ExecutionSemanticsManifest` | Sole normalized identity of behavior-affecting semantics | Session epoch |
| `ResolvedExecution` | Engine-sealed specification, root, exact resolved artifact set, normalized semantics, and activation binding validated together | Session construction |
| `ResolvedExecutionClosureManifest` | Immutable pre-run closure required to construct or reopen a resolved execution | Attempt creation/recovery |
| `ArtifactClosureManifest` | Frozen root-relative closure for a checkpoint, archive, or finalized run | Retained root |
| `ScenarioArtifact` | Lab-owned immutable study provenance and checked recipe for root materialization; not runtime authority | Run case |
| `InitialStateRoot` | Runtime-owned canonical complete materialized starting state checked against exact definitions and semantics | Session epoch root |
| `InitialStateRootId` | Content identity of the execution-spec-independent complete starting mode/time/state/control/scheduler root | Session epoch root |
| `ExecutionSpecId` | Exact pre-run configuration identity, excluding study/analysis | Run configuration |
| `TrajectoryId` | Execution specification plus resulting authority-history identity | Run result |
| `AttemptAuthorityDomainId` | Stable namespace of one exclusive writable attempt-control domain; absent from world semantics | Control-store lifetime |
| `RunAttemptControl` | Durable active/reserved/finalized gate selecting one physical attempt's unique trajectory prefix | Run attempt |
| `AttemptBinding` | Permanent binding among authority domain, run attempt, execution specification, initial root, and semantic epoch lineage | Run attempt |
| `AttemptArtifactRetention` | Crash-safe attempt-owned, retained-run-owned, or discarded artifact-pin state | Run attempt retention |
| `AttemptControlTraceArtifact` | Captured replay inputs plus regenerated control observations used for verification | Finalized run |
| `TerminationView` | Stage-checked projection of only the semantic facts a termination contract may read | One serialized attempt barrier |
| `WorldSnapshot` | Immutable read image at one authoritative revision | Until superseded |
| `ActorViewK` | Projection-safe actor-relative component assembled into `PolicyPayloadK` | One lifecycle invocation |
| `LifecycleReadWitnessK` | Specification name for a projector-owned, port-specific private freshness witness; `ActionReadWitness` is the first concrete type | Retained lifecycle invocation |
| `PreparationReadEvidence` | Runtime-owned evidence for authoritative reads used by one prepared transaction | One prepared step; digest may enter history |
| `Belief` | Accepted actor-relative epistemic proposition | Until revised or retracted |
| `ActorSocialInterpretation` | Accepted actor-scoped social meaning | Until revised or retracted |
| `IntersubjectiveClaim` | Accepted record that identified parties asserted, declared, or committed something | Declared social-rule lifetime |
| `InstitutionalFact` | Rule-accepted status authoritative in a declared institutional scope | Institution-rule lifetime |
| `AppraisalResult` | Derived interpretation paired privately with its invocation envelope | One invocation or typed continuation |
| `Intent` | Accepted commitment to a desired condition | Many action cycles |
| `Activity` | Accepted controller state pursuing an intent | Many action cycles |
| `ActionOpportunity` | Accepted one-shot permission for one action disposition | Until consumed |
| `ActionOpportunityView` | Projection-safe policy view of an open opportunity | One action decision |
| `GroundedActionCandidate` | One actor-visible action with complete bindings | One action decision |
| `CommandEnvelope` | Concrete request for runtime authority | One attempt |
| `ProcessInstance` | Authoritative time-bearing world mechanism | Until completion/interruption |
| `AttemptRecord` | Durable accepted or rejected runtime attempt | Durable history |
| `AuthorityRecord` | Atomic `Admission(Commands | ActionEvaluation)`, moment, or management publication | Durable history |
| `WorldCheckpoint` | State-complete recovery root bound to an artifact closure | Durable |
| `DecisionTrace` | Non-authoritative explanation and provenance graph | Configurable retention |

## Authority model

### Authoritative session partitions

The authoritative session contains logically separate state partitions:

```text
Domain state
  physical, spatial, resource, inventory, possession, and mechanical-control
  facts

Epistemic state
  accepted actor-relative evidence records, beliefs, uncertainty, memory
  references

Social state
  accepted actor-scoped social interpretations, intersubjective claims,
  relationships, obligations, reputations, and institutionally authoritative
  facts

Agency state
  intentions, activities, focus, and interruption state

Runtime-control state
  process instances, reservations, resolution tier, cancellation generations
  typed lifecycle continuations and open action opportunities
  pending evaluator invocations
  input, management-request, and command deduplication ledgers
  coalescing/debounce generations and bounded retry/fallback dispositions

Scheduler state
  ordered future triggers and current simulation moment
```

These partitions share one revisioned session head, but not one unrestricted
mutation API.

Runtime control is only a logical storage partition. It must not become a
generic lifecycle value map: every record family has one protocol owner, typed
schema, explicit state machine, and removal rule.

### Social and epistemic truth classes

Social semantics use four non-interchangeable accepted value classes:

| Value class | Meaning | Commit owner |
|---|---|---|
| `Belief` | One actor's evidence-supported proposition; it may be false, stale, or uncertain | Epistemic gate |
| `ActorSocialInterpretation` | One actor's accepted social reading of an event or relation, such as insult, betrayal, or perceived legitimacy | Social gate, actor-scoped |
| `IntersubjectiveClaim` | A recorded assertion, declaration, promise, or commitment among identified parties; it proves that the act occurred, not that its proposition is true | Social gate |
| `InstitutionalFact` | A status constituted and accepted under installed rules in a declared jurisdiction, such as membership, office, recognized title, or obligation | Social gate |

An `InstitutionalFact` is authoritative within its declared institutional
scope, not a physical fact and not every actor's belief. An
`IntersubjectiveClaim` becomes an institutional fact only through an explicit
rule-checked social transition. Different `ActorSocialInterpretation` values
may coexist for the same event, claim, or institutional fact.

Domain state retains physical possession and mechanical control. Social
ownership, entitlement, permission, and title are claims or institutional
facts. A later domain transition may read those accepted social facts through
an explicit typed contract, but neither the social gate nor a social evaluator
applies a physical effect.

### Typed commit gates

Every accepted transition passes through a typed runtime gate:

| Gate | May change | Must not infer |
|---|---|---|
| Domain causal gate | Physical/systemic domain state and domain process/control state | Belief, social interpretation, claim, or institutional status |
| Epistemic gate | Evidence and `Belief` state for named actors | Hidden domain truth, another actor's interpretation, or institutional acceptance |
| Social gate | Actor-scoped interpretations, intersubjective claims, relationships, and institutional facts | Physical effects or a proposition's domain truth |
| Agency gate | Intent/activity and lifecycle-control state | Runtime action success |
| Resolution gate | Active representation and conversion evidence | Unrecorded reconstructed history |

A commit may coordinate more than one partition only through an explicit,
verified transaction type. There is no generic `mutate_world` escape hatch.

Scheduler and kernel-level runtime-control protocol operations are kernel-owned and
orthogonal to semantic gates. Every prepared subtransaction declares a
transaction kind and collects receipts from each participating semantic gate;
the kernel separately verifies its scheduler/control delta. This lets an
epistemic or agency transition atomically schedule required future work without
granting that gate general scheduler authority.

Possessing a value that resembles state is not authority. Authority is the
private capability to replace the current `WorldSession` head and append the
matching authority record. Read snapshots, evaluators, packs, and adapters never
receive that capability.

### Proposal and acceptance

The architecture uses a consistent pattern:

```text
pure component output
    -> typed proposal or selected stable ID
    -> freshness and schema validation
    -> policy acceptance
    -> prepared transaction
    -> invariant validation
    -> atomic commit
```

This is true for an action, a belief revision, a social interpretation, an
intent change, and a resolution transition. Different gates own different
validity rules, but all accepted changes remain explicit and journaled.

The social gate accepts a closed `SocialTransitionProposal`. An
`ActorSocialInterpretationProposal` produced by the optional social lifecycle
can change only actor-scoped interpretation state. Intersubjective claims and
institutional facts enter through their own typed, provenance-bearing social
act or institution-rule transitions; the interpretation evaluator cannot mint
them.

## Logical components

### `AuthoringCompiler`

Owns:

- pack parsing and source diagnostics;
- exact dependency-graph resolution before imported-name checking;
- import/name resolution against exact export and interface digests;
- source-level resolved and typed declarations;
- complete lowering to family-specific executable IR;
- authority and stage checking;
- construction of `ArtifactData`, defs-owned validation against its supplied
  `SemanticInterfaceCatalog`, and deterministic artifact encoding;
- final artifact-digest locking;
- cross-pack linking into a process-independent `RuntimeDefinitionSet`.

It receives a `SemanticInterfaceCatalog`. It does not own live state, runtime
implementations, process-local registry activation, or checkpoint migration.

### `EngineDistribution` and semantic interface catalog

The composition root assembles statically linked trusted semantic extension
bundles into one immutable `EngineDistribution`.

It exposes:

- a serializable `SemanticInterfaceCatalog` of primitive signatures,
  family/stage legality, authority and effect constraints, current structural
  or cost rules, interface versions, and exact interface digests;
- the matching runtime implementations for engine installation;
- exact identities for each semantic interface and implementation, from which
  the required semantic binding set is selected;
- the engine build identity needed for verification reruns.

Pack compilation binds only the exact transitive semantic interfaces it uses.
Session creation accepts an installed semantic superset when every required
interface resolves to the matching implementation. Adding an unused primitive
does not change a pack or definition-set identity.

Storage, indexing, and result-equivalent acceleration are infrastructure
providers, not pack-visible semantic extensions. If an implementation can
change a logical result or tie-break, it is a semantic policy and must appear
in the execution-semantics identity. Exact whole-build identity remains
provenance and may be retained for strict reproduction.

Activation binds a `RuntimeDefinitionSet` to the installed implementations and
constructs an `ActivatedDefinitionRegistry`. Its intern IDs, indexes, caches,
and implementation pointers are reconstructible and excluded from durable
definition identity.

### `RunAttempt`

The host-facing execution capability for one physical attempt. It owns:

- atomic create-or-open under one immutable `AttemptBinding`;
- pinning of the exact resolved execution artifact closure for every live
  attempt;
- crash-safe handoff to a retained run root or explicit terminal discard;
- serialized reserve/reconcile/finalize control;
- exposure of the runtime-owned projection and evaluation of the configured
  termination contract;
- typed, idempotent attempt cancellation;
- revocation of world-mutation access after finalization.

Logical policy belongs to the host/engine plane, while the durable
`RunAttemptControl` state machine, control ledger, and atomic
`StepPublicationReceipt` coupling are implemented behind the runtime
persistence boundary. This co-location permits crash-safe create/publication
protocols without letting `world-runtime` depend on cognition or research
types.

### `WorldSession`

The stable engine facade for one authoritative world.

Owns or coordinates:

- exact `ExecutionSpec` and normalized execution-semantics identity;
- exact `InitialStateRoot` referenced by that specification;
- exact `EngineProtocolVersion`, `LifecycleProfiles`,
  `ExecutionConfigArtifact`, and `SemanticImplementationSet` resolved from that
  identity;
- immutable process-independent `RuntimeDefinitionSet`;
- process-local `ActivatedDefinitionRegistry`;
- current authoritative revision;
- runtime kernel and scheduler;
- lifecycle coordinator;
- checkpoint and history cursors;
- deterministic advancement and external ingress behind an active
  `RunAttemptControl`;
- inspection handles.

It does not expose mutable stores or an ungated public mutation API.
`RunAttempt` supplies the host-facing capability for `advance`, `drain_until`,
system-command and captured-action-evaluation admission, and management while
its durable control state is active.
Finalization revokes that capability without changing the session's health
mode or authoritative state.

### `RuntimeKernel`

Owns:

- virtual time and deterministic scheduling;
- action and process request validation;
- primitive effect evaluation;
- conflict resolution;
- prepared transactions and invariant checks;
- atomic same-moment commit batches;
- atomic admission and management publications;
- all typed commit gates;
- process and reservation control;
- command idempotency;
- durable authority-record history;
- stage-checked `TerminationView` projection and the verified termination
  interpreter at root, after every publication, and during reconciliation;
- immutable snapshot publication.

It knows nothing about how a decision policy selected a candidate.

### `ContextProjector`

Builds immutable, actor-relative, lifecycle-specific inputs from a snapshot,
definitions, and accepted actor knowledge.

It owns:

- perception-safe queries;
- evidence and provenance projection;
- availability/completeness reporting;
- affordance and capability projection;
- grounded intent and action candidate generation;
- invalidation dependencies.

It cannot commit state.

### `ObservationProjector` and evidence assimilation

The observation projector maps committed domain events and perception-safe
state changes into actor-addressed `EvidenceDelivery` proposals:

- it applies observation and visibility definitions;
- it preserves the causal event, observer, source, time, and uncertainty;
- it may produce different evidence for different actors;
- it does not assert that an actor accepted evidence or a belief.

A pure evidence assimilator compares a delivered value with accepted epistemic
state and emits one `EpistemicTransitionProposal` containing the delivery
disposition, accepted `EvidenceRecord` delta, and belief delta. The coordinator
routes that proposal to the epistemic gate. This keeps delivery, provenance,
conclusion, and acceptance distinct without producing two ambiguous epistemic
commits.

### `PostCommitRouter`

Consumes a durable self-contained `ReactionEnvelope` carried by
`PostCommitDispatch`. It invokes observation projection and context
dependency analysis, then emits evidence, invalidation, coalescing, and
lifecycle-wake proposals for a later runtime batch.

It is pure and engine-owned. An empty envelope creates no dispatch, and
consuming an envelope creates another only when new accepted changes produce a
nonempty envelope. Runtime does not depend on context, while context does not
mutate the authoritative scheduler.

### `LifecycleCoordinator`

Consumes typed scheduler triggers and invokes the appropriate lifecycle only
when required.

It is operationally stateless. It deterministically performs:

- per-actor work normalization described by accepted control state;
- appraisal, intent, activity, and action invocation order;
- construction of a private lifecycle invocation envelope and a separate
  projection-safe policy payload for every actor-facing port;
- prepared-snapshot and expected-version checks, plus dependency-witness
  freshness checks for retained or deferred work;
- local payload reconstruction and private envelope rebinding without policy
  reinvocation when only hidden metadata changed;
- retention of private candidate-resolution tables used after a policy selects
  an actor-safe ID;
- routing proposals to the correct commit gate;
- lowering a selected candidate to a runtime request;
- consuming engine-private attempt resolutions and scheduling exactly one
  profile-timed neutral, actor-safe sponsor wake per submitted opportunity;
- explicit fallback and abstention handling.

Every value that can change later behavior—pending invocation identity,
eligibility, coalescing generation, retry budget, fallback state, cached
`AppraisalResult` payload/fingerprint retained across a causal boundary, or
pending result—lives in a concrete typed runtime-control continuation or
scheduler record and survives checkpoints. Only within-invocation work and
reconstructible performance caches remain inside the coordinator.

### Controllers

Player, AI, script, and test controllers use the same actor control boundary.
The engine presents an `ActorControlFrame` containing a projection-safe
`ActorInputFingerprint` and grounded actor-visible candidate set. Global
revision, raw trigger provenance, dependency witness, and private candidate
resolution data remain with the coordinator. A ready controller decision is
exactly selection of a supplied candidate ID or `NoApplicableAction`.
Inline versus captured-deferred execution is selected independently of that
decision. Cancellation, timeout, capture, reinvocation, and fallback are
runtime invocation-control transitions; a policy error is a trusted
coordination failure, not another action choice.
Waiting, suspension, retry, and intent reconsideration are later activity or
intent directives. The response is bound to the exact frame by the actor-safe
`ActorInputFingerprint` or an equivalent actor-safe opaque invocation token;
an old frame cannot select against a newer private candidate table.

`ActionPolicy` is the automated implementation of this controller role. A UI
may present the same candidate set to a player. Neither receives mutation
authority, and both pass through trusted lowering and runtime revalidation.
Every trajectory-affecting controller implementation is selected by
`LifecycleProfiles`, included in the exact `SemanticImplementationSet`, and
sealed into `ResolvedExecution` before activation.

Session setup, migration, and privileged host administration use separate
explicitly authorized request families and authority paths; they do not
masquerade as actor actions.

### Lifecycle evaluators

Stable production ports are:

```text
EvidenceAssimilator
AppraisalEvaluator
SocialInterpretationEvaluator    optional by domain/profile
IntentPolicy
ActivityController
ActionPolicy
```

Each consumes one projection-safe immutable typed payload and returns one
bounded typed result. A future research implementation may use a configurable
pass graph inside one port, but the current generic runner is not migrated and
a pass graph is never the engine's outer lifecycle or plugin API.

`ActivityController` owns both initialization and advancement. Planning,
search, behavior-tree execution, or scripted sequencing remains internal until
a concrete scenario justifies a separately substitutable planner port.

### `ExperimentRunner`

Owns immutable study/experiment plans, run-case expansion, execution
specifications, attempt/trajectory records, capture-compatible run reuse,
independent run parallelism, analysis manifests, metric computation, and
comparisons.

It uses the same `RunAttempt` API as other hosts. Metrics observe completed
artifacts and never influence simulation behavior.

### `Inspector`

Provides read-only queries such as:

- why an action was selected or unavailable;
- which context inputs were missing;
- why an intent changed;
- which runtime condition rejected a request;
- which commit changed a value;
- how two runs diverged.

It joins causal history and decision traces without conflating them.

Inspection is capability-scoped. Operator and research inspection may include
authoritative truth; actor-facing explanations are redacted through the same
knowledge boundary as normal context and cannot become a side channel.

### Presentation and narrative projection

Presentation adapters consume audience-scoped actor views, domain events, and
permitted explanation artifacts to produce text, UI models, animation cues, or
narrative framing.

They are derived views with no mutation authority. If a concept such as a
reputation, promise, quest state, or public declaration affects future rules,
it must exist as accepted domain/social state and committed events—not only as
generated prose.

## Scheduled causal loop

```mermaid
sequenceDiagram
    participant S as Scheduler
    participant E as LifecycleCoordinator
    participant C as ContextProjector
    participant P as Lifecycle Policy
    participant R as RuntimeKernel
    participant J as Atomic Authority Store

    S->>E: Typed trigger at SimMoment
    E->>R: Request immutable snapshot
    R-->>E: WorldSnapshot(revision)
    E->>C: Build lifecycle context
    C-->>E: Policy payload + private build material
    E->>P: Evaluate projection-safe payload
    P-->>E: Proposal / selection / abstention
    E->>E: Validate private envelope and output shape
    E->>R: Typed command or accepted-update proposal
    R->>R: Validate, prepare, resolve, verify
    R->>R: Seal one AuthorityRecord including exact scheduler delta
    R->>J: append_and_publish(expected head, sealed record, reservation)
    J-->>R: Committed new head + StepPublicationReceipt
    J-->>S: Same head exposes scheduler state
    Note over R,J,S: State, history, scheduler, and revision publish atomically
    R-->>E: Engine-private attempt resolution
```

An acceptance or rejection is explicit authority history and trusted protocol
input. It may trigger profile-fixed fresh projection and bounded
reconsideration, but raw status, reason, retryability, revision, and
record/process references never enter actor-facing controllers. Observable
meaning returns through the reaction, observation, and evidence path. Wake
presence, timing, generation, and visible cause are outcome-independent under
the same actor-visible sponsor state. Retry is never recursive or unbounded.

The sequence runs inside one durable `RunAttemptControl::StepReserved` gate.
After the committed head returns, the gate evaluates the pure termination and
finalization rule before another scheduled or admission transition may begin.

Ordinary commands use `Admit` to publish
`Admission(Commands(IngressBatchRecord))`. Captured action decisions use the
other concrete member,
`Admission(ActionEvaluation(ActionEvaluationAdmissionRecord))`, with their own
identity ledger; they are not wrapped as commands. Authorized management uses
the independent, scheduler-independent `Manage` path. The sequence above is
the `Fire` path after an ordinary delivery or lifecycle trigger is already
authoritative.

## Dependency direction

The target workspace remains domain-shaped:

```text
world-core
  shared IDs, virtual time, revisions, dependency keys, provenance,
  content hashes, diagnostics

world-defs -> world-core
  executable definition and IR families, durable definition keys,
  serializable semantic-interface descriptors

world-model -> world-core, world-defs
  immutable state records, read models, query contracts, and shared durable
  lifecycle-protocol record schemas

world-runtime -> world-core, world-defs, world-model
  authoritative session state, scheduler, execution configuration,
  transactions, attempt/commit history, durable attempt-control records,
  publication receipts, and verified termination interpretation

world-standard -> world-core, world-defs
  optional reusable standard definition vocabulary

world-standard-runtime
  -> world-core, world-defs, world-model, world-runtime, world-standard
  trusted standard primitive implementations

world-context -> world-core, world-defs, world-model
  actor-relative projections and grounded intent/action candidates

world-decision -> world-core, world-defs, world-context
  pure evidence/lifecycle evaluator ports and baseline implementations

world-authoring -> world-core, world-defs
  source forms, compiler, artifacts, lock resolution, source/artifact
  evolution and future target-schema upgraders

world-engine
  -> world-core, world-defs, world-model, world-runtime,
     world-context, world-decision
  stable builder, run-attempt/session facade, lifecycle coordination,
  verification, inspection, and offline child-root migration

world-lab -> world-core, world-engine, world-authoring
  scenarios, experiment plans, metrics, comparisons, trace export

world-cli
  -> world-authoring, world-engine, world-lab,
     world-standard, world-standard-runtime
  product composition root, trusted bundle selection, offline tooling

world-conformance
  -> world-authoring, world-engine,
     world-standard, world-standard-runtime
  test-only black-box architecture scenarios and dependency checks
```

`world-conformance` exports no production API and is never a dependency of
another package. The exact production package count may change where Rust
privacy makes a boundary weaker.
The logical dependency rules may not:

- runtime never depends on context or decision;
- decision never depends on runtime or unrestricted model state;
- packs never depend on live runtime mutation;
- the optional standard vocabulary is selected at the composition root rather
  than imported by generic context or authoring;
- standard definitions and trusted primitive implementations remain separate;
- only engine orchestration sees both decision outputs and runtime commands;
- research tooling is a leaf.

Runtime history schemas belong to `world-runtime`. Evaluator-local trace
fragments belong to `world-decision`; `world-engine` owns the normalized
lifecycle trace envelope and Inspector query schema. `world-lab` consumes those
engine surfaces and cannot become a dependency of production execution.
Study, metric, and analysis schemas belong to `world-lab`, not `world-defs`.
Cross-package conformance scenarios and exact dependency allowlist tests belong
to `world-conformance`, not to a lower production package.

`world-core` and `world-defs` are not miscellaneous shared-code bins. A concept
enters either crate only when multiple lower layers need a stable semantic
contract. Source-only forms stay in `world-authoring`; policy internals stay in
`world-decision`; process-local indexes stay behind activation.

`world-authoring` may upgrade source and compiled artifacts between explicitly
supported target-era schemas only. An offline
`CheckpointMigrationEngine` belongs with the engine host boundary because it
needs prior and successor target-era runtime definitions plus model/runtime
schemas. It consumes a supported prior-target-schema checkpoint and exact
artifact closures and emits a new child-epoch root checkpoint with provenance;
it never imports a pre-redesign format or mutates a live session.

Reliable adapter dispatch is coordinated by `world-engine`, but the
compaction-sensitive source-to-outbox handoff belongs to `world-runtime`.
Runtime holds an opaque committed-history lease until it has durably installed
a verified self-contained delivery root, and it fences those leases and roots
during archive generation. Engine owns adapter cursors, dispatch, and
acknowledgements, but receives no raw history-pin release or compaction
capability. These operations are outside `Σ` and cannot become another world
transition path.

Durable lifecycle records have one implementable owner according to their
meaning. `world-model` defines only shared accepted semantic lifecycle
schemas—such as intent, activity, action-opportunity, and process records—that
must appear in immutable snapshots, typed semantic deltas, or lower-package
queries. `world-runtime` defines and stores nonsemantic coordination and
control schemas, including typed lifecycle-continuation envelopes, pending
evaluator invocations, captured-result and capture-ledger records, frontier
blockers, cancellation generations, retry/fallback dispositions, and
scheduler integration. Runtime stores accepted model records and alone
enforces their authoritative transitions, but that storage role does not move
their semantic schemas into runtime.

`world-context` owns projection material and concrete projection witnesses;
`world-decision` owns evaluator-local algorithms and results; and
`world-engine` coordinates the typed handoff without durably owning either
semantic state or invocation control. Built-in persistent semantic controller
state uses concrete model types. Only genuinely implementation-defined
evaluator state may use a bounded, canonical, versioned sealed payload tied to
one exact lifecycle port and semantic implementation; runtime validates its
binding, schema, size, expected version, and replacement rule. This is not a
generic key/value blackboard, and it does not let runtime depend on context or
decision.

The authoritative session head and its commit capability must reside behind one
compile-time-enforceable boundary. If separating storage and runtime into
crates requires public mutation APIs, the storage implementation belongs
inside `world-runtime`; `world-model` should then expose only immutable records
and query contracts.

The normative physical module layout, top-level type ownership, Rust
visibility, and concrete facade signatures are defined in
[Target Rust Code Architecture](code-architecture.md).

## Public API shape

The intended high-level surfaces are deliberately small:

```text
EngineBuilder
  use exact EngineDistribution
  use ArtifactResolver
  use opaque RuntimeService
  build Engine

Engine
  resolve_execution(ExecutionSpec) -> ResolvedExecution
  start_attempt(ResolvedExecution, attempt key) -> RunAttempt
  restore_attempt
  open_archive
  branch
  inspect_capabilities

RunAttempt
  submit_system_command
  pending_action_evaluations
  capture_action_evaluation_result
  submit_management_request
  cancel_attempt
  advance
  drain_until
  inspect_finalization
  inspect

WorldSession
  checkpoint
  inspect

ExperimentRunner
  validate
  run
  compare

CheckpointMigrationEngine
  validate_migration
  migrate_to_child_epoch

Inspector
  explain_action
  explain_unavailability
  inspect_lifecycle
  inspect_commit
  diff_runs
```

`resolve_execution` loads the referenced `InitialStateRoot` and exact resolved
artifact set, verifies the normalized `ExecutionSemanticsManifest`
including engine protocol, definitions, lifecycle profiles, configuration, and
semantic implementation bindings against the installed distribution, and
performs `RootCompatible`. It treats serialized `ExecutionSpec` and
`TerminationContract` bytes as untrusted: it recomputes canonical identity,
checks schema and clause ordering, proves bounded evaluator shape, verifies
every declared `TerminationView` read and stage, closes semantic-interface
requirements against the manifest, and rejects unknown operations or versions.
`ResolvedExecution` is the sealed engine-owned result of those checks, not
another open configuration bag. Session creation cannot accept an unresolved
digest, unchecked contract, or independently supplied component set. Before
or atomically with attempt creation, an immutable
`ResolvedExecutionClosureManifest` records and pins that same resolved set.
It excludes run-produced control, input, history, and result artifacts.
Checkpoints, run artifacts, and portable archives use separate frozen,
root-relative `ArtifactClosureManifest`s that include the resolved closure;
neither manifest is a second construction gate.

`start_attempt` durably creates or opens the engine-owned session and
`RunAttemptControl` together, keyed by `RunAttemptId`. Its canonical
`AttemptCreationDescriptor` retains the complete binding, raw runner-assigned
attempt key, root cursor, `ResolvedExecutionClosureManifest` digest, and
format identity. The control store supplies its permanent
`AttemptAuthorityDomainId`; load rederives
`RunAttemptId = H(domain || ExecutionSpecId || attempt key)` and the creation
fingerprint from that descriptor. An exact creation retry returns the same
pair; any different descriptor or fingerprint under that ID fails closed.
Initial creation evaluates the root `TerminationView` and exposes either an
atomically installed active or already finalized initial pair, never an
unchecked active capability.
Every call that can change the authoritative world reserves one exact
authority cursor, permits at most one `Admit`, `Fire`, or `Manage` with a
matching atomic
`StepPublicationReceipt`, and reconciles the projection-safe
termination/finalization decision before another world call can enter.
Cancellation, ledger compaction, and artifact-retention transitions mutate
only the separately durable control plane through their own typed
compare-and-set protocols.

`restore_attempt` opens and reconciles that same durable control record in its
original persistence domain. It never clones a writable control plane from a
portable archive. Every portable copy, including a finalized one, is
read-only; continuing an active/reserved snapshot creates a child root and new
attempt unless a future exclusive fenced-transfer protocol is deliberately
added.

No public API accepts an arbitrary callback with mutable world access.

## System-wide invariants

1. Only the runtime commit capability can advance authoritative revision.
2. Every authoritative revision transition publishes one atomic admission,
   moment, or management authority record.
3. One due `SimMoment` is resolved from one base snapshot into one
   `MomentBatchRecord`; external admission is a separate typed transition.
4. Decisions consume projection-safe actor-relative immutable payloads; the
   coordinator privately binds them to one authority head and dependency
   witness.
5. Hidden truth cannot enter decision inputs through feasibility checks,
   diagnostics, or candidate omission.
6. Evaluators return bounded port-specific decisions selecting supplied stable
   IDs, never mutations.
7. Runtime deduplicates before logical admission and revalidates every
   genuinely new command against current authoritative state.
8. Intent and activity survive unrelated action cycles.
9. A process is runtime truth; an activity may request it and react through
   actor-visible monitors/evidence but cannot impersonate it.
10. Definition sets are immutable within a reproducible session epoch;
    activated indexes are reconstructible.
11. Ordering, randomness, external inputs, and evaluator results have explicit
    reproducibility contracts.
12. Higher cognition can be disabled while a basic actor can still select and
    submit legal actions.
13. Research instrumentation and wall-clock telemetry cannot affect simulated
    outcomes.
14. Lifecycle coordination has no uncheckpointed outcome-affecting mutable
    state.
15. Every runtime definition binds its exact required semantic-interface
    closure; every session binds one normalized execution-semantics manifest.
16. A post-commit dispatch exists if and only if its self-contained reaction
    envelope is nonempty.
17. Raw attempt/process outcomes remain engine-private; actor-facing wake
    presence, timing, generation, cause, and payload satisfy knowledge
    noninterference.
18. Global revision, raw moment/cause, dependency stamps and witness,
    authority-derived IDs, and private diagnostics do not enter policy
    payloads. Actor-visible IDs, ordering, and fingerprints are stable under
    actor-indistinguishable hidden-state changes.
19. Hidden-only witness or legality changes cannot alter logical
    policy-invocation/dispatch presence, timing, or generation. A new
    invocation requires a changed projection-safe payload, an explicit
    evaluator-binding change in a child branch/epoch, or another actor-visible
    configured cause.
20. Session creation accepts only a sealed `ResolvedExecution` whose exact
    specification, initial root, semantics, resolved artifacts, and activation
    binding passed `RootCompatible`.
21. Every host call that can mutate the authoritative world is scoped to one
    active `RunAttemptControl`; one attempt finalizes at most once at a
    deterministic authority cursor and cannot advance past it. Typed
    control-only housekeeping cannot change that cursor or the world.
22. Every active, reserved, finalized, archived, or restored attempt-control
    record retains one exact `AttemptBinding`; physical attempt identity never
    enters world semantic hashes or random keys.
23. Every fresh writable attempt domain has a unique
    `AttemptAuthorityDomainId`; same-domain create-or-open is linearizable and
    portable copies receive no writer capability.
24. Every active or reserved attempt is a durable retention root for the exact
    immutable resolved-execution closure in its creation descriptor. Dynamic
    run dependencies have independent authority/scheduler/delivery retention
    roots. Finalized handoff pins the retained target before releasing source
    pins; terminal discard installs a permanent descriptor/fingerprint
    tombstone before garbage collection.
25. Verification drives only canonical creation, ordered host step intents,
    and logically anchored captured control inputs. Reservations, receipts,
    reconciliation, termination selection, and finalization are regenerated
    observations.
26. A session archive canonically fingerprints its exact root-relative
    closure and attempt-control-plane snapshot; portable import activates
    neither control nor reliable delivery.
