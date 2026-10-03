use super::*;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;

impl OrchestratorContext {
    pub(in crate::video) async fn generate_chapter_video_frame_prompts_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.generate_chapter_video_prompt_stage(
            context,
            input,
            PromptGenerationStage::Frame,
            progress,
            chunks,
            cancel_token,
        )
        .await
    }

    pub(in crate::video) async fn generate_chapter_video_segment_plan_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.generate_chapter_video_prompt_stage(
            context,
            input,
            PromptGenerationStage::Segment,
            progress,
            chunks,
            cancel_token,
        )
        .await
    }

    pub(in crate::video) async fn generate_chapter_video_prompts_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.generate_chapter_video_prompt_stage(
            context,
            input,
            PromptGenerationStage::Video,
            progress,
            chunks,
            cancel_token,
        )
        .await
    }

    async fn generate_chapter_video_prompt_stage(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        stage: PromptGenerationStage,
        mut progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        report_progress(&mut progress, 0, 100, "正在检查章节和生成参数")?;
        check_cancelled(&cancel_token)?;
        let spec = generation_spec(stage);
        let prepared = self
            .prepare_video_prompt_generation(context.runtime_video_config, input, stage, spec)
            .await?;
        let result = self
            .execute_video_prompt_generation_stage(
                context.prompt_renderer,
                &prepared,
                stage,
                &mut progress,
                chunks,
                &cancel_token,
            )
            .await;
        let rows = self
            .finalize_workflow_result(prepared.workflow_run_id, result, "视频提示词生成")
            .await?;
        Ok(rows.iter().map(ChapterVideoPromptSummary::from).collect())
    }

    async fn prepare_video_prompt_generation(
        &self,
        runtime_video_config: Option<&VideoProviderConfig>,
        input: GenerateChapterVideoPromptInput,
        stage: PromptGenerationStage,
        spec: GenerationSpec,
    ) -> Result<PreparedGeneration, AppError> {
        let GenerateChapterVideoPromptInput {
            user_id,
            project_id,
            chapter_id,
            active_task_id,
            root_project_id,
            retry_of_task_id,
            mut options,
        } = input;
        let active_task_id = active_task_id.ok_or_else(|| AppError::InvalidState {
            current: "video_prompt_generation_without_task_owner".to_string(),
            operation: "启动视频提示词工作流".to_string(),
        })?;
        let root_project_id = root_project_id.ok_or_else(|| AppError::InvalidState {
            current: "video_prompt_generation_without_root_project".to_string(),
            operation: "启动视频提示词工作流".to_string(),
        })?;
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let (source_bundle, segment_recipe) = futures::join!(
            self.project_chapter_video_source(project_id, chapter_id, spec.operation, true),
            self.resolve_segment_recipe_options(project_id, chapter_id, stage, &mut options),
        );
        let source_bundle = source_bundle?;
        if source_bundle.source.content.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                spec.empty_content_message.to_string(),
            ));
        }
        let segment_recipe = segment_recipe?;
        let workflow_mode = ensure_video_workflow_mode_for_stage(&options, stage)?;
        let mut options = validate_generate_prompt_options(options, &workflow_mode)?;
        options.target_shot_number = segment_recipe.as_ref().map(|recipe| recipe.segment_number);
        options.segment_recipe = segment_recipe;
        let (config, reference_access) = futures::join!(
            async {
                if workflow_mode != chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO {
                    self.effective_video_config(runtime_video_config)
                        .await
                        .map(Some)
                } else {
                    Ok::<Option<VideoProviderConfig>, AppError>(None)
                }
            },
            self.verify_video_reference_image_access(
                user_id,
                project_id,
                chapter_id,
                &options.reference_image_ids,
            ),
        );
        let config = config?;
        reference_access?;
        if let Some(config) = config {
            apply_duration_capability(&config, &mut options)?;
        }
        let variables = build_video_prompt_vars(
            project_id,
            chapter_id,
            &source_bundle.chapter,
            &source_bundle.source,
            &options,
        )?;
        let workflow_run = self
            .begin_video_prompt_workflow_validated(
                spec.workflow_kind,
                project_id,
                chapter_id,
                &workflow_mode,
                spec.workflow_step_key,
                &source_bundle.source,
                &options,
                &active_task_id,
                root_project_id,
                retry_of_task_id.as_deref(),
            )
            .await?;
        options.workflow_run_id = Some(workflow_run.id);
        Ok(PreparedGeneration {
            project_id,
            chapter_id,
            workflow_mode,
            source: source_bundle.source,
            variables,
            options,
            workflow_run_id: workflow_run.id,
        })
    }

    async fn resolve_segment_recipe_options(
        &self,
        project_id: i64,
        chapter_id: i64,
        stage: PromptGenerationStage,
        options: &mut VideoPromptGenerationOptions,
    ) -> Result<Option<VideoSegmentExecutionRecipeV1>, AppError> {
        if matches!(stage, PromptGenerationStage::Segment) {
            if options.segment_plan_prompt_id.is_some() || options.target_segment.is_some() {
                return Err(AppError::InvalidArgument(
                    "生成共享片段规划时不能指定片段规划 ID 或目标片段".to_string(),
                ));
            }
            return Ok(None);
        }
        if options.target_narrative_unit_index.is_some() {
            return Err(AppError::InvalidArgument(
                "target_narrative_unit_index 只能用于生成片段规划".to_string(),
            ));
        }
        let plan_id = options.segment_plan_prompt_id.ok_or_else(|| {
            AppError::InvalidArgument("片段卡片生成必须明确提供 segment_plan_prompt_id".to_string())
        })?;
        let plan = self
            .require_current_shared_segment_plan(project_id, chapter_id, plan_id)
            .await?;
        let plan_style = plan
            .style
            .as_deref()
            .map(str::trim)
            .filter(|style| !style.is_empty())
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少 style".to_string()))?;
        let target_segment = options.target_segment.as_ref().ok_or_else(|| {
            AppError::InvalidArgument("片段卡片生成必须明确提供稳定片段目标".to_string())
        })?;
        let target = resolve_video_segment_number(
            plan.metadata_json.as_ref(),
            &target_segment.source_shot_id,
            target_segment.segment_index_in_shot,
        )
        .map_err(AppError::InvalidArgument)?;
        let recipe =
            video_segment_recipes_for_plan_from_metadata(plan.metadata_json.as_ref(), plan_style)
                .map_err(AppError::InvalidArgument)?
                .into_iter()
                .find(|recipe| recipe.segment_number == target)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!("当前片段规划缺少片段 {target} 的执行方案"))
                })?;
        if recipe.workflow_mode == VideoSegmentWorkflowModeV1::ReferenceMedia {
            return Err(AppError::InvalidArgument(
                "参考媒体方案应直接使用参考媒体生成入口，不能生成普通帧或视频提示词".to_string(),
            ));
        }
        if matches!(stage, PromptGenerationStage::Frame)
            && recipe.workflow_mode.as_str() == chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO
        {
            return Err(AppError::InvalidArgument(
                "Chat 方案不需要帧提示词，请直接生成当前片段的最终提示词".to_string(),
            ));
        }
        if matches!(stage, PromptGenerationStage::Video) {
            recipe
                .validate_generation_ready()
                .map_err(AppError::InvalidArgument)?;
        }
        options.workflow_mode = Some(recipe.workflow_mode.as_str().to_string());
        options.style.clone_from(&recipe.style);
        options
            .prompt_output_language
            .clone_from(&recipe.prompt_output_language);
        options.prompt_batch_size = if matches!(stage, PromptGenerationStage::Frame)
            && recipe.workflow_mode.as_str() == chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES
        {
            2
        } else {
            1
        };
        options.duration_secs = recipe.duration_secs;
        options
            .duration_secs_options
            .clone_from(&recipe.duration_secs_options);
        options.aspect_ratio.clone_from(&recipe.aspect_ratio);
        options
            .reference_image_ids
            .clone_from(&recipe.reference_image_ids);
        options
            .video_library_image_ids
            .clone_from(&recipe.video_library_image_ids);
        Ok(Some(recipe))
    }

    async fn execute_video_prompt_generation_stage(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        prepared: &PreparedGeneration,
        stage: PromptGenerationStage,
        progress: &mut Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: &CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptRecord>, AppError> {
        report_progress(
            progress,
            10,
            100,
            generation_prepare_message(&prepared.workflow_mode, stage),
        )?;
        check_cancelled(cancel_token)?;
        let source = ChapterVideoSourceContext {
            content: &prepared.source.content,
            summary: prepared.source.summary.as_deref(),
            analysis: prepared.source.analysis.as_ref(),
        };
        let input = ChapterVideoPromptStageInput {
            project_id: prepared.project_id,
            chapter_id: prepared.chapter_id,
            workflow_mode: &prepared.workflow_mode,
            source,
            prompt_renderer,
            variables: &prepared.variables,
            options: prepared.options.clone(),
            cancel_token: Some(cancel_token),
        };
        match stage {
            PromptGenerationStage::Frame => {
                self.generate_chapter_frame_prompts_for_mode(input, chunks, progress.take())
                    .await
            }
            PromptGenerationStage::Segment => {
                self.generate_chapter_segment_plan(input, chunks, progress.take())
                    .await
            }
            PromptGenerationStage::Video => {
                self.generate_chapter_video_prompts_for_mode(input, chunks, progress.take())
                    .await
            }
        }
    }

    pub async fn list_chapter_video_prompts_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.project_chapter_video_source(project_id, chapter_id, "查看视频提示词", false)
            .await?;
        let mut result = Vec::new();
        for row in self
            .chapter_video_prompts_for_chapter(project_id, chapter_id)
            .await?
        {
            let row = a4n_db::video::read_models::with_video_plan_source_sync(
                self.database.connection(),
                row,
            )
            .await
            .map_err(video_db_error)?;
            result.push(ChapterVideoPromptSummary::from(&row));
        }
        Ok(result)
    }

    pub async fn load_video_segment_plan_draft_validated(
        &self,
        user_id: i64,
        input: LoadVideoSegmentPlanDraftInput,
    ) -> Result<VideoSegmentPlanDraftSummary, AppError> {
        let VideoSegmentPlanDraftContext { run, style } = self
            .video_segment_plan_draft_context(
                user_id,
                input.project_id,
                input.chapter_id,
                input.workflow_run_id,
                "加载片段规划草稿执行方案",
            )
            .await?;
        let checkpoints = load_video_segment_plan_draft_checkpoints(self, run.id).await?;
        let (segments, recipes, completed_shot_count, total_shot_count) = if checkpoints.is_empty()
        {
            (Vec::new(), Vec::new(), 0, 0)
        } else {
            load_video_segment_plan_draft_recipe_state(&checkpoints, &style)?
        };
        Ok(VideoSegmentPlanDraftSummary {
            run: workflow_run_summary_to_api(run),
            segments,
            recipes,
            completed_shot_count,
            total_shot_count,
        })
    }

    pub async fn update_video_segment_plan_draft_recipe_validated(
        &self,
        user_id: i64,
        input: UpdateVideoSegmentPlanDraftRecipeInput,
    ) -> Result<(), AppError> {
        let context = self
            .video_segment_plan_draft_context(
                user_id,
                input.project_id,
                input.chapter_id,
                input.workflow_run_id,
                "更新片段规划草稿执行方案",
            )
            .await?;
        update_video_segment_plan_draft_recipe_checkpoint(
            self,
            context.run.id,
            &input.source_shot_id,
            input.segment_index_in_shot,
            &context.style,
            input.recipe,
        )
        .await
    }

    async fn video_segment_plan_draft_context(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        workflow_run_id: i64,
        operation: &str,
    ) -> Result<VideoSegmentPlanDraftContext, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let run = self.workflow_run_summary(workflow_run_id).await?;
        if run.workflow_kind != "video_segment_plan"
            || run.project_id != Some(project_id)
            || run.chapter_id != Some(chapter_id)
            || run.mode.as_deref() != Some(chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN)
        {
            return Err(AppError::InvalidArgument(format!(
                "片段规划 workflow {workflow_run_id} 与当前项目或章节不一致"
            )));
        }
        if !workflow_run_is_running_or_resumable(&run.status, run.can_resume) {
            return Err(AppError::InvalidState {
                current: run.status.clone(),
                operation: operation.to_string(),
            });
        }
        let fingerprint = run
            .input_fingerprint_json
            .as_ref()
            .and_then(Value::as_object)
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_input_fingerprint_json".to_string(),
                operation: format!("读取片段规划 workflow {workflow_run_id}"),
            })?;
        if fingerprint.get("schema_version").and_then(Value::as_i64) != Some(1)
            || fingerprint.get("workflow_kind").and_then(Value::as_str)
                != Some("video_segment_plan")
            || fingerprint.get("project_id").and_then(Value::as_i64) != Some(project_id)
            || fingerprint.get("chapter_id").and_then(Value::as_i64) != Some(chapter_id)
            || fingerprint.get("workflow_mode").and_then(Value::as_str)
                != Some(chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN)
        {
            return Err(AppError::InvalidState {
                current: "invalid_input_fingerprint_json".to_string(),
                operation: format!("读取片段规划 workflow {workflow_run_id}"),
            });
        }
        let options = fingerprint
            .get("options")
            .and_then(Value::as_object)
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_input_fingerprint_options".to_string(),
                operation: format!("读取片段规划 workflow {workflow_run_id}"),
            })?;
        let style = options
            .get("style")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::InvalidState {
                current: "invalid_input_fingerprint_style".to_string(),
                operation: format!("读取片段规划 workflow {workflow_run_id}"),
            })?
            .to_string();
        Ok(VideoSegmentPlanDraftContext { run, style })
    }

    pub async fn update_chapter_video_prompt_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: UpdateChapterVideoPromptInput,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, AppError> {
        let prompt = self.chapter_video_prompt(prompt_id).await?;
        self.verify_project_permission(prompt.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        if input.is_empty() {
            return Err(AppError::InvalidArgument(
                "至少需要更新一个视频提示词字段".to_string(),
            ));
        }
        if input.duration_secs.is_some_and(|value| value <= 0) {
            return Err(AppError::InvalidArgument(
                "视频提示词 duration_secs 必须是正整数".to_string(),
            ));
        }
        if input
            .visual_prompt
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AppError::InvalidArgument(
                "视频提示词 visual_prompt 不能为空".to_string(),
            ));
        }
        let negative_prompt = input
            .negative_prompt
            .as_ref()
            .map(|value| value.as_deref())
            .unwrap_or(prompt.negative_prompt.as_deref());
        a4n_db::video::prompts::consistency::sync_chapter_video_prompt_metadata_after_update(
            &prompt.prompt_type,
            prompt.metadata_json.clone(),
            input
                .visual_prompt
                .as_deref()
                .unwrap_or(prompt.visual_prompt.as_str()),
            negative_prompt,
            input.duration_secs.unwrap_or(prompt.duration_secs),
            prompt.reference_image_ids_json.as_ref(),
        )
        .map_err(video_db_error)?;
        let (source_result, plan_result) = futures::join!(
            self.project_chapter_video_source(
                prompt.project_id,
                prompt.chapter_id,
                "更新视频提示词",
                true,
            ),
            self.prepare_mutation_plan(
                prompt_id,
                input.downstream_strategy,
                "更新视频提示词",
                "该视频提示词",
            ),
        );
        source_result?;
        let plan = plan_result?;
        self.apply_downstream_cleanup(cleanup, &plan).await?;
        let row = self
            .update_chapter_video_prompt_with_policy(
                prompt_id,
                ChapterVideoPromptPatch {
                    visual_prompt: input.visual_prompt,
                    motion_prompt: input.motion_prompt,
                    negative_prompt: input.negative_prompt,
                    duration_secs: input.duration_secs,
                    ..Default::default()
                },
                plan.allow_downstream,
            )
            .await?;
        Ok(ChapterVideoPromptSummary::from(&row))
    }

    pub async fn update_chapter_video_segment_plan_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: UpdateChapterVideoSegmentPlanInput,
    ) -> Result<ChapterVideoPromptSummary, AppError> {
        let prompt = self.chapter_video_prompt(prompt_id).await?;
        self.verify_project_permission(prompt.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.validate_segment_plan_edit_timestamp(&prompt, &input.expected_updated_at)?;
        let current = self
            .require_current_shared_segment_plan(prompt.project_id, prompt.chapter_id, prompt_id)
            .await?;
        if current.updated_at != prompt.updated_at {
            return Err(AppError::InvalidState {
                current: format!(
                    "segment_plan_updated_at_conflict:expected={},current={}",
                    prompt.updated_at, current.updated_at
                ),
                operation: "编辑片段规划".to_string(),
            });
        }
        let current_metadata = current.metadata_json.as_ref().ok_or_else(|| {
            AppError::InvalidArgument("当前片段规划缺少结构化 metadata_json，不能编辑".to_string())
        })?;
        let current_segments =
            a4n_video_contract::video_segment_plan_from_metadata(current_metadata)
                .map_err(AppError::InvalidArgument)?;
        a4n_video_contract::validate_video_segment_plan_identity(
            &current_metadata["video_execution_identity"],
            &current_segments,
        )
        .map_err(AppError::InvalidArgument)?;
        let mut segments = input
            .segments
            .into_iter()
            .map(validate_segment_plan)
            .collect::<Result<Vec<_>, _>>()?;
        validate_segment_plan_edit_identity(&current_segments, &segments)?;
        let changed_shots = segments
            .iter()
            .filter(|segment| {
                current_segments.iter().find(|old| {
                    old.source_shot_id == segment.source_shot_id
                        && old.segment_index_in_shot == segment.segment_index_in_shot
                }) != Some(*segment)
            })
            .map(|segment| segment.source_shot_id.clone())
            .collect::<std::collections::HashSet<_>>();
        if changed_shots.is_empty() {
            return Ok(ChapterVideoPromptSummary::from(&current));
        }

        let resolved = self
            .require_current_chapter_shooting_script(
                crate::shooting_script::ChapterShootingScriptSourceInput {
                    project_id: current.project_id,
                    source_chapter_id: current.chapter_id,
                    visual_plan_id: None,
                },
            )
            .await?;
        let script_record = &resolved.shooting_script.record;
        let confirmed = self
            .confirmed_visual_plan_for_shooting_script(&resolved.source, None)
            .await?;
        let director = crate::chapter_visual_plan::project_visual_plan_for_shooting_script(
            &confirmed.plan,
            &[],
        )?;
        let plan_sync = resolved.shooting_script.script.plan_sync(&director);
        if plan_sync
            .stale_shot_ids
            .iter()
            .any(|id| changed_shots.contains(id))
        {
            return Err(AppError::InvalidArgument(
                "请先更新目标镜头的拍摄剧本，再编辑其片段规划".into(),
            ));
        }
        let mut projection = video_projection_from_shooting_script(
            &resolved.shooting_script.script,
            VideoShootingScriptIdentityV1 {
                record_id: script_record.id,
                revision: script_record.revision,
                source_revision_key: script_record.source_revision_key.clone(),
                source_content_sha256: script_record.source_content_sha256.clone(),
                script_sha256: script_record.script_sha256.clone(),
                shot_fingerprints: resolved
                    .shooting_script
                    .script
                    .shot_content_fingerprints(&director)?,
            },
        )?;
        for segment in segments
            .iter_mut()
            .filter(|segment| changed_shots.contains(&segment.source_shot_id))
        {
            let shot = projection.shot(&segment.source_shot_id).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "当前拍摄剧本不存在镜头 {}",
                    segment.source_shot_id
                ))
            })?;
            let director = &shot.director_constraints;
            segment.director_constraints = Some(director.clone());
            segment.duration_secs = director
                .video_duration_secs()
                .map_err(AppError::InvalidArgument)?;
            segment.aspect_ratio = director
                .frame
                .aspect_ratio()
                .map_err(AppError::InvalidArgument)?;
        }
        let mut changed_projection = projection.clone();
        for scene in &mut changed_projection.scenes {
            scene
                .shots
                .retain(|shot| changed_shots.contains(&shot.source_shot_id));
        }
        let changed_segments = segments
            .iter()
            .filter(|segment| changed_shots.contains(&segment.source_shot_id))
            .cloned()
            .collect::<Vec<_>>();
        validate_segment_plan_against_shooting_script_projection(
            &changed_segments,
            &changed_projection,
        )?;
        let previous_source = serde_json::from_value(
            current_metadata["video_execution_identity"]["shooting_script_identity"].clone(),
        )?;
        projection
            .shooting_script_identity
            .carry_forward_sources(&previous_source, &changed_shots)
            .map_err(AppError::InvalidArgument)?;

        let style = current
            .style
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少 style".to_string()))?;
        let aspect_ratio = current
            .aspect_ratio
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::InvalidArgument("当前片段规划缺少 aspect_ratio".to_string())
            })?;
        let identity = video_execution_chain_identity(
            &projection,
            &segments,
            style,
            current.duration_secs,
            aspect_ratio,
        )?;
        let mut recipes =
            a4n_video_contract::video_segment_recipes_from_metadata(Some(current_metadata))
                .map_err(AppError::InvalidArgument)?;
        a4n_video_contract::validate_video_segment_recipes_for_plan(
            &recipes,
            &current_segments,
            style,
        )
        .map_err(AppError::InvalidArgument)?;
        let mut invalidated_adoptions = Vec::new();
        for ((before, after), recipe) in current_segments.iter().zip(&segments).zip(&mut recipes) {
            if before != after {
                recipe.duration_secs = after.duration_secs;
                recipe.duration_secs_options = vec![after.duration_secs];
                recipe.aspect_ratio = after.aspect_ratio.clone();
                invalidated_adoptions.push((recipe.segment_number, recipe.adopted_video_id.take()));
            }
        }
        for (number, adopted) in invalidated_adoptions {
            a4n_video_contract::propagate_adopted_video_change(&mut recipes, number, adopted, None);
        }
        a4n_video_contract::validate_video_segment_recipes_for_plan(&recipes, &segments, style)
            .map_err(AppError::InvalidArgument)?;
        let mut metadata = current_metadata.clone();
        let root = metadata.as_object_mut().ok_or_else(|| {
            AppError::InvalidArgument("当前片段规划 metadata_json 必须是对象".to_string())
        })?;
        root.insert("segments".to_string(), serde_json::to_value(&segments)?);
        root.insert(
            "segment_recipes".to_string(),
            serde_json::to_value(recipes)?,
        );
        root.insert("video_execution_identity".to_string(), identity);
        root.remove("stale_source_shot_ids");
        let row = self
            .update_chapter_video_segment_plan_metadata(prompt_id, prompt.updated_at, metadata)
            .await?;
        Ok(ChapterVideoPromptSummary::from(&row))
    }

    fn validate_segment_plan_edit_timestamp(
        &self,
        prompt: &ChapterVideoPromptRecord,
        expected_updated_at: &str,
    ) -> Result<(), AppError> {
        if expected_updated_at.trim().is_empty()
            || expected_updated_at != prompt.updated_at.to_string()
        {
            return Err(AppError::InvalidState {
                current: format!(
                    "segment_plan_updated_at_conflict:expected={expected_updated_at},current={}",
                    prompt.updated_at
                ),
                operation: "编辑片段规划".to_string(),
            });
        }
        Ok(())
    }

    pub(in crate::video) async fn current_shared_segment_plan(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterVideoPromptRecord, AppError> {
        let current_source_hash = self
            .current_video_source_hash(project_id, chapter_id)
            .await?;
        let prompt_id = a4n_db::video::completed_shared_segment_plan_id_for_source_hash(
            self.database.connection(),
            project_id,
            chapter_id,
            &current_source_hash,
        )
        .await
        .map_err(video_db_error)?
        .ok_or_else(|| AppError::InvalidArgument("当前章节没有可用的共享片段规划".to_string()))?;
        let row = self.chapter_video_prompt(prompt_id).await?;
        self.with_video_plan_source_sync(row).await
    }

    pub(in crate::video) async fn current_shared_segment_plan_optional(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Option<ChapterVideoPromptRecord>, AppError> {
        let current_source_hash = self
            .current_video_source_hash(project_id, chapter_id)
            .await?;
        let prompt_id = a4n_db::video::completed_shared_segment_plan_id_for_source_hash(
            self.database.connection(),
            project_id,
            chapter_id,
            &current_source_hash,
        )
        .await
        .map_err(video_db_error)?;
        match prompt_id {
            Some(prompt_id) => {
                let row = self.chapter_video_prompt(prompt_id).await?;
                self.with_video_plan_source_sync(row).await.map(Some)
            }
            None => Ok(None),
        }
    }

    pub(in crate::video) async fn require_current_shared_segment_plan(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
    ) -> Result<ChapterVideoPromptRecord, AppError> {
        let (prompt_result, current_source_hash_result) = futures::join!(
            self.chapter_video_prompt(prompt_id),
            self.current_video_source_hash(project_id, chapter_id),
        );
        let prompt = prompt_result?;
        self.validate_loaded_current_shared_segment_plan(
            &prompt,
            project_id,
            chapter_id,
            current_source_hash_result,
        )
        .await?;
        self.with_video_plan_source_sync(prompt).await
    }

    async fn with_video_plan_source_sync(
        &self,
        row: ChapterVideoPromptRecord,
    ) -> Result<ChapterVideoPromptRecord, AppError> {
        a4n_db::video::read_models::with_video_plan_source_sync(self.database.connection(), row)
            .await
            .map_err(video_db_error)
    }

    async fn validate_loaded_current_shared_segment_plan(
        &self,
        prompt: &ChapterVideoPromptRecord,
        project_id: i64,
        chapter_id: i64,
        current_source_hash_result: Result<String, AppError>,
    ) -> Result<(), AppError> {
        if prompt.project_id != project_id
            || prompt.chapter_id != chapter_id
            || prompt.prompt_type != chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
            || prompt.workflow_mode != chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
            || prompt.status != chapter_video_prompt::STATUS_COMPLETED
            || prompt
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("video_execution_identity"))
                .is_none()
        {
            return Err(AppError::InvalidArgument(
                "目标片段规划不是当前章节已完成的共享片段规划".to_string(),
            ));
        }
        let current_source_hash = current_source_hash_result?;
        if video_prompt_source_content_hash(prompt.metadata_json.as_ref())
            != Some(current_source_hash.as_str())
        {
            return Err(AppError::InvalidArgument(
                "目标片段规划对应的章节正文已变化，请重新生成片段规划".to_string(),
            ));
        }
        let current_prompt_id = a4n_db::video::completed_shared_segment_plan_id_for_source_hash(
            self.database.connection(),
            project_id,
            chapter_id,
            &current_source_hash,
        )
        .await
        .map_err(video_db_error)?;
        if current_prompt_id != Some(prompt.id) {
            return Err(AppError::InvalidArgument(
                "目标片段规划不是当前规划，请刷新后重试".to_string(),
            ));
        }
        Ok(())
    }

    async fn current_video_source_hash(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<String, AppError> {
        Ok(source_content_sha256(
            &self
                .project_chapter_video_source(project_id, chapter_id, "读取当前共享片段规划", false)
                .await?
                .source
                .content,
        ))
    }

    pub async fn update_chapter_video_segment_recipe_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        recipe: VideoSegmentExecutionRecipeV1,
    ) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError> {
        let (project_id, chapter_id) =
            a4n_db::video::chapter_video_prompt_scope_by_id(self.database.connection(), prompt_id)
                .await
                .map_err(video_db_error)?
                .ok_or_else(|| AppError::NotFound("视频提示词不存在".to_string()))?;
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let mut reference_ids = recipe.reference_image_ids.clone();
        reference_ids.extend(recipe.video_library_image_ids.iter().copied());
        reference_ids.extend(recipe.guidance_reference_image_ids.iter().copied());
        reference_ids.sort_unstable();
        reference_ids.dedup();
        self.verify_video_reference_image_access_json(
            user_id,
            project_id,
            chapter_id,
            Some(&json!(reference_ids)),
        )
        .await?;
        self.update_chapter_video_segment_recipe(prompt_id, recipe)
            .await
    }

    pub async fn delete_chapter_video_prompt_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        downstream_strategy: DownstreamConflictStrategy,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<(), AppError> {
        let prompt = self.chapter_video_prompt(prompt_id).await?;
        self.verify_project_permission(prompt.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let (source_result, plan_result) = futures::join!(
            self.project_chapter_video_source(
                prompt.project_id,
                prompt.chapter_id,
                "删除视频提示词",
                true,
            ),
            self.prepare_mutation_plan(
                prompt_id,
                downstream_strategy,
                "删除视频提示词",
                "该视频提示词",
            ),
        );
        source_result?;
        let plan = plan_result?;
        self.apply_downstream_cleanup(cleanup, &plan).await?;
        self.delete_chapter_video_prompts_checked(
            std::slice::from_ref(&prompt_id),
            plan.allow_downstream,
        )
        .await
        .map(|_| ())
    }

    pub async fn optimize_chapter_video_prompt_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: OptimizeChapterVideoPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, AppError> {
        let prompt = self.chapter_video_prompt(prompt_id).await?;
        self.verify_project_permission(prompt.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let plan = self
            .prepare_mutation_plan(
                prompt_id,
                input.downstream_strategy,
                "优化视频提示词",
                "该视频提示词",
            )
            .await?;
        self.optimize_video_prompt_validated(
            VideoPromptOptimizationInput {
                prompt,
                instruction: input.instruction,
                allow_downstream: plan.allow_downstream,
                delete_dependencies: plan.delete_dependencies,
            },
            prompt_renderer,
            cleanup,
        )
        .await
        .map(|row| ChapterVideoPromptSummary::from(&row))
    }

    async fn prepare_mutation_plan(
        &self,
        prompt_id: i64,
        strategy: DownstreamConflictStrategy,
        operation: &str,
        subject: &str,
    ) -> Result<VideoPromptMutationPlan, AppError> {
        let dependencies = self.prompt_downstream_dependencies(prompt_id).await?;
        mutation_plan_from_dependencies(dependencies, strategy, operation, subject)
    }

    async fn apply_downstream_cleanup(
        &self,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
        plan: &VideoPromptMutationPlan,
    ) -> Result<(), AppError> {
        let Some(dependencies) = plan.delete_dependencies.as_ref() else {
            return Ok(());
        };
        cleanup
            .delete_downstream_media(&dependencies.image_ids, &dependencies.video_ids)
            .await?;
        if !dependencies.prompt_ids.is_empty() {
            self.delete_chapter_video_prompts_checked(&dependencies.prompt_ids, true)
                .await?;
        }
        Ok(())
    }

    async fn effective_video_config(
        &self,
        runtime_video_config: Option<&VideoProviderConfig>,
    ) -> Result<VideoProviderConfig, AppError> {
        match runtime_video_config {
            Some(config) => Ok(config.clone()),
            None => {
                let config = self.default_video_config().await?;
                super::super::super::video_provider_config_from_record(&config).map_err(|error| {
                    AppError::InvalidState {
                        current: error,
                        operation: "读取视频 Provider 价格配置".to_string(),
                    }
                })
            }
        }
    }

    async fn verify_video_reference_image_access(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids: &[i64],
    ) -> Result<(), AppError> {
        if reference_image_ids.is_empty() {
            return Ok(());
        }
        let ids = json!(reference_image_ids);
        self.verify_video_reference_image_access_json(user_id, project_id, chapter_id, Some(&ids))
            .await
    }

    async fn verify_video_reference_image_access_json(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids: Option<&Value>,
    ) -> Result<(), AppError> {
        let project_ids = self
            .validated_video_reference_images(project_id, Some(chapter_id), reference_image_ids)
            .await?
            .into_iter()
            .map(|image| image.project_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if project_ids.is_empty() {
            return Ok(());
        }
        self.verify_project_permissions(&project_ids, user_id, ProjectAccessRole::Editor)
            .await
    }
}
