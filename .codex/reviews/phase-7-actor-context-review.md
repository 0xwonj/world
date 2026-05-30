# Phase 7 Actor Context Implementation Review

Date: 2026-05-29

## Scope

This review covers the current uncommitted actor-context implementation:

- `crates/world-context/src/*`
- `crates/world-runtime/tests/dependency_direction.rs`
- `docs/architecture/crates.md`

The review used four independent read-only agents:

- architecture and long-term simulation direction;
- compiler-shaped pipeline and representation boundaries;
- Rust code quality and API design;
- correctness, tests, and guardrails.

Architecture/design documents were used as context, not as absolute source of
truth. The review prioritizes whether the current shape is the right direction
for a simulation-first RPG engine whose core should not know game-specific
logic.

## Executive Assessment

The implementation establishes a good crate-level starting point: `world-context`
has a concrete `ActorContextPipeline`, uses read-only model/definition input,
returns owned context values, avoids runtime dependencies, and does not execute
primitive semantics or choose intent.

The main concern is that the current API can look more complete and
actor-relative than it really is. The highest-risk areas are:

1. `CapabilitySet` and `ActionRepertoire` are derived from global definitions,
   not actor-specific evidence.
2. `ActorContextInput` publicly exposes the full `WorldModel`, leaving an easy
   path to privileged reads in future context or decision code.
3. Empty observation/social/affordance projections are indistinguishable from
   real actor-visible absence unless debug diagnostics are enabled.
4. Read dependencies and invalidation are model-only even though definition
   dependencies are recorded.
5. Guardrails do not yet fully protect the intended Phase 8 boundary.

The current shape is acceptable as a skeleton, but it should be tightened before
building decision logic on top of it.

## Root-Cause Clusters And Ideal Fixes

### Cluster A: Context Boundary Is Not Yet A Narrow Read Capability

Related findings:

- P1: Public input surface exposes full model authority.
- P1: Phase 8 raw model access is not guarded.
- P1: Context-level actor filtering is not tested with data.

Shared root cause:

- `world-context` currently receives a full `&WorldModel` and exposes that input
  publicly. The implementation uses actor-relative reads today, but the API and
  guardrails do not yet make privileged reads structurally hard to reach.

Ideal fix:

- Treat `ActorContextPipeline` as the only public context projection entry
  point.
- Keep raw model access internal to `world-context`, or replace it with a narrow
  actor-scoped projection source.
- Add dependency guardrails so `world-decision` cannot depend on `world-model`.
- Add source guardrails or focused tests that prevent `world-context` from using
  `debug()`, `kernel()`, or direct hard/runtime model reads for actor-facing
  context unless explicitly allowlisted.
- Add a context-level actor-isolation test as soon as there is a legitimate
  accepted epistemic fixture path.

### Cluster B: Definition-Level Schema Discovery Is Mixed With Actor Capability

Related findings:

- P1: Repertoire and capability overclaim actor-relative meaning.
- P3: Capability and repertoire duplicate registry scans.
- P3: Element-level provenance is uneven.

Shared root cause:

- `ActionDef::actor_role()` is being used as both schema metadata and capability
  evidence. It only means the action is actor-facing; it does not mean a
  specific actor can attempt it in context.

Ideal fix:

- Split actor-facing schema candidates from actor-owned capability evidence.
- Keep definition-derived candidates honest with status names that cannot be
  confused with runtime validation or actor entitlement.
- Keep `CapabilitySet` empty or explicitly evidence-backed until actor-specific
  capability sources exist.
- Introduce a private shared action-schema candidate projection if both
  capability and repertoire need the same checked-definition scan.
- Add per-entry evidence/provenance only for entries that decision code will
  inspect individually.

### Cluster C: Projection Completeness Is Not Explicit

Related findings:

- P2: Empty projection stages hide missing semantics.
- P2: Social context is not yet decision-usable.
- P2: Observed event shape reuses checked event schema as visible content.
- P2: `PerceivedAffordance` is coupled to action identity too early.

Shared root cause:

- The first implementation created the major context slots, but some slots are
  placeholders. Their output shape does not always say whether the stage is
  genuinely empty, shallow, or unavailable.

Ideal fix:

- Add projection completeness/status metadata that is always available, not only
  behind debug diagnostics.
- Keep unavailable slots explicitly unavailable until a meaningful shallow
  record-reference view exists.
- Keep checked schemas and committed event specs as provenance/evidence, not as
  actor-perceived content.
- Represent affordances as perceived target/context facts first; derive
  action-binding candidates later by matching repertoire against affordances.

### Cluster D: Dependency Tracking Has Two Kinds Of Inputs But One Staleness Path

Related findings:

- P2: Invalidation is model-only despite definition dependencies.
- P1: Context read/invalidation metadata will be wrong once projection becomes
  real.

Shared root cause:

- `ContextReadSet` records both model authority reads and checked definition
  ids, but the staleness predicate only understands `InvalidationPackage`.
  `InvalidationPackage` is a model-state invalidation mechanism, not a registry
  or definition-version invalidation mechanism.

Ideal fix:

- Keep model invalidation and definition invalidation distinct.
- Rename the current predicate to model-only semantics, or add a separate
  registry/definition invalidation input.
- Before introducing caches, add a registry fingerprint, definition epoch, or
  explicit definition invalidation predicate.

## Accepted Decisions

These decisions are now accepted and should guide the cleanup plan.

1. `world-decision -> world-model` should be forbidden now.

   If Phase 8 is meant to consume actor context, the dependency guardrail should
   block raw model access before decision code exists.

2. `ActorContextInput::model()` should not remain public as raw model authority.

   External callers need to construct input and call the pipeline; they should
   not need direct model access through the context crate API.

3. Definition-derived action schemas are repertoire candidates, not
   capabilities.

   Avoid capability/availability wording for pure schema discovery. Use a
   candidate/status vocabulary that means actor-facing by definition, not
   actor-owned or runtime-valid.

4. `CapabilitySet` should stay empty until actor-specific evidence exists.

   A global action schema belongs in repertoire/catalog, not capability.

5. Projection-unavailable status should always be visible.

   Prefer projection status/completeness metadata rather than debug-only
   diagnostics.

6. Social context should not be filled with count-only data now.

   Keep unsupported social context explicitly unavailable/shallow until it can
   carry meaningful record references, access mode, and provenance.

7. `PerceivedAffordance` should not contain `action: DefinitionId` as its core
   identity.

   Affordance should describe perceived target/context facts. Action binding
   should be a later matching result.

8. Public field-complete constructors should not remain for projected context
   values unless there is a real invariant-preserving external construction use
   case.

   Keep them crate-private for now.

## Findings

### P1: Repertoire And Capability Overclaim Actor-Relative Meaning

References:

- `crates/world-context/src/pipeline.rs:34`
- `crates/world-context/src/capability.rs:102`
- `crates/world-context/src/repertoire.rs:135`
- `crates/world-context/src/tests.rs:48`

`CapabilitySet` and `ActionRepertoire` are derived only from
`DefinitionRegistry`. The request actor and model state are not inputs to those
derivations, so every actor receives every action that declares `actor_role()`.

That means actor-role metadata is effectively being treated as actor capability
evidence. It proves that an action schema is actor-facing, but it does not prove
that this actor owns the capability, knows the procedure, has the equipment, is
socially authorized, or can satisfy contextual requirements.

Root cause:

- schema-level action discovery and actor-owned capability projection are
  represented in the same output vocabulary.

Recommended direction:

- Keep actor-facing action schemas, but label them honestly as definition-level
  candidates rather than capability grants or validated availability.
- Avoid status names like `AvailableByDefinition` if they can be read as
  actor availability. Prefer wording such as `DefinitionCandidate`,
  `ActorFacingSchema`, or another term that clearly means "can be considered in
  principle, not validated for this actor."
- Consider leaving `CapabilitySet` empty until there is actual actor-specific
  evidence, or derive capability entries from a future actor-evidence layer.
- Long term, derive actor repertoire from actor-owned state, epistemic access,
  learned procedures, equipment/body/skill evidence, and social authority. This
  should still be generic and pack-driven, not hardcoded game logic.

### P1: Public Input Surface Exposes Full Model Authority

References:

- `crates/world-context/src/request.rs:19`
- `crates/world-context/src/lib.rs:25`
- `crates/world-model/src/query.rs:16`
- `crates/world-decision/Cargo.toml:10`

`ActorContextInput::model()` publicly returns `&WorldModel`. The current
pipeline uses the actor-relative query surface correctly, but the public API
makes it easy for later context or decision code to reach `query_layer().kernel()`
or `query_layer().debug()`.

Root cause:

- the projection input is a public transport for the full model rather than a
  narrow projection capability.

Recommended direction:

- Make `ActorContextInput` accessors crate-private if external callers only need
  to construct the input and call `ActorContextPipeline::project`.
- Alternatively, expose a narrower projection source that does not include
  debug/kernel surfaces.
- Add a source-level guardrail or focused test that prevents `world-context`
  from using `query_layer().debug()`, `query_layer().kernel()`, or raw hard
  model reads in actor-facing projection code unless explicitly allowlisted.

### P1: Phase 8 Raw Model Access Is Not Guarded

References:

- `crates/world-runtime/tests/dependency_direction.rs:61`
- `docs/architecture/crates.md:146`
- `docs/architecture/crates.md:223`

The dependency-direction test forbids `world-decision -> world-runtime` and
`world-decision -> world-standard-runtime`, but it does not forbid
`world-decision -> world-model`. If Phase 8 depends on `world-model`, it can
bypass `ActorContextPipeline` and read raw model state directly.

Root cause:

- guardrails were tightened for `world-context`, but not for the future decision
  boundary.

Recommended direction:

- Add `world-model` to the forbidden dependency list for `world-decision`.
- Consider converting the dependency test from a partial forbidden-edge list to
  an explicit allowed-dependency matrix. That would also catch missing edges
  such as `world-core -> any world-*` and `world-authoring -> world-context` /
  `world-authoring -> world-decision`.

### P1: Context-Level Actor Filtering Is Not Tested With Data

References:

- `crates/world-context/src/tests.rs:12`
- `crates/world-context/src/epistemic.rs:85`
- `crates/world-model/src/tests.rs:1504`

`world-model` tests prove that `ActorRelativeQuery` filters epistemic records by
actor. `world-context` tests only project an empty model. A future change from
`actor_relative()` to a broader query could still pass the current
`world-context` tests.

Root cause:

- tests protect output shape and dependency bookkeeping, but not actor isolation
  at the context boundary.

Recommended direction:

- Add a context-level test with actor A and actor B epistemic records, proving
  that actor A sees only actor A records through `ActorContextPipeline`.
- Avoid backdoor test fixtures that weaken the model authority boundary. If the
  public soft-authority write gate is not available yet, this should be tracked
  as a test gap until that gate exists.

### P2: Empty Projection Stages Hide Missing Semantics

References:

- `crates/world-context/src/observation.rs:138`
- `crates/world-context/src/social.rs:52`
- `crates/world-context/src/affordance.rs:71`
- `crates/world-context/src/pipeline.rs:27`

Observation, social, and affordance stages currently return empty values. They
emit `ProjectionUnavailable` only when debug diagnostics are enabled. A normal
consumer cannot distinguish "the actor sees nothing" from "this projection
family has not been implemented."

Root cause:

- projection completeness is represented as optional debug information rather
  than normal context metadata.

Recommended direction:

- Always record projection status/completeness in `ContextProjectionReport`, or
  in each projection slot.
- Keep source-free structured diagnostics, but do not make semantic
  completeness depend on a debug flag.
- Tests should assert that unavailable/shallow projection families are visible
  in the report even when the domain output is empty.

### P2: Social Context Is Not Yet Decision-Usable

References:

- `crates/world-context/src/social.rs:6`
- `crates/world-context/src/social.rs:52`
- `crates/world-model/src/query.rs:121`

`SocialContextView` contains only three counters and currently always returns
empty. Existing `world-model` already exposes semantic-context counts plus read
labels, but count-only data is not enough to serve as decision context.

Root cause:

- the skeleton created a typed slot, but count-only data would make the slot
  look more decision-ready than it is.

Recommended direction:

- Do not populate `SocialContextView` with count-only data now.
- Keep the slot explicitly unavailable/shallow in projection metadata.
- Add minimal record references, access mode, and provenance anchors when social
  context becomes a real decision input.

### P2: `PerceivedAffordance` Is Coupled To Action Identity Too Early

References:

- `crates/world-context/src/affordance.rs:10`
- `docs/design/capability-affordance-and-actor-interface.md:29`

`PerceivedAffordance` stores `action: DefinitionId`. That couples actor-visible
target/context facts directly to action schemas. In the intended model,
affordance should describe what the actor perceives about the subject/context;
matching that against repertoire and role requirements is a later binding step.

Root cause:

- perceived context and action-binding candidates are collapsed into one type.

Recommended direction:

- Represent affordances by perceived kind, target/context, status, confidence,
  and provenance.
- Keep action ids in a later binding/candidate layer, or make the field clearly
  optional evidence produced by a matcher rather than the identity of the
  affordance itself.

### P2: Report Can Be Dropped Too Easily By Decision Code

References:

- `crates/world-context/src/context.rs:14`
- `crates/world-context/src/context.rs:90`
- `crates/world-context/src/context.rs:119`

The split between `ActorContext` and `ContextProjectionReport` is clean, but it
makes `.context()` an attractive lossy handoff. If Phase 8 consumes only
`ActorContext`, it may lose read dependencies, provenance, and projection
diagnostics.

Root cause:

- context values and projection evidence are sibling outputs, not a single
  decision-facing input.

Recommended direction:

- Make Phase 8 consume `ActorContextProjection`, or introduce a
  `DecisionContextInput` wrapper that includes both context and report.
- If `ActorContext` remains the handoff type, embed an evidence/report view or
  make provenance item-addressable from the context.

### P2: Invalidation Is Model-Only Despite Definition Dependencies

References:

- `crates/world-context/src/dependency.rs:79`

`ContextReadSet` records `DefinitionId`s, but `is_invalidated_by()` only checks
`InvalidationPackage` authority/store labels. Capability and repertoire are
currently definition-derived, so registry changes cannot stale cached context.

Root cause:

- model invalidation and definition/registry invalidation are conflated in one
  read set.

Recommended direction:

- Rename `is_invalidated_by()` to make model-only semantics explicit, or add a
  separate definition/registry invalidation path.
- Before adding real caches, introduce a registry fingerprint, definition epoch,
  or direct definition invalidation predicate.

### P2: Public Value Constructors Can Forge Projected Context

References:

- `crates/world-context/src/observation.rs:53`
- `crates/world-context/src/observation.rs:96`
- `crates/world-context/src/affordance.rs:17`
- `crates/world-context/src/repertoire.rs:107`

Most context structs have private fields, but several field-complete
constructors are public. External code can synthesize values that look like
pipeline output, and the constructor signatures become API commitments before
the projection semantics are stable.

Root cause:

- value-object constructors were exposed before there is a clear external
  authoring or test-support use case.

Recommended direction:

- Keep field-complete constructors `pub(crate)` for now.
- If external construction is needed later, use narrow builders that require
  explicit provenance/source/status and preserve invariants.

### P2: Observed Event Shape Reuses Checked Event Schema As Visible Content

References:

- `crates/world-context/src/observation.rs:87`
- `crates/world-defs/src/events.rs:9`

`ObservedEvent` stores `EventRecordSpec`. That is checked schema metadata, not
necessarily what an actor perceived. Perception needs room for perceived kind,
roles, channel, confidence, uncertainty, and recognition state.

Root cause:

- schema-level event metadata is being reused as actor-visible observation
  content.

Recommended direction:

- Treat committed event/schema ids as provenance or backing evidence.
- Add actor-visible perceived event fields only when the perception model can
  justify them.

### P3: Capability And Repertoire Duplicate Registry Scans

References:

- `crates/world-context/src/capability.rs:106`
- `crates/world-context/src/repertoire.rs:139`

Both modules scan `definitions.actions()`, filter on `actor_role()`, and record
similar provenance. The current duplication is small, but it reflects the same
semantic confusion as the overclaiming issue.

Root cause:

- there is no private intermediate projection for actor-facing action schemas.

Recommended direction:

- Introduce a private action-schema candidate projection helper, or derive one
  output from the other after the semantics are clarified.
- Do not add a broad trait/pass framework; a private helper is enough.

### P3: Element-Level Provenance Is Uneven

References:

- `crates/world-context/src/affordance.rs:14`
- `crates/world-context/src/capability.rs:36`
- `crates/world-context/src/repertoire.rs:36`

`PerceivedAffordance` has per-entry provenance, but capability and repertoire
entries rely on the global report. This makes it harder to explain why one
specific candidate exists.

Root cause:

- provenance is gathered globally before entry-level explanation requirements
  are known.

Recommended direction:

- Add per-entry evidence/provenance anchors for entries that decision code will
  inspect individually, or make the global report item-addressable.

## Positive Findings

- The crate dependency direction is broadly correct: `world-context` does not
  depend on `world-runtime`, `world-standard-runtime`, `world-decision`, or
  `world-authoring`.
- A concrete `ActorContextPipeline` is the right first shape. No generic pass
  manager or plugin abstraction has leaked into the API.
- Outputs are owned values rather than long-lived model borrows.
- Diagnostics are structured and source-free; there is no parser/span
  dependency in `world-context`.
- The implementation does not stage runtime effects, install primitive
  semantics, or choose intent.

## Recommended Remediation Order

1. Tighten guardrails:
   - forbid `world-decision -> world-model`;
   - add missing dependency-direction edges or move to an allowed matrix;
   - add a source guardrail against `debug()` / `kernel()` use in
     `world-context`.

2. Narrow the projection input surface:
   - make raw model accessors crate-private, or replace them with a narrower
     actor-context projection source.

3. Correct capability/repertoire semantics:
   - stop presenting global definition-derived action schemas as actor-owned
     capability;
   - use honest candidate/status names;
   - consider keeping `CapabilitySet` empty until actor evidence exists.

4. Make projection completeness explicit:
   - always report unavailable/shallow projection families;
   - do not hide semantic incompleteness behind debug diagnostics.

5. Improve social context status:
   - keep social explicitly unavailable/shallow rather than silently empty;
   - do not populate count-only social context;
   - add record references, access mode, and provenance when the slot becomes
     decision-usable.

6. Clarify affordance and observation value shapes:
   - avoid action-coupled affordance identity;
   - keep checked event schema as provenance/evidence, not actor-visible event
     content.

7. Strengthen tests:
   - add context-level actor filtering when a legitimate accepted epistemic
     fixture path exists;
   - pin no hard/debug/runtime read labels in actor-context projection;
   - test projection completeness metadata;
   - test dependency guardrails for decision/model boundaries.

## Verification Reported By Agents

Agents reported the following read-only checks:

- `cargo fmt --all --check`
- `cargo check -p world-context`
- `cargo clippy -p world-context --all-targets`
- `cargo test -p world-context`
- `cargo test -p world-model actor_relative_query_filters_epistemic_records_by_actor`
- `cargo test -p world-runtime --test dependency_direction`
- `cargo check --workspace`

The main implementation turn previously passed:

- `cargo fmt --all --check`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo test --workspace --all-targets`
- `git diff --check`
