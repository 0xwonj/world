# Cataclysm: Dark Days Ahead

## Why It Matters

Cataclysm: Dark Days Ahead is useful for `world` not because its genre matches
the target game, but because it is a large open-source systemic simulation with
a visible boundary between data definitions and hardcoded engine behavior.

CDDA is a post-apocalyptic survival roguelike. `world` is aiming at a
single-protagonist fantasy RPG with richer semantic, social, and narrative
layers. The transferable question is narrower:

```text
Which concepts did a deep systemic game need as engine primitives, and which
could it expose as authored data?
```

This makes CDDA a pressure test for the current kernel primitive hypothesis.

## Research Focus

Study CDDA mainly through these lenses:

- item, material, body, wound, field, terrain, and activity representation
- JSON content versus C++ behavior boundary
- long activities versus immediate actions
- material properties and environmental effects
- tool qualities, flags, and action repertoire pressure
- crafting, construction, and item transformation
- limitations created by accumulated hardcoded actions
- what should not be copied into a fantasy RPG semantic architecture

The goal is not to import CDDA's feature list. The goal is to learn where a
large simulation needs explicit ontology.

## Core Observation

CDDA has a very large data surface, but it is not a pure data-driven engine.

Its content definitions are broad:

- items
- materials
- body parts
- wounds
- fields
- terrain
- furniture
- recipes
- constructions
- activities
- factions
- NPCs
- dialogue
- effects on conditions
- tool qualities
- flags
- map generation

But many behaviors still require C++ actors, hardcoded functions, or special
engine hooks.

For `world`, this is the key lesson:

```text
Data-driven content does not remove the need for a small, explicit, typed
kernel vocabulary.
```

The better target is not "everything in JSON." The better target is:

```text
Content authors define typed data and typed effect programs.
The kernel exposes a small set of checked primitives.
All hard state mutation flows through those primitives.
```

## World Model

CDDA's world model is broad and practical rather than conceptually minimal.

Important state surfaces include:

- local map terrain and furniture
- fields such as fire, smoke, blood, gas, slime, and webs
- overmap locations
- items and item contents
- vehicles and vehicle parts
- monsters
- NPCs
- faction state
- player and NPC bodies
- effects, wounds, mutations, bionics, needs, morale, addictions
- activities and activity backlog
- recipes, constructions, and requirements
- dimensions and region settings

This is close to the kind of system pressure `world` expects, but CDDA's model
is not organized around an action/event/semantic boundary. It grew around
survival simulation needs.

## Data And Code Boundary

CDDA's official JSON docs describe data as many typed JSON objects. Each object
has a `type` member that tells the game how to interpret it, and most core data
lives under `data/json`.

This is useful because the authored data surface is explicit. But the actual
behavior boundary is mixed.

Examples:

- an item can define materials, flags, tool qualities, charges, use actions,
  armor coverage, pockets, damage, and many other fields
- a use action may be a simple built-in function name or a structured actor
  definition
- long activities have JSON properties, but new activity behavior requires C++
  `activity_actor` code
- construction is mostly data, but it still has hardcoded `pre_special`,
  `post_special`, `do_turn_special`, and similar escape hatches
- `effect_on_condition` provides a broad scripting surface, but it is still a
  catalog of engine-known conditions and effects

This suggests that `world` should avoid pretending that authored data alone is
the architecture.

Better:

```text
Define a typed effect language over kernel primitives.
Compile authored actions, processes, and magic into that language.
Keep hardcoded escape hatches explicit and rare.
```

## Item And Material Grammar

CDDA items are not only inventory entries. They can carry:

- material composition
- weight and volume
- damage values
- tool qualities
- charges and power behavior
- use actions
- armor coverage and material thickness
- containers and pockets
- flags
- spoilage, wetness, temperature, and other state through related systems

The material system is especially relevant to `world`.

CDDA material data includes concepts such as:

- damage resistance
- chip resistance
- density
- breathability
- wind resistance
- specific heat
- latent heat
- freeze point
- rotting
- softness
- conductivity
- sheet thickness
- repair difficulty
- vitamins
- fuel data
- burn data
- burn products
- salvage and repair outputs

This supports the idea that `world`'s physical substrate should include real
material properties, not only tags.

However, CDDA also shows the danger:

```text
Material can become a large bag of practical properties unless the engine has a
clear typed model for how properties participate in effects.
```

For `world`, material should probably be a typed physical primitive, but
individual properties should be introduced only when at least two systems use
them.

Example:

```text
flammability:
  fire, crafting, spell effects, building hazards

conductivity:
  lightning, machines, traps, magic, wetness interactions

porosity / absorbency:
  blood, poison, water, scent, ritual residue
```

## Fields, Traces, And Environmental Effects

CDDA has first-class field types. Examples include fire, smoke, blood, acid,
toxic gas, webs, electricity, plasma, slime, sap, cold air, hot air, and many
others.

Fields can have:

- intensity levels
- transparency
- movement cost
- radiation
- light emission
- concentration
- temperature modification
- scent neutralization
- field effects applied to actors
- decay and spread behavior
- immunity data
- bashing behavior

This strongly supports the earlier `world` idea that spatial fields, traces,
smoke, blood, poison, and magical residue should not be separate unrelated
systems.

But CDDA represents many of these as named field types. That is practical, but
it can turn into a catalog of special environmental objects.

For `world`, a better target may be:

```text
Field = spatial distribution of a substance or signal
Trace = residue with source, age, visibility, and decay
Contamination = substance attached to body, item, surface, or container
```

Then fire, smoke, scent, blood, poison gas, holy residue, and magical traces
can share a substrate while still having domain-specific rules.

## Body, Capacity, And Wounds

CDDA has a strong body model compared with ordinary RPG stat blocks.

Body part representation includes:

- body part ids
- sub body parts
- parent and connected body parts
- opposite parts
- limb types
- hit size and hit difficulty
- side
- encumbrance thresholds and limits
- health limits
- body part flags
- conditional flags
- limb scores
- qualities provided by limbs
- innate environmental protection
- base HP
- healing and mending rates
- temperature and wetness behavior
- vital and limb flags
- unarmed damage and techniques
- potential wounds

Wounds are also separate typed data. A wound can specify:

- damage types that can apply it
- required damage range
- pain
- healing time
- body part type whitelist or blacklist
- body part flag whitelist or blacklist

This is highly relevant to `world`.

It supports the current chain:

```text
body state
  + wounds
  + equipment constraints
  + conditions
  -> derived physical capability
  -> actor-owned action repertoire
```

CDDA also suggests that physical capability should be graded, not only boolean.
However, this does not mean `world` should make `LimbScore` or
`CapacityScore` a foundational kernel primitive.

Better interpretation:

```text
LimbScore / CapacityScore
  useful implementation technique for physical capability derivation

Actor-Owned Capability Derivation
  general system that can also cover knowledge, equipment, conditions, social
  authority, magic, and learned schemas
```

Instead of a body part only enabling or disabling an action, limbs can
contribute graded scores:

- manipulation
- locomotion
- vision
- balance
- speech
- breathing
- casting
- grip
- carrying
- protection

Wounds, encumbrance, equipment, mutations, magic, and fatigue can then affect
the same score vocabulary.

For physical interaction in `world`, this is likely better than direct one-off
checks such as:

```text
if left_arm_wounded:
  cannot_use_shield
```

Better:

```text
ShieldBlock requires:
  manipulate >= threshold
  grip >= threshold
  arm_guard_capacity >= threshold
```

But those scores should normally be derived from hard truth:

```text
body parts + wounds + equipment + conditions
  -> physical capability view
  -> actor-owned repertoire and validation modifiers
```

## Long Activities

CDDA's activity system is one of the most useful references.

Activities are long-term actions that can be interrupted and sometimes resumed.
They allow the avatar or an NPC to react to events while doing something that
takes more than one turn.

Important activity properties include:

- descriptive verb
- activity level
- interruptible
- can resume
- based on time, speed, or custom `do_turn`
- rooted
- refuel fires
- auto needs
- multi-activity
- completion EOC
- do-turn EOC

Adding a new activity requires:

- JSON properties in `player_activities.json`
- a C++ `activity_actor` subclass
- `start`, `do_turn`, and `finish`
- serialization and deserialization
- cancellation behavior if needed

This maps cleanly to `world`'s `Intent / Activity / ActionRequest` separation.

For `world`, the lesson is:

```text
Long work should be first-class state.
It must be serializable, interruptible, inspectable, and able to emit ordinary
action requests or kernel effects over time.
```

Examples:

- crafting
- construction
- ritual preparation
- reading
- training
- tracking
- treating wounds
- butchering
- excavation
- repairing
- travel

CDDA's implementation also warns against hiding too much behavior in activity
actors. If activity actors mutate state directly without structured event
contracts, replay and semantic interpretation become harder.

For `world`, an activity should probably not be a mini-engine. It should be a
stateful process that advances through typed effects and events.

## Item Use Actions

CDDA item use is a mixed model.

An item `use_action` can refer to a built-in function, or it can use a more
structured actor. The C++ item factory registers many built-in use functions,
and also registers actor classes such as transform, unpack, sound, explosion,
consume drug, heal, holster, deploy, place monster, reveal map, salvage, emit,
and others.

This is a practical compromise, but it is exactly the pressure `world` should
respond to with a typed effect system.

CDDA pattern:

```text
item data
  -> use_action string or actor definition
  -> C++ function / actor
  -> direct gameplay behavior
```

Desired `world` pattern:

```text
item affordance
  -> ActionDef / EffectDef
  -> typed kernel primitives
  -> physical events
  -> observation
  -> semantic interpretation
```

The CDDA lesson is not "avoid hardcoded actions entirely." It is:

```text
Make the hardcoded primitive vocabulary smaller than the authored action
vocabulary.
```

## Tool Qualities And Action Repertoire

CDDA uses tool qualities to connect items to possible actions.

Examples include sawing, wrenching, drilling, lock picking, chopping, hacking,
and similar practical affordances.

This is relevant to
[Actor-Owned Capability-Derived Actions](../ideas/capability-derived-actions.md).

In `world`, tool qualities should not be loose tags alone. They should be typed
actor-owned capability providers when the tool is carried, worn, installed, or
otherwise controlled by the actor:

```text
tool quality
  + actor body capacity
  + skill
  + knowledge
  -> actor owns an ApplyTool method/schema

target affordance + context
  -> validates and resolves a concrete ActionRequest
```

Example:

```text
FineWrench(level: 1)
  enables AdjustMechanism
  if actor can manipulate
  and target exposes AdjustableMechanism
```

This is more structured than a flag table, but still keeps content authorable.

## Crafting And Construction

CDDA crafting and construction are mostly data-driven, but not purely generic.

Construction definitions can include:

- construction group and category
- primary skill and difficulty
- required skills
- tool qualities
- tools and charges
- reusable requirement groups
- activity level
- special per-turn hooks
- hardcoded vehicle-start checks
- time required
- material components
- prerequisite terrain or flags
- post-construction terrain
- post-construction special hooks
- byproducts
- hidden construction entries

This is a good model for `world`'s long-process layer, but it reveals an
important limitation:

```text
Data-driven construction still needs an effect vocabulary for terrain,
furniture, entity, material, and process mutation.
```

If the effect vocabulary is not explicit, special hooks accumulate.

For `world`, crafting/building should probably be:

```text
ProcessDef:
  roles:
    actor, target place, materials, tools
  requirements:
    skill, knowledge, body capacity, tool quality, terrain affordance
  progress:
    time, labor, interruption rules, risk
  effects:
    typed effect program
  event contract:
    material consumed, entity transformed, structure created, residue emitted
```

## Terrain, Furniture, And Destruction

CDDA terrain and furniture can transform. `ter_furn_transform` can transform
terrain, furniture, fields, and traps based on direct ids or flags. It can be
called from effects on conditions, mapgen, and spells.

CDDA also has a smashing model for damaging terrain, furniture, and fields.
Damage types from the character's weapon are applied to a damage profile, and
damage accumulates until terrain or furniture is destroyed.

This supports `world`'s plan to treat terrain and structures as part of the hard
physical substrate, not only map presentation.

For `world`, the primitive should likely be more general:

```text
apply_damage(target, damage)
change_integrity(target, delta)
transform_entity_or_tile(target, result)
alter_passability(target, passable)
emit_residue_or_byproduct(...)
```

Then smashing, burning, mining, digging, spell transformation, and construction
can share mechanics.

## Effects On Conditions

CDDA's Effect On Condition system is a broad scripting mechanism. It can query
many kinds of "talkers," including avatar, NPC, monster, furniture, item, and
vehicle. It can check traits, species, body type, wielded items, worn flags,
known recipes, senses, terrain, fields, weather, overmap locations, and many
other conditions.

This is close to a semantic or rule layer, but it is not the same as the PL
direction discussed for `world`.

CDDA EOC is useful as evidence that large games need:

- a query language
- typed targets
- condition composition
- effect composition
- access to actor, item, terrain, and world state

But it also shows a risk:

```text
If the query/effect language grows as a list of special predicates, it becomes
powerful but hard to reason about as a coherent semantics.
```

For `world`, the PL layer should probably be more disciplined:

- typed context views
- stage permissions
- explicit truth layer access
- provenance
- event contracts
- separation between physical mutation and semantic interpretation

## NPCs, Factions, And Social State

CDDA is less useful for `world`'s intended social semantics.

It supports NPC classes, NPC instances, dialogue state machines, faction ids,
opinion/trust fields, faction relations, shopkeeper stock, price rules, and
faction food/wealth data.

Important details:

- NPCs can be defined in JSON
- dialogue works like a state machine
- NPCs have attitude and mission enum-like fields
- factions can track likes, respects, trust, known-by-player, currency, price
  rules, food supply, wealth, and relations
- some faction fields such as size and power are explicitly documented as
  currently having no gameplay effect
- faction trust can gate trade stock
- shop stock can be consumed and restocked over time

This is useful, but it is not the semantic layer `world` wants.

CDDA's social state is mostly:

```text
faction / NPC config
dialogue state
trade trust
shop stock
mission / attitude
some area claim behavior
```

`world` needs more:

```text
observed events
relationship meaning
ownership and claims
law and taboo
memory and rumor
witness reports
actor-specific interpretation
pressure and intent
institutional process
```

So CDDA should not be the primary reference for social semantics.

## Dimensions And Offscreen State

CDDA dimensions show a useful limitation.

The docs describe dimensions as stored in save folders and mostly disconnected
from the main dimension. They also state that unloaded dimension world data
cannot be altered directly; changes must be done when travelling into the
dimension.

For `world`, this is a warning for multi-resolution simulation.

If distant places are simply unloaded and unmodifiable, the world cannot really
progress away from the protagonist except through delayed materialization.

`world` wants something stronger:

```text
distant abstract state can progress
nearby abstract intent can progress
local concrete state can materialize
```

CDDA's dimension model is useful as a technical caution, not as the target
model.

## Representation Pressure

CDDA had to make the following concepts explicit to achieve its depth:

- item type
- item contents
- material identity
- material physical properties
- flags
- tool qualities
- charges and power
- terrain and furniture ids
- fields and field intensity
- damage types and resistances
- body parts
- sub body parts
- limb scores and encumbrance
- wounds
- activities
- activity serialization
- recipes and requirements
- construction prerequisites and results
- factions
- NPC classes and dialogue states
- effects on conditions

This supports the current `world` kernel primitive families, especially:

- material / substance / property
- body / condition / sense
- inventory / equipment / containment
- typed effect primitives
- passive physical processes
- physical events
- observation projection

It also suggests two additional design pressures. These should not necessarily
be kernel primitive families:

```text
Actor-Owned Capability Derivation
  deterministic read-only projection from actor-owned hard truth to action
  repertoire; physical capacity scores may be one implementation technique

Requirement / Affordance Validation
  typed checks used by action request validation, crafting, construction,
  magic, and long processes
```

## Lessons For Kernel Primitives

### Keep

- Treat materials as real data with physical consequences.
- Treat fields/traces/residue as part of the physical world, not only UI text.
- Treat body parts and wounds as explicit state that changes capabilities.
- Treat controlled tool qualities as actor-owned action repertoire
  contributors.
- Treat long activities as first-class, serializable, interruptible state.
- Treat construction/crafting as long processes with requirements, tools,
  materials, and results.
- Let terrain and furniture transform through rules.
- Use data definitions for content breadth, but keep engine primitives typed.

### Adapt

- Adapt CDDA's fields into a more unified substance/signal/residue model.
- Adapt body parts into a `BodyPart + Wound + PhysicalCapabilityDerivation`
  model, where capacity scores are optional derived measures.
- Adapt tool qualities into typed capability providers rather than loose tags.
- Adapt activities into `Activity` or `Process` records that advance through
  typed effects and emit events.
- Adapt construction and crafting into effect programs with event contracts.
- Adapt EOC-like conditions into a typed PL/context-query layer with stage
  permissions and provenance.
- Adapt JSON content breadth into a checked content IR instead of raw stringly
  definitions.

### Avoid

- Do not copy CDDA's post-apocalyptic survival loop as the target game shape.
- Do not let item use actions become a giant registry of hardcoded verbs.
- Do not let fields, residues, gases, and traces become unrelated special
  systems.
- Do not let JSON flags become an untyped tag soup.
- Do not allow long activities to mutate state without structured events.
- Do not rely on UI messages as the record of consequences.
- Do not use CDDA as the primary model for social, legal, or semantic meaning.
- Do not treat unloaded regions as inert if `world` wants meaningful distant
  simulation.

## Open Questions For `world`

- Should physical capability derivation use named scores, typed methods,
  boolean traits, or a mix?
- Should `Field`, `Trace`, and `Contamination` be specialized storage over a
  shared `Substance` model?
- Which material properties are needed first for combat, stealth, crafting,
  fire, poison, and magic?
- Should terrain and furniture be ordinary entities, tile state, or a hybrid?
- How should long activities emit action requests versus direct effect
  programs?
- What is the smallest typed effect vocabulary that can express CDDA-like item
  use, terrain transformation, and construction without hardcoded use verbs?
- How should construction/crafting requirements be typed so they can distinguish
  actor-owned capacity, skills, tool qualities, and knowledge from target/place
  affordances?
- How much C++/Rust kernel code should be required to add a new behavior family?
- Can the semantic PL use EOC-like condition composition without becoming a
  large predicate catalog?
- Which passive processes belong in the first kernel: fire, smoke, scent,
  wetness, bleeding, poison, temperature, decay?

## Extracted Design Notes

### Kernel Primitive Revision

CDDA does not invalidate the current primitive list. It strengthens it.

The current list:

```text
1. Identity / Time / RNG / Replay
2. Location / Topology / Containment
3. Entity / Physical Object
4. Material / Substance / Property
5. Body / Condition / Sense
6. Inventory / Equipment / Container
7. Typed Effect Primitives
8. Passive Physical Processes
9. Physical Events
10. Observation Projection
```

Suggested refinement:

```text
Body / Condition / Sense
  should preserve hard body, wound, condition, and sense facts

Actor-Owned Capability Derivation
  should derive action repertoire from body, equipment, skill, knowledge,
  conditions, social authority, magic, and learned schemas

Physical capacity scores
  may be useful derived measures, but should not be treated as foundational
  hard truth

Material / Substance / Property
  should include substance, residue, field, and contamination as one family

Typed Effect Primitives
  should be tested against item use, construction, field transformation,
  wound application, and terrain destruction

Passive Physical Processes
  should include at least fire, field decay/spread, scent/residue decay,
  bleeding, poison metabolism, wetness drying, and decay

Long Process / Activity
  may deserve explicit treatment near kernel/effect boundary, even if it is not
  itself a hard mutation primitive
```

### Main Architectural Lesson

CDDA's greatest lesson for `world` is negative:

```text
If action behavior is represented as a mixture of data fields, built-in
function names, actor subclasses, flags, and special hooks, the game can grow
very deep, but the causal model becomes hard to expose as a clean semantic
pipeline.
```

`world` should preserve the depth but make the causal path more explicit:

```text
ActionDef / ProcessDef
  -> typed effects
  -> kernel primitives
  -> physical events
  -> observation
  -> semantic interpretation
  -> memory / pressure / intent
```

## Sources

- [CDDA official docs](https://docs.cataclysmdda.org/)
- [CDDA GitHub repository](https://github.com/CleverRaven/Cataclysm-DDA)
- [JSON INFO](https://docs.cataclysmdda.org/JSON/JSON_INFO.html)
- [ITEM JSON docs](https://docs.cataclysmdda.org/JSON/ITEM.html)
- [PLAYER_ACTIVITY docs](https://docs.cataclysmdda.org/PLAYER_ACTIVITY.html)
- [WOUNDS docs](https://docs.cataclysmdda.org/JSON/WOUNDS.html)
- [Terrain and Furniture Transforms](https://docs.cataclysmdda.org/JSON/TER_FURN_TRANSFORM.html)
- [Smashing docs](https://docs.cataclysmdda.org/JSON/MAP_SMASHING.html)
- [Effect On Condition docs](https://docs.cataclysmdda.org/JSON/EFFECT_ON_CONDITION.html)
- [NPC docs](https://docs.cataclysmdda.org/JSON/NPCs.html)
- [NPC faction docs](https://docs.cataclysmdda.org/JSON/FACTIONS.html)
- [JSON Loading Order](https://docs.cataclysmdda.org/JSON/JSON_LOADING_ORDER.html)
- [src/material.h](https://raw.githubusercontent.com/CleverRaven/Cataclysm-DDA/master/src/material.h)
- [src/field_type.h](https://raw.githubusercontent.com/CleverRaven/Cataclysm-DDA/master/src/field_type.h)
- [src/bodypart.h](https://raw.githubusercontent.com/CleverRaven/Cataclysm-DDA/master/src/bodypart.h)
- [src/activity_actor.h](https://raw.githubusercontent.com/CleverRaven/Cataclysm-DDA/master/src/activity_actor.h)
- [src/activity_actor_definitions.h](https://raw.githubusercontent.com/CleverRaven/Cataclysm-DDA/master/src/activity_actor_definitions.h)
- [src/iuse_actor.h](https://raw.githubusercontent.com/CleverRaven/Cataclysm-DDA/master/src/iuse_actor.h)
- [src/item_factory.cpp](https://raw.githubusercontent.com/CleverRaven/Cataclysm-DDA/master/src/item_factory.cpp)
