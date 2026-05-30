# Phase 10 Research Plan: Social-Cognitive Representation Slice

## Status

Research plan before Phase 10 design and implementation.

## Purpose

Phase 10 research should identify the first concrete social-cognitive
representation families to implement on top of the existing `world-decision`
substrate.

The research goal is not to choose a general cognitive architecture. It is to
decide which typed artifacts, pass contracts, and trace semantics are needed to
compare small decision profiles such as:

- direct action vs typed speech;
- no other-model vs bounded other-model;
- unstructured decision signal vs structured motivation or strategic signal.

## Source Precedence

When local documents conflict, use the newest active documents in this order:

1. `docs/architecture/implementation-plan.md`
2. `docs/architecture/configurable-decision-pipeline.md`
3. current `docs/design/` documents
4. current `docs/research/` documents
5. `.codex/research/` and `.codex/plans/` phase-local notes

Archived architecture documents are historical context only.

## Execution Rule

Run the steps sequentially. Do not start a later step until the previous step's
research output exists under `.codex/research/` and is usable as input.

Subagents may be used freely inside each step for literature review, local
document synthesis, API-shape critique, or risk review. The main agent should
consolidate their results into the step output.

## Steps

### 1. Local Constraints

Output:

`.codex/research/phase-10-01-local-constraints.md`

Extract the current Phase 10 boundaries from active architecture, design, and
research documents. Record stable constraints, replaceable implementation
areas, deferred work, and document conflicts resolved by source precedence.

### 2. Broad External Survey

Output:

`.codex/research/phase-10-02-broad-external-survey.md`

Survey external work broadly enough to identify useful lenses: speech acts,
social commitments, BDI and intention-as-commitment, appraisal theory, bounded
theory of mind, signaling, bargaining, and social-agent benchmarks.

### 3. Representation Family Synthesis

Output:

`.codex/research/phase-10-03-representation-family-synthesis.md`

Map local constraints and external findings into candidate representation
families. Decide which families should be first-class Phase 10 candidates and
which should remain deferred.

### 4. Family-Specific Deep Research

Output:

`.codex/research/phase-10-04-family-specific-deep-research.md`

Research the selected families in enough detail to support concrete typed
artifacts. Focus on speech, commitment, bounded other-model, motivation, and
strategic assessment only where they support comparable traces.

### 5. Artifact Shape And Ablation Plan

Output:

`.codex/research/phase-10-05-artifact-shape-and-ablation-plan.md`

Propose minimal payload shapes, representation roles or role additions, pass
input/output contracts, trace semantics, and ablation pairs. Mark which parts
are stable contracts and which executor algorithms can be replaced later.

### 6. Consolidated Research Handoff

Output:

`.codex/research/phase-10-social-cognitive-representation-slice-research.md`

Consolidate the step outputs into one research handoff for the design and
implementation plan. Include the recommended minimal slice, alternatives
rejected, deferred work, and implementation risks.

## Done Criteria

- Each step output exists in `.codex/research/`.
- The final handoff identifies 3-5 representation families or explicitly
  explains why fewer are enough.
- The final handoff separates stable typed contracts from replaceable executor
  implementations.
- The final handoff names at least two comparable trace ablations Phase 10 can
  implement.
- The final handoff does not introduce new crate dependency direction,
  persistence, parser, model-provider, or runtime mutation authority decisions.
