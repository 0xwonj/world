# Actor Pressure And Interpretation

## Status

Promoted source history.

## Promotion / Boundary Note

Stable design pressure from this idea has been promoted into:

- [Epistemic State](../design/epistemic-state.md)
- [Truth, Authority, And Layer Boundaries](../design/truth-authority-and-layer-boundaries.md)
- [Social Institutional Model](../design/social-institutional-model.md)
- [Semantic Appraisal And Motivation](../design/semantic-appraisal-and-motivation.md)
- [Intent Templates And Planning](../design/intent-templates-and-planning.md)

The current boundary is stricter than some wording below: hard
`EventRecord`s do not declare theft, crime, taboo, grief, or revenge.
Observation and epistemic persistence create `EpistemicRecord`s. Semantic
appraisal creates `Thought`, `Pressure`, `GoalPressure`, and semantic records.
Intent planning owns candidate intent binding and final intent choice.
Story-specific examples such as `PursueBandit` or `AskVillagers` below are
historical shorthand; the promoted design uses generic `CandidateIntent`
bindings and `IntentScore`.

## Dependency Chain

This idea connects several lower-level ideas:

1. [Action and Event Model](../design/action-event-model.md)
2. [Perception And Observation](../design/perception-and-observation.md)
3. [Knowledge, History, And Belief](knowledge-history-and-belief.md)
4. [Actor Intent And Activity](actor-intent-and-activity.md)

Action/event rules define what happened. Perception determines who observed it.
Knowledge and belief provide actor-specific context. Actor pressure turns
observed and interpreted events into future intent bias.

## Core Idea

Events should not directly hardcode actor behavior. Instead, structured events
should pass through actor-specific interpretation rules and persistence gates
that create `EpistemicRecord`s, thoughts, pressures, and proposed intent bias.

```text
ActionRequest
  -> EventRecord
  -> actor-specific ObservedEvents
  -> EpistemicRecord persistence gate
  -> AppraisalRules
  -> Thoughts / Pressures / proposed intent bias
  -> CandidateIntent generation in intent planning
  -> Intent scoring
  -> selected or suggested Intent
  -> next ActionRequest
```

This lets the game respond to meaningful events without writing one-off logic
for every story case.

## Motivation

Consider this scenario:

```text
Event: bandit killed player's mentor
Memory: saw mentor killed by bandit
Thought: grief and revenge
Pressure: revenge high, caution low
Candidate intents:
  - pursue bandit
  - ask villagers about bandit
  - bury mentor
  - flee area
```

This cannot be implemented as:

```text
if bandit killed mentor:
  player wants revenge
```

That would only handle one case. The game needs a reusable pipeline that can
also handle:

- sibling killed by guard
- enemy killed by ally
- faction leader assassinated
- prisoner executed under local law
- forbidden ritual victim sacrificed
- animal companion wounded
- stranger murdered in front of a cowardly merchant

The reusable rule is not "bandit killed mentor." The reusable rule is closer
to:

```text
witnessed violent death
  + victim relationship
  + known cause actor
  + actor personality, duty, belief, law, and faction context
  -> grief, fear, revenge, alarm, relief, guilt, or approval
```

## Design Principle

The system should move hardcoding up one level.

Bad hardcoding:

```text
if event == BanditKilledMentor:
  create Revenge
```

Better rule:

```text
if observed ActorDied
and observer has close emotional relationship to victim
and cause actor is known
then create Grief and RevengeToward(cause_actor)
```

The second rule is still authored design logic, but it is reusable across many
characters, factions, relationships, and stories.

## Structured Event Records For Appraisal

The action/event model needs `EventRecord`s that are structured enough for
later interpretation.

Raw events are often too weak:

```text
AttackHit
  attacker: bandit_1
  target: mentor_1
  damage: 14
```

The engine also needs `EventRecord`s with enough roles and physical context for
appraisal:

```text
AttackResolved
  attacker: bandit_1
  target: mentor_1
  weapon: rusted_knife
  intent_kind: lethal_attack
  damage: 14
  tags: [violence]

ActorDied
  actor: mentor_1
  cause_actor: bandit_1
  cause_event: attack_event_123
  tags: [death, violence]
```

`ActorDied` is not a special "mentor killed" event. It is a general event. The
mentor meaning comes from relationship and interpretation.

## Event Roles And Tags

Generic interpretation requires events to expose roles and tags.

Example:

```text
Event
  kind: ActorDied
  roles:
    victim: mentor_1
    cause_actor: bandit_1
  tags:
    - death
    - violence
    - hostile
  location: old_bridge
  time: 1042
  source_action: attack_action_221
```

Useful roles:

- actor
- target
- victim
- attacker
- helper
- owner
- thief
- witness
- speaker
- listener
- cause_actor
- source_item
- location
- faction

Useful hard event tags:

- death
- violence
- threat
- help
- secret
- fire
- poison
- disease
- public
- hidden

Useful semantic/appraisal labels, not hard event tags:

- betrayal
- theft
- trespass
- taboo
- law
- ritual
- insult
- gift
- promise

Roles answer "who played what part?" Tags answer "what kind of event is this?"

## Interpretation Taxonomy

The interpretation taxonomy should be broad enough for appraisal, but not so
broad that every story beat becomes a bespoke hard fact. Some entries below
are `EventRecord` candidates; others are semantic/appraisal record labels.
The current design owner must decide which layer owns each one.

Candidate families:

### Violence

```text
AttackResolved
ActorWounded
ActorDied
Threatened
ForcedMovement
KnockedUnconscious
```

### Social

```text
Insulted
Helped
Betrayed
GiftGiven
PromiseMade
PromiseBroken
Protected
Abandoned
```

### Knowledge

```text
SecretLearned
RumorHeard
LieDetected
LawLearned
NameLearned
MapLocationLearned
```

### Property And Social Interpretation

```text
EventRecord candidates:
  ItemDestroyed
  ItemGiven
  DoorOpened

Semantic/appraisal record labels:
  ItemStolen
  TrespassObserved
  OwnershipClaimed
```

### Ritual And Law

```text
EventRecord candidates:
  RitualStarted
  RitualInterrupted
  RitualCompleted
  SentenceCarriedOut

Semantic/appraisal record labels:
  TabooViolated
  CrimeWitnessed
  OathBroken
```

These are not final type names. They are pressure points for the ontology.

## Observed Events

World events and actor-observed events are not the same thing.

Example:

```text
ActorDied
  victim: mentor_1
  cause_actor: bandit_1
```

Different actors may observe different projections:

```text
player:
  observed: ActorDied
  perceived victim: mentor_1
  perceived cause_actor: bandit_1
  channel: sight
  confidence: high

guard:
  observed: scream and body later
  perceived victim: unknown_person
  perceived cause_actor: unknown
  channel: hearing_then_sight
  confidence: medium

merchant:
  observed: body after event
  perceived victim: traveler
  perceived cause_actor: unknown
  channel: sight
  confidence: low
```

This suggests a perception pass:

```text
Event
  -> visibility, audibility, smell, social recognition, magical detection
  -> ObservedEvent for each actor
```

`ObservedEvent` should preserve uncertainty. An actor may know a crime happened
without knowing who did it.

## Actor Context

Interpretation needs more than the event.

Input context may include:

- relationship to involved actors
- faction membership and reputation
- personal memory
- beliefs and false beliefs
- known laws and taboos
- culture or ideology
- role and duty
- traits and personality
- current needs and pressure
- body condition
- location context
- ownership and permission

The same event can then produce different interpretation.

Example:

```text
Event:
  ActorDied(victim: cult_sacrifice, cause_actor: priest)

zealot:
  Thought: awe
  Pressure: faith_up

guard:
  Thought: legal_alarm
  Pressure: report_crime

victim_kin:
  Thought: grief
  Pressure: revenge

scholar:
  Thought: curiosity
  Pressure: investigate_ritual
```

This is why actor-specific interpretation matters.

## Interpretation Rules

Current design splits this flow: epistemic persistence creates or updates
`EpistemicRecord`s; appraisal rules convert observed events plus actor context
into thoughts and pressure changes. Appraisal rules may propose epistemic
updates, but they do not directly own memory storage.

Example rule:

```text
Rule: witnessed_death_of_close_relation

When:
  observed_event.kind == ActorDied
  observer perceived victim
  observer relationship to victim has emotional_weight >= high

Then:
  require or propose EpistemicRecord(kind: WitnessedDeath)
  create Thought(kind: Grief, intensity: high, duration: long)

  if cause_actor is known:
    create Pressure(kind: Revenge, target: cause_actor, intensity: high)
```

Another rule:

```text
Rule: witnessed_violent_death_of_stranger

When:
  observed_event.kind == ActorDied
  observed_event has tag violence
  observer relationship to victim is neutral_or_unknown

Then:
  require or propose EpistemicRecord(kind: WitnessedViolentDeath)
  create Thought(kind: Fear, intensity: medium, duration: short)
  create Pressure(kind: AvoidDanger, target: cause_actor_or_location)
```

The rules should avoid directly choosing an intent. They should create pressure
that later influences candidate intents.

## Data-Driven Rules

Interpretation rules can be authored as data where possible.

Example shape:

```yaml
- id: witnessed_death_of_close_relation
  when:
    event: ActorDied
    observed: true
    relationship_to:
      role: victim
      emotional_weight_at_least: high
  memories:
    - kind: WitnessedDeath
      subject: event.roles.victim
      source_event: event.id
  thoughts:
    - kind: Grief
      intensity: high
      duration: long
      source_memory: created.memory
  pressures:
    - kind: Revenge
      target: event.roles.cause_actor
      intensity: high
      if:
        role_known: cause_actor
```

This is not a no-code design requirement. Some rule families may need code.
The important point is that authored rules should target reusable patterns, not
one-off story cases.

## Memory, Thought, And Pressure

The system should distinguish memory, thought, and pressure.

### Memory

Memory records what an actor perceived or learned.

```text
Memory
  kind: WitnessedDeath
  subject: mentor_1
  cause_actor: bandit_1
  source_event: actor_died_456
  channel: sight
  confidence: high
  age: 0
```

Memory can be stale, incomplete, or false if it came from rumor or bad
perception.

### Thought

Thought is an actor-specific interpretation of a memory or current situation.

```text
Thought
  kind: Grief
  source_memory: memory_789
  intensity: high
  duration: long
```

The same memory may produce different thoughts for different actors.

### Pressure

Pressure is the behavioral force that affects intent selection.

```text
Pressure
  kind: Revenge
  target: bandit_1
  intensity: 0.9
  source: memory_789
  decay: slow
  effects:
    - bias intent PursueActor +80
    - bias intent AskAboutActor +40
    - bias intent BuryDead +20
    - reduce caution against target
```

Pressure should not directly mutate the world. It should bias intent and action
selection.

## Pressure Kinds

Initial pressure kinds might include:

- fear
- revenge
- grief
- anger
- guilt
- gratitude
- loyalty
- duty
- curiosity
- hunger
- fatigue
- pain
- faith
- shame
- suspicion
- protectiveness
- greed

These are not meant to be a complete emotion model. They are gameplay-relevant
forces that can explain why actors choose or avoid intents.

## Candidate Intent Generation

Pressure should usually generate or bias candidate intents, not directly pick
one.

Example mappings:

```text
Pressure: Revenge(target: bandit_1)
  candidates:
    - PursueActor(bandit_1)
    - AskAboutActor(bandit_1)
    - ConfrontActor(bandit_1)
    - ReportCrime(actor: bandit_1)

Pressure: Grief(subject: mentor_1)
  candidates:
    - BuryDead(mentor_1)
    - Mourn(mentor_1)
    - SeekComfort

Pressure: Fear(source: bandit_1)
  candidates:
    - FleeArea
    - Hide
    - CallForHelp
```

Pressures generate candidate intents. Actor-owned capabilities and perceived
affordances determine how those intents can be attempted:

```text
corpse perceived nearby -> candidate intent: BuryDead, InspectCorpse
villagers perceived nearby -> candidate intent: AskAboutActor
visible trail -> candidate intent: FollowTrail

actor can track -> owns tracking-related process schema
actor has shovel -> has tool method for digging/burying
hands injured -> manipulation-heavy requests become harder or invalid
```

This keeps pressure connected to the action system without bypassing it.

## Intent Scoring

Candidate intents can be scored from pressures, capabilities, risk, context,
and actor policy.

Example:

```text
IntentScore(PursueBandit)
  revenge +80
  visible trail +20
  actor wounded -30
  fear -10
  total 60

IntentScore(AskVillagers)
  revenge +40
  villagers nearby +30
  low tracking skill +20
  total 90

IntentScore(BuryMentor)
  grief +60
  corpse nearby +30
  revenge urgency -20
  total 70

IntentScore(FleeArea)
  fear +50
  revenge -40
  total 10
```

For NPCs, the selected intent may be the highest-scoring acceptable intent, with
some personality or randomness added.

For the protagonist, the game should usually not force the highest-scoring
intent. It can expose pressure through UI, available options, risks, prompts,
or AI-assist context while preserving player agency.

## Player Character Versus NPCs

The system should apply differently to NPCs and the protagonist.

NPCs:

```text
pressure feeds intent scoring and continuation policy
```

Examples:

- a terrified guard flees
- a grieving sibling seeks revenge
- a hungry beast stalks prey
- a loyal retainer protects their lord

Player character:

```text
pressure should mostly change costs, risks, options, prompts, and consequences
```

Examples:

- fatigue reduces travel or combat effectiveness
- fear increases failure risk for bold actions
- anger unlocks aggressive dialogue but makes diplomacy harder
- grief biases suggested intents but does not force revenge
- panic or mind control may create explicit temporary control-state changes

This preserves the single-protagonist RPG feel.

## Control-State Changes

Extreme pressure may create a temporary control-state change.

Examples:

```text
Panic
  caused by fear pressure above threshold
  possible intents: flee, hide, freeze

Berserk
  caused by anger or pain above threshold
  possible intents: attack nearby threat

Despair
  caused by grief and fatigue
  possible intents: collapse, refuse action, seek comfort
```

These should be rare, explicit, and event-backed. They should not silently steal
control from the player.

Events:

```text
ControlStateChanged
PanicStarted
PanicEnded
```

## Worked Example

### Step 1: Action Resolution

```text
ActionRequest
  actor: bandit_1
  action: Attack
  target: mentor_1

Events
  AttackResolved
    attacker: bandit_1
    target: mentor_1
    tags: [violence, hostile]

  ActorDied
    victim: mentor_1
    cause_actor: bandit_1
    cause_event: attack_resolved_123
    tags: [death, violence]
```

### Step 2: Perception

```text
ObservedEvent
  observer: player
  event: ActorDied
  perceived_roles:
    victim: mentor_1
    cause_actor: bandit_1
  channel: sight
  confidence: high
```

### Step 3: Interpretation

```text
ActorContext
  relationship(player, mentor_1):
    kind: mentor
    emotional_weight: high
    trust: high
  relationship(player, bandit_1):
    kind: hostile
```

Rules apply:

```text
witnessed_death_of_close_relation
known_cause_actor_of_violent_death
```

Outputs:

```text
Memory
  kind: WitnessedDeath
  subject: mentor_1
  cause_actor: bandit_1

Thought
  kind: Grief
  intensity: high
  source_memory: witnessed_death

Thought
  kind: Revenge
  target: bandit_1
  intensity: high
  source_memory: witnessed_death

Pressure
  kind: Revenge
  target: bandit_1
  intensity: high

Pressure
  kind: Grief
  subject: mentor_1
  intensity: high
```

### Step 4: Candidate Intents

```text
Candidates
  PursueActor(bandit_1)
  AskAboutActor(bandit_1)
  BuryDead(mentor_1)
  InspectCorpse(mentor_1)
  FleeArea
```

### Step 5: Scoring

```text
PursueActor
  revenge high
  trail visible
  player wounded

AskAboutActor
  revenge high
  villagers nearby
  bandit fled

BuryDead
  grief high
  corpse nearby

FleeArea
  fear medium
  revenge high opposes flee
```

For an NPC, this may strongly bias the planning layer toward one intent. For
the protagonist, it can shape suggested activities, dialogue tone, risks, and
available emotional actions.

## Observation And Debugging

Players should not always see raw pressure values. They should see evidence and
felt state.

Possible player-facing outputs:

- "You remember the bandit's face."
- "Grief makes it hard to focus."
- "You can still see blood leading north."
- "The villagers may know where the bandit went."

Debug tools can show the full chain:

```text
Actor: player
ObservedEvent: ActorDied(mentor_1)
Memory: WitnessedDeath
Thoughts: Grief(high), Revenge(high)
Pressure: Revenge(target: bandit_1, 0.9)
CandidateIntents:
  AskAboutActor: 90
  BuryDead: 70
  PursueActor: 60
  FleeArea: 10
```

This makes emergent behavior explainable.

## Relationship To Actor Intent

Actor pressure answers:

```text
Why does this actor want or avoid certain intents?
```

Actor intent answers:

```text
What is this actor trying to do across turns?
```

Action requests answer:

```text
What does this actor attempt now?
```

Events answer:

```text
What actually happened?
```

Pressure should feed intent selection, not replace intent.

## Relationship To Knowledge And Belief

Interpretation must respect partial information.

Examples:

- The actor saw the victim die but not the killer.
- The actor falsely believes a rival caused the death.
- The actor heard a rumor rather than witnessing the event.
- The actor knows local law and interprets the event as lawful execution.
- The actor believes the victim deserved punishment.

Therefore pressure may be based on belief, not truth.

Example:

```text
Belief
  actor: guard
  proposition: merchant killed priest
  confidence: medium
  source: rumor

Pressure
  kind: suspicion
  target: merchant
  source: belief
```

This can create false accusations, fear, revenge, or protection behavior while
remaining explainable.

## Design Risks

- If events are too raw, interpretation rules cannot work.
- If events are too bespoke, the model becomes story-case hardcoding.
- If tags are vague, rules become unpredictable.
- If interpretation ignores perception, actors become omniscient.
- If interpretation ignores relationship and belief, actors react too
  uniformly.
- If pressure directly chooses actions, the intent layer becomes pointless.
- If pressure is only numeric mood, behavior becomes hard to explain.
- If player pressure removes agency too often, the RPG becomes frustrating.
- If every small event creates pressure, actor state becomes noisy.
- If rules are too data-driven without tests, behavior becomes hard to reason
  about.

## Open Questions

- What is the minimal semantic/appraisal record taxonomy needed first?
- Should interpretation rules be authored in data, code, or a hybrid?
- How should conflicting thoughts combine?
- How should pressure decay over time?
- Which pressures should be continuous needs versus event-derived thoughts?
- How much pressure should affect the protagonist directly?
- Should pressure create new candidate intents, or only score existing
  candidates?
- How should false belief and rumor-driven pressure be corrected?
- Which pressure transitions should emit canonical events?
- How should debug tools display the chain from event to intent?

## Related References

- [RimWorld](../references/rimworld.md)
- [Action and Event Model](../design/action-event-model.md)
- [Perception And Observation](../design/perception-and-observation.md)
- [Knowledge, History, And Belief](knowledge-history-and-belief.md)
- [Actor Intent And Activity](actor-intent-and-activity.md)
- [Actor-Owned Capability-Derived Actions](capability-derived-actions.md)
