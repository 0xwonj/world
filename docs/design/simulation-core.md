# Simulation Core

## Status

Frozen pre-target design overview.

This was the map of the design before the target rewrite. Its domain goals
remain useful, but its `ActionRequest`, `CausalTransaction`, `EventRecord`,
replay, and ownership summaries are not current contracts. Use the
[Target Architecture Package](../architecture/target-architecture/README.md)
as the system map and [Design Document Index](README.md) for current routing.

## Design Documents

Historical design map:

- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Pack Authoring And Semantic Declarations](pack-authoring-and-semantic-declarations.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
- [World Model](world-model.md)
- [Physical Simulation Grammar](physical-simulation-grammar.md)
- [Typed Effect Primitives](typed-effect-primitives.md)
- [Standard World Library And Primitive Semantics](standard-world-library.md)
- [Causal Runtime](causal-runtime.md)
- [Action And Event Model](action-event-model.md) terminology sketch
- [Time Model](time-model.md)
- [Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md)
- [Perception And Observation](perception-and-observation.md)
- [Epistemic State](epistemic-state.md)
- [Social Institutional Model](social-institutional-model.md)
- [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md)
- [Intent Templates And Planning](intent-templates-and-planning.md)
- [Multi-Resolution Simulation](multi-resolution-simulation.md)

Design areas intentionally left for later drafts:

- final pack source syntax, package management, migration, and editor tooling
- semantic appraisal rule language and pack-owned vocabularies
- full intent/activity lifecycle implementation and pack-owned intent libraries
- concrete game-system packs and specific game content

## Core Principle

The engine should be simulation-core-first.

Authoritative state, causal mutation, replay, and actor-relative access must be
stable before content, AI assistance, semantic interpretation, or presentation
layers are allowed to become powerful.

The target is not a universal engine for every genre. It is a reusable
simulation foundation for deep, actor-relative, causally inspectable RPG and
sandbox worlds.

The design boundary is:

```text
Core owns mechanism.
Packs own vocabulary.
Game owns content, balance, and premise.
```

```text
hard truth
  -> perception
  -> epistemic state
  -> social / semantic context
  -> semantic appraisal
  -> pressure and GoalPressure
  -> intent
  -> ActionRequest
  -> typed effects
  -> CausalTransaction
  -> EventRecord
```

The loop is circular across turns: committed `EventRecord`s become later
observations, memories, social consequences, pressures, and future action
requests.

## Ownership Map

```text
Engine Core / Game System Boundary:
  reusable core, reusable world-simulation library, game-system packs,
  specific game content, extension discipline

Pack Authoring / Semantic Declarations:
  pack source organization, semantic declaration framework, declaration IR,
  verifier boundaries, source-theme organization, runtime registry mapping

Simulation Transition Compiler:
  ahead-of-time pack checking, runtime contextual projection/analysis/lowering,
  representation/pass taxonomy, representation ladder, pass contracts,
  verifier/runtime validation split, incremental query discipline

Truth / authority boundaries:
  hard truth, soft truth, actor truth, AI authority modes, stage permissions

World Model:
  authority-class store families, relation stores, event history, runtime
  control state, epistemic/appraisal/social/chronology stores, derived views,
  query access boundaries

Physical Simulation Grammar:
  entity, object, topology, containment, material, substance, body, wound,
  condition, equipment, signal, trace, residue, field, passive physical process

Typed Effect Primitives:
  checked primitive effect vocabulary, mutation contracts, mandatory
  EventRecord contracts, forbidden semantic effects

Standard World Library:
  reusable RPG-world grammar, standard primitive definitions, trusted
  primitive semantics, and the boundary between runtime mechanism and
  pack-owned gameplay vocabulary

Causal Runtime:
  ActionRequest lifecycle, Typed Effect Program execution, CausalTransaction,
  EventRecord append, process, reservation, reaction, replay

Action And Event Model:
  short terminology sketch for actions as requests and `EventRecord`s as
  committed hard facts

Time Model:
  scheduler, time cost, duration, windup, continuous process timing, actor turns

Capability / Affordance / Actor Interface:
  CapabilitySet, ActionRepertoire, perceptual capability, PerceivedAffordance,
  actor-facing and AI-agent-facing inputs

Perception:
  current actor-relative ObservedState and ObservedEvent projections

Epistemic State:
  holder-relative EpistemicRecord storage, memory, belief, knowledge, rumor,
  secret, procedure knowledge, retrieval working sets

Social Institutional Model:
  relationships, factions, membership, rank, SocialClaim, norm, law, taboo,
  permission, obligation, oath, debt, reputation, jurisdiction

Semantic Appraisal:
  meaning, `Thought`, `Pressure`, `GoalPressure`, `AppraisalRecordStore`
  semantics, provenance

Intent Planning:
  reusable intent templates, binding, scoring, selected intent, Activity

Multi-Resolution Simulation:
  resolution tiers, intent lowering by resolution, shared ProcessInstance
  execution, resolution-aware location, promotion, demotion, materialization
  constraints
```

## Mutation Path

All hard world mutation must pass through the causal path:

```text
ActionRequest / ProcessTick / ReactionRequest
  -> ActionDef
  -> Typed Effect Program
  -> effect permission checks
  -> CausalTransaction staging
  -> invariant checks
  -> atomic commit
  -> EventRecord append
  -> derived view invalidation
  -> actor observation projection
```

No client, AI agent, semantic rule, event listener, or content script may mutate
hard truth by bypassing this path.

Non-hard gameplay state uses separate commit surfaces:

```text
AcceptedSocialUpdate
  -> SocialInstitutionalStore / soft RelationStore families

AcceptedChronologyRecord
  -> ChronologyStore

AcceptedEpistemicUpdate
  -> EpistemicStore

AcceptedAppraisalRecord
  -> AppraisalRecordStore
```

The [World Model](world-model.md) hosts these stores and query surfaces. The
domain documents own record meaning and lifecycle. The
[Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
document owns which commit surface may write each authority class.

## Resolution Path

Resolution controls execution detail, not truth authority.

```text
Concrete:
  Intent -> Activity -> ActionRequest or ProcessInstance
  ProcessTick can continue long activities

Abstract:
  Intent -> Activity -> ProcessInstance
  ProcessTick advances coarse hard state

Strategic:
  region / faction / world processes
```

All hard outcomes at every resolution still commit through
`CausalTransaction` and `EventRecord`.

The process system is shared. There is no separate `AbstractProcess` system.
A `ProcessInstance` may change active resolution and tick policy, but it keeps
identity, provenance, reservations where still valid, and durable consequences.

## Actor-Facing Path

Actors and AI agents should receive a stable, non-omniscient interface:

```text
actor-owned hard state + accessible actor truth + recognized authority
  -> CapabilitySet, including perceptual capability
  -> ActionRepertoire
  -> PerceptionContext
  -> ObservedState / ObservedEvent
  -> Epistemic WorkingSet
  -> SocialContextView where accessible
  -> PerceivedAffordance
  -> Pressure / GoalPressure / stable Goal when available
  -> AgentTurnInput
  -> AgentTurnOutput
  -> intent or ActionRequest choice
```

The action space belongs to the actor. External objects and places provide
signals, constraints, context, and perceived affordances; they do not create the
actor's owned repertoire.

## Meaning Path

Semantic meaning is derived from `EventRecord`s and actor-relative context. It
is not written directly by physical effects.

```text
EventRecord
  -> ObservedEvent
  -> EpistemicRecord when persisted
  -> SocialContextView
  -> AppraisalRule
  -> Thought / Pressure / GoalPressure
  -> IntentTemplate binding
```

Example:

```text
EventRecord:
  ItemTransferred(shrine_relic, shrine_floor, actor_inventory)

Social context:
  SocialClaim(shrine owns shrine_relic)
  Norm(shrine forbids non-priest removal)

Semantic appraisal:
  possible theft, taboo violation, guard duty pressure
```

## Replay And Auditability

Committed hard outcomes should be auditable and replayable at the level each
subsystem declares.

Baseline audit inputs:

- engine version
- content version
- world seed
- initial scenario
- ordered action/process/reaction log
- committed `EventRecord` history
- RNG stream provenance where needed

Soft truth and non-hard proposed content may be less deterministic, but accepted
game-relevant changes still need provenance and a commit gate.

Full deterministic command replay is a selected debug, test, or subsystem
requirement. It is not the global baseline for every runtime path.

## Historical decisions

- `ActionRequest` is the attempted-change interface.
- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
  separates reusable mechanism from pack-owned vocabulary and specific game
  content.
- [Simulation Transition Compiler](simulation-transition-compiler.md) frames
  the engine as staged projection, analysis, selection, lowering, and
  transactional interpretation from actor-relative context to checked
  simulation transition.
- `Typed Effect Program` is the checked hard-mutation body of an action.
- `CausalTransaction` is the mandatory cause/effect staging boundary before
  hard commit.
- `EventRecord` is the committed fact surface for observation, memory,
  replay, semantic appraisal, and debugging.
- `EventRecord` records hard causal facts; semantic meaning and social
  interpretation use typed soft-truth records.
- [World Model](world-model.md) hosts non-hard gameplay stores such as
  `SocialInstitutionalStore`, `ChronologyStore`, `EpistemicStore`, and
  `AppraisalRecordStore`, but does not own their domain meaning.
- Non-hard records commit through explicit accepted updates:
  `AcceptedSocialUpdate`, `AcceptedChronologyRecord`,
  `AcceptedEpistemicUpdate`, and `AcceptedAppraisalRecord`.
- [Multi-Resolution Simulation](multi-resolution-simulation.md) controls
  execution detail and representation granularity, not truth authority.
- Concrete resolution supports actor `ActionRequest`s; abstract resolution uses
  shared `ProcessInstance` / `ProcessTick` execution instead of hidden
  per-turn concrete actions.
- `Reservation` is runtime conflict-control state, not epistemic state.
- `SocialClaim` is social/institutional state or content, not physical
  possession and not memory by itself.
- Perception and action repertoire are actor-relative and actor-owned.
- Semantic layers may interpret `EventRecord`s but must not directly mutate
  hard truth.
- Named RPG systems such as stats, combat, magic, crafting, economy, appraisal
  vocabularies, and intent libraries should be authored as checked game-system
  packs unless their mechanism belongs in core.

## Deferred Design Areas

- final pack source syntax, package management, migration, and editor tooling
- semantic appraisal rule language and pack-owned vocabularies
- full intent/activity lifecycle implementation and pack-owned intent libraries
- pack manifests, dependency rules, and content packaging
- concrete implementation APIs and data serialization
