# Phase 10 Social-Cognitive Representation Slice Research

## Status

Research handoff for Phase 10 design and implementation planning.

This document consolidates:

- `.codex/research/phase-10-01-local-constraints.md`
- `.codex/research/phase-10-02-broad-external-survey.md`
- `.codex/research/phase-10-03-representation-family-synthesis.md`
- `.codex/research/phase-10-04-family-specific-deep-research.md`
- `.codex/research/phase-10-05-artifact-shape-and-ablation-plan.md`

## Source Precedence

When local documents conflict, use the newest active documents in this order:

1. `docs/architecture/implementation-plan.md`
2. `docs/architecture/configurable-decision-pipeline.md`
3. current `docs/design/` documents
4. current `docs/research/` documents
5. `.codex/research/` and `.codex/plans/` phase-local notes

Archived architecture documents are historical context only. In particular,
older notes that describe Phase 10 as an engine facade are superseded by the
active implementation plan, where Phase 10 is the social-cognitive
representation slice and Phase 11 owns the engine facade.

## Research Conclusion

Phase 10 should implement a narrow concrete representation slice inside the
existing `world-decision` substrate. It should not change crate dependency
direction, introduce runtime mutation authority, add persistence, or choose an
LLM/provider architecture.

The first slice should make social-strategic behavior comparable through typed
decision artifacts and traces. The target is not "smarter NPCs" in general. The
target is to compare profiles such as:

- direct baseline vs typed speech;
- typed speech vs speech plus social commitment candidate;
- no other-model vs bounded other-model;
- bounded other-model vs oracle other-model;
- strategy-only vs strategy plus optional motivation.

## Recommended First-Class Families

Implement five first-class representation families.

### 1. Speech Interpretation

Roles:

- `SpeechSurface`
- `SpeechAct`

Purpose:

- preserve actor-visible utterance material;
- interpret it into typed communicative acts with ambiguity, confidence, and
  evidence.

Initial communicative functions:

```text
inform
query
request
propose
promise
accept
reject
refuse
confirm
deny
warn
threaten
```

Stable contract:

- speech surface and speech-act candidate remain separate;
- speech acts do not directly mutate epistemic or social state.

Replaceable implementation:

- parser, classifier, semantic extractor, LLM prompt/model, confidence scoring.

### 2. Social Commitment Candidate

Role:

- `CommitmentCandidate`

Purpose:

- represent possible public social commitments such as promises, agreements,
  debts, duties, oaths, threats, and offers;
- keep these separate from private intent and accepted social state.

Initial lifecycle labels:

```text
proposed
created
detached
discharged
canceled
violated
contested
```

Stable contract:

- commitment candidates carry parties, content, lifecycle candidate, trigger or
  horizon, witness scope, evidence, and sanction/repair expectations;
- they are proposal-shaped or derived artifacts, not accepted authority.

Replaceable implementation:

- extraction rules, lifecycle inference, sanction estimation, scoring.

### 3. Bounded Other-Model View

Role:

- `OtherModelView`

Purpose:

- represent what the acting actor models about one target actor or institution;
- make uncertainty, evidence, and oracle use explicit.

Stable contract:

- owner actor, target, depth limit, modeled claims, likely responses,
  uncertainty, evidence, and oracle label;
- first normal slice uses depth limit one.

Replaceable implementation:

- heuristic target model, response predictor, LLM/hybrid model, oracle fixture,
  replay.

### 4. Strategic Assessment

Role:

- `StrategicAssessment`

Purpose:

- expose predicted responses, leverage, sanctions, disclosure risk, trust or
  reputation risk, and expected commitment effects as a typed artifact.

Stable contract:

- strategy remains separate from speech, other-model, commitment, and
  motivation;
- strategic assessment does not become accepted future truth.

Replaceable implementation:

- risk scoring, bargaining heuristic, policy estimator, LLM/hybrid executor.

### 5. Motivational Signal

Role:

- `MotivationalSignal`

Purpose:

- optionally expose actor pressure such as urgency, threat, opportunity, norm
  conflict, relationship salience, coping feasibility, and expected sanction.

Stable contract:

- motivation is a decision artifact, not accepted actor mood or emotion state;
- the family stays optional because appraisal is one dialect, not the kernel.

Replaceable implementation:

- appraisal dialect, utility dialect, normative priority model, emotion labels,
  aggregation.

## Connector Artifact

`IntentCandidate` should remain a connector artifact rather than a deep Phase 10
family.

Use it where examples need:

```text
speech / commitment / other-model / strategy / motivation
  -> intent candidate
  -> choice
```

This preserves the local distinction between:

- public social commitment among parties;
- private or selected intent as the actor's commitment to a purpose and
  approach.

## Proposed Representation Kinds

The initial kind set can be:

| Kind | Role | Authority stance |
| --- | --- | --- |
| `speech_surface` | `SpeechSurface` | derived |
| `speech_act_candidate` | `SpeechAct` | derived |
| `social_commitment_candidate` | `CommitmentCandidate` | proposal-shaped or derived |
| `bounded_other_model_view` | `OtherModelView` | derived |
| `oracle_other_model_view` | `OtherModelView` | oracle |
| `strategic_assessment` | `StrategicAssessment` | derived |
| `motivational_signal` | `MotivationalSignal` | derived |
| `intent_candidate` | `IntentCandidate` | derived |
| `social_decision_choice` | `Choice` | derived |
| `social_cognitive_diagnostic` | `Diagnostic` | diagnostic |

No new `RepresentationRole` appears necessary for the initial slice.

If the existing authority taxonomy does not clearly express a non-hard social
proposal for `social_commitment_candidate`, keep the first kind derived and
defer accepted social-state routing rather than expanding architecture during
Phase 10.

## Recommended Pass Chain

The initial profile family can use this ordering:

```text
ActorRelativeView / SocialContextView
  -> SpeechSurface
  -> SpeechAct
  -> CommitmentCandidate

ActorRelativeView / SocialContextView / SpeechAct / CommitmentCandidate
  -> OtherModelView

SpeechAct / CommitmentCandidate / OtherModelView / SocialContextView
  -> StrategicAssessment

ActorRelativeView / CommitmentCandidate / OtherModelView / StrategicAssessment
  -> MotivationalSignal

CommitmentCandidate / OtherModelView / StrategicAssessment / MotivationalSignal
  -> IntentCandidate
  -> Choice
```

This chain is a recommended implementation starting point, not a mandatory
global architecture. Profiles can omit passes for ablation.

## Recommended Ablation Profiles

### Profile A: Direct Baseline

```text
ActorRelativeView
  -> IntentCandidate
  -> Choice
```

Use this to show behavior without typed speech, commitment, other-model, or
strategy artifacts.

### Profile B: Typed Speech

```text
ActorRelativeView
  -> SpeechSurface
  -> SpeechAct
  -> IntentCandidate
  -> Choice
```

Compare against Profile A to test raw context versus typed speech
interpretation.

### Profile C: Speech Plus Commitment

```text
ActorRelativeView
  -> SpeechSurface
  -> SpeechAct
  -> CommitmentCandidate
  -> IntentCandidate
  -> Choice
```

Compare against Profile B to test whether social commitment representation adds
value beyond typed speech.

### Profile D: Bounded Other-Model Plus Strategy

```text
ActorRelativeView
  -> SpeechSurface
  -> SpeechAct
  -> CommitmentCandidate
  -> BoundedOtherModelView
  -> StrategicAssessment
  -> IntentCandidate
  -> Choice
```

Compare against Profile C to test bounded other-modeling and explicit strategic
assessment.

### Profile E: Oracle Other-Model Upper Bound

```text
ActorRelativeView
  -> SpeechSurface
  -> SpeechAct
  -> CommitmentCandidate
  -> OracleOtherModelView
  -> StrategicAssessment
  -> IntentCandidate
  -> Choice
```

Compare against Profile D to measure the ceiling from oracle target modeling.

### Profile F: Strategy Plus Motivation

```text
ActorRelativeView
  -> SpeechSurface
  -> SpeechAct
  -> CommitmentCandidate
  -> BoundedOtherModelView
  -> StrategicAssessment
  -> MotivationalSignal
  -> IntentCandidate
  -> Choice
```

Compare against Profile D to test whether explicit pressure signals add value
beyond strategic assessment.

## Minimum Implementation Slice

The smallest useful implementation target is:

1. concrete representation kind definitions for the five selected families;
2. fixture-backed or rule-backed executors for Profiles A, C, and D;
3. oracle other-model executor for Profile E if oracle labeling is already easy
   in the existing profile metadata;
4. trace assertions that distinguish omitted, produced, oracle, and consumed
   artifacts.

Profile F can be implemented after the first three comparisons if motivation
would otherwise make the slice too large.

## Test And Trace Expectations

Targeted tests should prove:

- representation kinds declare expected roles and no hard authority;
- pass contracts resolve required and optional-all inputs;
- speech act artifacts link back to speech surfaces;
- commitment candidates link to speech acts or social context;
- other-model views record owner, target, depth, uncertainty, and evidence;
- oracle other-model views require oracle-aware labeling;
- strategic assessments consume other-model or commitment evidence;
- motivational signals are optional;
- choice artifacts can be terminal outputs without becoming runtime execution
  requests;
- profile ablations produce visible trace differences.

## External Research Lenses Used

The external research supports the selected families but does not need to be
imported wholesale:

- ISO 24617-2: dialogue acts are multidimensional, so speech-act candidates
  should allow ambiguity and qualifiers.
  <https://www.iso.org/standard/76443.html>
- FIPA Communicative Act Library: useful source for a small communicative
  function vocabulary.
  <https://jmvidal.cse.sc.edu/library/XC00037H.pdf>
- Singh social semantics: communication is more traceable through public social
  commitments than private mental states alone.
  <https://www.csc2.ncsu.edu/faculty/mpsingh/papers/mas/ijcai-99-acl.pdf>
- Cohen and Levesque / Bratman: intent is commitment-like practical reasoning,
  but this should remain distinct from public social commitment.
  <https://ai.stanford.edu/~epacuit/classes/lori-spr09/cohenlevesque-intention-aij90.pdf>
  <https://web.stanford.edu/group/cslipublications/cslipublications/site/1575861925.shtml>
- Commitment-protocol work: lifecycle and alignment matter, but full protocol
  synthesis should be deferred.
  <https://www.ijcai.org/proceedings/2017/37>
  <https://www.csc2.ncsu.edu/faculty/mpsingh/papers/mas/aamas-03-ctl.pdf>
- Interactive POMDP work: other-modeling is useful but must be bounded.
  <https://www.cs.cmu.edu/afs/cs.cmu.edu/project/jair/pub/volume24/gmytrasiewicz05a.pdf>
- CICERO: language and strategy should be separable trace artifacts.
  <https://www.science.org/doi/10.1126/science.ade9097>
  <https://ai.meta.com/research/cicero/>
- SOTOPIA, Concordia, MACHIAVELLI, AvalonBench, MafiaBench, M3-BENCH, and
  Cattle Trade: current benchmarks highlight social intelligence, hidden
  information, deception, bargaining, and mixed motives, but `world` should
  emphasize typed causal traces.
  <https://arxiv.org/abs/2310.11667>
  <https://arxiv.org/abs/2312.03664>
  <https://arxiv.org/abs/2304.03279>
  <https://openreview.net/pdf?id=ltUrSryS0K>
  <https://www.mafiabench.org/>
  <https://arxiv.org/abs/2601.08462>
  <https://arxiv.org/abs/2605.14537>
- OCC and EMA appraisal references: appraisal is useful as a structured signal
  lens, not a required engine kernel.
  <https://users.cs.northwestern.edu/~ortony/Andrew_Ortony_files/2013%20-%20OCC%20and%20constructionism.pdf>
  <https://ict.usc.edu/pubs/EMA-%20A%20process%20model%20of%20appraisal%20dynamics.pdf>

## Alternatives Rejected

Do not implement these in Phase 10:

- full BDI interpreter;
- full dialogue-act standard;
- full commitment protocol engine;
- accepted social, epistemic, appraisal, or memory stores;
- recursive theory-of-mind trees;
- generic LLM provider, prompt runtime, or persistence layer;
- engine facade or runtime request orchestration;
- broad plugin framework.

These may become useful later, but they are larger than the current slice and
would blur the stable boundary between decision artifacts and authority-bearing
simulation state.

## Main Risks

### Authority leakage

Commitment, speech, and other-model artifacts may look like accepted truth. The
implementation must keep them derived, proposal-shaped, oracle-labeled, or
diagnostic according to their kind metadata.

### Schema overreach

The payloads should be just rich enough to support trace comparison. Full
standards, legal commitment systems, or emotion theories should be deferred.

### False precision

Structured artifacts can imply more certainty than the executor has. Confidence,
ambiguity, uncertainty, and evidence refs are required to make this visible.

### Hidden LLM state

If an LLM executor is added later, its natural-language rationale must not
replace typed payload fields. The stable artifact stays the contract.

### Profile sprawl

The first implementation should land a few profile comparisons, not a full
evaluation suite. Broader benchmark work belongs after the representation slice
is proven.

## Design Plan Input

The next design step should decide:

1. where concrete payload types live inside `world-decision`;
2. whether the first payloads are typed Rust structs, test fixture data, or a
   small typed value layer;
3. exact representation kind identifiers and version anchors;
4. fixture scenario for the first ablation tests;
5. whether `social_commitment_candidate` can safely use an existing non-hard
   authority proposal class or should remain derived for the first version.

The implementation should start only after those decisions are explicit.
