# Target Architecture Decision Record

## Status

Accepted for the target architecture. Implementation details may evolve, but a
change that contradicts one of these decisions requires an explicit
replacement decision and an update to the validation scenarios.

## D-001: Simulation-first production engine

`world` is a headless, domain-shaped RPG simulation engine first and a research
platform second.

Research capabilities use the production boundaries. They do not create a
second runtime or a privileged experimental mutation path.

## D-002: Runtime capability is the only mutation authority

Only the runtime's private commit capability can replace the current
authoritative session head.

All other components receive immutable values and return proposals, selections,
or prepared data. There is no public mutable store, generic system callback, or
pack-level mutation hook.

## D-003: Authoritative state is partitioned by semantic authority

Domain, epistemic, social, and agency state are accepted semantic partitions
with typed commit gates. Runtime-control and scheduler state are distinct
authoritative operational partitions with kernel-owned typed protocols.

A physical effect cannot directly assert an actor's belief or social meaning.
An appraisal cannot directly change physical truth. Cross-partition
transactions are explicit and verified.

## D-004: Cognition is a set of lifecycles, not one pipeline

Evidence assimilation, appraisal, optional domain/profile social
interpretation, intent reconsideration, activity
initialization/advancement, and concrete action choice have different
triggers, cadence, persistence, and budgets.

The engine exposes typed lifecycle ports. A configurable pass pipeline may
implement one port for research but is not the outer engine architecture.

## D-005: Intent and activity persist; action selection is immediate

Intent records why an actor remains committed. Activity records how the actor
is pursuing that intent. Concrete action selection chooses the next attempt.

Planning and search are internal to `ActivityController`. Any method or plan
state that can affect a later invocation is part of its versioned controller
state; only reconstructible within-invocation work may remain transient. A
runtime `ProcessInstance` remains distinct from cognitive activity state.

The `ActivityController` owns both initialization after intent adoption and
later advancement. Its planning/search algorithm stays private. It proposes an
updated semantic state plus one directive and wake proposal; the coordinator
binds that result to the expected version in its private invocation envelope.
Actor-initiated process start/control is initially a grounded action, not
executable structure invented by the controller.

There is no separate persistent goal object in the initial model. An intent
contains its desired condition. An accepted
`OpenActionOpportunity(ActionScope)` directive may atomically create one
accepted one-shot `ActionOpportunity`; the scheduler's action-ready trigger
only references that durable opportunity.

## D-006: Decisions use actor-relative immutable inputs

For every lifecycle, the trusted coordinator binds the invocation to one
authority head, global revision, raw cause, and dependency witness in an
engine-private envelope. The evaluator consumes only the separate
projection-safe actor-relative payload.

`Unavailable` and valid empty data are distinct. Projection-specific reduced
views require explicit omission semantics rather than a universal shallow
status. Hidden truth cannot leak through candidates, feasibility, score
features, diagnostics, raw attempt/process resolutions, or outcome-derived
opaque IDs. Runtime success or failure becomes actor-visible only through a
declared observation contract and accepted evidence; controllers otherwise
receive exactly one profile-timed neutral attempt-resolution wake. Wake
presence, timing, generation, and cause obey the same noninterference rule as
payloads, as do logical policy-invocation and dispatch presence, timing, and
generation. Global revision, raw moment and cause, dependency stamps and
witness, authority-derived IDs, and private diagnostics are not policy
metadata. Actor-facing IDs and fingerprints remain stable across
actor-indistinguishable hidden-state changes. Raw process progress/completion
is not a controller cause.

## D-007: Intent and action policies select grounded candidate IDs

The context layer produces bounded actor-relative
`GroundedIntentCandidateSet` and `GroundedActionCandidateSet` values with
complete bindings. Policies select stable candidate IDs and cannot invent
definition predicates, policy references, or bindings.

Trusted engine code lowers the selected candidate to a command. Runtime
revalidates it against current authoritative state.

## D-008: Initial action-decision semantics remain minimal

The first shared decision representation is action meaning tags, base priority,
and explicit signed score contributions.

A richer action or utility DSL may later live behind the same candidate and
policy contracts. The core does not predict its full instruction set now.

## D-009: Virtual time uses deterministic discrete events

The global kernel uses quantized integer simulation time plus microsteps and a
serializable future-trigger scheduler.

Compiled definitions retain durations in a canonical
configuration-independent exact unit. Session creation validates exact
representation under the selected time quantum; the initial contract does not
round.

Fixed-step numerical components are subordinate scheduled subsystems. There is
no mandatory global actor tick, and wall-clock time never defines authoritative
causality.

## D-010: One atomic authoritative publication lane

All work at one `SimMoment` reads one base snapshot. Validation and effect
evaluation produce internal prepared transactions, which conflict resolution
combines into one `SameMomentCommitBatch`.

Every revision publishes one outer `AuthorityRecord`: an `IngressBatchRecord`,
`MomentBatchRecord`, or `ManagementBatchRecord`. A moment record atomically
records every accepted/rejected attempt, changes accepted state and runtime
control, consumes and schedules work, and advances revision. It contains a
self-contained `ReactionEnvelope` only when observable or reliably deliverable
consequences exist; only a nonempty envelope schedules post-commit dispatch.

Pure projection and evaluation may run in parallel. Conflict resolution and
authoritative publication remain canonically ordered.

Input, host-management, and command requests use typed, non-reusable
idempotency namespaces with monotonically issued IDs. Full outcomes may compact
after their acknowledgement horizon only into a permanent contiguous
non-reuse frontier: an older retry is rejected as `DuplicateExpired`, never
treated as new work. Exact retained retries short-circuit before
current-state validation. Advancing that frontier is an authoritative ledger
delta, not wall-clock garbage collection.

## D-011: Persistence is checkpoint-centered

The recovery model is:

```text
state-complete WorldCheckpoint
  + exact root-relative ArtifactClosureManifest including the canonical
    ExecutionSpec
  + compactable committed tail
```

Domain events are not forced to be permanent storage reducers. Checkpoints
include all accepted state, processes, scheduler work, compatibility metadata,
pending lifecycle/external state, and an exact hashed history cursor.
Their canonical state fingerprint covers the complete authoritative
checkpoint projection plus semantic/format identities; install compares it
with the projection recomputed from the expected immutable head, not merely
with a caller-supplied cursor.

The backend must implement one crash-safe
`append_and_publish(expected_head, sealed_authority_record,
sealed_step_reservation)` linearization point. The resulting head is the
canonical deterministic application of that record to the expected head, not
an independent caller-supplied value. The same point stores the non-semantic
`StepPublicationReceipt` used for run-attempt crash reconciliation.
Recovery can expose the complete old head or complete new head, never a mixed
state/history/scheduler revision.

Pending scheduler work, every live `RunAttemptControl`, and reliable
external-delivery obligations are retention roots. Attempt creation pins the
exact resolved execution artifact closure before or atomically with publishing
an active control record. A reliable adapter either pins required history
with its durable cursor or uses a self-contained transactional outbox until
acknowledgement; compaction may invalidate none of these obligations.

## D-012: Restoration, verification, and branching are different operations

Restoration applies recorded committed results. Verification reruns compatible
logic and compares fingerprints. Counterfactual branching creates new immutable
lineage from a retained cursor.

No mode silently calls external or nondeterministic evaluators during replay.
Definition changes and persistent-policy replacements require a migrated or
reset child epoch rather than reinterpretation in place.

## D-013: Determinism is manifest-scoped and randomness is keyed

The engine promises identical logical results under a declared execution
semantics manifest and one immutable `ExecutionConfigArtifact`. Persistence
readability and analysis-format compatibility are separate questions.

Random draws are keyed by logical purpose and identity rather than taken from
one mutable global stream. Ordering never depends on hash iteration, host
completion order, wall time, or allocation order.

External ingress is admitted only at serialized barriers. The returned
`AdmissionFrontier` seals earlier moments against backdated input.

## D-014: Multi-resolution keeps one active authoritative representation

An entity has a canonical core and exactly one active tier-specific
representation per declared `ResolutionScopeId`.

Promotion and demotion are checked transactions with declared invariants,
versioned conversion semantics, scheduler replacement, and recorded
approximation. Individual background simulation precedes population
aggregation.

## D-015: Packs compile into immutable exact artifacts

Source packs compile through family-specific checked IR into immutable,
content-addressed artifacts, an exact dependency lock, and a
`RuntimeDefinitionSet`.

Resolution first fixes an exact package/source graph. Compilation or artifact
loading and reverification then produces exact artifact digests, after which
`PackLock` is finalized and the artifacts are linked into a
process-independent `RuntimeDefinitionSet`. Process-local activation builds a
reconstructible `ActivatedDefinitionRegistry`.

Durable definition identity is pack-qualified. Process-local numeric interning
is never persistent identity. Compilation records the exact required semantic
interface closure; activation binds that closure against the
`SemanticInterfaceCatalog` and implementations supplied by an
`EngineDistribution`. Unused installed interfaces do not change definition-set
identity. Serialized artifacts remain untrusted until reverified into sealed
values, and a reproducible session does not hot-reload semantics in place.

## D-016: Extensibility follows a trust ladder

The extension tiers are:

```text
content data
  -> checked interpreted IR
  -> proposal-only evaluator
  -> statically linked trusted engine extension
```

There is no general native dynamic Rust plugin ABI. A future Wasm evaluator is
capability-limited, deterministic where required, resource-bounded, and
proposal-only.

Simulation authority and host isolation are independent: in-process native
evaluators are host-trusted even though their outputs remain proposal-only.
T1 interpreters and isolated T2 evaluators have deterministic resource limits.

Storage, indexing, and result-equivalent accelerators are infrastructure
providers rather than semantic extension tiers. Any implementation choice that
can change a logical result is instead named in execution semantics.

## D-017: Executable families share infrastructure, not authority

Executable families, when introduced, remain specific to their authority and
stage, with their own legal operations and verifiers. The initial IR surface is
limited to action, effect, event, and process definitions and the condition
roots their real consumers require. Projection, observation, appraisal, intent,
metrics, and study representations may remain checked declarative records;
they become IR only when a real source/lowering/transformation or interpreter
boundary justifies it.

Families may share compiler libraries for names, types, bindings, provenance,
and diagnostics. Metrics and study designs remain canonical `world-lab`
artifacts outside runtime compatibility. No flattened universal semantic
envelope is introduced.

## D-018: Rules provide a complete baseline

Basic actors can operate with deterministic rule implementations for evidence
assimilation, appraisal, intent, activity initialization/advancement, and
action selection. Social interpretation is enabled only for profiles and
domains that need it.

Planning, learned policies, language models, recursive theory of mind, and rich
emotion models are optional implementations. Higher cognition may be disabled
without disabling basic action behavior.

Planning or search may be an internal `ActivityController` strategy; there is
no separate planner port initially.

## D-019: External computation is captured and freshness-checked

Human input, remote services, LLMs, nondeterministic learned evaluators, wall
clock, and trajectory-affecting host I/O enter through captured ingress
records. Immediate authorized session pause/resume/quarantine/failure, invocation
cancellation/timeout/failure, or admission-sealing requests are instead
captured and applied atomically by `Manage` in one `ManagementBatchRecord`.
Physical run-attempt cancellation is a separately durable, idempotent
`RunAttemptControl` disposition and cannot mutate session state.

The engine-private asynchronous invocation/result record identifies its exact
source revision, witness, and implementation; the external policy payload does
not. Completion order cannot decide simulation order; stale results are
explicitly discarded, privately rebound/revalidated, or reevaluated. A new
logical evaluation is permitted only when the projection-safe payload changed
or evaluator semantic binding changed explicitly on a child branch/epoch, or
another actor-visible configured cause requests it; a hidden-only witness
change and a transport retry never create a new logical invocation.

Pending invocation, captured result, admission, cancellation, and fallback are
checkpointed runtime-control state. External ingress atomically records the
input and its explicit simulation-time delivery.

Inline deterministic and deferred captured evaluation are distinct execution
classes. Deferred results run only at a later microstep and cannot retroactively
join their invoking moment. The initial `FrontierBlocking` policy blocks the
session frontier; `HostScheduled` is the explicit nonblocking alternative. An
admission-sealing management transition cannot cross unresolved scheduled work
or a `FrontierBlocking` invocation unless that same atomic record resolves or
disposes of the blocker.

## D-020: Causal history, explanation, and telemetry remain separate

Authoritative history supports recovery and causal audit and includes captured
inputs and typed `AuthorityRecord`s with nested attempts and commits. Decision
traces explain lifecycle work. Performance telemetry operates the system.
Separately durable run-attempt control gates host execution and freezes a
terminal cursor; it is neither world history nor explanatory telemetry.

They share references but differ in authority, retention, schema, and whether
they may use wall-clock data.

## D-021: Experiments freeze all treatment-relevant artifacts

An `ExecutionSpecId` hashes the canonical pre-run behavior-affecting specification
body without an ID field. The body references one normalized
`ExecutionSemanticsManifest` for the engine protocol, definitions, required
semantic implementations, lifecycle profiles, and configuration, plus the
root seed, `InitialStateRootId`, exact `TerminationContract`, and
trajectory-affecting external bindings. The initial-state root hashes a
canonical body containing no child `ExecutionSpecId`; scenario defaults and
descriptive assumptions remain planning provenance after their resolved
behavioral outputs enter the specification. Exact whole-build identity remains
reproduction provenance unless it changes declared execution semantics.

A `RunCaseId` combines `ExecutionSpecId` with one immutable study assignment,
including its exact optional scenario-provenance reference. That reference
remains outside execution and trajectory identity. A separate immutable
`RunCaseResult` maps the assignment to the selected attempt,
`RunArtifactSet`, and trajectory or terminal status. The study artifact records
pairing, exclusions, stopping, and intended analysis; an `AnalysisManifest`
references the case-result mappings and records prespecified, deviating, and
exploratory analyses without changing trajectory identity.

`ExecutionSpecId` is pre-run configuration identity. A completed or retained
run receives `TrajectoryId` from that specification and its cumulative
authority-history hash. Reuse requires the exact trajectory and sufficient
captured artifacts; an equal execution specification alone is not enough when
external inputs remained open.

Component comparisons replace one lifecycle implementation while holding the
others constant where the experimental question permits. Metrics are
recomputed from immutable artifacts and never feed back into the run.

## D-022: Scale by skipping and replication before distribution

The scale order is event skipping, incremental projection, individual
background simulation, parallel independent runs, then parallel pure
evaluation.

Intra-world parallel discrete-event simulation and distribution require
profiling evidence and a separate design decision.

## D-023: Initial products are library, CLI, lab, and inspector

The first supported surfaces are:

- Rust engine and authoring libraries;
- deterministic headless CLI;
- experiment runner;
- trace and causal-history inspector/export.

Server, editor, package registry, database backend, Wasm host, and distributed
execution remain leaf adapters or deferred products.

## D-024: Deferred internals must not leak into shared contracts

The final pack syntax, expression language, action/effect DSL, planner
algorithm, appraisal ontology, storage backend, metric query language, and
resolution conversion algorithms are deliberately deferred.

They become shared architecture only when they have a real producer, consumer,
failure model, versioning story, and validation scenario.

## D-025: Outcome-affecting coordination state is durable

The lifecycle coordinator is operationally stateless. Pending evaluator
invocations, coalescing/cancellation generations, bounded retry/fallback
dispositions, open or evaluation-pending action opportunities, and scheduler
wakes live in concrete runtime-control or scheduler state. An exact
`AppraisalResult` payload or fingerprint is retained in a typed lifecycle
continuation only when it crosses a microstep, external invocation, or
checkpoint.

“Not world truth” does not mean “safe to omit from checkpoints.”

Runtime-control is a partition of concrete typed state machines, not a generic
blackboard or `Map<Key, Value>`. Every record has one owner, schema, transition
rule, and removal rule.

The host/run plane follows the same rule: `RunAttemptControl` reservation and
`RunFinalization` are separately durable even though they do not enter world
state or create another world-mutation path.

Shared durable lifecycle protocol schemas belong below context and decision,
in `world-model`; runtime stores and structurally transitions them, while
engine coordination and evaluators retain their narrower responsibilities.
Implementation-defined persistent state is allowed only in a bounded,
versioned sealed slot owned by one exact lifecycle port, never in a generic
blackboard.

## D-026: One formal kernel explains the architecture without becoming a framework

The normative world-transition kernel is:

```text
immutable execution semantics
  + authoritative session state
  + capability-scoped immutable typed input
  -> bounded proposal or selected supplied ID
  -> verified atomic authority transition
  -> later typed causal work
```

This is a refinement target and test oracle, not a requirement for one generic
subsystem trait, universal IR, universal result envelope, pass manager, or
physical state tuple. Concrete domain types and narrow private capabilities
remain the implementation default.

The complete controlled-attempt model is
`Ω = (AttemptControlPlane, Σ)`, where the control plane contains
`RunAttemptControl`, its accepted control-event log and trace head,
artifact-retention/pin-transfer state, disposition evidence, and
reconciliation receipts. Attempt-control transitions gate permission and
select a prefix; only the three kernel transitions can change `Σ`.

## D-027: Run finalization is separate from session health

`WorldSession.mode` describes runtime health and administrative control.
Completion of one physical execution is instead owned by a separately durable
`RunAttemptControl`.

Attempt creation is an atomic create-or-open bound to one authority domain,
attempt, specification, root, and semantic epoch. Every attempt-scoped call
that may change the authoritative world reserves one exact authority cursor,
permits at most one `Admit`, `Fire`, or `Manage`, stores a matching
`StepPublicationReceipt` with publication, and then evaluates the pure,
versioned `TerminationContract` over a stage-checked `TerminationView` plus the
manifest-fixed finalization policy before another world call can enter.
Finalization compare-and-sets one
`RunFinalization(RunAttemptId, terminal cursor, reason, evidence,
TrajectoryId)`. Crash recovery accepts only an unchanged head or an exact
receipt-proven direct successor and never reruns effects, policies, or external
services.

An idempotent attempt-cancellation request may atomically record its
disposition, deduplication outcome, and finalization at the current reconciled
cursor without a world transition. It cannot pass a reserved step. Any desired
session pause, quarantine, failure, or invocation cancellation remains an
explicit `Manage` request.

A finalized attempt cannot mutate the world again even if the retained session
mode is still `Running` and future work remains. Inspection, archive,
idempotency lookup, and reliable adapter delivery remain legal. Continued
simulation requires an explicit child root/branch and new attempt. This keeps
one world mutation lane while making the selected trajectory prefix unique and
crash-safe.

`RunAttemptId` is namespaced by a durable, unique
`AttemptAuthorityDomainId`; a fresh independent writable control domain cannot
mint the same physical attempt ID, and a portable copy receives no writer
capability. Artifact retention is a separate typed control state. It pins the
immutable resolved-execution closure at creation, acquires a finalized
`RunArtifactSet` target before releasing source pins, and installs a permanent
descriptor/fingerprint tombstone before terminal discard and garbage
collection. Handoff/discard retries are idempotent and cannot change the
terminal prefix.

## D-028: Runtime is the physical authority boundary

The mutable session head, scheduler, authority-history append, record sealer,
run-attempt control, publication receipt, and atomic repository protocol reside
inside `world-runtime`. `world-model` exposes immutable records, checked
protocol schemas, read models, and typed deltas but no store or `apply_*`
surface.

`world-engine` coordinates lifecycle work through one explicit
`prepare_fire -> evaluate immutable due work -> complete_fire` protocol.
Runtime creates the durable reservation and opaque prepared token, validates
all returned typed proposals, interprets the verified termination contract,
and retains the only publication capability. A typed
`fail_prepared_fire` path consumes that same token to attach canonical
host-budget, external-failure, or engine-failure evidence in the attempt
control plane; a crash with no such call remains observably different.
Runtime also owns the opaque committed-history retention lease that prevents
compaction until a self-contained reliable-delivery root is durable, plus the
archive-generation fence over those roots. Neither control-only protocol can
change the world cursor or finalization chosen by the fixed policy. The
semantic repository remains crate-private behind an opaque runtime service.
If the engine/runtime split ever requires exposing a forgeable head
replacement or publication token, the two packages must be merged rather than
weakening authority.

No separate kernel, storage, artifact, or per-lifecycle package is introduced
without a demonstrated dependency or implementation boundary. The normative
physical layout is
[Target Rust Code Architecture](code-architecture.md).

## D-029: The current implementation receives a clean replacement

The current repository has no production consumer or published durable format
that justifies compatibility. The rewrite therefore preserves the old
implementation only in Git history.

There is no compatibility facade, deprecated alias, legacy module, old-format
importer, feature-selected authority path, dual pipeline, or wrapper around
the current `WorldModel`, `CausalRuntime`, context pipeline, or decision
runner. Reusable algorithms and invariant tests are rewritten under their
target owners.

The first target-state merge combines immutable artifact/definition
foundations with the minimal authoritative vertical slice and deletes the old
authority and decision paths in the same architectural cut. Later increments
only extend target-shaped structures.

## D-030: Canonical identities use an explicit versioned byte protocol

Identity-bearing values use `world-canonical-v1` and BLAKE3-256. Each preimage
starts with the literal ASCII bytes `world-canonical-v1`, followed by one
mandatory domain encoded as a `u64` big-endian byte length and its exact ASCII
bytes. Domain labels are 1–64 bytes and match
`[a-z][a-z0-9-]{0,63}`.

The body protocol is byte-complete:

- unsigned integers are exactly `u8`, `u16`, `u32`, `u64`, or `u128`;
  multi-byte integers are big-endian;
- a boolean is one byte: false is `0` and true is `1`;
- an enum is a `u32` big-endian discriminant selected through an exhaustive
  match owned by its schema;
- bytes are a `u64` big-endian byte length followed by the exact bytes;
- text is its `u64` big-endian UTF-8 byte length followed by exact UTF-8 bytes;
- an option begins with one byte: absent is `0`, present is `1` followed by
  the encoded value;
- an ordered sequence is a `u64` big-endian element count followed by each
  element in order.

The literal protocol prefix is not itself length-framed. Every identity schema
uses a distinct versioned domain label. Changing that schema's field set,
field order, or interpretation without changing the domain label is forbidden.

Maps and floating-point values are forbidden in identity preimages. A logical
map is first converted by its owner into a sequence sorted by the canonical
bytes of its key. The protocol performs no implicit Unicode normalization;
identity-critical identifiers define their own accepted alphabet and
normalization before encoding. Identity is never derived from Rust memory
layout, `std::hash::Hash`, debug/display text, or a convenience serializer.

The digest is the standard unkeyed BLAKE3 hash of the complete preimage using
its default 32-byte output. Keyed hashing and key derivation are not this
protocol.

This small protocol separates durable identity from storage and wire formats,
keeps golden vectors implementable in other languages, and makes schema/domain
evolution explicit. BLAKE3 provides an official Rust implementation with
published cross-implementation test vectors. The implementation pins the
algorithm in the protocol identifier and artifact metadata rather than
treating a crate version as semantic identity.

Primary references:

- [official BLAKE3 Rust implementation](https://docs.rs/blake3/latest/blake3/)
- [official BLAKE3 repository and test vectors](https://github.com/BLAKE3-team/BLAKE3)
