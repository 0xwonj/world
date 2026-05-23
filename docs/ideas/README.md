# Design Ideas

This directory contains structured candidate design notes.

An idea note is more concrete than brainstorming, but less authoritative than a
design document. It records a possible model, why it is interesting, how it
would affect state/action/event/observation boundaries, and what risks remain.

Ideas can later be promoted into `docs/design/`, revised, split, or rejected.

## Status Labels

- `Exploratory`: early thought, not yet shaped into a candidate model.
- `Candidate`: promising enough to compare against other ideas.
- `Accepted`: adopted as a design principle and should be reflected in
  `docs/design/`.
- `Rejected`: intentionally not adopted, with the reason preserved.

## Working Rules

- Keep ideas tied to source-of-truth boundaries.
- Separate capability, action, event, state, and observation effects.
- Do not treat an idea as a feature requirement until it is accepted.
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
