# Phase 10 Local Constraints

## Purpose

This note extracts the local constraints that Phase 10 research must respect
before external literature is mapped into the implementation plan.

Phase 10 is the first concrete social-cognitive representation slice on top of
the existing `world-decision` substrate. It is not a new architecture phase and
not an engine facade phase.

## Source Precedence Used

When local documents conflict, this note follows:

1. `docs/architecture/implementation-plan.md`
2. `docs/architecture/configurable-decision-pipeline.md`
3. current `docs/design/` documents
4. current `docs/research/` documents
5. `.codex/research/` and `.codex/plans/` phase-local notes

Archived architecture documents and older phase-local notes are historical
context only. For example, older notes that label Phase 10 as an engine facade
are superseded by the active implementation plan, where Phase 10 is
`Social-Cognitive Representation Slice` and Phase 11 owns the engine facade.

## Active Phase 10 Boundary

The active implementation plan defines Phase 10 as adding the first
research-relevant representation families on top of the decision substrate.

The required work is:

- typed speech surfaces and speech-act candidates;
- commitment candidates and commitment lifecycle inputs;
- bounded other-model views;
- strategic or motivation signals;
- optional appraisal-like variables as one standard dialect;
- paired ablation examples.

The exit condition is not "better NPC behavior" in general. The exit condition
is the ability to express at least one meaningful social-strategic contrast,
such as direct action versus typed speech or no other-model versus bounded
other-model, with comparable typed traces.

## Non-Negotiable Boundaries

### Decision artifacts are not authority

Decision artifacts can be selected choices, candidate intents, speech acts,
commitment candidates, other-model views, strategic assessments, executable
request candidates, non-hard proposals, or diagnostics. They do not commit
hard truth or accepted non-hard truth by themselves.

Any hard mutation remains outside Phase 10 and must go through the causal
runtime path. Any accepted soft, actor, epistemic, appraisal, or social state
must later pass through the appropriate commit gate.

### Decision code has no mutation authority

`world-decision` currently depends on `world-core`, `world-defs`, and
`world-context`. It must not gain dependencies on `world-model`,
`world-runtime`, `world-standard-runtime`, or `world-engine`.

Executor implementations receive a restricted `DecisionContextView`, declared
resolved inputs, and input-scoped artifact lookup. They do not receive raw
world model access, runtime staging authority, or a full artifact-store
capability.

### Actor-relative access remains explicit

A profile declares which context projection families are available. A pass
declares which of those context families it may inspect. The actual execution
context exposes only the intersection.

Phase 10 representation payloads must therefore preserve actor-relative
uncertainty. They must not collapse observed, believed, rumored, inferred, and
oracle facts into one omniscient field.

### Appraisal is optional

The configurable decision pipeline document is explicit that appraisal-like
motivation is one possible dialect, not a required primitive. Phase 10 may add
appraisal-like variables or pressure signals, but it must not make one
appraisal theory the root of the decision substrate.

A useful Phase 10 design should allow a profile to omit the motivation dialect,
replace it with a strategic assessment dialect, or use a diagnostic-only
signal for research comparison.

### Intent remains the commitment boundary

Design documents treat `Intent` as the selected or suggested commitment to a
purpose and approach. `Pressure`, `GoalPressure`, social state, memory, and
speech do not directly become actions.

Phase 10 can add `CandidateIntent`, `CommitmentCandidate`, `IntentScore`, or
choice-support artifacts, but selected intent, activity binding, runtime
request submission, and scheduler coordination should remain outside the
initial representation slice unless they are just typed decision artifacts.

### Speech and commitments are typed but not automatically accepted

The social model distinguishes durable social commitments such as promise,
debt, duty, oath, and obligation from the appraisal that turns them into
current pressure. Phase 10 can model speech acts and commitment candidates, but
accepting a promise, debt, obligation, reputation change, or social claim as
game state is a later gated update.

### LLMs and oracles are implementation modes

LLM, hybrid, heuristic, rule, replay, and oracle behavior belongs behind pass
implementation modes and execution metadata. Phase 10 should not introduce an
LLM provider, oracle provider, prompt runtime, persistence backend, or replay
store.

The stable contract is the typed artifact and trace. The replaceable part is
the executor algorithm that produces it.

## Current Substrate Available

`world-decision` already provides:

- `RepresentationRole` entries for `SpeechSurface`, `SpeechAct`,
  `MotivationalSignal`, `StrategicAssessment`, `OtherModelView`,
  `CommitmentCandidate`, `IntentCandidate`, `Choice`, `ActivityPlan`,
  `ExecutableRequest`, `NonHardUpdateProposal`, and `Diagnostic`;
- `RepresentationKindDef` with role, visibility, persistence, authority, and
  version metadata;
- `DecisionPassContract` with pass class, inputs, outputs, allowed context,
  write policy, implementation modes, determinism, and version metadata;
- `DecisionProfile` with static ordered steps, explicit terminal output, and
  oracle policy;
- `DecisionRunner` with trace-backed execution, input resolution, output
  validation, metadata validation, abstention, and failed reports;
- `DecisionTrace` with context input refs, artifact refs, step status,
  verifier result, execution metadata, and artifact provenance.

This means Phase 10 should not begin by creating a new generic framework. It
should define a small set of concrete representation kinds and trusted test
executors that prove the existing substrate can express the needed contrasts.

## Stable Contracts To Preserve

Phase 10 should keep these stable:

- representation role semantics;
- concrete representation kind identity and declared authority;
- pass input/output contracts;
- actor-relative context and provenance requirements;
- trace semantics for comparing profiles;
- oracle labeling and metadata separation;
- no direct mutation authority from decision artifacts.

## Replaceable Implementation Areas

Phase 10 should keep these replaceable:

- executor implementation strategy;
- scoring and ranking algorithms;
- candidate-generation heuristics;
- speech-act grounding implementation;
- other-model construction algorithm;
- motivation or strategic signal computation;
- caching or indexing strategy inside a future executor;
- LLM, hybrid, or oracle implementation behind the same pass contract.

## Deferred Work

Phase 10 should not implement:

- engine session facade;
- runtime request submission orchestration;
- scenario/evaluation substrate;
- source authoring syntax;
- parser or diagnostic renderer;
- persistence or replay backend;
- LLM provider integration;
- accepted social, epistemic, or appraisal stores;
- broad plugin framework;
- graph, Datalog, ECS, Wasm, or async runtime integration.

## Local Research Questions For Later Steps

The external survey and synthesis should answer:

- Which speech-act distinctions are useful for comparable traces without
  forcing a large dialogue framework?
- Which commitment lifecycle states are needed as decision artifacts before
  accepted social state exists?
- What is the minimum bounded other-model schema that preserves uncertainty
  and evidence links without causing nested-belief explosion?
- What motivation or strategic signal can help profile comparison while
  keeping appraisal optional?
- Which ablation pairs are small enough to implement in Phase 10 tests?

## Initial Constraint-Based Recommendation

The first viable Phase 10 slice should prefer a narrow profile set:

```text
direct action baseline
typed speech enabled
typed speech + commitment candidate
bounded other-model enabled
bounded other-model + strategic assessment
oracle other-model upper bound
```

The slice should prove comparability through typed traces before adding richer
payloads or optimizing executors.
