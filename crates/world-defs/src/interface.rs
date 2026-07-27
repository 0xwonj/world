use core::fmt;

use world_core::{CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest};

use crate::key::InterfaceVersion;
pub use crate::key::{OperationName, ParameterName, SemanticInterfaceKey};

/// Canonical schema version of a semantic-interface descriptor.
pub const SEMANTIC_INTERFACE_SCHEMA_VERSION: u16 = 1;

/// Maximum number of operations declared by one semantic interface.
pub const MAX_INTERFACE_OPERATIONS: usize = 256;

const SEMANTIC_INTERFACE_DOMAIN: &str = "semantic-interface-v1";

/// Value shapes supported by the initial semantic-interface boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueKind {
    /// An actor identity.
    Actor,
    /// A general entity identity.
    Entity,
}

impl ValueKind {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Actor => 0,
            Self::Entity => 1,
        }
    }
}

/// Authority class of one semantic operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperationKind {
    /// A read-only Boolean requirement evaluated against authoritative state.
    Predicate,
    /// An operation that may prepare an authoritative domain effect.
    Effect,
}

impl OperationKind {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Predicate => 0,
            Self::Effect => 1,
        }
    }
}

/// One ordered, named operation parameter.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationParameter {
    name: ParameterName,
    value_kind: ValueKind,
}

impl OperationParameter {
    /// Creates a parameter in an operation signature.
    #[must_use]
    pub const fn new(name: ParameterName, value_kind: ValueKind) -> Self {
        Self { name, value_kind }
    }

    /// Returns the durable parameter name.
    #[must_use]
    pub const fn name(&self) -> &ParameterName {
        &self.name
    }

    /// Returns the accepted value shape.
    #[must_use]
    pub const fn value_kind(&self) -> ValueKind {
        self.value_kind
    }
}

/// Checked signature of one semantic operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticOperationDescriptor {
    name: OperationName,
    kind: OperationKind,
    parameters: Vec<OperationParameter>,
}

impl SemanticOperationDescriptor {
    /// Creates an operation while preserving parameter order.
    ///
    /// Parameter order is part of the signature. Names must be unique within
    /// that ordered signature.
    pub fn new(
        name: OperationName,
        kind: OperationKind,
        parameters: Vec<OperationParameter>,
    ) -> Result<Self, InterfaceError> {
        for (index, parameter) in parameters.iter().enumerate() {
            if parameters[..index]
                .iter()
                .any(|previous| previous.name == parameter.name)
            {
                return Err(InterfaceError::DuplicateParameter {
                    operation: name,
                    parameter: parameter.name.clone(),
                });
            }
        }

        Ok(Self {
            name,
            kind,
            parameters,
        })
    }

    /// Returns the operation name.
    #[must_use]
    pub const fn name(&self) -> &OperationName {
        &self.name
    }

    /// Returns the operation's authority class.
    #[must_use]
    pub const fn kind(&self) -> OperationKind {
        self.kind
    }

    /// Returns the ordered operation parameters.
    #[must_use]
    pub fn parameters(&self) -> &[OperationParameter] {
        &self.parameters
    }
}

/// Canonical identity of one complete semantic-interface descriptor.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticInterfaceDigest(ContentDigest);

impl SemanticInterfaceDigest {
    /// Creates a decoded digest value.
    ///
    /// This validates only its fixed-width representation. Catalog resolution
    /// checks it against a locally constructed descriptor.
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
}

impl fmt::Display for SemanticInterfaceDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

impl fmt::Debug for SemanticInterfaceDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "SemanticInterfaceDigest({self})")
    }
}

/// Exact descriptor identity carried by a compiled artifact.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticInterfaceReference {
    key: SemanticInterfaceKey,
    version: InterfaceVersion,
    digest: SemanticInterfaceDigest,
}

impl SemanticInterfaceReference {
    /// Creates an exact reference decoded from an owning representation.
    #[must_use]
    pub const fn new(
        key: SemanticInterfaceKey,
        version: InterfaceVersion,
        digest: SemanticInterfaceDigest,
    ) -> Self {
        Self {
            key,
            version,
            digest,
        }
    }

    /// Returns the interface key.
    #[must_use]
    pub const fn key(&self) -> &SemanticInterfaceKey {
        &self.key
    }

    /// Returns the exact interface version.
    #[must_use]
    pub const fn version(&self) -> InterfaceVersion {
        self.version
    }

    /// Returns the expected descriptor digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticInterfaceDigest {
        self.digest
    }
}

/// Immutable declarative semantic-interface contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticInterfaceDescriptor {
    key: SemanticInterfaceKey,
    version: InterfaceVersion,
    operations: Vec<SemanticOperationDescriptor>,
    digest: SemanticInterfaceDigest,
}

impl SemanticInterfaceDescriptor {
    /// Creates a descriptor and computes its canonical identity.
    ///
    /// Operations are normalized by name. Operation names must be unique.
    pub fn new(
        key: SemanticInterfaceKey,
        version: InterfaceVersion,
        mut operations: Vec<SemanticOperationDescriptor>,
    ) -> Result<Self, InterfaceError> {
        if operations.len() > MAX_INTERFACE_OPERATIONS {
            return Err(InterfaceError::TooManyOperations {
                actual: operations.len(),
                maximum: MAX_INTERFACE_OPERATIONS,
            });
        }

        operations.sort_by(|left, right| left.name.cmp(&right.name));

        for adjacent in operations.windows(2) {
            if adjacent[0].name == adjacent[1].name {
                return Err(InterfaceError::DuplicateOperation {
                    operation: adjacent[0].name.clone(),
                });
            }
        }

        let digest = compute_descriptor_digest(&key, version, &operations)?;

        Ok(Self {
            key,
            version,
            operations,
            digest,
        })
    }

    /// Returns the durable interface key.
    #[must_use]
    pub const fn key(&self) -> &SemanticInterfaceKey {
        &self.key
    }

    /// Returns the exact interface version.
    #[must_use]
    pub const fn version(&self) -> InterfaceVersion {
        self.version
    }

    /// Returns operations in canonical name order.
    #[must_use]
    pub fn operations(&self) -> &[SemanticOperationDescriptor] {
        &self.operations
    }

    /// Returns the descriptor's canonical identity.
    #[must_use]
    pub const fn digest(&self) -> SemanticInterfaceDigest {
        self.digest
    }

    /// Creates the exact artifact-facing reference to this descriptor.
    #[must_use]
    pub fn reference(&self) -> SemanticInterfaceReference {
        SemanticInterfaceReference::new(self.key.clone(), self.version, self.digest)
    }

    /// Finds one operation by its exact name.
    #[must_use]
    pub fn operation(&self, name: &OperationName) -> Option<&SemanticOperationDescriptor> {
        self.operations
            .binary_search_by(|operation| operation.name.cmp(name))
            .ok()
            .map(|index| &self.operations[index])
    }
}

/// Checked immutable collection of available semantic-interface descriptors.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SemanticInterfaceCatalog {
    descriptors: Vec<SemanticInterfaceDescriptor>,
}

impl SemanticInterfaceCatalog {
    /// Creates a catalog in canonical key order.
    ///
    /// A catalog selects exactly one active descriptor for each durable key.
    /// This keeps source operation calls unambiguous while artifacts still
    /// bind the selected descriptor's exact version and digest.
    pub fn new(mut descriptors: Vec<SemanticInterfaceDescriptor>) -> Result<Self, CatalogError> {
        descriptors.sort_by(compare_descriptors);

        for adjacent in descriptors.windows(2) {
            let left = &adjacent[0];
            let right = &adjacent[1];
            if left.key == right.key {
                if left.version == right.version && left.digest == right.digest {
                    return Err(CatalogError::DuplicateEntry {
                        key: left.key.clone(),
                        version: left.version,
                    });
                }
                return Err(CatalogError::ConflictingEntry {
                    key: left.key.clone(),
                    first: Box::new(left.reference()),
                    second: Box::new(right.reference()),
                });
            }
        }

        Ok(Self { descriptors })
    }

    /// Returns descriptors in canonical key order.
    #[must_use]
    pub fn descriptors(&self) -> &[SemanticInterfaceDescriptor] {
        &self.descriptors
    }

    /// Returns the number of descriptors in the catalog.
    #[must_use]
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether the catalog contains no descriptors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    /// Finds an exact key/version pair.
    #[must_use]
    pub fn get(
        &self,
        key: &SemanticInterfaceKey,
        version: InterfaceVersion,
    ) -> Option<&SemanticInterfaceDescriptor> {
        self.descriptors
            .binary_search_by(|descriptor| descriptor.key.cmp(key))
            .ok()
            .map(|index| &self.descriptors[index])
            .filter(|descriptor| descriptor.version == version)
    }

    /// Finds the catalog's single active descriptor for a durable key.
    #[must_use]
    pub fn get_by_key(&self, key: &SemanticInterfaceKey) -> Option<&SemanticInterfaceDescriptor> {
        self.descriptors
            .binary_search_by(|descriptor| descriptor.key.cmp(key))
            .ok()
            .map(|index| &self.descriptors[index])
    }

    /// Resolves an exact reference and verifies its descriptor identity.
    pub fn resolve(
        &self,
        reference: &SemanticInterfaceReference,
    ) -> Result<&SemanticInterfaceDescriptor, CatalogError> {
        let Some(descriptor) = self.get(reference.key(), reference.version()) else {
            return Err(CatalogError::MissingInterface {
                key: reference.key().clone(),
                version: reference.version(),
            });
        };

        if descriptor.digest != reference.digest {
            return Err(CatalogError::DigestMismatch {
                key: reference.key().clone(),
                version: reference.version(),
                expected: reference.digest,
                actual: descriptor.digest,
            });
        }

        Ok(descriptor)
    }
}

fn compare_descriptors(
    left: &SemanticInterfaceDescriptor,
    right: &SemanticInterfaceDescriptor,
) -> core::cmp::Ordering {
    left.key
        .cmp(&right.key)
        .then_with(|| left.version.cmp(&right.version))
}

fn compute_descriptor_digest(
    key: &SemanticInterfaceKey,
    version: InterfaceVersion,
    operations: &[SemanticOperationDescriptor],
) -> Result<SemanticInterfaceDigest, InterfaceError> {
    let domain =
        CanonicalDomain::new(SEMANTIC_INTERFACE_DOMAIN).map_err(InterfaceError::Canonical)?;
    let mut writer = CanonicalWriter::new(domain);
    writer.write_u16(SEMANTIC_INTERFACE_SCHEMA_VERSION);
    writer
        .write_str(key.as_str())
        .map_err(InterfaceError::Canonical)?;
    writer.write_u16(version.get());
    writer
        .write_sequence(operations, |writer, operation| {
            writer.write_str(operation.name.as_str())?;
            writer.write_discriminant(operation.kind.canonical_tag());
            writer.write_sequence(&operation.parameters, |writer, parameter| {
                writer.write_str(parameter.name.as_str())?;
                writer.write_discriminant(parameter.value_kind.canonical_tag());
                Ok(())
            })
        })
        .map_err(InterfaceError::Canonical)?;

    Ok(SemanticInterfaceDigest(ContentDigest::of_canonical(
        &writer.finish(),
    )))
}

/// Failure to construct a semantic-interface descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterfaceError {
    /// An interface exceeded the format's operation limit.
    TooManyOperations {
        /// Supplied operation count.
        actual: usize,
        /// Maximum accepted operation count.
        maximum: usize,
    },
    /// An interface declared one operation name more than once.
    DuplicateOperation {
        /// Repeated operation name.
        operation: OperationName,
    },
    /// An operation declared one parameter name more than once.
    DuplicateParameter {
        /// Operation whose signature was rejected.
        operation: OperationName,
        /// Repeated parameter name.
        parameter: ParameterName,
    },
    /// A checked descriptor could not be represented canonically.
    Canonical(CanonicalError),
}

impl fmt::Display for InterfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyOperations { actual, maximum } => write!(
                formatter,
                "semantic interface declares {actual} operations; maximum is {maximum}"
            ),
            Self::DuplicateOperation { operation } => write!(
                formatter,
                "semantic interface repeats operation '{}'",
                operation.as_str()
            ),
            Self::DuplicateParameter {
                operation,
                parameter,
            } => write!(
                formatter,
                "operation '{}' repeats parameter '{}'",
                operation.as_str(),
                parameter.as_str()
            ),
            Self::Canonical(error) => write!(
                formatter,
                "semantic interface cannot be represented canonically: {error}"
            ),
        }
    }
}

impl std::error::Error for InterfaceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            _ => None,
        }
    }
}

/// Failure to construct or resolve a semantic-interface catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogError {
    /// The same exact key/version pair appeared more than once.
    DuplicateEntry {
        /// Repeated interface key.
        key: SemanticInterfaceKey,
        /// Repeated interface version.
        version: InterfaceVersion,
    },
    /// One key/version pair named two different descriptor identities.
    ConflictingEntry {
        /// Conflicting interface key.
        key: SemanticInterfaceKey,
        /// First exact descriptor reference.
        first: Box<SemanticInterfaceReference>,
        /// Second exact descriptor reference.
        second: Box<SemanticInterfaceReference>,
    },
    /// No descriptor exists for an exact key/version pair.
    MissingInterface {
        /// Missing interface key.
        key: SemanticInterfaceKey,
        /// Missing interface version.
        version: InterfaceVersion,
    },
    /// A reference's digest does not match the local descriptor.
    DigestMismatch {
        /// Referenced interface key.
        key: SemanticInterfaceKey,
        /// Referenced interface version.
        version: InterfaceVersion,
        /// Digest carried by the reference.
        expected: SemanticInterfaceDigest,
        /// Digest computed by the local descriptor.
        actual: SemanticInterfaceDigest,
    },
}

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateEntry { key, version } => write!(
                formatter,
                "semantic-interface catalog repeats '{}@{}'",
                key.as_str(),
                version.get()
            ),
            Self::ConflictingEntry { key, first, second } => write!(
                formatter,
                "semantic-interface catalog selects conflicting descriptors for '{}': @{} {} and @{} {}",
                key.as_str(),
                first.version().get(),
                first.digest(),
                second.version().get(),
                second.digest()
            ),
            Self::MissingInterface { key, version } => write!(
                formatter,
                "semantic-interface catalog does not contain '{}@{}'",
                key.as_str(),
                version.get()
            ),
            Self::DigestMismatch {
                key,
                version,
                expected,
                actual,
            } => write!(
                formatter,
                "semantic-interface reference '{}@{}' expects {expected}, but the catalog contains {actual}",
                key.as_str(),
                version.get()
            ),
        }
    }
}

impl std::error::Error for CatalogError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn interface_key(value: &str) -> SemanticInterfaceKey {
        match SemanticInterfaceKey::parse(value) {
            Ok(value) => value,
            Err(error) => panic!("invalid interface-key fixture: {error}"),
        }
    }

    fn interface_version(value: u16) -> InterfaceVersion {
        match InterfaceVersion::new(value) {
            Ok(value) => value,
            Err(error) => panic!("invalid interface-version fixture: {error}"),
        }
    }

    fn operation_name(value: &str) -> OperationName {
        match OperationName::parse(value) {
            Ok(value) => value,
            Err(error) => panic!("invalid operation-name fixture: {error}"),
        }
    }

    fn parameter_name(value: &str) -> ParameterName {
        match ParameterName::parse(value) {
            Ok(value) => value,
            Err(error) => panic!("invalid parameter-name fixture: {error}"),
        }
    }

    fn transfer_parameters() -> Vec<OperationParameter> {
        vec![
            OperationParameter::new(parameter_name("actor"), ValueKind::Actor),
            OperationParameter::new(parameter_name("item"), ValueKind::Entity),
            OperationParameter::new(parameter_name("source"), ValueKind::Entity),
            OperationParameter::new(parameter_name("destination"), ValueKind::Entity),
        ]
    }

    fn operation(
        name: &str,
        kind: OperationKind,
        parameters: Vec<OperationParameter>,
    ) -> SemanticOperationDescriptor {
        match SemanticOperationDescriptor::new(operation_name(name), kind, parameters) {
            Ok(value) => value,
            Err(error) => panic!("invalid operation fixture: {error}"),
        }
    }

    fn descriptor(
        key: &str,
        operations: Vec<SemanticOperationDescriptor>,
    ) -> SemanticInterfaceDescriptor {
        match SemanticInterfaceDescriptor::new(interface_key(key), interface_version(1), operations)
        {
            Ok(value) => value,
            Err(error) => panic!("invalid interface fixture: {error}"),
        }
    }

    #[test]
    fn descriptor_normalizes_operations_and_has_stable_identity() {
        let descriptor = descriptor(
            "world.standard.transfer",
            vec![
                operation(
                    "transfer-item",
                    OperationKind::Effect,
                    transfer_parameters(),
                ),
                operation(
                    "can-transfer-item",
                    OperationKind::Predicate,
                    transfer_parameters(),
                ),
            ],
        );

        let names: Vec<_> = descriptor
            .operations()
            .iter()
            .map(|operation| operation.name().as_str())
            .collect();
        assert_eq!(names, ["can-transfer-item", "transfer-item"]);
        assert_eq!(
            descriptor.digest().to_string(),
            "70f1b02ad7847bada652d0631a1385ff997dcefe9674d0ac7566a190cc3f2067"
        );
    }

    #[test]
    fn operation_rejects_duplicate_parameter_names() {
        let duplicate = parameter_name("item");
        let error = SemanticOperationDescriptor::new(
            operation_name("transfer-item"),
            OperationKind::Effect,
            vec![
                OperationParameter::new(duplicate.clone(), ValueKind::Entity),
                OperationParameter::new(duplicate.clone(), ValueKind::Entity),
            ],
        );

        assert_eq!(
            error,
            Err(InterfaceError::DuplicateParameter {
                operation: operation_name("transfer-item"),
                parameter: duplicate,
            })
        );
    }

    #[test]
    fn interface_rejects_duplicate_operation_names() {
        let error = SemanticInterfaceDescriptor::new(
            interface_key("world.standard.transfer"),
            interface_version(1),
            vec![
                operation("transfer-item", OperationKind::Effect, Vec::new()),
                operation("transfer-item", OperationKind::Predicate, Vec::new()),
            ],
        );

        assert_eq!(
            error,
            Err(InterfaceError::DuplicateOperation {
                operation: operation_name("transfer-item"),
            })
        );
    }

    #[test]
    fn catalog_orders_and_resolves_exact_references() {
        let later = descriptor(
            "world.standard.transfer",
            vec![operation(
                "transfer-item",
                OperationKind::Effect,
                transfer_parameters(),
            )],
        );
        let earlier = descriptor(
            "world.standard.condition",
            vec![operation(
                "can-transfer-item",
                OperationKind::Predicate,
                transfer_parameters(),
            )],
        );
        let exact_reference = later.reference();
        let catalog = match SemanticInterfaceCatalog::new(vec![later, earlier]) {
            Ok(value) => value,
            Err(error) => panic!("invalid catalog fixture: {error}"),
        };

        assert_eq!(
            catalog.descriptors()[0].key().as_str(),
            "world.standard.condition"
        );
        assert_eq!(
            catalog.resolve(&exact_reference).map(|value| value.key()),
            Ok(exact_reference.key())
        );

        let mismatched = SemanticInterfaceReference::new(
            exact_reference.key().clone(),
            exact_reference.version(),
            SemanticInterfaceDigest::from_bytes([0; 32]),
        );
        assert!(matches!(
            catalog.resolve(&mismatched),
            Err(CatalogError::DigestMismatch { .. })
        ));
    }

    #[test]
    fn catalog_distinguishes_duplicate_and_conflicting_entries() {
        let first = descriptor(
            "world.standard.transfer",
            vec![operation(
                "transfer-item",
                OperationKind::Effect,
                transfer_parameters(),
            )],
        );

        assert_eq!(
            SemanticInterfaceCatalog::new(vec![first.clone(), first.clone()]),
            Err(CatalogError::DuplicateEntry {
                key: first.key().clone(),
                version: first.version(),
            })
        );

        let conflicting = descriptor(
            "world.standard.transfer",
            vec![operation(
                "can-transfer-item",
                OperationKind::Predicate,
                transfer_parameters(),
            )],
        );
        assert!(matches!(
            SemanticInterfaceCatalog::new(vec![first, conflicting]),
            Err(CatalogError::ConflictingEntry { .. })
        ));
    }
}
