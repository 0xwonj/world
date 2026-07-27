# Architecture Redesign Research Synthesis

## Status

Research basis for the target architecture under
[`docs/architecture/target-architecture/`](../architecture/target-architecture/README.md).

This is a design synthesis, not a claim that the engine implements a formal
human cognition model or copies any one simulation framework.

## Research question

Given permission to replace the current architecture completely, what
high-level system best supports:

- a production-grade simulation-first RPG engine;
- persistent, partially informed actors;
- cleanly separated appraisal, commitment, execution, and action cadence;
- deterministic execution, inspection, replay, and branching;
- data-defined games with controlled semantic extension;
- controlled comparison of rule, search, learned, and language-model
  evaluators;
- future scale without premature distributed complexity?

## Method

The redesign combined:

1. an audit of the current repository's documents, crate boundaries, public
   contracts, and in-progress context/decision implementation;
2. primary cognitive-agent and planning literature;
3. official and primary discrete-event, deterministic simulation, persistence,
   and multi-resolution sources;
4. official language, IR, capability-security, artifact, and schema sources;
5. experiment and agent-based-modeling methodology;
6. formal specification, modularity, noninterference, and atomicity literature;
7. adversarial scenario validation of the resulting contracts.

The selection rule was practical: borrow representation pressure and proven
operational separations, not an entire framework's ontology or implementation.

## Repository audit

### Durable ideas

The existing architecture already established several decisions worth
preserving:

- simulation owns hard truth;
- actor context is distinct from authoritative state;
- semantic decision work proposes rather than commits;
- runtime validates and commits;
- packs declare content through checked representations;
- standard definitions and trusted primitive implementations are separate;
- actions, effects, events, and long-running processes are distinct;
- explanation and provenance are first-class.

These are visible across the existing
[simulation core](../design/simulation-core.md),
[truth authority](../design/truth-authority-and-layer-boundaries.md),
[causal runtime](../design/causal-runtime.md), and
[pack authoring](../design/pack-authoring-and-semantic-declarations.md)
documents.

### Structural problems exposed by implementation

The in-progress implementation also showed where generality was arriving
before operational semantics:

- one configurable decision runner spans concerns that need different wake
  conditions and persistence;
- some declared representation/input kinds have no meaningful producer,
  consumer, or downstream influence;
- context source registration can make a required input appear present even
  when projection reported it unavailable;
- trace headers can retain successful reads while losing completeness and
  diagnostic meaning;
- action definitions contain role, binding, requirement, and effect structure,
  while action candidate generation does not yet ground the real definition;
- immediate actions, process continuation, waits, and abstract targets are
  forced toward one selection representation despite different owners.

The redesign therefore retains typed artifacts, pure evaluators, profiles, and
traces while moving them behind concrete lifecycle ports. It does not evolve
the current broad pass graph into the engine's universal control plane.

## Finding 1: cognition needs separate cadences

### Evidence

[Rao and Georgeff's BDI architecture](https://cdn.aaai.org/ICMAS/1995/ICMAS95-042.pdf)
distinguishes beliefs, desires, and intentions and treats commitment as the
balance between reactive reconsideration and stable goal-directed behavior.

[Georgeff and Lansky's Procedural Reasoning System](https://aaai.org/Papers/AAAI/1987/AAAI87-121.pdf)
interleaves reasoning with execution, supports interruption and resumption, and
defers decisions until information is available.

[3T](https://cdn.aaai.org/Symposia/Spring/1996/SS-96-04/SS96-04-001.pdf)
separates deliberative planning, procedure sequencing/monitoring, and reactive
skills. [ATLANTIS](https://flownet.com/gat/papers/aaai92.pdf) similarly argues
for heterogeneous asynchronous planning and reaction, where a plan guides
action rather than directly controlling it.

[EMA](https://stacymarsella.org/publications/pdf/EMA_Dynamics.pdf) models
appraisal as a uniform, rapidly updated operation over an incrementally
changing actor-environment interpretation. The apparent difference between
fast and slow appraisal can come from the processes that update the
interpretation, rather than from several unrelated appraisal engines.

The [official Soar architecture](https://soar.eecs.umich.edu/soar_manual/02_TheSoarArchitecture/)
distinguishes temporary elaboration, operator proposal, selection, and
application.

### Synthesis

The stable lifecycle is:

```text
evidence assimilation
  -> incremental appraisal
  -> material-change-triggered intent reconsideration
  -> persistent activity initialization/advancement
  -> on-demand grounded action selection
  -> runtime
```

Intent owns commitment. Activity owns execution method and local recovery.
Action policy owns one next choice. Runtime owns accepted effects. Appraisal is
incremental and orthogonal rather than a mandatory full-stack prelude to every
action.

This supports separate scheduling, budgets, tracing, and experimental
substitution.

### What was not adopted

- formal modal BDI logic;
- a complete PRS or Soar production system;
- a universal mutable cognition blackboard;
- separate appraisal frameworks for every domain;
- a claim to model human psychology.

## Finding 2: partial observability is architectural

### Evidence

[Kaelbling, Littman, and Cassandra's POMDP treatment](https://doi.org/10.1016/S0004-3702(98)00023-X)
formalizes why action under partial observability depends on an internal state
derived from observation history, and why information-gathering can be
rational.

The [Soar semantic-memory](https://soar.eecs.umich.edu/soar_manual/06_SemanticMemory/)
and [episodic-memory](https://soar.eecs.umich.edu/soar_manual/07_EpisodicMemory/)
interfaces distinguish working representations from longer-lived memory.
[Jason's belief annotations](https://jason-lang.github.io/jason/tech/annotations.html)
retain provenance such as perception, self, or another agent.

### Synthesis

The engine needs distinct transient `EvidenceDelivery`, accepted
`EvidenceRecord`, accepted `Belief`, optional memory, and ephemeral decision
working frames. Every actor-facing evaluator consumes a projection-safe
immutable policy payload paired with an engine-private invocation envelope.
Empty, unavailable, false, uncertain, outdated, and contradictory are distinct
states. Any reduced-detail projection needs type-specific omission semantics
rather than a universal shallow marker.

Action candidate generation must use actor-visible bindings and perceived
availability. Runtime legality is a separate authoritative test.
The rich authoritative attempt/process resolution remains engine-private.
Actor-facing controllers receive a neutral wake; success, failure, and other
world meaning reach them only through declared observation projection and
accepted evidence. Wake presence, timing, generation, and cause are part of
the information boundary, not merely the payload. Global revision, raw moment
and cause, dependency witnesses, authority-derived IDs, and private build
diagnostics remain in an engine-private invocation envelope. Policy-visible
IDs and fingerprints derive only from actor-visible semantics, so hidden-only
changes under fixed execution semantics cannot alter the policy request or
create an extra logical evaluator invocation as a metadata or control-flow
side channel.

### What was not adopted

- a world-scale belief distribution;
- a mandatory POMDP solver;
- a complete theory-of-mind recursion engine;
- one combined forgetting, consolidation, inference, and retrieval subsystem.

## Finding 3: planning belongs behind the activity boundary

### Evidence

[HTN complexity and expressivity research](https://cdn.aaai.org/AAAI/1994/AAAI94-173.pdf)
shows both the usefulness of domain procedure structure and the rapid
complexity growth of unrestricted task networks.

[F.E.A.R.'s real-time planning architecture](https://ojs.aaai.org/index.php/AIIDE/article/view/18724)
reports the practical need to restrict symbolic representations, cache costly
preconditions, and update sensors at different cadences.

The 3T and ATLANTIS evidence above supports a sequencer/executive between
planning and reactive execution.

### Synthesis

Standardize one outer activity boundary:

```text
ActivityController::{initialize, advance}
ActionPolicy
```

Planning, search, behavior-tree execution, scripts, or learned method state
remain internal `ActivityController` strategies. Any state that affects a later
invocation is versioned controller state. Do not standardize a universal plan
IR or a separately substitutable planner port before a concrete scenario needs
one. The controller opens a grounded one-shot action opportunity; action policy
selects a supplied candidate, and runtime revalidates it.

## Finding 4: the kernel should be deterministic discrete-event simulation

### Evidence

The official [OMNeT++ manual](https://doc.omnetpp.org/omnetpp/manual/index.html)
describes strict future-event scheduling, separates simulation from wall-clock
time, and uses fixed-point integer simulation time to avoid floating-point
ordering problems.

The official [ns-3 scheduler documentation](https://www.nsnam.org/docs/manual/html/events.html)
uses timestamp-ordered zero-duration event execution and explicitly calls out
same-time ordering. [Ptolemy II superdense time](https://ptolemy.berkeley.edu/ptolemyII/ptII10.0/ptII10.0.1/doc/codeDoc/ptolemy/actor/util/SuperdenseTime.html)
provides the relevant `(time, microstep)` model for zero-time causal chains.

[FoundationDB's simulation testing documentation](https://apple.github.io/foundationdb/testing.html)
and [paper](https://www.foundationdb.org/files/fdb-paper.pdf) demonstrate the
value of running complex systems under deterministic simulation with controlled
nondeterminism and reproducible failures.

[Fujimoto's parallel discrete-event survey](https://doi.org/10.1145/84537.84545)
and the [Time Warp paper](https://ics.uci.edu/~cs230/reading/jefferson.pdf)
show the causality, state-saving, rollback, and irreversible-I/O complexity
introduced by parallel event execution.

### Synthesis

Use quantized virtual time, microsteps, serialized typed triggers, and a single
canonical authoritative publication lane. One sealed `SimMoment` is an atomic
resolved batch over a shared base snapshot. External ingress and management are
separate typed revision transitions rather than fake simulation moments. A
moment record preserves accepted/rejected attempts and schedules
later-microstep dispatch only for a nonempty self-contained reaction envelope.
Wake only meaningful work. Parallelize immutable projection/evaluation and
independent runs before considering parallel discrete-event commit inside one
world.

Same-time domain conflicts require an explicit rule; incidental queue order is
only a final mechanical ordering.

### What was not adopted

- a global per-actor update tick;
- wall-clock-authoritative execution;
- arbitrary user priority numbers;
- closure-based scheduled work;
- distributed or optimistic rollback simulation as a first kernel.

## Finding 5: state and history need atomicity without pure event sourcing

### Evidence

[AWS transactional-outbox guidance](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/transactional-outbox.html)
documents the dual-write failure created when state and event publication are
independent.

[Microsoft's event-sourcing guidance](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
records the permanent schema, reconstruction, projection, ordering, and
idempotency costs of making an event stream the source of truth for all state.

[Raft's snapshot design](https://raft.github.io/raft.pdf) provides a useful
general invariant: a snapshot covers an exact committed prefix and records the
last included position.

[AWS deterministic replay guidance](https://docs.aws.amazon.com/durable-execution/patterns/best-practices/determinism/)
identifies time, randomness, external services, files, generated IDs, and
mutable globals as values that must be captured or controlled for replay.

### Synthesis

Use:

```text
materialized authoritative state
+ state-complete WorldCheckpoint
+ immutable pre-run ResolvedExecutionClosureManifest
+ exact root-relative checkpoint/archive ArtifactClosureManifest
+ compactable committed history tail
```

Publish trigger consumption, accepted state, runtime control, scheduler
changes, accepted/rejected `AttemptRecord`s, and matching commit records
atomically in one typed `AuthorityRecord`. The storage boundary has one
crash-safe linearization point. A nonempty reaction envelope and its dispatch
obligation are part of the same publication; an empty envelope schedules
nothing. Keep semantic domain events distinct from exact recovery deltas.

Distinguish restoration, verification, and counterfactual branch operations.
Capture every nondeterministic external input and evaluator result. Restoration
never reruns it.

## Finding 6: reproducibility needs logical randomness identity

### Evidence

The [ns-3 random-stream manual](https://www.nsnam.org/docs/manual/html/random-variables.html)
shows how assigning streams by object creation order can make unrelated
configuration changes perturb results.

[Random123](https://random123.com/) supplies counter-based random generation in
which values and independent streams are selected from explicit keys and
counters.

### Synthesis

Use named, versioned random keys built from run, namespace, causal identity,
semantic purpose, and ordinal. Do not use one global mutable PRNG sequence.

Experiment manifests distinguish shared exogenous streams from
branch-dependent endogenous streams.

[Yang and Nelson's comparative-simulation research](https://doi.org/10.1287/opre.39.4.583)
supports the variance-reduction value of common random numbers while also
showing that pairing and synchronization policy are part of experimental
method, not an incidental seed choice.

## Finding 7: multi-resolution is a consistency problem

### Evidence

[Dynamic level of detail for large-scale agent simulation](https://aamas.csc.liv.ac.uk/Proceedings/aamas2011/papers/C5_B67.pdf)
measures both computational gain and behavioral dissimilarity when agent detail
changes.

[Multi-resolution modeling research](https://citeseerx.ist.psu.edu/document?doi=37eb2fe155f7bd7f114584d447c28468b0ca74d7&repid=rep1&type=pdf)
identifies invalid aggregate/disaggregate states, temporal inconsistency,
transition latency, and chain disaggregation as central problems.

### Synthesis

Keep a canonical entity core and exactly one active tier representation.
Promotion and demotion are explicit transactions with declared conserved
properties, trigger replacement, conversion version, and approximation
evidence.

Begin with individually identified detailed/background/dormant actors and
sparse process wakes. Defer population aggregation until a domain defines
membership, conservation, conversion, interaction, and error contracts.

## Finding 8: extensibility needs typed dialects and capabilities

### Evidence

[MLIR dialects](https://mlir.llvm.org/docs/DefiningDialects/) and
[interfaces](https://mlir.llvm.org/docs/Interfaces/) demonstrate the useful
separation between shared compiler infrastructure and family-owned operations,
types, verification, and capabilities. Its
[dialect-conversion model](https://mlir.llvm.org/docs/DialectConversion/)
also makes legality explicit at a staged lowering boundary rather than
assuming that every intermediate form is executable.

The [rustc compiler overview](https://rustc-dev-guide.rust-lang.org/overview.html)
is a concrete example of distinct resolution, typed representation, and
lowering phases without claiming one representation serves every job.
[Translation validation](https://ofers.dds.technion.ac.il/publications/w98.pdf)
motivates independently checking a produced result at a trust boundary instead
of trusting the mere fact that a compiler ran.

[Cargo dependency resolution](https://doc.rust-lang.org/stable/cargo/reference/resolver.html)
illustrates why semantic dependency requirements and an exact lock serve
different purposes. The [OCI descriptor](https://github.com/opencontainers/image-spec/blob/main/descriptor.md)
and [manifest](https://github.com/opencontainers/image-spec/blob/main/manifest.md)
specifications provide a useful content-addressed artifact model.

The [Rust reference](https://doc.rust-lang.org/reference/items/external-blocks.html#abi)
does not promise a stable native Rust ABI. Native dynamic Rust plugins are
therefore a poor foundational extension contract.

The [WebAssembly component model](https://component-model.bytecodealliance.org/design/component-model-concepts.html)
provides typed imported/exported interfaces, while
[WASI capabilities](https://github.com/WebAssembly/WASI/blob/main/docs/Capabilities.md)
provides an explicit capability-grant model. Wasmtime distinguishes
[deterministic fuel interruption](https://docs.wasmtime.dev/examples-interrupting-wasm.html)
from wall-time-dependent interruption and documents
[deterministic execution constraints](https://docs.wasmtime.dev/examples-deterministic-wasm-execution.html).

The official [Protocol Buffers evolution rules](https://protobuf.dev/programming-guides/proto3/)
show why removed field identifiers must be reserved rather than reused.
[CEL](https://cel.dev/) is a useful example of a bounded, non-Turing-complete
embedded expression model; it motivates resource and termination contracts
without being selected as this engine's DSL.

### Synthesis

Use four extension tiers:

```text
content data
checked family-specific IR
proposal-only evaluator
statically linked trusted extension
```

Packs first resolve an exact package/source graph, then compile source forms
through resolved typed declarations into fully lowered executable family IR,
or reverify an existing binary artifact. Compilation finalizes an
artifact-digest `PackLock`; linking produces a process-independent
`RuntimeDefinitionSet`; and activation binds that set to installed
implementations in a process-local `ActivatedDefinitionRegistry`.

A pack binds only its exact required semantic-interface closure from the
`SemanticInterfaceCatalog`. Unused installed primitives and
semantics-preserving infrastructure do not change pack or definition-set
identity. Catalog descriptors are declarative; custom lowering/verifier code
and runtime primitive implementations remain statically linked and
host-trusted.

Executable families share private compiler machinery but retain separate
authority vocabularies, verifiers, interpreters, and deterministic resource
limits. Study and metric artifacts remain validated `world-lab` data until a
real analysis language justifies separate analysis IR. Wasm remains a future
bounded evaluator mechanism, not a reason to expose mutable state. In-process
native evaluators remain host-trusted even when their simulation output is
proposal-only.

## Finding 9: research requires frozen artifacts and separate records

### Evidence

[NetLogo BehaviorSpace](https://ccl.northwestern.edu/netlogo/docs/behaviorspace.html)
and [FLAME GPU ensembles](https://docs.flamegpu.com/guide/running-multiple-simulations/index.html)
treat configured independent runs as the natural unit of batch simulation.

The [ODD protocol](https://www.jasss.org/23/2/7.html) provides a durable
human-facing structure for describing agent-based models.

[HELM](https://nlp.stanford.edu/helm/vhelm/), and
[Melting Pot](https://proceedings.mlr.press/v139/leibo21a.html) support
controlled scenario/metric decomposition, multi-metric evaluation, and
generalization across scenarios and partners rather than one aggregate score.

[W3C PROV-DM](https://www.w3.org/TR/prov-dm/) supplies a useful distinction
among entities, activities, agents, usage, generation, and derivation.
[OpenTelemetry](https://opentelemetry.io/docs/specs/otel/overview/) supplies
useful export concepts for spans, links, events, and operational telemetry.

The [RO-Crate specification](https://www.researchobject.org/ro-crate/specification/1.3/index.html)
provides a useful precedent for packaging data, software, contextual entities,
and provenance into a portable research-object closure.

### Synthesis

Normalize trajectory-affecting runtime definitions, required semantic-interface
bindings, lifecycle implementations, execution configuration, evaluator
identity, and semantic engine implementations into one
`ExecutionSemanticsManifest`. An ID-free `ExecutionSpec` body references that
manifest together with one content-addressed `InitialStateRootId`, seed,
effective termination, and any frozen input schedule or authorized
input-channel bindings. The initial root contains the complete starting
lineage/state/control/scheduler body without a child specification ID. A full
`ScenarioArtifact` remains optional planning provenance in a study's
`RunCase`; its resolved behavior-affecting outputs reach execution only through
the initial root and other canonical specification fields.
`ExecutionSpecId` hashes that canonical body.

`RunCaseId` combines `ExecutionSpecId` with the study assignment; physical
execution attempts receive `RunAttemptId`s tied to an exclusive
`AttemptAuthorityDomainId`, `ExecutionSpecId`, and runner-assigned key.
`TrajectoryId` combines that pre-run specification with the resulting
cumulative authority-history hash. `RunCase` is assignment-only; a separate
immutable `RunCaseResult` references the selected trajectory and run artifacts
or a terminal no-trajectory status. Matching `ExecutionSpecId` alone cannot
justify result reuse when external inputs remained open. Study design, capture
policy, metric definitions, report layout, and analysis method remain outside
`ExecutionSpecId`. Later studies may reference an exact retained trajectory
when its capture is sufficient. Preserve the exact artifact and whole-build
provenance closure, and parallelize complete execution specifications first.

Separate:

1. authoritative ingress, moment, and management history with nested captured
   inputs, accepted/rejected attempts, and commits;
2. decision/provenance trace;
3. durable run-attempt control, accepted control events, artifact-pin
   ownership, disposition evidence, and reconciliation receipts;
4. disposable performance telemetry.

Reliable adapter cursor/outbox state is a separately owned operational delivery
plane, not world history or research assignment.

Metrics are versioned post-run computations over immutable artifacts and never
feed back into the simulation.

## Finding 10: a small formal kernel should explain the boundaries

### Evidence

[Parnas's information-hiding decomposition](https://doi.org/10.1145/361598.361623)
organizes modules around design decisions likely to change, rather than stages
in a processing flow. [Lamport's state-machine specification
method](https://lamport.azurewebsites.net/tla/book-02-02-28.pdf) and
[Lynch and Tuttle's I/O automata](https://groups.csail.mit.edu/tds/papers/Lynch/TM-373.pdf)
separate state, actions, safety, and conditional progress without prescribing a
particular object model.

[Herlihy and Wing's linearizability](https://www.cs.cmu.edu/~wing/publications/HerlihyWing90.pdf)
provides the relevant correctness lens for a publication that must appear
atomic despite concurrency or crashes. [Goguen and Meseguer's
noninterference](https://www.cs.purdue.edu/homes/ninghui/readings/AccessControl/goguen_meseguer_82.pdf)
provides the relational shape for proving that hidden authoritative differences
do not change an actor-facing result. [Miller's object-capability
work](https://www.erights.org/talks/thesis/) supports granting authority through
narrow unforgeable capabilities rather than broad access plus convention.
The [Rust visibility model](https://doc.rust-lang.org/reference/visibility-and-privacy.html)
makes crate-local privacy the strongest ordinary language boundary available
between workspace packages, and the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/dependability.html)
recommend encoding validity in argument types where practical. Together they
support co-locating the mutable session head, record sealer, and publication
capability inside one runtime crate while exposing only checked requests,
sealed products, and immutable snapshots.

### Synthesis

The smallest explanatory model is:

```text
immutable execution semantics Γ
  + authoritative session state Σ
  + capability-scoped immutable typed input
  -> bounded proposal or selected supplied ID
  -> verified Admit | Fire | Manage transition
  -> one atomic AuthorityRecord
  -> later typed causal work
```

This model makes authority, atomic publication, attempt and trigger
conservation, idempotency, actor-relative noninterference, deterministic
identity, and conditional progress explicit. Concrete state and APIs must
refine it, and restoration must preserve the refinement mapping.

The complete controlled-attempt state is
`Ω = (AttemptControlPlane, WorldSession)`. The control plane contains
`RunAttemptControl`, its accepted control-event log and artifact-pin ledger,
disposition evidence, and the non-semantic publication receipts required for
reconciliation. It wraps the three transitions, binds each publication to one
reservation, evaluates the exact pure termination contract over a
stage-checked `TerminationView`, and freezes one terminal authority cursor.
This separates run completion from session health without granting the host
another way to mutate world state or letting persistence metadata become
termination semantics.

Information hiding keeps planning, appraisal, storage, compiler passes, and
other volatile algorithms inside their owning subsystems. Functional-core,
ports-and-adapters, object-capability, transactional-outbox, and compiler
staging vocabulary explain specific boundaries; none requires a
pattern-named module or universal framework.

### What was not adopted

- one generic `Subsystem<I, O>` trait;
- a universal proposal/result envelope across lifecycles;
- a generic runtime-control blackboard;
- a universal IR or configurable pass manager;
- proof-complete formal verification before implementation;
- pattern-driven crate decomposition without independent ownership pressure.

## Alternatives considered and rejected

| Alternative | Why it was rejected |
|---|---|
| One configurable cognition pipeline for all actors and cadences | Appraisal, commitment, activity, and action differ in trigger, persistence, budget, and failure behavior |
| Separate planner and activity-executive ports immediately | Planning strategy has no current independent caller or lifecycle; keeping it inside `ActivityController` preserves changeability without another shared protocol |
| One generic subsystem/evaluator framework | The useful shared algebra is conceptual; universal traits and envelopes would erase domain-specific authority, state, and failure contracts |
| One universal semantic declaration/IR | It erases authority classes and creates unused abstract variants before real producers and consumers exist |
| Universal planner representation | HTN, GOAP, behavior trees, scripts, and learned controllers need different internal state and complexity |
| World-scale POMDP | Preserving uncertainty is necessary; solving a global belief space is not |
| Context policy invents action bindings | It risks invalid structure and hidden-world access; grounding belongs in actor-relative context projection |
| Decision prevalidation as final legality | World state may change and actor belief may be false; runtime must revalidate |
| Global actor tick | It repeatedly evaluates inactive actors and couples unrelated cadences |
| Pure event sourcing from genesis | It makes semantic event schemas permanent recovery machinery and raises migration/reconstruction cost |
| Mandatory post-commit dispatch for every revision | Empty control-only records would create no-op causal waves; only nonempty self-contained reaction envelopes require dispatch |
| Bind execution identity to the full installed catalog or whole build | Unused capabilities and unrelated code would invalidate semantic identity; bind required semantic closure and retain whole-build provenance separately |
| One global RNG sequence | Unrelated draw insertion and parallel ordering perturb results |
| Detailed and coarse simulation concurrently authoritative | It creates double updates and cross-resolution inconsistency |
| Population aggregation first | Identity, conservation, and disaggregation contracts are not yet established |
| General mutable plugin callback | It destroys authority, replay, verification, and sandboxing boundaries |
| Native dynamic Rust plugins | No stable native Rust ABI and too much authority for ordinary content |
| Distributed simulation first | Causal rollback/coordination cost is high and current scale requirements do not justify it |

## Result

The target architecture is complete where premature change would be expensive:

- authority;
- lifecycle ownership;
- scheduling;
- persistence and replay semantics;
- formal safety, determinism, and refinement obligations;
- context and action grounding;
- extension trust;
- experiment reproducibility;
- multi-resolution consistency.

It remains deliberately narrow where future game requirements should drive
complexity:

- action and effect language;
- planning algorithm;
- appraisal and social model;
- belief revision;
- source syntax;
- storage backend;
- resolution conversion;
- optional model host.

This is the intended meaning of an extensible but not overdesigned
architecture: future complexity has a typed place to live, but does not become
shared machinery before a real scenario requires it.
