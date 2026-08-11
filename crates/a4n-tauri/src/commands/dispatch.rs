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
            Ok(handle) => serde_json::to_value(handle).map_err(|error| error.to_string().into()),
            Err(error) => Err(serde_json::to_value(error)
                .unwrap_or_else(|serialize_error| serialize_error.to_string().into())),
        };
    }

    a4n_server::dispatch_runtime_command(&state, user_id, args, command)
        .await
        .map_err(Value::String)
}
