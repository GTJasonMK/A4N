//! 资料素材领域私有 owner。
//!
//! 持有 `DbHandle`，封装资料列表、按类型图片查询与主素材 upsert/delete。

use a4n_core::error::AppError;
use a4n_core::models::ai::generated_image;
use a4n_db::DbHandle;
use a4n_db::profile_material::{ProfileMaterialDbError, ProfileMaterialListRows};

use super::OrchestratorContext;

#[derive(Clone)]
pub(super) struct ProfileMaterialApplication {
    database: DbHandle,
}

impl ProfileMaterialApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &sea_orm::DatabaseConnection {
        self.database.connection()
    }

    async fn profile_material_list_rows(
        &self,
        project_id: i64,
        image_types: &[&str],
    ) -> Result<ProfileMaterialListRows, AppError> {
        a4n_db::profile_material::profile_material_list_rows(
            self.connection(),
            project_id,
            image_types,
        )
        .await
        .map_err(profile_material_db_error)
    }

    async fn profile_material_images_by_type(
        &self,
        project_id: i64,
        image_type: &str,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::profile_material::profile_material_images_by_type(
            self.connection(),
            project_id,
            image_type,
        )
        .await
        .map_err(profile_material_db_error)
    }

    async fn upsert_profile_primary_material(
        &self,
        project_id: i64,
        profile_kind: &str,
        profile_id: i64,
        image_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::profile_material::upsert_profile_primary_material(
            self.connection(),
            project_id,
            profile_kind,
            profile_id,
            image_id,
        )
        .await
        .map_err(profile_material_db_error)
    }

    async fn delete_profile_primary_material(
        &self,
        project_id: i64,
        profile_kind: &str,
        profile_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::profile_material::delete_profile_primary_material(
            self.connection(),
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

impl OrchestratorContext {
    pub(crate) async fn profile_material_list_rows(
        &self,
        project_id: i64,
        image_types: &[&str],
    ) -> Result<ProfileMaterialListRows, AppError> {
        self.profile_material
            .profile_material_list_rows(project_id, image_types)
            .await
    }

    pub(crate) async fn profile_material_images_by_type(
        &self,
        project_id: i64,
        image_type: &str,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.profile_material
            .profile_material_images_by_type(project_id, image_type)
            .await
    }

    pub(crate) async fn upsert_profile_primary_material(
        &self,
        project_id: i64,
        profile_kind: &str,
        profile_id: i64,
        image_id: i64,
    ) -> Result<(), AppError> {
        self.profile_material
            .upsert_profile_primary_material(project_id, profile_kind, profile_id, image_id)
            .await
    }

    pub(crate) async fn delete_profile_primary_material(
        &self,
        project_id: i64,
        profile_kind: &str,
        profile_id: i64,
    ) -> Result<(), AppError> {
        self.profile_material
            .delete_profile_primary_material(project_id, profile_kind, profile_id)
            .await
    }
}
