use a4n_core::models::{ai::image_gen_config, chapter::chapter_manga_prompt};
use a4n_image::ImageGenerationRequest;
use a4n_orchestrator::project::ChapterMangaSourceSummary;
use serde_json::Value;

#[derive(Debug, Clone)]
pub(in crate::actions::image_generation) struct DirectImageGenerationInput {
    pub(in crate::actions::image_generation) project_id: i64,
    pub(in crate::actions::image_generation) prompt: String,
    pub(in crate::actions::image_generation) image_type: String,
    pub(in crate::actions::image_generation) config_id: Option<i64>,
    pub(in crate::actions::image_generation) chapter_id: Option<i64>,
    pub(in crate::actions::image_generation) panel_id: Option<String>,
    pub(in crate::actions::image_generation) metadata_json: Option<Value>,
    pub(in crate::actions::image_generation) workflow_run_id: Option<i64>,
    pub(in crate::actions::image_generation) command_name: String,
    pub(in crate::actions::image_generation) preview_workflow: String,
    pub(in crate::actions::image_generation) preview_title: String,
    pub(in crate::actions::image_generation) preview_step_key: String,
    pub(in crate::actions::image_generation) preview_step_title: String,
}

pub(in crate::actions::image_generation) struct SceneImageRequestContext {
    pub(in crate::actions::image_generation) request: ImageGenerationRequest,
    pub(in crate::actions::image_generation) config: image_gen_config::Model,
    pub(in crate::actions::image_generation) chapter_id: Option<i64>,
    pub(in crate::actions::image_generation) source_metadata: Option<Value>,
}

pub(in crate::actions::image_generation) struct PageImageRequestContext {
    pub(in crate::actions::image_generation) request: ImageGenerationRequest,
    pub(in crate::actions::image_generation) config: image_gen_config::Model,
    pub(in crate::actions::image_generation) chapter: ChapterMangaSourceSummary,
    pub(in crate::actions::image_generation) page_prompt: chapter_manga_prompt::Model,
    pub(in crate::actions::image_generation) panel_id: String,
    pub(in crate::actions::image_generation) reference_bindings: Option<Value>,
    pub(in crate::actions::image_generation) reference_image_count: usize,
    pub(in crate::actions::image_generation) reference_image_mode: String,
}
