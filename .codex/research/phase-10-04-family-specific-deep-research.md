# Phase 10 Family-Specific Deep Research

## Purpose

This note deepens the five first-class Phase 10 representation families selected
in `.codex/research/phase-10-03-representation-family-synthesis.md`.

It focuses on implementable artifact semantics rather than full theories. Each
family is described in terms of:

- research basis;
- minimal payload semantics;
- pass contract expectations;
- trace value;
- replacement boundary;
- failure modes.

The intended consumer is a later design and implementation plan for
`world-decision`.

## Common Design Rules

All Phase 10 families should follow these rules:

- artifacts are actor-relative unless explicitly oracle-labeled;
- every inference-heavy artifact should carry evidence or provenance links;
- confidence and ambiguity are trace data, not authority;
- no artifact directly mutates hard, social, epistemic, or appraisal state;
- executor algorithms are replaceable behind stable representation kinds and
  pass contracts;
- natural-language rationale may be diagnostic text, but never the only
  machine-readable content.

## 1. Speech Interpretation

### Research basis

FIPA-style communicative-act work supports a typed message vocabulary. ISO
24617-2 supports multidimensional dialogue annotation and is useful as a guard
against collapsing speech into a single flat label. Singh-style social semantics
argues that communication in open multiagent settings should be traceable
through public social effects such as commitments, not only private mental
states.

For `world`, this points to a two-layer family:

```text
SpeechSurface -> SpeechAct
```

The surface records what the actor can inspect. The speech-act candidate records
a typed interpretation that can feed later passes.

### Minimal semantics

`SpeechSurface` should preserve:

- speaker;
- addressees;
- raw or normalized utterance reference;
- time or local sequence marker;
- channel or scene context;
- visible participants or overhearers if available;
- source evidence.

`SpeechAct` should preserve:

- interpreted communicative function;
- semantic content;
- target object or target actor when available;
- relation to the surface;
- qualifiers such as conditionality, modality, polarity, sincerity uncertainty,
  and public/private audience;
- ambiguity alternatives;
- confidence;
- evidence links.

The initial communicative function set should stay small:

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

This set is enough to support social-strategic contrasts without implementing a
dialogue standard.

### Pass contract expectations

The first pass shape can be:

```text
ContextDerivation:
  ActorRelativeView / SocialContextView
  -> SpeechSurface

SemanticGrounding:
  SpeechSurface
  optional SocialContextView
  optional EpistemicView
  -> SpeechAct
```

Later executors can be rule, heuristic, LLM, hybrid, oracle, or replay-backed.
The output contract should not change when the executor changes.

### Trace value

Useful trace questions:

- Did the actor have access to the utterance?
- Which speech function did the profile infer?
- Did the profile keep an alternative interpretation?
- Did a later pass use the typed speech act or ignore it?
- Did the speech act generate a commitment candidate, other-model update, or
  strategic assessment input?

### Replacement boundary

Stable:

- representation kind identities;
- speech surface vs speech-act separation;
- communicative function vocabulary for the first schema version;
- provenance and ambiguity fields.

Replaceable:

- text parsing;
- function classifier;
- semantic content extraction;
- confidence scoring;
- LLM prompt or model choice.

### Failure modes

- Treating raw text as accepted fact.
- Collapsing indirect or ambiguous speech into one overconfident label.
- Letting a free-form LLM rationale become the artifact.
- Creating a large dialogue framework before concrete ablations need it.

## 2. Social Commitment Candidate

### Research basis

Commitment-based multiagent systems distinguish social commitments from private
intentions. Bratman and Cohen/Levesque are relevant for individual practical
commitment, but Singh-style work is the better local fit for public commitments
created or modified through communication and social institutions.

For `world`, a commitment candidate should be a possible public social relation,
not an accepted store mutation.

### Minimal semantics

A useful `CommitmentCandidate` should capture:

- commitment kind;
- debtor or bearer;
- creditor or claimant;
- beneficiary when distinct;
- content;
- activation condition or trigger;
- horizon, deadline, or expiry;
- witness scope;
- lifecycle candidate;
- source speech act or social context evidence;
- expected sanction, repair, or reputational consequence;
- confidence and contested status.

The initial commitment kinds can be:

```text
promise
agreement
debt
duty
obligation
oath
threat
offer
```

The initial lifecycle candidate set can be:

```text
proposed
created
detached
discharged
canceled
violated
contested
```

These states should be interpreted as candidate labels only. Later authority
gates can accept, reject, or transform them.

### Pass contract expectations

Two small pass shapes are enough:

```text
CandidateGeneration:
  SpeechAct
  optional SocialContextView
  -> CommitmentCandidate

CandidateGeneration:
  SocialContextView
  optional EpistemicView
  -> CommitmentCandidate
```

The first handles speech-derived commitments such as promises, threats, offers,
and agreements. The second handles context-derived duties, debts, oaths, or
institutional obligations visible to the actor.

### Trace value

Useful trace questions:

- Did the profile recognize a promise, threat, offer, duty, or debt?
- Which parties and witnesses did it assign?
- Was the commitment conditional or already active?
- Did the actor expect sanction or repair if the commitment is breached?
- Did the later choice pass treat the commitment as relevant?

### Replacement boundary

Stable:

- commitment candidate is separate from accepted social state;
- party/content/lifecycle/evidence fields;
- link to source speech act or social context.

Replaceable:

- extraction heuristic;
- lifecycle inference;
- sanction estimate;
- commitment ranking;
- later provider or model.

### Failure modes

- Treating candidate commitments as accepted obligations.
- Conflating private intent with public commitment.
- Encoding legal or institutional authority rules into the first schema.
- Losing witness scope, which is central to later reputation and sanction
  reasoning.

## 3. Bounded Other-Model View

### Research basis

Interactive multiagent planning shows why modeling another actor's beliefs,
goals, and likely policy can improve decisions. It also shows why unrestricted
nested models are costly. Social-agent benchmarks make hidden information,
deception, and mixed motives central, but often leave the internal causal trace
implicit.

For `world`, the representation should be one bounded, evidence-linked model of
a target actor or institution.

### Minimal semantics

`OtherModelView` should capture:

- target actor or institution;
- model owner or perspective actor;
- modeled beliefs or knowledge claims;
- modeled goals, needs, or pressures;
- modeled commitments;
- likely response or policy;
- uncertainty;
- evidence links;
- model depth;
- oracle flag or oracle source when applicable;
- stale/validity horizon when available.

The first schema should support exactly one explicit target per artifact.
Nested beliefs should be summarized as claims rather than expanded recursively.

### Pass contract expectations

Initial pass shapes:

```text
OtherModeling:
  ActorRelativeView
  optional SocialContextView
  optional SpeechAct
  optional CommitmentCandidate
  -> OtherModelView
```

Oracle comparison profile:

```text
OtherModeling:
  ActorRelativeView
  -> OtherModelView(oracle-labeled)
```

The oracle variant must be visibly oracle-labeled in representation metadata and
execution metadata.

### Trace value

Useful trace questions:

- Did the profile model the relevant actor at all?
- Which evidence supported the model?
- Did it confuse known fact, inferred belief, and oracle knowledge?
- Did a strategic assessment consume the model?
- Did oracle access improve the result relative to bounded non-oracle access?

### Replacement boundary

Stable:

- target and owner separation;
- depth bound;
- evidence links;
- uncertainty and oracle labeling.

Replaceable:

- model-construction heuristic;
- ranking of likely responses;
- policy representation;
- stale-model invalidation;
- LLM or oracle implementation.

### Failure modes

- Omniscient leakage through an unlabeled other-model.
- Recursive theory-of-mind expansion.
- False precision from structured fields generated by weak evidence.
- Treating another actor's modeled belief as actual world truth.

## 4. Strategic Assessment

### Research basis

Systems such as CICERO illustrate the value of separating strategic reasoning
from dialogue generation. For `world`, the useful artifact is not a
Diplomacy-specific policy; it is a bounded assessment that turns typed social
inputs into predicted consequences relevant to choice.

This family is especially important because it gives profiles a traceable place
to expose leverage, sanctions, bargaining position, and disclosure risk.

### Minimal semantics

`StrategicAssessment` should capture:

- assessed candidate or situation;
- likely responses from relevant actors;
- leverage or bargaining position;
- expected sanctions;
- expected commitment effects;
- information disclosure risk;
- trust, reputation, or relationship risk;
- upside and downside;
- time horizon;
- confidence and evidence.

The payload should be a structured assessment. Free-form rationale can be a
diagnostic supplement, not the core data.

### Pass contract expectations

Initial pass shape:

```text
CognitiveSignal:
  SpeechAct
  optional CommitmentCandidate
  optional OtherModelView
  optional SocialContextView
  -> StrategicAssessment
```

An alternative profile can consume the same inputs but omit this pass, allowing
direct comparison against a choice path that lacks explicit strategic
assessment.

### Trace value

Useful trace questions:

- Did the profile assess likely retaliation, compliance, or refusal?
- Did it identify leverage or bargaining weakness?
- Did it account for public witnesses or information disclosure?
- Did the later choice pass use the assessment?
- Did an oracle other-model improve strategy without changing speech parsing?

### Replacement boundary

Stable:

- strategic assessment is explicit and separate from speech and other-model
  artifacts;
- fields for response prediction, leverage, sanctions, and disclosure risk;
- evidence and confidence.

Replaceable:

- scoring model;
- risk aggregation;
- expected response predictor;
- bargaining heuristic;
- LLM or hybrid implementation.

### Failure modes

- Hiding strategic reasoning inside a narrative explanation.
- Collapsing strategy into motivation or emotion.
- Hard-coding one game protocol.
- Treating assessed risks as accepted future events.

## 5. Motivational Signal

### Research basis

Appraisal theories are useful because they decompose action-readiness into
structured variables such as threat, goal relevance, coping feasibility, and
norm conflict. Local architecture deliberately keeps appraisal optional.

For `world`, the first useful artifact is not an emotion state. It is a
replaceable motivational signal that helps explain why a candidate action,
speech act, or commitment matters to the actor.

### Minimal semantics

`MotivationalSignal` should capture:

- signal target;
- pressure kind;
- valence or direction;
- intensity or priority;
- urgency;
- threat or opportunity;
- norm conflict;
- relationship salience;
- coping feasibility;
- expected sanction;
- evidence and confidence.

Initial pressure kinds can be:

```text
self_preservation
relationship
norm
reputation
resource
curiosity
loyalty
fear
anger
guilt
opportunity
```

The schema should allow non-emotion dialects. For example, a utility-style
executor could produce resource/opportunity signals while an appraisal-style
executor produces fear/guilt/anger-like signals.

### Pass contract expectations

Initial pass shape:

```text
CognitiveSignal:
  ActorRelativeView
  optional SocialContextView
  optional CommitmentCandidate
  optional OtherModelView
  optional StrategicAssessment
  -> MotivationalSignal
```

The pass should be optional in Phase 10 profiles.

### Trace value

Useful trace questions:

- Which pressures did the profile expose before choice?
- Were the pressures grounded in evidence, commitments, or other-models?
- Did motivational signals change the selected choice relative to strategy-only
  profiles?
- Did a signal overfit an emotion label without useful decision content?

### Replacement boundary

Stable:

- motivation is a decision artifact, not accepted actor state;
- target, pressure kind, strength, urgency, and evidence fields;
- optional profile role.

Replaceable:

- appraisal model;
- utility computation;
- normative priority model;
- emotion labels;
- aggregation into candidate scores.

### Failure modes

- Making appraisal mandatory.
- Treating emotion labels as explanations without feature-level evidence.
- Writing long-term mood or personality state from a decision pass.
- Blurring motivational pressure with strategic consequence prediction.

## Cross-Family Ordering

The smallest useful ordering is:

```text
SpeechSurface
  -> SpeechAct
  -> CommitmentCandidate

ActorRelativeView / SocialContextView
  -> OtherModelView

SpeechAct + CommitmentCandidate + OtherModelView
  -> StrategicAssessment

StrategicAssessment + CommitmentCandidate + ActorRelativeView
  -> MotivationalSignal

selected subset
  -> IntentCandidate / Choice
```

The ordering is not a hard architecture requirement. It is a recommended initial
profile family for tests and examples because it makes each incremental artifact
visible in the trace.

## Cross-Family Evidence Model

Each artifact should be able to point backward to prior artifacts or context
inputs:

```text
SpeechAct
  evidence: SpeechSurface

CommitmentCandidate
  evidence: SpeechAct, SocialContextView

OtherModelView
  evidence: ActorRelativeView, SpeechAct, CommitmentCandidate

StrategicAssessment
  evidence: SpeechAct, CommitmentCandidate, OtherModelView

MotivationalSignal
  evidence: ActorRelativeView, CommitmentCandidate, StrategicAssessment
```

This supports trace debugging without granting any artifact authority.

## Minimal Deep-Research Conclusion

The five selected families are enough for Phase 10. They cover:

- language input and typed interpretation;
- public social relation candidates;
- bounded theory of mind;
- consequence prediction;
- optional motivational pressure.

Together they support the Phase 10 requirement: paired social-strategic ablation
profiles with typed, comparable traces. They also keep the implementation
swappable. Future work can replace the executors with stronger algorithms or
LLM-backed implementations without changing the first stable contract surface.
