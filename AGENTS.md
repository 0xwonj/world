# AGENTS.md

## Repository Contract

This repository is an early Rust implementation of a simulation-first RPG
engine. Start documentation work at `docs/README.md`. The only normative
cross-system architecture lives under
`docs/architecture/target-architecture/`. `docs/design/` and `docs/research/`
are individually classified nonnormative inputs by their local indexes.
Older files directly under `docs/architecture/` are frozen legacy planning,
not alternate contracts.

A required capability is architecture-complete only when it has a normative
owner, validation scenario, roadmap milestone, and completion evidence. A
research link or design description alone does not make it part of the
executable target.

## Working Rules

- Check `git status --short` before editing.
- Do not revert unrelated dirty work.
- Preserve the crate dependency direction described by the target architecture
  docs.
- Keep implementation minimal and domain-shaped; prefer concrete types and
  narrow APIs over broad framework traits.
- Ask before changing architecture boundaries, crate dependency direction,
  persistence/backend choices, or adding major dependencies.
- Discuss before broad documentation rewrites unrelated to the implementation
  task.

## Naming And Comments

Do not encode planning terms into code. Names, comments, tests, and diagnostics
must describe domain meaning, not project-management stages.

Avoid names or comments like `phase1`, `temporary for Phase 2`, `for the
current plan`, or `handoff`.

Comments should explain non-obvious domain or implementation reasoning for
future maintainers. Do not use comments to narrate the work, justify choices to
the user, or restate obvious code.

## Verification

After meaningful Rust changes, run the relevant subset of:

```bash
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
git diff --check
```

Add targeted tests when they protect behavior, authority boundaries, or
cross-crate contracts.
