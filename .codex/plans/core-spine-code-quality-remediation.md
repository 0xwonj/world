# Core Spine Code Quality Remediation Plan

## Purpose

Resolve the code quality and Rust idiom issues found in the first deep review
of the Phase 0-5 core spine.

This plan is intentionally about implementation quality, not a new
architecture direction. It should preserve the current crate split and
authority model while making the code easier to audit before Phase 6 adds the
standard world library and primitive semantics layer.

## Non-Goals

- Do not introduce a new kernel crate.
- Do not change crate dependency direction.
- Do not redesign public authority boundaries beyond the targeted API
  ergonomics listed below.
- Do not introduce a shared test-support crate yet.
- Do not add major dependencies for production code.

## Work Package 1: Runtime-Control Planning And Application

Problem:

- `RuntimeControlChange` is doing too much: command input, validation input,
  and materialization input.
- `plan_change` and `materialize_change` duplicate the same variant logic.
- `apply_changes` silently skips impossible materialization failures.
- `plan_changes` clones the full runtime-control record map for every update.
- `runtime_control.rs` has grown into a broad record/schema/store/planner/
  validator module.
- accepted runtime-control envelope validation is split between constructor
  and receiver in a way that makes "accepted" weaker than the name implies.

Target shape:

- First mechanically split `crates/world-model/src/runtime_control.rs` into
  private submodules while preserving the exact public re-export surface:
  - record/domain types;
  - accepted update/change types;
  - store/read API;
  - planning/apply validation.
- This split should be behavior-preserving and verified before changing the
  plan/apply contract.
- Introduce a concrete internal planned-delta type in `world-model`, for
  example:

```rust
pub(crate) struct RuntimeControlDelta {
    kind: RuntimeControlRecordKind,
    record: RuntimeControlRecord,
}
```

- Make `RuntimeControlChangeApplyPlan` own the validated materialized deltas
  in change order, not only changed keys.
- Make `RuntimeControlUpdateApplyPlan` wrap the change plan plus update cursor
  metadata.
- Replace `materialize_change` with plan-owned deltas. Apply should never
  reinterpret `RuntimeControlChange`.
- Replace full-map cloning with a small planning overlay:
  - read first from planned overlay, then from base records;
  - write planned deltas into the overlay;
  - validate touched process/wakeup links through overlay-aware lookup;
  - scan base plus overlay by record family when reservation conflicts require
    checking all effective held reservations;
  - scan base plus overlay when transitioned wakeups require checking existing
    scheduled processes.
- Preserve the existing lane split:
  - control-only accepted updates append runtime-control update history and
    advance the update cursor;
  - transaction-coupled changes apply as part of a hard commit and do not append
    runtime-control update history.
- Decide and document inside code shape whether accepted update history remains
  a touched-key index or stores richer accepted change facts. Do not accidentally
  change the history semantics while refactoring apply.
- Make envelope validation explicit:
  - decide whether `AcceptedRuntimeControlUpdate::new` validates invalidation
    source/authority/store-family or whether the model receiver remains the
    validation point;
  - if receiver validation remains intentional, rename/rustdoc/test the weaker
    "accepted package shape" meaning clearly;
  - validate or document which `RuntimeControlSource` values are legal for
    control-only updates versus transaction-coupled changes.
- Make time/provenance materialization policy explicit:
  - create/update/schedule changes carry record update metadata;
  - terminal transitions carry transition time;
  - provenance preservation versus replacement must be encoded once in the
    planner, not rediscovered during apply.

Acceptance criteria:

- No duplicated transition materialization logic.
- No silent skip during apply.
- Apply consumes only plan-owned deltas and does not receive raw
  `RuntimeControlChange`.
- Runtime-control preflight no longer clones all stored records.
- Control-only and transaction-coupled lane behavior remains covered by tests.
- Existing runtime-control tests still pass.
- New focused tests cover multi-change validation where a process update and
  wakeup transition depend on each other in the same package.
- New focused tests cover overlay reservation conflict validation.

## Work Package 2: Hard Commit Finalization Path Alignment

Problem:

- Normal action execution finalizes through `CommitFinalizer`.
- Scheduler-driven process commits manually construct `AcceptedHardCommit`.
- Process commits currently use an accepted runtime-control update as an
  intermediate even when the changes are transaction-coupled.
- This creates two hard-commit construction paths, bypasses finalizer policy,
  and can misrepresent process wakeup work as an action/effect-program
  transaction.

Target shape:

- Introduce an internal runtime-control draft/change-set type in
  `world-runtime`, for example:

```rust
pub(crate) struct RuntimeControlDraft {
    source: RuntimeControlSource,
    occurred_at: SimulationTime,
    replay_level: ReplayLevel,
    provenance: Option<ProvenanceKey>,
    changes: Vec<RuntimeControlChange>,
}
```

- A draft can be consumed in exactly one of two ways:
  - `accept_control_only()` for control-only model application;
  - `into_transaction_changes()` for transaction-coupled hard commit staging.
- Process runtime should return drafts, not already accepted updates, when the
  caller must decide between skipped control-only application and process hard
  commit attachment.
- No process runtime path should create `AcceptedRuntimeControlUpdate` until
  the caller has chosen control-only application.
- Extend the hard transaction header/finalization model so it can represent
  both action-request work and process-wakeup work without pretending that a
  process definition is an action definition.
- Explicitly decide process event-contract semantics:
  - if process ticks are eventless control/hard transactions, encode that as a
    distinct allowed transaction cause;
  - if process effect programs require events, scheduler finalization must
    validate or emit them through the shared finalizer path.
- Add a finalizer path for process transactions that shares finalizer policy
  with action transactions.
- Remove direct `AcceptedHardCommit::with_control_changes` construction from
  scheduler code.
- Replace `unreachable!` process-transition assumptions with typed finalization
  inputs. Non-skipped transitions should carry the process/effect context needed
  to finalize; skipped transitions should remain control-only.
- Shrink the authority surface guardrail allowlist so scheduler can no longer
  construct accepted hard commits directly.

Acceptance criteria:

- There is one hard-commit finalization policy in runtime code.
- Scheduler process commits no longer manually build accepted hard commits.
- Control-only and transaction-coupled runtime-control lanes remain visible.
- Ownership improves: avoid `update.changes().to_vec()` for process commits.
- A test fails on the current scheduler path if a process effect contract is
  violated or if eventless process transactions are not explicitly allowed.

## Work Package 3: Process Lifecycle Construction Cleanup

Problem:

- Process lifecycle functions repeat builder creation, wakeup consume/cancel,
  process update, wakeup schedule, and accept steps.
- Narrow lifecycle transitions are represented by full-record replacement at
  many call sites.
- `ProcessInstanceRecord::new` is positional and already requires a
  `too_many_arguments` suppression.

Target shape:

- Keep concrete domain helpers instead of adding broad traits.
- Make the runtime-control draft from Work Package 2 own process-control helper
  methods:
  - `create_process`;
  - `update_process`;
  - `schedule_wakeup`;
  - `consume_wakeup`;
  - `cancel_current_wakeup`;
  - `skip_wakeup`.
- Direct `RuntimeControlChange` assembly should be limited to the draft/helper
  layer.
- Add a private `ProcessInstanceSpec` or `NewProcessInstance` for initial
  process construction so call sites no longer pass a long positional list.
- Keep full `ProcessInstanceRecord` as durable model data for now, but make
  runtime code express lifecycle transitions through named helpers rather than
  hand-built record replacement at every site.
- Isolate placeholder tick policy such as fixed one-unit progress and next-tick
  scheduling behind named helpers so it is not mistaken for final process
  semantics.

Acceptance criteria:

- `ProcessRuntime::start`, `resume`, `advance_wakeup`, and terminal update
  helpers read as domain transitions, not manual record assembly.
- Repeated builder/accept boilerplate is removed.
- Public API does not gain a generic lifecycle framework or trait hierarchy.

## Work Package 4: API Ergonomics And Misuse Resistance

Problem:

- Some APIs are valid but easy to misuse or hard to read.
- Loaded-model runtime construction can accidentally use fresh runtime-control
  issuers.
- Several domain constructors rely on long positional argument lists.
- `stage_acquire_reservation` swallows all actor-role lookup errors.
- `EventRecordSpec` diagnostics hide role-shape differences.
- The authority surface guardrail test uses raw substring matching.
- Some public exports appear accidental or premature.

Target shape:

- Split this work package into two passes:
  - misuse-safety prerequisites before Work Package 2/3;
  - diagnostics and guardrail polish after production shape stabilizes.
- Rename or supplement runtime constructors so intent is explicit:
  - prefer `CausalRuntime::for_empty_model` over ambiguous `new`;
  - prefer `with_hard_issuers_for_empty_model` over ambiguous `with_issuers`;
  - keep hydrated constructors for existing model state.
- Audit runtime public re-exports and either remove or document accidental
  surfaces such as internal-only process tick and reservation staging request
  types.
- Add named request/spec structs only where positional arguments are already
  causing confusion:
  - checked definition constructors may be cleaned up incrementally;
  - process instance construction should be addressed with Work Package 3.
- After Work Package 3, consider request structs for public process-control APIs
  such as wait, pause, interrupt, abandon, and resume if call sites still read
  as positional plumbing.
- Add an explicit optional actor lookup for reservation staging instead of
  `Err(_) => ReservationHolder::Runtime`.
- Improve `EventRecordSpec` display/debug diagnostics to include role shape.
- Strengthen `authority_surface.rs`:
  - scan all relevant workspace crate sources, not only `world-model/src` and
    `world-runtime/src`;
  - account for moved fixture modules without broadening production allowlists;
  - minimally, ignore comments/strings and scan token-like paths;
  - preferably, use `syn` as a dev-dependency only if the extra dependency is
    accepted as worthwhile for this repository.

Acceptance criteria:

- Constructor names distinguish empty-model runtime from hydrated runtime.
- Reservation holder fallback only handles actor absence, not arbitrary role
  errors.
- Public re-exports are intentional and documented by use.
- Event contract failures identify event kind, version, and role shape.
- Guardrail test is less vulnerable to alias/wrapper/comment drift than raw
  substring matching.

## Work Package 5: Test Fixture And Signal Cleanup

Problem:

- `world-model` and `world-runtime` test helpers duplicate many id/key
  constructors.
- Runtime tests rely on seed commit side effects such as total transaction
  count.
- Some tests combine too many lifecycle/failure cases in one function.
- Test setup length makes production behavior harder to see.

Target shape:

- Keep fixtures crate-local for now.
- Split large test files with local modules:
  - `src/tests.rs` may keep top-level scenarios;
  - `src/tests/fixtures.rs` holds id/key/definition/request helpers;
  - additional focused modules can hold process, scheduler, and runtime-control
    test groups if that keeps the test names clearer.
- Replace brittle total-count assertions with baseline/delta helpers.
- Split broad lifecycle tests into focused tests for each transition family.
- Rename reservation tests/fixtures so they do not read as transfer tests.
- Update authority-surface allowlists deliberately when fixture files move.

Acceptance criteria:

- Test failures identify one behavior at a time.
- Seed helpers do not leak into assertions except where seed behavior is the
  subject under test.
- Fixture modules make tests shorter without hiding important scenario setup.
- Reservation fixture names match reservation behavior rather than transfer
  behavior.

## Suggested Order

1. Mechanically split `world-model` runtime-control modules with no behavior
   change.
2. Runtime-control planning/application refactor.
3. Misuse-safety prerequisites from API ergonomics:
   - explicit runtime constructors;
   - explicit optional actor lookup;
   - public re-export audit.
4. Hard commit finalization path alignment, including process transaction cause
   and event-contract policy.
5. Process lifecycle construction cleanup using the runtime-control draft.
6. Test fixture and signal cleanup.
7. Remaining API diagnostics and guardrail polish.

The production cleanup is coordinated, but it should still land in reviewable
slices. Do behavior-preserving movement separately from semantic refactors.
Test cleanup should follow the production shape enough to avoid rewriting tests
twice, but minimal fixture/delta helpers may be introduced earlier if they make
the production refactor easier to verify.

## Verification

Run after each substantial work package:

```bash
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
git diff --check
```

Before considering the cleanup complete, also rerun the accepted package
authority surface guardrail and inspect its failure message by temporarily
adding a local unapproved call in a scratch diff, then removing it before final
verification.
