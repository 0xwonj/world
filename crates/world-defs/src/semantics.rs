use world_core::{DefinitionId, VersionAnchor};

use crate::error::{DefinitionError, empty_definition_field};
use crate::keys::DefinitionName;

/// Checked semantic declaration family.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticDeclarationKind {
    /// Interprets social or institutional meaning.
    SocialRule,
    /// Interprets chronology or world-context meaning.
    ChronologyRule,
    /// Interprets perception, memory, belief, or knowledge.
    EpistemicRule,
    /// Produces appraisal or motivation state.
    AppraisalRule,
    /// Defines a candidate intent template.
    IntentTemplate,
    /// Defines a derived semantic view.
    SemanticView,
}

/// Input surface a semantic declaration may read.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticInputKind {
    /// Committed hard event evidence.
    HardEventEvidence,
    /// Actor-visible social context.
    SocialContext,
    /// Accepted chronology or world-context records.
    ChronologyContext,
    /// Actor-relative memory, belief, knowledge, or observation context.
    EpistemicContext,
    /// Existing appraisal or motivation state.
    AppraisalState,
    /// Thought or pressure inputs from earlier appraisal.
    Pressure,
    /// Actor capabilities available to intent preparation.
    CapabilitySet,
    /// Actor-owned action repertoire.
    ActionRepertoire,
    /// Perceived target affordances.
    PerceivedAffordance,
    /// Active local, abstract, or strategic resolution context.
    ActiveResolution,
    /// Declared actor-relative context view.
    ActorContext,
}

/// Output family a semantic declaration may produce.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticOutputKind {
    /// Proposed social or institutional state update.
    SocialUpdateProposal,
    /// Proposed chronology or world-context record.
    ChronologyRecordProposal,
    /// Proposed memory, belief, knowledge, or observation update.
    EpistemicUpdateProposal,
    /// Interpreted thought.
    Thought,
    /// Motivational pressure.
    Pressure,
    /// Goal-directed motivational pressure.
    GoalPressure,
    /// Proposed appraisal record.
    AppraisalRecordProposal,
    /// Candidate intent.
    CandidateIntent,
    /// Feature used by intent scoring.
    IntentScoreFeature,
    /// Contract for lowering an intent into executable work.
    LoweringContract,
    /// Metadata used to prepare an activity.
    ActivityPreparation,
    /// Derived actor-relative context for later semantic stages.
    DerivedActorContext,
}

/// Checked semantic declaration envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticDeclarationDef {
    id: DefinitionId,
    name: DefinitionName,
    kind: SemanticDeclarationKind,
    inputs: Vec<SemanticInputKind>,
    outputs: Vec<SemanticOutputKind>,
    version: VersionAnchor,
}

impl SemanticDeclarationDef {
    /// Creates a semantic declaration when its outputs belong to its declaration kind.
    pub fn new(
        id: DefinitionId,
        name: DefinitionName,
        kind: SemanticDeclarationKind,
        inputs: impl IntoIterator<Item = SemanticInputKind>,
        outputs: impl IntoIterator<Item = SemanticOutputKind>,
        version: VersionAnchor,
    ) -> Result<Self, DefinitionError> {
        let outputs = outputs.into_iter().collect::<Vec<_>>();
        if outputs.is_empty() {
            return Err(empty_definition_field(
                id,
                "SemanticDeclarationDef",
                "outputs",
            ));
        }

        for output in &outputs {
            if !kind_allows_output(kind, *output) {
                return Err(DefinitionError::ForbiddenSemanticOutput {
                    definition: id,
                    kind,
                    output: *output,
                });
            }
        }

        Ok(Self {
            id,
            name,
            kind,
            inputs: inputs.into_iter().collect(),
            outputs,
            version,
        })
    }

    /// Returns the definition id.
    pub fn id(&self) -> DefinitionId {
        self.id
    }

    /// Returns the definition name.
    pub fn name(&self) -> &DefinitionName {
        &self.name
    }

    /// Returns the semantic declaration kind.
    pub fn kind(&self) -> SemanticDeclarationKind {
        self.kind
    }

    /// Returns semantic inputs read by this declaration.
    pub fn inputs(&self) -> &[SemanticInputKind] {
        &self.inputs
    }

    /// Returns semantic outputs produced by this declaration.
    pub fn outputs(&self) -> &[SemanticOutputKind] {
        &self.outputs
    }

    /// Returns the version anchor.
    pub fn version(&self) -> VersionAnchor {
        self.version
    }
}

fn kind_allows_output(kind: SemanticDeclarationKind, output: SemanticOutputKind) -> bool {
    match kind {
        SemanticDeclarationKind::SocialRule => {
            matches!(output, SemanticOutputKind::SocialUpdateProposal)
        }
        SemanticDeclarationKind::ChronologyRule => {
            matches!(output, SemanticOutputKind::ChronologyRecordProposal)
        }
        SemanticDeclarationKind::EpistemicRule => {
            matches!(output, SemanticOutputKind::EpistemicUpdateProposal)
        }
        SemanticDeclarationKind::AppraisalRule => matches!(
            output,
            SemanticOutputKind::Thought
                | SemanticOutputKind::Pressure
                | SemanticOutputKind::GoalPressure
                | SemanticOutputKind::AppraisalRecordProposal
        ),
        SemanticDeclarationKind::IntentTemplate => matches!(
            output,
            SemanticOutputKind::CandidateIntent
                | SemanticOutputKind::IntentScoreFeature
                | SemanticOutputKind::LoweringContract
                | SemanticOutputKind::ActivityPreparation
        ),
        SemanticDeclarationKind::SemanticView => {
            matches!(output, SemanticOutputKind::DerivedActorContext)
        }
    }
}
