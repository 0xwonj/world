# Phase 10 Broad External Survey

## Purpose

This survey identifies external research lenses that are useful for Phase 10.
The goal is not to import a full cognitive architecture. The goal is to find
representation ideas that can become typed decision artifacts, pass contracts,
and trace semantics inside `world-decision`.

This survey builds on `.codex/research/phase-10-01-local-constraints.md`.

## Sources Reviewed

Speech and dialogue:

- ISO 24617-2:2020, `Language resource management -- Semantic annotation
  framework -- Part 2: Dialogue acts`
  <https://www.iso.org/standard/76443.html>
- FIPA Communicative Act Library Specification
  <https://jmvidal.cse.sc.edu/library/XC00037H.pdf>
- Singh, `A Social Semantics for Agent Communication Languages`
  <https://www.csc2.ncsu.edu/faculty/mpsingh/papers/mas/ijcai-99-acl.pdf>

Commitment and intention:

- Cohen and Levesque, `Intention Is Choice with Commitment`
  <https://ai.stanford.edu/~epacuit/classes/lori-spr09/cohenlevesque-intention-aij90.pdf>
- Bratman, `Intention, Plans, and Practical Reason`
  <https://web.stanford.edu/group/cslipublications/cslipublications/site/1575861925.shtml>
- King, Gunay, Chopra, and Singh, `Tosca: Operationalizing Commitments Over
  Information Protocols`
  <https://www.ijcai.org/proceedings/2017/37>
- Singh and Xing, `Engineering Commitment-Based Multiagent Systems`
  <https://www.csc2.ncsu.edu/faculty/mpsingh/papers/mas/aamas-03-ctl.pdf>

Other-modeling and strategic reasoning:

- Gmytrasiewicz and Doshi, `A Framework for Sequential Planning in Multi-Agent
  Settings`
  <https://www.cs.cmu.edu/afs/cs.cmu.edu/project/jair/pub/volume24/gmytrasiewicz05a.pdf>
- Meta FAIR, `Human-level play in the game of Diplomacy by combining language
  models with strategic reasoning`
  <https://www.science.org/doi/10.1126/science.ade9097>
- Meta CICERO research page
  <https://ai.meta.com/research/cicero/>

Social-agent evaluation:

- SOTOPIA
  <https://arxiv.org/abs/2310.11667>
- Concordia
  <https://arxiv.org/abs/2312.03664>
- AvalonBench
  <https://openreview.net/pdf?id=ltUrSryS0K>
- MACHIAVELLI
  <https://arxiv.org/abs/2304.03279>
- MafiaBench
  <https://www.mafiabench.org/>
- M3-BENCH
  <https://arxiv.org/abs/2601.08462>
- Cattle Trade
  <https://arxiv.org/abs/2605.14537>

Appraisal:

- OCC-related construction/appraisal discussion
  <https://users.cs.northwestern.edu/~ortony/Andrew_Ortony_files/2013%20-%20OCC%20and%20constructionism.pdf>
- EMA appraisal dynamics reference
  <https://ict.usc.edu/pubs/EMA-%20A%20process%20model%20of%20appraisal%20dynamics.pdf>

## Survey Findings

### 1. Speech acts are useful only if they stay multidimensional

The FIPA Communicative Act Library is useful because it treats communicative
acts as typed message primitives rather than raw text. It suggests that Phase
10 should preserve a small communicative function vocabulary such as inform,
request, propose, promise, accept, reject, refuse, query, confirm, deny, warn,
and threaten.

ISO 24617-2 is more useful as a warning against flat labels. Dialogue units may
carry multiple communicative functions across dimensions. For Phase 10, a
speech act should not be only:

```text
kind = promise
```

It should be able to preserve:

```text
speaker
addressee
surface reference
communicative function
semantic content
qualifiers
target social object
evidence/provenance
confidence or ambiguity
```

The implementation should still start smaller than ISO. The transferable idea
is multidimensional typed annotation, not the full standard.

### 2. Social semantics is a better fit than mentalistic speech semantics

Singh's social semantics argument is especially relevant to `world`.
Communication in open multiagent settings should not depend only on private
beliefs and intentions that an observer cannot verify. Publicly inspectable
social commitments give communication a traceable meaning.

For Phase 10, this means typed speech acts should be allowed to produce:

- commitment candidates;
- social-claim proposals;
- epistemic-update proposals;
- diagnostic rationale;
- strategic assessment inputs.

They should not directly mutate social state. The trace should show how a
speech act candidate led to a commitment candidate or strategic assessment.

### 3. Psychological commitment and social commitment must stay separate

Bratman and Cohen/Levesque motivate intent as a commitment-like practical
attitude that stabilizes future action. Singh-style social commitment work
models public commitments between agents.

Phase 10 should preserve this separation:

```text
IntentCandidate / Intent:
  actor's possible or selected commitment to a purpose and approach.

CommitmentCandidate:
  possible social commitment, such as promise, threat, agreement, duty, debt,
  or obligation relation involving multiple social parties.
```

Conflating these would damage trace quality. A character can intend to lie,
promise to help, be socially obligated to help, and strategically expect another
actor to punish non-help. Those are related but not the same artifact.

### 4. Commitment protocols emphasize lifecycle and alignment

Commitment-based multiagent work repeatedly emphasizes operations and
alignment: commitments are created, detached, discharged, canceled, violated,
delegated, assigned, or otherwise transformed. Tosca specifically highlights
alignment across decentralized agents: agents must make compatible inferences
about commitments despite partial information and communication delay.

Phase 10 does not need full commitment protocol synthesis. It does need enough
fields to make lifecycle and observer scope visible:

```text
debtor
creditor
beneficiary?
content
condition / trigger
deadline / horizon
witnesses
status candidate
evidence links
expected sanction or repair path
```

This makes later metrics like promise kept, promise breached, threat credible,
and commitment witnessed possible.

### 5. Bounded other-models should be evidence-linked and depth-limited

Interactive POMDPs show why modeling another agent can improve decisions: the
acting agent can reason about another agent's beliefs, preferences, capabilities,
and intended actions. They also show why unrestricted nested modeling is
dangerous: interactive belief spaces become difficult quickly.

Phase 10 should therefore use a bounded representation:

```text
OtherModelView
  subject actor or institution
  modeled beliefs
  modeled goals or pressures
  modeled commitments
  likely policy / response
  uncertainty
  evidence links
  depth limit
  oracle flag if applicable
```

No nested theory-of-mind tree should be introduced in the first slice.

### 6. Strategic reasoning benefits from separating dialogue from strategy

CICERO is relevant because it combines language and strategic reasoning rather
than treating free-form dialogue as the whole agent. This maps well to Phase
10: typed speech should feed strategic assessment, but the profile should still
make the strategy artifact explicit.

For `world`, the useful artifact is not a Diplomacy-specific plan. It is a
general traceable assessment:

```text
StrategicAssessment
  likely responses
  leverage
  risks
  sanctions
  bargaining position
  information disclosure risk
  expected commitment effects
```

This should be a decision artifact, not an accepted truth package.

### 7. Social-agent benchmarks expose the gap Phase 10 should target

SOTOPIA, AvalonBench, MafiaBench, M3-BENCH, MACHIAVELLI, Concordia, and Cattle
Trade show active interest in social intelligence, hidden information,
deception, bargaining, mixed motives, and long-horizon interaction.

Their common limitation for `world` is that many measurements are game- or
transcript-centered. They may record rich logs, but the causal variables behind
social success are often embedded in a fixed protocol or natural-language
transcript.

Phase 10 should target a complementary strength:

```text
typed belief / speech / commitment / intent / action / consequence traces
```

The system should make it possible to ask whether an agent failed because it
missed a speech act, lacked a commitment representation, mis-modeled another
actor, ignored a sanction, or chose poorly despite having the right artifacts.

### 8. Appraisal theory is useful as a structured-signal lens, not a kernel

Appraisal models are useful because they connect situations to action-readiness
or motivational pressure through structured variables rather than raw emotion
labels. This supports the local design stance that mood-only or
emotion-taxonomy-only models are too weak.

However, Phase 10 should not import a full appraisal model. The useful
transfer is a minimal `MotivationalSignal` or `StrategicAssessment` shape with
feature-level reasons:

```text
norm conflict
relationship salience
threat
opportunity
coping feasibility
urgency
risk
expected sanction
```

Appraisal-like signals should remain replaceable by utility, normative, or
strategic dialects.

## Implications For Phase 10

### Keep

- typed speech acts with ambiguity and provenance;
- social commitment candidates distinct from intent candidates;
- bounded other-model views with depth and evidence limits;
- strategic assessment as explicit trace artifact;
- motivation/appraisal-like signals as optional dialect;
- oracle and LLM labels as metadata, not authority.

### Avoid

- importing a complete dialogue-act standard;
- making a BDI interpreter;
- making appraisal the central engine primitive;
- modeling unlimited nested beliefs;
- treating free-form LLM rationale as state;
- tying the first slice to a fixed game protocol;
- creating a generic plugin framework before concrete artifacts exist.

## Candidate Lenses For The Next Step

The next synthesis step should map this survey into representation families:

1. `SpeechAct` and `SpeechSurface`;
2. `CommitmentCandidate`;
3. `BoundedOtherModelView`;
4. `StrategicAssessment`;
5. `MotivationalSignal` or `PressureSignal`;
6. optional `IntentCandidate` / `IntentScore` support only if needed for the
   ablation.

The first implementation slice should choose the smallest subset that can
produce comparable traces, not the largest theory coverage.
