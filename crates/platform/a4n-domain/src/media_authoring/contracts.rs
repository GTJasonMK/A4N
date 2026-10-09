use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MEDIA_AUTHORING_PIPELINE_VERSION: &str = "media_authoring_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationTarget {
    Manga,
    Galgame,
}

impl AdaptationTarget {
    pub const fn project_kind(self) -> &'static str {
        match self {
            Self::Manga => crate::project::ProjectKind::Comic.as_str(),
            Self::Galgame => crate::project::ProjectKind::Galgame.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AdaptationPlanVersion {
    #[serde(rename = "adaptation_plan_v1")]
    V1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationOrigin {
    Source,
    Adapted,
    Added,
    Historical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationImportance {
    Required,
    Important,
    Supporting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationBeatKind {
    Action,
    Dialogue,
    Reaction,
    Revelation,
    Transition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdaptationBeat {
    pub id: String,
    pub kind: AdaptationBeatKind,
    pub is_climax: bool,
    pub origin: AdaptationOrigin,
    pub importance: AdaptationImportance,
    pub action: String,
    pub reaction: Option<String>,
    pub dialogue_intent: Option<String>,
    pub entry_state: String,
    pub exit_state: String,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdaptationUnit {
    pub id: String,
    pub origin: AdaptationOrigin,
    pub title: String,
    pub location: Option<String>,
    pub participants: Vec<String>,
    pub dramatic_question: String,
    pub objective: String,
    pub obstacle: String,
    pub entry_state: String,
    pub exit_state: String,
    pub source_span_ids: Vec<String>,
    pub beats: Vec<AdaptationBeat>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdaptationDecision {
    pub description: String,
    pub reason: String,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdaptationPlanV1 {
    pub schema_version: AdaptationPlanVersion,
    pub target: AdaptationTarget,
    #[serde(default)]
    pub page_count: Option<u32>,
    pub intention: String,
    pub decisions: Vec<AdaptationDecision>,
    pub units: Vec<AdaptationUnit>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct AdaptationBeatRef {
    pub unit_id: String,
    pub beat_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdaptationUnitRef {
    pub source_chapter_id: i64,
    pub unit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgameBranchDesign {
    pub choices: Vec<GalgamePipelineChoicePlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgamePipelineChoicePlan {
    pub after_beat_ref: crate::galgame::GalgamePipelineBeatRefV4,
    pub question: String,
    pub options: Vec<GalgamePipelineChoiceOptionPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgamePipelineChoiceOptionPlan {
    pub branch_id: String,
    pub option_text: String,
    pub branch_title: String,
    pub branch_objective: String,
    pub dramatic_action: String,
    pub reaction: String,
    pub player_knowledge: Vec<String>,
    pub returns_to: AdaptationUnitRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringSourceRef {
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VideoAdaptationReference {
    /// A lookup hint for checking updates, never required to consume the snapshot.
    pub video_project_id: Option<i64>,
    pub project_title: String,
    pub source: AuthoringSourceRef,
    pub script_id: i64,
    pub script_revision: i32,
    pub script_sha256: String,
    pub units: Vec<AdaptationUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringDocumentInputs {
    pub sources: Vec<AuthoringSourceRef>,
    pub video_reference: Option<VideoAdaptationReference>,
    pub dependencies: Vec<AuthoringDocumentRef>,
    #[serde(default)]
    pub preprocessing_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringDocumentRef {
    pub id: i64,
    pub revision: i32,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringDocumentDeletionImpact {
    pub dependent_documents: Vec<AuthoringDocumentDeletionDependency>,
    pub manga_pages: Vec<AuthoringMangaPageDeletion>,
    pub galgame_script: Option<AuthoringGalgameScriptDeletion>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringDocumentDeletionDependency {
    pub id: i64,
    pub kind: AuthoringDocumentKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringMangaPageDeletion {
    pub chapter_id: i64,
    pub page_number: i32,
    pub image_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringGalgameScriptDeletion {
    pub scene_count: usize,
    pub line_count: usize,
    pub choice_count: usize,
    pub story_graph_revision_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthoringDocumentKind {
    AdaptationPlan,
    GalgameBranchPlan,
    MangaPagePlan,
}

impl AuthoringDocumentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdaptationPlan => "adaptation_plan",
            Self::GalgameBranchPlan => "galgame_branch_plan",
            Self::MangaPagePlan => "manga_page_plan",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthoringDocumentStatus {
    Draft,
    Adopted,
    Superseded,
}

impl AuthoringDocumentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Adopted => "adopted",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringDocumentRecord {
    pub id: i64,
    pub project_id: i64,
    pub source_snapshot_id: i64,
    pub source_chapter_id: Option<i64>,
    pub kind: AuthoringDocumentKind,
    pub version: i32,
    pub revision: i32,
    pub status: AuthoringDocumentStatus,
    pub inputs: AuthoringDocumentInputs,
    pub document: serde_json::Value,
    pub sha256: String,
}

impl AuthoringDocumentRecord {
    pub fn reference(&self) -> AuthoringDocumentRef {
        AuthoringDocumentRef {
            id: self.id,
            revision: self.revision,
            sha256: self.sha256.clone(),
        }
    }
}
