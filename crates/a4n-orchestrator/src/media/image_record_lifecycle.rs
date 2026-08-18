use std::collections::{HashMap, HashSet};

use a4n_api_contract::image::UploadImageInput;
use a4n_core::{error::AppError, models::ai::generated_image};
use a4n_db::video::{CreateGeneratedImageRecordDraftInput, UpdateGeneratedImageRecordInput};
use a4n_domain::EXTERNAL_IMPORT_MODEL;
use chrono::Utc;
use serde_json::{Map, Value, json};

use super::{DecodedUploadedImage, decode_uploaded_image_data};
use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    project::required_text,
    video::{
        image_type_rank, is_video_reference_image_candidate,
        resolve_video_reference_image_project_ids, video_reference_scope_rank,
    },
};

const MAX_IMPORT_IMAGE_BYTES: usize = 50 * 1024 * 1024;

pub struct PreparedImportedImageUpload {
    pub project_id: i64,
    pub image_type: String,
    pub prompt: String,
    pub model: String,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Map<String, Value>>,
    pub decoded: DecodedUploadedImage,
}

impl OrchestratorContext {
    pub async fn list_generated_images_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        self.generated_images_for_project(project_id).await
    }

    pub async fn list_chapter_images_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        self.generated_images_for_valid_chapter(project_id, chapter_id)
            .await
    }

    pub async fn list_video_reference_images_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        requested_project_ids: Vec<i64>,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let allowed_project_ids = self.video_asset_project_ids(project_id).await?;
        let project_ids =
            resolve_video_reference_image_project_ids(requested_project_ids, &allowed_project_ids)
                .map_err(AppError::InvalidArgument)?;
        self.verify_project_permissions(&project_ids, user_id, ProjectAccessRole::Editor)
            .await?;
        let primary_image_ids = self
            .primary_material_image_ids_for_projects(&project_ids)
            .await?
            .into_iter()
            .collect::<HashSet<_>>();
        let mut by_id = HashMap::new();
        for image in self
            .video_reference_candidate_images(&project_ids, chapter_id)
            .await?
        {
            if is_video_reference_image_candidate(&image, chapter_id) {
                by_id.insert(image.id, image);
            }
        }
        let mut images = by_id.into_values().collect::<Vec<_>>();
        sort_video_reference_images(&mut images, chapter_id, &primary_image_ids);
        Ok(images)
    }

    pub async fn prepare_imported_image_upload_for_user_validated(
        &self,
        user_id: i64,
        input: UploadImageInput,
    ) -> Result<PreparedImportedImageUpload, AppError> {
        let project_id = input.project_id;
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let image_type = required_text(&input.image_type, "图片类型")
            .map(str::to_string)
            .map_err(AppError::InvalidArgument)?;
        let prompt = required_text(&input.prompt, "图片提示词")
            .map(str::to_string)
            .map_err(AppError::InvalidArgument)?;
        let decoded = decode_uploaded_image_data(
            &input.file_name,
            &input.mime_type,
            &input.data_base64,
            MAX_IMPORT_IMAGE_BYTES,
        )
        .map_err(AppError::InvalidArgument)?;
        Ok(PreparedImportedImageUpload {
            project_id,
            image_type,
            prompt,
            model: EXTERNAL_IMPORT_MODEL.to_string(),
            chapter_id: input.chapter_id,
            panel_id: input.panel_id,
            metadata_json: input.metadata_json,
            decoded,
        })
    }

    pub async fn create_imported_image_record_for_user_validated(
        &self,
        user_id: i64,
        prepared: PreparedImportedImageUpload,
        file_path: String,
        file_size: i64,
    ) -> Result<generated_image::Model, AppError> {
        self.verify_project_permission(prepared.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let PreparedImportedImageUpload {
            project_id,
            image_type,
            prompt,
            model,
            chapter_id,
            panel_id,
            metadata_json,
            decoded,
        } = prepared;
        let metadata_json = imported_image_metadata(
            metadata_json,
            decoded.source_file_name,
            decoded.extension,
            decoded.mime_type,
        );
        self.create_generated_image_record(CreateGeneratedImageRecordDraftInput {
            project_id,
            image_type,
            file_path,
            prompt,
            model,
            width: decoded.width,
            height: decoded.height,
            file_size: Some(file_size),
            chapter_id,
            panel_id,
            metadata_json,
            created_at: None,
        })
        .await
    }

    pub async fn update_generated_image_for_user_validated(
        &self,
        user_id: i64,
        image_id: i64,
        input: UpdateGeneratedImageRecordInput,
    ) -> Result<generated_image::Model, AppError> {
        self.generated_image_for_user_validated(user_id, image_id)
            .await?;
        self.update_generated_image_record(image_id, input).await
    }

    pub async fn delete_generated_image_for_user_validated(
        &self,
        user_id: i64,
        image_id: i64,
    ) -> Result<(), AppError> {
        let image = self
            .generated_image_for_user_validated(user_id, image_id)
            .await?;
        self.delete_generated_image_validated(image).await
    }

    pub async fn delete_generated_image_validated(
        &self,
        image: generated_image::Model,
    ) -> Result<(), AppError> {
        if image.id <= 0 {
            return Err(AppError::InvalidArgument(
                "图片记录 ID 必须是正整数".to_string(),
            ));
        }
        self.delete_generated_image_record_with_cleanup(image.id)
            .await
    }

    async fn generated_image_for_user_validated(
        &self,
        user_id: i64,
        image_id: i64,
    ) -> Result<generated_image::Model, AppError> {
        let image = self
            .generated_image_by_id(image_id)
            .await?
            .ok_or_else(|| AppError::NotFound("图片不存在".to_string()))?;
        self.verify_project_permission(image.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(image)
    }
}

fn sort_video_reference_images(
    images: &mut [generated_image::Model],
    chapter_id: i64,
    primary_image_ids: &HashSet<i64>,
) {
    images.sort_by(|a, b| {
        video_reference_scope_rank(a, chapter_id)
            .cmp(&video_reference_scope_rank(b, chapter_id))
            .then_with(|| {
                primary_image_ids
                    .contains(&b.id)
                    .cmp(&primary_image_ids.contains(&a.id))
            })
            .then_with(|| b.is_favorite.cmp(&a.is_favorite))
            .then_with(|| image_type_rank(&a.image_type).cmp(&image_type_rank(&b.image_type)))
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| b.id.cmp(&a.id))
    });
}

fn imported_image_metadata(
    metadata_json: Option<Map<String, Value>>,
    source_file_name: String,
    source_extension: &'static str,
    mime_type: &'static str,
) -> Option<Value> {
    let mut metadata = metadata_json.unwrap_or_default();
    metadata.insert(
        "import".to_string(),
        json!({
            "external": true,
            "source_file_name": source_file_name,
            "source_extension": source_extension,
            "mime_type": mime_type,
            "imported_at": Utc::now().to_rfc3339(),
        }),
    );
    super::image_generation::normalize_image_metadata(Some(Value::Object(metadata)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::image::IMAGE_TYPE_SCENE_MATERIAL;

    #[test]
    fn video_reference_sort_prefers_primary_before_favorite() {
        let created_at = chrono::NaiveDate::from_ymd_opt(2026, 7, 24)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let image = |id, favorite| generated_image::Model {
            id,
            project_id: 1,
            image_type: IMAGE_TYPE_SCENE_MATERIAL.to_string(),
            file_path: format!("images/{id}.png"),
            prompt: "scene".to_string(),
            model: "test".to_string(),
            width: 1,
            height: 1,
            file_size: Some(1),
            ai_score: None,
            is_favorite: favorite,
            chapter_id: Some(2),
            panel_id: None,
            metadata_json: None,
            created_at,
        };
        let mut images = vec![image(1, true), image(2, false)];
        sort_video_reference_images(&mut images, 2, &HashSet::from([2]));
        assert_eq!(
            images.iter().map(|image| image.id).collect::<Vec<_>>(),
            vec![2, 1]
        );
    }
}
