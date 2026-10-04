use serde_json::Value;

use super::CharacterProfileRecord;
use crate::generation::RecordCompletedGenerationRunInput;

#[derive(Debug, Clone)]
pub enum CharacterDiscoveryCandidateRecordInput {
    ExistingProfile {
        profile_id: i64,
        evidence_anchor_id: i64,
    },
    ExistingWithoutEvidence,
    PreviouslyResolvedCandidate {
        candidate_index: usize,
        evidence_anchor_id: i64,
    },
    Create {
        name: String,
        base_attributes: Value,
        evidence_anchor_id: i64,
        generation_run: RecordCompletedGenerationRunInput,
    },
}

#[derive(Debug, Clone)]
pub struct CharacterDiscoveryCandidateRecordOutcome {
    pub profile: Option<CharacterProfileRecord>,
    pub created: bool,
}
