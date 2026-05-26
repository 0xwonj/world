# Reference Research Questions

This document defines how to study reference games, simulations, agent
environments, and content systems for `world`.

The goal is not to copy any single reference. The goal is to extract useful
world-modeling ideas: what the reference treats as state, how actions become
consequences, how actors perceive and remember, how content composes, and which
distinctive systems create depth.

## How To Use This Document

Use the question bank as a guide, not as a required template. Each reference
should keep the structure that best fits the systems being studied.

For each reference:

- Identify the systems that matter to `world`.
- Separate observed mechanics from inferred implementation.
- Capture both common architectural lessons and distinctive ideas.
- Mark which patterns transfer cleanly, which require adaptation, and which
  should be rejected.
- Prefer design principles over surface features.

## Lightweight Example

Reference notes do not need fixed sections. A small note can be as light as:

```md
# Reference Name

## Why It Matters

## Notable Systems

## Design Insights

## Open Questions
```

Longer notes can add sections for world model, action/event boundaries,
knowledge systems, content composition, agent implications, risks, or sources
when those details are useful.

## Core Question Bank

### Foundational World Model

- What does this reference treat as the basic substance of the world?
- Are actors, items, terrain, factions, knowledge, and history part of one
  shared world model, or are they separate systems?
- Which concepts are first-class state, and which are only presentation,
  narrative, or player-facing convenience?
- Does the world exist independently of the current screen, player location, or
  active quest?
- Does the same world model support multiple modes of play or observation?

### Simulation Truth and State Ownership

- Where does authoritative truth live?
- What owns map state, entity state, inventory state, faction state, knowledge
  state, scheduled effects, and random state?
- Do clients or UI layers ever own gameplay truth?
- Are derived views clearly separated from authoritative state?
- Can debug tools inspect privileged truth without changing the model that
  ordinary actors use?
- Is there a clean boundary between state storage, rule resolution, and
  presentation?

### Action, Event, and Consequence

- Is there a clear distinction between an attempted action and what actually
  happened?
- Are failed, blocked, interrupted, or partially successful actions represented?
- Are consequences modeled as structured events, direct state mutation, or UI
  messages?
- Can events drive replay, debugging, memory updates, observations, and logs?
- How are turn order, simultaneous action, interruption, delay, and scheduled
  effects handled?
- How granular are events, and what does that granularity make possible or
  impossible?
- Are rules allowed to produce presentation text directly, or do they produce
  structured consequences first?

### Perception, Memory, Knowledge, and Belief

- Does each actor receive actor-specific observation, or does gameplay assume
  omniscient state?
- Are sight, sound, smell, touch, social recognition, and other senses modeled
  as distinct channels?
- Is remembered map knowledge separate from current perception?
- Can memory become stale?
- Can actors hold false beliefs?
- Are rumors, laws, recipes, secrets, names, maps, histories, and faction
  symbols gameplay state?
- Does knowledge change action repertoire, perceived affordances, dialogue,
  trade, travel, or social outcomes?
- Is player knowledge separated from character knowledge?

### Entity, Body, Item, and Terrain Grammar

- Do players, NPCs, monsters, corpses, items, terrain, and environmental
  features share a common simulation grammar?
- Are body parts, wounds, senses, equipment slots, needs, and conditions modeled
  as data?
- Are items passive inventory entries or active simulation participants?
- Do materials, tags, components, ownership, temperature, durability, wetness,
  contamination, or charge affect behavior?
- Can terrain be transformed, damaged, occupied, consumed, opened, burned,
  flooded, frozen, or otherwise changed by rules?
- Do abilities, mutations, tools, bodies, and knowledge reshape the action
  space rather than only modifying stats?

### Content and Rule Composition

- Is content authored mostly in code, data, scripts, rules, tags, components, or
  a mixture?
- How do content definitions connect to rules?
- Are special cases isolated, or do they accumulate into hard-to-predict rule
  tangles?
- Can new content create new interactions by composing with existing rules?
- Does procedural generation create only layout, or also history, social
  context, secrets, artifacts, and knowledge?
- How does content versioning affect reproducibility, saved games, replay, and
  debugging?
- What does the reference make easy to author, and what remains expensive?

### Social, Faction, Law, and Reputation Systems

- Are factions simple hostility flags, relationship graphs, social identities,
  or institutional actors?
- Are local law, taboo, crime, witness, reputation, debt, promise, rumor, and
  social role represented as state?
- Do social systems affect combat, trade, dialogue, perception, access,
  alliance, punishment, and movement?
- Can NPCs act differently based on personal memory, faction knowledge, role, or
  social context?
- Do player actions leave social consequences in the world?
- Does social knowledge propagate through observation, rumor, witness reports,
  or explicit communication?

### Agent and Actor Interface

- What is an actor in this reference: a command source, a body in the world, a
  decision process, or all of these?
- Do human players, NPCs, scripted agents, and AI agents use the same action
  path?
- What observation would an external AI agent need to act without receiving
  omniscient state?
- Is the action interface fully enumerated, represented as actor-owned schemas
  plus perceived affordances, or free-form?
- How are invalid actions handled?
- Does the engine own agent memory, or can agents keep private external notes?
- How much context would need to be serialized for a language-model agent?
- Does the actor interface expose semantic state, render-oriented state, or
  both?

### Determinism, Replay, and Debuggability

- Can the same seed, content version, initial scenario, and action log reproduce
  the same outcome?
- Is random choice centralized, or can clients and subsystems introduce hidden
  nondeterminism?
- Are event logs sufficient for replay or only for human-readable history?
- Can a surprising outcome be explained from state, rules, actions, random
  choices, and events?
- Are simulation tests natural to write?
- Are debug inspectors, replay viewers, and automated runs supported by the
  architecture or bolted on afterward?

### Distinctive Systems and Signature Ideas

- What systems make this reference unusually recognizable?
- What state representation does each distinctive system require?
- What action, event, perception, memory, or content model does it assume?
- Which parts look like content but are actually core simulation grammar?
- How does the distinctive system combine with other systems to create
  emergent behavior?
- What does the system make possible for players, NPCs, agents, history, or
  replay?
- What general principle can be extracted without copying the surface mechanic?
- What would be distorted if `world` adopted this system directly?

### Representation Pressure

- What did this reference need to represent explicitly in order to achieve its
  depth?
- What limitations appear because something was not represented as state?
- Which modeling choices opened later design space?
- Which modeling choices closed design space or created historical baggage?
- Where does implementation convenience appear to have shaped the design?
- Which ideas need a strong ontology even before all related systems are active?

### Transfer, Adaptation, and Rejection

- What principle should `world` keep?
- What pattern should be adapted into a different form?
- What should be rejected because it is too genre-specific, historical,
  technical, opaque, or incompatible with the desired model?
- What new question does this reference raise for `world`?
- What source-of-truth boundary does this reference clarify?

## Optional Extraction Prompts

When a reference note needs a concise takeaway, these prompts can help. They
are not mandatory sections.

```md
## Takeaways

### Keep

What principle transfers cleanly?

### Avoid

What should not shape `world` directly?

### Open Questions for `world`

What did this reference make unclear or newly important?
```
