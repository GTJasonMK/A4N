//! 图片记录与 publication owner。

use super::*;

impl VideoApplication {
    pub(in crate::context) async fn video_asset_project_ids_record(
        &self,
        project_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::video::video_asset_project_ids_record(self.connection(), project_id)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn validated_video_reference_image_records(
        &self,
        project_id: i64,
        chapter_id: Option<i64>,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        a4n_db::video::read_models::validated_video_reference_image_records(
            self.database.executor(),
            project_id,
            chapter_id,
            reference_image_ids_json,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn ensure_video_reference_images_available_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<(), AppError> {
        a4n_db::video::ensure_video_reference_images_available_record(
            self.connection(),
            project_id,
            chapter_id,
            reference_image_ids_json,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn generated_images_for_video_prompt_references(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedImageReferenceRecord>, AppError> {
        let project_ids = self.video_asset_project_ids_record(project_id).await?;
        a4n_db::video::read_models::generated_images_for_projects_chapter_by_id_desc(
            self.connection(),
            &project_ids,
            chapter_id,
        )
        .await
        .map_err(AppError::from)
    }

    pub(in crate::context) async fn generated_images_by_ids(
        &self,
        image_ids: &[i64],
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        a4n_db::video::read_models::generated_images_by_ids(self.database.executor(), image_ids)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn generated_image_ids_by_ids(
        &self,
        image_ids: &[i64],
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::video::generated_image_ids_by_ids(self.connection(), image_ids)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn generated_images_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        a4n_db::video::read_models::generated_images_for_project(self.connection(), project_id)
            .await
            .map_err(AppError::from)
    }

    pub(in crate::context) async fn generated_images_for_projects(
        &self,
        project_ids: &[i64],
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        a4n_db::video::read_models::generated_images_for_projects(self.connection(), project_ids)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn generated_images_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        a4n_db::video::read_models::generated_images_for_chapter(
            self.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(AppError::from)
    }

    pub(in crate::context) async fn ensure_image_chapter_belongs_to_project(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::video::ensure_image_chapter_belongs_to_project_record(
            self.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn video_reference_candidate_image_records(
        &self,
        project_ids: &[i64],
        chapter_id: i64,
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        a4n_db::video::read_models::video_reference_candidate_image_records(
            self.connection(),
            project_ids,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn generated_image_by_id(
        &self,
        image_id: i64,
    ) -> Result<Option<GeneratedImageRecord>, AppError> {
        a4n_db::video::read_models::generated_image_by_id(self.connection(), image_id)
            .await
            .map_err(AppError::from)
    }

    pub(in crate::context) async fn update_generated_image_record(
        &self,
        image_id: i64,
        input: UpdateGeneratedImageRecordInput,
    ) -> Result<GeneratedImageRecord, AppError> {
        a4n_db::video::read_models::update_generated_image_record(
            self.connection(),
            image_id,
            input,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn delete_generated_image_record_with_cleanup(
        &self,
        image_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::video::delete_generated_image_record_with_cleanup(self.connection(), image_id)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn delete_manga_page_image_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
        preserve_image_id: Option<i64>,
    ) -> Result<Vec<String>, AppError> {
        a4n_db::video::delete_manga_page_image_records(
            self.connection(),
            project_id,
            chapter_id,
            page_number,
            preserve_image_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn video_tail_frame_images(
        &self,
    ) -> Result<Vec<(i64, String, Option<serde_json::Value>)>, AppError> {
        a4n_db::video::video_tail_frame_images(self.connection())
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn create_generated_image_record(
        &self,
        input: CreateGeneratedImageRecordDraftInput,
    ) -> Result<GeneratedImageRecord, AppError> {
        a4n_db::video::read_models::create_generated_image_record(self.connection(), input)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn create_image_publication_intent(
        &self,
        input: CreateImagePublicationIntentInput,
    ) -> Result<(ImagePublicationRecord, OwnedMutexGuard<()>), AppError> {
        let guard = self.image_publication_gate.clone().lock_owned().await;
        let publication =
            a4n_db::video::create_image_publication_intent_record(self.connection(), input)
                .await
                .map_err(video_db_error)?;
        Ok((publication, guard))
    }

    pub(in crate::context) async fn image_publication_by_id(
        &self,
        publication_id: &str,
    ) -> Result<Option<ImagePublicationRecord>, AppError> {
        a4n_db::video::image_publication_record_by_id(self.connection(), publication_id)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn publish_image_publication(
        &self,
        publication_id: &str,
        facts: ImagePublicationFileFacts,
    ) -> Result<GeneratedImageRecord, AppError> {
        a4n_db::video::read_models::publish_image_publication(
            self.connection(),
            publication_id,
            facts,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn publish_image_publication_with_generation_run_checkpoint(
        &self,
        publication_id: &str,
        facts: ImagePublicationFileFacts,
        generation_run: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<GeneratedImageRecord, AppError> {
        a4n_db::video::read_models::publish_image_publication_with_generation_run_checkpoint(
            self.connection(),
            publication_id,
            facts,
            generation_run,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn update_image_publication_metadata(
        &self,
        publication_id: &str,
        expected_version: i64,
        metadata_json: Value,
    ) -> Result<ImagePublicationRecord, AppError> {
        a4n_db::video::update_image_publication_metadata_record(
            self.connection(),
            publication_id,
            expected_version,
            metadata_json,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn image_publications_for_recovery(
        &self,
        limit: usize,
        published_after: Option<&str>,
    ) -> Result<Vec<ImagePublicationRecord>, AppError> {
        a4n_db::video::image_publication_recovery_records(self.connection(), limit, published_after)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn mark_image_publication_reclaiming(
        &self,
        publication_id: &str,
        version: i64,
        reason: String,
    ) -> Result<ImagePublicationRecord, AppError> {
        a4n_db::video::mark_image_publication_reclaiming_record(
            self.connection(),
            publication_id,
            version,
            reason,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn mark_image_publication_reclaimed(
        &self,
        publication_id: &str,
        version: i64,
        reason: Option<String>,
    ) -> Result<ImagePublicationRecord, AppError> {
        a4n_db::video::mark_image_publication_reclaimed_record(
            self.connection(),
            publication_id,
            version,
            reason,
        )
        .await
        .map_err(video_db_error)
    }
}
