# Research Index

## Role

Research records evidence, alternatives, case studies, and proposed
falsification tests. It does not amend the
[Target Architecture Package](../architecture/target-architecture/README.md)
by implication.

A recommendation becomes accepted only when its disposition is recorded in
the relevant normative owner and, when executable evidence is required, in a
validation scenario and roadmap gate.

## Architecture synthesis and open falsification

| Document | Role |
|---|---|
| [Architecture Redesign Research Synthesis](architecture-redesign-synthesis.md) | Evidence base for the current target architecture |
| [Gameplay Composition And Evolution Research](gameplay-composition-and-evolution-research.md) | Post-kernel composition hypotheses, fitness properties, and the M8 falsification suite |

The gameplay-composition document deliberately contains both accepted-law
restatements and new hypotheses. Its normative dispositions are maintained in
[Extensibility and Research](../architecture/target-architecture/extensibility-and-research.md);
the milestone evidence belongs to the roadmap and validation scenarios.

## Milestone decision inputs

| Document | Milestone |
|---|---|
| [M2 Deterministic Kernel Research](m2-deterministic-kernel-research.md) | Completed M2 |
| [M3 Grounded Action Research](m3-grounded-action-research.md) | Completed M3 |
| [M4 Agency Lifecycles Research](m4-agency-lifecycles-research.md) | Completed M4 |

These explain why completed implementation choices were selected. Accepted
exit reviews, not research prose, are the completion evidence.

## Domain and evaluation research

- [Cognitive And Agent Research Map](cognitive-agent-research-map.md)
- [Epistemic State And Agent Memory](epistemic-state-and-agent-memory.md)
- [Semantic Appraisal, Intent, Activity, And Planning](semantic-appraisal-intent-activity-planning.md)
- [Social Strategic Evaluation Program](social-strategic-evaluation-program.md)
- [Social Strategic Benchmark Methodology](social-strategic-benchmark-methodology.md)
- [Social Strategic Research Positioning](social-strategic-research-positioning-ko.md)
- [Reference Research Questions](reference-questions.md)

These documents can shape future evaluator internals, social vocabulary,
experiments, and metrics without becoming runtime authority.

## Pre-target research inputs

The following research predates the normative redesign. It remains useful for
evidence and rejected alternatives, but references to older design or
architecture files are historical routing:

- [Engine Architecture Research Entry](engine-architecture-entry.md)
- [Implementation Architecture And Library Survey](implementation-architecture-and-library-survey.md)
- [Runtime Pipeline Implementation Research](runtime-pipeline-implementation-research.md)
- [Causal Runtime, Action, Effect, And Event](causal-runtime-action-effect-event.md)
- [Time Model And Turn Scheduling](time-model-and-turn-scheduling.md)
- [World Representation And Query Model](world-representation-query-model.md)

## Promotion rule

For each research recommendation, record exactly one disposition:

```text
already normative
accepted clarification
accepted new decision
implementation evidence required
deferred hypothesis
rejected
```

Only accepted decisions are copied into a normative owner. Evidence-required
items additionally receive a validation scenario and milestone. This prevents
linking a broad research synthesis from silently accepting every mechanism it
discusses.
