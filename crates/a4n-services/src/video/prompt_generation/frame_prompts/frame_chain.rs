use super::*;

fn backend_frame_reference_ids(
    material_reference_image_ids: &[i64],
    first_reference_image_id: Option<i64>,
) -> Vec<i64> {
    let mut ids = Vec::new();
    if let Some(id) = first_reference_image_id {
        ids.push(id);
    }
    for id in material_reference_image_ids.iter().copied() {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

pub(super) struct SegmentPlanForSpecInput<'a, 'source> {
    pub service: &'a VideoService,
    pub spec: &'a FramePromptGenerationSpec,
    pub stage: ChapterVideoPromptStageInput<'a, 'source>,
}

pub(super) struct FramePromptChainInput<'a, 'source> {
    pub service: &'a VideoService,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: ChapterVideoSourceContext<'source>,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub options: ChapterVideoPromptOptions,
    pub cancel_token: Option<&'a CancellationToken>,
}

pub(super) struct ShotAnalysisPreparationInput<'a, 'source> {
    pub service: &'a VideoService,
    pub workflow_mode: PromptWorkflowMode,
    pub prompt_type: &'a str,
    pub title: &'a str,
    pub progress_message: &'a str,
    pub ignored_downstream_prompt_types: &'a [&'a str],
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: ChapterVideoSourceContext<'source>,
    pub source_metadata: &'a Value,
    pub source_spans: &'a [SourceSpan],
    pub project_context: Option<&'a Value>,
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub prompt_engine: &'a dyn PromptEngine,
    pub llm: &'a dyn LlmProvider,
    pub variables: &'a HashMap<String, Value>,
    pub regenerate_existing: bool,
    pub workflow_run_id: Option<i64>,
    pub progress_span: ProgressSpan,
    pub cancel_token: Option<&'a CancellationToken>,
}

pub(super) struct SegmentPlanFromShotAnalysisInput<'a, 'source> {
    pub shot_analysis: ShotAnalysisPreparationInput<'a, 'source>,
    pub segment_context: &'a str,
    pub segment_progress: &'a str,
    pub segment_span: ProgressSpan,
}

pub(super) async fn prepare_shot_analysis(
    input: ShotAnalysisPreparationInput<'_, '_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
    chunks: &mut Option<VideoPromptChunk<'_>>,
) -> Result<PreparedShotAnalysis, AppError> {
    let ShotAnalysisPreparationInput {
        service,
        workflow_mode,
        prompt_type: shot_analysis_prompt_type,
        title: shot_analysis_title,
        progress_message: shot_analysis_progress,
        ignored_downstream_prompt_types,
        project_id,
        chapter_id,
        source,
        source_metadata,
        source_spans,
        project_context,
        style,
        duration_secs,
        aspect_ratio,
        prompt_engine,
        llm,
        variables,
        regenerate_existing,
        workflow_run_id,
        progress_span,
        cancel_token,
    } = input;
    let reusable_analysis = if regenerate_existing {
        None
    } else {
        reusable_shared_shot_analysis_output(
            service,
            project_id,
            chapter_id,
            workflow_mode,
            source_metadata,
        )
        .await?
    };
    let reused_existing_analysis = reusable_analysis.is_some();
    if !regenerate_existing && reusable_analysis.is_none() {
        ensure_shot_analysis_generation_can_start(
            service,
            project_id,
            chapter_id,
            workflow_mode,
            shot_analysis_prompt_type,
            ignored_downstream_prompt_types,
        )
        .await?;
    }
    let (analysis_out, analysis_trace, analysis_json, analysis, analysis_stage_outputs) =
        if let Some((reused_spec, existing)) = reusable_analysis {
            let analysis =
                parse_stored_video_shot_analysis(&existing, source_spans, duration_secs)?;
            let analysis_json = serde_json::to_value(&analysis).map_err(|err| {
                AppError::InvalidArgument(format!(
                    "chapter_video_shot_analysis 派生结果序列化失败: {err}"
                ))
            })?;
            report_video_prompt_progress(
                progress,
                progress_span.end(),
                100,
                &format!("已复用{}", reused_spec.title),
            )?;
            (existing, None, analysis_json, analysis, None)
        } else {
            report_video_prompt_progress(
                progress,
                progress_span.start(),
                100,
                shot_analysis_progress,
            )?;
            let (trace, analysis_json, analysis, stage_outputs) =
                render_staged_shot_analysis(StagedShotAnalysisInput {
                    service,
                    prompt_engine,
                    llm,
                    project_id,
                    workflow_run_id,
                    variables,
                    source,
                    source_spans,
                    project_context,
                    style,
                    duration_secs,
                    aspect_ratio,
                    progress_message: shot_analysis_progress,
                    progress_span,
                    cancel_token,
                    progress,
                    chunks,
                })
                .await?;
            let analysis_out = analysis_json.to_string();
            (
                analysis_out,
                trace.map(|trace| video_prompt_generation_trace(trace, workflow_run_id)),
                analysis_json,
                analysis,
                Some(stage_outputs),
            )
        };
    let draft = (!reused_existing_analysis || regenerate_existing).then(|| {
        new_prompt_draft(NewChapterVideoPromptInput {
            project_id,
            chapter_id,
            workflow_mode,
            style: Some(style),
            prompt_type: shot_analysis_prompt_type,
            shot_number: 0,
            title: Some(shot_analysis_title),
            scene_summary: None,
            visual_prompt: &analysis_out,
            motion_prompt: None,
            negative_prompt: None,
            duration_secs,
            aspect_ratio: Some(aspect_ratio),
            reference_image_ids_json: Some(json!([])),
            metadata_json: Some(json!({
                "contract_version": SHOT_ANALYSIS_CONTRACT_VERSION,
                "canonical": analysis_out,
                "stage_outputs": analysis_stage_outputs,
                "cinematic_shots": analysis.cinematic_shots.clone(),
                "planning_quality": analysis.planning_quality.clone(),
                "source": source_metadata.clone()
            })),
        })
        .with_generation_trace_if(analysis_trace)
    });
    Ok(PreparedShotAnalysis {
        analysis_json,
        analysis,
        draft,
    })
}

pub(super) async fn render_segment_plan_from_shot_analysis(
    input: SegmentPlanFromShotAnalysisInput<'_, '_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
    chunks: &mut Option<VideoPromptChunk<'_>>,
) -> Result<PreparedSegmentPlan, AppError> {
    let SegmentPlanFromShotAnalysisInput {
        shot_analysis,
        segment_context,
        segment_progress,
        segment_span,
    } = input;
    let ShotAnalysisPreparationInput {
        service,
        workflow_mode,
        prompt_type: shot_analysis_prompt_type,
        title: shot_analysis_title,
        progress_message: shot_analysis_progress,
        ignored_downstream_prompt_types: ignored_shot_analysis_downstream_prompt_types,
        project_id,
        chapter_id,
        source,
        source_metadata,
        source_spans,
        project_context,
        style,
        duration_secs,
        aspect_ratio,
        prompt_engine,
        llm,
        variables,
        regenerate_existing,
        workflow_run_id,
        progress_span: shot_analysis_span,
        cancel_token,
    } = shot_analysis;
    let PreparedShotAnalysis {
        analysis_json,
        analysis,
        draft,
    } = prepare_shot_analysis(
        ShotAnalysisPreparationInput {
            service,
            workflow_mode,
            prompt_type: shot_analysis_prompt_type,
            title: shot_analysis_title,
            progress_message: shot_analysis_progress,
            ignored_downstream_prompt_types: ignored_shot_analysis_downstream_prompt_types,
            project_id,
            chapter_id,
            source,
            source_metadata,
            source_spans,
            project_context,
            style,
            duration_secs,
            aspect_ratio,
            prompt_engine,
            llm,
            variables,
            regenerate_existing,
            workflow_run_id,
            progress_span: shot_analysis_span,
            cancel_token,
        },
        progress,
        chunks,
    )
    .await?;
    if let Some(shot_analysis_draft) = draft.clone() {
        service
            .sync_chapter_prompts(chapter_id, vec![shot_analysis_draft])
            .await?;
        report_video_prompt_progress(
            progress,
            shot_analysis_span.end(),
            100,
            "镜头分析已保存，可用于断点续传",
        )?;
    }
    let (segment_trace, segment_json, segment_plan, segment_stage_outputs) =
        render_staged_segment_plan_with_duration(StagedSegmentPlanInput {
            service,
            prompt_engine,
            llm,
            project_id,
            workflow_run_id,
            variables,
            source,
            source_spans,
            project_context,
            shot_analysis_json: &analysis_json,
            shot_analysis: &analysis,
            default_aspect_ratio: aspect_ratio,
            style,
            duration_secs,
            context: segment_context,
            progress_message: segment_progress,
            progress_span: segment_span,
            cancel_token,
            progress,
            chunks,
        })
        .await?;
    Ok(PreparedSegmentPlan {
        segment_stage_outputs,
        segment_trace: segment_trace
            .map(|trace| video_prompt_generation_trace(trace, workflow_run_id)),
        segment_json,
        segment_plan,
        shot_analysis_draft: draft,
    })
}

pub(super) fn shot_analysis_downstream_prompt_types(
    spec: &FramePromptGenerationSpec,
) -> Vec<&'static str> {
    std::iter::once(spec.segment_prompt_type)
        .chain(spec.downstream_frame_prompt_types.iter().copied())
        .collect()
}

pub(super) fn shot_analysis_spec_for_workflow(
    workflow_mode: &str,
) -> Result<(PromptWorkflowMode, &'static str, &'static str, &'static str), AppError> {
    match workflow_mode {
        "boundary_frames" | "first_frame_chain" | "chat_video" | "storyboard" => Ok((
            SHARED_SHOT_ANALYSIS_WORKFLOW_MODE,
            SHARED_SHOT_ANALYSIS_PROMPT_TYPE,
            SHARED_SHOT_ANALYSIS_TITLE,
            SHARED_SHOT_ANALYSIS_PROGRESS,
        )),
        other => Err(AppError::InvalidArgument(format!(
            "镜头语言分析不支持 workflow_mode={other}"
        ))),
    }
}

pub(super) async fn generate_chapter_segment_plan_for_spec(
    input: SegmentPlanForSpecInput<'_, '_>,
    mut chunks: Option<VideoPromptChunk<'_>>,
    mut progress: Option<VideoPromptProgress<'_>>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let SegmentPlanForSpecInput {
        service,
        spec,
        stage,
    } = input;
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
    } = stage;
    let PreparedVideoPromptInputs {
        source_metadata,
        source_spans,
        regenerate_existing,
        style,
        duration_secs,
        aspect_ratio,
        project_context,
        vars,
        workflow_run_id,
        ..
    } = service
        .prepare_video_prompt_inputs(project_id, chapter_id, &source, variables, options, true)
        .await?;

    let segment_storage_workflow_mode = spec.segment_storage_workflow_mode();
    let segment_storage_prompt_type = spec.segment_storage_prompt_type();
    let segment_storage_contract_version = spec.segment_storage_contract_version();
    let segment_storage_context = spec.segment_storage_context();
    let segment_storage_progress = spec.segment_storage_progress();
    let segment_only_shot_analysis_downstream_prompt_types = [segment_storage_prompt_type];
    if regenerate_existing {
        ensure_no_existing_downstream_prompts_before_segment_planning(
            DownstreamSegmentPlanningGuardInput {
                service,
                project_id,
                chapter_id,
                spec,
                source_metadata: &source_metadata,
                style: &style,
                aspect_ratio: &aspect_ratio,
                operation: "重新生成",
            },
        )
        .await?;
        ensure_existing_segment_plans_can_regenerate(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            &[],
        )
        .await?;
    } else {
        ensure_no_existing_downstream_prompts_before_segment_planning(
            DownstreamSegmentPlanningGuardInput {
                service,
                project_id,
                chapter_id,
                spec,
                source_metadata: &source_metadata,
                style: &style,
                aspect_ratio: &aspect_ratio,
                operation: "生成",
            },
        )
        .await?;
    }

    let reusable_segment = if regenerate_existing {
        None
    } else {
        reusable_segment_plan_for_spec(ReusableSegmentPlanForSpecQuery {
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
    };
    if !regenerate_existing && reusable_segment.is_none() {
        ensure_no_existing_segment_plan_before_new_planning(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            segment_storage_context,
        )
        .await?;
        ensure_segment_plan_generation_can_start(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            &[],
        )
        .await?;
    }

    report_video_prompt_progress(&mut progress, 20, 100, segment_storage_progress)?;
    let mut shot_analysis_draft = None;
    let (_segment_plan, segment_draft) = if let Some(existing) = reusable_segment {
        (existing.segment_plan, existing.draft)
    } else {
        let prepared = render_segment_plan_from_shot_analysis(
            SegmentPlanFromShotAnalysisInput {
                shot_analysis: ShotAnalysisPreparationInput {
                    service,
                    workflow_mode: SHARED_SHOT_ANALYSIS_WORKFLOW_MODE,
                    prompt_type: spec.shot_analysis_prompt_type,
                    title: spec.shot_analysis_title,
                    progress_message: spec.shot_analysis_progress,
                    ignored_downstream_prompt_types:
                        &segment_only_shot_analysis_downstream_prompt_types,
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
                    progress_span: ProgressSpan::new(20, 45),
                    cancel_token,
                },
                segment_context: segment_storage_context,
                segment_progress: segment_storage_progress,
                segment_span: ProgressSpan::new(45, 90),
            },
            &mut progress,
            &mut chunks,
        )
        .await?;
        shot_analysis_draft = prepared.shot_analysis_draft;
        let canonical_segment_out = prepared.segment_json.to_string();
        let segment_draft = new_prompt_draft(NewChapterVideoPromptInput {
            project_id,
            chapter_id,
            workflow_mode: segment_storage_workflow_mode,
            style: Some(&style),
            prompt_type: segment_storage_prompt_type,
            shot_number: 0,
            title: Some(spec.segment_storage_title()),
            scene_summary: None,
            visual_prompt: &canonical_segment_out,
            motion_prompt: None,
            negative_prompt: None,
            duration_secs,
            aspect_ratio: Some(&aspect_ratio),
            reference_image_ids_json: Some(json!([])),
            metadata_json: Some(json!({
                "contract_version": segment_storage_contract_version,
                "stage_outputs": prepared.segment_stage_outputs,
                "segments": prepared.segment_plan,
                "source": source_metadata.clone()
            })),
        })
        .with_generation_trace_if(prepared.segment_trace);
        (prepared.segment_plan, segment_draft)
    };
    let mut drafts = Vec::new();
    if let Some(draft) = shot_analysis_draft {
        drafts.push(draft);
    }
    drafts.push(segment_draft);
    let rows = service.sync_chapter_prompts(chapter_id, drafts).await?;
    report_video_prompt_progress(&mut progress, 100, 100, "视频片段规划记录已更新")?;
    Ok(rows)
}

pub(super) async fn generate_chapter_frame_prompt_chain(
    input: FramePromptChainInput<'_, '_>,
    mut chunks: Option<VideoPromptChunk<'_>>,
    mut progress: Option<VideoPromptProgress<'_>>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let FramePromptChainInput {
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
        reference_ids,
        generation_asset_fingerprint,
        project_context,
        available_assets,
        vars,
        workflow_run_id,
        ..
    } = service
        .prepare_video_prompt_inputs(project_id, chapter_id, &source, variables, options, true)
        .await?;
    let segment_storage_workflow_mode = spec.segment_storage_workflow_mode();
    let segment_storage_prompt_type = spec.segment_storage_prompt_type();
    let segment_storage_contract_version = spec.segment_storage_contract_version();
    let segment_storage_context = spec.segment_storage_context();
    let segment_storage_progress = spec.segment_storage_progress();
    let frame_source_prompt_type = spec.frame_source_prompt_type();
    let shot_analysis_downstream_prompt_types = shot_analysis_downstream_prompt_types(spec);
    if regenerate_existing {
        ensure_existing_frame_prompts_can_regenerate(
            service,
            project_id,
            chapter_id,
            spec.workflow_mode,
            spec.frame_prompt_type,
            prompt_output_language,
        )
        .await?;
        ensure_existing_segment_plans_can_regenerate(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            spec.downstream_frame_prompt_types,
        )
        .await?;
        ensure_existing_shot_analysis_can_regenerate(
            service,
            project_id,
            chapter_id,
            SHARED_SHOT_ANALYSIS_WORKFLOW_MODE,
            spec.shot_analysis_prompt_type,
            &shot_analysis_downstream_prompt_types,
        )
        .await?;
    }

    let reusable_segment = if regenerate_existing {
        None
    } else {
        reusable_segment_plan_for_spec(ReusableSegmentPlanForSpecQuery {
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
    };
    if !regenerate_existing && reusable_segment.is_none() {
        ensure_no_existing_segment_plan_before_new_planning(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            segment_storage_context,
        )
        .await?;
        ensure_segment_plan_generation_can_start(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            spec.downstream_frame_prompt_types,
        )
        .await?;
    }
    report_video_prompt_progress(&mut progress, 20, 100, segment_storage_progress)?;
    let mut shot_analysis_draft = None;
    let (segment_json, segment_plan, segment_draft) = if let Some(existing) = reusable_segment {
        (existing.segment_json, existing.segment_plan, None)
    } else {
        let prepared = render_segment_plan_from_shot_analysis(
            SegmentPlanFromShotAnalysisInput {
                shot_analysis: ShotAnalysisPreparationInput {
                    service,
                    workflow_mode: SHARED_SHOT_ANALYSIS_WORKFLOW_MODE,
                    prompt_type: spec.shot_analysis_prompt_type,
                    title: spec.shot_analysis_title,
                    progress_message: spec.shot_analysis_progress,
                    ignored_downstream_prompt_types: &shot_analysis_downstream_prompt_types,
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
                    progress_span: ProgressSpan::new(25, 45),
                    cancel_token,
                },
                segment_context: segment_storage_context,
                segment_progress: segment_storage_progress,
                segment_span: ProgressSpan::new(45, 60),
            },
            &mut progress,
            &mut chunks,
        )
        .await?;
        shot_analysis_draft = prepared.shot_analysis_draft;
        let segment_stage_outputs = prepared.segment_stage_outputs;
        let segment_trace = prepared.segment_trace;
        let segment_json = prepared.segment_json;
        let canonical_segment_out = segment_json.to_string();
        let segment_plan = prepared.segment_plan;
        let segment_draft = new_prompt_draft(NewChapterVideoPromptInput {
            project_id,
            chapter_id,
            workflow_mode: segment_storage_workflow_mode,
            style: Some(&style),
            prompt_type: segment_storage_prompt_type,
            shot_number: 0,
            title: Some(spec.segment_storage_title()),
            scene_summary: None,
            visual_prompt: &canonical_segment_out,
            motion_prompt: None,
            negative_prompt: None,
            duration_secs,
            aspect_ratio: Some(&aspect_ratio),
            reference_image_ids_json: Some(json!([])),
            metadata_json: Some(json!({
                "contract_version": segment_storage_contract_version,
                "stage_outputs": segment_stage_outputs,
                "segments": segment_plan,
                "source": source_metadata.clone()
            })),
        })
        .with_generation_trace_if(segment_trace);
        (segment_json, segment_plan, Some(segment_draft))
    };
    let allowed_reference_ids = available_video_image_ids(&available_assets, &reference_image_ids)?;
    if !regenerate_existing {
        let mut initial_drafts = Vec::new();
        if let Some(draft) = shot_analysis_draft.clone() {
            initial_drafts.push(draft);
        }
        if let Some(draft) = segment_draft.clone() {
            initial_drafts.push(draft);
        }
        if !initial_drafts.is_empty() {
            service
                .sync_chapter_prompts(chapter_id, initial_drafts)
                .await?;
        }
    }

    let expected_frame_count = match spec.layout {
        FramePromptLayout::BoundaryFrames => expected_boundary_frame_count(&segment_plan),
        FramePromptLayout::InitialFirstFrame => FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT,
        FramePromptLayout::PerSegmentStoryboard => expected_video_unit_count(&segment_plan),
    };
    let video_units = video_units_for_segment_plan(&segment_plan, duration_secs);
    let frame_draft_common = VideoPromptDraftCommonInput {
        project_id,
        chapter_id,
        style: &style,
        prompt_output_language,
        source_metadata: &source_metadata,
        available_assets: &available_assets,
        generation_asset_fingerprint: &generation_asset_fingerprint,
    };
    report_video_prompt_progress(&mut progress, 60, 100, spec.frame_check_progress)?;
    let reusable_frame_rows =
        if regenerate_existing || matches!(spec.layout, FramePromptLayout::InitialFirstFrame) {
            None
        } else {
            let rows = reusable_frame_rows_for_resume(ReusableFrameRowsQuery {
                service,
                project_id,
                chapter_id,
                workflow_mode: spec.workflow_mode,
                prompt_type: spec.frame_prompt_type,
                source_metadata: &source_metadata,
                style: &style,
                duration_secs,
                aspect_ratio: &aspect_ratio,
                reference_ids: &reference_ids,
                generation_asset_fingerprint: Some(&generation_asset_fingerprint),
                source_prompt_type: Some(frame_source_prompt_type),
                prompt_output_language,
            })
            .await?;
            Some(if spec.is_first_frame_chain() {
                initial_first_frame_rows(rows)
            } else {
                rows
            })
        };
    let frame_drafts = if matches!(spec.layout, FramePromptLayout::BoundaryFrames) {
        let reusable_rows = reusable_frame_rows.unwrap_or_default();
        let frame_image_lookup_rows = if regenerate_existing {
            Vec::new()
        } else {
            boundary_frame_rows_for_image_lookup(FrameImageLookupRowsQuery {
                service,
                project_id,
                chapter_id,
                context: FrameImageLookupContext {
                    workflow_mode: spec.workflow_mode,
                    prompt_type: spec.frame_prompt_type,
                    source_metadata: &source_metadata,
                    style: &style,
                    aspect_ratio: &aspect_ratio,
                    source_prompt_type: Some(frame_source_prompt_type),
                    prompt_output_language,
                },
            })
            .await?
        };
        let boundary_frame_image_id_by_number = boundary_frame_image_ids_by_number(
            service,
            project_id,
            chapter_id,
            &frame_image_lookup_rows,
        )
        .await?;
        let reusable_rows = sync_reusable_boundary_frame_previous_image_refs(
            service,
            reusable_rows,
            &boundary_frame_image_id_by_number,
        )
        .await?;
        let mut allowed_boundary_frame_reference_ids = allowed_reference_ids.clone();
        for image_id in boundary_frame_image_id_by_number.values().copied() {
            allowed_boundary_frame_reference_ids.insert(image_id);
        }
        let mut prefix_rows = boundary_frame_prompt_rows_prefix(
            reusable_rows,
            &segment_plan,
            &allowed_boundary_frame_reference_ids,
            &boundary_frame_image_id_by_number,
        )?;
        let mut frame_prompts = frame_prompts_from_rows(&prefix_rows)?;
        let reusable_prefix_len = reusable_frame_prefix_len_with_previous_images(
            &frame_prompts,
            &boundary_frame_image_id_by_number,
        );
        if reusable_prefix_len < frame_prompts.len() {
            tracing::info!(
                target: "a4n::video_prompt",
                project_id,
                chapter_id,
                reusable_prefix_len,
                original_prefix_len = frame_prompts.len(),
                "边界帧提示词缺少已生成上一帧普通参考图，将从该帧开始重新生成"
            );
            prefix_rows.truncate(reusable_prefix_len);
            frame_prompts.truncate(reusable_prefix_len);
        }
        let mut kept_prompt_ids = prefix_rows
            .iter()
            .map(|row| row.id)
            .collect::<BTreeSet<_>>();
        if frame_prompts.len() == expected_frame_count {
            validate_frame_prompts(
                &frame_prompts,
                &segment_plan,
                &allowed_boundary_frame_reference_ids,
            )?;
            report_video_prompt_progress(
                &mut progress,
                90,
                100,
                &format!(
                    "已复用 {} 条{}，正在写入记录",
                    prefix_rows.len(),
                    spec.reused_frame_label
                ),
            )?;
            prefix_rows
                .iter()
                .map(prompt_row_draft)
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let message = if regenerate_existing {
                spec.frame_regenerate_progress
            } else {
                spec.frame_generate_progress
            };
            let mut generated_this_run = 0usize;
            while frame_prompts.len() < expected_frame_count
                && generated_this_run < prompt_batch_size
            {
                ensure_boundary_frame_checkpoint_can_sync(
                    service,
                    project_id,
                    chapter_id,
                    spec.workflow_mode,
                    spec.frame_prompt_type,
                    &kept_prompt_ids,
                )
                .await?;
                let frame_number = (frame_prompts.len() + 1) as i32;
                let generation_message = format!(
                    "{message}：正在生成第 {}/{} 张边界帧，本批第 {}/{} 条",
                    frame_number,
                    expected_frame_count,
                    generated_this_run + 1,
                    prompt_batch_size
                );
                let frame_progress_span = ProgressSpan::new(60, 90).split(
                    frame_number.saturating_sub(1) as usize,
                    expected_frame_count,
                );
                report_video_prompt_progress(
                    &mut progress,
                    frame_progress_span.start(),
                    100,
                    &generation_message,
                )?;
                let previous_frame_image_id = if frame_number > 1 {
                    boundary_frame_image_id_by_number
                        .get(&(frame_number - 1))
                        .copied()
                } else {
                    None
                };
                let frame_available_assets = available_assets_with_previous_boundary_frame(
                    &available_assets,
                    previous_frame_image_id,
                    frame_prompts.last(),
                );
                let frame_prompt_available_assets =
                    frame_prompt_available_assets_context(&frame_available_assets)?;
                let frame_allowed_reference_ids =
                    available_video_image_ids(&frame_available_assets, &reference_image_ids)?;
                let frame_reference_image_ids =
                    backend_frame_reference_ids(&reference_image_ids, previous_frame_image_id);
                let frame_reference_ids = json!(frame_reference_image_ids.clone());
                let current_frame = boundary_frame_generation_context(
                    &segment_plan,
                    &frame_prompts,
                    frame_number,
                    previous_boundary_frame_asset_context(
                        &frame_available_assets,
                        previous_frame_image_id,
                    ),
                )?;
                let frame_prompt_context = FramePromptContext {
                    prompt_engine,
                    project_id,
                    chapter_summary: source.summary,
                    chapter_analysis: source.analysis,
                    segment_plan: &segment_json,
                    available_assets: &frame_prompt_available_assets,
                    style: &style,
                    prompt_output_language,
                    aspect_ratio: &aspect_ratio,
                    reference_ids: &frame_reference_ids,
                };
                let frame_user =
                    build_single_frame_prompt_user(frame_prompt_context, &current_frame)?;
                let frame_context_manifest =
                    build_video_context_manifest(VideoContextManifestInput {
                        prompt_engine,
                        project_id,
                        variables: &vars,
                        source: &source,
                        source_spans: &source_spans,
                        prompt_names: &["chapter_video_segment_plan", spec.frame_prompt_name],
                        previous_outputs: &[VideoContextPreviousOutput {
                            pack_kind: VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN,
                            prompt_name: "chapter_video_segment_plan",
                            parsed_output: &segment_json,
                            raw_output: None,
                            placeholder: false,
                        }],
                        project_context: project_context.as_ref(),
                        asset_context: Some(&frame_prompt_available_assets),
                        dynamic_user_content: frame_user.clone(),
                    })?;
                let frame_step_key = format!("{}:frame:{frame_number}", spec.frame_prompt_name);
                let frame_fingerprint_json = video_stage_fingerprint(
                    spec.frame_prompt_name,
                    &frame_context_manifest,
                    json!({
                        "project_id": project_id,
                        "chapter_id": chapter_id,
                        "frame_number": frame_number,
                        "frame_prompt_type": spec.frame_prompt_type,
                    }),
                );
                let frame_workflow_run_id =
                    required_llm_call_workflow_run_id(workflow_run_id, &frame_step_key)?;
                let (frame_trace, frame_prompt) =
                    render_or_restore_json_with_validation_with_context(
                        RenderOrRestoreJsonWithValidationInput {
                            service,
                            workflow_run_id: frame_workflow_run_id,
                            call_key: frame_step_key,
                            call_order: 300 + frame_number,
                            input_fingerprint_json: frame_fingerprint_json,
                            prompt_engine,
                            llm,
                            prompt_name: spec.frame_prompt_name,
                            project_id,
                            variables: &vars,
                            context_manifest: frame_context_manifest,
                            temperature: 0.45,
                            cancel_token,
                            step: VideoPromptStepProgress::new(
                                frame_progress_span.start(),
                                100,
                                generation_message.as_str(),
                            )
                            .with_span(frame_progress_span),
                            progress: &mut progress,
                            chunks: &mut chunks,
                            structured_progress: Some(frame_prompt_progress_spec(
                                "边界帧",
                                Some(1),
                            )),
                            checkpoint_context: "边界帧提示词断点",
                        },
                        |raw| {
                            let expected_segment_numbers = boundary_frame_expected_segment_numbers(
                                &segment_plan,
                                frame_number,
                            )?;
                            let default_duration_secs = boundary_frame_expected_duration(
                                &segment_plan,
                                &expected_segment_numbers,
                            )?;
                            let mut frame_prompt = parse_single_video_frame_prompt(
                                raw,
                                VideoFramePromptDefaults {
                                    frame_number,
                                    duration_secs: default_duration_secs,
                                    aspect_ratio: &aspect_ratio,
                                    segment_numbers: expected_segment_numbers,
                                    reference_image_ids: frame_reference_image_ids.clone(),
                                },
                                prompt_output_language,
                                frame_number,
                                "边界帧提示词",
                            )?;
                            apply_boundary_frame_style_template(
                                &mut frame_prompt,
                                &style,
                                prompt_output_language,
                            );
                            validate_single_boundary_frame_prompt(
                                &frame_prompt,
                                &segment_plan,
                                &frame_allowed_reference_ids,
                                previous_frame_image_id,
                            )?;
                            Ok(frame_prompt)
                        },
                        |frame_prompt| serialize_video_value(frame_prompt, "边界帧提示词断点"),
                    )
                    .await?;
                allowed_boundary_frame_reference_ids
                    .extend(frame_prompt.reference_image_ids.iter().copied());
                frame_prompts.push(frame_prompt);
                generated_this_run += 1;

                let frame_drafts = frame_prompt_drafts(FramePromptDraftsInput {
                    common: frame_draft_common,
                    workflow_mode: spec.workflow_mode,
                    prompt_type: spec.frame_prompt_type,
                    source_prompt_type: frame_source_prompt_type,
                    frame_prompts: &frame_prompts,
                });
                let frame_drafts = attach_generation_trace_to_draft(
                    frame_drafts,
                    spec.workflow_mode,
                    spec.frame_prompt_type,
                    frame_number,
                    video_prompt_generation_trace(frame_trace, workflow_run_id),
                );
                let mut checkpoint_drafts = Vec::new();
                if let Some(draft) = shot_analysis_draft.clone() {
                    checkpoint_drafts.push(draft);
                }
                if let Some(draft) = segment_draft.clone() {
                    checkpoint_drafts.push(draft);
                }
                checkpoint_drafts.extend(frame_drafts);
                let checkpoint_rows = service
                    .sync_chapter_prompts_preserving_generated_frame_prompts(
                        chapter_id,
                        checkpoint_drafts,
                    )
                    .await?;
                kept_prompt_ids = checkpoint_rows
                    .iter()
                    .filter(|row| row.prompt_type == spec.frame_prompt_type)
                    .map(|row| row.id)
                    .collect();
                report_video_prompt_progress(
                    &mut progress,
                    frame_progress_span.end(),
                    100,
                    &format!(
                        "边界帧提示词第 {}/{} 张已保存，本批已生成 {} 条",
                        frame_prompts.len(),
                        expected_frame_count,
                        generated_this_run
                    ),
                )?;
            }
            validate_partial_boundary_frame_prompts(
                &frame_prompts,
                &segment_plan,
                &allowed_boundary_frame_reference_ids,
                &boundary_frame_image_id_by_number,
            )?;
            if frame_prompts.len() == expected_frame_count {
                validate_frame_prompts(
                    &frame_prompts,
                    &segment_plan,
                    &allowed_boundary_frame_reference_ids,
                )?;
            }
            let final_message = if frame_prompts.len() == expected_frame_count {
                format!(
                    "边界帧提示词已全部生成，共 {}/{} 条",
                    frame_prompts.len(),
                    expected_frame_count
                )
            } else {
                format!(
                    "本批已生成 {} 条边界帧提示词，当前共 {}/{} 条",
                    generated_this_run,
                    frame_prompts.len(),
                    expected_frame_count
                )
            };
            report_video_prompt_progress(&mut progress, 90, 100, &final_message)?;
            frame_prompt_drafts(FramePromptDraftsInput {
                common: frame_draft_common,
                workflow_mode: spec.workflow_mode,
                prompt_type: spec.frame_prompt_type,
                source_prompt_type: frame_source_prompt_type,
                frame_prompts: &frame_prompts,
            })
        }
    } else if spec.is_storyboard() {
        let reusable_rows = reusable_frame_rows.unwrap_or_default();
        let mut prefix_rows = Vec::new();
        for row in reusable_rows {
            let expected = (prefix_rows.len() + 1) as i32;
            if row.shot_number == expected {
                if storyboard_prompts_from_rows(std::slice::from_ref(&row)).is_err() {
                    break;
                }
                prefix_rows.push(row);
            } else if row.shot_number > expected {
                break;
            }
        }
        let mut storyboard_prompts = storyboard_prompts_from_rows(&prefix_rows)?;
        if storyboard_prompts.len() == expected_frame_count {
            validate_storyboard_prompts(
                &storyboard_prompts,
                &segment_plan,
                &allowed_reference_ids,
            )?;
            report_video_prompt_progress(
                &mut progress,
                90,
                100,
                &format!(
                    "已复用 {} 条{}，正在写入记录",
                    prefix_rows.len(),
                    spec.reused_frame_label
                ),
            )?;
            prefix_rows
                .iter()
                .map(prompt_row_draft)
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let message = if regenerate_existing {
                spec.frame_regenerate_progress
            } else {
                spec.frame_generate_progress
            };
            if !regenerate_existing && prefix_rows.is_empty() {
                ensure_existing_frame_prompts_can_regenerate(
                    service,
                    project_id,
                    chapter_id,
                    spec.workflow_mode,
                    spec.frame_prompt_type,
                    prompt_output_language,
                )
                .await?;
            }
            let mut generated_this_run = 0usize;
            let frame_prompt_available_assets =
                frame_prompt_available_assets_context(&available_assets)?;
            while storyboard_prompts.len() < expected_frame_count
                && generated_this_run < prompt_batch_size
            {
                let index = storyboard_prompts.len();
                let unit = video_units.get(index).ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "分镜提示词缺少第 {} 个视频单元规划",
                        index + 1
                    ))
                })?;
                let frame_number = unit.unit_number;
                let segment = unit.representative_segment()?;
                let expected_segment_numbers =
                    storyboard_expected_segment_numbers(&segment_plan, frame_number)?;
                let generation_message = format!(
                    "{message}：正在生成第 {}/{} 个视频单元分镜，本批第 {}/{} 条",
                    frame_number,
                    expected_frame_count,
                    generated_this_run + 1,
                    prompt_batch_size
                );
                let frame_progress_span =
                    ProgressSpan::new(60, 90).split(index, expected_frame_count);
                report_video_prompt_progress(
                    &mut progress,
                    frame_progress_span.start(),
                    100,
                    &generation_message,
                )?;
                let current_frame = json!({
                    "frame_number": frame_number,
                    "total_frame_count": expected_frame_count,
                    "role": if unit.unit_type == VideoUnitType::Transition { "transition_storyboard" } else { "content_storyboard" },
                    "video_unit": unit,
                    "expected_segment_numbers": expected_segment_numbers,
                    "required_duration_secs": unit.duration_secs,
                    "required_aspect_ratio": &segment.aspect_ratio,
                    "previous_segment": unit.from_segment_number.and_then(|number| segment_plan.iter().find(|segment| segment.segment_number == number)),
                    "current_segment": if unit.unit_type == VideoUnitType::Content { unit.source_segments.first() } else { None },
                    "next_segment": unit.to_segment_number.and_then(|number| segment_plan.iter().find(|segment| segment.segment_number == number)),
                    "reference_slot_instruction": "后端提供的 constraints.reference_slots 顺序就是本帧成图输入顺序；frame_prompt 只能按槽位顺序写参考图 1、参考图 2，不要输出 reference_image_ids，不得写 image_id、图片 ID、available_assets、候选素材标签或原始图片数字 ID。",
                    "continuity_instruction": if unit.unit_type == VideoUnitType::Transition {
                        "这是相邻内容片段之间的过渡多格分镜板，只表达前一片段尾态到后一片段首态之间的桥接动作、空间移动或情绪转场；不得新增关键剧情或重复任一内容片段的主动作。"
                    } else {
                        "这是当前内容片段的视频单元多格分镜板，用于表达本段核心构图、动作姿态、空间关系和光影，不是首帧或尾帧。"
                    }
                });
                let frame_prompt_context = FramePromptContext {
                    prompt_engine,
                    project_id,
                    chapter_summary: source.summary,
                    chapter_analysis: source.analysis,
                    segment_plan: &segment_json,
                    available_assets: &frame_prompt_available_assets,
                    style: &style,
                    prompt_output_language,
                    aspect_ratio: &aspect_ratio,
                    reference_ids: &reference_ids,
                };
                let frame_user =
                    build_storyboard_prompt_user(frame_prompt_context, &current_frame)?;
                let frame_context_manifest =
                    build_video_context_manifest(VideoContextManifestInput {
                        prompt_engine,
                        project_id,
                        variables: &vars,
                        source: &source,
                        source_spans: &source_spans,
                        prompt_names: &["chapter_video_segment_plan", spec.frame_prompt_name],
                        previous_outputs: &[VideoContextPreviousOutput {
                            pack_kind: VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN,
                            prompt_name: "chapter_video_segment_plan",
                            parsed_output: &segment_json,
                            raw_output: None,
                            placeholder: false,
                        }],
                        project_context: project_context.as_ref(),
                        asset_context: Some(&frame_prompt_available_assets),
                        dynamic_user_content: frame_user.clone(),
                    })?;
                let frame_step_key = format!("{}:frame:{frame_number}", spec.frame_prompt_name);
                let frame_fingerprint_json = video_stage_fingerprint(
                    spec.frame_prompt_name,
                    &frame_context_manifest,
                    json!({
                        "project_id": project_id,
                        "chapter_id": chapter_id,
                        "frame_number": frame_number,
                        "frame_prompt_type": spec.frame_prompt_type,
                    }),
                );
                let frame_workflow_run_id =
                    required_llm_call_workflow_run_id(workflow_run_id, &frame_step_key)?;
                let (frame_trace, mut frame_prompt) =
                    render_or_restore_json_with_validation_with_context(
                        RenderOrRestoreJsonWithValidationInput {
                            service,
                            workflow_run_id: frame_workflow_run_id,
                            call_key: frame_step_key,
                            call_order: 300 + frame_number,
                            input_fingerprint_json: frame_fingerprint_json,
                            prompt_engine,
                            llm,
                            prompt_name: spec.frame_prompt_name,
                            project_id,
                            variables: &vars,
                            context_manifest: frame_context_manifest,
                            temperature: 0.45,
                            cancel_token,
                            step: VideoPromptStepProgress::new(
                                frame_progress_span.start(),
                                100,
                                generation_message.as_str(),
                            )
                            .with_span(frame_progress_span),
                            progress: &mut progress,
                            chunks: &mut chunks,
                            structured_progress: Some(frame_prompt_progress_spec("分镜", Some(1))),
                            checkpoint_context: "分镜提示词断点",
                        },
                        |raw| {
                            let mut frame_prompt = parse_single_video_frame_prompt(
                                raw,
                                VideoFramePromptDefaults {
                                    frame_number,
                                    duration_secs: unit.duration_secs,
                                    aspect_ratio: &aspect_ratio,
                                    segment_numbers: expected_segment_numbers.clone(),
                                    reference_image_ids: reference_image_ids.clone(),
                                },
                                prompt_output_language,
                                frame_number,
                                "分镜提示词",
                            )?;
                            apply_boundary_frame_style_template(
                                &mut frame_prompt,
                                &style,
                                prompt_output_language,
                            );
                            if frame_prompt.segment_numbers != expected_segment_numbers {
                                return Err(AppError::InvalidArgument(format!(
                                    "分镜提示词 #{} segment_numbers 必须为 {:?}，实际为 {:?}",
                                    frame_number,
                                    expected_segment_numbers,
                                    frame_prompt.segment_numbers
                                )));
                            }
                            if frame_prompt.duration_secs != unit.duration_secs {
                                return Err(AppError::InvalidArgument(format!(
                                    "分镜提示词 #{} duration_secs 必须为 {}，实际为 {}",
                                    frame_number, unit.duration_secs, frame_prompt.duration_secs
                                )));
                            }
                            if frame_prompt.aspect_ratio != segment.aspect_ratio {
                                return Err(AppError::InvalidArgument(format!(
                                    "分镜提示词 #{} aspect_ratio 必须为 {}，实际为 {}",
                                    frame_number, segment.aspect_ratio, frame_prompt.aspect_ratio
                                )));
                            }
                            ensure_video_reference_ids_known(
                                &frame_prompt.reference_image_ids,
                                &allowed_reference_ids,
                                &format!("视频分镜 #{}", frame_number),
                            )?;
                            frame_prompt.frame_number = frame_number;
                            Ok(frame_prompt)
                        },
                        |frame_prompt| serialize_video_value(frame_prompt, "分镜提示词断点"),
                    )
                    .await?;
                frame_prompt.frame_number = frame_number;
                storyboard_prompts.push(frame_prompt);
                generated_this_run += 1;

                let frame_drafts = storyboard_prompt_drafts(StoryboardPromptDraftsInput {
                    common: frame_draft_common,
                    source_prompt_type: frame_source_prompt_type,
                    video_units: &video_units,
                    storyboard_prompts: &storyboard_prompts,
                });
                let frame_drafts = attach_generation_trace_to_draft(
                    frame_drafts,
                    spec.workflow_mode,
                    spec.frame_prompt_type,
                    frame_number,
                    video_prompt_generation_trace(frame_trace, workflow_run_id),
                );
                let mut checkpoint_drafts = Vec::new();
                if let Some(draft) = shot_analysis_draft.clone() {
                    checkpoint_drafts.push(draft);
                }
                if let Some(draft) = segment_draft.clone() {
                    checkpoint_drafts.push(draft);
                }
                checkpoint_drafts.extend(frame_drafts);
                service
                    .sync_chapter_prompts(chapter_id, checkpoint_drafts)
                    .await?;
                report_video_prompt_progress(
                    &mut progress,
                    frame_progress_span.end(),
                    100,
                    &format!(
                        "分镜提示词第 {}/{} 条已保存，本批已生成 {} 条",
                        storyboard_prompts.len(),
                        expected_frame_count,
                        generated_this_run
                    ),
                )?;
            }
            if storyboard_prompts.len() == expected_frame_count {
                validate_storyboard_prompts(
                    &storyboard_prompts,
                    &segment_plan,
                    &allowed_reference_ids,
                )?;
            }
            let final_message = if storyboard_prompts.len() == expected_frame_count {
                format!(
                    "分镜提示词已全部生成，共 {}/{} 条",
                    storyboard_prompts.len(),
                    expected_frame_count
                )
            } else {
                format!(
                    "本批已生成 {} 条分镜提示词，当前共 {}/{} 条",
                    generated_this_run,
                    storyboard_prompts.len(),
                    expected_frame_count
                )
            };
            report_video_prompt_progress(&mut progress, 90, 100, &final_message)?;
            storyboard_prompt_drafts(StoryboardPromptDraftsInput {
                common: frame_draft_common,
                source_prompt_type: frame_source_prompt_type,
                video_units: &video_units,
                storyboard_prompts: &storyboard_prompts,
            })
        }
    } else {
        let validate_frame_prompts_for_spec =
            |frame_prompts: &[VideoFramePrompt]| -> Result<(), AppError> {
                validate_first_frame_prompts(frame_prompts, &segment_plan, &allowed_reference_ids)
            };
        if let Some(rows) =
            reusable_frame_rows.and_then(|rows| complete_reusable_rows(rows, expected_frame_count))
        {
            let frame_prompts = first_frame_prompts_from_rows(&rows)?;
            validate_frame_prompts_for_spec(&frame_prompts)?;
            report_video_prompt_progress(
                &mut progress,
                90,
                100,
                &format!(
                    "已复用 {} 条{}，正在写入记录",
                    rows.len(),
                    spec.reused_frame_label
                ),
            )?;
            rows.iter()
                .map(prompt_row_draft)
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let message = if regenerate_existing {
                spec.frame_regenerate_progress
            } else {
                spec.frame_generate_progress
            };
            if !regenerate_existing {
                ensure_existing_frame_prompts_can_regenerate(
                    service,
                    project_id,
                    chapter_id,
                    spec.workflow_mode,
                    spec.frame_prompt_type,
                    prompt_output_language,
                )
                .await?;
            }
            report_video_prompt_progress(&mut progress, 60, 100, message)?;
            let frame_prompt_available_assets =
                frame_prompt_available_assets_context(&available_assets)?;
            let frame_prompt_context = FramePromptContext {
                prompt_engine,
                project_id,
                chapter_summary: source.summary,
                chapter_analysis: source.analysis,
                segment_plan: &segment_json,
                available_assets: &frame_prompt_available_assets,
                style: &style,
                prompt_output_language,
                aspect_ratio: &aspect_ratio,
                reference_ids: &reference_ids,
            };
            let frame_user = build_first_frame_prompt_user(frame_prompt_context, source.content)?;
            let frame_context_manifest = build_video_context_manifest(VideoContextManifestInput {
                prompt_engine,
                project_id,
                variables: &vars,
                source: &source,
                source_spans: &source_spans,
                prompt_names: &["chapter_video_segment_plan", spec.frame_prompt_name],
                previous_outputs: &[VideoContextPreviousOutput {
                    pack_kind: VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN,
                    prompt_name: "chapter_video_segment_plan",
                    parsed_output: &segment_json,
                    raw_output: None,
                    placeholder: false,
                }],
                project_context: project_context.as_ref(),
                asset_context: Some(&frame_prompt_available_assets),
                dynamic_user_content: frame_user.clone(),
            })?;
            let frame_step_key = format!("{}:batch:first_frame", spec.frame_prompt_name);
            let frame_fingerprint_json = video_stage_fingerprint(
                spec.frame_prompt_name,
                &frame_context_manifest,
                json!({
                    "project_id": project_id,
                    "chapter_id": chapter_id,
                    "frame_prompt_type": spec.frame_prompt_type,
                    "expected_frame_count": expected_frame_count,
                }),
            );
            let frame_workflow_run_id =
                required_llm_call_workflow_run_id(workflow_run_id, &frame_step_key)?;
            let (frame_trace, frame_prompts) = render_or_restore_json_with_validation_with_context(
                RenderOrRestoreJsonWithValidationInput {
                    service,
                    workflow_run_id: frame_workflow_run_id,
                    call_key: frame_step_key,
                    call_order: 300,
                    input_fingerprint_json: frame_fingerprint_json,
                    prompt_engine,
                    llm,
                    prompt_name: spec.frame_prompt_name,
                    project_id,
                    variables: &vars,
                    context_manifest: frame_context_manifest,
                    temperature: 0.45,
                    cancel_token,
                    step: VideoPromptStepProgress::new(60, 100, message)
                        .with_span(ProgressSpan::new(60, 90)),
                    progress: &mut progress,
                    chunks: &mut chunks,
                    structured_progress: Some(frame_prompt_progress_spec(
                        "首帧",
                        Some(expected_frame_count),
                    )),
                    checkpoint_context: "首帧提示词断点",
                },
                |raw| {
                    let first_segment = segment_plan.first().ok_or_else(|| {
                        AppError::InvalidArgument("首帧串联片段规划至少需要 1 个片段".to_string())
                    })?;
                    let frame_prompts = parse_video_frame_prompts_with_defaults(
                        raw,
                        &[VideoFramePromptDefaults {
                            frame_number: 1,
                            duration_secs: first_segment.duration_secs,
                            aspect_ratio: &aspect_ratio,
                            segment_numbers: vec![first_segment.segment_number],
                            reference_image_ids: reference_image_ids.clone(),
                        }],
                        prompt_output_language,
                    )?;
                    validate_frame_prompts_for_spec(&frame_prompts)?;
                    Ok(frame_prompts)
                },
                |frame_prompts| serialize_video_value(frame_prompts, "首帧提示词断点"),
            )
            .await?;
            let frame_drafts = first_frame_prompt_drafts(FirstFramePromptDraftsInput {
                common: frame_draft_common,
                source_prompt_type: frame_source_prompt_type,
                first_frame_prompts: &frame_prompts,
            });
            let frame_drafts = attach_generation_trace_to_all_drafts(
                frame_drafts,
                video_prompt_generation_trace(frame_trace, workflow_run_id),
            );
            let mut checkpoint_drafts = Vec::new();
            if let Some(draft) = shot_analysis_draft.clone() {
                checkpoint_drafts.push(draft);
            }
            if let Some(draft) = segment_draft.clone() {
                checkpoint_drafts.push(draft);
            }
            checkpoint_drafts.extend(frame_drafts.clone());
            service
                .sync_chapter_prompts(chapter_id, checkpoint_drafts)
                .await?;
            report_video_prompt_progress(&mut progress, 90, 100, spec.frame_saved_progress)?;
            frame_drafts
        }
    };
    if !(1..=expected_frame_count as i32).any(|shot_number| {
        frame_drafts.iter().any(|draft| {
            draft.matches_identity(spec.workflow_mode, spec.frame_prompt_type, shot_number)
        })
    }) {
        return Err(AppError::InvalidArgument(format!(
            "本次{}工作流未产生任何目标记录；请检查本次生成数量、片段规划和已有提示词状态",
            spec.reused_frame_label
        )));
    }

    let mut drafts = Vec::new();
    if let Some(draft) = shot_analysis_draft {
        drafts.push(draft);
    }
    if let Some(draft) = segment_draft {
        drafts.push(draft);
    }
    drafts.extend(frame_drafts);
    let rows = if matches!(spec.layout, FramePromptLayout::BoundaryFrames) && !regenerate_existing {
        service
            .sync_chapter_prompts_preserving_generated_frame_prompts(chapter_id, drafts)
            .await?
    } else {
        service.sync_chapter_prompts(chapter_id, drafts).await?
    };
    report_video_prompt_progress(&mut progress, 100, 100, spec.final_progress)?;
    Ok(rows)
}
