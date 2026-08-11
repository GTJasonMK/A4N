use super::super::*;

pub(super) async fn try_dispatch(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Option<Value>, ApiError> {
    let result = match cmd {
        "list_themes" => list_themes(state, user_id, args).await?,
        "create_theme" => create_theme(state, user_id, args).await?,
        "update_theme" => update_theme(state, user_id, args).await?,
        "delete_theme" => delete_theme(state, user_id, args).await?,
        "activate_theme" => activate_theme(state, user_id, args).await?,
        "export_theme" => export_theme(state, user_id, args).await?,
        "import_theme" => import_theme(state, user_id, args).await?,
        "export_all_configs_json" => export_all_configs_json(state, user_id, args).await?,
        "import_all_configs" => import_all_configs(state, user_id, args).await?,
        "list_llm_config" => list_llm_config(state, user_id, args).await?,
        "create_llm_config" => create_llm_config(state, user_id, args).await?,
        "update_llm_config" => update_llm_config(state, user_id, args).await?,
        "set_default_llm_config" => set_default_llm_config(state, user_id, args).await?,
        "delete_llm_config" => delete_llm_config(state, user_id, args).await?,
        "get_llm_max_tokens_config" => get_llm_max_tokens_config(state, user_id, args).await?,
        "update_llm_max_tokens_config" => {
            update_llm_max_tokens_config(state, user_id, args).await?
        }
        "list_embedding_config" => list_embedding_config(state, user_id, args).await?,
        "create_embedding_config" => create_embedding_config(state, user_id, args).await?,
        "update_embedding_config" => update_embedding_config(state, user_id, args).await?,
        "set_default_embedding_config" => {
            set_default_embedding_config(state, user_id, args).await?
        }
        "delete_embedding_config" => delete_embedding_config(state, user_id, args).await?,
        "list_model_provider_catalog" => list_model_provider_catalog(state, user_id, args).await?,
        "rag_enhanced_search" => rag_enhanced_search(state, user_id, args).await?,
        "rag_prepare_enhanced_search" => rag_prepare_enhanced_search(state, user_id, args).await?,
        "rag_enhanced_search_with_vectors" => {
            rag_enhanced_search_with_vectors(state, user_id, args).await?
        }
        "rag_ingest_all" => rag_ingest_all(state, user_id, args).await?,
        "rag_prepare_ingest_all" => rag_prepare_ingest_all(state, user_id, args).await?,
        "rag_upload_ingest_embeddings" => {
            rag_upload_ingest_embeddings(state, user_id, args).await?
        }
        "test_embedding_config" => test_embedding_config(state, user_id, args).await?,
        "preview_test_embedding_config_request" => {
            preview_test_embedding_config_request(state, user_id, args).await?
        }
        "list_local_embedding_models" => list_local_embedding_models(state, user_id, args).await?,
        "get_embedding_model_download_info" => {
            get_embedding_model_download_info(state, user_id, args).await?
        }
        "download_embedding_model" => download_embedding_model(state, user_id, args).await?,
        "cancel_embedding_model_download" => {
            cancel_embedding_model_download(state, user_id, args).await?
        }
        "delete_local_embedding_model" => {
            delete_local_embedding_model(state, user_id, args).await?
        }
        "test_llm_config" => test_llm_config(state, user_id, args).await?,
        "preview_test_llm_config_request" => {
            preview_test_llm_config_request(state, user_id, args).await?
        }
        "detect_llm_config_models" => detect_llm_config_models(state, user_id, args).await?,
        "get_storage_paths" => get_storage_paths(state, user_id, args).await?,
        "prepare_chapter_generation_rag_embedding" => {
            prepare_chapter_generation_rag_embedding(state, user_id, args).await?
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
