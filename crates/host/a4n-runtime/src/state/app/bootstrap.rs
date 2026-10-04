use super::*;

/// Runtime-owned resources prepared before a Host binds application behavior.
pub struct RuntimeBootstrap {
    config: AppConfig,
    runtime_profile: RuntimeProfile,
    runtime_role: RuntimeRole,
    db: a4n_db_session::DbHandle,
    capabilities: RuntimeCapabilities,
    service_manager: Arc<a4n_service_manager::ServiceManager>,
    event_bus: Arc<BroadcastEventBus>,
    task_manager: Arc<a4n_task_runtime::channel::TaskManager>,
    prompt_engine: Arc<a4n_prompt::tera_engine::TeraPromptEngine>,
    rag_store: Arc<dyn a4n_rag_core::vector_store::VectorStore>,
    generation_owner_lock: Option<a4n_db_session::DbHandle>,
    data_dir_lock: Option<Arc<File>>,
}

impl RuntimeBootstrap {
    pub async fn prepare(
        config: AppConfig,
        runtime_profile: RuntimeProfile,
        runtime_role: RuntimeRole,
    ) -> Result<Self, a4n_error::AppError> {
        runtime_profile.validate_build_capabilities()?;
        validate_runtime_config(&config)?;
        let data_dir_lock = match runtime_role {
            RuntimeRole::Host | RuntimeRole::DesktopHost => {
                Some(Arc::new(acquire_data_dir_lock(&config.data_dir)?))
            }
            RuntimeRole::AgentWorker => None,
        };

        let db = a4n_db_session::DbHandle::connect_default(&config.database_url).await?;
        let generation_owner_lock = match runtime_role {
            RuntimeRole::Host | RuntimeRole::DesktopHost => {
                Some(acquire_generation_owner_lock(&config.database_url).await?)
            }
            RuntimeRole::AgentWorker => None,
        };

        if runtime_role.is_host() {
            a4n_db_session::verify_required_schema(&db).await?;
            a4n_db_session::verify_canvas_project_split(&db).await?;
        }

        let service_manager = Arc::new(
            crate::sidecar_clients::runtime_service_manager(runtime_profile, &config)
                .map_err(a4n_error::AppError::ConfigError)?,
        );
        let capabilities =
            crate::sidecar_clients::runtime_capabilities(runtime_profile, service_manager.as_ref())
                .map_err(a4n_error::AppError::ConfigError)?;
        let event_bus = Arc::new(BroadcastEventBus::new(config.system.event_bus_capacity));
        let task_manager = Arc::new(a4n_task_runtime::channel::TaskManager::new());
        let (prompt_engine_result, rag_store_result) = futures::join!(
            async { create_prompt_engine(&config) },
            create_rag_store(runtime_profile, capabilities.rag.clone()),
        );

        Ok(Self {
            config,
            runtime_profile,
            runtime_role,
            db,
            capabilities,
            service_manager,
            event_bus,
            task_manager,
            prompt_engine: Arc::new(prompt_engine_result?),
            rag_store: rag_store_result?,
            generation_owner_lock,
            data_dir_lock,
        })
    }

    pub fn database(&self) -> a4n_db_session::DbHandle {
        self.db.clone()
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn prompt_client(&self) -> Option<Arc<dyn a4n_prompt_client::PromptClient>> {
        self.capabilities.prompt.clone()
    }

    pub fn service_manager(&self) -> Arc<a4n_service_manager::ServiceManager> {
        self.service_manager.clone()
    }

    pub fn event_bus(&self) -> Arc<BroadcastEventBus> {
        self.event_bus.clone()
    }

    pub fn task_manager(&self) -> Arc<a4n_task_runtime::channel::TaskManager> {
        self.task_manager.clone()
    }

    pub fn rag_store(&self) -> Arc<dyn a4n_rag_core::vector_store::VectorStore> {
        self.rag_store.clone()
    }

    pub fn prompt_engine(&self) -> Arc<a4n_prompt::tera_engine::TeraPromptEngine> {
        self.prompt_engine.clone()
    }

    pub fn document_export_renderer(
        &self,
    ) -> Result<Arc<dyn a4n_export_contract::ExportDocumentRenderer>, a4n_error::AppError> {
        crate::sidecar_clients::document_export_renderer(
            self.runtime_profile,
            self.capabilities.export.clone(),
        )
        .map_err(a4n_error::AppError::ConfigError)
    }

    pub fn create_provider(
        &self,
        provider_port: Arc<dyn a4n_provider_runtime_contract::ProviderRuntimePort>,
    ) -> Result<
        (
            crate::DynamicProviderFacade,
            Arc<dyn a4n_llm_provider_contract::provider::LlmProvider>,
            Arc<dyn a4n_embedding_provider_contract::EmbeddingProvider>,
        ),
        a4n_error::AppError,
    > {
        let provider_facade =
            crate::provider_runtime::DynamicProviderFacade::new(provider_port, self.runtime_role);
        let embedding = crate::sidecar_clients::create_embedding_provider(
            provider_facade.clone(),
            &self.config,
            self.runtime_profile,
            self.capabilities.embedding.clone(),
        )?;
        let llm = crate::sidecar_clients::create_llm_provider(
            provider_facade.clone(),
            self.runtime_profile,
            &self.capabilities,
        )?;
        Ok((provider_facade, llm, embedding))
    }

    pub async fn finish(
        self,
        application_ports: ApplicationPorts,
        provider_runtime: crate::DynamicProviderFacade,
        rag_search: Arc<dyn a4n_rag_contract::RagSearchPort>,
        rag_ingest: Arc<dyn a4n_rag_contract::RagIngestPort>,
    ) -> Result<AppState, a4n_error::AppError> {
        let RuntimeBootstrap {
            config,
            runtime_profile,
            runtime_role,
            capabilities,
            service_manager,
            event_bus,
            task_manager,
            generation_owner_lock,
            data_dir_lock,
            ..
        } = self;

        application_ports
            .prompt_mutation_application
            .hydrate_overrides()
            .await?;

        if runtime_role.is_host() {
            application_ports
                .package_mutation_application
                .initialize_states()
                .await?;
            crate::baseline_configs::ensure_baseline_model_configs(
                application_ports.config_read_application.as_ref(),
                application_ports.config_mutation_application.as_ref(),
                &config,
            )
            .await?;
        }

        let managed_sidecars = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let state = AppState {
            config,
            task_manager,
            service_health_application: Arc::new(
                crate::service_health::ServiceHealthApplication::new(service_manager.clone()),
            ),
            application_ports,
            capabilities,
            event_bus,
            provider_runtime,
            rag_search,
            rag_ingest,
            runtime_profile,
            service_manager,
            destructive_lifecycle: Arc::new(
                crate::actions::generation_tasks::destructive_lifecycle::DestructiveLifecycleCoordinator::default(),
            ),
            _generation_owner_lock: generation_owner_lock,
            _data_dir_lock: data_dir_lock,
            managed_sidecars,
            managed_sidecar_lifecycle: Arc::new(tokio::sync::Mutex::new(())),
            agent_worker_cancel: tokio_util::sync::CancellationToken::new(),
            agent_worker_notify: Arc::new(tokio::sync::Notify::new()),
            agent_worker_handle: Arc::new(tokio::sync::Mutex::new(None)),
            agent_runtime_shared: AgentRuntimeShared::new(),
            agent_host_dispatcher: Arc::new(tokio::sync::RwLock::new(None)),
        };

        if runtime_role.is_host() {
            let mut enabled_services = state.package_read_application().enabled_services().await?;
            include_runtime_sidecar_services(runtime_profile, &mut enabled_services);
            state
                .start_managed_sidecars_for_services(&enabled_services)
                .await
                .map_err(a4n_error::AppError::ConfigError)?;
            state
                .destructive_lifecycle()
                .continue_pending(&state)
                .await
                .map_err(|current| a4n_error::AppError::InvalidState {
                    current,
                    operation: "恢复未完成 generation lifecycle operation".to_string(),
                })?;
            crate::actions::video_records::recover_expired_uploaded_video_sources(state.temp_dir());
            crate::actions::video_records::recover_expired_assembly_caches(&state).await;
            crate::cleanup_outbox::recover_cleanup_outbox(&state).await;
            crate::cleanup_outbox::spawn_cleanup_worker(state.clone());
            crate::actions::package::spawn_package_lifecycle_worker(state.clone());
        }

        Ok(state)
    }
}

pub(super) fn include_runtime_sidecar_services(
    profile: RuntimeProfile,
    services: &mut Vec<String>,
) {
    if profile.uses_sidecars()
        && !services
            .iter()
            .any(|service| service == a4n_media_contract::MEDIA_SERVICE_NAME)
    {
        services.push(a4n_media_contract::MEDIA_SERVICE_NAME.to_string());
    }
}

pub(super) fn acquire_data_dir_lock(data_dir: &Path) -> Result<File, a4n_error::AppError> {
    let path = data_dir.join(".a4n-host.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|error| {
            a4n_error::AppError::ConfigError(format!(
                "无法打开 data directory ownership lock {}：{error}",
                path.display()
            ))
        })?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(a4n_error::AppError::ConfigError(format!(
            "data directory 已被另一个 A4N Host 占用：{}；同一目录只允许一个写入 Host",
            data_dir.display()
        ))),
        Err(TryLockError::Error(error)) => Err(a4n_error::AppError::ConfigError(format!(
            "无法锁定 data directory {}：{error}",
            data_dir.display()
        ))),
    }
}
