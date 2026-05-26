# Axis Research Template

## Status

Draft research

## Axis

Name the research axis from
[Engine Architecture Research Entry](../engine-architecture-entry.md).

## Core Question

State the one question this axis must answer.

## Why This Must Be Researched Together

Explain why the decisions in this axis are coupled. This should justify the
axis boundary, not summarize the whole topic.

## Scope

In scope:

- ...

Out of scope:

- ...

## Theory Baseline

Use this section for thin theory research before deep design.

For each theory source or concept:

- what problem it addresses
- what vocabulary it gives us
- what assumptions do not transfer cleanly to `world`
- what design pressure it creates

Avoid turning this into a literature survey. Capture only the theory needed to
make engine design decisions.

## Reference Targets

List the games, engines, simulations, papers, or systems to inspect.

For each reference:

- why it matters for this axis
- what system or behavior to inspect
- what implementation detail or design pattern to look for
- what would count as a transferable principle

## Observations

Separate observed mechanics from inferred implementation.

Recommended format:

```text
Observation:
  What the reference appears to do.

Inference:
  What implementation or architecture this may imply.

Transfer:
  Keep, adapt, reject, or unresolved.
```

## Design Decisions

List the decisions this axis must eventually settle.

Examples:

- What is authoritative state?
- What is derived?
- What is actor-relative?
- What is semantic?
- What must emit structured events?
- What can be authored through a checked PL/DSL?
- What must remain host-code kernel behavior?

## Candidate Models

Record candidate designs without choosing too early.

For each candidate:

- model sketch
- what it makes easy
- what it makes hard
- likely failure modes
- relation to existing `world` notes

## Failure Modes

List ways this axis can go wrong.

Examples:

- tag soup
- hidden mutation path
- omniscient actor interface
- over-generalized abstraction
- feature-specific hardcoding
- nondeterminism without provenance
- PL surface that is expressive but uncheckable

## Test Scenarios

Use concrete scenarios to pressure-test the axis.

Each scenario should describe:

- initial world state
- actor perspective
- attempted action or process
- expected events
- expected observations
- semantic consequences, if relevant
- what the scenario reveals about the design

## Open Questions

Keep unresolved questions explicit.

Prefer questions that block design decisions over general curiosity.

## Takeaways For `world`

Summarize only what should affect `world`.

Use:

- Keep
- Adapt
- Reject
- Defer
