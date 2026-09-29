use a4n_api_contract::galgame::{
    DeleteGalgameLineInput, GalgameChoice, GalgameLine, GalgameRecordMutationApplicationPort,
    GalgameScene, InsertGalgameLineInput, ReorderGalgameLinesInput, UpdateGalgameChoicesInput,
    UpdateGalgameLineInput, UpdateGalgameSceneInput,
};
use a4n_error::AppError;

use crate::OrchestratorContext;

#[async_trait::async_trait]
impl GalgameRecordMutationApplicationPort for OrchestratorContext {
    async fn update_line(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateGalgameLineInput,
    ) -> Result<GalgameLine, AppError> {
        self.update_galgame_line_for_user_validated(user_id, project_id, input)
            .await
    }

    async fn insert_line(
        &self,
        user_id: i64,
        project_id: i64,
        input: InsertGalgameLineInput,
    ) -> Result<GalgameLine, AppError> {
        self.insert_galgame_line_for_user_validated(user_id, project_id, input)
            .await
    }

    async fn update_scene(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateGalgameSceneInput,
    ) -> Result<GalgameScene, AppError> {
        self.update_galgame_scene_for_user_validated(user_id, project_id, input)
            .await
    }

    async fn delete_line(
        &self,
        user_id: i64,
        project_id: i64,
        input: DeleteGalgameLineInput,
    ) -> Result<(), AppError> {
        self.delete_galgame_line_for_user_validated(user_id, project_id, input.line_id)
            .await
    }

    async fn reorder_lines(
        &self,
        user_id: i64,
        project_id: i64,
        input: ReorderGalgameLinesInput,
    ) -> Result<Vec<GalgameLine>, AppError> {
        self.reorder_galgame_lines_for_user_validated(user_id, project_id, input)
            .await
    }

    async fn update_choices(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateGalgameChoicesInput,
    ) -> Result<Vec<GalgameChoice>, AppError> {
        self.update_galgame_choices_for_user_validated(user_id, project_id, input)
            .await
    }
}
