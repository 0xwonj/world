# Social-Strategic Benchmark Methodology

## Status

Nonnormative benchmark-methodology research. The production lifecycle,
research-artifact, trace, and experiment boundaries are owned by the
[Target Architecture Package](../architecture/target-architecture/README.md).

## Purpose

This document defines the target evaluation methodology for `world` as a
social-strategic LLM-agent benchmark substrate.

It is not an implementation plan and not a final benchmark specification. It
sets the methodological shape that later benchmark suites, paper experiments,
and open-source evaluation runners should follow.

The central evaluation question is:

```text
When language agents act in partially observable, mixed-motive, socially
consequential simulations, which explicit cognitive and social representations
change their behavior, and how?
```

The benchmark should therefore evaluate:

- agents
- scenario families
- decision profiles
- representation ablations
- process traces
- outcome and social-strategic metrics
- leakage, oracle, and judge-dependence risks

It should not collapse the work into a single leaderboard score.

Related local documents:

- [Social-Strategic Evaluation Program](social-strategic-evaluation-program.md)
- [Cognitive And Agent Research Map](cognitive-agent-research-map.md)
- [Cognition And Agency](../architecture/target-architecture/cognition-and-agency.md)
- [Extensibility And Research](../architecture/target-architecture/extensibility-and-research.md)

## Reference Lessons

### HELM And Holistic Evaluation

Reference:

- [Holistic Evaluation of Language Models](https://crfm.stanford.edu/helm/index.html)

Useful lesson:

HELM's core methodological lesson is that a model should be evaluated across
scenarios, adaptations, metrics, and transparency artifacts, not through one
aggregate number. It also emphasizes reproducibility through released prompts,
generations, and results.

For `world`:

- keep scenario, agent, decision profile, and metric definitions separate
- report outcome, process, safety, and validity metrics separately
- release raw traces or trace summaries when possible
- avoid ranking agents only by average win rate or payoff

Risk:

Too many metrics can make the benchmark unfocused. `world` should define a
small primary metric set per scenario family and keep additional diagnostics
secondary.

### Dynabench And Dynamic Benchmarking

Reference:

- [Dynabench: Rethinking Benchmarking in NLP](https://arxiv.org/abs/2104.14337)

Useful lesson:

Static benchmarks saturate and can miss simple challenge examples. Dynamic
benchmarking uses model failures to create harder future evaluations.

For `world`:

- keep a static held-out set for paper comparability
- also support generated or adversarial scenario variants
- record which scenario parameters were generated after observing model
  failures
- separate development, held-out, and adversarial splits

Risk:

If scenario generation is too unconstrained, the benchmark becomes a moving
target and results become hard to compare. Dynamic generation should augment,
not replace, frozen benchmark releases.

### Datasheets And Model Cards

References:

- [Datasheets for Datasets](https://www.microsoft.com/en-us/research/publication/datasheets-for-datasets/)
- [Model Cards for Model Reporting](https://arxiv.org/abs/1810.03993)

Useful lesson:

Benchmark artifacts need documentation: motivation, composition, collection,
recommended uses, limitations, distribution, maintenance, and ethical risks.

For `world`:

- every benchmark release should include a benchmark card
- every scenario family should include a scenario card
- every reported agent/model setup should include a model/profile card
- oracle and leakage conditions must be documented separately from normal
  actor-facing runs

Risk:

Documentation can become ceremonial. The cards should answer concrete
reproducibility and validity questions, not just repeat marketing claims.

### AgentBench And WebArena

References:

- [AgentBench: Evaluating LLMs as Agents](https://arxiv.org/abs/2308.03688)
- [WebArena: A Realistic Web Environment for Building Autonomous Agents](https://arxiv.org/abs/2307.13854)

Useful lesson:

Agent benchmarks need interactive environments, action validation, long-horizon
task execution, resettable state, and reproducible runs. WebArena is especially
useful as a reminder that realistic environments can reveal large gaps between
LLM agents and human performance.

For `world`:

- define resettable scenario instances
- validate every action against engine state
- record failed actions and invalid-action feedback
- keep task success separate from trace/process quality
- make the agent interface stable enough for repeatable comparisons

Risk:

Task-completion benchmarks often underrepresent social sanctions, reputation,
belief, commitment, and norm violation. `world` should keep those as first-
class metrics.

### SOTOPIA

Reference:

- [SOTOPIA: Interactive Evaluation for Social Intelligence in Language Agents](https://arxiv.org/abs/2310.11667)

Useful lesson:

Social intelligence evaluation needs multi-turn interaction, role-specific
goals, social context, and multi-dimensional scoring.

For `world`:

- define actor-specific goals and knowledge
- make social success distinct from raw task success
- use role and relationship variables
- ground scoring in typed traces before using LLM judges

Risk:

Free-form role-play can be difficult to diagnose. `world` should expose typed
scenario variables and actor-relative state so failures can be attributed to
belief, speech, commitment, social context, or action selection.

### Melting Pot

Reference:

- [Scalable Evaluation of Multi-Agent Reinforcement Learning with Melting Pot](https://arxiv.org/abs/2107.06857)

Useful lesson:

Multi-agent evaluation should test generalization across social partners and
social situations, not only performance in one fixed environment.

For `world`:

- include partner-policy variation
- test scripted, LLM, mixed, and adversarial partners
- evaluate social generalization under held-out actors and scenario parameters
- separate coordination failure from exploitation failure

Risk:

Many MARL environments abstract away language, institutions, and durable social
meaning. `world` should complement this literature by making typed speech,
commitment, and social consequence inspectable.

### CICERO

Reference:

- [Human-level play in the game of Diplomacy by combining language models with strategic reasoning](https://www.science.org/doi/10.1126/science.ade9097)

Useful lesson:

CICERO separates language interaction from strategic reasoning in a game that
requires cooperation, competition, negotiation, and tactical coordination.

For `world`:

- separate public speech, private belief, private intent, and committed action
- evaluate whether speech is strategically aligned with future actions
- treat negotiation and coalition formation as traceable processes

Risk:

CICERO is a specialized fixed-game system. `world` should not compete by
claiming stronger Diplomacy play. The contribution should be generated
social-strategic scenarios and representation ablations across many dilemma
families.

### Social Deduction And Bargaining Benchmarks

References:

- [AvalonBench](https://arxiv.org/abs/2310.05036)
- [MafiaBench](https://www.mafiabench.org/)
- [Cattle Trade](https://arxiv.org/abs/2605.14537)
- [Cooperation, Competition, and Maliciousness](https://proceedings.neurips.cc/paper_files/paper/2024/hash/984dd3db213db2d1454a163b65b84d08-Abstract-Datasets_and_Benchmarks_Track.html)

Useful lesson:

Social deduction and bargaining benchmarks expose hidden information,
deception, bluffing, accusation, voting, resource pressure, and negotiation.
Recent work also emphasizes behavioral trace analysis beyond final win rate.

For `world`:

- include hidden-role, hidden-preference, and hidden-resource scenario families
- track every offer, claim, vote, promise, breach, concession, and accusation
- evaluate phase adaptation and resource discipline
- distinguish bluff, lie, false belief, and uncertainty

Risk:

Some of these benchmarks are recent or narrow. They should be treated as
pressure sources, not as proof that the field already accepts one methodology.

### MACHIAVELLI

Reference:

- [MACHIAVELLI: Evaluating Agents' Ethical and Power-Seeking Behavior](https://arxiv.org/abs/2304.03279)

Useful lesson:

Agent evaluation should not reward goal completion while ignoring harmful
side effects, deception, coercion, power-seeking, or norm violation.

For `world`:

- report social harm, norm violation, coercion, reputation cost, and sanction
  exposure separately from payoff
- include dilemmas where short-term gain creates delayed social cost
- avoid a single scalar reward that hides social damage

Risk:

Ethical labels can be judge-dependent and culturally brittle. Engine-native
norms, commitments, and event records should carry as much of the measurement
as possible.

### LLM-As-Judge Methods

References:

- [G-Eval](https://arxiv.org/abs/2303.16634)
- [Judging LLM-as-a-Judge with MT-Bench and Chatbot Arena](https://arxiv.org/abs/2306.05685)
- [Chatbot Arena](https://arxiv.org/abs/2403.04132)

Useful lesson:

LLM judges are scalable and often useful for open-ended outputs, but they need
calibration against human labels, prompt sensitivity checks, and bias controls.

For `world`:

- use LLM judges for secondary classification or summarization
- prefer engine-native metrics where typed traces exist
- validate judge prompts on labeled examples
- randomize order for pairwise comparisons
- report judge model and prompt version
- report sensitivity to judge choice when a metric depends on a judge

Risk:

If promise keeping, deception, norm violation, or social intelligence depends
primarily on an LLM judge, the benchmark becomes fragile. Typed traces are the
main differentiator and should carry the primary metrics.

## Evaluation Thesis

`world` should be evaluated as a benchmark generator plus traceable simulator,
not as a fixed game.

Weak benchmark shape:

```text
Give several LLMs the same RPG scenario and compare who wins.
```

Stronger benchmark shape:

```text
Generate controlled social-strategic scenario instances from typed variables,
run agents under paired decision profiles, and measure outcome, process,
social, epistemic, and validity metrics from actor-relative traces.
```

The benchmark should answer questions such as:

- Does typed speech improve promise tracking over raw transcript context?
- Does explicit intent improve long-horizon consistency?
- Does bounded theory-of-mind improve deception detection or negotiation?
- Does social context help because it contains more information, or because the
  representation is structured?
- Do LLM agents fail because they lack strategic reasoning, because they miss
  social context, or because the environment interface is underspecified?
- Which structured representations are useful, redundant, harmful, or merely
  expensive?

## Benchmark Unit

A benchmark run should be decomposed into explicit units.

```text
BenchmarkSuite
  ScenarioFamily
    ScenarioTemplate
      ScenarioInstance
        ActorSpec
        InformationPartition
        SocialStateSpec
        NormSpec
        ActionSpeechSpace
        SuccessFailureSpec
        MetricSpec
  AgentPolicy
  DecisionProfile
  RunConfig
  RunTrace
  MetricReport
```

### Benchmark Suite

A `BenchmarkSuite` is a released set of scenario families, splits, metrics,
profiles, and runner rules.

It should declare:

- version
- intended use
- known limitations
- scenario families
- splits
- default profiles
- default agent policies
- metrics
- judge dependencies, if any
- release artifacts

### Scenario Family

A `ScenarioFamily` is a repeatable dilemma pattern with controllable variables.

Examples:

- hidden-preference bargaining
- promise under temptation
- witness and sanction
- rumor cascade
- false accusation
- blackmail or secret disclosure
- faction duty conflict
- reputation market
- trust repair after breach
- common resource depletion
- private warning versus public claim

Each family should declare:

- social-strategic mechanism
- required state variables
- controllable parameters
- actor roles
- information asymmetry
- available speech/action space
- primary metrics
- failure modes it is meant to expose

### Scenario Instance

A `ScenarioInstance` is a concrete seed and parameterization of a family.

It should include:

- content version
- random seed
- initial hard state
- initial social state
- initial actor truth
- role assignments
- hidden variables
- observation policy
- time horizon
- allowed actions
- allowed speech acts
- success/failure criteria
- expected oracle trace, when available

### Agent Policy

An `AgentPolicy` is the actual controller used for an actor.

Examples:

- direct LLM policy
- ReAct-style LLM policy
- structured LLM policy using decision-profile outputs
- rule/scripted policy
- heuristic social policy
- game-theoretic policy for small games
- human policy, if later used
- oracle-assisted policy

The policy must not be confused with the decision profile. The same agent
policy can run under different profile inputs.

### Decision Profile

A `DecisionProfile` defines which representations and passes are available.

Examples:

- direct action
- intent only
- structured context
- typed speech
- appraisal-like signals
- explicit other-model
- oracle other-model
- no-social-context control

The benchmark should compare paired profiles where possible.

### Run Trace

A `RunTrace` is the primary evidence artifact.

It should include:

- scenario instance id
- agent policy id
- decision profile id
- model id and version
- prompt/profile version
- random seed and sampling config
- actor-facing observations
- context items exposed to each pass
- pass outputs and verifier results
- speech acts and raw utterances
- selected intents, activities, and action requests
- invalid actions and feedback
- committed event records
- accepted social, epistemic, appraisal, and chronology updates
- metric inputs
- judge calls, if any

The trace must be actor-relative. It should support leakage checks.

## Scenario Splits

The benchmark should use multiple splits rather than one test set.

### Smoke Split

Purpose:

- verify runner health
- catch broken prompts, invalid actions, and obvious metric bugs

Properties:

- tiny
- public
- deterministic
- cheap to run

### Development Split

Purpose:

- design profiles
- tune prompts
- debug scenario mechanics

Properties:

- public or semi-public
- not used for final claims
- may include explanatory oracle traces

### Calibration Split

Purpose:

- validate metric behavior and LLM-judge prompts
- compare typed metrics with human or gold labels

Properties:

- small but carefully labeled
- includes edge cases
- includes ambiguous cases

### Held-Out Evaluation Split

Purpose:

- paper results
- leaderboard-style comparisons

Properties:

- fixed release
- no prompt tuning against it
- scenario parameter distribution documented
- enough seeds for paired statistical comparison

### Generalization Split

Purpose:

- test transfer across social variables, partners, and scenario families

Properties:

- held-out roles, norms, institutions, or payoff structures
- partner-policy variation
- may include unseen combinations of known variables

### Adversarial Split

Purpose:

- expose failure modes after models improve
- prevent benchmark saturation

Properties:

- generated or curated from observed failures
- reported separately from the frozen held-out split
- versioned over time

## Experimental Design

### Paired Runs

The default design should be paired:

```text
same scenario seed
same actor role
same model
same partner policy
different decision profile
```

This isolates the effect of a representation or pass more cleanly than
comparing unrelated runs.

Examples:

```text
direct action vs intent only
raw speech vs typed speech
no social context vs social context
no other-model vs bounded other-model
bounded other-model vs oracle other-model
rule speech grounding vs LLM speech grounding
```

### Factorial Profile Design

Not every combination should be run. The initial benchmark should use a small
factorial design over the most important axes:

```text
context:
  observation_only
  epistemic_context
  epistemic_plus_social_context

speech:
  raw
  typed

intent:
  direct_action
  explicit_intent

other_model:
  none
  bounded
  oracle
```

The full Cartesian product may be too expensive. Start with paired contrasts
that answer concrete research questions.

### Partner Policy Variation

Social-strategic competence depends on other actors.

Partner policies should include:

- scripted cooperative
- scripted adversarial
- scripted opportunistic
- rule-based rational baseline
- weak LLM
- strong LLM
- mixed population
- mirror self-play

Report whether results hold across partner types. A model that performs only
against itself may not be socially robust.

### Oracle Conditions

Oracle conditions are useful, but dangerous.

They should be used for:

- upper bounds
- sanity checks
- distinguishing model limitation from interface limitation
- verifying that a representation could help if correctly inferred

They must be labeled separately:

```text
normal actor-facing condition:
  actor receives only accessible context.

oracle condition:
  actor receives controlled privileged signal for diagnostic purposes.
```

Oracle results should not be mixed into normal benchmark rankings.

## Metrics

Metrics should be grouped, not collapsed.

### Outcome Metrics

Outcome metrics answer whether the agent achieved the task.

Examples:

- payoff
- win/loss
- goal completion
- agreement reached
- resource retained
- survival
- action validity rate
- time or turn efficiency

Outcome metrics are necessary but insufficient.

### Process Metrics

Process metrics answer how the outcome happened.

Examples:

- belief calibration
- belief update after evidence
- false-belief-sensitive action
- source reliability use
- promise creation
- promise fulfillment
- promise breach
- repair attempt after breach
- deception attempt
- deception success
- deception detection
- accusation accuracy
- speech-act consistency
- intent/action consistency
- activity persistence
- sanction anticipation
- norm compliance
- norm violation
- reputation cost
- coalition stability
- bargaining efficiency
- concession pattern
- resource discipline
- phase adaptation
- social regret

These should come from typed traces where possible.

### Validity Metrics

Validity metrics answer whether the run is trustworthy.

Examples:

- hidden-truth leakage
- oracle contamination
- invalid action rate
- unsupported belief creation
- unsupported social update
- untyped state laundering
- contradictory memory persistence
- prompt/version mismatch
- judge-dependence rate
- profile comparability failure

Validity metrics should be reported before interpreting success metrics.

### Trace Support Metrics

A trace support metric asks:

```text
Was the selected action supported by the actor-facing evidence and intermediate
representations recorded before the action?
```

This can be computed in several ways:

- rule-based support check for simple scenarios
- typed evidence reference count
- missing-critical-evidence flag
- contradiction flag
- LLM-assisted secondary audit
- human audit for calibration examples

Trace support is not the same as faithful chain-of-thought. It should depend
on engine-visible artifacts, not private model reasoning.

## LLM Judges

LLM judges should be secondary tools.

Acceptable uses:

- classify ambiguous natural-language speech when no typed label exists
- summarize a long trace for human inspection
- assist in adjudicating social tone or pragmatic ambiguity
- compare two explanations when primary metrics are already computed

Unacceptable primary uses:

- decide whether a promise was kept when typed commitment state exists
- decide whether an actor knew something when epistemic records exist
- decide whether a hard action succeeded when event records exist
- decide the main leaderboard score without calibration

Required controls:

- fixed judge model and version
- fixed judge prompt version
- order randomization for pairwise judgments
- calibration against labeled examples
- sensitivity report across at least one alternate judge or prompt where
  feasible
- explicit flag when a reported metric is judge-dependent

## Baselines

The benchmark should include several baseline families.

### Random Or Minimal Baseline

Purpose:

- catch broken metrics
- establish task difficulty floor

Risk:

- not meaningful as a social-strategic baseline beyond sanity checking

### Scripted Baseline

Purpose:

- test scenario mechanics
- establish known behavior
- provide reproducible partner policies

Risk:

- may be brittle or exploitable

### Heuristic Strategic Baseline

Purpose:

- compare LLMs against simple but coherent strategies
- expose overbidding, overpromising, or weak resource discipline

Risk:

- hand-designed heuristics may encode scenario-specific knowledge

### Direct LLM Baseline

Purpose:

- measure unstructured agent performance
- establish whether the cognitive pipeline adds value

Risk:

- strong prompts can hide structure inside natural language context

### Structured LLM Baseline

Purpose:

- evaluate the value of typed context and profile outputs

Risk:

- may improve only because it receives more information, not because structure
  is better

### Oracle Baseline

Purpose:

- upper-bound representation value
- determine whether better inference could help

Risk:

- contaminates normal comparison if not isolated

### Game-Theoretic Or Solver Baseline

Purpose:

- provide reference behavior for small formal games
- estimate regret where equilibrium or dynamic-programming analysis is
  tractable

Risk:

- often unavailable for rich social scenarios
- may assume common knowledge or rationality that the scenario does not
  provide

## Scenario Family Requirements

Every scenario family should include a scenario card with these fields:

```text
ScenarioFamilyCard:
  name
  motivation
  primary social-strategic mechanism
  controlled variables
  actor roles
  hidden information
  public information
  social state required
  epistemic state required
  norm or institution required
  speech acts required
  action types required
  time horizon
  partner policies
  primary metrics
  validity checks
  known ambiguities
  unsafe capability concerns
  intended splits
```

The scenario card should make it possible to answer:

- What is the dilemma?
- What information is hidden from whom?
- What social consequence matters?
- What cognitive representation is being tested?
- What failure mode should appear if the representation is absent?
- Which metrics are primary?
- Which metrics are judge-dependent?

## Candidate Scenario Families

### Promise Under Temptation

Core variables:

- promise value
- temptation reward
- witness presence
- sanction strength
- relationship value
- probability of detection

Primary metrics:

- promise fulfillment
- breach rate
- repair attempt
- reputation cost
- short-term payoff versus long-term social regret

Representation tested:

- commitment lifecycle
- sanction expectation
- intent persistence

### Hidden-Preference Bargaining

Core variables:

- private valuation
- outside option
- deadline
- trust history
- bluff cost
- deal enforceability

Primary metrics:

- agreement reached
- welfare
- fairness
- concession pattern
- bluff rate
- opponent-model quality

Representation tested:

- bounded other-model
- bargaining leverage estimate
- typed offers and counteroffers

### Witness And Sanction

Core variables:

- number of witnesses
- authority of witness
- reliability of witness
- report delay
- sanction severity
- publicness of action

Primary metrics:

- violation rate
- sanction anticipation
- concealment attempt
- false-denial rate
- delayed reputation cost

Representation tested:

- social context
- norm view
- witness-aware planning

### False Accusation And Evidence Discovery

Core variables:

- initial suspicion
- evidence quality
- rumor spread
- actor incentives
- correction cost
- audience composition

Primary metrics:

- accusation accuracy
- belief update
- apology or repair
- rumor persistence
- sanction misfire

Representation tested:

- epistemic state
- source reliability
- typed speech acts

### Blackmail Or Secret Disclosure

Core variables:

- secret severity
- disclosure audience
- blackmail demand
- credibility of threat
- counter-threat options
- institutional protection

Primary metrics:

- compliance
- resistance
- strategic disclosure
- threat credibility
- social harm

Representation tested:

- private belief
- leverage estimate
- threat speech act
- norm and sanction context

### Faction Duty Conflict

Core variables:

- faction loyalty
- personal relationship
- legal obligation
- material reward
- betrayal cost
- observability

Primary metrics:

- duty compliance
- betrayal rate
- conflict acknowledgment
- repair or justification
- long-horizon faction cost

Representation tested:

- duty conflict
- value conflict
- social commitment

### Trust Repair After Breach

Core variables:

- breach severity
- apology quality
- compensation amount
- prior trust
- future dependency
- third-party witness

Primary metrics:

- repair attempt
- acceptance of repair
- future cooperation
- repeated breach
- trust recovery

Representation tested:

- memory
- commitment lifecycle
- social relationship state

### Common Resource Depletion

Core variables:

- resource renewability
- group size
- monitoring capacity
- individual reward
- collective loss
- punishment mechanism

Primary metrics:

- overuse rate
- enforcement
- cooperation stability
- retaliation
- long-term group outcome

Representation tested:

- social dilemma awareness
- norm compliance
- partner modeling

## Minimal First Benchmark Release

The first serious release should be small enough to execute, inspect, and
debug.

Recommended scope:

```text
scenario families:
  promise_under_temptation
  hidden_preference_bargaining
  witness_and_sanction

profiles:
  direct_action
  intent_only
  typed_speech
  explicit_other_model
  oracle_other_model

agent policies:
  scripted_baseline
  direct_llm_baseline
  structured_llm

partner policies:
  cooperative_scripted
  opportunistic_scripted
  adversarial_scripted
  same_model_llm

splits:
  smoke
  development
  calibration
  held_out
```

Primary claims should be limited:

- typed speech improves or fails to improve commitment/deception metrics
- explicit other-model improves or fails to improve bargaining/sanction metrics
- direct LLM policy exhibits specific process failures
- oracle context reveals whether better inference could plausibly help

Avoid claiming general human-like cognition from the first release.

## Reporting Template

A paper or benchmark report should include:

```text
1. Benchmark motivation
2. Scenario family definitions
3. Agent policies
4. Decision profiles
5. Splits and seeds
6. Metrics
7. LLM-judge use and calibration
8. Baselines
9. Main results
10. Ablation results
11. Process failure analysis
12. Leakage and validity checks
13. Cost and reproducibility
14. Limitations
15. Release artifacts
```

Every result table should indicate:

- scenario family
- split
- agent policy
- decision profile
- model version
- number of runs
- mean and uncertainty interval
- primary metric group
- whether any metric uses an LLM judge
- whether any condition is oracle-assisted

## Reproducibility Requirements

For each run, record:

- repository commit
- benchmark suite version
- scenario family and instance id
- seed
- model provider
- model id
- model version or date
- temperature and sampling settings
- system prompt and policy prompt version
- decision profile version
- content pack version
- partner policy ids
- wall-clock date
- token/cost metadata where available
- raw transcript
- typed trace
- metric report

For public release:

- release frozen benchmark config
- release runner instructions
- release metric code
- release scenario cards
- release model/profile cards
- release at least sampled traces
- release full traces if safety and licensing allow it

## Validity Risks

### Risk: Benchmark Measures Prompting Skill

If most performance comes from prompt engineering rather than agent competence,
results will be unstable.

Mitigation:

- freeze prompt versions
- report prompt changes
- compare direct and structured prompts
- avoid tuning on held-out splits

### Risk: Context Amount Confounds Structure

Structured profiles may perform better simply because they receive more
information.

Mitigation:

- include no-social-context controls
- include raw-context controls with equivalent information
- pair runs on the same scenario seed

### Risk: LLM Judge Dependence

Social intelligence is tempting to score with an LLM judge.

Mitigation:

- compute primary metrics from typed traces
- calibrate judge-dependent metrics
- label judge-dependent results

### Risk: Hidden-Truth Leakage

Actor-facing prompts or traces may accidentally include omniscient state.

Mitigation:

- maintain actor-relative trace views
- test profiles with leakage checks
- include adversarial hidden-information scenarios
- record every context item shown to the agent

### Risk: Oracle Contamination

Oracle signals can accidentally become part of normal profiles.

Mitigation:

- use separate profile ids
- mark oracle runs in reports
- exclude oracle runs from normal rankings

### Risk: Scenario Overfitting

Models or prompts may adapt to known families.

Mitigation:

- use held-out parameter combinations
- use generalization split
- version adversarial split separately
- report sensitivity to scenario variation

### Risk: Self-Play Artifact

Agents may perform well only against copies of themselves.

Mitigation:

- vary partner policies
- include scripted and adversarial partners
- report partner-specific results

### Risk: Unsafe Capability Framing

Deception and manipulation benchmarks can be misread as capability training.

Mitigation:

- frame deception metrics as diagnosis and safety analysis
- release scenarios with clear intended use
- avoid optimizing agents for deception as the primary goal
- include norm, sanction, and harm metrics alongside deception success

## What Counts As Success

The benchmark methodology is working if a first paper can show:

- reproducible scenario instances
- paired profile comparisons
- typed traces sufficient for primary metrics
- at least one meaningful ablation delta
- at least one negative or null result
- leakage checks
- oracle upper-bound analysis
- baseline agents that catch broken metrics
- process failure examples that are not visible from outcome score alone

The most valuable result is not necessarily "structured cognition wins". A
credible result might be:

```text
Typed speech improves commitment traceability but does not improve payoff.
Explicit other-modeling helps only in bargaining with high information
asymmetry. Appraisal-like signals are redundant for strong models but help
weaker models avoid immediate norm violations. Direct LLM policies can win
short-term rewards while producing worse long-horizon social regret.
```

That kind of result would make `world` research-relevant because it reveals how
agent behavior changes under controlled social-cognitive interventions.
