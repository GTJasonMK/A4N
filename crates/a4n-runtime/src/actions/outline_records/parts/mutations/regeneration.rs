use crate::{
    AppState,
    actions::{
        outline_records::{
            OutlineProgressReporter,
            access::{advance_status, ensure_part_access, report},
            preview::single_request_preview,
            types::{PartOutlineSummary, RegeneratePartOutlineInput, part_outline_summary},
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
use serde_json::json;

pub async fn regenerate_part_outline(
    state: &AppState,
    user_id: i64,
    input: RegeneratePartOutlineInput,
    progress: Option<OutlineProgressReporter<'_>>,
) -> Result<PartOutlineSummary, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    report(progress, 0, 3, "正在重新生成指定分部")?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_run_id = start_outline_workflow_run(
        state,
        input.project_id,
        "part_outline_generation",
        "regenerate_one",
        &format!("part_id_{}", input.part_id),
        input.task_id.clone(),
        json!({
            "schema_version": 1,
            "workflow": "part_outline_generation",
            "mode": "regenerate_one",
            "project_id": input.project_id,
            "part_id": input.part_id,
            "variables": &variables,
        }),
    )
    .await?;
    let result = state
        .orchestrator
        .regenerate_part_outline(
            input.project_id,
            input.part_id,
            state,
            &variables,
            Some(workflow_run_id),
        )
        .await;
    let part = unwrap_outline_workflow_result(state, workflow_run_id, result).await?;
    finish_outline_postprocess(
        state,
        workflow_run_id,
        async {
            report(progress, 2, 3, "正在刷新项目状态")?;
            advance_status(state, input.project_id, ProjectStatus::PartOutlinesReady).await?;
            report(progress, 3, 3, "指定分部已重新生成")?;
            Ok(())
        }
        .await,
    )
    .await?;
    Ok(part_outline_summary(&part))
}

pub async fn preview_regenerate_part_outline_request(
    state: &AppState,
    user_id: i64,
    input: RegeneratePartOutlineInput,
) -> Result<ModelRequestPreview, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let request = state
        .orchestrator
        .preview_regenerate_part_outline_request(input.project_id, input.part_id, state, &variables)
        .await?;
    single_request_preview(
        state,
        "part_outline_regenerate_one",
        "指定分部大纲重生成请求预览",
        "regenerate_part_outline",
        format!("重新生成分部 #{}", input.part_id),
        "part_outline_single",
        request,
    )
    .await
}
