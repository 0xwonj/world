# Phase 6 Local Plan: Standard World Library And Primitive Semantics

## Status

Draft implementation plan for Phase 6.

This plan should be reviewed before implementation. It is intentionally concrete
about module shape, public API direction, and verification gates, but it should
not be treated as a final permanent API reference.

This version assumes the pre-Phase-6 architecture alignment has already landed:
`world-defs` effects and registry code are module directories,
`DefinitionRegistryBuilder` exists and is used by `DefinitionRegistry::new(...)`,
runtime validation/staging contexts no longer expose `BuiltinRole`-typed APIs,
and current runtime effect-kind test fixtures are centralized.

## Purpose

Move reusable primitive world semantics out of `world-runtime` while preserving
the runtime as the mutation authority.

The phase target is:

```text
Checked definition assembly
  -> EffectPrimitiveDef table
  -> EffectOp references primitive ids, not stringly builtin names
  -> PrimitiveSemanticsRegistry built against DefinitionRegistry
  -> CausalRuntime executes checked primitive invocations through trusted,
     capability-limited semantics handlers
```

The phase is complete when:

- `world-defs` has first-class primitive definitions and checked operation
  references;
- `world-standard` supplies pure standard primitive definitions without runtime
  authority;
- `world-runtime` owns primitive semantics lookup and capability-gated dispatch;
- `world-standard-runtime` supplies trusted handlers for the first standard
  primitives;
- current `BuiltinEffect` string dispatch is removed from production runtime
  code;
- runtime execution works through the registry shape;
- missing, duplicate, or mismatched primitive semantics fail clearly;
- later actor context can depend on pure standard vocabulary without depending
  on runtime semantics installers.

## Research Inputs

Primary local research:

- `.codex/research/phase-6-standard-primitive-semantics-research.md`
- `.codex/research/phase-4-5-runtime-research.md`

Primary local architecture/design:

- `docs/architecture/implementation-plan.md`
- `docs/architecture/crates.md`
- `docs/architecture/runtime-pipeline.md`
- `docs/design/standard-world-library.md`
- `docs/design/typed-effect-primitives.md`
- `docs/design/simulation-transition-compiler.md`
- `docs/design/pack-authoring-and-semantic-declarations.md`

External patterns to apply selectively:

- MLIR dialects/ODS: primitive definitions are checked operation facts grouped
  into dialect-like bundles.
- MLIR symbol tables/verifiers: operation references are resolved and checked by
  registry verification, not by runtime string matching.
- MLIR side-effect modeling: primitive contracts should expose the capabilities
  they require instead of hiding mutation authority.
- rustc/Salsa query discipline: use explicit registries and checked providers;
  do not introduce an incremental query engine in this phase.
- Rust trait-object rules: registry-dispatched semantics traits must be
  object-safe.
- Bevy plugin pattern: explicit bundle/installer registration is useful;
  broad app/world mutation authority is not.
- Temporal replay discipline: durable effects go through controlled APIs and
  committed records, not arbitrary callbacks.

## Baseline Decisions

### Do Not Prebuild `world-engine`

`world-engine` remains a Phase 10 facade/integration crate.

Do not add session lifecycle, pack loading, runtime orchestration, or public
engine APIs in this phase. If implementation needs assembly examples, use crate
tests or small helper functions in the relevant crate, not a premature engine
facade.

### Preserve Crate Direction

Phase 6 may add `world-standard` and `world-standard-runtime` to the workspace if
they do not exist yet.

Allowed dependency direction for this phase:

```text
world-core
  <- world-defs
  <- world-model
  <- world-runtime
  <- world-standard-runtime

world-standard
  -> world-core
  -> world-defs

world-standard-runtime
  -> world-core
  -> world-defs
  -> world-model
  -> world-runtime
  -> world-standard
```

Forbidden new edges:

```text
world-runtime -> world-standard
world-runtime -> world-standard-runtime
world-model -> world-standard
world-model -> world-standard-runtime
world-context -> world-standard-runtime
world-decision -> world-runtime
```

This list is a Phase 6 emphasis, not a replacement for the full crate-boundary
contract in `docs/architecture/crates.md`. Once the standard crates exist, add a
dependency-direction guardrail for the full forbidden-edge set.

For this implementation, `world-model -> world-standard` stays forbidden. Model
stores receive checked ids and accepted packages; they do not learn which
standard vocabulary is installed.

`world-runtime` owns the registry and staging APIs. Standard crates install into
that registry from outside runtime core.

### Trait-Based, But Narrow

Use traits at the actual extension boundaries:

- `DefinitionBundle` in `world-defs` for pure checked definition installation;
- `PrimitiveSemantics` in `world-runtime` for trusted executable primitive
  semantics;
- `PrimitiveSemanticsInstaller` in `world-runtime` for installing handlers into
  a registry builder.

Do not introduce broad traits such as:

```text
RuntimeSystem
CompilerPass
WorldPlugin
EffectExecutor
```

Traits should encode real API contracts, not framework language.

`DefinitionBundle` is a pure definition-install boundary. Ordinary packs may
later be compiled into checked definitions, but they should not become Rust
callbacks through this trait.

### Primitive Semantics Are Trusted Extensions

Ordinary game-system packs compose installed primitive definitions through
checked `Typed Effect Program`s.

They do not implement `PrimitiveSemantics`, receive `StageContext`, or mutate
stores directly.

Future trusted extension packages can follow the same dependency shape as
`world-standard-runtime`, but package loading/signing and sandboxed execution are
not Phase 6 work.

### Keep The First Standard Bundle Small

Phase 6 proves the architecture by migrating the current seed behavior and
adding only enough pure standard vocabulary to make that migration meaningful.

Do not implement final damage, wound, condition, material, resource, signal,
field, spell, combat, crafting, or social taxonomies in this phase.

### No Legacy Completion State

Compatibility shims are acceptable during implementation only as temporary
migration scaffolding. By the Phase 6 exit gate, there must be one canonical
architecture for primitive execution.

Do not finish Phase 6 with two supported ways to do the same thing. In
particular:

- no production `BuiltinEffect` or `BuiltinRole` dispatch path;
- no runtime fallback from missing primitive semantics to string matching or
  generic mutation;
- no parallel `DefinitionRegistry` verifier paths;
- no public compatibility API that lets callers bypass primitive definitions,
  semantics registry lookup, or staging capabilities;
- no tests that keep old behavior alive except as explicit rejection/guardrail
  coverage.

## Current Baseline After Pre-Phase-6 Alignment

Phase 6 should extend the already-cleaned structure rather than redo setup work.

- `world-defs/src/effects/` already owns `StagePermission`, `EffectOp`, and
  `EffectProgramDef`.
- `world-defs/src/registry/` already owns `DefinitionRegistryBuilder` and
  shared registry verification.
- `DefinitionRegistry::new(...)` already delegates to the builder; do not add a
  second verifier path.
- `world-runtime` validation and staging contexts already take domain
  `RoleName` values instead of `BuiltinRole`.
- staging capability methods already enforce the current hard mutation,
  reservation acquisition, and event emission permissions where the current
  model can express them.
- runtime tests already centralize current executable effect-kind fixtures and
  distinguish action-executed seed effects from process-definition metadata.

## Target Module Shape

### Workspace

Add missing crates:

```text
crates/
  world-standard/
  world-standard-runtime/
```

Keep `world-engine` untouched except for workspace membership if it is already
present.

### `world-defs`

Target shape:

```text
crates/world-defs/src/
  lib.rs
  actions.rs
  error.rs
  events.rs
  keys.rs
  processes.rs
  registry/
    mod.rs
    builder.rs
    validate.rs
  effects/
    mod.rs
    permissions.rs
    primitive.rs
    program.rs
    args.rs
  roles.rs
  semantics.rs
  tests.rs
```

The `effects/` and `registry/` directories already exist. Preserve crate-root
re-exports so callers use `world_defs::EffectOp` and
`world_defs::DefinitionRegistryBuilder`, not deep internal module paths.

Ownership:

- `effects/primitive.rs`: `EffectPrimitiveId`, `EffectPrimitiveDef`,
  primitive signature and contract types.
- `effects/args.rs`: minimal typed invocation argument model.
- `effects/permissions.rs`: `StagePermission` and small access metadata if
  introduced.
- `effects/program.rs`: `EffectOp`, `EffectProgramDef`.
- `registry/builder.rs`: existing `DefinitionRegistryBuilder`, plus
  `DefinitionBundle`.
- `registry/validate.rs`: cross-definition verifier functions.

### `world-runtime`

Target additions:

```text
crates/world-runtime/src/
  primitive/
    mod.rs
    invocation.rs
    registry.rs
    semantics.rs
```

Existing modules stay in place:

```text
transaction/
process/
scheduler/
control/
```

Ownership:

- `primitive/semantics.rs`: object-safe `PrimitiveSemantics` and installer
  traits.
- `primitive/registry.rs`: immutable registry and builder.
- `primitive/invocation.rs`: resolved primitive invocation view and role/arg
  helpers.
- `transaction/validation.rs`: runtime validation dispatches through primitive
  registry.
- `transaction/effects.rs`: staging dispatches through primitive registry.
- `runtime.rs`: `CausalRuntime` owns both `DefinitionRegistry` and
  `PrimitiveSemanticsRegistry`.

Remove production dependency on `builtin.rs` by the end of the phase. The
current runtime contexts already use domain role names and capability methods,
so Phase 6 should replace dispatch and invocation binding rather than redo that
context cleanup. If a temporary adapter helps migration, keep it private and
delete it before the exit gate.

### `world-standard`

Target shape:

```text
crates/world-standard/src/
  lib.rs
  ids.rs
  events.rs
  bundle.rs
  primitives/
    mod.rs
    physical.rs
    reservation.rs
    process.rs
```

Ownership:

- stable standard primitive ids and names;
- event specs/families needed by the first primitives;
- helper constructors for standard primitive definitions;
- `StandardWorldDefinitions` zero-sized bundle implementing
  `DefinitionBundle`.

No runtime/model dependency is allowed.

### `world-standard-runtime`

Target shape:

```text
crates/world-standard-runtime/src/
  lib.rs
  bundle.rs
  physical/
    mod.rs
    create.rs
    transfer.rs
  reservation.rs
  events.rs
```

Ownership:

- trusted handler structs for standard primitive semantics;
- `StandardPrimitiveSemantics` zero-sized installer implementing
  `PrimitiveSemanticsInstaller`;
- tests that standard definitions and handlers stay compatible.

No engine/session facade belongs here.

## Key Data Model

### Primitive Identity

Add a newtyped primitive id:

```rust
pub struct EffectPrimitiveId(DefinitionId);
```

Use this as the runtime dispatch key. Keep operation names for diagnostics and
authoring lookup, not as execution dispatch keys.

This should be a private-field newtype with small constructor/accessor methods,
following the existing id patterns. Avoid a bare type alias.

### Primitive Definition

Initial shape:

```rust
pub struct EffectPrimitiveDef {
    id: EffectPrimitiveId,
    name: DefinitionName,
    params: Vec<EffectParamDef>,
    required_permissions: BTreeSet<StagePermission>,
    event_contract: EventContract,
    replay_level: ReplayLevel,
    version: VersionAnchor,
}
```

Constructor checks:

- params are unique by name;
- required permissions are not empty for executable primitives;
- event requirements match declared event-emission permissions;
- names and ids are retained for diagnostics;
- fields stay private with accessors.

Standard primitive crates should define zero-sized descriptor types that
implement an `EffectPrimitiveDescriptor` trait in `world-defs`. The descriptor is
the single pure schema source used both to materialize `EffectPrimitiveDef` and
to derive trusted runtime handler contracts.

Do not add a rich resource-effect lattice unless it is enforced immediately.
`StagePermission` is the enforced capability contract for this phase. If a
future resource-effect model is needed, it can be added as checked metadata on
top of the same primitive definition shape.

If primitive replay metadata stays in Phase 6, registry verification must define
a simple composition rule with `EffectProgramDef::replay_level()`. Otherwise keep
replay requirements program-owned and defer primitive-level replay metadata.

### Minimal Argument Model

Add only the typed argument model needed to remove `BuiltinRole` and support
checked primitive invocation.

Suggested shape:

```rust
pub struct EffectParamDef {
    name: EffectParamName,
    kind: EffectParamKind,
}

#[non_exhaustive]
pub enum EffectParamKind {
    EntityRole,
    OptionalEntityRole,
}

pub struct EffectArgBinding {
    param: EffectParamName,
    value: EffectArgValue,
}

#[non_exhaustive]
pub enum EffectArgValue {
    Role(RoleName),
}
```

This supports current primitives by mapping primitive params to action roles:

```text
place_entity:
  item -> RoleName("item")
  destination -> RoleName("destination")

acquire_reservation:
  target -> RoleName("item")
  holder -> RoleName("actor") when present
```

Do not implement a full expression language, literal value system, formula
language, or final taxonomy model in Phase 6. Leave those for authoring and
later semantic phases.

### Effect Operation

Move from:

```rust
EffectOp {
    kind: EffectKind,
    permissions: BTreeSet<StagePermission>,
    emitted_events: BTreeSet<EventRecordSpec>,
}
```

to:

```rust
EffectOp {
    primitive: EffectPrimitiveId,
    args: Vec<EffectArgBinding>,
    emitted_events: BTreeSet<EventRecordSpec>,
}
```

`EffectProgramDef` may still validate local invariants that do not need a
registry, such as non-empty operations and its own event contract coverage. It
should not pretend to verify primitive signatures without a registry.

`EffectOp.emitted_events` is invocation metadata: it may only select or bind
events allowed by the primitive definition. It must not expand primitive
authority.

Cross-definition checks belong in `DefinitionRegistry` verification.

### Definition Registry And Bundle Installation

Add primitive definitions to `DefinitionRegistry`.

Add a builder:

```rust
pub struct DefinitionRegistryBuilder { ... }

pub trait DefinitionBundle {
    fn install_definitions(
        &self,
        builder: &mut DefinitionRegistryBuilder,
    ) -> Result<(), DefinitionError>;
}
```

Builder API should use `&mut self` methods and return typed errors:

```rust
impl DefinitionRegistryBuilder {
    pub fn new() -> Self;
    pub fn install<B: DefinitionBundle + ?Sized>(
        &mut self,
        bundle: &B,
    ) -> Result<&mut Self, DefinitionError>;
    pub fn add_primitive(&mut self, primitive: EffectPrimitiveDef)
        -> Result<&mut Self, DefinitionError>;
    pub fn add_effect_program(&mut self, program: EffectProgramDef)
        -> Result<&mut Self, DefinitionError>;
    pub fn add_action(&mut self, action: ActionDef)
        -> Result<&mut Self, DefinitionError>;
    pub fn add_process(&mut self, process: ProcessDef)
        -> Result<&mut Self, DefinitionError>;
    pub fn add_semantic_declaration(&mut self, declaration: SemanticDeclarationDef)
        -> Result<&mut Self, DefinitionError>;
    pub fn build(self) -> Result<DefinitionRegistry, DefinitionError>;
}
```

Registry verification should check:

- unique ids across all definition families;
- unique primitive names within primitive definitions;
- every `EffectOp.primitive` resolves;
- every op argument matches the primitive's params;
- every required primitive param is bound;
- every role referenced by `EffectArgValue::Role` is declared by the owning
  action or process definition;
- every operation event is permitted by the primitive event contract;
- every primitive-required event is emitted by the operation or rejected as
  invalid;
- action/process declared stage permissions cover the permissions derived from
  the primitives used by their effect programs;
- action/process event contracts cover the events their effect programs can
  emit;
- existing process/action/effect-program checks continue to pass.

Keep `DefinitionRegistry` immutable after build.

`DefinitionRegistryBuilder` is the authoritative validation path. If
`DefinitionRegistry::new(...)` remains public, it should delegate to the builder
instead of maintaining a second verifier.

## Runtime Semantics Design

### Object-Safe Handler Trait

`PrimitiveSemantics` should be object-safe because the runtime registry needs
heterogeneous handlers.

Suggested shape:

```rust
pub trait PrimitiveSemantics: Send + Sync + 'static {
    fn primitive(&self) -> EffectPrimitiveId;

    fn contract(&self) -> PrimitiveSemanticsContract;

    fn validate(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveValidationContext<'_>,
    ) -> Result<(), PrimitiveValidationError>;

    fn stage(
        &self,
        invocation: PrimitiveInvocation<'_>,
        context: &mut PrimitiveStageContext<'_, '_, '_, '_>,
    ) -> Result<(), RuntimeError>;
}
```

Avoid associated consts, generic methods, `Self` return types, or static
dispatch-only methods in this trait.

`PrimitiveSemanticsContract` is a small pure descriptor used during registry
build to catch handler/definition drift such as params, permissions, event
contract, or version mismatch. Runtime `validate` is current-world validation,
not definition compatibility verification.

The context and error types in this trait must be public enough for
`world-standard-runtime` to implement handlers while still hiding `EffectStager`,
id issuers, model apply methods, and accepted-package constructors.

Handlers should usually be zero-sized structs:

```rust
pub struct TransferEntity;
```

### Registry Builder

Suggested shape:

```rust
pub struct PrimitiveSemanticsRegistryBuilder { ... }

pub trait PrimitiveSemanticsInstaller {
    fn install_semantics(
        &self,
        builder: &mut PrimitiveSemanticsRegistryBuilder,
    ) -> Result<(), RuntimeError>;
}

pub struct PrimitiveSemanticsRegistry { ... }
```

Builder behavior:

- `add_handler(Box<dyn PrimitiveSemantics>)` rejects duplicate primitive ids;
- `install(&impl PrimitiveSemanticsInstaller)` composes trusted bundles;
- `build_against(&DefinitionRegistry)` verifies each handler has a matching
  primitive definition;
- `build_against` verifies handler contracts against primitive definitions;
- `build_against` requires handler coverage for every primitive used by
  action-executed effect programs.

Do not allow handler replacement or "last installer wins" behavior.

`PrimitiveSemanticsRegistry` should be immutable once built.

Process definition effect programs are not interpreted by the current process
runtime. Classify current process fixtures before implementation; either leave
those programs definition-only for Phase 6 coverage or add a minimal process
primitive only if execution requires it.

### Resolved Invocation View

Add a resolved invocation value:

```rust
pub struct PrimitiveInvocation<'a> {
    operation: &'a EffectOp,
    primitive: &'a EffectPrimitiveDef,
}
```

It should provide small helpers for semantics code:

```rust
impl PrimitiveInvocation<'_> {
    pub fn primitive(&self) -> &EffectPrimitiveDef;
    pub fn operation(&self) -> &EffectOp;
    pub fn required_role(&self, param: &EffectParamName) -> Result<RoleName, RuntimeError>;
    pub fn optional_role(&self, param: &EffectParamName) -> Result<Option<RoleName>, RuntimeError>;
}
```

The exact helper names can change, but the semantics code should no longer know
about `BuiltinRole`.

### Validation And Stage Contexts

Rename or adapt current contexts:

```text
ValidationContext -> PrimitiveValidationContext
StageContext      -> PrimitiveStageContext
```

They must expose capabilities, not raw store mutation:

- role binding lookup;
- committed/staged entity and relation reads;
- staged entity/relation visibility;
- reservation conflict checks;
- reservation staging through runtime-control change APIs;
- hard-state change staging through transaction APIs;
- declared event emission through event id issuers and event contracts;
- request time and provenance.

Permission checks should live in these context methods wherever possible.
Handler code asks for a capability; the context enforces that the current
primitive invocation declared the matching permission/event authority.

They must not expose:

- `&mut WorldModel`;
- accepted commit constructors;
- model receiver apply methods;
- raw runtime-control store mutation;
- host IO or arbitrary callbacks.

### Runtime Construction

Change `CausalRuntime` to own both definitions and semantics:

```rust
pub struct CausalRuntime {
    definitions: DefinitionRegistry,
    semantics: PrimitiveSemanticsRegistry,
    transaction_ids: CausalTransactionIdIssuer,
    event_ids: EventRecordIdIssuer,
    control_ids: RuntimeControlIds,
    interpreter: TypedEffectInterpreter,
}
```

Constructors should become fallible where compatibility checks can fail:

```rust
pub fn for_empty_model(
    definitions: DefinitionRegistry,
    semantics: PrimitiveSemanticsRegistry,
) -> Result<Self, RuntimeError>;
```

Existing `for_model` constructors already return `Result` and can follow the
same pattern.

Do not create a default standard runtime inside `world-runtime`; that would
reintroduce `world-runtime -> world-standard` coupling.

### Runtime Execution Flow

`RuntimeValidator` should dispatch like:

```text
for operation in program.operations():
  primitive = definitions.effect_primitive(operation.primitive())?
  semantics = primitive_registry.handler(operation.primitive())?
  invocation = PrimitiveInvocation::new(operation, primitive)?
  semantics.validate(invocation, &mut context)?
```

`TypedEffectInterpreter` should dispatch the same resolved invocation through
`semantics.stage(...)`.

This keeps validator/interpreter dispatch symmetrical and removes runtime
string matching.

## Standard Bundle Design

### First Standard Definitions

Migrate current seed primitives first:

```text
create_entity or insert_entity
place_entity
acquire_reservation
record_event only if kept as an explicit primitive
```

Prefer domain names that can survive beyond tests. If `insert_entity` is too
store-shaped, consider `create_entity` for the standard primitive while keeping
the model-facing hard change named as it currently is.
Use `place_entity` for containment insertion. A true `transfer_entity`
primitive should be added only when relation removal/update semantics exist.

Do not add `apply_damage`, `add_condition`, `emit_signal`, `schedule_process`,
or resource consumption in the first patch unless they are needed to prove the
registry architecture. Leave them as near-future standard primitives.

### Event Emission

Prefer this shape:

- ordinary primitives declare and emit their own event candidates through
  `PrimitiveStageContext`;
- `record_event` exists only if there is a clear need for a primitive whose sole
  job is to emit declared hard evidence.

Do not let event emission become an unchecked event bus or listener mutation
path.

### Standard Runtime Handlers

Move current `BuiltinEffect` behavior into handler structs:

```text
world_standard_runtime::physical::CreateEntity
world_standard_runtime::physical::TransferEntity
world_standard_runtime::reservation::AcquireReservation
world_standard_runtime::events::RecordEvent
```

Handlers should share validation helpers with staging where practical, but keep
gameplay rejection and infrastructure errors distinct:

```text
validation failure that is a gameplay result -> RejectedOutcome
corrupt/missing registry or impossible runtime state -> RuntimeError
```

For the Phase 6 seed primitives, staging should only report infrastructure
failure. Gameplay rejection should happen during runtime validation before
staging. Future primitives that need domain failure during staging should add an
explicit outcome shape rather than using `RuntimeError` for gameplay failure.

## Implementation Order

Before changing shared APIs, confirm the current effect-kind fixture inventory in
`world-runtime/src/tests/helpers.rs` and map each kind to a migrated executable
standard primitive, definition-only metadata for this phase, or a removed
fixture.

Keep each shared API break compile-preserving. The `EffectOp` shape change, seed
primitive definitions, fixture builders, semantics registry, and runtime
dispatch migration should be planned as one migration slice with
`cargo check --workspace` after the slice.

Any temporary adapter introduced to keep the tree compiling must be removed in
the cleanup step before Phase 6 is considered complete.

### 1. Add Crate Skeletons

- Add `crates/world-standard` and `crates/world-standard-runtime`.
- Add them to workspace members.
- Keep crate roots minimal and dependency direction correct.
- Do not add `world-engine` APIs.
- Add or update the dependency-direction guardrail for the full crate contract.

Checkpoint:

- `cargo check --workspace` should still pass with empty/minimal crates.

### 2. Add Primitive Definition Model

- Extend the existing `world-defs/src/effects/` modules.
- Add `EffectPrimitiveId`, `EffectPrimitiveDef`, param/arg types, and updated
  `EffectOp`.
- Update `EffectProgramDef` local validation around operation events.
- Add structured `DefinitionError` variants for primitive and argument
  validation.
- Update crate-root re-exports.

Checkpoint:

- `world-defs` unit tests cover primitive constructor invariants and operation
  argument shape.

### 3. Extend Registry Builder And Cross-Definition Verification

- Extend the existing `DefinitionRegistryBuilder` with primitive installation.
- Add `DefinitionBundle`.
- Add primitive table to `DefinitionRegistry`.
- Extend existing `registry/validate.rs` checks for primitive references.
- Validate primitive references, args, permissions, and event contracts.
- Update existing tests and fixtures from direct `DefinitionRegistry::new(...)`
  assembly to builder-based assembly where clearer.

Checkpoint:

- existing action/process/effect-program validation behavior is preserved;
- new tests reject missing primitive definitions, bad args, and event/permission
  mismatches.

### 4. Add Runtime Semantics Registry

- Add `world-runtime::primitive` module.
- Define object-safe `PrimitiveSemantics`.
- Define `PrimitiveSemanticsInstaller`, builder, and immutable registry.
- Add structured `RuntimeError` variants for missing handler, duplicate handler,
  and definition/semantics mismatch.
- Build registry against `DefinitionRegistry`.

Checkpoint:

- unit tests reject duplicate handlers, missing primitive definitions, and
  missing handlers for executable programs.

### 5. Refactor Runtime Validation And Staging Contexts

- Extend the existing `RoleName`-based contexts with primitive invocation
  arg/role helpers.
- Rename/adapt contexts to primitive validation/stage contexts if that improves
  clarity.
- Keep capability methods narrow and permission/event checked.
- Ensure no handler receives raw model mutation authority.

Checkpoint:

- runtime validation tests still distinguish `RejectedOutcome` from
  `RuntimeError`.

### 6. Add Standard Definition Bundle

- Add standard primitive ids/names/events.
- Implement `StandardWorldDefinitions`.
- Install the first standard primitive definitions through `DefinitionBundle`.
- Add tests that the standard bundle builds a valid registry on its own.

Checkpoint:

- `world-standard` has no dependency on `world-runtime`, `world-model`, or
  `world-standard-runtime`.

### 7. Add Standard Runtime Semantics

- Implement handlers equivalent to current seed builtins.
- Implement `StandardPrimitiveSemantics`.
- Test standard definitions plus standard runtime semantics build together.
- Use zero-sized handler structs and shared small helpers where useful.

Checkpoint:

- `world-standard-runtime` depends on `world-runtime` and `world-standard`, but
  no reverse edge exists.

### 8. Wire `CausalRuntime` Through The Registry

- Change runtime constructors to accept `PrimitiveSemanticsRegistry`.
- Update `RuntimeValidator` and `TypedEffectInterpreter` dispatch to use
  resolved primitive definitions and handlers.
- Update all runtime tests to build definitions and semantics explicitly.
- Remove production `BuiltinEffect` dispatch.

Checkpoint:

- existing runtime behavior tests pass through standard runtime handlers.
- no production code calls `BuiltinEffect::from_operation`.

### 9. Cleanup And Guardrails

- Remove obsolete `EffectKind` runtime dispatch usage. Keep `EffectKind` only if
  it remains useful for authoring/source-name compatibility; otherwise defer its
  removal if broad churn is not worth it.
- Remove all temporary migration adapters or compatibility shims introduced
  earlier in the phase.
- Update authority surface guardrails for the new trusted semantics surface:
  handler implementations, handler registration, and primitive stage-context
  construction should appear only in runtime internals, `world-standard-runtime`,
  or explicit tests.
- Add tests that prove ordinary checked effect programs cannot execute without
  installed semantics.
- Keep dependency-direction checks active after adding the standard crates.

Checkpoint:

- `rg -n "BuiltinEffect|BuiltinRole" crates/world-runtime/src` should find no
  production dispatch path by the exit gate.

## Tests To Add Or Preserve

### `world-defs`

- primitive id/name/param constructor invariants;
- duplicate primitive ids rejected;
- duplicate primitive names rejected if name uniqueness is enforced;
- missing primitive reference in `EffectOp` rejected by registry build;
- missing required operation arg rejected;
- unknown operation arg rejected;
- wrong arg kind rejected;
- role arg references undeclared by the owning action/process rejected;
- primitive event contract mismatch rejected;
- action/process permission coverage still enforced from primitive-derived
  permissions;
- existing event contract tests continue to pass.

### `world-runtime`

- semantics registry rejects duplicate handlers;
- semantics registry rejects handler for unknown primitive;
- semantics registry rejects executable program with missing handler;
- runtime constructor rejects incompatible definitions/semantics;
- stage context rejects permission/event authority not declared by the primitive
  invocation;
- runtime validation dispatches through handler and returns `RejectedOutcome`
  for gameplay validation failures;
- runtime staging dispatches through handler and commits expected hard changes;
- event emission still requires declaration and permission;
- reservation acquisition remains transaction-coupled where required;
- authority surface guardrail still passes.

### `world-standard`

- standard definition bundle installs without runtime dependency;
- each standard primitive has stable id/name/version;
- standard primitive event contracts match intended emitted events;
- standard bundle can be composed with local test definitions.

### `world-standard-runtime`

- standard semantics installer covers every primitive in the first standard
  bundle that is executable;
- handler ids match standard primitive ids;
- create/transfer/reservation/event handlers preserve current runtime behavior;
- definition/semantics mismatch test proves drift is caught.

### Workspace

Run:

```bash
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
git diff --check
```

During development, use targeted package tests for the crate being changed, then
run the full gate before marking the phase complete.

## Acceptance Criteria

- `world-standard` and `world-standard-runtime` exist if they were missing.
- `world-runtime` does not depend on either standard crate.
- `world-standard` does not depend on `world-runtime` or `world-model`.
- `world-standard-runtime` is the only standard crate with runtime semantics
  handlers.
- dependency-direction guardrails cover the full crate contract.
- `world-engine` has not gained a premature facade or session lifecycle.
- Checked `EffectOp` dispatch is primitive-id based.
- Runtime execution no longer string-matches operation names.
- no production compatibility shim or legacy dispatch path remains.
- Current seed runtime behavior is preserved through standard semantics
  handlers.
- Missing semantics fails clearly at registry build or runtime construction.
- Handler/definition contract drift fails during semantics registry build.
- Ordinary pack-like definitions compose primitives but do not implement trusted
  semantics.
- Primitive stage capabilities enforce declared permissions instead of relying
  on handler-local convention.
- No broad plugin/system/pass framework has been introduced.
- Full workspace verification passes.

## Explicitly Out Of Scope

- `world-engine` session API or facade wiring.
- Final standard primitive taxonomy.
- Damage, wound, resource, material, field, signal, condition, combat, spell, or
  crafting systems beyond minimal placeholders needed for the migrated seed
  primitives.
- Parser/source syntax.
- Pack manifest/dependency resolution.
- Incremental authoring or Salsa integration.
- MLIR dependency or generic compiler pass manager.
- ECS/Datalog/graph/scripting/Wasm adapters.
- External plugin loading, signing, or sandbox policy.
- Async host IO.

## Design Notes For Implementation

- Keep fields private and expose narrow accessors.
- Use newtypes for identities that carry different domain meaning even when they
  wrap the same representation.
- Use `BTreeMap`/`BTreeSet` for deterministic ordering unless performance proves
  otherwise.
- Use `#[non_exhaustive]` for public enums that are expected to grow.
- Keep handler traits object-safe; prefer zero-sized handler structs.
- Keep registries immutable after build.
- Use structured errors with domain context; do not use string-only failures.
- Keep gameplay outcomes separate from infrastructure errors.
- Prefer one complete replacement over permanent compatibility layers. Temporary
  migration code is acceptable only if it is deleted before the phase exit gate.
- Prefer small shared helper functions over macro abstraction unless repetition
  becomes both large and mechanically identical.
- Do not encode planning terms into code names, comments, tests, or diagnostics.

## Implementation Defaults

- Use `create_entity` as the standard primitive name if tests can be migrated
  cleanly; keep model hard change names unchanged.
- Keep `record_event` only if an existing test or runtime path needs an
  event-only operation; otherwise treat event emission as a staging capability.
- Introduce the builder as the authoritative construction path. Keep direct
  constructors only as wrappers over the same verifier.
- Require semantics coverage for every primitive used by executable effect
  programs, not every primitive merely defined in the registry.
