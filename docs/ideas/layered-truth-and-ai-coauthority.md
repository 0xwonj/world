# Layered Truth And AI Co-Authority

## Status

Promoted source history.

## Promotion Note

Stable design content from this idea has been promoted into
[Truth, Authority, And Layer Boundaries](../design/truth-authority-and-layer-boundaries.md).
Multi-resolution and AI-assisted distant-simulation material is now owned by
[Multi-Resolution Simulation](../design/multi-resolution-simulation.md).

## Core Idea

Not all truth in `world` should have the same authority level.

The hard simulation kernel should protect physical, causal truth. But an
AI-native RPG may reasonably allow AI to co-author softer layers of truth:
background motives, rumors, social interpretations, hidden relationships,
actor beliefs, and narrative framing.

The useful distinction is:

```text
Hard Truth
  physical and causal facts protected by the kernel

Soft Truth
  semantic, historical, social, and story facts that may be co-authored

Actor Truth
  actor-specific belief, memory, rumor, suspicion, and interpretation

Narrative Framing
  wording, mood, description, and presentation
```

AI authority should depend on the truth layer.

## Relationship To Kernel And PL Boundary

This idea extends [Semantic Kernel And PL Boundary](semantic-kernel-and-pl-boundary.md).

The boundary still matters:

```text
Kernel owns causality.
PL owns meaning.
```

AI co-authority should not erase that boundary. Instead, it introduces a policy
question:

```text
Which layers of truth may AI propose or commit?
```

The answer should not be the same for every layer.

## Why This Matters

A strict simulation can be consistent and replayable, but it may require a lot
of authored semantic content before the world feels narratively rich.

An AI co-author can create connections that are hard to author exhaustively:

- the bandit killed the mentor because of an old debt
- the village misremembers the killer's faction
- the merchant secretly knew the mentor
- a faction interprets the murder as political provocation
- a rumor connects the event to a forgotten oath
- an NPC forms a false but plausible suspicion

These can make a single-protagonist RPG feel more alive.

The risk is that AI can also create contradictions, unfair outcomes, hidden
retcons, or unverifiable story drift. That is why authority must be layered.

## Truth Layers

### Hard Truth

Hard truth is the authoritative physical and causal state owned by the kernel.

Examples:

- actor positions
- item locations
- inventory transfers
- body wounds
- actor death
- door and lock state
- action log
- typed event log
- turn order
- deterministic random choices

AI should not directly mutate hard truth.

Bad:

```text
AI decides the mentor did not die after ActorDied was committed.
AI moves the bandit to another town without an action or event.
AI removes the stolen item from inventory because the story needs it.
```

Hard truth should change through:

```text
ActionRequest
  -> typed action effects
  -> kernel primitive effects
  -> typed events
```

### Soft Truth

Soft truth is semantic, historical, social, or story-relevant truth that does
not directly contradict hard physical facts.

Examples:

- motive
- hidden relationship
- old debt
- faction interest
- backstory link
- secret allegiance
- regional rumor origin
- why an NPC was traveling
- what a symbol historically means

AI may be allowed to propose or commit soft truth, depending on game mode and
scope.

Example:

```text
Hard Truth:
  bandit_1 killed mentor_1 at old_bridge

Soft Truth:
  bandit_1 killed mentor_1 because mentor_1 once betrayed the Red Shrine
```

Soft truth can later influence semantic interpretation, dialogue, rumor, and
intent bias.

### Actor Truth

Actor truth is what a specific actor believes, remembers, suspects, heard, or
claims.

It may be incomplete or false.

Examples:

- guard believes merchant killed priest
- villager heard the killer wore a red sash
- player remembers the bandit's face
- merchant suspects the mentor had a secret debt
- priest believes the death was divine punishment

AI can have more freedom here because actor truth is not global truth.

Example:

```text
EpistemicRecord
  holder: guard_1
  content: Proposition(merchant_1 killed priest_1)
  mode: believed
  confidence: medium
  source: tavern_rumor
```

Actor truth can drive pressure and intent while remaining corrigible.

### Narrative Framing

Narrative framing is presentation.

Examples:

- prose description
- tone
- scene mood
- dialogue phrasing
- summary text
- sensory embellishment

AI can own this layer heavily as long as it does not introduce uncommitted
truth as if it were authoritative.

Good:

```text
"The old bridge feels colder after the blood dries."
```

Risky:

```text
"You realize the guard was the real killer."
```

The second sentence creates a truth claim and should go through a truth layer.

## Authority Modes

`world` can support more than one authority policy.

### Strict Simulation Mode

AI may propose meaning, but it cannot commit truth.

```text
AI:
  proposes semantic suggestions

Typed system:
  validates and commits only rule-derived effects
```

Useful for:

- deterministic replay
- tests
- debug-heavy simulation
- content validation
- competitive or puzzle-like play

### AI Co-Author Mode

AI may commit scoped soft truth and actor truth through a commit gate.

```text
AI:
  proposes or commits soft truth / actor truth

Commit gate:
  checks scope, consistency, authority, and provenance
```

Useful for:

- richer single-player RPG story
- dynamic rumors and backstory
- responsive NPC motivation
- emergent quest hooks
- AI-assisted worldbuilding

The mode should be explicit. The game should know whether AI is only proposing
or actually co-authoring truth.

## Commit Gate

AI-authored truth should pass through a commit gate.

The commit gate should check:

- truth layer: hard, soft, actor, narrative
- authority mode
- contradiction with hard truth
- scope
- referenced entities
- whether new entities or facts are allowed
- whether player-observed truth would be overwritten
- confidence or uncertainty
- provenance
- lifetime or expiration

Example proposal:

```text
AIProposal
  layer: SoftTruth
  proposed_content: Proposition(bandit_1 killed mentor_1 because of old debt)
  links:
    - bandit_1
    - mentor_1
    - old_shrine_debt
  scope: local_story_thread
  source_event_records:
    - ActorDied(mentor_1)
  constraints:
    - must_not_contradict_hard_truth
    - must_link_to_observed_events_or_records
```

Accepted result:

```text
SoftFact
  kind: Motive
  subject: bandit_1
  target: mentor_1
  reason: old_shrine_debt
  source: ai_proposal_456
  committed_at: turn_1043
```

Rejected example:

```text
AIProposal:
  mentor_1 is secretly alive

Rejected:
  contradicts committed ActorDied hard truth
```

## Determinism Versus Auditability

AI co-authority weakens strict determinism.

Strict determinism:

```text
same seed + same actions = same result
```

With AI co-authoring, exact regeneration may be hard or undesirable.

But the game can still preserve auditability:

```text
AI committed fact
  model
  prompt context hash
  input events
  accepted output
  authority layer
  scope
  commit turn
  checker result
```

The important distinction:

```text
not fully deterministic
but logged, inspectable, scoped, and reversible where possible
```

This may be sufficient for a narrative RPG.

## Provenance

Every AI-authored truth should carry provenance.

Useful fields:

```text
source: ai | rule | player | worldgen | rumor | actor
model?
prompt_context_hash?
source_events
source_actor?
confidence
scope
created_at
expires_at?
accepted_by
```

Provenance lets the engine and tools answer:

- where did this fact come from?
- did a rule or AI create it?
- which events justified it?
- is it actor belief or world-level truth?
- can it be contradicted later?

## Relationship To Actor Pressure

AI-authored truth can feed the pressure pipeline.

Example:

```text
Hard Truth:
  mentor_1 died
  bandit_1 caused death

AI Soft Truth:
  bandit_1 killed mentor_1 over old debt

Semantic interpretation:
  player grief/revenge pressure
  merchant fear pressure
  faction suspicion pressure

Actor Truth:
  guard believes the debt implicates the shrine
```

The AI-created soft truth does not directly force behavior. It becomes context
for interpretation, pressure, and intent scoring.

## Relationship To Knowledge And Belief

Actor truth is closely related to knowledge, belief, rumor, and memory.

Important distinction:

```text
SoftTruth:
  there was an old debt between mentor and bandit

ActorTruth:
  merchant believes the debt involved the shrine

Rumor:
  villagers say the mentor betrayed someone years ago

Belief:
  guard believes merchant is hiding the truth
```

AI can create actor truth that is false or incomplete. That can produce
misunderstanding, suspicion, false accusation, and conflicting accounts without
rewriting hard truth.

## Relationship To Semantic PL

AI co-authoring should not replace the semantic PL.

Better:

```text
AI proposes a soft fact, `SocialClaim`, or actor-relative `EpistemicRecord`.
Semantic PL interprets events using that accepted context.
```

Risky:

```text
AI directly says:
  create revenge pressure and arrest intent now
```

AI may propose semantic effects in some modes, but committed effects should
still pass through typed validation and provenance.

## Design Risks

- AI may contradict hard truth.
- AI may create too much hidden backstory.
- AI may make the world feel authored against the player.
- AI may overfit every event into dramatic significance.
- Player-observed facts may be silently retconned.
- Strict replay may become impossible.
- Soft truth may accumulate until the world becomes incoherent.
- Actor truth may be mistaken for world truth.
- Narrative framing may smuggle in uncommitted facts.

## Guardrails

Useful guardrails:

- hard truth cannot be directly changed by AI
- AI-authored facts must declare a truth layer
- soft truth should be scoped
- actor truth can be false but must have a holder
- player-observed hard truth should not be overwritten
- committed AI facts need provenance
- AI suggestions should be inspectable
- narrative text must distinguish fact from atmosphere
- game mode should declare whether AI can commit truth

## Open Questions

- Which truth layers should exist in the first design?
- Should AI ever be allowed to create new hard entities indirectly?
- Can soft truth later become hard truth?
- How should conflicting soft truths resolve?
- How long should AI-created actor truth persist?
- Should players be able to inspect AI-authored provenance?
- Should strict simulation mode and AI co-author mode be save-compatible?
- What kinds of narrative framing are allowed to imply facts?
- Can AI propose semantic rules, or only facts and claims?

## Related References

- [Semantic Kernel And PL Boundary](semantic-kernel-and-pl-boundary.md)
- [Actor Pressure And Interpretation](actor-pressure-and-interpretation.md)
- [Knowledge, History, And Belief](knowledge-history-and-belief.md)
- [Actor Intent And Activity](actor-intent-and-activity.md)
- [Multi-Resolution Simulation](../design/multi-resolution-simulation.md)
