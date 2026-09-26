use super::*;

impl AppState {
    pub(crate) fn task_manager(&self) -> &a4n_task_runtime::channel::TaskManager {
        self.task_manager.as_ref()
    }

    pub(crate) fn generation_task_read_application(
        &self,
    ) -> &dyn a4n_generation_host_contract::GenerationTaskReadApplicationPort {
        self.generation_task_read_application.as_ref()
    }

    pub(crate) fn generation_task_mutation_application(
        &self,
    ) -> &dyn a4n_generation_host_contract::GenerationTaskMutationApplicationPort {
        self.generation_task_mutation_application.as_ref()
    }

    pub(crate) fn generation_lifecycle_application(
        &self,
    ) -> &dyn a4n_generation_host_contract::GenerationLifecycleApplicationPort {
        self.generation_lifecycle_application.as_ref()
    }

    pub(crate) fn generation_cleanup_application(
        &self,
    ) -> &dyn a4n_generation_host_contract::GenerationCleanupApplicationPort {
        self.generation_cleanup_application.as_ref()
    }

    pub(crate) fn source_sync_read_application(
        &self,
    ) -> &dyn a4n_api_contract::source_sync::SourceSyncReadApplicationPort {
        self.source_sync_read_application.as_ref()
    }

    pub(crate) fn source_sync_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::source_sync::SourceSyncMutationApplicationPort {
        self.source_sync_mutation_application.as_ref()
    }

    pub(crate) fn shooting_script_unit_progress_application(
        &self,
    ) -> &dyn a4n_api_contract::shooting_script::ShootingScriptUnitProgressReadApplicationPort {
        self.shooting_script_unit_progress_application.as_ref()
    }

    pub(crate) fn shooting_script_current_application(
        &self,
    ) -> &dyn a4n_api_contract::shooting_script::ShootingScriptCurrentReadApplicationPort {
        self.shooting_script_current_application.as_ref()
    }

    pub(crate) fn shooting_script_record_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::shooting_script::ShootingScriptRecordMutationApplicationPort {
        self.shooting_script_record_mutation_application.as_ref()
    }

    pub(crate) fn shooting_script_generation_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ShootingScriptGenerationApplicationPort {
        self.shooting_script_generation_application.as_ref()
    }

    pub(crate) fn rag_search(&self) -> &dyn a4n_rag_contract::RagSearchPort {
        self.rag_search.as_ref()
    }

    pub(crate) fn rag_ingest(&self) -> &dyn a4n_rag_contract::RagIngestPort {
        self.rag_ingest.as_ref()
    }

    pub(crate) fn provider_runtime(&self) -> &crate::provider_runtime::DynamicProviderFacade {
        &self.provider_runtime
    }

    pub(crate) fn spawn_generation_execution<F, Fut>(
        &self,
        handle: GenerationTaskHandle,
        worker_kind: &'static str,
        worker: F,
    ) where
        F: FnOnce(GenerationTaskHandle) -> Fut,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let task_id = handle.task_id.clone();
        let attempt = handle.attempt;
        self.spawn_generation_execution_scoped(
            task_id,
            attempt,
            worker_kind,
            a4n_queue::with_llm_slot(worker(handle)),
        );
    }

    pub(crate) fn spawn_image_generation_execution<F, Fut>(
        &self,
        handle: GenerationTaskHandle,
        worker_kind: &'static str,
        worker: F,
    ) where
        F: FnOnce(GenerationTaskHandle) -> Fut,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let task_id = handle.task_id.clone();
        let attempt = handle.attempt;
        self.spawn_generation_execution_scoped(
            task_id,
            attempt,
            worker_kind,
            a4n_queue::with_image_slot(worker(handle)),
        );
    }

    pub(crate) fn spawn_video_generation_execution<F, Fut>(
        &self,
        handle: GenerationTaskHandle,
        worker_kind: &'static str,
        worker: F,
    ) where
        F: FnOnce(GenerationTaskHandle) -> Fut,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let task_id = handle.task_id.clone();
        let attempt = handle.attempt;
        self.spawn_generation_execution_scoped(
            task_id,
            attempt,
            worker_kind,
            a4n_queue::with_video_slot(worker(handle)),
        );
    }

    fn spawn_generation_execution_scoped(
        &self,
        task_id: String,
        attempt: u32,
        worker_kind: &'static str,
        future: impl Future<Output = Result<(), String>> + Send + 'static,
    ) {
        drop(tokio::spawn(
            crate::runtime_config::inherit_runtime_context(async move {
                if let Err(error) = future.await {
                    tracing::error!(
                        task_id = %task_id,
                        attempt,
                        worker = worker_kind,
                        error = %error,
                        "generation worker 失败"
                    );
                }
            }),
        ));
    }

    pub(crate) fn destructive_lifecycle(
        &self,
    ) -> &Arc<
        crate::actions::generation_tasks::destructive_lifecycle::DestructiveLifecycleCoordinator,
    > {
        &self.destructive_lifecycle
    }

    pub(crate) fn dynamic_llm_provider_for_profile(
        &self,
    ) -> Result<crate::dynamic_llm::DynamicLlmProvider, a4n_error::AppError> {
        let facade = self.provider_runtime.clone();
        if self.runtime_profile.uses_sidecars() {
            let client = self.capabilities.llm.clone().ok_or_else(|| {
                a4n_error::AppError::ConfigError(
                    "LLM sidecar capability 未配置，不能回退到 embedded provider".to_string(),
                )
            })?;
            return Ok(
                crate::dynamic_llm::DynamicLlmProvider::new_with_sidecar_client(facade, client),
            );
        }
        Ok(crate::dynamic_llm::DynamicLlmProvider::new(facade))
    }
}
