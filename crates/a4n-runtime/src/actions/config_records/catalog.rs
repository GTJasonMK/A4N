use a4n_api_contract::provider_catalog::{
    EmbeddingProviderDescriptor, ImageProviderDescriptor, LlmProviderDescriptor,
    ModelProviderCatalog, VideoProviderDescriptor,
};
use a4n_orchestrator::config::image_backend_requires_api_key;

pub fn model_provider_catalog() -> ModelProviderCatalog {
    ModelProviderCatalog {
        llm: vec![
            llm("openai", "OpenAI / 兼容协议", "https://api.openai.com/v1"),
            llm("anthropic", "Anthropic Claude", "https://api.anthropic.com"),
            llm("deepseek", "DeepSeek", "https://api.deepseek.com/v1"),
            llm(
                "qwen",
                "通义千问",
                "https://dashscope.aliyuncs.com/compatible-mode/v1",
            ),
            llm("ollama", "Ollama 本地", "http://127.0.0.1:11434/v1"),
        ],
        image: vec![
            image("openai-compatible-image", "OpenAI 兼容图像"),
            image("openai-compatible-image-edit", "OpenAI 兼容图像编辑"),
            image("openai-chat-image", "Chat Completions 图像"),
            image("cliproxy-image", "CLIProxy / Grok 图片"),
            image("stable-diffusion", "Stable Diffusion WebUI"),
            image("comfyui", "ComfyUI"),
        ],
        video: vec![
            video("openai-compatible-video", "OpenAI 兼容视频", 1, 60),
            video("newapi-video", "New API 视频", 1, 60),
            video("gemini-veo-video", "Gemini Veo 视频", 1, 60),
            video("seedance2-video", "Seedance 2.0 参考媒体", 4, 15),
            video("cliproxy-openai-video", "CLIProxy / Grok 视频", 1, 15),
            video("openai-chat-video", "Chat Completions 视频", 1, 60),
        ],
        embedding: vec![
            embedding("api", "OpenAI 兼容 API"),
            embedding("fastembed", "FastEmbed 本地模型"),
            embedding("local_hash", "本地哈希（开发）"),
        ],
    }
}

fn llm(id: &str, display_name: &str, default_base_url: &str) -> LlmProviderDescriptor {
    LlmProviderDescriptor {
        id: id.into(),
        display_name: display_name.into(),
        default_base_url: default_base_url.into(),
    }
}

fn image(id: &str, display_name: &str) -> ImageProviderDescriptor {
    ImageProviderDescriptor {
        id: id.into(),
        display_name: display_name.into(),
        requires_api_key: image_backend_requires_api_key(id),
    }
}

fn video(
    id: &str,
    display_name: &str,
    min_duration_secs: u32,
    max_duration_secs: u32,
) -> VideoProviderDescriptor {
    VideoProviderDescriptor {
        id: id.into(),
        display_name: display_name.into(),
        min_duration_secs,
        max_duration_secs,
    }
}

fn embedding(id: &str, display_name: &str) -> EmbeddingProviderDescriptor {
    EmbeddingProviderDescriptor {
        id: id.into(),
        display_name: display_name.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::config::{
        CreateEmbeddingConfigParams, CreateImageConfigParams, validate_embedding_config_params,
        validate_image_config_params,
    };
    use a4n_llm::provider::LlmProtocolKind;
    use a4n_video::video_provider_cleanup_capabilities;
    use std::collections::HashSet;

    #[test]
    fn provider_ids_are_unique_within_each_capability() {
        let catalog = model_provider_catalog();
        for ids in [
            catalog
                .llm
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            catalog.image.iter().map(|item| item.id.as_str()).collect(),
            catalog.video.iter().map(|item| item.id.as_str()).collect(),
            catalog
                .embedding
                .iter()
                .map(|item| item.id.as_str())
                .collect(),
        ] {
            assert_eq!(ids.len(), ids.iter().copied().collect::<HashSet<_>>().len());
        }
    }

    #[test]
    fn catalog_provider_ids_are_accepted_by_request_routers() {
        let catalog = model_provider_catalog();

        for provider in catalog.llm {
            assert!(
                LlmProtocolKind::for_provider(&provider.id).is_some(),
                "LLM provider is not routable: {}",
                provider.id
            );
        }
        for provider in catalog.image {
            assert!(
                validate_image_config_params(CreateImageConfigParams {
                    name: "catalog-test".into(),
                    backend: provider.id.clone(),
                    base_url: "http://127.0.0.1:1".into(),
                    api_key_encrypted: None,
                    model: "catalog-test".into(),
                    max_concurrent: 1,
                    default_width: 1,
                    default_height: 1,
                })
                .is_ok(),
                "image provider is not routable: {}",
                provider.id
            );
        }
        for provider in catalog.video {
            assert!(
                video_provider_cleanup_capabilities(&provider.id).is_some(),
                "video provider is not routable: {}",
                provider.id
            );
        }
        for provider in catalog.embedding {
            assert!(
                validate_embedding_config_params(CreateEmbeddingConfigParams {
                    name: "catalog-test".into(),
                    backend: provider.id.clone(),
                    base_url: (provider.id == "api").then(|| "http://127.0.0.1:1".into()),
                    api_key_encrypted: None,
                    model: "catalog-test".into(),
                    dimension: 1,
                    batch_size: 1,
                    local_model_path: None,
                })
                .is_ok(),
                "embedding provider is not routable: {}",
                provider.id
            );
        }
    }
}
