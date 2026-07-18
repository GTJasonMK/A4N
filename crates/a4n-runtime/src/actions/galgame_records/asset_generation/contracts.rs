use serde::{Deserialize, Serialize};

pub(super) const GALGAME_ASSET_RENDERER_VERSION: &str = "galgame_asset_generation_v1";
pub(super) const GALGAME_PROMPT_KIND_BACKGROUND: &str = "background_image_prompt";
pub(super) const GALGAME_PROMPT_KIND_EVENT_CG: &str = "event_cg_prompt";
pub(super) const GALGAME_BACKGROUND_IMAGE_PROMPT_TEMPLATE: &str = "galgame_background_image_prompt";
pub(super) const GALGAME_EVENT_CG_PROMPT_TEMPLATE: &str = "galgame_event_cg_prompt";
pub(super) const GALGAME_CG_IMPORTANCE: &[&str] = &["normal", "high", "critical"];
pub(super) const GALGAME_PORTRAIT_VARIANT_STYLE: &str =
    "Galgame 视觉小说角色立绘表情差分；必须保持参考图中的角色身份、服装、体型比例和美术风格一致。";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunGalgameAssetGenerationInput {
    pub project_id: i64,
    #[serde(default)]
    pub generation_spec_ids: Vec<i64>,
    #[serde(default)]
    pub config_id: Option<i64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameAssetGenerationSummary {
    pub project_id: i64,
    pub generated_count: usize,
    pub skipped_completed_count: usize,
    pub items: Vec<GalgameAssetGenerationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameAssetGenerationItem {
    pub generation_spec_id: i64,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: i64,
    pub status: String,
    pub prompt_artifact_id: Option<i64>,
    pub generated_image_id: Option<i64>,
    pub portrait_id: Option<i64>,
    pub media_asset_id: Option<i64>,
    pub binding_id: Option<i64>,
}
