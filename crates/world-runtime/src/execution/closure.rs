use world_core::{CanonicalBytes, CanonicalDomain, CanonicalError, CanonicalWriter};
use world_defs::{
    ArtifactDigest, PackCoordinate, PackExportDigest, RuntimeDefinitionSet,
    RuntimeSemanticFingerprint, VerifiedPackArtifact,
};

use crate::authority::AuthorityCursor;

use super::binding::VerifiedInitialExecutionBinding;
use super::{
    CanonicalExecutionSpecV1, ExecutionSemanticsManifestV1, InitialExecutionBindingError,
    InitialStateRootV1, ResolvedExecutionClosureManifestDigest,
};

/// Canonical schema of a resolved immutable execution closure.
pub const RESOLVED_EXECUTION_CLOSURE_SCHEMA_VERSION: u16 = 1;

const RESOLVED_EXECUTION_CLOSURE_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("resolved-execution-closure-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("resolved execution closure domain must be valid"),
    };

/// Exact pack artifact retained by a resolved execution closure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifactReference {
    coordinate: PackCoordinate,
    format_version: u16,
    byte_length: u64,
    artifact: ArtifactDigest,
    export: PackExportDigest,
    semantics: RuntimeSemanticFingerprint,
}

impl RuntimeArtifactReference {
    fn from_artifact(artifact: &VerifiedPackArtifact) -> Self {
        Self {
            coordinate: artifact.coordinate().clone(),
            format_version: artifact.envelope().descriptor().format_version(),
            byte_length: artifact.envelope().descriptor().blob_length(),
            artifact: artifact.artifact_digest(),
            export: artifact.export_digest(),
            semantics: artifact.semantic_fingerprint(),
        }
    }

    /// Returns the exact pack coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &PackCoordinate {
        &self.coordinate
    }

    /// Returns the artifact storage-format version.
    #[must_use]
    pub const fn format_version(&self) -> u16 {
        self.format_version
    }

    /// Returns the exact encoded artifact length.
    #[must_use]
    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    /// Returns the exact stored-byte artifact identity.
    #[must_use]
    pub const fn artifact_digest(&self) -> ArtifactDigest {
        self.artifact
    }

    /// Returns the public-definition signature identity.
    #[must_use]
    pub const fn export_digest(&self) -> PackExportDigest {
        self.export
    }

    /// Returns the normalized pack behavior identity.
    #[must_use]
    pub const fn semantic_fingerprint(&self) -> RuntimeSemanticFingerprint {
        self.semantics
    }
}

/// Complete immutable material required to construct or reopen an execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedExecutionClosureManifestV1 {
    initial_root: InitialStateRootV1,
    specification: CanonicalExecutionSpecV1,
    semantics: ExecutionSemanticsManifestV1,
    artifacts: Vec<RuntimeArtifactReference>,
    root_cursor: AuthorityCursor,
    digest: ResolvedExecutionClosureManifestDigest,
}

impl ResolvedExecutionClosureManifestV1 {
    pub(crate) fn bind(
        initial_root: InitialStateRootV1,
        specification: CanonicalExecutionSpecV1,
        semantics: ExecutionSemanticsManifestV1,
    ) -> Result<Self, InitialExecutionBindingError> {
        let binding = VerifiedInitialExecutionBinding::new(initial_root, specification, semantics)?;
        Ok(Self::from_verified(binding))
    }

    fn from_verified(binding: VerifiedInitialExecutionBinding) -> Self {
        let artifacts = artifact_references(binding.semantics().definitions());
        let bytes = resolved_execution_closure_bytes(
            binding.root(),
            binding.specification(),
            binding.semantics(),
            &artifacts,
        );
        let digest = ResolvedExecutionClosureManifestDigest::of_canonical(&bytes);
        let (initial_root, specification, semantics, root_cursor) = binding.into_parts();
        Self {
            initial_root,
            specification,
            semantics,
            artifacts,
            root_cursor,
            digest,
        }
    }

    /// Returns the exact initial state root.
    #[must_use]
    pub const fn initial_root(&self) -> &InitialStateRootV1 {
        &self.initial_root
    }

    /// Returns the canonical execution specification.
    #[must_use]
    pub const fn specification(&self) -> &CanonicalExecutionSpecV1 {
        &self.specification
    }

    /// Returns the normalized execution semantics and retained definitions.
    #[must_use]
    pub const fn semantics(&self) -> &ExecutionSemanticsManifestV1 {
        &self.semantics
    }

    /// Returns exact pack artifact references in canonical pack-key order.
    #[must_use]
    pub fn artifacts(&self) -> &[RuntimeArtifactReference] {
        &self.artifacts
    }

    /// Returns the distinguished cursor before the first authority record.
    #[must_use]
    pub const fn root_cursor(&self) -> AuthorityCursor {
        self.root_cursor
    }

    /// Returns the immutable execution-closure identity.
    #[must_use]
    pub const fn digest(&self) -> ResolvedExecutionClosureManifestDigest {
        self.digest
    }

    #[cfg(test)]
    pub(crate) fn canonical_bytes(&self) -> CanonicalBytes {
        resolved_execution_closure_bytes(
            &self.initial_root,
            &self.specification,
            &self.semantics,
            &self.artifacts,
        )
    }
}

fn artifact_references(definitions: &RuntimeDefinitionSet) -> Vec<RuntimeArtifactReference> {
    definitions
        .artifacts()
        .iter()
        .map(RuntimeArtifactReference::from_artifact)
        .collect()
}

fn resolved_execution_closure_bytes(
    initial_root: &InitialStateRootV1,
    specification: &CanonicalExecutionSpecV1,
    semantics: &ExecutionSemanticsManifestV1,
    artifacts: &[RuntimeArtifactReference],
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(RESOLVED_EXECUTION_CLOSURE_DOMAIN);
    writer.write_u16(RESOLVED_EXECUTION_CLOSURE_SCHEMA_VERSION);
    write_fixed_bytes(&mut writer, initial_root.id().as_bytes());
    write_fixed_bytes(&mut writer, specification.id().as_bytes());
    write_fixed_bytes(&mut writer, semantics.digest().as_bytes());
    write_fixed_bytes(&mut writer, semantics.definition_set_digest().as_bytes());
    write_fixed_bytes(
        &mut writer,
        semantics.lifecycle_profiles().digest().as_bytes(),
    );
    write_fixed_bytes(&mut writer, semantics.config().digest().as_bytes());
    if writer
        .write_sequence(artifacts, write_artifact_reference)
        .is_err()
    {
        unreachable!("validated artifact closure must fit the canonical protocol");
    }
    writer.finish()
}

fn write_artifact_reference(
    writer: &mut CanonicalWriter,
    artifact: &RuntimeArtifactReference,
) -> Result<(), CanonicalError> {
    writer.write_str(artifact.coordinate.pack_key().as_str())?;
    writer.write_u32(artifact.coordinate.version().major());
    writer.write_u32(artifact.coordinate.version().minor());
    writer.write_u32(artifact.coordinate.version().patch());
    writer.write_u16(artifact.format_version);
    writer.write_u64(artifact.byte_length);
    writer.write_bytes(artifact.artifact.as_bytes())?;
    writer.write_bytes(artifact.export.as_bytes())?;
    writer.write_bytes(artifact.semantics.as_bytes())
}

fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width identity length must fit the canonical protocol");
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use world_core::{ActorId, EntityId, SimMoment};
    use world_defs::{
        ActionBindingData, ActionData, ArtifactData, ArtifactValidator, BindingName, DefinitionKey,
        DefinitionLinker, EffectCallData, EngineProtocolVersion, EventData, EventEmissionData,
        EventFieldBindingData, EventFieldData, EventFieldName, ExactPackSet, ExactPackageSelection,
        InterfaceVersion, LocalDefinitionName, OperationCallData, OperationKind, OperationName,
        OperationParameter, PackCoordinate, PackKey, PackManifestData, PackVersion, ParameterName,
        RuntimeDefinitionSet, RuntimeRequirementData, SelectedPackage, SemanticInterfaceCatalog,
        SemanticInterfaceDescriptor, SemanticInterfaceKey, SemanticOperationDescriptor,
        SourceSnapshotId, ValueKind,
    };
    use world_model::{
        AcceptedState, AgencyState, ContainerAuthorityRecord, ContainerRecord, ContainmentRecord,
        DomainState, EpistemicState, SocialState,
    };

    use crate::authority::{AuthorityCursor, AuthorityPosition, EpochIdentity};
    use crate::execution::{
        ExecutionConfigArtifactV3, ExternalInputBindingV1, RootSeed, SemanticImplementationBinding,
        SemanticImplementationId, TerminationContractV1,
    };
    use crate::session::SessionMode;

    use super::*;

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("execution fixture must be valid: {error}"),
        }
    }

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
            encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        encoded
    }

    fn definitions(version: PackVersion) -> RuntimeDefinitionSet {
        let pack = valid(PackKey::parse("test.execution"));
        let coordinate = PackCoordinate::new(pack.clone(), version);
        let interface_key = valid(SemanticInterfaceKey::parse("test.containment"));
        let actor_parameter =
            OperationParameter::new(valid(ParameterName::parse("actor")), ValueKind::Actor);
        let item_parameter =
            OperationParameter::new(valid(ParameterName::parse("item")), ValueKind::Entity);
        let requirement_name = valid(OperationName::parse("can-move"));
        let effect_name = valid(OperationName::parse("move"));
        let descriptor = valid(SemanticInterfaceDescriptor::new(
            interface_key.clone(),
            valid(InterfaceVersion::new(1)),
            vec![
                valid(SemanticOperationDescriptor::new(
                    requirement_name.clone(),
                    OperationKind::Predicate,
                    vec![actor_parameter.clone(), item_parameter.clone()],
                )),
                valid(SemanticOperationDescriptor::new(
                    effect_name.clone(),
                    OperationKind::Effect,
                    vec![actor_parameter, item_parameter],
                )),
            ],
        ));
        let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor.clone()]));
        let actor_binding = valid(BindingName::parse("actor"));
        let item_binding = valid(BindingName::parse("item"));
        let arguments = vec![actor_binding.clone(), item_binding.clone()];
        let event_name = valid(LocalDefinitionName::parse("item-moved"));
        let event = EventData::new(
            event_name.clone(),
            vec![
                EventFieldData::new(valid(EventFieldName::parse("actor")), ValueKind::Actor),
                EventFieldData::new(valid(EventFieldName::parse("item")), ValueKind::Entity),
            ],
        );
        let action_name = valid(LocalDefinitionName::parse("move-item"));
        let action = ActionData::new(
            action_name,
            vec![
                ActionBindingData::new(actor_binding.clone(), ValueKind::Actor),
                ActionBindingData::new(item_binding.clone(), ValueKind::Entity),
            ],
            vec![RuntimeRequirementData::new(OperationCallData::new(
                interface_key.clone(),
                requirement_name,
                arguments.clone(),
            ))],
            vec![EffectCallData::new(OperationCallData::new(
                interface_key,
                effect_name,
                arguments,
            ))],
            vec![EventEmissionData::new(
                DefinitionKey::new(pack.clone(), event_name.clone()),
                vec![
                    EventFieldBindingData::new(
                        valid(EventFieldName::parse("actor")),
                        actor_binding,
                    ),
                    EventFieldBindingData::new(valid(EventFieldName::parse("item")), item_binding),
                ],
            )],
        );
        let artifact = valid(ArtifactValidator::new(&catalog).validate(ArtifactData::new(
            PackManifestData::new(
                EngineProtocolVersion::new(1),
                coordinate.clone(),
                Vec::new(),
            ),
            vec![descriptor.reference()],
            vec![action],
            vec![event],
        )));
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

    fn semantics(implementation_byte: u8) -> ExecutionSemanticsManifestV1 {
        let definitions = definitions(PackVersion::new(1, 0, 0));
        let interface = definitions.required_interfaces()[0].clone();
        valid(ExecutionSemanticsManifestV1::new(
            definitions,
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            vec![SemanticImplementationBinding::new(
                interface,
                SemanticImplementationId::from_bytes([implementation_byte; 32]),
            )],
        ))
    }

    fn accepted_state(item_byte: u8) -> AcceptedState {
        let source = EntityId::from_bytes([0x21; 32]);
        let destination = EntityId::from_bytes([0x22; 32]);
        AcceptedState::new(
            valid(DomainState::new(
                vec![
                    ContainerRecord::new(source, 1),
                    ContainerRecord::new(destination, 1),
                ],
                vec![ContainmentRecord::new(
                    EntityId::from_bytes([item_byte; 32]),
                    source,
                )],
                vec![ContainerAuthorityRecord::new(
                    ActorId::from_bytes([0x41; 32]),
                    source,
                )],
            )),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        )
    }

    fn root(item_byte: u8) -> InitialStateRootV1 {
        valid(InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            accepted_state(item_byte),
            Vec::new(),
        ))
    }

    fn specification(
        root: &InitialStateRootV1,
        semantics: &ExecutionSemanticsManifestV1,
    ) -> CanonicalExecutionSpecV1 {
        CanonicalExecutionSpecV1::new(
            root,
            semantics,
            RootSeed::from_bytes([0x61; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        )
    }

    #[test]
    fn resolved_closure_retains_and_binds_every_immutable_input() {
        let root = root(0x31);
        let semantics = semantics(0x51);
        let specification = specification(&root, &semantics);
        let expected = (
            semantics.digest().to_string(),
            root.id().to_string(),
            specification.id().to_string(),
        );
        let closure = valid(ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ));

        assert_eq!(closure.artifacts().len(), 1);
        assert_eq!(
            closure.artifacts()[0].coordinate().pack_key().as_str(),
            "test.execution"
        );
        let AuthorityPosition::Root {
            record_anchor,
            cumulative_anchor,
        } = closure.root_cursor().position()
        else {
            panic!("verified initial binding must produce a root cursor");
        };
        assert_eq!(
            closure.root_cursor().epoch().lineage(),
            closure.initial_root().lineage_id()
        );
        assert_eq!(
            closure.root_cursor().epoch().execution(),
            closure.specification().id()
        );
        assert_eq!(
            closure.root_cursor(),
            AuthorityCursor::root(
                EpochIdentity::new(
                    closure.initial_root().lineage_id(),
                    closure.specification().id(),
                ),
                closure.initial_root().id(),
            )
        );
        assert_eq!(
            closure
                .initial_root()
                .accepted_state()
                .domain()
                .containers()
                .len(),
            2
        );
        assert_eq!(
            closure
                .initial_root()
                .accepted_state()
                .domain()
                .containment()
                .len(),
            1
        );
        assert_eq!(
            closure
                .initial_root()
                .accepted_state()
                .domain()
                .container_authority()
                .len(),
            1
        );
        assert_eq!(
            closure.digest(),
            ResolvedExecutionClosureManifestDigest::of_canonical(&closure.canonical_bytes())
        );
        assert_eq!(
            hex(closure.canonical_bytes().as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d7631000000000000001d7265736f6c7665642d65",
                "7865637574696f6e2d636c6f737572652d763100010000000000000020faeed241fe74949c",
                "4f58ab915610c51767662ee7a5aea483f58463516f0114ff0000000000000020dce4582fda",
                "b1a346b3e32f0b752db11cee490c32171953b972e5ba05d09323ad00000000000000208521",
                "f216c8d5068b2ff55849e77c67b2eed4b481590d6c21e1a1acbf67c97f2f000000000000",
                "0020e15b82695294defcdd82db0350f090e9d0cedffd879dc4d1998549a498bf86fd000000",
                "00000000203aaea302d4a6ff6b178dee51269baf56134f09612c178498abce3dc13005e6ee",
                "000000000000002049937ac98d56333b9b4341f2686d96324e6a45767682d91c3f03cd620",
                "40c2aaa0000000000000001000000000000000e746573742e657865637574696f6e000000",
                "010000000000000000000100000000000000ee00000000000000201cb3787f0655972bd460",
                "cca50a10b0778c0a6a0d70a9feba8b0c35877fb0839c0000000000000020c6bfe1760753",
                "de1be559a778294538c27b3b517eb557b3666d292bf6fdea3beb0000000000000020df7450",
                "b6aca3fcdc1bb9b41e1baf5a96f9df3956bd1227b5c9066c3e5f38f18c"
            )
        );
        assert_eq!(
            (record_anchor.to_string(), cumulative_anchor.to_string()),
            (
                "0792921a4316152540137e47fd2b4acf4acbe67d6fa0f0a398112338c27b64f7".to_owned(),
                "ae76359db15817f27367749bdc96383a864d266c6dbe21fdebae9ec90f9a5d6d".to_owned(),
            )
        );
        assert_eq!(
            (
                expected.0,
                expected.1,
                expected.2,
                closure.digest().to_string(),
            ),
            (
                "8521f216c8d5068b2ff55849e77c67b2eed4b481590d6c21e1a1acbf67c97f2f".to_owned(),
                "faeed241fe74949c4f58ab915610c51767662ee7a5aea483f58463516f0114ff".to_owned(),
                "dce4582fdab1a346b3e32f0b752db11cee490c32171953b972e5ba05d09323ad".to_owned(),
                "4875048619093a30c968ad8c6917208f56be37255ced10282432b9d47d77a146".to_owned(),
            )
        );
    }

    #[test]
    fn closure_identity_tracks_nested_content_through_child_identities() {
        let base_root = root(0x31);
        let base_semantics = semantics(0x51);
        let base_specification = specification(&base_root, &base_semantics);
        let base = valid(ResolvedExecutionClosureManifestV1::bind(
            base_root,
            base_specification,
            base_semantics,
        ));

        let changed_root = root(0x32);
        let unchanged_semantics = semantics(0x51);
        let changed_root_specification = specification(&changed_root, &unchanged_semantics);
        let changed_root_closure = valid(ResolvedExecutionClosureManifestV1::bind(
            changed_root,
            changed_root_specification,
            unchanged_semantics,
        ));

        assert_ne!(
            base.initial_root().id(),
            changed_root_closure.initial_root().id()
        );
        assert_eq!(
            base.semantics().digest(),
            changed_root_closure.semantics().digest()
        );
        assert_ne!(
            base.specification().id(),
            changed_root_closure.specification().id()
        );
        assert_ne!(base.digest(), changed_root_closure.digest());

        let unchanged_root = root(0x31);
        let changed_semantics = semantics(0x52);
        let changed_semantics_specification = specification(&unchanged_root, &changed_semantics);
        let changed_semantics_closure = valid(ResolvedExecutionClosureManifestV1::bind(
            unchanged_root,
            changed_semantics_specification,
            changed_semantics,
        ));

        assert_eq!(
            base.initial_root().id(),
            changed_semantics_closure.initial_root().id()
        );
        assert_ne!(
            base.semantics().digest(),
            changed_semantics_closure.semantics().digest()
        );
        assert_ne!(
            base.specification().id(),
            changed_semantics_closure.specification().id()
        );
        assert_ne!(base.digest(), changed_semantics_closure.digest());
    }

    #[test]
    fn binding_rejects_a_different_root_or_semantics_manifest() {
        let first_root = root(0x31);
        let other_root = root(0x32);
        let first_semantics = semantics(0x51);
        let other_semantics = semantics(0x52);
        let specification = specification(&first_root, &first_semantics);

        assert_eq!(
            ResolvedExecutionClosureManifestV1::bind(
                other_root.clone(),
                specification.clone(),
                first_semantics.clone(),
            ),
            Err(InitialExecutionBindingError::InitialRootMismatch {
                expected: other_root.id(),
                actual: specification.initial_root(),
            })
        );
        assert_eq!(
            ResolvedExecutionClosureManifestV1::bind(
                first_root,
                specification.clone(),
                other_semantics.clone(),
            ),
            Err(InitialExecutionBindingError::ExecutionSemanticsMismatch {
                expected: other_semantics.digest(),
                actual: specification.semantics(),
            })
        );
    }
}
