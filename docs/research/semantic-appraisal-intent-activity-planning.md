# Semantic Appraisal, Intent, Activity, And Planning

## Status

Draft research.

## Design Outputs

This research should inform later expansion of:

- [Semantic Appraisal And Motivation](../design/semantic-appraisal-and-motivation.md)
- [Intent Templates And Planning](../design/intent-templates-and-planning.md)

It should not be treated as an implementation plan, schema, test plan, or
vertical-slice proposal.

## Core Question

```text
How should actor-relative meaning become motivational pressure, and how should
that pressure become an explainable commitment to activity without bypassing
the action, process, transaction, and event boundaries?
```

The design needs enough cognitive structure to make behavior intelligible, but
not a monolithic mind simulation. It needs enough planning structure to support
multi-turn purpose, but not one planner that owns emotion, memory, social
truth, execution, and mutation.

## Current Engine Constraints

The existing design already settles the most important boundaries:

```text
hard truth / EventRecord
  -> ObservedState / ObservedEvent
  -> EpistemicRecord / EpistemicWorkingSet
  -> SocialContextView
  -> semantic appraisal
  -> Thought / Pressure / GoalPressure
  -> CandidateIntent
  -> IntentScore
  -> selected or suggested Intent
  -> Activity
  -> resolution-aware lowering
      concrete target: ActionRequest or ProcessInstance
      abstract target: ProcessInstance
  -> Typed Effect Program
  -> CausalTransaction
  -> EventRecord
```

The strict rules:

- Appraisal does not choose actions.
- Memory and epistemic state do not choose actions directly.
- Social context does not choose actions directly.
- Pressure biases later intent selection, but does not execute anything.
- Intent is the commitment boundary.
- Activity is the temporal execution boundary.
- `ActionRequest` is the actor-facing concrete attempt boundary.
- Concrete simulation lowers selected intent toward `ActionRequest` and/or
  `ProcessInstance`.
- Abstract simulation lowers selected intent toward `ProcessInstance`, not
  repeated hidden concrete action requests.
- State mutation still goes through `Typed Effect Program`,
  `CausalTransaction`, and `EventRecord` boundaries.
- AI may propose appraisal, intent, or planning outputs only through typed
  gates. It must not directly mutate hard truth, memory, social truth, or final
  intent.

## Research Inputs

### Cognitive Appraisal Theory

The useful lesson from appraisal theory is that meaning is not a direct
property of an event. Meaning is produced by evaluating an event, action, or
object relative to an actor's concerns, beliefs, relationships, standards,
coping potential, and social context.

[OCC](https://www.cambridge.org/core/books/cognitive-structure-of-emotions/cognitive-structure-of-emotions/D16784CA50F5BBD58F3140B575BB7881)
is valuable because it separates emotional construal around events, actions of
agents, and objects. For `world`, this maps cleanly onto:

```text
EventRecord / ObservedEvent:
  what happened or appeared to happen.

Agent-responsibility appraisal:
  who is believed responsible, praiseworthy, blameworthy, intentional, careless,
  helpful, or hostile.

Object / entity appraisal:
  what an actor finds appealing, sacred, taboo, dangerous, beloved, disgusting,
  useful, or suspicious.
```

OCC should not be copied as the emotion taxonomy. Its transferable pressure is
that appraisal variables should stay explicit and inspectable instead of
collapsing into one mood number.

[Scherer's Component Process Model](https://journals.sagepub.com/doi/10.1177/0539018405058216)
is useful because it treats emotion as a multi-component process, not just a
label. Its appraisal checks point toward a reusable `AppraisalVariableSet`:

```text
novelty / expectancy:
  did this surprise the actor?

relevance:
  does it matter to the actor's concerns, body, role, duty, relationship, or
  current activity?

goal conduciveness:
  does it help or obstruct something the actor cares about?

agency / responsibility:
  who or what caused it, and can blame or credit be assigned?

coping / control:
  can the actor respond, endure, repair, escape, punish, hide, or ask for help?

norm and value compatibility:
  does it violate law, taboo, role, oath, identity, taste, or sacred order?
```

This suggests that appraisal output should retain features, not only a
selected thought. A guard seeing shrine relic removal may have high norm
violation, high role relevance, medium uncertainty about permission, and high
coping potential. Those variables explain why the resulting pressure is to
warn, detain, report, or investigate.

[Lazarus and Smith's core relational theme work](https://www.tandfonline.com/doi/abs/10.1080/02699939308409189)
and Lazarus's later stress-to-emotion review emphasize relational meaning:
emotion depends on the actor-environment relationship, not stimulus alone.
For `world`, this supports treating `Thought` as actor-relative interpreted
meaning:

```text
same EventRecord:
  mentor killed by bandit

player:
  loss, grief, retaliation pressure

bandit's ally:
  victory, relief, caution

town guard:
  crime, duty, public-safety pressure

cowardly witness:
  fear, avoid-testimony pressure
```

[Frijda's action-readiness work](https://mitpressbookstore.mit.edu/book/9780521316002)
is the strongest justification for a distinct `Pressure` layer. Appraisal does
not need to issue an action. It can change the actor's readiness toward
approach, avoidance, attack, submission, repair, concealment, disclosure,
seeking help, or ritual response. `Pressure` is therefore motivational force,
not executable command.

[EMA](https://www.sciencedirect.com/science/article/abs/pii/S1389041708000314)
is the most directly relevant computational appraisal reference. It models
appraisal dynamics over an explicit interpretation of the agent-environment
relationship and connects appraisal to coping and later behavior. The useful
transfer is computational discipline: appraisals should operate over typed
state, preferences, events, actions, and causal interpretations. The important
adaptation is architectural: in `world`, EMA-like appraisal may propose
`Thought`, `Pressure`, and `GoalPressure`, but intent selection and execution
remain separate gates.

### Agent And Planning Theory

[Bratman's planning theory of intention](https://press.uchicago.edu/ucp/books/book/distributed/I/bo3629095.html)
is the best fit for the `Intent` boundary. Intentions are not merely desires;
they are elements of partial plans that organize activity over time and support
further practical reasoning. This maps directly to:

```text
GoalPressure:
  "I feel pulled toward finding the killer."

CandidateIntent:
  "I could follow tracks, ask witnesses, report to the guard, bury the mentor,
  or flee."

Intent:
  "I commit for now to finding the killer by following the tracks."

Activity / ProcessInstance:
  "I am currently tracking across time, with progress, interruptions, and
  execution state."
```

This is why `Intent` should be the commitment boundary. Before intent, the
system can hold competing pressures and candidates. After intent, downstream
systems can explain what the actor is trying to do and why a plan or process is
being continued.

[BDI agent theory](https://cdn.aaai.org/ICMAS/1995/ICMAS95-042.pdf) is useful
as a separation of roles:

```text
belief:
  actor-relative information and working set.

desire / goal pressure:
  motivating possible outcomes.

intention:
  selected commitment that constrains later behavior.
```

The design should adapt the separation, not import a single BDI black box.
`EpistemicRecord` remains a store and retrieval layer, not an action selector.
`GoalPressure` remains motivational pressure, not chosen method. `Intent`
is selected through the intent layer and then lowered through resolution and
runtime gates.

[PRS](https://aaai.org/Papers/AAAI/1987/AAAI87-121.pdf) is useful because it
combines beliefs, goals, and procedural knowledge in dynamic environments. Its
"knowledge areas" resemble reusable methods for achieving classes of goals.
For `world`, the corresponding concept should be pack-authored
`IntentTemplate` and `ProcessDef` families, not arbitrary procedure callbacks
that mutate the world.

[AgentSpeak(L)](https://dblp.org/rec/conf/maamaw/Rao96.html) is useful mainly
as a formalized BDI language reference. It reinforces the idea that triggering
events, beliefs, goals, and plans can be represented explicitly. It should not
drive the core architecture by itself, because `world` needs stronger
truth-authority boundaries, resolution-aware lowering, and hard mutation
discipline than ordinary agent programming examples usually model.

### Game AI

Game AI contributes practical decomposition tools, but none should own the full
cognition-to-mutation chain.

[GOAP in F.E.A.R.](https://www.madwomb.com/tutorials/gamedesign/prototyping/gdc2006_JeffOrkin_AI_FEAR.pdf)
is useful because it decouples goals from actions and can re-plan when the
world changes. It supports the `IntentTemplate` idea:

```text
generic template:
  LocateActor(target)
  AskInformationSource(topic, source)
  TrackPhysicalTrace(target_or_event)
  ConfrontActor(target, reason)

not story script:
  PursueSpecificBanditBecauseSpecificMentorDied
```

GOAP is not sufficient alone. It tends to plan action sequences toward a goal,
while `world` also needs appraisal provenance, actor-relative ignorance,
social context, process execution, and abstract resolution.

[HTN planning](https://cir.nii.ac.jp/crid/1360298343490834816?lang=en)
is useful for decomposing a selected purpose into methods and substeps. It maps
well to transient `Plan` construction and authored `ProcessDef` methods:

```text
Intent(RecoverFromWound)
  -> method: find safe place
  -> method: clean wound
  -> method: apply bandage
  -> method: rest until condition improves
```

The transfer is hierarchical decomposition. The rejection is allowing HTN to
become a hidden mutation authority. Decomposition still lowers to
`ActionRequest` and/or `ProcessInstance`.

[Utility AI](https://www.gdcvault.com/play/1015683/Embracing-the-Dark-Art-of)
is useful for `IntentScore`. Candidate intents often need explainable ranking
over heterogeneous features:

```text
pressure intensity
goal relevance
capability fit
known path or target
risk
time cost
social duty
relationship weight
personality or value fit
current activity interruption cost
resolution support
```

Utility-style scoring is a good local ranking method, not an ontology and not
a direct action path. A high score selects or suggests an intent only through
the intent gate.

[Behavior Trees](https://arxiv.org/abs/1709.00084) are useful for modular,
reactive task switching and execution policies. They fit best inside a
`ProcessDef`, NPC local policy, or activity continuation rule. They are weaker
as the top-level model for appraisal or commitment because they tend to hide
why a branch is active unless the surrounding system supplies provenance.

[The Sims smart terrain pattern](https://spectrum.ieee.org/mind-games) is
useful because objects advertise possible need satisfaction and interaction
semantics. `world` should adapt this through `PerceivedAffordance`, but keep
the actor-owned action model:

```text
external object:
  exposes observed affordances and possible bindings.

actor:
  owns CapabilitySet and ActionRepertoire.

intent layer:
  binds templates to perceived affordances and actor-owned schemas.
```

RimWorld-style thoughts, needs, jobs, reservations, and schedules are useful as
reference pressure already captured in [RimWorld](../references/rimworld.md).
The transferable shape is:

```text
event / condition / social relation
  -> thought or need pressure
  -> job or activity candidate
  -> reservations and execution
  -> inspectable consequences
```

For a single-protagonist RPG, the adaptation is important. Use thoughts,
pressures, activities, and reservations to make behavior legible. Do not copy a
colony work scheduler or make protagonist agency disappear behind forced mood
automation.

### AI-Agent Memory And Planning

[Generative Agents](https://arxiv.org/abs/2304.03442) is useful because it
shows a practical loop of observation, memory, reflection, retrieval, and
planning for believable agents. The transfer is selective retrieval and
reflection. The rejection is treating natural-language memory or LLM plans as
authoritative game state.

In `world`:

```text
LLM memory / reflection:
  may be private agent note, presentation, or typed proposal.

EpistemicRecord:
  gameplay-relevant actor-relative information accepted by the engine.

Thought / Pressure / GoalPressure:
  typed appraisal records accepted by the appraisal gate.

Intent:
  selected or suggested commitment accepted by the intent gate.
```

[CoALA](https://arxiv.org/abs/2309.02427) is useful because it names modular
memory, structured action space, and generalized decision-making as separate
parts of a language-agent architecture. That reinforces `world`'s existing
split between `AgentTurnInput`, `AgentTurnOutput`, working sets, action
schemas, intent choice, and mutation gates.

The engine should treat LLM output as proposal or policy choice, never as
state authority:

```text
AI proposes appraisal:
  -> appraisal gate
  -> AcceptedAppraisalRecord

AI proposes intent:
  -> intent gate / actor policy boundary
  -> selected or suggested Intent

AI submits concrete attempt:
  -> ActionRequest
  -> validation
  -> Typed Effect Program
  -> CausalTransaction
```

## Synthesis: Hybrid Architecture

Use a hybrid architecture. Do not build one monolithic planner.

```text
Observation / EpistemicWorkingSet / SocialContextView
  -> AppraisalVariableSet
  -> Thought
  -> Pressure
  -> GoalPressure
  -> CandidateIntent
  -> IntentScore
  -> selected or suggested Intent
  -> Activity
  -> ActionRequest or ProcessInstance depending on resolution
```

Each stage has a different job.

### AppraisalVariableSet

`AppraisalVariableSet` is the typed intermediate evaluation of actor-relative
meaning.

It should hold features such as:

- novelty
- relevance
- goal conduciveness
- agency / blame / credit
- certainty
- urgency
- coping potential
- norm compatibility
- role or duty relevance
- relationship weight
- threat / opportunity direction
- provenance

It is not itself a durable emotion label or action. It is the explainable
feature set that produces thoughts and pressures.

### Thought

`Thought` is actor-relative interpreted meaning.

Examples:

```text
Thought(GriefAboutDeath)
Thought(SawTabooViolation)
Thought(SuspectsTheft)
Thought(FeelsDebtToRescuer)
Thought(FearsFurtherAttack)
Thought(SeesChanceForEscape)
```

Rules:

- A thought references observations, epistemic records, social context, and
  appraisal variables.
- A thought may be accepted into `AppraisalRecordStore`.
- A thought may influence salience or later memory proposals, but it does not
  write `EpistemicStore` directly.
- A thought does not select an action.

### Pressure

`Pressure` is motivational force or action-readiness vector.

Examples:

```text
Pressure(Approach, target=mentor_body)
Pressure(Avoid, target=bandit_1)
Pressure(Retaliate, target=bandit_1)
Pressure(Report, target=town_guard)
Pressure(Conceal, target=stolen_relic)
Pressure(RepairViolation, target=shrine_order)
```

Pressure should carry:

- source thought or event
- direction
- target or topic
- intensity
- decay or persistence policy
- conflict links
- provenance

Pressure does not execute. It biases candidate generation and scoring.

### GoalPressure

`GoalPressure` is pressure shaped toward a possible desired state.

Examples:

```text
GoalPressure(FindKiller)
GoalPressure(AvoidPunishment)
GoalPressure(RestoreShrineOrder)
GoalPressure(RecoverFromWound)
GoalPressure(GetThroughLockedDoor)
```

`GoalPressure` is still not a commitment. It says which possible state is
motivationally salient, not which method has been chosen.

### Goal

`Goal` is a more stable objective, preference, duty, need, or authored
standing aim.

Examples:

```text
protect mentor
preserve shrine taboo
survive the night
reach market
keep oath to village
learn ritual language
```

A goal may be durable actor state, social obligation, schedule, role duty,
pack-authored need, or player-authored command. It can produce pressure, but
it does not choose a method by itself.

### CandidateIntent

`CandidateIntent` is a possible commitment generated from templates and
current bindings.

Examples:

```text
CandidateIntent(LocateActor, target=bandit_1)
CandidateIntent(AskInformationSource, topic=bandit_1, source=villager_1)
CandidateIntent(TrackPhysicalTrace, source_event=mentor_death)
CandidateIntent(ReportToAuthority, event=mentor_death, authority=town_guard)
CandidateIntent(CareForDead, subject=mentor_1)
CandidateIntent(FleeArea, threat=bandit_1)
```

Candidate generation should use:

- `Pressure`
- `GoalPressure`
- durable `Goal`
- `CapabilitySet`
- `ActionRepertoire`
- `PerceivedAffordance`
- `EpistemicWorkingSet`
- `SocialContextView`
- active `Activity` and interruption cost
- active resolution
- registered `IntentTemplate` definitions

Candidate generation should not mutate truth.

### IntentScore

`IntentScore` is an explainable ranking result, likely utility-style.

It should preserve feature contributions rather than only a final number:

```text
IntentScore
  candidate
  total
  features:
    pressure_fit
    capability_fit
    known_target
    risk
    urgency
    duty
    relationship_weight
    social_cost
    process_support
    current_activity_switch_cost
  explanation
```

`IntentScore` is allowed to rank and recommend. It is not an execution
surface.

### Intent

`Intent` is the selected or suggested commitment to a purpose and approach.

It should say:

- who holds it
- what purpose it commits to
- what approach or template it uses
- why it was selected
- what source pressures and goals support it
- what constraints and interruption rules apply
- whether it is player-selected, NPC-selected, AI-suggested, schedule-driven,
  or reaction-driven
- which resolutions it can lower into

Intent owns commitment. It still does not mutate hard truth.

### Plan

`Plan` is a decomposed structure for carrying out an intent.

It may be transient:

```text
Intent(FindBandit)
  -> Plan:
      inspect tracks
      follow route
      ask witness if trail lost
      confront if found
```

Or it may be embodied in a durable process definition:

```text
ProcessDef(TrackActor)
  methods:
    inspect traces
    update route hypothesis
    advance route progress
    stop on encounter / lost trail / danger / player interrupt
```

A plan is not automatically durable state. Durability belongs to `Intent`,
`Activity`, and `ProcessInstance` where the design requires it.

### Activity

`Activity` is the ongoing execution frame visible as actor-facing meaning over
process or plan execution.

Examples:

```text
tracking the bandit
recovering from wound
preparing ritual
traveling to market
guarding shrine door
searching room
```

Activity answers:

```text
What is the actor doing over time?
Why are future ticks or action requests connected?
What would interrupt or complete this work?
What can observers perceive about it?
```

Activity is the temporal execution boundary. It may be backed by one or more
`ProcessInstance`s and may produce concrete `ActionRequest`s when local.

### ActionRequest

`ActionRequest` is the concrete actor-owned attempt through validation,
effects, transaction, and events.

Examples:

```text
ApplyTool(lockpick, door.lock, pick)
Move(direction=north)
Inspect(target=tracks)
Speak(target=villager_1, speech_act=ask_about_bandit)
ApplyBandage(target=self.left_hand)
```

It is the local attempt boundary, not the whole purpose. It can fail,
partially succeed, be blocked, or emit actor-visible feedback.

### ProcessInstance

`ProcessInstance` is the durable execution/progress frame.

It is especially important for abstract resolution:

```text
ProcessInstance(TravelToMarket)
  active_resolution=abstract
  route_progress=0.53
  risk=storm, bandit_activity
```

It also supports concrete long work:

```text
ProcessInstance(PrepareRitual)
  active_resolution=concrete
  progress=0.35
  required_roles=[caster, assistant, offering]
  interrupt_policy=stop_on_attack_or_missing_offering
```

Abstract simulation should advance `ProcessInstance` with `ProcessTick`, not
spam hidden concrete `ActionRequest`s.

## Scenario Checks

### Mentor Killed By Bandit

```text
CausalTransaction
  -> EventRecord(ActorDied(victim=mentor_1, cause_actor=bandit_1))
  -> ObservedEvent(observer=player, victim=mentor_1, cause_actor=bandit_1)
  -> EpistemicRecord(holder=player, content=EventRecordRef(...))
  -> SocialContextView(relationship(player, mentor_1), law, witnesses, duty)
  -> AppraisalVariableSet(
       relevance=high,
       relationship_weight=high,
       agency=bandit_1,
       goal_conduciveness=strongly_negative,
       coping=uncertain,
       urgency=high
     )
  -> Thought(GriefAboutDeath)
  -> Pressure(Retaliate, target=bandit_1)
  -> GoalPressure(FindOrConfrontBandit)
  -> CandidateIntent(TrackPhysicalTrace)
  -> CandidateIntent(AskInformationSource)
  -> CandidateIntent(ReportToAuthority)
  -> CandidateIntent(CareForDead)
  -> IntentScore(...)
  -> selected Intent(FindBanditByTracking)
```

Lowering depends on resolution:

```text
concrete:
  Intent -> Activity(TrackingBandit)
         -> ActionRequest(Inspect(tracks))
         -> ActionRequest(MoveAlongTrace)

abstract:
  Intent -> Activity(TrackingBandit)
         -> ProcessInstance(TrackBandit, active_resolution=abstract)
         -> ProcessTick
         -> EventRecord(RouteProgressed / TrailLost / EncounterFound)
```

No hardcoded story action is needed.

### Wounded Hand And Lockpick

The wound should affect `CapabilitySet`, validation, and scoring, not action
availability through a bespoke case.

```text
hard truth:
  left_hand wounded
  actor carries lockpick
  actor knows lockpicking

CapabilitySet:
  fine manipulation degraded

ActionRepertoire:
  ApplyTool(tool, target, mode) still exists

PerceivedAffordance:
  door.lock appears pickable

CandidateIntent:
  OpenLockedDoorByPicking

IntentScore:
  lower capability_fit
  higher risk
  higher time cost

concrete lowering:
  ActionRequest(ApplyTool(lockpick, door.lock, pick))
  -> validation and effect execution
```

The wound may make the attempt slower, riskier, noisier, or invalid if severe,
but the logic belongs to capability derivation, scoring, validation, and effect
resolution.

### Shrine Relic Removal

Keep physical transfer, social context, actor belief, appraisal, and intent
separate.

```text
hard truth:
  EventRecord(ItemTransferred(shrine_relic, shrine_floor, actor_inventory))

social state:
  SocialClaim(shrine_order owns shrine_relic)
  Norm(shrine forbids non-priest removal)

actor belief:
  EpistemicRecord(holder=actor, content=SocialClaimRef(...), confidence=low)

guard observation:
  ObservedEvent(actor took shrine_relic)

guard social context:
  role_granted shrine law and jurisdiction

guard appraisal:
  Thought(SawTabooViolation)
  Pressure(EnforceShrineLaw)
  GoalPressure(RestoreRelicToShrine)

intent:
  CandidateIntent(WarnActor)
  CandidateIntent(DetainActor)
  CandidateIntent(ReportToPriest)
  CandidateIntent(RecoverObject)
```

The transfer event does not itself say theft, sacrilege, guilt, or revenge.
Those meanings are appraisal outputs under social and epistemic context.

### Abstract Travel

Selected travel intent becomes durable process progress, not repeated concrete
move spam.

```text
Intent(TravelToMarket)
  -> ProcessInstance(TravelToMarket, active_resolution=abstract)
  -> ProcessTick(route_progress += delta, risk checks)
  -> CausalTransaction
  -> EventRecord(RouteProgressed)
  -> EventRecord(DelayedByStorm?) if triggered
```

Promotion later refines the process into local concrete state with provenance.
Demotion coarsens maintained current location while preserving important hard
facts, active process identity, and event evidence.

## Design Implications For Later Documents

### Semantic Appraisal And Motivation

The later design doc should probably define:

- `AppraisalVariableSet`
- `AppraisalRule`
- `Thought`
- `Pressure`
- `GoalPressure`
- accepted appraisal record lifecycle
- provenance and explanation format
- pressure persistence, decay, reactivation, suppression, and conflict
- AI appraisal proposal gate
- social and epistemic query requirements

It should not define final intent selection, action validation, or process
execution.

### Intent Templates And Planning

The later design doc should probably define:

- `Goal` relation to `GoalPressure`
- `IntentTemplate`
- binding query requirements
- `CandidateIntent`
- `IntentScore`
- selected or suggested `Intent`
- transient `Plan`
- `Activity`
- resolution-aware lowering to `ActionRequest` and/or `ProcessInstance`
- protagonist suggestion policy
- NPC selection policy
- AI intent proposal gate
- invalid, impossible, blocked, and unavailable candidate representation

It should not own appraisal itself, hard mutation, or typed effect execution.

## Failure Modes To Avoid

Avoid one global planner that reads everything and returns actions. That would
collapse perception, memory, social context, appraisal, intent, and mutation
authority.

Avoid treating memory as a behavior script. `EpistemicRecord` can influence
working sets and appraisal, but it should not directly select actions.

Avoid treating social state as motive. `SocialClaim`, norm, law, role, and duty
provide context. Appraisal produces current thoughts and pressures.

Avoid making pressure executable. Pressure is readiness and bias. Intent is the
commitment boundary.

Avoid making intent an effect. Intent guides lowering and activity. It does not
move bodies, transfer items, create wounds, or write events.

Avoid using concrete action spam to simulate abstract work. Abstract execution
uses `ProcessInstance` and `ProcessTick`.

Avoid LLM-authored natural language as authority. AI output must become typed
proposal, actor policy choice, or explanation over existing provenance.

## Source Notes

- [Ortony, Clore, and Collins, The Cognitive Structure of Emotions](https://www.cambridge.org/core/books/cognitive-structure-of-emotions/cognitive-structure-of-emotions/D16784CA50F5BBD58F3140B575BB7881)
  for OCC's computationally tractable appraisal structure over events, agents,
  and objects.
- [Scherer, What are emotions? And how can they be measured?](https://journals.sagepub.com/doi/10.1177/0539018405058216)
  for component-process framing and the need to distinguish affective
  processes and components.
- [Smith and Lazarus, Appraisal components, core relational themes, and the emotions](https://www.tandfonline.com/doi/abs/10.1080/02699939308409189)
  and [Lazarus, From Psychological Stress to the Emotions](https://www.annualreviews.org/content/journals/10.1146/annurev.ps.44.020193.000245)
  for relational meaning, appraisal components, coping, and core themes.
- [Frijda, The Emotions](https://mitpressbookstore.mit.edu/book/9780521316002)
  and [Frijda and Parrott, Basic Emotions or Ur-Emotions?](https://journals.sagepub.com/doi/10.1177/1754073911410742)
  for appraisal as concern-relative evaluation that modifies action readiness.
- [Marsella and Gratch, EMA: A process model of appraisal dynamics](https://www.sciencedirect.com/science/article/abs/pii/S1389041708000314)
  for computational appraisal over an interpreted agent-environment relation.
- [Bratman, Intention, Plans and Practical Reason](https://press.uchicago.edu/ucp/books/book/distributed/I/bo3629095.html)
  for intention as commitment and partial planning over time.
- [Rao and Georgeff, BDI Agents: From Theory to Practice](https://cdn.aaai.org/ICMAS/1995/ICMAS95-042.pdf)
  for the belief, desire, and intention separation in practical agents.
- [Georgeff and Lansky, Reactive Reasoning and Planning](https://aaai.org/Papers/AAAI/1987/AAAI87-121.pdf)
  for PRS-style procedural reasoning in dynamic environments.
- [Rao, AgentSpeak(L)](https://dblp.org/rec/conf/maamaw/Rao96.html)
  for a BDI programming-language reference point.
- [Orkin, Three States and a Plan: The A.I. of F.E.A.R.](https://www.madwomb.com/tutorials/gamedesign/prototyping/gdc2006_JeffOrkin_AI_FEAR.pdf)
  for GOAP's goal/action decoupling and dynamic replanning pressure.
- [Nau et al., SHOP2: An HTN Planning System](https://cir.nii.ac.jp/crid/1360298343490834816?lang=en)
  for HTN decomposition and temporal/metric planning pressure.
- [Colledanchise and Ogren, Behavior Trees in Robotics and AI](https://arxiv.org/abs/1709.00084)
  for behavior trees as modular task switching.
- [Dill and Mark, Embracing the Dark Art of Mathematical Modeling in AI](https://www.gdcvault.com/play/1015683/Embracing-the-Dark-Art-of)
  for utility-style game-AI scoring.
- [IEEE Spectrum, Mind Games](https://spectrum.ieee.org/mind-games)
  for The Sims smart terrain pattern, where objects advertise interaction
  possibilities.
- [Park et al., Generative Agents](https://arxiv.org/abs/2304.03442)
  for observation, memory, reflection, retrieval, and planning with LLM-based
  agents.
- [Sumers et al., Cognitive Architectures for Language Agents](https://arxiv.org/abs/2309.02427)
  for modular memory, structured action spaces, and decision processes in
  language-agent architectures.
