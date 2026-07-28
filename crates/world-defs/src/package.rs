use core::fmt;

use world_core::{CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest};

use crate::artifact::{ArtifactDigest, PackExportDigest, VerifiedPackArtifact};
use crate::interface::{SemanticInterfaceKey, SemanticInterfaceReference};
use crate::key::{EngineProtocolVersion, PackCoordinate, PackKey};

/// Maximum number of packages in one exact definition closure.
pub const MAX_PACKAGES_PER_SET: usize = 256;

/// Canonical schema version of [`PackLock`].
pub const PACK_LOCK_SCHEMA_VERSION: u16 = 1;

/// Exact resolver protocol used to produce [`PackLock`].
pub const PACK_RESOLVER_VERSION: u16 = 1;

const PACK_LOCK_DOMAIN: CanonicalDomain = match CanonicalDomain::new("pack-lock-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("pack lock identity domain must be valid"),
};

/// Exact identity of the source snapshot selected for one package.
///
/// Source provenance is recorded in a [`PackLock`]. It does not enter an
/// artifact digest or runtime semantic fingerprint.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceSnapshotId([u8; 32]);

impl SourceSnapshotId {
    /// Creates a source-snapshot identity from its exact protocol bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the exact identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the identity and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for SourceSnapshotId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for SourceSnapshotId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "SourceSnapshotId({self})")
    }
}

/// One package in an exact source selection.
///
/// This is a constructible description, not proof of graph closure. Exact-set
/// finalization checks its coordinate and edges against a verified artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedPackage {
    coordinate: PackCoordinate,
    source_snapshot: SourceSnapshotId,
    dependencies: Vec<PackCoordinate>,
}

impl SelectedPackage {
    /// Describes one selected source and its resolved direct dependencies.
    #[must_use]
    pub fn new(
        coordinate: PackCoordinate,
        source_snapshot: SourceSnapshotId,
        dependencies: Vec<PackCoordinate>,
    ) -> Self {
        Self {
            coordinate,
            source_snapshot,
            dependencies,
        }
    }

    /// Returns the selected package coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &PackCoordinate {
        &self.coordinate
    }

    /// Returns the exact source-snapshot identity.
    #[must_use]
    pub const fn source_snapshot(&self) -> SourceSnapshotId {
        self.source_snapshot
    }

    /// Returns resolved direct dependencies in their current input order.
    #[must_use]
    pub fn dependencies(&self) -> &[PackCoordinate] {
        &self.dependencies
    }
}

/// An exact root and package/source selection supplied to set finalization.
///
/// Construction records resolver output without claiming that the graph is
/// closed, acyclic, or consistent with any artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPackageSelection {
    root: PackCoordinate,
    packages: Vec<SelectedPackage>,
}

impl ExactPackageSelection {
    /// Creates a package-selection description.
    #[must_use]
    pub fn new(root: PackCoordinate, packages: Vec<SelectedPackage>) -> Self {
        Self { root, packages }
    }

    /// Returns the selected root coordinate.
    #[must_use]
    pub const fn root(&self) -> &PackCoordinate {
        &self.root
    }

    /// Returns packages in their current input order.
    #[must_use]
    pub fn packages(&self) -> &[SelectedPackage] {
        &self.packages
    }
}

/// Canonical identity of a complete exact package lock.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackLockDigest(ContentDigest);

impl PackLockDigest {
    /// Returns the exact digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Consumes the digest and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0.into_bytes()
    }
}

impl fmt::Display for PackLockDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

impl fmt::Debug for PackLockDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "PackLockDigest({self})")
    }
}

/// One resolved direct edge stored in a package lock.
///
/// Both the dependency's exact artifact bytes and its checked public export
/// surface are recorded. This makes a direct edge independent of lookup order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackLockDependency {
    coordinate: PackCoordinate,
    artifact_digest: ArtifactDigest,
    export_digest: PackExportDigest,
}

impl PackLockDependency {
    /// Returns the dependency coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &PackCoordinate {
        &self.coordinate
    }

    /// Returns the dependency's exact artifact-byte identity.
    #[must_use]
    pub const fn artifact_digest(&self) -> ArtifactDigest {
        self.artifact_digest
    }

    /// Returns the dependency's public export identity.
    #[must_use]
    pub const fn export_digest(&self) -> PackExportDigest {
        self.export_digest
    }
}

/// One canonically ordered package entry in a [`PackLock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackLockEntry {
    coordinate: PackCoordinate,
    source_snapshot: SourceSnapshotId,
    artifact_format_version: u16,
    artifact_byte_length: u64,
    artifact_digest: ArtifactDigest,
    export_digest: PackExportDigest,
    dependencies: Vec<PackLockDependency>,
}

impl PackLockEntry {
    /// Returns the exact package coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &PackCoordinate {
        &self.coordinate
    }

    /// Returns the selected source-snapshot identity.
    #[must_use]
    pub const fn source_snapshot(&self) -> SourceSnapshotId {
        self.source_snapshot
    }

    /// Returns the exact artifact storage-format version.
    #[must_use]
    pub const fn artifact_format_version(&self) -> u16 {
        self.artifact_format_version
    }

    /// Returns the exact artifact byte length.
    #[must_use]
    pub const fn artifact_byte_length(&self) -> u64 {
        self.artifact_byte_length
    }

    /// Returns the package's exact artifact-byte identity.
    #[must_use]
    pub const fn artifact_digest(&self) -> ArtifactDigest {
        self.artifact_digest
    }

    /// Returns the package's checked public export identity.
    #[must_use]
    pub const fn export_digest(&self) -> PackExportDigest {
        self.export_digest
    }

    /// Returns direct dependencies in canonical `PackKey` order.
    #[must_use]
    pub fn dependencies(&self) -> &[PackLockDependency] {
        &self.dependencies
    }
}

/// Immutable exact package, provenance, dependency, and interface lock.
///
/// Its canonical `pack-lock-v1` preimage contains, in order:
///
/// 1. lock-schema and resolver versions;
/// 2. the exact root coordinate;
/// 3. entries sorted by `PackKey`, each containing its coordinate, source
///    snapshot, artifact format version, byte length, artifact digest, export
///    digest, and direct dependencies;
/// 4. for every dependency, its coordinate, artifact digest, and export
///    digest;
/// 5. the exact required semantic-interface union sorted by interface key.
///
/// Only [`ExactPackSet::finalize`] can construct a lock.
///
/// ```compile_fail
/// let _ = world_defs::PackLock {};
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackLock {
    schema_version: u16,
    resolver_version: u16,
    root: PackCoordinate,
    entries: Vec<PackLockEntry>,
    required_interfaces: Vec<SemanticInterfaceReference>,
    digest: PackLockDigest,
}

impl PackLock {
    /// Returns the canonical lock-schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// Returns the exact resolver protocol version.
    #[must_use]
    pub const fn resolver_version(&self) -> u16 {
        self.resolver_version
    }

    /// Returns the exact root coordinate.
    #[must_use]
    pub const fn root(&self) -> &PackCoordinate {
        &self.root
    }

    /// Returns package entries in canonical `PackKey` order.
    #[must_use]
    pub fn entries(&self) -> &[PackLockEntry] {
        &self.entries
    }

    /// Returns the exact required-interface union in canonical key order.
    #[must_use]
    pub fn required_interfaces(&self) -> &[SemanticInterfaceReference] {
        &self.required_interfaces
    }

    /// Returns the canonical lock identity.
    #[must_use]
    pub const fn digest(&self) -> PackLockDigest {
        self.digest
    }
}

/// A closed, exact, proof-carrying set of verified pack artifacts.
///
/// There is no public constructor. Finalization creates the matching private
/// lock only after the source selection, artifact coordinates, graph edges,
/// exports, engine protocol, and interface union agree.
///
/// ```compile_fail
/// let _ = world_defs::ExactPackSet {};
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPackSet {
    artifacts: Vec<VerifiedPackArtifact>,
    lock: PackLock,
    engine_protocol: EngineProtocolVersion,
}

impl ExactPackSet {
    /// Finalizes an exact package selection and its verified artifacts.
    pub fn finalize(
        selection: ExactPackageSelection,
        mut artifacts: Vec<VerifiedPackArtifact>,
    ) -> Result<Self, PackSetError> {
        let packages = normalize_selection(selection)?;
        sort_and_check_artifacts(&mut artifacts)?;

        check_selection_artifact_correspondence(&packages, &artifacts)?;

        let root_index = match find_artifact_index(&artifacts, packages.root.pack_key()) {
            Some(index) => index,
            None => unreachable!("selection correspondence must include the root artifact"),
        };
        let engine_protocol = artifacts[root_index].engine_protocol();
        check_engine_protocol(engine_protocol, &artifacts)?;
        check_dependencies(&artifacts)?;
        check_graph(&packages.root, &artifacts)?;
        let required_interfaces = collect_required_interfaces(&artifacts)?;
        let entries = build_lock_entries(&packages.packages, &artifacts);
        let digest = compute_lock_digest(
            PACK_LOCK_SCHEMA_VERSION,
            PACK_RESOLVER_VERSION,
            &packages.root,
            &entries,
            &required_interfaces,
        )?;
        let lock = PackLock {
            schema_version: PACK_LOCK_SCHEMA_VERSION,
            resolver_version: PACK_RESOLVER_VERSION,
            root: packages.root,
            entries,
            required_interfaces,
            digest,
        };

        Ok(Self {
            artifacts,
            lock,
            engine_protocol,
        })
    }

    /// Returns the exact root coordinate.
    #[must_use]
    pub const fn root(&self) -> &PackCoordinate {
        self.lock.root()
    }

    /// Returns the common required engine protocol.
    #[must_use]
    pub const fn engine_protocol(&self) -> EngineProtocolVersion {
        self.engine_protocol
    }

    /// Returns the matching immutable package lock.
    #[must_use]
    pub const fn lock(&self) -> &PackLock {
        &self.lock
    }

    /// Returns verified artifacts in canonical `PackKey` order.
    #[must_use]
    pub fn artifacts(&self) -> &[VerifiedPackArtifact] {
        &self.artifacts
    }

    pub(crate) fn into_parts(self) -> (PackLock, Vec<VerifiedPackArtifact>) {
        (self.lock, self.artifacts)
    }
}

/// Why an exact package set could not be finalized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackSetError {
    /// A package-bearing input exceeded the exact-set closure limit.
    TooManyPackages {
        /// Input collection being checked.
        collection: &'static str,
        /// Supplied number of packages.
        actual: usize,
        /// Maximum accepted package count.
        maximum: usize,
    },
    /// The selection repeats the same package coordinate.
    DuplicateSelectedPackage {
        /// Repeated package key.
        pack: PackKey,
    },
    /// The selection contains two coordinates for one durable pack key.
    ConflictingSelectedPackages {
        /// Conflicting durable pack key.
        pack: PackKey,
        /// First exact coordinate.
        first: PackCoordinate,
        /// Second exact coordinate.
        second: PackCoordinate,
    },
    /// One selected package repeats a direct dependency.
    DuplicateSelectedDependency {
        /// Package declaring the edge.
        package: PackCoordinate,
        /// Repeated dependency key.
        dependency: PackKey,
    },
    /// One selected package declares conflicting coordinates for a dependency.
    ConflictingSelectedDependencies {
        /// Package declaring the edges.
        package: PackCoordinate,
        /// Conflicting durable dependency key.
        dependency: PackKey,
        /// First exact coordinate.
        first: Box<PackCoordinate>,
        /// Second exact coordinate.
        second: Box<PackCoordinate>,
    },
    /// The verified input repeats the same package coordinate.
    DuplicateArtifact {
        /// Repeated package key.
        pack: PackKey,
    },
    /// Verified inputs contain two coordinates for one durable pack key.
    ConflictingArtifacts {
        /// Conflicting durable pack key.
        pack: PackKey,
        /// First exact coordinate.
        first: PackCoordinate,
        /// Second exact coordinate.
        second: PackCoordinate,
    },
    /// The selected root package is absent.
    MissingRoot {
        /// Requested root coordinate.
        root: PackCoordinate,
    },
    /// The selected package for the root key has a different coordinate.
    RootCoordinateMismatch {
        /// Requested root coordinate.
        requested: PackCoordinate,
        /// Coordinate selected for the same key.
        selected: PackCoordinate,
    },
    /// A selected package has no verified artifact.
    MissingArtifact {
        /// Selected coordinate without an artifact.
        coordinate: PackCoordinate,
    },
    /// A verified artifact is outside the exact source selection.
    ExtraArtifact {
        /// Unselected artifact coordinate.
        coordinate: PackCoordinate,
    },
    /// A selected coordinate and its artifact coordinate differ.
    CoordinateMismatch {
        /// Durable pack key.
        pack: PackKey,
        /// Selected exact coordinate.
        selected: PackCoordinate,
        /// Verified artifact coordinate.
        artifact: PackCoordinate,
    },
    /// Source selection and artifact manifest declare different direct edges.
    DependencyEdgesMismatch {
        /// Package whose edges differ.
        package: PackCoordinate,
        /// Resolver-selected exact edges.
        selected: Vec<PackCoordinate>,
        /// Artifact-declared exact edges.
        artifact: Vec<PackCoordinate>,
    },
    /// A package requires a different engine protocol from the root.
    EngineProtocolMismatch {
        /// Package with the conflicting requirement.
        package: PackCoordinate,
        /// Root package protocol.
        expected: EngineProtocolVersion,
        /// Conflicting protocol.
        actual: EngineProtocolVersion,
    },
    /// A declared direct dependency has no selected artifact.
    MissingDependency {
        /// Package declaring the dependency.
        package: PackCoordinate,
        /// Missing exact dependency.
        dependency: PackCoordinate,
    },
    /// A dependency key resolves to a different exact coordinate.
    DependencyCoordinateMismatch {
        /// Package declaring the dependency.
        package: PackCoordinate,
        /// Exact coordinate required by the package.
        dependency: PackCoordinate,
        /// Selected coordinate for the same durable key.
        selected: PackCoordinate,
    },
    /// A dependency's public export identity differs from the expected value.
    DependencyExportMismatch {
        /// Package declaring the dependency.
        package: Box<PackCoordinate>,
        /// Exact dependency coordinate.
        dependency: Box<PackCoordinate>,
        /// Expected public export identity.
        expected: PackExportDigest,
        /// Actual selected public export identity.
        actual: PackExportDigest,
    },
    /// The selected dependency graph contains a cycle.
    DependencyCycle {
        /// A package reached while it was already being visited.
        package: PackCoordinate,
    },
    /// A selected package is not reachable from the exact root.
    UnreachablePackage {
        /// Unreachable package coordinate.
        package: PackCoordinate,
    },
    /// Two packages require incompatible descriptors for one interface key.
    InterfaceConflict {
        /// Conflicting interface key.
        interface: SemanticInterfaceKey,
        /// First exact requirement.
        first: Box<SemanticInterfaceReference>,
        /// Second exact requirement.
        second: Box<SemanticInterfaceReference>,
    },
    /// The normalized lock could not be represented canonically.
    Canonical(CanonicalError),
}

impl fmt::Display for PackSetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyPackages {
                collection,
                actual,
                maximum,
            } => write!(
                formatter,
                "{collection} contains {actual} packages; the maximum is {maximum}"
            ),
            Self::DuplicateSelectedPackage { pack } => {
                write!(formatter, "package selection repeats {pack}")
            }
            Self::ConflictingSelectedPackages {
                pack,
                first,
                second,
            } => write!(
                formatter,
                "package selection has conflicting coordinates for {pack}: {first} and {second}"
            ),
            Self::DuplicateSelectedDependency {
                package,
                dependency,
            } => write!(
                formatter,
                "selected package {package} repeats dependency {dependency}"
            ),
            Self::ConflictingSelectedDependencies {
                package,
                dependency,
                first,
                second,
            } => write!(
                formatter,
                "selected package {package} has conflicting coordinates for dependency \
                 {dependency}: {first} and {second}"
            ),
            Self::DuplicateArtifact { pack } => {
                write!(formatter, "verified artifacts repeat {pack}")
            }
            Self::ConflictingArtifacts {
                pack,
                first,
                second,
            } => write!(
                formatter,
                "verified artifacts have conflicting coordinates for {pack}: {first} and {second}"
            ),
            Self::MissingRoot { root } => {
                write!(formatter, "selected root {root} is missing")
            }
            Self::RootCoordinateMismatch {
                requested,
                selected,
            } => write!(
                formatter,
                "requested root {requested} differs from selected coordinate {selected}"
            ),
            Self::MissingArtifact { coordinate } => {
                write!(formatter, "selected package {coordinate} has no artifact")
            }
            Self::ExtraArtifact { coordinate } => {
                write!(formatter, "artifact {coordinate} is outside the selection")
            }
            Self::CoordinateMismatch {
                pack,
                selected,
                artifact,
            } => write!(
                formatter,
                "selected coordinate {selected} for {pack} differs from artifact {artifact}"
            ),
            Self::DependencyEdgesMismatch {
                package,
                selected,
                artifact,
            } => write!(
                formatter,
                "selected and artifact dependency edges differ for {package}: selected \
                 {selected:?}, artifact {artifact:?}"
            ),
            Self::EngineProtocolMismatch {
                package,
                expected,
                actual,
            } => write!(
                formatter,
                "package {package} requires engine protocol {actual}; root requires {expected}"
            ),
            Self::MissingDependency {
                package,
                dependency,
            } => write!(
                formatter,
                "package {package} requires missing dependency {dependency}"
            ),
            Self::DependencyCoordinateMismatch {
                package,
                dependency,
                selected,
            } => write!(
                formatter,
                "package {package} requires dependency {dependency}, but {selected} is selected"
            ),
            Self::DependencyExportMismatch {
                package,
                dependency,
                expected,
                actual,
            } => write!(
                formatter,
                "package {package} expected export {expected} from {dependency}, found {actual}"
            ),
            Self::DependencyCycle { package } => {
                write!(formatter, "dependency cycle reaches {package}")
            }
            Self::UnreachablePackage { package } => {
                write!(
                    formatter,
                    "package {package} is unreachable from the selected root"
                )
            }
            Self::InterfaceConflict {
                interface,
                first,
                second,
            } => write!(
                formatter,
                "packages require conflicting descriptors for interface {interface}: \
                 {first:?} and {second:?}"
            ),
            Self::Canonical(error) => write!(formatter, "pack lock identity failed: {error}"),
        }
    }
}

impl std::error::Error for PackSetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CanonicalError> for PackSetError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

fn normalize_selection(
    selection: ExactPackageSelection,
) -> Result<ExactPackageSelection, PackSetError> {
    let ExactPackageSelection { root, mut packages } = selection;
    check_package_limit("selected packages", packages.len())?;

    packages.sort_by(|left, right| {
        left.coordinate
            .pack_key()
            .cmp(right.coordinate.pack_key())
            .then_with(|| left.coordinate.cmp(&right.coordinate))
    });
    for adjacent in packages.windows(2) {
        if adjacent[0].coordinate.pack_key() != adjacent[1].coordinate.pack_key() {
            continue;
        }
        if adjacent[0].coordinate == adjacent[1].coordinate {
            return Err(PackSetError::DuplicateSelectedPackage {
                pack: adjacent[0].coordinate.pack_key().clone(),
            });
        }
        return Err(PackSetError::ConflictingSelectedPackages {
            pack: adjacent[0].coordinate.pack_key().clone(),
            first: adjacent[0].coordinate.clone(),
            second: adjacent[1].coordinate.clone(),
        });
    }

    for package in &mut packages {
        package.dependencies.sort_by(|left, right| {
            left.pack_key()
                .cmp(right.pack_key())
                .then_with(|| left.cmp(right))
        });
        for adjacent in package.dependencies.windows(2) {
            if adjacent[0].pack_key() != adjacent[1].pack_key() {
                continue;
            }
            if adjacent[0] == adjacent[1] {
                return Err(PackSetError::DuplicateSelectedDependency {
                    package: package.coordinate.clone(),
                    dependency: adjacent[0].pack_key().clone(),
                });
            }
            return Err(PackSetError::ConflictingSelectedDependencies {
                package: package.coordinate.clone(),
                dependency: adjacent[0].pack_key().clone(),
                first: Box::new(adjacent[0].clone()),
                second: Box::new(adjacent[1].clone()),
            });
        }
    }

    match packages.binary_search_by(|package| package.coordinate.pack_key().cmp(root.pack_key())) {
        Ok(index) if packages[index].coordinate == root => {}
        Ok(index) => {
            return Err(PackSetError::RootCoordinateMismatch {
                requested: root,
                selected: packages[index].coordinate.clone(),
            });
        }
        Err(_) => return Err(PackSetError::MissingRoot { root }),
    }

    Ok(ExactPackageSelection { root, packages })
}

fn sort_and_check_artifacts(artifacts: &mut [VerifiedPackArtifact]) -> Result<(), PackSetError> {
    artifacts.sort_by(|left, right| {
        left.coordinate()
            .pack_key()
            .cmp(right.coordinate().pack_key())
            .then_with(|| left.coordinate().cmp(right.coordinate()))
    });
    for adjacent in artifacts.windows(2) {
        if adjacent[0].coordinate().pack_key() != adjacent[1].coordinate().pack_key() {
            continue;
        }
        if adjacent[0].coordinate() == adjacent[1].coordinate() {
            return Err(PackSetError::DuplicateArtifact {
                pack: adjacent[0].coordinate().pack_key().clone(),
            });
        }
        return Err(PackSetError::ConflictingArtifacts {
            pack: adjacent[0].coordinate().pack_key().clone(),
            first: adjacent[0].coordinate().clone(),
            second: adjacent[1].coordinate().clone(),
        });
    }
    Ok(())
}

fn check_selection_artifact_correspondence(
    selection: &ExactPackageSelection,
    artifacts: &[VerifiedPackArtifact],
) -> Result<(), PackSetError> {
    for package in &selection.packages {
        let Some(index) = find_artifact_index(artifacts, package.coordinate.pack_key()) else {
            return Err(PackSetError::MissingArtifact {
                coordinate: package.coordinate.clone(),
            });
        };
        let artifact = &artifacts[index];
        if artifact.coordinate() != &package.coordinate {
            return Err(PackSetError::CoordinateMismatch {
                pack: package.coordinate.pack_key().clone(),
                selected: package.coordinate.clone(),
                artifact: artifact.coordinate().clone(),
            });
        }

        let artifact_dependencies = artifact
            .dependencies()
            .iter()
            .map(|dependency| dependency.coordinate().clone())
            .collect::<Vec<_>>();
        if package.dependencies != artifact_dependencies {
            return Err(PackSetError::DependencyEdgesMismatch {
                package: package.coordinate.clone(),
                selected: package.dependencies.clone(),
                artifact: artifact_dependencies,
            });
        }
    }

    for artifact in artifacts {
        if find_selected_index(&selection.packages, artifact.coordinate().pack_key()).is_none() {
            return Err(PackSetError::ExtraArtifact {
                coordinate: artifact.coordinate().clone(),
            });
        }
    }
    Ok(())
}

fn check_engine_protocol(
    expected: EngineProtocolVersion,
    artifacts: &[VerifiedPackArtifact],
) -> Result<(), PackSetError> {
    for artifact in artifacts {
        let actual = artifact.engine_protocol();
        if actual != expected {
            return Err(PackSetError::EngineProtocolMismatch {
                package: artifact.coordinate().clone(),
                expected,
                actual,
            });
        }
    }
    Ok(())
}

fn check_dependencies(artifacts: &[VerifiedPackArtifact]) -> Result<(), PackSetError> {
    for artifact in artifacts {
        for dependency in artifact.dependencies() {
            let Some(index) = find_artifact_index(artifacts, dependency.coordinate().pack_key())
            else {
                return Err(PackSetError::MissingDependency {
                    package: artifact.coordinate().clone(),
                    dependency: dependency.coordinate().clone(),
                });
            };
            let actual = &artifacts[index];
            if actual.coordinate() != dependency.coordinate() {
                return Err(PackSetError::DependencyCoordinateMismatch {
                    package: artifact.coordinate().clone(),
                    dependency: dependency.coordinate().clone(),
                    selected: actual.coordinate().clone(),
                });
            }
            if dependency.expected_export_digest() != actual.export_digest() {
                return Err(PackSetError::DependencyExportMismatch {
                    package: Box::new(artifact.coordinate().clone()),
                    dependency: Box::new(dependency.coordinate().clone()),
                    expected: dependency.expected_export_digest(),
                    actual: actual.export_digest(),
                });
            }
        }
    }

    Ok(())
}

fn check_graph(
    root: &PackCoordinate,
    artifacts: &[VerifiedPackArtifact],
) -> Result<(), PackSetError> {
    let root_index = match find_artifact_index(artifacts, root.pack_key()) {
        Some(index) => index,
        None => unreachable!("validated selection correspondence must include the root"),
    };
    let mut states = vec![VisitState::Unvisited; artifacts.len()];
    visit_artifact(root_index, artifacts, &mut states)?;

    for (artifact, state) in artifacts.iter().zip(states) {
        if state == VisitState::Unvisited {
            return Err(PackSetError::UnreachablePackage {
                package: artifact.coordinate().clone(),
            });
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

fn visit_artifact(
    index: usize,
    artifacts: &[VerifiedPackArtifact],
    states: &mut [VisitState],
) -> Result<(), PackSetError> {
    match states[index] {
        VisitState::Visited => return Ok(()),
        VisitState::Visiting => {
            return Err(PackSetError::DependencyCycle {
                package: artifacts[index].coordinate().clone(),
            });
        }
        VisitState::Unvisited => {}
    }

    states[index] = VisitState::Visiting;
    for dependency in artifacts[index].dependencies() {
        let dependency_index =
            match find_artifact_index(artifacts, dependency.coordinate().pack_key()) {
                Some(index) => index,
                None => unreachable!("validated artifact dependencies must be present"),
            };
        visit_artifact(dependency_index, artifacts, states)?;
    }
    states[index] = VisitState::Visited;
    Ok(())
}

fn collect_required_interfaces(
    artifacts: &[VerifiedPackArtifact],
) -> Result<Vec<SemanticInterfaceReference>, PackSetError> {
    let mut interfaces = artifacts
        .iter()
        .flat_map(|artifact| artifact.required_interfaces().iter().cloned())
        .collect::<Vec<_>>();
    interfaces.sort_by(|left, right| left.key().cmp(right.key()).then_with(|| left.cmp(right)));

    let mut exact: Vec<SemanticInterfaceReference> = Vec::with_capacity(interfaces.len());
    for interface in interfaces {
        match exact.last() {
            Some(previous) if previous.key() == interface.key() && previous != &interface => {
                return Err(PackSetError::InterfaceConflict {
                    interface: interface.key().clone(),
                    first: Box::new(previous.clone()),
                    second: Box::new(interface),
                });
            }
            Some(previous) if previous == &interface => {}
            _ => exact.push(interface),
        }
    }
    Ok(exact)
}

fn build_lock_entries(
    selected: &[SelectedPackage],
    artifacts: &[VerifiedPackArtifact],
) -> Vec<PackLockEntry> {
    let mut entries = Vec::with_capacity(artifacts.len());
    for artifact in artifacts {
        let selected_index = match find_selected_index(selected, artifact.coordinate().pack_key()) {
            Some(index) => index,
            None => unreachable!("validated artifact must have a selected source"),
        };
        let selected_package = &selected[selected_index];

        let mut dependencies = Vec::with_capacity(artifact.dependencies().len());
        for dependency in artifact.dependencies() {
            let dependency_index =
                match find_artifact_index(artifacts, dependency.coordinate().pack_key()) {
                    Some(index) => index,
                    None => unreachable!("validated artifact dependency must be present"),
                };
            let dependency_artifact = &artifacts[dependency_index];
            dependencies.push(PackLockDependency {
                coordinate: dependency_artifact.coordinate().clone(),
                artifact_digest: dependency_artifact.artifact_digest(),
                export_digest: dependency_artifact.export_digest(),
            });
        }

        entries.push(PackLockEntry {
            coordinate: artifact.coordinate().clone(),
            source_snapshot: selected_package.source_snapshot,
            artifact_format_version: artifact.envelope().descriptor().format_version(),
            artifact_byte_length: artifact.envelope().descriptor().blob_length(),
            artifact_digest: artifact.artifact_digest(),
            export_digest: artifact.export_digest(),
            dependencies,
        });
    }
    entries
}

fn compute_lock_digest(
    schema_version: u16,
    resolver_version: u16,
    root: &PackCoordinate,
    entries: &[PackLockEntry],
    required_interfaces: &[SemanticInterfaceReference],
) -> Result<PackLockDigest, CanonicalError> {
    let mut writer = CanonicalWriter::new(PACK_LOCK_DOMAIN);
    writer.write_u16(schema_version);
    writer.write_u16(resolver_version);
    write_coordinate(&mut writer, root)?;
    writer.write_sequence(entries, |writer, entry| {
        write_coordinate(writer, &entry.coordinate)?;
        writer.write_bytes(entry.source_snapshot.as_bytes())?;
        writer.write_u16(entry.artifact_format_version);
        writer.write_u64(entry.artifact_byte_length);
        writer.write_bytes(entry.artifact_digest.as_bytes())?;
        writer.write_bytes(entry.export_digest.as_bytes())?;
        writer.write_sequence(&entry.dependencies, |writer, dependency| {
            write_coordinate(writer, &dependency.coordinate)?;
            writer.write_bytes(dependency.artifact_digest.as_bytes())?;
            writer.write_bytes(dependency.export_digest.as_bytes())
        })
    })?;
    writer.write_sequence(required_interfaces, write_interface_reference)?;
    Ok(PackLockDigest(ContentDigest::of_canonical(
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

fn write_interface_reference(
    writer: &mut CanonicalWriter,
    interface: &SemanticInterfaceReference,
) -> Result<(), CanonicalError> {
    writer.write_str(interface.key().as_str())?;
    writer.write_u16(interface.version().get());
    writer.write_bytes(interface.digest().as_bytes())
}

fn find_artifact_index(artifacts: &[VerifiedPackArtifact], pack_key: &PackKey) -> Option<usize> {
    artifacts
        .binary_search_by(|artifact| artifact.coordinate().pack_key().cmp(pack_key))
        .ok()
}

fn find_selected_index(selected: &[SelectedPackage], pack_key: &PackKey) -> Option<usize> {
    selected
        .binary_search_by(|package| package.coordinate.pack_key().cmp(pack_key))
        .ok()
}

fn check_package_limit(collection: &'static str, actual: usize) -> Result<(), PackSetError> {
    if actual > MAX_PACKAGES_PER_SET {
        return Err(PackSetError::TooManyPackages {
            collection,
            actual,
            maximum: MAX_PACKAGES_PER_SET,
        });
    }
    Ok(())
}

fn write_hex(formatter: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{ArtifactData, ArtifactValidator, PackDependency, PackManifestData};
    use crate::definition::{
        ActionBindingData, ActionData, EffectCallData, EventData, EventEmissionData,
        EventFieldBindingData, EventFieldData, OperationCallData,
    };
    use crate::interface::{
        OperationKind, OperationName, OperationParameter, ParameterName, SemanticInterfaceCatalog,
        SemanticInterfaceDescriptor, SemanticOperationDescriptor, ValueKind,
    };
    use crate::key::{
        BindingName, DefinitionKey, EventFieldName, InterfaceVersion, LocalDefinitionName,
        PackVersion, SemanticInterfaceKey,
    };

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("fixture must be valid: {error}"),
        }
    }

    fn rejected(result: Result<ExactPackSet, PackSetError>) -> PackSetError {
        match result {
            Ok(_) => panic!("invalid exact-set fixture was accepted"),
            Err(error) => error,
        }
    }

    fn coordinate(name: &str) -> PackCoordinate {
        coordinate_at(name, 1)
    }

    fn coordinate_at(name: &str, major: u32) -> PackCoordinate {
        PackCoordinate::new(valid(PackKey::parse(name)), PackVersion::new(major, 0, 0))
    }

    fn interface_fixture() -> (
        SemanticInterfaceCatalog,
        SemanticInterfaceReference,
        OperationName,
    ) {
        interface_fixture_at(1)
    }

    fn interface_fixture_at(
        version: u16,
    ) -> (
        SemanticInterfaceCatalog,
        SemanticInterfaceReference,
        OperationName,
    ) {
        let interface_key = valid(SemanticInterfaceKey::parse("test.interface"));
        let operation_name = valid(OperationName::parse("apply"));
        let parameter =
            OperationParameter::new(valid(ParameterName::parse("subject")), ValueKind::Entity);
        let operation = valid(SemanticOperationDescriptor::new(
            operation_name.clone(),
            OperationKind::Effect,
            vec![parameter],
        ));
        let descriptor = valid(SemanticInterfaceDescriptor::new(
            interface_key,
            valid(InterfaceVersion::new(version)),
            vec![operation],
        ));
        let reference = descriptor.reference();
        let catalog = valid(SemanticInterfaceCatalog::new(vec![descriptor]));
        (catalog, reference, operation_name)
    }

    fn artifact(
        validator: &ArtifactValidator<'_>,
        coordinate: PackCoordinate,
        engine_protocol: EngineProtocolVersion,
        dependencies: Vec<PackDependency>,
        interface: SemanticInterfaceReference,
        operation: OperationName,
    ) -> VerifiedPackArtifact {
        let binding_name = valid(BindingName::parse("subject"));
        let event_name = valid(LocalDefinitionName::parse("changed"));
        let field_name = valid(EventFieldName::parse("subject"));
        let event = EventData::new(
            event_name.clone(),
            vec![EventFieldData::new(field_name.clone(), ValueKind::Entity)],
        );
        let call = OperationCallData::new(
            interface.key().clone(),
            operation,
            vec![binding_name.clone()],
        );
        let emission = EventEmissionData::new(
            DefinitionKey::new(coordinate.pack_key().clone(), event_name),
            vec![EventFieldBindingData::new(field_name, binding_name.clone())],
        );
        let action = ActionData::new(
            valid(LocalDefinitionName::parse("change")),
            vec![ActionBindingData::new(binding_name, ValueKind::Entity)],
            Vec::new(),
            vec![EffectCallData::new(call)],
            vec![emission],
        );
        let manifest = PackManifestData::new(engine_protocol, coordinate, dependencies);
        valid(validator.validate(ArtifactData::new(
            manifest,
            vec![interface],
            vec![action],
            vec![event],
        )))
    }

    struct PairFixture {
        root_coordinate: PackCoordinate,
        leaf_coordinate: PackCoordinate,
        root: VerifiedPackArtifact,
        leaf: VerifiedPackArtifact,
    }

    fn pair_fixture(
        validator: &ArtifactValidator<'_>,
        interface: SemanticInterfaceReference,
        operation: OperationName,
        root_protocol: u16,
        leaf_protocol: u16,
    ) -> PairFixture {
        let root_coordinate = coordinate("test.root");
        let leaf_coordinate = coordinate("test.leaf");
        let leaf = artifact(
            validator,
            leaf_coordinate.clone(),
            EngineProtocolVersion::new(leaf_protocol),
            Vec::new(),
            interface.clone(),
            operation.clone(),
        );
        let root = artifact(
            validator,
            root_coordinate.clone(),
            EngineProtocolVersion::new(root_protocol),
            vec![PackDependency::new(
                leaf_coordinate.clone(),
                leaf.export_digest(),
            )],
            interface,
            operation,
        );
        PairFixture {
            root_coordinate,
            leaf_coordinate,
            root,
            leaf,
        }
    }

    fn pair_selection(pair: &PairFixture) -> ExactPackageSelection {
        ExactPackageSelection::new(
            pair.root_coordinate.clone(),
            vec![
                selected(
                    pair.root_coordinate.clone(),
                    1,
                    vec![pair.leaf_coordinate.clone()],
                ),
                selected(pair.leaf_coordinate.clone(), 2, Vec::new()),
            ],
        )
    }

    fn selected(
        coordinate: PackCoordinate,
        source_byte: u8,
        dependencies: Vec<PackCoordinate>,
    ) -> SelectedPackage {
        SelectedPackage::new(
            coordinate,
            SourceSnapshotId::from_bytes([source_byte; 32]),
            dependencies,
        )
    }

    #[test]
    fn finalization_normalizes_order_and_source_changes_only_the_lock() {
        let (catalog, interface, operation) = interface_fixture();
        let validator = ArtifactValidator::new(&catalog);
        let leaf_coordinate = coordinate("test.leaf");
        let root_coordinate = coordinate("test.root");
        let leaf = artifact(
            &validator,
            leaf_coordinate.clone(),
            EngineProtocolVersion::new(1),
            Vec::new(),
            interface.clone(),
            operation.clone(),
        );
        let root = artifact(
            &validator,
            root_coordinate.clone(),
            EngineProtocolVersion::new(1),
            vec![PackDependency::new(
                leaf_coordinate.clone(),
                leaf.export_digest(),
            )],
            interface,
            operation,
        );

        let forward_selection = ExactPackageSelection::new(
            root_coordinate.clone(),
            vec![
                selected(root_coordinate.clone(), 1, vec![leaf_coordinate.clone()]),
                selected(leaf_coordinate.clone(), 2, Vec::new()),
            ],
        );
        let reverse_selection = ExactPackageSelection::new(
            root_coordinate.clone(),
            vec![
                selected(leaf_coordinate.clone(), 2, Vec::new()),
                selected(root_coordinate.clone(), 1, vec![leaf_coordinate.clone()]),
            ],
        );
        let forward = valid(ExactPackSet::finalize(
            forward_selection,
            vec![root.clone(), leaf.clone()],
        ));
        let reverse = valid(ExactPackSet::finalize(
            reverse_selection,
            vec![leaf.clone(), root.clone()],
        ));

        assert_eq!(forward, reverse);
        assert_eq!(forward.artifacts()[0].coordinate(), &leaf_coordinate);
        assert_eq!(forward.artifacts()[1].coordinate(), &root_coordinate);

        let root_entry = &forward.lock().entries()[1];
        assert_eq!(
            root_entry.artifact_format_version(),
            root.envelope().descriptor().format_version()
        );
        assert_eq!(
            root_entry.artifact_byte_length(),
            root.envelope().descriptor().blob_length()
        );
        assert_eq!(root_entry.dependencies().len(), 1);
        assert_eq!(
            root_entry.dependencies()[0].artifact_digest(),
            leaf.artifact_digest()
        );
        assert_eq!(
            root_entry.dependencies()[0].export_digest(),
            leaf.export_digest()
        );

        let changed_source = ExactPackageSelection::new(
            root_coordinate.clone(),
            vec![
                selected(root_coordinate.clone(), 9, vec![leaf_coordinate.clone()]),
                selected(leaf_coordinate, 2, Vec::new()),
            ],
        );
        let changed = valid(ExactPackSet::finalize(
            changed_source,
            vec![root.clone(), leaf.clone()],
        ));

        assert_ne!(forward.lock().digest(), changed.lock().digest());
        assert_eq!(
            forward
                .artifacts()
                .iter()
                .map(VerifiedPackArtifact::artifact_digest)
                .collect::<Vec<_>>(),
            changed
                .artifacts()
                .iter()
                .map(VerifiedPackArtifact::artifact_digest)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn finalization_rejects_duplicate_and_conflicting_verified_artifacts() {
        let (catalog, interface, operation) = interface_fixture();
        let validator = ArtifactValidator::new(&catalog);
        let first_coordinate = coordinate_at("test.root", 1);
        let second_coordinate = coordinate_at("test.root", 2);
        let first = artifact(
            &validator,
            first_coordinate.clone(),
            EngineProtocolVersion::new(1),
            Vec::new(),
            interface.clone(),
            operation.clone(),
        );
        let second = artifact(
            &validator,
            second_coordinate.clone(),
            EngineProtocolVersion::new(1),
            Vec::new(),
            interface,
            operation,
        );
        let selection = ExactPackageSelection::new(
            first_coordinate.clone(),
            vec![selected(first_coordinate.clone(), 1, Vec::new())],
        );

        assert_eq!(
            rejected(ExactPackSet::finalize(
                selection.clone(),
                vec![first.clone(), first.clone()],
            )),
            PackSetError::DuplicateArtifact {
                pack: first_coordinate.pack_key().clone()
            }
        );
        assert_eq!(
            rejected(ExactPackSet::finalize(selection, vec![second, first],)),
            PackSetError::ConflictingArtifacts {
                pack: first_coordinate.pack_key().clone(),
                first: first_coordinate,
                second: second_coordinate,
            }
        );
    }

    #[test]
    fn finalization_rejects_edge_and_expected_export_mismatches() {
        let (catalog, interface, operation) = interface_fixture();
        let validator = ArtifactValidator::new(&catalog);
        let leaf_coordinate = coordinate("test.leaf");
        let root_coordinate = coordinate("test.root");
        let leaf = artifact(
            &validator,
            leaf_coordinate.clone(),
            EngineProtocolVersion::new(1),
            Vec::new(),
            interface.clone(),
            operation.clone(),
        );
        let root = artifact(
            &validator,
            root_coordinate.clone(),
            EngineProtocolVersion::new(1),
            vec![PackDependency::new(
                leaf_coordinate.clone(),
                leaf.export_digest(),
            )],
            interface.clone(),
            operation.clone(),
        );

        let wrong_edges = ExactPackageSelection::new(
            root_coordinate.clone(),
            vec![
                selected(root_coordinate.clone(), 1, Vec::new()),
                selected(leaf_coordinate.clone(), 2, Vec::new()),
            ],
        );
        assert!(matches!(
            ExactPackSet::finalize(wrong_edges, vec![root, leaf.clone()]),
            Err(PackSetError::DependencyEdgesMismatch { .. })
        ));

        let wrong_export_root = artifact(
            &validator,
            root_coordinate.clone(),
            EngineProtocolVersion::new(1),
            vec![PackDependency::new(
                leaf_coordinate.clone(),
                PackExportDigest::from_bytes([7; 32]),
            )],
            interface,
            operation,
        );
        let matching_edges = ExactPackageSelection::new(
            root_coordinate.clone(),
            vec![
                selected(root_coordinate, 1, vec![leaf_coordinate.clone()]),
                selected(leaf_coordinate, 2, Vec::new()),
            ],
        );
        assert!(matches!(
            ExactPackSet::finalize(matching_edges, vec![wrong_export_root, leaf]),
            Err(PackSetError::DependencyExportMismatch { .. })
        ));
    }

    #[test]
    fn finalization_rejects_inexact_closure_members_and_versions() {
        let (catalog, interface, operation) = interface_fixture();
        let validator = ArtifactValidator::new(&catalog);
        let pair = pair_fixture(&validator, interface.clone(), operation.clone(), 1, 1);

        assert_eq!(
            rejected(ExactPackSet::finalize(
                pair_selection(&pair),
                vec![pair.root.clone()],
            )),
            PackSetError::MissingArtifact {
                coordinate: pair.leaf_coordinate.clone(),
            }
        );

        let standalone_root = artifact(
            &validator,
            pair.root_coordinate.clone(),
            EngineProtocolVersion::new(1),
            Vec::new(),
            interface.clone(),
            operation.clone(),
        );
        let root_only = ExactPackageSelection::new(
            pair.root_coordinate.clone(),
            vec![selected(pair.root_coordinate.clone(), 1, Vec::new())],
        );
        assert_eq!(
            rejected(ExactPackSet::finalize(
                root_only,
                vec![standalone_root, pair.leaf.clone()],
            )),
            PackSetError::ExtraArtifact {
                coordinate: pair.leaf_coordinate.clone(),
            }
        );

        let required_leaf = pair.leaf_coordinate.clone();
        let selected_leaf = coordinate_at("test.leaf", 2);
        let selected_leaf_artifact = artifact(
            &validator,
            selected_leaf.clone(),
            EngineProtocolVersion::new(1),
            Vec::new(),
            interface.clone(),
            operation.clone(),
        );
        let version_bound_root = artifact(
            &validator,
            pair.root_coordinate.clone(),
            EngineProtocolVersion::new(1),
            vec![PackDependency::new(
                required_leaf.clone(),
                selected_leaf_artifact.export_digest(),
            )],
            interface.clone(),
            operation.clone(),
        );
        let conflicting_version = ExactPackageSelection::new(
            pair.root_coordinate.clone(),
            vec![
                selected(pair.root_coordinate.clone(), 1, vec![required_leaf.clone()]),
                selected(selected_leaf.clone(), 2, Vec::new()),
            ],
        );
        assert_eq!(
            rejected(ExactPackSet::finalize(
                conflicting_version,
                vec![version_bound_root, selected_leaf_artifact],
            )),
            PackSetError::DependencyCoordinateMismatch {
                package: pair.root_coordinate.clone(),
                dependency: required_leaf,
                selected: selected_leaf,
            }
        );

        let orphan_coordinate = coordinate("test.orphan");
        let orphan = artifact(
            &validator,
            orphan_coordinate.clone(),
            EngineProtocolVersion::new(1),
            Vec::new(),
            interface,
            operation,
        );
        let mut unreachable_selection = pair_selection(&pair);
        unreachable_selection
            .packages
            .push(selected(orphan_coordinate.clone(), 3, Vec::new()));
        assert_eq!(
            rejected(ExactPackSet::finalize(
                unreachable_selection,
                vec![pair.root, pair.leaf, orphan],
            )),
            PackSetError::UnreachablePackage {
                package: orphan_coordinate,
            }
        );
    }

    #[test]
    fn finalization_rejects_runtime_contract_conflicts() {
        let (catalog, interface, operation) = interface_fixture();
        let validator = ArtifactValidator::new(&catalog);
        let mismatched_engine = pair_fixture(&validator, interface, operation, 1, 2);
        assert_eq!(
            rejected(ExactPackSet::finalize(
                pair_selection(&mismatched_engine),
                vec![
                    mismatched_engine.root.clone(),
                    mismatched_engine.leaf.clone(),
                ],
            )),
            PackSetError::EngineProtocolMismatch {
                package: mismatched_engine.leaf_coordinate,
                expected: EngineProtocolVersion::new(1),
                actual: EngineProtocolVersion::new(2),
            }
        );

        let (root_catalog, root_interface, root_operation) = interface_fixture_at(1);
        let (leaf_catalog, leaf_interface, leaf_operation) = interface_fixture_at(2);
        let root_validator = ArtifactValidator::new(&root_catalog);
        let leaf_validator = ArtifactValidator::new(&leaf_catalog);
        let root_coordinate = coordinate("test.root");
        let leaf_coordinate = coordinate("test.leaf");
        let leaf = artifact(
            &leaf_validator,
            leaf_coordinate.clone(),
            EngineProtocolVersion::new(1),
            Vec::new(),
            leaf_interface.clone(),
            leaf_operation,
        );
        let root = artifact(
            &root_validator,
            root_coordinate.clone(),
            EngineProtocolVersion::new(1),
            vec![PackDependency::new(
                leaf_coordinate.clone(),
                leaf.export_digest(),
            )],
            root_interface.clone(),
            root_operation,
        );
        let selection = ExactPackageSelection::new(
            root_coordinate.clone(),
            vec![
                selected(root_coordinate, 1, vec![leaf_coordinate.clone()]),
                selected(leaf_coordinate, 2, Vec::new()),
            ],
        );
        assert_eq!(
            rejected(ExactPackSet::finalize(selection, vec![root, leaf])),
            PackSetError::InterfaceConflict {
                interface: root_interface.key().clone(),
                first: Box::new(root_interface),
                second: Box::new(leaf_interface),
            }
        );
    }
}
