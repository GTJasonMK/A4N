use a4n_db::source_sync::{
    SourceSyncEntityRefRecordInput as EntityRefInput,
    SourceSyncMediaRefRecordInput as MediaRefInput,
    SourceSyncTextAnchorRefRecordInput as TextAnchorRefInput,
};
use a4n_prompt::rendering::PromptRenderer;
use serde::Deserialize;
use serde_json::Value;

pub(super) use a4n_api_contract::source_sync::SourceSyncDecisionInput;

#[derive(Clone, Copy, Default)]
pub struct SourceSyncExecutionContext<'a> {
    pub galgame: Option<GalgameSourceSyncExecutionContext<'a>>,
}

#[derive(Clone, Copy)]
pub struct GalgameSourceSyncExecutionContext<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub workflow_run_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RefreshSpecPayload {
    pub(super) generation_spec_id: i64,
    pub(super) prompt_kind: String,
    pub(super) params_json: Value,
    pub(super) text_anchor_refs: Vec<TextAnchorRefInput>,
    pub(super) entity_refs: Vec<EntityRefInput>,
    pub(super) media_refs: Vec<MediaRefInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RebindAnchorPayload {
    pub(super) generation_spec_id: i64,
    pub(super) anchor_mappings: Vec<AnchorMappingPayload>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AnchorMappingPayload {
    pub(super) old_text_anchor_id: i64,
    pub(super) new_text_anchor_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ManualPromptPayload {
    pub(super) prompt_artifact_id: i64,
    pub(super) prompt_text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReplacementSpecPayload {
    pub(super) generation_spec_id: i64,
    pub(super) prompt_kind: String,
    pub(super) params_json: Value,
    pub(super) text_anchor_refs: Vec<TextAnchorRefInput>,
    pub(super) entity_refs: Vec<EntityRefInput>,
    pub(super) media_refs: Vec<MediaRefInput>,
    pub(super) replacement_segment: ReplacementSegmentPayload,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReplacementSegmentPayload {
    pub(super) title: String,
    pub(super) duration_secs: f32,
    pub(super) order_index: i32,
    pub(super) previous_segment_id: Option<i64>,
    pub(super) next_segment_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RegenerateGalgameScenePayload {
    pub(super) galgame_scene_id: i64,
    pub(super) reason: Option<String>,
}
