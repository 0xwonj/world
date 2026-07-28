# Perception And Observation

## Status

Partially superseded nonnormative perception design input.

This document replaces the older `Perception and Knowledge` framing. Knowledge,
belief, memory, rumor, and secrets now belong to
[Epistemic State](epistemic-state.md). This document only covers current
actor-relative perception and observation projection.

The normative disclosure, projection, evidence, and routing contracts live in
[Cognition And Agency](../architecture/target-architecture/cognition-and-agency.md).

## Related Designs

- [World Model](world-model.md)
- [Physical Simulation Grammar](physical-simulation-grammar.md)
- [Causal Runtime](causal-runtime.md)
- [Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md)
- [Epistemic State](epistemic-state.md)
- [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)

## Purpose

Perception turns authoritative world state and committed domain occurrences
carried by post-commit reaction work into
actor-relative observations.

It answers:

```text
What can this actor currently sense, and how does the world or committed
domain occurrence
appear from that actor's perspective?
```

It does not answer:

```text
What does the actor remember?
What does the actor believe?
What does the observation mean socially or emotionally?
What should the actor do?
```

Those belong to later layers.

## Position In The Engine

```text
Authoritative domain state / committed reaction input
  -> ObservationProjection
  -> EvidenceDelivery proposal
  -> Epistemic gate
  -> later appraisal
  -> later intent
  -> persistent activity
  -> one-shot grounded action opportunity
```

Stable `Goal` records may also influence later intent selection, but
perception and appraisal output motivational pressure as `GoalPressure`.

Perception is a projection layer. It must not mutate hard state and must not
leak omniscient truth.

## Boundary

Perception owns:

- actor-relative sensing
- visibility, audibility, smell, touch, magical detection, and other channels
- observation projection from committed `EventRecord`s
- current observed state projection from world state
- perceived roles in observed events
- uncertainty, partial identification, and confidence
- recognition that depends on the actor's body, senses, distance, conditions,
  equipment, and accessible epistemic context

Perception does not own:

- persistent memory
- belief, knowledge, rumor, or secret state
- semantic appraisal into thought, pressure, or goal
- intent generation
- hard mutation
- action validation
- social truth itself

## Actor-Owned Perceptual Capability

Perception follows the same ownership principle as the actor-owned action
repertoire: the ability to perceive is derived from the actor, not from external
objects.

Perceptual capability comes from actor-owned state:

- body and sensory organs: eyes, ears, nose, skin, magical senses, antennae
- conditions: blindness, deafness, pain, fatigue, poison, silence, panic
- controlled equipment: torch, lens, mask, detector, familiar, carried light
- skills and procedures: tracking, literacy, monster lore, ritual diagnosis
- magic and active effects: divination, aura sight, detect poison, true seeing

The outside world supplies signals and constraints: light, distance, occlusion,
noise, scent, disguise, terrain, walls, weather, and target-emitted properties.
Those can make an observation possible, easier, harder, misleading, or
impossible, but they do not create the actor's perceptual capability.

Examples:

```text
carried torch:
  actor-owned equipment, modifies the observer's sight capability.

wall torch:
  environmental signal, improves local visibility but is not owned by the actor.

target has magical aura:
  target-emitted signal.

actor has aura sight:
  actor-owned perceptual capability that can receive that signal.
```

Recognition may also use actor-accessible epistemic context, such as a known
face, faction symbol, language, or remembered voice. That context parameterizes
recognition; it does not turn perception into persistent memory.

## Core Types

```text
PerceptionContext
  observer
  observer perceptual capability
  observer body and senses
  observer conditions
  observer equipment and carried light
  current place and local environment
  relevant world state
  relevant committed EventRecord set
  accessible recognition context

ObservedState
  observer
  subject
  perceived_kind
  perceived_properties
  channels
  confidence
  freshness
  uncertainty

ObservedEvent
  observer
  source_event
  perceived_event_kind
  perceived_roles
  channels
  confidence
  uncertainty
```

These names are conceptual. The final implementation may use different
serialized shapes.

## Observation Channels

Channels are enabled or degraded by actor-owned perceptual capability, then
resolved against world signals and environmental constraints.

Initial channels:

```text
sight:
  visibility, light, line of sight, distance, occlusion, blindness, disguise.

sound:
  noise, distance, walls, directionality, hearing ability, silence effects.

smell:
  scent, blood, smoke, decay, species senses, wind or airflow if modeled.

touch:
  contact, texture, heat, pressure, pain, vibration.

magic:
  detection effects, wards, aura, divination, illusions, hidden magical state.

social recognition:
  face, voice, clothes, insignia, gait, reputation marker, known name.
```

Channels can disagree. The output should preserve that uncertainty instead of
forcing one omniscient answer too early.

Example:

```text
sight:
  a hooded figure near the gate

sound:
  voice resembles guard_1

recognition:
  maybe guard_1, confidence=medium
```

## Observed State

Observed state is the actor-facing projection of current world state.

Examples:

```text
Hard truth:
  door_1 is locked and trapped.

ObservedState for untrained actor:
  subject=door_1
  perceived_properties=[closed, old iron lock]

ObservedState for trained thief:
  subject=door_1
  perceived_properties=[closed, old iron lock, suspicious wire marks]
  confidence(trap_suspected)=medium
```

Observed state is current and transient. If it matters later, epistemic state
may persist a memory or belief derived from it.

## Observed Events

Observed events are actor-relative projections of committed `EventRecord`s.

Example:

```text
EventRecord:
  ActorDied(victim=mentor_1, cause_actor=bandit_1)

ObservedEvent for player:
  perceived_event_kind=violent_death
  perceived_roles:
    victim=mentor_1
    cause_actor=bandit_1
  channel=sight
  confidence=high

ObservedEvent for villager inside house:
  perceived_event_kind=violence_or_scream
  perceived_roles:
    victim=unknown
    cause_actor=unknown
  channel=sound
  confidence=low
```

The same `EventRecord` can produce different observations for different actors.

## Recognition

Recognition is where perception may consult actor-accessible epistemic context.

Examples:

- seeing a symbol and recognizing a faction
- hearing a voice and recognizing a person
- reading a script and recognizing a language
- seeing tracks and recognizing a monster type
- sensing an aura and recognizing a school of magic

Recognition should still be expressed as an observation:

```text
ObservedState
  subject=sigil_1
  perceived_kind=faction_symbol
  recognized_as=ashen_order
  confidence=medium
  recognition_source=holder knowledge / skill / condition / magic
```

Perception can say what the actor appears to recognize. Epistemic state records
what the actor knows or believes. Semantic appraisal decides what that
recognition means.

## Uncertainty And Hidden Truth

Perception must preserve uncertainty.

Examples:

```text
unknown actor:
  someone in a red cloak

partial role:
  the player saw the victim fall but did not see the attacker

mistaken source:
  the actor heard a shout from the north, but the source was actually west

false recognition:
  a disguised noble appears to be a merchant
```

Hidden hard truth should not appear in actor-facing observations unless the
actor has a channel that can reveal it.

## From Perception To Epistemic State

Perception output is usually transient.

Only salient or useful observations should pass through the epistemic
persistence gate.

```text
ObservedState / ObservedEvent
  -> Epistemic State persistence gate
  -> EpistemicRecord create/update
```

Examples:

```text
ObservedEvent:
  saw bandit_1 kill mentor_1

EpistemicRecord:
  remembered EventRecordRef(actor_died_456)
```

```text
ObservedState:
  saw suspicious wire marks on door_1

EpistemicRecord:
  believed door_1 may be trapped
```

Perception should not decide long-term importance by itself. It supplies
channels, confidence, perceived roles, and uncertainty to the epistemic gate.

## Actor And AI Input

An AI agent or NPC policy should receive actor-facing observations, not hard
truth.

Input should combine:

```text
AgentTurnInput:
  current ObservedState / ObservedEvent
  WorkingSet from Epistemic State
  actor-owned CapabilitySet and ActionRepertoire
  PerceivedAffordance for observed targets and contexts
  actor-visible invalid-action feedback where relevant
```

Perception supplies only the current observation part. The complete
actor-facing interface is defined by
[Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md).

## Complexity Controls

The first stable design surface should stay small.

Keep the baseline surface to:

- sight
- sound
- basic recognition
- observed event projection
- confidence as `low | medium | high`
- hidden-truth filtering

Defer:

- smell simulation
- detailed acoustics
- detailed lighting math
- illusions and disguises beyond simple flags
- social recognition nuance
- magical perception taxonomy
- attention modeling
- precise probabilistic perception

## Hardcoding Boundary

Perception should use generic sensing and recognition rules, not story-case
scripts.

Avoid:

```text
if player is watching mentor death:
  create revenge observation
```

Allowed:

```text
line of sight says player can see EventRecord
recognition says victim is mentor_1
recognition says attacker is bandit_1
ObservedEvent preserves perceived roles and confidence
```

Revenge belongs to semantic appraisal, not perception.

## Domain design conclusions

- Perception is current actor-relative projection, not memory.
- Perception is derived from actor-owned perceptual capability plus world
  signals and environmental constraints.
- Knowledge, belief, rumor, and secret belong to epistemic state.
- Observed state and observed events must be actor-relative.
- Observation must preserve uncertainty and hidden truth boundaries.
- Recognition may use actor-accessible epistemic context, but the output is
  still an observation.
- Perception feeds epistemic state; it does not directly create pressure,
  intent, or action.

## Deferred Decisions

- exact channel taxonomy
- exact line-of-sight and sound propagation model
- attention and focus model
- detailed recognition rules
- illusion, disguise, and deception model
- how much perception belongs in kernel primitives versus perception rules
- player-facing phrasing for uncertainty
