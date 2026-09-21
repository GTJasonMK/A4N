use std::sync::Arc;

use a4n_db::SharedDatabaseConnection;
use a4n_llm_provider_contract::provider::LlmProvider;

use super::OrchestratorContext;
use crate::test_support::TestOrchestratorBuilder;

pub(crate) async fn test_video_orchestrator(db: SharedDatabaseConnection) -> OrchestratorContext {
    TestOrchestratorBuilder::new(db).build().await
}

pub(crate) async fn test_video_orchestrator_with_llm(
    db: SharedDatabaseConnection,
    llm: Arc<dyn LlmProvider>,
) -> OrchestratorContext {
    TestOrchestratorBuilder::new(db).with_llm(llm).build().await
}
