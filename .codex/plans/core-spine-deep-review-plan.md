# Core Spine Deep Review Plan

## Purpose

Review the current codebase after the Phase 0-5 core spine implementation and
before Phase 6 expands the engine through the standard world library layer.

This review is not limited to Phase 5. It should evaluate the whole current
workspace shape, with special attention to `world-core`, `world-defs`,
`world-model`, and `world-runtime`.

## Review Tracks

1. Code quality and Rust idiom
   - Check whether the code is concise, readable, and domain-shaped.
   - Review function/module size, naming, visibility, error/outcome shape,
     ownership, cloning/allocation, iterator use, enum/newtype use, and helper
     extraction opportunities.
   - Look for duplication, overly broad abstractions, missing small
     abstractions, and tests that normalize awkward production APIs.

2. Authority and public API boundaries
   - Check crate dependency direction, re-export surfaces, visibility, accepted
     package boundaries, and model/runtime ownership.
   - Confirm that mutation authority stays in runtime paths and that model
     apply APIs remain receiver surfaces.
   - Evaluate whether the source allowlist guardrail is strict enough without
     pretending to provide external compile-time forge prevention.

3. Architecture and design challenge
   - Actively challenge the current design rather than only validating it.
   - Compare the structure against compiler/runtime/game-simulation patterns.
   - Reconsider whether the model/runtime split, process/scheduler/control
     model, accepted package shape, and future primitive semantics boundary are
     the right long-term choices.

4. Runtime correctness and domain semantics
   - Review scheduler, wakeup, process lifecycle, reservation, hard commit, and
     runtime-control semantics inside the chosen architecture.
   - Focus on lost work, stale work, blocked work, atomicity, provenance,
     invalidation, replay posture, and failure/outcome separation.

5. Future phase readiness
   - Check whether Phase 6 standard world library work can move built-in
     primitive semantics out of runtime cleanly.
   - Check whether Phase 7 actor context, Phase 8 decision, Phase 9 authoring,
     and Phase 10 engine facade can attach without receiving hidden mutation
     authority.

6. Test architecture and verification
   - Review whether tests protect behavior, invariants, authority boundaries,
     and cross-crate contracts.
   - Check whether tests are too brittle, too implementation-coupled, or too
     narrow around failure paths.
   - Confirm the verification command set remains appropriate before commit.

## Suggested Execution

- Run the tracks in order, but keep findings separate by root cause rather than
  by file.
- Use multiple review agents for the architecture/design challenge track:
  compiler/runtime architecture, simulation/game engine architecture, and Rust
  crate/API architecture.
- Do not fix findings immediately during review. First collect findings,
  merge duplicates, group them by root cause, and decide which fixes should be
  handled before committing the Phase 0-5 core spine.

## Expected Output

- Findings grouped by severity and root cause.
- Clear distinction between bugs, design risks, code quality issues, and future
  phase readiness gaps.
- A recommended fix plan for issues that should be resolved before Phase 6.
