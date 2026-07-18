pub use a4n_db::profile_material::{
    IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL, PROFILE_KIND_OBJECT, PROFILE_KIND_SCENE,
    ProfileMaterialDbError, ProfileMaterialListRows, generated_image_profile_material_identity,
    is_project_reusable_video_material_image, is_video_frame_input_metadata,
    profile_material_identity,
};

use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn profile_material_list_rows(
        &self,
        project_id: i64,
        image_types: &[&str],
    ) -> Result<ProfileMaterialListRows, AppError> {
        a4n_db::profile_material::profile_material_list_rows(
            self.db.connection(),
            project_id,
            image_types,
        )
        .await
        .map_err(profile_material_db_error)
    }

    pub async fn profile_material_images_by_type(
        &self,
        project_id: i64,
        image_type: &str,
    ) -> Result<Vec<a4n_core::models::ai::generated_image::Model>, AppError> {
        a4n_db::profile_material::profile_material_images_by_type(
            self.db.connection(),
            project_id,
            image_type,
        )
        .await
        .map_err(profile_material_db_error)
    }

    pub async fn upsert_profile_primary_material(
        &self,
        project_id: i64,
        profile_kind: &str,
        profile_id: i64,
        image_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::profile_material::upsert_profile_primary_material(
            self.db.connection(),
            project_id,
            profile_kind,
            profile_id,
            image_id,
        )
        .await
        .map_err(profile_material_db_error)
    }

    pub async fn delete_profile_primary_material(
        &self,
        project_id: i64,
        profile_kind: &str,
        profile_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::profile_material::delete_profile_primary_material(
            self.db.connection(),
            project_id,
            profile_kind,
            profile_id,
        )
        .await
        .map_err(profile_material_db_error)
    }
}

fn profile_material_db_error(error: ProfileMaterialDbError) -> AppError {
    match error {
        ProfileMaterialDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ProfileMaterialDbError::Database { source, .. } => AppError::Database(source),
    }
}
