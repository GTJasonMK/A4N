use a4n_error::AppError;

use crate::galgame::{OrchestratorContext, validate_positive_id};

use super::GalgameAssetPlan;
use super::collection::collect_galgame_asset_plan_drafts;
use super::persistence::persist_galgame_asset_plan;

impl OrchestratorContext {
    pub async fn plan_galgame_assets(&self, project_id: i64) -> Result<GalgameAssetPlan, AppError> {
        validate_positive_id(project_id, "project_id")?;
        let context = self.load_galgame_context(project_id).await?;
        let read_model =
            a4n_db::galgame::galgame_asset_planning_read_model(&self.database, context.project.id)
                .await
                .map_err(AppError::database)?
                .ok_or_else(|| AppError::NotFound(format!("Galgame 剧本不存在: {project_id}")))?;
        if read_model.scenes.is_empty() {
            return Err(AppError::InvalidState {
                current: "galgame_script_has_no_scenes".to_string(),
                operation: "plan_galgame_assets".to_string(),
            });
        }
        let item_drafts = collect_galgame_asset_plan_drafts(&context, read_model)?;
        persist_galgame_asset_plan(self, context.project.id, item_drafts).await
    }
}
