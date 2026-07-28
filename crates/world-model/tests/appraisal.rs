use world_core::{ActorId, EntityId};
use world_model::{ContainmentAppraisal, EvidenceDeliveryId};

fn actor(byte: u8) -> ActorId {
    ActorId::from_bytes([byte; 32])
}

fn entity(byte: u8) -> EntityId {
    EntityId::from_bytes([byte; 32])
}

fn appraisal(
    actor_byte: u8,
    item_byte: u8,
    current_byte: u8,
    restore_byte: u8,
    evidence_byte: u8,
) -> ContainmentAppraisal {
    ContainmentAppraisal::new(
        actor(actor_byte),
        entity(item_byte),
        entity(current_byte),
        entity(restore_byte),
        EvidenceDeliveryId::from_bytes([evidence_byte; 32]),
    )
}

#[test]
fn containment_appraisal_exposes_only_actor_safe_material() {
    let value = appraisal(0x40, 0x50, 0x20, 0x10, 0x60);

    assert_eq!(value.actor(), actor(0x40));
    assert_eq!(value.item(), entity(0x50));
    assert_eq!(value.believed_current_container(), entity(0x20));
    assert_eq!(value.restore_container(), entity(0x10));
    assert_eq!(
        value.supporting_evidence(),
        EvidenceDeliveryId::from_bytes([0x60; 32])
    );
    assert_eq!(
        value.material_fingerprint().to_string(),
        "493597360ec5f3f67a3977be74ac004ce1f7aecad493b7b1f0cdaaf954aa62e7"
    );
}

#[test]
fn material_fingerprint_changes_with_meaning_but_not_support_provenance() {
    let baseline = appraisal(0x40, 0x50, 0x20, 0x10, 0x60).material_fingerprint();
    let variants = [
        appraisal(0x41, 0x50, 0x20, 0x10, 0x60),
        appraisal(0x40, 0x51, 0x20, 0x10, 0x60),
        appraisal(0x40, 0x50, 0x21, 0x10, 0x60),
        appraisal(0x40, 0x50, 0x20, 0x11, 0x60),
    ];

    for variant in variants {
        assert_ne!(baseline, variant.material_fingerprint());
    }
    assert_eq!(
        baseline,
        appraisal(0x40, 0x50, 0x20, 0x10, 0x61).material_fingerprint()
    );
}
