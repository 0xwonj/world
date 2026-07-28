# Typed Effect Primitives

## Status

Partially superseded vocabulary input.

The primitive examples remain useful. The `CausalTransaction`, universal
effect-program, and runtime-dispatch shapes below are not current contracts.
Owner-specific preparation, extension tiers, and family-specific IR are
defined by
[Extensibility And Research](../architecture/target-architecture/extensibility-and-research.md)
and the normative runtime model.

## Source Ideas

- [Typed Action Effects](../ideas/typed-action-effects.md)
- [Kernel Primitives](../ideas/kernel-primitives.md)
- [Causal Runtime / Action-Effect-Event](../research/causal-runtime-action-effect-event.md)

## Related Design Owners

- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Standard World Library And Primitive Semantics](standard-world-library.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)

## Purpose

This document defines the checked primitive effect vocabulary used by action
definitions and processes to mutate hard truth.

Actions should not each be a hidden bespoke resolver. They should lower into a
typed effect program over engine-owned primitives.

```text
ActionDef = roles + requirements + checks + Typed Effect Program
```

Typed effect primitives are the checked hard-mutation call surface. The
runtime owns staging and commit authority. The standard world library owns
common reusable primitive definitions and trusted semantics. Game-system packs
may define new `ActionDef`, `ActionSchema`, process, spell, combat, crafting,
or magic vocabularies, but their hard outcomes still lower into checked
primitive effects and runtime-owned commit paths.

In the [Simulation Transition Compiler](simulation-transition-compiler.md)
model, a `Typed Effect Program` is the low-level mutation IR that is verified
before it can be interpreted by the causal runtime.

[Pack Authoring And Semantic Declarations](pack-authoring-and-semantic-declarations.md)
treats it as a separate hard-mutation IR family, not part of the semantic
declaration IR framework used for social, appraisal, and intent declarations.

## Boundary

Typed effect primitives own:

- checked hard-state mutation vocabulary
- effect input and permission contracts
- mandatory `EventRecord` contract expectations
- transaction staging expectations
- forbidden semantic mutation boundaries

Typed effect primitives do not own:

- actor-owned action repertoire
- actor intent selection
- process scheduling policy
- named game-system vocabularies such as stats, spells, combat moves, recipes,
  or economy rules
- social meaning
- memory, belief, rumor, or secret creation
- UI text

Typed effect primitives are not one monolithic implementation location. The
definition model, runtime registry, and standard semantics have separate
owners:

```text
EffectPrimitiveDef:
  checked primitive signature and contract

PrimitiveSemanticsRegistry:
  runtime-owned lookup and capability-gated dispatch

standard world library:
  reusable primitive definitions and trusted semantics for common RPG-world
  mechanics
```

Ordinary pack-authored effect programs call installed primitives. They do not
receive raw staging contexts or direct store mutation authority.

## Core Flow

```text
ActionRequest
  -> bind roles and parameters
  -> ActionDef
  -> requirements and checks
  -> Typed Effect Program
  -> CausalTransaction staging
  -> commit
  -> EventRecord append
```

The causal runtime owns execution, staging, commit, failure semantics, replay,
reservation, and reaction. This document owns the primitive effect contract
shape that the runtime executes.

## Effect Families

### Query Effects

Read-only effects inside hard typed effect programs expose declared hard-truth
and allowed derived-engine reads.

Examples:

```text
location(entity)
material(target)
contained_in(entity)
reachable(actor, target)
body_part(actor, part)
equipment(actor, slot)
active_condition(target, condition)
field_at(place, kind)
```

Query effects must declare whether they read hard truth or a derived engine
view rebuilt from hard truth. Hard mutation programs must not read actor belief,
semantic context, or debug-only state as validation truth.

Actor-relative `PerceivedAffordance` may inform request construction and
binding provenance, but the typed effect program must revalidate against hard
truth before committing physical mutation.

### Check Effects

Check effects validate uncertainty, skill, risk, contest, and random outcomes.

Examples:

```text
check_reachable(actor, target)
check_can_manipulate(actor, required_precision)
check_skill(actor, skill, difficulty)
check_attack(actor, target, instrument)
check_resistance(target, damage_or_force)
check_random(rng_stream, table)
```

Random checks must use simulation RNG streams, not client randomness.

### Mutation Effects

Initial primitive mutation vocabulary:

```text
move_entity(entity, destination)
transfer_entity(entity, from, to)
attach_entity(entity, target)
detach_entity(entity, target)
embed_entity(entity, target)
attach_substance(substance, target_surface)
remove_substance(substance, target_surface)
change_temperature(target, delta)
ignite(target)
extinguish(target)
apply_force(target, force)
apply_damage(target, damage)
change_integrity(target, delta)
set_open_state(target, open_state)
set_lock_state(lock_or_lockable, lock_state)
set_mechanism_state(mechanism, mechanism_state)
alter_passability(target, passable)
add_condition(target, condition)
remove_condition(target, condition)
emit_signal(kind, source, intensity)
create_entity(kind, location)
destroy_entity(entity)
schedule_process(process, wakeup)
cancel_process(process, reason)
```

The vocabulary should stay small and typed. Avoid generic mutation effects such
as:

```text
SetField(entity, "locked", false)
AddTag(actor, "angry")
ModifyStat(actor, "mood", -10)
```

Use domain-specific hard effects and semantic-stage effects instead.

`add_condition` and `remove_condition` are not generic tag escape hatches. They
operate on typed physical condition taxonomies such as wound, burning,
poisoned, stunned, soaked, chilled, or blinded, with source/provenance,
duration, and stacking rules where needed.

### Event Record Effects

Every hard mutation that matters for replay, observation, memory, or semantic
interpretation must emit a typed `EventRecord`.

Examples:

```text
transfer_entity -> EntityTransferred / ItemTaken
move_entity -> EntityMoved / ActorMoved
attach_entity -> EntityAttached
detach_entity -> EntityDetached
embed_entity -> EntityEmbedded
attach_substance -> SubstanceAttached / ResidueAttached
remove_substance -> SubstanceRemoved / ResidueRemoved
change_temperature -> TemperatureChanged
apply_damage -> DamageApplied / BodyPartWounded / ActorDied
apply_force -> ForceApplied / ObjectDisplaced
change_integrity -> IntegrityChanged / ObjectBroken
ignite -> FireStarted / SmokeEmitted / HeatEmitted
extinguish -> FireExtinguished / HeatReduced
set_open_state -> OpenStateChanged / DoorOpened / DoorClosed
set_lock_state -> LockStateChanged / LockUnlocked / LockLocked
set_mechanism_state -> MechanismStateChanged / MechanismJammed
alter_passability -> PassabilityChanged
add_condition -> ConditionAdded
remove_condition -> ConditionRemoved
emit_signal -> SoundEmitted / LightEmitted / ScentEmitted
create_entity -> EntityCreated
destroy_entity -> EntityDestroyed
schedule_process -> ProcessScheduled
cancel_process -> ProcessCancelled
```

`EventRecord`s are structured hard facts, not UI text.

## Primitive Contract Shape

Each primitive effect should define:

- name
- typed inputs
- required hard-truth reads
- allowed derived reads
- validation responsibilities
- staged writes
- mandatory `EventRecord` family
- rollback behavior inside `CausalTransaction`
- RNG provenance if random
- process or reservation interactions if any

Example:

```text
ignite(target)
  inputs:
    target: EntityId
  reads:
    material(target)
    active_conditions(target)
    local_field(target.location)
  validates:
    target can burn or can host a fire process
  writes:
    add burning condition or schedule fire process
  event_records:
    FireStarted
    SmokeEmitted when smoke is produced
    HeatEmitted when heat is modeled
```

## Stage Permissions

Allowed in typed effect programs:

- physical requirements and checks
- physical state mutation through primitive effects
- process and reservation interaction through runtime primitives
- time and resource cost staging
- physical and sensory `EventRecord` emission

Forbidden in typed effect programs:

- create `EpistemicRecord`
- create grief, revenge, shame, guilt, duty, or fear pressure
- declare theft, trespass, taboo, crime, justice, or holiness
- mutate relationship because of semantic interpretation
- directly set intent
- emit UI text as the source of truth

Semantic meaning is derived later from `EventRecord`s, observations,
epistemic state, and social context.

## Binding And Validation

Action definitions do not create the actor's action repertoire. The actor owns
schemas through `CapabilitySet` and `ActionRepertoire`.

Typed effect definitions answer:

```text
Given this ActionRequest, bound roles, hard truth, and perceived target
affordances, what checked effects and `EventRecord`s define the attempt?
```

Validation should distinguish:

- schema not owned by actor
- target not perceived or not bindable
- perceived affordance was misleading
- hard truth blocks the action
- random or contested check failed
- process or reservation conflict blocks the action

## Scenario: Torch And Wooden Door

```text
Actor interface:
  ActionRepertoire includes ApplyTool(tool, target, mode)
  actor carries lit_torch
  door is observed as wooden and burnable

ActionRequest:
  ApplyTool(lit_torch, door_1, ignite)

Typed Effect Program:
  check_reachable(actor, door_1)
  check tool emits heat or flame
  check material(door_1).flammability
  ignite(door_1)
  add_condition(door_1, burning)
  schedule_process(fire_spread_or_burn_down, wakeup)
  emit_signal(smoke, door_1, medium)

EventRecord set:
  ToolApplied
  FireStarted
  ConditionAdded(burning)
  ProcessScheduled when fire progression is modeled
  SmokeEmitted
  HeatEmitted when heat is modeled
```

The door did not grant a special `BurnDoor` action. It provided an affordance
that allowed a binding of an actor-owned schema.

## Scenario: Wounded Hand And Lockpick

```text
Hard truth:
  right_hand has deep_cut
  actor holds lockpick
  gate has lock

CapabilitySet:
  fine_manipulation degraded

ActionRequest:
  ApplyTool(lockpick, gate, pick_lock)

Binding:
  perceived gate affordance suggests lock
  hard validation resolves gate_lock from gate mechanism
  check lock is reachable and exposed enough to manipulate
  check tool is compatible with lock mechanism

Typed Effect Program:
  check_can_manipulate(actor, fine)
  check lock state is locked
  check mechanism is not sealed or jammed beyond this method
  check_skill(actor, lockpicking, increased_difficulty)
  on success:
    set_lock_state(gate_lock, unlocked)
    emit_signal(sound, gate_lock, quiet)
  on failure:
    optionally apply_damage(lockpick, minor_bend)
    emit_signal(sound, gate_lock, quiet)

EventRecord set:
  LockpickAttempted
  LockpickSucceeded or LockpickFailed
  LockStateChanged / LockUnlocked on success
  ToolDamaged if the lockpick bends
  SoundEmitted when modeled
```

The wound changes capability, cost, risk, or validation. It does not require a
story-specific lockpick resolver.

## Relationship To Other Documents

- [Physical Simulation Grammar](physical-simulation-grammar.md) defines the
  hard substrate the effects mutate.
- [Standard World Library And Primitive Semantics](standard-world-library.md)
  defines how reusable primitive definitions and trusted runtime semantics are
  supplied outside the runtime core.
- [Causal Runtime](causal-runtime.md) executes and commits typed effect
  programs.
- [Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md)
  defines actor-owned repertoire and perceived target affordances.
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
  defines forbidden semantic mutations.

## Reusable vocabulary conclusions

- `ActionRequest` remains the attempted-change interface.
- Actions lower into `Typed Effect Program`s.
- Primitive mutation semantics are trusted engine/library semantics, not
  ordinary pack callbacks.
- Every important hard mutation must have a structured `EventRecord` contract.
- Semantic effects are forbidden in hard typed effect programs.
- Pack-authored actions and processes must use checked typed effects rather
  than direct store mutation.

## Deferred Decisions

- exact first primitive set
- exact type system for effect inputs
- effect checker design
- exact primitive semantics registry API
- `EventRecord` schema versioning
- how many derived views are allowed in hard validation
- whether process effects are a separate sublanguage
