use core::cmp::Ordering;
use core::fmt;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalError, CanonicalWriter};
use world_defs::{
    EngineProtocolVersion, RuntimeDefinitionSet, RuntimeDefinitionSetDigest,
    SemanticInterfaceReference,
};

use super::{
    ExecutionConfigArtifactV3, ExecutionSemanticsManifestDigest, LifecycleProfilesV2,
    SemanticImplementationId,
};

/// Canonical schema of the normalized execution-semantics manifest.
pub const EXECUTION_SEMANTICS_MANIFEST_SCHEMA_VERSION: u16 = 1;

const EXECUTION_SEMANTICS_MANIFEST_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("execution-semantics-manifest-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("execution semantics manifest domain must be valid"),
    };

/// Exact implementation selected for one required semantic interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticImplementationBinding {
    interface: SemanticInterfaceReference,
    implementation: SemanticImplementationId,
}

impl SemanticImplementationBinding {
    /// Associates one exact interface descriptor with its implementation.
    #[must_use]
    pub const fn new(
        interface: SemanticInterfaceReference,
        implementation: SemanticImplementationId,
    ) -> Self {
        Self {
            interface,
            implementation,
        }
    }

    /// Returns the exact interface descriptor reference.
    #[must_use]
    pub const fn interface(&self) -> &SemanticInterfaceReference {
        &self.interface
    }

    /// Returns the behavior-affecting implementation identity.
    #[must_use]
    pub const fn implementation(&self) -> SemanticImplementationId {
        self.implementation
    }
}

/// Why implementation bindings could not close the definition requirements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticBindingError {
    /// More than one implementation was supplied for one interface key.
    DuplicateBinding {
        /// Repeated exact interface reference.
        interface: SemanticInterfaceReference,
    },
    /// A required interface had no implementation binding.
    MissingBinding {
        /// Unbound required interface.
        interface: SemanticInterfaceReference,
    },
    /// An implementation was supplied for an interface outside the closure.
    UnexpectedBinding {
        /// Interface outside the required closure.
        interface: SemanticInterfaceReference,
    },
    /// A binding used the right interface key but a different exact descriptor.
    ReferenceMismatch {
        /// Descriptor required by the definition set.
        expected: Box<SemanticInterfaceReference>,
        /// Descriptor named by the implementation binding.
        actual: Box<SemanticInterfaceReference>,
    },
}

impl fmt::Display for SemanticBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateBinding { interface } => {
                write!(
                    formatter,
                    "semantic implementation binding repeats interface {}",
                    interface.key()
                )
            }
            Self::MissingBinding { interface } => {
                write!(
                    formatter,
                    "semantic implementation is missing for interface {}",
                    interface.key()
                )
            }
            Self::UnexpectedBinding { interface } => {
                write!(
                    formatter,
                    "semantic implementation names unexpected interface {}",
                    interface.key()
                )
            }
            Self::ReferenceMismatch { expected, actual } => write!(
                formatter,
                "semantic implementation reference for {} does not match version {} digest {}",
                actual.key(),
                expected.version(),
                expected.digest()
            ),
        }
    }
}

impl std::error::Error for SemanticBindingError {}

/// Normalized identity of every behavior-affecting execution dependency.
///
/// The retained definition set, configuration, lifecycle selection, and
/// implementation bindings are the values from which the manifest identity is
/// derived. Callers cannot assemble a manifest from detached digests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionSemanticsManifestV1 {
    definitions: RuntimeDefinitionSet,
    lifecycle_profiles: LifecycleProfilesV2,
    config: ExecutionConfigArtifactV3,
    required_interfaces: Vec<SemanticInterfaceReference>,
    implementation_bindings: Vec<SemanticImplementationBinding>,
    digest: ExecutionSemanticsManifestDigest,
}

impl ExecutionSemanticsManifestV1 {
    pub(crate) fn new(
        definitions: RuntimeDefinitionSet,
        lifecycle_profiles: LifecycleProfilesV2,
        config: ExecutionConfigArtifactV3,
        implementation_bindings: Vec<SemanticImplementationBinding>,
    ) -> Result<Self, SemanticBindingError> {
        let required_interfaces = definitions.required_interfaces().to_vec();
        let implementation_bindings =
            normalize_bindings(&required_interfaces, implementation_bindings)?;
        let bytes = execution_semantics_manifest_bytes(
            &definitions,
            lifecycle_profiles,
            config,
            &required_interfaces,
            &implementation_bindings,
        );

        Ok(Self {
            definitions,
            lifecycle_profiles,
            config,
            required_interfaces,
            implementation_bindings,
            digest: ExecutionSemanticsManifestDigest::of_canonical(&bytes),
        })
    }

    /// Returns the required engine protocol.
    #[must_use]
    pub const fn engine_protocol(&self) -> EngineProtocolVersion {
        self.definitions.engine_protocol()
    }

    /// Returns the exact linked runtime definition set.
    #[must_use]
    pub const fn definitions(&self) -> &RuntimeDefinitionSet {
        &self.definitions
    }

    /// Returns the definition-set identity committed by this manifest.
    #[must_use]
    pub const fn definition_set_digest(&self) -> RuntimeDefinitionSetDigest {
        self.definitions.digest()
    }

    /// Returns the selected lifecycle profiles.
    #[must_use]
    pub const fn lifecycle_profiles(&self) -> LifecycleProfilesV2 {
        self.lifecycle_profiles
    }

    /// Returns the selected execution configuration.
    #[must_use]
    pub const fn config(&self) -> ExecutionConfigArtifactV3 {
        self.config
    }

    /// Returns the exact required semantic-interface closure.
    #[must_use]
    pub fn required_interfaces(&self) -> &[SemanticInterfaceReference] {
        &self.required_interfaces
    }

    /// Returns implementation bindings in canonical interface-key order.
    #[must_use]
    pub fn implementation_bindings(&self) -> &[SemanticImplementationBinding] {
        &self.implementation_bindings
    }

    /// Returns the normalized execution-semantics identity.
    #[must_use]
    pub const fn digest(&self) -> ExecutionSemanticsManifestDigest {
        self.digest
    }

    pub(crate) fn canonical_bytes(&self) -> CanonicalBytes {
        execution_semantics_manifest_bytes(
            &self.definitions,
            self.lifecycle_profiles,
            self.config,
            &self.required_interfaces,
            &self.implementation_bindings,
        )
    }
}

fn normalize_bindings(
    required_interfaces: &[SemanticInterfaceReference],
    mut bindings: Vec<SemanticImplementationBinding>,
) -> Result<Vec<SemanticImplementationBinding>, SemanticBindingError> {
    bindings.sort_by(|left, right| {
        left.interface
            .key()
            .cmp(right.interface.key())
            .then_with(|| left.interface.cmp(&right.interface))
    });

    if let Some(duplicate) = bindings
        .windows(2)
        .find(|pair| pair[0].interface.key() == pair[1].interface.key())
    {
        return Err(SemanticBindingError::DuplicateBinding {
            interface: duplicate[1].interface.clone(),
        });
    }

    let mut required = required_interfaces.to_vec();
    required.sort_by(|left, right| left.key().cmp(right.key()).then_with(|| left.cmp(right)));

    let mut required_index = 0;
    let mut binding_index = 0;
    while required_index < required.len() && binding_index < bindings.len() {
        let expected = &required[required_index];
        let actual = &bindings[binding_index].interface;
        match actual.key().cmp(expected.key()) {
            Ordering::Less => {
                return Err(SemanticBindingError::UnexpectedBinding {
                    interface: actual.clone(),
                });
            }
            Ordering::Greater => {
                return Err(SemanticBindingError::MissingBinding {
                    interface: expected.clone(),
                });
            }
            Ordering::Equal if actual != expected => {
                return Err(SemanticBindingError::ReferenceMismatch {
                    expected: Box::new(expected.clone()),
                    actual: Box::new(actual.clone()),
                });
            }
            Ordering::Equal => {
                required_index += 1;
                binding_index += 1;
            }
        }
    }

    if let Some(expected) = required.get(required_index) {
        return Err(SemanticBindingError::MissingBinding {
            interface: expected.clone(),
        });
    }
    if let Some(binding) = bindings.get(binding_index) {
        return Err(SemanticBindingError::UnexpectedBinding {
            interface: binding.interface.clone(),
        });
    }

    Ok(bindings)
}

fn execution_semantics_manifest_bytes(
    definitions: &RuntimeDefinitionSet,
    lifecycle_profiles: LifecycleProfilesV2,
    config: ExecutionConfigArtifactV3,
    required_interfaces: &[SemanticInterfaceReference],
    implementation_bindings: &[SemanticImplementationBinding],
) -> CanonicalBytes {
    let lifecycle_body = lifecycle_profiles.canonical_bytes();
    let config_body = config.canonical_bytes();
    execution_semantics_manifest_preimage(
        definitions.engine_protocol().get(),
        definitions.digest().as_bytes(),
        lifecycle_body.as_bytes(),
        lifecycle_profiles.digest().as_bytes(),
        config_body.as_bytes(),
        config.digest().as_bytes(),
        required_interfaces,
        implementation_bindings,
    )
}

#[allow(clippy::too_many_arguments)]
fn execution_semantics_manifest_preimage(
    engine_protocol: u16,
    definition_set_digest: &[u8; 32],
    lifecycle_body: &[u8],
    lifecycle_digest: &[u8; 32],
    config_body: &[u8],
    config_digest: &[u8; 32],
    required_interfaces: &[SemanticInterfaceReference],
    implementation_bindings: &[SemanticImplementationBinding],
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(EXECUTION_SEMANTICS_MANIFEST_DOMAIN);
    writer.write_u16(EXECUTION_SEMANTICS_MANIFEST_SCHEMA_VERSION);
    writer.write_u16(engine_protocol);
    write_fixed_bytes(&mut writer, definition_set_digest);
    write_owned_bytes(&mut writer, lifecycle_body);
    write_fixed_bytes(&mut writer, lifecycle_digest);
    write_owned_bytes(&mut writer, config_body);
    write_fixed_bytes(&mut writer, config_digest);
    if writer
        .write_sequence(required_interfaces, write_interface_reference)
        .is_err()
    {
        unreachable!("checked interface closure must fit the canonical protocol");
    }
    if writer
        .write_sequence(implementation_bindings, |writer, binding| {
            write_interface_reference(writer, &binding.interface)?;
            writer.write_bytes(binding.implementation.as_bytes())
        })
        .is_err()
    {
        unreachable!("checked implementation closure must fit the canonical protocol");
    }
    writer.finish()
}

fn write_interface_reference(
    writer: &mut CanonicalWriter,
    interface: &SemanticInterfaceReference,
) -> Result<(), CanonicalError> {
    writer.write_str(interface.key().as_str())?;
    writer.write_u16(interface.version().get());
    writer.write_bytes(interface.digest().as_bytes())
}

fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width identity length must fit the canonical protocol");
    }
}

fn write_owned_bytes(writer: &mut CanonicalWriter, bytes: &[u8]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("owned canonical bytes must fit the canonical protocol");
    }
}

#[cfg(test)]
mod tests {
    use world_defs::{
        ActionBindingData, ActionData, ArtifactData, ArtifactValidator, BindingName, DefinitionKey,
        DefinitionLinker, EffectCallData, EngineProtocolVersion, EventData, EventEmissionData,
        EventFieldBindingData, EventFieldData, EventFieldName, ExactPackSet, ExactPackageSelection,
        InterfaceVersion, LocalDefinitionName, OperationCallData, OperationKind, OperationName,
        OperationParameter, PackCoordinate, PackKey, PackManifestData, PackVersion, ParameterName,
        RuntimeDefinitionSet, RuntimeRequirementData, SelectedPackage, SemanticInterfaceCatalog,
        SemanticInterfaceDescriptor, SemanticInterfaceDigest, SemanticInterfaceKey,
        SemanticInterfaceReference, SemanticOperationDescriptor, SourceSnapshotId, ValueKind,
    };

    use super::*;

    fn reference(name: &str, version: u16, digest_byte: u8) -> SemanticInterfaceReference {
        let key = match SemanticInterfaceKey::parse(name) {
            Ok(key) => key,
            Err(error) => panic!("interface fixture key must be valid: {error}"),
        };
        let version = match InterfaceVersion::new(version) {
            Ok(version) => version,
            Err(error) => panic!("interface fixture version must be valid: {error}"),
        };
        SemanticInterfaceReference::new(
            key,
            version,
            SemanticInterfaceDigest::from_bytes([digest_byte; 32]),
        )
    }

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("execution-semantics fixture must be valid: {error}"),
        }
    }

    fn linked_definitions() -> RuntimeDefinitionSet {
        let pack = valid(PackKey::parse("test.runtime"));
        let coordinate = PackCoordinate::new(pack.clone(), PackVersion::new(1, 0, 0));
        let subject = valid(BindingName::parse("subject"));
        let alpha_key = valid(SemanticInterfaceKey::parse("test.alpha"));
        let beta_key = valid(SemanticInterfaceKey::parse("test.beta"));
        let allows = valid(OperationName::parse("allows"));
        let applies = valid(OperationName::parse("applies"));
        let parameter =
            OperationParameter::new(valid(ParameterName::parse("subject")), ValueKind::Entity);
        let alpha = valid(SemanticInterfaceDescriptor::new(
            alpha_key.clone(),
            valid(InterfaceVersion::new(1)),
            vec![valid(SemanticOperationDescriptor::new(
                allows.clone(),
                OperationKind::Predicate,
                vec![parameter.clone()],
            ))],
        ));
        let beta = valid(SemanticInterfaceDescriptor::new(
            beta_key.clone(),
            valid(InterfaceVersion::new(2)),
            vec![valid(SemanticOperationDescriptor::new(
                applies.clone(),
                OperationKind::Effect,
                vec![parameter],
            ))],
        ));
        let catalog = valid(SemanticInterfaceCatalog::new(vec![
            beta.clone(),
            alpha.clone(),
        ]));
        let event_name = valid(LocalDefinitionName::parse("subject-changed"));
        let event_field = valid(EventFieldName::parse("subject"));
        let event = EventData::new(
            event_name.clone(),
            vec![EventFieldData::new(event_field.clone(), ValueKind::Entity)],
        );
        let action = ActionData::new(
            valid(LocalDefinitionName::parse("change-subject")),
            vec![ActionBindingData::new(subject.clone(), ValueKind::Entity)],
            vec![RuntimeRequirementData::new(OperationCallData::new(
                alpha_key,
                allows,
                vec![subject.clone()],
            ))],
            vec![EffectCallData::new(OperationCallData::new(
                beta_key,
                applies,
                vec![subject.clone()],
            ))],
            vec![EventEmissionData::new(
                DefinitionKey::new(pack, event_name),
                vec![EventFieldBindingData::new(event_field, subject)],
            )],
        );
        let artifact = valid(ArtifactValidator::new(&catalog).validate(ArtifactData::new(
            PackManifestData::new(
                EngineProtocolVersion::new(7),
                coordinate.clone(),
                Vec::new(),
            ),
            vec![beta.reference(), alpha.reference()],
            vec![action],
            vec![event],
        )));
        let selection = ExactPackageSelection::new(
            coordinate.clone(),
            vec![SelectedPackage::new(
                coordinate,
                SourceSnapshotId::from_bytes([0x77; 32]),
                Vec::new(),
            )],
        );
        let exact = valid(ExactPackSet::finalize(selection, vec![artifact]));
        valid(DefinitionLinker::link(exact))
    }

    fn implementation_bindings(
        definitions: &RuntimeDefinitionSet,
    ) -> Vec<SemanticImplementationBinding> {
        definitions
            .required_interfaces()
            .iter()
            .map(|interface| {
                let byte = match interface.key().as_str() {
                    "test.alpha" => 0xa1,
                    "test.beta" => 0xb2,
                    other => panic!("unexpected fixture interface {other}"),
                };
                SemanticImplementationBinding::new(
                    interface.clone(),
                    SemanticImplementationId::from_bytes([byte; 32]),
                )
            })
            .collect()
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

    #[test]
    fn bindings_are_canonicalized_by_interface_key() {
        let first = reference("test.alpha", 1, 0x11);
        let second = reference("test.beta", 1, 0x22);
        let bindings = normalize_bindings(
            &[first.clone(), second.clone()],
            vec![
                SemanticImplementationBinding::new(
                    second,
                    SemanticImplementationId::from_bytes([0x42; 32]),
                ),
                SemanticImplementationBinding::new(
                    first,
                    SemanticImplementationId::from_bytes([0x41; 32]),
                ),
            ],
        );
        let bindings = match bindings {
            Ok(bindings) => bindings,
            Err(error) => panic!("complete fixture bindings must be valid: {error}"),
        };

        assert_eq!(bindings[0].interface().key().as_str(), "test.alpha");
        assert_eq!(bindings[1].interface().key().as_str(), "test.beta");
    }

    #[test]
    fn binding_closure_reports_each_structural_mismatch() {
        let required = reference("test.required", 1, 0x11);
        let unexpected = reference("test.unexpected", 1, 0x22);
        let mismatched = reference("test.required", 2, 0x33);
        let implementation = SemanticImplementationId::from_bytes([0x44; 32]);

        assert_eq!(
            normalize_bindings(core::slice::from_ref(&required), Vec::new()),
            Err(SemanticBindingError::MissingBinding {
                interface: required.clone(),
            })
        );
        assert_eq!(
            normalize_bindings(
                &[],
                vec![SemanticImplementationBinding::new(
                    unexpected.clone(),
                    implementation,
                )],
            ),
            Err(SemanticBindingError::UnexpectedBinding {
                interface: unexpected,
            })
        );
        assert_eq!(
            normalize_bindings(
                core::slice::from_ref(&required),
                vec![SemanticImplementationBinding::new(
                    mismatched.clone(),
                    implementation,
                )],
            ),
            Err(SemanticBindingError::ReferenceMismatch {
                expected: Box::new(required.clone()),
                actual: Box::new(mismatched),
            })
        );
        assert_eq!(
            normalize_bindings(
                core::slice::from_ref(&required),
                vec![
                    SemanticImplementationBinding::new(required.clone(), implementation),
                    SemanticImplementationBinding::new(required.clone(), implementation),
                ],
            ),
            Err(SemanticBindingError::DuplicateBinding {
                interface: required,
            })
        );
    }

    #[test]
    fn linked_manifest_matches_the_byte_complete_vector_and_normalizes_bindings() {
        let definitions = linked_definitions();
        let forward = implementation_bindings(&definitions);
        let mut reversed = forward.clone();
        reversed.reverse();
        let lifecycle = crate::execution::fixture_lifecycle_profiles();
        let config = valid(ExecutionConfigArtifactV3::inline(64, 32, 16));
        let normalized = valid(ExecutionSemanticsManifestV1::new(
            definitions.clone(),
            lifecycle,
            config,
            forward,
        ));
        let reversed = valid(ExecutionSemanticsManifestV1::new(
            definitions.clone(),
            lifecycle,
            config,
            reversed,
        ));

        assert_eq!(normalized, reversed);
        assert_eq!(normalized.engine_protocol(), EngineProtocolVersion::new(7));
        assert_eq!(normalized.definition_set_digest(), definitions.digest());
        assert_eq!(normalized.lifecycle_profiles(), lifecycle);
        assert_eq!(normalized.config(), config);
        assert_eq!(
            normalized
                .required_interfaces()
                .iter()
                .map(|interface| (
                    interface.key().as_str(),
                    interface.version().get(),
                    interface.digest().to_string(),
                ))
                .collect::<Vec<_>>(),
            [
                (
                    "test.alpha",
                    1,
                    "6700ca768ebda356148af66c97060027b324bed589085329a04aa17222891e49".to_owned(),
                ),
                (
                    "test.beta",
                    2,
                    "2992e29668c7fe61fe06056a17a87414181e841247f1269ed573a24200b45863".to_owned(),
                ),
            ]
        );
        assert_eq!(
            normalized
                .implementation_bindings()
                .iter()
                .map(|binding| (
                    binding.interface().key().as_str(),
                    binding.implementation().into_bytes()[0],
                ))
                .collect::<Vec<_>>(),
            [("test.alpha", 0xa1), ("test.beta", 0xb2)]
        );
        assert_eq!(
            hex(normalized.canonical_bytes().as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d7631000000000000001f657865637574696f6e2d73656d616e746963732d6d616e69666573742d76310001000700000000000000204239ddf27cec82fda9dd100c057e270eb9c4e069dcbabc1bdbd0d0491a193d67000000000000013d776f726c642d63616e6f6e6963616c2d763100000000000000156c6966656379636c652d70726f66696c65732d763200020000000000000020e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1000000000000000000000020e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e200000000000000000000000000000020e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3000000000000000000000020e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e400000001000000000000002054545454545454545454545454545454545454545454545454545454545454540000000000000020a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1000000000000000000000000000000203aaea302d4a6ff6b178dee51269baf56134f09612c178498abce3dc13005e6ee000000000000005f",
                "776f726c642d63616e6f6e6963616c2d76310000000000000013657865637574696f6e2d636f6e6669672d76330003000000400000002000000010000000010000000000000000000000000000000000000000000000000000000000000000",
                "000000000000002049937ac98d56333b9b4341f2686d96324e6a45767682d91c3f03cd62040c2aaa0000000000000002000000000000000a746573742e616c706861000100000000000000206700ca768ebda356148af66c97060027b324bed589085329a04aa17222891e490000000000000009746573742e62657461000200000000000000202992e29668c7fe61fe06056a17a87414181e841247f1269ed573a24200b458630000000000000002000000000000000a746573742e616c706861000100000000000000206700ca768ebda356148af66c97060027b324bed589085329a04aa17222891e490000000000000020a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a10000000000000009746573742e62657461000200000000000000202992e29668c7fe61fe06056a17a87414181e841247f1269ed573a24200b458630000000000000020b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2",
            )
        );
        assert_eq!(
            normalized.digest().to_string(),
            "da0cf796720a22163e08ddd194778c88910b1472fc26c4090a50d64155e22026"
        );
    }

    #[test]
    fn every_manifest_field_changes_the_manifest_identity() {
        let definitions = linked_definitions();
        let required = definitions.required_interfaces().to_vec();
        let bindings = implementation_bindings(&definitions);
        let lifecycle = crate::execution::fixture_lifecycle_profiles();
        let config = valid(ExecutionConfigArtifactV3::inline(64, 32, 16));
        let lifecycle_body = lifecycle.canonical_bytes();
        let config_body = config.canonical_bytes();
        let definition_digest = definitions.digest().into_bytes();
        let lifecycle_digest = lifecycle.digest().into_bytes();
        let config_digest = config.digest().into_bytes();
        let baseline =
            ExecutionSemanticsManifestDigest::of_canonical(&execution_semantics_manifest_preimage(
                definitions.engine_protocol().get(),
                &definition_digest,
                lifecycle_body.as_bytes(),
                &lifecycle_digest,
                config_body.as_bytes(),
                &config_digest,
                &required,
                &bindings,
            ));
        macro_rules! assert_changes {
            ($bytes:expr) => {
                assert_ne!(
                    baseline,
                    ExecutionSemanticsManifestDigest::of_canonical(&$bytes)
                )
            };
        }

        assert_changes!(execution_semantics_manifest_preimage(
            8,
            &definition_digest,
            lifecycle_body.as_bytes(),
            &lifecycle_digest,
            config_body.as_bytes(),
            &config_digest,
            &required,
            &bindings,
        ));

        let mut changed_definition_digest = definition_digest;
        changed_definition_digest[0] ^= 1;
        assert_changes!(execution_semantics_manifest_preimage(
            definitions.engine_protocol().get(),
            &changed_definition_digest,
            lifecycle_body.as_bytes(),
            &lifecycle_digest,
            config_body.as_bytes(),
            &config_digest,
            &required,
            &bindings,
        ));

        let mut changed_lifecycle_body = lifecycle_body.as_bytes().to_vec();
        changed_lifecycle_body[0] ^= 1;
        assert_changes!(execution_semantics_manifest_preimage(
            definitions.engine_protocol().get(),
            &definition_digest,
            &changed_lifecycle_body,
            &lifecycle_digest,
            config_body.as_bytes(),
            &config_digest,
            &required,
            &bindings,
        ));

        let mut changed_lifecycle_digest = lifecycle_digest;
        changed_lifecycle_digest[0] ^= 1;
        assert_changes!(execution_semantics_manifest_preimage(
            definitions.engine_protocol().get(),
            &definition_digest,
            lifecycle_body.as_bytes(),
            &changed_lifecycle_digest,
            config_body.as_bytes(),
            &config_digest,
            &required,
            &bindings,
        ));

        let mut changed_config_body = config_body.as_bytes().to_vec();
        changed_config_body[0] ^= 1;
        assert_changes!(execution_semantics_manifest_preimage(
            definitions.engine_protocol().get(),
            &definition_digest,
            lifecycle_body.as_bytes(),
            &lifecycle_digest,
            &changed_config_body,
            &config_digest,
            &required,
            &bindings,
        ));

        let mut changed_config_digest = config_digest;
        changed_config_digest[0] ^= 1;
        assert_changes!(execution_semantics_manifest_preimage(
            definitions.engine_protocol().get(),
            &definition_digest,
            lifecycle_body.as_bytes(),
            &lifecycle_digest,
            config_body.as_bytes(),
            &changed_config_digest,
            &required,
            &bindings,
        ));

        let mut changed_key = required.clone();
        changed_key[0] = reference("test.changed", 1, 0x11);
        assert_changes!(execution_semantics_manifest_preimage(
            definitions.engine_protocol().get(),
            &definition_digest,
            lifecycle_body.as_bytes(),
            &lifecycle_digest,
            config_body.as_bytes(),
            &config_digest,
            &changed_key,
            &bindings,
        ));

        let mut changed_version = required.clone();
        changed_version[0] = SemanticInterfaceReference::new(
            changed_version[0].key().clone(),
            valid(InterfaceVersion::new(2)),
            changed_version[0].digest(),
        );
        assert_changes!(execution_semantics_manifest_preimage(
            definitions.engine_protocol().get(),
            &definition_digest,
            lifecycle_body.as_bytes(),
            &lifecycle_digest,
            config_body.as_bytes(),
            &config_digest,
            &changed_version,
            &bindings,
        ));

        let mut changed_interface_digest = required.clone();
        changed_interface_digest[0] = SemanticInterfaceReference::new(
            changed_interface_digest[0].key().clone(),
            changed_interface_digest[0].version(),
            SemanticInterfaceDigest::from_bytes([0x99; 32]),
        );
        assert_changes!(execution_semantics_manifest_preimage(
            definitions.engine_protocol().get(),
            &definition_digest,
            lifecycle_body.as_bytes(),
            &lifecycle_digest,
            config_body.as_bytes(),
            &config_digest,
            &changed_interface_digest,
            &bindings,
        ));

        let mut changed_implementation = bindings.clone();
        changed_implementation[0] = SemanticImplementationBinding::new(
            changed_implementation[0].interface().clone(),
            SemanticImplementationId::from_bytes([0xee; 32]),
        );
        assert_changes!(execution_semantics_manifest_preimage(
            definitions.engine_protocol().get(),
            &definition_digest,
            lifecycle_body.as_bytes(),
            &lifecycle_digest,
            config_body.as_bytes(),
            &config_digest,
            &required,
            &changed_implementation,
        ));
    }
}
