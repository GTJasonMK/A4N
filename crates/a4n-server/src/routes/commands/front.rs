use super::*;

const SYSTEM_DEFAULT_CONFIG_ID: i64 = -1;

pub(super) async fn oauth2_authorize_url(
    _state: &ServerState,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: oauth_actions::OAuth2AuthorizeInput = command_input(args, "oauth2_authorize_url")?;
    Ok(json!(oauth_actions::oauth2_authorize_url(input)?))
}

pub(super) async fn oauth2_callback(
    state: &ServerState,
    headers: &HeaderMap,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: oauth_actions::OAuth2CallbackInput = command_input(args, "oauth2_callback")?;
    let user = oauth_actions::oauth2_login_user(&state.auth, input).await?;
    let response = issue_auth_response(state, headers, user).await?;
    to_value(response)
}

pub(super) async fn get_user(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let user_id = i64_arg(args, "userId")?;
    to_value(auth_records_actions::get_user(state, user_id).await?)
}

pub(super) async fn update_user_profile(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let target_user_id = i64_arg(args, "userId")?;
    let input: auth_records_actions::UpdateProfileInput =
        command_input(args, "update_user_profile")?;
    to_value(
        auth_records_actions::update_user_profile_for_user(state, user_id, target_user_id, input)
            .await?,
    )
}

pub(super) async fn change_user_password(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let target_user_id = i64_arg(args, "userId")?;
    let input: auth_records_actions::ChangePasswordInput =
        command_input(args, "change_user_password")?;
    auth_records_actions::change_user_password_for_user(state, user_id, target_user_id, input)
        .await?;
    Ok(Value::Null)
}

pub(super) async fn list_sessions(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let target_user_id = i64_arg(args, "userId")?;
    to_value(auth_records_actions::list_sessions_for_user(state, user_id, target_user_id).await?)
}

pub(super) async fn revoke_session(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let target_user_id = i64_arg(args, "userId")?;
    let session_id = string_arg(args, "sessionId")?;
    auth_records_actions::revoke_session_for_user(state, user_id, target_user_id, &session_id)
        .await?;
    Ok(Value::Null)
}

macro_rules! theme_web_command {
    ($name:ident($state:ident, $user_id:ident, $args:ident) $body:block) => {
        pub(super) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> $body
    };
}

theme_web_command!(list_themes(state, user_id, _args) {
    to_value(theme_actions::list_themes_for_user(state, user_id).await?)
});
theme_web_command!(get_system_default_theme(state, _user_id, _args) {
    to_value(
        theme_actions::get_system_default_theme(state)
            .await
            .map_err(ApiError::bad_request)?,
    )
});
theme_web_command!(list_user_themes(state, user_id, args) {
    let target_user_id = i64_arg(args, "userId")?;
    let themes = theme_actions::list_user_themes_for_user(state, user_id, target_user_id).await?;
    to_value(themes)
});
theme_web_command!(create_theme(state, user_id, args) {
    let input: theme_actions::CreateThemeInput = command_input(args, "create_theme")?;
    to_value(theme_actions::create_theme_for_user(state, user_id, input).await?)
});
theme_web_command!(update_theme(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    let input: theme_actions::UpdateThemeInput = command_input(args, "update_theme")?;
    to_value(theme_actions::update_theme_for_user(state, user_id, theme_id, input).await?)
});
theme_web_command!(delete_theme(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    theme_actions::delete_theme_for_user(state, user_id, theme_id).await?;
    Ok(Value::Null)
});
theme_web_command!(activate_theme(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    to_value(theme_actions::activate_theme_for_user(state, user_id, theme_id).await?)
});
theme_web_command!(export_theme(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    Ok(json!(
        theme_actions::export_theme_for_user(state, user_id, theme_id).await?
    ))
});
theme_web_command!(import_theme(state, user_id, args) {
    let input: theme_actions::ImportThemeInput = command_input(args, "import_theme")?;
    to_value(theme_actions::import_theme_for_user(state, user_id, input).await?)
});
theme_web_command!(get_theme_css_variables(state, user_id, args) {
    let theme_id = i64_arg(args, "themeId")?;
    Ok(json!(
        theme_actions::get_theme_css_variables_for_user(state, user_id, theme_id).await?
    ))
});

pub(super) async fn export_all_configs_json(
    state: &ServerState,
    _user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    Ok(json!(
        a4n_runtime::actions::settings_all::export_all_configs_to_json(state).await?
    ))
}

pub(super) async fn import_all_configs(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: a4n_runtime::actions::settings_all::ImportAllConfigsInput =
        command_input(args, "import_all_configs")?;
    Ok(json!(
        a4n_runtime::actions::settings_all::import_all_configs(state, input).await?
    ))
}

macro_rules! config_web_no_arg_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            _user_id: i64,
            _args: &Value,
        ) -> Result<Value, ApiError> {
            to_value($action(state).await.map_err(ApiError::bad_request)?)
        }
    };
}

macro_rules! config_web_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            require_global_config_admin(state, user_id).await?;
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state, input).await.map_err(ApiError::bad_request)?)
        }
    };
}

macro_rules! config_web_update_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            require_global_config_admin(state, user_id).await?;
            let config_id = i64_arg(args, "configId")?;
            let input: $input = command_input(args, stringify!($name))?;
            to_value(
                $action(state, config_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! config_web_id_value_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            _user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let config_id = i64_arg(args, "configId")?;
            to_value(
                $action(state, config_id)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! config_web_admin_id_value_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            require_global_config_admin(state, user_id).await?;
            let config_id = i64_arg(args, "configId")?;
            to_value(
                $action(state, config_id)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! config_web_id_unit_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            require_global_config_admin(state, user_id).await?;
            let config_id = i64_arg(args, "configId")?;
            $action(state, config_id)
                .await
                .map_err(ApiError::bad_request)?;
            Ok(Value::Null)
        }
    };
}

macro_rules! config_web_model_discovery_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: config_records_actions::ModelDiscoveryInput =
                command_input(args, stringify!($name))?;
            if input.use_saved_api_key {
                require_global_config_admin(state, user_id).await?;
            }
            to_value($action(state, input).await.map_err(ApiError::bad_request)?)
        }
    };
}

async fn require_global_config_admin(state: &ServerState, user_id: i64) -> Result<(), ApiError> {
    let user = state.auth.get(user_id).await?;
    if !user.is_active {
        return Err(ApiError::forbidden(format!("用户 {user_id} 已停用")));
    }
    let role = a4n_core::models::user::role::parse_user_role(&user.role)?;
    if role == a4n_core::models::user::role::UserRole::Admin {
        Ok(())
    } else {
        Err(ApiError::forbidden("只有管理员可以修改全局 API 配置"))
    }
}

fn i64_arg_or_system_default(args: &Value, key: &str) -> Result<i64, ApiError> {
    let value = required_arg(args, key)?
        .as_i64()
        .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是整数")))?;
    if value == SYSTEM_DEFAULT_CONFIG_ID || value > 0 {
        Ok(value)
    } else {
        Err(ApiError::bad_request(format!(
            "{key} 必须是正整数或系统默认配置 ID"
        )))
    }
}

config_web_no_arg_command!(list_llm_config, config_records_actions::list_llm_config);
config_web_input_command!(
    create_llm_config,
    config_records_actions::CreateLlmInput,
    config_records_actions::create_llm_config,
);
config_web_update_command!(
    update_llm_config,
    config_records_actions::UpdateLlmInput,
    config_records_actions::update_llm_config,
);
config_web_id_unit_command!(
    set_default_llm_config,
    config_records_actions::set_default_llm_config,
);
config_web_id_unit_command!(delete_llm_config, config_records_actions::delete_llm_config);
config_web_no_arg_command!(
    get_llm_max_tokens_config,
    config_records_actions::get_llm_max_tokens_config,
);
config_web_input_command!(
    update_llm_max_tokens_config,
    a4n_core::config::app_config::LlmMaxTokensConfig,
    config_records_actions::update_llm_max_tokens_config,
);
pub(super) async fn test_llm_config(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let config_id = i64_arg_or_system_default(args, "configId")?;
    if a4n_runtime::runtime_config::current_llm_model_for_id(Some(config_id)).is_none() {
        require_global_config_admin(state, user_id).await?;
    }
    to_value(
        config_records_actions::test_llm_config(state, config_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(super) async fn preview_test_llm_config_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let config_id = i64_arg_or_system_default(args, "configId")?;
    if a4n_runtime::runtime_config::current_llm_model_for_id(Some(config_id)).is_none() {
        require_global_config_admin(state, user_id).await?;
    }
    to_value(
        config_records_actions::preview_test_llm_config_request(state, config_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}
config_web_model_discovery_command!(
    detect_llm_config_models,
    config_records_actions::detect_llm_config_models,
);

config_web_no_arg_command!(
    list_embedding_config,
    config_records_actions::list_embedding_config,
);
config_web_input_command!(
    create_embedding_config,
    config_records_actions::CreateEmbeddingInput,
    config_records_actions::create_embedding_config,
);
config_web_update_command!(
    update_embedding_config,
    config_records_actions::UpdateEmbeddingInput,
    config_records_actions::update_embedding_config,
);
config_web_id_unit_command!(
    set_default_embedding_config,
    config_records_actions::set_default_embedding_config,
);
config_web_id_unit_command!(
    delete_embedding_config,
    config_records_actions::delete_embedding_config,
);
config_web_id_value_command!(
    test_embedding_config,
    config_records_actions::test_embedding_config,
);
config_web_id_value_command!(
    preview_test_embedding_config_request,
    config_records_actions::preview_test_embedding_config_request,
);

pub(super) async fn list_embedding_backends(
    _state: &ServerState,
    _user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    Ok(json!(config_records_actions::list_embedding_backends()))
}

pub(super) async fn rag_enhanced_search(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: rag_search_actions::EnhancedSearchInput =
        command_input(args, "rag_enhanced_search")?;
    Ok(json!(
        rag_search_actions::rag_enhanced_search(state, user_id, project_id, input).await?
    ))
}

pub(super) async fn rag_prepare_enhanced_search(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: rag_search_actions::EnhancedSearchInput =
        command_input(args, "rag_prepare_enhanced_search")?;
    Ok(json!(
        rag_search_actions::rag_prepare_enhanced_search(state, user_id, project_id, input).await?
    ))
}

pub(super) async fn rag_enhanced_search_with_vectors(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: rag_search_actions::RagEnhancedSearchWithVectorsInput =
        command_input(args, "rag_enhanced_search_with_vectors")?;
    Ok(json!(
        rag_search_actions::rag_enhanced_search_with_vectors(state, user_id, project_id, input)
            .await?
    ))
}

pub(super) async fn rag_ingest_all(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let scope = a4n_runtime::rag_ingest::rag_ingest_task_scope(state, user_id, project_id)
        .await
        .map_err(ApiError::bad_request)?;
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskType::RagIngestion,
        scope.project_id,
        scope.total,
        "准备同步 RAG 数据",
    )
    .await;
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let report_progress =
        web_fire_and_forget_progress_reporter(state, task.clone(), task_id.clone());
    let reporter: a4n_runtime::rag_ingest::RagProgressReporter<'_> = task_id
        .as_ref()
        .map(|_| &report_progress as &(dyn Fn(u32, u32, &str) + Send + Sync));
    let mut progress = a4n_runtime::rag_ingest::RagIngestProgress::new(
        scope.expected_records,
        reporter,
        cancel_token.as_ref(),
    );
    let result =
        a4n_runtime::rag_ingest::rag_ingest_all(state, user_id, project_id, Some(&mut progress))
            .await
            .map_err(|err| {
                if is_rag_cancelled_message(&err) {
                    a4n_core::error::AppError::Cancelled
                } else {
                    a4n_core::error::AppError::Internal(anyhow::anyhow!(err))
                }
            });
    let value = finish_web_task(state, task_id, result, "RAG 同步完成").await?;
    Ok(value)
}

pub(super) async fn rag_prepare_ingest_all(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    Ok(json!(
        a4n_runtime::rag_ingest::rag_prepare_ingest_all(state, user_id, project_id)
            .await
            .map_err(ApiError::bad_request)?
    ))
}

pub(super) async fn rag_upload_ingest_embeddings(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: a4n_runtime::rag_ingest::ClientRagIngestUploadInput =
        command_input(args, "rag_upload_ingest_embeddings")?;
    Ok(json!(
        a4n_runtime::rag_ingest::rag_upload_ingest_embeddings(state, user_id, project_id, input)
            .await
            .map_err(ApiError::bad_request)?
    ))
}

pub(super) fn is_rag_cancelled_message(message: &str) -> bool {
    message.contains("已停止") || message.contains("已取消")
}

pub(super) async fn list_local_embedding_models(
    state: &ServerState,
    _user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    Ok(json!(embedding_model_actions::list_local_embedding_models(
        state
    )?))
}

pub(super) async fn get_embedding_model_download_info(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: embedding_model_actions::EmbeddingModelInput =
        command_input(args, "get_embedding_model_download_info")?;
    Ok(json!(
        embedding_model_actions::get_embedding_model_download_info(state, input).await?
    ))
}

pub(super) async fn download_embedding_model(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let mut input: embedding_model_actions::EmbeddingModelInput =
        command_input(args, "download_embedding_model")?;
    let task_id = sanitize_optional_task_id(input.task_id.clone())?;
    input.task_id = task_id.clone();
    let scope = embedding_model_actions::embedding_model_download_task_scope();
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskType::EmbeddingDownload,
        None,
        scope.total,
        scope.initial_message,
    )
    .await;
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let progress_tx = task_id
        .as_ref()
        .map(|task_id| spawn_embedding_download_progress_bridge(state, task_id, task.clone()));
    let result =
        embedding_model_actions::download_embedding_model(state, input, progress_tx, cancel_token)
            .await;
    let summary = finish_web_task(state, task_id, result, "模型已下载并完成初始化").await?;
    Ok(json!(summary))
}

pub(super) async fn cancel_embedding_model_download(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: embedding_model_actions::CancelEmbeddingModelDownloadInput =
        command_input(args, "cancel_embedding_model_download")?;
    let task_id = embedding_model_actions::required_task_id(input)?;
    if !state.task_manager.cancel(&task_id) {
        return Err(ApiError::bad_request(format!(
            "模型下载任务不存在或已结束：{task_id}"
        )));
    }
    emit_workflow_error(state, &task_id, "cancelled", "模型下载已停止", false).await;
    Ok(Value::Null)
}

pub(super) async fn delete_local_embedding_model(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: embedding_model_actions::EmbeddingModelInput =
        command_input(args, "delete_local_embedding_model")?;
    embedding_model_actions::delete_local_embedding_model(state, input)?;
    Ok(Value::Null)
}

config_web_no_arg_command!(
    list_image_configs,
    config_records_actions::list_image_configs
);
config_web_input_command!(
    add_image_config,
    config_records_actions::ImageConfigInput,
    config_records_actions::add_image_config,
);
config_web_update_command!(
    update_image_config,
    config_records_actions::UpdateImageConfigInput,
    config_records_actions::update_image_config,
);
config_web_id_unit_command!(
    remove_image_config,
    config_records_actions::remove_image_config
);
config_web_admin_id_value_command!(
    activate_image_config,
    config_records_actions::activate_image_config,
);
pub(super) async fn test_image_config(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let config_id = i64_arg_or_system_default(args, "configId")?;
    if a4n_runtime::runtime_config::current_image_model_for_id(Some(config_id)).is_none() {
        require_global_config_admin(state, user_id).await?;
    }
    to_value(
        config_records_actions::test_image_config(state, config_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(super) async fn preview_test_image_config_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let config_id = i64_arg_or_system_default(args, "configId")?;
    if a4n_runtime::runtime_config::current_image_model_for_id(Some(config_id)).is_none() {
        require_global_config_admin(state, user_id).await?;
    }
    to_value(
        config_records_actions::preview_test_image_config_request(state, config_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}
config_web_model_discovery_command!(
    detect_image_config_models,
    config_records_actions::detect_image_config_models,
);

config_web_no_arg_command!(
    list_video_configs,
    config_records_actions::list_video_configs
);
config_web_input_command!(
    add_video_config,
    config_records_actions::VideoConfigInput,
    config_records_actions::add_video_config,
);
config_web_update_command!(
    update_video_config,
    config_records_actions::UpdateVideoConfigInput,
    config_records_actions::update_video_config,
);
config_web_id_unit_command!(
    remove_video_config,
    config_records_actions::remove_video_config
);
config_web_admin_id_value_command!(
    activate_video_config,
    config_records_actions::activate_video_config,
);
pub(super) async fn test_video_config(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let config_id = i64_arg_or_system_default(args, "configId")?;
    if a4n_runtime::runtime_config::current_video_model_for_id(Some(config_id)).is_none() {
        require_global_config_admin(state, user_id).await?;
    }
    to_value(
        config_records_actions::test_video_config(state, config_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}
config_web_model_discovery_command!(
    detect_video_config_models,
    config_records_actions::detect_video_config_models,
);

macro_rules! prompt_web_sync_value_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            _user_id: i64,
            _args: &Value,
        ) -> Result<Value, ApiError> {
            to_value($action(state))
        }
    };
}

macro_rules! prompt_web_sync_result_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            _user_id: i64,
            _args: &Value,
        ) -> Result<Value, ApiError> {
            to_value($action(state).map_err(ApiError::bad_request)?)
        }
    };
}

macro_rules! prompt_web_async_result_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            _user_id: i64,
            _args: &Value,
        ) -> Result<Value, ApiError> {
            to_value($action(state).await.map_err(ApiError::bad_request)?)
        }
    };
}

macro_rules! prompt_web_user_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value(
                $action(state, user_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! prompt_web_input_async_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            _user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state, input).await.map_err(ApiError::bad_request)?)
        }
    };
}

macro_rules! prompt_web_input_sync_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            _user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state, input).map_err(ApiError::bad_request)?)
        }
    };
}

macro_rules! prompt_web_name_sync_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            _user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let name = string_arg(args, "name")?;
            to_value($action(state, &name).map_err(ApiError::bad_request)?)
        }
    };
}

macro_rules! prompt_web_name_unit_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            _user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let name = string_arg(args, "name")?;
            $action(state, &name).await.map_err(ApiError::bad_request)?;
            Ok(Value::Null)
        }
    };
}

macro_rules! prompt_web_user_input_unit_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            $action(state, user_id, input)
                .await
                .map_err(ApiError::bad_request)?;
            Ok(Value::Null)
        }
    };
}

pub(super) async fn prompt_list_project_overrides(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        prompt_records_actions::prompt_list_project_overrides(state, user_id, project_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

prompt_web_user_input_command!(
    prompt_render,
    prompt_records_actions::RenderInput,
    prompt_records_actions::prompt_render,
);
prompt_web_sync_value_command!(prompt_list, prompt_records_actions::prompt_list);
prompt_web_async_result_command!(
    prompt_load_defaults,
    prompt_records_actions::prompt_load_defaults,
);
prompt_web_input_async_command!(
    prompt_update,
    prompt_records_actions::UpdatePromptInput,
    prompt_records_actions::prompt_update,
);
prompt_web_name_unit_command!(prompt_reset, prompt_records_actions::prompt_reset);
prompt_web_name_sync_command!(
    prompt_version_history,
    prompt_records_actions::prompt_version_history,
);
prompt_web_input_sync_command!(
    prompt_version_diff,
    prompt_records_actions::VersionDiffInput,
    prompt_records_actions::prompt_version_diff,
);
prompt_web_input_sync_command!(
    prompt_validate_variables,
    prompt_records_actions::ValidateVarsInput,
    prompt_records_actions::prompt_validate_variables,
);
prompt_web_sync_value_command!(
    prompt_dependency_graph,
    prompt_records_actions::prompt_dependency_graph,
);
prompt_web_user_input_command!(
    prompt_render_ab,
    prompt_records_actions::AbRenderInput,
    prompt_records_actions::prompt_render_ab,
);
prompt_web_name_sync_command!(
    prompt_get_dependencies,
    prompt_records_actions::prompt_get_dependencies,
);
prompt_web_name_sync_command!(
    prompt_get_reverse_dependencies,
    prompt_records_actions::prompt_get_reverse_dependencies,
);
prompt_web_user_input_unit_command!(
    prompt_set_project_override,
    prompt_records_actions::SetOverrideInput,
    prompt_records_actions::prompt_set_project_override,
);
prompt_web_user_input_unit_command!(
    prompt_remove_project_override,
    prompt_records_actions::RemoveOverrideInput,
    prompt_records_actions::prompt_remove_project_override,
);
prompt_web_sync_result_command!(
    prompt_list_categories,
    prompt_records_actions::prompt_list_categories,
);

pub(super) async fn list_active_workflows(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    let active = task_records_actions::list_active_workflows_for_user(state, user_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(active)
}

pub(super) async fn list_workflow_runs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: task_records_actions::ListWorkflowRunsInput =
        command_input(args, "list_workflow_runs")?;
    let runs = task_records_actions::list_workflow_runs_for_user(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(runs)
}

pub(super) async fn get_workflow_run_detail(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: task_records_actions::GetWorkflowRunInput =
        command_input(args, "get_workflow_run_detail")?;
    let detail = task_records_actions::get_workflow_run_detail_for_user(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(detail)
}

pub(super) async fn resume_workflow_run(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: task_records_actions::ResumeWorkflowRunInput =
        command_input(args, "resume_workflow_run")?;
    let run = task_records_actions::resume_workflow_run_for_user(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(run)
}

pub(super) async fn get_workflow_progress_events(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: task_records_actions::WorkflowProgressEventsInput =
        command_input(args, "get_workflow_progress_events")?;
    let events = task_records_actions::get_workflow_progress_events(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(events)
}

pub(super) async fn cancel_workflow(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: task_records_actions::CancelWorkflowInput = command_input(args, "cancel_workflow")?;
    task_records_actions::cancel_workflow(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(super) async fn cancel_task(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let task_id = string_arg(args, "taskId")?;
    task_records_actions::cancel_task(state, user_id, &task_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(super) async fn get_queue_stats(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    let stats = task_records_actions::get_queue_stats(state, user_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(stats)
}

pub(super) async fn get_task_history(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = nullable_i64_arg(args, "projectId")?;
    let offset = non_negative_u64_arg(args, "offset")?;
    let limit = u64_arg(args, "limit")?;
    let history = task_records_actions::get_task_history(state, user_id, project_id, offset, limit)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(history)
}
