use super::*;
use a4n_api_contract::outline::{
    ChapterOutlineListEntry, ChapterOutlineSummary, OutlineGenerationProgressSummary,
    PartOutlineSummary,
};
use a4n_outline_worker_contract::{
    ChapterOutlineAccessInput, ENSURE_CHAPTER_ACCESS_OPERATION, ENSURE_PART_ACCESS_OPERATION,
    GENERATION_PROGRESS_OPERATION, LIST_CHAPTER_INDEX_OPERATION, LIST_CHAPTERS_OPERATION,
    LIST_PARTS_OPERATION, PartOutlineAccessInput, ProjectOutlineInput,
};
use a4n_worker_contract::WorkerEnvelope;

impl WorkerState {
    pub(super) async fn list_chapters(
        &self,
        request: WorkerEnvelope<ProjectOutlineInput>,
    ) -> Result<Vec<ChapterOutlineSummary>, WorkerFailure> {
        validate_request(&request, LIST_CHAPTERS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let outlines = a4n_db::project::chapter_outlines_for_project(
            self.database.connection(),
            request.payload.project_id,
        )
        .await
        .map_err(outline_record_error)?;
        let ids = outlines
            .iter()
            .map(|outline| outline.id)
            .collect::<Vec<_>>();
        let source_kinds =
            a4n_db::project::chapter_outline_source_kinds_for_ids(self.database.connection(), &ids)
                .await
                .map_err(outline_record_error)?;
        Ok(outlines
            .iter()
            .map(|outline| chapter_outline_summary(outline, source_kinds.get(&outline.id)))
            .collect())
    }

    pub(super) async fn list_parts(
        &self,
        request: WorkerEnvelope<ProjectOutlineInput>,
    ) -> Result<Vec<PartOutlineSummary>, WorkerFailure> {
        validate_request(&request, LIST_PARTS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        Ok(a4n_db::project::part_outlines_for_project(
            self.database.connection(),
            request.payload.project_id,
        )
        .await
        .map_err(outline_record_error)?
        .iter()
        .map(part_outline_summary)
        .collect())
    }

    pub(super) async fn list_chapter_index(
        &self,
        request: WorkerEnvelope<ProjectOutlineInput>,
    ) -> Result<Vec<ChapterOutlineListEntry>, WorkerFailure> {
        validate_request(&request, LIST_CHAPTER_INDEX_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        Ok(a4n_db::project::chapter_outline_index_for_project(
            self.database.connection(),
            request.payload.project_id,
        )
        .await
        .map_err(outline_record_error)?
        .into_iter()
        .map(|outline| ChapterOutlineListEntry {
            id: outline.id,
            project_id: outline.project_id,
            chapter_number: outline.chapter_number,
            title: outline.title,
            summary: outline.summary,
            word_count_target: outline.word_count_target,
            source_kind: outline.source_kind,
            updated_at: Some(outline.updated_at.to_string()),
        })
        .collect())
    }

    pub(super) async fn generation_progress(
        &self,
        request: WorkerEnvelope<ProjectOutlineInput>,
    ) -> Result<OutlineGenerationProgressSummary, WorkerFailure> {
        validate_request(&request, GENERATION_PROGRESS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let rows = a4n_db::project::outline_generation_progress_rows(
            self.database.connection(),
            request.payload.project_id,
        )
        .await
        .map_err(outline_record_error)?;
        generation_progress_summary(&rows)
    }

    pub(super) async fn ensure_chapter_access(
        &self,
        request: WorkerEnvelope<ChapterOutlineAccessInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, ENSURE_CHAPTER_ACCESS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        WorkerFailure::require_positive_id(request.payload.outline_id, "outline_id")?;
        let user_id = request.principal.require_user_id()?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let outline = a4n_db::project::chapter_outline_by_id(
            self.database.connection(),
            request.payload.outline_id,
        )
        .await
        .map_err(outline_record_error)?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("章节大纲不存在: {}", request.payload.outline_id))
        })?;
        ensure_project_ownership(outline.project_id, request.payload.project_id, "章节大纲")
    }

    pub(super) async fn ensure_part_access(
        &self,
        request: WorkerEnvelope<PartOutlineAccessInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, ENSURE_PART_ACCESS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        WorkerFailure::require_positive_id(request.payload.part_id, "part_id")?;
        let user_id = request.principal.require_user_id()?;
        verify_project_permission(
            self,
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let part = a4n_db::project::part_outline_by_id(
            self.database.connection(),
            request.payload.part_id,
        )
        .await
        .map_err(outline_record_error)?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("分部大纲不存在: {}", request.payload.part_id))
        })?;
        ensure_project_ownership(part.project_id, request.payload.project_id, "分部大纲")
    }
}
