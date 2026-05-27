# World Model

## Status

Current design draft

## Source Research

- [World Representation / Query Model](../research/world-representation-query-model.md)
- [Causal Runtime / Action-Effect-Event](../research/causal-runtime-action-effect-event.md)
- [Epistemic State](epistemic-state.md)

## Related Design Owners

- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
- [Physical Simulation Grammar](physical-simulation-grammar.md)
- [Typed Effect Primitives](typed-effect-primitives.md)
- [Causal Runtime](causal-runtime.md)
- [Social Institutional Model](social-institutional-model.md)
- [Multi-Resolution Simulation](multi-resolution-simulation.md)

## Purpose

The world model defines what counts as authoritative gameplay state, how that
state is organized, and which read surfaces can expose it.

It does not define the meaning of every record family, and it does not grant
mutation authority by itself. The world model supplies store families, query
contracts, indexes, and provenance surfaces. Each truth layer still mutates
only through its own commit surface:

- hard truth through [Causal Runtime](causal-runtime.md)
- social and institutional soft truth through
  [Social Institutional Model](social-institutional-model.md)
- holder-relative actor truth through [Epistemic State](epistemic-state.md)
- appraisal and motivation records through
  [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md)

The world model is reusable substrate. Game-system packs may define concrete
record families, content schemas, and derived views, but they must register
through typed storage/query contracts and must not gain direct mutation
authority over the underlying stores.

[Simulation Transition Compiler](simulation-transition-compiler.md) depends on
the world model for query inputs, derived-view plumbing, provenance, and
invalidation boundaries. It does not turn derived views into hidden truth.

## Core Principle

Hard truth must be typed, inspectable, and separated from belief and meaning.

```text
physical possession != social claim != actor belief
stored fact != derived view
authoritative state != actor projection
```

The model should support deep local simulation without collapsing the whole
world into either raw ECS components or an untyped fact graph.

## Chosen Shape

Use a typed hybrid model:

```text
WorldModel
  WorldStore
  RelationStore
  EventHistoryStore
  RuntimeControlStore
  SocialInstitutionalStore
  ChronologyStore
  EpistemicStore
  AppraisalRecordStore
  DerivedViewRegistry
  QueryLayer
  CausalTransactionGate
```

ECS-like storage is allowed inside `WorldStore`, but raw ECS mutation is not
the public game API. Fact/relational projections are allowed for queries,
history, provenance, and semantic context, but a universal untyped fact graph
is not the hard-truth core.

`CausalTransactionGate` in this shape names the model-adjacent apply boundary
for accepted hard commits. It does not mean `world-model` owns causal
transaction construction, effect interpretation, invariant checking, or commit
authority. Those semantics belong to the causal runtime; the model owns the
stores and the narrow receiver surface needed to apply accepted changes.
In the current model-substrate implementation, that receiver surface is not a
public write API. Accepted hard, runtime-control, social, chronology,
epistemic, and appraisal package construction remains a later runtime or engine
facade concern.

## Authority Store Families

The world model hosts stores. It does not make every store hard truth.

```text
Hard truth:
  WorldStore
  hard RelationStore families
  RuntimeControlStore
  EventHistoryStore

Soft social/institutional truth:
  SocialInstitutionalStore
  soft RelationStore families

Soft chronology/world-context truth:
  ChronologyStore

Actor truth:
  EpistemicStore

Appraisal / motivation state:
  AppraisalRecordStore
```

Ownership rule:

```text
World Model owns storage, indexing, query surfaces, and provenance plumbing.
Domain design documents own record meaning, lifecycle, and validation rules.
Truth Boundary owns which commit surface may write each authority class.
```

Commit surfaces:

```text
CausalTransaction:
  writes hard truth stores and appends EventRecord entries.

AcceptedSocialUpdate:
  writes social/institutional soft truth.

AcceptedChronologyRecord:
  writes authored, generated, or AI-accepted soft chronology.

AcceptedEpistemicUpdate:
  writes holder-relative EpistemicRecord entries.

AcceptedAppraisalRecord:
  writes Thought, Pressure, GoalPressure, and related appraisal records.
```

No commit surface may write outside its authority class. A hard
`CausalTransaction` may create `EventRecord`s that later become provenance for
social, epistemic, chronology, or appraisal commits, but it does not write
those non-hard stores directly.

## Store Responsibilities

### WorldStore

Owns current typed state for entities and physical objects.

Examples:

- runtime entity handles
- persistent entity ids
- physical form
- material composition
- body and body-part state
- conditions
- containers
- equipment slot definitions
- terrain features
- structures
- local map/cell occupancy where performance requires direct storage

Implementation may be ECS-like:

```text
entity id
  + typed components
  + storage optimized for hot local queries
```

Rules:

- ECS entity handles are runtime handles, not stable story identity.
- Stable ids must exist for save/load, replay, memory, history, and semantic
  references.
- Components are storage facts, not the whole ontology.
- Arbitrary systems must not mutate components outside the causal transaction
  gate.

### RelationStore

Owns typed edges between world objects.

Initial relation families:

```text
ContainedIn
EquippedInSlot
AttachedTo
EmbeddedIn
LocatedIn
PassageTo
MemberOf
SocialClaimOn
```

These families do not all have the same authority class. Physical/topological
relations such as `ContainedIn`, `EquippedInSlot`, `AttachedTo`, `EmbeddedIn`,
`LocatedIn`, and `PassageTo` are hard truth. Social or institutional relation
families such as `MemberOf` and `SocialClaimOn` are committed soft truth and
must pass through the social/institutional commit gate.

Use relation indexes by subject, relation, and object where query pressure
requires inverse or closure queries.

Rules:

- Inventory, equipment, body slots, attachments, embedded objects, interiors,
  and abstract places should share the same typed topology/containment
  vocabulary where possible.
- `LocatedIn` is resolution-aware. Its target may be a tile, local place,
  route segment, region, or other typed place depending on active simulation
  resolution.
- Physical possession is a containment/equipment fact.
- `SocialClaimOn` is a social/institutional assertion, not physical possession.
- Relation schemas must stay typed. Do not drift into arbitrary predicate
  strings.
- Social and institutional relation meaning is defined by
  [Social Institutional Model](social-institutional-model.md). This document
  only defines how typed state and relations can be stored and queried.

### EventHistoryStore

Owns committed hard `EventRecord`s and their transaction envelopes.

Initial record families:

```text
TransactionRecord
EventRecord
MutationTraceRef
```

Responsibilities:

- preserve committed causal evidence
- support replay and audit
- give semantic interpretation rules structured input
- let other stores point to concrete hard evidence through `EventRecord` and
  `TransactionRecord` references

Rules:

- Mutation traces may exist for debug, but `EventRecord`s are the meaningful
  hard records used by gameplay and interpretation.
- Generated or authored history does not live here as hard causal truth. It
  belongs to `ChronologyStore` unless a later multi-resolution or scenario
  transition commits hard state through the causal runtime.

### RuntimeControlStore

Owns hard runtime control state.

Initial families:

```text
ProcessInstance
Reservation
ScheduledWakeup
RngStream
RngDraw
```

Responsibilities:

- long-running processes
- action/process reservations
- scheduler state
- deterministic RNG state and draw provenance

Rules:

- Runtime control state is hard state because it affects validation, replay,
  interruption, and future mutation.
- A `Reservation` is temporary runtime conflict-control state.
- A `Reservation` is not a `SocialClaim`.

### Resolution-Aware State

Resolution changes do not replace entity identity or process identity. They
change which state surface is currently authoritative.

Examples:

```text
Concrete location:
  LocatedIn(actor_1, tile_12_08)

Abstract location:
  LocatedIn(actor_1, north_road_segment_3)
  RoutePosition(actor_1, route=north_road_to_old_mill, progress=0.53)

Strategic location:
  LocatedIn(actor_1, north_road_region)
```

Rules:

- Current location is hard truth at the active granularity.
- Previous concrete positions may remain as provenance, observations, or
  epistemic records, but they should not remain current exact location if the
  engine is no longer maintaining tile-level movement.
- Movement processes update location and route progress at the active
  resolution.
- Promotion refines coarse location into concrete placement.
- Demotion may coarsen current location, release local-only reservations, and
  invalidate local derived views.
- Hard changes caused by promotion, demotion, and route progress still commit
  through `CausalTransaction`.

### SocialInstitutionalStore

Owns committed social and institutional soft truth.

Initial record families:

```text
Relationship
Faction
Institution
Membership
Rank
Office
SocialClaim
Norm
Law
Taboo
Permission
Obligation
Oath
Debt
Reputation
Jurisdiction
```

Some social state may be stored internally as typed `RelationStore` families,
such as `MemberOf` or `SocialClaimOn`. That is a storage choice, not a change
of authority class.

Rules:

- Social/institutional state is committed soft truth, not hard physical state.
- Meaning and validation rules are owned by
  [Social Institutional Model](social-institutional-model.md).
- Writes require an accepted social commit such as `AcceptedSocialUpdate`.
- Social state may reference hard `EventRecord`s, `EpistemicRecord`s, or
  proposals as provenance, but it must not rewrite them.

### ChronologyStore

Owns authored, generated, or AI-accepted soft chronology and world-context
history.

Initial record families:

```text
HistoricalEvent
GeneratedChronology
SiteHistory
ArtifactHistory
EventCollection
```

Rules:

- Chronology records are not hard causal truth.
- Chronology records should use compatible identity and reference conventions
  with `EventRecord`s so memories, rumors, sites, artifacts, and generated
  backstory can point to each other.
- If a generated historical claim must become hard physical state, it must be
  materialized later through the causal runtime.

### EpistemicStore

Owns holder-relative actor truth records.

Initial record family:

```text
EpistemicRecord
```

Rules:

- The world model hosts the store and query indexes.
- [Epistemic State](epistemic-state.md) owns record meaning, holder semantics,
  persistence rules, access, confidence, salience, contradiction, and
  retrieval behavior.
- Actors, factions, cultures, and places are holders. They do not directly own
  large memory arrays.
- Writes require an accepted epistemic commit such as
  `AcceptedEpistemicUpdate`.
- Actor-relative query surfaces must filter by epistemic holder. A global
  epistemic count is a debug or semantic-context concern, not actor-relative
  visibility.

### AppraisalRecordStore

Owns accepted appraisal and motivation records that can affect later behavior.

Initial record families:

```text
Thought
Pressure
GoalPressure
```

Rules:

- Appraisal records are not hard truth, social truth, or epistemic state.
- [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md)
  owns their meaning, taxonomy, scoring, decay, and reactivation rules.
- Writes require an accepted appraisal commit such as
  `AcceptedAppraisalRecord`.
- Appraisal records may reference `EventRecord`, `ObservedEvent`,
  `EpistemicRecord`, `SocialClaim`, and `SocialContextView` provenance.

## Terminology

Use these terms consistently:

```text
Reservation:
  temporary runtime fact that reserves an actor, object, place, resource, role,
  or time slot for a process or action.

SocialClaim:
  social, legal, institutional, or customary assertion such as ownership,
  office, permission, right, debt, or obligation.

Belief over SocialClaim:
  holder-relative epistemic record whose content is a SocialClaim reference.

Physical possession:
  hard containment/equipment state.
```

Plain `Claim` should not appear in design docs without a qualifier.

## Derived Views

Derived views are computed from authoritative state. They may be cached, but
they are not directly mutable hard truth.

Initial derived view families:

```text
containment closure
reachability
visibility
passability
material exposure
body capability inputs
actor-observed facts
semantic context views
debug/provenance explanations
```

Rules:

- Every cached derived view must be rebuildable from authoritative stores.
- Derived views must declare dependencies or invalidation sources.
- Causal transactions should report which families changed so derived views can
  invalidate or refresh safely.
- Actor-facing derived views must go through actor projection and must not read
  omniscient hard truth directly.

## Query Layer

The query layer exposes typed read surfaces.

```text
Kernel query:
  privileged hard-truth read used by validation and causal transactions.

Derived engine query:
  reachability, passability, containment closure, material exposure,
  capability inputs.

Actor-relative query:
  observed state, EpistemicStore retrieval, known secrets, perceived
  affordances.

Semantic context query:
  ObservedEvents + SocialInstitutionalStore records + EpistemicStore working
  set + appraisal records where the downstream layer is allowed to see them.

Debug query:
  omniscient state plus provenance and derivation explanations.
```

Rules:

- Queries declare which truth layer they read.
- Actor and AI-agent interfaces must use actor-relative query surfaces.
- Query surfaces may compose several store families, but they must label which
  authority classes were read.
- Dynamic query languages are acceptable for read-only tooling and debugging,
  not for unchecked authoritative mutation.
- Implementation can stage query depth. Kernel and debug reads may become
  functional before full actor-context projection, but actor-relative and
  semantic query surfaces must still carry actor/scope and authority labels.

## Mutation Boundary

The only hard-truth write path is:

```text
Typed Effect Program
  -> CausalTransaction
  -> CausalTransactionGate
  -> hard truth stores
  -> EventHistoryStore append
  -> derived-view invalidation
```

Non-hard gameplay records also require explicit commit surfaces:

```text
Social proposal / social rule result
  -> social commit gate
  -> AcceptedSocialUpdate
  -> SocialInstitutionalStore or soft RelationStore families

Worldgen / authored / AI chronology proposal
  -> chronology commit gate
  -> AcceptedChronologyRecord
  -> ChronologyStore

Observation / testimony / AI memory proposal
  -> epistemic persistence gate
  -> AcceptedEpistemicUpdate
  -> EpistemicStore

Appraisal rule result / AI appraisal proposal
  -> appraisal commit gate
  -> AcceptedAppraisalRecord
  -> AppraisalRecordStore
```

No world store, relation store, process store, event listener, semantic rule,
AI proposal, or debug tool should become a hidden mutation path. Non-hard
commit gates may reference hard `EventRecord`s as provenance, but they must not
rewrite hard truth or bypass their own authority layer.

The public `world-model` surface is read-first until those commit gates are
implemented. Public callers can create the model, borrow read-only stores,
inspect read labels, and use query surfaces. They cannot construct committed
hard records, accepted non-hard records, runtime-control records, or apply
packages directly through public model APIs. Rust has no friend-crate
visibility, so final authority is enforced by private fields, narrow APIs, and
ownership by higher runtime or engine facades rather than by exposing public
constructors and trusting convention.

## Extension Rule

When adding a new concept, split it by responsibility:

```text
hot current physical state:
  WorldStore

typed relationship:
  RelationStore

committed hard occurrence:
  EventHistoryStore

ongoing runtime control:
  RuntimeControlStore

committed social or institutional state:
  SocialInstitutionalStore or soft RelationStore family

authored/generated chronology:
  ChronologyStore

actor-specific observation or belief:
  actor projection / EpistemicStore

context-dependent meaning:
  AppraisalRecordStore or downstream semantic layer

computed fact:
  DerivedViewRegistry
```

Example: disease

```text
WorldStore:
  infection condition, fever, immunity

RelationStore:
  exposure relation, contaminated source if needed

RuntimeControlStore:
  disease progression process

EventHistoryStore:
  DiseaseContracted, FeverWorsened, DiseaseRecovered

EpistemicStore:
  actor believes disease came from cursed well

DerivedViewRegistry:
  contagious(actor), weakened(actor)

Semantic layer:
  village interprets plague as curse or taboo, producing appraisal records
```

## Current Open Questions

- Which relation families belong in the first kernel subset?
- Which derived views must be incremental from day one?
- How closely should `ChronologyStore` references mirror `EventRecord`
  identity conventions?
- Which world stores are ECS-like internally, and which should be custom typed
  tables?
- What is the minimum provenance format for derived views?
