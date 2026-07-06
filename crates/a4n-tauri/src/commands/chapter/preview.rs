use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::actions::chapter_generation::{
    self, FinishChapterGenerationRagContextInput, PreviewChapterPromptInput,
};
use tauri::State;

#[tauri::command]
pub async fn preview_chapter_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    input: PreviewChapterPromptInput,
) -> Result<ModelRequestPreview, String> {
    chapter_generation::preview_chapter_prompt(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn prepare_chapter_generation_rag_embedding(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    writing_notes: Option<String>,
    min_word_count: Option<i32>,
    max_word_count: Option<i32>,
) -> Result<serde_json::Value, String> {
    let output = chapter_generation::prepare_chapter_generation_rag_embedding(
        state.inner(),
        user_id,
        PreviewChapterPromptInput {
            project_id,
            chapter_number,
            writing_notes,
            min_word_count,
            max_word_count,
        },
    )
    .await?;
    serde_json::to_value(output).map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn finish_chapter_generation_rag_context(
    state: State<'_, AppState>,
    user_id: i64,
    input: FinishChapterGenerationRagContextInput,
) -> Result<serde_json::Value, String> {
    let output =
        chapter_generation::finish_chapter_generation_rag_context(state.inner(), user_id, input)
            .await?;
    serde_json::to_value(output).map_err(|err| err.to_string())
}
