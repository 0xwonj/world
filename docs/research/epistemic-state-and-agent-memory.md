# Epistemic State / Agent Memory

## Status

Draft research

## Axis

[Actor Perspective / Epistemic Interface](engine-architecture-entry.md), with
direct pressure on [Semantic / Social / Motivation Layer](engine-architecture-entry.md).

## Design Output

- [Epistemic State](../design/epistemic-state.md)

This research should inform later cleanup of [World Model](../design/world-model.md)
and [Perception And Observation](../design/perception-and-observation.md).

## Core Question

```text
How should actors hold, retrieve, revise, forget, share, hide, and act on
actor-relative information without becoming omniscient or uninspectable?
```

This includes:

- perception-derived memory
- remembered events and stale observations
- belief and false belief
- knowledge, names, recipes, laws, procedures, and map information
- rumors, secrets, and source chains
- actor identity and self-belief
- AI-agent memory interfaces
- social propagation and motivated interpretation

## Why This Must Be Researched Together

Epistemic state is the bridge between hard world facts and actor behavior.

It cannot be designed as a pile of isolated stores:

```text
Memory
Belief
Knowledge
Rumor
Secret
SocialClaim
Reservation
```

Those terms are not all the same kind of thing.

- `Reservation` is runtime conflict-control state, not epistemic state.
- `SocialClaim` is social/institutional content, not memory by itself.
- `Memory`, `Belief`, `Knowledge`, `Rumor`, and `Secret` are actor-facing
  information modes, views, or wrappers over content with provenance.
- `Procedure` and `Skill` are partly epistemic, but also feed the capability
  and action-schema model.
- `Thought`, `Pressure`, and `Intent` consume epistemic state, but should not be
  collapsed into it.

The coupled problem is:

```text
What does an actor think it knows, why does it think that, how accessible is
that information right now, and what can that information change?
```

If this is wrong, the game gets either omniscient actors, brittle hardcoded
story flags, infinite memory logs, or black-box AI behavior.

## Scope

In scope:

- actor-relative information records
- source/provenance, confidence, staleness, and contradiction
- memory retrieval and forgetting pressure
- social transmission of information
- secrets as restricted and socially valuable information
- actor-specific belief divergence from hard truth
- memory summaries, reflections, and AI-assisted interpretation
- what to expose to `AgentTurnInput`
- links to action repertoire, perceived affordance, pressure, and intent

Out of scope:

- final storage backend
- exact numeric confidence formula
- final social/reputation model
- full cognitive architecture simulation
- natural-language memory prompt design
- detailed UI presentation of memories and thoughts
- final AI coauthority policy

## Research Inputs

This pass used parallel lenses:

- game systems: RimWorld, Dwarf Fortress, Caves of Qud, Shadow of Mordor,
  Disco Elysium, Crusader Kings III, Prom Week, Versu, and Talk of the Town
- AI-agent memory: Generative Agents, MemGPT/Letta, Reflexion, Voyager, CoALA,
  MemoryBank, and recent memory surveys
- cognitive and social psychology: episodic/semantic/procedural memory,
  working memory, reconstructive memory, source monitoring, forgetting,
  emotion appraisal, motivated reasoning, trust
- knowledge representation: provenance models, named graphs, truth
  maintenance, belief revision, and non-monotonic reasoning pressure
- social simulation: agent-based models, culture propagation, emergent
  narrative, gossip, reputation, and community-scale social simulation

## Theory Baseline

### Functional Memory Types

Cognitive psychology strongly suggests not treating memory as one bucket.

Useful game-facing distinctions:

```text
Working memory:
  the tiny active set currently shaping attention and action.

Episodic memory:
  remembered event traces with time, place, participants, source, and affect.

Semantic memory:
  actor-relative claims, facts, concepts, names, laws, maps, meanings, and
  general knowledge.

Procedural memory:
  learned ways to do things: recipes, rituals, tactics, social forms, combat
  maneuvers, spells, habits, and action schemas.
```

Transfer:

- The model should be functional, not neurological.
- `world` needs small active working sets for AI-agent input.
- Episodic records should not be confused with current hard truth.
- Semantic beliefs may be compressed summaries of many episodes.
- Procedural memory should likely feed actor-owned capabilities and learned
  action schemas.

Do not over-transfer:

- Human brain categories are not storage schemas.
- A game does not need a detailed model of human cognition to produce believable
  social stories.

### Memory Is Reconstructive

Human memory is reliable enough to act on, but it is not a perfect event log.
Psychology references emphasize forgetting, source confusion, suggestibility,
bias, and intrusive persistence.

Design pressure:

- Store `source`, `confidence`, `channel`, `last_recalled`, and `evidence`.
- Rumor should be represented as actor-relative information with a source chain,
  not as downgraded global truth.
- Recalled memory can be biased by current goals, relationships, identity, and
  emotion.
- Strong or repeated memories can become more accessible, but the causal event
  record should remain separate and auditable.

Important distinction:

```text
EventHistoryStore:
  what was committed as hard or generated history.

Epistemic state:
  what some actor, faction, culture, or place remembers, believes, can retrieve,
  can communicate, or can act on.
```

### Appraisal Instead Of Mood Only

Emotion research points toward appraisal relative to goals, agency, blame,
control, certainty, novelty, and norms.

Transfer:

- A single `mood` value is too weak to explain rich social behavior.
- Events should be interpreted through actor context.
- Appraisal can create thoughts, pressure, and proposed salience updates for
  existing or future epistemic records.
- Different actors can remember the same event as rescue, betrayal,
  humiliation, justice, omen, debt, or threat.

This supports the existing pipeline:

```text
ObservedEvent
  -> actor-specific interpretation
  -> memory / belief update
  -> thought / pressure
  -> intent bias
  -> action request
```

### Trust Is Multidimensional

Social psychology separates trust in ability, benevolence, integrity, and
sometimes predictability or faith.

Design pressure:

- Avoid one universal `trust` number.
- Rich stories need distinctions such as:
  - I trust her courage, not her honesty.
  - I trust his craft, not his loyalty.
  - I believe she means well, but I do not trust her judgment.
- Trust should affect belief update, rumor propagation, secrecy, cooperation,
  fear, and social action risk.

### Belief Revision And Truth Maintenance

Formal belief revision and truth-maintenance systems are useful mainly as
vocabulary:

- beliefs can have justifications
- new information can contradict old information
- revision should preserve as much as possible while resolving conflict
- explanations need dependency links
- assumptions can be retracted or superseded

Do not over-transfer:

- Full AGM-style logically closed belief sets are too expensive and too brittle
  for a large RPG simulation.
- A complete TMS for every NPC is overkill.

Transfer:

- Store reasons, not only values.
- Contradiction should not silently overwrite older information.
- Use `supersedes`, `contradicts`, and `supported_by` links where behavior or
  explanation depends on them.
- Actor beliefs can remain inconsistent if that inconsistency is narratively or
  behaviorally useful.

### Provenance And Named Contexts

W3C PROV and RDF dataset/named-graph work are not game designs, but they give a
clean pressure point:

```text
Do not store only a claim. Store the context in which the claim was produced,
who or what produced it, and what activity or evidence generated it.
```

Transfer:

- `source_event`, `source_actor`, `source_text`, `source_process`, `created_at`,
  and `confidence` matter.
- The same content can appear in different contexts:
  - hard event record
  - actor memory
  - rumor heard from another actor
  - faction secret
  - AI-proposed actor truth or accepted soft truth
  - generated historical account
- Provenance should be queryable for debugging and AI-agent grounding.

Do not over-transfer:

- RDF triples and named graphs should not become the hard-truth core.
- Stringly predicate graphs risk tag soup unless wrapped in typed schemas.

## Reference Observations

### Generative Agents

Observation:

Generative Agents keeps an experience stream, retrieves memories by relevance,
recency, and importance, synthesizes higher-level reflections, and uses those
memories for planning and social behavior.

Inference:

The believability comes less from perfect world simulation and more from
selective retrieval plus reflection. Raw observations are not enough; the agent
needs compressed interpretations.

Transfer:

Adapt. Use memory streams and reflection as actor-facing derived memory, but do
not make natural-language memory the authoritative state. Engine-owned records
need typed ids and provenance.

### MemGPT / Letta

Observation:

MemGPT frames agent memory as virtual context management across fast and slow
memory tiers. Letta operationalizes persistent agents with core memory, message
history, archival memory, and tool/state persistence.

Inference:

An AI agent needs a compact current context plus searchable long-term state.
The memory interface is as important as the memory store.

Transfer:

Adapt. `AgentTurnInput` should look like a curated working set, not a dump of
all actor memory. External AI agents may maintain private notes, but engine-owned
epistemic state should be the canonical game-facing memory.

### Reflexion

Observation:

Reflexion stores verbal lessons after failures and uses them to improve later
attempts without changing model weights.

Inference:

Failure memory is useful, but it can encode false causal explanations if not
grounded in engine evidence.

Transfer:

Adapt. Let agents propose reflections after failed actions, invalid attempts,
social mistakes, or combat defeats. Store the failed action, evaluator signal,
and reflection separately.

### Voyager

Observation:

Voyager grows a library of executable skills from successful experience and
retrieves them for later tasks.

Inference:

Not all memory is belief memory. Some long-term learning should become
procedural knowledge.

Transfer:

Adapt. Learned recipes, rituals, tactics, social scripts, and spell procedures
belong near capability/action-schema derivation. They still need provenance and
validation.

### CoALA

Observation:

CoALA organizes language agents around working, episodic, semantic, and
procedural memory with internal and external actions.

Inference:

This vocabulary fits `world` better than a single memory bag.

Transfer:

Keep as architectural vocabulary, not as a storage schema.

### Dwarf Fortress

Observation:

Dwarf Fortress has thoughts, short-term memories, long-term memories, core
memories, personality changes, values, needs, rumors, historical records, and
investigable world history.

Inference:

The important pattern is memory lifecycle. Not every experience becomes a
permanent actor fact. Strong memories can persist and later change personality
or behavior.

Transfer:

Adapt. Use promotion, salience, and forgetting/retrieval rules so actor memory
does not become an infinite behavior-driving log.

### RimWorld

Observation:

RimWorld turns needs and experienced situations into thoughts, mood, mental
break risk, relationships, and story pressure.

Inference:

Memory-like state is strongest when it has visible behavioral consequences and
player-readable explanations.

Transfer:

Adapt. `Memory -> Thought/Pressure -> IntentBias/BehaviorRisk` is a useful
shape, but `world` should keep the protagonist's agency and avoid forcing
actions too often.

### Caves of Qud

Observation:

Caves of Qud treats secrets, histories, locations, faction knowledge, and water
ritual exchanges as gameplay-relevant knowledge. Sultan histories create
cultural artifacts, sites, relics, and discoverable narrative context.

Inference:

Knowledge can be loot, currency, social access, map access, and historical
explanation.

Transfer:

Keep. `Secret` should not be a UI note. It should wrap content with access,
source, category, audience, value, and disclosure history.

### Talk Of The Town

Observation:

Talk of the Town explicitly models character mental models. Characters observe
perceptible attributes, propagate beliefs socially, misremember, forget, and
track belief histories.

Inference:

Rich character depth can be expressed more through belief structures than
through physical world size.

Transfer:

Adapt strongly. Mental models should be typed enough to support play, but the
documented parameter complexity is a warning against over-modeling every
attribute for every actor.

### Prom Week And Versu

Observation:

Prom Week and Versu treat social interaction as playable state manipulation.
Versu uses social practices and reactive joint plans; Prom Week uses social
rules over traits, relationships, histories, and statuses.

Inference:

Social memory needs typed predicates and action-facing effects, not only prose.

Transfer:

Adapt. Start from a small social ontology: relation, obligation, grievance,
secret, rumor, trust, fear, debt, taboo, role, permission, and reputation.

### Shadow Of Mordor / Nemesis

Observation:

Recurring enemies remember encounters, change rank or traits, taunt the player,
and make failure narratively productive.

Inference:

Memory becomes powerful when the actor performs it back to the player through
dialogue, tactics, scars, titles, or changed confidence.

Transfer:

Adapt the principle, not the patented structure. In a single-protagonist RPG,
fewer deeper recurring actors are likely more valuable than many shallow
memory-bearing NPCs.

### Disco Elysium

Observation:

Disco Elysium treats thoughts, identity commitments, copotypes, internal voices,
and self-interpretation as mechanically relevant.

Inference:

The protagonist's epistemic state is not only world knowledge. It includes
self-belief, ideology, obsession, shame, memory recovery, and identity.

Transfer:

Adapt. `world` should reserve room for protagonist self-model and identity
commitments without forcing all NPCs to run the same detailed internal model.

### Social Simulation References

Observation:

Schelling, Axelrod, and Sugarscape show that simple local rules can produce
macro social patterns such as segregation, polarization, trade, migration,
cultural regions, and conflict.

Inference:

Social richness does not require every actor to reason deeply. Local rules,
bounded information, social influence, and environment pressure can create
large-scale dynamics.

Transfer:

Keep the principle. Social propagation and faction/culture drift can often be
lightweight, especially outside the protagonist's local interaction radius.

## Candidate Models

### Model A: Separate Stores For Every Term

Sketch:

```text
MemoryStore
BeliefStore
KnowledgeStore
RumorStore
SecretStore
```

Makes easy:

- straightforward feature implementation
- simple UI labels
- local tuning per feature

Makes hard:

- provenance duplication
- conversion between rumor, belief, knowledge, and secret
- contradiction handling
- deciding where a record lives when it is both remembered and secret
- keeping belief-over-`SocialClaim`, `SocialClaim`, and `Secret` boundaries
  clean

Likely failure:

Feature silos and glue code.

### Model B: One Generic Fact Graph

Sketch:

```text
Fact(subject, predicate, object, qualifiers)
```

Makes easy:

- arbitrary new content
- flexible queries
- AI-authored facts

Makes hard:

- type safety
- mutation authority
- distinguishing hard truth, rumor, belief, memory, and soft truth
- debugging rule firing
- preventing stringly predicates

Likely failure:

Tag soup and hidden gameplay rules.

### Model C: Event Log Plus Natural-Language Memory Stream

Sketch:

```text
EventHistoryStore
  -> actor memory text stream
  -> retrieval / reflection
  -> agent context
```

Makes easy:

- LLM-agent compatibility
- autobiographical summaries
- flexible story explanations
- rich natural-language recall

Makes hard:

- deterministic inspection
- contradiction and revision
- action unlocks from knowledge
- non-LLM NPCs
- localization and tooling

Likely failure:

Black-box memory that sounds plausible but cannot reliably drive rules.

### Model D: Typed Epistemic Records With Projections

Sketch:

```text
EpistemicRecord
  holder: Actor | Faction | Culture | Place | Group
  content: EventRef | EntityRef | Proposition | SocialClaimRef |
           ProcedureRef | LocationRef | TextFragment
  mode: observed | remembered | believed | known | rumored | inferred |
        secret | taught | reflected
  provenance
  confidence
  salience
  freshness
  access / disclosure
  contradiction / supersession links
```

Views:

```text
WorkingSetView
EpisodeMemoryView
BeliefView
KnowledgeView
RumorView
SecretView
ProcedureView
AgentTurnInputView
```

Makes easy:

- shared provenance
- actor-relative divergence from hard truth
- one content item appearing as memory, rumor, belief, or secret
- AI reflection under explicit provenance
- social propagation and disclosure history
- compact agent input

Makes hard:

- schema design
- avoiding too much generality
- deciding which modes are real stored state and which are derived views
- performance if every actor has too many records

Likely failure:

A too-generic abstraction if it loses typed content schemas.

Current research pressure:

Model D is the strongest direction, but only if the content remains typed and
the system does not become a universal untyped fact graph.

## Representation Pressures

### Holder

Information can belong to more than individual actors:

```text
Actor
Faction
Culture
Profession
Place
Institution
Party
Narrative agent / storyteller
```

Examples:

- a village remembers an old betrayal
- a guild knows a recipe
- a culture treats a tomb as taboo
- a place has local rumor ecology
- a party shares a map location

### Content

The content of an epistemic record should not always be prose.

Useful content forms:

```text
EventRef
EntityRef
RelationRef
SocialClaimRef
LocationRef
ProcedureRef
Proposition
GeneratedTextRef
EvidenceBundleRef
```

Natural-language text is useful for presentation and AI context, but it should
usually point back to typed content where gameplay depends on it.

### Mode

Mode answers what kind of epistemic relationship the holder has to the content:

```text
observed
remembered
believed
known
rumored
inferred
taught
secret
reflected
forgotten / inaccessible
```

Open question:

Some modes may be stored; others may be derived from evidence, confidence,
source, and access policy.

### Provenance

Minimum useful provenance:

```text
source_event?
source_actor?
source_faction?
source_item?
source_text?
source_process?
channel: sight | sound | speech | book | ritual | inference | AI | worldgen
created_at
last_confirmed_at?
source_chain?
```

This is required for:

- rumor chains
- lies and misinformation
- debugging NPC behavior
- AI-agent grounding
- belief revision
- social blame and trust updates

### Confidence And Truth Relation

Do not collapse these:

```text
confidence:
  how strongly the holder treats the content as usable.

truth_relation:
  how the content relates to hard truth, if the engine knows or chooses to
  expose that relation to debug tools.
```

An actor can confidently believe a false rumor. A player can have weak evidence
for a true secret.

### Salience And Accessibility

Every record should not be equally retrievable.

Useful factors:

- recency
- emotional salience
- repetition / rehearsal
- relation to current goal
- relation to current actor / place / object
- source trust
- identity relevance
- danger or reward relevance

This is where Generative Agents-style retrieval, Dwarf Fortress-style memory
promotion, and psychological forgetting can meet.

### Disclosure And Secrecy

`Secret` should probably be a wrapper or mode over content, not a completely
separate world object.

A useful distinction:

```text
Secret:
  information whose restricted access creates value or risk.

KnownSecret:
  a holder knows or believes the secret.

Leverage / Hook:
  a holder can use the known secret to force, threaten, trade, accuse, or
  negotiate.
```

The social force of a secret depends on audience and norms.

### Behavioral Hooks

Epistemic state should matter through explicit surfaces:

- action repertoire: knows recipe, passphrase, ritual, route, legal form
- perceived affordance: recognizes seal, weakness, trap sign, social opening
- expected consequence: believes theft will be punished
- pressure: remembers betrayal, fears monster, wants revenge
- intent generation: seek witness, bury mentor, avoid town, spread rumor
- dialogue: ask, accuse, lie, confess, blackmail, teach
- social update: trust, resentment, debt, shame, gratitude, obligation

It should not mutate hard world state directly.

## AI-Agent Memory Boundary

AI agents create a special risk:

```text
external agent memory can diverge from engine state
```

Possible boundary:

- The engine owns gameplay-relevant epistemic records.
- The agent may keep private notes for style and continuity.
- Private notes do not unlock actions, establish facts, or justify social
  consequences unless imported through a validated epistemic write.
- The agent can propose:
  - reflection summaries
  - belief updates
  - rumor wording
  - interpretation hypotheses
  - retrieval queries
  - self-model notes
- The causal/semantic runtime records accepted proposals with provenance.

Useful AI roles:

- summarize many episodes into a memory or identity reflection
- generate actor-specific interpretation text
- propose why an actor is angry, afraid, ashamed, or grateful
- turn structured records into compact `AgentTurnInput`
- produce natural-language rumor variants from typed content
- reconcile contradictory accounts for presentation

Risks:

- AI hallucinated memory becomes gameplay state
- AI compresses away crucial source/provenance
- agent-private notes make debugging impossible
- nondeterministic reflection changes replay behavior

Mitigation:

- separate proposal from acceptance
- store prompt/context/model/provenance where AI affects state
- allow soft truth and actor belief to be false
- keep hard truth mutation outside AI memory

## Test Scenarios

### Witnessed Mentor Death

Initial state:

- bandit attacks mentor
- player sees the killing clearly
- villagers hear only shouting

Expected epistemic result:

```text
player:
  EpisodeMemory(WitnessedDeath, victim=mentor, cause_actor=bandit,
                source_event=ActorDied, confidence=high)
  Belief(bandit killed mentor, confidence=high)

villager:
  EpisodeMemory(HeardViolence, place=road, confidence=medium)
  Belief(someone was attacked, confidence=low)
```

Semantic result:

- player grief/revenge pressure
- villager fear/suspicion
- no omniscient village-wide belief that the bandit did it

Reveals:

Observed events and beliefs are not the same record.

### False Rumor Becomes Social Pressure

Initial state:

- priest died from poison
- merchant is innocent
- rival tells guard the merchant bought rare poison

Expected epistemic result:

```text
guard:
  Rumor(merchant bought poison, source_actor=rival, confidence=medium)
  Belief(merchant may be involved, confidence=medium)
  source_chain: rival -> guard
```

Semantic result:

- suspicion pressure toward merchant
- question / follow / search / arrest intents become more likely
- later evidence can contradict the belief without deleting the rumor record

Reveals:

Rumor, belief, and hard truth must diverge cleanly.

### Secret Becomes Leverage

Initial state:

- noble has illegal cult membership
- spy discovers evidence
- local law makes this scandalous

Expected epistemic result:

```text
Secret(content=cult_membership, subject=noble, norm_context=local_law)
KnownSecret(holder=spy, source=evidence_bundle)
Leverage(holder=spy, target=noble, basis=KnownSecret)
```

Reveals:

Secret is not just information. It is information plus access, audience,
norms, and possible social action.

### Stale Guard Location

Initial state:

- actor saw guard at door ten turns ago
- guard has moved

Expected epistemic result:

```text
EpisodeMemory(guard_at_door, observed_at=t-10, freshness=stale)
Belief(guard may still be near door, confidence=low or decayed)
```

Reveals:

Remembered location should not be treated as current truth.

### Recurring Rival

Initial state:

- enemy survived a duel with the player
- enemy was burned by the player's fire spell

Expected epistemic result:

```text
rival:
  EpisodeMemory(defeated_by_player_fire, salience=high)
  Belief(player uses fire, confidence=high)
  ProcedureBias(avoid_fire_range or acquire_fire_resistance)
```

Player-facing result:

- rival references the duel
- changes equipment or tactics
- social reputation around the duel spreads locally

Reveals:

Memory is strongest when it changes future behavior and presentation.

### Contradictory Village History

Initial state:

- two villages descend from opposite sides of an old battle
- generated history has one hard event but two inherited accounts

Expected epistemic result:

```text
village_a_culture:
  Belief(we were betrayed at the river)

village_b_culture:
  Belief(we were defending the shrine)

EventHistoryStore:
  HistoricalEvent(battle_at_river)
```

Reveals:

History can produce multiple actor/culture-relative accounts without changing
the hard event.

## Failure Modes

- Omniscient actors because memory reads hard truth directly.
- Infinite event logs where every minor observation influences behavior forever.
- Natural-language memory that sounds rich but cannot unlock actions or support
  debug queries.
- Generic fact graph where every predicate becomes stringly and untyped.
- Separate feature stores that duplicate source/confidence/age/disclosure.
- Rumor overwrites truth instead of existing as sourced actor belief.
- Secrets modeled as journal text rather than social leverage.
- AI reflection accepted as fact without provenance.
- Agent-private memory changes behavior in ways the engine cannot explain.
- Full cognitive realism consumes complexity without improving gameplay.
- One trust/reputation score hides the difference between competence,
  benevolence, integrity, fear, debt, and affection.
- Belief revision silently deletes older beliefs, losing story and debug value.

## Open Questions

- Should the central type be called `EpistemicRecord`, `InformationRecord`,
  `ActorInformation`, or something else?
- Which modes are stored state and which are derived views?
- What content forms are required first: `EventRef`, `Proposition`,
  `SocialClaimRef`, `ProcedureRef`, `LocationRef`, `TextFragment`?
- How much natural language belongs inside records versus generated views?
- How much actor-private AI memory should be allowed outside engine-owned
  epistemic state?
- Can AI-authored reflections affect NPC behavior immediately, or must they be
  accepted through a semantic rule gate?
- How should memory promotion/forgetting work outside the active local
  simulation radius?
- How should faction, culture, place, and institution memory differ from actor
  memory?
- What should make knowledge action-enabling instead of merely descriptive?
- How should contradictory beliefs be exposed to debug tools and players?
- How small should `AgentTurnInput` be, and who chooses the working set?

## Takeaways For `world`

Keep:

- actor-relative information
- provenance on every gameplay-relevant information record
- clean separation between hard truth, social claims, and actor belief
- source chains for rumor and testimony
- secrets as access-controlled, socially valuable information
- compact working sets for agents
- memory/thought/pressure/intent as a visible chain

Adapt:

- Generative Agents-style retrieval: recency, relevance, importance
- MemGPT/Letta-style tiering: core visible context plus archival query
- Dwarf Fortress-style memory promotion and long-term effects
- Talk of the Town-style mental models and belief propagation
- Caves of Qud-style knowledge as currency and world-history vector
- Reflexion-style failure lessons, but with engine-grounded provenance
- CoALA vocabulary: working, episodic, semantic, procedural
- provenance/named-context ideas from W3C PROV and RDF datasets

Reject:

- treating `Memory`, `Belief`, `Knowledge`, `Rumor`, `Secret`,
  `SocialClaim`, and `Reservation` as peer stores
- using a universal untyped fact graph as the hard-truth core
- treating every lore line as simulation state
- letting AI mutate hard truth through memory
- a single global trust score
- full logical belief revision for every actor

Defer:

- exact schema
- exact confidence math
- exact forgetting/promoting thresholds
- exact AI coauthority policy
- exact player-facing UI language
- exact storage/index strategy

## Source References

- Generative Agents: <https://arxiv.org/abs/2304.03442>
- MemGPT: <https://arxiv.org/abs/2310.08560>
- Letta memory docs: <https://docs.letta.com/guides/agents/memory>
- Reflexion: <https://arxiv.org/abs/2303.11366>
- Voyager: <https://arxiv.org/abs/2305.16291>
- CoALA: <https://arxiv.org/abs/2309.02427>
- MemoryBank: <https://arxiv.org/abs/2305.10250>
- Memory for Autonomous LLM Agents survey: <https://arxiv.org/abs/2603.07670>
- Memory in the Age of AI Agents survey: <https://arxiv.org/abs/2512.13564>
- W3C PROV-DM: <https://www.w3.org/TR/prov-dm/>
- RDF 1.1 Concepts and Abstract Syntax: <https://www.w3.org/TR/rdf11-concepts/>
- Doyle, A truth maintenance system:
  <https://www.sciencedirect.com/science/article/pii/0004370279900080>
- Logic of Belief Revision:
  <https://plato.stanford.edu/entries/logic-belief-revision/>
- Schacter, The seven sins of memory:
  <https://pubmed.ncbi.nlm.nih.gov/10199218/>
- Tulving episodic/semantic memory reference:
  <https://cir.nii.ac.jp/crid/1574231874408386176?lang=en>
- Squire and Zola, memory systems:
  <https://pmc.ncbi.nlm.nih.gov/articles/PMC33639/>
- Cowan, working-memory capacity:
  <https://pubmed.ncbi.nlm.nih.gov/11515286/>
- Johnson, Hashtroudi, and Lindsay, source monitoring:
  <https://pubmed.ncbi.nlm.nih.gov/8346328/>
- Lazarus, emotion and adaptation:
  <https://pubmed.ncbi.nlm.nih.gov/1928936/>
- Mayer, Davis, and Schoorman, trust:
  <https://journals.aom.org/doi/abs/10.5465/amr.1995.9508080335>
- Kunda, motivated reasoning:
  <https://pubmed.ncbi.nlm.nih.gov/2270237/>
- Tversky and Kahneman, judgment under uncertainty:
  <https://pubmed.ncbi.nlm.nih.gov/17835457/>
- Dwarf Fortress memory: <https://dwarffortresswiki.org/index.php/Memory_%28thought%29>
- Bay 12 Dwarf Fortress dev log:
  <https://www.bay12games.com/dwarves/dev_2018.html>
- RimWorld thoughts: <https://rimworldwiki.com/wiki/Thoughts>
- Caves of Qud secrets: <https://wiki.cavesofqud.com/wiki/Secret>
- Caves of Qud water ritual: <https://wiki.cavesofqud.com/wiki/Water_ritual>
- Caves of Qud mythic biographies:
  <https://www.freeholdgames.com/papers/Generation_of_Mythic_Biographies_in_CavesofQud.pdf>
- Shadow of Mordor Nemesis patent:
  <https://patents.justia.com/patent/10926179>
- Disco Elysium Thought Cabinet:
  <https://discoelysium.com/devblog/2019/09/30/introducing-the-thought-cabinet>
- Crusader Kings III schemes, secrets, and hooks:
  <https://forum.paradoxplaza.com/forum/developer-diary/ck3-dev-diary-5-schemes-secrets-and-hooks.1289167/>
- Prom Week:
  <https://www.fdg2013.org/program/papers/paper13_mccoy_etal.pdf>
- Talk of the Town character knowledge:
  <https://www.gameaipro.com/GameAIPro3/GameAIPro3_Chapter37_Simulating_Character_Knowledge_Phenomena_in_Talk_of_the_Town.pdf>
- Neighborly: <https://www.kmjn.org/publications/Neighborly_CoG22-abstract.html>
- Versu:
  <https://colab.ws/articles/10.1109%2Ftciaig.2013.2287297>
- Schelling dynamic models of segregation:
  <https://www.tandfonline.com/doi/abs/10.1080/0022250X.1971.9989794>
- Axelrod cultural dissemination:
  <https://web.mit.edu/curhan/www/docs/Articles/15341_Readings/Culture_and_Identity/Axelrod-1997.pdf>
- Sugarscape / Growing Artificial Societies:
  <https://www.brookings.edu/books/growing-artificial-societies/>
