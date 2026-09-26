use a4n_db::galgame::{GalgameAssetPlanSpecRecordInput, GalgameGenerationSpecUpsertInput};
use serde_json::Value;

pub(super) struct GalgameSpecInput {
    pub(super) canonical_key: String,
    pub(super) project_id: i64,
    pub(super) root_project_id: i64,
    pub(super) source_snapshot_id: i64,
    pub(super) source_chapter_id: Option<i64>,
    pub(super) source_snapshot_chapter_id: Option<i64>,
    pub(super) target_kind: &'static str,
    pub(super) target_entity_kind: &'static str,
    pub(super) target_entity_id: Option<i64>,
    pub(super) params_json: Value,
}

impl GalgameSpecInput {
    pub(super) fn into_record_input(self) -> GalgameGenerationSpecUpsertInput {
        GalgameGenerationSpecUpsertInput {
            canonical_key: self.canonical_key,
            project_id: self.project_id,
            root_project_id: self.root_project_id,
            source_snapshot_id: self.source_snapshot_id,
            source_chapter_id: self.source_chapter_id,
            source_snapshot_chapter_id: self.source_snapshot_chapter_id,
            target_kind: self.target_kind.to_string(),
            target_entity_kind: self.target_entity_kind.to_string(),
            target_entity_id: self.target_entity_id,
            params_json: self.params_json,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum GalgameAssetPlanGroup {
    Background,
    PortraitExpression,
    EventCg,
}

#[derive(Debug, Clone)]
pub(super) struct GalgameAssetPlanItemDraft {
    pub(super) group: GalgameAssetPlanGroup,
    pub(super) canonical_key: String,
    pub(super) label: String,
    pub(super) source_label: String,
    pub(super) record_input: GalgameAssetPlanSpecRecordInput,
}
