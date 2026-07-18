use super::*;

pub(super) async fn prepare_chapter_video_prompt_preview(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    source_operation: &str,
    workflow_operation: &str,
    chat_duration_note: &str,
) -> Result<PreparedChapterVideoPromptPreview, String> {
    chapter_video_prompt_task_scope(state, &input).await?;
    let user_id = input.user_id;
    let project_id = input.project_id;
    let chapter_id = input.chapter_id;
    let source_bundle = resolve_project_chapter_video_source(
        state,
        project_id,
        chapter_id,
        source_operation,
        false,
    )
    .await?;
    let mut chapter = source_bundle.chapter;
    let mut source = source_bundle.source;
    let mut placeholder_notes = prepare_chapter_prompt_preview_inputs(&mut chapter, &mut source);
    let workflow_mode = ensure_video_workflow_mode(
        &input.options,
        CHAPTER_VIDEO_PROMPT_WORKFLOW_MODES,
        workflow_operation,
    )?;
    append_prompt_option_preview_notes(&input.options, &mut placeholder_notes);
    let mut prompt_options =
        validate_generate_prompt_options_for_preview(input.options, &workflow_mode)?;
    if workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        placeholder_notes.push(chat_duration_note.to_string());
    } else {
        apply_default_duration_capability_for_preview(
            state,
            &mut prompt_options,
            &mut placeholder_notes,
        )
        .await?;
    }
    verify_video_reference_image_project_access_for_ids(
        state,
        user_id,
        project_id,
        chapter_id,
        &prompt_options.reference_image_ids,
    )
    .await?;
    let vars = build_video_prompt_vars(project_id, chapter_id, &chapter, &source, &prompt_options)?;
    Ok(PreparedChapterVideoPromptPreview {
        project_id,
        chapter_id,
        workflow_mode,
        source,
        vars,
        prompt_options,
        placeholder_notes,
    })
}

pub(super) async fn prepare_chapter_video_prompt_generation(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: &mut Option<VideoPromptProgress<'_>>,
    cancel_token: &CancellationToken,
    spec: ChapterVideoPromptGenerationSpec,
) -> Result<PreparedChapterVideoPromptGeneration, String> {
    report_progress(progress, 0, 100, "正在检查章节和生成参数")?;
    check_cancelled(cancel_token, spec.initial_stop_message)?;
    chapter_video_prompt_task_scope(state, &input).await?;
    let user_id = input.user_id;
    let project_id = input.project_id;
    let chapter_id = input.chapter_id;
    let context =
        resolve_project_chapter_video_source(state, project_id, chapter_id, spec.operation, true)
            .await?;
    let chapter = context.chapter;
    let source = context.source;
    if source.content.trim().is_empty() {
        return Err(spec.empty_content_message.to_string());
    }
    ensure_no_running_video_prompts(state, project_id, chapter_id, spec.running_message).await?;
    let workflow_mode = ensure_video_workflow_mode(
        &input.options,
        CHAPTER_VIDEO_PROMPT_WORKFLOW_MODES,
        spec.workflow_operation,
    )?;
    let mut prompt_options = validate_generate_prompt_options(input.options, &workflow_mode)?;
    if workflow_mode != VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        apply_default_duration_capability(state, &mut prompt_options).await?;
    }
    verify_video_reference_image_project_access_for_ids(
        state,
        user_id,
        project_id,
        chapter_id,
        &prompt_options.reference_image_ids,
    )
    .await?;
    let workflow_fingerprint = video_prompt_workflow_fingerprint(
        spec.workflow_kind,
        project_id,
        chapter_id,
        &workflow_mode,
        &source,
        &prompt_options,
    );
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!(
                "video_prompt:{}:{project_id}:{chapter_id}:{workflow_mode}",
                spec.workflow_kind
            ),
            workflow_kind: spec.workflow_kind.to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: Some(chapter_id),
            chapter_id: Some(chapter_id),
            mode: Some(workflow_mode.clone()),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(spec.workflow_step_key.to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    prompt_options.workflow_run_id = Some(workflow_run.id);
    let vars = build_video_prompt_vars(project_id, chapter_id, &chapter, &source, &prompt_options)?;
    Ok(PreparedChapterVideoPromptGeneration {
        project_id,
        chapter_id,
        workflow_mode,
        source,
        vars,
        prompt_options,
        workflow_run_id: workflow_run.id,
    })
}
