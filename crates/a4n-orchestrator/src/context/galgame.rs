//! Galgame 领域私有 owner。
//!
//! 持有 `DbHandle`，封装剧本读写、行/场景/选项变更、资产生成意图与媒体绑定的数据库行为。
//! 流水线 LLM 编排仍在 `galgame/` 模块，但 DB 只经本 owner（及既有 workflow owner）。

use a4n_core::error::AppError;
use a4n_core::models::{
    ai::generated_image,
    character::character_portrait,
    generation::{generation_spec, prompt_artifact},
};
use a4n_db::DbHandle;
use a4n_db::galgame::{
    CompletedGalgameVisualBindingRecords, GalgameMediaBindingResult,
    GalgamePortraitSpecTargetInput, GalgamePromptArtifactUpsertInput, GalgameTargetContext,
};

use super::OrchestratorContext;
use crate::galgame::{
    galgame_asset_plan_spec_record_error, galgame_generation_record_error,
    galgame_media_record_error,
};

/// Galgame 领域 application owner。
#[derive(Clone)]
pub(super) struct GalgameApplication {
    database: DbHandle,
}

impl GalgameApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn galgame_asset_generation_specs(
        &self,
        project_id: i64,
        target_kinds: &[&str],
        generation_spec_ids: &[i64],
        limit: Option<usize>,
    ) -> Result<Vec<generation_spec::Model>, AppError> {
        a4n_db::galgame::galgame_asset_generation_specs(
            self.database.connection(),
            project_id,
            target_kinds,
            generation_spec_ids,
            limit,
        )
        .await
        .map_err(galgame_asset_plan_spec_record_error)
    }

    async fn galgame_webgal_portraits(
        &self,
        root_project_id: i64,
    ) -> Result<Vec<character_portrait::Model>, AppError> {
        a4n_db::galgame::galgame_webgal_portraits(self.database.connection(), root_project_id)
            .await
            .map_err(galgame_generation_record_error)
    }

    async fn galgame_portrait_spec_target_profile(
        &self,
        spec: &generation_spec::Model,
        input: &GalgamePortraitSpecTargetInput,
    ) -> Result<a4n_db::galgame::GalgamePortraitSpecTargetProfile, AppError> {
        a4n_db::galgame::galgame_portrait_spec_target_profile(
            self.database.connection(),
            spec,
            input,
        )
        .await
        .map_err(galgame_generation_record_error)
    }

    async fn completed_galgame_portrait_record(
        &self,
        spec: &generation_spec::Model,
        profile_id: i64,
        expression: &str,
        pose: &str,
    ) -> Result<character_portrait::Model, AppError> {
        a4n_db::galgame::completed_galgame_portrait_record(
            self.database.connection(),
            spec,
            profile_id,
            expression,
            pose,
        )
        .await
        .map_err(galgame_generation_record_error)
    }

    async fn primary_galgame_portrait_reference_image_ids(
        &self,
        profile: &a4n_db::galgame::GalgamePortraitSpecTargetProfile,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::galgame::primary_galgame_portrait_reference_image_ids(
            self.database.connection(),
            profile,
        )
        .await
        .map_err(galgame_generation_record_error)
    }

    async fn mark_galgame_generation_spec_status_record(
        &self,
        generation_spec_id: i64,
        status: &str,
    ) -> Result<(), AppError> {
        a4n_db::galgame::mark_galgame_generation_spec_status_record(
            self.database.connection(),
            generation_spec_id,
            status,
        )
        .await
        .map_err(galgame_generation_record_error)
    }

    async fn galgame_prompt_artifact_by_generation_spec(
        &self,
        generation_spec_id: i64,
    ) -> Result<Option<prompt_artifact::Model>, AppError> {
        a4n_db::galgame::galgame_prompt_artifact_by_generation_spec(
            self.database.connection(),
            generation_spec_id,
        )
        .await
        .map_err(galgame_generation_record_error)
    }

    async fn upsert_galgame_prompt_artifact_record(
        &self,
        input: GalgamePromptArtifactUpsertInput,
    ) -> Result<prompt_artifact::Model, AppError> {
        a4n_db::galgame::upsert_galgame_prompt_artifact_record(self.database.connection(), input)
            .await
            .map_err(galgame_generation_record_error)
    }

    async fn galgame_media_target_context(
        &self,
        project_id: i64,
        target_kind: &str,
        target_id: i64,
    ) -> Result<GalgameTargetContext, AppError> {
        a4n_db::galgame::galgame_media_target_context(
            self.database.connection(),
            project_id,
            target_kind,
            target_id,
        )
        .await
        .map_err(galgame_media_record_error)
    }

    async fn galgame_generated_image_by_id(
        &self,
        image_id: i64,
        project_id: i64,
    ) -> Result<generated_image::Model, AppError> {
        a4n_db::galgame::galgame_generated_image_by_id(
            self.database.connection(),
            image_id,
            project_id,
        )
        .await
        .map_err(galgame_generation_record_error)
    }

    async fn bind_galgame_media_records(
        &self,
        project_id: i64,
        media_asset_id: i64,
        target_kind: &str,
        target_id: i64,
        role: &str,
        required_media_kind: &str,
    ) -> Result<GalgameMediaBindingResult, AppError> {
        a4n_db::galgame::bind_galgame_media_records(
            self.database.connection(),
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

    async fn unbind_galgame_media_records(
        &self,
        project_id: i64,
        target_kind: &str,
        target_id: i64,
        role: &str,
        media_asset_id: Option<i64>,
    ) -> Result<u64, AppError> {
        a4n_db::galgame::unbind_galgame_media_records(
            self.database.connection(),
            project_id,
            target_kind,
            target_id,
            role,
            media_asset_id,
        )
        .await
        .map_err(galgame_media_record_error)
    }

    async fn completed_galgame_visual_binding_records(
        &self,
        spec: &generation_spec::Model,
        target_entity_kind: &str,
        target_entity_id: i64,
        role: &str,
    ) -> Result<CompletedGalgameVisualBindingRecords, AppError> {
        a4n_db::galgame::completed_galgame_visual_binding_records(
            self.database.connection(),
            spec,
            target_entity_kind,
            target_entity_id,
            role,
        )
        .await
        .map_err(galgame_media_record_error)
    }

    async fn delete_media_asset_with_provenance_records(
        &self,
        media_asset_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::galgame::delete_media_asset_with_provenance_records(
            self.database.connection(),
            media_asset_id,
        )
        .await
        .map_err(galgame_media_record_error)
    }
}

impl OrchestratorContext {
    pub async fn galgame_asset_generation_specs(
        &self,
        project_id: i64,
        target_kinds: &[&str],
        generation_spec_ids: &[i64],
        limit: Option<usize>,
    ) -> Result<Vec<a4n_core::models::generation::generation_spec::Model>, a4n_core::error::AppError>
    {
        self.galgame
            .galgame_asset_generation_specs(project_id, target_kinds, generation_spec_ids, limit)
            .await
    }

    pub(crate) async fn galgame_webgal_portraits(
        &self,
        root_project_id: i64,
    ) -> Result<
        Vec<a4n_core::models::character::character_portrait::Model>,
        a4n_core::error::AppError,
    > {
        self.galgame.galgame_webgal_portraits(root_project_id).await
    }

    pub async fn galgame_portrait_spec_target_profile(
        &self,
        spec: &a4n_core::models::generation::generation_spec::Model,
        input: &a4n_db::galgame::GalgamePortraitSpecTargetInput,
    ) -> Result<a4n_db::galgame::GalgamePortraitSpecTargetProfile, a4n_core::error::AppError> {
        self.galgame
            .galgame_portrait_spec_target_profile(spec, input)
            .await
    }

    pub async fn completed_galgame_portrait_record(
        &self,
        spec: &a4n_core::models::generation::generation_spec::Model,
        profile_id: i64,
        expression: &str,
        pose: &str,
    ) -> Result<a4n_core::models::character::character_portrait::Model, a4n_core::error::AppError>
    {
        self.galgame
            .completed_galgame_portrait_record(spec, profile_id, expression, pose)
            .await
    }

    pub async fn primary_galgame_portrait_reference_image_ids(
        &self,
        profile: &a4n_db::galgame::GalgamePortraitSpecTargetProfile,
    ) -> Result<Vec<i64>, a4n_core::error::AppError> {
        self.galgame
            .primary_galgame_portrait_reference_image_ids(profile)
            .await
    }

    pub async fn mark_galgame_generation_spec_status_record(
        &self,
        generation_spec_id: i64,
        status: &str,
    ) -> Result<(), a4n_core::error::AppError> {
        self.galgame
            .mark_galgame_generation_spec_status_record(generation_spec_id, status)
            .await
    }

    pub async fn galgame_prompt_artifact_by_generation_spec(
        &self,
        generation_spec_id: i64,
    ) -> Result<
        Option<a4n_core::models::generation::prompt_artifact::Model>,
        a4n_core::error::AppError,
    > {
        self.galgame
            .galgame_prompt_artifact_by_generation_spec(generation_spec_id)
            .await
    }

    pub async fn upsert_galgame_prompt_artifact_record(
        &self,
        input: a4n_db::galgame::GalgamePromptArtifactUpsertInput,
    ) -> Result<a4n_core::models::generation::prompt_artifact::Model, a4n_core::error::AppError>
    {
        self.galgame
            .upsert_galgame_prompt_artifact_record(input)
            .await
    }

    pub async fn galgame_media_target_context(
        &self,
        project_id: i64,
        target_kind: &str,
        target_id: i64,
    ) -> Result<a4n_db::galgame::GalgameTargetContext, a4n_core::error::AppError> {
        self.galgame
            .galgame_media_target_context(project_id, target_kind, target_id)
            .await
    }

    pub async fn galgame_generated_image_by_id(
        &self,
        image_id: i64,
        project_id: i64,
    ) -> Result<a4n_core::models::ai::generated_image::Model, a4n_core::error::AppError> {
        self.galgame
            .galgame_generated_image_by_id(image_id, project_id)
            .await
    }

    pub async fn bind_galgame_media_records(
        &self,
        project_id: i64,
        media_asset_id: i64,
        target_kind: &str,
        target_id: i64,
        role: &str,
        required_media_kind: &str,
    ) -> Result<a4n_db::galgame::GalgameMediaBindingResult, a4n_core::error::AppError> {
        self.galgame
            .bind_galgame_media_records(
                project_id,
                media_asset_id,
                target_kind,
                target_id,
                role,
                required_media_kind,
            )
            .await
    }

    pub async fn unbind_galgame_media_records(
        &self,
        project_id: i64,
        target_kind: &str,
        target_id: i64,
        role: &str,
        media_asset_id: Option<i64>,
    ) -> Result<u64, a4n_core::error::AppError> {
        self.galgame
            .unbind_galgame_media_records(project_id, target_kind, target_id, role, media_asset_id)
            .await
    }

    pub async fn completed_galgame_visual_binding_records(
        &self,
        spec: &a4n_core::models::generation::generation_spec::Model,
        target_entity_kind: &str,
        target_entity_id: i64,
        role: &str,
    ) -> Result<a4n_db::galgame::CompletedGalgameVisualBindingRecords, a4n_core::error::AppError>
    {
        self.galgame
            .completed_galgame_visual_binding_records(
                spec,
                target_entity_kind,
                target_entity_id,
                role,
            )
            .await
    }

    pub async fn delete_media_asset_with_provenance_records(
        &self,
        media_asset_id: i64,
    ) -> Result<(), a4n_core::error::AppError> {
        self.galgame
            .delete_media_asset_with_provenance_records(media_asset_id)
            .await
    }
}
