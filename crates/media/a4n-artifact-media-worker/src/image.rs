use a4n_api_contract::image::{
    DeleteImageInput, GeneratedImageSummary, ListImagesByChapterInput, ListImagesByProjectInput,
    ListImagesByProjectsInput, UpdateGeneratedImageInput,
};
use a4n_artifact_media_contract::ImageReferenceIdsRequest;

use super::*;

impl WorkerState {
    pub(super) async fn image_by_id(
        &self,
        user_id: i64,
        request: ImageIdRequest,
    ) -> Result<Option<GeneratedImageSummary>, String> {
        if request.image_id <= 0 {
            return Err("image_id must be positive".to_string());
        }
        let Some(image) = a4n_db::video::read_models::generated_image_by_id(
            self.db.connection(),
            request.image_id,
        )
        .await
        .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        verify_image_permission(&self.db, user_id, image.project_id, "viewer").await?;
        Ok(Some(generated_image_summary(&image)))
    }

    pub(super) async fn image_list_by_project(
        &self,
        user_id: i64,
        request: ListImagesByProjectInput,
    ) -> Result<Vec<GeneratedImageSummary>, String> {
        if request.project_id <= 0 {
            return Err("project_id must be positive".to_string());
        }
        verify_image_permission(&self.db, user_id, request.project_id, "viewer").await?;
        a4n_db::video::read_models::generated_images_for_project(
            self.db.connection(),
            request.project_id,
        )
        .await
        .map(|images| images.iter().map(generated_image_summary).collect())
        .map_err(|error| error.to_string())
    }

    pub(super) async fn image_list_by_projects(
        &self,
        user_id: i64,
        request: ListImagesByProjectsInput,
    ) -> Result<Vec<GeneratedImageSummary>, String> {
        let project_ids = normalized_project_ids(request.project_ids)?;
        for project_id in &project_ids {
            verify_image_permission(&self.db, user_id, *project_id, "viewer").await?;
        }
        let mut images = a4n_db::video::read_models::generated_images_for_projects(
            self.db.connection(),
            &project_ids,
        )
        .await
        .map_err(|error| error.to_string())?;
        sort_images_by_project_order(&mut images, &project_ids);
        Ok(images.iter().map(generated_image_summary).collect())
    }

    pub(super) async fn image_list_by_chapter(
        &self,
        user_id: i64,
        request: ListImagesByChapterInput,
    ) -> Result<Vec<GeneratedImageSummary>, String> {
        if request.project_id <= 0 || request.chapter_id <= 0 {
            return Err("project_id 和 chapter_id must be positive".to_string());
        }
        verify_image_permission(&self.db, user_id, request.project_id, "viewer").await?;
        a4n_db::video::ensure_image_chapter_belongs_to_project_record(
            self.db.connection(),
            request.project_id,
            request.chapter_id,
        )
        .await
        .map_err(|error| error.to_string())?;
        a4n_db::video::read_models::generated_images_for_chapter(
            self.db.connection(),
            request.project_id,
            request.chapter_id,
        )
        .await
        .map(|images| images.iter().map(generated_image_summary).collect())
        .map_err(|error| error.to_string())
    }

    pub(super) async fn image_reference_by_ids(
        &self,
        user_id: i64,
        request: ImageReferenceIdsRequest,
    ) -> Result<Vec<GeneratedImageSummary>, String> {
        if request.project_id <= 0 || request.image_ids.iter().any(|id| *id <= 0) {
            return Err("project_id 和 image_ids 必须为正整数".to_string());
        }
        verify_image_permission(&self.db, user_id, request.project_id, "viewer").await?;
        a4n_db::video::read_models::portrait_reference_images_by_ids(
            self.db.connection(),
            request.project_id,
            &request.image_ids,
        )
        .await
        .map(|images| images.iter().map(generated_image_summary).collect())
        .map_err(|error| error.to_string())
    }

    pub(super) async fn update_image(
        &self,
        user_id: i64,
        input: UpdateGeneratedImageInput,
    ) -> Result<GeneratedImageSummary, String> {
        if input.is_empty() {
            return Err("图片更新至少需要一个字段".to_string());
        }
        let image =
            a4n_db::video::read_models::generated_image_by_id(self.db.connection(), input.image_id)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "图片不存在".to_string())?;
        verify_image_permission(&self.db, user_id, image.project_id, "editor").await?;
        let updated = a4n_db::video::read_models::update_generated_image_record(
            self.db.connection(),
            input.image_id,
            a4n_domain::media::UpdateGeneratedImageRecordInput {
                prompt: input.prompt,
                image_type: input.image_type,
                chapter_id: input.chapter_id.flatten(),
                clear_chapter_id: input.chapter_id.as_ref().is_some_and(Option::is_none),
                panel_id: input.panel_id.clone().flatten(),
                clear_panel_id: input.panel_id.as_ref().is_some_and(Option::is_none),
                ai_score: input.ai_score.flatten(),
                clear_ai_score: input.ai_score.as_ref().is_some_and(Option::is_none),
                is_favorite: input.is_favorite,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
        Ok(generated_image_summary(&updated))
    }

    pub(super) async fn delete_image(
        &self,
        user_id: i64,
        input: DeleteImageInput,
    ) -> Result<(), String> {
        if input.image_id <= 0 {
            return Err("image_id must be positive".to_string());
        }
        let image =
            a4n_db::video::read_models::generated_image_by_id(self.db.connection(), input.image_id)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "图片不存在".to_string())?;
        verify_image_permission(&self.db, user_id, image.project_id, "editor").await?;
        a4n_db::video::delete_generated_image_record_with_cleanup(
            self.db.connection(),
            input.image_id,
        )
        .await
        .map_err(|error| error.to_string())
    }
}
