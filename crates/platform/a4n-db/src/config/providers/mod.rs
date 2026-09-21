mod baseline;
mod defaults;
mod embedding;
mod image;
mod llm;
mod video;

pub use baseline::{
    image_configs_exist, insert_baseline_embedding_config_if_empty,
    insert_baseline_image_config_if_empty, insert_baseline_llm_config_if_empty,
    insert_baseline_video_config_if_empty, llm_configs_exist, video_configs_exist,
};
pub(crate) use defaults::clear_provider_config_defaults;
pub use defaults::{ProviderConfigKind, set_default_provider_config};
pub(crate) use embedding::insert_embedding_config_record;
pub use embedding::{
    EmbeddingConfigRecordInput, create_embedding_config_record, default_embedding_config,
    default_embedding_config_optional, delete_embedding_config_record, embedding_config_by_id,
    list_embedding_configs, update_embedding_config_record, validate_embedding_config_model,
};
pub(crate) use image::insert_image_config_record;
pub use image::{
    ImageConfigRecordInput, create_image_config_record, delete_image_config_record,
    image_config_by_id, list_image_config_records, resolve_image_config,
    update_image_config_record, validate_image_config_model,
};
pub(crate) use llm::insert_llm_config_record;
pub use llm::{
    LlmConfigRecordInput, create_llm_config_record, delete_llm_config_record,
    list_llm_config_records, llm_config_by_id, resolve_llm_config, update_llm_config_record,
};
pub(crate) use video::insert_video_config_record;
pub use video::{
    VideoConfigRecordInput, active_generated_video_ids_for_config, create_video_config_record,
    default_video_config, delete_video_config_record, list_video_config_records,
    update_video_config_record, video_config_by_id,
};
