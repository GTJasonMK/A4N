//! Shared provider presets for configuration, discovery and request routing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProtocolKind {
    OpenAiChatCompletions,
    OpenAiResponses,
    AnthropicMessages,
    GeminiGenerateContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmProviderCapabilities {
    pub id: &'static str,
    pub display_name: &'static str,
    pub default_base_url: &'static str,
    pub protocol: LlmProtocolKind,
}

pub const LLM_PROVIDER_CAPABILITIES: &[LlmProviderCapabilities] = &[
    LlmProviderCapabilities {
        id: "openai",
        display_name: "OpenAI · Chat Completions",
        default_base_url: "https://api.openai.com/v1",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "openai-responses",
        display_name: "OpenAI · Responses",
        default_base_url: "https://api.openai.com/v1",
        protocol: LlmProtocolKind::OpenAiResponses,
    },
    LlmProviderCapabilities {
        id: "anthropic",
        display_name: "Anthropic Claude",
        default_base_url: "https://api.anthropic.com",
        protocol: LlmProtocolKind::AnthropicMessages,
    },
    LlmProviderCapabilities {
        id: "gemini",
        display_name: "Google Gemini",
        default_base_url: "https://generativelanguage.googleapis.com/v1beta",
        protocol: LlmProtocolKind::GeminiGenerateContent,
    },
    LlmProviderCapabilities {
        id: "deepseek",
        display_name: "DeepSeek",
        default_base_url: "https://api.deepseek.com/v1",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "qwen",
        display_name: "通义千问",
        default_base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "moonshot",
        display_name: "Moonshot / Kimi",
        default_base_url: "https://api.moonshot.cn/v1",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "zhipu",
        display_name: "智谱 GLM",
        default_base_url: "https://open.bigmodel.cn/api/paas/v4",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "doubao",
        display_name: "豆包 / 火山方舟",
        default_base_url: "https://ark.cn-beijing.volces.com/api/v3",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "siliconflow",
        display_name: "硅基流动",
        default_base_url: "https://api.siliconflow.cn/v1",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "ollama",
        display_name: "Ollama 本地",
        default_base_url: "http://127.0.0.1:11434/v1",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "custom-chat-completions",
        display_name: "自定义 · Chat Completions",
        default_base_url: "",
        protocol: LlmProtocolKind::OpenAiChatCompletions,
    },
    LlmProviderCapabilities {
        id: "custom-responses",
        display_name: "自定义 · Responses",
        default_base_url: "",
        protocol: LlmProtocolKind::OpenAiResponses,
    },
    LlmProviderCapabilities {
        id: "custom-anthropic",
        display_name: "自定义 · Anthropic Messages",
        default_base_url: "",
        protocol: LlmProtocolKind::AnthropicMessages,
    },
    LlmProviderCapabilities {
        id: "custom-gemini",
        display_name: "自定义 · Gemini",
        default_base_url: "",
        protocol: LlmProtocolKind::GeminiGenerateContent,
    },
];

impl LlmProtocolKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::OpenAiChatCompletions => "OpenAI Chat Completions",
            Self::OpenAiResponses => "OpenAI Responses",
            Self::AnthropicMessages => "Anthropic Messages",
            Self::GeminiGenerateContent => "Gemini GenerateContent",
        }
    }

    pub fn for_provider(provider: &str) -> Option<Self> {
        LLM_PROVIDER_CAPABILITIES
            .iter()
            .find(|preset| preset.id.eq_ignore_ascii_case(provider.trim()))
            .map(|preset| preset.protocol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_unique_ids_and_resolve_to_the_declared_protocol() {
        let mut ids = std::collections::HashSet::new();
        for preset in LLM_PROVIDER_CAPABILITIES {
            assert!(ids.insert(preset.id));
            assert_eq!(
                LlmProtocolKind::for_provider(&format!(" {} ", preset.id.to_uppercase())),
                Some(preset.protocol)
            );
            assert!(
                preset.id.starts_with("custom-") || preset.default_base_url.starts_with("http")
            );
        }
        assert_eq!(LlmProtocolKind::for_provider("unknown"), None);
    }
}
