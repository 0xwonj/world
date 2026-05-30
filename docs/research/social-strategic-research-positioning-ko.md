# 사회적-전략적 시뮬레이션 엔진으로서의 world

## 관점

`world`는 부분 지식, 사회적 의미, 약속, 규범, 평판, 제재, 의도, 행동,
장기 결과가 함께 움직이는 RPG 세계를 시뮬레이션한다. 인간 플레이어와 LLM
에이전트는 같은 세계 안에서 관찰하고, 해석하고, 말하고, 약속하고,
판단하고, 행동한다.

이 프로젝트의 중심은 지속적인 세계 상태다. actor가 무엇을 보았는지,
무엇을 믿는지, 어떤 사회적 관계와 규범 속에 있는지, 어떤 말을 했고 어떤
약속이 남았는지, 어떤 행동이 어떤 결과로 이어졌는지가 시뮬레이션 상태와
trace로 유지된다.

한 문장으로 정리하면 다음과 같다.

```text
world는 사회적 의미와 장기 결과를 가진 지속적 RPG 세계에서 인간과 LLM
에이전트가 부분 지식과 전략적 압력 아래 행동할 수 있게 하는
simulation-first engine이다.
```

## 왜 중요한가

복잡한 에이전트 환경에서는 action success만으로 행동을 설명하기 어렵다.
흥미로운 실패와 성공은 사회적 맥락에서 나온다.

```text
무엇을 알고 있었는가?
누가 봤는가?
어떤 약속을 했는가?
그 약속은 지켜졌는가?
거짓말은 성공했는가?
평판과 신뢰는 어떻게 변했는가?
단기 이득이 장기 손실로 돌아왔는가?
상대는 무엇을 알고 있다고 생각했는가?
```

이런 질문은 세계 상태, actor별 관찰, 믿음, 사회적 관계, speech act,
commitment, intent, action, consequence가 서로 연결되어 있을 때 다룰 수
있다. `world`는 사회적 행동의 원인과 결과를 시뮬레이션 상태로 유지하는
방향을 잡는다.

## 현재 시뮬레이션 코어

현재 아키텍처는 authority와 representation 경계를 명시한 domain-owned
runtime으로 설계되어 있다.

큰 흐름은 다음과 같다.

```text
checked definitions
  -> authoritative world state
  -> actor-relative query and observation
  -> semantic / cognitive / decision passes
  -> intent
  -> activity
  -> action request or process tick
  -> typed effect program
  -> causal transaction
  -> event record and store updates
```

hard truth mutation은 검증된 실행 경로를 가진다.

```text
ActionRequest / ProcessTick
  -> binding and validation
  -> Typed Effect Program
  -> CausalTransaction
  -> EventRecord + store updates
```

에이전트는 말하고, 제안하고, 선택하고, 시도한다. hard state 변화는 typed
effect와 causal transaction을 거쳐 event record로 남는다.

actor-facing context는 별도로 구성된다.

```text
WorldModel / EventHistory
  -> ObservationPipeline
  -> ObservedState / ObservedEvent
  -> ActorContextPipeline
  -> EpistemicWorkingSet
  -> SocialContextView
  -> CapabilitySet / ActionRepertoire / PerceivedAffordance
```

같은 hard world라도 actor마다 관찰, 믿음, 사회적 해석, 가능한 행동이
달라진다. normal actor-relative condition, targeted oracle condition, full
omniscient condition도 이 구분 위에서 명확해진다.

decision 쪽은 configurable middle-end로 구성된다.

```text
ObservedEvent / ActorContext
  -> epistemic and social context
  -> appraisal-like or strategic representation
  -> candidate intent
  -> selected or suggested intent
  -> activity
  -> action request
```

direct action, intent-only, typed speech, bounded theory-of-mind, oracle
context, full omniscience 같은 profile을 같은 world/scenario 위에서 비교할
수 있다.

`world`는 multi-resolution simulation도 전제로 한다. local area는 구체적
action과 perception으로 다루고, 먼 지역이나 장기 사회적 변화는
abstract/strategic process로 유지한다.

```text
local:
  concrete action, perception, affordance, combat, dialogue

near / abstract:
  active intent, process progress, risk, route, trace, delayed consequence

distant / strategic:
  faction pressure, regional process, reputation, economy, conflict trend
```

resolution은 detail과 execution policy를 정한다. authority는 어떤 state가
어떤 commit surface를 통해 기록되는지 정한다. hard consequence는 어느
resolution에서 생겨도 causal transaction과 event record로 남는다.

## 아키텍처적 강점

**1. Authority separation**

LLM, agent policy, appraisal pass, ToM pass, speech classifier는 proposal,
classification, choice를 만든다. hard mutation은 typed effect와 causal
transaction을 통해 발생한다. social, epistemic, appraisal, runtime-control
state도 각각의 accepted update gate를 가진다.

**2. Actor-relative state**

actor가 접근할 수 있는 observation, belief, social context, action
repertoire가 분리된다. 같은 사건도 actor마다 다르게 관찰되고, 다르게
기억되고, 다르게 해석될 수 있다.

**3. Typed intermediate representations**

speech, belief, commitment, intent, activity, action, event를 typed artifact로
남길 수 있다. 이 구조는 복잡한 사회적 행동을 시뮬레이션 상태로 다루게
해준다.

**4. Configurable decision structure**

인지 구조는 decision profile의 일부로 다룬다. `SpeechSurface`만 쓰는 조건,
`SpeechAct`와 `Commitment`를 명시하는 조건, `OtherModelView`를 쓰는 조건,
oracle로 상대 정보를 제공하는 조건을 같은 scenario 위에서 비교할 수 있다.

**5. Multi-resolution consequence**

약속 위반, 평판 손실, 제재, 소문, faction 반응은 시간이 지나서 나타난다.
multi-resolution process는 이런 delayed consequence를 큰 세계에서도 유지할
수 있게 한다.

**6. Traceability**

`world`의 trace는 engine-visible record를 중심으로 한다.

```text
actor가 무엇을 관찰했는가
어떤 belief/context가 제공되었는가
어떤 speech act가 생성되었는가
어떤 commitment가 생겼는가
어떤 intent가 선택되었는가
어떤 action이 요청되었는가
어떤 event와 social consequence가 발생했는가
```

이 trace는 결과 점수와 행동 과정을 함께 분석하게 해준다.

## LLM 에이전트와 잘 맞는 이유

LLM은 자연어, 사회적 상황, 의도 추론, 설득, 협상에서 강한 가능성을
보인다. 동시에 일관성, 장기 기억, 상태 추적, 정보 경계, 행동 검증에서는
취약할 수 있다.

`world`는 이 문제를 시뮬레이션 구조로 다룬다.

- LLM은 actor-relative context를 받는다.
- LLM은 action을 직접 실행하지 않고 `ActionRequest`를 만든다.
- speech는 raw text로 남거나 typed speech act로 해석된다.
- belief와 memory는 hard truth와 분리된다.
- intent와 activity는 action보다 앞선 commitment layer가 된다.
- oracle과 omniscient 조건은 diagnostic profile로 다룬다.

이 구조는 게임에도 유용하다. NPC가 더 일관된 동기로 행동하고, 플레이어의
말과 약속이 세계 상태에 남고, 먼 지역의 평판과 faction consequence가
지속될 수 있다. 같은 구조는 controlled experiment에도 이어진다.

## 열리는 질문

이 시뮬레이션 코어가 있으면 다음 질문을 실험할 수 있다.

- LLM agent는 사회적 비용과 장기 consequence를 얼마나 안정적으로 추적하는가?
- typed speech act는 promise, deception, accusation, negotiation을 더 잘
  추적하게 하는가?
- explicit intent/activity layer는 장기 행동 일관성을 개선하는가?
- bounded theory-of-mind는 어떤 상황에서 실제 도움이 되는가?
- oracle ToM과 generated ToM 사이의 gap은 얼마나 큰가?
- full omniscience에서도 실패하는 경우는 planning/action selection 병목을
  의미하는가?
- multi-resolution simulation은 social regret, reputation, sanction 같은
  delayed consequence를 평가 가능하게 만드는가?

좋은 결과는 구조가 모든 상황에서 점수를 올리는 형태일 필요가 없다. 어떤
구조는 특정 상황에서만 도움되고, 어떤 구조는 traceability는 높이지만
payoff는 개선하지 못할 수 있다. 충분히 구조화된 시뮬레이션은 이런 차이를
분석할 수 있게 한다.

## 기존 접근과의 관계

Generative Agents나 Concordia는 believable social simulation에 강하다.
`world`는 authority-bounded state, actor-relative context, causal event,
multi-resolution consequence를 강하게 가져간다.

CICERO, Avalon, Mafia, bargaining benchmark는 특정 고정 게임에서 강한
전략 능력을 측정한다. `world`는 scenario family를 만들고, 같은 seed에서
인지/사회적 representation만 바꾸어 볼 수 있는 실험 구조를 제공한다.

BDI, appraisal theory, theory of mind, speech act, social commitment,
normative multi-agent system은 중요한 배경이다. `world`는 이런 개념들을
세계 시뮬레이션 안에서 교체 가능한 representation과 pass로 다룬다.

## 평가 방향

평가는 시뮬레이션 코어가 실제로 가치 있는 행동 차이를 만들어내는지
확인하는 방법이다.

첫 실험은 작고 scoring 가능한 social game으로 시작하는 것이 좋다.

```text
promise under temptation:
  약속, 유혹, 증인, 제재, 평판

hidden-preference bargaining:
  private value, bluff, concession, deal surplus

witness and sanction:
  규범 위반, 목격자, 권위, 처벌, reputation cost
```

primary score는 가능한 한 simulation state에서 계산한다. 돈, 자원, 승패,
deal surplus, sanction cost, reputation delta, promise breach cost 같은 값이
중심이 된다.

핵심 결과는 total score보다 delta와 gap이다.

```text
typed_speech_delta
tom_delta
oracle_gap
omniscience_gap
long_horizon_social_regret
```

이 숫자들은 구조가 실제로 행동을 바꾸는지, 정보 병목인지, planning
병목인지, 장기 사회 비용을 놓치는지 보여준다.

## Contribution

논문이나 오픈소스 프로젝트로 발전시킨다면 contribution은 다음처럼 잡을 수
있다.

1. **Simulation engine**

   부분 관찰, 사회적 의미, commitment, 규범, 평판, 제재, delayed consequence를
   유지할 수 있는 simulation-first RPG/world engine.

2. **Authority-bounded architecture**

   truth, actor belief, social meaning, intent, activity, action, event를
   분리하고, LLM이나 cognitive pass의 proposal이 검증된 commit 경로를 통해
   세계에 반영되는 구조.

3. **Configurable social-cognitive decision structure**

   direct action, intent, typed speech, bounded ToM, oracle, omniscient 조건을
   같은 world/scenario 위에서 비교할 수 있는 profile 구조.

4. **Traceable social consequence**

   belief, speech, commitment, intent, action, outcome, consequence를 연결한
   trace로 행동 과정을 분석할 수 있는 substrate.

5. **Empirical evaluation path**

   여러 LLM과 profile을 비교하여 정보 병목, 상대 모델링 병목, commitment
   tracking 실패, 장기 social regret, planning/action selection 병목을
   분리해서 보여줄 수 있는 실험 경로.

## 리스크와 범위

리스크는 있다.

- 인간 인지 전체를 설명하는 방향으로 넓어질 수 있다.
- 연구 benchmark만 강조하면 시뮬레이션 엔진의 장점이 희석된다.
- 게임 엔진만 강조하면 연구 contribution이 약해진다.
- representation이 너무 많고 profile이 너무 자유로우면 underconstrained
  system으로 보일 수 있다.
- scoring이 작위적으로 보이면 설득력이 약하다.
- structured profile이 direct LLM보다 별 차이를 만들지 못할 수도 있다.

좋은 방향은 균형이다. `world`는 강한 시뮬레이션 코어를 중심에 두고, 그
위에서 LLM agent, 게임, 연구 benchmark가 각각 자연스럽게 나와야 한다.

## 최종 정리

```text
world is a simulation-first engine for persistent social-strategic worlds,
where human players and LLM agents act under partial knowledge, social meaning,
commitment, strategic pressure, and delayed consequences.
```

한국어로는 다음이다.

```text
world는 부분 지식, 사회적 의미, 약속, 전략적 압력, 지연된 결과가 있는
지속적 세계에서 인간과 LLM 에이전트가 함께 행동할 수 있게 하는
simulation-first engine이다.
```

게임은 이 시뮬레이션이 실제로 재미 있고 쓸모 있는지를 보여주는 응용이다.
연구는 이 시뮬레이션이 LLM agent의 사회적-전략적 행동을 얼마나 잘
드러내는지 검증하는 응용이다.
