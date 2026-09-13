use super::{embedding::EmbeddingModelIdentity, generation_tasks::GenerationStartRequestMeta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CHAPTER_ANALYSIS_PROGRESS_TOTAL: u32 = 100;
pub const CHAPTER_SCENE_PROGRESS_TOTAL: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparedGenerationRagEmbeddingRequest {
    pub query_texts: Vec<String>,
    pub embedding_identity: EmbeddingModelIdentity,
    pub query_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreviewChapterPromptInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    pub writing_notes: Option<String>,
    #[schemars(range(min = 1))]
    pub min_word_count: Option<i32>,
    #[schemars(range(min = 1))]
    pub max_word_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImportChapterInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateChapterContentInput {
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterAnalysisInput {
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq)]
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
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerateChapterSceneInput {
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
    #[schemars(range(min = 1))]
    pub scene_number: i32,
    pub scene_prompt: String,
}

/// Host callbacks for chapter-derived cleanup and RAG synchronization.
///
/// File, RAG and derived-artifact side effects stay with the Host; chapter
/// owners only validate and persist their durable state.
#[async_trait::async_trait]
pub trait ChapterDerivedCleanupHostPort: Send + Sync {
    fn validate_chapter_cleanup_artifacts(
        &self,
        artifacts: &a4n_domain::project::ChapterDerivedCleanupArtifacts,
    ) -> Result<(), a4n_error::AppError>;

    async fn finish_chapter_derived_cleanup(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        artifacts: a4n_domain::project::ChapterDerivedCleanupArtifacts,
    ) -> Result<(), a4n_error::AppError>;

    async fn sync_chapter_rag_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_run_id: i64,
    ) -> Result<(), a4n_error::AppError>;
}

/// Host callback for chapter-scene generation progress.
pub type ChapterSceneProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), a4n_error::AppError> + Send + Sync);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateChapterSceneInput {
    #[schemars(range(min = 1))]
    pub scene_id: i64,
    pub content: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub scene_type: Option<String>,
    pub location: Option<String>,
    pub timeline_position: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReorderChapterScenesInput {
    #[schemars(length(min = 1))]
    pub order: Vec<(i64, i32)>,
}

/// Host-facing chapter scene boundary.
///
/// Authorization, scene persistence and ordering remain owned by the chapter
/// scene application; Runtime receives only API DTOs.
#[async_trait::async_trait]
pub trait ChapterSceneApplicationPort: Send + Sync {
    async fn list(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterSceneSummary>, a4n_error::AppError>;

    async fn update(
        &self,
        user_id: i64,
        input: UpdateChapterSceneInput,
    ) -> Result<ChapterSceneSummary, a4n_error::AppError>;

    async fn delete(&self, user_id: i64, scene_id: i64) -> Result<(), a4n_error::AppError>;

    async fn reorder(
        &self,
        user_id: i64,
        input: ReorderChapterScenesInput,
    ) -> Result<Vec<ChapterSceneSummary>, a4n_error::AppError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChapterMaterialSyncStatus {
    Completed,
    NotAnalyzed,
    NeedsAnalysis,
    Pending,
    Running,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ChapterMaterialSyncState {
    pub content_index: ChapterMaterialSyncStatus,
    pub outline_and_foreshadowing: ChapterMaterialSyncStatus,
    pub objects: ChapterMaterialSyncStatus,
    pub characters: ChapterMaterialSyncStatus,
    pub scenes: ChapterMaterialSyncStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChapterMaterialUpdateKind {
    Outline,
    Foreshadowing,
    Object,
    Character,
    Scene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChapterMaterialUpdateAction {
    Created,
    Updated,
    Existing,
    Preserved,
    Synced,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ChapterMaterialUpdate {
    pub material_kind: ChapterMaterialUpdateKind,
    pub action: ChapterMaterialUpdateAction,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq)]
pub struct ChapterSummary {
    pub id: i64,
    pub project_id: i64,
    pub outline_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub word_count: i32,
    pub status: String,
    pub summary: Option<String>,
    pub style_tags: Option<Value>,
    pub analysis_data: Option<Value>,
    pub analysis_sync_status: Option<String>,
    pub analysis_sync_can_resume: bool,
    pub analysis_sync_has_saved_analysis: bool,
    pub analysis_sync_resume_block_reason: Option<String>,
    pub analysis_sync_error: Option<String>,
    pub derived_sync_status: Option<String>,
    pub derived_sync_workflow_run_id: Option<i64>,
    pub derived_sync_can_resume: bool,
    pub derived_sync_error: Option<String>,
    pub material_sync: ChapterMaterialSyncState,
    pub material_updates: Vec<ChapterMaterialUpdate>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Host-facing chapter content read boundary.
///
/// Chapter persistence, workflow status projection and project visibility stay
/// with the chapter owner; Runtime only routes complete API summaries.
#[async_trait::async_trait]
pub trait ChapterContentReadApplicationPort: Send + Sync {
    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterSummary>, a4n_error::AppError>;

    async fn summary(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterSummary, a4n_error::AppError>;
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq)]
pub struct ChapterAnalysisMutationResult {
    pub chapter: ChapterSummary,
    pub warnings: Vec<String>,
}

/// Start a chapter body generation through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartChapterGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    #[schemars(length(max = 32768))]
    pub writing_notes: Option<String>,
    #[schemars(range(min = 1))]
    pub min_word_count: Option<i32>,
    #[schemars(range(min = 1))]
    pub max_word_count: Option<i32>,
    pub client_rag_embedding_identity: Option<EmbeddingModelIdentity>,
    #[schemars(length(equal = 64))]
    pub client_rag_query_fingerprint: Option<String>,
    #[schemars(length(max = 7))]
    pub client_rag_query_vectors: Option<Vec<Vec<f32>>>,
}

#[cfg(test)]
mod tests {
    use super::{ChapterAnalysisInput, GenerateChapterSceneInput, ImportChapterInput};

    #[test]
    fn import_chapter_rejects_removed_manual_flag() {
        let error = serde_json::from_value::<ImportChapterInput>(serde_json::json!({
            "project_id": 1,
            "chapter_number": 1,
            "content": "正文",
            "title": "第一章",
            "manual": true,
        }))
        .expect_err("removed manual flag must not be ignored");

        assert!(error.to_string().contains("unknown field `manual`"));
    }

    #[test]
    fn chapter_analysis_rejects_removed_identity_field() {
        let error = serde_json::from_value::<ChapterAnalysisInput>(serde_json::json!({
            "chapter_id": 1,
            "user_id": 7,
        }))
        .expect_err("caller identity must not enter the chapter analysis input");

        assert!(error.to_string().contains("unknown field `user_id`"));
    }

    #[test]
    fn chapter_scene_generation_rejects_removed_identity_field() {
        let error = serde_json::from_value::<GenerateChapterSceneInput>(serde_json::json!({
            "chapter_id": 1,
            "scene_number": 2,
            "scene_prompt": "继续推进冲突",
            "user_id": 7,
        }))
        .expect_err("caller identity must not enter the chapter scene input");

        assert!(error.to_string().contains("unknown field `user_id`"));
    }
}
