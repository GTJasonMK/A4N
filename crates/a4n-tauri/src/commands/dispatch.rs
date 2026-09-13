use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn invoke_host_command(
    state: State<'_, a4n_runtime::AppState>,
    command: String,
    args: Value,
    user_id: i64,
) -> Result<Value, Value> {
    if let Some(result) =
        a4n_server::dispatch_generation_start(&state, user_id, &args, &command).await
    {
        return match result {
            Ok(handle) => Ok(serde_json::to_value(handle)
                .expect("generation task handle must serialize to JSON")),
            Err(error) => {
                Err(serde_json::to_value(error)
                    .expect("generation host error must serialize to JSON"))
            }
        };
    }

    a4n_server::dispatch_runtime_command_value(&state, user_id, args, command).await
}
