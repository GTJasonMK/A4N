use a4n_api_contract::shooting_script::{
    ChapterShootingScriptSourceInput, ChapterShootingScriptView, DeleteChapterShootingScriptInput,
    EditChapterShootingScriptInput, EditChapterShootingScriptProgressInput,
    EditedChapterShootingScriptProgressView, EditedChapterShootingScriptView,
    PublishChapterShootingScriptInput, ShootingScriptCurrentReadApplicationPort,
    ShootingScriptRecordMutationApplicationPort, ShootingScriptUnitProgressReadApplicationPort,
};
use a4n_error::AppError;
use a4n_novel_planning_worker_contract::{
    CurrentShootingScriptPayload, CurrentShootingScriptResultPayload,
    DELETE_SHOOTING_SCRIPT_OPERATION, DeleteShootingScriptPayload, EDIT_SHOOTING_SCRIPT_OPERATION,
    EDIT_SHOOTING_SCRIPT_PROGRESS_OPERATION, EditShootingScriptPayload,
    EditShootingScriptProgressPayload, EditShootingScriptProgressResultPayload,
    GET_CURRENT_SHOOTING_SCRIPT_OPERATION, GET_SHOOTING_SCRIPT_UNIT_PROGRESS_OPERATION,
    PUBLISH_SHOOTING_SCRIPT_OPERATION, PublishShootingScriptPayload,
    ShootingScriptUnitProgressPayload, ShootingScriptUnitProgressResultPayload,
};

use super::NovelPlanningWorkerClient;

#[async_trait::async_trait]
impl ShootingScriptCurrentReadApplicationPort for NovelPlanningWorkerClient {
    async fn current(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptView>, AppError> {
        self.command::<CurrentShootingScriptPayload, CurrentShootingScriptResultPayload>(
            user_id,
            input.project_id,
            None,
            GET_CURRENT_SHOOTING_SCRIPT_OPERATION,
            input,
        )
        .await
        .map(|payload| payload.value)
    }
}

#[async_trait::async_trait]
impl ShootingScriptUnitProgressReadApplicationPort for NovelPlanningWorkerClient {
    async fn unit_progress(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<
        Option<a4n_api_contract::shooting_script::ChapterShootingScriptUnitProgressView>,
        AppError,
    > {
        self.command::<ShootingScriptUnitProgressPayload, ShootingScriptUnitProgressResultPayload>(
            user_id,
            input.project_id,
            None,
            GET_SHOOTING_SCRIPT_UNIT_PROGRESS_OPERATION,
            input,
        )
        .await
        .map(|payload| payload.value)
    }
}

#[async_trait::async_trait]
impl ShootingScriptRecordMutationApplicationPort for NovelPlanningWorkerClient {
    async fn edit_progress(
        &self,
        user_id: i64,
        input: EditChapterShootingScriptProgressInput,
    ) -> Result<EditedChapterShootingScriptProgressView, AppError> {
        self.command::<EditShootingScriptProgressPayload, EditShootingScriptProgressResultPayload>(
            user_id,
            input.project_id,
            None,
            EDIT_SHOOTING_SCRIPT_PROGRESS_OPERATION,
            input,
        )
        .await
    }

    async fn publish(
        &self,
        user_id: i64,
        input: PublishChapterShootingScriptInput,
    ) -> Result<ChapterShootingScriptView, AppError> {
        self.command::<PublishShootingScriptPayload, ChapterShootingScriptView>(
            user_id,
            input.project_id,
            input.expected_revision,
            PUBLISH_SHOOTING_SCRIPT_OPERATION,
            input,
        )
        .await
    }

    async fn edit(
        &self,
        user_id: i64,
        input: EditChapterShootingScriptInput,
    ) -> Result<EditedChapterShootingScriptView, AppError> {
        self.command::<EditShootingScriptPayload, EditedChapterShootingScriptView>(
            user_id,
            input.project_id,
            Some(input.expected_revision),
            EDIT_SHOOTING_SCRIPT_OPERATION,
            input,
        )
        .await
    }

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteChapterShootingScriptInput,
    ) -> Result<(), AppError> {
        self.command::<DeleteShootingScriptPayload, ()>(
            user_id,
            input.project_id,
            Some(input.expected_revision),
            DELETE_SHOOTING_SCRIPT_OPERATION,
            input,
        )
        .await
    }
}
