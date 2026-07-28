# Gameplay Composition And Evolution Research

## Status

This document is a research input for the target architecture. It does not
silently amend the normative package under
`docs/architecture/target-architecture/`. It records the evidence, formal
model, target scenarios, repository gaps, and the acceptance evidence used to
decide when the gameplay extension boundary may be frozen.

That review has now adopted the three-slice falsification suite as M8 while
leaving unproven mechanisms, such as a universal semantic dependency graph,
evidence-gated. The authoritative dispositions live in
[Extensibility And Research](../architecture/target-architecture/extensibility-and-research.md#gameplay-composition-research-disposition);
milestone ownership and gates live in the
[Implementation Roadmap](../architecture/target-architecture/implementation-roadmap.md).

The question is not merely whether another mechanic can be implemented. It is:

> What architecture lets independently developed physical, social, cognitive,
> temporal, and authored mechanics produce deep interactions without hidden
> mutation paths, global ordering dependencies, or repeated changes to the
> authority kernel?

## Executive result

The research supports the current target direction, but gives its intended end
state a sharper form:

> `world` should be a heterogeneous semantic microkernel: one deterministic,
> transactional, discrete-event authority model coordinating several
> domain-specific models of computation.

The cross-system law remains small:

```text
immutable execution semantics Γ
  + one authoritative session state
  + capability-scoped immutable typed input
  -> bounded proposal or selected supplied ID
  -> verified atomic authority transition
  -> later typed causal work
```

The engine should not adopt a global ECS, universal rule language, mutable
event bus, or unrestricted scripting layer as its semantic center. Each solves
one useful problem, but none supplies authority, actor-relative knowledge,
temporal persistence, conflict semantics, replay, and multi-resolution
refinement together.

The desired structure is:

```text
              immutable execution environment Γ for one session epoch
                                      |
              exact implementations, schemas, definitions, policies
                                      |
                                      v
owned typed state -> pure typed derivations -> actor-relative projections
       ^                                      |
       |                                      v
atomic commit <- prepared compatible deltas <- bounded proposals
       |
       +-> authority records -> observations / interpretations / later wakes
```

Subsystems remain free to use the model that fits them:

- dense component storage for local physical simulation;
- typed relational views for capability and affordance derivation;
- explicit state machines for activities and processes;
- BDI, utility, HTN, GOAP, rules, or AI for agency;
- evidence and justification graphs for bounded epistemics;
- institutions, practices, claims, and obligations for social interpretation;
- aggregate equations or sparse tables for distant simulation.

They compose through typed state ownership, declared views, proposals,
transactions, authority records, domain events, and scheduled work. They do
not acquire ambient mutation authority or synchronously call one another's
handlers.

This architecture can enforce tier-relative locality, ownership, deterministic
composition, causal explainability, replay, and exact semantic identity. It
cannot guarantee that a game will be deep or fun. Combinatorial depth still
depends on choosing good shared semantic dimensions, authoring useful bridge
rules, and validating them through gameplay scenarios.

## Research method

The study used four complementary forms of evidence:

1. open-source engines and moddable simulation games;
2. public developer documentation and postmortems from shipped games;
3. formal models from programming languages, simulation, databases, planning,
   multi-agent systems, and concurrency;
4. a repository-grounded audit of the M1-M4 implementation.

Claims about an engine's internal structure use official documentation or
source-oriented project material where possible. Commercial games are used
primarily as design and authoring evidence, not as proof of undocumented
internals.

The architecture was evaluated against scenarios intended to falsify weak
forms of extensibility. The test is not "can this behavior be hard-coded?" Every
scenario can be hard-coded. The test is whether it can be introduced through
the intended extension plane while preserving authority, information, time,
and replay laws.

## Premises that the design must preserve

The preceding target work establishes several non-negotiable premises:

- world truth, observation, evidence, belief, social interpretation, intent,
  activity, action, and process are distinct;
- only the runtime authority path changes accepted state;
- replaceable policies and AI receive immutable capability-scoped inputs and
  return bounded outputs;
- actor-visible opportunity discovery is not authoritative legality;
- time-bearing work is explicit and serializable;
- causally later work is scheduled rather than recursively executed;
- replay never silently invokes external computation;
- exact definitions and implementation identities are closed into a semantic
  epoch;
- resolution may vary by phenomenon and subsystem, while identity, causal
  commitments, and selected invariants survive conversion;
- complicated subsystem internals must not enlarge the cross-system waist.

The present research accepts these premises and asks whether the gameplay
extension model actually preserves them at scale.

## Vocabulary used in this note

Several established target terms contain the word "family." They are not one
universal abstraction:

| Term | Meaning here |
|---|---|
| gameplay mechanic | player-visible behavior that may combine several definitions, owners, and lifecycles |
| definition family | one checked T1 IR kind, such as actions, processes, appraisal rules, or social rules |
| primitive implementation | trusted T3 code implementing one exact semantic interface or solver |
| state owner | concrete code with private authority to validate one accepted-state or runtime-control representation |
| request or lifecycle family | one typed protocol with its own request, result, continuation, and removal rules |
| `ActivatedDefinitionRegistry` | the normative reconstructible realization of compiled definitions and exact implementation bindings; not a registry of accepted-state owners |

Likewise, "event" is not used as a generic callback:

| Term | Meaning here |
|---|---|
| authority record | immutable evidence of an accepted or rejected authority attempt |
| domain event | typed semantic occurrence produced by an accepted domain transition |
| observation signal/evidence | observer-scoped consequence derived through the epistemic path |
| `ReactionEnvelope` | self-contained committed description of causally later reaction work |
| scheduler trigger or wake | typed future input to a process or lifecycle |

An ordinary T0/T1 mechanic may use several installed primitive
implementations and state owners. A genuinely new T3 primitive may legitimately
add concrete owner types, schemas, codecs, protocol variants, and
composition-root wiring. The claim of locality is stricter for T0/T1 than for
T3.

## Case studies

### Comparative result

| Project | Strong compositional unit | Main source of depth | Main architectural warning |
|---|---|---|---|
| Cataclysm: DDA | typed JSON definitions, shared IDs, materials, qualities, pockets, and EOCs | many mechanics interpreting the same semantic vocabulary | a broad condition/effect language can accumulate legacy context, inconsistent capabilities, and hard-coded ceilings |
| Space Station 14 | data-only ECS components, prototypes, small physical unit operations | structural composition over shared gases, reagents, power, containment, and affordances | mutable events, cancellation, direct system dependencies, and sorted handlers hide semantic ordering |
| Veloren | detailed ECS plus a distinct low-resolution relational simulation | different representations for different spatial and temporal scales | RtSim does not establish the transactional and replay contracts required by `world` |
| Caves of Qud | blueprint parts, mixins, effects, and event-reactive objects | pervasive object composition and abstract-to-concrete generation | parts mix state and handlers; cascading events, merge order, legacy protocols, and patching complicate causality |
| Project Zomboid | script/Lua content plus newer resource and fluid foundations | intended semantic continuity across inventory, world objects, machines, fluids, and crafting | the foundational overhaul required substantial mod migration and explicit version boundaries |
| Dwarf Fortress | shared materials, tissues, anatomy, history, relationships, and time-bearing actions | orthogonal simulation axes reused across game modes | data definitions cannot compensate indefinitely for hard-coded semantic algorithms |
| Versu | concurrent social practices, roles, affordances, and agent utility | several social contexts contribute actions without taking actor autonomy | one shared practice state cannot represent divergent participant interpretations |
| Factorio | separate prototype construction and deterministic runtime | exact data-stage semantics plus deterministic simulation | runtime scripting remains powerful enough that semantic identity and desync policy must be explicit |

No reviewed case demonstrates that one mechanism is sufficient. Across this
purposive sample, successful designs combine structural data composition, a
shared vocabulary, time-bearing state, system-specific solvers, authored
definitions, and a runtime coordinator. We infer that recurring long-term
failure modes cluster around missing ownership, version, or causal boundaries.

### Cataclysm: Dark Days Ahead

CDDA demonstrates the leverage of a large shared semantic vocabulary. Items,
materials, bodies, recipes, terrain, vehicles, monsters, effects, mutations,
time, volume, mass, energy, qualities, pockets, and damage types participate in
typed definitions mapped into C++ structures. Its Effect-on-Condition system
adds manual, recurring, and event-triggered conditions and effects. See the
official [JSON interface](https://docs.cataclysmdda.org/c%2B%2B/JSON_INTERFACE.html),
[definition catalogue](https://docs.cataclysmdda.org/JSON/JSON_INFO.html), and
[Effect-on-Condition documentation](https://docs.cataclysmdda.org/JSON/EFFECT_ON_CONDITION.html).

The important positive lesson is not that JSON itself creates depth. It is that
several systems agree that a substance has material, phase, energy, quantity,
containment, consumption, and crafting meanings. An instance can participate
in many mechanics without each pair of mechanics knowing about the other.

The official documentation also reveals the cost of allowing an authoring
language to expand without a firm semantic boundary:

- inheritance and merge support varies by definition kind;
- chained inheritance is difficult to understand;
- EOC retains dialogue-derived "alpha/beta talker" context;
- some reactivation paths lose variables or conditions;
- important vehicle and combat behavior remains hard-coded.

See the official
[JSON inheritance limitations](https://docs.cataclysmdda.org/JSON/JSON_INHERITANCE.html),
[vehicle-part contributor guide](https://github.com/CleverRaven/Cataclysm-DDA/wiki/New-Contributor-Guide-Vehicle-Parts),
and
[martial-arts contributor guide](https://github.com/CleverRaven/Cataclysm-DDA/wiki/New-Contributor-Guide-Martial-Arts).

The implication for `world` is:

```text
adopt:
  checked typed definition kinds
  common units and semantic dimensions
  reusable conditions and effects inside bounded domains

avoid:
  one string-variable condition/effect language becoming the world ontology
  authored rules acquiring a second mutation path
  definition behavior that depends on accidental merge order
```

### Space Station 14 and RobustToolbox

Space Station 14 is the clearest open-source case for ECS-style content
composition. Entities are identifiers, components are data, systems own
behavior, and YAML prototypes select and parameterize components. The
[RobustToolbox ECS documentation](https://docs.spacestation14.com/en/robust-toolbox/ecs.html)
and
[bike-horn example](https://docs.spacestation14.com/en/ss14-by-example/adding-a-simple-bikehorn.html)
show how small reusable pieces can produce new objects without a bespoke class.

Its atmospherics design is more important than ECS itself. Vents, pumps,
scrubbers, radiators, sensors, pipes, gases, pressure, and heat form small
"unit operations" whose networks produce larger behavior. Chemistry similarly
separates stored solutions from capabilities such as draining, drawing,
injecting, and refilling. See the official
[atmospherics design](https://docs.spacestation14.com/en/space-station-14/departments/atmos.html)
and
[solution-container model](https://docs.spacestation14.com/en/space-station-14/core-tech/chemistry/solution-containers.html).

This is strong evidence for:

- small semantically meaningful state facets;
- capabilities rather than object-class dispatch;
- quantitative shared media such as heat, gas, liquid, and power;
- long-running mechanics represented by serializable state rather than an
  ordinary async continuation.

The same ECS documentation exposes the limitation. Cross-component behavior
may require a third system. Systems may depend on and call other systems.
Mutable events allow multiple listeners to alter a shared result. Cancellable
and sorted subscriptions make cooperation and order part of semantics.

`world` should therefore borrow structural composition and unit operations,
but replace mutable interception with:

```text
immutable attempt
  -> pure constraint or proposal contributions
  -> private authoritative qualification
  -> explicit resolution
  -> atomic commit
```

In `world`, a domain event reports what committed, while a
`ReactionEnvelope` or scheduler input identifies later work. Neither is a
shared mutable object through which listeners negotiate hidden authority.

### Veloren

Veloren separates detailed ECS gameplay from a low-resolution world
simulation. Its crate architecture supports headless server, client, and world
libraries; its detailed simulation uses ECS. See the official
[codebase structure](https://book.veloren.net/contributors/developers/codebase-structure.html)
and [ECS rationale](https://book.veloren.net/contributors/developers/ecs.html).

RtSim is particularly relevant. It represents long-running factions,
migration, trade, ecosystems, disasters, quests, and unloaded regions using
simple ID-keyed tables closer to a relational model than the detailed ECS. It
may update different work at different rates and intentionally omits detailed
combat, items, physics, and movement. See the
[RtSim architecture documentation](https://docs.veloren.net/veloren_rtsim/).

This supports a central multi-resolution decision:

> Different resolutions should preserve shared meaning, not necessarily share
> storage representation or algorithms.

The same identity may have detailed physical state, background travel state,
retained social commitments, and dormant unrelated cognition. A promotion or
demotion boundary should preserve declared quantities and causal commitments;
it should not require that both sides use the same ECS components.

RtSim itself permits direct mutation and intentionally tolerates weak
relational invariants. Direct mutation is not inherently nondeterministic, but
RtSim does not establish the transactional authority, exact replay, and
actor-relative evidence contracts required by `world`. The representation
lesson is adopted; its mutation model is not evidence for those contracts.

### Caves of Qud

Qud objects are composed from blueprint parts, mixins, and effects. Skills and
mutations are also parts, and most behavior participates in an event system.
The official modding material gives a compact example: combining liquid
storage and melee-weapon parts creates a weapon that can hold liquid. See
[objects and blueprints](https://wiki.cavesofqud.com/wiki/Modding%3AObjects),
[parts](https://wiki.cavesofqud.com/wiki/Modding%3AParts), and
[events](https://wiki.cavesofqud.com/wiki/Modding%3AEvents).

Qud also provides unusually useful authoring evidence. Its village generation
passes through several resolutions:

```text
authored premises
  -> abstract history and neighbors
  -> culture
  -> architecture
  -> concrete places, objects, NPCs, dialogue, and quests
```

The official GDC
[session](https://www.gdcvault.com/play/1026313/Math-for-Game-Developers-End)
and
[slides](https://media.gdcvault.com/gdc2019/presentations/Grinblat_Jason_End-to-End_Procedural_Generation.pdf)
emphasize explicit module inputs and outputs, reusable parameterized tools, and
reification from abstract products into concrete content.

These are valuable models for AI-assisted authoring. AI need not emit every
item and line of dialogue directly. It can produce checked social premises,
institutions, history, relationships, needs, and site requirements that later
compilers reify into ordinary pack artifacts.

Qud's modding documentation is also cautionary evidence. Parts combine state
and handlers. Standard and minimal events coexist, and event registration may
be serialized; see
[events](https://wiki.cavesofqud.com/wiki/Modding%3AEvents). Mixin priority and
load order affect composition; see
[objects](https://wiki.cavesofqud.com/wiki/Modding%3AObjects). Global naming
requires conventions, Harmony patching reaches outside the normal model, and
save evolution needs migration-aware part types; see
[mod compatibility](https://wiki.cavesofqud.com/wiki/Modding%3ACompatibility).

The lesson is to preserve pervasive composition but split a Qud-like "part"
into:

```text
owned typed state
+ exact implementation identity
+ declared views and capabilities
+ typed proposal/effect ports
+ explicit codec and migration
```

It should not become a serialized collection of callbacks.

### Project Zomboid

Project Zomboid's established mod surface includes broad Lua access and
string-keyed events, visible in its
[LuaEventManager API](https://projectzomboid.com/modding/zombie/Lua/LuaEventManager.html).
Build 42 development work is useful as a foundation case study: its public
design and testbed posts proposed unified machines and stations,
item/fluid/power resources, a fluid registry, quantitative per-volume
properties, mixed-fluid behavior, and crafted attributes inherited from inputs
and skill. These posts were forward-looking and do not establish that every
shown detail shipped unchanged. See the official development posts on
[machine and resource foundations](https://projectzomboid.com/blog/news/2023/08/the-connection-is-made/),
[liquids](https://projectzomboid.com/blog/news/2022/07/liquid-zedball/), and
[crafting and logistics](https://projectzomboid.com/blog/news/2023/04/crafting-ramblz/).

The most transferable design goal is semantic continuity across
representations: an object should retain meaningful attributes when it is
carried, placed in the world, attached to a machine, or consumed as crafting
input.

The migration cost is equally important. The developers introduced
version-specific mod directories and load ordering while warning that nearly
all existing mods would need work. See
[the Build 42 mod transition](https://projectzomboid.com/blog/news/2024/08/tidy-up-time/).

For `world`, persistence, definition identity, migrations, and semantic epochs
are therefore part of gameplay architecture rather than infrastructure to add
after content stabilizes.

### Dwarf Fortress

Dwarf Fortress demonstrates combinatorial depth through broad orthogonal
dimensions: material properties, tissues, anatomy, skills, timed attacks,
pain, bleeding, poison, weather, geology, populations, civilizations, roles,
relationships, artifacts, and historical events. The same persistent world
supports world generation and several play modes. See the official
[feature model](https://bay12games.com/dwarves/features.html) and
[modding guide](https://bay12games.com/dwarves/modding_guide.html).

This is evidence for a reusable semantic substrate. Material × tissue ×
temperature × force × anatomy has much more leverage than a catalogue of
special attack/creature pairs.

It is not evidence that data files can express every future mechanic. In its
dated 2025-07-21 entry, the official development log describes exposing
roughly fifteen years of hard-coded procedural generation for forgotten
beasts, curses, necromancers, evil weather, and related features through Lua.
See the [official development log](https://bay12games.com/dwarves/index.html).

The architecture should be honest about this boundary:

```text
new content under existing meanings
  != new rule composed from existing primitives
  != a genuinely new primitive solver
```

The third case requires a trusted extension and a new semantic epoch, not a
DSL stretched until it embeds a second engine.

### Versu and social practices

Versu is the strongest direct model for the desired social layer. Its
architecture represents several concurrent social practices. Each practice
defines roles and contributes contextually appropriate affordances; an
individual actor still chooses among the union using its own preferences and
utility. The practice coordinates meaning and continuity without replacing
actor autonomy. See Evans and Short,
[*The AI Architecture of Versu*](https://versu.com/wp-content/uploads/2014/05/versu.pdf).

This supports:

```text
physical and dialogue occurrences
  -> observer-relative evidence
  -> active social-practice interpretation
  -> role-relative affordances and pressures
  -> actor-owned appraisal and intent
```

A dinner, court proceeding, debt negotiation, romance, theft investigation,
and faction duty can overlap. Their action contributions form a union; their
norms remain separately identifiable. The architecture assumes one shared
practice state, so it cannot represent participants who disagree about which
practice state they are in. `world` should adopt concurrent practice
affordances but reject that limitation by grounding interpretation in
actor-relative evidence and institution-scoped accepted records.

The correct response is not a universal social score. It is typed practice,
role, claim, standing, obligation, and counts-as records with explicit
institutional scope.

### Factorio and exact semantic closure

Factorio separates prototype construction from runtime. Mods define data in a
staged prototype lifecycle, while its multiplayer architecture relies on
deterministic simulation. See the official
[prototype documentation](https://lua-api.factorio.com/latest/index-prototype.html)
and developer explanation of
[deterministic lockstep](https://www.factorio.com/blog/post/fff-76).

This supports two `world` decisions:

- definitions should be compiled and linked before activation;
- one live simulation should bind an exact semantic set rather than depend on
  ambient installation order.

`world` should be stricter than an ordinary runtime script environment because
AI-authored content, replay, branches, and research comparison require exact
meaning. A behavior-relevant definition change creates a child epoch; it does
not hot-reload into the current one.

### Supporting commercial-engine evidence

Larian's Osiris demonstrates the authoring leverage of typed, event-driven,
relational facts, queries, calls, and global databases for game-wide narrative
logic. See the official
[Baldur's Gate 3 scripting introduction](https://docs.baldursgate3.game/Scripting%3A_Introduction_to_Osiris).
It is `world`'s authority-boundary inference—not a Larian claim—that this form
of global rule database should derive narrative and social reactions rather
than own hard physical mutation.

Unreal's Gameplay Ability System decomposes abilities, attributes, effects,
tasks, and tags and provides server-authoritative activation patterns. It is a
useful action/effect subsystem reference, not a complete world semantics. See
the official
[Gameplay Ability System overview](https://dev.epicgames.com/documentation/en-us/unreal-engine/understanding-the-unreal-engine-gameplay-ability-system).

The transferable conclusion is consistent: relational rules and ability
frameworks are useful bounded subsystems. Neither should become the authority
model for every physical, epistemic, social, and temporal fact.

## Formal and theoretical frameworks

### Heterogeneous models of computation

Ptolemy II is the closest theoretical analogue to the desired whole. It
separates components from the model of computation supplied by a director.
Different domains define different communication, concurrency, and execution
semantics, and heterogeneous models can compose hierarchically. See the
Berkeley report
[*Heterogeneous Composition of Models of Computation*](https://www2.eecs.berkeley.edu/Pubs/TechRpts/2007/EECS-2007-139.html)
and the official
[Ptolemy II domain documentation](https://ptolemy.berkeley.edu/ptolemyII/ptII10.0/ptII10.0.1/doc/domains.htm).

The transferable principle is:

> Composition requires an explicit model governing how unlike components
> interact. It is not created merely by giving them the same interface.

`world` should use a stricter form. There is one outer authority director:
deterministic superdense time, immutable epoch semantics, proposal admission,
atomic commit, records, and future work. Inner domains can retain their own
models, but may only derive information or propose bounded changes.

This is why the architecture should not seek a generic
`Subsystem<Input, Output>` abstraction. That interface would erase the exact
properties that matter: state ownership, temporal behavior, information
authority, conflict algebra, and persistence.

### Labeled transition systems and rewriting logic

The engine's world and attempt-control transitions can be viewed as a typed
labeled transition system using the target model's existing notation:

```text
Γ ⊢ Ωa --ℓ--> Ωa'

Ωa = (Ca, Σ)
```

where:

- `Γ` is immutable execution semantics;
- `Ca` is the separately durable `AttemptControlPlane` for controlled attempt
  `a`;
- `Σ` is the authoritative `WorldSession`, including accepted state, typed
  runtime control, scheduler state, and the authority-history head;
- `ℓ` identifies either a world-authority transition or an attempt-control
  transition and its retained evidence.

This notation does not give `Ca` authority over world meaning. Only the target
model's `Admit`, `Fire`, and `Manage` relations change an existing `Σ`.
Reservation, reconciliation, cancellation, retention, and compaction
operations may change `Ca` but do not reinterpret or advance `Σ`.

Rewriting logic provides a powerful executable model of local conditional
state transitions and concurrency. See Meseguer,
[*Conditional Rewriting Logic as a Unified Model of Concurrency*](https://doi.org/10.1016/0304-3975(92)90182-F),
and the [Maude system-module manual](https://maude.lcc.uma.es/maude-manual/maude-manualch5.html).

It is valuable for offline mechanic models and critical-pair exploration:

```text
integrity(door) <= 0
  -> barrier(door) = breached

exposed(oil) and temperature(oil) >= ignition_point(oil)
  -> burning(oil)
```

It should not become the universal production rule engine. Unrestricted
rewrites obscure ownership and require global choices about confluence,
termination, priorities, and rule strategy. Many game interactions are
intentionally nonconfluent.

Modular Structural Operational Semantics offers another useful criterion:
semantic rules for an existing construct should not need reformulation merely
because an unrelated construct was added. See Mosses,
[*Modular Structural Operational Semantics*](https://tidsskrift.dk/brics/article/view/21873).

For `world`, the analogous fitness property is:

> Adding a T0/T1 construct under installed semantics should add definitions
> and tests without reformulating the authority rules of existing constructs.
> A T3 primitive may add concrete owners and protocols but should preserve the
> outer authority semantics.

### Discrete-event simulation and DEVS

Classic DEVS describes an atomic model using typed inputs and outputs, explicit
state, internal and external transition functions, an output function, and a
time-advance function. Coupled models retain model semantics. See Van Tendeloo
and Vangheluwe,
[*An Introduction to Classic DEVS*](https://arxiv.org/pdf/1701.07697).

Superdense time `(time, microstep)` distinguishes causally ordered work at the
same simulation time. Ptolemy's
[discrete-event model chapter](https://ptolemy.eecs.berkeley.edu/books/Systems/chapters/DiscreteEventModels.pdf)
provides the relevant model.

This supports explicit timed state and motivates the current runtime
direction:

- one global simulation-time basis and explicit microsteps;
- persistent typed process state;
- next wake, external input, interruption, completion, and cancellation;
- a complete least-due batch treated as a semantic unit;
- later consequences scheduled instead of synchronous reentrancy;
- a declared bound or termination argument for zero-time causal chains.

The complete-batch policy, ramification staging, and zero-time bound are
`world` policies rather than consequences uniquely implied by DEVS. DEVS is a
process-design discipline, not the ontology for belief, appraisal,
institutions, or every derived rule.

### Datalog, fixed points, and incremental views

Typed relational derivation is a good fit for facts such as:

```text
reachable(actor, target)
has_effective_tool(actor, operation)
physically_affords(actor, target, action)
perceived_affordance(actor, target, action)
witnessed(actor, event)
institution_member(actor, institution)
obligation_due(obligation, moment)
```

Datalog gives monotone recursive relations a least-fixed-point meaning.
Incremental view maintenance and systems such as DBSP show how to update rich
queries from base changes rather than recompute everything. See
[*DBSP: Automatic Incremental View Maintenance for Rich Query Languages*](https://arxiv.org/abs/2203.16684)
and McSherry et al.,
[*Differential Dataflow*](https://www.microsoft.com/en-us/research/publication/composable-incremental-and-iterative-data-parallel-computation-with-naiad/).

The necessary boundary is:

> Relational rules derive views, explanations, constraints, and candidates.
> They do not directly mutate authoritative state.

A derivation program should declare:

- typed base relations and owners;
- dependencies and invalidation;
- monotone strata;
- explicit strata for negation, absence, maxima, priority, or defaults;
- output relation and provenance policy;
- full recomputation as the semantic oracle.

Incremental caches are reconstructible acceleration. A global untyped EAV fact
store is not the world model.

### Action formalisms, the frame problem, and affordances

STRIPS and PDDL model parameterized actions with preconditions and effects;
later variants add time, conditional effects, and numeric resources. See the
[IPC PDDL resources](https://ipc08.icaps-conference.org/deterministic/PddlResources.html)
and
[Fox and Long's PDDL 2.1 paper](https://doi.org/10.1613/jair.1129).

Situation-calculus work identifies three recurring difficulties:

- the frame problem: what remains unchanged;
- the qualification problem: what may prevent an action;
- the ramification problem: which indirect consequences follow.

See McCarthy and Hayes,
[*Some Philosophical Problems from the Standpoint of Artificial Intelligence*](https://www-formal.stanford.edu/jmc/mcchay69.html),
and Lin and Reiter,
[*State Constraints Revisited*](https://doi.org/10.1093/logcom/4.5.655).

The current target has a strong practical answer:

```text
grounded semantic choice
  -> private authoritative qualification
  -> owner-validated direct delta
  -> atomic transition
  -> typed later ramifications
```

State owners preserve unmentioned state by construction. Runtime may reject an
actor-reasonable attempt because hidden or stale qualifications fail. Indirect
consequences need not be embedded in one enormous action effect list.

Affordances should be relational. Chemero formalizes them as relations between
an organism's abilities and environmental features rather than properties of
either in isolation. See
[*An Outline of a Theory of Affordances*](https://doi.org/10.1207/S15326969ECO1502_5).

The architecture should therefore distinguish:

```text
perceived affordance
  actor-relative evidence or belief says an action appears possible

physical affordance
  actual capabilities, target state, tools, and environment support it

authoritative executability
  permissions, resources, freshness, and invariants admit it now
```

False perceived affordances are a feature. They support darkness, deception,
disguise, traps, unknown locks, and fallible planning without special cases.

### Linear logic, resources, and mechanic prototypes

Ceptre uses linear logic to model resource-sensitive generative interactive
systems. It is well suited to rapid exploration of multi-agent gameplay spaces
where facts are consumed and produced. See Martens,
[*Ceptre: A Language for Modeling Generative Interactive Systems*](https://www.cs.cmu.edu/~cmartens/ceptre.pdf).

It is useful for prototypes such as:

```text
has(actor, key) * locked(door)
  -o has(actor, key) * unlocked(door)
```

The useful lesson is that resources and transitions should be explicit and
machine-checkable. Ceptre should not become the whole runtime. Quantitative
physics, private knowledge, persistent processes, stochastic policies,
multi-resolution approximation, and owner-specific invariants need different
models.

### Agency: BDI, planning, utility, and AI

BDI research separates beliefs, desires or pressures, intentions, and plans.
Its most useful property here is persistent commitment: an intention survives
individual action attempts until explicit success, failure, or reconsideration
changes it. See Rao and Georgeff,
[*BDI Agents: From Theory to Practice*](https://cdn.aaai.org/ICMAS/1995/ICMAS95-042.pdf),
and Georgeff and Lansky,
[*Reactive Reasoning and Planning*](https://aaai.org/Papers/AAAI/1987/AAAI87-121.pdf).

HTN, GOAP, utility selection, behavior trees, authored rules, and LLMs answer
different questions. They are replaceable implementations inside lifecycle
boundaries, not competing world architectures.

The stable law is:

```text
actor-relative evidence and belief
  -> appraisal
  -> persistent intent
  -> persistent activity or plan hypothesis
  -> actor-visible grounded opportunity
  -> ordinary authoritative action path
```

Plans are hypotheses over an actor's belief model. They are not reservations
against hidden world truth. An external AI selects supplied semantic IDs or
returns bounded proposals and never receives another command path.

### Epistemics, social practices, and normative systems

Dynamic epistemic logic models how public, private, and partially observed
events change knowledge. Truth-maintenance systems retain justifications and
revise beliefs when assumptions change. See Baltag, Moss, and Solecki,
[*Logics for Epistemic Actions*](https://arxiv.org/abs/2203.06744), and Doyle,
[*A Truth Maintenance System*](https://doi.org/10.1016/0004-3702(79)90008-0).

The engine does not need a full possible-world model for every NPC. It needs
the architectural separations:

- a domain-event occurrence and its authority record are retained history;
- observation is observer-specific;
- evidence retains source and provenance where gameplay needs it;
- belief is accepted actor state and may be contradictory or wrong;
- trust and bounded nested reasoning are explicit policies;
- social recognition is not physical truth.

Normative multi-agent systems distinguish constitutive rules—"this counts as
theft in this institution"—from regulative rules—"theft is forbidden and
activates an obligation or sanction." See Boella and van der Torre,
[*Regulative and Constitutive Norms in Normative Multiagent Systems*](https://cdn.aaai.org/KR/2004/KR04-028.pdf).
That distinction suggests:

```text
accepted domain event
  -> observer evidence
  -> scoped counts-as interpretation
  -> norm activation or violation
  -> claim, obligation, standing, or sanction
  -> actor appraisal and later behavior
```

This distinction is necessary for secret crimes, false accusations, trials,
conflicting jurisdictions, role authority, and reputation.

### Algebraic effects and definition-specific authoring languages

Algebraic effects separate a requested operation from the handler that
interprets it. See Plotkin and Pretnar,
[*Handlers of Algebraic Effects*](https://www.research.ed.ac.uk/en/publications/handlers-of-algebraic-effects/).

This is a useful authoring model:

```text
authored operation
  -> checked definition-specific IR
  -> exact trusted handler or lowerer
  -> bounded typed proposal
```

The engine may share small expression fragments where semantics genuinely
match, but actions, processes, appraisal, social rules, observations, and
effects should retain domain-specific IRs. A universal DSL would either become
an entire unsafe programming language or accumulate escape hatches.

### Transactions, conflict, and local reasoning

The global correctness target is deterministic set-wise resolution of one
complete least-due batch: proposals share a declared base, conflicts and merge
laws are explicit, and the combined successor satisfies all participating
invariants. A deliberately simultaneous commutative result need not always be
defined by executing the proposals one at a time.

Database serializability remains one useful domain policy when the desired
meaning is equivalence to a valid sequential ordering. Optimistic concurrency
also informs the existing read-witness and commit-time freshness protocol. See
Papadimitriou,
[*The Serializability of Concurrent Database Updates*](https://doi.org/10.1145/322154.322158),
and Kung and Robinson,
[*On Optimistic Methods for Concurrency Control*](https://doi.org/10.1145/319566.319567).

Each state owner should conceptually expose a partial delta composition:

```text
delta_a (+) delta_b
```

The operation is defined only when changes are:

- disjoint;
- explicitly commutative;
- or merged by a domain-defined law bound into the semantic epoch.

Otherwise they conflict. Examples:

- heat from two independent sources may be additive;
- separate testimony records may coexist;
- two actors taking one unique item conflict;
- opening and destroying the same door needs explicit domain semantics;
- incompatible ownership judgments from different institutions may coexist
  rather than collapse into one value.

Separation logic provides a helpful local-reasoning analogy: a transition
should preserve framed state outside its declared footprint. See Reynolds,
[*Separation Logic: A Logic for Shared Mutable Data Structures*](https://www.cs.cmu.edu/~jcr/seplogic.pdf).
The engine need not implement separation logic. It should adopt the discipline
that independence is declared and checked rather than inferred from execution
order.

### Multi-resolution semantics as refinement

Abstract interpretation suggests a disciplined concrete-to-abstract mapping,
while simulation and refinement relations suggest how relevant behavior may be
preserved. See Cousot and Cousot,
[*Abstract Interpretation*](https://www.di.ens.fr/~cousot/COUSOTpapers/POPL77.shtml),
and Lynch and Vaandrager,
[*Forward and Backward Simulations, Part I*](https://doi.org/10.1006/inco.1995.1134).
Together they provide part of a useful shape for
Detailed/Background/Dormant correspondence; they do not dictate `world`'s
promotion algorithm.

For a resolution-capable scope:

```text
abstract : DetailedState -> BackgroundState
promote  : BackgroundState x evidence/seed -> DetailedState
R        : DetailedState x BackgroundState -> Boolean
```

The following contract is `world`'s synthesis from those ideas. It should
name:

- stable identities and lineage;
- conserved quantities or bounded error;
- retained obligations and in-progress processes;
- simulation time and causal timestamps;
- allowed abstract transitions;
- observation summaries that remain causally relevant;
- newly materialized detail and its provenance;
- stale-wakeup replacement;
- repeated promotion/demotion error policy.

Exact bisimulation is usually too strong. The useful target is
scenario-specific observational refinement: active observers and later
authoritative behavior cannot distinguish the two beyond the declared
abstraction.

### Formal tools as validation, not runtime dependencies

Different risks call for different tools:

- TLA+ for scheduler frontiers, idempotency, crash recovery, checkpoint, and
  replay protocols;
- Maude for small gameplay rewrite models and critical-pair search;
- Alloy for bounded structural ownership and composition checks;
- coloured Petri nets for lifecycle liveness, resources, and deadlock;
- property and metamorphic tests for permutation invariance, replay purity, and
  incremental/full-recompute equivalence.

No tool should become a production dependency merely because it is useful for
design verification.

## Architecture synthesis

This section separates the already normative outer law from research
hypotheses about future gameplay composition. The authority, time,
information, and epoch boundaries are confirmed target constraints. The T3
extension dossier, dependency graph, and cross-primitive implementation shape
remain hypotheses to test before any new common API is selected.

### One outer law, several inner semantic models

The unifier is authority, not representation:

```text
Γ ⊢ Ωa --ℓ--> Ωa'
```

Retain the normative decomposition:

```text
Γ =
  ExecutionSpec
  + EngineProtocolVersion
  + RuntimeDefinitionSet
  + SemanticImplementationSet
  + LifecycleProfiles
  + ExecutionConfigArtifact

Ωa = (Ca, Σ)

Ca = AttemptControlPlane for attempt a

Σ = authoritative WorldSession

accepted_state(Σ) =
  domain product
  x epistemic product
  x social product
  x agency product

each partition = conceptual product of privately owned typed state
  representations
```

The product is conceptual. It does not require one giant Rust struct, one ECS
world, or one dynamic property map.

### Trusted primitive extension dossier

A candidate T3 primitive implementation `K_i` should be reviewable through
the following dossier:

```text
K_i =
  identity and schema
  owned accepted state S_i
  checked definitions D_i
  required typed reads R_i
  pure derived views V_i
  input authority and stage labels L_i
  permitted disclosure/declassification and output audience X_i
  bounded proposal algebra P_i
  private validation and lowering T_i
  canonical resource and invariant claims F_i
  emitted domain-event contracts E_i
  process and wake contracts W_i
  invariants I_i
  codec and migration policy C_i
  optional resolution obligations A_i
```

This is a research checklist, not one Rust trait, accepted-state registry, or
mandatory linker contract. Some primitives will not own every listed artifact.
Forcing them into a uniform runtime interface would erase useful static types
and encourage an unbounded framework.

At distribution construction, exact primitive implementations remain
statically linked by concrete code unless a later scenario justifies another
mechanism. At activation, the normative `ActivatedDefinitionRegistry`
reconstructibly resolves compiled definitions and exact semantic-interface
bindings. It does not own accepted state and does not imply one dispatcher for
all primitive protocols.

During an epoch, the selected definitions, implementations, schemas, and
policies cannot appear or change. General type-erased primitive state is not
selected here. D-025's sealed implementation-defined persistent state remains
limited to the exact lifecycle-port owner and bounds described by the target
architecture.

### Six distinct forms of composition

#### 1. Structural composition

Entities participate in several typed, separately owned state facets:

```text
door_17:
  spatial barrier
  open/closed configuration
  lock mechanism
  material and integrity
  combustion state
  institutional ownership relation
```

Storage may be ECS, tables, graphs, arenas, or domain-specific indexes inside
the owner. The cross-system contract is typed identity and projections, not
storage layout.

#### 2. Derivational composition

Pure relations combine owned facts:

```text
actor capability
  x perceived target features
  x available tools
  x local topology and environment
  x checked action definition
  -> actor-visible candidate
```

Dependencies, authority class, stage, provenance, permitted disclosure, output
audience, and invalidation are explicit. Monotone recursion may use a fixed
point. Nonmonotone rules require declared
strata or recomputation boundaries.

#### 3. Behavioral composition

Several action, appraisal, intent, or practice definition families may
contribute bounded candidates. Their union is an immutable choice surface.
Ranking and selection belong to actor policy. Candidate contributors do not
mutate one another or the world.

#### 4. Causal and transactional composition

Selected proposals are privately qualified and lowered to owner-validated
deltas with canonical resource claims. Compatible deltas co-commit atomically;
incompatible changes receive explicit dispositions. No semantic state uses
last-writer-wins.

Within one authority class, a transition that must preserve an invariant
across several physical or control owners requires all participating owner
validations to succeed. One owner cannot decode or directly edit another
owner's state.

This does not imply one transaction across every truth or lifecycle partition.
Only an explicit verified transaction type may coordinate more than one
partition. The normal chain from physical change through observer evidence,
social interpretation, and agency adaptation remains causally ordered through
typed commit gates and communicates by immutable records and later work.

#### 5. Temporal composition

Activities, physical processes, institutional deadlines, and evaluator
invocations have persistent typed state and explicit wakes. Same-time work is
resolved under batch and microstep semantics, not callback delivery order.

#### 6. Interpretive composition

The same physical occurrence may create different evidence for different
observers, different social meaning under different institutions, and
different appraisals for different actors. Interpretation adds scoped records;
it does not rewrite physical history.

These forms should not be collapsed into one API. Their shared waist is exact
identity, typed input, bounded output, authority admission, time, records, and
replay.

### Shared semantic substrate and bridge rules

Depth comes from multiple mechanics interpreting the same small semantic
dimensions. Candidate standard-world dimensions include:

- stable identity and lineage;
- topology, position, reachability, and barrier relations;
- containment, attachment, and support;
- quantity, substance, phase, material, temperature, and energy;
- integrity, structure, force, and damage;
- bodies, conditions, senses, skills, tools, and capabilities;
- observation signals, evidence, claims, and source provenance;
- roles, institutions, ownership claims, practices, obligations, and standing;
- simulation time, duration, process identity, and causal ancestry.

This list is not a final ontology. Each term should enter only with an active
producer, consumer, invariant, and scenario.

Bridge semantics must distinguish three consequence classes:

1. a pure derived consequence is a view over the current accepted head and
   performs no transition;
2. an invariant-coupled direct consequence is prepared inside the same
   explicit transaction and co-commits atomically, preventing an invalid
   intermediate accepted head;
3. a causally later reaction is recorded in a `ReactionEnvelope` or scheduler
   input and runs at a later microstep through its own typed gate.

Cross-system interactions occur through shared facts or explicit bridge rules:

```text
impact changes lamp integrity
  -> [same transaction if required by invariant] broken containment exposes oil
  -> [derived or later physical reaction] heat plus exposed fuel starts combustion
  -> [later process transitions] combustion produces light, heat, and smoke
  -> [derived view] smoke changes visible affordances
  -> [later typed gates] observers receive unequal evidence
  -> an institution records a scoped suspected-theft interpretation
  -> a guard later appraises a duty conflict and forms an intent
```

There is no `CombatSystem -> FireSystem -> PerceptionSystem -> SocialSystem`
call chain. There are also no magical "emergent" interactions without authored
semantics. The oil exposure and institutional counts-as rules are explicit,
typed, local, and testable bridges.

### Semantic dependency graph

The compiler/linker should eventually construct a semantic dependency graph.
Nodes remain definition-specific typed artifacts:

```text
state schema
derived view
capability or affordance
action/effect definition
process definition
domain-event definition
observation rule
appraisal rule
intent template
social rule
resolution conversion
```

Edges describe:

```text
reads
derives
invalidates
proposes
commits through
emits
schedules
observes
interprets
converts
```

This graph supports:

- missing-provider and cycle diagnostics;
- dependency and invalidation planning;
- explanation and causal tracing;
- effect and authority audits;
- AI authoring constraints;
- compatibility digests;
- scenario coverage reports.

It is compile/link metadata, not a universal runtime IR. Each node preserves
its domain semantics.

### Extension planes

The existing T0-T3 distinction should become an enforceable law:

| Tier | Meaning | Expected change |
|---|---|---|
| T0 | instances and content using existing definitions | pack/content only |
| T1 | checked definitions and effect programs over installed primitives | definition artifacts, compilation, and tests |
| T2 | replaceable policy or external computation | exact evaluator binding; bounded proposals or supplied IDs |
| T3 | genuinely new primitive semantics or solver | trusted implementation, concrete owner/schema/protocol changes as needed, and composition-root installation |

A new sword, recipe, spell composition, social practice, or intent template
should usually be T0/T1. A new conductivity solver or fluid dynamics model may
be T3.

T0/T1 additions should not change the runtime core. A T3 addition may
legitimately add concrete state owners, schemas, codecs, primitive protocol
variants, and distribution wiring. It should preserve the authority, staging,
time, and replay laws and avoid changes to unrelated primitives except through
explicit typed bridges.

The engine is therefore:

```text
open while authoring, compiling, linking, and composing a distribution
closed and exactly identified during one semantic epoch
```

### AI-assisted authoring

AI should be able to author at several levels:

```text
high-level premise
  -> history, institutions, needs, and relations
  -> checked definitions and instances
  -> compiled artifact
  -> validation and preview
  -> explicit activation as a child epoch or branch
```

AI may use installed T0/T1 vocabulary to create a poison recipe, ritual,
social practice, quest premise, or local institution. It cannot invent
unregistered primitive effects, execute generated host code in the live
session, or reinterpret an old checkpoint under new semantics.

Generated natural language remains presentation or evidence:

- an utterance does not directly create truth;
- a promise becomes an obligation only through an accepted social act and
  scoped institutional semantics;
- a narrative description does not mutate physical state;
- AI interpretation produces a bounded proposal with provenance.

New trusted code remains a separate engineering and distribution process.

## Target scenarios

### Scenario matrix

| Scenario | Primary systems | Architectural property under test |
|---|---|---|
| Burning locked warehouse | material, integrity, lock, tool, wound, fire, smoke, perception, agency | shared vocabulary, processes, actor-safe affordances, no direct system chain |
| Medicine contention during collapse | containment, destruction, topology, simultaneous actions | cross-primitive resource claims and atomic conflict resolution |
| Shrine relic theft | transfer, unequal witnesses, claims, taboo, institution, duty | truth/evidence/belief/social-meaning separation |
| Interrupted treatment | activity, tool, bleeding/healing process, fire interruption | independent lifetimes and recovery without central activity sums |
| Background caravan ambush | travel, cargo, obligations, combat, promotion/demotion | multi-resolution identity, time, commitments, and wake replacement |
| Capability-space mutation | skill, injury, tool, smoke, door | affordances derived from joins rather than bespoke actions |
| AI-authored poison | substances, recipe, conditions, child epoch | T1 authoring, validation, exact semantic closure |
| Promise, delay, and breach | dialogue, evidence, obligation, trust, caravan delay | AI language separated from social authority |

### 1. Burning locked warehouse with a wounded responder

A wooden locked door combines barrier, configuration, lock, material,
integrity, and combustion state. A responder has a wounded hand, a crowbar,
partial smoke-obscured perception, and a duty to recover medicine. Fire may
weaken the door before a force-open activity resolves.

Required causal shape:

```text
heat/exposure advances fire process
  -> integrity and smoke proposals
  -> atomic accepted changes
  -> observation signals
  -> actor-relative evidence
  -> capability and perceived-affordance derivation changes
  -> activity continues, interrupts, retries, or replans
```

The architecture fails if:

- the door owns bespoke gameplay callbacks;
- fire directly calls perception or agency;
- smoke is merely a presentation effect;
- hidden lock or integrity facts leak into actor policy;
- adding an existing-vocabulary `ForceOpen` definition expands global action
  and activity enums;
- handler order changes the result.

### 2. Medicine contention during same-moment structural failure

Two actors try to take the final medicine crate while fire or collapse destroys
the supporting shelf or invalidates the destination at the same `SimMoment`.

Required properties:

- all proposals in one least-due `(time, microstep)` Fire batch evaluate
  against its immutable base; a later microstep at the same physical time sees
  the prior committed head;
- containment, existence, support, and destruction use compatible canonical
  resource or invariant claims;
- independent evaluation order cannot change the result;
- all owner validations inside one prepared cross-owner transaction succeed or
  that transaction does not commit; compatible sibling attempts may still
  commit;
- every logical attempt receives a disposition;
- later observation and agency work cannot join the original transaction.

This is the most direct falsifier for a resolver whose conflict vocabulary is
specific to one primitive.

### 3. Shrine relic theft with unequal witnesses

An actor removes a relic. One guard sees the transfer. Another hears a false
accusation. A priest knows the local ownership claim and taboo. The player sees
only an empty pedestal.

The same occurrence should produce:

```text
authoritative physical transfer
  + different observer evidence
  + possibly inconsistent beliefs
  + institution-scoped theft interpretations
  + role-specific duties and appraisals
  + later investigation, concealment, arrest, or negotiation intents
```

The architecture fails if:

- physical transfer itself means theft;
- every actor receives the same raw domain-event payload;
- a claim automatically becomes truth;
- dialogue or AI directly writes belief, reputation, or obligation;
- adding the social interpretation changes the transfer primitive.

### 4. Interrupted treatment with an independent condition process

A healer starts treatment. Treatment changes or creates a bleeding/healing
process. The healer loses the tool or is interrupted by fire. The patient's
condition continues according to its own process semantics.

Required properties:

- intent, activity, action, and condition process have separate identities and
  lifetimes;
- cancelling an activity does not implicitly erase a physical process;
- process completion does not directly complete an intent;
- the activity can wait, recover, choose another method, or report exhaustion;
- adding an existing-vocabulary treatment definition does not enlarge central
  lifecycle sums across every crate.

### 5. Caravan promotion during an ambush

A caravan travels in Background while retaining people, cargo, relationships,
a delivery obligation, process identity, elapsed time, and causal history. It
is promoted to Detailed when the player approaches, encounters an ambush, then
later demotes.

Required properties:

- stable actor, cargo, and process identities;
- no replay of past decisions during promotion;
- no new knowledge created merely by reification;
- elapsed time and outstanding commitments preserved;
- stale background wakes cannot create a second arrival;
- different subsystem scopes may retain different resolutions;
- demotion records approximation and preserves causally relevant observations.

### 6. Capability-space mutation without bespoke actions

An actor learns lockpicking, acquires a compatible tool, injures one hand,
enters smoke, and encounters the same door.

The available candidates, duration, risk, and confidence should change through
typed joins:

```text
skill and body capability
  x tool capability
  x perceived door affordance
  x local environmental constraints
  x action definition
```

The architecture fails if:

- a door grants a bespoke `PickLock` command;
- capability is an untyped global tag list;
- progression edits cached candidate lists directly;
- actor-visible candidates depend on hidden target facts;
- an existing-vocabulary action requires runtime-core edits.

### 7. AI-authored poison recipe in a child epoch

During a campaign, AI drafts a poison recipe and application action using
installed substance, transfer, ingestion, wound, and condition primitives.

Required path:

```text
structured authoring request
  -> T1 source artifact
  -> type, stage, authority, dependency, and bound checks
  -> preview
  -> exact compiled definition set
  -> explicit child epoch/branch
```

The original branch must still replay identically. Compilation rejects
unregistered effects or illegal stage use. The generated recipe cannot hot
reload into the current epoch.

### 8. AI-mediated promise, delay, and breach

A merchant makes a structured promise to deliver medicine while AI generates
the natural-language utterance. Listeners hear different portions and assign
different trust. The caravan is delayed; the obligation becomes overdue;
institutions and actors respond differently.

The architecture fails if:

- the utterance directly creates truth or obligation;
- social state is a single global reputation score;
- travel delay calls social logic directly;
- unrelated observers learn the promise;
- the AI both interprets the act and commits its authoritative consequences.

## Repository audit

### What M1-M4 have already proven

The implemented foundation has several strong properties:

- a small authority/lifecycle kernel rather than a recursive mega-pipeline;
- separate accepted domain, epistemic, social, and agency partitions;
- actor-safe candidate selection separated from private runtime legality;
- immutable qualified pack definition identity;
- deterministic same-moment processing and causal microsteps;
- persistent typed process and lifecycle state within the session;
- proposal-only replaceable evaluation;
- mechanically checked crate dependency direction.

The target extension ladder and engine/game-system boundary are also
conceptually aligned with the research. See:

- [formal model](../architecture/target-architecture/formal-model.md);
- [extensibility and research](../architecture/target-architecture/extensibility-and-research.md);
- [engine core and game-system boundary](../design/engine-core-and-game-system-boundary.md);
- [reference game vision](../design/reference-game-vision.md).

### What remains unproven

#### Pack expressiveness

The current
[`ArtifactData`](../../crates/world-defs/src/artifact/mod.rs) model primarily
proves actions, ordered calls to installed effects, and physical events.
Standalone or richer checked effect programs, process definitions, observation
rules, appraisal rules, intent templates, and social rules remain target
concepts rather than a general implemented T1 surface.

#### Closed action and activity products

The current action path explicitly represents containment transfer and
relocation through closed products such as
[`ActionInteractionScope`](../../crates/world-model/src/action_opportunity.rs)
and
[`GroundedActionInteraction`](../../crates/world-context/src/action.rs). The
activity model similarly contains transfer and travel variants in
[`ActivityState`](../../crates/world-model/src/accepted/agency.rs). That was
appropriate evidence for M3 and M4; it prevented premature generalization.

It cannot be the normal T0/T1 mechanic-extension path. An existing-vocabulary
action or activity should not require coordinated new variants and match arms
through model, context, decision, engine, runtime, serialization, and tests. A
genuinely new T3 primitive may legitimately add concrete types and wiring
across its owning layers.

#### Hard-coded distribution and activation

[`EngineDistribution`](../../crates/world-engine/src/distribution.rs) and the
[runtime activation path](../../crates/world-runtime/src/execution/activation.rs)
currently name two implemented primitive semantics directly: containment
transfer and relocation. This is a valid seed composition root. It does not yet
prove how several additional T1 definition kinds resolve to installed
semantics, or what concrete wiring a third T3 primitive should require.

#### Cross-primitive contention

The
[deterministic resolver](../../crates/world-runtime/src/kernel/resolution.rs)
has been proven around containment-oriented footprints. It has not yet proven
a simultaneous transaction involving containment plus destruction, topology
invalidation, material transformation, or another independent primitive.

#### State-owner scaling

[`DomainState`](../../crates/world-model/src/accepted/domain.rs) is currently a
concrete aggregate of the implemented domain facets. Adding every future
material, integrity, lock, body, field, and resource record directly to that
aggregate would create constructor, schema, digest, query, and transition
churn. `ProcessInstance` remains runtime-control state rather than domain
accepted state.

The implementation still needs evidence for an extensible product of
privately owned typed stores without falling back to an untyped property bag.
The current target does not select a generalized type-erased state-owner
registry; concrete composition should remain the default until thin slices
demonstrate a narrower need.

#### Event and process closure

Domain events and scheduled process work presently use closed sums such as
[`PhysicalEvent`](../../crates/world-model/src/accepted/domain.rs) and
[`ScheduledWork`](../../crates/world-runtime/src/scheduler.rs). Fire, bleeding,
crafting, disease, construction, and economic T1 definitions cannot yet target
a general installed process surface. A genuinely new T3 process primitive may
still require explicit variants or another concrete protocol selected by
evidence.

#### Perception and social depth

Current
[post-commit routing](../../crates/world-engine/src/routing.rs) is
intentionally minimal, evidence provenance is specific to the implemented
transfer and relocation occurrences, and
[`SocialState`](../../crates/world-model/src/accepted/social.rs) is an existing
empty accepted-state partition. The social lifecycle port is disabled. Unequal
observation, contradiction, institutions, claims, practices, and obligations
remain conceptual.

#### Multi-resolution refinement

The target has the correct vision but has not yet selected concrete
conservation, observational, wake-replacement, and approximation obligations
for promotion and demotion.

### Audit verdict

The current architecture is not wrong. It is deliberately narrower than its
eventual gameplay claim.

The appropriate conclusion is:

```text
M1-M4 prove the authority and lifecycle spine.
They do not yet prove ordinary T0/T1 gameplay extensibility across several
installed primitives.
```

The next design should preserve the spine while replacing the seed's repeated
closed products only where an existing-vocabulary mechanic needs a broader
definition/dispatch surface. The T3 state-owner and protocol boundary should
be learned from orthogonal thin slices before a generic API is frozen.

## Alternatives considered

### Global ECS

ECS is excellent for:

- structural composition;
- cache-friendly iteration;
- entity identity and sparse/dense storage;
- parallel scheduling with declared component access.

It does not inherently define authority, actor-relative information, event
meaning, process persistence, replay, definition identity, cross-resolution
refinement, or semantic conflict. It may be used inside a state owner or
subsystem but should not be the architecture's stable waist.

### Mutable event bus

A mutable event bus makes local extension easy but introduces listener order,
reentrancy, cancellation cooperation, hidden write paths, and causal
ambiguity. Immutable authority records, domain events, and later scheduled
reactions retain decoupling without those semantics.

### Universal Datalog or production-rule engine

Datalog is ideal for pure relations and fixed points. It is awkward for
noncommutative mutation, numeric solvers, time-bearing state, private
qualification, and intentionally nonconfluent choices. It should power or
specify derived views, not own world transitions.

### Universal effect DSL or virtual machine

A common expression library and definition-specific checked IRs are useful. One
universal effect language will either remain too weak for new primitives or
grow into an unsafe general-purpose language with escape hatches and a second
authority path.

### Unrestricted runtime scripting

Runtime scripting maximizes reach but weakens static authority checks,
dependency analysis, deterministic replay, boundedness, migration, and AI
authoring safety. Trusted T3 code can exist, but it is linked into an exact
distribution rather than ambiently injected into a live session.

### One formalism for all mechanics

Rewriting logic, DEVS, Petri nets, linear logic, BDI, or algebraic effects can
each model a large part of the system. Using any one as the entire engine would
discard the distinctions that make the target coherent. The architecture
synthesis uses each as a local discipline where it fits.

## Recommended architecture refinements

These are the research recommendations submitted to architecture review. Their
current dispositions are recorded in the normative
[Gameplay-composition research disposition](../architecture/target-architecture/extensibility-and-research.md#gameplay-composition-research-disposition).
Some are now accepted laws or M8 evidence obligations; the universal semantic
dependency graph remains an investigation rather than an accepted runtime
contract. The rationale below does not supersede those dispositions.

### 1. Keep extension contracts tier-specific

T1 definition families should declare exact interfaces, stages, typed reads,
permitted disclosure, bounded outputs, and installed primitive operations.
They do not own accepted state.

For a proposed T3 primitive, use the extension dossier in this note to review
state ownership, invariants, definitions, views, proposals, validation,
footprints, domain events, scheduled work, codecs, migrations, and resolution
obligations.

Do not combine those two contracts into one large Rust trait.

### 2. Prove cross-primitive footprints in the existing transaction model

The normative `PreparedTransaction` already includes read, write, resource,
conflict, and invariant footprints plus participating gate receipts. The
implementation must prove that vocabulary across more than containment.

The medicine-during-destruction scenario should investigate the smallest typed
representation for cross-primitive contention. It must avoid string paths and
preserve owner-private validation, but this research does not preselect a
global resource registry.

### 3. Keep definition activation separate from state-owner composition

Extend the normative `ActivatedDefinitionRegistry` only as needed to
reconstruct compiled T1 definitions, indexes, caches, and exact semantic
bindings. Keep T3 state owners and protocols concrete and private by default.

A generalized type-erased state-owner or primitive dispatcher would be a new
architecture decision. The thin slices may motivate a narrower mechanism, but
this note does not select one.

### 4. Separate derivation semantics from transition semantics

Define:

- monotone fixed-point views;
- stratified nonmonotone views;
- reconstructible caches;
- owner-validated authoritative transitions.

Test incremental evaluation against full recomputation. Never permit a derived
rule to mutate accepted state.

### 5. Build the semantic dependency graph at link time

Use definition-specific nodes and typed edges for validation, invalidation,
explanation, compatibility, and AI authoring. Each node and edge must retain
its authority class, stage, permitted disclosure, and output audience. Do not
lower all mechanics into one runtime instruction language.

### 6. Prove the existing prepared-subtransaction contract across owners

The target already requires explicit transaction kinds, participating-gate
receipts, combined-invariant validation, and atomic commit. A domain transition
spanning several physical owners should provide implementation evidence for
that contract.

The proof must exercise:

- immutable base and read witnesses;
- canonical resource/invariant claims;
- partial commutative merge laws;
- explicit conflict dispositions;
- no semantic last-writer-wins;
- later, not recursive, consequences.

Epistemic, social, and agency consequences are not included merely for
convenience. They normally remain later transitions through their existing
authority classes unless a separately justified explicit multi-partition
transaction requires otherwise.

### 7. Retain semantic epochs as the evolution boundary

M5 persistence work remains correctly prioritized. A checkpoint must bind all
currently supported accepted state, control state, pending work, schemas,
exact definitions and implementations, and captured external results. It does
not need to generalize for primitives that do not exist; future primitives
enter through versioned child epochs and explicit migration policy.

AI-assisted T1 authoring should exercise an explicit child epoch after that
boundary is proven.

### 8. Validate before generalizing

Use these as the recommended post-kernel falsification suite:

```text
A. door / lock / tool / body capability
B. material / integrity / heat / combustion / smoke process
C. witness / ownership claim / institution / obligation
```

Make them interact in one causal scenario and in one same-moment contention
scenario. Only then consider freezing any shared primitive-installation,
resource, domain-event/process, or state-owner API. Adoption as a milestone
gate requires an explicit roadmap decision.

## Architectural fitness properties

The final design should continuously verify:

1. **Locality**

   Adding T0/T1 content changes definitions and tests, not primitive owners or
   the authority kernel. A T3 primitive may add concrete owners, protocols,
   schemas, codecs, and composition-root wiring without rewriting the
   authority laws or unrelated primitives.

2. **Ownership**

   Every accepted field and runtime-control record has one semantic owner.

3. **Noninterference**

   Under the target's D-032 relation, actor-indistinguishable authoritative
   states produce identical policy payloads, candidate IDs and order,
   projection fingerprints, and logical invocation timing.

4. **Determinism**

   Identical semantic epoch, initial state, commands, and captured external
   inputs produce identical authority history.

5. **Permutation invariance**

   Reordering independent proposal evaluation does not change the committed
   result.

6. **Explicit conflict**

   Incompatible deltas cannot silently merge or use persistence order as game
   semantics.

7. **Write confinement**

   Only state named by the prepared write footprint changes.

8. **Dependency completeness**

   Qualification, output, and invalidation depend only on declared reads,
   immutable semantics, and admitted captured input.

9. **Frame stability**

   Adding state disjoint from declared reads, writes, resources, and invariants
   cannot change the transition's result.

10. **Incremental equivalence**

   Incrementally maintained views equal full recomputation.

11. **Causal boundedness**

   A simulation moment cannot create an unbounded zero-time cascade.

12. **Replay purity**

    Replay performs no policy, AI, network, or other external evaluation.

13. **Epoch closure**

    Installing an unused implementation does not alter semantics, and a live
    epoch cannot acquire a new implementation.

14. **Resolution refinement**

    Coarse and detailed traces remain related under declared observations,
    conservation laws, and approximation bounds.

15. **No imperative pairwise coordination**

    Selected physical-to-perceptual-to-social interactions use shared typed
    state, records, and explicit bridge rules rather than direct subsystem
    calls, mutable callbacks, or handler order.

16. **Extension-tier conformance**

    Existing-vocabulary content and mechanics do not require trusted primitive
    or runtime-core changes.

## Roadmap consequence

The research does not justify abandoning the current roadmap or inserting a
large generalized gameplay framework between its foundation milestones. The
accepted order is:

```text
M5: checkpoint, restore, replay, branch, and delivery durability
  -> M6: CLI, authoring, experiment, inspection, and adapter product
  -> M7: individual multi-resolution evidence
  -> M8: gameplay composition proof before gameplay/API stabilization
```

M5 must preserve the exact schemas, implementation bindings, accepted/control
state, pending work, and captured external results of every currently
supported concrete type. It does not need a generalized future state-owner
registry.

M6's authoring and inspection surfaces make composition experiments
reproducible. M7 retains ownership of promotion/demotion and resolution
refinement. M8 now explicitly adopts the full three-slice suite as a
falsification gate:

```text
specify the smallest unresolved T1 or T3 boundary
  -> implement the three interacting research slices
  -> falsify hypotheses with simultaneous and multi-resolution scenarios
  -> freeze only the abstractions that survived
  -> expand content and definition-specific authoring languages
```

M8 is not permission to build the generalized registries rejected by this
research. It must prove T0/T1 locality, T3 owner locality, cross-owner prepared
transactions, write confinement, frame stability, and the selected
cross-system causal chain with concrete domain types. Only an incremental
derived view introduced by the slices owes incremental/full-recomputation
equivalence.

This extension preserves the foundation sequence while ensuring that M1-M4's
concrete seed enums and composition fields are not mistaken for a proven
general gameplay extension model.

## Final judgment

The research synthesis recommends understanding the target as:

> A deterministic superdense-time labeled transition system with transactional
> authority and a conceptual product of concrete, privately owned typed state
> representations. Pure relational derivations expose capabilities and
> actor-relative affordances.
> Replaceable rules, planners, and AI return bounded proposals. Owner-specific
> validators prepare compatible deltas for atomic commit within a typed
> authority transition. Commits emit typed causal work. Epistemic and social
> systems interpret observer-relative evidence without acquiring physical
> authority. Coarse models refine detailed ones under declared observational
> and conservation obligations. T0/T1 definitions are open through checked
> compilation, T3 primitives through explicit distribution composition, and
> all selected semantics are closed within a live epoch.

This is more expressive than ECS without becoming a universal symbolic
interpreter. It gives different gameplay domains the semantics they need while
preserving one small, stable law for authority, information, time, causality,
and replay.
