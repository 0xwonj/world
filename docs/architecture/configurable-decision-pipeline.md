# Configurable Decision Pipeline

## Purpose

This document defines the target architecture for configurable actor decision
pipelines in `world`.

The goal is not to build a general workflow engine. The goal is to make actor
decision structure configurable enough for research ablations while preserving
the engine's fixed truth, authority, access-control, provenance, and commit
boundaries.

The motivating use case is social-strategic evaluation:

- compare direct action, intent-only, appraisal-like, theory-of-mind, and
  full social-cognitive agent conditions
- swap a pass implementation between rules, heuristics, LLM generation, hybrid
  generation, or oracle labels
- add new cognitive/social representations without making `Appraisal` a sacred
  engine primitive
- keep every condition auditable, actor-relative, and comparable

This document should be read with:

- [Runtime Pipeline](runtime-pipeline.md)
- [Engine Architecture](engine.md)
- [Simulation Transition Compiler](../design/simulation-transition-compiler.md)
- [Truth, Authority, And Layer Boundaries](../design/truth-authority-and-layer-boundaries.md)
- [Social-Strategic Evaluation Program](../research/social-strategic-evaluation-program.md)

## Compiler Reference Lessons

The architecture should borrow from compiler infrastructure, but selectively.

### MLIR

MLIR's useful lesson is not "copy MLIR". Its useful lesson is that an
extensible system can remain tractable when extension happens through typed
dialects, operation contracts, traits/interfaces, verifiers, and staged
lowering. MLIR dialects define operations, attributes, and types under a
namespace; multiple dialects can coexist and be consumed by passes. MLIR also
uses traits and interfaces so generic transformations can reason about common
properties without understanding every operation.

For `world`, this maps to:

- semantic dialects, not one universal cognitive language
- representation roles/interfaces, not a single base representation type
- pass contracts with typed input/output, stage permissions, and verifiers
- generic tooling over common roles such as actor-relative view, decision
  signal, commitment candidate, and executable request

Relevant references:

- [MLIR Language Reference](https://mlir.llvm.org/docs/LangRef/)
- [MLIR Defining Dialects](https://mlir.llvm.org/docs/DefiningDialects/)
- [MLIR Traits](https://mlir.llvm.org/docs/Traits/)

### Pass Managers And Pipelines

MLIR and LLVM pass managers are useful because they separate individual
transforms from registered pipelines, analysis dependencies, preservation, and
invalidation. They also avoid pretending that any pass can safely run anywhere:
passes are anchored to appropriate IR units and must obey pass-manager rules.

For `world`, this maps to:

- named decision profiles, not arbitrary runtime graph mutation
- pass contracts validated before use
- declared read sets and produced representation roles
- analysis/query invalidation after accepted commits
- failure diagnostics when a profile violates authority, visibility, or type
  constraints

Relevant references:

- [MLIR Pass Infrastructure](https://mlir.llvm.org/docs/PassManagement/)
- [LLVM New Pass Manager](https://llvm.org/docs/NewPassManager.html)

### Query-Based Incremental Compilation

The Rust compiler query model and Salsa show a different lesson: many derived
views should be demand-driven, keyed, memoized, and invalidated by explicit
dependencies rather than rebuilt globally. Salsa also makes a strong assumption
that tracked computations are deterministic functions of their inputs; mutation
of inputs happens outside the derived computation loop.

For `world`, this maps to:

- `ObservedState(actor, focus)`, `EpistemicWorkingSet(actor, focus)`,
  `SocialContextView(actor, focus)`, `CapabilitySet(actor)`, and candidate
  decision structures as query-like derived artifacts
- accepted hard, social, epistemic, appraisal, and runtime-control commits as
  input changes that invalidate derived artifacts
- clear separation between deterministic derivation and nondeterministic/LLM
  proposal stages

Relevant references:

- [rustc demand-driven queries](https://rustc-dev-guide.rust-lang.org/query.html)
- [rustc incremental compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html)
- [Salsa overview](https://salsa-rs.github.io/salsa/overview.html)

### Abstract Interpretation

Abstract interpretation is the right analogy for multi-resolution simulation:
a coarser representation is not just a faster implementation. It is an
abstraction with a preservation contract. It may lose detail, but it must not
change authority, hidden-truth boundaries, or committed consequences without an
explicit materialization/refinement step.

For decision pipelines, this means a low-resolution cognitive pass can produce
coarser motivational or strategic signals, but those signals need a declared
relationship to more precise signals if they are used as substitutes in an
experiment.

Relevant reference:

- [Cousot and Cousot, Abstract Interpretation, POPL 1977](https://cs.nyu.edu/~pcousot/COUSOTpapers/POPL77.shtml)

### Datalog, Rete, And Differential Dataflow

Logic and incremental dataflow systems are useful for derived relation
maintenance, candidate generation, and provenance. Souffle is a practical
example of compiling Datalog-like rules for static analysis and other relation
problems. Differential dataflow shows how changing input collections can
incrementally update derived results.

For `world`, this maps to:

- rule-derived candidates and context views
- efficient incremental updates at larger population scale
- provenance over why a candidate became available
- no mutation authority for rule engines or dataflow engines

Relevant references:

- [Souffle Datalog documentation](https://souffle-lang.github.io/docs.html)
- [Differential Dataflow talk](https://www.microsoft.com/en-us/research/video/incremental-iterative-and-interactive-computation-using-differential-dataflow/)

## Core Thesis

The decision system should have a fixed authority kernel and a configurable
decision middle-end.

Fixed kernel:

- authoritative hard truth stores
- accepted social, chronology, epistemic, appraisal, and runtime-control commit
  gates
- actor-relative access rules
- typed effect execution and causal transaction commit
- provenance and event records
- profile validation and leakage checks

Configurable middle-end:

- which actor-relative context views are available
- which semantic or cognitive representations are computed
- whether speech is interpreted as raw text, typed speech acts, commitments,
  social claims, deception attempts, or several of these
- whether theory-of-mind-like representations exist
- whether motivational or strategic signals are rule-derived, LLM-generated,
  hybrid, oracle-provided, or absent
- how candidate intents, activities, and action requests are selected

This split is the main architectural requirement. Research flexibility belongs
in the middle-end. Truth authority does not.

## Required Vocabulary

### Decision Pipeline

A `DecisionPipeline` is the actor-facing path from accessible observations and
context to one of:

- `ActionRequest`
- `Intent`
- `Activity`
- non-hard proposal to a commit gate
- no-op, wait, or abstention

It is not the whole runtime pipeline. It sits between actor-relative derived
context and the existing execution/commit boundaries.

### Decision Profile

A `DecisionProfile` is an experiment or runtime condition that selects:

- pass sequence or small pass graph
- pass implementation mode
- allowed context inputs
- representation dialects
- LLM prompts or model adapters, if any
- oracle inputs, if any
- determinism and sampling policy
- trace and metric requirements

Examples:

```text
profile: direct_action_baseline
  context: ObservedEvent + ActionRepertoire
  passes:
    - ReactivePolicy(rule)
    - ActionRequestValidation

profile: no_tom_structured_context
  context: ObservedEvent + EpistemicWorkingSet + SocialContextView
  passes:
    - SpeechActGrounding(rule_or_llm)
    - MotivationalSignal(rule)
    - IntentSelection(llm)
    - ActivityBinding(rule)
    - ActionRequestValidation

profile: oracle_other_model
  context: ObservedEvent + EpistemicWorkingSet + SocialContextView
  passes:
    - OtherModelView(oracle)
    - StrategicAssessment(llm)
    - IntentSelection(llm)
    - ActivityBinding(rule)
    - ActionRequestValidation
```

A profile is not itself authority. It is a declared way to assemble permitted
passes under engine validation.

### Semantic Dialect

A `SemanticDialect` is a pack-defined or engine-defined family of
representations, declarations, passes, and verifiers for one semantic area.

Examples:

- physical/action dialect
- perception dialect
- epistemic dialect
- social claim dialect
- speech-act dialect
- commitment dialect
- appraisal-like motivation dialect
- strategic assessment dialect
- theory-of-mind dialect
- institution/law dialect

The important point is that `Appraisal` is not a required primitive of the
architecture. It can be a standard dialect supplied by the engine or standard
library. Another research condition can replace it with a different
motivational representation as long as the replacement declares compatible
roles, permissions, and verifiers.

### Representation Role

A `RepresentationRole` is a broad interface category used for pass matching and
tooling. It is not a concrete data type.

Likely roles:

| Role | Meaning |
| --- | --- |
| `ActorRelativeView` | What this actor can observe or query without hidden-truth leakage. |
| `EpistemicView` | Holder-relative belief, memory, uncertainty, or rumor surface. |
| `SocialContextView` | Actor-visible social/institutional context. |
| `SpeechSurface` | Raw or lightly structured utterance content. |
| `SpeechAct` | Typed communicative act such as promise, threat, accusation, lie, request, refusal, or claim. |
| `DecisionSignal` | Intermediate signal that can affect choice but does not commit intent. |
| `MotivationalSignal` | Pressure, preference, value conflict, utility component, emotion-like state, or other drive model. |
| `StrategicAssessment` | Prediction or evaluation of likely responses, incentives, sanctions, or bargaining leverage. |
| `OtherModelView` | Explicit model of another actor's belief, goal, intent, or likely policy. |
| `CommitmentCandidate` | Possible intent, promise, plan, agreement, threat, or social commitment. |
| `ActivityPlan` | Durable ongoing activity or process target. |
| `ExecutableRequest` | Request that can enter typed effect validation or a non-hard commit gate. |

Generic tooling can operate over roles. Specific passes still consume and
produce concrete representation kinds.

### Representation Kind

A `RepresentationKind` is a concrete typed artifact inside a dialect.

Examples:

```text
epistemic.BeliefWorkingSet
social.VisibleNormSet
speech.PromiseAct
speech.DeceptionAttempt
motivation.PressureVector
motivation.ValueConflict
strategy.BargainingLeverageEstimate
tom.OtherActorBeliefModel
intent.CandidateIntent
activity.ProcessTarget
action.ActionRequest
```

A representation kind declares:

- dialect namespace
- role or roles
- authority class
- visibility class
- stable schema
- provenance fields
- verifier
- whether it is durable, cacheable, actor-facing, or proposal-only

### Pass Contract

Every configurable pass needs an explicit contract.

```text
PassContract:
  id
  pass_class
  input_roles
  input_kinds
  output_roles
  output_kinds
  allowed_authority_reads
  allowed_actor_visibility
  forbidden_reads
  forbidden_writes
  implementation_modes
  determinism_policy
  llm_policy
  oracle_policy
  invalidation_dependencies
  verifier
  diagnostics
  provenance_schema
  metric_hooks
```

`input_roles` and `output_roles` allow profile validation to check broad
compatibility. `input_kinds` and `output_kinds` keep actual execution typed.

### Implementation Mode

A pass can have multiple implementations if they produce the same output role
and satisfy the same verifier.

```text
ImplementationMode:
  rule
  heuristic
  llm
  hybrid
  oracle
  replay
  disabled
```

This is central for ablation. For example, a `SpeechActGrounding` pass may have
`raw_text`, `rule`, `llm`, and `oracle` implementations. The output can still
be a typed `speech.SpeechActSet`, or the profile can deliberately disable the
typed output and force downstream passes to consume raw text.

## What Must Be Configurable

The architecture should support all of the following as first-class experiment
conditions.

### Different Pipeline Shapes

```text
ObservedEvent
  -> ReactivePolicy
  -> ActionRequest

ObservedEvent
  -> IntentSelection
  -> ActionRequest

ObservedEvent
  -> DecisionSignal
  -> CandidateIntent
  -> ActionRequest

ObservedEvent
  -> MotivationalSignal
  -> CandidateIntent
  -> ActivityPlan
  -> ActionRequest

ObservedEvent
  -> SpeechActGrounding
  -> OtherModelView
  -> StrategicAssessment
  -> CandidateIntent
  -> ActivityPlan
  -> ActionRequest
```

These are examples, not privileged ladders. The invariant is that every edge is
type-checked, every pass obeys authority, and executable effects still go
through the runtime commit boundary.

### Different Context Exposure

The same pass can be run under different input policies:

```text
IntentSelection:
  condition A inputs:
    ObservedEvent
    ActionRepertoire

  condition B inputs:
    ObservedEvent
    ActionRepertoire
    EpistemicWorkingSet

  condition C inputs:
    ObservedEvent
    ActionRepertoire
    EpistemicWorkingSet
    SocialContextView
    OtherModelView
```

The profile must declare this explicitly. The trace must record which context
was available. Hidden truth must not become visible through a convenience API.

### Different Semantic Granularity

Speech can be represented at different levels:

```text
raw:
  SpeechSurface(text)

typed:
  SpeechAct(kind=promise, content=..., addressee=...)

social:
  SpeechAct
  -> CommitmentCandidate
  -> AcceptedSocialUpdate proposal

strategic:
  SpeechAct
  -> OtherModelView update proposal
  -> StrategicAssessment
```

This makes it possible to test whether typed speech-act structure improves
promise tracking, deception, threat response, or negotiation without forcing
all scenarios to use that structure.

### Different Cognitive Structures

`Appraisal` should be one possible motivational dialect, not a mandatory
primitive.

Possible alternatives:

```text
motivation.PressureVector
utility.ExpectedValueEstimate
emotion.AffectState
normative.DutyConflict
strategy.ThreatOpportunityAssessment
rl.PolicyLogitExplanation
llm.FreeformRationaleSummary
```

These are not all equally desirable as authoritative state. Some may be
proposal-only or diagnostic-only. But the architecture should allow them to be
declared, verified, traced, and used by compatible downstream passes.

### Different Pass Implementations

The same conceptual pass can be evaluated under several implementations:

```text
OtherModelView:
  disabled
  heuristic
  llm
  oracle

IntentSelection:
  rule
  llm
  utility_argmax
  hybrid_llm_with_rule_filter

SpeechActGrounding:
  raw_passthrough
  llm_json
  rule_grammar
  oracle_annotation
```

This allows controlled comparisons:

- Does explicit theory-of-mind help?
- Does typed speech interpretation help?
- Does structured motivational state help beyond raw LLM context?
- Does oracle other-model information expose an upper bound that current LLMs
  fail to reach?
- Does hiding social context degrade negotiation or deception detection?

## What Must Not Be Configurable

The following are fixed architecture boundaries.

Hard truth mutation:

- only through typed effect execution and causal transaction commit

Accepted non-hard state:

- only through the relevant accepted update gate

Actor-relative access:

- a profile may remove context from a pass
- a profile may not grant omniscient hard truth to an actor-facing pass unless
  the experiment is explicitly an oracle/leakage condition and marked as such

Hidden truth:

- no pass implementation may smuggle hidden truth into natural-language prompt
  text, diagnostic fields, embeddings, cache keys, or provenance summaries

LLM output:

- may propose
- may choose among permitted candidates
- may generate typed records for verification
- may not directly mutate hard truth or accepted non-hard truth

Commit gates:

- cannot be skipped by profile configuration
- cannot be replaced by a general pass implementation

Trace:

- every research condition must record pass inputs by reference, output
  artifacts, implementation mode, verifier result, model/prompt identity when
  relevant, random seed/sampling metadata when relevant, and commit outcome

## Profile Validation

A decision profile is valid only if the engine can prove the following before
running it:

- every pass input role is available from an earlier pass or allowed context
  query
- concrete input/output kinds match the declared pass contract
- every pass reads only permitted authority classes
- every actor-facing output satisfies visibility constraints
- every durable write routes to an accepted commit gate
- nondeterministic and LLM stages declare replay/provenance policy
- disabled passes do not leave required downstream inputs unresolved
- oracle passes are marked so their results are not mixed with normal
  non-oracle metrics
- multi-resolution substitutions declare an abstraction/refinement relation
  when compared against higher-resolution conditions

Profile validation should fail early and explain the failed edge or pass.

## Pass Classes

The existing transition-compiler taxonomy remains useful, but decision
pipelines need more specific pass classes.

| Pass class | Purpose | Mutation authority |
| --- | --- | --- |
| `ContextDerivation` | Build actor-relative context views. | None. |
| `SemanticGrounding` | Convert events, state, or text into typed semantic candidates. | None. |
| `CognitiveSignal` | Produce motivational, strategic, affective, or other decision signals. | None, except proposal to accepted appraisal-like gate when configured. |
| `OtherModeling` | Produce explicit models of another actor's knowledge, goals, commitments, or policy. | None, except proposal to epistemic gate when configured. |
| `CandidateGeneration` | Produce candidate intents, commitments, plans, or actions. | None. |
| `Choice` | Select or rank a candidate. | None. |
| `ActivityBinding` | Bind selected intent to durable activity/process target. | May propose activity/process creation through proper gate. |
| `ExecutionRequest` | Lower selected candidate to action/process/non-hard update request. | Request only. |
| `Validation` | Check request against current authority boundary. | None, or runtime-owned staging only. |
| `Publication` | Publish accepted outcomes and invalidation. | Only through existing commit gates. |

These classes are not runtime inheritance roots. They are architecture labels
for validation, diagnostics, and documentation.

## LLM Passes

An LLM implementation is a pass implementation, not an authority owner.

An LLM pass contract must declare:

- prompt input representation kinds
- hidden-truth policy
- output schema
- parser and repair policy
- verifier
- retry and abstention policy
- sampling policy
- model identity and version capture
- whether outputs are used for gameplay, evaluation only, or diagnostics only
- whether the pass is allowed to see previous failed verifier diagnostics

Acceptable LLM uses:

- classify a speech act into a typed schema
- generate candidate intents from actor-relative context
- estimate another actor's likely response from visible evidence
- rank candidate actions under explicit utility/social criteria
- produce a rationale trace marked as non-authoritative

Unacceptable LLM uses:

- inspect omniscient hard truth in a normal actor-facing condition
- directly write hard truth
- directly commit social truth, memory, or appraisal state
- create untyped natural-language state that downstream systems treat as
  authoritative gameplay fact
- silently choose an action that bypasses candidate validation

## Example Research Profiles

### Direct Action Baseline

```text
ObservedEvent(actor)
ActionRepertoire(actor)
  -> ReactivePolicy(rule or llm)
  -> ActionRequest
  -> CausalRuntime
```

Purpose:

- test how much performance comes from direct LLM action selection
- provide a low-structure baseline

Expected weakness:

- poor long-horizon commitment tracking
- weak negotiation memory unless prompt context already contains it

### Intent Boundary Only

```text
ObservedEvent(actor)
ActionRepertoire(actor)
  -> CandidateIntent
  -> IntentSelection
  -> ActionRequest
  -> CausalRuntime
```

Purpose:

- isolate whether explicit commitment improves action consistency

Expected weakness:

- still lacks typed belief/social reasoning

### Typed Speech And Social Context

```text
ObservedEvent(actor)
SpeechSurface
EpistemicWorkingSet(actor)
SocialContextView(actor)
  -> SpeechActGrounding
  -> CommitmentCandidate / SocialClaim proposal
  -> IntentSelection
  -> ActivityBinding
  -> ActionRequest or AcceptedSocialUpdate proposal
```

Purpose:

- test whether typed speech acts improve promise, threat, lie, accusation,
  bargain, and norm-violation tracking

Expected weakness:

- speech-act typing may be expensive and brittle
- overly rigid typing may miss ambiguity that matters socially

### Explicit Theory Of Mind

```text
ObservedEvent(actor)
EpistemicWorkingSet(actor)
SocialContextView(actor)
SpeechActSet
  -> OtherModelView
  -> StrategicAssessment
  -> CandidateIntent
  -> IntentSelection
  -> ActivityBinding
  -> ActionRequest
```

Purpose:

- test whether explicit other-model representations improve bargaining,
  deception, threat response, coalition behavior, and sanction avoidance

Expected weakness:

- model explosion if nested beliefs are unconstrained
- false precision if `OtherModelView` looks structured but is only LLM guesswork

### Oracle Upper Bound

```text
ObservedEvent(actor)
EpistemicWorkingSet(actor)
SocialContextView(actor)
OracleOtherModelView
  -> StrategicAssessment
  -> IntentSelection
  -> ActionRequest
```

Purpose:

- estimate the maximum value of better belief/opponent modeling
- separate architecture value from current LLM capability

Constraint:

- oracle runs must be labeled and reported separately from normal actor-facing
  conditions

## Research Metrics Enabled By This Architecture

The architecture should make these metrics practical:

- action validity
- goal achievement
- utility or payoff
- regret against a known game-theoretic baseline where available
- promise consistency
- threat credibility
- deception success and deception detection
- belief calibration
- false-belief handling
- commitment creation, fulfillment, violation, and repair
- sanction anticipation
- coalition stability
- social claim acceptance/rejection
- trace faithfulness: whether the selected action is supported by recorded
  intermediate representations
- leakage detection: whether performance depends on forbidden context
- ablation delta per representation role or pass implementation

The point is not that every scenario uses every metric. The point is that
typed intermediate artifacts make these metrics inspectable without relying
only on transcript grading.

## Architectural Shape

The recommended shape is:

```text
AuthorityState / EventHistory / AcceptedRecords
  -> actor-relative query layer
  -> decision profile validation
  -> context derivation passes
  -> semantic and cognitive dialect passes
  -> candidate generation and choice passes
  -> executable request or non-hard proposal
  -> existing validation and commit gates
  -> accepted records, invalidation, trace
```

The configurable layer should be represented by data declarations and typed
contracts before it becomes an implementation framework. A premature generic
plugin system would make the research story weaker, not stronger, because it
would be hard to prove that two ablation conditions differ only in the intended
place.

## Minimal Implementation Direction

The first implementation should not attempt to implement a full MLIR-like
system. A practical path is:

1. Define `DecisionProfile` as a small static declaration used by tests and
   research scenarios.
2. Define `RepresentationRole` as architecture-level metadata attached to
   concrete Rust/domain types.
3. Define `PassContract` for a small number of concrete passes.
4. Validate profile shape before execution.
5. Record a `DecisionTrace` with pass input references, output references,
   implementation mode, verifier result, and selected request.
6. Add ablation profiles for direct action, intent-only, structured speech,
   and explicit other-modeling.
7. Only after those are useful, consider a declarative profile file format.

This keeps the architecture research-ready without turning the early engine
into a compiler framework project.

## Design Risks

### Risk: Too General

If every representation and edge is fully dynamic, the system becomes hard to
verify and impossible to explain. The mitigation is role-based compatibility
plus concrete representation kinds and explicit pass contracts.

### Risk: Appraisal Becomes Hardcoded

If `Pressure` and `Appraisal` become the only blessed middle representation,
the research agenda narrows too early. The mitigation is to treat appraisal as
a standard dialect, not a kernel primitive.

### Risk: LLM Text Becomes Hidden Authority

If downstream passes consume untyped LLM prose as state, authority boundaries
collapse. The mitigation is typed outputs, verifier gates, and non-authoritative
rationale traces.

### Risk: Ablations Are Not Comparable

If profiles differ in many undeclared ways, experimental results become weak.
The mitigation is explicit context exposure, implementation mode, prompt/model
identity, oracle labels, and trace requirements.

### Risk: Theory Of Mind Explodes

Nested belief modeling can grow without bound. The mitigation is bounded
`OtherModelView` schemas: limited depth, explicit uncertainty, evidence links,
and scenario-specific roles.

### Risk: Compiler Analogy Overreaches

Compiler references are useful for structure, not for identity. Actors are not
programs, social meaning is not SSA, and LLM passes are not pure compiler
passes. The mitigation is to keep the analogy at the contract/tooling level:
dialects, verifiers, pass contracts, analyses, invalidation, and traces.

## Success Criteria

This architecture is working if the project can run the same scenario under
profiles like:

```text
direct action
intent only
intent + typed speech
intent + social context
intent + explicit other-model
intent + explicit other-model + structured motivation
oracle other-model upper bound
```

and produce:

- comparable outcome metrics
- comparable trace artifacts
- clear pass-level ablation deltas
- no hidden-truth leakage in normal conditions
- reproducible replay for deterministic passes
- explicit model/prompt/sampling metadata for LLM passes
- commit records that still flow through the same runtime authority boundaries

If this is achieved, the project has a stronger research position than "an RPG
engine with agents". It becomes a controlled substrate for testing which
structured cognitive/social representations actually help LLM agents act in
complex mixed-motive worlds.

