# Actor Intent And Activity

## Status

Promoted source history.

## Promotion Note

Stable intent and activity boundary pressure from this idea has been promoted
into
[Intent Templates And Planning](../design/intent-templates-and-planning.md).
This file remains source history.

Some wording below predates the current design boundary. The promoted design
uses one `Intent` concept with resolution-aware lowering, `Activity` as the
temporal execution boundary, and `ProcessInstance` for durable progress. It no
longer treats `AbstractIntent` and `ConcreteIntent` as separate design owners.

## Dependency Chain

This idea builds on:

1. [Action and Event Model](../design/action-event-model.md)
2. Actor Intent And Activity

Action/event rules define how world changes are requested and recorded. Intent
defines how an actor keeps purpose across one or more action requests.

## Core Idea

Actors should be able to hold an intent or activity that explains what they are
trying to do across one or more turns.

Intent does not mutate world state directly. It guides action selection.

```text
Capability = why an actor can attempt something
Intent = what an actor is trying to accomplish
ActionRequest = what the actor attempts now
Event = what actually happened
```

This gives actors continuity without weakening the action/event boundary.

## Motivation

A single-protagonist RPG still needs actors with persistent purposes.

The player may directly choose many atomic actions:

- move
- inspect
- attack
- talk
- pick up
- use
- read
- wait

But many RPG activities are not atomic:

- search a room
- follow a trail
- craft an item
- read a difficult book
- treat a wound
- camp for the night
- prepare a ritual
- escort a traveler
- stalk a wounded enemy
- hold a doorway in combat

NPCs also need continuity. A guard who is patrolling, a merchant traveling, a
beast stalking prey, or a cultist protecting a ritual should not choose every
turn from scratch with no memory of what it was doing.

Intent provides the missing middle layer.

## Model Sketch

```text
ActorState
  + capabilities
  + needs
  + knowledge
  + memory
  + social state
  + current intent/activity
    |
    v
Intent policy
    |
    v
next ActionRequest
    |
    v
validation
    |
    v
resolution
    |
    v
Events
    |
    v
intent update or completion
```

The intent can be chosen by:

- human player command
- scripted behavior
- rule-based NPC policy
- AI agent
- scenario director
- reaction to events
- schedule, duty, need, or social obligation

All of these still resolve through ordinary action requests and events.

## Terms

### Intent

An actor's current purpose.

Examples:

- flank the player
- ask the guard about a rumor
- search the room
- flee to safety
- protect the merchant
- finish reading the book

### Activity

A longer-running or non-combat intent with progress.

Examples:

- travel to market
- prepare ritual
- craft sword
- recover from wound
- patrol gate

The distinction between `Intent` and `Activity` may be mostly vocabulary. A
possible rule:

- use `Intent` for the general model
- use `Activity` for long-running, progress-bearing intents
- use `CombatIntent` for tactical local combat intents when useful

### ActionRequest

The current attempted step.

Examples:

- move east
- inspect tracks
- attack target
- spend turn reading
- apply bandage
- shout warning

### Event

The actual resolved fact.

Examples:

- actor moved
- movement blocked
- tracks inspected
- attack missed
- wound treated
- reading progressed
- actor interrupted

## Intent Shape

A basic intent record might contain:

```text
Intent
  id
  actor
  kind
  goal
  target?
  location?
  progress?
  urgency
  commitment
  created_by
  source_event?
  constraints
  next_action_policy
  interrupt_conditions
  fallback
  visible_tell?
  debug_reason
```

Important fields:

- `kind`: semantic category, such as `TrackTarget`, `HoldDoorway`, or
  `ReadObject`.
- `goal`: what success means.
- `progress`: optional, for multi-turn activities.
- `urgency`: how strongly this competes with other intents.
- `commitment`: how resistant this is to interruption or replanning.
- `created_by`: player, NPC policy, AI agent, director, reaction, schedule.
- `source_event`: event that caused this intent, if any.
- `interrupt_conditions`: what cancels or changes the intent.
- `fallback`: what to try if the intent cannot continue.
- `visible_tell`: what an observer may perceive before or during execution.
- `debug_reason`: structured explanation for tools and AI context.

## Lifecycle

Intent should have an explicit lifecycle.

```text
Created
  -> Active
  -> Advancing
  -> Completed
  -> Interrupted
  -> Abandoned
  -> Failed
  -> Replaced
```

Examples:

```text
Created
  player chooses "search room"

Active
  actor has SearchRoom intent

Advancing
  actor spends turns inspecting containers and floor

Completed
  hidden key found or room fully searched

Interrupted
  enemy enters the room

Abandoned
  player chooses to leave

Failed
  actor lacks light or necessary sense
```

Lifecycle changes should emit structured events when they matter:

```text
IntentStarted
IntentProgressed
IntentInterrupted
IntentCompleted
IntentFailed
IntentAbandoned
```

Not every internal replanning detail needs to become a public event, but
debugging and replay should be able to explain it.

## Action Boundary

Intent must not bypass action validation.

Bad:

```text
Intent: PickLock
  directly changes door.locked = false
```

Good:

```text
Intent: PickLock
  -> ActionRequest: UseToolOnLock
  -> validation checks actor, tool, lock, noise risk
  -> Events: LockpickAttempted, LockpickSucceeded, DoorUnlocked, NoiseEmitted
```

Intent answers:

```text
Why is this actor trying this?
What is the actor trying to accomplish across turns?
What should the actor try next?
When should the actor stop?
```

ActionRequest answers:

```text
What is the actor attempting now?
```

Event answers:

```text
What actually happened?
```

## Concrete Intent

Concrete intent operates in the local simulation layer.

It can see concrete state available to the actor's decision policy:

- position
- visible entities
- reachable targets
- known hazards
- current body state
- current needs or fear
- actor-owned action repertoire
- perceived affordances
- nearby allies and enemies

Concrete intent produces one or more action requests over time.

Example:

```text
ConcreteIntent: HoldDoorway
  actor: guard
  goal: prevent player from entering shrine
  target_location: shrine_door
  next_action_policy:
    - move to doorway if not there
    - attack intruder if adjacent
    - shout alarm if outnumbered
  interrupt_conditions:
    - morale_broken
    - shrine_burns
    - superior_orders_retreat
```

Possible action sequence:

```text
MoveToDoorway
WaitInDefensiveStance
AttackIntruder
ShoutAlarm
```

## Combat Use

Intent is especially useful in combat because it separates tactical purpose
from atomic action.

Without intent:

```text
if adjacent_to_player:
  attack
else:
  move_toward_player
```

With intent:

```text
CombatIntent: FlankPlayer
  turn 1: move east
  turn 2: move southeast
  turn 3: wait behind cover
  turn 4: attack from side
```

Useful combat intents:

- `FlankTarget`
- `HoldDoorway`
- `MaintainRange`
- `ProtectAlly`
- `ProtectCaster`
- `RetreatToCover`
- `LureIntoTrap`
- `SuppressTarget`
- `InterruptRitual`
- `FinishWoundedTarget`
- `BreakLineOfSight`
- `CallForHelp`

### Telegraphing

Intent can create visible tells before dangerous actions.

```text
CombatIntent: HeavyStrike
  visible_tell: raises axe
  next_action_delay: 1 turn
```

Events:

```text
AttackTelegraphed
HeavyAttackResolved
```

This lets players react by dodging, blocking, stunning, retreating, or
interrupting.

### Interruptible Plans

Many combat intents should be interruptible.

```text
CombatIntent: CastRitual
  progress: 2 / 4
  interrupt_conditions:
    - takes_damage
    - loses_line_of_sight
    - silenced
    - pushed_out_of_circle
```

This makes non-damage actions strategically meaningful.

### Roles

Different actors can share the same action system but choose different intents.

```text
Guard
  HoldDoorway
  CallForHelp
  ProtectCivilian

Beast
  StalkWoundedTarget
  FleeFire
  CircleInDarkness

Cultist
  ProtectCaster
  DelayIntruder
  CompleteRitual

Bandit
  Ambush
  StealAndFlee
  RetreatWhenInjured
```

This makes enemy type visible through behavior, not only stats.

### Group Tactics

Group tactics can assign or bias individual intents.

```text
GroupIntent: SurroundPlayer
  bandit_1: BlockNorth
  bandit_2: FlankEast
  archer_1: MaintainRange
```

This should still resolve through each actor's own action requests.

## Non-Combat Use

### Search

```text
Intent: SearchRoom
  target_area: shrine_back_room
  progress: 0 / 3
  next_action_policy:
    - inspect floor
    - inspect containers
    - inspect suspicious wall
```

Events:

```text
SearchProgressed
HiddenCompartmentFound
SearchInterrupted
```

### Reading

```text
Intent: ReadObject
  target: cracked_tablet
  requirements:
    - sufficient_light
    - known_language_or_translation
  progress: 1 / 5
```

Events:

```text
ReadingProgressed
KnowledgeLearned
ReadingFailedLanguageUnknown
```

### Tracking

```text
Intent: FollowTrail
  target: wounded_bandit
  known_trace: blood_tracks
  progress: trail_confidence
```

Events:

```text
TrackInspected
TrailFollowed
TrailLost
TargetLocated
```

### Treatment

```text
Intent: TreatWound
  target: self
  tool: clean_bandage
  progress: bandaging
  interrupt_conditions:
    - attacked
    - tool_lost
    - unconscious
```

Events:

```text
TreatmentStarted
BandageApplied
BleedingReduced
TreatmentInterrupted
```

### Travel

```text
Intent: TravelToKnownSite
  destination: old_mill
  route: north_road
  progress: 0.4
```

Local travel may produce movement actions over multiple turns.

## Player-Facing Use

The protagonist can use intent without turning the game into an automation
manager.

Player-facing examples:

- "search this room"
- "read until interrupted"
- "follow these tracks"
- "rest until dawn or danger"
- "travel to known shrine"
- "keep aiming"
- "hold position"

These should be optional conveniences or explicit activity choices. The core
feel should still allow direct turn-by-turn action.

The player should be able to interrupt their own intent at any time unless a
specific condition prevents control, such as panic, sleep, paralysis, or a
mental state.

## NPC And Agent Use

NPCs need intent to avoid feeling like random action emitters.

An NPC decision loop might be:

```text
if current_intent can continue:
  produce next ActionRequest
else:
  choose intent from duties, needs, knowledge, memory, actor-owned repertoire,
  and perceived context
```

AI agents may choose intent rather than atomic action when useful:

```text
AgentTurnOutput
  intent: QuestionGuardAboutRumor
  speech: "Have you seen a wounded rider pass through?"
```

The engine can then validate whether that intent is plausible and translate it
into action requests over turns.

Agent input should include:

```text
current_intent
intent_progress
candidate_intents
unavailable_intents_with_reasons
interruptions
visible_tells
active_needs
relevant_memories
known_social_constraints
action_repertoire
perceived_affordances
```

This gives agents meaningful decisions without giving them omniscient truth.

## Observation And Debugging

Intent should be partially observable.

Human players and AI agents may see:

- an enemy raising a weapon
- a guard blocking a doorway
- a merchant packing to leave
- a cultist chanting
- a beast circling in darkness
- a wounded enemy fleeing north

These are observations of evidence, not direct access to hidden intent state.

Debug tools can inspect more:

```text
actor: guard_1
current_intent: HoldDoorway
reason:
  duty: shrine_guard
  threat: player_trespassing
  morale: steady
next_action: ShoutWarning
fallback: CallForHelp
```

This is important for testing, replay, and AI-agent explanation.

## State Implications

Intent state may need:

- actor id
- intent id
- intent kind
- target references
- progress
- urgency and commitment
- source event
- visible tells
- interruption state
- fallback state
- deterministic random choices used by the intent

Intent should be serializable and replayable.

If an intent is derived and not stored, it must still be explainable from stored
state. Long-running activities probably need stored progress.

## Event Implications

Events should preserve meaningful intent transitions.

Examples:

```text
IntentStarted
IntentAdvanced
IntentInterrupted
IntentCompleted
IntentFailed
IntentAbandoned
IntentReplaced
```

Combat examples:

```text
AttackTelegraphed
FlankPositionReached
RitualCastingInterrupted
CallForHelpHeard
```

Non-combat examples:

```text
SearchProgressed
ReadingProgressed
TrailFollowed
TreatmentCompleted
CampInterrupted
```

These events can drive replay, debugging, memory, observation, and AI context.

## Relationship To Actor-Owned Capability-Derived Actions

Capability answers which action schemas an actor owns and can attempt in
principle.

Intent answers what the actor wants to accomplish.

Example:

```text
Capability:
  actor owns ApplyTool because they can manipulate tools
  actor has lockpicking skill and carries a lockpick

Perceived affordance:
  bronze gate appears locked

Intent:
  actor wants to enter the sealed room quietly

ActionRequest:
  actor attempts ApplyTool(lockpick, bronze_gate, pick_lock)

Events:
  LockpickAttempted
  LockpickSucceeded
  DoorUnlocked
  NoiseEmitted
```

An intent may be blocked if required capabilities are missing. Missing
capabilities should be explainable:

```text
UnavailableIntent:
  PickLock
  reason:
    - no lockpick
    - hands injured
```

## Design Risks

- If intent mutates state directly, the action/event boundary collapses.
- If intent is too hidden, players cannot make strategic decisions.
- If intent is too explicit, enemies may feel mechanical or predictable.
- If every action creates an intent, the model becomes noisy.
- If intent persists too strongly, actors feel stubborn or stupid.
- If intent is replanned every turn, it loses continuity.
- If player-facing intent automates too much, the single-protagonist RPG feel
  weakens.
- If AI agents choose broad intents without validation, they may bypass rules.
- If interrupt conditions are vague, long activities become hard to debug.

## Open Questions

- Should `Intent` and `Activity` be separate types or one model with variants?
- What is the smallest useful intent lifecycle for the first implementation?
- Which player actions should create explicit activities?
- How much of enemy intent should be visible through tells?
- How should simultaneous intent conflicts resolve?
- Can group tactics be represented as shared intent, or should they only bias
  individual intents?
- Should intent transition events be part of the canonical replay log?
- How should intent interact with actor needs, memories, beliefs, and social
  obligations?
- How does an AI agent choose between atomic action and higher-level intent?

## Related References

- [Action and Event Model](../design/action-event-model.md)
- [Intent Templates And Planning](../design/intent-templates-and-planning.md)
- [Actor-Owned Capability-Derived Actions](capability-derived-actions.md)
- [RimWorld](../references/rimworld.md)
- [Caves of Qud](../references/caves-of-qud.md)
- [Agent Interface](../brainstorming/agent-interface.md)
