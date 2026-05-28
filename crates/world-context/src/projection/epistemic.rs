use world_core::{DefinitionId, ProvenanceKey};
use world_model::{AcceptedRecordId, EpistemicHolder, EpistemicRecord};

use crate::{
    ContextProjectionCompleteness, ContextProjectionKind, ContextProvenanceSource,
    ContextReadDependency,
    context::ContextProjectionReportBuilder,
    request::{ActorContextInput, ActorContextRequest},
};

/// Actor-owned epistemic working set.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EpistemicWorkingSet {
    records: Vec<EpistemicContextRecord>,
}

impl EpistemicWorkingSet {
    /// Creates a working set from records already sorted by model order.
    #[must_use]
    pub(crate) fn new(records: Vec<EpistemicContextRecord>) -> Self {
        Self { records }
    }

    /// Returns whether no epistemic records were projected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns actor-visible epistemic records.
    #[must_use]
    pub fn records(&self) -> &[EpistemicContextRecord] {
        &self.records
    }
}

/// Value projection of an accepted epistemic record visible to the actor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpistemicContextRecord {
    id: AcceptedRecordId,
    holder: EpistemicHolder,
    definition: Option<DefinitionId>,
    provenance: Option<ProvenanceKey>,
}

impl EpistemicContextRecord {
    fn from_record(record: &EpistemicRecord) -> Self {
        Self {
            id: record.id(),
            holder: record.holder(),
            definition: record.definition(),
            provenance: record.provenance(),
        }
    }

    /// Returns the accepted record id.
    #[must_use]
    pub const fn id(&self) -> AcceptedRecordId {
        self.id
    }

    /// Returns the epistemic holder.
    #[must_use]
    pub const fn holder(&self) -> EpistemicHolder {
        self.holder
    }

    /// Returns the checked definition associated with this record, if any.
    #[must_use]
    pub const fn definition(&self) -> Option<DefinitionId> {
        self.definition
    }

    /// Returns model provenance associated with this record, if known.
    #[must_use]
    pub const fn provenance(&self) -> Option<ProvenanceKey> {
        self.provenance
    }
}

pub(crate) fn project(
    input: ActorContextInput<'_>,
    request: &ActorContextRequest,
    report: &mut ContextProjectionReportBuilder,
) -> EpistemicWorkingSet {
    report.push_status(
        ContextProjectionKind::Epistemic,
        ContextProjectionCompleteness::Complete,
    );

    let query = input.model().query_layer().actor_relative(request.actor());
    for read in query.read_labels() {
        report.insert_read(ContextReadDependency::Authority(read));
        report.insert_provenance(ContextProvenanceSource::QueryRead(read));
    }

    let records = query
        .epistemic_records()
        .map(|record| {
            report.insert_provenance(ContextProvenanceSource::AcceptedRecord(record.id()));
            if let Some(definition) = record.definition() {
                report.insert_read(ContextReadDependency::Definition(definition));
                report.insert_provenance(ContextProvenanceSource::Definition(definition));
            }
            if let Some(provenance) = record.provenance() {
                report.insert_provenance(ContextProvenanceSource::RecordProvenance(provenance));
            }
            EpistemicContextRecord::from_record(record)
        })
        .collect();

    EpistemicWorkingSet::new(records)
}
