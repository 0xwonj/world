//! Deterministic diagnostics for structured pack compilation.

use core::fmt;
use std::slice;

use world_defs::{
    ArtifactError, EngineProtocolVersion, LinkError, PackCoordinate, PackKey, SemanticInterfaceKey,
};

/// Why structured source packages do not form one exact compilation graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceGraphError {
    /// The compilation request exceeded the exact package limit.
    TooManyPackages {
        /// Number of supplied source packages.
        actual: usize,
        /// Maximum accepted source packages.
        maximum: usize,
    },
    /// The same exact package coordinate appeared more than once.
    DuplicatePackage {
        /// Repeated coordinate.
        coordinate: PackCoordinate,
    },
    /// Two source packages used one durable pack key with different versions.
    ConflictingPackages {
        /// Conflicting durable pack key.
        pack: PackKey,
        /// First exact coordinate.
        first: Box<PackCoordinate>,
        /// Second exact coordinate.
        second: Box<PackCoordinate>,
    },
    /// One source package repeated the same exact direct dependency.
    DuplicateDependency {
        /// Package declaring the edge.
        package: Box<PackCoordinate>,
        /// Repeated dependency coordinate.
        dependency: Box<PackCoordinate>,
    },
    /// One source package declared two versions of the same direct dependency.
    ConflictingDependencies {
        /// Package declaring the conflicting edges.
        package: Box<PackCoordinate>,
        /// Conflicting durable dependency key.
        dependency: PackKey,
        /// First exact dependency coordinate.
        first: Box<PackCoordinate>,
        /// Second exact dependency coordinate.
        second: Box<PackCoordinate>,
    },
    /// No source package has the requested root key.
    MissingRoot {
        /// Requested exact root.
        root: PackCoordinate,
    },
    /// The source selected for the root key has a different exact coordinate.
    RootCoordinateMismatch {
        /// Requested exact root.
        requested: Box<PackCoordinate>,
        /// Source coordinate selected for the same durable key.
        selected: Box<PackCoordinate>,
    },
    /// A declared direct dependency has no supplied source package.
    MissingDependency {
        /// Package declaring the dependency.
        package: Box<PackCoordinate>,
        /// Missing exact dependency.
        dependency: Box<PackCoordinate>,
    },
    /// A dependency key is present at a different exact coordinate.
    DependencyCoordinateMismatch {
        /// Package declaring the dependency.
        package: Box<PackCoordinate>,
        /// Exact dependency requested by the package.
        requested: Box<PackCoordinate>,
        /// Supplied source coordinate for the same durable key.
        selected: Box<PackCoordinate>,
    },
    /// One source package requires a different engine protocol from the root.
    EngineProtocolMismatch {
        /// Package with the conflicting requirement.
        package: PackCoordinate,
        /// Root package protocol.
        expected: EngineProtocolVersion,
        /// Conflicting package protocol.
        actual: EngineProtocolVersion,
    },
    /// The exact source dependency graph contains a cycle.
    DependencyCycle {
        /// Package reached while already being visited.
        package: PackCoordinate,
    },
    /// A supplied source package is unreachable from the exact root.
    UnreachablePackage {
        /// Unreachable package.
        package: PackCoordinate,
    },
}

impl fmt::Display for SourceGraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyPackages { actual, maximum } => write!(
                formatter,
                "compilation contains {actual} source packages; the maximum is {maximum}"
            ),
            Self::DuplicatePackage { coordinate } => {
                write!(formatter, "compilation repeats source package {coordinate}")
            }
            Self::ConflictingPackages {
                pack,
                first,
                second,
            } => write!(
                formatter,
                "compilation has conflicting coordinates for {pack}: {first} and {second}"
            ),
            Self::DuplicateDependency {
                package,
                dependency,
            } => write!(
                formatter,
                "source package {package} repeats dependency {dependency}"
            ),
            Self::ConflictingDependencies {
                package,
                dependency,
                first,
                second,
            } => write!(
                formatter,
                "source package {package} has conflicting coordinates for dependency \
                 {dependency}: {first} and {second}"
            ),
            Self::MissingRoot { root } => {
                write!(formatter, "requested source root {root} is missing")
            }
            Self::RootCoordinateMismatch {
                requested,
                selected,
            } => write!(
                formatter,
                "requested source root {requested} differs from supplied coordinate {selected}"
            ),
            Self::MissingDependency {
                package,
                dependency,
            } => write!(
                formatter,
                "source package {package} requires missing dependency {dependency}"
            ),
            Self::DependencyCoordinateMismatch {
                package,
                requested,
                selected,
            } => write!(
                formatter,
                "source package {package} requires dependency {requested}, but {selected} was supplied"
            ),
            Self::EngineProtocolMismatch {
                package,
                expected,
                actual,
            } => write!(
                formatter,
                "source package {package} requires engine protocol {actual}; root requires {expected}"
            ),
            Self::DependencyCycle { package } => {
                write!(formatter, "source dependency cycle reaches {package}")
            }
            Self::UnreachablePackage { package } => write!(
                formatter,
                "source package {package} is unreachable from the requested root"
            ),
        }
    }
}

impl std::error::Error for SourceGraphError {}

/// One concrete failure produced while compiling structured pack source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompilationDiagnostic {
    /// Exact source packages do not form a valid compilation graph.
    SourceGraph(SourceGraphError),
    /// A source package calls an interface absent from the supplied catalog.
    MissingInterface {
        /// Package containing the unresolved call.
        package: PackCoordinate,
        /// Missing semantic-interface key.
        interface: SemanticInterfaceKey,
    },
    /// One package failed defs-owned artifact validation.
    Artifact {
        /// Package being compiled.
        package: PackCoordinate,
        /// Concrete artifact validation failure.
        error: Box<ArtifactError>,
    },
    /// The exact package set could not be linked.
    Link(Box<LinkError>),
}

impl fmt::Display for CompilationDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceGraph(source) => fmt::Display::fmt(source, formatter),
            Self::MissingInterface { package, interface } => write!(
                formatter,
                "source package {package} requires missing semantic interface {interface}"
            ),
            Self::Artifact { package, error } => {
                write!(formatter, "source package {package} is invalid: {error}")
            }
            Self::Link(source) => {
                write!(formatter, "runtime definitions cannot be linked: {source}")
            }
        }
    }
}

impl std::error::Error for CompilationDiagnostic {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SourceGraph(source) => Some(source),
            Self::MissingInterface { .. } => None,
            Self::Artifact { error, .. } => Some(error.as_ref()),
            Self::Link(source) => Some(source.as_ref()),
        }
    }
}

/// A deterministic, nonempty set of compilation failures.
///
/// W2 compilation stops at the first owner boundary that fails, so the initial
/// implementation constructs a set from one concrete diagnostic. The
/// collection shape leaves room for deterministic multi-error reporting
/// without introducing warnings, spans, or a general diagnostic framework.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticSet {
    diagnostics: Vec<CompilationDiagnostic>,
}

impl DiagnosticSet {
    pub(crate) fn single(diagnostic: CompilationDiagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }

    /// Returns the number of diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    /// Returns whether this set is empty.
    ///
    /// A value produced by this crate always returns `false`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Iterates over diagnostics in deterministic reporting order.
    pub fn iter(&self) -> slice::Iter<'_, CompilationDiagnostic> {
        self.diagnostics.iter()
    }
}

impl<'diagnostic> IntoIterator for &'diagnostic DiagnosticSet {
    type Item = &'diagnostic CompilationDiagnostic;
    type IntoIter = slice::Iter<'diagnostic, CompilationDiagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl fmt::Display for DiagnosticSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index != 0 {
                formatter.write_str("\n")?;
            }
            fmt::Display::fmt(diagnostic, formatter)?;
        }
        Ok(())
    }
}

impl std::error::Error for DiagnosticSet {}

#[cfg(test)]
mod tests {
    use world_defs::{KeyError, PackKey, PackVersion};

    use super::*;

    #[test]
    fn diagnostic_set_is_nonempty_and_read_only() -> Result<(), KeyError> {
        let package =
            PackCoordinate::new(PackKey::parse("world.standard")?, PackVersion::new(1, 0, 0));
        let interface = SemanticInterfaceKey::parse("world.standard.transfer")?;
        let diagnostic = CompilationDiagnostic::MissingInterface { package, interface };
        let diagnostics = DiagnosticSet::single(diagnostic.clone());

        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics.iter().next(), Some(&diagnostic));
        assert_eq!((&diagnostics).into_iter().count(), 1);
        Ok(())
    }

    #[test]
    fn source_graph_diagnostics_preserve_exact_coordinates() -> Result<(), KeyError> {
        let first = PackCoordinate::new(PackKey::parse("world.items")?, PackVersion::new(1, 0, 0));
        let second = PackCoordinate::new(PackKey::parse("world.items")?, PackVersion::new(2, 0, 0));
        let error = SourceGraphError::ConflictingPackages {
            pack: first.pack_key().clone(),
            first: Box::new(first.clone()),
            second: Box::new(second.clone()),
        };

        assert_eq!(
            error.to_string(),
            format!(
                "compilation has conflicting coordinates for world.items: {first} and {second}"
            )
        );
        Ok(())
    }
}
