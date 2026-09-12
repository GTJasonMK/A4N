#[derive(Clone, Copy)]
pub(crate) enum ModelDiscoveryAuth {
    Bearer,
    Anthropic,
    GeminiApiKey,
    None,
}

#[derive(Clone, Copy)]
pub(crate) enum ModelDiscoveryParser {
    DataList,
    Gemini,
    StableDiffusion,
    ComfyUiCheckpointLoader,
}

pub(crate) struct ModelDiscoveryEndpoint {
    pub(crate) url: String,
    pub(crate) label: &'static str,
    pub(crate) auth: ModelDiscoveryAuth,
    pub(crate) parser: ModelDiscoveryParser,
}

pub(crate) fn model_discovery_endpoint(provider: &str, base_url: &str) -> ModelDiscoveryEndpoint {
    let provider = provider.trim().to_ascii_lowercase();
    if provider == "stable-diffusion" {
        return ModelDiscoveryEndpoint {
            url: format!("{base_url}/sdapi/v1/sd-models"),
            label: "Stable Diffusion 模型列表端点",
            auth: ModelDiscoveryAuth::None,
            parser: ModelDiscoveryParser::StableDiffusion,
        };
    }
    if provider == "comfyui" {
        return ModelDiscoveryEndpoint {
            url: format!("{base_url}/object_info/CheckpointLoaderSimple"),
            label: "ComfyUI checkpoint 列表端点",
            auth: ModelDiscoveryAuth::None,
            parser: ModelDiscoveryParser::ComfyUiCheckpointLoader,
        };
    }
    let protocol = LlmProtocolKind::for_provider(&provider);
    if protocol == Some(LlmProtocolKind::AnthropicMessages) {
        return ModelDiscoveryEndpoint {
            url: format!("{}/v1/models", base_url.trim_end_matches('/')),
            label: "Anthropic 模型列表端点",
            auth: ModelDiscoveryAuth::Anthropic,
            parser: ModelDiscoveryParser::DataList,
        };
    }
    if protocol == Some(LlmProtocolKind::GeminiGenerateContent) || provider == "gemini-veo-video" {
        return ModelDiscoveryEndpoint {
            url: format!(
                "{}/models",
                if provider == "gemini-veo-video" {
                    gemini_api_base_url(base_url)
                } else {
                    base_url.trim_end_matches('/').to_string()
                }
            ),
            label: "Gemini 模型列表端点",
            auth: ModelDiscoveryAuth::GeminiApiKey,
            parser: ModelDiscoveryParser::Gemini,
        };
    }

    ModelDiscoveryEndpoint {
        url: openai_model_list_url(base_url),
        label: "OpenAI-compatible 模型列表端点",
        auth: ModelDiscoveryAuth::Bearer,
        parser: ModelDiscoveryParser::DataList,
    }
}

fn gemini_api_base_url(base_url: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    if base_url.ends_with("/v1beta") {
        base_url.to_string()
    } else if let Some(prefix) = base_url.strip_suffix("/v1") {
        format!("{prefix}/v1beta")
    } else {
        format!("{base_url}/v1beta")
    }
}

pub(crate) fn openai_model_list_url(base_url: &str) -> String {
    let base = strip_known_model_endpoint_suffix(base_url);
    if base.ends_with("/models") {
        base
    } else {
        format!("{base}/models")
    }
}

fn strip_known_model_endpoint_suffix(base_url: &str) -> String {
    let suffixes = [
        "/chat/completions",
        "/responses",
        "/images/generations",
        "/images/edits",
        "/videos/generations",
        "/video/generations",
        "/videos",
        "/task/submit",
    ];
    let mut base = base_url.trim_end_matches('/').to_string();
    if let Some(suffix) = suffixes.iter().find(|suffix| base.ends_with(**suffix)) {
        let next_len = base.len() - suffix.len();
        base.truncate(next_len);
        base = base.trim_end_matches('/').to_string();
    }
    base
}
use a4n_domain::llm::LlmProtocolKind;
