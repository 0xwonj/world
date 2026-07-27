//! Immutable pack definitions, artifacts, and exact definition sets.
//!
//! This crate owns the checked vocabulary between authoring and runtime. Input
//! data becomes authoritative only after artifact validation, exact package
//! finalization, and definition linking.

mod artifact;
mod definition;
mod interface;
mod key;
mod link;
mod package;

pub use artifact::{
    ARTIFACT_FORMAT_VERSION, ARTIFACT_MEDIA_TYPE, ActionDefinition, ArtifactCodecError,
    ArtifactData, ArtifactDescriptor, ArtifactDigest, ArtifactEnvelope, ArtifactError,
    ArtifactMediaType, ArtifactValidator, EventDefinition, MAX_ACTION_BINDINGS, MAX_ARTIFACT_BYTES,
    MAX_DEFINITIONS_PER_ARTIFACT, MAX_DIRECT_DEPENDENCIES, MAX_EFFECTS_PER_ACTION,
    MAX_EVENT_FIELDS, MAX_OPERATION_ARGUMENTS, MAX_REQUIRED_INTERFACES,
    MAX_REQUIREMENTS_PER_ACTION, MAX_SUCCESS_EVENTS_PER_ACTION, PackDependency, PackExportDigest,
    PackManifestData, RuntimeSemanticFingerprint, VerifiedPackArtifact,
};
pub use definition::{
    ActionBindingData, ActionData, EffectCallData, EventData, EventEmissionData,
    EventFieldBindingData, EventFieldData, OperationCallData, RuntimeRequirementData,
};
pub use interface::{
    CatalogError, InterfaceError, OperationKind, OperationName, OperationParameter, ParameterName,
    SemanticInterfaceCatalog, SemanticInterfaceDescriptor, SemanticInterfaceDigest,
    SemanticInterfaceKey, SemanticInterfaceReference, SemanticOperationDescriptor, ValueKind,
};
pub use key::{
    BindingName, DefinitionKey, EngineProtocolVersion, EventFieldName, InterfaceVersion, KeyError,
    LocalDefinitionName, PackCoordinate, PackKey, PackVersion,
};
pub use link::{
    DefinitionLinker, LinkError, RUNTIME_DEFINITION_SET_SCHEMA_VERSION, RuntimeDefinitionSet,
    RuntimeDefinitionSetDigest,
};
pub use package::{
    ExactPackSet, ExactPackageSelection, MAX_PACKAGES_PER_SET, PACK_LOCK_SCHEMA_VERSION,
    PACK_RESOLVER_VERSION, PackLock, PackLockDependency, PackLockDigest, PackLockEntry,
    PackSetError, SelectedPackage, SourceSnapshotId,
};
