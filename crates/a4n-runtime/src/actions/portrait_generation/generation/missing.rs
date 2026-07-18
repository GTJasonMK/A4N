use super::*;
use serde_json::json;

pub async fn generate_missing_portraits(
    state: &AppState,
    user_id: i64,
    input: GenerateMissingPortraitsInput,
    mut progress: Option<PortraitProgress<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<PortraitSummary>, String> {
    generate_missing_portraits_task_scope(state, user_id, &input).await?;
    let missing_profiles = state
        .orchestrator
        .missing_portrait_profiles(input.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let portrait_count = u32::try_from(missing_profiles.len())
        .map_err(|_| "待生成人物三视图拆解数量超出 u32 范围".to_string())?;
    i32::try_from(missing_profiles.len())
        .map_err(|_| "待生成人物三视图拆解数量超出 checkpoint 序号范围".to_string())?;
    let workflow_fingerprint = json!({
        "workflow": "missing_portrait_generation",
        "project_id": input.project_id});
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!("missing_portrait_generation:{}", input.project_id),
            workflow_kind: "missing_portrait_generation".to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_project".to_string()),
            scope_id: Some(input.project_id),
            chapter_id: None,
            mode: Some("missing".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("missing_portrait_generation.items".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        let total = portrait_count.max(1);
        let mut results = Vec::new();
        for (index, profile) in missing_profiles.iter().enumerate() {
            let display_index = index + 1;
            let current = u32::try_from(index)
                .map_err(|_| "人物三视图拆解进度序号超出 u32 范围".to_string())?;
            let next_current = u32::try_from(display_index)
                .map_err(|_| "人物三视图拆解进度序号超出 u32 范围".to_string())?;
            let step_order = i32::try_from(display_index)
                .map_err(|_| "人物三视图拆解 checkpoint 序号超出 i32 范围".to_string())?;
            check_cancelled(&cancel_token)?;
            report_progress(
                &mut progress,
                current,
                total,
                &format!(
                    "正在生成人物三视图拆解 {display_index}/{}：{}",
                    missing_profiles.len(),
                    profile.name
                ),
            )?;
            let prompt = profile_portrait_seed_prompt(profile)?;
            let step_key = format!("missing_portrait_generation.profile.{}", profile.id);
            let item_fingerprint =
                missing_portrait_item_fingerprint(input.project_id, profile, &prompt);
            if let Some(summary) = load_completed_portrait_final_checkpoint(
                state,
                workflow_run_id,
                &step_key,
                &item_fingerprint,
                input.project_id,
            )
            .await?
            {
                results.push(summary);
                report_progress(
                    &mut progress,
                    next_current,
                    total,
                    &format!(
                        "已从断点恢复人物三视图拆解 {display_index}/{}：{}",
                        missing_profiles.len(),
                        profile.name
                    ),
                )?;
                continue;
            }
            let generated = run_portrait_image_generation(
                state,
                input.project_id,
                &prompt,
                &[],
                Some(workflow_run_id),
                cancel_token.clone(),
            )
            .await?;
            check_cancelled(&cancel_token)?;
            let portrait = persist_generated_portrait(
                state,
                &generated,
                PersistGeneratedPortraitInput {
                    workflow_run_id: Some(workflow_run_id),
                    profile_id: profile.id,
                    project_id: input.project_id,
                    portrait_type: "half_body",
                    prompt: &prompt,
                    expression: None,
                    pose: None,
                    is_primary: true,
                    character_name: &profile.name,
                    variant_kind: None,
                    auto_generated: true,
                    path_context: "缺失人物三视图拆解",
                    reference_image_ids: &[],
                },
            )
            .await?;
            state
                .orchestrator
                .save_completed_workflow_checkpoint(CompletedWorkflowCheckpointInput {
                    workflow_run_id: Some(workflow_run_id),
                    step_key,
                    step_order,
                    input_fingerprint_json: item_fingerprint,
                    checkpoint_json: json!({
                        "portrait_id": portrait.id,
                        "image_path": portrait.image_path,
                        "summary": to_summary(&portrait)}),
                })
                .await
                .map_err(|error| error.to_string())?;
            results.push(to_summary(&portrait));
            report_progress(
                &mut progress,
                next_current,
                total,
                &format!(
                    "已生成人物三视图拆解 {display_index}/{}：{}",
                    missing_profiles.len(),
                    profile.name
                ),
            )?;
        }
        Ok(results)
    }
    .await;

    if let Err(status_err) = finalize_portrait_workflow(state, workflow_run_id, &result).await {
        return match result {
            Ok(_) => Err(format!(
                "缺失立绘生成完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}
