use a4n_core::{
    error::AppError,
    models::{ai::generated_image, chapter::chapter},
};
use serde_json::Value;

use crate::OrchestratorContext;

use super::{ChapterVideoPromptOptions, errors::video_db_error};

impl OrchestratorContext {
    pub(crate) async fn get_chapter(&self, chapter_id: i64) -> Result<chapter::Model, AppError> {
        a4n_db::video::video_chapter_record(self.db.connection(), chapter_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn ensure_chapter_belongs_to_project(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::video::ensure_video_chapter_belongs_to_project_record(
            self.db.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn video_asset_project_ids_app(
        &self,
        project_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::video::video_asset_project_ids_record(self.db.connection(), project_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn ensure_video_reference_images_available_app(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<(), AppError> {
        self.validated_video_reference_images_app(project_id, chapter_id, reference_image_ids_json)
            .await
            .map(|_| ())
    }

    pub(super) async fn validated_video_reference_images_app(
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

    pub(super) async fn ensure_prompt_generation_inputs(
        &self,
        project_id: i64,
        chapter_id: i64,
        options: &ChapterVideoPromptOptions,
    ) -> Result<(), AppError> {
        self.ensure_chapter_belongs_to_project(project_id, chapter_id)
            .await?;
        let reference_ids_json = serde_json::json!(options.reference_image_ids);
        self.ensure_video_reference_images_available_app(
            project_id,
            chapter_id,
            Some(&reference_ids_json),
        )
        .await
    }
}
