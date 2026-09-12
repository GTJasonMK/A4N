mod embedding;
mod image;
mod llm;
mod settings_all;
mod shared;
mod video;
mod web_search;

pub(in crate::routes::commands) use embedding::{
    create_embedding_config, delete_embedding_config, list_embedding_config,
    list_model_provider_catalog, preview_test_embedding_config_request,
    set_default_embedding_config, test_embedding_config, update_embedding_config,
};
pub(in crate::routes::commands) use image::{
    add_image_config, detect_image_config_models, list_image_configs,
    preview_test_image_config_request, remove_image_config, set_default_image_config,
    test_image_config, update_image_config,
};
pub(in crate::routes::commands) use llm::{
    create_llm_config, delete_llm_config, detect_llm_config_models, get_llm_max_tokens_config,
    list_llm_config, preview_test_llm_config_request, set_default_llm_config, test_llm_config,
    update_llm_config, update_llm_max_tokens_config,
};
pub(in crate::routes::commands) use settings_all::{export_all_configs_json, import_all_configs};
pub(in crate::routes::commands) use video::{
    add_video_config, detect_video_config_models, list_video_configs, remove_video_config,
    set_default_video_config, update_video_config,
};
pub(in crate::routes::commands) use web_search::{
    get_web_search_config, update_web_search_config, web_search,
};

#[cfg(test)]
mod tests;
