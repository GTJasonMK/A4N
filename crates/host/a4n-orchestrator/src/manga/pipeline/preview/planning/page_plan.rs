use a4n_domain::manga::MangaPagePlanV4;
use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::adaptation_projection::manga_page_planning_source;
use super::super::super::json::parse_manga_page_plan;
use super::super::super::prompt::manga_page_planning_user_message;
use super::super::call::build_manga_preview_call;
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) async fn preview_page_plan(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    extraction_summary: &Value,
) -> Result<Option<MangaPagePlanV4>, AppError> {
    if context.stop_after != Some(a4n_api_contract::manga::MangaPipelineStopAfter::PagePlanning)
        && let Some(record) = a4n_db::media_authoring::editable_authoring_document(
            context.service.database.connection(),
            context.project_id,
            context.adaptation.record.source_snapshot_id,
            Some(context.chapter_id),
            a4n_domain::media_authoring::AuthoringDocumentKind::MangaPagePlan,
        )
        .await
        .map_err(crate::media_authoring::authoring_error)?
        && record.inputs.sources == context.adaptation.record.inputs.sources
        && record.inputs.dependencies == [context.adaptation.record.reference()]
    {
        let plan =
            parse_manga_page_plan(&record.document, &context.adaptation, "采用保存的分页计划")?;
        steps.push(MangaPipelineRequestPreviewStep::reused(
            "manga_page_planning",
            "漫画分页分镜",
            "manga_page_planning",
            "使用已保存的本地分页计划；草稿会在继续时采用。".into(),
        ));
        return Ok(Some(plan));
    }
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_page_planning",
        workflow_run_id: context.workflow_run_id,
        call_key: format!("manga_pipeline:{}:manga_page_planning", context.chapter_id),
        call_order: 6,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content: manga_page_planning_user_message(
            extraction_summary.clone(),
            manga_page_planning_source(&context.adaptation),
        )?,
        temperature: 0.3,
    })
    .await?;
    let raw_plan_json = match call {
        MangaPreviewCall::Restored { parsed_output, .. } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                "manga_page_planning",
                "漫画 Step 6：分页规划",
                "manga_page_planning",
            ));
            parsed_output
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                "manga_page_planning",
                "漫画 Step 6：分页规划",
                "manga_page_planning",
                None,
                *request,
            ));
            return Ok(None);
        }
    };
    let plan = parse_manga_page_plan(&raw_plan_json, &context.adaptation, "manga_page_planning")?;
    Ok(Some(plan))
}
