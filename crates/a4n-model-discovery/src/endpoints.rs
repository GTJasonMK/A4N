#[derive(Debug, Clone, Copy)]
pub(crate) enum ModelDiscoveryAuth {
    Bearer,
    Anthropic,
    GeminiApiKey,
    None,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ModelDiscoveryParser {
    OpenAi,
    Gemini,
    StableDiffusion,
    ComfyUiCheckpointLoader,
}

#[derive(Debug, Clone)]
pub(crate) struct ModelDiscoveryEndpoint {
    pub(crate) url: String,
    pub(crate) label: &'static str,
    pub(crate) auth: ModelDiscoveryAuth,
    pub(crate) parser: ModelDiscoveryParser,
}

pub(crate) fn model_discovery_endpoints(
    provider: &str,
    base_url: &str,
) -> Vec<ModelDiscoveryEndpoint> {
    let provider = provider.trim().to_ascii_lowercase();
    if provider == "stable-diffusion" {
        return vec![ModelDiscoveryEndpoint {
            url: format!("{base_url}/sdapi/v1/sd-models"),
            label: "Stable Diffusion 模型列表端点",
            auth: ModelDiscoveryAuth::None,
            parser: ModelDiscoveryParser::StableDiffusion,
        }];
    }
    if provider == "comfyui" {
        return vec![ModelDiscoveryEndpoint {
            url: format!("{base_url}/object_info/CheckpointLoaderSimple"),
            label: "ComfyUI checkpoint 列表端点",
            auth: ModelDiscoveryAuth::None,
            parser: ModelDiscoveryParser::ComfyUiCheckpointLoader,
        }];
    }
    if provider == "anthropic" {
        return vec![ModelDiscoveryEndpoint {
            url: format!("{}/v1/models", base_url.trim_end_matches('/')),
            label: "Anthropic 模型列表端点",
            auth: ModelDiscoveryAuth::Anthropic,
            parser: ModelDiscoveryParser::OpenAi,
        }];
    }
    if provider.contains("gemini") || base_url.contains("generativelanguage.googleapis.com") {
        return vec![ModelDiscoveryEndpoint {
            url: format!("{}/models", gemini_api_base_url(base_url)),
            label: "Gemini 模型列表端点",
            auth: ModelDiscoveryAuth::GeminiApiKey,
            parser: ModelDiscoveryParser::Gemini,
        }];
    }

    openai_model_list_urls(base_url)
        .into_iter()
        .map(|url| ModelDiscoveryEndpoint {
            url,
            label: "OpenAI-compatible 模型列表端点",
            auth: ModelDiscoveryAuth::Bearer,
            parser: ModelDiscoveryParser::OpenAi,
        })
        .collect()
}

fn gemini_api_base_url(base_url: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    if base_url.ends_with("/models") {
        return base_url
            .trim_end_matches("/models")
            .trim_end_matches('/')
            .to_string();
    }
    if base_url.ends_with("/v1beta") {
        base_url.to_string()
    } else if let Some(prefix) = base_url.strip_suffix("/v1") {
        format!("{prefix}/v1beta")
    } else {
        format!("{base_url}/v1beta")
    }
}

pub(crate) fn openai_model_list_urls(base_url: &str) -> Vec<String> {
    let base = strip_known_model_endpoint_suffix(base_url);
    let mut urls = Vec::new();
    let primary = if base.ends_with("/models") {
        base.clone()
    } else {
        format!("{base}/models")
    };
    push_unique_url(&mut urls, primary);
    if !base.ends_with("/v1")
        && !base.ends_with("/v1beta")
        && !base.ends_with("/models")
        && !base.contains("/v1/")
        && !base.contains("/v1beta/")
    {
        push_unique_url(&mut urls, format!("{base}/v1/models"));
    }
    urls
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
    loop {
        let lower = base.to_ascii_lowercase();
        let Some(suffix) = suffixes.iter().find(|suffix| lower.ends_with(**suffix)) else {
            return base;
        };
        let next_len = base.len() - suffix.len();
        base.truncate(next_len);
        base = base.trim_end_matches('/').to_string();
    }
}

fn push_unique_url(urls: &mut Vec<String>, url: String) {
    if !urls.iter().any(|existing| existing == &url) {
        urls.push(url);
    }
}
