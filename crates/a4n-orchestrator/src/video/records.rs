use std::collections::HashSet;

use a4n_core::{
    error::AppError,
    models::{
        ai::{generated_image, generated_video},
        chapter::chapter_video_prompt,
    },
    source_spans::{metadata_source_content_sha256, source_text_hash},
};
use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;

use super::{
    ASSEMBLY_INSERT_VIDEO_TYPE, ASSEMBLY_POSTPROCESS_EDIT_VIDEO_TYPE,
    ASSEMBLY_PUSH_STATUS_ACCEPTED, ASSEMBLY_PUSH_STATUS_PENDING, ASSEMBLY_PUSH_STATUS_REJECTED,
    ASSEMBLY_PUSH_VIDEO_TYPE, AssemblyClipOrderAnchor, active_assembly_push_source_ids,
    assembly_clip_display_shot_number, assembly_clip_insert_after_video_id,
    assembly_clip_sort_order, assembly_insert_metadata_with_position,
    assembly_push_metadata_with_status, assembly_push_records,
    ensure_insert_position_not_inside_pushed_prefix, is_assembly_clip_video_type,
    latest_non_rejected_push_index, next_assembly_insert_sort_order,
    normalized_assembly_dimensions, normalized_assembly_fps, parse_assembly_push_metadata,
    parse_postprocess_edit_metadata, postprocess_edit_range_bounds,
    selected_assembly_push_end_index, validate_assembly_workflow_mode,
    validate_postprocess_operation, validate_reference_media_import_options,
    validate_video_trim_range, video_shot_number, video_source_content_hash,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGeneratedVideoInput {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    #[serde(default)]
    pub clear_negative_prompt: bool,
    #[serde(default)]
    pub metadata_json: Option<Value>,
    #[serde(default)]
    pub clear_metadata_json: bool,
    #[serde(default)]
    pub is_favorite: Option<bool>,
    #[serde(default)]
    pub insert_after_video_id: Option<i64>,
    #[serde(default)]
    pub move_assembly_insert_to_start: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAssemblyPushStatusInput {
    pub action: String,
}

#[async_trait]
pub trait VideoRecordLifecycleHostPort: Send + Sync {
    async fn delete_video_tail_frame_assets(&self, video_id: i64) -> Result<(), AppError>;
}

#[derive(Debug, Clone)]
pub struct AssemblyPushClipInput {
    pub video_id: i64,
    pub sort_order: f64,
    pub duration_secs: f32,
    pub width: i32,
    pub height: i32,
    pub fps: i32,
}

#[derive(Debug, Clone)]
pub struct AssemblyVideoRecords {
    pub workflow_mode: String,
    pub clips: Vec<AssemblyClipRecord>,
}

#[derive(Debug, Clone)]
pub struct AssemblyClipRecord {
    pub video_id: i64,
    pub video_type: String,
    pub shot_number: i32,
    pub sort_order: f64,
    pub insert_after_video_id: Option<i64>,
    pub file_path: String,
    pub duration_secs: f32,
    pub width: i32,
    pub height: i32,
    pub fps: i32,
}

#[derive(Debug, Clone)]
pub struct PreparedAssemblyPushRecord {
    pub video: generated_video::Model,
    pub source_video_ids: Vec<i64>,
    pub push_index: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct ImportedVideoProperties {
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
}

#[derive(Debug, Clone)]
pub struct PreparedImportedChapterVideoRecord {
    pub video: generated_video::Model,
    pub prompt: chapter_video_prompt::Model,
}

#[derive(Debug, Clone)]
pub struct CanvasAssemblyPlan {
    pub sources: Vec<generated_video::Model>,
    pub title: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
}

#[derive(Debug, Clone)]
pub struct CanvasFramePlan {
    pub video: generated_video::Model,
    pub frame_kind: String,
    pub timestamp: f32,
}

#[derive(Debug, Clone)]
pub struct VideoTrimPlan {
    pub source: generated_video::Model,
    pub start_secs: f32,
    pub end_secs: f32,
    pub target_workflow_mode: String,
}

#[derive(Debug, Clone)]
pub struct AssemblyPostprocessPlan {
    pub push: generated_video::Model,
    pub base: generated_video::Model,
    pub operation: &'static str,
}

#[derive(Debug, Clone)]
pub enum AssemblyPostprocessSourcePlan {
    None,
    Existing(generated_video::Model),
    Upload,
}

#[derive(Debug, Clone)]
pub struct PreparedAssemblyPostprocessEditRecord {
    pub video: generated_video::Model,
    pub base: generated_video::Model,
    pub operation: &'static str,
    pub start_secs: f32,
    pub end_secs: f32,
}

#[derive(Debug, Clone)]
pub struct AssemblyPostprocessEditRecordInput {
    pub at_secs: Option<f32>,
    pub start_secs: Option<f32>,
    pub end_secs: Option<f32>,
    pub source_video_id: Option<i64>,
    pub source_file_name: Option<String>,
    pub source_duration_secs: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct VideoGenerationRecordInput {
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,
    pub metadata_json: Option<Value>,
    pub provider_request_snapshot: Value,
    pub provider_backend: String,
    pub step_key: String,
    pub title: String,
    pub prompt_template_name: Option<String>,
    pub prompt_id: Option<i64>,
}

impl crate::OrchestratorContext {
    pub async fn create_video_generation_record_validated(
        &self,
        input: VideoGenerationRecordInput,
    ) -> Result<generated_video::Model, a4n_core::error::AppError> {
        let video_record = a4n_db::video::CreateGeneratedVideoRecordDraftInput {
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            video_type: input.video_type,
            workflow_mode: input.workflow_mode,
            prompt: input.prompt,
            negative_prompt: input.negative_prompt,
            model: input.model,
            width: input.width,
            height: input.height,
            duration_secs: input.duration_secs,
            fps: input.fps,
            source_video_id: input.source_video_id,
            source_image_id: input.source_image_id,
            provider_job_id: None,
            metadata_json: input.metadata_json,
        };
        let generation_run = crate::video::started_video_generation_run_input(
            &video_record,
            &input.step_key,
            &input.title,
            input.provider_backend,
            input.prompt_template_name.as_deref(),
            input.provider_request_snapshot,
        );
        match input.prompt_id {
            Some(prompt_id) => {
                self.create_prompt_video_record_with_generation_run_validated(
                    video_record,
                    generation_run,
                    prompt_id,
                )
                .await
            }
            None => {
                self.create_video_record_with_generation_run_validated(video_record, generation_run)
                    .await
            }
        }
    }

    pub async fn list_video_records_by_chapter_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_video::Model>, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, "查看章节视频", false)
            .await?;
        self.generated_videos_for_chapter(project_id, chapter_id)
            .await
    }

    pub async fn toggle_video_record_favorite_validated(
        &self,
        video_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        self.required_video_record(video_id).await?;
        self.toggle_generated_video_favorite(video_id).await
    }

    pub async fn select_segment_video_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        segment_plan_prompt_id: i64,
        shot_number: i32,
        video_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, "采用片段视频", true)
            .await?;
        let plan = self
            .require_current_shared_segment_plan(project_id, chapter_id, segment_plan_prompt_id)
            .await?;
        let mut recipe = a4n_core::video_execution::video_segment_recipe_from_metadata(
            plan.metadata_json.as_ref(),
            shot_number,
        )
        .map_err(AppError::InvalidArgument)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("当前规划缺少片段 {shot_number} 的执行方案"))
        })?;
        let selected = self.required_video_record(video_id).await?;
        let selected_shot = selected
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("shot_number"))
            .and_then(Value::as_i64)
            .and_then(|value| i32::try_from(value).ok());
        let plan_identity = plan
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity"))
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少执行身份".to_string()))?;
        let expected_identity = super::prompt_generation::video_segment_execution_identity(
            plan_identity.clone(),
            Some(plan.id),
            Some(&recipe),
        )?;
        if selected.project_id != project_id
            || selected.chapter_id != Some(chapter_id)
            || selected.status != "completed"
            || selected
                .file_path
                .as_deref()
                .map(str::trim)
                .is_none_or(str::is_empty)
            || selected_shot != Some(shot_number)
            || selected.workflow_mode.as_deref() != Some(recipe.workflow_mode.as_str())
            || !super::prompt_records::video_execution_segment_identity_matches(
                selected.metadata_json.as_ref(),
                &expected_identity,
            )
        {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{video_id} 不属于当前规划片段 {shot_number} 的所选执行方案"
            )));
        }
        recipe.adopted_video_id = Some(video_id);
        self.update_chapter_video_segment_recipe(plan.id, recipe)
            .await?;
        self.sync_project_asset_lineage_after_commit(project_id, "同步视频方案资产谱系")
            .await;
        Ok(selected)
    }

    pub async fn update_video_record_validated(
        &self,
        video_id: i64,
        input: UpdateGeneratedVideoInput,
    ) -> Result<generated_video::Model, AppError> {
        let existing = self.required_video_record(video_id).await?;
        let modifies_pushed_source = input.prompt.is_some()
            || input.negative_prompt.is_some()
            || input.clear_negative_prompt
            || input.metadata_json.is_some()
            || input.clear_metadata_json
            || input.insert_after_video_id.is_some()
            || input.move_assembly_insert_to_start;
        if modifies_pushed_source && self.video_record_is_pushed_source(&existing).await? {
            return Err(AppError::InvalidArgument(
                "该视频片段已推送到成片后处理，不能修改".to_string(),
            ));
        }

        let mut metadata_json = input.metadata_json;
        let mut clear_metadata_json = input.clear_metadata_json;
        if input.move_assembly_insert_to_start || input.insert_after_video_id.is_some() {
            if clear_metadata_json {
                return Err(AppError::InvalidArgument(
                    "调整插入片段位置时不能同时清空视频 metadata".to_string(),
                ));
            }
            let target_after_video_id = if input.move_assembly_insert_to_start {
                None
            } else {
                input.insert_after_video_id
            };
            metadata_json = Some(
                self.resolve_assembly_insert_position_metadata(
                    &existing,
                    metadata_json.or_else(|| existing.metadata_json.clone()),
                    target_after_video_id,
                )
                .await?,
            );
            clear_metadata_json = false;
        }

        self.update_generated_video_record(
            video_id,
            a4n_db::video::UpdateGeneratedVideoRecordInput {
                prompt: input.prompt,
                negative_prompt: input.negative_prompt,
                clear_negative_prompt: input.clear_negative_prompt,
                metadata_json,
                clear_metadata_json,
                is_favorite: input.is_favorite,
            },
        )
        .await
    }

    pub async fn delete_video_record_validated(
        &self,
        host_port: &dyn VideoRecordLifecycleHostPort,
        video_id: i64,
    ) -> Result<(), AppError> {
        let Some(existing) = self.generated_video_by_id(video_id).await? else {
            self.delete_missing_generated_video_references(video_id)
                .await?;
            return Ok(());
        };
        if self.video_record_is_pushed_source(&existing).await? {
            return Err(AppError::InvalidArgument(
                "该视频片段已推送到成片后处理，不能删除".to_string(),
            ));
        }
        if let Some(run) = self
            .latest_workflow_run_by_business_key(&format!("video_provider_job:{video_id}"))
            .await?
        {
            self.cancel_workflow_run_and_interrupt_videos(run.id, "关联视频已删除")
                .await?;
        }
        host_port.delete_video_tail_frame_assets(video_id).await?;
        self.delete_generated_video_record_with_cleanup(video_id)
            .await
    }

    pub async fn update_assembly_push_status_validated(
        &self,
        video_id: i64,
        input: UpdateAssemblyPushStatusInput,
    ) -> Result<generated_video::Model, AppError> {
        let existing = self.required_video_record(video_id).await?;
        if existing.video_type != ASSEMBLY_PUSH_VIDEO_TYPE {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{} 不是成片后处理推送批次",
                existing.id
            )));
        }
        let chapter_id = existing.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument(format!("推送批次 #{} 缺少章节绑定", existing.id))
        })?;
        let workflow_mode = required_workflow_mode(&existing, "推送批次")?;
        let rows = self
            .generated_videos_for_chapter(existing.project_id, chapter_id)
            .await?;
        let pushes =
            assembly_push_records(&rows, workflow_mode).map_err(AppError::InvalidArgument)?;
        let latest_index =
            latest_non_rejected_push_index(pushes.iter().map(|push| push.status.as_str()))
                .ok_or_else(|| {
                    AppError::InvalidArgument("当前章节没有可管理的推送批次".to_string())
                })?;
        if pushes[latest_index].video.id != existing.id {
            return Err(AppError::InvalidArgument(
                "只能管理当前最新推送批次，不能跨推送操作".to_string(),
            ));
        }
        let next_status =
            next_assembly_push_status(input.action.trim(), pushes[latest_index].status.as_str())?;
        self.update_generated_video_record(
            existing.id,
            a4n_db::video::UpdateGeneratedVideoRecordInput {
                prompt: None,
                negative_prompt: None,
                clear_negative_prompt: false,
                metadata_json: Some(assembly_push_metadata_with_status(
                    existing.metadata_json,
                    next_status,
                )),
                clear_metadata_json: false,
                is_favorite: None,
            },
        )
        .await
    }

    pub async fn assembly_video_records_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        operation: &str,
    ) -> Result<AssemblyVideoRecords, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, operation, false)
            .await?;
        let workflow_mode =
            validate_assembly_workflow_mode(workflow_mode).map_err(AppError::InvalidArgument)?;
        let rows = self
            .generated_videos_for_chapter(project_id, chapter_id)
            .await?;
        let adopted_video_ids = if workflow_mode == "mixed" {
            Some(
                self.current_mixed_adopted_video_ids(project_id, chapter_id, &rows)
                    .await?,
            )
        } else {
            None
        };
        Ok(AssemblyVideoRecords {
            workflow_mode: workflow_mode.to_string(),
            clips: assembly_clip_records_from_rows(
                &rows,
                project_id,
                chapter_id,
                workflow_mode,
                None,
                adopted_video_ids.as_ref(),
            )?,
        })
    }

    async fn current_mixed_adopted_video_ids(
        &self,
        project_id: i64,
        chapter_id: i64,
        videos: &[generated_video::Model],
    ) -> Result<HashSet<i64>, AppError> {
        let plan = self
            .current_shared_segment_plan(project_id, chapter_id)
            .await?;
        let segments = plan
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("segments"))
            .and_then(Value::as_array)
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少 segments".to_string()))?;
        let plan_style = plan
            .style
            .as_deref()
            .map(str::trim)
            .filter(|style| !style.is_empty())
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少 style".to_string()))?;
        let recipes = a4n_core::video_execution::video_segment_recipes_for_plan_from_metadata(
            plan.metadata_json.as_ref(),
            plan_style,
        )
        .map_err(AppError::InvalidArgument)?;
        let plan_identity = plan
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity"))
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少执行身份".to_string()))?;
        let mut adopted = HashSet::new();
        for segment in segments {
            let segment_number = segment
                .get("segment_number")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
                .filter(|value| *value > 0)
                .ok_or_else(|| {
                    AppError::InvalidArgument("当前片段规划包含无效 segment_number".to_string())
                })?;
            let recipe = recipes
                .iter()
                .find(|recipe| recipe.segment_number == segment_number)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "片段 {segment_number} 尚未保存执行方案，不能合成"
                    ))
                })?;
            let expected_identity = super::prompt_generation::video_segment_execution_identity(
                plan_identity.clone(),
                Some(plan.id),
                Some(recipe),
            )?;
            let video_id = recipe.adopted_video_id.ok_or_else(|| {
                AppError::InvalidArgument(format!("片段 {segment_number} 尚未采用视频，不能合成"))
            })?;
            if !adopted.insert(video_id) {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{video_id} 被多个片段重复采用"
                )));
            }
            let video = videos
                .iter()
                .find(|video| video.id == video_id)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!("采用的视频 #{video_id} 不存在"))
                })?;
            let video_segment = video
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("shot_number"))
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok());
            if video.project_id != project_id
                || video.chapter_id != Some(chapter_id)
                || video.status != "completed"
                || video
                    .file_path
                    .as_deref()
                    .map(str::trim)
                    .is_none_or(str::is_empty)
                || video_segment != Some(segment_number)
                || video.workflow_mode.as_deref() != Some(recipe.workflow_mode.as_str())
                || !super::prompt_records::video_execution_segment_identity_matches(
                    video.metadata_json.as_ref(),
                    &expected_identity,
                )
            {
                return Err(AppError::InvalidArgument(format!(
                    "片段 {segment_number} 采用的视频 #{video_id} 已失效或不属于当前规划"
                )));
            }
        }
        Ok(adopted)
    }

    async fn prepare_assembly_insert_metadata_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        base_metadata: Option<Value>,
        insert_after_video_id: Option<i64>,
    ) -> Result<Value, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, "导入衔接片段", true)
            .await?;
        let workflow_mode =
            validate_assembly_workflow_mode(workflow_mode).map_err(AppError::InvalidArgument)?;
        let rows = self
            .generated_videos_for_chapter(project_id, chapter_id)
            .await?;
        let adopted_video_ids = if workflow_mode == "mixed" {
            Some(
                self.current_mixed_adopted_video_ids(project_id, chapter_id, &rows)
                    .await?,
            )
        } else {
            None
        };
        assembly_insert_position_metadata_from_rows(
            &rows,
            project_id,
            chapter_id,
            workflow_mode,
            None,
            adopted_video_ids.as_ref(),
            base_metadata,
            insert_after_video_id,
        )
    }

    pub async fn create_imported_assembly_insert_video_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        insert_after_video_id: Option<i64>,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<generated_video::Model, AppError> {
        let workflow_mode =
            validate_assembly_workflow_mode(workflow_mode).map_err(AppError::InvalidArgument)?;
        let metadata = self
            .prepare_assembly_insert_metadata_validated(
                project_id,
                chapter_id,
                workflow_mode,
                Some(serde_json::json!({
                    "assembly_insert": {
                        "source_file_name": source_file_name,
                        "source_extension": source_extension,
                        "imported_at": imported_at,
                    }
                })),
                insert_after_video_id,
            )
            .await?;
        let anchor_label = insert_after_video_id
            .map(|id| format!("视频 #{id} 之后"))
            .unwrap_or_else(|| "成片开头".to_string());
        self.create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: ASSEMBLY_INSERT_VIDEO_TYPE.to_string(),
            workflow_mode: Some(workflow_mode.to_string()),
            prompt: format!("成片衔接片段，插入位置：{anchor_label}。"),
            negative_prompt: None,
            model: "external-assembly-insert".to_string(),
            width: properties.width,
            height: properties.height,
            duration_secs: properties.duration_secs,
            fps: properties.fps,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(metadata),
        })
        .await
    }

    pub async fn prepare_assembly_postprocess_edit_validated(
        &self,
        push_video_id: i64,
        operation: &str,
    ) -> Result<AssemblyPostprocessPlan, AppError> {
        let push = self.required_video_record(push_video_id).await?;
        if push.video_type != ASSEMBLY_PUSH_VIDEO_TYPE {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{} 不是后处理推送批次",
                push.id
            )));
        }
        if parse_assembly_push_metadata(push.id, push.metadata_json.as_ref())
            .map_err(AppError::InvalidArgument)?
            .status
            == ASSEMBLY_PUSH_STATUS_REJECTED
        {
            return Err(AppError::InvalidArgument(
                "已拒绝的推送批次不能继续编辑".to_string(),
            ));
        }
        let chapter_id = push.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument(format!("推送批次 #{} 缺少章节绑定", push.id))
        })?;
        let workflow_mode = required_workflow_mode(&push, "推送批次")?;
        let rows = self
            .generated_videos_for_chapter(push.project_id, chapter_id)
            .await?;
        let pushes =
            assembly_push_records(&rows, workflow_mode).map_err(AppError::InvalidArgument)?;
        let latest_index = latest_non_rejected_push_index(
            pushes.iter().map(|candidate| candidate.status.as_str()),
        )
        .ok_or_else(|| AppError::InvalidArgument("当前章节没有可编辑的推送批次".to_string()))?;
        if pushes[latest_index].video.id != push.id {
            return Err(AppError::InvalidArgument(
                "只能编辑当前最新推送批次，不能跨推送操作".to_string(),
            ));
        }
        let base = self
            .latest_assembly_postprocess_version_for_push(&push)
            .await?;
        let operation =
            validate_postprocess_operation(operation).map_err(AppError::InvalidArgument)?;
        Ok(AssemblyPostprocessPlan {
            push,
            base,
            operation,
        })
    }

    pub async fn create_assembly_postprocess_edit_record_validated(
        &self,
        plan: AssemblyPostprocessPlan,
        input: AssemblyPostprocessEditRecordInput,
    ) -> Result<PreparedAssemblyPostprocessEditRecord, AppError> {
        let (start_secs, end_secs, source_duration_secs) = postprocess_edit_range_bounds(
            plan.base.duration_secs,
            plan.operation,
            input.at_secs,
            input.start_secs,
            input.end_secs,
            input.source_duration_secs,
        )
        .map_err(AppError::InvalidArgument)?;
        let output_duration = match plan.operation {
            "insert" => plan.base.duration_secs.max(0.0) + source_duration_secs.unwrap_or(0.0),
            "delete" => (plan.base.duration_secs - (end_secs - start_secs)).max(0.1),
            "replace" => (plan.base.duration_secs - (end_secs - start_secs)
                + source_duration_secs.unwrap_or(0.0))
            .max(0.1),
            _ => plan.base.duration_secs,
        };
        let video = self
            .create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
                project_id: plan.push.project_id,
                chapter_id: plan.push.chapter_id,
                video_type: ASSEMBLY_POSTPROCESS_EDIT_VIDEO_TYPE.to_string(),
                workflow_mode: plan.push.workflow_mode.clone(),
                prompt: format!("后处理编辑：推送批次 #{} {}", plan.push.id, plan.operation),
                negative_prompt: None,
                model: "a4n-postprocess-edit".to_string(),
                width: plan.base.width,
                height: plan.base.height,
                duration_secs: output_duration,
                fps: plan.base.fps,
                source_video_id: Some(plan.base.id),
                source_image_id: None,
                provider_job_id: None,
                metadata_json: Some(serde_json::json!({
                    "postprocess_edit": {
                        "push_video_id": plan.push.id,
                        "base_video_id": plan.base.id,
                        "operation": plan.operation,
                        "at_secs": input.at_secs,
                        "start_secs": start_secs,
                        "end_secs": end_secs,
                        "source_video_id": input.source_video_id,
                        "source_file_name": input.source_file_name,
                        "edited_at": Utc::now().to_rfc3339(),
                    }
                })),
            })
            .await?;
        Ok(PreparedAssemblyPostprocessEditRecord {
            video,
            base: plan.base,
            operation: plan.operation,
            start_secs,
            end_secs,
        })
    }

    pub async fn prepare_assembly_postprocess_source_validated(
        &self,
        plan: &AssemblyPostprocessPlan,
        source_video_id: Option<i64>,
        has_uploaded_source: bool,
    ) -> Result<AssemblyPostprocessSourcePlan, AppError> {
        if plan.operation == "delete" {
            return Ok(AssemblyPostprocessSourcePlan::None);
        }
        if source_video_id.is_some() && has_uploaded_source {
            return Err(AppError::InvalidArgument(
                "插入或替换后处理片段只能选择一个来源视频".to_string(),
            ));
        }
        if let Some(video_id) = source_video_id {
            let video = self.required_video_record(video_id).await?;
            if video.project_id != plan.push.project_id {
                return Err(AppError::InvalidArgument(format!(
                    "替换/插入来源视频 #{} 不属于当前项目",
                    video.id
                )));
            }
            if video.status != "completed" {
                return Err(AppError::InvalidArgument(format!(
                    "替换/插入来源视频 #{} 尚未完成",
                    video.id
                )));
            }
            if video
                .file_path
                .as_deref()
                .map(str::trim)
                .is_none_or(|path| path.is_empty())
            {
                return Err(AppError::InvalidArgument(format!(
                    "替换/插入来源视频 #{} 没有本地文件",
                    video.id
                )));
            }
            return Ok(AssemblyPostprocessSourcePlan::Existing(video));
        }
        if has_uploaded_source {
            return Ok(AssemblyPostprocessSourcePlan::Upload);
        }
        Err(AppError::InvalidArgument(
            "插入或替换后处理片段时必须选择上传视频或 API 生成视频".to_string(),
        ))
    }

    async fn latest_assembly_postprocess_version_for_push(
        &self,
        push: &generated_video::Model,
    ) -> Result<generated_video::Model, AppError> {
        let chapter_id = push.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument(format!("推送批次 #{} 缺少章节绑定", push.id))
        })?;
        let mut versions = Vec::new();
        for row in self
            .generated_videos_for_chapter(push.project_id, chapter_id)
            .await?
        {
            if row.video_type != ASSEMBLY_POSTPROCESS_EDIT_VIDEO_TYPE || row.status != "completed" {
                continue;
            }
            let metadata = parse_postprocess_edit_metadata(row.id, row.metadata_json.as_ref())
                .map_err(AppError::InvalidArgument)?;
            if metadata.push_video_id != push.id {
                continue;
            }
            if row.source_video_id != Some(metadata.base_video_id) {
                return Err(AppError::InvalidArgument(format!(
                    "后处理编辑视频 #{} 的 source_video_id 与 postprocess_edit.base_video_id 不一致",
                    row.id
                )));
            }
            if row.workflow_mode.as_deref() != push.workflow_mode.as_deref() {
                return Err(AppError::InvalidArgument(format!(
                    "后处理编辑视频 #{} 的 workflow_mode 与推送批次 #{} 不一致",
                    row.id, push.id
                )));
            }
            versions.push(row);
        }
        versions.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(versions.pop().unwrap_or_else(|| push.clone()))
    }

    pub async fn create_assembly_push_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        clips: Vec<AssemblyPushClipInput>,
        end_video_id: Option<i64>,
    ) -> Result<PreparedAssemblyPushRecord, AppError> {
        let workflow_mode =
            validate_assembly_workflow_mode(workflow_mode).map_err(AppError::InvalidArgument)?;
        if clips.is_empty() {
            return Err(AppError::InvalidArgument(
                "当前视频生成流程还没有可推送的成片片段".to_string(),
            ));
        }
        let rows = self
            .generated_videos_for_chapter(project_id, chapter_id)
            .await?;
        let pushes =
            assembly_push_records(&rows, workflow_mode).map_err(AppError::InvalidArgument)?;
        if pushes
            .iter()
            .any(|push| push.status == ASSEMBLY_PUSH_STATUS_PENDING)
        {
            return Err(AppError::InvalidArgument(
                "当前最新推送尚未同意或拒绝，不能继续推送后续片段".to_string(),
            ));
        }
        let active_source_ids = active_assembly_push_source_ids(
            pushes
                .iter()
                .map(|push| (push.status.as_str(), push.source_video_ids.as_slice())),
        );
        let clip_ids = clips.iter().map(|clip| clip.video_id).collect::<Vec<_>>();
        super::ensure_active_pushes_match_clip_prefix(&clip_ids, &active_source_ids)
            .map_err(AppError::InvalidArgument)?;
        let pending = &clips[active_source_ids.len()..];
        let pending_ids = pending.iter().map(|clip| clip.video_id).collect::<Vec<_>>();
        let end_index = selected_assembly_push_end_index(&pending_ids, end_video_id)
            .map_err(AppError::InvalidArgument)?;
        let source_clips = &pending[..=end_index];
        let push_index = pushes
            .iter()
            .map(|push| push.push_index)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| {
                AppError::InvalidArgument("成片推送批次序号超过 i32 上限".to_string())
            })?;
        let first = source_clips.first();
        let (width, height) = normalized_assembly_dimensions(
            first.map(|clip| clip.width),
            first.map(|clip| clip.height),
        );
        let fps = normalized_assembly_fps(first.map(|clip| clip.fps));
        let duration_secs = source_clips
            .iter()
            .map(|clip| clip.duration_secs.max(0.0))
            .sum::<f32>();
        let source_video_ids = source_clips
            .iter()
            .map(|clip| clip.video_id)
            .collect::<Vec<_>>();
        let source_sort_orders = source_clips
            .iter()
            .map(|clip| clip.sort_order)
            .collect::<Vec<_>>();
        let video = self
            .create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
                project_id,
                chapter_id: Some(chapter_id),
                video_type: ASSEMBLY_PUSH_VIDEO_TYPE.to_string(),
                workflow_mode: Some(workflow_mode.to_string()),
                prompt: format!("成片后处理推送批次 {push_index}"),
                negative_prompt: None,
                model: "a4n-assembly-push".to_string(),
                width,
                height,
                duration_secs,
                fps,
                source_video_id: source_video_ids.first().copied(),
                source_image_id: None,
                provider_job_id: None,
                metadata_json: Some(serde_json::json!({
                    "assembly_push": {
                        "status": ASSEMBLY_PUSH_STATUS_PENDING,
                        "push_index": push_index,
                        "workflow_mode": workflow_mode,
                        "source_video_ids": source_video_ids,
                        "source_sort_orders": source_sort_orders,
                        "source_clip_count": source_clips.len(),
                        "pushed_at": Utc::now().to_rfc3339(),
                    }
                })),
            })
            .await?;
        Ok(PreparedAssemblyPushRecord {
            video,
            source_video_ids,
            push_index,
        })
    }

    pub async fn create_imported_chapter_video_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<PreparedImportedChapterVideoRecord, AppError> {
        let source = self
            .project_chapter_video_source(project_id, chapter_id, "导入视频", true)
            .await?;
        let prompt = self
            .chapter_video_prompt_for_import(project_id, chapter_id, prompt_id)
            .await?;
        if let Some(source_hash) = metadata_source_content_sha256(prompt.metadata_json.as_ref())
            && source_hash != source_text_hash(&source.source.content)
        {
            return Err(AppError::InvalidArgument(
                "当前提示词基于旧正文内容，请重新生成视频提示词".to_string(),
            ));
        }
        let video = self
            .create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
                project_id,
                chapter_id: Some(chapter_id),
                video_type: super::CHAPTER_VIDEO_TYPE.to_string(),
                workflow_mode: Some(prompt.workflow_mode.clone()),
                prompt: imported_video_prompt_text(&prompt),
                negative_prompt: prompt.negative_prompt.clone(),
                model: "external-import".to_string(),
                width: properties.width,
                height: properties.height,
                duration_secs: properties.duration_secs,
                fps: properties.fps,
                source_video_id: None,
                source_image_id: None,
                provider_job_id: None,
                metadata_json: Some(super::imported_chapter_video_metadata(
                    prompt.id,
                    prompt.shot_number,
                    &prompt.prompt_type,
                    source_file_name,
                    source_extension,
                    imported_at,
                    prompt.metadata_json.as_ref(),
                )),
            })
            .await?;
        Ok(PreparedImportedChapterVideoRecord { video, prompt })
    }

    pub async fn create_imported_reference_media_video_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
        segment_plan_prompt_id: i64,
        shot_number: i32,
    ) -> Result<generated_video::Model, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, "导入参考媒体视频", true)
            .await?;
        let plan = self
            .require_current_shared_segment_plan(project_id, chapter_id, segment_plan_prompt_id)
            .await?;
        let recipe = a4n_core::video_execution::video_segment_recipe_from_metadata(
            plan.metadata_json.as_ref(),
            shot_number,
        )
        .map_err(AppError::InvalidArgument)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("当前规划缺少片段 {shot_number} 的执行方案"))
        })?;
        if recipe.workflow_mode.as_str() != "reference_media" {
            return Err(AppError::InvalidArgument(format!(
                "片段 {shot_number} 当前未选择参考媒体方案"
            )));
        }
        let reference_media = recipe.reference_media.as_ref().ok_or_else(|| {
            AppError::InvalidArgument(format!("片段 {shot_number} 尚未保存参考媒体提示词和媒体槽"))
        })?;
        let prompt =
            super::reference_media_rules::reference_media_prompt_from_recipe(reference_media);
        if prompt.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "当前片段参考媒体提示词不能为空".to_string(),
            ));
        }
        let options =
            validate_reference_media_import_options(recipe.duration_secs, &recipe.aspect_ratio)
                .map_err(AppError::InvalidArgument)?;
        let plan_identity = plan
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity"))
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少执行身份".to_string()))?;
        let video_execution_identity = super::prompt_generation::video_segment_execution_identity(
            plan_identity,
            Some(plan.id),
            Some(&recipe),
        )?;
        self.create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: "reference_media_video".to_string(),
            workflow_mode: Some("reference_media".to_string()),
            prompt,
            negative_prompt: None,
            model: "external-reference-media-import".to_string(),
            width: properties.width,
            height: properties.height,
            duration_secs: properties.duration_secs,
            fps: properties.fps,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(serde_json::json!({
                "segment_plan_prompt_id": segment_plan_prompt_id,
                "shot_number": shot_number,
                "video_execution_identity": video_execution_identity,
                "provider_backend": "external-import",
                "reference_media": {
                    "functionMode": "omni_reference",
                    "aspect_ratio": options.aspect_ratio,
                },
                "import": {
                    "external": true,
                    "source_file_name": source_file_name,
                    "source_extension": source_extension,
                    "imported_at": imported_at,
                }
            })),
        })
        .await
    }

    pub async fn create_imported_canvas_video_record_validated(
        &self,
        project_id: i64,
        prompt: Option<&str>,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<generated_video::Model, AppError> {
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != "video" {
            return Err(AppError::InvalidArgument(
                "画布视频导入只支持视频项目".to_string(),
            ));
        }
        self.create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: None,
            video_type: "canvas_video".to_string(),
            workflow_mode: Some("canvas_video".to_string()),
            prompt: prompt
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("导入画布视频")
                .to_string(),
            negative_prompt: None,
            model: "external-import".to_string(),
            width: properties.width,
            height: properties.height,
            duration_secs: properties.duration_secs,
            fps: properties.fps,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(serde_json::json!({
                "canvas_import": {
                    "external": true,
                    "source_file_name": source_file_name,
                    "source_extension": source_extension,
                    "imported_at": imported_at,
                }
            })),
        })
        .await
    }

    pub async fn prepare_canvas_assembly_validated(
        &self,
        project_id: i64,
        video_ids: Vec<i64>,
        title: Option<&str>,
    ) -> Result<CanvasAssemblyPlan, AppError> {
        if video_ids.is_empty() {
            return Err(AppError::InvalidArgument(
                "AssembleNode 至少需要一个视频片段".to_string(),
            ));
        }
        let mut seen = HashSet::new();
        let mut sources = Vec::with_capacity(video_ids.len());
        for video_id in video_ids {
            if !seen.insert(video_id) {
                return Err(AppError::InvalidArgument(format!(
                    "AssembleNode 视频片段重复: {video_id}"
                )));
            }
            let video = self.required_video_record(video_id).await?;
            if video.project_id != project_id {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{video_id} 不属于项目 {project_id}"
                )));
            }
            if video.chapter_id.is_some() {
                return Err(AppError::InvalidArgument(format!(
                    "AssembleNode 只支持项目级画布视频: {video_id}"
                )));
            }
            if video.status != "completed" {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{video_id} 尚未完成，不能成片"
                )));
            }
            if video
                .file_path
                .as_deref()
                .map(str::trim)
                .is_none_or(|path| path.is_empty())
            {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{video_id} 缺少本地文件"
                )));
            }
            sources.push(video);
        }
        let first = &sources[0];
        Ok(CanvasAssemblyPlan {
            width: first.width.max(1),
            height: first.height.max(1),
            fps: first.fps.max(1),
            duration_secs: sources
                .iter()
                .map(|video| video.duration_secs.max(0.0))
                .sum(),
            sources,
            title: title
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("画布成片")
                .to_string(),
        })
    }

    pub async fn prepare_canvas_frame_validated(
        &self,
        project_id: i64,
        video_id: i64,
        frame_kind: &str,
        at_secs: Option<f32>,
    ) -> Result<CanvasFramePlan, AppError> {
        let video = self.required_video_record(video_id).await?;
        if video.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{video_id} 不属于项目 {project_id}"
            )));
        }
        if video.chapter_id.is_some() {
            return Err(AppError::InvalidArgument(
                "ExtractFrameNode 只支持项目级画布视频".to_string(),
            ));
        }
        if video
            .file_path
            .as_deref()
            .map(str::trim)
            .is_none_or(|path| path.is_empty())
        {
            return Err(AppError::InvalidArgument(
                "源视频还没有可抽帧的本地文件".to_string(),
            ));
        }
        let frame_kind =
            super::normalize_canvas_frame_kind(frame_kind).map_err(AppError::InvalidArgument)?;
        let timestamp = super::canvas_frame_timestamp(video.duration_secs, &frame_kind, at_secs)
            .map_err(AppError::InvalidArgument)?;
        Ok(CanvasFramePlan {
            video,
            frame_kind,
            timestamp,
        })
    }

    pub async fn create_canvas_frame_record_validated(
        &self,
        plan: CanvasFramePlan,
        file_path: String,
        file_size: i64,
    ) -> Result<generated_image::Model, AppError> {
        self.create_generated_image_record(a4n_db::video::CreateGeneratedImageRecordDraftInput {
            project_id: plan.video.project_id,
            image_type: "canvas_frame".to_string(),
            file_path,
            prompt: format!("视频 #{} 画布抽帧 {}", plan.video.id, plan.frame_kind),
            model: "ffmpeg".to_string(),
            width: plan.video.width.max(1),
            height: plan.video.height.max(1),
            file_size: Some(file_size),
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(serde_json::json!({
                "canvas_frame": {
                    "source_video_id": plan.video.id,
                    "frame_kind": plan.frame_kind,
                    "at_secs": plan.timestamp,
                    "source_workflow_mode": plan.video.workflow_mode,
                    "extracted_at": Utc::now().to_rfc3339(),
                },
                "source": "video_canvas",
            })),
            created_at: None,
        })
        .await
    }

    /// 将 Runtime 已提取的真实尾帧登记为视频素材。
    /// 文件提取仍属于 Host；记录字段和归属元数据只在 Video application 生成。
    pub async fn create_video_tail_frame_record_validated(
        &self,
        video: &generated_video::Model,
        chapter_id: i64,
        prompt_id: Option<i64>,
        file_path: String,
        file_size: i64,
    ) -> Result<generated_image::Model, AppError> {
        if video.chapter_id != Some(chapter_id) {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{} 不属于章节 {}，无法登记真实尾帧",
                video.id, chapter_id
            )));
        }
        if file_path.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "真实尾帧文件路径不能为空".to_string(),
            ));
        }
        if file_size <= 0 {
            return Err(AppError::InvalidArgument(
                "真实尾帧文件大小必须大于 0".to_string(),
            ));
        }
        let shot_number = video_shot_number(video).map_err(AppError::InvalidArgument)?;
        let source = video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source"))
            .cloned();
        self.create_generated_image_record(a4n_db::video::CreateGeneratedImageRecordDraftInput {
            project_id: video.project_id,
            image_type: "video_tail_frame".to_string(),
            file_path,
            prompt: format!("视频 #{} 第 {} 段真实尾帧采集", video.id, shot_number),
            model: "ffmpeg".to_string(),
            width: video.width.max(1),
            height: video.height.max(1),
            file_size: Some(file_size),
            chapter_id: Some(chapter_id),
            panel_id: None,
            metadata_json: Some(serde_json::json!({
                "generation_context": {
                    "source": "video_tail_frame_capture",
                    "surface": "novel_detail.video",
                    "frame_kind": "actual_tail_frame",
                    "project_id": video.project_id,
                    "chapter_id": chapter_id,
                    "video_id": video.id,
                    "prompt_id": prompt_id,
                    "shot_number": shot_number,
                    "source_content_sha256": video_source_content_hash(
                        video.metadata_json.as_ref()
                    ),
                },
                "source": source,
            })),
            created_at: None,
        })
        .await
    }

    pub async fn create_canvas_assembly_record_validated(
        &self,
        project_id: i64,
        plan: CanvasAssemblyPlan,
    ) -> Result<generated_video::Model, AppError> {
        let source_video_ids = plan
            .sources
            .iter()
            .map(|video| video.id)
            .collect::<Vec<_>>();
        self.create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: None,
            video_type: "canvas_assembly".to_string(),
            workflow_mode: Some("canvas_assembly".to_string()),
            prompt: plan.title,
            negative_prompt: None,
            model: "ffmpeg".to_string(),
            width: plan.width,
            height: plan.height,
            duration_secs: plan.duration_secs,
            fps: plan.fps,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(serde_json::json!({
                "canvas_assembly": {
                    "source_video_ids": source_video_ids,
                    "clip_count": plan.sources.len(),
                    "assembled_at": Utc::now().to_rfc3339(),
                },
                "source": "video_canvas",
            })),
        })
        .await
    }

    pub async fn prepare_video_trim_validated(
        &self,
        video_id: i64,
        start_secs: f32,
        end_secs: f32,
    ) -> Result<VideoTrimPlan, AppError> {
        validate_video_trim_range(start_secs, end_secs).map_err(AppError::InvalidArgument)?;
        let source = self.required_video_record(video_id).await?;
        let source_workflow_mode = source.workflow_mode.clone().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "源视频 #{} 缺少 workflow_mode，无法按生成模式保存裁剪结果",
                source.id
            ))
        })?;
        if source.duration_secs > 0.0 && end_secs > source.duration_secs + 0.5 {
            return Err(AppError::InvalidArgument(
                "裁剪结束时间超过视频时长".to_string(),
            ));
        }
        let target_workflow_mode =
            if source.chapter_id.is_none() || source_workflow_mode.starts_with("canvas_") {
                "canvas_trim".to_string()
            } else {
                source_workflow_mode
            };
        Ok(VideoTrimPlan {
            source,
            start_secs,
            end_secs,
            target_workflow_mode,
        })
    }

    pub async fn create_trimmed_video_record_validated(
        &self,
        plan: VideoTrimPlan,
    ) -> Result<generated_video::Model, AppError> {
        self.create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
            project_id: plan.source.project_id,
            chapter_id: plan.source.chapter_id,
            video_type: "trimmed_clip".to_string(),
            workflow_mode: Some(plan.target_workflow_mode),
            prompt: plan.source.prompt.clone(),
            negative_prompt: plan.source.negative_prompt.clone(),
            model: plan.source.model.clone(),
            width: plan.source.width,
            height: plan.source.height,
            duration_secs: plan.end_secs - plan.start_secs,
            fps: plan.source.fps,
            source_video_id: Some(plan.source.id),
            source_image_id: plan.source.source_image_id,
            provider_job_id: None,
            metadata_json: Some(super::trimmed_video_metadata(
                plan.source.metadata_json.as_ref(),
                plan.start_secs,
                plan.end_secs,
            )),
        })
        .await
    }

    async fn required_video_record(
        &self,
        video_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        self.generated_video_by_id(video_id)
            .await?
            .ok_or_else(|| AppError::NotFound("视频不存在".to_string()))
    }

    async fn video_record_is_pushed_source(
        &self,
        video: &generated_video::Model,
    ) -> Result<bool, AppError> {
        if !is_assembly_clip_video_type(&video.video_type) {
            return Ok(false);
        }
        let (Some(chapter_id), Some(workflow_mode)) = (
            video.chapter_id,
            video.workflow_mode.as_deref().map(str::trim),
        ) else {
            return Ok(false);
        };
        Ok(self
            .active_pushed_source_ids(video.project_id, chapter_id, workflow_mode)
            .await?
            .contains(&video.id))
    }

    async fn active_pushed_source_ids(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
    ) -> Result<HashSet<i64>, AppError> {
        let rows = self
            .generated_videos_for_chapter(project_id, chapter_id)
            .await?;
        let pushes =
            assembly_push_records(&rows, workflow_mode).map_err(AppError::InvalidArgument)?;
        Ok(active_assembly_push_source_ids(
            pushes
                .iter()
                .map(|push| (push.status.as_str(), push.source_video_ids.as_slice())),
        )
        .into_iter()
        .collect())
    }

    async fn resolve_assembly_insert_position_metadata(
        &self,
        video: &generated_video::Model,
        base_metadata: Option<Value>,
        insert_after_video_id: Option<i64>,
    ) -> Result<Value, AppError> {
        if video.video_type != ASSEMBLY_INSERT_VIDEO_TYPE {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{} 不是成片插入片段，不能调整插入位置",
                video.id
            )));
        }
        if insert_after_video_id == Some(video.id) {
            return Err(AppError::InvalidArgument(
                "插入片段不能插入到自己之后".to_string(),
            ));
        }
        let chapter_id = video.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument(format!("插入片段 #{} 缺少章节绑定", video.id))
        })?;
        let workflow_mode = required_workflow_mode(video, "插入片段")?;
        let rows = self
            .generated_videos_for_chapter(video.project_id, chapter_id)
            .await?;
        let adopted_video_ids = if workflow_mode == "mixed" {
            Some(
                self.current_mixed_adopted_video_ids(video.project_id, chapter_id, &rows)
                    .await?,
            )
        } else {
            None
        };
        assembly_insert_position_metadata_from_rows(
            &rows,
            video.project_id,
            chapter_id,
            workflow_mode,
            Some(video.id),
            adopted_video_ids.as_ref(),
            base_metadata,
            insert_after_video_id,
        )
    }
}

fn assembly_insert_position_metadata_from_rows(
    rows: &[generated_video::Model],
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    excluded_video_id: Option<i64>,
    adopted_video_ids: Option<&HashSet<i64>>,
    base_metadata: Option<Value>,
    insert_after_video_id: Option<i64>,
) -> Result<Value, AppError> {
    let anchors = completed_assembly_clip_anchors(
        rows,
        project_id,
        chapter_id,
        workflow_mode,
        excluded_video_id,
        adopted_video_ids,
    )?;
    let clip_ids = anchors.iter().map(|clip| clip.video_id).collect::<Vec<_>>();
    let pushes = assembly_push_records(rows, workflow_mode).map_err(AppError::InvalidArgument)?;
    let active_source_ids = active_assembly_push_source_ids(
        pushes
            .iter()
            .map(|push| (push.status.as_str(), push.source_video_ids.as_slice())),
    );
    ensure_insert_position_not_inside_pushed_prefix(
        &clip_ids,
        &active_source_ids,
        insert_after_video_id,
    )
    .map_err(AppError::InvalidArgument)?;
    let (sort_order, insert_after_shot_number) =
        next_assembly_insert_sort_order(&anchors, insert_after_video_id)
            .map_err(AppError::InvalidArgument)?;
    assembly_insert_metadata_with_position(
        base_metadata,
        sort_order,
        insert_after_video_id,
        insert_after_shot_number,
    )
    .map_err(AppError::InvalidArgument)
}

fn required_workflow_mode<'a>(
    video: &'a generated_video::Model,
    label: &str,
) -> Result<&'a str, AppError> {
    video
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{label} #{} 缺少 workflow_mode", video.id))
        })
}

fn completed_assembly_clip_anchors(
    rows: &[generated_video::Model],
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    excluded_video_id: Option<i64>,
    adopted_video_ids: Option<&HashSet<i64>>,
) -> Result<Vec<AssemblyClipOrderAnchor>, AppError> {
    Ok(assembly_clip_records_from_rows(
        rows,
        project_id,
        chapter_id,
        workflow_mode,
        excluded_video_id,
        adopted_video_ids,
    )?
    .into_iter()
    .map(|clip| AssemblyClipOrderAnchor {
        video_id: clip.video_id,
        sort_order: clip.sort_order,
        shot_number: clip.shot_number,
    })
    .collect())
}

fn assembly_clip_records_from_rows(
    rows: &[generated_video::Model],
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    excluded_video_id: Option<i64>,
    adopted_video_ids: Option<&HashSet<i64>>,
) -> Result<Vec<AssemblyClipRecord>, AppError> {
    let mut clips = rows
        .iter()
        .filter(|video| {
            Some(video.id) != excluded_video_id
                && video.project_id == project_id
                && video.chapter_id == Some(chapter_id)
                && if workflow_mode == "mixed" {
                    adopted_video_ids.is_some_and(|ids| ids.contains(&video.id))
                        || (video.video_type == ASSEMBLY_INSERT_VIDEO_TYPE
                            && video.workflow_mode.as_deref() == Some("mixed"))
                } else {
                    video.workflow_mode.as_deref() == Some(workflow_mode)
                }
                && is_assembly_clip_video_type(&video.video_type)
                && video.status == "completed"
                && video
                    .file_path
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|path| !path.is_empty())
        })
        .map(|video| {
            let file_path = video
                .file_path
                .as_deref()
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!("成片片段视频 #{} 缺少本地文件", video.id))
                })?;
            Ok(AssemblyClipRecord {
                video_id: video.id,
                video_type: video.video_type.clone(),
                shot_number: assembly_clip_display_shot_number(
                    video.id,
                    &video.video_type,
                    video.metadata_json.as_ref(),
                )
                .map_err(AppError::InvalidArgument)?,
                sort_order: assembly_clip_sort_order(
                    video.id,
                    &video.video_type,
                    video.metadata_json.as_ref(),
                )
                .map_err(AppError::InvalidArgument)?,
                insert_after_video_id: assembly_clip_insert_after_video_id(
                    video.id,
                    &video.video_type,
                    video.metadata_json.as_ref(),
                )
                .map_err(AppError::InvalidArgument)?,
                file_path: file_path.to_string(),
                duration_secs: video.duration_secs,
                width: video.width,
                height: video.height,
                fps: video.fps,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    clips.sort_by(|left, right| {
        left.sort_order
            .total_cmp(&right.sort_order)
            .then_with(|| left.video_id.cmp(&right.video_id))
    });
    Ok(clips)
}

fn next_assembly_push_status(action: &str, status: &str) -> Result<&'static str, AppError> {
    match (action, status) {
        ("accept", ASSEMBLY_PUSH_STATUS_PENDING) => Ok(ASSEMBLY_PUSH_STATUS_ACCEPTED),
        ("reject", ASSEMBLY_PUSH_STATUS_PENDING) => Ok(ASSEMBLY_PUSH_STATUS_REJECTED),
        ("undo", ASSEMBLY_PUSH_STATUS_ACCEPTED) => Ok(ASSEMBLY_PUSH_STATUS_PENDING),
        ("accept", _) => Err(AppError::InvalidArgument(
            "只有待处理推送可以同意".to_string(),
        )),
        ("reject", _) => Err(AppError::InvalidArgument(
            "只有待处理推送可以拒绝".to_string(),
        )),
        ("undo", _) => Err(AppError::InvalidArgument(
            "只有已同意的最新推送可以撤销".to_string(),
        )),
        _ => Err(AppError::InvalidArgument(format!(
            "未知推送管理操作：{action}"
        ))),
    }
}

fn imported_video_prompt_text(prompt: &chapter_video_prompt::Model) -> String {
    let mut sections = vec![prompt.visual_prompt.trim().to_string()];
    if let Some(motion) = prompt
        .motion_prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        sections.push(format!("Motion Prompt:\n{motion}"));
    }
    sections.join("\n\n")
}

#[cfg(test)]
mod tests;
