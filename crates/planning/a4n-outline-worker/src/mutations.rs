use super::*;
use a4n_api_contract::outline::{
    OutlineMutationResult, SaveChapterOutlineResult, SavePartOutlineResult,
    UpdateChapterOutlineResult,
};
use a4n_outline_worker_contract::{
    DELETE_CHAPTERS_OPERATION, DELETE_PARTS_OPERATION, DeleteLatestOutlineRecordsInput,
    SAVE_CHAPTER_OPERATION, SAVE_PART_OPERATION, SaveChapterOutlineInput, SavePartOutlineInput,
    UPDATE_CHAPTER_OPERATION, UpdateChapterOutlineInput,
};
use a4n_worker_contract::WorkerEnvelope;

impl WorkerState {
    pub(super) async fn delete_chapters(
        &self,
        request: WorkerEnvelope<DeleteLatestOutlineRecordsInput>,
    ) -> Result<OutlineMutationResult, WorkerFailure> {
        validate_request(&request, DELETE_CHAPTERS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        a4n_db::project::delete_latest_chapter_outline_records(
            self.database.connection(),
            delete_input(request.payload),
        )
        .await
        .map_err(outline_record_error)?;
        Ok(OutlineMutationResult {
            warnings: Vec::new(),
        })
    }

    pub(super) async fn delete_parts(
        &self,
        request: WorkerEnvelope<DeleteLatestOutlineRecordsInput>,
    ) -> Result<OutlineMutationResult, WorkerFailure> {
        validate_request(&request, DELETE_PARTS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        a4n_db::project::delete_latest_part_outline_records(
            self.database.connection(),
            delete_input(request.payload),
        )
        .await
        .map_err(outline_record_error)?;
        Ok(OutlineMutationResult {
            warnings: Vec::new(),
        })
    }

    pub(super) async fn save_chapter(
        &self,
        request: WorkerEnvelope<SaveChapterOutlineInput>,
    ) -> Result<SaveChapterOutlineResult, WorkerFailure> {
        validate_request(&request, SAVE_CHAPTER_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let outline = a4n_db::project::save_manual_chapter_outline(
            self.database.connection(),
            a4n_db::project::ManualChapterOutlineSaveInput {
                project_id: request.payload.project_id,
                outline_id: request.payload.outline_id,
                expected_updated_at: request.payload.expected_updated_at,
                chapter_number: request.payload.chapter_number,
                title: request.payload.title,
                summary: request.payload.summary,
            },
        )
        .await
        .map_err(outline_record_error)?;
        Ok(SaveChapterOutlineResult {
            outline: chapter_outline_summary(&outline, None),
            warnings: Vec::new(),
        })
    }

    pub(super) async fn update_chapter(
        &self,
        request: WorkerEnvelope<UpdateChapterOutlineInput>,
    ) -> Result<UpdateChapterOutlineResult, WorkerFailure> {
        validate_request(&request, UPDATE_CHAPTER_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let input = normalize_update_input(request.payload)?;
        let outline =
            a4n_db::project::update_chapter_outline_record(self.database.connection(), input)
                .await
                .map_err(update_error)?;
        Ok(UpdateChapterOutlineResult {
            outline: chapter_outline_summary(&outline, None),
            warnings: Vec::new(),
        })
    }

    pub(super) async fn save_part(
        &self,
        request: WorkerEnvelope<SavePartOutlineInput>,
    ) -> Result<SavePartOutlineResult, WorkerFailure> {
        validate_request(&request, SAVE_PART_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let part = a4n_db::project::save_manual_part_outline(
            self.database.connection(),
            a4n_db::project::ManualPartOutlineSaveInput {
                project_id: request.payload.project_id,
                part_id: request.payload.part_id,
                expected_updated_at: request.payload.expected_updated_at,
                part_number: request.payload.part_number,
                start_chapter: request.payload.start_chapter,
                end_chapter: request.payload.end_chapter,
                title: request.payload.title,
                summary: request.payload.summary,
                theme: request.payload.theme,
            },
        )
        .await
        .map_err(outline_record_error)?;
        Ok(SavePartOutlineResult {
            part: part_outline_summary(&part),
            warnings: Vec::new(),
        })
    }
}
