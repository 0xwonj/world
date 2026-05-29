# Cognitive And Agent Research Map

## Purpose

This document maps relevant AI, LLM-agent, multi-agent, and cognitive-science
research areas into design pressure for `world`.

It is intentionally not a complete literature review. The goal is to identify
which research traditions should shape the engine's social-cognitive
representations, decision profiles, ablations, and metrics.

The main conclusion is:

```text
world should not claim to simulate the human mind.

world should provide an authority-bounded substrate where cognitive and
social-strategic representations can be introduced, removed, substituted, and
measured under controlled conditions.
```

The useful research frame is not "build a better NPC brain". It is:

```text
Which explicit social-cognitive structures help or hurt LLM agents in
partially observable, mixed-motive, socially consequential simulations?
```

Related local documents:

- [Social-Strategic Evaluation Program](social-strategic-evaluation-program.md)
- [Configurable Decision Pipeline](../architecture/configurable-decision-pipeline.md)
- [Epistemic State](../design/epistemic-state.md)
- [Semantic Appraisal And Motivation](../design/semantic-appraisal-and-motivation.md)
- [Intent Templates And Planning](../design/intent-templates-and-planning.md)
- [Social Institutional Model](../design/social-institutional-model.md)

## How To Use This Map

Each research area below should be treated as one of three things:

1. **Representation candidate**

   A typed artifact that may appear in a decision profile.

   Examples: `OtherModelView`, `SpeechAct`, `SanctionExpectation`,
   `DutyConflict`, `CommitmentCandidate`.

2. **Pass candidate**

   A transform from one representation to another.

   Examples: speech grounding, belief update, appraisal, opponent modeling,
   intent selection, reflection summarization.

3. **Metric candidate**

   A measurable behavior or failure mode.

   Examples: promise consistency, deception detection, belief calibration,
   sanction awareness, social regret.

Research traditions should not be imported as monolithic subsystems. BDI,
appraisal theory, theory of mind, POMDPs, cognitive architectures, LLM
reflection loops, and social simulation frameworks are all useful references,
but none should become the engine's single theory of cognition.

## High-Level Research Landscape

| Area | Use in `world` | Main risk |
| --- | --- | --- |
| LLM generative agents | Memory, reflection, planning, social emergence. | Believability without causal or authority grounding. |
| LLM agent loops | Observation-action protocols, tool use, reflection, skill libraries. | Free-form traces become untyped hidden state. |
| Social-strategic benchmarks | Negotiation, deception, coalition, social deduction, mixed motives. | Fixed games and outcome-only scores hide causal failure modes. |
| Theory of mind | Bounded opponent models and false-belief evaluation. | Nested belief explosion and overclaiming "real ToM". |
| BDI and intention theory | Intent as commitment, not just next action. | Full BDI interpreter would overstructure the engine. |
| Cognitive appraisal | Event-to-meaning and pressure generation. | Universal emotion taxonomies become brittle primitives. |
| POMDP / epistemic reasoning | Partial observability, belief state, uncertainty. | Exact planning is intractable and too formal for content authors. |
| Speech acts / commitments | Promises, threats, accusations, agreements, obligations. | Typed labels ignore uptake, witness, ambiguity, and jurisdiction. |
| Normative MAS | Norms, permissions, sanctions, duties, institutional state. | Deontic formalism overwhelms concrete gameplay semantics. |
| Cognitive architectures | Modular memory, procedural skill, working memory, action selection. | Copying a whole mind architecture bloats the project. |

The project should take the representation and evaluation pressure, not the
whole architecture.

## LLM Agent And Social Simulation Work

### Generative Agents

Reference:

- [Generative Agents: Interactive Simulacra of Human Behavior](https://arxiv.org/abs/2304.03442)

Why it matters:

Generative Agents is a key reference for LLM-driven believable agents. Its
architecture combines observation, memory, reflection, and planning in a small
simulated town. It is highly relevant because it shows that LLM agents can
produce convincing social behavior when given memory and reflection loops.

How `world` should use it:

- treat observation, memory, reflection, and planning as separable pass
  candidates
- make reflection optional and comparable against no-reflection baselines
- avoid storing reflection as authoritative truth unless it passes an explicit
  actor-truth or appraisal gate
- distinguish believable behavior from causally valid behavior

Risk:

Believability is not the same as research-grade social-strategic evaluation.
Free-form memory and reflection can launder hallucinated beliefs into later
actions. `world` should keep memory actor-relative, sourced, and typed when it
affects gameplay or metrics.

### Concordia And Generative Agent-Based Modeling

References:

- [Generative agent-based modeling with actions grounded in physical, social, or digital space using Concordia](https://arxiv.org/abs/2312.03664)
- [Google DeepMind Concordia repository](https://github.com/google-deepmind/concordia)

Why it matters:

Concordia is close in spirit to LLM-based social simulation. It supports
generative agent-based modeling with grounded actions and social environments.
It is one of the strongest neighboring projects for any claim around LLM social
simulation infrastructure.

How `world` should use it:

- treat it as the main comparison point for open-source generative social
  simulation
- differentiate through typed authority boundaries, causal transactions,
  actor-relative visibility, configurable cognitive representations, and
  process-level metrics
- avoid framing `world` as merely "another LLM social simulator"

Risk:

If `world` only offers agent prompts, memory, and simulation scenes, Concordia
is already closer to that space. The stronger position is a controlled
evaluation substrate for social-strategic cognition, not a general LLM society
playground.

### CoALA And Cognitive Language Agents

Reference:

- [Cognitive Architectures for Language Agents](https://arxiv.org/abs/2309.02427)

Why it matters:

CoALA systematizes language agents using modular memory, structured actions,
internal reasoning, grounding, learning, and decision loops. It is useful
because it gives modern LLM-agent vocabulary without requiring old symbolic
cognitive architectures.

How `world` should use it:

- use modular memory/action/decision categories as a sanity check
- keep agent-facing action space structured and typed
- separate internal memory operations from world-facing actions
- evaluate whether a module is absent, heuristic, LLM-generated, hybrid, or
  oracle-provided

Risk:

CoALA is a conceptual framework for agents, not a simulation truth model.
`world` should not let an agent architecture define the authority boundaries of
the world.

### ReAct, Reflexion, Voyager, And Agent Loops

References:

- [ReAct: Synergizing Reasoning and Acting in Language Models](https://arxiv.org/abs/2210.03629)
- [Reflexion: Language Agents with Verbal Reinforcement Learning](https://arxiv.org/abs/2303.11366)
- [Voyager: An Open-Ended Embodied Agent with Large Language Models](https://arxiv.org/abs/2305.16291)

Why they matter:

These works define much of the modern LLM-agent loop: observe, reason, act,
receive feedback, reflect, and accumulate reusable skills. They are relevant to
how an LLM-controlled actor should interact with a world.

How `world` should use them:

- define `AgentTurnInput` and `AgentTurnOutput` as typed interfaces
- keep reasoning traces non-authoritative unless explicitly imported through a
  gate
- treat reflection as an optional pass that can improve or harm future behavior
- treat skills/procedures as actor knowledge or procedural capability, not
  hard world truth
- evaluate whether free-form reasoning improves action validity, strategy, and
  long-horizon consistency

Risk:

Reasoning traces are often unfaithful. A system that logs "thoughts" may look
explainable while the trace is post-hoc or unstable. `world` should prefer
engine-grounded trace artifacts over relying on chain-of-thought.

### Agent Frameworks And Orchestration

Reference:

- [AutoGen: Enabling Next-Gen LLM Applications via Multi-Agent Conversation](https://arxiv.org/abs/2308.08155)

Why it matters:

Agent frameworks show the engineering value of multiple specialized agents,
conversation patterns, tools, and orchestration. They are relevant for building
evaluators, scenario generators, judges, and content authoring helpers around
the engine.

How `world` should use it:

- use multi-agent orchestration for tooling and evaluation support
- avoid making the runtime itself a loose conversation among agent roles
- require typed boundaries where an agent proposal affects simulation state

Risk:

Conversation-driven orchestration is flexible but weak as a truth boundary.
For `world`, agents can propose, classify, summarize, or evaluate; commit
authority remains in the engine.

## Social-Strategic Evaluation And Benchmarks

### CICERO And Diplomacy

Reference:

- [Human-level play in the game of Diplomacy by combining language models with strategic reasoning](https://www.science.org/doi/10.1126/science.ade9097)

Why it matters:

CICERO is a major reference for combining language negotiation with strategic
planning in a mixed cooperative-competitive game. It shows the value of
separating dialogue generation from strategic reasoning.

How `world` should use it:

- separate public speech, private belief, private intent, and committed action
- evaluate whether language acts align with planned strategy
- include negotiation and coalition scenarios
- distinguish tactical action validity from social-strategic coherence

Risk:

CICERO is specialized for Diplomacy. `world` should not claim to outperform
specialized game agents. The relevant gap is general generation of typed
social-strategic dilemmas and ablation of cognitive context.

### SOTOPIA

Reference:

- [SOTOPIA: Interactive Evaluation for Social Intelligence in Language Agents](https://arxiv.org/abs/2310.11667)

Why it matters:

SOTOPIA evaluates social intelligence through role-play scenarios, social
goals, and multi-turn interaction. It is a strong reference for social
evaluation beyond task completion.

How `world` should use it:

- borrow the idea of diverse social tasks and role-specific goals
- ground social tasks in typed engine state instead of free-form prompt text
- use LLM judges only as secondary analysis where possible

Risk:

Free-form social interactions are hard to diagnose causally. `world` should
make scenario variables, actor-relative knowledge, commitments, and outcomes
inspectable.

### Social Deduction Benchmarks

References:

- [AvalonBench: Evaluating LLMs Playing the Game of Avalon](https://avalonbench.github.io/)
- [MafiaBench](https://www.mafiabench.org/)

Why they matter:

Avalon, Mafia, Werewolf-like games are natural tests for hidden roles, lying,
accusation, coalition, public/private knowledge, and deduction.

How `world` should use them:

- include typed speech acts such as claim, accusation, denial, reveal, threat,
  promise, vote, and refusal
- separate deception from false belief
- track who had access to which evidence at each moment
- evaluate vote influence, suspicion shifts, and coalition stability

Risk:

Social deduction games can turn deception into a leaderboard capability without
enough safety framing. They also use narrow game conventions. `world` should
use them as scenario families, not as the entire research agenda.

### Bargaining, Bluffing, And Economic Pressure

References:

- [Cattle Trade: A Multi-Agent Benchmark for LLM Bluffing, Bidding, and Bargaining](https://arxiv.org/abs/2605.14537)
- [Cooperation, Competition, and Maliciousness: LLM-Stakeholders Interactive Negotiation](https://proceedings.neurips.cc/paper_files/paper/2024/hash/984dd3db213db2d1454a163b65b84d08-Abstract-Datasets_and_Benchmarks_Track.html)

Why they matter:

Bargaining and market games expose partial information, hidden value, bluffing,
resource discipline, phase-specific strategy, and opponent adaptation.

How `world` should use them:

- include hidden valuation, scarcity, deadline, debt, and repeated interaction
  variables
- measure resource discipline, phase adaptation, credible threats, concession
  patterns, and post-deal reputation
- compare raw LLM negotiation against typed intent/commitment/social-context
  conditions

Risk:

Economic games are excellent for strategic pressure but weak on wider
institutional and emotional meaning. They should be one scenario family, not
the whole definition of social strategy.

### MACHIAVELLI And Harmful Tradeoffs

Reference:

- [MACHIAVELLI: Evaluating Agents' Ethical and Power-Seeking Behavior](https://arxiv.org/abs/2304.03279)

Why it matters:

MACHIAVELLI evaluates reward pursuit, power-seeking, deception, and ethical
violations in text-game environments. It is relevant to measuring when agents
choose socially harmful or norm-violating paths.

How `world` should use it:

- include social harm, norm violation, coercion, reputation damage, and
  sanction exposure metrics
- distinguish short-term utility from long-term social cost
- avoid outcome-only reward where harmful strategies look competent

Risk:

Labeling harm or ethical violations is hard and can become judge-dependent.
`world` should ground as much as possible in typed norms, commitments,
authority relations, and event records.

### Melting Pot And Multi-Agent Reinforcement Learning

References:

- [Scalable Evaluation of Multi-Agent Reinforcement Learning with Melting Pot](https://proceedings.mlr.press/v139/leibo21a.html)
- [Hypothetical Minds: Scaffolding Theory of Mind for Multi-Agent Tasks with Large Language Models](https://arxiv.org/abs/2407.07086)

Why they matter:

Melting Pot is a major reference for evaluating agents in social dilemmas,
reciprocity, resource sharing, cooperation, and generalization to novel social
partners. Hypothetical Minds is relevant because it adds an explicit
theory-of-mind scaffold for LLM agents in mixed-motive multi-agent settings.

How `world` should use them:

- include social-generalization pressure, not only fixed scenario success
- test behavior against varied partner policies
- make explicit `OtherModelView` an ablation axis
- distinguish coordination failure from belief/modeling failure

Risk:

MARL environments often abstract away language, commitments, institutions, and
rich event history. `world` can complement them by focusing on typed social
meaning and causal traces.

## Theory Of Mind And Epistemic Reasoning

### Theory Of Mind Benchmarks For LLMs

References:

- [Evaluating Large Language Models in Theory of Mind Tasks](https://arxiv.org/abs/2302.02083)
- [OpenToM: A Comprehensive Benchmark for Evaluating Theory-of-Mind Reasoning Capabilities of Large Language Models](https://arxiv.org/abs/2402.06044)
- [MindGames: Targeting Theory of Mind in Large Language Models with Dynamic Epistemic Modal Logic](https://arxiv.org/abs/2305.03353)

Why they matter:

These benchmarks test false belief, mental-state attribution, higher-order
reasoning, and narrative understanding. They are directly relevant to whether
LLM agents can reason about what another actor knows, believes, wants, or
intends.

How `world` should use them:

- represent other-model state as bounded, actor-relative, and evidence-linked
- evaluate false-belief-sensitive actions, not only QA answers
- measure whether an agent exploits, repairs, or ignores belief gaps
- separate first-order belief from second-order belief

Risk:

Static QA success is not robust evidence of interactive theory of mind. The
engine should test ToM through action consequences in dynamic scenarios.

### Bounded Other-Model Views

The initial `OtherModelView` should be deliberately small:

```text
OtherModelView:
  subject_actor
  modeled_actor
  focus
  believed_observations
  likely_beliefs
  likely_goals
  likely_commitments
  likely_intents
  likely_next_actions
  evidence_refs
  confidence
  uncertainty
  depth
```

Depth should usually be capped at one:

```text
Alice believes Bob thinks the gem is in the shrine.
```

Depth two may be allowed only for specific scenarios:

```text
Alice believes Bob thinks Cara suspects Alice.
```

Arbitrary recursive belief nesting should not be a default engine feature.

### Dynamic Epistemic Logic And Information Actions

References:

- [Dynamic Epistemic Logic, Stanford Encyclopedia of Philosophy](https://plato.stanford.edu/entries/dynamic-epistemic/)
- [The Logic of Public Announcements, Common Knowledge, and Private Suspicions](https://scholarworks.iu.edu/dspace/items/091c4363-4700-4b17-8202-8e9daa850d04)

Why it matters:

Many social-strategic events are information actions: public announcement,
private disclosure, overheard conversation, rumor, lie, confession, secret
signal, or staged evidence.

How `world` should use it:

- model information-changing events explicitly
- distinguish public, private, overheard, rumored, and concealed information
- record who had access to which signal
- let false belief persist when an actor misses or distrusts a signal

Risk:

Full modal logic is too heavy. `world` should use operational records,
provenance, and actor-relative views rather than full epistemic closure.

### Belief Revision And Truth Maintenance

References:

- [Logic of Belief Revision, Stanford Encyclopedia of Philosophy](https://plato.stanford.edu/entries/logic-belief-revision/)
- [A Truth Maintenance System](https://www.sciencedirect.com/science/article/pii/0004370279900080)

Why it matters:

Agents need to handle stale belief, contradiction, unreliable testimony,
rumor, correction, and revealed deception.

How `world` should use it:

- store support/evidence links for belief-like artifacts
- record contradiction and supersession
- distinguish confidence, source reliability, freshness, and public acceptance
- make belief update a pass that can be heuristic, LLM, rule-based, or oracle

Risk:

Full logical closure or a TMS per actor is too expensive and too formal. The
engine needs lightweight contradiction/provenance machinery first.

## Cognitive Architecture And Decision Structure

### BDI And Intention As Commitment

References:

- [BDI Agents: From Theory to Practice](https://cdn.aaai.org/ICMAS/1995/ICMAS95-042.pdf)
- [Intention is Choice with Commitment](https://research.monash.edu/en/publications/intention-is-choice-with-commitment/)

Why it matters:

BDI is important because it separates belief, desire, and intention. `world`
already has a similar design pressure: belief/epistemic state, pressure/goals,
intent, activity, and action should not collapse into one direct policy call.

How `world` should use it:

- treat `Intent` as a commitment boundary
- distinguish wanting, planning, committing, attempting, and succeeding
- evaluate intent persistence and abandonment under pressure
- support ablations where intent is absent, shallow, or explicit

Risk:

A full BDI architecture would overfit the engine to one cognitive theory.
`world` should borrow the separation, not install a BDI interpreter.

### Cognitive Appraisal

References:

- [The Cognitive Structure of Emotions](https://www.cambridge.org/core/books/cognitive-structure-of-emotions/cognitive-structure-of-emotions/D16784CA50F5BBD58F3140B575BB7881)
- [EMA: A process model of appraisal dynamics](https://people.ict.usc.edu/~gratch/CSCI534/Readings/COGSYS-RS-EMOTION-2008-6.pdf)

Why it matters:

Appraisal theory explains how events become actor-relative meaning: harm,
benefit, blame, obligation, threat, opportunity, loss, control, and coping.
This aligns strongly with `world` because the same hard event can mean
different things to different actors.

How `world` should use it:

- keep appraisal-like outputs typed and actor-relative
- use appraisal as one standard dialect, not a kernel primitive
- make appraisal variables optional and substitutable
- evaluate whether appraisal-like structure improves long-horizon social
  coherence

Risk:

Emotion labels can become vague and culturally brittle. The useful artifacts
are often lower-level variables such as threat, blame, obligation, loss,
leverage, control, and urgency.

### Classical Cognitive Architectures

References:

- [Soar: An Architecture for General Intelligence](https://kilthub.cmu.edu/articles/journal_contribution/Soar_an_architecture_for_general_intelligence/6618113)
- [An Integrated Theory of the Mind](https://pubmed.ncbi.nlm.nih.gov/15482072/)

Why they matter:

Soar and ACT-R are important because they represent the long tradition of
modular cognition, working memory, procedural memory, production rules,
learning, and action selection.

How `world` should use them:

- use them as background pressure for modularity and traceability
- consider working memory, episodic memory, procedural knowledge, and action
  selection as separate representation families
- avoid claiming cognitive fidelity

Risk:

The project is not trying to be a cognitive architecture. Copying Soar or ACT-R
would shift the thesis away from simulation/evaluation and toward artificial
cognitive modeling.

## Speech, Commitments, Norms, And Institutions

### Speech Acts

References:

- [Speech Acts](https://www.cambridge.org/core/books/speech-acts/contents/B683091EC0F6D70DC173662C9E25C8EA)
- [Performatives in a Rationally Based Speech Act Theory](https://aclanthology.org/P90-1011/)

Why it matters:

In social-strategic settings, speech is action. Promises, accusations,
confessions, refusals, threats, lies, and commands can alter social state and
future incentives.

How `world` should use it:

- separate `SpeechSurface` from `SpeechAct`
- make speech grounding an ablation pass
- track speaker, audience, witness, uptake, ambiguity, and jurisdiction
- let typed speech acts propose social or epistemic updates through gates

Risk:

Speech-act labels can look precise while missing pragmatic ambiguity. A threat,
joke, warning, bluff, and promise may overlap. The engine should allow
uncertainty and multiple candidate interpretations.

### Social Commitments

References:

- [Social and Psychological Commitments in Multiagent Systems](https://www.dfki.de/web/forschung/projekte-publikationen/publikation/6257)
- [Clouseau: Generating Communication Protocols from Commitments](https://ojs.aaai.org/index.php/AAAI/article/view/6215)

Why it matters:

Commitments are central to negotiation, promises, contracts, duties, betrayal,
coordination, and trust repair. They are also a clean way to separate internal
intent from public social obligation.

How `world` should use it:

- distinguish psychological commitment from social commitment
- model debtor, creditor, condition, deadline, witness, fulfillment, breach,
  release, repair, and sanction
- make commitment lifecycle a core social-strategic metric

Risk:

Commitment protocols can become too formal. The engine should begin with
simple lifecycle records and concrete scenario variables.

### Normative Multi-Agent Systems

References:

- [Introduction to Normative Multiagent Systems](https://orbilu.uni.lu/handle/10993/25302)
- [Norm emergence in multiagent systems: a viewpoint paper](https://link.springer.com/article/10.1007/s10458-019-09422-0)
- [Detection and resolution of normative conflicts in multi-agent systems](https://research.ibm.com/publications/detection-and-resolution-of-normative-conflicts-in-multi-agent-systems-a-literature-survey)

Why it matters:

Norms and institutions define permission, prohibition, obligation, authority,
sanction, role, rank, law, taboo, and local custom. These are exactly the
social variables that make an action strategically meaningful beyond immediate
reward.

How `world` should use it:

- expose actor-visible `NormView`
- represent `DutyConflict`, `PermissionClaim`, `ViolationRisk`, and
  `SanctionExpectation`
- keep norm scope explicit: group, place, institution, role, time, witness
- evaluate whether agents anticipate sanction and reputation effects

Risk:

Normative formalisms can become abstract and disconnected from simulation. The
initial model should stay domain-shaped and scenario-driven.

## Representation Families To Consider

The following families should be candidates for configurable profiles.

### Actor-Relative Context

```text
ObservedEvent
ObservedState
EpistemicWorkingSet
SocialContextView
NormView
ActionRepertoire
PerceivedAffordance
```

Core question:

```text
What did this actor have access to when deciding?
```

### Belief And Memory

```text
BeliefRecord
RumorRecord
TestimonyRecord
ContradictionRecord
MemorySummary
WorkingMemoryItem
SourceReliabilityEstimate
```

Core question:

```text
What does the actor think is true, why, and how fresh is that support?
```

### Speech And Communication

```text
SpeechSurface
SpeechAct
Claim
Promise
Threat
Accusation
Confession
Request
Refusal
DeceptionAttempt
AmbiguousSpeechActSet
```

Core question:

```text
What did the utterance do socially or epistemically, if anything?
```

### Commitment And Social Obligation

```text
CommitmentCandidate
AcceptedCommitment
CommitmentFulfilled
CommitmentBreached
CommitmentReleased
RepairAttempt
Debt
Oath
Duty
```

Core question:

```text
What has been adopted, promised, owed, witnessed, breached, or repaired?
```

### Motivation And Appraisal-Like Signals

```text
AppraisalVariableSet
PressureVector
ThreatAssessment
OpportunityAssessment
LossAssessment
BlameAssessment
DutyConflict
ValueConflict
Urgency
ControlEstimate
CopingOption
```

Core question:

```text
What actor-relative meaning or pressure does the situation create?
```

### Theory Of Mind And Strategy

```text
OtherModelView
FalseBeliefHypothesis
LikelyGoalEstimate
LikelyIntentEstimate
LikelyPolicyEstimate
BargainingLeverageEstimate
SanctionExpectation
ReputationRisk
CoalitionAssessment
```

Core question:

```text
What does the actor believe others know, want, intend, and likely will do?
```

### Intent, Activity, And Action

```text
CandidateIntent
SelectedIntent
ActivityPlan
ProcessTarget
ActionRequest
NonHardUpdateProposal
Abstention
```

Core question:

```text
What is the actor committing to trying, and how does that become an executable
request?
```

## Initial Ablation Matrix

The first research profiles should avoid too many degrees of freedom. A useful
starting matrix:

| Profile | Context | Cognitive structure | Speech | Other-model | Implementation |
| --- | --- | --- | --- | --- | --- |
| Direct action | Observation + action repertoire | none | raw text | none | LLM policy |
| Intent only | Observation + action repertoire | candidate intent | raw text | none | LLM or rule selection |
| Structured context | Observation + epistemic + social context | candidate intent | raw text | none | LLM selection |
| Typed speech | Observation + epistemic + social context | candidate intent | typed speech acts | none | LLM/rule grounding |
| Appraisal-like | Observation + epistemic + social context | pressure/appraisal signal | typed speech optional | none | rule/LLM/hybrid |
| Explicit ToM | Observation + epistemic + social context | strategic assessment | typed speech optional | bounded `OtherModelView` | LLM/hybrid |
| Oracle ToM | Same as explicit ToM | strategic assessment | typed speech optional | oracle `OtherModelView` | oracle + LLM |
| No-social-context control | Observation + action repertoire | selected structure | selected speech mode | selected ToM mode | same as paired profile |

The important rule is that paired profiles should differ in only one intended
dimension whenever possible.

## Metrics Suggested By The Literature

Outcome metrics:

- reward or payoff
- goal completion
- action validity
- survival or resource retention
- agreement reached
- win/loss in game-like scenarios

Process metrics:

- belief calibration
- belief update after evidence
- false-belief-sensitive action
- source reliability use
- promise creation and fulfillment
- promise breach and repair
- deception attempt
- deception success
- deception detection
- speech-act consistency
- intent/action consistency
- activity persistence
- sanction anticipation
- norm compliance or violation
- reputation cost
- coalition stability
- bargaining efficiency
- resource discipline
- social regret
- trace support for selected action

Safety and validity metrics:

- hidden-truth leakage
- oracle contamination
- LLM judge dependence
- unsupported belief creation
- untyped state laundering
- contradictory memory persistence
- profile comparability failure

## Prioritization

The strongest near-term design priorities are:

1. **Actor-relative context**

   Without this, ToM, deception, belief, and social evaluation are not
   meaningful.

2. **Typed speech and commitment lifecycle**

   This is one of the clearest differentiators from generic LLM-agent
   environments. It makes promises, lies, threats, accusations, agreements,
   and breaches measurable.

3. **Bounded other-model view**

   This gives a concrete ToM ablation without pretending to solve full theory
   of mind.

4. **Appraisal-like motivational signals as optional dialect**

   Useful, but should not be a mandatory primitive. Start with variables like
   threat, opportunity, loss, blame, duty, urgency, and control rather than a
   large emotion taxonomy.

5. **Process-aware traces**

   The research contribution depends on explaining why an action happened,
   not only whether it won.

Lower priority for now:

- full BDI interpreter
- full POMDP/I-POMDP solver
- full dynamic epistemic logic
- full cognitive architecture
- large-scale society simulation before the core trace/ablation story works

## Objective Positioning

The positive case:

- The literature already shows interest in LLM agents, social simulation,
  negotiation, deception, ToM, reflection, and multi-agent benchmarks.
- Existing work often uses fixed games, free-form social prompts, outcome
  scores, or black-box agent transcripts.
- `world` can contribute by generating controlled social-strategic dilemmas
  with typed actor-relative state, configurable cognitive representations, and
  process-level traces.

The skeptical case:

- Many individual ingredients are established: BDI, appraisal, ToM benchmarks,
  social commitments, generative agents, Concordia-like simulations, and
  game/negotiation benchmarks.
- A broad "cognitive simulation engine" claim would be too vague and too easy
  to reject.
- The project only becomes research-interesting if it produces clean
  ablations and reveals failures or improvements that neighboring benchmarks
  do not expose.

The defensible claim:

```text
world is a simulation substrate for controlled social-strategic evaluation of
language agents, where cognitive and social representations are experimental
variables rather than fixed assumptions.
```

This is narrower than human cognition, broader than a single game benchmark,
and more defensible than claiming a universal agent architecture.

## Open Research Questions

- Which representation families provide the largest behavioral delta over
  direct LLM policy?
- Which structures help only because they expose more context, not because the
  representation itself is useful?
- When does typed speech improve promise/deception/negotiation metrics over
  raw transcript context?
- What is the smallest useful `OtherModelView`?
- Can LLM-generated appraisal-like signals improve behavior without becoming
  unfaithful rationalization?
- Which failures are model limitations, and which are environment/interface
  limitations?
- How much of social-strategic competence can be measured with engine-native
  traces rather than LLM judges?
- How should oracle conditions be reported so they reveal upper bounds without
  contaminating normal agent evaluation?

