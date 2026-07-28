use std::collections::BTreeMap;

use world_core::{
    ActorId, CanonicalBytes, CanonicalDomain, CanonicalWriter, ContentDigest, EntityId, SimMoment,
};
use world_model::{CommandId, CommandSource};

use crate::execution::RootSeed;

const SEMANTIC_RANDOM_KEY_SCHEMA_VERSION: u16 = 1;
#[cfg(test)]
const CONTAINMENT_RANDOM_RANK_EVIDENCE_SCHEMA_VERSION: u16 = 1;
const BLAKE3_KEYED_PRF_256_VERSION: u16 = 1;
const RANDOM_KEY_POLICY_VERSION: u16 = 1;
const ROOT_RANDOM_NAMESPACE_TAG: u32 = 0;

const RANDOM_MASTER_CONTEXT: &str = "world runtime 2026-07-28 authoritative randomness root v1";

const SEMANTIC_RANDOM_KEY_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("semantic-random-key-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("semantic random key domain must be valid"),
    };

#[cfg(test)]
const CONTAINMENT_RANDOM_RANK_EVIDENCE_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("containment-random-rank-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("containment random rank evidence domain must be valid"),
    };

/// Closed logical purpose of an authoritative random result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SemanticRandomPurposeV1 {
    ContainmentConflictRank,
}

impl SemanticRandomPurposeV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::ContainmentConflictRank => 0,
        }
    }
}

/// Semantic containment resource that can connect same-moment contenders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ContainmentConflictResourceV1 {
    /// Exclusive authority to change one item's direct containment relation.
    ExclusiveItem(EntityId),
    /// Shared opportunity constrained by one destination's direct capacity.
    DestinationCapacity(EntityId),
}

impl ContainmentConflictResourceV1 {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::ExclusiveItem(_) => 0,
            Self::DestinationCapacity(_) => 1,
        }
    }

    const fn entity(self) -> EntityId {
        match self {
            Self::ExclusiveItem(entity) | Self::DestinationCapacity(entity) => entity,
        }
    }
}

/// One same-moment containment conflict opportunity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ContainmentConflictGroupV1 {
    moment: SimMoment,
    resource: ContainmentConflictResourceV1,
}

impl ContainmentConflictGroupV1 {
    /// Names one typed semantic resource at one causal moment.
    #[must_use]
    pub(crate) const fn new(moment: SimMoment, resource: ContainmentConflictResourceV1) -> Self {
        Self { moment, resource }
    }

    /// Returns the shared causal moment.
    #[must_use]
    pub(crate) const fn moment(self) -> SimMoment {
        self.moment
    }

    /// Returns the resource defining this conflict opportunity.
    #[must_use]
    pub(crate) const fn resource(self) -> ContainmentConflictResourceV1 {
        self.resource
    }
}

/// Format-independent identity of one containment contender.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ContainmentConflictContenderV1 {
    actor: ActorId,
    source: CommandSource,
    command: CommandId,
}

impl ContainmentConflictContenderV1 {
    /// Names one exact logical command competing for a containment resource.
    #[must_use]
    pub(crate) const fn new(actor: ActorId, source: CommandSource, command: CommandId) -> Self {
        Self {
            actor,
            source,
            command,
        }
    }

    /// Returns the acting actor.
    #[must_use]
    pub(crate) const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns the command-producing semantic source.
    #[must_use]
    pub(crate) const fn source(self) -> CommandSource {
        self.source
    }

    /// Returns the source-scoped logical command identity.
    #[must_use]
    pub(crate) const fn command(self) -> CommandId {
        self.command
    }
}

/// Complete semantic key for one containment-conflict rank score.
///
/// The root seed participates through the keyed oracle rather than being
/// duplicated in these canonical message bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SemanticRandomKeyV1 {
    purpose: SemanticRandomPurposeV1,
    group: ContainmentConflictGroupV1,
    contender: ContainmentConflictContenderV1,
    draw_ordinal: u32,
    key_policy_version: u16,
}

impl SemanticRandomKeyV1 {
    const fn containment_rank(
        group: ContainmentConflictGroupV1,
        contender: ContainmentConflictContenderV1,
    ) -> Self {
        Self {
            purpose: SemanticRandomPurposeV1::ContainmentConflictRank,
            group,
            contender,
            draw_ordinal: 0,
            key_policy_version: RANDOM_KEY_POLICY_VERSION,
        }
    }

    /// Returns the exact conflict group named by this key.
    #[must_use]
    pub(crate) const fn group(self) -> ContainmentConflictGroupV1 {
        self.group
    }

    /// Returns the semantic contender named by this key.
    #[must_use]
    pub(crate) const fn contender(self) -> ContainmentConflictContenderV1 {
        self.contender
    }

    /// Returns the schema-owned draw ordinal.
    #[must_use]
    pub(crate) const fn draw_ordinal(self) -> u32 {
        self.draw_ordinal
    }

    /// Returns the semantic-key policy version captured by this key.
    #[must_use]
    pub(crate) const fn key_policy_version(self) -> u16 {
        self.key_policy_version
    }

    /// Returns the canonical, schema-owned PRF message.
    #[must_use]
    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        semantic_random_key_bytes(self)
    }

    pub(crate) fn id(self) -> SemanticRandomKeyId {
        SemanticRandomKeyId(ContentDigest::of_canonical(&self.canonical_bytes()).into_bytes())
    }
}

/// Canonical identity of a complete semantic random key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SemanticRandomKeyId([u8; 32]);

impl SemanticRandomKeyId {
    /// Returns the exact key-identity bytes.
    #[must_use]
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// One fixed-width result from the configured keyed PRF.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RandomScore256([u8; 32]);

impl RandomScore256 {
    /// Returns the exact score bytes, ordered lexicographically for ranking.
    #[must_use]
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[cfg(test)]
    const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// BLAKE3 keyed PRF with a master deterministically derived from `RootSeed`.
///
/// The type deliberately exposes neither a mutable stream nor a generic
/// random-provider interface.
pub(crate) struct Blake3KeyedPrf256V1 {
    master_key: [u8; 32],
}

impl Blake3KeyedPrf256V1 {
    /// Binds the sole random root of an execution to this fixed algorithm.
    #[must_use]
    pub(crate) fn from_root_seed(root_seed: RootSeed) -> Self {
        Self {
            master_key: blake3::derive_key(RANDOM_MASTER_CONTEXT, root_seed.as_bytes()),
        }
    }

    /// Evaluates one semantic key without advancing any shared state.
    #[must_use]
    pub(crate) fn score(&self, key: SemanticRandomKeyV1) -> RandomScore256 {
        RandomScore256(
            *blake3::keyed_hash(&self.master_key, key.canonical_bytes().as_bytes()).as_bytes(),
        )
    }
}

/// One canonical key/result pair retained as conflict-resolution evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ContainmentRandomRankEntryV1 {
    key: SemanticRandomKeyV1,
    key_id: SemanticRandomKeyId,
    score: RandomScore256,
}

impl ContainmentRandomRankEntryV1 {
    /// Returns the complete semantic random key.
    #[must_use]
    pub(crate) const fn key(self) -> SemanticRandomKeyV1 {
        self.key
    }

    /// Returns the checked canonical key identity.
    #[must_use]
    pub(crate) const fn key_id(self) -> SemanticRandomKeyId {
        self.key_id
    }

    /// Returns the exact PRF result used for ranking.
    #[must_use]
    pub(crate) const fn score(self) -> RandomScore256 {
        self.score
    }
}

/// Complete authoritative random evidence for one containment conflict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContainmentConflictRandomEvidenceV1 {
    entries: Vec<ContainmentRandomRankEntryV1>,
    winner: ContainmentConflictContenderV1,
}

impl ContainmentConflictRandomEvidenceV1 {
    /// Returns canonical contender-key-score entries.
    #[must_use]
    pub(crate) fn entries(&self) -> &[ContainmentRandomRankEntryV1] {
        &self.entries
    }

    /// Returns the highest-scoring contender.
    #[must_use]
    pub(crate) const fn winner(&self) -> ContainmentConflictContenderV1 {
        self.winner
    }

    /// Returns the canonical bytes retained by the owning conflict receipt.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn canonical_bytes(&self) -> CanonicalBytes {
        containment_random_rank_evidence_bytes(self)
    }
}

/// Why a containment conflict could not produce trustworthy random evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContainmentRandomRankError {
    /// A conflict group contained no contender to rank.
    EmptyConflictGroup,
    /// Two distinct uses constructed the same semantic key.
    SemanticKeyReuse {
        key: SemanticRandomKeyId,
        first: ContainmentConflictContenderV1,
        second: ContainmentConflictContenderV1,
    },
    /// Two distinct semantic keys produced the same fixed-width score.
    ScoreCollision {
        score: RandomScore256,
        first: SemanticRandomKeyId,
        second: SemanticRandomKeyId,
    },
}

/// Ranks one containment conflict independently of input order.
pub(crate) fn rank_containment_conflict(
    oracle: &Blake3KeyedPrf256V1,
    group: ContainmentConflictGroupV1,
    contenders: &[ContainmentConflictContenderV1],
) -> Result<ContainmentConflictRandomEvidenceV1, Box<ContainmentRandomRankError>> {
    rank_containment_conflict_with(group, contenders, |key| oracle.score(key))
}

fn rank_containment_conflict_with(
    group: ContainmentConflictGroupV1,
    contenders: &[ContainmentConflictContenderV1],
    mut score: impl FnMut(SemanticRandomKeyV1) -> RandomScore256,
) -> Result<ContainmentConflictRandomEvidenceV1, Box<ContainmentRandomRankError>> {
    if contenders.is_empty() {
        return Err(Box::new(ContainmentRandomRankError::EmptyConflictGroup));
    }

    let mut canonical_contenders = contenders.to_vec();
    canonical_contenders.sort_unstable();

    let mut key_uses = BTreeMap::new();
    let mut score_uses = BTreeMap::new();
    let mut entries = Vec::with_capacity(canonical_contenders.len());
    let mut winner: Option<(RandomScore256, ContainmentConflictContenderV1)> = None;

    for contender in canonical_contenders {
        let key = SemanticRandomKeyV1::containment_rank(group, contender);
        let key_id = key.id();
        if let Some(first) = key_uses.insert(key_id, contender) {
            return Err(Box::new(ContainmentRandomRankError::SemanticKeyReuse {
                key: key_id,
                first,
                second: contender,
            }));
        }

        let contender_score = score(key);
        if let Some(first) = score_uses.insert(contender_score, key_id) {
            return Err(Box::new(ContainmentRandomRankError::ScoreCollision {
                score: contender_score,
                first,
                second: key_id,
            }));
        }

        if winner.is_none_or(|(winning_score, _)| contender_score > winning_score) {
            winner = Some((contender_score, contender));
        }
        entries.push(ContainmentRandomRankEntryV1 {
            key,
            key_id,
            score: contender_score,
        });
    }

    let Some((_, winner)) = winner else {
        unreachable!("the nonempty contender set must produce one winner");
    };
    Ok(ContainmentConflictRandomEvidenceV1 { entries, winner })
}

fn semantic_random_key_bytes(key: SemanticRandomKeyV1) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(SEMANTIC_RANDOM_KEY_DOMAIN);
    writer.write_u16(SEMANTIC_RANDOM_KEY_SCHEMA_VERSION);
    writer.write_u16(BLAKE3_KEYED_PRF_256_VERSION);
    writer.write_u16(key.key_policy_version);
    writer.write_discriminant(ROOT_RANDOM_NAMESPACE_TAG);
    writer.write_discriminant(key.purpose.canonical_tag());
    write_moment(&mut writer, key.group.moment);
    writer.write_discriminant(key.group.resource.canonical_tag());
    write_fixed_bytes(&mut writer, key.group.resource.entity().as_bytes());
    write_fixed_bytes(&mut writer, key.contender.actor.as_bytes());
    write_fixed_bytes(&mut writer, key.contender.source.as_bytes());
    writer.write_u64(key.contender.command.get());
    writer.write_u32(key.draw_ordinal);
    writer.finish()
}

#[cfg(test)]
fn containment_random_rank_evidence_bytes(
    evidence: &ContainmentConflictRandomEvidenceV1,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(CONTAINMENT_RANDOM_RANK_EVIDENCE_DOMAIN);
    writer.write_u16(CONTAINMENT_RANDOM_RANK_EVIDENCE_SCHEMA_VERSION);
    writer.write_u16(BLAKE3_KEYED_PRF_256_VERSION);
    writer.write_u16(RANDOM_KEY_POLICY_VERSION);
    write_sequence(&mut writer, &evidence.entries, |writer, entry| {
        write_owned_bytes(writer, entry.key.canonical_bytes().as_bytes());
        write_fixed_bytes(writer, entry.key_id.as_bytes());
        write_fixed_bytes(writer, entry.score.as_bytes());
    });
    write_contender(&mut writer, evidence.winner);
    writer.finish()
}

#[cfg(test)]
fn write_contender(writer: &mut CanonicalWriter, contender: ContainmentConflictContenderV1) {
    write_fixed_bytes(writer, contender.actor.as_bytes());
    write_fixed_bytes(writer, contender.source.as_bytes());
    writer.write_u64(contender.command.get());
}

fn write_moment(writer: &mut CanonicalWriter, moment: SimMoment) {
    writer.write_u64(moment.time().ticks());
    writer.write_u64(moment.microstep().get());
}

fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width random evidence must fit canonical bytes");
    }
}

#[cfg(test)]
fn write_owned_bytes(writer: &mut CanonicalWriter, bytes: &[u8]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("owned random evidence must fit canonical bytes");
    }
}

#[cfg(test)]
fn write_sequence<T>(
    writer: &mut CanonicalWriter,
    values: &[T],
    write_value: impl FnMut(&mut CanonicalWriter, &T),
) {
    let mut write_value = write_value;
    if writer
        .write_sequence(values, |writer, value| {
            write_value(writer, value);
            Ok(())
        })
        .is_err()
    {
        unreachable!("in-memory random evidence length must fit canonical bytes");
    }
}

#[cfg(test)]
mod tests {
    use world_core::{Microstep, SimTime};

    use super::*;

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(ticks), Microstep::new(microstep))
    }

    fn group(item: u8) -> ContainmentConflictGroupV1 {
        ContainmentConflictGroupV1::new(
            moment(11, 2),
            ContainmentConflictResourceV1::ExclusiveItem(EntityId::from_bytes([item; 32])),
        )
    }

    fn contender(actor: u8, source: u8, command: u64) -> ContainmentConflictContenderV1 {
        ContainmentConflictContenderV1::new(
            ActorId::from_bytes([actor; 32]),
            CommandSource::from_bytes([source; 32]),
            CommandId::new(command),
        )
    }

    fn oracle() -> Blake3KeyedPrf256V1 {
        Blake3KeyedPrf256V1::from_root_seed(RootSeed::from_bytes([0x5a; 32]))
    }

    fn must_rank(
        contenders: &[ContainmentConflictContenderV1],
    ) -> ContainmentConflictRandomEvidenceV1 {
        match rank_containment_conflict(&oracle(), group(0x41), contenders) {
            Ok(evidence) => evidence,
            Err(error) => panic!("fixture must rank: {error:?}"),
        }
    }

    fn hex(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            use core::fmt::Write;
            if write!(output, "{byte:02x}").is_err() {
                unreachable!("writing to a String cannot fail");
            }
        }
        output
    }

    #[test]
    fn golden_master_key_semantic_key_score_and_evidence_are_stable() {
        let oracle = oracle();
        let key = SemanticRandomKeyV1::containment_rank(group(0x41), contender(0x11, 0x21, 7));
        let evidence = must_rank(&[contender(0x11, 0x21, 7), contender(0x12, 0x22, 8)]);

        assert_eq!(
            hex(&oracle.master_key),
            "bc4de4720b8fb4eb499b882b3d78ba8961543beec569b55958d0f629db532230"
        );
        assert_eq!(
            hex(key.canonical_bytes().as_bytes()),
            "776f726c642d63616e6f6e6963616c2d7631000000000000001673656d616e7469632d72616e646f6d2d6b65792d76310001000100010000000000000000000000000000000b000000000000000200000000000000000000002041414141414141414141414141414141414141414141414141414141414141410000000000000020111111111111111111111111111111111111111111111111111111111111111100000000000000202121212121212121212121212121212121212121212121212121212121212121000000000000000700000000"
        );
        assert_eq!(
            hex(key.id().as_bytes()),
            "95274f47bd3c1ee656ac9a6dfeb02e75a1c3a1ba812199a4774a6ac72ef20f3f"
        );
        assert_eq!(
            hex(oracle.score(key).as_bytes()),
            "a6613ffdfb0ca0f4b40ace3f67bccf8d18440d9ec9c551ed8bbc0b66bfdb7a06"
        );
        assert_eq!(
            ContentDigest::of_canonical(&evidence.canonical_bytes()).to_string(),
            "86a0b003212f20e2a198c1612aab68b468579155f3034b1ec7c8571e50aea3de"
        );
        assert_eq!(evidence.winner(), contender(0x11, 0x21, 7));
    }

    #[test]
    fn rank_is_permutation_invariant() {
        let first = contender(0x11, 0x21, 7);
        let second = contender(0x12, 0x22, 8);
        let third = contender(0x13, 0x23, 9);

        let forward = must_rank(&[first, second, third]);
        let reversed = must_rank(&[third, second, first]);
        let rotated = must_rank(&[second, third, first]);

        assert_eq!(forward, reversed);
        assert_eq!(forward, rotated);
    }

    #[test]
    fn unrelated_draw_does_not_shift_existing_scores() {
        let first = contender(0x11, 0x21, 7);
        let second = contender(0x12, 0x22, 8);
        let oracle = oracle();
        let before = must_rank(&[first, second]);

        let unrelated =
            SemanticRandomKeyV1::containment_rank(group(0x77), contender(0x31, 0x41, 99));
        let _ = oracle.score(unrelated);

        let after = must_rank(&[first, second]);
        assert_eq!(before, after);
    }

    #[test]
    fn duplicate_semantic_key_is_rejected() {
        let duplicate = contender(0x11, 0x21, 7);
        let result = rank_containment_conflict(&oracle(), group(0x41), &[duplicate, duplicate]);

        assert!(matches!(
            result,
            Err(error)
                if matches!(
                    *error,
                    ContainmentRandomRankError::SemanticKeyReuse {
                        first,
                        second,
                        ..
                    } if first == duplicate && second == duplicate
                )
        ));
    }

    #[test]
    fn distinct_key_score_collision_is_rejected() {
        let first = contender(0x11, 0x21, 7);
        let second = contender(0x12, 0x22, 8);
        let result = rank_containment_conflict_with(group(0x41), &[first, second], |_| {
            RandomScore256::from_bytes([0x55; 32])
        });

        assert!(matches!(
            result,
            Err(error)
                if matches!(*error, ContainmentRandomRankError::ScoreCollision { .. })
        ));
    }

    #[test]
    fn semantic_fields_and_seed_change_scores() {
        let base_group = group(0x41);
        let base_contender = contender(0x11, 0x21, 7);
        let key = SemanticRandomKeyV1::containment_rank(base_group, base_contender);
        let base = oracle().score(key);

        let variants = [
            SemanticRandomKeyV1::containment_rank(
                ContainmentConflictGroupV1::new(moment(11, 3), base_group.resource()),
                base_contender,
            ),
            SemanticRandomKeyV1::containment_rank(group(0x42), base_contender),
            SemanticRandomKeyV1::containment_rank(
                ContainmentConflictGroupV1::new(
                    moment(11, 2),
                    ContainmentConflictResourceV1::DestinationCapacity(EntityId::from_bytes(
                        [0x41; 32],
                    )),
                ),
                base_contender,
            ),
            SemanticRandomKeyV1::containment_rank(base_group, contender(0x12, 0x21, 7)),
            SemanticRandomKeyV1::containment_rank(base_group, contender(0x11, 0x22, 7)),
            SemanticRandomKeyV1::containment_rank(base_group, contender(0x11, 0x21, 8)),
        ];

        for variant in variants {
            assert_ne!(oracle().score(variant), base);
        }
        let other_seed = Blake3KeyedPrf256V1::from_root_seed(RootSeed::from_bytes([0x5b; 32]));
        assert_ne!(other_seed.score(key), base);
    }
}
