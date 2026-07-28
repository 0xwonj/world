use std::collections::BTreeMap;

use world_core::{ActorId, EntityId, SimMoment};
use world_model::{
    AcceptedState, CommandEnvelope, CommandId, CommandSource, ContainmentTransferDelta,
    DomainStateError, StableCommandRejection,
};

use crate::authority::{ContainmentTransitionError, apply_containment_transfers};
use crate::execution::{
    ContainmentConflictPolicyV1, MomentResolutionPolicyV2, RandomKeyPolicyV1, RandomOraclePolicyV1,
};
use crate::randomness::{
    Blake3KeyedPrf256V1, ContainmentConflictContenderV1, ContainmentConflictGroupV1,
    ContainmentConflictRandomEvidenceV1, ContainmentConflictResourceV1, ContainmentRandomRankError,
    RandomScore256, rank_containment_conflict,
};

/// Canonical logical identity of one newly evaluated containment command.
///
/// Persistence, scheduler, worker, and collection coordinates are
/// deliberately absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ContainmentCommandIdentity {
    source: CommandSource,
    command: CommandId,
}

impl ContainmentCommandIdentity {
    #[must_use]
    pub(crate) const fn new(source: CommandSource, command: CommandId) -> Self {
        Self { source, command }
    }

    #[must_use]
    pub(crate) fn from_command(command: &CommandEnvelope) -> Self {
        Self::new(command.source(), command.id())
    }

    #[must_use]
    pub(crate) const fn source(self) -> CommandSource {
        self.source
    }

    #[must_use]
    pub(crate) const fn command(self) -> CommandId {
        self.command
    }

    const fn contender(self, actor: ActorId) -> ContainmentConflictContenderV1 {
        ContainmentConflictContenderV1::new(actor, self.source, self.command)
    }

    const fn from_contender(contender: ContainmentConflictContenderV1) -> Self {
        Self::new(contender.source(), contender.command())
    }
}

/// Closed semantic result proposed by one independently evaluated command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContainmentCandidateProposal {
    /// Evaluation already produced a stable rejection.
    Rejected(StableCommandRejection),
    /// Evaluation proposed one typed containment transition.
    Transfer(ContainmentTransferDelta),
}

/// One logical command and its complete evaluator proposal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ContainmentCandidate {
    identity: ContainmentCommandIdentity,
    actor: ActorId,
    proposal: ContainmentCandidateProposal,
}

impl ContainmentCandidate {
    #[must_use]
    pub(crate) const fn new(
        identity: ContainmentCommandIdentity,
        actor: ActorId,
        proposal: ContainmentCandidateProposal,
    ) -> Self {
        Self {
            identity,
            actor,
            proposal,
        }
    }
}

/// Why a candidate collection was not a set of logical commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContainmentCandidateSetError {
    DuplicateCommand {
        identity: ContainmentCommandIdentity,
    },
}

/// Canonical, non-duplicated candidates for one due moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContainmentCandidateSet {
    candidates: Vec<ContainmentCandidate>,
}

impl ContainmentCandidateSet {
    pub(crate) fn new(
        mut candidates: Vec<ContainmentCandidate>,
    ) -> Result<Self, ContainmentCandidateSetError> {
        candidates.sort_by_key(|candidate| candidate.identity);
        if let Some(pair) = candidates
            .windows(2)
            .find(|pair| pair[0].identity == pair[1].identity)
        {
            return Err(ContainmentCandidateSetError::DuplicateCommand {
                identity: pair[0].identity,
            });
        }
        Ok(Self { candidates })
    }

    #[must_use]
    pub(crate) fn candidates(&self) -> &[ContainmentCandidate] {
        &self.candidates
    }
}

/// Immutable accepted-state fact read by one containment transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ContainmentReadKey {
    ItemContainment(EntityId),
    Container(EntityId),
    SourceAuthority { actor: ActorId, container: EntityId },
    DirectItemCount(EntityId),
}

/// Accepted-state relation written by one containment transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ContainmentWriteKey {
    ItemContainment(EntityId),
}

/// Combined invariant that must hold after selected deltas are applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ContainmentInvariantKey {
    ContainerCapacity(EntityId),
}

/// Complete concrete footprint of one independently valid transfer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContainmentTransactionFootprint {
    reads: Vec<ContainmentReadKey>,
    writes: Vec<ContainmentWriteKey>,
    resources: Vec<ContainmentConflictResourceV1>,
    invariants: Vec<ContainmentInvariantKey>,
}

impl ContainmentTransactionFootprint {
    fn for_transfer(delta: ContainmentTransferDelta) -> Self {
        let mut reads = vec![
            ContainmentReadKey::ItemContainment(delta.item()),
            ContainmentReadKey::Container(delta.expected_source()),
            ContainmentReadKey::Container(delta.destination()),
            ContainmentReadKey::SourceAuthority {
                actor: delta.actor(),
                container: delta.expected_source(),
            },
            ContainmentReadKey::DirectItemCount(delta.destination()),
        ];
        reads.sort_unstable();
        reads.dedup();
        let mut invariants = vec![
            ContainmentInvariantKey::ContainerCapacity(delta.expected_source()),
            ContainmentInvariantKey::ContainerCapacity(delta.destination()),
        ];
        invariants.sort_unstable();
        invariants.dedup();

        Self {
            reads,
            writes: vec![ContainmentWriteKey::ItemContainment(delta.item())],
            resources: vec![
                ContainmentConflictResourceV1::ExclusiveItem(delta.item()),
                ContainmentConflictResourceV1::DestinationCapacity(delta.destination()),
            ],
            invariants,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn writes(&self) -> &[ContainmentWriteKey] {
        &self.writes
    }

    #[must_use]
    pub(crate) fn resources(&self) -> &[ContainmentConflictResourceV1] {
        &self.resources
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn invariants(&self) -> &[ContainmentInvariantKey] {
        &self.invariants
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EligibleContainmentCandidate {
    identity: ContainmentCommandIdentity,
    actor: ActorId,
    delta: ContainmentTransferDelta,
    footprint: ContainmentTransactionFootprint,
}

impl EligibleContainmentCandidate {
    const fn contender(&self) -> ContainmentConflictContenderV1 {
        self.identity.contender(self.actor)
    }
}

/// Complete authoritative outcome of one resolver candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ContainmentCandidateOutcome {
    Accepted { delta: ContainmentTransferDelta },
    Rejected(StableCommandRejection),
}

/// One canonical candidate identity and its exactly one terminal resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedContainmentCandidate {
    identity: ContainmentCommandIdentity,
    actor: ActorId,
    footprint: Option<ContainmentTransactionFootprint>,
    outcome: ContainmentCandidateOutcome,
}

impl ResolvedContainmentCandidate {
    #[must_use]
    pub(crate) const fn identity(&self) -> ContainmentCommandIdentity {
        self.identity
    }

    #[must_use]
    pub(crate) const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the independently certified footprint when evaluation proposed
    /// a valid transfer, including for a contender rejected by resolution.
    #[must_use]
    pub(crate) const fn footprint(&self) -> Option<&ContainmentTransactionFootprint> {
        self.footprint.as_ref()
    }

    #[must_use]
    pub(crate) const fn outcome(&self) -> &ContainmentCandidateOutcome {
        &self.outcome
    }
}

/// Random evidence for one exact resource and only its actual contenders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContainmentResourceEvidence {
    group: ContainmentConflictGroupV1,
    admission_limit: u32,
    ranking: ContainmentConflictRandomEvidenceV1,
}

impl ContainmentResourceEvidence {
    #[must_use]
    pub(crate) const fn group(&self) -> ContainmentConflictGroupV1 {
        self.group
    }

    #[must_use]
    pub(crate) const fn admission_limit(&self) -> u32 {
        self.admission_limit
    }

    #[must_use]
    pub(crate) const fn ranking(&self) -> &ContainmentConflictRandomEvidenceV1 {
        &self.ranking
    }
}

/// Resource-accurate evidence retained for one connected conflict component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContainmentComponentEvidence {
    contenders: Vec<ContainmentConflictContenderV1>,
    resources: Vec<ContainmentResourceEvidence>,
}

impl ContainmentComponentEvidence {
    #[must_use]
    pub(crate) fn contenders(&self) -> &[ContainmentConflictContenderV1] {
        &self.contenders
    }

    #[must_use]
    pub(crate) fn resources(&self) -> &[ContainmentResourceEvidence] {
        &self.resources
    }
}

/// Typed reason the resolver deliberately selected its rejection-only result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContainmentResolutionFallback {
    RandomEvidence {
        group: ContainmentConflictGroupV1,
        admission_limit: u32,
        error: ContainmentRandomRankError,
    },
    CombinedTransition {
        error: ContainmentTransitionError,
    },
}

/// Complete policy and random evidence for one containment resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContainmentResolutionEvidence {
    resolution_policy: MomentResolutionPolicyV2,
    conflict_policy: ContainmentConflictPolicyV1,
    random_oracle_policy: RandomOraclePolicyV1,
    random_key_policy: RandomKeyPolicyV1,
    components: Vec<ContainmentComponentEvidence>,
    fallback: Option<ContainmentResolutionFallback>,
}

impl ContainmentResolutionEvidence {
    #[must_use]
    pub(crate) const fn resolution_policy(&self) -> MomentResolutionPolicyV2 {
        self.resolution_policy
    }

    #[must_use]
    pub(crate) const fn conflict_policy(&self) -> ContainmentConflictPolicyV1 {
        self.conflict_policy
    }

    #[must_use]
    pub(crate) const fn random_oracle_policy(&self) -> RandomOraclePolicyV1 {
        self.random_oracle_policy
    }

    #[must_use]
    pub(crate) const fn random_key_policy(&self) -> RandomKeyPolicyV1 {
        self.random_key_policy
    }

    #[must_use]
    pub(crate) fn components(&self) -> &[ContainmentComponentEvidence] {
        &self.components
    }

    #[must_use]
    pub(crate) const fn fallback(&self) -> Option<ContainmentResolutionFallback> {
        self.fallback
    }
}

/// Total, canonical containment result over one immutable accepted-state base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContainmentMomentResolution {
    outcomes: Vec<ResolvedContainmentCandidate>,
    accepted_deltas: Vec<ContainmentTransferDelta>,
    successor: AcceptedState,
    evidence: ContainmentResolutionEvidence,
}

impl ContainmentMomentResolution {
    #[must_use]
    pub(crate) fn outcomes(&self) -> &[ResolvedContainmentCandidate] {
        &self.outcomes
    }

    #[must_use]
    pub(crate) fn accepted_deltas(&self) -> &[ContainmentTransferDelta] {
        &self.accepted_deltas
    }

    #[must_use]
    pub(crate) const fn successor(&self) -> &AcceptedState {
        &self.successor
    }

    #[must_use]
    pub(crate) const fn evidence(&self) -> &ContainmentResolutionEvidence {
        &self.evidence
    }
}

/// Resolves all candidates against one shared immutable base.
#[must_use]
pub(crate) fn resolve_containment_candidates(
    moment: SimMoment,
    base: &AcceptedState,
    candidates: &ContainmentCandidateSet,
    oracle: &Blake3KeyedPrf256V1,
) -> ContainmentMomentResolution {
    resolve_containment_candidates_with(moment, base, candidates, |group, contenders| {
        rank_containment_conflict(oracle, group, contenders)
    })
}

fn resolve_containment_candidates_with(
    moment: SimMoment,
    base: &AcceptedState,
    candidates: &ContainmentCandidateSet,
    mut rank: impl FnMut(
        ContainmentConflictGroupV1,
        &[ContainmentConflictContenderV1],
    )
        -> Result<ContainmentConflictRandomEvidenceV1, Box<ContainmentRandomRankError>>,
) -> ContainmentMomentResolution {
    let mut terminal = BTreeMap::new();
    let mut certified_footprints = BTreeMap::new();
    let mut eligible = Vec::new();

    for candidate in candidates.candidates() {
        match candidate.proposal {
            ContainmentCandidateProposal::Rejected(reason) => {
                terminal.insert(
                    candidate.identity,
                    ContainmentCandidateOutcome::Rejected(reason),
                );
            }
            ContainmentCandidateProposal::Transfer(delta) => {
                match independently_revalidate(base, candidate.actor, delta) {
                    Ok(footprint) => {
                        certified_footprints.insert(candidate.identity, footprint.clone());
                        eligible.push(EligibleContainmentCandidate {
                            identity: candidate.identity,
                            actor: candidate.actor,
                            delta,
                            footprint,
                        });
                    }
                    Err(reason) => {
                        terminal.insert(
                            candidate.identity,
                            ContainmentCandidateOutcome::Rejected(reason),
                        );
                    }
                }
            }
        }
    }

    let binding_limits = binding_resource_limits(base, &eligible);
    let components = connected_components(&eligible, &binding_limits);
    let mut component_evidence = Vec::new();
    let mut resource_priorities = BTreeMap::new();
    let mut candidate_priorities = BTreeMap::new();

    for component in &components {
        let binding_resources: Vec<_> = binding_limits
            .iter()
            .filter(|(resource, _)| {
                component
                    .iter()
                    .any(|index| eligible[*index].footprint.resources().contains(resource))
            })
            .map(|(resource, limit)| (*resource, *limit))
            .collect();
        if binding_resources.is_empty() {
            continue;
        }
        let mut component_contenders: Vec<_> = component
            .iter()
            .map(|index| eligible[*index].contender())
            .collect();
        component_contenders.sort_unstable();
        let mut resource_evidence = Vec::with_capacity(binding_resources.len());

        for (resource, admission_limit) in binding_resources {
            let group = ContainmentConflictGroupV1::new(moment, resource);
            let resource_candidates: Vec<_> = component
                .iter()
                .map(|index| &eligible[*index])
                .filter(|candidate| candidate.footprint.resources().contains(&resource))
                .collect();
            let contenders: Vec<_> = resource_candidates
                .iter()
                .map(|candidate| candidate.contender())
                .collect();
            let ranking = match rank(group, &contenders) {
                Ok(ranking) => ranking,
                Err(error) => {
                    return rejection_only(
                        base,
                        candidates,
                        &certified_footprints,
                        component_evidence,
                        ContainmentResolutionFallback::RandomEvidence {
                            group,
                            admission_limit,
                            error: *error,
                        },
                    );
                }
            };

            let mut scores = BTreeMap::new();
            for entry in ranking.entries() {
                scores.insert(
                    ContainmentCommandIdentity::from_contender(entry.key().contender()),
                    entry.score(),
                );
            }
            for candidate in resource_candidates {
                let score = *scores
                    .get(&candidate.identity)
                    .unwrap_or_else(|| unreachable!("ranking must cover every contender"));
                // A contender spanning several constrained resources receives one
                // component priority from its weakest resource-local rank. A single
                // greedy capacity pass can then admit a maximal feasible set; taking
                // the intersection of independent resource winners can reject every
                // otherwise feasible contender.
                candidate_priorities
                    .entry(candidate.identity)
                    .and_modify(|priority: &mut RandomScore256| {
                        *priority = (*priority).min(score);
                    })
                    .or_insert(score);
            }

            if resource_priorities.insert(resource, scores).is_some() {
                unreachable!("one semantic resource belongs to one connected component");
            }
            resource_evidence.push(ContainmentResourceEvidence {
                group,
                admission_limit,
                ranking,
            });
        }
        component_evidence.push(ContainmentComponentEvidence {
            contenders: component_contenders,
            resources: resource_evidence,
        });
    }

    eligible.sort_by(|left, right| {
        match (
            candidate_priorities.get(&left.identity),
            candidate_priorities.get(&right.identity),
        ) {
            (Some(left_score), Some(right_score)) => right_score
                .cmp(left_score)
                .then_with(|| left.identity.cmp(&right.identity)),
            (Some(_), None) => core::cmp::Ordering::Less,
            (None, Some(_)) => core::cmp::Ordering::Greater,
            (None, None) => left.identity.cmp(&right.identity),
        }
    });

    let mut selected = BTreeMap::new();
    let mut admitted_by_resource = BTreeMap::<ContainmentConflictResourceV1, u32>::new();
    for candidate in eligible {
        let fits = candidate.footprint.resources().iter().all(|resource| {
            binding_limits.get(resource).is_none_or(|limit| {
                admitted_by_resource.get(resource).copied().unwrap_or(0) < *limit
            })
        });
        if !fits {
            terminal.insert(
                candidate.identity,
                ContainmentCandidateOutcome::Rejected(StableCommandRejection::Conflict),
            );
        } else {
            for resource in candidate.footprint.resources() {
                if binding_limits.contains_key(resource) {
                    let admitted = admitted_by_resource.entry(*resource).or_default();
                    *admitted = admitted.saturating_add(1);
                }
            }
            selected.insert(candidate.identity, candidate);
        }
    }

    let successor = loop {
        let deltas = selected
            .values()
            .map(|candidate| candidate.delta)
            .collect::<Vec<_>>();
        match apply_containment_transfers(base, &deltas) {
            Ok(successor) => break successor,
            Err(error) => {
                let Some(identity) = refinement_removal(&selected, &resource_priorities, error)
                else {
                    return rejection_only(
                        base,
                        candidates,
                        &certified_footprints,
                        component_evidence,
                        ContainmentResolutionFallback::CombinedTransition { error },
                    );
                };
                selected.remove(&identity);
                terminal.insert(
                    identity,
                    ContainmentCandidateOutcome::Rejected(StableCommandRejection::Conflict),
                );
            }
        }
    };

    for candidate in selected.values() {
        terminal.insert(
            candidate.identity,
            ContainmentCandidateOutcome::Accepted {
                delta: candidate.delta,
            },
        );
    }

    let outcomes = candidates
        .candidates()
        .iter()
        .map(|candidate| ResolvedContainmentCandidate {
            identity: candidate.identity,
            actor: candidate.actor,
            footprint: certified_footprints.get(&candidate.identity).cloned(),
            outcome: terminal
                .remove(&candidate.identity)
                .unwrap_or_else(|| unreachable!("every resolver candidate must be terminal")),
        })
        .collect();
    let accepted_deltas = selected.values().map(|candidate| candidate.delta).collect();

    ContainmentMomentResolution {
        outcomes,
        accepted_deltas,
        successor,
        evidence: policy_evidence(component_evidence, None),
    }
}

fn independently_revalidate(
    base: &AcceptedState,
    actor: ActorId,
    delta: ContainmentTransferDelta,
) -> Result<ContainmentTransactionFootprint, StableCommandRejection> {
    if actor != delta.actor() {
        return Err(StableCommandRejection::BindingMismatch);
    }
    apply_containment_transfers(base, &[delta])
        .map(|_| ContainmentTransactionFootprint::for_transfer(delta))
        .map_err(revalidation_rejection)
}

fn revalidation_rejection(error: ContainmentTransitionError) -> StableCommandRejection {
    match error {
        ContainmentTransitionError::ItemNotContained { .. }
        | ContainmentTransitionError::SourceMismatch { .. } => StableCommandRejection::Stale,
        ContainmentTransitionError::DestinationContainerMissing { .. }
        | ContainmentTransitionError::SourceAuthorityMissing { .. }
        | ContainmentTransitionError::InvalidSuccessor(_) => {
            StableCommandRejection::RequirementUnsatisfied
        }
        ContainmentTransitionError::DuplicateItemClaim { .. } => StableCommandRejection::Conflict,
    }
}

fn connected_components(
    candidates: &[EligibleContainmentCandidate],
    binding_limits: &BTreeMap<ContainmentConflictResourceV1, u32>,
) -> Vec<Vec<usize>> {
    let mut assigned = vec![false; candidates.len()];
    let mut components = Vec::new();

    for start in 0..candidates.len() {
        if assigned[start] {
            continue;
        }
        assigned[start] = true;
        let mut component = vec![start];
        let mut cursor = 0;
        while cursor < component.len() {
            let current = component[cursor];
            for other in 0..candidates.len() {
                if !assigned[other]
                    && shares_binding_resource(
                        &candidates[current],
                        &candidates[other],
                        binding_limits,
                    )
                {
                    assigned[other] = true;
                    component.push(other);
                }
            }
            cursor += 1;
        }
        component.sort_unstable();
        components.push(component);
    }
    components
}

fn shares_binding_resource(
    left: &EligibleContainmentCandidate,
    right: &EligibleContainmentCandidate,
    binding_limits: &BTreeMap<ContainmentConflictResourceV1, u32>,
) -> bool {
    left.footprint.resources().iter().any(|resource| {
        binding_limits.contains_key(resource) && right.footprint.resources().contains(resource)
    })
}

fn binding_resource_limits(
    base: &AcceptedState,
    candidates: &[EligibleContainmentCandidate],
) -> BTreeMap<ContainmentConflictResourceV1, u32> {
    let mut uses = BTreeMap::<ContainmentConflictResourceV1, usize>::new();
    for candidate in candidates {
        for resource in candidate.footprint.resources() {
            *uses.entry(*resource).or_default() += 1;
        }
    }
    uses.into_iter()
        .filter_map(|(resource, count)| match resource {
            ContainmentConflictResourceV1::ExclusiveItem(_) if count > 1 => Some((resource, 1)),
            ContainmentConflictResourceV1::DestinationCapacity(container) => {
                let capacity = base
                    .domain()
                    .container(container)
                    .map(|record| u64::from(record.item_capacity()))
                    .unwrap_or_else(|| {
                        unreachable!("eligible transfer destination must be a container")
                    });
                let occupied = base
                    .domain()
                    .containment()
                    .iter()
                    .filter(|record| record.container() == container)
                    .fold(0_u64, |total, _| total.saturating_add(1));
                let remaining = capacity.saturating_sub(occupied);
                let incoming = u64::try_from(count).unwrap_or(u64::MAX);
                (incoming > remaining)
                    .then_some((resource, u32::try_from(remaining).unwrap_or(u32::MAX)))
            }
            ContainmentConflictResourceV1::ExclusiveItem(_) => None,
        })
        .collect()
}

fn refinement_removal(
    selected: &BTreeMap<ContainmentCommandIdentity, EligibleContainmentCandidate>,
    priorities: &BTreeMap<
        ContainmentConflictResourceV1,
        BTreeMap<ContainmentCommandIdentity, RandomScore256>,
    >,
    error: ContainmentTransitionError,
) -> Option<ContainmentCommandIdentity> {
    let ContainmentTransitionError::InvalidSuccessor(DomainStateError::ContainerCapacityExceeded {
        container,
        ..
    }) = error
    else {
        return None;
    };
    let resource = ContainmentConflictResourceV1::DestinationCapacity(container);
    let scores = priorities.get(&resource)?;

    selected
        .values()
        .filter(|candidate| candidate.delta.destination() == container)
        .filter_map(|candidate| {
            scores
                .get(&candidate.identity)
                .map(|score| (*score, candidate.identity))
        })
        .min_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, identity)| identity)
}

fn rejection_only(
    base: &AcceptedState,
    candidates: &ContainmentCandidateSet,
    certified_footprints: &BTreeMap<ContainmentCommandIdentity, ContainmentTransactionFootprint>,
    components: Vec<ContainmentComponentEvidence>,
    fallback: ContainmentResolutionFallback,
) -> ContainmentMomentResolution {
    let outcomes = candidates
        .candidates()
        .iter()
        .map(|candidate| {
            let reason = match candidate.proposal {
                ContainmentCandidateProposal::Rejected(reason) => reason,
                ContainmentCandidateProposal::Transfer(_) => StableCommandRejection::Conflict,
            };
            ResolvedContainmentCandidate {
                identity: candidate.identity,
                actor: candidate.actor,
                footprint: certified_footprints.get(&candidate.identity).cloned(),
                outcome: ContainmentCandidateOutcome::Rejected(reason),
            }
        })
        .collect();
    let successor = apply_containment_transfers(base, &[])
        .unwrap_or_else(|_| unreachable!("an accepted-state value must remain valid unchanged"));
    ContainmentMomentResolution {
        outcomes,
        accepted_deltas: Vec::new(),
        successor,
        evidence: policy_evidence(components, Some(fallback)),
    }
}

fn policy_evidence(
    components: Vec<ContainmentComponentEvidence>,
    fallback: Option<ContainmentResolutionFallback>,
) -> ContainmentResolutionEvidence {
    ContainmentResolutionEvidence {
        resolution_policy: MomentResolutionPolicyV2::CanonicalComponentGreedy,
        conflict_policy: ContainmentConflictPolicyV1::EqualHighestRandomWeight,
        random_oracle_policy: RandomOraclePolicyV1::Blake3KeyedPrf256,
        random_key_policy: RandomKeyPolicyV1::SemanticContainmentConflict,
        components,
        fallback,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use world_core::{Microstep, SimTime};
    use world_model::{
        ActionOpportunity, ActionOpportunityGeneration, ActionSponsor, ActorReactionCause,
        AgencyState, ContainerAuthorityRecord, ContainerRecord, ContainmentInteractionScope,
        ContainmentRecord, DomainState, EpistemicState, SocialState,
    };

    use crate::execution::RootSeed;

    use super::*;

    fn actor(byte: u8) -> ActorId {
        ActorId::from_bytes([byte; 32])
    }

    fn entity(byte: u8) -> EntityId {
        EntityId::from_bytes([byte; 32])
    }

    fn moment() -> SimMoment {
        SimMoment::new(SimTime::from_ticks(11), Microstep::new(2))
    }

    fn state(destination_capacity: u32) -> AcceptedState {
        state_with_capacities(destination_capacity, 3)
    }

    fn state_with_capacities(
        first_destination_capacity: u32,
        second_destination_capacity: u32,
    ) -> AcceptedState {
        AcceptedState::new(
            DomainState::new(
                vec![
                    ContainerRecord::new(entity(0x31), 3),
                    ContainerRecord::new(entity(0x32), 3),
                    ContainerRecord::new(entity(0x41), first_destination_capacity),
                    ContainerRecord::new(entity(0x42), second_destination_capacity),
                    ContainerRecord::new(entity(0x43), 3),
                ],
                vec![
                    ContainmentRecord::new(entity(0x21), entity(0x31)),
                    ContainmentRecord::new(entity(0x22), entity(0x31)),
                    ContainmentRecord::new(entity(0x23), entity(0x32)),
                ],
                vec![
                    ContainerAuthorityRecord::new(actor(0x11), entity(0x31)),
                    ContainerAuthorityRecord::new(actor(0x12), entity(0x31)),
                    ContainerAuthorityRecord::new(actor(0x13), entity(0x32)),
                ],
            )
            .unwrap_or_else(|error| panic!("domain-state fixture must be valid: {error}")),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        )
    }

    fn identity(source_byte: u8, command: u64) -> ContainmentCommandIdentity {
        ContainmentCommandIdentity::new(
            CommandSource::from_bytes([source_byte; 32]),
            CommandId::new(command),
        )
    }

    fn transfer(
        actor_byte: u8,
        identity: ContainmentCommandIdentity,
        item: u8,
        source: u8,
        destination: u8,
    ) -> ContainmentCandidate {
        let delta = ContainmentTransferDelta::new(
            actor(actor_byte),
            entity(item),
            entity(source),
            entity(destination),
        )
        .unwrap_or_else(|error| panic!("transfer fixture must be valid: {error}"));
        ContainmentCandidate::new(
            identity,
            actor(actor_byte),
            ContainmentCandidateProposal::Transfer(delta),
        )
    }

    fn candidate_set(candidates: Vec<ContainmentCandidate>) -> ContainmentCandidateSet {
        ContainmentCandidateSet::new(candidates)
            .unwrap_or_else(|error| panic!("candidate fixture must be canonicalizable: {error:?}"))
    }

    fn oracle() -> Blake3KeyedPrf256V1 {
        Blake3KeyedPrf256V1::from_root_seed(RootSeed::from_bytes([0x5a; 32]))
    }

    fn resolve(
        base: &AcceptedState,
        candidates: &ContainmentCandidateSet,
    ) -> ContainmentMomentResolution {
        resolve_containment_candidates(moment(), base, candidates, &oracle())
    }

    fn outcome(
        resolution: &ContainmentMomentResolution,
        identity: ContainmentCommandIdentity,
    ) -> &ContainmentCandidateOutcome {
        resolution
            .outcomes()
            .iter()
            .find(|candidate| candidate.identity() == identity)
            .map(ResolvedContainmentCandidate::outcome)
            .unwrap_or_else(|| panic!("resolution must cover candidate {identity:?}"))
    }

    fn is_accepted(outcome: &ContainmentCandidateOutcome) -> bool {
        matches!(outcome, ContainmentCandidateOutcome::Accepted { .. })
    }

    fn assert_component_selection_is_feasible_and_maximal(
        resolution: &ContainmentMomentResolution,
    ) {
        let accepted = resolution
            .outcomes()
            .iter()
            .filter(|candidate| is_accepted(candidate.outcome()))
            .map(ResolvedContainmentCandidate::identity)
            .collect::<BTreeSet<_>>();

        for component in resolution.evidence().components() {
            for resource in component.resources() {
                let admitted = resource
                    .ranking()
                    .entries()
                    .iter()
                    .filter(|entry| {
                        accepted.contains(&ContainmentCommandIdentity::from_contender(
                            entry.key().contender(),
                        ))
                    })
                    .count();
                assert!(admitted <= resource.admission_limit() as usize);
            }

            for contender in component.contenders() {
                let identity = ContainmentCommandIdentity::from_contender(*contender);
                if accepted.contains(&identity) {
                    continue;
                }
                assert!(component.resources().iter().any(|resource| {
                    let participates = resource.ranking().entries().iter().any(|entry| {
                        ContainmentCommandIdentity::from_contender(entry.key().contender())
                            == identity
                    });
                    let admitted = resource
                        .ranking()
                        .entries()
                        .iter()
                        .filter(|entry| {
                            accepted.contains(&ContainmentCommandIdentity::from_contender(
                                entry.key().contender(),
                            ))
                        })
                        .count();
                    participates && admitted >= resource.admission_limit() as usize
                }));
            }
        }
    }

    #[test]
    fn resolution_is_permutation_invariant_and_keeps_a_disjoint_candidate() {
        let first = identity(0x51, 1);
        let second = identity(0x52, 2);
        let disjoint = identity(0x53, 3);
        let candidates = [
            transfer(0x11, first, 0x21, 0x31, 0x41),
            transfer(0x12, second, 0x21, 0x31, 0x42),
            transfer(0x13, disjoint, 0x23, 0x32, 0x43),
        ];
        let base = state(3);

        let forward = resolve(&base, &candidate_set(candidates.to_vec()));
        let reversed = resolve(
            &base,
            &candidate_set(candidates.iter().rev().copied().collect()),
        );
        let rotated = resolve(
            &base,
            &candidate_set(vec![candidates[1], candidates[2], candidates[0]]),
        );

        assert_eq!(forward, reversed);
        assert_eq!(forward, rotated);
        assert_eq!(forward.outcomes().len(), 3);
        assert_eq!(
            [first, second]
                .into_iter()
                .filter(|identity| is_accepted(outcome(&forward, *identity)))
                .count(),
            1
        );
        assert!(is_accepted(outcome(&forward, disjoint)));
        assert_eq!(forward.accepted_deltas().len(), 2);
        assert_eq!(
            apply_containment_transfers(&base, forward.accepted_deltas()),
            Ok(forward.successor().clone())
        );
        assert!(forward.evidence().fallback().is_none());
    }

    #[test]
    fn shared_destination_capacity_admits_only_hrw_top_remaining_slots() {
        let first = identity(0x61, 1);
        let second = identity(0x62, 2);
        let base = state(1);
        let candidates = candidate_set(vec![
            transfer(0x11, first, 0x21, 0x31, 0x41),
            transfer(0x13, second, 0x23, 0x32, 0x41),
        ]);

        let resolution = resolve(&base, &candidates);

        assert_eq!(resolution.accepted_deltas().len(), 1);
        assert_eq!(
            [first, second]
                .into_iter()
                .filter(|identity| is_accepted(outcome(&resolution, *identity)))
                .count(),
            1
        );
        assert!(resolution.outcomes().iter().any(|candidate| matches!(
            candidate.outcome(),
            ContainmentCandidateOutcome::Rejected(StableCommandRejection::Conflict)
        )));
        assert_eq!(
            resolution
                .successor()
                .domain()
                .containment()
                .iter()
                .filter(|record| record.container() == entity(0x41))
                .count(),
            1
        );

        let [component] = resolution.evidence().components() else {
            panic!("shared capacity must produce one conflict component");
        };
        assert_eq!(
            resolution.evidence().resolution_policy(),
            MomentResolutionPolicyV2::CanonicalComponentGreedy
        );
        assert_eq!(
            resolution.evidence().conflict_policy(),
            ContainmentConflictPolicyV1::EqualHighestRandomWeight
        );
        assert_eq!(
            resolution.evidence().random_oracle_policy(),
            RandomOraclePolicyV1::Blake3KeyedPrf256
        );
        assert_eq!(
            resolution.evidence().random_key_policy(),
            RandomKeyPolicyV1::SemanticContainmentConflict
        );
        let [resource] = component.resources() else {
            panic!("binding capacity must produce one resource ranking");
        };
        assert_eq!(
            resource.group().resource(),
            ContainmentConflictResourceV1::DestinationCapacity(entity(0x41))
        );
        assert_eq!(resource.admission_limit(), 1);
        assert_eq!(resource.ranking().entries().len(), 2);
        let accepted_identity = resolution
            .outcomes()
            .iter()
            .find(|candidate| is_accepted(candidate.outcome()))
            .map(ResolvedContainmentCandidate::identity)
            .unwrap_or_else(|| panic!("one capacity contender must be accepted"));
        assert_eq!(
            accepted_identity,
            ContainmentCommandIdentity::from_contender(resource.ranking().winner())
        );
        assert_eq!(
            apply_containment_transfers(&base, resolution.accepted_deltas()),
            Ok(resolution.successor().clone())
        );

        let accepted = resolution
            .outcomes()
            .iter()
            .find_map(|candidate| {
                is_accepted(candidate.outcome())
                    .then(|| candidate.footprint())
                    .flatten()
            })
            .unwrap_or_else(|| panic!("one capacity contender must be accepted"));
        assert_eq!(accepted.writes().len(), 1);
        assert!(accepted.resources().contains(
            &ContainmentConflictResourceV1::DestinationCapacity(entity(0x41))
        ));
        assert!(
            accepted
                .invariants()
                .contains(&ContainmentInvariantKey::ContainerCapacity(entity(0x41)))
        );
    }

    #[test]
    fn sufficient_destination_capacity_needs_no_random_conflict_group() {
        let first = identity(0x63, 1);
        let second = identity(0x64, 2);
        let base = state(2);
        let resolution = resolve(
            &base,
            &candidate_set(vec![
                transfer(0x11, first, 0x21, 0x31, 0x41),
                transfer(0x13, second, 0x23, 0x32, 0x41),
            ]),
        );

        assert_eq!(resolution.accepted_deltas().len(), 2);
        assert!(resolution.evidence().components().is_empty());
        assert_eq!(
            apply_containment_transfers(&base, resolution.accepted_deltas()),
            Ok(resolution.successor().clone())
        );
    }

    #[test]
    fn transitive_component_ranks_each_resource_with_only_its_actual_contenders() {
        let first = identity(0x64, 1);
        let bridge = identity(0x65, 2);
        let third = identity(0x66, 3);
        let candidates = [
            transfer(0x11, first, 0x21, 0x31, 0x41),
            transfer(0x12, bridge, 0x21, 0x31, 0x42),
            transfer(0x13, third, 0x23, 0x32, 0x42),
        ];
        let base = state_with_capacities(3, 1);

        let forward = resolve(&base, &candidate_set(candidates.to_vec()));
        let reversed = resolve(
            &base,
            &candidate_set(candidates.iter().rev().copied().collect()),
        );
        assert_eq!(forward, reversed);

        let [component] = forward.evidence().components() else {
            panic!("transitive conflicts must remain one connected component");
        };
        assert_eq!(component.contenders().len(), 3);
        let [item_group, capacity_group] = component.resources() else {
            panic!("transitive component must retain both binding resources");
        };
        assert_eq!(
            item_group.group().resource(),
            ContainmentConflictResourceV1::ExclusiveItem(entity(0x21))
        );
        assert_eq!(item_group.admission_limit(), 1);
        assert_eq!(
            capacity_group.group().resource(),
            ContainmentConflictResourceV1::DestinationCapacity(entity(0x42))
        );
        assert_eq!(capacity_group.admission_limit(), 1);

        let item_contenders: BTreeSet<_> = item_group
            .ranking()
            .entries()
            .iter()
            .map(|entry| ContainmentCommandIdentity::from_contender(entry.key().contender()))
            .collect();
        let capacity_contenders: BTreeSet<_> = capacity_group
            .ranking()
            .entries()
            .iter()
            .map(|entry| ContainmentCommandIdentity::from_contender(entry.key().contender()))
            .collect();
        assert_eq!(item_contenders, BTreeSet::from([first, bridge]));
        assert_eq!(capacity_contenders, BTreeSet::from([bridge, third]));
        assert!(!item_contenders.contains(&third));
        assert!(!capacity_contenders.contains(&first));
        assert_component_selection_is_feasible_and_maximal(&forward);
        assert_eq!(
            apply_containment_transfers(&base, forward.accepted_deltas()),
            Ok(forward.successor().clone())
        );
    }

    #[test]
    fn overlapping_resource_rankings_still_admit_a_maximal_feasible_set() {
        let acting = [actor(0x12), actor(0x13)];
        let item = entity(0x23);
        let source = entity(0x33);
        let destination = entity(0x43);
        let identities = acting.map(|actor| {
            let opportunity = ActionOpportunity::open(
                actor,
                ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x71; 32])),
                world_model::ActionInteractionScope::containment(
                    ContainmentInteractionScope::new(source, vec![destination], vec![item], 8)
                        .unwrap_or_else(|error| {
                            panic!("action scope fixture must be valid: {error}")
                        }),
                ),
                ActionOpportunityGeneration::new(1),
            );
            ContainmentCommandIdentity::new(
                CommandSource::derive_action(opportunity.id()),
                CommandId::new(0),
            )
        });
        let base = AcceptedState::new(
            DomainState::new(
                vec![
                    ContainerRecord::new(source, 4),
                    ContainerRecord::new(destination, 1),
                ],
                vec![ContainmentRecord::new(item, source)],
                acting
                    .into_iter()
                    .map(|actor| ContainerAuthorityRecord::new(actor, source))
                    .collect(),
            )
            .unwrap_or_else(|error| panic!("domain-state fixture must be valid: {error}")),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        );
        let candidates = [
            transfer(0x12, identities[0], 0x23, 0x33, 0x43),
            transfer(0x13, identities[1], 0x23, 0x33, 0x43),
        ];
        let oracle = Blake3KeyedPrf256V1::from_root_seed(RootSeed::from_bytes([0x61; 32]));

        let resolution = resolve_containment_candidates(
            SimMoment::new(SimTime::from_ticks(0), Microstep::new(1)),
            &base,
            &candidate_set(candidates.to_vec()),
            &oracle,
        );
        let reversed = resolve_containment_candidates(
            SimMoment::new(SimTime::from_ticks(0), Microstep::new(1)),
            &base,
            &candidate_set(candidates.into_iter().rev().collect()),
            &oracle,
        );

        assert_eq!(resolution, reversed);
        let [component] = resolution.evidence().components() else {
            panic!("shared item and destination must form one component");
        };
        let [item_group, capacity_group] = component.resources() else {
            panic!("the component must retain both binding resource rankings");
        };
        assert_ne!(
            item_group.ranking().winner(),
            capacity_group.ranking().winner()
        );
        assert_eq!(resolution.accepted_deltas().len(), 1);
        assert_eq!(
            resolution
                .successor()
                .domain()
                .containment_for(item)
                .map(|record| record.container()),
            Some(destination)
        );
        assert_component_selection_is_feasible_and_maximal(&resolution);
    }

    #[test]
    fn independent_revalidation_rejects_stale_and_actor_mismatched_proposals() {
        let valid = identity(0x71, 1);
        let stale = identity(0x72, 2);
        let mismatched = identity(0x73, 3);
        let mismatched_delta =
            ContainmentTransferDelta::new(actor(0x11), entity(0x22), entity(0x31), entity(0x42))
                .unwrap_or_else(|error| {
                    panic!("mismatched fixture delta must be structural: {error}")
                });
        let candidates = candidate_set(vec![
            transfer(0x13, valid, 0x23, 0x32, 0x42),
            transfer(0x11, stale, 0x21, 0x32, 0x41),
            ContainmentCandidate::new(
                mismatched,
                actor(0x12),
                ContainmentCandidateProposal::Transfer(mismatched_delta),
            ),
        ]);

        let resolution = resolve(&state(2), &candidates);

        assert!(is_accepted(outcome(&resolution, valid)));
        assert_eq!(
            outcome(&resolution, stale),
            &ContainmentCandidateOutcome::Rejected(StableCommandRejection::Stale)
        );
        assert_eq!(
            outcome(&resolution, mismatched),
            &ContainmentCandidateOutcome::Rejected(StableCommandRejection::BindingMismatch)
        );
        assert_eq!(resolution.accepted_deltas().len(), 1);
    }

    #[test]
    fn unrelated_random_draw_does_not_change_resolution_or_evidence() {
        let first = identity(0x81, 1);
        let second = identity(0x82, 2);
        let candidates = candidate_set(vec![
            transfer(0x11, first, 0x21, 0x31, 0x41),
            transfer(0x12, second, 0x21, 0x31, 0x42),
        ]);
        let base = state(2);
        let oracle = oracle();
        let before = resolve_containment_candidates(moment(), &base, &candidates, &oracle);

        let unrelated_group = ContainmentConflictGroupV1::new(
            SimMoment::new(SimTime::from_ticks(99), Microstep::new(7)),
            ContainmentConflictResourceV1::DestinationCapacity(entity(0x77)),
        );
        rank_containment_conflict(
            &oracle,
            unrelated_group,
            &[
                identity(0x91, 91).contender(actor(0x11)),
                identity(0x92, 92).contender(actor(0x13)),
            ],
        )
        .unwrap_or_else(|error| panic!("unrelated draw fixture must rank: {error:?}"));

        let after = resolve_containment_candidates(moment(), &base, &candidates, &oracle);
        assert_eq!(before, after);
    }

    #[test]
    fn random_evidence_failure_has_a_total_rejection_only_fallback() {
        let first = identity(0xa1, 1);
        let second = identity(0xa2, 2);
        let pre_rejected = identity(0xa3, 3);
        let candidates = candidate_set(vec![
            transfer(0x11, first, 0x21, 0x31, 0x41),
            transfer(0x12, second, 0x21, 0x31, 0x42),
            ContainmentCandidate::new(
                pre_rejected,
                actor(0x13),
                ContainmentCandidateProposal::Rejected(StableCommandRejection::Stale),
            ),
        ]);
        let base = state(2);

        let resolution =
            resolve_containment_candidates_with(moment(), &base, &candidates, |_, _| {
                Err(Box::new(ContainmentRandomRankError::EmptyConflictGroup))
            });

        assert!(resolution.accepted_deltas().is_empty());
        assert_eq!(resolution.successor(), &base);
        assert_eq!(
            outcome(&resolution, first),
            &ContainmentCandidateOutcome::Rejected(StableCommandRejection::Conflict)
        );
        assert_eq!(
            outcome(&resolution, second),
            &ContainmentCandidateOutcome::Rejected(StableCommandRejection::Conflict)
        );
        assert_eq!(
            outcome(&resolution, pre_rejected),
            &ContainmentCandidateOutcome::Rejected(StableCommandRejection::Stale)
        );
        assert!(matches!(
            resolution.evidence().fallback(),
            Some(ContainmentResolutionFallback::RandomEvidence {
                error: ContainmentRandomRankError::EmptyConflictGroup,
                ..
            })
        ));
    }

    #[test]
    fn source_scoped_command_identity_rejects_a_cross_actor_duplicate() {
        let duplicate = identity(0xb1, 1);
        assert!(matches!(
            ContainmentCandidateSet::new(vec![
                transfer(0x11, duplicate, 0x21, 0x31, 0x41),
                transfer(0x12, duplicate, 0x22, 0x31, 0x42),
            ]),
            Err(ContainmentCandidateSetError::DuplicateCommand { identity })
                if identity == duplicate
        ));
    }
}
