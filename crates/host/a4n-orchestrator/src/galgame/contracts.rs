use super::{GalgamePipelinePromptChapter, GalgamePipelineSourceSpan};
use a4n_domain::media_authoring::AdaptationPlanV1;
use serde::Serialize;

pub(crate) use a4n_api_contract::galgame::{
    BindGalgameMediaInput, GalgameAssetGenerationItem, GalgameAssetGenerationSummary,
    GalgameAssetPlan, GalgameChoice, GalgameLine, GalgameMediaAsset, GalgameMediaBinding,
    GalgameMediaBindingSummary, GalgameScene, GalgameSceneDetail, GalgameScriptDetail,
    GalgameSpeakerProfile, GalgameTextAnchorSummary, ImportGalgameAudioInput,
    InsertGalgameLineInput, ReorderGalgameLinesInput, RunGalgameAssetGenerationInput,
    UnbindGalgameMediaInput, UpdateGalgameChoicesInput, UpdateGalgameLineInput,
    UpdateGalgameSceneInput,
};
pub(crate) use a4n_domain::galgame::{
    GALGAME_ENTITY_CHARACTER_PROFILE, GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE,
    GALGAME_EXPRESSIONS, GALGAME_PIPELINE_TOTAL_STEPS, GALGAME_TARGET_BACKGROUND,
    GALGAME_TARGET_CG, GALGAME_TARGET_PORTRAIT_EXPRESSION, GalgameAudioRole, GalgameCgImportance,
    GalgameGenerationTargetKind, GalgameLinePresentationDocumentV3,
    GalgameLinePresentationSchemaVersion, GalgameLinePresentationV2, GalgameLineType,
    GalgameMediaRole, GalgameMediaTargetKind, GalgamePipelineBeatRefV4, GalgamePipelineStartStage,
    GalgameSceneEntryKind, GalgameStagePositionV2, GalgameTargetEntityKind,
    GalgameVisibleCastMemberV2, LINE_TYPE_CHOICE, LINE_TYPE_DIALOGUE, LINE_TYPE_MONOLOGUE,
    LINE_TYPE_NARRATION, LINE_TYPE_STAGE_DIRECTION, SCENE_ENTRY_BRANCH, SCENE_ENTRY_MAIN,
};

pub(super) const SCRIPT_STATUS_READY: &str = "ready";

#[derive(Debug, Clone, Serialize)]
pub struct GalgamePipelinePromptContext {
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: i64,
    pub chapters: Vec<GalgamePipelinePromptChapter>,
    pub source_spans: Vec<GalgamePipelineSourceSpan>,
    pub speaker_profiles: Vec<GalgameSpeakerProfile>,
    pub adaptations: Vec<GalgameChapterAdaptation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GalgameChapterAdaptation {
    pub source_chapter_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub chapter_number: i32,
    pub adaptation_plan_id: i64,
    pub adaptation_plan_revision: i32,
    pub plan: AdaptationPlanV1,
}
