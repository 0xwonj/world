# Physical Simulation Grammar

## Status

Current design draft.

## Source Ideas

- [Kernel Primitives](../ideas/kernel-primitives.md)
- [World Representation / Query Model](../research/world-representation-query-model.md)
- [Engine Architecture Research Entry](../research/engine-architecture-entry.md)

## Related Design Owners

- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)

## Purpose

This document defines the hard physical vocabulary of the world.

The goal is not a continuous real-world physics engine. The goal is a discrete
RPG simulation grammar rich enough for combat, stealth, crafting, building,
fire, poison, wounds, traces, tools, monsters, and basic magic to share one
causal substrate.

## Core Principle

Physical systems should share primitives where that creates real explanatory
power, but the grammar should stay typed and game-suitable.

```text
shared substrate:
  material, substance, containment, body, condition, signal, trace, process

not shared by force:
  every feature-specific rule collapsed into one vague tag system
```

The physical layer answers:

```text
What exists physically?
Where is it?
What is it made of?
What is attached, contained, equipped, embedded, emitted, damaged, or changing?
```

It does not answer:

```text
Who owns it socially?
Was this a crime?
Who believes what?
What should an actor do next?
```

Physical grammar is reusable world-simulation library, not one game's complete
physics, combat, crafting, or magic system. It defines common categories and
mutation surfaces. Exact material, damage, wound, condition, poison, disease,
structural, and magical taxonomies may be supplied by game-system packs when
they are world- or genre-specific.

## Boundary

Physical simulation owns:

- entity and physical object facts
- location, topology, containment, attachment, equipment, and embedded-object
  relationships
- material, substance, property, residue, contamination, and field facts
- body, body part, wound, condition, and sense facts
- physical access and reachability inputs
- physical signals such as light, sound, scent, heat, smoke, and magical aura
  where modeled as hard signals
- passive physical processes
- physical event families

Physical simulation does not own:

- legal ownership
- social permission
- law, norm, taboo, ritual meaning, or reputation
- memory, belief, knowledge, rumor, or secret
- semantic appraisal, thought, pressure, goal, or intent
- UI wording or narrative framing

## Core Vocabulary

### Entity And Physical Object

An entity is a physically addressable thing in the world.

Examples:

- actor body
- sword
- cloak
- door
- blood pool
- smoke cloud
- ritual seal
- wagon
- wall segment

Physical objects should have stable identity when future memory, history,
replay, or semantic context may refer to them.

### Location, Topology, And Containment

The grammar should use one typed topology vocabulary for physical placement and
access:

```text
LocatedIn(entity, place)
ContainedIn(entity, container)
EquippedInSlot(item, actor, slot)
AttachedTo(entity, target)
EmbeddedIn(entity, body_part_or_object)
PassageTo(place_a, place_b)
```

Inventory, equipment, body slots, interiors, attachments, wounds, and embedded
objects should not become unrelated location systems.

Physical possession is containment or equipment. Social ownership is a
`SocialClaim` and belongs to the social/institutional layer.

### Material, Substance, Property, And Dynamic State

Materials describe what an object is made of. Substances describe physical
matter that can be attached, pooled, mixed, carried, or dispersed.

Examples:

```text
Material:
  wood, iron, flesh, bone, glass, cloth, stone

Substance:
  blood, oil, poison, ash, water, acid, smoke residue

Property:
  flammable, brittle, conductive, wet, contaminated, sharp, hot, cold
```

Properties should be typed enough to drive validation and effects. Avoid
string-only tags where gameplay depends on the result.

Separate the ownership of physical qualities:

```text
MaterialProfile:
  mostly static facts about material kind.

ObjectPhysicalState:
  dynamic facts on an object, body part, structure, tile, or container.

SubstanceState:
  dynamic facts about a quantity of matter, residue, liquid, gas, powder, or
  contamination.

PhysicalCondition:
  typed state such as burning, soaked, poisoned, stunned, bleeding, blinded, or
  frozen.

DerivedExposure:
  computed fact such as exposed_to_fire, wet_enough_to_resist_ignition,
  contagious, slippery, visible_smoke, or reachable_surface.
```

Static material profile, dynamic state, physical condition, and derived
exposure should not collapse into one tag list.

Candidate material fields:

```text
MaterialProfile:
  hardness
  durability
  density
  flammability
  conductivity
  permeability
  brittleness
  sharpness_affinity
  insulation
  magical_conductivity?
```

Candidate substance fields:

```text
SubstanceState:
  kind
  quantity
  phase: solid | liquid | gas | powder | residue
  temperature
  purity
  toxicity
  viscosity?
  volatility?
  contamination: ContaminationState?
  magical_charge?
```

```text
ContaminationState:
  kind: poison | disease | acid | blood | ash | magical_residue | other typed kind
  source
  carrier
  dose_or_intensity
  freshness
```

These are design categories, not a required first schema. The important point
is that effects such as ignition, corrosion, poison transfer, smoke creation,
and contamination read typed physical properties instead of string tags.

### Body, Wound, Condition, And Sense

The physical layer owns body facts and physical conditions.

Examples:

```text
Body:
  torso, head, left_hand, right_hand, eyes, ears, wings

Wound:
  cut, bruise, burn, fracture, bleeding, embedded_arrow

Condition:
  blinded, deafened, poisoned, exhausted, burning, stunned

Sense:
  sight, hearing, smell, touch, magical_sense
```

Derived capacity scores such as grip, balance, fine manipulation, and movement
control are capability derivation outputs, not foundational physical facts.

### Inventory, Equipment, Attachment, And Embedded Objects

The grammar must distinguish:

- carried in container
- held in body part
- worn in equipment slot
- attached to surface
- embedded in object or body part
- installed into body or object

These distinctions matter for reachability, action validation, perception,
damage, evidence, and later semantic appraisal.

### Signal, Trace, Residue, Contamination, And Field

These should be a unified family rather than unrelated systems.

```text
Signal:
  emitted physical information that can be perceived now, such as sound, light,
  heat, scent, smoke, vibration, or aura.

Trace:
  durable evidence of a past event, such as footprints, broken branches, blood
  spatter, tool marks, ash, or lingering scent.

Residue:
  substance attached to a surface, item, body, tile, or container.

Contamination:
  harmful or meaningful substance/property state that can affect future
  contact, perception, disease, poison, or semantic interpretation.

Field:
  spatially distributed physical state such as smoke, gas, heat, light,
  magical pressure, or dangerous terrain effect.
```

The exact representation can differ by performance needs, but the concepts
should share source events, provenance, perception hooks, and cleanup/process
rules.

### Passive Physical Process

Not all physical change is actor-caused.

Examples:

- fire spreads
- bleeding continues
- poison acts
- smoke disperses
- scent decays
- wet cloth dries
- wound heals
- structure collapses
- ritual charge dissipates

Passive processes are scheduled or continuous runtime state. They still commit
through the causal runtime and emit structured events.

## Relationship To Typed Effects

Typed effects are the mutation language over this grammar.

Examples:

```text
ignite(wooden_door)
  reads material/property
  writes burning condition or fire process
  emits FireStarted / SmokeEmitted / HeatEmitted

apply_damage(left_hand, cut_damage)
  reads body part and protection
  writes wound and possibly condition
  emits BodyPartWounded / BloodResidueAttached

attach_substance(blood, cloak)
  writes residue
  emits SubstanceAttached

set_lock_state(gate_lock, unlocked)
  reads lock/mechanism state
  writes lock state
  emits LockStateChanged / LockUnlocked

set_mechanism_state(portcullis_chain, jammed)
  reads mechanism parts and obstruction
  writes mechanism state
  emits MechanismStateChanged
```

The physical grammar defines what can be changed. Typed effects define checked
ways to change it.

## Relationship To Capability

Actor-owned capability is derived from physical and actor-owned facts.

Example:

```text
physical hard truth:
  right_hand has deep_cut
  actor holds lockpick

actor-owned skill/procedure input:
  actor knows lockpicking

CapabilitySet:
  fine_manipulation = degraded
  ApplyTool(lockpick, target, pick_lock) remains owned but higher risk
```

The wound is hard truth. The degraded capability is a derived actor-facing
projection.

## Relationship To Perception

Perception samples physical state through actor-owned perceptual capability.

Example:

```text
hard truth:
  door has hidden tripwire
  door has suspicious scratch marks

ObservedState for untrained actor:
  closed old door

ObservedState for trained thief:
  closed old door, suspicious wire marks, trap suspected
```

The physical layer stores the facts and signals. Perception decides what this
actor can observe.

## Relationship To Social Meaning

Physical facts can become evidence for semantic interpretation, but they do not
own meaning.

Example:

```text
physical:
  blood residue on cloak

epistemic:
  guard observed blood on cloak

social / semantic:
  suspicious evidence in murder context
```

## Stress Cases

The grammar should be tested against:

- torch applied to wooden door
- wounded hand trying to pick a lock
- smoke obscuring sight and carrying scent
- poison applied to blade and later transferred by wound
- corpse leaving blood, scent, and social consequences
- wall damaged, repaired, or built over time
- ritual seal emitting magical signal and blocking passage
- stolen shrine item preserving physical transfer separate from `SocialClaim`

## Stable Decisions

- Physical simulation is discrete RPG physics, not continuous real-world
  physics.
- Physical grammar is reusable world-simulation library; concrete physical
  taxonomies can be pack-owned.
- Physical possession is hard containment/equipment state.
- Social ownership is `SocialClaim`, not physical possession.
- Signals, traces, residues, contamination, and fields should share provenance
  and event hooks where possible.
- Capability scores are derived views, not foundational hard truth.
- Passive processes mutate through the causal runtime and emit events.

## Deferred Decisions

- exact material/property taxonomy
- exact field representation and propagation rules
- first body-part schema
- first wound and condition taxonomy
- first passive process families
- how much magic is physical signal versus semantic rule
- performance layout for local hot physical state
