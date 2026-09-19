use a4n_error::AppError;
use serde_json::{Value, json};

const MAX_OBSERVATION_BYTES: usize = 16 * 1024;

pub(super) fn bounded_observation(execution_id: i64, value: Value) -> Result<Value, AppError> {
    let size = serde_json::to_vec(&value)?.len();
    if size <= MAX_OBSERVATION_BYTES {
        return Ok(value);
    }
    Ok(json!({
        "execution_id": execution_id.to_string(),
        "result_bytes": size,
        "complete": false,
        "name": value.get("name").and_then(Value::as_str).filter(|name| name.len() < 200),
        "read_tool": "read_agent_execution_result",
        "message": "结果较大，已保存完整结果。按需使用 execution_id、JSON pointer、offset 和 limit 继续读取。"
    }))
}
