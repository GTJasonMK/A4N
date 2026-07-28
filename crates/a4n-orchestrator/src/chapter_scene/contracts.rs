use a4n_core::{
    error::AppError,
    models::chapter::{chapter, chapter_scene},
};
use a4n_llm::provider::ChatRequest;

pub type ChapterSceneProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Debug, Clone)]
pub struct ChapterSceneGenerationInput {
    pub chapter: chapter::Model,
    pub scene_number: i32,
    pub scene_prompt: String,
}

#[derive(Debug, Clone)]
pub struct ChapterSceneGenerationApplicationInput {
    pub chapter_id: i64,
    pub scene_number: i32,
    pub scene_prompt: String,
}

#[derive(Debug, Clone)]
pub struct ChapterSceneRequestPreview {
    pub chapter_number: i32,
    pub request: ChatRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChapterSceneTaskScope {
    pub project_id: i64,
    pub total: u32,
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
