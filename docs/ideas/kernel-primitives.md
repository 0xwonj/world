# Kernel Primitives

## Status

Promoted source history.

## Promotion Note

Stable design content from this idea has been promoted into:

- [Physical Simulation Grammar](../design/physical-simulation-grammar.md)
- [Typed Effect Primitives](../design/typed-effect-primitives.md)
- [Capability, Affordance, And Actor Interface](../design/capability-affordance-and-actor-interface.md)
- [Perception And Observation](../design/perception-and-observation.md)
- [World Model](../design/world-model.md)
- [Causal Runtime](../design/causal-runtime.md)
- [Time Model](../design/time-model.md)

This file remains as source history and a pressure list for later design work.
Current primitive names and `EventRecord` contracts are owned by
[Typed Effect Primitives](../design/typed-effect-primitives.md), including
`set_open_state`, `set_lock_state`, and `set_mechanism_state`.

## Core Idea

The kernel should own the hard causal substrate of the game world.

Its job is to decide what is physically possible, what state changed, what
events were emitted, what each actor could observe, and how the result can be
replayed.

The kernel should not decide what events socially, legally, emotionally, or
narratively mean.

```text
Kernel owns hard truth and causal mutation.
Semantic layers own meaning.
```

Equivalent:

```text
Kernel:
  what happened

Semantic layer:
  what it counts as
```

Examples:

```text
Kernel:
  actor died
  item moved from shrine to actor inventory
  door became passable
  blood residue attached to cloak
  sound signal emitted

Semantic layer:
  murder
  theft
  sacrilege
  justified killing
  ritual offering
  suspicious evidence
```

This document is a first hypothesis before studying larger reference
implementations such as Cataclysm: Dark Days Ahead. The goal is not to lock the
architecture, but to create a clear lens for reference research.

## Design Principle

The kernel should be small enough to remain deterministic and inspectable, but
rich enough that higher-level systems do not need to bypass it.

Useful pressure:

```text
Can combat, stealth, crafting, building, fire, poison, traces, wounds,
equipment, and basic magic all use the same physical substrate?
```

If the answer is no, the kernel is probably missing a primitive.

If every feature requires a new bespoke primitive, the kernel is probably too
specific.

## Non-Responsibilities

The kernel should not own:

- law
- morality
- social permission
- ownership as social right
- grief, revenge, shame, duty, or loyalty
- narrative framing
- quest meaning
- faction interpretation
- religious meaning
- whether an event was heroic, criminal, holy, taboo, or insulting

These can be derived or authored in semantic layers from kernel events,
observations, beliefs, relationships, institutions, and norms.

## Primitive Families

The first candidate primitive families are:

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

These are not separate gameplay systems. They are the hard substrate that
gameplay systems use.

One important derived layer is intentionally not listed as a primitive family:

```text
Actor-Owned Capability Derivation
```

Capability derivation computes an actor's action repertoire from actor-owned
hard truth. It may use physical capacity scores internally, but those scores
are derived views, not foundational kernel facts.

## Identity / Time / RNG / Replay

The kernel needs stable identities and deterministic execution.

Candidate primitives:

```text
EntityId
ActorId
ItemId
PlaceId
EventId
ActionId
Turn
RngStream
EngineVersion
ContentVersion
```

Responsibilities:

- assign and track stable ids
- own deterministic time and turn ordering
- own all simulation randomness
- preserve event ordering
- support replay from seed, content version, engine version, initial state, and
  ordered action log

All nontrivial mutation should be traceable through events.

## Location / Topology / Containment

Location should be more general than a grid coordinate.

RPG objects can be on the ground, inside a container, equipped on a body, held
in a hand, attached to a door, embedded in a wound, or abstractly located in a
faraway place.

Candidate shape:

```text
Location =
  InCell(map_id, x, y)
  InContainer(container_id)
  Equipped(actor_id, slot)
  Held(actor_id, hand)
  AttachedTo(entity_id, attachment_point)
  EmbeddedIn(body_part_id)
  AbstractPlace(place_id)
```

This lets the same containment model represent:

- a sword on the floor
- a key in a bag
- a torch held in a hand
- an arrow embedded in a shoulder
- a seal attached to a door
- a distant town represented at abstract resolution

The kernel should own physical containment and reachability. It should not own
social possession, legal ownership, or permission.

## Entity / Physical Object

An entity is a physically addressable world object.

Examples:

- actor
- item
- terrain feature
- structure
- liquid volume
- gas cloud
- fire
- corpse
- machine
- portal
- plant

Candidate shape:

```text
Entity {
  id
  kind
  location
  physical_form
  state
}
```

`kind` is a kernel classification, not a semantic judgment.

For example:

```text
Corpse:
  kernel fact

martyr, victim, sacrifice, ancestor, enemy trophy:
  semantic interpretations
```

The kernel may know that an actor became a corpse. It should not decide what the
corpse means.

## Material / Substance / Property

The project needs a unified physical substrate, not separate unrelated systems
for traces, fields, liquids, fire, poison, residue, and contamination.

This should be a discrete turn-based RPG physics model, not a continuous
real-world physics engine.

Candidate shape:

```text
Material {
  hardness
  density
  flammability
  conductivity
  permeability
  toxicity
  purity
  magical_charge?
}

Substance {
  material
  quantity
  temperature
  phase: solid / liquid / gas / powder / residue
}
```

Useful expressions:

```text
Trace
  residue + source + age + decay

Field
  concentration over space + propagation + decay

Contamination
  substance attached to surface, body, item, tile, or container
```

Examples this should support:

- blood on a cloak
- mud on boots
- oil spilled across a floor
- smoke spreading from a fire
- poison in a wound
- holy water residue on a blade
- scent left by a monster
- magical trace around a ritual site
- wet clothing drying over time
- rotten food contaminating a bag

The semantic layer can interpret these physical facts as evidence, pollution,
taboo, ritual purity, guilt, danger, or useful clue.

## Body / Condition / Sense

Actors should have bodies, not only stats.

Candidate shape:

```text
Body {
  parts
  slots
  senses
  wounds
}
```

Examples:

```text
BodyPart:
  left_arm
  right_hand
  eye
  heart
  wing
  tail
  horn

Sense:
  sight
  hearing
  smell
  touch
  taste
  magical_sense

Condition:
  wounded
  poisoned
  exhausted
  blinded
  silenced
  burning
  cursed
```

Core flow:

```text
body state
  + wounds
  + conditions
  + equipment constraints
  -> derived physical capability
  -> actor-owned action repertoire
```

Examples:

- a wounded arm may degrade manipulation, grip, or shield methods
- damaged eyes change visual observation
- a broken jaw may block speech actions
- wings may enable flight or gliding
- missing hands may prevent tool use
- pain or poison may increase action cost or risk

The kernel should own body facts, wound facts, sensory organs, and physical
conditions. It should not need to store a canonical `GripScore` or
`LimbScore` as hard truth.

Physical capacity scores can be useful implementation details:

```text
fine_manipulation
grip
balance
locomotion
speech_control
breathing
gesture_control
```

But these should usually be derived from body, wounds, equipment, and
conditions. They are part of capability derivation, not separate foundational
state.

The kernel also should not own how an actor emotionally responds to injury.
Pain, fear, grief, revenge, and shame belong to actor pressure and semantic
interpretation layers.

## Inventory / Equipment / Container

Inventory is a specialized form of containment.

Candidate shapes:

```text
Container {
  capacity
  allowed_contents
  physical_access_rules
}

EquipmentSlot {
  body_part
  slot_kind
  occupied_by
}
```

Kernel responsibilities:

- track what is physically held, worn, stored, attached, or embedded
- validate physical access
- validate weight, volume, slot, and body constraints
- expose inventory facts to observation when perceptible

Semantic non-responsibilities:

- legal ownership
- theft
- borrowing
- permitted ritual use
- rightful inheritance
- faction claim

Those meanings depend on context and belong above the kernel.

## Actor-Owned Capability Derivation

Capability derivation is a deterministic read-only projection from actor-owned
hard truth into action repertoire.

It is not a primitive mutation family. It does not create events and it does not
change world state directly.

Candidate flow:

```text
ActorOwnedState:
  body facts
  wounds and conditions
  controlled equipment
  installed implants or artifacts
  learned skills
  known procedures, rituals, and recipes
  internalized roles, permissions, and claims
    |
    v
CapabilityDerivation
    |
    v
ActionRepertoire:
  action schemas
  allowed parameter kinds
  actor-owned methods
  costs, risks, and degradations
```

Examples:

```text
hands + fine tool control + lockpicking skill + carried lockpick
  -> actor owns ApplyTool(tool, target, pick_lock)

known ritual phrase + speech or equivalent casting channel
  -> actor owns InvokeKnownRitual(ritual, target)

rank token + recognized identity
  -> actor owns AssertAuthority(target, claim)

wing body parts + low encumbrance + enough locomotion control
  -> actor owns Fly or Glide movement schema
```

External objects do not create the actor's repertoire. They expose perceived
affordances that a repertoire action can bind to:

```text
actor owns ApplyTool(torch, target, ignite)
door appears wooden and burnable
  -> request can bind and validate
```

This separation keeps the AI-agent interface stable:

```text
stable:
  action_repertoire

dynamic:
  perceived targets
  perceived affordances
  validation outcomes
  effect paths
```

Physical capacity scores are one possible derivation technique, not a required
kernel primitive:

```text
body + wounds + equipment + poison
  -> fine_manipulation = degraded
  -> lockpicking is risky or invalid
```

This is useful, but the hard truth remains the body, wound, equipment, and
condition state.

## Typed Effect Primitives

Actions should mutate kernel state through typed effect primitives.

Candidate initial vocabulary:

```text
move_entity(entity, destination)
transfer_entity(entity, from, to)
attach_substance(substance, target_surface)
remove_substance(substance, target_surface)
change_temperature(target, delta)
ignite(target)
extinguish(target)
apply_force(target, vector_or_intensity)
apply_damage(target, damage)
change_integrity(target, delta)
set_open_state(target, open_state)
set_lock_state(lock_or_lockable, lock_state)
set_mechanism_state(mechanism, mechanism_state)
alter_passability(target, passable)
emit_signal(kind, source, intensity)
create_entity(kind, location)
destroy_entity(entity)
schedule_process(process, time)
```

Example:

```text
Attack
  -> apply_force
  -> apply_damage
  -> attach_substance(blood, weapon)
  -> attach_substance(blood, target clothing)
  -> emit_signal(sound)
  -> emit BodyPartWounded
```

Example:

```text
SpillOil
  -> transfer_entity(oil, bottle, tile)
  -> attach_substance(oil, floor)
  -> emit SubstanceSpilled
```

Example:

```text
OpenDoor
  -> validate reachability
  -> validate mechanism and lock state
  -> set_open_state(door, open)
  -> alter_passability(door, true)
  -> emit DoorOpened
```

The exact vocabulary should be tested against real gameplay cases and reference
implementations.

## Passive Physical Processes

Not all physical change is caused by an actor action.

The kernel should support passive processes that advance over time and use the
same primitives as action effects.

Examples:

- fire spread
- smoke diffusion
- scent decay
- wound bleeding
- poison metabolism
- wetness drying
- corpse decay
- temperature drift
- structure weakening
- disease or contamination progression

Principle:

```text
Action effects and passive processes should call the same typed primitives.
```

For example, an actor can ignite oil with a torch, and nearby fire can ignite
oil through a passive process. Both should produce comparable physical events.

## Physical Events

Kernel events are structured facts before semantic interpretation.

Candidate event types:

```text
ActorMoved
EntityTransferred
SubstanceAttached
SubstanceRemoved
TemperatureChanged
EntityIgnited
EntityExtinguished
DamageApplied
BodyPartWounded
ActorDied
SignalEmitted
DoorOpened
ContainerOpened
EntityCreated
EntityDestroyed
ProcessAdvanced
```

Events should be structured enough for later interpretation.

Example:

```text
ActorDied {
  actor
  cause_event
  apparent_source
  location
  time
}
```

But events should avoid semantic judgment.

Bad kernel event:

```text
MurderCommitted
SacrilegePerformed
TheftCommitted
HeroicRescue
```

Better:

```text
ActorDied
ItemTransferred
ActorEnteredPlace
ActorSpoke
ActorOpenedContainer
```

The semantic layer can interpret those events differently for each actor,
faction, law, ritual system, or belief context.

## Observation Projection

The kernel should project actor-specific observations from state and events.

Candidate flow:

```text
observe(actor, event_or_state)
  -> ObservedEvent / ObservedState
```

Senses sample physical substrate:

```text
see:
  light, line of sight, occlusion, visible form

hear:
  sound signal, distance, obstruction

smell:
  scent field, residue, wind or diffusion

touch:
  contact, temperature, texture, pressure

magic sense:
  magical signal, residue, aura, disturbance
```

The kernel can say:

```text
actor smelled blood
actor heard a scream
actor saw a door open
actor noticed smoke
actor felt heat
```

It should not decide:

```text
actor understands this as murder
actor believes the killer was guilty
actor feels revenge
actor considers the smoke a ritual sign
```

Those belong to perception, belief, interpretation, and pressure layers.

## Relationship To Action Effects

Typed action effects depend on this kernel vocabulary.

```text
ActionRequest
  -> ActionDef
  -> typed effect primitives
  -> kernel state mutation
  -> physical events
  -> observation projection
  -> semantic interpretation
```

The action language should not bypass the kernel. It should only compose
approved typed primitives.

This allows authored actions, NPC actions, AI agent actions, passive processes,
crafting operations, and magic effects to share the same causal substrate.

## Relationship To Physics

Physics should not be a separate meaning-making system.

It should be the kernel's typed physical substrate:

```text
material
quantity
temperature
phase
residue
field
integrity
containment
passability
signal
```

This can support:

- combat
- stealth
- crafting
- alchemy
- building
- fire
- poison
- tracking
- environmental hazards
- physical evidence
- material-based magic

without creating unrelated systems for each feature.

## Relationship To Capability Derivation

The kernel owns the hard facts that capability derivation reads.

Examples:

```text
body part exists
body part wounded
tool is carried
artifact is installed
book was learned
actor holds a rank token
actor is poisoned
```

Capability derivation computes what the actor can attempt in principle:

```text
ActionRepertoire
  Move
  Inspect
  Manipulate
  ApplyTool
  Speak
  StartProcess
  InvokeKnownRitual
```

Perceived affordances then determine how those schemas can bind to observed
targets. Validation still checks hard truth at execution time.

This means the kernel primitive set should not grow a new primitive for every
possible ability score. It should preserve the facts from which ability can be
derived.

## Relationship To Semantic Layers

Semantic layers should interpret kernel facts.

Examples:

```text
Item transferred from shrine to actor inventory
  + shrine norm forbids removal
  + actor lacks permission
  -> theft, sacrilege, alarm pressure

Actor died from blade wound
  + observer loved victim
  + apparent source known
  -> grief, revenge pressure

Blood residue on cloak
  + actor knows murder happened nearby
  + cloak belongs to suspect
  -> suspicion or evidence claim
```

The kernel provides the facts. Semantic layers provide contextual meaning.

## Relationship To Long Processes

Crafting, building, institutions, ecology, and other long-running processes
should not bypass the kernel.

They can be represented as processes that schedule work and call typed effects.

Examples:

```text
Crafting
  materials + tools + skill + time
  -> transform substances/entities
  -> emit physical events

Building
  reserved space + materials + labor + time
  -> create or alter structures
  -> emit construction events

Ritual
  place + substances + bodies + words + timing
  -> physical effects and semantic commitments
```

The process may have semantic meaning, but any hard state mutation still flows
through kernel primitives.

## Initial Design Risks

- If the kernel is too small, higher layers will need to fake hard truth.
- If the kernel is too broad, it will absorb semantic meaning and become rigid.
- If material properties are too generic, they become vague tags.
- If material properties are too detailed, simulation cost and authoring cost
  explode.
- If events are too raw, semantic interpretation cannot work.
- If events are too semantic, the kernel starts making social judgments.
- If passive processes use a different path from actions, replay and debugging
  become harder.
- If containment is too simple, equipment, wounds, storage, attachments, and
  abstract locations become special cases.
- If observation projection leaks hard truth, actor knowledge and player
  knowledge collapse.
- If derived physical capacity scores are treated as hard truth, the kernel
  becomes feature-shaped around one body model.
- If capability derivation is too generic, it can become an untyped rule soup
  that is hard to debug.

## Open Questions

- What is the smallest useful typed effect vocabulary?
- Which physical concepts are truly kernel primitives?
- Should support, load, and structural collapse be first-class early or later?
- How generic should material properties be?
- Should fields and residues be unified under substance, or have specialized
  storage for performance?
- Should ownership have any kernel representation, or only physical possession?
- How should abstract locations relate to concrete locations?
- How should passive process scheduling interact with turn order?
- Which `EventRecord` contracts are mandatory for each primitive?
- How much of observation projection belongs in kernel versus perception rules?
- How should magic alter physical substrate without becoming untyped arbitrary
  mutation?
- What derived capability families are needed first: physical body, equipment,
  knowledge, skill, condition, social authority, magic?
- Should physical capability derivation use named scores, boolean traits, typed
  methods, or a mix?
- How should content definitions refer to body parts, materials, slots, and
  effects without becoming stringly typed?

## Reference Questions For CDDA

When studying Cataclysm: Dark Days Ahead, use this document as a lens.

Questions:

- Which concepts are engine primitives rather than data definitions?
- How are items, materials, body parts, wounds, and status effects represented?
- Where is item use data-driven, and where does it require code?
- How are long activities separated from atomic actions?
- How does the game model containers, equipment, wielded items, and embedded or
  attached objects?
- How are fire, smoke, scent, liquid, residue, contamination, or similar
  physical processes represented?
- What event, logging, debugging, or replay structures exist?
- How are actor-owned action schemas derived from body, equipment, condition,
  and knowledge?
- How are perceived target affordances derived from item, terrain, field,
  structure, and observation state?
- Where did accumulated feature pressure force special cases?
- Which implementation choices are useful lessons, and which are artifacts of
  CDDA's genre, age, or scope?

## Related References

- [Action and Event Model](../design/action-event-model.md)
- [Simulation Core](../design/simulation-core.md)
- [Perception And Observation](../design/perception-and-observation.md)
- [Typed Action Effects](typed-action-effects.md)
- [Semantic Kernel And PL Boundary](semantic-kernel-and-pl-boundary.md)
- [Actor-Owned Capability-Derived Actions](capability-derived-actions.md)
- [Actor Pressure And Interpretation](actor-pressure-and-interpretation.md)
- [Multi-Resolution Simulation](../design/multi-resolution-simulation.md)
- [Caves of Qud](../references/caves-of-qud.md)
