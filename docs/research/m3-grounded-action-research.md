# M3 Grounded-Action Research

## Purpose

This note records the research and repository audit used to enter M3. It asks
one architectural question:

> What is the smallest actor-control boundary that supports human players,
> deterministic NPCs, learned policies, and later AI agents without exposing
> hidden world truth or moving authority out of the runtime?

The answer must fit the completed M2 kernel, preserve the target crate
dependency direction, and leave capability, perception, planning, social
reasoning, authoring, and external adapters free to become sophisticated
inside their own boundaries.

## Result

M3 is best understood as a deterministic compiler:

```text
authoritative snapshot and open action opportunity
  -> actor-relative action projection
  -> bounded fully grounded candidates
  -> candidate-ID-only policy selection
  -> private resolution and lowering
  -> authoritative runtime revalidation
  -> neutral attempt-resolution wake
```

The key distinction is:

```text
candidate
  = complete and meaningful from the actor's permitted view

authoritatively executable
  = legal against current hidden runtime truth
```

Conflating them would make candidate presence, order, score, or diagnostics an
oracle for hidden state. A false or stale actor-relative view may legitimately
produce an attempt that runtime later rejects.

M3 should implement only the action lifecycle. It should not pre-create the
appraisal, social, intent, activity, planning, evidence, or external-agent
frameworks that M4 and M6 will exercise.

## Current repository evidence

M2 already provides the required authority substrate:

- immutable execution semantics and one accepted snapshot per prepared moment;
- complete least-due work preparation;
- opaque pure evaluation;
- typed command preparation and authoritative revalidation;
- deterministic same-moment conflict resolution;
- one atomic authority publication;
- later causal work and bounded progress;
- attempt receipts, recovery fencing, and finalization.

The missing pieces are actor-relative rather than authoritative:

- `world-context` and `world-decision` are not yet workspace members;
- the pre-M4 singleton lifecycle profile has only the explicit disabled
  selection;
- scheduler work has command and post-commit families but no action-ready
  lifecycle work;
- accepted state contains containment and hard transfer authority but no broad
  context or cognition object;
- the public normal controller request still accepts an action definition and
  arbitrary bindings directly;
- no `ReadWitness` vocabulary separates policy-input dependencies from private
  execution-validation dependencies.

This is a favorable boundary. M3 can add projection and selection without
changing the one mutation authority or the engine/runtime decision seam.

## Formal and theoretical foundations

### Partial observability

POMDPs separate world state, observations, an agent's information state, and
policy input. That separation is useful even though M3 does not need
probability distributions, rewards, belief-state planning, or Bellman
optimization. The policy must act on actor-relative input rather than the
underlying state.

- [Kaelbling, Littman, and Cassandra, *Planning and Acting in Partially
  Observable Stochastic Domains*](https://people.csail.mit.edu/lpk/papers/aij98-pomdp.pdf)
- [Halpern and Moses, *Knowledge and Common Knowledge in a Distributed
  Environment*](https://groups.csail.mit.edu/tds/papers/Halpern/JACM90.pdf)

OpenSpiel makes a similar practical distinction between a game state and the
observation or information state available to a player. Its `LegalActions`
surface is useful precedent for supplying a bounded choice set, but this
engine needs a stronger contract: authoritative legality must not change an
actor-visible choice set when the relevant fact is hidden.

- [OpenSpiel concepts](https://github.com/google-deepmind/open_spiel/blob/master/docs/concepts.md)
- [OpenSpiel paper](https://arxiv.org/abs/1908.09453)

### Noninterference is a paired-state property

Noninterference states that activity hidden from one observer cannot change
what that observer can see. It is not adequately tested by inspecting one
output in isolation. It is a relation between executions or states that agree
on permitted inputs.

- [Goguen and Meseguer, *Security Policies and Security
  Models*](https://www.cs.purdue.edu/homes/ninghui/readings/AccessControl/goguen_meseguer_82.pdf)
- [Clarkson and Schneider, *Hyperproperties*](https://ecommons.cornell.edu/items/f161e450-990d-4f74-8dc1-53255e069857)
- [Rushby, *Noninterference, Transitivity, and Channel-Control Security
  Policies*](https://www.csl.sri.com/papers/csl-92-2/)

For M3, the observable surface includes more than serialized values. Candidate
presence, ordering, IDs, fingerprints, coverage, diagnostics, invocation
count, generation, and effective wake timing are all observable behavior and
must be equal across actor-indistinguishable paired states.

### Affordance is relational

Gibson introduced affordances as actionable relations between an environment
and an actor. Norman's later distinction between real and perceived
affordances is particularly important here: what an actor perceives as
possible can differ from what the system can actually perform.

- [Gibson, *The Theory of
  Affordances*](https://monoskop.org/images/c/c6/Gibson_James_J_1977_1979_The_Theory_of_Affordances.pdf)
- [Norman, *Affordance, Conventions and Design*](https://jnd.org/affordance-conventions-and-design-part-2/)
- [Chemero, *An Outline of a Theory of
  Affordances*](https://doi.org/10.1207/S15326969ECO1502_5)

This implies:

- `burnable`, `closed`, or `has-handle` are observed features or
  dispositions, not complete affordances;
- actor capability alone is not a target-specific action;
- a perceived affordance relates an actor, an action schema, a complete
  binding, and actor-visible features;
- a fully grounded candidate is already the smallest public representation of
  that relation.

Capability, repertoire, observed features, and affordance may therefore remain
typed internal derivations until a real policy needs them separately.

### Lifted schema, grounded action, and execution are different stages

STRIPS and PDDL distinguish an action schema from a fully instantiated action
and from the state transition produced when its preconditions hold. That
staging is useful. A universal planning language, closed-world propositional
state, and planner-owned execution are not.

- [Fikes and Nilsson, *STRIPS: A New Approach to the Application of Theorem
  Proving to Problem Solving*](https://doi.org/10.1016/0004-3702(71)90010-5)
- [PDDL 1.2 specification](https://ipc08.icaps-conference.org/deterministic/data/mcdermott-et-al-tr-1998.pdf)

M3 retains three different representations:

```text
ActionDefinition
  lifted roles and authoritative semantic calls

GroundedActionCandidate
  complete actor-safe binding available for selection

CommandEnvelope
  private exact binding submitted for runtime validation
```

They are not aliases and cannot be converted by untrusted policy code.

### AI grounding

SayCan combines language-model semantic preference with a trusted repertoire
of grounded robot skills. The useful lesson is that an AI selects among
supplied capabilities rather than manufacturing executable commands. Its
feasibility score is still not a suitable authoritative legal-action mask for
this engine because hidden truth must remain hidden.

- [Ichter et al., *Do As I Can, Not As I
  Say*](https://proceedings.mlr.press/v205/ichter23a.html)
- [Google Research publication page](https://research.google/pubs/do-as-i-can-not-as-i-say-grounding-language-in-robotic-affordances/)

ReAct, Voyager, and Code as Policies show useful later directions for
deliberation, skill libraries, and AI-generated procedures. They do not justify
allowing an M3 policy to emit code or raw commands.

- [ReAct](https://arxiv.org/abs/2210.03629)
- [Voyager](https://arxiv.org/abs/2305.16291)
- [Code as Policies](https://arxiv.org/abs/2209.07753)

## Game and agent-system evidence

### Deep interaction remains subsystem-local

Caves of Qud demonstrates deep interaction through composable parts, activated
abilities, events, and variable action energy. The architecture lesson is to
allow capabilities and systems to contribute to action grounding without
copying a global synchronous event bus into the cross-system waist.

- [Caves of Qud objects](https://wiki.cavesofqud.com/wiki/Modding%3AObjects)
- [Caves of Qud parts](https://wiki.cavesofqud.com/wiki/Modding%3AParts)
- [Caves of Qud events](https://wiki.cavesofqud.com/wiki/Modding%3AEvents)
- [Caves of Qud turns, segments, and
  actions](https://wiki.cavesofqud.com/wiki/Modding%3ATurns%2C_Segments%2C_and_Actions)

Unreal's Gameplay Ability System similarly treats abilities as actor-owned and
separates them from attributes and effects. Its size, reflective registration,
tag-rich coordination, and ability-owned engine callbacks are not an
appropriate framework to import into M3.

- [Unreal Gameplay Ability System](https://dev.epicgames.com/documentation/en-us/unreal-engine/understanding-the-unreal-engine-gameplay-ability-system)

The retained principle is narrow:

```text
actor-owned state contributes capability
observed target/environment contributes perceived affordance
context grounds a bounded candidate
runtime alone validates and mutates
```

### Observation is not FOV and is not memory

A grid field-of-view algorithm computes potentially visible cells from a point
of view and transparency map. It does not define recognition, memory, belief,
audibility, accessibility, or action grounding.

- [libtcod field of view](https://python-tcod.readthedocs.io/en/11.19.3/tcod/map.html)

Angband separates command handling, derived visibility updates, and remembered
knowledge. Inform separates parser scope and progressively stronger visible,
touchable, and carried requirements from action checks and execution.

- [Angband architecture](https://angband.readthedocs.io/en/stable/hacking/how-it-works.html)
- [Inform action processing](https://ganelson.github.io/inform-website/book/WI_12_2.html)
- [Inform visible, touchable, and carried
  requirements](https://ganelson.github.io/inform-website/book/WI_12_17.html)

M3 should therefore use a real but small relational interaction projection
over the existing containment model. It should not add coordinates, light,
line of sight, sound propagation, or persistent memory before their first
vertical producer and consumer.

### Controller interchange

PettingZoo and OpenSpiel expose per-agent observations and action spaces.
PettingZoo's action masks are useful at an RL adapter, but an M3 adapter must
expose the perceived candidate set rather than a mask derived from hidden
authoritative validity.

- [PettingZoo AEC API](https://pettingzoo.farama.org/main/api/aec/)
- [Huang and Ontañón, invalid action
  masking](https://arxiv.org/abs/2006.14171)

Player UI, deterministic rules, scripts, learned policies, and external agents
must all receive the same action payload. Controller authorization to act for
an actor is a host concern and is distinct from the actor's in-world
capabilities.

### Virtual time

Qud and Angband both demonstrate that an action opportunity is not equivalent
to a fixed turn. Actor speed, action energy, and world updates determine
virtual-time progression.

- [Qud turns, segments, and
  actions](https://wiki.cavesofqud.com/wiki/Modding%3ATurns%2C_Segments%2C_and_Actions)
- [Angband architecture](https://angband.readthedocs.io/en/stable/hacking/how-it-works.html)

M3 does not need a detailed duration formula. It must avoid a Boolean
`consumes_turn` contract and preserve checked virtual-duration semantics for
later capability, condition, equipment, and environment modifiers.

## Adapter and incremental-computation evidence

MCP separates context resources from executable tools and allows dynamic tool
discovery. This confirms that MCP belongs above the engine actor-control API.
The engine should not create one MCP tool per candidate. A later adapter can
keep stable operations such as `get_action_opportunity`,
`submit_action_choice`, and `wait_for_actor_update`, with candidate data
returned inside the opportunity.

- [MCP architecture](https://modelcontextprotocol.io/docs/learn/architecture)
- [MCP server tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)

Salsa and self-adjusting computation show how precise dependency tracking can
reuse results after unrelated changes while retaining from-scratch
consistency. M3 retains that principle as explicit, domain-shaped read
witnesses. It does not add Salsa or a generic incremental-computation
framework while the state and query surface remain small.

- [Salsa algorithm](https://salsa-rs.github.io/salsa/reference/algorithm.html)
- [Hammer et al., *Adapton: Composable, Demand-Driven Incremental
  Computation*](https://arxiv.org/abs/1106.0478)

Capability-security literature also motivates a terminology boundary. An
actor's gameplay capability is derived domain data. Runtime publication
authority is an unforgeable program capability. They must not be represented
or described as the same thing.

- [Miller et al., *Capability Myths
  Demolished*](https://srl.cs.jhu.edu/pubs/SRL2003-02.pdf)

## Formal M3 model

Let:

```text
Γ  exact immutable definitions and execution semantics
Σ  authoritative accepted state
a  actor
u  one open action opportunity
B  primitive actor-permitted view basis
V  projected action view
C  actor-safe grounded candidate set
R  private candidate-resolution table
W  private read witness
```

Actor indistinguishability is defined from the primitive permitted basis, not
from the implementation output:

```text
Σ1 ≈a,u Σ2
  iff canonical(BΓ(a, u, Σ1)) = canonical(BΓ(a, u, Σ2))
```

Context construction is:

```text
BuildActionΓ(Σ, a, u)
  -> Unavailable(actor_safe_reason, W)
   | Complete(payload(V, C), R, W)
```

`Complete(empty)` is not `Unavailable`.

For action definition `d` with typed roles and binding `β`:

```text
CandidateΓ(V, u, d, β)
  iff InScope(u, d)
   ∧ CompleteTypedBinding(d.roles, β)
   ∧ DiscoverΓ(V, d, β)
```

Authoritative executability is a different predicate:

```text
ExecutableΓ(Σ, a, d, β)
  iff PermissionΓ(Σ, a, d, β)
   ∧ RequirementΓ(Σ, d, β)
   ∧ ResourceAvailabilityΓ(Σ, d, β)
   ∧ HardInvariantsΓ(Σ, d, β)
```

Candidate generation must not invoke `Executable`.

Policy is confined to:

```text
PolicyΓ(payload)
  -> Select(candidate_id)
   | NoApplicableAction
   | Wait
   | Abstain
   | Defer
   | Fail
```

Only variants with a real M3 producer and consumer are implemented. Deferred
external evaluation and intent reconsideration remain M4 work even if the
closed result algebra reserves their architectural place.

Private lowering is:

```text
LowerΓ(R, W, selected_id, fresh_snapshot)
  -> CommandEnvelope
   | RebuildOrRebind
   | private integrity failure
```

Runtime independently revalidates the opportunity state, binding shape,
permission, requirements, resources, and current authoritative state.

## Required invariants

### Candidate soundness

- every public candidate has one and only one private resolution entry;
- every declared role is bound with the correct value kind;
- candidates are bounded and canonically ordered;
- policy can select only a supplied ID;
- a candidate ID excludes global revision, raw moment, authority record,
  witness, private metadata, and collection insertion order;
- private attachment cannot remove, add, or reorder public candidates.

### Freshness

- policy-input and execution-validation dependencies occupy different witness
  sections;
- a visible-input change changes the actor-safe input fingerprint and permits
  a new logical invocation;
- an unrelated or hidden-only change may rebuild or rebind privately but
  cannot create a logical policy invocation when the actor-safe payload is
  byte-identical;
- authoritative legality is rechecked on every lowering path.

### Paired-state noninterference

For fixed `Γ`, actor, profile, and opportunity:

```text
Σ1 ≈a,u Σ2
  => canonical(payload1) = canonical(payload2)
```

Equality covers:

- complete/unavailable branch and actor-safe reason;
- candidate presence, ordering, IDs, bindings, meaning, hints, and scores;
- coverage, omission summaries, fingerprints, and diagnostics;
- policy invocation count, identity, generation, and effective timing;
- fallback behavior;
- immediate neutral-wake presence, identity, generation, and effective
  microstep.

The private witness, authority cursor, runtime requirement result, and final
accepted/rejected disposition may differ.

### One-shot progress

- one opportunity is consumed exactly once;
- terminal selection, abstention, failure, and modeled no-action outcomes do
  not reopen it in place;
- retry or continuation creates a causally linked successor under an explicit
  bound;
- rich runtime resolution remains private;
- later actor-visible divergence requires a declared observation and accepted
  evidence transition.

## First concrete grounding family

M3 retains the target dependency graph:

```text
world-context  -> world-core, world-defs, world-model
world-decision -> world-core, world-defs, world-context
world-engine   -> world-core, world-defs, world-model, world-runtime,
                  world-context, world-decision
```

The first projector is a concrete containment-transfer projector in
`world-context`.

It:

- validates the exact actor/item/source/destination role contract;
- reads immutable containment and hard actor/source authority queries;
- uses an explicit bounded interaction scope;
- exposes controlled-source contents and known destination anchors;
- deliberately excludes destination capacity and hidden occupancy from
  candidate membership;
- builds actor-safe object references and a private exact-reference table;
- never imports `world-standard` or matches a standard-pack string.

`world-engine` invokes it only after sealed runtime activation has classified
the checked definition as the typed containment-transfer family. This keeps
the family-specific join at the only layer that already sees runtime
activation, definitions, context, and decision.

A generic discovery DSL or provider registry is deferred until a second
different grounding family supplies evidence for the shared abstraction.

## First vertical scenario

Use the existing standard containment transfer:

1. checked origin input seeds one reaction-sponsored open action opportunity
   and its ready work for each actor;
2. the opportunity supplies bounded source and destination interaction
   anchors;
3. the projector exposes direct items in actor-controlled sources but not
   hidden destination occupancy or capacity;
4. it produces fully bound, canonically ordered transfer candidates and a
   private resolution table;
5. the deterministic baseline selects one supplied candidate ID;
6. engine validates the opportunity and fingerprint, privately lowers the
   selection, and submits the existing command;
7. M2 resolves same-moment contention and publishes once;
8. runtime consumes each opportunity once and schedules the same neutral
   attempt-resolution wake for acceptance and rejection.

Required subcases:

- successful transfer;
- item outside interaction scope produces no candidate;
- guessed entity, definition, binding, or candidate IDs cannot produce a
  command;
- full and non-full hidden destinations produce identical public payloads but
  may produce different private runtime outcomes;
- two same-moment actors select the same item and M2 deterministically resolves
  the conflict;
- `Complete(empty)` yields a modeled no-applicable-action result;
- contract-level `Unavailable` follows a distinct disposition;
- hidden-only revision change preserves payload and logical invocation;
- visible containment or authority change rebuilds and invokes;
- baseline and manual test controllers select through the same ID-only seam.

This is a real local interaction projection. A later spatial/FOV subsystem can
produce the same actor-safe observed-subject and relation inputs without
changing policy, lowering, or runtime.

## Roadmap audit

The milestone order remains correct:

```text
M2 deterministic authority
  -> M3 safe actor-control waist
  -> M4 independently scheduled cognition and agency
  -> M5 restoration and delivery durability
  -> M6 CLI, MCP, AI, and laboratory adapters
  -> M7 multi-resolution simulation
```

Three wording corrections were required:

1. M3 owns the action projection, not every future lifecycle projection.
2. Scenario 2 is split: M3 owns false-belief grounding, private rejection,
   neutral wake, and noninterference; M4 owns observation, evidence, belief,
   and appraisal.
3. M4 extends the M3 opportunity with activity sponsors and higher lifecycles;
   it does not introduce action selection or a second `ActionOpportunity`.

Scenario 4 also needs staged proof. M3 implements and tests the
`Complete(empty)` versus `Unavailable` contract. The first real partial
projection provider completes the end-to-end unavailable path. A fake provider
or fake domain flag would violate vertical-delivery discipline.

## Explicit non-transplants and deferrals

M3 does not add:

- a universal context object, mutable blackboard, or cross-lifecycle runner;
- a capability, affordance, or projection plugin framework;
- a general action, query, planning, or effect DSL;
- a complete Gameplay Ability System analogue;
- grid coordinates, field of view, lighting, acoustics, smell, or recognition;
- memory, belief, evidence assimilation, appraisal, social interpretation,
  intent, activity, or process control;
- GOAP, HTN, behavior trees, utility AI, learned policies, or LLM reasoning;
- detailed action duration modifiers;
- direct AI-generated commands or executable code;
- CLI, MCP, wire schemas, authentication, or transport state;
- checkpoints, archive, external-delivery durability, or multi-resolution
  simulation;
- Salsa or another incremental-computation dependency.

M3 preserves stable seams for those later systems:

- actor-safe immutable payloads;
- bounded candidate sets and stable opportunity/candidate identity;
- private dependency witnesses and exact lowering tables;
- controller-neutral ID-only selection;
- checked virtual-time semantics;
- typed later observation;
- one engine composition root and one runtime mutation authority.
