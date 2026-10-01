use a4n_api_contract::galgame::GalgameReadApplicationPort;
use a4n_error::AppError;

use crate::OrchestratorContext;

use super::{GalgameAssetPlan, GalgameScriptDetail};

#[async_trait::async_trait]
impl GalgameReadApplicationPort for OrchestratorContext {
    async fn script(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Option<GalgameScriptDetail>, AppError> {
        self.get_galgame_script_for_user_validated(user_id, project_id)
            .await
    }

    async fn asset_plan(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Option<GalgameAssetPlan>, AppError> {
        self.verify_galgame_project_access(
            user_id,
            project_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        self.read_galgame_asset_plan(project_id).await
    }
}
