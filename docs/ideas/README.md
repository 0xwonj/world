# Design Ideas

## Role

This directory contains structured candidate design notes.

An idea note is more concrete than brainstorming, but less authoritative than a
design document. It records a possible model, why it is interesting, how it
would affect state/action/event/observation boundaries, and what risks remain.

Ideas can later be promoted into `docs/design/`, revised, split, or rejected.
Neither an idea nor its promotion into design changes the normative
[Target Architecture](../architecture/target-architecture/README.md).

## Status Labels

- `Exploratory`: early thought, not yet shaped into a candidate model.
- `Candidate`: promising enough to compare against other ideas.
- `Accepted`: adopted as a nonnormative design principle and should be
  reflected in `docs/design/`.
- `Promoted source history`: stable content has moved into `docs/design/`;
  the idea remains as historical context and may use older terminology.
- `Rejected`: intentionally not adopted, with the reason preserved.

## Working Rules

- Keep ideas tied to source-of-truth boundaries.
- Separate capability, action, event, state, and observation effects.
- Do not treat an idea as an executable feature requirement until it has a
  normative owner, roadmap milestone, validation scenario, and eventual
  completion evidence.
- Capture concrete examples, but avoid locking the whole project to one
  reference game.
- Prefer reusable simulation grammar over one-off content lists.

## Lightweight Example

Idea notes do not need a fixed template. Use only the sections that help the
idea.

A small note can be as light as:

```md
# Idea Name

## Status

Exploratory

## Core Idea

## Examples

## Open Questions
```

Longer notes can add sections for motivation, state implications, action/event
boundaries, observation effects, risks, or related references when those details
are useful.
