# Intent Templates And Planning

## Status

Current design draft.

This document defines the target structure for intent templates, candidate
generation, intent scoring, commitment, activity, and resolution-aware
lowering. It is not an implementation schema or delivery plan.

## Source Research

- [Semantic Appraisal, Intent, Activity, And Planning](../research/semantic-appraisal-intent-activity-planning.md)
- [Actor Intent And Activity](../ideas/actor-intent-and-activity.md)
- [Actor Pressure And Interpretation](../ideas/actor-pressure-and-interpretation.md)

## Related Design Owners

- [Simulation Core](simulation-core.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
- [World Model](world-model.md)
- [Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md)
- [Perception And Observation](perception-and-observation.md)
- [Epistemic State](epistemic-state.md)
- [Social Institutional Model](social-institutional-model.md)
- [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md)
- [Multi-Resolution Simulation](multi-resolution-simulation.md)
- [Causal Runtime](causal-runtime.md)
- [Time Model](time-model.md)
- [Typed Effect Primitives](typed-effect-primitives.md)

## Purpose

Intent templates turn pressures, goals, capabilities, perceived affordances,
and actor-relative context into possible commitments.

They answer:

```text
Given this actor's pressure, goals, current context, capabilities, perceived
affordances, and active resolution, what could the actor commit to trying?
```

This layer exists because the engine needs a commitment boundary between
motivation and execution:

```text
Pressure / GoalPressure:
  motivational pull, not chosen method.

Intent:
  selected or suggested commitment to purpose and approach.

Activity:
  ongoing actor-facing execution frame over time.

ActionRequest:
  concrete actor-owned attempt now.

ProcessInstance:
  durable progress/execution frame, especially for abstract resolution.
```

The planning framework is a reusable mechanism. Concrete intent-template
libraries for combat, stealth, survival, magic, crafting, diplomacy, religion,
faction play, rituals, investigation, social repair, or a specific game's
social vocabulary may be defined by game-system packs.

## Chosen Structure

Use a hybrid intent architecture:

```text
Pressure, GoalPressure, and stable Goal
  + CapabilitySet / ActionRepertoire
  + PerceivedAffordance
  + EpistemicWorkingSet
  + SocialContextView
  + active Intent / Activity / ProcessInstance
  + active resolution
  + IntentTemplate definitions
  -> CandidateIntent
  -> IntentScore
  -> selected or suggested Intent
  -> Activity
  -> resolution-aware lowering
      concrete target: ActionRequest or ProcessInstance
      abstract target: ProcessInstance
      strategic: usually region/faction/process pressure, not individual intent
```

The core separation:

```text
CandidateIntent:
  a possible commitment generated from a reusable template and current binding.

IntentScore:
  explainable ranking over candidates.

Intent:
  selected or suggested commitment.

Plan:
  decomposed structure for carrying out an intent, often transient.

Activity:
  ongoing actor-facing execution frame.
```

## Compiler Placement

In the [Simulation Transition Compiler](simulation-transition-compiler.md),
intent templates and planning form the second half of the decision middle-end.
This layer consumes appraisal output and actor-relative context, then prepares
the handoff to executable representations. It does not replace the overall
compiler ladder.

Folded view:

```text
Thought / Pressure / GoalPressure
  + DerivedContext
  -> CandidateIntent
  -> IntentScore
  -> selected or suggested Intent
  -> Activity / lowering target preparation
```

Pass breakdown:

```text
pass:
  IntentCandidateGeneration

pass class:
  Choice

transformation kind:
  Generation / rule matching

input representations:
  Pressure
  GoalPressure
  Goal
  CapabilitySet
  ActionRepertoire
  PerceivedAffordance
  EpistemicWorkingSet
  SocialContextView
  IntentTemplate definitions

output representation:
  CandidateIntent
```

```text
pass:
  IntentScoringSelection

pass class:
  Choice

transformation kind:
  Ranking / selection

input representation:
  CandidateIntent set

output representations:
  IntentScore
  selected or suggested Intent
```

```text
pass:
  ActivityPreparation

pass class:
  Translation

transformation kind:
  Activity preparation / lowering target selection

input representations:
  Intent
  active resolution
  current Activity / ProcessInstance state

output representations:
  Activity
  lowering target decision
```

This is where complexity is managed. The engine may implement these passes
together at first, but the logical stages must remain separate enough for
debugging, explanation, AI proposal gates, and resolution-aware lowering.

Intent planning may choose a commitment. It still must not execute hard effects
or mutate truth:

```text
Intent -> Activity -> ActionRequest / ProcessInstance
```

not:

```text
Intent -> hard mutation
CandidateIntent -> CausalTransaction
Pressure -> hidden action sequence
```

## Alternatives Considered

### Story-Specific Intent Enumeration

Rejected.

```text
ScenarioSpecificPursuit
ScenarioSpecificWitnessQuestion
ScenarioSpecificRevenge
ScenarioSpecificRelicQuest
```

This produces brittle scenario code. The engine should instead combine generic
templates with actor-relative bindings:

```text
LocateActor(target)
AskInformationSource(topic, source)
TrackPhysicalTrace(target_or_event)
ApproachActor(target)
ConfrontActor(target, reason)
CareForDead(subject)
ReportToAuthority(event_or_suspect)
RecoverObject(object, claimant?)
```

### GOAP-Only Planner

Rejected as the full architecture.

GOAP is useful for decoupling goals and actions, but `world` also needs
appraisal provenance, actor-relative ignorance, social context, abstract
resolution, long-running process state, and causal transaction boundaries.
GOAP-like search can be one template or plan-construction method, not the
owner of cognition and execution.

### HTN-Only Planner

Rejected as the full architecture.

HTN decomposition is useful after an intent is selected, especially for
multi-step activities. It should not replace appraisal, pressure, scoring,
player agency, or causal runtime validation.

### Behavior-Tree-Only Controller

Rejected as the top-level model.

Behavior trees are useful inside process definitions or local NPC policies.
They are weaker for explaining why a commitment exists because the reason is
often hidden in blackboard state unless external provenance is preserved.

### Utility-Only Action Selector

Rejected as the full architecture.

Utility scoring is ideal for `IntentScore`, but direct scoring of atomic
actions loses commitment, activity continuity, process state, and
multi-resolution lowering.

### BDI Black Box

Rejected.

BDI provides the right separation pressure: belief, goal/desire, and
intention. But a single BDI interpreter would blur `EpistemicRecord`,
`GoalPressure`, `Intent`, `ProcessInstance`, and `ActionRequest` authority.
`world` adapts the separation while keeping each engine layer explicit.

### LLM Planner As Authority

Rejected.

LLM output may propose intent, select among actor-facing candidates in a policy
role, or explain a trace. It must not directly mutate hard truth, create
memory, commit social state, or install final intent outside the intent gate.

## Position In The Engine

Intent planning sits after semantic appraisal and before resolution lowering.

```text
ObservedState / ObservedEvent
  -> EpistemicWorkingSet
  -> SocialContextView
  -> Thought / Pressure / GoalPressure
  -> Intent Templates And Planning
  -> CandidateIntent
  -> IntentScore
  -> selected or suggested Intent
  -> Activity
  -> resolution-aware lowering
      concrete target: ActionRequest or ProcessInstance
      abstract target: ProcessInstance
  -> Causal Runtime
```

Intent planning is a choice and translation preparation layer. It does not run
effects and does not commit hard state.

## Boundary

This layer owns:

- durable and transient use of `Goal` as intent input
- `IntentTemplate`
- candidate intent generation
- binding templates to perceived targets, traces, places, people, tools,
  topics, authorities, and processes
- `CandidateIntent`
- candidate feasibility status and actor-visible unavailability
- `IntentScore`
- selected or suggested `Intent`
- `Plan` as decomposed intent structure
- `Activity` as ongoing execution frame
- resolution-aware lowering contracts
- intent proposal and selection gates
- player suggestion policy and NPC selection policy
- debug explanation for why an intent was considered, rejected, selected, or
  replaced

This layer does not own:

- semantic appraisal itself
- `Thought`, `Pressure`, or `GoalPressure` creation
- hard world mutation
- `EventRecord` emission
- `Typed Effect Program` execution
- action validation
- `ProcessTick` implementation
- `EpistemicRecord` storage
- social or institutional state commit
- perception projection
- final UI phrasing

## Core Principle

Intent is the commitment boundary.

Before intent, the actor can hold many competing pressures, goals, candidates,
and scores. After intent, the engine can explain what purpose is being pursued,
why future activity belongs together, and how concrete or abstract execution
should proceed.

Intent still does not mutate truth:

```text
Intent
  -> Activity
  -> resolution-aware lowering
  -> ActionRequest or ProcessInstance
  -> Typed Effect Program
  -> CausalTransaction
  -> EventRecord
```

## Goal

`Goal` is a stable objective, duty, preference, need, command, schedule item,
or authored standing aim.

Examples:

```text
survive
protect mentor
reach market
keep oath to village
preserve shrine taboo
guard gate
recover from wound
learn ritual language
```

Rules:

- A goal is more stable than pressure.
- A goal is not necessarily a chosen method.
- This document owns how goals feed candidate generation and scoring.
- The source of a goal may be actor policy, social obligation, schedule,
  player command, pack-authored need, current process, or scenario state.
- If a goal is game-relevant durable state, its storage must belong to the
  appropriate owner. This document does not create a universal goal store.
- A goal can create or shape `GoalPressure`, but `GoalPressure` can also arise
  directly from appraisal.

## IntentTemplate

`IntentTemplate` is a reusable commitment pattern.

In [Pack Authoring And Semantic Declarations](pack-authoring-and-semantic-declarations.md),
`IntentTemplate` is the `intent_template` declaration kind inside the shared
semantic declaration framework.

Conceptual shape:

```text
IntentTemplate
  id
  purpose_kind
  input_pressure_kinds
  supported_goal_shapes
  binding_roles
  required_capability_queries
  required_affordance_queries
  required_epistemic_queries
  required_social_context_queries
  candidate_generation_rules
  scoring_features
  supported_resolutions
  lowering_contract
  plan_methods?
  activity_policy?
  debug_explanation_policy
```

Examples:

```text
LocateActor(target)
AskInformationSource(topic, source)
TrackPhysicalTrace(target_or_event)
ApproachActor(target)
ConfrontActor(target, reason)
CareForDead(subject)
ReportToAuthority(event_or_suspect)
RecoverObject(object, claimant?)
FleeThreat(threat)
TravelToPlace(place)
RecoverFromCondition(condition)
PrepareRitual(ritual, site, offering?)
```

Rules:

- Templates are generic and reusable.
- Templates may be pack-owned when vocabulary is specific.
- Templates bind to actor-relative context, not omniscient state.
- Templates produce candidates, not direct mutation.
- Templates must declare supported resolutions.
- Templates must not bypass capability, affordance, action, process, or
  transaction boundaries.

## Binding Inputs

Candidate generation binds templates to the actor's current view.

Allowed binding sources:

```text
Pressure / GoalPressure:
  why this kind of commitment is motivationally relevant.

Goal:
  stable objective or standing duty.

CapabilitySet:
  what the actor can do or how attempts are degraded.

ActionRepertoire:
  actor-owned action schemas available in principle.

PerceivedAffordance:
  actor-relative target/context binding possibilities.

EpistemicWorkingSet:
  remembered, believed, known, rumored, or secret information available to the
  actor.

SocialContextView:
  holder-known, role-granted, or authoritative social context allowed by the
  current mode.

Active Intent / Activity / ProcessInstance:
  current commitment and interruption cost.

Resolution:
  concrete, abstract, or strategic execution target.
```

Rules:

- Binding must preserve uncertainty.
- A rumored target may produce a speculative candidate.
- A hidden target cannot be bound as known unless actor-relative context
  exposes it.
- External objects and places provide perceived affordances and bindings, not
  actor-owned action repertoire.
- Actor-owned capability can degrade, enable, or change score and lowering.

## CandidateIntent

`CandidateIntent` is a possible commitment generated from a template and
current bindings.

Conceptual shape:

```text
CandidateIntent
  id
  actor
  template_id
  purpose
  desired_state?
  bindings
  source_pressures
  source_goals
  required_capabilities
  perceived_affordances
  social_context_refs
  epistemic_refs
  supported_resolutions
  feasibility_status
  risk_summary
  expected_activity_shape
  provenance
```

Examples:

```text
CandidateIntent(TrackPhysicalTrace, trace=blood_trail_1)
CandidateIntent(AskInformationSource, topic=bandit_1, source=villager_1)
CandidateIntent(ReportToAuthority, event=mentor_death, authority=town_guard)
CandidateIntent(RecoverObject, object=shrine_relic, claimant=shrine_order)
CandidateIntent(FleeThreat, threat=bandit_1)
```

Rules:

- Candidate intent is not final intent.
- Candidate intent is not a plan.
- Candidate intent is not an action.
- Candidate intent may be unavailable but still explainable.
- Candidate generation should prefer many weak candidates with clear reasons
  over one opaque choice.

## Candidate Feasibility

Feasibility should be explicit.

Initial statuses:

```text
available:
  can be selected or suggested now.

speculative:
  based on rumor, uncertainty, hidden target, incomplete location, or inferred
  affordance.

blocked:
  known current condition prevents execution, but the purpose is coherent.

unavailable:
  actor lacks required capability, authority, knowledge, tool, target, or
  context.

impossible:
  template cannot apply under current known rules.

unsafe:
  possible, but risk exceeds the current policy threshold.

deferred:
  possible later, but current activity, schedule, or resolution makes it poor
  now.
```

Rules:

- Actor-facing feedback must not leak hidden truth.
- `unavailable` and `blocked` can still be useful for debug and AI recovery.
- A candidate can be ranked low without being invalid.
- Runtime validation can still fail later because hard truth may differ from
  actor-relative belief.

## IntentScore

`IntentScore` is an explainable ranking result.

Conceptual shape:

```text
IntentScore
  candidate_id
  total
  feature_scores
  blockers
  risks
  uncertainty
  selection_policy
  explanation
```

Initial feature families:

```text
pressure_fit:
  how well the candidate addresses active pressure.

goal_fit:
  how well it advances stable goals or duties.

capability_fit:
  whether the actor can plausibly execute it.

affordance_fit:
  whether perceived targets and contexts support it.

knowledge_fit:
  whether the actor knows enough to pursue it.

social_fit:
  whether role, law, taboo, debt, rank, oath, or reputation supports it.

risk:
  physical, social, epistemic, time, resource, and interruption risk.

urgency:
  whether delay matters.

commitment_cost:
  cost of abandoning or interrupting current intent/activity.

resolution_fit:
  whether the intent has concrete, abstract, or strategic lowering support.

protagonist_policy:
  whether to suggest, auto-continue, block automation, or require player
  confirmation.
```

Rules:

- Numeric scoring is allowed, but final explanation must keep feature
  contributions.
- Scores rank candidates. They do not execute.
- Ties and near-ties should remain visible to debug tools.
- NPC policy may sample, threshold, or choose best candidate. Player-facing
  policy usually suggests rather than forces.

## Intent

`Intent` is the selected or suggested commitment to a purpose and approach.

Conceptual shape:

```text
Intent
  id
  actor
  purpose
  approach
  source_candidate
  source_score
  source_pressures
  source_goals
  bindings
  commitment_strength
  selected_by
  supported_resolutions
  lowering_contract
  interrupt_policy
  reconsideration_policy
  created_at
  lifecycle_state
  provenance
```

Initial lifecycle states:

```text
suggested:
  shown or proposed, not committed.

selected:
  accepted as current commitment.

active:
  currently driving activity or lowering.

suspended:
  paused because of interruption, wait condition, or temporary override.

completed:
  purpose satisfied.

failed:
  purpose cannot continue under current conditions.

abandoned:
  actor or policy intentionally dropped it.

replaced:
  superseded by another intent.
```

Rules:

- Intent owns purpose and approach.
- Intent does not mutate hard truth.
- Intent can be selected by player, NPC policy, AI-controlled actor policy,
  schedule, reaction, or process continuation.
- Intent must carry enough provenance to explain why it exists.
- Durable active intent is runtime decision/control state. It may be stored
  with actor control state or attached to an active `ProcessInstance`; exact
  physical storage is a world-model detail.
- If an intent is selected by AI, the selection must pass through the actor
  policy or intent proposal gate.

## Intent Selection Gate

Any final or durable intent choice must pass through an explicit gate.

Inputs:

```text
player selection
NPC policy selection
AI-controlled actor policy output
schedule or duty policy
reaction policy
process continuation policy
```

Gate checks:

- actor identity and control authority
- candidate provenance
- actor-facing access boundaries
- current view version where relevant
- selected candidate feasibility
- protagonist agency policy
- AI proposal scope and provenance
- conflict with current `Activity` or `ProcessInstance`

Outputs:

```text
selected Intent
suggested Intent
rejected selection with explanation
continue current Activity
interrupt current Activity request
```

Rules:

- The gate does not validate hard physical success.
- It only accepts a commitment or suggestion as decision/control state.
- Hard effects still require later `ActionRequest` or `ProcessTick`
  validation.

## Plan

`Plan` is a decomposed structure for carrying out an `Intent`.

It may be transient:

```text
Intent(FindBanditByTracking)
  -> Plan:
      inspect visible traces
      follow strongest trace
      ask witness if trail lost
      confront target if found
```

It may be implicit in a `ProcessDef`:

```text
ProcessDef(TrackActor)
  abstract_tick:
    advance route hypothesis
    check trail freshness
    update risk

  concrete_tick:
    inspect trace
    choose next local move or wait
```

Rules:

- Plan is not automatically durable state.
- Plan can be rebuilt, revised, or discarded if provenance and active
  commitment are preserved.
- Durable progress belongs to `Activity` and `ProcessInstance`.
- Decomposition methods may use GOAP, HTN, behavior tree fragments, hand
  rules, or pack-authored process methods.
- Plans must lower to `ActionRequest` or `ProcessInstance`, not hard mutation.

## Activity

`Activity` is the ongoing actor-facing execution frame.

Examples:

```text
tracking the bandit
searching the room
recovering from wound
preparing ritual
traveling to market
guarding shrine door
interrogating witness
crafting sword
```

Conceptual shape:

```text
Activity
  id
  actor
  source_intent
  kind
  actor_facing_meaning
  active_process_instances
  current_plan?
  progress_summary
  interrupt_policy
  resume_policy
  completion_condition
  visible_tells
  lifecycle_state
  provenance
```

Rules:

- Activity explains what the actor is doing over time.
- Activity can be a view over selected `Intent` plus one or more
  `ProcessInstance`s.
- Activity may be persisted when it must survive save/load, interruption, or
  resolution transition.
- Activity can produce local `ActionRequest`s when concrete.
- Activity can be backed by `ProcessInstance` progress when abstract.
- Activity does not directly mutate truth.

## Resolution-Aware Lowering

Intent lowering depends on active resolution.

```text
Concrete:
  Intent -> Activity
         -> ActionRequest for discrete attempts
         -> ProcessInstance for long-running work
         -> ProcessTick for continuation

Abstract:
  Intent -> Activity
         -> ProcessInstance
         -> ProcessTick at abstract resolution

Strategic:
  individual Intent usually inactive by default
  region / faction / institution processes or pressure dominate
```

Rules:

- Concrete lowering may produce `ActionRequest` or `ProcessInstance`.
- Abstract lowering produces `ProcessInstance`, not hidden repeated concrete
  `ActionRequest`s.
- If an intent has no valid lowering for the current resolution, it may be
  blocked, deferred, or require promotion.
- Multi-resolution simulation owns promotion, demotion, and active resolution
  policy.
- Causal runtime owns `ActionRequest` and `ProcessTick` validation and hard
  commit.

## Protagonist And NPC Policy

The same candidate and scoring system can serve protagonist, NPC, and AI
agents, but selection policy differs.

### Protagonist

For the protagonist, the default is suggestion and explicit commitment.

Allowed:

- show likely intents
- let the player select an intent
- let the player submit direct `ActionRequest`
- continue explicit player-started `Activity`
- interrupt automation on actor-visible danger, failure, new evidence, or
  player input

Avoid:

- forcing major protagonist decisions from pressure alone
- continuing automation through actor-visible danger
- hiding why an intent is suggested

### NPC

NPC policy may select intents autonomously.

Inputs:

- active pressures and goals
- schedule and duty
- personality or pack-defined policy
- accessible observations and working set
- current activity cost
- candidate scores

Rules:

- NPC selection still uses actor-relative context.
- NPC selection still passes through the intent selection gate.
- NPCs may keep low-detail policies at abstract or strategic resolution.
- NPC policy must not bypass causal runtime.

### AI-Controlled Actor

An AI-controlled actor may choose among actor-facing candidates or propose an
intent.

Allowed output:

```text
AgentTurnOutput:
  SelectIntent(intent_candidate_id)
  SubmitActionRequest(action_request)
  ContinueActivity(activity_id)
  InterruptActivity(activity_id, reason)
  Wait
```

Rules:

- AI receives actor-facing context, not omniscient hard truth.
- AI intent choice must reference candidates or submit a typed proposal.
- AI cannot create final intent outside the gate.
- AI cannot mutate hard truth, memory, social truth, or appraisal truth.

## Commit And Storage Semantics

Intent and activity are decision/control state, not hard physical truth.
Durable decision/control changes are accepted through the runtime-control gate,
not by directly mutating `RuntimeControlStore`.

Rules:

- Suggested intent may be transient.
- Selected intent should be durable when it affects future simulation,
  abstract execution, save/load, replay, or explanation. Durable selection
  becomes an `AcceptedRuntimeControlUpdate`.
- Active activity should be durable when it has progress, reservations,
  process identity, or interruption/resume behavior. Durable activity changes
  become `ActivityTransition`s accepted as runtime-control updates.
- `ProcessInstance` remains the durable execution/progress frame owned by the
  process runtime.
- The world model owns physical storage. This document owns intent/activity
  meaning and lifecycle.
- Intent selection is not an `EventRecord` unless a later action/process commit
  emits hard evidence. Debug logs may record selection traces separately.

## Relationship To Appraisal

Appraisal produces meaning and pressure. Intent planning turns pressure into
possible commitments.

```text
Thought:
  "This was desecration."

Pressure:
  "Respond to shrine-law violation."

GoalPressure:
  "Restore relic to shrine."

CandidateIntent:
  WarnActor, DetainActor, ReportToPriest, RecoverObject.

Intent:
  DetainActorForShrineViolation.
```

Rules:

- A `Thought` can explain candidate generation, but it is not a method.
- `Pressure` and `GoalPressure` bias candidates and scores.
- Appraisal cannot select final `Intent`.
- Intent planning cannot create `Thought`, `Pressure`, or `GoalPressure`.

## Relationship To Capability And Affordance

Actor-owned capability and observed affordance shape candidates.

```text
CapabilitySet:
  why this actor can attempt a schema or has degraded performance.

ActionRepertoire:
  actor-owned action schemas available in principle.

PerceivedAffordance:
  what observed targets appear to support.

CandidateIntent:
  binds purpose to actor-owned schema and perceived target/context.
```

Rules:

- External objects do not create actor-owned action repertoire.
- Missing or degraded capability affects feasibility and score.
- Runtime validation still checks hard truth later.

## Scenario Checks

### Mentor Killed By Bandit

Inputs:

```text
GoalPressure(FindOrConfrontActor, target=bandit_1)
Pressure(Retaliate, target=bandit_1)
CapabilitySet(player)
ActionRepertoire(player)
PerceivedAffordance(visible_tracks)
EpistemicWorkingSet(player remembers bandit_1)
SocialContextView(mentor relationship)
```

Candidates:

```text
CandidateIntent(TrackPhysicalTrace, trace=visible_tracks)
CandidateIntent(AskInformationSource, topic=bandit_1, source=villager_1)
CandidateIntent(ReportToAuthority, event=mentor_death)
CandidateIntent(CareForDead, subject=mentor_1)
CandidateIntent(FleeThreat, threat=bandit_1)
```

Selection:

```text
Intent(FindBanditByTracking)
  source_pressure=Retaliate
  approach=TrackPhysicalTrace
  selected_by=player_or_policy
```

Lowering:

```text
concrete:
  Activity(TrackingBandit)
  -> ActionRequest(Inspect(tracks))
  -> ActionRequest(MoveAlongTrace)

abstract:
  Activity(TrackingBandit)
  -> ProcessInstance(TrackActor, active_resolution=abstract)
```

### Wounded Hand And Lockpick

Inputs:

```text
GoalPressure(GetThroughLockedDoor)
CapabilitySet(fine_manipulation=degraded)
ActionRepertoire(ApplyTool)
PerceivedAffordance(door.lock pickable)
EpistemicWorkingSet(actor knows lockpicking)
```

Candidate:

```text
CandidateIntent(OpenLockedDoorByPicking)
```

Score effects:

```text
capability_fit: reduced
risk: increased
time_cost: increased
noise_risk: maybe increased
```

Lowering:

```text
Intent(OpenLockedDoorByPicking)
  -> ActionRequest(ApplyTool(lockpick, door.lock, pick))
  -> Causal Runtime validation
```

The wound does not require a special action list. It affects capability,
scoring, validation, duration, and risk.

### Shrine Relic Removal

Inputs:

```text
Thought(SawTabooViolation)
Pressure(EnforceShrineLaw)
GoalPressure(RestoreRelicToShrine)
SocialClaim(shrine_order owns shrine_relic)
PerceivedAffordance(actor holding shrine_relic)
```

Candidates:

```text
CandidateIntent(WarnActor)
CandidateIntent(DetainActor)
CandidateIntent(ReportToPriest)
CandidateIntent(RecoverObject, object=shrine_relic)
CandidateIntent(AskForPermissionEvidence)
```

Intent planning does not decide whether the physical transfer was hard truth
or whether the social claim is valid. It uses actor-accessible context to
generate and rank commitments.

### Abstract Travel

Inputs:

```text
Goal(ReachMarket)
GoalPressure(TravelToMarket)
CapabilitySet(can_walk_or_ride)
EpistemicWorkingSet(known_route_to_market)
active_resolution=abstract
```

Candidate:

```text
CandidateIntent(TravelToPlace, place=market)
```

Lowering:

```text
Intent(TravelToMarket)
  -> Activity(TravelingToMarket)
  -> ProcessInstance(TravelToMarket, active_resolution=abstract)
  -> ProcessTick
  -> CausalTransaction
  -> EventRecord(RouteProgressed)
```

No repeated hidden concrete `Move` requests are generated.

## Debug Explanation

Intent planning should explain:

- which pressures and goals produced candidates
- which templates matched
- which bindings were used
- why candidates were unavailable, blocked, unsafe, or speculative
- which features contributed to `IntentScore`
- who or what selected the final `Intent`
- why current `Activity` continued, paused, failed, or was interrupted
- how resolution lowering chose `ActionRequest` or `ProcessInstance`

Example:

```text
Why was CandidateIntent(TrackPhysicalTrace) selected?

because:
  GoalPressure(FindOrConfrontActor) was high
  visible_tracks provided PerceivedAffordance(trackable)
  actor has tracking procedure knowledge
  AskInformationSource had no known source nearby
  ReportToAuthority had lower urgency fit
  current resolution supports concrete tracking
```

## Stable Decisions

- Intent is the commitment boundary.
- Activity is the temporal execution boundary.
- `ActionRequest` is the concrete actor-owned attempt boundary.
- `ProcessInstance` is the durable execution/progress frame.
- Intent templates generate generic candidates, not story-specific commands.
- Candidate generation uses actor-relative context and preserves uncertainty.
- `IntentScore` is explainable and feature-based.
- Concrete intent may lower to `ActionRequest` or `ProcessInstance`.
- Abstract intent lowers to `ProcessInstance`.
- Abstract execution must not synthesize hidden concrete action spam.
- AI may choose or propose intent only through actor-facing policy or typed
  proposal gates.
- Intent and activity do not mutate hard truth.

## Deferred Decisions

- exact serialized `IntentTemplate` format
- exact binding query language
- exact score aggregation math
- exact storage layout for active intent and activity state
- exact player suggestion UI policy
- exact NPC autonomy policy
- first pack-owned intent-template libraries
- detailed plan-repair and reconsideration policy
- exact debug/explanation trace format
