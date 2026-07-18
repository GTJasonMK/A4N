use a4n_core::models::{generation::generation_spec, project::text_anchor};

#[derive(Debug, Clone)]
pub struct SourceSyncProfileGenerationSpecRefRecord {
    pub spec: generation_spec::Model,
    pub entity_role: String,
}

#[derive(Debug, Clone)]
pub struct SourceSyncAnchorConflictCandidateRecord {
    pub spec: generation_spec::Model,
    pub local_state: String,
    pub text_anchor_role: String,
    pub old_anchor: text_anchor::Model,
    pub candidates: Vec<text_anchor::Model>,
}
