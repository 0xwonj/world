//! Programmatic source values accepted by the authoring compiler.

use world_defs::{ActionData, EngineProtocolVersion, EventData, PackCoordinate, SourceSnapshotId};

/// One exact pack source supplied to a compilation.
///
/// This value records resolver output and structured definition input. It does
/// not claim that the complete source graph is closed, acyclic, or internally
/// consistent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackSource {
    source_snapshot: SourceSnapshotId,
    engine_protocol: EngineProtocolVersion,
    coordinate: PackCoordinate,
    dependencies: Vec<PackCoordinate>,
    actions: Vec<ActionData>,
    events: Vec<EventData>,
}

impl PackSource {
    /// Creates structured source for one exact pack coordinate.
    #[must_use]
    pub fn new(
        source_snapshot: SourceSnapshotId,
        engine_protocol: EngineProtocolVersion,
        coordinate: PackCoordinate,
        dependencies: Vec<PackCoordinate>,
        actions: Vec<ActionData>,
        events: Vec<EventData>,
    ) -> Self {
        Self {
            source_snapshot,
            engine_protocol,
            coordinate,
            dependencies,
            actions,
            events,
        }
    }

    /// Returns the owner-supplied exact source identity.
    #[must_use]
    pub const fn source_snapshot(&self) -> SourceSnapshotId {
        self.source_snapshot
    }

    /// Returns the engine protocol required by this source.
    #[must_use]
    pub const fn engine_protocol(&self) -> EngineProtocolVersion {
        self.engine_protocol
    }

    /// Returns the source's exact pack coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &PackCoordinate {
        &self.coordinate
    }

    /// Returns exact direct dependency coordinates in resolver-supplied order.
    #[must_use]
    pub fn dependencies(&self) -> &[PackCoordinate] {
        &self.dependencies
    }

    /// Returns structured action definitions in source order.
    #[must_use]
    pub fn actions(&self) -> &[ActionData] {
        &self.actions
    }

    /// Returns structured physical-event definitions in source order.
    #[must_use]
    pub fn events(&self) -> &[EventData] {
        &self.events
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        SourceSnapshotId,
        EngineProtocolVersion,
        PackCoordinate,
        Vec<PackCoordinate>,
        Vec<ActionData>,
        Vec<EventData>,
    ) {
        (
            self.source_snapshot,
            self.engine_protocol,
            self.coordinate,
            self.dependencies,
            self.actions,
            self.events,
        )
    }
}

/// One exact root and the complete structured source input offered for
/// compilation.
///
/// Construction is intentionally permissive. The compiler validates package
/// uniqueness, exact dependency closure, reachability, and acyclicity before
/// compiling any definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileRequest {
    root: PackCoordinate,
    sources: Vec<PackSource>,
}

impl CompileRequest {
    /// Creates a compilation request from resolver-selected source values.
    #[must_use]
    pub fn new(root: PackCoordinate, sources: Vec<PackSource>) -> Self {
        Self { root, sources }
    }

    /// Returns the requested exact root coordinate.
    #[must_use]
    pub const fn root(&self) -> &PackCoordinate {
        &self.root
    }

    /// Returns source packages in resolver-supplied order.
    #[must_use]
    pub fn sources(&self) -> &[PackSource] {
        &self.sources
    }

    pub(crate) fn into_parts(self) -> (PackCoordinate, Vec<PackSource>) {
        (self.root, self.sources)
    }
}

#[cfg(test)]
mod tests {
    use world_defs::{KeyError, PackKey, PackVersion};

    use super::*;

    #[test]
    fn source_values_preserve_exact_resolver_input() -> Result<(), KeyError> {
        let root = PackCoordinate::new(PackKey::parse("world.root")?, PackVersion::new(1, 0, 0));
        let dependency =
            PackCoordinate::new(PackKey::parse("world.items")?, PackVersion::new(2, 3, 4));
        let snapshot = SourceSnapshotId::from_bytes([7; 32]);
        let source = PackSource::new(
            snapshot,
            EngineProtocolVersion::new(1),
            root.clone(),
            vec![dependency.clone()],
            Vec::new(),
            Vec::new(),
        );
        let request = CompileRequest::new(root.clone(), vec![source]);

        assert_eq!(request.root(), &root);
        assert_eq!(request.sources().len(), 1);
        assert_eq!(request.sources()[0].source_snapshot(), snapshot);
        assert_eq!(request.sources()[0].dependencies(), &[dependency]);
        Ok(())
    }
}
