use chrono::Utc;
use sea_orm::*;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

use a4n_core::error::AppError;
use a4n_core::models::ai::{generated_image, generated_video};
use a4n_core::models::canvas::{canvas_graph, canvas_node};
use a4n_core::models::chapter::{chapter, chapter_video_prompt};
use a4n_core::models::project::novel_project;

use super::{CreateVideoRecord, VideoService, validate_positive_i64};
use crate::asset_lineage_sync::AssetLineageSyncService;
use crate::generation::{
    GenerationRunTraceTarget, RecordStartedGenerationRunInput, delete_generation_run_targets_for,
    record_started_generation_run,
};

struct ValidatedCreateVideoRecord {
    project_id: i64,
    chapter_id: Option<i64>,
    video_type: String,
    workflow_mode: String,
    prompt: String,
    negative_prompt: Option<String>,
    model: String,
    width: i32,
    height: i32,
    duration_secs: f32,
    fps: i32,
    source_video_id: Option<i64>,
    source_image_id: Option<i64>,
    provider_job_id: Option<String>,
    metadata_json: Option<Value>,
}

fn validate_generation_run_for_video(
    input: &ValidatedCreateVideoRecord,
    generation_run: &RecordStartedGenerationRunInput,
) -> Result<(), AppError> {
    if generation_run.project_id != input.project_id {
        return Err(AppError::InvalidArgument(
            "视频生成运行记录 project_id 与视频资产不一致".to_string(),
        ));
    }
    if !generation_run.targets.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频生成运行记录目标由视频服务绑定，调用方不能预填 targets".to_string(),
        ));
    }
    Ok(())
}

fn validate_video_workflow_mode(workflow_mode: Option<String>) -> Result<String, AppError> {
    let Some(mode) = workflow_mode
        .map(|mode| mode.trim().to_string())
        .filter(|mode| !mode.is_empty())
    else {
        return Err(AppError::InvalidArgument(
            "视频记录缺少 workflow_mode".to_string(),
        ));
    };
    match mode.as_str() {
        "boundary_frames"
        | "first_frame_chain"
        | "reference_media"
        | "chat_video"
        | "storyboard"
        | "canvas_video"
        | "canvas_reference_media"
        | "canvas_trim"
        | "canvas_assembly" => Ok(mode),
        _ => Err(AppError::InvalidArgument(format!(
            "视频记录 workflow_mode 无效: {mode}"
        ))),
    }
}

impl VideoService {
    async fn ensure_source_video_belongs_to_project(
        &self,
        project_id: i64,
        source_video_id: i64,
    ) -> Result<(), AppError> {
        validate_positive_i64(source_video_id, "source_video_id")?;
        let source = self.get_video(source_video_id).await?;
        if source.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "源视频 #{source_video_id} 不属于项目 {project_id}"
            )));
        }
        Ok(())
    }

    async fn ensure_source_image_belongs_to_project(
        &self,
        project_id: i64,
        source_image_id: i64,
    ) -> Result<(), AppError> {
        validate_positive_i64(source_image_id, "source_image_id")?;
        let source = generated_image::Entity::find_by_id(source_image_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("源图片不存在: {source_image_id}")))?;
        if source.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "源图片 #{source_image_id} 不属于项目 {project_id}"
            )));
        }
        Ok(())
    }

    pub async fn list_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_video::Model>, AppError> {
        self.ensure_chapter_belongs_to_project(project_id, chapter_id)
            .await?;
        generated_video::Entity::find()
            .filter(generated_video::Column::ProjectId.eq(project_id))
            .filter(generated_video::Column::ChapterId.eq(chapter_id))
            .order_by_desc(generated_video::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn get_video(&self, video_id: i64) -> Result<generated_video::Model, AppError> {
        generated_video::Entity::find_by_id(video_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频不存在".into()))
    }

    pub async fn create_video_record(
        &self,
        input: CreateVideoRecord,
    ) -> Result<generated_video::Model, AppError> {
        let input = self.validate_create_video_record_input(input).await?;
        insert_validated_video_record(self.db.as_ref(), input).await
    }

    pub async fn create_video_record_with_generation_run(
        &self,
        input: CreateVideoRecord,
        mut generation_run: RecordStartedGenerationRunInput,
    ) -> Result<generated_video::Model, AppError> {
        let input = self.validate_create_video_record_input(input).await?;
        validate_generation_run_for_video(&input, &generation_run)?;
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let video = insert_validated_video_record(&txn, input).await?;
        generation_run.targets = vec![GenerationRunTraceTarget {
            target_kind: video.video_type.clone(),
            target_table: "generated_video".to_string(),
            target_id: video.id,
        }];
        record_started_generation_run(&txn, generation_run).await?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(video)
    }

    pub async fn create_prompt_video_record_with_generation_run(
        &self,
        input: CreateVideoRecord,
        mut generation_run: RecordStartedGenerationRunInput,
        prompt_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        let input = self.validate_create_video_record_input(input).await?;
        validate_generation_run_for_video(&input, &generation_run)?;
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let video = insert_validated_video_record(&txn, input).await?;
        let prompt = self.prompt_for_video(&txn, prompt_id, &video).await?;
        if prompt.generated_video_id.is_some() {
            return Err(AppError::InvalidArgument(
                "已存在由该视频提示词生成的视频，不能重新生成视频；请先删除对应视频".to_string(),
            ));
        }
        let mut active_prompt: chapter_video_prompt::ActiveModel = prompt.into();
        active_prompt.generated_video_id = Set(None);
        active_prompt.status = Set("running".to_string());
        active_prompt.updated_at = Set(Utc::now().naive_utc());
        active_prompt
            .update(&txn)
            .await
            .map_err(AppError::Database)?;
        generation_run.targets = vec![GenerationRunTraceTarget {
            target_kind: video.video_type.clone(),
            target_table: "generated_video".to_string(),
            target_id: video.id,
        }];
        record_started_generation_run(&txn, generation_run).await?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(video)
    }

    async fn validate_create_video_record_input(
        &self,
        input: CreateVideoRecord,
    ) -> Result<ValidatedCreateVideoRecord, AppError> {
        validate_positive_i64(input.project_id, "project_id")?;
        if let Some(chapter_id) = input.chapter_id {
            self.ensure_chapter_belongs_to_project(input.project_id, chapter_id)
                .await?;
        } else {
            self.ensure_project_exists(input.project_id).await?;
        }
        if let Some(source_video_id) = input.source_video_id {
            self.ensure_source_video_belongs_to_project(input.project_id, source_video_id)
                .await?;
        }
        if let Some(source_image_id) = input.source_image_id {
            self.ensure_source_image_belongs_to_project(input.project_id, source_image_id)
                .await?;
        }

        let workflow_mode = validate_video_workflow_mode(input.workflow_mode)?;
        Ok(ValidatedCreateVideoRecord {
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            video_type: input.video_type,
            workflow_mode,
            prompt: input.prompt,
            negative_prompt: input.negative_prompt,
            model: input.model,
            width: input.width,
            height: input.height,
            duration_secs: input.duration_secs,
            fps: input.fps,
            source_video_id: input.source_video_id,
            source_image_id: input.source_image_id,
            provider_job_id: normalize_provider_job_id(input.provider_job_id.as_deref())?,
            metadata_json: input.metadata_json,
        })
    }

    pub async fn mark_completed_and_link_prompt(
        &self,
        video_id: i64,
        file_path: &str,
        thumbnail_path: Option<&str>,
        file_size: i64,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let video = generated_video::Entity::find_by_id(video_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频不存在".into()))?;
        let prompt = match prompt_id {
            Some(prompt_id) => Some(self.prompt_for_video(&txn, prompt_id, &video).await?),
            None => None,
        };
        let existing_metadata = video.metadata_json.clone();
        let mut active: generated_video::ActiveModel = video.into();
        active.file_path = Set(Some(file_path.to_string()));
        if let Some(path) = thumbnail_path {
            active.thumbnail_path = Set(Some(path.to_string()));
        }
        active.file_size = Set(Some(file_size));
        active.status = Set("completed".to_string());
        active.error_message = Set(None);
        if metadata_json.is_some() {
            active.metadata_json = Set(merge_metadata(existing_metadata, metadata_json)?);
        }
        active.updated_at = Set(Utc::now().naive_utc());
        let updated = active.update(&txn).await.map_err(AppError::Database)?;
        if let Some(prompt) = prompt {
            let mut active: chapter_video_prompt::ActiveModel = prompt.into();
            active.generated_video_id = Set(Some(updated.id));
            active.status = Set("completed".to_string());
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(&txn).await.map_err(AppError::Database)?;
        }
        txn.commit().await.map_err(AppError::Database)?;
        AssetLineageSyncService::new(self.db.clone())
            .sync_project_asset_lineage(updated.project_id)
            .await?;
        Ok(updated)
    }

    pub async fn mark_job(
        &self,
        video_id: i64,
        provider_job_id: &str,
        metadata_json: Option<Value>,
    ) -> Result<generated_video::Model, AppError> {
        self.mark_job_and_update_prompt(video_id, provider_job_id, metadata_json, None)
            .await
    }

    pub async fn mark_job_and_update_prompt(
        &self,
        video_id: i64,
        provider_job_id: &str,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        let provider_job_id = normalize_provider_job_id(Some(provider_job_id))?
            .ok_or_else(|| AppError::InvalidArgument("provider_job_id 不能为空".to_string()))?;
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let video = generated_video::Entity::find_by_id(video_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频不存在".into()))?;
        let prompt = match prompt_id {
            Some(prompt_id) => Some(self.prompt_for_video(&txn, prompt_id, &video).await?),
            None => None,
        };
        let existing_metadata = video.metadata_json.clone();
        let mut active: generated_video::ActiveModel = video.into();
        active.status = Set("running".to_string());
        active.provider_job_id = Set(Some(provider_job_id));
        if metadata_json.is_some() {
            active.metadata_json = Set(merge_metadata(existing_metadata, metadata_json)?);
        }
        active.updated_at = Set(Utc::now().naive_utc());
        let updated = active.update(&txn).await.map_err(AppError::Database)?;
        if let Some(prompt) = prompt {
            let mut active: chapter_video_prompt::ActiveModel = prompt.into();
            active.status = Set("running".to_string());
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(&txn).await.map_err(AppError::Database)?;
        }
        txn.commit().await.map_err(AppError::Database)?;
        Ok(updated)
    }

    pub async fn update_status(
        &self,
        video_id: i64,
        status: &str,
    ) -> Result<generated_video::Model, AppError> {
        let video = self.get_video(video_id).await?;
        let mut active: generated_video::ActiveModel = video.into();
        active.status = Set(status.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn update_waiting_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
    ) -> Result<generated_video::Model, AppError> {
        if !matches!(status, "pending" | "running") {
            return Err(AppError::InvalidArgument(format!(
                "视频等待状态无效: {status}"
            )));
        }
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let video = generated_video::Entity::find_by_id(video_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频不存在".into()))?;
        let prompt = match prompt_id {
            Some(prompt_id) => Some(self.prompt_for_video(&txn, prompt_id, &video).await?),
            None => None,
        };
        let mut active: generated_video::ActiveModel = video.into();
        active.status = Set(status.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        let updated = active.update(&txn).await.map_err(AppError::Database)?;
        if let Some(prompt) = prompt {
            let mut active: chapter_video_prompt::ActiveModel = prompt.into();
            active.status = Set("running".to_string());
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(&txn).await.map_err(AppError::Database)?;
        }
        txn.commit().await.map_err(AppError::Database)?;
        Ok(updated)
    }

    pub async fn update_record(
        &self,
        video_id: i64,
        input: UpdateVideoRecord,
    ) -> Result<generated_video::Model, AppError> {
        validate_positive_i64(video_id, "video_id")?;
        let video = self.get_video(video_id).await?;
        if video.status == "running" {
            return Err(AppError::InvalidArgument(
                "视频生成中，不能编辑视频记录".to_string(),
            ));
        }
        validate_metadata_update_preserves_system_fields(
            video.metadata_json.as_ref(),
            input.metadata_json.as_ref(),
            input.clear_metadata_json,
        )?;
        let mut active: generated_video::ActiveModel = video.into();
        if let Some(prompt) = input.prompt {
            active.prompt = Set(required_trimmed(&prompt, "prompt")?);
        }
        if input.clear_negative_prompt {
            active.negative_prompt = Set(None);
        } else if input.negative_prompt.is_some() {
            active.negative_prompt = Set(normalize_optional_text(input.negative_prompt.as_deref()));
        }
        if let Some(is_favorite) = input.is_favorite {
            active.is_favorite = Set(is_favorite);
        }
        if input.clear_metadata_json {
            active.metadata_json = Set(None);
        } else if input.metadata_json.is_some() {
            active.metadata_json = Set(input.metadata_json);
        }
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    async fn prompt_for_video<C>(
        &self,
        db: &C,
        prompt_id: i64,
        video: &generated_video::Model,
    ) -> Result<chapter_video_prompt::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let prompt = self.get_prompt_with_valid_chapter(db, prompt_id).await?;
        if video.project_id != prompt.project_id {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{} 不属于提示词项目 {}",
                video.id, prompt.project_id
            )));
        }
        if video.chapter_id != Some(prompt.chapter_id) {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{} 不属于提示词章节 {}",
                video.id, prompt.chapter_id
            )));
        }
        Ok(prompt)
    }

    async fn get_prompt_with_valid_chapter<C>(
        &self,
        db: &C,
        prompt_id: i64,
    ) -> Result<chapter_video_prompt::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频提示词不存在".into()))?;
        let project = novel_project::Entity::find_by_id(prompt.project_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {}", prompt.project_id)))?;
        let chapter = chapter::Entity::find_by_id(prompt.chapter_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", prompt.chapter_id)))?;
        match project.project_kind.as_str() {
            "text" => {
                if chapter.project_id != project.id {
                    return Err(AppError::InvalidArgument(format!(
                        "章节 #{} 不属于文本项目 {}",
                        prompt.chapter_id, project.id
                    )));
                }
            }
            "video" => {
                if project.root_project_id != Some(chapter.project_id) {
                    return Err(AppError::InvalidArgument(format!(
                        "章节 #{} 不属于视频项目 {} 绑定的根文本项目",
                        prompt.chapter_id, project.id
                    )));
                }
            }
            other => {
                return Err(AppError::InvalidArgument(format!(
                    "视频链路不支持项目类型 {other}: project_id={}",
                    project.id
                )));
            }
        }
        Ok(prompt)
    }

    pub async fn update_prompt_status_for_video(
        &self,
        video_id: i64,
        prompt_id: i64,
        status: &str,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        let video = self.get_video(video_id).await?;
        let prompt = self
            .prompt_for_video(self.db.as_ref(), prompt_id, &video)
            .await?;
        let mut active: chapter_video_prompt::ActiveModel = prompt.into();
        active.status = Set(status.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn begin_prompt_video_generation(
        &self,
        prompt_id: i64,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        let prompt = self
            .ensure_prompt_video_generation_allowed(prompt_id)
            .await?;
        let mut active: chapter_video_prompt::ActiveModel = prompt.into();
        active.generated_video_id = Set(None);
        active.status = Set("running".to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn ensure_prompt_video_generation_allowed(
        &self,
        prompt_id: i64,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        let prompt = self
            .get_prompt_with_valid_chapter(self.db.as_ref(), prompt_id)
            .await?;
        if prompt.generated_video_id.is_some() {
            return Err(AppError::InvalidArgument(
                "已存在由该视频提示词生成的视频，不能重新生成视频；请先删除对应视频".to_string(),
            ));
        }
        Ok(prompt)
    }

    pub async fn mark_failed_and_update_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        let message = normalize_failure_message(message)?;
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let video = generated_video::Entity::find_by_id(video_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频不存在".into()))?;
        let prompt = match prompt_id {
            Some(prompt_id) => Some(self.prompt_for_video(&txn, prompt_id, &video).await?),
            None => None,
        };
        let mut active: generated_video::ActiveModel = video.into();
        active.status = Set("failed".to_string());
        active.error_message = Set(Some(message.clone()));
        active.updated_at = Set(Utc::now().naive_utc());
        let updated = active.update(&txn).await.map_err(AppError::Database)?;
        if let Some(prompt) = prompt {
            let mut active: chapter_video_prompt::ActiveModel = prompt.into();
            active.status = Set("failed".to_string());
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(&txn).await.map_err(AppError::Database)?;
        }
        txn.commit().await.map_err(AppError::Database)?;
        AssetLineageSyncService::new(self.db.clone())
            .sync_project_asset_lineage(updated.project_id)
            .await?;
        Ok(updated)
    }

    pub async fn mark_interrupted_and_update_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        let message = normalize_failure_message(message)?;
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let video = generated_video::Entity::find_by_id(video_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频不存在".into()))?;
        let prompt = match prompt_id {
            Some(prompt_id) => Some(self.prompt_for_video(&txn, prompt_id, &video).await?),
            None => None,
        };
        let mut active: generated_video::ActiveModel = video.into();
        active.status = Set("interrupted".to_string());
        active.error_message = Set(Some(message.clone()));
        active.updated_at = Set(Utc::now().naive_utc());
        let updated = active.update(&txn).await.map_err(AppError::Database)?;
        if let Some(prompt) = prompt {
            let mut active: chapter_video_prompt::ActiveModel = prompt.into();
            active.status = Set("interrupted".to_string());
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(&txn).await.map_err(AppError::Database)?;
        }
        txn.commit().await.map_err(AppError::Database)?;
        Ok(updated)
    }

    pub async fn toggle_favorite(&self, video_id: i64) -> Result<generated_video::Model, AppError> {
        let video = self.get_video(video_id).await?;
        let is_favorite = video.is_favorite;
        let mut active: generated_video::ActiveModel = video.into();
        active.is_favorite = Set(!is_favorite);
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn delete(&self, video_id: i64) -> Result<(), AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let video = generated_video::Entity::find_by_id(video_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("视频不存在".into()))?;

        let prompts = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::GeneratedVideoId.eq(video_id))
            .all(&txn)
            .await
            .map_err(AppError::Database)?;
        for prompt in prompts {
            let mut active: chapter_video_prompt::ActiveModel = prompt.into();
            active.generated_video_id = Set(None);
            active.status = Set("pending".to_string());
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(&txn).await.map_err(AppError::Database)?;
        }
        let child_videos = generated_video::Entity::find()
            .filter(generated_video::Column::SourceVideoId.eq(video_id))
            .all(&txn)
            .await
            .map_err(AppError::Database)?;
        for video in child_videos {
            let mut metadata = video.metadata_json.clone();
            let mut active: generated_video::ActiveModel = video.into();
            active.source_video_id = Set(None);
            if let Some(mut metadata) = metadata.take() {
                if let Some(obj) = metadata.as_object_mut() {
                    obj.remove("source_video_id");
                    if let Some(trim) = obj.get_mut("trim")
                        && let Some(trim_obj) = trim.as_object_mut()
                    {
                        trim_obj.remove("source_video_id");
                    }
                }
                active.metadata_json = Set(Some(metadata));
            }
            active.updated_at = Set(Utc::now().naive_utc());
            active.update(&txn).await.map_err(AppError::Database)?;
        }
        mark_canvas_generated_video_refs_missing(&txn, video.project_id, video_id).await?;
        delete_generation_run_targets_for(&txn, "generated_video", &[video_id]).await?;
        generated_video::Entity::delete_by_id(video_id)
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn missing_video_reference_project_ids(
        &self,
        video_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        if let Some(video) = generated_video::Entity::find_by_id(video_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
        {
            return Ok(vec![video.project_id]);
        }
        missing_video_reference_project_ids(self.db.as_ref(), video_id).await
    }

    pub async fn delete_missing_video_references(&self, video_id: i64) -> Result<u64, AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        if generated_video::Entity::find_by_id(video_id)
            .one(&txn)
            .await
            .map_err(AppError::Database)?
            .is_some()
        {
            return Err(AppError::InvalidArgument(
                "视频记录仍存在，请使用正常视频删除流程".to_string(),
            ));
        }

        let prompts = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::GeneratedVideoId.eq(video_id))
            .all(&txn)
            .await
            .map_err(AppError::Database)?;
        let mut project_ids = prompts
            .iter()
            .map(|prompt| prompt.project_id)
            .collect::<BTreeSet<_>>();
        project_ids.extend(missing_canvas_video_ref_project_ids(&txn, video_id).await?);
        if prompts.is_empty() && project_ids.is_empty() {
            return Err(AppError::NotFound("视频不存在".into()));
        }

        let now = Utc::now().naive_utc();
        for prompt in prompts.iter().cloned() {
            let mut active: chapter_video_prompt::ActiveModel = prompt.into();
            active.generated_video_id = Set(None);
            active.status = Set("pending".to_string());
            active.updated_at = Set(now);
            active.update(&txn).await.map_err(AppError::Database)?;
        }
        for project_id in project_ids {
            mark_canvas_generated_video_refs_missing(&txn, project_id, video_id).await?;
        }
        delete_generation_run_targets_for(&txn, "generated_video", &[video_id]).await?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(prompts.len() as u64)
    }
}

pub async fn mark_canvas_generated_video_refs_missing<C>(
    db: &C,
    project_id: i64,
    video_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let graph_ids = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.eq(project_id))
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|graph| graph.id)
        .collect::<Vec<_>>();
    if graph_ids.is_empty() {
        return Ok(());
    }
    let nodes = canvas_node::Entity::find()
        .filter(canvas_node::Column::GraphId.is_in(graph_ids))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let now = Utc::now().naive_utc();
    for node in nodes {
        if !canvas_node_references_generated_video(&node, video_id) {
            continue;
        }
        let primary_ref_matches = node.ref_type.as_deref() == Some("legacy_generated_video")
            && node.ref_id == Some(video_id);
        let missing_ref_id = node
            .ref_id
            .filter(|id| *id == video_id)
            .or_else(|| first_config_i64(&node.config_json, VIDEO_REF_CONFIG_KEYS))
            .or_else(|| first_config_array_i64(&node.config_json, "video_ids", video_id));
        let mut config = node.config_json.clone();
        if let Some(object) = config.as_object_mut() {
            object.insert(
                "missing_reason".to_string(),
                Value::String("referenced generated video was deleted".to_string()),
            );
            object.insert(
                "missing_ref_type".to_string(),
                Value::String("generated_video".to_string()),
            );
            if let Some(id) = missing_ref_id {
                object.insert("missing_ref_id".to_string(), Value::from(id));
            }
            remove_config_i64_refs(object, VIDEO_REF_CONFIG_KEYS, video_id);
            remove_config_i64_array_refs(object, "video_ids", video_id);
        }
        let mut active: canvas_node::ActiveModel = node.into();
        if primary_ref_matches {
            active.ref_type = Set(None);
            active.ref_id = Set(None);
        }
        active.state = Set("missing".to_string());
        active.config_json = Set(config);
        active.updated_at = Set(now);
        active.update(db).await.map_err(AppError::Database)?;
    }
    Ok(())
}

async fn missing_video_reference_project_ids<C>(db: &C, video_id: i64) -> Result<Vec<i64>, AppError>
where
    C: ConnectionTrait,
{
    let mut project_ids = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::ProjectId)
        .filter(chapter_video_prompt::Column::GeneratedVideoId.eq(video_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    project_ids.extend(missing_canvas_video_ref_project_ids(db, video_id).await?);
    Ok(project_ids.into_iter().collect())
}

async fn missing_canvas_video_ref_project_ids<C>(
    db: &C,
    video_id: i64,
) -> Result<BTreeSet<i64>, AppError>
where
    C: ConnectionTrait,
{
    let graphs = canvas_graph::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|graph| (graph.id, graph.project_id))
        .collect::<BTreeMap<_, _>>();
    if graphs.is_empty() {
        return Ok(BTreeSet::new());
    }

    let project_ids = canvas_node::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .filter(|node| canvas_node_references_generated_video(node, video_id))
        .filter_map(|node| graphs.get(&node.graph_id).copied())
        .collect::<BTreeSet<_>>();
    Ok(project_ids)
}

const VIDEO_REF_CONFIG_KEYS: &[&str] = &[
    "generated_video_id",
    "legacy_generated_video_id",
    "source_video_id",
    "video_id",
];

fn canvas_node_references_generated_video(node: &canvas_node::Model, video_id: i64) -> bool {
    if node.ref_type.as_deref() == Some("legacy_generated_video") && node.ref_id == Some(video_id) {
        return true;
    }
    config_contains_i64(&node.config_json, VIDEO_REF_CONFIG_KEYS, video_id)
        || config_array_contains_i64(&node.config_json, "video_ids", video_id)
}

fn config_contains_i64(config: &Value, keys: &[&str], target_id: i64) -> bool {
    keys.iter()
        .filter_map(|key| config.get(*key).and_then(Value::as_i64))
        .any(|id| id == target_id)
}

fn config_array_contains_i64(config: &Value, key: &str, target_id: i64) -> bool {
    config
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values
                .iter()
                .filter_map(Value::as_i64)
                .any(|id| id == target_id)
        })
}

fn first_config_i64(config: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| config.get(*key).and_then(Value::as_i64))
}

fn first_config_array_i64(config: &Value, key: &str, target_id: i64) -> Option<i64> {
    config
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .find(|id| *id == target_id)
}

fn remove_config_i64_refs(
    object: &mut serde_json::Map<String, Value>,
    keys: &[&str],
    target_id: i64,
) {
    for key in keys {
        if object
            .get(*key)
            .and_then(Value::as_i64)
            .is_some_and(|id| id == target_id)
        {
            object.remove(*key);
        }
    }
}

fn remove_config_i64_array_refs(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    target_id: i64,
) {
    let Some(values) = object.get(key).and_then(Value::as_array) else {
        return;
    };
    let retained = values
        .iter()
        .filter(|value| value.as_i64() != Some(target_id))
        .cloned()
        .collect::<Vec<_>>();
    if retained.is_empty() {
        object.remove(key);
    } else {
        object.insert(key.to_string(), Value::Array(retained));
    }
}

async fn insert_validated_video_record<C>(
    db: &C,
    input: ValidatedCreateVideoRecord,
) -> Result<generated_video::Model, AppError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    generated_video::ActiveModel {
        project_id: Set(input.project_id),
        chapter_id: Set(input.chapter_id),
        video_type: Set(input.video_type),
        workflow_mode: Set(Some(input.workflow_mode)),
        file_path: Set(None),
        thumbnail_path: Set(None),
        prompt: Set(input.prompt),
        negative_prompt: Set(input.negative_prompt),
        model: Set(input.model),
        width: Set(input.width),
        height: Set(input.height),
        duration_secs: Set(input.duration_secs),
        fps: Set(input.fps),
        file_size: Set(None),
        source_video_id: Set(input.source_video_id),
        source_image_id: Set(input.source_image_id),
        status: Set("running".to_string()),
        provider_job_id: Set(input.provider_job_id),
        error_message: Set(None),
        metadata_json: Set(input.metadata_json),
        is_favorite: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

#[derive(Debug, Clone, Default)]
pub struct UpdateVideoRecord {
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub clear_negative_prompt: bool,
    pub metadata_json: Option<Value>,
    pub clear_metadata_json: bool,
    pub is_favorite: Option<bool>,
}

const SYSTEM_VIDEO_METADATA_KEYS: &[&str] = &[
    "boundary_frame_image_count",
    "boundary_frame_image_ids",
    "boundary_frame_image_url_count",
    "boundary_frame_request",
    "canvas",
    "config_id",
    "first_frame_image_id",
    "material_reference_image_count",
    "material_reference_image_ids",
    "prompt_id",
    "provider_backend",
    "provider_reference_image_count",
    "provider_reference_image_ids",
    "provider_request_snapshot",
    "prompt_reference_image_count",
    "prompt_reference_image_ids",
    "provider_response",
    "reference_image_count",
    "reference_image_ids",
    "reference_media",
    "runtime_config_snapshot",
    "source",
    "source_prompt_type",
    "start_frame_image_id",
];

fn validate_metadata_update_preserves_system_fields(
    existing: Option<&Value>,
    incoming: Option<&Value>,
    clear_metadata_json: bool,
) -> Result<(), AppError> {
    let Some(existing) = existing.and_then(Value::as_object) else {
        return Ok(());
    };
    let protected = SYSTEM_VIDEO_METADATA_KEYS
        .iter()
        .filter_map(|key| existing.get(*key).map(|value| (*key, value)))
        .collect::<Vec<_>>();
    if protected.is_empty() {
        return Ok(());
    }

    if clear_metadata_json {
        return Err(AppError::InvalidArgument(format!(
            "视频 metadata 包含系统字段 {}，不能清空",
            protected_metadata_keys(&protected)
        )));
    }
    let Some(incoming) = incoming else {
        return Ok(());
    };
    let incoming = incoming.as_object().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "视频 metadata 更新必须是对象，并保留系统字段 {}",
            protected_metadata_keys(&protected)
        ))
    })?;
    for (key, existing_value) in protected {
        if incoming.get(key) != Some(existing_value) {
            return Err(AppError::InvalidArgument(format!(
                "视频 metadata 系统字段 {key} 不能修改或删除"
            )));
        }
    }
    Ok(())
}

fn protected_metadata_keys(protected: &[(&str, &Value)]) -> String {
    protected
        .iter()
        .map(|(key, _)| *key)
        .collect::<Vec<_>>()
        .join(", ")
}

fn required_trimmed(value: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn normalize_provider_job_id(value: Option<&str>) -> Result<Option<String>, AppError> {
    value
        .map(str::trim)
        .map(|value| {
            if value.is_empty() {
                Err(AppError::InvalidArgument(
                    "provider_job_id 不能为空".to_string(),
                ))
            } else {
                Ok(value.to_string())
            }
        })
        .transpose()
}

fn normalize_failure_message(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(
            "视频失败原因不能为空".to_string(),
        ))
    } else {
        Ok(value.to_string())
    }
}

fn merge_metadata(
    existing: Option<Value>,
    incoming: Option<Value>,
) -> Result<Option<Value>, AppError> {
    let incoming = incoming.filter(|value| !value.is_null());
    match (existing, incoming) {
        (Some(Value::Object(mut base)), Some(provider)) => {
            base.insert("provider_response".into(), provider);
            Ok(Some(Value::Object(base)))
        }
        (None, Some(provider)) => Ok(Some(json!({ "provider_response": provider }))),
        (existing, None) => Ok(existing),
        (Some(_), Some(_)) => Err(AppError::InvalidArgument(
            "视频 metadata 必须是对象，不能写入 provider_response".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn metadata_update_rejects_clearing_system_fields() {
        let err = validate_metadata_update_preserves_system_fields(
            Some(&json!({
                "config_id": 7,
                "provider_request_snapshot": { "request": "body" },
            })),
            None,
            true,
        )
        .expect_err("system metadata must not be cleared");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("不能清空"));
        assert!(err.to_string().contains("config_id"));
        assert!(err.to_string().contains("provider_request_snapshot"));
    }

    #[test]
    fn metadata_update_rejects_removing_or_changing_system_fields() {
        let existing = json!({
            "config_id": 7,
            "provider_request_snapshot": { "request": "body" },
            "assembly_insert": { "sort_order": 1000.0 },
        });

        let missing = validate_metadata_update_preserves_system_fields(
            Some(&existing),
            Some(&json!({
                "provider_request_snapshot": { "request": "body" },
                "assembly_insert": { "sort_order": 2000.0 },
            })),
            false,
        )
        .expect_err("missing config_id must be rejected");
        assert!(missing.to_string().contains("config_id"));

        let changed = validate_metadata_update_preserves_system_fields(
            Some(&existing),
            Some(&json!({
                "config_id": 8,
                "provider_request_snapshot": { "request": "body" },
                "assembly_insert": { "sort_order": 2000.0 },
            })),
            false,
        )
        .expect_err("changed config_id must be rejected");
        assert!(changed.to_string().contains("config_id"));
    }

    #[test]
    fn metadata_update_allows_business_fields_when_system_fields_are_preserved() {
        validate_metadata_update_preserves_system_fields(
            Some(&json!({
                "config_id": 7,
                "provider_request_snapshot": { "request": "body" },
                "shot_number": 1,
                "assembly_insert": { "sort_order": 1000.0 },
            })),
            Some(&json!({
                "config_id": 7,
                "provider_request_snapshot": { "request": "body" },
                "shot_number": 2,
                "assembly_insert": { "sort_order": 2000.0 },
            })),
            false,
        )
        .expect("business metadata fields should remain editable");
    }

    #[test]
    fn provider_job_id_normalization_trims_and_rejects_blank_values() {
        assert_eq!(
            normalize_provider_job_id(Some(" provider-job ")).expect("nonblank job id"),
            Some("provider-job".to_string())
        );
        assert_eq!(
            normalize_provider_job_id(None).expect("missing job id is allowed"),
            None
        );

        let err = normalize_provider_job_id(Some(" \n\t "))
            .expect_err("blank job id must not enter running video records");
        assert!(matches!(
            err,
            AppError::InvalidArgument(message) if message == "provider_job_id 不能为空"
        ));
    }

    #[test]
    fn failure_message_normalization_preserves_full_nonblank_text() {
        let long = format!("  {}  ", "provider failed: ".to_string() + &"x".repeat(900));
        let normalized = normalize_failure_message(&long).expect("nonblank message");

        assert!(normalized.starts_with("provider failed: "));
        assert_eq!(normalized.len(), "provider failed: ".len() + 900);

        let err = normalize_failure_message(" \n\t ")
            .expect_err("failed records need an explicit reason");
        assert!(matches!(
            err,
            AppError::InvalidArgument(message) if message == "视频失败原因不能为空"
        ));
    }

    #[test]
    fn provider_response_metadata_merges_only_into_objects() {
        let merged = merge_metadata(
            Some(json!({
                "config_id": 7,
                "provider_request_snapshot": {"schema_version": 1}
            })),
            Some(json!({"job_id": "provider-job"})),
        )
        .expect("object metadata can accept provider response")
        .expect("merged metadata should exist");

        assert_eq!(merged["config_id"], json!(7));
        assert_eq!(
            merged["provider_response"],
            json!({"job_id": "provider-job"})
        );

        let created = merge_metadata(None, Some(json!({"job_id": "provider-job"})))
            .expect("missing metadata can be initialized")
            .expect("created metadata should exist");
        assert_eq!(
            created,
            json!({"provider_response": {"job_id": "provider-job"}})
        );

        let unchanged = merge_metadata(Some(json!({"config_id": 7})), Some(Value::Null))
            .expect("null provider metadata should be treated as missing")
            .expect("existing metadata should be preserved");
        assert_eq!(unchanged, json!({"config_id": 7}));

        let empty = merge_metadata(None, Some(Value::Null))
            .expect("null provider metadata should not create fake provider_response");
        assert_eq!(empty, None);

        let err = merge_metadata(
            Some(json!("corrupted metadata")),
            Some(json!({"job_id": "provider-job"})),
        )
        .expect_err("non-object metadata must not be wrapped into previous");
        assert!(matches!(
            err,
            AppError::InvalidArgument(message)
                if message == "视频 metadata 必须是对象，不能写入 provider_response"
        ));
    }
}
