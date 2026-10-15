//! 图片记录与 publication facade。

use super::*;

impl OrchestratorContext {
    pub(crate) async fn generated_images_for_video_prompt_references(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedImageReferenceRecord>, AppError> {
        self.video
            .generated_images_for_video_prompt_references(project_id, chapter_id)
            .await
    }

    pub(crate) async fn generated_images_by_ids(
        &self,
        image_ids: &[i64],
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        self.video.generated_images_by_ids(image_ids).await
    }

    pub(crate) async fn generated_image_ids_by_ids(
        &self,
        image_ids: &[i64],
    ) -> Result<Vec<i64>, AppError> {
        self.video.generated_image_ids_by_ids(image_ids).await
    }

    pub(crate) async fn generated_images_for_project(
        &self,
        project_id: i64,
        image_ids: Option<Vec<i64>>,
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        self.video
            .generated_images_for_project(project_id, image_ids)
            .await
    }

    pub(crate) async fn generated_images_for_projects(
        &self,
        project_ids: &[i64],
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        self.video.generated_images_for_projects(project_ids).await
    }

    pub(crate) async fn generated_images_for_valid_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        self.video
            .ensure_image_chapter_belongs_to_project(project_id, chapter_id)
            .await?;
        self.video
            .generated_images_for_chapter(project_id, chapter_id)
            .await
    }

    pub(crate) async fn video_asset_project_ids(
        &self,
        project_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        self.video.video_asset_project_ids_record(project_id).await
    }

    pub(crate) async fn video_reference_candidate_images(
        &self,
        project_ids: &[i64],
        chapter_id: i64,
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        self.video
            .video_reference_candidate_image_records(project_ids, chapter_id)
            .await
    }

    pub(crate) async fn ensure_video_reference_images_available(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<(), AppError> {
        self.video
            .ensure_video_reference_images_available_record(
                project_id,
                chapter_id,
                reference_image_ids_json,
            )
            .await
    }

    pub(crate) async fn validated_video_reference_images(
        &self,
        project_id: i64,
        chapter_id: Option<i64>,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        self.video
            .validated_video_reference_image_records(
                project_id,
                chapter_id,
                reference_image_ids_json,
            )
            .await
    }

    pub(crate) async fn generated_image_by_id(
        &self,
        image_id: i64,
    ) -> Result<Option<GeneratedImageRecord>, AppError> {
        self.video.generated_image_by_id(image_id).await
    }

    pub(crate) async fn update_generated_image_record(
        &self,
        image_id: i64,
        input: UpdateGeneratedImageRecordInput,
    ) -> Result<GeneratedImageRecord, AppError> {
        self.video
            .update_generated_image_record(image_id, input)
            .await
    }

    pub(crate) async fn delete_manga_page_image_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
        preserve_image_id: Option<i64>,
    ) -> Result<Vec<String>, AppError> {
        self.video
            .delete_manga_page_image_records(project_id, chapter_id, page_number, preserve_image_id)
            .await
    }

    pub(crate) async fn video_tail_frame_images(
        &self,
    ) -> Result<Vec<(i64, String, Option<serde_json::Value>)>, AppError> {
        self.video.video_tail_frame_images().await
    }

    pub(crate) async fn create_generated_image_record(
        &self,
        input: CreateGeneratedImageRecordDraftInput,
    ) -> Result<GeneratedImageRecord, AppError> {
        self.video.create_generated_image_record(input).await
    }

    pub(crate) async fn create_image_publication_intent(
        &self,
        input: CreateImagePublicationIntentInput,
    ) -> Result<(ImagePublicationRecord, OwnedMutexGuard<()>), AppError> {
        self.video.create_image_publication_intent(input).await
    }

    /// The Host already owns its data directory/database exclusively. Keep this
    /// guard through file inspection and publication so recovery cannot race a writer.
    pub(crate) async fn lock_image_publications(&self) -> OwnedMutexGuard<()> {
        self.video.image_publication_gate.clone().lock_owned().await
    }

    pub(crate) async fn image_publication_by_id(
        &self,
        publication_id: &str,
    ) -> Result<Option<ImagePublicationRecord>, AppError> {
        self.video.image_publication_by_id(publication_id).await
    }

    pub(crate) async fn publish_image_publication(
        &self,
        publication_id: &str,
        facts: ImagePublicationFileFacts,
    ) -> Result<GeneratedImageRecord, AppError> {
        self.video
            .publish_image_publication(publication_id, facts)
            .await
    }

    pub(crate) async fn publish_image_publication_with_generation_run_checkpoint(
        &self,
        publication_id: &str,
        facts: ImagePublicationFileFacts,
        generation_run: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<GeneratedImageRecord, AppError> {
        self.video
            .publish_image_publication_with_generation_run_checkpoint(
                publication_id,
                facts,
                generation_run,
            )
            .await
    }

    pub(crate) async fn update_image_publication_metadata(
        &self,
        publication_id: &str,
        expected_version: i64,
        metadata_json: Value,
    ) -> Result<ImagePublicationRecord, AppError> {
        self.video
            .update_image_publication_metadata(publication_id, expected_version, metadata_json)
            .await
    }

    pub(crate) async fn image_publications_for_recovery(
        &self,
        limit: usize,
        published_after: Option<&str>,
    ) -> Result<Vec<ImagePublicationRecord>, AppError> {
        self.video
            .image_publications_for_recovery(limit, published_after)
            .await
    }

    pub(crate) async fn mark_image_publication_reclaiming(
        &self,
        publication_id: &str,
        version: i64,
        reason: String,
    ) -> Result<ImagePublicationRecord, AppError> {
        self.video
            .mark_image_publication_reclaiming(publication_id, version, reason)
            .await
    }

    pub(crate) async fn mark_image_publication_reclaimed(
        &self,
        publication_id: &str,
        version: i64,
        reason: Option<String>,
    ) -> Result<ImagePublicationRecord, AppError> {
        self.video
            .mark_image_publication_reclaimed(publication_id, version, reason)
            .await
    }
}
