use a4n_domain::manga::{MangaPageDrawingPromptV2, MangaPageGenerationSchemaVersion, MangaPageV3};
use serde::Deserialize;
use serde_json::Value;

pub use a4n_api_contract::manga::{
    BindMangaReferenceImageInput, DeleteMangaPromptInput, MangaChapterInput,
    MangaPagePromptSourceSummary, MangaPagePromptSummary, MangaPipelineStatusSummary,
    MangaReferenceBindingSummary, OptimizeMangaPromptInput, UpdateMangaPromptInput,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MangaPageGenerationLayoutV2 {
    #[serde(rename = "schema_version")]
    pub _schema_version: MangaPageGenerationSchemaVersion,
    pub shooting_script_id: i64,
    pub shooting_script_revision: i32,
    pub page_plan: MangaPageV3,
    pub drawing_prompt: MangaPageDrawingPromptV2,
    pub reference_bindings: Vec<MangaReferenceBindingSummary>,
    #[serde(rename = "prompt_fingerprint")]
    pub _prompt_fingerprint: Value,
    pub source: MangaPagePromptSourceSummary,
}
