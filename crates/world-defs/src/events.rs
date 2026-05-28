use std::collections::BTreeSet;
use std::fmt;

use world_core::VersionAnchor;

use crate::error::DefinitionError;
use crate::keys::{EventKind, RoleName};

/// Checked event record contract item.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventRecordSpec {
    kind: EventKind,
    roles: BTreeSet<RoleName>,
    version: VersionAnchor,
}

impl EventRecordSpec {
    /// Creates an event record spec when at least one role is declared.
    pub fn new(
        kind: EventKind,
        roles: impl IntoIterator<Item = RoleName>,
        version: VersionAnchor,
    ) -> Result<Self, DefinitionError> {
        let roles = roles.into_iter().collect::<BTreeSet<_>>();

        if roles.is_empty() {
            Err(DefinitionError::EmptyItemField {
                type_name: "EventRecordSpec",
                field: "roles",
            })
        } else {
            Ok(Self {
                kind,
                roles,
                version,
            })
        }
    }

    /// Returns the event record family.
    pub fn kind(&self) -> &EventKind {
        &self.kind
    }

    /// Returns roles that must be present in the event record.
    pub fn roles(&self) -> impl Iterator<Item = &RoleName> {
        self.roles.iter()
    }

    /// Returns the event record schema/content version anchor.
    pub fn version(&self) -> VersionAnchor {
        self.version
    }
}

impl fmt::Display for EventRecordSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}@{}(", self.kind, self.version.get())?;
        for (index, role) in self.roles.iter().enumerate() {
            if index > 0 {
                formatter.write_str(",")?;
            }
            write!(formatter, "{role}")?;
        }
        formatter.write_str(")")
    }
}

/// Event records a definition requires or permits from its effect programs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventContract {
    required_events: BTreeSet<EventRecordSpec>,
    allowed_events: BTreeSet<EventRecordSpec>,
}

impl EventContract {
    /// Creates an event contract from required event specs.
    pub fn new(required_events: impl IntoIterator<Item = EventRecordSpec>) -> Self {
        Self {
            required_events: required_events.into_iter().collect(),
            allowed_events: BTreeSet::new(),
        }
    }

    /// Creates an event contract from required and optional/conditional event specs.
    pub fn with_allowed(
        required_events: impl IntoIterator<Item = EventRecordSpec>,
        allowed_events: impl IntoIterator<Item = EventRecordSpec>,
    ) -> Self {
        let required_events = required_events.into_iter().collect::<BTreeSet<_>>();
        let mut allowed_events = allowed_events.into_iter().collect::<BTreeSet<_>>();
        for event in &required_events {
            allowed_events.remove(event);
        }

        Self {
            required_events,
            allowed_events,
        }
    }

    /// Returns true when the contract declares no event specs.
    pub fn is_empty(&self) -> bool {
        self.required_events.is_empty() && self.allowed_events.is_empty()
    }

    /// Returns the required event specs.
    pub fn required_events(&self) -> impl Iterator<Item = &EventRecordSpec> {
        self.required_events.iter()
    }

    /// Returns optional, conditional, or failure-path event specs this definition may emit.
    pub fn allowed_events(&self) -> impl Iterator<Item = &EventRecordSpec> {
        self.allowed_events.iter()
    }

    /// Returns true when this contract requires the event spec.
    pub fn requires_event(&self, event: &EventRecordSpec) -> bool {
        self.required_events.contains(event)
    }

    /// Returns true when this contract permits an emitted event spec.
    pub fn permits_event(&self, event: &EventRecordSpec) -> bool {
        self.required_events.contains(event) || self.allowed_events.contains(event)
    }

    /// Returns role references used by required and allowed event specs.
    pub fn role_refs(&self) -> impl Iterator<Item = &RoleName> {
        self.required_events
            .iter()
            .chain(self.allowed_events.iter())
            .flat_map(EventRecordSpec::roles)
    }
}
