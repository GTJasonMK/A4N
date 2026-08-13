use a4n_api_contract::provider_catalog::{
    EmbeddingProviderDescriptor, ImageProviderDescriptor, LlmProviderDescriptor,
    ModelProviderCatalog, VideoProviderDescriptor,
};
use a4n_core::config::{EMBEDDING_PROVIDER_CAPABILITIES, IMAGE_PROVIDER_CAPABILITIES};
use a4n_llm::provider::LLM_PROVIDER_CAPABILITIES;
use a4n_video::VIDEO_PROVIDER_CAPABILITIES;

pub fn model_provider_catalog() -> ModelProviderCatalog {
    ModelProviderCatalog {
        llm: LLM_PROVIDER_CAPABILITIES
            .iter()
            .map(|capabilities| LlmProviderDescriptor {
                id: capabilities.id.into(),
                display_name: capabilities.display_name.into(),
                default_base_url: capabilities.default_base_url.into(),
            })
            .collect(),
        image: IMAGE_PROVIDER_CAPABILITIES
            .iter()
            .map(|capabilities| ImageProviderDescriptor {
                id: capabilities.backend.into(),
                display_name: capabilities.display_name.into(),
                requires_api_key: capabilities.requires_api_key,
            })
            .collect(),
        video: VIDEO_PROVIDER_CAPABILITIES
            .iter()
            .map(|capabilities| VideoProviderDescriptor {
                id: capabilities.backend.into(),
                display_name: capabilities.display_name.into(),
                min_duration_secs: capabilities.min_duration_secs as u32,
                max_duration_secs: capabilities.max_duration_secs as u32,
            })
            .collect(),
        embedding: EMBEDDING_PROVIDER_CAPABILITIES
            .iter()
            .map(|capabilities| EmbeddingProviderDescriptor {
                id: capabilities.backend.into(),
                display_name: capabilities.display_name.into(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
