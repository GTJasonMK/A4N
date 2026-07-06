use chrono::Utc;
use sea_orm::*;
use serde_json::{Value, json};

use crate::prompt_language::{PromptOutputLanguage, metadata_matches_prompt_output_language};
use std::collections::{BTreeMap, BTreeSet};

use a4n_core::error::AppError;
use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter_video_prompt;

use super::prompt_context::validate_prompt_duration;
use super::prompt_store::PromptWorkflowMode;
use super::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, CHAT_VIDEO_CONTRACT_VERSION,
    FIRST_FRAME_CHAIN_CONTRACT_VERSION, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
    SHOT_ANALYSIS_CONTRACT_VERSION, STORYBOARD_PROMPT_CONTRACT_VERSION,
    STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION, VideoService,
};
use crate::generation::{
    GenerationRunTraceTarget, RecordCompletedGenerationRunWithCheckpointInput,
    append_generation_artifact_target, delete_generation_run_targets_for,
    record_completed_generation_run_with_checkpoint,
};
use crate::source::source_content_sha256;

#[derive(Debug, Clone, Default)]
pub struct VideoPromptDownstreamDependencies {
    pub prompt_ids: Vec<i64>,
    pub image_ids: Vec<i64>,
    pub video_ids: Vec<i64>,
}

impl VideoPromptDownstreamDependencies {
    pub fn is_empty(&self) -> bool {
        self.prompt_ids.is_empty() && self.image_ids.is_empty() && self.video_ids.is_empty()
    }
}

pub struct VideoPromptUpdateInput<'a> {
    pub visual_prompt: Option<&'a str>,
    pub motion_prompt: Option<Option<&'a str>>,
    pub negative_prompt: Option<Option<&'a str>>,
    pub duration_secs: Option<i32>,
    pub reference_image_ids_json: Option<Option<Value>>,
}

#[derive(Clone, Copy)]
pub(in crate::video) struct PromptWorkflowRowsContext<'a> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: &'a str,
    pub prompt_type: &'a str,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
    pub generation_asset_fingerprint: Option<&'a Value>,
    pub source_prompt_type: Option<&'a str>,
}

pub(in crate::video) struct ReusablePromptOutputQuery<'a> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub prompt_type: &'a str,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
    pub prompt_output_language: PromptOutputLanguage,
}

struct PromptWorkflowRowsQuery<'a> {
    context: PromptWorkflowRowsContext<'a>,
    prompt_output_language: Option<PromptOutputLanguage>,
    exclude_running: bool,
}

#[derive(Clone, Copy)]
struct PromptGenerationContext<'a> {
    prompt_type: &'a str,
    source_metadata: &'a Value,
    style: &'a str,
    aspect_ratio: &'a str,
    reference_ids: &'a Value,
    generation_asset_fingerprint: Option<&'a Value>,
    source_prompt_type: Option<&'a str>,
    prompt_output_language: PromptOutputLanguage,
}

#[derive(Clone, Copy)]
struct PromptGenerationContextForWorkflow<'a> {
    expected_workflow_mode: &'a str,
    prompt_type: &'a str,
    source_metadata: &'a Value,
    style: &'a str,
    aspect_ratio: &'a str,
    reference_ids: &'a Value,
    generation_asset_fingerprint: Option<&'a Value>,
    source_prompt_type: Option<&'a str>,
    prompt_output_language: Option<PromptOutputLanguage>,
}

fn chapter_video_prompt_parsed_output(
    parsed_output: Option<Value>,
    row: &chapter_video_prompt::Model,
) -> Result<Option<Value>, AppError> {
    let prompt = json!({
        "chapter_video_prompt_id": row.id,
        "project_id": row.project_id,
        "chapter_id": row.chapter_id,
        "workflow_mode": &row.workflow_mode,
        "prompt_type": &row.prompt_type,
        "shot_number": row.shot_number,
        "title": &row.title,
        "scene_summary": &row.scene_summary,
        "visual_prompt": &row.visual_prompt,
        "motion_prompt": &row.motion_prompt,
        "negative_prompt": &row.negative_prompt,
        "duration_secs": row.duration_secs,
        "aspect_ratio": &row.aspect_ratio,
        "reference_image_ids": &row.reference_image_ids_json,
        "metadata": &row.metadata_json,
    });
    match parsed_output {
        Some(Value::Object(mut map)) => {
            map.insert("chapter_video_prompt".to_string(), prompt);
            Ok(Some(Value::Object(map)))
        }
        Some(_) => Err(AppError::InvalidArgument(
            "视频提示词生成运行记录 parsed_output_json 必须是对象".to_string(),
        )),
        None => Ok(Some(prompt)),
    }
}

impl VideoService {
    pub async fn list_prompts_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.ensure_chapter_belongs_to_project(project_id, chapter_id)
            .await?;
        chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
            .order_by_asc(chapter_video_prompt::Column::WorkflowMode)
            .order_by_asc(chapter_video_prompt::Column::ShotNumber)
            .order_by_asc(chapter_video_prompt::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn delete_prompt(&self, prompt_id: i64) -> Result<(), AppError> {
        self.delete_prompt_inner(prompt_id, false).await
    }

    pub async fn delete_prompt_allowing_downstream(&self, prompt_id: i64) -> Result<(), AppError> {
        self.delete_prompt_inner(prompt_id, true).await
    }

    async fn delete_prompt_inner(
        &self,
        prompt_id: i64,
        allow_downstream: bool,
    ) -> Result<(), AppError> {
        let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频提示词不存在".into()))?;
        if prompt.status == "running" {
            return Err(AppError::InvalidState {
                current: prompt.status,
                operation: "删除视频提示词".to_string(),
            });
        }
        if !allow_downstream {
            self.ensure_prompt_stage_mutable(&prompt, "删除").await?;
        }
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        delete_generation_run_targets_for(&txn, "chapter_video_prompt", &[prompt_id]).await?;
        chapter_video_prompt::Entity::delete_by_id(prompt_id)
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn delete_prompts(&self, prompt_ids: &[i64]) -> Result<u64, AppError> {
        self.delete_prompts_inner(prompt_ids, false).await
    }

    pub async fn delete_prompts_with_downstream_policy(
        &self,
        prompt_ids: &[i64],
        allow_downstream: bool,
    ) -> Result<u64, AppError> {
        self.delete_prompts_inner(prompt_ids, allow_downstream)
            .await
    }

    async fn delete_prompts_inner(
        &self,
        prompt_ids: &[i64],
        allow_downstream: bool,
    ) -> Result<u64, AppError> {
        let prompt_ids = prompt_ids.iter().copied().collect::<BTreeSet<_>>();
        if prompt_ids.is_empty() {
            return Ok(0);
        }
        let rows = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.iter().copied()))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if rows.len() != prompt_ids.len() {
            return Err(AppError::NotFound(
                "部分视频提示词不存在，不能批量删除".into(),
            ));
        }
        for row in &rows {
            if row.status == "running" {
                return Err(AppError::InvalidState {
                    current: row.status.clone(),
                    operation: "删除视频提示词".to_string(),
                });
            }
            if !allow_downstream {
                self.ensure_prompt_stage_mutable_ignoring(row, "删除", &prompt_ids)
                    .await?;
            }
        }
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let prompt_ids_vec = prompt_ids.iter().copied().collect::<Vec<_>>();
        delete_generation_run_targets_for(&txn, "chapter_video_prompt", &prompt_ids_vec).await?;
        let result = chapter_video_prompt::Entity::delete_many()
            .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.iter().copied()))
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(result.rows_affected)
    }

    pub async fn update_prompt(
        &self,
        prompt_id: i64,
        visual_prompt: Option<&str>,
        motion_prompt: Option<Option<&str>>,
        negative_prompt: Option<Option<&str>>,
        duration_secs: Option<i32>,
        reference_image_ids_json: Option<Option<Value>>,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        self.update_prompt_inner(
            prompt_id,
            VideoPromptUpdateInput {
                visual_prompt,
                motion_prompt,
                negative_prompt,
                duration_secs,
                reference_image_ids_json,
            },
            false,
        )
        .await
    }

    pub async fn update_prompt_with_downstream_policy(
        &self,
        prompt_id: i64,
        input: VideoPromptUpdateInput<'_>,
        allow_downstream: bool,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        self.update_prompt_inner(prompt_id, input, allow_downstream)
            .await
    }

    async fn update_prompt_inner(
        &self,
        prompt_id: i64,
        input: VideoPromptUpdateInput<'_>,
        allow_downstream: bool,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let row = self
            .update_prompt_inner_in(&txn, prompt_id, input, allow_downstream)
            .await?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(row)
    }

    pub async fn update_prompt_with_generation_run_checkpoint(
        &self,
        prompt_id: i64,
        input: VideoPromptUpdateInput<'_>,
        metadata_json: Option<Value>,
        allow_downstream: bool,
        mut generation_run: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        if !generation_run.run.targets.is_empty() {
            return Err(AppError::InvalidArgument(
                "视频提示词生成运行记录目标由视频服务绑定，调用方不能预填 targets".to_string(),
            ));
        }
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let mut row = self
            .update_prompt_inner_in(&txn, prompt_id, input, allow_downstream)
            .await?;
        if let Some(metadata_json) = metadata_json {
            let metadata_json = sync_prompt_metadata_after_update(
                &row.prompt_type,
                Some(metadata_json),
                &row.visual_prompt,
                row.negative_prompt.as_deref(),
                row.duration_secs,
                row.reference_image_ids_json.as_ref(),
            )?
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "视频提示词 metadata 同步返回空值，prompt_type={}",
                    row.prompt_type
                ))
            })?;
            let mut active: chapter_video_prompt::ActiveModel = row.clone().into();
            active.metadata_json = Set(Some(metadata_json));
            active.updated_at = Set(Utc::now().naive_utc());
            row = active.update(&txn).await.map_err(AppError::Database)?;
        }
        if generation_run.run.project_id != row.project_id {
            return Err(AppError::InvalidArgument(
                "视频提示词生成运行记录 project_id 与提示词资产不一致".to_string(),
            ));
        }
        let target = GenerationRunTraceTarget {
            target_kind: "chapter_video_prompt".to_string(),
            target_table: "chapter_video_prompt".to_string(),
            target_id: row.id,
        };
        generation_run.run.targets = vec![target.clone()];
        generation_run.checkpoint.parsed_output_json =
            chapter_video_prompt_parsed_output(generation_run.checkpoint.parsed_output_json, &row)?;
        generation_run.checkpoint.artifact_refs_json = Some(append_generation_artifact_target(
            generation_run.checkpoint.artifact_refs_json,
            target,
        )?);
        record_completed_generation_run_with_checkpoint(
            &txn,
            generation_run.run,
            generation_run.checkpoint,
        )
        .await?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(row)
    }

    async fn update_prompt_inner_in<C>(
        &self,
        db: &C,
        prompt_id: i64,
        input: VideoPromptUpdateInput<'_>,
        allow_downstream: bool,
    ) -> Result<chapter_video_prompt::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let VideoPromptUpdateInput {
            visual_prompt,
            motion_prompt,
            negative_prompt,
            duration_secs,
            reference_image_ids_json,
        } = input;
        let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频提示词不存在".into()))?;

        if prompt.status == "running" {
            return Err(AppError::InvalidState {
                current: prompt.status,
                operation: "编辑视频提示词".to_string(),
            });
        }
        self.ensure_chapter_belongs_to_project(prompt.project_id, prompt.chapter_id)
            .await?;
        if !allow_downstream {
            self.ensure_prompt_stage_mutable(&prompt, "编辑").await?;
        }
        if let Some(value) = reference_image_ids_json.as_ref() {
            self.ensure_video_reference_images_available(
                prompt.project_id,
                prompt.chapter_id,
                value.as_ref(),
            )
            .await?;
        }
        let visual_prompt = visual_prompt
            .map(|value| required_prompt_text(value, "视频提示词 visual_prompt"))
            .transpose()?;
        let motion_prompt = motion_prompt.map(normalize_optional_prompt_text);
        let negative_prompt = negative_prompt.map(normalize_optional_prompt_text);

        let was_video_prompt = prompt.prompt_type == "video_prompt";
        let content_changed = visual_prompt
            .as_ref()
            .is_some_and(|v| prompt.visual_prompt.as_str() != v.as_str())
            || motion_prompt
                .as_ref()
                .is_some_and(|v| prompt.motion_prompt.as_deref() != v.as_deref())
            || negative_prompt
                .as_ref()
                .is_some_and(|v| prompt.negative_prompt.as_deref() != v.as_deref())
            || duration_secs.is_some_and(|v| prompt.duration_secs != v)
            || reference_image_ids_json
                .as_ref()
                .is_some_and(|v| prompt.reference_image_ids_json.as_ref() != v.as_ref());
        let final_visual_prompt = visual_prompt
            .as_deref()
            .unwrap_or(prompt.visual_prompt.as_str())
            .to_string();
        let final_negative_prompt = negative_prompt
            .as_ref()
            .map(|value| value.as_deref())
            .unwrap_or(prompt.negative_prompt.as_deref())
            .map(ToString::to_string);
        let final_duration_secs = duration_secs.unwrap_or(prompt.duration_secs);
        let final_reference_image_ids_json = reference_image_ids_json
            .clone()
            .unwrap_or_else(|| prompt.reference_image_ids_json.clone());
        let metadata_json = sync_prompt_metadata_after_update(
            &prompt.prompt_type,
            prompt.metadata_json.clone(),
            &final_visual_prompt,
            final_negative_prompt.as_deref(),
            final_duration_secs,
            final_reference_image_ids_json.as_ref(),
        )?;
        let mut active: chapter_video_prompt::ActiveModel = prompt.into();
        if let Some(v) = visual_prompt {
            active.visual_prompt = Set(v);
        }
        if let Some(v) = motion_prompt {
            active.motion_prompt = Set(v);
        }
        if let Some(v) = negative_prompt {
            active.negative_prompt = Set(v);
        }
        if let Some(v) = duration_secs {
            validate_prompt_duration(v)?;
            active.duration_secs = Set(v);
        }
        if let Some(v) = reference_image_ids_json {
            active.reference_image_ids_json = Set(v);
        }
        if let Some(metadata_json) = metadata_json {
            active.metadata_json = Set(Some(metadata_json));
        }
        if was_video_prompt && content_changed && !allow_downstream {
            active.generated_video_id = Set(None);
            active.status = Set("pending".to_string());
        }
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await.map_err(AppError::Database)
    }

    pub async fn prompt_downstream_dependencies(
        &self,
        prompt_id: i64,
    ) -> Result<VideoPromptDownstreamDependencies, AppError> {
        let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频提示词不存在".into()))?;
        self.prompt_downstream_dependencies_for_prompt(&prompt)
            .await
    }

    pub async fn ensure_frame_prompt_image_generation_allowed(
        &self,
        prompt_id: i64,
        generation_source: &str,
    ) -> Result<(), AppError> {
        let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频帧提示词不存在".into()))?;
        ensure_frame_prompt_generation_source_matches(&prompt, generation_source)?;
        ensure_frame_prompt_workflow_matches_type(&prompt)?;
        if prompt.status == "running" {
            return Err(AppError::InvalidState {
                current: prompt.status.clone(),
                operation: format!("生成{}", prompt_stage_label(&prompt)),
            });
        }
        self.ensure_chapter_belongs_to_project(prompt.project_id, prompt.chapter_id)
            .await?;
        self.ensure_frame_prompt_has_no_generated_image(&prompt, "成图")
            .await
    }

    pub async fn effective_frame_prompt_image_reference_ids(
        &self,
        prompt_id: i64,
        generation_source: &str,
        requested_reference_image_ids: &[i64],
    ) -> Result<Vec<i64>, AppError> {
        let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频帧提示词不存在".into()))?;
        let expected_source =
            ensure_frame_prompt_generation_source_matches(&prompt, generation_source)?;
        ensure_frame_prompt_workflow_matches_type(&prompt)?;
        if expected_source != "video_boundary_frame_prompt" || prompt.shot_number <= 1 {
            return Ok(requested_reference_image_ids.to_vec());
        }

        let previous_image = self
            .previous_boundary_frame_image_for_prompt(&prompt)
            .await?
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "边界帧提示词 #{} 是第 {} 张边界帧，生成前必须先生成第 {} 张边界帧图片，上一帧图片会作为参考图 1 输入",
                    prompt.id,
                    prompt.shot_number,
                    prompt.shot_number - 1
                ))
            })?;
        effective_boundary_frame_reference_image_ids(
            requested_reference_image_ids,
            previous_image.id,
        )
    }

    pub(super) async fn reusable_prompt_output(
        &self,
        query: ReusablePromptOutputQuery<'_>,
    ) -> Result<Option<String>, AppError> {
        let ReusablePromptOutputQuery {
            project_id,
            chapter_id,
            prompt_type,
            source_metadata,
            style,
            duration_secs,
            aspect_ratio,
            reference_ids,
            prompt_output_language,
        } = query;
        let Some(workflow_mode) = expected_prompt_workflow_mode(prompt_type) else {
            return Ok(None);
        };
        let rows = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
            .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type))
            .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode))
            .filter(chapter_video_prompt::Column::Status.eq("completed"))
            .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
            .order_by_desc(chapter_video_prompt::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(rows
            .into_iter()
            .find(|row| {
                row.duration_secs == duration_secs
                    && prompt_matches_generation_context(
                        row,
                        PromptGenerationContext {
                            prompt_type,
                            source_metadata,
                            style,
                            aspect_ratio,
                            reference_ids,
                            generation_asset_fingerprint: None,
                            source_prompt_type: None,
                            prompt_output_language,
                        },
                    )
            })
            .map(|row| row.visual_prompt))
    }

    pub(super) async fn reusable_shared_shot_analysis_row(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        prompt_type: &str,
        source_metadata: &Value,
    ) -> Result<Option<chapter_video_prompt::Model>, AppError> {
        let rows = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
            .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type))
            .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode))
            .filter(chapter_video_prompt::Column::Status.eq("completed"))
            .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
            .order_by_desc(chapter_video_prompt::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(rows.into_iter().find(|row| {
            prompt_matches_shared_shot_analysis_context(
                row,
                workflow_mode,
                prompt_type,
                source_metadata,
            )
        }))
    }

    pub(super) async fn reusable_shared_shot_analysis_output(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        prompt_type: &str,
        source_metadata: &Value,
    ) -> Result<Option<String>, AppError> {
        Ok(self
            .reusable_shared_shot_analysis_row(
                project_id,
                chapter_id,
                workflow_mode,
                prompt_type,
                source_metadata,
            )
            .await?
            .map(|row| row.visual_prompt))
    }

    pub(super) async fn reusable_prompt_rows(
        &self,
        context: PromptWorkflowRowsContext<'_>,
        prompt_output_language: PromptOutputLanguage,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        let rows = self
            .prompt_rows_matching_workflow(PromptWorkflowRowsQuery {
                context,
                prompt_output_language: Some(prompt_output_language),
                exclude_running: true,
            })
            .await?;

        let mut by_shot = BTreeMap::<i32, chapter_video_prompt::Model>::new();
        for row in rows {
            by_shot.entry(row.shot_number).or_insert(row);
        }
        Ok(by_shot.into_values().collect())
    }

    pub(super) async fn matching_prompt_rows_for_workflow(
        &self,
        context: PromptWorkflowRowsContext<'_>,
        prompt_output_language: PromptOutputLanguage,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.prompt_rows_matching_workflow(PromptWorkflowRowsQuery {
            context,
            prompt_output_language: Some(prompt_output_language),
            exclude_running: false,
        })
        .await
    }

    pub(super) async fn matching_prompt_rows_for_workflow_any_language(
        &self,
        context: PromptWorkflowRowsContext<'_>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.prompt_rows_matching_workflow(PromptWorkflowRowsQuery {
            context,
            prompt_output_language: None,
            exclude_running: false,
        })
        .await
    }

    async fn prompt_rows_matching_workflow(
        &self,
        query: PromptWorkflowRowsQuery<'_>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        let PromptWorkflowRowsQuery {
            context,
            prompt_output_language,
            exclude_running,
        } = query;
        let mut query = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(context.project_id))
            .filter(chapter_video_prompt::Column::ChapterId.eq(context.chapter_id))
            .filter(chapter_video_prompt::Column::PromptType.eq(context.prompt_type))
            .filter(chapter_video_prompt::Column::WorkflowMode.eq(context.workflow_mode));
        if exclude_running {
            query = query.filter(chapter_video_prompt::Column::Status.ne("running"));
        }
        let rows = query
            .order_by_asc(chapter_video_prompt::Column::ShotNumber)
            .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
            .order_by_desc(chapter_video_prompt::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        Ok(rows
            .into_iter()
            .filter(|row| {
                row.duration_secs == context.duration_secs
                    && prompt_matches_generation_context_for_workflow(
                        row,
                        PromptGenerationContextForWorkflow {
                            expected_workflow_mode: context.workflow_mode,
                            prompt_type: context.prompt_type,
                            source_metadata: context.source_metadata,
                            style: context.style,
                            aspect_ratio: context.aspect_ratio,
                            reference_ids: context.reference_ids,
                            generation_asset_fingerprint: context.generation_asset_fingerprint,
                            source_prompt_type: context.source_prompt_type,
                            prompt_output_language,
                        },
                    )
            })
            .collect())
    }

    pub(super) async fn ensure_prompt_stage_mutable(
        &self,
        prompt: &chapter_video_prompt::Model,
        operation: &str,
    ) -> Result<(), AppError> {
        self.ensure_prompt_stage_mutable_ignoring(prompt, operation, &BTreeSet::new())
            .await
    }

    pub(super) async fn ensure_prompt_stage_mutable_ignoring(
        &self,
        prompt: &chapter_video_prompt::Model,
        operation: &str,
        ignored_prompt_ids: &BTreeSet<i64>,
    ) -> Result<(), AppError> {
        let Some(stage) = prompt_stage(prompt) else {
            return Ok(());
        };
        if stage == PromptStage::Video && prompt.generated_video_id.is_some() {
            return Err(AppError::InvalidArgument(format!(
                "已存在由该视频提示词生成的视频，不能{operation}视频提示词；请先删除对应视频"
            )));
        }
        if stage == PromptStage::FramePrompt {
            self.ensure_frame_prompt_has_no_generated_image(prompt, operation)
                .await?;
        }

        let rows = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(prompt.project_id))
            .filter(chapter_video_prompt::Column::ChapterId.eq(prompt.chapter_id))
            .filter(chapter_video_prompt::Column::Id.ne(prompt.id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if let Some(later) = rows
            .iter()
            .filter(|row| !ignored_prompt_ids.contains(&row.id))
            .filter(|row| prompt_same_generation_chain(prompt, row))
            .filter_map(|row| {
                prompt_stage(row)
                    .filter(|later_stage| *later_stage > stage)
                    .map(|later_stage| (later_stage, row))
            })
            .min_by_key(|(later_stage, _)| *later_stage)
            .map(|(_, row)| row)
        {
            let current_label = prompt_stage_label(prompt);
            let later_label = prompt_stage_label(later);
            return Err(AppError::InvalidArgument(format!(
                "已存在后续{later_label}，不能{operation}{current_label}；请先删除后续{later_label}"
            )));
        }
        Ok(())
    }

    async fn ensure_frame_prompt_has_no_generated_image(
        &self,
        prompt: &chapter_video_prompt::Model,
        operation: &str,
    ) -> Result<(), AppError> {
        if let Some(image) = self.generated_frame_image_for_prompt(prompt).await? {
            let current_label = prompt_stage_label(prompt);
            let image_label = match prompt.prompt_type.as_str() {
                "first_frame_prompt" => "首帧图片",
                "chat_video_frame_prompt" => "Chat 首尾帧图片",
                "storyboard_prompt" => "分镜板图片",
                _ => "边界帧图片",
            };
            return Err(AppError::InvalidArgument(format!(
                "已存在由该{current_label}生成的{image_label} #{}，不能{operation}{current_label}；请先删除对应图片",
                image.id
            )));
        }
        Ok(())
    }

    async fn generated_frame_image_for_prompt(
        &self,
        prompt: &chapter_video_prompt::Model,
    ) -> Result<Option<generated_image::Model>, AppError> {
        Ok(self
            .generated_frame_images_for_prompt(prompt)
            .await?
            .into_iter()
            .next())
    }

    pub(in crate::video) async fn frame_prompt_has_generated_image(
        &self,
        prompt: &chapter_video_prompt::Model,
    ) -> Result<bool, AppError> {
        Ok(self
            .generated_frame_image_for_prompt(prompt)
            .await?
            .is_some())
    }

    async fn generated_frame_images_for_prompt(
        &self,
        prompt: &chapter_video_prompt::Model,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        let Some(source) = frame_image_generation_source_for_prompt(prompt) else {
            return Ok(Vec::new());
        };
        let images = generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(prompt.project_id))
            .filter(generated_image::Column::ChapterId.eq(prompt.chapter_id))
            .order_by_desc(generated_image::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(images
            .into_iter()
            .filter(|image| {
                let metadata = image.metadata_json.as_ref();
                image_generation_prompt_id(metadata) == Some(prompt.id)
                    && image_generation_source(metadata) == Some(source)
            })
            .collect())
    }

    async fn previous_boundary_frame_image_for_prompt(
        &self,
        prompt: &chapter_video_prompt::Model,
    ) -> Result<Option<generated_image::Model>, AppError> {
        let Some(previous_frame_number) = prompt.shot_number.checked_sub(1) else {
            return Ok(None);
        };
        let previous_prompts = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(prompt.project_id))
            .filter(chapter_video_prompt::Column::ChapterId.eq(prompt.chapter_id))
            .filter(chapter_video_prompt::Column::WorkflowMode.eq(&prompt.workflow_mode))
            .filter(chapter_video_prompt::Column::PromptType.eq(&prompt.prompt_type))
            .filter(chapter_video_prompt::Column::ShotNumber.eq(previous_frame_number))
            .filter(chapter_video_prompt::Column::Status.ne("running"))
            .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
            .order_by_desc(chapter_video_prompt::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let Some(previous_prompt) = previous_prompts
            .into_iter()
            .find(|row| prompt_same_frame_reference_chain(prompt, row))
        else {
            return Ok(None);
        };
        let previous_source_hash = prompt_source_hash(&previous_prompt);
        let allowed_project_ids = self.video_asset_project_ids(prompt.project_id).await?;
        let images = generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.is_in(allowed_project_ids))
            .filter(generated_image::Column::ChapterId.eq(prompt.chapter_id))
            .order_by_desc(generated_image::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        for image in images {
            let metadata = image.metadata_json.as_ref();
            match image_generation_source(metadata) {
                Some("video_boundary_frame_prompt")
                    if image_generation_prompt_id(metadata) == Some(previous_prompt.id) =>
                {
                    return Ok(Some(image));
                }
                Some("video_tail_frame_capture")
                    if image_generation_shot_number(metadata)
                        .and_then(|shot_number| shot_number.checked_add(1))
                        == Some(previous_frame_number)
                        && previous_source_hash.is_some()
                        && image_generation_source_hash(metadata) == previous_source_hash =>
                {
                    return Ok(Some(image));
                }
                _ => {}
            }
        }
        Ok(None)
    }

    async fn prompt_downstream_dependencies_for_prompt(
        &self,
        prompt: &chapter_video_prompt::Model,
    ) -> Result<VideoPromptDownstreamDependencies, AppError> {
        let Some(stage) = prompt_stage(prompt) else {
            return Ok(VideoPromptDownstreamDependencies::default());
        };

        let mut prompt_ids = BTreeSet::<i64>::new();
        let mut image_ids = BTreeSet::<i64>::new();
        let mut video_ids = BTreeSet::<i64>::new();
        collect_prompt_asset_dependencies(self, prompt, &mut image_ids, &mut video_ids).await?;

        let rows = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(prompt.project_id))
            .filter(chapter_video_prompt::Column::ChapterId.eq(prompt.chapter_id))
            .filter(chapter_video_prompt::Column::Id.ne(prompt.id))
            .order_by_asc(chapter_video_prompt::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        for row in rows {
            let Some(later_stage) = prompt_stage(&row) else {
                continue;
            };
            if later_stage <= stage || !prompt_same_generation_chain(prompt, &row) {
                continue;
            }
            prompt_ids.insert(row.id);
            collect_prompt_asset_dependencies(self, &row, &mut image_ids, &mut video_ids).await?;
        }

        Ok(VideoPromptDownstreamDependencies {
            prompt_ids: prompt_ids.into_iter().collect(),
            image_ids: image_ids.into_iter().collect(),
            video_ids: video_ids.into_iter().collect(),
        })
    }
}

async fn collect_prompt_asset_dependencies(
    service: &VideoService,
    prompt: &chapter_video_prompt::Model,
    image_ids: &mut BTreeSet<i64>,
    video_ids: &mut BTreeSet<i64>,
) -> Result<(), AppError> {
    if prompt_stage(prompt) == Some(PromptStage::FramePrompt) {
        for image in service.generated_frame_images_for_prompt(prompt).await? {
            image_ids.insert(image.id);
        }
    }
    if prompt_stage(prompt) == Some(PromptStage::Video)
        && let Some(video_id) = prompt.generated_video_id
    {
        video_ids.insert(video_id);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PromptStage {
    ShotAnalysis,
    SegmentPlan,
    FramePrompt,
    Video,
}

fn prompt_stage(row: &chapter_video_prompt::Model) -> Option<PromptStage> {
    match row.prompt_type.as_str() {
        "frame_shot_analysis" | "first_frame_shot_analysis" | "chat_video_shot_analysis" => {
            Some(PromptStage::ShotAnalysis)
        }
        "frame_segment_plan" | "first_frame_segment_plan" | "chat_video_segment_plan" => {
            Some(PromptStage::SegmentPlan)
        }
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt" | "storyboard_prompt" => {
            Some(PromptStage::FramePrompt)
        }
        "video_prompt" => Some(PromptStage::Video),
        _ => None,
    }
}

fn frame_image_generation_source_for_prompt(
    row: &chapter_video_prompt::Model,
) -> Option<&'static str> {
    match row.prompt_type.as_str() {
        "frame_prompt" | "chat_video_frame_prompt" => Some("video_boundary_frame_prompt"),
        "first_frame_prompt" => Some("video_first_frame_prompt"),
        "storyboard_prompt" => Some("video_storyboard_prompt"),
        _ => None,
    }
}

fn frame_prompt_metadata_key(prompt_type: &str) -> Option<(&'static str, &'static str)> {
    match prompt_type {
        "frame_prompt" | "chat_video_frame_prompt" => Some(("boundary_frame", "边界帧提示词")),
        "first_frame_prompt" => Some(("first_frame", "首帧提示词")),
        "storyboard_prompt" => Some(("storyboard_frame", "分镜提示词")),
        _ => None,
    }
}

fn sync_prompt_metadata_after_update(
    prompt_type: &str,
    metadata_json: Option<Value>,
    visual_prompt: &str,
    negative_prompt: Option<&str>,
    duration_secs: i32,
    reference_image_ids_json: Option<&Value>,
) -> Result<Option<Value>, AppError> {
    let Some((metadata_key, label)) = frame_prompt_metadata_key(prompt_type) else {
        return Ok(metadata_json);
    };
    let Some(mut metadata) = metadata_json else {
        return Err(AppError::InvalidArgument(format!(
            "{label}缺少结构化 metadata_json，不能保存编辑后继续用于下游步骤；请重新生成该提示词"
        )));
    };
    let reference_image_ids =
        reference_ids_from_value(reference_image_ids_json).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "{label} reference_image_ids_json 必须是图片 ID 数组，不能保存编辑"
            ))
        })?;
    let Some(root) = metadata.as_object_mut() else {
        return Err(AppError::InvalidArgument(format!(
            "{label} metadata_json 必须是对象，不能保存编辑后继续用于下游步骤"
        )));
    };
    let Some(frame) = root.get_mut(metadata_key).and_then(Value::as_object_mut) else {
        return Err(AppError::InvalidArgument(format!(
            "{label} metadata_json 缺少 {metadata_key} 对象，不能保存编辑后继续用于下游步骤"
        )));
    };
    frame.insert(
        "frame_prompt".to_string(),
        Value::String(visual_prompt.trim().to_string()),
    );
    frame.insert(
        "negative_prompt".to_string(),
        Value::String(negative_prompt.unwrap_or_default().trim().to_string()),
    );
    frame.insert("duration_secs".to_string(), Value::from(duration_secs));
    frame.insert(
        "reference_image_ids".to_string(),
        json!(reference_image_ids),
    );
    Ok(Some(metadata))
}

fn ensure_frame_prompt_workflow_matches_type(
    row: &chapter_video_prompt::Model,
) -> Result<(), AppError> {
    let Some(expected_workflow_mode) = expected_prompt_workflow_mode(&row.prompt_type) else {
        return Err(AppError::InvalidArgument(format!(
            "提示词 #{} 类型 {} 不能生成视频帧图片",
            row.id, row.prompt_type
        )));
    };
    if row.workflow_mode != expected_workflow_mode {
        return Err(AppError::InvalidArgument(format!(
            "视频帧提示词 #{} 类型 {} 必须属于 {} workflow，实际为 {}",
            row.id, row.prompt_type, expected_workflow_mode, row.workflow_mode
        )));
    }
    Ok(())
}

fn prompt_workflow(row: &chapter_video_prompt::Model) -> Option<PromptWorkflowMode> {
    PromptWorkflowMode::from_row(row).ok()
}

fn prompt_same_generation_chain(
    left: &chapter_video_prompt::Model,
    right: &chapter_video_prompt::Model,
) -> bool {
    if left.project_id != right.project_id || left.chapter_id != right.chapter_id {
        return false;
    }
    let left_stage = prompt_stage(left);
    let right_stage = prompt_stage(right);
    let shared_upstream = left_stage != right_stage
        && (prompt_is_shared_video_upstream(left) || prompt_is_shared_video_upstream(right));
    if left_stage != Some(PromptStage::ShotAnalysis)
        && right_stage != Some(PromptStage::ShotAnalysis)
        && !shared_upstream
        && prompt_workflow(left) != prompt_workflow(right)
    {
        return false;
    }
    if left_stage != Some(PromptStage::ShotAnalysis)
        && right_stage != Some(PromptStage::ShotAnalysis)
        && (left.style != right.style || left.aspect_ratio != right.aspect_ratio)
    {
        return false;
    }
    match (prompt_source_hash(left), prompt_source_hash(right)) {
        (Some(left_hash), Some(right_hash)) => left_hash == right_hash,
        (None, None) => true,
        _ => false,
    }
}

fn prompt_same_frame_reference_chain(
    current: &chapter_video_prompt::Model,
    previous: &chapter_video_prompt::Model,
) -> bool {
    if !prompt_same_generation_chain(current, previous)
        || current.prompt_type != previous.prompt_type
        || current.workflow_mode != previous.workflow_mode
    {
        return false;
    }
    let Some(current_metadata) = current.metadata_json.as_ref() else {
        return false;
    };
    let Some(previous_metadata) = previous.metadata_json.as_ref() else {
        return false;
    };
    if current_metadata.get("contract_version") != previous_metadata.get("contract_version") {
        return false;
    }
    if current_metadata.get("source_prompt_type") != previous_metadata.get("source_prompt_type") {
        return false;
    }
    if prompt_type_uses_generation_assets(&current.prompt_type)
        && current_metadata
            .get("generation_asset_fingerprint")
            .is_none()
    {
        return false;
    }
    if current_metadata.get("generation_asset_fingerprint")
        != previous_metadata.get("generation_asset_fingerprint")
    {
        return false;
    }
    PromptOutputLanguage::from_metadata(Some(current_metadata))
        == PromptOutputLanguage::from_metadata(Some(previous_metadata))
}

fn prompt_is_shared_video_upstream(row: &chapter_video_prompt::Model) -> bool {
    row.workflow_mode == "boundary_frames"
        && matches!(
            row.prompt_type.as_str(),
            "frame_shot_analysis" | "frame_segment_plan"
        )
}

fn prompt_source_hash(row: &chapter_video_prompt::Model) -> Option<&str> {
    row.metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn prompt_stage_label(row: &chapter_video_prompt::Model) -> &'static str {
    match row.prompt_type.as_str() {
        "frame_shot_analysis" => "视频镜头分析",
        "frame_segment_plan" => "视频片段规划",
        "first_frame_shot_analysis" => "首帧串联镜头分析",
        "first_frame_segment_plan" => "首帧串联片段规划",
        "chat_video_shot_analysis" => "Chat 视频镜头分析",
        "chat_video_segment_plan" => "Chat 视频片段规划",
        "frame_prompt" => "边界帧提示词",
        "first_frame_prompt" => "首帧提示词",
        "chat_video_frame_prompt" => "Chat 首尾帧提示词",
        "storyboard_prompt" => "分镜提示词",
        "video_prompt" => match prompt_workflow(row) {
            Some(PromptWorkflowMode::FirstFrameChain) => "首帧串联视频提示词",
            Some(PromptWorkflowMode::ChatVideo) => "Chat 视频提示词",
            Some(PromptWorkflowMode::Storyboard) => "分镜视频提示词",
            _ => "视频提示词",
        },
        _ => "视频提示词",
    }
}

fn image_generation_prompt_id(metadata: Option<&Value>) -> Option<i64> {
    let value = metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("prompt_id"))?;
    value.as_i64()
}

fn image_generation_source(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("source"))
        .and_then(Value::as_str)
}

fn image_generation_shot_number(metadata: Option<&Value>) -> Option<i32> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("shot_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
}

fn image_generation_source_hash(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("source_content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn ensure_frame_prompt_generation_source_matches(
    prompt: &chapter_video_prompt::Model,
    generation_source: &str,
) -> Result<&'static str, AppError> {
    let expected_source = frame_image_generation_source_for_prompt(prompt).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "提示词 #{} 不是可成图的视频帧提示词，不能生成视频帧图片",
            prompt.id
        ))
    })?;
    if generation_source.trim() != expected_source {
        return Err(AppError::InvalidArgument(format!(
            "视频帧成图来源 {generation_source} 与{}不匹配",
            prompt_stage_label(prompt)
        )));
    }
    Ok(expected_source)
}

fn effective_boundary_frame_reference_image_ids(
    requested_reference_image_ids: &[i64],
    previous_frame_image_id: i64,
) -> Result<Vec<i64>, AppError> {
    if let Some(index) = requested_reference_image_ids
        .iter()
        .position(|id| *id == previous_frame_image_id)
    {
        if index > 0 {
            return Err(AppError::InvalidArgument(format!(
                "上一帧边界帧图片 #{} 必须是 reference_image_ids 第 1 项，才能保证提示词中的“参考图 1”语义正确",
                previous_frame_image_id
            )));
        }
        return Ok(requested_reference_image_ids.to_vec());
    }

    let mut ids = Vec::with_capacity(requested_reference_image_ids.len() + 1);
    ids.push(previous_frame_image_id);
    for id in requested_reference_image_ids {
        if !ids.contains(id) {
            ids.push(*id);
        }
    }
    Ok(ids)
}

pub(super) fn reusable_rows_are_complete(
    rows: &[chapter_video_prompt::Model],
    expected_count: usize,
) -> bool {
    rows.len() == expected_count
        && rows
            .iter()
            .enumerate()
            .all(|(idx, row)| row.shot_number == (idx + 1) as i32)
}

fn prompt_matches_generation_context(
    row: &chapter_video_prompt::Model,
    context: PromptGenerationContext<'_>,
) -> bool {
    let PromptGenerationContext {
        prompt_type,
        source_metadata,
        style,
        aspect_ratio,
        reference_ids,
        generation_asset_fingerprint,
        source_prompt_type,
        prompt_output_language,
    } = context;
    let Some(expected_workflow_mode) = expected_prompt_workflow_mode(prompt_type) else {
        return false;
    };
    prompt_matches_generation_context_for_workflow(
        row,
        PromptGenerationContextForWorkflow {
            expected_workflow_mode,
            prompt_type,
            source_metadata,
            style,
            aspect_ratio,
            reference_ids,
            generation_asset_fingerprint,
            source_prompt_type,
            prompt_output_language: Some(prompt_output_language),
        },
    )
}

fn prompt_matches_generation_context_for_workflow(
    row: &chapter_video_prompt::Model,
    context: PromptGenerationContextForWorkflow<'_>,
) -> bool {
    if row.prompt_type != context.prompt_type
        || row.workflow_mode != context.expected_workflow_mode
        || row.style.as_deref() != Some(context.style)
        || row.aspect_ratio.as_deref() != Some(context.aspect_ratio)
    {
        return false;
    }
    let Some(metadata) = row.metadata_json.as_ref() else {
        return false;
    };
    let matches_source = metadata
        .get("source")
        .and_then(|source| source.get("content_sha256"))
        == context.source_metadata.get("content_sha256");
    if !matches_source {
        return false;
    }
    let Some(expected_contract) =
        expected_contract_version(context.expected_workflow_mode, context.prompt_type)
    else {
        return false;
    };
    if metadata.get("contract_version").and_then(Value::as_str) != Some(expected_contract) {
        return false;
    }
    if let Some(prompt_output_language) = context.prompt_output_language
        && prompt_type_uses_prompt_output_language(context.prompt_type)
        && !metadata_matches_prompt_output_language(
            row.metadata_json.as_ref(),
            prompt_output_language,
        )
    {
        return false;
    }
    if prompt_type_uses_generation_assets(context.prompt_type) {
        let Some(expected_fingerprint) = context.generation_asset_fingerprint else {
            return false;
        };
        if metadata.get("generation_asset_fingerprint") != Some(expected_fingerprint) {
            return false;
        }
    }
    if context.prompt_type == "video_prompt"
        && context.expected_workflow_mode == "chat_video"
        && !reference_ids_match(
            metadata.get("generation_reference_image_ids"),
            context.reference_ids,
        )
    {
        return false;
    }

    context.source_prompt_type.is_none_or(|expected| {
        metadata.get("source_prompt_type").and_then(Value::as_str) == Some(expected)
    })
}

pub(super) fn prompt_type_uses_prompt_output_language(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "frame_prompt"
            | "first_frame_prompt"
            | "chat_video_frame_prompt"
            | "storyboard_prompt"
            | "video_prompt"
    )
}

fn prompt_type_uses_generation_assets(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "frame_prompt"
            | "first_frame_prompt"
            | "chat_video_frame_prompt"
            | "storyboard_prompt"
            | "video_prompt"
    )
}

fn prompt_matches_shared_shot_analysis_context(
    row: &chapter_video_prompt::Model,
    expected_workflow_mode: &str,
    prompt_type: &str,
    source_metadata: &Value,
) -> bool {
    if prompt_stage(row) != Some(PromptStage::ShotAnalysis)
        || row.prompt_type != prompt_type
        || row.workflow_mode != expected_workflow_mode
    {
        return false;
    }

    let Some(metadata) = row.metadata_json.as_ref() else {
        return false;
    };
    if metadata
        .get("source")
        .and_then(|source| source.get("content_sha256"))
        != source_metadata.get("content_sha256")
    {
        return false;
    }

    metadata.get("contract_version").and_then(Value::as_str)
        == expected_contract_version(expected_workflow_mode, prompt_type)
}

fn expected_contract_version(
    expected_workflow_mode: &str,
    prompt_type: &str,
) -> Option<&'static str> {
    match (expected_workflow_mode, prompt_type) {
        ("boundary_frames", "frame_shot_analysis") => Some(SHOT_ANALYSIS_CONTRACT_VERSION),
        ("boundary_frames", "frame_segment_plan" | "video_prompt") => {
            Some(FRAME_SEGMENT_PLAN_CONTRACT_VERSION)
        }
        ("boundary_frames", "frame_prompt") | ("chat_video", "chat_video_frame_prompt") => {
            Some(BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION)
        }
        ("first_frame_chain", "first_frame_prompt" | "video_prompt") => {
            Some(FIRST_FRAME_CHAIN_CONTRACT_VERSION)
        }
        ("chat_video", "video_prompt") => Some(CHAT_VIDEO_CONTRACT_VERSION),
        ("storyboard", "storyboard_prompt") => Some(STORYBOARD_PROMPT_CONTRACT_VERSION),
        ("storyboard", "video_prompt") => Some(STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION),
        _ => None,
    }
}

fn expected_prompt_workflow_mode(prompt_type: &str) -> Option<&'static str> {
    match prompt_type {
        "frame_shot_analysis" | "frame_segment_plan" | "frame_prompt" => Some("boundary_frames"),
        "first_frame_prompt" => Some("first_frame_chain"),
        "chat_video_frame_prompt" => Some("chat_video"),
        "storyboard_prompt" => Some("storyboard"),
        "video_prompt" => None,
        _ => None,
    }
}

fn reference_ids_match(stored: Option<&Value>, expected: &Value) -> bool {
    reference_ids_from_value(stored) == reference_ids_from_value(Some(expected))
}

fn reference_ids_from_value(value: Option<&Value>) -> Option<Vec<i64>> {
    value?
        .as_array()?
        .iter()
        .map(Value::as_i64)
        .collect::<Option<Vec<_>>>()
}

pub(super) fn video_generation_asset_fingerprint(
    reference_image_ids: &[i64],
    video_library_image_ids: &[i64],
    available_assets: &Value,
) -> Value {
    let available_assets_json = available_assets.to_string();
    serde_json::json!({
        "schema_version": 1,
        "reference_image_ids": reference_image_ids,
        "video_library_image_ids": video_library_image_ids,
        "available_assets_sha256": source_content_sha256(&available_assets_json),
    })
}

fn required_prompt_text(value: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

fn normalize_optional_prompt_text(value: Option<&str>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn prompt_row(id: i64, workflow_mode: &str, prompt_type: &str) -> chapter_video_prompt::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter_video_prompt::Model {
            id,
            project_id: 1,
            chapter_id: 2,
            style: Some("测试风格".to_string()),
            prompt_type: prompt_type.to_string(),
            workflow_mode: workflow_mode.to_string(),
            shot_number: 0,
            title: Some("测试提示词".to_string()),
            scene_summary: None,
            visual_prompt: "{}".to_string(),
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: None,
            status: "completed".to_string(),
            generated_video_id: None,
            metadata_json: Some(json!({
                "source": {"content_sha256": "same-source"},
                "contract_version": SHOT_ANALYSIS_CONTRACT_VERSION
            })),
            created_at: now,
            updated_at: now,
        }
    }

    fn test_generation_context_for_workflow<'a>(
        expected_workflow_mode: &'a str,
        prompt_type: &'a str,
        source_metadata: &'a Value,
        reference_ids: &'a Value,
        generation_asset_fingerprint: Option<&'a Value>,
        source_prompt_type: Option<&'a str>,
        prompt_output_language: PromptOutputLanguage,
    ) -> PromptGenerationContextForWorkflow<'a> {
        PromptGenerationContextForWorkflow {
            expected_workflow_mode,
            prompt_type,
            source_metadata,
            style: "测试风格",
            aspect_ratio: "16:9",
            reference_ids,
            generation_asset_fingerprint,
            source_prompt_type,
            prompt_output_language: Some(prompt_output_language),
        }
    }

    fn test_generation_context<'a>(
        prompt_type: &'a str,
        source_metadata: &'a Value,
        reference_ids: &'a Value,
        generation_asset_fingerprint: Option<&'a Value>,
        source_prompt_type: Option<&'a str>,
        prompt_output_language: PromptOutputLanguage,
    ) -> PromptGenerationContext<'a> {
        PromptGenerationContext {
            prompt_type,
            source_metadata,
            style: "测试风格",
            aspect_ratio: "16:9",
            reference_ids,
            generation_asset_fingerprint,
            source_prompt_type,
            prompt_output_language,
        }
    }

    #[test]
    fn reference_ids_match_requires_exact_order_without_deduping() {
        assert!(reference_ids_match(
            Some(&json!([6, 1, 2])),
            &json!([6, 1, 2])
        ));
        assert!(!reference_ids_match(
            Some(&json!([1, 2, 6])),
            &json!([6, 1, 2])
        ));
        assert!(!reference_ids_match(
            Some(&json!([6, 1, 6, 2])),
            &json!([6, 1, 2])
        ));
    }

    #[test]
    fn effective_boundary_frame_reference_ids_prepend_previous_image() {
        assert_eq!(
            effective_boundary_frame_reference_image_ids(&[11, 12], 10).unwrap(),
            vec![10, 11, 12]
        );
        assert_eq!(
            effective_boundary_frame_reference_image_ids(&[10, 11], 10).unwrap(),
            vec![10, 11]
        );
    }

    #[test]
    fn effective_boundary_frame_reference_ids_reject_previous_image_not_first() {
        let err = effective_boundary_frame_reference_image_ids(&[11, 10], 10)
            .expect_err("previous boundary frame image must be the first reference");

        assert!(matches!(
            err,
            AppError::InvalidArgument(message)
                if message.contains("上一帧边界帧图片 #10")
                    && message.contains("第 1 项")
        ));
    }

    #[test]
    fn shared_segment_plan_context_does_not_depend_on_generation_reference_ids() {
        let source_metadata = json!({"content_sha256": "same-source"});
        let mut segment = prompt_row(1, "boundary_frames", "frame_segment_plan");
        segment.reference_image_ids_json = Some(json!([1]));
        segment.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "generation_reference_image_ids": [1],
            "prompt_output_language": "en"
        }));

        assert!(prompt_matches_generation_context_for_workflow(
            &segment,
            test_generation_context_for_workflow(
                "boundary_frames",
                "frame_segment_plan",
                &source_metadata,
                &json!([2]),
                None,
                None,
                PromptOutputLanguage::ZhCn,
            ),
        ));
    }

    #[test]
    fn deprecated_upstream_prompt_types_do_not_match_generation_context() {
        let source_metadata = json!({"content_sha256": "same-source"});
        let mut first_frame_segment =
            prompt_row(1, "first_frame_chain", "first_frame_segment_plan");
        first_frame_segment.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION
        }));
        let mut chat_segment = prompt_row(2, "chat_video", "chat_video_segment_plan");
        chat_segment.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": CHAT_VIDEO_CONTRACT_VERSION
        }));

        assert!(!prompt_matches_generation_context_for_workflow(
            &first_frame_segment,
            test_generation_context_for_workflow(
                "first_frame_chain",
                "first_frame_segment_plan",
                &source_metadata,
                &json!([]),
                None,
                None,
                PromptOutputLanguage::ZhCn,
            ),
        ));
        assert!(!prompt_matches_generation_context_for_workflow(
            &chat_segment,
            test_generation_context_for_workflow(
                "chat_video",
                "chat_video_segment_plan",
                &source_metadata,
                &json!([]),
                None,
                None,
                PromptOutputLanguage::ZhCn,
            ),
        ));
    }

    #[test]
    fn chat_video_prompt_context_keeps_generation_reference_snapshot() {
        let source_metadata = json!({"content_sha256": "same-source"});
        let asset_fingerprint = json!({
            "schema_version": 1,
            "reference_image_ids": [1],
            "video_library_image_ids": [],
            "available_assets_sha256": "same-assets"
        });
        let mut chat_video_prompt = prompt_row(1, "chat_video", "video_prompt");
        chat_video_prompt.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": CHAT_VIDEO_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_reference_image_ids": [1],
            "generation_asset_fingerprint": asset_fingerprint.clone()
        }));

        assert!(prompt_matches_generation_context_for_workflow(
            &chat_video_prompt,
            test_generation_context_for_workflow(
                "chat_video",
                "video_prompt",
                &source_metadata,
                &json!([1]),
                Some(&asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::ZhCn,
            ),
        ));
        assert!(!prompt_matches_generation_context_for_workflow(
            &chat_video_prompt,
            test_generation_context_for_workflow(
                "chat_video",
                "video_prompt",
                &source_metadata,
                &json!([2]),
                Some(&asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::ZhCn,
            ),
        ));
    }

    #[test]
    fn final_prompt_context_matches_prompt_output_language() {
        let source_metadata = json!({"content_sha256": "same-source"});
        let asset_fingerprint = json!({
            "schema_version": 1,
            "reference_image_ids": [],
            "video_library_image_ids": [],
            "available_assets_sha256": "same-assets"
        });
        let mut video_prompt = prompt_row(1, "first_frame_chain", "video_prompt");
        video_prompt.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_asset_fingerprint": asset_fingerprint.clone(),
            "prompt_output_language": "en"
        }));

        assert!(prompt_matches_generation_context_for_workflow(
            &video_prompt,
            test_generation_context_for_workflow(
                "first_frame_chain",
                "video_prompt",
                &source_metadata,
                &json!([]),
                Some(&asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::En,
            ),
        ));
        assert!(!prompt_matches_generation_context_for_workflow(
            &video_prompt,
            test_generation_context_for_workflow(
                "first_frame_chain",
                "video_prompt",
                &source_metadata,
                &json!([]),
                Some(&asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::ZhCn,
            ),
        ));
    }

    #[test]
    fn final_prompt_context_requires_explicit_workflow_after_shared_segment_plan() {
        let source_metadata = json!({"content_sha256": "same-source"});
        let asset_fingerprint = json!({
            "schema_version": 1,
            "reference_image_ids": [],
            "video_library_image_ids": [],
            "available_assets_sha256": "same-assets"
        });
        let mut video_prompt = prompt_row(1, "first_frame_chain", "video_prompt");
        video_prompt.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_asset_fingerprint": asset_fingerprint.clone(),
            "prompt_output_language": "zh_cn"
        }));

        assert!(
            !prompt_matches_generation_context(
                &video_prompt,
                test_generation_context(
                    "video_prompt",
                    &source_metadata,
                    &json!([]),
                    Some(&asset_fingerprint),
                    Some("frame_segment_plan"),
                    PromptOutputLanguage::ZhCn,
                ),
            ),
            "source_prompt_type=frame_segment_plan is shared and must not imply boundary_frames"
        );
        assert!(prompt_matches_generation_context_for_workflow(
            &video_prompt,
            test_generation_context_for_workflow(
                "first_frame_chain",
                "video_prompt",
                &source_metadata,
                &json!([]),
                Some(&asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::ZhCn,
            ),
        ));
    }

    #[test]
    fn frame_prompt_context_rejects_changed_generation_asset_fingerprint() {
        let source_metadata = json!({"content_sha256": "same-source"});
        let stored_asset_fingerprint = json!({
            "schema_version": 1,
            "reference_image_ids": [1],
            "video_library_image_ids": [],
            "available_assets_sha256": "old-assets"
        });
        let current_asset_fingerprint = json!({
            "schema_version": 1,
            "reference_image_ids": [1],
            "video_library_image_ids": [9],
            "available_assets_sha256": "new-assets"
        });
        let mut first_frame_prompt = prompt_row(1, "first_frame_chain", "first_frame_prompt");
        first_frame_prompt.metadata_json = Some(json!({
            "source": {"content_sha256": "same-source"},
            "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "generation_asset_fingerprint": stored_asset_fingerprint
        }));

        assert!(!prompt_matches_generation_context_for_workflow(
            &first_frame_prompt,
            test_generation_context_for_workflow(
                "first_frame_chain",
                "first_frame_prompt",
                &source_metadata,
                &json!([1]),
                Some(&current_asset_fingerprint),
                Some("frame_segment_plan"),
                PromptOutputLanguage::ZhCn,
            ),
        ));
    }

    #[test]
    fn frame_prompt_image_generation_rejects_workflow_type_mismatch() {
        let row = prompt_row(1, "chat_video", "first_frame_prompt");
        let err = ensure_frame_prompt_workflow_matches_type(&row)
            .expect_err("first_frame_prompt must not run under chat_video workflow");

        assert!(matches!(
            err,
            AppError::InvalidArgument(message)
                if message.contains("first_frame_prompt")
                    && message.contains("first_frame_chain")
                    && message.contains("chat_video")
        ));
    }

    #[test]
    fn frame_prompt_edit_syncs_structured_metadata_copy() {
        let metadata = json!({
            "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
            "boundary_frame": {
                "frame_number": 1,
                "title": "旧标题",
                "scene_summary": "旧摘要",
                "frame_prompt": "旧帧提示词",
                "negative_prompt": "旧负面词",
                "duration_secs": 4,
                "aspect_ratio": "16:9",
                "reference_image_ids": [1],
                "segment_numbers": [1],
                "blocking_state": {},
                "lighting_state": {}
            }
        });

        let synced = sync_prompt_metadata_after_update(
            "frame_prompt",
            Some(metadata),
            " 新帧提示词 ",
            Some(" 新负面词 "),
            6,
            Some(&json!([2, 3])),
        )
        .unwrap()
        .unwrap();
        let frame = synced
            .get("boundary_frame")
            .and_then(Value::as_object)
            .expect("boundary frame metadata should remain an object");

        assert_eq!(
            frame.get("frame_prompt").and_then(Value::as_str),
            Some("新帧提示词")
        );
        assert_eq!(
            frame.get("negative_prompt").and_then(Value::as_str),
            Some("新负面词")
        );
        assert_eq!(frame.get("duration_secs").and_then(Value::as_i64), Some(6));
        assert_eq!(frame.get("reference_image_ids"), Some(&json!([2, 3])));
        assert_eq!(frame.get("title").and_then(Value::as_str), Some("旧标题"));
    }

    #[test]
    fn shot_analysis_chain_is_shared_across_video_workflows() {
        let mut shared_shot_analysis = prompt_row(1, "boundary_frames", "frame_shot_analysis");
        let first_frame_segment = prompt_row(2, "first_frame_chain", "first_frame_segment_plan");
        let boundary_segment = prompt_row(3, "boundary_frames", "frame_segment_plan");
        shared_shot_analysis.style = Some("Chat 模式风格".to_string());
        shared_shot_analysis.aspect_ratio = Some("9:16".to_string());

        assert!(prompt_same_generation_chain(
            &shared_shot_analysis,
            &first_frame_segment
        ));
        assert!(!prompt_same_generation_chain(
            &boundary_segment,
            &first_frame_segment
        ));
    }

    #[test]
    fn boundary_frame_prompt_uses_separate_contract_from_segment_plan() {
        assert_eq!(
            expected_contract_version("boundary_frames", "frame_segment_plan"),
            Some(FRAME_SEGMENT_PLAN_CONTRACT_VERSION)
        );
        assert_eq!(
            expected_contract_version("boundary_frames", "frame_prompt"),
            Some(BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION)
        );
        assert_eq!(
            expected_contract_version("first_frame_chain", "frame_segment_plan"),
            None
        );
        assert_eq!(
            expected_contract_version("chat_video", "frame_segment_plan"),
            None
        );
    }

    #[test]
    fn image_generation_prompt_id_requires_json_integer() {
        assert_eq!(
            image_generation_prompt_id(Some(&json!({
                "generation_context": {"prompt_id": 12}
            }))),
            Some(12)
        );
        assert_eq!(
            image_generation_prompt_id(Some(&json!({
                "generation_context": {"prompt_id": "12"}
            }))),
            None
        );
    }
}
