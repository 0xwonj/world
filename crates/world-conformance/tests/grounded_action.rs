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
    PhysicalEvent, ResolvedExecution, RootSeed, RuntimeService, SimMoment, SimTime, SocialState,
    TerminationContractV1, baseline_lifecycle_profiles,
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
    command_count: usize,
    post_commit_consumed: usize,
    action_opportunities_consumed: Vec<ActionOpportunityId>,
    attempt_resolved: Vec<ActionOpportunityId>,
}

#[derive(Debug, PartialEq, Eq)]
struct CausalStep {
    item_container: Option<EntityId>,
    evidence_count: usize,
    intent_count: usize,
    activity_count: usize,
}

fn valid<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("grounded-action conformance fixture must be valid: {error:?}"),
    }
}

fn accepted_domain(
    containers: Vec<ContainerRecord>,
    containment: Vec<ContainmentRecord>,
    authority: Vec<ContainerAuthorityRecord>,
    beliefs: Vec<(ActorId, EntityId, EntityId)>,
) -> AcceptedState {
    let mut epistemic = EpistemicState::empty();
    for (observer, item, container) in beliefs {
        let prior = if container == entity(0xf0) {
            entity(0xf1)
        } else {
            entity(0xf0)
        };
        let delta = valid(ContainmentTransferDelta::new(
            observer, item, prior, container,
        ));
        let PhysicalEvent::ItemTransferred(event) = PhysicalEvent::item_transferred(delta) else {
            panic!("containment belief fixture must produce an item-transfer event")
        };
        let evidence = EvidenceRecord::direct_item_transfer(
            observer,
            EvidenceDeliveryGeneration::new(1)
                .unwrap_or_else(|| panic!("fixture evidence generation is nonzero")),
            event,
        );
        epistemic = valid(epistemic.assimilate(observer, EpistemicVersion::EMPTY, vec![evidence]));
    }
    AcceptedState::new(
        valid(DomainState::new(containers, containment, authority)),
        epistemic,
        SocialState::empty(),
        AgencyState::empty(),
    )
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
        SourceSnapshotId::from_bytes([0x53; 32]),
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
    opportunities: Vec<ActionOpportunity>,
    profiles: LifecycleProfilesV2,
) -> ResolvedExecution {
    valid(engine.resolve_execution(ExecutionSpecInput::origin(
        compilation.definitions().lock().clone(),
        ExecutionOrigin::new(
            accepted,
            opportunities,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
        ),
        profiles,
        valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
        RootSeed::from_bytes([0x61; 32]),
        TerminationContractV1::never(),
    )))
}

fn opportunity(
    acting: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
    generation: u64,
) -> ActionOpportunity {
    ActionOpportunity::open(
        acting,
        ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x71; 32])),
        ActionInteractionScope::containment(valid(ContainmentInteractionScope::new(
            source,
            vec![destination],
            vec![item],
            8,
        ))),
        ActionOpportunityGeneration::new(generation),
    )
}

fn hidden_capacity_state(
    acting: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
    hidden_destination_item: Option<EntityId>,
) -> AcceptedState {
    let mut containment = vec![ContainmentRecord::new(item, source)];
    if let Some(hidden) = hidden_destination_item {
        containment.push(ContainmentRecord::new(hidden, destination));
    }
    accepted_domain(
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 1),
        ],
        containment,
        vec![ContainerAuthorityRecord::new(acting, source)],
        vec![(acting, item, source)],
    )
}

fn shared_item_state(
    first_actor: ActorId,
    second_actor: ActorId,
    item: EntityId,
    item_container: EntityId,
    source: EntityId,
    destination: EntityId,
) -> AcceptedState {
    accepted_domain(
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 1),
        ],
        vec![ContainmentRecord::new(item, item_container)],
        vec![
            ContainerAuthorityRecord::new(first_actor, source),
            ContainerAuthorityRecord::new(second_actor, source),
        ],
        vec![(first_actor, item, source), (second_actor, item, source)],
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
            command_count: commands.len(),
            post_commit_consumed,
            action_opportunities_consumed,
            attempt_resolved,
        },
        other => panic!("grounded-action step must publish, found {other:?}"),
    }
}

fn sorted_opportunities(mut opportunities: Vec<ActionOpportunityId>) -> Vec<ActionOpportunityId> {
    opportunities.sort_unstable();
    opportunities
}

#[test]
fn committed_transfer_drives_evidence_intent_activity_and_a_restoring_action() {
    let policy = Arc::new(RecordingPolicy::new());
    let (distribution, profiles) = distribution_with_policy(policy.clone());
    let compilation = compile_standard(&distribution);
    let engine = engine(distribution, &compilation);

    let acting = actor(0x14);
    let item = entity(0x24);
    let home = entity(0x34);
    let displaced = entity(0x44);
    let initial_opportunity = opportunity(acting, item, home, displaced, 1);
    let initial = accepted_domain(
        vec![
            ContainerRecord::new(home, 4),
            ContainerRecord::new(displaced, 4),
        ],
        vec![ContainmentRecord::new(item, home)],
        vec![
            ContainerAuthorityRecord::new(acting, home),
            ContainerAuthorityRecord::new(acting, displaced),
        ],
        vec![(acting, item, home)],
    );
    let execution = resolve_origin(
        &engine,
        &compilation,
        initial,
        vec![initial_opportunity],
        profiles,
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x84; 32])));

    let through = moment(64);
    let mut published_moments = Vec::new();
    let mut causal_steps = Vec::new();
    let mut post_commit_consumed = 0;
    let mut neutral_attempts = 0;
    for _ in 0..64 {
        match valid(attempt.advance(AdvanceRequest::through(through))) {
            AdvanceOutcome::Published {
                moment,
                post_commit_consumed: consumed,
                attempt_resolved,
                ..
            } => {
                let snapshot = valid(attempt.session().snapshot());
                let accepted = snapshot.accepted();
                published_moments.push(moment);
                causal_steps.push(CausalStep {
                    item_container: accepted
                        .domain()
                        .containment_for(item)
                        .map(|record| record.container()),
                    evidence_count: accepted.epistemic().evidence().len(),
                    intent_count: accepted.agency().intents().len(),
                    activity_count: accepted.agency().activities().len(),
                });
                post_commit_consumed += consumed;
                neutral_attempts += attempt_resolved.len();
            }
            AdvanceOutcome::NoScheduledWork => break,
            other => panic!("lifecycle vertical must reach quiescence, found {other:?}"),
        }
    }

    assert_eq!(
        published_moments,
        (0..=13).map(moment).collect::<Vec<_>>(),
        "the complete causal chain must advance exactly one microstep per successor"
    );
    assert_eq!(causal_steps[0].item_container, Some(home));
    assert_eq!(causal_steps[1].item_container, Some(displaced));
    assert!(
        causal_steps[1..8]
            .iter()
            .all(|step| step.item_container == Some(displaced))
    );
    assert_eq!(causal_steps[8].item_container, Some(home));
    assert!(
        causal_steps[..3]
            .iter()
            .all(|step| step.evidence_count == 1),
        "the physical commit and post-commit dispatch must precede new actor evidence"
    );
    assert!(
        causal_steps[3..10]
            .iter()
            .all(|step| step.evidence_count == 2)
    );
    assert!(
        causal_steps[10..]
            .iter()
            .all(|step| step.evidence_count == 3)
    );
    assert!(
        causal_steps[..5].iter().all(|step| step.intent_count == 0),
        "accepted evidence and appraisal must precede intent adoption"
    );
    assert_eq!(causal_steps[5].intent_count, 1);
    assert!(
        causal_steps[..6]
            .iter()
            .all(|step| step.activity_count == 0),
        "intent adoption must precede activity initialization"
    );
    assert_eq!(causal_steps[6].activity_count, 1);
    assert_eq!(post_commit_consumed, 2);
    assert_eq!(neutral_attempts, 2);
    assert_eq!(policy.invocations().len(), 2);

    let snapshot = valid(attempt.session().snapshot());
    assert_eq!(
        snapshot
            .accepted()
            .domain()
            .containment_for(item)
            .map(|record| record.container()),
        Some(home)
    );
    assert_eq!(
        snapshot
            .accepted()
            .epistemic()
            .contained_in(acting, item)
            .map(|belief| belief.container()),
        Some(home)
    );
    let agency = snapshot.accepted().agency();
    let [intent] = agency.intents() else {
        panic!("the displacement must adopt exactly one persistent intent");
    };
    let [activity] = agency.activities() else {
        panic!("the intent must start exactly one persistent activity");
    };
    assert!(
        intent.status().is_terminal(),
        "the restored condition must explicitly close its intent"
    );
    assert!(
        activity.status().is_terminal(),
        "the successful restoring action must explicitly close its activity"
    );
}

#[test]
fn hidden_destination_capacity_does_not_change_actor_input_or_reveal_action_result() {
    let open_policy = Arc::new(RecordingPolicy::new());
    let full_policy = Arc::new(RecordingPolicy::new());
    let (open_distribution, open_profiles) = distribution_with_policy(open_policy.clone());
    let compilation = compile_standard(&open_distribution);
    let (full_distribution, full_profiles) = distribution_with_policy(full_policy.clone());
    let open_engine = engine(open_distribution, &compilation);
    let full_engine = engine(full_distribution, &compilation);

    let acting = actor(0x11);
    let item = entity(0x21);
    let hidden_item = entity(0x22);
    let source = entity(0x31);
    let destination = entity(0x41);
    let opportunity = opportunity(acting, item, source, destination, 1);
    let opportunity_id = opportunity.id();

    let open_initial = hidden_capacity_state(acting, item, source, destination, None);
    let full_initial = hidden_capacity_state(acting, item, source, destination, Some(hidden_item));
    let open_execution = resolve_origin(
        &open_engine,
        &compilation,
        open_initial.clone(),
        vec![opportunity.clone()],
        open_profiles,
    );
    let full_execution = resolve_origin(
        &full_engine,
        &compilation,
        full_initial.clone(),
        vec![opportunity],
        full_profiles,
    );
    let mut open_attempt =
        valid(open_engine.start_attempt(&open_execution, AttemptKey::from_bytes([0x81; 32])));
    let mut full_attempt =
        valid(full_engine.start_attempt(&full_execution, AttemptKey::from_bytes([0x82; 32])));
    let through = moment(2);

    let open_ready = published(valid(
        open_attempt.advance(AdvanceRequest::through(through)),
    ));
    let full_ready = published(valid(
        full_attempt.advance(AdvanceRequest::through(through)),
    ));
    assert_eq!(open_ready.moment, SimMoment::ORIGIN);
    assert_eq!(full_ready.moment, SimMoment::ORIGIN);
    assert_eq!(
        open_ready.action_opportunities_consumed,
        vec![opportunity_id]
    );
    assert_eq!(
        full_ready.action_opportunities_consumed,
        vec![opportunity_id]
    );
    assert!(open_ready.attempt_resolved.is_empty());
    assert!(full_ready.attempt_resolved.is_empty());
    assert_eq!(open_ready.command_count, 0);
    assert_eq!(full_ready.command_count, 0);
    assert_eq!(open_ready.post_commit_consumed, 0);
    assert_eq!(full_ready.post_commit_consumed, 0);
    assert_eq!(
        valid(open_attempt.session().snapshot()).accepted(),
        &open_initial
    );
    assert_eq!(
        valid(full_attempt.session().snapshot()).accepted(),
        &full_initial
    );

    let open_invocations = open_policy.invocations();
    let full_invocations = full_policy.invocations();
    let [open_invocation] = open_invocations.as_slice() else {
        panic!("open execution must invoke its action policy exactly once");
    };
    let [full_invocation] = full_invocations.as_slice() else {
        panic!("full execution must invoke its action policy exactly once");
    };
    assert_eq!(open_invocation.input, full_invocation.input);
    assert_eq!(open_invocation.decision, full_invocation.decision);
    assert_eq!(open_invocation.input.opportunity(), opportunity_id);
    assert_eq!(open_invocation.input.candidates().candidates().len(), 1);
    assert_eq!(
        open_invocation.decision.selected_candidate(),
        open_invocation
            .input
            .candidates()
            .candidates()
            .first()
            .map(|candidate| candidate.id())
    );
    assert_eq!(
        open_invocation
            .input
            .candidates()
            .candidates()
            .iter()
            .map(|candidate| candidate.id())
            .collect::<Vec<_>>(),
        full_invocation
            .input
            .candidates()
            .candidates()
            .iter()
            .map(|candidate| candidate.id())
            .collect::<Vec<_>>()
    );

    let open_command = published(valid(
        open_attempt.advance(AdvanceRequest::through(through)),
    ));
    let full_command = published(valid(
        full_attempt.advance(AdvanceRequest::through(through)),
    ));
    assert_eq!(open_command.moment, moment(1));
    assert_eq!(full_command.moment, moment(1));
    assert_eq!(open_command.command_count, 0);
    assert_eq!(full_command.command_count, 0);
    assert!(open_command.action_opportunities_consumed.is_empty());
    assert!(full_command.action_opportunities_consumed.is_empty());
    assert!(open_command.attempt_resolved.is_empty());
    assert!(full_command.attempt_resolved.is_empty());
    assert_eq!(open_policy.invocations().len(), 1);
    assert_eq!(full_policy.invocations().len(), 1);

    let open_after_command = valid(open_attempt.session().snapshot());
    let full_after_command = valid(full_attempt.session().snapshot());
    assert_eq!(
        open_after_command
            .accepted()
            .domain()
            .containment_for(item)
            .map(|record| record.container()),
        Some(destination)
    );
    assert_eq!(
        full_after_command
            .accepted()
            .domain()
            .containment_for(item)
            .map(|record| record.container()),
        Some(source)
    );

    let open_wake = published(valid(
        open_attempt.advance(AdvanceRequest::through(through)),
    ));
    let full_wake = published(valid(
        full_attempt.advance(AdvanceRequest::through(through)),
    ));
    assert_eq!(open_wake.moment, moment(2));
    assert_eq!(full_wake.moment, moment(2));
    assert_eq!(open_wake.command_count, 0);
    assert_eq!(full_wake.command_count, 0);
    assert!(open_wake.action_opportunities_consumed.is_empty());
    assert!(full_wake.action_opportunities_consumed.is_empty());
    assert_eq!(open_wake.attempt_resolved, vec![opportunity_id]);
    assert_eq!(full_wake.attempt_resolved, vec![opportunity_id]);
    assert_eq!(open_wake.post_commit_consumed, 1);
    assert_eq!(full_wake.post_commit_consumed, 0);
    assert_eq!(open_policy.invocations().len(), 1);
    assert_eq!(full_policy.invocations().len(), 1);
}

#[test]
fn same_moment_actor_actions_resolve_one_physical_transfer_and_two_neutral_wakes() {
    let policy = Arc::new(RecordingPolicy::new());
    let (distribution, profiles) = distribution_with_policy(policy.clone());
    let compilation = compile_standard(&distribution);
    let engine = engine(distribution, &compilation);

    let first_actor = actor(0x12);
    let second_actor = actor(0x13);
    let item = entity(0x23);
    let source = entity(0x33);
    let destination = entity(0x43);
    let first_opportunity = opportunity(first_actor, item, source, destination, 1);
    let second_opportunity = opportunity(second_actor, item, source, destination, 1);
    let expected_opportunities =
        sorted_opportunities(vec![first_opportunity.id(), second_opportunity.id()]);
    let initial = shared_item_state(first_actor, second_actor, item, source, source, destination);
    let expected = shared_item_state(
        first_actor,
        second_actor,
        item,
        destination,
        source,
        destination,
    );
    let execution = resolve_origin(
        &engine,
        &compilation,
        initial.clone(),
        vec![second_opportunity, first_opportunity],
        profiles,
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x83; 32])));
    let through = moment(2);

    let ready = published(valid(attempt.advance(AdvanceRequest::through(through))));
    assert_eq!(ready.moment, SimMoment::ORIGIN);
    assert_eq!(ready.command_count, 0);
    assert_eq!(ready.post_commit_consumed, 0);
    assert_eq!(ready.action_opportunities_consumed, expected_opportunities);
    assert!(ready.attempt_resolved.is_empty());
    assert_eq!(valid(attempt.session().snapshot()).accepted(), &initial);

    let invocations = policy.invocations();
    assert_eq!(invocations.len(), 2);
    assert_eq!(
        sorted_opportunities(
            invocations
                .iter()
                .map(|invocation| invocation.input.opportunity())
                .collect()
        ),
        expected_opportunities
    );
    assert!(invocations.iter().all(|invocation| {
        let candidates = invocation.input.candidates().candidates();
        candidates.len() == 1
            && invocation.decision.selected_candidate()
                == candidates.first().map(|candidate| candidate.id())
    }));

    let command = published(valid(attempt.advance(AdvanceRequest::through(through))));
    assert_eq!(command.moment, moment(1));
    assert_eq!(command.command_count, 0);
    assert_eq!(command.post_commit_consumed, 0);
    assert!(command.action_opportunities_consumed.is_empty());
    assert!(command.attempt_resolved.is_empty());
    assert_eq!(policy.invocations().len(), 2);
    assert_eq!(valid(attempt.session().snapshot()).accepted(), &expected);

    let wakes = published(valid(attempt.advance(AdvanceRequest::through(through))));
    assert_eq!(wakes.moment, moment(2));
    assert_eq!(wakes.command_count, 0);
    assert_eq!(wakes.post_commit_consumed, 1);
    assert!(wakes.action_opportunities_consumed.is_empty());
    assert_eq!(wakes.attempt_resolved, expected_opportunities);
    assert_eq!(policy.invocations().len(), 2);
    assert_eq!(valid(attempt.session().snapshot()).accepted(), &expected);
    assert_eq!(
        valid(attempt.advance(AdvanceRequest::through(through))),
        AdvanceOutcome::NoWorkDue {
            next: moment(3),
            through
        }
    );
}
