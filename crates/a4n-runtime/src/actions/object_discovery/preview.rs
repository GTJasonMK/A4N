use super::context::build_object_discovery_request;
use super::types::{ChapterAnchorMode, ObjectDiscoveryPreviewStep};
use crate::state::AppState;

pub(crate) async fn object_discovery_preview_steps(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
    initial_state: &str,
    initial_note: Option<String>,
) -> Result<Vec<ObjectDiscoveryPreviewStep>, String> {
    let context = build_object_discovery_request(
        state,
        project_id,
        chapter_number,
        ChapterAnchorMode::RequireExistingCurrent,
    )
    .await?;
    let note = match initial_note {
        Some(note) => format!("{note}；分析章节数：{}", context.chapters.len()),
        None => format!("分析章节数：{}", context.chapters.len()),
    };
    Ok(vec![ObjectDiscoveryPreviewStep {
        key: "object_discovery".to_string(),
        title: "物体档案发现 LLM 请求".to_string(),
        prompt_name: Some("object_discovery".to_string()),
        state: initial_state.to_string(),
        note: Some(note),
        request: context.request,
    }])
}
