#[cfg(test)]
use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter};

use crate::authority::{AppliedAuthorityRecord, AuthorityCursor, AuthorityRecordId};

use super::{AttemptBinding, AttemptStepId, ReservedOperationFingerprint, StepReservation};

#[cfg(test)]
const STEP_PUBLICATION_RECEIPT_SCHEMA_VERSION: u16 = 1;

#[cfg(test)]
const STEP_PUBLICATION_RECEIPT_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("step-publication-receipt-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("step publication receipt domain must be valid"),
    };

/// Host-control provenance for one successfully published reserved step.
///
/// The receipt is retained in attempt control. It deliberately does not
/// participate in authority-record or trajectory identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StepPublicationReceipt {
    binding: AttemptBinding,
    step: AttemptStepId,
    operation_fingerprint: ReservedOperationFingerprint,
    expected: AuthorityCursor,
    resulting: AuthorityCursor,
    record: AuthorityRecordId,
}

impl StepPublicationReceipt {
    pub(crate) fn from_publication(
        reservation: &StepReservation,
        publication: &AppliedAuthorityRecord,
    ) -> Self {
        Self {
            binding: reservation.binding(),
            step: reservation.step(),
            operation_fingerprint: reservation.operation_fingerprint(),
            expected: reservation.expected(),
            resulting: publication.resulting_head().cursor(),
            record: publication.record().header().id(),
        }
    }

    pub(crate) const fn binding(self) -> AttemptBinding {
        self.binding
    }

    pub(crate) const fn step(self) -> AttemptStepId {
        self.step
    }

    pub(crate) const fn operation_fingerprint(self) -> ReservedOperationFingerprint {
        self.operation_fingerprint
    }

    pub(crate) const fn expected(self) -> AuthorityCursor {
        self.expected
    }

    pub(crate) const fn resulting(self) -> AuthorityCursor {
        self.resulting
    }

    pub(crate) const fn record(self) -> AuthorityRecordId {
        self.record
    }

    #[cfg(test)]
    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        let mut writer = CanonicalWriter::new(STEP_PUBLICATION_RECEIPT_DOMAIN);
        writer.write_u16(STEP_PUBLICATION_RECEIPT_SCHEMA_VERSION);
        write_owned_bytes(&mut writer, self.binding.canonical_bytes().as_bytes());
        write_fixed_bytes(&mut writer, self.step.as_bytes());
        write_fixed_bytes(&mut writer, self.operation_fingerprint.as_bytes());
        write_owned_bytes(&mut writer, self.expected.canonical_bytes().as_bytes());
        write_owned_bytes(&mut writer, self.resulting.canonical_bytes().as_bytes());
        write_fixed_bytes(&mut writer, self.record.as_bytes());
        writer.finish()
    }
}

#[cfg(test)]
fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width identity length must fit the canonical protocol");
    }
}

#[cfg(test)]
fn write_owned_bytes(writer: &mut CanonicalWriter, bytes: &[u8]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("owned canonical bytes must fit the canonical protocol");
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use world_core::{Microstep, SimMoment, SimTime};
    use world_model::{AcceptedState, AgencyState, DomainState, EpistemicState, SocialState};

    use crate::authority::{DraftAuthorityRecord, apply_authority_record, seal_authority_record};
    use crate::execution::{
        CanonicalExecutionSpecV1, ExecutionConfigArtifactV3, ExecutionSemanticsManifestV1,
        ExternalInputBindingV1, InitialStateRootV1, ResolvedExecutionClosureManifestV1, RootSeed,
        SemanticImplementationBinding, SemanticImplementationId, TerminationContractV1,
    };
    use crate::kernel::{AdmitRequest, InputId, fixtures};
    use crate::session::{SessionHead, SessionMode};

    use super::super::{
        AttemptAuthorityDomainId, AttemptCreation, AttemptKey, ReservationGrant,
        ReservedOperationDescriptor,
    };
    use super::*;

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("receipt fixture must be valid: {error}"),
        }
    }

    fn closure() -> ResolvedExecutionClosureManifestV1 {
        let definitions = fixtures::command_definitions();
        let interface = match definitions.required_interfaces().first() {
            Some(interface) => interface.clone(),
            None => panic!("command fixture must require one semantic interface"),
        };
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            definitions,
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            vec![SemanticImplementationBinding::new(
                interface,
                SemanticImplementationId::from_bytes([0x62; 32]),
            )],
        ));
        let root = valid(InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            AcceptedState::new(
                valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
                EpistemicState::empty(),
                SocialState::empty(),
                AgencyState::empty(),
            ),
            Vec::new(),
        ));
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &semantics,
            RootSeed::from_bytes([0x61; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        valid(ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ))
    }

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
            encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        encoded
    }

    #[test]
    fn publication_receipt_has_a_frozen_vector() {
        let closure = closure();
        let head = SessionHead::root(&closure);
        let creation = AttemptCreation::derive(
            AttemptAuthorityDomainId::from_bytes([0x11; 32]),
            AttemptKey::from_bytes([0x21; 32]),
            &closure,
        );
        let request = AdmitRequest::new(
            InputId::new(7),
            SimMoment::new(SimTime::from_ticks(5), Microstep::new(3)),
            fixtures::command(0x31, 11),
        );
        let reservation = StepReservation::new(
            creation.binding(),
            ReservationGrant::FIRST,
            head.cursor(),
            ReservedOperationDescriptor::admit_command(&request),
        );
        let sealed = match seal_authority_record(
            &head,
            &closure,
            DraftAuthorityRecord::admit_commands(head.cursor(), vec![request]),
        ) {
            Ok(sealed) => sealed,
            Err(error) => panic!("receipt record must seal: {error:?}"),
        };
        let publication = match apply_authority_record(&head, sealed) {
            Ok(publication) => publication,
            Err(error) => panic!("receipt record must apply: {error:?}"),
        };
        let receipt = StepPublicationReceipt::from_publication(&reservation, &publication);

        assert_eq!(receipt.binding(), creation.binding());
        assert_eq!(receipt.step(), reservation.step());
        assert_eq!(
            receipt.operation_fingerprint(),
            reservation.operation_fingerprint()
        );
        assert_eq!(receipt.expected(), head.cursor());
        assert_eq!(receipt.resulting(), publication.resulting_head().cursor());
        assert_eq!(receipt.record(), publication.record().header().id());
        assert_eq!(
            hex(receipt.canonical_bytes().as_bytes()),
            "776f726c642d63616e6f6e6963616c2d7631000000000000001b737465702d7075626c69636174696f6e2d726563656970742d7631000100000000000000f6776f726c642d63616e6f6e6963616c2d76310000000000000012617474656d70742d62696e64696e672d76310001000000000000002011111111111111111111111111111111111111111111111111111111111111110000000000000020daba7c6632cc3325da448c17b4c485ae0ef6eaf97186e6aa211d6843335b0d3900000000000000201642a9a270e7e639562c2929780672ee37cfdc151e7ed3c0dbf6a2f3fbf01bf1000000000000002092986bc277239050c90cd9a7386cc08b0cd49011bd9fa649da6779b03c72d8180000000000000020b11dbda2fa2028edbcd0b4d91d3f44e515c5108300d1b890d19b4dbdb91bb4870000000000000020bb38e48560b5c0cace33dc2efe284af9b95ac7409345af4a653c19e1b8a149460000000000000020d0bb589001dfa969e9892b1492b25dab632cea46911d54d07f2692a2287ef44200000000000000d3776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d637572736f722d763100010000000000000020b11dbda2fa2028edbcd0b4d91d3f44e515c5108300d1b890d19b4dbdb91bb48700000000000000201642a9a270e7e639562c2929780672ee37cfdc151e7ed3c0dbf6a2f3fbf01bf10000000000000000000000201e3de46da2171d68f574a296b97af52e24db9254225b4ae461d48bc7c83026c900000000000000209f322ce8a10cc1d87b6b0558d111803846707c2287958775db9990adca0ade6200000000000000e3776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d637572736f722d763100010000000000000020b11dbda2fa2028edbcd0b4d91d3f44e515c5108300d1b890d19b4dbdb91bb48700000000000000201642a9a270e7e639562c2929780672ee37cfdc151e7ed3c0dbf6a2f3fbf01bf10000000100000000000000010000000000000001000000000000002011b241554d6c9c9f9da39c53991add2d5797a3ec195c73bc8875003e8cf1c43a0000000000000020be4163cb98f5418b21689d7c4d242544c4654048e57c463a9279203226ed7e62000000000000002011b241554d6c9c9f9da39c53991add2d5797a3ec195c73bc8875003e8cf1c43a"
        );
    }
}
