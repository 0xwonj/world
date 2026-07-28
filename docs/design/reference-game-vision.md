# Reference Game Vision

## Status and purpose

This document defines a small reference game used to pressure-test the target
architecture while it is implemented. It is a product and validation
direction, not a complete game design document and not a second source of
runtime, persistence, or lifecycle contracts.

The normative architecture remains under
[`docs/architecture/target-architecture/`](../architecture/target-architecture/README.md).
When this document and that package disagree, the target architecture wins.
Detailed design documents linked below remain inputs to the reference game;
this document joins them into one coherent player experience and staged
validation fixture.

The reference game exists to prevent two failures:

- designing a clean but gameplay-unproven framework in isolation;
- expanding into a large content project before the causal and authoring
  foundations are ready.

Every implemented reference-game slice must be vertical: it has a real
definition, state, producer, consumer, invariant, authoritative result, and
test. Empty future-facing interfaces and placeholder type catalogs do not
count as progress.

## Product thesis

The reference game is a small, dense frontier-settlement systemic RPG.

```text
one settlement
+ a few nearby sites
+ one distant route and caravan
+ a small population with unequal knowledge, needs, relationships, and duties
+ interacting physical, social, epistemic, and agency systems
-> consequences that continue without the player
```

The intended feel draws from deep simulation RPGs rather than
presentation-led games:

- most simulated objects expose meaningful interactions through reusable
  capabilities and affordances;
- actors act from what they have perceived, remembered, and inferred rather
  than from global world truth;
- appraisal, intent, activity, action, and process have different lifetimes;
- social claims, promises, obligations, reputation, and institutions affect
  behavior without becoming physical truth;
- local play is concrete and tactical while distant work progresses through
  summarized individual activities and processes;
- human, rule, script, and AI controllers use the same actor-control boundary;
- surprising outcomes can be reconstructed from definitions, inputs,
  decisions, authority records, and later causal work.

The reference game is not itself the reusable engine. Its named setting,
content, balance, and progression belong outside the lower architecture.

## Player experience

The player inhabits one actor rather than an omniscient camera.

The actor can reason only from information made available through its own:

- senses and perceptual capabilities;
- observations and accepted evidence;
- memory and beliefs, including uncertainty and false belief;
- learned procedures and cultural knowledge;
- relationships, social position, promises, and obligations;
- current body, equipment, conditions, skills, and active commitments.

The ordinary interaction loop is:

```text
perceive
  -> interpret
  -> commit to an intent
  -> pursue an activity
  -> select one grounded action
  -> receive an authoritative outcome or observable consequence
  -> revise evidence, interpretation, and ongoing commitments
```

NPCs use the same loop at the complexity selected by their lifecycle profile.
A simple creature may use only deterministic rules. An important companion may
use richer appraisal, activity, dialogue, or action evaluators. Disabling a
higher cognitive implementation must not disable basic action behavior.

## Turn shell and virtual time

The player-facing game is turn-based. The simulation kernel is not organized
around fixed player/NPC rounds; it uses deterministic integer virtual time and
typed scheduled work.

```text
player experience:
  choose an action
  -> the world resolves until the next player input opportunity

runtime model:
  SimMoment
  -> globally ordered due work
  -> bounded preparation
  -> one atomic authority publication
  -> later typed causal work
```

Every action or process has declared timing semantics. A concrete duration may
depend on:

- the action's base duration;
- actor capability and learned proficiency;
- body condition, injury, fatigue, or another typed debuff;
- equipment and tool quality;
- stance or activity state;
- terrain, weather, obstruction, or another explicit environmental input;
- execution-configured timing policy.

The exact duration function is deliberately deferred. It must eventually use
checked deterministic arithmetic, declare lower and upper bounds, and record
the behavior-affecting policy in execution semantics. Wall-clock latency,
worker count, and AI response time never determine simulation order.

Zero-time world mutation is exceptional and must be structurally bounded.
Player inspection and other non-authoritative UI actions may consume no
simulation time.

## Spatial model

The reference game uses a hybrid spatial model:

```text
regional topology
  settlements, sites, routes, and coarse travel position

local detailed space
  bounded two-dimensional square grids
```

The local grid makes movement, line of sight, doors, walls, cover, sound,
fire, smoke, object placement, and tactical interactions concrete. The
regional graph makes distant travel and multi-resolution work explicit
without requiring one global tile map.

The square grid is a selected standard-world implementation for this reference
game, not a universal assumption of `world-runtime`. Exact neighborhood,
diagonal movement, path cost, field propagation, and map-partition policies
remain deferred until their first vertical consumer.

## System composition

Deep interaction should arise from the composition of small typed rules rather
than from object-specific callbacks or a universal mutation language.

The shared causal form is:

```text
typed state and definitions
  + capability-scoped immutable input
  -> bounded proposal or selected supplied ID
  -> trusted validation and atomic publication
  -> typed events and scheduled consequences
```

Subsystems may grow sophisticated internally, but they communicate through
typed state, proposals, records, events, observations, and future work. They
do not recursively call one another to mutate shared state.

### Physical and object interaction

The initial object vocabulary should be small and compositional. Representative
typed capabilities or state families include:

```text
Portable
Container
Openable
Lockable
Breakable
Combustible
Consumable or Usable
```

A wooden door may combine barrier, open, lock, integrity, and combustion
state. A chest may combine container, portability, open, lock, and integrity
state. Action grounding joins:

```text
actor-owned capability
  x perceived target affordance
  x available tools
  x local environment
  x action definition
```

to produce fully bound candidates such as `Open`, `Unlock`, `ForceOpen`,
`Ignite`, or `Extinguish`.

Descriptive tags may support discovery or presentation, but authoritative
semantics must not reduce to arbitrary string-keyed properties or
`SetField(path, value)`.

### Processes

Long-running world mechanisms are explicit processes rather than hidden
per-turn callbacks. The reference game eventually exercises:

- one local environmental process, such as fire and smoke;
- one actor activity with interruption and fallback;
- one distant travel process;
- one condition process, such as healing or hunger, only if needed to validate
  timing and capability degradation.

Processes own typed state, wakeups, completion, interruption, and observable
consequences. They do not receive direct publication authority.

### Knowledge and perception

World truth, observation, evidence, belief, and interpretation remain
different records.

The initial slice should demonstrate that:

- two actors can receive different evidence from the same event;
- an unobserved fact cannot appear in a candidate or AI payload;
- a claim is evidence that somebody asserted something, not proof that the
  proposition is true;
- an actor may attempt something reasonable that runtime later rejects because
  its belief was incomplete or stale.

### Social interaction

Social interaction is a first-class game axis rather than presentation around
physical actions. The reference game distinguishes at least:

```text
physical or institutional fact
claim
actor belief
relationship
membership or role
promise, obligation, or contract
reputation or standing
```

The first social slice remains deliberately narrow:

- one typed claim or rumor;
- one relationship dimension relevant to interpretation;
- one promise, obligation, or delivery contract;
- a small structured dialogue-act family such as `Inform`, `Request`,
  `Promise`, and `Refuse`;
- one later consequence that depends on accepted social or epistemic state.

Natural-language dialogue may be generated by an AI evaluator, but the
utterance does not directly set belief, trust, obligation, or world truth.
The structured act, referenced semantic content, captured utterance, delivery,
observation, appraisal, and any accepted social transition remain distinct.

### Agency

The reference actor model uses the target lifecycle separation:

```text
evidence assimilation
  -> appraisal
  -> intent
  -> activity
  -> action opportunity
  -> grounded candidate selection
  -> runtime attempt
```

One reference NPC must preserve an intent across multiple action
opportunities, suffer an expected failure or interruption, and choose a
bounded fallback from fresh actor-relative context.

### Growth

Progression is initially demonstrated as a change in capability and available
action space rather than as a complete level system.

The minimal slice is:

```text
skill, trait, learned procedure, or equipped tool changes
  -> derived actor capability changes
  -> projection or candidate generation changes
  -> one new or improved grounded action becomes available
  -> runtime still performs authoritative legality checks
```

Examples include learning lockpicking, acquiring first-aid knowledge, or
equipping a crowbar. Full skill trees, mutations, spells, professions, body
plans, and progression pacing are later game-system work.

## AI roles

AI participates through three different authority classes.

### Controller

An AI may control an NPC or act for the player. It receives the same
projection-safe control frame as another controller and returns a supplied
candidate ID, wait, reconsideration request, or abstention.

CLI, UI, script, in-process AI, and MCP are adapters over this same boundary.
MCP is not a runtime mutation interface and does not expose unrestricted
snapshots or private candidate resolution data.

### Lifecycle evaluator

An AI may implement one appraisal, intent, activity, dialogue, or action port.
It receives only that port's bounded policy payload and returns its typed
result. An external or nondeterministic result is captured, freshness-checked,
and applied only through later admitted work. Replay uses the captured result
rather than silently invoking the model again.

### Author

An AI may create source content, but its output is always a draft:

```text
user or story need
  -> generated structured source
  -> compile and diagnose
  -> repair
  -> verify
  -> preview
  -> activate through the appropriate boundary
```

Activation depends on semantic impact:

```text
presentation-only material
  -> retained sidecar or presentation artifact

new instances under existing semantics
  -> ordinary authoritative world proposal

new definitions using installed typed IR
  -> new exact definition set and explicit child epoch

new primitive semantics
  -> separately validated engine extension and new semantic epoch
```

M6 implements only one small AI-assisted authoring loop over the existing
foundation T1 action and physical-event definitions with embedded ordered
typed effect calls. It does not introduce reusable T0 content data or later
process/social definition families. Automatic generation and deployment of
new trusted primitive code remains a later research problem.

## Multi-resolution world

The reference game uses the target tiers:

```text
Detailed
  local grid, full actor-relative context, concrete action interaction

Background
  individual identity retained, summarized activity/process, sparse wakes

Dormant
  no recurring evaluation, activation only from an external cause or deadline
```

Resolution is selected per entity-or-phenomenon and subsystem scope, not once
for an entire actor. The same caravan guard may therefore have background
movement, retained social obligations, and dormant unrelated cognition.

Every scope preserves its declared canonical core. Promotion and demotion are
checked transactions with explicit conversion identity, conserved state,
timing, replacement wakeups, and approximation evidence. The initial scaling
slice retains individuals and summarizes their work; population aggregation
is not part of the reference game.

## Reference scenario

The canonical pressure test is **the warehouse fire and delayed caravan**.

Initial conditions:

- the settlement has a limited medicine supply;
- a caravan carrying additional medicine is traveling on a distant route;
- a merchant has a delivery obligation to a local institution;
- a warehouse contains food, medicine, portable crates, and a locked area;
- guards, a merchant, an injured resident, a suspect, and the player begin
  with different locations, relationships, and knowledge;
- at least one actor knows of a fire risk that the others do not.

Possible causal development:

```text
fire starts
  -> material and smoke process advances
  -> witnesses receive different evidence
  -> evacuation, firefighting, theft, investigation, or flight intents form
  -> door, lock, container, tool, and item capabilities create grounded actions
  -> actions contend, fail, interrupt, or create later consequences
  -> claims and rumors change beliefs and social behavior
  -> medicine loss affects the delivery obligation and treatment options
  -> the distant caravan continues in Background
  -> approaching the caravan or settlement promotes the relevant scopes
```

No single playthrough must exercise every branch. The scenario is a reusable
family of deterministic fixtures and black-box validation cases.

## Roadmap validation lens

The stable milestone sequence remains owned by the
[Target Architecture Execution Roadmap](../architecture/target-architecture/implementation-roadmap.md).
The reference game contributes the following validation pressure. M1 through
M4 record the examples selected by their accepted exit reviews. For M5-M8,
the detailed fixture is selected only when that milestone begins. Names,
setting, concrete layout within the selected bounded local square-grid and
regional topology, and content volume may change. The selected topology,
capability, causal interaction, owner boundary, and assertions named by the
roadmap may not be weakened or deferred by choosing a different fixture.

| Milestone | Current reference-game expectation |
|---|---|
| M1 | One exact pack-defined containment transfer reaches the public engine/runtime authority path and is inspectable afterward. |
| M2 | Representative same-moment resource contention resolves deterministically without duplicated effect or skipped causal work. |
| M3 | A representative local-world projection exposes only actor-visible, actually bindable candidates. |
| M4 | Authoritative process control, actor-relative evidence-to-agency recovery, and captured action evaluation share one causal spine; travel arrival itself does not yet complete agency, and the social port remains explicitly disabled. |
| M5 | An in-progress chain restores and verifies without external reevaluation; semantic evolution uses an explicit child epoch. |
| M6 | CLI and MCP-style control are equivalent adapters; captured external evaluation is authenticated and projection-safe; AI-assisted foundation T1 action/event source with embedded effect calls follows compile, diagnose, repair, isolated preview over an existing root, and explicit child-epoch activation. |
| M7 | Positive promotion, demotion, and dormant activation replace obsolete work while retaining individual identity, hard invariants, commitments, and causal time. |
| M8 | The first minimal reusable object-archetype T0 family feeds checked root materialization; only afterward does a separate additional T0/T1 change prove locality. Door/lock/tool/body, material/integrity/heat/fire/smoke, and witness/claim/institution/obligation slices compose with checked process, condition, scenario-required social families, and one concrete T3 primitive in a persistent multi-resolution scenario without a new authority path. |

These are validation slices, not permission to add every named system to the
lower engine. A milestone introduces only the types and implementations
required by its active producer, consumer, invariant, and test.

## Required evidence at roadmap completion

Roadmap completion requires executable evidence for the following capabilities
in one small headless playable scenario family. A milestone may replace
fictional names, exact item choices, concrete layout within the selected local
square-grid and regional topology, and narrative framing. Changing that
topology requires a reference-vision decision. A milestone may not replace
this list with a smaller slice that omits an architectural capability. The
normative owner, milestone, and validation scenario for every item are
recorded in the roadmap traceability matrix.

- one bounded local square settlement grid, one remote site, and one
  connecting regional route;
- a player actor and a small number of rule-controlled actors;
- one first minimal reusable object-archetype T0 family, distinct scenario
  planning provenance, and a checked materialized initial-state root;
- portable items, containers, and a composed door/lock/tool/body interaction;
- material, integrity, heat, fire, and smoke interaction through one local
  checked physical-process definition and stage-specific conditions, plus one
  distant travel process;
- actor-relative perception and unequal evidence;
- one typed witness claim, institutional interpretation, relationship input,
  and obligation;
- one persistent intent/activity and bounded action fallback;
- one capability-changing skill, trait, procedure, tool, or condition;
- equivalent CLI and MCP actor control plus one authenticated external
  evaluator;
- checkpoint, verification replay, and explicit branch;
- positive promotion, demotion, and dormant activation with obsolete-work
  replacement;
- compiler diagnostics and one AI-assisted foundation T1 action/event
  authoring example that activates only through an explicit child epoch;
- one cross-primitive same-moment conflict and one continuous
  physical-to-evidence-to-social-to-agency causal explanation;
- after the first T0 family and baseline root exist, one separate additional
  existing-vocabulary T0/T1 change that requires no content-family,
  primitive-owner, or runtime-kernel change;
- one concrete owner-local T3 primitive whose addition does not change
  authority laws, dependency direction, or unrelated primitive APIs.

The first reusable T0 family is deliberately one concrete content-data
consumer for this scenario, not a universal entity schema, property bag, or
open content registry. Its introduction and the later existing-vocabulary
locality change are separate pieces of evidence.

It does not need production content volume, graphical presentation, a complete
combat or spell system, or a polished campaign. Passing M8 validates and
stabilizes the gameplay composition boundaries; it does not declare the game
content complete.

## Deferred game depth

The following remain post-roadmap game-system or evidence-gated extension
work:

- complete combat, armor, anatomy, wounds, spells, mutations, and progression;
- fluids, gases, temperature, electricity, structural destruction, and a broad
  material simulation;
- large crafting, construction, economy, ecology, religion, law, and faction
  catalogs;
- procedural world generation and a large authored setting;
- free-form natural-language command interpretation;
- fully autonomous AI story direction;
- AI-generated trusted primitive deployment;
- population aggregation, multiplayer authority, real-time rendering, and a
  server/editor product.

These concerns must fit through the established definition, proposal,
authority, lifecycle, and epoch-evolution boundaries. They do not justify
speculative lower-layer extension points before a concrete vertical consumer
exists.

## Decisions fixed by this vision

1. The reference product is player-facing turn-based over deterministic
   discrete-event virtual time.
2. The reference spatial implementation combines bounded local square grids
   with a regional topology.
3. Deep interaction is capability- and affordance-derived using typed domain
   state, not arbitrary property mutation.
4. Social and epistemic interaction is a core validation axis.
5. Natural-language dialogue is captured expression around typed social and
   epistemic acts, not direct mutation authority.
6. Growth is first validated through capability and candidate-space change.
7. AI controller, evaluator, and author are distinct roles.
8. New behavior-relevant definitions or implementations never hot-reload into
   an existing immutable semantic epoch.
9. Multi-resolution preserves one authority, canonical identity, important
   invariants, and causal time.
10. The roadmap proves these choices with thin vertical scenarios culminating
    in the M8 gameplay-composition falsification suite rather than attempting
    a full game.

## Deliberately deferred choices

- the exact local-grid neighborhood and movement-cost rules;
- the final duration formula and balance scale;
- the complete object-capability, material, social, and progression
  vocabularies;
- the final dialogue-act and relationship taxonomies;
- the pack source syntax and action/effect DSL;
- model providers, prompts, memory retrieval, and AI approval policy;
- sandbox technology for future generated implementation code;
- presentation, camera, rendering, and input UX.

## Related documents

- [Vision](../vision.md)
- [Target Architecture Package](../architecture/target-architecture/README.md)
- [Execution Roadmap](../architecture/target-architecture/implementation-roadmap.md)
- [Validation Scenarios](../architecture/target-architecture/validation-scenarios.md)
- [Gameplay Composition And Evolution Research](../research/gameplay-composition-and-evolution-research.md)
- [Design Document Index And Status](README.md)
- [Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md)
- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Physical Simulation Grammar](physical-simulation-grammar.md)
- [Social Institutional Model](social-institutional-model.md)
- [Epistemic State](epistemic-state.md)
- [Intent Templates And Planning](intent-templates-and-planning.md)
- [Multi-Resolution Simulation](multi-resolution-simulation.md)
- [Standard World Library And Primitive Semantics](standard-world-library.md)
- [Caves of Qud Reference](../references/caves-of-qud.md)
- [Cataclysm: Dark Days Ahead Reference](../references/cataclysm-dda.md)
- [RimWorld Reference](../references/rimworld.md)
