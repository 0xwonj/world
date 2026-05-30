# Phase 10 Artifact Shape And Ablation Plan

## Purpose

This note proposes minimal artifact shapes, representation-kind declarations,
pass contracts, trace semantics, and ablation pairs for the Phase 10
social-cognitive representation slice.

It is not an implementation patch. It is a research-to-design bridge for the
next implementation plan.

## Contract Principles

Phase 10 should stabilize:

- concrete representation kind names;
- representation roles, visibility, persistence, and authority declarations;
- pass input and output contracts;
- trace semantics for comparing profiles;
- evidence-link and oracle-label requirements.

Phase 10 should keep replaceable:

- parser or classifier implementation;
- scoring algorithm;
- other-model construction;
- strategic/risk estimation;
- motivation/appraisal dialect;
- LLM, hybrid, oracle, replay, or rule executor implementation.

## Representation Kind Set

The initial slice can use this kind set.

| Kind | Role | Visibility | Persistence | Authority |
| --- | --- | --- | --- | --- |
| `speech_surface` | `SpeechSurface` | actor-visible or research trace | trace-recorded | derived |
| `speech_act_candidate` | `SpeechAct` | research trace | trace-recorded | derived |
| `social_commitment_candidate` | `CommitmentCandidate` | research trace | proposal-only | proposal to non-hard authority, or derived if routing is not yet implemented |
| `bounded_other_model_view` | `OtherModelView` | research trace | trace-recorded | derived |
| `oracle_other_model_view` | `OtherModelView` | oracle-only | trace-recorded | oracle |
| `strategic_assessment` | `StrategicAssessment` | research trace | trace-recorded | derived |
| `motivational_signal` | `MotivationalSignal` | research trace | trace-recorded | derived |
| `intent_candidate` | `IntentCandidate` | research trace | trace-recorded | derived |
| `social_decision_choice` | `Choice` | research trace | trace-recorded | derived |
| `social_cognitive_diagnostic` | `Diagnostic` | diagnostic-only | trace-recorded | diagnostic |

The exact `AuthorityClass` for `social_commitment_candidate` should follow the
existing authority taxonomy. If the current non-hard class is not specific
enough, keep the first schema `Derived` and route accepted social-state
proposals later rather than expanding authority in Phase 10.

No kind in this slice should declare hard authority.

## Minimal Payload Shapes

These shapes are conceptual. The implementation can use local concrete Rust
types, fixtures, or lightweight typed payload enums according to the existing
codebase style.

### `SpeechSurfacePayload`

```text
SpeechSurfacePayload:
  surface_id
  speaker
  addressees
  overhearers
  channel
  local_sequence
  utterance_ref
  normalized_text
  context_refs
  evidence_refs
```

Required for the first slice:

- `surface_id`
- `speaker`
- `utterance_ref` or `normalized_text`
- `evidence_refs`

Optional:

- addressees;
- overhearers;
- channel;
- local sequence;
- context refs.

### `SpeechActCandidatePayload`

```text
SpeechActCandidatePayload:
  speech_act_id
  surface_ref
  speaker
  addressees
  function
  semantic_content
  target_refs
  qualifiers:
    polarity
    conditionality
    modality
    publicness
    sincerity_uncertainty
  alternatives
  confidence
  evidence_refs
```

Initial `function` values:

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

`alternatives` should be allowed from the beginning. It prevents early examples
from pretending that speech interpretation is exact.

### `SocialCommitmentCandidatePayload`

```text
SocialCommitmentCandidatePayload:
  commitment_id
  commitment_kind
  lifecycle_candidate
  debtor
  creditor
  beneficiary
  content
  activation_condition
  horizon
  witness_scope
  expected_sanction
  expected_repair
  contested_by
  source_refs
  confidence
```

Initial `commitment_kind` values:

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

Initial `lifecycle_candidate` values:

```text
proposed
created
detached
discharged
canceled
violated
contested
```

The payload should explicitly distinguish the actor who bears the commitment
from the actor or institution that can claim it.

### `BoundedOtherModelViewPayload`

```text
BoundedOtherModelViewPayload:
  model_id
  owner_actor
  target
  depth_limit
  modeled_beliefs
  modeled_goals
  modeled_commitments
  likely_responses
  uncertainty
  validity_horizon
  evidence_refs
  oracle_label
```

Required:

- owner actor;
- target;
- depth limit;
- at least one modeled claim or likely response;
- uncertainty or confidence;
- evidence refs;
- oracle label when oracle-sourced.

The first slice should set `depth_limit = 1` for normal models. If a fixture
needs a nested belief, encode it as a claim in `modeled_beliefs`, not as an
expanded model tree.

### `StrategicAssessmentPayload`

```text
StrategicAssessmentPayload:
  assessment_id
  assessed_options
  relevant_actors
  likely_responses
  leverage
  expected_sanctions
  commitment_effects
  information_disclosure_risk
  trust_or_reputation_risk
  upside
  downside
  horizon
  confidence
  evidence_refs
```

Required:

- assessed option or situation;
- at least one predicted response, risk, leverage, or sanction entry;
- confidence;
- evidence refs.

This artifact should stay separate from `MotivationalSignal`: strategy predicts
or evaluates social consequences, while motivation records pressure on the
actor.

### `MotivationalSignalPayload`

```text
MotivationalSignalPayload:
  signal_id
  target_ref
  pressure_kind
  direction
  intensity
  urgency
  threat
  opportunity
  norm_conflict
  relationship_salience
  coping_feasibility
  expected_sanction
  confidence
  evidence_refs
```

Initial `pressure_kind` values:

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

The first schema should allow sparse fields. A strategy-oriented executor can
produce `resource` and `opportunity` signals without claiming an emotion model.

### `IntentCandidatePayload`

```text
IntentCandidatePayload:
  intent_id
  actor
  purpose
  approach
  target_refs
  supporting_refs
  blocking_refs
  confidence
```

This should be treated as a connector payload. It is useful for choice examples,
but Phase 10 does not need to make intent theory a deep new area.

## Pass Contract Sketches

These are contract sketches, not final Rust definitions.

### Extract Speech Surface

```text
PassClass: ContextDerivation
Inputs:
  ActorRelativeView required
  SocialContextView optional
Outputs:
  SpeechSurface
Implementation modes:
  Rule, Heuristic, Replay
```

Trace expectation:

- records whether the actor could inspect the utterance;
- never adds unseen speech unless explicitly oracle-labeled.

### Ground Speech Act

```text
PassClass: SemanticGrounding
Inputs:
  SpeechSurface required_all
  SocialContextView optional
  EpistemicView optional
Outputs:
  SpeechAct
Implementation modes:
  Rule, Heuristic, Llm, Hybrid, Oracle, Replay
```

Trace expectation:

- records function, content, ambiguity, confidence, and source surface.

### Generate Commitment Candidate

```text
PassClass: CandidateGeneration
Inputs:
  SpeechAct optional_all
  SocialContextView optional
  EpistemicView optional
Outputs:
  CommitmentCandidate
Implementation modes:
  Rule, Heuristic, Llm, Hybrid, Oracle, Replay
```

Trace expectation:

- records party assignment, lifecycle candidate, evidence, and whether the
  result is speech-derived or context-derived.

### Build Bounded Other-Model

```text
PassClass: OtherModeling
Inputs:
  ActorRelativeView required
  SpeechAct optional_all
  CommitmentCandidate optional_all
  SocialContextView optional
Outputs:
  OtherModelView
Implementation modes:
  Rule, Heuristic, Llm, Hybrid, Oracle, Replay
```

Trace expectation:

- records target, owner, depth, uncertainty, evidence, and oracle status.

### Assess Strategy

```text
PassClass: CognitiveSignal
Inputs:
  SpeechAct optional_all
  CommitmentCandidate optional_all
  OtherModelView optional_all
  SocialContextView optional
Outputs:
  StrategicAssessment
Implementation modes:
  Rule, Heuristic, Llm, Hybrid, Oracle, Replay
```

Trace expectation:

- records predicted responses, sanctions, leverage, disclosure risk, evidence,
  and confidence.

### Compute Motivation

```text
PassClass: CognitiveSignal
Inputs:
  ActorRelativeView required
  CommitmentCandidate optional_all
  OtherModelView optional_all
  StrategicAssessment optional_all
Outputs:
  MotivationalSignal
Implementation modes:
  Rule, Heuristic, Llm, Hybrid, Oracle, Replay
```

Trace expectation:

- records pressure target, pressure kind, intensity, urgency, evidence, and
  confidence.

### Generate Intent Candidate

```text
PassClass: CandidateGeneration
Inputs:
  CommitmentCandidate optional_all
  OtherModelView optional_all
  StrategicAssessment optional_all
  MotivationalSignal optional_all
  ActionRepertoire optional
Outputs:
  IntentCandidate
Implementation modes:
  Rule, Heuristic, Llm, Hybrid, Oracle, Replay
```

Trace expectation:

- records what artifacts supported or blocked a candidate purpose and approach.

### Select Choice

```text
PassClass: Choice
Inputs:
  IntentCandidate required_all
  StrategicAssessment optional_all
  MotivationalSignal optional_all
Outputs:
  Choice
Implementation modes:
  Rule, Heuristic, Llm, Hybrid, Oracle, Replay
```

Trace expectation:

- records selected candidate and evidence refs;
- remains a decision artifact, not a runtime execution request.

## Profile And Ablation Plan

The implementation should start with small fixed profiles. Each profile should
use the same fixture context where possible.

### Profile A: Direct Action Baseline

```text
ActorRelativeView
  -> IntentCandidate
  -> Choice
```

Purpose:

- establishes behavior without typed social-cognitive artifacts.

Expected trace:

- no speech act;
- no commitment candidate;
- no other-model;
- no strategic assessment.

### Profile B: Typed Speech

```text
ActorRelativeView
  -> SpeechSurface
  -> SpeechAct
  -> IntentCandidate
  -> Choice
```

Purpose:

- tests whether typed speech interpretation changes candidate generation or
  choice compared with raw context.

Primary comparison:

- Profile A vs Profile B.

### Profile C: Speech Plus Commitment

```text
ActorRelativeView
  -> SpeechSurface
  -> SpeechAct
  -> CommitmentCandidate
  -> IntentCandidate
  -> Choice
```

Purpose:

- tests whether a promise, threat, offer, debt, or duty candidate changes choice
  compared with typed speech alone.

Primary comparison:

- Profile B vs Profile C.

### Profile D: Bounded Other-Model

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

Purpose:

- tests whether a bounded model of another actor improves strategic assessment
  and choice.

Primary comparison:

- Profile C vs Profile D.

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

Purpose:

- measures the ceiling gained from oracle target modeling without changing the
  rest of the profile.

Primary comparison:

- Profile D vs Profile E.

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

Purpose:

- tests whether explicit pressure signals add explanatory or behavioral value
  beyond strategic assessment.

Primary comparison:

- Profile D vs Profile F.

## Minimum Implementation Target

Phase 10 does not need all six profiles to land at once. The minimal target is:

1. Profile A direct baseline;
2. Profile C speech plus commitment;
3. Profile D bounded other-model plus strategic assessment;
4. Profile E oracle other-model upper bound, if oracle metadata is already
   convenient to express in the existing substrate.

Profile F is valuable but optional because motivation/appraisal is explicitly a
dialect, not a required primitive.

## Trace Assertions

Targeted tests should assert:

- every produced artifact satisfies its declared role;
- required pass inputs are resolved through profile flow validation;
- optional-all inputs can consume multiple compatible artifacts;
- oracle other-model artifacts require oracle-aware profile labeling;
- commitment candidates are proposal or derived artifacts, not hard authority;
- choice artifacts can be terminal outputs without becoming runtime execution
  requests;
- omitted passes leave visible trace differences rather than silently producing
  empty placeholders.

## Metrics For Research Examples

The first examples can use trace-level metrics instead of full simulation
outcome metrics:

- speech act recognized or missed;
- commitment candidate produced or absent;
- correct party assignment in fixture;
- other-model target and evidence coverage;
- oracle vs bounded other-model divergence;
- strategic risk identified or missed;
- selected choice changed relative to baseline;
- trace contains enough evidence to explain the difference.

These metrics can later feed the broader social-strategic benchmark methodology.

## Implementation Risk Checklist

Before implementing, check these risks:

- Does any artifact imply accepted social or epistemic state?
- Does any pass need access outside `DecisionContextView`?
- Does any kind require a new authority class?
- Does any profile depend on a new runtime, persistence, parser, or provider?
- Are speech, commitment, other-model, strategy, and motivation still separable
  in traces?
- Can the same profile contract run with rule, replay, oracle, or later LLM
  executors?

If any answer violates the local constraints, shrink the slice rather than
changing architecture boundaries.
