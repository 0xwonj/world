use world_core::{ActorId, EntityId};

use crate::{ContainerRecord, ContainmentRecord, DomainState};

/// Immutable facts required to evaluate one direct-containment transfer.
///
/// The view is tied to one accepted state but exposes neither that state nor
/// an open-ended query surface. It carries the exact bound roles, references
/// only the relevant accepted records, and materializes the one aggregate
/// count required by containment-capacity policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainmentTransferReadView<'world> {
    actor: ActorId,
    item: EntityId,
    source: EntityId,
    destination: EntityId,
    item_containment: Option<&'world ContainmentRecord>,
    source_container: Option<&'world ContainerRecord>,
    destination_container: Option<&'world ContainerRecord>,
    actor_controls_source: bool,
    destination_direct_item_count: u64,
}

impl<'world> ContainmentTransferReadView<'world> {
    pub(crate) fn from_domain(
        domain: &'world DomainState,
        actor: ActorId,
        item: EntityId,
        source: EntityId,
        destination: EntityId,
    ) -> Self {
        let destination_direct_item_count = domain
            .containment()
            .iter()
            .filter(|record| record.container() == destination)
            .fold(0_u64, |count, _| count.saturating_add(1));

        Self {
            actor,
            item,
            source,
            destination,
            item_containment: domain.containment_for(item),
            source_container: domain.container(source),
            destination_container: domain.container(destination),
            actor_controls_source: domain.actor_controls(actor, source),
            destination_direct_item_count,
        }
    }

    /// Returns the actor bound to the transfer request.
    #[must_use]
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the item bound to the transfer request.
    #[must_use]
    pub const fn item(self) -> EntityId {
        self.item
    }

    /// Returns the expected source bound to the transfer request.
    #[must_use]
    pub const fn source(self) -> EntityId {
        self.source
    }

    /// Returns the destination bound to the transfer request.
    #[must_use]
    pub const fn destination(self) -> EntityId {
        self.destination
    }

    /// Returns the item's accepted direct container, when the item is present.
    #[must_use]
    pub fn item_container(self) -> Option<EntityId> {
        self.item_containment.map(|record| record.container())
    }

    /// Returns whether the expected source is an accepted container.
    #[must_use]
    pub const fn source_exists(self) -> bool {
        self.source_container.is_some()
    }

    /// Returns whether the bound actor has hard transfer authority at source.
    #[must_use]
    pub const fn actor_controls_source(self) -> bool {
        self.actor_controls_source
    }

    /// Returns the destination's direct-item capacity, when it exists.
    #[must_use]
    pub fn destination_capacity(self) -> Option<u32> {
        self.destination_container
            .map(|record| record.item_capacity())
    }

    /// Returns the accepted number of items directly in the destination.
    #[must_use]
    pub const fn destination_direct_item_count(self) -> u64 {
        self.destination_direct_item_count
    }
}

impl DomainState {
    /// Projects the immutable facts used by direct-containment semantics.
    #[must_use]
    pub fn containment_transfer_view(
        &self,
        actor: ActorId,
        item: EntityId,
        source: EntityId,
        destination: EntityId,
    ) -> ContainmentTransferReadView<'_> {
        ContainmentTransferReadView::from_domain(self, actor, item, source, destination)
    }
}
