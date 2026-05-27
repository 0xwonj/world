# Engine Core And Game System Boundary

## Status

Current design principle.

This document defines how the reusable simulation foundation stays separate
from game-system packs and specific game content. It is not an implementation
plugin API, package format, or content roadmap.

## Source Context

- [Simulation Core](simulation-core.md)
- [Pack Authoring And Semantic Declarations](pack-authoring-and-semantic-declarations.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
- [Typed Effect Primitives](typed-effect-primitives.md)
- [Standard World Library And Primitive Semantics](standard-world-library.md)
- [Causal Runtime](causal-runtime.md)
- [Physical Simulation Grammar](physical-simulation-grammar.md)
- [Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md)
- [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md)
- [Intent Templates And Planning](intent-templates-and-planning.md)
- [Engine Architecture Research Entry](../research/engine-architecture-entry.md)

## Purpose

The engine should support more than one specific RPG without becoming an
all-purpose game engine.

The target is:

```text
a reusable simulation foundation for deep, actor-relative,
causally inspectable RPG and sandbox worlds.
```

This boundary exists to prevent two opposite failures:

- hardcoding one game's stats, combat, magic, crafting, economy, or social
  rules into the core
- making the core so generic that game systems become untyped scripts, tag
  soup, or unchecked plugin mutations

## Core Rule

```text
Core owns mechanism.
Packs own vocabulary.
Game owns content, balance, and premise.
```

Mechanism means how state is represented, checked, mutated, recorded,
scheduled, observed, and explained.

Vocabulary means a coherent set of named game concepts, rule families,
schemas, formulas, and authored behaviors that use the mechanism.

Content means this game's concrete setting, values, objects, people, places,
encounters, progression, and presentation.

## Layer Split

### 1. Reusable Simulation Core

The reusable simulation core owns the mandatory authority and execution
machinery.

It includes:

- truth authority and commit boundaries
- world store families, query surfaces, indexes, and derived-view plumbing
- `ActionRequest`, `Typed Effect Program`, `CausalTransaction`, and
  `EventRecord`
- scheduler, turn order, duration, wakeups, and time provenance
- `ProcessInstance`, reservation, reaction, interruption, replay, and
  provenance mechanics
- actor-facing access boundaries such as non-omniscient observations,
  available repertoire, invalid-action feedback, and AI-agent input/output
  shape

Core should not contain named RPG features such as `Strength`, `Fireball`,
`Pickpocket`, `Blacksmithing`, `HonorDuel`, or `IronSwordRecipe`.

### 2. Reusable World-Simulation Library

The reusable world-simulation library provides common simulation grammar for
deep RPG worlds.

It includes:

- topology, containment, equipment, attachment, and embedded-object relations
- material, substance, trace, residue, signal, field, condition, body, wound,
  and passive physical process categories
- perception channels and observation projection rules
- capability, affordance, and actor-interface derivation shapes
- epistemic state mechanics for memory, belief, knowledge, rumor, secret, and
  learned procedure records
- social and institutional substrate shapes such as relationship, membership,
  rank, `SocialClaim`, norm, law, taboo, permission, debt, oath, reputation,
  and jurisdiction
- multi-resolution execution rules, promotion, demotion, route progress, and
  abstract process constraints

This layer is not the causal runtime core. The runtime owns transaction
authority, staging, commit, replay, and process-control gates. The standard
world library owns reusable primitive definitions and trusted primitive
semantics installed into the runtime.

The library may define reusable categories, but concrete taxonomies remain
pack-owned when different games reasonably need different vocabularies.

### 3. Game System Packs

Game system packs define coherent gameplay vocabularies on top of the reusable
mechanisms.

Examples:

- stat and derived-capability vocabularies
- skill, technique, spell, ritual, and procedure vocabularies
- combat, stealth, magic, crafting, building, economy, trade, travel, disease,
  weather, faction, religion, law, and reputation rule packs
- `ActionDef`, `ActionSchema`, `ProcessDef`, `AppraisalRule`,
  `IntentTemplate`, `SocialRule`, `ContentSchema`, `DerivedView`, and checked
  `Typed Effect Program` definitions
- material, damage, wound, condition, magic, recipe, and item taxonomies where
  they are genre- or world-specific

Packs may extend what actors can try, what processes exist, how events are
interpreted, and what content schemas are valid. They do not gain authority to
mutate truth by bypassing the core runtime.

### 4. Specific Game / Content

The specific game owns the concrete premise and authored experience.

Examples:

- setting, cosmology, history, cultures, factions, races, monsters, regions,
  dungeons, settlements, and scenario premises
- concrete stat values, growth curves, progression pacing, economy numbers,
  encounter tuning, reward pacing, and balance
- item lists, spell lists, recipes, monster definitions, profession lists,
  authored secrets, quests, rumors, and narrative presentation
- UI, camera, input, wording, visual design, audio direction, and player-facing
  framing

Specific game content may be checked by the same tooling as packs, but it is
not what makes the simulation core reusable.

## Boundary Test

Use these questions when deciding where a concept belongs:

```text
Does this define how authoritative state may change, be recorded, or be
replayed?
  -> Core.

Does this provide a reusable RPG-world substrate that many packs can share,
while leaving named taxonomy choices open?
  -> World-simulation library.

Does this define named mechanics, formulas, rule vocabularies, schemas, or
semantic categories for a family of gameplay?
  -> Game system pack.

Does this define the concrete setting, values, content, balance, or
presentation of one game?
  -> Specific game / content.
```

When in doubt, keep authority in the core and move vocabulary outward.

## Extension Discipline

Packs extend the engine through checked declarations, not arbitrary hard-state
mutation.

Typical pack declarations:

```text
ActionSchema
ActionDef
Typed Effect Program
ProcessDef
AppraisalRule
IntentTemplate
SocialRule
ContentSchema
DerivedView
```

The allowed path is:

```text
Pack declaration
  -> schema / type / effect / authority checking
  -> registered action, process, rule, content, or derived-view family
  -> actor-relative binding or scheduled process tick
  -> Typed Effect Program or accepted non-hard commit proposal
  -> CausalTransaction / accepted soft-truth gate
  -> EventRecord or typed non-hard record
```

The forbidden path is:

```text
Pack callback
  -> direct WorldStore mutation
  -> direct EventRecord rewrite
  -> direct EpistemicStore / SocialInstitutionalStore / AppraisalRecordStore write
  -> hidden intent or pressure mutation
```

Packs can define new game systems, but every accepted outcome must still obey
the truth, mutation, provenance, and replay boundaries of the engine.

## PL And Tooling Role

The PL/tooling layer should express, check, migrate, inspect, and explain pack
declarations. It is not the owner of truth.

[Pack Authoring And Semantic Declarations](pack-authoring-and-semantic-declarations.md)
owns the pack declaration authoring model, semantic declaration framework, and
verification boundary. It keeps `Typed Effect Program` separate from semantic
declarations while allowing social, appraisal, intent, and semantic-view rules
to share one checked declaration substrate.

[Simulation Transition Compiler](simulation-transition-compiler.md) defines the
compiler-shaped model behind this role: pack declarations are checked ahead of
time, runtime situations are projected, analyzed, selected, and lowered where
needed, representation/pass classes provide design taxonomy rather than a
generic runtime abstraction, and hard effects are interpreted transactionally.

Good PL uses:

- checking that an authored effect program only performs permitted effects
- checking that primitive invocations match installed standard or trusted
  extension definitions
- validating content schemas and typed references
- checking that required `EventRecord` contracts are emitted
- typing query inputs for semantic, social, and intent rules
- requiring provenance for accepted AI, worldgen, epistemic, social, and
  appraisal records
- explaining why an action was available, unavailable, accepted, failed, or
  interrupted
- explaining why an event was interpreted as pressure, duty, crime, taboo, or
  opportunity
- supporting schema migration and replay/version inspection

Bad PL uses:

- replacing engine-owned primitive semantics with arbitrary script mutation
- treating ordinary pack source as trusted primitive semantics
- treating natural language as authoritative state when gameplay depends on it
- letting content packs mutate hard truth without `CausalTransaction`
- letting semantic rules write memory, social truth, or intent directly

## Examples

### Stats

Core should not know `Strength`, `Agility`, `Endurance`, `Will`, or
`ArcaneFocus`.

Core/library should know that actor-owned state, conditions, equipment,
knowledge, social authority, and derived views can affect `CapabilitySet`,
`ActionRepertoire`, checks, cost, duration, and risk.

A stat pack may define:

```text
Strength
Agility
Endurance
Will
ArcaneFocus

carry_capacity = f(Strength, body_size, wounds)
lockpick_cost = f(Agility, hand_condition, tool_quality)
spell_failure = f(ArcaneFocus, fatigue, pain)
```

The specific game defines starting values, growth curves, species modifiers,
balance, and progression pacing.

### Magic

Core should not know `Fireball`.

Core/library should know how pack-declared resources, fields, signals,
material transformation, body damage, conditions, range binding, area binding,
process timing, and `EventRecord` contracts are checked and committed.

A magic pack may define:

```text
ActionSchema: CastSpell(spell, target, mode)
Resource: mana
School: fire
Spell: Fireball

Effects:
  consume_resource(mana)
  emit_signal(light, source, intensity)
  create_field(heat, area, intensity)
  apply_damage(target, fire_damage)
```

The specific game decides what spells exist, how they are learned, how they
scale, and what they mean in the setting.

### Physics

Physics is split.

Reusable library categories:

```text
material
substance
containment
equipment
body
wound
condition
field
signal
trace
residue
passive process
```

Pack-owned vocabulary:

```text
damage types
material taxonomy
magical substances
poison and disease rules
fire, smoke, scent, or sound propagation detail
structural collapse policy
```

This keeps physical causality shared without forcing every game to use the
same materials, damage model, or magic metaphysics.

### Crafting

Core should not have a privileged `crafting` subsystem.

Core/library should provide:

```text
ProcessInstance
requirements and checks
input consumption
material transformation
entity creation
condition modification
time cost
interruption and resume
EventRecord emission
```

A crafting pack may define:

```text
Recipe
CraftItem
RepairItem
BuildStructure
Disassemble
tool quality
workstation requirements
material substitution rules
```

The specific game defines recipes, materials, professions, item balance, and
progression.

### Combat

Core should not hardcode a complete combat system.

Core/library should provide time, position, topology, body, wound, condition,
equipment, checks, contests, typed effects, and `EventRecord`s.

A combat pack may define:

```text
Attack
Block
Dodge
Grapple
Aim
Bleed
ArmorPenetration
MoralePressure
```

The specific game defines weapon balance, monster attacks, styles, skills, and
encounter pacing.

### Shrine Item Removal

Core records physical transfer:

```text
EntityTransferred(shrine_relic, shrine_floor, actor_inventory)
```

The social pack provides:

```text
SocialClaim(shrine owns shrine_relic)
Norm(shrine forbids non-priest removal)
```

Semantic appraisal interprets the observed transfer in context:

```text
possible theft
possible taboo violation
guard duty pressure
```

The specific game decides what the shrine is, why the relic matters, who cares,
and how consequences are balanced.

## Anti-Goals

- a universal engine for every genre
- one-game RPG systems hardcoded into core
- plugin callbacks that bypass truth authority
- stringly typed predicates as the main extension model
- semantic rules that directly mutate hard truth, memory, social truth, or
  final intent
- content definitions that only produce prose when gameplay depends on
  structured state

## Stable Decisions

- The project should build a reusable simulation foundation for deep,
  actor-relative, causally inspectable RPG and sandbox worlds.
- Core owns mechanism; packs own vocabulary; specific games own content,
  balance, and premise.
- The reusable world-simulation library is a real layer between runtime core
  and game-system packs. It supplies standard primitive definitions and trusted
  semantics without owning causal commit authority.
- Named RPG systems such as stats, combat, magic, crafting, building, economy,
  social vocabularies, appraisal rules, and intent templates should be authored
  as pack-level definitions unless their mechanism belongs in core.
- Packs must use checked declarations and engine commit gates. They do not
  bypass `CausalTransaction`, `EventRecord`, or accepted non-hard commit
  surfaces.
- PL/tooling should check and explain pack declarations; it should not become
  the owner of truth.

## Deferred Decisions

- pack manifest and dependency model
- pack ordering, override, and conflict-resolution policy
- versioning and migration policy for pack declarations
- exact first bundled standard primitive set and exact taxonomy boundaries
- how much modding/user-authored content should be supported by the same
  discipline
