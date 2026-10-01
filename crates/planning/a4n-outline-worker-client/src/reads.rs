use super::*;

#[async_trait::async_trait]
impl OutlineReadApplicationPort for OutlineWorkerClient {
    async fn list_chapters(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterOutlineSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            LIST_CHAPTERS_OPERATION,
            ProjectOutlineInput { project_id },
        )
        .await
    }

    async fn list_parts(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<PartOutlineSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            LIST_PARTS_OPERATION,
            ProjectOutlineInput { project_id },
        )
        .await
    }

    async fn generation_progress(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<OutlineGenerationProgressSummary, AppError> {
        self.command(
            user_id,
            project_id,
            GENERATION_PROGRESS_OPERATION,
            ProjectOutlineInput { project_id },
        )
        .await
    }

    async fn ensure_chapter_outline_access(
        &self,
        user_id: i64,
        project_id: i64,
        outline_id: i64,
    ) -> Result<(), AppError> {
        self.command::<_, ()>(
            user_id,
            project_id,
            ENSURE_CHAPTER_ACCESS_OPERATION,
            ChapterOutlineAccessInput {
                project_id,
                outline_id,
            },
        )
        .await
    }

    async fn ensure_part_access(
        &self,
        user_id: i64,
        project_id: i64,
        part_id: i64,
    ) -> Result<(), AppError> {
        self.command::<_, ()>(
            user_id,
            project_id,
            ENSURE_PART_ACCESS_OPERATION,
            PartOutlineAccessInput {
                project_id,
                part_id,
            },
        )
        .await
    }
}
