# Standard World Library And Primitive Semantics

## Status

Current design principle.

This document defines where reusable world-simulation vocabulary and primitive
effect semantics live. It is not a pack manifest format, parser design, modding
API, or full gameplay standard library.

## Source Context

- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Physical Simulation Grammar](physical-simulation-grammar.md)
- [Typed Effect Primitives](typed-effect-primitives.md)
- [Causal Runtime](causal-runtime.md)
- [Pack Authoring And Semantic Declarations](pack-authoring-and-semantic-declarations.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)

## Purpose

The engine needs more than one layer between a tiny runtime kernel and
game-specific packs.

If every primitive is pushed into game packs, authored actions become awkward
sequences of overly generic mutations. If every common primitive is placed
inside `world-runtime`, the runtime quietly becomes a hardcoded RPG system.

The target split is:

```text
runtime core:
  owns transaction authority, staging, validation, commit, replay, and process
  control

standard world library:
  owns reusable RPG-world grammar, standard primitive definitions, and trusted
  primitive semantics

game-system packs:
  own named mechanics, formulas, taxonomies, actions, processes, and semantic
  rules that compose checked primitives

specific game content:
  owns concrete setting, values, entities, spells, items, balance, and
  presentation
```

This keeps effects meaningful enough to author against while preserving the
rule that packs do not gain raw mutation authority.

## Boundary

The standard world library owns:

- reusable physical/topological effect primitives such as transfer, movement,
  attachment, passability, lock/mechanism state, conditions, fields, signals,
  substances, damage, wounds, and passive physical process hooks
- reusable primitive signatures and type contracts
- reusable event contract families for those primitives
- trusted runtime semantics for standard primitives
- reusable vocabulary needed by actor context projection, capability,
  affordance, and perception when that vocabulary is not game-specific

The standard world library does not own:

- `CausalTransaction` authority
- accepted commit construction or publication
- process scheduler authority
- final actor intent selection
- semantic appraisal or social meaning
- pack source syntax
- concrete game taxonomies such as the final material list, damage type list,
  spell list, combat style list, skill list, recipe list, or faction law set
- arbitrary user/plugin code execution

## Runtime Core Versus Standard Primitives

The runtime core should stay small.

Runtime core owns mechanism:

```text
ActionRequest binding
Typed Effect Program interpretation
CausalTransaction staging
stage permission enforcement
runtime-control update gates
invariant checks
atomic commit
EventRecord append
replay and audit provenance
```

Standard primitives own reusable world meaning:

```text
transfer_entity
move_entity
attach_entity
apply_damage
change_integrity
set_open_state
set_lock_state
add_condition
remove_condition
emit_signal
create_field
schedule_process
cancel_process
```

The runtime may need a tiny seed dispatcher while the causal waist is being
implemented. That seed should not become the permanent home for physical,
damage, condition, signal, or process vocabulary.

## Definition And Semantics Split

A primitive has two halves.

### Primitive Definition

The definition half is checked data consumed by authoring, validation,
inspection, and runtime lookup.

Conceptual shape:

```text
EffectPrimitiveDef {
  id
  name
  params
  required_permissions
  hard_reads
  derived_reads
  writes
  event_contract
  replay_requirements
  version
}
```

This belongs in the runtime-facing definition model. It must be visible without
depending on a parser or on runtime mutation internals.

### Primitive Semantics

The semantics half is trusted executable engine code that validates and stages
one primitive through runtime capabilities.

Conceptual shape:

```text
PrimitiveSemantics {
  primitive
  validate(invocation, validation_context)
  stage(invocation, stage_context)
}
```

Semantics receive staging capabilities, not raw store mutators. They can ask
runtime-controlled questions, stage hard changes, stage event candidates, and
stage runtime-control updates where the primitive permits it.

## Crate Shape

The target crate shape should preserve dependency direction.

```text
world-standard
  reusable standard-world definitions, event families, value categories, and
  helper builders
  depends on: world-core, world-defs
  does not depend on: world-runtime

world-standard-runtime
  trusted semantics installers for standard primitives
  depends on: world-core, world-defs, world-model, world-runtime,
              world-standard
  does not own: pack parsing, engine facade, actor decision, or host IO

world-runtime
  owns PrimitiveSemanticsRegistry, transaction staging APIs, runtime-control
  gates, and causal commit
  does not depend on: world-standard or world-standard-runtime

world-engine
  wires selected definition bundles and semantics installers into a session
```

Pure actor context code may consume `world-standard` vocabulary when it needs
standard physical categories. It must not depend on `world-standard-runtime`,
because that would pull runtime mutation authority into actor-relative
projection.

Typical bootstrap:

```text
definitions = DefinitionRegistryBuilder::new()
semantics = PrimitiveSemanticsRegistry::new()

world_standard::install_definitions(definitions)
world_standard_runtime::install_semantics(semantics)

runtime = CausalRuntime::new(definitions.build(), semantics)
```

The exact Rust API names are implementation details. The architecture point is
that runtime owns the registry and staging authority, while the standard
library supplies reusable definitions and trusted handlers from outside the
runtime core crate.

Model-facing categories should enter through checked definition contracts and
runtime commit packages. `world-model` must not depend on the standard library
to know which game or standard vocabulary is installed.

## Pack Extension Model

Most game-system packs should not define new primitive semantics. They should
compose existing primitives through checked `Typed Effect Program`s.

Example:

```text
Fireball action:
  consume_resource(caster, mana, cost)
  emit_signal(light, source, intensity)
  create_field(heat, area, intensity)
  apply_damage(target, fire_damage)
  add_condition(target, burning)
```

The magic pack may own:

```text
Fireball
mana
fire school
fire_damage formula
burning taxonomy entry
learning requirements
spell failure policy
```

The standard world library owns the reusable execution semantics for resource
consumption, signal emission, field creation, damage application, and condition
application when those primitives are installed.

Pack-defined higher-level templates may exist, but they should lower before
runtime execution:

```text
pack effect template:
  fire_burst(caster, area, intensity)

lowered Typed Effect Program:
  emit_signal(...)
  create_field(...)
  apply_damage(...)
  add_condition(...)
```

The causal runtime sees checked primitive invocations, not arbitrary pack
callbacks with mutation authority.

## Trusted Primitive Extensions

Some future extension packages may need new primitive semantics, not only new
templates. That is a trusted engine extension boundary, not ordinary pack
authoring.

Rules:

- primitive semantics are installed by trusted host/engine code
- installed semantics must match checked `EffectPrimitiveDef` signatures
- untrusted pack source cannot receive `StageContext` or raw store access
- semantics are versioned and auditable
- missing semantics cause runtime rejection or load failure, not fallback to
  generic field mutation

This leaves room for advanced game systems without turning the first pack
language into a general scripting authority.

## Relationship To Typed Effect Programs

`Typed Effect Program` is the checked call graph over primitive definitions.

```text
EffectProgramDef
  -> EffectOp(primitive id, typed args, declared events)
  -> PrimitiveSemanticsRegistry lookup
  -> validation / staging through runtime capabilities
  -> CausalTransaction
```

Effect programs should be typed at the primitive boundary. A program should not
encode important gameplay semantics only as strings such as:

```text
SetField(entity, "hp", hp - 5)
AddTag(entity, "burning")
ModifyStat(actor, "mood", -10)
```

The useful authoring level is semantic enough to inspect and verify:

```text
apply_damage(target, fire_damage)
add_condition(target, burning)
emit_signal(sound, source, intensity)
set_lock_state(lock, unlocked)
```

## Relationship To Hard State Changes

`HardStateChange` is the model-facing write package used by accepted commits.
It is not the authoring vocabulary.

Standard primitive semantics may lower one meaningful primitive into one or
more model-shaped hard changes and event candidates.

Examples:

```text
apply_damage(target, damage)
  -> add wound
  -> change integrity
  -> add condition
  -> emit DamageApplied / BodyPartWounded

transfer_entity(item, destination)
  -> update containment relation
  -> emit EntityTransferred

ignite(target)
  -> add burning condition
  -> schedule fire process
  -> emit FireStarted / SmokeEmitted / HeatEmitted
```

This keeps pack/action authoring meaningful while keeping the model receiver
small and authority-shaped.

## Relationship To Actor Context

Actor context projection needs standard vocabulary for capabilities and
affordances.

Examples:

- a wounded hand degrades fine manipulation
- smoke changes sight and scent projection
- a locked door exposes different affordances to an actor with lockpicking
  procedure knowledge
- containment and equipment determine reachable items

Actor context may depend on pure standard vocabulary and definitions. It must
not depend on runtime semantics installers or gain mutation authority.

## Stable Decisions

- The runtime core should not become the home for the growing physical,
  damage, condition, signal, resource, and process primitive vocabulary.
- The standard world library is the reusable layer between runtime mechanism
  and game-system packs.
- Primitive definitions and primitive runtime semantics are distinct.
- The runtime owns semantics registry lookup and staging authority; installed
  semantics own the reusable behavior of their primitive.
- Ordinary game-system packs compose primitives; they do not receive raw
  mutation callbacks.
- Future primitive extension packages are trusted engine extensions, not
  unrestricted pack scripts.
- Actor context may consume pure standard vocabulary, but not runtime semantics
  installers.

## Deferred Decisions

- exact first standard primitive set
- exact crate names if implementation discovers a better naming convention
- exact `EffectPrimitiveDef` field names
- exact argument expression model for `EffectOp`
- exact event family schemas
- exact damage, wound, condition, material, resource, and field taxonomies
- trusted extension package signing/loading policy
- whether any primitive semantics can later be supplied by Wasm or another
  sandboxed runtime
