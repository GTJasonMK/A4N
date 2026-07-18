use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_scene;
use a4n_llm::provider::ChatRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type ChapterSceneProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSceneSummary {
    pub id: i64,
    pub chapter_id: i64,
    pub scene_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub word_count: i32,
    pub summary: Option<String>,
    pub scene_type: String,
    pub characters_present: Option<Value>,
    pub location: Option<String>,
    pub timeline_position: Option<String>,
    pub transition_from_previous: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChapterSceneInput {
    pub scene_id: i64,
    pub content: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub scene_type: Option<String>,
    pub location: Option<String>,
    pub timeline_position: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReorderScenesInput {
    pub order: Vec<(i64, i32)>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterSceneInput {
    pub chapter_id: i64,
    pub scene_number: i32,
    pub scene_prompt: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ChapterSceneTaskScope {
    pub project_id: i64,
    pub total: u32,
}

pub(super) struct ChapterSceneGenerationRequestContext {
    pub(super) project_id: i64,
    pub(super) chapter_number: i32,
    pub(super) request: ChatRequest,
    pub(super) call_key: String,
}

pub(super) fn chapter_scene_summary(scene: &chapter_scene::Model) -> ChapterSceneSummary {
    ChapterSceneSummary {
        id: scene.id,
        chapter_id: scene.chapter_id,
        scene_number: scene.scene_number,
        title: scene.title.clone(),
        content: scene.content.clone(),
        word_count: scene.word_count,
        summary: scene.summary.clone(),
        scene_type: scene.scene_type.clone(),
        characters_present: scene.characters_present.clone(),
        location: scene.location.clone(),
        timeline_position: scene.timeline_position.clone(),
        transition_from_previous: scene.transition_from_previous.clone(),
        created_at: Some(scene.created_at.to_string()),
        updated_at: Some(scene.updated_at.to_string()),
    }
}
