impl WorkerState {
    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

async fn health(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let status = match state.database.ping().await {
        Ok(()) => WorkerHealthStatus::Healthy,
        Err(_) => WorkerHealthStatus::Degraded,
    };
    Json(WorkerHealth {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        status,
        version: env!("CARGO_PKG_VERSION").into(),
    })
    .into_response()
}

async fn capabilities(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    Json(WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        capabilities: vec![
            WorkerCapability {
                name: CONFIG_READ_CAPABILITY.into(),
                version: CONFIG_READ_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: CONFIG_MUTATION_CAPABILITY.into(),
                version: CONFIG_READ_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: CONFIG_SETTINGS_CAPABILITY.into(),
                version: CONFIG_READ_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

macro_rules! command_handler {
    ($name:ident, $method:ident, $input:ty) => {
        async fn $name(
            State(state): State<WorkerState>,
            headers: HeaderMap,
            Json(request): Json<WorkerEnvelope<$input>>,
        ) -> Response {
            command_response(&state, &headers, request, |request| state.$method(request)).await
        }
    };
}

command_handler!(llm_exists, llm_exists, EmptyInput);
command_handler!(image_exists, image_exists, EmptyInput);
command_handler!(video_exists, video_exists, EmptyInput);
command_handler!(saved_api_key, saved_api_key, SavedApiKeyInput);
command_handler!(llm_connectivity, llm_connectivity, ConfigIdInput);
command_handler!(
    embedding_connectivity,
    embedding_connectivity,
    ConfigIdInput
);
command_handler!(image_connectivity, image_connectivity, ConfigIdInput);
command_handler!(llm_list, llm_list, EmptyInput);
command_handler!(image_list, image_list, EmptyInput);
command_handler!(video_list, video_list, EmptyInput);
command_handler!(embedding_list, embedding_list, EmptyInput);
command_handler!(baseline_embedding, baseline_embedding, EmptyInput);
command_handler!(baseline_llm, baseline_llm, CreateLlmCommand);
command_handler!(baseline_image, baseline_image, CreateImageCommand);
command_handler!(baseline_video, baseline_video, CreateVideoCommand);
command_handler!(create_llm, create_llm, CreateLlmCommand);
command_handler!(update_llm, update_llm, UpdateLlmCommand);
command_handler!(set_default_llm, set_default_llm, ConfigIdInput);
command_handler!(delete_llm, delete_llm, ConfigIdInput);
command_handler!(create_image, create_image, CreateImageCommand);
command_handler!(update_image, update_image, UpdateImageCommand);
command_handler!(set_default_image, set_default_image, ConfigIdInput);
command_handler!(delete_image, delete_image, ConfigIdInput);
command_handler!(create_video, create_video, CreateVideoCommand);
command_handler!(update_video, update_video, UpdateVideoCommand);
command_handler!(set_default_video, set_default_video, ConfigIdInput);
command_handler!(delete_video, delete_video, ConfigIdInput);
command_handler!(create_embedding, create_embedding, CreateEmbeddingCommand);
command_handler!(update_embedding, update_embedding, UpdateEmbeddingCommand);
command_handler!(set_default_embedding, set_default_embedding, ConfigIdInput);
command_handler!(delete_embedding, delete_embedding, ConfigIdInput);
command_handler!(settings_export, settings_export, SettingsExportInput);
command_handler!(
    settings_prepare_import,
    settings_prepare_import,
    a4n_api_contract::config::ImportAllConfigsInput
);
command_handler!(settings_import, settings_import, SettingsImportCommand);

async fn command_response<T, O, F, Fut>(
    state: &WorkerState,
    headers: &HeaderMap,
    request: WorkerEnvelope<T>,
    command: F,
) -> Response
where
    T: serde::Serialize + Clone,
    O: serde::Serialize,
    F: FnOnce(WorkerEnvelope<T>) -> Fut,
    Fut: std::future::Future<Output = Result<O, WorkerFailure>>,
{
    a4n_worker_runtime::worker_command_response(
        state.authorized(headers),
        request,
        command,
        unauthorized,
    )
    .await
}
