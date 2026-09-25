use a4n_domain::source_text_hash;
use a4n_error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ChatRequest, LlmProtocolKind, LlmProviderCapabilities, MessageRole};

const MAX_CACHE_WRITES: usize = 4;
pub const PROMPT_CACHE_OPTIONS_FIELD: &str = "prompt_cache_options";
pub const PROMPT_CACHE_MODE_FIELD: &str = "mode";

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
    options: Option<&Value>,
) -> Result<Option<PromptCacheMode>, AppError> {
    let Some(options) = options.and_then(|value| value.get(PROMPT_CACHE_OPTIONS_FIELD)) else {
        return Ok(None);
    };
    if !provider_supports_prompt_cache_options(provider) {
        return Err(AppError::ConfigError(format!(
            "prompt_cache_options 仅适用于 OpenAI Chat Completions/Responses，当前 provider 为 {provider}"
        )));
    }
    let options = options
        .as_object()
        .ok_or_else(|| AppError::ConfigError("prompt_cache_options 必须是对象".into()))?;
    match options.get(PROMPT_CACHE_MODE_FIELD) {
        None => Ok(None),
        Some(mode) => serde_json::from_value(mode.clone()).map(Some).map_err(|_| {
            AppError::ConfigError("prompt_cache_options.mode 必须为 explicit 或 implicit".into())
        }),
    }
}

fn provider_supports_prompt_cache_options(provider: &str) -> bool {
    LlmProviderCapabilities::for_provider(provider)
        .is_some_and(|capabilities| capabilities.supports_prompt_cache_options)
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
        let configured_mode = validate_prompt_cache_options(provider, options)?;
        self.cache_breakpoints = self.normalized_cache_breakpoints()?;
        self.cache_scope_key = None;
        self.cache_mode = None;

        let cache_options = options.and_then(|value| value.get(PROMPT_CACHE_OPTIONS_FIELD));
        let openai_cache_enabled =
            provider_supports_prompt_cache_options(provider) && cache_options.is_some();
        if !openai_cache_enabled && protocol != LlmProtocolKind::AnthropicMessages {
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
        if openai_cache_enabled {
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
        let points = self.normalized_cache_breakpoints()?;
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

    pub(super) fn normalized_cache_breakpoints(
        &self,
    ) -> Result<Vec<PromptCacheBreakpoint>, AppError> {
        let mut points = self.cache_breakpoints.clone();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ChatMessage;
    use serde_json::json;

    #[test]
    fn native_dialogue_caches_the_initial_task_before_history_exists() {
        let options = json!({"prompt_cache_options": {}});
        for provider in ["openai", "openai-responses"] {
            let task = "长任务资料\n".repeat(2000);
            let mut request = ChatRequest::system_user("agent rules", &task, None, None);
            request
                .configure_prompt_cache(provider, "proxy-alias", Some(&options), Some(7))
                .unwrap();
            assert_eq!(request.cache_mode, Some(PromptCacheMode::Implicit));
            assert_eq!(request.messages[1].content, task);
            let routing_key = request.cache_scope_key.clone();

            request.messages.extend([
                ChatMessage::assistant("继续确认"),
                ChatMessage::user("继续"),
            ]);
            request
                .configure_prompt_cache(provider, "proxy-alias", Some(&options), Some(7))
                .unwrap();
            assert_eq!(request.cache_mode, Some(PromptCacheMode::Implicit));
            assert_eq!(request.cache_scope_key, routing_key);
            assert!(request.cache_breakpoints.contains(&PromptCacheBreakpoint {
                message_index: 1,
                byte_offset: task.len(),
            }));
            assert!(request.cache_breakpoints.len() < MAX_CACHE_WRITES);
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
                validate_prompt_cache_options(provider, Some(&options))
                    .unwrap_err()
                    .to_string()
                    .contains("仅适用于 OpenAI")
            );
        }
        assert_eq!(
            validate_prompt_cache_options("openai", Some(&options)).unwrap(),
            Some(PromptCacheMode::Explicit)
        );
        for options in [
            json!({"prompt_cache_options": []}),
            json!({"prompt_cache_options": {"mode": "invalid"}}),
        ] {
            assert!(validate_prompt_cache_options("openai", Some(&options)).is_err());
        }
        assert_eq!(
            validate_prompt_cache_options("custom-responses", Some(&options)).unwrap(),
            Some(PromptCacheMode::Explicit)
        );
        assert_eq!(validate_prompt_cache_options("gemini", None).unwrap(), None);
    }

    #[test]
    fn cache_metadata_requires_explicit_configuration_for_openai_protocols() {
        for (provider, model, expected) in [
            ("anthropic", "claude-sonnet", true),
            ("custom-anthropic", "proxy-alias", true),
            ("openai-responses", "openai/gpt-5.6-luna", false),
            ("openai", "gpt-6-astra", false),
            ("openai", "gpt-6-luna", false),
            ("custom-chat-completions", "gpt-6-luna", false),
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
        let options = json!({"prompt_cache_options": {"mode": "explicit"}});
        for provider in [
            "openai",
            "openai-responses",
            "custom-chat-completions",
            "custom-responses",
        ] {
            let mut request = ChatRequest::system_user("rules", "task", None, None);
            request
                .configure_prompt_cache(provider, "gpt-6-luna", Some(&options), Some(7))
                .unwrap();
            assert_eq!(request.cache_mode, Some(PromptCacheMode::Explicit));
            assert!(!request.cache_breakpoints.is_empty());
        }
    }

    #[test]
    fn layered_prefixes_preserve_text_and_routing_across_context_changes() {
        let options = json!({"prompt_cache_options": {"mode": "explicit"}});
        let mut request = ChatRequest::system_user("rules", "共享章节\n阶段一\n任务", None, None);
        request.add_cache_breakpoint(1, "共享章节".len()).unwrap();
        request
            .add_cache_breakpoint(1, "共享章节\n阶段一".len())
            .unwrap();
        request
            .configure_prompt_cache("openai", "proxy-alias", Some(&options), Some(7))
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
            .configure_prompt_cache("openai", "proxy-alias", Some(&options), Some(7))
            .unwrap();
        assert_eq!(request.cache_scope_key, key);
        assert!(request.add_cache_breakpoint(1, 1).is_err());
    }
}
