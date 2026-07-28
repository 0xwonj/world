//! Public relocation conformance through the engine facade.
//!
//! Origins may supply modeled actor opportunities, while successor controls
//! are opened only by accepted activity progression. The host may evaluate a
//! retained actor decision later, but cannot inject pause or resume controls.

use std::sync::{Arc, Mutex};

use world_authoring::{
    AuthoringCompiler, Compilation, CompileRequest, PackSource, SourceSnapshotId,
};
use world_engine::{
    AcceptedState, ActionContextPayload, ActionDecision, ActionEvaluationCaptureId,
    ActionEvaluationResultCapture, ActionInteractionScope, ActionOpportunity,
    ActionOpportunityGeneration, ActionOpportunityId, ActionPolicy, ActionPolicyError,
    ActionPolicyInstallation, ActionSponsor, Activity, ActivityControllerId, ActivityFocus,
    ActivityGeneration, ActivityStatus, ActorId, ActorLocation, ActorPosition, ActorReactionCause,
    AdvanceOutcome, AdvanceRequest, AgencyState, ArtifactEnvelope, ArtifactResolveError,
    ArtifactResolver, AttemptKey, BaselineActionPolicy, BaselineActivityController,
    ContainerAuthorityRecord, ContainerRecord, ContainmentInteractionScope, ContainmentRecord,
    ContainmentTransferDelta, DeferredActionAdmissionModeV1, DeferredActionControlV1,
    DeferredActionEvaluatorDescriptor, DesiredCondition, DirectedRoute, DomainState, Engine,
    EngineBuilder, EngineDistribution, EntityId, EpistemicState, EpistemicVersion,
    EvidenceDeliveryGeneration, EvidenceRecord, ExecutionConfigArtifactV3, ExecutionOrigin,
    ExecutionSpecInput, Intent, IntentGeneration, LifecycleImplementationSet, LifecycleProfilesV2,
    Microstep, PackLockEntry, PhysicalEvent, RelocationActionVerb, RelocationInteraction,
    RelocationInteractionAnchor, RelocationInteractionScope, ResolvedExecution, RootSeed,
    RunAttempt, RuntimeService, SimDuration, SimMoment, SimTime, SocialState,
    TerminationContractV1, TravelActivityState, TravelActivityStep, activity_state_schema,
    baseline_lifecycle_profiles,
};
use world_standard::{relocation_artifact_data, transfer_artifact_data};
use world_standard_runtime::{
    standard_relocation_implementation, standard_transfer_implementation,
};

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

#[derive(Clone, Debug)]
struct PublishedStep {
    moment: SimMoment,
    action_opportunities_consumed: Vec<ActionOpportunityId>,
    attempt_resolved: Vec<ActionOpportunityId>,
}

fn valid<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("relocation conformance fixture must be valid: {error:?}"),
    }
}

fn actor(byte: u8) -> ActorId {
    ActorId::from_bytes([byte; 32])
}

fn entity(byte: u8) -> EntityId {
    EntityId::from_bytes([byte; 32])
}

fn moment(time: u64, microstep: u64) -> SimMoment {
    SimMoment::new(SimTime::from_ticks(time), Microstep::new(microstep))
}

fn accepted(domain: DomainState) -> AcceptedState {
    AcceptedState::new(
        domain,
        EpistemicState::empty(),
        SocialState::empty(),
        AgencyState::empty(),
    )
}

fn containment_belief(
    acting: ActorId,
    item: EntityId,
    previous: EntityId,
    current: EntityId,
) -> EpistemicState {
    let delta = valid(ContainmentTransferDelta::new(
        acting, item, previous, current,
    ));
    let PhysicalEvent::ItemTransferred(event) = PhysicalEvent::item_transferred(delta) else {
        unreachable!("containment transfer must derive item-transfer evidence");
    };
    let generation = EvidenceDeliveryGeneration::new(1)
        .unwrap_or_else(|| unreachable!("one is a nonzero evidence generation"));
    let evidence = EvidenceRecord::direct_item_transfer(acting, generation, event);
    valid(EpistemicState::empty().assimilate(acting, EpistemicVersion::EMPTY, vec![evidence]))
}

fn mobility_domain(
    routes: Vec<DirectedRoute>,
    acting: ActorId,
    location: ActorLocation,
) -> DomainState {
    valid(
        valid(DomainState::new(Vec::new(), Vec::new(), Vec::new()))
            .with_mobility(routes, vec![ActorPosition::new(acting, location)]),
    )
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
            vec![standard_relocation_implementation()],
            lifecycle,
        )),
        profiles,
    )
}

fn deferred_distribution() -> (EngineDistribution, LifecycleProfilesV2) {
    let descriptor =
        DeferredActionEvaluatorDescriptor::new(BaselineActionPolicy::new().semantics_id());
    let action = ActionPolicyInstallation::deferred_captured(descriptor);
    let profiles = baseline_lifecycle_profiles(action.binding());
    let lifecycle = valid(LifecycleImplementationSet::baseline(vec![action]));
    (
        valid(EngineDistribution::new(
            vec![standard_transfer_implementation()],
            vec![standard_relocation_implementation()],
            lifecycle,
        )),
        profiles,
    )
}

fn compile_relocation(distribution: &EngineDistribution) -> Compilation {
    let data = relocation_artifact_data();
    let source = PackSource::new(
        SourceSnapshotId::from_bytes([0x51; 32]),
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

fn compile_transfer(distribution: &EngineDistribution) -> Compilation {
    let data = transfer_artifact_data();
    let source = PackSource::new(
        SourceSnapshotId::from_bytes([0x52; 32]),
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
    initial: AcceptedState,
    opportunities: Vec<ActionOpportunity>,
    profiles: LifecycleProfilesV2,
) -> ResolvedExecution {
    valid(engine.resolve_execution(ExecutionSpecInput::origin(
        compilation.definitions().lock().clone(),
        ExecutionOrigin::new(initial, opportunities, SimMoment::ORIGIN, SimMoment::ORIGIN),
        profiles,
        valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
        RootSeed::from_bytes([0x61; 32]),
        TerminationContractV1::never(),
    )))
}

fn resolve_deferred_origin(
    engine: &Engine,
    compilation: &Compilation,
    initial: AcceptedState,
    opportunities: Vec<ActionOpportunity>,
    profiles: LifecycleProfilesV2,
) -> ResolvedExecution {
    let control = valid(DeferredActionControlV1::enabled(
        DeferredActionAdmissionModeV1::HostScheduled,
        1,
        64 * 1024,
        64 * 1024,
        64 * 1024,
        64 * 1024,
    ));
    valid(engine.resolve_execution(ExecutionSpecInput::origin(
        compilation.definitions().lock().clone(),
        ExecutionOrigin::new(initial, opportunities, SimMoment::ORIGIN, SimMoment::ORIGIN),
        profiles,
        valid(ExecutionConfigArtifactV3::deferred(64, 32, 16, control)),
        RootSeed::from_bytes([0x62; 32]),
        TerminationContractV1::never(),
    )))
}

fn relocation_opportunity(
    acting: ActorId,
    interaction: RelocationInteraction,
    source: EntityId,
    destination: EntityId,
) -> ActionOpportunity {
    ActionOpportunity::open(
        acting,
        ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x71; 32])),
        ActionInteractionScope::relocation(valid(RelocationInteractionScope::new(
            vec![RelocationInteractionAnchor::new(
                interaction,
                source,
                destination,
            )],
            1,
        ))),
        ActionOpportunityGeneration::new(1),
    )
}

fn published(outcome: AdvanceOutcome) -> PublishedStep {
    match outcome {
        AdvanceOutcome::Published {
            moment,
            action_opportunities_consumed,
            attempt_resolved,
            ..
        } => PublishedStep {
            moment,
            action_opportunities_consumed,
            attempt_resolved,
        },
        other => panic!("relocation conformance step must publish, found {other:?}"),
    }
}

fn drive_until_pending(attempt: &mut RunAttempt, through: SimMoment) -> Vec<PublishedStep> {
    let mut steps = Vec::new();
    for _ in 0..64 {
        if !valid(attempt.pending_action_evaluations()).is_empty() {
            return steps;
        }
        match valid(attempt.advance(AdvanceRequest::through(through))) {
            outcome @ AdvanceOutcome::Published { .. } => steps.push(published(outcome)),
            other => panic!("activity must open its successor control, found {other:?}"),
        }
    }
    panic!("activity did not open one successor control within its causal budget")
}

fn capture_only_pending(
    attempt: &mut RunAttempt,
    capture: u64,
    effective: SimMoment,
    expected_verb: RelocationActionVerb,
) -> ActionOpportunityId {
    let pending = valid(attempt.pending_action_evaluations());
    let [pending] = pending.as_slice() else {
        panic!("one foreground activity must expose exactly one pending action evaluation");
    };
    let interaction = pending
        .payload()
        .interaction()
        .relocation()
        .unwrap_or_else(|| panic!("travel activity must expose relocation interaction"));
    let [interaction] = interaction.interactions() else {
        panic!("travel activity must expose one exact control");
    };
    assert_eq!(interaction.verb(), expected_verb);
    let opportunity = pending.payload().opportunity();
    let decision = valid(BaselineActionPolicy::new().decide(pending.payload()));
    valid(
        attempt.capture_action_evaluation_result(ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(capture),
            pending.invocation(),
            effective,
            decision,
        )),
    );
    opportunity
}

fn drain_through(attempt: &mut RunAttempt, through: SimMoment) -> Vec<PublishedStep> {
    let mut steps = Vec::new();
    for _ in 0..128 {
        match valid(attempt.advance(AdvanceRequest::through(through))) {
            outcome @ AdvanceOutcome::Published { .. } => steps.push(published(outcome)),
            AdvanceOutcome::NoWorkDue { .. } | AdvanceOutcome::NoScheduledWork => return steps,
            other => panic!("travel activity must advance normally, found {other:?}"),
        }
    }
    panic!("travel activity did not settle within its causal budget")
}

#[test]
fn travel_activity_owns_start_pause_resume_and_awaits_one_rescheduled_arrival() {
    let (distribution, profiles) = deferred_distribution();
    let compilation = compile_relocation(&distribution);
    let engine = engine(distribution, &compilation);

    let acting = actor(0x14);
    let source = entity(0x25);
    let destination = entity(0x26);
    let route = valid(DirectedRoute::new(
        source,
        destination,
        SimDuration::from_ticks(10),
    ));
    let controller = BaselineActivityController::new();
    let intent = Intent::adopt(
        acting,
        IntentGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
        DesiredCondition::actor_at(destination),
    );
    let travel = valid(TravelActivityState::after_start_opened(
        source,
        destination,
        ActionOpportunityGeneration::new(2),
    ));
    let activity = Activity::start(
        acting,
        intent.id(),
        ActivityGeneration::new(1).unwrap_or_else(|| panic!("test generation is nonzero")),
        ActivityControllerId::from_bytes(controller.implementation_id()),
        activity_state_schema(),
        travel,
    );
    let agency = valid(AgencyState::new(
        vec![intent],
        vec![activity],
        vec![ActivityFocus::new(acting, activity.id())],
    ));
    let start = ActionOpportunity::open(
        acting,
        ActionSponsor::activity(activity.id(), activity.version()),
        ActionInteractionScope::relocation(valid(RelocationInteractionScope::new(
            vec![RelocationInteractionAnchor::new(
                RelocationInteraction::Start(route.id()),
                source,
                destination,
            )],
            1,
        ))),
        ActionOpportunityGeneration::new(1),
    );
    let start_id = start.id();
    let execution = resolve_deferred_origin(
        &engine,
        &compilation,
        AcceptedState::new(
            mobility_domain(vec![route], acting, ActorLocation::at(source)),
            EpistemicState::empty(),
            SocialState::empty(),
            agency,
        ),
        vec![start],
        profiles,
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x84; 32])));

    drive_until_pending(&mut attempt, SimMoment::ORIGIN);
    assert_eq!(
        capture_only_pending(&mut attempt, 1, moment(1, 0), RelocationActionVerb::Start,),
        start_id
    );

    let through_pause_open = drive_until_pending(&mut attempt, moment(4, 64));
    assert!(
        through_pause_open
            .iter()
            .flat_map(|step| step.attempt_resolved.iter())
            .any(|resolved| *resolved == start_id)
    );
    assert_eq!(
        valid(attempt.session().snapshot())
            .accepted()
            .domain()
            .actor_location(acting),
        Some(ActorLocation::in_transit(route))
    );
    let pause_id = capture_only_pending(&mut attempt, 2, moment(4, 0), RelocationActionVerb::Pause);
    assert_ne!(pause_id, start_id);

    let through_resume_open = drive_until_pending(&mut attempt, moment(8, 64));
    assert!(
        through_resume_open
            .iter()
            .flat_map(|step| step.attempt_resolved.iter())
            .any(|resolved| *resolved == pause_id)
    );
    let resume_id =
        capture_only_pending(&mut attempt, 3, moment(8, 0), RelocationActionVerb::Resume);
    assert_ne!(resume_id, pause_id);

    let through_await = drain_through(&mut attempt, moment(10, 64));
    assert!(
        through_await
            .iter()
            .flat_map(|step| step.attempt_resolved.iter())
            .any(|resolved| *resolved == resume_id)
    );
    assert!(valid(attempt.pending_action_evaluations()).is_empty());
    let awaiting = valid(attempt.session().snapshot());
    let accepted_activity = awaiting
        .accepted()
        .agency()
        .activity(activity.id())
        .copied()
        .unwrap_or_else(|| panic!("travel activity must remain accepted"));
    assert_eq!(accepted_activity.status(), ActivityStatus::Waiting);
    let travel = accepted_activity
        .state()
        .travel()
        .unwrap_or_else(|| panic!("travel activity must retain travel state"));
    assert_eq!(travel.step(), TravelActivityStep::AwaitArrival);
    assert_eq!(travel.next_opportunity_generation().get(), 4);
    assert_eq!(
        awaiting.accepted().domain().actor_location(acting),
        Some(ActorLocation::in_transit(route))
    );

    let stale = drain_through(&mut attempt, moment(11, 64));
    assert!(
        stale
            .iter()
            .any(|step| step.moment.time() == SimTime::from_ticks(11)),
        "the original start wake must be consumed as stale"
    );
    assert_eq!(
        valid(attempt.session().snapshot())
            .accepted()
            .domain()
            .actor_location(acting),
        Some(ActorLocation::in_transit(route))
    );

    let mut arrivals = Vec::new();
    let mut previous = ActorLocation::in_transit(route);
    for _ in 0..64 {
        match valid(attempt.advance(AdvanceRequest::through(moment(15, 64)))) {
            AdvanceOutcome::Published { moment, .. } => {
                let location = valid(attempt.session().snapshot())
                    .accepted()
                    .domain()
                    .actor_location(acting)
                    .unwrap_or_else(|| panic!("traveling actor must retain a location"));
                if location == ActorLocation::at(destination) && location != previous {
                    arrivals.push(moment);
                }
                previous = location;
            }
            AdvanceOutcome::NoScheduledWork => break,
            AdvanceOutcome::NoWorkDue { .. } => {
                panic!("resumed arrival must be due by the requested horizon")
            }
            other => panic!("resumed travel must settle normally, found {other:?}"),
        }
    }
    assert_eq!(arrivals.len(), 1);
    assert_eq!(arrivals[0].time(), SimTime::from_ticks(15));
    assert_eq!(
        valid(attempt.session().snapshot())
            .accepted()
            .domain()
            .actor_location(acting),
        Some(ActorLocation::at(destination))
    );
    let arrived = valid(attempt.session().snapshot());
    let arrived_activity = arrived
        .accepted()
        .agency()
        .activity(activity.id())
        .copied()
        .unwrap_or_else(|| panic!("arrival must preserve the travel activity"));
    assert_eq!(arrived_activity.status(), ActivityStatus::Waiting);
    assert_eq!(
        arrived_activity
            .state()
            .travel()
            .map(TravelActivityState::step),
        Some(TravelActivityStep::AwaitArrival),
        "process completion must not mutate agency state"
    );
}

#[test]
fn grounded_start_hides_runtime_legality_and_arrives_once_after_positive_time() {
    let legal_policy = Arc::new(RecordingPolicy::new());
    let rejected_policy = Arc::new(RecordingPolicy::new());
    let (legal_distribution, legal_profiles) = distribution_with_policy(legal_policy.clone());
    let compilation = compile_relocation(&legal_distribution);
    let (rejected_distribution, rejected_profiles) =
        distribution_with_policy(rejected_policy.clone());
    let legal_engine = engine(legal_distribution, &compilation);
    let rejected_engine = engine(rejected_distribution, &compilation);

    let acting = actor(0x11);
    let source = entity(0x21);
    let destination = entity(0x22);
    let route = valid(DirectedRoute::new(
        source,
        destination,
        SimDuration::from_ticks(6),
    ));
    let opportunity = relocation_opportunity(
        acting,
        RelocationInteraction::Start(route.id()),
        source,
        destination,
    );
    let opportunity_id = opportunity.id();
    let legal_initial = accepted(mobility_domain(
        vec![route],
        acting,
        ActorLocation::at(source),
    ));
    let rejected_initial = accepted(mobility_domain(
        Vec::new(),
        acting,
        ActorLocation::at(source),
    ));
    let legal_execution = resolve_origin(
        &legal_engine,
        &compilation,
        legal_initial,
        vec![opportunity.clone()],
        legal_profiles,
    );
    let rejected_execution = resolve_origin(
        &rejected_engine,
        &compilation,
        rejected_initial,
        vec![opportunity],
        rejected_profiles,
    );
    let mut legal_attempt =
        valid(legal_engine.start_attempt(&legal_execution, AttemptKey::from_bytes([0x81; 32])));
    let mut rejected_attempt = valid(
        rejected_engine.start_attempt(&rejected_execution, AttemptKey::from_bytes([0x82; 32])),
    );
    let through = moment(12, 64);

    let legal_ready = published(valid(
        legal_attempt.advance(AdvanceRequest::through(through)),
    ));
    let rejected_ready = published(valid(
        rejected_attempt.advance(AdvanceRequest::through(through)),
    ));
    assert_eq!(legal_ready.moment, SimMoment::ORIGIN);
    assert_eq!(rejected_ready.moment, SimMoment::ORIGIN);
    assert_eq!(
        legal_ready.action_opportunities_consumed,
        vec![opportunity_id]
    );
    assert_eq!(
        rejected_ready.action_opportunities_consumed,
        vec![opportunity_id]
    );
    assert!(legal_ready.attempt_resolved.is_empty());
    assert!(rejected_ready.attempt_resolved.is_empty());
    assert_eq!(
        valid(legal_attempt.session().snapshot())
            .accepted()
            .domain()
            .actor_location(acting),
        Some(ActorLocation::in_transit(route))
    );
    assert_eq!(
        valid(rejected_attempt.session().snapshot())
            .accepted()
            .domain()
            .actor_location(acting),
        Some(ActorLocation::at(source))
    );

    let legal_invocations = legal_policy.invocations();
    let rejected_invocations = rejected_policy.invocations();
    let [legal_invocation] = legal_invocations.as_slice() else {
        panic!("legal execution must ground exactly one actor-facing input");
    };
    let [rejected_invocation] = rejected_invocations.as_slice() else {
        panic!("rejected execution must ground exactly one actor-facing input");
    };
    assert_eq!(legal_invocation, rejected_invocation);
    assert_eq!(legal_invocation.input.opportunity(), opportunity_id);
    assert_eq!(legal_invocation.input.candidates().candidates().len(), 1);

    let mut legal_previous = ActorLocation::in_transit(route);
    let mut legal_arrivals = Vec::new();
    let mut legal_neutral = Vec::new();
    let mut legal_quiesced = false;
    for _ in 0..64 {
        match valid(legal_attempt.advance(AdvanceRequest::through(through))) {
            AdvanceOutcome::Published {
                moment,
                attempt_resolved,
                ..
            } => {
                legal_neutral.extend(attempt_resolved.into_iter().map(|id| (moment, id)));
                let location = valid(legal_attempt.session().snapshot())
                    .accepted()
                    .domain()
                    .actor_location(acting)
                    .unwrap_or_else(|| panic!("the moving actor must remain positioned"));
                if location == ActorLocation::at(destination) && location != legal_previous {
                    legal_arrivals.push(moment);
                }
                legal_previous = location;
            }
            AdvanceOutcome::NoScheduledWork => {
                legal_quiesced = true;
                break;
            }
            other => panic!("legal relocation must quiesce through its arrival, found {other:?}"),
        }
    }

    let mut rejected_moments = Vec::new();
    let mut rejected_neutral = Vec::new();
    let mut rejected_quiesced = false;
    for _ in 0..64 {
        match valid(rejected_attempt.advance(AdvanceRequest::through(through))) {
            AdvanceOutcome::Published {
                moment,
                attempt_resolved,
                ..
            } => {
                rejected_moments.push(moment);
                rejected_neutral.extend(attempt_resolved.into_iter().map(|id| (moment, id)));
            }
            AdvanceOutcome::NoScheduledWork => {
                rejected_quiesced = true;
                break;
            }
            other => panic!("rejected relocation must quiesce, found {other:?}"),
        }
    }

    assert!(legal_quiesced);
    assert!(rejected_quiesced);
    assert_eq!(legal_arrivals, vec![moment(6, 0)]);
    assert_eq!(legal_neutral.len(), 1);
    assert_eq!(rejected_neutral.len(), 1);
    assert_eq!(legal_neutral[0].1, opportunity_id);
    assert_eq!(rejected_neutral[0].1, opportunity_id);
    assert!(
        rejected_moments
            .iter()
            .all(|published| published.time() == SimTime::from_ticks(0)),
        "a rejected start must not create positive-time process work"
    );
    assert_eq!(
        valid(legal_attempt.session().snapshot())
            .accepted()
            .domain()
            .actor_location(acting),
        Some(ActorLocation::at(destination))
    );
    assert_eq!(
        valid(rejected_attempt.session().snapshot())
            .accepted()
            .domain()
            .actor_location(acting),
        Some(ActorLocation::at(source))
    );
}

#[test]
fn grounded_pause_and_resume_without_a_process_resolve_neutrally() {
    for (case, pause) in [(0x01, true), (0x02, false)] {
        let policy = Arc::new(RecordingPolicy::new());
        let (distribution, profiles) = distribution_with_policy(policy.clone());
        let compilation = compile_relocation(&distribution);
        let engine = engine(distribution, &compilation);

        let acting = actor(0x12);
        let source = entity(0x23);
        let destination = entity(0x24);
        let route = valid(DirectedRoute::new(
            source,
            destination,
            SimDuration::from_ticks(8),
        ));
        let interaction = if pause {
            RelocationInteraction::Pause(route.id())
        } else {
            RelocationInteraction::Resume(route.id())
        };
        let opportunity = relocation_opportunity(acting, interaction, source, destination);
        let opportunity_id = opportunity.id();
        let initial = accepted(mobility_domain(
            vec![route],
            acting,
            ActorLocation::in_transit(route),
        ));
        let execution = resolve_origin(&engine, &compilation, initial, vec![opportunity], profiles);
        let mut attempt =
            valid(engine.start_attempt(&execution, AttemptKey::from_bytes([case; 32])));
        let through = moment(16, 64);

        let ready = published(valid(attempt.advance(AdvanceRequest::through(through))));
        assert_eq!(ready.moment, SimMoment::ORIGIN);
        assert_eq!(ready.action_opportunities_consumed, vec![opportunity_id]);
        assert!(ready.attempt_resolved.is_empty());

        let mut published_moments = Vec::new();
        let mut neutral = Vec::new();
        let mut quiesced = false;
        for _ in 0..16 {
            match valid(attempt.advance(AdvanceRequest::through(through))) {
                AdvanceOutcome::Published {
                    moment,
                    attempt_resolved,
                    ..
                } => {
                    published_moments.push(moment);
                    neutral.extend(attempt_resolved);
                }
                AdvanceOutcome::NoScheduledWork => {
                    quiesced = true;
                    break;
                }
                other => panic!("grounded process control must quiesce, found {other:?}"),
            }
        }

        assert!(quiesced);
        assert_eq!(neutral, vec![opportunity_id]);
        assert!(
            published_moments
                .iter()
                .all(|published| published.time() == SimTime::from_ticks(0)),
            "a rejected control interaction must not invent process wake work"
        );
        assert_eq!(
            valid(attempt.session().snapshot())
                .accepted()
                .domain()
                .actor_location(acting),
            Some(ActorLocation::in_transit(route))
        );
        let invocations = policy.invocations();
        let [invocation] = invocations.as_slice() else {
            panic!("each process-control opportunity must reach the actor policy once");
        };
        assert_eq!(invocation.input.opportunity(), opportunity_id);
        assert_eq!(invocation.input.candidates().candidates().len(), 1);
    }
}

#[test]
fn containment_transfer_is_immediate_and_creates_no_relocation_process() {
    let policy = Arc::new(RecordingPolicy::new());
    let (distribution, profiles) = distribution_with_policy(policy);
    let compilation = compile_transfer(&distribution);
    let engine = engine(distribution, &compilation);

    let acting = actor(0x13);
    let item = entity(0x33);
    let source = entity(0x43);
    let destination = entity(0x44);
    let route = valid(DirectedRoute::new(
        source,
        destination,
        SimDuration::from_ticks(9),
    ));
    let domain = valid(
        valid(DomainState::new(
            vec![
                ContainerRecord::new(source, 4),
                ContainerRecord::new(destination, 4),
            ],
            vec![ContainmentRecord::new(item, source)],
            vec![ContainerAuthorityRecord::new(acting, source)],
        ))
        .with_mobility(
            vec![route],
            vec![ActorPosition::new(acting, ActorLocation::at(source))],
        ),
    );
    let opportunity = ActionOpportunity::open(
        acting,
        ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x72; 32])),
        ActionInteractionScope::containment(valid(ContainmentInteractionScope::new(
            source,
            vec![destination],
            vec![item],
            1,
        ))),
        ActionOpportunityGeneration::new(1),
    );
    let execution = resolve_origin(
        &engine,
        &compilation,
        AcceptedState::new(
            domain,
            containment_belief(acting, item, destination, source),
            SocialState::empty(),
            AgencyState::empty(),
        ),
        vec![opportunity],
        profiles,
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x83; 32])));
    let through = moment(18, 64);
    let mut published_moments = Vec::new();
    let mut quiesced = false;

    for _ in 0..64 {
        match valid(attempt.advance(AdvanceRequest::through(through))) {
            AdvanceOutcome::Published { moment, .. } => published_moments.push(moment),
            AdvanceOutcome::NoScheduledWork => {
                quiesced = true;
                break;
            }
            other => panic!("containment transfer must quiesce, found {other:?}"),
        }
    }

    assert!(quiesced);
    assert!(
        published_moments
            .iter()
            .all(|published| published.time() == SimTime::from_ticks(0)),
        "an immediate containment action may use causal microsteps but no travel-time process"
    );
    let snapshot = valid(attempt.session().snapshot());
    assert_eq!(
        snapshot
            .accepted()
            .domain()
            .containment_for(item)
            .map(|record| record.container()),
        Some(destination)
    );
    assert_eq!(
        snapshot.accepted().domain().actor_location(acting),
        Some(ActorLocation::at(source))
    );
}
