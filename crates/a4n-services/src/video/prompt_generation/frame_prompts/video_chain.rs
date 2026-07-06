use super::*;

fn build_video_prompt_user_for_spec(
    spec: &FramePromptGenerationSpec,
    context: FinalVideoPromptContext<'_>,
) -> Result<String, AppError> {
    match spec.workflow_mode {
        PromptWorkflowMode::BoundaryFrames => build_boundary_video_prompt_user(context),
        PromptWorkflowMode::FirstFrameChain => build_first_frame_video_prompt_user(context),
        PromptWorkflowMode::ChatVideo => build_chat_video_prompt_user(context),
        PromptWorkflowMode::Storyboard => build_storyboard_video_prompt_user(context),
    }
}

fn build_video_prompt_review_user_for_spec(
    spec: &FramePromptGenerationSpec,
    context: FinalVideoPromptContext<'_>,
    draft_prompt_json: &Value,
) -> Result<String, AppError> {
    match spec.workflow_mode {
        PromptWorkflowMode::BoundaryFrames => {
            build_boundary_video_prompt_review_user(context, draft_prompt_json)
        }
        PromptWorkflowMode::FirstFrameChain => {
            build_first_frame_video_prompt_review_user(context, draft_prompt_json)
        }
        PromptWorkflowMode::ChatVideo => {
            build_chat_video_prompt_review_user(context, draft_prompt_json)
        }
        PromptWorkflowMode::Storyboard => {
            build_storyboard_video_prompt_review_user(context, draft_prompt_json)
        }
    }
}

struct VideoPromptDraftsForSpecInput<'a> {
    spec: &'a FramePromptGenerationSpec,
    project_id: i64,
    chapter_id: i64,
    style: &'a str,
    prompt_output_language: PromptOutputLanguage,
    source_metadata: &'a Value,
    segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    video_unit_by_number: &'a HashMap<i32, VideoUnitPlan>,
    frame_by_number: &'a HashMap<i32, VideoFramePrompt>,
    available_assets: &'a Value,
    generation_reference_ids: &'a Value,
    generation_asset_fingerprint: &'a Value,
    video_prompts: &'a [GeneratedVideoPromptShot],
    preserved_generated_reused_shots: &'a HashSet<i32>,
}

fn video_prompt_drafts_for_spec(
    input: VideoPromptDraftsForSpecInput<'_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let VideoPromptDraftsForSpecInput {
        spec,
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        segment_by_number,
        video_unit_by_number,
        frame_by_number,
        available_assets,
        generation_reference_ids,
        generation_asset_fingerprint,
        video_prompts,
        preserved_generated_reused_shots,
    } = input;
    let writable_video_prompts =
        writable_generated_video_prompts(video_prompts, preserved_generated_reused_shots);
    let common = VideoPromptDraftCommonInput {
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        available_assets,
        generation_asset_fingerprint,
    };
    match spec.workflow_mode {
        PromptWorkflowMode::BoundaryFrames => video_prompt_drafts(VideoPromptDraftsInput {
            common,
            segment_by_number,
            video_unit_by_number,
            frame_by_number,
            video_prompts: &api_generated_video_prompts(
                &writable_video_prompts,
                spec.video_prompt_label,
            )?,
        }),
        PromptWorkflowMode::FirstFrameChain => {
            first_frame_video_prompt_drafts(FirstFrameVideoPromptDraftsInput {
                common,
                source_prompt_type: spec.frame_source_prompt_type(),
                segment_by_number,
                video_unit_by_number,
                first_frame_by_number: frame_by_number,
                video_prompts: &api_generated_video_prompts(
                    &writable_video_prompts,
                    spec.video_prompt_label,
                )?,
            })
        }
        PromptWorkflowMode::ChatVideo => {
            let writable_prompts =
                chat_generated_video_prompts(&writable_video_prompts, spec.video_prompt_label)?;
            chat_video_prompt_drafts(ChatVideoPromptDraftsInput {
                common,
                source_prompt_type: spec.segment_storage_prompt_type(),
                segment_by_number,
                video_unit_by_number,
                generation_reference_ids,
                video_prompts: &writable_prompts,
            })
        }
        PromptWorkflowMode::Storyboard => {
            storyboard_video_prompt_drafts(StoryboardVideoPromptDraftsInput {
                common,
                segment_by_number,
                video_unit_by_number,
                storyboard_by_number: frame_by_number,
                video_prompts: &api_generated_video_prompts(
                    &writable_video_prompts,
                    spec.video_prompt_label,
                )?,
            })
        }
    }
}

fn writable_generated_video_prompts(
    prompts: &[GeneratedVideoPromptShot],
    preserved_generated_reused_shots: &HashSet<i32>,
) -> Vec<GeneratedVideoPromptShot> {
    prompts
        .iter()
        .filter(|prompt| !preserved_generated_reused_shots.contains(&prompt.shot_number()))
        .cloned()
        .collect()
}

fn select_boundary_video_prompt_inputs(
    frame_rows: Vec<chapter_video_prompt::Model>,
    segment_plan: &[VideoSegmentPlan],
    duration_secs: i32,
    allowed_reference_ids: &HashSet<i64>,
    label: &str,
) -> Result<(Vec<VideoFramePrompt>, Vec<VideoUnitPlan>), AppError> {
    let actual_frame_count = frame_rows.len();
    let prompts = frame_prompts_from_rows(&frame_rows)?;
    let prompts = validate_frame_prompts_for_video_prompt_inputs(
        prompts,
        segment_plan,
        allowed_reference_ids,
        label,
    )?;
    let frame_by_number = frame_rows
        .iter()
        .zip(prompts.iter())
        .map(|(row, frame)| {
            if row.shot_number != frame.frame_number {
                return Err(AppError::InvalidArgument(format!(
                    "已保存{label} #{} 的 frame_number 必须等于存储编号，实际为 {}",
                    row.shot_number, frame.frame_number
                )));
            }
            Ok((frame.frame_number, frame.clone()))
        })
        .collect::<Result<HashMap<_, _>, AppError>>()?;

    let units = video_units_for_segment_plan(segment_plan, duration_secs);
    let mut selected_units = Vec::new();
    let mut selected_frame_numbers = BTreeSet::new();
    for unit in units {
        let Some(start_frame_number) = unit.start_frame_number else {
            continue;
        };
        let Some(end_frame_number) = unit.end_frame_number else {
            continue;
        };
        let Some(start_frame) = frame_by_number.get(&start_frame_number) else {
            continue;
        };
        let Some(end_frame) = frame_by_number.get(&end_frame_number) else {
            continue;
        };
        let start_segment = unit.source_segments.first().ok_or_else(|| {
            AppError::InvalidArgument(format!("视频单元 #{} 缺少起始片段", unit.unit_number))
        })?;
        let end_segment = unit.source_segments.last().ok_or_else(|| {
            AppError::InvalidArgument(format!("视频单元 #{} 缺少结束片段", unit.unit_number))
        })?;
        validate_boundary_frame_for_video_segment(
            start_frame,
            start_segment,
            allowed_reference_ids,
            label,
        )?;
        validate_boundary_frame_for_video_segment(
            end_frame,
            end_segment,
            allowed_reference_ids,
            label,
        )?;
        selected_units.push(unit);
        selected_frame_numbers.insert(start_frame_number);
        selected_frame_numbers.insert(end_frame_number);
    }

    if selected_units.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "当前章节{label}不足以生成视频提示词：需要至少一组连续边界帧，当前已有 {actual_frame_count} 条"
        )));
    }

    let selected_frames = prompts
        .into_iter()
        .filter(|frame| selected_frame_numbers.contains(&frame.frame_number))
        .collect::<Vec<_>>();
    Ok((selected_frames, selected_units))
}

fn validate_boundary_frame_for_video_segment(
    frame: &VideoFramePrompt,
    segment: &VideoSegmentPlan,
    allowed_reference_ids: &HashSet<i64>,
    label: &str,
) -> Result<(), AppError> {
    if !frame.segment_numbers.contains(&segment.segment_number) {
        return Err(AppError::InvalidArgument(format!(
            "已保存{label} #{} 未声明关联片段 {}，不能作为该段视频提示词输入",
            frame.frame_number, segment.segment_number
        )));
    }
    ensure_video_reference_ids_known(
        &frame.reference_image_ids,
        allowed_reference_ids,
        &format!("{label} #{}", frame.frame_number),
    )?;
    Ok(())
}

struct ChapterVideoPromptChainInput<'a, 'source> {
    service: &'a VideoService,
    spec: &'a FramePromptGenerationSpec,
    project_id: i64,
    chapter_id: i64,
    source: ChapterVideoSourceContext<'source>,
    llm: &'a dyn LlmProvider,
    prompt_engine: &'a dyn PromptEngine,
    variables: &'a HashMap<String, Value>,
    options: ChapterVideoPromptOptions,
    cancel_token: Option<&'a CancellationToken>,
}

fn frame_prompt_chain_input<'a, 'source>(
    service: &'a VideoService,
    spec: &'a FramePromptGenerationSpec,
    input: ChapterVideoPromptStageInput<'a, 'source>,
) -> FramePromptChainInput<'a, 'source> {
    let ChapterVideoPromptStageInput {
        project_id,
        chapter_id,
        workflow_mode: _,
        source,
        llm,
        prompt_engine,
        variables,
        options,
        cancel_token,
    } = input;
    FramePromptChainInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        llm,
        prompt_engine,
        variables,
        options,
        cancel_token,
    }
}

fn video_prompt_chain_input<'a, 'source>(
    service: &'a VideoService,
    spec: &'a FramePromptGenerationSpec,
    input: ChapterVideoPromptStageInput<'a, 'source>,
) -> ChapterVideoPromptChainInput<'a, 'source> {
    let ChapterVideoPromptStageInput {
        project_id,
        chapter_id,
        workflow_mode: _,
        source,
        llm,
        prompt_engine,
        variables,
        options,
        cancel_token,
    } = input;
    ChapterVideoPromptChainInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        llm,
        prompt_engine,
        variables,
        options,
        cancel_token,
    }
}

async fn generate_chapter_video_prompt_chain(
    input: ChapterVideoPromptChainInput<'_, '_>,
    mut chunks: Option<VideoPromptChunk<'_>>,
    mut progress: Option<VideoPromptProgress<'_>>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let ChapterVideoPromptChainInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        llm,
        prompt_engine,
        variables,
        options,
        cancel_token,
    } = input;
    let is_chat_video = spec.is_chat_video();
    let workflow_run_id = options.workflow_run_id;
    let PreparedVideoPromptInputs {
        source_metadata,
        source_spans,
        regenerate_existing,
        prompt_batch_size,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_image_ids,
        video_library_image_ids,
        available_assets,
        vars,
        ..
    } = service
        .prepare_video_prompt_inputs(project_id, chapter_id, &source, variables, options, false)
        .await?;
    let segment_storage_prompt_type = spec.segment_storage_prompt_type();
    let frame_source_prompt_type = spec.frame_source_prompt_type();

    report_video_prompt_progress(&mut progress, 20, 100, spec.read_video_inputs_progress)?;
    let reusable_segment = reusable_segment_plan_for_spec(ReusableSegmentPlanForSpecQuery {
        service,
        project_id,
        chapter_id,
        spec,
        source_metadata: &source_metadata,
        source_spans: &source_spans,
        style: &style,
        aspect_ratio: &aspect_ratio,
    })
    .await?
    .ok_or_else(|| AppError::InvalidArgument(spec.missing_video_inputs_message.to_string()))?;
    let (_segment_json, segment_plan) =
        (reusable_segment.segment_json, reusable_segment.segment_plan);
    let mut allowed_reference_ids =
        available_video_image_ids(&available_assets, &reference_image_ids)?;

    let frame_rows = reusable_frame_rows_for_video_prompt_inputs(
        service,
        project_id,
        chapter_id,
        spec.workflow_mode,
        spec.frame_prompt_type,
        Some(frame_source_prompt_type),
        prompt_output_language,
    )
    .await?;
    let frame_rows = if spec.is_first_frame_chain() {
        initial_first_frame_rows(frame_rows)
    } else {
        frame_rows
    };
    let frame_image_ids_by_number = if matches!(
        spec.workflow_mode,
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo
    ) {
        boundary_frame_image_ids_by_number(service, project_id, chapter_id, &frame_rows).await?
    } else if spec.workflow_mode == PromptWorkflowMode::Storyboard {
        storyboard_frame_image_ids_by_number(service, project_id, chapter_id, &frame_rows).await?
    } else {
        HashMap::new()
    };
    allowed_reference_ids.extend(frame_image_ids_by_number.values().copied());
    let all_video_units = video_units_for_segment_plan(&segment_plan, duration_secs);
    if spec.is_storyboard() {
        let missing = all_video_units
            .iter()
            .filter(|unit| {
                unit.storyboard_number
                    .is_none_or(|number| !frame_image_ids_by_number.contains_key(&number))
            })
            .map(|unit| unit.unit_number.to_string())
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "当前章节分镜板不完整：缺少第 {} 个视频单元分镜板，请先在分镜生成里生成对应图片",
                missing.join("、")
            )));
        }
    }
    let (frame_prompts, prompt_video_units) = if spec.is_first_frame_chain() {
        let expected_frame_count = FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT;
        let actual_frame_count = frame_rows.len();
        let Some(frame_rows) = complete_reusable_rows(frame_rows, expected_frame_count) else {
            return Err(AppError::InvalidArgument(format!(
                "当前章节{}不完整：需要 {} 条，当前 {} 条，请先生成完整{}",
                spec.incomplete_frame_label,
                expected_frame_count,
                actual_frame_count,
                spec.incomplete_frame_label
            )));
        };
        let prompts = first_frame_prompts_from_rows(&frame_rows)?;
        let prompts = validate_frame_prompts_for_video_prompt_inputs(
            prompts,
            &segment_plan,
            &allowed_reference_ids,
            spec.incomplete_frame_label,
        )?;
        validate_first_frame_prompts(&prompts, &segment_plan, &allowed_reference_ids)?;
        (prompts, all_video_units.clone())
    } else if matches!(
        spec.workflow_mode,
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo
    ) {
        select_boundary_video_prompt_inputs(
            frame_rows,
            &segment_plan,
            duration_secs,
            &allowed_reference_ids,
            spec.incomplete_frame_label,
        )?
    } else if spec.is_storyboard() {
        let expected_frame_count = expected_video_unit_count(&segment_plan);
        let actual_frame_count = frame_rows.len();
        let Some(frame_rows) = complete_reusable_rows(frame_rows, expected_frame_count) else {
            return Err(AppError::InvalidArgument(format!(
                "当前章节{}不完整：需要 {} 条，当前 {} 条，请先生成完整{}",
                spec.incomplete_frame_label,
                expected_frame_count,
                actual_frame_count,
                spec.incomplete_frame_label
            )));
        };
        let prompts = storyboard_prompts_from_rows(&frame_rows)?;
        validate_storyboard_prompts(&prompts, &segment_plan, &allowed_reference_ids)?;
        (prompts, all_video_units.clone())
    } else {
        let expected_frame_count = expected_boundary_frame_count(&segment_plan);
        let actual_frame_count = frame_rows.len();
        let Some(frame_rows) = complete_reusable_rows(frame_rows, expected_frame_count) else {
            return Err(AppError::InvalidArgument(format!(
                "当前章节{}不完整：需要 {} 条，当前 {} 条，请先生成完整{}",
                spec.incomplete_frame_label,
                expected_frame_count,
                actual_frame_count,
                spec.incomplete_frame_label
            )));
        };
        let prompts = frame_prompts_from_rows(&frame_rows)?;
        let prompts = validate_frame_prompts_for_video_prompt_inputs(
            prompts,
            &segment_plan,
            &allowed_reference_ids,
            spec.incomplete_frame_label,
        )?;
        validate_frame_prompts(&prompts, &segment_plan, &allowed_reference_ids)?;
        (prompts, all_video_units.clone())
    };
    let frame_json = json!({ "frames": frame_prompts });
    let frame_by_number = frame_prompts
        .iter()
        .map(|frame| (frame.frame_number, frame.clone()))
        .collect::<HashMap<_, _>>();
    let video_segment_plan = video_prompt_unit_segments_with_reference_ids(
        spec.workflow_mode,
        &prompt_video_units,
        &frame_by_number,
        &frame_image_ids_by_number,
        &reference_image_ids,
    )?;
    let reference_image_ids = video_prompt_workflow_reference_ids(&video_segment_plan);
    let reference_ids = json!(reference_image_ids.clone());
    let available_assets = service
        .video_library_image_context(
            project_id,
            chapter_id,
            &source,
            &reference_image_ids,
            &video_library_image_ids,
        )
        .await?;
    let generation_asset_fingerprint = video_generation_asset_fingerprint(
        &reference_image_ids,
        &video_library_image_ids,
        &available_assets,
    );
    let mut vars = vars;
    vars.insert("reference_image_ids".into(), reference_ids.clone());
    vars.insert("available_video_assets".into(), available_assets.clone());
    let allowed_reference_ids = available_video_image_ids(&available_assets, &reference_image_ids)?;
    if matches!(
        spec.workflow_mode,
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo
    ) && video_segment_plan.len() < all_video_units.len()
    {
        report_video_prompt_progress(
            &mut progress,
            45,
            100,
            &format!(
                "已找到 {} 条可用边界帧，可生成 {}/{} 个视频单元提示词",
                frame_prompts.len(),
                video_segment_plan.len(),
                all_video_units.len()
            ),
        )?;
    }
    for segment in &video_segment_plan {
        ensure_video_reference_ids_known(
            &segment.reference_image_ids,
            &allowed_reference_ids,
            &format!("{} #{}", spec.video_prompt_label, segment.segment_number),
        )?;
    }
    let video_segment_json = json!({ "segments": &video_segment_plan });
    let segment_by_number = video_segment_plan
        .iter()
        .map(|segment| (segment.segment_number, segment))
        .collect::<HashMap<_, _>>();
    let video_unit_by_number = prompt_video_units
        .iter()
        .map(|unit| (unit.unit_number, unit.clone()))
        .collect::<HashMap<_, _>>();

    let matching_video_prompts = service
        .matching_prompt_rows_for_workflow(
            PromptWorkflowRowsContext {
                project_id,
                chapter_id,
                workflow_mode: spec.workflow_mode.as_str(),
                prompt_type: "video_prompt",
                source_metadata: &source_metadata,
                style: &style,
                duration_secs,
                aspect_ratio: &aspect_ratio,
                reference_ids: &reference_ids,
                generation_asset_fingerprint: Some(&generation_asset_fingerprint),
                source_prompt_type: Some(segment_storage_prompt_type),
            },
            prompt_output_language,
        )
        .await?;
    let matching_video_prompts_any_language = service
        .matching_prompt_rows_for_workflow_any_language(PromptWorkflowRowsContext {
            project_id,
            chapter_id,
            workflow_mode: spec.workflow_mode.as_str(),
            prompt_type: "video_prompt",
            source_metadata: &source_metadata,
            style: &style,
            duration_secs,
            aspect_ratio: &aspect_ratio,
            reference_ids: &reference_ids,
            generation_asset_fingerprint: Some(&generation_asset_fingerprint),
            source_prompt_type: Some(segment_storage_prompt_type),
        })
        .await?;
    let all_video_prompts =
        video_prompt_rows_for_workflow(service, project_id, chapter_id, spec.workflow_mode)
            .await?
            .into_iter()
            .filter(|row| {
                metadata_matches_prompt_output_language(
                    row.metadata_json.as_ref(),
                    prompt_output_language,
                )
            })
            .collect::<Vec<_>>();
    let all_chat_video_prompts = if is_chat_video {
        all_video_prompts.clone()
    } else {
        Vec::new()
    };
    let preflight_rows = if is_chat_video {
        &all_chat_video_prompts
    } else {
        &matching_video_prompts
    };
    if preflight_rows.iter().any(|row| row.status == "running") {
        return Err(AppError::InvalidArgument(format!(
            "当前章节已有{}正在运行，不能重新生成视频提示词",
            spec.video_prompt_label
        )));
    }
    if regenerate_existing {
        ensure_video_prompts_can_regenerate(preflight_rows, spec.video_prompt_label)?;
    }

    let generation_limit = prompt_batch_size;
    let reusable_video_prompts =
        reusable_generated_video_prompts_from_rows(spec, &matching_video_prompts)?;
    let reusable_context_video_prompts =
        reusable_generated_video_prompts_from_rows(spec, &matching_video_prompts_any_language)?;
    let existing_generated_video_prompt_rows = all_video_prompts
        .iter()
        .filter(|row| row.generated_video_id.is_some())
        .cloned()
        .collect::<Vec<_>>();
    let existing_generated_video_prompts =
        reusable_generated_video_prompts_from_rows(spec, &existing_generated_video_prompt_rows)?;
    let matching_video_prompt_by_shot = matching_video_prompts.iter().fold(
        HashMap::<i32, &chapter_video_prompt::Model>::new(),
        |mut rows, row| {
            rows.entry(row.shot_number).or_insert(row);
            rows
        },
    );
    if is_chat_video {
        let reusable_chat_video_prompts =
            chat_generated_video_prompt_map(&reusable_video_prompts, spec.video_prompt_label)?;
        ensure_chat_video_prompts_can_generate_without_overwriting(
            &all_chat_video_prompts,
            &matching_video_prompts,
            &reusable_chat_video_prompts,
            &video_segment_plan,
            regenerate_existing,
            generation_limit,
        )?;
    }
    let mut video_prompts =
        Vec::<GeneratedVideoPromptShot>::with_capacity(video_segment_plan.len());
    let mut context_video_prompts =
        Vec::<GeneratedVideoPromptShot>::with_capacity(video_segment_plan.len());
    let mut preserved_generated_reused_shots = HashSet::<i32>::new();
    let mut generated_video_prompt_traces = HashMap::<i32, VideoPromptGenerationTrace>::new();
    let mut generated_this_run = 0usize;
    let mut reuse_prefix = true;

    report_video_prompt_progress(&mut progress, 60, 100, spec.generate_video_progress)?;
    for (index, segment) in video_segment_plan.iter().enumerate() {
        let current_index = index + 1;
        let total_segments = video_segment_plan.len();
        let shot_number = segment.segment_number;

        if spec.is_first_frame_chain() && generated_this_run >= generation_limit {
            if let Some(reusable) = reusable_video_prompts.get(&shot_number) {
                video_prompts.push(reusable.clone());
                context_video_prompts.push(reusable.clone());
                report_video_prompt_progress(
                    &mut progress,
                    ProgressSpan::new(60, 90).split(index, total_segments).end(),
                    100,
                    &format!(
                        "已保留{}全章第 {current_index}/{total_segments} 段，本批已达到 {generation_limit} 条",
                        spec.video_prompt_label
                    ),
                )?;
                continue;
            }
            if let Some(reusable) = reusable_context_video_prompts.get(&shot_number) {
                context_video_prompts.push(reusable.clone());
                report_video_prompt_progress(
                    &mut progress,
                    ProgressSpan::new(60, 90).split(index, total_segments).end(),
                    100,
                    &format!(
                        "已跳过已存在{}全章第 {current_index}/{total_segments} 段，本批已达到 {generation_limit} 条",
                        spec.video_prompt_label
                    ),
                )?;
                continue;
            }
            break;
        }
        if !regenerate_existing
            && let Some(existing_generated) = existing_generated_video_prompts.get(&shot_number)
        {
            preserved_generated_reused_shots.insert(shot_number);
            video_prompts.push(existing_generated.clone());
            context_video_prompts.push(existing_generated.clone());
            report_video_prompt_progress(
                &mut progress,
                ProgressSpan::new(60, 90).split(index, total_segments).end(),
                100,
                &format!(
                    "已保留已生成视频的{}全章第 {current_index}/{total_segments} 段，继续生成后续段",
                    spec.video_prompt_label
                ),
            )?;
            continue;
        }
        if let Some(reusable) = reusable_video_prompts.get(&shot_number) {
            let can_reuse = if spec.is_first_frame_chain() {
                reuse_prefix && (!regenerate_existing || generated_this_run >= generation_limit)
            } else {
                !regenerate_existing || generated_this_run >= generation_limit
            };
            if can_reuse {
                if matching_video_prompt_by_shot
                    .get(&shot_number)
                    .is_some_and(|row| row.generated_video_id.is_some())
                {
                    preserved_generated_reused_shots.insert(shot_number);
                }
                video_prompts.push(reusable.clone());
                context_video_prompts.push(reusable.clone());
                let reuse_message = if regenerate_existing {
                    format!(
                        "已保留{}全章第 {current_index}/{total_segments} 段，本批已达到 {generation_limit} 条",
                        spec.video_prompt_label
                    )
                } else {
                    format!(
                        "已复用{}全章第 {current_index}/{total_segments} 段",
                        spec.video_prompt_label
                    )
                };
                report_video_prompt_progress(
                    &mut progress,
                    ProgressSpan::new(60, 90).split(index, total_segments).end(),
                    100,
                    &reuse_message,
                )?;
                continue;
            }
        }
        if !regenerate_existing
            && let Some(reusable) = reusable_context_video_prompts.get(&shot_number)
        {
            context_video_prompts.push(reusable.clone());
            report_video_prompt_progress(
                &mut progress,
                ProgressSpan::new(60, 90).split(index, total_segments).end(),
                100,
                &format!(
                    "已跳过已存在{}全章第 {current_index}/{total_segments} 段，继续生成后续段",
                    spec.video_prompt_label
                ),
            )?;
            continue;
        }
        if spec.is_first_frame_chain() {
            reuse_prefix = false;
        }
        if generated_this_run >= generation_limit {
            continue;
        }

        ensure_video_prompt_generation_can_start(
            service,
            project_id,
            chapter_id,
            spec.workflow_mode,
            shot_number,
            spec.video_prompt_label,
            prompt_output_language,
        )
        .await?;
        let current_segment_json = json!(segment);
        let previous_shot_json = context_video_prompts
            .last()
            .map(GeneratedVideoPromptShot::to_json_value)
            .transpose()?;
        let generation_message = format!(
            "正在生成{}全章第 {} 段（共 {} 段），本批第 {}/{} 条",
            spec.video_prompt_label,
            current_index,
            total_segments,
            generated_this_run + 1,
            generation_limit
        );
        let video_progress_span = ProgressSpan::new(60, 90).split(index, total_segments);
        let final_prompt_context = FinalVideoPromptContext {
            prompt_engine,
            project_id,
            chapter_summary: source.summary,
            chapter_analysis: source.analysis,
            segment_plan: &video_segment_json,
            frame_prompts: &frame_json,
            current_segment: &current_segment_json,
            previous_completed_shot: previous_shot_json.as_ref(),
            available_assets: &available_assets,
            style: &style,
            prompt_output_language,
            duration_secs,
            aspect_ratio: &aspect_ratio,
            reference_ids: &reference_ids,
        };
        let prompt_user = build_video_prompt_user_for_spec(spec, final_prompt_context)?;
        let prompt_context_manifest = build_final_video_context_manifest(
            prompt_engine,
            project_id,
            &vars,
            &source,
            &[
                "chapter_video_segment_plan",
                spec.frame_prompt_name,
                spec.video_prompt_name,
            ],
            prompt_user.clone(),
        )?;
        let draft_step_key = format!("{}:shot:{shot_number}:draft", spec.video_prompt_name);
        let draft_fingerprint_json = video_stage_fingerprint(
            spec.video_prompt_name,
            &prompt_context_manifest,
            json!({
                "project_id": project_id,
                "chapter_id": chapter_id,
                "shot_number": shot_number,
                "prompt_type": "video_prompt_draft",
                "output_contract_version": FINAL_VIDEO_PROMPT_OUTPUT_CONTRACT_VERSION,
            }),
        );
        let draft_workflow_run_id =
            required_llm_call_workflow_run_id(workflow_run_id, &draft_step_key)?;
        let (_, draft_prompt_json) = render_or_restore_json_with_validation_with_context(
            RenderOrRestoreJsonWithValidationInput {
                service,
                workflow_run_id: draft_workflow_run_id,
                call_key: draft_step_key,
                call_order: 400 + shot_number * 2 - 1,
                input_fingerprint_json: draft_fingerprint_json,
                prompt_engine,
                llm,
                prompt_name: spec.video_prompt_name,
                project_id,
                variables: &vars,
                context_manifest: prompt_context_manifest,
                temperature: 0.45,
                cancel_token,
                step: VideoPromptStepProgress::new(
                    video_progress_span.first_half().start(),
                    100,
                    generation_message.as_str(),
                )
                .with_span(video_progress_span.first_half()),
                progress: &mut progress,
                chunks: &mut chunks,
                structured_progress: Some(video_prompt_progress_spec(is_chat_video)),
                checkpoint_context: "视频提示词初稿断点",
            },
            |raw| {
                let prompt_json = parse_video_step_object(raw, spec.video_prompt_name)?;
                parse_single_generated_video_prompt_shot(
                    spec,
                    raw,
                    &segment_by_number,
                    prompt_output_language,
                    shot_number,
                    &allowed_reference_ids,
                    &format!("{}初稿", spec.video_prompt_label),
                )?;
                Ok(prompt_json)
            },
            |prompt_json| Ok(prompt_json.clone()),
        )
        .await?;

        let review_message = format!(
            "{}全章第 {} 段初稿已生成，正在自检修订（本批第 {}/{} 条）",
            spec.video_prompt_label,
            current_index,
            generated_this_run + 1,
            generation_limit
        );
        report_video_prompt_progress(
            &mut progress,
            video_progress_span.first_half().end(),
            100,
            &review_message,
        )?;
        let prompt_review_user = build_video_prompt_review_user_for_spec(
            spec,
            final_prompt_context,
            &draft_prompt_json,
        )?;
        let review_context_manifest = build_final_video_context_manifest(
            prompt_engine,
            project_id,
            &vars,
            &source,
            &[
                "chapter_video_segment_plan",
                spec.frame_prompt_name,
                spec.video_prompt_name,
                spec.video_prompt_review_name,
            ],
            prompt_review_user.clone(),
        )?;
        let review_step_key = format!(
            "{}:shot:{shot_number}:review",
            spec.video_prompt_review_name
        );
        let review_fingerprint_json = video_stage_fingerprint(
            spec.video_prompt_review_name,
            &review_context_manifest,
            json!({
                "project_id": project_id,
                "chapter_id": chapter_id,
                "shot_number": shot_number,
                "prompt_type": "video_prompt_review",
                "output_contract_version": FINAL_VIDEO_PROMPT_OUTPUT_CONTRACT_VERSION,
            }),
        );
        let (reviewed_trace, (_, reviewed_video_prompt)) =
            render_or_restore_json_with_validation_with_context(
                RenderOrRestoreJsonWithValidationInput {
                    service,
                    workflow_run_id: required_llm_call_workflow_run_id(
                        workflow_run_id,
                        &review_step_key,
                    )?,
                    call_key: review_step_key,
                    call_order: 400 + shot_number * 2,
                    input_fingerprint_json: review_fingerprint_json,
                    prompt_engine,
                    llm,
                    prompt_name: spec.video_prompt_review_name,
                    project_id,
                    variables: &vars,
                    context_manifest: review_context_manifest,
                    temperature: 0.25,
                    cancel_token,
                    step: VideoPromptStepProgress::new(
                        video_progress_span.second_half().start(),
                        100,
                        review_message.as_str(),
                    )
                    .with_span(video_progress_span.second_half()),
                    progress: &mut progress,
                    chunks: &mut chunks,
                    structured_progress: Some(video_prompt_progress_spec(is_chat_video)),
                    checkpoint_context: "视频提示词修订稿断点",
                },
                |raw| {
                    let prompt_json = parse_video_step_object(raw, spec.video_prompt_review_name)?;
                    let video_prompt = parse_single_generated_video_prompt_shot(
                        spec,
                        raw,
                        &segment_by_number,
                        prompt_output_language,
                        shot_number,
                        &allowed_reference_ids,
                        &format!("{}修订稿", spec.video_prompt_label),
                    )?;
                    Ok((prompt_json, video_prompt))
                },
                |(prompt_json, _)| Ok(prompt_json.clone()),
            )
            .await?;
        generated_video_prompt_traces.insert(
            shot_number,
            video_prompt_generation_trace(reviewed_trace, workflow_run_id),
        );
        context_video_prompts.push(reviewed_video_prompt.clone());
        video_prompts.push(reviewed_video_prompt);
        generated_this_run += 1;
        report_video_prompt_progress(
            &mut progress,
            video_progress_span.end(),
            100,
            &format!(
                "{}第 {}/{} 段已生成，等待整体验证后保存，本批已生成 {} 条",
                spec.video_prompt_label, current_index, total_segments, generated_this_run
            ),
        )?;
    }

    validate_partial_generated_video_prompts(
        spec,
        &video_prompts,
        &segment_by_number,
        prompt_output_language,
    )?;
    if video_prompts.len() == segment_by_number.len() {
        validate_generated_video_prompts(
            spec,
            &video_prompts,
            &segment_by_number,
            prompt_output_language,
        )?;
    }
    let video_drafts = video_prompt_drafts_for_spec(VideoPromptDraftsForSpecInput {
        spec,
        project_id,
        chapter_id,
        style: &style,
        prompt_output_language,
        source_metadata: &source_metadata,
        segment_by_number: &segment_by_number,
        video_unit_by_number: &video_unit_by_number,
        frame_by_number: &frame_by_number,
        available_assets: &available_assets,
        generation_reference_ids: &reference_ids,
        generation_asset_fingerprint: &generation_asset_fingerprint,
        video_prompts: &video_prompts,
        preserved_generated_reused_shots: &preserved_generated_reused_shots,
    })?;
    let video_drafts =
        generated_video_prompt_traces
            .iter()
            .fold(video_drafts, |drafts, (shot_number, trace)| {
                attach_generation_trace_to_draft(
                    drafts,
                    spec.workflow_mode,
                    "video_prompt",
                    *shot_number,
                    trace.clone(),
                )
            });
    let rows = if video_drafts.is_empty() {
        video_prompt_result_rows(
            service,
            project_id,
            chapter_id,
            spec.workflow_mode,
            &video_segment_plan,
            prompt_output_language,
        )
        .await?
    } else {
        sync_video_prompt_drafts(service, chapter_id, video_drafts, regenerate_existing).await?;
        video_prompt_result_rows(
            service,
            project_id,
            chapter_id,
            spec.workflow_mode,
            &video_segment_plan,
            prompt_output_language,
        )
        .await?
    };
    let completed_shots = context_video_prompts
        .iter()
        .map(GeneratedVideoPromptShot::shot_number)
        .collect::<HashSet<_>>();
    let completed_count = completed_shots.len();
    let total_video_unit_count = all_video_units.len();
    if generated_this_run == 0 && completed_count < total_video_unit_count {
        return Err(no_new_video_prompt_progress_error(
            spec,
            &all_video_units,
            &completed_shots,
            &frame_by_number,
        ));
    }
    let message = if generated_this_run == 0 {
        format!(
            "{}已全部生成，共 {completed_count}/{} 条",
            spec.video_prompt_label, total_video_unit_count
        )
    } else {
        format!(
            "本批已生成 {} 条{}，当前共 {}/{} 条",
            generated_this_run, spec.video_prompt_label, completed_count, total_video_unit_count
        )
    };
    report_video_prompt_progress(&mut progress, 100, 100, &message)?;
    Ok(rows)
}

fn no_new_video_prompt_progress_error(
    spec: &FramePromptGenerationSpec,
    all_video_units: &[VideoUnitPlan],
    completed_shots: &HashSet<i32>,
    frame_by_number: &HashMap<i32, VideoFramePrompt>,
) -> AppError {
    let Some(next_unit) = all_video_units
        .iter()
        .find(|unit| !completed_shots.contains(&unit.unit_number))
    else {
        return AppError::InvalidArgument(format!(
            "{}未生成新内容；请刷新后重试",
            spec.video_prompt_label
        ));
    };
    let completed_count = completed_shots.len();
    let total_count = all_video_units.len();
    let unit_number = next_unit.unit_number;
    match spec.workflow_mode {
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo => {
            let start_frame = next_unit.start_frame_number;
            let end_frame = next_unit.end_frame_number;
            let mut missing = Vec::new();
            if start_frame.is_none_or(|frame| !frame_by_number.contains_key(&frame)) {
                let start_text =
                    start_frame.map_or_else(|| "?".to_string(), |frame| frame.to_string());
                missing.push(format!("#{start_text}"));
            }
            if end_frame.is_none_or(|frame| !frame_by_number.contains_key(&frame)) {
                let end_text = end_frame.map_or_else(|| "?".to_string(), |frame| frame.to_string());
                missing.push(format!("#{end_text}"));
            }
            let start_text = start_frame.map_or_else(|| "?".to_string(), |frame| frame.to_string());
            let end_text = end_frame.map_or_else(|| "?".to_string(), |frame| frame.to_string());
            let missing_text = if missing.is_empty() {
                format!("视频单元 #{unit_number} 的连续边界帧")
            } else {
                format!("{} {}", spec.incomplete_frame_label, missing.join("、"))
            };
            AppError::InvalidArgument(format!(
                "当前章节{}不足，无法继续生成{}：已完成/可复用 {completed_count}/{total_count} 个视频单元；下一条视频提示词 #{} 需要连续边界帧 #{} 和 #{}，当前缺少{}。请先生成或补齐{}。",
                spec.incomplete_frame_label,
                spec.video_prompt_label,
                unit_number,
                start_text,
                end_text,
                missing_text,
                spec.incomplete_frame_label
            ))
        }
        PromptWorkflowMode::FirstFrameChain => AppError::InvalidArgument(format!(
            "当前章节{}不足，无法继续生成{}：已完成/可复用 {completed_count}/{total_count} 个视频单元；下一条视频提示词 #{} 缺少可承接的首帧输入。请先生成或补齐{}。",
            spec.incomplete_frame_label,
            spec.video_prompt_label,
            unit_number,
            spec.incomplete_frame_label
        )),
        PromptWorkflowMode::Storyboard => AppError::InvalidArgument(format!(
            "当前章节{}不足，无法继续生成{}：已完成/可复用 {completed_count}/{total_count} 个视频单元；下一条视频提示词 #{} 缺少同单元分镜提示词或分镜板。请先生成或补齐{}。",
            spec.incomplete_frame_label,
            spec.video_prompt_label,
            unit_number,
            spec.incomplete_frame_label
        )),
    }
}

async fn sync_video_prompt_drafts(
    service: &VideoService,
    chapter_id: i64,
    drafts: Vec<NewChapterVideoPrompt>,
    regenerate_existing: bool,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    if regenerate_existing {
        service.sync_chapter_prompts(chapter_id, drafts).await
    } else {
        service
            .sync_chapter_prompts_preserving_generated_video_prompts(chapter_id, drafts)
            .await
    }
}

async fn video_prompt_result_rows(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    segment_plan: &[VideoSegmentPlan],
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let segment_shots = segment_plan
        .iter()
        .map(|segment| segment.segment_number)
        .collect::<HashSet<_>>();
    Ok(
        video_prompt_rows_for_workflow(service, project_id, chapter_id, workflow_mode)
            .await?
            .into_iter()
            .filter(|row| segment_shots.contains(&row.shot_number))
            .filter(|row| {
                metadata_matches_prompt_output_language(
                    row.metadata_json.as_ref(),
                    prompt_output_language,
                )
            })
            .collect::<Vec<_>>(),
    )
}

impl VideoService {
    pub(super) async fn prepare_video_prompt_inputs(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: &ChapterVideoSourceContext<'_>,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
        include_project_context: bool,
    ) -> Result<PreparedVideoPromptInputs, AppError> {
        let options = validate_prompt_options(options)?;
        let source_metadata = video_source_metadata(source);
        self.ensure_prompt_generation_inputs(project_id, chapter_id, &options)
            .await?;
        let source_spans = source_spans_for_video_prompt(self, chapter_id, source).await?;
        let ChapterVideoPromptOptions {
            style,
            prompt_output_language,
            prompt_batch_size,
            duration_secs,
            aspect_ratio,
            reference_image_ids,
            video_library_image_ids,
            regenerate_existing,
            workflow_run_id,
            ..
        } = options;
        let prompt_output_language =
            PromptOutputLanguage::from_optional_code(prompt_output_language.as_deref())?;
        let reference_ids = json!(reference_image_ids.clone());
        let project_context = if include_project_context {
            self.chapter_video_project_context(project_id, source.analysis)
                .await?
        } else {
            None
        };
        let available_assets = self
            .video_library_image_context(
                project_id,
                chapter_id,
                source,
                &reference_image_ids,
                &video_library_image_ids,
            )
            .await?;
        let generation_asset_fingerprint = video_generation_asset_fingerprint(
            &reference_image_ids,
            &video_library_image_ids,
            &available_assets,
        );
        let mut vars = variables.clone();
        vars.insert("style".into(), json!(style));
        vars.insert(
            "prompt_output_language".into(),
            json!(prompt_output_language.as_str()),
        );
        vars.insert(
            "prompt_output_language_instruction".into(),
            json!(prompt_output_language.instruction()),
        );
        vars.insert("duration_secs".into(), json!(duration_secs));
        vars.insert("aspect_ratio".into(), json!(aspect_ratio));
        vars.insert("reference_image_ids".into(), reference_ids.clone());
        vars.insert("available_video_assets".into(), available_assets.clone());
        Ok(PreparedVideoPromptInputs {
            source_metadata,
            source_spans,
            regenerate_existing,
            prompt_batch_size: prompt_batch_size as usize,
            style,
            prompt_output_language,
            duration_secs,
            aspect_ratio,
            reference_image_ids,
            video_library_image_ids,
            reference_ids,
            generation_asset_fingerprint,
            project_context,
            available_assets,
            vars,
            workflow_run_id,
        })
    }

    pub async fn generate_chapter_boundary_frame_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_frame_prompt_chain(
            frame_prompt_chain_input(self, &BOUNDARY_FRAME_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub async fn generate_chapter_segment_plan(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        let workflow_mode = input.workflow_mode;
        let spec = frame_prompt_generation_spec_for_workflow(workflow_mode)?;
        generate_chapter_segment_plan_for_spec(
            SegmentPlanForSpecInput {
                service: self,
                spec,
                stage: input,
            },
            chunks,
            progress,
        )
        .await
    }

    pub async fn generate_chapter_shot_analysis(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        mut chunks: Option<VideoPromptChunk<'_>>,
        mut progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        let ChapterVideoPromptStageInput {
            project_id,
            chapter_id,
            workflow_mode,
            source,
            llm,
            prompt_engine,
            variables,
            options,
            cancel_token,
        } = input;
        let (workflow_mode, prompt_type, title, progress_message) =
            shot_analysis_spec_for_workflow(workflow_mode)?;
        let PreparedVideoPromptInputs {
            source_metadata,
            source_spans,
            regenerate_existing,
            style,
            duration_secs,
            aspect_ratio,
            reference_image_ids: _,
            project_context,
            available_assets: _,
            vars,
            workflow_run_id,
            ..
        } = self
            .prepare_video_prompt_inputs(project_id, chapter_id, &source, variables, options, true)
            .await?;
        if regenerate_existing {
            ensure_existing_shot_analysis_can_regenerate(
                self,
                project_id,
                chapter_id,
                workflow_mode,
                prompt_type,
                &[],
            )
            .await?;
        }
        let prepared = prepare_shot_analysis(
            ShotAnalysisPreparationInput {
                service: self,
                workflow_mode,
                prompt_type,
                title,
                progress_message,
                ignored_downstream_prompt_types: &[],
                project_id,
                chapter_id,
                source,
                source_metadata: &source_metadata,
                source_spans: &source_spans,
                project_context: project_context.as_ref(),
                style: &style,
                duration_secs,
                aspect_ratio: &aspect_ratio,
                prompt_engine,
                llm,
                variables: &vars,
                regenerate_existing,
                workflow_run_id,
                progress_span: ProgressSpan::new(20, 90),
                cancel_token,
            },
            &mut progress,
            &mut chunks,
        )
        .await?;
        if let Some(draft) = prepared.draft {
            self.sync_chapter_prompts(chapter_id, vec![draft]).await
        } else {
            self.reusable_shared_shot_analysis_row(
                project_id,
                chapter_id,
                SHARED_SHOT_ANALYSIS_WORKFLOW_MODE.as_str(),
                SHARED_SHOT_ANALYSIS_PROMPT_TYPE,
                &source_metadata,
            )
            .await?
            .map(|row| vec![row])
            .ok_or_else(|| AppError::InvalidState {
                current: "missing".to_string(),
                operation: "读取共享镜头分析".to_string(),
            })
        }
    }

    pub async fn generate_chapter_first_frame_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_frame_prompt_chain(
            frame_prompt_chain_input(self, &FIRST_FRAME_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub async fn generate_chapter_chat_video_frame_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_frame_prompt_chain(
            frame_prompt_chain_input(self, &CHAT_VIDEO_FRAME_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub async fn generate_chapter_storyboard_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_frame_prompt_chain(
            frame_prompt_chain_input(self, &STORYBOARD_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub async fn generate_chapter_chat_video_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_video_prompt_chain(
            video_prompt_chain_input(self, &CHAT_VIDEO_FRAME_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub async fn generate_chapter_video_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_video_prompt_chain(
            video_prompt_chain_input(self, &BOUNDARY_FRAME_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub async fn generate_chapter_first_frame_video_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_video_prompt_chain(
            video_prompt_chain_input(self, &FIRST_FRAME_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }

    pub async fn generate_chapter_storyboard_video_prompts(
        &self,
        input: ChapterVideoPromptStageInput<'_, '_>,
        chunks: Option<VideoPromptChunk<'_>>,
        progress: Option<VideoPromptProgress<'_>>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        generate_chapter_video_prompt_chain(
            video_prompt_chain_input(self, &STORYBOARD_PROMPT_GENERATION, input),
            chunks,
            progress,
        )
        .await
    }
}
