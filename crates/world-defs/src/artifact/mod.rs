mod codec;

use core::fmt;

use world_core::{
    CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest, DigestAlgorithm,
    SELECTED_DIGEST_ALGORITHM,
};

use crate::definition::{
    ActionBindingData, ActionData, EffectCallData, EventData, EventEmissionData,
    EventFieldBindingData, EventFieldData, OperationCallData, RuntimeRequirementData,
};
use crate::interface::{
    CatalogError, OperationKind, OperationName, SemanticInterfaceCatalog,
    SemanticInterfaceDescriptor, SemanticInterfaceKey, SemanticInterfaceReference, ValueKind,
};
use crate::key::{
    BindingName, DefinitionKey, EngineProtocolVersion, EventFieldName, LocalDefinitionName,
    PackCoordinate, PackKey,
};

pub use codec::CodecError as ArtifactCodecError;

/// MIME type of every W2 pack artifact.
pub const ARTIFACT_MEDIA_TYPE: &str = "application/vnd.world.pack+cbor";

/// Schema version of the ArtifactBlobV1 storage representation.
pub const ARTIFACT_FORMAT_VERSION: u16 = 1;

/// Schema version of the manifest nested in an artifact.
const MANIFEST_SCHEMA_VERSION: u16 = 1;

/// Schema version of every W2 action and event family.
const DEFINITION_FAMILY_SCHEMA_VERSION: u16 = 1;

/// Maximum exact artifact size accepted or emitted by W2.
pub const MAX_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;

/// Maximum direct dependencies declared by one pack.
pub const MAX_DIRECT_DEPENDENCIES: usize = 128;

/// Maximum semantic interfaces used by one artifact.
pub const MAX_REQUIRED_INTERFACES: usize = 128;

/// Maximum action and event definitions in one artifact.
pub const MAX_DEFINITIONS_PER_ARTIFACT: usize = 4_096;

/// Maximum named bindings declared by one action.
pub const MAX_ACTION_BINDINGS: usize = 32;

/// Maximum runtime requirements declared by one action.
pub const MAX_REQUIREMENTS_PER_ACTION: usize = 64;

/// Maximum effect calls declared by one action.
pub const MAX_EFFECTS_PER_ACTION: usize = 256;

/// Maximum physical events emitted by one successful action.
pub const MAX_SUCCESS_EVENTS_PER_ACTION: usize = 32;

/// Maximum fields declared or mapped by one physical event.
pub const MAX_EVENT_FIELDS: usize = 32;

/// Maximum positional arguments supplied to one semantic operation.
pub const MAX_OPERATION_ARGUMENTS: usize = 32;

const PACK_EXPORT_DOMAIN: CanonicalDomain = match CanonicalDomain::new("pack-exports-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("pack export identity domain must be valid"),
};
const PACK_SEMANTICS_DOMAIN: CanonicalDomain = match CanonicalDomain::new("pack-semantics-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("pack semantic identity domain must be valid"),
};
const PACK_EXPORT_SCHEMA_VERSION: u16 = 1;
const PACK_SEMANTICS_SCHEMA_VERSION: u16 = 1;

macro_rules! artifact_digest_type {
    ($(#[$metadata:meta])* $name:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(ContentDigest);

        impl $name {
            /// Constructs a fixed-width digest decoded from an owning
            /// representation.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(ContentDigest::from_bytes(bytes))
            }

            /// Returns the exact digest bytes.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                self.0.as_bytes()
            }

            /// Consumes the value and returns its exact digest bytes.
            #[must_use]
            pub const fn into_bytes(self) -> [u8; 32] {
                self.0.into_bytes()
            }

            pub(crate) const fn from_content_digest(digest: ContentDigest) -> Self {
                Self(digest)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, formatter)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}({self})", stringify!($name))
            }
        }
    };
}

artifact_digest_type!(
    /// BLAKE3-256 identity of exact stored artifact bytes.
    ArtifactDigest
);
artifact_digest_type!(
    /// Canonical identity of one pack's public definition signatures.
    PackExportDigest
);
artifact_digest_type!(
    /// Canonical identity of one pack's normalized executable semantics.
    RuntimeSemanticFingerprint
);

impl ArtifactDigest {
    /// Hashes exact ArtifactBlobV1 bytes.
    #[must_use]
    pub fn of_blob_bytes(bytes: &[u8]) -> Self {
        Self::from_content_digest(ContentDigest::of_blob_bytes(bytes))
    }
}

/// One exact direct dependency and the public surface expected from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackDependency {
    coordinate: PackCoordinate,
    expected_export_digest: PackExportDigest,
}

impl PackDependency {
    /// Creates an exact dependency reference.
    #[must_use]
    pub const fn new(coordinate: PackCoordinate, expected_export_digest: PackExportDigest) -> Self {
        Self {
            coordinate,
            expected_export_digest,
        }
    }

    /// Returns the selected dependency coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &PackCoordinate {
        &self.coordinate
    }

    /// Returns the expected public-signature identity.
    #[must_use]
    pub const fn expected_export_digest(&self) -> PackExportDigest {
        self.expected_export_digest
    }
}

/// Input representation of the manifest embedded in one artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackManifestData {
    engine_protocol: EngineProtocolVersion,
    coordinate: PackCoordinate,
    dependencies: Vec<PackDependency>,
}

impl PackManifestData {
    /// Creates manifest input without claiming graph closure.
    #[must_use]
    pub fn new(
        engine_protocol: EngineProtocolVersion,
        coordinate: PackCoordinate,
        dependencies: Vec<PackDependency>,
    ) -> Self {
        Self {
            engine_protocol,
            coordinate,
            dependencies,
        }
    }

    /// Returns the required engine protocol.
    #[must_use]
    pub const fn engine_protocol(&self) -> EngineProtocolVersion {
        self.engine_protocol
    }

    /// Returns the exact pack coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &PackCoordinate {
        &self.coordinate
    }

    /// Returns direct dependencies in their current representation order.
    #[must_use]
    pub fn dependencies(&self) -> &[PackDependency] {
        &self.dependencies
    }

    fn into_parts(self) -> (EngineProtocolVersion, PackCoordinate, Vec<PackDependency>) {
        (self.engine_protocol, self.coordinate, self.dependencies)
    }
}

/// Compiler- or decoder-produced representation of one pack artifact.
///
/// Construction validates only leaf values. [`ArtifactValidator`] establishes
/// whole-artifact invariants and creates a sealed value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactData {
    manifest: PackManifestData,
    interfaces: Vec<SemanticInterfaceReference>,
    actions: Vec<ActionData>,
    events: Vec<EventData>,
}

impl ArtifactData {
    /// Creates unchecked aggregate data from checked leaves.
    #[must_use]
    pub fn new(
        manifest: PackManifestData,
        interfaces: Vec<SemanticInterfaceReference>,
        actions: Vec<ActionData>,
        events: Vec<EventData>,
    ) -> Self {
        Self {
            manifest,
            interfaces,
            actions,
            events,
        }
    }

    /// Returns the embedded manifest.
    #[must_use]
    pub const fn manifest(&self) -> &PackManifestData {
        &self.manifest
    }

    /// Returns exact semantic-interface references.
    #[must_use]
    pub fn interfaces(&self) -> &[SemanticInterfaceReference] {
        &self.interfaces
    }

    /// Returns action input data.
    #[must_use]
    pub fn actions(&self) -> &[ActionData] {
        &self.actions
    }

    /// Returns physical-event input data.
    #[must_use]
    pub fn events(&self) -> &[EventData] {
        &self.events
    }

    fn into_parts(
        self,
    ) -> (
        PackManifestData,
        Vec<SemanticInterfaceReference>,
        Vec<ActionData>,
        Vec<EventData>,
    ) {
        (self.manifest, self.interfaces, self.actions, self.events)
    }
}

/// Closed media-type family for compiled world packs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArtifactMediaType {
    /// Deterministic CBOR using the ArtifactBlobV1 schema.
    WorldPackCbor,
}

impl ArtifactMediaType {
    /// Returns the registered media-type string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WorldPackCbor => ARTIFACT_MEDIA_TYPE,
        }
    }
}

/// Exact storage metadata carried beside one artifact blob.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactDescriptor {
    media_type: ArtifactMediaType,
    format_version: u16,
    digest_algorithm: DigestAlgorithm,
    blob_length: u64,
    blob_digest: ArtifactDigest,
}

impl ArtifactDescriptor {
    /// Creates descriptor input. Loading verifies it against the exact bytes.
    #[must_use]
    pub const fn new(
        media_type: ArtifactMediaType,
        format_version: u16,
        digest_algorithm: DigestAlgorithm,
        blob_length: u64,
        blob_digest: ArtifactDigest,
    ) -> Self {
        Self {
            media_type,
            format_version,
            digest_algorithm,
            blob_length,
            blob_digest,
        }
    }

    fn for_blob(bytes: &[u8]) -> Self {
        Self::new(
            ArtifactMediaType::WorldPackCbor,
            ARTIFACT_FORMAT_VERSION,
            SELECTED_DIGEST_ALGORITHM,
            bytes.len() as u64,
            ArtifactDigest::of_blob_bytes(bytes),
        )
    }

    /// Returns the closed media type.
    #[must_use]
    pub const fn media_type(&self) -> ArtifactMediaType {
        self.media_type
    }

    /// Returns the artifact schema version.
    #[must_use]
    pub const fn format_version(&self) -> u16 {
        self.format_version
    }

    /// Returns the exact-byte digest algorithm.
    #[must_use]
    pub const fn digest_algorithm(&self) -> DigestAlgorithm {
        self.digest_algorithm
    }

    /// Returns the declared exact byte length.
    #[must_use]
    pub const fn blob_length(&self) -> u64 {
        self.blob_length
    }

    /// Returns the declared exact-byte identity.
    #[must_use]
    pub const fn blob_digest(&self) -> ArtifactDigest {
        self.blob_digest
    }
}

/// An unchecked serialized artifact and its exact storage descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactEnvelope {
    descriptor: ArtifactDescriptor,
    blob: Vec<u8>,
}

impl ArtifactEnvelope {
    /// Creates envelope input. The descriptor is checked only when loaded.
    #[must_use]
    pub fn new(descriptor: ArtifactDescriptor, blob: Vec<u8>) -> Self {
        Self { descriptor, blob }
    }

    /// Creates a descriptor matching the supplied exact bytes.
    ///
    /// This does not decode or semantically validate the artifact.
    fn from_blob(blob: Vec<u8>) -> Self {
        let descriptor = ArtifactDescriptor::for_blob(&blob);
        Self { descriptor, blob }
    }

    /// Returns exact storage metadata.
    #[must_use]
    pub const fn descriptor(&self) -> &ArtifactDescriptor {
        &self.descriptor
    }

    /// Returns the exact stored bytes.
    #[must_use]
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }

    /// Consumes the envelope into its descriptor and bytes.
    #[must_use]
    pub fn into_parts(self) -> (ArtifactDescriptor, Vec<u8>) {
        (self.descriptor, self.blob)
    }
}

/// Artifact-validated action definition.
///
/// Only [`ArtifactValidator`] can construct this value. Its calls remain bound
/// to the containing artifact's exact interface table, and cross-pack event
/// signatures are resolved only when a [`crate::RuntimeDefinitionSet`] is
/// linked. Runtime consumers therefore use definitions through that aggregate
/// rather than treating a cloned action as an independent proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionDefinition {
    data: ActionData,
}

impl ActionDefinition {
    /// Returns the pack-local definition name.
    #[must_use]
    pub const fn name(&self) -> &LocalDefinitionName {
        self.data.name()
    }

    /// Returns bindings in canonical name order.
    #[must_use]
    pub fn bindings(&self) -> &[ActionBindingData] {
        self.data.bindings()
    }

    /// Returns requirements in canonical call order.
    #[must_use]
    pub fn requirements(&self) -> &[RuntimeRequirementData] {
        self.data.requirements()
    }

    /// Returns effects in semantic execution order.
    #[must_use]
    pub fn effects(&self) -> &[EffectCallData] {
        self.data.effects()
    }

    /// Returns physical events in semantic emission order.
    #[must_use]
    pub fn success_events(&self) -> &[EventEmissionData] {
        self.data.success_events()
    }
}

/// Whole-artifact-checked physical-event definition.
///
/// Only [`ArtifactValidator`] can construct this value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDefinition {
    data: EventData,
}

impl EventDefinition {
    /// Returns the pack-local definition name.
    #[must_use]
    pub const fn name(&self) -> &LocalDefinitionName {
        self.data.name()
    }

    /// Returns fields in canonical name order.
    #[must_use]
    pub fn fields(&self) -> &[EventFieldData] {
        self.data.fields()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckedArtifactData {
    manifest: PackManifestData,
    interfaces: Vec<SemanticInterfaceReference>,
    actions: Vec<ActionDefinition>,
    events: Vec<EventDefinition>,
}

impl CheckedArtifactData {
    pub(crate) const fn manifest(&self) -> &PackManifestData {
        &self.manifest
    }

    pub(crate) fn interfaces(&self) -> &[SemanticInterfaceReference] {
        &self.interfaces
    }

    pub(crate) fn actions(&self) -> &[ActionDefinition] {
        &self.actions
    }

    pub(crate) fn events(&self) -> &[EventDefinition] {
        &self.events
    }
}

struct ValidatedArtifactData {
    data: CheckedArtifactData,
    export_digest: PackExportDigest,
    semantic_fingerprint: RuntimeSemanticFingerprint,
}

/// Sealed, immutable compiled pack artifact.
///
/// The exact envelope, normalized definitions, interface closure, and derived
/// identities have passed the same semantic validator regardless of whether
/// the input came directly from a compiler or from stored bytes.
///
/// ```compile_fail
/// let _ = world_defs::VerifiedPackArtifact {};
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedPackArtifact {
    data: CheckedArtifactData,
    envelope: ArtifactEnvelope,
    export_digest: PackExportDigest,
    semantic_fingerprint: RuntimeSemanticFingerprint,
}

impl VerifiedPackArtifact {
    /// Returns the exact pack coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &PackCoordinate {
        self.data.manifest.coordinate()
    }

    /// Returns the required engine protocol.
    #[must_use]
    pub const fn engine_protocol(&self) -> EngineProtocolVersion {
        self.data.manifest.engine_protocol()
    }

    /// Returns exact direct dependency declarations.
    #[must_use]
    pub fn dependencies(&self) -> &[PackDependency] {
        self.data.manifest.dependencies()
    }

    /// Returns the exact semantic-interface closure.
    #[must_use]
    pub fn required_interfaces(&self) -> &[SemanticInterfaceReference] {
        self.data.interfaces()
    }

    /// Returns normalized, validated actions.
    #[must_use]
    pub fn actions(&self) -> &[ActionDefinition] {
        self.data.actions()
    }

    /// Returns normalized, validated physical events.
    #[must_use]
    pub fn events(&self) -> &[EventDefinition] {
        self.data.events()
    }

    /// Returns the exact stored artifact envelope.
    #[must_use]
    pub const fn envelope(&self) -> &ArtifactEnvelope {
        &self.envelope
    }

    /// Returns the BLAKE3 identity of exact stored bytes.
    #[must_use]
    pub const fn artifact_digest(&self) -> ArtifactDigest {
        self.envelope.descriptor.blob_digest
    }

    /// Returns the canonical public-signature identity.
    #[must_use]
    pub const fn export_digest(&self) -> PackExportDigest {
        self.export_digest
    }

    /// Returns the canonical normalized behavior identity.
    #[must_use]
    pub const fn semantic_fingerprint(&self) -> RuntimeSemanticFingerprint {
        self.semantic_fingerprint
    }
}

/// Catalog-bound owner of artifact validation and loading.
#[derive(Clone, Copy, Debug)]
pub struct ArtifactValidator<'catalog> {
    catalog: &'catalog SemanticInterfaceCatalog,
}

impl<'catalog> ArtifactValidator<'catalog> {
    /// Binds artifact validation to the available interface declarations.
    #[must_use]
    pub const fn new(catalog: &'catalog SemanticInterfaceCatalog) -> Self {
        Self { catalog }
    }

    /// Validates compiler-produced domain data and emits it exactly once.
    pub fn validate(&self, data: ArtifactData) -> Result<VerifiedPackArtifact, ArtifactError> {
        let validated = validate_artifact_data(data, self.catalog)?;
        let blob = codec::encode(&validated.data);
        if blob.len() > MAX_ARTIFACT_BYTES {
            return Err(ArtifactError::ArtifactTooLarge {
                actual: blob.len(),
                maximum: MAX_ARTIFACT_BYTES,
            });
        }
        let envelope = ArtifactEnvelope::from_blob(blob);
        Ok(seal_artifact(validated, envelope))
    }

    /// Loads exact bytes, then applies the same semantic validator used by
    /// direct compiler data.
    pub fn load(&self, envelope: ArtifactEnvelope) -> Result<VerifiedPackArtifact, ArtifactError> {
        validate_envelope(&envelope)?;
        let data = codec::decode(envelope.blob()).map_err(ArtifactError::Codec)?;
        let validated = validate_artifact_data(data, self.catalog)?;
        Ok(seal_artifact(validated, envelope))
    }
}

fn seal_artifact(
    validated: ValidatedArtifactData,
    envelope: ArtifactEnvelope,
) -> VerifiedPackArtifact {
    VerifiedPackArtifact {
        data: validated.data,
        envelope,
        export_digest: validated.export_digest,
        semantic_fingerprint: validated.semantic_fingerprint,
    }
}

fn validate_envelope(envelope: &ArtifactEnvelope) -> Result<(), ArtifactError> {
    let descriptor = envelope.descriptor();
    match descriptor.media_type() {
        ArtifactMediaType::WorldPackCbor => {}
    }
    match descriptor.digest_algorithm() {
        DigestAlgorithm::Blake3_256 => {}
    }
    if descriptor.format_version() != ARTIFACT_FORMAT_VERSION {
        return Err(ArtifactError::UnsupportedFormatVersion {
            actual: descriptor.format_version(),
        });
    }
    if envelope.blob().len() > MAX_ARTIFACT_BYTES {
        return Err(ArtifactError::ArtifactTooLarge {
            actual: envelope.blob().len(),
            maximum: MAX_ARTIFACT_BYTES,
        });
    }
    let actual_length = envelope.blob().len() as u64;
    if descriptor.blob_length() != actual_length {
        return Err(ArtifactError::LengthMismatch {
            declared: descriptor.blob_length(),
            actual: actual_length,
        });
    }
    let actual_digest = ArtifactDigest::of_blob_bytes(envelope.blob());
    if descriptor.blob_digest() != actual_digest {
        return Err(ArtifactError::DigestMismatch {
            declared: descriptor.blob_digest(),
            actual: actual_digest,
        });
    }
    Ok(())
}

fn validate_artifact_data(
    data: ArtifactData,
    catalog: &SemanticInterfaceCatalog,
) -> Result<ValidatedArtifactData, ArtifactError> {
    let (manifest, mut interfaces, actions, events) = data.into_parts();
    let (engine_protocol, coordinate, mut dependencies) = manifest.into_parts();

    check_limit(
        "direct dependencies",
        dependencies.len(),
        MAX_DIRECT_DEPENDENCIES,
    )?;
    dependencies.sort_by(|left, right| left.coordinate.pack_key().cmp(right.coordinate.pack_key()));
    for adjacent in dependencies.windows(2) {
        if adjacent[0].coordinate.pack_key() == adjacent[1].coordinate.pack_key() {
            return Err(ArtifactError::DuplicateDependency {
                pack: adjacent[0].coordinate.pack_key().clone(),
            });
        }
    }

    check_limit(
        "required interfaces",
        interfaces.len(),
        MAX_REQUIRED_INTERFACES,
    )?;
    interfaces.sort_by(|left, right| left.key().cmp(right.key()));
    for adjacent in interfaces.windows(2) {
        if adjacent[0].key() == adjacent[1].key() {
            return Err(ArtifactError::DuplicateInterface {
                interface: adjacent[0].key().clone(),
            });
        }
    }
    let resolved_interfaces = interfaces
        .iter()
        .map(|reference| {
            catalog
                .resolve(reference)
                .map_err(ArtifactError::InterfaceCatalog)
        })
        .collect::<Result<Vec<_>, ArtifactError>>()?;

    let definition_count = actions.len() + events.len();
    check_limit(
        "definitions",
        definition_count,
        MAX_DEFINITIONS_PER_ARTIFACT,
    )?;

    let events = normalize_events(events)?;
    let actions = normalize_actions(
        actions,
        &coordinate,
        &dependencies,
        &interfaces,
        &resolved_interfaces,
        &events,
    )?;
    validate_definition_namespace(&actions, &events)?;

    let manifest = PackManifestData::new(engine_protocol, coordinate, dependencies);
    let checked = CheckedArtifactData {
        manifest,
        interfaces,
        actions,
        events,
    };
    let export_digest = compute_export_digest(&checked)?;
    let semantic_fingerprint = compute_semantic_fingerprint(&checked)?;
    Ok(ValidatedArtifactData {
        data: checked,
        export_digest,
        semantic_fingerprint,
    })
}

fn normalize_events(events: Vec<EventData>) -> Result<Vec<EventDefinition>, ArtifactError> {
    let mut normalized = Vec::with_capacity(events.len());
    for event in events {
        let (name, mut fields) = event.into_parts();
        if fields.is_empty() {
            return Err(ArtifactError::EmptyEvent {
                event: name.clone(),
            });
        }
        check_limit("event fields", fields.len(), MAX_EVENT_FIELDS)?;
        fields.sort_by(|left, right| left.name().cmp(right.name()));
        for adjacent in fields.windows(2) {
            if adjacent[0].name() == adjacent[1].name() {
                return Err(ArtifactError::DuplicateEventField {
                    event: name.clone(),
                    field: adjacent[0].name().clone(),
                });
            }
        }
        normalized.push(EventDefinition {
            data: EventData::new(name, fields),
        });
    }
    normalized.sort_by(|left, right| left.name().cmp(right.name()));
    for adjacent in normalized.windows(2) {
        if adjacent[0].name() == adjacent[1].name() {
            return Err(ArtifactError::DuplicateDefinition {
                definition: adjacent[0].name().clone(),
            });
        }
    }
    Ok(normalized)
}

fn normalize_actions(
    actions: Vec<ActionData>,
    coordinate: &PackCoordinate,
    dependencies: &[PackDependency],
    interfaces: &[SemanticInterfaceReference],
    resolved_interfaces: &[&SemanticInterfaceDescriptor],
    events: &[EventDefinition],
) -> Result<Vec<ActionDefinition>, ArtifactError> {
    let mut normalized = Vec::with_capacity(actions.len());
    let mut used_interfaces = Vec::new();

    for action in actions {
        let (name, mut bindings, mut requirements, effects, success_events) = action.into_parts();
        check_limit("action bindings", bindings.len(), MAX_ACTION_BINDINGS)?;
        check_limit(
            "runtime requirements",
            requirements.len(),
            MAX_REQUIREMENTS_PER_ACTION,
        )?;
        check_limit("effect calls", effects.len(), MAX_EFFECTS_PER_ACTION)?;
        check_limit(
            "success events",
            success_events.len(),
            MAX_SUCCESS_EVENTS_PER_ACTION,
        )?;
        if effects.is_empty() {
            return Err(ArtifactError::EmptyEffects {
                action: name.clone(),
            });
        }
        if success_events.is_empty() {
            return Err(ArtifactError::EmptySuccessEvents {
                action: name.clone(),
            });
        }

        bindings.sort_by(|left, right| left.name().cmp(right.name()));
        for adjacent in bindings.windows(2) {
            if adjacent[0].name() == adjacent[1].name() {
                return Err(ArtifactError::DuplicateBinding {
                    action: name.clone(),
                    binding: adjacent[0].name().clone(),
                });
            }
        }

        requirements.sort();
        for requirement in &requirements {
            validate_operation_call(
                &name,
                requirement.call(),
                OperationKind::Predicate,
                &bindings,
                interfaces,
                resolved_interfaces,
                &mut used_interfaces,
            )?;
        }
        for effect in &effects {
            validate_operation_call(
                &name,
                effect.call(),
                OperationKind::Effect,
                &bindings,
                interfaces,
                resolved_interfaces,
                &mut used_interfaces,
            )?;
        }

        let mut normalized_events = Vec::with_capacity(success_events.len());
        for emission in success_events {
            normalized_events.push(normalize_event_emission(
                &name,
                emission,
                coordinate,
                dependencies,
                &bindings,
                events,
            )?);
        }

        normalized.push(ActionDefinition {
            data: ActionData::new(name, bindings, requirements, effects, normalized_events),
        });
    }

    normalized.sort_by(|left, right| left.name().cmp(right.name()));
    for adjacent in normalized.windows(2) {
        if adjacent[0].name() == adjacent[1].name() {
            return Err(ArtifactError::DuplicateDefinition {
                definition: adjacent[0].name().clone(),
            });
        }
    }

    used_interfaces.sort();
    used_interfaces.dedup();
    for reference in interfaces {
        if used_interfaces
            .binary_search_by(|key| key.cmp(reference.key()))
            .is_err()
        {
            return Err(ArtifactError::UnusedInterface {
                interface: reference.key().clone(),
            });
        }
    }

    Ok(normalized)
}

#[allow(clippy::too_many_arguments)]
fn validate_operation_call(
    action: &LocalDefinitionName,
    call: &OperationCallData,
    expected_kind: OperationKind,
    bindings: &[ActionBindingData],
    interfaces: &[SemanticInterfaceReference],
    resolved_interfaces: &[&SemanticInterfaceDescriptor],
    used_interfaces: &mut Vec<SemanticInterfaceKey>,
) -> Result<(), ArtifactError> {
    check_limit(
        "operation arguments",
        call.arguments().len(),
        MAX_OPERATION_ARGUMENTS,
    )?;
    let reference_index = interfaces
        .binary_search_by(|reference| reference.key().cmp(call.interface()))
        .map_err(|_| ArtifactError::MissingInterfaceReference {
            action: action.clone(),
            interface: call.interface().clone(),
        })?;
    let descriptor = resolved_interfaces[reference_index];
    let operation =
        descriptor
            .operation(call.operation())
            .ok_or_else(|| ArtifactError::UnknownOperation {
                action: action.clone(),
                interface: call.interface().clone(),
                operation: call.operation().clone(),
            })?;
    if operation.kind() != expected_kind {
        return Err(ArtifactError::WrongOperationStage {
            action: action.clone(),
            interface: call.interface().clone(),
            operation: call.operation().clone(),
            expected: expected_kind,
            actual: operation.kind(),
        });
    }
    if call.arguments().len() != operation.parameters().len() {
        return Err(ArtifactError::OperationArityMismatch {
            action: action.clone(),
            interface: call.interface().clone(),
            operation: call.operation().clone(),
            expected: operation.parameters().len(),
            actual: call.arguments().len(),
        });
    }
    for (argument_index, (binding_name, parameter)) in call
        .arguments()
        .iter()
        .zip(operation.parameters())
        .enumerate()
    {
        let binding_index = bindings
            .binary_search_by(|binding| binding.name().cmp(binding_name))
            .map_err(|_| ArtifactError::UnknownBinding {
                action: action.clone(),
                binding: binding_name.clone(),
            })?;
        let binding = &bindings[binding_index];
        if *binding.value_kind() != parameter.value_kind() {
            return Err(ArtifactError::BindingKindMismatch {
                action: action.clone(),
                binding: binding_name.clone(),
                argument_index,
                expected: parameter.value_kind(),
                actual: *binding.value_kind(),
            });
        }
    }
    used_interfaces.push(call.interface().clone());
    Ok(())
}

fn normalize_event_emission(
    action: &LocalDefinitionName,
    emission: EventEmissionData,
    coordinate: &PackCoordinate,
    dependencies: &[PackDependency],
    bindings: &[ActionBindingData],
    events: &[EventDefinition],
) -> Result<EventEmissionData, ArtifactError> {
    let (event, mut field_bindings) = emission.into_parts();
    if field_bindings.is_empty() {
        return Err(ArtifactError::EmptyEventMapping {
            action: action.clone(),
            event,
        });
    }
    check_limit(
        "event field mappings",
        field_bindings.len(),
        MAX_EVENT_FIELDS,
    )?;
    field_bindings.sort_by(|left, right| left.field().cmp(right.field()));
    for adjacent in field_bindings.windows(2) {
        if adjacent[0].field() == adjacent[1].field() {
            return Err(ArtifactError::DuplicateEventMapping {
                action: action.clone(),
                event: event.clone(),
                field: adjacent[0].field().clone(),
            });
        }
    }
    if event.pack_key() == coordinate.pack_key() {
        validate_local_event_mapping(action, &event, &field_bindings, bindings, events)?;
    } else {
        if dependencies
            .binary_search_by(|dependency| dependency.coordinate.pack_key().cmp(event.pack_key()))
            .is_err()
        {
            return Err(ArtifactError::UndeclaredEventPack {
                action: action.clone(),
                event,
            });
        }
        for mapping in &field_bindings {
            if bindings
                .binary_search_by(|binding| binding.name().cmp(mapping.binding()))
                .is_err()
            {
                return Err(ArtifactError::UnknownBinding {
                    action: action.clone(),
                    binding: mapping.binding().clone(),
                });
            }
        }
    }

    Ok(EventEmissionData::new(event, field_bindings))
}

fn validate_local_event_mapping(
    action: &LocalDefinitionName,
    event: &DefinitionKey,
    mappings: &[EventFieldBindingData],
    bindings: &[ActionBindingData],
    events: &[EventDefinition],
) -> Result<(), ArtifactError> {
    let event_index = events
        .binary_search_by(|candidate| candidate.name().cmp(event.local_name()))
        .map_err(|_| ArtifactError::MissingLocalEvent {
            action: action.clone(),
            event: event.clone(),
        })?;
    let target = &events[event_index];
    if mappings.len() != target.fields().len() {
        return Err(ArtifactError::EventMappingArityMismatch {
            action: action.clone(),
            event: event.clone(),
            expected: target.fields().len(),
            actual: mappings.len(),
        });
    }
    for (mapping, field) in mappings.iter().zip(target.fields()) {
        if mapping.field() != field.name() {
            return Err(ArtifactError::EventFieldMismatch {
                action: action.clone(),
                event: event.clone(),
                expected: field.name().clone(),
                actual: mapping.field().clone(),
            });
        }
        let binding_index = bindings
            .binary_search_by(|binding| binding.name().cmp(mapping.binding()))
            .map_err(|_| ArtifactError::UnknownBinding {
                action: action.clone(),
                binding: mapping.binding().clone(),
            })?;
        let binding = &bindings[binding_index];
        if *binding.value_kind() != *field.value_kind() {
            return Err(ArtifactError::EventFieldKindMismatch {
                action: action.clone(),
                event: event.clone(),
                field: field.name().clone(),
                binding: mapping.binding().clone(),
                expected: *field.value_kind(),
                actual: *binding.value_kind(),
            });
        }
    }
    Ok(())
}

fn validate_definition_namespace(
    actions: &[ActionDefinition],
    events: &[EventDefinition],
) -> Result<(), ArtifactError> {
    let mut action_index = 0;
    let mut event_index = 0;
    while action_index < actions.len() && event_index < events.len() {
        match actions[action_index].name().cmp(events[event_index].name()) {
            core::cmp::Ordering::Less => action_index += 1,
            core::cmp::Ordering::Greater => event_index += 1,
            core::cmp::Ordering::Equal => {
                return Err(ArtifactError::DuplicateDefinition {
                    definition: actions[action_index].name().clone(),
                });
            }
        }
    }
    Ok(())
}

fn check_limit(
    collection: &'static str,
    actual: usize,
    maximum: usize,
) -> Result<(), ArtifactError> {
    if actual > maximum {
        return Err(ArtifactError::CollectionLimit {
            collection,
            actual,
            maximum,
        });
    }
    Ok(())
}

enum DefinitionRef<'definition> {
    Action(&'definition ActionDefinition),
    Event(&'definition EventDefinition),
}

impl DefinitionRef<'_> {
    fn name(&self) -> &LocalDefinitionName {
        match self {
            Self::Action(action) => action.name(),
            Self::Event(event) => event.name(),
        }
    }
}

fn ordered_definitions(data: &CheckedArtifactData) -> Vec<DefinitionRef<'_>> {
    let mut definitions = Vec::with_capacity(data.actions.len() + data.events.len());
    definitions.extend(data.actions.iter().map(DefinitionRef::Action));
    definitions.extend(data.events.iter().map(DefinitionRef::Event));
    definitions.sort_by(|left, right| left.name().cmp(right.name()));
    definitions
}

fn compute_export_digest(data: &CheckedArtifactData) -> Result<PackExportDigest, ArtifactError> {
    let mut writer = CanonicalWriter::new(PACK_EXPORT_DOMAIN);
    writer.write_u16(PACK_EXPORT_SCHEMA_VERSION);
    write_coordinate(&mut writer, data.manifest.coordinate())?;
    let definitions = ordered_definitions(data);
    writer.write_sequence(&definitions, |writer, definition| {
        match definition {
            DefinitionRef::Action(action) => {
                writer.write_discriminant(0);
                writer.write_u16(DEFINITION_FAMILY_SCHEMA_VERSION);
                writer.write_str(action.name().as_str())?;
                writer.write_sequence(action.bindings(), |writer, binding| {
                    writer.write_str(binding.name().as_str())?;
                    writer.write_discriminant(value_kind_tag(*binding.value_kind()));
                    Ok(())
                })?;
            }
            DefinitionRef::Event(event) => {
                writer.write_discriminant(1);
                writer.write_u16(DEFINITION_FAMILY_SCHEMA_VERSION);
                writer.write_str(event.name().as_str())?;
                writer.write_sequence(event.fields(), |writer, field| {
                    writer.write_str(field.name().as_str())?;
                    writer.write_discriminant(value_kind_tag(*field.value_kind()));
                    Ok(())
                })?;
            }
        }
        Ok(())
    })?;
    Ok(PackExportDigest::from_content_digest(
        ContentDigest::of_canonical(&writer.finish()),
    ))
}

fn compute_semantic_fingerprint(
    data: &CheckedArtifactData,
) -> Result<RuntimeSemanticFingerprint, ArtifactError> {
    let mut writer = CanonicalWriter::new(PACK_SEMANTICS_DOMAIN);
    writer.write_u16(PACK_SEMANTICS_SCHEMA_VERSION);
    writer.write_u16(data.manifest.engine_protocol().get());
    write_coordinate(&mut writer, data.manifest.coordinate())?;
    writer.write_sequence(data.manifest.dependencies(), |writer, dependency| {
        write_coordinate(writer, dependency.coordinate())?;
        writer.write_bytes(dependency.expected_export_digest().as_bytes())
    })?;
    writer.write_sequence(data.interfaces(), write_interface_reference)?;

    let definitions = ordered_definitions(data);
    writer.write_sequence(&definitions, |writer, definition| {
        write_definition_semantics(writer, definition, data.interfaces())
    })?;
    Ok(RuntimeSemanticFingerprint::from_content_digest(
        ContentDigest::of_canonical(&writer.finish()),
    ))
}

fn write_coordinate(
    writer: &mut CanonicalWriter,
    coordinate: &PackCoordinate,
) -> Result<(), CanonicalError> {
    writer.write_str(coordinate.pack_key().as_str())?;
    writer.write_u32(coordinate.version().major());
    writer.write_u32(coordinate.version().minor());
    writer.write_u32(coordinate.version().patch());
    Ok(())
}

fn write_interface_reference(
    writer: &mut CanonicalWriter,
    reference: &SemanticInterfaceReference,
) -> Result<(), CanonicalError> {
    writer.write_str(reference.key().as_str())?;
    writer.write_u16(reference.version().get());
    writer.write_bytes(reference.digest().as_bytes())
}

fn write_definition_semantics(
    writer: &mut CanonicalWriter,
    definition: &DefinitionRef<'_>,
    interfaces: &[SemanticInterfaceReference],
) -> Result<(), CanonicalError> {
    match definition {
        DefinitionRef::Action(action) => {
            writer.write_discriminant(0);
            writer.write_u16(DEFINITION_FAMILY_SCHEMA_VERSION);
            writer.write_str(action.name().as_str())?;
            writer.write_sequence(action.bindings(), |writer, binding| {
                writer.write_str(binding.name().as_str())?;
                writer.write_discriminant(value_kind_tag(*binding.value_kind()));
                Ok(())
            })?;
            writer.write_sequence(action.requirements(), |writer, requirement| {
                write_operation_call(writer, requirement.call(), interfaces)
            })?;
            writer.write_sequence(action.effects(), |writer, effect| {
                write_operation_call(writer, effect.call(), interfaces)
            })?;
            writer.write_sequence(action.success_events(), |writer, emission| {
                write_definition_key(writer, emission.event())?;
                writer.write_sequence(emission.field_bindings(), |writer, mapping| {
                    writer.write_str(mapping.field().as_str())?;
                    writer.write_str(mapping.binding().as_str())
                })
            })?;
        }
        DefinitionRef::Event(event) => {
            writer.write_discriminant(1);
            writer.write_u16(DEFINITION_FAMILY_SCHEMA_VERSION);
            writer.write_str(event.name().as_str())?;
            writer.write_sequence(event.fields(), |writer, field| {
                writer.write_str(field.name().as_str())?;
                writer.write_discriminant(value_kind_tag(*field.value_kind()));
                Ok(())
            })?;
        }
    }
    Ok(())
}

fn write_operation_call(
    writer: &mut CanonicalWriter,
    call: &OperationCallData,
    interfaces: &[SemanticInterfaceReference],
) -> Result<(), CanonicalError> {
    let reference =
        match interfaces.binary_search_by(|reference| reference.key().cmp(call.interface())) {
            Ok(index) => &interfaces[index],
            Err(_) => unreachable!("checked operation call must have an interface reference"),
        };
    write_interface_reference(writer, reference)?;
    writer.write_str(call.operation().as_str())?;
    writer.write_sequence(call.arguments(), |writer, argument| {
        writer.write_str(argument.as_str())
    })
}

fn write_definition_key(
    writer: &mut CanonicalWriter,
    key: &DefinitionKey,
) -> Result<(), CanonicalError> {
    writer.write_str(key.pack_key().as_str())?;
    writer.write_str(key.local_name().as_str())
}

const fn value_kind_tag(kind: ValueKind) -> u32 {
    match kind {
        ValueKind::Actor => 0,
        ValueKind::Entity => 1,
    }
}

/// Why unchecked artifact input could not become a sealed pack artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactError {
    /// The envelope names an unsupported artifact schema.
    UnsupportedFormatVersion {
        /// Format version found in the descriptor.
        actual: u16,
    },
    /// The exact byte size exceeds the artifact boundary.
    ArtifactTooLarge {
        /// Exact supplied byte count.
        actual: usize,
        /// Maximum accepted byte count.
        maximum: usize,
    },
    /// Descriptor and actual byte lengths differ.
    LengthMismatch {
        /// Length carried by the descriptor.
        declared: u64,
        /// Exact supplied byte length.
        actual: u64,
    },
    /// Descriptor and actual byte identities differ.
    DigestMismatch {
        /// Digest carried by the descriptor.
        declared: ArtifactDigest,
        /// Digest of the exact supplied bytes.
        actual: ArtifactDigest,
    },
    /// ArtifactBlobV1 could not be decoded.
    Codec(ArtifactCodecError),
    /// One semantic collection exceeded its protocol limit.
    CollectionLimit {
        /// Domain collection being checked.
        collection: &'static str,
        /// Supplied element count.
        actual: usize,
        /// Maximum accepted element count.
        maximum: usize,
    },
    /// One direct dependency key appeared more than once.
    DuplicateDependency {
        /// Repeated pack key.
        pack: PackKey,
    },
    /// One interface key appeared more than once in the artifact table.
    DuplicateInterface {
        /// Repeated interface key.
        interface: SemanticInterfaceKey,
    },
    /// An interface reference did not resolve exactly in the supplied catalog.
    InterfaceCatalog(CatalogError),
    /// One local name appeared more than once across definition families.
    DuplicateDefinition {
        /// Repeated local name.
        definition: LocalDefinitionName,
    },
    /// A physical event declared no fields.
    EmptyEvent {
        /// Rejected event.
        event: LocalDefinitionName,
    },
    /// One event field name appeared more than once.
    DuplicateEventField {
        /// Event containing the duplicate.
        event: LocalDefinitionName,
        /// Repeated field.
        field: EventFieldName,
    },
    /// One action binding name appeared more than once.
    DuplicateBinding {
        /// Action containing the duplicate.
        action: LocalDefinitionName,
        /// Repeated binding.
        binding: BindingName,
    },
    /// An action declared no authoritative effect.
    EmptyEffects {
        /// Rejected action.
        action: LocalDefinitionName,
    },
    /// An action declared no physical success event.
    EmptySuccessEvents {
        /// Rejected action.
        action: LocalDefinitionName,
    },
    /// A call names an interface omitted from the exact artifact table.
    MissingInterfaceReference {
        /// Action containing the call.
        action: LocalDefinitionName,
        /// Missing interface.
        interface: SemanticInterfaceKey,
    },
    /// A referenced operation does not exist in its descriptor.
    UnknownOperation {
        /// Action containing the call.
        action: LocalDefinitionName,
        /// Referenced interface.
        interface: SemanticInterfaceKey,
        /// Unknown operation.
        operation: OperationName,
    },
    /// A predicate was used as an effect or an effect as a requirement.
    WrongOperationStage {
        /// Action containing the call.
        action: LocalDefinitionName,
        /// Referenced interface.
        interface: SemanticInterfaceKey,
        /// Referenced operation.
        operation: OperationName,
        /// Operation kind legal at this stage.
        expected: OperationKind,
        /// Kind declared by the interface.
        actual: OperationKind,
    },
    /// A semantic operation received the wrong number of positional arguments.
    OperationArityMismatch {
        /// Action containing the call.
        action: LocalDefinitionName,
        /// Referenced interface.
        interface: SemanticInterfaceKey,
        /// Referenced operation.
        operation: OperationName,
        /// Descriptor parameter count.
        expected: usize,
        /// Supplied argument count.
        actual: usize,
    },
    /// A call or event mapping names an undeclared action binding.
    UnknownBinding {
        /// Action containing the reference.
        action: LocalDefinitionName,
        /// Unknown binding.
        binding: BindingName,
    },
    /// A call argument binding has the wrong value kind.
    BindingKindMismatch {
        /// Action containing the call.
        action: LocalDefinitionName,
        /// Binding supplied as the argument.
        binding: BindingName,
        /// Positional operation argument.
        argument_index: usize,
        /// Kind required by the operation descriptor.
        expected: ValueKind,
        /// Kind declared by the action.
        actual: ValueKind,
    },
    /// An exact interface-table entry is not used by any operation call.
    UnusedInterface {
        /// Unused interface.
        interface: SemanticInterfaceKey,
    },
    /// A success event contains no field mapping.
    EmptyEventMapping {
        /// Action declaring the event.
        action: LocalDefinitionName,
        /// Referenced event.
        event: DefinitionKey,
    },
    /// One event field was mapped more than once.
    DuplicateEventMapping {
        /// Action declaring the event.
        action: LocalDefinitionName,
        /// Referenced event.
        event: DefinitionKey,
        /// Repeated event field.
        field: EventFieldName,
    },
    /// A cross-pack event names neither this pack nor a direct dependency.
    UndeclaredEventPack {
        /// Action declaring the event.
        action: LocalDefinitionName,
        /// Rejected event reference.
        event: DefinitionKey,
    },
    /// A same-pack event reference does not name an event definition.
    MissingLocalEvent {
        /// Action declaring the event.
        action: LocalDefinitionName,
        /// Missing event.
        event: DefinitionKey,
    },
    /// A same-pack event mapping has the wrong number of fields.
    EventMappingArityMismatch {
        /// Action declaring the event.
        action: LocalDefinitionName,
        /// Referenced event.
        event: DefinitionKey,
        /// Event field count.
        expected: usize,
        /// Supplied mapping count.
        actual: usize,
    },
    /// A same-pack event mapping names a different field set.
    EventFieldMismatch {
        /// Action declaring the event.
        action: LocalDefinitionName,
        /// Referenced event.
        event: DefinitionKey,
        /// Event field required at this sorted position.
        expected: EventFieldName,
        /// Supplied field.
        actual: EventFieldName,
    },
    /// An event field is populated from a binding of the wrong value kind.
    EventFieldKindMismatch {
        /// Action declaring the event.
        action: LocalDefinitionName,
        /// Referenced event.
        event: DefinitionKey,
        /// Event field being populated.
        field: EventFieldName,
        /// Source action binding.
        binding: BindingName,
        /// Kind declared by the event.
        expected: ValueKind,
        /// Kind declared by the action.
        actual: ValueKind,
    },
    /// A validated semantic value could not be written canonically.
    Canonical(CanonicalError),
}

impl From<CanonicalError> for ArtifactError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormatVersion { actual } => {
                write!(formatter, "unsupported artifact format version {actual}")
            }
            Self::ArtifactTooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "artifact is {actual} bytes; maximum is {maximum}"
                )
            }
            Self::LengthMismatch { declared, actual } => write!(
                formatter,
                "artifact descriptor declares {declared} bytes, but contains {actual}"
            ),
            Self::DigestMismatch { declared, actual } => write!(
                formatter,
                "artifact descriptor digest {declared} does not match {actual}"
            ),
            Self::Codec(error) => write!(formatter, "invalid ArtifactBlobV1: {error}"),
            Self::CollectionLimit {
                collection,
                actual,
                maximum,
            } => write!(
                formatter,
                "{collection} contains {actual} elements; maximum is {maximum}"
            ),
            Self::DuplicateDependency { pack } => {
                write!(formatter, "artifact repeats direct dependency '{pack}'")
            }
            Self::DuplicateInterface { interface } => {
                write!(formatter, "artifact repeats interface '{interface}'")
            }
            Self::InterfaceCatalog(error) => {
                write!(formatter, "artifact interface cannot be resolved: {error}")
            }
            Self::DuplicateDefinition { definition } => {
                write!(
                    formatter,
                    "artifact repeats local definition '{definition}'"
                )
            }
            Self::EmptyEvent { event } => {
                write!(formatter, "event '{event}' must declare at least one field")
            }
            Self::DuplicateEventField { event, field } => {
                write!(formatter, "event '{event}' repeats field '{field}'")
            }
            Self::DuplicateBinding { action, binding } => {
                write!(formatter, "action '{action}' repeats binding '{binding}'")
            }
            Self::EmptyEffects { action } => {
                write!(
                    formatter,
                    "action '{action}' must declare at least one effect"
                )
            }
            Self::EmptySuccessEvents { action } => write!(
                formatter,
                "action '{action}' must declare at least one success event"
            ),
            Self::MissingInterfaceReference { action, interface } => write!(
                formatter,
                "action '{action}' calls interface '{interface}' absent from the artifact table"
            ),
            Self::UnknownOperation {
                action,
                interface,
                operation,
            } => write!(
                formatter,
                "action '{action}' calls unknown operation '{interface}.{operation}'"
            ),
            Self::WrongOperationStage {
                action,
                interface,
                operation,
                expected,
                actual,
            } => write!(
                formatter,
                "action '{action}' uses '{interface}.{operation}' as {expected:?}, but it is {actual:?}"
            ),
            Self::OperationArityMismatch {
                action,
                interface,
                operation,
                expected,
                actual,
            } => write!(
                formatter,
                "action '{action}' calls '{interface}.{operation}' with {actual} arguments; expected {expected}"
            ),
            Self::UnknownBinding { action, binding } => {
                write!(
                    formatter,
                    "action '{action}' references unknown binding '{binding}'"
                )
            }
            Self::BindingKindMismatch {
                action,
                binding,
                argument_index,
                expected,
                actual,
            } => write!(
                formatter,
                "action '{action}' argument {argument_index} uses binding '{binding}' of kind {actual:?}; expected {expected:?}"
            ),
            Self::UnusedInterface { interface } => {
                write!(formatter, "artifact interface '{interface}' is never used")
            }
            Self::EmptyEventMapping { action, event } => write!(
                formatter,
                "action '{action}' emits event '{event}' without field mappings"
            ),
            Self::DuplicateEventMapping {
                action,
                event,
                field,
            } => write!(
                formatter,
                "action '{action}' maps field '{field}' of event '{event}' more than once"
            ),
            Self::UndeclaredEventPack { action, event } => write!(
                formatter,
                "action '{action}' emits event '{event}' from an undeclared pack"
            ),
            Self::MissingLocalEvent { action, event } => {
                write!(
                    formatter,
                    "action '{action}' emits missing local event '{event}'"
                )
            }
            Self::EventMappingArityMismatch {
                action,
                event,
                expected,
                actual,
            } => write!(
                formatter,
                "action '{action}' maps {actual} fields for event '{event}'; expected {expected}"
            ),
            Self::EventFieldMismatch {
                action,
                event,
                expected,
                actual,
            } => write!(
                formatter,
                "action '{action}' maps event '{event}' field '{actual}'; expected '{expected}'"
            ),
            Self::EventFieldKindMismatch {
                action,
                event,
                field,
                binding,
                expected,
                actual,
            } => write!(
                formatter,
                "action '{action}' maps event '{event}' field '{field}' from binding '{binding}' of kind {actual:?}; expected {expected:?}"
            ),
            Self::Canonical(error) => {
                write!(
                    formatter,
                    "artifact identity cannot be represented: {error}"
                )
            }
        }
    }
}

impl std::error::Error for ArtifactError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Codec(error) => Some(error),
            Self::InterfaceCatalog(error) => Some(error),
            Self::Canonical(error) => Some(error),
            _ => None,
        }
    }
}
