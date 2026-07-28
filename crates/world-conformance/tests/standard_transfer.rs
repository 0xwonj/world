use std::sync::Arc;

use world_authoring::{
    AuthoringCompiler, Compilation, CompileRequest, EngineProtocolVersion, PackSource,
    SourceSnapshotId,
};
use world_engine::{
    AcceptedState, ActionContextPayload, ActionDecision, ActionInteractionScope, ActionOpportunity,
    ActionOpportunityGeneration, ActionPolicy, ActionPolicyError, ActionPolicyInstallation,
    ActionPolicySemanticsId, ActionSponsor, ActorId, ActorReactionCause, AdvanceOutcome,
    AdvanceRequest, AgencyState, ArtifactEnvelope, ArtifactResolveError, ArtifactResolver,
    AttemptError, AttemptKey, BaselineActionPolicy, CommandAttemptOutcome, CommandBinding,
    CommandId, CommandResolution, CommandSource, CommandValue, ContainerAuthorityRecord,
    ContainerRecord, ContainmentInteractionScope, ContainmentRecord, ContainmentTransferDelta,
    DomainState, Engine, EngineBuilder, EngineDistribution, EntityId, EpistemicState,
    EpistemicVersion, EvidenceDeliveryGeneration, EvidenceRecord, ExecutionActivationError,
    ExecutionConfigArtifactV3, ExecutionOrigin, ExecutionSpecInput, InputId, KernelSafetyCause,
    KernelSafetyDisposition, LedgerRetirement, LifecycleImplementationSet, LifecycleProfilesV2,
    ManageRequest, ManagementRequestId, Microstep, PackLockEntry, PhysicalEvent,
    ResolveExecutionError, ResolvedExecution, RootSeed, RunAttemptStatus, RuntimeService,
    SessionManagement, SessionMode, SimMoment, SimTime, SocialState, StableCommandRejection,
    StartAttemptError, SystemCommandAdmissionOutcome, SystemCommandRequest, SystemCommandSourceId,
    TerminationContractV1, WorldRevision, baseline_lifecycle_profiles,
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

fn valid<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("conformance fixture must be valid: {error:?}"),
    }
}

fn accepted_domain(
    containers: Vec<ContainerRecord>,
    containment: Vec<ContainmentRecord>,
    authority: Vec<ContainerAuthorityRecord>,
) -> AcceptedState {
    AcceptedState::new(
        valid(DomainState::new(containers, containment, authority)),
        EpistemicState::empty(),
        SocialState::empty(),
        AgencyState::empty(),
    )
}

fn single_resolution(outcome: &AdvanceOutcome) -> Option<CommandResolution> {
    match outcome {
        AdvanceOutcome::Published { commands, .. } => {
            let [resolved] = commands.as_slice() else {
                return None;
            };
            Some(resolved.resolution())
        }
        AdvanceOutcome::KernelSafety { .. }
        | AdvanceOutcome::NoScheduledWork
        | AdvanceOutcome::NoWorkDue { .. } => None,
    }
}

fn distribution() -> EngineDistribution {
    distribution_with_policy(Arc::new(BaselineActionPolicy::new())).0
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

fn baseline_profiles() -> LifecycleProfilesV2 {
    let action =
        ActionPolicyInstallation::inline_deterministic(Arc::new(BaselineActionPolicy::new()));
    baseline_lifecycle_profiles(action.binding())
}

fn compile_standard(
    distribution: &EngineDistribution,
    engine_protocol: EngineProtocolVersion,
) -> Compilation {
    let data = transfer_artifact_data();
    let source = PackSource::new(
        SourceSnapshotId::from_bytes([0x53; 32]),
        engine_protocol,
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

fn engine_with(distribution: EngineDistribution, envelopes: Vec<ArtifactEnvelope>) -> Engine {
    let artifacts: Arc<dyn ArtifactResolver> = Arc::new(InMemoryArtifacts { envelopes });
    valid(EngineBuilder::new(distribution, artifacts, valid(RuntimeService::in_memory())).build())
}

fn resolve_origin(
    engine: &Engine,
    compilation: &Compilation,
    accepted: AcceptedState,
    termination: TerminationContractV1,
) -> ResolvedExecution {
    resolve_origin_with_profiles(
        engine,
        compilation,
        accepted,
        termination,
        baseline_profiles(),
    )
}

fn resolve_origin_with_profiles(
    engine: &Engine,
    compilation: &Compilation,
    accepted: AcceptedState,
    termination: TerminationContractV1,
    profiles: LifecycleProfilesV2,
) -> ResolvedExecution {
    resolve_origin_with_config(
        engine,
        compilation,
        accepted,
        termination,
        valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
        profiles,
    )
}

fn resolve_origin_with_config(
    engine: &Engine,
    compilation: &Compilation,
    accepted: AcceptedState,
    termination: TerminationContractV1,
    config: ExecutionConfigArtifactV3,
    profiles: LifecycleProfilesV2,
) -> ResolvedExecution {
    valid(engine.resolve_execution(ExecutionSpecInput::origin(
        compilation.definitions().lock().clone(),
        ExecutionOrigin::new(accepted, Vec::new(), SimMoment::ORIGIN, SimMoment::ORIGIN),
        profiles,
        config,
        RootSeed::from_bytes([0x61; 32]),
        termination,
    )))
}

fn resolve_origin_with_opportunities(
    engine: &Engine,
    compilation: &Compilation,
    accepted: AcceptedState,
    opportunities: Vec<ActionOpportunity>,
) -> ResolvedExecution {
    resolve_origin_with_opportunities_and_profiles(
        engine,
        compilation,
        accepted,
        opportunities,
        baseline_profiles(),
    )
}

fn resolve_origin_with_opportunities_and_profiles(
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

#[allow(clippy::too_many_arguments)]
fn accepted_state(
    actor: ActorId,
    item: EntityId,
    item_container: EntityId,
    source: EntityId,
    destination: EntityId,
    alternate: EntityId,
    destination_capacity: u32,
    source_authority: bool,
) -> AcceptedState {
    let authority = source_authority
        .then_some(ContainerAuthorityRecord::new(actor, source))
        .into_iter()
        .collect();
    accepted_domain(
        vec![
            ContainerRecord::new(source, 4),
            ContainerRecord::new(destination, destination_capacity),
            ContainerRecord::new(alternate, 4),
        ],
        vec![ContainmentRecord::new(item, item_container)],
        authority,
    )
}

#[allow(clippy::too_many_arguments)]
fn accepted_state_with_belief(
    actor: ActorId,
    item: EntityId,
    item_container: EntityId,
    believed_container: EntityId,
    source: EntityId,
    destination: EntityId,
    alternate: EntityId,
    destination_capacity: u32,
    source_authority: bool,
) -> AcceptedState {
    let state = accepted_state(
        actor,
        item,
        item_container,
        source,
        destination,
        alternate,
        destination_capacity,
        source_authority,
    );
    let prior = if believed_container == alternate {
        destination
    } else {
        alternate
    };
    let delta = valid(ContainmentTransferDelta::new(
        actor,
        item,
        prior,
        believed_container,
    ));
    let PhysicalEvent::ItemTransferred(event) = PhysicalEvent::item_transferred(delta) else {
        panic!("containment belief fixture must produce an item-transfer event")
    };
    let evidence = EvidenceRecord::direct_item_transfer(
        actor,
        EvidenceDeliveryGeneration::new(1)
            .unwrap_or_else(|| panic!("fixture evidence generation is nonzero")),
        event,
    );
    let epistemic =
        valid(EpistemicState::empty().assimilate(actor, EpistemicVersion::EMPTY, vec![evidence]));
    AcceptedState::new(
        state.domain().clone(),
        epistemic,
        *state.social(),
        state.agency().clone(),
    )
}

#[allow(clippy::too_many_arguments)]
fn transfer_request(
    input: u64,
    effective: SimMoment,
    command_source: CommandSource,
    command: u64,
    actor: ActorId,
    bound_actor: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
) -> SystemCommandRequest {
    SystemCommandRequest::new(
        InputId::new(input),
        effective,
        SystemCommandSourceId::from_bytes(command_source.into_bytes()),
        CommandId::new(command),
        actor,
        transfer_action_key(),
        vec![
            CommandBinding::new(
                valid(world_engine::BindingName::parse("actor")),
                CommandValue::Actor(bound_actor),
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

fn derived_system_command_source(source: CommandSource) -> CommandSource {
    CommandSource::derive_system(SystemCommandSourceId::from_bytes(source.into_bytes()))
}

fn transfer_opportunity(
    actor: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
) -> ActionOpportunity {
    ActionOpportunity::open(
        actor,
        ActionSponsor::actor_reaction(ActorReactionCause::from_bytes([0x51; 32])),
        ActionInteractionScope::containment(valid(ContainmentInteractionScope::new(
            source,
            vec![destination],
            vec![item],
            8,
        ))),
        ActionOpportunityGeneration::new(0),
    )
}

struct SelectingActionPolicy {
    semantics: ActionPolicySemanticsId,
    observed: std::sync::Mutex<Option<(ActionContextPayload, ActionDecision)>>,
}

impl SelectingActionPolicy {
    fn new(semantics: ActionPolicySemanticsId) -> Self {
        Self {
            semantics,
            observed: std::sync::Mutex::new(None),
        }
    }

    fn observed(&self) -> (ActionContextPayload, ActionDecision) {
        self.observed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .unwrap_or_else(|| panic!("manual action policy must receive one payload"))
    }
}

impl ActionPolicy for SelectingActionPolicy {
    fn semantics_id(&self) -> ActionPolicySemanticsId {
        self.semantics
    }

    fn decide(&self, input: &ActionContextPayload) -> Result<ActionDecision, ActionPolicyError> {
        let candidate = input
            .candidates()
            .candidates()
            .first()
            .map(|candidate| candidate.id())
            .ok_or(ActionPolicyError::EvaluationFailed)?;
        let decision = ActionDecision::Select {
            candidate,
            input: input.input_fingerprint(),
        };
        *self
            .observed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((input.clone(), decision));
        Ok(decision)
    }
}

#[test]
fn standard_transfer_uses_only_public_authority_and_read_facades() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state(actor, item, source, source, destination, alternate, 2, true);
    let expected = accepted_state(
        actor,
        item,
        destination,
        source,
        destination,
        alternate,
        2,
        true,
    );
    let execution = resolve_origin(
        &engine,
        &compilation,
        initial,
        TerminationContractV1::never(),
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x71; 32])));

    assert_eq!(attempt.binding().execution(), execution.execution_id());
    assert_eq!(
        attempt.binding().initial_root(),
        execution.initial_root_id()
    );
    assert_eq!(attempt.binding().lineage(), execution.epoch_lineage_id());
    assert_eq!(valid(attempt.status()), RunAttemptStatus::Active);

    let effective = SimMoment::at(SimTime::from_ticks(8));
    let request = transfer_request(
        1,
        effective,
        CommandSource::from_bytes([0x81; 32]),
        1,
        actor,
        actor,
        item,
        source,
        destination,
    );
    let admission = valid(attempt.submit_system_command(request.clone()));
    assert!(matches!(
        admission,
        SystemCommandAdmissionOutcome::Scheduled {
            effective: scheduled,
            ..
        } if scheduled == effective
    ));

    let published = valid(attempt.advance(AdvanceRequest::through(effective)));
    let AdvanceOutcome::Published {
        commands, cursor, ..
    } = published
    else {
        panic!("the transfer command must publish");
    };
    let [resolved] = commands.as_slice() else {
        panic!("the transfer moment must resolve exactly one command delivery");
    };
    assert_eq!(
        resolved.resolution(),
        CommandResolution::New(CommandAttemptOutcome::Accepted)
    );

    let session = attempt.session();
    let snapshot = valid(session.snapshot());
    assert_eq!(snapshot.revision(), WorldRevision::from_raw(2));
    assert_eq!(snapshot.accepted(), &expected);
    assert_eq!(valid(session.cursor()), cursor);
    let inspection = valid(session.inspector().direct_container(item));
    assert_eq!(inspection.revision(), snapshot.revision());
    assert_eq!(inspection.container(), Some(destination));

    let cursor_before_retry = valid(session.cursor());
    assert_eq!(valid(attempt.submit_system_command(request)), admission);
    assert_eq!(valid(session.cursor()), cursor_before_retry);
    assert!(matches!(
        attempt.submit_system_command(transfer_request(
            1,
            effective,
            CommandSource::from_bytes([0x81; 32]),
            1,
            actor,
            actor,
            item,
            source,
            alternate,
        )),
        Err(AttemptError::InputIdReuse)
    ));
    assert_eq!(valid(session.cursor()), cursor_before_retry);

    let Some(post_commit_moment) = effective.checked_next_microstep() else {
        panic!("fixture moment must have a causal successor");
    };
    assert!(matches!(
        valid(attempt.advance(AdvanceRequest::through(post_commit_moment))),
        AdvanceOutcome::Published {
            moment,
            commands,
            post_commit_consumed: 1,
            ..
        } if moment == post_commit_moment && commands.is_empty()
    ));
}

#[test]
fn baseline_actor_control_lowers_privately_and_consumes_its_neutral_wake() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state_with_belief(
        actor,
        item,
        source,
        source,
        source,
        destination,
        alternate,
        2,
        true,
    );
    let expected = accepted_state_with_belief(
        actor,
        item,
        destination,
        source,
        source,
        destination,
        alternate,
        2,
        true,
    );
    let opportunity = transfer_opportunity(actor, item, source, destination);
    let opportunity_id = opportunity.id();
    let execution = resolve_origin_with_opportunities(
        &engine,
        &compilation,
        initial.clone(),
        vec![opportunity],
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0xb1; 32])));

    let AdvanceOutcome::Published {
        moment,
        commands,
        post_commit_consumed,
        action_opportunities_consumed,
        attempt_resolved,
        ..
    } = valid(attempt.advance(AdvanceRequest::through(SimMoment::ORIGIN)))
    else {
        panic!("the origin action opportunity must publish");
    };
    assert_eq!(moment, SimMoment::ORIGIN);
    assert!(commands.is_empty());
    assert_eq!(post_commit_consumed, 0);
    assert_eq!(action_opportunities_consumed, vec![opportunity_id]);
    assert!(attempt_resolved.is_empty());
    assert_eq!(valid(attempt.session().snapshot()).accepted(), &initial);

    let action_moment = SimMoment::ORIGIN
        .checked_next_microstep()
        .unwrap_or_else(|| panic!("origin must have a following microstep"));
    let AdvanceOutcome::Published {
        moment,
        commands,
        post_commit_consumed,
        action_opportunities_consumed,
        attempt_resolved,
        ..
    } = valid(attempt.advance(AdvanceRequest::through(action_moment)))
    else {
        panic!("the privately lowered actor command must publish");
    };
    assert_eq!(moment, action_moment);
    assert!(commands.is_empty());
    assert_eq!(post_commit_consumed, 0);
    assert!(action_opportunities_consumed.is_empty());
    assert!(attempt_resolved.is_empty());
    assert_eq!(valid(attempt.session().snapshot()).accepted(), &expected);

    let wake_moment = action_moment
        .checked_next_microstep()
        .unwrap_or_else(|| panic!("actor command must have a following microstep"));
    let AdvanceOutcome::Published {
        moment,
        commands,
        post_commit_consumed,
        action_opportunities_consumed,
        attempt_resolved,
        ..
    } = valid(attempt.advance(AdvanceRequest::through(wake_moment)))
    else {
        panic!("the neutral attempt-resolution wake must publish");
    };
    assert_eq!(moment, wake_moment);
    assert!(commands.is_empty());
    assert_eq!(post_commit_consumed, 1);
    assert!(action_opportunities_consumed.is_empty());
    assert_eq!(attempt_resolved, vec![opportunity_id]);
}

#[test]
fn manual_action_policy_selects_only_from_its_exact_actor_safe_payload() {
    let semantics = ActionPolicySemanticsId::from_bytes([0x91; 32]);
    let policy = Arc::new(SelectingActionPolicy::new(semantics));
    let (distribution, profiles) = distribution_with_policy(policy.clone());
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x12; 32]);
    let item = EntityId::from_bytes([0x22; 32]);
    let source = EntityId::from_bytes([0x32; 32]);
    let destination = EntityId::from_bytes([0x43; 32]);
    let alternate = EntityId::from_bytes([0x44; 32]);
    let initial = accepted_state_with_belief(
        actor,
        item,
        source,
        source,
        source,
        destination,
        alternate,
        2,
        true,
    );
    let expected = accepted_state_with_belief(
        actor,
        item,
        destination,
        source,
        source,
        destination,
        alternate,
        2,
        true,
    );
    let opportunity = transfer_opportunity(actor, item, source, destination);
    let opportunity_id = opportunity.id();
    let execution = resolve_origin_with_opportunities_and_profiles(
        &engine,
        &compilation,
        initial,
        vec![opportunity],
        profiles,
    );
    assert_eq!(execution.action_policy_semantics(), semantics);
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0xb2; 32])));

    let AdvanceOutcome::Published {
        commands,
        action_opportunities_consumed,
        ..
    } = valid(attempt.advance(AdvanceRequest::through(SimMoment::ORIGIN)))
    else {
        panic!("manual actor selection must consume the origin opportunity");
    };
    assert!(commands.is_empty());
    assert_eq!(action_opportunities_consumed, vec![opportunity_id]);

    let (payload, decision) = policy.observed();
    assert_eq!(payload.actor(), actor);
    assert_eq!(payload.opportunity(), opportunity_id);
    assert_eq!(payload.policy_semantics(), semantics);
    assert_eq!(payload.candidates().opportunity(), opportunity_id);
    assert_eq!(payload.candidates().candidate_limit(), 8);
    let interaction = payload
        .interaction()
        .containment()
        .unwrap_or_else(|| panic!("transfer policy input must expose containment"));
    assert_eq!(interaction.destinations().len(), 1);
    assert_eq!(interaction.items().len(), 1);
    let [candidate] = payload.candidates().candidates() else {
        panic!("the bounded actor-safe payload must supply one transfer candidate");
    };
    assert_eq!(candidate.opportunity(), opportunity_id);
    assert_eq!(candidate.action(), &transfer_action_key());
    assert_eq!(
        candidate
            .bindings()
            .iter()
            .map(|binding| binding.name().as_str())
            .collect::<Vec<_>>(),
        ["actor", "destination", "item", "source"]
    );
    assert_eq!(
        decision,
        ActionDecision::Select {
            candidate: candidate.id(),
            input: payload.input_fingerprint(),
        }
    );

    let action_moment = SimMoment::ORIGIN
        .checked_next_microstep()
        .unwrap_or_else(|| panic!("origin must have a following microstep"));
    let AdvanceOutcome::Published {
        commands,
        action_opportunities_consumed,
        attempt_resolved,
        ..
    } = valid(attempt.advance(AdvanceRequest::through(action_moment)))
    else {
        panic!("the manually selected action must use the actor lowering path");
    };
    assert!(commands.is_empty());
    assert!(action_opportunities_consumed.is_empty());
    assert!(attempt_resolved.is_empty());
    assert_eq!(valid(attempt.session().snapshot()).accepted(), &expected);
}

#[test]
fn rejected_transfer_retry_and_command_id_reuse_are_inert() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state(
        actor,
        item,
        source,
        source,
        destination,
        alternate,
        2,
        false,
    );
    let execution = resolve_origin(
        &engine,
        &compilation,
        initial.clone(),
        TerminationContractV1::never(),
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x72; 32])));
    let command_source = CommandSource::from_bytes([0x82; 32]);

    let first_due = SimMoment::at(SimTime::from_ticks(8));
    valid(attempt.submit_system_command(transfer_request(
        1,
        first_due,
        command_source,
        1,
        actor,
        actor,
        item,
        source,
        destination,
    )));
    let first = valid(attempt.advance(AdvanceRequest::through(first_due)));
    assert_eq!(
        single_resolution(&first),
        Some(CommandResolution::New(CommandAttemptOutcome::Rejected(
            StableCommandRejection::RequirementUnsatisfied
        )))
    );
    assert_eq!(valid(attempt.session().snapshot()).accepted(), &initial);

    let retained_due = SimMoment::at(SimTime::from_ticks(9));
    valid(attempt.submit_system_command(transfer_request(
        2,
        retained_due,
        command_source,
        1,
        actor,
        actor,
        item,
        source,
        destination,
    )));
    let retained = valid(attempt.advance(AdvanceRequest::through(retained_due)));
    assert_eq!(
        single_resolution(&retained),
        Some(CommandResolution::Retained(
            CommandAttemptOutcome::Rejected(StableCommandRejection::RequirementUnsatisfied)
        ))
    );
    assert_eq!(valid(attempt.session().snapshot()).accepted(), &initial);

    let mismatch_due = SimMoment::at(SimTime::from_ticks(10));
    valid(attempt.submit_system_command(transfer_request(
        3,
        mismatch_due,
        command_source,
        1,
        actor,
        actor,
        item,
        source,
        alternate,
    )));
    let mismatch = valid(attempt.advance(AdvanceRequest::through(mismatch_due)));
    assert_eq!(
        single_resolution(&mismatch),
        Some(CommandResolution::IdReuseMismatch)
    );
    assert_eq!(valid(attempt.session().snapshot()).accepted(), &initial);
    assert_eq!(
        valid(attempt.advance(AdvanceRequest::through(mismatch_due))),
        AdvanceOutcome::NoScheduledWork
    );
}

#[test]
fn actor_role_mismatch_is_a_modeled_rejection() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let bound_actor = ActorId::from_bytes([0x12; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state(actor, item, source, source, destination, alternate, 2, true);
    let execution = resolve_origin(
        &engine,
        &compilation,
        initial.clone(),
        TerminationContractV1::never(),
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x73; 32])));
    let due = SimMoment::at(SimTime::from_ticks(8));
    valid(attempt.submit_system_command(transfer_request(
        1,
        due,
        CommandSource::from_bytes([0x83; 32]),
        1,
        actor,
        bound_actor,
        item,
        source,
        destination,
    )));

    let published = valid(attempt.advance(AdvanceRequest::through(due)));
    assert_eq!(
        single_resolution(&published),
        Some(CommandResolution::New(CommandAttemptOutcome::Rejected(
            StableCommandRejection::BindingMismatch
        )))
    );
    assert_eq!(valid(attempt.session().snapshot()).accepted(), &initial);
}

#[test]
fn malformed_system_command_binding_is_rejected_without_ingress() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let execution = resolve_origin(
        &engine,
        &compilation,
        accepted_state(actor, item, source, source, destination, alternate, 2, true),
        TerminationContractV1::never(),
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x77; 32])));
    let session = attempt.session();
    let initial_cursor = valid(session.cursor());
    let initial_snapshot = valid(session.snapshot());
    let malformed = SystemCommandRequest::new(
        InputId::new(1),
        SimMoment::at(SimTime::from_ticks(8)),
        SystemCommandSourceId::from_bytes([0x87; 32]),
        CommandId::new(1),
        actor,
        transfer_action_key(),
        vec![
            CommandBinding::new(
                valid(world_engine::BindingName::parse("actor")),
                CommandValue::Actor(actor),
            ),
            CommandBinding::new(
                valid(world_engine::BindingName::parse("destination")),
                CommandValue::Entity(destination),
            ),
            CommandBinding::new(
                valid(world_engine::BindingName::parse("item")),
                CommandValue::Entity(item),
            ),
        ],
    );

    assert!(matches!(
        attempt.submit_system_command(malformed),
        Err(AttemptError::InvalidSystemCommand(_))
    ));
    assert_eq!(valid(session.cursor()), initial_cursor);
    assert_eq!(valid(session.snapshot()), initial_snapshot);
    assert_eq!(
        valid(
            attempt.advance(AdvanceRequest::through(SimMoment::at(SimTime::from_ticks(
                8
            ))))
        ),
        AdvanceOutcome::NoScheduledWork
    );
}

#[test]
fn advance_before_the_least_due_moment_is_inert() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state(actor, item, source, source, destination, alternate, 2, true);
    let execution = resolve_origin(
        &engine,
        &compilation,
        initial,
        TerminationContractV1::never(),
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x78; 32])));
    let due = SimMoment::at(SimTime::from_ticks(8));
    valid(attempt.submit_system_command(transfer_request(
        1,
        due,
        CommandSource::from_bytes([0x88; 32]),
        1,
        actor,
        actor,
        item,
        source,
        destination,
    )));
    let session = attempt.session();
    let cursor_after_ingress = valid(session.cursor());
    let snapshot_after_ingress = valid(session.snapshot());
    let through = SimMoment::at(SimTime::from_ticks(7));

    assert_eq!(
        valid(attempt.advance(AdvanceRequest::through(through))),
        AdvanceOutcome::NoWorkDue { next: due, through }
    );
    assert_eq!(valid(session.cursor()), cursor_after_ingress);
    assert_eq!(valid(session.snapshot()), snapshot_after_ingress);
    let published = valid(attempt.advance(AdvanceRequest::through(due)));
    assert_eq!(
        single_resolution(&published),
        Some(CommandResolution::New(CommandAttemptOutcome::Accepted))
    );
}

#[test]
fn same_moment_commands_publish_as_one_resolved_batch() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let execution = resolve_origin(
        &engine,
        &compilation,
        accepted_state(actor, item, source, source, destination, alternate, 2, true),
        TerminationContractV1::never(),
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x79; 32])));
    let due = SimMoment::at(SimTime::from_ticks(8));
    valid(attempt.submit_system_command(transfer_request(
        1,
        due,
        CommandSource::from_bytes([0x89; 32]),
        1,
        actor,
        actor,
        item,
        source,
        destination,
    )));
    valid(attempt.submit_system_command(transfer_request(
        2,
        due,
        CommandSource::from_bytes([0x8a; 32]),
        2,
        actor,
        actor,
        item,
        source,
        alternate,
    )));
    let session = attempt.session();
    let published = valid(attempt.advance(AdvanceRequest::through(due)));
    let AdvanceOutcome::Published {
        commands, moment, ..
    } = published
    else {
        panic!("the complete same-moment command set must publish");
    };
    assert_eq!(moment, due);
    assert_eq!(commands.len(), 2);
    assert_eq!(
        commands
            .iter()
            .filter(|delivery| {
                delivery.resolution() == CommandResolution::New(CommandAttemptOutcome::Accepted)
            })
            .count(),
        1
    );
    assert_eq!(
        commands
            .iter()
            .filter(|delivery| {
                delivery.resolution()
                    == CommandResolution::New(CommandAttemptOutcome::Rejected(
                        StableCommandRejection::Conflict,
                    ))
            })
            .count(),
        1
    );
    let accepted_source = commands
        .iter()
        .find(|delivery| {
            delivery.resolution() == CommandResolution::New(CommandAttemptOutcome::Accepted)
        })
        .map(|delivery| delivery.source())
        .unwrap_or_else(|| panic!("one same-moment command must win"));
    let expected_container = if accepted_source
        == derived_system_command_source(CommandSource::from_bytes([0x89; 32]))
    {
        destination
    } else {
        alternate
    };
    assert_eq!(
        valid(session.inspector().direct_container(item)).container(),
        Some(expected_container)
    );
}

#[test]
fn contested_and_disjoint_transfers_are_ingress_permutation_invariant() {
    type CommandOutcome = (CommandSource, CommandId, CommandResolution);
    type PermutationResult = (Vec<CommandOutcome>, AcceptedState);

    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let contested_item = EntityId::from_bytes([0x21; 32]);
    let disjoint_item = EntityId::from_bytes([0x22; 32]);
    let contested_source = EntityId::from_bytes([0x31; 32]);
    let disjoint_source = EntityId::from_bytes([0x32; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let disjoint_destination = EntityId::from_bytes([0x43; 32]);
    let initial = accepted_domain(
        vec![
            ContainerRecord::new(contested_source, 4),
            ContainerRecord::new(disjoint_source, 4),
            ContainerRecord::new(destination, 2),
            ContainerRecord::new(alternate, 2),
            ContainerRecord::new(disjoint_destination, 2),
        ],
        vec![
            ContainmentRecord::new(contested_item, contested_source),
            ContainmentRecord::new(disjoint_item, disjoint_source),
        ],
        vec![
            ContainerAuthorityRecord::new(actor, contested_source),
            ContainerAuthorityRecord::new(actor, disjoint_source),
        ],
    );
    let execution = resolve_origin(
        &engine,
        &compilation,
        initial,
        TerminationContractV1::never(),
    );
    let due = SimMoment::at(SimTime::from_ticks(12));
    let contested_destination_source = CommandSource::from_bytes([0x91; 32]);
    let contested_alternate_source = CommandSource::from_bytes([0x92; 32]);
    let disjoint_command_source = CommandSource::from_bytes([0x93; 32]);
    let requests = [
        transfer_request(
            0,
            due,
            contested_destination_source,
            1,
            actor,
            actor,
            contested_item,
            contested_source,
            destination,
        ),
        transfer_request(
            1,
            due,
            contested_alternate_source,
            1,
            actor,
            actor,
            contested_item,
            contested_source,
            alternate,
        ),
        transfer_request(
            2,
            due,
            disjoint_command_source,
            1,
            actor,
            actor,
            disjoint_item,
            disjoint_source,
            disjoint_destination,
        ),
    ];
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let mut expected: Option<PermutationResult> = None;

    for (index, permutation) in permutations.into_iter().enumerate() {
        let key_byte = 0xa0_u8
            .checked_add(u8::try_from(index).unwrap_or_else(|_| unreachable!()))
            .unwrap_or_else(|| unreachable!());
        let mut attempt =
            valid(engine.start_attempt(&execution, AttemptKey::from_bytes([key_byte; 32])));
        for request_index in permutation {
            valid(attempt.submit_system_command(requests[request_index].clone()));
        }
        let session = attempt.session();
        let AdvanceOutcome::Published {
            moment, commands, ..
        } = valid(attempt.advance(AdvanceRequest::through(due)))
        else {
            panic!("the complete three-command moment must publish");
        };
        assert_eq!(moment, due);
        assert_eq!(commands.len(), 3);
        assert_eq!(
            commands
                .iter()
                .filter(|delivery| {
                    delivery.resolution() == CommandResolution::New(CommandAttemptOutcome::Accepted)
                })
                .count(),
            2
        );
        assert_eq!(
            commands
                .iter()
                .filter(|delivery| {
                    delivery.resolution()
                        == CommandResolution::New(CommandAttemptOutcome::Rejected(
                            StableCommandRejection::Conflict,
                        ))
                })
                .count(),
            1
        );
        assert!(commands.iter().any(|delivery| {
            delivery.source() == derived_system_command_source(disjoint_command_source)
                && delivery.resolution() == CommandResolution::New(CommandAttemptOutcome::Accepted)
        }));
        assert_eq!(
            valid(session.inspector().direct_container(disjoint_item)).container(),
            Some(disjoint_destination)
        );

        let outcomes = commands
            .iter()
            .map(|delivery| (delivery.source(), delivery.command(), delivery.resolution()))
            .collect::<Vec<_>>();
        let successor = valid(session.snapshot()).accepted().clone();
        match &expected {
            Some((expected_outcomes, expected_successor)) => {
                assert_eq!(&outcomes, expected_outcomes);
                assert_eq!(&successor, expected_successor);
            }
            None => expected = Some((outcomes, successor)),
        }
    }
}

#[test]
fn public_advance_reports_candidate_quarantine_and_preserves_the_due_set() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state(actor, item, source, source, destination, alternate, 2, true);
    let execution = resolve_origin_with_config(
        &engine,
        &compilation,
        initial.clone(),
        TerminationContractV1::never(),
        valid(ExecutionConfigArtifactV3::inline(2, 1, 16)),
        baseline_profiles(),
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x7a; 32])));
    let due = SimMoment::at(SimTime::from_ticks(8));
    valid(attempt.submit_system_command(transfer_request(
        1,
        due,
        CommandSource::from_bytes([0x8b; 32]),
        1,
        actor,
        actor,
        item,
        source,
        destination,
    )));
    valid(attempt.submit_system_command(transfer_request(
        2,
        due,
        CommandSource::from_bytes([0x8c; 32]),
        2,
        actor,
        actor,
        item,
        source,
        alternate,
    )));

    let session = attempt.session();
    let outcome = valid(attempt.advance(AdvanceRequest::through(due)));
    let AdvanceOutcome::KernelSafety {
        cursor,
        cause,
        disposition,
        ..
    } = outcome
    else {
        panic!("candidate excess must publish a modeled safety transition");
    };
    let KernelSafetyCause::EvaluableCommandPopulationExceeded {
        limit,
        observed,
        evidence,
    } = cause
    else {
        panic!("candidate excess must retain its exact cause family");
    };
    assert_eq!(limit.get(), 1);
    assert_eq!(observed, 2);
    assert_eq!(evidence.due(), due);
    assert_eq!(evidence.due_count().get(), 2);
    assert_eq!(disposition, KernelSafetyDisposition::Quarantined);

    let read = valid(session.read());
    assert_eq!(read.cursor(), cursor);
    assert_eq!(read.mode(), SessionMode::Quarantined);
    assert_eq!(read.snapshot().accepted(), &initial);
    assert_eq!(
        read.safety_blocker().map(|blocker| blocker.cause()),
        Some(cause)
    );
}

#[test]
fn wave_pause_resume_preserves_and_then_consumes_post_commit_work() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let execution = resolve_origin_with_config(
        &engine,
        &compilation,
        accepted_state(actor, item, source, source, destination, alternate, 2, true),
        TerminationContractV1::never(),
        valid(ExecutionConfigArtifactV3::inline(64, 32, 1)),
        baseline_profiles(),
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0xa6; 32])));
    let due = SimMoment::at(SimTime::from_ticks(20));
    valid(attempt.submit_system_command(transfer_request(
        0,
        due,
        CommandSource::from_bytes([0x94; 32]),
        1,
        actor,
        actor,
        item,
        source,
        destination,
    )));
    let session = attempt.session();
    assert!(matches!(
        valid(attempt.advance(AdvanceRequest::through(due))),
        AdvanceOutcome::Published {
            moment,
            post_commit_consumed: 0,
            ref commands,
            ..
        } if moment == due
            && commands.len() == 1
            && commands[0].resolution()
                == CommandResolution::New(CommandAttemptOutcome::Accepted)
    ));

    let post_commit_moment = SimMoment::new(due.time(), Microstep::new(1));
    let before_safety = valid(session.read());
    assert_eq!(before_safety.same_time_wave_tranche().time(), due.time());
    assert_eq!(before_safety.same_time_wave_tranche().completed_waves(), 1);
    let AdvanceOutcome::KernelSafety {
        cause, disposition, ..
    } = valid(attempt.advance(AdvanceRequest::through(post_commit_moment)))
    else {
        panic!("the second same-time wave must publish a pause transition");
    };
    let KernelSafetyCause::SameTimeWaveExhausted {
        limit,
        attempted_wave,
        evidence,
    } = cause
    else {
        panic!("the wave bound must retain its exact cause");
    };
    assert_eq!(limit.get(), 1);
    assert_eq!(attempted_wave, 2);
    assert_eq!(evidence.due(), post_commit_moment);
    assert_eq!(disposition, KernelSafetyDisposition::Paused);

    let paused = valid(session.read());
    assert_eq!(paused.mode(), SessionMode::Paused);
    assert_eq!(
        paused.safety_blocker().map(|blocker| blocker.cause()),
        Some(cause)
    );
    assert_eq!(paused.same_time_wave_tranche().time(), due.time());
    assert_eq!(paused.same_time_wave_tranche().completed_waves(), 1);

    let resume = ManageRequest::new(ManagementRequestId::new(0), SessionManagement::Resume);
    let resumed = valid(attempt.submit_management_request(resume));
    assert_eq!(resumed.resulting_mode(), Some(SessionMode::Running));
    let resumed_cursor = valid(session.cursor());
    assert_eq!(valid(attempt.submit_management_request(resume)), resumed);
    assert_eq!(valid(session.cursor()), resumed_cursor);
    let running = valid(session.read());
    assert_eq!(running.mode(), SessionMode::Running);
    assert_eq!(running.safety_blocker(), None);
    assert_eq!(running.same_time_wave_tranche().time(), due.time());
    assert_eq!(running.same_time_wave_tranche().completed_waves(), 0);

    assert!(matches!(
        valid(attempt.advance(AdvanceRequest::through(post_commit_moment))),
        AdvanceOutcome::Published {
            moment,
            ref commands,
            post_commit_consumed: 1,
            ..
        } if moment == post_commit_moment && commands.is_empty()
    ));
}

#[test]
fn admission_population_limit_is_typed_and_atomic() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let execution = resolve_origin_with_config(
        &engine,
        &compilation,
        accepted_state(actor, item, source, source, destination, alternate, 2, true),
        TerminationContractV1::never(),
        valid(ExecutionConfigArtifactV3::inline(1, 1, 16)),
        baseline_profiles(),
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x7b; 32])));
    let due = SimMoment::at(SimTime::from_ticks(8));
    valid(attempt.submit_system_command(transfer_request(
        1,
        due,
        CommandSource::from_bytes([0x8d; 32]),
        1,
        actor,
        actor,
        item,
        source,
        destination,
    )));
    let session = attempt.session();
    let cursor = valid(session.cursor());
    let snapshot = valid(session.snapshot());

    assert!(matches!(
        attempt.submit_system_command(transfer_request(
            2,
            due,
            CommandSource::from_bytes([0x8e; 32]),
            2,
            actor,
            actor,
            item,
            source,
            alternate,
        )),
        Err(AttemptError::MomentPopulationExceeded {
            moment,
            maximum: 1,
            actual: 2,
        }) if moment == due
    ));
    assert_eq!(valid(session.cursor()), cursor);
    assert_eq!(valid(session.snapshot()), snapshot);
    assert_eq!(
        single_resolution(&valid(attempt.advance(AdvanceRequest::through(due)))),
        Some(CommandResolution::New(CommandAttemptOutcome::Accepted))
    );
}

#[test]
fn public_retirement_and_admission_sealing_preserve_request_boundaries() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let execution = resolve_origin(
        &engine,
        &compilation,
        accepted_state(actor, item, source, source, destination, alternate, 2, true),
        TerminationContractV1::never(),
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0xa7; 32])));
    let due = SimMoment::at(SimTime::from_ticks(30));
    let captured = transfer_request(
        0,
        due,
        CommandSource::from_bytes([0x95; 32]),
        1,
        actor,
        actor,
        item,
        source,
        destination,
    );
    valid(attempt.submit_system_command(captured.clone()));
    let session = attempt.session();

    let retirement = LedgerRetirement::InputThrough(InputId::new(0));
    let retire = ManageRequest::new(
        ManagementRequestId::new(0),
        SessionManagement::Retire(retirement),
    );
    let retired = valid(attempt.submit_management_request(retire));
    assert_eq!(retired.retirement(), Some(retirement));
    let retired_cursor = valid(session.cursor());
    assert_eq!(valid(attempt.submit_management_request(retire)), retired);
    assert_eq!(valid(session.cursor()), retired_cursor);
    assert!(matches!(
        attempt.submit_system_command(captured),
        Err(AttemptError::InputRetired { id }) if id == InputId::new(0)
    ));

    let frontier_before = valid(session.read()).admission_frontier();
    let seal = ManageRequest::new(
        ManagementRequestId::new(1),
        SessionManagement::SealAdmissionThrough(due),
    );
    let sealed = valid(attempt.submit_management_request(seal));
    assert_eq!(sealed.admission_frontier(), Some(due));
    let sealed_cursor = valid(session.cursor());
    assert_eq!(valid(attempt.submit_management_request(seal)), sealed);
    assert_eq!(valid(session.cursor()), sealed_cursor);
    let frontier_after = valid(session.read()).admission_frontier();
    assert!(frontier_after > frontier_before);
    assert_eq!(frontier_after, due);

    let backdated = SimMoment::at(SimTime::from_ticks(29));
    assert!(matches!(
        attempt.submit_system_command(transfer_request(
            1,
            backdated,
            CommandSource::from_bytes([0x96; 32]),
            2,
            actor,
            actor,
            item,
            source,
            alternate,
        )),
        Err(AttemptError::EffectiveMomentBeforeFrontier {
            effective,
            frontier,
        }) if effective == backdated && frontier == due
    ));
    assert_eq!(valid(session.cursor()), sealed_cursor);
    assert_eq!(valid(session.read()).admission_frontier(), due);
}

#[test]
fn repeated_public_execution_has_identical_semantic_history() {
    let first_distribution = distribution();
    let second_distribution = distribution();
    let first_compilation =
        compile_standard(&first_distribution, first_distribution.engine_protocol());
    let second_compilation =
        compile_standard(&second_distribution, second_distribution.engine_protocol());
    assert_eq!(
        first_compilation.definitions().digest(),
        second_compilation.definitions().digest()
    );
    assert_eq!(
        first_compilation.envelopes(),
        second_compilation.envelopes()
    );

    let first_engine = engine_with(first_distribution, first_compilation.envelopes().to_vec());
    let second_engine = engine_with(second_distribution, second_compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state(actor, item, source, source, destination, alternate, 2, true);
    let effective = SimMoment::at(SimTime::from_ticks(8));
    let termination = TerminationContractV1::at_or_after_moment(effective);
    let first_execution = resolve_origin(
        &first_engine,
        &first_compilation,
        initial.clone(),
        termination,
    );
    let second_execution =
        resolve_origin(&second_engine, &second_compilation, initial, termination);
    assert_eq!(
        first_execution.execution_id(),
        second_execution.execution_id()
    );
    assert_eq!(
        first_execution.initial_root_id(),
        second_execution.initial_root_id()
    );
    assert_eq!(
        first_execution.semantics_digest(),
        second_execution.semantics_digest()
    );
    assert_eq!(
        first_execution.closure_digest(),
        second_execution.closure_digest()
    );

    let attempt_key = AttemptKey::from_bytes([0x74; 32]);
    let mut first_attempt = valid(first_engine.start_attempt(&first_execution, attempt_key));
    let mut second_attempt = valid(second_engine.start_attempt(&second_execution, attempt_key));
    assert_ne!(first_attempt.id(), second_attempt.id());

    let request = transfer_request(
        1,
        effective,
        CommandSource::from_bytes([0x84; 32]),
        1,
        actor,
        actor,
        item,
        source,
        destination,
    );
    assert_eq!(
        valid(first_attempt.submit_system_command(request.clone())),
        valid(second_attempt.submit_system_command(request))
    );
    assert_eq!(
        valid(first_attempt.advance(AdvanceRequest::through(effective))),
        valid(second_attempt.advance(AdvanceRequest::through(effective)))
    );
    let first_snapshot = valid(first_attempt.session().snapshot());
    let second_snapshot = valid(second_attempt.session().snapshot());
    assert_eq!(first_snapshot, second_snapshot);
    assert_eq!(
        first_snapshot.accepted().digest(),
        second_snapshot.accepted().digest()
    );

    let first_finalization = match valid(first_attempt.status()) {
        RunAttemptStatus::Finalized(finalization) => *finalization,
        status => panic!("first reproduction must finalize, found {status:?}"),
    };
    let second_finalization = match valid(second_attempt.status()) {
        RunAttemptStatus::Finalized(finalization) => *finalization,
        status => panic!("second reproduction must finalize, found {status:?}"),
    };
    assert_ne!(first_finalization.attempt(), second_finalization.attempt());
    assert_eq!(
        first_finalization.terminal(),
        second_finalization.terminal()
    );
    assert_eq!(
        first_finalization.trajectory(),
        second_finalization.trajectory()
    );
    assert_eq!(first_finalization.cause(), second_finalization.cause());
}

#[test]
fn action_policy_semantics_change_the_resolved_execution_semantics() {
    let first_policy = ActionPolicySemanticsId::from_bytes([0xa1; 32]);
    let second_policy = ActionPolicySemanticsId::from_bytes([0xa2; 32]);
    let (first_distribution, first_profiles) =
        distribution_with_policy(Arc::new(SelectingActionPolicy::new(first_policy)));
    let compilation = compile_standard(&first_distribution, first_distribution.engine_protocol());
    let (second_distribution, second_profiles) =
        distribution_with_policy(Arc::new(SelectingActionPolicy::new(second_policy)));
    let first_engine = engine_with(first_distribution, compilation.envelopes().to_vec());
    let second_engine = engine_with(second_distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state(actor, item, source, source, destination, alternate, 2, true);
    let first_execution = resolve_origin_with_profiles(
        &first_engine,
        &compilation,
        initial.clone(),
        TerminationContractV1::never(),
        first_profiles,
    );
    let second_execution = resolve_origin_with_profiles(
        &second_engine,
        &compilation,
        initial,
        TerminationContractV1::never(),
        second_profiles,
    );

    assert_eq!(first_execution.action_policy_semantics(), first_policy);
    assert_eq!(second_execution.action_policy_semantics(), second_policy);
    assert_eq!(
        first_execution.initial_root_id(),
        second_execution.initial_root_id()
    );
    assert_eq!(
        first_execution.definition_set_digest(),
        second_execution.definition_set_digest()
    );
    assert_ne!(
        first_execution.semantics_digest(),
        second_execution.semantics_digest()
    );
    assert_ne!(
        first_execution.execution_id(),
        second_execution.execution_id()
    );
    assert_ne!(
        first_execution.closure_digest(),
        second_execution.closure_digest()
    );
}

#[test]
fn altered_artifact_fails_owner_validation_before_activation() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let (descriptor, mut blob) = compilation.envelopes()[0].clone().into_parts();
    match blob.first_mut() {
        Some(byte) => *byte ^= 0xff,
        None => panic!("compiled artifact must contain bytes"),
    }
    let engine = engine_with(distribution, vec![ArtifactEnvelope::new(descriptor, blob)]);
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state(actor, item, source, source, destination, alternate, 2, true);

    assert!(matches!(
        engine.resolve_execution(ExecutionSpecInput::origin(
            compilation.definitions().lock().clone(),
            ExecutionOrigin::new(initial, Vec::new(), SimMoment::ORIGIN, SimMoment::ORIGIN),
            baseline_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            RootSeed::from_bytes([0x61; 32]),
            TerminationContractV1::never(),
        )),
        Err(ResolveExecutionError::ArtifactValidation { .. })
    ));
}

#[test]
fn missing_artifact_fails_resolution_before_activation() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, Vec::new());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state(actor, item, source, source, destination, alternate, 2, true);

    assert!(matches!(
        engine.resolve_execution(ExecutionSpecInput::origin(
            compilation.definitions().lock().clone(),
            ExecutionOrigin::new(initial, Vec::new(), SimMoment::ORIGIN, SimMoment::ORIGIN),
            baseline_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            RootSeed::from_bytes([0x61; 32]),
            TerminationContractV1::never(),
        )),
        Err(ResolveExecutionError::ArtifactResolve {
            error: ArtifactResolveError::NotFound,
            ..
        })
    ));
}

#[test]
fn unsupported_engine_protocol_fails_before_execution_activation() {
    let distribution = distribution();
    let supported = distribution.engine_protocol();
    let required = EngineProtocolVersion::new(2);
    assert_ne!(required, supported);
    let compilation = compile_standard(&distribution, required);
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state(actor, item, source, source, destination, alternate, 2, true);

    assert!(matches!(
        engine.resolve_execution(ExecutionSpecInput::origin(
            compilation.definitions().lock().clone(),
            ExecutionOrigin::new(initial, Vec::new(), SimMoment::ORIGIN, SimMoment::ORIGIN),
            baseline_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            RootSeed::from_bytes([0x61; 32]),
            TerminationContractV1::never(),
        )),
        Err(ResolveExecutionError::Activation(error))
            if error
                == ExecutionActivationError::UnsupportedEngineProtocol {
                    required,
                    supported,
                }
    ));
}

#[test]
fn execution_cannot_cross_engine_composition_boundaries() {
    let first_distribution = distribution();
    let second_distribution = distribution();
    let compilation = compile_standard(&first_distribution, first_distribution.engine_protocol());
    let first_engine = engine_with(first_distribution, compilation.envelopes().to_vec());
    let second_engine = engine_with(second_distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let execution = resolve_origin(
        &first_engine,
        &compilation,
        accepted_state(actor, item, source, source, destination, alternate, 2, true),
        TerminationContractV1::never(),
    );

    assert!(matches!(
        second_engine.start_attempt(&execution, AttemptKey::from_bytes([0x75; 32])),
        Err(StartAttemptError::EngineMismatch)
    ));
}

#[test]
fn terminal_clock_exhaustion_publishes_failed_safety_without_consuming_due_work() {
    let distribution = distribution();
    let compilation = compile_standard(&distribution, distribution.engine_protocol());
    let engine = engine_with(distribution, compilation.envelopes().to_vec());
    let actor = ActorId::from_bytes([0x11; 32]);
    let item = EntityId::from_bytes([0x21; 32]);
    let source = EntityId::from_bytes([0x31; 32]);
    let destination = EntityId::from_bytes([0x41; 32]);
    let alternate = EntityId::from_bytes([0x42; 32]);
    let initial = accepted_state(actor, item, source, source, destination, alternate, 2, true);
    let execution = resolve_origin(
        &engine,
        &compilation,
        initial,
        TerminationContractV1::never(),
    );
    let mut attempt = valid(engine.start_attempt(&execution, AttemptKey::from_bytes([0x76; 32])));
    let terminal_moment = SimMoment::new(SimTime::from_ticks(u64::MAX), Microstep::new(u64::MAX));
    valid(attempt.submit_system_command(transfer_request(
        1,
        terminal_moment,
        CommandSource::from_bytes([0x86; 32]),
        1,
        actor,
        actor,
        item,
        source,
        destination,
    )));
    let session = attempt.session();
    let terminal_snapshot = valid(session.snapshot());

    let outcome = valid(attempt.advance(AdvanceRequest::through(terminal_moment)));
    let AdvanceOutcome::KernelSafety {
        cursor,
        cause,
        disposition,
        ..
    } = outcome
    else {
        panic!("terminal clock exhaustion must publish a safety transition");
    };
    assert!(matches!(
        cause,
        KernelSafetyCause::TerminalClockExhausted { evidence }
            if evidence.due() == terminal_moment
    ));
    assert_eq!(disposition, KernelSafetyDisposition::Failed);
    assert!(matches!(valid(attempt.status()), RunAttemptStatus::Active));
    let read = valid(session.read());
    assert_eq!(read.cursor(), cursor);
    assert_eq!(read.mode(), SessionMode::Failed);
    assert_eq!(
        read.safety_blocker().map(|blocker| blocker.cause()),
        Some(cause)
    );
    assert_eq!(read.snapshot().accepted(), terminal_snapshot.accepted());
    assert_eq!(read.snapshot().revision(), cursor.revision());
    assert!(matches!(
        attempt.advance(AdvanceRequest::through(terminal_moment)),
        Err(AttemptError::SessionNotRunning {
            current: SessionMode::Failed
        })
    ));
}
