use a4n_orchestrator::portrait::{
    PortraitPromptGenerationInput, PortraitPromptRequestInput, PortraitPromptSpec,
};
use a4n_orchestrator::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

use crate::AppState;
use crate::actions::access;
use crate::actions::template_variables::parse_template_variables;
use crate::prompt_renderer::RuntimePromptRenderer;

use super::fingerprint::{portrait_prompt_business_key, portrait_prompt_workflow_fingerprint};
use super::rules::portrait_profile_context;

pub async fn generate_portrait_prompt(
    state: &AppState,
    user_id: i64,
    input: super::super::GeneratePortraitPromptInput,
) -> Result<String, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(input.variables.as_ref())?;
    let rows = state
        .orchestrator
        .portrait_prompt_profile_rows(input.project_id, input.profile_id)
        .await
        .map_err(|err| err.to_string())?;
    let profile = rows.profile;
    let profile_context = portrait_profile_context(&profile);
    let project_context = rows.project_context;
    let spec = PortraitPromptSpec {
        profile_name: &profile.name,
        appearance: profile.appearance.as_deref(),
        gender: profile.gender.as_deref(),
        age: profile.age.as_deref(),
        personality: profile.personality.as_deref(),
        background: profile.background.as_deref(),
        profile_context: Some(&profile_context),
        project_context: project_context.as_ref(),
        portrait_type: &input.portrait_type,
        art_style: None,
        expression: None,
        existing_portrait_count: None,
    };
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let request = state
        .build_generate_prompt_request(PortraitPromptRequestInput {
            project_id: input.project_id,
            spec,
            prompt_renderer: &prompt_renderer,
            variables: &variables,
        })
        .await
        .map_err(|e| e.to_string())?;
    let workflow_fingerprint = portrait_prompt_workflow_fingerprint(
        &input,
        &profile,
        &profile_context,
        project_context.as_ref(),
        &request,
    )?;
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: portrait_prompt_business_key(&workflow_fingerprint)?,
            workflow_kind: "portrait_prompt_generation".to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(input.profile_id),
            chapter_id: None,
            mode: Some(input.portrait_type.clone()),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("portrait_prompt.generate".to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = state
        .generate_prompt_with_checkpoint(PortraitPromptGenerationInput {
            project_id: input.project_id,
            spec,
            prompt_renderer: &prompt_renderer,
            variables: &variables,
            workflow_run_id: Some(workflow_run_id),
        })
        .await
        .map(|prompt| prompt.trim().to_string())
        .map_err(|e| e.to_string());

    super::super::workflow::finalize_portrait_workflow(state, workflow_run_id, &result)
        .await
        .map_err(|status_err| match result.as_ref() {
            Ok(_) => format!("人物三视图提示词生成成功，但工作流状态写入失败: {status_err}"),
            Err(err) => format!("{err}；工作流失败状态写入失败: {status_err}"),
        })?;
    result
}
