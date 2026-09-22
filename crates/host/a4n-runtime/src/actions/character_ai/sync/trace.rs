use a4n_character_host_contract::{
    CharacterProfileSyncStageTraceInput, CharacterProfileSyncStageTracePort,
};
use a4n_llm_provider_contract::provider::ChatMessage;
use serde_json::json;
use std::path::{Path, PathBuf};

pub(in crate::actions::character_ai) fn record_character_sync_llm_trace(
    log_dir: &Path,
    profile_name: &str,
    phase: &str,
    messages: &[ChatMessage],
    response_content: &str,
    temperature: Option<f32>,
    parse_error: Option<&str>,
) -> Option<String> {
    match write_character_sync_llm_trace(
        log_dir,
        profile_name,
        phase,
        messages,
        response_content,
        temperature,
        parse_error,
    ) {
        Ok(path) => {
            let path = path.to_string_lossy().to_string();
            tracing::info!(
                target: "a4n::llm::trace",
                profile_name,
                phase,
                path = %path,
                parse_error = ?parse_error,
                "角色同步 LLM 调试日志已写入"
            );
            Some(path)
        }
        Err(err) => {
            tracing::warn!(
                target: "a4n::llm::trace",
                profile_name,
                phase,
                error = %err,
                "写入角色同步 LLM 调试日志失败"
            );
            None
        }
    }
}

pub(crate) struct RuntimeCharacterProfileSyncStageTracePort<'a> {
    log_dir: &'a Path,
}

impl<'a> RuntimeCharacterProfileSyncStageTracePort<'a> {
    pub(crate) fn new(log_dir: &'a Path) -> Self {
        Self { log_dir }
    }
}

impl CharacterProfileSyncStageTracePort for RuntimeCharacterProfileSyncStageTracePort<'_> {
    fn record_stage_trace(&self, input: CharacterProfileSyncStageTraceInput) -> Option<String> {
        record_character_sync_llm_trace(
            self.log_dir,
            &input.profile_name,
            &input.phase,
            &input.messages,
            &input.response_content,
            input.temperature,
            input.parse_error.as_deref(),
        )
    }
}

fn write_character_sync_llm_trace(
    log_dir: &Path,
    profile_name: &str,
    phase: &str,
    messages: &[ChatMessage],
    response_content: &str,
    temperature: Option<f32>,
    parse_error: Option<&str>,
) -> Result<PathBuf, String> {
    let dir = log_dir.join("llm").join("character_sync");
    std::fs::create_dir_all(&dir).map_err(|err| format!("创建 LLM 调试日志目录失败: {err}"))?;
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let filename = format!(
        "{timestamp}_{}_{}.json",
        safe_trace_filename_part(profile_name),
        safe_trace_filename_part(phase)
    );
    let path = dir.join(filename);
    let payload = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "kind": "character_sync_llm_exchange",
        "profile_name": profile_name,
        "phase": phase,
        "parse_error": parse_error,
        "request": {
            "temperature": temperature,
            "response_format": {
                "type": "json_object"
            },
            "messages": messages,
        },
        "response": {
            "content": response_content,
        }
    });
    let content = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("序列化 LLM 调试日志失败: {err}"))?;
    std::fs::write(&path, content).map_err(|err| format!("写入 LLM 调试日志失败: {err}"))?;
    Ok(path)
}

fn safe_trace_filename_part(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else if ch.is_whitespace() {
            out.push('_');
        }
    }
    if out.is_empty() {
        "trace".to_string()
    } else {
        out.chars().take(48).collect()
    }
}
