# Crate Boundary Architecture

## Status

Current architecture planning draft.

## Purpose

This document translates the logical engine architecture into candidate Rust
crate boundaries.

It answers:

```text
Which Rust crates should exist first?
Which dependencies point inward or outward?
Which crates own public authority boundaries?
Which logical components should remain modules inside a crate for now?
```

It is not:

- engine code
- a final `Cargo.toml`
- a final public API reference
- a module-by-module implementation plan
- a parser, persistence, ECS, or scripting selection document
- a vertical slice plan

The goal is to make crate boundaries enforce the architecture's authority and
dependency rules without splitting every logical component into a separate
public crate too early.

## Inputs

Primary architecture inputs:

- [Architecture Roadmap](roadmap.md)
- [Architecture Decisions](ADR.md)
- [Engine Architecture](engine.md)
- [Runtime Pipeline Architecture](runtime-pipeline.md)

Primary research inputs:

- [Implementation Architecture And Library Survey](../research/implementation-architecture-and-library-survey.md)
- [Runtime Pipeline Implementation Research](../research/runtime-pipeline-implementation-research.md)

Relevant Rust references:

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Cargo Features](https://doc.rust-lang.org/cargo/reference/features.html)
- [Rust API Guidelines: Type Safety](https://rust-lang.github.io/api-guidelines/type-safety.html)
- [Rust Book: References And Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)
- [Rust Book: Recoverable Errors With Result](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)
- [Serde Enum Representations](https://serde.rs/enum-representations.html)

## Thesis

Crates should follow authority and dependency direction, not one-to-one
logical component names.

The important split is:

```text
domain vocabulary
  -> checked definitions
  -> authoritative state and queries
  -> runtime mutation authority
  -> actor-relative context
  -> semantic decision work
  -> host orchestration
```

This keeps the architecture enforceable in Rust:

- semantic decision code cannot call causal commit directly
- authoring/parsing code cannot depend on live runtime mutation
- stores do not expose broad mutable authority to arbitrary systems
- optional accelerators stay behind leaf crates or private adapters
- public APIs expose domain-specific boundaries instead of a generic system
  framework

## Workspace Shape

Use a Cargo workspace with packages under `crates/`.

Candidate package names are hyphenated. Rust crate imports use underscores:

```text
package: world-core
crate:   world_core
```

Initial target workspace:

```text
crates/
  world-core/
  world-defs/
  world-model/
  world-runtime/
  world-context/
  world-decision/
  world-authoring/
  world-engine/
```

Workspace management rules:

- keep shared dependency versions in the root workspace manifest
- keep lints, package metadata, profiles, and patch policy at workspace root
- use one root lockfile
- do not make optional accelerators default dependencies of foundational crates
- prefer package-level features only for optional adapters, serialization
  helpers, diagnostics renderers, and tooling surfaces
- avoid feature flags that change authority rules

The root workspace can later add `xtask`, benches, examples, or adapter crates
without changing the core dependency direction.

## Dependency Graph

Allowed dependency direction:

```text
world-core
  <- world-defs
  <- world-model
  <- world-runtime
  <- world-context
  <- world-decision
  <- world-authoring
  <- world-engine
```

Expanded view:

```text
world-core

world-defs
  -> world-core

world-model
  -> world-core
  -> world-defs

world-runtime
  -> world-core
  -> world-defs
  -> world-model

world-context
  -> world-core
  -> world-defs
  -> world-model

world-decision
  -> world-core
  -> world-defs
  -> world-context

world-authoring
  -> world-core
  -> world-defs

world-engine
  -> world-core
  -> world-defs
  -> world-model
  -> world-runtime
  -> world-context
  -> world-decision
  -> world-authoring
```

Forbidden dependency edges:

```text
world-core -> any world-* crate
world-defs -> world-model or world-runtime
world-model -> world-runtime, world-context, or world-decision
world-runtime -> world-context or world-decision
world-context -> world-runtime
world-decision -> world-runtime
world-authoring -> world-model or world-runtime
```

The most important rule is:

```text
world-runtime does not depend on world-decision.
```

`world-engine` orchestrates the decision and runtime crates. This preserves the
boundary where appraisal and intent may propose or select, but only runtime
authority can validate and commit hard outcomes.

## Crates

### `world-core`

Owns low-level domain primitives shared by every other crate.

Likely contents:

- newtyped ids
- simulation time primitives
- authority class tags
- version anchors
- replay level
- provenance keys
- stable ordering keys
- small shared result and diagnostic reference types

Examples:

```text
EntityId
DefinitionId
EventRecordId
CausalTransactionId
ProcessInstanceId
ActivityId
ActorId
StoreCursor
QueryEpoch
SimulationTime
ReplayLevel
VersionAnchor
```

Allowed dependencies:

- small foundational Rust crates only when the type needs them
- optional serialization support where persistence requires it

Must not own:

- world stores
- query execution
- transaction staging
- parser syntax
- runtime orchestration
- semantic rules

Design notes:

- use newtypes, not raw `u64`, raw `String`, or raw ECS ids
- derive common traits where appropriate because downstream crates cannot add
  foreign trait impls later
- avoid large dependencies here; every crate pays for them

### `world-defs`

Owns checked engine definition types.

Likely contents:

- `DefinitionRegistry`
- `ActionDef`
- `ProcessDef`
- `Typed Effect Program` IR
- semantic declaration IR
- appraisal and intent template definitions
- social rule definitions
- content schema definitions
- definition version anchors

Allowed dependencies:

- `world-core`

Must not own:

- pack source parser implementation
- source syntax spans tied to a parser frontend
- live world state
- runtime mutation
- actor-relative query execution

Design notes:

- this is the runtime-facing normalized definition model
- `world-authoring` produces this crate's checked definitions
- `world-runtime`, `world-model`, `world-context`, and `world-decision` consume
  these definitions without depending on parser internals

### `world-model`

Owns authoritative and holder-relative state storage plus read surfaces.

Likely contents:

- `WorldModel`
- authority-class store families
- relation store families
- `EventHistoryStore`
- `RuntimeControlStore`
- social, chronology, epistemic, and appraisal record stores
- `DerivedViewRegistry`
- `QueryLayer`
- `KernelQuery`
- `ActorRelativeQuery`
- `SemanticContextQuery`
- `DebugQuery`
- invalidation package types

Allowed dependencies:

- `world-core`
- `world-defs`

Must not own:

- causal commit authority
- typed effect interpretation
- final intent scoring
- pack compilation
- ECS or graph identity as public ontology

Design notes:

- `WorldModel` hosts stores, but does not make every store directly mutable
- query APIs return ids, value snapshots, read tokens, or derived views
- broad `&mut WorldModel` access should stay inside authority gates
- `EventHistoryStore` is a committed-history facade, not a generated-history
  owner
- `RuntimeControlStore` stores runtime control state, but updates arrive
  through runtime-control gates or transaction-coupled updates

### `world-runtime`

Owns hard mutation discipline and durable runtime control execution.

Likely modules:

```text
action
transaction
effects
runtime_control
scheduler
process
resolution
reaction
outcome
```

Likely contents:

- `CausalRuntime`
- `CausalTransactionBuilder`
- `CausalTransactionGate`
- `TypedEffectInterpreter`
- `ActionRequest` lifecycle handling
- `ProcessTick` lifecycle handling
- `ReactionRequest` lifecycle handling
- `RuntimeControlUpdate`
- `AcceptedRuntimeControlUpdate`
- `ActivityTransition`
- `ProcessTransition`
- `Scheduler`
- `ScheduledWakeup`
- `DrainOutcome`
- `ProcessRuntime`
- `ResolutionRuntime`
- runtime outcomes and runtime infrastructure errors

Allowed dependencies:

- `world-core`
- `world-defs`
- `world-model`

Must not own:

- semantic appraisal
- final intent scoring
- actor memory policy
- pack parsing
- direct UI/editor/IO adapters
- generic public system scheduling framework

Design notes:

- keep `Scheduler`, `ProcessRuntime`, `ResolutionRuntime`,
  `TypedEffectInterpreter`, and `CausalRuntime` as modules at first
- do not split them into crates until their internal interfaces stabilize
- runtime code stages mutation through transaction or runtime-control APIs
- no semantic decision crate dependency is allowed
- ordinary gameplay outcomes are domain results, not infrastructure errors

### `world-context`

Owns actor-relative context projection and readable decision inputs.

Likely contents:

- `ObservationPipeline`
- `ObservedState`
- `ObservedEvent`
- `ActorContextPipeline`
- `CapabilitySet`
- `ActionRepertoire`
- `PerceivedAffordance`
- `EpistemicWorkingSet`
- `SocialContextView`
- context provenance and explanation summaries

Allowed dependencies:

- `world-core`
- `world-defs`
- `world-model`

Must not own:

- final intent selection
- hard validation
- transaction staging
- durable memory/social/appraisal writes without accepted update gates

Design notes:

- this crate converts stores and queries into actor-facing context
- it should not expose omniscient truth to decision code
- context results should be value-like snapshots or explicit read handles
- context invalidation follows `world-model` invalidation packages

### `world-decision`

Owns semantic interpretation and intent preparation.

Likely contents:

- appraisal variable derivation
- `Thought`
- `Pressure`
- `GoalPressure`
- candidate intent generation
- `CandidateIntent`
- `IntentScore`
- intent selection/suggestion interfaces
- activity preparation data
- decision explanations

Allowed dependencies:

- `world-core`
- `world-defs`
- `world-context`

Must not own:

- `CausalRuntime`
- `CausalTransaction`
- `EventRecord` append
- `RuntimeControlStore` mutation
- hard validation
- raw world-store access

Design notes:

- this crate may rank, suggest, or select according to actor/policy authority
- durable selected intent still becomes runtime control state through
  `world-runtime` or `world-engine` orchestration
- appraisal cannot execute and cannot mutate hard truth
- decision code consumes actor-visible context, not privileged kernel queries

### `world-authoring`

Owns pack authoring, verification, and source diagnostics.

Likely contents:

- pack compiler
- pack dependency graph
- source parser frontends when syntax exists
- semantic declaration verifier
- typed effect verifier
- definition lowering into `world-defs`
- source spans and authoring diagnostics

Allowed dependencies:

- `world-core`
- `world-defs`

Must not own:

- live `WorldModel`
- runtime mutation
- save/load state
- actor-relative runtime queries

Design notes:

- parser technology remains isolated here
- source syntax can change without forcing runtime crates to change
- diagnostics may use source-span libraries that should not leak into
  foundational runtime APIs
- authoring produces checked definitions, not executable callbacks with hidden
  authority

### `world-engine`

Owns the public facade and orchestration layer.

Likely contents:

- engine/session lifecycle
- pack loading and registry construction orchestration
- world creation/load/save coordination
- runtime tick and scheduler drain entry points
- controller input surfaces
- decision/context/runtime orchestration
- inspection entry points
- stable facade types for application users

Allowed dependencies:

- all inner `world-*` crates

Must not own:

- hidden hard mutation outside `world-runtime`
- duplicate store ownership outside `world-model`
- parser-specific or accelerator-specific public API as default surface

Design notes:

- this is the crate most applications should depend on first
- it can re-export selected stable types from inner crates
- it should not re-export every internal module
- async host integrations belong here or in later adapter crates, not inside
  the synchronous core runtime

## Public API Policy

Public API should be narrow and domain-specific.

Prefer:

```text
EngineSession::submit_player_command(...)
EngineSession::drain_until(...)
CausalRuntime::execute_action_request(...)
QueryLayer::actor_context(...)
DefinitionRegistry::lookup_action(...)
```

Avoid early generic extension APIs:

```text
trait RuntimeSystem {
  fn run(&mut self, world: &mut WorldModel);
}
```

Rules:

- foundational crates expose stable domain types, not broad mutation hooks
- crates may use `pub(crate)` or sealed traits for internal capability
  boundaries
- public plugin traits wait until the extension contract is stable
- `world-engine` may provide convenience re-exports, but inner crates remain
  the source of truth for their own types
- infrastructure errors use `Result<T, E>`; gameplay outcomes use domain
  outcome enums

## Internal Module Co-Location

Do not create a crate for every logical component yet.

Keep these together inside `world-runtime` initially:

- request binding
- causal transaction staging
- typed effect handling
- scheduler drain
- process runtime
- runtime-control gate
- resolution runtime
- reaction handling

Reason:

These roles share internal transaction, process, wakeup, reservation, and
runtime-control vocabulary. Splitting them too early would force unstable
internal shapes into public crate APIs.

Keep these together inside `world-context` initially:

- observation projection
- actor context assembly
- capability derivation
- affordance derivation
- social context view assembly
- epistemic working set projection

Reason:

These roles produce actor-relative readable context. They should share access
filtering and provenance rules without giving decision code privileged store
access.

Keep these together inside `world-decision` initially:

- semantic appraisal
- pressure production
- candidate intent generation
- intent scoring
- selection/suggestion explanations

Reason:

These roles form the decision middle-end, but they must remain outside runtime
mutation authority.

## Feature And Dependency Policy

Feature flags are allowed for leaf capabilities, not for changing core
authority.

Allowed feature classes:

```text
serde:
  serialization derives or adapters for stable data types

diagnostics:
  source diagnostics renderers and rich error reporting

inspect:
  extra debug and explanation surfaces

adapter-*:
  ECS, graph, Datalog-like, scripting, Wasm, editor, or host integrations
```

Avoid:

- default features that pull in ECS, scripting, async runtime, or parser stacks
  into foundational crates
- mutually exclusive features on foundational crates
- feature combinations that change whether `CausalTransaction` is required
- feature combinations that let semantic declarations mutate hard truth

Optional accelerator crates can be added later:

```text
world-ecs-adapter
world-graph-adapter
world-datalog-adapter
world-script-adapter
world-wasm-plugin
world-inspect
world-persistence
world-test-support
world-xtask
```

These should depend outward from the core crates. Core crates should not depend
on them.

Deep game-system packs may use ECS-backed local projections or adapter crates
for high-volume concrete simulation, as long as accepted outcomes still return
through `ActionRequest`, `ProcessTick`, `RuntimeControlUpdate`, or
`CausalTransaction`.

## Error And Outcome Policy

Keep gameplay outcomes separate from infrastructure errors.

Runtime APIs should tend toward:

```text
Result<RuntimeOutcome, RuntimeError>
```

`RuntimeOutcome` examples:

```text
Rejected
Blocked
AttemptFailed
Interrupted
ConflictResolved
Committed
AbortedWithNoCommit
```

`RuntimeError` examples:

```text
missing checked definition
malformed checked data
violated engine invariant
IO or serialization failure
version incompatibility
corrupted save or registry
```

Authoring APIs should tend toward diagnostics collections rather than panic or
string-only errors:

```text
Result<CheckedDefinitions, AuthoringDiagnostics>
```

This keeps failed gameplay attempts inspectable without treating ordinary
simulation outcomes as Rust failures.

## Implementation Order

Recommended crate implementation order:

1. `world-core`
2. `world-defs`
3. `world-model`
4. `world-runtime`
5. `world-context`
6. `world-decision`
7. `world-authoring`
8. `world-engine`

Reason:

- ids and version anchors must exist before definitions and stores
- checked definitions must exist before runtime can bind requests
- model/query surfaces must exist before transaction staging and context
  projection
- runtime authority should be established before semantic decision integration
- authoring can initially construct definitions directly before final source
  syntax exists
- `world-engine` is most useful after the inner crate contracts are visible

Early implementation may use internal modules before every crate is populated,
but the dependency direction should be preserved from the start.

## Deferred

Defer until implementation planning:

- exact `Cargo.toml` contents
- exact Rust module tree
- exact public function names
- exact persistence backend
- exact parser crate choice
- exact ECS or graph adapter
- exact async runtime, if any host adapter needs one
- exact benchmark and fuzzing crates
- exact CI command matrix

Do not defer:

- dependency direction
- public mutation authority boundaries
- separation between definitions, model, runtime, context, decision, authoring,
  and engine facade
- keeping accelerators out of the source-of-truth path

## Summary

The target crate structure is:

```text
world-core
world-defs
world-model
world-runtime
world-context
world-decision
world-authoring
world-engine
```

This is intentionally coarser than the logical component map and narrower than
a single monolithic engine crate. It gives Rust enough crate-level boundaries
to protect authority and dependency direction while leaving unstable internal
runtime roles free to evolve as modules.

## Next Document

The high-level implementation plan now lives in
[Implementation Plan](implementation-plan.md). Detailed phase-local plans
should be written only when implementation begins for that phase.
