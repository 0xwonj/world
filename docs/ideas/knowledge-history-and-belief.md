# Knowledge, History, And Belief

## Status

Candidate

## Core Idea

Lore, history, memory, secrets, rumors, and beliefs should be considered
possible world state, not just presentation text.

This does not mean every piece of flavor text must become simulation state. It
means information should be allowed to matter mechanically when the world,
actors, agents, factions, or actions need to reason about it.

## Why It Matters

Many RPGs treat lore as text attached to quests, books, locations, or dialogue.
That can create atmosphere, but it usually cannot be queried by rules or acted
on by NPCs and agents.

`world` should leave room for information to become part of the simulation:

- a known password can open a gate
- a rumor can send an actor toward a ruin
- a stale memory can make an actor search the wrong place
- a faction secret can be traded for trust
- a historical event can explain an old feud
- a false belief can cause a bad decision
- a discovered recipe can unlock a craft action
- a law or taboo can make an otherwise valid action socially dangerous

The important point is not to implement all of these immediately. The important
point is that the ontology should not force knowledge into UI-only logs.

## Information Kinds

The system may eventually need distinctions like these:

- `Perception`: what an actor currently senses.
- `Memory`: what an actor previously perceived.
- `Knowledge`: relatively stable information the actor can use.
- `Belief`: what an actor thinks is true, even if it is false.
- `Rumor`: uncertain information with source and reliability.
- `Secret`: restricted information with gameplay or social value.
- `History`: past events that can leave present-world consequences.
- `Law`: social or institutional information that changes consequences.

These should not be treated as final type names yet. They are pressure points:
places where different kinds of information may need different behavior.

## History As Present State

History is most useful when it leaves traces that the current simulation can
query.

Examples:

- a battle creates a ruin, graves, relics, and faction resentment
- a founder story creates a sacred site and a local taboo
- a betrayal creates an oath, a rumor, and a hostile relationship
- an old migration creates language fragments and mixed ancestry
- a lost expedition creates a map clue and conflicting accounts
- a forbidden ritual creates a place, a fear, a law, and a hidden recipe

This suggests that historical generation should not produce only prose. It
should be able to produce locations, artifacts, relationships, social facts,
knowledge records, rumors, and contradictions.

## Actor-Specific Information

Different actors should be able to know, remember, or believe different things.

Useful questions:

- Who knows this?
- How did they learn it?
- Do they trust it?
- Is it true, false, uncertain, or outdated?
- Can they share it?
- Have they already shared it with someone?
- Does a faction, culture, profession, body, skill, or item let them interpret
  it differently?
- Does the information unlock or modify an action?

The same world fact can have several actor-facing forms:

```text
Truth:
  The bronze gate opens with the passphrase "blue rain".

Actor A Knowledge:
  The gate opens with "blue rain".

Actor B Rumor:
  The gate opens with a weather phrase.

Actor C False Belief:
  The gate opens with "red rain".

Actor D Memory:
  A merchant said the old passphrase was "blue rain" twenty turns ago.
```

## Interaction With Actions

Information should be able to affect action space without bypassing the action
model.

Examples:

- knowing a password grants `RecitePassphrase`
- knowing a recipe grants or improves `Craft`
- knowing a law marks `Steal` as socially risky
- knowing a monster weakness modifies `Attack` resolution
- knowing a map location enables `TravelToKnownSite`
- believing a rumor can make `SearchArea` attractive to an agent
- remembering a witness can enable `Accuse`, `Threaten`, or `ReportCrime`

As with other capabilities, information should not mutate the world directly.
It should change what actions are available, how actions are interpreted, or
what consequences actors expect.

## Examples

### Secret Trade

An actor knows the location of a sealed shrine. Another actor belongs to a
faction that values shrine locations. Sharing the secret can produce reputation,
trust, betrayal risk, or new rumors.

The secret is not just a journal entry. It has owner, subject, source, value,
audience, and sharing history.

### Contradictory History

Two villages remember the same ancient battle differently. Both accounts point
to the same ruin, but each produces different social reactions, taboos, and
claims of ownership.

The contradiction is useful because it can drive behavior. The simulation does
not need every actor to agree on a single public lore string.

### Stale Memory

An actor saw a guard near a door ten turns ago. The guard has moved. The actor's
memory is still useful, but it should not be treated as current truth.

This distinction matters for both NPC behavior and AI-agent input.

## Design Risks

- If every line of lore becomes state, authoring becomes heavy and noisy.
- If knowledge has no provenance, rumor and belief collapse into generic facts.
- If all knowledge is global, actors become accidentally omniscient.
- If false belief is too unconstrained, debugging behavior becomes difficult.
- If history generation creates only text, it will not affect simulation.
- If history generation creates too much state, the world becomes hard to
  explain.

## Open Questions

- What is the boundary between `Memory`, `Knowledge`, `Belief`, `Rumor`, and
  `Secret`?
- Should historical events and live simulation events share an event model?
- Can information have confidence, source, age, owner, audience, and secrecy
  without becoming overmodeled?
- Which information should belong to actors, factions, cultures, places, or the
  world itself?
- How should contradictory accounts be represented?
- How should an AI agent receive knowledge without receiving omniscient truth?
- What information should change available actions versus only changing action
  ranking or expected consequences?

## Related References

- [Caves of Qud](../references/caves-of-qud.md)
- [Perception and Knowledge](../design/perception-and-knowledge.md)
- [Capability-Derived Actions](capability-derived-actions.md)

