use crate::{
    actions::{
        access,
        request_preview::{
            ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
        },
    },
    state::AppState,
};

pub async fn preview_import_project_analysis_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let prompt_renderer = state;
    let steps = state
        .orchestrator()
        .prepare_import_project_analysis_preview_validated(project_id, prompt_renderer)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.status,
                step.note,
                step.request,
            )
        })
        .collect::<Vec<_>>();
    chat_status_preview_for_state(
        state,
        "import_project_analysis",
        "导入项目分析请求预览",
        steps,
        "llm_chat",
    )
    .await
}
