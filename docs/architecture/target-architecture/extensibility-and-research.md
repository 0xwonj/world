# Extensibility and Research Architecture

## Purpose

This document defines how game content, future DSLs, trusted primitives,
optional evaluators, experiments, traces, and metrics extend the engine without
weakening runtime authority or reproducibility.

## Extension principle

Extensibility is a ladder of simulation authority, not one generic plugin
interface.

| Tier | Form | Simulation authority | Host execution trust |
|---|---|---|---|
| T0 | Content data | Instantiate checked vocabulary | Unchecked serialized data; trusted engine validation |
| T1 | Checked typed IR | Interpreted proposal/effect vocabulary | Unchecked serialized data; trusted bounded interpreter |
| T2 | Evaluator | Return bounded typed proposals | Host-trusted native code or isolated process/Wasm |
| T3 | Statically linked engine extension | Install trusted primitive semantics | Engine-trusted |

Artifact signing proves origin and integrity. It does not grant capabilities.

Simulation authority and host isolation are separate axes. An in-process native
Rust, C++, Python, or learned evaluator is proposal-only with respect to the
simulation, but it is still trusted by the host process. Genuinely untrusted
code requires Wasm or OS/process isolation. A trusted remote adapter owns
network access and materializes only the evaluator's bounded role input.

### T0: content data

Examples:

- reusable actor, object, location, item, and stat declarations;
- concrete recipes, encounters, and dialogue;
- game- or lab-owned scenario parameters and initial-state declarations.

Content data contains no executable semantic operation.

T0 is an authority tier, not one universal artifact family. Its durable home
depends on what the data means:

```text
reusable content declaration
  -> pack-owned checked data
  -> may be referenced by RuntimeDefinitionSet definitions

research scenario declaration
  -> world-lab-owned ScenarioArtifact
  -> planning provenance and a checked recipe for an InitialStateRoot

game/product initial-world source
  -> game- or product-owned authoring input
  -> checked materialization into an InitialStateRoot

InitialStateRoot
  -> exact materialized authoritative starting state
  -> runtime contract, not source content and not a second definition set
```

The same actor archetype may be reusable pack content while one actor instance
is materialized in a root. Pack identity does not replace instance identity,
and a `ScenarioArtifact` does not become runtime mutation authority.

The foundation `ArtifactBlobV1` contains no reusable T0 content-data family;
it contains only its checked T1 action/event vocabulary. M6 exercises that
existing T1 vocabulary through source authoring, diagnostics, preview, and
child-epoch construction. M8 introduces the first minimal reusable T0
pack-content family only with the concrete gameplay and root-materialization
consumer that fixes its schema and validation rules. That addition uses a
successor artifact protocol rather than changing the meaning of
`ArtifactBlobV1`.

Root materialization is an offline checked construction. It resolves every
definition reference against one exact `RuntimeDefinitionSet`, validates the
complete accepted state, runtime-control state, scheduler, lineage, and
execution compatibility, and then produces the canonical
`InitialStateRootId`. Product- or lab-specific source schemas remain outside
the runtime. After session creation, new content can affect accepted state only
through the normal admission, moment, or management protocols; changing a
source pack or scenario never mutates a live epoch.

### T1: checked typed IR

Examples:

- action and process definitions;
- effect programs built from installed primitives;
- role-binding and requirement expressions;
- projection and observation rules;
- action-decision meaning declarations;
- appraisal rules and intent templates;
- stage-checked conditions.

T1 declarations may describe an effect only through a sealed, typed vocabulary
whose verifier and interpreter are trusted engine code.

Artifact loading has one outer byte limit. Every T1 family separately declares
the semantic limits that affect execution, such as binding and effect
cardinality, structural termination, or deterministic fuel. Exhausting an
execution limit rejects the proposed effect before commit and produces no
partial change. When a family requires a cost model beyond its structural
cardinality, that policy becomes part of the semantic interface catalog and
execution contract: its descriptor digest enters the required interface
closure and the normalized `ExecutionSemanticsManifest`.

### T2: proposal-only computation

Future sandboxed Wasm, an isolated learned model, an LLM service, or a
host-trusted local evaluator may implement one bounded role such as action
scoring or semantic interpretation.

It receives capability-scoped immutable typed input and returns a checked
result. The evaluator role never receives a world pointer, store handle,
transaction builder, or arbitrary runtime command surface. Isolated
implementations receive no ambient network, filesystem, clock, or entropy
capability unless explicitly granted.

### T3: trusted engine extensions

Examples:

- a new physical or economic primitive;
- standard-world primitive semantics.

These are initially statically linked Rust crates installed during
composition-root construction. The resulting `EngineDistribution` exposes a
serializable `SemanticInterfaceCatalog` for authoring and the matching
host-trusted semantic implementations for activation. The catalog contains
declarative contracts, not executable implementations. There is no native
dynamic Rust plugin ABI.

Storage, indexing, and result-equivalent accelerators are infrastructure
providers rather than content semantic extensions. A pathfinder whose
tie-breaking changes the selected logical path is a semantic policy; an
implementation that preserves a canonical result is infrastructure.

Even trusted primitives receive narrow staging capabilities and return a
prepared result. They do not receive unrestricted mutable session state.

## Pack compilation

Runtime never consumes source packs directly. Compilation and loading are
separate outer paths that converge on one defs-owned validator:

```text
source:
pack manifests
  -> parse manifests
  -> resolve one exact package/source identity per PackKey
  -> ResolvedPackageGraph
  -> compile dependencies topologically
  -> parse source AST
  -> resolve imports against exact export/interface digests
  -> resolved typed declarations
  -> complete lowering to executable family IR
  -> ArtifactData
  -> validate(ArtifactData, SemanticInterfaceCatalog)
  -> deterministic ArtifactBlob encoding
  -> ArtifactDescriptor + ArtifactEnvelope + sealed VerifiedPackArtifact

load:
ArtifactEnvelope
  -> format/version/length/digest checks
  -> decode ArtifactData
  -> validate(ArtifactData, SemanticInterfaceCatalog)
  -> sealed VerifiedPackArtifact

both:
  -> finalized artifact-digest PackLock
  -> linked immutable RuntimeDefinitionSet
  -> process-local ActivatedDefinitionRegistry
```

`ResolvedPackageGraph` is an internal compiler value containing exact package
version and source snapshot identities; source packages do not yet have
artifact digests. `PackLock` is finalized only after every selected source has
compiled or a verified binary artifact has loaded.

Termination conditions are compiled against a versioned `TerminationView`
schema. Resolution and stage checking reject reads of undeclared accepted-state
projections, raw scheduler entries, authority/history metadata, deduplication
state, or private lifecycle state. A new semantic termination signal must first
become an explicit typed projection with an owner and compatibility rule.

Every lowering has an explicit semantic-preservation obligation. The initial
compiler favors direct, typed, verifier-checked lowering and
source/IR differential tests over optimization. A later non-obvious optimizer
must preserve a mechanically enforced construction invariant or validate each
output against its input semantics; performance alone does not authorize an
uncheckable rewrite.

### Artifact records

```text
PackManifest
  manifest schema
  pack key and semantic version
  compatible engine protocol
  dependencies
  exported namespaces
  required semantic interfaces
  declared definition families

ArtifactDescriptor
  media type and artifact-format version
  digest algorithm, exact blob digest, and byte length
  optional signature descriptors
  optional source-map sidecar descriptors

ArtifactEnvelope
  unchecked ArtifactDescriptor
  unchecked serialized ArtifactBlob bytes

ArtifactData
  compiler-produced or decoded domain representation
  normalized manifest, definitions, and exact interface references

VerifiedPackArtifact
  sealed in-memory verified value
  artifact-format version
  manifest
  per-family schema versions
  normalized definitions
  capability requirements
  exact import/export and semantic-interface digests
  exact ArtifactBlob digest
  runtime semantic fingerprint

PackLock
  resolver version
  exact package/source identities
  exact artifact identities and digests
  exact resolved dependency graph
  exact direct import/export edges
  semantic-interface descriptor-schema version
  exact required semantic-interface digests

RuntimeDefinitionSet
  exact linked artifact graph
  linked executable definitions
  required semantic-interface closure
  canonical definition-set body

ActivatedDefinitionRegistry
  RuntimeDefinitionSet digest
  verified semantic implementation bindings
  process-local intern mappings
  dispatch tables, indexes, and reconstructible caches
```

Semantic version ranges express compatibility. Content digests express exact
identity. Both are required.

Serialized artifacts, locks, and definition sets are unchecked
representations. Artifact loading checks the descriptor, blob length, and exact
blob digest, decodes `ArtifactData`, and invokes the same catalog-aware domain
validator used by authoring. That validator checks executable-family
invariants, semantic execution limits, stage authority, references, and exact
interface digests before a private constructor yields a
`VerifiedPackArtifact`. The compiler validates its in-memory `ArtifactData`
directly and does not encode and decode its own output.

Linking checks the exact dependency graph, direct import/export edges, symbol
closure, and cross-pack invariants before yielding a `RuntimeDefinitionSet`.
Activation then verifies the required semantic-interface closure against
installed implementations before yielding an `ActivatedDefinitionRegistry`.
A valid signature never skips domain validation or grants a higher extension
tier.

The exact blob digest preimage is the exact stored `ArtifactBlob` bytes. The
project-owned emitter is deterministic, but loading does not require every
valid semantic value to have one possible byte representation. Alternative
accepted representations have different blob and exact-set identities while
the normalized runtime semantic fingerprint may remain equal. The digest lives
in the external `ArtifactDescriptor`, avoiding a self-referential field.
Signatures cover the descriptor/digest according to their declared scheme.

The runtime semantic fingerprint separately covers:

```text
canonical normalized behavior-relevant IR
semantic canonicalization version
required semantic-interface digests
```

Source maps are content-addressed sidecars by default. Changing debug metadata
changes the sidecar/descriptor closure without changing the runtime semantic
fingerprint; changing exact artifact bytes changes the blob digest.

Semantic fingerprints support equivalence diagnostics, cache reuse, and
human-readable change classification. They never independently authorize
loading, checkpoint compatibility, or reproducible execution; those decisions
use exact artifact/definition-set digests and typed compatibility manifests.

Definition-set identity is likewise non-self-referential:

```text
RuntimeDefinitionSetDigest =
  H(canonical RuntimeDefinitionSet body without a digest field)
```

The process-local `ActivatedDefinitionRegistry` has no independent semantic
identity. It must be reconstructible from the exact definition set and matching
installed semantic implementations.

### Definition identity

Durable identity is qualified:

```text
DefinitionKey
  pack key
  local name
```

Numeric IDs may be interned for speed inside one
`ActivatedDefinitionRegistry`. They are never durable identities in saves,
events, artifacts, checkpoints, or experiment manifests. Durable records use
`DefinitionKey`; activation reconstructs any process-local mapping.

One `RuntimeDefinitionSet` contains exactly one selected artifact per
`PackKey`. Incompatible constraints fail resolution. If parallel major
versions are ever required, explicit package-instance aliases and import
identity must be designed first; the resolver may not reinterpret durable
logical definition keys implicitly.

### Composition rules

The default composition policy is strict:

- duplicate exported keys are errors;
- load order has no semantic meaning;
- silent override is forbidden;
- dependency cycles fail unless a future named family explicitly supports
  them;
- imported names and semantic interfaces bind exact digests, not only version
  ranges;
- unknown required operations fail closed;
- installed runtime implementations must exactly satisfy the definition set's
  required semantic-interface closure;
- additional unused installed semantic interfaces are allowed and do not
  change the definition-set identity;
- activation of a complete `RuntimeDefinitionSet` is atomic.

A future patch artifact must name the exact base digest and declare
verifier-visible replacements. There is no ambient “last pack wins” behavior.

An authoritative session uses one immutable `RuntimeDefinitionSet`. Editor
preview may rebuild a separate non-authoritative session. Live semantic hot
reload does not mutate a reproducible session in place.

## Executable definition families

The engine shares compiler infrastructure, not one universal instruction set.
It also does not create one `*IR` type for every domain noun.

Definition families enter the artifact protocol only with an immediate
producer, verifier, interpreter or other runtime consumer, identity rule, and
validation scenario. The rollout is therefore explicit:

```text
foundation artifact protocol (M1-M4 evidence)
  checked action definitions
  checked physical-event definitions
  nonempty ordered typed effect calls embedded in an action

product authoring proof (M6)
  the same existing T1 vocabulary through source, diagnostics, preview,
  and explicit child-epoch construction

gameplay composition proof (M8)
  the first minimal reusable T0 content-data family, with a real
    gameplay and root-materialization consumer
  the first checked process and stage-specific condition families
  plus only the observation, social, capability, or other declarations
  required by the accepted cross-system scenario
  a separate locality fixture composed from the now-installed T0/T1
    vocabulary without another artifact-family or authority-kernel change
```

The foundation's ordered effect-call sequence is the first effect-program
representation. It need not acquire a public `EffectProgramIR` wrapper merely
to match an architectural noun. A separately named effect IR is introduced
only when a real producer or consumer needs shared programs, structured
control, independent verification, or transformation.

The relocation process introduced by M4 is a concrete trusted runtime process,
not evidence that an authored process-definition family already exists.
Conversely, M8 must not satisfy its process gate by hard-coding another closed
process variant and calling it pack extensibility.

Observation, projection, appraisal, intent-template, scheduler-monitor, and
decision-semantics representations are introduced only with their first real
producer, verifier, and consumer. A checked declarative record need not be
called IR unless it has a meaningful lowering, transformation, or interpreter
boundary.

Condition-family stage remains part of the type. Actor-relative discovery,
authoritative legality, observation, agency monitoring, and scheduler
deadlines cannot execute one another's conditions even when they share a
private predicate implementation. The private implementation may be
capability-indexed; serialized roots and public interpreter contracts remain
family-specific.

In the target, `StudyDesignArtifact` and `MetricSetArtifact` remain validated,
canonical structured artifacts owned by `world-lab`, not additional IR
families. They never enter the `RuntimeDefinitionSet`, its exact digest,
checkpoint compatibility, or trajectory identity. A future analysis language
may introduce analysis IR only when it has a real source language, lowering
boundary, interpreter or code generator, and independent migration policy.

Runtime and analysis compilers may reuse private libraries for names, types,
values, bindings, source spans, provenance, diagnostics, and dependency
analysis. Each executable runtime family retains its own:

- legal operations;
- authority class;
- verification rules;
- runtime interpreter;
- deterministic structural limits and any consumer-required cost model;
- version and migration policy.

Ordinary packs may instantiate and compose installed operations. A
`SemanticInterfaceCatalog` entry is declarative: it describes signatures,
legal families and stages, authority/effect constraints, and the structural or
cost rules needed by current consumers. The initial authoring boundary accepts
only operations that the installed generic lowering and validator understand
from that catalog. Custom compiler hooks are unsupported, not implicitly
discovered from the runtime distribution. If a future primitive genuinely
requires custom lowering or validation code, it first requires an explicit
composition-root injection boundary, exact compiler-extension set identity,
dependency direction, artifact-closure binding, and failure model. Pack data
can never supply that code. The `EngineDistribution` supplies the matching
host-trusted runtime implementation and configuration identity.

Pack activation fails unless every required semantic-interface digest resolves
to the matching implementation.

### Definition activation and primitive composition

`ActivatedDefinitionRegistry` reconstructs checked T0/T1 definitions, exact
semantic bindings, indexes, and caches. It does not own T3 state owners or turn
them into an open registry of mutable subsystems.

A T3 primitive remains a concrete, statically linked participant installed by
the composition root. Adding one may add its own private state, gate,
preparation logic, codecs, migration, and wiring, but it must not change the
authority laws or grant packs an implementation callback. Cross-owner domain
work composes through the existing prepared-subtransaction and combined
invariant protocol rather than through handler order or direct mutable calls.

This distinction is a required M8 proof:

- after the required minimal T0 family is installed, one separate mechanic
  composed entirely from the then-existing T0/T1 vocabulary changes
  definitions and tests, not artifact-family schemas, the authority kernel, or
  unrelated primitive owners;
- one genuinely new T3 primitive may add concrete owner-local code and
  composition-root wiring without reversing dependency direction or changing
  unrelated public APIs;
- a transition spanning existing owners declares typed reads, writes,
  resources, invariants, and participating-gate receipts before one atomic
  commit;
- no global type-erased state-owner, string resource path, or universal
  semantic dispatcher is presumed in advance.

### Source syntax

Source syntax is replaceable. Structured data is sufficient initially.

The stable semantic contract is:

```text
fully lowered executable family IR
+ independent verifier contract
+ deterministic owner-defined artifact encoding
+ interpreter behavior
```

A duration field in family IR uses a canonical configuration-independent exact
unit. The session's `ExecutionConfigArtifact` selects the simulation quantum;
session creation must prove exact representability and materialize integer
`SimDuration` values. Pack identity therefore does not depend on a
session-specific time quantum.

A future constraint language or Starlark-like authoring frontend may generate
the same declarations at authoring time. It does not become live gameplay
authority.

Parsing, resolution, typing, lowering, verification, and linking are distinct
semantic phases. They do not require a public data type, crate, or configurable
pass manager for every arrow. Compiler-internal representations are introduced
only when a phase needs an independently testable invariant or more than one
producer or consumer.

## Version and migration model

One opaque version number cannot represent every compatibility question.

At minimum, distinguish:

```text
SemanticVersion
  human-facing compatibility and dependency resolution

SchemaVersion
  serialized format or IR-family dialect

ArtifactDigest
  exact immutable content identity

SemanticFingerprint
  exact behavior-relevant normalized meaning
```

Additional named compatibility identities include:

```text
EngineProtocolVersion
RuntimeDefinitionSetDigest
SemanticInterfaceCatalogSchemaVersion
ExecutionConfigDigest
EventSchemaVersion
CheckpointSchemaVersion
HistorySchemaVersion
TraceSchemaVersion
MetricSetVersion
RandomKeyPolicyVersion
SchedulerPolicyVersion
```

Rules:

- every source declares its source dialect;
- every artifact declares its container and family versions;
- migrations are explicit transformations, never implicit reinterpretation;
- runtime supports a bounded declared artifact-version window;
- a checkpoint records the exact `RuntimeDefinitionSetDigest`;
- loading a checkpoint against another definition set requires explicit
  migration;
- deleted field or operation identifiers are reserved rather than silently
  reused;
- old scientific runs remain reproducible through preserved exact artifacts
  and engine identities, not an infinite promise that the latest engine
  emulates every past semantic bug.

Compatibility answers are typed:

- execution-semantics compatibility controls verification reruns;
- persistence-format readability/migration controls restore;
- analysis/observation-format conversion controls traces and reports.

A trace schema change does not invalidate a trajectory, and a compatible
history reader does not imply identical execution semantics.

Migration ownership is separate:

```text
world-authoring
  source and compiled-artifact upgraders

world-runtime / world-engine readers
  bounded history, checkpoint, and trace schema adapters

offline CheckpointMigrationEngine
  old checkpoint + exact old/new artifact closures
    -> new child-epoch root checkpoint + provenance
```

The offline migrator never mutates a live session.

### Artifact closure and retention

Content retention uses two deliberately distinct, acyclic closure roles. The
first is fixed before execution and cannot grow:

```text
ResolvedExecutionClosureManifest
  canonical ExecutionSpec, InitialStateRoot, and ExecutionSemanticsManifest
  runtime pack and required lifecycle-profile artifacts
  configured external evaluator modules
  required semantic-interface descriptors and ExecutionConfigArtifact
  engine distribution/build artifact or reproducible build recipe

  excludes:
    RunAttemptControl and AttemptControlTraceArtifact
    captured run inputs and evaluator results
    authority history, checkpoints, receipts, and RunFinalization
    RunArtifactSet and every downstream study/analysis artifact
```

Its digest is part of the canonical `AttemptCreationDescriptor`. It therefore
pins the exact immutable material needed to construct or reopen
`ResolvedExecution` without depending on anything produced by that attempt.

The second role is a frozen, root-relative snapshot for a checkpoint, archive,
or finalized run. The root artifact that references the manifest is excluded
from the manifest itself:

```text
WorldCheckpoint, SessionArchive, or RunArtifactSet ArtifactClosureManifest
  ResolvedExecutionClosureManifest and its dependency closure
  captured ordinary inputs and evaluator results
  AttemptControlTraceArtifact, matching StepPublicationReceipts, and
    RunFinalization, when retaining a finalized attempt
  required history, final-state, capture, and trace partitions

RunCaseResult dependency closure
  RunCase
  selected RunArtifactSet and its dependency closure, when one exists
  separately referenced failure, exclusion, or reuse evidence, when present

AnalysisManifest or report dependency closure
  StudyDesignArtifact, MetricSetArtifact, and labeled RunCaseResults
  analysis implementation, trace/report inputs, and uncertainty artifacts
  optional ScenarioArtifacts reachable as study/planning provenance
```

A `SessionArchive` closure additionally follows every settled
`AttemptArtifactRetention` reference. `RetainedBy(R, M1)` includes both the
`RunArtifactSet` root and its complete closure. Archive creation first
reconciles or rejects an in-flight handoff intent, so neither same-domain
restore nor portable inspection can observe a dangling provisional target. A
discarded attempt cannot be exported as a restorable/verification
`SessionArchive`; only its permanent control tombstone remains inspectable.

In particular, a `RunArtifactSet` closure never includes a downstream
`RunCaseResult`, study assignment, analysis manifest, metric, or report.
Selecting the same retained run for another study therefore changes no run
artifact or digest.

The local artifact store pins reachable blobs while any retained root depends
on them and supports portable read-only archive export/import; same-domain
runtime restoration is the separate create-or-open protocol. Attempt creation
materializes and pins the immutable `ResolvedExecutionClosureManifest` before
or atomically with publishing a live control record. The live authoritative
session, scheduler, and delivery roots separately pin dynamic captured inputs,
history, and pending work; the creation manifest never grows to include them.
`Active` and `StepReserved` attempts cannot release these required pins.
After finalization, the durable `AttemptArtifactRetention` state either owns
those pins, prepares an owner-scoped handoff, names a pinned
`RunArtifactSet`/closure root, or records an explicit discard tombstone.
Handoff acquires the target before releasing the source; discard installs the
permanent creation-descriptor/fingerprint non-reuse tombstone before garbage
collection. Idempotent recovery may leave extra pins but never a zero-pin
window. A digest proves identity, not availability. Exact verification rerun
requires the referenced engine artifact or reproducible build, not merely its
identifier.

## Future Wasm boundary

The architecture reserves Wasm for T2 evaluators, but does not implement it
until a concrete extension role requires it.

A future component must:

- implement one role-specific typed interface;
- receive a fully materialized, capability-scoped immutable input;
- return a bounded proposal;
- receive no filesystem, network, environment, wall clock, or entropy by
  default;
- never receive a state or transaction handle;
- run under deterministic fuel and memory/table/instance limits;
- trap without partial effects;
- pass the same result verifier as a native evaluator.

The run record captures module digest, interface version, limits, input hashes,
and explicit randomness. A general WASI environment is not granted by default.

## Gameplay-composition research disposition

The gameplay-composition research separates accepted architecture from
hypotheses that still need implementation evidence. Its recommendations have
the following normative disposition:

| Research recommendation | Disposition |
|---|---|
| Keep T0/T1, T2, and T3 contracts distinct | Accepted clarification of the trust ladder |
| Keep definition activation separate from T3 state-owner composition | Accepted; no generic state-owner registry |
| Preserve semantic epochs as the evolution boundary | Already normative; M5 proves it and M6 exercises it |
| Use the existing prepared-subtransaction contract across owners | Already normative; M8 must supply cross-primitive evidence |
| Separate pure derivation from authoritative transition | Accepted architectural law; concrete incremental representation remains evidence-gated |
| Build a definition-level semantic dependency graph | Investigate in M8; not a required universal runtime graph or accepted public API |
| Prove incremental evaluation against full recomputation | Required only for an incremental derived view actually introduced by a milestone |
| Use the three interacting gameplay slices as a falsification suite | Adopted as the M8 composition gate |

The three slices may motivate narrower shared types after they run. They do
not pre-authorize a universal component model, effect language, resource
registry, event bus, or primitive plugin trait.

## Research architecture

Research is a first-class leaf of the production engine, not an alternate
mutation path.

```text
ExperimentSuite
  scenario families
  frozen scenarios
  lifecycle profiles
  StudyDesignArtifact
  intended MetricSetArtifacts

ExperimentPlan
  -> deterministic study assignments
  -> resolve each assignment's trajectory-affecting settings
  -> ExecutionSpec

RunCase
  RunCaseId
  -> references StudyDesignArtifact and canonical assignment
  -> references exact optional ScenarioArtifact as planning provenance
  -> references ExecutionSpecId

RunCaseResult
  -> maps RunCaseId to selected RunAttemptId
  -> references immutable RunArtifactSet and TrajectoryId, when produced
  -> records selection, failure, exclusion, or reuse provenance

AttemptAuthorityDomainId + ExecutionSpecId + runner-assigned attempt key
  -> RunAttemptId
  -> RunAttemptControl + WorldSession
  -> RunFinalization + RunArtifactSet + RunResult

AnalysisManifest
  StudyDesignArtifact digest
  RunCaseResult digests
  MetricSetArtifact digest
  analysis implementation and uncertainty method
  prespecification/conformance status

AnalysisManifest -> MetricReport
```

`ExecutionSpecId` identifies exact pre-run execution configuration.
`RunCaseId` identifies that configuration's assignment within one study.
`TrajectoryId` identifies the authoritative history actually produced after
admitted inputs and deferred results are known. Analysis identity is separate
from all three. Changing a metric, report layout, or analysis method does not
create a new execution specification or simulation trajectory.

### Study design

The minimal `StudyDesignArtifact` records:

```text
factor and condition identities
scenario-family/selection rules, when scenarios are used
pairing/block identity
replicate and seed schedule
prespecified primary and secondary metric identities
analysis hypotheses and estimands
train/validation/held-out split assignment
run budget and stopping rule
failed, cancelled, and excluded-run treatment
analysis and uncertainty method identities
```

This is not a statistics DSL. It prevents post-hoc changes to pairing,
exclusion, stopping, or primary metrics from becoming invisible.

An actual `AnalysisManifest` references the exact `StudyDesignArtifact` and
labels each result as one of:

```text
PrespecifiedPrimary
PrespecifiedSecondary
ProtocolDeviation(reason)
Exploratory(reason)
```

This records honest deviation without making the original study artifact
mutable.

### Frozen scenarios

A scenario generator produces an immutable `ScenarioArtifact`. Paired policy
runs reference the same artifact rather than independently regenerating
nominally identical worlds.

The artifact records initial state, parameter assignments, required definition
capabilities, permitted/default controller assignments, termination
requirements/defaults, and scenario assumptions. It does not silently own a
second exact copy of settings later fixed by the execution specification.

Experiment planning deterministically resolves scenario requirements/defaults,
the selected definition set, lifecycle profiles, execution configuration, and
trajectory-affecting values selected by the study assignment into one exact
`ExecutionSpec` body. Conflicts or unresolved requirements fail before a run
case exists. An ODD-inspired model card documents purpose, entities,
scheduling, processes, initialization, inputs, and assumptions for human
readers.

The full `ScenarioArtifact` remains planning and provenance input, not a member
of `ExecutionSpecId`. After resolution, its behavior-affecting contribution is
already represented by `InitialStateRootId`, selected execution semantics,
resolved termination, seed, and external-input binding. Descriptive
assumptions or an overridden default therefore cannot perturb trajectory
identity.

### Lifecycle profiles

Profiles are scoped by lifecycle:

```text
EpistemicAssimilationProfile
AppraisalProfile
SocialInterpretationProfile
IntentProfile
ActivityControllerProfile
ActionPolicyProfile
```

A run profile selects one implementation requirement for each enabled port;
the `SemanticImplementationSet` binds those requirements to exact installed
implementations. The engine does not expose one cross-lifecycle arbitrary pass
list.

This permits controlled ablations:

```text
same frozen scenario
same actor view
same grounded candidate set
same intent/activity state
different ActionPolicy
```

or:

```text
same beliefs, AppraisalResult, and grounded intent candidate set
different IntentPolicy
same remaining lifecycle implementations
```

A profile that changes memory, appraisal, intent, planning, and action at once
is a treatment bundle, not a component-level ablation.

Every persistent port declares its state schema and predecessor
compatibility. Replacing it from a mid-run checkpoint requires explicit
migration/reset provenance; only stateless compatible ports may be swapped
directly on a branch.

### Execution specification and run artifacts

Identity is non-self-referential:

```text
ExecutionSpecId =
  H(canonical ExecutionSpec body without any ID field)

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

`ExecutionSpecId` is reusable configuration identity. `RunCaseId` is a
study-scoped assignment identity and may differ when another study assigns the
same execution specification. `RunAttemptId` identifies a physical attempt of
the execution specification inside one exclusive writable attempt-authority
domain, not of the study-scoped case. A fresh independent control domain has a
different durable `AttemptAuthorityDomainId`; that host identity never enters
the trajectory or world semantics.

The runner-assigned attempt key is consumed by an atomic create-or-open
operation. The persisted canonical `AttemptCreationDescriptor` contains the
complete `AttemptBinding`, including `AttemptAuthorityDomainId`, raw
runner-assigned key, root cursor, exact resolved execution
`ResolvedExecutionClosureManifest` digest, and control format. The loader
rederives both
`RunAttemptId = H(domain || ExecutionSpecId || attempt key)` and the
descriptor fingerprint. An exact retry opens the existing attempt; any
different descriptor or fingerprint under the same ID is rejected. Physical
attempt identity remains outside authority-record, randomness, and
`TrajectoryId` preimages.

The scenario-provenance field is either one exact `ScenarioArtifact` digest or
an explicit no-scenario/root-origin value. It participates in the study-scoped
assignment key but never in `ExecutionSpecId` or `TrajectoryId`. `RunCase` is
the immutable assignment artifact. A separate immutable `RunCaseResult` binds
that assignment to the selected attempt, artifact set, and trajectory (or
terminal no-trajectory status) under the study's declared retry, exclusion, and
reuse rules. `AnalysisManifest` references these result mappings rather than
unlabeled run artifacts.

An existing run may be reused only by referencing its exact `TrajectoryId` and
when its immutable `RunArtifactSet` satisfies the new study's capture
requirements. Matching `ExecutionSpecId` alone is insufficient when external
inputs or deferred evaluator results were not frozen before execution.

`ExecutionSpec` contains only pre-run settings that may affect the trajectory:

```text
schema and canonicalization versions
InitialStateRootId
ExecutionSemanticsManifest digest
root seed
resolved TerminationContract
external-input schedule/binding artifact digest
```

The resolved `TerminationContract` is a canonical value inside the
specification: ordered typed clauses, declared `TerminationView` reads,
bounded condition roots, and exact semantic-interface references. It is not a
callback or arbitrary pack code. Authoring compiles and verifies it, and
`Engine::resolve_execution` independently reverifies its schema,
canonicalization, boundedness, clause order, stage/read legality, and interface
closure before sealing `ResolvedExecution`.

`ExecutionSemanticsManifest` is the sole normalized identity for the
`EngineProtocolVersion`, `SemanticImplementationSet`,
`RuntimeDefinitionSet`, required semantic-interface closure,
`LifecycleProfiles`, and `ExecutionConfigArtifact`. RNG/key construction and any
branch-affecting numerical or platform choices are typed configuration
requirements closed by those semantic implementations. An `ExecutionSpec` may
carry component digests as verified denormalized indexes, but they cannot form
independent competing identities.

`InitialStateRootId` hashes a canonical root-state body that contains no
`ExecutionSpecId` or self-ID. Scenario materialization, branching, or migration
produces that root first; the specification then references it. The root owns
child lineage using only parent/cursor and migration/reset references,
preventing both duplicate ownership and a specification/checkpoint identity
cycle.

The external-input artifact either freezes a planned schedule or declares the
authorized channels and admission policy. Actual dynamic input payloads and
deferred results remain captured in authority history and therefore distinguish
`TrajectoryId`, not `ExecutionSpecId`.

The execution configuration closes time quantum, RNG/key policy, scheduler,
conflict resolution, retries, coalescing, external-result admission,
resolution, run finalization, and every other state-affecting budget or policy.

The pure, versioned `TerminationContract` and manifest-fixed
`RunFinalizationPolicy` select one authority cursor under a durable
`RunAttemptControl` gate. They are execution-affecting even though
`RunFinalization` itself belongs to the host/run plane rather than world state.
The contract reads only its declared stage-checked `TerminationView`;
attempt-plane cancellation or failure enters through an explicit durable
`AttemptDisposition`.

Exact whole-build and platform artifacts remain reproduction provenance in the
artifact closure. They do not enter `ExecutionSpecId` merely because unrelated
code or an unused installed semantic interface changed. Any implementation
choice capable of changing the trajectory must appear in the
`SemanticImplementationSet` committed by the normalized
`ExecutionSemanticsManifest`.

`RunArtifactSet` records the execution outcome together with non-authoritative
capture and provenance:

```text
ExecutionSpecId
RunAttemptId
TrajectoryId
RunFinalization digest and terminal AuthorityCursor
RunResult status
durable history and final-state digests
AttemptControlTraceArtifact digest, including ReplayInputs with canonical
  creation and ExpectedObservations through finalization
CaptureProfile and TraceCompletenessManifest
host/platform observation metadata
root-relative ArtifactClosureManifest digest
exact engine build and host provenance
```

`AttemptControlTraceArtifact` independently captures the hash-chained accepted
control history in two typed partitions. `ReplayInputs` contains the canonical
attempt creation descriptor/binding and fingerprint, ordered host step intents
(`Admit` input references, exact `advance`/`drain_until` requests, and captured
host `Manage` references), and accepted exogenous cancellation and
host/external/engine-failure dispositions. Every exogenous control input
carries a logical root/before-step/reserved-step/after-reconciliation
injection anchor, never an expected cursor.
`ExpectedObservations` contains canonical derived reservations and step IDs,
published or no-publication reconciliations, matching publication receipts,
generated safety-management steps, semantic termination-clause selection, and
finalization barriers. Verification supplies only `ReplayInputs`; it
regenerates and compares `ExpectedObservations` and `RunFinalization`, so an
expected selector, clause, receipt, or terminal cursor can never mask a
regression by becoming input.
The durable control log retains its segments until this artifact takes
ownership. Rejected mismatched, unauthorized, or malformed control requests
and post-finalization retention/compaction housekeeping are not replay inputs;
an optional security audit may capture them only when declared by
`TraceCompletenessManifest`.

A safely finalized `RunArtifactSet` records one explicit result:

```text
Completed
TerminatedByScenario
BudgetExceeded
Cancelled
ExternalFailure
EngineFault
```

`RunResult` is a typed projection of the already durable
`RunFinalization.reason`; it does not choose the terminal cursor after the
fact. `Completed` denotes the contract's normal completion clause, while
`TerminatedByScenario` preserves an explicit scenario termination reason.

A fail-closed attempt-control or storage-integrity mismatch remains reserved
and has no trustworthy `RunFinalization`, `TrajectoryId`, or `RunResult`.
`RunCaseResult` records that no-trajectory operational failure plus retained
forensic evidence; it must not relabel an unverified cursor as `EngineFault`.
`EngineFault` above is reserved for an explicit durable engine-failure
disposition at a receipt-validated cursor.

Retries receive new `RunAttemptId`s while retaining the same
`ExecutionSpecId`. An immutable `RunCaseResult` records which attempt/result
and trajectory the run case selected.

External evaluator requests and exact responses are captured. Replaying a
captured result is possible; promising that a remote model will return the same
answer later is not.

### Run-level parallelism

The natural initial unit of parallelism is an independent `RunAttempt`
selected for a `RunCase`:

```text
scenario x seed x lifecycle profile x partner policy
```

This improves research throughput without complicating one world's causal
kernel. Paired comparisons share explicitly named exogenous random streams.

### Metrics

Metrics are versioned, typed, post-run definitions:

```text
MetricSetArtifact
  exact digest
  required capture capabilities
  MetricDescriptor[]

MetricDescriptor
  metric key and version
  value kind and unit
  source artifact requirements
  dimensions
  aggregation
  sampling policy
```

Scientific metrics are recomputable:

```text
AnalysisManifest(
  StudyDesignArtifact digest,
  immutable RunCaseResult digests,
  MetricSetArtifact digest,
  analysis method
) -> MetricReport
```

Changing metric code creates a new report. It does not rewrite the run.

Experiment planning validates capture requirements before execution. A metric
that needs candidate scores, evaluator payloads, or fine events cannot be
computed from a run whose `CaptureProfile` omitted them; the report records
`UnavailableInput` rather than fabricating an empty value.

Typed validity and outcome metrics are primary. Model- or LLM-judged metrics
are labeled, calibrated where possible, and never silently mixed with
ground-truth metrics. Reports preserve paired differences, uncertainty, and
held-out scenario or partner conditions where relevant.

## Trace and observability

### Record and attempt-control planes

```text
Authoritative history
  AuthorityRecord:
    Admission(Commands(IngressBatchRecord))
    | Admission(ActionEvaluation(ActionEvaluationAdmissionRecord))
    | Moment(MomentBatchRecord)
    | Management(ManagementBatchRecord)
  captured command inputs and separately captured evaluator results
  nested accepted/rejected AttemptRecords and accepted CommitRecords
  recovery and causal audit

Decision/provenance trace
  context builds
  lifecycle invocations
  candidate and score artifacts
  proposals, selections, fallback, links to durable attempts

Run-attempt control
  permanent AttemptBinding and create-or-open identity
  active/reserved/finalized state
  idempotent cancellation ledger and crash reconciliation receipts
  independent AttemptControlTraceArtifact
  terminal authority cursor and RunFinalization

Performance telemetry
  wall latency, allocation, queue depth, cache hits, token/cost usage
```

Authoritative history determines world restoration. Decision trace explains.
Run-attempt control gates execution and selects a retained prefix without
mutating world state. Telemetry operates the system. They are linked by IDs but
never substituted for one another.

Reliable adapter cursor/outbox state is another narrowly owned operational
delivery plane. It preserves at-least-once obligations but cannot affect the
world trajectory or analysis assignment.

### Trace graph

Decision provenance is a causal DAG rather than only a nested log:

```text
TraceNode
  deterministic TraceNodeId and content hash
  logical InvocationId
  node kind
  run, branch, actor, lifecycle, and implementation identity
  simulation moment and input revision
  input and output artifact references
  status and diagnostics
  random-key references
  capture status

TraceEdge
  Parent
  Uses
  Produces
  Supports
  DerivedFrom
  Attempts
  Commits
  Rejects
  Invalidates
```

This supports explanations that cross pipeline boundaries, such as:

```text
evidence
  -> appraisal signal
  -> interrupt proposal
  -> suspended activity
  -> selected defensive candidate
  -> runtime commit
```

Trace retention is configurable. Accepted state and commit history remain valid
without retaining every score artifact.

The engine, not an evaluator, mints invocation/node IDs, input/output artifact
hashes, causal edges, attempt links, and commit links. IDs derive from logical
run/lifecycle/cause identity rather than worker completion. Trace DAG
serialization is canonical before hashing.

Evaluator explanations are `SelfReportedRationale` claims with implementation
provenance. They cannot forge engine-observed support or causal edges.

Every run carries:

```text
TraceCompletenessManifest
  CaptureProfile
  captured node/artifact families
  sampled ranges
  NotCaptured markers
  Evicted markers and retention provenance
  canonicalization/schema version
```

`diff_runs` reports unavailable evidence when a required artifact was not
captured or was evicted. It must not report a false first divergence from
incomplete traces. Wall-clock ordering remains telemetry only.

### Inspector queries

The architecture should make these questions answerable:

- Which actor-visible facts supported this decision?
- Was a projection empty or unavailable?
- Why did an intent persist, change, or terminate?
- Which activity and process were active?
- Which candidates were grounded and why were others unavailable?
- Which score contributions selected the candidate?
- Did runtime reject the attempt because of staleness, legality, or conflict?
- Which commit changed this state?
- Which first available durable or trace artifact differs between two runs?

## Initial product scope

Build first:

- local immutable artifact storage with reachability retention, same-domain
  restore, and portable read-only archive import/export;
- synchronous deterministic engine library;
- headless CLI;
- experiment runner;
- JSON/JSONL diagnostic and trace export;
- read-only inspector APIs.

Defer:

- package registry and signing service;
- server and multiplayer transport;
- editor;
- dynamic native plugins;
- Wasm evaluator host;
- database requirement;
- distributed experiment scheduling;
- distributed simulation;
- cross-storage continuation of the same active `RunAttemptId` or reliable
  delivery owner, which requires exclusive fenced transfer;
- custom compiler lowering/verifier hooks beyond the declarative
  `SemanticInterfaceCatalog`;
- final metric query language;
- final action/effect DSL syntax.

The deferred products attach through authoring, host, inspection, or evaluator
ports. None requires widening runtime's mutation authority.

## Extensibility invariants

1. Source packs never execute inside the authoritative commit boundary.
2. Unknown required operations and versions are rejected before activation.
3. Definition identity is durable and qualified; numeric interning is local.
4. A session's `RuntimeDefinitionSet` is exact and immutable.
5. IR families share infrastructure but not one universal authority vocabulary.
6. T1 interpretation and isolated T2 computation are deterministically
   resource-bounded; budget failure has no partial effect.
7. In-process native evaluator and extension code is host-trusted; simulation
   proposal/commit authority remains narrow.
8. Optional evaluators cannot invent executable structure.
9. Every execution freezes one exact initial-state root, runtime definition
   set, required semantic-interface closure, lifecycle profile set, and
   complete execution-semantics/configuration identity. An experiment
   assignment that uses a scenario also freezes that exact planning artifact
   as study provenance.
10. Metrics and telemetry cannot influence simulation.
11. Research substitution occurs one typed lifecycle port at a time.
12. A future extension must have an owner, authority class, typed contract,
    versioning rule, failure behavior, and trace story before implementation.
13. Analysis artifacts never participate in runtime definition activation or
    trajectory identity.
14. Serialized artifacts are decoded and owner-validated before sealed runtime
    activation.
15. Retained checkpoints, runs, and reports pin their exact artifact closure.
16. Engine-minted trace structure is distinguished from evaluator
    self-reported rationale.
17. `ExecutionSpecId`, `RunCaseId`, `RunAttemptId`, and `TrajectoryId` have
    distinct, non-self-referential preimages.
18. Process-local activation indexes and intern mappings are reconstructible and
    never durable identity.
19. One physical run attempt has one durable finalization cursor selected by
    its exact termination/finalization contract; analysis never chooses or
    changes that prefix.
20. Reusable T0 declarations, scenario provenance, and the materialized
    `InitialStateRoot` remain distinct artifacts with distinct owners and
    identities.
21. After the required minimal T0 family is installed, a separate mechanic
    composed from the existing T0/T1 vocabulary requires neither another
    artifact-family schema nor an authority-kernel change; a new T3 primitive
    may add concrete owner-local implementation and composition-root wiring
    without reversing existing dependency direction.
22. Pure derivations cannot mutate accepted state, and any retained
    incremental result introduced by a milestone is checked against full
    recomputation.
23. Gameplay-facing API stabilization follows, rather than precedes, the M8
    cross-primitive and cross-system composition evidence.
