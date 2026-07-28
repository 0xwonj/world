mod admit;
mod fire;
mod manage;
mod resolution;
mod safety;

pub use admit::{
    AdmitOutcome, AdmitRequest, INPUT_REQUEST_SCHEMA_VERSION, InputId, InputRequestFingerprint,
};
pub use fire::{
    ActionEvaluationDecision, ActionEvaluationResultFailure, ActivityAdvanceResult,
    ActivityInitializationResult, AppraisalResult, CommandFireClassification,
    CommandFireResolution, DeferredActionArtifactInput, DeferredActionInvocationInput,
    EvaluatedAction, FireOutcome, FireRequest, IntentReviewResult, MomentWorkDecision,
    MomentWorkInput, MomentWorkProposals, PostCommitRoutingDecision, PreparedFire,
    PreparedFireFailure, PreparedFireFailureOutcome, ProposalBuildError, WorkId,
};
pub use manage::{
    ActionEvaluationManagementDisposition, LedgerRetirement, MANAGEMENT_REQUEST_SCHEMA_VERSION,
    ManageOutcome, ManageRequest, ManagementRequestFingerprint, ManagementRequestId,
    SessionManagement,
};
pub use safety::{
    FirePreparation, KERNEL_SAFETY_CAUSE_SCHEMA_VERSION, KernelSafetyBlocker, KernelSafetyCause,
    KernelSafetyDisposition, KernelSafetyDueSetEvidence, KernelSafetyOutcome,
    KernelSafetyTriggerCoordinate, KernelSafetyTriggerLane, KernelSafetyTriggerSample,
    PreparedKernelSafety,
};

pub(crate) use admit::derive_input_request_namespace;
pub(crate) use fire::{
    ActionProposal, CommandProposal, PreparedCommandResolution, PreparedDelivery, WorkProposal,
};
pub(crate) use resolution::{
    ContainmentCandidate, ContainmentCandidateOutcome, ContainmentCandidateProposal,
    ContainmentCandidateSet, ContainmentCommandIdentity, ContainmentMomentResolution,
    ContainmentResolutionEvidence, ContainmentResolutionFallback, resolve_containment_candidates,
};
pub(crate) use safety::select_kernel_safety_cause;

#[cfg(test)]
pub(crate) mod fixtures {
    use core::fmt;

    use world_core::ActorId;
    use world_defs::{
        ActionBindingData, ActionData, ArtifactData, ArtifactValidator, BindingName, DefinitionKey,
        DefinitionLinker, EffectCallData, EngineProtocolVersion, EventData, EventEmissionData,
        EventFieldBindingData, EventFieldData, EventFieldName, ExactPackSet, ExactPackageSelection,
        InterfaceVersion, LocalDefinitionName, OperationCallData, OperationKind, OperationName,
        OperationParameter, PackCoordinate, PackKey, PackManifestData, PackVersion, ParameterName,
        RuntimeDefinitionSet, SelectedPackage, SemanticInterfaceCatalog,
        SemanticInterfaceDescriptor, SemanticInterfaceKey, SemanticOperationDescriptor,
        SourceSnapshotId, ValueKind,
    };
    use world_model::{CommandBinding, CommandEnvelope, CommandId, CommandSource, CommandValue};

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("runtime command fixture must be valid: {error}"),
        }
    }

    fn link(
        coordinate: PackCoordinate,
        artifact: world_defs::VerifiedPackArtifact,
    ) -> RuntimeDefinitionSet {
        let selection = ExactPackageSelection::new(
            coordinate.clone(),
            vec![SelectedPackage::new(
                coordinate,
                SourceSnapshotId::from_bytes([0x71; 32]),
                Vec::new(),
            )],
        );
        valid(DefinitionLinker::link(valid(ExactPackSet::finalize(
            selection,
            vec![artifact],
        ))))
    }

    fn definition_fixture() -> (RuntimeDefinitionSet, DefinitionKey, BindingName) {
        let pack = valid(PackKey::parse("test.runtime"));
        let coordinate = PackCoordinate::new(pack.clone(), PackVersion::new(1, 0, 0));
        let interface_key = valid(SemanticInterfaceKey::parse("test.delivery"));
        let operation_name = valid(OperationName::parse("deliver"));
        let actor_parameter =
            OperationParameter::new(valid(ParameterName::parse("actor")), ValueKind::Actor);
        let descriptor = valid(SemanticInterfaceDescriptor::new(
            interface_key.clone(),
            valid(InterfaceVersion::new(1)),
            vec![valid(SemanticOperationDescriptor::new(
                operation_name.clone(),
                OperationKind::Effect,
                vec![actor_parameter],
            ))],
        ));
        let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor.clone()]));
        let actor_binding = valid(BindingName::parse("actor"));
        let event_name = valid(LocalDefinitionName::parse("delivered"));
        let event_field = valid(EventFieldName::parse("actor"));
        let action_name = valid(LocalDefinitionName::parse("deliver"));
        let action = DefinitionKey::new(pack.clone(), action_name.clone());
        let artifact = valid(ArtifactValidator::new(&catalog).validate(ArtifactData::new(
            PackManifestData::new(
                EngineProtocolVersion::new(1),
                coordinate.clone(),
                Vec::new(),
            ),
            vec![descriptor.reference()],
            vec![ActionData::new(
                action_name,
                vec![ActionBindingData::new(
                    actor_binding.clone(),
                    ValueKind::Actor,
                )],
                Vec::new(),
                vec![EffectCallData::new(OperationCallData::new(
                    interface_key,
                    operation_name,
                    vec![actor_binding.clone()],
                ))],
                vec![EventEmissionData::new(
                    DefinitionKey::new(pack, event_name.clone()),
                    vec![EventFieldBindingData::new(
                        event_field.clone(),
                        actor_binding.clone(),
                    )],
                )],
            )],
            vec![EventData::new(
                event_name,
                vec![EventFieldData::new(event_field, ValueKind::Actor)],
            )],
        )));
        (link(coordinate, artifact), action, actor_binding)
    }

    pub(crate) fn definitions() -> RuntimeDefinitionSet {
        let pack = valid(PackKey::parse("test.root"));
        let coordinate = PackCoordinate::new(pack, PackVersion::new(1, 0, 0));
        let event_name = valid(LocalDefinitionName::parse("marker"));
        let event_field = valid(EventFieldName::parse("entity"));
        let catalog = valid(SemanticInterfaceCatalog::new(Vec::new()));
        let artifact = valid(ArtifactValidator::new(&catalog).validate(ArtifactData::new(
            PackManifestData::new(
                EngineProtocolVersion::new(1),
                coordinate.clone(),
                Vec::new(),
            ),
            Vec::new(),
            Vec::new(),
            vec![EventData::new(
                event_name,
                vec![EventFieldData::new(event_field, ValueKind::Entity)],
            )],
        )));
        link(coordinate, artifact)
    }

    pub(crate) fn command_definitions() -> RuntimeDefinitionSet {
        definition_fixture().0
    }

    pub(crate) fn command(source_byte: u8, command_id: u64) -> CommandEnvelope {
        command_with_actor(source_byte, command_id, 0x41)
    }

    pub(crate) fn command_with_actor(
        source_byte: u8,
        command_id: u64,
        actor_byte: u8,
    ) -> CommandEnvelope {
        let (definitions, action, actor_binding) = definition_fixture();
        let actor = ActorId::from_bytes([actor_byte; 32]);

        valid(CommandEnvelope::new(
            &definitions,
            CommandSource::from_bytes([source_byte; 32]),
            CommandId::new(command_id),
            actor,
            action,
            vec![CommandBinding::new(
                actor_binding,
                CommandValue::Actor(actor),
            )],
        ))
    }
}
