use a4n_domain::source_text_hash;
use a4n_error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ChatRequest, LlmProtocolKind, MessageRole};

const MAX_CACHE_WRITES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptCacheMode {
    Explicit,
    Implicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptCacheBreakpoint {
    pub message_index: usize,
    pub byte_offset: usize,
}

/// Validate native cache options at both configuration and dispatch boundaries.
pub fn validate_prompt_cache_options(
    provider: &str,
    model: &str,
    options: Option<&Value>,
) -> Result<Option<PromptCacheMode>, AppError> {
    let Some(options) = options.and_then(|value| value.get("prompt_cache_options")) else {
        return Ok(None);
    };
    if !uses_openai_prompt_cache(provider) {
        return Err(AppError::ConfigError(format!(
            "prompt_cache_options 仅适用于 OpenAI Chat Completions/Responses，当前 provider 为 {provider}"
        )));
    }
    let options = options
        .as_object()
        .ok_or_else(|| AppError::ConfigError("prompt_cache_options 必须是对象".into()))?;
    if openai_generation(model).is_some_and(|version| version < (5, 6)) {
        return Err(AppError::ConfigError(
            "当前 OpenAI 模型不支持显式缓存选项".into(),
        ));
    }
    match options.get("mode") {
        None => Ok(None),
        Some(Value::String(mode)) if mode == "explicit" => Ok(Some(PromptCacheMode::Explicit)),
        Some(Value::String(mode)) if mode == "implicit" => Ok(Some(PromptCacheMode::Implicit)),
        Some(_) => Err(AppError::ConfigError(
            "prompt_cache_options.mode 必须为 explicit 或 implicit".into(),
        )),
    }
}

fn uses_openai_prompt_cache(provider: &str) -> bool {
    LlmProtocolKind::for_provider(provider) == Some(LlmProtocolKind::OpenAiResponses)
        || matches!(
            provider.trim().to_ascii_lowercase().as_str(),
            "openai" | "custom-chat-completions"
        )
}

impl ChatRequest {
    pub fn add_cache_breakpoint(
        &mut self,
        message_index: usize,
        byte_offset: usize,
    ) -> Result<(), AppError> {
        let point = PromptCacheBreakpoint {
            message_index,
            byte_offset,
        };
        self.validate_cache_breakpoint(point)?;
        self.cache_breakpoints.push(point);
        Ok(())
    }

    pub fn configure_prompt_cache(
        &mut self,
        provider: &str,
        model: &str,
        options: Option<&Value>,
        project_id: Option<i64>,
    ) -> Result<(), AppError> {
        let protocol = LlmProtocolKind::for_provider(provider)
            .ok_or_else(|| AppError::ConfigError(format!("不支持的 LLM provider: {provider}")))?;
        let configured_mode = validate_prompt_cache_options(provider, model, options)?;
        self.cache_breakpoints = self.effective_cache_breakpoints()?;
        self.cache_system_prefix = false;
        self.cache_user_prefix_len = None;
        self.cache_scope_key = None;
        self.cache_mode = None;

        let openai = uses_openai_prompt_cache(provider);
        let cache_options = options.and_then(|value| value.get("prompt_cache_options"));
        let generation = openai_generation(model);
        // Native Claude supports block caching regardless of its model alias.
        // OpenAI cache_options also allow an explicit capability declaration for
        // proxy aliases, while known older models retain their automatic cache.
        let explicit_openai =
            openai && generation.map_or(cache_options.is_some(), |version| version >= (5, 6));
        if !openai && protocol != LlmProtocolKind::AnthropicMessages {
            self.cache_breakpoints.clear();
            return Ok(());
        }
        if let Some(project_id) = project_id {
            // Content is matched by the provider. Routing by an entire context
            // hash would split requests that can reuse a shorter common prefix.
            self.cache_scope_key = Some(format!(
                "a4n:project:{project_id}:m{}",
                &source_text_hash(model)[..12]
            ));
        }
        if openai && !explicit_openai {
            self.cache_breakpoints.clear();
            return Ok(());
        }
        let has_stable_user_prefix = self
            .cache_breakpoints
            .iter()
            .any(|point| matches!(self.messages[point.message_index].role, MessageRole::User));
        let has_history = self
            .messages
            .iter()
            .any(|message| matches!(message.role, MessageRole::Assistant | MessageRole::Tool));
        for (index, message) in self.messages.iter().enumerate() {
            if matches!(message.role, MessageRole::System) && !message.content.is_empty() {
                self.cache_breakpoints.push(PromptCacheBreakpoint {
                    message_index: index,
                    byte_offset: message.content.len(),
                });
            }
        }
        if explicit_openai {
            // A system boundary alone does not make the user's task disposable:
            // native Agent calls must cache it before their first tool round.
            self.cache_mode = Some(configured_mode.unwrap_or(
                if !has_stable_user_prefix || has_history {
                    PromptCacheMode::Implicit
                } else {
                    PromptCacheMode::Explicit
                },
            ));
        }
        if has_history
            || (protocol == LlmProtocolKind::AnthropicMessages && !has_stable_user_prefix)
        {
            for (index, message) in self.messages.iter().enumerate() {
                if matches!(message.role, MessageRole::User | MessageRole::Tool)
                    && !message.content.is_empty()
                    && !(self.cache_mode == Some(PromptCacheMode::Implicit)
                        && index + 1 == self.messages.len())
                {
                    self.cache_breakpoints.push(PromptCacheBreakpoint {
                        message_index: index,
                        byte_offset: message.content.len(),
                    });
                }
            }
        }
        self.cache_breakpoints.sort_unstable();
        self.cache_breakpoints.dedup();
        let limit =
            MAX_CACHE_WRITES - usize::from(self.cache_mode == Some(PromptCacheMode::Implicit));
        if self.cache_breakpoints.len() > limit {
            // ponytail: providers allow four writes; retain the broadest two
            // prefixes and newest extensions. Add workload priorities if measurements require them.
            let tail = self
                .cache_breakpoints
                .split_off(self.cache_breakpoints.len() - (limit - 2));
            self.cache_breakpoints.truncate(2);
            self.cache_breakpoints.extend(tail);
        }
        Ok(())
    }

    pub fn cached_text_parts(&self, message_index: usize) -> Result<Vec<(&str, bool)>, AppError> {
        let message = self
            .messages
            .get(message_index)
            .ok_or_else(|| AppError::InvalidArgument("缓存断点引用不存在的消息".into()))?;
        let points = self.effective_cache_breakpoints()?;
        let mut parts = Vec::new();
        let mut start = 0;
        for point in points
            .into_iter()
            .filter(|point| point.message_index == message_index)
        {
            parts.push((&message.content[start..point.byte_offset], true));
            start = point.byte_offset;
        }
        if start < message.content.len() || parts.is_empty() {
            parts.push((&message.content[start..], false));
        }
        Ok(parts)
    }

    fn effective_cache_breakpoints(&self) -> Result<Vec<PromptCacheBreakpoint>, AppError> {
        let mut points = self.cache_breakpoints.clone();
        if self.cache_system_prefix {
            points.extend(
                self.messages
                    .iter()
                    .enumerate()
                    .filter_map(|(index, message)| {
                        (matches!(message.role, MessageRole::System) && !message.content.is_empty())
                            .then_some(PromptCacheBreakpoint {
                                message_index: index,
                                byte_offset: message.content.len(),
                            })
                    }),
            );
        }
        if let Some(byte_offset) = self.cache_user_prefix_len {
            let message_index = self
                .messages
                .iter()
                .rposition(|message| matches!(message.role, MessageRole::User))
                .ok_or_else(|| AppError::InvalidArgument("旧缓存断点缺少 user 消息".into()))?;
            points.push(PromptCacheBreakpoint {
                message_index,
                byte_offset,
            });
        }
        for point in &points {
            self.validate_cache_breakpoint(*point)?;
        }
        points.sort_unstable();
        points.dedup();
        Ok(points)
    }

    fn validate_cache_breakpoint(&self, point: PromptCacheBreakpoint) -> Result<(), AppError> {
        let valid = self
            .messages
            .get(point.message_index)
            .is_some_and(|message| {
                point.byte_offset > 0
                    && point.byte_offset <= message.content.len()
                    && message.content.is_char_boundary(point.byte_offset)
            });
        if !valid {
            return Err(AppError::InvalidArgument(
                "缓存断点必须位于消息文本的 UTF-8 边界".into(),
            ));
        }
        Ok(())
    }
}

fn openai_generation(model: &str) -> Option<(u32, u32)> {
    let model = model.rsplit('/').next()?.strip_prefix("gpt-")?;
    let version = model.split('-').next()?;
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().map(str::parse).transpose().ok()?.unwrap_or(0);
    parts.next().is_none().then_some((major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ChatMessage;
    use serde_json::json;

    #[test]
    fn native_dialogue_caches_the_initial_task_before_history_exists() {
        for provider in ["openai", "openai-responses"] {
            let task = "长任务资料\n".repeat(2000);
            let mut request = ChatRequest::system_user("agent rules", &task, None, None);
            request.cache_system_prefix = true;
            request
                .configure_prompt_cache(provider, "gpt-5.6-luna", None, Some(7))
                .unwrap();
            assert_eq!(request.cache_mode, Some(PromptCacheMode::Implicit));
            assert_eq!(request.messages[1].content, task);
            let routing_key = request.cache_scope_key.clone();

            request.messages.extend([
                ChatMessage::assistant("继续确认"),
                ChatMessage::user("继续"),
            ]);
            request
                .configure_prompt_cache(provider, "gpt-5.6-luna", None, Some(7))
                .unwrap();
            assert_eq!(request.cache_mode, Some(PromptCacheMode::Implicit));
            assert_eq!(request.cache_scope_key, routing_key);
            assert!(request.cache_breakpoints.contains(&PromptCacheBreakpoint {
                message_index: 1,
                byte_offset: task.len(),
            }));
            assert!(request.cache_breakpoints.len() <= MAX_CACHE_WRITES - 1);
        }
        let mut request = ChatRequest::system_user("agent rules", "initial task", None, None);
        request
            .configure_prompt_cache("anthropic", "claude-alias", None, Some(7))
            .unwrap();
        assert_eq!(
            request.cached_text_parts(1).unwrap(),
            vec![("initial task", true)]
        );
    }

    #[test]
    fn cache_options_reject_foreign_protocols_and_invalid_modes() {
        let options = json!({"prompt_cache_options":{"mode":"explicit","ttl":"30m"}});
        for provider in [
            "gemini",
            "custom-gemini",
            "anthropic",
            "custom-anthropic",
            "deepseek",
        ] {
            assert!(
                validate_prompt_cache_options(provider, "alias", Some(&options))
                    .unwrap_err()
                    .to_string()
                    .contains("仅适用于 OpenAI")
            );
        }
        for (model, options) in [
            ("gpt-5.5", options.clone()),
            ("gpt-5.6-luna", json!({"prompt_cache_options": []})),
            (
                "gpt-5.6-luna",
                json!({"prompt_cache_options": {"mode": "invalid"}}),
            ),
        ] {
            assert!(validate_prompt_cache_options("openai", model, Some(&options)).is_err());
        }
        assert_eq!(
            validate_prompt_cache_options("custom-responses", "alias", Some(&options)).unwrap(),
            Some(PromptCacheMode::Explicit)
        );
        assert_eq!(
            validate_prompt_cache_options("gemini", "alias", None).unwrap(),
            None
        );
    }

    #[test]
    fn protocols_and_model_capabilities_control_cache_metadata() {
        for (provider, model, expected) in [
            ("anthropic", "claude-sonnet", true),
            ("custom-anthropic", "proxy-alias", true),
            ("openai-responses", "openai/gpt-5.6-luna", true),
            ("openai", "gpt-6-astra", true),
            ("openai", "gpt-5.5", false),
            ("deepseek", "deepseek-chat", false),
            ("gemini", "gemini-2.5-pro", false),
        ] {
            let mut request = ChatRequest::system_user("rules", "task", None, None);
            request
                .configure_prompt_cache(provider, model, None, Some(7))
                .unwrap();
            assert_eq!(
                !request.cache_breakpoints.is_empty(),
                expected,
                "{provider}/{model}"
            );
        }
        let mut request = ChatRequest::system_user("rules", "task", None, None);
        request
            .configure_prompt_cache(
                "custom-responses",
                "alias",
                Some(&json!({"prompt_cache_options":{"mode":"explicit"}})),
                Some(7),
            )
            .unwrap();
        assert_eq!(request.cache_mode, Some(PromptCacheMode::Explicit));
        assert!(!request.cache_breakpoints.is_empty());
    }

    #[test]
    fn layered_prefixes_preserve_text_and_routing_across_context_changes() {
        let mut request = ChatRequest::system_user("rules", "共享章节\n阶段一\n任务", None, None);
        request.add_cache_breakpoint(1, "共享章节".len()).unwrap();
        request
            .add_cache_breakpoint(1, "共享章节\n阶段一".len())
            .unwrap();
        request
            .configure_prompt_cache("openai", "gpt-5.6-luna", None, Some(7))
            .unwrap();
        let key = request.cache_scope_key.clone();
        assert_eq!(request.cache_breakpoints.len(), 3);
        assert_eq!(request.cache_mode, Some(PromptCacheMode::Explicit));
        assert_eq!(
            request.cached_text_parts(1).unwrap().last(),
            Some(&("\n任务", false))
        );
        assert_eq!(
            request
                .cached_text_parts(1)
                .unwrap()
                .iter()
                .map(|(text, _)| *text)
                .collect::<String>(),
            request.messages[1].content
        );
        request.messages[1].content = "共享章节\n阶段二\n任务".into();
        request
            .configure_prompt_cache("openai", "gpt-5.6-luna", None, Some(7))
            .unwrap();
        assert_eq!(request.cache_scope_key, key);
        assert!(request.add_cache_breakpoint(1, 1).is_err());
    }
}
