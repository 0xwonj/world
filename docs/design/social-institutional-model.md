# Social Institutional Model

## Status

Current design draft.

## Source Ideas

- [Semantic Kernel And PL Boundary](../ideas/semantic-kernel-and-pl-boundary.md)
- [Actor Pressure And Interpretation](../ideas/actor-pressure-and-interpretation.md)
- [Layered Truth And AI Co-Authority](../ideas/layered-truth-and-ai-coauthority.md)
- [Epistemic State / Agent Memory](../research/epistemic-state-and-agent-memory.md)

## Related Design Owners

- [Engine Core And Game System Boundary](engine-core-and-game-system-boundary.md)
- [Simulation Transition Compiler](simulation-transition-compiler.md)

## Purpose

This document defines the social and institutional substrate that semantic
appraisal queries.

It sits between hard physical truth and semantic appraisal. It stores typed
social context such as relationships, faction membership, norms, law,
permission, social ownership, debts, oaths, and reputation. It does not decide
final emotional or motivational meaning by itself.

The substrate should be reusable. Concrete law, norm, taboo, office, religion,
rank, reputation, honor, debt, oath, and permission vocabularies may be
game-system pack definitions when they belong to a particular setting or rule
family.

## Boundary

This layer owns:

- relationships
- faction and institution identity
- membership, rank, role, and office
- `SocialClaim` over item, place, action, role, or authority
- norm, law, taboo, permission, and prohibition
- obligation, oath, debt, duty, and promise
- reputation and standing
- jurisdiction and scope
- social context views for semantic appraisal

This layer does not own:

- physical possession
- containment, equipment, or inventory transfer
- perception
- memory, belief, knowledge, rumor, or secret storage
- thought, pressure, goal, or intent
- final declaration that an event emotionally matters to a holder

## Core Distinctions

```text
physical possession:
  hard containment/equipment state

SocialClaim:
  social or institutional assertion about ownership, permission, role,
  authority, duty, or entitlement

actor belief:
  holder-relative epistemic state about possession or SocialClaim

semantic appraisal:
  interpreted meaning of an observed event in social context
```

Example:

```text
Hard truth:
  shrine_relic is in actor inventory

Social institutional state:
  SocialClaim(shrine owns shrine_relic)
  Norm(shrine forbids non-priest removal)

Epistemic state:
  actor may not know the taboo
  guard observed the transfer

Semantic appraisal:
  guard interprets event as theft and sacrilege
```

## State Families

### Relationship

Relationship state describes typed social connection between actors, groups,
institutions, places, or other social parties.

Examples:

```text
Relationship(actor_a, actor_b)
  kind: mentor
  emotional_weight: high
  trust: high
  loyalty: medium
  hostility: low
```

Relationship fields should be classified by owner:

```text
committed social relation:
  kinship, office relation, oath relation, mentorship, declared alliance,
  public feud, marriage, patronage, debt relation.

holder-relative epistemic relation:
  what one actor believes about trust, loyalty, betrayal, or hostility.

appraisal output:
  current anger, fear, grief, resentment, revenge pressure, guilt, or urgency.
```

Fields such as trust, loyalty, and hostility may exist as committed social
state only when the design explicitly wants public or institutionally tracked
standing. Otherwise they should be `EpistemicRecord` content or appraisal
inputs/outputs.

Relationships are context for appraisal. They should not directly select
actions.

### Faction, Institution, Membership, And Rank

Factions and institutions provide social identity and authority.

Examples:

```text
Faction:
  village_guard
  shrine_order
  merchant_guild

Membership:
  actor -> shrine_order
  role: priest
  rank: initiate

Office:
  actor -> village_guard
  authority: arrest_in_market
```

Rank and office may contribute to actor-owned capability when internalized or
recognized, such as `AssertAuthority`.

### SocialClaim

`SocialClaim` is the typed social assertion family for ownership, entitlement,
permission, or authority.

Examples:

```text
SocialClaim:
  claimant: shrine_order
  relation: owns
  subject: shrine_relic
  scope: shrine_law
  source: founding_charter

SocialClaim:
  claimant: village_guard
  relation: may_enter
  subject: restricted_armory
  scope: emergency_duty
```

`SocialClaim` is not physical state. It can conflict with physical possession
or actor belief.

### Norm, Law, Taboo, Permission, And Prohibition

Norms describe social rules inside a scope.

Examples:

```text
Norm:
  scope: shrine_inner_room
  forbids: non_priest_entry
  severity: high
  enforcement: shrine_guard

Law:
  scope: market_town
  forbids: assault
  severity: high
  enforcement: town_guard

Permission:
  actor: healer_1
  permits: enter_sickroom
  source: village_elder
```

Norms should be queryable by event role, place, actor role, target, institution,
and source.

### Obligation, Oath, Debt, Duty, And Promise

These are durable social commitments or constraints before appraisal turns
them into current motivation.

Examples:

```text
Debt:
  debtor: actor
  creditor: merchant
  value: favor
  due_context: next_market_day

Oath:
  actor: guard_1
  institution: village_guard
  duty: protect_villagers

Promise:
  actor: player
  recipient: mentor_1
  content: deliver_letter
```

The existence of a duty is social state. Feeling guilt, urgency, or loyalty is
semantic appraisal.

### Reputation And Standing

Reputation is social state about how an actor is regarded by a group, place, or
institution.

Examples:

```text
Reputation(actor, village)
  trust: medium
  fear: low
  honor: high

Standing(actor, shrine_order)
  status: suspect
  reason: observed_relic_removal
```

Reputation can be authoritative social state, while an individual actor's
belief about reputation belongs to epistemic state.

### Jurisdiction And Scope

Every social rule should have a scope.

Examples:

- place scope
- faction scope
- institution scope
- relationship scope
- ritual scope
- emergency scope
- time-limited scope

This prevents norms from becoming global hidden rules.

## Social Context View

Semantic appraisal should not query arbitrary social state directly. It should
receive a typed `SocialContextView` assembled for the observed event, holder,
place, and accessible epistemic context.

There are three useful view modes:

`SocialContextView` is the umbrella term for these access-filtered views.

```text
AuthoritativeSocialContextView:
  committed social/institutional state relevant to a rule, place, institution,
  or event. This is available to engine-side appraisal when the scenario mode
  permits authoritative social evaluation.

HolderKnownSocialContextView:
  social context the holder currently knows, believes, remembers, or can infer
  through `EpistemicRecord`s and current observation.

RoleGrantedSocialContextView:
  social context made available by role, office, membership, jurisdiction, or
  explicit permission, such as a guard's access to town law.
```

Actor-facing policies and AI agents should normally receive
`HolderKnownSocialContextView` plus any role-granted context. They should not
receive arbitrary authoritative social truth unless the mode explicitly allows
it.

Example input:

```text
ObservedEvent:
  actor took shrine_relic

Holder:
  shrine_guard_1

HolderKnownSocialContextView + RoleGrantedSocialContextView:
  relationship(shrine_guard_1, shrine_order)
    source: committed_social_state
    access: role_granted

  membership(shrine_guard_1, shrine_order, guard)
    source: committed_social_state
    access: role_granted

  SocialClaim(shrine_order owns shrine_relic)
    source: committed_social_state
    access: role_granted

  Norm(shrine forbids non-priest removal)
    source: committed_social_state
    access: role_granted

  Permission(actor may take shrine_relic)
    source: holder_belief
    confidence: low

  jurisdiction(shrine_inner_room)
    source: committed_social_state
    access: role_granted
```

Semantic appraisal can then produce:

```text
Thought:
  desecration_witnessed

Pressure:
  enforce_shrine_law
  warn_or_detain_actor
```

The social model supplies context. Appraisal decides current meaning and
pressure.

## Actor Belief About Social State

Actors may be wrong about social and institutional state.

Examples:

- actor believes the relic is abandoned
- guard falsely believes player has permission
- village believes a faction owns land it no longer controls
- outsider does not know a local taboo

These are `EpistemicRecord`s over social content, not replacement social truth.

Important distinction:

```text
Authoritative social state:
  the committed soft-truth record used by social rules.

Actor belief about social state:
  holder-relative `EpistemicRecord` over a SocialClaim, norm, law, permission,
  duty, rank, reputation, or institution.

Semantic appraisal:
  current interpretation of an observed event against one of the social context
  views.
```

## Commit Surface And Storage

Social and institutional state is committed soft truth.

[Pack Authoring And Semantic Declarations](pack-authoring-and-semantic-declarations.md)
owns the shared semantic declaration framework. Social and institutional
vocabularies use its `social_rule` declaration kind for checked social
interpretation and social update proposals.

[World Model](world-model.md) hosts the storage substrate:

```text
SocialInstitutionalStore
soft RelationStore families
```

This document owns the meaning, validation, lifecycle, and access rules for
those records. Writes use the social commit gate:

```text
social rule result / authored fact / AI proposal
  -> social commit gate
  -> AcceptedSocialUpdate
  -> SocialInstitutionalStore or soft RelationStore family
```

`AcceptedSocialUpdate` is the conceptual commit surface for:

- relationships
- faction and institution records
- membership, rank, office, and role records
- `SocialClaim`
- norm, law, taboo, permission, and prohibition records
- obligation, oath, debt, duty, and promise records
- reputation and standing records
- jurisdiction and scope records

An accepted update should carry:

- record kind
- affected parties, subject, scope, and jurisdiction
- authority source or proposer
- provenance, such as `EventRecord`, `EpistemicRecord`, authored source, or AI
  proposal reference
- effective time or condition
- supersession, contradiction, or conflict links where needed

This is not a final schema. It is the authority boundary: social state is not
written by hard physical effects, semantic appraisal, epistemic persistence, or
AI prose directly.

## AI Boundary

AI may propose social context such as:

- hidden relationship
- faction interest
- local rumor about ownership
- possible motive
- informal taboo

Accepted gameplay-relevant social state must become typed social/institutional
state through `AcceptedSocialUpdate` with provenance. AI-generated prose alone
is not enough.

## Relationship To Other Documents

- [World Model](world-model.md) defines where typed social and relation state
  is stored and queried.
- [Truth, Authority, And Layer Boundaries](truth-authority-and-layer-boundaries.md)
  defines soft truth authority and commit rules.
- [Epistemic State](epistemic-state.md) defines actor belief about social state.
- [Semantic Appraisal And Motivation](semantic-appraisal-and-motivation.md)
  consumes social context and produces thoughts and pressures.
- [Capability, Affordance, And Actor Interface](capability-affordance-and-actor-interface.md)
  may derive actor-owned authority actions from internalized or recognized
  social authority.

## Stable Decisions

- Social/institutional state is typed soft truth, not an unstructured fact
  graph.
- Physical possession, `SocialClaim`, and actor belief must stay separate.
- Social context feeds appraisal; it does not directly choose action.
- Social rules need explicit scope and provenance.
- AI may propose social state but cannot commit it without a typed acceptance
  path.
- [World Model](world-model.md) hosts `SocialInstitutionalStore` and soft
  `RelationStore` families; this document owns their social meaning and
  lifecycle.
- Gameplay-relevant social writes require `AcceptedSocialUpdate`.
- Concrete social and institutional vocabularies may be pack-owned, but
  accepted writes still use the social commit surface.

## Deferred Decisions

- exact social state schema
- exact physical layout between `SocialInstitutionalStore` tables and soft
  `RelationStore` families
- first norm/law/taboo taxonomy
- reputation math
- conflict handling between institutions
- inheritance between faction, culture, place, and institution
- how much social context actors can access without explicit knowledge
