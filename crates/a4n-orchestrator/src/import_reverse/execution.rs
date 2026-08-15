mod call;
mod checkpoint;
mod contracts;
mod preparation;
mod schema;

pub(in crate::import_reverse) use checkpoint::restored_import_reverse_checkpoint;

use a4n_api_contract::import_reverse::ReverseEngineeringCandidate;
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;

use crate::OrchestratorContext;

use self::call::execute_import_reverse_call;
use self::preparation::prepare_import_reverse_call;

impl OrchestratorContext {
    pub(crate) async fn reverse_engineer_request(
        &self,
        prompt_name: &str,
        request: ChatRequest,
        workflow_run_id: Option<i64>,
    ) -> Result<ReverseEngineeringCandidate, AppError> {
        let prepared =
            prepare_import_reverse_call(self, prompt_name, request, workflow_run_id).await?;
        execute_import_reverse_call(self, prepared).await
    }
}
