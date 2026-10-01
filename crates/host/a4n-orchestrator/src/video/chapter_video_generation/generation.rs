impl OrchestratorContext {
    pub async fn preview_chapter_video_request_validated(
        &self,
        host_port: &dyn ChapterVideoPreviewHostPort,
        input: GenerateChapterVideoInput,
        prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
    ) -> Result<ChapterVideoRequestPreview, AppError> {
        let prepared = self
            .prepare_chapter_video_generation(
                host_port,
                input,
                VideoProviderValidationMode::Preview,
                prompt_override,
            )
            .await?;
        let body = host_port
            .chapter_video_provider_request_body_for_preview(VideoProviderRequestInput {
                user_id: prepared.user_id,
                project_id: prepared.project_id,
                config: prepared.context.config.clone(),
                request: prepared.context.request.clone(),
                provider_idempotency_key_sha256: None,
            })
            .await?;
        let descriptor = chapter_video_provider_descriptor(&prepared.context, &body.source_label)
            .map_err(AppError::InvalidArgument)?;
        Ok(ChapterVideoRequestPreview {
            request: body.request,
            request_kind: descriptor.request_kind,
            request_title: descriptor.request_title,
            note: descriptor.note,
        })
    }

    pub async fn run_chapter_video_generation_validated(
        &self,
        host_port: &dyn ChapterVideoGenerationHostPort,
        input: GenerateChapterVideoInput,
        progress: &mut VideoGenerationProgress<'_>,
    ) -> Result<GeneratedVideoRecord, AppError> {
        progress.progress(
            0,
            VIDEO_GENERATION_PROGRESS_TOTAL,
            "正在检查视频提示词和配置",
        )?;
        progress.check_cancelled("视频生成已停止")?;
        let prepared = self
            .prepare_chapter_video_generation(
                host_port,
                input,
                VideoProviderValidationMode::Generate,
                None,
            )
            .await?;
        let context = &prepared.context;
        self.ensure_video_capacity(context.config.id, context.config.max_concurrent)
            .await?;

        progress.progress(2, VIDEO_GENERATION_PROGRESS_TOTAL, "正在创建视频生成记录")?;
        progress.check_cancelled("视频生成已停止")?;
        let video = self
            .create_chapter_video_generation_record(host_port, &prepared)
            .await?;
        if let Err(error) = self
            .ensure_created_video_capacity(
                context.config.id,
                context.config.max_concurrent,
                video.id,
            )
            .await
        {
            return self
                .mark_generated_video_failed_validated(
                    video.id,
                    Some(context.prompt.id),
                    &error.to_string(),
                )
                .await;
        }

        progress.progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, "正在调用视频模型")?;
        if progress.check_cancelled("视频生成已停止").is_err() {
            self.mark_generated_video_cancelled_validated(
                video.id,
                Some(context.prompt.id),
                "视频生成已停止",
            )
            .await?;
            return Err(AppError::Cancelled);
        }
        let prompt_length_context = video_request_prompt_length_context(&context.request);
        let mut provider_input = VideoProviderRequestInput {
            user_id: prepared.user_id,
            project_id: prepared.project_id,
            config: context.config.clone(),
            request: context.request.clone(),
            provider_idempotency_key_sha256: None,
        };
        provider_input.provider_idempotency_key_sha256 = progress
            .checkpoint_provider_request(
                video.id,
                &provider_input.config,
                &provider_input.request,
                &prepared.staging_artifacts,
            )
            .await?;
        let result = host_port
            .submit_chapter_video(provider_input, progress)
            .await
            .map_err(|error| {
                append_video_request_prompt_length_context(
                    error.to_string(),
                    &prompt_length_context,
                )
            });
        if let Ok(result) = &result
            && let Some(job_id) = result
                .provider_job_id
                .as_deref()
                .filter(|id| !id.trim().is_empty())
        {
            progress
                .checkpoint_provider_job(video.id, &context.config, job_id)
                .await?;
        }
        if result.as_ref().is_ok_and(|result| result.bytes.is_empty()) {
            progress.progress(
                4,
                VIDEO_GENERATION_PROGRESS_TOTAL,
                "视频模型已提交异步任务，开始等待生成结果",
            )?;
        } else {
            progress.progress(
                6,
                VIDEO_GENERATION_PROGRESS_TOTAL,
                "视频模型已返回，正在保存文件",
            )?;
        }
        let updated = self
            .apply_video_provider_result_validated(
                VideoProviderExecutionInput {
                    video,
                    config: context.config.clone(),
                    prompt_id: Some(context.prompt.id),
                    missing_result_message: "视频 provider 未返回文件或任务 ID".to_string(),
                    cancel_token: progress.cancel_token(),
                },
                result,
                host_port.video_provider_host_port(),
            )
            .await?;
        progress.progress(7, VIDEO_GENERATION_PROGRESS_TOTAL, "视频文件已保存")?;
        Ok(updated)
    }

    async fn prepare_chapter_video_generation(
        &self,
        host_port: &dyn ChapterVideoPreviewHostPort,
        input: GenerateChapterVideoInput,
        mode: VideoProviderValidationMode,
        prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
    ) -> Result<PreparedChapterVideoGeneration, AppError> {
        let GenerateChapterVideoInput {
            user_id,
            project_id,
            chapter_id,
            prompt_id,
            config_id,
        } = input;
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let (operation, writable) = match mode {
            VideoProviderValidationMode::Generate => ("生成视频", true),
            VideoProviderValidationMode::Preview => ("预览视频生成请求", false),
        };
        let (prompt_result, source_result) = futures::join!(
            self.chapter_video_prompt(prompt_id),
            self.project_chapter_video_source(project_id, chapter_id, operation, writable),
        );
        let mut prompt = prompt_result?;
        if prompt.project_id != project_id || prompt.chapter_id != chapter_id {
            return Err(invalid_argument("视频提示词不属于该章节"));
        }
        let source = source_result?;
        if prompt.prompt_type != chapter_video_prompt::PROMPT_TYPE_VIDEO {
            return Err(invalid_argument("请选择 video_prompt 类型的提示词生成视频"));
        }
        let execution_contract = self
            .require_current_segment_recipe_for_video_prompt(&prompt)
            .await?;

        let provider_workflow =
            prompt_provider_workflow_for_recipe(&prompt, &execution_contract.recipe)
                .map_err(AppError::InvalidArgument)?;
        let mut provider_input_note = None;
        if mode == VideoProviderValidationMode::Preview {
            apply_chapter_video_prompt_preview_override(
                &mut prompt,
                prompt_override,
                &mut provider_input_note,
            )
            .map_err(AppError::InvalidArgument)?;
        } else {
            if prompt.status == chapter_video_prompt::STATUS_RUNNING {
                return Err(invalid_argument(
                    "当前视频提示词正在生成视频，请先等待完成后再重新生成",
                ));
            }
            if let Some(source_hash) = metadata_source_content_sha256(prompt.metadata_json.as_ref())
                && source_hash != source_text_hash(&source.source.content)
            {
                return Err(invalid_argument(
                    "当前提示词基于旧正文内容，请重新生成视频提示词",
                ));
            }
        }

        let config_id = if mode == VideoProviderValidationMode::Preview {
            self.chapter_video_preview_config_id(&prompt, config_id)
                .await?
        } else {
            config_id
        };
        let config = host_port
            .resolve_video_config_for_preview(config_id)
            .await?;
        validate_video_duration_for_provider_config_secs(&config, prompt.duration_secs)
            .map_err(AppError::InvalidArgument)?;

        let prompt_reference_image_ids_json = prompt.reference_image_ids_json.clone();
        let prompt_reference_image_ids =
            reference_image_ids(prompt_reference_image_ids_json.as_ref())
                .map_err(AppError::InvalidArgument)?;
        let prompt_reference_image_count = prompt_reference_image_ids.len();
        let source_prompt_type = Some(execution_contract.source_prompt_type);
        let video_execution_identity = execution_contract.execution_identity;
        let prompt_workflow_mode = prompt.workflow_mode.clone();
        if provider_workflow.uses_text_only() && !prompt_reference_image_ids.is_empty() {
            return Err(invalid_argument(
                "纯文本生成不能绑定参考图片；请清空当前片段参考图后重试",
            ));
        }
        let storyboard_workflow = provider_workflow.uses_storyboard();
        enforce_provider_image_binding_contract(
            &config.backend,
            provider_workflow,
            prompt_reference_image_count,
        )
        .map_err(AppError::InvalidArgument)?;

        let transport = host_port.resolve_image_transport_for_preview(&config.backend)?;
        if transport == ProviderImageInputTransport::RemoteTempUpload {
            append_provider_input_note(
                &mut provider_input_note,
                "远端临时媒体上传：生成时会携带用户、项目和来源图片归属上传，并要求远端返回最长 24 小时有效的 HTTPS artifact；预览省略真实图片数据。",
            );
        }
        let references = self
            .resolve_chapter_provider_references(
                project_id,
                chapter_id,
                &prompt,
                &prompt_reference_image_ids,
                prompt_reference_image_ids_json.as_ref(),
                provider_workflow,
                &execution_contract.recipe,
                &mut provider_input_note,
            )
            .await?;
        let selected_reference_image_ids = if config.backend != NEWAPI_VIDEO_BACKEND {
            references.provider_reference_image_ids.clone()
        } else {
            Vec::new()
        };
        let selected_first_frame_image_id = if storyboard_workflow
            || provider_workflow.uses_keyframes()
            || provider_workflow.uses_text_only()
        {
            None
        } else {
            references.boundary_frame_ids.first
        };
        let selected_last_frame_image_id = if matches!(
            provider_workflow,
            ProviderGenerationWorkflow::BoundaryFrames
        ) {
            references.boundary_frame_ids.last
        } else {
            None
        };
        let image_inputs = host_port
            .resolve_chapter_video_image_inputs_for_preview(ChapterVideoImageInput {
                user_id,
                project_id,
                chapter_id,
                mode,
                transport,
                reference_image_ids: selected_reference_image_ids,
                first_frame_image_id: selected_first_frame_image_id,
                last_frame_image_id: selected_last_frame_image_id,
            })
            .await?;
        if config.backend == NEWAPI_VIDEO_BACKEND && mode == VideoProviderValidationMode::Generate {
            ensure_newapi_first_frame_image_supported(image_inputs.first_frame_image_url.as_ref())
                .map_err(AppError::InvalidArgument)?;
        }
        let first_frame_image_url_available = image_inputs.first_frame_image_url.is_some();
        let last_frame_image_url_available = image_inputs.last_frame_image_url.is_some();
        let image_binding = ProviderPromptImageBinding {
            has_first_frame_image: first_frame_image_url_available,
            has_last_frame_image: last_frame_image_url_available,
            reference_image_count: image_inputs.reference_image_urls.len(),
            has_storyboard_reference_image: storyboard_workflow
                && references.storyboard_image_id.is_some(),
        };
        let full_prompt = build_video_generation_provider_prompt_with_renderer(
            self,
            &prompt,
            provider_workflow,
            image_binding,
        )
        .await
        .map_err(AppError::InvalidArgument)?;
        let prompt_source = prompt
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source"))
            .cloned();
        let request = VideoGenerationRequest {
            prompt: full_prompt,
            negative_prompt: provider_specific_negative_prompt(prompt.negative_prompt.as_deref()),
            width: config.default_width,
            height: config.default_height,
            duration_secs: prompt.duration_secs,
            fps: config.default_fps,
            model: config.model.clone(),
            media: a4n_video::VideoMediaInput::from_images(
                image_inputs.first_frame_image_url,
                image_inputs.last_frame_image_url,
                image_inputs.reference_image_urls,
            )
            .map_err(AppError::InvalidArgument)?,
        };
        enforce_provider_prompt_length_contract(
            &config.backend,
            &request.prompt,
            request.negative_prompt.as_deref(),
        )
        .map_err(AppError::InvalidArgument)?;
        Ok(PreparedChapterVideoGeneration {
            user_id,
            project_id,
            chapter_id,
            staging_artifacts: image_inputs.staging_artifacts,
            context: ChapterVideoProviderRequestContext {
                prompt,
                config,
                request,
                prompt_reference_image_ids: prompt_reference_image_ids_json,
                provider_reference_image_ids: references.provider_reference_image_ids_json,
                prompt_source,
                workflow_mode: prompt_workflow_mode,
                source_prompt_type,
                video_execution_identity,
                boundary_frame_ids: references.boundary_frame_ids,
                prompt_reference_image_count,
                provider_reference_image_count: references.provider_reference_image_count,
                first_frame_image_url_available,
                last_frame_image_url_available,
                provider_input_note,
            },
        })
    }

    async fn create_chapter_video_generation_record(
        &self,
        host_port: &dyn ChapterVideoGenerationHostPort,
        prepared: &PreparedChapterVideoGeneration,
    ) -> Result<GeneratedVideoRecord, AppError> {
        let context = &prepared.context;
        let snapshot_request = sanitized_chapter_video_request_for_snapshot(context)
            .map_err(AppError::InvalidArgument)?;
        let body = host_port
            .chapter_video_provider_request_body(VideoProviderRequestInput {
                user_id: prepared.user_id,
                project_id: prepared.project_id,
                config: context.config.clone(),
                request: snapshot_request,
                provider_idempotency_key_sha256: None,
            })
            .await?;
        let descriptor =
            chapter_video_provider_descriptor(context, &format!("生成时{}", body.source_label))
                .map_err(AppError::InvalidArgument)?;
        let provider_request_snapshot = provider_request_snapshot_value(
            "chapter_video",
            descriptor.request_title,
            descriptor.request_kind,
            Some("chapter_video_prompt"),
            Some(descriptor.note),
            body.request,
        );
        let mut metadata = json!({
            "config_id": context.config.id,
            "prompt_id": context.prompt.id,
            "shot_number": context.prompt.shot_number,
            "source": context.prompt_source.clone(),
            "source_prompt_type": context.source_prompt_type.clone(),
            "video_execution_identity": context.video_execution_identity.clone(),
            "prompt_output_language": context.prompt.metadata_json.as_ref()
                .and_then(|metadata| metadata.get("prompt_output_language")),
            "prompt_reference_image_ids": context.prompt_reference_image_ids.clone(),
            "prompt_reference_image_count": context.prompt_reference_image_count,
            "provider_reference_image_ids": context.provider_reference_image_ids.clone(),
            "provider_reference_image_count": context.provider_reference_image_count,
            "start_frame_image_id": context.boundary_frame_ids.first,
            "boundary_frame_image_ids": {
                "first": context.boundary_frame_ids.first,
                "last": context.boundary_frame_ids.last
            },
            "boundary_frame_image_count": usize::from(context.boundary_frame_ids.first.is_some())
                + usize::from(context.boundary_frame_ids.last.is_some()),
            "boundary_frame_image_url_count": usize::from(context.first_frame_image_url_available)
                + usize::from(context.last_frame_image_url_available),
            "boundary_frame_request": {
                "first_frame_image_url": context.first_frame_image_url_available,
                "last_frame_image_url": context.last_frame_image_url_available,
            },
            "provider_request_snapshot": provider_request_snapshot.clone(),
            "runtime_config_snapshot": host_port.runtime_config_snapshot(),
        });
        if context.prompt.timeline_item_id.is_none()
            && let Some(fingerprint) = context
                .video_execution_identity
                .get("generation_asset_fingerprint")
                .cloned()
        {
            metadata["generation_asset_fingerprint"] = fingerprint;
        }
        self.create_video_generation_record_validated(VideoGenerationRecordInput {
            project_id: prepared.project_id,
            chapter_id: Some(prepared.chapter_id),
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.into(),
            workflow_mode: Some(context.workflow_mode.clone()),
            prompt: context.request.prompt.clone(),
            negative_prompt: context.request.negative_prompt.clone(),
            model: context.config.model.clone(),
            width: context.request.width,
            height: context.request.height,
            duration_secs: context.request.duration_secs as f32,
            fps: context.request.fps,
            source_video_id: None,
            source_image_id: context.boundary_frame_ids.first,
            metadata_json: Some(metadata),
            provider_request_snapshot,
            provider_backend: context.config.backend.clone(),
            step_key: "generate_chapter_video".to_string(),
            title: "章节视频 provider 生成".to_string(),
            prompt_template_name: Some("chapter_video_prompt".to_string()),
            prompt_id: Some(context.prompt.id),
        })
        .await
    }

    async fn chapter_video_preview_config_id(
        &self,
        prompt: &ChapterVideoPromptRecord,
        requested_config_id: Option<i64>,
    ) -> Result<Option<i64>, AppError> {
        if requested_config_id.is_some() {
            return Ok(requested_config_id);
        }
        let Some(video_id) = prompt.generated_video_id else {
            return Ok(None);
        };
        let video = self.generated_video_by_id(video_id).await?.ok_or_else(|| {
            AppError::NotFound(format!(
                "视频提示词 #{} 关联的视频 #{video_id} 不存在",
                prompt.id
            ))
        })?;
        linked_video_preview_config_id(
            prompt.id,
            prompt.project_id,
            prompt.chapter_id,
            video.id,
            video.project_id,
            video.chapter_id,
            video.metadata_json.as_ref(),
        )
        .map_err(AppError::InvalidArgument)
    }

    pub(crate) async fn validate_chapter_video_batch_prompts(
        &self,
        input: &GenerateChapterVideosBatchInput,
    ) -> Result<Vec<i64>, AppError> {
        if input.prompt_ids.is_empty() {
            return Err(invalid_argument("批量生成视频至少需要选择一条视频提示词"));
        }
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Editor)
            .await?;
        let (source_result, prompts_result) = futures::join!(
            self.project_chapter_video_source(
                input.project_id,
                input.chapter_id,
                "批量生成视频",
                true,
            ),
            async {
                let mut unique_prompt_ids = input.prompt_ids.clone();
                unique_prompt_ids.sort_unstable();
                unique_prompt_ids.dedup();
                if unique_prompt_ids.len() != input.prompt_ids.len() {
                    return Err(invalid_argument("批量生成视频 prompt_ids 不能重复"));
                }
                self.chapter_video_prompts_for_batch_generation(&unique_prompt_ids)
                    .await
                    .map(|rows| (unique_prompt_ids, rows))
            },
        );
        source_result?;
        let (unique_prompt_ids, rows) = prompts_result?;
        if rows.len() != unique_prompt_ids.len() {
            return Err(invalid_argument("部分视频提示词不存在，不能批量生成视频"));
        }
        let mut validation_cache = VideoPromptValidationCache::default();
        for row in &rows {
            if row.project_id != input.project_id || row.chapter_id != input.chapter_id {
                return Err(invalid_argument(format!(
                    "视频提示词 #{} 不属于当前项目章节",
                    row.id
                )));
            }
            if row.prompt_type != chapter_video_prompt::PROMPT_TYPE_VIDEO {
                return Err(invalid_argument(format!(
                    "视频提示词 #{} 不是 video_prompt 类型",
                    row.id
                )));
            }
            self.require_current_segment_recipe_for_video_prompt_cached(row, &mut validation_cache)
                .await?;
            super::ensure_provider_generation_workflow(row).map_err(AppError::InvalidArgument)?;
            if row.status == chapter_video_prompt::STATUS_RUNNING {
                return Err(invalid_argument(format!(
                    "视频提示词 #{} 正在运行，不能批量生成视频",
                    row.id
                )));
            }
        }
        Ok(rows.into_iter().map(|row| row.id).collect())
    }

    async fn require_current_segment_recipe_for_video_prompt(
        &self,
        prompt: &ChapterVideoPromptRecord,
    ) -> Result<VideoPromptExecutionContract, AppError> {
        self.require_current_segment_recipe_for_video_prompt_cached(
            prompt,
            &mut VideoPromptValidationCache::default(),
        )
        .await
    }

    async fn require_current_segment_recipe_for_video_prompt_cached(
        &self,
        prompt: &ChapterVideoPromptRecord,
        cache: &mut VideoPromptValidationCache,
    ) -> Result<VideoPromptExecutionContract, AppError> {
        let metadata = prompt
            .metadata_json
            .as_ref()
            .and_then(Value::as_object)
            .ok_or_else(|| invalid_argument(format!("视频提示词 #{} 缺少 metadata", prompt.id)))?;
        let source_prompt_type =
            required_prompt_source_prompt_type(prompt.id, prompt.metadata_json.as_ref())
                .map_err(AppError::InvalidArgument)?;
        if prompt.timeline_item_id.is_some() {
            if source_prompt_type != super::timeline::INSERTED_SEGMENT_SOURCE_PROMPT_TYPE {
                return Err(invalid_argument(format!(
                    "插入片段提示词 #{} source_prompt_type 无效",
                    prompt.id
                )));
            }
            let item_id = prompt
                .timeline_item_id
                .filter(|id| *id > 0)
                .ok_or_else(|| {
                    invalid_argument(format!("插入片段提示词 #{} 缺少有效时间线项 ID", prompt.id))
                })?;
            let timeline_key = (prompt.project_id, prompt.chapter_id);
            match cache.inserted_timeline_items.get(&timeline_key) {
                Some(_) => {}
                None => {
                    let timeline = self
                        .chapter_video_timeline_by_scope(
                            prompt.project_id,
                            prompt.chapter_id,
                            chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
                        )
                        .await?
                        .ok_or_else(|| invalid_argument("当前章节不存在插入片段时间线"))?;
                    let snapshot = self.chapter_video_timeline_snapshot(timeline.id).await?;
                    cache
                        .inserted_timeline_items
                        .insert(timeline_key, timeline_items_by_id(snapshot.items));
                }
            }
            let items_by_id = cache
                .inserted_timeline_items
                .get(&timeline_key)
                .expect("inserted timeline items are loaded");
            let item = items_by_id
                .get(&item_id)
                .ok_or_else(|| invalid_argument(format!("插入片段时间线项 #{item_id} 不存在")))?;
            if item.item_kind != a4n_db::video::TIMELINE_INSERTED_KIND
                || item.status != a4n_db::video::TIMELINE_ACTIVE_STATUS
                || item.insert_plan_prompt_id != Some(prompt.id)
            {
                return Err(invalid_argument(format!(
                    "插入片段提示词 #{} 对应的时间线项不是当前活动项，请刷新时间线后重试",
                    prompt.id
                )));
            }
            let recipe = serde_json::from_value::<VideoSegmentExecutionRecipeV1>(
                metadata.get("segment_recipe").cloned().ok_or_else(|| {
                    invalid_argument(format!("插入片段提示词 #{} 缺少 segment_recipe", prompt.id))
                })?,
            )
            .map_err(|error| {
                invalid_argument(format!(
                    "插入片段提示词 #{} segment_recipe 无效: {error}",
                    prompt.id
                ))
            })?;
            recipe
                .validate_generation_ready()
                .map_err(AppError::InvalidArgument)?;
            let execution_identity = inserted_video_execution_identity(prompt, &recipe)?;
            return Ok(VideoPromptExecutionContract {
                source_prompt_type: source_prompt_type.to_string(),
                execution_identity,
                recipe,
            });
        }
        if source_prompt_type != chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN {
            return Err(invalid_argument(format!(
                "视频提示词 #{} source_prompt_type={source_prompt_type}，当前要求 {}",
                prompt.id,
                chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
            )));
        }
        let execution_identity =
            required_video_execution_identity(prompt.id, prompt.metadata_json.as_ref())
                .map_err(AppError::InvalidArgument)?;
        let plan_id = execution_identity
            .get("segment_plan_prompt_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                invalid_argument(format!("视频提示词 #{} 缺少片段规划 ID", prompt.id))
            })?;
        let source_shot_id = execution_identity
            .get("source_shot_id")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_argument(format!("视频提示词 #{} 缺少镜头 ID", prompt.id)))?;
        let segment_index_in_shot = execution_identity
            .get("segment_index_in_shot")
            .and_then(Value::as_i64)
            .and_then(|value| i32::try_from(value).ok())
            .ok_or_else(|| invalid_argument(format!("视频提示词 #{} 缺少镜头内序号", prompt.id)))?;
        match cache.shared_segment_plans.get(&plan_id) {
            Some(_) => {}
            None => {
                let plan = self
                    .require_current_shared_segment_plan(
                        prompt.project_id,
                        prompt.chapter_id,
                        plan_id,
                    )
                    .await?;
                cache.shared_segment_plans.insert(plan_id, plan);
            }
        }
        let plan = cache
            .shared_segment_plans
            .get(&plan_id)
            .expect("shared segment plan is loaded");
        a4n_video_contract::ensure_video_segment_source_current(
            plan.metadata_json.as_ref(),
            source_shot_id,
        )
        .map_err(AppError::InvalidArgument)?;
        let segment_number = match cache.shared_segment_indexes.get(&plan_id) {
            Some(index) => {
                let segment_number =
                    resolve_cached_segment_number(index, source_shot_id, segment_index_in_shot)
                        .map_err(AppError::InvalidArgument)?;
                plan.style
                    .as_deref()
                    .map(str::trim)
                    .filter(|style| !style.is_empty())
                    .ok_or_else(|| invalid_argument("当前片段规划缺少 style"))?;
                segment_number
            }
            None => {
                let source_shot_id = source_shot_id.trim();
                if source_shot_id.is_empty() || segment_index_in_shot <= 0 {
                    return Err(invalid_argument(
                        "稳定片段目标必须提供镜头 ID 和正整数镜头内序号",
                    ));
                }
                let segments = serde_json::from_value::<Vec<VideoSegmentPlanV2>>(
                    plan.metadata_json
                        .as_ref()
                        .and_then(|metadata| metadata.get("segments"))
                        .cloned()
                        .ok_or_else(|| invalid_argument("片段规划 metadata_json 缺少 segments"))?,
                )
                .map_err(|error| invalid_argument(format!("片段规划 segments 无效：{error}")))?;
                let mut segment_numbers_by_shot = HashMap::with_capacity(segments.len());
                for segment in &segments {
                    segment_numbers_by_shot
                        .entry(segment.source_shot_id.clone())
                        .or_insert_with(HashMap::new)
                        .entry(segment.segment_index_in_shot)
                        .or_insert(segment.segment_number);
                }
                let segment_number = segment_numbers_by_shot
                    .get(source_shot_id)
                    .and_then(|by_index| by_index.get(&segment_index_in_shot))
                    .copied()
                    .ok_or_else(|| {
                        invalid_argument(format!(
                            "当前片段规划不存在镜头 {source_shot_id} 的第 {segment_index_in_shot} 个片段"
                        ))
                    })?;
                let plan_style = plan
                    .style
                    .as_deref()
                    .map(str::trim)
                    .filter(|style| !style.is_empty())
                    .ok_or_else(|| invalid_argument("当前片段规划缺少 style"))?;
                let recipes = video_segment_recipes_from_metadata(plan.metadata_json.as_ref())
                    .map_err(AppError::InvalidArgument)?;
                validate_video_segment_recipes_for_plan(&recipes, &segments, plan_style)
                    .map_err(AppError::InvalidArgument)?;
                let recipe_count = recipes.len();
                let recipes_by_number = recipes.into_iter().fold(
                    HashMap::with_capacity(recipe_count),
                    |mut by_number, recipe| {
                        by_number.entry(recipe.segment_number).or_insert(recipe);
                        by_number
                    },
                );
                cache.shared_segment_indexes.insert(
                    plan_id,
                    SharedSegmentValidationIndex {
                        segments_by_number: segments
                            .into_iter()
                            .map(|segment| (segment.segment_number, segment))
                            .collect(),
                        segment_numbers_by_shot,
                        recipes_by_number,
                    },
                );
                segment_number
            }
        };
        let recipe = cache
            .shared_segment_indexes
            .get(&plan_id)
            .and_then(|index| index.recipes_by_number.get(&segment_number))
            .cloned()
            .ok_or_else(|| {
                invalid_argument(format!("当前规划缺少片段 {segment_number} 的执行方案"))
            })?;
        let segment = cache
            .shared_segment_indexes
            .get(&plan_id)
            .and_then(|index| index.segments_by_number.get(&segment_number))
            .ok_or_else(|| invalid_argument("当前规划缺少执行片段"))?;
        recipe
            .validate_director_generation(segment)
            .map_err(AppError::InvalidArgument)?;
        let plan_identity = plan
            .metadata_json
            .as_ref()
            .and_then(|value| value.get("video_execution_identity"))
            .cloned()
            .ok_or_else(|| invalid_argument("当前片段规划缺少执行身份"))?;
        let expected_identity = super::prompts::generation::video_segment_execution_identity(
            plan_identity,
            Some(plan_id),
            Some(&recipe),
            (source_shot_id, segment_index_in_shot),
        )?;
        let valid = super::prompts::records::video_execution_segment_lineage_matches(
            prompt.metadata_json.as_ref(),
            &expected_identity,
        ) && prompt.workflow_mode == recipe.workflow_mode.as_str()
            && prompt.style.as_deref().map(str::trim) == Some(recipe.style.trim())
            && prompt.aspect_ratio.as_deref() == Some(recipe.aspect_ratio.as_str())
            && metadata
                .get("prompt_output_language")
                .and_then(Value::as_str)
                == Some(recipe.prompt_output_language.as_str())
            && reference_image_ids(prompt.reference_image_ids_json.as_ref())
                .map_err(AppError::InvalidArgument)?
                == recipe.provider_reference_image_ids();
        if !valid {
            return Err(invalid_argument(format!(
                "视频提示词 #{} 不属于当前片段执行方案，请在当前片段配置中重新生成",
                prompt.id
            )));
        }
        Ok(VideoPromptExecutionContract {
            source_prompt_type: source_prompt_type.to_string(),
            execution_identity,
            recipe,
        })
    }

    async fn resolve_chapter_provider_references(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt: &ChapterVideoPromptRecord,
        prompt_reference_image_ids: &[i64],
        prompt_reference_image_ids_json: Option<&Value>,
        provider_workflow: ProviderGenerationWorkflow,
        recipe: &VideoSegmentExecutionRecipeV1,
        provider_input_note: &mut Option<String>,
    ) -> Result<ChapterProviderReferences, AppError> {
        let storyboard_workflow = provider_workflow.uses_storyboard();
        let (boundary_frame_ids, storyboard_image_id) = if provider_workflow.uses_text_only()
            || provider_workflow.uses_keyframes()
        {
            (BoundaryFrameImageIds::default(), None)
        } else if provider_workflow.uses_first_frame_chain() {
            let start = self
                .first_frame_chain_start_image_id_for_prompt(project_id, chapter_id, prompt, recipe)
                .await?;
            if start.is_none() {
                return Err(invalid_argument(
                    super::first_frame_chain_missing_start_image_message(prompt),
                ));
            }
            (
                BoundaryFrameImageIds {
                    first: start,
                    last: None,
                },
                None,
            )
        } else if storyboard_workflow {
            let storyboard_image_id = self
                .storyboard_image_id_for_prompt(project_id, chapter_id, prompt)
                .await?;
            if storyboard_image_id.is_none() {
                return Err(invalid_argument(super::storyboard_missing_image_message(
                    prompt,
                )));
            }
            (BoundaryFrameImageIds::default(), storyboard_image_id)
        } else {
            let frame_ids = self
                .boundary_frame_image_ids_for_prompt(project_id, chapter_id, prompt)
                .await?;
            if let Some(message) = super::boundary_frame_missing_image_message(
                prompt,
                frame_ids.first.is_some(),
                frame_ids.last.is_some(),
            ) {
                return Err(invalid_argument(message));
            }
            (frame_ids, None)
        };
        let provider_reference_image_ids = if provider_workflow.uses_text_only() {
            Vec::new()
        } else if provider_workflow.uses_keyframes() {
            prompt_reference_image_ids.to_vec()
        } else if storyboard_workflow {
            reference_ids_with_storyboard_image(storyboard_image_id, prompt_reference_image_ids)
        } else {
            prompt_reference_image_ids.to_vec()
        };
        let provider_reference_ids_are_authoritative = provider_workflow.uses_text_only()
            || provider_workflow.uses_keyframes()
            || storyboard_workflow;
        if storyboard_workflow && storyboard_image_id.is_some() {
            append_provider_input_note(
                provider_input_note,
                format!(
                    "分镜模式已将分镜图作为第 1 张 reference_images 发送，后续为素材参考图；实际参考图 ID 顺序：{}",
                    joined_ids(&provider_reference_image_ids)
                ),
            );
        }
        let provider_reference_image_ids_json = if provider_reference_ids_are_authoritative {
            Some(json!(provider_reference_image_ids.clone()))
        } else {
            prompt_reference_image_ids_json.cloned()
        };
        Ok(ChapterProviderReferences {
            boundary_frame_ids,
            storyboard_image_id,
            provider_reference_image_ids_json,
            provider_reference_image_count: provider_reference_image_ids.len(),
            provider_reference_image_ids,
        })
    }

    async fn boundary_frame_image_ids_for_prompt(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt: &ChapterVideoPromptRecord,
    ) -> Result<BoundaryFrameImageIds, AppError> {
        let (first_frame_number, last_frame_number) =
            boundary_frame_numbers_for_prompt(prompt).map_err(AppError::InvalidArgument)?;
        let frame_prompts = self
            .chapter_video_boundary_frame_prompt_candidates(
                project_id,
                chapter_id,
                first_frame_number,
                last_frame_number,
            )
            .await?;
        let mut first_prompt_id = None;
        let mut last_prompt_id = None;
        for frame in &frame_prompts {
            if !frame_prompt_matches_video_prompt(prompt, frame) {
                continue;
            }
            if first_prompt_id.is_none() && frame.shot_number() == first_frame_number {
                first_prompt_id = Some(frame.id);
            }
            if last_prompt_id.is_none() && frame.shot_number() == last_frame_number {
                last_prompt_id = Some(frame.id);
            }
            if first_prompt_id.is_some() && last_prompt_id.is_some() {
                break;
            }
        }
        if first_prompt_id.is_none() && last_prompt_id.is_none() {
            return Ok(BoundaryFrameImageIds::default());
        }
        let images = self
            .generated_images_for_video_prompt_references(project_id, chapter_id)
            .await?;
        let (first_image_id, last_image_id) =
            latest_boundary_frame_image_ids(&images, (first_prompt_id, last_prompt_id));
        Ok(BoundaryFrameImageIds {
            first: first_image_id,
            last: last_image_id,
        })
    }

    pub(in crate::video) async fn completed_chapter_continuity_source(
        &self,
        project_id: i64,
        chapter_id: i64,
        source_video_id: i64,
    ) -> Result<GeneratedVideoRecord, AppError> {
        let source_video = self
            .generated_video_by_id(source_video_id)
            .await?
            .ok_or_else(|| {
                invalid_argument(format!(
                    "连续性来源视频 #{source_video_id} 已不存在，请重新选择"
                ))
            })?;
        if source_video.project_id != project_id
            || source_video.chapter_id != Some(chapter_id)
            || source_video.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
        {
            return Err(invalid_argument(format!(
                "连续性来源视频 #{source_video_id} 不再是当前章节的已完成视频，请重新选择"
            )));
        }
        Ok(source_video)
    }

    async fn first_frame_chain_start_image_id_for_prompt(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt: &ChapterVideoPromptRecord,
        recipe: &VideoSegmentExecutionRecipeV1,
    ) -> Result<Option<i64>, AppError> {
        if prompt.timeline_item_id.is_none() && prompt.shot_number <= 1 {
            let candidates = self
                .chapter_video_first_frame_prompt_candidates(project_id, chapter_id)
                .await?;
            let Some(first_frame_prompt) = candidates
                .iter()
                .find(|frame| first_frame_prompt_matches_video_prompt(prompt, frame))
            else {
                return Ok(None);
            };
            let images = self
                .generated_images_for_video_prompt_references(project_id, chapter_id)
                .await?;
            return Ok(images
                .iter()
                .find(|image| {
                    let metadata = image.metadata_json.as_ref();
                    image_generation_prompt_id(metadata) == Some(first_frame_prompt.id)
                        && image_generation_source(metadata)
                            == Some(IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT)
                })
                .map(|image| image.id));
        }
        if recipe.guidance_mode == VideoSegmentGuidanceModeV1::Continuity {
            let source_video_id = recipe.continuity_source_video_id.ok_or_else(|| {
                invalid_argument("首帧续接提示词缺少已冻结的 continuity_source_video_id")
            })?;
            self.completed_chapter_continuity_source(project_id, chapter_id, source_video_id)
                .await?;
            let images = self
                .generated_images_for_video_prompt_references(project_id, chapter_id)
                .await?;
            return Ok(images
                .iter()
                .find(|image| tail_frame_reference_belongs_to_video(image, source_video_id))
                .map(|image| image.id));
        }
        Err(invalid_argument(
            "首帧续接的后续片段必须显式冻结 continuity 来源视频",
        ))
    }

    async fn storyboard_image_id_for_prompt(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt: &ChapterVideoPromptRecord,
    ) -> Result<Option<i64>, AppError> {
        let prompts = self
            .chapter_video_storyboard_prompt_candidates(project_id, chapter_id, prompt.shot_number)
            .await?;
        let Some(storyboard_prompt) = prompts
            .iter()
            .find(|candidate| storyboard_prompt_matches_video_prompt(prompt, candidate))
        else {
            return Ok(None);
        };
        let images = self
            .generated_images_for_video_prompt_references(project_id, chapter_id)
            .await?;
        Ok(latest_storyboard_image_id(&images, storyboard_prompt))
    }
}
