use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use a4n_domain::media_authoring::{
    AdaptationBeat, AdaptationBeatKind, AdaptationBeatRef, AdaptationDecision,
    AdaptationImportance, AdaptationOrigin, AdaptationPlanV1, AdaptationPlanVersion,
    AdaptationTarget, AdaptationUnit, AdaptationUnitRef, AuthoringDocumentInputs,
    AuthoringDocumentKind, AuthoringDocumentRecord, AuthoringDocumentRef, AuthoringDocumentStatus,
    AuthoringSourceRef, GalgameBranchDesign, GalgamePipelineChoiceOptionPlan,
    GalgamePipelineChoicePlan, VideoAdaptationReference,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MediaAuthoringInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveAuthoringDocumentInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub kind: AuthoringDocumentKind,
    pub record_id: Option<i64>,
    pub expected_revision: Option<i32>,
    pub document: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdoptAuthoringDocumentInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub record_id: i64,
    #[schemars(range(min = 1))]
    pub expected_revision: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VideoScriptSelection {
    #[schemars(range(min = 1))]
    pub video_project_id: i64,
    #[schemars(range(min = 1))]
    pub script_id: i64,
    #[schemars(range(min = 1))]
    pub revision: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateAdaptationPlanInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
    #[schemars(length(min = 1))]
    pub task_id: String,
    #[schemars(length(min = 1))]
    pub instruction: String,
    pub video_reference: Option<VideoScriptSelection>,
    #[serde(default)]
    pub clear_video_reference: bool,
    pub record_id: Option<i64>,
    pub expected_revision: Option<i32>,
    #[serde(default)]
    pub resume_from_checkpoint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuthoringSourceSpan {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuthoringChapter {
    pub source: AuthoringSourceRef,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub source_spans: Vec<AuthoringSourceSpan>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VideoReferenceStatus {
    Current,
    Updated,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuthoringDocumentView {
    pub record: AuthoringDocumentRecord,
    pub stale: bool,
    pub video_reference_status: Option<VideoReferenceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaAuthoringWorkspace {
    pub target: AdaptationTarget,
    pub source_snapshot_id: i64,
    pub chapters: Vec<AuthoringChapter>,
    pub documents: Vec<AuthoringDocumentView>,
    pub artifacts: Vec<MediaAuthoringArtifactView>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MediaAuthoringArtifactKind {
    MangaPages,
    GalgameScript,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaAuthoringArtifactView {
    pub kind: MediaAuthoringArtifactKind,
    pub source_chapter_id: Option<i64>,
    pub count: usize,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VideoScriptReferenceCandidate {
    pub selection: VideoScriptSelection,
    pub project_title: String,
    pub chapter_number: i32,
    pub source: AuthoringSourceRef,
    pub same_source_content: bool,
}

#[async_trait::async_trait]
pub trait MediaAuthoringApplicationPort: Send + Sync {
    async fn workspace(
        &self,
        user_id: i64,
        input: MediaAuthoringInput,
    ) -> Result<MediaAuthoringWorkspace, a4n_error::AppError>;
    async fn save(
        &self,
        user_id: i64,
        input: SaveAuthoringDocumentInput,
    ) -> Result<AuthoringDocumentRecord, a4n_error::AppError>;
    async fn adopt(
        &self,
        user_id: i64,
        input: AdoptAuthoringDocumentInput,
    ) -> Result<AuthoringDocumentRecord, a4n_error::AppError>;
    async fn video_references(
        &self,
        user_id: i64,
        input: MediaAuthoringInput,
    ) -> Result<Vec<VideoScriptReferenceCandidate>, a4n_error::AppError>;
    async fn generate(
        &self,
        user_id: i64,
        input: GenerateAdaptationPlanInput,
        renderer: &dyn crate::prompt::PromptRenderPort,
    ) -> Result<AuthoringDocumentRecord, a4n_error::AppError>;
    async fn preview(
        &self,
        user_id: i64,
        input: GenerateAdaptationPlanInput,
        renderer: &dyn crate::prompt::PromptRenderPort,
    ) -> Result<Value, a4n_error::AppError>;
}
