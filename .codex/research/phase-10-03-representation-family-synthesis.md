# Phase 10 Representation Family Synthesis

## Purpose

This note maps the local Phase 10 constraints and broad external survey into a
small set of representation families that are worth researching deeply before
implementation.

Inputs:

- `.codex/research/phase-10-01-local-constraints.md`
- `.codex/research/phase-10-02-broad-external-survey.md`
- active architecture, design, and research documents under `docs/`
- current `world-decision` representation and pass substrate

The goal is to select concrete families for Phase 10, not to choose a complete
cognitive architecture.

## Selection Criteria

A first-class Phase 10 representation family should satisfy most of these:

- it maps to an existing `RepresentationRole` or needs only a narrow role
  addition;
- it can be produced by a small deterministic, heuristic, oracle, replay, or
  later LLM-backed executor behind the same pass contract;
- it preserves actor-relative uncertainty and provenance;
- it does not imply accepted hard, social, epistemic, or appraisal authority;
- it supports at least one comparable trace ablation;
- it can be represented as a bounded typed payload rather than an open-ended
  reasoning transcript;
- it creates useful evidence for later social-strategic evaluation.

Families that are theoretically important but fail these criteria should remain
deferred.

## Synthesis Result

Phase 10 should treat these five families as first-class candidates:

1. speech interpretation;
2. social commitment candidate;
3. bounded other-model view;
4. strategic assessment;
5. motivational signal.

`IntentCandidate` should remain a connector artifact for choice and activity
binding, but it does not need deep new representation work in Phase 10 unless a
specific ablation requires it.

No new `RepresentationRole` is required for the initial slice. Existing roles
already cover the needed compatibility surface:

- `SpeechSurface`
- `SpeechAct`
- `CommitmentCandidate`
- `OtherModelView`
- `StrategicAssessment`
- `MotivationalSignal`
- `IntentCandidate`
- `Choice`
- `Diagnostic`

## Candidate Family 1: Speech Interpretation

### Local role mapping

Use two related representation roles:

- `SpeechSurface`
- `SpeechAct`

`SpeechSurface` should preserve raw or lightly structured utterance material.
`SpeechAct` should hold a typed interpretation of that surface.

### Why first-class

Phase 10's active boundary explicitly names typed speech surfaces and speech-act
candidates. External speech-act and dialogue-act work supports the same
direction, but it also warns against a single flat label.

The useful research value is not "NPCs can talk". The useful value is that a
trace can show whether a profile:

- ignored speech;
- treated speech as raw text only;
- interpreted speech as a request, promise, threat, offer, refusal, or claim;
- routed the speech act into a commitment, other-model, strategic, or diagnostic
  artifact.

### Minimal scope

Start with a narrow communicative function vocabulary:

- inform;
- request;
- propose;
- promise;
- accept;
- reject;
- refuse;
- query;
- confirm;
- deny;
- warn;
- threaten.

The payload should preserve ambiguity, confidence, evidence, and target object
links. It should not import a full ISO dialogue-act standard.

### Deferred

Defer full dialogue-state tracking, turn-taking protocol, repair sequences,
sarcasm, indirect speech, rich natural-language parsing, and accepted social or
epistemic updates.

## Candidate Family 2: Social Commitment Candidate

### Local role mapping

Use:

- `CommitmentCandidate`

The family should represent possible social commitments, not accepted social
truth.

### Why first-class

Local social design documents treat promises, debts, duties, oaths, obligations,
and related relations as durable social structures. External commitment-based
multiagent work supports making these public and traceable instead of reducing
them to private mental states.

Phase 10 needs this family because typed speech without a commitment artifact
cannot express the central contrast between saying something and becoming
socially bound by it.

### Minimal scope

Represent a possible social commitment with parties, content, trigger, horizon,
witness scope, lifecycle candidate, evidence, and possible sanction or repair
expectation.

The initial lifecycle vocabulary can be small:

- proposed;
- created;
- detached;
- discharged;
- canceled;
- violated;
- contested.

These are candidate labels only. Accepted commitment state remains outside Phase
10.

### Deferred

Defer protocol synthesis, legal/normative authority systems, reputation updates,
accepted debt or obligation stores, and cross-agent consensus over commitment
state.

## Candidate Family 3: Bounded Other-Model View

### Local role mapping

Use:

- `OtherModelView`

The family should be explicitly bounded and evidence-linked.

### Why first-class

The active plan names bounded other-model views, and local research documents
make explicit theory-of-mind an evaluation axis. External multiagent planning
work shows the value of other-modeling but also shows why unrestricted nested
belief modeling is too expensive and too easy to overfit.

Phase 10 should make other-modeling comparable through traces:

- no other-model;
- heuristic other-model;
- oracle other-model;
- other-model plus strategic assessment.

### Minimal scope

Represent what the acting actor models about one target actor or institution:

- modeled beliefs or knowledge claims;
- modeled goals, pressures, or preferences;
- modeled commitments;
- likely response or policy;
- uncertainty;
- evidence links;
- depth limit;
- oracle label when applicable.

Depth should be limited to one explicit target model in the first slice. Nested
models should be summarized as claims, not expanded as trees.

### Deferred

Defer recursive theory-of-mind trees, POMDP solvers, belief propagation across
the world model, and accepted epistemic state updates.

## Candidate Family 4: Strategic Assessment

### Local role mapping

Use:

- `StrategicAssessment`

### Why first-class

The external survey suggests that social dialogue works better when language is
not treated as the whole strategic agent. CICERO-like separation between
language and strategy maps directly to the local substrate: speech artifacts can
feed a separate strategic assessment artifact.

This family also makes Phase 10 ablations interpretable. A profile can possess
the same speech and other-model artifacts but differ in whether it estimates
likely responses, sanctions, leverage, and information disclosure risk.

### Minimal scope

Represent decision-relevant strategic consequences:

- likely responses;
- leverage;
- sanctions;
- bargaining position;
- information disclosure risk;
- trust or reputation risk;
- expected commitment effects;
- downside and upside summaries.

The payload should not contain a free-form plan as its only data. It should be a
bounded assessment that a later choice pass can consume.

### Deferred

Defer a general game-theory solver, negotiation protocol engine, equilibrium
analysis, long-horizon planner, and domain-specific game policies.

## Candidate Family 5: Motivational Signal

### Local role mapping

Use:

- `MotivationalSignal`

### Why first-class, but optional

Local architecture says appraisal-like motivation is one dialect, not the root
of the pipeline. The external survey supports this: appraisal variables are
useful when they expose structured reasons for action-readiness, but Phase 10
should not import an emotion theory as the engine kernel.

This family is still useful as a first-class candidate if it remains optional
and replaceable. It can expose why a choice is pressured without forcing every
profile through the same appraisal model.

### Minimal scope

Represent bounded signals such as:

- urgency;
- threat;
- opportunity;
- norm conflict;
- relationship salience;
- coping feasibility;
- expected sanction;
- value conflict;
- confidence.

The signal should explain pressure inputs to candidate generation or choice. It
should not become accepted mood, emotion, personality, or memory state.

### Deferred

Defer full OCC, EMA, personality, mood, affect dynamics, and long-term emotion
state. If a richer appraisal model is useful later, it should be an executor and
schema version behind the same or successor role.

## Connector Artifact: IntentCandidate

`IntentCandidate` remains important, but it should not become a separate deep
research family in Phase 10.

Use it only where needed to connect signals and commitments to choice:

```text
speech / commitment / other-model / strategy / motivation
  -> candidate intent
  -> choice
```

This preserves the local design boundary: intent is the actor's possible or
selected commitment to a purpose and approach, while social commitment is a
public relation among parties.

Deep intent-template and activity-binding work can stay with existing design
documents and later implementation phases.

## Families Rejected Or Deferred

### Full BDI architecture

Belief, desire, and intention concepts are useful as vocabulary, but Phase 10
should not implement a BDI interpreter. It would overtake the existing
pass/profile substrate and blur the distinction between representation
artifacts and runtime authority.

### Full dialogue-act standard

ISO-style multidimensional annotation is useful as a design warning, but the
full standard is too broad for the first slice. A small speech-act candidate
schema with ambiguity and provenance is enough.

### Full commitment protocol engine

Lifecycle fields are necessary, but accepted social commitments and protocol
alignment are later authority and evaluation work.

### Appraisal kernel

Appraisal should be one optional motivational-signal dialect. It should not
become the required substrate for all social-cognitive decisions.

### Generic LLM prompt/runtime layer

LLMs can later produce any of these artifacts behind `ImplementationMode::Llm`
or `ImplementationMode::Hybrid`. Phase 10 should define the typed artifacts and
trace semantics first.

### Recursive theory-of-mind model

Nested beliefs are valuable but expensive and risky. The first slice should use
one bounded target model with evidence links and uncertainty.

## Role And Kind Implications

The initial implementation should define concrete representation kinds rather
than new broad roles. Candidate kind names can follow domain meaning:

- `speech_surface`
- `speech_act_candidate`
- `social_commitment_candidate`
- `bounded_other_model_view`
- `oracle_other_model_view`
- `strategic_assessment`
- `motivational_signal`
- `intent_candidate` when needed by an example profile

Authority should default to derived or proposal-shaped metadata:

- speech surfaces: derived trace or actor-visible input;
- speech acts: derived trace;
- commitment candidates: proposal-only, not accepted authority;
- other-model views: derived trace or oracle-labeled;
- strategic assessments: derived trace;
- motivational signals: derived trace;
- intent candidates: derived or proposal-like, depending on the eventual
  choice/intent boundary.

No Phase 10 representation kind should declare hard authority.

## Pass Implications

The first slice can be expressed with existing pass classes:

- `ContextDerivation` for extracting actor-relative speech or social context;
- `SemanticGrounding` for speech-act interpretation;
- `OtherModeling` for bounded target models;
- `CognitiveSignal` for motivation and strategic assessment;
- `CandidateGeneration` for commitment and intent candidates;
- `Choice` only for example profiles that need a terminal comparison.

Executors should be replaceable:

- rule executor for baseline deterministic tests;
- heuristic executor for plausible non-oracle comparisons;
- oracle executor only when labeled by profile and trace metadata;
- replay executor for fixture-based regression tests;
- LLM or hybrid executor later, behind the same contracts.

## Trace Ablation Value

The selected families support these small ablation pairs:

1. raw speech ignored vs typed speech interpreted;
2. typed speech only vs typed speech plus commitment candidate;
3. no other-model vs bounded other-model;
4. bounded other-model vs oracle other-model;
5. strategic assessment absent vs strategic assessment present;
6. strategic assessment only vs strategic plus motivational signal.

At least two of these should be implemented as concrete Phase 10 profile
examples. They are useful because each profile can be compared through typed
artifacts instead of free-form behavior descriptions.

## Recommended Deep-Research Scope

The next step should research these five families in detail:

1. speech interpretation;
2. social commitment candidate;
3. bounded other-model view;
4. strategic assessment;
5. motivational signal.

The deep research should produce minimal field recommendations, pass input and
output expectations, trace semantics, failure modes, and replaceable executor
boundaries for each family.
