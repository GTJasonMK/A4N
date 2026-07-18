use crate::AppState;
use crate::actions::access as project_access;
use crate::actions::outline_records::OutlineProgressReporter;
use crate::actions::outline_records::access::{advance_status, report};
use crate::actions::outline_records::types::{
    GeneratePartsInput, PartOutlineSummary, RegenerateLastPartInput, RegeneratePartsInput,
    part_outline_summary,
};
use crate::actions::outline_records::workflow::{
    finish_outline_postprocess, start_outline_workflow_run, unwrap_outline_workflow_result,
};
use crate::actions::template_variables::parse_template_variables_app_error as parse_template_variables;
use a4n_core::error::AppError;
use a4n_core::state::status::ProjectStatus;
use a4n_orchestrator::outline::{PartOutlineGenerationInput, PartOutlineRegenerationInput};
use serde_json::json;

pub async fn generate_parts(
    state: &AppState,
    user_id: i64,
    input: GeneratePartsInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<Vec<PartOutlineSummary>, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    report(progress, 0, 3, "启动分部大纲生成")?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "part_outline_generation",
        "generate",
        "part_1",
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "part_outline_generation",
            "mode": "generate",
            "project_id": input.project_id,
            "part_count": input.part_count,
            "chapters_per_part": input.chapters_per_part,
            "variables": &variables,
        }),
    )
    .await?;
    let result = state
        .orchestrator
        .generate_parts(PartOutlineGenerationInput {
            project_id: input.project_id,
            part_count: input.part_count,
            chapters_per_part: input.chapters_per_part,
            prompt_renderer: state,
            variables: &variables,
            progress,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    let parts = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        async {
            advance_status(state, input.project_id, ProjectStatus::PartOutlinesReady).await?;
            report(progress, 3, 3, "分部大纲已生成")?;
            Ok(())
        }
        .await,
    )
    .await?;
    Ok(parts.iter().map(part_outline_summary).collect())
}

pub async fn regenerate_parts(
    state: &AppState,
    user_id: i64,
    input: RegeneratePartsInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<Vec<PartOutlineSummary>, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    report(progress, 0, 3, "启动分部大纲重生成")?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "part_outline_generation",
        "regenerate_all",
        "part_1",
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "part_outline_generation",
            "mode": "regenerate_all",
            "project_id": input.project_id,
            "part_count": input.part_count,
            "chapters_per_part": input.chapters_per_part,
            "variables": &variables,
        }),
    )
    .await?;
    let result = state
        .orchestrator
        .regenerate_parts(PartOutlineRegenerationInput {
            project_id: input.project_id,
            part_count: input.part_count,
            chapters_per_part: input.chapters_per_part,
            prompt_renderer: state,
            variables: &variables,
            progress,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    let parts = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        async {
            advance_status(state, input.project_id, ProjectStatus::PartOutlinesReady).await?;
            report(progress, 3, 3, "已重新生成所有部")?;
            Ok(())
        }
        .await,
    )
    .await?;
    Ok(parts.iter().map(part_outline_summary).collect())
}

pub async fn regenerate_last_part(
    state: &AppState,
    user_id: i64,
    input: RegenerateLastPartInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<PartOutlineSummary, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    report(progress, 0, 3, "正在重新生成最后一部")?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "part_outline_generation",
        "regenerate_last",
        "part_latest",
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "part_outline_generation",
            "mode": "regenerate_last",
            "project_id": input.project_id,
            "variables": &variables,
        }),
    )
    .await?;
    let result = state
        .orchestrator
        .regenerate_last_part(input.project_id, state, &variables, Some(workflow_run_id))
        .await;
    let part = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        async {
            report(progress, 2, 3, "正在刷新项目状态")?;
            advance_status(state, input.project_id, ProjectStatus::PartOutlinesReady).await?;
            report(progress, 3, 3, "最后一部已重新生成")?;
            Ok(())
        }
        .await,
    )
    .await?;
    Ok(part_outline_summary(&part))
}
