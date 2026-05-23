# Caves of Qud

## Why It Matters

Caves of Qud is a strong reference for `world` because its depth comes from
simulation grammar rather than from visual fidelity. Its public materials
describe a hybrid handcrafted and procedurally generated world, deep physical
simulation, fully simulated creatures, faction systems, body-changing character
builds, secrets, histories, and moddable object definitions.

For `world`, the useful question is not how to reproduce Qud. The useful
question is what kinds of state must exist before physical, social, historical,
and actor-specific systems can compose.

## Distinctive Systems

### Hybrid Authored And Generated World

Qud mixes fixed story locations and authored worldbuilding with generated
regions, villages, historic sites, sultans, sultan events, relics, faction
relationships, and discoverable lore.

The important pattern is that generated history is not just prose. It can leave
behind discoverable sites, named regions, artifacts, faction preferences, and
secrets. A generated past becomes queryable present-day state.

### Body As Action-Space

Qud treats body structure as gameplay machinery. Characters and creatures have
limbs, body-part slots, equipment constraints, mutations, cybernetics, wounds,
and sometimes altered anatomy. Mutations can add senses, movement modes,
natural weapons, body slots, social reputation effects, wall interaction,
environmental effects, or mental powers.

This matters because "character build" is not only stat allocation. It changes
what an actor can perceive, equip, traverse, attack, survive, and socially
represent.

### Fully Simulated Creatures

The player, NPCs, and monsters share more simulation machinery than in many
RPGs. Public descriptions emphasize that creatures have levels, skills,
equipment, faction allegiances, and body parts. Mind-control and body-changing
effects are only convincing because an NPC body is already a real actor body,
not a thin combat token.

### Physical Matter And Affordances

Qud makes matter tactically meaningful. Walls can be dug, corroded, or melted.
Objects have physical properties through parts such as `Physics`, including
weight, conductivity, flame temperature, vapor temperature, freeze temperature,
solidity, ownership, and takeability.

The key lesson is that terrain and objects should not be mere rendering tiles.
They should carry affordances that rules can query.

### Secrets As Knowledge Currency

Qud's secrets are tracked in the journal and can be exchanged through social
systems such as the water ritual. Secrets include locations, sultan histories,
village histories, gossip, lore, recipes, chronology, and notes.

The important design point is that information can behave like inventory
without becoming a physical item. It can be discovered, remembered, categorized,
traded, and exhausted in specific social contexts.

### Factions, Reputation, And Ritualized Exchange

Qud has persistent factions as well as procedurally generated factions, with
player reputation influencing hostility and social options. The water ritual
connects reputation, secrets, gossip, recruitment, and faction-specific
interests.

The notable property is that social state is not isolated dialogue flavor. It
feeds creature behavior, access to knowledge, alliance, and consequences.

### XML Blueprints And Parts

The modding documentation describes an entity-component-style architecture.
Objects are defined through XML blueprints, and "parts" attach data and behavior
to objects. Data mods can define objects, bodies, conversations, factions,
genotypes, locations, and quests, while unique behavior often requires C#.

The useful pattern is not "use XML". The useful pattern is the separation
between content declarations, reusable behavior parts, and code needed for new
rule families.

## Signature Design Ideas

- A world can feel deep when history, matter, society, body, and knowledge all
  leave explicit state behind.
- Character identity is a bundle of capabilities, constraints, senses, social
  relations, and embodied affordances.
- Generated lore becomes stronger when it points to places, relics, factions,
  secrets, and future actions.
- Social systems become gameplay when they control hostility, exchange,
  recruitment, and knowledge flow.
- Content systems need an extensible grammar, but that grammar still needs
  strong source-of-truth boundaries.

## Foundational World Model

Qud's world model appears to center on game objects in zones, with creatures,
items, terrain features, bodies, liquids, gases, factions, histories, and
knowledge systems layered on top. The public modding documents show that many
world things are object blueprints composed from parts.

For `world`, this suggests that the foundational ontology should avoid a hard
split between "creature", "item", and "terrain decoration" too early. Those
categories should be views over entities/components/rules where possible.

The caveat is that Qud has a long-lived codebase with many historical layers.
`world` should extract the ontology, not the exact implementation texture.

## Simulation Truth and State Ownership

Qud is not publicly documented as an event-sourced simulation. The available
evidence points instead to authoritative game state stored across objects,
parts, zones, factions, bodies, and journal/knowledge systems.

Useful implications for `world`:

- Authoritative truth should live in the simulation, not the renderer.
- Actor-facing views should be derived from authoritative state.
- Knowledge state should not be treated as UI-only text.
- Debug views may inspect truth, but ordinary actors should receive filtered
  observations and known facts.
- Save/load, replay, and versioning concerns need to be designed explicitly,
  not inferred from a component system alone.

## Action, Event, and Consequence Model

Qud exposes many actions and consequences: movement, attacks, digging,
conversation, water ritual choices, mutation powers, cybernetic installation,
body-part loss, item use, cooking, reading, learning secrets, and environmental
changes.

The public documentation does not show a clean action/event model suitable for
direct adoption. For `world`, the lesson is therefore contrastive:

- Keep Qud's richness of consequences.
- Do not assume direct mutation plus UI messages is enough.
- Represent attempted actions separately from resolved consequences.
- Emit structured events before deriving logs, memories, AI context, and UI
  text.
- Treat failed, partial, blocked, delayed, and socially consequential actions
  as first-class outcomes.

## Perception, Memory, Knowledge, and Belief

Qud strongly supports knowledge as gameplay state. Secrets are recorded,
classified, and exchangeable. Sultan histories are discovered from shrines,
engraved or painted objects, water ritual trades, skills, cooking effects,
infections, books, and other sources.

The perception side is also gameplay-relevant. Mutations such as night vision,
heightened hearing, clairvoyance, sense psychic, and psychometry show that
senses and recognition abilities can be part of actor capability.

For `world`, the extracted model should separate:

- `Perception`: what the actor currently senses.
- `Memory`: what the actor previously observed.
- `Knowledge`: stable facts, recipes, maps, histories, names, and secrets.
- `Belief`: actor-specific conclusions that may be wrong.
- `Disclosure`: what an actor is willing or able to share.

Qud is especially useful for knowledge and disclosure. It is less useful as a
complete model of false belief, because public sources emphasize player
journaled knowledge more than autonomous NPC belief state.

## Entity, Body, Item, and Terrain Grammar

Qud's body model is one of its most relevant systems. Public modding docs
describe bodies as data with body-part types, variants, and anatomies.
Equipment slots map to body parts, and body loss can remove slots, drop
equipped items, and block related weapon slots.

Mutations and cybernetics both bind capabilities to body and identity:

- Mutations can add or alter anatomy, senses, natural weapons, reputation, and
  active abilities.
- Cybernetics are found in the world, installed into body slots, constrained by
  license points, and can interact with dismemberment.
- Some creature innate abilities are represented through the same mutation
  system used for player customization.

Terrain and items also participate in simulation through material and physics
properties. The useful direction for `world` is a shared grammar:

- bodies define slots and senses
- items define affordances and physical properties
- terrain defines material interaction and occupancy
- abilities query body, item, knowledge, position, and social state
- damage can affect actor capability, not only hit points

## Content and Rule Composition

Qud's modding model is the best public window into its content composition.
Object definitions live in XML blueprints. Blueprints can merge with existing
definitions. Parts can be added or removed. Parts attach data and behavior to
objects, and some parts respond to events or active conditions.

This suggests a useful three-layer model for `world`:

- `Content`: definitions of entities, bodies, factions, items, terrain,
  conversations, locations, and histories.
- `Parts/Components`: reusable capabilities and state-bearing attachments.
- `Rules/Systems`: code that interprets components and resolves actions.

The risk is that an open-ended parts system can accumulate opaque special cases.
For `world`, every component should answer:

- What state does it own?
- Which actions does it enable?
- Which events can it emit or consume?
- Which observations does it affect?
- Is it content data, simulation state, or derived presentation?

## Social, Faction, Law, and Reputation Systems

Qud models factions as more than enemy categories. The wiki describes persistent
factions, procedurally generated factions, starting reputation, hostility
thresholds, faction-specific interests, secrets, gossip, recruitment through
water ritual, and reputation consequences for oathbreaking.

The transfer point is that social state should be represented as simulation
state, not only as dialogue flags.

For `world`, useful social primitives include:

- faction membership
- faction reputation
- personal relationship
- local law or taboo
- witnessed action
- promise, oath, debt, and betrayal
- known secret and share eligibility
- faction interest in a class of knowledge

Qud does not by itself answer whether social truth, rumor, and belief should be
actor-specific. `world` should make that boundary explicit.

## Agent and Actor Interface

Qud is not an AI-agent benchmark and does not expose an external agent contract
as part of its public design. Its value for `world` is indirect: it shows what
kind of semantic observation an agent would need in a rich simulation.

An agent-facing interface inspired by Qud would need to expose:

- current body and impaired body parts
- senses and current perception
- known secrets, maps, histories, recipes, and faction knowledge
- equipment slots and available item affordances
- active abilities from mutations, cybernetics, skills, tools, and conditions
- faction reputation and local social risk
- environmental affordances such as diggable, meltable, flammable, wet,
  conductive, locked, owned, or forbidden objects

The important boundary is that an agent should not receive the whole truth just
because the simulation can represent it. Qud's richness makes partial
observation more necessary, not less.

## Determinism, Replay, and Debuggability

Qud has clear deterministic-adjacent features, including Daily mode with a fixed
character and world seed, and mutation-buy choices that are deterministic under
normal save reloads. Public sources do not show a replayable event log suitable
for `world`'s goals.

For `world`, the conclusion is:

- Keep seed-driven generation as a hard requirement.
- Include content version in reproducibility.
- Centralize random choice inside the simulation.
- Record ordered action requests and structured events.
- Do not rely on save/load determinism as a substitute for replayability.

## Representation Pressure

Qud's depth pressures the model in several directions:

- Walls need material properties, not just passability.
- Bodies need slots, loss, mutation, cybernetic attachment, and equipment
  consequences.
- Creatures need the same simulation grammar as the player for domination,
  followers, faction response, and body-dependent action.
- Knowledge needs identity and exchange history, not only journal text.
- Generated history needs persistent references to regions, sites, factions,
  relics, and secrets.
- Social state needs thresholds, rituals, interests, and consequences.

The largest warning is that richness grows from years of system accretion. If
`world` adopts a parts/component model, it also needs stricter event,
observation, and state-ownership contracts than Qud exposes publicly.

## Transferable Principles

- Treat generated history as state that can create places, artifacts, factions,
  and secrets.
- Treat body structure as a source of action affordances.
- Make knowledge a gameplay resource with provenance and sharing rules.
- Let creatures and players share the same actor grammar.
- Model physical properties that rules can query.
- Separate content definitions from reusable behavior components.
- Let social systems affect combat, dialogue, trade, recruitment, and
  knowledge.

## Rejected or Risky Patterns

- Do not import Qud's whole scope as a requirement.
- Do not let flavorful lore replace explicit state.
- Do not rely on UI messages as the primary record of world change.
- Do not allow parts/components to become an unbounded bag of special cases.
- Do not make player knowledge and actor knowledge indistinguishable.
- Do not assume faction reputation alone is enough for social simulation.

## Open Questions for `world`

- Should `world` use an entity/component model, or a narrower typed state model
  with component-like extensions?
- What is the minimal body ontology that can support senses, equipment,
  injury, natural weapons, cybernetic-like implants, and mutations?
- Should knowledge be stored per actor, per faction, globally, or across all
  three?
- Should generated history emit the same event type as live simulation, or use a
  separate historical event model?
- How should physical properties such as heat, conductivity, ownership,
  solidity, and contamination be represented?
- What social facts are truth, and what social facts are actor belief?

## Extracted Design Notes

### Keep

- Character build should reshape action space, not only stats.
- World history should leave inspectable, discoverable, and tradable state.
- Actor bodies should be real simulation structures with slots, senses, wounds,
  and capability consequences.
- Secrets, names, maps, recipes, histories, and faction knowledge should be
  gameplay state.
- Objects and terrain should carry material affordances that rules can query.

### Adapt

- Adapt Qud's XML blueprint and parts idea into a stricter content/component/rule
  contract with explicit state ownership.
- Adapt water ritual style exchanges into a general social transaction model:
  actor, relation, knowledge offered, knowledge requested, cost, consequence.
- Adapt sultan histories into a historical event system that can seed places,
  factions, relics, rumors, and contradictory accounts.
- Adapt mutations and cybernetics into a broader capability system tied to
  body, item, knowledge, social identity, and condition.

### Avoid

- Avoid copying Qud's setting, faction list, mutation list, or prose style.
- Avoid treating Qud's accumulated special cases as the target architecture.
- Avoid building content breadth before the source-of-truth model is clear.
- Avoid hidden omniscience in agent or NPC decision-making.

### Open Questions for `world`

- What event granularity is required for memory, replay, debug inspection, and
  agent context?
- What is the boundary between `Knowledge`, `Memory`, `Secret`, `Rumor`, and
  `Belief`?
- Can body changes, equipment, faction identity, and knowledge all contribute to
  one available-action derivation path?
- Should historical generation and live simulation share the same ontology?

## Sources

- Official site: https://www.cavesofqud.com/
- Steam page: https://store.steampowered.com/app/333640/Caves_of_Qud/
- Official wiki, World generation:
  https://wiki.cavesofqud.com/wiki/World_generation
- Official wiki, Sultan histories:
  https://wiki.cavesofqud.com/wiki/Sultan_histories
- Official wiki, Secret: https://wiki.cavesofqud.com/wiki/Secret
- Official wiki, Player Character:
  https://wiki.cavesofqud.com/wiki/Player_Character
- Official wiki, Equipment: https://wiki.cavesofqud.com/wiki/Equipment
- Official wiki, Mutations: https://wiki.cavesofqud.com/wiki/Mutations
- Official wiki, Cybernetics: https://wiki.cavesofqud.com/wiki/Cybernetic
- Official wiki, Factions: https://wiki.cavesofqud.com/wiki/Factions
- Official wiki, Modding: Objects:
  https://wiki.cavesofqud.com/wiki/Modding%3AObjects
- Official wiki, Modding: Parts:
  https://wiki.cavesofqud.com/wiki/Modding%3AParts
- Official wiki, Modding: Bodies:
  https://wiki.cavesofqud.com/wiki/Modding%3ABodies
- Official wiki, Modding: Overview:
  https://wiki.cavesofqud.com/wiki/Modding%3AOverview
