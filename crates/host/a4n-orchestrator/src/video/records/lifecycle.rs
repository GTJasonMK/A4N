//! 视频记录生命周期。

use super::*;

impl crate::OrchestratorContext {
    pub(crate) async fn create_video_generation_record_validated(
        &self,
        input: VideoGenerationRecordInput,
    ) -> Result<GeneratedVideoRecord, a4n_error::AppError> {
        let video_record = a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
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
    ) -> Result<Vec<GeneratedVideoRecord>, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, "查看章节视频", false)
            .await?;
        self.generated_videos_for_chapter(project_id, chapter_id)
            .await
    }

    pub async fn select_segment_video_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        segment_plan_prompt_id: i64,
        target: (&str, i32),
        video_id: i64,
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, "采用片段视频", true)
            .await?;
        let (plan_result, selected_result) = futures::join!(
            self.require_current_shared_segment_plan(
                project_id,
                chapter_id,
                segment_plan_prompt_id
            ),
            self.required_video_record(video_id),
        );
        let plan = plan_result?;
        let shot_number = a4n_video_contract::resolve_video_segment_number(
            plan.metadata_json.as_ref(),
            target.0,
            target.1,
        )
        .map_err(AppError::InvalidArgument)?;
        let mut recipe = a4n_video_contract::video_segment_recipe_from_metadata(
            plan.metadata_json.as_ref(),
            shot_number,
        )
        .map_err(AppError::InvalidArgument)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("当前规划缺少片段 {shot_number} 的执行方案"))
        })?;
        let selected = selected_result?;
        let plan_identity = plan
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity"))
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少执行身份".to_string()))?;
        let expected_identity =
            crate::video::prompts::generation::video_segment_execution_identity(
                plan_identity.clone(),
                Some(plan.id),
                Some(&recipe),
                target,
            )?;
        if selected.project_id != project_id
            || selected.chapter_id != Some(chapter_id)
            || selected.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
            || selected
                .file_path
                .as_deref()
                .map(str::trim)
                .is_none_or(str::is_empty)
            || selected.workflow_mode.as_deref() != Some(recipe.workflow_mode.as_str())
            || !crate::video::prompts::records::video_execution_segment_lineage_matches(
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
        Ok(selected)
    }

    pub async fn update_video_record_validated(
        &self,
        video_id: i64,
        input: UpdateGeneratedVideoInput,
    ) -> Result<GeneratedVideoRecord, AppError> {
        if input.is_empty() {
            return Err(AppError::InvalidArgument(
                "视频更新内容不能为空".to_string(),
            ));
        }
        let existing = self.required_video_record(video_id).await?;
        let modifies_pushed_source = input.prompt.is_some() || input.negative_prompt.is_some();
        if modifies_pushed_source && self.video_record_is_pushed_source(&existing).await? {
            return Err(AppError::InvalidArgument(
                "该视频片段已推送到成片后处理，不能修改".to_string(),
            ));
        }

        let (negative_prompt, clear_negative_prompt) = match input.negative_prompt {
            Some(Some(value)) => (Some(value), false),
            Some(None) => (None, true),
            None => (None, false),
        };

        self.update_generated_video_record(
            video_id,
            a4n_domain::media::UpdateGeneratedVideoRecordInput {
                prompt: input.prompt,
                negative_prompt,
                clear_negative_prompt,
                metadata_json: None,
                clear_metadata_json: false,
                is_favorite: input.is_favorite,
            },
        )
        .await
    }

    pub async fn delete_video_record_validated(&self, video_id: i64) -> Result<(), AppError> {
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
        self.delete_video_tail_frame_assets(video_id).await?;
        self.delete_generated_video_record_with_cleanup(video_id)
            .await
    }

    pub async fn update_assembly_push_status_validated(
        &self,
        video_id: i64,
        input: UpdateAssemblyPushStatusInput,
    ) -> Result<GeneratedVideoRecord, AppError> {
        let existing = self.required_video_record(video_id).await?;
        if existing.video_type != a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_PUSH {
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
            next_assembly_push_status(input.action, pushes[latest_index].status.as_str())?;
        self.update_generated_video_record(
            existing.id,
            a4n_domain::media::UpdateGeneratedVideoRecordInput {
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
}
