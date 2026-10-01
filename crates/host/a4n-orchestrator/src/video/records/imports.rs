//! 导入、画布与 trim 记录。

use super::*;

impl crate::OrchestratorContext {
    pub(crate) async fn create_imported_chapter_video_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<PreparedImportedChapterVideoRecord, AppError> {
        let (source_result, prompt_result) = futures::join!(
            self.project_chapter_video_source(project_id, chapter_id, "导入视频", true),
            self.chapter_video_prompt_for_import(project_id, chapter_id, prompt_id),
        );
        let source = source_result?;
        let prompt = prompt_result?;
        if let Some(source_hash) = metadata_source_content_sha256(prompt.metadata_json.as_ref())
            && source_hash != source_text_hash(&source.source.content)
        {
            return Err(AppError::InvalidArgument(
                "当前提示词基于旧正文内容，请重新生成视频提示词".to_string(),
            ));
        }
        let video = self
            .create_video_record(a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
                project_id,
                chapter_id: Some(chapter_id),
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.to_string(),
                workflow_mode: Some(prompt.workflow_mode.clone()),
                prompt: imported_video_prompt_text(&prompt),
                negative_prompt: prompt.negative_prompt.clone(),
                model: EXTERNAL_IMPORT_MODEL.to_string(),
                width: properties.width,
                height: properties.height,
                duration_secs: properties.duration_secs,
                fps: properties.fps,
                source_video_id: None,
                source_image_id: None,
                provider_job_id: None,
                metadata_json: Some(imported_chapter_video_metadata(
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
        target: (&str, i32),
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, "导入参考媒体视频", true)
            .await?;
        let plan = self
            .require_current_shared_segment_plan(project_id, chapter_id, segment_plan_prompt_id)
            .await?;
        let shot_number = a4n_video_contract::resolve_video_segment_number(
            plan.metadata_json.as_ref(),
            target.0,
            target.1,
        )
        .map_err(AppError::InvalidArgument)?;
        let recipe = a4n_video_contract::video_segment_recipe_from_metadata(
            plan.metadata_json.as_ref(),
            shot_number,
        )
        .map_err(AppError::InvalidArgument)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("当前规划缺少片段 {shot_number} 的执行方案"))
        })?;
        if recipe.workflow_mode != VideoSegmentWorkflowModeV1::ReferenceMedia {
            return Err(AppError::InvalidArgument(format!(
                "片段 {shot_number} 当前未选择参考媒体方案"
            )));
        }
        let reference_media = recipe.reference_media.as_ref().ok_or_else(|| {
            AppError::InvalidArgument(format!("片段 {shot_number} 尚未保存参考媒体提示词和媒体槽"))
        })?;
        let prompt = crate::video::reference_media_rules::reference_media_prompt_from_recipe(
            reference_media,
        );
        if prompt.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "当前片段参考媒体提示词不能为空".to_string(),
            ));
        }
        validate_reference_media_import_options(recipe.duration_secs, &recipe.aspect_ratio)
            .map_err(AppError::InvalidArgument)?;
        let plan_identity = plan
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity"))
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少执行身份".to_string()))?;
        let video_execution_identity =
            crate::video::prompts::generation::video_segment_execution_identity(
                plan_identity,
                Some(plan.id),
                Some(&recipe),
                target,
            )?;
        self.create_video_record(a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_REFERENCE_MEDIA.to_string(),
            workflow_mode: Some(
                VideoSegmentWorkflowModeV1::ReferenceMedia
                    .as_str()
                    .to_string(),
            ),
            prompt,
            negative_prompt: None,
            model: EXTERNAL_IMPORT_MODEL.to_string(),
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
                    "aspect_ratio": recipe.aspect_ratio,
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
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.create_video_record(a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: None,
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
            workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
            prompt: source_file_name.to_string(),
            negative_prompt: None,
            model: EXTERNAL_IMPORT_MODEL.to_string(),
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
        for &video_id in &video_ids {
            if !seen.insert(video_id) {
                return Err(AppError::InvalidArgument(format!(
                    "AssembleNode 视频片段重复: {video_id}"
                )));
            }
        }
        let videos_by_id = self
            .generated_videos_by_ids(&video_ids)
            .await?
            .into_iter()
            .map(|video| (video.id, video))
            .collect::<std::collections::HashMap<_, _>>();
        let mut sources = Vec::with_capacity(video_ids.len());
        for video_id in video_ids {
            let video = videos_by_id
                .get(&video_id)
                .cloned()
                .ok_or_else(|| AppError::NotFound("视频不存在".to_string()))?;
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
            if video.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED {
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
        let (width, height, fps, duration_secs) =
            assembly_media_properties(sources.iter().map(|video| {
                (
                    video.id,
                    video.duration_secs,
                    video.width,
                    video.height,
                    video.fps,
                )
            }))
            .map_err(AppError::InvalidArgument)?;
        Ok(CanvasAssemblyPlan {
            width,
            height,
            fps,
            duration_secs,
            sources: sources.iter().map(generated_video_summary).collect(),
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
        frame_kind: CanvasFrameKind,
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
        if video.width <= 0 || video.height <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "源视频尺寸无效：{}x{}",
                video.width, video.height
            )));
        }
        let timestamp = canvas_frame_timestamp(video.duration_secs, frame_kind, at_secs)
            .map_err(AppError::InvalidArgument)?;
        Ok(CanvasFramePlan {
            video: generated_video_summary(&video),
            frame_kind,
            timestamp,
        })
    }

    pub async fn create_canvas_frame_record_validated(
        &self,
        plan: CanvasFramePlan,
        file_path: String,
        file_size: i64,
    ) -> Result<GeneratedImageSummary, AppError> {
        let image = self
            .create_generated_image_record(
                a4n_domain::media::CreateGeneratedImageRecordDraftInput {
                    project_id: plan.video.project_id,
                    image_type: IMAGE_TYPE_CANVAS_FRAME.to_string(),
                    file_path,
                    prompt: format!("视频 #{} 画布抽帧 {}", plan.video.id, plan.frame_kind),
                    model: "ffmpeg".to_string(),
                    width: plan.video.width,
                    height: plan.video.height,
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
                },
            )
            .await?;
        Ok(crate::media::generated_image_summary(&image))
    }

    /// 将 Runtime 已提取的真实尾帧登记为视频素材。
    /// 文件提取仍属于 Host；记录字段和归属元数据只在 Video application 生成。
    pub async fn create_video_tail_frame_record_validated(
        &self,
        video: &GeneratedVideoRecord,
        chapter_id: i64,
        file_path: String,
        file_size: i64,
    ) -> Result<a4n_domain::media::GeneratedImageRecord, AppError> {
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
        self.create_generated_image_record(
            a4n_domain::media::CreateGeneratedImageRecordDraftInput {
                project_id: video.project_id,
                image_type: IMAGE_TYPE_VIDEO_TAIL_FRAME.to_string(),
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
                        "source": IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE,
                        "video_id": video.id,
                        "shot_number": shot_number,
                        "source_content_sha256": metadata_source_content_sha256(
                            video.metadata_json.as_ref()
                        ),
                    },
                    "source": source,
                })),
                created_at: None,
            },
        )
        .await
    }

    pub async fn create_canvas_assembly_record_validated(
        &self,
        project_id: i64,
        plan: CanvasAssemblyPlan,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let source_video_ids = plan
            .sources
            .iter()
            .map(|video| video.id)
            .collect::<Vec<_>>();
        let record = self
            .create_video_record(a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
                project_id,
                chapter_id: None,
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS_ASSEMBLY.to_string(),
                workflow_mode: Some(WORKFLOW_MODE_CANVAS_ASSEMBLY.to_string()),
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
            .await?;
        Ok(generated_video_summary(&record))
    }

    pub fn prepare_video_trim_validated(
        &self,
        source: GeneratedVideoRecord,
        start_secs: f32,
        end_secs: f32,
    ) -> Result<VideoTrimPlan, AppError> {
        validate_video_trim_range(start_secs, end_secs).map_err(AppError::InvalidArgument)?;
        let source_workflow_mode = source.workflow_mode.clone().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "源视频 #{} 缺少 workflow_mode，无法按生成模式保存裁剪结果",
                source.id
            ))
        })?;
        if !is_generated_video_workflow_mode(&source_workflow_mode) {
            return Err(AppError::InvalidArgument(format!(
                "源视频 #{} workflow_mode 无效：{}",
                source.id, source_workflow_mode
            )));
        }
        if !source.duration_secs.is_finite() || source.duration_secs <= 0.0 {
            return Err(AppError::InvalidArgument(format!(
                "源视频 #{} 时长无效：{}",
                source.id, source.duration_secs
            )));
        }
        if end_secs > source.duration_secs {
            return Err(AppError::InvalidArgument(
                "裁剪结束时间超过视频时长".to_string(),
            ));
        }
        if source.width <= 0 || source.height <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "源视频尺寸无效：{}x{}",
                source.width, source.height
            )));
        }
        if source.fps <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "源视频帧率无效：{}",
                source.fps
            )));
        }
        let target_workflow_mode = match source.chapter_id {
            None if is_canvas_video_record(&source) => WORKFLOW_MODE_CANVAS_TRIM.to_string(),
            None => {
                return Err(AppError::InvalidArgument(format!(
                    "项目级源视频 workflow_mode 无效：{source_workflow_mode}"
                )));
            }
            Some(_) if !is_assembly_video_workflow_mode(&source_workflow_mode) => {
                return Err(AppError::InvalidArgument(format!(
                    "章节源视频不能使用 Canvas workflow_mode：{source_workflow_mode}"
                )));
            }
            Some(_) => source_workflow_mode,
        };
        Ok(VideoTrimPlan {
            source: generated_video_summary(&source),
            start_secs,
            end_secs,
            target_workflow_mode,
        })
    }

    pub async fn create_trimmed_video_record_validated(
        &self,
        plan: VideoTrimPlan,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let record = self
            .create_video_record(a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
                project_id: plan.source.project_id,
                chapter_id: plan.source.chapter_id,
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_TRIMMED_CLIP.to_string(),
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
                metadata_json: Some(trimmed_video_metadata(
                    plan.source.metadata_json.as_ref(),
                    plan.start_secs,
                    plan.end_secs,
                )),
            })
            .await?;
        Ok(generated_video_summary(&record))
    }

    pub async fn required_video_record(
        &self,
        video_id: i64,
    ) -> Result<GeneratedVideoRecord, AppError> {
        self.generated_video_by_id(video_id)
            .await?
            .ok_or_else(|| AppError::NotFound("视频不存在".to_string()))
    }

    pub(in crate::video) async fn video_record_is_pushed_source(
        &self,
        video: &GeneratedVideoRecord,
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

    pub(in crate::video) async fn active_pushed_source_ids(
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
}
