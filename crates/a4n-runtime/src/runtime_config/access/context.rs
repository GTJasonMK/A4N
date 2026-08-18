use a4n_core::config::app_config::LlmMaxTokensConfig;
use serde_json::Value;

use super::super::contracts::RuntimeConfigSnapshot;
use super::super::scope::{RUNTIME_CONFIG_SNAPSHOT, RUNTIME_USER_ID};

pub fn current_snapshot() -> Option<RuntimeConfigSnapshot> {
    RUNTIME_CONFIG_SNAPSHOT.try_with(Clone::clone).ok()
}

pub fn current_user_id() -> Option<i64> {
    RUNTIME_USER_ID.try_with(|user_id| *user_id).ok()
}

pub fn current_llm_max_tokens() -> Option<LlmMaxTokensConfig> {
    current_snapshot().and_then(|snapshot| snapshot.llm_max_tokens)
}

pub fn uses_client_desktop_embedding() -> bool {
    RUNTIME_CONFIG_SNAPSHOT
        .try_with(|snapshot| {
            snapshot.client_context.as_ref().is_some_and(|context| {
                context
                    .get("frontend_kind")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value == "tauri")
                    && context
                        .get("backend_kind")
                        .and_then(Value::as_str)
                        .is_some_and(|value| value == "remote_http")
            })
        })
        .unwrap_or(false)
}
