use crate::{
    AppState,
    actions::{
        access as project_access,
        outline_records::{
            OutlineProgressReporter,
            access::{advance_status, report},
            preview::single_request_preview,
            types::{ContinuePartOutlinesInput, PartOutlineSummary, part_outline_summary},
            workflow::{
                finish_outline_postprocess, start_outline_workflow_run,
                unwrap_outline_workflow_result,
            },
        },
        request_preview::ModelRequestPreview,
        template_variables::parse_template_variables_app_error as parse_template_variables,
    },
};
use a4n_core::{error::AppError, state::status::ProjectStatus};
use a4n_orchestrator::outline::PartOutlineContinuationInput;
use serde_json::json;

pub async fn continue_part_outlines(
    state: &AppState,
    user_id: i64,
    input: ContinuePartOutlinesInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<PartOutlineSummary, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    report(
        progress,
        0,
        3,
        &format!("正在续作第 {} 部", input.part_number),
    )?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "part_outline_generation",
        "continue",
        &format!("part_{}", input.part_number),
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "part_outline_generation",
            "mode": "continue",
            "project_id": input.project_id,
            "part_number": input.part_number,
            "start_chapter": input.start_chapter,
            "end_chapter": input.end_chapter,
            "variables": &variables,
        }),
    )
    .await?;
    let result = state
        .orchestrator
        .continue_part_outline(PartOutlineContinuationInput {
            project_id: input.project_id,
            part_number: input.part_number,
            start_chapter: input.start_chapter,
            end_chapter: input.end_chapter,
            prompt_renderer: state,
            variables: &variables,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    let part = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        async {
            report(progress, 2, 3, "正在刷新项目状态")?;
            advance_status(state, input.project_id, ProjectStatus::PartOutlinesReady).await?;
            report(progress, 3, 3, "分部续作完成")?;
            Ok(())
        }
        .await,
    )
    .await?;
    Ok(part_outline_summary(&part))
}

pub async fn preview_continue_part_outlines_request(
    state: &AppState,
    user_id: i64,
    input: ContinuePartOutlinesInput,
) -> Result<ModelRequestPreview, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let request = state
        .orchestrator
        .preview_continue_part_outline_request(
            input.project_id,
            input.part_number,
            input.start_chapter,
            input.end_chapter,
            state,
            &variables,
        )
        .await?;
    single_request_preview(
        state,
        "part_outline_continue",
        format!("第 {} 部续作请求预览", input.part_number),
        "continue_part_outline",
        format!("续作第 {} 部", input.part_number),
        "part_outline_single",
        request,
    )
    .await
}
