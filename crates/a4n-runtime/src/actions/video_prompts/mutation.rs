use super::*;

pub(super) async fn prepare_video_prompt_mutation_plan(
    state: &AppState,
    prompt_id: i64,
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<VideoPromptMutationPlan, String> {
    let dependencies = state
        .orchestrator
        .prompt_downstream_dependencies(prompt_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    video_prompt_mutation_plan_from_dependencies(dependencies, strategy, operation, "该视频提示词")
}

pub(super) async fn prepare_video_prompt_group_mutation_plan(
    state: &AppState,
    prompt_ids: &[i64],
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<VideoPromptMutationPlan, String> {
    let target_prompt_ids = prompt_ids.iter().copied().collect::<BTreeSet<_>>();
    let mut merged = VideoPromptDownstreamDependencies::default();
    let mut downstream_prompt_ids = BTreeSet::<i64>::new();
    let mut downstream_image_ids = BTreeSet::<i64>::new();
    let mut downstream_video_ids = BTreeSet::<i64>::new();
    for prompt_id in prompt_ids {
        let dependencies = state
            .orchestrator
            .prompt_downstream_dependencies(*prompt_id)
            .await
            .map_err(crate::actions::app_error_message)?;
        downstream_prompt_ids.extend(
            dependencies
                .prompt_ids
                .into_iter()
                .filter(|id| !target_prompt_ids.contains(id)),
        );
        downstream_image_ids.extend(dependencies.image_ids);
        downstream_video_ids.extend(dependencies.video_ids);
    }
    merged.prompt_ids = downstream_prompt_ids.into_iter().collect();
    merged.image_ids = downstream_image_ids.into_iter().collect();
    merged.video_ids = downstream_video_ids.into_iter().collect();

    video_prompt_mutation_plan_from_dependencies(merged, strategy, operation, "这些视频提示词")
}

pub(super) fn video_prompt_mutation_plan_from_dependencies(
    dependencies: VideoPromptDownstreamDependencies,
    strategy: DownstreamConflictStrategy,
    operation: &str,
    subject: &str,
) -> Result<VideoPromptMutationPlan, String> {
    if dependencies.is_empty() {
        return Ok(VideoPromptMutationPlan {
            allow_downstream: false,
            delete_dependencies: None,
        });
    }

    match strategy {
        DownstreamConflictStrategy::Reject => {
            let downstream_count = dependencies.prompt_ids.len()
                + dependencies.image_ids.len()
                + dependencies.video_ids.len();
            Err(format!(
                "{subject}已有 {downstream_count} 个下游派生产物，不能直接{operation}；请选择删除下游后继续，或保留下游继续"
            ))
        }
        DownstreamConflictStrategy::KeepDownstream => Ok(VideoPromptMutationPlan {
            allow_downstream: true,
            delete_dependencies: None,
        }),
        DownstreamConflictStrategy::DeleteDownstream => Ok(VideoPromptMutationPlan {
            allow_downstream: false,
            delete_dependencies: Some(dependencies),
        }),
    }
}

pub(super) async fn apply_video_prompt_downstream_deletions(
    state: &AppState,
    user_id: i64,
    plan: &VideoPromptMutationPlan,
) -> Result<(), String> {
    let Some(dependencies) = plan.delete_dependencies.as_ref() else {
        return Ok(());
    };

    for video_id in &dependencies.video_ids {
        video_records::delete_video(state, user_id, *video_id).await?;
    }
    for image_id in &dependencies.image_ids {
        image_records::delete_image(state, user_id, *image_id).await?;
    }
    if !dependencies.prompt_ids.is_empty() {
        state
            .orchestrator
            .delete_chapter_video_prompts_checked(&dependencies.prompt_ids, true)
            .await
            .map_err(crate::actions::app_error_message)?;
    }
    Ok(())
}

pub(super) fn is_generated_frame_prompt_type(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt" | "storyboard_prompt"
    )
}

pub(super) fn video_prompt_delete_group_rows(
    rows: &[chapter_video_prompt::Model],
    group: &str,
) -> Result<Vec<chapter_video_prompt::Model>, String> {
    match group {
        "video_prompts" => Ok(rows
            .iter()
            .filter(|row| row.prompt_type == "video_prompt")
            .cloned()
            .collect()),
        "frame_prompts" => Ok(rows
            .iter()
            .filter(|row| is_generated_frame_prompt_type(&row.prompt_type))
            .cloned()
            .collect()),
        "all_chain_prompts" => Ok(rows.to_vec()),
        other => Err(format!(
            "未知视频提示词删除分组：{other}，支持 video_prompts、frame_prompts 或 all_chain_prompts"
        )),
    }
}

pub(super) async fn validate_project_chapter_video_source(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    operation: &str,
    writable: bool,
) -> Result<(), String> {
    resolve_project_chapter_video_source(state, project_id, chapter_id, operation, writable)
        .await
        .map(|_| ())
}
