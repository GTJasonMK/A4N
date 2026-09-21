use a4n_db_models::models::chapter::{chapter, chapter_scene};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;

pub(crate) use a4n_novel_host_contract::ChapterSceneRequestPreview;

pub(super) struct ChapterSceneGenerationInput {
    pub(super) chapter: chapter::Model,
    pub(super) scene_number: i32,
    pub(super) scene_prompt: String,
}

#[derive(Debug)]
pub(super) struct RequiredChapterSceneGenerationInput {
    pub(super) title: String,
    pub(super) summary: String,
    pub(super) scene_prompt: String,
}

pub(super) struct PreparedChapterSceneGeneration {
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) chapter_number: i32,
    pub(super) scene_number: i32,
    pub(super) scene_prompt: String,
    pub(super) call_key: String,
    pub(super) request: ChatRequest,
}

pub(super) type ChapterSceneGenerationResult = Result<chapter_scene::Model, AppError>;
