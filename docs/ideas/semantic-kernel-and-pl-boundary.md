# Semantic Kernel And PL Boundary

## Status

Promoted source history.

## Promotion Note

Stable design content from this idea has been promoted into:

- [Truth, Authority, And Layer Boundaries](../design/truth-authority-and-layer-boundaries.md)
- [Social Institutional Model](../design/social-institutional-model.md)
- [Semantic Appraisal And Motivation](../design/semantic-appraisal-and-motivation.md)

Later PL-specific material should be promoted into a dedicated PL authoring and
verification design document.

## Core Idea

`world` should have a hard typed kernel for deterministic causal simulation and
a PL-shaped semantic layer for context-dependent meaning.

The boundary principle:

```text
Kernel owns causality.
PL owns meaning.
```

Equivalent:

```text
Kernel decides what happened.
PL decides what it counts as.
```

This separates authoritative world mutation from social, psychological,
cultural, legal, and narrative interpretation.

## Why This Boundary Matters

The project needs rich world semantics:

- a death may be murder, sacrifice, execution, accident, or victory
- taking an item may be theft, permitted use, ritual duty, or necessity
- entering a room may be normal movement, trespass, taboo, or infiltration
- speech may be a promise, lie, insult, oath, threat, or confession

These meanings are context-dependent and actor-specific. The kernel should not
hardcode them.

At the same time, the semantic layer must not freely mutate authoritative state.
It should not move actors, damage bodies, transfer items, or unlock doors.

The clean split is:

```text
Kernel:
  physical and causal truth

PL:
  interpretation of observed truth
```

## Kernel Responsibilities

The kernel should own context-independent state and execution.

Candidate responsibilities:

- stable ids: actors, items, entities, actions, events
- deterministic time and turn ordering
- seeded RNG
- entity storage
- map, topology, position, occupancy
- body, health, physical capacity
- inventory and containers
- physical action execution
- physical state mutation
- typed event log
- perception projection into observed events
- replay transcript

The kernel answers:

```text
What is possible?
What state changed?
What happened?
Who could observe it?
```

Examples:

```text
ActorMoved
AttackResolved
ActorWounded
ActorDied
ItemTaken
DoorOpened
SoundEmitted
SpeechActPerformed
```

The kernel may know that a body died. It should not decide that the death was
murder, justice, tragedy, or holy sacrifice.

## PL Responsibilities

This is historical/future authoring direction. Current ownership is split:
truth/layer authority, social state, semantic appraisal, epistemic state, and
intent planning each have separate design owners. A future PL design may
express and check rules over those owners; it should not collapse them into one
current subsystem.

The PL-shaped semantic layer could help express context-dependent
interpretation.

Candidate future authoring responsibilities:

- event interpretation
- law, norm, taboo, and ritual meaning
- relationship and social meaning
- ownership and permission consequences
- identity, recognition, belief, and rumor interpretation
- memory, thought, and pressure generation
- candidate intent bias and scoring
- explanation and provenance

The PL answers:

```text
What does this event mean to this actor?
Which context applies?
Who cares?
What memory, belief, pressure, or intent follows?
```

Examples:

```text
ActorDied + close relation + known killer
  -> grief and revenge pressure

ItemTaken + owner is shrine + no permission + shrine norm forbids taking
  -> theft/taboo interpretation and guard duty pressure

ActorMoved + forbidden area + observer is shrine guard
  -> trespass interpretation and warning/arrest intent bias
```

## Boundary Examples

### Attack

```text
Kernel:
  can actor attack target?
  hit or miss?
  how much damage?
  did the target die?
  emit ActorDied

PL:
  was this murder, lawful execution, revenge, sacrifice, or battle?
  who grieves?
  who seeks revenge?
  who reports a crime?
```

### Item Taking

```text
Kernel:
  item moved from container/tile to actor inventory
  emit ItemTaken

PL:
  was it theft?
  was the actor permitted?
  was the item sacred?
  does any observer care?
```

### Movement

```text
Kernel:
  actor moved from one tile to another
  emit ActorMoved

PL:
  was this trespass?
  did it violate taboo?
  did it look threatening?
```

### Speech

```text
Kernel:
  actor performed a speech act
  listener heard it
  emit SpeechActPerformed

PL:
  was it insult, promise, confession, lie, oath, or threat?
  did the listener believe it?
  did it create obligation or suspicion?
```

## Stage Permissions

The boundary should be enforced by stage permissions.

Kernel/action stages may emit physical effects and typed events.

Allowed:

- move actor
- transfer item
- apply damage
- set door or lock state
- spend time or resources
- emit physical and sensory events

Forbidden:

- create grief
- create revenge
- declare theft
- change relationship because of meaning
- create belief or rumor

Semantic interpretation stages may emit semantic effects.

The current design splits this older sketch across separate owners:

- epistemic persistence creates `EpistemicRecord`s
- social commit creates accepted social/institutional state and `SocialClaim`s
- appraisal creates `Thought`, `Pressure`, `GoalPressure`, and semantic records
- intent planning owns intent binding, scoring, and final choice

So the permission list below is historical pressure, not a current unified
semantic-stage API.

Allowed:

- propose epistemic update
- create thought
- create pressure
- propose belief update or social state update
- propose intent bias
- record provenance

Forbidden:

- move actor
- transfer item
- apply damage
- unlock door
- directly mutate physical state

## Typed Semantic Stores

The semantic layer needs typed context, but that does not mean every context
fact should collapse into one generic graph.

Better:

```text
RelationState
OwnershipState
NormState
RoleState
SocialClaimState
EpistemicRecord
ThoughtState
PressureState
IntentBiasProposal
```

These stores can share metadata:

- source event
- source actor
- confidence where relevant
- created time
- expiration
- provenance

But their internal structure should stay typed.

Example:

```text
RelationState
  relation(player, mentor_1):
    kind: mentor
    emotional_weight: high
    trust: high
```

Example:

```text
NormState
  shrine_inner_room:
    forbids: non_priest_entry
    severity: high
```

## Why Not One Generic Fact Graph

A universal fact graph is tempting:

```text
Fact(subject, predicate, object, qualifiers)
```

But it is risky as the core architecture.

Risks:

- weak type safety
- unclear distinction between truth and belief
- stringly predicates
- fragile queries
- hard-to-debug rule firing
- gameplay rules hidden inside graph traversal
- too much abstraction before the game model is known

`world` should keep typed domain state and expose it through typed context
queries.

## Why Not Many Isolated Hardcoded Systems

The opposite extreme is also risky:

```text
RelationSystem
OwnershipSystem
LawSystem
IdentitySystem
PressureSystem
IntentSystem
```

If each system owns its behavior in isolation, glue code spreads everywhere.

Bad shape:

```text
if victim_is_close_relation
and killer_is_known
and local_law_forbids_murder
and observer_is_guard
then ...
```

This logic should live in checked semantic rule modules that combine typed
context queries.

## Social Context View

Semantic rules should see a typed `SocialContextView`, not arbitrary internal
state. Current design splits this into authoritative, holder-known, and
role-granted social context views.

Example:

```text
SocialContextView(observer, observed_event)
  relation_to(victim)
  recognized(cause_actor)
  known_social_claims_about(cause_actor)
  applicable_norms(location)
  ownership_of(item)
  roles_of(observer)
  current_pressures(observer)
```

This lets rules combine relation, law, ownership, role, belief, and memory
without requiring one untyped fact graph.

The interpretation rule should run on observed state plus context, not raw
omniscient truth.

## PL As Semantic Entry Point

For a future PL design, checked semantic rules should be the required entry
point for game-specific meaning. This is not a current implementation
requirement.

Allowed:

```text
ActorDied observed by player
  -> semantic rule creates grief/revenge pressure
```

Not allowed:

```text
Rust code somewhere:
  if mentor died:
    add revenge pressure
```

Primitive simulation mechanics are not "PL bypass." They are kernel
responsibility. Game-specific interpretation bypassing the semantic layer is
the problem.

## Open Questions

- What belongs in the hard kernel versus typed semantic stores?
- Which semantic stores are needed first?
- What should `SocialContextView` expose?
- How should truth, observation, claim, and belief be typed?
- How much semantic interpretation should be visible to the player?
- What semantic effects need provenance?
- Which stage permissions should be enforced by the checker first?

## Related References

- [Action and Event Model](../design/action-event-model.md)
- [Perception And Observation](../design/perception-and-observation.md)
- [Knowledge, History, And Belief](knowledge-history-and-belief.md)
- [Actor Pressure And Interpretation](actor-pressure-and-interpretation.md)
- [Actor Intent And Activity](actor-intent-and-activity.md)
