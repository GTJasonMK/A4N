//! assembly 记录与后处理准备。

use super::*;

impl crate::OrchestratorContext {
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
        let (adopted_video_ids, timeline_order) = self
            .assembly_selection_for_workflow(project_id, chapter_id, workflow_mode, &rows)
            .await?;
        Ok(AssemblyVideoRecords {
            workflow_mode: workflow_mode.to_string(),
            clips: assembly_clip_records_from_rows(
                &rows,
                project_id,
                chapter_id,
                workflow_mode,
                None,
                adopted_video_ids.as_ref(),
                timeline_order.as_ref(),
            )?,
        })
    }

    pub(in crate::video) async fn current_mixed_adopted_video_ids(
        &self,
        project_id: i64,
        chapter_id: i64,
        videos: &[generated_video::Model],
        snapshot: &a4n_db::video::VideoTimelineSnapshot,
    ) -> Result<HashSet<i64>, AppError> {
        let (plan_result, prompts_result) = futures::join!(
            self.current_shared_segment_plan(project_id, chapter_id),
            self.chapter_video_prompt_video_binding_records_for_chapter(project_id, chapter_id),
        );
        let plan = plan_result?;
        let segments = serde_json::from_value::<Vec<a4n_video_contract::VideoSegmentPlanV2>>(
            plan.metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("segments"))
                .cloned()
                .ok_or_else(|| {
                    AppError::InvalidArgument("当前片段规划缺少 segments".to_string())
                })?,
        )
        .map_err(|error| AppError::InvalidArgument(format!("当前片段规划无效：{error}")))?;
        let segments_by_number = segments.iter().fold(
            HashMap::with_capacity(segments.len()),
            |mut by_number, segment| {
                by_number.entry(segment.segment_number).or_insert(segment);
                by_number
            },
        );
        let videos_by_id =
            videos
                .iter()
                .fold(HashMap::with_capacity(videos.len()), |mut by_id, video| {
                    by_id.entry(video.id).or_insert(video);
                    by_id
                });
        let plan_style = plan
            .style
            .as_deref()
            .map(str::trim)
            .filter(|style| !style.is_empty())
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少 style".to_string()))?;
        let recipes = a4n_video_contract::video_segment_recipes_for_plan_from_metadata(
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
        for recipe in &recipes {
            let Some(video_id) = recipe.adopted_video_id else {
                continue;
            };
            let segment_number = recipe.segment_number;
            let segment = segments_by_number
                .get(&segment_number)
                .copied()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "片段 {segment_number} 的执行方案不属于当前规划"
                    ))
                })?;
            let expected_identity =
                crate::video::prompt_generation::video_segment_execution_identity(
                    plan_identity.clone(),
                    Some(plan.id),
                    Some(recipe),
                    (&segment.source_shot_id, segment.segment_index_in_shot),
                )?;
            let Some(video) = videos_by_id.get(&video_id).copied() else {
                continue;
            };
            if video.project_id == project_id
                && video.chapter_id == Some(chapter_id)
                && video.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
                && video
                    .file_path
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|path| !path.is_empty())
                && video.workflow_mode.as_deref() == Some(recipe.workflow_mode.as_str())
                && crate::video::prompt_records::video_execution_segment_lineage_matches(
                    video.metadata_json.as_ref(),
                    &expected_identity,
                )
                && !adopted.insert(video_id)
            {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{video_id} 被多个片段重复采用"
                )));
            }
        }
        let prompts = prompts_result?;
        let prompt_count = prompts.len();
        let prompts_by_id = prompts.into_iter().fold(
            HashMap::with_capacity(prompt_count),
            |mut by_id, (prompt_id, video_id)| {
                by_id.entry(prompt_id).or_insert(video_id);
                by_id
            },
        );
        for item in snapshot.items.iter().filter(|item| {
            item.item_kind == a4n_db::video::TIMELINE_INSERTED_KIND
                && item.status == a4n_db::video::TIMELINE_ACTIVE_STATUS
        }) {
            let current_video_id = item
                .insert_plan_prompt_id
                .and_then(|prompt_id| prompts_by_id.get(&prompt_id).copied().flatten());
            let video_candidates = current_video_id
                .and_then(|video_id| videos_by_id.get(&video_id).copied())
                .map_or(&[][..], std::slice::from_ref);
            let Some(video_id) = crate::video::timeline::current_timeline_item_video_id(
                item,
                current_video_id,
                video_candidates,
            )?
            else {
                continue;
            };
            if !adopted.insert(video_id) {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{video_id} 被多个时间线片段重复采用"
                )));
            }
        }
        Ok(adopted)
    }

    pub(in crate::video) async fn assembly_selection_for_workflow(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        videos: &[generated_video::Model],
    ) -> Result<
        (
            Option<HashSet<i64>>,
            Option<crate::video::timeline::VideoTimelineAssemblyOrder>,
        ),
        AppError,
    > {
        if workflow_mode != WORKFLOW_MODE_MIXED {
            return Ok((None, None));
        }
        let snapshot = self
            .current_video_timeline_snapshot(project_id, chapter_id)
            .await?;
        let order = crate::video::timeline::video_timeline_assembly_order_from_snapshot(&snapshot)?;
        let adopted = self
            .current_mixed_adopted_video_ids(project_id, chapter_id, videos, &snapshot)
            .await?;
        Ok((Some(adopted), Some(order)))
    }

    pub(in crate::video) async fn prepare_assembly_insert_metadata_validated(
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
        let (adopted_video_ids, timeline_order) = self
            .assembly_selection_for_workflow(project_id, chapter_id, workflow_mode, &rows)
            .await?;
        assembly_insert_position_metadata_from_rows(
            &rows,
            project_id,
            chapter_id,
            workflow_mode,
            None,
            adopted_video_ids.as_ref(),
            timeline_order.as_ref(),
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
        self.create_video_record(a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_INSERT.to_string(),
            workflow_mode: Some(workflow_mode.to_string()),
            prompt: format!("成片衔接片段，插入位置：{anchor_label}。"),
            negative_prompt: None,
            model: EXTERNAL_IMPORT_MODEL.to_string(),
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
        operation: AssemblyPostprocessOperation,
    ) -> Result<AssemblyPostprocessPlan, AppError> {
        let push = self.required_video_record(push_video_id).await?;
        if push.video_type != a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_PUSH {
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
        Ok(AssemblyPostprocessPlan {
            push: generated_video_summary(&push),
            base: generated_video_summary(&base),
            operation,
        })
    }

    pub async fn create_assembly_postprocess_edit_record_validated(
        &self,
        plan: AssemblyPostprocessPlan,
        input: AssemblyPostprocessEditRecordInput,
    ) -> Result<PreparedAssemblyPostprocessEditRecord, AppError> {
        if plan.base.width <= 0 || plan.base.height <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "后处理基底视频尺寸无效：{}x{}",
                plan.base.width, plan.base.height
            )));
        }
        if plan.base.fps <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "后处理基底视频帧率无效：{}",
                plan.base.fps
            )));
        }
        let (start_secs, end_secs, output_duration) = postprocess_edit_timing(
            plan.base.duration_secs,
            plan.operation,
            input.at_secs,
            input.start_secs,
            input.end_secs,
            input.source_duration_secs,
        )
        .map_err(AppError::InvalidArgument)?;
        let video = self
            .create_video_record(a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
                project_id: plan.push.project_id,
                chapter_id: plan.push.chapter_id,
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_POSTPROCESS_EDIT
                    .to_string(),
                workflow_mode: plan.push.workflow_mode.clone(),
                prompt: format!(
                    "后处理编辑：推送批次 #{} {}",
                    plan.push.id,
                    plan.operation.as_str()
                ),
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
            video: generated_video_summary(&video),
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
        if plan.operation == AssemblyPostprocessOperation::Delete {
            if source_video_id.is_some() || has_uploaded_source {
                return Err(AppError::InvalidArgument(
                    "删除后处理片段不接受来源视频".to_string(),
                ));
            }
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
            if video.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED {
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
            return Ok(AssemblyPostprocessSourcePlan::Existing(
                generated_video_summary(&video),
            ));
        }
        if has_uploaded_source {
            return Ok(AssemblyPostprocessSourcePlan::Upload);
        }
        Err(AppError::InvalidArgument(
            "插入或替换后处理片段时必须选择上传视频或 API 生成视频".to_string(),
        ))
    }

    pub(in crate::video) async fn latest_assembly_postprocess_version_for_push(
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
            if row.video_type != a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_POSTPROCESS_EDIT
                || row.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
            {
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
        crate::video::ensure_active_pushes_match_clip_prefix(&clip_ids, &active_source_ids)
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
        let (width, height, fps, duration_secs) =
            assembly_media_properties(source_clips.iter().map(|clip| {
                (
                    clip.video_id,
                    clip.duration_secs,
                    clip.width,
                    clip.height,
                    clip.fps,
                )
            }))
            .map_err(AppError::InvalidArgument)?;
        let source_video_ids = source_clips
            .iter()
            .map(|clip| clip.video_id)
            .collect::<Vec<_>>();
        let source_sort_orders = source_clips
            .iter()
            .map(|clip| clip.sort_order)
            .collect::<Vec<_>>();
        let video = self
            .create_video_record(a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
                project_id,
                chapter_id: Some(chapter_id),
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_PUSH.to_string(),
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
            video: generated_video_summary(&video),
            source_video_ids,
            push_index,
        })
    }
}
