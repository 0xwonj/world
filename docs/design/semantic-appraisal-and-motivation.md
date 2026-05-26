# Semantic Appraisal And Motivation

## Status

Current design draft.

This document defines the target structure for semantic appraisal and
motivation. It is not an implementation schema or delivery plan. The record
shapes are conceptual contracts for later implementation.

## Source Research

- [Semantic Appraisal, Intent, Activity, And Planning](../research/semantic-appraisal-intent-activity-planning.md)
- [Epistemic State / Agent Memory](../research/epistemic-state-and-agent-memory.md)
- [Actor Pressure And Interpretation](../ideas/actor-pressure-and-interpretation.md)
- [Semantic Kernel And PL Boundary](../ideas/semantic-kernel-and-pl-boundary.md)

## Related Design Owners

- [Simulation Core](simulation-core.md)
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
- [World Model](world-model.md)
- [Perception And Observation](perception-and-observation.md)
- [Epistemic State](epistemic-state.md)
- [Social Institutional Model](social-institutional-model.md)
- [Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md)
- [Intent Templates And Planning](intent-templates-and-planning.md)
- [Multi-Resolution Simulation](multi-resolution-simulation.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)

## Purpose

Semantic appraisal turns actor-relative information into actor-relative
meaning and motivational pressure.

It answers:

```text
What does this observed event, memory, belief, rumor, secret, social context,
or current condition mean to this holder right now?
```

It does not answer:

```text
Which intent should the actor commit to?
Which concrete action should happen?
Which hard state should mutate?
```

The appraisal framework is a reusable mechanism. Concrete vocabularies such as
honor, sin, taboo, revenge, shame, awe, gratitude, fear, loyalty, legal
culpability, ritual impurity, faction duty, or religious pollution may be
defined by game-system packs. Accepted appraisal output still uses the common
appraisal commit surface.

## Chosen Structure

Use a typed appraisal pipeline:

```text
ObservedEvent / ObservedState
  + EpistemicWorkingSet
  + SocialContextView
  + actor context
  + current Activity / Intent context where visible
  -> AppraisalVariableSet
  -> AppraisalRule match
  -> Thought
  -> Pressure
  -> GoalPressure
  -> AcceptedAppraisalRecord
  -> AppraisalRecordStore
```

The essential split is:

```text
Thought:
  interpreted meaning.

Pressure:
  motivational force / action-readiness direction.

GoalPressure:
  pressure shaped toward a possible desired state, still not commitment.
```

This layer may create, decay, suppress, reactivate, and explain appraisal
records. It must not select final `Intent`, submit `ActionRequest`, tick
`ProcessInstance`, mutate hard truth, write `EpistemicStore`, or commit
`SocialInstitutionalStore`.

## Compiler Placement

In the [Simulation Transition Compiler](simulation-transition-compiler.md),
semantic appraisal is the first half of the decision middle-end. It is semantic
analysis over actor-relative context, not executable lowering.

```text
pass:
  SemanticAppraisal

pass class:
  Derivation

transformation kind:
  Analysis / rule matching

input class:
  DerivedContext

input representations:
  ObservedEvent
  ObservedState
  EpistemicWorkingSet
  SocialContextView
  actor context
  visible current Intent / Activity context

output class:
  Decision

output representations:
  AppraisalVariableSet
  Thought
  Pressure
  GoalPressure
  appraisal record proposal
```

Its job is to turn "what the actor can currently perceive, remember, believe,
and socially interpret" into meaning and motivational pressure.

It is not a choice pass:

```text
AppraisalVariableSet -> Thought / Pressure / GoalPressure
```

not:

```text
ObservedEvent -> Intent
Pressure -> ActionRequest
Thought -> hard mutation
```

An implementation may fold appraisal into a larger decision pass for
performance or iteration speed, but the logical boundary must remain visible:
the pass must preserve provenance, obey actor-relative access limits, and route
accepted appraisal writes through the appraisal commit surface.

## Alternatives Considered

### Mood-Only Model

Rejected as the primary structure.

```text
event -> mood number -> behavior bias
```

This is compact, but too weak for an RPG where the same event can mean grief,
crime, taboo, relief, justice, betrayal, debt, threat, or opportunity depending
on actor and context. A mood value can be a derived view later, but it should
not be the core appraisal state.

### Emotion-Taxonomy-Only Model

Rejected as the core.

```text
event -> emotion label
```

Emotion labels are useful presentation and grouping vocabulary, but they do not
preserve why the emotion was produced. The engine needs variables such as
relationship weight, agency, coping potential, duty relevance, and norm
compatibility so later intent scoring and debug tools can explain behavior.

### Social-State-Direct-Motive Model

Rejected.

```text
SocialClaim / Norm / Law -> intent bias
```

Social state is context, not motive. A shrine norm does not itself make a guard
warn, arrest, report, or ignore. Appraisal evaluates the observed event against
the actor's accessible social context and role, then creates pressure.

### Memory-Direct-Action Model

Rejected.

```text
EpistemicRecord -> action or intent
```

Memory, belief, rumor, and secret state provide actor-relative information.
They do not choose actions. They feed working sets, appraisal, pressure, and
later intent generation.

### Monolithic Agent Brain

Rejected.

```text
perception + memory + social context + emotion + planner -> action
```

This makes behavior hard to inspect and lets one component accidentally read
omniscient state or bypass mutation authority. `world` uses separated typed
passes: observation, epistemic retrieval, social context, appraisal, intent
generation, resolution lowering, and causal execution.

### LLM Interpretation As Authority

Rejected.

LLMs may propose appraisals or explanations, but gameplay-relevant
interpretations must become typed `AcceptedAppraisalRecord`s with provenance.
Generated prose is not actor truth, social truth, or hard truth.

## Position In The Engine

Semantic appraisal sits after perception, epistemic retrieval, and social
context construction. It sits before intent generation.

```text
EventRecord / WorldState
  -> Perception And Observation
  -> ObservedEvent / ObservedState
  -> Epistemic State persistence and retrieval
  -> EpistemicWorkingSet
  -> SocialContextView
  -> Semantic Appraisal And Motivation
  -> Thought / Pressure / GoalPressure
  -> Intent Templates And Planning
  -> Intent
  -> Activity
  -> ActionRequest or ProcessInstance
  -> Causal Runtime
```

Appraisal is a derivation and accepted-record publication layer. It is not hard
mutation and not final choice.

## Boundary

This layer owns:

- `AppraisalVariableSet`
- `AppraisalRule`
- `Thought`
- `Pressure`
- `GoalPressure`
- appraisal record provenance
- appraisal record lifecycle
- pressure conflict, decay, suppression, and reactivation
- accepted appraisal commit semantics
- AI appraisal proposal acceptance rules
- debug explanation for why a meaning or pressure exists

This layer does not own:

- hard world mutation
- `EventRecord` emission
- perception projection
- `EpistemicRecord` storage
- social or institutional state commit
- final `Intent` selection
- `CandidateIntent` generation
- `ActionRequest` validation
- `Typed Effect Program` execution
- `ProcessInstance` ticking
- final natural-language presentation

## Inputs

Appraisal reads actor-relative or access-filtered inputs. It should not receive
an omniscient world dump.

Primary inputs:

```text
ObservedEvent:
  actor-relative projection of a committed EventRecord.

ObservedState:
  actor-relative projection of current world state.

EpistemicWorkingSet:
  relevant EpistemicRecord views for the holder.

SocialContextView:
  authoritative, holder-known, or role-granted social context made explicit by
  the social model.

Actor context:
  body condition, role, active obligations, current activity, current intent,
  accessible values, and pack-defined traits where the current mode exposes
  them.

AppraisalRule definitions:
  checked pack-authored rules over the above inputs.
```

Rules:

- `ObservedEvent` and `ObservedState` preserve uncertainty.
- `EpistemicWorkingSet` can contain false or stale beliefs.
- `SocialContextView` must label access mode and provenance.
- If appraisal uses privileged social context, the resulting record should
  preserve that mode so actor-facing output does not leak hidden truth.
- Current `Intent` or `Activity` may be read for relevance and interruption
  pressure, but appraisal cannot replace it directly.

## AppraisalVariableSet

`AppraisalVariableSet` is the typed intermediate evaluation of meaning.

It is the main way to avoid turning appraisal into opaque labels.

Conceptual shape:

```text
AppraisalVariableSet
  holder
  focus
  source_observation?
  source_epistemic_records
  source_social_context
  novelty
  relevance
  goal_conduciveness
  agency
  blame_or_credit
  certainty
  urgency
  coping_potential
  control_potential
  norm_compatibility
  role_or_duty_relevance
  relationship_weight
  threat_or_opportunity
  provenance
```

Initial variable families:

```text
novelty / expectancy:
  whether the event is surprising or expected.

relevance:
  whether it matters to the holder's body, current activity, relationship,
  role, obligation, desire, fear, or need.

goal conduciveness:
  whether it helps, blocks, threatens, or changes a goal or pressure.

agency:
  who or what the holder believes caused the situation.

blame_or_credit:
  whether agency is praise-worthy, blame-worthy, accidental, unknown, or
  contested.

certainty:
  how confident the holder can be in the observation or interpretation.

urgency:
  whether delayed response matters.

coping_potential:
  whether the holder can act, endure, repair, escape, report, punish, hide, or
  seek help.

norm_compatibility:
  whether the focus fits or violates law, taboo, role, oath, permission,
  identity, taste, or sacred order.

relationship_weight:
  how strongly relevant relationships alter meaning.
```

Rules:

- Appraisal variables may be ephemeral or stored as explanation fragments
  inside accepted appraisal records.
- They should be typed enough for scoring and debug tools.
- They should not become a universal psychological model. Packs may add
  domain-specific variables behind checked vocabulary.

## AppraisalRule

`AppraisalRule` maps actor-relative context and appraisal variables to
candidate appraisal records.

In [Pack Authoring And Semantic Declarations](pack-authoring-and-semantic-declarations.md),
`AppraisalRule` is the `appraisal_rule` declaration kind inside the shared
semantic declaration framework.

Conceptual shape:

```text
AppraisalRule
  id
  focus_kind
  required_inputs
  variable_conditions
  output_templates
  conflict_policy
  provenance_policy
  authority_mode
```

Example pattern:

```text
if focus is ObservedEvent(ActorDied)
and holder has close relationship to victim
and believed cause_actor is known
then Thought(GriefAboutDeath)
then Pressure(Retaliate, target=cause_actor)
then GoalPressure(FindOrConfrontActor, target=cause_actor)
```

Rules:

- Rules should target reusable semantic patterns, not quest cases.
- Rules may be pack-owned when vocabulary is setting-specific.
- Rules may read only their declared inputs.
- Rules may emit appraisal candidates or accepted records through the
  appraisal commit gate.
- Rules may propose social or epistemic updates, but those proposals must go
  through their own gates.
- Rules must not select final `Intent`.

## Thought

`Thought` is actor-relative interpreted meaning.

Examples:

```text
Thought(GriefAboutDeath)
Thought(SawTabooViolation)
Thought(SuspectsTheft)
Thought(FeelsDebtToRescuer)
Thought(FearsFurtherAttack)
Thought(SeesChanceForEscape)
Thought(FeelsShame)
Thought(RecognizesOathConflict)
```

Conceptual shape:

```text
Thought
  id
  holder
  kind
  focus
  appraisal_variables_ref?
  source_records
  intensity
  salience
  confidence
  access
  lifecycle_state
  created_at
  expires_at?
  provenance
```

Rules:

- A thought is not hard truth.
- A thought is not the same as an `EpistemicRecord`.
- A thought may reference `EventRecord`, `ObservedEvent`, `EpistemicRecord`,
  `SocialClaim`, `SocialContextView`, prior appraisal records, or current
  `Activity`.
- A thought can affect pressure and future salience.
- A thought may propose an epistemic salience update, but it must not write
  `EpistemicStore` directly.
- A thought does not choose action.

## Pressure

`Pressure` is motivational force or action-readiness direction.

Examples:

```text
Pressure(Approach, target=mentor_body)
Pressure(Avoid, target=bandit_1)
Pressure(Retaliate, target=bandit_1)
Pressure(Report, target=town_guard)
Pressure(Conceal, target=stolen_relic)
Pressure(RepairViolation, target=shrine_order)
Pressure(SeekSafety)
Pressure(Protect, target=ally_1)
```

Conceptual shape:

```text
Pressure
  id
  holder
  direction
  target?
  topic?
  source_thoughts
  source_records
  intensity
  urgency
  persistence
  decay_policy
  suppression_policy
  conflict_links
  lifecycle_state
  provenance
```

Initial direction families:

```text
approach
avoid
protect
retaliate
repair
report
conceal
investigate
seek_help
seek_safety
restore_order
submit
defy
continue_current_activity
interrupt_current_activity
```

Rules:

- Pressure does not execute.
- Pressure does not select `Intent`.
- Pressure can bias `CandidateIntent` generation and `IntentScore`.
- Pressure can conflict with other pressure.
- Pressure should retain provenance for explanation.
- Pressure may be latent until reactivated by observation, recall, role, or
  social context.

## GoalPressure

`GoalPressure` is pressure shaped toward a possible desired state.

Examples:

```text
GoalPressure(FindKiller)
GoalPressure(AvoidPunishment)
GoalPressure(RestoreShrineOrder)
GoalPressure(RecoverFromWound)
GoalPressure(GetThroughLockedDoor)
GoalPressure(KeepOath)
GoalPressure(ProtectWitness)
```

Conceptual shape:

```text
GoalPressure
  id
  holder
  desired_state
  source_pressures
  target?
  constraints
  intensity
  urgency
  stability
  conflict_links
  provenance
```

Rules:

- `GoalPressure` is still not a commitment.
- It says which possible state is motivationally salient.
- It does not decide a method.
- It feeds `IntentTemplate` matching and `IntentScore`.
- A more stable `Goal` may exist elsewhere, such as actor policy, role duty,
  schedule, oath, player command, or pack-authored need. This layer may derive
  pressure from it, but it does not own all goal storage.

## Appraisal Record Lifecycle

Initial lifecycle states:

```text
candidate:
  produced by a rule or AI proposal before commit.

active:
  currently affects retrieval, intent generation, or scoring.

latent:
  stored but not currently active unless cued.

suppressed:
  overridden by stronger context, coping, command, or condition.

decayed:
  intensity or salience has fallen below normal behavioral relevance.

resolved:
  no longer active because the motivating condition was addressed.

invalidated:
  contradicted or superseded by later evidence or social context.
```

Rules:

- Lifecycle changes are appraisal records or accepted updates, not hard
  `EventRecord`s.
- If a lifecycle change has gameplay relevance, it should be provenance-backed.
- Decay can be time-based, cue-based, event-based, or activity-based.
- Suppression is not deletion. A suppressed pressure may reactivate.
- Resolution can be partial. One intent may reduce but not erase pressure.

## Conflict And Aggregation

Actors can hold conflicting appraisals.

Examples:

```text
Pressure(Retaliate, target=bandit_1)
Pressure(Avoid, target=bandit_1)
Pressure(CareForDead, target=mentor_1)
Pressure(ProtectSelf)
Pressure(Report, target=town_guard)
```

Rules:

- Appraisal does not need to collapse conflicts into one result.
- Conflicts should be explicit links, not hidden cancellation.
- Intent scoring decides which commitment is selected or suggested later.
- Some pressures may combine into stronger `GoalPressure`.
- Some pressures may suppress others temporarily.

## Commit Surface

Accepted appraisal outputs are gameplay-relevant records, but they are not hard
truth, social truth, or epistemic state.

[World Model](world-model.md) hosts:

```text
AppraisalRecordStore
```

Writes use the appraisal commit gate:

```text
AppraisalRule result / AI appraisal proposal
  -> appraisal commit gate
  -> AcceptedAppraisalRecord
  -> AppraisalRecordStore
```

Accepted appraisal record families:

```text
Thought
Pressure
GoalPressure
```

Accepted records may reference:

- `EventRecord`
- `ObservedEvent`
- `ObservedState`
- `EpistemicRecord`
- `SocialClaim`
- `SocialContextView`
- `CapabilitySet` explanation where coping potential depends on capability
- current `Intent` or `Activity` when relevance depends on ongoing work
- prior appraisal records

They may propose social or epistemic updates for appropriate later gates, but
they do not directly write `SocialInstitutionalStore` or `EpistemicStore`.

## AI Boundary

AI may participate in appraisal as proposal and explanation, not authority.

Allowed:

```text
AI appraisal proposal:
  "This guard may interpret the relic removal as taboo violation."

AI explanation:
  summarize provenance for debug or actor-facing narration.

AI vocabulary suggestion:
  propose pack-authored appraisal terms for later review.
```

Required path:

```text
AI proposal
  -> typed appraisal proposal
  -> scope / provenance / contradiction checks
  -> AcceptedAppraisalRecord or rejection
```

Forbidden:

- directly creating hard truth
- directly creating `EpistemicRecord`
- directly committing `SocialClaim`, norm, reputation, debt, or relationship
- selecting final `Intent`
- generating hidden actor knowledge
- using prose as the only gameplay-relevant record

## Relationship To Intent Planning

Intent planning consumes appraisal output.

```text
Thought / Pressure / GoalPressure
  -> IntentTemplate binding
  -> CandidateIntent
  -> IntentScore
  -> selected or suggested Intent
```

Rules:

- Appraisal may propose intent bias but not final intent.
- `Pressure` and `GoalPressure` should be typed enough for intent templates to
  match them.
- Appraisal records should not assume a specific method.
- The same `GoalPressure` can yield many candidate intents.

Example:

```text
GoalPressure(FindKiller, target=bandit_1)
  may support:
    LocateActor(bandit_1)
    TrackPhysicalTrace(source_event=mentor_death)
    AskInformationSource(topic=bandit_1)
    ReportToAuthority(event=mentor_death)
```

## Scenario Checks

### Mentor Killed By Bandit

```text
EventRecord:
  ActorDied(victim=mentor_1, cause_actor=bandit_1)

ObservedEvent:
  observer=player
  victim=mentor_1
  cause_actor=bandit_1
  confidence=high

EpistemicRecord:
  holder=player
  content=EventRecordRef(actor_died_456)
  mode=remembered
  confidence=high

SocialContextView:
  relationship(player, mentor_1).kind=mentor
  relationship(player, mentor_1).emotional_weight=high

AppraisalVariableSet:
  relevance=high
  relationship_weight=high
  agency=bandit_1
  goal_conduciveness=strongly_negative
  certainty=high
  coping_potential=uncertain

AcceptedAppraisalRecord:
  Thought(GriefAboutDeath)
  Pressure(Retaliate, target=bandit_1)
  GoalPressure(FindOrConfrontActor, target=bandit_1)
```

No scenario-specific pursuit action is created here. Intent planning later
chooses or suggests generic intent templates.

### Wounded Hand And Lockpick

```text
ObservedState:
  door has old lock

EpistemicWorkingSet:
  actor knows lockpicking procedure

Actor context:
  right hand wounded
  lockpick carried

AppraisalVariableSet:
  relevance=medium
  coping_potential=degraded
  urgency=depends_on_context

Possible appraisal:
  Thought(CanTryButHandIsUnreliable)
  Pressure(GetThroughLockedDoor)
  GoalPressure(OpenLockedDoor)
```

The wound affects capability, validation, and later score. Appraisal may create
meaning or pressure around risk and urgency, but it does not hardcode action
availability.

### Shrine Relic Removal

```text
EventRecord:
  ItemTransferred(shrine_relic, shrine_floor, actor_inventory)

SocialContextView for shrine_guard_1:
  SocialClaim(shrine_order owns shrine_relic)
  Norm(shrine forbids non-priest removal)
  role(shrine_guard_1, shrine_guard)

ObservedEvent:
  actor took shrine_relic

AppraisalVariableSet:
  norm_compatibility=violated
  role_or_duty_relevance=high
  agency=actor
  certainty=medium
  coping_potential=high

AcceptedAppraisalRecord:
  Thought(SawTabooViolation)
  Pressure(EnforceShrineLaw)
  GoalPressure(RestoreRelicToShrine)
```

The physical transfer, `SocialClaim`, actor belief, guard appraisal, and later
intent remain separate.

### Abstract Travel

```text
ObservedState:
  road dangerous

EpistemicWorkingSet:
  actor remembers market route

Pressure:
  ReachMarket

GoalPressure:
  TravelToMarket
```

Appraisal can create or refresh travel pressure. Intent planning and
multi-resolution simulation decide whether the selected intent becomes a local
`ActionRequest` sequence or an abstract `ProcessInstance`.

## Debug Explanation

Every accepted appraisal record should be explainable.

Explanation should include:

- focus record or observation
- holder
- input working set references
- social context references
- appraisal variables
- rule id or AI proposal id
- confidence and access mode
- resulting thought, pressure, or goal pressure
- conflict, suppression, decay, or reactivation links

Example:

```text
Why does shrine_guard_1 have Pressure(EnforceShrineLaw)?

because:
  observed actor took shrine_relic
  role_granted context says shrine_order owns shrine_relic
  role_granted norm forbids non-priest removal
  guard role gives high duty relevance
  AppraisalRule(taboo_violation_by_responsible_actor) matched
```

## Stable Decisions

- Appraisal turns actor-relative information into meaning and pressure.
- Appraisal does not choose actions.
- Memory and epistemic state do not choose actions directly.
- Social context does not choose actions directly.
- `AppraisalVariableSet` is the explainable feature layer between context and
  appraisal records.
- `Thought` is actor-relative interpreted meaning.
- `Pressure` is motivational force / action-readiness direction.
- `GoalPressure` is pressure shaped toward a possible desired state, not
  commitment.
- `AppraisalRecordStore` holds accepted `Thought`, `Pressure`, and
  `GoalPressure` records.
- Accepted appraisal records use `AcceptedAppraisalRecord`.
- Appraisal may propose social or epistemic updates, but cannot commit them.
- AI may propose appraisal output only through typed gates.
- Concrete appraisal vocabularies and rules may be pack-owned, but accepted
  outputs still use the appraisal commit surface.

## Deferred Decisions

- exact serialized record schema
- exact variable enum and extension mechanism
- exact pressure decay formulas
- exact conflict aggregation math
- first pack-owned appraisal vocabulary
- protagonist-specific pressure visibility and agency rules
- NPC autonomy policy for pressure-driven intent selection
- exact debug/explanation trace format
