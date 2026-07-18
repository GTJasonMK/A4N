use super::*;

impl OrchestratorContext {
    pub async fn generated_images_for_video_prompt_references(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::generated_images_for_video_prompt_references(
            self.db.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn generated_images_by_ids(
        &self,
        image_ids: &[i64],
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::generated_images_by_ids(self.db.connection(), image_ids)
            .await
            .map_err(video_db_error)
    }

    pub async fn generated_images_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::generated_images_for_project(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn generated_images_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::generated_images_for_chapter(self.db.connection(), project_id, chapter_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn generated_images_for_valid_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::ensure_image_chapter_belongs_to_project_record(
            self.db.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)?;
        self.generated_images_for_chapter(project_id, chapter_id)
            .await
    }

    pub async fn video_asset_project_ids(&self, project_id: i64) -> Result<Vec<i64>, AppError> {
        a4n_db::video::video_asset_project_ids_record(self.db.connection(), project_id)
            .await
            .map_err(video_db_error)
    }

    pub async fn video_reference_candidate_images(
        &self,
        project_ids: &[i64],
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::video_reference_candidate_image_records(
            self.db.connection(),
            project_ids,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn ensure_video_reference_images_available(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<(), AppError> {
        a4n_db::video::ensure_video_reference_images_available_record(
            self.db.connection(),
            project_id,
            chapter_id,
            reference_image_ids_json,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn validated_video_reference_images(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::validated_video_reference_image_records(
            self.db.connection(),
            project_id,
            chapter_id,
            reference_image_ids_json,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn generated_image_by_id(
        &self,
        image_id: i64,
    ) -> Result<Option<generated_image::Model>, AppError> {
        a4n_db::video::generated_image_by_id(self.db.connection(), image_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn toggle_generated_image_favorite(
        &self,
        image_id: i64,
    ) -> Result<generated_image::Model, AppError> {
        a4n_db::video::toggle_generated_image_favorite_record(self.db.connection(), image_id)
            .await
            .map_err(video_db_error)
    }

    pub async fn update_generated_image_record(
        &self,
        image_id: i64,
        input: UpdateGeneratedImageRecordInput,
    ) -> Result<generated_image::Model, AppError> {
        a4n_db::video::update_generated_image_record(self.db.connection(), image_id, input)
            .await
            .map_err(video_db_error)
    }

    pub async fn delete_generated_image_record(&self, image_id: i64) -> Result<(), AppError> {
        a4n_db::video::delete_generated_image_record(self.db.connection(), image_id)
            .await
            .map_err(video_db_error)
    }

    pub async fn delete_manga_page_image_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
        preserve_image_id: Option<i64>,
    ) -> Result<Vec<String>, AppError> {
        a4n_db::video::delete_manga_page_image_records(
            self.db.connection(),
            project_id,
            chapter_id,
            page_number,
            preserve_image_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub async fn video_tail_frame_images(&self) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::video_tail_frame_images(self.db.connection())
            .await
            .map_err(video_db_error)
    }
}
