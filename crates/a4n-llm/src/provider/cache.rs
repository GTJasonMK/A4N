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
        self.cache_breakpoints = self.effective_cache_breakpoints()?;
        self.cache_system_prefix = false;
        self.cache_user_prefix_len = None;
        self.cache_scope_key = None;
        self.cache_mode = None;

        let openai = protocol == LlmProtocolKind::OpenAiResponses
            || (protocol == LlmProtocolKind::OpenAiChatCompletions
                && matches!(
                    provider.trim().to_ascii_lowercase().as_str(),
                    "openai" | "custom-chat-completions"
                ));
        let cache_options = options.and_then(|value| value.get("prompt_cache_options"));
        if cache_options.is_some_and(|value| !value.is_object()) {
            return Err(AppError::ConfigError(
                "prompt_cache_options 必须是对象".into(),
            ));
        }
        let generation = openai_generation(model);
        // Native Claude supports block caching regardless of its model alias.
        // OpenAI cache_options also allow an explicit capability declaration for
        // proxy aliases, while known older models retain their automatic cache.
        let explicit_openai =
            openai && generation.map_or(cache_options.is_some(), |version| version >= (5, 6));
        if openai && cache_options.is_some() && !explicit_openai {
            return Err(AppError::ConfigError(
                "当前 OpenAI 模型不支持显式缓存选项".into(),
            ));
        }
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
        for (index, message) in self.messages.iter().enumerate() {
            if matches!(message.role, MessageRole::System) && !message.content.is_empty() {
                self.cache_breakpoints.push(PromptCacheBreakpoint {
                    message_index: index,
                    byte_offset: message.content.len(),
                });
            }
        }
        if explicit_openai {
            let mode = match cache_options.and_then(|options| options.get("mode")) {
                Some(Value::String(mode)) if mode == "explicit" => PromptCacheMode::Explicit,
                Some(Value::String(mode)) if mode == "implicit" => PromptCacheMode::Implicit,
                Some(_) => {
                    return Err(AppError::ConfigError(
                        "prompt_cache_options.mode 必须为 explicit 或 implicit".into(),
                    ));
                }
                None if self.cache_breakpoints.is_empty()
                    || self.messages.iter().any(|message| {
                        matches!(message.role, MessageRole::Assistant | MessageRole::Tool)
                    }) =>
                {
                    PromptCacheMode::Implicit
                }
                None => PromptCacheMode::Explicit,
            };
            self.cache_mode = Some(mode);
        }
        if self
            .messages
            .iter()
            .any(|message| matches!(message.role, MessageRole::Assistant | MessageRole::Tool))
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
    use serde_json::json;

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
