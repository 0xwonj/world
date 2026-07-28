//! Public false-belief conformance through the engine facade.

use std::sync::{Arc, Mutex};

use world_authoring::{
    AuthoringCompiler, Compilation, CompileRequest, PackSource, SourceSnapshotId,
};
use world_engine::{
    AcceptedState, ActionContextPayload, ActionDecision, ActionInteractionScope, ActionOpportunity,
    ActionOpportunityGeneration, ActionOpportunityId, ActionPolicy, ActionPolicyError,
    ActionPolicyInstallation, ActionSponsor, ActorId, ActorReactionCause, AdvanceOutcome,
    AdvanceRequest, AgencyState, ArtifactEnvelope, ArtifactResolveError, ArtifactResolver,
    AttemptKey, BaselineActionPolicy, ContainerAuthorityRecord, ContainerRecord,
    ContainmentInteractionScope, ContainmentRecord, ContainmentTransferDelta, DomainState, Engine,
    EngineBuilder, EngineDistribution, EntityId, EpistemicState, EpistemicVersion,
    EvidenceDeliveryGeneration, EvidenceRecord, ExecutionConfigArtifactV3, ExecutionOrigin,
    ExecutionSpecInput, LifecycleImplementationSet, LifecycleProfilesV2, Microstep, PackLockEntry,
    PhysicalEvent, ResolvedCommandDelivery, ResolvedExecution, RootSeed, RuntimeService, SimMoment,
    SimTime, SocialState, TerminationContractV1, baseline_lifecycle_profiles,
};
use world_standard::transfer_artifact_data;
use world_standard_runtime::standard_transfer_implementation;

#[derive(Clone)]
struct InMemoryArtifacts {
    envelopes: Vec<ArtifactEnvelope>,
}

impl ArtifactResolver for InMemoryArtifacts {
    fn resolve(&self, reference: &PackLockEntry) -> Result<ArtifactEnvelope, ArtifactResolveError> {
        self.envelopes
            .iter()
            .find(|envelope| {
                envelope.descriptor().blob_digest() == reference.artifact_digest()
                    && envelope.descriptor().format_version() == reference.artifact_format_version()
                    && envelope.descriptor().blob_length() == reference.artifact_byte_length()
            })
            .cloned()
            .ok_or(ArtifactResolveError::NotFound)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PolicyInvocation {
    input: ActionContextPayload,
    decision: ActionDecision,
}

struct RecordingPolicy {
    baseline: BaselineActionPolicy,
    invocations: Mutex<Vec<PolicyInvocation>>,
}

impl RecordingPolicy {
    fn new() -> Self {
        Self {
            baseline: BaselineActionPolicy::new(),
            invocations: Mutex::new(Vec::new()),
        }
    }

    fn invocations(&self) -> Vec<PolicyInvocation> {
        self.invocations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl ActionPolicy for RecordingPolicy {
    fn semantics_id(&self) -> world_engine::ActionPolicySemanticsId {
        self.baseline.semantics_id()
    }

    fn decide(&self, input: &ActionContextPayload) -> Result<ActionDecision, ActionPolicyError> {
        let decision = self.baseline.decide(input)?;
        self.invocations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(PolicyInvocation {
                input: input.clone(),
                decision,
            });
        Ok(decision)
    }
}

#[derive(Debug)]
struct PublishedStep {
    moment: SimMoment,
    commands: Vec<ResolvedCommandDelivery>,
    post_commit_consumed: usize,
    action_opportunities_consumed: Vec<ActionOpportunityId>,
    attempt_resolved: Vec<ActionOpportunityId>,
}

fn valid<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("false-belief conformance fixture must be valid: {error:?}"),
    }
}

fn actor(byte: u8) -> ActorId {
    ActorId::from_bytes([byte; 32])
}

fn entity(byte: u8) -> EntityId {
    EntityId::from_bytes([byte; 32])
}

fn moment(microstep: u64) -> SimMoment {
    SimMoment::new(SimTime::from_ticks(0), Microstep::new(microstep))
}

fn distribution_with_policy(
    policy: Arc<dyn ActionPolicy>,
) -> (EngineDistribution, LifecycleProfilesV2) {
    let action = ActionPolicyInstallation::inline_deterministic(policy);
    let profiles = baseline_lifecycle_profiles(action.binding());
    let lifecycle = valid(LifecycleImplementationSet::baseline(vec![action]));
    (
        valid(EngineDistribution::new(
            vec![standard_transfer_implementation()],
            Vec::new(),
            lifecycle,
        )),
        profiles,
    )
}

fn compile_standard(distribution: &EngineDistribution) -> Compilation {
    let data = transfer_artifact_data();
    let source = PackSource::new(
        SourceSnapshotId::from_bytes([0x56; 32]),
        distribution.engine_protocol(),
        data.manifest().coordinate().clone(),
        data.manifest()
            .dependencies()
            .iter()
            .map(|dependency| dependency.coordinate().clone())
            .collect(),
        data.actions().to_vec(),
        data.events().to_vec(),
    );
    valid(
        AuthoringCompiler::new(distribution.semantic_interfaces()).compile(CompileRequest::new(
            source.coordinate().clone(),
            vec![source],
        )),
    )
}

fn engine(distribution: EngineDistribution, compilation: &Compilation) -> Engine {
    let artifacts: Arc<dyn ArtifactResolver> = Arc::new(InMemoryArtifacts {
        envelopes: compilation.envelopes().to_vec(),
    });
    valid(EngineBuilder::new(distribution, artifacts, valid(RuntimeService::in_memory())).build())
}

fn resolve_origin(
    engine: &Engine,
    compilation: &Compilation,
    accepted: AcceptedState,
    opportunity: ActionOpportunity,
    profiles: LifecycleProfilesV2,
) -> ResolvedExecution {
    valid(engine.resolve_execution(ExecutionSpecInput::origin(
        compilation.definitions().lock().clone(),
        ExecutionOrigin::new(
            accepted,
            vec![opportunity],
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
        ),
        profiles,
        valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
        RootSeed::from_bytes([0x66; 32]),
        TerminationContractV1::never(),
    )))
}

fn false_belief_state(
    acting: ActorId,
    item: EntityId,
    actual_container: EntityId,
    hidden_containers: [EntityId; 2],
    believed_container: EntityId,
    belief_prior: EntityId,
    destination: EntityId,
) -> AcceptedState {
    let domain = valid(DomainState::new(
        vec![
            ContainerRecord::new(hidden_containers[0], 2),
            ContainerRecord::new(hidden_containers[1], 2),
            ContainerRecord::new(believed_container, 2),
            ContainerRecord::new(belief_prior, 2),
            ContainerRecord::new(destination, 2),
        ],
        vec![ContainmentRecord::new(item, actual_container)],
        vec![ContainerAuthorityRecord::new(acting, believed_container)],
    ));
    let delta = valid(ContainmentTransferDelta::new(
        acting,
        item,
        belief_prior,
        believed_container,
    ));
    let PhysicalEvent::ItemTransferred(event) = PhysicalEvent::item_transferred(delta) else {
        panic!("the accepted false belief must use containment evidence")
    };
    let evidence = EvidenceRecord::direct_item_transfer(
        acting,
        EvidenceDeliveryGeneration::new(1)
            .unwrap_or_else(|| panic!("fixture evidence generation is nonzero")),
        event,
    );
    let epistemic =
        valid(EpistemicState::empty().assimilate(acting, EpistemicVersion::EMPTY, vec![evidence]));
    AcceptedState::new(
        domain,
        epistemic,
        SocialState::empty(),
        AgencyState::empty(),
    )
}

fn transfer_opportunity(
    acting: ActorId,
    item: EntityId,
    believed_container: EntityId,
    destination: EntityId,
) -> ActionOpportunity {
    ActionOpportunity::open(
        acting,
        ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x76; 32])),
        ActionInteractionScope::containment(valid(ContainmentInteractionScope::new(
            believed_container,
            vec![destination],
            vec![item],
            1,
        ))),
        ActionOpportunityGeneration::new(1),
    )
}

fn published(outcome: AdvanceOutcome) -> PublishedStep {
    match outcome {
        AdvanceOutcome::Published {
            moment,
            commands,
            post_commit_consumed,
            action_opportunities_consumed,
            attempt_resolved,
            ..
        } => PublishedStep {
            moment,
            commands,
            post_commit_consumed,
            action_opportunities_consumed,
            attempt_resolved,
        },
        other => panic!("false-belief step must publish, found {other:?}"),
    }
}

#[test]
fn rejected_false_belief_retracts_only_after_modeled_absence_reaches_appraisal() {
    let first_policy = Arc::new(RecordingPolicy::new());
    let second_policy = Arc::new(RecordingPolicy::new());
    let (first_distribution, first_profiles) = distribution_with_policy(first_policy.clone());
    let compilation = compile_standard(&first_distribution);
    let (second_distribution, second_profiles) = distribution_with_policy(second_policy.clone());
    let first_engine = engine(first_distribution, &compilation);
    let second_engine = engine(second_distribution, &compilation);

    let acting = actor(0x11);
    let item = entity(0x21);
    let first_hidden = entity(0x31);
    let second_hidden = entity(0x32);
    let believed = entity(0x41);
    let belief_prior = entity(0x42);
    let destination = entity(0x51);
    let hidden_containers = [first_hidden, second_hidden];
    let opportunity = transfer_opportunity(acting, item, believed, destination);
    let opportunity_id = opportunity.id();
    let first_initial = false_belief_state(
        acting,
        item,
        first_hidden,
        hidden_containers,
        believed,
        belief_prior,
        destination,
    );
    let second_initial = false_belief_state(
        acting,
        item,
        second_hidden,
        hidden_containers,
        believed,
        belief_prior,
        destination,
    );
    let first_execution = resolve_origin(
        &first_engine,
        &compilation,
        first_initial,
        opportunity.clone(),
        first_profiles,
    );
    let second_execution = resolve_origin(
        &second_engine,
        &compilation,
        second_initial,
        opportunity,
        second_profiles,
    );
    let mut first_attempt =
        valid(first_engine.start_attempt(&first_execution, AttemptKey::from_bytes([0x81; 32])));
    let mut second_attempt =
        valid(second_engine.start_attempt(&second_execution, AttemptKey::from_bytes([0x82; 32])));
    let through = moment(32);

    let first_ready = published(valid(
        first_attempt.advance(AdvanceRequest::through(through)),
    ));
    let second_ready = published(valid(
        second_attempt.advance(AdvanceRequest::through(through)),
    ));
    assert_eq!(first_ready.moment, SimMoment::ORIGIN);
    assert_eq!(second_ready.moment, SimMoment::ORIGIN);
    assert_eq!(
        first_ready.action_opportunities_consumed,
        vec![opportunity_id]
    );
    assert_eq!(
        second_ready.action_opportunities_consumed,
        vec![opportunity_id]
    );
    assert!(first_ready.commands.is_empty());
    assert!(second_ready.commands.is_empty());
    assert_eq!(first_policy.invocations(), second_policy.invocations());
    assert_eq!(first_policy.invocations().len(), 1);

    let first_rejection = published(valid(
        first_attempt.advance(AdvanceRequest::through(through)),
    ));
    let second_rejection = published(valid(
        second_attempt.advance(AdvanceRequest::through(through)),
    ));
    assert_eq!(first_rejection.moment, moment(1));
    assert_eq!(second_rejection.moment, moment(1));
    assert!(
        first_rejection.commands.is_empty(),
        "actor-derived resolution must stay behind the neutral actor-control boundary"
    );
    assert!(second_rejection.commands.is_empty());
    assert_eq!(
        first_rejection.post_commit_consumed,
        second_rejection.post_commit_consumed
    );
    assert_eq!(
        first_rejection.attempt_resolved,
        second_rejection.attempt_resolved
    );

    for (attempt, hidden) in [
        (&first_attempt, first_hidden),
        (&second_attempt, second_hidden),
    ] {
        let snapshot = valid(attempt.session().snapshot());
        assert_eq!(
            snapshot
                .accepted()
                .domain()
                .containment_for(item)
                .map(|record| record.container()),
            Some(hidden)
        );
        assert_eq!(
            snapshot
                .accepted()
                .epistemic()
                .contained_in(acting, item)
                .map(|belief| belief.container()),
            Some(believed),
            "runtime rejection must not directly install authoritative truth"
        );
        assert_eq!(
            snapshot.accepted().epistemic().actor_version(acting),
            EpistemicVersion::new(1)
        );
        assert_eq!(snapshot.accepted().epistemic().evidence().len(), 1);
    }

    let expected_absence = EvidenceRecord::direct_item_absent(
        acting,
        EvidenceDeliveryGeneration::new(9)
            .unwrap_or_else(|| panic!("fixture evidence generation is nonzero")),
        item,
        believed,
    )
    .provenance();
    let mut published_moments = Vec::new();
    let mut retraction_moment = None;
    let mut quiesced = false;

    for _ in 0..32 {
        let first = valid(first_attempt.advance(AdvanceRequest::through(through)));
        let second = valid(second_attempt.advance(AdvanceRequest::through(through)));
        match (first, second) {
            (
                AdvanceOutcome::Published {
                    moment: first_moment,
                    commands: first_commands,
                    post_commit_consumed: first_post_commit,
                    action_opportunities_consumed: first_consumed,
                    attempt_resolved: first_resolved,
                    ..
                },
                AdvanceOutcome::Published {
                    moment: second_moment,
                    commands: second_commands,
                    post_commit_consumed: second_post_commit,
                    action_opportunities_consumed: second_consumed,
                    attempt_resolved: second_resolved,
                    ..
                },
            ) => {
                assert_eq!(first_moment, second_moment);
                assert_eq!(first_commands, second_commands);
                assert_eq!(first_post_commit, second_post_commit);
                assert_eq!(first_consumed, second_consumed);
                assert_eq!(first_resolved, second_resolved);
                published_moments.push(first_moment);

                let first_snapshot = valid(first_attempt.session().snapshot());
                let second_snapshot = valid(second_attempt.session().snapshot());
                assert_eq!(
                    first_snapshot.accepted().epistemic(),
                    second_snapshot.accepted().epistemic(),
                    "hidden authoritative location must not alter actor evidence or belief"
                );
                let first_belief = first_snapshot
                    .accepted()
                    .epistemic()
                    .contained_in(acting, item);
                let second_belief = second_snapshot
                    .accepted()
                    .epistemic()
                    .contained_in(acting, item);
                assert_eq!(first_belief, second_belief);
                if first_belief.is_none() && retraction_moment.is_none() {
                    retraction_moment = Some(first_moment);
                }
                for snapshot in [&first_snapshot, &second_snapshot] {
                    let epistemic = snapshot.accepted().epistemic();
                    if first_belief.is_none() {
                        assert_eq!(epistemic.actor_version(acting), EpistemicVersion::new(2));
                        assert_eq!(epistemic.evidence().len(), 2);
                        assert!(
                            epistemic
                                .evidence()
                                .iter()
                                .any(|record| record.provenance() == expected_absence)
                        );
                    }
                    assert!(snapshot.accepted().agency().intents().is_empty());
                    assert!(snapshot.accepted().agency().activities().is_empty());
                }
            }
            (AdvanceOutcome::NoScheduledWork, AdvanceOutcome::NoScheduledWork) => {
                quiesced = true;
                break;
            }
            (first, second) => {
                panic!("paired false-belief executions diverged: {first:?} versus {second:?}")
            }
        }
    }

    assert!(quiesced);
    let retraction_moment =
        retraction_moment.unwrap_or_else(|| panic!("modeled absence must retract the belief"));
    assert!(retraction_moment > first_rejection.moment);
    assert!(
        published_moments
            .iter()
            .any(|published| *published > retraction_moment),
        "appraisal must consume a later scheduled moment after belief retraction"
    );
    assert_eq!(first_policy.invocations().len(), 1);
    assert_eq!(second_policy.invocations().len(), 1);
}
