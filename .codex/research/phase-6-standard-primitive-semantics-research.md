# Phase 6 Standard Primitive Semantics Research

## Status

Research note for Phase 6 preparation.

This is not a concrete implementation plan. It records local architecture
constraints, external reference pressure, code-level architecture options, and
Phase 7-9 co-design implications for the next `.codex/plans` document.

## Scope

Phase 6 concerns the reusable primitive layer between runtime mechanism and
game-specific packs:

- checked primitive definitions
- typed effect operations that reference checked primitive definitions
- runtime-owned primitive semantics lookup
- pure standard-world definition bundles
- trusted standard runtime semantics installers
- definition/semantics compatibility verification
- migration of current hardcoded runtime builtins into the registry shape

The core question is not whether to use "compiler ideas" conceptually. The
question is which compiler/runtime patterns should become real code structure
without turning the engine into a generic compiler framework or plugin system.

## Local Contract

The local docs already converge on one boundary:

```text
world-defs:
  checked runtime-facing definition data

world-runtime:
  transaction authority, staging capabilities, runtime validation, semantics
  registry lookup, and commit

world-standard:
  pure reusable world vocabulary and primitive definition bundles

world-standard-runtime:
  trusted executable semantics installers for standard primitives

world-engine:
  selected bundle and runtime wiring
```

Important local anchors:

- `docs/design/standard-world-library.md` defines the definition/semantics split
  and says the runtime core should not absorb growing physical, damage,
  condition, signal, resource, or process vocabulary.
- `docs/design/typed-effect-primitives.md` says typed effect primitives are the
  checked hard-mutation call surface, while runtime owns staging and commit
  authority.
- `docs/design/simulation-transition-compiler.md` frames authored declarations
  as checked, staged representations and explicitly calls for dialect-like
  boundaries with owned vocabulary, type rules, verifier, provenance
  expectations, and lowering/query boundaries.
- `docs/architecture/runtime-pipeline.md` already names the intended flow:
  `TypedEffectInterpreter -> PrimitiveSemanticsRegistry lookup ->
  CausalTransactionBuilder -> staged reads/reservations/control/RNG/mutations/
  events/schedules`.
- `docs/architecture/crates.md` already allows `world-context` to depend on
  pure `world-standard`, but forbids `world-runtime -> world-standard` and
  forbids `world-context -> world-standard-runtime`.

The current code is pre-Phase-6 in the expected way:

- `EffectOp` in `crates/world-defs/src/effects.rs` still stores `EffectKind`,
  stage permissions, and emitted events directly.
- `DefinitionRegistry` in `crates/world-defs/src/registry.rs` has effect
  programs, actions, processes, and semantic declarations, but no primitive
  definition table or builder/install surface yet.
- `world-runtime` dispatches through `BuiltinEffect::from_operation(...)` and
  string-like operation names in `crates/world-runtime/src/builtin.rs`.
- `RuntimeValidator` and `TypedEffectInterpreter` both call `BuiltinEffect`
  directly.
- `CausalRuntime` owns a `DefinitionRegistry` and a `TypedEffectInterpreter`,
  but no `PrimitiveSemanticsRegistry`.
- `StageContext` and `ValidationContext` currently expose role helpers tied to
  `BuiltinRole`, which will not scale once primitive signatures come from
  installed definitions.

## External Reference Pressure

### MLIR Dialects And Operations

MLIR's dialect model is the strongest architectural reference for Phase 6, but
the lesson is structural rather than technological. The engine should not embed
MLIR or create an MLIR clone.

Useful MLIR patterns:

- Dialects group operations, attributes, and types around an owned abstraction
  boundary. See [MLIR Defining Dialects](https://mlir.llvm.org/docs/DefiningDialects/).
- ODS makes operation facts declarative: operation name, operands, attributes,
  properties, results, traits, interfaces, builders, and verifier hooks. See
  [MLIR Operation Definition Specification](https://mlir.llvm.org/docs/DefiningDialects/Operations/).
- Traits and interfaces let generic analyses ask narrow questions about
  operations without knowing every concrete op. See
  [MLIR Traits](https://mlir.llvm.org/docs/Traits/) and
  [MLIR Interfaces](https://mlir.llvm.org/docs/Interfaces/).
- Symbol tables separate definition from symbolic reference and enforce scoped
  uniqueness/resolution. See
  [MLIR Symbols and Symbol Tables](https://mlir.llvm.org/docs/SymbolsAndSymbolTables/).
- The pass infrastructure runs typed analyses/transforms over specific operation
  constraints rather than a universal untyped callback. See
  [MLIR Pass Infrastructure](https://mlir.llvm.org/docs/PassManagement/).
- Side-effect modeling treats hidden mutable effects as first-class operation
  metadata, including effect resources and stages. See
  [MLIR Side Effects & Speculation](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/).

Mapping to this engine:

```text
MLIR dialect
  -> standard definition bundle / future trusted primitive extension bundle

MLIR operation definition
  -> EffectPrimitiveDef

MLIR operation instance
  -> EffectOp in a checked EffectProgramDef

MLIR symbol reference
  -> EffectPrimitiveId / DefinitionId reference from EffectOp

MLIR op traits/interfaces
  -> primitive contract metadata such as permissions, resource effects,
     event requirements, replay requirements, and process-control effects

MLIR verifier / pass pipeline
  -> DefinitionRegistry and semantics-coverage verification
```

The important code-level consequence is that `EffectOp` should not be a raw
operation-name string with copied permission/event metadata. It should reference
a checked primitive definition, and the registry/verifier should derive or check
permissions, event contracts, typed arguments, and semantics requirements from
that definition.

### rustc Queries And Salsa

`rustc` and Salsa are useful for dependency discipline, not for adopting an
incremental query engine in Phase 6.

Useful patterns:

- rustc query providers are registered function tables, not a broad public trait
  hierarchy; the query key determines where the provider applies. See
  [rustc-dev-guide: queries](https://rustc-dev-guide.rust-lang.org/query.html).
- rustc incremental compilation depends on pure query functions and an explicit
  dependency graph. See
  [rustc-dev-guide: incremental compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html).
- Salsa keeps mutation of inputs outside the deterministic computation and
  reuses computations through a database. See
  [Salsa overview](https://salsa-rs.github.io/salsa/overview.html).

Mapping to this engine:

```text
provider table / registry
  -> PrimitiveSemanticsRegistry keyed by EffectPrimitiveId

pure query dependency graph
  -> future authoring/diagnostic/incremental verification dependency model

input mutation outside deterministic computation
  -> runtime staging capabilities and accepted commit gates, not arbitrary
     mutation inside primitive handlers
```

Phase 6 should record enough primitive definition metadata to make Phase 9
authoring verification and future incremental diagnostics possible. It should
not introduce Salsa or an incremental dependency engine yet.

### Rust API Design

Trait-based APIs are appropriate here, but only if the traits are narrow and
match actual extension boundaries.

Useful Rust references:

- A dyn-compatible trait cannot require `Self: Sized`, cannot have associated
  constants, and dispatchable functions cannot be generic or use `Self` except
  through the receiver. See
  [Rust Reference: dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility).
- Newtypes provide static distinctions between different interpretations of the
  same representation. See
  [Rust API Guidelines: type safety](https://rust-lang.github.io/api-guidelines/type-safety.html).
- Sealed traits are useful when downstream implementation would freeze an API too
  early. See
  [Rust API Guidelines: future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html).
- Meaningful error types should implement normal error traits and carry domain
  context. See
  [Rust API Guidelines: interoperability](https://rust-lang.github.io/api-guidelines/interoperability.html).

Code-level implication:

- `EffectPrimitiveId` should be a newtype around `DefinitionId`, not a bare
  alias, if primitive ids need distinct APIs from action/process/effect-program
  ids.
- Runtime handler traits intended for `dyn` dispatch should avoid associated
  constants and generic methods.
- Definition bundle traits may be public and implemented by trusted/pure crates.
  Runtime semantics traits should stay narrow and may need sealing or explicit
  rustdoc warnings if the project wants to avoid advertising ordinary pack-level
  implementation.
- Errors such as duplicate primitive definition, missing primitive semantics, and
  definition/semantics mismatch should be structured errors, not strings.

### Bevy Plugins And Game Runtime Modularity

Bevy is useful as a modular assembly reference: plugins are explicitly added to
an application and configure it. See
[Bevy Plugins](https://bevy.org/learn/quick-start/getting-started/plugins/) and
[bevy::app::Plugin](https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html).

What to borrow:

- explicit installation
- small bundles of related functionality
- duplicate/uniqueness checks
- default bundle versus custom bundle composition

What not to borrow:

- broad app mutation authority for ordinary game packs
- a generic public system scheduler as the main runtime abstraction
- giving extension code raw world mutation authority

For this repo, the Bevy-like part should be the assembly surface:

```text
definition_builder.install(&StandardWorldDefinitions)?;
semantics_builder.install(&StandardWorldRuntimeSemantics)?;
```

The runtime part should remain capability-gated:

```text
PrimitiveSemantics::stage(invocation, &mut StageContext)
```

where `StageContext` exposes only declared, audited staging operations.

### Temporal Replay And Durable Execution

Temporal is a useful reference for durable replay boundaries. Workflow execution
records commands and events into history, and replay checks generated commands
against existing history. See
[Temporal Workflows](https://docs.temporal.io/workflows) and
[Temporal Workflow Execution](https://docs.temporal.io/workflow-execution).

Mapping to this engine:

- committed `EventRecord`s are hard facts and audit/projection inputs, not
  transient event-bus messages;
- primitive semantics should use runtime APIs for time, RNG, scheduling,
  reservations, and event emission;
- non-determinism that affects committed outcomes must be captured through
  provenance/replay metadata;
- replay-like validation should not re-run external side effects or arbitrary
  plugin callbacks.

Temporal should not be copied literally. This engine already chose materialized
stores plus event history, not full command-history replay as the only source of
truth.

## Recommended Target Architecture

### Definition Layer

`world-defs` should own parser-free checked primitive definitions:

```rust
pub struct EffectPrimitiveId(DefinitionId);

pub struct EffectPrimitiveDef {
    id: EffectPrimitiveId,
    name: DefinitionName,
    params: Vec<EffectParamDef>,
    required_permissions: BTreeSet<StagePermission>,
    effects: Vec<PrimitiveResourceEffect>,
    event_contract: EventContract,
    replay_level: ReplayLevel,
    version: VersionAnchor,
}

pub struct EffectOp {
    primitive: EffectPrimitiveId,
    args: Vec<EffectArgBinding>,
    emitted_events: BTreeSet<EventRecordSpec>,
}
```

The exact field names can change in the implementation plan. The important
shape is:

- primitive definition owns the signature and contract;
- operation instance references the primitive and supplies invocation-specific
  arguments/events;
- operation names stay useful for diagnostics, but not for runtime dispatch;
- permissions and event requirements should be checked against definitions, not
  freely copied into every operation as unchecked authority claims.

`DefinitionRegistry` should gain primitive definitions and likely a builder:

```rust
DefinitionRegistryBuilder
  -> install definition bundles
  -> validate unique ids and names
  -> validate action/process/effect-program references
  -> validate primitive arg/event/permission contracts
  -> build DefinitionRegistry
```

The builder is the native Rust equivalent of a compiler symbol table plus
verification pipeline. It avoids forcing callers to construct a large registry
with many parallel vectors after standard bundles exist.

### Pure Standard Library

`world-standard` should contain pure data and helper constructors only:

```text
crates/world-standard/src/
  lib.rs
  ids.rs
  events.rs
  primitives/
    mod.rs
    physical.rs
    reservation.rs
    process.rs
  bundle.rs
```

Likely first bundle content:

- stable primitive ids and names
- event specs/families used by the first primitives
- definitions for current seed primitives:
  - `insert_entity` or a more domain-shaped creation primitive
  - `transfer_entity`
  - `acquire_reservation`
  - `record_event` only if deliberately kept as a primitive rather than a
    staging helper
- narrow room for later physical primitives such as `move_entity`,
  `apply_damage`, `add_condition`, `emit_signal`, and `schedule_process`

`world-standard` must not depend on `world-runtime` or `world-model`. It is safe
for Phase 7 actor context and Phase 9 authoring to depend on it.

### Runtime Semantics Layer

`world-runtime` should own the registry and capability traits:

```rust
pub trait PrimitiveSemantics: Send + Sync + 'static {
    fn primitive(&self) -> EffectPrimitiveId;

    fn validate(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveValidationContext<'_>,
    ) -> Result<(), RuntimeValidationFailure>;

    fn stage(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveStageContext<'_, '_, '_, '_>,
    ) -> Result<(), RuntimeError>;
}

pub trait PrimitiveSemanticsInstaller {
    fn install_semantics(
        &self,
        builder: &mut PrimitiveSemanticsRegistryBuilder,
    ) -> Result<(), RuntimeError>;
}

pub struct PrimitiveSemanticsRegistry {
    handlers: BTreeMap<EffectPrimitiveId, Box<dyn PrimitiveSemantics>>,
}
```

This trait should be small and capability-based. It is not a `RuntimeSystem`
trait. It should not receive `&mut WorldModel`, raw commit constructors, raw
runtime-control apply APIs, or arbitrary host IO handles.

`PrimitiveInvocation` should expose the checked `EffectOp`, resolved
`EffectPrimitiveDef`, and typed role/arg helpers. It should replace the current
`BuiltinRole` dependency in runtime validation and staging.

`PrimitiveSemanticsRegistryBuilder::build_against(&DefinitionRegistry)` should
check:

- every handler primitive id exists as an `EffectPrimitiveDef`;
- duplicate handlers are rejected;
- handler-declared primitive identity matches the definition being installed;
- required standard definitions are present when installing standard handlers;
- every primitive used by executable effect programs has installed semantics, at
  least for runtime sessions that intend to execute those programs.

### Standard Runtime Library

`world-standard-runtime` should implement trusted handlers and installers:

```text
crates/world-standard-runtime/src/
  lib.rs
  physical/
    mod.rs
    create.rs
    transfer.rs
  reservation.rs
  events.rs
  bundle.rs
```

It may depend on:

- `world-core`
- `world-defs`
- `world-model`
- `world-runtime`
- `world-standard`

It should not own parser/source syntax, engine session lifecycle, actor context,
decision work, arbitrary plugin loading, or direct store mutation outside
runtime staging capabilities.

### Runtime Flow

The intended runtime flow after Phase 6:

```text
CausalRuntime construction:
  DefinitionRegistryBuilder
    -> install world-standard definitions
    -> install game/test definitions
    -> build checked DefinitionRegistry

  PrimitiveSemanticsRegistryBuilder
    -> install world-standard-runtime semantics
    -> build_against DefinitionRegistry

  CausalRuntime::for_empty_model(definitions, semantics)

Execution:
  RuntimeRequest
    -> bind roles
    -> ActionDef / EffectProgramDef lookup
    -> RuntimeValidator
       -> for each EffectOp:
          -> primitive definition lookup
          -> semantics handler lookup
          -> validate through PrimitiveValidationContext
    -> CausalTransactionBuilder
    -> TypedEffectInterpreter
       -> for each EffectOp:
          -> primitive definition lookup
          -> semantics handler lookup
          -> stage through PrimitiveStageContext
    -> CommitFinalizer
    -> WorldModel::apply_hard_commit
```

This preserves the Phase 4/5 authority waist while moving world vocabulary out
of `world-runtime`.

## Co-Design With Later Phases

### Phase 7 Actor Context Projection

Phase 7 needs standard vocabulary without runtime mutation authority.

Therefore Phase 6 should expose pure standard ids, categories, event specs, and
primitive definitions from `world-standard`. Actor context can use that to derive
capabilities and affordances such as:

- reachable item from containment/equipment vocabulary;
- lock/open/passability affordances;
- wounded hand reducing manipulation capability;
- smoke/signal/field affecting perception;
- condition/resource/process vocabulary used as context facts.

Phase 7 must not depend on `world-standard-runtime`.

### Phase 8 Semantic Decision Middle-End

Phase 8 needs primitive/event facts as inputs to appraisal and intent, but it
must not execute effects or mutate hard truth.

Phase 6 should therefore keep `EventRecord`s as hard facts and avoid turning
events into hidden commands. Semantic decision code should receive actor-relative
context, observed event records, capabilities, and action repertoires. It should
not receive `PrimitiveStageContext` or the semantics registry.

### Phase 9 Authoring And Verification

Phase 9 needs stable checked IR targets:

- primitive definition ids and names;
- effect operation signatures and typed args;
- event contracts;
- stage permissions and resource effects;
- bundle dependency/version metadata;
- missing/duplicate/mismatched semantics diagnostics.

Phase 6 should not choose the final source syntax, parser, or diagnostics
renderer. It should create definition and verification targets that authoring can
lower into later.

Future incremental authoring can borrow rustc/Salsa dependency ideas, but Phase
6 only needs to preserve enough identity and dependency metadata to make that
possible.

## Main Risks

- Keeping `EffectKind` as the runtime dispatch key will preserve stringly typed
  builtins and make Phase 6 mostly cosmetic.
- Copying permissions/events directly into every `EffectOp` without checking
  against `EffectPrimitiveDef` lets authored programs overclaim authority.
- A broad public `RuntimeSystem` or `CompilerPass` trait would hide authority
  boundaries and contradict the repo's local planning rules.
- Letting `world-runtime` depend on `world-standard` would invert the intended
  crate boundary and make standard vocabulary part of runtime core.
- Letting ordinary game-system packs implement `PrimitiveSemantics` would turn
  pack source into trusted executable mutation code too early.
- A fallback like `SetField`, `AddTag`, or unknown-primitive generic mutation
  would defeat the purpose of typed primitive semantics.
- Separate `validate` and `stage` logic can diverge; each installed primitive
  needs tests that cover validation rejection, staging success, event emission,
  and definition/semantics mismatch.
- Runtime event listeners that mutate during commit would recreate an authority
  bypass. Reactions should enqueue later work through runtime-control gates.

## Recommended Carry-Forward Decisions

1. Use trait-based APIs from the start, but only at the real boundaries:
   `DefinitionBundle`/`PrimitiveDialect` for pure definitions and
   `PrimitiveSemantics`/`PrimitiveSemanticsInstaller` for trusted runtime
   handlers.
2. Keep the traits narrow, object-safe where dispatch is needed, and
   capability-based.
3. Introduce `EffectPrimitiveDef` and `EffectPrimitiveId` before migrating
   runtime dispatch.
4. Change checked `EffectOp` to reference primitive definitions rather than raw
   `EffectKind`.
5. Add a registry builder or equivalent assembly surface so standard definitions
   and later pack definitions install through one checked path.
6. Add `PrimitiveSemanticsRegistry` to `world-runtime` and require
   `CausalRuntime` construction to receive or build an explicit semantics
   registry.
7. Move current `BuiltinEffect` behavior into `world-standard-runtime` handlers
   instead of growing it inside `world-runtime`.
8. Replace `BuiltinRole`-specific context helpers with primitive-signature-driven
   role/arg lookup.
9. Keep standard primitive set small in Phase 6. Migrate existing seed behavior
   and add only enough standard vocabulary to prove the architecture.
10. Defer external plugin loading, signing, Wasm/scripting, final taxonomies,
    source syntax, and incremental authoring.

## Reference Index

Local:

- `docs/architecture/implementation-plan.md`
- `docs/architecture/crates.md`
- `docs/architecture/runtime-pipeline.md`
- `docs/design/standard-world-library.md`
- `docs/design/typed-effect-primitives.md`
- `docs/design/simulation-transition-compiler.md`
- `docs/design/pack-authoring-and-semantic-declarations.md`
- `crates/world-defs/src/effects.rs`
- `crates/world-defs/src/registry.rs`
- `crates/world-runtime/src/builtin.rs`
- `crates/world-runtime/src/transaction/effects.rs`
- `crates/world-runtime/src/transaction/validation.rs`
- `crates/world-runtime/src/runtime.rs`

External:

- [MLIR Defining Dialects](https://mlir.llvm.org/docs/DefiningDialects/)
- [MLIR Operation Definition Specification](https://mlir.llvm.org/docs/DefiningDialects/Operations/)
- [MLIR Traits](https://mlir.llvm.org/docs/Traits/)
- [MLIR Interfaces](https://mlir.llvm.org/docs/Interfaces/)
- [MLIR Symbols and Symbol Tables](https://mlir.llvm.org/docs/SymbolsAndSymbolTables/)
- [MLIR Pass Infrastructure](https://mlir.llvm.org/docs/PassManagement/)
- [MLIR Side Effects & Speculation](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/)
- [rustc-dev-guide: queries](https://rustc-dev-guide.rust-lang.org/query.html)
- [rustc-dev-guide: incremental compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html)
- [Salsa overview](https://salsa-rs.github.io/salsa/overview.html)
- [Rust Reference: dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)
- [Rust API Guidelines: type safety](https://rust-lang.github.io/api-guidelines/type-safety.html)
- [Rust API Guidelines: future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)
- [Rust API Guidelines: interoperability](https://rust-lang.github.io/api-guidelines/interoperability.html)
- [Bevy Plugins](https://bevy.org/learn/quick-start/getting-started/plugins/)
- [bevy::app::Plugin](https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html)
- [Temporal Workflows](https://docs.temporal.io/workflows)
- [Temporal Workflow Execution](https://docs.temporal.io/workflow-execution)
