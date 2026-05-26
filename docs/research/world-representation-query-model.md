# World Representation / Query Model

## Status

Draft research

## Axis

[World Representation / Query Model](engine-architecture-entry.md)

## Design Outputs

- [World Model](../design/world-model.md)
- [Social Institutional Model](../design/social-institutional-model.md)
- [Truth, Authority, And Layer Boundaries](../design/truth-authority-and-layer-boundaries.md)

## Core Question

```text
How should world facts be stored, indexed, derived, and queried?
```

## Why This Must Be Researched Together

World representation and query design cannot be separated.

The shape of stored facts determines which queries are natural. The shape of
queries determines which state must be indexed, derived, cached, permissioned,
or made authoritative. If representation is chosen without query pressure, the
engine will grow hidden indexes, ad hoc caches, and direct state shortcuts. If
query design is chosen without representation pressure, the engine will drift
toward an untyped universal graph.

For `world`, this axis must answer one coupled problem:

```text
What is hard truth, how can systems ask questions about it, and how do derived
views stay useful without becoming hidden truth?
```

## Scope

In scope:

- entity identity and persistent identity
- component, relation, fact, and history storage models
- containment, topology, body, inventory, equipment, material, and place facts
- spatial, containment, relation, and event indexes
- derived views and invalidation
- query permission boundaries
- debug/provenance views

Out of scope:

- action/effect/event runtime semantics
- process scheduling
- physical simulation rules themselves
- actor memory and belief internals
- semantic interpretation rules
- final implementation language or database choice

Those topics depend on this axis, but they are not settled here.

## Terminology Alignment

This document is aligned with
[Causal Runtime / Action-Effect-Event](causal-runtime-action-effect-event.md).

Use these names consistently:

- `CausalTransactionGate`: the only hard mutation gate. This is the concrete
  form of the earlier mutation-gate idea.
- `Reservation`: temporary hard runtime state used for process/action
  conflict resolution. It is written through causal transactions.
- `SocialClaim`: social, legal, institutional, or customary assertion such as
  ownership, rank, right, debt, or permission.
- Belief over `SocialClaim`: a holder-relative epistemic record whose content
  is a social claim reference. Do not model this as a separate peer store.
- `EventHistoryStore`: the shared hard event/history family containing
  transaction records, `EventRecord`s, generated historical records, and
  long-term history references.

Do not use plain `Claim` without a qualifier. It is ambiguous between
reservation, social claim, and actor belief.

## Theory Baseline

### ECS And Data-Oriented Storage

ECS gives a useful storage and query vocabulary:

- `Entity`: a runtime identifier for a thing.
- `Component`: typed data associated with an entity.
- `System`: logic that queries components and operates on matching entities.
- `Query`: declared component access pattern.
- `Archetype` / table / chunk: storage grouping based on component sets.

Bevy describes ECS as breaking a program into entities, components, and
systems, and its query API lets systems access entity ids and components
without direct access to the whole world. Bevy also distinguishes component
storage strategies such as table storage for query iteration and sparse-set
storage for frequent insertion/removal.

Flecs adds a useful relation vocabulary. A relationship is encoded as a pair:

```text
(relationship, target)
```

This makes graph-shaped facts queryable inside ECS-like storage: containment,
inventory, equipment, faction membership, parent/child hierarchy, or trade
relations can all be represented as typed edges rather than opaque component
fields.

Transfer:

- ECS is a strong storage substrate for hot local simulation.
- Query access declarations are useful for conflict detection and tooling.
- Relationship/pair models are valuable for containment and inverse queries.

Adapt:

- ECS entity handles should not be the only identity. `world` needs stable ids
  for save, replay, memory, history, and semantic references.
- Components should be treated as storage facts, not the whole ontology.
- Relations should be typed and constrained, not arbitrary pairs everywhere.

Reject:

- Raw ECS as the public game API.
- Arbitrary systems mutating components directly.
- A single parent/child hierarchy as universal containment.
- Marker-tag explosion for every semantic condition.
- Runtime string queries as authoritative gameplay rules.

### Fact, Relation, And Datalog Models

Datalog/fact systems give a useful derivation vocabulary:

- base fact: authoritative input fact
- relation: typed set of tuples
- rule: derived relation from other facts
- recursive query: derived closure, such as reachability or containment ancestry
- provenance: explanation of why a derived tuple exists

Souffle models facts as relation atoms and rules as Horn clauses. Its rule
system supports recursive derivation, type checking, stratified negation
constraints, and provenance explanations as proof trees.

Datomic is also useful as a conceptual reference. Its database is a set of
immutable atomic facts called datoms. Transactions add facts at a point in
time; facts can be queried historically through time-aware views; attributes
have schema, cardinality, and value types; indexes support different access
patterns.

Transfer:

- Use a fact/relational mental model for inspectability.
- Treat derived views as pure, explainable relations over base facts.
- Use provenance as a requirement, not an optional debug feature.
- Use transaction records and `EventRecord` provenance for hard fact changes.

Adapt:

- Datalog-like rules are probably best as a derivation/query layer, not the hot
  mutation path.
- Datomic-like datoms are useful as a history/provenance model, not necessarily
  as literal runtime storage.
- Materialized views should be explicit and dependency-tracked.

Reject:

- A universal untyped `Fact(subject, predicate, object, qualifiers)` as the core
  world model.
- Derived caches that can be mutated directly.
- Actor-facing queries that read hard truth without projection.
- "Schema later" for hard world facts.

## Reference Targets

### CDDA

Observed:

- CDDA uses broad typed JSON object families. Its documentation describes JSON
  objects with a `type` field, unique ids, inheritance via `copy-from`, and many
  content families such as materials, body parts, monster factions, terrain,
  furniture, fields, emissions, scent types, skills, tools, vehicles, and
  activities.
- Items can have material composition and container `pocket_data`.
- Body graphs, body parts, field types, monster factions, terrain transforms,
  and material definitions all appear as explicit content/state categories.

Inference:

CDDA pressures `world` toward explicit typed state families:

- item definitions and item instances
- material composition
- container and pocket capacity
- body parts and body graphs
- fields, emissions, scent, and residue-like phenomena
- terrain/furniture transformation
- faction relation tables

Transfer:

- Material should be real data, not flavor.
- Body structure should matter to capability, equipment, damage, and senses.
- Terrain and furniture should be transformable world state.
- Field/residue/contamination-like phenomena need first-class representation.

Adapt:

- Convert broad JSON families into checked content schemas.
- Treat physical capacity scores as derived views over body/equipment/condition
  facts, not hard truth.
- Keep typed ids and schemas, but avoid stringly dispatch and flag soup.

Reject:

- Behavior hidden behind item-specific code hooks.
- UI/log text as the primary record of consequence.
- A catalog of flags with unclear ownership and semantics.

### Caves Of Qud

Observed:

- Qud exposes object definitions and reusable parts. Parts can be attached to
  object definitions through XML and cover creature, corpse, item, rendering,
  body, faction, conversation, inventory-like, and behavior concerns.
- Qud has explicit zone/world terminology: zones, cells, worlds, parasangs,
  world map, and interior zones.

Inference:

Qud pressures `world` toward:

- shared object grammar across creatures, items, terrain, and furniture
- topology beyond a single grid
- bodies as anatomy/equipment/capability structure
- generated history and secrets as state that points to places, relics,
  factions, and objects

Transfer:

- Let generated history create queryable places, artifacts, factions, and
  knowledge.
- Treat body anatomy as a source of equipment slots, senses, wounds, and action
  capacity.
- Treat zones, interiors, containers, body slots, and abstract places as
  related topology/containment concepts.

Adapt:

- Use part/component composition only with explicit state ownership and query
  contracts.
- Model secrets/knowledge as typed facts with provenance and disclosure rules.

Reject:

- Unbounded part bags with unclear ownership.
- Lore text that is not backed by queryable world facts.
- Player journal as the only knowledge store.

### Dwarf Fortress

Observed:

- Dwarf Fortress is described by Bay 12 as a persistent generated fantasy world
  with civilizations, regions, towns, caves, wildlife, history, historical
  events, artifacts, reputations, rumors, body parts, tissues, and materials.
- Its development notes point toward creation myths and recorded history feeding
  future world generation: maps, sites, entities, historical figures, artifacts,
  myths, magical landforms, deities, laws, and materials.

Inference:

Dwarf Fortress is the strongest pressure for history as structured state rather
than prose.

Useful shape:

```text
HistoricalEvent {
  id
  time
  kind
  participants
  place
  factions
  objects
  causal_links?
}

Site {
  id
  location
  controller
  structures
  history_refs
}

Artifact {
  id
  physical_object
  creator
  owners_or_social_claims
  location_history
}
```

Transfer:

- Persistent world history should be structured and queryable.
- Historical figures, sites, factions, artifacts, and events need stable ids.
- Generated world history should be able to seed present-day objects, social
  claims, ruins, rumors, and conflicts.

Adapt:

- Use generated history, but store it in typed event/fact models from the
  start.
- Maintain summaries and indexes instead of making every local action scan the
  full history.

Reject:

- Offscreen history stored only as prose.
- Huge unindexed historical dumps as the main query surface.
- Global omniscient history exposed directly to actors.

## Observations

### Observation: ECS Is Good Storage, But Not Enough Ontology

Inference:

ECS works well for hot entity/component iteration, especially for physical
simulation and local state. But relation-heavy facts such as containment,
equipment, attachment, membership, history, knowledge, and social claims are
awkward if buried inside arbitrary component fields.

Transfer:

Use ECS-like component stores for local physical state, but pair them with typed
relation/index stores and a controlled mutation gate.

### Observation: Query Shape Follows State Shape

Inference:

If containment is represented only as `Location { parent }`, inverse queries
such as "all items inside this container", "all visible residues on this body",
or "all entities in this abstract place" need separate indexes anyway.

Transfer:

Containment, equipment, attachment, embedded-in, passage, location, membership,
and institutional or social-claim edges should be explicit typed relations with
indexes by subject, relation, and object.

### Observation: Derived Facts Are Central, Not Secondary

Inference:

Reachability, visibility, containment closure, passability, material exposure,
capability inputs, and social relevance are all derived from hard facts.
Treating them as ordinary mutable state will create stale hidden truth.

Transfer:

Derived facts should have definitions, dependencies, invalidation rules, and
provenance. Hot derived views may be cached, but they must be rebuildable.

### Observation: History Must Be Queryable

Inference:

Qud and Dwarf Fortress both show that generated history becomes gameplay-rich
when it points to places, artifacts, factions, secrets, and current social
state. If history is stored as prose, it cannot support perception, rumor,
semantic interpretation, or later reification.

Transfer:

Represent generated and live history as structured event/fact families with
stable ids and query indexes.

## Design Decisions

This axis must eventually settle:

- What is the runtime entity handle?
- What is the persistent world identity?
- Which fact families are stored as components?
- Which fact families are stored as typed relations?
- Which fact families belong in event/history stores?
- Which queries are kernel-only, actor-relative, semantic, or debug-only?
- Which derived views are recomputed, cached, incrementally maintained, or
  persisted?
- Which indexes are mandatory for local simulation?
- Which indexes are optional tooling/debug aids?
- How does every derived fact explain its source?
- How does query access avoid leaking hard truth to actors or agents?

## Candidate Models

### Candidate A: Pure ECS

Sketch:

```text
World = entities + components + systems + queries
```

Makes easy:

- fast local iteration
- component-level storage locality
- simple system/query programming model

Makes hard:

- relation-heavy facts
- inverse containment queries
- history and provenance
- actor-relative query permissions
- semantic and epistemic fact separation

Likely failure modes:

- marker-tag explosion
- components with opaque references
- hidden mutation from arbitrary systems
- no stable identity for memory/history

Assessment:

Use ECS as storage substrate, not as the whole world model.

### Candidate B: Universal Fact Graph

Sketch:

```text
Fact(subject, predicate, object, qualifiers)
```

Makes easy:

- arbitrary new relations
- uniform query shape
- rapid prototyping of semantic facts

Makes hard:

- type safety
- phase separation between truth, belief, semantic meaning, and presentation
- performance predictability
- rule debugging
- authoring discipline

Likely failure modes:

- stringly predicates
- weak ownership boundaries
- rule soup
- omniscient queries

Assessment:

Reject as the core model. Keep a fact/query projection only where typed
relation schemas and permissions are explicit.

### Candidate C: Typed Hybrid World Model

Sketch:

```text
WorldStore
  RuntimeEntity table
  PersistentId map
  typed component stores

RelationStore
  typed edges:
    ContainedIn
    EquippedInSlot
    AttachedTo
    EmbeddedIn
    LocatedIn
    PassageTo
    MemberOf
    SocialClaimOn
  indexes by subject / relation / object

EventHistoryStore
  transaction records
  EventRecord
  generated historical records
  artifact/site/faction references

RuntimeControlStore
  process instances
  reservations
  scheduler state
  RNG state

DerivedViewRegistry
  reachability
  containment closure
  visibility
  passability
  material exposure
  capability inputs
  actor-observed facts

QueryLayer
  kernel queries
  actor-relative queries
  semantic context queries
  debug/provenance queries

CausalTransactionGate
  typed effect programs only
  staged mutation
  atomic commit
  event emission
  derived-view invalidation
```

Makes easy:

- hot local physical simulation
- relation-heavy world facts
- explicit derived views
- query permissions
- stable history and provenance
- actor-relative projections

Makes hard:

- more up-front schema work
- view invalidation discipline
- deciding relation cardinality and ownership
- integrating host storage with PL/query tooling

Likely failure modes:

- too many stores without a single mutation gate
- relations becoming untyped graph soup
- derived views becoming stale truth
- overbuilding before concrete scenarios pressure the model

Assessment:

This is the strongest candidate model for `world`.

## Proposed State Families

Initial candidate families:

```text
Identity:
  RuntimeEntity
  PersistentEntityId
  ActorId
  PlaceId
  FactionId
  EventId
  TransactionId
  ProcessId
  ReservationId
  KnowledgeId
  MaterialId

Physical:
  Entity
  PhysicalForm
  MaterialComposition
  Substance
  Body
  BodyPart
  Condition
  Container
  EquipmentSlot
  TerrainFeature
  Structure

Topology:
  Cell
  Zone
  Region
  AbstractPlace
  ContainmentEdge
  AttachmentEdge
  EquippedEdge
  EmbeddedEdge
  PassageEdge

EventRecord / History:
  TransactionRecord
  EventRecord
  HistoricalEvent
  EventCollection
  HistoricalFigure
  SiteHistory
  ArtifactHistory
  GeneratedChronology

Runtime Control:
  ProcessInstance
  Reservation
  ScheduledWakeup
  RngStream
  RngDraw

Social / Institutional:
  Faction
  Membership
  Rank
  SocialClaim
  Reputation
  NormReference

Epistemic:
  KnownFact
  Secret
  Rumor
  MemoryRef
  Source
  DisclosureState
```

Important distinction:

```text
Physical possession:
  hard containment/equipment fact

Reservation:
  temporary hard runtime fact for process/action conflict resolution

Legal ownership:
  social claim

Actor belief about ownership:
  epistemic fact
```

The storage/query model must keep these separable even when the same UI or
semantic rule wants to view them together.

## Query Taxonomy

```text
Kernel query:
  hard truth used for validation and mutation.

Derived engine query:
  passability, reachability, containment closure, material exposure,
  body capability inputs.

Actor-relative query:
  observed state, remembered facts, known secrets, perceived affordances.

Semantic context query:
  events + norms + relationships + social claims + beliefs -> interpreted
  meaning.

Debug query:
  omniscient state plus provenance and derivation explanations.
```

Rules:

- Query layers must declare whether they read hard truth, projected truth,
  belief, semantic facts, or debug-only facts.
- Actor-facing and AI-agent-facing queries must never read omniscient hard
  truth directly.
- Semantic queries should use typed context views, not arbitrary internal state.
- Dynamic query languages are acceptable for read-only tooling and debugging,
  but not for unchecked authoritative mutation.

## Failure Modes

- Components own behavior but do not declare state ownership.
- Relations are hidden inside arbitrary component fields.
- Generated history is stored as text and cannot be queried.
- Actor knowledge is accidentally global truth.
- Containment splits into unrelated systems: inventory, equipment, body slots,
  interiors, wounds, maps, and abstract places.
- Materials become a bag of vague properties without typed effect semantics.
- Query APIs bypass causal runtime and mutate state.
- Derived views become stale hidden truth.
- Reservations, social claims, physical possession, and actor belief collapse
  into one field.
- Faction/reputation data is global and cannot represent observer-specific
  belief or rumor.
- A PL/query layer is added later and has to reverse-engineer state semantics.

## Test Scenarios

### Container Closure

Initial state:

- a relic is inside a lockbox
- the lockbox is inside a mule cart
- the cart is inside a ruined stable
- the stable is in an abstract distant village

Expected query pressure:

- kernel can find physical containment ancestry
- actor projection only shows observed or remembered portions
- promotion from abstract village to local map preserves stable ids
- debug can explain why the relic is considered in the village

Reveals:

- containment must support closure, abstract places, and stable identity

### Blood Evidence

Initial state:

- an NPC is killed with a blade
- blood residue attaches to the suspect's cloak
- a guard later sees the cloak

Expected query pressure:

- blood on cloak is hard physical state
- "evidence" is semantic interpretation
- guard observation creates actor-relative knowledge
- debug can trace from residue to wound event to suspicion

Reveals:

- physical, semantic, and epistemic facts must remain distinct

### Wounded Hand And Lockpick

Initial state:

- actor has right-hand wound
- actor carries lockpick
- target door appears locked

Expected query pressure:

- body/wound/tool facts are authoritative
- fine manipulation is derived
- `PickLock` action schema comes from actor-owned capability
- target affordance only helps bind the action

Reveals:

- capability derivation should be a derived view, not stored hard truth

### Historical Relic

Initial state:

- generated history says a saint forged a relic in a monastery
- the monastery later burned
- a faction now asserts a social claim over the relic
- the player hears a rumor about it

Expected query pressure:

- history references stable site, actor, object, and faction ids
- rumor is actor-relative knowledge
- faction social claim is social state, not physical possession or
  reservation
- relic's physical location may be unknown or false in actor belief

Reveals:

- generated history must be structured state, not prose

## Open Questions

- Should `WorldStore` be ECS-like internally, or a custom typed store with ECS
  query ideas?
- Which relations deserve hard kernel status first?
- Which social claims belong in typed relation state versus semantic/social
  stores?
- Which reservations should be stored as hard runtime state versus short-lived
  transaction scratch data?
- Is generated history stored in the same event/history family as live events,
  or in a separate store with a shared event schema?
- How much Datalog-like derivation should be runtime, and how much should be
  offline/tooling?
- What is the minimal provenance format for derived views?
- Which derived views must be incremental from day one?
- How do saves store caches: persist, rebuild, or selectively snapshot?

## Takeaways For `world`

Keep:

- hard truth as typed authoritative state
- stable persistent ids separate from runtime handles
- explicit typed relations for containment/topology/equipment/attachment
- query permissions by consumer type
- derived views with provenance
- generated history as structured queryable state

Adapt:

- ECS storage as an internal substrate, not public mutation API
- Flecs-style relationship pairs into typed constrained relations
- Datomic-like immutable fact/history thinking into event/provenance design
- Souffle-like Datalog rules into a checked derivation/query layer
- CDDA/Qud/DF content breadth into typed schemas with ownership boundaries

Reject:

- pure ECS as full ontology
- universal untyped fact graph as core architecture
- raw component mutation by arbitrary systems
- marker/tag soup for semantic conditions
- actor-facing omniscient queries
- lore/history as only prose

Defer:

- exact storage implementation
- exact Datalog/rule engine choice
- final list of relation types
- incremental-view strategy
- save/snapshot cache policy

## Sources

Primary or official sources:

- [Bevy ECS overview](https://bevy.org/learn/quick-start/getting-started/ecs/)
- [Bevy Query docs](https://docs.rs/bevy/latest/bevy/ecs/system/struct.Query.html)
- [Bevy Component docs](https://docs.rs/bevy/latest/bevy/ecs/prelude/trait.Component.html)
- [Bevy Relationship docs](https://docs.rs/bevy/latest/bevy/ecs/relationship/trait.Relationship.html)
- [Flecs relationships](https://www.flecs.dev/flecs/md_docs_2Relationships.html)
- [Souffle facts](https://souffle-lang.github.io/facts)
- [Souffle rules](https://souffle-lang.github.io/rules)
- [Souffle provenance](https://souffle-lang.github.io/provenance)
- [Datomic transaction model](https://docs.datomic.com/transactions/model.html)
- [Datomic schema reference](https://docs.datomic.com/schema/schema-reference.html)
- [Datomic query reference](https://docs.datomic.com/query/query-data-reference.html)
- [Datomic indexes](https://docs.datomic.com/indexes/indexes.html)
- [Datomic history](https://docs.datomic.com/client-tutorial/history.html)
- [CDDA JSON docs](https://docs.cataclysmdda.org/JSON/JSON_INFO.html)
- [CDDA item docs](https://docs.cataclysmdda.org/JSON/ITEM.html)
- [Caves of Qud object modding](https://wiki.cavesofqud.com/wiki/Modding%3AObjects)
- [Caves of Qud zones and worlds](https://wiki.cavesofqud.com/wiki/Modding%3AIntro_-_Zones_and_Worlds)
- [Dwarf Fortress features](https://bay12games.com/dwarves/features.html)
- [Dwarf Fortress development page](https://bay12games.com/dwarves/dev.html)

Local anchors:

- [Kernel Primitives](../ideas/kernel-primitives.md)
- [Semantic Kernel And PL Boundary](../ideas/semantic-kernel-and-pl-boundary.md)
- [Actor-Owned Capability-Derived Actions](../ideas/capability-derived-actions.md)
- [Caves of Qud reference note](../references/caves-of-qud.md)
- [CDDA reference note](../references/cataclysm-dda.md)
