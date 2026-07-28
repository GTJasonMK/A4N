//! 导入反推 runtime 窄 facade。

use a4n_core::error::AppError;
use a4n_orchestrator::{
    OrchestratorContext,
    import_reverse::{ReverseEngineerApplicationInput, ReverseEngineerPreviewInput},
};
use serde_json::Value;

pub(crate) struct ImportReverseRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ImportReverseRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn prepare_reverse_engineer_request_validated(
        &self,
        input: ReverseEngineerPreviewInput<'_>,
    ) -> Result<a4n_llm::provider::ChatRequest, AppError> {
        self.orchestrator
            .prepare_reverse_engineer_request_validated(input)
            .await
    }

    pub(crate) async fn run_reverse_engineering_validated(
        &self,
        input: ReverseEngineerApplicationInput<'_>,
    ) -> Result<Value, AppError> {
        self.orchestrator
            .run_reverse_engineering_validated(input)
            .await
    }
}
