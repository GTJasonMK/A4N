use super::{
    WorkerState,
    errors::{
        WorkerFailure, app_error, chapter_error, generation_lifecycle_error, import_error,
        project_error,
    },
    permissions::verify_project_editor,
    transactions::finish_transaction,
    validation::{positive, user_id, validate_request},
};
use a4n_chapter_content_core::begin_chapter_content_sync_in_transaction;
use a4n_chapter_content_worker_contract::{
    CHAPTER_CONTENT_IMPORT_OPERATION, CHAPTER_CONTENT_UPDATE_OPERATION,
    ChapterContentMutationResult, ImportChapterInput, UpdateChapterContentInput,
};
use a4n_domain::chapter::{EditChapterContentInput, ImportChapterContentInput};
use a4n_worker_contract::WorkerEnvelope;

impl WorkerState {
    pub(super) async fn update(
        &self,
        request: WorkerEnvelope<UpdateChapterContentInput>,
    ) -> Result<ChapterContentMutationResult, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_UPDATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let chapter_id = positive("chapter_id", request.payload.chapter_id)?;
        let existing = a4n_db::chapter_content::chapter_by_id_for_analysis(
            self.database.connection(),
            chapter_id,
        )
        .await
        .map_err(chapter_error)?;
        self.verify_text_project_editor(existing.project_id, user_id, "更新章节正文")
            .await?;

        let input = request.payload;
        let tx = self
            .database
            .begin()
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        let result = async {
            a4n_db::generation::require_project_lifecycle_active(&tx, user_id, existing.project_id)
                .await
                .map_err(generation_lifecycle_error)?;
            let chapter = a4n_db::chapter_content::save_chapter_content_edit_record(
                &tx,
                EditChapterContentInput {
                    chapter_id: input.chapter_id,
                    expected_revision: input.expected_revision,
                    content: input.content,
                    title: input.title,
                },
            )
            .await
            .map_err(chapter_error)?;
            let chapter_record =
                a4n_db::chapter_content::chapter_by_id_for_analysis(&tx, chapter.id)
                    .await
                    .map_err(chapter_error)?;
            let run = begin_chapter_content_sync_in_transaction(&tx, &chapter_record)
                .await
                .map_err(app_error)?;
            Ok(ChapterContentMutationResult {
                chapter_id: chapter.id,
                workflow_run_id: run.id,
            })
        }
        .await;
        finish_transaction(tx, result, "保存章节正文并登记资料同步").await
    }

    pub(super) async fn import_chapter(
        &self,
        request: WorkerEnvelope<ImportChapterInput>,
    ) -> Result<ChapterContentMutationResult, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_IMPORT_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let project_id = positive("project_id", input.project_id)?;
        if input.chapter_number <= 0 {
            return Err(WorkerFailure::invalid_request("章节序号必须大于 0"));
        }
        self.verify_text_project_editor(project_id, user_id, "导入章节正文")
            .await?;

        let tx = self
            .database
            .begin()
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        let result = async {
            a4n_db::generation::require_project_lifecycle_active(&tx, user_id, project_id)
                .await
                .map_err(generation_lifecycle_error)?;
            let chapter = a4n_db::chapter_content::import_chapter_content_record(
                &tx,
                ImportChapterContentInput {
                    project_id: input.project_id,
                    chapter_number: input.chapter_number,
                    content: input.content,
                    title: input.title,
                },
                "导入覆盖正文",
            )
            .await
            .map_err(import_error)?;
            let chapter_record =
                a4n_db::chapter_content::chapter_by_id_for_analysis(&tx, chapter.id)
                    .await
                    .map_err(chapter_error)?;
            let run = begin_chapter_content_sync_in_transaction(&tx, &chapter_record)
                .await
                .map_err(app_error)?;
            Ok(ChapterContentMutationResult {
                chapter_id: chapter.id,
                workflow_run_id: run.id,
            })
        }
        .await;
        finish_transaction(tx, result, "导入章节正文并登记资料同步").await
    }

    async fn verify_text_project_editor(
        &self,
        project_id: i64,
        user_id: i64,
        operation: &str,
    ) -> Result<(), WorkerFailure> {
        verify_project_editor(&self.database, project_id, user_id).await?;
        let project = a4n_db::project::project_by_id_record(self.database.connection(), project_id)
            .await
            .map_err(project_error)?
            .ok_or_else(|| WorkerFailure::not_found(format!("项目不存在: {project_id}")))?;
        if project.project_kind != a4n_db::project::PROJECT_KIND_TEXT {
            return Err(WorkerFailure::invalid_request(format!(
                "{operation}只能作用于文本项目"
            )));
        }
        Ok(())
    }
}
