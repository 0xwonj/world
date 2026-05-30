# Pre-Phase 6 Architecture Alignment Plan

## Status

Draft implementation plan for a focused architecture cleanup before Phase 6.

This plan should run before
`phase-6-standard-world-library-and-primitive-semantics.md`. It intentionally
does not implement standard primitive definitions, standard crates, primitive
semantics registry, or primitive-id-based `EffectOp`. It prepares the existing
code so Phase 6 can make those changes without mixing them with avoidable
structural churn.

## Purpose

Align the current Phase 0-5 code with the Phase 6 target architecture by
removing the roughest seams in existing code:

```text
current code:
  flat world-defs effect/registry files
  direct DefinitionRegistry::new(...) assembly
  runtime contexts coupled to BuiltinRole
  handler-local permission checks
  fixture effect kinds spread through runtime tests

pre-Phase-6 target:
  module structure ready for primitive definitions
  one registry validation path
  runtime contexts accept domain role names, not builtin enums
  staging capabilities enforce permissions where possible
  current effect-kind usage is inventoried and fixtures are centralized
```

The goal is not to make Phase 6 smaller by secretly implementing it early. The
goal is to make Phase 6 mostly about primitive semantics, not unrelated file
splits, test fixture cleanup, and context API untangling.

## Non-Goals

- Do not add `world-standard` or `world-standard-runtime`.
- Do not add `EffectPrimitiveDef`, `EffectPrimitiveId`, or primitive argument
  types.
- Do not change `EffectOp` away from `EffectKind` yet.
- Do not add `PrimitiveSemantics`, `PrimitiveSemanticsRegistry`, or semantics
  installers.
- Do not remove `BuiltinEffect` yet.
- Do not prebuild `world-engine`.
- Do not change crate dependency direction.
- Do not add major dependencies.

## Baseline Findings

The existing code is working, but several shapes will make Phase 6 harder than
it needs to be:

- `crates/world-defs/src/effects.rs` mixes stage permissions, operation shape,
  effect program validation, and permission/event helper functions in one file.
- `crates/world-defs/src/registry.rs` is both the registry type and the only
  cross-definition verifier; there is no builder/install surface for future
  definition bundles.
- `EffectOp` currently owns permissions and emitted events directly. This stays
  until Phase 6, but surrounding validation should be easier to move.
- `crates/world-runtime/src/transaction/effects.rs` exposes `StageContext`
  helpers that take `BuiltinRole`, so the staging context is coupled to one
  built-in vocabulary.
- `crates/world-runtime/src/transaction/validation.rs` has the same
  `BuiltinRole` coupling in `ValidationContext`.
- current builtins manually remember permission checks before staging hard
  changes or reservations. Phase 6 wants capability methods to enforce declared
  permission/event authority.
- runtime test helpers construct string-like effect kinds throughout the test
  surface, including `schedule_process`, which is process definition metadata in
  current code rather than an interpreted action effect.

## Recommended Changes

### 1. Split `world-defs` Effect Modules Without Behavior Change

Restructure `world-defs` from:

```text
crates/world-defs/src/effects.rs
```

to:

```text
crates/world-defs/src/effects/
  mod.rs
  permissions.rs
  program.rs
```

Ownership:

- `permissions.rs`: `StagePermission` and permission/event helper predicates.
- `program.rs`: current `EffectOp`, `EffectProgramDef`, and local program
  validation.
- `mod.rs`: internal re-export hub.

Keep the public crate-root API unchanged:

```rust
pub use effects::{EffectOp, EffectProgramDef, StagePermission};
```

Do not add primitive definition modules yet. Phase 6 can later add
`primitive.rs` and `args.rs` into this already-prepared directory.

### 2. Split `world-defs` Registry Modules And Add A Builder

Restructure `world-defs` from:

```text
crates/world-defs/src/registry.rs
```

to:

```text
crates/world-defs/src/registry/
  mod.rs
  builder.rs
  validate.rs
```

Add `DefinitionRegistryBuilder` for the current definition families only:

```rust
pub struct DefinitionRegistryBuilder { ... }

impl DefinitionRegistryBuilder {
    pub fn new() -> Self;
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

Rules:

- `add_*` should reject duplicate ids and local invariants only.
- cross-reference validation remains in `build()`.
- `DefinitionRegistry::new(...)` stays public for compatibility but delegates to
  the builder.
- `DefinitionRegistry` remains immutable after build.
- crate-root should re-export `DefinitionRegistryBuilder`.

Do not add `DefinitionBundle` yet unless it is needed to avoid awkward builder
API. The real bundle trait belongs to Phase 6 when standard definitions exist.

### 3. Decouple Runtime Contexts From `BuiltinRole`

Keep `BuiltinEffect` for now, but make transaction contexts operate on domain
role names rather than the builtin enum.

Current:

```rust
context.required_role(BuiltinRole::Item)
context.optional_role(BuiltinRole::Actor)
```

Target:

```rust
context.required_role_entity(&RoleName)
context.optional_role_entity(&RoleName)
```

`BuiltinRole` may remain as a small helper that produces `RoleName`, but it
should be used only inside the current builtin adapter. `StageContext` and
`ValidationContext` should not expose methods that require `BuiltinRole`.

This is the most useful pre-Phase-6 runtime change because Phase 6 can then
replace builtin role constants with primitive invocation args without also
rewriting context ownership.

### 4. Move Permission Checks Into Staging Capabilities Where Practical

Keep the current `EffectOp` permission model until Phase 6, but make staging
methods enforce it.

Add or adapt context methods such as:

```rust
stage_physical_change(operation, change)
stage_reservation_acquire(operation, request)
emit_declared_events(operation)
emit_event(operation, spec)
```

These methods should check the relevant `StagePermission` before touching the
underlying `EffectStager`, runtime-control ids, or event ids.

The current builtin handlers should ask the context for capabilities instead of
calling permission helpers in each handler. This preserves existing behavior
while making the future `PrimitiveStageContext` direction explicit.

Do not try to design the final public `PrimitiveStageContext` yet.

### 5. Centralize Current Effect-Kind Fixtures

Before Phase 6 changes `EffectOp`, reduce fixture sprawl:

- keep all current effect-kind test constructors in `world-runtime/src/tests/helpers.rs`;
- add a small inventory helper or comment grouping current kinds:
  - action-executed seed effects: `insert_entity`, `transfer_entity`,
    `acquire_reservation`, `record_event` if still used;
  - process-definition metadata fixture: `schedule_process`;
  - intentionally invalid/missing handler fixture names.
- update tests to use named helpers instead of scattering raw strings where it
  is clearer.

This is not a product taxonomy. It is a migration aid so Phase 6 can classify
which current names become standard primitives and which remain metadata/tests.

### 6. Strengthen Guardrail Placement, Not Semantics Yet

The existing `authority_surface` test protects accepted-package constructors
and model apply calls. Before Phase 6, keep it passing and prepare it for the
next authority surface by isolating scan helpers clearly.

Do not add `PrimitiveSemantics` guardrails before the type exists. But avoid
making `authority_surface.rs` harder to extend.

### 7. Keep Public API Churn Intentional

Expected public changes before Phase 6:

- `world_defs::DefinitionRegistryBuilder` is added.
- existing `world_defs::DefinitionRegistry::new(...)` remains available.

Avoid public runtime API changes unless needed by the internal refactor. In
particular, do not make current transaction contexts public yet; Phase 6 should
define the public capability context shape only when `world-standard-runtime`
needs to implement handlers.

## Implementation Order

1. Split `world-defs/src/effects.rs` into `effects/` modules with no behavior
   change.
2. Split `world-defs/src/registry.rs` into `registry/` modules and introduce
   `DefinitionRegistryBuilder`.
3. Migrate `world-defs` and `world-runtime` tests that benefit from builder
   construction, while keeping `DefinitionRegistry::new(...)` compatibility.
4. Add role-name-based runtime context methods and migrate `BuiltinEffect` to
   use them.
5. Move permission enforcement into staging capability methods and remove
   duplicated handler-local permission checks where possible.
6. Centralize current effect-kind fixture helpers and document the
   `schedule_process` classification in test helpers.
7. Run the full verification gate and review that Phase 6 plan still matches the
   updated code.

Each step should keep the workspace compiling. If a step needs broad API churn,
finish that slice before moving on rather than leaving the tree in an
intermediate broken state.

## Tests

Preserve existing behavior tests. Add only targeted tests that protect the new
architecture surface:

- `DefinitionRegistryBuilder` accepts the same valid registry as
  `DefinitionRegistry::new(...)`;
- builder and direct constructor reject the same duplicate id and
  cross-reference errors;
- runtime context role-name helpers preserve missing-role rejection behavior;
- staging context rejects hard mutation without the relevant permission;
- staging context rejects reservation acquisition without the relevant
  permission;
- event emission permission behavior remains unchanged;
- `authority_surface` guardrail still passes.

Do not add tests for primitive ids, standard definitions, semantics registry, or
standard runtime handlers here. Those are Phase 6 tests.

## Acceptance Criteria

- No crate dependency direction changes.
- No new crates are added.
- No standard primitive definitions or semantics registry are implemented.
- `world-defs` effect and registry modules are easier to extend without changing
  crate-root public imports.
- `DefinitionRegistryBuilder` is the single internal validation path, and
  `DefinitionRegistry::new(...)` delegates to it.
- runtime validation/staging contexts no longer expose `BuiltinRole`-typed APIs.
- staging permission checks are enforced by context capability methods where the
  current model allows it.
- runtime tests use centralized current effect-kind helpers.
- Phase 6 can start by adding primitive definitions/standard crates instead of
  first doing broad unrelated module cleanup.

## Verification

Run:

```bash
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
git diff --check
```

## Follow-Up Into Phase 6

After this cleanup, Phase 6 should still own:

- adding `world-standard` and `world-standard-runtime`;
- adding `EffectPrimitiveDef`, primitive args, and primitive-id-based `EffectOp`;
- adding `DefinitionBundle`;
- adding `PrimitiveSemantics`, public primitive capability contexts, and
  `PrimitiveSemanticsRegistry`;
- migrating current `BuiltinEffect` behavior into standard runtime handlers;
- adding semantics authority/dependency guardrails for the new standard crates.
