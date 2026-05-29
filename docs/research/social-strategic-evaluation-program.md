# Social Strategic Evaluation Program

## Status

Research program draft.

This document records the research framing for `world` as a platform for
studying social-strategic behavior in language-model agents. It is not an
implementation plan, benchmark specification, or paper outline. It should guide
later design and evaluation work without replacing the architecture documents
that define engine authority, state ownership, and runtime boundaries.

## Purpose

The research value of `world` should not be framed as "an RPG engine with
LLM agents." That framing is too broad and already overlaps with existing
agent environments, social simulations, and game benchmarks.

The stronger research framing is:

```text
`world` is a causally auditable simulation substrate for generating and
evaluating mixed-motive social dilemmas for language agents under partial
knowledge, social commitments, institutional rules, and long-horizon
consequences.
```

The core question is:

```text
Which structured social-cognitive representations help language agents reason
and act strategically in situations involving private beliefs, public
commitments, norms, sanctions, deception, reputation, and delayed consequences?
```

This question makes the simulation engine a research instrument. The engine is
valuable only insofar as it can generate controlled social situations, expose
typed actor-relative context, run comparable agent policies, and produce traces
that support process-aware evaluation.

## Target Research Contribution

The target contribution is not a new universal model of human cognition. It is
not a claim that an RPG engine by itself is novel. The target contribution is a
controlled evaluation substrate:

```text
fixed authority and causal runtime
  + configurable social-cognitive decision structure
  + generated mixed-motive social dilemmas
  + process-aware metrics over belief, speech, commitment, action, and outcome
```

This contribution should let experiments ask whether an agent behaves better
when it receives or produces structured representations such as:

- actor-relative observations rather than omniscient world state
- bounded epistemic working sets rather than raw memory transcripts
- explicit social claims, norms, obligations, and sanctions
- typed speech acts rather than ungrounded dialogue text
- limited models of other actors' beliefs, goals, and likely sanctions
- motivational or strategic signals before intent selection
- durable commitments and activity state rather than direct per-turn actions
- local and multi-resolution consequences rather than single-turn rewards

The research target is therefore not merely to build a benchmark. It is to
study how the representation of social context changes language-agent behavior.

## Motivation

Language-model agents are increasingly evaluated in social games and interactive
environments. Existing work shows that this is an active research area:

- [CICERO](https://pubmed.ncbi.nlm.nih.gov/36413172/) demonstrated human-level
  play in Diplomacy by combining language models with strategic reasoning in a
  game that requires cooperation, competition, and natural-language negotiation.
- [AvalonBench](https://avalonbench.github.io/) evaluates multi-agent LLMs in a
  hidden-role social deduction game.
- [MafiaBench](https://www.mafiabench.org/) evaluates LLM agents in a deception
  and persuasion-heavy social deduction setting.
- [Cattle Trade](https://arxiv.org/abs/2605.14537) evaluates bluffing, bidding,
  bargaining, opponent modeling, and resource allocation in a long-horizon
  multi-agent game.
- [M3-BENCH](https://arxiv.org/abs/2601.08462) studies mixed-motive games with
  process-aware analysis of behavior, reasoning, and communication.
- [MACHIAVELLI](https://arxiv.org/abs/2304.03279) evaluates reward pursuit,
  power seeking, deception, and ethical tradeoffs in text-game environments.
- [Concordia](https://github.com/google-deepmind/concordia) supports generative
  social simulation for multi-agent experiments.

These systems make the opportunity real, but they also make weak novelty claims
unsafe. It is not enough to say that LLMs can negotiate, deceive, cooperate, or
play social games. That has already been studied.

The gap `world` should target is more specific:

```text
Many benchmarks evaluate agents inside a fixed game or fixed protocol. `world`
should evaluate agents in generated social dilemmas where the causal sources of
strategic pressure can be varied and inspected: who knows what, who believes
what, who promised what, which norms apply, who can punish whom, what is
observable, and how local choices create delayed social consequences.
```

This gap matters because fixed games often make it difficult to tell which
part of the situation caused a model's failure. If an agent fails in a social
deduction game, the failure may come from hidden-role inference, deception
timing, speech grounding, opponent modeling, norm interpretation, long-horizon
planning, or simple action selection. A generative simulation substrate can vary
these factors independently enough to produce stronger diagnosis.

## Problem Statement

The central problem can be stated as:

```text
How can we generate and evaluate mixed-motive social dilemmas for language
agents in which success depends on actor-relative knowledge, explicit social
commitments, social/institutional rules, communication acts, and durable
consequences rather than only on immediate task reward?
```

A good experimental environment for this problem should support:

- partial and false beliefs
- stale memory and rumor provenance
- private secrets and disclosure risk
- typed social claims over ownership, permission, role, obligation, and debt
- norms, laws, taboos, jurisdiction, and enforcement capacity
- promises, threats, offers, accusations, denials, and confessions as traceable
  social actions
- durable commitment lifecycles: made, accepted, witnessed, fulfilled,
  breached, excused, sanctioned
- local action consequences and delayed reputation or faction consequences
- comparable agent policies that can use raw text, structured state, LLM
  proposals, rule-based signals, or oracle information under controlled
  conditions

The main evaluation question is not only:

```text
Did the agent win?
```

It is:

```text
What did the agent observe, believe, infer, say, commit to, attempt, violate,
repair, or exploit, and how did those steps affect both local and long-horizon
outcomes?
```

## Why `world`

The existing `world` design is aligned with this research target because its
core documents already emphasize:

- simulation-core-first state authority and causal mutation in
  [Simulation Core](../design/simulation-core.md)
- hard truth, soft truth, actor truth, appraisal state, and commit gates in
  [Truth, Authority, And Layer Boundaries](../design/truth-authority-and-layer-boundaries.md)
- actor-relative observation in
  [Perception And Observation](../design/perception-and-observation.md)
- holder-relative memory, belief, rumor, secret, confidence, salience, and
  provenance in [Epistemic State](../design/epistemic-state.md)
- social claims, norms, law, obligation, debt, duty, promise, reputation, and
  jurisdiction in [Social Institutional Model](../design/social-institutional-model.md)
- meaning and motivational pressure in
  [Semantic Appraisal And Motivation](../design/semantic-appraisal-and-motivation.md)
- intent as a commitment boundary in
  [Intent Templates And Planning](../design/intent-templates-and-planning.md)
- resolution-aware execution and wider-world consequence preservation in
  [Multi-Resolution Simulation](../design/multi-resolution-simulation.md)
- staged, typed, provenance-aware transformation in
  [Simulation Transition Compiler](../design/simulation-transition-compiler.md)

This alignment does not make the research contribution automatic. It means the
architecture has the right pressure: actor-relative inputs, typed social state,
commit gates, causal records, and explainable intermediate representations.

The research program should preserve that pressure while avoiding overclaiming.

## Novelty Claim

The novelty claim should be narrow and defensible.

Weak claims:

- "We introduce LLM agents in an RPG world."
- "We simulate human cognition."
- "We model belief, emotion, and intent."
- "We evaluate deception and negotiation."
- "We provide an event log for replay."

These are weak because neighboring work already covers LLM social simulation,
agent-game benchmarks, BDI-style separation, appraisal-inspired architectures,
and replayable agent environments.

Stronger claim:

```text
We introduce a causally auditable generator of social-strategic dilemmas in
which the experimenter can vary the typed social-cognitive structure available
to language agents and evaluate the resulting behavior through process-level
traces rather than outcome reward alone.
```

The novelty should come from the combination of:

1. **Generated social dilemma structure**

   Scenarios are not only fixed games. They are assembled from typed variables
   such as hidden preference, false belief, obligation, ownership claim,
   witness, sanction capacity, reputation context, promise, and delayed
   consequence.

2. **Configurable social-cognitive representation**

   Experiments can compare direct action, intent-only, appraisal-like,
   theory-of-mind, typed speech, raw speech, rule-based, LLM-generated, and
   oracle-assisted conditions under the same causal runtime.

3. **Actor-relative and authority-bounded traces**

   The trace records what the actor could observe, what the actor believed or
   was given as context, which social context was accessible, what speech act
   was made, which commitment was selected, and what hard or social consequence
   followed.

4. **Process-aware social-strategic metrics**

   Evaluation should measure belief calibration, promise consistency,
   deception attempt and success, sanction awareness, opponent-model quality,
   norm compliance, reputation cost, utility/regret, and action-outcome
   consistency.

This is still a risky novelty claim. It must be supported by experiments that
show the substrate reveals model differences or failure modes that fixed-game
or outcome-only benchmarks hide.

## Research Method

The research method should treat the engine as an experimental apparatus.

### Scenario Families

The first benchmark suite should use families rather than one-off scenarios.
Each family should expose variables that can be varied while keeping the task
recognizable.

Candidate families:

- hidden-preference bargaining
- promise under temptation
- witness, rumor, and sanction
- sacred or institutionally claimed object removal
- faction duty conflict
- blackmail or secret disclosure
- reputation market
- false accusation and evidence discovery
- costly punishment
- trust repair after breach

Each family should have:

- controlled initial state
- social and epistemic variables
- actor-specific observations
- allowed action and speech spaces
- success and failure conditions
- process-level metrics
- oracle traces for sanity checks

### Configurable Decision Conditions

The research should compare conditions, not only agents.

Examples:

```text
direct action:
  actor-facing observation -> LLM policy -> ActionRequest

intent only:
  actor-facing observation -> candidate intents -> LLM or rule selection
  -> ActionRequest

structured social context:
  observation + EpistemicWorkingSet + SocialContextView -> policy

typed speech:
  natural language + SpeechAct classification -> social update / commitment

theory-of-mind:
  actor-relative context -> OtherModelView -> policy

oracle upper bound:
  controlled privileged information is exposed with explicit oracle labeling
```

The design should avoid making any one cognitive structure sacred. Appraisal,
pressure, theory-of-mind, strategic assessment, bargaining leverage, and typed
speech acts should be treated as typed representations that can be introduced,
removed, or substituted when the experiment demands it.

### Implementation Modes

For each relevant pass or representation, experiments should be able to compare
different implementation modes:

```text
off:
  representation is absent.

heuristic:
  hand-written or rule-based approximation.

typed rule:
  checked game-system or benchmark rule.

LLM proposal:
  model proposes a typed representation through a gate.

hybrid:
  rule system constrains or verifies LLM proposals.

oracle:
  controlled upper-bound signal, explicitly marked and not actor-realistic.
```

The important rule is that different implementation modes should emit the same
typed output shape when they are meant to be compared. Otherwise the experiment
will compare incompatible agent interfaces rather than cognitive structure.

### Metrics

Outcome reward should be reported, but it should not be the primary research
object.

Candidate metric families:

- belief calibration: whether the agent's selected action matches the evidence
  available to its actor
- belief update: whether new observations, testimony, and contradiction change
  later behavior
- promise consistency: whether public commitments align with later actions
- breach handling: whether the agent repairs, excuses, conceals, or ignores a
  breached commitment
- deception attempt: whether an agent communicates a claim that conflicts with
  its own actor-relative belief or intent
- deception success: whether the target's later belief or action changes in
  the intended direction
- sanction awareness: whether the agent accounts for witness, authority, law,
  taboo, reputation, or punishment capacity
- opponent modeling: whether the agent's choices reflect plausible beliefs
  about another actor's knowledge, goals, commitments, or likely response
- norm compliance: whether the agent obeys or violates explicit norms under
  pressure
- long-horizon regret: whether short-term gain leads to delayed loss through
  reputation, faction, punishment, or lost cooperation
- process validity: whether the trace respects actor access boundaries and does
  not rely on hidden truth leakage

Where possible, metrics should be computed from typed traces rather than from
LLM judges. LLM judges may summarize or classify where necessary, but the
primary metrics should come from engine state, speech-act records, commitment
records, event records, and accepted social or epistemic updates.

## Paper Shape

The strongest paper shape is:

```text
Generating Social Dilemmas for Strategic Language Agents under Partial
Knowledge, Commitments, and Long-Horizon Consequences
```

Possible abstract-level story:

1. Existing LLM social-agent evaluations often use fixed games or fixed
   protocols.
2. Fixed games make it difficult to isolate which social-cognitive variables
   cause strategic success or failure.
3. We introduce a simulation substrate that generates mixed-motive social
   dilemmas from typed actor-relative belief, social commitment, institutional
   rule, communication, and consequence variables.
4. We evaluate frontier LLM agents under multiple representation conditions,
   including direct action, structured context, typed speech, theory-of-mind,
   and oracle conditions.
5. We show which structured representations improve or fail to improve
   behavior, and identify failure modes in belief updating, commitment
   consistency, deception, sanction awareness, and long-horizon tradeoffs.

The paper should be written as an empirical evaluation paper, not as a software
architecture paper. The architecture matters because it enables controlled
interventions and trace-based measurement.

## Venue Fit

### AAMAS

[AAMAS](https://cyprusconferences.org/aamas2026/call-for-papers-main-track/)
is the best field fit. The program naturally touches autonomous agents,
multi-agent systems, game theory, negotiation, norms, institutions, social
simulation, agent engineering, and generative/agentic AI.

For AAMAS, the paper should emphasize:

- multi-agent social dilemma generation
- norms, commitments, sanctions, trust, and reputation
- mixed-motive behavior under partial knowledge
- process-aware evaluation of LLM agents
- comparison of cognitive and strategic representations

AAMAS is likely the most realistic primary venue if the experiments are strong.

### NeurIPS Evaluations And Datasets

[NeurIPS Evaluations & Datasets](https://blog.neurips.cc/2026/03/23/introducing-the-evaluations-datasets-track-at-neurips-2026/)
is a higher-prestige benchmark and evaluation target. It is appropriate only if
the artifact is strong: documented scenarios, reproducible runners, metrics,
baselines, model comparisons, leakage checks, and public release quality.

For this venue, the paper should emphasize:

- evaluation methodology
- benchmark generator rather than fixed task list
- reproducibility and artifact quality
- model ranking instability under different social-cognitive conditions
- limitations of outcome-only evaluation

The risk is that the work may be judged as an environment without sufficiently
general machine-learning insight. The empirical findings must be strong.

### AAAI

[AAAI](https://aaai.org/conference/aaai/aaai-26/main-technical-track-call/)
is a possible broad AI target if the results speak beyond games and benchmarks.
The paper would need to emphasize general agent evaluation, strategic social
reasoning, and formal scenario structure more than engine design.

AAAI is not the most natural first target, but it is reasonable if submission
timing and results align.

## Risks

### Risk: The Work Looks Like A Game Engine

If the paper reads as "we built an RPG engine," the research contribution is
weak. The artifact must be positioned as an evaluation substrate with clear
scientific questions and measurable outputs.

Mitigation:

- foreground scenario generation, representation interventions, metrics, and
  findings
- keep game content minimal and diagnostic
- include baselines and ablations

### Risk: The Cognitive Architecture Is Not Novel

Belief/desire/intention separation, appraisal theory, social simulation, and
agent-based modeling are established areas. The paper should not claim novelty
for those ideas alone.

Mitigation:

- treat cognitive structures as experimental variables
- cite prior cognitive and social simulation work
- make novelty about controlled generation and evaluation, not inventing
  belief, appraisal, or intent

### Risk: Too Much Generality

If the system supports arbitrary pass graphs, arbitrary representations, and
arbitrary LLM proposals, reviewers may see it as underconstrained.

Mitigation:

- keep authority boundaries fixed
- require typed representations, access policies, provenance, and metrics
- define a small set of standard evaluation profiles
- distinguish experiment extension points from trusted mutation paths

### Risk: Hidden Truth Leakage

Structured context may accidentally reveal information the actor should not
know. This would invalidate partial-information experiments.

Mitigation:

- add explicit actor access policies
- mark oracle conditions separately
- include leakage checks in every scenario
- log provenance for every context item shown to an agent

### Risk: LLM Judge Dependence

If deception, promise keeping, or norm violation is measured only by LLM judges,
the benchmark becomes fragile.

Mitigation:

- ground metrics in typed speech acts, commitments, event records, social
  updates, and epistemic updates
- use LLM judges only for secondary analysis or natural-language classification
  where typed traces are unavailable

### Risk: Scenario Overfitting

If the suite contains only a few hand-authored cases, it will not support the
claim of generated social dilemma evaluation.

Mitigation:

- use scenario families with controllable variables
- split scenario seeds and parameter combinations into development and held-out
  evaluation sets
- report sensitivity to scenario variation

### Risk: Strong Models Already Handle The Task

It is possible that frontier models perform well under many conditions. This
would not invalidate the research, but it would weaken a failure-focused story.

Mitigation:

- frame the question as representation sensitivity, not only model failure
- compare model classes and scaffolds
- include oracle and ablation conditions to reveal where structure matters or
  does not matter

## Success Criteria

The research program is worth pursuing if the first serious evaluation can
demonstrate at least some of the following:

- structured social-cognitive context changes model behavior in measurable ways
- typed speech acts improve traceability or strategic consistency over raw text
- limited theory-of-mind views improve opponent modeling or sanction awareness
- explicit commitments improve promise tracking or reveal breach patterns
- long-horizon social consequences change short-term deception or cooperation
  behavior
- different models fail in different process stages, not only in final reward
- ablations identify which representations are useful, redundant, harmful, or
  mostly cosmetic

The strongest outcome would be a clear empirical finding such as:

```text
Outcome scores alone overestimate agent competence. When belief, speech,
commitment, and delayed social consequence traces are evaluated separately,
frontier LLM agents show systematic failures in commitment consistency and
sanction-aware deception, and these failures change under typed social-cognitive
representations.
```

The weaker but still useful outcome would be:

```text
Some structured representations provide little benefit over direct LLM policy,
suggesting that current models already infer those structures from natural
language or that the proposed representation is not operationally useful.
```

That result would still be scientifically meaningful if the experiment is
controlled and reproducible.

## Open Questions

- What is the minimum scenario grammar needed to support meaningful mixed-motive
  social dilemmas?
- Which social-cognitive representations should be standard profiles, and which
  should remain experimental extensions?
- How should typed speech acts be obtained: agent self-labeling, parser,
  rule-based grounding, LLM classification, or gold annotation?
- What is the right limited theory-of-mind representation without introducing
  hidden truth leakage or unbounded nested beliefs?
- Which metrics can be computed entirely from engine traces, and which require
  external annotation?
- How much multi-resolution simulation is needed for delayed social
  consequences in the first paper?
- What baselines are fair: direct LLM, rule-based agents, structured LLM,
  oracle-context LLM, or game-theoretic scripted policies?
- How should scenario generation avoid becoming an opaque content generator?
- Which claims belong in the first paper, and which should remain future work?

