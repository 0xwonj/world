//! Trusted executable semantics for the pure `world-standard` vocabulary.
//!
//! This crate owns statically linked implementations only. It receives bounded
//! typed inputs from `world-runtime` and has no world, repository, scheduler,
//! record, or publication capability.

use world_core::{CanonicalDomain, CanonicalWriter, ContentDigest};
use world_defs::OperationName;
use world_model::ContainmentTransferReadView;
use world_runtime::{
    ContainmentTransferEvaluation, ContainmentTransferImplementation, ContainmentTransferInput,
    RelocationActionImplementation, SemanticImplementationId,
};

const IMPLEMENTATION_SCHEMA_VERSION: u16 = 1;
const TRANSFER_IMPLEMENTATION_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("standard-transfer-implementation-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("standard implementation identity domain must be valid"),
    };
const RELOCATION_IMPLEMENTATION_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("standard-relocation-implementation-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("standard relocation implementation identity domain must be valid"),
    };

/// Constructs the trusted implementation of `world.standard.transfer`.
#[must_use]
pub fn standard_transfer_implementation() -> ContainmentTransferImplementation {
    let descriptor = world_standard::transfer_interface_descriptor();
    let implementation = standard_implementation_id(TRANSFER_IMPLEMENTATION_DOMAIN, &descriptor);
    match ContainmentTransferImplementation::new(descriptor, implementation, evaluate_transfer) {
        Ok(implementation) => implementation,
        Err(_) => panic!("standard transfer declaration must satisfy its runtime contract"),
    }
}

/// Constructs the trusted operation-role binding for
/// `world.standard.relocation`.
#[must_use]
pub fn standard_relocation_implementation() -> RelocationActionImplementation {
    let descriptor = world_standard::relocation_interface_descriptor();
    let implementation = standard_implementation_id(RELOCATION_IMPLEMENTATION_DOMAIN, &descriptor);
    let start = operation_name(world_standard::start_relocation_action_key());
    let pause = operation_name(world_standard::pause_relocation_action_key());
    let resume = operation_name(world_standard::resume_relocation_action_key());
    match RelocationActionImplementation::new(descriptor, implementation, start, pause, resume) {
        Ok(implementation) => implementation,
        Err(_) => panic!("standard relocation declaration must satisfy its runtime contract"),
    }
}

fn operation_name(action: world_defs::DefinitionKey) -> OperationName {
    match OperationName::parse(action.local_name().as_str()) {
        Ok(operation) => operation,
        Err(_) => unreachable!("checked standard action name must be an operation name"),
    }
}

fn standard_implementation_id(
    domain: CanonicalDomain,
    descriptor: &world_defs::SemanticInterfaceDescriptor,
) -> SemanticImplementationId {
    let mut writer = CanonicalWriter::new(domain);
    writer.write_u16(IMPLEMENTATION_SCHEMA_VERSION);
    if writer.write_str(descriptor.key().as_str()).is_err()
        || writer.write_bytes(descriptor.digest().as_bytes()).is_err()
    {
        unreachable!("checked standard descriptor must fit the canonical protocol");
    }
    SemanticImplementationId::from_bytes(ContentDigest::of_canonical(&writer.finish()).into_bytes())
}

fn evaluate_transfer(input: ContainmentTransferInput<'_>) -> ContainmentTransferEvaluation {
    if transfer_is_allowed(input.view()) {
        ContainmentTransferEvaluation::Accepted
    } else {
        ContainmentTransferEvaluation::RequirementUnsatisfied
    }
}

fn transfer_is_allowed(view: ContainmentTransferReadView<'_>) -> bool {
    if view.item_container() != Some(view.source())
        || view.source() == view.destination()
        || view.item() == view.source()
        || view.item() == view.destination()
        || !view.source_exists()
        || !view.actor_controls_source()
    {
        return false;
    }
    let Some(destination_capacity) = view.destination_capacity() else {
        return false;
    };
    view.destination_direct_item_count() < u64::from(destination_capacity)
}

#[cfg(test)]
mod tests {
    use world_core::{ActorId, EntityId};
    use world_model::{
        AcceptedState, AgencyState, ContainerAuthorityRecord, ContainerRecord, ContainmentRecord,
        DomainState, EpistemicState, SocialState,
    };

    use super::*;

    fn valid<T, E: core::fmt::Debug>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("standard runtime fixture must be valid: {error:?}"),
        }
    }

    fn entity(byte: u8) -> EntityId {
        EntityId::from_bytes([byte; 32])
    }

    fn accepted(
        containers: Vec<ContainerRecord>,
        containment: Vec<ContainmentRecord>,
        authority: Vec<ContainerAuthorityRecord>,
    ) -> AcceptedState {
        AcceptedState::new(
            valid(DomainState::new(containers, containment, authority)),
            EpistemicState::empty(),
            SocialState::empty(),
            AgencyState::empty(),
        )
    }

    #[test]
    fn installation_matches_the_pure_standard_descriptor() {
        let implementation = standard_transfer_implementation();
        let expected = world_standard::transfer_interface_descriptor();

        assert_eq!(implementation.descriptor(), &expected);
        assert_eq!(implementation.interface(), expected.reference());
        assert_eq!(
            implementation.implementation_id(),
            standard_implementation_id(TRANSFER_IMPLEMENTATION_DOMAIN, &expected)
        );
        assert_eq!(
            implementation.implementation_id(),
            SemanticImplementationId::from_bytes([
                0x04, 0xbb, 0xb7, 0xc4, 0x8e, 0x05, 0xa8, 0x49, 0x9c, 0xb4, 0xa7, 0x2b, 0x7c, 0x8b,
                0x56, 0x45, 0x63, 0xb5, 0xfe, 0x7b, 0x39, 0x93, 0xd6, 0xcd, 0x95, 0x41, 0x35, 0x52,
                0x2d, 0x1a, 0xf8, 0xc2,
            ])
        );
    }

    #[test]
    fn relocation_installation_matches_the_pure_standard_descriptor() {
        let implementation = standard_relocation_implementation();
        let expected = world_standard::relocation_interface_descriptor();

        assert_eq!(implementation.descriptor(), &expected);
        assert_eq!(implementation.interface(), expected.reference());
    }

    #[test]
    fn relocation_operation_roles_are_part_of_semantic_identity() {
        let descriptor = world_standard::relocation_interface_descriptor();
        let base = standard_implementation_id(RELOCATION_IMPLEMENTATION_DOMAIN, &descriptor);
        let start = operation_name(world_standard::start_relocation_action_key());
        let pause = operation_name(world_standard::pause_relocation_action_key());
        let resume = operation_name(world_standard::resume_relocation_action_key());
        let installed = valid(RelocationActionImplementation::new(
            descriptor.clone(),
            base,
            start.clone(),
            pause.clone(),
            resume.clone(),
        ));
        let swapped = valid(RelocationActionImplementation::new(
            descriptor, base, pause, start, resume,
        ));

        assert_ne!(
            installed.implementation_id(),
            swapped.implementation_id(),
            "semantic identity must commit to the operation-role assignment"
        );
    }

    #[test]
    fn transfer_requirement_covers_state_and_authority_failures() {
        let actor = ActorId::from_bytes([0x11; 32]);
        let item = entity(0x21);
        let other_item = entity(0x22);
        let source = entity(0x31);
        let other_source = entity(0x32);
        let destination = entity(0x41);
        let missing_destination = entity(0x42);
        let containers = vec![
            ContainerRecord::new(source, 2),
            ContainerRecord::new(other_source, 2),
            ContainerRecord::new(destination, 1),
        ];
        let valid_state = accepted(
            containers.clone(),
            vec![ContainmentRecord::new(item, source)],
            vec![ContainerAuthorityRecord::new(actor, source)],
        );
        assert!(transfer_is_allowed(
            valid_state
                .domain()
                .containment_transfer_view(actor, item, source, destination,)
        ));

        let missing_authority = accepted(
            containers.clone(),
            vec![ContainmentRecord::new(item, source)],
            Vec::new(),
        );
        assert!(!transfer_is_allowed(
            missing_authority
                .domain()
                .containment_transfer_view(actor, item, source, destination,)
        ));

        let wrong_source = accepted(
            containers.clone(),
            vec![ContainmentRecord::new(item, source)],
            vec![ContainerAuthorityRecord::new(actor, other_source)],
        );
        assert!(!transfer_is_allowed(
            wrong_source
                .domain()
                .containment_transfer_view(actor, item, other_source, destination,)
        ));
        assert!(!transfer_is_allowed(
            valid_state.domain().containment_transfer_view(
                actor,
                item,
                source,
                missing_destination,
            )
        ));

        let full_destination = accepted(
            containers,
            vec![
                ContainmentRecord::new(item, source),
                ContainmentRecord::new(other_item, destination),
            ],
            vec![ContainerAuthorityRecord::new(actor, source)],
        );
        assert!(!transfer_is_allowed(
            full_destination
                .domain()
                .containment_transfer_view(actor, item, source, destination,)
        ));
    }
}
