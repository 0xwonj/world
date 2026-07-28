# Target Architecture Execution Roadmap

## Status and purpose

This document owns the stable implementation sequence for replacing the
current code with the target architecture.

It deliberately specifies complete outcomes and objective gates without
freezing method-level plans for distant work. Detailed planning is rolling:
only the active milestone receives executable work packages. At each milestone
boundary, implementation evidence is reviewed before the next detailed plan is
accepted.

The normative architecture remains in the other documents in this package.
Operational status and active milestone plans live under
[`docs/implementation/target-rewrite/`](../../implementation/target-rewrite/README.md).

## Execution policy

### Clean replacement

The repository has no compatibility obligation to the current internal APIs or
formats. The rewrite introduces no:

- compatibility or legacy production module;
- deprecated alias for replaced types;
- old/new feature switch;
- current-format checkpoint or artifact importer;
- wrapper around `WorldModel`, `CausalRuntime`, or the generic decision runner;
- selectable dual authority pipeline.

Reusable algorithms and invariant tests are rewritten under their target
owners. The old implementation is retained only on the verified preservation
branch and in Git history.

### Rolling detail

The execution model has three planning horizons:

1. **Architecture** fixes authority, ownership, dependency direction, formal
   invariants, and final product boundaries.
2. **Roadmap** fixes milestone order, outcome, dependencies, and exit gates.
3. **Active milestone plan** fixes the next concrete work packages and may
   adapt local implementation choices as evidence arrives.

Future milestones must remain coarse. A milestone plan may not override a
normative architecture boundary. If implementation evidence requires changing
authority, package dependency direction, persistence ownership, or another
cross-system contract, work stops for an explicit architecture decision.

### Vertical delivery

Milestones deliver executable causal slices, not disconnected type catalogs.
A shared type is introduced only with a real producer, consumer, invariant,
and test. Empty placeholder crates and speculative public traits are forbidden.

The first target-state merge is intentionally larger than later merges:
Milestone 1 must contain both immutable definition/artifact foundations and one
minimal authoritative interaction. The target branch does not merge a state
that retains the old authority path alongside the new one.

## Milestone map

```mermaid
flowchart LR
    M0["M0<br/>Preservation and baseline"]
    M1["M1<br/>First authoritative slice"]
    M2["M2<br/>Deterministic kernel"]
    M3["M3<br/>Grounded action"]
    M4["M4<br/>Agency lifecycles"]
    M5["M5<br/>Durable execution"]
    M6["M6<br/>Product and research"]
    M7["M7<br/>Scale and optional semantics"]
    M8["M8<br/>Gameplay composition proof"]

    M0 --> M1 --> M2 --> M3 --> M4 --> M5 --> M6 --> M7 --> M8
```

## Reference-game validation lens

The non-normative
[Reference Game Vision](../../design/reference-game-vision.md) supplies one
small, continuous gameplay pressure test for the normative architecture. It
does not add another authority model, change milestone order, or require a
complete game before the engine foundations exist.

M1 through M4 record the pressure selected by their accepted exit reviews.
Rows after M4 are roadmap commitments but not accepted work-package plans. At
each later milestone entry, current implementation evidence selects the
smallest concrete fixture that proves the named capability and exit gate.
Names, setting, concrete layout within the selected bounded local square-grid
and regional-topology model, and content volume may change without an
architecture decision. The selected topology, capability, owner boundary,
causal interaction, and validation assertion may not be removed or weakened
by replacing the fixture.

| Milestone | Selected or anticipated gameplay pressure |
|---|---|
| M1 | One exact pack-defined containment transfer executes through the public engine/runtime path. |
| M2 | A representative same-moment contention and its complete later causal work exercise the deterministic kernel. |
| M3 | A representative local-world projection proves actor-visible, actually bindable action selection. |
| M4 | A short causal chain crosses process, unequal evidence, persistent agency, bounded recovery, and captured action evaluation. |
| M5 | An in-progress causal chain restores and verifies without external reevaluation; one explicit semantic-evolution case proves the child-epoch boundary. |
| M6 | CLI and MCP-style adapters share the actor-control boundary; authenticated captured evaluation is projection-safe and inspectable; AI-assisted source uses the ordinary compiler, diagnostics, preview, and explicit child-epoch path. |
| M7 | Positive promotion, demotion, and dormant activation replace obsolete work while preserving individual identity, hard invariants, commitments, and causal time. |
| M8 | Three interacting gameplay slices try to falsify composition boundaries through physical, epistemic, social, and agency consequences in one headless scenario. |

Each slice introduces only types with a current producer, consumer, invariant,
and test. Named game systems remain definitions, standard-world semantics,
game packs, or product content rather than becoming generic runtime concepts.
M8 is deliberately last: gameplay-facing primitive, resource, process, and
state-owner APIs are not declared stable until independent mechanics compose
without requiring a new authority path or generic framework.

## M0: Preservation and clean rewrite baseline

### Outcome

The complete pre-redesign workspace is recoverable from a verified commit, the
rewrite proceeds on a clean branch from the original baseline, the target
architecture is tracked, and the first executable milestone has frozen inputs
and a detailed plan.

### Exit gate

- the preservation branch names an explicit verified commit;
- all selected tracked and untracked files exist in that commit;
- ignored build output is absent;
- the rewrite branch starts from the intended base and contains no uncommitted
  legacy implementation work;
- the target architecture and execution documents are tracked;
- baseline build, dependency, and superseded-symbol evidence is recorded;
- canonical identity protocol, first standard interaction, and minimum
  source/artifact surface are selected for M1;
- the detailed M1 plan defines work packages, deletion scope, decision
  triggers, and binary acceptance gates.

## M1: Immutable packs through one authoritative interaction

### Outcome

One standard controller request compiles from exact pack artifacts, resolves
into immutable execution semantics, and produces one atomic authoritative
transition through the new engine/runtime facade. The old authority, context,
and generic decision paths are absent from the merged target state.

### Major capabilities

- canonical identity and version primitives;
- minimal checked action and physical-event definitions with nonempty ordered
  typed effect calls embedded in an action;
- verified pack artifact, exact lock, definition-set linking, and activation;
- private runtime session head and in-memory atomic repository;
- `Admit`, staged `Fire`, and minimal `Manage`;
- sealed authority record, cursor, publication receipt, and attempt control;
- sealed `ResolvedExecution`;
- `Engine`, non-cloneable `RunAttempt`, and read-only `WorldSession`;
- one trusted standard primitive and one inspector query;
- dependency, privacy, canonicalization, and vertical conformance tests.

### Exit gate

- every world change is exactly one `Admit`, `Fire`, or `Manage`;
- state, scheduler, history, cursor, and receipt publish atomically;
- no external package can construct or replace the session head;
- artifact loading and linking reject invalid identity or closure before
  session construction;
- repeated execution yields identical semantic fingerprints;
- no replaced public symbol or forbidden dependency edge remains;
- the M1-supported artifact/interface subset of validation scenario 13 passes;
- deterministic repetition, an exact dependency graph without a randomness
  provider, and absence of the target random protocol are proven; full
  keyed-randomness scenario 18 begins in M2;
- the single-slot atomic-authority subset of scenario 1 passes; complete
  same-moment batching and conflict resolution remain M2.

### Deferred

Actor-relative policy, rich process protocols, durable restoration, a database,
textual DSL design, CLI product work, and optional evaluators.

## M2: Complete deterministic runtime protocol

### Outcome

The minimal kernel becomes the complete deterministic authority protocol for
admission, moments, management, conflicts, deduplication, causal routing,
bounded work, termination, and attempt reconciliation.

### Major capabilities

- complete authority-record families and identity rules;
- admission frontier and typed request ledgers;
- same-moment preparation from one base snapshot;
- explicit footprints and total deterministic conflict resolution;
- rejection-only valid fallback;
- complete reaction, scheduling, reservation, and management-control records
  exercised by the M2 kernel; M3 adds the first action-opportunity record and
  M4 adds the remaining cognition and agency lifecycle records;
- generalize the M1 in-memory reserved-step subset into the complete
  backend-independent attempt reservation, disposition, receipt, cancellation,
  reconciliation, termination, and finalization state machines;
- deterministic budgets, keyed randomness, and exhaustive small-case
  permutation and invariant tests.

### Exit gate

- order, worker count, and collection representation cannot alter results;
- duplicate or retired request identities cannot create another effect;
- every admitted logical command has one durable outcome;
- invalid bindings, receipts, successors, and termination reads fail closed;
- the kernel portion of scenario 1 passes; actor projection and selection
  remain M3;
- the bounded-work and safety portion of scenario 10 passes; a real
  self-generating process/lifecycle cycle waits for an authored rule that
  needs it rather than being invented solely for a gate;
- the endogenous authoritative-randomness portion of scenario 18 passes;
  paired exogenous study streams wait for a real M6 consumer;
- kernel portions of scenarios 14, 15, and 19 pass.

## M3: Actor-relative context and grounded action

### Outcome

An actor chooses only from actor-visible, actually bindable action candidates,
while authority and lowering data remains private to the trusted coordinator.

### Major capabilities

- a concrete bounded containment-transfer projector that performs actor source
  capability gating and interaction projection internally;
- explicit successful empty projection;
- grounded candidate generation and private resolution tables;
- durable one-shot action opportunities;
- deterministic baseline `ActionPolicy`;
- execution-bound synchronous controller replacement;
- private selected-candidate lowering and runtime revalidation;
- neutral attempt-resolution wakes and hidden-state noninterference tests.

### Exit gate

- policy cannot invent a definition, binding, or runtime command;
- hidden-only state cannot change actor-visible payloads or logical invocation
  timing;
- every opportunity reaches one typed terminal disposition or bounded,
  causally linked successor;
- the M3-owned portions of validation scenarios 1, 2, 4, and 16 pass.

M3 proves the synchronous action-control waist only. In scenario 2 it owns
actor-relative candidate generation, private runtime rejection, the neutral
wake, and paired hidden-state noninterference; M4 owns the later observation,
evidence, and appraisal chain. In scenario 4, M3 proves successful
`Complete(empty)` behavior. Typed unavailability enters only with the first
genuine partial provider rather than being assigned to a milestone that would
need to invent one solely for a test. In scenario 16, M3 proves
revision-independent actor-safe fingerprints, prepared-step reservation, and
runtime legality; M4 owns retained-result witnesses, private rebind, discard,
and reinvocation.

## M4: Independently scheduled agency lifecycles

### Outcome

Evidence assimilation, appraisal, intent, activity, and action operate as
distinct typed lifecycles with independent cadence and explicit durable
protocols. Social interpretation has a distinct versioned binding but remains
explicitly disabled until a concrete social semantic slice justifies it.

### Major capabilities

- post-commit routing and coalescing generations;
- accepted evidence and belief transition;
- deterministic appraisal and intent baselines;
- persistent intent and versioned activity state machines;
- activity initialization and advancement;
- activity-sponsored creation and continuation of the M3 action-opportunity
  protocol;
- retained and deferred action evaluation, positive dependency witnesses,
  retained-result freshness, private rebind, cancellation, and failure;
- process instances and grounded actor-initiated control;
- canonical cross-lifecycle causal links in authority and lifecycle records.

### Exit gate

- no lifecycle recursively executes the full stack;
- intent, activity, action opportunity, and process remain distinct;
- basic actors operate without planning or rich appraisal;
- deferred work cannot be skipped by admission sealing;
- the action-policy result remains `Select | NoApplicableAction`; inline versus
  captured execution is an independent binding, and capture, cancellation,
  reinvocation, and fallback remain runtime control;
- the evidence/appraisal portion of scenario 2, the authoritative relocation
  and grounded-control portion of scenario 3, the semantic deferred-evaluation
  portion of scenario 5, the physical/evidence/appraisal portion of scenario
  11, scenario 12, and the retained-evaluation portion of scenario 16 pass;
- scenario 10 retains the proven kernel budget and management escape without
  claiming an authored self-generating M4 rule, and social interpretation is
  not claimed for scenario 11.

M4 owns the authoritative invocation state machine, captured-result ingress,
freshness, rebind/discard, cancellation, and deterministic fallback. M5 owns
checkpoint restoration and replay without external reevaluation. M6 owns
authentication, transport, CLI/MCP/player/AI adapters, and product inspection.
Scenario 4 remains unassigned until a real partial projection provider has a
production consumer and can distinguish `Unavailable` from `Complete(empty)`
without a synthetic failure source.

## M5: Checkpoint, restore, replay, branch, and delivery durability

### Outcome

A controlled attempt and its exact semantic closure survive interruption,
restore without re-executing external computation, support verification, and
produce read-only portable archives or explicit child epochs.

### Major capabilities

- durable checkpoint and typed history tail;
- exact artifact-closure manifests and retention;
- two-stage same-domain attempt restoration;
- verification replay and first-divergence diagnostics;
- immutable child lineage and offline target-schema migration boundary;
- reliable-delivery history leases and archive generation fencing;
- portable read-only archive import;
- crash-safe finalization, artifact handoff, discard, and compaction.

### Exit gate

- restoration invokes no evaluator or external service;
- active and reserved attempts retain their exact execution closure;
- portable data cannot create a second writer or delivery owner;
- incompatible definitions and schemas fail closed;
- verification identifies the first divergence;
- validation scenarios 5, 6, 8, 14, 15, and 19 pass.

M5 also owns restoration and verification assertions for the implemented
physical, evidence, and appraisal partitions of scenario 11.

An actual checkpoint transform is implemented only after a second supported
target checkpoint schema exists.

## M6: CLI, adapters, authoring, experiment, and inspection product

### Outcome

The architecture is usable as a headless engine and research product through
one actor-control boundary, authenticated projection-safe evaluator transport,
the ordinary checked-authoring pipeline, and stable composition, scenario,
run, trace, metric, comparison, and explanation surfaces.

### Major capabilities

- `world-cli` composition root;
- `world-lab` scenario and experiment artifacts;
- CLI and MCP-style actor-control adapters over the same core request and
  capability boundary;
- captured-evaluator transport that is separate from actor control and cannot
  submit runtime commands;
- authentication and dispatch of already committed projection-safe evaluator
  requests;
- AI-assisted source authoring over only the existing foundation T1 action and
  physical-event definitions with embedded ordered typed effect calls, through
  the ordinary compiler, structured diagnostics and repair, isolated preview,
  and explicit child-epoch activation;
- deterministic run-case expansion and parallel independent execution;
- immutable run results and trajectory-bound reuse;
- normalized decision/provenance trace DAG;
- recomputable metrics and analysis manifests;
- pack checking, running, replay audit, experiment, and explanation commands.

### Exit gate

- one lifecycle can be replaced while others remain fixed;
- `advance` and `run` deterministically stop at the same next player-input
  boundary, while inspection consumes no simulation time;
- equivalent CLI and MCP-style actor choices under the same authenticated
  principal and scope become the same canonical actor-control input and
  produce the same simulation trajectory;
- no adapter receives private resolution tables, hidden state, or mutation
  authority;
- external evaluator dispatch is authenticated, occurs only after the exact
  projection-safe request is durably committed, and admits only a captured
  result through the normal serialized boundary;
- AI-authored source is untrusted input to the same compiler as human-authored
  source; diagnostics and preview cannot mutate the parent epoch;
- M6 authoring neither introduces nor claims a reusable T0 content-data family,
  a process-definition family, or a social-definition family;
- accepted behavior-relevant authoring creates an explicit child epoch with an
  exact artifact closure and never hot-reloads the running parent;
- paired runs share declared exogenous inputs;
- metrics and telemetry cannot affect simulation behavior;
- run reuse requires exact trajectory identity and sufficient capture;
- validation scenarios 5, 9, 20, and 21 pass.

## M7: Scale and evidence-gated extensions

### Outcome

The engine supports individual background resolution and only those optional
semantic implementations justified by concrete scenarios and measurements.

### Major capabilities

- mutually exclusive detailed/background/dormant entity representations;
- checked promotion and demotion;
- explicit cancellation and replacement of obsolete tier-specific scheduler
  work on every transition;
- deadline- or event-triggered activation of dormant scopes without recurring
  dormant evaluation;
- background scheduling with fidelity evidence and bounded hysteresis;
- optional planner, learned, remote, or Wasm evaluator implementations behind
  existing lifecycle ports.

### Exit gate

- resolution tiers never create double authority;
- hard process and resource invariants survive transitions;
- positive promotion, positive demotion, and dormant activation each preserve
  identity, accepted causal progress, and declared commitments;
- every transition atomically invalidates obsolete tier-specific work and
  installs exactly the replacement work required by the target tier;
- a complete Background-to-Detailed-to-Background cycle neither replays past
  decisions nor invents observations;
- optional evaluators pass the same contracts as the deterministic baseline;
- replay never silently invokes external computation;
- validation scenarios 7, 17, and 22 pass.

Population aggregation, database persistence, a server/editor, dynamic package
registry, intra-world parallel commit, and distributed simulation require
separate evidence-backed roadmap decisions.

## M8: Gameplay composition proof

### Outcome

The gameplay-facing architecture survives a deliberately small falsification
suite in which independent physical, epistemic, social, and agency mechanics
compose through the established definition, proposal, authority, lifecycle,
and epoch boundaries. This milestone proves architectural fitness; it does not
build a generic gameplay framework or a content-complete game.

### Major capabilities

- a capability slice combining door, lock, tool, and body state;
- a physical-process slice combining material, integrity, heat, fire, and
  smoke;
- a social-causal slice combining witness, claim, institution, and obligation;
- the first minimal reusable T0 content-data family: pack-owned object
  archetype declarations containing only the typed references and parameters
  required by the accepted gameplay slices and checked root materialization;
- distinct reusable pack-owned T0 declarations, lab-owned
  `ScenarioArtifact` provenance, and checked materialization into an
  `InitialStateRoot`;
- the first checked process and stage-specific condition definition families,
  plus only the observation, social, capability, or other declarations used by
  the accepted slices;
- one cross-primitive same-moment conflict resolved by declared footprints and
  the existing atomic transition protocol;
- one causal chain from physical outcome through unequal evidence and accepted
  social state into persistent agency and later grounded action;
- one active long activity and independently owned physical process interrupted
  by the new threat, completing the interruption pressure from scenario 3;
- after the first T0 family and baseline root are working, one additional
  existing-vocabulary T0/T1 mechanic whose compiler, linker, root
  materialization, activation, and execution require no family-schema,
  primitive, or runtime-kernel change;
- one genuinely new T3 primitive installed as concrete statically linked
  owner-local semantics rather than a pack callback or generic state-owner
  registry;
- checkpoint, verification replay, branch, and multi-resolution transitions
  over state introduced by the slices;
- one integrated deterministic headless scenario exercised through the public
  product boundary.

### Exit gate

- each slice uses concrete definitions, state owners, proposals, and processes
  with a real producer, consumer, invariant, and test;
- no slice adds a second mutation path, bypasses grounded action, or lets
  observation, dialogue, AI output, or content directly mutate authoritative
  state;
- independently defined primitives compose through footprints, effects,
  events, and later work rather than pair-specific coordinator branches;
- a physical event can produce different actor-relative evidence without
  changing physical truth, and only an accepted typed social transition can
  create a claim, institutional finding, or obligation;
- changed capability or body state changes the grounded candidate space,
  duration, or risk without a bespoke central action sum;
- the first reusable T0 family has one immediate pack producer and checked
  root-materializer consumer, canonical identity and encoding, bounded
  validation, and no executable semantic operation;
- that first family contains only the object-archetype data required by the
  accepted slices; it does not introduce a universal entity schema, property
  bag, content registry, or executable callback;
- the process slice is interpreted from checked process definitions and
  stage-specific conditions rather than another hard-coded closed process
  variant;
- only after a baseline root has been materialized from the first T0 family,
  at least one additional mechanic expressed entirely with the installed
  T0/T1 vocabulary changes source, checked artifacts, root data, and tests,
  not the T0 family schema or checker, authority kernel, unrelated primitive
  owners, or unrelated public APIs;
- the new T3 primitive adds only owner-local state, preparation, codecs,
  migration, and composition-root wiring; it does not reverse dependency
  direction, grant code authority to a pack, or change unrelated public APIs;
- the exit review records crate, public-API, and dependency-impact evidence for
  the first T0 family, the later T0/T1-only mechanic, and the new T3 primitive;
- a transition spanning primitive owners declares typed reads, writes,
  resources, invariants, and participating-gate receipts before the existing
  prepared-subtransaction protocol combines it into one atomic commit;
- pack content, scenario planning provenance, and materialized initial state
  retain different owners and identities; none becomes a second mutation path
  after session creation;
- pure derivation introduced by a slice cannot mutate accepted state, and any
  retained incremental derived view is checked against full recomputation;
- same-moment cross-primitive conflict remains deterministic and atomic;
- the integrated scenario restores, verifies, branches, promotes, demotes, and
  activates dormant work without losing new hard state or duplicating effects;
- validation scenarios 23, 24, 25, and 26 pass; the threat-driven interruption
  portion of scenario 3 and the social portion of scenario 11 are complete.

The M8 exit review is the first stabilization decision for gameplay-facing
composition surfaces. A failed slice triggers the smallest owner-boundary
correction supported by evidence. It does not justify a universal registry,
untyped property bag, scripting escape hatch, or pair-specific exception.

## Validation scenario allocation

| Milestone | Primary scenarios |
|---|---|
| M1 | M1-supported portion of 13; deterministic/absence baseline for 18; single-slot authority subset of 1 |
| M2 | Kernel portion of 1; budget/safety portion of 10; endogenous authoritative portion of 18; kernel portions of 14, 15, 19 |
| M3 | Actor-control portion of 1; grounding, rejection, neutral-wake, and noninterference portion of 2; successful-empty portion of 4; synchronous portion of 16 |
| M4 | Evidence/appraisal completion of 2; authoritative relocation and grounded-control portion of 3; semantic deferred-evaluation portion of 5; physical/evidence/appraisal portion of 11; 12; retained-evaluation portion of 16 |
| M5 | Restoration/replay portion of 5; 6, 8, 14, 15, 19; restoration/verification of the implemented scenario-11 partitions |
| M6 | Authenticated product transport for captured evaluation in 5; 9, 20, 21 |
| M7 | 7, 17, 22 |
| M8 | 23, 24, 25, 26; threat-driven interruption completion of 3; social completion of 11 |

The deterministic engine foundation is operational when its implemented
contracts have passed the owning M1–M7 gates. The target roadmap is complete
only after M8 passes and every mandatory capability in the traceability matrix
has executable evidence. A documented later-scope justification may defer an
optional extension, but it cannot substitute for a named roadmap-completion
gate. Scenario 4 is deliberately awaiting the first genuine partial
projection provider rather than forcing an unused abstraction into M4; it
becomes mandatory when such a provider enters the supported product.

## Vision-to-evidence traceability

This matrix is the closure contract between product vision, normative
ownership, execution order, and falsifiable evidence. A detailed milestone
plan may replace fixture names or content, but it must preserve the capability
and all assertions of the assigned scenarios.

| Mandatory capability | Normative owner or boundary | Milestone | Primary scenarios |
|---|---|---|---|
| Immutable execution semantics and checked extension artifacts | definition, artifact, compiler, linker, and activation owners | M1, M5 | 8, 13 |
| Turn-based control over deterministic discrete-event virtual time | scheduler, admission frontier, moment protocol, and actor-control boundary | M2, M3, M6, M8 | 1, 2, 10, 18, 20, 26 |
| Single deterministic mutation authority and atomic conflict resolution | engine proposal verification and runtime authority protocol | M1, M2 | 1, 10, 14, 15, 19 |
| Actor-relative grounded control without hidden-state leakage | context projection, decision candidate, and private lowering boundary | M3 | 1, 2, 4, 16 |
| Independent evidence, appraisal, intent, activity, action, and process lifecycles | cognition/agency state owners and post-commit routing | M4 | 2, 3, 5, 11, 12, 16 |
| Durable restore, verification, branching, lineage, and delivery | repository, history, attempt-control, artifact-retention, and epoch owners | M5 | 5, 6, 8, 14, 15, 19 |
| CLI/MCP actor-control parity and authenticated projection-safe evaluation | product adapter and captured-evaluation boundaries | M6 | 5, 20 |
| AI-assisted foundation T1 action/event authoring with preview and explicit child epoch | compiler, artifact, preview composition, and epoch activation boundaries | M6 | 21 |
| Controller, lifecycle evaluator, and author remain distinct AI roles | actor-control, lifecycle port, captured-evaluation, and authoring boundaries | M4, M6 | 5, 20, 21 |
| Reproducible experiment, trace, metric, and reuse isolation | lab artifact, execution-specification, trajectory, and analysis owners | M6 | 9, 18, 19 |
| Individual Detailed/Background/Dormant refinement with work replacement | resolution-scope owner and checked resolution transition | M7 | 7, 17, 22 |
| Bounded local square grid and regional topology share identity and causal time | spatial domain owners and resolution-scoped process state | M8 | 22, 26 |
| Capability-derived physical interaction and cross-primitive atomicity | standard-world state owners, grounded actions, footprints, and effect primitives | M8 | 23, 24, 26 |
| Capability-changing growth alters grounded candidate space | capability state owner, context projection, and action-definition requirements | M8 | 23, 26 |
| Physical-to-evidence-to-social-to-agency causality | physical, epistemic, social, and agency state owners joined only by typed later work | M8 | 3, 11, 25, 26 |
| Natural-language expression cannot bypass typed social acts | presentation/capture boundary and social proposal authority | M8 | 25, 26 |
| Process and stage-specific condition families are checked executable artifacts | family-specific compiler, verifier, interpreter, and activation owners | M8 | 24, 26 |
| First reusable T0 object-archetype family is narrow data consumed by checked root materialization | pack, authoring, artifact, and root-materializer owners | M8 | 23, 26 |
| A later existing-vocabulary T0/T1 change remains local to authoring and activation | definition families, checked artifact, linker, root materializer, and activation registry | M8 | 23, 26 |
| New T3 semantics remain concrete owner-local participants | primitive state/gate owner and composition-root wiring | M8 | 24, 26 |
| Reusable content, scenario provenance, and materialized initial state stay distinct | pack, world-lab, root materializer, and runtime owners | M8 | 9, 23, 26 |
| New gameplay state survives persistence and resolution changes | checkpoint/history and resolution-transition owners | M8 | 22, 26 |

Roadmap completion means that these architecture capabilities are executable
and independently inspectable. It does not mean production content volume,
balance, graphical presentation, unrestricted generated code, population
aggregation, or every possible game system is complete.

## Gate applied to every milestone

For the contracts introduced by the milestone:

- formatting, workspace compilation, lints, tests, and whitespace checks pass;
- the exact direct dependency allowlist passes;
- focused unit, negative, state-machine, and compile-fail privacy tests pass;
- durable storage representations round-trip through decoding and owner
  validation;
- canonical identities and repeated-run fingerprints are stable;
- every concrete authoritative head refines a valid formal `Σ`;
- every concrete world publication maps to `Admit`, `Fire`, or `Manage`;
- documentation, public API, and implementation ownership agree;
- no forbidden dependency or superseded symbol has been introduced.

Completion evidence is recorded in the milestone plan before the next
milestone becomes active.
