use crate::actions::access;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::prompt_renderer::RuntimePromptRenderer;
use crate::state::AppState;
use a4n_orchestrator::import_analysis::chapter_outline::importable_chapters;

pub async fn preview_import_project_analysis_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let chapters = importable_chapters(&state.orchestrator, project_id).await?;
    if chapters.is_empty() {
        return Err("项目没有可用于导入分析的章节正文".to_string());
    }

    let prompt_renderer = RuntimePromptRenderer::new(state);
    let mut steps = Vec::with_capacity(chapters.len() * 4 + 2);
    for chapter in chapters {
        let requests = state
            .orchestrator
            .build_chapter_analysis_requests(chapter.id, &prompt_renderer)
            .await
            .map_err(|e| e.to_string())?;
        for request in requests {
            steps.push(ChatStatusPreviewStep::llm(
                format!("import_chapter_analysis_{}_{}", chapter.id, request.key),
                format!("第 {} 章{}", chapter.chapter_number, request.title),
                Some(request.prompt_name),
                "ready",
                Some("导入项目分析的分析阶段会按顺序发送该请求".to_string()),
                Some(request.request),
            ));
        }
    }
    steps.push(ChatStatusPreviewStep::llm(
        "import_analysis_character_sync",
        "导入章节角色发现和角色同步请求",
        Some("character_discovery".to_string()),
        "blocked",
        Some("这些请求依赖对应章节结构化分析输出，运行前无法提前构造真实请求".to_string()),
        None,
    ));
    steps.push(ChatStatusPreviewStep::llm(
        "import_analysis_reverse_blueprint",
        "导入蓝图反推请求",
        Some("reverse_blueprint".to_string()),
        "blocked",
        Some("该请求依赖前序章节分析和大纲更新后的上下文，运行前无法提前构造真实请求".to_string()),
        None,
    ));
    chat_status_preview_for_state(
        state,
        "import_project_analysis",
        "导入项目分析请求预览",
        steps,
        "llm_chat",
    )
    .await
}
