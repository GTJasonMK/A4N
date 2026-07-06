use a4n_engine::llm::provider::{ChatRequest, UsageInfo};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};

use a4n_core::error::AppError;
use a4n_core::models::{
    chapter::chapter_video_prompt,
    generation::{generation_run, generation_run_target},
};

use crate::asset_lineage_sync::AssetLineageSyncService;
use crate::generation::{
    GenerationRunCheckpointContext, GenerationRunTraceTarget, RecordCompletedGenerationRunInput,
    context_aware_chat_request_trace_json, delete_generation_run_targets_for,
    generation_checkpoint_fingerprint_sha256, record_completed_generation_run_with_checkpoint,
};
use crate::prompt_language::PromptOutputLanguage;

use super::VideoService;
use super::prompt_records::prompt_type_uses_prompt_output_language;

type PromptIdentity = (PromptWorkflowMode, String, i32, Option<String>);
type PromptScope = (PromptWorkflowMode, String, Option<String>);

#[derive(Debug, Clone, Copy, Default)]
struct ChapterPromptSyncPolicy {
    preserve_generated_video_prompts: bool,
    preserve_frame_prompts_with_generated_images: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum PromptWorkflowMode {
    BoundaryFrames,
    FirstFrameChain,
    ChatVideo,
    Storyboard,
}

impl PromptWorkflowMode {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::BoundaryFrames => "boundary_frames",
            Self::FirstFrameChain => "first_frame_chain",
            Self::ChatVideo => "chat_video",
            Self::Storyboard => "storyboard",
        }
    }

    pub(super) fn from_row(row: &chapter_video_prompt::Model) -> Result<Self, AppError> {
        match row.workflow_mode.as_str() {
            "boundary_frames" => Ok(Self::BoundaryFrames),
            "first_frame_chain" => Ok(Self::FirstFrameChain),
            "chat_video" => Ok(Self::ChatVideo),
            "storyboard" => Ok(Self::Storyboard),
            other => Err(AppError::InvalidArgument(format!(
                "章节视频提示词 #{} workflow_mode 无效: {}",
                row.id, other
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct NewChapterVideoPrompt {
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    style: Option<String>,
    prompt_type: String,
    shot_number: i32,
    title: Option<String>,
    scene_summary: Option<String>,
    visual_prompt: String,
    motion_prompt: Option<String>,
    negative_prompt: Option<String>,
    duration_secs: i32,
    aspect_ratio: Option<String>,
    reference_image_ids_json: Option<Value>,
    metadata_json: Option<Value>,
    generation_trace: Option<VideoPromptGenerationTrace>,
}

#[derive(Debug, Clone)]
pub(super) struct VideoPromptGenerationTrace {
    pub(super) prompt_name: String,
    pub(super) request: ChatRequest,
    pub(super) response_text: String,
    pub(super) workflow_run_id: Option<i64>,
    pub(super) usage: Option<UsageInfo>,
}

pub(super) struct NewChapterVideoPromptInput<'a> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: PromptWorkflowMode,
    pub style: Option<&'a str>,
    pub prompt_type: &'a str,
    pub shot_number: i32,
    pub title: Option<&'a str>,
    pub scene_summary: Option<&'a str>,
    pub visual_prompt: &'a str,
    pub motion_prompt: Option<&'a str>,
    pub negative_prompt: Option<&'a str>,
    pub duration_secs: i32,
    pub aspect_ratio: Option<&'a str>,
    pub reference_image_ids_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl NewChapterVideoPrompt {
    pub(super) fn with_generation_trace(mut self, trace: VideoPromptGenerationTrace) -> Self {
        self.generation_trace = Some(trace);
        self
    }

    pub(super) fn with_generation_trace_if(
        mut self,
        trace: Option<VideoPromptGenerationTrace>,
    ) -> Self {
        self.generation_trace = trace;
        self
    }

    pub(super) fn matches_identity(
        &self,
        workflow_mode: PromptWorkflowMode,
        prompt_type: &str,
        shot_number: i32,
    ) -> bool {
        self.workflow_mode == workflow_mode
            && self.prompt_type == prompt_type
            && self.shot_number == shot_number
    }
}

impl VideoService {
    pub(super) async fn sync_chapter_prompts(
        &self,
        chapter_id: i64,
        drafts: Vec<NewChapterVideoPrompt>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.write_chapter_prompts(chapter_id, drafts, ChapterPromptSyncPolicy::default())
            .await
    }

    pub(super) async fn sync_chapter_prompts_preserving_generated_video_prompts(
        &self,
        chapter_id: i64,
        drafts: Vec<NewChapterVideoPrompt>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.write_chapter_prompts(
            chapter_id,
            drafts,
            ChapterPromptSyncPolicy {
                preserve_generated_video_prompts: true,
                ..ChapterPromptSyncPolicy::default()
            },
        )
        .await
    }

    pub(super) async fn sync_chapter_prompts_preserving_generated_frame_prompts(
        &self,
        chapter_id: i64,
        drafts: Vec<NewChapterVideoPrompt>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.write_chapter_prompts(
            chapter_id,
            drafts,
            ChapterPromptSyncPolicy {
                preserve_frame_prompts_with_generated_images: true,
                ..ChapterPromptSyncPolicy::default()
            },
        )
        .await
    }

    async fn write_chapter_prompts(
        &self,
        chapter_id: i64,
        drafts: Vec<NewChapterVideoPrompt>,
        policy: ChapterPromptSyncPolicy,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        let Some(project_id) = drafts.first().map(|draft| draft.project_id) else {
            return Ok(Vec::new());
        };
        self.ensure_chapter_belongs_to_project(project_id, chapter_id)
            .await?;
        validate_prompt_drafts_belong_to_chapter(project_id, chapter_id, &drafts)?;
        for draft in &drafts {
            self.ensure_video_reference_images_available(
                project_id,
                chapter_id,
                draft.reference_image_ids_json.as_ref(),
            )
            .await?;
        }
        let draft_count = drafts.len();
        let prompt_types = drafts
            .iter()
            .map(|draft| draft.prompt_type.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let draft_workflows = drafts
            .iter()
            .map(|draft| draft.workflow_mode)
            .collect::<BTreeSet<_>>();
        let draft_workflow_modes = draft_workflows
            .iter()
            .map(|workflow| workflow.as_str().to_string())
            .collect::<Vec<_>>();
        let draft_identities = drafts
            .iter()
            .map(prompt_identity_for_draft)
            .collect::<BTreeSet<_>>();
        let draft_scopes = drafts
            .iter()
            .map(prompt_scope_for_draft)
            .collect::<BTreeSet<_>>();
        let existing = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
            .filter(chapter_video_prompt::Column::PromptType.is_in(prompt_types))
            .filter(chapter_video_prompt::Column::WorkflowMode.is_in(draft_workflow_modes))
            .order_by_asc(chapter_video_prompt::Column::WorkflowMode)
            .order_by_asc(chapter_video_prompt::Column::PromptType)
            .order_by_asc(chapter_video_prompt::Column::ShotNumber)
            .order_by_asc(chapter_video_prompt::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let mut existing_by_identity =
            HashMap::<PromptIdentity, chapter_video_prompt::Model>::new();
        let mut duplicate_rows = Vec::new();
        for row in existing {
            let workflow = PromptWorkflowMode::from_row(&row)?;
            if !draft_workflows.contains(&workflow) {
                continue;
            }
            let identity = prompt_identity(
                &row.prompt_type,
                row.shot_number,
                workflow,
                row.metadata_json.as_ref(),
            );
            if let Some(current) = existing_by_identity.get(&identity) {
                if should_prefer_existing_prompt(&row, current) {
                    duplicate_rows.push(current.clone());
                    existing_by_identity.insert(identity, row);
                } else {
                    duplicate_rows.push(row);
                }
                continue;
            }
            existing_by_identity.insert(identity, row);
        }
        if !duplicate_rows.is_empty() {
            for row in &duplicate_rows {
                self.ensure_prompt_stage_mutable(row, "删除").await?;
            }
        }
        let preserved_unmatched_prompt_ids = self
            .preserved_unmatched_prompt_ids_by_policy(
                &existing_by_identity,
                &draft_identities,
                &draft_scopes,
                policy,
            )
            .await?;
        let replace_prompt_ids = existing_by_identity
            .iter()
            .filter(|(identity, _)| draft_identities.contains(identity))
            .map(|(_, row)| row.id)
            .collect::<BTreeSet<_>>();
        let delete_prompt_ids = existing_by_identity
            .iter()
            .filter(|(identity, row)| {
                !draft_identities.contains(identity)
                    && !should_keep_unmatched_prompt(row, &draft_scopes)
                    && !preserved_unmatched_prompt_ids.contains(&row.id)
            })
            .map(|(_, row)| row.id)
            .chain(duplicate_rows.iter().map(|row| row.id))
            .collect::<BTreeSet<_>>();
        let ignored_prompt_ids = replace_prompt_ids
            .union(&delete_prompt_ids)
            .copied()
            .collect::<BTreeSet<_>>();

        let mut planned_rows = Vec::with_capacity(drafts.len());
        for draft in drafts {
            let identity = prompt_identity_for_draft(&draft);
            if let Some(existing) = existing_by_identity.remove(&identity) {
                let preserve_existing = self
                    .should_preserve_existing_prompt(&existing, &draft, policy)
                    .await?;
                if !preserve_existing && !prompt_draft_fields_match(&existing, &draft) {
                    self.ensure_prompt_stage_mutable_ignoring(
                        &existing,
                        "重新生成",
                        &ignored_prompt_ids,
                    )
                    .await?;
                }
                planned_rows.push((Some(existing), draft, preserve_existing));
            } else {
                planned_rows.push((None, draft, false));
            }
        }

        let stale_rows = existing_by_identity
            .into_values()
            .filter(|row| {
                !should_keep_unmatched_prompt(row, &draft_scopes)
                    && !preserved_unmatched_prompt_ids.contains(&row.id)
            })
            .collect::<Vec<_>>();
        for row in &stale_rows {
            self.ensure_prompt_stage_mutable(row, "删除").await?;
        }

        let txn = self.db.begin().await.map_err(AppError::Database)?;
        if !duplicate_rows.is_empty() {
            let duplicate_ids = duplicate_rows.iter().map(|row| row.id).collect::<Vec<_>>();
            delete_generation_run_targets_for(&txn, "chapter_video_prompt", &duplicate_ids).await?;
            chapter_video_prompt::Entity::delete_many()
                .filter(chapter_video_prompt::Column::Id.is_in(duplicate_ids))
                .exec(&txn)
                .await
                .map_err(AppError::Database)?;
        }

        let mut rows = Vec::with_capacity(draft_count);
        let mut trace_bindings = Vec::new();
        let now = Utc::now().naive_utc();
        for (existing, draft, preserve_existing) in planned_rows {
            let generation_trace = if preserve_existing {
                None
            } else {
                draft.generation_trace.clone()
            };
            let row = if let Some(existing) = existing {
                if preserve_existing {
                    existing
                } else {
                    update_prompt_from_draft(existing, draft, now, &txn).await?
                }
            } else {
                insert_prompt_from_draft(draft, now, &txn).await?
            };
            if let Some(trace) = generation_trace {
                trace_bindings.push((row.clone(), trace));
            }
            rows.push(row);
        }

        let stale_ids = stale_rows.iter().map(|row| row.id).collect::<Vec<_>>();
        if !stale_ids.is_empty() {
            delete_generation_run_targets_for(&txn, "chapter_video_prompt", &stale_ids).await?;
            chapter_video_prompt::Entity::delete_many()
                .filter(chapter_video_prompt::Column::Id.is_in(stale_ids))
                .exec(&txn)
                .await
                .map_err(AppError::Database)?;
        }

        for (row, trace) in trace_bindings {
            self.record_chapter_video_prompt_generation_run(&txn, &row, trace)
                .await?;
        }
        txn.commit().await.map_err(AppError::Database)?;
        AssetLineageSyncService::new(self.db.clone())
            .sync_project_asset_lineage(project_id)
            .await?;
        Ok(rows)
    }

    async fn preserved_unmatched_prompt_ids_by_policy(
        &self,
        existing_by_identity: &HashMap<PromptIdentity, chapter_video_prompt::Model>,
        draft_identities: &BTreeSet<PromptIdentity>,
        draft_scopes: &BTreeSet<PromptScope>,
        policy: ChapterPromptSyncPolicy,
    ) -> Result<BTreeSet<i64>, AppError> {
        if !policy.preserve_frame_prompts_with_generated_images {
            return Ok(BTreeSet::new());
        }
        let mut ids = BTreeSet::new();
        for (identity, row) in existing_by_identity {
            if draft_identities.contains(identity)
                || should_keep_unmatched_prompt(row, draft_scopes)
            {
                continue;
            }
            if is_frame_prompt_type(&row.prompt_type)
                && self.frame_prompt_has_generated_image(row).await?
            {
                ids.insert(row.id);
            }
        }
        Ok(ids)
    }

    async fn record_chapter_video_prompt_generation_run<C>(
        &self,
        db: &C,
        row: &chapter_video_prompt::Model,
        trace: VideoPromptGenerationTrace,
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        if self
            .chapter_video_prompt_generation_trace_exists(db, row.id, &trace)
            .await?
        {
            return Ok(());
        }
        let request_json = context_aware_chat_request_trace_json(&trace.request)?;
        let messages_json = serde_json::to_value(&trace.request.messages).map_err(|err| {
            AppError::InvalidArgument(format!("视频提示词 LLM 消息序列化失败: {err}"))
        })?;
        let source_refs_json = row
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source"))
            .cloned()
            .map(|source| json!({ "source": source }));
        let media_refs_json = row.reference_image_ids_json.as_ref().map(|references| {
            json!({
                "reference_image_ids": references,
            })
        });
        let call_key = format!(
            "{}:{}:{}:{}",
            trace.prompt_name, row.workflow_mode, row.prompt_type, row.shot_number
        );
        let input_fingerprint_json = json!({
            "prompt_name": trace.prompt_name.clone(),
            "workflow_mode": row.workflow_mode.clone(),
            "prompt_type": row.prompt_type.clone(),
            "shot_number": row.shot_number,
            "chapter_id": row.chapter_id,
            "request": request_json.clone(),
            "source_refs": source_refs_json.clone(),
            "media_refs": media_refs_json.clone(),
        });
        let output_json = json!({
            "chapter_video_prompt_id": row.id,
            "chapter_id": row.chapter_id,
            "workflow_mode": row.workflow_mode.clone(),
            "prompt_type": row.prompt_type.clone(),
            "shot_number": row.shot_number,
            "title": row.title.clone(),
            "scene_summary": row.scene_summary.clone(),
            "visual_prompt": row.visual_prompt.clone(),
            "motion_prompt": row.motion_prompt.clone(),
            "negative_prompt": row.negative_prompt.clone(),
            "duration_secs": row.duration_secs,
            "aspect_ratio": row.aspect_ratio.clone(),
            "reference_image_ids": row.reference_image_ids_json.clone(),
            "metadata": row.metadata_json.clone(),
        });
        let artifact_refs_json = json!({
            "targets": [{
                "target_kind": row.prompt_type.clone(),
                "target_table": "chapter_video_prompt",
                "target_id": row.id,
            }],
        });
        record_completed_generation_run_with_checkpoint(
            db,
            RecordCompletedGenerationRunInput {
                project_id: row.project_id,
                workflow: "video_prompt_generation".to_string(),
                step_key: trace.prompt_name.clone(),
                title: format!("视频提示词生成: {}", row.prompt_type),
                model_kind: "llm".to_string(),
                provider: None,
                model: None,
                prompt_template_names_json: Some(json!([trace.prompt_name])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(messages_json),
                rendered_prompts_json: None,
                input_variables_json: None,
                source_refs_json: source_refs_json.clone(),
                media_refs_json: media_refs_json.clone(),
                request_json: Some(request_json),
                response_summary_json: Some(json!({
                    "chapter_video_prompt_id": row.id,
                    "prompt_type": row.prompt_type,
                    "shot_number": row.shot_number,
                    "output": trace.response_text.clone(),
                    "llm_usage": trace.usage.clone(),
                })),
                targets: vec![GenerationRunTraceTarget {
                    target_kind: row.prompt_type.clone(),
                    target_table: "chapter_video_prompt".to_string(),
                    target_id: row.id,
                }],
            },
            GenerationRunCheckpointContext {
                workflow_run_id: trace.workflow_run_id,
                call_key: Some(call_key),
                input_fingerprint_sha256: Some(generation_checkpoint_fingerprint_sha256(
                    &input_fingerprint_json,
                )?),
                input_fingerprint_json: Some(input_fingerprint_json),
                raw_response_json: Some(json!({
                    "text": trace.response_text,
                    "usage": trace.usage,
                })),
                parsed_output_json: Some(output_json),
                validation_json: None,
                artifact_refs_json: Some(artifact_refs_json),
                attempt: None,
            },
        )
        .await?;
        Ok(())
    }

    async fn chapter_video_prompt_generation_trace_exists<C>(
        &self,
        db: &C,
        prompt_id: i64,
        trace: &VideoPromptGenerationTrace,
    ) -> Result<bool, AppError>
    where
        C: ConnectionTrait,
    {
        let targets = generation_run_target::Entity::find()
            .filter(generation_run_target::Column::TargetTable.eq("chapter_video_prompt"))
            .filter(generation_run_target::Column::TargetId.eq(prompt_id))
            .all(db)
            .await
            .map_err(AppError::Database)?;
        if targets.is_empty() {
            return Ok(false);
        }
        let run_ids = targets
            .iter()
            .map(|target| target.generation_run_id)
            .collect::<BTreeSet<_>>();
        let runs = generation_run::Entity::find()
            .filter(generation_run::Column::Id.is_in(run_ids))
            .filter(generation_run::Column::StepKey.eq(&trace.prompt_name))
            .all(db)
            .await
            .map_err(AppError::Database)?;
        Ok(runs.into_iter().any(|run| {
            run.response_summary_json
                .as_ref()
                .and_then(|summary| summary.get("output"))
                .and_then(Value::as_str)
                == Some(trace.response_text.as_str())
        }))
    }
}

fn validate_prompt_drafts_belong_to_chapter(
    project_id: i64,
    chapter_id: i64,
    drafts: &[NewChapterVideoPrompt],
) -> Result<(), AppError> {
    for draft in drafts {
        validate_prompt_draft_identity(draft)?;
        if draft.chapter_id != chapter_id {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词草稿章节 {} 与同步章节 {} 不一致",
                draft.chapter_id, chapter_id
            )));
        }
        if draft.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词草稿项目 {} 与章节所属项目 {} 不一致",
                draft.project_id, project_id
            )));
        }
    }
    Ok(())
}

fn validate_prompt_draft_identity(draft: &NewChapterVideoPrompt) -> Result<(), AppError> {
    if let Some(metadata_workflow_mode) = draft
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("workflow_mode"))
        .and_then(Value::as_str)
        && metadata_workflow_mode != draft.workflow_mode.as_str()
    {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词草稿 metadata.workflow_mode={} 与显式 workflow_mode={} 冲突",
            metadata_workflow_mode,
            draft.workflow_mode.as_str()
        )));
    }
    let invalid_workflow = |expected: PromptWorkflowMode| {
        AppError::InvalidArgument(format!(
            "视频提示词草稿类型 {} 必须属于 {} workflow，实际为 {}",
            draft.prompt_type,
            expected.as_str(),
            draft.workflow_mode.as_str()
        ))
    };
    match draft.prompt_type.as_str() {
        "frame_shot_analysis" | "frame_segment_plan" | "frame_prompt" => {
            if draft.workflow_mode != PromptWorkflowMode::BoundaryFrames {
                return Err(invalid_workflow(PromptWorkflowMode::BoundaryFrames));
            }
        }
        prompt_type if is_deprecated_upstream_prompt_type(prompt_type) => {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词草稿类型 {} 已废弃；镜头分析和片段规划必须写入共享 frame_shot_analysis/frame_segment_plan",
                draft.prompt_type
            )));
        }
        "first_frame_prompt" => {
            if draft.workflow_mode != PromptWorkflowMode::FirstFrameChain {
                return Err(invalid_workflow(PromptWorkflowMode::FirstFrameChain));
            }
        }
        "chat_video_frame_prompt" => {
            if draft.workflow_mode != PromptWorkflowMode::ChatVideo {
                return Err(invalid_workflow(PromptWorkflowMode::ChatVideo));
            }
        }
        "storyboard_prompt" => {
            if draft.workflow_mode != PromptWorkflowMode::Storyboard {
                return Err(invalid_workflow(PromptWorkflowMode::Storyboard));
            }
        }
        "video_prompt" => {
            let actual = draft
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("source_prompt_type"))
                .and_then(Value::as_str);
            if actual != Some("frame_segment_plan") {
                return Err(AppError::InvalidArgument(format!(
                    "视频提示词草稿 workflow {} 的 video_prompt 必须声明 source_prompt_type={}，实际为 {}",
                    draft.workflow_mode.as_str(),
                    "frame_segment_plan",
                    actual.unwrap_or("<missing>")
                )));
            }
        }
        other => {
            return Err(AppError::InvalidArgument(format!(
                "未知视频提示词草稿类型：{}",
                other
            )));
        }
    }
    Ok(())
}

pub(super) fn new_prompt_draft(input: NewChapterVideoPromptInput<'_>) -> NewChapterVideoPrompt {
    let NewChapterVideoPromptInput {
        project_id,
        chapter_id,
        workflow_mode,
        style,
        prompt_type,
        shot_number,
        title,
        scene_summary,
        visual_prompt,
        motion_prompt,
        negative_prompt,
        duration_secs,
        aspect_ratio,
        reference_image_ids_json,
        metadata_json,
    } = input;
    NewChapterVideoPrompt {
        project_id,
        chapter_id,
        workflow_mode,
        style: style.map(ToString::to_string),
        prompt_type: prompt_type.to_string(),
        shot_number,
        title: title.map(ToString::to_string),
        scene_summary: scene_summary.map(ToString::to_string),
        visual_prompt: visual_prompt.to_string(),
        motion_prompt: motion_prompt.map(ToString::to_string),
        negative_prompt: negative_prompt.map(ToString::to_string),
        duration_secs,
        aspect_ratio: aspect_ratio.map(ToString::to_string),
        reference_image_ids_json,
        metadata_json,
        generation_trace: None,
    }
}

fn prompt_identity(
    prompt_type: &str,
    shot_number: i32,
    workflow: PromptWorkflowMode,
    metadata: Option<&Value>,
) -> PromptIdentity {
    (
        workflow,
        prompt_type.to_string(),
        shot_number,
        prompt_output_language_identity(prompt_type, metadata),
    )
}

fn prompt_identity_for_draft(draft: &NewChapterVideoPrompt) -> PromptIdentity {
    prompt_identity(
        &draft.prompt_type,
        draft.shot_number,
        draft.workflow_mode,
        draft.metadata_json.as_ref(),
    )
}

fn prompt_scope(
    prompt_type: &str,
    workflow: PromptWorkflowMode,
    metadata: Option<&Value>,
) -> PromptScope {
    (
        workflow,
        prompt_type.to_string(),
        prompt_output_language_identity(prompt_type, metadata),
    )
}

fn prompt_scope_for_draft(draft: &NewChapterVideoPrompt) -> PromptScope {
    prompt_scope(
        &draft.prompt_type,
        draft.workflow_mode,
        draft.metadata_json.as_ref(),
    )
}

fn prompt_scope_for_row(row: &chapter_video_prompt::Model) -> Result<PromptScope, AppError> {
    Ok(prompt_scope(
        &row.prompt_type,
        PromptWorkflowMode::from_row(row)?,
        row.metadata_json.as_ref(),
    ))
}

fn prompt_output_language_identity(prompt_type: &str, metadata: Option<&Value>) -> Option<String> {
    prompt_type_uses_prompt_output_language(prompt_type).then(|| {
        PromptOutputLanguage::from_metadata(metadata)
            .as_str()
            .to_string()
    })
}

fn draft_workflow_mode(draft: &NewChapterVideoPrompt) -> &'static str {
    draft.workflow_mode.as_str()
}

fn generated_prompt_status(prompt_type: &str) -> String {
    match prompt_type {
        "frame_shot_analysis" | "frame_segment_plan" => "completed".to_string(),
        _ => "pending".to_string(),
    }
}

fn is_deprecated_upstream_prompt_type(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "first_frame_shot_analysis"
            | "first_frame_segment_plan"
            | "chat_video_shot_analysis"
            | "chat_video_segment_plan"
    )
}

fn should_prefer_existing_prompt(
    candidate: &chapter_video_prompt::Model,
    current: &chapter_video_prompt::Model,
) -> bool {
    let candidate_linked = candidate.generated_video_id.is_some();
    let current_linked = current.generated_video_id.is_some();
    if candidate_linked != current_linked {
        return candidate_linked;
    }
    candidate.updated_at > current.updated_at || candidate.id < current.id
}

fn should_keep_unmatched_prompt(
    row: &chapter_video_prompt::Model,
    draft_scopes: &BTreeSet<PromptScope>,
) -> bool {
    if prompt_type_uses_prompt_output_language(&row.prompt_type)
        && prompt_scope_for_row(row).is_ok_and(|scope| !draft_scopes.contains(&scope))
    {
        return true;
    }
    row.prompt_type == "video_prompt"
        && (row.generated_video_id.is_some()
            || matches!(row.status.as_str(), "running" | "completed"))
}

fn is_frame_prompt_type(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt" | "storyboard_prompt"
    )
}

fn prompt_is_locked(row: &chapter_video_prompt::Model) -> bool {
    row.prompt_type == "video_prompt" && row.status == "running"
}

fn should_preserve_generated_video_prompt(
    existing: &chapter_video_prompt::Model,
    draft: &NewChapterVideoPrompt,
    policy: ChapterPromptSyncPolicy,
) -> bool {
    policy.preserve_generated_video_prompts
        && existing.prompt_type == "video_prompt"
        && draft.prompt_type == "video_prompt"
        && existing.generated_video_id.is_some()
        && !prompt_draft_fields_match(existing, draft)
}

impl VideoService {
    async fn should_preserve_existing_prompt(
        &self,
        existing: &chapter_video_prompt::Model,
        draft: &NewChapterVideoPrompt,
        policy: ChapterPromptSyncPolicy,
    ) -> Result<bool, AppError> {
        if should_preserve_generated_video_prompt(existing, draft, policy) {
            return Ok(true);
        }
        if policy.preserve_frame_prompts_with_generated_images
            && existing.prompt_type == draft.prompt_type
            && is_frame_prompt_type(&existing.prompt_type)
            && !prompt_draft_fields_match(existing, draft)
            && self.frame_prompt_has_generated_image(existing).await?
        {
            return Ok(true);
        }
        Ok(false)
    }
}

fn prompt_generation_fields_changed(
    existing: &chapter_video_prompt::Model,
    draft: &NewChapterVideoPrompt,
) -> bool {
    existing.visual_prompt != draft.visual_prompt
        || existing.motion_prompt != draft.motion_prompt
        || existing.negative_prompt != draft.negative_prompt
        || existing.duration_secs != draft.duration_secs
        || existing.aspect_ratio != draft.aspect_ratio
        || existing.reference_image_ids_json != draft.reference_image_ids_json
        || !metadata_generation_context_matches(
            existing.metadata_json.as_ref(),
            draft.metadata_json.as_ref(),
        )
}

fn metadata_generation_context_matches(existing: Option<&Value>, draft: Option<&Value>) -> bool {
    existing.and_then(|metadata| metadata.get("source"))
        == draft.and_then(|metadata| metadata.get("source"))
        && existing
            .and_then(|metadata| metadata.get("contract_version"))
            .and_then(Value::as_str)
            == draft
                .and_then(|metadata| metadata.get("contract_version"))
                .and_then(Value::as_str)
}

fn prompt_draft_fields_match(
    existing: &chapter_video_prompt::Model,
    draft: &NewChapterVideoPrompt,
) -> bool {
    existing.project_id == draft.project_id
        && existing.chapter_id == draft.chapter_id
        && existing.style == draft.style
        && existing.prompt_type == draft.prompt_type
        && existing.workflow_mode == draft_workflow_mode(draft)
        && existing.shot_number == draft.shot_number
        && existing.title == draft.title
        && existing.scene_summary == draft.scene_summary
        && existing.visual_prompt == draft.visual_prompt
        && existing.motion_prompt == draft.motion_prompt
        && existing.negative_prompt == draft.negative_prompt
        && existing.duration_secs == draft.duration_secs
        && existing.aspect_ratio == draft.aspect_ratio
        && existing.reference_image_ids_json == draft.reference_image_ids_json
        && existing.metadata_json == draft.metadata_json
}

async fn insert_prompt_from_draft<C>(
    draft: NewChapterVideoPrompt,
    now: chrono::NaiveDateTime,
    db: &C,
) -> Result<chapter_video_prompt::Model, AppError>
where
    C: ConnectionTrait,
{
    let status = generated_prompt_status(&draft.prompt_type);
    let workflow_mode = draft_workflow_mode(&draft).to_string();
    let insert_result = chapter_video_prompt::Entity::insert(chapter_video_prompt::ActiveModel {
        project_id: Set(draft.project_id),
        chapter_id: Set(draft.chapter_id),
        style: Set(draft.style),
        prompt_type: Set(draft.prompt_type),
        workflow_mode: Set(workflow_mode),
        shot_number: Set(draft.shot_number),
        title: Set(draft.title),
        scene_summary: Set(draft.scene_summary),
        visual_prompt: Set(draft.visual_prompt),
        motion_prompt: Set(draft.motion_prompt),
        negative_prompt: Set(draft.negative_prompt),
        duration_secs: Set(draft.duration_secs),
        aspect_ratio: Set(draft.aspect_ratio),
        reference_image_ids_json: Set(draft.reference_image_ids_json),
        status: Set(status),
        generated_video_id: Set(None),
        metadata_json: Set(draft.metadata_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(db)
    .await
    .map_err(AppError::Database)?;

    chapter_video_prompt::Entity::find_by_id(insert_result.last_insert_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "插入章节视频提示词后无法回读记录：id={}",
                insert_result.last_insert_id
            ))
        })
}

async fn update_prompt_from_draft<C>(
    existing: chapter_video_prompt::Model,
    draft: NewChapterVideoPrompt,
    now: chrono::NaiveDateTime,
    db: &C,
) -> Result<chapter_video_prompt::Model, AppError>
where
    C: ConnectionTrait,
{
    if prompt_is_locked(&existing) {
        return Ok(existing);
    }
    let is_video_prompt = draft.prompt_type == "video_prompt";
    if prompt_draft_fields_match(&existing, &draft) {
        return Ok(existing);
    }
    let generation_fields_changed = prompt_generation_fields_changed(&existing, &draft);
    if is_video_prompt && existing.generated_video_id.is_some() && generation_fields_changed {
        return Err(AppError::InvalidArgument(
            "已存在由该视频提示词生成的视频，不能重新生成视频提示词；请先删除对应视频".to_string(),
        ));
    }
    let workflow_mode = draft_workflow_mode(&draft).to_string();
    let mut active: chapter_video_prompt::ActiveModel = existing.into();
    active.project_id = Set(draft.project_id);
    active.chapter_id = Set(draft.chapter_id);
    active.style = Set(draft.style);
    active.prompt_type = Set(draft.prompt_type.clone());
    active.workflow_mode = Set(workflow_mode);
    active.shot_number = Set(draft.shot_number);
    active.title = Set(draft.title);
    active.scene_summary = Set(draft.scene_summary);
    active.visual_prompt = Set(draft.visual_prompt);
    active.motion_prompt = Set(draft.motion_prompt);
    active.negative_prompt = Set(draft.negative_prompt);
    active.duration_secs = Set(draft.duration_secs);
    active.aspect_ratio = Set(draft.aspect_ratio);
    active.reference_image_ids_json = Set(draft.reference_image_ids_json);
    active.metadata_json = Set(draft.metadata_json);
    if is_video_prompt && generation_fields_changed {
        active.generated_video_id = Set(None);
        active.status = Set("pending".to_string());
    } else if !is_video_prompt {
        active.status = Set(generated_prompt_status(&draft.prompt_type));
    }
    active.updated_at = Set(now);
    active.update(db).await.map_err(AppError::Database)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn draft(prompt_type: &str, workflow_mode: PromptWorkflowMode) -> NewChapterVideoPrompt {
        new_prompt_draft(NewChapterVideoPromptInput {
            project_id: 1,
            chapter_id: 2,
            workflow_mode,
            style: Some("测试风格"),
            prompt_type,
            shot_number: 0,
            title: Some("测试提示词"),
            scene_summary: None,
            visual_prompt: "{}",
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9"),
            reference_image_ids_json: Some(json!([])),
            metadata_json: Some(json!({
                "contract_version": "test",
                "source": {"content_sha256": "same-source"}
            })),
        })
    }

    #[test]
    fn deprecated_upstream_prompt_types_are_rejected_for_new_drafts() {
        for (prompt_type, workflow_mode) in [
            (
                "first_frame_shot_analysis",
                PromptWorkflowMode::FirstFrameChain,
            ),
            (
                "first_frame_segment_plan",
                PromptWorkflowMode::FirstFrameChain,
            ),
            ("chat_video_shot_analysis", PromptWorkflowMode::ChatVideo),
            ("chat_video_segment_plan", PromptWorkflowMode::ChatVideo),
        ] {
            let err = validate_prompt_draft_identity(&draft(prompt_type, workflow_mode))
                .expect_err("deprecated upstream prompt type must be rejected");

            assert!(
                matches!(&err, AppError::InvalidArgument(message) if message.contains("已废弃")),
                "unexpected error for {prompt_type}: {err}"
            );
        }
    }

    #[test]
    fn only_shared_upstream_prompts_are_inserted_as_completed() {
        assert_eq!(generated_prompt_status("frame_shot_analysis"), "completed");
        assert_eq!(generated_prompt_status("frame_segment_plan"), "completed");
        assert_eq!(
            generated_prompt_status("first_frame_shot_analysis"),
            "pending"
        );
        assert_eq!(
            generated_prompt_status("first_frame_segment_plan"),
            "pending"
        );
        assert_eq!(
            generated_prompt_status("chat_video_shot_analysis"),
            "pending"
        );
        assert_eq!(
            generated_prompt_status("chat_video_segment_plan"),
            "pending"
        );
    }

    #[test]
    fn video_prompt_drafts_must_point_to_shared_segment_plan() {
        let valid = new_prompt_draft(NewChapterVideoPromptInput {
            project_id: 1,
            chapter_id: 2,
            workflow_mode: PromptWorkflowMode::FirstFrameChain,
            style: Some("测试风格"),
            prompt_type: "video_prompt",
            shot_number: 1,
            title: Some("视频提示词"),
            scene_summary: None,
            visual_prompt: "visual",
            motion_prompt: Some("motion"),
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9"),
            reference_image_ids_json: Some(json!([])),
            metadata_json: Some(json!({
                "source_prompt_type": "frame_segment_plan",
                "source": {"content_sha256": "same-source"}
            })),
        });
        assert!(validate_prompt_draft_identity(&valid).is_ok());

        let legacy = new_prompt_draft(NewChapterVideoPromptInput {
            project_id: 1,
            chapter_id: 2,
            workflow_mode: PromptWorkflowMode::FirstFrameChain,
            style: Some("测试风格"),
            prompt_type: "video_prompt",
            shot_number: 1,
            title: Some("视频提示词"),
            scene_summary: None,
            visual_prompt: "visual",
            motion_prompt: Some("motion"),
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9"),
            reference_image_ids_json: Some(json!([])),
            metadata_json: Some(json!({
                "source_prompt_type": "first_frame_segment_plan",
                "source": {"content_sha256": "same-source"}
            })),
        });
        let err = validate_prompt_draft_identity(&legacy)
            .expect_err("video_prompt must use shared segment source");
        assert!(
            matches!(&err, AppError::InvalidArgument(message) if message.contains("source_prompt_type=frame_segment_plan")),
            "unexpected error: {err}"
        );
    }
}
