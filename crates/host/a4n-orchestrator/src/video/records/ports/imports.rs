//! 视频导入 ports。

use super::*;
use async_trait::async_trait;

#[async_trait]
impl ChapterVideoImportApplicationPort for crate::OrchestratorContext {
    async fn create_imported_chapter_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
        facts: ImportedVideoRecordFacts<'_>,
    ) -> Result<VideoImportRecordSummary, AppError> {
        let prepared = self
            .create_imported_chapter_video_record_validated(
                project_id, chapter_id, prompt_id, facts,
            )
            .await?;
        Ok(video_import_record_summary(
            &prepared.video,
            Some(prepared.prompt.id),
        ))
    }
}

#[async_trait]
impl ReferenceMediaVideoImportApplicationPort for crate::OrchestratorContext {
    async fn create_imported_reference_media_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        facts: ImportedVideoRecordFacts<'_>,
        segment_plan_prompt_id: i64,
        target_source_shot_id: &str,
        target_segment_index: i32,
    ) -> Result<VideoImportRecordSummary, AppError> {
        let video = self
            .create_imported_reference_media_video_record_validated(
                project_id,
                chapter_id,
                facts,
                segment_plan_prompt_id,
                (target_source_shot_id, target_segment_index),
            )
            .await?;
        Ok(video_import_record_summary(&video, None))
    }
}

#[async_trait]
impl AssemblyVideoImportApplicationPort for crate::OrchestratorContext {
    async fn create_imported_assembly_insert_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        insert_after_video_id: Option<i64>,
        facts: ImportedVideoRecordFacts<'_>,
    ) -> Result<VideoImportRecordSummary, AppError> {
        let video = self
            .create_imported_assembly_insert_video_record_validated(
                project_id,
                chapter_id,
                workflow_mode,
                insert_after_video_id,
                facts,
            )
            .await?;
        Ok(video_import_record_summary(&video, None))
    }
}

#[async_trait]
impl CanvasVideoImportApplicationPort for crate::OrchestratorContext {
    async fn create_imported_canvas_video_record(
        &self,
        project_id: i64,
        facts: ImportedVideoRecordFacts<'_>,
    ) -> Result<VideoImportRecordSummary, AppError> {
        let video = self
            .create_imported_canvas_video_record_validated(project_id, facts)
            .await?;
        Ok(video_import_record_summary(&video, None))
    }
}
