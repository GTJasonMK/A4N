mod call;
mod checkpoint;
mod contracts;
mod preparation;
mod schema;

use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use crate::OrchestratorContext;

use self::call::execute_import_reverse_call;
use self::preparation::prepare_import_reverse_call;

impl OrchestratorContext {
    pub(crate) async fn reverse_engineer_request(
        &self,
        prompt_name: &str,
        request: ChatRequest,
        workflow_run_id: Option<i64>,
    ) -> Result<Value, AppError> {
        let prepared =
            prepare_import_reverse_call(self, prompt_name, request, workflow_run_id).await?;
        execute_import_reverse_call(self, prepared).await
    }
}
