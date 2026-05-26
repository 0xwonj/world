# RimWorld

## Why It Matters

RimWorld is a useful reference for `world` because it makes actor motivation,
work selection, needs, social relationships, and external pressure legible.

Caves of Qud is stronger for embodied affordances, systemic items, generated
history, and knowledge as content. RimWorld is stronger for the question:

```text
Given a stateful actor in a changing environment, why does it choose this job
now, and what past or present pressures make that choice believable?
```

For `world`, the goal is not to copy RimWorld's colony-management structure.
The goal is to extract machinery for autonomous actors, inspectable motivation,
and world-pressure systems that can operate without giving agents omniscient
state or letting any client own game truth.

## Research Focus

RimWorld should be studied mainly through these lenses:

- pawn state as the source of action selection
- jobs, work priorities, schedules, allowed areas, and reservations
- needs, thoughts, mood targets, mental breaks, and recovery
- social opinion, relationships, romance, family, ideology, and ritual effects
- storyteller incidents as an external pressure layer
- zones, bills, stockpiles, workbenches, and player-authored work surfaces
- inspectable causality in UI and debugging tools
- data definitions that connect content to behavior

## Distinctive Systems

### Pawn As Motivated Actor

RimWorld pawns are not just tokens that execute orders. A pawn carries a large
bundle of state:

- skills and passions
- traits and backstory-derived work restrictions
- health conditions, body parts, pain, and capacities
- needs such as food, rest, recreation, comfort, and addictions
- thoughts and mood
- social opinions and relationships
- work assignments and priorities
- schedule
- allowed area
- current job and queued orders

The important pattern is that actor behavior is not driven by one "AI" field.
It is assembled from state, constraints, environment, player-authored
assignments, and job-selection rules.

For `world`, an actor should probably be modeled as:

```text
Actor = Body + Mind + Skills + Needs + SocialState + Knowledge + Constraints
        + CurrentTask + DecisionPolicy
```

This does not require every actor to be as detailed as a RimWorld colonist. It
does suggest that actor autonomy needs explicit state that can be inspected and
explained.

### Job As Behavior Unit

RimWorld's useful behavior abstraction is the job. Public modding material
describes pawn behavior as job-based: a job is assigned to a pawn, a job driver
decides how it is carried out, and the job breaks into smaller toils.

This is not the same as `world`'s current `ActionRequest` idea. A job can be
longer-lived than one action:

```text
Job: build wall
  -> reserve target and ingredients
  -> walk to material
  -> pick up material
  -> walk to blueprint
  -> work for some ticks
  -> finish or fail
```

For `world`, this suggests a useful distinction:

- `ActionRequest`: one attempted world-changing step.
- `Task` or `Job`: an actor-local plan that may submit many action requests.
- `Event`: what actually happened after each attempt.

This boundary would let a human, rule-based NPC, or AI agent choose a larger
task while the simulation still records deterministic action/event steps.

### Work Priority And Schedule As Soft Control

RimWorld's player usually does not directly pilot every pawn. The player shapes
behavior through work priorities, schedules, allowed areas, bills, zones, and
manual orders. Pawns then attempt work according to those constraints unless
needs become urgent.

This creates an important middle layer between full autonomy and direct command:

```text
Player or scenario intent
  -> standing assignments and constraints
  -> pawn job search
  -> current job
  -> job steps
  -> world consequences
```

For `world`, this is useful even outside colony management. A guard can have a
patrol schedule, a priest can have ritual obligations, a beast can have hunger
and territory constraints, and an AI-controlled character can have standing
goals that bias job selection without bypassing action validation.

### Needs, Thoughts, Mood, And Breaks

RimWorld turns circumstances and events into actor-local pressure. Needs fall
or rise over time. Situation-specific thoughts appear or expire. The pawn's
mood target reflects the current set of thoughts, while the mood value moves
toward that target over time. Low mood can trigger mental breaks that temporarily
remove ordinary control and create destructive, avoidant, or strange behavior.

The transferable principle is not the exact mood numbers. The useful shape is:

```text
Event or condition
  -> actor-local thought/memory/pressure
  -> mood or motive target
  -> behavior risk or opportunity
  -> possible control-state change
```

This matters for `world` because memory should not be only a log. Some memories
should be active pressures that affect future action choice.

Examples:

- saw a corpse
- ate forbidden food
- was insulted
- slept in danger
- worked on a passion-aligned task
- was wounded
- lost a friend
- witnessed a taboo

Each can become a structured actor-local record with source, duration,
intensity, and behavioral effects.

### Social Opinion And Relationship State

RimWorld's social model is not just faction reputation. It has directed opinion
between pawns, family ties, romance, lovers, ex-lovers, spouses, rivals,
friends, and event-driven social consequences. Some relations are symmetric,
but opinions can be asymmetric.

This is useful for `world` because social state should not collapse into one
global reputation number.

Useful primitives:

- directed opinion: A's current opinion of B
- relationship fact: sibling, rival, friend, lover, spouse, ex-lover
- event memory: B insulted A, A killed B's family member, A helped B
- social expectation: sleep together, avoid enemy, obey role, honor vow
- ideological filter: the same act may be acceptable to one group and offensive
  to another

RimWorld is weaker as a model of belief and deception. It is stronger as a
model of relationship facts, opinion modifiers, and social consequences that
feed back into mood and behavior.

### Health And Capacity As Action Pressure

RimWorld tracks health through body parts, injuries, pain, and capacities such
as sight, hearing, moving, manipulation, talking, consciousness, breathing, and
digestion. A pawn's body changes what it can do and how well it can do it.

This overlaps with the Qud-derived body-as-action-space idea, but RimWorld adds
an important motivation angle: pain and impaired capacity also feed mood,
speed, risk, and work suitability.

For `world`, body state should influence both:

- actor-owned action repertoire: can this actor see, speak, move, manipulate,
  or hear?
- action selection: is this actor in too much pain, fear, hunger, or exhaustion
  to choose ordinary work?

### Storyteller As World Pressure

RimWorld's storyteller does not inhabit the map as a normal pawn. It is an
external event-selection layer that looks at the colony situation and creates
incidents such as raids, traders, resource drops, and animal threats.

For `world`, the useful abstraction is a pressure generator:

```text
WorldState + history + scenario rules
  -> IncidentProposal
  -> validation/scheduling
  -> structured world events or actor spawns
```

This should not bypass the simulation. A storyteller-like layer should submit
incidents through a deterministic, replayable boundary. It can shape drama, but
it should not secretly mutate authoritative state.

### Zones, Bills, And Declarative Work Surfaces

RimWorld lets players create work by marking areas and configuring objects:

- stockpile zones declare what items belong where
- growing zones declare what should be planted
- bills request workstation production or pawn operations
- allowed areas constrain where pawns may work
- workbenches, beds, doors, shelves, and stockpiles become action affordances

This is a strong pattern for `world`: not every action opportunity needs to
come from an actor's private goal. Some work exists because the world contains
standing requests and configured affordances.

Possible `world` equivalents:

- a shrine has a ritual schedule
- a forge has active craft orders
- a guard post has a watch assignment
- a town has public laws and forbidden zones
- a storage area requests certain item categories
- a wounded actor has a treatment request

These should still become action requests when an actor attempts them.

### Reservations And Conflict Avoidance

RimWorld jobs can reserve targets before execution so two pawns do not try to
perform incompatible work on the same object at the same time.

For `world`, this suggests a first-class conflict layer between action
selection and action resolution:

```text
CandidateTask
  -> reserve actor/target/resource/time slot
  -> execute action requests
  -> release reservation on finish, failure, or interruption
```

This matters whenever multiple actors can compete for one door, item, patient,
conversation target, workstation, hiding spot, or ritual role.

### Inspectable Causality

RimWorld exposes a lot of causal state to the player: needs, mood markers,
thoughts, jobs, queued orders, work priorities, restrictions, and social tabs.
Modding documentation also encourages checking job legality, job definitions,
job drivers, and individual job steps when debugging behavior.

For `world`, this is not just UI polish. It is architectural pressure:

- every chosen action should have inspectable reasons
- every failed action should have structured failure causes
- actor mood or belief changes should point back to source events
- AI-agent input should expose enough semantic state to choose actions without
  reading hidden truth
- replay tools should explain why an actor did something, not only what it did

## Foundational World Model

RimWorld appears to center on a map of things, pawns, zones, buildings, items,
jobs, needs, thoughts, and incidents. It is not publicly presented as an
event-sourced architecture, but its visible systems strongly distinguish:

- standing configuration: schedules, work priorities, bills, zones
- actor state: health, skills, needs, mood, relationships
- environment state: items, buildings, terrain, stockpiles
- active work: current job, reservations, queued orders
- external pressure: storyteller incidents and quests

For `world`, the extracted model should avoid mixing these together. A useful
boundary might be:

```text
WorldState
  map, entities, items, bodies, social facts, knowledge facts, standing requests

ActorState
  needs, memories, beliefs, skills, body capacities, relationships, current task

TaskState
  chosen job/plan, targets, reservations, progress, interruption rules

DirectorState
  scenario pressure, incident cooldowns, pacing, reproducible random state
```

## Action, Job, And Event Implications

RimWorld's job model suggests that `world` should not make every high-level
intention a single action.

Bad collapse:

```text
ActionRequest = CraftSword
```

This hides too much. It erases resource search, workstation use, interruptions,
skill checks, progress, social consequences, and failure points.

Better shape:

```text
Task
  actor: blacksmith
  kind: Craft
  target: iron_sword_bill
  workstation: village_forge

ActionRequests over time
  ReserveWorkstation
  MoveToStorage
  PickUpMaterial
  MoveToForge
  WorkAtForge
  FinishCraft

Events
  WorkstationReserved
  MaterialPickedUp
  CraftProgressed
  CraftInterrupted
  ItemCreated
```

This keeps replay and debugging granular while still letting agents reason at
the task level.

## Need, Thought, And Memory Implications

`world` already has a candidate idea around knowledge, history, and belief.
RimWorld adds a sharper model for actor-local pressure.

Possible distinction:

- `Memory`: record of a perceived or experienced event.
- `Thought`: active interpretation of a memory or situation.
- `Need`: continuous actor state that trends over time.
- `Mood` or `Stability`: aggregated behavioral pressure.
- `MentalState`: temporary change to ordinary action selection or control.

Example:

```text
Event
  AllyKilled(actor: guard, ally: captain)

Memory
  guard remembers captain died at gate, source: direct sight, age: 0 turns

Thought
  grief over captain death, intensity: high, duration: 3 days

Mood/Stability
  lowered by active grief thought

Behavior
  less likely to patrol alone, more likely to seek revenge, risk of breakdown
```

The key design question is whether these are generic records, typed enums, or a
hybrid. They must stay explainable.

## Social System Implications

RimWorld suggests that social simulation needs at least three layers:

```text
Relationship fact
  sibling, lover, rival, spouse, ex-lover, faction member

Directed opinion
  A's current attitude toward B

Social memory/thought
  A remembers B insulted them, saved them, betrayed them, killed kin
```

This is stronger than a faction-only model. It allows actor behavior to vary
based on personal history even when faction membership is identical.

For `world`, faction state and personal relationship state should probably be
separate:

- faction relation: village A distrusts village B
- actor relation: guard A likes merchant B
- belief: guard A falsely believes merchant B stole from the shrine
- memory: guard A saw merchant B near the shrine last night

The same action can then have different consequences for different observers.

## Storyteller And Scenario Implications

RimWorld is explicitly framed as a story generator. Its storyteller layer is
valuable because it separates actor autonomy from dramatic pacing.

For `world`, this suggests a non-core but important layer:

```text
ScenarioDirector
  observes allowed summary of world state
  proposes incidents
  respects pacing, seed, history, and content version
  schedules pressure through normal simulation records
```

Examples:

- a traveling merchant arrives because the region needs trade pressure
- a rumor appears after a witnessed crime
- a faction patrol enters because reputation fell
- a storm threatens an outdoor ritual
- a wounded stranger arrives carrying partial knowledge

This layer should be deterministic and auditable. It should produce proposals
or scheduled incidents, not hidden mutations.

## Agent And Actor Interface Implications

An AI agent inspired by RimWorld should not receive omniscient world state. It
also should not receive only a renderable view.

Useful agent input might include:

```text
AgentTurnInput
  actor_id
  current_task
  task_progress
  urgent_needs
  active_thoughts
  relevant_memories
  mood_or_stability
  body_capacities
  work_constraints
  known_social_relations
  local_affordances
  standing_requests
  candidate_tasks
  unavailable_tasks_with_reasons
```

The important addition beyond the earlier `Agent Interface` note is
`candidate_tasks` and `unavailable_tasks_with_reasons`. RimWorld's value is
less about giving an agent a list of atomic moves, and more about making the
reasoning layer between standing pressure and concrete action visible.

## Representation Pressure

RimWorld pressures the model in several directions:

- actor needs must be state, not flavor
- jobs/tasks need progress, targets, reservations, and interruption rules
- work surfaces should create standing action opportunities
- memories and thoughts need source, duration, and behavioral effect
- social relationships need directed opinion and relationship facts
- body injury should affect both capability and motivation
- incidents need a source outside ordinary actor intent but inside replay
- UI/debug output should explain actor behavior from state and rules

## Transferable Principles

- Treat actor motivation as explicit, inspectable state.
- Separate high-level tasks from low-level action requests.
- Let needs and memories apply pressure over time instead of instantly forcing
  all behavior.
- Preserve the source of each mood, thought, belief, or social modifier.
- Model social state at both personal and faction levels.
- Let world objects and configured areas create standing work opportunities.
- Use reservations or equivalent claims to avoid multi-actor task conflicts.
- Keep storyteller/director pressure outside core truth mutation.

## Rejected Or Risky Patterns

- Do not copy RimWorld's colony-management UI as the core `world` interface.
- Do not reduce actor psychology to opaque mood arithmetic if beliefs,
  knowledge, and explanation matter more.
- Do not let a storyteller layer override simulation truth.
- Do not make player-authored priorities the only form of actor motivation.
- Do not assume every actor needs full colonist-level needs and mood systems.
- Do not copy the exact thought catalog or comedic severity values as design
  requirements.
- Do not let long jobs hide granular action/event records needed for replay.

## Open Questions For `world`

- Does `world` need a first-class `Task` or `Job` layer between agent choice and
  `ActionRequest`?
- Which actor pressures should be continuous needs, and which should be
  event-derived thoughts or memories?
- Should mood/stability be a generic aggregate, or should actors reason from
  individual active pressures?
- How should a task reserve actors, objects, locations, and resources?
- Can a scenario director be deterministic, replayable, and still surprising?
- How much of an actor's motivation should be exposed to human UI and AI-agent
  input?
- Should standing world requests, such as bills or zones, exist independently
  of any actor?
- What makes an actor temporarily lose ordinary control, and how is that
  represented as an event?

## Extracted Design Notes

### Keep

- Actor behavior should be explainable from needs, constraints, memory, body,
  social state, and available work.
- A high-level job/task layer is useful for multi-step behavior.
- Needs and thoughts are a practical bridge from past events to future action.
- Directed social opinion is different from faction reputation.
- Story pressure can be a separate deterministic layer that schedules incidents.
- Debuggability requires visible causes, not only final actions.

### Adapt

- Adapt RimWorld jobs into `Task` records that submit deterministic
  `ActionRequest`s.
- Adapt thoughts into structured actor-local pressures with source, duration,
  intensity, and effect.
- Adapt schedules and work priorities into broader standing goals,
  obligations, taboos, and constraints.
- Adapt bills and zones into world-authored or player-authored standing
  requests.
- Adapt mental breaks into explicit control-state transitions with event
  records.

### Avoid

- Avoid treating RimWorld's exact pawn-management layer as mandatory.
- Avoid making social systems only a table of numeric opinion modifiers.
- Avoid hidden pacing logic that cannot be replayed or inspected.
- Avoid letting task selection bypass action validation.

## Sources

- Official site: https://rimworldgame.com/index.php?lang=en
- RimWorld Wiki, AI Storytellers:
  https://rimworldwiki.com/wiki/AI_Storytellers
- RimWorld Wiki, Work:
  https://rimworldwiki.com/wiki/Work
- RimWorld Wiki, Menus:
  https://rimworldwiki.com/wiki/Menus
- RimWorld Wiki, Needs:
  https://rimworldwiki.com/wiki/Needs
- RimWorld Wiki, Mood:
  https://rimworldwiki.com/wiki/Mood
- RimWorld Wiki, Thoughts:
  https://rimworldwiki.com/wiki/Thoughts
- RimWorld Wiki, Mental break:
  https://rimworldwiki.com/wiki/Mental_break
- RimWorld Wiki, Social:
  https://rimworldwiki.com/wiki/Social
- RimWorld Wiki, Health:
  https://rimworldwiki.com/wiki/Health
- RimWorld Wiki, Capacity:
  https://rimworldwiki.com/wiki/Capacity
- RimWorld Wiki, Zone/Area:
  https://rimworldwiki.com/wiki/Zone/Area
- RimWorld Wiki, Bill:
  https://rimworldwiki.com/wiki/Bill
- RimWorld Wiki, Example Mending Job:
  https://rimworldwiki.com/wiki/Modding_Tutorials/Code_MendingJob
- RimWorld Wiki, XML Defs:
  https://rimworldwiki.com/wiki/Modding_Tutorials/XML_Defs
