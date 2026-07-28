# Documentation Guide

## One active architecture

`world` has one normative cross-system architecture:

- [Target Architecture Package](architecture/target-architecture/README.md)

All other documentation has a narrower role. A design note can explain a
domain deeply, research can challenge an assumption, and an implementation
plan can choose concrete work, but none may silently redefine authority,
ownership, dependency direction, lifecycle, persistence, or extension trust.
The [Architecture Index](architecture/README.md) routes every frozen sibling
document and lists the complete normative package.

## Documentation layers

| Layer | Purpose | May change normative architecture? |
|---|---|---|
| [Vision](vision.md) and [Reference Game Vision](design/reference-game-vision.md) | Product direction and required pressure | No; they create requirements that must be assigned to normative owners |
| [Target architecture](architecture/target-architecture/README.md) | Accepted cross-system contracts, decisions, validation, and roadmap | Yes, through an explicit owner update and decision |
| [Design](design/README.md) | Nonnormative domain and subsystem models | No |
| [Research](research/README.md) | Evidence, alternatives, and falsification proposals | No |
| [Implementation execution](implementation/target-rewrite/README.md) | Active milestone plans and completion evidence | No; it refines accepted contracts |
| `ideas/`, `brainstorming/`, and `references/` | Raw inputs and case-study material | No |
| `architecture/archive/` and any design archive | Frozen history | No |

## Reading paths

For the complete system:

```text
Vision
  -> Reference Game Vision
  -> Target Architecture README
  -> System Architecture
  -> Formal Model
  -> Code Architecture
  -> Roadmap and Coverage
  -> Active implementation status
```

For implementation work:

```text
AGENTS.md
  -> target contract owner for the affected boundary
  -> validation scenario
  -> implementation roadmap milestone
  -> current execution status and active milestone plan, when present
  -> current code and tests
```

For a new gameplay or AI capability:

```text
design/research evidence
  -> explicit disposition
  -> normative owner and decision, if cross-system
  -> validation scenario
  -> milestone owner and exit gate
  -> vertical implementation and completion evidence
```

## Completion rule

A capability is not part of the executable architecture merely because a
vision, design, or research document describes it. A required capability is
closed only when it has:

1. one normative contract owner;
2. one state/authority owner and extension tier;
3. a concrete producer and consumer;
4. a failure, versioning, and persistence story where relevant;
5. a named roadmap milestone;
6. a falsifiable validation scenario and exit assertion;
7. implementation evidence when that milestone is complete.

The target architecture maintains the trace from requirement to evidence.
Documented later scope is honest deferral, but it does not count as proof that
the complete product vision is executable.

## Change discipline

- Correct stale summaries at their source; do not add another competing
  definition.
- Move obsolete cross-system plans to an archive or mark them frozen.
- Keep final DSLs, algorithms, and taxonomies local until a real producer,
  consumer, invariant, and scenario require a shared contract.
- Promote research recommendations individually. Linking a research document
  does not accept all of its proposals.
- Prefer a small vertical counterexample over a broad placeholder framework.
- Preserve accepted implementation evidence even when later plans change.
