use a4n_core::config::app_config::DEFAULT_LLM_MAX_OUTPUT_TOKENS;
use a4n_engine::image::{
    ChatCompletionsImageProvider, CliProxyImageProvider, ComfyUiImageProvider,
    ImageGenerationRequest, OpenAiCompatibleImageEditProvider, OpenAiCompatibleImageProvider,
    StableDiffusionWebUiImageProvider,
};
use a4n_engine::llm::provider::{ChatMessage, ChatRequest, MessageRole, ResponseFormat};
use a4n_services::llm_context::preview_context_aware_chat_request;
use serde::Serialize;
use serde_json::{Value, json};

use crate::{dynamic_llm, state::AppState};

#[derive(Debug, Clone, Serialize)]
pub struct ModelRequestPreview {
    pub workflow: String,
    pub title: String,
    pub steps: Vec<ModelRequestPreviewStep>,
    pub ready_step_count: usize,
    pub total_chars: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelRequestPreviewStep {
    pub key: String,
    pub title: String,
    pub kind: String,
    pub prompt_name: Option<String>,
    pub state: String,
    pub note: Option<String>,
    pub system_prompt: Option<String>,
    pub user_prompt: Option<String>,
    pub messages: Option<Value>,
    pub request: Option<Value>,
    pub context_manifest: Option<Value>,
    pub context_plan: Option<Value>,
    pub context_pack_refs: Option<Value>,
    pub total_chars: usize,
    pub char_counts: Option<Value>,
}

pub(crate) struct ChatRequestStepInput {
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) prompt_name: Option<String>,
    pub(crate) state: String,
    pub(crate) note: Option<String>,
    pub(crate) request: ChatRequest,
}

pub(crate) struct ChatRequestStepWithBudgetInput {
    pub(crate) step: ChatRequestStepInput,
    pub(crate) model_context_tokens: usize,
    pub(crate) output_token_reserve_override: Option<usize>,
    pub(crate) budget_note: Option<String>,
}

pub(crate) fn model_request_preview(
    workflow: impl Into<String>,
    title: impl Into<String>,
    steps: Vec<ModelRequestPreviewStep>,
) -> ModelRequestPreview {
    let ready_step_count = steps
        .iter()
        .filter(|step| step.state == "ready" || step.state == "preview_placeholder")
        .count();
    let total_chars = steps.iter().map(|step| step.total_chars).sum();
    ModelRequestPreview {
        workflow: workflow.into(),
        title: title.into(),
        steps,
        ready_step_count,
        total_chars,
    }
}

pub(crate) fn chat_request_step_with_budget(
    input: ChatRequestStepWithBudgetInput,
) -> Result<ModelRequestPreviewStep, String> {
    let ChatRequestStepWithBudgetInput {
        step:
            ChatRequestStepInput {
                key,
                title,
                prompt_name,
                state,
                note,
                mut request,
            },
        model_context_tokens,
        output_token_reserve_override,
        budget_note,
    } = input;
    let output_token_reserve = output_token_reserve_override
        .or(request.max_tokens)
        .unwrap_or(DEFAULT_LLM_MAX_OUTPUT_TOKENS);
    let context_preview = preview_context_aware_chat_request(
        &mut request,
        model_context_tokens,
        output_token_reserve,
    )
    .map_err(|err| err.to_string())?;
    let system_prompt = request
        .messages
        .iter()
        .find(|message| matches!(message.role, MessageRole::System))
        .map(|message| message.content.clone());
    let user_prompt = request
        .messages
        .iter()
        .rev()
        .find(|message| matches!(message.role, MessageRole::User))
        .map(|message| message.content.clone());
    let messages =
        sanitize_preview_value(serde_json::to_value(&request.messages).map_err(|e| e.to_string())?);
    let request =
        sanitize_preview_value(serde_json::to_value(&request).map_err(|e| e.to_string())?);
    let system_chars = system_prompt
        .as_deref()
        .map(str::chars)
        .map(Iterator::count);
    let user_chars = user_prompt.as_deref().map(str::chars).map(Iterator::count);
    let messages_text = messages.to_string();
    let request_text = request.to_string();
    let total_chars = messages_text.chars().count();
    let request_chars = request_text.chars().count();
    Ok(ModelRequestPreviewStep {
        key,
        title,
        kind: "llm_chat".to_string(),
        prompt_name,
        state,
        note: merge_preview_notes(note, budget_note),
        system_prompt: system_prompt.map(|text| redact_data_uris_in_text(&text)),
        user_prompt: user_prompt.map(|text| redact_data_uris_in_text(&text)),
        messages: Some(messages),
        request: Some(request),
        context_manifest: context_preview
            .as_ref()
            .map(|preview| sanitize_preview_value(preview.manifest_json.clone())),
        context_plan: context_preview
            .as_ref()
            .map(|preview| sanitize_preview_value(preview.plan_json.clone())),
        context_pack_refs: context_preview
            .as_ref()
            .map(|preview| sanitize_preview_value(preview.pack_refs_json.clone())),
        total_chars,
        char_counts: Some(json!({
            "system_chars": system_chars,
            "user_chars": user_chars,
            "messages_chars": total_chars,
            "messages_bytes": messages_text.len(),
            "request_chars": request_chars,
            "request_bytes": request_text.len(),
            "context_stable_prefix_tokens": context_preview.as_ref().map(|preview| preview.stable_prefix_tokens),
            "context_dynamic_delta_tokens": context_preview.as_ref().map(|preview| preview.dynamic_delta_tokens),
            "context_available_input_tokens": context_preview.as_ref().map(|preview| preview.available_input_tokens),
            "context_model_context_tokens": model_context_tokens,
            "context_output_token_reserve": output_token_reserve
        })),
    })
}

pub(crate) async fn chat_request_step_for_state(
    state: &AppState,
    step: ChatRequestStepInput,
) -> Result<ModelRequestPreviewStep, String> {
    let budget =
        dynamic_llm::resolve_effective_llm_preview_budget(state.db.as_ref(), &step.request)
            .await
            .map_err(|err| err.to_string())?;
    chat_request_step_with_budget(ChatRequestStepWithBudgetInput {
        step,
        model_context_tokens: budget.max_context_tokens,
        output_token_reserve_override: Some(budget.output_token_reserve),
        budget_note: Some(format!(
            "预览模型={}；上下文={}；输出上限={}；输出保留={}（{}）",
            budget.model_name,
            budget.max_context_tokens,
            budget.max_output_tokens,
            budget.output_token_reserve,
            budget.output_token_source
        )),
    })
}

pub(crate) fn chat_request_from_preview_value(
    request: &Value,
    context_manifest: Option<Value>,
) -> Result<ChatRequest, String> {
    let object = request
        .as_object()
        .ok_or_else(|| "LLM 预览 request 必须是对象".to_string())?;
    let messages_value = object
        .get("messages")
        .ok_or_else(|| "LLM 预览 request 缺少 messages".to_string())?
        .clone();
    let messages: Vec<ChatMessage> =
        serde_json::from_value(messages_value).map_err(|err| err.to_string())?;
    let temperature = object
        .get("temperature")
        .and_then(Value::as_f64)
        .map(|value| value as f32);
    let max_tokens = optional_usize_field(object.get("max_tokens"), "max_tokens")?;
    let max_tokens_profile = object
        .get("max_tokens_profile")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let response_format = match object.get("response_format") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            serde_json::from_value::<ResponseFormat>(value.clone())
                .map_err(|err| format!("response_format 解析失败: {err}"))?,
        ),
    };
    let tools = match object.get("tools") {
        Some(Value::Null) | None => None,
        Some(Value::Array(values)) => Some(values.clone()),
        Some(_) => return Err("tools 必须是数组".to_string()),
    };
    let tool_choice = object
        .get("tool_choice")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let stop_sequences = match object.get("stop_sequences") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            serde_json::from_value::<Vec<String>>(value.clone())
                .map_err(|err| format!("stop_sequences 解析失败: {err}"))?,
        ),
    };
    let mut chat_request = ChatRequest::new(messages, temperature, max_tokens, max_tokens_profile);
    chat_request.response_format = response_format;
    chat_request.tools = tools;
    chat_request.tool_choice = tool_choice;
    chat_request.stop_sequences = stop_sequences;
    chat_request.context_manifest_json = context_manifest;
    Ok(chat_request)
}

fn optional_usize_field(value: Option<&Value>, field: &str) -> Result<Option<usize>, String> {
    match value {
        Some(Value::Number(number)) => number
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| format!("{field} 必须是非负整数")),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(format!("{field} 必须是非负整数")),
    }
}

fn merge_preview_notes(note: Option<String>, budget_note: Option<String>) -> Option<String> {
    match (note, budget_note) {
        (Some(note), Some(budget_note)) if !budget_note.trim().is_empty() => {
            Some(format!("{note}；{budget_note}"))
        }
        (Some(note), _) => Some(note),
        (None, Some(budget_note)) if !budget_note.trim().is_empty() => Some(budget_note),
        (None, _) => None,
    }
}

pub(crate) fn provider_request_step<T: Serialize>(
    key: impl Into<String>,
    title: impl Into<String>,
    kind: impl Into<String>,
    prompt_name: impl Into<Option<String>>,
    state: impl Into<String>,
    note: impl Into<Option<String>>,
    request: &T,
) -> Result<ModelRequestPreviewStep, String> {
    let request = sanitize_preview_value(serde_json::to_value(request).map_err(|e| e.to_string())?);
    let request_text = request.to_string();
    let total_chars = request_text.chars().count();
    let prompt = prompt_from_request(&request);
    let prompt_chars = prompt.as_deref().map(str::chars).map(Iterator::count);
    let prompt_bytes = prompt.as_deref().map(str::len);
    Ok(ModelRequestPreviewStep {
        key: key.into(),
        title: title.into(),
        kind: kind.into(),
        prompt_name: prompt_name.into(),
        state: state.into(),
        note: note.into(),
        system_prompt: None,
        user_prompt: prompt,
        messages: None,
        request: Some(request),
        context_manifest: None,
        context_plan: None,
        context_pack_refs: None,
        total_chars,
        char_counts: Some(json!({
            "prompt_chars": prompt_chars,
            "prompt_bytes": prompt_bytes,
            "request_chars": total_chars,
            "request_bytes": request_text.len()
        })),
    })
}

pub(crate) fn image_provider_request_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: impl Into<Option<String>>,
    state: impl Into<String>,
    note: impl Into<Option<String>>,
    backend: &str,
    request: ImageGenerationRequest,
    provider_options_json: Option<&Value>,
) -> Result<ModelRequestPreviewStep, String> {
    let key = key.into();
    let title = title.into();
    let prompt_name = prompt_name.into();
    let state = state.into();
    let note = note.into();
    match image_provider_preview_payload(backend, &request, provider_options_json) {
        Ok((kind, provider_note, provider_request)) => provider_request_step(
            key,
            title,
            kind,
            prompt_name,
            state,
            join_preview_notes(note, Some(provider_note)),
            &provider_request,
        ),
        Err(err) => Ok(blocked_step(
            key,
            title,
            "image_generation",
            prompt_name,
            join_preview_notes(note, Some(err))
                .unwrap_or_else(|| "图片 provider 请求不可用".into()),
        )),
    }
}

pub(crate) fn blocked_step(
    key: impl Into<String>,
    title: impl Into<String>,
    kind: impl Into<String>,
    prompt_name: impl Into<Option<String>>,
    note: impl Into<String>,
) -> ModelRequestPreviewStep {
    ModelRequestPreviewStep {
        key: key.into(),
        title: title.into(),
        kind: kind.into(),
        prompt_name: prompt_name.into(),
        state: "blocked".to_string(),
        note: Some(note.into()),
        system_prompt: None,
        user_prompt: None,
        messages: None,
        request: None,
        context_manifest: None,
        context_plan: None,
        context_pack_refs: None,
        total_chars: 0,
        char_counts: None,
    }
}

fn status_step(
    key: impl Into<String>,
    title: impl Into<String>,
    kind: impl Into<String>,
    prompt_name: impl Into<Option<String>>,
    state: impl Into<String>,
    note: impl Into<Option<String>>,
) -> ModelRequestPreviewStep {
    ModelRequestPreviewStep {
        key: key.into(),
        title: title.into(),
        kind: kind.into(),
        prompt_name: prompt_name.into(),
        state: state.into(),
        note: note.into(),
        system_prompt: None,
        user_prompt: None,
        messages: None,
        request: None,
        context_manifest: None,
        context_plan: None,
        context_pack_refs: None,
        total_chars: 0,
        char_counts: None,
    }
}

pub(crate) struct ChatStatusPreviewStep {
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) kind: Option<String>,
    pub(crate) prompt_name: Option<String>,
    pub(crate) state: String,
    pub(crate) note: Option<String>,
    pub(crate) request: Option<ChatRequest>,
}

impl ChatStatusPreviewStep {
    pub(crate) fn llm(
        key: impl Into<String>,
        title: impl Into<String>,
        prompt_name: impl Into<Option<String>>,
        state: impl Into<String>,
        note: impl Into<Option<String>>,
        request: Option<ChatRequest>,
    ) -> Self {
        Self::new(key, title, None, prompt_name, state, note, request)
    }

    pub(crate) fn new(
        key: impl Into<String>,
        title: impl Into<String>,
        kind: impl Into<Option<String>>,
        prompt_name: impl Into<Option<String>>,
        state: impl Into<String>,
        note: impl Into<Option<String>>,
        request: Option<ChatRequest>,
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            kind: kind.into(),
            prompt_name: prompt_name.into(),
            state: state.into(),
            note: note.into(),
            request,
        }
    }
}

pub(crate) async fn chat_status_preview_for_state(
    state: &AppState,
    workflow: impl Into<String>,
    title: impl Into<String>,
    steps: impl IntoIterator<Item = ChatStatusPreviewStep>,
    default_status_kind: &'static str,
) -> Result<ModelRequestPreview, String> {
    Ok(model_request_preview(
        workflow,
        title,
        chat_status_preview_steps_for_state(state, steps, default_status_kind).await?,
    ))
}

async fn chat_status_preview_steps_for_state(
    state: &AppState,
    steps: impl IntoIterator<Item = ChatStatusPreviewStep>,
    default_status_kind: &'static str,
) -> Result<Vec<ModelRequestPreviewStep>, String> {
    let mut result = Vec::new();
    for step in steps {
        let preview_step = match step.request {
            Some(request) => {
                chat_request_step_for_state(
                    state,
                    ChatRequestStepInput {
                        key: step.key,
                        title: step.title,
                        prompt_name: step.prompt_name,
                        state: step.state,
                        note: step.note,
                        request,
                    },
                )
                .await?
            }
            None => status_step(
                step.key,
                step.title,
                step.kind.unwrap_or_else(|| default_status_kind.to_string()),
                step.prompt_name,
                step.state,
                step.note,
            ),
        };
        result.push(preview_step);
    }
    Ok(result)
}

fn prompt_from_request(request: &Value) -> Option<String> {
    request
        .get("prompt")
        .or_else(|| request.pointer("/fields/prompt"))
        .or_else(|| request.pointer("/parameters/prompt"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| prompt_from_messages(request.get("messages")?))
}

fn prompt_from_messages(messages: &Value) -> Option<String> {
    messages.as_array()?.iter().rev().find_map(|message| {
        let role = message.get("role").and_then(Value::as_str).unwrap_or("");
        if role != "user" {
            return None;
        }
        message_content_text(message.get("content")?)
    })
}

fn message_content_text(content: &Value) -> Option<String> {
    if let Some(text) = content
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        return Some(text.to_string());
    }
    let parts = content.as_array()?;
    let text = parts
        .iter()
        .filter_map(|part| {
            part.get("text")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    if text.is_empty() { None } else { Some(text) }
}

fn image_provider_preview_payload(
    backend: &str,
    request: &ImageGenerationRequest,
    provider_options_json: Option<&Value>,
) -> Result<(&'static str, String, Value), String> {
    match backend {
        "openai-compatible-image" => Ok((
            "image_generation_openai_compatible",
            "OpenAI 兼容 `/images/generations` JSON 请求；该后端不支持参考图输入。".into(),
            OpenAiCompatibleImageProvider::request_body_preview(request)
                .map_err(|err| err.to_string())?,
        )),
        "openai-compatible-image-edit" => {
            let note = if request.reference_images.is_empty() {
                "OpenAI 兼容图片编辑后端在无参考图时走 `/images/generations` JSON 请求。"
            } else {
                "OpenAI 兼容 `/images/edits` multipart 请求；预览省略参考图媒体数据。"
            };
            Ok((
                "image_generation_openai_compatible_edit",
                note.into(),
                OpenAiCompatibleImageEditProvider::request_body_preview(request)
                    .map_err(|err| err.to_string())?,
            ))
        }
        "openai-chat-image" => Ok((
            "image_generation_chat_completions",
            "OpenAI Chat-Completions 图像请求；预览省略参考图媒体数据。".into(),
            ChatCompletionsImageProvider::request_body_preview(request),
        )),
        "cliproxy-image" => {
            let note = if request.reference_images.is_empty() {
                "CLIProxy `/images/generations` JSON 请求；强制 response_format=url。"
            } else {
                "CLIProxy `/images/edits` multipart 请求；强制 response_format=url，预览省略参考图媒体数据。"
            };
            Ok((
                "image_generation_cliproxy",
                note.into(),
                CliProxyImageProvider::request_body_preview_with_options(
                    request,
                    provider_options_json,
                )
                .map_err(|err| err.to_string())?,
            ))
        }
        "stable-diffusion" => Ok((
            "image_generation_sd_webui",
            "Stable Diffusion WebUI `/sdapi/v1/txt2img` JSON 请求。".into(),
            StableDiffusionWebUiImageProvider::request_body_preview(request)
                .map_err(|err| err.to_string())?,
        )),
        "comfyui" => Ok((
            "image_generation_comfyui",
            "ComfyUI `/prompt` workflow 请求；真实生成时 client_id 与 seed 会动态生成。".into(),
            ComfyUiImageProvider::request_body_preview(request).map_err(|err| err.to_string())?,
        )),
        other => Err(format!(
            "未知图片 backend={other}，无法展示真实 provider 请求体"
        )),
    }
}

fn join_preview_notes(first: Option<String>, second: Option<String>) -> Option<String> {
    let notes = [first, second]
        .into_iter()
        .flatten()
        .map(|note| note.trim().to_string())
        .filter(|note| !note.is_empty())
        .collect::<Vec<_>>();
    if notes.is_empty() {
        None
    } else {
        Some(notes.join("；"))
    }
}

fn sanitize_preview_value(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact_data_uris_in_text(&text)),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(sanitize_preview_value).collect())
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (key, value) in map {
                if let Value::String(text) = &value
                    && let Some(summary) = inline_media_summary(text)
                {
                    out.insert(format!("{key}_omitted"), summary);
                    continue;
                }
                out.insert(key, sanitize_preview_value(value));
            }
            Value::Object(out)
        }
        other => other,
    }
}

fn inline_media_summary(text: &str) -> Option<Value> {
    let text = text.trim();
    let (mime, payload_chars, payload_end) = data_uri_parts(text)?;
    if payload_end != text.len() {
        return None;
    }
    Some(json!({
        "kind": "inline_media",
        "mime": mime,
        "payload_chars": payload_chars,
        "omitted": true,
        "note": "预览已省略完整内联媒体数据；真实请求按对应 provider 的传输方式处理"
    }))
}

fn redact_data_uris_in_text(text: &str) -> String {
    if !text.contains("data:") || !text.contains(";base64,") {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len().min(1024));
    let mut cursor = 0;
    while let Some(relative_start) = text[cursor..].find("data:") {
        let start = cursor + relative_start;
        out.push_str(&text[cursor..start]);
        if text[start..].find(',').is_none() {
            out.push_str(&text[start..]);
            cursor = text.len();
            break;
        }
        let Some((mime, payload_chars, payload_end)) = data_uri_parts(&text[start..]) else {
            out.push_str("data:");
            cursor = start + "data:".len();
            continue;
        };
        out.push_str(&format!(
            "[已省略内联媒体数据: mime={mime}, payload_chars={payload_chars}]"
        ));
        cursor = start + payload_end;
    }
    if cursor < text.len() {
        out.push_str(&text[cursor..]);
    }
    out
}

fn data_uri_parts(text: &str) -> Option<(&str, usize, usize)> {
    let rest = text.strip_prefix("data:")?;
    let (header_rest, _) = rest.split_once(',')?;
    let header_len = "data:".len() + header_rest.len();
    let header = &text[..header_len];
    if !header
        .split(';')
        .any(|part| part.eq_ignore_ascii_case("base64"))
    {
        return None;
    }
    let mime = header
        .strip_prefix("data:")
        .and_then(|rest| rest.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    let payload_start = header_len + 1;
    let payload_end = text[payload_start..]
        .find(|ch: char| !is_base64_payload_char(ch))
        .map(|relative_end| payload_start + relative_end)
        .unwrap_or(text.len());
    let payload_chars = text[payload_start..payload_end].chars().count();
    Some((mime, payload_chars, payload_end))
}

fn is_base64_payload_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '+' | '/' | '=')
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_engine::image::{ImageGenerationRequest, ImageReference};
    use a4n_services::llm_context::{
        COMPRESSION_COMPRESSED_SCENE, CONTEXT_MANIFEST_SCHEMA_VERSION, LlmContextManifest,
        LlmContextManifestPack, LlmContextManifestPackVariant, context_aware_system_user_request,
    };

    #[test]
    fn model_request_preview_counts_placeholder_steps_as_displayable() {
        let preview = model_request_preview(
            "workflow",
            "title",
            vec![
                status_step("ready", "ready", "llm", None, "ready", None),
                status_step(
                    "placeholder",
                    "placeholder",
                    "llm",
                    None,
                    "preview_placeholder",
                    None,
                ),
                blocked_step("blocked", "blocked", "llm", None, "等待前序输出"),
            ],
        );

        assert_eq!(preview.ready_step_count, 2);
    }

    #[test]
    fn provider_request_step_extracts_nested_provider_prompt() {
        let step = provider_request_step(
            "seedance",
            "Seedance 请求",
            "video_generation_seedance2_omni_reference",
            Some("reference_media_video".to_string()),
            "ready",
            None,
            &serde_json::json!({
                "model_id": "seedance",
                "parameters": {
                    "prompt": "嵌套 provider prompt"
                }
            }),
        )
        .expect("provider request step should render");

        assert_eq!(step.user_prompt.as_deref(), Some("嵌套 provider prompt"));
    }

    #[test]
    fn chat_request_step_renders_context_aware_preview() {
        let request = context_aware_system_user_request(
            "系统规则",
            LlmContextManifest {
                schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
                packs: vec![LlmContextManifestPack {
                    scope_kind: "chapter".to_string(),
                    pack_kind: "chapter_context".to_string(),
                    source_refs_json: None,
                    config_hash: "preview:test".to_string(),
                    required: true,
                    stable_prefix: true,
                    variants: vec![LlmContextManifestPackVariant {
                        compression_level: COMPRESSION_COMPRESSED_SCENE.to_string(),
                        canonical_content_json: json!({
                            "summary": "章节上下文",
                            "characters": ["甲", "乙"]
                        }),
                    }],
                }],
                dynamic_user_content: "生成当前镜头".to_string(),
                output_contract: Some("只输出 JSON".to_string()),
            },
            Some(0.2),
            Some(1024),
            None,
        )
        .expect("context-aware request");

        let step = chat_request_step_with_budget(ChatRequestStepWithBudgetInput {
            step: ChatRequestStepInput {
                key: "step".to_string(),
                title: "上下文预览".to_string(),
                prompt_name: Some("prompt".to_string()),
                state: "ready".to_string(),
                note: None,
                request,
            },
            model_context_tokens: 128_000,
            output_token_reserve_override: Some(1024),
            budget_note: Some("测试模型预算".to_string()),
        })
        .expect("preview step");

        let user_prompt = step.user_prompt.as_deref().expect("user prompt");
        assert!(user_prompt.contains("[稳定上下文]"));
        assert!(user_prompt.contains("chapter_context"));
        assert!(user_prompt.contains("[当前任务]"));
        assert!(user_prompt.contains("[输出契约]"));
        assert!(step.context_manifest.is_some());
        assert!(step.context_plan.is_some());
        assert!(step.context_pack_refs.is_some());
    }

    #[test]
    fn chat_request_from_preview_value_restores_context_request_fields() {
        let manifest = json!({"schema_version": 1, "packs": []});
        let request = json!({
            "messages": [
                {"role": "system", "content": "system"},
                {"role": "user", "content": "user"}
            ],
            "temperature": 0.2,
            "max_tokens": 512,
            "max_tokens_profile": "media_prompt",
            "response_format": {"type": "json_object"},
            "tools": [{"type": "function"}],
            "tool_choice": "auto",
            "stop_sequences": ["END"]
        });

        let restored = chat_request_from_preview_value(&request, Some(manifest.clone()))
            .expect("preview request restores");

        assert_eq!(restored.messages.len(), 2);
        assert_eq!(restored.max_tokens, Some(512));
        assert_eq!(restored.max_tokens_profile.as_deref(), Some("media_prompt"));
        assert_eq!(
            restored
                .response_format
                .as_ref()
                .map(|format| format.r#type.as_str()),
            Some("json_object")
        );
        assert_eq!(restored.tools.as_ref().map(Vec::len), Some(1));
        assert_eq!(restored.tool_choice.as_deref(), Some("auto"));
        assert_eq!(
            restored
                .stop_sequences
                .as_ref()
                .and_then(|items| items.first()),
            Some(&"END".to_string())
        );
        assert_eq!(restored.context_manifest_json, Some(manifest));
    }

    #[test]
    fn provider_request_step_redacts_inline_base64_media() {
        let step = provider_request_step(
            "image",
            "图片请求",
            "image_generation",
            None,
            "ready",
            None,
            &serde_json::json!({
                "prompt": "画图",
                "reference_images": [
                    {
                        "label": "参考图 1",
                        "data_uri": "data:image/png;base64,QUJDREVGRw=="
                    }
                ]
            }),
        )
        .expect("provider request step should render");

        let request = step.request.expect("request should exist").to_string();

        assert!(!request.contains("QUJDREVGRw"));
        assert!(!request.contains("data:image/png;base64"));
        assert!(!request.contains("\"data_uri\""));
        assert!(request.contains("data_uri_omitted"));
        assert!(request.contains("预览已省略完整内联媒体数据"));
        assert!(request.contains("image/png"));
    }

    #[test]
    fn image_provider_request_step_uses_backend_request_body_and_redacts_media() {
        let step = image_provider_request_step(
            "image",
            "图片请求",
            None,
            "ready",
            None,
            "openai-chat-image",
            ImageGenerationRequest {
                prompt: "画一张角色设定图".into(),
                width: 1024,
                height: 1024,
                model: "chat-image".into(),
                reference_images: vec![ImageReference {
                    label: "参考图 1".into(),
                    data_uri: "data:image/png;base64,QUJDREVGRw==".into(),
                }],
            },
            None,
        )
        .expect("image provider preview");

        assert_eq!(step.kind, "image_generation_chat_completions");
        assert_eq!(step.state, "ready");
        assert!(
            step.user_prompt
                .as_deref()
                .unwrap()
                .contains("画一张角色设定图")
        );
        let request = step.request.expect("request should exist").to_string();
        assert!(request.contains("\"messages\""));
        assert!(!request.contains("QUJDREVGRw"));
        assert!(!request.contains("data:image/png;base64"));
        assert!(request.contains("url_omitted"));
    }

    #[test]
    fn image_provider_request_step_blocks_unsupported_reference_images() {
        let step = image_provider_request_step(
            "image",
            "图片请求",
            None,
            "ready",
            Some("调用方备注".to_string()),
            "openai-compatible-image",
            ImageGenerationRequest {
                prompt: "画一张角色设定图".into(),
                width: 1024,
                height: 1024,
                model: "image-model".into(),
                reference_images: vec![ImageReference {
                    label: "参考图 1".into(),
                    data_uri: "data:image/png;base64,QUJDREVGRw==".into(),
                }],
            },
            None,
        )
        .expect("blocked preview step");

        assert_eq!(step.state, "blocked");
        assert!(step.request.is_none());
        let note = step.note.as_deref().unwrap();
        assert!(note.contains("调用方备注"));
        assert!(note.contains("真实请求不支持参考图输入"));
    }
}
