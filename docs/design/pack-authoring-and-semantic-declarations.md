# Pack Authoring And Semantic Declarations

## Status

Current design draft.

This document defines the pack authoring, semantic declaration, and
verification boundary. It is not a final source syntax, parser design, package
manager, editor format, or implementation plan.

## Related Design Owners

- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
- [World Model](world-model.md)
- [Typed Effect Primitives](typed-effect-primitives.md)
- [Standard World Library And Primitive Semantics](standard-world-library.md)
- [Causal Runtime](causal-runtime.md)
- [Social Institutional Model](social-institutional-model.md)
- [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md)
- [Intent Templates And Planning](intent-templates-and-planning.md)
- [Multi-Resolution Simulation](multi-resolution-simulation.md)

## Purpose

Packs are how game-specific vocabulary enters the engine without gaining
arbitrary authority.

They should let a game define things such as shrine law, ritual magic,
tracking, wounds, social rank, reputation, faction duties, appraisal
vocabularies, and intent libraries while preserving the core simulation
boundaries:

```text
Core owns mechanism.
Packs own vocabulary and checked declarations.
Game content instantiates pack vocabulary.
```

This document answers:

```text
How should authored pack declarations become verified engine definitions?
Which authoring surfaces are real DSLs or IRs?
Which declarations share one semantic authoring framework?
How are semantic declarations used by different runtime pipeline stages?
What must the verifier reject before a pack can run?
```

## Core Decision

Use two top-level IR families with replaceable authoring frontends:

```text
Typed Effect Program IR:
  hard-mutation effect program for actions, processes, reactions, and checked
  physical runtime work.

Semantic Declaration IR:
  one shared semantic declaration framework for social rules, appraisal rules,
  intent templates, and semantic views.
```

The split is authority-based.

`Typed Effect Program` is the only pack-authored low-level IR family that may
describe hard mutation, and it still only stages mutation through
`CausalTransaction`.

`Semantic Declaration IR` cannot mutate hard truth. It can match
actor-relative context, bind variables, emit typed non-hard proposals, create
appraisal output, and generate decision candidates according to declaration
kind.

The first authoring frontend may be text, structured data, editor-authored
objects, or several equivalent formats. The stable contract is the checked IR,
verifier behavior, and runtime registry mapping.

## Not One Universal DSL

The engine should not have one unrestricted language that can express
everything:

```text
UniversalWorldDSL:
  physical effects
  social law
  appraisal
  intent choice
  memory writes
  process ticks
  AI proposals
  transaction commit
```

That would hide authority boundaries inside a large language and make it too
easy for a rule to cross stages accidentally.

Bad outcomes:

- a social rule rewrites an `EventRecord`
- an appraisal rule selects final `Intent`
- an intent template emits a `CausalTransaction`
- a semantic rule reads omniscient hard truth instead of actor-relative views
- natural-language content becomes gameplay authority without typing or
  provenance

## Not Four Separate Semantic DSLs

The engine also should not define unrelated custom languages for each semantic
domain:

```text
SocialRuleLanguage
AppraisalRuleLanguage
IntentTemplateLanguage
SemanticViewLanguage
```

Those domains share too much machinery:

- namespaces and imports
- typed references
- actor-relative query inputs
- pattern matching
- variable binding
- conditions
- output templates
- provenance
- diagnostics
- migration/version metadata
- pack ownership and extension rules

Separate languages would duplicate parser, verifier, error reporting,
tooling, and migration work while making a single gameplay theme harder to
author.

## One Semantic Framework, Many Declaration Kinds

Use one semantic declaration framework with multiple checked declaration kinds.

```text
Semantic Declaration IR / framework
  social_rule
  appraisal_rule
  intent_template
  semantic_view
```

These kinds share source structure, type infrastructure, matching, binding,
and diagnostics. They do not share authority.

```text
social_rule:
  reads social context, hard event evidence, epistemic context where allowed
  emits social interpretations or social update proposals
  forbidden: hard mutation, direct appraisal pressure, final intent

appraisal_rule:
  reads observed events, epistemic working sets, social context, actor context
  emits Thought, Pressure, GoalPressure, appraisal record proposals
  forbidden: hard mutation, direct memory/social writes, final intent

intent_template:
  reads Pressure, GoalPressure, Goal, CapabilitySet, ActionRepertoire,
  PerceivedAffordance, epistemic/social context, active resolution
  emits CandidateIntent, IntentScore features, lowering contracts,
  Activity preparation metadata
  forbidden: hard mutation, CausalTransaction, EventRecord, direct effects

semantic_view:
  reads declared context inputs
  emits derived actor-relative context for later semantic stages
  forbidden: durable writes unless routed through a declared commit gate
```

## Source Organization

Authoring should be organized by pack, theme, or extension, not by runtime
pipeline stage.

Poor authoring shape:

```text
all_social_rules.semantic
all_appraisal_rules.semantic
all_intent_templates.semantic
```

Better authoring shape:

```text
packs/shrine-law/
  pack.toml
  vocabulary.semantic
  relic.semantic
  priest-authority.semantic
  relic-actions.effect
  purification-process.effect

packs/banditry/
  pack.toml
  violence.semantic
  intimidation.semantic
  pursuit.semantic

packs/wilderness-survival/
  pack.toml
  hunger.semantic
  injury.semantic
  shelter.semantic
```

One semantic source file may contain several declaration kinds when they belong
to the same gameplay theme:

```text
social_rule relic_removed_from_shrine
appraisal guard_sees_relic_removed
intent_template recover_relic
intent_template report_to_priest
```

Hard mutation definitions and effect programs should stay in the typed effect
authoring family:

```text
action perform_purification
process purify_relic
effect_program purify_relic_effects
```

## Runtime Organization

Runtime lookup is organized by declaration kind, pipeline stage, and trigger.

Authoring layout:

```text
pack/theme/module
```

Compiled runtime layout:

```text
DefinitionRegistry
  social_rules_by_trigger
  appraisal_rules_by_focus
  intent_templates_by_pressure_or_goal
  semantic_views_by_input
  action_defs_by_schema
  process_defs_by_kind
  effect_programs_by_definition
```

The source organization is human-facing. The registry organization is
compiler/runtime-facing.

## Authoring Pipeline

Pack authoring is compiled before runtime use:

```text
pack source
  -> parse / load source syntax
  -> declaration AST
  -> typed declaration IR
  -> symbol resolution
  -> type checking
  -> stage permission checking
  -> domain-specific verification
  -> DefinitionRegistry
```

Runtime stages consume only the registry entries they own:

```text
Social Context stage:
  reads social_rule and semantic_view declarations
  emits SocialContextView contributions or social proposals

Appraisal stage:
  reads appraisal_rule declarations
  consumes ObservedEvent, EpistemicWorkingSet, SocialContextView
  emits Thought, Pressure, GoalPressure

Intent stage:
  reads intent_template declarations
  consumes Pressure, GoalPressure, CapabilitySet, PerceivedAffordance
  emits CandidateIntent and IntentScore features

Effect execution stage:
  reads Typed Effect Program instances
  stages hard mutation through CausalTransaction
```

Declarations not relevant to the current stage do not run. They remain in the
verified registry until their owning stage queries them.

## Semantic Declaration IR

The semantic source syntax may evolve. The stable target is the semantic
declaration IR.

Conceptual shape:

```text
SemanticDeclarationIR
  id
  namespace
  kind
  source_span
  imports
  declared_reads
  trigger_patterns
  required_inputs
  bindings
  conditions
  outputs
  score_features?
  lowering_contract?
  provenance_policy
  authority_policy
  diagnostics_policy
  invalidation_dependencies
```

`kind` controls verification and runtime ownership:

```text
kind = social_rule | appraisal_rule | intent_template | semantic_view
```

The verifier must reject outputs that the declaration kind cannot own.

## Typed Effect Program Boundary

`Typed Effect Program` is separate from the semantic declaration framework.

It is closer to an effect IR than a semantic rule language:

```text
ActionDef / ProcessDef / ReactionDef
  -> Typed Effect Program instance
  -> CausalTransaction staging
  -> invariant checks
  -> EventRecord append
```

It may use checked hard-effect primitives such as:

```text
transfer_entity
set_lock_state
apply_damage
emit_signal
schedule_process
cancel_process
```

It must not emit semantic meaning:

```text
forbidden in Typed Effect Program:
  Thought(...)
  Pressure(...)
  GoalPressure(...)
  CandidateIntent(...)
  SocialClaim(...) unless routed through the social commit gate
  DeclareTheft(...)
  SetMood(...)
```

Semantic interpretation happens after hard facts are committed and projected
into actor-relative context.

## Declaration Kinds

### Social Rule

`social_rule` declarations interpret hard evidence and social context under a
scope.

Allowed reads:

- `EventRecord` references where accessible
- `ObservedEvent`
- `EpistemicWorkingSet` where holder-relative access is declared
- existing `SocialClaim`, norm, law, taboo, permission, obligation, rank,
  office, jurisdiction, and scope records

Allowed outputs:

- social interpretation candidates
- `AcceptedSocialUpdate` proposals
- `SocialContextView` contributions

Forbidden outputs:

- hard mutation
- `EventRecord` rewrite
- `Thought`, `Pressure`, or `GoalPressure` unless routed through appraisal
- `Intent`
- `ActionRequest`

### Appraisal Rule

`appraisal_rule` declarations turn actor-relative context into meaning and
motivational pressure.

Allowed reads:

- `ObservedEvent`
- `ObservedState`
- `EpistemicWorkingSet`
- `SocialContextView`
- actor context
- visible current `Intent` / `Activity` context

Allowed outputs:

- `Thought`
- `Pressure`
- `GoalPressure`
- appraisal record proposals
- optional social/epistemic update proposals routed through their own gates

Forbidden outputs:

- final `Intent`
- `CandidateIntent`
- `ActionRequest`
- `ProcessTick`
- `CausalTransaction`
- direct `EpistemicStore` or `SocialInstitutionalStore` writes

### Intent Template

`intent_template` declarations generate possible commitments and scoring
features.

Allowed reads:

- `Pressure`
- `GoalPressure`
- stable `Goal`
- `CapabilitySet`
- `ActionRepertoire`
- `PerceivedAffordance`
- `EpistemicWorkingSet`
- `SocialContextView`
- active `Intent`, `Activity`, and `ProcessInstance`
- active resolution

Allowed outputs:

- `CandidateIntent`
- `IntentScore` features
- selected-lowering support metadata
- `Activity` preparation metadata

Forbidden outputs:

- hard mutation
- `CausalTransaction`
- `EventRecord`
- direct `Thought`, `Pressure`, or `GoalPressure`
- direct final intent selection unless routed through the intent selection gate

### Semantic View

`semantic_view` declarations define reusable derived actor-relative context.

Allowed reads and outputs depend on the view owner, but all views must declare:

- input representation classes
- output representation class
- access filtering
- invalidation dependencies
- provenance policy

Semantic views are not a general write path. Durable state still requires the
appropriate commit gate.

## Example: Shrine Relic Pack

Illustrative source syntax:

```text
pack world.shrine_law:
  imports:
    world.core
    world.social
    world.appraisal
    world.intent

social_rule relic_removed_from_shrine:
  when observed EntityTransferred(object, from, to)
  where object has kind ShrineRelic
  where social_claim(shrine_order owns object)
  where norm(shrine_law forbids removal by non_priest)
  bind remover = actor_holding(object)

  interpret TabooViolation(
    actor: remover,
    object: object,
    institution: shrine_order,
    basis: observed.event
  )

  propose_social_update ViolationRecorded(
    actor: remover,
    institution: shrine_order,
    severity: high,
    provenance: observed.event
  )

appraisal guard_sees_relic_removed:
  when social_interpretation TabooViolation(actor, object, institution)
  where holder has office ShrineGuard(institution)

  emit Thought(SawTabooViolation, subject: object)
  emit Pressure(EnforceShrineLaw, target: actor)
  emit GoalPressure(RestoreRelicToShrine, object: object)

intent_template recover_shrine_relic:
  when goal_pressure RestoreRelicToShrine(object)
  where perceived_affordance actor_holding(object)
  where actor.repertoire can ApproachActor
  where actor.repertoire can RequestTransfer or DetainActor

  candidate RecoverObject(object: object, claimant: shrine_order)

  score:
    + goal_pressure.urgency * 40
    + office_duty(holder, shrine_order) * 30
    - visible_threat(actor) * 20

  lowers concrete:
    Activity(RecoveringShrineRelic)
      -> ActionRequest(ApproachActor(actor))
      -> ActionRequest(RequestTransfer(object))

  lowers abstract:
    Activity(RecoveringShrineRelic)
      -> ProcessInstance(RecoverObject)
```

This pack source is organized by theme. The compiler registers its declarations
by runtime stage:

```text
social_rules_by_trigger:
  EntityTransferred -> relic_removed_from_shrine

appraisal_rules_by_focus:
  TabooViolation -> guard_sees_relic_removed

intent_templates_by_goal_pressure:
  RestoreRelicToShrine -> recover_shrine_relic
```

## Example: Mentor Killed By Bandit

Illustrative source syntax:

```text
appraisal grief_and_retaliation:
  when observed ActorDied(victim)
  where holder.relationship(victim).kind in [mentor, kin, oath_guardian]
  where holder.believes(cause_actor caused victim.death)
  bind killer = cause_actor

  emit Thought(GriefAboutDeath, subject: victim)
  emit Pressure(Retaliate, target: killer, intensity: high)
  emit GoalPressure(FindOrConfrontActor, target: killer, urgency: high)

intent_template track_responsible_actor:
  when goal_pressure FindOrConfrontActor(target)
  where perceived_affordance TraceVisible(trace)
  where trace.likely_source == target
  where actor.repertoire can Inspect
  where actor.repertoire can Move

  candidate TrackPhysicalTrace(target: target, trace: trace)

  score:
    + pressure.urgency * 40
    + trace.freshness * 20
    - actor.fatigue * 10

  lowers concrete:
    Activity(TrackingActor)
      -> ActionRequest(Inspect(trace))
      -> ActionRequest(MoveAlongTrace(trace))

  lowers abstract:
    Activity(TrackingActor)
      -> ProcessInstance(TrackActor)
```

The appraisal declaration does not select the final intent. The intent
template does not execute hard effects. Each declaration is interpreted by its
own stage.

## Verification

The verifier checks pack declarations before runtime use.

Common checks:

- namespace and imported symbols exist
- referenced declaration kinds are visible
- typed roles and arguments match
- query inputs are legal for the declaration kind
- output templates are legal for the declaration kind
- required provenance is declared
- authority policy is explicit
- invalidation dependencies are declared where a derived view is cached
- supported resolutions are declared where lowering is possible
- pack dependencies and extension points are explicit

Hard-effect checks:

- `Typed Effect Program` uses only allowed primitive effects
- primitive calls resolve to installed standard or trusted extension
  definitions
- hard mutation is staged through `CausalTransaction`
- required `EventRecord` contracts are emitted
- semantic/social/appraisal outputs are not emitted as hard effects

Semantic declaration checks:

- `social_rule` cannot emit hard mutation or appraisal pressure directly
- `appraisal_rule` cannot select final `Intent`
- `intent_template` cannot emit `CausalTransaction`
- `semantic_view` cannot leak hidden truth across actor access boundaries
- AI-authored declarations require explicit proposal/review provenance before
  becoming accepted pack definitions

## Diagnostics

Diagnostics should name the violated boundary, not only the syntax failure.

Examples:

```text
error:
  appraisal_rule grief_and_retaliation emits CandidateIntent.

reason:
  Appraisal declarations may emit Thought, Pressure, and GoalPressure.
  CandidateIntent is owned by intent_template declarations.
```

```text
error:
  intent_template recover_shrine_relic lowers abstract execution to repeated
  ActionRequest(MoveStep).

reason:
  Abstract execution must lower through ProcessInstance, not hidden concrete
  action spam.
```

```text
error:
  effect_program take_relic emits TabooViolation.

reason:
  Typed Effect Program may emit hard EventRecord facts. Social meaning belongs
  to social_rule declarations.
```

## AI And Tooling Boundary

AI may help author packs, propose rules, summarize diagnostics, or suggest
vocabulary. AI output is not authoritative.

Allowed:

- propose a semantic declaration
- propose an effect program
- explain why a declaration failed verification
- suggest missing provenance or imports
- generate example pack content for review

Forbidden:

- silently install AI-generated pack definitions as accepted gameplay rules
- bypass the verifier
- use natural-language prose as gameplay authority when typed declarations are
  required
- grant AI direct mutation authority over hard, social, epistemic, appraisal,
  or intent state

## Relationship To Runtime Passes

Semantic declarations are not executed as one monolithic script.

```text
pack source
  -> SemanticDeclarationIR
  -> DefinitionRegistry
  -> stage-specific lookup
```

Runtime:

```text
Social Context stage:
  registry.social_rules_for(trigger)

Appraisal stage:
  registry.appraisal_rules_for(focus)

Intent stage:
  registry.intent_templates_for(goal_pressure)
```

The same source pack can contain declarations for all three stages. Runtime
passes only interpret the subset they own.

## Process Definition IR

`ProcessDefinitionIR` is a checked runtime definition family. It is not a
semantic declaration and it is not final source syntax.

Minimum concept fields:

```text
ProcessDefinitionIR
  process state schema
  tick entrypoint
  wait policy
  interrupt policy
  resume policy
  failure policy
  supported resolutions
  allowed Typed Effect Program references
  resolution lowering contract
```

Process source authoring may later use a text DSL, structured data, or an
editor format. The stable early decision is that process definitions compile
to checked IR and execute through `ProcessRuntime`, `ProcessTick`, and
`CausalRuntime`.

## Stable Decisions

- `Typed Effect Program` is a separate hard-mutation IR family with
  replaceable authoring frontends.
- Ordinary packs compose installed primitives; trusted primitive semantics are
  supplied by the standard world library or trusted engine extensions, not by
  arbitrary pack callbacks.
- Semantic, social, appraisal, and intent declarations share one semantic
  declaration IR framework.
- The semantic framework has multiple declaration kinds, not one unrestricted
  universal rule.
- `ProcessDefinitionIR` is a checked runtime definition family for durable
  process execution.
- Pack source is organized by theme, vocabulary, or extension.
- Runtime registries are organized by declaration kind, stage, and trigger.
- The stable target is typed declaration IR and verifier behavior, not final
  surface syntax.
- Semantic declarations cannot bypass `CausalTransaction`, non-hard commit
  gates, actor-relative access filtering, or intent selection gates.
- AI may propose declarations but cannot make them authoritative without
  verification and acceptance.

## Deferred Decisions

- final pack manifest format
- final source syntax and file extensions
- whether the first source syntax is text, structured data, editor-authored
  objects, or several equivalent frontends
- exact semantic declaration IR serialization
- exact condition and binding expression language
- exact query language for actor-relative views
- exact pack dependency, override, and conflict policy
- exact migration/versioning model
- exact diagnostics format and editor integration
- final source syntax for authoring process definitions
