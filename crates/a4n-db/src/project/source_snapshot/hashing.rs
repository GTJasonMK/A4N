use super::super::ProjectMutationError;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(super) fn to_json<T>(value: &T) -> Result<Value, ProjectMutationError>
where
    T: Serialize,
{
    serde_json::to_value(value)
        .map_err(|err| ProjectMutationError::Internal(format!("源快照序列化失败: {err}")))
}

pub(super) fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(super) fn hash_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("serde_json::Value serialization cannot fail");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}
