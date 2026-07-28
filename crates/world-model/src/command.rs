use core::cmp::Ordering;
use core::fmt;

use world_core::{
    ActorId, CanonicalBytes, CanonicalDomain, CanonicalError, CanonicalWriter, ContentDigest,
    EntityId,
};
use world_defs::{
    BindingName, DefinitionKey, RuntimeDefinitionSet, RuntimeDefinitionSetDigest, ValueKind,
};

use crate::action_opportunity::ActionOpportunityId;

/// Canonical schema version of command-source derivation.
pub const COMMAND_SOURCE_SCHEMA_VERSION: u16 = 1;

/// Canonical schema version of [`CommandEnvelope`] request fingerprints.
pub const COMMAND_REQUEST_SCHEMA_VERSION: u16 = 1;

const SYSTEM_COMMAND_SOURCE_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("system-command-source-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("system command-source identity domain must be valid"),
    };

const COMMAND_REQUEST_DOMAIN: CanonicalDomain = match CanonicalDomain::new("command-request-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("command-request identity domain must be valid"),
};

/// Stable host-owned namespace input for trusted system commands.
///
/// This identity is not itself a [`CommandSource`]. The engine derives the
/// command-ledger namespace through the system-source domain, so the same
/// bytes cannot impersonate an action-opportunity namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemCommandSourceId([u8; 32]);

impl SystemCommandSourceId {
    /// Constructs a host-owned source identity from exact protocol bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the exact host-owned identity bytes.
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

/// Stable identity of a command-producing source or namespace.
///
/// System and actor-action producers use separate canonical derivation
/// domains. Raw construction remains available for durable decoding and
/// low-level protocol fixtures; normal ingress uses the domain-shaped
/// derivation functions.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandSource([u8; 32]);

impl CommandSource {
    /// Constructs a source identity from exact protocol bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Derives the command-ledger namespace for trusted system ingress.
    #[must_use]
    pub fn derive_system(source: SystemCommandSourceId) -> Self {
        Self(derive_command_source(
            SYSTEM_COMMAND_SOURCE_DOMAIN,
            source.as_bytes(),
        ))
    }

    /// Derives the one-shot command-ledger namespace for an action opportunity.
    ///
    /// [`ActionOpportunityId`] is already a canonical hash in its own domain,
    /// so preserving its bytes also preserves its established semantic random
    /// identity. Trusted system sources are hashed through a separate domain.
    #[must_use]
    pub fn derive_action(opportunity: ActionOpportunityId) -> Self {
        Self(opportunity.into_bytes())
    }

    /// Returns the exact source-identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the source identity and returns its bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

fn derive_command_source(domain: CanonicalDomain, owner: &[u8; 32]) -> [u8; 32] {
    let mut writer = CanonicalWriter::new(domain);
    writer.write_u16(COMMAND_SOURCE_SCHEMA_VERSION);
    if writer.write_bytes(owner).is_err() {
        unreachable!("fixed-width command-source owner must fit the canonical protocol");
    }
    ContentDigest::of_canonical(&writer.finish()).into_bytes()
}

impl fmt::Display for CommandSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for CommandSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "CommandSource({self})")
    }
}

/// Source-scoped command sequence identity.
///
/// Sequence monotonicity belongs to the source namespace. Zero is a valid
/// representable sequence value and is not silently reserved here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId(u64);

impl CommandId {
    /// Constructs a command identity from its source-scoped sequence.
    #[must_use]
    pub const fn new(sequence: u64) -> Self {
        Self(sequence)
    }

    /// Returns the source-scoped sequence.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One typed value bound to an action role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandValue {
    /// An actor identity.
    Actor(ActorId),
    /// A general entity identity.
    Entity(EntityId),
}

impl CommandValue {
    /// Returns the definition-level value kind of this concrete value.
    #[must_use]
    pub const fn value_kind(self) -> ValueKind {
        match self {
            Self::Actor(_) => ValueKind::Actor,
            Self::Entity(_) => ValueKind::Entity,
        }
    }

    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Actor(_) => 0,
            Self::Entity(_) => 1,
        }
    }

    const fn identity_bytes(self) -> [u8; 32] {
        match self {
            Self::Actor(actor) => actor.into_bytes(),
            Self::Entity(entity) => entity.into_bytes(),
        }
    }
}

/// One named, typed action binding supplied by a command.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandBinding {
    name: BindingName,
    value: CommandValue,
}

impl CommandBinding {
    /// Binds a concrete value to one definition-owned role name.
    #[must_use]
    pub const fn new(name: BindingName, value: CommandValue) -> Self {
        Self { name, value }
    }

    /// Returns the action binding name.
    #[must_use]
    pub const fn name(&self) -> &BindingName {
        &self.name
    }

    /// Returns the concrete bound value.
    #[must_use]
    pub const fn value(&self) -> CommandValue {
        self.value
    }
}

/// Canonical fingerprint of a command request body.
///
/// The source and every effect-bearing field are included. [`CommandId`] is
/// deliberately omitted because it is the key whose exact reuse this
/// fingerprint distinguishes.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandRequestFingerprint(ContentDigest);

impl CommandRequestFingerprint {
    /// Returns the exact fingerprint bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Consumes the fingerprint and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0.into_bytes()
    }
}

impl fmt::Display for CommandRequestFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for CommandRequestFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "CommandRequestFingerprint({self})")
    }
}

/// Why a command envelope could not be bound to an exact definition set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandEnvelopeError {
    /// The selected action was absent from the exact runtime definition set.
    DefinitionUnavailable {
        /// Missing durable action key.
        action: DefinitionKey,
    },
    /// The same action role was supplied more than once.
    DuplicateBinding {
        /// Reused role name.
        binding: BindingName,
    },
    /// A definition-required action role was not supplied.
    MissingBinding {
        /// Missing role name.
        binding: BindingName,
    },
    /// A supplied role is not declared by the selected action.
    UnexpectedBinding {
        /// Unknown role name.
        binding: BindingName,
    },
    /// A supplied value had a different kind from its declared role.
    BindingKindMismatch {
        /// Mismatched role name.
        binding: BindingName,
        /// Definition-declared kind.
        expected: ValueKind,
        /// Supplied concrete kind.
        actual: ValueKind,
    },
}

impl fmt::Display for CommandEnvelopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefinitionUnavailable { action } => {
                write!(formatter, "action {action} is unavailable")
            }
            Self::DuplicateBinding { binding } => {
                write!(
                    formatter,
                    "command binding {binding} was supplied more than once"
                )
            }
            Self::MissingBinding { binding } => {
                write!(formatter, "command binding {binding} is missing")
            }
            Self::UnexpectedBinding { binding } => {
                write!(
                    formatter,
                    "command binding {binding} is not declared by the action"
                )
            }
            Self::BindingKindMismatch {
                binding,
                expected,
                actual,
            } => write!(
                formatter,
                "command binding {binding} expects {expected:?} but received {actual:?}"
            ),
        }
    }
}

impl std::error::Error for CommandEnvelopeError {}

/// A definition-bound request for runtime action authority.
///
/// Only [`Self::new`] can construct a value, and construction resolves the
/// action through the complete immutable definition set rather than accepting
/// a detached action definition.
///
/// ```compile_fail
/// let _ = world_model::CommandEnvelope {};
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandEnvelope {
    source: CommandSource,
    id: CommandId,
    actor: ActorId,
    definition_set: RuntimeDefinitionSetDigest,
    action: DefinitionKey,
    bindings: Vec<CommandBinding>,
    fingerprint: CommandRequestFingerprint,
}

impl CommandEnvelope {
    /// Resolves, validates, canonicalizes, and fingerprints a command.
    pub fn new(
        definitions: &RuntimeDefinitionSet,
        source: CommandSource,
        id: CommandId,
        actor: ActorId,
        action: DefinitionKey,
        mut bindings: Vec<CommandBinding>,
    ) -> Result<Self, CommandEnvelopeError> {
        let definition = definitions.action(&action).ok_or_else(|| {
            CommandEnvelopeError::DefinitionUnavailable {
                action: action.clone(),
            }
        })?;

        bindings.sort_by(|left, right| left.name.cmp(&right.name));
        if let Some(duplicate) = bindings
            .windows(2)
            .find(|pair| pair[0].name == pair[1].name)
        {
            return Err(CommandEnvelopeError::DuplicateBinding {
                binding: duplicate[1].name.clone(),
            });
        }

        let expected = definition.bindings();
        let mut expected_index = 0;
        let mut supplied_index = 0;
        while expected_index < expected.len() && supplied_index < bindings.len() {
            let declared = &expected[expected_index];
            let supplied = &bindings[supplied_index];
            match supplied.name.cmp(declared.name()) {
                Ordering::Less => {
                    return Err(CommandEnvelopeError::UnexpectedBinding {
                        binding: supplied.name.clone(),
                    });
                }
                Ordering::Greater => {
                    return Err(CommandEnvelopeError::MissingBinding {
                        binding: declared.name().clone(),
                    });
                }
                Ordering::Equal => {
                    let expected_kind = *declared.value_kind();
                    let actual = supplied.value.value_kind();
                    if actual != expected_kind {
                        return Err(CommandEnvelopeError::BindingKindMismatch {
                            binding: supplied.name.clone(),
                            expected: expected_kind,
                            actual,
                        });
                    }
                    expected_index += 1;
                    supplied_index += 1;
                }
            }
        }

        if let Some(declared) = expected.get(expected_index) {
            return Err(CommandEnvelopeError::MissingBinding {
                binding: declared.name().clone(),
            });
        }
        if let Some(supplied) = bindings.get(supplied_index) {
            return Err(CommandEnvelopeError::UnexpectedBinding {
                binding: supplied.name.clone(),
            });
        }

        let definition_set = definitions.digest();
        let fingerprint =
            compute_request_fingerprint(source, definition_set, actor, &action, &bindings);
        Ok(Self {
            source,
            id,
            actor,
            definition_set,
            action,
            bindings,
            fingerprint,
        })
    }

    /// Returns the source namespace.
    #[must_use]
    pub const fn source(&self) -> CommandSource {
        self.source
    }

    /// Returns the source-scoped command identity.
    #[must_use]
    pub const fn id(&self) -> CommandId {
        self.id
    }

    /// Returns the actor requesting action authority.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the exact definition-set identity used for construction.
    #[must_use]
    pub const fn definition_set_digest(&self) -> RuntimeDefinitionSetDigest {
        self.definition_set
    }

    /// Returns the selected durable action key.
    #[must_use]
    pub const fn action(&self) -> &DefinitionKey {
        &self.action
    }

    /// Returns bindings in the action definition's canonical name order.
    #[must_use]
    pub fn bindings(&self) -> &[CommandBinding] {
        &self.bindings
    }

    /// Returns the canonical request fingerprint.
    #[must_use]
    pub const fn fingerprint(&self) -> CommandRequestFingerprint {
        self.fingerprint
    }
}

fn compute_request_fingerprint(
    source: CommandSource,
    definition_set: RuntimeDefinitionSetDigest,
    actor: ActorId,
    action: &DefinitionKey,
    bindings: &[CommandBinding],
) -> CommandRequestFingerprint {
    CommandRequestFingerprint(ContentDigest::of_canonical(&command_request_preimage(
        source.as_bytes(),
        definition_set.as_bytes(),
        actor.as_bytes(),
        action,
        bindings,
    )))
}

fn command_request_preimage(
    source: &[u8; 32],
    definition_set: &[u8; 32],
    actor: &[u8; 32],
    action: &DefinitionKey,
    bindings: &[CommandBinding],
) -> CanonicalBytes {
    let encoded = (|| -> Result<_, CanonicalError> {
        let mut writer = CanonicalWriter::new(COMMAND_REQUEST_DOMAIN);
        writer.write_u16(COMMAND_REQUEST_SCHEMA_VERSION);
        writer.write_bytes(source)?;
        writer.write_bytes(definition_set)?;
        writer.write_bytes(actor)?;
        writer.write_str(action.pack_key().as_str())?;
        writer.write_str(action.local_name().as_str())?;
        writer.write_sequence(bindings, |writer, binding| {
            writer.write_str(binding.name.as_str())?;
            writer.write_discriminant(binding.value.canonical_tag());
            writer.write_bytes(&binding.value.identity_bytes())
        })?;
        Ok(writer.finish())
    })();
    match encoded {
        Ok(bytes) => bytes,
        Err(error) => unreachable!(
            "definition-checked command values must fit the canonical protocol: {error}"
        ),
    }
}

/// Stable reason attached to a rejected command attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StableCommandRejection {
    /// The selected durable definition was unavailable at execution.
    DefinitionUnavailable,
    /// Captured values did not satisfy the action's exact binding contract.
    BindingMismatch,
    /// A declared freshness condition no longer held.
    Stale,
    /// At least one authoritative action requirement evaluated to false.
    RequirementUnsatisfied,
    /// The proposal conflicted with another accepted invariant or resource use.
    Conflict,
    /// Multiple distinct requests introduced the same previously unused
    /// source-scoped command identity in one admission barrier.
    IdCollision,
}

/// Stable result of one newly attempted command.
///
/// Record identity and commit references remain runtime-owned and are not
/// embedded in this model-facing result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandAttemptOutcome {
    /// The command's checked proposal was accepted.
    Accepted,
    /// The command produced no accepted model change for a stable reason.
    Rejected(StableCommandRejection),
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
    use core::fmt::Write;
    use world_defs::{BindingName, LocalDefinitionName, PackKey};

    fn declared<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("command preimage fixture must be valid: {error}"),
        }
    }

    fn hex(bytes: &[u8]) -> String {
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            if write!(&mut encoded, "{byte:02x}").is_err() {
                unreachable!("writing to String cannot fail");
            }
        }
        encoded
    }

    #[test]
    fn system_and_action_sources_are_domain_separated() {
        let owner = [0x11; 32];
        let system = CommandSource::derive_system(SystemCommandSourceId::from_bytes(owner));
        let action = CommandSource::derive_action(ActionOpportunityId::from_bytes(owner));

        assert_eq!(
            system,
            CommandSource::derive_system(SystemCommandSourceId::from_bytes(owner))
        );
        assert_eq!(
            action,
            CommandSource::derive_action(ActionOpportunityId::from_bytes(owner))
        );
        assert_ne!(system, action);
        assert_ne!(system, CommandSource::from_bytes(owner));
        assert_eq!(action, CommandSource::from_bytes(owner));
    }

    #[test]
    fn command_request_preimage_is_byte_complete() {
        let action = DefinitionKey::new(
            declared(PackKey::parse("test.commands")),
            declared(LocalDefinitionName::parse("move-item")),
        );
        let bindings = [
            CommandBinding::new(
                declared(BindingName::parse("actor")),
                CommandValue::Actor(ActorId::from_bytes([0x44; 32])),
            ),
            CommandBinding::new(
                declared(BindingName::parse("item")),
                CommandValue::Entity(EntityId::from_bytes([0x55; 32])),
            ),
        ];

        assert_eq!(
            hex(command_request_preimage(
                &[0x11; 32],
                &[0x22; 32],
                &[0x33; 32],
                &action,
                &bindings,
            )
            .as_bytes()),
            "776f726c642d63616e6f6e6963616c2d76310000000000000012636f6d6d616e642d726571756573742d76310001000000000000002011111111111111111111111111111111111111111111111111111111111111110000000000000020222222222222222222222222222222222222222222222222222222222222222200000000000000203333333333333333333333333333333333333333333333333333333333333333000000000000000d746573742e636f6d6d616e647300000000000000096d6f76652d6974656d000000000000000200000000000000056163746f72000000000000000000000020444444444444444444444444444444444444444444444444444444444444444400000000000000046974656d0000000100000000000000205555555555555555555555555555555555555555555555555555555555555555"
        );
    }
}
