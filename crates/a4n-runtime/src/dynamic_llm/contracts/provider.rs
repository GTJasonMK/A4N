use a4n_capabilities::LlmClient;
use a4n_orchestrator::{OrchestratorContext, task_queue::PersistentTaskQueue};
use std::sync::Arc;

pub struct DynamicLlmProvider {
    pub(in crate::dynamic_llm) orchestrator: OrchestratorContext,
    pub(in crate::dynamic_llm) task_queue: Arc<PersistentTaskQueue>,
    pub(in crate::dynamic_llm) sidecar_client: Option<Arc<dyn LlmClient>>,
}
