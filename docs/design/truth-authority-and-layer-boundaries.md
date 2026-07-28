# Truth, Authority, And Layer Boundaries

## Status

Frozen pre-target cross-system design.

The layered-truth motivation remains useful. Its stores, `CausalTransaction`,
and direct cross-layer contracts are superseded by the normative state
partitions, typed gates, and later-causal-work model in the
[Target Architecture Package](../architecture/target-architecture/README.md).

## Source Ideas

- [Semantic Kernel And PL Boundary](../ideas/semantic-kernel-and-pl-boundary.md)
- [Layered Truth And AI Co-Authority](../ideas/layered-truth-and-ai-coauthority.md)
- [Engine Architecture Research Entry](../research/engine-architecture-entry.md)

## Related Design Owners

- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)

## Purpose

This document defines which layers may author, mutate, interpret, or merely
propose different kinds of game truth.

It exists to prevent three failures:

- semantic rules mutating hard state directly
- physical effects declaring social or emotional meaning
- AI-generated content silently becoming authoritative without provenance

## Truth Layers

### Hard Truth

Hard truth is authoritative physical, causal, and runtime state.

Examples:

- entity identity and position
- containment, equipment, attachment, and embedded-object facts
- body parts, wounds, conditions, material composition, and substances
- process, reservation, scheduler, and RNG state
- committed `EventRecord`s

Hard truth is owned by the engine substrate:

- [World Model](world-model.md)
- [Physical Simulation Grammar](physical-simulation-grammar.md)
- [Typed Effect Primitives](typed-effect-primitives.md)
- [Causal Runtime](causal-runtime.md)
- [Time Model](time-model.md)

Hard truth may only change through a staged `CausalTransaction`.

`EventRecord` is reserved for hard truth. It records committed causal facts
such as movement, damage, transfer, sound, light, process progress, and runtime
failure outcomes. It is not the record type for theft, grief, holiness,
justice, insult, or obligation.

### Soft Truth

Soft truth is structured meaning, social context, institutional state, and
world-context information that is not direct physical mutation.

Examples:

- relationship weight
- faction membership
- norm, law, taboo, and permission
- `SocialClaim` over an item, place, role, or action
- reputation
- obligation, oath, debt, and duty
- hidden motive or unresolved social tension

Soft truth must be typed and provenance-backed when gameplay depends on it. It
may be authored, generated, inferred, or AI-proposed, but accepted game state
still needs a commit gate.

### Actor Truth

Actor truth is what a holder perceives, remembers, believes, knows, suspects,
or has access to.

Examples:

- `ObservedEvent`
- `ObservedState`
- `EpistemicRecord`
- remembered location
- believed culprit
- known secret
- rumored event
- known procedure

Actor truth may diverge from hard truth. It must never be treated as
authoritative physical state.

### Narrative Framing

Narrative framing is presentation, phrasing, and story-facing explanation.

Examples:

- UI text
- dialogue phrasing
- recap text
- generated prose summary
- quest-log wording

Narrative framing can reflect hard truth, soft truth, and actor truth, but it
does not own them.

## Stage Permissions

Game-system packs follow the same permissions as any other authoring source.
A pack may define vocabularies, rules, content schemas, and checked effect
programs, but it must not acquire a private mutation path around the relevant
commit gate.

### Kernel And Action Stages

Allowed:

- read hard truth through declared query surfaces
- validate physical and runtime requirements
- stage physical mutations
- stage process, reservation, scheduler, and RNG changes
- emit physical, sensory, and runtime `EventRecord`s

Forbidden:

- create grief, revenge, guilt, loyalty, or fear pressure
- declare theft, taboo, crime, insult, holiness, or justice
- create memory, rumor, or belief
- mutate relationship because of semantic meaning
- advance quest or narrative state as a hidden side effect

### Perception And Epistemic Stages

Allowed:

- project actor-relative observations
- preserve uncertainty and hidden-truth boundaries
- persist holder-relative `EpistemicRecord`s when the persistence gate accepts
  them
- attach provenance, confidence, salience, freshness, and access metadata

Forbidden:

- mutate hard physical truth
- decide social or emotional meaning
- select final intent or action

### Social Commit Stages

Allowed:

- query social and institutional context
- accept typed social or institutional state changes with provenance
- update relationships, membership, `SocialClaim`, norm state, reputation,
  debt, oath, duty, or permission through the social commit gate

Forbidden:

- move entities
- transfer items
- apply physical damage
- unlock doors
- rewrite committed physical `EventRecord`s

### Semantic Appraisal Stages

Allowed:

- query accessible social and institutional context
- interpret observed events and epistemic records
- produce `Thought`, `Pressure`, `GoalPressure`, and semantic provenance
  through the appraisal commit gate
- propose social or epistemic updates for later gates

Forbidden:

- directly commit hard truth
- directly commit social or institutional truth
- directly create `EpistemicRecord`s
- directly select final intent or action

### AI Proposal Stages

Allowed:

- propose soft truth
- propose appraisal interpretations
- propose memory summaries
- propose narrative framing
- propose distant or abstract simulation details where a later design permits it

Forbidden:

- directly mutate hard truth
- erase provenance
- silently contradict committed `EventRecord`s
- expose omniscient hard truth to actor-facing interfaces

## Authority Modes

The engine should support at least two explicit authority modes. The mode is a
game or scenario policy, not a hidden behavior of the AI layer.

### Strict Simulation Mode

AI may propose interpretation, narrative wording, or debug-facing suggestions,
but it cannot commit game truth.

```text
AI proposal
  -> typed checker / rule system
  -> accepted rule-derived result or rejection
```

Use this mode for deterministic replay, tests, validation, puzzle-like
simulation, and debugging.

### AI Co-Author Mode

AI may propose scoped soft truth or actor truth through a commit gate.

```text
AI proposal
  -> scope / contradiction / provenance checks
  -> accepted soft truth, actor truth, or narrative framing
```

Use this mode for dynamic rumors, hidden relationships, backstory links,
contextual motives, and richer single-player story. It still cannot directly
mutate hard truth or rewrite committed `EventRecord`s.

## PL And Tooling Boundary

The PL layer is not the owner of truth.

Its job is to express, check, inspect, and explain behavior over engine-owned
truth:

- typed action/effect definitions
- process/activity definitions
- semantic appraisal rules
- query and derived-view definitions
- content schemas
- migrations and versioning
- replay tests and invariant checks
- provenance and explanation tooling

Primitive mutation semantics belong to the engine. Authored language can call
checked primitives; it cannot invent unchecked mutation authority.

## Commit Gate

Any game-relevant proposal that would affect future simulation must pass
through a commit gate appropriate to its truth layer.

Examples:

```text
Physical mutation:
  must become a `CausalTransaction` and `EventRecord`.

Soft social fact:
  must become an `AcceptedSocialUpdate` with provenance.

Soft chronology:
  must become an `AcceptedChronologyRecord` with provenance.

Semantic appraisal:
  must become an `AcceptedAppraisalRecord` such as `Thought`, `Pressure`, or
  `GoalPressure` with provenance.

Actor memory:
  must become an `AcceptedEpistemicUpdate` that writes an `EpistemicRecord`
  with holder, content, mode, source, confidence, salience, and access
  metadata.

Narrative prose:
  may remain presentation if it has no gameplay effect.
```

## Commit Surfaces And Store Owners

Gameplay-relevant records are committed through different surfaces depending on
their authority class. The [World Model](world-model.md) hosts the stores and
query indexes; this document owns the authority boundary; domain documents own
record semantics.

| Authority class | Commit surface | Store family | Semantic owner | AI proposal authority |
| --- | --- | --- | --- | --- |
| Hard truth | `CausalTransaction` | `WorldStore`, hard `RelationStore` families, `RuntimeControlStore`, `EventHistoryStore` | [Physical Simulation Grammar](physical-simulation-grammar.md), [Typed Effect Primitives](typed-effect-primitives.md), [Causal Runtime](causal-runtime.md), [Time Model](time-model.md) | No direct AI commit |
| Soft social/institutional truth | `AcceptedSocialUpdate` through the social commit gate | `SocialInstitutionalStore`, soft `RelationStore` families | [Social Institutional Model](social-institutional-model.md) | Allowed only as a gated proposal |
| Soft chronology/world-context truth | `AcceptedChronologyRecord` through the chronology commit gate | `ChronologyStore` | scenario/worldgen design, [Multi-Resolution Simulation](multi-resolution-simulation.md), and relevant domain owner | Allowed only as a gated proposal |
| Actor truth | `AcceptedEpistemicUpdate` through the epistemic persistence gate | `EpistemicStore` | [Epistemic State](epistemic-state.md) | Allowed only as a gated proposal |
| Appraisal/motivation state | `AcceptedAppraisalRecord` through the appraisal commit gate | `AppraisalRecordStore` | [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md) | Allowed only as a gated proposal |
| Narrative framing | no authoritative gameplay commit unless imported into another layer | presentation record or no store | presentation layer | May generate prose; no gameplay effect by itself |

The rule is strict:

```text
Commit surface determines write authority.
Store family determines persistence and query location.
Domain owner determines meaning and lifecycle.
```

Examples:

- A sword hit commits through `CausalTransaction`, mutates hard stores, and
  appends an `EventRecord`.
- A temple ownership assertion commits through `AcceptedSocialUpdate` and
  writes `SocialInstitutionalStore`, even if the assertion references an
  `EventRecord`.
- A witness memory commits through `AcceptedEpistemicUpdate` and writes
  `EpistemicStore`, even if the memory content is an `EventRecordRef`.
- A grief or revenge pressure commits through `AcceptedAppraisalRecord` and
  writes `AppraisalRecordStore`, even if it was derived from memory and social
  context.

No semantic, social, epistemic, chronology, or AI proposal gate may mutate hard
truth. No hard physical effect may smuggle social meaning, memory, belief, or
appraisal state into an `EventRecord`.

## Examples

### Mentor Killed By Bandit

```text
Hard truth:
  ActorDied(mentor_1)
  DamageResolved(attacker=bandit_1, victim=mentor_1)

Actor truth:
  player observed bandit_1 kill mentor_1
  player remembers the event

Soft truth:
  player has close relationship to mentor_1
  village has norm against murder

Semantic appraisal:
  grief pressure
  retaliation pressure
  caution pressure depending on threat context
```

No physical effect creates grief directly. Grief is an appraisal result over
event, perception, memory, and relationship context.

### Shrine Relic Removed

```text
Hard truth:
  ItemTransferred(shrine_relic, shrine_floor, actor_inventory)

Soft truth:
  SocialClaim(shrine owns shrine_relic)
  Norm(shrine forbids non-priest removal)

Actor truth:
  guard saw actor take shrine_relic
  actor may or may not know the taboo

Semantic appraisal:
  theft interpretation
  sacrilege interpretation
  guard duty pressure
```

Physical possession, social ownership, and actor belief about ownership remain
separate.

## Relationships To Other Documents

- [World Model](world-model.md) defines the typed storage and query surfaces.
- [Typed Effect Primitives](typed-effect-primitives.md) defines allowed hard
  mutation primitives.
- [Standard World Library And Primitive Semantics](standard-world-library.md)
  defines where reusable primitive definitions and trusted semantics live.
- [Causal Runtime](causal-runtime.md) defines staging, commit, `EventRecord`
  append, replay, process, reservation, and reaction.
- [Epistemic State](epistemic-state.md) defines holder-relative actor truth.
- [Social Institutional Model](social-institutional-model.md) defines social
  and institutional state.
- [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md)
  will define the appraisal layer that turns context into pressure.

## Historical decisions

- Hard truth changes only through causal transactions.
- `EventRecord` is a hard causal fact record, not a semantic meaning
  record.
- Physical effects must not write semantic meaning directly.
- Actor truth may diverge from hard truth.
- `SocialClaim` is social/institutional state, not physical containment and not
  memory by itself.
- AI may propose but not directly commit hard truth.
- AI co-authority over soft truth or actor truth is mode-gated and
  provenance-backed.
- PL/tooling expresses and checks behavior; it does not replace trusted
  primitive semantics with arbitrary mutation code.
- [World Model](world-model.md) hosts store families and query surfaces, while
  each domain document owns record meaning and lifecycle.
- Non-hard gameplay records have explicit commit surfaces:
  `AcceptedSocialUpdate`, `AcceptedChronologyRecord`,
  `AcceptedEpistemicUpdate`, and `AcceptedAppraisalRecord`.
- Game-system packs do not bypass authority boundaries; they author checked
  declarations that commit through the same hard and non-hard gates.

## Deferred Decisions

- exact record schema for each non-hard commit surface
- exact AI proposal review and acceptance policy
- whether soft truth can ever be replay-authoritative without deterministic
  generation
- detailed PL/checker design
- player-facing provenance UI
