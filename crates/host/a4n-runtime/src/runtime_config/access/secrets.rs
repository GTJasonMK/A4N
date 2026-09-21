use super::context::current_snapshot;
use crate::runtime_config::RuntimeConfigSnapshot;

pub fn current_system_llm_api_key() -> Option<String> {
    current_snapshot_secret(|snapshot| snapshot.system_llm_api_key_encrypted)
}

pub fn current_system_image_api_key() -> Option<String> {
    current_snapshot_secret(|snapshot| snapshot.system_image_api_key_encrypted)
}

pub fn current_system_video_api_key() -> Option<String> {
    current_snapshot_secret(|snapshot| snapshot.system_video_api_key_encrypted)
}

fn current_snapshot_secret(
    secret: impl FnOnce(RuntimeConfigSnapshot) -> Option<String>,
) -> Option<String> {
    current_snapshot()
        .and_then(secret)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
