# Epistemic State

## Status

Current design draft.

This document defines the first stable design for actor-relative information.
It is not a final implementation spec. The main storage model, boundaries, and
terminology are settled enough to use; scoring formulas, decay rules, and
presentation details remain deferred.

## Source Research

- [Epistemic State / Agent Memory](../research/epistemic-state-and-agent-memory.md)
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
- [World Model](world-model.md)
- [Perception And Observation](perception-and-observation.md)
- [Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md)
- [Social Institutional Model](social-institutional-model.md)
- [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md)
- [Intent Templates And Planning](intent-templates-and-planning.md)
- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)
- [Knowledge, History, And Belief](../ideas/knowledge-history-and-belief.md)

## Purpose

The epistemic state system represents what a holder can remember, believe,
know, suspect, retrieve, disclose, or use as learned information.

It exists because the game must distinguish:

```text
hard truth
  what is actually true in authoritative world state

perception
  what a holder currently observes

epistemic state
  what a holder retains, believes, knows, has heard, inferred, or can retrieve

semantic appraisal
  what that information means to the holder

intent
  what the holder is trying to do
```

The core rule:

```text
Belief is not truth.
Memory is not current state.
Knowledge is holder-relative.
Rumor needs provenance.
Secret needs access and disclosure state.
```

Epistemic mechanics are reusable actor-truth substrate. Concrete learned
procedure, recipe, spell, ritual, legal form, combat technique, social form,
and lore vocabularies may be supplied by game-system packs when they are
specific to a world or rule family.

## Position In The Engine

Epistemic state sits after observation projection and before semantic
appraisal.

```text
EventRecord / WorldState
  -> perception and observation projection
  -> ObservedEvent / ObservedState
  -> EpistemicRecord creation or update
  -> WorkingSet / Epistemic views
  -> Semantic Appraisal And Motivation
  -> Intent Templates And Planning
  -> ActionRequest
  -> Causal Runtime
```

Epistemic state may affect future behavior, but it does not select actions
directly. It supplies the actor-relative information that later layers
interpret.

## Boundary

Epistemic state owns:

- persistent actor-relative information records
- holder-relative memory, belief, knowledge, rumor, secret, and learned
  procedure views
- source/provenance chains
- confidence, salience, freshness, access, and disclosure metadata
- retrieval-time accessibility
- working-set construction for actors and AI agents
- contradiction and supersession links between information records
- gameplay-relevant AI-agent memory accepted by the engine

Epistemic state does not own:

- hard physical state
- social truth itself
- action validation
- typed effect execution
- appraisal into thought, pressure, or goal
- intent-template binding or scoring
- natural-language presentation as the authoritative record
- agent-private notes that do not affect gameplay

## Core Design

Use a single typed `EpistemicStore` with holder-relative
`EpistemicRecord`s.

Do not create peer stores like this:

```text
MemoryStore
BeliefStore
KnowledgeStore
RumorStore
SecretStore
```

Those concepts share too much metadata: holder, source, confidence, staleness,
access, and contradiction. Splitting them too early causes duplication and
glue code.

Do not use one untyped fact graph either:

```text
Fact(subject, predicate, object, qualifiers)
```

The content must stay typed enough for rules, action schemas, debug tools, and
AI grounding.

Preferred shape:

```text
EpistemicStore
  records_by_holder
  records_by_content
  records_by_source_event
  records_by_subject
  records_by_source_actor
  disclosure_index
```

The [World Model](world-model.md) hosts `EpistemicStore` as the actor-truth
store family. This document owns `EpistemicRecord` meaning, holder semantics,
persistence rules, access, confidence, salience, contradiction, and retrieval
behavior.

Actors, factions, cultures, and places do not directly own large memory
arrays. They are holders that records point to.

Writes use the epistemic persistence gate:

```text
ObservedEvent / testimony / inference / AI memory proposal
  -> epistemic persistence gate
  -> AcceptedEpistemicUpdate
  -> EpistemicStore
```

An `AcceptedEpistemicUpdate` may reference hard `EventRecord`s, soft
`ChronologyStore` records, `SocialClaim`s, or prior `EpistemicRecord`s as
provenance. It does not mutate those source records.

## Minimal Design Surface

Do not treat the whole conceptual model as the required baseline surface.

The minimal stable surface is:

```text
EpistemicStore indexes:
  records_by_holder
  records_by_content
  records_by_source_event

EpistemicRecord fields:
  id
  holder
  content
  mode
  provenance
  confidence
  salience
  freshness
  access
  created_at
  updated_at

Content:
  EventRecordRef
  HistoricalEventRef
  Proposition
  LocationRef
  SocialClaimRef
  ProcedureRef

Mode:
  remembered
  believed
  known
  rumored

Access:
  public
  private
  restricted
```

Use coarse values for confidence, salience, and freshness:

```text
low | medium | high
```

Defer these from the minimal design surface unless a concrete scenario forces
them:

- `secret` as a primary mode
- detailed accessibility decay
- evidence bundles
- generated text records
- disclosure index
- contradiction graph
- AI prompt/model provenance

Additional indexes should be added only when a real query requires them.

## Holder Model

An epistemic holder is an entity or abstract group that can possess
actor-relative information.

Initial holder kinds:

```text
Actor
Faction
Culture
Place
Institution
Party
```

Examples:

```text
Actor:
  a guard remembers seeing the player near a corpse.

Faction:
  a thieves' guild knows a tunnel route.

Culture:
  a village culture preserves an account of an old betrayal.

Place:
  a tavern has a local rumor ecology.

Institution:
  a temple records a taboo and initiation rite.

Party:
  the player's group shares a discovered map location.
```

Rules:

- A record has exactly one primary holder.
- The same content can be held by many holders through separate records.
- Shared knowledge is represented by group/faction/culture/party holders, not
  by duplicating it eagerly onto every member.
- Individual actors may inherit or query group knowledge through context views.
- Holder does not imply subject or source.

## Holder, Subject, And Source

Keep these separate:

```text
holder:
  who has the information.

subject:
  who or what the information is about.

source:
  where the holder got the information.
```

Example:

```text
EpistemicRecord
  holder: guard_1
  subject: merchant_1
  content: Proposition(merchant_1 bought rare poison)
  mode: rumored
  source_actor: rival_1
  confidence: medium
```

This says:

- the guard holds the record
- the merchant is the subject
- the rival is the source

It does not say the merchant owns the information.

## Record Shape

Conceptual shape:

```text
EpistemicRecord
  id: EpistemicRecordId
  holder: EpistemicHolder
  content: EpistemicContent
  mode: EpistemicMode
  provenance: EpistemicProvenance
  confidence: Confidence
  salience: Salience
  freshness: Freshness
  accessibility: Accessibility
  access: AccessPolicy
  links: EpistemicLinks
  created_at: SimTime
  updated_at: SimTime
```

This is not the final serialized schema. It defines the long-term shape the
system should leave room for.

The minimal design surface should start with the smaller record:

```text
EpistemicRecord
  id
  holder
  content
  mode
  provenance
  confidence
  salience
  freshness
  access
  created_at
  updated_at
```

Defer:

- exact accessibility model
- contradiction/supersession link graph
- disclosure index
- evidence bundles
- generated text records
- AI prompt/model provenance fields

## Content

Content is what the record is about. It should be typed where gameplay depends
on it.

Initial content forms:

```text
EventRecordRef:
  reference to a committed `EventRecord`.

HistoricalEventRef:
  reference to provenance-backed worldgen, authored history, or AI-accepted
  soft chronology. It is not hard causal truth unless later materialized
  through the causal runtime.

LocationRef:
  reference to a place, map region, route, hidden entrance, or site.

SocialClaimRef:
  reference to a social/institutional claim: ownership, right, office,
  permission, obligation, debt, oath, rank, or accusation.

Proposition:
  typed propositional content when no existing event or relation is enough.

ProcedureRef:
  recipe, ritual, spell form, combat technique, social form, legal form,
  password, or learned action schema.
```

Defer these until a concrete scenario needs them:

```text
EntityRef:
  reference to an actor, object, item, artifact, monster, plant, or other
  entity. This may often be a subject rather than record content.

EvidenceBundleRef:
  collection of event refs, item refs, testimony refs, or text refs used as
  evidence.

GeneratedTextRef:
  prose summary, rumor wording, journal entry, tale, or AI-authored phrasing
  that points back to typed content.
```

Rules:

- Natural language may be attached, but it should not be the only
  gameplay-relevant content.
- If content can affect capability derivation, perceived affordance, validation,
  intent input, or appraisal, it needs a typed form.
- `Proposition` is allowed, but it should not become a stringly escape hatch
  for every fact.

## Modes And Access

Mode describes the holder's relationship to the content.

Stored modes should remain deliberately small in the baseline design:

```text
remembered:
  retained from past observation, testimony, reading, inference, or experience.

believed:
  treated by the holder as plausibly true. Can be false.

known:
  stable or verified enough for the holder to use as reliable information.

rumored:
  socially transmitted and uncertain, with source chain.
```

Access is separate from mode:

```text
public:
  broadly shareable or socially public for the holder.

private:
  held by the holder but not generally shared.

restricted:
  sensitive, secret, taboo, illegal, valuable, or dangerous to disclose.
```

Conceptual mode labels that may become later facets or derived views:

```text
observed:
  current or recent perception. Usually belongs to perception/observation, not
  persistent epistemic state unless retained as memory.

secret:
  better treated initially as restricted access plus disclosure state, not as
  a primary stored mode.

inferred:
  derived from other records rather than directly observed.

taught:
  intentionally transferred as instruction, doctrine, recipe, or technique.

reflected:
  summarized or interpreted from other records, possibly AI-assisted.

forgotten:
  retained only as inaccessible or debug/audit state.
```

Important:

- Modes are not final enum names.
- Some modes may later become derived views rather than stored values.
- A record may need multiple tags or mode facets later. For now, keep one
  primary `mode` and a separate `access` field.

## Memory, Belief, Knowledge, Rumor, Secret

These should be understood as views or mode families over `EpistemicRecord`,
not as peer systems.

### Memory

Memory is retained information from prior experience, observation, testimony,
reading, or inference.

Memory can be:

- stale
- incomplete
- false if the original source was mistaken
- emotionally salient
- inaccessible unless cued

Example:

```text
EpistemicRecord
  holder: player
  content: EventRecordRef(actor_died_456)
  mode: remembered
  provenance.source_event: actor_died_456
  confidence: high
  salience: high
```

### Belief

Belief is content the holder treats as plausibly true.

Belief can come from:

- direct memory
- rumor
- inference
- authority
- culture
- fear
- deception
- AI-proposed actor truth accepted as holder-relative belief

Belief must not be collapsed into hard truth.

Example:

```text
EpistemicRecord
  holder: guard_1
  content: Proposition(merchant_1 poisoned priest_1)
  mode: believed
  confidence: medium
  provenance.source_record: rumor_77
```

### Knowledge

Knowledge is information stable enough for the holder to use.

Examples:

- known map location
- known password
- known recipe
- known law
- known monster weakness
- known name
- known faction symbol
- known ritual form

Open point:

`known` may eventually be derived from confidence, verification, source
authority, and content kind instead of stored as a mode.

### Rumor

Rumor is socially transmitted uncertain information.

Required structure:

```text
holder
content
source_actor or source_group
source_chain
confidence
freshness
transmission_event?
```

Rumor is not weak truth. It is a sourced actor-relative record.

### Secret

Secret is restricted information whose access creates value, risk, or leverage.

Useful distinction:

```text
SecretContent:
  the underlying content that is restricted or sensitive.

KnownSecret:
  a holder has access to that content.

Leverage:
  a holder can use a known secret in a social action.
```

Open point:

`secret` should not be an initial primary stored mode. Start with `access:
restricted` plus content sensitivity and disclosure records. A later design can
promote secret into a wrapper type if needed.

### Procedure

Procedure-like knowledge is partly epistemic and partly capability-related.

Examples:

- recipe
- ritual
- spell form
- combat technique
- legal form
- social script
- lockpicking method
- monster-hunting tactic

Rule:

Epistemic state records that the holder knows or remembers the procedure.
Actor-owned capability derivation decides whether that procedure grants or
modifies an action schema.

## Claim Terminology

Do not use plain `Claim` in design docs.

Use:

```text
SocialClaim:
  social, legal, institutional, or customary assertion such as ownership,
  right, debt, rank, office, permission, obligation, taboo, oath, or accusation.

Proposition:
  content that can be believed, rumored, inferred, or disputed.

Belief:
  a holder-relative epistemic relation to content.
```

Examples:

```text
SocialClaim:
  merchant_1 owns dagger_1.

Belief:
  guard_1 believes merchant_1 owns dagger_1.

Rumor:
  villagers say merchant_1 stole dagger_1.
```

If an actor believes a `SocialClaim`, represent it as an epistemic record over
`SocialClaimRef`, not as a separate core concept.

## Provenance

Every gameplay-relevant epistemic record needs provenance.

Minimum provenance:

```text
EpistemicProvenance
  source_event?
  source_record?
  source_actor?
  source_faction?
  source_item?
  source_text?
  source_process?
  channel: sight | sound | smell | touch | speech | book | ritual |
           inference | dream | magic | AI | worldgen
  source_chain?
  created_by: rule | worldgen | AI | actor_action | import
```

Provenance is required for:

- rumor source chains
- lies and misinformation
- trust-sensitive belief updates
- false accusations
- social blame
- AI grounding
- debug explanation
- replay and save/load audits

## Confidence, Freshness, Salience, Accessibility

These are separate concepts. They do not all need precise numeric formulas in
the first implementation.

```text
confidence:
  how strongly the holder treats the content as usable or true.

freshness:
  how current the information may be.

salience:
  how important the information is to the holder.

accessibility:
  how likely the information is to be retrieved in a given context. This can be
  derived during retrieval at first.
```

Examples:

```text
high confidence, low freshness:
  "I clearly saw the guard at the gate ten turns ago."

low confidence, high salience:
  "Someone said my brother might be alive."

high salience, low accessibility:
  a traumatic memory not currently cued.
```

Initial implementation can use coarse buckets:

```text
low | medium | high
```

Do not introduce detailed decay math until retrieval behavior needs it.

## Creation Flow

Do not create an `EpistemicRecord` for every event.

Expected flow:

```text
EventRecord
  -> observation projection
  -> ObservedEvent per observer
  -> persistence gate
  -> EpistemicRecord create/update
```

Example:

```text
EventRecord:
  ActorDied(victim=mentor_1, cause_actor=bandit_1)

ObservedEvent:
  observer: player
  perceived_roles:
    victim: mentor_1
    cause_actor: bandit_1
  channel: sight
  confidence: high

EpistemicRecord:
  holder: player
  content: EventRecordRef(ActorDied)
  mode: remembered
  confidence: high
  salience: high
```

The event log remains authoritative evidence. Epistemic state records the
holder-relative retention or belief.

## Persistence Gate

An observation, message, inference, or generated account should persist only
when it can matter later.

Initial conservative gate:

- it can affect later action repertoire through capability derivation
- it can affect perceived affordance or expected consequence
- it can affect relationship, trust, fear, duty, debt, resentment, gratitude,
  suspicion, grief, or revenge
- it is a secret, clue, password, recipe, route, law, taboo, name, ritual, map,
  or weakness
- it can become rumor, testimony, accusation, blackmail, teaching, trade, or
  warning
- it explains a local or historical situation
- it is attached to the protagonist, a major NPC, active local context, or an
  important faction/place

Broader gates such as "AI-agent context" or "narrative salience" are allowed,
but they should be implemented through explicit policies. Do not use them as a
generic reason to persist everything.

Usually do not persist:

- low-salience repeated movement
- transient visibility changes
- obvious current-state facts that can be queried directly
- social chatter with no source value, consequence, or future query
- generic environmental observations with no active holder concern
- facts that no holder, system, or future query can use

## Record Update

New information should not blindly overwrite old information.

Possible outcomes:

```text
create:
  no related record exists.

reinforce:
  new evidence supports an existing record.

revise:
  new evidence changes confidence, freshness, mode, or content.

contradict:
  new evidence conflicts but both records remain useful.

supersede:
  old information is retained for explanation but no longer active.

merge:
  several low-level records become a summary or reflection.
```

Examples:

```text
Rumor:
  merchant bought poison.

Evidence:
  poison bottle found in merchant's locked chest.

Update:
  reinforce or revise belief; retain rumor source.
```

```text
Memory:
  guard was near the gate ten turns ago.

Current observation:
  guard is now in the market.

Update:
  do not delete memory; mark old location stale or superseded for current
  location queries.
```

## Retrieval And Working Set

The store may contain many records. An actor or AI agent should not receive all
of them every turn.

Use retrieval to produce a bounded working set.

Retrieval cues:

- current place
- visible actors
- current goal or pressure
- recent event
- topic of conversation
- object being inspected
- danger or opportunity
- social role or authority
- emotional trigger
- AI-agent query

Ranking inputs:

- relevance to cue
- confidence
- freshness
- salience
- accessibility
- source trust
- current holder state
- relation to active goal or threat

Output:

```text
WorkingSet
  active_records
  relevant_beliefs
  relevant_known_procedures
  relevant_secrets
  recent_observations
  uncertainty_notes
```

The working set is an actor-facing view, not a separate source of truth.

## Actor-Facing Query Flow

When an actor updates intent or chooses an action, it should not scan all
records directly.

Use this shape:

```text
ActorContext
  actor state
  current perception
  current place
  visible entities
  current goal / pressure
  recent events or conversation topic

EpistemicRetrieval
  holder = actor
  inherited holders = party / faction / culture / place if accessible
  cues = context entities, place, topics, goals, threats
  access filter
  relevance ranking
  working-set limit

WorkingSet
  relevant actor-facing records for appraisal and planning
```

The query answers what the actor can currently remember, believe, know, or
retrieve that matters in context. It does not decide what the actor should do.

## Views

The system should expose typed query views over records.

Initial views:

```text
EpisodeMemoryView:
  event-like memories with time, place, participants, source, confidence,
  salience, and freshness.

BeliefView:
  propositions or content the holder treats as plausible.

KnowledgeView:
  stable, verified, or rule-usable information available to the holder.

RumorView:
  socially transmitted information with source chain and confidence.

SecretView:
  records with restricted access or sensitive content, plus disclosure state.

ProcedureView:
  learned recipes, rituals, routes, tactics, spells, techniques, and forms.

WorkingSetView:
  compact current input for NPC policy, player aid, or AI-agent context.

DebugProvenanceView:
  explanation chain for why a holder believes or remembers something.
```

## Secrecy And Disclosure

Secret state needs both content and access.

Initial shape:

```text
SecretContent:
  content that is sensitive in some context.

KnownSecret:
  holder-relative epistemic record with `access: restricted` whose content is
  sensitive in the current social context.

Disclosure:
  event or record showing that a holder shared, exposed, sold, taught, or lost
  control of the secret.

Leverage:
  derived social possibility created by known secret + audience + norm.
```

Example:

```text
content:
  noble_1 belongs to illegal_cult_7.

holder:
  spy_1

access:
  restricted

possible downstream use:
  blackmail, accusation, sale, protection, warning, confession.
```

`Leverage` belongs primarily to semantic/social appraisal and intent planning,
but it depends on epistemic state.

## Rumor And Testimony

Rumor and testimony are created by social information transfer.

Example flow:

```text
SpeechAction:
  rival_1 tells guard_1 that merchant_1 bought poison.

EventRecord:
  InformationTransferred(speaker=rival_1, listener=guard_1, topic=...)

EpistemicRecord:
  holder: guard_1
  content: Proposition(merchant_1 bought poison)
  mode: rumored
  provenance.source_actor: rival_1
  provenance.source_event: InformationTransferred
  confidence: medium
```

Rumor quality should depend on source, context, and later evidence. The exact
formula is deferred.

## Relationship To Perception

Perception is current actor-relative sensing.

Epistemic state is retained actor-relative information.

```text
Perception:
  I currently see a blood trail.

Epistemic state:
  I remember seeing the bandit leave north.

Perception:
  I hear someone shouting behind the door.

Epistemic state:
  I believe a fight happened in the house.
```

Only salient or useful perception should become persistent epistemic state.

## Relationship To World Model

[World Model](world-model.md) owns the storage substrate and query indexes for
`EpistemicStore`. Epistemic state owns the actor-truth semantics.

This split is intentional:

```text
World Model:
  where `EpistemicRecord`s are stored, indexed, referenced, and queried.

Epistemic State:
  what `EpistemicRecord`s mean, when they are created, how they are retrieved,
  and how holder-relative truth can diverge from hard truth.
```

Actor-relative query surfaces may retrieve from `EpistemicStore`, but they
must preserve holder, access, confidence, and provenance. Querying a record for
an actor is not the same as making the record true in hard world state.

## Relationship To Semantic Appraisal

Epistemic state provides inputs. Semantic appraisal assigns meaning.

```text
Epistemic state:
  player remembers bandit_1 killed mentor_1.

Context:
  mentor_1 is a close relation.

Semantic appraisal:
  close relation harmed by known aggressor.

Outputs:
  grief thought
  retaliation pressure
```

Epistemic state must not create pressure directly. It supplies inputs to
appraisal; appraisal creates `Thought`, `Pressure`, or `GoalPressure`.

## Relationship To Intent And Action

Epistemic state can change action behavior through explicit downstream
surfaces:

- action repertoire: known recipe, ritual, password, route, procedure
- perceived affordance: recognized seal, weakness, trap sign, social opening
- expected consequence: believed law, taboo, risk, punishment, promise
- intent input: goal-relevant memory or belief in working set
- dialogue: topic, accusation, confession, lie, teaching, rumor, blackmail

It should not directly choose actions.

```text
Epistemic state:
  knows bandit_1 killed mentor_1

Semantic appraisal:
  creates retaliation pressure

Intent planning:
  binds generic templates such as LocateActor or AskInformationSource

Action:
  Speak(to=villager_1, topic=bandit_1)
```

## AI-Agent Boundary

The engine owns gameplay-relevant epistemic state.

AI agents may maintain private notes for style or continuity. Those notes do
not count as game state until imported through an accepted record or event.

AI may propose:

- reflection summaries
- memory compression
- belief updates
- rumor wording
- natural-language recall
- self-model notes
- retrieval queries

Accepted proposals must become explicit records with provenance:

```text
created_by: AI
model?
prompt_context_hash?
source_records
accepted_by
created_at
```

Rules:

- AI memory does not mutate hard truth.
- AI reflection may affect behavior only through accepted epistemic records and
  downstream appraisal.
- Agent-private notes must not unlock actions, establish facts, or justify
  social consequences.

## Multi-Resolution Policy

Epistemic state must scale across simulation distance. The resolution policy is
owned by [Multi-Resolution Simulation](multi-resolution-simulation.md); this
section records epistemic constraints.

Suggested resolution levels:

```text
Protagonist and major recurring NPCs:
  detailed records, beliefs, secrets, procedure knowledge, and working-set
  retrieval.

Nearby ordinary NPCs:
  limited records, local rumors, relevant beliefs, and small working set.

Faction / culture / place:
  aggregate beliefs, public rumors, known secrets, laws, taboos, recipes, and
  history accounts.

Distant simulation:
  abstract knowledge and rumor state, materialized into detailed records only
  when interaction pulls it closer.
```

Do not eagerly instantiate full personal memories for every distant actor.

## Complexity Controls

The system should remain expressive without becoming an infinite memory
database.

Controls:

- persistence gate before record creation
- salience and relevance thresholds
- holder-specific storage budgets
- retrieval by cue rather than full scans
- working-set size limits
- eventual accessibility decay instead of deleting evidence
- group holders for shared or distant information
- summary records for repeated or old episodes
- typed content references
- provenance links to event history and accepted AI proposals
- deferred materialization for distant actors

## Hardcoding Boundary

The epistemic layer should not contain story-specific behavior rules.

Avoid:

```text
if mentor_1 killed by bandit_1:
  create revenge memory
  make player pursue bandit
```

Allowed here:

- content schemas
- holder/source/provenance recording
- conservative persistence policies
- record reinforcement, revision, contradiction, and supersession
- retrieval into a working set

Belonging to later layers:

- whether the memory means grief, duty, suspicion, revenge, fear, or shame
- whether a pressure creates `LocateActor`, `AskInformationSource`, or
  `ConfrontActor`
- how an NPC scores competing intents

This keeps epistemic state as reusable information infrastructure rather than
a quest-scripting layer.

## Examples

### Witnessed Death

```text
EventRecord:
  ActorDied(victim=mentor_1, cause_actor=bandit_1)

ObservedEvent:
  observer=player, channel=sight, confidence=high

EpistemicRecord:
  holder=player
  content=EventRecordRef(ActorDied)
  mode=remembered
  confidence=high
  salience=high
```

Downstream, semantic appraisal may create grief and retaliation pressure.

### False Rumor

```text
EventRecord:
  InformationTransferred(rival_1, guard_1, topic=merchant_poison)

EpistemicRecord:
  holder=guard_1
  content=Proposition(merchant_1 bought poison)
  mode=rumored
  provenance.source_actor=rival_1
  confidence=medium
```

The rumor can later support a belief, suspicion pressure, or accusation without
becoming hard truth.

### Stale Location Memory

```text
EpistemicRecord:
  holder=thief_1
  content=Proposition(guard_1 was at north_gate at t-10)
  mode=remembered
  created_at=t-10
  freshness=low
```

The thief may still act on it, but current truth should be queried separately
when resolution occurs.

### Known Secret

```text
EpistemicRecord:
  holder=spy_1
  content=Proposition(noble_1 belongs to illegal_cult_7)
  mode=known
  access=restricted
  provenance.source_item=stolen_letter_3
```

Semantic/social systems can later derive leverage depending on audience and
law.

### Known Procedure

```text
EpistemicRecord:
  holder=player
  content=ProcedureRef(blue_rain_passphrase)
  mode=known
  provenance.source_actor=old_merchant_1
```

Capability derivation may let the actor use a passphrase action schema at the
matching gate.

## Stable Decisions

- [World Model](world-model.md) hosts `EpistemicStore`; this document owns
  `EpistemicRecord` semantics and lifecycle.
- `EpistemicRecord` is holder-relative.
- `holder`, `subject`, and `source` are separate.
- `Memory`, `Belief`, `Knowledge`, `Rumor`, and `Secret` are views or mode
  families over records, not independent peer stores.
- Initial stored modes are `remembered`, `believed`, `known`, and `rumored`.
- `secret` starts as restricted access and disclosure state, not as a primary
  stored mode.
- `Reservation` is not epistemic state.
- `SocialClaim` is content or social state, not memory by itself.
- Plain `Claim` should not be used.
- Not every event creates epistemic state.
- Natural language can be attached, but typed content is required when gameplay
  depends on the information.
- Memory affects behavior through semantic appraisal, pressure, goal, intent,
  and action layers.
- AI can propose epistemic content, but accepted game-relevant memory needs
  provenance and an `AcceptedEpistemicUpdate`.
- Pack-owned procedure or lore vocabularies can appear as typed
  `EpistemicRecord` content, but accepted records still use the epistemic
  commit surface.

## Deferred Decisions

- final schema and serialization format
- extended mode/facet taxonomy
- whether `known` is stored or derived
- whether `secret` later becomes a wrapper type
- confidence math
- freshness and accessibility decay
- salience thresholds and storage budgets
- contradiction resolution details
- rumor reliability formula
- testimony and lie detection model
- procedure knowledge versus capability boundary
- faction/culture/place inheritance rules
- AI proposal acceptance policy
- player-facing memory UI
- debug explanation format

## Open Questions

- Which content forms should exist in the first implementation?
- How much actor memory should be materialized for distant simulation?
- Should group holders produce inherited views or copied records for members?
- What is the minimum retrieval model needed before AI-agent integration?
- How should contradictory beliefs be represented to players without exposing
  omniscient truth?
- How should memory summaries relate to original event evidence?
