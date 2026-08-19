mod catalog;
mod discovery;
mod embedding;
mod image;
mod llm;
mod shared;
mod video;

pub use catalog::model_provider_catalog;
pub use discovery::{
    detect_image_config_models, detect_llm_config_models, detect_video_config_models,
};
pub use embedding::{
    CreateEmbeddingInput, EmbeddingConfigSummary, UpdateEmbeddingInput,
    create_embedding_config, delete_embedding_config, list_embedding_config,
    preview_test_embedding_config_request, set_default_embedding_config, test_embedding_config,
    update_embedding_config,
};
pub use image::{
    ImageConfigInput, ImageConfigSummary, UpdateImageConfigInput, add_image_config,
    list_image_configs, preview_test_image_config_request, remove_image_config,
    set_default_image_config, test_image_config, update_image_config,
};
pub use llm::{
    CreateLlmInput, LlmConfigSummary, LlmMaxTokensConfig, UpdateLlmInput, create_llm_config,
    delete_llm_config, get_llm_max_tokens_config, list_llm_config,
    preview_test_llm_config_request, set_default_llm_config, test_llm_config,
    update_llm_config, update_llm_max_tokens_config,
};
pub use shared::{ModelDiscoveryInput, ModelRequestPreview, ModelRequestPreviewStep};
pub use video::{
    UpdateVideoConfigInput, VideoConfigInput, VideoConfigSummary, add_video_config,
    list_video_configs, remove_video_config, set_default_video_config, update_video_config,
};
