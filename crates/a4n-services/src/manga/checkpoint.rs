use a4n_core::error::AppError;
use serde::Serialize;
use serde_json::{Map, Value};

pub(super) fn checkpoint_insert(checkpoint: &mut Map<String, Value>, key: &str, value: Value) {
    checkpoint.insert(key.to_string(), value);
}

pub(super) fn checkpoint_insert_trace<T: Serialize>(
    checkpoint: &mut Map<String, Value>,
    key: &str,
    trace: &T,
) -> Result<(), AppError> {
    let value = serde_json::to_value(trace)
        .map_err(|err| AppError::InvalidArgument(format!("漫画 trace 断点序列化失败: {err}")))?;
    checkpoint_insert(checkpoint, key, value);
    Ok(())
}
