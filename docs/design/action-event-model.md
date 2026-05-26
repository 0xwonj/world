# Action And Event Model

## Status

Terminology sketch.

This is not a detailed design owner. It is a short anchor for the core
terminology that older research and idea notes still reference.

Actions are requests. `EventRecord`s are hard facts.

This document intentionally stays short to prevent terminology drift while the
detailed owners carry the real design. The current detailed design is
[Causal Runtime](causal-runtime.md), with primitive effect contracts in
[Typed Effect Primitives](typed-effect-primitives.md) and scheduling details in
[Time Model](time-model.md).

## Action

An action represents what an actor attempts to do.

Examples:

- move north
- wait
- inspect target
- pick up item
- drop item
- use item on target
- talk to actor
- attack target
- open door
- read object

Actions can fail validation.

## EventRecord

`EventRecord` is the reserved name for what actually happened in authoritative
hard causal state. Other layers may keep their own factual records, but they
should not call those records events without qualification.

Examples:

- actor moved
- movement blocked
- item picked up
- attack missed
- attack hit
- actor heard sound
- door opened
- fire started
- process completed

Other records may also be factual inside their own layer, but they are not hard
`EventRecord`s:

- `EpistemicRecord`: what a holder remembers, believes, knows, or heard
- `SocialClaim`: social or institutional assertion about ownership, role,
  permission, or obligation
- semantic/appraisal record: interpreted meaning such as grief, theft
  interpretation, guilt pressure, or revenge pressure

`EventRecord`s are the basis for:

- replay
- logs
- debugging
- observation derivation
- epistemic record creation
- semantic appraisal
- UI messages
- AI context updates

## Resolution Flow

```text
ActionRequest
  -> bind roles and context
  -> validate against authoritative state
  -> run typed effect program
  -> stage CausalTransaction
  -> commit state and event_record_set atomically
  -> derive observations
```

## Rule

Physical and effect rules should not directly produce UI text. They should
produce structured `EventRecord`s. Presentation can translate those records
into text later.

Semantic appraisal rules should produce structured semantic/appraisal records,
not `EventRecord`s and not UI text.

Physical rules should not produce semantic meaning. For example, item transfer
can emit an `EntityTransferred` or `ItemTaken` `EventRecord`, while a later
appraisal layer may interpret the observed transfer as theft, permitted use,
sacrilege, or rescue.

## Question Ownership

Detailed action/event questions are owned elsewhere:

- Failed validation feedback and public failed-attempt records are owned by
  [Causal Runtime](causal-runtime.md).
- Simultaneous and same-time ordering is owned by
  [Time Model](time-model.md) and [Causal Runtime](causal-runtime.md).
- `EventRecord` granularity and contracts are owned by
  [Causal Runtime](causal-runtime.md) and
  [Typed Effect Primitives](typed-effect-primitives.md).
- Append-only histories for soft, actor, social, epistemic, or appraisal
  records are owned by [World Model](world-model.md) and
  [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md).
