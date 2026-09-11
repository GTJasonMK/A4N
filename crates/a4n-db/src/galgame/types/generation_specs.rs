use a4n_db_models::models::generation::generation_spec;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameGenerationSpecUpsertInput {
    pub canonical_key: String,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: Option<i64>,
    pub params_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameAssetPlanSpecRecordInput {
    pub spec: GalgameGenerationSpecUpsertInput,
    pub text_anchor_ids: Vec<i64>,
    pub replace_entity_refs: bool,
    pub entity_profile_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct GalgameGenerationSpecUpsert {
    pub model: generation_spec::Model,
    pub outcome: GalgameGenerationSpecUpsertOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GalgameGenerationSpecUpsertOutcome {
    Created,
    Updated,
    Reused,
}
