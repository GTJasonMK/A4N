use super::{
    CompletedGalgameVisualBindingRecords, GalgameMediaBindingResult, GalgameTargetContext,
    galgame_generation_record_error, galgame_media_record_error,
};
use crate::OrchestratorContext;
use a4n_core::error::AppError;
use a4n_core::models::{ai::generated_image, generation::generation_spec};

impl OrchestratorContext {
    pub async fn galgame_media_target_context(
        &self,
        project_id: i64,
        target_kind: &str,
        target_id: i64,
    ) -> Result<GalgameTargetContext, AppError> {
        a4n_db::galgame::galgame_media_target_context(
            self.db.connection(),
            project_id,
            target_kind,
            target_id,
        )
        .await
        .map_err(galgame_media_record_error)
    }

    pub async fn galgame_generated_image_by_id(
        &self,
        image_id: i64,
        project_id: i64,
    ) -> Result<generated_image::Model, AppError> {
        a4n_db::galgame::galgame_generated_image_by_id(self.db.connection(), image_id, project_id)
            .await
            .map_err(galgame_generation_record_error)
    }

    pub async fn bind_galgame_media_records(
        &self,
        project_id: i64,
        media_asset_id: i64,
        target_kind: &str,
        target_id: i64,
        role: &str,
        required_media_kind: &str,
    ) -> Result<GalgameMediaBindingResult, AppError> {
        a4n_db::galgame::bind_galgame_media_records(
            self.db.connection(),
            project_id,
            media_asset_id,
            target_kind,
            target_id,
            role,
            required_media_kind,
        )
        .await
        .map_err(galgame_media_record_error)
    }

    pub async fn unbind_galgame_media_records(
        &self,
        project_id: i64,
        target_kind: &str,
        target_id: i64,
        role: &str,
        media_asset_id: Option<i64>,
    ) -> Result<u64, AppError> {
        a4n_db::galgame::unbind_galgame_media_records(
            self.db.connection(),
            project_id,
            target_kind,
            target_id,
            role,
            media_asset_id,
        )
        .await
        .map_err(galgame_media_record_error)
    }

    pub async fn completed_galgame_visual_binding_records(
        &self,
        spec: &generation_spec::Model,
        target_entity_kind: &str,
        target_entity_id: i64,
        role: &str,
    ) -> Result<CompletedGalgameVisualBindingRecords, AppError> {
        a4n_db::galgame::completed_galgame_visual_binding_records(
            self.db.connection(),
            spec,
            target_entity_kind,
            target_entity_id,
            role,
        )
        .await
        .map_err(galgame_media_record_error)
    }

    pub async fn delete_media_asset_with_provenance_records(
        &self,
        media_asset_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::galgame::delete_media_asset_with_provenance_records(
            self.db.connection(),
            media_asset_id,
        )
        .await
        .map_err(galgame_media_record_error)
    }
}
