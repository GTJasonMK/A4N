mod image;
mod prompt;

use a4n_orchestrator::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use tokio_util::sync::CancellationToken;

use super::super::prompt::GeneratePortraitPromptRequestContext;
use super::super::workflow::{
    finalize_portrait_workflow, portrait_generate_business_key,
    portrait_generate_workflow_fingerprint,
};
use super::super::{
    AppState, GeneratePortraitInput, PortraitProgress, PortraitSummary,
    build_generate_portrait_prompt_request,
};
use super::{check_cancelled, generate_portrait_task_scope, report_progress};
use image::{InitialPortraitImageContext, generate_and_persist_initial_portrait};
use prompt::resolve_initial_portrait_prompt;

pub async fn generate_portrait(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitInput,
    mut progress: Option<PortraitProgress<'_>>,
    cancel_token: CancellationToken,
) -> Result<PortraitSummary, String> {
    generate_portrait_task_scope(state, user_id, &input).await?;
    report_progress(&mut progress, 0, 6, "正在读取角色档案")?;
    check_cancelled(&cancel_token)?;
    let GeneratePortraitPromptRequestContext {
        profile,
        existing_count,
        expression,
        request,
    } = build_generate_portrait_prompt_request(
        state,
        input.project_id,
        &input.character_name,
        input.style.as_deref(),
        input.expressions.as_deref(),
        input.pose.as_deref(),
    )
    .await?;
    let workflow_fingerprint =
        portrait_generate_workflow_fingerprint(&input, &profile, &expression, &request)?;
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: portrait_generate_business_key(&workflow_fingerprint)?,
            workflow_kind: "portrait_generation".to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(profile.id),
            chapter_id: None,
            mode: Some("full_body".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("portrait_generation.prompt".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        let prompt = resolve_initial_portrait_prompt(
            state,
            workflow_run_id,
            &workflow_fingerprint,
            request,
            &mut progress,
            &cancel_token,
        )
        .await?;
        generate_and_persist_initial_portrait(
            state,
            InitialPortraitImageContext {
                input: &input,
                profile: &profile,
                existing_count,
                expression: &expression,
                workflow_run_id,
                workflow_fingerprint,
                prompt,
            },
            &mut progress,
            &cancel_token,
        )
        .await
    }
    .await;

    if let Err(status_err) = finalize_portrait_workflow(state, workflow_run_id, &result).await {
        return match result {
            Ok(_) => Err(format!(
                "人物三视图拆解生成完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}
