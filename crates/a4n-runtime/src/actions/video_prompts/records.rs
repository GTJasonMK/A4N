use super::*;

pub async fn list_chapter_video_prompts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    validate_project_chapter_video_source(state, project_id, chapter_id, "查看视频提示词", false)
        .await?;
    let rows = state
        .orchestrator
        .chapter_video_prompts_for_chapter(project_id, chapter_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(rows.iter().map(prompt_to_summary).collect())
}

pub async fn update_chapter_video_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    input: UpdateChapterVideoPromptInput,
) -> Result<ChapterVideoPromptSummary, String> {
    let prompt = state
        .orchestrator
        .chapter_video_prompt(prompt_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    access::verify_project_editor_string(state, user_id, prompt.project_id).await?;
    validate_project_chapter_video_source(
        state,
        prompt.project_id,
        prompt.chapter_id,
        "更新视频提示词",
        true,
    )
    .await?;
    if let Some(Some(reference_image_ids_json)) = input.reference_image_ids_json.as_ref() {
        verify_video_reference_image_project_access(
            state,
            user_id,
            prompt.project_id,
            prompt.chapter_id,
            Some(reference_image_ids_json),
        )
        .await?;
    }
    let mutation_plan = prepare_video_prompt_mutation_plan(
        state,
        prompt_id,
        input.downstream_strategy,
        "更新视频提示词",
    )
    .await?;
    apply_video_prompt_downstream_deletions(state, user_id, &mutation_plan).await?;
    let row = state
        .orchestrator
        .update_chapter_video_prompt_with_policy(
            prompt_id,
            DbUpdateChapterVideoPromptInput {
                visual_prompt: input.visual_prompt,
                motion_prompt: input.motion_prompt,
                negative_prompt: input.negative_prompt,
                duration_secs: input.duration_secs,
                reference_image_ids_json: input.reference_image_ids_json,
            },
            mutation_plan.allow_downstream,
        )
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(prompt_to_summary(&row))
}

pub async fn delete_chapter_video_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    let prompt = state
        .orchestrator
        .chapter_video_prompt(prompt_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    access::verify_project_editor_string(state, user_id, prompt.project_id).await?;
    validate_project_chapter_video_source(
        state,
        prompt.project_id,
        prompt.chapter_id,
        "删除视频提示词",
        true,
    )
    .await?;
    let mutation_plan =
        prepare_video_prompt_mutation_plan(state, prompt_id, downstream_strategy, "删除视频提示词")
            .await?;
    apply_video_prompt_downstream_deletions(state, user_id, &mutation_plan).await?;
    state
        .orchestrator
        .delete_chapter_video_prompt_checked(prompt_id, mutation_plan.allow_downstream)
        .await
        .map_err(crate::actions::app_error_message)
}

pub async fn delete_chapter_video_prompt_group(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    group: &str,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<DeleteChapterVideoPromptGroupResult, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    validate_project_chapter_video_source(state, project_id, chapter_id, "删除视频提示词", true)
        .await?;

    let rows = state
        .orchestrator
        .chapter_video_prompts_for_chapter_by_id(project_id, chapter_id)
        .await
        .map_err(crate::actions::app_error_message)?;

    let target_rows = video_prompt_delete_group_rows(&rows, group)?;

    if target_rows.is_empty() {
        return Ok(DeleteChapterVideoPromptGroupResult {
            deleted_prompt_count: 0,
            deleted_image_count: 0,
        });
    }
    if let Some(row) = target_rows.iter().find(|row| row.status == "running") {
        return Err(format!("视频提示词 #{} 正在运行，不能批量删除", row.id));
    }

    let target_prompt_ids = target_rows.iter().map(|row| row.id).collect::<Vec<_>>();
    let mutation_plan = prepare_video_prompt_group_mutation_plan(
        state,
        &target_prompt_ids,
        downstream_strategy,
        "批量删除视频提示词",
    )
    .await?;
    let deleted_image_count = mutation_plan
        .delete_dependencies
        .as_ref()
        .map(|dependencies| dependencies.image_ids.len() as u64)
        .unwrap_or_default();
    apply_video_prompt_downstream_deletions(state, user_id, &mutation_plan).await?;
    let deleted_prompt_count = state
        .orchestrator
        .delete_chapter_video_prompts_checked(&target_prompt_ids, mutation_plan.allow_downstream)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(DeleteChapterVideoPromptGroupResult {
        deleted_prompt_count,
        deleted_image_count,
    })
}
