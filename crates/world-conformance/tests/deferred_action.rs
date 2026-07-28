//! Public retained-action-evaluation conformance through the engine facade.

use std::{collections::BTreeMap, sync::Arc};

use world_authoring::{
    AuthoringCompiler, Compilation, CompileRequest, PackSource, SourceSnapshotId,
};
use world_engine::{
    AcceptedState, ActionDecision, ActionEvaluationCaptureError, ActionEvaluationCaptureId,
    ActionEvaluationCaptureOutcome, ActionEvaluationManagementDisposition,
    ActionEvaluationResultCapture, ActionInteractionScope, ActionOpportunity,
    ActionOpportunityGeneration, ActionOpportunityId, ActionPolicyInstallation, ActionSponsor,
    ActorId, ActorReactionCause, AdvanceOutcome, AdvanceRequest, AgencyState, ArtifactEnvelope,
    ArtifactResolveError, ArtifactResolver, AttemptError, AttemptKey, BaselineActionPolicy,
    CommandBinding, CommandId, CommandValue, ContainerAuthorityRecord, ContainerRecord,
    ContainmentInteractionScope, ContainmentRecord, ContainmentTransferDelta,
    DeferredActionAdmissionModeV1, DeferredActionControlV1, DeferredActionEvaluatorDescriptor,
    DomainState, EngineBuilder, EngineDistribution, EntityId, EpistemicState,
    EvidenceDeliveryGeneration, EvidenceRecord, ExecutionConfigArtifactV3, ExecutionOrigin,
    ExecutionSpecInput, GroundedActionCandidateId, InputId, LifecycleImplementationSet,
    LifecycleProfilesV2, ManageOutcome, ManageRequest, ManagementRequestId, Microstep,
    PackLockEntry, PhysicalEvent, RootSeed, RunAttempt, RuntimeService, SessionManagement,
    SimMoment, SimTime, SocialState, SystemCommandRequest, SystemCommandSourceId,
    TerminationContractV1, baseline_lifecycle_profiles,
};
use world_standard::{transfer_action_key, transfer_artifact_data};
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

#[derive(Debug)]
struct PublishedStep {
    moment: SimMoment,
    action_opportunities_consumed: Vec<ActionOpportunityId>,
    attempt_resolved: Vec<ActionOpportunityId>,
}

fn valid<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("deferred-action conformance fixture must be valid: {error:?}"),
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

fn accepted_domain(
    containers: Vec<ContainerRecord>,
    containment: Vec<ContainmentRecord>,
    authority: Vec<ContainerAuthorityRecord>,
    beliefs: Vec<(ActorId, EntityId, EntityId)>,
) -> AcceptedState {
    let mut epistemic = EpistemicState::empty();
    let mut next_evidence_generation = BTreeMap::new();
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
        let generation = next_evidence_generation.entry(observer).or_insert(1);
        let evidence = EvidenceRecord::direct_item_transfer(
            observer,
            EvidenceDeliveryGeneration::new(*generation)
                .unwrap_or_else(|| panic!("fixture evidence generation is nonzero")),
            event,
        );
        *generation += 1;
        epistemic = valid(epistemic.assimilate(
            observer,
            epistemic.actor_version(observer),
            vec![evidence],
        ));
    }
    AcceptedState::new(
        valid(DomainState::new(containers, containment, authority)),
        epistemic,
        SocialState::empty(),
        AgencyState::empty(),
    )
}

fn base_state(
    acting: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
) -> AcceptedState {
    accepted_domain(
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 4),
        ],
        vec![ContainmentRecord::new(item, source)],
        vec![ContainerAuthorityRecord::new(acting, source)],
        vec![(acting, item, source)],
    )
}

fn opportunity(
    acting: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
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
        ActionOpportunityGeneration::new(1),
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
            Vec::new(),
            lifecycle,
        )),
        profiles,
    )
}

fn compile_standard(distribution: &EngineDistribution) -> Compilation {
    let data = transfer_artifact_data();
    let source = PackSource::new(
        SourceSnapshotId::from_bytes([0x57; 32]),
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

fn deferred_attempt(
    admission_mode: DeferredActionAdmissionModeV1,
    maximum_visible_reinvocations: u32,
    accepted: AcceptedState,
    opportunities: Vec<ActionOpportunity>,
    key: u8,
) -> RunAttempt {
    let (distribution, profiles) = deferred_distribution();
    let compilation = compile_standard(&distribution);
    let artifacts: Arc<dyn ArtifactResolver> = Arc::new(InMemoryArtifacts {
        envelopes: compilation.envelopes().to_vec(),
    });
    let engine = valid(
        EngineBuilder::new(distribution, artifacts, valid(RuntimeService::in_memory())).build(),
    );
    let control = valid(DeferredActionControlV1::enabled(
        admission_mode,
        maximum_visible_reinvocations,
        64 * 1024,
        64 * 1024,
        64 * 1024,
        64 * 1024,
    ));
    let config = valid(ExecutionConfigArtifactV3::deferred(64, 32, 16, control));
    let execution = valid(engine.resolve_execution(ExecutionSpecInput::origin(
        compilation.definitions().lock().clone(),
        ExecutionOrigin::new(
            accepted,
            opportunities,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
        ),
        profiles,
        config,
        RootSeed::from_bytes([0x61; 32]),
        TerminationContractV1::never(),
    )));
    valid(engine.start_attempt(&execution, AttemptKey::from_bytes([key; 32])))
}

#[allow(
    clippy::too_many_arguments,
    reason = "the conformance fixture keeps each transfer role explicit at call sites"
)]
fn transfer_request(
    input: u64,
    effective: SimMoment,
    source_id: u8,
    command: u64,
    acting: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
) -> SystemCommandRequest {
    SystemCommandRequest::new(
        InputId::new(input),
        effective,
        SystemCommandSourceId::from_bytes([source_id; 32]),
        CommandId::new(command),
        acting,
        transfer_action_key(),
        vec![
            CommandBinding::new(
                valid(world_engine::BindingName::parse("actor")),
                CommandValue::Actor(acting),
            ),
            CommandBinding::new(
                valid(world_engine::BindingName::parse("destination")),
                CommandValue::Entity(destination),
            ),
            CommandBinding::new(
                valid(world_engine::BindingName::parse("item")),
                CommandValue::Entity(item),
            ),
            CommandBinding::new(
                valid(world_engine::BindingName::parse("source")),
                CommandValue::Entity(source),
            ),
        ],
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
        other => panic!("deferred-action step must publish, found {other:?}"),
    }
}

fn selected_decision(payload: &world_engine::ActionContextPayload) -> ActionDecision {
    valid(BaselineActionPolicy::new().decide(payload))
}

fn direct_container(attempt: &RunAttempt, item: EntityId) -> Option<EntityId> {
    valid(attempt.session().snapshot())
        .accepted()
        .domain()
        .containment_for(item)
        .map(|record| record.container())
}

#[test]
fn host_scheduled_result_is_durable_typed_idempotent_and_later_than_dispatch() {
    let acting = actor(0x11);
    let item = entity(0x21);
    let source = entity(0x31);
    let destination = entity(0x41);
    let action = opportunity(acting, item, source, destination);
    let action_id = action.id();
    let mut attempt = deferred_attempt(
        DeferredActionAdmissionModeV1::HostScheduled,
        1,
        base_state(acting, item, source, destination),
        vec![action],
        0x81,
    );

    assert!(valid(attempt.pending_action_evaluations()).is_empty());
    let dispatch = published(valid(
        attempt.advance(AdvanceRequest::through(SimMoment::ORIGIN)),
    ));
    assert_eq!(dispatch.moment, SimMoment::ORIGIN);
    assert!(dispatch.action_opportunities_consumed.is_empty());
    assert!(dispatch.attempt_resolved.is_empty());
    assert_eq!(direct_container(&attempt, item), Some(source));

    let pending = valid(attempt.pending_action_evaluations());
    let [pending] = pending.as_slice() else {
        panic!("the published creating moment must expose exactly one retained request");
    };
    assert_eq!(pending.payload().actor(), acting);
    assert_eq!(pending.payload().opportunity(), action_id);
    assert_eq!(
        pending.payload().policy_semantics(),
        BaselineActionPolicy::new().semantics_id()
    );
    assert_eq!(
        pending.admission_mode(),
        DeferredActionAdmissionModeV1::HostScheduled
    );
    assert_eq!(pending.payload().candidates().candidates().len(), 1);

    let invocation = pending.invocation();
    let decision = selected_decision(pending.payload());
    let effective = moment(3);
    let capture = ActionEvaluationResultCapture::host_scheduled(
        ActionEvaluationCaptureId::new(1),
        invocation,
        effective,
        decision,
    );
    let first = valid(attempt.capture_action_evaluation_result(capture));
    assert!(matches!(
        first,
        ActionEvaluationCaptureOutcome::ResultCaptured {
            invocation: captured,
            effective: due,
            ..
        } if captured == invocation && due == effective
    ));
    assert!(valid(attempt.pending_action_evaluations()).is_empty());
    assert_eq!(direct_container(&attempt, item), Some(source));

    let cursor_after_capture = valid(attempt.session().cursor());
    assert_eq!(
        valid(attempt.capture_action_evaluation_result(capture)),
        first
    );
    assert_eq!(valid(attempt.session().cursor()), cursor_after_capture);
    assert_eq!(
        valid(attempt.advance(AdvanceRequest::through(moment(2)))),
        AdvanceOutcome::NoWorkDue {
            next: effective,
            through: moment(2),
        }
    );
    assert_eq!(direct_container(&attempt, item), Some(source));

    let result_ready = published(valid(attempt.advance(AdvanceRequest::through(effective))));
    assert_eq!(result_ready.moment, effective);
    assert_eq!(result_ready.action_opportunities_consumed, vec![action_id]);
    assert!(result_ready.attempt_resolved.is_empty());
    assert_eq!(direct_container(&attempt, item), Some(source));

    let action_moment = moment(4);
    assert_eq!(
        published(valid(
            attempt.advance(AdvanceRequest::through(action_moment))
        ))
        .moment,
        action_moment
    );
    assert_eq!(direct_container(&attempt, item), Some(destination));
    assert_eq!(
        attempt.capture_action_evaluation_result(ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(2),
            invocation,
            moment(6),
            decision,
        )),
        Err(ActionEvaluationCaptureError::LateInvocation { invocation })
    );
}

#[test]
fn cancellation_replaces_a_captured_result_with_the_fixed_later_fallback() {
    let acting = actor(0x12);
    let item = entity(0x22);
    let source = entity(0x32);
    let destination = entity(0x42);
    let action = opportunity(acting, item, source, destination);
    let action_id = action.id();
    let mut attempt = deferred_attempt(
        DeferredActionAdmissionModeV1::HostScheduled,
        1,
        base_state(acting, item, source, destination),
        vec![action],
        0x82,
    );
    published(valid(
        attempt.advance(AdvanceRequest::through(SimMoment::ORIGIN)),
    ));
    let pending = valid(attempt.pending_action_evaluations());
    let [pending] = pending.as_slice() else {
        panic!("the action must retain one evaluator request");
    };
    let invocation = pending.invocation();
    let decision = selected_decision(pending.payload());
    valid(
        attempt.capture_action_evaluation_result(ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(3),
            invocation,
            moment(5),
            decision,
        )),
    );

    let management = ManageRequest::new(
        ManagementRequestId::new(1),
        SessionManagement::ResolveActionEvaluation {
            invocation,
            disposition: ActionEvaluationManagementDisposition::Cancel,
        },
    );
    let managed = valid(attempt.submit_management_request(management));
    assert!(matches!(
        managed,
        ManageOutcome::ActionEvaluationFallbackScheduled {
            invocation: managed_invocation,
            disposition: ActionEvaluationManagementDisposition::Cancel,
            ..
        } if managed_invocation == invocation
    ));
    let cursor_after_management = valid(attempt.session().cursor());
    assert_eq!(
        valid(attempt.submit_management_request(management)),
        managed
    );
    assert_eq!(
        valid(attempt.session().cursor()),
        cursor_after_management,
        "an exact management retry must not publish again"
    );
    assert_eq!(
        attempt.capture_action_evaluation_result(ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(4),
            invocation,
            moment(6),
            decision,
        )),
        Err(ActionEvaluationCaptureError::LateInvocation { invocation })
    );

    let fallback = published(valid(attempt.advance(AdvanceRequest::through(moment(5)))));
    assert_eq!(fallback.moment, moment(5));
    assert_eq!(fallback.action_opportunities_consumed, vec![action_id]);
    assert!(fallback.attempt_resolved.is_empty());
    assert_eq!(direct_container(&attempt, item), Some(source));
    let wake = published(valid(attempt.advance(AdvanceRequest::through(moment(6)))));
    assert_eq!(wake.moment, moment(6));
    assert_eq!(wake.attempt_resolved, vec![action_id]);
    assert_eq!(
        valid(attempt.advance(AdvanceRequest::through(moment(7)))),
        AdvanceOutcome::NoScheduledWork,
        "management must remove the captured ResultReady before scheduling fallback"
    );
}

#[test]
fn frontier_blocking_holds_advancement_and_sealing_until_management_releases_it() {
    let acting = actor(0x13);
    let item = entity(0x23);
    let source = entity(0x33);
    let destination = entity(0x43);
    let action = opportunity(acting, item, source, destination);
    let action_id = action.id();
    let mut attempt = deferred_attempt(
        DeferredActionAdmissionModeV1::FrontierBlocking,
        1,
        base_state(acting, item, source, destination),
        vec![action],
        0x83,
    );
    published(valid(
        attempt.advance(AdvanceRequest::through(SimMoment::ORIGIN)),
    ));
    let pending = valid(attempt.pending_action_evaluations());
    let [pending] = pending.as_slice() else {
        panic!("the blocking action must retain one evaluator request");
    };
    assert_eq!(
        pending.admission_mode(),
        DeferredActionAdmissionModeV1::FrontierBlocking
    );
    let invocation = pending.invocation();
    let blocked_at = moment(1);

    assert_eq!(
        attempt.advance(AdvanceRequest::through(moment(2))),
        Err(AttemptError::ActionEvaluationFrontierBlocked { blocked_at })
    );
    assert_eq!(
        attempt.submit_management_request(ManageRequest::new(
            ManagementRequestId::new(2),
            SessionManagement::SealAdmissionThrough(moment(2)),
        )),
        Err(AttemptError::ActionEvaluationFrontierBlocked { blocked_at })
    );

    valid(attempt.submit_management_request(ManageRequest::new(
        ManagementRequestId::new(3),
        SessionManagement::ResolveActionEvaluation {
            invocation,
            disposition: ActionEvaluationManagementDisposition::Timeout,
        },
    )));
    let fallback = published(valid(attempt.advance(AdvanceRequest::through(blocked_at))));
    assert_eq!(fallback.moment, blocked_at);
    assert_eq!(fallback.action_opportunities_consumed, vec![action_id]);
    assert!(fallback.attempt_resolved.is_empty());
    let wake = published(valid(attempt.advance(AdvanceRequest::through(moment(2)))));
    assert_eq!(wake.moment, moment(2));
    assert_eq!(wake.attempt_resolved, vec![action_id]);
    assert!(matches!(
        valid(
            attempt.submit_management_request(ManageRequest::new(
                ManagementRequestId::new(4),
                SessionManagement::SealAdmissionThrough(moment(4)),
            ))
        ),
        ManageOutcome::AdmissionSealed {
            frontier,
            ..
        } if frontier == moment(4)
    ));
}

#[test]
fn hidden_execution_change_revalidates_without_reinvoking_the_actor_policy() {
    let acting = actor(0x14);
    let hidden_actor = actor(0x15);
    let item = entity(0x24);
    let hidden_item = entity(0x25);
    let source = entity(0x34);
    let hidden_source = entity(0x35);
    let destination = entity(0x44);
    let initial = accepted_domain(
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(hidden_source, 4),
            ContainerRecord::new(destination, 1),
        ],
        vec![
            ContainmentRecord::new(item, source),
            ContainmentRecord::new(hidden_item, hidden_source),
        ],
        vec![
            ContainerAuthorityRecord::new(acting, source),
            ContainerAuthorityRecord::new(hidden_actor, hidden_source),
        ],
        vec![(acting, item, source)],
    );
    let action = opportunity(acting, item, source, destination);
    let mut attempt = deferred_attempt(
        DeferredActionAdmissionModeV1::HostScheduled,
        1,
        initial,
        vec![action],
        0x84,
    );
    published(valid(
        attempt.advance(AdvanceRequest::through(SimMoment::ORIGIN)),
    ));
    let pending = valid(attempt.pending_action_evaluations());
    let [pending] = pending.as_slice() else {
        panic!("the action must retain one evaluator request");
    };
    let invocation = pending.invocation();
    let payload = pending.payload().clone();
    let decision = selected_decision(&payload);
    valid(
        attempt.capture_action_evaluation_result(ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(5),
            invocation,
            moment(2),
            decision,
        )),
    );
    valid(attempt.submit_system_command(transfer_request(
        1,
        moment(1),
        0xa1,
        1,
        hidden_actor,
        hidden_item,
        hidden_source,
        destination,
    )));

    assert_eq!(
        published(valid(attempt.advance(AdvanceRequest::through(moment(1))))).moment,
        moment(1)
    );
    assert_eq!(direct_container(&attempt, hidden_item), Some(destination));
    assert_eq!(direct_container(&attempt, item), Some(source));

    let result = published(valid(attempt.advance(AdvanceRequest::through(moment(2)))));
    assert_eq!(result.moment, moment(2));
    assert_eq!(direct_container(&attempt, item), Some(source));
    assert!(
        valid(attempt.pending_action_evaluations()).is_empty(),
        "a private legality change must not create a visible successor request"
    );

    assert_eq!(
        published(valid(attempt.advance(AdvanceRequest::through(moment(3))))).moment,
        moment(3)
    );
    assert_eq!(
        direct_container(&attempt, item),
        Some(source),
        "the original policy decision must be revalidated against fresh private legality"
    );
    assert_eq!(
        attempt.capture_action_evaluation_result(ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(6),
            invocation,
            moment(4),
            decision,
        )),
        Err(ActionEvaluationCaptureError::LateInvocation { invocation })
    );
}

#[test]
fn byte_identical_payload_rebinds_changed_projection_without_a_successor_invocation() {
    let acting = actor(0x18);
    let item = entity(0x28);
    let decoy = entity(0x29);
    let source = entity(0x38);
    let destination = entity(0x48);
    let decoy_source = entity(0x39);
    let decoy_destination = entity(0x49);
    let initial = accepted_domain(
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, 4),
            ContainerRecord::new(decoy_source, 4),
            ContainerRecord::new(decoy_destination, 4),
        ],
        vec![
            ContainmentRecord::new(item, source),
            ContainmentRecord::new(decoy, decoy_source),
        ],
        vec![
            ContainerAuthorityRecord::new(acting, source),
            ContainerAuthorityRecord::new(acting, decoy_source),
        ],
        vec![(acting, item, source), (acting, decoy, decoy_source)],
    );
    let action = ActionOpportunity::open(
        acting,
        ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x71; 32])),
        ActionInteractionScope::containment(valid(ContainmentInteractionScope::new(
            source,
            vec![destination],
            vec![item, decoy],
            8,
        ))),
        ActionOpportunityGeneration::new(1),
    );
    let probe_action = action.clone();
    let action_id = action.id();
    let mut attempt = deferred_attempt(
        DeferredActionAdmissionModeV1::HostScheduled,
        1,
        initial,
        vec![action],
        0x87,
    );

    published(valid(
        attempt.advance(AdvanceRequest::through(SimMoment::ORIGIN)),
    ));
    let pending = valid(attempt.pending_action_evaluations());
    let [pending] = pending.as_slice() else {
        panic!("the action must retain one evaluator request");
    };
    let invocation = pending.invocation();
    let payload = pending.payload().clone();
    assert_eq!(payload.candidates().candidates().len(), 1);
    let decision = selected_decision(&payload);
    valid(
        attempt.capture_action_evaluation_result(ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(9),
            invocation,
            moment(4),
            decision,
        )),
    );
    valid(attempt.submit_system_command(transfer_request(
        3,
        moment(1),
        0xa3,
        3,
        acting,
        decoy,
        decoy_source,
        decoy_destination,
    )));

    for expected in [moment(1), moment(2), moment(3)] {
        assert_eq!(
            published(valid(attempt.advance(AdvanceRequest::through(expected)))).moment,
            expected
        );
    }
    let snapshot = valid(attempt.session().snapshot());
    assert_eq!(
        snapshot
            .accepted()
            .epistemic()
            .contained_in(acting, decoy)
            .map(|belief| belief.container()),
        Some(decoy_destination),
        "the scoped decoy belief must change the retained projection witness"
    );
    assert_eq!(direct_container(&attempt, item), Some(source));

    let mut rebuilt = deferred_attempt(
        DeferredActionAdmissionModeV1::HostScheduled,
        1,
        snapshot.accepted().clone(),
        vec![probe_action],
        0x89,
    );
    published(valid(
        rebuilt.advance(AdvanceRequest::through(SimMoment::ORIGIN)),
    ));
    let rebuilt_pending = valid(rebuilt.pending_action_evaluations());
    let [rebuilt_pending] = rebuilt_pending.as_slice() else {
        panic!("the rebuilt state must expose one comparison request");
    };
    assert_eq!(
        rebuilt_pending.payload(),
        &payload,
        "the changed actor belief remains outside the opportunity source and must rebuild byte-identical actor input"
    );

    let rebound = published(valid(attempt.advance(AdvanceRequest::through(moment(4)))));
    assert_eq!(rebound.moment, moment(4));
    assert_eq!(rebound.action_opportunities_consumed, vec![action_id]);
    assert!(
        valid(attempt.pending_action_evaluations()).is_empty(),
        "a byte-identical rebuilt payload must retain the original logical invocation"
    );
    assert_eq!(
        attempt.capture_action_evaluation_result(ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(10),
            invocation,
            moment(6),
            decision,
        )),
        Err(ActionEvaluationCaptureError::LateInvocation { invocation }),
        "private rebinding must terminalize the original invocation without a successor"
    );

    assert_eq!(
        published(valid(attempt.advance(AdvanceRequest::through(moment(5))))).moment,
        moment(5)
    );
    assert_eq!(direct_container(&attempt, item), Some(destination));
}

#[test]
fn invalid_typed_result_is_recorded_then_finishes_through_the_later_fallback() {
    let acting = actor(0x19);
    let item = entity(0x2a);
    let source = entity(0x3a);
    let destination = entity(0x4a);
    let action = opportunity(acting, item, source, destination);
    let action_id = action.id();
    let mut attempt = deferred_attempt(
        DeferredActionAdmissionModeV1::HostScheduled,
        1,
        base_state(acting, item, source, destination),
        vec![action],
        0x88,
    );

    published(valid(
        attempt.advance(AdvanceRequest::through(SimMoment::ORIGIN)),
    ));
    let pending = valid(attempt.pending_action_evaluations());
    let [pending] = pending.as_slice() else {
        panic!("the action must retain one evaluator request");
    };
    let invocation = pending.invocation();
    let invalid_decision = ActionDecision::Select {
        candidate: GroundedActionCandidateId::from_bytes([0xfe; 32]),
        input: pending.payload().input_fingerprint(),
    };
    let captured = valid(attempt.capture_action_evaluation_result(
        ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(11),
            invocation,
            moment(2),
            invalid_decision,
        ),
    ));
    assert!(matches!(
        captured,
        ActionEvaluationCaptureOutcome::ResultCaptured {
            invocation: captured_invocation,
            effective,
            ..
        } if captured_invocation == invocation && effective == moment(2)
    ));

    let interpreted = published(valid(attempt.advance(AdvanceRequest::through(moment(2)))));
    assert_eq!(interpreted.moment, moment(2));
    assert!(interpreted.action_opportunities_consumed.is_empty());
    assert!(interpreted.attempt_resolved.is_empty());
    assert!(
        valid(attempt.pending_action_evaluations()).is_empty(),
        "invalid typed content must close dispatch and record the fixed fallback"
    );
    assert_eq!(direct_container(&attempt, item), Some(source));

    let fallback = published(valid(attempt.advance(AdvanceRequest::through(moment(3)))));
    assert_eq!(fallback.moment, moment(3));
    assert_eq!(fallback.action_opportunities_consumed, vec![action_id]);
    assert!(fallback.attempt_resolved.is_empty());
    assert_eq!(direct_container(&attempt, item), Some(source));

    let wake = published(valid(attempt.advance(AdvanceRequest::through(moment(4)))));
    assert_eq!(wake.moment, moment(4));
    assert_eq!(wake.attempt_resolved, vec![action_id]);
    assert_eq!(
        valid(attempt.advance(AdvanceRequest::through(moment(5)))),
        AdvanceOutcome::NoScheduledWork
    );
    assert_eq!(
        attempt.capture_action_evaluation_result(ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(12),
            invocation,
            moment(6),
            invalid_decision,
        )),
        Err(ActionEvaluationCaptureError::LateInvocation { invocation })
    );
}

#[test]
fn visible_source_membership_change_creates_one_linked_successor_request() {
    let acting = actor(0x16);
    let item = entity(0x26);
    let source = entity(0x36);
    let destination = entity(0x46);
    let action = opportunity(acting, item, source, destination);
    let mut attempt = deferred_attempt(
        DeferredActionAdmissionModeV1::HostScheduled,
        1,
        base_state(acting, item, source, destination),
        vec![action],
        0x85,
    );
    published(valid(
        attempt.advance(AdvanceRequest::through(SimMoment::ORIGIN)),
    ));
    let pending = valid(attempt.pending_action_evaluations());
    let [pending] = pending.as_slice() else {
        panic!("the action must retain one evaluator request");
    };
    let first_invocation = pending.invocation();
    let first_request = pending.request();
    let first_payload = pending.payload().clone();
    let first_decision = selected_decision(&first_payload);
    valid(
        attempt.capture_action_evaluation_result(ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(7),
            first_invocation,
            moment(4),
            first_decision,
        )),
    );
    valid(attempt.submit_system_command(transfer_request(
        2,
        moment(1),
        0xa2,
        2,
        acting,
        item,
        source,
        destination,
    )));

    for expected in [moment(1), moment(2), moment(3)] {
        assert_eq!(
            published(valid(attempt.advance(AdvanceRequest::through(expected)))).moment,
            expected
        );
    }
    assert_eq!(direct_container(&attempt, item), Some(destination));
    assert_eq!(
        valid(attempt.session().snapshot())
            .accepted()
            .epistemic()
            .contained_in(acting, item)
            .map(|belief| belief.container()),
        Some(destination)
    );

    let rebound = published(valid(attempt.advance(AdvanceRequest::through(moment(4)))));
    assert_eq!(rebound.moment, moment(4));
    let successor = valid(attempt.pending_action_evaluations());
    let [successor] = successor.as_slice() else {
        panic!("one visible payload change must create exactly one successor request");
    };
    assert_ne!(successor.invocation(), first_invocation);
    assert_ne!(successor.request(), first_request);
    assert_ne!(successor.payload(), &first_payload);
    assert_eq!(successor.payload().actor(), acting);
    assert!(successor.payload().candidates().candidates().is_empty());

    let successor_invocation = successor.invocation();
    let successor_decision = selected_decision(successor.payload());
    valid(
        attempt.capture_action_evaluation_result(ActionEvaluationResultCapture::host_scheduled(
            ActionEvaluationCaptureId::new(8),
            successor_invocation,
            moment(5),
            successor_decision,
        )),
    );
    assert_eq!(
        published(valid(attempt.advance(AdvanceRequest::through(moment(5))))).moment,
        moment(5)
    );
    assert!(valid(attempt.pending_action_evaluations()).is_empty());
    assert_eq!(direct_container(&attempt, item), Some(destination));
}
