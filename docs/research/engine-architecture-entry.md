# Engine Architecture Research Entry

## Status

Research entry

## Purpose

This document is the entry point for designing the engine before designing the
game.

The goal is to identify research axes that should be studied and designed
together. An axis is not a feature category, a runtime module, or a philosophical
layer. It is a cluster of decisions that cannot be made well in isolation.

This is not a build roadmap, a vertical slice, or a content roadmap.

## Target

`world` should become a simulation engine for a single-protagonist RPG with:

- deep physical and social causality
- actor-owned action interfaces
- typed action effects over kernel primitives
- structured events as facts
- actor-specific perception, memory, belief, and intent
- semantic interpretation above hard truth
- optional AI co-authorship without direct mutation of hard truth
- deterministic replay where the engine chooses to require it

The central question is:

```text
What engine architecture lets many systems compose without every feature
becoming a bespoke hardcoded subsystem?
```

## Existing Anchors

Current design notes already define several important boundaries:

- [Action and Event Model](../design/action-event-model.md)
- [Simulation Core](../design/simulation-core.md)
- [Simulation Transition Compiler](../design/simulation-transition-compiler.md)

Implementation-oriented follow-up research:

- [Implementation Architecture And Library Survey](implementation-architecture-and-library-survey.md)

Historical or deferred idea sources that informed the current design:

- [Kernel Primitives](../ideas/kernel-primitives.md)
- [Typed Action Effects](../ideas/typed-action-effects.md)
- [Actor-Owned Capability-Derived Actions](../ideas/capability-derived-actions.md)
- [Actor Intent And Activity](../ideas/actor-intent-and-activity.md)
- [Semantic Kernel And PL Boundary](../ideas/semantic-kernel-and-pl-boundary.md)
- [Layered Truth And AI Coauthority](../ideas/layered-truth-and-ai-coauthority.md)
- [Multi-Resolution Simulation](../design/multi-resolution-simulation.md)

This entry document should not duplicate those notes. It should organize the
next research questions around them.

## Axis Selection Rule

Research axes should be grouped by dependency, not by surface similarity.

Use this test:

```text
If two decisions must be made together to avoid designing the wrong abstraction,
they belong in the same research axis.

If a topic needs different references, different failure modes, or a different
modeling vocabulary, it should remain a separate research axis even if it will
eventually run on the same engine substrate.
```

This means:

- `Event Log / Replay / Save` belongs with causal runtime, because event
  granularity, action resolution, process scheduling, and replay cannot be
  designed separately.
- `Query / Indexing / Derived Views` belongs with world representation, because
  query shape follows state shape.
- `Testing / Inspection` belongs with PL-aided authoring and verification,
  because explainability and validation must be designed into the authoring
  surface.
- `Action / Capability / Affordance / Typed Effects` should be split:
  - typed effect execution belongs to causal runtime
  - capability-derived action repertoire and perceived affordance belong to
    actor perspective
- Physical simulation, semantic/social motivation, and multi-resolution
  simulation remain separate axes because each needs its own focused research.

## Research Axes

### 1. World Representation / Query Model

Design output:

- [World Model](../design/world-model.md)

Research result:

- [World Representation / Query Model](world-representation-query-model.md)

Core question:

```text
How should world facts be stored, indexed, derived, and queried?
```

This axis covers the representation of hard world state and the query surfaces
that expose it:

- entity identity
- component, relation, or fact model
- location, topology, containment, and spatial structure
- bodies, items, terrain, materials, substances, conditions, inventories, and
  equipment as stored facts
- derived facts, caches, indexes, and invalidation
- privileged engine queries, actor-relative queries, and debug queries

These belong together because the storage model determines what queries are
natural, what derived facts are cheap, and what invariants are enforceable.
Choosing an ECS, relational, graph, Datalog, or hybrid model is not separable
from deciding how containment, perception, capability derivation, and semantic
rules will ask questions.

Important pressure:

- Hard truth must be authoritative and inspectable.
- Derived views must not become hidden sources of truth.
- The model should support physical, social, and epistemic facts without
  collapsing them into one untyped tag soup.
- Query/index design must not create mutation paths around the causal runtime.

Initial research seeds:

Theory baseline:

- ECS and data-oriented storage models
- relational and graph-style world models
- Datalog/fact systems and incremental derived views
- spatial, containment, and topology indexes
- save/snapshot implications of state representation

Reference candidates:

- Flecs and Bevy ECS, for entity/component storage and query pressure.
- Souffle and Datomic-like systems, for fact/query/derivation pressure.
- CDDA, Caves of Qud, and Dwarf Fortress, as world-model stress tests.

First questions:

- Should hard truth be component-shaped, relation-shaped, fact-shaped, or
  hybrid?
- How should containment and topology be represented so inventory, equipment,
  maps, bodies, and abstract places share one model?
- Which derived facts may be cached, and how are they invalidated or explained?

Expected output:

- authoritative state families
- entity/relation/fact model decision
- containment and topology representation
- derived-view and cache ownership rules
- query taxonomy and permissions

### 2. Causal Runtime / Action-Effect-Event

Design outputs:

- [Causal Runtime](../design/causal-runtime.md)
- [Typed Effect Primitives](../design/typed-effect-primitives.md)
- [Time Model](../design/time-model.md)

Research result:

- [Causal Runtime / Action-Effect-Event](causal-runtime-action-effect-event.md)
- [Time Model / Turn Scheduling](time-model-and-turn-scheduling.md)

Core question:

```text
How does an attempted change become validated world mutation and recorded fact?
```

This axis covers the world-changing runtime:

- turn, tick, or event scheduling
- action request submission
- validation and failure semantics
- typed effect execution over kernel primitives
- process/activity runtime
- interruption, reservation, reaction, and conflict resolution
- event emission and event granularity
- RNG, determinism, replay, save/load, snapshots, and provenance

These belong together because action granularity, event granularity, process
progression, replay determinism, and interruption semantics constrain each
other. A long activity cannot be designed independently from the event log that
explains it or the scheduler that interrupts it.

Important pressure:

- Actions are attempts; events are facts.
- Actions should be typed effect programs over kernel primitives, not one
  opaque resolver per feature.
- Long activities should be serializable, interruptible, resumable, and
  explainable.
- Passive processes must use the same causal discipline as actor actions.
- Determinism should be explicit: required for hard replay paths, relaxed only
  behind declared AI or soft-truth gates.

Boundary:

- Actor-owned capability and perceived affordance are not owned here; they
  shape actor-facing action construction.
- Physical simulation details are not owned here; this axis provides the
  generic process/effect/event machinery they use.
- Semantic meaning is not owned here; this axis records what happened.

Initial research seeds:

Theory baseline:

- discrete-event simulation
- process-based simulation
- command/event models
- event sourcing
- deterministic lockstep and replay
- transaction, rollback, and provenance models

Reference candidates:

- SimPy and ns-3, for scheduling and event simulation vocabulary.
- Event sourcing and deterministic lockstep literature, for replay boundaries.
- RimWorld jobs and CDDA activities, for long-running action pressure.

First questions:

- What is the narrow waist: `ActionRequest`, effect program, event, or process?
- How granular must events be to support replay, observation, memory, and
  debugging?
- How do interruption, reservation, and partial progress become structured
  events rather than hidden state mutation?

Expected output:

- scheduler semantics
- `ActionRequest` lifecycle
- typed effect IR and interpreter boundary
- `ProcessInstance` model
- event schema and granularity rules
- replay/save/provenance contract

### 3. Physical Simulation Grammar

Design output:

- [Physical Simulation Grammar](../design/physical-simulation-grammar.md)

Core question:

```text
What physical vocabulary makes the world deep without becoming a full
continuous physics engine?
```

This axis covers the discrete RPG physics grammar:

- materials, substances, properties, and transformations
- durability, force, temperature, wetness, contamination, residue, charge, and
  other physical qualities
- fire, poison, liquid, gas, smoke, smell, sound, light, and similar
  propagation or field-like phenomena
- body-part damage, wounds, embedded objects, bleeding, healing, senses, and
  physical capability pressure
- terrain transformation, construction, destruction, crafting consequences, and
  environmental hazards
- passive physical processes

This axis remains separate because physical simulation has its own reference
set and failure modes. It uses world representation and causal runtime, but it
must decide its own vocabulary: what should be a material property, a substance,
a field, a residue, a signal, an entity, a condition, or a process.

Important pressure:

- `field`, `trace`, `residue`, `contamination`, and `signal` should not become
  unrelated systems if a unified material/substance model can express them.
- The model should support combat, stealth, crafting, building, exploration,
  tools, monsters, magic, and environmental consequences through shared
  physical grammar.
- It should be discrete and game-suitable, not a continuous real-world physics
  engine.
- It must feed perception, action validation, passive processes, memory, and
  semantic interpretation.

Initial research seeds:

Theory baseline:

- material/property systems
- discrete field and propagation models
- falling-sand and cellular-automata models
- body-part damage and condition models
- terrain transformation and construction grammars
- process coupling between physics, perception, and events

Reference candidates:

- CDDA, for materials, fields, body parts, activities, and survival pressure.
- Caves of Qud, for object/material interaction and exotic physical effects.
- Dwarf Fortress and Noita, for deep material/world interaction pressure.

First questions:

- What is the common representation for substance, residue, signal, field, and
  contamination?
- Which physical facts belong to hard truth, and which are derived capability
  or perception facts?
- How far can discrete physical grammar go before feature-specific systems are
  more honest?

Expected output:

- material/substance/property vocabulary
- field/residue/signal model
- body and wound simulation boundary
- passive physical process families
- limits of physical generality

### 4. Actor Perspective / Epistemic Interface

Design output:

- [Perception And Observation](../design/perception-and-observation.md)
- [Epistemic State](../design/epistemic-state.md)
- [Capability, Affordance, And Actor Interface](../design/capability-affordance-and-actor-interface.md)

Research result:

- [Epistemic State / Agent Memory](epistemic-state-and-agent-memory.md)

Core question:

```text
What can each actor perceive, know, believe, remember, and attempt?
```

This axis covers the actor-facing world interface:

- perception projection
- senses and observation channels
- remembered map and stale information
- knowledge, belief, false belief, rumor, secrets, recipes, laws, names, and
  procedures
- actor-owned capability derivation
- actor-owned action repertoire
- perceived target/context affordances
- compact AI-agent input and output
- invalid-action feedback visible to actors or agents

These belong together because actor action space cannot be designed separately
from what the actor knows and perceives. An AI-native game especially needs a
stable, non-omniscient interface: actor-owned action schemas plus perceived
affordances, not a target-created action list.

Important pressure:

- AI agents and NPCs must not receive omniscient state.
- Actor action space should come from actor-owned capabilities: body, skills,
  equipment, knowledge, conditions, magic, and internalized authority.
- External objects expose perceived affordances and context; they do not own
  the actor's action repertoire.
- Belief must be able to diverge from hard truth.
- Knowledge can change observation, action repertoire, social interpretation,
  and intent.

Boundary:

- Typed effect execution belongs to causal runtime.
- Social meaning belongs to semantic/social motivation.
- Hard truth remains in world representation; this axis controls projection,
  belief, and actor-facing access.

Initial research seeds:

Theory baseline:

- partially observable environments
- BDI-style agent models
- epistemic logic and belief revision
- game perception and memory systems
- LLM-agent environment APIs
- embodied agent interfaces

Reference candidates:

- POMDP and BDI agent models, for partial observation and belief-driven action.
- Generative Agents and Voyager-like systems, for memory and agent loops.
- Stealth, survival, and simulation games, for perception and epistemic limits.

First questions:

- What exact interface should an actor receive instead of omniscient state?
- How does actor-owned capability become action repertoire without reading
  target-owned affordances as action ownership?
- Which beliefs are engine-owned, which are agent-private, and how can they be
  wrong?

Expected output:

- observation projection contract
- memory and belief representation
- capability-to-action-repertoire derivation
- perceived affordance model
- actor-facing query interface
- `AgentTurnInput` and `AgentTurnOutput` contract

### 5. Semantic / Social / Motivation Layer

Design outputs:

- [Truth, Authority, And Layer Boundaries](../design/truth-authority-and-layer-boundaries.md)
- [Social Institutional Model](../design/social-institutional-model.md)
- [Semantic Appraisal And Motivation](../design/semantic-appraisal-and-motivation.md)
- [Intent Templates And Planning](../design/intent-templates-and-planning.md)

Core question:

```text
How do events become meaning, and how does meaning change future behavior?
```

This axis covers interpretation above hard truth:

- semantic interpretation of events
- relationships and social identity
- institutions, law, taboo, ownership as social claim, debt, promise,
  reputation, role, rank, and permission
- witness reports, rumor, social memory, and local knowledge propagation
- interpreted memory, thought, emotion pressure, intent, and activity selection
- revenge, grief, loyalty, fear, shame, duty, obligation, and caution as
  motivated behavior pressures
- AI-assisted soft-truth interpretation under explicit gates

These belong together because social meaning, memory, pressure, and intent form
one loop. A physical event is observed, interpreted in social context, stored as
experience, converted into pressure, and later biases intent or activity.

Important pressure:

- The kernel records physical facts; this layer decides what those facts count
  as.
- The same event may mean different things to different observers.
- Social and emotional consequences should usually derive from interpreted
  experience, not raw scripted triggers.
- AI may enrich soft truth, but it must not mutate hard truth directly.
- Intent can exist at abstract or concrete resolution, but concrete action still
  resolves through the causal runtime.

Initial research seeds:

Theory baseline:

- institutional facts and constitutive rules
- social simulation and reputation systems
- event calculus and temporal social facts
- rule-based legal/social interpretation
- motivation, pressure, utility, and intent selection models
- LLM-assisted interpretation with provenance

Reference candidates:

- RimWorld, for thought/need/pressure-to-behavior pressure.
- Dwarf Fortress and RPG faction/reputation systems, for social memory and
  history pressure.
- Institutional-rule and event-calculus research, for meaning over facts.

First questions:

- What turns a physical event into an interpreted social fact?
- How do interpreted memories update pressure and intent without hardcoding
  every scenario?
- Where can AI enrich soft truth without contaminating hard truth?

Expected output:

- semantic interpretation boundary
- relationship and institution model
- interpreted-memory update flow
- pressure/intent/activity model
- social provenance and explanation contract
- AI coauthority gates for soft truth

### 6. Multi-Resolution Simulation

Design output:

- Current design document:
  [Multi-Resolution Simulation](../design/multi-resolution-simulation.md).
  Source history remains in
  [source idea note](../ideas/multi-resolution-simulation.md).

Core question:

```text
How does the world remain causally and semantically coherent when it is not
locally simulated?
```

This axis covers simulation across levels of detail:

- local concrete action/effect/event simulation
- nearby abstract intent, process, and risk
- distant strategic state, pressure, and summarized progress
- promotion and demotion between resolutions
- abstract event provenance
- rumor, report, and historical summary generation
- consistency between regional progress and local reification

This axis remains separate because it is not just optimization. It asks how a
world can continue away from the protagonist while still producing events,
memories, rumors, social consequences, and later local details that make sense.

Important pressure:

- Distant simulation should not pretend every actor took concrete local turns.
- Abstract state still needs causal provenance.
- Nearby abstract simulation can share intent/process concepts with local
  simulation.
- Far distant simulation may operate mostly through pressures, resources,
  relationships, and summarized state.
- AI coauthorship may be useful here, but only with explicit truth-layer and
  provenance boundaries.

Initial research seeds:

Theory baseline:

- level-of-detail simulation
- regional and agent-based simulation
- abstract event generation
- promotion/demotion consistency
- history summarization and reification
- AI-assisted soft truth under constraints

Reference candidates:

- Level-of-detail simulation and regional agent simulation work.
- Games with persistent off-screen worlds, abstract factions, or strategic
  layers.
- Narrative/history summarization systems, for reifying abstract progress.

First questions:

- What state exists at distant, nearby, and local resolution?
- How does abstract progress become local concrete state without contradiction?
- What provenance is required when AI helps summarize or reify distant events?

Expected output:

- resolution tier contract
- abstract state and process vocabulary
- promotion/demotion rules
- abstract event provenance model
- constraints on AI-generated soft truth

### 7. PL-Aided Authoring / Verification / Inspection

Design output:

- Deferred future design document. Current constraints live in
  [Truth, Authority, And Layer Boundaries](../design/truth-authority-and-layer-boundaries.md),
  [Typed Effect Primitives](../design/typed-effect-primitives.md), and
  [Simulation Core](../design/simulation-core.md).

Core question:

```text
How can engine behavior be authored, checked, explained, and evolved without
collapsing into hardcoded feature piles?
```

This axis covers the language and tooling layer around the engine:

- content registry
- schemas and content validation
- typed action/effect language
- process/activity DSL
- semantic rule language
- query or derived-fact language
- effect checking and permission checking
- migrations and versioning
- inspectors, replay viewers, provenance traces, explainers, tests, and lints

This axis is not the truth owner. The PL should not replace the kernel, storage
model, scheduler, primitive effect semantics, replay contract, or RNG contract.
It should express, constrain, check, and explain behavior over engine-owned
truth.

This makes it secondary in authority but strategic in value. It is where the
project can use PL/compiler research most productively: not by forcing
everything into a language, but by using language design where hardcoding would
make actions, processes, semantic rules, content, and explanations
unmanageable.

Important pressure:

- Primitive mutation semantics belong to the engine, not to arbitrary scripts.
- Authored content should compose through typed primitives and checked effects.
- Semantic rules should be inspectable and provenance-backed.
- Designers and debuggers need answers to why an action was unavailable, why an
  effect happened, why an NPC chose an intent, why a belief is false, and why an
  event counted as a crime.
- Tests should target invariants and replayable scenarios, not only examples.

Initial research seeds:

Theory baseline:

- typed effect languages
- algebraic effects
- Datalog and rule systems
- PDDL/STRIPS-style action schemas
- schema languages and content validation
- static analysis and property-based testing
- trace and explanation systems

Reference candidates:

- Koka and algebraic effect systems, for typed effect vocabulary.
- Datalog/rule engines, for derived facts and semantic rules.
- PDDL/STRIPS, for action schema and precondition pressure.
- Game scripting, modding, and data-driven content systems, for practical
  authoring failures.

First questions:

- Which parts need formal semantics before any DSL is designed?
- Which authored behaviors need type/effect checking, and which should stay in
  host code?
- What explanation data should the PL/checker require from every authored
  rule?

Expected output:

- content definition families
- typed effect checker shape
- process DSL boundary
- semantic rule checker shape
- query/derived-fact language boundary
- migration/versioning model
- debug/inspection contract
- simulation test taxonomy

## Cross-Axis Decisions

Some decisions cut across many axes and should be tracked explicitly:

- What must be deterministic, and what may be intentionally non-deterministic?
- What can AI coauthor, and what can AI never mutate directly?
- Which facts are authoritative, derived, believed, semantic, or presentational?
- Which changes must emit structured events?
- Which layers may query hard truth?
- Which layers may observe only actor-relative truth?
- Where should content be data, where should it be typed effect code, and where
  should it be semantic rules?
- Which abstractions are engine foundations, and which are merely feature
  implementations?
- Which behavior should live in host code, and which behavior should be
  authored through a checked PL/DSL?

## Suggested Research Order

This is a research order, not an implementation order.

1. World representation and query model: establish the state/query substrate.
2. Causal runtime and action-effect-event model: establish mutation, time,
   event, replay, and process semantics.
3. Physical simulation grammar: stress-test the substrate with concrete
   material/body/terrain/process demands.
4. Actor perspective and epistemic interface: define non-omniscient perception,
   belief, capability-derived action space, affordance, and agent I/O.
5. Semantic/social/motivation layer: define interpretation, relationship,
   institution, pressure, intent, and soft-truth gates.
6. PL-aided authoring/verification/inspection: define the checked authoring and
   explanation surface after the first pass of the engine semantics is visible.
7. Multi-resolution simulation: define promotion/demotion and abstract
   provenance once local truth, runtime, actor perspective, and semantic meaning
   are concrete enough to preserve across resolution changes.

The order should be iterative. PL/tooling should not wait until the end, but it
should be shaped by the kernel and semantic boundaries rather than replacing
them.

## Reference Link Index

These links are only a source index for the research seeds above. The axis
sections own the actual research routing.

World representation and query model:

- [Flecs](https://www.flecs.dev/flecs/) and
  [Bevy ECS](https://bevy.org/learn/quick-start/getting-started/ecs/), for
  entity/component storage and query pressure.
- Datalog systems such as [Souffle](https://souffle-lang.github.io/), for
  declarative derived facts.
- Datomic-like event/fact/query thinking, if persistent facts become central.

Causal runtime and action-effect-event:

- [SimPy](https://simpy.readthedocs.io/), for process-based discrete-event
  simulation.
- [ns-3 events and simulator](https://www.nsnam.org/docs/manual/html/events.html)
  and DEVS-style simulation, for event scheduling and reproducibility.
- [Event sourcing](https://www.martinfowler.com/eaaDev/EventSourcing.html) and
  [Akka event sourcing](https://doc.akka.io/libraries/guide/concepts/event-sourcing.html),
  for event logs as state reconstruction.
- [Deterministic lockstep](https://gafferongames.com/post/deterministic_lockstep/)
  literature, for replay and input-log thinking.
- RimWorld and CDDA, for activities, interruption, and practical simulation
  constraints.

Physical simulation grammar:

- CDDA, Caves of Qud, Dwarf Fortress, and Noita, for physical depth,
  interaction density, world materiality, and emergent consequences.
- Roguelike field/substance systems, falling-sand simulations, and cellular
  automata, for discrete physical propagation.

Actor perspective and epistemic interface:

- POMDP and BDI agent models, for partial observation and belief-driven action.
- [Generative Agents](https://arxiv.org/abs/2304.03442) and
  [Voyager](https://arxiv.org/abs/2305.16291)-like LLM agent systems, for
  memory, reflection, and long-horizon agent behavior.
- AI environment APIs, for compact observation/action contracts.

Semantic/social/motivation layer:

- RimWorld, for practical thought, pressure, need, and work-selection pressure.
- Social simulation and institutional-rule research, for turning physical
  events into social meaning.
- Event calculus, for temporal facts and derived interpretations.

Multi-resolution simulation:

- Level-of-detail simulation, agent-based regional simulation, and historical
  summarization systems.
- Games with persistent off-screen worlds, rumors, abstract factions, or
  strategic layers.

PL-aided authoring, verification, and inspection:

- [Koka](https://github.com/koka-lang/koka) and algebraic effect systems, for
  typed effect vocabulary.
- Datalog/rule systems, for derived facts and semantic rules.
- PDDL/STRIPS-style planning languages, for action schemas and preconditions.
- Schema languages, static analysis, and property-based testing, for content
  validation and invariant checking.

## Immediate Use

Use this document as a map when choosing the next reference or research pass.

For each future research note:

- name the engine axis it informs
- explain why the topic belongs to that axis rather than another one
- identify what it treats as authoritative state
- extract what action/event/process/query boundary it implies
- note whether it supports or challenges the current architecture
- avoid copying content features unless they reveal engine pressure
