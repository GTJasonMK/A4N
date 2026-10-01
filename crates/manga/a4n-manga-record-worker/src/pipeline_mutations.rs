use a4n_domain::collaboration::ProjectAccessRole;
use a4n_manga_record_worker_contract::{MANGA_PIPELINE_CANCEL_OPERATION, MangaChapterInput};
use a4n_worker_contract::WorkerEnvelope;

use super::{
    WorkerState,
    access::{user_id, verify_project_permission},
    protocol::{
        WorkerFailure, manga_pipeline_error, positive, project_family_error, validate_request,
        workflow_error, workflow_write_error,
    },
};

impl WorkerState {
    pub(super) async fn pipeline_cancel(
        &self,
        request: WorkerEnvelope<MangaChapterInput>,
    ) -> Result<i64, WorkerFailure> {
        validate_request(&request, MANGA_PIPELINE_CANCEL_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        positive(input.project_id, "project_id")?;
        positive(input.chapter_id, "chapter_id")?;
        verify_project_permission(
            &self.database,
            input.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let source = a4n_db::project::chapter_manga_source_by_id(
            self.database.connection(),
            input.chapter_id,
            input.project_id,
            false,
        )
        .await
        .map_err(project_family_error)?;
        if source.status == "deleted" {
            return Err(WorkerFailure::invalid_request(format!(
                "第 {} 章已删除，无法取消漫画流水线",
                source.chapter_number
            )));
        }
        if source.status == "generating" {
            return Err(WorkerFailure::invalid_request(format!(
                "第 {} 章正在生成中，请等待完成后再取消漫画流水线",
                source.chapter_number
            )));
        }
        let run = a4n_db::workflow::list_workflow_runs(
            self.database.connection(),
            a4n_db::workflow::ListWorkflowRunsInput {
                project_id: Some(input.project_id),
                chapter_id: Some(input.chapter_id),
                chapter_ids: None,
                workflow_kind: Some("manga_pipeline".to_string()),
                status: Some("running".to_string()),
                resumable_only: None,
                limit: Some(1),
            },
        )
        .await
        .map_err(workflow_error)?
        .into_iter()
        .next()
        .ok_or_else(|| WorkerFailure::conflict("no_running_manga_pipeline: 取消漫画流水线"))?;
        a4n_db::workflow::mark_workflow_status(
            self.database.connection(),
            run.id,
            "cancelled",
            true,
            Some("用户请求取消漫画流水线"),
        )
        .await
        .map_err(workflow_write_error)?;
        a4n_db::manga::cancel_manga_page_prompts(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(manga_pipeline_error)
    }
}
