use world_defs::{
    ActionData, ArtifactData, ArtifactEnvelope, ArtifactValidator, DefinitionLinker,
    EngineProtocolVersion, EventData, ExactPackSet, ExactPackageSelection, MAX_PACKAGES_PER_SET,
    PackCoordinate, PackDependency, PackKey, PackManifestData, RuntimeDefinitionSet,
    SelectedPackage, SemanticInterfaceCatalog, SourceSnapshotId, VerifiedPackArtifact,
};

use crate::diagnostic::{CompilationDiagnostic, DiagnosticSet, SourceGraphError};
use crate::source::{CompileRequest, PackSource};

/// Successful output of one complete structured-source compilation.
///
/// Envelopes are ordered by `PackKey`. The definition set is already sealed
/// by the defs-owned exact-set and linker boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compilation {
    envelopes: Vec<ArtifactEnvelope>,
    definitions: RuntimeDefinitionSet,
}

impl Compilation {
    /// Returns emitted artifacts in canonical `PackKey` order.
    #[must_use]
    pub fn envelopes(&self) -> &[ArtifactEnvelope] {
        &self.envelopes
    }

    /// Returns the sealed process-independent runtime definitions.
    #[must_use]
    pub const fn definitions(&self) -> &RuntimeDefinitionSet {
        &self.definitions
    }

    /// Consumes the output into its emitted artifacts and linked definitions.
    #[must_use]
    pub fn into_parts(self) -> (Vec<ArtifactEnvelope>, RuntimeDefinitionSet) {
        (self.envelopes, self.definitions)
    }
}

/// Catalog-bound compiler for programmatic pack source.
#[derive(Clone, Copy, Debug)]
pub struct AuthoringCompiler<'catalog> {
    catalog: &'catalog SemanticInterfaceCatalog,
    artifact_validator: ArtifactValidator<'catalog>,
}

impl<'catalog> AuthoringCompiler<'catalog> {
    /// Binds compilation to one immutable semantic-interface catalog.
    #[must_use]
    pub const fn new(catalog: &'catalog SemanticInterfaceCatalog) -> Self {
        Self {
            catalog,
            artifact_validator: ArtifactValidator::new(catalog),
        }
    }

    /// Compiles one exact source graph or returns one deterministic nonempty
    /// diagnostic set.
    ///
    /// Artifact data is validated and encoded once. The compiler then passes
    /// sealed artifacts through exact-set finalization and definition linking
    /// without recreating either proof.
    pub fn compile(&self, request: CompileRequest) -> Result<Compilation, DiagnosticSet> {
        self.compile_one(request).map_err(DiagnosticSet::single)
    }

    fn compile_one(&self, request: CompileRequest) -> Result<Compilation, CompilationDiagnostic> {
        let graph = SourceGraph::normalize(request).map_err(CompilationDiagnostic::SourceGraph)?;
        let selection = graph.exact_selection();
        let mut artifacts = Vec::with_capacity(graph.sources.len());

        for &source_index in &graph.dependency_first {
            let source = &graph.sources[source_index];
            let dependencies = source
                .dependencies
                .iter()
                .map(|coordinate| {
                    let dependency = match find_compiled_artifact(&artifacts, coordinate.pack_key())
                    {
                        Some(dependency) => dependency,
                        None => unreachable!(
                            "validated dependency-first graph must compile dependencies first"
                        ),
                    };
                    PackDependency::new(coordinate.clone(), dependency.export_digest())
                })
                .collect();

            let interfaces = self.interface_references(source)?;
            let manifest = PackManifestData::new(
                source.engine_protocol,
                source.coordinate.clone(),
                dependencies,
            );
            let data = ArtifactData::new(
                manifest,
                interfaces,
                source.actions.clone(),
                source.events.clone(),
            );
            let artifact = self.artifact_validator.validate(data).map_err(|error| {
                CompilationDiagnostic::Artifact {
                    package: source.coordinate.clone(),
                    error: Box::new(error),
                }
            })?;
            artifacts.push(artifact);
        }

        let exact = match ExactPackSet::finalize(selection, artifacts) {
            Ok(exact) => exact,
            Err(error) => {
                unreachable!("compiler-generated exact package set must finalize: {error}")
            }
        };
        let definitions = DefinitionLinker::link(exact)
            .map_err(|error| CompilationDiagnostic::Link(Box::new(error)))?;
        let envelopes = definitions
            .artifacts()
            .iter()
            .map(|artifact| artifact.envelope().clone())
            .collect();

        Ok(Compilation {
            envelopes,
            definitions,
        })
    }

    fn interface_references(
        &self,
        source: &NormalizedSource,
    ) -> Result<Vec<world_defs::SemanticInterfaceReference>, CompilationDiagnostic> {
        let mut keys = source
            .actions
            .iter()
            .flat_map(|action| {
                action
                    .requirements()
                    .iter()
                    .map(|requirement| requirement.call().interface().clone())
                    .chain(
                        action
                            .effects()
                            .iter()
                            .map(|effect| effect.call().interface().clone()),
                    )
            })
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();

        keys.into_iter()
            .map(|key| {
                self.catalog
                    .get_by_key(&key)
                    .map(|descriptor| descriptor.reference())
                    .ok_or_else(|| CompilationDiagnostic::MissingInterface {
                        package: source.coordinate.clone(),
                        interface: key,
                    })
            })
            .collect()
    }
}

struct SourceGraph {
    root: PackCoordinate,
    sources: Vec<NormalizedSource>,
    dependency_first: Vec<usize>,
}

impl SourceGraph {
    fn normalize(request: CompileRequest) -> Result<Self, SourceGraphError> {
        let (root, sources) = request.into_parts();
        if sources.len() > MAX_PACKAGES_PER_SET {
            return Err(SourceGraphError::TooManyPackages {
                actual: sources.len(),
                maximum: MAX_PACKAGES_PER_SET,
            });
        }

        let mut sources = sources
            .into_iter()
            .map(NormalizedSource::from_source)
            .collect::<Result<Vec<_>, SourceGraphError>>()?;
        sources.sort_by(|left, right| {
            left.coordinate
                .pack_key()
                .cmp(right.coordinate.pack_key())
                .then_with(|| left.coordinate.cmp(&right.coordinate))
        });
        check_unique_sources(&sources)?;

        let root_index = match find_source_index(&sources, root.pack_key()) {
            Some(index) if sources[index].coordinate == root => index,
            Some(index) => {
                return Err(SourceGraphError::RootCoordinateMismatch {
                    requested: Box::new(root),
                    selected: Box::new(sources[index].coordinate.clone()),
                });
            }
            None => return Err(SourceGraphError::MissingRoot { root }),
        };
        let engine_protocol = sources[root_index].engine_protocol;
        check_engine_protocol(engine_protocol, &sources)?;
        check_dependency_closure(&sources)?;

        let mut states = vec![VisitState::Unvisited; sources.len()];
        let mut dependency_first = Vec::with_capacity(sources.len());
        visit_source(root_index, &sources, &mut states, &mut dependency_first)?;
        for (source, state) in sources.iter().zip(states) {
            if state == VisitState::Unvisited {
                return Err(SourceGraphError::UnreachablePackage {
                    package: source.coordinate.clone(),
                });
            }
        }

        Ok(Self {
            root,
            sources,
            dependency_first,
        })
    }

    fn exact_selection(&self) -> ExactPackageSelection {
        ExactPackageSelection::new(
            self.root.clone(),
            self.sources
                .iter()
                .map(|source| {
                    SelectedPackage::new(
                        source.coordinate.clone(),
                        source.source_snapshot,
                        source.dependencies.clone(),
                    )
                })
                .collect(),
        )
    }
}

struct NormalizedSource {
    source_snapshot: SourceSnapshotId,
    engine_protocol: EngineProtocolVersion,
    coordinate: PackCoordinate,
    dependencies: Vec<PackCoordinate>,
    actions: Vec<ActionData>,
    events: Vec<EventData>,
}

impl NormalizedSource {
    fn from_source(source: PackSource) -> Result<Self, SourceGraphError> {
        let (source_snapshot, engine_protocol, coordinate, mut dependencies, actions, events) =
            source.into_parts();
        dependencies.sort_by(|left, right| {
            left.pack_key()
                .cmp(right.pack_key())
                .then_with(|| left.cmp(right))
        });

        for adjacent in dependencies.windows(2) {
            if adjacent[0].pack_key() != adjacent[1].pack_key() {
                continue;
            }
            if adjacent[0] == adjacent[1] {
                return Err(SourceGraphError::DuplicateDependency {
                    package: Box::new(coordinate),
                    dependency: Box::new(adjacent[0].clone()),
                });
            }
            return Err(SourceGraphError::ConflictingDependencies {
                package: Box::new(coordinate),
                dependency: adjacent[0].pack_key().clone(),
                first: Box::new(adjacent[0].clone()),
                second: Box::new(adjacent[1].clone()),
            });
        }

        Ok(Self {
            source_snapshot,
            engine_protocol,
            coordinate,
            dependencies,
            actions,
            events,
        })
    }
}

fn check_unique_sources(sources: &[NormalizedSource]) -> Result<(), SourceGraphError> {
    for adjacent in sources.windows(2) {
        if adjacent[0].coordinate.pack_key() != adjacent[1].coordinate.pack_key() {
            continue;
        }
        if adjacent[0].coordinate == adjacent[1].coordinate {
            return Err(SourceGraphError::DuplicatePackage {
                coordinate: adjacent[0].coordinate.clone(),
            });
        }
        return Err(SourceGraphError::ConflictingPackages {
            pack: adjacent[0].coordinate.pack_key().clone(),
            first: Box::new(adjacent[0].coordinate.clone()),
            second: Box::new(adjacent[1].coordinate.clone()),
        });
    }
    Ok(())
}

fn check_engine_protocol(
    expected: EngineProtocolVersion,
    sources: &[NormalizedSource],
) -> Result<(), SourceGraphError> {
    for source in sources {
        if source.engine_protocol != expected {
            return Err(SourceGraphError::EngineProtocolMismatch {
                package: source.coordinate.clone(),
                expected,
                actual: source.engine_protocol,
            });
        }
    }
    Ok(())
}

fn check_dependency_closure(sources: &[NormalizedSource]) -> Result<(), SourceGraphError> {
    for source in sources {
        for dependency in &source.dependencies {
            let Some(index) = find_source_index(sources, dependency.pack_key()) else {
                return Err(SourceGraphError::MissingDependency {
                    package: Box::new(source.coordinate.clone()),
                    dependency: Box::new(dependency.clone()),
                });
            };
            if sources[index].coordinate != *dependency {
                return Err(SourceGraphError::DependencyCoordinateMismatch {
                    package: Box::new(source.coordinate.clone()),
                    requested: Box::new(dependency.clone()),
                    selected: Box::new(sources[index].coordinate.clone()),
                });
            }
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

fn visit_source(
    index: usize,
    sources: &[NormalizedSource],
    states: &mut [VisitState],
    dependency_first: &mut Vec<usize>,
) -> Result<(), SourceGraphError> {
    match states[index] {
        VisitState::Visited => return Ok(()),
        VisitState::Visiting => {
            return Err(SourceGraphError::DependencyCycle {
                package: sources[index].coordinate.clone(),
            });
        }
        VisitState::Unvisited => {}
    }

    states[index] = VisitState::Visiting;
    for dependency in &sources[index].dependencies {
        let dependency_index = match find_source_index(sources, dependency.pack_key()) {
            Some(index) => index,
            None => unreachable!("validated source graph must contain every dependency"),
        };
        visit_source(dependency_index, sources, states, dependency_first)?;
    }
    states[index] = VisitState::Visited;
    dependency_first.push(index);
    Ok(())
}

fn find_source_index(sources: &[NormalizedSource], key: &PackKey) -> Option<usize> {
    sources
        .binary_search_by(|source| source.coordinate.pack_key().cmp(key))
        .ok()
}

fn find_compiled_artifact<'artifacts>(
    artifacts: &'artifacts [VerifiedPackArtifact],
    key: &PackKey,
) -> Option<&'artifacts VerifiedPackArtifact> {
    artifacts
        .iter()
        .find(|artifact| artifact.coordinate().pack_key() == key)
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use world_defs::{
        ActionBindingData, ActionData, BindingName, DefinitionKey, EffectCallData, EventData,
        EventEmissionData, EventFieldBindingData, EventFieldData, EventFieldName, InterfaceVersion,
        LocalDefinitionName, OperationCallData, OperationKind, OperationName, OperationParameter,
        PackVersion, ParameterName, SemanticInterfaceDescriptor, SemanticOperationDescriptor,
        ValueKind,
    };

    use super::*;

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("fixture must be valid: {error}"),
        }
    }

    fn coordinate(name: &str) -> PackCoordinate {
        PackCoordinate::new(valid(PackKey::parse(name)), PackVersion::new(1, 0, 0))
    }

    fn interface_fixture() -> (
        SemanticInterfaceCatalog,
        world_defs::SemanticInterfaceKey,
        OperationName,
    ) {
        let key = valid(world_defs::SemanticInterfaceKey::parse("test.interface"));
        let operation_name = valid(OperationName::parse("apply"));
        let operation = valid(SemanticOperationDescriptor::new(
            operation_name.clone(),
            OperationKind::Effect,
            vec![OperationParameter::new(
                valid(ParameterName::parse("subject")),
                ValueKind::Entity,
            )],
        ));
        let descriptor = valid(SemanticInterfaceDescriptor::new(
            key.clone(),
            valid(InterfaceVersion::new(1)),
            vec![operation],
        ));
        (
            valid(SemanticInterfaceCatalog::new(vec![descriptor])),
            key,
            operation_name,
        )
    }

    fn pack_source(
        coordinate: PackCoordinate,
        snapshot_byte: u8,
        dependencies: Vec<PackCoordinate>,
        interface: world_defs::SemanticInterfaceKey,
        operation: OperationName,
    ) -> PackSource {
        let binding = valid(BindingName::parse("subject"));
        let event_name = valid(LocalDefinitionName::parse("changed"));
        let field_name = valid(EventFieldName::parse("subject"));
        let event = EventData::new(
            event_name.clone(),
            vec![EventFieldData::new(field_name.clone(), ValueKind::Entity)],
        );
        let action = ActionData::new(
            valid(LocalDefinitionName::parse("change")),
            vec![ActionBindingData::new(binding.clone(), ValueKind::Entity)],
            Vec::new(),
            vec![EffectCallData::new(OperationCallData::new(
                interface,
                operation,
                vec![binding.clone()],
            ))],
            vec![EventEmissionData::new(
                DefinitionKey::new(coordinate.pack_key().clone(), event_name),
                vec![EventFieldBindingData::new(field_name, binding)],
            )],
        );

        PackSource::new(
            SourceSnapshotId::from_bytes([snapshot_byte; 32]),
            EngineProtocolVersion::new(1),
            coordinate,
            dependencies,
            vec![action],
            vec![event],
        )
    }

    #[test]
    fn compilation_is_order_independent_and_keeps_source_identity_in_the_lock() {
        let (catalog, interface, operation) = interface_fixture();
        let compiler = AuthoringCompiler::new(&catalog);
        let leaf_coordinate = coordinate("test.leaf");
        let root_coordinate = coordinate("test.root");

        let leaf = pack_source(
            leaf_coordinate.clone(),
            1,
            Vec::new(),
            interface.clone(),
            operation.clone(),
        );
        let root = pack_source(
            root_coordinate.clone(),
            2,
            vec![leaf_coordinate.clone()],
            interface.clone(),
            operation.clone(),
        );
        let forward = valid(compiler.compile(CompileRequest::new(
            root_coordinate.clone(),
            vec![root.clone(), leaf.clone()],
        )));
        let reverse = valid(compiler.compile(CompileRequest::new(
            root_coordinate.clone(),
            vec![leaf.clone(), root.clone()],
        )));

        assert_eq!(forward, reverse);
        assert_eq!(forward.envelopes().len(), 2);
        assert_eq!(
            forward.definitions().artifacts()[0].coordinate(),
            &leaf_coordinate
        );
        assert_eq!(
            forward.definitions().artifacts()[1].coordinate(),
            &root_coordinate
        );

        let changed_root = pack_source(
            root_coordinate.clone(),
            9,
            vec![leaf_coordinate],
            interface,
            operation,
        );
        let changed = valid(compiler.compile(CompileRequest::new(
            root_coordinate,
            vec![changed_root, leaf],
        )));

        assert_eq!(forward.envelopes(), changed.envelopes());
        assert_eq!(
            forward.definitions().digest(),
            changed.definitions().digest()
        );
        assert_ne!(
            forward.definitions().lock().digest(),
            changed.definitions().lock().digest()
        );
    }

    #[test]
    fn graph_and_catalog_failures_return_one_deterministic_diagnostic() {
        let (_, interface, operation) = interface_fixture();
        let root_coordinate = coordinate("test.root");
        let source = pack_source(
            root_coordinate.clone(),
            1,
            Vec::new(),
            interface.clone(),
            operation,
        );
        let empty_catalog = SemanticInterfaceCatalog::default();
        let missing_interface = AuthoringCompiler::new(&empty_catalog)
            .compile(CompileRequest::new(root_coordinate.clone(), vec![source]));
        assert!(matches!(
            missing_interface,
            Err(ref diagnostics)
                if matches!(
                    diagnostics.iter().next(),
                    Some(CompilationDiagnostic::MissingInterface {
                        package,
                        interface: missing,
                    }) if package == &root_coordinate && missing == &interface
                ) && diagnostics.len() == 1
        ));

        let other_coordinate = coordinate("test.other");
        let root = PackSource::new(
            SourceSnapshotId::from_bytes([1; 32]),
            EngineProtocolVersion::new(1),
            root_coordinate.clone(),
            vec![other_coordinate.clone()],
            Vec::new(),
            Vec::new(),
        );
        let other = PackSource::new(
            SourceSnapshotId::from_bytes([2; 32]),
            EngineProtocolVersion::new(1),
            other_coordinate,
            vec![root_coordinate.clone()],
            Vec::new(),
            Vec::new(),
        );
        let cycle = AuthoringCompiler::new(&empty_catalog)
            .compile(CompileRequest::new(root_coordinate, vec![other, root]));
        assert!(matches!(
            cycle,
            Err(ref diagnostics)
                if matches!(
                    diagnostics.iter().next(),
                    Some(CompilationDiagnostic::SourceGraph(
                        SourceGraphError::DependencyCycle { .. }
                    ))
                ) && diagnostics.len() == 1
        ));
    }
}
