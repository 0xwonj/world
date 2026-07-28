use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter, SimMoment};

use super::{TerminationClauseId, TerminationContractDigest};

/// Canonical schema of one V1 termination clause.
pub const TERMINATION_CLAUSE_SCHEMA_VERSION: u16 = 1;

/// Canonical schema of the V1 termination contract.
pub const TERMINATION_CONTRACT_SCHEMA_VERSION: u16 = 1;

const TERMINATION_CLAUSE_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("termination-clause-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("termination clause domain must be valid"),
    };

const TERMINATION_CONTRACT_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("termination-contract-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("termination contract domain must be valid"),
    };

/// Semantic reason selected by the termination contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticTerminationReasonV1 {
    /// The authoritative session reached the configured simulation moment.
    ReachedConfiguredMoment,
}

impl SemanticTerminationReasonV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::ReachedConfiguredMoment => 0,
        }
    }
}

/// Pure, closed semantic termination contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminationContractV1 {
    /// No semantic time condition finalizes execution.
    Never,
    /// Execution finalizes once its exact simulation moment reaches the
    /// configured threshold.
    AtOrAfterMoment {
        /// Inclusive simulation-moment threshold.
        moment: SimMoment,
    },
}

impl TerminationContractV1 {
    /// Constructs a contract that never selects semantic finalization.
    #[must_use]
    pub const fn never() -> Self {
        Self::Never
    }

    /// Constructs a contract that finalizes at or after one exact simulation
    /// moment.
    #[must_use]
    pub const fn at_or_after_moment(moment: SimMoment) -> Self {
        Self::AtOrAfterMoment { moment }
    }

    /// Returns the canonical contract identity.
    #[must_use]
    pub fn digest(self) -> TerminationContractDigest {
        TerminationContractDigest::of_canonical(&self.canonical_bytes())
    }

    /// Returns the configured clause identity, if this contract can
    /// terminate.
    #[must_use]
    pub fn clause_id(self) -> Option<TerminationClauseId> {
        match self {
            Self::Never => None,
            Self::AtOrAfterMoment { moment } => Some(TerminationClauseId::of_canonical(
                &termination_clause_bytes(moment),
            )),
        }
    }

    /// Returns the configured threshold moment, if present.
    #[must_use]
    pub const fn configured_moment(self) -> Option<SimMoment> {
        match self {
            Self::Never => None,
            Self::AtOrAfterMoment { moment } => Some(moment),
        }
    }

    /// Returns the fixed semantic reason selected by a configured clause.
    #[must_use]
    pub const fn reason(self) -> Option<SemanticTerminationReasonV1> {
        match self {
            Self::Never => None,
            Self::AtOrAfterMoment { .. } => {
                Some(SemanticTerminationReasonV1::ReachedConfiguredMoment)
            }
        }
    }

    /// Evaluates the contract against the current simulation moment.
    #[must_use]
    pub fn evaluate(self, now: SimMoment) -> Option<SemanticTerminationReasonV1> {
        match self {
            Self::Never => None,
            Self::AtOrAfterMoment { moment } if now >= moment => {
                Some(SemanticTerminationReasonV1::ReachedConfiguredMoment)
            }
            Self::AtOrAfterMoment { .. } => None,
        }
    }

    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        termination_contract_bytes(self)
    }
}

fn termination_clause_bytes(moment: SimMoment) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(TERMINATION_CLAUSE_DOMAIN);
    writer.write_u16(TERMINATION_CLAUSE_SCHEMA_VERSION);
    writer.write_discriminant(0);
    write_moment(&mut writer, moment);
    writer.write_discriminant(SemanticTerminationReasonV1::ReachedConfiguredMoment.canonical_tag());
    writer.finish()
}

fn termination_contract_bytes(contract: TerminationContractV1) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(TERMINATION_CONTRACT_DOMAIN);
    writer.write_u16(TERMINATION_CONTRACT_SCHEMA_VERSION);
    match contract {
        TerminationContractV1::Never => writer.write_discriminant(0),
        TerminationContractV1::AtOrAfterMoment { moment } => {
            writer.write_discriminant(1);
            write_moment(&mut writer, moment);
            writer.write_discriminant(
                SemanticTerminationReasonV1::ReachedConfiguredMoment.canonical_tag(),
            );
        }
    }
    writer.finish()
}

fn write_moment(writer: &mut CanonicalWriter, moment: SimMoment) {
    writer.write_u64(moment.time().ticks());
    writer.write_u64(moment.microstep().get());
}

#[cfg(test)]
mod tests {
    use world_core::{Microstep, SimTime};

    use super::*;

    #[test]
    fn never_contract_has_no_clause_or_reason() {
        let contract = TerminationContractV1::never();

        assert_eq!(contract.clause_id(), None);
        assert_eq!(contract.configured_moment(), None);
        assert_eq!(contract.reason(), None);
        assert_eq!(contract.evaluate(SimMoment::ORIGIN), None);
        assert_eq!(
            hex(contract.canonical_bytes().as_bytes()),
            "776f726c642d63616e6f6e6963616c2d763100000000000000177465726d696e6174696f6e2d636f6e74726163742d7631000100000000"
        );
        assert_eq!(
            contract.digest().to_string(),
            "9caa5ddacf46747d1ad1a0cce85b919e261f26dfcfca65c6fe2a685ff9ffdd7f"
        );
    }

    #[test]
    fn configured_moment_clause_has_stable_threshold_semantics() {
        let threshold = SimMoment::new(SimTime::from_ticks(7), Microstep::new(3));
        let contract = TerminationContractV1::at_or_after_moment(threshold);

        assert!(contract.clause_id().is_some());
        assert_eq!(contract.configured_moment(), Some(threshold));
        assert_eq!(
            contract.reason(),
            Some(SemanticTerminationReasonV1::ReachedConfiguredMoment)
        );
        assert_eq!(
            contract.evaluate(SimMoment::new(SimTime::from_ticks(7), Microstep::new(2))),
            None
        );
        assert_eq!(
            contract.evaluate(threshold),
            Some(SemanticTerminationReasonV1::ReachedConfiguredMoment)
        );
        assert_eq!(
            contract.evaluate(SimMoment::new(SimTime::from_ticks(8), Microstep::ZERO)),
            Some(SemanticTerminationReasonV1::ReachedConfiguredMoment)
        );
        assert_eq!(
            contract.clause_id().map(|identity| identity.to_string()),
            Some("1ea878d2d699e0f4618ca99bbe48432dbf327b97e8c80ff5eb826d4a3848ab60".to_owned())
        );
        assert_eq!(
            contract.digest().to_string(),
            "6f7b3b3b184bb0d39fbecef72c9ed4e8d7b13b0e4778cc1db0a6dbe79cff214c"
        );
        assert_eq!(
            hex(termination_clause_bytes(threshold).as_bytes()),
            "776f726c642d63616e6f6e6963616c2d763100000000000000157465726d696e6174696f6e2d636c617573652d76310001000000000000000000000007000000000000000300000000"
        );
        assert_eq!(
            hex(contract.canonical_bytes().as_bytes()),
            "776f726c642d63616e6f6e6963616c2d763100000000000000177465726d696e6174696f6e2d636f6e74726163742d76310001000000010000000000000007000000000000000300000000"
        );
    }

    #[test]
    fn clause_and_contract_identities_are_sensitive_to_the_threshold() {
        let first =
            TerminationContractV1::at_or_after_moment(SimMoment::at(SimTime::from_ticks(1)));
        let second =
            TerminationContractV1::at_or_after_moment(SimMoment::at(SimTime::from_ticks(2)));

        assert_ne!(first.clause_id(), second.clause_id());
        assert_ne!(first.digest(), second.digest());
        assert_ne!(TerminationContractV1::never().digest(), first.digest());
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
}
