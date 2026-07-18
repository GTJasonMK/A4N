use super::{
    GalgamePortraitSpecTargetInput, GalgamePromptArtifactUpsertInput,
    galgame_asset_plan_spec_record_error, galgame_generation_record_error,
};
use crate::OrchestratorContext;
use a4n_core::error::AppError;
use a4n_core::models::{
    character::{character_portrait, protagonist_profile},
    generation::{generation_spec, prompt_artifact},
};

impl OrchestratorContext {
    pub async fn galgame_asset_generation_specs(
        &self,
        project_id: i64,
        target_kinds: &[&str],
        generation_spec_ids: &[i64],
        limit: Option<usize>,
    ) -> Result<Vec<generation_spec::Model>, AppError> {
        a4n_db::galgame::galgame_asset_generation_specs(
            self.db.connection(),
            project_id,
            target_kinds,
            generation_spec_ids,
            limit,
        )
        .await
        .map_err(galgame_asset_plan_spec_record_error)
    }

    pub async fn galgame_webgal_portraits(
        &self,
        root_project_id: i64,
    ) -> Result<Vec<character_portrait::Model>, AppError> {
        a4n_db::galgame::galgame_webgal_portraits(self.db.connection(), root_project_id)
            .await
            .map_err(galgame_generation_record_error)
    }

    pub async fn galgame_portrait_spec_target_profile(
        &self,
        spec: &generation_spec::Model,
        input: &GalgamePortraitSpecTargetInput,
    ) -> Result<protagonist_profile::Model, AppError> {
        a4n_db::galgame::galgame_portrait_spec_target_profile(self.db.connection(), spec, input)
            .await
            .map_err(galgame_generation_record_error)
    }

    pub async fn completed_galgame_portrait_record(
        &self,
        spec: &generation_spec::Model,
        profile_id: i64,
        expression: &str,
        pose: Option<&str>,
    ) -> Result<character_portrait::Model, AppError> {
        a4n_db::galgame::completed_galgame_portrait_record(
            self.db.connection(),
            spec,
            profile_id,
            expression,
            pose,
        )
        .await
        .map_err(galgame_generation_record_error)
    }

    pub async fn primary_galgame_portrait_reference_image_ids(
        &self,
        profile: &protagonist_profile::Model,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::galgame::primary_galgame_portrait_reference_image_ids(self.db.connection(), profile)
            .await
            .map_err(galgame_generation_record_error)
    }

    pub async fn mark_galgame_generation_spec_status_record(
        &self,
        generation_spec_id: i64,
        status: &str,
    ) -> Result<(), AppError> {
        a4n_db::galgame::mark_galgame_generation_spec_status_record(
            self.db.connection(),
            generation_spec_id,
            status,
        )
        .await
        .map_err(galgame_generation_record_error)
    }

    pub async fn galgame_prompt_artifact_by_generation_spec(
        &self,
        generation_spec_id: i64,
    ) -> Result<Option<prompt_artifact::Model>, AppError> {
        a4n_db::galgame::galgame_prompt_artifact_by_generation_spec(
            self.db.connection(),
            generation_spec_id,
        )
        .await
        .map_err(galgame_generation_record_error)
    }

    pub async fn upsert_galgame_prompt_artifact_record(
        &self,
        input: GalgamePromptArtifactUpsertInput,
    ) -> Result<prompt_artifact::Model, AppError> {
        a4n_db::galgame::upsert_galgame_prompt_artifact_record(self.db.connection(), input)
            .await
            .map_err(galgame_generation_record_error)
    }
}
