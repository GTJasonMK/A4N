//! Galgame script service.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::{
    character::protagonist_profile,
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor, prompt_artifact,
    },
    media::{media_asset, media_binding, media_provenance},
    project::{source_snapshot_chapter, text_anchor},
};
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::generation::SPEC_STATUS_DRAFT;
use crate::media::{MEDIA_BINDING_ROLE_BACKGROUND, MEDIA_BINDING_ROLE_CG};
use crate::project_family::{ProjectFamilyContext, load_derived_project_context};
use crate::source::source_content_sha256;

mod pipeline_json;
mod pipeline_persist;
mod pipeline_prompt;
mod pipeline_run;
mod pipeline_state;

pub use self::pipeline_json::{
    GalgameBranchDesign, GalgameCharacterStaging, GalgamePipelineBackgroundPlan,
    GalgamePipelineChoiceOptionPlan, GalgamePipelineChoicePlan, GalgamePipelineEventCgPlan,
    GalgamePipelineLineDraft, GalgamePipelineLineStaged, GalgamePipelineScene,
    GalgamePipelineSceneLineDraft, GalgamePipelineSceneLineStaged, GalgameSceneSegmentation,
    GalgameScriptAdaptation, GalgameVisualPlan, branch_design_from_value,
    character_staging_from_value, parse_galgame_json_object, scene_segmentation_from_value,
    script_adaptation_from_value, visual_plan_from_value,
};
pub use self::pipeline_persist::{
    GalgamePersistLine, GalgamePersistScene, GalgamePipelineSourceSpan, GalgameScriptPersistPlan,
    build_galgame_script_persist_plan,
};
pub use self::pipeline_prompt::{
    GalgamePipelinePromptChapter, GalgamePipelinePromptSourceSpan,
    galgame_branch_design_user_message, galgame_character_staging_user_message,
    galgame_pipeline_source_spans_from_prompt_chapters, galgame_prompt_chapter_from_snapshot,
    galgame_scene_segmentation_user_message, galgame_script_adaptation_user_message,
    galgame_speaker_profiles_from_models, galgame_visual_planning_user_message,
};
pub(crate) use self::pipeline_run::validate_source_sync_scene_regeneration_stage_outputs;
pub use self::pipeline_run::{
    GalgamePipelineProgressUpdate, GalgamePipelineRunOptions, GalgameProgressReporter,
    GalgameSceneRegenerationInput, GalgameSceneRegenerationPlan,
};
pub use self::pipeline_state::{
    GALGAME_PIPELINE_TOTAL_STEPS, GalgamePipelineStartStage, galgame_pipeline_stage_keys,
    parse_galgame_start_stage, should_restore_galgame_stage_checkpoint,
};

pub const PROJECT_KIND_GALGAME: &str = "galgame";
const SCRIPT_STATUS_READY: &str = "ready";
pub const SCENE_ENTRY_MAIN: &str = "main";
pub const SCENE_ENTRY_BRANCH: &str = "branch";
pub const LINE_TYPE_NARRATION: &str = "narration";
pub const LINE_TYPE_DIALOGUE: &str = "dialogue";
pub const LINE_TYPE_MONOLOGUE: &str = "monologue";
pub const LINE_TYPE_STAGE_DIRECTION: &str = "stage_direction";
pub const LINE_TYPE_CHOICE: &str = "choice";
pub const GALGAME_TARGET_BACKGROUND: &str = "galgame_background";
pub const GALGAME_TARGET_CG: &str = "galgame_cg";
pub const GALGAME_TARGET_PORTRAIT_EXPRESSION: &str = "portrait_expression";
const GALGAME_CG_IMPORTANCE: &[&str] = &["normal", "high", "critical"];
pub const GALGAME_ENTITY_SCENE: &str = "galgame_scene";
pub const GALGAME_ENTITY_LINE: &str = "galgame_line";
pub const GALGAME_ENTITY_CHARACTER_PROFILE: &str = "character_profile";
pub const GALGAME_EXPRESSIONS: &[&str] = &[
    "neutral",
    "smile",
    "sad",
    "angry",
    "surprised",
    "shy",
    "serious",
];
const SNAPSHOT_BACKED_SOURCE_CONTEXT: &str = "snapshot_backed";

const LINE_TYPES: &[&str] = &[
    LINE_TYPE_NARRATION,
    LINE_TYPE_DIALOGUE,
    LINE_TYPE_MONOLOGUE,
    LINE_TYPE_STAGE_DIRECTION,
    LINE_TYPE_CHOICE,
];
const POSITIONS: &[&str] = &["left", "center", "right"];

#[derive(Clone)]
pub struct GalgameService {
    db: Arc<DatabaseConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameScriptDetail {
    pub script: galgame_script::Model,
    pub speaker_profiles: Vec<GalgameSpeakerProfile>,
    pub text_anchors: Vec<GalgameTextAnchorSummary>,
    pub media_bindings: Vec<GalgameMediaBindingSummary>,
    pub scenes: Vec<GalgameSceneDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameSpeakerProfile {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameTextAnchorSummary {
    pub id: i64,
    pub source_snapshot_chapter_id: Option<i64>,
    pub paragraph_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    pub quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameSceneDetail {
    pub scene: galgame_scene::Model,
    pub lines: Vec<galgame_line::Model>,
    pub choices: Vec<galgame_choice::Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameMediaBindingSummary {
    pub binding: media_binding::Model,
    pub asset: media_asset::Model,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameAssetPlan {
    pub summary: GalgameAssetPlanSummary,
    pub backgrounds: Vec<GalgameAssetPlanItem>,
    pub portrait_expressions: Vec<GalgameAssetPlanItem>,
    pub event_cgs: Vec<GalgameAssetPlanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GalgameAssetPlanSummary {
    pub background_specs: usize,
    pub portrait_expression_specs: usize,
    pub event_cg_specs: usize,
    pub created_specs: usize,
    pub updated_specs: usize,
    pub reused_specs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameAssetPlanItem {
    pub canonical_key: String,
    pub label: String,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: i64,
    pub generation_spec_id: i64,
    pub status: String,
    pub outcome: GalgameAssetPlanOutcome,
    pub params_json: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct GalgamePipelinePromptContext {
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: i64,
    pub chapters: Vec<GalgamePipelinePromptChapter>,
    pub source_spans: Vec<GalgamePipelineSourceSpan>,
    pub speaker_profiles: Vec<GalgameSpeakerProfile>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GalgameAssetPlanOutcome {
    Created,
    Updated,
    Reused,
}

struct GalgameSpecUpsert {
    model: generation_spec::Model,
    outcome: GalgameAssetPlanOutcome,
}

struct GalgameSpecInput {
    canonical_key: String,
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
    source_chapter_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
    target_kind: &'static str,
    target_entity_kind: &'static str,
    target_entity_id: Option<i64>,
    params_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameLineInput {
    pub line_id: i64,
    pub line_type: Option<String>,
    pub speaker_profile_id: Option<Option<i64>>,
    pub text: Option<String>,
    pub expression: Option<Option<String>>,
    pub pose: Option<Option<String>>,
    pub position: Option<Option<String>>,
    pub presentation_json: Option<Value>,
    pub text_anchor_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameSceneInput {
    pub scene_id: i64,
    pub title: Option<String>,
    pub location: Option<Option<String>>,
    pub time_of_day: Option<Option<String>>,
    pub background_note: Option<Option<String>>,
    pub next_scene_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InsertGalgameLineInput {
    pub scene_id: i64,
    pub after_line_id: Option<i64>,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub position: Option<String>,
    pub presentation_json: Option<Value>,
    pub text_anchor_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReorderGalgameLinesInput {
    pub scene_id: i64,
    pub line_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameChoicesInput {
    pub line_id: i64,
    pub choices: Vec<UpdateGalgameChoiceOptionInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameChoiceOptionInput {
    pub option_text: String,
    pub target_scene_id: i64,
}

impl GalgameService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn load_pipeline_prompt_context(
        &self,
        project_id: i64,
    ) -> Result<GalgamePipelinePromptContext, AppError> {
        validate_positive_id(project_id, "project_id")?;
        let context = self.load_galgame_context(project_id).await?;
        let chapters = source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::SnapshotId.eq(context.source_snapshot.id))
            .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if chapters.is_empty() {
            return Err(AppError::InvalidState {
                current: "source_snapshot_has_no_chapters".to_string(),
                operation: "load_galgame_pipeline_prompt_context".to_string(),
            });
        }
        let chapter_ids = chapters
            .iter()
            .map(|chapter| chapter.id)
            .collect::<Vec<_>>();
        let mut anchors_by_chapter = text_anchor::Entity::find()
            .filter(text_anchor::Column::ProjectId.eq(context.root_project.id))
            .filter(text_anchor::Column::SourceSnapshotId.eq(context.source_snapshot.id))
            .filter(text_anchor::Column::SourceSnapshotChapterId.is_in(chapter_ids))
            .order_by_asc(text_anchor::Column::SourceSnapshotChapterId)
            .order_by_asc(text_anchor::Column::StartOffset)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .fold(
                BTreeMap::<i64, Vec<text_anchor::Model>>::new(),
                |mut acc, anchor| {
                    if let Some(chapter_id) = anchor.source_snapshot_chapter_id {
                        acc.entry(chapter_id).or_default().push(anchor);
                    }
                    acc
                },
            );
        let mut prompt_chapters = Vec::with_capacity(chapters.len());
        for chapter in &chapters {
            let anchors = anchors_by_chapter.remove(&chapter.id).unwrap_or_default();
            prompt_chapters.push(galgame_prompt_chapter_from_snapshot(chapter, &anchors)?);
        }
        let source_spans = galgame_pipeline_source_spans_from_prompt_chapters(&prompt_chapters);
        Ok(GalgamePipelinePromptContext {
            project_id: context.project.id,
            root_project_id: context.root_project.id,
            source_snapshot_id: context.source_snapshot.id,
            speaker_profiles: self.speaker_profiles(context.root_project.id).await?,
            chapters: prompt_chapters,
            source_spans,
        })
    }

    pub async fn persist_pipeline_script(
        &self,
        project_id: i64,
        plan: GalgameScriptPersistPlan,
    ) -> Result<GalgameScriptDetail, AppError> {
        validate_positive_id(project_id, "project_id")?;
        if plan.scenes.is_empty() {
            return Err(AppError::InvalidArgument(
                "Galgame persist_script 至少需要一个场".to_string(),
            ));
        }
        let context = self.load_galgame_context(project_id).await?;
        let validation_scope = self.load_persist_validation_scope(&plan).await?;
        for scene in &plan.scenes {
            validate_positive_id(i64::from(scene.scene_index), "scene_index")?;
            validate_positive_id(
                scene.source_snapshot_chapter_id,
                "source_snapshot_chapter_id",
            )?;
            validation_scope
                .validate_source_snapshot_chapter(&context, scene.source_snapshot_chapter_id)?;
            validate_scene_entry_kind(&scene.entry_kind)?;
            if scene.next_scene_index == Some(scene.scene_index) {
                return Err(AppError::InvalidArgument(format!(
                    "Galgame 场 next_scene_index 不能指向自身: scene_index={}",
                    scene.scene_index
                )));
            }
            validation_scope
                .validate_scene_participants_json(&context, &scene.participants_json)?;
            for line in &scene.lines {
                validate_positive_id(i64::from(line.line_index), "line_index")?;
                validate_line_semantics(
                    &line.line_type,
                    line.speaker_profile_id,
                    line.expression.as_deref(),
                )?;
                ensure_json_object(&line.presentation_json, "presentation_json")?;
                if let Some(speaker_profile_id) = line.speaker_profile_id {
                    validation_scope.validate_speaker_profile(&context, speaker_profile_id)?;
                }
                if let Some(text_anchor_id) = line.text_anchor_id {
                    validation_scope.validate_text_anchor(
                        &context,
                        text_anchor_id,
                        Some(scene.source_snapshot_chapter_id),
                    )?;
                }
            }
        }

        let tx = self.db.begin().await.map_err(AppError::Database)?;
        delete_galgame_target_generation_media_graph(&tx, context.project.id).await?;
        delete_script_for_project(&tx, context.project.id).await?;
        let now = Utc::now().naive_utc();
        let script = galgame_script::ActiveModel {
            project_id: Set(context.project.id),
            root_project_id: Set(context.root_project.id),
            source_snapshot_id: Set(context.source_snapshot.id),
            title: Set(context.project.title.clone()),
            style_config_json: Set(json!({
                "pipeline": "galgame_v1",
            })),
            status: Set(SCRIPT_STATUS_READY.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(AppError::Database)?;

        let mut scene_id_by_index = BTreeMap::<i32, i64>::new();
        let mut scene_entry_kind_by_index = BTreeMap::<i32, String>::new();
        let mut pending_scene_next = Vec::new();
        let mut pending_choices = Vec::new();
        for scene_plan in plan.scenes {
            let scene_index = scene_plan.scene_index;
            let next_scene_index = scene_plan.next_scene_index;
            let entry_kind = scene_plan.entry_kind;
            let scene = galgame_scene::ActiveModel {
                script_id: Set(script.id),
                source_snapshot_chapter_id: Set(scene_plan.source_snapshot_chapter_id),
                scene_index: Set(scene_index),
                title: Set(required_text(&scene_plan.title, "scene.title")?.to_string()),
                location: Set(normalize_optional_text(scene_plan.location)),
                time_of_day: Set(normalize_optional_text(scene_plan.time_of_day)),
                participants_json: Set(scene_plan.participants_json),
                background_note: Set(normalize_optional_text(scene_plan.background_note)),
                entry_kind: Set(entry_kind.clone()),
                next_scene_id: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&tx)
            .await
            .map_err(AppError::Database)?;

            scene_id_by_index.insert(scene_index, scene.id);
            scene_entry_kind_by_index.insert(scene_index, entry_kind);
            if let Some(next_scene_index) = next_scene_index {
                pending_scene_next.push((scene.id, scene_index, next_scene_index));
            }

            for line_plan in scene_plan.lines {
                let choices = line_plan.choices;
                let line = galgame_line::ActiveModel {
                    scene_id: Set(scene.id),
                    line_index: Set(line_plan.line_index),
                    line_type: Set(line_plan.line_type),
                    speaker_profile_id: Set(line_plan.speaker_profile_id),
                    text: Set(required_text(&line_plan.text, "line.text")?.to_string()),
                    expression: Set(normalize_optional_text(line_plan.expression)),
                    pose: Set(normalize_optional_text(line_plan.pose)),
                    position: Set(normalize_position(line_plan.position)?),
                    presentation_json: Set(line_plan.presentation_json),
                    text_anchor_id: Set(line_plan.text_anchor_id),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(&tx)
                .await
                .map_err(AppError::Database)?;
                if !choices.is_empty() {
                    pending_choices.push((line.id, choices));
                }
            }
        }
        for (scene_id, scene_index, next_scene_index) in pending_scene_next {
            let next_scene_id = scene_id_by_index
                .get(&next_scene_index)
                .copied()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "Galgame scene next_scene_index 不存在: scene_index={scene_index}, next_scene_index={next_scene_index}"
                    ))
                })?;
            if scene_entry_kind_by_index
                .get(&scene_index)
                .map(String::as_str)
                == Some(SCENE_ENTRY_BRANCH)
                && scene_entry_kind_by_index
                    .get(&next_scene_index)
                    .map(String::as_str)
                    != Some(SCENE_ENTRY_MAIN)
            {
                return Err(AppError::InvalidArgument(format!(
                    "Galgame 支线场 next_scene_index 必须指向主线场: scene_index={scene_index}, next_scene_index={next_scene_index}"
                )));
            }
            galgame_scene::Entity::update_many()
                .col_expr(
                    galgame_scene::Column::NextSceneId,
                    Expr::value(next_scene_id),
                )
                .filter(galgame_scene::Column::Id.eq(scene_id))
                .exec(&tx)
                .await
                .map_err(AppError::Database)?;
        }
        for (line_id, choices) in pending_choices {
            for choice in choices {
                let target_scene_id = scene_id_by_index
                    .get(&choice.target_scene_index)
                    .copied()
                    .ok_or_else(|| {
                        AppError::InvalidArgument(format!(
                            "Galgame choice target_scene_index 不存在: {}",
                            choice.target_scene_index
                        ))
                    })?;
                galgame_choice::ActiveModel {
                    line_id: Set(line_id),
                    option_index: Set(choice.option_index),
                    option_text: Set(
                        required_text(&choice.option_text, "choice.option_text")?.to_string()
                    ),
                    target_scene_id: Set(Some(target_scene_id)),
                    created_at: Set(now),
                    ..Default::default()
                }
                .insert(&tx)
                .await
                .map_err(AppError::Database)?;
            }
        }

        tx.commit().await.map_err(AppError::Database)?;
        self.get_script_detail(project_id).await
    }

    pub async fn get_script_detail(
        &self,
        project_id: i64,
    ) -> Result<GalgameScriptDetail, AppError> {
        let context = self.load_galgame_context(project_id).await?;
        let script = galgame_script::Entity::find()
            .filter(galgame_script::Column::ProjectId.eq(context.project.id))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("Galgame 剧本不存在: {project_id}")))?;
        self.script_detail(script).await
    }

    pub async fn update_line(
        &self,
        project_id: i64,
        input: UpdateGalgameLineInput,
    ) -> Result<galgame_line::Model, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(input.line_id, "line_id")?;
        let context = self.load_galgame_context(project_id).await?;
        let line = self
            .load_line_in_project(context.project.id, input.line_id)
            .await?;
        let next_line_type = match input.line_type.as_deref() {
            Some(line_type) => normalize_line_type(line_type)?.to_string(),
            None => line.line_type.clone(),
        };
        let next_speaker_profile_id = match input.speaker_profile_id {
            Some(speaker_profile_id) => {
                normalize_optional_positive_id(speaker_profile_id, "speaker_profile_id")?
            }
            None => line.speaker_profile_id,
        };
        let next_expression = match input.expression.as_ref() {
            Some(expression) => normalize_optional_expression(expression.clone())?,
            None => line.expression.clone(),
        };
        validate_line_semantics(
            &next_line_type,
            next_speaker_profile_id,
            next_expression.as_deref(),
        )?;
        if let Some(speaker_profile_id) = next_speaker_profile_id {
            self.validate_speaker_profile(&context, speaker_profile_id)
                .await?;
        }
        let visual_generation_inputs_changed = input.line_type.is_some()
            || input.speaker_profile_id.is_some()
            || input.text.is_some()
            || input.expression.is_some()
            || input.pose.is_some()
            || input.position.is_some()
            || input.presentation_json.is_some()
            || input.text_anchor_id.is_some();
        let old_line_type = line.line_type.clone();
        let line_scene_id = line.scene_id;
        let now = Utc::now().naive_utc();
        let mut active: galgame_line::ActiveModel = line.into();
        if input.line_type.is_some() {
            active.line_type = Set(next_line_type);
        }
        if input.speaker_profile_id.is_some() {
            active.speaker_profile_id = Set(next_speaker_profile_id);
        }
        if let Some(text) = input.text {
            active.text = Set(required_text(&text, "line.text")?.to_string());
        }
        if let Some(expression) = input.expression {
            active.expression = Set(normalize_optional_expression(expression)?);
        }
        if let Some(pose) = input.pose {
            active.pose = Set(normalize_optional_text(pose));
        }
        if let Some(position) = input.position {
            active.position = Set(normalize_position(position)?);
        }
        if let Some(presentation_json) = input.presentation_json {
            validate_presentation_json(
                &presentation_json,
                &format!("Galgame 行 {}", input.line_id),
            )?;
            active.presentation_json = Set(presentation_json);
        }
        if let Some(text_anchor_id) = input.text_anchor_id {
            if let Some(anchor_id) = text_anchor_id {
                validate_positive_id(anchor_id, "text_anchor_id")?;
                let scene = self
                    .load_scene_in_project(context.project.id, line_scene_id)
                    .await?;
                self.validate_text_anchor(
                    &context,
                    anchor_id,
                    Some(scene.source_snapshot_chapter_id),
                )
                .await?;
            }
            active.text_anchor_id = Set(text_anchor_id);
        }
        active.updated_at = Set(now);
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        if visual_generation_inputs_changed {
            delete_galgame_line_visual_generation_media_graph(
                &tx,
                context.project.id,
                input.line_id,
            )
            .await?;
        }
        let updated = active.update(&tx).await.map_err(AppError::Database)?;
        if old_line_type == LINE_TYPE_CHOICE && updated.line_type != LINE_TYPE_CHOICE {
            galgame_choice::Entity::delete_many()
                .filter(galgame_choice::Column::LineId.eq(updated.id))
                .exec(&tx)
                .await
                .map_err(AppError::Database)?;
        }
        tx.commit().await.map_err(AppError::Database)?;
        Ok(updated)
    }

    pub async fn insert_line(
        &self,
        project_id: i64,
        input: InsertGalgameLineInput,
    ) -> Result<galgame_line::Model, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(input.scene_id, "scene_id")?;
        if let Some(after_line_id) = input.after_line_id {
            validate_positive_id(after_line_id, "after_line_id")?;
        }
        if let Some(anchor_id) = input.text_anchor_id {
            validate_positive_id(anchor_id, "text_anchor_id")?;
        }
        let context = self.load_galgame_context(project_id).await?;
        let scene = self
            .load_scene_in_project(context.project.id, input.scene_id)
            .await?;
        if let Some(anchor_id) = input.text_anchor_id {
            self.validate_text_anchor(&context, anchor_id, Some(scene.source_snapshot_chapter_id))
                .await?;
        }
        let line_type = normalize_line_type(&input.line_type)?;
        let speaker_profile_id =
            normalize_optional_positive_id(input.speaker_profile_id, "speaker_profile_id")?;
        let expression = normalize_optional_expression(input.expression)?;
        validate_line_semantics(line_type, speaker_profile_id, expression.as_deref())?;
        if let Some(speaker_profile_id) = speaker_profile_id {
            self.validate_speaker_profile(&context, speaker_profile_id)
                .await?;
        }
        let position = normalize_position(input.position)?;
        let presentation_json = input.presentation_json.unwrap_or_else(|| json!({}));
        validate_presentation_json(&presentation_json, "Galgame 新增行")?;

        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let insert_index = if let Some(after_line_id) = input.after_line_id {
            let after = galgame_line::Entity::find_by_id(after_line_id)
                .one(&tx)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound(format!("Galgame 行不存在: {after_line_id}")))?;
            if after.scene_id != scene.id {
                return Err(AppError::InvalidArgument(
                    "after_line_id 不属于目标场".to_string(),
                ));
            }
            after.line_index + 1
        } else {
            1
        };
        galgame_line::Entity::update_many()
            .col_expr(
                galgame_line::Column::LineIndex,
                Expr::col(galgame_line::Column::LineIndex).add(1),
            )
            .filter(galgame_line::Column::SceneId.eq(scene.id))
            .filter(galgame_line::Column::LineIndex.gte(insert_index))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?;
        let now = Utc::now().naive_utc();
        let inserted = galgame_line::ActiveModel {
            scene_id: Set(scene.id),
            line_index: Set(insert_index),
            line_type: Set(line_type.to_string()),
            speaker_profile_id: Set(speaker_profile_id),
            text: Set(required_text(&input.text, "line.text")?.to_string()),
            expression: Set(expression),
            pose: Set(normalize_optional_text(input.pose)),
            position: Set(position),
            presentation_json: Set(presentation_json),
            text_anchor_id: Set(input.text_anchor_id),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(AppError::Database)?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(inserted)
    }

    pub async fn update_scene(
        &self,
        project_id: i64,
        input: UpdateGalgameSceneInput,
    ) -> Result<galgame_scene::Model, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(input.scene_id, "scene_id")?;
        let context = self.load_galgame_context(project_id).await?;
        let scene = self
            .load_scene_in_project(context.project.id, input.scene_id)
            .await?;
        let scene_id = scene.id;
        let script_id = scene.script_id;
        let current_entry_kind = scene.entry_kind.clone();
        let background_generation_inputs_changed = input.title.is_some()
            || input.location.is_some()
            || input.time_of_day.is_some()
            || input.background_note.is_some();
        let line_cg_generation_inputs_changed =
            input.title.is_some() || input.location.is_some() || input.time_of_day.is_some();
        let now = Utc::now().naive_utc();
        let mut active: galgame_scene::ActiveModel = scene.into();
        if let Some(title) = input.title {
            active.title = Set(required_text(&title, "scene.title")?.to_string());
        }
        if let Some(location) = input.location {
            active.location = Set(normalize_optional_text(location));
        }
        if let Some(time_of_day) = input.time_of_day {
            active.time_of_day = Set(normalize_optional_text(time_of_day));
        }
        if let Some(background_note) = input.background_note {
            active.background_note = Set(normalize_optional_text(background_note));
        }
        if let Some(next_scene_id) = input.next_scene_id {
            let next_scene_id = if let Some(next_scene_id) = next_scene_id {
                validate_positive_id(next_scene_id, "next_scene_id")?;
                if next_scene_id == scene_id {
                    return Err(AppError::InvalidArgument(
                        "Galgame 场 next_scene_id 不能指向自身".to_string(),
                    ));
                }
                let target_scene = self
                    .load_scene_in_project(context.project.id, next_scene_id)
                    .await?;
                if target_scene.script_id != script_id {
                    return Err(AppError::InvalidArgument(
                        "Galgame 场 next_scene_id 不属于同一剧本".to_string(),
                    ));
                }
                if current_entry_kind == SCENE_ENTRY_BRANCH
                    && target_scene.entry_kind != SCENE_ENTRY_MAIN
                {
                    return Err(AppError::InvalidArgument(
                        "Galgame 支线场 next_scene_id 必须指向主线场".to_string(),
                    ));
                }
                Some(next_scene_id)
            } else {
                None
            };
            active.next_scene_id = Set(next_scene_id);
        }
        active.updated_at = Set(now);
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        if background_generation_inputs_changed || line_cg_generation_inputs_changed {
            delete_galgame_scene_visual_generation_media_graph(
                &tx,
                context.project.id,
                scene_id,
                background_generation_inputs_changed,
                line_cg_generation_inputs_changed,
            )
            .await?;
        }
        let updated = active.update(&tx).await.map_err(AppError::Database)?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(updated)
    }

    pub async fn delete_line(&self, project_id: i64, line_id: i64) -> Result<(), AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(line_id, "line_id")?;
        let context = self.load_galgame_context(project_id).await?;
        let line = self
            .load_line_in_project(context.project.id, line_id)
            .await?;
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        delete_galgame_line_generation_media_graph(&tx, context.project.id, line.id).await?;
        galgame_choice::Entity::delete_many()
            .filter(galgame_choice::Column::LineId.eq(line.id))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?;
        galgame_line::Entity::delete_by_id(line.id)
            .exec(&tx)
            .await
            .map_err(AppError::Database)?;
        compact_line_indexes(&tx, line.scene_id).await?;
        tx.commit().await.map_err(AppError::Database)
    }

    pub async fn reorder_lines(
        &self,
        project_id: i64,
        input: ReorderGalgameLinesInput,
    ) -> Result<Vec<galgame_line::Model>, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(input.scene_id, "scene_id")?;
        for line_id in &input.line_ids {
            validate_positive_id(*line_id, "line_ids")?;
        }
        let context = self.load_galgame_context(project_id).await?;
        self.load_scene_in_project(context.project.id, input.scene_id)
            .await?;
        if input.line_ids.is_empty() {
            return Err(AppError::InvalidArgument("line_ids 不能为空".to_string()));
        }
        let existing = galgame_line::Entity::find()
            .filter(galgame_line::Column::SceneId.eq(input.scene_id))
            .order_by_asc(galgame_line::Column::LineIndex)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if existing.len() != input.line_ids.len() {
            return Err(AppError::InvalidArgument(
                "line_ids 必须包含该场全部行".to_string(),
            ));
        }
        let expected = existing.iter().map(|line| line.id).collect::<Vec<_>>();
        let mut actual = input.line_ids.clone();
        actual.sort_unstable();
        let mut sorted_expected = expected;
        sorted_expected.sort_unstable();
        if actual != sorted_expected {
            return Err(AppError::InvalidArgument(
                "line_ids 与该场行集合不匹配".to_string(),
            ));
        }
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let now = Utc::now().naive_utc();
        for (index, line_id) in input.line_ids.iter().enumerate() {
            galgame_line::Entity::update_many()
                .col_expr(
                    galgame_line::Column::LineIndex,
                    Expr::value(to_i32(index + 1, "line_index")?),
                )
                .col_expr(galgame_line::Column::UpdatedAt, Expr::value(now))
                .filter(galgame_line::Column::Id.eq(*line_id))
                .exec(&tx)
                .await
                .map_err(AppError::Database)?;
        }
        tx.commit().await.map_err(AppError::Database)?;
        galgame_line::Entity::find()
            .filter(galgame_line::Column::SceneId.eq(input.scene_id))
            .order_by_asc(galgame_line::Column::LineIndex)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn update_choices(
        &self,
        project_id: i64,
        input: UpdateGalgameChoicesInput,
    ) -> Result<Vec<galgame_choice::Model>, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(input.line_id, "line_id")?;
        if !(2..=3).contains(&input.choices.len()) {
            return Err(AppError::InvalidArgument(
                "Galgame choice 选项数量必须是 2 到 3 个".to_string(),
            ));
        }
        let context = self.load_galgame_context(project_id).await?;
        let line = self
            .load_line_in_project(context.project.id, input.line_id)
            .await?;
        if line.line_type != LINE_TYPE_CHOICE {
            return Err(AppError::InvalidArgument(format!(
                "Galgame 行 {} 不是 choice 类型，不能编辑选择支",
                line.id
            )));
        }
        let line_scene = self
            .load_scene_in_project(context.project.id, line.scene_id)
            .await?;
        let mut seen_targets = BTreeSet::new();
        let mut normalized = Vec::with_capacity(input.choices.len());
        for (index, choice) in input.choices.into_iter().enumerate() {
            validate_positive_id(choice.target_scene_id, "target_scene_id")?;
            let target_scene = self
                .load_scene_in_project(context.project.id, choice.target_scene_id)
                .await?;
            if target_scene.script_id != line_scene.script_id {
                return Err(AppError::InvalidArgument(format!(
                    "Galgame choice target_scene_id {} 不属于同一剧本",
                    target_scene.id
                )));
            }
            if target_scene.entry_kind == SCENE_ENTRY_BRANCH {
                let return_scene_id = target_scene.next_scene_id.ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "Galgame choice target_scene_id {} 指向未回收敛支线",
                        target_scene.id
                    ))
                })?;
                let return_scene = self
                    .load_scene_in_project(context.project.id, return_scene_id)
                    .await?;
                if return_scene.script_id != line_scene.script_id {
                    return Err(AppError::InvalidArgument(format!(
                        "Galgame choice target_scene_id {} 的回收敛目标不属于同一剧本",
                        target_scene.id
                    )));
                }
                if return_scene.entry_kind != SCENE_ENTRY_MAIN {
                    return Err(AppError::InvalidArgument(format!(
                        "Galgame choice target_scene_id {} 的回收敛目标必须是主线场",
                        target_scene.id
                    )));
                }
            }
            let option_text = required_text(&choice.option_text, "choice.option_text")?.to_string();
            normalized.push((
                to_i32(index + 1, "option_index")?,
                option_text,
                target_scene.id,
            ));
            seen_targets.insert(target_scene.id);
        }
        if seen_targets.is_empty() {
            return Err(AppError::InvalidArgument(
                "Galgame choice 至少需要一个有效目标场".to_string(),
            ));
        }

        let tx = self.db.begin().await.map_err(AppError::Database)?;
        galgame_choice::Entity::delete_many()
            .filter(galgame_choice::Column::LineId.eq(line.id))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?;
        let now = Utc::now().naive_utc();
        for (option_index, option_text, target_scene_id) in normalized {
            galgame_choice::ActiveModel {
                line_id: Set(line.id),
                option_index: Set(option_index),
                option_text: Set(option_text),
                target_scene_id: Set(Some(target_scene_id)),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&tx)
            .await
            .map_err(AppError::Database)?;
        }
        tx.commit().await.map_err(AppError::Database)?;
        galgame_choice::Entity::find()
            .filter(galgame_choice::Column::LineId.eq(line.id))
            .order_by_asc(galgame_choice::Column::OptionIndex)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn plan_assets(&self, project_id: i64) -> Result<GalgameAssetPlan, AppError> {
        validate_positive_id(project_id, "project_id")?;
        let context = self.load_galgame_context(project_id).await?;
        let script = galgame_script::Entity::find()
            .filter(galgame_script::Column::ProjectId.eq(context.project.id))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("Galgame 剧本不存在: {project_id}")))?;
        let scenes = galgame_scene::Entity::find()
            .filter(galgame_scene::Column::ScriptId.eq(script.id))
            .order_by_asc(galgame_scene::Column::SceneIndex)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if scenes.is_empty() {
            return Err(AppError::InvalidState {
                current: "galgame_script_has_no_scenes".to_string(),
                operation: "plan_galgame_assets".to_string(),
            });
        }
        let scene_ids = scenes.iter().map(|scene| scene.id).collect::<Vec<_>>();
        let lines = galgame_line::Entity::find()
            .filter(galgame_line::Column::SceneId.is_in(scene_ids.clone()))
            .order_by_asc(galgame_line::Column::SceneId)
            .order_by_asc(galgame_line::Column::LineIndex)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let snapshot_chapter_ids = scenes
            .iter()
            .map(|scene| scene.source_snapshot_chapter_id)
            .collect::<Vec<_>>();
        let snapshot_chapters = source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::Id.is_in(snapshot_chapter_ids))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .map(|chapter| (chapter.id, chapter))
            .collect::<BTreeMap<_, _>>();
        let mut lines_by_scene = BTreeMap::<i64, Vec<galgame_line::Model>>::new();
        for line in lines {
            lines_by_scene.entry(line.scene_id).or_default().push(line);
        }

        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let mut existing_specs_by_canonical_key =
            load_generation_specs_by_canonical_key(&tx, context.project.id).await?;
        let mut summary = GalgameAssetPlanSummary::default();
        let mut backgrounds = Vec::new();
        let mut portrait_expressions = Vec::new();
        let mut event_cgs = Vec::new();
        let mut portrait_drafts = BTreeMap::<String, GalgamePortraitAssetDraft>::new();
        let mut pending_spec_refs = PendingGalgameSpecRefs::default();

        for scene in scenes {
            let chapter = snapshot_chapters
                .get(&scene.source_snapshot_chapter_id)
                .ok_or_else(|| AppError::InvalidState {
                    current: format!(
                        "source_snapshot_chapter_{}_missing_for_galgame_scene_{}",
                        scene.source_snapshot_chapter_id, scene.id
                    ),
                    operation: "plan_galgame_assets".to_string(),
                })?;
            let scene_lines = lines_by_scene.remove(&scene.id).unwrap_or_default();
            let scene_anchor_ids = scene_lines
                .iter()
                .filter_map(|line| line.text_anchor_id)
                .collect::<BTreeSet<_>>();
            if scene_anchor_ids.is_empty() {
                return Err(AppError::InvalidState {
                    current: format!("galgame_scene_{}_has_no_text_anchor_context", scene.id),
                    operation: "plan_galgame_assets".to_string(),
                });
            }

            let background_params = json!({
                "canonical_key": format!("galgame:{}:background:scene:{}", context.project.id, scene.id),
                "asset_kind": "background",
                "scene": {
                    "id": scene.id,
                    "index": scene.scene_index,
                    "title": scene.title,
                    "location": scene.location,
                    "time_of_day": scene.time_of_day,
                    "background_note": scene.background_note,
                },
                "source": {
                    "source_snapshot_chapter_id": chapter.id,
                    "source_chapter_id": chapter.source_chapter_id,
                    "chapter_number": chapter.chapter_number,
                    "content_hash": chapter.content_hash,
                },
            });
            let background_key = required_json_string(&background_params, "canonical_key")?;
            let background = upsert_galgame_generation_spec(
                &tx,
                &mut existing_specs_by_canonical_key,
                GalgameSpecInput {
                    canonical_key: background_key.clone(),
                    project_id: context.project.id,
                    root_project_id: context.root_project.id,
                    source_snapshot_id: context.source_snapshot.id,
                    source_chapter_id: Some(chapter.source_chapter_id),
                    source_snapshot_chapter_id: Some(chapter.id),
                    target_kind: GALGAME_TARGET_BACKGROUND,
                    target_entity_kind: GALGAME_ENTITY_SCENE,
                    target_entity_id: Some(scene.id),
                    params_json: background_params,
                },
            )
            .await?;
            pending_spec_refs.queue_text_anchors(background.model.id, &scene_anchor_ids)?;
            push_plan_item(
                &mut backgrounds,
                &mut summary,
                background,
                background_key,
                format!("{} 背景", scene.title),
            )?;

            for line in &scene_lines {
                if let Some(profile_id) = line.speaker_profile_id
                    && let Some(variant) = portrait_variant(line)
                {
                    let canonical_key = format!(
                        "galgame:{}:portrait:{}:{}",
                        context.project.id, profile_id, variant.key
                    );
                    let draft = portrait_drafts
                        .entry(canonical_key.clone())
                        .or_insert_with(|| GalgamePortraitAssetDraft {
                            canonical_key,
                            profile_id,
                            variant_key: variant.key,
                            expression: variant.expression,
                            pose: variant.pose,
                            position_samples: BTreeSet::new(),
                            source_line_ids: BTreeSet::new(),
                            text_anchor_ids: BTreeSet::new(),
                        });
                    draft.add_line_sample(line, &scene_anchor_ids);
                }

                let Some(cg_directive) = event_cg_directive(&line.presentation_json, line.id)?
                else {
                    continue;
                };
                let canonical_key = format!("galgame:{}:cg:line:{}", context.project.id, line.id);
                let line_anchor_ids = line.text_anchor_id.into_iter().collect::<BTreeSet<_>>();
                let anchor_ids = if line_anchor_ids.is_empty() {
                    &scene_anchor_ids
                } else {
                    &line_anchor_ids
                };
                let params_json = json!({
                    "canonical_key": canonical_key,
                    "asset_kind": "event_cg",
                    "line": {
                        "id": line.id,
                        "line_index": line.line_index,
                        "line_type": line.line_type,
                        "speaker_profile_id": line.speaker_profile_id,
                        "text": line.text,
                        "expression": line.expression,
                        "pose": line.pose,
                        "position": line.position,
                    },
                    "scene": {
                        "id": scene.id,
                        "title": scene.title,
                        "location": scene.location,
                        "time_of_day": scene.time_of_day,
                    },
                    "cg_note": cg_directive.note,
                    "cg_importance": cg_directive.importance,
                });
                let canonical_key = required_json_string(&params_json, "canonical_key")?;
                let spec = upsert_galgame_generation_spec(
                    &tx,
                    &mut existing_specs_by_canonical_key,
                    GalgameSpecInput {
                        canonical_key: canonical_key.clone(),
                        project_id: context.project.id,
                        root_project_id: context.root_project.id,
                        source_snapshot_id: context.source_snapshot.id,
                        source_chapter_id: Some(chapter.source_chapter_id),
                        source_snapshot_chapter_id: Some(chapter.id),
                        target_kind: GALGAME_TARGET_CG,
                        target_entity_kind: GALGAME_ENTITY_LINE,
                        target_entity_id: Some(line.id),
                        params_json,
                    },
                )
                .await?;
                pending_spec_refs.queue_text_anchors(spec.model.id, anchor_ids)?;
                pending_spec_refs.queue_entity_ref(spec.model.id, line.speaker_profile_id)?;
                push_plan_item(
                    &mut event_cgs,
                    &mut summary,
                    spec,
                    canonical_key,
                    format!("{} #{} CG", scene.title, line.line_index),
                )?;
            }
        }

        for draft in portrait_drafts.into_values() {
            let params_json = draft.params_json();
            let canonical_key = required_json_string(&params_json, "canonical_key")?;
            let spec = upsert_galgame_generation_spec(
                &tx,
                &mut existing_specs_by_canonical_key,
                GalgameSpecInput {
                    canonical_key: canonical_key.clone(),
                    project_id: context.project.id,
                    root_project_id: context.root_project.id,
                    source_snapshot_id: context.source_snapshot.id,
                    source_chapter_id: None,
                    source_snapshot_chapter_id: None,
                    target_kind: GALGAME_TARGET_PORTRAIT_EXPRESSION,
                    target_entity_kind: GALGAME_ENTITY_CHARACTER_PROFILE,
                    target_entity_id: Some(draft.profile_id),
                    params_json,
                },
            )
            .await?;
            pending_spec_refs.queue_text_anchors(spec.model.id, &draft.text_anchor_ids)?;
            pending_spec_refs.queue_entity_ref(spec.model.id, Some(draft.profile_id))?;
            push_plan_item(
                &mut portrait_expressions,
                &mut summary,
                spec,
                canonical_key,
                format!("角色 {} 差分 {}", draft.profile_id, draft.variant_key),
            )?;
        }

        replace_galgame_spec_refs(&tx, pending_spec_refs).await?;
        tx.commit().await.map_err(AppError::Database)?;
        summary.background_specs = backgrounds.len();
        summary.portrait_expression_specs = portrait_expressions.len();
        summary.event_cg_specs = event_cgs.len();
        Ok(GalgameAssetPlan {
            summary,
            backgrounds,
            portrait_expressions,
            event_cgs,
        })
    }

    async fn script_detail(
        &self,
        script: galgame_script::Model,
    ) -> Result<GalgameScriptDetail, AppError> {
        let scenes = galgame_scene::Entity::find()
            .filter(galgame_scene::Column::ScriptId.eq(script.id))
            .order_by_asc(galgame_scene::Column::SceneIndex)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let scene_ids = scenes.iter().map(|scene| scene.id).collect::<Vec<_>>();
        let lines = if scene_ids.is_empty() {
            Vec::new()
        } else {
            galgame_line::Entity::find()
                .filter(galgame_line::Column::SceneId.is_in(scene_ids.clone()))
                .order_by_asc(galgame_line::Column::SceneId)
                .order_by_asc(galgame_line::Column::LineIndex)
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?
        };
        let line_ids = lines.iter().map(|line| line.id).collect::<Vec<_>>();
        let text_anchor_ids = lines
            .iter()
            .filter_map(|line| line.text_anchor_id)
            .collect::<BTreeSet<_>>();
        let choices = if line_ids.is_empty() {
            Vec::new()
        } else {
            galgame_choice::Entity::find()
                .filter(galgame_choice::Column::LineId.is_in(line_ids))
                .order_by_asc(galgame_choice::Column::LineId)
                .order_by_asc(galgame_choice::Column::OptionIndex)
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?
        };
        let mut lines_by_scene = BTreeMap::<i64, Vec<galgame_line::Model>>::new();
        for line in lines {
            lines_by_scene.entry(line.scene_id).or_default().push(line);
        }
        let mut choices_by_scene = BTreeMap::<i64, Vec<galgame_choice::Model>>::new();
        let line_scene = lines_by_scene
            .iter()
            .flat_map(|(scene_id, lines)| lines.iter().map(|line| (line.id, *scene_id)))
            .collect::<BTreeMap<_, _>>();
        for choice in choices {
            if let Some(scene_id) = line_scene.get(&choice.line_id) {
                choices_by_scene.entry(*scene_id).or_default().push(choice);
            }
        }
        let media_bindings =
            galgame_media_bindings(self.db.as_ref(), &scene_ids, &line_scene).await?;
        Ok(GalgameScriptDetail {
            speaker_profiles: self.speaker_profiles(script.root_project_id).await?,
            text_anchors: self
                .text_anchor_summaries(
                    script.root_project_id,
                    script.source_snapshot_id,
                    text_anchor_ids,
                )
                .await?,
            media_bindings,
            script,
            scenes: scenes
                .into_iter()
                .map(|scene| GalgameSceneDetail {
                    lines: lines_by_scene.remove(&scene.id).unwrap_or_default(),
                    choices: choices_by_scene.remove(&scene.id).unwrap_or_default(),
                    scene,
                })
                .collect(),
        })
    }

    async fn load_galgame_context(
        &self,
        project_id: i64,
    ) -> Result<ProjectFamilyContext, AppError> {
        validate_positive_id(project_id, "project_id")?;
        let context = load_derived_project_context(self.db.as_ref(), project_id).await?;
        if context.project.project_kind != PROJECT_KIND_GALGAME {
            return Err(AppError::InvalidArgument(format!(
                "项目不是 Galgame 派生项目: {project_id}"
            )));
        }
        Ok(context)
    }

    async fn load_scene_in_project(
        &self,
        project_id: i64,
        scene_id: i64,
    ) -> Result<galgame_scene::Model, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(scene_id, "scene_id")?;
        let scene = galgame_scene::Entity::find_by_id(scene_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("Galgame 场不存在: {scene_id}")))?;
        let script = galgame_script::Entity::find_by_id(scene.script_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Galgame 剧本不存在: {}", scene.script_id))
            })?;
        if script.project_id != project_id {
            return Err(AppError::InvalidArgument(
                "Galgame 场不属于目标项目".to_string(),
            ));
        }
        Ok(scene)
    }

    async fn load_line_in_project(
        &self,
        project_id: i64,
        line_id: i64,
    ) -> Result<galgame_line::Model, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(line_id, "line_id")?;
        let line = galgame_line::Entity::find_by_id(line_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("Galgame 行不存在: {line_id}")))?;
        self.load_scene_in_project(project_id, line.scene_id)
            .await?;
        Ok(line)
    }

    pub async fn speaker_profiles_for_root_project(
        &self,
        root_project_id: i64,
    ) -> Result<Vec<GalgameSpeakerProfile>, AppError> {
        self.speaker_profiles(root_project_id).await
    }

    async fn speaker_profiles(
        &self,
        root_project_id: i64,
    ) -> Result<Vec<GalgameSpeakerProfile>, AppError> {
        validate_positive_id(root_project_id, "root_project_id")?;
        let profiles = protagonist_profile::Entity::find()
            .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
            .order_by_asc(protagonist_profile::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(profiles
            .into_iter()
            .map(|profile| GalgameSpeakerProfile {
                id: profile.id,
                name: profile.name,
            })
            .collect())
    }

    async fn text_anchor_summaries(
        &self,
        root_project_id: i64,
        source_snapshot_id: i64,
        text_anchor_ids: BTreeSet<i64>,
    ) -> Result<Vec<GalgameTextAnchorSummary>, AppError> {
        validate_positive_id(root_project_id, "root_project_id")?;
        validate_positive_id(source_snapshot_id, "source_snapshot_id")?;
        if text_anchor_ids.is_empty() {
            return Ok(Vec::new());
        }
        let anchors = text_anchor::Entity::find()
            .filter(text_anchor::Column::Id.is_in(text_anchor_ids))
            .filter(text_anchor::Column::ProjectId.eq(root_project_id))
            .filter(text_anchor::Column::SourceSnapshotId.eq(source_snapshot_id))
            .order_by_asc(text_anchor::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(anchors
            .into_iter()
            .map(|anchor| GalgameTextAnchorSummary {
                id: anchor.id,
                source_snapshot_chapter_id: anchor.source_snapshot_chapter_id,
                paragraph_index: anchor.paragraph_index,
                start_offset: anchor.start_offset,
                end_offset: anchor.end_offset,
                quote: anchor.quote,
            })
            .collect())
    }

    async fn load_persist_validation_scope(
        &self,
        plan: &GalgameScriptPersistPlan,
    ) -> Result<GalgamePersistValidationScope, AppError> {
        let mut source_snapshot_chapter_ids = BTreeSet::new();
        let mut speaker_profile_ids = BTreeSet::new();
        let mut text_anchor_ids = BTreeSet::new();

        for scene in &plan.scenes {
            if scene.source_snapshot_chapter_id > 0 {
                source_snapshot_chapter_ids.insert(scene.source_snapshot_chapter_id);
            }
            collect_participant_profile_ids(&scene.participants_json, &mut speaker_profile_ids);
            for line in &scene.lines {
                if let Some(speaker_profile_id) = line.speaker_profile_id
                    && speaker_profile_id > 0
                {
                    speaker_profile_ids.insert(speaker_profile_id);
                }
                if let Some(text_anchor_id) = line.text_anchor_id
                    && text_anchor_id > 0
                {
                    text_anchor_ids.insert(text_anchor_id);
                }
            }
        }

        let source_snapshot_chapters = if source_snapshot_chapter_ids.is_empty() {
            BTreeMap::new()
        } else {
            source_snapshot_chapter::Entity::find()
                .filter(
                    source_snapshot_chapter::Column::Id
                        .is_in(source_snapshot_chapter_ids.iter().copied()),
                )
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?
                .into_iter()
                .map(|chapter| (chapter.id, chapter))
                .collect()
        };
        let speaker_profiles = if speaker_profile_ids.is_empty() {
            BTreeMap::new()
        } else {
            protagonist_profile::Entity::find()
                .filter(protagonist_profile::Column::Id.is_in(speaker_profile_ids.iter().copied()))
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?
                .into_iter()
                .map(|profile| (profile.id, profile))
                .collect()
        };
        let text_anchors = if text_anchor_ids.is_empty() {
            BTreeMap::new()
        } else {
            text_anchor::Entity::find()
                .filter(text_anchor::Column::Id.is_in(text_anchor_ids.iter().copied()))
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?
                .into_iter()
                .map(|anchor| (anchor.id, anchor))
                .collect()
        };

        Ok(GalgamePersistValidationScope {
            source_snapshot_chapters,
            speaker_profiles,
            text_anchors,
        })
    }

    async fn validate_speaker_profile(
        &self,
        context: &ProjectFamilyContext,
        speaker_profile_id: i64,
    ) -> Result<(), AppError> {
        validate_positive_id(speaker_profile_id, "speaker_profile_id")?;
        let profile = protagonist_profile::Entity::find_by_id(speaker_profile_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("角色档案不存在: {speaker_profile_id}")))?;
        if profile.project_id != context.root_project.id {
            return Err(AppError::InvalidArgument(format!(
                "角色档案 {} 不属于当前 Galgame 根项目",
                profile.id
            )));
        }
        Ok(())
    }

    async fn validate_text_anchor(
        &self,
        context: &ProjectFamilyContext,
        text_anchor_id: i64,
        expected_source_snapshot_chapter_id: Option<i64>,
    ) -> Result<(), AppError> {
        validate_positive_id(text_anchor_id, "text_anchor_id")?;
        let anchor = text_anchor::Entity::find_by_id(text_anchor_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("文本锚点不存在: {text_anchor_id}")))?;
        if anchor.project_id != context.root_project.id
            || anchor.source_snapshot_id != Some(context.source_snapshot.id)
            || anchor.source_snapshot_chapter_id.is_none()
        {
            return Err(AppError::InvalidArgument(format!(
                "文本锚点 {} 不属于当前 Galgame 源快照",
                anchor.id
            )));
        }
        if let Some(expected_source_snapshot_chapter_id) = expected_source_snapshot_chapter_id
            && anchor.source_snapshot_chapter_id != Some(expected_source_snapshot_chapter_id)
        {
            return Err(AppError::InvalidArgument(format!(
                "文本锚点 {} 不属于当前 Galgame 场源快照章节 {}",
                anchor.id, expected_source_snapshot_chapter_id
            )));
        }
        Ok(())
    }
}

struct GalgamePersistValidationScope {
    source_snapshot_chapters: BTreeMap<i64, source_snapshot_chapter::Model>,
    speaker_profiles: BTreeMap<i64, protagonist_profile::Model>,
    text_anchors: BTreeMap<i64, text_anchor::Model>,
}

impl GalgamePersistValidationScope {
    fn validate_source_snapshot_chapter(
        &self,
        context: &ProjectFamilyContext,
        source_snapshot_chapter_id: i64,
    ) -> Result<(), AppError> {
        validate_positive_id(source_snapshot_chapter_id, "source_snapshot_chapter_id")?;
        let snapshot_chapter = self
            .source_snapshot_chapters
            .get(&source_snapshot_chapter_id)
            .ok_or_else(|| {
                AppError::NotFound(format!("源快照章节不存在: {source_snapshot_chapter_id}"))
            })?;
        if snapshot_chapter.snapshot_id != context.source_snapshot.id {
            return Err(AppError::InvalidArgument(format!(
                "源快照章节 {} 不属于当前 Galgame 源快照",
                snapshot_chapter.id
            )));
        }
        Ok(())
    }

    fn validate_speaker_profile(
        &self,
        context: &ProjectFamilyContext,
        speaker_profile_id: i64,
    ) -> Result<(), AppError> {
        validate_positive_id(speaker_profile_id, "speaker_profile_id")?;
        let profile = self
            .speaker_profiles
            .get(&speaker_profile_id)
            .ok_or_else(|| AppError::NotFound(format!("角色档案不存在: {speaker_profile_id}")))?;
        if profile.project_id != context.root_project.id {
            return Err(AppError::InvalidArgument(format!(
                "角色档案 {} 不属于当前 Galgame 根项目",
                profile.id
            )));
        }
        Ok(())
    }

    fn validate_scene_participants_json(
        &self,
        context: &ProjectFamilyContext,
        value: &Value,
    ) -> Result<(), AppError> {
        let items = value.as_array().ok_or_else(|| {
            AppError::InvalidArgument("participants_json 必须是 JSON 数组".to_string())
        })?;
        for item in items {
            let object = item.as_object().ok_or_else(|| {
                AppError::InvalidArgument("Galgame 场 participants_json 项必须是对象".to_string())
            })?;
            let Some(profile_id_value) = object.get("character_profile_id") else {
                continue;
            };
            let profile_id = profile_id_value
                .as_i64()
                .filter(|profile_id| *profile_id > 0)
                .ok_or_else(|| {
                    AppError::InvalidArgument(
                        "Galgame 场 participants_json.character_profile_id 必须是正整数"
                            .to_string(),
                    )
                })?;
            self.validate_speaker_profile(context, profile_id)?;
        }
        Ok(())
    }

    fn validate_text_anchor(
        &self,
        context: &ProjectFamilyContext,
        text_anchor_id: i64,
        expected_source_snapshot_chapter_id: Option<i64>,
    ) -> Result<(), AppError> {
        validate_positive_id(text_anchor_id, "text_anchor_id")?;
        let anchor = self
            .text_anchors
            .get(&text_anchor_id)
            .ok_or_else(|| AppError::NotFound(format!("文本锚点不存在: {text_anchor_id}")))?;
        if anchor.project_id != context.root_project.id
            || anchor.source_snapshot_id != Some(context.source_snapshot.id)
            || anchor.source_snapshot_chapter_id.is_none()
        {
            return Err(AppError::InvalidArgument(format!(
                "文本锚点 {} 不属于当前 Galgame 源快照",
                anchor.id
            )));
        }
        if let Some(expected_source_snapshot_chapter_id) = expected_source_snapshot_chapter_id
            && anchor.source_snapshot_chapter_id != Some(expected_source_snapshot_chapter_id)
        {
            return Err(AppError::InvalidArgument(format!(
                "文本锚点 {} 不属于当前 Galgame 场源快照章节 {}",
                anchor.id, expected_source_snapshot_chapter_id
            )));
        }
        Ok(())
    }
}

fn collect_participant_profile_ids(value: &Value, profile_ids: &mut BTreeSet<i64>) {
    let Some(items) = value.as_array() else {
        return;
    };
    for item in items {
        let Some(object) = item.as_object() else {
            continue;
        };
        if let Some(profile_id) = object
            .get("character_profile_id")
            .and_then(Value::as_i64)
            .filter(|profile_id| *profile_id > 0)
        {
            profile_ids.insert(profile_id);
        }
    }
}

async fn delete_script_for_project<C>(db: &C, project_id: i64) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let scripts = galgame_script::Entity::find()
        .filter(galgame_script::Column::ProjectId.eq(project_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let script_ids = scripts.iter().map(|script| script.id).collect::<Vec<_>>();
    if script_ids.is_empty() {
        return Ok(());
    }
    let scene_ids = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .filter(galgame_scene::Column::ScriptId.is_in(script_ids.clone()))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let line_ids = if scene_ids.is_empty() {
        Vec::new()
    } else {
        galgame_line::Entity::find()
            .select_only()
            .column(galgame_line::Column::Id)
            .filter(galgame_line::Column::SceneId.is_in(scene_ids.clone()))
            .into_tuple::<i64>()
            .all(db)
            .await
            .map_err(AppError::Database)?
    };
    if !line_ids.is_empty() {
        galgame_choice::Entity::delete_many()
            .filter(galgame_choice::Column::LineId.is_in(line_ids.clone()))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
        galgame_line::Entity::delete_many()
            .filter(galgame_line::Column::Id.is_in(line_ids))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }
    if !scene_ids.is_empty() {
        galgame_scene::Entity::delete_many()
            .filter(galgame_scene::Column::Id.is_in(scene_ids))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }
    galgame_script::Entity::delete_many()
        .filter(galgame_script::Column::Id.is_in(script_ids))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

async fn delete_galgame_line_generation_media_graph<C>(
    db: &C,
    project_id: i64,
    line_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let mut generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_LINE))
        .filter(generation_spec::Column::TargetEntityId.eq(line_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_ids.extend(
        galgame_portrait_generation_specs_for_source_lines(db, project_id, &[line_id]).await?,
    );
    generation_spec_ids.sort_unstable();
    generation_spec_ids.dedup();
    let prompt_artifact_ids = if generation_spec_ids.is_empty() {
        Vec::new()
    } else {
        prompt_artifact::Entity::find()
            .select_only()
            .column(prompt_artifact::Column::Id)
            .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.clone()))
            .into_tuple::<i64>()
            .all(db)
            .await
            .map_err(AppError::Database)?
    };

    let bound_media_asset_ids = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
        .filter(media_binding::Column::TargetId.eq(line_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    media_binding::Entity::delete_many()
        .filter(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
        .filter(media_binding::Column::TargetId.eq(line_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;

    let provenance_media_asset_ids = if generation_spec_ids.is_empty() {
        Vec::new()
    } else {
        media_provenance::Entity::find()
            .select_only()
            .column(media_provenance::Column::MediaAssetId)
            .filter(media_provenance::Column::GenerationSpecId.is_in(generation_spec_ids.clone()))
            .into_tuple::<i64>()
            .all(db)
            .await
            .map_err(AppError::Database)?
    };
    delete_unbound_project_media_assets(
        db,
        project_id,
        bound_media_asset_ids
            .into_iter()
            .chain(provenance_media_asset_ids)
            .collect(),
    )
    .await?;
    delete_generation_specs_by_ids(db, generation_spec_ids, prompt_artifact_ids).await
}

async fn delete_galgame_line_visual_generation_media_graph<C>(
    db: &C,
    project_id: i64,
    line_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let mut generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_LINE))
        .filter(generation_spec::Column::TargetEntityId.eq(line_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_ids.extend(
        galgame_portrait_generation_specs_for_source_lines(db, project_id, &[line_id]).await?,
    );
    generation_spec_ids.sort_unstable();
    generation_spec_ids.dedup();

    let prompt_artifact_ids =
        prompt_artifact_ids_for_generation_specs(db, &generation_spec_ids).await?;
    let bound_media_asset_ids = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
        .filter(media_binding::Column::TargetId.eq(line_id))
        .filter(media_binding::Column::Role.eq(MEDIA_BINDING_ROLE_CG))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    media_binding::Entity::delete_many()
        .filter(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
        .filter(media_binding::Column::TargetId.eq(line_id))
        .filter(media_binding::Column::Role.eq(MEDIA_BINDING_ROLE_CG))
        .exec(db)
        .await
        .map_err(AppError::Database)?;

    let provenance_media_asset_ids =
        media_asset_ids_for_generation_specs(db, &generation_spec_ids).await?;
    delete_unbound_project_media_assets(
        db,
        project_id,
        bound_media_asset_ids
            .into_iter()
            .chain(provenance_media_asset_ids)
            .collect(),
    )
    .await?;
    delete_generation_specs_by_ids(db, generation_spec_ids, prompt_artifact_ids).await
}

async fn delete_galgame_scene_visual_generation_media_graph<C>(
    db: &C,
    project_id: i64,
    scene_id: i64,
    delete_background: bool,
    delete_line_cgs: bool,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    if !delete_background && !delete_line_cgs {
        return Ok(());
    }
    let line_ids = if delete_line_cgs {
        galgame_line::Entity::find()
            .select_only()
            .column(galgame_line::Column::Id)
            .filter(galgame_line::Column::SceneId.eq(scene_id))
            .into_tuple::<i64>()
            .all(db)
            .await
            .map_err(AppError::Database)?
    } else {
        Vec::new()
    };
    if !delete_background && line_ids.is_empty() {
        return Ok(());
    }

    let mut spec_condition = Condition::any();
    if delete_background {
        spec_condition = spec_condition.add(
            Condition::all()
                .add(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_SCENE))
                .add(generation_spec::Column::TargetEntityId.eq(scene_id)),
        );
    }
    if delete_line_cgs && !line_ids.is_empty() {
        spec_condition = spec_condition.add(
            Condition::all()
                .add(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_LINE))
                .add(generation_spec::Column::TargetEntityId.is_in(line_ids.clone())),
        );
    }
    let generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(spec_condition)
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let prompt_artifact_ids =
        prompt_artifact_ids_for_generation_specs(db, &generation_spec_ids).await?;

    let mut binding_condition = Condition::any();
    if delete_background {
        binding_condition = binding_condition.add(
            Condition::all()
                .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_SCENE))
                .add(media_binding::Column::TargetId.eq(scene_id))
                .add(media_binding::Column::Role.eq(MEDIA_BINDING_ROLE_BACKGROUND)),
        );
    }
    if delete_line_cgs && !line_ids.is_empty() {
        binding_condition = binding_condition.add(
            Condition::all()
                .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
                .add(media_binding::Column::TargetId.is_in(line_ids))
                .add(media_binding::Column::Role.eq(MEDIA_BINDING_ROLE_CG)),
        );
    }
    let bound_media_asset_ids = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(binding_condition.clone())
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    media_binding::Entity::delete_many()
        .filter(binding_condition)
        .exec(db)
        .await
        .map_err(AppError::Database)?;

    let provenance_media_asset_ids =
        media_asset_ids_for_generation_specs(db, &generation_spec_ids).await?;
    delete_unbound_project_media_assets(
        db,
        project_id,
        bound_media_asset_ids
            .into_iter()
            .chain(provenance_media_asset_ids)
            .collect(),
    )
    .await?;
    delete_generation_specs_by_ids(db, generation_spec_ids, prompt_artifact_ids).await
}

async fn delete_galgame_target_generation_media_graph<C>(
    db: &C,
    project_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let scripts = galgame_script::Entity::find()
        .filter(galgame_script::Column::ProjectId.eq(project_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let script_ids = scripts.iter().map(|script| script.id).collect::<Vec<_>>();
    if script_ids.is_empty() {
        return Ok(());
    }
    let scene_ids = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .filter(galgame_scene::Column::ScriptId.is_in(script_ids))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if scene_ids.is_empty() {
        return Ok(());
    }
    let line_ids = galgame_line::Entity::find()
        .select_only()
        .column(galgame_line::Column::Id)
        .filter(galgame_line::Column::SceneId.is_in(scene_ids.clone()))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut spec_condition = Condition::any().add(
        Condition::all()
            .add(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_SCENE))
            .add(generation_spec::Column::TargetEntityId.is_in(scene_ids.clone())),
    );
    if !line_ids.is_empty() {
        spec_condition = spec_condition.add(
            Condition::all()
                .add(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_LINE))
                .add(generation_spec::Column::TargetEntityId.is_in(line_ids.clone())),
        );
    }
    let mut generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(spec_condition)
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if !line_ids.is_empty() {
        generation_spec_ids.extend(
            galgame_portrait_generation_specs_for_source_lines(db, project_id, &line_ids).await?,
        );
        generation_spec_ids.sort_unstable();
        generation_spec_ids.dedup();
    }
    let prompt_artifact_ids = if generation_spec_ids.is_empty() {
        Vec::new()
    } else {
        prompt_artifact::Entity::find()
            .select_only()
            .column(prompt_artifact::Column::Id)
            .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.clone()))
            .into_tuple::<i64>()
            .all(db)
            .await
            .map_err(AppError::Database)?
    };

    let mut binding_condition = Condition::any().add(
        Condition::all()
            .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_SCENE))
            .add(media_binding::Column::TargetId.is_in(scene_ids)),
    );
    if !line_ids.is_empty() {
        binding_condition = binding_condition.add(
            Condition::all()
                .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
                .add(media_binding::Column::TargetId.is_in(line_ids)),
        );
    }
    let bound_media_asset_ids = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(binding_condition.clone())
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    media_binding::Entity::delete_many()
        .filter(binding_condition)
        .exec(db)
        .await
        .map_err(AppError::Database)?;

    let provenance_media_asset_ids = if generation_spec_ids.is_empty() {
        Vec::new()
    } else {
        media_provenance::Entity::find()
            .select_only()
            .column(media_provenance::Column::MediaAssetId)
            .filter(media_provenance::Column::GenerationSpecId.is_in(generation_spec_ids.clone()))
            .into_tuple::<i64>()
            .all(db)
            .await
            .map_err(AppError::Database)?
    };
    let candidate_media_asset_ids = bound_media_asset_ids
        .into_iter()
        .chain(provenance_media_asset_ids)
        .collect::<BTreeSet<_>>();
    delete_unbound_project_media_assets(db, project_id, candidate_media_asset_ids).await?;
    delete_generation_specs_by_ids(db, generation_spec_ids, prompt_artifact_ids).await
}

async fn prompt_artifact_ids_for_generation_specs<C>(
    db: &C,
    generation_spec_ids: &[i64],
) -> Result<Vec<i64>, AppError>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(Vec::new());
    }
    prompt_artifact::Entity::find()
        .select_only()
        .column(prompt_artifact::Column::Id)
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.to_vec()))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)
}

async fn media_asset_ids_for_generation_specs<C>(
    db: &C,
    generation_spec_ids: &[i64],
) -> Result<Vec<i64>, AppError>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(Vec::new());
    }
    media_provenance::Entity::find()
        .select_only()
        .column(media_provenance::Column::MediaAssetId)
        .filter(media_provenance::Column::GenerationSpecId.is_in(generation_spec_ids.to_vec()))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)
}

async fn galgame_portrait_generation_specs_for_source_lines<C>(
    db: &C,
    project_id: i64,
    line_ids: &[i64],
) -> Result<Vec<i64>, AppError>
where
    C: ConnectionTrait,
{
    if line_ids.is_empty() {
        return Ok(Vec::new());
    }
    let line_ids = line_ids.iter().copied().collect::<BTreeSet<_>>();
    let rows = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetKind.eq(GALGAME_TARGET_PORTRAIT_EXPRESSION))
        .filter(generation_spec::Column::TargetEntityKind.eq(GALGAME_ENTITY_CHARACTER_PROFILE))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(rows
        .into_iter()
        .filter(|spec| galgame_portrait_spec_references_lines(&spec.params_json, &line_ids))
        .map(|spec| spec.id)
        .collect())
}

fn galgame_portrait_spec_references_lines(params_json: &Value, line_ids: &BTreeSet<i64>) -> bool {
    params_json
        .get("source_line_ids")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .filter_map(Value::as_i64)
                .any(|line_id| line_ids.contains(&line_id))
        })
}

async fn delete_unbound_project_media_assets<C>(
    db: &C,
    project_id: i64,
    candidate_media_asset_ids: BTreeSet<i64>,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    if candidate_media_asset_ids.is_empty() {
        return Ok(());
    }
    let candidates = candidate_media_asset_ids
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let still_bound = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(media_binding::Column::MediaAssetId.is_in(candidates))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let delete_media_asset_ids = candidate_media_asset_ids
        .difference(&still_bound)
        .copied()
        .collect::<Vec<_>>();
    if delete_media_asset_ids.is_empty() {
        return Ok(());
    }
    let delete_media_asset_ids = media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::Id)
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .filter(media_asset::Column::Id.is_in(delete_media_asset_ids))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if delete_media_asset_ids.is_empty() {
        return Ok(());
    }
    media_provenance::Entity::delete_many()
        .filter(media_provenance::Column::MediaAssetId.is_in(delete_media_asset_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::MediaAssetId.is_in(delete_media_asset_ids.clone()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    media_asset::Entity::delete_many()
        .filter(media_asset::Column::Id.is_in(delete_media_asset_ids))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

async fn delete_generation_specs_by_ids<C>(
    db: &C,
    generation_spec_ids: Vec<i64>,
    prompt_artifact_ids: Vec<i64>,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(());
    }
    let mut provenance_update = media_provenance::Entity::update_many()
        .col_expr(
            media_provenance::Column::GenerationSpecId,
            Expr::value(Option::<i64>::None),
        )
        .filter(media_provenance::Column::GenerationSpecId.is_in(generation_spec_ids.clone()));
    if !prompt_artifact_ids.is_empty() {
        provenance_update = provenance_update.col_expr(
            media_provenance::Column::PromptArtifactId,
            Expr::value(Option::<i64>::None),
        );
    }
    provenance_update
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    if !prompt_artifact_ids.is_empty() {
        media_provenance::Entity::update_many()
            .col_expr(
                media_provenance::Column::PromptArtifactId,
                Expr::value(Option::<i64>::None),
            )
            .filter(media_provenance::Column::PromptArtifactId.is_in(prompt_artifact_ids))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::GenerationSpecId.is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_text_anchor::Entity::delete_many()
        .filter(
            generation_spec_text_anchor::Column::GenerationSpecId
                .is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_entity_ref::Entity::delete_many()
        .filter(
            generation_spec_entity_ref::Column::GenerationSpecId.is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    prompt_artifact::Entity::delete_many()
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec::Entity::delete_many()
        .filter(generation_spec::Column::Id.is_in(generation_spec_ids))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

async fn galgame_media_bindings<C>(
    db: &C,
    scene_ids: &[i64],
    line_scene: &BTreeMap<i64, i64>,
) -> Result<Vec<GalgameMediaBindingSummary>, AppError>
where
    C: ConnectionTrait,
{
    let line_ids = line_scene.keys().copied().collect::<Vec<_>>();
    if scene_ids.is_empty() && line_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut condition = Condition::any();
    if !scene_ids.is_empty() {
        condition = condition.add(
            Condition::all()
                .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_SCENE))
                .add(media_binding::Column::TargetId.is_in(scene_ids.to_vec())),
        );
    }
    if !line_ids.is_empty() {
        condition = condition.add(
            Condition::all()
                .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
                .add(media_binding::Column::TargetId.is_in(line_ids)),
        );
    }
    let bindings = media_binding::Entity::find()
        .filter(condition)
        .order_by_asc(media_binding::Column::TargetKind)
        .order_by_asc(media_binding::Column::TargetId)
        .order_by_asc(media_binding::Column::Role)
        .order_by_asc(media_binding::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if bindings.is_empty() {
        return Ok(Vec::new());
    }
    let asset_ids = bindings
        .iter()
        .map(|binding| binding.media_asset_id)
        .collect::<BTreeSet<_>>();
    let assets = media_asset::Entity::find()
        .filter(media_asset::Column::Id.is_in(asset_ids))
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|asset| (asset.id, asset))
        .collect::<BTreeMap<_, _>>();
    galgame_media_binding_summaries(bindings, &assets)
}

fn galgame_media_binding_summaries(
    bindings: Vec<media_binding::Model>,
    assets: &BTreeMap<i64, media_asset::Model>,
) -> Result<Vec<GalgameMediaBindingSummary>, AppError> {
    let mut summaries = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let asset = assets
            .get(&binding.media_asset_id)
            .cloned()
            .ok_or_else(|| AppError::InvalidState {
                current: format!(
                    "galgame_media_binding_missing_asset: binding_id={}, media_asset_id={}",
                    binding.id, binding.media_asset_id
                ),
                operation: "load_galgame_media_bindings".to_string(),
            })?;
        summaries.push(GalgameMediaBindingSummary { binding, asset });
    }
    Ok(summaries)
}

async fn compact_line_indexes<C>(db: &C, scene_id: i64) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let lines = galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.eq(scene_id))
        .order_by_asc(galgame_line::Column::LineIndex)
        .order_by_asc(galgame_line::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let now = Utc::now().naive_utc();
    for (index, line) in lines.iter().enumerate() {
        let next = to_i32(index + 1, "line_index")?;
        if line.line_index == next {
            continue;
        }
        galgame_line::Entity::update_many()
            .col_expr(galgame_line::Column::LineIndex, Expr::value(next))
            .col_expr(galgame_line::Column::UpdatedAt, Expr::value(now))
            .filter(galgame_line::Column::Id.eq(line.id))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }
    Ok(())
}

pub(crate) fn normalize_line_type(value: &str) -> Result<&str, AppError> {
    let value = value.trim();
    if LINE_TYPES.contains(&value) {
        Ok(value)
    } else {
        Err(AppError::InvalidArgument(format!(
            "Galgame 行类型无效: {value}"
        )))
    }
}

fn validate_scene_entry_kind(value: &str) -> Result<(), AppError> {
    match value.trim() {
        SCENE_ENTRY_MAIN | SCENE_ENTRY_BRANCH => Ok(()),
        other => Err(AppError::InvalidArgument(format!(
            "Galgame 场 entry_kind 无效: {other}"
        ))),
    }
}

pub(crate) fn normalize_position(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = normalize_optional_text(value) else {
        return Ok(None);
    };
    if POSITIONS.contains(&value.as_str()) {
        Ok(Some(value))
    } else {
        Err(AppError::InvalidArgument(format!(
            "Galgame 立绘站位无效: {value}"
        )))
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_optional_expression(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = normalize_optional_text(value) else {
        return Ok(None);
    };
    if is_valid_galgame_expression(&value) {
        Ok(Some(value))
    } else {
        Err(AppError::InvalidArgument(format!(
            "Galgame 表情无效: {value}"
        )))
    }
}

fn normalize_optional_positive_id(
    value: Option<i64>,
    label: &str,
) -> Result<Option<i64>, AppError> {
    match value {
        Some(id) => {
            validate_positive_id(id, label)?;
            Ok(Some(id))
        }
        None => Ok(None),
    }
}

pub(crate) fn validate_line_semantics(
    line_type: &str,
    speaker_profile_id: Option<i64>,
    expression: Option<&str>,
) -> Result<(), AppError> {
    let expression = expression.map(str::trim).filter(|value| !value.is_empty());
    if let Some(expression) = expression
        && !is_valid_galgame_expression(expression)
    {
        return Err(AppError::InvalidArgument(format!(
            "Galgame 表情无效: {expression}"
        )));
    }
    match line_type {
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE if speaker_profile_id.is_none() => Err(
            AppError::InvalidArgument(format!("{line_type} 行必须绑定 speaker_profile_id")),
        ),
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE if expression.is_none() => Err(
            AppError::InvalidArgument(format!("{line_type} 行绑定角色时必须显式提供 expression")),
        ),
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE => Ok(()),
        _ if speaker_profile_id.is_some() => Err(AppError::InvalidArgument(format!(
            "{line_type} 行不能绑定 speaker_profile_id"
        ))),
        _ => Ok(()),
    }
}

fn is_valid_galgame_expression(value: &str) -> bool {
    GALGAME_EXPRESSIONS.contains(&value)
}

fn validate_positive_id(id: i64, label: &str) -> Result<(), AppError> {
    if id > 0 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!("{label} 必须是正整数")))
    }
}

fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label} 不能为空")))
    } else {
        Ok(value)
    }
}

fn ensure_json_object(value: &Value, label: &str) -> Result<(), AppError> {
    if value.is_object() {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{label} 必须是 JSON 对象"
        )))
    }
}

pub(crate) fn validate_presentation_json(value: &Value, line_label: &str) -> Result<(), AppError> {
    event_cg_directive_for_label(value, line_label).map(|_| ())
}

async fn upsert_galgame_generation_spec<C>(
    db: &C,
    existing_specs_by_canonical_key: &mut BTreeMap<String, generation_spec::Model>,
    input: GalgameSpecInput,
) -> Result<GalgameSpecUpsert, AppError>
where
    C: ConnectionTrait,
{
    let source_version_hash = source_hash_for_value(&input.params_json)?;
    let now = Utc::now().naive_utc();
    if let Some(existing) = existing_specs_by_canonical_key
        .get(&input.canonical_key)
        .cloned()
    {
        let content_changed =
            galgame_generation_spec_content_changed(&existing, &input, &source_version_hash);
        if !content_changed {
            return Ok(GalgameSpecUpsert {
                model: existing,
                outcome: GalgameAssetPlanOutcome::Reused,
            });
        }
        let mut active: generation_spec::ActiveModel = existing.into();
        active.root_project_id = Set(input.root_project_id);
        active.source_context_kind = Set(Some(SNAPSHOT_BACKED_SOURCE_CONTEXT.to_string()));
        active.source_snapshot_id = Set(Some(input.source_snapshot_id));
        active.source_chapter_id = Set(input.source_chapter_id);
        active.source_snapshot_chapter_id = Set(input.source_snapshot_chapter_id);
        active.canvas_text_asset_id = Set(None);
        active.target_kind = Set(input.target_kind.to_string());
        active.target_entity_kind = Set(input.target_entity_kind.to_string());
        active.target_entity_id = Set(input.target_entity_id);
        active.status = Set(SPEC_STATUS_DRAFT.to_string());
        active.source_version_hash = Set(source_version_hash);
        active.params_json = Set(input.params_json);
        active.updated_at = Set(now);
        let canonical_key = input.canonical_key.clone();
        let model = active.update(db).await.map_err(AppError::Database)?;
        existing_specs_by_canonical_key.insert(canonical_key, model.clone());
        return Ok(GalgameSpecUpsert {
            model,
            outcome: GalgameAssetPlanOutcome::Updated,
        });
    }

    let canonical_key = input.canonical_key.clone();
    let model = generation_spec::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_context_kind: Set(Some(SNAPSHOT_BACKED_SOURCE_CONTEXT.to_string())),
        source_snapshot_id: Set(Some(input.source_snapshot_id)),
        source_chapter_id: Set(input.source_chapter_id),
        source_snapshot_chapter_id: Set(input.source_snapshot_chapter_id),
        canvas_text_asset_id: Set(None),
        target_kind: Set(input.target_kind.to_string()),
        target_entity_kind: Set(input.target_entity_kind.to_string()),
        target_entity_id: Set(input.target_entity_id),
        status: Set(SPEC_STATUS_DRAFT.to_string()),
        source_version_hash: Set(source_version_hash),
        params_json: Set(input.params_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    existing_specs_by_canonical_key.insert(canonical_key, model.clone());
    Ok(GalgameSpecUpsert {
        model,
        outcome: GalgameAssetPlanOutcome::Created,
    })
}

fn galgame_generation_spec_content_changed(
    existing: &generation_spec::Model,
    input: &GalgameSpecInput,
    source_version_hash: &str,
) -> bool {
    existing.root_project_id != input.root_project_id
        || existing.source_context_kind.as_deref() != Some(SNAPSHOT_BACKED_SOURCE_CONTEXT)
        || existing.source_snapshot_id != Some(input.source_snapshot_id)
        || existing.source_chapter_id != input.source_chapter_id
        || existing.source_snapshot_chapter_id != input.source_snapshot_chapter_id
        || existing.canvas_text_asset_id.is_some()
        || existing.target_kind != input.target_kind
        || existing.target_entity_kind != input.target_entity_kind
        || existing.target_entity_id != input.target_entity_id
        || existing.source_version_hash != source_version_hash
        || existing.params_json != input.params_json
}

async fn load_generation_specs_by_canonical_key<C>(
    db: &C,
    project_id: i64,
) -> Result<BTreeMap<String, generation_spec::Model>, AppError>
where
    C: ConnectionTrait,
{
    let rows = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .order_by_asc(generation_spec::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut specs_by_canonical_key = BTreeMap::new();
    for row in rows {
        let Some(canonical_key) = row.params_json.get("canonical_key").and_then(Value::as_str)
        else {
            continue;
        };
        specs_by_canonical_key
            .entry(canonical_key.to_string())
            .or_insert(row);
    }
    Ok(specs_by_canonical_key)
}

const GALGAME_SPEC_REF_INSERT_CHUNK_SIZE: usize = 1000;

#[derive(Default)]
struct PendingGalgameSpecRefs {
    text_anchor_spec_ids: BTreeSet<i64>,
    text_anchor_refs: Vec<(i64, i64)>,
    entity_ref_spec_ids: BTreeSet<i64>,
    entity_refs: Vec<(i64, i64)>,
}

impl PendingGalgameSpecRefs {
    fn queue_text_anchors(
        &mut self,
        generation_spec_id: i64,
        text_anchor_ids: &BTreeSet<i64>,
    ) -> Result<(), AppError> {
        validate_positive_id(generation_spec_id, "generation_spec_id")?;
        if text_anchor_ids.is_empty() {
            return Err(AppError::InvalidArgument(
                "Galgame 资产生成意图必须至少绑定一个文本锚点".to_string(),
            ));
        }
        self.text_anchor_spec_ids.insert(generation_spec_id);
        self.text_anchor_refs.extend(
            text_anchor_ids
                .iter()
                .map(|text_anchor_id| (generation_spec_id, *text_anchor_id)),
        );
        Ok(())
    }

    fn queue_entity_ref(
        &mut self,
        generation_spec_id: i64,
        speaker_profile_id: Option<i64>,
    ) -> Result<(), AppError> {
        validate_positive_id(generation_spec_id, "generation_spec_id")?;
        self.entity_ref_spec_ids.insert(generation_spec_id);
        if let Some(profile_id) = speaker_profile_id {
            self.entity_refs.push((generation_spec_id, profile_id));
        }
        Ok(())
    }
}

async fn replace_galgame_spec_refs<C>(
    db: &C,
    pending: PendingGalgameSpecRefs,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    if !pending.text_anchor_spec_ids.is_empty() {
        generation_spec_text_anchor::Entity::delete_many()
            .filter(
                generation_spec_text_anchor::Column::GenerationSpecId
                    .is_in(pending.text_anchor_spec_ids.iter().copied()),
            )
            .exec(db)
            .await
            .map_err(AppError::Database)?;
        for chunk in pending
            .text_anchor_refs
            .chunks(GALGAME_SPEC_REF_INSERT_CHUNK_SIZE)
        {
            generation_spec_text_anchor::Entity::insert_many(chunk.iter().map(
                |(generation_spec_id, text_anchor_id)| generation_spec_text_anchor::ActiveModel {
                    generation_spec_id: Set(*generation_spec_id),
                    text_anchor_id: Set(*text_anchor_id),
                    role: Set("context".to_string()),
                },
            ))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
        }
    }

    if !pending.entity_ref_spec_ids.is_empty() {
        generation_spec_entity_ref::Entity::delete_many()
            .filter(
                generation_spec_entity_ref::Column::GenerationSpecId
                    .is_in(pending.entity_ref_spec_ids.iter().copied()),
            )
            .exec(db)
            .await
            .map_err(AppError::Database)?;
        for chunk in pending
            .entity_refs
            .chunks(GALGAME_SPEC_REF_INSERT_CHUNK_SIZE)
        {
            generation_spec_entity_ref::Entity::insert_many(chunk.iter().map(
                |(generation_spec_id, profile_id)| generation_spec_entity_ref::ActiveModel {
                    generation_spec_id: Set(*generation_spec_id),
                    entity_kind: Set("character".to_string()),
                    entity_id: Set(*profile_id),
                    role: Set("subject".to_string()),
                },
            ))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
        }
    }
    Ok(())
}

fn push_plan_item(
    items: &mut Vec<GalgameAssetPlanItem>,
    summary: &mut GalgameAssetPlanSummary,
    spec: GalgameSpecUpsert,
    canonical_key: String,
    label: String,
) -> Result<(), AppError> {
    match spec.outcome {
        GalgameAssetPlanOutcome::Created => summary.created_specs += 1,
        GalgameAssetPlanOutcome::Updated => summary.updated_specs += 1,
        GalgameAssetPlanOutcome::Reused => summary.reused_specs += 1,
    }
    let target_entity_id = spec
        .model
        .target_entity_id
        .ok_or_else(|| AppError::InvalidState {
            current: format!(
                "galgame_generation_spec_{}_missing_target_entity_id",
                spec.model.id
            ),
            operation: "plan_galgame_assets".to_string(),
        })?;
    items.push(GalgameAssetPlanItem {
        canonical_key,
        label,
        target_kind: spec.model.target_kind.clone(),
        target_entity_kind: spec.model.target_entity_kind.clone(),
        target_entity_id,
        generation_spec_id: spec.model.id,
        status: spec.model.status.clone(),
        outcome: spec.outcome,
        params_json: spec.model.params_json,
    });
    Ok(())
}

struct GalgamePortraitAssetDraft {
    canonical_key: String,
    profile_id: i64,
    variant_key: String,
    expression: String,
    pose: Option<String>,
    position_samples: BTreeSet<String>,
    source_line_ids: BTreeSet<i64>,
    text_anchor_ids: BTreeSet<i64>,
}

impl GalgamePortraitAssetDraft {
    fn add_line_sample(&mut self, line: &galgame_line::Model, scene_anchor_ids: &BTreeSet<i64>) {
        self.source_line_ids.insert(line.id);
        if let Some(position) = line
            .position
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            self.position_samples.insert(position.to_string());
        }
        if let Some(text_anchor_id) = line.text_anchor_id {
            self.text_anchor_ids.insert(text_anchor_id);
        } else {
            self.text_anchor_ids
                .extend(scene_anchor_ids.iter().copied());
        }
    }

    fn params_json(&self) -> Value {
        json!({
            "canonical_key": self.canonical_key.clone(),
            "asset_kind": "portrait_expression",
            "character_profile_id": self.profile_id,
            "expression": self.expression.clone(),
            "pose": self.pose.clone(),
            "position_samples": self.position_samples.iter().cloned().collect::<Vec<_>>(),
            "source_line_ids": self.source_line_ids.iter().copied().collect::<Vec<_>>(),
        })
    }
}

struct GalgamePortraitVariant {
    key: String,
    expression: String,
    pose: Option<String>,
}

fn portrait_variant(line: &galgame_line::Model) -> Option<GalgamePortraitVariant> {
    let expression = line
        .expression
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let pose = line
        .pose
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    match (expression, pose) {
        (Some(expression), None) => Some(GalgamePortraitVariant {
            key: format!("expression:{expression}"),
            expression: expression.to_string(),
            pose: None,
        }),
        (Some(expression), Some(pose)) => Some(GalgamePortraitVariant {
            key: format!("expression:{expression}:pose:{pose}"),
            expression: expression.to_string(),
            pose: Some(pose.to_string()),
        }),
        (None, _) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::MEDIA_BINDING_ROLE_BACKGROUND;

    const SPEC_STATUS_COMPLETED_FOR_TEST: &str = "completed";

    #[test]
    fn portrait_variant_requires_expression() {
        let expression_only = test_line(Some("smile"), None);
        let expression_with_pose = test_line(Some("smile"), Some("standing"));
        let pose_only = test_line(None, Some("standing"));

        assert_eq!(
            portrait_variant(&expression_only).map(|variant| variant.key),
            Some("expression:smile".to_string())
        );
        assert_eq!(
            portrait_variant(&expression_with_pose).map(|variant| variant.key),
            Some("expression:smile:pose:standing".to_string())
        );
        assert!(portrait_variant(&pose_only).is_none());
    }

    #[test]
    fn portrait_asset_draft_aggregates_line_samples() {
        let variant = portrait_variant(&test_line(Some("smile"), Some("standing")))
            .expect("portrait variant");
        let mut draft = GalgamePortraitAssetDraft {
            canonical_key: format!("galgame:10:portrait:7:{}", variant.key),
            profile_id: 7,
            variant_key: variant.key,
            expression: variant.expression,
            pose: variant.pose,
            position_samples: BTreeSet::new(),
            source_line_ids: BTreeSet::new(),
            text_anchor_ids: BTreeSet::new(),
        };
        let line_a = test_line_with(10, Some(100), Some("smile"), Some("standing"), Some("left"));
        let line_b = test_line_with(11, None, Some("smile"), Some("standing"), Some("right"));
        let scene_anchor_ids = BTreeSet::from([201, 202]);

        draft.add_line_sample(&line_a, &scene_anchor_ids);
        draft.add_line_sample(&line_b, &scene_anchor_ids);
        let params = draft.params_json();

        assert_eq!(params["source_line_ids"], json!([10, 11]));
        assert_eq!(params["position_samples"], json!(["left", "right"]));
        assert_eq!(draft.text_anchor_ids, BTreeSet::from([100, 201, 202]));
        assert_eq!(params["source_chapter_id"], Value::Null);
    }

    #[test]
    fn line_semantics_require_bound_dialogue_expression() {
        let err =
            validate_line_semantics(LINE_TYPE_DIALOGUE, Some(1), None).expect_err("expression");

        assert!(err.to_string().contains("必须显式提供 expression"));
    }

    #[test]
    fn line_semantics_reject_invalid_expression() {
        let err = validate_line_semantics(LINE_TYPE_DIALOGUE, Some(1), Some("ecstatic"))
            .expect_err("invalid expression");

        assert!(err.to_string().contains("表情无效"));
    }

    #[test]
    fn line_semantics_accept_bound_dialogue_expression() {
        validate_line_semantics(LINE_TYPE_DIALOGUE, Some(1), Some("neutral"))
            .expect("neutral expression");
        validate_line_semantics(LINE_TYPE_MONOLOGUE, Some(1), Some("smile"))
            .expect("smile expression");
    }

    #[test]
    fn generation_spec_content_unchanged_preserves_existing_status() {
        let input = test_spec_input(json!({
            "canonical_key": "galgame:10:background:scene:100",
            "asset_kind": "background",
            "scene": {"id": 100}
        }));
        let source_version_hash = source_hash_for_value(&input.params_json).expect("source hash");
        let mut existing = test_generation_spec(&input, &source_version_hash);
        existing.status = SPEC_STATUS_COMPLETED_FOR_TEST.to_string();

        assert!(!galgame_generation_spec_content_changed(
            &existing,
            &input,
            &source_version_hash
        ));
        assert_eq!(existing.status, SPEC_STATUS_COMPLETED_FOR_TEST);
    }

    #[test]
    fn generation_spec_content_change_resets_existing_spec_to_draft_path() {
        let input = test_spec_input(json!({
            "canonical_key": "galgame:10:background:scene:100",
            "asset_kind": "background",
            "scene": {"id": 100, "background_note": "雨夜月台"}
        }));
        let old_params = json!({
            "canonical_key": "galgame:10:background:scene:100",
            "asset_kind": "background",
            "scene": {"id": 100, "background_note": "晴天月台"}
        });
        let old_hash = source_hash_for_value(&old_params).expect("old source hash");
        let new_hash = source_hash_for_value(&input.params_json).expect("new source hash");
        let mut existing = test_generation_spec(&input, &old_hash);
        existing.params_json = old_params;
        existing.status = SPEC_STATUS_COMPLETED_FOR_TEST.to_string();

        assert!(galgame_generation_spec_content_changed(
            &existing, &input, &new_hash
        ));
    }

    #[test]
    fn generation_spec_content_change_detects_legacy_canvas_context() {
        let input = test_spec_input(json!({
            "canonical_key": "galgame:10:background:scene:100",
            "asset_kind": "background",
            "scene": {"id": 100}
        }));
        let source_version_hash = source_hash_for_value(&input.params_json).expect("source hash");
        let mut existing = test_generation_spec(&input, &source_version_hash);
        existing.canvas_text_asset_id = Some(99);

        assert!(galgame_generation_spec_content_changed(
            &existing,
            &input,
            &source_version_hash
        ));
    }

    #[test]
    fn scene_entry_kind_rejects_unknown_value() {
        validate_scene_entry_kind(SCENE_ENTRY_MAIN).expect("main scene entry");
        validate_scene_entry_kind(SCENE_ENTRY_BRANCH).expect("branch scene entry");

        let err = validate_scene_entry_kind("side_path").expect_err("unknown scene entry kind");

        assert!(err.to_string().contains("entry_kind 无效"));
    }

    #[test]
    fn event_cg_directive_requires_note_when_enabled() {
        let err = event_cg_directive(&json!({"cg": true}), 10)
            .expect_err("enabled event CG must provide a concrete note");

        assert!(err.to_string().contains("必须提供 cg_note"));
    }

    #[test]
    fn event_cg_directive_rejects_invalid_importance() {
        let err = event_cg_directive(
            &json!({
                "cg": true,
                "cg_note": "雨中回头",
                "cg_importance": "major"
            }),
            10,
        )
        .expect_err("invalid importance must fail visibly");

        assert!(err.to_string().contains("cg_importance 无效"));
    }

    #[test]
    fn event_cg_directive_accepts_note_and_importance() {
        let directive = event_cg_directive(
            &json!({
                "cg": true,
                "cg_note": "雨中回头",
                "cg_importance": "critical"
            }),
            10,
        )
        .expect("event CG directive")
        .expect("enabled directive");

        assert_eq!(directive.note, "雨中回头");
        assert_eq!(directive.importance, "critical");
    }

    #[test]
    fn presentation_json_save_contract_rejects_cg_without_note() {
        let err = validate_presentation_json(&json!({"cg": true}), "Galgame 新增行")
            .expect_err("save layer must reject empty event CG");

        assert!(err.to_string().contains("必须提供 cg_note"));
    }

    #[test]
    fn presentation_json_save_contract_rejects_invalid_cg_importance() {
        let err = validate_presentation_json(
            &json!({
                "cg_note": "雨中回头",
                "cg_importance": "major"
            }),
            "Galgame 新增行",
        )
        .expect_err("save layer must reject invalid event CG importance");

        assert!(err.to_string().contains("cg_importance 无效"));
    }

    #[test]
    fn presentation_json_save_contract_accepts_valid_event_cg() {
        validate_presentation_json(
            &json!({
                "cg": true,
                "cg_note": "雨中回头",
                "cg_importance": "critical"
            }),
            "Galgame 新增行",
        )
        .expect("valid event CG presentation json");
    }

    #[test]
    fn media_binding_summaries_reject_missing_asset_instead_of_dropping_binding() {
        let now = Utc::now().naive_utc();
        let bindings = vec![media_binding::Model {
            id: 10,
            media_asset_id: 99,
            target_kind: GALGAME_ENTITY_SCENE.to_string(),
            target_id: 100,
            role: MEDIA_BINDING_ROLE_BACKGROUND.to_string(),
            created_at: now,
        }];

        let err = galgame_media_binding_summaries(bindings, &BTreeMap::new())
            .expect_err("missing media asset must not be silently dropped");

        assert!(matches!(
            err,
            AppError::InvalidState { ref operation, .. }
                if operation == "load_galgame_media_bindings"
        ));
        assert!(err.to_string().contains("binding_id=10"));
        assert!(err.to_string().contains("media_asset_id=99"));
    }

    fn test_line(expression: Option<&str>, pose: Option<&str>) -> galgame_line::Model {
        test_line_with(1, Some(1), expression, pose, Some("center"))
    }

    fn test_line_with(
        id: i64,
        text_anchor_id: Option<i64>,
        expression: Option<&str>,
        pose: Option<&str>,
        position: Option<&str>,
    ) -> galgame_line::Model {
        let now = Utc::now().naive_utc();
        galgame_line::Model {
            id,
            scene_id: 1,
            line_index: 1,
            line_type: LINE_TYPE_DIALOGUE.to_string(),
            speaker_profile_id: Some(1),
            text: "测试台词".to_string(),
            expression: expression.map(str::to_string),
            pose: pose.map(str::to_string),
            position: position.map(str::to_string),
            presentation_json: json!({}),
            text_anchor_id,
            created_at: now,
            updated_at: now,
        }
    }

    fn test_spec_input(params_json: Value) -> GalgameSpecInput {
        GalgameSpecInput {
            canonical_key: "galgame:10:background:scene:100".to_string(),
            project_id: 10,
            root_project_id: 1,
            source_snapshot_id: 20,
            source_chapter_id: Some(2001),
            source_snapshot_chapter_id: Some(1001),
            target_kind: GALGAME_TARGET_BACKGROUND,
            target_entity_kind: GALGAME_ENTITY_SCENE,
            target_entity_id: Some(100),
            params_json,
        }
    }

    fn test_generation_spec(
        input: &GalgameSpecInput,
        source_version_hash: &str,
    ) -> generation_spec::Model {
        let now = Utc::now().naive_utc();
        generation_spec::Model {
            id: 301,
            project_id: input.project_id,
            root_project_id: input.root_project_id,
            source_context_kind: Some(SNAPSHOT_BACKED_SOURCE_CONTEXT.to_string()),
            source_snapshot_id: Some(input.source_snapshot_id),
            source_chapter_id: input.source_chapter_id,
            source_snapshot_chapter_id: input.source_snapshot_chapter_id,
            canvas_text_asset_id: None,
            target_kind: input.target_kind.to_string(),
            target_entity_kind: input.target_entity_kind.to_string(),
            target_entity_id: input.target_entity_id,
            status: SPEC_STATUS_DRAFT.to_string(),
            source_version_hash: source_version_hash.to_string(),
            params_json: input.params_json.clone(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug)]
struct GalgameEventCgDirective {
    note: String,
    importance: String,
}

fn event_cg_directive(
    value: &Value,
    line_id: i64,
) -> Result<Option<GalgameEventCgDirective>, AppError> {
    event_cg_directive_for_label(value, &format!("Galgame 行 {line_id}"))
}

fn event_cg_directive_for_label(
    value: &Value,
    line_label: &str,
) -> Result<Option<GalgameEventCgDirective>, AppError> {
    let Some(object) = value.as_object() else {
        return Err(AppError::InvalidArgument(format!(
            "{line_label} presentation_json 必须是 JSON 对象"
        )));
    };
    let cg_enabled = match object.get("cg") {
        Some(Value::Bool(enabled)) => *enabled,
        Some(Value::Null) | None => false,
        Some(_) => {
            return Err(AppError::InvalidArgument(format!(
                "{line_label} presentation_json.cg 必须是布尔值"
            )));
        }
    };
    let note = event_cg_note_text(object);
    let importance = match object.get("cg_importance") {
        Some(Value::Null) | None => "normal".to_string(),
        Some(Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                "normal".to_string()
            } else if GALGAME_CG_IMPORTANCE.contains(&value) {
                value.to_string()
            } else {
                return Err(AppError::InvalidArgument(format!(
                    "{line_label} presentation_json.cg_importance 无效: {value}"
                )));
            }
        }
        Some(_) => {
            return Err(AppError::InvalidArgument(format!(
                "{line_label} presentation_json.cg_importance 必须是字符串"
            )));
        }
    };
    if !cg_enabled && note.is_none() {
        return Ok(None);
    }
    let note = note.ok_or_else(|| {
        AppError::InvalidArgument(format!("{line_label} 启用事件 CG 时必须提供 cg_note"))
    })?;
    Ok(Some(GalgameEventCgDirective { note, importance }))
}

fn event_cg_note_text(object: &serde_json::Map<String, Value>) -> Option<String> {
    for key in ["cg_note", "cg_prompt", "event_cg_note"] {
        if let Some(note) = object.get(key).and_then(Value::as_str) {
            let note = note.trim();
            if !note.is_empty() {
                return Some(note.to_string());
            }
        }
    }
    None
}

fn source_hash_for_value(value: &Value) -> Result<String, AppError> {
    let serialized = serde_json::to_string(value).map_err(AppError::Serialization)?;
    Ok(source_content_sha256(&serialized))
}

fn required_json_string(value: &Value, field: &str) -> Result<String, AppError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidArgument(format!("params_json.{field} 不能为空")))
}

fn to_i32(value: usize, label: &str) -> Result<i32, AppError> {
    i32::try_from(value)
        .map_err(|_| AppError::InvalidArgument(format!("{label} 超出 i32 可表示范围")))
}
