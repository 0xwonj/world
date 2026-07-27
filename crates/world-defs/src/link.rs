use core::fmt;

use world_core::{CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest};

use crate::artifact::{
    ActionDefinition, EventDefinition, RuntimeSemanticFingerprint, VerifiedPackArtifact,
};
use crate::interface::{SemanticInterfaceReference, ValueKind};
use crate::key::{
    BindingName, DefinitionKey, EngineProtocolVersion, EventFieldName, LocalDefinitionName,
    PackCoordinate, PackKey,
};
use crate::package::{ExactPackSet, PackLock};

/// Canonical schema of the immutable linked definition-set identity.
pub const RUNTIME_DEFINITION_SET_SCHEMA_VERSION: u16 = 1;

const RUNTIME_DEFINITION_SET_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("runtime-definition-set-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("runtime definition-set identity domain must be valid"),
    };

/// Canonical identity of one exact linked runtime definition set.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeDefinitionSetDigest(ContentDigest);

impl RuntimeDefinitionSetDigest {
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
}

impl fmt::Display for RuntimeDefinitionSetDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

impl fmt::Debug for RuntimeDefinitionSetDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RuntimeDefinitionSetDigest({self})")
    }
}

/// Immutable, process-independent definitions linked from one exact pack set.
///
/// This value retains package provenance in its private lock, while its
/// canonical identity excludes source snapshots and covers only the exact
/// runtime artifacts and required semantic-interface closure.
///
/// ```compile_fail
/// let _ = world_defs::RuntimeDefinitionSet {};
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeDefinitionSet {
    lock: PackLock,
    artifacts: Vec<VerifiedPackArtifact>,
    engine_protocol: EngineProtocolVersion,
    digest: RuntimeDefinitionSetDigest,
}

impl RuntimeDefinitionSet {
    /// Returns the exact package lock retained as resolution provenance.
    #[must_use]
    pub const fn lock(&self) -> &PackLock {
        &self.lock
    }

    /// Returns the selected root package coordinate.
    #[must_use]
    pub const fn root(&self) -> &PackCoordinate {
        self.lock.root()
    }

    /// Returns the common engine protocol required by every linked artifact.
    #[must_use]
    pub const fn engine_protocol(&self) -> EngineProtocolVersion {
        self.engine_protocol
    }

    /// Returns the exact required semantic-interface closure.
    #[must_use]
    pub fn required_interfaces(&self) -> &[SemanticInterfaceReference] {
        self.lock.required_interfaces()
    }

    /// Returns the canonical runtime definition-set identity.
    #[must_use]
    pub const fn digest(&self) -> RuntimeDefinitionSetDigest {
        self.digest
    }

    /// Returns linked artifacts in canonical `PackKey` order.
    #[must_use]
    pub fn artifacts(&self) -> &[VerifiedPackArtifact] {
        &self.artifacts
    }

    /// Finds the exact selected artifact for a durable pack key.
    #[must_use]
    pub fn artifact(&self, key: &PackKey) -> Option<&VerifiedPackArtifact> {
        find_artifact(&self.artifacts, key)
    }

    /// Finds one linked action by durable definition key.
    #[must_use]
    pub fn action(&self, key: &DefinitionKey) -> Option<&ActionDefinition> {
        let artifact = self.artifact(key.pack_key())?;
        find_action(artifact.actions(), key.local_name())
    }

    /// Finds one linked physical event by durable definition key.
    #[must_use]
    pub fn event(&self, key: &DefinitionKey) -> Option<&EventDefinition> {
        let artifact = self.artifact(key.pack_key())?;
        find_event(artifact.events(), key.local_name())
    }
}

/// Total linker from an exact pack-set proof to immutable runtime definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefinitionLinker {
    _private: (),
}

impl DefinitionLinker {
    /// Links an exact pack set and validates cross-pack physical-event
    /// contracts.
    ///
    /// Package selection, graph closure, artifact correspondence, engine
    /// protocol agreement, and interface union have already been proved by
    /// [`ExactPackSet`]. Linking adds only cross-artifact definition
    /// invariants and the runtime definition-set identity.
    pub fn link(set: ExactPackSet) -> Result<RuntimeDefinitionSet, LinkError> {
        validate_cross_pack_events(set.artifacts())?;
        let engine_protocol = set.engine_protocol();
        let (lock, artifacts) = set.into_parts();
        let digest = compute_definition_set_digest(&lock, &artifacts, engine_protocol)?;

        Ok(RuntimeDefinitionSet {
            lock,
            artifacts,
            engine_protocol,
            digest,
        })
    }
}

fn validate_cross_pack_events(artifacts: &[VerifiedPackArtifact]) -> Result<(), LinkError> {
    for artifact in artifacts {
        let source_pack = artifact.coordinate().pack_key();
        for action in artifact.actions() {
            for emission in action.success_events() {
                if emission.event().pack_key() == source_pack {
                    continue;
                }

                let action_key = DefinitionKey::new(source_pack.clone(), action.name().clone());
                let target_artifact = match find_artifact(artifacts, emission.event().pack_key()) {
                    Some(artifact) => artifact,
                    None => {
                        unreachable!("exact pack set must contain every declared event dependency")
                    }
                };
                let target = find_event(target_artifact.events(), emission.event().local_name())
                    .ok_or_else(|| LinkError::MissingEventDefinition {
                        action: Box::new(action_key.clone()),
                        event: Box::new(emission.event().clone()),
                    })?;

                validate_cross_pack_mapping(action, &action_key, emission, target)?;
            }
        }
    }
    Ok(())
}

fn validate_cross_pack_mapping(
    action: &ActionDefinition,
    action_key: &DefinitionKey,
    emission: &crate::definition::EventEmissionData,
    target: &EventDefinition,
) -> Result<(), LinkError> {
    if emission.field_bindings().len() != target.fields().len() {
        return Err(LinkError::EventMappingArityMismatch {
            action: Box::new(action_key.clone()),
            event: Box::new(emission.event().clone()),
            expected: target.fields().len(),
            actual: emission.field_bindings().len(),
        });
    }

    for (mapping, field) in emission.field_bindings().iter().zip(target.fields()) {
        if mapping.field() != field.name() {
            return Err(LinkError::EventFieldMismatch {
                action: Box::new(action_key.clone()),
                event: Box::new(emission.event().clone()),
                expected: field.name().clone(),
                actual: mapping.field().clone(),
            });
        }

        let binding = match find_binding(action, mapping.binding()) {
            Some(binding) => binding,
            None => unreachable!("artifact-validated event mapping must reference a binding"),
        };
        if binding.value_kind() != field.value_kind() {
            return Err(LinkError::EventFieldKindMismatch {
                action: Box::new(action_key.clone()),
                event: Box::new(emission.event().clone()),
                field: field.name().clone(),
                binding: mapping.binding().clone(),
                expected: *field.value_kind(),
                actual: *binding.value_kind(),
            });
        }
    }

    Ok(())
}

fn find_binding<'action>(
    action: &'action ActionDefinition,
    name: &BindingName,
) -> Option<&'action crate::definition::ActionBindingData> {
    action
        .bindings()
        .binary_search_by(|binding| binding.name().cmp(name))
        .ok()
        .map(|index| &action.bindings()[index])
}

fn compute_definition_set_digest(
    lock: &PackLock,
    artifacts: &[VerifiedPackArtifact],
    engine_protocol: EngineProtocolVersion,
) -> Result<RuntimeDefinitionSetDigest, LinkError> {
    let mut writer = CanonicalWriter::new(RUNTIME_DEFINITION_SET_DOMAIN);
    writer.write_u16(RUNTIME_DEFINITION_SET_SCHEMA_VERSION);
    writer.write_u16(engine_protocol.get());
    write_coordinate(&mut writer, lock.root())?;
    writer.write_sequence(artifacts, |writer, artifact| {
        write_coordinate(writer, artifact.coordinate())?;
        writer.write_u16(artifact.envelope().descriptor().format_version());
        writer.write_bytes(artifact.artifact_digest().as_bytes())?;
        write_semantic_fingerprint(writer, artifact.semantic_fingerprint())
    })?;
    writer.write_sequence(lock.required_interfaces(), |writer, reference| {
        writer.write_str(reference.key().as_str())?;
        writer.write_u16(reference.version().get());
        writer.write_bytes(reference.digest().as_bytes())
    })?;
    Ok(RuntimeDefinitionSetDigest(ContentDigest::of_canonical(
        &writer.finish(),
    )))
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

fn write_semantic_fingerprint(
    writer: &mut CanonicalWriter,
    fingerprint: RuntimeSemanticFingerprint,
) -> Result<(), CanonicalError> {
    writer.write_bytes(fingerprint.as_bytes())
}

fn find_artifact<'artifacts>(
    artifacts: &'artifacts [VerifiedPackArtifact],
    key: &PackKey,
) -> Option<&'artifacts VerifiedPackArtifact> {
    artifacts
        .binary_search_by(|artifact| artifact.coordinate().pack_key().cmp(key))
        .ok()
        .map(|index| &artifacts[index])
}

fn find_action<'artifact>(
    actions: &'artifact [ActionDefinition],
    name: &LocalDefinitionName,
) -> Option<&'artifact ActionDefinition> {
    actions
        .binary_search_by(|action| action.name().cmp(name))
        .ok()
        .map(|index| &actions[index])
}

fn find_event<'artifact>(
    events: &'artifact [EventDefinition],
    name: &LocalDefinitionName,
) -> Option<&'artifact EventDefinition> {
    events
        .binary_search_by(|event| event.name().cmp(name))
        .ok()
        .map(|index| &events[index])
}

/// Failure to link exact artifacts into a runtime definition set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkError {
    /// A selected target pack does not export the referenced physical event.
    MissingEventDefinition {
        /// Action declaring the success event.
        action: Box<DefinitionKey>,
        /// Missing event reference.
        event: Box<DefinitionKey>,
    },
    /// A cross-pack event mapping has the wrong field count.
    EventMappingArityMismatch {
        /// Action declaring the success event.
        action: Box<DefinitionKey>,
        /// Referenced event.
        event: Box<DefinitionKey>,
        /// Event field count.
        expected: usize,
        /// Supplied mapping count.
        actual: usize,
    },
    /// A cross-pack mapping names a different field at one canonical position.
    EventFieldMismatch {
        /// Action declaring the success event.
        action: Box<DefinitionKey>,
        /// Referenced event.
        event: Box<DefinitionKey>,
        /// Required event field.
        expected: EventFieldName,
        /// Supplied event field.
        actual: EventFieldName,
    },
    /// An event field is populated from a binding of the wrong value kind.
    EventFieldKindMismatch {
        /// Action declaring the success event.
        action: Box<DefinitionKey>,
        /// Referenced event.
        event: Box<DefinitionKey>,
        /// Event field being populated.
        field: EventFieldName,
        /// Source action binding.
        binding: BindingName,
        /// Kind declared by the event.
        expected: ValueKind,
        /// Kind declared by the action.
        actual: ValueKind,
    },
    /// A checked definition set could not be represented canonically.
    Canonical(CanonicalError),
}

impl From<CanonicalError> for LinkError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl fmt::Display for LinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEventDefinition { action, event } => write!(
                formatter,
                "action '{action}' references missing physical event '{event}'"
            ),
            Self::EventMappingArityMismatch {
                action,
                event,
                expected,
                actual,
            } => write!(
                formatter,
                "action '{action}' maps {actual} fields for cross-pack event '{event}'; expected {expected}"
            ),
            Self::EventFieldMismatch {
                action,
                event,
                expected,
                actual,
            } => write!(
                formatter,
                "action '{action}' maps cross-pack event '{event}' field '{actual}'; expected '{expected}'"
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
                "action '{action}' maps cross-pack event '{event}' field '{field}' from binding '{binding}' of kind {actual:?}; expected {expected:?}"
            ),
            Self::Canonical(error) => {
                write!(
                    formatter,
                    "runtime definition-set identity cannot be represented: {error}"
                )
            }
        }
    }
}

impl std::error::Error for LinkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            _ => None,
        }
    }
}
