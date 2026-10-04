use a4n_domain::generation::GenerationSpecRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(in crate::galgame) struct GalgameGenerationSpecRefsRecordInput {
    pub(in crate::galgame) text_anchor_spec_ids: Vec<i64>,
    pub(in crate::galgame) text_anchor_refs: Vec<GalgameGenerationSpecTextAnchorRefRecordInput>,
    pub(in crate::galgame) entity_ref_spec_ids: Vec<i64>,
    pub(in crate::galgame) entity_refs: Vec<GalgameGenerationSpecEntityRefRecordInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::galgame) struct GalgameGenerationSpecTextAnchorRefRecordInput {
    pub(in crate::galgame) generation_spec_id: i64,
    pub(in crate::galgame) text_anchor_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::galgame) struct GalgameGenerationSpecEntityRefRecordInput {
    pub(in crate::galgame) generation_spec_id: i64,
    pub(in crate::galgame) profile_id: i64,
}

#[derive(Debug, Clone)]
pub struct GalgameGenerationSpecUpsert {
    pub record: GenerationSpecRecord,
    pub outcome: GalgameGenerationSpecUpsertOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GalgameGenerationSpecUpsertOutcome {
    Created,
    Updated,
    Reused,
}
