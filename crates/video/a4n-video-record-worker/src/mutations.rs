use super::*;
use a4n_worker_contract::WorkerEnvelope;

impl WorkerState {
    pub(super) async fn select_segment(
        &self,
        request: WorkerEnvelope<VideoSegmentSelectionInput>,
    ) -> Result<GeneratedVideoSummary, WorkerFailure> {
        validate_request(&request, VIDEO_RECORD_SELECT_SEGMENT_OPERATION)?;
        validate_required_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let project_id = positive(input.project_id, "project_id")?;
        let chapter_id = positive(input.chapter_id, "chapter_id")?;
        let video_id = positive(input.video_id, "video_id")?;
        verify_project_permission(
            &self.database,
            project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let source_content =
            validate_video_source(&self.database, project_id, chapter_id, "采用片段视频", true)
                .await?;
        let source_content_sha256 = a4n_domain::source_text_hash(&source_content);
        let db = self.database.connection();
        let (plan_result, video_result) = tokio::join!(
            a4n_db::video::read_models::chapter_video_prompt_by_id(
                db,
                input.input.segment_plan_prompt_id
            ),
            a4n_db::video::read_models::generated_video_by_id(db, video_id),
        );
        let plan = plan_result
            .map_err(video_error)?
            .ok_or_else(|| WorkerFailure::not_found("视频提示词不存在"))?;
        let current_plan_id = a4n_db::video::completed_shared_segment_plan_id_for_source_hash(
            db,
            project_id,
            chapter_id,
            &source_content_sha256,
        )
        .await
        .map_err(video_error)?;
        if current_plan_id != Some(plan.id) {
            return Err(WorkerFailure::invalid_request(
                "目标片段规划不是当前章节已完成的共享片段规划",
            ));
        }
        let plan = a4n_db::video::read_models::with_video_plan_source_sync(db, plan)
            .await
            .map_err(video_error)?;
        let target = &input.input.target_segment;
        let segment_number = a4n_video_contract::resolve_video_segment_number(
            plan.metadata_json.as_ref(),
            &target.source_shot_id,
            target.segment_index_in_shot,
        )
        .map_err(WorkerFailure::invalid_request)?;
        let mut recipe = a4n_video_contract::video_segment_recipe_from_metadata(
            plan.metadata_json.as_ref(),
            segment_number,
        )
        .map_err(WorkerFailure::invalid_request)?
        .ok_or_else(|| {
            WorkerFailure::invalid_request(format!("当前规划缺少片段 {segment_number} 的执行方案"))
        })?;
        let video = video_result
            .map_err(database_error)?
            .ok_or_else(|| WorkerFailure::not_found("视频不存在"))?;
        let plan_identity = plan
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity"))
            .cloned()
            .ok_or_else(|| WorkerFailure::invalid_request("当前片段规划缺少执行身份"))?;
        let expected_identity = a4n_video_contract::video_segment_execution_identity(
            plan_identity,
            plan.id,
            &recipe,
            (&target.source_shot_id, target.segment_index_in_shot),
        )
        .map_err(WorkerFailure::invalid_request)?;
        if video.project_id != project_id
            || video.chapter_id != Some(chapter_id)
            || video.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
            || video
                .file_path
                .as_deref()
                .map(str::trim)
                .is_none_or(str::is_empty)
            || video.workflow_mode.as_deref() != Some(recipe.workflow_mode.as_str())
            || !a4n_video_contract::video_execution_segment_lineage_matches(
                video.metadata_json.as_ref(),
                &expected_identity,
            )
        {
            return Err(WorkerFailure::invalid_request(format!(
                "视频 #{video_id} 不属于当前规划片段 {segment_number} 的所选执行方案"
            )));
        }
        recipe.adopted_video_id = Some(video_id);
        a4n_db::video::prompts::consistency::update_chapter_video_segment_recipe(
            db, plan.id, recipe,
        )
        .await
        .map_err(video_error)?;
        Ok(video_summary(&video))
    }

    pub(super) async fn update_record(
        &self,
        request: WorkerEnvelope<VideoUpdateInput>,
    ) -> Result<GeneratedVideoSummary, WorkerFailure> {
        validate_request(&request, VIDEO_RECORD_UPDATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let request_project_id = request.project_id;
        let input = request.payload;
        let video_id = positive(input.video_id, "video_id")?;
        if input.input.is_empty() {
            return Err(WorkerFailure::invalid_request("视频更新内容不能为空"));
        }
        let video =
            a4n_db::video::read_models::generated_video_by_id(self.database.connection(), video_id)
                .await
                .map_err(database_error)?
                .ok_or_else(|| WorkerFailure::not_found("视频不存在"))?;
        validate_optional_project_scope(request_project_id, video.project_id)?;
        verify_project_permission(
            &self.database,
            video.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let modifies_pushed_source =
            input.input.prompt.is_some() || input.input.negative_prompt.is_some();
        if modifies_pushed_source && self.video_record_is_pushed_source(&video).await? {
            return Err(WorkerFailure::invalid_request(
                "该视频片段已推送到成片后处理，不能修改",
            ));
        }
        let (negative_prompt, clear_negative_prompt) = match input.input.negative_prompt {
            Some(Some(value)) => (Some(value), false),
            Some(None) => (None, true),
            None => (None, false),
        };
        let updated = a4n_db::video::read_models::update_generated_video_record(
            self.database.connection(),
            video_id,
            a4n_domain::media::UpdateGeneratedVideoRecordInput {
                prompt: input.input.prompt,
                negative_prompt,
                clear_negative_prompt,
                metadata_json: None,
                clear_metadata_json: false,
                is_favorite: input.input.is_favorite,
                expected_updated_at: Some(input.input.expected_updated_at),
            },
        )
        .await
        .map_err(video_error)?;
        Ok(video_summary(&updated))
    }

    async fn video_record_is_pushed_source(
        &self,
        video: &a4n_domain::media::GeneratedVideoRecord,
    ) -> Result<bool, WorkerFailure> {
        if !matches!(
            video.video_type.as_str(),
            a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER
                | a4n_domain::video_workflow::VIDEO_TYPE_REFERENCE_MEDIA
                | a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_INSERT
        ) {
            return Ok(false);
        }
        let (Some(chapter_id), Some(workflow_mode)) = (
            video.chapter_id,
            video.workflow_mode.as_deref().map(str::trim),
        ) else {
            return Ok(false);
        };
        let rows = a4n_db::video::read_models::generated_videos_for_chapter(
            self.database.connection(),
            video.project_id,
            chapter_id,
        )
        .await
        .map_err(database_error)?;
        for row in rows.into_iter().filter(|row| {
            row.video_type == a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_PUSH
                && row.workflow_mode.as_deref() == Some(workflow_mode)
                && row.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
        }) {
            let Some(metadata) = row
                .metadata_json
                .as_ref()
                .and_then(|value| value.get("assembly_push"))
                .and_then(serde_json::Value::as_object)
            else {
                return Err(WorkerFailure::invalid_request(format!(
                    "推送批次 #{} 缺少 assembly_push metadata",
                    row.id
                )));
            };
            let status = metadata
                .get("status")
                .and_then(serde_json::Value::as_str)
                .filter(|status| {
                    matches!(
                        *status,
                        a4n_domain::video_workflow::ASSEMBLY_PUSH_STATUS_PENDING
                            | a4n_domain::video_workflow::ASSEMBLY_PUSH_STATUS_ACCEPTED
                            | a4n_domain::video_workflow::ASSEMBLY_PUSH_STATUS_REJECTED
                    )
                })
                .ok_or_else(|| {
                    WorkerFailure::invalid_request(format!(
                        "推送批次 #{} 的 assembly_push.status 无效",
                        row.id
                    ))
                })?;
            if status == a4n_domain::video_workflow::ASSEMBLY_PUSH_STATUS_REJECTED {
                continue;
            }
            let source_ids = metadata
                .get("source_video_ids")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| {
                    WorkerFailure::invalid_request(format!(
                        "推送批次 #{} 缺少 source_video_ids",
                        row.id
                    ))
                })?;
            if source_ids.iter().any(|source_id| {
                source_id
                    .as_i64()
                    .is_some_and(|source_id| source_id == video.id)
            }) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
