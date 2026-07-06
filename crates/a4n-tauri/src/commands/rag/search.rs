use a4n_runtime::AppState;
use a4n_runtime::actions::rag_search::{
    EnhancedSearchInput, RagEnhancedSearchOutput, RagEnhancedSearchWithVectorsInput,
    RagPreparedEnhancedSearchOutput,
};
use tauri::State;

#[tauri::command]
pub async fn rag_enhanced_search(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: EnhancedSearchInput,
) -> Result<RagEnhancedSearchOutput, String> {
    a4n_runtime::actions::rag_search::rag_enhanced_search(state.inner(), user_id, project_id, input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn rag_prepare_enhanced_search(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: EnhancedSearchInput,
) -> Result<RagPreparedEnhancedSearchOutput, String> {
    a4n_runtime::actions::rag_search::rag_prepare_enhanced_search(
        state.inner(),
        user_id,
        project_id,
        input,
    )
    .await
    .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn rag_enhanced_search_with_vectors(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: RagEnhancedSearchWithVectorsInput,
) -> Result<RagEnhancedSearchOutput, String> {
    a4n_runtime::actions::rag_search::rag_enhanced_search_with_vectors(
        state.inner(),
        user_id,
        project_id,
        input,
    )
    .await
    .map_err(|err| err.to_string())
}
