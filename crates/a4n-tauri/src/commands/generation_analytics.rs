use a4n_runtime::actions::generation_analytics::{
    GenerationCallAnalytics, GenerationCallAnalyticsInput,
    get_generation_call_analytics_for_admin as get_generation_call_analytics_action,
};
use tauri::State;

#[tauri::command]
pub async fn get_generation_call_analytics(
    state: State<'_, a4n_runtime::AppState>,
    caller_user_id: i64,
    input: GenerationCallAnalyticsInput,
) -> Result<GenerationCallAnalytics, String> {
    get_generation_call_analytics_action(&state, caller_user_id, input)
        .await
        .map_err(|err| err.to_string())
}
