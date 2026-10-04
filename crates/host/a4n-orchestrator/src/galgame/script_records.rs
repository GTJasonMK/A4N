use crate::galgame::{
    OrchestratorContext, ReorderGalgameLinesInput, galgame_line_record_error,
    galgame_line_reorder_record_error, validate_positive_id,
};
use a4n_domain::galgame::GalgameLineRecord;
use a4n_error::AppError;

impl OrchestratorContext {
    pub async fn reorder_galgame_lines(
        &self,
        project_id: i64,
        input: ReorderGalgameLinesInput,
    ) -> Result<Vec<GalgameLineRecord>, AppError> {
        let context = self.load_galgame_context(project_id).await?;
        let scene = self
            .load_scene_in_project(context.project.id, input.scene_id)
            .await?;
        for line_id in &input.line_ids {
            validate_positive_id(*line_id, "line_ids")?;
        }
        a4n_db::galgame::reorder_galgame_line_records_transaction(
            self.database.executor(),
            scene.id,
            input.line_ids,
        )
        .await
        .map_err(galgame_line_reorder_record_error)
    }

    pub async fn delete_galgame_line(&self, project_id: i64, line_id: i64) -> Result<(), AppError> {
        let context = self.load_galgame_context(project_id).await?;
        let line = self
            .load_line_in_project(context.project.id, line_id)
            .await?;
        a4n_db::galgame::delete_galgame_line_with_generation_media_graph_record(
            self.database.executor(),
            context.project.id,
            line.id,
        )
        .await
        .map_err(galgame_line_record_error)
    }
}
