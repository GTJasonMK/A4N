use a4n_api_contract::blueprint::{
    BlueprintReadApplicationPort, BlueprintSummary, BlueprintVersionSummary,
    DiffBlueprintVersionsInput, ListBlueprintVersionsInput,
};
use a4n_blueprint_worker_contract::{
    BLUEPRINT_DIFF_OPERATION, BLUEPRINT_SUMMARY_OPERATION, BLUEPRINT_VERSIONS_OPERATION,
    BlueprintDiffSummary, BlueprintProjectInput,
};
use a4n_error::AppError;

use super::BlueprintWorkerClient;

#[async_trait::async_trait]
impl BlueprintReadApplicationPort for BlueprintWorkerClient {
    async fn summary(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Option<BlueprintSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            BLUEPRINT_SUMMARY_OPERATION,
            BlueprintProjectInput { project_id },
        )
        .await
    }

    async fn versions(
        &self,
        user_id: i64,
        input: ListBlueprintVersionsInput,
    ) -> Result<Vec<BlueprintVersionSummary>, AppError> {
        self.command(
            user_id,
            input.project_id,
            BLUEPRINT_VERSIONS_OPERATION,
            input,
        )
        .await
    }

    async fn diff(
        &self,
        user_id: i64,
        input: DiffBlueprintVersionsInput,
    ) -> Result<BlueprintDiffSummary, AppError> {
        self.command(user_id, input.project_id, BLUEPRINT_DIFF_OPERATION, input)
            .await
    }
}
